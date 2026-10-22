//! One description of each domain/card pair.
//!
//! Request parsing reads [`parameter_names`], [`phase_names`], and [`load_names`]
//! from this list. The numbers come from `genesis_kernel` helpers and constants.

use std::sync::OnceLock;

use genesis_kernel::{
    battery_sag, cable_stretch, cantilever_eth, capsule_stardust, cell_18650pf, generator_mmrtg_f1, grid_node, hydraulic_fluid, resonance, satellite_lageos1, swing, tesseract, thermal, ti64_lattice,
    reactor_triga_jsi,
    creep_cmsx6,
    capsule_insight,
    crater_ryugu_sci,
    link_gothenburg_10010,
    solar_system,
    tire_eve_r19_ice,
    accumulator_sbo450,
    los_jwst_fine_guide,
    tokamak_vde_mast,
    fingerpad_p02_glass,
};
use serde_json::{Map, Value, json};

use crate::DoorError;
use crate::view::View;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Role {
    Phase,
    Load,
}

pub struct FieldSpec {
    pub name: &'static str,
    pub role: Role,
    pub unit: &'static str,
    pub components: u8,
    pub minimum: Option<f64>,
    pub minimum_exclusive: bool,
    pub maximum: Option<f64>,
    pub maximum_exclusive: bool,
    pub bound: String,
}

pub struct Advisory {
    pub name: &'static str,
    pub mask: u64,
    pub seals: bool,
}

pub struct FrameSlot {
    pub slot: &'static str,
    pub holds: &'static str,
    /// The key of the state that this slot holds, `name[i]` for one component
    /// of a vector. Empty for the flags and the proof.
    pub state: String,
    pub unit: &'static str,
}

pub struct Card {
    pub domain: &'static str,
    pub card: &'static str,
    pub specimen: &'static str,
    pub fields: Vec<FieldSpec>,
    pub parameter_names: Vec<&'static str>,
    pub phase_names: Vec<&'static str>,
    pub load_names: Vec<&'static str>,
    /// The card's own step floor, `floor_dt_s` in `genesis_kernel`.
    pub floor_s: f64,
    pub floor_from: &'static str,
    /// The card's own step ceiling, `gate_dt_s` in `genesis_kernel`.
    pub ceiling_s: f64,
    pub ceiling_from: &'static str,
    /// `π/ω`, for a card that has a natural frequency or a pole.
    pub nyquist_s: Option<f64>,
    pub nyquist_rule: String,
    pub rejects: &'static [&'static str],
    pub advisory: Vec<Advisory>,
    pub slots: Vec<FrameSlot>,
}

/// The keys a describe request requires.
pub(crate) const DESCRIBE_KEYS: &[&str] = &["domain", "card"];
/// The key a describe request may add: which form of the description.
pub(crate) const FORM: &str = "form";

/// The two forms of a description.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Form {
    /// Everything the description holds: the answer when `form` is left out.
    Full,
    /// What an agent needs to write a first request, in a page.
    Short,
}

/// The form a describe request asks for, of a card or of a solver. Any value
/// but "short" and "full" is refused by name.
pub(crate) fn form_of(object: &Map<String, Value>) -> Result<Form, DoorError> {
    match object.get(FORM) {
        None => Ok(Form::Full),
        Some(Value::String(form)) if form == "full" => Ok(Form::Full),
        Some(Value::String(form)) if form == "short" => Ok(Form::Short),
        Some(other) => Err(DoorError::Unprocessable(format!(
            "Physical Violation: '{FORM}' must be one of \"short\", \"full\"; this request has {other}."
        ))),
    }
}

/// How the `form` key is declared in the schema of a describe request.
pub(crate) fn form_schema() -> Value {
    json!({
        "type": "string",
        "enum": ["short", "full"],
        "description": "Leave this key out, or \"full\", for the whole description; \"short\" for a page that is enough to write a first request."
    })
}

/// What the descriptions of `describe_card` and `describe_solver` say of the
/// two forms: that the short one exists, and what only the full one has.
pub(crate) const SHORT_FORM_SAYS: &str = "With form \"short\" the answer is a page, enough to write a first request: what the model is of, the evidence behind it in numbers, each input with its unit and range, the names of its stops, and a worked request. The full form, the answer when form is left out, adds each bound's reason, the validation record and each stop's meaning.";

/// What the full form of a card's description has that the short form leaves out.
const FULL_FORM_HAS: &str = "each bound's reason, the validation record, the frame layout and each stop's meaning";

/// What every request number must be, whatever range a field publishes.
const NUMBER_RULE: &str = "Every request number must be a finite, normal 64-bit number whose copy in a 32-bit frame slot is finite, and a number written as non-zero must not read as zero in 64 bits, as 1e-400 does. This rule stands beside each field's range and refuses some values inside it: a subnormal such as 1e-310 is refused wherever zero and larger numbers are accepted. A normal 64-bit number under about 1.2e-38 is accepted by this rule and kept exactly in the state; its copy in a 32-bit frame slot reads as a subnormal or as zero.";

pub fn all() -> &'static [Card] {
    static CELL: OnceLock<Vec<Card>> = OnceLock::new();
    CELL.get_or_init(build).as_slice()
}

pub fn find(domain: &str, card: &str) -> Option<&'static Card> {
    all()
        .iter()
        .find(|item| item.domain == domain && item.card == card)
}

pub fn parameter_names(domain: &str, card: &str) -> &'static [&'static str] {
    find(domain, card)
        .expect("descriptor")
        .parameter_names
        .as_slice()
}

pub fn phase_names(domain: &str, card: &str) -> &'static [&'static str] {
    find(domain, card)
        .expect("descriptor")
        .phase_names
        .as_slice()
}

pub fn load_names(domain: &str, card: &str) -> &'static [&'static str] {
    find(domain, card)
        .expect("descriptor")
        .load_names
        .as_slice()
}

pub fn list_value() -> Value {
    list_value_in(&View::full())
}

pub fn list_value_in(view: &View) -> Value {
    let cards: Vec<Value> = view
        .cards()
        .into_iter()
        .map(|item| {
            let mut row = Map::new();
            row.insert("domain".to_string(), json!(item.domain));
            row.insert("card".to_string(), json!(item.card));
            // the evidence behind the model, in one word, before its sentence
            if let Some((word, _)) = crate::find_model::verdict_of_card(item.domain, item.card) {
                row.insert("against_measurement".to_string(), json!(word));
            }
            row.insert("specimen".to_string(), json!(crate::spec_sheet::row_of(item.card, item.specimen)));
            Value::Object(row)
        })
        .collect();
    json!({ "count": cards.len(), "cards": cards })
}

pub fn describe_value_in(view: &View, domain: &str, card: &str) -> Option<Value> {
    let mut body = describe_one(view.find(domain, card)?);
    // The couplings the view holds both cards of.
    let couplings = crate::couplings::table_in(view, domain, card);
    if couplings.as_array().is_some_and(|list| !list.is_empty()) {
        body["couplings"] = couplings;
    }
    Some(body)
}

/// A field of the short form: a row of the full form with its prose left
/// out. The name, the unit, the minimum and the maximum are the full row's
/// own; a vector says it has three components, and an end that is outside
/// the range says so.
fn short_field(full: &Value) -> Value {
    let mut row = Map::new();
    for key in ["name", "unit", "components", "minimum", "minimum_exclusive", "maximum", "maximum_exclusive"] {
        let value = &full[key];
        let said = match key {
            "components" => *value != json!(1),
            "minimum_exclusive" | "maximum_exclusive" => *value == json!(true),
            _ => true,
        };
        if said {
            row.insert(key.to_string(), value.clone());
        }
    }
    Value::Object(row)
}

/// The short form of a card's description: what an agent needs to write a
/// first request and to know what it is dealing with, in a page. Every part
/// is taken from the full description, so no bound, name or number is
/// written twice; the evidence is the one `find_model` and `list_cards` give.
pub fn describe_short_in(view: &View, domain: &str, card: &str) -> Option<Value> {
    let full = describe_value_in(view, domain, card)?;
    let fields = |key: &str| Value::Array(full[key].as_array().into_iter().flatten().map(short_field).collect());
    let names = |key: &str, name: &str| Value::Array(full[key].as_array().into_iter().flatten().map(|row| row[name].clone()).collect());
    let mut short = Map::new();
    short.insert("domain".to_string(), full["domain"].clone());
    short.insert("card".to_string(), full["card"].clone());
    short.insert(FORM.to_string(), json!("short"));
    short.insert("specimen".to_string(), json!(crate::spec_sheet::short_of(card, full["specimen"].as_str().unwrap_or(""))));
    if let Some((word, phrase)) = crate::find_model::verdict_of_card(domain, card) {
        short.insert("against_measurement".to_string(), json!(word));
        short.insert("accuracy_against_measurement".to_string(), json!(phrase));
    }
    calibration_into(&mut short, card);
    short.insert("phase_fields".to_string(), fields("phase_fields"));
    short.insert("load_fields".to_string(), fields("load_fields"));
    short.insert(
        "dt_s".to_string(),
        json!({ "unit": full["dt_s"]["unit"], "floor_s": full["dt_s"]["floor_s"], "ceiling_s": full["dt_s"]["ceiling_s"] }),
    );
    short.insert("stops".to_string(), names("rejects", "name"));
    short.insert("declared_ends".to_string(), names("declared_ends", "reject"));
    short.insert("continues_with_from".to_string(), full["continuation"]["served"].clone());
    short.insert("example_run_trajectory".to_string(), full["example_run_trajectory"].clone());
    if let Some(refused) = full.get(EXAMPLE_REFUSED) {
        short.insert(EXAMPLE_REFUSED.to_string(), refused.clone());
    }
    short.insert(
        "full_form".to_string(),
        json!(format!(
            "Short form: phase_fields are initial's keys, load_fields each segment's load keys, a null bound is given in words in full, stops are refusal names. describe_card with form \"full\" or none has {FULL_FORM_HAS}."
        )),
    );
    Some(Value::Object(short))
}

/// A describe request. Keys other than `domain`, `card` and `form` are
/// refused with the same sentence a step uses for that key. With no `form`,
/// or "full", the answer is the whole description; with "short", its short
/// form. Both doors call this.
pub fn describe_request(value: &Value) -> Result<Value, DoorError> {
    describe_request_in(&View::full(), value)
}

/// The same, for the pairs of one view. A pair outside it is not recognized.
pub fn describe_request_in(view: &View, value: &Value) -> Result<Value, DoorError> {
    let object = value.as_object().ok_or_else(|| {
        DoorError::Unprocessable("Physical Violation: the request must be a JSON object.".into())
    })?;
    for key in object.keys() {
        if !DESCRIBE_KEYS.contains(&key.as_str()) && key != FORM {
            return Err(DoorError::Unprocessable(crate::locked_or_unknown(key)));
        }
    }
    let domain = required_string(object, "domain")?;
    let card = required_string(object, "card")?;
    let described = match form_of(object)? {
        Form::Full => describe_value_in(view, &domain, &card),
        Form::Short => describe_short_in(view, &domain, &card),
    };
    described.ok_or_else(|| DoorError::NotFound(crate::CARD_NOT_RECOGNIZED.to_string()))
}

fn required_string(object: &Map<String, Value>, key: &str) -> Result<String, DoorError> {
    match object.get(key) {
        None => Err(DoorError::Unprocessable(format!(
            "Missing required parameter: '{key}' (InputMissing)."
        ))),
        Some(Value::String(text)) if !text.is_empty() => Ok(text.clone()),
        Some(Value::String(_)) => Err(DoorError::Unprocessable(format!(
            "Missing required parameter: '{key}' (InputMissing)."
        ))),
        Some(_) => Err(DoorError::Unprocessable(format!(
            "Physical Violation: '{key}' must be a string."
        ))),
    }
}

pub fn list_schema() -> Map<String, Value> {
    object_map(json!({
        "type": "object",
        "properties": {},
        "additionalProperties": false,
        "description": "No arguments. Returns every domain/card pair."
    }))
}

/// The domain and card properties of a plain request schema: two strings,
/// declared once. The catalogue is left to `list_cards`, which names every
/// pair of the caller's view, and to `describe_card`, which gives a pair's
/// fields; no schema lists the pairs or their fields.
fn pair_properties(_view: &View) -> (Value, Value) {
    (
        json!({ "type": "string", "description": "The domain of the pair, as list_cards names it." }),
        json!({ "type": "string", "description": "The card of the pair, as list_cards names it." }),
    )
}

/// The plain schema of a describe request: two required strings. The pairs
/// of the view are named in the descriptions, and no variant is declared.
pub fn describe_schema_in(view: &View) -> Map<String, Value> {
    let (domain, card) = pair_properties(view);
    object_map(json!({
        "type": "object",
        "properties": { "domain": domain, "card": card, FORM: form_schema() },
        "required": ["domain", "card"],
        "additionalProperties": false,
        "description": "Full limits, fields, reject names and meanings, advisory flags, frame slots, and worked examples for one pair."
    }))
}

pub fn compute_step_schema_in(view: &View) -> Map<String, Value> {
    let (domain, card) = pair_properties(view);
    let parameters = json!({
        "type": "object",
        "description": "The step's inputs: the card's phase fields, its load fields and dt_s, each a number (a vector field is an array of three numbers), and no other key. describe_card gives the fields of the pair, with each unit and bound."
    });
    object_map(json!({
        "type": "object",
        "properties": { "domain": domain, "card": card, "parameters": parameters, "from": crate::chain::from_schema(false) },
        "required": ["domain", "card", "parameters"],
        "additionalProperties": false,
        "description": "One anchored step. The shape of parameters depends on the pair: describe_card gives it."
    }))
}

pub fn run_trajectory_schema_in(view: &View) -> Map<String, Value> {
    let mut schema = run_request_schema_in(view);
    schema["properties"]
        .as_object_mut()
        .expect("properties")
        .insert(crate::readout::READOUT.to_string(), crate::readout::schema());
    schema["properties"]
        .as_object_mut()
        .expect("properties")
        .insert(crate::chain::FROM.to_string(), crate::chain::from_schema(true));
    object_map(schema)
}

/// The run of a `run_trajectory` request without its readout, as
/// `validate_series` takes it under `run`.
pub(crate) fn run_request_schema_in(view: &View) -> Value {
    let description = format!(
        "One anchored run. Total steps must be <= {}. Recorded frames must be <= {}. The shape of initial and of each segment's load depends on the pair: describe_card gives it.",
        super::trajectory::MAX_TRAJECTORY_STEPS,
        super::trajectory::MAX_RECORDED_FRAMES
    );
    let (domain, card) = pair_properties(view);
    let initial = json!({
        "type": "object",
        "description": "The phase fields of the card at the start of the run, each a number (a vector field is an array of three numbers), and no other key but signed: an instrument's signature over the rest of initial, under the key \"initial\", by the rule of a segment's signed. describe_card gives the fields, with each unit and bound."
    });
    let load = json!({
        "type": "object",
        "description": "The load fields of the card for this segment, and no other key. describe_card gives them, with each unit and bound."
    });
    json!({
        "type": "object",
        "properties": {
            "domain": domain,
            "card": card,
            "initial": initial,
            "dt_s": { "type": "number", "description": "Step, in seconds. Held for the whole run." },
            "record_every": { "type": "integer", "minimum": 1 },
            "segments": {
                "type": "array",
                "minItems": 1,
                "items": {
                    "type": "object",
                    "properties": {
                        "steps": { "type": "integer", "minimum": 1 },
                        "load": load,
                        "signed": crate::signed_load::schema()
                    },
                    "required": ["steps", "load"],
                    "additionalProperties": false
                }
            }
        },
        "required": ["domain", "card", "initial", "dt_s", "segments", "record_every"],
        "additionalProperties": false,
        "description": description,
    })
}

/// The declaration of a tape check: the ten keys of a run's answer that the
/// check reads, each with its JSON type. `inputs` is declared as an object and
/// no card's fields are listed: a tape's `inputs` is the request that was run,
/// sent back as it was returned, and a card's fields, units and bounds are in
/// `describe_card`. The check itself (`trajectory::check_in`) reads `inputs`
/// with the card's own lists and is the same for every caller.
pub fn tape_schema_in(_view: &View) -> Map<String, Value> {
    object_map(json!({
        "type": "object",
        "properties": {
            "request": {
                "type": "object",
                "description": "With run_seal alone: the run_trajectory request as it was sent, with its readout and its from when it had them (the answer's inputs holds neither; the upstream request in that from goes without a readout, as run_trajectory took it). It is run again, under a physics permit and the limits of run_trajectory, and its seal is compared with run_seal. Its fields are run_trajectory's."
            },
            "inputs": {
                "type": "object",
                "description": "The inputs of the run_trajectory answer, sent back as returned: the request that was run. It is hashed, and run again, with the answer's readout when it has one, under a physics permit and the limits of run_trajectory. A card's fields, units and bounds are in describe_card."
            },
            "frames": {
                "type": "array",
                "items": {
                    "type": "object",
                    "properties": {
                        "step": { "type": "integer", "minimum": 1 },
                        "frame_hex": { "type": "string" },
                        "link": { "type": "string" }
                    },
                    "required": ["step", "frame_hex", "link"],
                    "additionalProperties": false
                }
            },
            "outcome": { "type": "string" },
            "steps_landed": { "type": "integer", "minimum": 0 },
            "reject": { "type": ["string", "null"] },
            "run_seal": { "type": "string", "pattern": "^[0-9a-f]{96}$", "description": "The run_seal of the run_trajectory answer: with request, or as part of the whole answer." },
            "final_state": {},
            "final_residual_conservation_error_hex": { "type": ["string", "null"] },
            "max_residual_conservation_error_hex": { "type": ["string", "null"] },
            "state_digest": { "type": ["string", "null"] },
            "chain_seal": { "type": "string", "pattern": "^[0-9a-f]{96}$", "description": "With request and run_seal: a chained answer's chain_seal." },
            "links": { "type": "array", "items": { "type": "object" }, "description": "Alone: a chain's answers in order." },
            "public_key": { "type": "string", "description": "The base64 of the ML-DSA-87 public key (2,592 bytes) to verify the answer's signature with; without it, this engine's own." },
            "seen_nonces": { "type": "array", "items": { "type": "string", "pattern": "^[0-9a-f]{64}$" }, "description": "Nonces of signatures already seen: one of them again is SignatureNonceReplayed." }
        },
        "additionalProperties": true,
        "description": "Either request and run_seal alone, the short form: the run is computed again and its seal compared with run_seal. Or the whole run_trajectory answer sent back as it was returned, without request, the long form: inputs, frames, outcome, steps_landed, reject, run_seal, final_state, final_residual_conservation_error_hex, max_residual_conservation_error_hex and state_digest are then all required, along with the rest of the answer. The trajectory chain, the state digest and the run seal are recomputed from its exact hex values; then its request is computed again under a physics permit and the limits of run_trajectory, and the engine's whole answer to it compared with the answer sent, value by value and then byte for byte, the receipt included, so an answer that holds is every byte the engine's. A seal has no key: an answer whose values were rewritten with its seals worked out again holds every seal, and only the computation shows it is not the engine's. An answer that continues a sealed upstream run (its chain names upstream_run_seal) carries the upstream's seal and not its request, and its long form is refused (AnswerCarriesNoRequest): its chain's answers in order, as links, carry each request, and each link is computed again from the links before it. Or links alone: a chain's answers in order, held as the long form holds one. A design_run answer sent whole is designed again and held the same way. The long form's answer carries holds, true when the answer sent is every byte the engine's, and values_hold beside it, true when every value of it is the engine's answer to its request computed again, whatever its spelling: each number by its 64-bit value, strings, flags and null exactly, keys and their order, the receipt string among them. A copy whose numbers a client wrote again in its own spelling (0.0001 for 1.0e-4, 12 for 12.0) has holds false and values_hold true, and differs_at names the first place its text differs. An answer the engine signed (signature, ML-DSA-87 over its receipt, with issued_at, expires_at and nonce) has its signature verified first, against public_key or else this engine's own key: one that does not hold is cause signature, named (SignatureDoesNotVerify, SignatureExpired, SignatureNonceReplayed, SignatureKeyMismatch, ReceiptDoesNotMatch, SignatureAbsent when public_key is given and there is none), and nothing is computed. A signed answer a client wrote again in its own spacing or key order has its signature verified over the receipt it carries; where that receipt does not cover the bytes sent, signature names ReceiptDoesNotMatch with written_again, and the answer is computed again: holds, values_hold and differs_at say what changed."
    }))
}

#[path = "card_grasp_lift_glass_4tu_descriptor.rs"]
mod card_grasp_lift_glass_4tu_descriptor;
#[path = "card_momentum_jwst_srp_descriptor.rs"]
mod card_momentum_jwst_srp_descriptor;

#[path = "card_momentum_jwst_dihedral_descriptor.rs"]
mod card_momentum_jwst_dihedral_descriptor;

#[path = "card_generator_mmrtg_seebeck_descriptor.rs"]
mod card_generator_mmrtg_seebeck_descriptor;

#[path = "card_reactor_msre_pump_descriptor.rs"]
mod card_reactor_msre_pump_descriptor;
#[path = "card_reactor_spert3_twin_descriptor.rs"]
mod card_reactor_spert3_twin_descriptor;
#[path = "card_cell_2170_twin_descriptor.rs"]
mod card_cell_2170_twin_descriptor;

#[path = "card_grid_gb_twin_descriptor.rs"]
mod card_grid_gb_twin_descriptor;

#[path = "card_tokamak_jet_dte1_twin_descriptor.rs"]
mod card_tokamak_jet_dte1_twin_descriptor;

#[path = "card_runaway_18650_lco_twin_descriptor.rs"]
mod card_runaway_18650_lco_twin_descriptor;

#[path = "card_reactor_spert3_ccore_plate_descriptor.rs"]
mod card_reactor_spert3_ccore_plate_descriptor;

fn build() -> Vec<Card> {
    let mut cards = built();
    // A card compared with measurement is described by its spec sheet: the
    // six parts as one paragraph, built once, here.
    for card in &mut cards {
        if let Some(sheet) = crate::spec_sheet::of(card.card) {
            card.specimen = Box::leak(sheet.paragraph().into_boxed_str());
        }
    }
    cards
}

fn built() -> Vec<Card> {
    vec![
        ti64(),
        grid(),
        swing(),
        battery(),
        cell_18650pf_card(),
        thermal_plate(),
        resonance_mount(),
        tesseract_drive(),
        hydraulic(),
        cable(),
        generator(),
        capsule(),
        satellite(),
        card_reactor_spert3_twin_descriptor::card(),
        cantilever_eth_card(),
        card_cell_2170_twin_descriptor::card(),
        creep_cmsx6_card(),
        capsule_insight_card(),
        crater_ryugu_sci_card(),
        card_generator_mmrtg_seebeck_descriptor::card(),
        card_momentum_jwst_dihedral_descriptor::card(),
        card_momentum_jwst_srp_descriptor::card(),
        link_gothenburg_10010_card(),
        card_grid_gb_twin_descriptor::card(),
        solar_system_card(),
        tire_eve_r19_ice_card(),
        los_jwst_fine_guide_card(),
        card_tokamak_jet_dte1_twin_descriptor::card(),
        fingerpad_p02_glass_card(),
        accumulator_sbo450_card(),
        reactor_triga_jsi_card(),
        card_reactor_spert3_ccore_plate_descriptor::card(),
        card_runaway_18650_lco_twin_descriptor::card(),
        card_grasp_lift_glass_4tu_descriptor::card(),
        tokamak_vde_mast_card(),
        card_reactor_msre_pump_descriptor::card(),
    ]
}

/// The one line an agent reads in `list_cards`.
pub(crate) const ACCUMULATOR_SBO450_SPECIMEN: &str = "The nitrogen charge of one HYDAC SBO450-0.6 diaphragm accumulator on the test rig at TU Darmstadt, as a piston pushes oil in and draws it out: gas pressure, temperature and an energy ledger. The ratio of piston area to vessel volume and the thermal time constant are fitted to this accumulator's own runs at 40 bar precharge and 60 bar. Compared with 285 held-back runs at 40, 80 and 120 bar and at half the charge, 0.01 to 20 Hz: the comparison failed, two of six lines met, and a second scoring confirmed it. Amended after that with every run opened, which is not a new test: on the 196 runs in the range in which it held, a precharge of 20 bar at 37 to 43 or 55 to 67 bar, or of 40 bar at 57 to 64 or 75 to 86 bar, a wall at 293.4 to 293.7 K, gas at 285.9 to 301.8 K and a piston at most 0.7 m/s, the swing of gas pressure per piston travel is within 0.49 percent in the median and 4.05 at most. At 109 to 132 bar it missed, 2.72 percent median against 2.0. Outside that range a step lands with FLAG_OUTSIDE_COMPARED_RUNS, up to the vessel's rated 330 bar; heat above 0.065 of the work, 0.1 Hz and slower, is refused.";

fn accumulator_sbo450_card() -> Card {
    let accumulator = accumulator_sbo450::AccumulatorSbo450::on_darmstadt_rig();
    // The clock comes from the thermal time constant, so no `π/ω` is published.
    let gate = accumulator.gate_dt_s().expect("specimen gate");
    let floor = accumulator.floor_dt_s().expect("specimen floor");
    finish(
        "energy",
        "accumulator_sbo450",
        ACCUMULATOR_SBO450_SPECIMEN,
        vec![
            scalar(
                "gas_mass_kg",
                Role::Phase,
                "kg",
                Some(0.0),
                true,
                None,
                false,
                "above 0 kg (GasMassNotPositive): the nitrogen in the accumulator, which never changes. A charge is the vessel's 0.58879 l full of nitrogen at the precharge, absolute, and 293.15 K by the card's own equation of state: 0.013653 kg for 20 bar and 0.027467 kg for 40 bar. Each band is a thousandth either side, so a charge typed to four figures is inside, as the worked examples' 0.02747 kg is. Compared at those two charges, 0.013639 to 0.013667 kg and 0.02744 to 0.027495 kg; any other mass sets FLAG_OUTSIDE_COMPARED_RUNS. The card as first compared held a smaller vessel and its charges were 0.01336 and 0.02688 kg".into(),
            ),
            scalar(
                "gas_pressure_pa",
                Role::Phase,
                "Pa",
                Some(0.0),
                true,
                Some(accumulator_sbo450::PRESSURE_RATED_PA),
                false,
                "above 0 Pa and at most 33000000 Pa, absolute: the gas pressure at the start; the card works the gas volume back from it. 330 bar is the vessel's rated maximum working pressure, the rating HYDAC's SBO catalogue gives the type SBO450-0.6 in its dimension table, 4700 psi; the two 0.6 l parts it lists are rated 3600 psi, 250 bar, and which the rig carried is not on disk (PressureAboveRated); a gas volume past the vessel's, below the precharge, has no oil left (OilSideEmpty). Compared at 37.76 to 42.71 bar at 20 / 40, 55.26 to 66.37 bar at 20 / 60, 57.36 to 63.14 bar at 40 / 60 and 75.31 to 85.55 bar at 40 / 80, held as 37 to 43 and 55 to 67 bar for the 20 bar charge and 57 to 64 and 75 to 86 bar for the 40 bar charge; a pressure outside its charge's bands sets FLAG_OUTSIDE_COMPARED_RUNS. At 109 to 132 bar, 40 / 120, the card as first compared reads 2.72 percent median error of the swing (target 2.0), three runs above 130 bar with no answer".into(),
            ),
            scalar(
                "gas_temperature_k",
                Role::Phase,
                "K",
                Some(accumulator_sbo450::CRITICAL_TEMPERATURE_K),
                true,
                None,
                false,
                "above 126.192 K, nitrogen's critical temperature, where it can condense and one phase of the equation of state is not this charge (GasBelowCriticalTemperature): the gas temperature at the start. Compared on 285.97 to 301.78 K over every step the compared runs landed, held as 285.9 to 301.8 K; outside it sets FLAG_OUTSIDE_COMPARED_RUNS".into(),
            ),
            scalar(
                "wall_temperature_k",
                Role::Phase,
                "K",
                Some(0.0),
                true,
                None,
                false,
                "above 0 K (WallTemperatureNotPositive): the temperature of the wall, held for the whole run. It is part of the start and no segment can change it. The wall of every compared run, the mean room temperature of its record, was 293.46 to 293.60 K, held as 293.4 to 293.7 K; any other wall sets FLAG_OUTSIDE_COMPARED_RUNS. The diaphragm's rated temperatures are not on the card".into(),
            ),
            scalar(
                "work_moved_j",
                Role::Phase,
                "J",
                Some(0.0),
                false,
                None,
                false,
                "at least 0 J, required: M at the start, the work of the oil on the gas, either way, summed step by step over the motion before this start. State 0 for gas that has not moved. To carry on a run or a single step, state the work_moved_j of its returned state; the run then weighs its heat against the whole motion, as one run would. A start whose heat_moved_j is already more than 0.065 of the larger of work_moved_j and 100 J is refused as HeatShareAboveCard, and a sum below zero as NegativeMovedSum. The card cannot see motion before its start, so it holds the sum it is given and cannot confirm it; a caller who states a sum that is not so gets answers the comparison does not cover".into(),
            ),
            scalar(
                "heat_moved_j",
                Role::Phase,
                "J",
                Some(0.0),
                false,
                None,
                false,
                "at least 0 J, required: H at the start, the heat between the gas and the wall, either way, summed step by step over the motion before this start. State 0 for gas that has not moved. To carry on a run or a single step, state the heat_moved_j of its returned state. A start whose sum is already more than 0.065 of the larger of work_moved_j and 100 J is refused as HeatShareAboveCard, and a sum below zero as NegativeMovedSum. The card cannot confirm it".into(),
            ),
            scalar(
                "piston_speed_m_s",
                Role::Load,
                "m/s",
                None,
                false,
                None,
                false,
                "the speed of the piston, held over a step. Positive pushes oil in and compresses the gas. A speed times dt_s above 1 mm, 1.5 percent of the smallest compared gas volume, is refused as PistonTravelAboveStep. The fastest compared run, 5 mm at 20 Hz, reached 0.69999 m/s as the mean over a step, above 1 Hz 212 runs at 0.59 percent median error of the swing; a faster speed sets FLAG_OUTSIDE_COMPARED_RUNS".into(),
            ),
        ],
        floor,
        "the ceiling divided by 2^20, about 3.34e-7 s",
        gate,
        "0.3504 s, a fiftieth of the thermal time constant of 17.52 s",
        None,
        "Over a step the piston speed is held, and the gas volume changes by the piston area, 19.635 cm2 (a 50 mm bore), times the travel. The gas is nitrogen by the Peng-Robinson equation of state; it exchanges heat with the wall with one time constant, 17.52 s; the vessel's gas volume is 0.58879 l. The time constant and the ratio of the area to the volume are fitted to this accumulator's own runs. A run starts at time zero with the work and the heat at zero; the time, the work, the heat and the energy at the start are the anchor of the ledger and begin again at each start; the ledger's conservation balance is held from each start, and no limit of the card's range depends on them. The card keeps two more sums over the steps of the motion: M, the absolute work of each step, and H, the absolute heat of each step. A start carries them in as work_moved_j and heat_moved_j, 0 for gas that has not moved; each step adds to them; the state gives them back under the same names, and heat_share is H over the larger of M and 100 J. A step lands if H is at most 0.065 of the larger of M and 100 J; otherwise it is refused as HeatShareAboveCard and the run ends there, sealed, with its proof up to the last step that landed. In the measured sines that refuses 0.1 Hz and slower and answers 0.2 Hz and faster. A piston at rest lands for as long as the gas is at the wall's temperature; off it, a rest from the start of a run is answered for its first 6.5 J of heat. A single step or a run is carried on by handing back its returned gas_pressure_pa, gas_temperature_k, work_moved_j and heat_moved_j as a new start: the two sums travel with the motion, so single steps handed back, runs carried on from each other and one run are weighed alike, and the step a run refuses is refused when it is asked alone from that run's returned state. A start without the two sums is refused as a missing input. The four limits the card works out itself (the travel in a step, the pressure and the volume worked back from the starting pressure, and the heat against the work) carry an allowance of a relative 1e-12, so a start, a step or a run stated or landing exactly at a limit is inside it. A run ends in a sealed refusal at the step that would land above the rated 330 bar, at or below nitrogen's critical temperature, past the vessel volume, or past the share of heat. A step outside the range compared with measurement, a gas mass other than the two charges, a pressure outside its charge's bands, a wall or a gas temperature or a piston speed outside the compared, lands with FLAG_OUTSIDE_COMPARED_RUNS: the Peng-Robinson gas, the one time constant and the ledger are the same there.".to_string(),
        &[
            "NonFiniteInput",
            "NonFiniteState",
            "NonPositiveDt",
            "InvalidAccumulator",
            "GasMassNotPositive",
            "WallTemperatureNotPositive",
            "PistonTravelAboveStep",
            "GasBelowCriticalTemperature",
            "PressureAboveRated",
            "NegativeMovedSum",
            "OilSideEmpty",
            "HeatShareAboveCard",
            "SpontaneousGeneration",
            "FirstLawBreach",
        ],
        vec![Advisory {
            name: "FLAG_OUTSIDE_COMPARED_RUNS",
            mask: accumulator_sbo450::FLAG_OUTSIDE_COMPARED_RUNS,
            seals: true,
        }],
        slots(&[
            ("t", "Time since the start of the run, in seconds."),
            ("pos[0]", "gas_pressure_pa"),
            ("pos[1]", "gas_temperature_k"),
            ("pos[2]", "gas_volume_m3"),
            ("vel[0]", "work_on_gas_j"),
            ("vel[1]", "heat_to_wall_j"),
            ("vel[2]", "gas_mass_kg"),
            (
                "force_torque",
                "The gas pressure times the area of the 50 mm piston, in newtons: the force the gas pressure puts on the piston through the oil.",
            ),
            ("residual", "Conservation residual."),
            (
                "flags",
                "FLAG_OUTSIDE_COMPARED_RUNS when the step lies outside the range compared with measurement. It seals.",
            ),
            ("proof", "Not written by the step."),
        ]),
    )
}

fn crater_ryugu_sci_card() -> Card {
    let crater = crater_ryugu_sci::CraterRyuguSci::sci_impact();
    let gate = crater.gate_dt_s().expect("specimen gate");
    let floor = crater.floor_dt_s().expect("specimen floor");
    finish(
        "aerospace",
        "crater_ryugu_sci",
        "The crater made on asteroid Ryugu by the 2 kg copper impactor of Hayabusa2 on 5 April 2019: its radius growing from 0.1622 m until it stops, by the law of crater size for dry sand held by gravity alone, fitted in the laboratory on Earth and not to Ryugu, with no load; its predicted radius is 18 percent above the measured crater, a miss of its own line, and its clock is not compared with Ryugu.",
        vec![scalar(
            "radius_m",
            Role::Phase,
            "m",
            Some(crater_ryugu_sci::RADIUS_FLOOR_M),
            false,
            Some(crater_ryugu_sci::FINAL_RADIUS_M),
            false,
            format!(
                "inclusive [{}, {}] m: the crater's radius at the level of the undisturbed surface, up to the radius at which this crater stops. The speed of the wall is the one this impact gives it at that radius; the caller does not set it. Below 0.1622 m the energy the wall has yet to give up, over 22 800 J/kg, is so large that one rounding of it in 64 bits exceeds a tenth of the ledger's limit of 5.064e-11 J/kg, and the start is refused as RadiusBelowLedgerResolution; that radius is 4.3 radii of the projectile's equivalent sphere and the wall there moves at 214 m/s, a tenth of the projectile's speed. The final radius is compared with Ryugu, 8.571 m against the measured 7.25 +/- 0.4 m, 18.2 percent above, outside the allowance of 0.97 m; the growth and its clock, read by eye from a laboratory figure at 1 g, are computed with no measurement of Ryugu to meet at any radius",
                token(crater_ryugu_sci::RADIUS_FLOOR_M),
                token(crater_ryugu_sci::FINAL_RADIUS_M)
            ),
        )],
        floor,
        "the ceiling divided by 2^20: a step that moves the wall by about a tenth of a micrometre at its fastest. Under it a step is mostly rounding",
        gate,
        "0.25 s, a locked constant: about 1,600 steps from 1.75 m to the stop. Over that whole growth the largest conservation residual at this step is 5.8e-8 against a limit of 1e-6; at 0.5 s it is 8.8e-7 and at 1 s the first step is refused. Nearer the impact the wall is faster and a step of 0.25 s is refused by the ledger: from 0.1622 m a run takes steps of a few microseconds at first, growing with the radius",
        None,
        "No load: nothing acts on the crater but the impact and Ryugu, and a run segment carries an empty load object. A run starts at time zero at the radius given, with the wall at the speed this impact gives it there; 1.75 m is 0.917 s after the impact by the card's clock, and the worked example starts there. A single step starts in the same way at each call: its radius_m handed back as a new start goes on with the wall at the speed the card sets from that radius, not the speed the earlier steps integrated, and t_s begins again at zero; through from, a continuation is served from the crater stopped and refused before it (ContinuationNotOnCard). The radius grows and never shrinks. On the step in which the wall would turn back, the crater is set at 8.571304329477922 m with the wall at rest, the frame carries FLAG_STOPPED, and from then on a step moves the time alone. From 1.75 m at the step ceiling that is step 1,597, at 399.25 s. The time is the clock of Gault and Wedekind, 1977, which runs until the ejecta near the rim have landed; no time of Ryugu's crater is in the comparison.".into(),
        &[
            "NonFiniteInput",
            "NonFiniteState",
            "NonPositiveDt",
            "InvalidCrater",
            "RadiusBelowLedgerResolution",
            "RadiusAboveFinal",
            "WallMovingInward",
            "SpontaneousGeneration",
            "FirstLawBreach",
        ],
        vec![Advisory {
            name: "FLAG_STOPPED",
            mask: crater_ryugu_sci::FLAG_STOPPED,
            seals: true,
        }],
        slots(&[
            ("t", "Time since the start of the run, in seconds."),
            ("pos[0]", "radius_m"),
            ("pos[1]", "wall_speed_m_s"),
            ("pos[2]", "radius_over_final"),
            ("vel[0]", "kinetic_j_per_kg"),
            ("vel[1]", "given_up_j_per_kg"),
            ("vel[2]", "energy_defect_j_per_kg"),
            ("force_torque", "radius_to_go_m"),
            ("residual", "Conservation residual."),
            (
                "flags",
                "FLAG_STOPPED: the crater has stopped growing, at 8.571304329477922 m with its wall at rest. It seals.",
            ),
            ("proof", "Not written by the step."),
        ]),
    )
}

fn ti64() -> Card {
    let cell = ti64_lattice::Ti64LatticeCell::annealed_column();
    let omega = cell.max_natural_frequency_rad_s();
    let (gate, nyquist) = clock(omega, cell.gate_dt_s(), None);
    let minimum = ti64_lattice::CARD_TEMPERATURE_MIN_K;
    let maximum = ti64_lattice::BETA_TRANSUS_K;
    let floor = cell.floor_dt_s().expect("specimen floor");
    let yield_position_m = ti64_yield_position_m(&cell);
    let (lowest_m, highest_m) = ti64_position_window_m(&cell);
    let yield_speed_m_s = ti64_yield_speed_m_s(&cell);
    let held_speed_m_s = 2.0 * omega * yield_position_m;
    let yield_force_n = ti64_yield_force_n(&cell);
    let growth_m = highest_m - lowest_m - 2.0 * yield_position_m;
    let mut position = vec3_within(
        "position_m",
        "m",
        highest_m,
        format!(
            "each component from {0} to {1} m, both inclusive. The bound is the specimen's yield over the card's temperatures: a node above {1} m stretches its ligament past 880 MPa at every temperature up to the beta transus, 1268.15 K, and a node below {0} m compresses it past 880 MPa at every temperature down to 250 K, and the card refuses either as YieldReached. Inside the bound the first step tests yield on the incoming state at its start_temperature_k: at the reference temperature, 293.15 K, a ligament whose node stands further out than {2} m either way is past 880 MPa, and that start is refused as YieldReached on the first step",
            token(lowest_m),
            token(highest_m),
            token(yield_position_m)
        ),
    );
    position.minimum = Some(lowest_m);
    finish(
        "materials",
        "ti64_lattice",
        "Company specimen: a three-strut annealed Ti-6Al-4V lattice column, yield 880 MPa, modulus 113.8 GPa, under nodal force, torque and temperature to 1268.15 K.",
        vec![
            position,
            vec3_within(
                "velocity_m_s",
                "m/s",
                yield_speed_m_s,
                format!(
                    "each component from -{0} to {0} m/s, both inclusive. The bound is the highest speed along a ligament that a run under forces and temperatures inside their ranges can approach, from rest or from a turning point, before that ligament reaches 880 MPa: 2 omega sqrt(x_y (x_y + g)), with omega the highest natural frequency, x_y = {1} m the yield position at 293.15 K and g = {2} m the growth of a ligament heated from 250 K to 1268.15 K. It is approached with the largest force pulling and the ligament heated as the node moves, so that its stretch stays at yield in compression up to the beta transus and then runs out to yield in tension. With the temperature held, the highest is 2 omega x_y, {3} m/s; a run of the largest force from -x_y lands 45.2537 m/s. A faster node cannot come to rest before that ligament reaches 880 MPa under any force and temperature in the range, and the card refuses it as YieldReached. A position and a velocity each inside its range can together still carry a ligament past yield during a run: that step is refused as YieldReached, on the incoming state, at a turning point of the motion inside the step, or on the landing, and the run is sealed as refused",
                    token(yield_speed_m_s),
                    token(yield_position_m),
                    token(growth_m),
                    token(held_speed_m_s)
                ),
            ),
            scalar(
                "start_t_s",
                Role::Phase,
                "s",
                Some(0.0),
                false,
                None,
                false,
                "from 0 s up, inclusive: the clock of the start. A new start states 0. A state the card returned is handed back with its t_s as start_t_s, and the clock runs on from it as a running sum, t_s += dt_s. A clock below zero is refused as StartNotWritten. With start_force_n, start_temperature_k, start_energy_anchor_j and start_boundary_work_j it carries the run across a restart: a run handed back its own returned state with these five values steps on exactly as the run straight through, bit for bit in every field but torque_nm, which is the applied couple of the last step, and its ledger is weighed against the conservation limit over the whole of it. The card cannot tell the values a run returned from values a caller states: any start whose start_energy_anchor_j and start_boundary_work_j leave a defect inside the conservation limit is accepted by the card and weighed from that defect, a returned state restated as new (start_t_s 0, start_boundary_work_j 0, start_energy_anchor_j its own energy) among them, and its proof carries the values stated. A whole chain's ledger is weighed as one when each start hands back the values the card returned, and not otherwise.".to_string(),
            ),
            vec3_within(
                "start_force_n",
                "N",
                yield_force_n,
                format!(
                    "each component from -{0} to {0} N, both inclusive, the range of force_n: the force the start stands under, which a returned state gives as applied_force_n. A new start states the force its start_energy_anchor_j is taken under; the card's rest has none. It enters the ledger and nothing else: the first step books the change from it to the step's own force_n as boundary work",
                    token(yield_force_n)
                ),
            ),
            scalar(
                "start_temperature_k",
                Role::Phase,
                "K",
                Some(minimum),
                false,
                Some(maximum),
                false,
                format!(
                    "inclusive [{}, {}] K, the range of temperature_k: the temperature the start stands at, which a returned state gives as temperature_k. Yield is tested on the incoming state at it, and the first step books the change from it to the step's own temperature_k as boundary work. The card's rest is at the reference temperature, 293.15 K. A value outside is refused as TemperatureOutsideCard",
                    token(minimum),
                    token(maximum)
                ),
            ),
            scalar(
                "start_energy_anchor_j",
                Role::Phase,
                "J",
                None,
                false,
                None,
                false,
                format!(
                    "no bound of its own: the energy the start's ledger is anchored at, which a returned state gives as energy_anchor_j. A new start states its own mechanical energy under start_force_n and start_temperature_k: 1/2 m |v|^2 plus, over the three ligaments, 1/2 k (x_i - alpha (T - 293.15 K) L)^2 - F_i x_i, with m = {} kg, k = {} N/m, L = 0.02 m and alpha = 8.6e-6 /K. At rest at 293.15 K with no force it is 0. The ledger's defect is the start's mechanical energy less start_energy_anchor_j and start_boundary_work_j; a start whose defect is past the conservation limit, 1e-6 of the yield energy of {} J, could not have been written by the card and is refused as StartNotWritten",
                    token(cell.mass_kg()),
                    token(cell.stiffness_n_m()[0]),
                    token(cell.yield_energy_scale_j())
                ),
            ),
            scalar(
                "start_boundary_work_j",
                Role::Phase,
                "J",
                None,
                false,
                None,
                false,
                "no bound of its own: the boundary work booked since the anchor, which a returned state gives as boundary_work_j. A new start states 0. A start with start_t_s 0 and any other value is refused as StartNotWritten: no step has landed to book it".to_string(),
            ),
            vec3_within(
                "force_n",
                "N",
                yield_force_n,
                format!(
                    "each component from -{0} to {0} N, both inclusive. The bound is the specimen's yield: the yield strength, 880 MPa, times the section of a ligament, the force that holds a ligament at yield at rest. The load rule of dt_s also holds; at every step the card accepts it allows more than this maximum, so the maximum is reached first. A force inside the range, held from rest, carries the node out to twice its rest deflection: above half the maximum that excursion goes past yield, the step that holds it is refused as YieldReached, at its turning point if no landing is past yield first, and the run is sealed as refused",
                    token(yield_force_n)
                ),
            ),
            vec3_stated(
                "torque_nm",
                "N*m",
                "no bound. The specimen is concentric: the torque is reported in the state and does not enter the motion, the stress or the ledger. Any number the number rule admits is accepted and returned".to_string(),
            ),
            scalar(
                "temperature_k",
                Role::Load,
                "K",
                Some(minimum),
                false,
                Some(maximum),
                false,
                format!(
                    "inclusive [{}, {}] K. Yield is tested on the incoming state at its start_temperature_k and again at the instant this temperature is applied. From the rest position the cell yields from heat alone above about 1192.32 K, so a higher temperature is refused as YieldReached unless the start position is part of the way out. The published maximum is inclusive and is answered, but never from rest with no force: a step at exactly 1268.15 K lands from a start position of 3e-5 m in each component, where the ligaments are already stretched, and from the rest position the largest temperature that lands is 1192.3203110311847 K. A start position at rest when hot and not when cold is past yield at 293.15 K: stated with a start_temperature_k of 293.15 K it is refused on the first step, and stated with its own hot start_temperature_k it lands",
                    token(minimum),
                    token(maximum)
                ),
            ),
        ],
        floor,
        "the ceiling divided by 2^20: a step that changes the state by two parts in a hundred million. Under it a step is mostly rounding",
        gate,
        "0.02 rad on its highest natural frequency",
        nyquist,
        format!(
            "{}. omega is max_natural_frequency_rad_s, the shortest ligament and the mass are the cell's, and each of the three components of force_n is held to the rule alone",
            impulse_rule(
                "force_n component",
                "omega * shortest ligament * mass",
                super::ti64_impulse_limit_n_s(&cell).expect("specimen impulse limit")
            )
        ),
        &[
            "NonFiniteInput",
            "NonFiniteState",
            "NonPositiveDt",
            "TemperatureOutsideCard",
            "InvalidCell",
            "YieldReached",
            "SpontaneousGeneration",
            "FirstLawBreach",
            "StartNotWritten",
        ],
        vec![],
        slots(&[
            ("t", "The clock: the start's t_s plus the steps since, in seconds."),
            ("pos[0]", "position_m[0]"),
            ("pos[1]", "position_m[1]"),
            ("pos[2]", "position_m[2]"),
            ("vel[0]", "velocity_m_s[0]"),
            ("vel[1]", "velocity_m_s[1]"),
            ("vel[2]", "velocity_m_s[2]"),
            ("force_torque", "Dominant ligament tension, in newtons."),
            ("residual", "Conservation residual."),
            (
                "flags",
                "Always zero. The card has no advisory condition: yield and a residual past the limit are refusals.",
            ),
            ("proof", "Not written by the step."),
        ]),
    )
}

fn grid() -> Card {
    let node = grid_node::GridNode::copper_fuse_link();
    let omega = node.natural_frequency_rad_s();
    let (gate, nyquist) = clock(omega, node.gate_dt_s(), None);
    let minimum = grid_node::CARD_TEMPERATURE_MIN_K;
    let melt = grid_node::COPPER_MELT_K;
    let floor = node.floor_dt_s().expect("specimen floor");
    let melt_current_a = grid_melt_current_a(&node);
    let melt_voltage_v = grid_melt_voltage_v(&node);
    finish(
        "energy",
        "grid_node",
        "Company specimen: a 48 V DC-plant node, a 50 uH feeder into a 50 uF shunt that rings at 20 krad/s, with an AWG 34 copper fuse link 0.1601 mm across and 120 mm long that melts at 1357.77 K, between an upstream voltage and a load current. It holds an energy ledger of the feeder, the shunt and the copper's heat.",
        vec![
            within(
                "current_a",
                Role::Phase,
                "A",
                melt_current_a,
                format!(
                    "from -{0} to {0} A, both inclusive. The bound is the specimen's energy bound, not a melt: the current whose energy in the feeder inductance equals the heat that takes the link from the reference temperature, 293.15 K, to its melting point, 1357.77 K. A feeder that holds more has more energy than the link can take as heat without melting; whether the link melts is CopperMelt. A step that lands with |current_a| above the bound is refused as CurrentOutsideCard and that state is not sealed",
                    token(melt_current_a)
                ),
            ),
            within(
                "voltage_v",
                Role::Phase,
                "V",
                melt_voltage_v,
                format!(
                    "from -{0} to {0} V, both inclusive. The bound is the specimen's energy bound, not a melt: the voltage whose energy in the shunt capacitance equals the heat that takes the link from the reference temperature, 293.15 K, to its melting point, 1357.77 K. A step that lands with |voltage_v| above the bound is refused as VoltageOutsideCard and that state is not sealed. Inside the range a node voltage at or above 120 V is flagged FLAG_GRID_SURGE and still seals",
                    token(melt_voltage_v)
                ),
            ),
            scalar(
                "temperature_k",
                Role::Phase,
                "K",
                Some(minimum),
                false,
                Some(melt),
                true,
                format!("inclusive minimum {} K, exclusive maximum {} K", token(minimum), token(melt)),
            ),
            within(
                "upstream_voltage_v",
                Role::Load,
                "V",
                melt_voltage_v,
                format!(
                    "from -{0} to {0} V, both inclusive: the bound of voltage_v, because the node settles at the upstream voltage that is held. A held voltage well inside the range can ring the node past the bound on the way: from rest, a held voltage above about 0.55 of the bound does. That step is refused as VoltageOutsideCard and that state is not sealed",
                    token(melt_voltage_v)
                ),
            ),
            within(
                "load_current_a",
                Role::Load,
                "A",
                melt_current_a,
                format!(
                    "from -{0} to {0} A, both inclusive: the bound of current_a, because the feeder settles at the load current that is held. A held current well inside the range can ring the node past a bound on the way: from rest, a held current above about 0.58 of the bound rings the feeder past the bound of current_a, and that step is refused as CurrentOutsideCard; at the bound the node voltage swings past the bound of voltage_v first, and that step is refused as VoltageOutsideCard. Neither state is sealed. A held current far inside the range still melts the link in time, and that step is refused as CopperMelt",
                    token(melt_current_a)
                ),
            ),
            scalar(
                "ambient_k",
                Role::Load,
                "K",
                Some(minimum),
                false,
                Some(melt),
                true,
                format!(
                    "inclusive minimum {} K, exclusive maximum {} K",
                    token(minimum),
                    token(melt)
                ),
            ),
        ],
        floor,
        "the ceiling divided by 2^20: a step that changes the state by two parts in a hundred million. Under it a step is mostly rounding",
        gate,
        "0.02 rad on its natural frequency",
        nyquist,
        "No scalar load Nyquist. The step refuses when the fuse temperature reaches the melt point. energy_anchor_j, boundary_work_j, energy_defect_j and the residual are counted from the state this request starts from, and voltage_spike_v is the largest |voltage_v| since that start: a restart from a returned state starts all of them again, so they can differ from a straight run while current_a, voltage_v and temperature_k agree. No limit depends on voltage_spike_v, and the conservation limit is held on each step from the new start.".into(),
        &[
            "NonFiniteInput",
            "NonFiniteState",
            "NonPositiveDt",
            "NegativeTemperature",
            "TemperatureOutsideCard",
            "InvalidNode",
            "CopperMelt",
            "SpontaneousGeneration",
            "FirstLawBreach",
            "CurrentOutsideCard",
            "VoltageOutsideCard",
        ],
        vec![Advisory {
            name: "FLAG_GRID_SURGE",
            mask: grid_node::FLAG_GRID_SURGE,
            seals: true,
        }],
        slots(&[
            ("t", "Time since the anchor, in seconds."),
            ("pos[0]", "current_a"),
            ("pos[1]", "voltage_v"),
            ("pos[2]", "temperature_k"),
            ("vel[0]", "joule_power_w"),
            ("vel[1]", "thermal_energy_j"),
            ("vel[2]", "voltage_spike_v"),
            ("force_torque", "Melt margin, in kelvin."),
            ("residual", "Conservation residual."),
            ("flags", "FLAG_GRID_SURGE when |voltage| is at the surge withstand. It seals."),
            ("proof", "Not written by the step."),
        ]),
    )
}

fn swing() -> Card {
    let machine = swing::SwingMachine::synchronous_60hz();
    let (gate, nyquist) = clock(
        machine.omega_electrical_rad_s,
        machine.gate_dt_s(),
        machine.nyquist_dt_s(),
    );
    let half = std::f64::consts::FRAC_PI_2;
    let floor = machine.floor_dt_s().expect("specimen floor");
    finish(
        "energy",
        "swing",
        "Company specimen: one round-rotor synchronous machine on a 60 Hz bus, inertia constant 5 s, damping 0.05 pu, peak electrical power 1.04 pu and a phase-locked loop of 20 per second, under mechanical power, held in synchronism inside a rotor angle of plus or minus pi/2.",
        vec![
            scalar(
                "delta_rad",
                Role::Phase,
                "rad",
                Some(-half),
                true,
                Some(half),
                true,
                format!("|delta| < {}", token(half)),
            ),
            scalar(
                "delta_omega_rad_s",
                Role::Phase,
                "rad/s",
                Some(-swing::SPEED_DEVIATION_LIMIT_RAD_S),
                false,
                Some(swing::SPEED_DEVIATION_LIMIT_RAD_S),
                false,
                format!("|delta_omega| <= {}", token(swing::SPEED_DEVIATION_LIMIT_RAD_S)),
            ),
            scalar(
                "pll_err_rad",
                Role::Phase,
                "rad",
                None,
                false,
                None,
                false,
                "no bound. The PLL error is carried beside the rotor and does not enter the rotor's motion or the ledger. At or beyond 20 degrees the frame is flagged FLAG_PLL_SLIP and still seals".to_string(),
            ),
            scalar(
                "p_mech_pu",
                Role::Load,
                "pu",
                Some(-swing::DISPATCH_LIMIT_PU),
                false,
                Some(swing::DISPATCH_LIMIT_PU),
                false,
                format!("|p_mech| <= {}", token(swing::DISPATCH_LIMIT_PU)),
            ),
        ],
        floor,
        "the ceiling divided by 2^20: a step that changes the state by two parts in a hundred million. Under it a step is mostly rounding",
        gate,
        "0.02 rad on its electrical frequency",
        nyquist,
        "No scalar load Nyquist. The rotor must start inside +/- pi/2, with a speed deviation of at most 6 pi rad/s and a dispatch of at most 2 pu either way. The transient energy, transient_energy_pu, is U = 1/2 M delta_omega^2 - p_mech delta - 1.04 cos delta, with M = 2 x 5 s / (2 pi 60 rad/s). A run starts at time zero with its ledger anchored at the angle, the speed and the dispatch given: energy_anchor_pu is U at that angle and speed under the p_mech_pu of the first segment, and boundary_work_pu starts at zero. A first step held at that dispatch books no change of dispatch. boundary_work_pu is the energy that has entered the rotor since the anchor: the damping, minus 0.05 pu per rad/s times delta_omega squared, integrated over each step, and at each later change of dispatch the change of U it makes at the angle before the step, -(new p_mech - old p_mech) x delta. energy_defect_pu is transient_energy_pu - energy_anchor_pu - boundary_work_pu, the energy the integration created (positive) or lost (negative); residual_conservation_error is its size over the synchronism barrier, the gap in U from the stable to the unstable equilibrium at 1 pu. A step whose defect is above 1e-6 of the barrier is refused as SpontaneousGeneration, and one below -1e-6 of it as FirstLawBreach. A single step starts in the same way at each call: its delta_rad, delta_omega_rad_s and pll_err_rad handed back as a new start continue the same motion, and t_s, the anchor, the boundary work and the residual begin again. No refusal depends on the ledger across calls: the conservation limit weighs each step against the run it is in.".into(),
        &[
            "NonFiniteInput",
            "NonFiniteState",
            "NonPositiveDt",
            "InvalidMachine",
            "LossOfSynchronism",
            "SpeedDeviationOutsideCard",
            "DispatchOutsideCard",
            "SpontaneousGeneration",
            "FirstLawBreach",
        ],
        vec![Advisory {
            name: "FLAG_PLL_SLIP",
            mask: swing::FLAG_PLL_SLIP,
            seals: true,
        }],
        slots(&[
            ("t", "Time since the anchor, in seconds."),
            ("pos[0]", "delta_rad"),
            ("pos[1]", "delta_omega_rad_s"),
            ("pos[2]", "pll_err_rad"),
            ("vel[0]", "p_elec_pu"),
            ("vel[1]", "p_mech_pu"),
            ("vel[2]", "transient_energy_pu"),
            ("force_torque", "Synchronism margin, in radians."),
            ("residual", "Conservation residual."),
            ("flags", "FLAG_PLL_SLIP. It seals."),
            ("proof", "Not written by the step."),
        ]),
    )
}

fn battery() -> Card {
    let pack = battery_sag::LiIonPack::pack_12s();
    let (gate, nyquist) = clock(pack.pole_rad_s(), pack.gate_dt_s(), None);
    let impulse = pack.charge_impulse_c();
    // The largest charging current any state of charge could take before the
    // terminal voltage lands above the charging limit: at an empty pack.
    let charge_current_limit_a =
        (pack.charge_limit_v - pack.open_circuit_voltage(0.0)) / pack.resistance_ohm(0.0);
    let soc_floor = battery_soc_floor(&pack);
    let floor = pack.floor_dt_s().expect("specimen floor");
    finish(
        "energy",
        "battery_sag",
        "Company specimen: a 12S lithium-ion pack of 22 Ah, 50.4 V full and 36.0 V empty, with a straight-line rest voltage and a 0.060 ohm resistance that doubles as it empties, under a terminal current. It holds a chemical-energy ledger, a 42.0 V warning, a 36.0 V cutoff and a 50.4 V charging limit.",
        vec![
            scalar(
                "soc",
                Role::Phase,
                "1",
                Some(soc_floor),
                false,
                Some(1.0),
                false,
                format!(
                    "inclusive minimum {}, inclusive maximum 1. The minimum is the lowest state of charge at which the open-circuit terminal voltage, {} V + {} V * soc, is above the {} V cutoff; a lower value is refused. A step from a low state of charge lands if, and not unless, the landing state of charge, soc - current_a * dt_s / {} C, still has a terminal voltage above the cutoff: with a discharging current the lowest usable soc is higher than the minimum and rises with current_a and dt_s (UndervoltageCutoff). For any current the terminal voltage at the landing state of charge s' is {} V + {} V * s' - current_a * {} ohm * (2 - s'), with current_a positive when discharging",
                    token(soc_floor),
                    token(pack.empty_v),
                    token(pack.full_v - pack.empty_v),
                    token(pack.cutoff_v),
                    token(pack.capacity_c),
                    token(pack.empty_v),
                    token(pack.full_v - pack.empty_v),
                    token(pack.series_resistance_ohm)
                ),
            ),
            scalar(
                "current_a",
                Role::Load,
                "A",
                Some(-charge_current_limit_a),
                true,
                None,
                false,
                format!(
                    "The largest charging current (negative) that is accepted depends on the state of charge and on dt_s. The published exclusive minimum -{} A is approached as the state of charge goes to zero and nowhere else; at a full pack no charging current is accepted: at soc 1 every negative current_a, however small, is refused as OvervoltageCutoff. The rule: a charging current of size I at state of charge s and step dt_s lands when, with the landing state of charge s' = s + I * dt_s / {} C, {} V + {} V * s' + I * {} ohm * (2 - s') is at most {} V (OvervoltageCutoff). The table charging_current_limit_by_state_of_charge gives the largest accepted size, generated from the card, for a step of {} s. Both signs are bound by the load rule: {}",
                    token(charge_current_limit_a),
                    token(pack.capacity_c),
                    token(pack.empty_v),
                    token(pack.full_v - pack.empty_v),
                    token(pack.series_resistance_ohm),
                    token(pack.charge_limit_v),
                    token(BATTERY_TABLE_DT_S),
                    impulse_rule("current_a", "charge_impulse_c", impulse)
                ),
            ),
        ],
        floor,
        "the ceiling divided by 2^20: a step that changes the state by two parts in a hundred million. Under it a step is mostly rounding",
        gate,
        "0.02 rad on its pole, 1/(R0 C)",
        nyquist,
        impulse_rule("current_a", "charge_impulse_c", impulse),
        &[
            "NonFiniteInput",
            "NonFiniteState",
            "NonPositiveDt",
            "InvalidPack",
            "CurrentAboveNyquist",
            "UndervoltageCutoff",
            "OvervoltageCutoff",
            "StateOfChargeOutsideCard",
            "SpontaneousGeneration",
            "FirstLawBreach",
        ],
        vec![Advisory {
            name: "FLAG_SAG_WARNING",
            mask: battery_sag::FLAG_SAG_WARNING,
            seals: true,
        }],
        slots(&[
            ("t", "Time since the anchor, in seconds."),
            ("pos[0]", "terminal_v"),
            ("pos[1]", "soc"),
            ("pos[2]", "current_a"),
            ("vel[0]", "open_circuit_v"),
            ("vel[1]", "chemical_energy_j"),
            ("vel[2]", "joule_j"),
            ("force_torque", "Cutoff margin, in volts."),
            ("residual", "Conservation residual."),
            ("flags", "FLAG_SAG_WARNING. It seals."),
            ("proof", "Not written by the step."),
        ]),
    )
}

/// The step, in seconds, the table of charging limits is generated for.
pub const BATTERY_TABLE_DT_S: f64 = 0.1;

/// The states of charge the table of charging limits lists.
pub const BATTERY_TABLE_SOC: [f64; 9] = [0.001, 0.01, 0.1, 0.25, 0.5, 0.75, 0.9, 0.95, 0.99];

/// The lowest state of charge the pack can be anchored at: the smallest
/// 64-bit number for which the card builds a state. Found on the card's own
/// `try_soc`; below it the open-circuit terminal voltage is at or under the
/// cutoff.
pub fn battery_soc_floor(pack: &battery_sag::LiIonPack) -> f64 {
    let mut refused = 0.0_f64.to_bits();
    let mut built = 1.0_f64.to_bits();
    while built - refused > 1 {
        let middle = refused + (built - refused) / 2;
        if battery_sag::PackState::try_soc(pack, f64::from_bits(middle)).is_some() {
            built = middle;
        } else {
            refused = middle;
        }
    }
    f64::from_bits(built)
}

/// Whether the card's step lands a charging current of this size from this
/// state of charge at this step.
pub fn battery_charge_lands(pack: &battery_sag::LiIonPack, soc: f64, size_a: f64, dt_s: f64) -> bool {
    let Some(mut state) = battery_sag::PackState::try_soc(pack, soc) else {
        return false;
    };
    let mut frame = super::blank_frame([0; 16]);
    battery_sag::step(
        pack,
        &mut state,
        &mut frame,
        battery_sag::PackLoad { current_a: -size_a },
        dt_s,
    )
    .is_ok()
}

/// The largest charging current size the step lands from this state of
/// charge at this step, found on the 64-bit numbers: the pair of the table.
pub fn battery_largest_charge_a(pack: &battery_sag::LiIonPack, soc: f64, dt_s: f64) -> f64 {
    let mut lands = 0.0_f64.to_bits();
    let mut refused = (pack.charge_impulse_c() / dt_s).to_bits();
    while refused - lands > 1 {
        let middle = lands + (refused - lands) / 2;
        if battery_charge_lands(pack, soc, f64::from_bits(middle), dt_s) {
            lands = middle;
        } else {
            refused = middle;
        }
    }
    f64::from_bits(lands)
}

fn battery_charging_table(pack: &battery_sag::LiIonPack) -> Value {
    let rows = BATTERY_TABLE_SOC
        .iter()
        .map(|&soc| {
            let largest = battery_largest_charge_a(pack, soc, BATTERY_TABLE_DT_S);
            json!({
                "soc": exact_value(soc),
                "largest_charging_current_size_a": exact_value(largest),
                "current_a": exact_value(-largest),
            })
        })
        .collect::<Vec<_>>();
    json!({
        "dt_s": exact_value(BATTERY_TABLE_DT_S),
        "meaning": "For each state of charge, the largest charging current size that the step accepts at this dt_s, and the current_a to send. The next representable size beyond it is refused as OvervoltageCutoff. At a full pack (soc 1) no charging current is accepted.",
        "rows": rows,
    })
}

fn cell_18650pf_card() -> Card {
    let specimen = cell_18650pf::Cell18650Pf::cell();
    // No pole on the card, so no `π/ω`.
    let gate = specimen.gate_dt_s().expect("specimen gate");
    let floor = specimen.floor_dt_s().expect("specimen floor");
    finish(
        "energy",
        "cell_18650pf",
        "One Panasonic NCR18650PF cell in a held discharge at one current, from a rested cell at any state of charge, with no temperature and no memory of the load before. Compared on eleven held 1C discharges from full charge, 2.899 to 2.89982 A: all eleven within 50 mV, 27.8 to 30.0 mV rms. At 0.87 A, compared on 2026-10-10, it missed: 56.7 mV rms, and it ran 71 s past the cell's 2.5 V. A step at any other current lands with FLAG_CURRENT_OUTSIDE_PASSED_SPAN.",
        vec![
            scalar(
                "soc",
                Role::Phase,
                "1",
                Some(cell_18650pf::SOC_MIN),
                false,
                Some(1.0),
                false,
                "inclusive minimum 0.03, inclusive maximum 1: the state of charge of a rested cell, or of a discharge continued at the held current held_current_a states".into(),
            ),
            scalar(
                "held_current_a",
                Role::Phase,
                "A",
                Some(0.0),
                true,
                None,
                false,
                "above 0 A (CurrentNotDischarging): the held current of the discharge this run belongs to: for a run from a rested cell, the current it is held at; for a run from a state an earlier run returned, that run's held current. The step ceiling is one table interval at the larger of it and 3.0 A".into(),
            ),
            scalar(
                "current_a",
                Role::Load,
                "A",
                Some(0.0),
                true,
                None,
                false,
                "above 0 A, a discharge (CurrentNotDischarging), within 0.83 mA of held_current_a. The compared 1C span is 2.89817 to 2.90065 A, the measured 2.899 to 2.89982 A widened on each side by the 0.83 mA band; any other current sets FLAG_CURRENT_OUTSIDE_PASSED_SPAN".into(),
            ),
        ],
        floor,
        "the ceiling divided by 2^20: at 3.0 A, five parts in a thousand million of state of charge. Under it a step is mostly rounding",
        gate,
        "the time to cross one table interval at 3.0 A, the value here, for every held current at or below 3.0 A; above it, the time to cross one at the run's own held current, and a step above that is refused as DtAboveCeiling with the ceiling stated",
        None,
        "current_a is any discharge current above 0 A. The equations, the charge count and the ledger are the same at every current. Compared on eleven held 1C discharges from full charge, fed the tester's logged current, 2.899 to 2.89982 A: all eleven within 27.8 to 30.0 mV rms (target 50 mV), largest single error 57 mV, capacity +0.04 percent on the one run that reached cutoff. One discharge at another current was left out of the fitting: 04-27-17 3541_dis5_10p.mat, the same cell at 25 degrees Celsius discharged at a held 0.86888 to 0.86970 A from full charge to 2.5 V in 14 holds with rests at 0 A between them. Each hold run as a continuation of the one before, the rests left out, at steps of 0.1 s: the voltage error during the holds is 56.7 millivolts root mean square (target 50), rising to 204 and 206 in the last two holds, and the card does not end by its own cutoff but leaves its table at soc 0.03 and 2.66 V (StateOfChargeOutsideCard), 71 s after the cell reached 2.5 V, having delivered 0.60 percent more than the cell. The rest-voltage table is fitted at 0.145 A and the resistance on 0.87 A holds; no other current is in the record. A step at a current outside the 1C span lands with FLAG_CURRENT_OUTSIDE_PASSED_SPAN. From about 1.1 A up the 2.5 V cutoff ends a discharge (UndervoltageCutoff); below it the bottom of the table, soc 0.03, comes first (StateOfChargeOutsideCard). A current of zero or less, a rest or a charge, is refused as CurrentNotDischarging: the card has no charging limit at 4.2 V and no relaxation for a rest after a discharge. A current that is not a discharge in any segment of a run refuses the whole run before its first step. A run is one discharge at one held current: every segment's current_a must lie within 0.00083 A (HELD_CURRENT_BAND_A) of the held current, and a segment outside that band refuses the whole run before its first step as CurrentChangedDuringDischarge, with nothing sealed. The held current is held_current_a, which the request states. A returned state does not carry held_current_a; to continue from it, the request states the same held_current_a as the run that returned it. Held means what the tester held: the measured current moves 0.82 mA within a run, one step of the 0.81 mA in which the tester logs its current; the dataset states no accuracy for the tester, so the band is that spread, written as 0.83 mA, and the test allows four units in the last place at the size of the currents. A pulse and a drive cycle are refused: the card has no memory, the real cell's voltage relaxes for minutes after a change of load, and no changing current was compared with a measurement of this cell. Every compared 1C discharge started at full charge, soc 1; the 0.87 A comparison ran its holds as continuations from partial charge. A start below full charge is a rested cell at the rest-voltage table's own value, or a continuation; the two are the same physics on this card, which has no memory. A run continued from a state another run returned, at the same held current, gives the same state of charge and terminal voltage as the straight run; the ledger's anchor, time and work start again at the continuation, and no limit depends on them. A single step takes the state of charge and held_current_a alone: time, work and the ledger's anchor start again at each call. A run whose end in decimal is exactly the bottom table point, soc 0.03, lands its last step: a state of charge summed in binary that lands under 0.03 by no more than one unit in the last place at 1 for each step since the run or continuation began, plus four, lands on 0.03. At 1C the 2.5 V cutoff stops a discharge near soc 0.05, before that point. A step of zero or less is refused as NonPositiveDt before the step floor is tested.".into(),
        &[
            "NonFiniteInput",
            "NonFiniteState",
            "NonPositiveDt",
            "InvalidCell",
            "CurrentNotDischarging",
            "CurrentChangedDuringDischarge",
            "StateOfChargeOutsideCard",
            "UndervoltageCutoff",
            "SpontaneousGeneration",
            "FirstLawBreach",
        ],
        vec![
            Advisory {
                name: "FLAG_SAG_WARNING",
                mask: cell_18650pf::FLAG_SAG_WARNING,
                seals: true,
            },
            Advisory {
                name: "FLAG_CURRENT_OUTSIDE_PASSED_SPAN",
                mask: cell_18650pf::FLAG_CURRENT_OUTSIDE_PASSED_SPAN,
                seals: true,
            },
        ],
        slots(&[
            ("t", "Time since the anchor, in seconds."),
            ("pos[0]", "terminal_v"),
            ("pos[1]", "soc"),
            ("pos[2]", "current_a"),
            ("vel[0]", "open_circuit_v"),
            ("vel[1]", "chemical_energy_j"),
            ("vel[2]", "joule_j"),
            ("force_torque", "Cutoff margin, in volts."),
            ("residual", "Conservation residual."),
            ("flags", "FLAG_SAG_WARNING under 3.0 V; FLAG_CURRENT_OUTSIDE_PASSED_SPAN when the step's current is outside 2.89817 to 2.90065 A. Both seal."),
            ("proof", "Not written by the step."),
        ]),
    )
}

fn thermal_plate() -> Card {
    let plate = thermal::ThermalNode::avionics_cold_plate();
    let (gate, nyquist) = clock(plate.pole_rad_s(), plate.gate_dt_s(), None);
    let minimum = thermal::CARD_TEMPERATURE_MIN_K;
    let maximum = thermal::CARD_TEMPERATURE_MAX_K;
    let window = format!(
        "inclusive minimum {} K, exclusive maximum {} K",
        token(minimum),
        token(maximum)
    );
    let floor = plate.floor_dt_s().expect("specimen floor");
    finish(
        "energy",
        "thermal",
        "Company specimen: an aluminum 6061 avionics cold plate, 250 x 125 x 7.8 mm, heat capacity 590.625 J/K and 2 K/W to its bath, under heater power and a bath temperature, stepped by the exact exponential map from -40 to 125 degrees Celsius, the industrial rating of the electronics it carries.",
        vec![
            scalar(
                "temperature_k",
                Role::Phase,
                "K",
                Some(minimum),
                false,
                Some(maximum),
                true,
                window.clone(),
            ),
            scalar(
                "heat_power_w",
                Role::Load,
                "W",
                None,
                false,
                None,
                false,
                "no bound of its own. Positive is heat into the plate and negative is heat out. The plate's temperature is what the card holds: a power that takes the plate out of its temperature range in a step is refused as TemperatureOutsideCard, or as NegativeTemperature when the step would land at or below zero kelvin".to_string(),
            ),
            scalar(
                "ambient_k",
                Role::Load,
                "K",
                Some(minimum),
                false,
                Some(maximum),
                true,
                window,
            ),
        ],
        floor,
        "the ceiling divided by 2^20: a step that moves the plate about two parts in a hundred million of its distance to equilibrium. When the plate is a kelvin or more from equilibrium a floor step changes the temperature by 1.9e-8 K or more. Closer to equilibrium a step's change can round to zero. The temperature is a 64-bit number, so each step lands within 2.2737367544323206e-13 K (2^-42 K) of the exact map of the temperature it starts from, and the boundary heat booked is the capacitance times the rise that landed. N steps and one step over the same span differ by at most N + 1 times 2.2737367544323206e-13 K",
        gate,
        "0.02 rad on its pole, 1/(RC)",
        nyquist,
        "No scalar load Nyquist. Plate and bath stay inside the temperature window.".into(),
        &[
            "NonFiniteInput",
            "NonFiniteState",
            "NonPositiveDt",
            "NegativeTemperature",
            "TemperatureOutsideCard",
            "InvalidNode",
            "SpontaneousGeneration",
            "FirstLawBreach",
        ],
        vec![Advisory {
            name: "FLAG_THERMAL_DERATE",
            mask: thermal::FLAG_THERMAL_DERATE,
            seals: true,
        }],
        slots(&[
            ("t", "Time since the anchor, in seconds."),
            ("pos[0]", "temperature_k"),
            ("pos[1]", "ambient_k"),
            ("pos[2]", "heat_power_w"),
            ("vel[0]", "internal_energy_j"),
            ("vel[1]", "boundary_work_j"),
            ("vel[2]", "heat_to_ambient_w"),
            ("force_torque", "Ceiling margin, in kelvin."),
            ("residual", "Conservation residual."),
            ("flags", "FLAG_THERMAL_DERATE. It seals."),
            ("proof", "Not written by the step."),
        ]),
    )
}

fn resonance_mount() -> Card {
    let oscillator = resonance::DynamicOscillator::mount_10hz();
    let (gate, nyquist) = clock(
        oscillator.omega_rad_s,
        oscillator.gate_dt_s(),
        oscillator.nyquist_dt_s(),
    );
    let (largest_displacement_m, largest_velocity_m_s) = resonance_largest_phase(&oscillator);
    let floor = oscillator.floor_dt_s().expect("specimen floor");
    finish(
        "signal",
        "resonance",
        "Company specimen: a 1 kg mass on a 10 Hz mount, damping ratio 0.05 and stroke 15.625 mm, under an imposed acceleration, up to the energy of a free ring of twice the stroke.",
        vec![
            scalar(
                "displacement_m",
                Role::Phase,
                "m",
                Some(-largest_displacement_m),
                false,
                Some(largest_displacement_m),
                false,
                format!(
                    "from -{0} to {0} m, both inclusive, with the velocity at rest. The displacement and the velocity together must keep the mechanical energy at or below that of a free ring of twice the stroke, so a displacement inside this range is refused when the velocity is large enough",
                    token(largest_displacement_m)
                ),
            ),
            scalar(
                "velocity_m_s",
                Role::Phase,
                "m/s",
                Some(-largest_velocity_m_s),
                false,
                Some(largest_velocity_m_s),
                false,
                format!(
                    "from -{0} to {0} m/s, both inclusive, with the displacement at zero. The displacement and the velocity together must keep the mechanical energy at or below that of a free ring of twice the stroke, so a velocity inside this range is refused when the displacement is large enough",
                    token(largest_velocity_m_s)
                ),
            ),
            scalar(
                "t_s",
                Role::Phase,
                "s",
                Some(0.0),
                false,
                Some(resonance::START_TIME_MAX_S),
                false,
                format!(
                    "inclusive [0, {}] s, the time of the start since the run began. A new start is 0. A landed state handed back as a new start carries its own t_s, so the clock runs on across continued requests. A run adds at most 318.3 s, so its clock stays under 2048 s, where a floor step still moves it. Outside the span it is refused as StartTimeOutsideCard",
                    token(resonance::START_TIME_MAX_S)
                ),
            ),
            scalar(
                "energy_anchor_j",
                Role::Phase,
                "J",
                Some(0.0),
                false,
                None,
                false,
                format!(
                    "from 0 J, inclusive, to {} J, inclusive, four stroke energies: the mechanical energy the ledger of the run is anchored at. A new start states the mechanical energy of its own displacement and velocity, 0.5 m v^2 + 0.5 k x^2 with m 1 kg and k = m (20 pi)^2 N/m. A landed state handed back as a new start carries its own energy_anchor_j, so the anchor of the run stays where it was. A start whose own mechanical energy is above four stroke energies is refused as EnergyAboveCard; otherwise an anchor below 0 or above four stroke energies is refused as LedgerNotOfThisState",
                    token(oscillator.energy_ceiling_j())
                ),
            ),
            scalar(
                "boundary_work_j",
                Role::Phase,
                "J",
                None,
                false,
                None,
                false,
                "the work booked since the anchor: the work of the held acceleration less the heat taken by the damper. A new start states 0, and t_s 0 with any other value is refused as LedgerNotOfThisState. A landed state handed back as a new start carries its own boundary_work_j. No numeric bound: the defect E - energy_anchor_j - boundary_work_j, with E the mechanical energy of the stated displacement and velocity, must be within the conservation limit, 1e-6 of the stroke energy, or the start is refused as LedgerNotOfThisState. With these three fields handed back, a run cut into requests lands the same steps, with the same time, ledger and residual bit for bit, as the run sent whole, and is refused on the same step. The engine keeps nothing between requests: it holds a request to the ledger the request states, and a request at t_s 0 with no booked work is a new start with a full limit".to_string(),
            ),
            scalar(
                "accel_m_s2",
                Role::Load,
                "m/s^2",
                None,
                false,
                None,
                false,
                impulse_rule("accel_m_s2", "omega * STROKE_M", oscillator.accel_impulse_limit_m_s()),
            ),
        ],
        floor,
        "the ceiling divided by 2^20: a step that changes the state by two parts in a hundred million. Under it a step is mostly rounding",
        gate,
        "0.02 rad on its natural frequency. The reach of a driven run at this step: the step method's own error is summed in the ledger from the anchor of the run, and a mount driven at resonance spends the conservation limit after its damper has taken 342 stroke energies of heat: 447,824 steps under a square wave of +/-3 m/s^2, which rings the mount at 0.62 of the stroke, and 175,815 under one of +/-4.8 m/s^2, which rings it at 0.99 of the stroke. The reach counts across continued requests that hand back t_s, energy_anchor_j and boundary_work_j. The step after that is refused as SpontaneousGeneration or FirstLawBreach with nothing wrong in the mount. Halving the step cuts that error about fifteen-fold",
        nyquist,
        impulse_rule("accel_m_s2", "omega * STROKE_M", oscillator.accel_impulse_limit_m_s()),
        &[
            "NonFiniteInput",
            "NonFiniteState",
            "NonPositiveDt",
            "InvalidOscillator",
            "ForceAboveNyquist",
            "EnergyAboveCard",
            "StartTimeOutsideCard",
            "LedgerNotOfThisState",
            "SpontaneousGeneration",
            "FirstLawBreach",
        ],
        vec![Advisory {
            name: "FLAG_STROKE_LIMIT",
            mask: resonance::FLAG_STROKE_LIMIT,
            seals: true,
        }],
        slots(&[
            ("t", "Time since the run began, carried across continued requests, in seconds."),
            ("pos[0]", "displacement_m"),
            ("pos[1]", "velocity_m_s"),
            ("pos[2]", "accel_m_s2"),
            ("vel[0]", "kinetic_j"),
            ("vel[1]", "potential_j"),
            ("vel[2]", "mechanical_energy_j"),
            ("force_torque", "Stroke margin, in metres."),
            ("residual", "Conservation residual."),
            ("flags", "FLAG_STROKE_LIMIT. It seals."),
            ("proof", "Not written by the step."),
        ]),
    )
}

fn tesseract_drive() -> Card {
    let resonator = tesseract::Tesseract::drive_mode();
    let (gate, nyquist) = clock(
        resonator.omega_rad_s,
        resonator.gate_dt_s(),
        resonator.nyquist_dt_s(),
    );
    let floor = resonator.floor_dt_s().expect("specimen floor");
    let reach_m = resonator.amplitude_limit_m;
    let speed_m_s = resonator.speed_limit_m_s();
    let defect_j = resonator.defect_limit_j();
    finish(
        "signal",
        "tesseract",
        "Company specimen: an 80 Hz drive-mode Duffing resonator, damping ratio 1/80, cubic stiffness 6e8 per square metre per square second and proof mass 1e-7 kg, under acceleration, an external rate and sensed velocity, up to an amplitude of 7.8125 mm.",
        vec![
            within(
                "displacement_m",
                Role::Phase,
                "m",
                reach_m,
                format!(
                    "from -{0} to {0} m, both inclusive: the card's amplitude limit, 1/128 m. It is a limit of the card and not the amplitude where the driven response folds: under one sinusoidal drive this resonator has two coexisting branches from an amplitude of 4.026 mm, the cusp of its averaged frequency response, sqrt(16 zeta omega^2 / (3 sqrt(3) alpha)). The limit holds at every step: a step that would land a displacement outside it is refused as AmplitudeOutsideCard and that state is not sealed",
                    token(reach_m)
                ),
            ),
            within(
                "velocity_m_s",
                Role::Phase,
                "m/s",
                speed_m_s,
                format!(
                    "from -{0} to {0} m/s, both inclusive: the speed at zero displacement whose kinetic energy is the energy of the mode at the amplitude limit. The limit holds at every step: a step that would land a velocity outside it is refused as AmplitudeOutsideCard and that state is not sealed. A displacement and a velocity each inside its range are a state of the card together, whatever their envelope",
                    token(speed_m_s)
                ),
            ),
            within(
                "energy_defect_j",
                Role::Phase,
                "J",
                defect_j,
                format!(
                    "from -{0} to {0} J, both inclusive: the signed ledger defect, mechanical energy less the anchor less the booked work, that the run reaching this state carried, and 0 for a new run. The limit is 1e-6 of the energy of the mode at the amplitude limit, the same limit every step is held to. A state handed back continues its run with its own energy_defect_j and not otherwise; a run restarted with 0 is a new run, and its ledger begins again",
                    token(defect_j)
                ),
            ),
            scalar(
                "accel_m_s2",
                Role::Load,
                "m/s^2",
                None,
                false,
                None,
                false,
                impulse_rule("accel_m_s2", "omega * AMPLITUDE_LIMIT_M", resonator.accel_impulse_limit_m_s()),
            ),
            scalar(
                "omega_ext_rad_s",
                Role::Load,
                "rad/s",
                None,
                false,
                None,
                false,
                TESSERACT_SENSE_REACH.to_string(),
            ),
            scalar(
                "sense_velocity_m_s",
                Role::Load,
                "m/s",
                None,
                false,
                None,
                false,
                TESSERACT_SENSE_REACH.to_string(),
            ),
        ],
        floor,
        "the ceiling divided by 2^20: a step that changes the state by two parts in a hundred million. Under it a step is mostly rounding",
        gate,
        "0.02 rad on its natural frequency. The step method's own error is summed in the ledger over a run, and a run driven long enough at this step is refused as SpontaneousGeneration or FirstLawBreach with nothing wrong in the resonator. Measured at this step inside the amplitude and speed limits: a drive of 46 m/s^2 peak at 83.3 Hz, a new value each step, from -6.6 mm at rest, rides the upper branch near 7 mm and 3.6 m/s and completes a million steps with the residual at 0.52 of its limit; that run continued from its last state with its energy_defect_j is refused as SpontaneousGeneration on step 914,931 of the second run, where the same run carried straight through is refused. A drive of 45 m/s^2 peak at 83.04 Hz completes a million steps from -6.5 mm with the residual at 0.50 of its limit. A held 1,000 m/s^2 from rest completes a million steps at 4.3e-4 of the limit; a held 1,500 m/s^2 is refused as AmplitudeOutsideCard on step 97. The largest acceleration the load rule allows at this step, held from rest, lands one step and is refused as AmplitudeOutsideCard on the second",
        nyquist,
        format!(
            "{}. A run starts at time zero with its ledger anchored so that the defect is the energy_defect_j given: the anchor is the starting mechanical energy less that defect and the booked work is zero. A run or a single step whose final state is handed back as a new start continues the same motion, and continues the same ledger when the request carries that state's energy_defect_j, and not otherwise; t_s begins again at zero. A start without energy_defect_j is refused as a missing field. A step that would land the displacement or the velocity outside its range is refused as AmplitudeOutsideCard, and a run that reaches one is sealed up to its last landed step",
            impulse_rule("accel_m_s2", "omega * AMPLITUDE_LIMIT_M", resonator.accel_impulse_limit_m_s())
        ),
        &[
            "NonFiniteInput",
            "NonFiniteState",
            "NonPositiveDt",
            "InvalidResonator",
            "ForceAboveNyquist",
            "AmplitudeOutsideCard",
            "SpontaneousGeneration",
            "FirstLawBreach",
        ],
        vec![],
        slots(&[
            ("t", "Time since the start of the run, in seconds."),
            ("pos[0]", "displacement_m"),
            ("pos[1]", "velocity_m_s"),
            ("pos[2]", "envelope_m"),
            ("vel[0]", "kinetic_j"),
            ("vel[1]", "potential_j"),
            ("vel[2]", "accel_m_s2"),
            ("force_torque", "Scale factor, in newtons."),
            ("residual", "Conservation residual."),
            ("flags", "0"),
            ("proof", "Not written by the step."),
        ]),
    )
}

fn hydraulic() -> Card {
    let column = hydraulic_fluid::FluidColumn::mineral_oil();
    let (gate, nyquist) = clock(column.acoustic_omega_rad_s(), column.gate_dt_s(), None);
    let vapor = hydraulic_fluid::VAPOR_PRESSURE_PA;
    let ceiling = column.pressure_ceiling_pa;
    let floor = column.floor_dt_s().expect("specimen floor");
    let flow_max = column.flow_acoustic_m3_s();
    finish(
        "materials",
        "hydraulic_fluid",
        "Company specimen: a 0.50 m mineral-oil column in a 50 mm bore, bulk modulus 1.72 GPa and density 870 kg/m^3, charged at one atmosphere, under a commanded flow, from its vapour pressure of 10 Pa to 20.74 MPa absolute.",
        vec![
            scalar(
                "pressure_pa",
                Role::Phase,
                "Pa",
                Some(vapor),
                false,
                Some(ceiling),
                false,
                format!(
                    "from vapor pressure {} to the pressure ceiling {}, both inclusive: the card ends at 6.0 mm of compression, and the landing is held to the same ceiling. FLAG_COLUMN_SOFT is set at or above 1.8 mm of compression and FLAG_COLUMN_LIMIT at or above 3.0 mm. Neither flag is a limit: both seal, the card stands to 6.0 mm, and a pressure above the ceiling is refused as PressureAboveCard",
                    token(vapor),
                    token(ceiling)
                ),
            ),
            scalar(
                "flow_m3_s",
                Role::Load,
                "m^3/s",
                Some(-flow_max),
                false,
                Some(flow_max),
                false,
                format!(
                    "from {} to {}, both inclusive, at every step; outside it the step is refused as FlowAboveAcoustic. The bound is the bore area times the bore speed (pressure ceiling - charge) / (density * wave speed), 0.012 of the oil's wave speed of about 1,406 m/s: a flow that starts at the end face sends a pressure front of density * wave speed * bore speed down the column, and at this flow that front equals the card's whole span from the charge to the ceiling. Inside the step window this bound is reached before the stroke rule on dt_s, which allows {} m^3 of oil per step",
                    token(-flow_max),
                    token(flow_max),
                    token(column.stroke_volume_m3())
                ),
            ),
        ],
        floor,
        "the ceiling divided by 2^20: a step that changes the state by two parts in a hundred million. Under it a step is mostly rounding",
        gate,
        "0.02 rad on its half-wave acoustic frequency",
        nyquist,
        impulse_rule("flow_m3_s", "area * COMPRESSION_LIMIT_M", column.stroke_volume_m3()),
        &[
            "NonFiniteInput",
            "NonFiniteState",
            "NonPositiveDt",
            "InvalidChamber",
            "FlowAboveNyquist",
            "FlowAboveAcoustic",
            "Cavitation",
            "PressureAboveCard",
            "SpontaneousGeneration",
            "FirstLawBreach",
        ],
        vec![
            Advisory {
                name: "FLAG_COLUMN_SOFT",
                mask: hydraulic_fluid::FLAG_COLUMN_SOFT,
                seals: true,
            },
            Advisory {
                name: "FLAG_COLUMN_LIMIT",
                mask: hydraulic_fluid::FLAG_COLUMN_LIMIT,
                seals: true,
            },
        ],
        slots(&[
            ("t", "Time since the anchor, in seconds."),
            ("pos[0]", "pressure_pa"),
            ("pos[1]", "fluid_volume_m3"),
            ("pos[2]", "compression_m"),
            ("vel[0]", "mass_kg"),
            ("vel[1]", "boundary_mass_kg"),
            ("vel[2]", "elastic_energy_j"),
            ("force_torque", "Vapor margin, in pascals."),
            ("residual", "Conservation residual."),
            (
                "flags",
                "FLAG_COLUMN_SOFT is set at or above 1.8 mm of compression and FLAG_COLUMN_LIMIT at or above 3.0 mm. Neither is a limit: both seal, the card stands to 6.0 mm of compression, and it refuses a pressure above that as PressureAboveCard.",
            ),
            ("proof", "Not written by the step."),
        ]),
    )
}

fn cable() -> Card {
    let tendon = cable_stretch::Tendon::dyneema();
    let (gate, nyquist) = clock(tendon.omega_rad_s, tendon.gate_dt_s(), None);
    let ultimate = tendon.ultimate_tension_n;
    let ultimate_elongation = tendon.ultimate_elongation_m();
    let stiffness = tendon.stiffness_n_per_m();
    let damping = tendon.damping_n_s_per_m();
    let floor = tendon.floor_dt_s().expect("specimen floor");
    finish(
        "materials",
        "cable_stretch",
        "Company specimen: a 1.20 m Dyneema tendon 1.50 mm in diameter, modulus 120 GPa, damping ratio 0.01 and ultimate stress 2800 MPa, 4948 N at 28 mm of elongation, under a commanded force, in tension up to the tension at which it snaps.",
        vec![
            scalar(
                "elongation_m",
                Role::Phase,
                "m",
                Some(0.0),
                false,
                Some(ultimate_elongation),
                true,
                format!(
                    "[0, {}) m, the ultimate elongation, where the elastic stress reaches the ultimate 2800 MPa and k x reaches the ultimate tension of {} N. A rope has no compressive branch: an elongation below 0 is a rope shorter than its free length and is refused as Slack. An elongation at or above the ultimate elongation is a broken braid and is refused as TensileFailure. The damper term does not widen this range. The initial tension k x + c v must also be in [0, {}) N",
                    token(ultimate_elongation),
                    token(ultimate),
                    token(ultimate)
                ),
            ),
            scalar(
                "velocity_m_s",
                Role::Phase,
                "m/s",
                None,
                false,
                None,
                false,
                format!(
                    "no fixed range. The velocity enters the tension with the elongation x, through the braid's damping, and the initial tension k x + c v must be in [0, {}) N, with k = {} N/m and c = {} N s/m: at each elongation the velocity is held to [-k x / c, ({} - k x) / c) m/s. A start outside it is refused as Slack or TensileFailure",
                    token(ultimate),
                    token(stiffness),
                    token(damping),
                    token(ultimate)
                ),
            ),
            scalar(
                "force_n",
                Role::Load,
                "N",
                None,
                false,
                None,
                false,
                impulse_rule("force_n", "mass * omega * ultimate elongation", tendon.force_impulse_limit_n_s()),
            ),
        ],
        floor,
        "the ceiling divided by 2^20: a step that changes the state by two parts in a hundred million. Under it a step is mostly rounding",
        gate,
        "0.02 rad on its natural frequency",
        nyquist,
        impulse_rule("force_n", "mass * omega * ultimate elongation", tendon.force_impulse_limit_n_s()),
        &[
            "NonFiniteInput",
            "NonFiniteState",
            "NonPositiveDt",
            "InvalidCable",
            "ForceAboveNyquist",
            "TensileFailure",
            "Slack",
            "SubnormalState",
            "SpontaneousGeneration",
            "FirstLawBreach",
        ],
        vec![
            Advisory {
                name: "FLAG_TRACKING_DEGRADE",
                mask: cable_stretch::FLAG_TRACKING_DEGRADE,
                seals: true,
            },
            Advisory {
                name: "FLAG_CONTROL_LIMIT",
                mask: cable_stretch::FLAG_CONTROL_LIMIT,
                seals: true,
            },
        ],
        slots(&[
            ("t", "Time since the anchor, in seconds."),
            ("pos[0]", "elongation_m"),
            ("pos[1]", "velocity_m_s"),
            ("pos[2]", "tension_n"),
            ("vel[0]", "kinetic_j"),
            ("vel[1]", "potential_j"),
            ("vel[2]", "mechanical_energy_j"),
            ("force_torque", "Ultimate margin, in newtons."),
            ("residual", "Conservation residual."),
            (
                "flags",
                "FLAG_TRACKING_DEGRADE and FLAG_CONTROL_LIMIT. Both seal.",
            ),
            ("proof", "Not written by the step."),
        ]),
    )
}

fn generator() -> Card {
    let unit = generator_mmrtg_f1::GeneratorMmrtgF1::curiosity();
    let (gate, nyquist) = clock(unit.combined_rate_per_s, unit.gate_dt_s(), None);
    let last_s = generator_mmrtg_f1::MISSION_TIME_MAX_S;
    let ledger_end_s = unit.ledger_end_mission_time_s().expect("ledger end");
    let floor = unit.floor_dt_s().expect("specimen floor");
    finish(
        "energy",
        "generator_mmrtg_f1",
        "One radioisotope generator, flight unit F1, on the Curiosity rover: its daily-average electrical output from landing on, with no load. Compared on the 1,279 held-back days 2,809 to 4,087, the end of the published record: inside its line of 0.010 of 115.4 W by root mean square (0.008129, about 0.94 W) and outside it in its mean over the final 365 days (+0.010094, about 1.16 W high), so it failed by the rule fixed beforehand. It reads high and the gap grew each year; past day 4,087 nothing was compared and the frame sets FLAG_PAST_RECORD.",
        vec![scalar(
            "mission_time_s",
            Role::Phase,
            "s",
            Some(0.0),
            false,
            Some(ledger_end_s),
            false,
            format!(
                "inclusive [0, {}] s. The maximum is where the decay heat the fuel still holds falls to the ledger's resolution, 1e-6 of the heat released from landing to mission day 4,087, about 2,060 years after landing: past it a breach of the ledger the size of its limit could not be seen (DecayHeatBelowLedgerResolution). The published record ends on mission day 4,087 ({} s); a later start is answered with FLAG_PAST_RECORD. The returned state gives the mission time as t_s",
                token(ledger_end_s),
                token(last_s)
            ),
        )],
        floor,
        "the ceiling divided by 2^20: one part in a thousand million of thermal power. Under it a step is mostly rounding",
        gate,
        "0.02 rad on its fastest rate, fuel decay plus converter fading",
        nyquist,
        "No load. A run segment carries an empty load object. Mission days 2,809 to 4,087 are the days the card is compared with, outside its fit: RMS 0.008129 of 115.4 W (target 0.010), final-year mean +0.010094 (target 0.010 in size); +0.0031, +0.0045, +0.0096 over successive years and +0.0107 over the last 184 days; it answers them with every constant as compared. Mission day 4,087 is the last day of the published record and not an end of the generator: a step that lands past it by more than the clock's tolerance lands with FLAG_PAST_RECORD set, the same exact map carried forward, and since the card read high and the gap grew each year, later days are expected to read higher still. The card stops where the decay heat still held falls under the ledger's resolution, about 2,060 years after landing, as DecayHeatBelowLedgerResolution, and before landing as MissionTimeBeforeLanding. The mission clock is a compensated sum: each step adds dt_s to t_s in 64-bit arithmetic, keeps what the addition rounded off and folds it back in, so t_s stays within half a spacing of 64-bit numbers of the exact sum of the start and the steps at any length of run. Every step, the first included, compares its landing with mission day 4,087 plus one tolerance, MISSION_END_TOLERANCE_S, 2^-22 s or about 2.38e-7 s: four spacings of 64-bit numbers between 2^28 s and 2^29 s, where the last day lies, enough for a start and a step read from decimals and for the clock's own rounding. A run or a single step whose end in decimal is exactly mission day 4,087 lands its last step without FLAG_PAST_RECORD, and range_margin_s, the time left to the last day of the record, is zero on and past it. A restart from a returned t_s begins with no carry. electrical_energy_j, heat_rejected_j and fading_electrical_energy_j count from the start of this run, the anchor of the ledger, and not from landing: energy_anchor_j is the decay heat held at that start. A restart from a returned state starts each again at zero. No refusal depends on these totals: every limit of the card is on the mission time, the heat still held or on a single step's ledgers".into(),
        &[
            "NonFiniteInput",
            "NonFiniteState",
            "NonPositiveDt",
            "InvalidGenerator",
            "MissionTimeBeforeLanding",
            "DecayHeatBelowLedgerResolution",
            "SpontaneousGeneration",
            "FirstLawBreach",
            "ConverterAccountBreach",
        ],
        vec![
            Advisory {
                name: "FLAG_PAST_FITTING_SPAN",
                mask: generator_mmrtg_f1::FLAG_PAST_FITTING_SPAN,
                seals: true,
            },
            Advisory {
                name: "FLAG_PAST_RECORD",
                mask: generator_mmrtg_f1::FLAG_PAST_RECORD,
                seals: true,
            },
        ],
        slots(&[
            ("t", "Mission time since landing, in seconds."),
            ("pos[0]", "electrical_power_w"),
            ("pos[1]", "thermal_power_w"),
            ("pos[2]", "converter_fading_fraction"),
            ("vel[0]", "electrical_energy_j"),
            ("vel[1]", "heat_rejected_j"),
            ("vel[2]", "fuel_heat_j"),
            ("force_torque", "Range margin, in seconds: the time left to mission day 4,087, the last day of the record; zero on and past it."),
            ("residual", "Conservation residual."),
            (
                "flags",
                "FLAG_PAST_FITTING_SPAN after mission day 2,060, FLAG_PAST_RECORD after mission day 4,087, the last day of the published record. Both seal.",
            ),
            ("proof", "Not written by the step."),
        ]),
    )
}

fn fingerpad_p02_glass_card() -> Card {
    let pad = fingerpad_p02_glass::FingerPadP02Glass::on_glass();
    // The model has no pole, so no `π/ω` is published.
    let gate = pad.gate_dt_s().expect("specimen gate");
    let floor = pad.floor_dt_s().expect("specimen floor");
    finish(
        "materials",
        "fingerpad_p02_glass",
        FINGERPAD_P02_GLASS_SUMMARY,
        vec![
            scalar(
                "twist_deg",
                Role::Phase,
                "deg",
                Some(0.0),
                false,
                Some(0.0),
                false,
                "inclusive [0, 0] degrees: the twist of the plate against the finger at the start, which is zero, and every run and every single step starts there: the card's closed forms are of a pad twisted from zero under the load it holds, and a pad already twisted holds springs set by the load and the path it was brought there under, which a start does not carry, so any other start is refused as TwistNotStartedFromZero, a twist such as 1e-46 that would read as zero in a 32-bit frame slot included. The plate turned 80 degrees in every trial and slowed to a stop between 77 and 80 degrees; the torque and the stuck fraction were compared with measurement from 1 to 75 degrees, and past 75 degrees a run goes on, each frame carrying FLAG_BEYOND_COMPARED".into(),
            ),
            scalar(
                "normal_force_n",
                Role::Load,
                "N",
                Some(0.0),
                true,
                None,
                false,
                "above 0 N: the force of the plate on the pad. A force of zero or less is no contact and is refused as NoContact. Measured on this finger at 0.5, 1, 2, 5 and 10 N; the card's slip torque was off by +13.6 percent at 0.5 N and +21.7 percent at 5 N on trials it was fitted to, by +9.4 percent at 1 N and +29.3 percent at 10 N on trials outside the fit, against a target of 5.62 percent, and within 0.7 to 7.4 percent at 2 N. Below 0.5 N or above 10 N, where nothing was measured, a frame carries FLAG_BEYOND_COMPARED".into(),
            ),
            scalar(
                "twist_rate_deg_s",
                Role::Load,
                "deg/s",
                Some(0.0),
                true,
                None,
                false,
                "above 0 degrees a second: the rate at which the plate turns about the normal. A rate of zero or less is refused as TwistRateNotPositive. The constants were fitted at 5, 10, 20 and 50 degrees a second at 2 N, where the slip torque is off by +0.7, -6.8, -5.0 and -4.2 percent, and compared at 100, outside the fit, where it is off by -13.0 percent. Below 5 or above 100 degrees a second, where nothing was measured, a frame carries FLAG_BEYOND_COMPARED; the characteristic twists go as the rate to the powers -0.341 and -0.015 and grow without limit as the rate falls. The direction of the turn is not an input: clockwise and counter-clockwise trials were pooled".into(),
            ),
        ],
        floor,
        "the ceiling divided by 2^8, a twenty-fifth of the sampling interval of the force sensors",
        gate,
        "0.01 s, one camera frame at the fastest twist measured, 100 degrees a second. The card holds the four-point rule that sums the work and the heat to its own step: a step that turns the plate by more than 0.05 of the characteristic twist of the torque is refused as TwistStepAboveQuadrature. At 2 N that is 0.73 degrees at 20 degrees a second and 0.42 at 100, so a step of 0.01 s holds to 52.6 degrees a second and a faster twist takes a shorter step",
        None,
        format!(
            "Over a step the normal force and the twist rate are held, and the twist grows by rate x dt_s. A bed of springs under a parabolic friction limit, twisted about its centre: with c = 2 / (u + sqrt(u^2 + 4)), the torque is T_limit x (1 - (5c^3 + 3c^5)/8) at u = twist / twist_T, and the stuck fraction is c^2 at u = twist / twist_S. T_limit = {} N m x (F / 2 N)^{} x (rate / 20)^{}; twist_T = {} deg x (F / 2 N)^{} x (rate / 20)^{}; twist_S = {} deg x (rate / 20)^{}. All eight numbers are fitted to this finger's own trials; the level, {} N m, is that of the block of trials at 2 N. The card answers at every force and every rate above zero, with the gaps at the other forces and at 100 degrees a second stated in its validation record, and a frame beyond 0.5 to 10 N, 5 to 100 degrees a second or 75 degrees of twist carries FLAG_BEYOND_COMPARED. A run is one twist from zero under one load: it starts at time zero with the pad untwisted and the work and the heat at zero, and a start at any other twist is refused as TwistNotStartedFromZero. A single step is the first step of such a twist, from zero, at each call: single steps cannot be chained, and a twist is followed with run_trajectory. The force and the rate cannot change during a twist: under a changed load the bed's springs hold a state set by the path, which the card's state does not carry, and a later segment of a run whose load differs from the first is refused as LoadChangedDuringTwist. Every measured twist began untwisted and held its load. Because no run and no step can start part way, no request can take up a twist under another load. The card's own step refuses the same change for as long as the state lives: the state carries the force and the rate it was twisted under, held_normal_force_n and held_twist_rate_deg_s, which are answers and not inputs. A value outside its range in any segment of a run refuses the whole run before its first step. The frame flag FLAG_FULL_SLIP is an event, the one the card exists to find, and not a warning. A run goes on past 75 degrees, the end of the twist compared with measurement, with the torque rising toward its slip limit and the heat growing with the twist; a frame past 75 degrees by more than {} degrees, the most the sum of the steps can round by, carries FLAG_BEYOND_COMPARED. A step that turns the plate by more than 0.05 of the characteristic twist of the torque is refused as TwistStepAboveQuadrature. The ledger is held to 1e-9 of the larger of 0.0508 J and the plate work of the run.",
            fingerpad_p02_glass::SLIP_TORQUE_REFERENCE_N_M,
            fingerpad_p02_glass::LOAD_EXPONENT,
            fingerpad_p02_glass::RATE_EXPONENT,
            fingerpad_p02_glass::TORQUE_TWIST_REFERENCE_DEG,
            fingerpad_p02_glass::TORQUE_TWIST_LOAD_EXPONENT,
            fingerpad_p02_glass::TORQUE_TWIST_RATE_EXPONENT,
            fingerpad_p02_glass::STUCK_TWIST_REFERENCE_DEG,
            fingerpad_p02_glass::STUCK_TWIST_RATE_EXPONENT,
            fingerpad_p02_glass::SLIP_TORQUE_REFERENCE_N_M,
            fingerpad_p02_glass::TWIST_END_ALLOWANCE_DEG,
        ),
        &[
            "NonFiniteInput",
            "NonFiniteState",
            "NonPositiveDt",
            "InvalidFingerPad",
            "NoContact",
            "TwistRateNotPositive",
            "TwistStepAboveQuadrature",
            "LoadChangedDuringTwist",
            "TwistNotStartedFromZero",
            "TwistBelowZero",
            "SpontaneousGeneration",
            "FirstLawBreach",
        ],
        vec![
            Advisory {
                name: "FLAG_FULL_SLIP",
                mask: fingerpad_p02_glass::FLAG_FULL_SLIP,
                seals: true,
            },
            Advisory {
                name: "FLAG_BEYOND_COMPARED",
                mask: fingerpad_p02_glass::FLAG_BEYOND_COMPARED,
                seals: true,
            },
        ],
        slots(&[
            ("t", "Time since the start of the run, in seconds."),
            ("pos[0]", "twist_deg"),
            ("pos[1]", "stuck_fraction"),
            ("pos[2]", "slip_torque_limit_n_m"),
            ("vel[0]", "plate_work_j"),
            ("vel[1]", "stored_energy_j"),
            ("vel[2]", "friction_heat_j"),
            (
                "force_torque",
                "Torque of the pad on the plate, in newton metres.",
            ),
            ("residual", "Conservation residual."),
            (
                "flags",
                "FLAG_FULL_SLIP, an event and not a warning: the model's stuck fraction is under 0.03, the threshold of the measurement, so the whole contact slides. It is the event the card exists to find. The model's torque is then 1.1 to 6.9 percent under its limit across the card's range, 2.4 percent at 2 N and 20 degrees a second, and goes on rising toward it. The card puts the event at 39 to 40 degrees of twist; on this finger at 2 N it was measured at 44 to 49 degrees. It seals. FLAG_BEYOND_COMPARED: the frame is beyond what was compared with this finger, a normal force below 0.5 N or above 10 N, a twist rate below 5 or above 100 degrees a second, or a twist past 75 degrees. It seals.",
            ),
            ("proof", "Not written by the step."),
        ]),
    )
}

/// The data of the JSI TRIGA reactor's validation record.
pub(crate) const REACTOR_TRIGA_JSI_DATA: &str = "The pulse database of the Jozef Stefan Institute, trigapulse.ijs.si: a table of 757 pulses from 1991 to 2023 and, for each pulse, a record of power and two fuel thermocouples at 20,000 samples a second. The constants taken from publications are from the institute's evaluation of its pulse experiments and, for the hot-rod factor, a University of Ljubljana master's thesis; cite gives both. None of the site's measured values is stored in the card. 43 recorded pulses of cores 215, 217 and 231 were read. Fitted on 36 of them: 1.4 to 3.0 printed dollars, a peak at least ten times the noise, a scram found, a printed value that follows the rod table. Compared with 20 table rows of the same cores not in the fit, on the table's peak power, energy within 50 ms of the peak and thermocouple maximum: 12 rows of core 231 from 22 and 28 March 2018, at 1.4 to 2.6 printed dollars, and 8 repeats of core 217 at 2.5 dollars from 24 March 2015. The start temperature and the scram instant of each row are set by rule.";

/// The pass line of the JSI TRIGA reactor's validation record.
pub(crate) const REACTOR_TRIGA_JSI_PASS_LINE: &str = "The uncertainty of a comparison: the repeat scatter of one pulse within a day (peak power 1.3 percent, width 0.5 percent, energy within 50 ms of the peak 0.7 percent, energy to the scram 0.8 percent, thermocouple maximum 0.9 K) and the change for 0.03 printed dollars, the shift of the day's reference; for a row without a record also the start temperature and the scram instant, which are set by rule. Errors are in units of that uncertainty. On the fitted pulses, in four bins of reactivity: RMS at most 1.5 for peak power, for width and for energy to the scram; the card's fitted range is the unbroken run of bins within that. On the 20 rows not in the fit, for each group and each of peak power, energy within 50 ms of the peak and thermocouple maximum: RMS at most 1.5, worst at most 3, mean at most 2 over the square root of the number of days (1.41 for the 2018 rows, on two days, and 2 for the 2015 repeats, on one). Computed beside the card on the same rows: a straight line in printed reactivity, the nearest recorded pulse, the Fuchs-Nordheim relations with the institute's constants, the model with its constants unfitted, and the card's earlier form. Record SHA-256 9a0720f1822161c3b53cbc351d862ec4c077da51521ee1a96d7ceb85d8275ddc.";

/// The one-line summary of the JSI TRIGA reactor, as the list shows it.
pub(crate) const REACTOR_TRIGA_JSI_SUMMARY: &str = "The TRIGA Mark II reactor of the Jozef Stefan Institute, Ljubljana, with its 56-element core of standard fuel, in pulse mode on the rod-worth tables of cores 215, 217 and 231: power, energy released and core-average fuel temperature from the fire signal through the pulse, the scram and after it. Ten constants are fitted to 36 pulses of this reactor, in a form added after the twelve fixed beforehand had failed; by the rule fixed beforehand the comparison with the 20 rows held back failed, four of six lines missed, with peak power 23 and 44 percent low on a day the card was not fitted to; the fuel temperature was compared only through one thermocouple of one element. It was compared at 2.0 to 3.0 printed dollars; below 2.0 dollars it read 29, 21 and 9 percent low in peak power at 1.4, 1.6 and 1.8 dollars. Outside that it computes the same equations with nothing compared: from any start in liquid water, at any reactivity above zero up to the rod's whole worth of 4.16 printed dollars, with the scram at any time or none.";

/// The result of the JSI TRIGA reactor's validation record.
pub(crate) const REACTOR_TRIGA_JSI_RESULT: &str = "On the 36 fitted pulses, 1.4 to 3.0 dollars, in all four bins: peak power 4.1 percent RMS (0.49; target 1.5), width 2.6 percent (0.80), energy to the scram 2.3 percent (0.96), each after the pulse channel's factor for that core's sessions. Thermocouple maximum, through the hot-rod factor 1.387 and a delay fitted to each recording chain: 4.9 K RMS on 29 pulses, 1.8 percent of the rise, worst 8.6 K low. From 2.0 dollars up, on 27 fitted pulses: 3.3 percent RMS in peak power, 1.6 percent in energy to the scram and 4.7 K. On the 20 rows not in the fit. Core 231 on 22 and 28 March 2018, twelve rows on a fitted table, on days not fitted. Peak power: 10.9 percent low on average, 18.0 percent RMS, worst 44 percent low; RMS 1.21 (target 1.5), worst 3.02 (target 3), mean -0.74 (target 1.41). Energy within 50 ms of the peak: 10.2 percent low, 15.7 percent RMS; RMS 1.47 (1.5), worst 3.75 (3), mean -1.11 (1.41). Thermocouple maximum, eleven rows: 2.6 K low on average, 8.0 K RMS, worst 20.3 K low; RMS 1.29 (1.5), worst 3.10 (3), mean -0.39 (1.41). The two rows of 22 March read 44 and 23 percent low in peak power: that day's printed 1.5 and 2.0 dollars stand at rod positions the fitted day prints as 1.75 and 2.25, and the measured pulses lie between the two readings. On 28 March the six rows at 2.0 dollars and above are within 3.7 percent in peak power, 2.7 percent in energy and 5.8 K; the four below 2.0 dollars read 29, 21 and 9 percent low in peak power at 1.4, 1.6 and 1.8 dollars and level on the repeat at 1.8. From 2.0 dollars up, on the seven rows of 2018: the thermocouple maximum within its targets; peak power and energy outside them on the row of 22 March at 2.0 dollars, 23 percent low. Core 217 on 24 March 2015, eight repeats at 2.5 dollars between two fitted pulses. Peak power: 5.6 percent high; RMS 1.09 (1.5), worst 1.33 (3), mean +1.07 (2). Thermocouple maximum: 2.1 K low; RMS 0.42 (1.5). Energy within 50 ms of the peak: 5.4 percent high; RMS 2.01 (1.5). Beside the card on the twelve rows of 2018: its earlier form, RMS 0.97 and worst 1.59 on the thermocouple maximum, 2.54 RMS on peak power and 4.18 on energy; a straight line in printed reactivity, RMS 1.45 on energy and 1.33 on the thermocouple maximum, each with its worst row above 3; the nearest recorded pulse, the Fuchs-Nordheim relations and the model with its constants unfitted, outside the targets on every measure. On the 2015 repeats the nearest recorded pulse, taken unchanged, is within the targets on peak power and thermocouple maximum as well. One row at 1.2 dollars, below the range of the comparison, is not in these numbers.";

/// The limits of the JSI TRIGA reactor's validation record.
pub(crate) const REACTOR_TRIGA_JSI_LIMITS: &str = "Ten constants are fitted in the card: for each of the three rod-worth tables a scale and an offset on the printed dollars (scale 0.6921, 0.6677 and 0.7191, offset -0.1204, -0.1366 and -0.04949 dollars, for cores 215, 217 and 231); the feedback coefficient, 6.241e-5 per K at 294 K, and its rise, 8.718e-8 per K2; the generation time, 29.59 microseconds, where the published value is 40 with 8 percent; and the heat flow from fuel to water, 890.6 W/K. More are fitted outside the card, to instruments: the pulse channel's factor for each core's sessions (0.9838, 0.9743 and 1.043; their geometric mean is 1 by convention, so two are free), the thermocouple's delay in the recording chain of 2014 and 2015 (1.336 s), and two delays in series in the chain of 2018 (1.152 s each): five free numbers outside the card and fifteen fitted in all. For a session whose channel factor is not fitted the floor on power or energy is about 4 percent at one sigma, and 7 to 8 percent between two sessions (pulses 219 and 243). Sourced and not fitted: the delayed fraction and the six groups, the fuel mass, the heat capacity, the rod's rate, and the hot-rod factor 1.387. The card's form, its targets and its compared range were set with the 43 recorded pulses read. Twelve forms read the power after the pulse too low in every bin of reactivity; the thirteenth, with the generation time fitted, is the one served. When the targets were set, the peak power of every row outside the fit had been read, and the spread and range of the thermocouple maxima of the 2015 repeats; their energies within 50 ms of the peak and the thermocouple maxima of 2018 had not. All three are outside their targets: worst 3.75 and 3.10 against 3, and RMS 2.01 against 1.5; the fourth measure outside, the peak power of 2018, reaches 3.02 on its worst row. The compared range of reactivity, 2.0 to 3.0 printed dollars, was set with the 20 rows outside the fit read, for the reason the result gives: below 2.0 dollars the card read 29, 21 and 9 percent low in peak power at 1.4, 1.6 and 1.8 dollars, and from 2.0 dollars up it was within 3.7 percent. The ten constants are the fit to the 36 pulses from 1.4 to 3.0 dollars. Rows not in the fit exist from 2.0 to 2.6 dollars on one day and at 2.5 dollars on core 217; nothing unfitted lies above 2.6 dollars. The energy within 50 ms of the peak is 0.99 correlated with peak power on the 2018 rows and is not the energy to the scram; no measure outside the fit tests the energy to the scram. A pulse determines products of the constants. A generation time of 29.59 microseconds with a printed dollar worth 0.69, and a feedback that falls back by a fifth with a time constant of 1.1 s after the pulse with a printed dollar worth 0.92 and the published 40 microseconds, fit the recorded pulses equally (pooled error 3.04 and 3.06 percent); the card takes the one with fewer constants. No constant of the card is therefore the reactor's own value of that quantity. What the pulses do determine: the inverse period for each printed dollar, 158 to 170 per second, 3 to 10 percent under the published 175, and prompt criticality at 1.27 to 1.32 printed dollars. The fitted generation time of 29.59 microseconds stands for the delayed fraction over the generation time, 237 per second against the published 175, and is not the reactor's generation time. The card cannot see the day. A printed dollar belongs to a day's reference as well as to a table; the comparison charges 0.03 dollars for it, from two pairs of days, and 22 March 2018 needed about three times that. How many of the six measures outside the fit come within their targets follows that charge: one of six with none or with 0.015 dollars, two with 0.03 and six with 0.06; on the fitted pulses nine of the twelve bin measures with 0.015 dollars, which would give a range of 2.25 to 2.75 dollars, and twelve with 0.03. 0.03 printed dollars moves the peak power by 44 percent at 1.4 dollars, 31 at 1.5, 21 at 1.6, 13 at 1.8 and 9 at 2.0 (core 231): below 2.0 dollars the uncertainty is as wide as the answer, and the card there is ill-conditioned and not biased; it computes there with the same equations. From 2.0 dollars up the day is still unseen: the two rows of 22 March 2018, printed 1.5 and 2.0 dollars, read 44 and 23 percent low, and a caller's printed 2.0 dollars of such a day is answered as the fitted day's. The power, and with it the energy and the rise of the fuel temperature, rests on the institute's pulse channel, whose calibration differed by 7.7 percent between November 2014 and March 2015 on two pulses of the same width and the same thermocouple reading; the card's power is the mean of three calibrations. The fuel temperature is the core average. It is compared with one thermocouple at the centre of one element, 6753 at position B1, with the instrument's response in between: the hot-rod factor is a calculated ratio of element powers (1.387 with 0.003, from the thesis), the factors along the element and inside it at the thermocouple are unsourced and taken as 1, and the delay is fitted. The traces are consistent with 1.387 and do not confirm it: with the factor left free they prefer 1.585 with a heat flow of 2930 W/K, and any factor from 1.39 to 1.73 fits as well or better. The card's heat flow, 890.6 W/K, is conditional on 1.387, and the thermocouple constrains the core-average rise to roughly 5 percent above and 20 percent below. That comparison tests the product of energy, heat capacity and those factors seconds after the pulse; it does not test the temperature during the pulse, in which the thermocouple shows under 4 percent of its rise, nor the temperature of the hottest fuel. No comparison isolates the dependence on the starting temperature. At 2.0 printed dollars and above the recorded pulses started at 20.9 to 29.2 degC: 24.00 to 27.94 degC on core 215, 21.31 to 29.23 on core 217 and 20.90 to 22.81 on core 231. Every row outside the fit at 2.0 printed dollars and above started inside its own table's span. On the compared pulses the rods dropped 1.51 to 7.45 s after the peak, none while the transient rod was moving. The records compared run to 14 s after the fire signal, to 18.3 MW s released, 954 MW and a core-average fuel temperature of 585 K; the quadratic feedback is fitted from 294 to about 585 K. Outside all of that the card computes the same equations. It refuses a start that is not liquid water, a reactivity of zero or less or above the rod's whole worth, a rod-worth table other than the three, a reactivity changed during a run and rods raised after the scram. The 20 rows outside the fit are other days on fitted tables, an interpolation in reactivity; the start temperature and the scram instant of each are set by rule, and those at 2.0 printed dollars and above lie inside both compared spans. The scram's worth, 8 dollars, and the rod's rate and whole worth, 4.16 dollars from the rod table of core 253, are sourced; no measurement here tests them. Nothing tests another core loading or a second pulse on warm fuel. A second program, written apart, recomputes the comparisons on the same machine; the measurements are the institute's own, and no other laboratory has repeated them.";

fn reactor_triga_jsi_card() -> Card {
    let specimen = reactor_triga_jsi::ReactorTrigaJsi::core_56_pulse_mode();
    // No frequency on the card, so no `π/ω`.
    let gate = specimen.gate_dt_s().expect("specimen gate");
    let floor = specimen.floor_dt_s().expect("specimen floor");
    let per_table = (0..3)
        .map(|table| {
            format!(
                "core {}: {} and {}",
                crate::message_number(specimen.rod_table_core[table]),
                crate::message_number(specimen.dollar_scale[table]),
                crate::message_number(specimen.dollar_offset[table])
            )
        })
        .collect::<Vec<_>>()
        .join("; ");
    finish(
        "energy",
        "reactor_triga_jsi",
        REACTOR_TRIGA_JSI_SUMMARY,
        vec![
            scalar(
                "start_temperature_k",
                Role::Phase,
                "K",
                Some(specimen.water_freezing_k),
                false,
                Some(specimen.water_boiling_k),
                false,
                format!(
                    "the fuel temperature at the fire signal, which is also the pool water's temperature, inclusive: liquid water at one standard atmosphere, from {} K, where water freezes, to {} K, where it boils (IAPWS-IF97, the pool open to the air). A start outside it is refused as StartTemperatureNotLiquidWater. Compared on starts of {}: the first thermocouple's reading at the fire signal on the recorded pulses compared at 2.0 printed dollars and above, rounded outward to the hundredth of a kelvin. Core 215 was compared from 24.0029 to 27.9375 degC (pulses 216 and 206), core 217 from 21.3171 to 29.2207 (222 and 240) and core 231 from 20.9023 to 22.8024 (331 and 340); no comparison isolates the dependence on the starting temperature. Outside those starts, on any of the three tables, the card computes the same equations",
                    crate::message_number(specimen.water_freezing_k),
                    crate::message_number(specimen.water_boiling_k),
                    crate::reactor_triga_start_spans_in_words()
                ),
            ),
            scalar(
                "rod_table_core",
                Role::Phase,
                "1",
                Some(specimen.rod_table_core[0]),
                false,
                Some(specimen.rod_table_core[2]),
                false,
                format!(
                    "exactly one of {}: the number of the core whose rod-worth table the printed dollars belong to. Core 215 was pulsed in November 2014, 217 in March 2015 and 231 in March and April 2018; all three are the same 56-element loading. A printed dollar is not the same reactivity on two tables: at the same printed 2.5 dollars the measured peak power was 439, 374 to 379 and 463 to 479 MW. The card multiplies the printed dollars by a fitted scale and takes off a fitted offset, each table's own, scale and offset in dollars: {per_table}. Any other value is refused as RodTableOutsideCard; nothing is rounded to a core, and the card holds no table of any other core",
                    crate::reactor_triga_tables_in_words()
                ),
            ),
            scalar(
                "reactivity_inserted_dollar",
                Role::Load,
                "dollar",
                Some(0.0),
                true,
                Some(specimen.rod_worth_dollar),
                false,
                format!(
                    "the reactivity the transient rod inserts in all, as the institute's pulse table prints it, in its dollars of 700 pcm; ramped in from zero over the printed value divided by 52 dollars per second, and then held. Above zero, up to and including {} printed dollars, the transient rod's whole worth on the institute's rod table of core 253: zero or less is refused as ReactivityNotPositive, since no step in reactivity is inserted and the inserted fraction divides by it, and more than the rod holds as ReactivityAboveRodWorth. The caller sends the printed value; the card applies the scale and the offset of the table named by rod_table_core, so a small insertion is still worth the table's offset, 0.05 to 0.14 dollars in the kinetics. The day's reference moves a printed dollar by about 0.03 dollars between days on one table. Compared at 2.0 to 3.0 printed dollars (fitted 1.4 to 3.0). Below 2.0 dollars the card read 29, 21 and 9 percent low in peak power at 1.4, 1.6 and 1.8 dollars, where 0.03 printed dollars of the day's reference moves the peak power by 44, 21 and 13 percent; nothing unfitted lies above 2.6 dollars. Outside 2.0 to 3.0 dollars the card computes the same equations with nothing compared: at 4.16 dollars the peak power is 2280 to 2700 MW. It cannot change during a run: a later segment with another value is refused before the first step as ReactivityChangedDuringRun",
                    crate::message_number(specimen.rod_worth_dollar),
                ),
            ),
            scalar(
                "rods_dropped",
                Role::Load,
                "1",
                Some(0.0),
                false,
                Some(1.0),
                false,
                "0 while the rods are held, 1 from the segment in which they are dropped (the scram, 8 dollars removed at once from the first step of that segment). The card does not decide when the rods drop: the caller does, at any time after the fire signal, or never. In the records the rods dropped when the thermocouple read about 260 degC in 2014 and 2015 or about 290 degC in 2018, or about 7.3 s after the peak of a weaker pulse; the comparison takes the interval from each record and drops the rods that long after the card's own peak. On the compared pulses the rods dropped 1.51 to 7.45 s after the peak (1.510 to 7.446 s, pulses 219 and 330). No compared scram came while the transient rod was moving; the 8 dollars of the scram is an assumption. A scram while the rod moves takes 8 dollars from its ramp, which goes on to the rod's stop. The longest compared hold was 7.45 s after the peak; a 3.0 dollar pulse with the rods held reaches 600 K after 5.2 to 6.4 s on cores 215 and 231. Any other value is refused as ScramValueOutsideCard; 0 after 1 as RodsRaisedAfterScram".to_string(),
            ),
        ],
        floor,
        "a locked 1e-9 s: the card's own constant, a step that still moves the clock at the end of the longest run, a million steps at the ceiling, 93.9 s",
        gate,
        "0.2 of the time constant of the prompt pole after the scram, the fitted generation time over the delayed fraction times nine; the same ceiling before the scram. The feedback after a pulse takes the net reactivity after the scram below -8 dollars: to -9.08 dollars on the compared runs, where a step at this ceiling advances 0.224 of the time constant of that pole, and on any run the card computes to -9.87 dollars (4.16 dollars on core 231 from 373.124 K, the rods dropped at the hottest fuel, 8.1 s after the fire signal), where it advances 0.241. Past the compared span the step itself also holds a ceiling that follows the state: a step longer than 0.25 of the fitted generation time over the delayed fraction times the distance of the net reactivity from one dollar, at the step's start or where it lands, is refused as DtAboveStateCeiling. No run from a start this door takes meets it at or under this ceiling. In the rise of a pulse stronger than any compared a step is taken in pieces of at most 0.03 of the time constant of the growing pole, and after the scram in pieces of 0.025 of the prompt time constant, each with its own ledger integral",
        None,
        format!(
            "start_temperature_k is in kelvin. The reactor starts at {} MW at zero net reactivity, with the fuel at the starting temperature, at the fire signal, and each precursor sum at its equilibrium with that power. The start is not stationary: the six printed group fractions sum to 0.00699891 and the power equation uses the delayed fraction 0.00700, so at zero net reactivity the power falls at 0.037 per second, as if 1.6e-4 dollar were taken out. FLAG_ROD_MOVING is set while the inserted reactivity is still below the load; the rod stops at reactivity_inserted_dollar / 52 seconds. A run is one pulse: a segment holds one reactivity_inserted_dollar, rods_dropped 0 while the rods are held, and the first segment with rods_dropped 1 is the scram, at any time after the fire signal, also while the rod moves, or never. A run that never scrams is computed on past 7.45 s after its peak, the longest compared hold: its power settles where the feedback and the heat flow to the water balance. A run is capped at 1,000,000 steps, 93.9 s at the ceiling. fuel_temperature_k is the core-average fuel temperature: the energy released, less the heat that has left to the water, over the heat capacity of the fuel. It is not the temperature of any one place in the fuel, and there is no thermocouple on the card. peak_fuel_temperature_k is its highest landed sample. energy_at_scram_mws is the energy released at the instant the rods dropped, and is 0 until then. peak_power_mw, peak_energy_mws and peak_time_s are the highest landed sample of the power, which lies within one step of the true maximum; the card does not interpolate. power_mw is the mean of three calibrations of the institute's pulse channel, one for each core's sessions; they differ by up to 7 percent. A single step takes the starting temperature and the rod table and starts again from there at each call, so single steps cannot be chained. The records compared run to {} s after the fire signal; a run goes on past it. The largest energy released in a compared run is {} MW s, the largest compared peak power {} MW and the highest compared core-average fuel temperature {} K, the fuel temperature compared through one thermocouple of one element and the hot-rod factor 1.387; past them the card computes the same equations, the quadratic feedback carried past the 294 to about 585 K it was fitted on. At the rod's whole worth the peak power is 2280 to 2700 MW and a held pulse releases 41 to 45 MW s in 93.9 s. The hottest core-average fuel any run reaches is 769.7 K (496.6 degC), at 4.16 dollars on core 231 from 373.124 K with the rods held, 8.1 s after the fire signal, 923 K (650 degC) in the hottest element through the hot-rod factor: far below 1150 degC, the limit of U-ZrH1.6 fuel in stainless clad. The card still holds that limit on the core-average fuel temperature, the one it carries: above 1423.15 K, in a state handed to its step or where a step would land, it is refused as FuelTemperatureAboveFuelLimit; no run from a start this door takes reaches it. Held on, that pulse settles at 636.6 K, where the feedback takes back the whole insertion. Past the compared span, with 3.25 printed dollars and more and the rods dropped in the rise of the pulse or at its peak, the error a step at the ceiling makes in the rise is carried by a population the scram leaves tens of times smaller, and a run at the ceiling can be stopped by the ledger more than 75 s after the fire signal, as SpontaneousGeneration or FirstLawBreach; over 14 s the largest residual found there is 3.2e-8 against the limit of 2e-7, and at half the ceiling every such run tried lands to 93.9 s. NegativeEnergy, ImpossiblePeak, ImpossibleState, FuelTemperatureAboveFuelLimit and DtAboveStateCeiling are refusals of the card's step for a state handed to it that the card did not write; this door takes the starting temperature and the rod table and nothing else of a state, and holds the step to the ceiling, so no request reaches them. A step of zero or less is refused as NonPositiveDt before the step floor is tested.",
            format!("{:e}", specimen.initial_power_mw),
            crate::message_number(reactor_triga_jsi::TIME_COMPARED_MAX_S),
            crate::message_number(reactor_triga_jsi::ENERGY_COMPARED_MAX_MWS),
            crate::message_number(reactor_triga_jsi::POWER_COMPARED_MAX_MW),
            crate::message_number(reactor_triga_jsi::FUEL_TEMPERATURE_COMPARED_MAX_K),
        ),
        &[
            "NonFiniteInput",
            "NonFiniteState",
            "NonPositiveDt",
            "InvalidCard",
            "StartTemperatureNotLiquidWater",
            "RodTableOutsideCard",
            "ReactivityNotPositive",
            "ReactivityAboveRodWorth",
            "ScramValueOutsideCard",
            "ReactivityChangedDuringRun",
            "RodsRaisedAfterScram",
            "NegativeEnergy",
            "ImpossiblePeak",
            "ImpossibleState",
            "FuelTemperatureAboveFuelLimit",
            "DtAboveStateCeiling",
            "SpontaneousGeneration",
            "FirstLawBreach",
        ],
        vec![
            Advisory {
                name: "FLAG_PROMPT_SUPERCRITICAL",
                mask: reactor_triga_jsi::FLAG_PROMPT_SUPERCRITICAL,
                seals: true,
            },
            Advisory {
                name: "FLAG_ROD_MOVING",
                mask: reactor_triga_jsi::FLAG_ROD_MOVING,
                seals: true,
            },
            Advisory {
                name: "FLAG_RODS_DROPPED",
                mask: reactor_triga_jsi::FLAG_RODS_DROPPED,
                seals: true,
            },
        ],
        slots(&[
            ("t", "Time since the fire signal, in seconds."),
            ("pos[0]", "power_mw"),
            ("pos[1]", "energy_mws"),
            ("pos[2]", "reactivity_net_dollar"),
            ("vel[0]", "fuel_temperature_k"),
            ("vel[1]", "peak_fuel_temperature_k"),
            ("vel[2]", "peak_power_mw"),
            ("force_torque", "Rate of change of power, in MW per second."),
            ("residual", "Conservation residual."),
            (
                "flags",
                "FLAG_PROMPT_SUPERCRITICAL: net reactivity above one dollar, in the card's fitted dollars and not in printed dollars, as reactivity_net_dollar is: the card scales a printed dollar by 0.67 to 0.72 and adds its table's offset, and by the pulses prompt criticality is at 1.27 to 1.32 printed dollars. FLAG_ROD_MOVING: the inserted reactivity is still below the load. FLAG_RODS_DROPPED: the rods have been dropped. All three seal.",
            ),
            ("proof", "Not written by the step."),
        ]),
    )
}

/// What the count rate, the centroid and `measured` say of a frame with no
/// measurement.
const LOS_UNMEASURED_RULE: &str = "On a frame with measured 0 it is 0, as the table writes a frame with no good centroid";

fn los_jwst_fine_guide_card() -> Card {
    let card = los_jwst_fine_guide::LosJwstFineGuide::observatory();
    let gate = card.gate_dt_s().expect("specimen gate");
    let floor = card.floor_dt_s().expect("specimen floor");
    finish(
        "aerospace",
        "los_jwst_fine_guide",
        "The line of sight of the James Webb Space Telescope in quiet fine guide, as its Fine Guidance Sensor reads it every 64 ms: a Kalman filter stepped through the flight centroids of one fine-guide file, with eighteen constants fitted to this observatory's own centroids of April 2022. On quiet frames of held-back visits it met its five marks. Inside disturbances it was measured worse than taking the last centroid, 2.03 against 1.00 mas on x and 1.57 against 0.78 on y, with its stated variance 12 to 17 times too small; it answers those frames and flags them.",
        vec![
            scalar(
                "lock_x_arcsec",
                Role::Phase,
                "arcsec",
                Some(-los_jwst_fine_guide::LOCK_FIELD_HALF_WIDTH_ARCSEC),
                false,
                Some(los_jwst_fine_guide::LOCK_FIELD_HALF_WIDTH_ARCSEC),
                false,
                format!(
                    "from -70 to 70 arcsec, both inclusive: the lock point on x in the sensor's ideal frame, where the guide star is held. It is not in the file and is the caller's. The guider's field is 2.3 by 2.3 arcmin, 69 arcsec either side of its centre; a lock point beyond 70 arcsec is refused as LockOutsideSensorField, since no star can be held there. The lock points of the files the comparison answered span {} to {} arcsec, fitted and outside the fit together, and the files outside the fit alone -61.1512947 to 49.05617, where all five targets hold on both axes; a lock point between that span and the field's edge is computed with the same equations. The arithmetic takes the lock point away from the centroid and does nothing else with it. The comparison took the median of the good centroids of the whole run, which uses centroids later than the frame being predicted. With a lock point that uses no future, the median of the first 10 s, a second comparison program gives the last-centroid, variance, correlation and per-file targets still held on quiet frames (0.793 and 0.821, 0.999 and 1.023, 0.071 and 0.095, 100 percent), and the lock-point target no longer a test; with that lock point more frames lie beyond 5 mas, and more carry FLAG_POINTING_DISTURBED",
                    token(los_jwst_fine_guide::LOCK_X_MIN_ARCSEC),
                    token(los_jwst_fine_guide::LOCK_X_MAX_ARCSEC)
                ),
            ),
            scalar(
                "lock_y_arcsec",
                Role::Phase,
                "arcsec",
                Some(-los_jwst_fine_guide::LOCK_FIELD_HALF_WIDTH_ARCSEC),
                false,
                Some(los_jwst_fine_guide::LOCK_FIELD_HALF_WIDTH_ARCSEC),
                false,
                format!(
                    "from -70 to 70 arcsec, both inclusive: the lock point on y in the sensor's ideal frame; beyond it, outside the guider's 2.3 by 2.3 arcmin field, it is refused as LockOutsideSensorField. The lock points of the files the comparison answered span {} to {} arcsec, fitted and outside the fit together, and the files outside the fit alone -62.0922546 to 50.1812668; a lock point between that span and the field's edge is computed with the same equations",
                    token(los_jwst_fine_guide::LOCK_Y_MIN_ARCSEC),
                    token(los_jwst_fine_guide::LOCK_Y_MAX_ARCSEC)
                ),
            ),
            scalar(
                "quiet_before_start_s",
                Role::Phase,
                "s",
                Some(0.0),
                false,
                None,
                false,
                format!(
                    "0 s or more: the time from the last centroid before this run that was more than {} mas from the lock point or more than four stated standard deviations from where it was expected, to the first frame of this run. The card cannot see a centroid that came before its run, so the caller states this; it cannot confirm it. Where no earlier centroid is known, as at the first frame of a fine-guide file, state {} or more. It starts the state's since_disturbance_s, and the frames that end less than {} s, 469 frames, after such a centroid carry FLAG_RECOVERING: the record's 30 s after an event, where on the fitted files the x axis is outside four of its five targets and on the files outside the fit three. The comparison with {} s carries an allowance of 2^-36 s, the most a difference of two times under 36 hours, each read from a decimal, can differ from the exact difference. Below zero it is refused as QuietBeforeStartBelowZero; a time under {} s is answered, and its frames carry FLAG_RECOVERING",
                    token(los_jwst_fine_guide::QUIET_OFFSET_MAS),
                    token(los_jwst_fine_guide::RECOVERY_S),
                    token(los_jwst_fine_guide::RECOVERY_S),
                    token(los_jwst_fine_guide::RECOVERY_S),
                    token(los_jwst_fine_guide::RECOVERY_S),
                ),
            ),
            scalar(
                "measured",
                Role::Load,
                "1",
                Some(0.0),
                false,
                Some(1.0),
                false,
                "0 or 1 and nothing between: 1 for a frame with a good centroid (centroid_time above 0 and bad_centroid_dq_flag GOOD in the table FGS CENTROID PACKET), 0 for a frame with none. A frame with measured 0 is predicted and not updated, and its stated variance grows toward its unmeasured value, 1.52574 mas squared over both axes, for any number of such frames in a row. The longest run of them a compared run crossed was 10 frames on the fitted files and 2 on the files outside the fit, and a longer run is predicted the same way".into(),
            ),
            scalar(
                "centroid_x_arcsec",
                Role::Load,
                "arcsec",
                None,
                false,
                None,
                false,
                format!(
                    "guide_star_position_x of the table, unchanged, arcsec in the sensor's ideal frame. No bound of its own: on a measured frame more than 5 mas from lock_x_arcsec the frame lands with FLAG_POINTING_DISTURBED, and more than {} mas from it the whole run is refused before its first step as OffsetOutsideCard. {LOS_UNMEASURED_RULE}",
                    token(los_jwst_fine_guide::OFFSET_MAX_MAS)
                ),
            ),
            scalar(
                "centroid_y_arcsec",
                Role::Load,
                "arcsec",
                None,
                false,
                None,
                false,
                format!(
                    "guide_star_position_y of the table, unchanged, arcsec in the sensor's ideal frame. No bound of its own: on a measured frame more than 5 mas from lock_y_arcsec the frame lands with FLAG_POINTING_DISTURBED, and more than {} mas from it the whole run is refused before its first step as OffsetOutsideCard. {LOS_UNMEASURED_RULE}",
                    token(los_jwst_fine_guide::OFFSET_MAX_MAS)
                ),
            ),
            scalar(
                "counts_per_s",
                Role::Load,
                "counts/s",
                Some(0.0),
                false,
                None,
                false,
                format!(
                    "guide_star_instrument_counts_per_sec of the table, unchanged. On a measured frame any count rate above zero: the noise variance of a centroid is the power law r_ref x (counts / 1e6)^(-r_power), which has no value at zero, and a count rate of zero or below, or under about 7e-237 counts per second where the y axis's variance exceeds the largest 64-bit number, is refused as CountRateNotPositive. The noise law was fitted on count rates from 22,910 to 2,813,637 counts per second, and every file outside the fit lay inside them, from 62,000 to 816,000; outside {} to {} a measured frame lands with FLAG_COUNT_RATE_BEYOND_COMPARED, the noise law carried on past where it was fitted. On the guide stars outside the fit the y noise exponent comes out 1.045 against the card's 1.273, so the y variance is the less certain the farther the count rate is from 1,000,000. {LOS_UNMEASURED_RULE}",
                    token(los_jwst_fine_guide::COUNTS_MIN_PER_S),
                    token(los_jwst_fine_guide::COUNTS_MAX_PER_S)
                ),
            ),
        ],
        floor,
        "a locked 0.0635 s: one frame of the sensor, 0.064 s, less half a millisecond. The centroid is an average over one frame and the noise law is that of one frame",
        gate,
        "a locked 0.0645 s: one frame of the sensor, 0.064 s, and half a millisecond. A step of another length has no centroid to go with it",
        None,
        "One segment of one step for each frame of the table, in order, with the table's own numbers as the load; the validation used dt_s 0.064. A run starts at time zero at the lock point given, with nothing known: every part at zero and each covariance at its unmeasured value. A single step starts in the same way at each call, so single steps cannot be chained; a run carries the filter from frame to frame. A frame the request itself shows to be malformed or off the card, a count rate of zero or below on a measured frame, a centroid more than 250 mas from the lock point, a measured that is neither 0 nor 1, or numbers on a frame with measured 0, refuses the whole run before its first step. Where the card was measured worse. The card describes quiet fine guide. When the pointing is disturbed the offset jumps by many mas and rings down, the filter follows slowly, and there its prediction is worse than the last centroid and its stated variance too small: inside disturbances on the files outside the fit its error one frame ahead is 2.03 mas against 1.00 for the last centroid on x and 1.57 against 0.78 on y, with the stated variance 12 to 17 times too small; over all 973,388 measured frames outside the fit, quiet and disturbed together, it is 1.35 on x and 1.26 on y of the last centroid's error, with a squared error over the stated variance of 3.9 and 3.0. The card answers those frames, with the filter's equations and its ledger closed, and says so with three flags that seal: FLAG_POINTING_DISTURBED on a measured centroid more than 5 mas from the lock point on x or y, the protocol's event sample; FLAG_BEYOND_FOUR_SIGMA on a measured centroid more than four stated standard deviations from where it was expected, which also comes by chance where the stated variance is right, once in about 7,900 measured frames, 8.4 minutes, and on the files outside the fit comes 21 times as often on x; and FLAG_RECOVERING on a frame that ends less than 30 s after the last centroid that carried either, counted from quiet_before_start_s at the start, where on the files outside the fit the x axis is outside three of the five targets. A measured frame whose count rate is outside 22,900 to 2,820,000 counts per second, the count rates the noise law was fitted on, lands with FLAG_COUNT_RATE_BEYOND_COMPARED, which seals. Each frame's slots carry where the centroid was expected before the frame was used and the variance stated for the difference, so the comparison with the centroid is the caller's: (centroid - lock) x 1000 - predicted, in mas, and its square over the variance, which averages 1 where the stated variance is right. The final state carries the sums nis_sum_x and nis_sum_y over measured_frames, and since_disturbance_s. The lock point is held to the guider's field, 70 arcsec from its centre on x and y; one beyond it is refused as LockOutsideSensorField before the first step. The comparison answered lock points from -61.1512947 to 60.4564934 arcsec on x and from -64.2767 to 50.1812668 arcsec on y, the fitted files and the files outside the fit together; a lock point nearer the field's edge is computed with the same equations. A frame with measured 0 is predicted for any number of frames in a row; the longest run of such frames a compared run crossed was 10 frames on the fitted files and 2 on the files outside the fit. The state counts them as unmeasured_in_row, and a measured frame sets it to zero. The card has no limit in time of its own; a run records at most 10,000 frames. A step of zero or less is refused as NonPositiveDt before the step floor is tested.".into(),
        &[
            "NonFiniteInput",
            "NonFiniteState",
            "NonPositiveDt",
            "InvalidObservatory",
            "LockOutsideSensorField",
            "QuietBeforeStartBelowZero",
            "InvalidCovariance",
            "MeasuredNotZeroOrOne",
            "UnmeasuredFrameCarriesNumbers",
            "CountRateNotPositive",
            "OffsetOutsideCard",
            "SpontaneousGeneration",
            "FirstLawBreach",
        ],
        vec![
            Advisory {
                name: "FLAG_NO_MEASUREMENT",
                mask: los_jwst_fine_guide::FLAG_NO_MEASUREMENT,
                seals: true,
            },
            Advisory {
                name: "FLAG_POINTING_DISTURBED",
                mask: los_jwst_fine_guide::FLAG_POINTING_DISTURBED,
                seals: true,
            },
            Advisory {
                name: "FLAG_BEYOND_FOUR_SIGMA",
                mask: los_jwst_fine_guide::FLAG_BEYOND_FOUR_SIGMA,
                seals: true,
            },
            Advisory {
                name: "FLAG_RECOVERING",
                mask: los_jwst_fine_guide::FLAG_RECOVERING,
                seals: true,
            },
            Advisory {
                name: "FLAG_COUNT_RATE_BEYOND_COMPARED",
                mask: los_jwst_fine_guide::FLAG_COUNT_RATE_BEYOND_COMPARED,
                seals: true,
            },
        ],
        slots(&[
            ("t", "Time since the start of the run, in seconds."),
            ("pos[0]", "predicted_x_mas"),
            ("pos[1]", "predicted_y_mas"),
            ("pos[2]", "innovation_variance_x_mas2"),
            ("vel[0]", "estimate_x_mas"),
            ("vel[1]", "estimate_y_mas"),
            ("vel[2]", "innovation_variance_y_mas2"),
            ("force_torque", "nis"),
            ("residual", "Conservation residual."),
            (
                "flags",
                "FLAG_NO_MEASUREMENT, bit 0: the table had no good centroid for the frame, and it was predicted and not updated; it marks no failure of the model. FLAG_POINTING_DISTURBED, bit 1: the measured centroid is more than 5 mas from the lock point. FLAG_BEYOND_FOUR_SIGMA, bit 2: it is more than four stated standard deviations from where it was expected. FLAG_RECOVERING, bit 3: the frame ends less than 30 s after the last centroid that carried bit 1 or 2. FLAG_COUNT_RATE_BEYOND_COMPARED, bit 4: the measured frame's count rate is outside 22,900 to 2,820,000 counts per second, the count rates the noise law was fitted on; the state holds no boolean of its own for it. Each seals.",
            ),
            ("proof", "Not written by the step."),
        ]),
    )
}

/// The largest speed at zero deflection the beam can start from: the largest
/// 64-bit number whose envelope, by the card's own function, is at or under
/// the amplitude ceiling.
pub fn cantilever_largest_speed_m_s(beam: &cantilever_eth::CantileverEth) -> f64 {
    let inside = |speed_m_s: f64| beam.envelope_m(0.0, speed_m_s) <= beam.amplitude_ceiling_m;
    let mut speed_m_s = beam.amplitude_ceiling_m * beam.omega0_rad_s();
    while !inside(speed_m_s) {
        speed_m_s = f64::from_bits(speed_m_s.to_bits() - 1);
    }
    loop {
        let above = f64::from_bits(speed_m_s.to_bits() + 1);
        if inside(above) {
            speed_m_s = above;
        } else {
            return speed_m_s;
        }
    }
}

/// What the two phase fields of the cantilever say of the bound they share.
const CANTILEVER_ENVELOPE_RULE: &str = "The deflection and the speed together must keep the envelope, sqrt(deflection_m^2 + (velocity_m_s / omega0)^2) with omega0 = 2 pi x 8.29235 Hz, at or below 0.01875 m, the largest amplitude the softening and the damping were fitted on (3.119 V x 0.006012 m/V); above it those laws were not fitted, and the strip's own limit, its yield at the clamp, needs its alloy, which the dataset does not state. The envelope is a size: a deflection or a speed of either sign past it is refused as EnvelopeAboveCard. Above 0.01579 m, the largest envelope the card held on the run compared with the second ring-down, outside the fit, the card answers with the same equations and ledger, and a frame there carries FLAG_ENVELOPE_ABOVE_COMPARED. Below 5.8e-5 m, the smallest envelope of the compared run, the card goes on to rest with the same equations and ledger, and a frame there carries FLAG_ENVELOPE_BELOW_COMPARED";

fn cantilever_eth_card() -> Card {
    let beam = cantilever_eth::CantileverEth::first_bending_mode();
    // The ceiling is a locked constant of the card, set by measurement and
    // far under `π/ω0`, so no `π/ω` is published.
    let gate = beam.gate_dt_s().expect("specimen gate");
    let floor = beam.floor_dt_s().expect("specimen floor");
    let largest_deflection_m = cantilever_eth::AMPLITUDE_CEILING_M;
    let largest_speed_m_s = cantilever_largest_speed_m_s(&beam);
    finish(
        "signal",
        "cantilever_eth",
        "One aluminium strip clamped at one end at ETH Zürich, ringing down freely in room air in its first bending mode: the deflection at the laser-1 station, 268 mm from the clamp, with no load.",
        vec![
            within(
                "deflection_m",
                Role::Phase,
                "m",
                largest_deflection_m,
                format!(
                    "from -{0} to {0} m, both inclusive, with the beam at rest: the deflection at the laser-1 station, 268 mm from the clamp. {CANTILEVER_ENVELOPE_RULE}. A deflection inside this range is refused as EnvelopeAboveCard when the speed is large enough; rest at zero is a start, and does not move",
                    token(largest_deflection_m)
                ),
            ),
            within(
                "velocity_m_s",
                Role::Phase,
                "m/s",
                largest_speed_m_s,
                format!(
                    "from -{0} to {0} m/s, both inclusive, with the deflection at zero: the speed of that deflection, signed. {CANTILEVER_ENVELOPE_RULE}. A speed inside this range is refused as EnvelopeAboveCard when the deflection is large enough",
                    token(largest_speed_m_s)
                ),
            ),
        ],
        floor,
        "the ceiling divided by 2^18, set where the deflection moves least: from rest at the amplitude ceiling, a turning point, one step at the floor moves the deflection by six units in the last place toward rest, and no step from one to four floors is refused there. At a quarter of the floor the deflection there moves by less than one unit in the last place while the speed grows, and such steps were refused as EnvelopeAboveCard by rounding alone",
        gate,
        "0.25 ms, about 482 steps per period, a locked constant set by measurement and not by a formula. The damping term has a sharp corner at zero deflection, which the beam crosses twice a period, so the error falls more slowly with the step than fourth-order Runge-Kutta usually gives. On the card's map over the 24 s of the compared run the largest conservation residual at this step is 5.6e-8 against a limit of 1e-6, and through the card's own step from the start of the compared ring-down to the compared floor, 5.8e-5 m, it is 7.4e-8, and over 200 s from 0.1 mm to 4.6e-13 m, 4.6e-12. Ten steps make one frame at 400 per second, the rate at which the reading procedure of the validation protocol was confirmed",
        None,
        "No load: the beam rings free, and a run segment carries an empty load object. A run starts at time zero with its ledger anchored at the deflection and the speed given. A single step starts in the same way at each call: its deflection_m and velocity_m_s handed back as a new start continue the same motion, and t_s and the heat begin again at zero. The envelope range compared with the second ring-down, outside the fit, in numbers: the compared run started 0.150 s before the start of the ring-down, on the model's own trajectory, at an envelope of 15.786 mm, and its readings are compared from the start crossing, 0.14873 s into the run, to the end of the last compared block, 23.469 s after it; over the run the card's envelope fell from 15.786 mm at its start, the largest it held, through 14.404 mm, at the first turning point after the start crossing, to 0.0587 mm. There the amplitude error is 7.03 percent (target 5), the frequency error 0.0031 Hz (target 0.010 Hz) and the timing error 0.021 of a period (target 0.20). The largest envelope compared, rounded up, is 0.01579 m. The card's ceiling is 0.01875 m: 3.119 V, the largest amplitude of the fitted recording, times 0.006012 m per volt, 18.7514 mm, rounded down; the softening and the damping are power laws fitted on amplitudes up to it, above it those laws were not fitted, and the strip's own limit, its yield at the clamp, needs its alloy, which the dataset does not state, so above it a start and a step are refused as EnvelopeAboveCard. From 0.01579 m to 0.01875 m the card answers with the same equations and the same ledger, and every frame there carries FLAG_ENVELOPE_ABOVE_COMPARED: the fitted laws, inside the amplitudes they were fitted on and above the largest envelope compared. From rest at 0.01875 m the ring-down lands with 1,149 frames flagged, the last at 0.317 s, its largest conservation residual 1.04e-7 against a limit of 1e-6. Below 0.0587 mm, rounded down to 5.8e-5 m, the card goes on toward rest with the same equations and the same ledger, and every frame there carries FLAG_ENVELOPE_BELOW_COMPARED: the fitted damping and the 8.29235 Hz frequency at vanishing amplitude are carried on below where they were compared, and the lowest frequency read on the beam was 8.22 Hz. The compared run lands 94,731 steps inside the compared envelope. From its state after 595 steps of 0.25 ms, at 0.14875 s, deflection -0.00104914043915228 m and speed 0.7408030982848752 m/s, the card gives the same deflections and speeds bit for bit, and it lands 94,136 steps, to 23.534 s, inside the compared envelope, and goes on below it. A free decay shrinks, and ends when the request's steps end; rest at zero is a start that does not move. The envelope is measured with the frequency at vanishing amplitude and the softened spring is weaker, so within one period the envelope is smallest at the rest position and rises toward the turning point, by about 1 percent at the largest swing: a start at zero deflection with its envelope within 1 percent of the ceiling is accepted and is refused as EnvelopeAboveCard a little under a quarter of a period later, and that run keeps its proof up to the last step that landed. With the envelope at 0.99 of the ceiling the step is refused after 107 steps of 0.25 ms; at 0.98 the beam rings through. For the same reason, as a ring-down crosses 0.01579 m, frames with FLAG_ENVELOPE_ABOVE_COMPARED and frames without it alternate within a period. The worked examples start at that state after 595 steps, the start of the compared ring-down. A step of zero or less is refused as NonPositiveDt before the step floor is tested.".into(),
        &[
            "NonFiniteInput",
            "NonFiniteState",
            "NonPositiveDt",
            "InvalidBeam",
            "EnvelopeAboveCard",
            "SpontaneousGeneration",
            "FirstLawBreach",
        ],
        vec![
            Advisory {
                name: "FLAG_ENVELOPE_BELOW_COMPARED",
                mask: cantilever_eth::FLAG_ENVELOPE_BELOW_COMPARED,
                seals: true,
            },
            Advisory {
                name: "FLAG_ENVELOPE_ABOVE_COMPARED",
                mask: cantilever_eth::FLAG_ENVELOPE_ABOVE_COMPARED,
                seals: true,
            },
        ],
        slots(&[
            ("t", "Time since the anchor, in seconds."),
            ("pos[0]", "deflection_m"),
            ("pos[1]", "velocity_m_s"),
            ("pos[2]", "envelope_m"),
            ("vel[0]", "kinetic_j_per_kg"),
            ("vel[1]", "potential_j_per_kg"),
            ("vel[2]", "heat_j_per_kg"),
            (
                "force_torque",
                "Margin to the largest envelope compared, in metres: 0.01579 − envelope_m, below zero on a frame with FLAG_ENVELOPE_ABOVE_COMPARED. The ceiling is 0.01875 m.",
            ),
            ("residual", "Conservation residual."),
            (
                "flags",
                "FLAG_ENVELOPE_BELOW_COMPARED, bit 0: the envelope is below 5.8e-5 m, the smallest the card was compared at on the second ring-down, outside the fit. FLAG_ENVELOPE_ABOVE_COMPARED, bit 1: the envelope is above 0.01579 m, the largest the card held on the compared run, and at or below 0.01875 m, the largest amplitude the constants were fitted on; the state holds no boolean of its own for it. Each seals. An envelope above the ceiling, 0.01875 m, and a residual past the limit are refusals.",
            ),
            ("proof", "Not written by the step."),
        ]),
    )
}

/// How a caller reads the capacity of the physical 2170 cell from a recharge
/// record. Stated on the field and in the header of the card.
/// The dry level the link's worked examples start from: the median of the 18
/// fitted events' dry levels.
pub(crate) const LINK_EXAMPLE_DRY_PATH_LOSS_DB: f64 = 52.3;

fn link_gothenburg_10010_card() -> Card {
    let link = link_gothenburg_10010::LinkGothenburg10010::sublink_587();
    let gate = link.gate_dt_s().expect("specimen gate");
    let floor = link.floor_dt_s().expect("specimen floor");
    finish(
        "signal",
        "link_gothenburg_10010",
        "One 32.445 GHz microwave link across 1.26 km of Gothenburg in rain, OpenMRG link 10010 sublink 587: its path loss from the rain rate at the gauge beside it. The rain term is Recommendation ITU-R P.838-3 with nothing fitted; the three constants of the water on the antennas are fitted to this link's own readings of June and July 2015. Compared with the nine rain events of August 2015, whose rain reached 54 mm/h, it met all three lines: RMS error 0.581 dB against 0.90.",
        vec![
            scalar(
                "dry_path_loss_db",
                Role::Phase,
                "dB",
                Some(link_gothenburg_10010::DRY_PATH_LOSS_FLOOR_DB),
                false,
                None,
                false,
                "from 44.32325388 dB up, inclusive: the sent level minus the received level with no rain and dry antennas. It is the caller's, read from the link before the rain, as the median over the hour before the first wet minute; the card holds it through the run and does not fit it. 44.32 dB is the free-space loss of this path, 124.667 dB, less the gain of two ideal 0.3 m apertures at this frequency, 40.172 dB each: a lower level is a gain no pair of these antennas can have, and is refused as DryPathLossBelowFreeSpace. Rain and antenna attenuation add to the dry level in dB, and the ledger holds at any level. The 18 fitted events lay between 51.75 and 52.55 dB, the monthly median moved 0.6 dB from June to July, and the nine compared events of August, within all three targets, lay between 52.60 and 52.90 dB".to_string(),
            ),
            scalar(
                "antenna_water_db",
                Role::Phase,
                "dB",
                Some(0.0),
                false,
                Some(link_gothenburg_10010::ANTENNA_WATER_MAX_DB),
                false,
                "from 0 to 2.13 dB, both inclusive: the attenuation of the water on the two antenna covers at the start. A link that has been dry for an hour starts at 0; covers still wet from earlier rain start above it, and a run's final state returns the value it reached. The card cannot see the rain that came before its run, so the caller states this; it cannot confirm it, and a value other than 0 is taken as stated, without from, as an honest start the card cannot tell from a continuation; a caller who states a value that is not so gets answers the comparison does not cover. 2.13 dB is the most the card gives it, a constant fitted to this link".to_string(),
            ),
            scalar(
                "gauge_minute_elapsed_s",
                Role::Phase,
                "s",
                Some(0.0),
                false,
                Some(link_gothenburg_10010::GAUGE_STEP_S),
                true,
                "from 0 s inclusive to 60 s exclusive: the seconds already spent in the current gauge minute at the start, reported and carried on by each step. No limit depends on it: the rain may change at any step. A link continued from a returned state takes that state's gauge_minute_elapsed_s. Outside the range it is refused as GaugeMinuteOutsideCard".to_string(),
            ),
            scalar(
                "gauge_minute_rain_mm_h",
                Role::Phase,
                "mm/h",
                Some(0.0),
                false,
                None,
                false,
                "0 mm/h or more: the rain rate of the last step, reported and carried on by each step. No limit depends on it and the first step may carry any rate. Below zero it is refused as NegativeRainRate. A link continued from a returned state takes that state's gauge_minute_rain_mm_h".to_string(),
            ),
            scalar(
                "rain_rate_mm_h",
                Role::Load,
                "mm/h",
                Some(0.0),
                false,
                None,
                false,
                "0 mm/h or more: the rain rate at the Barlastplatsen gauge, held through the step. Below zero it is refused as NegativeRainRate. Any rate from zero up is computed: the rain term is ITU-R P.838-3, k R^alpha L, at any rate, the water on the antennas builds toward 2.13 dB at any rate above zero, and the map and the power ledger are exact at any rate. Fitted on the one-minute rates of June and July 2015, 0 or 6 to 84 mm/h; compared with the nine rain events of August 2015, whose rain reached 54 mm/h: RMS error 0.581 dB (target 0.90), 0.585 of the error of P.838 alone (target 0.80), and 1.069 of the measured summed attenuation (target 0.75 to 1.25); in light rain under 2 mm/h the card was about 0.3 dB high. No rate above 84 mm/h was measured at this gauge. The gauge counts 0.1 mm, so every rate the card was fitted and compared on is 0 or at least 6 mm/h; a rate between them is computed with the antenna water fitted at rates of 6 mm/h and up. The frame's force_torque, 84 - rain_rate_mm_h, is below zero above the fitted rates".to_string(),
            ),
        ],
        floor,
        "the gauge's minute, 60 s, divided by 2^20: a step that moves the water on the antennas by at most seven parts in ten million of its ceiling. Under it a step is mostly rounding",
        gate,
        "364791569.8170102 s, about 11.6 years, set by the step's arithmetic and not by the data. The map of the water on the antennas is exact at any step, and each energy is its power integrated exactly over the step, so one step and many steps over the same span give the same state and the same energies to rounding, within one part in 10^13. The one number that loses figures as the step grows is the water built in a wet step, a difference of two numbers near the antennas' most water times the step; its rounding, one part in 2^52 of the step over the 81 s wetting time, is a thousandth of the antenna account's limit of one part in a million at this ceiling. The three antenna constants were fitted at the gauge's 60 s step, and the compared month was run at 60 s",
        None,
        "No load Nyquist: the rain rate is held through each step and a run segment carries one rain rate. The fit and the comparison fed rain held through whole gauge minutes at a step of 60 s; the map of the water on the antennas is exact for any rate held over a step, at any step length, so the rain may change at any step, inside a gauge minute or on its boundary. The card still reports the clock of the gauge minute: gauge_minute_elapsed_s, the seconds already spent in the current minute, and gauge_minute_rain_mm_h, the rate of the last step; no limit depends on either. A minute is complete when its clock is within half the step floor, about 2.9e-5 s, of 60 s. A run starts at time zero with its ledgers anchored at the dry level and the antenna water given, and with the gauge minute given. A single step starts in the same way at each call. Its dry_path_loss_db, antenna_water_db, gauge_minute_elapsed_s and gauge_minute_rain_mm_h handed back as a new start continue the same link and the same gauge minute. t_s and the ledgers start again from zero, and no limit depends on them.".into(),
        &[
            "NonFiniteInput",
            "NonFiniteState",
            "NonPositiveDt",
            "InvalidLink",
            "NegativeRainRate",
            "GaugeMinuteOutsideCard",
            "DryPathLossBelowFreeSpace",
            "AntennaWaterOutsideCard",
            "SpontaneousGeneration",
            "FirstLawBreach",
            "AntennaAccountBreach",
        ],
        vec![Advisory {
            name: "FLAG_RAINING",
            mask: link_gothenburg_10010::FLAG_RAINING,
            seals: true,
        }],
        slots(&[
            ("t", "Time since the anchor, in seconds."),
            ("pos[0]", "path_loss_db"),
            ("pos[1]", "rain_attenuation_db"),
            ("pos[2]", "antenna_water_db"),
            ("vel[0]", "received_level_dbm"),
            ("vel[1]", "received_power_w"),
            ("vel[2]", "received_energy_j"),
            (
                "force_torque",
                "Margin to the highest rain rate of the fitting data, in mm/h: 84 − rain_rate_mm_h, below zero for a rate above it.",
            ),
            ("residual", "Conservation residual."),
            (
                "flags",
                "FLAG_RAINING: it rained through the step that wrote the frame. It seals.",
            ),
            ("proof", "Not written by the step."),
        ]),
    )
}

fn tire_eve_r19_ice_card() -> Card {
    let tire = tire_eve_r19_ice::TireEveR19Ice::on_rig_ice();
    // The force law has no pole, so no `π/ω` is published.
    let gate = tire.gate_dt_s().expect("specimen gate");
    let floor = tire.floor_dt_s().expect("specimen floor");
    finish(
        "materials",
        "tire_eve_r19_ice",
        "One 235/55 R19 all-season tyre rolling on ice at a held slip on the Terramechanics Rig at Virginia Tech, at the rig's creep of 0.06 m/s: the longitudinal force and the work, from a friction ratio fitted to this tyre's own measurements.",
        vec![
            scalar(
                "distance_m",
                Role::Phase,
                "m",
                Some(0.0),
                false,
                None,
                false,
                "from 0 m, inclusive: where the wheel centre stands along the track at the start. A distance below zero is refused as DistanceBelowZero. The force does not depend on the distance: the rig's runs rolled about 5.7 m of ice, and a run goes on past that for as long as it goes on".into(),
            ),
            scalar(
                "slip_held_s",
                Role::Phase,
                "s",
                Some(0.0),
                false,
                None,
                false,
                "from 0 s, inclusive: the time the slip of the first step has already been held at the start. A time below zero is no time a slip can have been held and is refused as SlipHeldBelowZero. The measured force rises over the first seconds after a slip is set, and the card's force law has no rise: it gives the steady force from the instant the slip is set, where the measured force is still rising toward it, so a step that starts before 5 s lands with FLAG_SLIP_SETTLING. From 5 s to 21 s after a slip is set it is compared and inside every target, and past 21 s a frame carries FLAG_BEYOND_COMPARED. A run carries it from step to step while the slip is held, and its final state returns it; to go on from a returned state, hand back its slip_held_s with its distance_m. The card cannot see a hold that came before its run, so the caller states this; it cannot confirm it, and a caller who states a time that is not so gets answers the comparison does not cover".into(),
            ),
            scalar(
                "normal_load_n",
                Role::Load,
                "N",
                Some(0.0),
                true,
                None,
                false,
                "above 0 N: the normal load on the wheel. A load of zero or less is no contact and is refused as NoContact. The constants were fitted at 3500 and 5000 N and compared at 6500 N, outside the fit: above 5000 N a frame carries FLAG_LOAD_ABOVE_FITTED, and below 3500 N or above 6500 N, where nothing was measured, FLAG_BEYOND_COMPARED as well".into(),
            ),
            scalar(
                "inflation_pressure_pa",
                Role::Load,
                "Pa",
                Some(0.0),
                true,
                None,
                false,
                "above 0 Pa, gauge: the inflation pressure. The pressure does not enter the force. A gauge pressure of zero or less is a flat tyre and is refused as TyreFlat. The pressures measured are 1.5, 2.5 and 3.5 bar; below 150000 Pa or above 350000 Pa a frame carries FLAG_BEYOND_COMPARED".into(),
            ),
            scalar(
                "slip_ratio",
                Role::Load,
                "1",
                Some(-tire_eve_r19_ice::SLIP_CEILING),
                false,
                Some(tire_eve_r19_ice::SLIP_CEILING),
                false,
                "slip = 1 - carriage speed / rim speed: from 0.05 to 0.20 drives the wheel (traction) and from -0.20 to -0.05 retards it (braking), all four ends inclusive. A slip between -0.05 and 0.05, zero among them, is inside the published minimum and maximum and is refused as SlipOutsideCard: the measured force had not settled there".into(),
            ),
            scalar(
                "carriage_speed_m_s",
                Role::Load,
                "m/s",
                Some(tire_eve_r19_ice::CARRIAGE_SPEED_FLOOR_M_S),
                false,
                Some(tire_eve_r19_ice::CARRIAGE_SPEED_CEILING_M_S),
                false,
                "inclusive [0.05, 0.07] m/s: the speed of the carriage along the track. The files record no carriage speed; about 0.06 m/s is inferred from the wheel's 9.45 degrees a second at zero slip and the tyre's nominal radius of 0.371 m".into(),
            ),
        ],
        floor,
        "the ceiling divided by 2^32, 2^-10 s, about a sixth of the sampling interval of the measurements",
        gate,
        "2^22 s, 4,194,304 s, from the step's arithmetic. The force has no dynamics, so every work of a step is exact at any size. Below 2^42 s the spacing of 64-bit numbers is at most 2^-11 s, half the floor, so a floor step still moves the clock, the time held and the distance there; a run of 2^20 steps, more than the 1,000,000 one request takes, at the ceiling ends at 2^42 s. A step that a sum would not move is refused as StepTooSmallToResolve",
        None,
        "Over a step the normal load, the pressure, the slip and the carriage speed are held. The force is the same at every step of a held load: sign(slip) x MU x (|slip| / 0.10)^-0.132823 x normal_load_n, with MU 0.228109 in traction and 0.24162 in braking, all three numbers fitted to this tyre's own runs. The comparison takes each slip stage on the window from 10 s to 1 s before it ended. By the same cells and the same five targets on the time after each slip change, with the change placed where the wheel speed steps, the force ratio is inside all five targets in every one-second window from 5 s to 20 s, and every one-second window before 5 s is outside one: the first second gives 0.0477 at 6.5 kN (target 0.045), and 0.0449 against a constant ratio's 0.0441 over the 60 cells, and the seconds from 1 s to 5 s do no better than the constant ratio, 0.0430 against 0.0424 from 1 s to 2 s and 0.0399 against 0.0395 from 2 s to 5 s; from 5 s to 11 s the card has 0.0365 and from 11 s to 20 s 0.0337, inside every target. With the comparison's own stage cuts the window from 5 s to 11 s is inside all five targets as well. No compared slip was held longer than one stage of about 20 s, except the last slip of each run, 7 or 20 percent, which the rig held for 21 to 33 s. So a start states slip_held_s, the time the slip of the first step has already been held, from 0 s; a run carries it from step to step while the slip is held, and it starts again at zero when the slip changes. The measured force rises over the first seconds after a slip is set and the card's force law has no rise, so in those seconds the card gives the steady force where the measured force was still rising: a step that starts before 5 s, after a start that states less or after any change of slip, lands with the same force and the same ledger and carries FLAG_SLIP_SETTLING. A time held below zero is refused as SlipHeldBelowZero. From 5 s the card holds the steady force for as long as the slip is held; past 21 s, the end of the compared hold, a frame carries FLAG_BEYOND_COMPARED. The rise of the force from rest is not modelled. A run starts at time zero at distance_m with the three works at zero. A single step starts in the same way at each call: its distance_m and its slip_held_s handed back as a new start continue along the track and along the hold, and the works begin again at zero. The card cannot see a hold that came before its run, so it cannot confirm a stated slip_held_s. A value outside its range in any segment of a run refuses the whole run before its first step. The end of the rig's track is not an end of the tyre: the force does not depend on where the wheel is, and a run rolls on past 6 m. The ledger of the three works is held to 1e-9 of the larger of 10.3 kJ, the heaviest load measured at the highest friction ratio over 6 m, and the largest of the three works of the run, so it closes at the same relative size at every load and every distance.".to_string(),
        &[
            "NonFiniteInput",
            "NonFiniteState",
            "NonPositiveDt",
            "InvalidTire",
            "NoContact",
            "TyreFlat",
            "SlipOutsideCard",
            "CarriageSpeedOutsideCard",
            "DistanceBelowZero",
            "SlipHeldBelowZero",
            "StepTooSmallToResolve",
            "SpontaneousGeneration",
            "FirstLawBreach",
        ],
        vec![
            Advisory {
                name: "FLAG_LOAD_ABOVE_FITTED",
                mask: tire_eve_r19_ice::FLAG_LOAD_ABOVE_FITTED,
                seals: true,
            },
            Advisory {
                name: "FLAG_BEYOND_COMPARED",
                mask: tire_eve_r19_ice::FLAG_BEYOND_COMPARED,
                seals: true,
            },
            Advisory {
                name: "FLAG_SLIP_SETTLING",
                mask: tire_eve_r19_ice::FLAG_SLIP_SETTLING,
                seals: true,
            },
        ],
        slots(&[
            ("t", "Time since the start of the run, in seconds."),
            ("pos[0]", "distance_m"),
            ("pos[1]", "rim_travel_m"),
            ("pos[2]", "slip_ratio"),
            ("vel[0]", "wheel_work_j"),
            ("vel[1]", "drawbar_work_j"),
            ("vel[2]", "slip_heat_j"),
            (
                "force_torque",
                "Longitudinal force of the ice on the tyre, in newtons: positive pulls the carriage forward.",
            ),
            ("residual", "Conservation residual."),
            (
                "flags",
                "FLAG_LOAD_ABOVE_FITTED: the normal load is above 5000 N, the largest load the constants were fitted on. FLAG_BEYOND_COMPARED: the frame is beyond what was compared with the measurements, a normal load below 3500 N or above 6500 N, an inflation pressure below 150000 Pa or above 350000 Pa, or a slip held longer than 21 s. FLAG_SLIP_SETTLING: the step starts less than 5 s after its slip was set, where the measured force was still rising toward the card's steady force; the state holds no boolean of its own for it. All three seal.",
            ),
            ("proof", "Not written by the step."),
        ]),
    )
}

fn creep_cmsx6_card() -> Card {
    let alloy = creep_cmsx6::CreepCmsx6::at_980_c();
    // No frequency on the card, so no `π/ω`.
    let gate = alloy.gate_dt_s().expect("specimen gate");
    let floor = alloy.floor_dt_s().expect("specimen floor");
    let extension_min = creep_cmsx6::CREEP_EXTENSION_MIN_PERCENT;
    let extension_max = creep_cmsx6::CREEP_EXTENSION_MAX_PERCENT;
    let extension_max_low = creep_cmsx6::CREEP_EXTENSION_MAX_BELOW_FULL_REACH_PERCENT;
    let full_reach = creep_cmsx6::FULL_REACH_STRESS_MIN_MPA;
    let stress_min = creep_cmsx6::STRESS_MIN_MPA;
    let stress_max = creep_cmsx6::STRESS_MAX_MPA;
    finish(
        "materials",
        "creep_cmsx6",
        "One cast of the single-crystal nickel alloy CMSX-6, loaded along [001] at 980 degrees Celsius, creeping in tension under a held load: the percentage creep extension of a round test piece 6 mm in diameter against the time since the full load, from zero to 5 percent at 140 to 230 MPa. Six constants fitted to six creep tests of this cast; the time to a level met its three lines on six held-back tests, at the registered levels from 0.1 to 5 percent; the range compared at 140 MPa, 0.1 to 0.2 percent, was drawn after the scoring with every test read. Below 0.1 percent the law starts later than every measured curve, by a factor of 1.5 to 4 in the hours to 0.05 percent, and at 140 MPa past 0.2 percent it runs ahead of the three tests, by about a factor of 2 in extension at the hour they were stopped (1.70 to 1.78 percent against 0.79 to 0.83): the card answers there with those misses stated, and a frame there says so.",
        vec![
            scalar(
                "creep_extension_percent",
                Role::Phase,
                "%",
                Some(0.0),
                false,
                Some(extension_max),
                false,
                format!(
                    "from 0 percent, inclusive, the moment the full load is applied, to {extension_max} percent, inclusive, at every stress. It is the extension since the full load was applied, over the reference length, as DIN EN ISO 204 defines the percentage creep extension; the elastic and the initial plastic extension of loading are not in it. A start below zero is refused as CreepExtensionBelowZero, and one above {extension_max} percent, the card's top, as CreepExtensionOutsideCard. Below {extension_min} percent the law starts late against every measured curve, 1.5 to 4 times the hours to 0.05 percent, and above {extension_max_low} percent at a stress below {full_reach} MPa it runs ahead of the three 140 MPa tests, about 2 times the extension at the hour they stopped; the card computes there with the same law, and a frame there carries FLAG_OUTSIDE_MET_RANGE"
                ),
            ),
            scalar(
                "stress_mpa",
                Role::Load,
                "MPa",
                Some(stress_min),
                false,
                Some(stress_max),
                false,
                format!(
                    "inclusive [{stress_min}, {stress_max}] MPa: the initial stress, the held load over the cross-section before loading. The span tested. It is held through a run: a later segment with another stress is refused as StressNotHeld"
                ),
            ),
        ],
        floor,
        "a locked 1 s: a step of 1 s moves the creep extension by more than a thousand million spacings of 64-bit numbers, so the ledger's quadrature resolves the step to nine figures",
        gate,
        "0.2 over the largest rate of change of the creep rate with the creep extension that the law can reach from zero to 5 percent, at 230 MPa: about 50 s, a stability bound of the Runge-Kutta step. Its accuracy is held by the ledger: from 0.1 percent one honest step leaves at most 1.7e-10 against a limit of 2e-9. From zero extension, where the law's primary multiplier falls over the first 0.04 percent, a step of the ceiling at 200 and 230 MPa leaves more than the limit and is refused by the ledger, by name, with the state kept; a step of 5 s from zero lands at every stress through to 5 percent, the largest residual 1.1e-10 at 230 MPa",
        None,
        "The card answers one question: at a held stress, how far has the test piece crept at a given time since the full load, from zero creep extension up. t_s is the time since the full load was applied, as the card's law gives it. A run starts at the creep extension given, with empty books and the clock at the time the law itself takes to reach that extension from zero at the stress of the first segment: zero at zero, and 0.1 percent at 50.79 h at 140 MPa, 11.36 h at 170 MPa, 2.96 h at 200 MPa and 0.668 h at 230 MPa. That time since load is the quantity compared with measurement. The time between two extensions does not hold: against the measured curves the law's time from one level to the next is off by up to 1.06 in the natural logarithm, and a clock started at 0.1 percent is more than 0.30 off at 5 to 16 of 170 levels at 170, 200 and 230 MPa. Do not subtract one frame's time from another's. A single step starts in the same way at each call: its creep_extension_percent handed back as a new start continues the same test, and the clock continues with it, while the work and the heat begin again at zero. stress_mpa is held: every segment of a run carries the stress of the first, and a segment with another stress refuses the whole run before its first step as StressNotHeld; no test in the data changes its stress. A stress outside the span in any segment refuses the whole run before its first step. The temperature is 980 degrees Celsius and is not an input. The law against the worst-case target of 0.30 in the logarithm of the time to a level, over all twelve tests: from 0.1 to 5 percent at 170, 200 and 230 MPa it is within it on all nine tests at every one of 171 levels, the worst 0.289; at 140 MPa from 0.1 to 0.25 percent, the worst 0.296. Below 0.1 percent the law is slow to start against every measured curve at every stress (in hours to 0.05 percent, the law against the three test pieces, 38.9 against 15.4 to 26.7 at 140 MPa, 7.8 against 2.7 to 3.5 at 170 MPa, 1.69 against 0.61 to 0.66 at 200 MPa and 0.245 against 0.06 to 0.08 at 230 MPa). Past 0.2 percent at 140 MPa the law runs ahead: it is more than 0.30 off from 0.26 percent on one test and ahead of all three past 0.66 percent, and at the hour each test stopped, 1010 to 1030 h, it gives 1.70 to 1.78 percent against 0.79 to 0.83 measured, about a factor of 2; no 140 MPa test went past 0.83 percent, and no test lies between 140 and 170 MPa. The card answers in both regions, and every frame there carries FLAG_OUTSIDE_MET_RANGE. Under a held stress the extension grows at every step, and the step that would go past 5 percent, the card's top, is refused as CreepExtensionBeyondCard: a run that asks for more steps than that is sealed to its last landed step with that refusal, and its last landed state is within one step of 5 percent. A step that lands exactly on it lands. At the ceiling from 0.1 percent that is 4,462 steps at 230 MPa, 11,031 at 200 MPa, 30,506 at 170 MPa and 102,748 at 140 MPa, each to 5 percent, at 62.6, 156.1, 434.8 and 1477.1 h. All are inside the 1,000,000 steps of a run. The card has no limit in time: the latest hour it can reach is 1477.11 h, at 140 MPa and 5 percent. A step of zero or less is refused as NonPositiveDt before the step floor is tested.".into(),
        &[
            "NonFiniteInput",
            "NonFiniteState",
            "NonPositiveDt",
            "InvalidCard",
            "StressOutsideCard",
            "StressNotHeld",
            "CreepExtensionBelowZero",
            "CreepExtensionOutsideCard",
            "CreepExtensionBeyondCard",
            "StepTooSmallToResolve",
            "SpontaneousGeneration",
            "FirstLawBreach",
        ],
        vec![
            Advisory {
                name: "FLAG_CREEP_RATE_RISING",
                mask: creep_cmsx6::FLAG_CREEP_RATE_RISING,
                seals: true,
            },
            Advisory {
                name: "FLAG_OUTSIDE_MET_RANGE",
                mask: creep_cmsx6::FLAG_OUTSIDE_MET_RANGE,
                seals: true,
            },
        ],
        slots(&[
            ("t", "Time since the full load was applied, as the card's law gives it, in seconds."),
            ("pos[0]", "creep_extension_percent"),
            ("pos[1]", "creep_rate_percent_per_h"),
            ("pos[2]", "stress_mpa"),
            ("vel[0]", "work_by_load_mj_per_m3"),
            ("vel[1]", "heat_dissipated_mj_per_m3"),
            ("vel[2]", "work_defect_mj_per_m3"),
            ("force_torque", "Initial stress held through the step, in MPa."),
            ("residual", "Conservation residual."),
            (
                "flags",
                "FLAG_CREEP_RATE_RISING: the law's creep rate at the end of the step is above its rate at the start. The law's rate falls from 0.1 percent to a minimum near 0.26 percent and rises from there; the flag says where on its own curve the law is and marks no failure of it. FLAG_OUTSIDE_MET_RANGE: the step covers creep extension where the law is off the measured curves, below 0.1 percent at any stress, where it starts late, or past 0.2 percent below 170 MPa, where it runs ahead. Both seal.",
            ),
            ("proof", "Not written by the step."),
        ]),
    )
}

/// What the capsule says of its two speed fields: the speed through the air
/// they make, its band above 68.96 km, and the corridor below it.
const CAPSULE_SPEED_REACH: &str = "The speed through the air is the square root of the sum of the squares of radial_speed_m_s and of transverse_speed_m_s less the air's own turning, 5.4197601852574874e-05 rad/s times radius_m. Above 68.96 km, where drag is read by altitude from a table computed along the 1997 design trajectory at 12173 to 12620 m/s (12487 to 12620 m/s in the first table of AIAA 97-2304 and 12173 m/s at the 68.96 km row of its fourth; that paper does not say whether these speeds are inertial or through the air), the speed through the air must be from 11900 to 12620 m/s, the top the highest speed the table was computed at, and a state outside is refused as AirSpeedOutsideCard. The locked entry, compared with the flight, runs through 11911.98 to 12573.44 m/s there at every step from 0.05 s to 0.0005 s; from 12573.44 to 12620 m/s the card computes on the source's design speeds, past the flown entry. Below 68.96 km, where drag is read by Mach number, the Mach number must be from 0.16 to 40, the ends of that table, and a state outside is refused as MachOutsideCard; the four entries below Mach 1.0 are read from the fifteenth figure of the paper. Any path is computed on the same equations and ledger. The locked entry flew a corridor, the lowest and highest speed through the air and sine of the path angle it had within 500 m of each altitude, widened by 1 m/s and by 0.001, at knots every 500 m from 31 km to 69 km (at 40 km 1147.51 to 1350.11 m/s); a state off it is computed on the same equations and ledger. Above 86 km the speed of sound is held at its 86 km value, 274.096 m/s, so the Mach number reported there is not the real air's; the band, not the Mach number, is the limit there. A state under 31030 m and under Mach 1.23 at once, past the flown drogue deployment, is refused as PastFlightDeployment at the start; a run that reaches it is ended there, at the end the card declares. A start deep in the atmosphere at the entry speed can be refused by the ledger at the largest step and lands at a smaller one";

/// What the InSight capsule says of a speed above its entry: the two speed
/// fields publish no bound of their own; the speed through the air is held.
const CAPSULE_INSIGHT_SPEED_REACH: &str = "The speed has no bound of its own. Its reach: the speed through the air, which is this speed less the air's own turning with Mars, must be at or above Mach 0.9, the first entry of the drag table read by Mach number, or the state is refused as MachOutsideCard, and at most 5850 m/s, where the drag table read by speed ends, or it is refused as AirSpeedOutsideCard. A state under 6164.62 m and under 352.1169 m/s at once is past the flown parachute deployment and is refused as PastFlightDeployment at the start; a run that reaches it is ended there. Any path is computed on the same equations and ledger; the compared runs fly a corridor, never above 5546.4 m/s and never climbing, and a state off it is computed on the same equations and ledger";

fn capsule_insight_card() -> Card {
    let specimen = capsule_insight::CapsuleInsight::entry_capsule();
    // No pole on the card, so no `π/ω`.
    let gate = specimen.gate_dt_s().expect("specimen gate");
    let floor = specimen.floor_dt_s().expect("specimen floor");
    let datum_m = capsule_insight::DATUM_RADIUS_M;
    let lift = capsule_insight::LIFT_OVER_DRAG_LIMIT;
    finish(
        "aerospace",
        "capsule_insight",
        "The InSight entry capsule on its entry at Mars of 26 November 2018, from the top of the atmosphere to the flown parachute deployment, under 6164.62 m and under 352.1169 m/s at once: no wind, the air of orbiter soundings set on the lander's barometer, and the capsule's lift as the load. Compared with the flight from the locked entry state: with the lift measured on the first 150 s all four of NASA's first lines were met, the radius at deployment 114 m low against a line of 3.5 km and Mach 1.501 against 1.522. Other starts are computed down to the landing site and were not compared. It does not forecast an entry whose lift is not known.",
        vec![
            scalar(
                "radius_m",
                Role::Phase,
                "m",
                None,
                false,
                None,
                false,
                format!(
                    "distance from the centre of Mars. Altitude is radius_m - {} and is in [-2634.5, 128000] m: below -2634.5 m, the landing site's radius of 3393079.5 m, the capsule is at or under the ground and the state is refused as BelowLandingSite, and above 128 km as AltitudeOutsideCard. The air from 4 km up is the card's table; under 4 km the same law, the hydrostatic column from the lander's barometer, 733.724 Pa at the landing site, is carried down in 35 knots. The compared run with the flight's lift reaches down to 5793 m; below that the card computes the same equations, and a run ends at the flown deployment, under 6164.62 m and 352.1169 m/s at once",
                    token(datum_m)
                ),
            ),
            scalar(
                "radial_speed_m_s",
                Role::Phase,
                "m/s",
                None,
                false,
                None,
                false,
                format!("negative going down, in the frame that does not turn with Mars. {CAPSULE_INSIGHT_SPEED_REACH}"),
            ),
            scalar(
                "transverse_speed_m_s",
                Role::Phase,
                "m/s",
                None,
                false,
                None,
                false,
                format!("in the frame that does not turn with Mars. {CAPSULE_INSIGHT_SPEED_REACH}"),
            ),
            within(
                "lift_over_drag",
                Role::Load,
                "1",
                lift,
                format!(
                    "from -{0} to {0}, both inclusive: the capsule's lift divided by its drag, in the flight plane, positive away from Mars, held through the step. Zero is a capsule with no lift. The limit is the largest trim value the 2008 aerodynamic database states, for a centre of gravity 5 mm off the axis. The flight's own value, measured on its first 150 s, was -0.031",
                    lift
                ),
            ),
        ],
        floor,
        "the ceiling divided by 2^20: 0.11 mm of fall at entry. Under it a step is mostly rounding",
        gate,
        "the largest step the card stands behind, a locked constant, stated for the entry from the locked entry state: there the capsule falls 120 m in one step, under one interval of the air tables, and over the whole entry the conservation residual stays near a thousandth of its limit. A start deep in the atmosphere at entry speed, a state the entry itself never reaches, can be refused by the ledger at this step and lands at a smaller one",
        None,
        "lift_over_drag is in the published span, and a later segment of a run may change it. A lift outside that span in any segment refuses the whole run before its first step. A run from the entry state is ended as PastFlightDeployment, the end the card declares, at the first landed state under both 6164.62 m and 352.1169 m/s, the last row of the flight's record, the flown parachute deployment 209.27 s after the entry state: with no lift at step 2321, 232.1 s, 6163.7 m and Mach 1.373; with a lift over drag of -0.031 at step 2106, 210.6 s, 5793.3 m and Mach 1.470, the lowest state the compared run reached. Compared with the flight: runs from the locked entry state at 0.1 s, with one lift over drag from -0.08 to 0.08 held through the run, or with -0.031 or 0 to 150 s and one lift in that span after. They flew a corridor, the lowest and highest speed through the air and sine of the path angle within 500 m of each altitude, widened by 1 m/s and by 0.001, at knots every 500 m from 5.5 km to 126.5 km; any other start or path is computed on the same equations, and a run whose lift changes in any other way goes on. A restart from a returned state lands where the run does. A new start sets the time, the angle and the drag-loss ledger to zero and anchors the ledger there; no limit depends on them.".into(),
        &[
            "NonFiniteInput",
            "NonFiniteState",
            "NonPositiveDt",
            "InvalidCapsule",
            "LiftOutsideCard",
            "AltitudeOutsideCard",
            "BelowLandingSite",
            "MachOutsideCard",
            "AirSpeedOutsideCard",
            "PastFlightDeployment",
            "SpontaneousGeneration",
            "FirstLawBreach",
        ],
        vec![Advisory {
            name: "FLAG_LIFT_ACTING",
            mask: capsule_insight::FLAG_LIFT_ACTING,
            seals: true,
        }],
        slots(&[
            ("t", "Time since the anchor, in seconds."),
            ("pos[0]", "altitude_m"),
            ("pos[1]", "ground_distance_m"),
            ("pos[2]", "mach"),
            ("vel[0]", "air_speed_m_s"),
            ("vel[1]", "radial_speed_m_s"),
            ("vel[2]", "dynamic_pressure_pa"),
            ("force_torque", "Load, in Earth g: drag and lift together."),
            ("residual", "Conservation residual."),
            (
                "flags",
                "FLAG_LIFT_ACTING: the step was taken with a lift that is not zero. It seals.",
            ),
            ("proof", "Not written by the step."),
        ]),
    )
}

fn capsule() -> Card {
    let specimen = capsule_stardust::CapsuleStardust::sample_return_capsule();
    // No pole on the card, so no `π/ω`.
    let gate = specimen.gate_dt_s().expect("specimen gate");
    let earth_m = capsule_stardust::LOCAL_EARTH_RADIUS_M;
    let floor = specimen.floor_dt_s().expect("specimen floor");
    finish(
        "aerospace",
        "capsule_stardust",
        "The Stardust sample return capsule on its entry of 15 January 2006, from entry interface down to the drogue-deployment condition the radar recorded, under 31030 m and under Mach 1.23 at once, past which a state is refused: no lift, no wind, a yearly standard atmosphere, and no load. One entry was compared with the flight, the locked entry from entry interface, and it met all three lines: peak load 32.17 g against 32.89 g, and the drogue altitude and Mach number reached at 138.909 s and 136.953 s against 137.9 s recorded. Other paths are computed inside the tables, down to sea level, and were not compared.",
        vec![
            scalar(
                "radius_m",
                Role::Phase,
                "m",
                None,
                false,
                None,
                false,
                format!(
                    "distance from the Earth's centre. Altitude is radius_m - {} and is in [0, 134700] m: above 134.7 km, the top of the drag table read by altitude, a state is refused as AltitudeOutsideCard, and below 0 m, the sea level of the 1976 standard and the ellipsoid under the path, as BelowStandardSeaLevel; the card has no terrain. The air from 30 km up is the card's table of the 1976 standard; under 30 km the same standard's formulas in 151 knots from sea level. The flight was compared down to 31.03 km",
                    token(earth_m)
                ),
            ),
            scalar(
                "radial_speed_m_s",
                Role::Phase,
                "m/s",
                None,
                false,
                None,
                false,
                format!("negative going down, in the frame that does not turn with the Earth. {CAPSULE_SPEED_REACH}"),
            ),
            scalar(
                "transverse_speed_m_s",
                Role::Phase,
                "m/s",
                None,
                false,
                None,
                false,
                format!("in the frame that does not turn with the Earth. {CAPSULE_SPEED_REACH}"),
            ),
        ],
        floor,
        "the ceiling divided by 2^20: 88 micrometres of fall at entry. Under it a step is mostly rounding",
        gate,
        "the largest step the card stands behind, a locked constant, stated for the entry from entry interface: over the whole entry the conservation residual stays under a thousandth of its limit. Its reach: a start deep in the atmosphere at entry speed, a state the entry itself never reaches, is where the step method's own error at this step can exceed the conservation limit, and the ledger refuses it by name; a smaller step takes it, and from 40 km at the entry speed a hundred steps of 0.005 s land",
        None,
        "No load. A run segment carries an empty load object. A run from entry interface at the ceiling step lands 2779 steps, the last the first under both 31030 m and Mach 1.23, and is ended there as PastFlightDeployment, the end the card declares, the step past it not taken: a run asked for exactly those 2779 steps is ended there too. The step that first meets both drogue conditions lands, so a run at a smaller step can end earlier, by less than one of the larger steps: at 0.005 s the run lands 27782 steps, to 31029.67 m at 138.91 s. A start or a landed state off the corridor the locked entry flew is computed; a restart from a state the entry returns lands where the run does. A new start sets the time, the angle and the drag-loss ledger to zero and anchors the ledger there; no limit depends on them.".into(),
        &[
            "NonFiniteInput",
            "NonFiniteState",
            "NonPositiveDt",
            "InvalidCapsule",
            "AltitudeOutsideCard",
            "BelowStandardSeaLevel",
            "MachOutsideCard",
            "AirSpeedOutsideCard",
            "PastFlightDeployment",
            "SpontaneousGeneration",
            "FirstLawBreach",
        ],
        vec![Advisory {
            name: "FLAG_DROGUE_MACH_WINDOW",
            mask: capsule_stardust::FLAG_DROGUE_MACH_WINDOW,
            seals: true,
        }],
        slots(&[
            ("t", "Time since the anchor, in seconds."),
            ("pos[0]", "altitude_m"),
            ("pos[1]", "stagnation_pressure_pa"),
            ("pos[2]", "mach"),
            ("vel[0]", "recovery_enthalpy_j_kg"),
            ("vel[1]", "heat_transfer_coefficient_kg_m2_s"),
            ("vel[2]", "heat_flux_w_m2"),
            ("force_torque", "Load, in Earth g."),
            ("residual", "Conservation residual."),
            ("flags", "FLAG_DROGUE_MACH_WINDOW. It seals."),
            ("proof", "Not written by the step."),
        ]),
    )
}

fn satellite() -> Card {
    let specimen = satellite_lageos1::SatelliteLageos1::lageos1();
    let floor = specimen.floor_dt_s().expect("specimen floor");
    let last_start_epoch_utc_s = specimen.last_start_epoch_utc_s().expect("latest start");
    // The ceiling is a locked constant of the card, so no `π/ω`.
    let gate = specimen.gate_dt_s().expect("specimen gate");
    let mut epoch = scalar(
        "epoch_utc_s",
        Role::Phase,
        "s",
        Some(satellite_lageos1::EPOCH_MIN_S),
        false,
        Some(last_start_epoch_utc_s),
        false,
        format!(
            "UTC at the start of the run, in seconds after 2000-01-01 12:00 UTC: 2017-01-01 00:00 ({} s), the last leap second, from which TT - UTC is the card's locked 69.184 s, to the latest start from which the card's smallest step still lands on or before 2027-06-28 00:00 ({} s), when the IERS leap-second file of Bulletin C 72, of July 2026, expires; Bulletin C 72 announced no leap second at the end of December 2026, and the next the IERS can announce is at the end of June 2027. Outside those dates TT - UTC is not the card's, or not yet known, and a step is refused as EpochOutsideLeapSecondSpan. The card was compared on 28 days, 2026-08-23 to 2026-09-19, the last run ending 2026-09-20 00:00; a date outside them is computed. The Sun and the Moon are placed by a short analytic series to about a tenth of a degree at any of these dates, and the field's yearly rates are not applied",
            token(satellite_lageos1::EPOCH_MIN_S),
            token(satellite_lageos1::EPOCH_MAX_S)
        ),
    );
    epoch.components = 1;
    let forecast = scalar(
        "forecast_elapsed_s",
        Role::Phase,
        "s",
        Some(0.0),
        false,
        None,
        false,
        format!(
            "0 s or more: the time since the state was taken from a published orbit. Required. State 0 for a state turned from a published orbit, and the forecast_elapsed_s of a returned state for a state handed back; below zero it is refused as ForecastElapsedBelowZero. It is reported plus t_s and no limit depends on it. The compared forecasts were at most {} s, seven days, from a published state: the worst distance after seven days was 32 m for LAGEOS-1 and 22 m for LAGEOS-2, against a target of 100 m. A longer forecast is computed; the card has no drag, no light reflected from the Earth and no ocean tide, so its error grows with the length of the forecast. The card cannot confirm the number stated",
            token(satellite_lageos1::FORECAST_HORIZON_S)
        ),
    );
    let position = vec3_stated(
        "position_m",
        "m",
        "in the card's turning frame. Its length is 6 378.1363 km or more, the Earth's equatorial radius, to which the field is scaled; below it the satellite is under the sphere of the Earth's equator, in the ground at the equator, where the card's field, the series of the Earth's field outside the Earth, is not the field, and the start is refused as RadiusBelowEarthRadius. The compared runs reached radii from 11 997.57 km (LAGEOS-2 near perigee) to 12 326.40 km (LAGEOS-2 near apogee); a larger radius is computed, and a smaller one down to the equatorial radius, each landing under 11 997 km carrying FLAG_BELOW_COMPARED_RADIUS. The card carries no air, so the lower the orbit, the larger the drag on the sphere that it does not carry".into(),
    );
    let velocity = vec3_stated(
        "velocity_m_s",
        "m/s",
        "in the card's turning frame. Any orbit is computed, closed or not. Compared on two orbits: LAGEOS-1, semi-major axis 12 265.14 to 12 274.83 km, eccentricity 0.003438 to 0.004375, inclination 109.8039 to 109.8235 degrees; LAGEOS-2, 12 158.51 to 12 165.68 km, 0.013232 to 0.013761, 52.6315 to 52.6508 degrees, each worked out from the position and the velocity seen from a frame that does not turn, with the card's z axis as the pole. On the 42 runs outside the choice of structure: worst 0.65 m after one revolution, 3.8 m after one day, 32 m after seven days, against targets of 1, 10 and 100 m".into(),
    );
    finish(
        "aerospace",
        "satellite_lageos1",
        "The laser-ranging satellite LAGEOS-1 in free flight, in a frame turning with the Earth: the Earth's field to degree and order 12, the Sun and the Moon, the solid tide, sunlight pressure and relativity, with no load. Compared with the published orbits of LAGEOS-1 and LAGEOS-2 on 28 days of 2026 for up to seven days, it met every line on 42 held-back runs: worst 0.65 m after one revolution, 3.8 m after one day, 32 m after seven days.",
        vec![epoch, forecast, position, velocity],
        floor,
        "the ceiling divided by 2^20: 0.054 m of flight. Under it a step is mostly rounding",
        gate,
        "the largest step the card stands behind, a locked constant. Measured against an 8th-order reference from the worked example of engine 0.6.0, a circular polar orbit, the step error at this step is 11.2 mm after one day and 13.9 cm after seven days, and 0.4 mm to 21 mm after one day from four other start states inside the 0.6.0 bands. The step was measured for orbits near LAGEOS's; a lower or faster orbit needs a smaller step, and the two ledgers refuse one taken too large. The measurement is a recomputation outside the engine in 50-digit arithmetic, by one reader, not yet reviewed. Below the compared radii a lower orbit is faster and needs a smaller step, and the two ledgers refuse one taken too large: measured on circular orbits inclined 51.6 degrees, at 10 s the energy ledger reaches its limit and the step is refused by name after about 1.5 days at 6 400 km, 2.5 days at 6 700 km and 3.9 days at 7 000 km, and at 5 s a week at 6 700 km leaves 8.8e-11 and at 6 400 km 1.4e-10; over one revolution from 6 700 km to 11 000 km the 10 s step lands within 1.7 cm of steps of 2.5 s",
        None,
        "No load. A run segment carries an empty load object. Compared with published orbits: two orbits, LAGEOS-1 and LAGEOS-2, on 28 calendar days, 2026-08-23 to 2026-09-19, in 56 runs of 10 s steps of at most seven days each, the last ending 2026-09-20 00:00 UTC. On the 42 runs outside the choice of structure, against targets of 1 m after one revolution, 10 m after one day and 100 m after seven days: LAGEOS-1 worst 0.65 m, 3.8 m and 32 m, LAGEOS-2 worst 0.34 m, 2.9 m and 22 m. The card computes past all of it with the same equations: any orbit with a radius of 6 378.1363 km or more, the Earth's equatorial radius, any forecast length, any date from 2017-01-01 00:00 to 2027-06-28 00:00 UTC. A landing under 11 997 km, the lowest point a compared run reached, carries FLAG_BELOW_COMPARED_RADIUS, which seals; the card carries no air, so the lower the orbit, the larger the drag on the sphere that it does not carry. The lowest radius and the dates are applied to the incoming state and to every landing and refuse by name outside: RadiusBelowEarthRadius and EpochOutsideLeapSecondSpan. Two ledgers are closed at every step: the energy seen from the turning frame against the work of the Sun, the Moon, the tide, sunlight and relativity, and the angular momentum about the Earth's axis against the torque of the field and those forces. The second is the one a Coriolis term with the wrong sign breaks, which does no work and which the energy ledger cannot see. residual_conservation_error is the larger of the two residuals, each against a limit of 1e-9: over a week at 10 s from the test state they reach 1.5e-11 and 1.1e-11. The last date and rounding: the clock of a run is a running sum, t_s += dt_s in 64-bit arithmetic, and the time of a state is the start epoch plus that clock, rounded to the spacing of 64-bit numbers at these dates, at most 1.19e-7 s. The comparison with 2027-06-28 00:00 therefore carries a named allowance: EPOCH_END_ALLOWANCE_S, 1.19e-7 s, and CLOCK_ROUNDING_PER_STEP, one part in 2^53 of the clock, for each step taken so far at this step and two more, and never more than half the step. It is 1.2e-7 s on the first step of a run and 1.1e-5 s after 1,000,000 steps of 0.1 s. A run whose end in decimal is exactly the last date lands its last step, and its last epoch_utc_s can read up to the allowance past it; handed back as a new start, that value is past the card and is refused. The allowance is in the comparison a step makes and not in the published maximum of epoch_utc_s. A single first step that ends exactly on the last date lands.".into(),
        &[
            "NonFiniteInput",
            "NonFiniteState",
            "NonPositiveDt",
            "InvalidSatellite",
            "RadiusBelowEarthRadius",
            "EpochOutsideLeapSecondSpan",
            "ForecastElapsedBelowZero",
            "SpontaneousGeneration",
            "FirstLawBreach",
            "AngularMomentumBreach",
        ],
        vec![
            Advisory {
                name: "FLAG_EARTH_SHADOW",
                mask: satellite_lageos1::FLAG_EARTH_SHADOW,
                seals: true,
            },
            Advisory {
                name: "FLAG_BELOW_COMPARED_RADIUS",
                mask: satellite_lageos1::FLAG_BELOW_COMPARED_RADIUS,
                seals: true,
            },
        ],
        slots(&[
            ("t", "t_s, 64-bit"),
            (
                "pos[0..3]",
                "x, y, z, each rounded to the nearest 32-bit number, metres",
            ),
            (
                "vel[0..3]",
                "x, y, z less the number in pos, metres: at most 0.5 m in size",
            ),
            ("force_torque", "radius_m − REFERENCE_RADIUS_M, metres"),
            ("residual", "residual_conservation_error"),
            ("flags", "FLAG_EARTH_SHADOW and FLAG_BELOW_COMPARED_RADIUS, or 0"),
            ("proof", "Not written by the step."),
        ]),
    )
}

/// What a caller must do to a start state before this card takes it, in the
/// words of the card's header.
const SATELLITE_START_STATE: &str = "The published orbit files are in a frame fixed to the Earth's crust, which does not turn uniformly. A state must be turned into the card's frame before it is given to the card. That is done outside the engine; the compared runs did it with `evidence/research/lageos1_orbit/analysis/10_engine_comparison.py` and its `earth_orientation.py` and `subdaily_earth_rotation.py`, as follows. The card's z axis is the celestial intermediate pole of the start epoch, from the IAU 2006/2000A precession and nutation with the celestial pole offsets dX and dY of IERS Bulletin A, held fixed in the celestial frame (GCRS) through the run. Its x axis at the start is where the Earth Rotation Angle of the start epoch puts it, from UT1 − UTC of Bulletin A, so that at the start the card's frame is the crust-fixed frame with the pole's motion against the crust, Bulletin A's x and y, turned out. From there the frame turns about that fixed axis at the locked rate 7.29211514670698e-5 rad/s, 2π × 1.00273781191135448 / 86400, and follows neither the changing length of the day nor any motion of the pole after the start. The pole's x and y and UT1 carry the diurnal and semi-diurnal ocean-tide terms of the IERS Conventions (2010), routine ORTHO_EOP; the libration terms are not applied. The velocity is turned with the time derivative of the whole turning from the crust to the celestial frame, at the true spin rate and with the slow motions of the pole, which holds for a velocity that is the exact rate of change of the crust-fixed position, as GFZ's is: the compared runs started from GFZ's file, and the combined orbit's velocity record is not such a rate. Then the locked rate's ω × r is taken off. A crust-fixed state fed straight in starts 22.9 m from the state so turned, the pole's offset from the crust's z axis alone, and through the engine it was 69 to 81 m off the published orbit after one revolution, 374 to 385 m after one day and 2.7 km after seven days, from the worked example's start, LAGEOS-1 at 2026-09-06 0 h UTC. `epoch_utc_s` in a request is UTC at the start of the run in seconds after 2000-01-01 12:00 UTC, 86,400 to each day: the five leap seconds since then are not counted, so 2017-01-01 00:00 UTC is 536500800 s; a count that includes them is 5 s larger, and from the same start it moved the distance after seven days from 31.9 to 33.6 m. The `epoch_utc_s` of a returned state is the UTC of that state: the start epoch plus `t_s`, worked out from the two at each state with one rounding and not kept as a sum that gains a step at a time. `t_s` counts from the start of the run. A final state handed back as a new start, with its `epoch_utc_s`, `position_m` and `velocity_m_s`, continues the same orbit, and the new run's `t_s` begins again at zero. `forecast_elapsed_s` in a request is the time since the state was taken from a published orbit: zero for a state turned from a published orbit, and the `forecast_elapsed_s` of a returned state for a state handed back. It is required, and reported plus `t_s`: the compared forecasts were at most seven days long, and no limit depends on it. The card cannot confirm the number a caller states: a state stated as fresh that is not gets answers the comparison does not cover.";

/// How a reader rebuilds the position from one recorded frame, in the words
/// of the card's header, with the frame's byte layout and the one constant
/// the slots use.
const SATELLITE_FRAME_RULE: &str = "A reader adds `pos[i]` and `vel[i]` as 64-bit numbers and has each coordinate of the position to better than 3 × 10⁻⁸ m. The velocity is in the state. REFERENCE_RADIUS_M is 12270000 m.";

/// The start of the seven-day run of LAGEOS-1 week 260912, 2026-09-06 0 h
/// UTC, one of the held-back weeks, as the saved validation request gives
/// it: the published orbit turned into the card's frame, with no time since
/// it.
pub(crate) const SATELLITE_EXAMPLE_EPOCH_UTC_S: f64 = 841_924_800.0;
pub(crate) const SATELLITE_EXAMPLE_POSITION_M: [f64; 3] =
    [295558.7678903363, -5108650.370522532, -11100383.23606335];
pub(crate) const SATELLITE_EXAMPLE_VELOCITY_M_S: [f64; 3] =
    [-4821.08544993592, -3331.9602315669836, 1405.5539438152634];

pub(crate) fn satellite_example_state() -> satellite_lageos1::SatelliteState {
    let specimen = satellite_lageos1::SatelliteLageos1::lageos1();
    satellite_lageos1::SatelliteState::try_state(
        &specimen,
        SATELLITE_EXAMPLE_EPOCH_UTC_S,
        0.0,
        SATELLITE_EXAMPLE_POSITION_M,
        SATELLITE_EXAMPLE_VELOCITY_M_S,
    )
    .expect("the example orbit is on the card")
}

/// The phase fields of the solar_system card: a position and a velocity for
/// each of the thirteen bodies, in the card's own order.
pub(crate) const SOLAR_SYSTEM_POSITION_KEYS: [&str; solar_system::BODIES] = [
    "sun_position_m",
    "mercury_position_m",
    "venus_position_m",
    "earth_position_m",
    "moon_position_m",
    "mars_position_m",
    "jupiter_position_m",
    "saturn_position_m",
    "uranus_position_m",
    "neptune_position_m",
    "ceres_position_m",
    "pallas_position_m",
    "vesta_position_m",
];

pub(crate) const SOLAR_SYSTEM_VELOCITY_KEYS: [&str; solar_system::BODIES] = [
    "sun_velocity_m_s",
    "mercury_velocity_m_s",
    "venus_velocity_m_s",
    "earth_velocity_m_s",
    "moon_velocity_m_s",
    "mars_velocity_m_s",
    "jupiter_velocity_m_s",
    "saturn_velocity_m_s",
    "uranus_velocity_m_s",
    "neptune_velocity_m_s",
    "ceres_velocity_m_s",
    "pallas_velocity_m_s",
    "vesta_velocity_m_s",
];

fn solar_system_card() -> Card {
    let specimen = solar_system::SolarSystem::de440();
    // The ceiling is a locked constant of the card, so no `π/ω`.
    let gate = specimen.gate_dt_s().expect("specimen gate");
    let floor = specimen.floor_dt_s().expect("specimen floor");
    let fields = vec![
        vec3_stated("sun_position_m", "m", "of the Sun from the barycentre of the Solar System, ICRF axes. No band of its own: every other body's band is measured from it. The centre of mass of the thirteen bodies may be at any distance from the origin; in the compared run, to the last range compared with measurement, it went from 98409 m to 114929 m. A state moved bodily is the same motion with other rounding: moved 6e13 m, the Moon is 98.5 m off after a year".into()),
        vec3_stated("sun_velocity_m_s", "m/s", "of the Sun. The centre of mass of the thirteen bodies must move no faster than 0.01 m/s".into()),
        vec3_stated("mercury_position_m", "m", "of Mercury from the barycentre of the Solar System, ICRF axes. Its distance from the Sun is from 0.29 to 0.48 au; the compared run, to the last range compared with measurement, went through 0.30 to 0.47 au, and a distance outside that, inside the band, is computed".into()),
        vec3_stated("mercury_velocity_m_s", "m/s", "of Mercury. The semi-major axis of its orbit about the Sun is from 0.29 to 0.48 au".into()),
        vec3_stated("venus_position_m", "m", "of Venus from the barycentre of the Solar System, ICRF axes. Its distance from the Sun is from 0.70 to 0.74 au; the compared run, to the last range compared with measurement, went through 0.71 to 0.73 au, and a distance outside that, inside the band, is computed".into()),
        vec3_stated("venus_velocity_m_s", "m/s", "of Venus. The semi-major axis of its orbit about the Sun is from 0.70 to 0.74 au".into()),
        vec3_stated("earth_position_m", "m", "of the Earth from the barycentre of the Solar System, ICRF axes. Its distance from the Sun is from 0.97 to 1.03 au; the compared run, to the last range compared with measurement, went through 0.98 to 1.02 au, and a distance outside that, inside the band, is computed".into()),
        vec3_stated("earth_velocity_m_s", "m/s", "of the Earth. The semi-major axis of its orbit about the Sun is from 0.97 to 1.03 au".into()),
        vec3_stated("moon_position_m", "m", "of the Moon from the barycentre of the Solar System, ICRF axes. Its distance from the Earth is from 340 000 km to 420 000 km; the compared run, to the last range compared with measurement, went through 356 000 km to 407 000 km, and a distance outside that, inside the band, is computed".into()),
        vec3_stated("moon_velocity_m_s", "m/s", "of the Moon. The semi-major axis of its orbit about the Earth is from 340 000 km to 420 000 km".into()),
        vec3_stated("mars_position_m", "m", "of the Mars system barycentre from the barycentre of the Solar System, ICRF axes. Its distance from the Sun is from 1.35 to 1.70 au; the compared run, to the last range compared with measurement, went through 1.38 to 1.67 au, and a distance outside that, inside the band, is computed".into()),
        vec3_stated("mars_velocity_m_s", "m/s", "of the Mars system barycentre. The semi-major axis of its orbit about the Sun is from 1.35 to 1.70 au".into()),
        vec3_stated("jupiter_position_m", "m", "of the Jupiter system barycentre from the barycentre of the Solar System, ICRF axes. Its distance from the Sun is from 4.8 to 5.6 au; the compared run, to the last range compared with measurement, went through 4.94 to 5.46 au, and a distance outside that, inside the band, is computed".into()),
        vec3_stated("jupiter_velocity_m_s", "m/s", "of the Jupiter system barycentre. The semi-major axis of its orbit about the Sun is from 4.8 to 5.6 au".into()),
        vec3_stated("saturn_position_m", "m", "of the Saturn system barycentre from the barycentre of the Solar System, ICRF axes. Its distance from the Sun is from 8.8 to 10.3 au; the compared run, to the last range compared with measurement, went through 9.03 to 9.97 au, and a distance outside that, inside the band, is computed".into()),
        vec3_stated("saturn_velocity_m_s", "m/s", "of the Saturn system barycentre. The semi-major axis of its orbit about the Sun is from 8.8 to 10.3 au".into()),
        vec3_stated("uranus_position_m", "m", "of the Uranus system barycentre from the barycentre of the Solar System, ICRF axes. Its distance from the Sun is from 18.0 to 20.4 au; the compared run, to the last range compared with measurement, went through 19.88 to 20.10 au, and a distance outside that, inside the band, is computed".into()),
        vec3_stated("uranus_velocity_m_s", "m/s", "of the Uranus system barycentre. The semi-major axis of its orbit about the Sun is from 18.0 to 20.4 au".into()),
        vec3_stated("neptune_position_m", "m", "of the Neptune system barycentre from the barycentre of the Solar System, ICRF axes. Its distance from the Sun is from 29.5 to 30.7 au; the compared run, to the last range compared with measurement, went through 29.96 to 30.14 au, and a distance outside that, inside the band, is computed".into()),
        vec3_stated("neptune_velocity_m_s", "m/s", "of the Neptune system barycentre. The semi-major axis of its orbit about the Sun is from 29.5 to 30.7 au".into()),
        vec3_stated("ceres_position_m", "m", "of Ceres from the barycentre of the Solar System, ICRF axes. Its distance from the Sun is from 2.4 to 3.1 au; the compared run, to the last range compared with measurement, went through 2.54 to 2.99 au, and a distance outside that, inside the band, is computed".into()),
        vec3_stated("ceres_velocity_m_s", "m/s", "of Ceres. The semi-major axis of its orbit about the Sun is from 2.4 to 3.1 au".into()),
        vec3_stated("pallas_position_m", "m", "of Pallas from the barycentre of the Solar System, ICRF axes. Its distance from the Sun is from 2.0 to 3.6 au; the compared run, to the last range compared with measurement, went through 2.13 to 3.42 au, and a distance outside that, inside the band, is computed".into()),
        vec3_stated("pallas_velocity_m_s", "m/s", "of Pallas. The semi-major axis of its orbit about the Sun is from 2.0 to 3.6 au".into()),
        vec3_stated("vesta_position_m", "m", "of Vesta from the barycentre of the Solar System, ICRF axes. Its distance from the Sun is from 2.0 to 2.7 au; the compared run, to the last range compared with measurement, went through 2.14 to 2.58 au, and a distance outside that, inside the band, is computed".into()),
        vec3_stated("vesta_velocity_m_s", "m/s", "of Vesta. The semi-major axis of its orbit about the Sun is from 2.0 to 2.7 au".into()),
        scalar("energy_anchor", Role::Phase, "m^5/s^4", None, false, None, false, "E_anchor of the run's ledger: Newton's energy of the thirteen bodies times G, 1/2 sum mu_i v_i.v_i - sum_{i<j} mu_i mu_j / rho_ij, at the state the ledger started from. A returned state hands back its own; a new start gives its own E. Required".into()),
        scalar("booked_work", Role::Phase, "m^5/s^4", None, false, None, false, "W of the run's ledger: the work of relativity and of the Earth's oblateness, times G, since the ledger started. A returned state hands back its own; a new start gives 0. Required. E - energy_anchor - booked_work at the start must be within 1e-12 times 1.32e25, or the start is refused by the ledger's names".into()),
        vec3_stated("angular_momentum_anchor", "m^5/s^3", "L_anchor of the run's ledger: the angular momentum of the thirteen bodies times G, sum mu_i r_i x v_i, at the state the ledger started from. A returned state hands back its own; a new start gives its own L. Required".into()),
        vec3_stated("booked_torque", "m^5/s^3", "T of the run's ledger: the torque of relativity and of the Earth's oblateness, times G, since the ledger started. A returned state hands back its own; a new start gives [0, 0, 0]. Required. |L - angular_momentum_anchor - booked_torque| at the start must be within 1e-12 times 2.09e33, or the start is refused as AngularMomentumBreach".into()),
    ];
    finish(
        "aerospace",
        "solar_system",
        "The Solar System as thirteen point masses in free motion: the Sun, Mercury, Venus, the Earth, the Moon, the barycentres of the Mars, Jupiter, Saturn, Uranus and Neptune systems, and Ceres, Pallas and Vesta, under Newton's and the relativistic forces between every pair and the Earth's oblateness on the Moon, with the constants of JPL's DE440 and nothing fitted. The caller supplies every position and velocity. The comparison started from JPL's own state of 1999 and was made with range measurements that state was fitted to: over 16.2 years one Mars file, Mars Odyssey up to 2006, and MESSENGER at Mercury met their lines, two Mars files missed by at most 10 percent in mid-2007, and the Jupiter and Saturn files were not compared.",
        fields,
        floor,
        "the ceiling divided by 4096: Neptune moves 4.8 km. Under it a step is mostly rounding",
        gate,
        "the largest step the card stands behind, a locked constant. Measured with this card from the 1999 state over 2830 days against the same run at 1800 s, the largest distance at the end of a day, the step error at this step is 359 m for the Moon, 90 m for Mercury, 4.3 m for the Earth, 0.30 m for Venus, 0.21 m for Mars and under 0.2 m for each other body, the three asteroids among them",
        None,
        "No load. A run segment carries an empty load object. Each body's distance band, its orbit band and the speed of the centre of mass are applied to the incoming state and to every landing. Compared with Deep Space Network ranges for up to 16.2 years from JPL's 1999 state, in one run of 1800 s steps: Mars Odyssey to 2006 within 695 m at most (target 700 m) and MESSENGER at Mercury within 2.08 km at most (target 2.5 to 4.4 km); Odyssey after 2006 and Mars Reconnaissance Orbiter outside their targets by at most 8 to 10 percent, in mid-2007; Juno and Cassini are outside the comparison. In that run, to the last range compared with measurement, the bodies went through these distances from their primaries, each end rounded outward to 0.01 au: Mercury 0.30 to 0.47 au, Venus 0.71 to 0.73, the Earth 0.98 to 1.02, the Moon 356 000 to 407 000 km, Mars 1.38 to 1.67, Jupiter 4.94 to 5.46, Saturn 9.03 to 9.97, Uranus 19.88 to 20.10, Neptune 29.96 to 30.14, Ceres 2.54 to 2.99, Pallas 2.13 to 3.42, Vesta 2.14 to 2.58 au. The run swept the whole orbit of each inner body, of Jupiter and of the asteroids, and part of the orbits of Saturn, Uranus and Neptune. A state outside those distances, inside the bands, is computed on the same forces and both ledgers. From 16.2 to 21 years the run is compared with DE441, a calculation, and stays within 1.9 km of its Earth-to-Mars distance. No date is held; carried on past the last measured range, the compared run goes on, past day 5,934.6 from the 1999 state, where Saturn goes farther from the Sun than 9.97 au. Two bodies at exactly one point are refused as BodiesCoincide; any other separation is computed. The compared run's closest pair other than the Earth and the Moon was Mercury and Venus at 0.2631 au, and its closest two asteroids Ceres and Vesta at 0.3885 au; the card has no figures and no tides of the bodies other than the Earth's oblateness on the Moon, so two bodies closer than their radii are still point masses to it, and a close encounter needs a smaller step, which the ledger asks for by refusing one taken too large. The centre of mass may be at any distance from the origin; the compared run's stayed 98 409 to 114 929 m from it, and a state moved bodily is the same motion with other rounding, 98.5 m on the Moon after one year when moved by 6e13 m. The ledger of the incoming state, energy_anchor, booked_work, angular_momentum_anchor and booked_torque, is closed at the start as at a landing and refused by the ledger's names when it is out of balance. The axes are held to the one thing the card can tell them by: a state whose Earth-Moon barycentre does not orbit the Sun in a plane 21 to 27 degrees from the z axis is refused as AxesOutsideCard, which names a state in ecliptic axes; a turn about the z axis is not seen.".into(),
        &[
            "NonFiniteInput",
            "NonFiniteState",
            "NonPositiveDt",
            "InvalidSystem",
            "DistanceOutsideCard",
            "OrbitOutsideCard",
            "BodiesCoincide",
            "FrameOutsideCard",
            "AxesOutsideCard",
            "SpontaneousGeneration",
            "FirstLawBreach",
            "AngularMomentumBreach",
        ],
        vec![],
        slots(&[
            ("t", "t_s, 64-bit"),
            (
                "pos[0..3]",
                "the vector from the Earth to the Mars barycentre, x, y, z, each rounded to the nearest 32-bit number, metres",
            ),
            (
                "vel[0..3]",
                "x, y, z of the same vector less the number in pos, metres",
            ),
            ("force_torque", "earth_mars_distance_m"),
            ("residual", "residual_conservation_error"),
            ("flags", "0"),
            ("proof", "Not written by the step."),
        ]),
    )
}

/// What a caller must know of a start state for this card, in the words of
/// the card's header.
const SOLAR_SYSTEM_START_STATE: &str = "The card holds no ephemeris and no date: the forces depend on where the bodies are and how they move, and on nothing else. Positions and velocities are barycentric, on the axes of the ICRF, in metres and metres per second; time is TDB. The worked example is the state of 1999-01-01 00:00 TDB (JD 2451179.5) from JPL Horizons, the state the card was compared from. Horizons names its source as DE441 for the planets and its own orbit solutions for the three asteroids; DE441 is a calculation whose start values JPL fitted to observations, not a measurement. `t_s` counts from the start of the run. Every start carries a ledger: `energy_anchor`, `booked_work`, `angular_momentum_anchor` and `booked_torque`, and a start without them is refused. A final state handed back as a new start, with its twenty-six vectors and its ledger, continues the same motion and the same ledger bit for bit, and the new run's `t_s` begins again at zero; no limit depends on `t_s`. A new start gives its own energy and angular momentum as the anchors and zero work and torque, as the worked example does; the card cannot tell that from a returned state given a new ledger, and such a start begins a new ledger. What a station measures is not in the state: the light time between a station on the turning Earth and a planet is worked out from these positions outside the engine.";

/// How a reader rebuilds the Earth-to-Mars vector from one recorded frame,
/// in the words of the card's header.
const SOLAR_SYSTEM_FRAME_RULE: &str = "A reader adds `pos[i]` and `vel[i]` as 64-bit numbers and has each coordinate of the vector from the Earth to the Mars barycentre to about a millimetre. Every position and velocity is in the state.";

/// The state the card was scored from: 1999-01-01 00:00 TDB, JPL Horizons.
pub(crate) fn solar_system_example_state() -> solar_system::SolarSystemState {
    let specimen = solar_system::SolarSystem::de440();
    solar_system::SolarSystemState::try_state(
        &specimen,
        solar_system::START_1999_POSITION_M,
        solar_system::START_1999_VELOCITY_M_S,
    )
    .expect("the 1999 state is on the card")
}

fn finish(
    domain: &'static str,
    card: &'static str,
    specimen: &'static str,
    fields: Vec<FieldSpec>,
    floor_s: f64,
    floor_from: &'static str,
    ceiling_s: f64,
    ceiling_from: &'static str,
    nyquist_s: Option<f64>,
    load_rule: String,
    rejects: &'static [&'static str],
    advisory: Vec<Advisory>,
    slots: Vec<FrameSlot>,
) -> Card {
    let phase_names = fields
        .iter()
        .filter(|field| field.role == Role::Phase)
        .map(|field| field.name)
        .collect::<Vec<_>>();
    let load_names = fields
        .iter()
        .filter(|field| field.role == Role::Load)
        .map(|field| field.name)
        .collect::<Vec<_>>();
    let mut parameter_names = phase_names.clone();
    parameter_names.extend(load_names.iter().copied());
    parameter_names.push("dt_s");
    let nyquist_text = match nyquist_s {
        Some(nyquist_s) => format!(" pi/omega is {} s.", token(nyquist_s)),
        None => String::new(),
    };
    let nyquist_rule = format!(
        "dt_s is in [{}, {}] s. The floor is this card's own floor_dt_s: {}. The ceiling is this card's own gate_dt_s: {}.{} {}",
        token(floor_s),
        token(ceiling_s),
        floor_from,
        ceiling_from,
        nyquist_text,
        load_rule
    );
    Card {
        domain,
        card,
        specimen,
        fields,
        parameter_names,
        phase_names,
        load_names,
        floor_s,
        floor_from,
        ceiling_s,
        ceiling_from,
        nyquist_s,
        nyquist_rule,
        rejects,
        advisory,
        slots: with_quantities(card, slots),
    }
}

/// The load rule of the six cards that limit a load by its impulse in one
/// step, stated as the engine applies it: the product of the size of the
/// load and `dt_s` is one multiplication in 64-bit arithmetic, rounded once to
/// the nearest 64-bit number with no fused multiply-add, and that product is
/// compared with the constant. The quotient `C / dt_s` is not the test.
fn impulse_rule(key: &str, constant: &str, impulse: f64) -> String {
    format!(
        "|{key}| * dt_s <= C, with C = {constant} = {}. The product |{key}| * dt_s is one multiplication in 64-bit arithmetic, rounded once to the nearest 64-bit number (no fused multiply-add), and that product is compared with C: a load whose product is at most C is accepted, any other is refused",
        token(impulse)
    )
}

/// The card's own step ceiling, and `π/ω` on its frequency or pole.
fn clock(omega: f64, gate: Option<f64>, nyquist: Option<f64>) -> (f64, Option<f64>) {
    let gate = gate.expect("specimen gate");
    let nyquist = nyquist.unwrap_or_else(|| super::nyquist_limit_s(omega));
    (gate, Some(nyquist))
}

fn scalar(
    name: &'static str,
    role: Role,
    unit: &'static str,
    minimum: Option<f64>,
    minimum_exclusive: bool,
    maximum: Option<f64>,
    maximum_exclusive: bool,
    bound: String,
) -> FieldSpec {
    FieldSpec {
        name,
        role,
        unit,
        components: 1,
        minimum,
        minimum_exclusive,
        maximum,
        maximum_exclusive,
        bound,
    }
}

/// A single number held to `[-largest, largest]`, both ends inclusive.
fn within(
    name: &'static str,
    role: Role,
    unit: &'static str,
    largest: f64,
    bound: String,
) -> FieldSpec {
    scalar(name, role, unit, Some(-largest), false, Some(largest), false, bound)
}

fn vec3_role(name: &str) -> Role {
    match name {
        "force_n" | "torque_nm" => Role::Load,
        _ => Role::Phase,
    }
}

/// A 3-vector with no numeric bound, and the words that say so or say what
/// holds it.
fn vec3_stated(name: &'static str, unit: &'static str, bound: String) -> FieldSpec {
    FieldSpec {
        name,
        role: vec3_role(name),
        unit,
        components: 3,
        minimum: None,
        minimum_exclusive: false,
        maximum: None,
        maximum_exclusive: false,
        bound,
    }
}

/// A 3-vector whose three components are each held to `[-largest, largest]`,
/// both ends inclusive. The published minimum and maximum are those of one
/// component.
fn vec3_within(name: &'static str, unit: &'static str, largest: f64, bound: String) -> FieldSpec {
    FieldSpec {
        name,
        role: vec3_role(name),
        unit,
        components: 3,
        minimum: Some(-largest),
        minimum_exclusive: false,
        maximum: Some(largest),
        maximum_exclusive: false,
        bound,
    }
}

/// The largest position of the node along a ligament at which the titanium
/// cell is not past yield at the reference temperature, found on the card's
/// own stress: the state the card anchors at that position reports a stress
/// at or under the yield strength there, and above it at the next 64-bit
/// number. The smallest over the three ligaments.
pub fn ti64_yield_position_m(cell: &ti64_lattice::Ti64LatticeCell) -> f64 {
    let mut smallest = f64::INFINITY;
    for axis in 0..3 {
        let holds = |x: f64| -> bool {
            let mut position_m = [0.0; 3];
            position_m[axis] = x;
            ti64_lattice::Ti64LatticeState::try_phase(cell, position_m, [0.0; 3])
                .map(|state| state.von_mises_mpa <= ti64_lattice::YIELD_STRENGTH_MPA)
                .unwrap_or(false)
        };
        let mut value = ti64_lattice::uniaxial_yield_strain() * cell.length_m[axis];
        while !holds(value) {
            value = f64::from_bits(value.to_bits() - 1);
        }
        loop {
            let above = f64::from_bits(value.to_bits() + 1);
            if holds(above) {
                value = above;
            } else {
                break;
            }
        }
        if value < smallest {
            smallest = value;
        }
    }
    smallest
}

/// The span of node positions along a ligament at which that ligament is not
/// past yield at some temperature of the card, found on the card's own yield
/// test with the other two ligaments unstressed: from
/// the last position inside at 250 K, below the rest position, to the last
/// one inside at the beta transus, above it. The tightest over the three
/// ligaments.
pub fn ti64_position_window_m(cell: &ti64_lattice::Ti64LatticeCell) -> (f64, f64) {
    let mut lowest = f64::NEG_INFINITY;
    let mut highest = f64::INFINITY;
    for axis in 0..3 {
        // The other two ligaments stand at their own thermal length, unstressed.
        let holds = |x: f64, temperature_k: f64| -> bool {
            let mut position_m = [0.0; 3];
            for other in 0..3 {
                position_m[other] = ti64_lattice::thermal_extension_m(cell.length_m[other], temperature_k);
            }
            position_m[axis] = x;
            !ti64_lattice::past_yield(cell, position_m, temperature_k)
        };
        let yield_m = ti64_lattice::uniaxial_yield_strain() * cell.length_m[axis];
        for (temperature_k, outward) in [
            (ti64_lattice::BETA_TRANSUS_K, 1.0),
            (ti64_lattice::CARD_TEMPERATURE_MIN_K, -1.0),
        ] {
            let mut value = ti64_lattice::thermal_extension_m(cell.length_m[axis], temperature_k)
                + outward * yield_m;
            let inward = |x: f64| if outward > 0.0 { x.next_down() } else { x.next_up() };
            let further = |x: f64| if outward > 0.0 { x.next_up() } else { x.next_down() };
            while !holds(value, temperature_k) {
                value = inward(value);
            }
            while holds(further(value), temperature_k) {
                value = further(value);
            }
            if outward > 0.0 {
                highest = highest.min(value);
            } else {
                lowest = lowest.max(value);
            }
        }
    }
    (lowest, highest)
}

/// The largest speed along a ligament that a run under forces and
/// temperatures inside their ranges lands from rest or from a turning point
/// before that ligament passes yield: `2 ω sqrt(x_y (x_y + g))`, with `x_y`
/// the yield position at the reference temperature and `g` the thermal growth
/// over the card's temperatures, the width of [`ti64_position_window_m`] less
/// `2 x_y`. The node gains `½ v²` at most `ω² (x_y − s)` per metre it moves
/// with its stretch `s` held in `[−x_y, x_y]`; heating as it moves keeps `s` at
/// `−x_y` over `g`, and the sweep to `x_y` adds `2 ω² x_y²`. By the same sum a
/// faster node cannot come to rest before yield.
pub fn ti64_yield_speed_m_s(cell: &ti64_lattice::Ti64LatticeCell) -> f64 {
    let yield_m = ti64_yield_position_m(cell);
    let (lowest, highest) = ti64_position_window_m(cell);
    2.0 * cell.max_natural_frequency_rad_s() * (yield_m * (highest - lowest - yield_m)).sqrt()
}

/// The force that holds a ligament at yield at rest: the yield strength times
/// the smallest section of the three ligaments.
pub fn ti64_yield_force_n(cell: &ti64_lattice::Ti64LatticeCell) -> f64 {
    let mut area_m2 = cell.area_m2[0];
    for axis in 1..3 {
        if cell.area_m2[axis] < area_m2 {
            area_m2 = cell.area_m2[axis];
        }
    }
    ti64_lattice::YIELD_STRENGTH_MPA * 1.0e6 * area_m2
}

/// The feeder current whose energy in the inductance equals the heat that
/// takes the fuse link from the reference temperature to its melting point.
pub fn grid_melt_current_a(node: &grid_node::GridNode) -> f64 {
    node.melt_current_a()
}

/// The node voltage whose energy in the shunt capacitance equals the heat
/// that takes the fuse link from the reference temperature to its melting
/// point.
pub fn grid_melt_voltage_v(node: &grid_node::GridNode) -> f64 {
    node.melt_voltage_v()
}

/// What the drive-mode resonator says of its external rate and its sense
/// speed, the two loads that set a reported scale and nothing else.
const TESSERACT_SENSE_REACH: &str = "no bound. The external rate and the sense speed set the reported Coriolis scale, twice the proof mass times the sense speed times the rate, and do not enter the drive motion or the ledger. The scale is written to a 32-bit frame slot: when it is too large for that slot (above about 3.4e38 N) the step is refused as a non-finite frame slot and that state is not sealed";

fn slots(items: &'static [(&'static str, &'static str)]) -> Vec<FrameSlot> {
    items
        .iter()
        .map(|(slot, holds)| FrameSlot {
            slot,
            holds,
            state: String::new(),
            unit: "",
        })
        .collect()
}

/// The unit of a state quantity, by its key. Every quantity a card packs into
/// a numeric slot is here.
fn unit_of(state: &str) -> &'static str {
    unit_of_known(state).unwrap_or_else(|| panic!("{}", missing_row("unit_of", state)))
}

/// What a card needs and does not have here, in one sentence that names it
/// and says where it goes: a describe_card that meets it is answered as an
/// internal fault, and `every_card_has_the_rows_its_flags_and_its_frame_need`
/// names it in a test.
pub(crate) fn missing_row(table: &str, key: &str) -> String {
    format!(
        "{key} has no row in {table} in protocol_mcp_gateway/src/descriptor.rs: every flag a card declares needs the state boolean that sets it (flag_state), every quantity its frame packs needs its unit (unit_of), and every card the quantity of its force_torque slot (force_torque_state). python3 protocol_mcp_gateway/kit/new_model.py rows <card> adds the rows it can derive and names the rest."
    )
}

pub(crate) fn unit_of_known(state: &str) -> Option<&'static str> {
    Some(match state {
        "t_s" | "peak_time_s" | "range_margin_s" | "lowest_time_s" => "s",
        "frequency_hz" | "lowest_frequency_hz" => "Hz",
        "frequency_rate_hz_per_s" => "Hz/s",
        "response_mw" | "relief_mw" | "net_power_mw" => "MW",
        "path_loss_db" | "rain_attenuation_db" | "antenna_water_db" => "dB",
        "received_level_dbm" => "dBm",
        "rain_rate_margin_mm_h" => "mm/h",
        "terminal_v" | "open_circuit_v" | "cutoff_margin_v" | "voltage_v" | "voltage_spike_v"
        | "branch_v" | "lower_limit_margin_v" | "reaction_overpotential_v" | "hysteresis_v"
        | "electrolyte_v" | "electrolyte_drop_v" => "V",
        "charge_removed_ah" => "Ah",
        "case_temperature_c" | "core_temperature_c" => "degC",
        "soc" | "converter_fading_fraction" | "mach" | "residual_conservation_error"
        | "surface_state_of_charge" | "radius_over_final" | "slip_ratio" | "charge_level" => "1",
        "current_a" | "current_mean_a" => "A",
        "chemical_energy_j" | "joule_j" | "internal_energy_j" | "boundary_work_j" | "kinetic_j"
        | "potential_j" | "mechanical_energy_j" | "elastic_energy_j" | "electrical_energy_j"
        | "heat_rejected_j" | "fuel_heat_j" | "thermal_energy_j" | "received_energy_j"
        | "wheel_work_j" | "drawbar_work_j" | "slip_heat_j" | "fast_ion_energy_j"
        | "fusion_energy_j" | "plate_work_j" | "stored_energy_j" | "friction_heat_j" => "J",
        "twist_deg" => "deg",
        "stuck_fraction" => "1",
        "slip_torque_limit_n_m" | "torque_n_m" => "N m",
        "temperature_k" | "ambient_k" | "ceiling_margin_k" | "melt_margin_k"
        | "fuel_temperature_rise_k" | "water_temperature_rise_k" | "fuel_temperature_k"
        | "peak_fuel_temperature_k" | "hot_spot_plate_rise_k" | "water_rise_k" => "K",
        "heat_power_w" | "heat_to_ambient_w" | "joule_power_w" | "electrical_power_w"
        | "thermal_power_w" | "received_power_w" | "fusion_power_w" | "loss_power_w" => "W",
        "temperature_kev" => "keV",
        "beta_normalised" => "% m T/MA",
        "displacement_m" | "stroke_margin_m" | "envelope_m" | "compression_m" | "elongation_m"
        | "altitude_m" | "ground_distance_m" | "position_m" | "radius_m" | "deflection_m"
        | "radius_to_go_m" | "earth_to_mars_m" | "earth_mars_distance_m" | "distance_m"
        | "rim_travel_m" => "m",
        "velocity_m_s" | "air_speed_m_s" | "radial_speed_m_s" | "wall_speed_m_s" => "m/s",
        "accel_m_s2" => "m/s^2",
        "scale_factor_n" | "tension_n" | "ultimate_margin_n" | "longitudinal_force_n" => "N",
        "pressure_pa" | "vapor_margin_pa" | "dynamic_pressure_pa" => "Pa",
        "fluid_volume_m3" => "m^3",
        "mass_kg" | "boundary_mass_kg" => "kg",
        "heat_flux_w_m2" => "W/m^2",
        "stagnation_pressure_pa" => "Pa",
        "recovery_enthalpy_j_kg" => "J/kg",
        "heat_transfer_coefficient_kg_m2_s" => "kg/m²/s",
        "kinetic_j_per_kg" | "potential_j_per_kg" | "heat_j_per_kg" | "given_up_j_per_kg"
        | "energy_defect_j_per_kg" => "J/kg",
        "creep_extension_percent" => "%",
        "creep_rate_percent_per_h" => "%/h",
        "stress_mpa" => "MPa",
        "work_by_load_mj_per_m3" | "heat_dissipated_mj_per_m3" | "work_defect_mj_per_m3" => {
            "MJ/m^3"
        }
        "predicted_x_mas" | "predicted_y_mas" | "estimate_x_mas" | "estimate_y_mas" => "mas",
        "innovation_variance_x_mas2" | "innovation_variance_y_mas2" => "mas^2",
        "nis" => "1",
        "load_g" => "g (Earth)",
        "power_mw" | "peak_power_mw" => "MW",
        "energy_mws" | "peak_energy_mws" | "energy_released_mws" => "MW*s",
        "power_rate_mw_per_s" => "MW/s",
        "reactivity_net_dollar" | "net_reactivity_dollar" | "void_feedback_dollar" => "dollar",
        "delta_rad" | "pll_err_rad" | "synchronism_margin_rad" => "rad",
        "delta_omega_rad_s" => "rad/s",
        "p_elec_pu" | "p_mech_pu" | "transient_energy_pu" => "pu",
        "gas_pressure_pa" => "Pa",
        "gas_temperature_k" => "K",
        "gas_volume_m3" => "m^3",
        "work_on_gas_j" | "heat_to_wall_j" => "J",
        "gas_mass_kg" => "kg",
        "gas_force_n" => "N",
        "chemical_heat_w" => "W",
        "heat_released_j" | "heat_lost_j" => "J",
        "anode_lithium_fraction" | "cathode_unreacted_fraction" | "electrolyte_fraction" => "1",
        "object_height_m" | "hand_height_m" | "pad_travel_m" => "m",
        "object_velocity_m_s" | "hand_velocity_m_s" => "m/s",
        "tangential_force_n" => "N",
        "h_eci_nms" | "h_magnitude_nms" => "N m s",
        "torque_eci_nm" => "N m",
        "wall_current_a" | "p6_current_a" => "A",
        "wall_power_w" => "W",
        "gap_to_wall_m" => "m",
        "wall_force_n" => "N",
        "rod_compensation_percent_dk_k" => "% dk/k",
        "rod_reactivity_percent_dk_k" => "% dk/k",
        "circulation_loss_percent_dk_k" => "% dk/k",
        "fuel_flow_fraction" => "1",
        "pump_speed_fraction" => "1",
        "loss_at_start_percent_dk_k" => "% dk/k",
        "phase_s" => "s",
        "fuel_flow_rate_per_s" => "1/s",
        "fuel_average_k" | "fuel_melt_margin_k" => "K",
        "hot_clad_k" => "K",
        "water_k" => "K",
        "hot_spot_plate_temperature_k" => "K",
        "plant_response_mw" => "MW",
        "service_response_mw" => "MW",
        "ion_temperature_kev" => "keV",
        _ => return None,
    })
}

/// The state quantity that a card's `force_torque` slot holds.
fn force_torque_state(card: &str) -> &'static str {
    force_torque_state_known(card).unwrap_or_else(|| panic!("{}", missing_row("force_torque_state", card)))
}

fn force_torque_state_known(card: &str) -> Option<&'static str> {
    Some(match card {
        "ti64_lattice" => "tension_n",
        "grid_node" => "melt_margin_k",
        "swing" => "synchronism_margin_rad",
        "battery_sag" | "cell_18650pf" => "cutoff_margin_v",
        "thermal" => "ceiling_margin_k",
        "resonance" => "stroke_margin_m",
        "tesseract" => "scale_factor_n",
        "hydraulic_fluid" => "vapor_margin_pa",
        "cable_stretch" => "ultimate_margin_n",
        "generator_mmrtg_f1" => "range_margin_s",
        "capsule_stardust" | "capsule_insight" => "load_g",
        "satellite_lageos1" => "radius_m",
        // The slot holds the amplitude ceiling less the envelope; its row says so.
        "cantilever_eth" => "envelope_m",
        "creep_cmsx6" => "stress_mpa",
        "crater_ryugu_sci" => "radius_to_go_m",
        "grid_gb" => "net_power_mw",
        "tokamak_jet_dte1" => "loss_power_w",
        "runaway_18650_lco" => "heat_lost_j",
        "reactor_msre_pump" => "fuel_flow_rate_per_s",
        "reactor_spert3" => "fuel_melt_margin_k",
        "cell_2170_model3" => "current_mean_a",
        "reactor_spert3_ccore" => "power_rate_mw_per_s",
        "reactor_triga_jsi" => "power_rate_mw_per_s",
        "generator_mmrtg_seebeck" => "range_margin_s",
        "momentum_jwst_dihedral" => "h_magnitude_nms",
        "grasp_lift_glass_4tu" => "tangential_force_n",
        "momentum_jwst_srp" => "h_magnitude_nms",
        "link_gothenburg_10010" => "rain_rate_margin_mm_h",
        "solar_system" => "earth_mars_distance_m",
        "tire_eve_r19_ice" => "longitudinal_force_n",
        "accumulator_sbo450" => "gas_force_n",
        "los_jwst_fine_guide" => "nis",
        "tokamak_vde_mast" => "wall_force_n",
        "fingerpad_p02_glass" => "torque_n_m",
        _ => return None,
    })
}

/// Fill the state key and the unit of each slot from the card's packing.
/// A row that names a quantity alone is that quantity, rounded to 32 bits.
fn with_quantities(card: &str, rows: Vec<FrameSlot>) -> Vec<FrameSlot> {
    rows.into_iter()
        .map(|mut row| {
            let state = match row.slot {
                "t" => "t_s".to_string(),
                "residual" => "residual_conservation_error".to_string(),
                "force_torque" => force_torque_state(card).to_string(),
                "flags" | "proof" => String::new(),
                _ if row.slot.starts_with("pos[") || row.slot.starts_with("vel[") => {
                    if row.holds.contains(|c: char| !(c.is_ascii_alphanumeric() || c == '_' || c == '[' || c == ']')) {
                        // The satellite's two rows: the position, in pieces.
                        // The solar system's: the Earth-to-Mars vector.
                        if card == "solar_system" {
                            "earth_to_mars_m".to_string()
                        } else {
                            "position_m".to_string()
                        }
                    } else {
                        row.holds.to_string()
                    }
                }
                _ => unreachable!("a slot that is not in the frame"),
            };
            let base = state.split('[').next().unwrap_or("");
            row.unit = if base.is_empty() { "" } else { unit_of(base) };
            row.state = state;
            row
        })
        .collect()
}

pub(crate) fn token(value: f64) -> String {
    super::exact_f64_token(value)
}

fn exact_value(value: f64) -> Value {
    serde_json::from_str(&token(value)).expect("exact token is json")
}

fn opt_number(value: Option<f64>) -> Value {
    value.map(exact_value).unwrap_or(Value::Null)
}

/// The largest displacement at rest and the largest velocity at zero
/// displacement the mount can hold: where its mechanical energy meets the
/// card's limit. Found on the 64-bit numbers by the card's own energy.
fn resonance_largest_phase(oscillator: &resonance::DynamicOscillator) -> (f64, f64) {
    let limit_j = oscillator.energy_ceiling_j();
    let largest = |energy_j: &dyn Fn(f64) -> f64, start: f64| -> f64 {
        let mut value = start;
        while !(energy_j(value) <= limit_j) {
            value = f64::from_bits(value.to_bits() - 1);
        }
        loop {
            let above = f64::from_bits(value.to_bits() + 1);
            if energy_j(above) <= limit_j {
                value = above;
            } else {
                return value;
            }
        }
    };
    let displacement_m = largest(
        &|x| oscillator.mechanical_energy_j(x, 0.0),
        oscillator.amplitude_ceiling_m,
    );
    let velocity_m_s = largest(
        &|v| oscillator.mechanical_energy_j(0.0, v),
        oscillator.omega_rad_s * oscillator.amplitude_ceiling_m,
    );
    (displacement_m, velocity_m_s)
}

/// The five parts of a validation record, in this order.
pub(crate) const VALIDATION_PARTS: [&str; 5] = ["object", "data", "pass_line", "result", "limits"];

/// The result of the creep card's validation record: the comparison of
/// 2026-10-09, as the sealed result of the research record gives it.
pub(crate) const CREEP_CMSX6_RESULT: &str = "Over the 33 times to a level the six tests reached: RMS 0.093 in log time (target 0.15), worst 0.240 (target 0.30), mean +0.019 (target 0.10). On the three tests at 200 MPa, a stress no fitted test has: RMS 0.085, worst 0.170, mean +0.028 over 18 times. On the three test pieces at fitted stresses: RMS 0.102, worst 0.240, mean +0.008 over 15 times. The worst single time is 0.5 percent at 170 MPa, where the card is 27 percent late. In hours at 200 MPa the card gives 2.96, 10.6, 48.9, 81.1, 113.4 and 156.1 to the six levels, and the three test pieces 3.1 to 3.2, 11.2 to 12.2, 41.3 to 43.9, 74.6 to 79.3, 106.3 to 114.0 and 138.1 to 151.8. Started at 0.1 percent, inside the compared range: over 32 times since load (the 33 less 0.5 percent at 140 MPa, past the range compared there), RMS 0.094, worst 0.240, mean +0.021, each the same as from zero to 2 parts in 100,000 of the logarithm. The time from 0.1 percent to each higher level, over 26 times: RMS 0.108, worst 0.249, mean +0.029.";

/// The result of the crater's validation record.
const CRATER_RESULT: &str = "On Ryugu: the card stops at 8.571 m against the measured apparent radius of 7.25 m, 1.32 m or 18.2 percent above (allowance 0.97 m: 0.81 m from the fit, 0.40 m from the measurement, 0.34 m from the local gravity). 7.25 m is the apparent radius, which is what the law predicts: the apparent diameter is 14.5 +/- 0.8 m and the rim-to-rim diameter 17.6 +/- 0.7 m. The same team's submitted manuscript gives 6.5 +/- 0.4 m by another reduction. The published team's own range for sand scaling, 6.9 to 7.7 m by the dry-sand line of Housen and Holsapple, \"agrees well with the apparent radius\". In the laboratory, fitted on the nine craters made at 1 g: each of the four centrifuge craters inside its 95 percent prediction interval, 6.9 to 9.5 percent low, mean 7.9 percent (target 10). Those intervals are +/- 31 to 38 percent wide. The mean, 7.87 percent, follows from Schmidt's published line, which is fitted through the centrifuge craters, and it moves with the fit: with any one of four of the nine 1 g shots left out it is 10.4 to 11.8 percent (target 10). The four errors are all of one sign, a bias of the nine-shot line. The stop, against the closed form of the thirteen-shot law, 8.571304329477925 m (target 1 part in 10^9): the card types the law's two constants as the fit gives them and stops at 8.571304329477922 m, 4 parts in 10^16 from the closed form. That agreement is by construction: on the stopping step the card sets the radius to a typed constant, which states that the constant is typed as computed. A card with the two constants typed to six figures stopped at 8.571304624905444 m, 3.45 parts in 10^8 from the closed form; on Ryugu the gap is the same either way, 1.321 m. Without a target: the two lines Schmidt published for Ottawa sand and for number 24 quartz sand give 8.60 m and 8.89 m; the fitted exponent, 0.1661, is inside the 0.165 +/- 0.005 that Gault and Wedekind measured between 0.073 and 1 g; the card stops 400 s after the impact, where the crater was seen to grow for about 200 s, the team's submitted manuscript says growth would stop by about 300 s, and the whole event lasted about 500 s; the constant that would put the law on Ryugu's crater is 0.737 against the 0.871 fitted in sand. The six-figure run came from binary 425ef2b93d69be6f05da11eaf6ad4c4debecc37d548672950465d68ef2bf3b04; two later builds, 05efbd06… and 043d5765…, return its answer byte for byte. With the constants as fitted, the stop, the states and the proofs differ from that run in the eighth figure and beyond. A second comparison program, on the same machine, gives the same numbers.";

/// What a card that has not been compared with a measurement says.
pub(crate) const NOT_COMPARED: &str = "not compared with a measurement";

/// The pass line of the InSight capsule's validation record.
const CAPSULE_INSIGHT_PASS_LINE: &str = "Two sets of targets. NASA's own printed spread for its own forecast: peak deceleration within 0.5 g, each time within 6.946 percent of elapsed time (7.68 s on the peak, 10.42 s at 150 s, 15.1 s at the parachute), radius at the parachute within 3.5 km, Mach number there within 0.2. The stated spread of the card's own inputs: twice the root sum of squares of the shift each input gives when moved by one standard deviation, for the air density, the drag coefficient and the lift. Record SHA-256 ddafbbe8299da589e95a674bb31150f869d0f85d21415036bdbedfc4d7fa0a54, and 9bb5b953e4ae325eb954fb3979b3942a08dc19bd5ff80ae2f6bbeda37b137146 with the card's structure and the target on the Mach number added.";

/// The result of the InSight capsule's validation record: the comparison of
/// 2026-10-09 with the rows held back, as scored from the engine's frames.
const CAPSULE_INSIGHT_RESULT: &str = "On the 59 s not in the fit, two runs, each against both sets of targets. With the lift over drag of -0.031 from the first 150 s: the flight's speed at parachute deployment reached 1.31 s late and its radius 0.58 s early (target 15.1 s); at the flight's deployment time the radius 114 m low (target 3.5 km) and the Mach number 1.501 against 1.522 (target 0.2); inside all six stated-spread targets. Over the 58 whole seconds the speed within 7.2 m/s RMS, the radius within 245 m and the deceleration within 0.031 g. With no lift: 17.1 s and 22.8 s late (target 15.1 s), 4.24 km high (target 3.5 km) and Mach 1.91 (target 0.2 of 1.522); inside all six stated-spread targets, which carry a lift the capsule could take either way and are wider than a forecast: 6.8 km on the radius at the parachute, 27 s and 37 s on the two times and 0.72 on the Mach number. On the first 150 s, in the fit, with the lift: the peak 7.95 g against 8.12 g (target 0.5 g) and 4.0 s early (target 7.68 s); against its stated spread the time of the peak is 4.02 s off (target 3.27). With no lift: the peak 7.60 g against 8.12 g (target 0.5 g), the 150 s radius reached 17.0 s late (target 10.42 s); against the stated spread the peak 0.517 g off (target 0.489) and its time 5.81 s (target 1.82).";

/// The limits of the InSight capsule's validation record.
const CAPSULE_INSIGHT_LIMITS: &str = "One entry of one capsule, down to parachute deployment. The run inside NASA's targets takes one number from the flight: its lift, a single number measured on the first 150 s and held through the run. With nothing from the flight but its starting state the run is outside them. The card reproduces this entry given its lift; it does not forecast an entry whose lift is not known. The 59 s outside the fit do not test the lift: the run is already right at 150 s, where the lift was measured, and carried on past deployment to 4 km it comes inside the four NASA targets with any lift from -0.08 to 0.08 over the 59 s; the flight's own lift over those 59 s is -0.016, half the number the run holds. They test the air below 18 km and the drag below Mach 6, to which nothing is fitted. The 114 m at the parachute is finer than the flight record's own one standard deviation there, 694 m, and is not the card's accuracy. The air and the lift were both chosen with the first 150 s read; the card's structure and the target on the Mach number were set with NASA's printed values at the peak and at the parachute read, Mach 1.522 among them. The card's air is the mean of soundings one to seven sols old: between 36 km and 65 km it is 8 to 13 percent denser than the density reconstructed from the flight, more than its own stated spread allows, and the peak of the load comes 4 s early for it; below 18 km it is within 7 percent. NASA's targets are its spread for its own forecast and are wide: 0.5 g is 6 percent of the peak. The capsule's sideways lift, wind, and the rise of drag in thin air are not on the card. The card ends a run at the flown parachute deployment, and every compared value of the run with the flight's lift is on a landed step, the lowest at 5793 m and Mach 1.470. The run with no lift ends at 232.1 s, 6163.7 m up, after it reaches the flight's speed at the parachute and before it reaches the flight's radius there: its 22.8 s on that radius comes from the run carried on to 4 km and Mach 1.20, past its last landed step. Of 20 further runs with other lifts, 13 have a compared value on a state below 5700 m or Mach 1.46, past the deployment. The same aeroshell on Phoenix flew with its lift pointing the other way and has not been run. A second comparison program, written from the record alone, gives the same values on the same machine; no one outside the company has repeated them. The runs outside the fit were made with a debug build of the gateway.";

/// The solar_system card's validation record.
const SOLAR_SYSTEM_DATA: &str = "Round-trip light times between stations of NASA's Deep Space Network and spacecraft at Mars, Mercury, Jupiter and Saturn, as reduced and published by JPL's Solar System Dynamics group (ssd.jpl.nasa.gov/planets/obs_data.html). The constants are those of DE440 and the start state is DE441's, through JPL Horizons. Nothing is fitted. The Mars Global Surveyor ranges of 1999 to 2006 chose the card's structure and set the targets. Outside both: Mars Odyssey and Mars Reconnaissance Orbiter (Mars), MESSENGER (Mercury), Juno (Jupiter) and Cassini (Saturn).";
const SOLAR_SYSTEM_PASS_LINE: &str = "For every range of the files outside the structure's choice, measured minus computed as a one-way distance at most B x max(1, T / 7.75 years)^2, T counted from the start: B is 0.70 km for Mars, 1.0 km for Mercury, 1.5 km for Jupiter and 0.5 km for Saturn. One run from the 1999 state in steps of 1800 s. Record SHA-256 2d79e65df9fb8ad036444896e776e98d18cb98fc7b582f861747f27d79fd9679.";
const SOLAR_SYSTEM_RESULT: &str = "In one run of 7,670 days from the 1999 state, none refused. Mars Odyssey up to 2006, over the span of the Mars Global Surveyor ranges (2,389 ranges): 270 m rms and 695 m at most (target 700 m). Mars Odyssey after 2006 (4,939 ranges, out to 15.0 years): 612 m rms and 1,312 m at most; 44 ranges are outside the target, all in mid-2007, by at most 8 percent. Mars Reconnaissance Orbiter (1,993 ranges, out to 15.0 years): 619 m rms and 1,314 m at most; 34 ranges outside, all in mid-2007, by at most 10 percent. MESSENGER at Mercury (813 ranges, 12.3 to 16.2 years out): 1.42 km rms and 2.08 km at most (target 2.5 to 4.4 km). Juno at Jupiter (4 ranges) and Cassini at Saturn (196) are outside the comparison: with DE441's own positions the reduction leaves 20 km and 10 km rms on those two files. The measured range residual equals the card's distance from JPL's DE441 ephemeris to about 5 m: this simpler model, carried forward from JPL's own 1999 state, stays within 0.69 km of JPL's Earth-to-Mars distance over 7.75 years and 1.9 km over 21, within 2.7 km for Mercury, 3.2 km for Jupiter and 2.1 km for Saturn. The reduction leaves out the station term of TDB - TT, up to 48 m one way and 22 m rms. With a reduction that has it: Odyssey up to 2006-10, 689 m at most (target 700 m); Odyssey after, 45 of 4,939 ranges outside, the worst 1.095 of the target; Mars Reconnaissance Orbiter, 33 of 1,993 outside, the worst 1.093; MESSENGER, 2,054 m at most, 0.640 of its target. Odyssey up to 2006 is inside by 5 to 11 m of a target that is 678.3 m rounded up to 700; with 678.3 m, 11 to 13 ranges are outside. The causes: the Mars error is the 340 asteroids the card leaves out, 730 of the 933 m in mid-2007, and no single asteroid does it; Mercury's is the Sun's oblateness, 2,591 of 2,661 m; the Earth's remaining 69 to 183 m is the Moon's error of 6 to 16 km, from the figure of the Moon, the tides and the Earth's figure beyond J2. Newton alone would be 351 km off by 2006-10. The engine's daily positions against the outside model (target 10 m): every body within 3 m but the Moon, 14.6 m apart after 21 years, the two growing apart smoothly from nothing as rounding does. It is rounding: two 64-bit runs of these equations that differ in the order of operations, or by one unit in the last place at the start, end 5 to 24 m apart on the Moon after 21 years, so no 64-bit run holds the Moon to 10 m. Against a 32-digit run the engine is 9.5 m off on the Moon and 0.92 m on the Earth. The truncation of the 1800 s step over 21 years is 78 m on the Moon, 17.9 m on Mercury and under 1 m on the Earth. A second comparison program, on the same machine, with an independent reduction and an independent integrator, gives the same results.";
const SOLAR_SYSTEM_CAVEATS: &str = "The start state is JPL's DE441 through Horizons. DE441 is a calculation whose start values JPL fitted to observations, these range measurements among them, so the start already carries information from the ranges outside the structure's choice; what the comparison tests is whether this card's far simpler dynamics carry that one state forward for up to 16.2 years to where the stations measured (the run went on to 21 years, compared with DE441 alone). Before the record was set, the first lines of those files were printed to learn their layout, two rows of data among them in four files. The Odyssey ranges up to 2006 are interpolation: the same planet over the span of the Mars Global Surveyor ranges. Odyssey and Mars Reconnaissance Orbiter after 2006 are extrapolation in time. Mercury, Jupiter and Saturn had no range in the structure's choice, and their targets rest on a comparison with DE441, a calculation. The light time is worked out from the card's positions outside the engine: station position from the IAU 2006/2000A Earth orientation and the IERS series, the Sun's relativistic delay, no solar plasma. With DE441's own positions that reduction leaves 18 m rms on the Mars Global Surveyor file, the floor of the comparison. That floor of 18 m is the reduction's omission of the station term of TDB - TT; a reduction that has the term has a floor of about 5 m, and 1.3 m at Mercury. Left out of the card: every other asteroid, Pluto, the Sun's oblateness, the figure of the Moon, every tide. The square in the target is a guess at how the error grows beyond the span of the structure's choice; the Mars ranges outside their targets lie just past that span, where the target had barely risen. The square is not physical: the envelope of the error grows about in proportion to time (T^0.96), so the target is too tight just after that span and twice too loose at 15 years. The Juno and Cassini files as published leave about 20 km and 10 km with JPL's own ephemeris through this reduction. That is consistent with a station delay left twice in the round-trip time; this is an inference from DE441 and is not stated in the files' headers. After that correction the Cassini file still leaves 590 m rms with JPL's own positions, so Jupiter and Saturn are outside the comparison. FrameOutsideCard tests the speed of the centre of mass and nothing else. Where the centre of mass is, is not held: the compared run's stayed within 114 929 m of the origin, and a state moved bodily is the same motion in other numbers with other rounding: moved by 6e13 m it put the Moon 98.5 m from the unmoved run after one year. The axes have a limit of their own, AxesOutsideCard: the orbit of the Earth-Moon barycentre about the Sun must be inclined 21 to 27 degrees to the z axis, as the ecliptic is to the ICRF equator. In DE441, as Horizons serves it, that angle is 23.43 to 23.45 degrees from 1900 to 2100 and falls from 24.94 to 22.56 degrees from 9999 BC to AD 9999; a state given in ecliptic axes instead of ICRF axes has about 0 degrees and is refused. What that limit cannot tell: a turn of the axes about the z axis, by any angle; the equator of another date in place of the ICRF's, which keeps the angle and moves the true pole from the z axis by the precession of those years; and two bodies whose distance bands overlap, Ceres and Pallas, given in each other's place, which are taken without a refusal. Two bodies at exactly one point are refused by a name of the card, BodiesCoincide, before anything divides by the distance between them. Two bodies closer than the compared run's closest pair of their kind, any pair but the Earth and the Moon under 0.26 au or any two asteroids under 0.38 au, are computed as point masses, held by the ledger. The card has no figures: Pallas 100 km from Ceres's centre, inside Ceres, is still two point masses to it, and an agent asking about bodies in contact is asking about another object. A body where the compared run never had it is computed inside its band; the run went through its distances up to the last range compared with measurement, 16.2 years after the 1999 state, each end rounded outward to 0.01 au. From 16.2 to 21 years the run is compared with DE441, a calculation. The run to 16.2 years swept the whole orbit of each inner body, of Jupiter and of the asteroids, and part of the orbits of Saturn (9.03 to 9.97 au), Uranus (19.88 to 20.10 au) and Neptune (29.96 to 30.14 au). Long runs go on, since the card holds no date: carried on past the last measured range, day 5,920.4, in steps of 1800 s, the compared run goes past day 5,934.6, where Saturn goes farther from the Sun than 9.97 au. The comparison with the ranges and the comparison of the engine with the outside model ran together. Every result is of this one start. What was compared is the envelope of that one run up to the last range compared with measurement: each body within the distances the run went through, no pair closer than the run's closest of its kind, the centre of mass within 114 929 m of the origin. The card computes past that envelope, inside each body's band, with the same equations; a start state outside it, or inside it but off the run, Neptune at another longitude for instance, has no comparison of its own.";

/// The result of the cantilever's validation record: the comparison of
/// 2026-10-09 with the held-back half of the recording.
const CANTILEVER_RESULT: &str = "On the eleven blocks of eight periods between 12 s and 24 s of the second ring-down, outside the fit: frequency error 0.0031 Hz (target 0.010), timing error 0.021 of a period (target 0.20), amplitude error 7.03 percent (target 5.0). A plain damped oscillator gives 40.8 percent and 0.048 Hz; the card's amplitude error and frequency error are each under one third of those (target one third). The card's swing is too large in every block and the gap widens, from 3.6 percent at 13 s to 11.7 percent at 23 s: it dies away too slowly once the swing is small.";

/// The result of the link's validation record: the comparison of 2026-10-09
/// with the month that had been kept unread.
const LINK_RESULT: &str = "On the rain events of August 2015, in one run of each event through the engine. Root mean square error 0.581 dB (target 0.90). That is 0.585 of the error of P.838 alone, 0.994 dB (target 0.80). The summed attenuation is 1.069 of the measured sum (target 0.75 to 1.25). No attenuation at all gives 1.621 dB. The rule written with the targets finds 21 gauge events in August: eleven are under 1.0 mm and outside the rule, nine are compared and none was refused: 773 five-minute blocks; and one of 15.8 mm that starts 2015-08-31T17:37Z is outside the rule because its window runs past the end of the archive. A second program's author ran that event to the last minute of data, afterwards: the month then reads 0.622 dB, 0.528 and 0.974, all within the targets. In two of the nine events P.838 alone was closer than the card, 0.644 against 0.675 dB and 0.346 against 0.400 dB. August's rain reached 54 mm/h, inside the 84 mm/h fitted. Its dry levels, 52.60 to 52.90 dB, were above every fitted event's, 51.75 to 52.55 dB, and inside the card's limit. The card's five-minute peak is below the measured five-minute peak in seven of the nine events, by up to 2.2 dB. On the one-minute values the card returns, its peak is below the measured peak in six of the nine events, by up to 2.8 dB, and above it in three, by up to 3.3 dB. In light rain under 2 mm/h the card is about 0.3 dB high. The link leads the gauge by one to two minutes: moving the link series one minute later lowers the error from 0.580 to 0.516 dB, and this is not corrected. The other direction of the same link, sublink 588, run afterwards with the same constants, does not transfer under the same rules: 0.794 dB, no better than P.838 alone, with a summed attenuation of 2.30 of the measured sum, because that receiver's dry level wanders by more than 1 dB within a dry day; in the events where its level was steady the card does about as well as on the compared direction. A second program, on the same machine and from its own cut of the archive, recomputes the same results.";

/// The result of the tyre's validation record: the one scoring of
/// 2026-10-09 on the 60 held-back runs that hold a scored slip, of the 90
/// held back, as its record states it.
pub(crate) const TIRE_EVE_R19_ICE_RESULT: &str = "On the 60 of the 90 runs outside the fit that hold a slip of 5 to 20 percent, as 60 cells of three repeats each; the other 30 step through slips of 0 to 3 percent, outside the card. Root-mean-square error of the friction ratio: 0.034 over the 60 cells (target 0.045), 0.040 over the 24 cells at 2.5 bar (target 0.045) and 0.029 over the 36 cells at 6.5 kN (target 0.045). A ratio of 0.1 that uses no measurement of this tyre gives 0.122, and the fitted runs' geometric mean with no slip term (least squares on the logarithm) gives 0.036 (target: the card's error below both). The mean measured ratio is 0.218, and the card reads high by 0.019 on average. Two conditions outside the fit taken alone are above 0.045, traction at 2.5 bar at 3.5 kN and at 5 kN, with errors of opposite sign; no target is set on a single condition. One run outside the fit, number 153, is a faulty record: it rolled for about 90 s like the others, and its channels are on other scales. Its vertical force reads 1,849 N at a nominal 3,500 N, and its wheel angle spans 0 to 11.75 where every other file spans 0 to 360. It is in the comparison with the rest; without it the three errors are 0.031, 0.034 and 0.029. The card reads high on all six conditions at 6.5 kN, by 0.023 on average. On the 48 fitted cells the error is 0.030. The compared runs came from binary 7a2f3cc8e75aadbbdaf4f74289803ddf1149dbe882d94cadc96f668061b943c7; a later build returns all 108 saved answers byte for byte. A request now states slip_held_s, so those saved requests are refused as they were sent. A second comparison program, on the same machine and on its own reduction of the raw files, gives the same results.";

/// The result of the line-of-sight card's validation record: the comparison
/// of 2026-10-09 with the held-back visits, as its research record states
/// it.
pub(crate) const LOS_RESULT: &str = "On the 795,021 quiet frames of the 134 files outside the fit, one frame ahead, the card's error is 0.510 mas on x and 0.472 mas on y, root mean square, against 0.644 and 0.584 for taking the last centroid (0.790 and 0.807 of it; target 0.85) and 0.660 and 0.748 for taking the lock point (0.772 and 0.631 of it; target 0.85). The mean of the squared error over the variance the card states is 1.004 and 0.986 (target 0.67 to 1.50), the correlation from one frame to the next 0.074 and 0.074 (target 0.15), and in every one of the 134 files that mean is between 0.5 and 2.0 (target 80 percent of the files): from 0.68 to 1.27 on x and from 0.70 to 1.75 on y. All 134 runs completed and none was outside the card. The targets are on quiet frames. Over all 973,388 measured frames outside the fit, quiet and disturbed together, the card's error one frame ahead is 1.35 on x and 1.26 on y of the error of taking the last centroid, and the squared error over the variance it states is 3.9 and 3.0. Disturbances are in 121 of the 134 files and make up 18.3 percent of the measured frames. The card's own whole-file figure on x, nis_sum_x over measured_frames in the final state, lies in 0.5 to 2.0 in 39 of the 134 files. Inside the disturbances, 178,367 frames within 2 s before and 30 s after a centroid more than 5 mas from the lock point, the card is worse than taking the last centroid: 2.03 mas against 1.00 on x and 1.57 against 0.78 on y, and the variance it states is too small by a factor of 17 on x and 12 on y. The card answers those frames and marks them with three flags, FLAG_POINTING_DISTURBED, FLAG_BEYOND_FOUR_SIGMA and FLAG_RECOVERING; the whole-file numbers above are what it does there. A random walk with the same power-law sensor noise, three constants an axis fitted on the same April data, is inside all five targets, at 0.808 and 0.837 of the last centroid's error. The card's two modes and twelve further constants lower the error by 2 and 4 percent: 0.510 and 0.472 mas against 0.521 and 0.489. The targets do not test the mode frequencies: with both moved 20 percent up or down every target still holds. The error is near the floor the sensor's noise sets. The sensor-noise law is what carries the variance targets: a second-order autoregression with one variance is outside the per-file target. On the guide stars outside the fit the y noise exponent comes out 1.045 against the card's 1.273, where x gives 1.134 against 1.151. Inside the compared count rates the two y laws differ by +27 to -21 percent in variance, which the variance target cannot resolve. With a lock point that uses no future, the median of the first 10 s or a running median of past frames, the last-centroid, variance, correlation and per-file targets still hold, each number within 0.04 of the one above, and the lock-point target is no longer a test. Files longer than 10,000 frames were cut at the gateway's limit: 43 files, which lose 12 percent of the set's frames. A second comparison program, with its own filter on whole files and its own FITS reader, on the same machine, gives the same numbers. With the disturbed frames taken out, as a chain set with the numbers above read (Record SHA-256 e0e4b86fcc18c2cd868f587627a0ac2ed0d5dd692292b1febbcc6358976dd849): a run ends at a centroid more than 5 mas from the lock point or more than four stated standard deviations from where it was expected, and starts again 30 s later with nothing known. The three numbers are the record's own and none is fitted to a file outside the fit. The rule on four standard deviations was set on the fitted files of April 2022: there, in the 30 s after a centroid beyond four standard deviations, the x axis is outside four of the five targets (0.921, a squared error over stated variance of 1.641, a correlation of 0.272, 74 percent of files), and on the files outside the fit three (0.933, 1.513, 0.316); such a centroid comes 31 times as often on x as a right variance gives on the fitted files and 21 times on the files outside the fit. Each of the 134 files run as that chain, with the same lock point: of 973,388 measured frames the chain answers 721,727, 74.1 percent, in 428 runs, and takes out 251,661: 56,910 centroids above 5 mas, 305 beyond four standard deviations, where a right variance gives about 91 by chance, and 194,446 in the 30 s after one. 130 of the 134 files hold a frame taken out. Over the answered frames, x and y: 0.787 and 0.805 of the last centroid's error, 0.779 and 0.640 of the lock point's, a squared error over stated variance of 0.987 and 0.970, a frame-to-frame correlation of 0.064 and 0.072, and that mean between 0.5 and 2.0 in 100 and 100 percent of the files with 500 answered frames or more; beside 0.790 and 0.807, 0.772 and 0.631, 1.004 and 0.986, 0.074 and 0.074, 100 and 100 percent on the 795,021 quiet frames, which were chosen with 2 s of look-ahead that the card does not have. The card's own figure, nis_sum_x and nis_sum_y over measured_frames in the final state of a run, lies in 0.5 to 2.0 on both axes in every one of the 312 runs that landed 500 frames or more, where on x it does in 39 of the 134 whole files. The run through the engine and the same chain in plain Python answer the same frames, and their numbers agree to 2.4e-7 mas.";

/// The one line an agent sees in the list for the finger pad: what it is,
/// that its constants are fitted to the finger's own trials, and how the
/// comparison stands.
pub(crate) const FINGERPAD_P02_GLASS_SUMMARY: &str = "One human finger pad pressed on flat glass and twisted about the normal until it slips, the right index finger of participant P02 of the torsion experiment at UCLouvain: the torque, the stuck fraction of the contact, the work and the heat, from eight constants fitted to this finger's own trials. Compared on 2026-10-09 with its held-back trials at 1 N, 10 N and 100 degrees a second and missed: the slip torque was off by +9.4, +29.3 and -13.0 percent against a mark of 5.62 percent. On its fitted trials it is off by +13.6 percent at 0.5 N and +21.7 percent at 5 N, and within 0.7 to 7.4 percent at 2 N from 5 to 50 degrees a second, where its level was taken. It answers at every force and rate above zero with those misses stated; below 0.5 N, above 10 N, below 5 or above 100 degrees a second and past 75 degrees of twist nothing was measured, and a frame there says so. Another block of the same finger differs by 13 to 15 percent, another finger by about a fifth.";

/// The result of the finger pad's validation record.
pub(crate) const FINGERPAD_P02_GLASS_RESULT: &str = "On the 30 trials of this finger outside the fit, three of the twelve targets hold, and the slip torque is outside its target at all three conditions. Slip torque, card against the mean of ten trials: 6.03 against 5.51 N mm at 1 N (+9.4 percent), 35.63 against 27.55 at 10 N (+29.3 percent), 10.15 against 11.66 at 2 N and 100 degrees a second (-13.0 percent); target 5.62 percent. Torque curve from 1 to 75 degrees, as a fraction of the slip torque: 0.073, 0.263 and 0.115 (target 0.0832). Twist at full slip: 40 against 29.0 degrees, 40 against 40.4 and 39 against 48.1 (target 5.75 degrees). Stuck-fraction curve: 0.036, 0.081 and 0.135 (target 0.0595). A simpler rule comes closer than the card on the slip torque at each condition: at 1 N the same model with the constants of another finger, P01, +1.9 percent; at 10 N a straight line in load, +18.4 percent; at 100 degrees a second no effect of rate, -4.7 percent. The card comes closer than torque proportional to load (-25 and +50 percent) and than the nearest fitted load (-44 and -38 percent). The compared build answered from 0.5 to 10 N and from 5 to 100 degrees a second; this one answers at every force and rate above zero, with these gaps stated. Where the card holds its own slip-torque target, on the trials it is fitted to: at 2 N, at 5 degrees a second (10.32 against 10.24 N mm, +0.7 percent), at 50 (10.23 against 10.68, -4.2 percent) and at 20 against the ten trials twisted about the centre of the contact (10.30 against 10.84, -5.0 percent). At 2 N and 10 degrees a second it is just outside (10.33 against 11.08, -6.8 percent), and at 20 with the 20 off-centre trials pooled, as the fit took them, it is -7.4 percent (11.12). On its fitted trials at the other loads it is off by +13.6 percent at 0.5 N and +21.7 percent at 5 N. Why: this finger's slip torque at its five loads was 3.11, 5.51, 11.12, 17.15 and 27.55 N mm; 70 of the 80 fitted trials are the block at 2 N, the fit takes its level from them, and that block stands 24 percent above the power law that the other four loads follow to within 4 percent. At 2 N, the other targets on the four fitted conditions, 5, 10, 20 and 50 degrees a second: torque curve 0.028, 0.060, 0.089 and 0.048 of the slip torque (target 0.0832); twist at full slip 40, 40, 39 and 39 degrees against a measured 45.0, 44.1, 46.4 and 48.8, so 4 to 10 degrees early (target 5.75); stuck-fraction curve 0.110, 0.113, 0.132 and 0.141 (target 0.0595), outside it at all four. The card's slip-torque law is numerically a two-constant power law of load, 6.03 N mm x F^0.772, fitted to the same 80 trials weighted alike: it errs by +9.4 and +29.4 percent at 1 and 10 N where the card errs by +9.4 and +29.3, and the rate term and the five twist constants change the slip torque by 0.1 percent at those loads and by 0.6 percent at most inside the range. The paper's own fit of that law over the seven fingers is n = 0.76 +/- 0.06 and k = 6.74 +/- 1.31 mm. With the paper's exponent and this finger's level from its three fitted condition means the loads outside the fit read +1.4 and +16.7 percent, closer than the card at both; with the paper's own k, the mean of seven fingers, they read +22.3 and +40.8 percent, further than the card, because this finger is one of the two of lowest friction. The five other fingers, with no target: the card is off by 24 percent, root mean square, over their 45 condition means and reads low by 18 percent on average; between the seven fingers the slip torque at one load spreads by 17 to 26 percent. A second comparison program, on the same machine, from its own reduction of all 768 files, gives the same twelve results, and the same against the authors' own per-trial summaries, where the slip torque is off by +9.4, +29.3 and -12.4 percent. The compared binary is kept: 9b7ad4088cd7af1b3255eba36cc99cc2f7297c779f76bbdc8432920f13ff7c0a. The build that carries this text differs from it in the start it accepts, in its flags, in its ledger scale past the compared range and in this description; no fitted constant moved, and the four saved runs of the comparison that lie inside the range come back with every frame the same, byte for byte.";

/// The limits of the finger pad's validation record.
pub(crate) const FINGERPAD_P02_GLASS_LIMITS: &str = "All eight constants are fitted to this finger's own trials; nothing is derived from skin or glass, and the form (two characteristic twists, no load term in the twist of the stuck area) comes from another finger of the same experiment, P01. What the card holds: this finger, in one block of 70 trials of one sitting, slipped at 10.2 to 11.1 N mm at 2 N from 5 to 50 degrees a second (the means of its four conditions; single trials ran from 9.3 to 13.2), and the card gives that to about 8 percent. Those are trials the constants were fitted to. At 2 N from 5 to 50 degrees a second there is no trial outside the fit, so nothing there is a test; at the three conditions outside the fit the slip torque is 9.4 to 29.3 percent off (target 5.62), and the card answers there with those gaps stated. The floors of the slip torque, which no card of one finger can go under: one trial about the mean of its condition inside a block, 5.2 percent on this finger; block to block, 13 to 15 percent; finger to finger, 17 to 26 percent. The block-to-block figure is how far the block at 2 N sits from the power law through the other four loads over the seven fingers: +24 percent on this finger, from -12 to +24 over the seven, 13 percent as a standard deviation and 15 as a root mean square. The paper gives the order of the blocks, 0.5, 1, 5, 10 and then 2 N, not randomised because the camera was refocused for each force, and names the confound itself: total experiment time, with moisture increasing with occlusion time. The card takes its level from the last block and does not know the block. On this finger on another occasion a user should carry 13 to 15 percent, and on another finger about a fifth; another day, another surface, a wet or a treated finger, or a finger sliding along the glass is outside this comparison: the motion is a twist about the normal. The card answers at every force above zero with the law's two load exponents, off at every other load measured: +13.6 percent at 0.5 N, +9.4 at 1 N, +21.7 at 5 N and +29.3 at 10 N. Nothing was measured below 0.5 N or above 10 N, and there a frame carries FLAG_BEYOND_COMPARED. The mean at 2 N and 20 degrees a second pools 20 trials twisted about a point off the centre of the contact, which the paper does not count among its 90 trials a finger and which read 3.9 percent higher on this finger: 11.12 N mm pooled, 10.84 from the ten centred trials. On this finger the slip torque shows no trend with rate from 5 to 50 degrees a second (10.24, 11.08, 10.84 and 10.68 N mm) and the card's rate exponent is -0.009; at 100 degrees a second it was 11.66, the card is 13 percent off it, and the card answers there with that gap stated. Nothing was measured below 5 or above 100 degrees a second, and there a frame carries FLAG_BEYOND_COMPARED. On three of the other fingers the slip torque rose by 60 to 80 percent from 5 to 100 degrees a second. This finger and P01 have the lowest friction of the seven. The twist at full slip is the card's weakest output: 39 to 40 degrees at every load and rate, against a measured 19 to 49 degrees over this finger's nine conditions; inside the range it is 39 to 40 against a measured 44 to 49 degrees, and between fingers it spreads by 11 to 17 degrees. FLAG_FULL_SLIP marks that event, the one the card exists to find, and is not a warning. FLAG_BEYOND_COMPARED marks a frame beyond the forces, the rates and the twist that were compared. The stuck fraction is a second fitted curve of the same form as the torque's, with its own characteristic twist; a bed of springs on a rigid finger would have one; the curve is outside its target at seven of the nine conditions. The stuck fraction and the torque were never measured in one trial. The paper (J. R. Soc. Interface 20: 20220809, sections 2.2.1 and 4.3.1, read in its open text) says that the tangential force of the first experiment, the one with the camera, was corrupted, that the torque was acquired in a second experiment with one centred sensor and no camera, and that \"the experiments were conducted on separate days\", so that \"the moisture of the participant's finger could have impacted the friction\". The card's torque is fitted to the second experiment and its stuck fraction to the first. The trial key of the second experiment numbers this finger's 110 trials in one sequence with the block at 2 N last, trials 41 to 110; that the sequence was one sitting is this record's reading of the key, and the paper does not say so. The twist was compared from 1 to 75 degrees. The plate turned 80 degrees in every trial: at 2 N it held its rate to 77.0 degrees at 50 degrees a second and to 78.9, 79.4 and 79.7 degrees at 20, 10 and 5, and then slowed to a stop at 80, with the torque falling. Past 75 degrees the card goes on with the same bed of springs under the held rate, its torque rising toward its slip limit and its heat growing with the twist, and each frame carries FLAG_BEYOND_COMPARED: the twist from 75 to 80 degrees has no comparison, and the measured plate stopped at 80. The direction of the twist is not an input. The overshoot of the torque before it settles, a few percent, and the shrinking of the contact area during the twist are left out. No heat and no stored energy was measured: the ledger of work, stored energy and heat is that of the model's bed of springs and closes for any value of the eight constants, so it can catch a wrong stepping rule or a corrupted step and cannot catch a wrong constant. Its scale is the larger of 0.0508 J, the scale of the compared build (the law's slip-torque limit at 10 N and 5 degrees a second over the whole turn of 80 degrees), and the plate work of the run: the work of a twist to 75 degrees at 2.07 N and 50 degrees a second is 0.0126 J, so on every twist at 2 N to 75 degrees the fixed energy is the scale, and a heavier load or a longer twist is held to the same relative closure. A step that turns the plate by more than 0.05 of the characteristic twist of the torque is refused as TwistStepAboveQuadrature: the ledger sees the difference of the work and the heat, not each, and the card holds the step at which the four-point rule sums each of them exact to rounding. A force of zero or less is refused as NoContact, and a rate of zero or less as TwistRateNotPositive. The slip torque and the full-slip twist are this record's own reductions of the published per-frame files. The slip-torque window here is the frames whose twist lies within rate x 0.5 s below 75 degrees; the authors' is the 0.5 s before 75 degrees. The two agree to 0.02 percent from 5 to 50 degrees a second and differ by 0.67 percent at 100. The authors' own per-trial summaries, outside the fit and the comparison, are compared by the second comparison program and again by the builder: on this finger's 90 centred trials the values here agree with theirs to 0.04 percent in the condition means at 5 to 50 degrees a second. The raw 1 kHz files are not used. The values given here are this record's reductions of the authors' data, not the authors' own figures, except where the paper is quoted. A run is one twist from zero under one load. A run whose later segment changes the force or the rate is refused whole, with no proof, as LoadChangedDuringTwist, and every run and every single step starts untwisted, so that no request can take up a twist under another load: a start at any twist but zero is refused as TwistNotStartedFromZero. The compared build answered a start at any twist under any load. Every trial of all seven fingers in the archive is now read, so a card rebuilt on this finger needs other data to be compared with. One party wrote the card and ran it; a second comparison program, on one machine, gives the same results.";

/// The lines of the accumulator's validation record, fixed before a held-back
/// file was downloaded.
const ACCUMULATOR_SBO450_PASS_LINE: &str = "For each run the card is driven by a sine of the measured amplitude from the cyclic steady state at the measured mean pressure, and two numbers are compared: the stiffness, which is the swing of the gas pressure per unit of piston travel, and the lead of the pressure over the piston. Stiffness error: median at most 2.0 percent over the 285 runs and in each of the five records, and at most 5.0 percent in any run. Lead error: median at most 0.5 degrees, and at most 3.0 degrees in any run. And the card's two medians lower than those of five baselines: an isothermal and an adiabatic ideal gas, this model unfitted, the measured stiffness of the fitted condition scaled by the ideal-gas law, and this model with the ideal-gas law fitted to the same runs. The mean pressure of a run is taken from the measurement and is not tested. Record SHA-256 721d88a5264b0e15342824290e6305db1df27fab76011333b49d5ed253b38323.";

/// The result of the accumulator's validation record.
pub(crate) const ACCUMULATOR_SBO450_RESULT: &str = "On the 285 runs outside the fit, run once through the engine. The card answers 282 runs and refuses three, the 5 mm runs at 12, 15 and 18 Hz at 120 bar, whose pressure goes past the card's ceiling of 130 bar as the measured pressure does. Stiffness error over all runs: 0.93 percent in the median (target 2.0). By record: 0.26 percent at 20 / 40 bar, 1.45 at 20 / 60, 0.68 and 0.33 at 40 / 80, and 2.72 at 40 / 120 (target 2.0). Largest: 13.96 percent (target 5.0); 27 runs are above 5 percent, all at 0.1 Hz or under. Lead error: 0.27 degrees in the median (target 0.5); 2.82 degrees at most (target 3.0) on the runs answered, with three runs unanswered. Against the baselines: the card's median stiffness error is the lowest, 0.93 percent against 2.20 for the fitted condition's measurement scaled by the ideal-gas law, 2.46 for this model with the ideal-gas law, 3.50 for this model unfitted, 12.2 for an adiabatic and 37.2 for an isothermal ideal gas; its median lead error, 0.27 degrees, is above three baselines' 0.12, 0.15 and 0.25. Above 1 Hz, 212 runs: stiffness within 0.59 percent in the median and 3.49 at most, lead within 0.18 and 0.99 degrees; the two ideal-gas baselines are at 2.2 and 2.4 percent there. At 0.05 Hz and under, 30 runs: the card is too stiff by 7.7 percent on average and 7.5 in the median, and the scaled measurement, 2.4 percent, and the fitted ideal gas, 3.2, are closer. Every baseline is within the median lead target as well, an ideal gas with no heat exchange among them at 0.30 degrees, so that target says nothing of the card's exchange of heat. On the 114 runs of the fitted condition the stiffness error is 0.45 percent in the median and 3.17 at most and the lead error 0.17 and 2.20 degrees; two records of one condition differ in measured stiffness by 0.87 percent in the median. The binary that made the runs has SHA-256 a3d268ca094ba37d1edc91eb3e46d9748deb365d48b9295453e1fa45a8235428. These numbers are of the card as it was then, with a pressure range of 30 to 130 bar, a piston area of 19.2157 cm2 and a vessel of 0.57622 l. A second program, by another party, reduces all 399 files anew, rebuilds all 798 requests, asks them again of the same binary, verifies every proof and reproduces every number above. The card was then corrected: the geometry, and the range drawn in to where the card was within its targets, with refusals by name outside it. Run through that card (binary SHA-256 92e33dfeaacb2c2fceb0bed273e24424ed13d16fa68592c107e70c3171fbe56e): of the 285 runs it answers 196 and refuses 89, the 57 runs at 40 / 120 bar as PressureOutsideCard before the first step and the 32 runs at 0.1 Hz and under of the other four records as HeatShareAboveCard after 9 to 414 of 800 steps. Every run at 0.2 Hz and faster of the four records at 37 to 86 bar is answered. The pressure bands of each charge were drawn after this run, by the same whole-bar rule, from the measured pressures of those four records: on a transcription of the card's equations, which a later recomputation found equal to the engine's numbers in the three runs it compared, every one of the 196 runs outside the fit and the 98 fitted-condition runs answered here stays inside the bands of its charge at every step, so none of the numbers that follow changes. On the 196 answered, worked as the six lines were: stiffness error 0.49 percent in the median; by record 0.21 at 20 / 40 bar, 1.37 at 20 / 60, 0.60 and 0.26 at 40 / 80; 4.05 percent at most; lead error 0.21 degrees in the median and 1.79 at most; the median stiffness error is lower than every baseline's on the same runs, 2.06 percent for the best of them, and the median lead error is above two baselines' 0.10 and 0.11 degrees. 89 runs have no answer. Of the 255 runs within 5 percent and 3 degrees in the first run, 59 are no longer answered, 46 at 40 / 120 bar and 13 at 0.1 Hz and under; none of the 27 that were above 5 percent is answered. Of the 114 fitted-condition runs 98 are answered, 0.42 percent in the median and 1.26 at most. On every step both binaries landed, 479,346 frames of 684 runs, the gas pressure and the gas temperature are the first run's to the last bit of the 32-bit slot, and within 4e-15 in the 64-bit state at the end; gas volume, gas mass, force, work and heat are 1.021818 times the first run's, and the flags are zero. Record SHA-256 ba073e2a1c070aa1282bcea8b330b765a833c53b0ab63367a61fb46722a68bb7.";

/// The limits of the accumulator's validation record.
const ACCUMULATOR_SBO450_LIMITS: &str = "The card's geometry and range were corrected after the comparison, with every run read; the result states the corrected card's numbers on the measured runs. What the card gives: the swing of the gas pressure per unit of piston travel, for the two charges measured, each at the gas pressures it was measured at, and motion quick enough that the heat exchanged with the wall is a small share of the work. The range compared with measurement is the two charges measured, a pressure of 37 to 86 bar inside its charge's bands, a wall at 293.4 to 293.7 K, a gas at 285.9 to 301.8 K and a piston at most 0.7 m/s; a step outside it lands with FLAG_OUTSIDE_COMPARED_RUNS: 85.56 bar is the highest pressure measured in the four conditions in which the card is within its targets; the record at 109 to 132 bar, 40 bar precharge and 120 bar load, reads 2.72 percent median error of the swing (target 2.0) on the card as first compared, and three runs above 130 bar had no answer; between 86 and 109 bar and beyond 132 bar there is no measurement; the 20 bar charge was measured at 37.76 to 42.71 and 55.26 to 66.37 bar and the 40 bar charge at 57.36 to 63.14 and 75.31 to 85.55 bar; the wall of every compared run was 293.46 to 293.60 K; the gas reached 285.97 to 301.78 K; the fastest compared run reached 0.69999 m/s as the mean over a step. What the card refuses by name is physical: a gas mass of zero or less (GasMassNotPositive), a wall at or below absolute zero (WallTemperatureNotPositive), a gas at or below nitrogen's critical temperature, 126.192 K, where it can condense (GasBelowCriticalTemperature), a pressure above the vessel's rated 330 bar (PressureAboveRated: HYDAC's SBO catalogue rates the type SBO450-0.6 at 4700 psi, 330 bar, in its dimension table, and the two 0.6 l parts it lists at 3600 psi, 250 bar; which the rig carried is not on disk, and between 250 and 330 bar a step lands with FLAG_OUTSIDE_COMPARED_RUNS like any step past the compared range), a gas volume past the vessel's with no oil left (OilSideEmpty), and a motion in which the heat exceeds 0.065 of the work (HeatShareAboveCard). The card is never told a frequency, so slow motion is refused on what the card carries. M is the sum over the steps of the motion so far of the absolute work of each step and H the same sum of the heat; a start carries in the sums of the motion before it as work_moved_j and heat_moved_j, so a motion carried on from a returned state, by runs or by single steps, is weighed as one run; a step lands if H is at most 0.065 of the larger of M and 100 J, and is refused otherwise. Over one period of a sine the share is 0.05 at 0.2 Hz, 0.10 at 0.1 Hz and 0.8 at 0.01 Hz, nearly the same at every pressure, charge and amplitude measured. Run as they were compared, two periods from the cyclic steady state at the piston's outermost point, the measured runs at 0.2 Hz reach 0.062 at most and those at 0.1 Hz 0.104 at least; 0.065 is the next two-hundredth over what held, and nothing between 0.1 and 0.2 Hz was measured. The 100 J give the share a meaning before the oil has done work: it is a little over the work of a quarter of the largest measured swing inside the card, 91 J, and under the work of the smallest measured run, 125 J. A piston at rest with the gas at the wall's temperature exchanges nothing and may rest for any time. A piston at rest with the gas off the wall's temperature relaxes with the one time constant; no measurement here is of a rest, and the slow runs show that time constant wrong by a factor of two to three, so from the start of a run such a rest is answered for its first 6.5 J, about 0.3 K of the 40 bar charge, and refused past it. The sums run over the whole motion, so a rest that follows motion is weighed against all the work before it and is refused later than the same rest alone, or not at all; a share over a trailing window of work would weigh it alone, and no such share is in the comparison. The card cannot see motion before its start: it holds the sums a start states and cannot confirm them. Two constants are fitted to this accumulator's own runs at one condition, 40 bar precharge and 60 bar load: the ratio of the piston area to the vessel's gas volume, 3.3348 per metre, and the thermal time constant, 17.52 s. The gas pressure fixes that ratio and neither number alone. The piston area is that of a 50 mm bore, 19.635 cm2, which no source prints: over the seven records the measured force is 19.569 cm2 times the oil gauge pressure less 211 N, a bore of 49.92 mm, and every file's metadata give the force transducer an offset of -0.202 kN. At that area the fitted ratio is a vessel of 0.58879 l, where the catalogue gives 0.6 l and 36 cubic inches, which is 0.58993 l. The card as first compared held 19.2157 cm2 and 0.57622 l: force over oil pressure at one pressure with the offset in it, 2.1 percent low. Its pressure and temperature were not affected; its gas volume, gas mass, force, work and heat were 2.1 percent low. With the 50 mm bore, the 36 cubic inches and no fitted geometry the same model's median stiffness error on the 285 runs outside the fit is 0.72 percent, against 0.92 with the fitted ratio. A third fitted number belongs to the measurement and not to the card: the deflection reading is taken as a lag of 1.52 ms behind the piston, and the measured stiffness and lead are stated with that correction. Fitted to each record alone the lag is 1.47 to 1.53 ms in all seven: it is a property of the acquisition, in which the deflection runs through the test machine's own conditioner and the pressures do not. Above 2 Hz the lead targets compare that fitted lag with a model lead of under 0.1 degrees and say nothing of the card: every baseline is within them. At 0.5 Hz and under, where the lead says something about the exchange of heat, the card's median lead error on the 285 runs is 1.24 degrees, and two baselines are closer, the scaled measurement at 0.94 and the fitted ideal gas at 0.89. The 285 runs are at load pressures of 40, 80 and 120 bar and at half the gas mass: extrapolation in pressure and in charge, at the same frequencies and amplitudes as the fitted runs, on the same accumulator, rig, gas and room temperature. Nothing here tests another vessel, another gas, another temperature, a motion that is not a sine, a rest, or the mean pressure, which is taken from each run. The precharge is absolute: the metadata give both pressure sensors a range of 1 to 201 bar, and the records at 20 bar precharge agree with the card read that way and not as gauge. The exchange of heat is one time constant, and the measurements break that form. Fitted to each record's own runs it is 17 to 18 s at 40 / 60 bar, 13 s at 40 / 80, 9.3 s at 40 / 120, 10.6 s at 20 / 40 and 8.1 s at 20 / 60, about 45 s per litre of gas volume. A time constant proportional to the gas volume, fitted on the one condition in the fit, gives 2.56 percent in the worst record (target 2.0), 6.73 percent at most (5.0) and 4.03 degrees at most (3.0). Within one record the time constant that each frequency asks for falls threefold, from 20 s at 0.01 Hz to 6.6 s at 1 Hz at 40 / 60 bar. The group that made these measurements has published the same of two other HYDAC diaphragm accumulators, SBO500-0,1 and SBO330-0,75, at 20 and 40 bar precharge: \"Newton's law is incapable of representing this physical phenomenon\" and \"Experimental data confirm the failure of Newton's approach\" (Hartig, Depp, Rexer and Pelz, Effects of unsteady heat transfer on behaviour of commercial hydro-pneumatic accumulators, preprint 2012.06526). So at 0.1 Hz and under the card as first compared was up to 14 percent too stiff, and there the served card refuses (HeatShareAboveCard). The 2.72 percent at 40 / 120 bar does not come from the equation of state. Against the NIST WebBook isotherm at 293.15 K the Peng-Robinson density is 0.5 percent high at 20 bar and 1.9 at 120, and its isentropic stiffness 0.2 to 1.8 percent low; in the card these make the charge 0.8 percent softer at 40 / 120 bar than the reference equation would, so with the reference equation the card would be further off there, 2.9 percent too stiff above 1 Hz against 2.1. What the card leaves out is the give of the oil side, in series with the gas: one series compliance, fitted to all seven records by the second program's author, brings every record within 0.33 percent above 1 Hz. That is a lead for another card and is not in this one. Two records of one condition differ in measured stiffness by 0.87 percent in the median, most of it a difference of mean pressure between two fills: with that removed through the model it is 0.35 percent. The unfitted baseline takes its time constant from Rotthäuser's correlation, which its source gives for piston accumulators: a weak baseline for a diaphragm accumulator. The gas temperature sensor reads a fifth of the card's swing at 0.01 Hz and nothing above 1 Hz; its response time is not stated and the temperature is not in the comparison. The oil, the diaphragm and the line are left out. This is a closed gas spring: no orifice, no flow of gas, and choked flow does not arise. The ledger closes for any time constant and any vessel volume: it shows that the pressure, the heat capacity and the energy are one equation of state and that the step integrates them, and it cannot catch a wrong constant. The gas mass is a number the step never changes. One party wrote the card and ran the comparison, on one machine. A second program, by another party, recomputes it from the protocol and gives the same numbers. The corrected card and its run are the first party's alone; no second program has recomputed them.";

/// One validation record: the object, the data, the pass line, the result and
/// the limits. Fitted cards carry their rows of the claims file; a generic
/// specimen says that it was not compared with a measurement.
fn validation(card: &str, specimen: &str) -> Value {
    let parts: [String; 5] = match card {
        "los_jwst_fine_guide" => [
            "The James Webb Space Telescope holding a guide star in fine guide: the offset of the star's centroid from the lock point on the two axes of the Fine Guidance Sensor's ideal frame, every 64 ms, in quiet stretches.".into(),
            format!(
                "Space Telescope Science Institute, Mikulski Archive for Space Telescopes, programme 1163, OTE-26 Routine WFSC/CC Jitter Maintenance: the table FGS CENTROID PACKET of each calibrated fine-guide file, public. The centroid is the flight software's. The card's eighteen constants are fitted on the 127 files of the 19 visits that started in April 2022: 936,273 frames, six guide stars. The nine constants of each axis, x then y: the slow part {} and {} mas with a memory of {} and {} s; the fast part {} and {} mas with a memory of {} and {} s; the mode {} and {} mas at {} and {} Hz with a memory of {} and {} s; and the noise variance of one centroid, {} and {} mas2 at 1,000,000 counts per second, falling as the count rate to the power {} and {}. Outside the fit: the 134 files of the 24 visits that started from 2 May to 21 June 2022, at other dates, 100 of the 134 files from the other guider, and 19 guide stars of which 18 were never fitted. The acknowledgment the institute asks for: \"This work is based in part on observations made with the NASA/ESA/CSA James Webb Space Telescope. The data were obtained from the Mikulski Archive for Space Telescopes at the Space Telescope Science Institute, which is operated by the Association of Universities for Research in Astronomy, Inc., under NASA contract NAS5-03127 for JWST. These observations are associated with program #1163.\"",
                los_jwst_fine_guide::X_AXIS.slow_mas,
                los_jwst_fine_guide::Y_AXIS.slow_mas,
                los_jwst_fine_guide::X_AXIS.slow_memory_s,
                los_jwst_fine_guide::Y_AXIS.slow_memory_s,
                los_jwst_fine_guide::X_AXIS.fast_mas,
                los_jwst_fine_guide::Y_AXIS.fast_mas,
                los_jwst_fine_guide::X_AXIS.fast_memory_s,
                los_jwst_fine_guide::Y_AXIS.fast_memory_s,
                los_jwst_fine_guide::X_AXIS.mode_mas,
                los_jwst_fine_guide::Y_AXIS.mode_mas,
                los_jwst_fine_guide::X_AXIS.mode_hz,
                los_jwst_fine_guide::Y_AXIS.mode_hz,
                los_jwst_fine_guide::X_AXIS.mode_memory_s,
                los_jwst_fine_guide::Y_AXIS.mode_memory_s,
                los_jwst_fine_guide::X_AXIS.noise_reference_mas2,
                los_jwst_fine_guide::Y_AXIS.noise_reference_mas2,
                los_jwst_fine_guide::X_AXIS.noise_power,
                los_jwst_fine_guide::Y_AXIS.noise_power,
            ),
            "On the quiet frames of the 134 files outside the fit, for each axis: the error of the prediction one frame ahead at most 0.85 of the error of taking the last centroid and at most 0.85 of the error of taking the lock point; the mean of the squared error over the variance the card states between 0.67 and 1.50; the correlation of that normalised error from one frame to the next at most 0.15 in size; and that mean between 0.5 and 2.0 in at least 80 percent of the files with 500 quiet frames or more. Record SHA-256 9e160724092e182f847eb0103e6ca7a54aba0f75c5d1ca14dcd275c9a554c8f5.".into(),
            LOS_RESULT.into(),
            "The forecast, made outside the engine with constants fitted on the first nine April visits and compared on the other ten: 0.774 and 0.785 against the last centroid on x and y, 0.779 and 0.636 against the lock point, a mean normalised squared error of 0.680 and 0.938, a frame-to-frame correlation of 0.113 and 0.067, and 84 and 100 percent of files. The constants are fitted to this observatory's own centroids, so the files outside the fit test extrapolation in time and to other guide stars: one observatory, one season, one guiding mode. In brightness they are interpolation: their count rates, 62,000 to 816,000 counts per second, lie inside the fitted 22,900 to 2,820,000, so the noise law is compared at neither end of the card's range. One party wrote the card and ran it; a second comparison program, on one machine, gives the same numbers. The prediction is one frame, 64 ms, ahead. The estimate of the line of sight with the noise taken out is a model output that no measurement here tests. The lock point is not in the file and is the caller's; the validation took the median of the run's good centroids, which uses the whole run. The lock points of the files the comparison answered span -61.1512947 to 60.4564934 arcsec on x and -64.2767 to 50.1812668 arcsec on y, fitted and outside the fit together, and the card computes a lock point outside that span up to the guider's field, 70 arcsec from its centre, past which it refuses as LockOutsideSensorField. The longest run of frames with no measurement that a run of the comparison crossed was 10 frames on the fitted files and 2 on the files outside the fit, and the card predicts any longer run of them, its stated variance growing toward its unmeasured value. Nothing above 7.8 Hz is resolved by a 64 ms frame: the Nyquist frequency is 7.81 Hz, and the card cannot see or state jitter above it, nor tell a folded line from a true one. The card describes quiet fine guide: inside a disturbance, a centroid more than 5 mas from the lock point and the 30 s after it, the filter follows slowly, and on the fitted visits as on the others its error is larger than that of taking the last centroid. The card answers there and flags the frames, FLAG_POINTING_DISTURBED, FLAG_BEYOND_FOUR_SIGMA and FLAG_RECOVERING; an agent that needs the line of sight to the quiet frames' accuracy must not use the flagged ones. It counts the 30 s after a disturbance from the caller's statement at the start of a run, which it cannot confirm; and a caller's lock point that is off by a few mas makes quiet frames read as disturbed, so the card then flags frames that are quiet. It says nothing of the line of sight during a disturbance: the visits of March 2022 are unopened and kept for a card that follows one. The y axis shows lines near 0.51, 1.02 and 1.53 Hz that the model has no part for. In the spectra of the files outside the fit the y feature is a narrow line at 0.290 to 0.294 Hz with a shoulder at 0.32 to 0.34 Hz; the card's 0.3025 Hz is one damped mode laid across both. The commissioning report of the telescope (arXiv:2207.05632, section 3) names a 0.3 Hz oscillation of a vibration damper between the telescope and the spacecraft bus and a 0.045 Hz oscillation attributed to fuel slosh; the card has no part for the 0.045 Hz one. The x feature is a broad hump at 0.50 to 0.60 Hz, for which no published attribution was found. The ledger is an account of variance that follows from the model; it is not a conservation law, and it cannot detect a wrong constant. It confirms that each update is a Kalman update with the optimal gain: it holds with a wrong mode frequency, wrong noise or the two axes swapped, and it says nothing about the telescope.".into(),
        ],
        "cell_18650pf" => [
            "One Panasonic NCR18650PF lithium-ion cell, new, at 25 degrees Celsius, discharged from full charge at a held current of 1C.".into(),
            "The laboratory's tests of that cell (Kollmeyer, Mendeley Data 10.17632/wykht8y7tg.1). The card's constants are fitted on its low-rate and pulse tests. Outside the fit: eleven 1C discharges from full charge, 2.899 to 2.89982 A, and one discharge at 0.86888 to 0.86970 A in 14 holds.".into(),
            "Terminal voltage: RMS error at most 50 millivolts. Delivered capacity: within 3 percent.".into(),
            "On the eleven 1C discharges: voltage error 27.8 to 30.0 millivolts RMS (target 50), 25.7 to 32.4 across the three onset times tried; largest single error 57 millivolts. On the one test that ran to cutoff, delivered capacity +0.04 percent (target 3) at steps of 0.1 s, and +0.014 percent at the protocol's step of 1 s as a second program computes it, the discharge ended by the card's own cutoff.".into(),
            "One cell, new, at 25 degrees Celsius. Compared at 1C on discharges from full charge, 2.899 to 2.89982 A; widened by the card's 0.83 mA band that span is 2.89817 to 2.90065 A. The card takes any held discharge current above zero and flags a step outside that span as FLAG_CURRENT_OUTSIDE_PASSED_SPAN. The dataset's other held currents are the C/20 test (0.145 A), which sets the rest-voltage curve, and the 0.87 A holds of the pulse test, which set the resistance; both are in the fit. One discharge at another current is outside the fit: 04-27-17 3541_dis5_10p.mat, the same cell at 25 degrees Celsius discharged at a held 0.86888 to 0.86970 A from full charge to 2.5 V in 14 holds with rests at 0 A between them. Its totals and its voltage trace were read before it was compared; no output of the card was. Each hold run as a continuation of the one before, the rests left out, at steps of 0.1 s: voltage error during the holds 56.7 millivolts RMS (target 50), rising to 204 and 206 in the last two holds; the card does not end by its own cutoff but leaves its table at soc 0.03 and 2.66 V (StateOfChargeOutsideCard), 71 s after the cell reached 2.5 V, having delivered 0.60 percent more than the cell. 0.87 A is answered with that gap stated. A start below full charge is a rested cell at the rest-voltage table's value, or a continuation at the held current the request states. A current that changes during a run is refused as CurrentChangedDuringDischarge. No drive cycle, other temperature or pack is in the comparison; the twelve-cell pack card is a different card, with its own record. The capacity total of the full discharge was read before the comparison; the voltage traces were not. The card has no temperature and no memory. A second program, written by another party, recomputes the same results on all eleven discharges.".into(),
        ],
        "capsule_stardust" => [
            "The Stardust sample return capsule on its entry of 15 January 2006, from entry interface to parachute opening.".into(),
            "NASA's reconstruction of the entry (peak load 32.89 g) and the radar record of the altitude and speed at parachute opening, 137.9 s after entry interface: three values of one entry, none in a fit. The card is fixed to the capsule's pre-flight design and navigation values and to the U.S. Standard Atmosphere 1976 and World Geodetic System 1984.".into(),
            "Peak load within 3.64 g of 32.89 g; the altitude and the Mach number of parachute opening each reached within 7.4 s of the recorded 137.9 s. The targets are NASA's printed three-sigma pre-entry spreads, set before any trajectory was computed.".into(),
            "Peak load 32.17 g against 32.89 g: 0.72 g under, 2.2 percent (target 3.64 g). The altitude of 31.03 km reached at 138.909 s, 1.009 s after the recorded 137.9 s (target 7.4 s); Mach 1.23 at 136.953 s, 0.947 s before it (target 7.4 s).".into(),
            "One entry of one capsule, from entry interface to parachute opening. NASA's reconstruction is its own fitted simulation, not an instrument reading: no accelerometer flew, and the two times rest on range radar. The targets were set with 32.89 g public. The structure used, E, was chosen from a ladder of structures with the forecast's errors on each read; the simpler B1 comes within the peak-load target by 0.12 g and goes outside it under 8 of the 25 input changes tried on it. The air is a yearly standard atmosphere, not the air of that morning, which is worth about 1 g of peak load either way. Heat flux: the card gives 696 W/cm2 against NASA's computed 762 to 1000. The card computes NASA's pre-entry forecast from the same pre-flight values; a second capsule is the comparison past it. Past the later of the two deployment crossings the card refuses a state (PastFlightDeployment). A second program, written apart from the card, repeats the run with its own comparison: the same three numbers and the same run proof; no organisation outside the company has repeated it.".into(),
        ],
        "capsule_insight" => [
            "NASA's InSight entry capsule on its entry at Mars of 26 November 2018, from the first sample of the public flight record, 126.5 km up, to parachute deployment 209.3 s later.".into(),
            "The flight's own inertial record, 200 samples a second, and the path added up from it: NASA Planetary Data System bundle insight_edl. The card's air is the temperature of seven Mars Climate Sounder profiles (PDS volume MROM_2147) set on the surface pressure from the lander's barometer (NTRS 20200003204); its drag is the 2008 Phoenix aerodynamic database (NTRS 20080034648). No number in the card comes from the accelerometer. The first 150 s of the record are in the fit, for the lift; the last 59 s before the parachute are outside it.".into(),
            CAPSULE_INSIGHT_PASS_LINE.into(),
            CAPSULE_INSIGHT_RESULT.into(),
            CAPSULE_INSIGHT_LIMITS.into(),
        ],
        "generator_mmrtg_f1" => [
            "One radioisotope generator, flight unit F1, on NASA's Curiosity rover: its daily-average electrical output.".into(),
            "Historical RTG Performance Data, 2023 update (Dryad 10.5061/dryad.1zcrjdfw2): the daily-average electrical output on mission days 0 to 4,087, each as a fraction of 115.4 W, the highest daily average of the first 30 days. Three constants are fitted on the first 2,061 days, mission days 0 to 2,060. Days 2,061 to 2,808, 748 days, chose the card's structure. Days 2,809 to 4,087, 1,279 days, are outside both, and are the days the card is compared with.".into(),
            "Error: the card less the measurement, as a fraction of 115.4 W. Targets, each 0.010 of 115.4 W, which is 1.154 W: RMS error over days 2,809 to 4,087 at most 0.010, and mean error over the final 365 days at most 0.010 in size. Record SHA-256 ea2f70ec6b58a237d9df8ace08f62e90370114aeadce482eeb00fa99cad56cc7.".into(),
            "On the 1,279 days 2,809 to 4,087, in one run of 4,087 daily steps from landing. RMS error 0.008129, about 0.94 W (target 0.010). Mean error over the final 365 days +0.010094, about 1.16 W high (target 0.010 in size), 0.0000936, about 0.011 W, past the target. Largest error on one day +0.02251, 2.60 W high, on mission day 3,546.".into(),
            "One generator; the daily-average electrical output; the record holds no heat measurement. The card reads high on days 2,809 to 4,087 and the gap grows: its mean error is +0.0031, +0.0045 and +0.0096 of 115.4 W over three successive spans of 365 days and +0.0107 over the last 184, about 0.36, 0.52, 1.11 and 1.23 W. Days 2,061 to 2,808 chose the card's structure; they are not part of the comparison. Another structure comes within both targets on days 2,809 to 4,087; it was found with those days read, and the card keeps the structure chosen without them, every constant as compared. Mission day 4,087 is the last day of the published record; the card carries the same exact map past it and flags each later step FLAG_PAST_RECORD, and since it reads high and the gap grows each year, later days are expected to read higher still. The record after October 2023 is the next data it meets. A second script recomputes the comparison. A second program, written from the protocol by an SI that had read the record's numbers, on engine 0.2 on the same machine, recomputes the same RMS error, 0.008129, the same final-year mean, +0.010094, and the same run proof.".into(),
        ],
        "link_gothenburg_10010" => [
            "One commercial microwave link in Gothenburg, Sweden: link 10010, sublink 587, of the OpenMRG data set, 32.445 GHz, vertical polarisation, 1.25841 km, a 0.3 m antenna at each end, sent level 5 dBm, with the weighing rain gauge at Barlastplatsen 37 m from the path.".into(),
            "The OpenMRG data set (10.5281/zenodo.7107689): the sent and the received level every 10 s and the gauge every minute, June to August 2015. The rain term is Recommendation ITU-R P.838-3 at this frequency and polarisation, with nothing fitted: k = 0.27282714286921533 and alpha = 0.8947805998167349 at 32.445 GHz, vertical. The three constants of the water on the antennas are fitted on this link's own readings in the 18 rain events of June and July: a ceiling of 2.13 dB, a wetting time of 81 s and a drying time of 404 s. Every reading of August is outside the fit, unread until the targets were fingerprinted.".into(),
            "On five-minute means of the attenuation above the dry level over every August rain event of at least 1 mm: root mean square error at most 0.90 dB, at most 0.80 of the error of P.838 alone on the same blocks, and the summed attenuation between 0.75 and 1.25 of the measured sum. Record SHA-256 f60bfcc70553f95fef5d35963b6550120de3c67e712c057e40c5d37c51784625.".into(),
            LINK_RESULT.into(),
            "On the 18 events in the fit, the card's error is 0.667 dB, against 1.028 dB for P.838 alone and 1.589 dB for no attenuation; with each event left out of the fit in turn, 0.677 dB. The structure and the link were chosen on June and July. August tests new rain events on the same link in the same summer, inside the rain rates fitted: interpolation in rain rate, later in time. No other link, frequency, polarisation or season, and no snow, is in the comparison. One link, one direction, one gauge at one end of the path: rain on the path that differs from rain at the gauge is not in the card, and the fitted event with the heaviest rain, 84 mm/h, has the largest error, 1.84 dB. The dry level is the caller's, read from the link itself before each event, and the result depends on it: in the comparison it was the median of the hour before each event; one receiver step, 0.3 dB, of error in it puts the summed attenuation outside its band, at 1.69 of the measured sum, and a single dry level for the whole month, 52.30 dB, gives 0.644. An agent with a rain forecast and no reading of the link's level just before the rain does not have what this comparison used. An agent sizing a fade margin must not take the card's peak as the measured peak: the card's five-minute peak was the lower in seven of the nine events, by up to 2.2 dB, and its one-minute peak was the lower in six, by up to 2.8 dB, and the higher in three, by up to 3.3 dB. The dry level's median, 52.30 dB, is 7.98 dB above free space between two ideal 0.3 m apertures (124.667 dB less 2 x 40.172 dB); the data set gives no antenna gain or feeder loss, so that difference is not divided. The received power in watts and the energies are the card's arithmetic on the levels; no energy is in the comparison. The comparison fed one-minute gauge rates of 0 or multiples of 6 mm/h alone, held through whole minutes in steps of 60 s from dry antennas. The card computes past all of these, with the map and the ledgers exact: a rate above 84 mm/h, a rate above 0 and below 6 mm/h, rain changed inside a gauge minute, a step longer than 60 s and a dry level outside the fitted 51.75 to 52.90 dB; none of those is in the comparison.".into(),
        ],
        "reactor_triga_jsi" => [
            "The TRIGA Mark II research reactor of the Jozef Stefan Institute, Ljubljana, pulsed by pneumatic ejection of its transient rod, with the 56-element core of standard fuel (12 wt% uranium, 20 percent enriched, U-ZrH) loaded as core 215 in November 2014, core 217 in March 2015 and core 231 in March and April 2018: the power through the pulse and the seconds after it to the scram, the energy released, and the core-average fuel temperature that energy makes.".into(),
            REACTOR_TRIGA_JSI_DATA.into(),
            REACTOR_TRIGA_JSI_PASS_LINE.into(),
            REACTOR_TRIGA_JSI_RESULT.into(),
            REACTOR_TRIGA_JSI_LIMITS.into(),
        ],
        "cantilever_eth" => [
            "One aluminium strip, 326 mm long, 40 mm wide and 1 mm thick, clamped at one end under a bolted steel plate with nothing between plate and strip, at ETH Zürich: its first bending mode in free decay in room air, as the deflection at the laser-1 station, 268 mm from the clamp.".into(),
            "Ring-down recordings of the strip, doi 10.5281/zenodo.22723484. The card's six constants are fitted on one recording, test1a, from 0.863 s to 24 s after its start. A second ring-down of the same strip in the same clamp, test1b, is outside the fit: its first 12 s inform a forecast made outside the engine, and its 12 s to 24 s, outside the fit and the forecast, are what the card is compared with. Before the split was written, the coarse envelope of every recording, the peak-to-peak value of Laser_1, the channel of the amplitude error, in each half second of the first 30 s, was printed; no fit was made to it.".into(),
            "On the eleven blocks of eight periods between 12 s and 24 s of the second ring-down: amplitude error at most 5.0 percent, frequency error at most 0.010 Hz, timing error at most 0.20 of a period by 24 s, and the amplitude error and the frequency error each at most one third of a plain damped oscillator's. Record SHA-256 d081cc5ae51b5de4a028cd17b4e1ae80b402f71fdc786f82876713f3b925d4a6.".into(),
            CANTILEVER_RESULT.into(),
            "One strip, one clamp, one afternoon. A second comparison program, written from the record's text alone with a reader of its own, gives the same frequency and timing errors on the same eleven blocks; it takes each period's amplitude from a fitted sine where the first takes half the peak-to-peak value, and gives 7.94 percent where the first gives 7.03. The forecast for this structure, made outside the engine on the first 12 s of the second ring-down, is 2.22 percent in amplitude, 0.0047 Hz in frequency and 0.024 of a period in timing, and the card run through the engine gives the same figures there. This structure of six constants was chosen with the errors on those 12 s read, over a smaller structure of five; on the same blocks, computed outside the engine, the five-constant structure gives 6.48 percent in amplitude, closer than this card. The last two blocks lie below the smallest amplitude the constants are fitted on, 0.072 mm; without them the amplitude error is 5.7 percent (target 5.0). The comparison spans envelopes from 15.79 mm down to 0.058 mm: the compared run's own start, at 15.786 mm, is the largest envelope compared, and its readings start from 14.404 mm. From 15.79 mm up to 18.75 mm, the largest amplitude the constants are fitted on, the card answers with the frame flagged FLAG_ENVELOPE_ABOVE_COMPARED. Above 18.75 mm the card refuses by name, as EnvelopeAboveCard: the fitted laws end there, and the strip's own yield needs its alloy, which the dataset does not state. Below 0.058 mm the card answers on to rest with the frame flagged FLAG_ENVELOPE_BELOW_COMPARED. The 12 s to 24 s of this ring-down are now in this comparison: the next comparison needs a ring-down no model has seen. One party wrote the card, ran it and compared it twice, on one machine; no one outside the company has repeated it. The laser was logged in volts: the scale in metres rests on one line of the authors' script, confirmed by a second reading to 3 percent, and every compared quantity is a ratio or a frequency. The card has no mass, so its energies are per kilogram. The softening spring is a power law fitted on amplitudes from 0.072 to 18.75 mm, not derived from the strip's dimensions. A start inside the amplitude ceiling with nearly all its energy as speed can be refused some steps later, because the softened spring carries the deflection envelope past the ceiling: from zero deflection with the envelope at 0.99 of the ceiling the step is refused after 107 steps of 0.25 ms, and at 0.98 the beam rings through.".into(),
        ],
        "creep_cmsx6" => [
            "One cast of the single-crystal nickel-base superalloy CMSX-6, cast by PCC Airfoils as [001] bars, solution annealed and aged: twelve round test pieces 6 mm in diameter, within 1.9 to 7.5 degrees of [001], in tension creep at 980 degrees Celsius under a held load at initial stresses of 140, 170, 200 and 230 MPa, three at each.".into(),
            "BAM Reference Data: Creep of Single-Crystal Ni-Based Superalloy CMSX-6, version 2.1 (doi 10.5281/zenodo.20132712): twelve creep tests to DIN EN ISO 204; cite gives the full reference. Fitted on six, two at each of 140, 170 and 230 MPa: six constants. Compared with the other six, not in the fit: all three at 200 MPa, a stress no fitted test has, and one more test piece at each of 140, 170 and 230 MPa.".into(),
            "Error measure: the natural logarithm of the card's time over the measured time to reach 0.1, 0.2, 0.5, 1, 2 and 5 percent creep extension, over the six tests not in the fit. Targets: RMS at most 0.15, worst at most 0.30, mean at most 0.10 in size. Record SHA-256 f20f99b0e3419d66201a3e124b5c216c4d138d41f976b19d6a536daf15316d87.".into(),
            CREEP_CMSX6_RESULT.into(),
            "One cast, one temperature, one laboratory. Six constants are fitted on six tests of this cast; the law was chosen with the fitted tests read, and the range at 140 MPa was set with all twelve read. On the fitted tests, each predicted from the other five: RMS 0.112, worst 0.279, mean -0.001; the 170 MPa tests predicted from 140 and 230 MPa alone: 0.118, 0.229, -0.039. The test pieces predict each other about as well: each of the twelve predicted from the other two at its stress gives RMS 0.075, the card 0.093. A straight-line interpolation of the fitted measurements, with no creep law, gives 0.098, 0.237 and -0.013 on the same 33 times; a Norton power law gives 0.436 RMS and 0.952 worst. 200 MPa lies where the stress dependence is straight: the minimum creep rate has a stress exponent near 6 from 170 to 200 MPa and from 200 to 230 MPa, and over 10 from 140 to 170 MPa, where no test is outside the fit; how the law carries to a stress away from the fitted ones is not measured. At 0.5 percent the card is late by a mean of 0.095. Below 0.1 percent the card starts later than every measured curve at all four stresses: in hours to 0.05 percent, the card against the three test pieces, 38.9 against 15.4 to 26.7 at 140 MPa, 7.8 against 2.7 to 3.5 at 170 MPa, 1.69 against 0.61 to 0.66 at 200 MPa and 0.245 against 0.06 to 0.08 at 230 MPa. The measured curves gain 0.02 to 0.04 percent within the first hour and the card stays near zero; its extension is under the measured one by at most 0.07 percent. The card answers there, and each frame carries FLAG_OUTSIDE_MET_RANGE. At 140 MPa the three tests stopped at 1010 to 1030 h, at 0.79 to 0.83 percent, so the levels 0.1, 0.2 and 0.5 percent are the ones compared there, and the card's 1 to 2.09 percent at 140 MPa is beyond any measurement. Between 0.5 percent and where those tests stopped the card runs ahead of them: it reaches 0.8 percent at 683 h where the two tests that got there took about 1000 h, and at the hour each test stopped it gives 1.70 to 1.78 percent, about a factor of 2 in extension. The card answers past 0.2 percent below 170 MPa with this gap, and each frame there carries FLAG_OUTSIDE_MET_RANGE. On 171 levels from 0.10 to 5.00 percent, with all twelve tests: at 170, 200 and 230 MPa the time since load is within 0.30 in log time at every level on all nine tests, the worst 0.289, at 170 MPa and 0.33 percent on a test not in the fit, a margin of 0.011. At 140 MPa all three tests are within 0.30 at every level from 0.10 to 0.25 percent, the worst 0.296; one is outside it from 0.26 percent, a second from 0.31, and all three from 0.66 percent. Below 170 MPa the law is within 0.30 to 0.2 percent, the worst 0.196. No test lies between 140 and 170 MPa, so the law holds its 0.30 to 5 percent from 170 MPa, the lowest stress it holds there, and the stress exponent between 140 and 170 MPa is where it is least measured. The card's clock is the time since load because that is what holds: the law's time between two neighbouring levels is off by up to 1.06 in the logarithm against the measured curves, and its errors cancel only in the sum from zero. Its exponents, 33.4 and -27.2, are fitted numbers of this law and not physical constants of the alloy. Between repeat test pieces at one stress the measured time to 0.1 percent differs by up to 0.27 in the logarithm. No rupture, no extension beyond 5 percent, no other temperature, no changed or removed load (a changed stress is refused as StressNotHeld), no other crystal direction. A second program, written apart, recomputes the same 33 values to 1e-12 on the same machine; a second machine and a second organisation have not repeated it.".into(),
        ],
        "tire_eve_r19_ice" => [
            "One 235/55 R19 all-season sport-utility tyre of the European project EVE, rolled on a prepared ice track by the Terramechanics Rig of the Advanced Vehicle Dynamics Laboratory at Virginia Tech in April 2016, at a creep of about 0.06 m/s: the ratio of its longitudinal force to its normal load at a held slip, driven and retarded.".into(),
            "Dataset of tyre tests on ice, parts 1 to 3, doi 10.5281/zenodo.1009017, 10.5281/zenodo.1009023 and 10.5281/zenodo.1009025: 252 runs, each stepping the wheel through four slips, the first three for about 21 s each and the last until the wheel stopped, 21 to 33 s. The card's three constants are fitted on 72 runs at 3.5 and 5 kN and at 1.5 and 3.5 bar. Outside the fit: all 54 runs at 6.5 kN, a load above every fitted load, and all 36 runs at 2.5 bar at the two fitted loads, a pressure between the two fitted pressures. The 90 runs on a water film with a steered wheel are left out.".into(),
            "On the 60 cells outside the fit, each the mean over three repeats of the last nine seconds of a slip stage at 5, 6, 7, 12, 15 or 20 percent slip: root-mean-square error of the friction ratio at most 0.045 over all 60 cells, over the 24 cells at 2.5 bar and over the 36 cells at 6.5 kN, and over the 60 cells smaller than the error of a ratio of 0.1 that uses no measurement of this tyre and smaller than the error of the fitted runs' mean with no slip term. That mean is the geometric mean of each mode, least squares on the logarithm, as the fit is; the arithmetic mean gives 0.038, above the card's error either way. Record SHA-256 84a380a5c948cd741ac91845fbfeac7ed1b60ed3c6a01855bcb99c996e7782c1.".into(),
            TIRE_EVE_R19_ICE_RESULT.into(),
            "The three constants are fitted to this tyre's own runs; nothing is derived from the rubber or the ice. The runs at 2.5 bar test interpolation in pressure and those at 6.5 kN test extrapolation in load, on the same rig, the same ice and in the same campaign as the fitted runs: another ice temperature, another speed, another tyre or another laboratory is outside this comparison. The files record no ice temperature, no carriage speed and no rolling radius, and the tyre's make is not published. The load and the pressure do not enter the friction ratio: on the fitted runs a ratio that varied with them predicted a left-out condition worse than one that did not, and the differences between conditions, about 0.03, were three times the scatter between repeats, so part of what a condition shows is the state of the ice on the day. Slips under 5 percent and the rise of the force from rest are outside the card. The speed is a creep of 0.06 m/s: the friction of ice changes with sliding speed, and the card is of this rig's creep, not of a tyre on a road. The error is between conditions and not between repeats: a condition sits 0.01 to 0.06 from the card as a block, and the card cannot say which way the ice of a given session will sit. The error follows the test block, not the load or the pressure: the 18 condition means of the measured ratio, in six blocks, run from 0.185 to 0.287, and the standard deviation between them is 0.029, against 0.005 to 0.011 between repeats. The ice changed between sessions and within the first block, where the ratio rose by 0.010 to 0.024 from the first repeat to the third. Any model that does not know the block lands near 0.03, and the card is at that floor. A user should carry about ±0.03 (one standard deviation) on the ratio for the session. The figure of 0.23 to 0.24 is this tyre on that rig's ice in that fortnight at 0.06 m/s, not a figure for a tyre on ice at a stated temperature or at road speed. The slip term is real on the runs outside the fit and small: the exponent fitted to them alone is -0.107 ± 0.014, where the card holds -0.133, and the term is worth 0.002 to 0.003 of the 0.03 error. The longitudinal force at rest before a run, about -62 N with a standard deviation of 49 N, is not subtracted from the measured forces. The ledger of the three works closes for any constant, because each work is proportional to the force: it can catch a corrupted step, and it cannot catch a wrong constant, a wrong exponent or a wrong ice. The time after a slip change, by the same cells and targets, with the change placed where the wheel speed steps: every one-second window from 5 s to 20 s after a change is inside all five targets, and every one before 5 s is outside one: the first second gives 0.0477 at 6.5 kN (target 0.045), and 0.0449 against a constant ratio's 0.0441 over the 60 cells, and the seconds from 1 s to 5 s do no better than the constant ratio, 0.0430 against 0.0424 from 1 s to 2 s and 0.0399 against 0.0395 from 2 s to 5 s; from 5 s to 11 s the card has 0.0365 and from 11 s to 20 s 0.0337, inside every target. The card answers a step before 5 s with the same steady force and the frame carries FLAG_SLIP_SETTLING. The range the card was compared on, in numbers: loads of 3.5 to 6.5 kN, fitted at 3.5 and 5 kN and compared at 6.5 kN with a root-mean-square error of 0.029 (target 0.045); pressures of 1.5 to 3.5 bar, compared at 2.5 bar with 0.040; slips of 5 to 20 percent; a carriage speed of about 0.06 m/s; and 5 s to 21 s after a slip is set. Past the loads, the pressures and the hold measured, the card answers with the same law and the frame carries FLAG_BEYOND_COMPARED: nothing was measured there. A start states slip_held_s, and the state returns it. One party wrote the card and ran it; a second comparison program, on the same machine, gives the same results.".into(),
        ],
        "fingerpad_p02_glass" => [
            "The right index finger pad of participant P02, one of seven volunteers of 23 to 35 years, held still with the nail up while a robot presses a flat glass plate on it at a held normal force and turns the plate about the normal through 80 degrees, at a held rate to between 77 and 79.7 degrees and then slowing to a stop at 80, at the Université catholique de Louvain: the torque of the pad on the plate and the stuck fraction of the contact as the twist grows, to full slip.".into(),
            "Biomechanics of the finger pad in response to torsion, J. R. Soc. Interface 20, 20220809, doi 10.1098/rsif.2022.0809, and its data, doi 10.5281/zenodo.7646119: seven fingers, five normal forces at 20 degrees a second and five rates at 2 N, clockwise and counter-clockwise, five repeats, in two experiments on separate days: the first over a camera, which gives the stuck fraction, and the second over a centred torque sensor with no camera, which gives the torque. The archive's torque experiment has 110 trials a finger: the paper's 90 and 20 more at 2 N and 20 degrees a second twisted about a point off the centre of the contact. The card's eight constants are fitted on 80 trials of P02 at 0.5, 2 and 5 N and at 5, 10, 20 and 50 degrees a second, 70 of them in the block at 2 N. Outside the fit: P02's 10 trials at 1 N, 10 at 10 N and 10 at 100 degrees a second, and the fingers P03 to P07 whole. The finger P01 was read whole and gave the form of the model. The paper's own fit of the slip torque over load, T/F = k F^(n-1) for each finger, is k = 6.74 +/- 1.31 mm and n = 0.76 +/- 0.06 over the seven; this finger's own exponent is 0.72 to 0.77 by the weighting. The authors' per-trial summaries and the raw 1 kHz files are outside the fit and the comparison; the summaries are compared after it.".into(),
            "Twelve targets, four at each of P02's three conditions outside the fit, each set at the scatter of one trial about its own condition's mean on the fitted conditions: slip torque (the mean over the 0.5 s before 75 degrees) within 5.62 percent of the measured mean; torque curve from 1 to 75 degrees within 8.32 percent of the slip torque, root mean square; twist at full slip (stuck fraction under 0.03) within 5.75 degrees; stuck-fraction curve within 0.0595, root mean square. Beside each slip torque: a constant, torque proportional to load, a straight line in load, the nearest fitted condition, and the same model with the constants of finger P01, which never saw P02. A target that one of those also holds is no evidence for the card. The five other fingers carry no target. Record SHA-256 49a4788d1eb6e5d5881d4f3b2195ad248a2a543c2da45fb1c8ffb7dc27672081.".into(),
            FINGERPAD_P02_GLASS_RESULT.into(),
            FINGERPAD_P02_GLASS_LIMITS.into(),
        ],
        "accumulator_sbo450" => [
            "One hydraulic diaphragm accumulator, HYDAC SBO450-0.6 with no foam insert, charged with nitrogen, on the accumulator test rig of the Chair of Fluid Systems at Technische Universität Darmstadt in March 2024: the pressure of its gas as a piston pushes oil into it and draws it out in a sine of 2 mm or 5 mm at 0.01 to 20 Hz.".into(),
            "Seven records of the accumulator 600ml empty (Rexer and Lefemmine): 10.5281/zenodo.11265458 and 10.5281/zenodo.15478932 (precharge 40 bar, load 60 bar), 10.5281/zenodo.11265391 (20 / 40), 10.5281/zenodo.11265436 (20 / 60), 10.5281/zenodo.11265491 and 10.5281/zenodo.11265583 (40 / 80) and 10.5281/zenodo.11265625 (40 / 120): 399 runs, each one frequency and one amplitude, with piston deflection, force, gas pressure, oil pressure and three temperatures at 1 kHz. The ratio of piston area to vessel volume and the thermal time constant are fitted on the 100 runs from 0.01 to 10 Hz of the two records at 40 / 60 bar. Outside the fit: all 285 runs of the other five records. The piston area, a 50 mm bore, was set afterwards from the force and the oil pressure of all seven records. Nitrogen by the Peng-Robinson equation with the critical constants of Span et al. 2000; against the NIST WebBook isotherm at 293.15 K its density is 0.5 percent high at 20 bar and 1.5 percent high at 80 bar.".into(),
            ACCUMULATOR_SBO450_PASS_LINE.into(),
            ACCUMULATOR_SBO450_RESULT.into(),
            ACCUMULATOR_SBO450_LIMITS.into(),
        ],
        "tokamak_vde_mast" => [
            TOKAMAK_VDE_MAST_WHAT.into(),
            TOKAMAK_VDE_MAST_DATA.into(),
            TOKAMAK_VDE_MAST_PASS_LINE.into(),
            TOKAMAK_VDE_MAST_RESULT.into(),
            TOKAMAK_VDE_MAST_LIMITS.into(),
        ],
        "satellite_lageos1" => [
            "The laser-ranging satellite LAGEOS-1 in free flight, started from its published position and velocity at one instant. The same card, unchanged, was also run on the satellite LAGEOS-2.".into(),
            "International Laser Ranging Service combined orbits (product 10.5067/SLR/SLR_ILRSORB_001) of LAGEOS-1 and LAGEOS-2 on 28 days, 2026-08-23 to 2026-09-19: 56 runs of up to seven days. Two LAGEOS-1 weeks, 14 runs, chose the card's structure; two more LAGEOS-1 weeks and four LAGEOS-2 weeks, 42 runs, are outside that choice.".into(),
            "Distance from the published orbit, worst over the runs: at most 1 m after one revolution, 10 m after one day and 100 m after seven days. Record SHA-256 52114d795811fc014b3114c4bec8726d33ca0771dc295c57253d914243498fed.".into(),
            "On the 42 runs, none refused. LAGEOS-1: worst distance 0.65 m after one revolution (target 1 m), 3.8 m after one day (10 m), 32 m after seven days (100 m). LAGEOS-2: 0.34 m, 2.9 m and 22 m.".into(),
            "Two LAGEOS-1 weeks chose the card's structure, from a ladder of structures written first, with sunlight pressure and relativity as rungs and the solid tide added later. The smallest structure on the ladder within the targets on those two weeks was the field to degree 8; the card is built at degree 12, chosen on the same two weeks. Of the six weeks outside that choice, one LAGEOS-1 week had been downloaded and its analysis centres compared with each other, with no model run on it; the other five were unread. The published orbit is the analysis centres' fit to laser ranges, good to a few centimetres. The start state is turned into the card's frame outside the engine, as start_state says; a crust-fixed state fed in directly was 69 to 81 m off after one revolution and 374 to 385 m after one day. Free flight; compared on 28 calendar days, 2026-08-23 to 2026-09-19, for at most seven days from a published state, on the two orbits of LAGEOS-1 and LAGEOS-2 and on no orbit between or beside them. The card computes other orbits, longer forecasts and any date from 2017-01-01 to 2027-06-28, the span its locked TT - UTC holds, with the same equations; its error grows with the length of a forecast from what it leaves out. It computes any radius down to the Earth's equatorial radius, 6 378.1363 km, where the compared runs went no lower than 11 997.57 km; a landing under 11 997 km carries FLAG_BELOW_COMPARED_RADIUS, and the lower the orbit, the larger the air's drag on the sphere, which the card does not carry. No drag, no light reflected from the Earth, no ocean tide; the Sun and the Moon are approximate. Run once. A second program, written apart from the card, with its own comparison and its own Earth-orientation code, recomputes the same six worst distances to the millimetre. A third, written from the protocol by an agent that did not write the card, with its own Earth-orientation code, recomputes the worst distances to within 7.8 cm.".into(),
        ],
        "grasp_lift_glass_4tu" => card_grasp_lift_glass_4tu_descriptor::validation(specimen),
        "momentum_jwst_srp" => card_momentum_jwst_srp_descriptor::validation(specimen),
        "momentum_jwst_dihedral" => card_momentum_jwst_dihedral_descriptor::validation(specimen),
        "generator_mmrtg_seebeck" => card_generator_mmrtg_seebeck_descriptor::validation(specimen),
        "reactor_msre_pump" => card_reactor_msre_pump_descriptor::validation(specimen),
        "reactor_spert3" => card_reactor_spert3_twin_descriptor::validation(specimen),
        "grid_gb" => card_grid_gb_twin_descriptor::validation(specimen),
        "cell_2170_model3" => card_cell_2170_twin_descriptor::validation(specimen),
        "tokamak_jet_dte1" => card_tokamak_jet_dte1_twin_descriptor::validation(specimen),
        "runaway_18650_lco" => card_runaway_18650_lco_twin_descriptor::validation(specimen),
        "reactor_spert3_ccore" => card_reactor_spert3_ccore_plate_descriptor::validation(specimen),
        "crater_ryugu_sci" => [
            "The crater made on asteroid 162173 Ryugu on 5 April 2019 by the Small Carry-on Impactor of JAXA's Hayabusa2: a 2 kg copper projectile at 2 km/s into a rubble surface under a gravity of 9.3e-5 m/s2. The compared quantity is the crater's apparent radius, at the level of the original surface.".into(),
            "The law of crater size and its two constants: thirteen impact craters in dense dry Ottawa Flintshot sand at 1, 111 and 464 times Earth's gravity, Schmidt, Proc. Lunar Planet. Sci. Conf. 11, 2099, its first table, transcribed from the scan at the NASA Astrophysics Data System. The clock: seven points read by eye from the fourth figure of Gault and Wedekind, Impact and Explosion Cratering, 1231. The projectile and the target point: JAXA's press briefing before the impact. Ryugu's mass, density, size and rotation: the JPL Small-Body Database. Outside the fit, and no part of any constant: the measured crater, apparent diameter 14.5 +/- 0.8 m, Science 368, 67, as given in JAXA's summary.".into(),
            "In the laboratory: fitted on the nine craters made at 1 g, each of the four centrifuge craters, 2.5 to 4.5 decades away in scaled gravity, inside its 95 percent prediction interval, and a mean error of at most 10 percent. On Ryugu: the radius at which the card stops within an allowance worked out by rule from the fit's 95 percent prediction interval at Ryugu, the measurement's stated uncertainty and the uncertainty of the local gravity. Record SHA-256 ee2793f1ce97b42ed07210121d4fb7cc2f3599db1db1a46a9530efd3cb9da9b3.".into(),
            CRATER_RESULT.into(),
            "Nothing is fitted to Ryugu, and Ryugu lies 2.5 decades below the smallest laboratory crater in scaled gravity: the comparison is an extrapolation, in gravity and in material. The crater's measured diameter has been public since 2019, so the Ryugu target is a rule and not a chosen number; the measured diameter and a prediction of 8.6 m worked out by hand were read before the record's hash was taken, and all of Schmidt's first table, the four centrifuge craters outside the fit included, was read while it was transcribed. One crater, measured once. The measured crater is a half circle, stopped on its south side by a block, and the projectile came in at 60 degrees to the surface; the card is a full circle from a square hit. The impactor was a hollow copper shell 13 cm across, of 2 kg; the card models it as a solid copper sphere of the same mass, of radius 3.76 cm, at the full 2 km/s and under gravity reduced by the rotation. With the projectile taken as the 13 cm shell the same law gives 7.83 m, 8.0 percent above the measurement and inside the allowance; with the published team's own inputs (the shell, 2 km/s, g = 1.2e-4 m/s2) it gives 7.50 m, 3.5 percent above. Across twenty reasonable readings of the inputs the prediction runs from 7.15 to 8.80 m, wider than the gap. The projectile's radius and the 60-degree angle each move the prediction more than the rotation term does, and neither is in the allowance. The 18.2 percent belongs to the scaling of dense Ottawa Flintshot sand with these inputs: with the shell, or with the team's inputs, the same law comes inside the allowance. The measurement was read from JAXA's summary of the paper, not from the paper. The card has no cohesion, no boulders, no rim, no depth and no ejecta. Its time is a laboratory clock that runs until the ejecta near the rim have landed, read by eye from a printed figure, with no target on time. Its time constant, 1.318295773573555, comes from seven points read by eye at 1 g; the card stops at 400 s, where the open sources say growth for about 200 s and a stop by about 300 s. No time of Ryugu's crater is in the comparison. Below 0.1622 m the card refuses (RadiusBelowLedgerResolution). One party wrote the card and ran it; a second comparison program, on the same machine, gives the same numbers.".into(),
        ],
        "solar_system" => [
            "The Solar System: the Sun, the eight planets, the Moon and the asteroids Ceres, Pallas and Vesta, carried forward from one state, that of 1999-01-01 00:00 TDB.".into(),
            SOLAR_SYSTEM_DATA.into(),
            SOLAR_SYSTEM_PASS_LINE.into(),
            SOLAR_SYSTEM_RESULT.into(),
            SOLAR_SYSTEM_CAVEATS.into(),
        ],
        "cable_stretch" | "hydraulic_fluid" | "resonance" | "swing" | "tesseract" | "thermal"
        | "ti64_lattice" | "grid_node" | "battery_sag" => own_specimen_record(specimen),
        _ => [
            format!("{specimen} A generic specimen, not a named real object."),
            "none".into(),
            "none".into(),
            NOT_COMPARED.into(),
            "The run obeys the card's own equations and its conservation ledger; that has been confirmed by an outside recomputation in arithmetic of 50 digits or more. Nothing is stated about how closely it matches any real object.".into(),
        ],
    };
    let mut record = Map::new();
    for (name, mut text) in VALIDATION_PARTS.iter().zip(parts) {
        if *name == "limits" {
            if let Some(defect) = known_defect(card) {
                text.push(' ');
                text.push_str(defect);
            }
        }
        record.insert((*name).to_string(), Value::String(text));
    }
    calibration_into(&mut record, card);
    Value::Object(record)
}

/// A second form of a served model's object gives, where its evidence and
/// its record are given, the served model it stands beside as `companion_to`
/// (`find_model::CALIBRATIONS`). What it was fitted on is in its spec sheet.
pub(crate) fn calibration_into(map: &mut Map<String, Value>, model: &str) {
    if let Some(row) = crate::find_model::calibration_of(model) {
        map.insert("companion_to".to_string(), json!(row.companion_to));
    }
}

/// The result of the validation record of a card that is one of the
/// company's own specimens.
pub(crate) const OWN_SPECIMEN_RESULT: &str = "An outside recomputation in arithmetic of 50 digits or more gives the card's runs, and every step holds the card's conservation ledger to its stated limit.";

/// The result of the validation record of a solver that is a company
/// specimen. It claims no outside recomputation, which no such
/// solver has: what it states is the solver's own ledgers and where what its
/// method was held to is written.
pub(crate) const OWN_SPECIMEN_SOLVER_RESULT: &str = "Company specimen: its constants are the company's, stated in its description, and every answer holds the conservation ledgers stated under invariants to their tolerances; what its method is held to, and within what, is under reference.";

/// The validation record of one of the company's own specimens: what it is,
/// the constants it holds, and what its runs are held to.
fn own_specimen_record(specimen: &str) -> [String; 5] {
    [
        format!("{specimen} An object the company specified: every constant is stated in this description and fixed in the card."),
        "The company's constants for this specimen, fixed in the card and listed in its description and its step rule.".into(),
        "Each step holds the card's own equations and its conservation ledger within the limit the card states. Each run is recomputed outside the engine in arithmetic of 50 digits or more.".into(),
        OWN_SPECIMEN_RESULT.into(),
        "A run is the run of the object the company specified, with its stated constants, under the card's equations and inside the card's stated limits. A real object of the same kind has its own constants, from its own measurements.".into(),
    ]
}

/// A defect of a card that is known, stated in the limits of its validation
/// record, and not fixed in this engine.
pub(crate) fn known_defect(card: &str) -> Option<&'static str> {
    match card {
        _ => None,
    }
}

fn describe_one(item: &Card) -> Value {
    let (example_compute_step, example_run_trajectory) =
        worked_examples(item.domain, item.card);
    let mut body = describe_fields(item, example_compute_step, example_run_trajectory);
    if let Some(refused) = example_refused(item.domain, item.card) {
        body = with_after(body, "example_run_trajectory", EXAMPLE_REFUSED, refused);
    }
    if let Some(entry_state) = entry_state(item.domain, item.card) {
        body["entry_state"] = entry_state;
    }
    body["continuation"] = crate::chain::continuation_statement(item.card);
    if (item.domain, item.card) == ("energy", "battery_sag") {
        body["charging_current_limit_by_state_of_charge"] =
            battery_charging_table(&battery_sag::LiIonPack::pack_12s());
    }
    if (item.domain, item.card) == ("aerospace", "satellite_lageos1") {
        body["start_state"] = Value::String(SATELLITE_START_STATE.to_string());
    }
    if (item.domain, item.card) == ("aerospace", "solar_system") {
        body["start_state"] = Value::String(SOLAR_SYSTEM_START_STATE.to_string());
    }
    if let Some(equations) = equations(item.card) {
        body[EQUATIONS] = json!(equations);
    }
    body
}

/// The key of a card's every equation and constant, where its description
/// prints them for a rebuild from the description alone.
pub(crate) const EQUATIONS: &str = "equations";

/// Every equation, constant, property law and discretisation of the cards
/// that print them, one sentence each.
fn equations(card: &str) -> Option<Vec<String>> {
    match card {
        "reactor_spert3" => Some(card_reactor_spert3_twin_descriptor::equations()),
        _ => None,
    }
}

/// The locked state a card's documented run starts from, each number with
/// its hex. The capsule has one: entry interface.
fn entry_state(domain: &str, card: &str) -> Option<Value> {
    match (domain, card) {
        ("aerospace", "capsule_insight") => {
            let specimen = capsule_insight::CapsuleInsight::entry_capsule();
            let state = capsule_insight::CapsuleState::try_entry_interface(&specimen)
                .expect("entry state");
            super::state_object(&[
                ("radius_m", super::StateField::F64(state.radius_m)),
                (
                    "radial_speed_m_s",
                    super::StateField::F64(state.radial_speed_m_s),
                ),
                (
                    "transverse_speed_m_s",
                    super::StateField::F64(state.transverse_speed_m_s),
                ),
            ])
            .ok()
        }
        ("aerospace", "capsule_stardust") => {
            let specimen = capsule_stardust::CapsuleStardust::sample_return_capsule();
            let state = capsule_stardust::CapsuleState::try_entry_interface(&specimen)
                .expect("entry interface");
            super::state_object(&[
                ("radius_m", super::StateField::F64(state.radius_m)),
                (
                    "radial_speed_m_s",
                    super::StateField::F64(state.radial_speed_m_s),
                ),
                (
                    "transverse_speed_m_s",
                    super::StateField::F64(state.transverse_speed_m_s),
                ),
            ])
            .ok()
        }
        _ => None,
    }
}

/// The names a card gives that its kernel's list does not hold: the
/// refusals the gateway itself makes of a request to this card, each before
/// anything is computed. `describe_card` lists them after the kernel's.
pub(crate) fn door_rejects(item: &Card) -> Vec<&'static str> {
    let mut names: Vec<&'static str> = Vec::new();
    // A step under the floor and one over the ceiling. The 2170 cell's kernel
    // has one name for both, already in its list.
    for above in [false, true] {
        let name = crate::step_outside_name(item.card, above);
        if !item.rejects.contains(&name) && !names.contains(&name) {
            names.push(name);
        }
    }
    // A step over `π/ω` is reached only on a card whose ceiling lies above it.
    if item.nyquist_s.is_some_and(|nyquist_s| nyquist_s < item.ceiling_s) {
        names.push(crate::DT_ABOVE_NYQUIST);
    }
    // The titanium card's load rule is held here, and its kernel has no name
    // for it; the cards whose kernels hold the rule list it themselves.
    if item.card == "ti64_lattice" {
        names.push("ForceAboveNyquist");
    }
    if crate::fresh_start::cards().any(|card| card == item.card) {
        names.push(crate::fresh_start::NAME);
    }
    // A card that serves no continuation, and one that serves it from some
    // of its states only (the crater, until it stops), each say so.
    let statement = crate::chain::continuation_statement(item.card);
    if statement["served"] == json!(false)
        || statement["sentence"].as_str().is_some_and(|sentence| sentence.contains("(ContinuationNotOnCard)"))
    {
        names.push(CONTINUATION_NOT_ON_CARD);
    }
    names
}

/// The name of a continuation asked of a card that serves none.
const CONTINUATION_NOT_ON_CARD: &str = "ContinuationNotOnCard";

/// A declared end in the words `describe_card` gives it: the end's own
/// sentence, the same words the ended run's answer carries.
fn declared_end_words(end: &crate::ends::End) -> &'static str {
    end.end
}

fn describe_fields(item: &Card, example_compute_step: Value, example_run_trajectory: Value) -> Value {
    let mut body = json!({
        "domain": item.domain,
        "card": item.card,
        "specimen": item.specimen,
        "validation": validation(item.card, item.specimen),
        "phase_fields": item.fields.iter().filter(|field| field.role == Role::Phase).map(|field| phase_field_json(item.card, field)).collect::<Vec<_>>(),
        "load_fields": item.fields.iter().filter(|field| field.role == Role::Load).map(field_json).collect::<Vec<_>>(),
        "dt_s": {
            "unit": "s",
            "floor_s": exact_value(item.floor_s),
            "floor_from": item.floor_from,
            "ceiling_s": exact_value(item.ceiling_s),
            "ceiling_from": item.ceiling_from,
            "nyquist_s": opt_number(item.nyquist_s),
            "rule": item.nyquist_rule,
        },
        "number_rule": NUMBER_RULE,
        "rejects": item.rejects.iter().copied().chain(door_rejects(item)).map(|name| json!({
            "name": name,
            "meaning": reject_meaning(item.card, name),
        })).collect::<Vec<_>>(),
        "declared_ends": crate::ends::ENDS.iter().filter(|end| end.card == item.card).map(|end| json!({
            "reject": end.reject,
            "end": declared_end_words(end),
            "outcome": "ended",
        })).collect::<Vec<_>>(),
        "example_compute_step": example_compute_step,
        "example_run_trajectory": example_run_trajectory,
        "advisory_flags": item.advisory.iter().map(|flag| {
            let mut entry = json!({
                "name": flag.name,
                "mask": flag.mask,
                "seals": flag.seals,
                "state": flag_state(flag.name),
                "limits": crate::exceedance::describe_kinds(item.card, flag.mask),
            });
            // A flag that marks an event the card exists to find, and not a
            // warning about the model, says so.
            if flag_is_event(item.card, flag.name) {
                entry["kind"] = json!("event");
            }
            entry
        }).collect::<Vec<_>>(),
        "frame_slots": frame_slots(item),
        "frame_rule": frame_rule(item),
        "time_label": time_label(item),
    });
    // What a flag reports beside its bit, on a card that sets one.
    if !item.advisory.is_empty() {
        body["exceedances"] = json!(crate::exceedance::DESCRIBE_RULE);
    }
    body
}

/// A flag that is the event its card exists to find: the full slip of the
/// finger pad, and the incipient and the gross slip of the grasped object.
/// Every other flag of the engine reports a condition of its frame, such as
/// the grasped object resting on the table.
fn flag_is_event(card: &str, name: &str) -> bool {
    matches!(
        (card, name),
        ("fingerpad_p02_glass", "FLAG_FULL_SLIP") | ("grasp_lift_glass_4tu", "FLAG_INCIPIENT_SLIP" | "FLAG_GROSS_SLIP")
    )
}

/// The boolean in the state that sets each advisory flag.
fn flag_state(name: &str) -> &'static str {
    flag_state_known(name).unwrap_or_else(|| panic!("{}", missing_row("flag_state", name)))
}

fn flag_state_known(name: &str) -> Option<&'static str> {
    Some(match name {
        "FLAG_GRID_SURGE" => "surge",
        "FLAG_PLL_SLIP" => "pll_slipped",
        "FLAG_SAG_WARNING" => "sag_warning",
        "FLAG_THERMAL_DERATE" => "derated",
        "FLAG_SELF_HEATING" => "self_heating",
        "FLAG_RUNAWAY" => "runaway",
        "FLAG_STROKE_LIMIT" => "stroke_limited",
        "FLAG_DUFFING_FOLD" => "folded",
        "FLAG_COLUMN_SOFT" => "column_soft",
        "FLAG_COLUMN_LIMIT" => "column_limited",
        "FLAG_TRACKING_DEGRADE" => "tracking_degraded",
        "FLAG_CONTROL_LIMIT" => "control_limited",
        "FLAG_PAST_FITTING_SPAN" => "past_fitting_span",
        "FLAG_DROGUE_MACH_WINDOW" => "drogue_mach_window",
        "FLAG_LIFT_ACTING" => "lift_acting",
        "FLAG_PAST_FLIGHT_DEPLOYMENT" => "past_flight_deployment",
        "FLAG_EARTH_SHADOW" => "in_earth_shadow",
        "FLAG_PROMPT_SUPERCRITICAL" => "prompt_supercritical",
        "FLAG_ROD_MOVING" => "rod_moving",
        "FLAG_RODS_DROPPED" => "rods_dropped",
        "FLAG_PAST_POWER_PEAK" => "past_power_peak",
        "FLAG_SUSTAINED_ABOVE_1C" => "sustained_above_1c",
        "FLAG_CREEP_RATE_RISING" => "creep_rate_rising",
        "FLAG_STOPPED" => "stopped",
        "FLAG_RAINING" => "raining",
        "FLAG_LOAD_ABOVE_FITTED" => "load_above_fitted",
        "FLAG_NO_MEASUREMENT" => "no_measurement",
        "FLAG_POINTING_DISTURBED" => "pointing_disturbed",
        "FLAG_BEYOND_FOUR_SIGMA" => "beyond_four_sigma",
        "FLAG_RECOVERING" => "recovering",
        "FLAG_NEAR_DENSITY_LIMIT" => "near_density_limit",
        "FLAG_FULL_SLIP" => "full_slip",
        "FLAG_BEYOND_COMPARED" => "beyond_compared",
        "FLAG_OUTSIDE_MET_RANGE" => "outside_met_range",
        "FLAG_ENVELOPE_BELOW_COMPARED" => "envelope_below_compared",
        "FLAG_PRESSURE_OUTSIDE_FITTED" => "pressure_outside_fitted",
        // Flags that report where a step lies against the range compared with
        // measurement: the frame carries them, and the state holds no
        // boolean of its own for them.
        "FLAG_PAST_RECORD"
        | "FLAG_CURRENT_OUTSIDE_PASSED_SPAN"
        | "FLAG_OUTSIDE_COMPARED_TRIPS"
        | "FLAG_PAST_COMPARED_MINUTE"
        | "FLAG_BELOW_COMPARED_FREQUENCY"
        | "FLAG_BLOCK_MET_NO_MARK"
        | "FLAG_OUTSIDE_COMPARED_TESTS"
        | "FLAG_OUTSIDE_COMPARED_RUNS"
        | "FLAG_COUNT_RATE_BEYOND_COMPARED"
        | "FLAG_SLIP_SETTLING"
        | "FLAG_BELOW_COMPARED_RADIUS"
        | "FLAG_ENVELOPE_ABOVE_COMPARED" => "none: the frame's flags alone",
        "FLAG_INCIPIENT_SLIP" => "incipient_slip",
        "FLAG_GROSS_SLIP" => "gross_slip",
        "FLAG_ON_TABLE" => "on_table",
        "FLAG_OUTSIDE_FIELD_OF_REGARD" => "outside_field_of_regard",
        "FLAG_PAST_IDEAL_WALL_LIMIT" => "past_ideal_wall_limit",
        "FLAG_FLOW_CHANGING" => "flow_changing",
        "FLAG_FUEL_CIRCULATING" => "fuel_circulating",
        "FLAG_ROD_AT_DRIVE_SPEED" => "rod_at_drive_speed",
        "FLAG_PAST_PEAK" => "past_peak",
        "FLAG_HOT_SPOT_BOILING" => "hot_spot_boiling",
        "FLAG_OUTSIDE_COMPARED" => "outside_compared",
        "FLAG_OUTSIDE_COMPARED_RANGE" => "outside_compared_range",
        "FLAG_SEPARATOR_MELTED" => "separator_melted",
        "FLAG_VENTED" => "vented",
        _ => return None,
    })
}

/// Every slot of the 64-byte frame, in byte order: where it sits and what it
/// holds. Offsets, widths and types are the kernel's own layout; the
/// quantity and unit are the card's packing.
fn frame_slots(item: &Card) -> Vec<Value> {
    use genesis_kernel::frame::FRAME_LAYOUT;
    FRAME_LAYOUT
        .iter()
        .map(|place| {
            let row = item
                .slots
                .iter()
                .find(|row| {
                    row.slot == place.slot
                        || (row.slot.contains("..")
                            && place.slot.split('[').next() == row.slot.split('[').next())
                })
                .expect("a card row for every slot");
            let grouped = row.slot.contains("..");
            let index = place.slot.split('[').nth(1).map(|rest| rest.trim_end_matches(']'));
            let state = match (grouped, index) {
                (true, Some(i)) => format!("{}[{i}]", row.state),
                _ => row.state.clone(),
            };
            let mut slot = json!({
                "slot": place.slot,
                "offset": place.offset,
                "width": place.width,
                "type": place.kind,
                "state": state,
                "unit": row.unit,
                "holds": row.holds,
            });
            if place.slot == "flags" {
                slot["bits"] = Value::Array(
                    item.advisory
                        .iter()
                        .map(|flag| json!({
                            "name": flag.name,
                            "mask": flag.mask,
                            "state": flag_state(flag.name),
                        }))
                        .collect(),
                );
            }
            slot
        })
        .collect()
}

/// The sentence on the rounding of a 32-bit slot, the same on every card.
pub(crate) const FRAME_ROUNDING_SENTENCE: &str = "A frame slot is a 32-bit copy of a 64-bit value and can round to the other side of a limit that the exact value respects (for example a rotor angle just under π/2 stored as a value just over it). The exact values are the `_hex` fields of the answer, and the limits are enforced on the exact values.";

/// The frame, byte by byte, from the kernel's layout, and what a reader
/// needs to read it.
fn frame_rule(item: &Card) -> String {
    use genesis_kernel::frame::FRAME_LAYOUT;
    let layout = FRAME_LAYOUT
        .iter()
        .map(|place| {
            let bytes = if place.width == 1 {
                format!("byte {}", place.offset)
            } else {
                format!("bytes {} to {}", place.offset, place.offset + place.width - 1)
            };
            let kind = match place.kind {
                "f64" => "a 64-bit number",
                "f32" => "a 32-bit number",
                "u64" => "a 64-bit unsigned integer",
                _ => "bytes",
            };
            format!("{} in {bytes} as {kind}", place.slot)
        })
        .collect::<Vec<_>>()
        .join(", ");
    let flags = if item.advisory.is_empty() {
        "This card sets no flag bit; flags is always zero.".to_string()
    } else {
        let list = item
            .advisory
            .iter()
            .map(|flag| format!("{} (mask {})", flag.name, flag.mask))
            .collect::<Vec<_>>()
            .join(" and ");
        format!("The flag bits this card may set: {list}. No other bit is set.")
    };
    let mut rule = format!(
        "The 64 frame bytes are little-endian: {layout}. Each 32-bit slot is the 64-bit value of the quantity named in frame_slots, rounded to the nearest 32-bit number, in the unit given there, unless its row says otherwise. {flags} The proof slot is 16 bytes. The step does not write it. In a recorded frame it holds the first 16 bytes of the link before that frame (the link of the request for the first frame), as the gateway writes it. {FRAME_ROUNDING_SENTENCE}"
    );
    if item.card == "satellite_lageos1" {
        rule.push(' ');
        rule.push_str(SATELLITE_FRAME_RULE);
    }
    if item.card == "solar_system" {
        rule.push(' ');
        rule.push_str(SOLAR_SYSTEM_FRAME_RULE);
    }
    rule
}

/// What the time label is, said once and the same on every card but the two
/// that keep a compensated clock (`COMPENSATED_TIME_LABEL_SENTENCE`).
pub(crate) const TIME_LABEL_SENTENCE: &str = "The time label t_s is the running sum of the steps taken, t_s += dt_s in 64-bit arithmetic, not the step count times the step: ten steps of 0.1 s read 0.9999999999999999 s. The t slot of the frame and every hash over the frame carry this sum.";

/// The time label of a card that keeps a compensated clock, the two MMRTG
/// cards (the kernel's `clock_carry_s`): a running sum would be off by the
/// roundings of its additions, which these cards fold back in (the review of
/// G^G's sector 1, 16 October 2026; every card probed in
/// `fix_014/measure_fix/item10_clock_probe.txt`).
pub(crate) const COMPENSATED_TIME_LABEL_SENTENCE: &str = "The time label t_s is the mission clock, a compensated sum of the start and the steps taken: each step adds dt_s in 64-bit arithmetic, keeps what the addition rounded off and folds it back in at the next step, so t_s stays within half a spacing of 64-bit numbers of the exact sum: ten steps of 4.1 s read 41.0 s, where a running sum reads 41.00000000000001 s. A restart from a returned t_s begins with no carry. The t slot of the frame and every hash over the frame carry this clock.";

/// The cards whose clock is a compensated sum.
pub(crate) const COMPENSATED_CLOCKS: &[&str] = &["generator_mmrtg_f1", "generator_mmrtg_seebeck"];

fn time_label(item: &Card) -> String {
    if COMPENSATED_CLOCKS.contains(&item.card) {
        COMPENSATED_TIME_LABEL_SENTENCE.to_string()
    } else if item.card == "satellite_lageos1" {
        format!("{TIME_LABEL_SENTENCE} For this card the sum is also what places the Sun and the Moon and turns the Earth: they are evaluated at the start epoch plus t_s.")
    } else if item.card == "resonance" {
        format!("{TIME_LABEL_SENTENCE} For this card the sum starts at the t_s the request states, 0 for a new start.")
    } else {
        TIME_LABEL_SENTENCE.to_string()
    }
}

/// A start field, with the door's rule for a continuation field added to
/// its bound.
fn phase_field_json(card: &str, field: &FieldSpec) -> Value {
    let mut row = field_json(field);
    if let Some(sentence) = crate::fresh_start::field_sentence(card, field.name) {
        let stop = if field.bound.ends_with('.') { "" } else { "." };
        row["bound"] = Value::String(format!("{}{stop}{sentence}", field.bound));
    }
    row
}

fn field_json(field: &FieldSpec) -> Value {
    json!({
        "name": field.name,
        "unit": field.unit,
        "components": field.components,
        "minimum": opt_number(field.minimum),
        "minimum_exclusive": field.minimum_exclusive,
        "maximum": opt_number(field.maximum),
        "maximum_exclusive": field.maximum_exclusive,
        "bound": field.bound,
    })
}

fn object_map(value: Value) -> Map<String, Value> {
    match value {
        Value::Object(map) => map,
        _ => unreachable!("schema is an object"),
    }
}

fn worked_examples(domain: &str, card: &str) -> (Value, Value) {
    match (domain, card) {
        ("materials", "ti64_lattice") => ti64_examples(),
        ("energy", "grid_node") => grid_examples(),
        ("energy", "swing") => swing_examples(),
        ("energy", "battery_sag") => battery_examples(),
        ("energy", "cell_18650pf") => cell_examples(),
        ("energy", "thermal") => thermal_examples(),
        ("signal", "resonance") => resonance_examples(),
        ("signal", "tesseract") => tesseract_examples(),
        ("materials", "hydraulic_fluid") => hydraulic_examples(),
        ("materials", "cable_stretch") => cable_examples(),
        ("energy", "generator_mmrtg_f1") => generator_examples(),
        ("aerospace", "capsule_stardust") => capsule_examples(),
        ("aerospace", "satellite_lageos1") => satellite_examples(),
        ("signal", "cantilever_eth") => cantilever_examples(),
        ("materials", "creep_cmsx6") => creep_examples(),
        ("aerospace", "capsule_insight") => capsule_insight_examples(),
        ("energy", "reactor_triga_jsi") => reactor_triga_examples(),
        ("aerospace", "crater_ryugu_sci") => crater_examples(),
        ("energy", "grid_gb") => card_grid_gb_twin_descriptor::examples(),
        ("energy", "tokamak_jet_dte1") => card_tokamak_jet_dte1_twin_descriptor::examples(),
        ("energy", "runaway_18650_lco") => card_runaway_18650_lco_twin_descriptor::examples(),
        ("energy", "reactor_msre_pump") => card_reactor_msre_pump_descriptor::examples(),
        ("energy", "reactor_spert3") => card_reactor_spert3_twin_descriptor::examples(),
        ("energy", "cell_2170_model3") => card_cell_2170_twin_descriptor::examples(),
        ("energy", "reactor_spert3_ccore") => card_reactor_spert3_ccore_plate_descriptor::examples(),
        ("energy", "generator_mmrtg_seebeck") => card_generator_mmrtg_seebeck_descriptor::examples(),
        ("aerospace", "momentum_jwst_dihedral") => card_momentum_jwst_dihedral_descriptor::examples(),
        ("materials", "grasp_lift_glass_4tu") => card_grasp_lift_glass_4tu_descriptor::examples(),
        ("aerospace", "momentum_jwst_srp") => card_momentum_jwst_srp_descriptor::examples(),
        ("signal", "link_gothenburg_10010") => link_examples(),
        ("aerospace", "solar_system") => solar_system_examples(),
        ("materials", "tire_eve_r19_ice") => tire_eve_r19_ice_examples(),
        ("aerospace", "los_jwst_fine_guide") => los_examples(),
        ("energy", "tokamak_vde_mast") => tokamak_vde_mast_examples(),
        ("materials", "fingerpad_p02_glass") => fingerpad_examples(),
        ("energy", "accumulator_sbo450") => accumulator_sbo450_examples(),
        _ => unreachable!("descriptor pair"),
    }
}

/// The key of a worked request a card turns away by name before computing.
pub(crate) const EXAMPLE_REFUSED: &str = "example_refused";

/// An object with `key` set to `value` just after `after`, the other keys in
/// their order.
fn with_after(body: Value, after: &str, key: &str, value: Value) -> Value {
    let Value::Object(map) = body else { return body };
    let mut out = Map::new();
    for (name, item) in map {
        let here = name == after;
        out.insert(name, item);
        if here {
            out.insert(key.to_string(), value.clone());
        }
    }
    Value::Object(out)
}

/// The worked examples step at the card's own ceiling.
fn card_dt(gate: Option<f64>) -> Value {
    exact_value(gate.expect("specimen gate"))
}

fn scalar_value(value: f64) -> Value {
    exact_value(value)
}

fn vec3_value(value: [f64; 3]) -> Value {
    Value::Array(value.into_iter().map(exact_value).collect())
}

fn obj(entries: &[(&str, Value)]) -> Value {
    let mut map = Map::new();
    for (key, value) in entries {
        map.insert((*key).to_string(), value.clone());
    }
    Value::Object(map)
}

fn pair_examples(
    domain: &str,
    card: &str,
    phase: Value,
    load: Value,
    dt: Value,
) -> (Value, Value) {
    pair_examples_of(domain, card, phase, load, dt, 4)
}

/// One step, and a run of `steps` steps under one held load.
fn pair_examples_of(
    domain: &str,
    card: &str,
    phase: Value,
    load: Value,
    dt: Value,
    steps: u64,
) -> (Value, Value) {
    pair_examples_recorded(domain, card, phase, load, dt, steps, 1)
}

/// One step, and a run of `steps` steps that records every `record_every`.
fn pair_examples_recorded(
    domain: &str,
    card: &str,
    phase: Value,
    load: Value,
    dt: Value,
    steps: u64,
    record_every: u64,
) -> (Value, Value) {
    let mut parameters = phase.as_object().expect("phase").clone();
    if let Some(load_map) = load.as_object() {
        for (key, value) in load_map {
            parameters.insert(key.clone(), value.clone());
        }
    }
    parameters.insert("dt_s".to_string(), dt.clone());
    let step = obj(&[
        ("domain", Value::String(domain.to_string())),
        ("card", Value::String(card.to_string())),
        ("parameters", Value::Object(parameters)),
    ]);
    let segment = obj(&[("steps", Value::Number(steps.into())), ("load", load)]);
    let run = obj(&[
        ("domain", Value::String(domain.to_string())),
        ("card", Value::String(card.to_string())),
        ("initial", phase),
        ("dt_s", dt),
        ("segments", Value::Array(vec![segment])),
        ("record_every", Value::Number(record_every.into())),
    ]);
    (step, run)
}

fn ti64_examples() -> (Value, Value) {
    let cell = ti64_lattice::Ti64LatticeCell::annealed_column();
    let state = ti64_lattice::Ti64LatticeState::try_rest(&cell).expect("ti64 rest");
    pair_examples(
        "materials",
        "ti64_lattice",
        obj(&[
            ("position_m", vec3_value(state.position_m)),
            ("velocity_m_s", vec3_value(state.velocity_m_s)),
            ("start_t_s", scalar_value(state.t_s)),
            ("start_force_n", vec3_value(state.applied_force_n)),
            ("start_temperature_k", scalar_value(state.temperature_k)),
            ("start_energy_anchor_j", scalar_value(state.energy_anchor_j)),
            ("start_boundary_work_j", scalar_value(state.boundary_work_j)),
        ]),
        obj(&[
            ("force_n", vec3_value(state.applied_force_n)),
            ("torque_nm", vec3_value(state.torque_nm)),
            ("temperature_k", scalar_value(state.temperature_k)),
        ]),
        card_dt(cell.gate_dt_s()),
    )
}

/// The load of the fuse link's worked examples: the node at its catalogue
/// 48 V, carrying 10 A from the feeder into the load.
const GRID_EXAMPLE_LOAD_A: f64 = 10.0;

/// The steps of the fuse link's worked run: 10 ms at the card's own step of
/// 1 microsecond, in which the link warms from 293.15 K by about 12.6 K and
/// stays far from its melting point.
const GRID_EXAMPLE_STEPS: u64 = 10_000;

fn grid_examples() -> (Value, Value) {
    let node = grid_node::GridNode::copper_fuse_link();
    let state = grid_node::GridNodeState::try_rest(&node).expect("grid rest");
    pair_examples_recorded(
        "energy",
        "grid_node",
        obj(&[
            ("current_a", scalar_value(GRID_EXAMPLE_LOAD_A)),
            ("voltage_v", scalar_value(grid_node::CATALOG_VOLTAGE_V)),
            ("temperature_k", scalar_value(state.temperature_k)),
        ]),
        obj(&[
            ("upstream_voltage_v", scalar_value(grid_node::CATALOG_VOLTAGE_V)),
            ("load_current_a", scalar_value(GRID_EXAMPLE_LOAD_A)),
            ("ambient_k", scalar_value(state.ambient_k)),
        ]),
        card_dt(node.gate_dt_s()),
        GRID_EXAMPLE_STEPS,
        GRID_EXAMPLE_STEPS / 10,
    )
}

/// A worked request a card turns away by name before computing, with the
/// one sentence it is answered with: an error, not a result, and nothing
/// sealed. The fuse link's is its worked run started at 1400 K, above the
/// melting point of copper.
pub(crate) fn example_refused(domain: &str, card: &str) -> Option<Value> {
    if (domain, card) != ("energy", "grid_node") {
        return None;
    }
    let (_, mut run) = grid_examples();
    run["initial"]["temperature_k"] = exact_value(1_400.0);
    let answered = match crate::trajectory::prepare(run.clone()) {
        Err(refused) => refused.message().to_string(),
        Ok(_) => unreachable!("a start above the melting point of copper is refused"),
    };
    Some(json!({
        "run_trajectory": run,
        "answered": answered,
        "says": "A start the card cannot take is turned away by name before anything is computed: the answer is this one sentence, an error and not a result, and no state is sealed.",
    }))
}

fn swing_examples() -> (Value, Value) {
    let machine = swing::SwingMachine::synchronous_60hz();
    let state = swing::SwingState::try_rest(&machine).expect("swing rest");
    pair_examples(
        "energy",
        "swing",
        obj(&[
            ("delta_rad", scalar_value(state.delta_rad)),
            ("delta_omega_rad_s", scalar_value(state.delta_omega_rad_s)),
            ("pll_err_rad", scalar_value(state.pll_err_rad)),
        ]),
        obj(&[("p_mech_pu", scalar_value(state.p_mech_pu))]),
        card_dt(machine.gate_dt_s()),
    )
}

fn battery_examples() -> (Value, Value) {
    let pack = battery_sag::LiIonPack::pack_12s();
    let state = battery_sag::PackState::try_full(&pack).expect("full pack");
    pair_examples(
        "energy",
        "battery_sag",
        obj(&[("soc", scalar_value(state.soc))]),
        obj(&[("current_a", scalar_value(state.current_a))]),
        card_dt(pack.gate_dt_s()),
    )
}

fn cell_examples() -> (Value, Value) {
    let specimen = cell_18650pf::Cell18650Pf::cell();
    let state = cell_18650pf::CellState::try_full(&specimen).expect("full cell");
    pair_examples(
        "energy",
        "cell_18650pf",
        obj(&[
            ("soc", scalar_value(state.soc)),
            ("held_current_a", scalar_value(2.9)),
        ]),
        obj(&[("current_a", scalar_value(2.9))]),
        card_dt(specimen.gate_dt_s()),
    )
}

fn thermal_examples() -> (Value, Value) {
    let node = thermal::ThermalNode::avionics_cold_plate();
    let state = thermal::ThermalNodeState::try_rest(&node).expect("thermal rest");
    pair_examples(
        "energy",
        "thermal",
        obj(&[("temperature_k", scalar_value(state.temperature_k))]),
        obj(&[
            ("heat_power_w", scalar_value(state.heat_power_w)),
            ("ambient_k", scalar_value(state.ambient_k)),
        ]),
        card_dt(node.gate_dt_s()),
    )
}

fn resonance_examples() -> (Value, Value) {
    let oscillator = resonance::DynamicOscillator::mount_10hz();
    let state = resonance::OscillatorState::try_rest(&oscillator).expect("resonance rest");
    pair_examples(
        "signal",
        "resonance",
        obj(&[
            ("displacement_m", scalar_value(state.displacement_m)),
            ("velocity_m_s", scalar_value(state.velocity_m_s)),
            ("t_s", scalar_value(state.t_s)),
            ("energy_anchor_j", scalar_value(state.energy_anchor_j)),
            ("boundary_work_j", scalar_value(state.boundary_work_j)),
        ]),
        obj(&[("accel_m_s2", scalar_value(state.accel_m_s2))]),
        card_dt(oscillator.gate_dt_s()),
    )
}

fn tesseract_examples() -> (Value, Value) {
    let resonator = tesseract::Tesseract::drive_mode();
    let state = tesseract::TesseractState::try_rest(&resonator).expect("tesseract rest");
    let load = tesseract::TesseractLoad::default();
    pair_examples(
        "signal",
        "tesseract",
        obj(&[
            ("displacement_m", scalar_value(state.displacement_m)),
            ("velocity_m_s", scalar_value(state.velocity_m_s)),
            ("energy_defect_j", scalar_value(state.energy_defect_j)),
        ]),
        obj(&[
            ("accel_m_s2", scalar_value(load.accel_m_s2)),
            ("omega_ext_rad_s", scalar_value(load.omega_ext_rad_s)),
            ("sense_velocity_m_s", scalar_value(load.sense_velocity_m_s)),
        ]),
        card_dt(resonator.gate_dt_s()),
    )
}

fn hydraulic_examples() -> (Value, Value) {
    let column = hydraulic_fluid::FluidColumn::mineral_oil();
    let state = hydraulic_fluid::FluidState::try_rest(&column).expect("hydraulic rest");
    pair_examples(
        "materials",
        "hydraulic_fluid",
        obj(&[("pressure_pa", scalar_value(state.pressure_pa))]),
        obj(&[("flow_m3_s", scalar_value(state.flow_m3_s))]),
        card_dt(column.gate_dt_s()),
    )
}

fn cable_examples() -> (Value, Value) {
    let tendon = cable_stretch::Tendon::dyneema();
    let state = cable_stretch::TendonState::try_rest(&tendon).expect("cable rest");
    pair_examples(
        "materials",
        "cable_stretch",
        obj(&[
            ("elongation_m", scalar_value(state.elongation_m)),
            ("velocity_m_s", scalar_value(state.velocity_m_s)),
        ]),
        obj(&[("force_n", scalar_value(state.force_n))]),
        card_dt(tendon.gate_dt_s()),
    )
}

/// One step at the card's ceiling from 25 degC with 2.0 printed dollars on the
/// table of core 217, and a run of the same pulse through its power peak.
fn reactor_triga_examples() -> (Value, Value) {
    let reactor = reactor_triga_jsi::ReactorTrigaJsi::core_56_pulse_mode();
    pair_examples_recorded(
        "energy",
        "reactor_triga_jsi",
        obj(&[
            ("start_temperature_k", scalar_value(298.15)),
            ("rod_table_core", scalar_value(reactor_triga_jsi::ROD_TABLE_CORE[1])),
        ]),
        obj(&[
            ("reactivity_inserted_dollar", scalar_value(2.0)),
            ("rods_dropped", scalar_value(0.0)),
        ]),
        card_dt(reactor.gate_dt_s()),
        REACTOR_TRIGA_EXAMPLE_STEPS,
        100,
    )
}

/// The steps the worked run of the JSI TRIGA reactor asks for: 0.38 s, past
/// the power peak of a 2.0 dollar pulse.
pub(crate) const REACTOR_TRIGA_EXAMPLE_STEPS: u64 = 4_000;

/// The start of the compared run, 0.150 s before the start of the held-back
/// ring-down on the model's own trajectory: fixed by one number from the
/// opened data, the first-period amplitude. `validation_protocol.md`, section
/// 9, and `card_spec_draft.md`, section 8, test 2, of the research record.
/// Its envelope, 15.786 mm, is the largest the compared run held, under the
/// card's ceiling.
#[cfg(test)]
pub(crate) const CANTILEVER_VALIDATION_START_DEFLECTION_M: f64 = -1.526315579382e-2;
#[cfg(test)]
pub(crate) const CANTILEVER_VALIDATION_START_VELOCITY_M_S: f64 = 2.099361936006e-1;

/// The start of the scored ring-down: the compared run's state after 595
/// steps of 0.25 ms, 0.14875 s, the first step after the reading's start
/// crossing. Envelope 14.257 mm. The card's run from here gives the compared
/// run's deflections and speeds bit for bit.
pub(crate) const CANTILEVER_EXAMPLE_START_DEFLECTION_M: f64 = -0.00104914043915228;
pub(crate) const CANTILEVER_EXAMPLE_START_VELOCITY_M_S: f64 = 0.7408030982848752;

/// One step at the card's ceiling from the start of the scored ring-down, and
/// the first second from there: 4,000 steps, a frame every tenth step, 400
/// frames at 400 per second.
fn cantilever_examples() -> (Value, Value) {
    let beam = cantilever_eth::CantileverEth::first_bending_mode();
    pair_examples_recorded(
        "signal",
        "cantilever_eth",
        obj(&[
            (
                "deflection_m",
                scalar_value(CANTILEVER_EXAMPLE_START_DEFLECTION_M),
            ),
            (
                "velocity_m_s",
                scalar_value(CANTILEVER_EXAMPLE_START_VELOCITY_M_S),
            ),
        ]),
        obj(&[]),
        card_dt(beam.gate_dt_s()),
        4_000,
        10,
    )
}

/// The lock point and the first eight frames of the card's reference run:
/// frames 469 to 476, counted from 0, of
/// `jw01163017001_gs-fg_2022092164131_cal.fits`, programme 1163, visit 017,
/// 2022-04-02, a file of the fitted set. The file's first centroid is 4.5 mas
/// from the lock point and more than four stated standard deviations from
/// where a filter that knows nothing expects it, and the card refuses it; the
/// run starts 469 frames, 30.016 s, later.
pub(crate) const LOS_EXAMPLE_LOCK_ARCSEC: [f64; 2] = [17.3039684, -15.2342768];
pub(crate) const LOS_EXAMPLE_QUIET_BEFORE_START_S: f64 = 30.016;
pub(crate) const LOS_EXAMPLE_FRAMES: [[f64; 4]; 8] = [
    [1.0, 17.3038845, -15.2345047, 553666.4],
    [1.0, 17.3043156, -15.234498, 548737.25],
    [1.0, 17.3045425, -15.234539, 549291.063],
    [1.0, 17.3039627, -15.23488, 553444.8],
    [1.0, 17.303978, -15.2349653, 547832.7],
    [1.0, 17.3033333, -15.2353725, 547648.063],
    [1.0, 17.3037148, -15.2351408, 547629.563],
    [1.0, 17.303875, -15.2350893, 546318.9],
];

/// One frame of the table as a load.
pub(crate) fn los_example_load(row: [f64; 4]) -> Value {
    obj(&[
        ("measured", scalar_value(row[0])),
        ("centroid_x_arcsec", scalar_value(row[1])),
        ("centroid_y_arcsec", scalar_value(row[2])),
        ("counts_per_s", scalar_value(row[3])),
    ])
}

/// The first frame as one step, and the first eight frames as a run of eight
/// segments of one step each, every frame recorded.
fn los_examples() -> (Value, Value) {
    let phase = obj(&[
        ("lock_x_arcsec", scalar_value(LOS_EXAMPLE_LOCK_ARCSEC[0])),
        ("lock_y_arcsec", scalar_value(LOS_EXAMPLE_LOCK_ARCSEC[1])),
        (
            "quiet_before_start_s",
            scalar_value(LOS_EXAMPLE_QUIET_BEFORE_START_S),
        ),
    ]);
    let dt = scalar_value(los_jwst_fine_guide::FRAME_S);
    let (step, _) = pair_examples_recorded(
        "aerospace",
        "los_jwst_fine_guide",
        phase.clone(),
        los_example_load(LOS_EXAMPLE_FRAMES[0]),
        dt.clone(),
        1,
        1,
    );
    let segments: Vec<Value> = LOS_EXAMPLE_FRAMES
        .iter()
        .map(|row| obj(&[("steps", Value::Number(1.into())), ("load", los_example_load(*row))]))
        .collect();
    let run = obj(&[
        ("domain", Value::String("aerospace".to_string())),
        ("card", Value::String("los_jwst_fine_guide".to_string())),
        ("initial", phase),
        ("dt_s", dt),
        ("segments", Value::Array(segments)),
        ("record_every", Value::Number(1.into())),
    ]);
    (step, run)
}

/// One second of a discharge at the rated current, 4.5 A, from a full and
/// rested cell m80 at the capacity of its first tests, with its case at
/// 25 degC; and ten minutes of the same with a frame every minute.
/// One step at the card's ceiling from the smallest radius, and the whole
/// growth from there: 1,600 steps of 0.25 s, a frame every hundredth step. The
/// crater stops on step 1,597.
/// The start of the crater's worked examples, 1.75 m: the card's floor until
/// engine 0.9 and the start of the research record's reference run.
pub(crate) const CRATER_EXAMPLE_START_M: f64 = 1.75;

fn crater_examples() -> (Value, Value) {
    let crater = crater_ryugu_sci::CraterRyuguSci::sci_impact();
    pair_examples_recorded(
        "aerospace",
        "crater_ryugu_sci",
        obj(&[("radius_m", scalar_value(CRATER_EXAMPLE_START_M))]),
        obj(&[]),
        card_dt(crater.gate_dt_s()),
        1_600,
        100,
    )
}

/// One wet minute at 12 mm/h from a dry link at the median dry level of the
/// fitted events, and ten such minutes.
fn link_examples() -> (Value, Value) {
    let link = link_gothenburg_10010::LinkGothenburg10010::sublink_587();
    pair_examples_of(
        "signal",
        "link_gothenburg_10010",
        obj(&[
            ("dry_path_loss_db", scalar_value(LINK_EXAMPLE_DRY_PATH_LOSS_DB)),
            ("antenna_water_db", scalar_value(0.0)),
            ("gauge_minute_elapsed_s", scalar_value(0.0)),
            ("gauge_minute_rain_mm_h", scalar_value(0.0)),
        ]),
        obj(&[("rain_rate_mm_h", scalar_value(12.0))]),
        // The gauge's minute, the step the constants were fitted and compared at.
        card_dt(Some(link.gauge_step_s)),
        10,
    )
}

/// The stress of the card's worked examples, MPa: the stress no fitted test has.
pub(crate) const CREEP_EXAMPLE_STRESS_MPA: f64 = 200.0;

/// One step at the card's ceiling from 0.1 percent creep extension at
/// 200 MPa, the stress no fitted test has, and the next ten hours of that
/// test: 720 steps, a frame every 72nd step, ten frames an hour apart.
fn creep_examples() -> (Value, Value) {
    let alloy = creep_cmsx6::CreepCmsx6::at_980_c();
    pair_examples_recorded(
        "materials",
        "creep_cmsx6",
        obj(&[(
            "creep_extension_percent",
            scalar_value(creep_cmsx6::CREEP_EXTENSION_MIN_PERCENT),
        )]),
        obj(&[("stress_mpa", scalar_value(CREEP_EXAMPLE_STRESS_MPA))]),
        card_dt(alloy.gate_dt_s()),
        720,
        72,
    )
}

/// A push of 0.03 m/s from the fitted condition, 60 bar with the charge of a
/// 40 bar precharge typed to four figures and no motion before it: one step
/// of 2.5 ms, and forty such steps, 3 mm of travel.
fn accumulator_sbo450_examples() -> (Value, Value) {
    pair_examples_of(
        "energy",
        "accumulator_sbo450",
        obj(&[
            ("gas_mass_kg", scalar_value(0.02747)),
            ("gas_pressure_pa", scalar_value(6.0e6)),
            ("gas_temperature_k", scalar_value(293.15)),
            ("wall_temperature_k", scalar_value(293.55)),
            ("work_moved_j", scalar_value(0.0)),
            ("heat_moved_j", scalar_value(0.0)),
        ]),
        obj(&[("piston_speed_m_s", scalar_value(0.03))]),
        scalar_value(0.0025),
        40,
    )
}

/// One slip stage of 21 s at 12 percent traction, 5 kN and 2.5 bar, a
/// held-back condition, and four such stages: 84 s and 5.04 m of the track.
fn tire_eve_r19_ice_examples() -> (Value, Value) {
    pair_examples(
        "materials",
        "tire_eve_r19_ice",
        obj(&[
            ("distance_m", scalar_value(0.0)),
            ("slip_held_s", scalar_value(tire_eve_r19_ice::SLIP_SETTLE_S)),
        ]),
        obj(&[
            ("normal_load_n", scalar_value(5_000.0)),
            ("inflation_pressure_pa", scalar_value(250_000.0)),
            ("slip_ratio", scalar_value(0.12)),
            ("carriage_speed_m_s", scalar_value(0.06)),
        ]),
        // Four steps of 4 s: the compared hold from 5 s to 21 s.
        scalar_value(4.0),
    )
}

/// One step at the card's ceiling from zero twist at 2 N and 20 degrees a
/// second, and the twist to 75 degrees: 375 steps, a frame every 25th step,
/// every five degrees.
fn fingerpad_examples() -> (Value, Value) {
    let pad = fingerpad_p02_glass::FingerPadP02Glass::on_glass();
    pair_examples_recorded(
        "materials",
        "fingerpad_p02_glass",
        obj(&[("twist_deg", scalar_value(0.0))]),
        obj(&[
            ("normal_force_n", scalar_value(2.0)),
            ("twist_rate_deg_s", scalar_value(20.0)),
        ]),
        card_dt(pad.gate_dt_s()),
        375,
        25,
    )
}

/// One day from landing, and thirty daily steps from landing. A step at the
/// card's ceiling leaves the card at the 58th step, so the examples do not use it.
fn generator_examples() -> (Value, Value) {
    let unit = generator_mmrtg_f1::GeneratorMmrtgF1::curiosity();
    let state = generator_mmrtg_f1::GeneratorState::try_landing(&unit).expect("landing");
    pair_examples_of(
        "energy",
        "generator_mmrtg_f1",
        obj(&[("mission_time_s", scalar_value(state.t_s))]),
        obj(&[]),
        scalar_value(86_400.0),
        30,
    )
}

/// One step from entry interface, and the first 20 s of the entry at the
/// card's ceiling, one frame every half second.
fn capsule_examples() -> (Value, Value) {
    let specimen = capsule_stardust::CapsuleStardust::sample_return_capsule();
    let state =
        capsule_stardust::CapsuleState::try_entry_interface(&specimen).expect("entry interface");
    pair_examples_recorded(
        "aerospace",
        "capsule_stardust",
        obj(&[
            ("radius_m", scalar_value(state.radius_m)),
            ("radial_speed_m_s", scalar_value(state.radial_speed_m_s)),
            (
                "transverse_speed_m_s",
                scalar_value(state.transverse_speed_m_s),
            ),
        ]),
        obj(&[]),
        card_dt(specimen.gate_dt_s()),
        400,
        10,
    )
}

/// The lift over drag measured on the first 150 s of the flight, in the
/// flight plane, as the research record's analysis of the flight gives it.
pub(crate) const CAPSULE_INSIGHT_FLIGHT_LIFT_OVER_DRAG: f64 = -0.031;

/// One step at the ceiling from the locked entry state with the flight's
/// lift, and the first 200 s of that entry: 2,000 steps, a frame every tenth.
fn capsule_insight_examples() -> (Value, Value) {
    let specimen = capsule_insight::CapsuleInsight::entry_capsule();
    let state =
        capsule_insight::CapsuleState::try_entry_interface(&specimen).expect("entry state");
    pair_examples_recorded(
        "aerospace",
        "capsule_insight",
        obj(&[
            ("radius_m", scalar_value(state.radius_m)),
            ("radial_speed_m_s", scalar_value(state.radial_speed_m_s)),
            (
                "transverse_speed_m_s",
                scalar_value(state.transverse_speed_m_s),
            ),
        ]),
        obj(&[(
            "lift_over_drag",
            scalar_value(CAPSULE_INSIGHT_FLIGHT_LIFT_OVER_DRAG),
        )]),
        card_dt(specimen.gate_dt_s()),
        2_000,
        10,
    )
}

/// One step at the ceiling from the example orbit, and one revolution of it:
/// 1,356 steps of 10 s, a frame every 36.
fn satellite_examples() -> (Value, Value) {
    let specimen = satellite_lageos1::SatelliteLageos1::lageos1();
    let state = satellite_example_state();
    pair_examples_recorded(
        "aerospace",
        "satellite_lageos1",
        obj(&[
            ("epoch_utc_s", scalar_value(state.epoch_utc_s())),
            ("forecast_elapsed_s", scalar_value(state.forecast_elapsed_s())),
            ("position_m", vec3_value(state.position_m)),
            ("velocity_m_s", vec3_value(state.velocity_m_s)),
        ]),
        obj(&[]),
        card_dt(specimen.gate_dt_s()),
        1_356,
        36,
    )
}

/// The twenty-six vectors of a state, as a request carries them.
pub(crate) fn solar_system_phase(state: &solar_system::SolarSystemState) -> Value {
    let mut map = Map::new();
    for body in 0..solar_system::BODIES {
        map.insert(SOLAR_SYSTEM_POSITION_KEYS[body].to_string(), vec3_value(state.position_m[body]));
        map.insert(SOLAR_SYSTEM_VELOCITY_KEYS[body].to_string(), vec3_value(state.velocity_m_s[body]));
    }
    map.insert("energy_anchor".to_string(), scalar_value(state.energy_anchor));
    map.insert("booked_work".to_string(), scalar_value(state.booked_work));
    map.insert("angular_momentum_anchor".to_string(), vec3_value(state.angular_momentum_anchor));
    map.insert("booked_torque".to_string(), vec3_value(state.booked_torque));
    Value::Object(map)
}

/// One step at the ceiling from the 1999 state, and one day of it: 24 steps
/// of 3600 s, a frame every fourth.
fn solar_system_examples() -> (Value, Value) {
    let specimen = solar_system::SolarSystem::de440();
    let state = solar_system_example_state();
    pair_examples_recorded(
        "aerospace",
        "solar_system",
        solar_system_phase(&state),
        obj(&[]),
        card_dt(specimen.gate_dt_s()),
        24,
        4,
    )
}

pub(crate) fn reject_meaning(card: &str, name: &str) -> Option<&'static str> {
    Some(match (card, name) {
        ("grid_gb", which) if card_grid_gb_twin_descriptor::meaning(which).is_some() => return card_grid_gb_twin_descriptor::meaning(which),
        ("tokamak_jet_dte1", which) if card_tokamak_jet_dte1_twin_descriptor::meaning(which).is_some() => return card_tokamak_jet_dte1_twin_descriptor::meaning(which),
        ("runaway_18650_lco", which) if card_runaway_18650_lco_twin_descriptor::meaning(which).is_some() => return card_runaway_18650_lco_twin_descriptor::meaning(which),
        ("reactor_msre_pump", which) if card_reactor_msre_pump_descriptor::meaning(which).is_some() => return card_reactor_msre_pump_descriptor::meaning(which),
        ("reactor_spert3", which) if card_reactor_spert3_twin_descriptor::meaning(which).is_some() => return card_reactor_spert3_twin_descriptor::meaning(which),
        ("cell_2170_model3", which) if card_cell_2170_twin_descriptor::meaning(which).is_some() => return card_cell_2170_twin_descriptor::meaning(which),
        ("reactor_spert3_ccore", which) if card_reactor_spert3_ccore_plate_descriptor::meaning(which).is_some() => return card_reactor_spert3_ccore_plate_descriptor::meaning(which),
        ("generator_mmrtg_seebeck", which) if card_generator_mmrtg_seebeck_descriptor::meaning(which).is_some() => return card_generator_mmrtg_seebeck_descriptor::meaning(which),
        ("momentum_jwst_dihedral", which) if card_momentum_jwst_dihedral_descriptor::meaning(which).is_some() => return card_momentum_jwst_dihedral_descriptor::meaning(which),
        ("grasp_lift_glass_4tu", which) if card_grasp_lift_glass_4tu_descriptor::meaning(which).is_some() => return card_grasp_lift_glass_4tu_descriptor::meaning(which),
        ("momentum_jwst_srp", which) if card_momentum_jwst_srp_descriptor::meaning(which).is_some() => return card_momentum_jwst_srp_descriptor::meaning(which),
        ("tokamak_vde_mast", _) if tokamak_vde_mast_reject_meaning(name).is_some() => {
            return tokamak_vde_mast_reject_meaning(name);
        }
        ("solar_system", "NonFiniteInput") => "`dt` was NaN or infinite.",
        ("solar_system", "NonFiniteState") => {
            "The incoming state, or the trial step, was non-finite."
        }
        ("solar_system", "NonPositiveDt") => "`dt` was zero or negative.",
        ("solar_system", "InvalidSystem") => "A recorded constant is not this card's.",
        ("solar_system", "DistanceOutsideCard") => {
            "A body's distance from its primary, incoming or landing, is outside that body's band."
        }
        ("solar_system", "OrbitOutsideCard") => {
            "The semi-major axis of a body's orbit about its primary, incoming or landing, is outside that body's band, or the orbit is not closed."
        }
        ("solar_system", "BodiesCoincide") => {
            "Two bodies, incoming or landing, are at exactly one point, where the attraction between them has no value."
        }
        ("solar_system", "FrameOutsideCard") => {
            "The centre of mass of the thirteen bodies, incoming or landing, moves faster than 0.01 m/s."
        }
        ("solar_system", "AxesOutsideCard") => {
            "The orbit of the Earth-Moon barycentre about the Sun, incoming or landing, is not inclined 21 to 27 degrees to the z axis, as the ecliptic is to the ICRF equator: the axes are not those of the ICRF."
        }
        ("solar_system", "SpontaneousGeneration") => {
            "The incoming ledger, or the trial step, gained energy that the work of relativity and of the Earth's oblateness does not account for."
        }
        ("solar_system", "FirstLawBreach") => {
            "The incoming ledger, or the trial step, lost energy that the work of relativity and of the Earth's oblateness does not account for."
        }
        ("solar_system", "AngularMomentumBreach") => {
            "The incoming ledger, or the trial step, changed the angular momentum by more than the torque of relativity and of the Earth's oblateness accounts for."
        }
        ("ti64_lattice", "NonFiniteInput") => {
            "Force, torque, temperature, or `dt` was NaN or infinite."
        }
        ("ti64_lattice", "NonFiniteState") => "The incoming state was already non-finite.",
        ("ti64_lattice", "NonPositiveDt") => "`dt` was zero or negative.",
        ("ti64_lattice", "TemperatureOutsideCard") => {
            "Temperature is outside the annealed constant-CTE card."
        }
        ("ti64_lattice", "InvalidCell") => {
            "Length, area, hardening, or mass cannot support the card."
        }
        ("ti64_lattice", "YieldReached") => {
            "A ligament is past 880 MPa on the incoming state, at the instant the step's temperature is applied, at a turning point of the motion inside the step, or on the landing."
        }
        ("ti64_lattice", "SpontaneousGeneration") => {
            "Mechanical energy is above the anchor plus the booked boundary work by more than the limit."
        }
        ("ti64_lattice", "FirstLawBreach") => {
            "Mechanical energy is below the anchor plus the booked boundary work by more than the limit."
        }
        ("ti64_lattice", "StartNotWritten") => {
            "A handed-in start states what no run of the card writes: a clock below zero, boundary work booked at a clock of zero, or a ledger whose residual is already past the limit."
        }
        ("grid_node", "NonFiniteInput") => {
            "Voltage, current, ambient, or `dt` was NaN or infinite."
        }
        ("grid_node", "NonFiniteState") => "The incoming state, or the trial step, was non-finite.",
        ("grid_node", "NonPositiveDt") => "`dt` was zero or negative.",
        ("grid_node", "NegativeTemperature") => {
            "Temperature or ambient is below zero, or exactly zero."
        }
        ("grid_node", "TemperatureOutsideCard") => {
            "Temperature or ambient is outside the solid copper card."
        }
        ("grid_node", "InvalidNode") => {
            "Inductance, capacitance, resistance, or thermal capacity cannot support the card."
        }
        ("grid_node", "CopperMelt") => {
            "The trial copper temperature reached the melt point.\n\nThe solid heat capacity does not cross the phase change. The step is refused and nothing is written."
        }
        ("grid_node", "SpontaneousGeneration") => "The trial step created energy from nothing.",
        ("grid_node", "FirstLawBreach") => "The trial step destroyed energy.",
        ("grid_node", "CurrentOutsideCard") => {
            "The trial feeder current is past the card's current bound, the current whose energy in the feeder inductance equals the heat that takes the link from 293.15 K to its melting point."
        }
        ("grid_node", "VoltageOutsideCard") => {
            "The trial node voltage is past the card's voltage bound, the voltage whose energy in the shunt capacitance equals the heat that takes the link from 293.15 K to its melting point."
        }
        ("swing", "NonFiniteInput") => "Mechanical power or `dt` was NaN or infinite.",
        ("swing", "NonFiniteState") => "The incoming state, or the trial step, was non-finite.",
        ("swing", "NonPositiveDt") => "`dt` was zero or negative.",
        ("swing", "InvalidMachine") => {
            "Inertia, damping, ceiling, or the PLL gain is not this machine."
        }
        ("swing", "LossOfSynchronism") => "The landed rotor angle is at or beyond ±π/2.",
        ("swing", "SpeedDeviationOutsideCard") => {
            "Rotor speed deviation, incoming or landing, is beyond ±6π rad/s, 3 Hz off the bus."
        }
        ("swing", "DispatchOutsideCard") => {
            "Mechanical dispatch, held or commanded, is beyond ±2 pu, twice the machine's rating."
        }
        ("swing", "SpontaneousGeneration") => {
            "The trial step created transient energy from nothing."
        }
        ("swing", "FirstLawBreach") => "The trial step destroyed transient energy.",
        ("battery_sag", "NonFiniteInput") => "Current or `dt` was NaN or infinite.",
        ("battery_sag", "NonFiniteState") => {
            "The incoming state, or the trial step, was non-finite."
        }
        ("battery_sag", "NonPositiveDt") => "`dt` was zero or negative.",
        ("battery_sag", "InvalidPack") => {
            "Resistance, capacity, or the voltage anchors are not this pack."
        }
        ("battery_sag", "CurrentAboveNyquist") => "`|I| dt` is past `(V_full − V_empty) / R0`.",
        ("battery_sag", "UndervoltageCutoff") => "Terminal voltage is at or below the cutoff.",
        ("battery_sag", "OvervoltageCutoff") => {
            "Terminal voltage is above the 50.4 V charging limit, 12S × 4.2 V. A full pack (soc 1) refuses every charging current, however small, before the step is taken."
        }
        ("battery_sag", "StateOfChargeOutsideCard") => "State of charge left `[0, 1]`.",
        ("battery_sag", "SpontaneousGeneration") => {
            "The trial step created chemical energy from nothing."
        }
        ("battery_sag", "FirstLawBreach") => "The trial step destroyed chemical energy.",
        ("cell_18650pf", "NonFiniteInput") => "Current or `dt` was NaN or infinite.",
        ("cell_18650pf", "NonFiniteState") => {
            "The incoming state, or the trial step, was non-finite."
        }
        ("cell_18650pf", "NonPositiveDt") => "`dt` was zero or negative.",
        ("cell_18650pf", "InvalidCell") => "A recorded constant or table entry is not this cell.",
        ("cell_18650pf", "CurrentNotDischarging") => {
            "Current is zero or negative, a rest or a charge: the card is a held discharge, with no charging limit at 4.2 V and no relaxation after a change of load."
        }
        ("cell_18650pf", "CurrentChangedDuringDischarge") => {
            "The current is more than 0.83 mA from the held current, set by the run's first step or carried in from the discharge it continues: the card is of one held current."
        }
        ("cell_18650pf", "StateOfChargeOutsideCard") => {
            "State of charge, incoming or landing, is outside `[0.03, 1]`."
        }
        ("cell_18650pf", "UndervoltageCutoff") => "Landing terminal voltage is at or below 2.5 V.",
        ("cell_18650pf", "SpontaneousGeneration") => {
            "The trial step created chemical energy from nothing."
        }
        ("cell_18650pf", "FirstLawBreach") => "The trial step destroyed chemical energy.",
        ("thermal", "NonFiniteInput") => "Heater power, ambient, or `dt` was NaN or infinite.",
        ("thermal", "NonFiniteState") => "The incoming state, or the trial step, was non-finite.",
        ("thermal", "NonPositiveDt") => "`dt` was zero or negative.",
        ("thermal", "NegativeTemperature") => "A step would land the plate at or below zero kelvin.",
        ("thermal", "TemperatureOutsideCard") => {
            "Plate or ambient temperature is outside the industrial card."
        }
        ("thermal", "InvalidNode") => {
            "Capacitance, resistance, or the card temperatures are not this plate."
        }
        ("thermal", "SpontaneousGeneration") => "The trial step created heat from nothing.",
        ("thermal", "FirstLawBreach") => "The trial step destroyed heat.",
        ("resonance", "NonFiniteInput") => "Acceleration or `dt` was NaN or infinite.",
        ("resonance", "NonFiniteState") => "The incoming state, or the trial step, was non-finite.",
        ("resonance", "NonPositiveDt") => "`dt` was zero or negative.",
        ("resonance", "InvalidOscillator") => {
            "Frequency, damping, mass, or stroke is not this mount."
        }
        ("resonance", "ForceAboveNyquist") => "`|a| dt` is past `ω L`.",
        ("resonance", "EnergyAboveCard") => {
            "Mechanical energy, incoming or landing, is above that of a free ring of twice the stroke, four stroke energies: a size, reached by a displacement or a velocity of either sign."
        }
        ("resonance", "StartTimeOutsideCard") => {
            "The stated `t_s` of a started state is below zero or above 1024 s."
        }
        ("resonance", "LedgerNotOfThisState") => {
            "The stated ledger is not one the card could have written for this state: an anchor below zero or above four stroke energies, booked work at `t_s` 0, or `E − E_anchor − W` already past the limit."
        }
        ("resonance", "SpontaneousGeneration") => {
            "Mechanical energy is above the anchor plus the booked work by more than the limit: a wrong step, or the step method's own error summed over a long driven run."
        }
        ("resonance", "FirstLawBreach") => {
            "Mechanical energy is below the anchor plus the booked work by more than the limit: a wrong step, or the step method's own error summed over a long driven run."
        }
        ("tesseract", "NonFiniteInput") => {
            "Acceleration, sense speed, rate, or `dt` was NaN or infinite."
        }
        ("tesseract", "NonFiniteState") => "The incoming state, or a trial stage, was non-finite.",
        ("tesseract", "NonPositiveDt") => "`dt` was zero or negative.",
        ("tesseract", "InvalidResonator") => {
            "`α`, `ζ`, `ω`, mass, or the amplitude limit is not this drive mode."
        }
        ("tesseract", "ForceAboveNyquist") => "`|a| dt` is past `ω L`.",
        ("tesseract", "AmplitudeOutsideCard") => {
            "The displacement or the velocity, incoming or after the trial step, is outside the card's inclusive bounds."
        }
        ("tesseract", "SpontaneousGeneration") => {
            "The trial step created mechanical energy from nothing."
        }
        ("tesseract", "FirstLawBreach") => "The trial step destroyed mechanical energy.",
        ("hydraulic_fluid", "NonFiniteInput") => "Flow or `dt` was NaN or infinite.",
        ("hydraulic_fluid", "NonFiniteState") => {
            "The incoming state, or the trial step, was non-finite."
        }
        ("hydraulic_fluid", "NonPositiveDt") => "`dt` was zero or negative.",
        ("hydraulic_fluid", "InvalidChamber") => {
            "Bulk modulus, density, bore, or the pressure anchors are not this column."
        }
        ("hydraulic_fluid", "FlowAboveNyquist") => "`|Q| dt` is past the 3 mm stroke volume.",
        ("hydraulic_fluid", "FlowAboveAcoustic") => {
            "`|Q|` is above the acoustic flow limit: the bore speed `|Q| / A` is above `(P_ceiling − P_ref) / (ρ c)`, 0.012 of the oil's wave speed, about 16.87 m/s, so a flow of 0.0331 m³/s. Above it the pressure front that starting the flow sends down the column is larger than the card's whole span, and one pressure for the whole column does not describe it."
        }
        ("hydraulic_fluid", "Cavitation") => "The trial pressure is below vapor pressure.",
        ("hydraulic_fluid", "PressureAboveCard") => {
            "Pressure, incoming or landing, is above the card's ceiling of 20.64 MPa over the charge, 6.0 mm of compression."
        }
        ("hydraulic_fluid", "SpontaneousGeneration") => {
            "The trial step created mass or elastic energy from nothing."
        }
        ("hydraulic_fluid", "FirstLawBreach") => "The trial step destroyed mass or elastic energy.",
        ("cable_stretch", "NonFiniteInput") => "Force or `dt` was NaN or infinite.",
        ("cable_stretch", "NonFiniteState") => {
            "The incoming state, or the trial step, was non-finite."
        }
        ("cable_stretch", "NonPositiveDt") => "`dt` was zero or negative.",
        ("cable_stretch", "InvalidCable") => {
            "Frequency, damping, mass, or ultimate tension is not this braid."
        }
        ("cable_stretch", "ForceAboveNyquist") => "`|F| dt / m` is past `ω x_ult`.",
        ("cable_stretch", "TensileFailure") => {
            "Elongation reached the ultimate elongation, or tension reached the ultimate tensile load. Each is tested at the start, at each RK4 stage and at the landing."
        }
        ("cable_stretch", "Slack") => {
            "Elongation or tension went negative. The braid is slack. Each is tested at the start, at each RK4 stage and at the landing."
        }
        ("cable_stretch", "SubnormalState") => {
            "The incoming state, or the landing, has a subnormal elongation or velocity. The door does not accept a subnormal number as a start, so that state is not sealed."
        }
        ("cable_stretch", "SpontaneousGeneration") => {
            "The trial step created mechanical energy from nothing."
        }
        ("cable_stretch", "FirstLawBreach") => "The trial step destroyed mechanical energy.",
        ("generator_mmrtg_f1", "NonFiniteInput") => "`dt` was NaN or infinite.",
        ("generator_mmrtg_f1", "NonFiniteState") => {
            "The incoming state, or the trial step, was non-finite."
        }
        ("generator_mmrtg_f1", "NonPositiveDt") => "`dt` was zero or negative.",
        ("generator_mmrtg_f1", "InvalidGenerator") => {
            "A recorded constant is not this generator's."
        }
        ("generator_mmrtg_f1", "MissionTimeBeforeLanding") => {
            "Mission time, incoming or landing, is before landing on Mars, where the card's thermal power and converter are anchored."
        }
        ("generator_mmrtg_f1", "DecayHeatBelowLedgerResolution") => {
            "The decay heat the fuel still holds, incoming or landing, is under the ledger's resolution, 1e-6 of the decay heat released from landing to mission day 4,087 (about 6.6e5 J, some 2,070 years after landing): a breach of that size could no longer be seen."
        }
        ("generator_mmrtg_f1", "SpontaneousGeneration") => {
            "The trial step created energy from nothing."
        }
        ("generator_mmrtg_f1", "FirstLawBreach") => "The trial step destroyed energy.",
        ("capsule_stardust", "NonFiniteInput") => "`dt` was NaN or infinite.",
        ("capsule_stardust", "NonFiniteState") => {
            "The incoming state, or the trial step, was non-finite."
        }
        ("capsule_stardust", "NonPositiveDt") => "`dt` was zero or negative.",
        ("capsule_stardust", "InvalidCapsule") => {
            "A recorded constant or table entry is not this capsule."
        }
        ("capsule_stardust", "AltitudeOutsideCard") => {
            "Altitude, incoming, at a stage of the step, or landing, is above 134.7 km, the top of the drag table read by altitude."
        }
        ("capsule_stardust", "BelowStandardSeaLevel") => {
            "Altitude, incoming, at a stage of the step, or landing, is below 0 m, the sea level of the 1976 standard's air and the ellipsoid under the path: the capsule is on or under the ground."
        }
        ("capsule_stardust", "MachOutsideCard") => {
            "Mach number, incoming, at a stage of the step, or landing, is below 0.16, the first entry of the drag table read by Mach number, or is above 40 below 68.96 km, where that table ends."
        }
        ("capsule_stardust", "AirSpeedOutsideCard") => {
            "Air speed, incoming, at a stage of the step, or landing, above 68.96 km is below 11900 m/s or above 12620 m/s, the highest speed the drag table by altitude was computed at."
        }
        ("capsule_stardust", "PastFlightDeployment") => {
            "The incoming state is under 31030 m and under Mach 1.23 at once: past the flown drogue deployment."
        }
        ("capsule_stardust", "SpontaneousGeneration") => {
            "The trial step gained energy that drag did not account for."
        }
        ("capsule_stardust", "FirstLawBreach") => {
            "The trial step lost more energy than drag accounts for."
        }
        ("capsule_insight", "NonFiniteInput") => {
            "`dt` or the lift over drag was NaN or infinite."
        }
        ("capsule_insight", "NonFiniteState") => {
            "The incoming state, or the trial step, was non-finite."
        }
        ("capsule_insight", "NonPositiveDt") => "`dt` was zero or negative.",
        ("capsule_insight", "InvalidCapsule") => {
            "A recorded constant or table entry is not this capsule."
        }
        ("capsule_insight", "LiftOutsideCard") => {
            "The lift over drag is above 0.08 or below −0.08."
        }
        ("capsule_insight", "AltitudeOutsideCard") => {
            "Altitude, incoming, at a stage of the step, or landing, is above 128 km."
        }
        ("capsule_insight", "BelowLandingSite") => {
            "Altitude, incoming, at a stage of the step, or landing, is below -2634.5 m, the landing site's radius of 3 393 079.5 m: the capsule is at or under the ground there."
        }
        ("capsule_insight", "MachOutsideCard") => {
            "Mach number, incoming, at a stage of the step, or landing, is below 0.9, the first entry of the drag table read by Mach number."
        }
        ("capsule_insight", "AirSpeedOutsideCard") => {
            "Air speed, incoming, at a stage of the step, or landing, is above 5850 m/s."
        }
        ("capsule_insight", "PastFlightDeployment") => {
            "The incoming state is under 6164.62 m and under 352.1169 m/s through the air at once: past the flown parachute deployment."
        }
        ("capsule_insight", "SpontaneousGeneration") => {
            "The trial step gained energy that drag did not account for."
        }
        ("capsule_insight", "FirstLawBreach") => {
            "The trial step lost more energy than drag accounts for."
        }
        ("satellite_lageos1", "NonFiniteInput") => "`dt` was NaN or infinite.",
        ("satellite_lageos1", "NonFiniteState") => {
            "The incoming state, or the trial step, was non-finite."
        }
        ("satellite_lageos1", "NonPositiveDt") => "`dt` was zero or negative.",
        ("satellite_lageos1", "InvalidSatellite") => {
            "A recorded constant or table entry is not this satellite and its Earth."
        }
        ("satellite_lageos1", "RadiusBelowEarthRadius") => {
            "Distance from the Earth's centre, incoming or landing, is below 6 378.1363 km, the Earth's equatorial radius, to which the field is scaled: the satellite is under the sphere of the Earth's equator, in the ground at the equator, and the card's field, the series of the Earth's field outside the Earth, is not the field there."
        }
        ("satellite_lageos1", "EpochOutsideLeapSecondSpan") => {
            "The time, incoming or landing, is before 2017-01-01 00:00 UTC, the last leap second, or after 2027-06-28 00:00 UTC, when the IERS leap-second file of Bulletin C 72 expires: outside that span TT − UTC is not the card's 69.184 s, or is not yet known. The landing by more than the rounding allowance of the running sum of the clock: 1.19e-7 s, and one part in 2^53 of the clock for each step taken at this step and two more."
        }
        ("satellite_lageos1", "ForecastElapsedBelowZero") => {
            "The stated time since the state was taken from a published orbit is below zero."
        }
        ("satellite_lageos1", "SpontaneousGeneration") => {
            "The trial step gained energy that the work of the Sun, the Moon, the tide, sunlight and relativity does not account for."
        }
        ("satellite_lageos1", "FirstLawBreach") => {
            "The trial step lost energy that the work of the Sun, the Moon, the tide, sunlight and relativity does not account for."
        }
        ("satellite_lageos1", "AngularMomentumBreach") => {
            "The trial step changed the angular momentum about the Earth's axis by more than the torque of the field, the Sun, the Moon, the tide, sunlight and relativity accounts for. The Coriolis term does no work and is seen by this ledger alone."
        }
        ("reactor_triga_jsi", "NonFiniteInput") => {
            "`dt`, the reactivity or the scram value was NaN or infinite."
        }
        ("reactor_triga_jsi", "NonFiniteState") => {
            "The incoming state, or the trial step, was non-finite."
        }
        ("reactor_triga_jsi", "NonPositiveDt") => "`dt` was zero or negative.",
        ("reactor_triga_jsi", "InvalidCard") => "A recorded constant is not this reactor's.",
        ("reactor_triga_jsi", "StartTemperatureNotLiquidWater") => {
            "The recorded starting temperature, which is also the pool water's, is below 273.15 K, where water freezes, or above 373.124 K, where it boils at one standard atmosphere (IAPWS-IF97)."
        }
        ("reactor_triga_jsi", "RodTableOutsideCard") => {
            "The recorded rod-worth table is not that of core 215, 217 or 231."
        }
        ("reactor_triga_jsi", "ReactivityNotPositive") => {
            "The inserted reactivity is zero or less: no step in reactivity is inserted, and the inserted fraction min(1, R t / ρ_0) divides by it."
        }
        ("reactor_triga_jsi", "ReactivityAboveRodWorth") => {
            "The inserted reactivity is above 4.16 printed dollars, the transient rod's whole worth on the institute's rod table of core 253: the request asks more than the rod holds."
        }
        ("reactor_triga_jsi", "ScramValueOutsideCard") => {
            "The scram value is neither 0 (rods held) nor 1 (rods dropped)."
        }
        ("reactor_triga_jsi", "ReactivityChangedDuringRun") => {
            "The inserted reactivity differs from the one this run started with; the model defines no change of it but the scram."
        }
        ("reactor_triga_jsi", "RodsRaisedAfterScram") => "The rods were raised after the scram.",
        ("reactor_triga_jsi", "NegativeEnergy") => {
            "The energy released in the handed-in state is negative."
        }
        ("reactor_triga_jsi", "ImpossiblePeak") => {
            "A peak field of the handed-in state cannot have come from a run: a peak power below the power, a peak time or peak energy outside the run so far, or a highest fuel temperature below the fuel temperature."
        }
        ("reactor_triga_jsi", "ImpossibleState") => {
            "The handed-in state is not physically possible: a power that is not positive, a negative precursor sum, a negative time, a fuel temperature below the starting temperature, or a scram time or scram energy outside the run so far."
        }
        ("reactor_triga_jsi", "FuelTemperatureAboveFuelLimit") => {
            "The core-average fuel temperature, in the handed-in state or where the step would land, is above 1423.15 K (1150 degC), the temperature limit of U-ZrH1.6 fuel in stainless clad."
        }
        ("reactor_triga_jsi", "DtAboveStateCeiling") => {
            "The step is longer than 0.25 of the time constant of the fastest pole, Λ / (β |ρ − 1|), at the state it starts from or where it lands."
        }
        ("reactor_triga_jsi", "SpontaneousGeneration") => {
            "The trial step created neutron population beyond what the reactivity supplies."
        }
        ("reactor_triga_jsi", "FirstLawBreach") => {
            "The trial step destroyed neutron population the reactivity supplies."
        }
        ("cantilever_eth", "NonFiniteInput") => "`dt` was NaN or infinite.",
        ("cantilever_eth", "NonFiniteState") => {
            "The incoming state, or a stage of the trial step, was not finite."
        }
        ("cantilever_eth", "NonPositiveDt") => "`dt` was zero or negative.",
        ("cantilever_eth", "InvalidBeam") => "A recorded constant is not this beam's.",
        ("cantilever_eth", "EnvelopeAboveCard") => {
            "The deflection envelope, a size, is above 18.75 mm at the start of the step or where the trial step lands: a deflection or a speed of either sign larger than the card takes. 18.75 mm is 3.119 V × 0.006012 m/V, the largest amplitude the softening and the damping were fitted on; above it those laws were not fitted, and the strip's own limit, its yield at the clamp, needs its alloy, which the dataset does not state."
        }
        ("cantilever_eth", "SpontaneousGeneration") => {
            "The trial step created mechanical energy: the stored energy plus the heat rose above the energy the run started with."
        }
        ("cantilever_eth", "FirstLawBreach") => {
            "The trial step destroyed mechanical energy: the stored energy plus the heat fell below the energy the run started with."
        }
        ("link_gothenburg_10010", "NonFiniteInput") => {
            "The rain rate or `dt` was NaN or infinite."
        }
        ("link_gothenburg_10010", "NonFiniteState") => {
            "The incoming state, or the trial step, was non-finite."
        }
        ("link_gothenburg_10010", "NonPositiveDt") => "`dt` was zero or negative.",
        ("link_gothenburg_10010", "InvalidLink") => "A recorded constant is not this link's.",
        ("link_gothenburg_10010", "NegativeRainRate") => {
            "The rain rate is below zero: rain does not fall upward."
        }
        ("link_gothenburg_10010", "GaugeMinuteOutsideCard") => {
            "The seconds already spent in the current gauge minute are below zero or not below 60 s."
        }
        ("link_gothenburg_10010", "DryPathLossBelowFreeSpace") => {
            "The dry level is below 44.32 dB, the free-space loss of this path less the gain of two ideal 0.3 m apertures: no pair of these antennas has that much gain."
        }
        ("link_gothenburg_10010", "AntennaWaterOutsideCard") => {
            "The attenuation of the water on the antennas is below zero or above the most the card gives it, 2.13 dB."
        }
        ("link_gothenburg_10010", "SpontaneousGeneration") => {
            "The trial step delivered more power than the dry path carries."
        }
        ("link_gothenburg_10010", "FirstLawBreach") => {
            "The trial step lost power that neither the rain nor the water on the antennas took."
        }
        ("link_gothenburg_10010", "AntennaAccountBreach") => {
            "The trial step broke the fitted model's account of the water on the antennas, which is bookkeeping that follows from the model and not a conservation law."
        }
        ("los_jwst_fine_guide", "NonFiniteInput") => {
            "`dt` or a number of the load was NaN or infinite."
        }
        ("los_jwst_fine_guide", "NonFiniteState") => {
            "The incoming state, or a number of the trial step, was not finite."
        }
        ("los_jwst_fine_guide", "NonPositiveDt") => "`dt` was zero or negative.",
        ("los_jwst_fine_guide", "InvalidObservatory") => {
            "A recorded constant is not this observatory's."
        }
        ("los_jwst_fine_guide", "LockOutsideSensorField") => {
            "The lock point is more than 70 arcsec from the centre of the sensor's ideal frame on either axis: the guider's field is 2.3 by 2.3 arcmin, and no star can be held beyond it."
        }
        ("los_jwst_fine_guide", "QuietBeforeStartBelowZero") => {
            "The stated time since the last disturbed centroid before the run is below zero."
        }
        ("los_jwst_fine_guide", "InvalidCovariance") => {
            "A variance in the incoming state, or the variance stated for a centroid, was zero or negative."
        }
        ("los_jwst_fine_guide", "MeasuredNotZeroOrOne") => "`measured` was neither 0 nor 1.",
        ("los_jwst_fine_guide", "UnmeasuredFrameCarriesNumbers") => {
            "A frame whose `measured` is 0 carried a centroid or a count rate that was not zero: such a frame is predicted and not updated, it has no centroid, and a number there would be one the filter does not take."
        }
        ("los_jwst_fine_guide", "CountRateNotPositive") => {
            "The guide star's count rate on a frame with a centroid is zero or below, or so near zero, under about 7e-237 counts per second, that the noise variance of the centroid, a power of one over the count rate, passes the largest 64-bit number: the noise law has no value at zero, and a centroid with no light holds no position."
        }
        ("los_jwst_fine_guide", "OffsetOutsideCard") => {
            "The centroid is more than 250 mas from the lock point on either axis."
        }
        ("los_jwst_fine_guide", "SpontaneousGeneration") => {
            "The trial step left more variance than the fading, the drive and the measurement account for."
        }
        ("los_jwst_fine_guide", "FirstLawBreach") => {
            "The trial step left less variance than the fading, the drive and the measurement account for: the filter would claim knowledge it was not given."
        }
        ("creep_cmsx6", "NonFiniteInput") => "`dt` or the held stress was NaN or infinite.",
        ("creep_cmsx6", "NonFiniteState") => {
            "The incoming state, or the trial step, was non-finite."
        }
        ("creep_cmsx6", "NonPositiveDt") => "`dt` was zero or negative.",
        ("creep_cmsx6", "InvalidCard") => "A recorded constant is not this alloy's.",
        ("creep_cmsx6", "StressOutsideCard") => {
            "The held initial stress is below 140 MPa or above 230 MPa."
        }
        ("creep_cmsx6", "StressNotHeld") => {
            "The stress of the step is not the stress the state was made at."
        }
        ("creep_cmsx6", "CreepExtensionBelowZero") => {
            "The incoming creep extension is below zero: a test piece in tension does not shorten by creep."
        }
        ("creep_cmsx6", "CreepExtensionOutsideCard") => {
            "The incoming creep extension is above 5 percent, the card's top."
        }
        ("creep_cmsx6", "CreepExtensionBeyondCard") => {
            "The trial step went past 5 percent creep extension, the card's top."
        }
        ("creep_cmsx6", "StepTooSmallToResolve") => {
            "The trial step did not move the creep extension in 64-bit arithmetic."
        }
        ("creep_cmsx6", "SpontaneousGeneration") => {
            "The trial step did more work by the load than the creep law dissipates in the time."
        }
        ("creep_cmsx6", "FirstLawBreach") => {
            "The trial step did less work by the load than the creep law dissipates in the time."
        }
        ("crater_ryugu_sci", "NonFiniteInput") => "`dt` was NaN or infinite.",
        ("crater_ryugu_sci", "NonFiniteState") => {
            "The incoming state, or a stage of the trial step, was not finite."
        }
        ("crater_ryugu_sci", "NonPositiveDt") => "`dt` was zero or negative.",
        ("crater_ryugu_sci", "InvalidCrater") => "A recorded constant is not this crater's.",
        ("crater_ryugu_sci", "RadiusBelowLedgerResolution") => {
            "The radius is under 0.1622 m: there the energy the wall has yet to give up is so large that one rounding of it in 64 bits passes a tenth of the ledger's limit, and the ledger cannot tell a right step from a wrong one."
        }
        ("crater_ryugu_sci", "RadiusAboveFinal") => {
            "The radius is above 8.571304329477922 m, where this crater stops."
        }
        ("crater_ryugu_sci", "WallMovingInward") => {
            "The wall's speed is below zero. A crater does not shrink in this model."
        }
        ("crater_ryugu_sci", "SpontaneousGeneration") => {
            "The trial step created energy: the wall's kinetic energy plus the energy it has given up rose above zero."
        }
        ("crater_ryugu_sci", "FirstLawBreach") => {
            "The trial step destroyed energy: the wall's kinetic energy plus the energy it has given up fell below zero."
        }
        ("tire_eve_r19_ice", "NonFiniteInput") => {
            "The load, the pressure, the slip, the speed or `dt` was NaN or infinite."
        }
        ("tire_eve_r19_ice", "NonFiniteState") => "The incoming state was not finite.",
        ("tire_eve_r19_ice", "NonPositiveDt") => "`dt` was zero or negative.",
        ("tire_eve_r19_ice", "InvalidTire") => "A recorded constant is not this tyre's.",
        ("tire_eve_r19_ice", "NoContact") => {
            "The normal load is zero or below, or under the smallest normal 64-bit number: the tyre does not press on the ice, and a friction force has nothing to act on."
        }
        ("tire_eve_r19_ice", "TyreFlat") => {
            "The inflation pressure, gauge, is zero or below: the tyre is flat, and the card is of a tyre inflated."
        }
        ("tire_eve_r19_ice", "SlipOutsideCard") => {
            "The slip magnitude is below 0.05 or above 0.20, the slips the constants cover."
        }
        ("tire_eve_r19_ice", "CarriageSpeedOutsideCard") => {
            "The carriage speed is below 0.05 m/s or above 0.07 m/s, the creep of the rig."
        }
        ("tire_eve_r19_ice", "DistanceBelowZero") => "The distance along the track is below zero.",
        ("tire_eve_r19_ice", "SlipHeldBelowZero") => {
            "The time the slip has been held, stated at the start or carried in the state, is below zero: a slip is held from the instant it is set."
        }
        ("tire_eve_r19_ice", "StepTooSmallToResolve") => {
            "The step does not move the time held at the slip, or the distance, in 64-bit arithmetic."
        }
        ("tire_eve_r19_ice", "SpontaneousGeneration") => {
            "The trial step created work: the drawbar work plus the slip heat rose above the wheel work."
        }
        ("tire_eve_r19_ice", "FirstLawBreach") => {
            "The trial step destroyed work: the drawbar work plus the slip heat fell below the wheel work."
        }
        ("fingerpad_p02_glass", "NonFiniteInput") => {
            "The normal force, the twist rate or `dt` was NaN or infinite."
        }
        ("fingerpad_p02_glass", "NonFiniteState") => "The incoming state was not finite.",
        ("fingerpad_p02_glass", "NonPositiveDt") => "`dt` was zero or negative.",
        ("fingerpad_p02_glass", "InvalidFingerPad") => {
            "A recorded constant is not this finger pad's."
        }
        ("fingerpad_p02_glass", "NoContact") => {
            "The normal force is zero or below, or under the smallest normal 64-bit number: the plate does not press on the pad, and the slip-torque limit and the characteristic twist of the torque are zero."
        }
        ("fingerpad_p02_glass", "TwistRateNotPositive") => {
            "The twist rate is zero or below, or under the smallest normal 64-bit number: the plate does not turn forward, and the characteristic twists, powers of the rate, have no value."
        }
        ("fingerpad_p02_glass", "TwistStepAboveQuadrature") => {
            "The step turns the plate by more than 0.05 of the characteristic twist of the torque, past which the four-point rule that sums the plate work and the heat is not exact to rounding."
        }
        ("fingerpad_p02_glass", "LoadChangedDuringTwist") => {
            "The normal force or the twist rate differs from the one this twist began with: the torque, the stuck fraction and the stored energy are closed forms of the twist under one load held from the untwisted pad, and under a changed load the bed's springs hold a state set by the path, which the card's state does not carry; the stored energy would jump with no work done."
        }
        ("fingerpad_p02_glass", "TwistNotStartedFromZero") => {
            "The twist at the start is not zero: the card's closed forms are of a pad twisted from zero under the load it holds, and a pad already twisted holds springs set by the load and the path it was brought there under, which a start does not carry."
        }
        ("fingerpad_p02_glass", "TwistBelowZero") => {
            "The twist is below zero: the twist is of the plate turning forward from the untwisted pad."
        }
        ("fingerpad_p02_glass", "SpontaneousGeneration") => {
            "The trial step created energy: the stored energy plus the friction heat rose above the plate work."
        }
        ("fingerpad_p02_glass", "FirstLawBreach") => {
            "The trial step destroyed energy: the stored energy plus the friction heat fell below the plate work."
        }
        ("accumulator_sbo450", "NonFiniteInput") => {
            "The piston speed, a starting value or `dt` was NaN or infinite."
        }
        ("accumulator_sbo450", "NonFiniteState") => {
            "The incoming state, or the trial step, was not finite."
        }
        ("accumulator_sbo450", "NonPositiveDt") => "`dt` was zero or negative.",
        ("accumulator_sbo450", "InvalidAccumulator") => {
            "A recorded constant is not this accumulator's."
        }
        ("accumulator_sbo450", "GasMassNotPositive") => "The gas mass is zero or less.",
        ("accumulator_sbo450", "WallTemperatureNotPositive") => {
            "The wall temperature is at or below absolute zero."
        }
        ("accumulator_sbo450", "PistonTravelAboveStep") => {
            "The piston would travel more than 1 mm in the step."
        }
        ("accumulator_sbo450", "GasBelowCriticalTemperature") => {
            "The gas temperature, incoming or landing, is at or below 126.192 K, nitrogen's critical temperature: the gas can condense, and one phase of the equation of state is not this charge."
        }
        ("accumulator_sbo450", "PressureAboveRated") => {
            "The gas pressure, incoming or landing, is above 330 bar, the vessel's rated maximum working pressure."
        }
        ("accumulator_sbo450", "NegativeMovedSum") => {
            "The work moved or the heat moved carried into a start is below zero."
        }
        ("accumulator_sbo450", "OilSideEmpty") => {
            "The gas volume, incoming or landing, is above the vessel's 0.58879 l: there is no oil left to draw out."
        }
        ("accumulator_sbo450", "HeatShareAboveCard") => {
            "The heat exchanged with the wall over the motion so far, with the sums carried into the start, incoming or landing, is more than 0.065 of the work done either way, with the work counted as 100 J until it is more: the motion is too slow, or the rest too long, for the card's one time constant."
        }
        ("accumulator_sbo450", "SpontaneousGeneration") => {
            "The trial step created energy: the gas gained more than the work on it less the heat it gave."
        }
        ("accumulator_sbo450", "FirstLawBreach") => {
            "The trial step destroyed energy: the gas gained less than the work on it less the heat it gave."
        }
        ("generator_mmrtg_f1", "ConverterAccountBreach") => {
            "The trial step broke the fitted model's account of the fading part of the converter, which is bookkeeping that follows from the model and not a conservation law."
        }
        // The gateway's own names, the same on every card that gives them.
        (_, crate::DT_BELOW_FLOOR) => {
            "`dt_s` is below the card's step floor, `floor_s` under `dt_s`. Refused before anything is computed."
        }
        (_, crate::DT_ABOVE_CEILING) => {
            "`dt_s` is above the card's step ceiling, `ceiling_s` under `dt_s`. Refused before anything is computed."
        }
        (_, crate::DT_ABOVE_NYQUIST) => {
            "`dt_s` is above `nyquist_s` under `dt_s`, π over the card's natural frequency or pole. Refused before anything is computed."
        }
        ("ti64_lattice", "ForceAboveNyquist") => {
            "`|F| dt_s` is past the card's impulse constant, the load rule of `force_n`. Refused before anything is computed."
        }
        (_, crate::fresh_start::NAME) => {
            "A start field that carries a run on is not a fresh start's, and the request names no sealed run it continues in `from`. Refused before anything is computed."
        }
        ("crater_ryugu_sci", CONTINUATION_NOT_ON_CARD) => {
            "A request with `from` asks to continue a sealed run of a crater that has not stopped: while it grows the wall's speed and the energy accounts are not start fields, so a continuation is served only from 8.571304329477922 m, the crater stopped. Refused before anything is computed."
        }
        (_, CONTINUATION_NOT_ON_CARD) => {
            "A request with `from` asks to continue a sealed run on this card, which serves no continuation: every start is a fresh start."
        }
        _ => return None,
    })
}

impl Clone for FieldSpec {
    fn clone(&self) -> Self {
        Self {
            name: self.name,
            role: self.role,
            unit: self.unit,
            components: self.components,
            minimum: self.minimum,
            minimum_exclusive: self.minimum_exclusive,
            maximum: self.maximum,
            maximum_exclusive: self.maximum_exclusive,
            bound: self.bound.clone(),
        }
    }
}

/// The one line of the MAST VDE card in `list_cards`.
pub(crate) const TOKAMAK_VDE_MAST_SUMMARY: &str = "The vertical displacement event of the MAST tokamak's plasma, the company's own derivation from shot 15045 at 0.250 s: a rigid plasma (EFIT's current, 755 kA) in MAST's held coil field, against the eddy currents of its passive structure as one wall circuit and the P6 vertical-control coils driven by the amplifier's voltage, from any start inside the vessel until its boundary reaches the wall, with its growth rate from the wall time and the stability margin and nothing fitted to the motion. Compared with the deliberate VDE of shot 15045: met in part, one line of three: the card's growth rate, 6.55 per second, is 93 percent under the measured 96.3 (and 94 percent under 121 by the forecast's own window), while its P6 current at the window's end is within 6.2 percent of the shot's. P6 held its flux in the shot, and a plasma held by it rises as fast as this one did only with about twice the rigid shift's drive.";

/// What the MAST VDE card is.
pub(crate) const TOKAMAK_VDE_MAST_WHAT: &str = "The MAST spherical tokamak (UKAEA, Culham) in shot 15045, from 0.250 s, the EFIT time slice just before the vertical-position feedback was cut on purpose to make an upward vertical displacement event. The plasma is rigid: EFIT's toroidal current density at 0.250 s, 878 cells and 755.3 kA, shifted bodily in height, with its current and its mass (deuterium at the line-average density, 5.98e-7 kg) held. It moves in the field of MAST's PF coils at their 0.250 s currents and of the passive currents MAST's AMM signals give at that time. Its motion induces eddy currents in the 126 passive elements of MAST's machine description (the vessel, the centre column's Inconel casing and graphite, the P2 arms and divertor plates, the stainless coil cases), held as one wall circuit in the shape of the growing mode of the whole set, and in the P6 radial-field pair, a circuit of its own driven by the fast amplifier's voltage. The ledger holds the magnetic energy of the two circuits, the plasma's kinetic energy and its energy in the held field against the resistive losses and the amplifier's work. The event ends when EFIT's boundary, shifted, reaches the limiter: at +0.389 m and at -0.391 m, the corner at R 1.04 m, Z 1.033 m, by the P3 coil, where the halo currents of shot 15045 flowed.";

/// The data behind the MAST VDE card.
pub(crate) const TOKAMAK_VDE_MAST_DATA: &str = "FAIR-MAST, the open archive of MAST, level 2, shot 15045: EFIT's equilibrium (current density, flux, boundary, scalars), the PF coil currents and geometry, the passive structure's geometry and AMM currents, the limiter contour, the line-average density, the centre-column Bv array and the flux loops; 692 files, each kept as served with its time, status and SHA-256. The constants derived from them, none fitted to the motion: every coil element one turn, the channel current of P2 to P6 the coil's ampere-turns and the solenoid's elements half its channel (with them EFIT's flux at 0.250 s outside the plasma is reproduced to 1.1 percent of its variation, RMS; 0.73 percent with each coil's scale fitted free, which is how the convention was found); resistivities from the AMM currents over the flux-loop voltages where a loop is near an element (Inconel 1.29e-6, graphite 13.5e-6, divertor plates 3.3e-6 ohm m) and stainless steel 304 at 20 degC, 0.72e-6, for the vessel and the coil cases; P6 copper at 20 degC with nothing for the amplifier and feeders. Compared with: the growth rate of shot 15045 published by Windridge and others at the 34th EPS Conference on Plasma Physics, paper P1.108, 96.3 per second from the centre-column Bv signal, with no window stated; the centre-column Bv probes themselves; and the P6 currents.";

/// The pass lines of the MAST VDE card.
pub(crate) const TOKAMAK_VDE_MAST_PASS_LINE: &str = "Error measures and targets. The measured position is the vertex of a least-squares parabola through the centre-column probes within 0.25 m of the one reading the most, as Windridge's signal is the peak of that field; its baseline the mean from 0.200 to 0.240 s; its window from the first sample after 0.250 s at which it is 1 cm above the baseline and stays above, to 0.16 of EFIT's minor radius at 0.250 s (8.97 cm, TCV's linear-phase rule); its growth rate the least-squares slope of the logarithm. Line 1: the card's linear growth rate within 25 percent of Windridge's 96.3 per second. Line 2: the growth rate of the card's own synthetic signal, from a run through the engine started at 1 mm on its growing mode at zero volts, by the same rule, within 25 percent of the measured one. Line 3: the P6 current at the end of the card's window within 25 percent of the measured one at the end of the measured window. Record SHA-256 3743912cffaa1274685092dd247e5aaf596f7af0f1e8275be7a1e3ccdc4b9f70; its measurement program, b758bd068d44056c3cbb189c4fb18b367769d92094c41e872bd98e33ec9de7fa.";

/// The result of the MAST VDE card's comparison.
pub(crate) const TOKAMAK_VDE_MAST_RESULT: &str = "Line 1: the card's linear growth rate is 6.55 per second against Windridge's 96.3, 93.2 percent low (target within 25 percent). Line 2: by the record's rule the measured probes grow at 121.3 per second, over 0.2608 to 0.2746 s of the shot (1.97 to 9.05 cm); the card's own synthetic signal, from its run through the engine (729 steps of 1 ms from the growing mode at 1 mm, ended at the wall), grows at 7.10 per second over 0.353 to 0.663 s of the run: 94.1 percent low (target within 25 percent). Line 3: at the end of the two windows P6U carries -2,933 ampere-turns in the card and -3,126 in the shot, 6.2 percent low (target within 25 percent). The two events reach that current at different speeds: on the card's slow growth P6, whose time is 172 ms, carries 54 percent of the current that would hold its flux, at a centroid height of 8.6 cm; in the shot P6 held its flux while the probes' vertex ran ahead of the magnetic axis. Beside the three lines: the card's run reaches the wall 0.066 to 0.067 s after its window ends, and the shot's current spike came 0.0144 s after its window ended; other windows of the measured signal give 81 to 263 per second, and EFIT's magnetic axis 82 to 155. Why the growth rate is low, from the shot's data: at 0.280 s the measured P6 current, 5,011 ampere-turns, is 98.7 percent of the 5,078 that hold its flux for EFIT's axis there, 8.03 cm above its place at 0.250 s, and with that current P6 alone pushes the plasma back harder than the held field's drive at that height, 3,870 N against 2,350 N. With P6 left open the 127 circuits would grow at 90.7 per second; with P6 closed, as the data show it was, they need 2.04 times the rigid shift's stiffness to grow at 96.3. The plasma did not move rigidly: by 0.280 s its boundary's centre had risen 0.175 m and its magnetic axis 0.083 m, and its elongation fell from 1.91 to 1.72. A rigid shift of the 0.250 s current carries about half the drive the event shows.";

/// The limits of the MAST VDE card.
pub(crate) const TOKAMAK_VDE_MAST_LIMITS: &str = "The record was written with Windridge's published 96.3 per second, which line 1 compares with, in view; with growth rates of the same shot already computed, 219 per second at onset and 94 later from the centre-column array and 82 to 155 from EFIT's axis; and with the PF coil currents read to 0.295 s, the P6 pair's among them, about 5 kA by 0.280 s, which line 3 compares with. The P6 circuit, copper with nothing for its amplifier and feeders, was set with that trace in view; nothing is fitted to the motion. One shot, one time slice, one tokamak; no outside party has repeated it. The plasma is rigid: the 0.250 s current density is shifted and not changed, while in shot 15045 the boundary's centre rose 0.175 m by 0.280 s and the magnetic axis 0.083 m, and the elongation fell from 1.91 to 1.72. The held field's force on the unshifted plasma, 113.3 N upward by the derivation's direct sum over EFIT's 878 cells, is set to zero in the card, the plasma taken in equilibrium at z = 0: it is the force of 4.04 mm of the held field's stiffness there, and over the rise to the upper contact its work would be 44.1 J of the 3,209 J the held field releases (44.3 J of 3,245 J down to the lower contact). The PF coils are held at their 0.250 s currents, while P2 fell from 70 to 53 kA-turns by 0.280 s; their up and down halves carry equal currents through the event, as series pairs do, so no part of them is a passive circuit. The wall is one circuit in the shape of the growing mode at z = 0: the card's two circuits have a stability margin of 1.27 there, where all 127 circuits as perfect conductors have 4.13, so the card holds the plasma less firmly against fast motion than the whole wall would, and its ideal-wall limit at +0.238 m and -0.239 m is the card's own and comes farther out for the whole wall. The resistivities of the vessel and the coil cases are a handbook value at 20 degC, those of the other elements are read from the AMM signals, and the P6 circuit's resistance leaves out its amplifier and feeders; P6's ampere-turn convention is the one the other coils' fit established. Halo currents, the thermal quench and the current quench after contact are not in the card. The ledger verifies the method and the arithmetic of every step; how the physics matches the tokamak is the comparison above.";

fn tokamak_vde_mast_card() -> Card {
    let card = tokamak_vde_mast::TokamakVdeMast::mast_15045();
    // The ceiling and the floor are locked constants of the card. Its fastest
    // pole is resolved inside each step by substeps, so no `π/ω` is published.
    let gate = card.gate_dt_s().expect("specimen gate");
    let floor = card.floor_dt_s().expect("specimen floor");
    finish(
        "energy",
        "tokamak_vde_mast",
        TOKAMAK_VDE_MAST_SUMMARY,
        vec![
            scalar(
                "displacement_m",
                Role::Phase,
                "m",
                Some(tokamak_vde_mast::LOWER_CONTACT_M),
                true,
                Some(tokamak_vde_mast::UPPER_CONTACT_M),
                true,
                format!(
                    "exclusive ({}, {}) m: the plasma's height above its equilibrium at 0.250 s, upward positive. At either end EFIT's boundary, shifted bodily, meets the limiter at the corner at R 1.04 m, Z 1.033 m above or below: a start there or beyond is refused as StartOutsideVessel, and a step that reaches either is the end of the event, PlasmaAtWall",
                    token(tokamak_vde_mast::LOWER_CONTACT_M),
                    token(tokamak_vde_mast::UPPER_CONTACT_M)
                ),
            ),
            scalar(
                "velocity_m_s",
                Role::Phase,
                "m/s",
                Some(-tokamak_vde_mast::ALFVEN_SPEED_M_S),
                true,
                Some(tokamak_vde_mast::ALFVEN_SPEED_M_S),
                true,
                format!(
                    "exclusive (-{0}, {0}) m/s: the plasma's vertical velocity. {0} m/s is its Alfven speed on the magnetic axis at 0.250 s (0.5731 T, 8.255e-8 kg per cubic metre): the plasma cannot move as one body faster. A start at or above it is refused as SpeedAtAlfvenSpeed, and so is a step any of whose stages reaches it: the speed is tested at every stage of every substep, where the wall is, so the step asked for does not turn this limit into the end at the wall. The growing mode at 1 mm moves at 6.5465e-3 m/s",
                    token(tokamak_vde_mast::ALFVEN_SPEED_M_S)
                ),
            ),
            scalar(
                "wall_current_a",
                Role::Phase,
                "A",
                None,
                false,
                None,
                false,
                "any finite current, A: the current of the wall mode, the eddy currents of the 126 passive elements in the shape of the growing mode, scaled so that the sizes of the elements' currents sum to it. 0 for a plasma at rest at its equilibrium; the growing mode at 1 mm carries -21.946577663159488 A. A start whose currents do not balance the held field's force on the plasma sets off the plasma's inertial oscillation, 25.8 microseconds a period".into(),
            ),
            scalar(
                "p6_current_a",
                Role::Phase,
                "A",
                None,
                false,
                None,
                false,
                "any finite current, A: the current in each of the 8 turns of P6, positive in the upper coil in the sense of the plasma current and negative in the lower; each coil's ampere-turns, the value of the P6U channel, are 4 times it. The growing mode at 1 mm carries -8.150423262860404 A".into(),
            ),
            scalar(
                "p6_voltage_v",
                Role::Load,
                "V",
                None,
                false,
                None,
                false,
                "any finite voltage, V: the fast amplifier's voltage across the P6 pair, held through a step; a later segment may change it. 0 in shot 15045 after 0.250 s, when the position feedback was cut and P6 was left as a closed circuit. Positive pushes current up the upper coil and pulls the plasma up".into(),
            ),
        ],
        floor,
        "the ceiling divided by 2^20, about 9.54e-10 s: a step that still moves a clock of 100 s",
        gate,
        "a locked 1 ms, the load's hold: each step is cut into fourth-order Runge-Kutta substeps no longer than 0.02 over Fujiwara's bound on the roots of the characteristic polynomial of the rates' Jacobian at the substep's start, so the step ceiling comes from the fastest pole at every substep. That pole is the plasma's inertial oscillation, 2.44e5 rad/s at the equilibrium and 3.03e5 per second at the most over the span with no current: a step of 1 ms on the growing mode takes about 24,400 substeps, and a step that would take more than 1,048,576 is refused as SubstepsAboveBudget. Over hard starts (0.30 m off balance at rest, 20,000 m/s, 1 MA in the wall, 100 kA in P6, under 0, 300 and -3,000 V) the largest conservation residual at 0.1 and 1 ms is 8.6e-10 against a limit of 2e-8",
        None,
        "A run starts at time zero at the displacement, velocity and two currents given, with its ledger anchored there: the energy is the plasma's kinetic energy, its energy in the held field, zero at the equilibrium, and the magnetic energy of the wall mode and P6 with their mutual. Each step holds the amplifier's voltage p6_voltage_v, and a later segment may change it; the ledger books the amplifier's work and the resistive losses of the two circuits over every substep by the corrected trapezoid rule, and holds the defect at every substep to 2e-8 of the energy in play, the size of each term at the anchor or now with the losses and the work, or of 0.01 J when that is smaller. The state reports the held field's force on the plasma, the wall mode's force (the passive structure feels the opposite: the vertical force a VDE puts on the vessel) and P6's, the resistive power of each circuit and the amplifier's, each energy and the ledger's sums, the gap to the nearer contact height and the substeps the last step took. The run ends where the event ends: a step any of whose stages carries EFIT's boundary to the limiter, at +0.389 m or -0.391 m, is refused as PlasmaAtWall and the run is sealed up to the last step that landed, the outcome ended. A step any of whose stages carries the plasma to its Alfven speed is refused as SpeedAtAlfvenSpeed, the run sealed up to the last step that landed. The two are tested at the same stages, so which one a run meets does not follow the step asked for: from rest at the equilibrium with 1 MA in P6 every step from 1e-8 s to 1 ms meets the Alfven speed near 0.30 m, 3.4e-7 s after the start, before the wall. Two steps of different length meet them in a different order where the plasma would reach both within one substep, and nowhere else, because a step cut otherwise puts the stages of that substep at other times. A frame carries FLAG_PAST_IDEAL_WALL_LIMIT where the landed displacement is past +0.238 m or -0.239 m: there the held field's stiffness, which rises from 28,022.5 N/m at the equilibrium to 109,777 N/m at the upper contact and 109,812 N/m at the lower, is above what the wall mode and P6 as perfect conductors would push back with, and the plasma runs on its inertial time, in microseconds, to the wall. A single step starts again at each call from the state it is given: a returned state's displacement_m, velocity_m_s, wall_current_a and p6_current_a handed back as a new start continue the same motion, with t_s and the ledger begun again. A step of zero or less is refused as NonPositiveDt before the step floor is tested.".to_string(),
        &[
            "NonFiniteInput",
            "NonFiniteState",
            "NonPositiveDt",
            "InvalidCard",
            "StartOutsideVessel",
            "SpeedAtAlfvenSpeed",
            "PlasmaAtWall",
            "SubstepsAboveBudget",
            "SpontaneousGeneration",
            "FirstLawBreach",
        ],
        vec![Advisory {
            name: "FLAG_PAST_IDEAL_WALL_LIMIT",
            mask: tokamak_vde_mast::FLAG_PAST_IDEAL_WALL_LIMIT,
            seals: true,
        }],
        slots(&[
            ("t", "Time since the start of the run, in seconds."),
            ("pos[0]", "displacement_m"),
            ("pos[1]", "wall_current_a"),
            ("pos[2]", "p6_current_a"),
            ("vel[0]", "velocity_m_s"),
            ("vel[1]", "wall_power_w"),
            ("vel[2]", "gap_to_wall_m"),
            ("force_torque", "The wall mode's force on the plasma, in newtons, upward positive; the passive structure feels the opposite."),
            ("residual", "Conservation residual."),
            (
                "flags",
                "FLAG_PAST_IDEAL_WALL_LIMIT: the landed displacement is past the card's ideal-wall limit, +0.238 m or -0.239 m, where the plasma grows on its inertial time. It seals.",
            ),
            ("proof", "Not written by the step."),
        ]),
    )
}

/// The worked examples of the MAST VDE card: one step from the growing mode at
/// 1 mm, and 10 ms of its growth at zero volts, at 0.1 ms a step.
fn tokamak_vde_mast_examples() -> (Value, Value) {
    pair_examples_recorded(
        "energy",
        "tokamak_vde_mast",
        obj(&[
            ("displacement_m", scalar_value(0.001)),
            ("velocity_m_s", scalar_value(0.006546503766672119)),
            ("wall_current_a", scalar_value(-21.946577663159488)),
            ("p6_current_a", scalar_value(-8.150423262860404)),
        ]),
        obj(&[("p6_voltage_v", scalar_value(0.0))]),
        scalar_value(1.0e-4),
        TOKAMAK_VDE_MAST_EXAMPLE_STEPS,
        10,
    )
}

/// The steps the worked run of the MAST VDE card asks for: 10 ms.
pub(crate) const TOKAMAK_VDE_MAST_EXAMPLE_STEPS: u64 = 100;

/// The reject sentences of the MAST VDE card, word for word from its kernel.
fn tokamak_vde_mast_reject_meaning(name: &str) -> Option<&'static str> {
    Some(match name {
        "NonFiniteInput" => "`dt` or the amplifier voltage was NaN or infinite.",
        "NonFiniteState" => "The incoming state, or the trial step, was non-finite.",
        "NonPositiveDt" => "`dt` was zero or negative.",
        "InvalidCard" => "A recorded constant is not this card's.",
        "StartOutsideVessel" => "The displacement is at or beyond a contact height, −0.391 m or +0.389 m: there EFIT's boundary, shifted, touches or crosses the limiter, and the plasma is not inside the vessel.",
        "SpeedAtAlfvenSpeed" => "The plasma's vertical speed is at or above its Alfvén speed, 1.78e6 m/s, at the start or at any stage of a substep, where the wall is tested too: the plasma cannot move as one body faster than the field carries its force across it.",
        "PlasmaAtWall" => "The step carries EFIT's boundary to the limiter, at −0.391 m or +0.389 m: the plasma reaches the wall and the vertical displacement event ends.",
        "SubstepsAboveBudget" => "The step would take more than 1,048,576 substeps, each 0.02 of the inverse of the bound on its fastest pole.",
        "SpontaneousGeneration" => "The trial step created energy beyond what the amplifier supplied: a defect of the arithmetic or the integrator.",
        "FirstLawBreach" => "The trial step destroyed energy beyond the resistive losses: a defect of the arithmetic or the integrator.",
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every flag a card declares has the state boolean that sets it, every
    /// quantity its frame packs has its unit, and every card the quantity of
    /// its force_torque slot. A card without one is named here with the row
    /// it needs, rather than met as a panic inside describe_card.
    #[test]
    fn every_card_has_the_rows_its_flags_and_its_frame_need() {
        let mut missing = Vec::new();
        for item in all() {
            for flag in &item.advisory {
                if flag_state_known(flag.name).is_none() {
                    missing.push(format!("{}: {}", item.card, missing_row("flag_state", flag.name)));
                }
            }
            if force_torque_state_known(item.card).is_none() {
                missing.push(format!("{}: {}", item.card, missing_row("force_torque_state", item.card)));
                continue;
            }
            let described = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| frame_slots(item)));
            if let Err(fault) = described {
                let said = fault.downcast_ref::<String>().cloned().unwrap_or_default();
                missing.push(format!("{}: {said}", item.card));
            }
        }
        assert!(missing.is_empty(), "{missing:#?}");
        // The sentence names the key and the table, and where they are.
        assert!(missing_row("flag_state", "FLAG_NEW").starts_with("FLAG_NEW has no row in flag_state in protocol_mcp_gateway/src/descriptor.rs"));
        assert!(flag_state_known("FLAG_NEW").is_none() && unit_of_known("new_quantity_x").is_none() && force_torque_state_known("new_card").is_none());
    }


    #[test]
    fn parameter_lists_follow_the_request_order() {
        let expected = [
            (
                "materials",
                "ti64_lattice",
                vec![
                    "position_m",
                    "velocity_m_s",
                    "start_t_s",
                    "start_force_n",
                    "start_temperature_k",
                    "start_energy_anchor_j",
                    "start_boundary_work_j",
                    "force_n",
                    "torque_nm",
                    "temperature_k",
                    "dt_s",
                ],
            ),
            (
                "energy",
                "grid_node",
                vec![
                    "current_a",
                    "voltage_v",
                    "temperature_k",
                    "upstream_voltage_v",
                    "load_current_a",
                    "ambient_k",
                    "dt_s",
                ],
            ),
            (
                "energy",
                "swing",
                vec![
                    "delta_rad",
                    "delta_omega_rad_s",
                    "pll_err_rad",
                    "p_mech_pu",
                    "dt_s",
                ],
            ),
            (
                "energy",
                "battery_sag",
                vec!["soc", "current_a", "dt_s"],
            ),
            (
                "energy",
                "cell_18650pf",
                vec![
                    "soc",
                    "held_current_a",
                    "current_a",
                    "dt_s",
                ],
            ),
            (
                "energy",
                "thermal",
                vec![
                    "temperature_k",
                    "heat_power_w",
                    "ambient_k",
                    "dt_s",
                ],
            ),
            (
                "signal",
                "resonance",
                vec![
                    "displacement_m",
                    "velocity_m_s",
                    "t_s",
                    "energy_anchor_j",
                    "boundary_work_j",
                    "accel_m_s2",
                    "dt_s",
                ],
            ),
            (
                "signal",
                "tesseract",
                vec![
                    "displacement_m",
                    "velocity_m_s",
                    "energy_defect_j",
                    "accel_m_s2",
                    "omega_ext_rad_s",
                    "sense_velocity_m_s",
                    "dt_s",
                ],
            ),
            (
                "materials",
                "hydraulic_fluid",
                vec!["pressure_pa", "flow_m3_s", "dt_s"],
            ),
            (
                "materials",
                "cable_stretch",
                vec![
                    "elongation_m",
                    "velocity_m_s",
                    "force_n",
                    "dt_s",
                ],
            ),
            (
                "energy",
                "generator_mmrtg_f1",
                vec!["mission_time_s", "dt_s"],
            ),
            (
                "aerospace",
                "capsule_stardust",
                vec![
                    "radius_m",
                    "radial_speed_m_s",
                    "transverse_speed_m_s",
                    "dt_s",
                ],
            ),
            (
                "aerospace",
                "satellite_lageos1",
                vec![
                    "epoch_utc_s",
                    "forecast_elapsed_s",
                    "position_m",
                    "velocity_m_s",
                    "dt_s",
                ],
            ),
            (
                "energy",
                "reactor_spert3",
                vec![
                    "inlet_temperature_k",
                    "system_pressure_mpa",
                    "coolant_flow_m_per_s",
                    "initial_power_mw",
                    "reactivity_inserted_dollar",
                    "dt_s",
                ],
            ),
            (
                "signal",
                "cantilever_eth",
                vec!["deflection_m", "velocity_m_s", "dt_s"],
            ),
            (
                "energy",
                "cell_2170_model3",
                vec![
                    "cell_number",
                    "state_of_charge",
                    "capacity_ah",
                    "case_temperature_least_c",
                    "case_temperature_most_c",
                    "current_a",
                    "case_temperature_c",
                    "dt_s",
                ],
            ),
            (
                "materials",
                "creep_cmsx6",
                vec!["creep_extension_percent", "stress_mpa", "dt_s"],
            ),
            (
                "aerospace",
                "capsule_insight",
                vec![
                    "radius_m",
                    "radial_speed_m_s",
                    "transverse_speed_m_s",
                    "lift_over_drag",
                    "dt_s",
                ],
            ),
            (
                "aerospace",
                "crater_ryugu_sci",
                vec!["radius_m", "dt_s"],
            ),
            (
                "energy",
                "generator_mmrtg_seebeck",
                vec!["mission_time_s", "dt_s"],
            ),
            (
                "aerospace",
                "momentum_jwst_dihedral",
                vec![
                    "h_eci_nms",
                    "q1",
                    "q2",
                    "q3",
                    "q4",
                    "sun_x",
                    "sun_y",
                    "sun_z",
                    "sun_distance_au",
                    "dt_s",
                ],
            ),
            (
                "aerospace",
                "momentum_jwst_srp",
                vec![
                    "h_eci_nms",
                    "q1",
                    "q2",
                    "q3",
                    "q4",
                    "sun_x",
                    "sun_y",
                    "sun_z",
                    "sun_distance_au",
                    "dt_s",
                ],
            ),
            (
                "signal",
                "link_gothenburg_10010",
                vec![
                    "dry_path_loss_db",
                    "antenna_water_db",
                    "gauge_minute_elapsed_s",
                    "gauge_minute_rain_mm_h",
                    "rain_rate_mm_h",
                    "dt_s",
                ],
            ),
            (
                "energy",
                "grid_gb",
                vec![
                    "frequency_hz",
                    "inertia_mw_s",
                    "demand_mw",
                    "dc_low_mw",
                    "dm_low_mw",
                    "dr_low_mw",
                    "dc_high_mw",
                    "dm_high_mw",
                    "dr_high_mw",
                    "fast_delivery_s",
                    "regulation_delivery_s",
                    "loss_mw",
                    "dt_s",
                ],
            ),
            (
                "aerospace",
                "solar_system",
                vec![
                    "sun_position_m",
                    "sun_velocity_m_s",
                    "mercury_position_m",
                    "mercury_velocity_m_s",
                    "venus_position_m",
                    "venus_velocity_m_s",
                    "earth_position_m",
                    "earth_velocity_m_s",
                    "moon_position_m",
                    "moon_velocity_m_s",
                    "mars_position_m",
                    "mars_velocity_m_s",
                    "jupiter_position_m",
                    "jupiter_velocity_m_s",
                    "saturn_position_m",
                    "saturn_velocity_m_s",
                    "uranus_position_m",
                    "uranus_velocity_m_s",
                    "neptune_position_m",
                    "neptune_velocity_m_s",
                    "ceres_position_m",
                    "ceres_velocity_m_s",
                    "pallas_position_m",
                    "pallas_velocity_m_s",
                    "vesta_position_m",
                    "vesta_velocity_m_s",
                    "energy_anchor",
                    "booked_work",
                    "angular_momentum_anchor",
                    "booked_torque",
                    "dt_s",
                ],
            ),
            (
                "materials",
                "tire_eve_r19_ice",
                vec![
                    "distance_m",
                    "slip_held_s",
                    "normal_load_n",
                    "inflation_pressure_pa",
                    "slip_ratio",
                    "carriage_speed_m_s",
                    "dt_s",
                ],
            ),
            (
                "aerospace",
                "los_jwst_fine_guide",
                vec![
                    "lock_x_arcsec",
                    "lock_y_arcsec",
                    "quiet_before_start_s",
                    "measured",
                    "centroid_x_arcsec",
                    "centroid_y_arcsec",
                    "counts_per_s",
                    "dt_s",
                ],
            ),
            (
                "energy",
                "tokamak_jet_dte1",
                vec![
                    "thermal_energy_j",
                    "deuterium_beam_ion_energy_j",
                    "tritium_beam_ion_energy_j",
                    "tail_ion_energy_j",
                    "elapsed_time_s",
                    "deuterium_beam_power_w",
                    "tritium_beam_power_w",
                    "deuterium_beam_energy_kev",
                    "tritium_beam_energy_kev",
                    "icrh_power_w",
                    "ohmic_power_w",
                    "line_density_per_m3",
                    "plasma_current_a",
                    "toroidal_field_t",
                    "tritium_fraction",
                    "electron_temperature_kev",
                    "effective_charge",
                    "hot_ion_regime",
                    "dt_s",
                ],
            ),
            (
                "materials",
                "fingerpad_p02_glass",
                vec![
                    "twist_deg",
                    "normal_force_n",
                    "twist_rate_deg_s",
                    "dt_s",
                ],
            ),
            (
                "energy",
                "accumulator_sbo450",
                vec![
                    "gas_mass_kg",
                    "gas_pressure_pa",
                    "gas_temperature_k",
                    "wall_temperature_k",
                    "work_moved_j",
                    "heat_moved_j",
                    "piston_speed_m_s",
                    "dt_s",
                ],
            ),
            (
                "energy",
                "reactor_triga_jsi",
                vec![
                    "start_temperature_k",
                    "rod_table_core",
                    "reactivity_inserted_dollar",
                    "rods_dropped",
                    "dt_s",
                ],
            ),
            (
                "energy",
                "reactor_spert3_ccore",
                vec![
                    "initial_temperature_k",
                    "system_pressure_psig",
                    "reactivity_inserted_dollar",
                    "coolant_flow_m_per_s",
                    "dt_s",
                ],
            ),
            (
                "energy",
                "runaway_18650_lco",
                vec![
                    "temperature_k",
                    "sei_fraction",
                    "anode_lithium_fraction",
                    "cathode_unreacted_fraction",
                    "electrolyte_fraction",
                    "separator_unmelted_fraction",
                    "vent_gas_held_fraction",
                    "oven_temperature_k",
                    "heater_power_w",
                    "adiabatic",
                    "dt_s",
                ],
            ),
            (
                "materials",
                "grasp_lift_glass_4tu",
                vec![
                    "hand_velocity_m_s",
                    "grip_force_n",
                    "hand_acceleration_m_s2",
                    "dt_s",
                ],
            ),
            (
                "energy",
                "tokamak_vde_mast",
                vec![
                    "displacement_m",
                    "velocity_m_s",
                    "wall_current_a",
                    "p6_current_a",
                    "p6_voltage_v",
                    "dt_s",
                ],
            ),
            (
                "energy",
                "reactor_msre_pump",
                vec!["pump_speed_at_start", "pump_speed_fraction", "dt_s"],
            ),
        ];
        assert_eq!(all().len(), expected.len());
        for (domain, card, names) in expected {
            assert_eq!(parameter_names(domain, card), names.as_slice());
            let phase = phase_names(domain, card);
            let load = load_names(domain, card);
            assert_eq!(&names[..phase.len()], phase);
            assert_eq!(&names[phase.len()..names.len() - 1], load);
            assert_eq!(names.last().copied(), Some("dt_s"));
        }
    }

    /// The floor and the ceiling a card publishes are the kernel card's own,
    /// and every description carries the rule for a request number.
    #[test]
    fn every_card_publishes_its_own_floor_and_ceiling_and_the_number_rule() {
        let expected = [
            (
                "ti64_lattice",
                ti64_lattice::Ti64LatticeCell::annealed_column().floor_dt_s(),
                ti64_lattice::Ti64LatticeCell::annealed_column().gate_dt_s(),
            ),
            (
                "grid_node",
                grid_node::GridNode::copper_fuse_link().floor_dt_s(),
                grid_node::GridNode::copper_fuse_link().gate_dt_s(),
            ),
            (
                "swing",
                swing::SwingMachine::synchronous_60hz().floor_dt_s(),
                swing::SwingMachine::synchronous_60hz().gate_dt_s(),
            ),
            (
                "battery_sag",
                battery_sag::LiIonPack::pack_12s().floor_dt_s(),
                battery_sag::LiIonPack::pack_12s().gate_dt_s(),
            ),
            (
                "cell_18650pf",
                cell_18650pf::Cell18650Pf::cell().floor_dt_s(),
                cell_18650pf::Cell18650Pf::cell().gate_dt_s(),
            ),
            (
                "thermal",
                thermal::ThermalNode::avionics_cold_plate().floor_dt_s(),
                thermal::ThermalNode::avionics_cold_plate().gate_dt_s(),
            ),
            (
                "resonance",
                resonance::DynamicOscillator::mount_10hz().floor_dt_s(),
                resonance::DynamicOscillator::mount_10hz().gate_dt_s(),
            ),
            (
                "tesseract",
                tesseract::Tesseract::drive_mode().floor_dt_s(),
                tesseract::Tesseract::drive_mode().gate_dt_s(),
            ),
            (
                "hydraulic_fluid",
                hydraulic_fluid::FluidColumn::mineral_oil().floor_dt_s(),
                hydraulic_fluid::FluidColumn::mineral_oil().gate_dt_s(),
            ),
            (
                "cable_stretch",
                cable_stretch::Tendon::dyneema().floor_dt_s(),
                cable_stretch::Tendon::dyneema().gate_dt_s(),
            ),
            (
                "generator_mmrtg_f1",
                generator_mmrtg_f1::GeneratorMmrtgF1::curiosity().floor_dt_s(),
                generator_mmrtg_f1::GeneratorMmrtgF1::curiosity().gate_dt_s(),
            ),
            (
                "capsule_stardust",
                capsule_stardust::CapsuleStardust::sample_return_capsule().floor_dt_s(),
                capsule_stardust::CapsuleStardust::sample_return_capsule().gate_dt_s(),
            ),
            (
                "satellite_lageos1",
                satellite_lageos1::SatelliteLageos1::lageos1().floor_dt_s(),
                satellite_lageos1::SatelliteLageos1::lageos1().gate_dt_s(),
            ),
            (
                "reactor_spert3",
                genesis_kernel::reactor_spert3_twin::ReactorSpert3Twin::e_core().floor_dt_s(),
                genesis_kernel::reactor_spert3_twin::ReactorSpert3Twin::e_core().gate_dt_s(),
            ),
            (
                "cantilever_eth",
                cantilever_eth::CantileverEth::first_bending_mode().floor_dt_s(),
                cantilever_eth::CantileverEth::first_bending_mode().gate_dt_s(),
            ),
            (
                "cell_2170_model3",
                genesis_kernel::cell_2170_twin::Cell2170Twin::cell().floor_dt_s(),
                genesis_kernel::cell_2170_twin::Cell2170Twin::cell().gate_dt_s(),
            ),
            (
                "creep_cmsx6",
                creep_cmsx6::CreepCmsx6::at_980_c().floor_dt_s(),
                creep_cmsx6::CreepCmsx6::at_980_c().gate_dt_s(),
            ),
            (
                "capsule_insight",
                capsule_insight::CapsuleInsight::entry_capsule().floor_dt_s(),
                capsule_insight::CapsuleInsight::entry_capsule().gate_dt_s(),
            ),
            (
                "crater_ryugu_sci",
                crater_ryugu_sci::CraterRyuguSci::sci_impact().floor_dt_s(),
                crater_ryugu_sci::CraterRyuguSci::sci_impact().gate_dt_s(),
            ),
            (
                "generator_mmrtg_seebeck",
                genesis_kernel::generator_mmrtg_seebeck::GeneratorMmrtgSeebeck::curiosity().floor_dt_s(),
                genesis_kernel::generator_mmrtg_seebeck::GeneratorMmrtgSeebeck::curiosity().gate_dt_s(),
            ),
            (
                "momentum_jwst_dihedral",
                genesis_kernel::momentum_jwst_dihedral::MomentumJwstDihedral::observatory().floor_dt_s(),
                genesis_kernel::momentum_jwst_dihedral::MomentumJwstDihedral::observatory().gate_dt_s(),
            ),
            (
                "momentum_jwst_srp",
                genesis_kernel::momentum_jwst_srp::MomentumJwstSrp::observatory().floor_dt_s(),
                genesis_kernel::momentum_jwst_srp::MomentumJwstSrp::observatory().gate_dt_s(),
            ),
            (
                "link_gothenburg_10010",
                link_gothenburg_10010::LinkGothenburg10010::sublink_587().floor_dt_s(),
                link_gothenburg_10010::LinkGothenburg10010::sublink_587().gate_dt_s(),
            ),
            (
                "grid_gb",
                genesis_kernel::grid_gb_twin::GridGbTwin::great_britain().floor_dt_s(),
                genesis_kernel::grid_gb_twin::GridGbTwin::great_britain().gate_dt_s(),
            ),
            (
                "solar_system",
                solar_system::SolarSystem::de440().floor_dt_s(),
                solar_system::SolarSystem::de440().gate_dt_s(),
            ),
            (
                "tire_eve_r19_ice",
                tire_eve_r19_ice::TireEveR19Ice::on_rig_ice().floor_dt_s(),
                tire_eve_r19_ice::TireEveR19Ice::on_rig_ice().gate_dt_s(),
            ),
            (
                "los_jwst_fine_guide",
                los_jwst_fine_guide::LosJwstFineGuide::observatory().floor_dt_s(),
                los_jwst_fine_guide::LosJwstFineGuide::observatory().gate_dt_s(),
            ),
            (
                "tokamak_jet_dte1",
                genesis_kernel::tokamak_jet_dte1_twin::TokamakJetDte1Twin::dte1().floor_dt_s(),
                genesis_kernel::tokamak_jet_dte1_twin::TokamakJetDte1Twin::dte1().gate_dt_s(),
            ),
            (
                "fingerpad_p02_glass",
                fingerpad_p02_glass::FingerPadP02Glass::on_glass().floor_dt_s(),
                fingerpad_p02_glass::FingerPadP02Glass::on_glass().gate_dt_s(),
            ),
            (
                "accumulator_sbo450",
                accumulator_sbo450::AccumulatorSbo450::on_darmstadt_rig().floor_dt_s(),
                accumulator_sbo450::AccumulatorSbo450::on_darmstadt_rig().gate_dt_s(),
            ),
            (
                "reactor_triga_jsi",
                reactor_triga_jsi::ReactorTrigaJsi::core_56_pulse_mode().floor_dt_s(),
                reactor_triga_jsi::ReactorTrigaJsi::core_56_pulse_mode().gate_dt_s(),
            ),
            (
                "reactor_spert3_ccore",
                genesis_kernel::reactor_spert3_ccore_plate::ReactorSpert3CcorePlate::c_core_plate().floor_dt_s(),
                genesis_kernel::reactor_spert3_ccore_plate::ReactorSpert3CcorePlate::c_core_plate().gate_dt_s(),
            ),
            (
                "runaway_18650_lco",
                genesis_kernel::runaway_18650_lco_twin::Runaway18650LcoTwin::fitted().floor_dt_s(),
                genesis_kernel::runaway_18650_lco_twin::Runaway18650LcoTwin::fitted().gate_dt_s(),
            ),
            (
                "grasp_lift_glass_4tu",
                genesis_kernel::grasp_lift_glass_4tu::GraspLiftGlass4tu::specimen().floor_dt_s(),
                genesis_kernel::grasp_lift_glass_4tu::GraspLiftGlass4tu::specimen().gate_dt_s(),
            ),
            (
                "tokamak_vde_mast",
                tokamak_vde_mast::TokamakVdeMast::mast_15045().floor_dt_s(),
                tokamak_vde_mast::TokamakVdeMast::mast_15045().gate_dt_s(),
            ),
            (
                "reactor_msre_pump",
                genesis_kernel::reactor_msre_pump::ReactorMsrePump::specimen().floor_dt_s(),
                genesis_kernel::reactor_msre_pump::ReactorMsrePump::specimen().gate_dt_s(),
            ),
        ];
        assert_eq!(all().len(), expected.len());
        for (item, (card, floor, ceiling)) in all().iter().zip(expected) {
            assert_eq!(item.card, card);
            let floor = floor.expect("the card states a floor");
            let ceiling = ceiling.expect("the card states a ceiling");
            assert_eq!(item.floor_s.to_bits(), floor.to_bits(), "{card}");
            assert_eq!(item.ceiling_s.to_bits(), ceiling.to_bits(), "{card}");
            // The floor of each reactor card is a locked constant of the card, 1e-9 s.
            // The 2170 cell's floor is its ceiling: the one second it steps by.
            // The physical 2170 cell's floor is a locked constant, 1 s, equal
            // to its locked ceiling.
            // The line-of-sight card's floor and ceiling are one sensor frame
            // less and more half a millisecond.
            let locked_floor = card == "reactor_spert3"
                || card == "reactor_spert3_ccore"
                || card == "reactor_triga_jsi"
                || card == "cell_2170_model3"
                || card == "creep_cmsx6"
                || card == "grid_gb"
                || card == "solar_system"
                || card == "tire_eve_r19_ice"
                || card == "fingerpad_p02_glass"
                || card == "los_jwst_fine_guide";
            if card == "grid_gb" {
                assert_eq!(floor.to_bits(), genesis_kernel::grid_gb::FLOOR_DT_S.to_bits());
            } else if card == "reactor_triga_jsi" {
                assert_eq!(floor.to_bits(), reactor_triga_jsi::FLOOR_DT_S.to_bits());
                assert_eq!(floor.to_bits(), 1.0e-9_f64.to_bits());
                // The floor still moves the clock at the end of the longest run, a million
                // steps at the ceiling, 93.9 s.
                assert!(1.0e6 * ceiling + floor > 1.0e6 * ceiling);
                // The ceiling follows the fitted generation time: 0.094 ms.
                assert!((ceiling - 9.39365e-5).abs() < 1.0e-9);
            } else if card == "reactor_spert3" {
                // The twin's floor is a locked constant; its ceiling is 0.2 of l* at 70 degF.
                assert_eq!(floor.to_bits(), genesis_kernel::reactor_spert3_twin::FLOOR_DT_S.to_bits());
                assert_eq!(ceiling.to_bits(), (genesis_kernel::reactor_spert3_twin::GATE_PHASE_STEP * genesis_kernel::reactor_spert3_twin::GENERATION_TIME_REDUCED_S[0]).to_bits());
            } else if card == "reactor_spert3_ccore" {
                assert_eq!(floor.to_bits(), genesis_kernel::reactor_spert3_ccore_plate::FLOOR_DT_S.to_bits());
                assert!(600.0 + floor > 600.0);
                assert_eq!(ceiling.to_bits(), (0.1_f64 * 0.003).to_bits());
            } else if card == "cell_2170_model3" {
                assert_eq!(floor.to_bits(), genesis_kernel::cell_2170_twin::FLOOR_DT_S.to_bits());
                assert_eq!(ceiling.to_bits(), genesis_kernel::cell_2170_twin::GATE_DT_S.to_bits());
                assert_eq!((floor, ceiling), (1.0, 1.0));
            } else if card == "creep_cmsx6" {
                // The creep card's floor is a locked constant of the card, 1 s.
                assert_eq!(floor.to_bits(), creep_cmsx6::FLOOR_DT_S.to_bits());
            } else if card == "solar_system" {
                // The solar system's floor is its ceiling divided by 4096.
                assert_eq!(floor.to_bits(), (ceiling / solar_system::GATE_OVER_FLOOR).to_bits());
                assert_eq!((floor, ceiling), (0.87890625, 3600.0));
            } else if card == "tire_eve_r19_ice" {
                // The tyre's floor is its ceiling of 2^22 s divided by 2^32.
                assert_eq!(floor.to_bits(), (ceiling / 4_294_967_296.0).to_bits());
                assert_eq!((floor, ceiling), (1.0 / 1_024.0, 4_194_304.0));
            } else if card == "fingerpad_p02_glass" {
                // The finger pad's floor is its ceiling of 0.01 s divided by 2^8.
                assert_eq!(floor.to_bits(), (ceiling / 256.0).to_bits());
                assert_eq!(ceiling, 0.01);
            } else if card == "cantilever_eth" {
                // The cantilever's floor is its ceiling of 0.25 ms divided by
                // 2^18, so that a step from rest at the amplitude ceiling moves
                // the deflection.
                assert_eq!(floor.to_bits(), (ceiling / 262_144.0).to_bits());
                assert_eq!(ceiling, 2.5e-4);
            } else if card == "los_jwst_fine_guide" {
                assert_eq!(floor.to_bits(), los_jwst_fine_guide::FLOOR_DT_S.to_bits());
                assert_eq!(ceiling.to_bits(), los_jwst_fine_guide::GATE_DT_S.to_bits());
                assert_eq!((floor, ceiling), (0.0635, 0.0645));
            } else if card == "link_gothenburg_10010" {
                // The link's floor is the gauge's minute over 2^20; its ceiling
                // is set by the rounding of the water built in one step.
                assert_eq!(floor.to_bits(), (60.0_f64 / 1_048_576.0).to_bits());
                assert_eq!(ceiling.to_bits(), link_gothenburg_10010::GATE_DT_S.to_bits());
            } else if card == "grasp_lift_glass_4tu" {
                // The grasped object's floor is its ceiling, a twentieth of a
                // radian of its ringing on the pads, divided by 2^10.
                assert_eq!(floor.to_bits(), (ceiling / 1024.0).to_bits());
            } else if card == "reactor_msre_pump" {
                // The molten-salt reactor's floor and ceiling are locked constants of the card.
                assert_eq!(floor.to_bits(), genesis_kernel::reactor_msre_pump::FLOOR_DT_S.to_bits());
                assert_eq!(ceiling.to_bits(), genesis_kernel::reactor_msre_pump::GATE_DT_S.to_bits());
            } else if card == "tokamak_jet_dte1" {
                // The twin's floor and ceiling are locked constants of the card.
                assert_eq!(floor.to_bits(), genesis_kernel::tokamak_jet_dte1_twin::FLOOR_DT_S.to_bits());
                assert_eq!(ceiling.to_bits(), genesis_kernel::tokamak_jet_dte1_twin::GATE_DT_S.to_bits());
            } else {
                assert_eq!(floor.to_bits(), (ceiling / 1_048_576.0).to_bits(), "{card}");
            }
            let body = describe_one(item);
            let dt = &body["dt_s"];
            assert_eq!(dt["floor_s"].as_f64().unwrap().to_bits(), floor.to_bits());
            assert_eq!(dt["ceiling_s"].as_f64().unwrap().to_bits(), ceiling.to_bits());
            assert!(
                dt["floor_from"].as_str().unwrap().starts_with(if card == "creep_cmsx6" {
                    "a locked 1 s"
                } else if card == "cell_2170_model3" {
                    "a locked 1 s"
                } else if card == "grid_gb" {
                    "a locked 1e-6 s"
                } else if card == "solar_system" {
                    "the ceiling divided by 4096"
                } else if card == "tire_eve_r19_ice" {
                    "the ceiling divided by 2^32"
                } else if card == "fingerpad_p02_glass" {
                    "the ceiling divided by 2^8"
                } else if card == "grasp_lift_glass_4tu" {
                    "the ceiling divided by 2^10"
                } else if card == "cantilever_eth" {
                    "the ceiling divided by 2^18"
                } else if card == "los_jwst_fine_guide" {
                    "a locked 0.0635 s"
                } else if card == "link_gothenburg_10010" {
                    "the gauge's minute, 60 s, divided by 2^20"
                } else if card == "tokamak_jet_dte1" || card == "reactor_msre_pump" {
                    "a locked 1e-6 s"
                } else if locked_floor {
                    "a locked 1e-9 s"
                } else {
                    "the ceiling divided by 2^20"
                }),
                "{card}"
            );
            assert!(!dt["ceiling_from"].as_str().unwrap().is_empty(), "{card}");
            assert!(dt["rule"].as_str().unwrap().contains("floor_dt_s"), "{card}");
            let rule = body["number_rule"].as_str().expect("number rule");
            assert!(rule.contains("32-bit frame slot"), "{card}");
            assert!(rule.contains("refuses some values inside"), "{card}");
        }
    }

    #[test]
    fn kernel_nyquist_matches_pi_over_omega() {
        let mount = resonance::DynamicOscillator::mount_10hz();
        assert_eq!(
            mount.nyquist_dt_s().unwrap().to_bits(),
            super::super::nyquist_limit_s(mount.omega_rad_s).to_bits()
        );
        let machine = swing::SwingMachine::synchronous_60hz();
        assert_eq!(
            machine.nyquist_dt_s().unwrap().to_bits(),
            super::super::nyquist_limit_s(machine.omega_electrical_rad_s).to_bits()
        );
        let drive = tesseract::Tesseract::drive_mode();
        assert_eq!(
            drive.nyquist_dt_s().unwrap().to_bits(),
            super::super::nyquist_limit_s(drive.omega_rad_s).to_bits()
        );
    }

    const GENERATOR_SOURCE: &str = include_str!("../../genesis_kernel/src/generator_mmrtg_f1.rs");
    const CAPSULE_SOURCE: &str = include_str!("../../genesis_kernel/src/capsule_stardust.rs");
    const REACTOR_SOURCE: &str = include_str!("../../genesis_kernel/src/reactor_spert3_twin.rs");
    const REACTOR_TRIGA_SOURCE: &str =
        include_str!("../../genesis_kernel/src/reactor_triga_jsi.rs");
    const CELL_2170_SOURCE: &str = include_str!("../../genesis_kernel/src/cell_2170_twin.rs");
    const CANTILEVER_SOURCE: &str = include_str!("../../genesis_kernel/src/cantilever_eth.rs");
    const CREEP_SOURCE: &str = include_str!("../../genesis_kernel/src/creep_cmsx6.rs");
    const CAPSULE_INSIGHT_SOURCE: &str =
        include_str!("../../genesis_kernel/src/capsule_insight.rs");
    const CRATER_SOURCE: &str = include_str!("../../genesis_kernel/src/crater_ryugu_sci.rs");
    const LINK_SOURCE: &str = include_str!("../../genesis_kernel/src/link_gothenburg_10010.rs");

    /// The link lists every variant of its `StepReject`, in the kernel's
    /// order, with the kernel's sentence word for word.
    #[test]
    fn the_link_refusals_carry_the_sentences_from_the_card() {
        let documented = documented_rejects(LINK_SOURCE);
        let card = find("signal", "link_gothenburg_10010").expect("the link is served");
        assert_eq!(documented.len(), 11);
        assert_eq!(card.rejects.len(), documented.len());
        for ((name, sentence), listed) in documented.iter().zip(card.rejects) {
            assert_eq!(name, listed);
            assert_eq!(
                reject_meaning("link_gothenburg_10010", name),
                Some(sentence.as_str()),
                "{name}"
            );
        }
    }

    /// The link publishes the bounds the kernel's constants give, each value
    /// at a bound makes a start or lands a step and the next number does
    /// not, the step ceiling is the gauge's minute, the flag and the frame
    /// slots name the state they come from, and the line an agent reads in
    /// the list says which constants are fitted to the link itself.
    #[test]
    fn the_link_publishes_the_kernel_bounds_and_says_what_is_fitted() {
        let card = find("signal", "link_gothenburg_10010").expect("the link is served");
        let link = link_gothenburg_10010::LinkGothenburg10010::sublink_587();
        let body = describe_one(card);
        let bounds = |field: &Value| {
            (
                field["minimum"].as_f64().unwrap().to_bits(),
                field["maximum"].as_f64().map(f64::to_bits),
                field["minimum_exclusive"].as_bool().unwrap(),
                field["maximum_exclusive"].as_bool().unwrap(),
            )
        };
        assert!(card.specimen.contains("ITU-R P.838-3 with nothing fitted"));
        assert!(card.specimen.contains("Fitted on this link's own readings in the 18 rain events of June and July 2015"));
        assert_eq!(body["phase_fields"].as_array().unwrap().len(), 4);
        assert_eq!(body["load_fields"].as_array().unwrap().len(), 1);

        let minute = &body["phase_fields"][2];
        assert_eq!(minute["name"], "gauge_minute_elapsed_s");
        assert_eq!(minute["unit"], "s");
        assert_eq!(bounds(minute), (0.0_f64.to_bits(), Some(60.0_f64.to_bits()), false, true));
        let held = &body["phase_fields"][3];
        assert_eq!(held["name"], "gauge_minute_rain_mm_h");
        assert_eq!(held["unit"], "mm/h");
        assert_eq!(bounds(held), (0.0_f64.to_bits(), None, false, false));
        assert!(held.to_string().contains("refused as NegativeRainRate"), "{held}");
        assert!(minute.to_string().contains("No limit depends on it"), "{minute}");

        let dry = &body["phase_fields"][0];
        assert_eq!(dry["name"], "dry_path_loss_db");
        assert_eq!(dry["unit"], "dB");
        assert_eq!(
            bounds(dry),
            (link_gothenburg_10010::DRY_PATH_LOSS_FLOOR_DB.to_bits(), None, false, false)
        );
        let water = &body["phase_fields"][1];
        assert_eq!(water["name"], "antenna_water_db");
        assert_eq!(water["unit"], "dB");
        assert_eq!(bounds(water), (0.0_f64.to_bits(), Some(2.13_f64.to_bits()), false, false));
        let rain = &body["load_fields"][0];
        assert_eq!(rain["name"], "rain_rate_mm_h");
        assert_eq!(rain["unit"], "mm/h");
        assert_eq!(bounds(rain), (0.0_f64.to_bits(), None, false, false));
        assert!(rain.to_string().contains("refused as NegativeRainRate"), "{rain}");
        assert!(rain.to_string().contains("whose rain reached 54 mm/h"), "{rain}");

        let next = |value: f64| f64::from_bits(value.to_bits() + 1);
        let below = |value: f64| f64::from_bits(value.to_bits() - 1);
        let start = link_gothenburg_10010::LinkState::try_new;
        let floor = link_gothenburg_10010::DRY_PATH_LOSS_FLOOR_DB;
        assert!(start(&link, floor, 0.0).is_some());
        assert!(start(&link, 90.0, 2.13).is_some());
        assert!(start(&link, below(floor), 0.0).is_none());
        assert!(start(&link, 52.0, next(2.13)).is_none());
        let mut state = start(&link, 52.0, 0.0).unwrap();
        let mut frame = genesis_kernel::frame::LastStateFrame64 {
            t: 0.0,
            pos: [0.0; 3],
            vel: [0.0; 3],
            force_torque: 0.0,
            residual: 0.0,
            flags: 0,
            proof: [0; 16],
        };
        let load = |rain_rate_mm_h| link_gothenburg_10010::LinkLoad { rain_rate_mm_h };
        assert_eq!(
            link_gothenburg_10010::step(&link, &mut state, &mut frame, load(next(84.0)), 60.0),
            Ok(())
        );
        assert_eq!(
            link_gothenburg_10010::step(&link, &mut state, &mut frame, load(-1.0), 60.0),
            Err(link_gothenburg_10010::StepReject::NegativeRainRate)
        );
        assert_eq!(
            link_gothenburg_10010::step(&link, &mut state, &mut frame, load(84.0), 60.0),
            Ok(())
        );
        assert_eq!(
            link_gothenburg_10010::step(&link, &mut state, &mut frame, load(below(6.0)), 60.0),
            Ok(())
        );
        assert_eq!(
            link_gothenburg_10010::step(&link, &mut state, &mut frame, load(6.0), 60.0),
            Ok(())
        );

        let dt = &body["dt_s"];
        assert_eq!(dt["ceiling_s"].as_f64().unwrap(), link_gothenburg_10010::GATE_DT_S);
        assert_eq!(dt["floor_s"].as_f64().unwrap().to_bits(), (60.0_f64 / 1_048_576.0).to_bits());
        assert!(dt["nyquist_s"].is_null());
        assert!(dt["rule"].as_str().unwrap().contains("the gauge's minute"));
        assert!(dt["rule"].as_str().unwrap().contains("set by the step's arithmetic and not by the data"));
        assert!(dt["rule"].as_str().unwrap().contains("each energy is its power integrated exactly over the step"));

        let flags = body["advisory_flags"].as_array().unwrap();
        assert_eq!(flags.len(), 1);
        assert_eq!(flags[0]["name"], "FLAG_RAINING");
        assert_eq!(flags[0]["mask"], 1);
        assert_eq!(flags[0]["seals"], true);
        assert_eq!(flags[0]["state"], "raining");
        let slot = |name: &str| {
            body["frame_slots"]
                .as_array()
                .unwrap()
                .iter()
                .find(|slot| slot["slot"] == name)
                .unwrap()
                .clone()
        };
        for (name, state, unit) in [
            ("pos[0]", "path_loss_db", "dB"),
            ("pos[1]", "rain_attenuation_db", "dB"),
            ("pos[2]", "antenna_water_db", "dB"),
            ("vel[0]", "received_level_dbm", "dBm"),
            ("vel[1]", "received_power_w", "W"),
            ("vel[2]", "received_energy_j", "J"),
            ("force_torque", "rain_rate_margin_mm_h", "mm/h"),
            ("residual", "residual_conservation_error", "1"),
        ] {
            assert_eq!(slot(name)["state"], state, "{name}");
            assert_eq!(slot(name)["unit"], unit, "{name}");
        }

        assert_eq!(
            serde_json::to_string(&body["example_compute_step"]).unwrap(),
            r#"{"domain":"signal","card":"link_gothenburg_10010","parameters":{"dry_path_loss_db":52.3,"antenna_water_db":0.0,"gauge_minute_elapsed_s":0.0,"gauge_minute_rain_mm_h":0.0,"rain_rate_mm_h":12.0,"dt_s":60.0}}"#
        );
        assert_eq!(
            serde_json::to_string(&body["example_run_trajectory"]).unwrap(),
            r#"{"domain":"signal","card":"link_gothenburg_10010","initial":{"dry_path_loss_db":52.3,"antenna_water_db":0.0,"gauge_minute_elapsed_s":0.0,"gauge_minute_rain_mm_h":0.0},"dt_s":60.0,"segments":[{"steps":10,"load":{"rain_rate_mm_h":12.0}}],"record_every":1}"#
        );
    }

    /// The link's validation record: the citation, what is fitted and what
    /// is not, the month held back, the hash of the pre-registration with
    /// the three lines, the forecast named as a forecast, what the held-back
    /// month does and does not test, and no word of a proof.
    #[test]
    fn the_link_validation_record_states_the_data_the_lines_and_the_limits() {
        let card = find("signal", "link_gothenburg_10010").expect("the link is served");
        let record = &describe_one(card)["validation"];
        let part = |name: &str| record[name].as_str().unwrap().to_string();
        let object = part("object");
        for words in ["link 10010, sublink 587", "32.445 GHz", "vertical polarisation", "1.25841 km", "Barlastplatsen 37 m from the path"] {
            assert!(object.contains(words), "{words}: {object}");
        }
        let data = part("data");
        for words in [
            "The OpenMRG data set (10.5281/zenodo.7107689)",
            "Recommendation ITU-R P.838-3 at this frequency and polarisation, with nothing fitted",
            "fitted on this link's own readings in the 18 rain events of June and July",
            "Every reading of August is outside the fit",
        ] {
            assert!(data.contains(words), "{words}: {data}");
        }
        let pass_line = part("pass_line");
        for words in [
            "f60bfcc70553f95fef5d35963b6550120de3c67e712c057e40c5d37c51784625",
            "root mean square error at most 0.90 dB",
            "at most 0.80 of the error of P.838 alone",
            "between 0.75 and 1.25 of the measured sum",
        ] {
            assert!(pass_line.contains(words), "{words}: {pass_line}");
        }
        let limits = part("limits");
        assert!(limits.starts_with("On the 18 events in the fit,"), "{limits}");
        for words in [
            "0.667 dB, against 1.028 dB for P.838 alone and 1.589 dB for no attenuation",
            "interpolation in rain rate, later in time",
            "No other link, frequency, polarisation or season, and no snow, is in the comparison",
            "one gauge at one end of the path",
            "7.98 dB above free space between two ideal 0.3 m apertures",
            "one receiver step, 0.3 dB, of error in it puts the summed attenuation outside its band, at 1.69",
            "a single dry level for the whole month, 52.30 dB, gives 0.644",
            "must not take the card's peak as the measured peak",
            "the card's five-minute peak was the lower in seven of the nine events, by up to 2.2 dB",
            "its one-minute peak was the lower in six, by up to 2.8 dB, and the higher in three, by up to 3.3 dB",
            "no energy is in the comparison",
            "a rate above 84 mm/h, a rate above 0 and below 6 mm/h, rain changed inside a gauge minute, a step longer than 60 s",
            "none of those is in the comparison",
        ] {
            assert!(limits.contains(words), "{words}: {limits}");
        }
        assert_eq!(part("result"), LINK_RESULT);
        assert_ne!(part("result"), NOT_COMPARED);
        let result = part("result");
        for words in [
            "On the rain events of August 2015, in one run of each event through the engine.",
            "0.581 dB (target 0.90)",
            "0.585 of the error of P.838 alone, 0.994 dB (target 0.80)",
            "1.069 of the measured sum",
            "nine are compared and none was refused",
            "In two of the nine events P.838 alone was closer than the card",
            "finds 21 gauge events in August: eleven are under 1.0 mm",
            "one of 15.8 mm that starts 2015-08-31T17:37Z is outside the rule",
            "0.622 dB, 0.528 and 0.974",
            "The card's five-minute peak is below the measured five-minute peak in seven of the nine events, by up to 2.2 dB",
            "its peak is below the measured peak in six of the nine events, by up to 2.8 dB, and above it in three, by up to 3.3 dB",
            "lowers the error from 0.580 to 0.516 dB",
            "sublink 588",
            "0.794 dB, no better than P.838 alone, with a summed attenuation of 2.30",
            "A second program, on the same machine and from its own cut of the archive, recomputes the same results",
        ] {
            assert!(result.contains(words), "{words}: {result}");
        }
        let data = part("data");
        for words in [
            "k = 0.27282714286921533 and alpha = 0.8947805998167349 at 32.445 GHz, vertical",
            "a ceiling of 2.13 dB, a wetting time of 81 s and a drying time of 404 s",
        ] {
            assert!(data.contains(words), "{words}: {data}");
        }
        for words in [
            "all within the targets",
        ] {
            assert!(result.contains(words), "{words}: {result}");
        }
        assert!(!record.to_string().contains("seal"));
    }
    const LOS_SOURCE: &str = include_str!("../../genesis_kernel/src/los_jwst_fine_guide.rs");

    /// The line-of-sight card lists every variant of its `StepReject`, in the
    /// kernel's order, with the kernel's sentence word for word; its three
    /// flags are the kernel's bits; its bounds are the kernel's constants; and
    /// its examples are the first frames of the kernel's reference run.
    #[test]
    fn the_line_of_sight_card_carries_the_kernels_refusals_flags_bounds_and_frames() {
        let documented = documented_rejects(LOS_SOURCE);
        let card = find("aerospace", "los_jwst_fine_guide").expect("the line of sight is served");
        assert_eq!(documented.len(), 13);
        assert_eq!(card.rejects.len(), documented.len());
        for ((name, sentence), listed) in documented.iter().zip(card.rejects) {
            assert_eq!(name, listed);
            assert_eq!(
                reject_meaning("los_jwst_fine_guide", name),
                Some(sentence.as_str()),
                "{name}"
            );
        }
        let masks: Vec<u64> = card.advisory.iter().map(|flag| flag.mask).collect();
        assert_eq!(
            masks,
            [
                los_jwst_fine_guide::FLAG_NO_MEASUREMENT,
                los_jwst_fine_guide::FLAG_POINTING_DISTURBED,
                los_jwst_fine_guide::FLAG_BEYOND_FOUR_SIGMA,
                los_jwst_fine_guide::FLAG_RECOVERING,
                los_jwst_fine_guide::FLAG_COUNT_RATE_BEYOND_COMPARED
            ]
        );
        let field = |name: &str| card.fields.iter().find(|f| f.name == name).expect("field");
        for lock in ["lock_x_arcsec", "lock_y_arcsec"] {
            let half = los_jwst_fine_guide::LOCK_FIELD_HALF_WIDTH_ARCSEC;
            assert_eq!(field(lock).minimum, Some(-half));
            assert_eq!(field(lock).maximum, Some(half));
            assert!(field(lock).bound.contains("LockOutsideSensorField"));
        }
        assert!(field("lock_x_arcsec").bound.contains(&token(los_jwst_fine_guide::LOCK_X_MIN_ARCSEC)));
        assert!(field("lock_y_arcsec").bound.contains(&token(los_jwst_fine_guide::LOCK_Y_MAX_ARCSEC)));
        assert!(!card.nyquist_rule.contains("refused as MeasurementGapOutsideCard"));
        assert!(!card.nyquist_rule.contains("no limit in time or in frames"));
        let quiet = field("quiet_before_start_s");
        assert_eq!(quiet.minimum, Some(0.0));
        assert_eq!(quiet.maximum, None);
        assert!(quiet.bound.contains("QuietBeforeStartBelowZero") && quiet.bound.contains("it cannot confirm it"));
        assert_eq!(field("measured").minimum, Some(0.0));
        assert_eq!(field("measured").maximum, Some(1.0));
        assert_eq!(field("counts_per_s").minimum, Some(0.0));
        assert_eq!(field("counts_per_s").maximum, None);
        assert!(field("counts_per_s").bound.contains("CountRateNotPositive"));
        assert!(field("counts_per_s").bound.contains("FLAG_COUNT_RATE_BEYOND_COMPARED"));
        for centroid in ["centroid_x_arcsec", "centroid_y_arcsec"] {
            assert_eq!(field(centroid).minimum, None);
            assert_eq!(field(centroid).maximum, None);
            assert!(field(centroid).bound.contains("OffsetOutsideCard"));
        }
        assert!(LOS_SOURCE.contains("slow_mas: 1.03257"));
        // The examples are the first eight frames of the kernel's reference run.
        let reference = include_str!("../../genesis_kernel/src/los_jwst_fine_guide_reference.rs");
        assert!(reference.contains("const LOCK_X_ARCSEC: f64 = 17.3039684;"));
        assert!(reference.contains("const LOCK_Y_ARCSEC: f64 = -15.2342768;"));
        assert!(reference.contains("// `(measured, centroid_x_arcsec, centroid_y_arcsec, counts_per_s)` per frame.\nconst FRAMES: [(f64, f64, f64, f64); 1252] = [\n    (1.0, 17.3038845, -15.2345047, 553666.4),\n    (1.0, 17.3043156, -15.234498, 548737.25),\n    (1.0, 17.3045425, -15.234539, 549291.063),\n    (1.0, 17.3039627, -15.23488, 553444.8),\n    (1.0, 17.303978, -15.2349653, 547832.7),\n    (1.0, 17.3033333, -15.2353725, 547648.063),\n    (1.0, 17.3037148, -15.2351408, 547629.563),\n    (1.0, 17.303875, -15.2350893, 546318.9),\n"));
        assert_eq!(LOS_EXAMPLE_FRAMES[0], [1.0, 17.3038845, -15.2345047, 553666.4]);
        assert_eq!(LOS_EXAMPLE_FRAMES[7], [1.0, 17.303875, -15.2350893, 546318.9]);
        let body = describe_one(card);
        let run = &body["example_run_trajectory"];
        assert_eq!(run["segments"].as_array().unwrap().len(), 8);
        assert_eq!(run["segments"][3]["load"]["measured"], 1.0);
        assert_eq!(run["initial"]["quiet_before_start_s"], 30.016);
        assert_eq!(run["dt_s"], 0.064);
        // The summary an agent reads in the list says the constants are fitted to this observatory.
        assert!(card.specimen.contains("fitted to this observatory's own centroids"));
        let validation = &body["validation"];
        assert_eq!(validation["result"], LOS_RESULT);
    }
    const TOKAMAK_SOURCE: &str = include_str!("../../genesis_kernel/src/tokamak_jet_dte1_twin.rs");

    /// `(variant, doc sentence)` for every variant of a card's `StepReject`,
    /// read from the kernel source.
    fn documented_rejects(source: &str) -> Vec<(String, String)> {
        let start = source
            .find("pub enum StepReject {")
            .expect("StepReject is in the card");
        let mut documented = Vec::new();
        let mut doc = String::new();
        for line in source[start..].lines().skip(1) {
            if line == "}" {
                break;
            }
            let text = line.trim_start();
            if let Some(sentence) = text.strip_prefix("/// ") {
                if !doc.is_empty() {
                    doc.push(' ');
                }
                doc.push_str(sentence);
            } else if let Some(name) = text.strip_suffix(',') {
                documented.push((name.to_string(), doc.clone()));
                doc.clear();
            }
        }
        documented
    }

    const SATELLITE_SOURCE: &str = include_str!("../../genesis_kernel/src/satellite_lageos1.rs");

    /// The lines of the card's header under one `# ` heading, joined by
    /// single spaces, up to the next blank comment line.
    fn header_paragraph(source: &str, starts_with: &str) -> String {
        let mut words = Vec::new();
        let mut inside = false;
        for line in source.lines().skip_while(|line| !line.starts_with("//!")) {
            let Some(text) = line.strip_prefix("//!") else {
                break;
            };
            let text = text.trim();
            if !inside {
                inside = text.starts_with(starts_with);
            }
            if inside {
                if text.is_empty() {
                    break;
                }
                words.push(text);
            }
        }
        words.join(" ")
    }

    /// The satellite's refusals, its start-state warning and its frame rule
    /// are the card's own words.
    #[test]
    fn the_satellite_description_carries_the_words_of_the_card() {
        let documented = documented_rejects(SATELLITE_SOURCE);
        let card = find("aerospace", "satellite_lageos1").expect("the satellite is served");
        assert_eq!(documented.len(), 10);
        assert_eq!(card.rejects.len(), documented.len());
        for ((name, sentence), listed) in documented.iter().zip(card.rejects) {
            assert_eq!(name, listed);
            assert_eq!(
                reject_meaning("satellite_lageos1", name),
                Some(sentence.as_str()),
                "{name}"
            );
        }

        let body = describe_one(card);
        assert_eq!(
            body["start_state"],
            header_paragraph(SATELLITE_SOURCE, "The published orbit files")
        );
        // The frame as the compared runs built it (lageos1_orbit/analysis/
        // 10_engine_comparison.py, earth_orientation.py), and what a
        // crust-fixed state costs through the engine, not the frame's cost
        // with no flight (terminal_bench_checks_2026-10-16/lageos/out/budget.json).
        let start_state = body["start_state"].as_str().expect("start state");
        for words in [
            "IAU 2006/2000A precession and nutation with the celestial pole offsets dX and dY of IERS Bulletin A, held fixed in the celestial frame (GCRS)",
            "the locked rate 7.29211514670698e-5 rad/s",
            "ocean-tide terms of the IERS Conventions (2010), routine ORTHO_EOP",
            "Then the locked rate's ω × r is taken off.",
            "69 to 81 m off the published orbit after one revolution, 374 to 385 m after one day",
            "the five leap seconds since then are not counted, so 2017-01-01 00:00 UTC is 536500800 s",
        ] {
            assert!(start_state.contains(words), "{words}");
        }
        assert!(!body.to_string().contains("21 to 22 m") && !body.to_string().contains("about 20 m off"));
        assert_eq!(satellite_lageos1::EARTH_ROTATION_RATE_RAD_S, 7.29211514670698e-5);
        assert_eq!(satellite_lageos1::EPOCH_MIN_S, 536_500_800.0);
        let rule = body["frame_rule"].as_str().expect("frame rule");
        assert!(
            rule.contains(&header_paragraph(SATELLITE_SOURCE, "A reader adds")),
            "{rule}"
        );
        assert!(rule.contains("REFERENCE_RADIUS_M is 12270000 m"), "{rule}");
        assert_eq!(satellite_lageos1::REFERENCE_RADIUS_M, 12_270_000.0);

        // The slot table, row for row as the header prints it.
        let slots: Vec<String> = card
            .slots
            .iter()
            .filter(|slot| slot.slot != "proof")
            .map(|slot| format!("{} {}", slot.slot, slot.holds))
            .collect();
        let start = SATELLITE_SOURCE.find("//! t             t_s, 64-bit").expect("slots");
        let printed: Vec<String> = SATELLITE_SOURCE[start..]
            .lines()
            .take(6)
            .map(|line| {
                let text = line.trim_start_matches("//!").trim();
                let (slot, holds) = text.split_once(' ').expect("slot and holds");
                format!("{slot} {}", holds.trim())
            })
            .collect();
        assert_eq!(slots, printed);

        // Only the satellite and the solar system carry a start-state note, and
        // only the satellite this reader's rule.
        for other in all()
            .iter()
            .filter(|other| other.card != "satellite_lageos1" && other.card != "solar_system")
        {
            let other_body = describe_one(other);
            assert!(other_body.get("start_state").is_none(), "{}", other.card);
            assert!(!other_body["frame_rule"].as_str().unwrap().contains("REFERENCE_RADIUS_M"), "{}", other.card);
        }
    }

    /// The satellite's examples start from a compared start state: one step
    /// at the ceiling, and one revolution.
    #[test]
    fn the_satellite_examples_are_one_step_and_one_revolution_of_a_compared_orbit() {
        let card = find("aerospace", "satellite_lageos1").expect("the satellite is served");
        let body = describe_one(card);
        let state = satellite_example_state();
        assert_eq!(state.epoch_utc_s(), 841_924_800.0);
        assert_eq!(state.forecast_elapsed_s(), 0.0);
        assert!(state.semi_major_axis_m > 12_265_000.0 && state.semi_major_axis_m < 12_275_000.0);
        let step = &body["example_compute_step"];
        let run = &body["example_run_trajectory"];
        for (example, at) in [(step, "parameters"), (run, "initial")] {
            assert_eq!(
                example[at]["epoch_utc_s"].as_f64().unwrap().to_bits(),
                state.epoch_utc_s().to_bits()
            );
            assert_eq!(example[at]["forecast_elapsed_s"], 0.0);
            for axis in 0..3 {
                assert_eq!(
                    example[at]["position_m"][axis].as_f64().unwrap().to_bits(),
                    state.position_m[axis].to_bits()
                );
                assert_eq!(
                    example[at]["velocity_m_s"][axis].as_f64().unwrap().to_bits(),
                    state.velocity_m_s[axis].to_bits()
                );
            }
        }
        assert_eq!(step["parameters"]["dt_s"], 10.0);
        assert_eq!(run["dt_s"], 10.0);
        assert_eq!(run["segments"], json!([{ "steps": 1356, "load": {} }]));
        assert_eq!(run["record_every"], 36);
        assert!(body["load_fields"].as_array().unwrap().is_empty());
        assert_eq!(card.ceiling_s, 10.0);
        assert_eq!(card.floor_s, 10.0 / 1_048_576.0);
        assert!(card.nyquist_s.is_none());
    }

    /// The capsule's refusals, each with the sentence from its doc comment
    /// in the kernel source, word for word.
    #[test]
    fn the_capsule_refusals_carry_the_sentences_from_the_card() {
        let documented = documented_rejects(CAPSULE_SOURCE);
        let card = find("aerospace", "capsule_stardust").expect("the capsule is served");
        assert_eq!(documented.len(), 11);
        assert_eq!(card.rejects.len(), documented.len());
        for ((name, sentence), listed) in documented.iter().zip(card.rejects) {
            assert_eq!(name, listed);
            assert_eq!(
                reject_meaning("capsule_stardust", name),
                Some(sentence.as_str()),
                "{name}"
            );
        }
    }

    /// The reactor lists every variant of its `StepReject`, in the kernel's
    /// order, with the kernel's sentence word for word. (0.15.0: the kernel is
    /// the E-core twin's, `reactor_spert3_twin`.)
    #[test]
    fn the_reactor_refusals_carry_the_sentences_from_the_card() {
        let documented = documented_rejects(REACTOR_SOURCE);
        let card = find("energy", "reactor_spert3").expect("the reactor is served");
        assert_eq!(documented.len(), 12);
        assert_eq!(card.rejects.len(), documented.len());
        for ((name, sentence), listed) in documented.iter().zip(card.rejects) {
            assert_eq!(name, listed);
            assert_eq!(
                reject_meaning("reactor_spert3", name),
                Some(sentence.as_str()),
                "{name}"
            );
        }
    }

    /// The JSI TRIGA reactor lists every variant of its `StepReject`, in the
    /// kernel's order, with the kernel's sentence word for word.
    #[test]
    fn the_triga_reactor_refusals_carry_the_sentences_from_the_card() {
        let documented = documented_rejects(REACTOR_TRIGA_SOURCE);
        let card = find("energy", "reactor_triga_jsi").expect("the TRIGA reactor is served");
        assert_eq!(documented.len(), 18);
        assert_eq!(card.rejects.len(), documented.len());
        for ((name, sentence), listed) in documented.iter().zip(card.rejects) {
            assert_eq!(name, listed);
            assert_eq!(
                reject_meaning("reactor_triga_jsi", name),
                Some(sentence.as_str()),
                "{name}"
            );
        }
        // The numbers the sentences carry are the card's own limits.
        for (name, limit) in [
            ("StartTemperatureNotLiquidWater", reactor_triga_jsi::WATER_FREEZING_K),
            ("StartTemperatureNotLiquidWater", reactor_triga_jsi::WATER_BOILING_K),
            ("ReactivityAboveRodWorth", reactor_triga_jsi::ROD_WORTH_DOLLAR),
            ("DtAboveStateCeiling", reactor_triga_jsi::STATE_PHASE_STEP),
        ] {
            let sentence = reject_meaning("reactor_triga_jsi", name).unwrap();
            assert!(sentence.contains(&format!(" {limit} ")), "{name}: {sentence}");
        }
        // No refusal of the card is a wall at the edge of what was compared.
        for name in card.rejects {
            assert!(!name.contains("OutsideCard") || *name == "RodTableOutsideCard" || *name == "ScramValueOutsideCard", "{name}");
            assert!(!name.contains("Beyond") && !name.contains("Compared"), "{name}");
        }
        for core in reactor_triga_jsi::ROD_TABLE_CORE {
            let sentence = reject_meaning("reactor_triga_jsi", "RodTableOutsideCard").unwrap();
            assert!(sentence.contains(&format!("{core}")), "{sentence}");
        }
    }

    /// The JSI TRIGA reactor publishes the kernel's bounds, its three rod
    /// tables with their fitted scale and offset, its three flags and its
    /// frame as the card's `publish` writes it. Its one-line summary, the spec
    /// sheet, carries the fitted constants, the compared range, the numbers of
    /// the comparison outside the fit, the gap and what the fuel temperature
    /// is compared with. Its record names every fitted constant with its value,
    /// what is sourced, the numbers beside their targets, and no liturgy.
    /// The five known defects of engine 0.6.0 are fixed in this engine, so no
    /// card states a known defect in its limits.
    #[test]
    fn each_known_defect_is_stated_in_the_limits_of_its_card_and_nowhere_else() {
        let with: Vec<&str> = all().iter().map(|card| card.card).filter(|card| known_defect(card).is_some()).collect();
        assert!(with.is_empty(), "{with:?}");
        for card in all() {
            let limits = validation(card.card, card.specimen)["limits"].as_str().expect("limits").to_string();
            assert_eq!(limits.contains("Known defect, not fixed in engine 0.6.0"), known_defect(card.card).is_some(), "{}", card.card);
            if let Some(defect) = known_defect(card.card) {
                assert!(limits.ends_with(defect), "{}", card.card);
            }
        }
        let jet = validation("tokamak_jet_dte1", "")["limits"].as_str().unwrap().to_string();
        // 0.15.0: no served model is superseded, so the JET card's limits name no successor.
        assert!(!jet.contains("superseded"), "{jet}");
        // The heat share of the accumulator is carried into each start, so its limits state no known limit.
        assert!(!ACCUMULATOR_SBO450_LIMITS.contains("Known limit"));
        assert!(ACCUMULATOR_SBO450_LIMITS.ends_with("The corrected card and its run are the first party's alone; no second program has recomputed them."));
        let cell = find("energy", "cell_18650pf").unwrap();
        assert!(cell.specimen.contains("computes any held discharge current above 0 A from any state of charge from 0.03 to 1 with the same equations, each step outside 2.89817 to 2.90065 A flagged."));
    }

    #[test]
    fn the_triga_reactor_publishes_the_kernel_bounds_tables_flags_frame_and_record() {
        let card = find("energy", "reactor_triga_jsi").expect("the TRIGA reactor is served");
        // 0.15.0: one model per real object, 36 cards, each in its 0.14.3 place.
        assert_eq!(all()[30].card, "reactor_triga_jsi");
        assert_eq!(all()[31].card, "reactor_spert3_ccore");
        assert_eq!(all().last().map(|last| last.card), Some("reactor_msre_pump"));
        assert_eq!(all().len(), 36);
        assert_eq!(card.specimen, crate::spec_sheet::of("reactor_triga_jsi").unwrap().paragraph());
        for words in [
            "Fitted on 36 of the 43 recorded pulses",
            "ten constants",
            "one thermocouple of one element",
            "2.0 to 3.0 printed dollars",
            "below 2.0 dollars peak power reads 29, 21 and 9 percent low",
            "up to the rod's whole worth of 4.16 printed dollars",
            "cores 215, 217 and 231",
            "peak power within 1.21 RMS (target 1.5",
            "peak power reads 44 and 23 percent low on the two rows of 22 March 2018",
            "(ReactivityChangedDuringRun)",
        ] {
            assert!(card.specimen.contains(words), "the summary lacks: {words}");
        }
        let reactor = reactor_triga_jsi::ReactorTrigaJsi::core_56_pulse_mode();
        let body = describe_one(card);
        let bits = |value: &Value| value.as_f64().unwrap().to_bits();
        let phase = &body["phase_fields"];
        assert_eq!(phase.as_array().unwrap().len(), 2);
        assert_eq!(phase[0]["name"], "start_temperature_k");
        assert_eq!(bits(&phase[0]["minimum"]), reactor_triga_jsi::WATER_FREEZING_K.to_bits());
        assert_eq!(bits(&phase[0]["maximum"]), reactor_triga_jsi::WATER_BOILING_K.to_bits());
        assert_eq!(phase[0]["minimum_exclusive"], false);
        assert_eq!(phase[0]["maximum_exclusive"], false);
        let start_words = phase[0]["bound"].as_str().unwrap();
        for (core, (low, high)) in reactor_triga_jsi::ROD_TABLE_CORE.iter().zip(reactor_triga_jsi::START_TEMPERATURE_K_BY_TABLE) {
            assert!(start_words.contains(&format!("{low:.2} to {high:.2} K")), "{start_words}");
            assert!(start_words.contains(&format!("on core {core}")), "{start_words}");
        }
        assert!(start_words.contains("StartTemperatureNotLiquidWater"), "{start_words}");
        assert!(start_words.contains("no comparison isolates the dependence on the starting temperature"), "{start_words}");
        assert_eq!(phase[1]["name"], "rod_table_core");
        assert_eq!(bits(&phase[1]["minimum"]), 215.0_f64.to_bits());
        assert_eq!(bits(&phase[1]["maximum"]), 231.0_f64.to_bits());
        let table_words = phase[1]["bound"].as_str().unwrap();
        assert!(table_words.contains("exactly one of 215, 217 and 231"), "{table_words}");
        for table in 0..3 {
            assert!(
                table_words.contains(&format!(
                    "core {}: {} and {}",
                    reactor.rod_table_core[table], reactor.dollar_scale[table], reactor.dollar_offset[table]
                )),
                "{table_words}"
            );
        }
        assert!(table_words.contains("RodTableOutsideCard"));
        let load = &body["load_fields"];
        assert_eq!(load.as_array().unwrap().len(), 2);
        assert_eq!(load[0]["name"], "reactivity_inserted_dollar");
        assert_eq!(bits(&load[0]["minimum"]), 0.0_f64.to_bits());
        assert_eq!(load[0]["minimum_exclusive"], true);
        assert_eq!(bits(&load[0]["maximum"]), reactor_triga_jsi::ROD_WORTH_DOLLAR.to_bits());
        assert_eq!(load[0]["maximum_exclusive"], false);
        let reactivity_words = load[0]["bound"].as_str().unwrap();
        for words in ["ReactivityNotPositive", "ReactivityAboveRodWorth", "Compared at 2.0 to 3.0 printed dollars (fitted 1.4 to 3.0)", "29, 21 and 9 percent low in peak power at 1.4, 1.6 and 1.8 dollars", "44, 21 and 13 percent", "nothing unfitted lies above 2.6 dollars"] {
            assert!(reactivity_words.contains(words), "{words}: {reactivity_words}");
        }
        let scram_words = load[1]["bound"].as_str().unwrap();
        for words in ["No compared scram came while the transient rod was moving; the 8 dollars of the scram is an assumption", "On the compared pulses the rods dropped 1.51 to 7.45 s after the peak", "(1.510 to 7.446 s, pulses 219 and 330)", "The longest compared hold was 7.45 s after the peak; a 3.0 dollar pulse with the rods held reaches 600 K after 5.2 to 6.4 s on cores 215 and 231"] {
            assert!(scram_words.contains(words), "{words}: {scram_words}");
        }
        assert_eq!(load[1]["name"], "rods_dropped");
        assert_eq!(bits(&load[1]["minimum"]), 0.0_f64.to_bits());
        assert_eq!(bits(&load[1]["maximum"]), 1.0_f64.to_bits());
        assert!(load[1]["bound"].as_str().unwrap().contains("The card does not decide when the rods drop"));
        let flags = body["advisory_flags"].as_array().unwrap();
        assert_eq!(flags.len(), 3);
        for (flag, (name, mask, state)) in flags.iter().zip([
            ("FLAG_PROMPT_SUPERCRITICAL", reactor_triga_jsi::FLAG_PROMPT_SUPERCRITICAL, "prompt_supercritical"),
            ("FLAG_ROD_MOVING", reactor_triga_jsi::FLAG_ROD_MOVING, "rod_moving"),
            ("FLAG_RODS_DROPPED", reactor_triga_jsi::FLAG_RODS_DROPPED, "rods_dropped"),
        ]) {
            assert_eq!(flag["name"], name);
            assert_eq!(flag["mask"], mask);
            assert_eq!(flag["state"], state);
            assert_eq!(flag["seals"], true);
        }
        for (line, slot) in [
            ("frame.pos[0] = state.power_mw as f32;", "pos[0]"),
            ("frame.pos[1] = state.energy_mws as f32;", "pos[1]"),
            ("frame.pos[2] = state.reactivity_net_dollar as f32;", "pos[2]"),
            ("frame.vel[0] = state.fuel_temperature_k as f32;", "vel[0]"),
            ("frame.vel[1] = state.peak_fuel_temperature_k as f32;", "vel[1]"),
            ("frame.vel[2] = state.peak_power_mw as f32;", "vel[2]"),
            ("frame.force_torque = state.power_rate_mw_per_s as f32;", "force_torque"),
        ] {
            assert!(REACTOR_TRIGA_SOURCE.contains(line), "{slot}: the card packs this slot otherwise");
        }
        let slot = |name: &str| {
            body["frame_slots"].as_array().unwrap().iter().find(|slot| slot["slot"] == name).unwrap().clone()
        };
        assert_eq!(slot("vel[0]")["state"], "fuel_temperature_k");
        assert_eq!(slot("vel[0]")["unit"], "K");
        assert_eq!(slot("vel[1]")["state"], "peak_fuel_temperature_k");
        assert_eq!(slot("vel[1]")["unit"], "K");
        // No thermocouple anywhere a caller reads a state.
        assert!(!body["frame_slots"].to_string().contains("thermocouple"));
        let ceiling = reactor.gate_dt_s().unwrap();
        assert_eq!(bits(&body["dt_s"]["ceiling_s"]), ceiling.to_bits());
        let rule = body["dt_s"]["rule"].as_str().unwrap();
        for words in [
            "The records compared run to 14 s after the fire signal",
            "The largest energy released in a compared run is 18.3 MW s, the largest compared peak power 954 MW and the highest compared core-average fuel temperature 585 K",
            "A run that never scrams is computed on past 7.45 s after its peak, the longest compared hold",
            "The hottest core-average fuel any run reaches is 769.7 K",
            "far below 1150 degC, the limit of U-ZrH1.6 fuel in stainless clad",
            "there is no thermocouple on the card",
        ] {
            assert!(rule.contains(words), "{words}: {rule}");
        }
        let ceiling_from = body["dt_s"]["ceiling_from"].as_str().unwrap();
        for words in ["DtAboveStateCeiling", "0.224", "0.241", "Past the compared span the step itself also holds a ceiling that follows the state"] {
            assert!(ceiling_from.contains(words), "{words}: {ceiling_from}");
        }
        let run = &body["example_run_trajectory"];
        assert_eq!(run["segments"][0]["steps"], REACTOR_TRIGA_EXAMPLE_STEPS);
        assert_eq!(run["initial"]["rod_table_core"].as_f64().unwrap(), 217.0);
        let record = &body["validation"];
        let all_of_it = record.to_string();
        // every fitted constant of the card is stated with the kernel's value
        for value in reactor
            .dollar_scale
            .iter()
            .chain(reactor.dollar_offset.iter())
            .chain([reactor.heat_flow_w_per_k].iter())
        {
            assert!(all_of_it.contains(&format!("{value}")), "the record lacks {value}");
        }
        assert!(all_of_it.contains("6.241e-5 per K"));
        assert!(all_of_it.contains("8.718e-8 per K2"));
        assert!(all_of_it.contains("29.59 microseconds"));
        assert_eq!(reactor_triga_jsi::FEEDBACK_PER_K, 6.241e-5);
        assert_eq!(reactor_triga_jsi::FEEDBACK_SLOPE_PER_K2, 8.718e-8);
        assert_eq!(reactor_triga_jsi::GENERATION_TIME_S, 29.59e-6);
        assert!(all_of_it.contains(&format!("the hot-rod factor {}", reactor_triga_jsi::HOT_ROD_FACTOR)));
        for words in [
            "cite gives both",
            "None of the site's measured values is stored in the card",
            "one thermocouple at the centre of one element, 6753 at position B1, with the instrument's response in between",
            "its earlier form, RMS 0.97 and worst 1.59 on the thermocouple maximum, 2.54 RMS on peak power and 4.18 on energy",
            "A second program, written apart, recomputes the comparisons on the same machine",
            "Record SHA-256 9a0720f1822161c3b53cbc351d862ec4c077da51521ee1a96d7ceb85d8275ddc.",
            "Twelve forms read the power after the pulse too low in every bin of reactivity",
            "RMS 1.21 (target 1.5), worst 3.02 (target 3), mean -0.74 (target 1.41)",
            "the nearest recorded pulse, taken unchanged, is within the targets on peak power and thermocouple maximum as well",
            "Energy within 50 ms of the peak: 5.4 percent high; RMS 2.01 (1.5)",
            "The card cannot see the day",
            "no other laboratory has repeated them",
            "All three are outside their targets: worst 3.75 and 3.10 against 3, and RMS 2.01 against 1.5",
            "one of six with none or with 0.015 dollars, two with 0.03 and six with 0.06",
            "they prefer 1.585 with a heat flow of 2930 W/K",
            "158 to 170 per second",
            "fifteen fitted in all",
            "nothing unfitted lies above 2.6 dollars",
            "0.99 correlated with peak power",
            "the thirteenth, with the generation time fitted, is the one served",
            "No constant of the card is therefore the reactor's own value of that quantity",
            "2.0 to 3.0 printed dollars",
        ] {
            assert!(all_of_it.contains(words), "the record lacks: {words}");
        }
        // The record says its numbers and its scope, with no liturgy and no path of the company's disk.
        for said in record.as_object().unwrap().values().map(|value| value.as_str().unwrap()) {
            for word in crate::spec_sheet::UNSAID {
                assert!(!crate::spec_sheet::says(said, word), "the record says \"{word}\": {said}");
            }
            assert!(!said.contains("evidence/"), "{said}");
        }
    }

    /// The 2170 cell lists every variant of its `StepReject`, in the kernel's
    /// order, with the kernel's sentence word for word. (0.15.0: the served
    /// cell_2170_model3, the twin's kernel `cell_2170_twin`; the M80 card is
    /// retired.)
    #[test]
    fn the_2170_cell_refusals_carry_the_sentences_from_the_card() {
        let documented = documented_rejects(CELL_2170_SOURCE);
        let card = find("energy", "cell_2170_model3").expect("the 2170 cell is served");
        assert_eq!(documented.len(), 16);
        assert_eq!(card.rejects.len(), documented.len());
        for ((name, sentence), listed) in documented.iter().zip(card.rejects) {
            assert_eq!(name, listed);
            assert_eq!(
                reject_meaning("cell_2170_model3", name),
                Some(sentence.as_str()),
                "{name}"
            );
        }
        assert_eq!(card.advisory.len(), 2);
    }

    const FINGERPAD_SOURCE: &str = include_str!("../../genesis_kernel/src/fingerpad_p02_glass.rs");

    /// The finger pad lists every variant of its `StepReject`, in the
    /// kernel's order, with the kernel's sentence word for word.
    #[test]
    fn the_finger_pad_refusals_carry_the_sentences_from_the_card() {
        let documented = documented_rejects(FINGERPAD_SOURCE);
        let card = find("materials", "fingerpad_p02_glass").expect("the finger pad is served");
        assert_eq!(documented.len(), 12);
        assert_eq!(card.rejects.len(), documented.len());
        for ((name, sentence), listed) in documented.iter().zip(card.rejects) {
            assert_eq!(name, listed);
            assert_eq!(
                reject_meaning("fingerpad_p02_glass", name),
                Some(sentence.as_str()),
                "{name}"
            );
        }
    }

    /// The finger pad publishes the kernel's own bounds and constants, says
    /// in its one-line summary that its constants are fitted to the finger's
    /// own trials and how the comparison stands, and states in its validation
    /// record what the held-back trials do and do not test and that the
    /// ledger closes for any constant.
    #[test]
    fn the_finger_pad_publishes_the_kernel_bounds_and_says_what_is_fitted() {
        let card = find("materials", "fingerpad_p02_glass").expect("the finger pad is served");
        assert_eq!(card.specimen, crate::spec_sheet::of("fingerpad_p02_glass").unwrap().paragraph());
        assert!(card.specimen.contains("eight constants fitted to this finger's own trials"));
        assert!(card.specimen.contains("slip torque within 9.4, 29.3 and 13.0 percent of the mean of ten trials (target 5.62)"));
        assert!(card.specimen.contains("30 trials not in the fit, at 1 N, 10 N and 100 degrees a second"));
        let field = |name: &str| card.fields.iter().find(|f| f.name == name).expect("field");
        let ends = |name: &str| (field(name).minimum.unwrap(), field(name).maximum.unwrap());
        // Every run and every single step starts untwisted.
        assert_eq!(ends("twist_deg"), (0.0, 0.0));
        // Every force and every rate above zero is answered.
        for name in ["normal_force_n", "twist_rate_deg_s"] {
            assert_eq!((field(name).minimum, field(name).maximum), (Some(0.0), None), "{name}");
            assert!(field(name).minimum_exclusive, "{name}");
        }
        assert!(field("normal_force_n").bound.contains("refused as NoContact"));
        assert!(field("normal_force_n").bound.contains("+29.3 percent at 10 N"));
        assert!(field("twist_rate_deg_s").bound.contains("refused as TwistRateNotPositive"));
        assert!(field("twist_rate_deg_s").bound.contains("-13.0 percent"));
        assert!(card.specimen.contains("computes every force and rate above zero and the twist past 75 degrees with the same equations"));
        assert!(card.specimen.contains("+9.4, +29.3 and -13.0 percent"));
        for constant in [
            fingerpad_p02_glass::SLIP_TORQUE_REFERENCE_N_M,
            fingerpad_p02_glass::LOAD_EXPONENT,
            fingerpad_p02_glass::RATE_EXPONENT,
            fingerpad_p02_glass::TORQUE_TWIST_REFERENCE_DEG,
            fingerpad_p02_glass::TORQUE_TWIST_LOAD_EXPONENT,
            fingerpad_p02_glass::TORQUE_TWIST_RATE_EXPONENT,
            fingerpad_p02_glass::STUCK_TWIST_REFERENCE_DEG,
            fingerpad_p02_glass::STUCK_TWIST_RATE_EXPONENT,
        ] {
            assert!(card.nyquist_rule.contains(&format!("{constant}")), "{constant}");
        }
        assert!(card.nyquist_rule.contains("LoadChangedDuringTwist"));
        assert!(card.nyquist_rule.contains("TwistNotStartedFromZero"));
        let described =
            describe_value_in(&View::full(), "materials", "fingerpad_p02_glass").unwrap();
        // The event of full slip, named as an event, and the frame beyond
        // what was compared, a warning.
        let flags = described["advisory_flags"].as_array().unwrap();
        assert_eq!(flags.len(), 2);
        assert_eq!(flags[0]["name"], "FLAG_FULL_SLIP");
        assert_eq!(flags[0]["kind"], "event");
        assert_eq!(flags[0]["state"], "full_slip");
        assert_eq!(flags[1]["name"], "FLAG_BEYOND_COMPARED");
        assert!(flags[1].get("kind").is_none());
        assert_eq!(flags[1]["state"], "beyond_compared");
        let record = &described["validation"];
        assert!(record["data"].as_str().unwrap().contains("10.5281/zenodo.7646119"));
        assert!(record["pass_line"].as_str().unwrap().contains(
            "49a4788d1eb6e5d5881d4f3b2195ad248a2a543c2da45fb1c8ffb7dc27672081"
        ));
        assert_eq!(record["result"], FINGERPAD_P02_GLASS_RESULT);
        let limits = record["limits"].as_str().unwrap();
        for words in [
            "nothing there is a test",
            "block to block, 13 to 15 percent",
            "the experiments were conducted on separate days",
            "The twist at full slip is the card's weakest output",
            "closes for any value of the eight constants",
            "TwistNotStartedFromZero",
            "the work of a twist to 75 degrees at 2.07 N and 50 degrees a second is 0.0126 J",
            "The twist was compared from 1 to 75 degrees.",
            "+13.6 percent at 0.5 N, +9.4 at 1 N, +21.7 at 5 N and +29.3 at 10 N",
            "TwistStepAboveQuadrature",
        ] {
            assert!(limits.contains(words), "{words}");
        }
        assert!(!limits.contains("0.015 J"));
        assert!(field("twist_deg").bound.contains("past 75 degrees a run goes on, each frame carrying FLAG_BEYOND_COMPARED"));
        assert!(card.nyquist_rule.contains("A run goes on past 75 degrees"));
        assert_eq!(fingerpad_p02_glass::COMPARED_TWIST_END_DEG, 75.0);
        // No other card's flag is called an event, but the grasped object's,
        // whose slip is the event it exists to find.
        for other in all().iter().filter(|other| other.card != "fingerpad_p02_glass" && other.card != "grasp_lift_glass_4tu") {
            let body = describe_one(other);
            for flag in body["advisory_flags"].as_array().unwrap() {
                assert!(flag.get("kind").is_none(), "{}", other.card);
            }
        }
        // The grasped object's two slips are events; its resting on the
        // table is a condition of the frame, and is served as one.
        let grasp = describe_value_in(&View::full(), "materials", "grasp_lift_glass_4tu").unwrap();
        let kinds: Vec<(String, Value)> = grasp["advisory_flags"]
            .as_array()
            .unwrap()
            .iter()
            .map(|flag| (flag["name"].as_str().unwrap().to_string(), flag.get("kind").cloned().unwrap_or(Value::Null)))
            .collect();
        assert_eq!(
            kinds,
            vec![
                ("FLAG_INCIPIENT_SLIP".to_string(), json!("event")),
                ("FLAG_GROSS_SLIP".to_string(), json!("event")),
                ("FLAG_ON_TABLE".to_string(), Value::Null),
            ]
        );
    }

    const TIRE_SOURCE: &str = include_str!("../../genesis_kernel/src/tire_eve_r19_ice.rs");

    /// The tyre lists every variant of its `StepReject`, in the kernel's
    /// order, with the kernel's sentence word for word.
    #[test]
    fn the_tyre_refusals_carry_the_sentences_from_the_card() {
        let documented = documented_rejects(TIRE_SOURCE);
        let card = find("materials", "tire_eve_r19_ice").expect("the tyre is served");
        assert_eq!(documented.len(), 13);
        assert_eq!(card.rejects.len(), documented.len());
        for ((name, sentence), listed) in documented.iter().zip(card.rejects) {
            assert_eq!(name, listed);
            assert_eq!(
                reject_meaning("tire_eve_r19_ice", name),
                Some(sentence.as_str()),
                "{name}"
            );
        }
    }

    /// The tyre publishes the kernel's own bounds, says in its one-line
    /// summary that its friction ratio is fitted to the tyre's own
    /// measurements, and states in its validation record what the held-back
    /// runs do and do not test.
    #[test]
    fn the_tyre_publishes_the_kernel_bounds_and_says_what_is_fitted() {
        let card = find("materials", "tire_eve_r19_ice").expect("the tyre is served");
        assert!(card.specimen.contains("Fitted on 72 of the 252 runs of the EVE ice dataset"));
        let field = |name: &str| card.fields.iter().find(|f| f.name == name).expect("field");
        let ends = |name: &str| (field(name).minimum.unwrap(), field(name).maximum.unwrap());
        let lower = |name: &str| (field(name).minimum.unwrap(), field(name).maximum);
        // The end of the rig's track is not an end, and the hold goes on.
        assert_eq!(lower("distance_m"), (0.0, None));
        assert_eq!(lower("slip_held_s"), (0.0, None));
        assert!(field("slip_held_s").bound.contains("refused as SlipHeldBelowZero"));
        assert!(field("slip_held_s").bound.contains("lands with FLAG_SLIP_SETTLING"));
        assert!(field("slip_held_s").bound.contains("past 21 s a frame carries FLAG_BEYOND_COMPARED"));
        // No speed was measured: the bound says the speed is inferred.
        assert!(field("carriage_speed_m_s").bound.contains("The files record no carriage speed"));
        assert!(!field("carriage_speed_m_s").bound.contains("no other speed was measured"));
        // A load and a pressure above zero: zero or less is no contact, or flat.
        assert_eq!(lower("normal_load_n"), (0.0, None));
        assert!(field("normal_load_n").minimum_exclusive);
        assert!(field("normal_load_n").bound.contains("refused as NoContact"));
        assert_eq!(lower("inflation_pressure_pa"), (0.0, None));
        assert!(field("inflation_pressure_pa").minimum_exclusive);
        assert!(field("inflation_pressure_pa").bound.contains("refused as TyreFlat"));
        assert_eq!(
            ends("slip_ratio"),
            (-tire_eve_r19_ice::SLIP_CEILING, tire_eve_r19_ice::SLIP_CEILING)
        );
        assert!(field("slip_ratio").bound.contains("refused as SlipOutsideCard"));
        assert_eq!(
            ends("carriage_speed_m_s"),
            (
                tire_eve_r19_ice::CARRIAGE_SPEED_FLOOR_M_S,
                tire_eve_r19_ice::CARRIAGE_SPEED_CEILING_M_S
            )
        );
        assert!(card.nyquist_rule.contains(&format!("{}", tire_eve_r19_ice::MU_TRACTION)));
        assert!(card.nyquist_rule.contains(&format!("{}", tire_eve_r19_ice::MU_BRAKING)));
        assert!(card.nyquist_rule.contains(&format!("{}", tire_eve_r19_ice::SLIP_EXPONENT)));
        let described = describe_value_in(&View::full(), "materials", "tire_eve_r19_ice").unwrap();
        let record = &described["validation"];
        assert!(record["data"].as_str().unwrap().contains("10.5281/zenodo.1009017"));
        assert!(record["pass_line"].as_str().unwrap().contains(
            "84a380a5c948cd741ac91845fbfeac7ed1b60ed3c6a01855bcb99c996e7782c1"
        ));
        assert_eq!(record["result"], TIRE_EVE_R19_ICE_RESULT);
        let limits = record["limits"].as_str().unwrap();
        assert!(limits.contains("interpolation in pressure"));
        assert!(limits.contains("extrapolation in load"));
        assert!(limits.contains("fitted to this tyre's own runs"));
        // The comparator is the geometric mean, as the fit is.
        assert!(record["result"].as_str().unwrap().contains("geometric mean"));
        assert!(record["pass_line"].as_str().unwrap().contains("geometric mean"));
        assert!(card.nyquist_rule.contains("every one-second window from 5 s to 20 s"));
        assert!(card.nyquist_rule.contains("The end of the rig's track is not an end of the tyre"));
        assert!(limits.contains("compared at 6.5 kN with a root-mean-square error of 0.029"));
        assert_eq!(card.ceiling_s, 4_194_304.0);
    }

    const ACCUMULATOR_SOURCE: &str = include_str!("../../genesis_kernel/src/accumulator_sbo450.rs");

    /// The accumulator lists every variant of its `StepReject`, in the
    /// kernel's order, with the kernel's sentence word for word.
    #[test]
    fn the_accumulator_refusals_carry_the_sentences_from_the_card() {
        let documented = documented_rejects(ACCUMULATOR_SOURCE);
        let card = find("energy", "accumulator_sbo450").expect("the accumulator is served");
        assert_eq!(documented.len(), 14);
        assert_eq!(card.rejects.len(), documented.len());
        for ((name, sentence), listed) in documented.iter().zip(card.rejects) {
            assert_eq!(name, listed);
            assert_eq!(
                reject_meaning("accumulator_sbo450", name),
                Some(sentence.as_str()),
                "{name}"
            );
        }
    }

    /// The accumulator publishes the kernel's own bounds, says in its
    /// one-line summary which constants are fitted and what was compared, and
    /// states in its record what the held-back runs do and do not test and
    /// that the ledger closes for any constant.
    #[test]
    fn the_accumulator_publishes_the_kernel_bounds_and_says_what_is_fitted() {
        let card = find("energy", "accumulator_sbo450").expect("the accumulator is served");
        assert_eq!(card.specimen, crate::spec_sheet::of("accumulator_sbo450").unwrap().paragraph().as_str());
        assert!(card.specimen.contains("Fitted on the 100 runs from 0.01 to 10 Hz of the two records at 40 bar precharge and 60 bar load"));
        let field = |name: &str| card.fields.iter().find(|f| f.name == name).expect("field");
        let ends = |name: &str| (field(name).minimum, field(name).maximum);
        assert_eq!(ends("gas_mass_kg"), (Some(0.0), None));
        assert_eq!(ends("gas_pressure_pa"), (Some(0.0), Some(accumulator_sbo450::PRESSURE_RATED_PA)));
        assert_eq!(ends("gas_temperature_k"), (Some(accumulator_sbo450::CRITICAL_TEMPERATURE_K), None));
        assert_eq!(ends("wall_temperature_k"), (Some(0.0), None));
        assert_eq!(ends("piston_speed_m_s"), (None, None));
        assert!(field("gas_pressure_pa").bound.contains("PressureAboveRated"));
        assert!(field("gas_pressure_pa").bound.contains("held as 37 to 43 and 55 to 67 bar for the 20 bar charge and 57 to 64 and 75 to 86 bar for the 40 bar charge"));
        assert!(field("gas_pressure_pa").bound.contains("2.72 percent median error of the swing (target 2.0)"));
        assert_eq!(accumulator_sbo450::PRESSURE_BANDS_20_BAR_PA, [[3.7e6, 4.3e6], [5.5e6, 6.7e6]]);
        assert_eq!(accumulator_sbo450::PRESSURE_BANDS_40_BAR_PA, [[5.7e6, 6.4e6], [7.5e6, 8.6e6]]);
        // The sums of the motion before a start are required continuation inputs.
        for name in ["work_moved_j", "heat_moved_j"] {
            assert_eq!(field(name).minimum, Some(0.0));
            assert_eq!(field(name).maximum, None);
            assert!(field(name).bound.contains("required"), "{name}");
            assert!(field(name).bound.contains("NegativeMovedSum"), "{name}");
            assert!(field(name).bound.contains("HeatShareAboveCard"), "{name}");
        }
        assert!(card.nyquist_rule.contains("A start without the two sums is refused as a missing input."));
        assert!(!card.nyquist_rule.contains("never weighed as a run"));
        assert!(field("wall_temperature_k").bound.contains("no segment can change it"));
        // The wall, the gas temperature and the speed compared are stated, and past them the flag.
        assert!(field("wall_temperature_k").bound.contains("293.46 to 293.60 K"));
        assert!(field("wall_temperature_k").bound.contains("FLAG_OUTSIDE_COMPARED_RUNS"));
        assert!(field("gas_temperature_k").bound.contains("285.97 to 301.78 K"));
        assert!(field("piston_speed_m_s").bound.contains("FLAG_OUTSIDE_COMPARED_RUNS"));
        assert!(ACCUMULATOR_SBO450_LIMITS.contains("the wall of every compared run was 293.46 to 293.60 K"));
        // The conservation gate depends on the ledger's anchor; no limit of the range does.
        assert!(card.nyquist_rule.contains("the ledger's conservation balance is held from each start, and no limit of the card's range depends on them"));
        assert!(!card.nyquist_rule.contains("begin again at each start, and no limit depends on them"));
        assert!(field("gas_mass_kg").bound.contains("0.013639 to 0.013667 kg"));
        assert!(field("gas_mass_kg").bound.contains("0.02744 to 0.027495 kg"));
        assert!(field("gas_mass_kg").bound.contains("GasMassNotPositive"));
        assert_eq!(accumulator_sbo450::PISTON_BORE_M, 0.05);
        assert!(card.nyquist_rule.contains("19.635 cm2"));
        assert_eq!(accumulator_sbo450::HEAT_SHARE_LIMIT, 0.065);
        assert_eq!(accumulator_sbo450::HEAT_SHARE_WORK_FLOOR_J, 100.0);
        assert!(card.nyquist_rule.contains("at most 0.065 of the larger of M and 100 J"));
        assert!(card.nyquist_rule.contains("refused as HeatShareAboveCard"));
        assert_eq!(card.advisory.len(), 1);
        assert!(card
            .nyquist_rule
            .contains(&format!("{} s", accumulator_sbo450::THERMAL_TIME_CONSTANT_S)));
        assert!(card.nyquist_rule.contains("0.58879 l"));
        assert!(card.nyquist_rule.contains("1e-12"));
        let described = describe_value_in(&View::full(), "energy", "accumulator_sbo450").unwrap();
        let record = &described["validation"];
        assert!(record["data"].as_str().unwrap().contains("10.5281/zenodo.11265458"));
        assert!(record["pass_line"].as_str().unwrap().contains(
            "721d88a5264b0e15342824290e6305db1df27fab76011333b49d5ed253b38323"
        ));
        assert_eq!(record["result"], ACCUMULATOR_SBO450_RESULT);
        let limits = record["limits"].as_str().unwrap();
        assert!(limits.contains("extrapolation in pressure and in charge"));
        assert!(limits.contains("fitted to this accumulator's own runs"));
        assert!(limits.contains("The card's geometry and range were corrected after the comparison, with every run read"));
        assert!(limits.contains("The ledger closes for any time constant and any vessel volume"));
        assert!(limits.contains("choked flow does not arise"));
    }

    /// The tokamak lists every variant of its `StepReject`, in the kernel's
    /// order, with the kernel's sentence word for word. (0.15.0: the kernel is
    /// the JET twin's, `tokamak_jet_dte1_twin`.)
    #[test]
    fn the_tokamak_refusals_carry_the_sentences_from_the_card() {
        let documented = documented_rejects(TOKAMAK_SOURCE);
        let card = find("energy", "tokamak_jet_dte1").expect("the tokamak is served");
        assert_eq!(documented.len(), 24);
        assert_eq!(card.rejects.len(), documented.len());
        for ((name, sentence), listed) in documented.iter().zip(card.rejects) {
            assert_eq!(name, listed);
            assert_eq!(
                reject_meaning("tokamak_jet_dte1", name),
                Some(sentence.as_str()),
                "{name}"
            );
        }
    }

    /// The cantilever lists every variant of its `StepReject`, in the kernel's
    /// order, with the kernel's sentence word for word.
    #[test]
    fn the_cantilever_refusals_carry_the_sentences_from_the_card() {
        let documented = documented_rejects(CANTILEVER_SOURCE);
        let card = find("signal", "cantilever_eth").expect("the cantilever is served");
        assert_eq!(documented.len(), 7);
        assert_eq!(card.rejects.len(), documented.len());
        for ((name, sentence), listed) in documented.iter().zip(card.rejects) {
            assert_eq!(name, listed);
            assert_eq!(
                reject_meaning("cantilever_eth", name),
                Some(sentence.as_str()),
                "{name}"
            );
        }
    }

    /// The cantilever publishes the bounds the kernel's constants give: the
    /// amplitude ceiling for the deflection at rest, and for the speed at
    /// zero deflection the last 64-bit number whose envelope, by the card's
    /// own function, is at or under the ceiling. Both fields say that the
    /// envelope is the condition they share, with its floor and the largest
    /// envelope compared. The step ceiling and floor are the card's, no load
    /// field is published, the two flags are, the
    /// frame slots name the state they come from, and the examples start at
    /// the start of the scored ring-down, which the compared run reaches
    /// after 595 steps from its own start, which the card accepts.
    #[test]
    fn the_cantilever_publishes_the_kernel_bounds_and_the_start_of_the_scored_ring_down() {
        let card = find("signal", "cantilever_eth").expect("the cantilever is served");
        let beam = cantilever_eth::CantileverEth::first_bending_mode();
        let body = describe_one(card);
        let bounds = |field: &Value| {
            (
                field["minimum"].as_f64().unwrap().to_bits(),
                field["maximum"].as_f64().unwrap().to_bits(),
                field["minimum_exclusive"].as_bool().unwrap(),
                field["maximum_exclusive"].as_bool().unwrap(),
            )
        };
        assert_eq!(body["phase_fields"].as_array().unwrap().len(), 2);
        assert!(body["load_fields"].as_array().unwrap().is_empty());
        assert_eq!(card.load_names.len(), 0);

        let ceiling_m = cantilever_eth::AMPLITUDE_CEILING_M;
        let deflection = &body["phase_fields"][0];
        assert_eq!(deflection["name"], "deflection_m");
        assert_eq!(deflection["unit"], "m");
        assert_eq!(bounds(deflection), ((-ceiling_m).to_bits(), ceiling_m.to_bits(), false, false));
        // At rest the envelope is the size of the deflection, to the bit.
        assert_eq!(beam.envelope_m(ceiling_m, 0.0).to_bits(), ceiling_m.to_bits());
        assert_eq!(beam.envelope_m(-ceiling_m, 0.0).to_bits(), ceiling_m.to_bits());
        assert!(cantilever_eth::CantileverState::try_phase(&beam, ceiling_m, 0.0).is_some());
        let above = f64::from_bits(ceiling_m.to_bits() + 1);
        assert!(cantilever_eth::CantileverState::try_phase(&beam, above, 0.0).is_none());

        let speed = &body["phase_fields"][1];
        assert_eq!(speed["name"], "velocity_m_s");
        assert_eq!(speed["unit"], "m/s");
        let largest_m_s = cantilever_largest_speed_m_s(&beam);
        assert_eq!(bounds(speed), ((-largest_m_s).to_bits(), largest_m_s.to_bits(), false, false));
        // 0.01875 m times 2 pi times 8.29235 Hz, to the last few 64-bit numbers.
        assert_eq!(ceiling_m, 0.01875);
        let product = ceiling_m * beam.omega0_rad_s();
        assert!((largest_m_s - product).abs() <= 4.0 * f64::EPSILON * product, "{largest_m_s}");
        assert!(largest_m_s > 0.9769 && largest_m_s < 0.9770, "{largest_m_s}");
        for sign in [1.0, -1.0] {
            assert!(cantilever_eth::CantileverState::try_phase(&beam, 0.0, sign * largest_m_s).is_some());
            let beyond = sign * f64::from_bits(largest_m_s.to_bits() + 1);
            assert!(cantilever_eth::CantileverState::try_phase(&beam, 0.0, beyond).is_none());
        }
        for field in [deflection, speed] {
            let words = field["bound"].as_str().unwrap();
            assert!(words.contains("sqrt(deflection_m^2 + (velocity_m_s / omega0)^2)"), "{words}");
            assert!(words.contains("at or below 0.01875 m, the largest amplitude the softening and the damping were fitted on"), "{words}");
            assert!(words.contains("Above 0.01579 m, the largest envelope the card held on the run compared"), "{words}");
            assert!(words.contains("EnvelopeAboveCard"), "{words}");
            assert!(words.contains("FLAG_ENVELOPE_BELOW_COMPARED"), "{words}");
            assert!(words.contains("FLAG_ENVELOPE_ABOVE_COMPARED"), "{words}");
            assert!(!words.contains("DeflectionBelowCard"), "{words}");
        }
        // The compared floor of the envelope is no wall: rest at zero and
        // anything under 5.8e-5 m makes a start.
        let floor_m = cantilever_eth::AMPLITUDE_FLOOR_M;
        assert_eq!(floor_m, 5.8e-5);
        assert!(cantilever_eth::CantileverState::try_phase(&beam, floor_m, 0.0).is_some());
        let under = f64::from_bits(floor_m.to_bits() - 1);
        assert!(cantilever_eth::CantileverState::try_phase(&beam, under, 0.0).is_some());
        assert!(cantilever_eth::CantileverState::try_phase(&beam, 0.0, 0.0).is_some());

        let dt = &body["dt_s"];
        assert_eq!(dt["ceiling_s"].as_f64().unwrap().to_bits(), cantilever_eth::GATE_DT_S.to_bits());
        assert_eq!(dt["ceiling_s"].as_f64().unwrap(), 2.5e-4);
        assert_eq!(dt["floor_s"].as_f64().unwrap().to_bits(), (2.5e-4_f64 / 262_144.0).to_bits());
        assert!(dt["nyquist_s"].is_null());
        assert!(dt["rule"].as_str().unwrap().contains("No load: the beam rings free"));

        let flags = body["advisory_flags"].as_array().unwrap();
        assert_eq!(flags.len(), 2);
        assert_eq!(flags[0]["name"], "FLAG_ENVELOPE_BELOW_COMPARED");
        assert_eq!(flags[0]["state"], "envelope_below_compared");
        assert_eq!(flags[1]["name"], "FLAG_ENVELOPE_ABOVE_COMPARED");
        assert_eq!(flags[1]["state"], "none: the frame's flags alone");
        let slot = |name: &str| {
            body["frame_slots"]
                .as_array()
                .unwrap()
                .iter()
                .find(|slot| slot["slot"] == name)
                .unwrap()
                .clone()
        };
        for (name, state, unit) in [
            ("pos[0]", "deflection_m", "m"),
            ("pos[1]", "velocity_m_s", "m/s"),
            ("pos[2]", "envelope_m", "m"),
            ("vel[0]", "kinetic_j_per_kg", "J/kg"),
            ("vel[1]", "potential_j_per_kg", "J/kg"),
            ("vel[2]", "heat_j_per_kg", "J/kg"),
            ("force_torque", "envelope_m", "m"),
            ("residual", "residual_conservation_error", "1"),
        ] {
            assert_eq!(slot(name)["state"], state, "{name}");
            assert_eq!(slot(name)["unit"], unit, "{name}");
        }
        assert!(slot("force_torque")["holds"].as_str().unwrap().contains("0.01579 − envelope_m"));
        assert!(slot("flags")["holds"].as_str().unwrap().starts_with("FLAG_ENVELOPE_BELOW_COMPARED, bit 0: the envelope is below 5.8e-5 m"));
        assert!(slot("flags")["holds"].as_str().unwrap().contains("FLAG_ENVELOPE_ABOVE_COMPARED, bit 1: the envelope is above 0.01579 m"));

        assert_eq!(
            serde_json::to_string(&body["example_compute_step"]).unwrap(),
            r#"{"domain":"signal","card":"cantilever_eth","parameters":{"deflection_m":-0.00104914043915228,"velocity_m_s":0.7408030982848752,"dt_s":0.00025}}"#
        );
        assert_eq!(
            serde_json::to_string(&body["example_run_trajectory"]).unwrap(),
            r#"{"domain":"signal","card":"cantilever_eth","initial":{"deflection_m":-0.00104914043915228,"velocity_m_s":0.7408030982848752},"dt_s":0.00025,"segments":[{"steps":4000,"load":{}}],"record_every":10}"#
        );
        // The compared run's own start is the largest envelope it held, and
        // the card makes that start.
        let compared_envelope_m = beam.envelope_m(
            CANTILEVER_VALIDATION_START_DEFLECTION_M,
            CANTILEVER_VALIDATION_START_VELOCITY_M_S,
        );
        assert!((compared_envelope_m - 15.786e-3).abs() < 5.0e-8, "{compared_envelope_m}");
        assert!(cantilever_eth::CantileverState::try_phase(
            &beam,
            CANTILEVER_VALIDATION_START_DEFLECTION_M,
            CANTILEVER_VALIDATION_START_VELOCITY_M_S,
        )
        .is_some());
        // The examples start on the card, at the start of the scored ring-down.
        let start = cantilever_eth::CantileverState::try_phase(
            &beam,
            CANTILEVER_EXAMPLE_START_DEFLECTION_M,
            CANTILEVER_EXAMPLE_START_VELOCITY_M_S,
        )
        .expect("the scored ring-down starts on the card");
        assert!((start.envelope_m - 14.2569e-3).abs() < 5.0e-8, "{}", start.envelope_m);
        // The description states the range and what was compared.
        let rule = dt["rule"].as_str().unwrap();
        for words in [
            "The envelope range compared with the second ring-down, outside the fit, in numbers:",
            "from 15.786 mm at its start, the largest it held, through 14.404 mm, at the first turning point after the start crossing, to 0.0587 mm",
            "There the amplitude error is 7.03 percent (target 5)",
            "The compared run lands 94,731 steps inside the compared envelope.",
            "lands 94,136 steps, to 23.534 s, inside the compared envelope, and goes on below it",
            "rest at zero is a start that does not move",
            "the lowest frequency read on the beam was 8.22 Hz",
        ] {
            assert!(rule.contains(words), "{words}: {rule}");
        }
        assert!(rule.contains("FLAG_ENVELOPE_BELOW_COMPARED"), "{rule}");
        assert!(rule.contains("every frame there carries FLAG_ENVELOPE_ABOVE_COMPARED"), "{rule}");
        assert!(rule.contains("The card's ceiling is 0.01875 m"), "{rule}");
    }

    /// The cantilever's validation record: the citation, the recording the
    /// constants were fitted to, the half that was opened and the half kept
    /// unread, the hash of the pre-registration with the four lines, the
    /// forecast named as a forecast, and no word of a proof.
    #[test]
    fn the_cantilever_validation_record_states_the_data_the_lines_and_the_limits() {
        let card = find("signal", "cantilever_eth").expect("the cantilever is served");
        let record = &describe_one(card)["validation"];
        let part = |name: &str| record[name].as_str().unwrap().to_string();
        let object = part("object");
        for words in ["326 mm long, 40 mm wide and 1 mm thick", "first bending mode", "free decay", "laser-1 station"] {
            assert!(object.contains(words), "{words}: {object}");
        }
        let data = part("data");
        for words in [
            "Ring-down recordings of the strip, doi 10.5281/zenodo.22723484",
            "six constants are fitted on one recording, test1a",
            "test1b, is outside the fit",
            "its first 12 s inform a forecast made outside the engine",
            "its 12 s to 24 s, outside the fit and the forecast, are what the card is compared with",
            "Before the split was written, the coarse envelope of every recording",
            "was printed; no fit was made to it",
        ] {
            assert!(data.contains(words), "{words}: {data}");
        }
        let pass_line = part("pass_line");
        for words in [
            "d081cc5ae51b5de4a028cd17b4e1ae80b402f71fdc786f82876713f3b925d4a6",
            "between 12 s and 24 s",
            "amplitude error at most 5.0 percent",
            "frequency error at most 0.010 Hz",
            "timing error at most 0.20 of a period by 24 s",
            "at most one third of a plain damped oscillator's",
        ] {
            assert!(pass_line.contains(words), "{words}: {pass_line}");
        }
        let limits = part("limits");
        assert!(limits.starts_with("One strip, one clamp, one afternoon."), "{limits}");
        for words in [
            "2.22 percent in amplitude, 0.0047 Hz in frequency and 0.024 of a period in timing",
            "over a smaller structure of five",
            "This structure of six constants was chosen with the errors on those 12 s read",
            "One strip, one clamp, one afternoon.",
            "confirmed by a second reading to 3 percent",
            "every compared quantity is a ratio or a frequency",
            "no mass, so its energies are per kilogram",
            "a power law fitted on amplitudes from 0.072 to 18.75 mm",
            "refused after 107 steps of 0.25 ms",
            "The comparison spans envelopes from 15.79 mm down to 0.058 mm",
            "the card answers with the frame flagged FLAG_ENVELOPE_ABOVE_COMPARED",
            "Below 0.058 mm the card answers on to rest with the frame flagged",
        ] {
            assert!(limits.contains(words), "{words}: {limits}");
        }
        // The record states the outcome of the comparison.
        assert_eq!(part("result"), CANTILEVER_RESULT);
        assert_ne!(part("result"), NOT_COMPARED);
        assert!(!record.to_string().contains("seal"));
    }

    /// The crater lists every variant of its `StepReject`, in the kernel's
    /// order, with the kernel's sentence word for word, and its one flag.
    #[test]
    fn the_crater_refusals_carry_the_sentences_from_the_card() {
        let documented = documented_rejects(CRATER_SOURCE);
        let card = find("aerospace", "crater_ryugu_sci").expect("the crater is served");
        assert_eq!(documented.len(), 9);
        assert_eq!(card.rejects.len(), documented.len());
        for ((name, sentence), listed) in documented.iter().zip(card.rejects) {
            assert_eq!(name, listed);
            assert_eq!(
                reject_meaning("crater_ryugu_sci", name),
                Some(sentence.as_str()),
                "{name}"
            );
        }
        assert_eq!(card.advisory.len(), 1);
        assert_eq!(card.advisory[0].name, "FLAG_STOPPED");
        assert_eq!(card.advisory[0].mask, crater_ryugu_sci::FLAG_STOPPED);
        assert!(card.advisory[0].seals);
    }

    /// The crater publishes the kernel's two radii as its bounds, both
    /// inclusive, the card's own step ceiling and floor, no load field, and
    /// a record that says nothing is fitted to Ryugu.
    #[test]
    fn the_crater_publishes_the_kernel_bounds_and_says_what_is_fitted() {
        let card = find("aerospace", "crater_ryugu_sci").expect("the crater is served");
        let crater = crater_ryugu_sci::CraterRyuguSci::sci_impact();
        let body = describe_one(card);
        assert_eq!(body["phase_fields"].as_array().unwrap().len(), 1);
        assert!(body["load_fields"].as_array().unwrap().is_empty());
        let radius = &body["phase_fields"][0];
        assert_eq!(radius["name"], "radius_m");
        assert_eq!(radius["unit"], "m");
        assert_eq!(radius["minimum"].as_f64().unwrap().to_bits(), crater_ryugu_sci::RADIUS_FLOOR_M.to_bits());
        assert_eq!(radius["maximum"].as_f64().unwrap().to_bits(), crater_ryugu_sci::FINAL_RADIUS_M.to_bits());
        assert_eq!(radius["minimum_exclusive"], false);
        assert_eq!(radius["maximum_exclusive"], false);
        let words = radius["bound"].as_str().unwrap();
        assert!(
            words.starts_with(&format!(
                "inclusive [{}, {}] m",
                token(crater_ryugu_sci::RADIUS_FLOOR_M),
                token(crater_ryugu_sci::FINAL_RADIUS_M)
            )),
            "{words}"
        );
        for inside in [crater_ryugu_sci::RADIUS_FLOOR_M, crater_ryugu_sci::FINAL_RADIUS_M] {
            assert!(crater_ryugu_sci::CraterState::try_radius(&crater, inside).is_some());
        }
        let below = f64::from_bits(crater_ryugu_sci::RADIUS_FLOOR_M.to_bits() - 1);
        let above = f64::from_bits(crater_ryugu_sci::FINAL_RADIUS_M.to_bits() + 1);
        assert!(crater_ryugu_sci::CraterState::try_radius(&crater, below).is_none());
        assert!(crater_ryugu_sci::CraterState::try_radius(&crater, above).is_none());
        let dt = &body["dt_s"];
        assert_eq!(dt["ceiling_s"].as_f64().unwrap(), 0.25);
        assert_eq!(dt["floor_s"].as_f64().unwrap().to_bits(), (0.25_f64 / 1_048_576.0).to_bits());
        assert!(dt["nyquist_s"].is_null());
        assert!(dt["rule"].as_str().unwrap().contains("FLAG_STOPPED"));
        assert!(card.specimen.contains("nothing fitted to Ryugu"));
        assert_eq!(card.specimen, crate::spec_sheet::of("crater_ryugu_sci").unwrap().paragraph());
        let record = &body["validation"];
        assert_eq!(record["result"], CRATER_RESULT);
        assert!(record["pass_line"].as_str().unwrap().contains("ee2793f1ce97b42ed07210121d4fb7cc2f3599db1db1a46a9530efd3cb9da9b3"));
        assert!(record["limits"].as_str().unwrap().contains("Nothing is fitted to Ryugu"));
        assert!(record["limits"].as_str().unwrap().contains("an extrapolation"));
        assert!(record["data"].as_str().unwrap().contains("14.5 +/- 0.8 m"));
        let (step, run) = worked_examples("aerospace", "crater_ryugu_sci");
        assert_eq!(step["parameters"]["radius_m"], 1.75);
        assert_eq!(step["parameters"]["dt_s"], 0.25);
        assert_eq!(run["segments"][0]["steps"], 1600);
        assert_eq!(run["record_every"], 100);
    }

    /// The creep card lists every variant of its `StepReject`, in the kernel's
    /// order, with the kernel's sentence word for word, except that the two
    /// limits of 5 percent name it as the card's top.
    #[test]
    fn the_creep_refusals_carry_the_sentences_from_the_card() {
        let documented = documented_rejects(CREEP_SOURCE);
        let card = find("materials", "creep_cmsx6").expect("the creep card is served");
        assert_eq!(documented.len(), 12);
        assert_eq!(card.rejects.len(), documented.len());
        for ((name, sentence), listed) in documented.iter().zip(card.rejects) {
            assert_eq!(name, listed);
            let sentence = sentence.replace("passed 5 percent", "went past 5 percent").replace(", the largest level of the protocol.", ", the card's top.");
            assert_eq!(reject_meaning("creep_cmsx6", name), Some(sentence.as_str()), "{name}");
        }
        assert_eq!(reject_meaning("creep_cmsx6", "CreepExtensionOutsideCard"), Some("The incoming creep extension is above 5 percent, the card's top."));
        assert_eq!(reject_meaning("creep_cmsx6", "CreepExtensionBeyondCard"), Some("The trial step went past 5 percent creep extension, the card's top."));
    }

    /// The creep card publishes the kernel's bounds exactly, its step ceiling
    /// and floor, its flag and its frame slots, and its examples start a test
    /// at 200 MPa from 0.1 percent creep extension.
    #[test]
    fn the_creep_card_publishes_the_kernel_bounds_and_its_examples_start_at_a_tenth_of_a_percent() {
        let card = find("materials", "creep_cmsx6").expect("the creep card is served");
        let alloy = creep_cmsx6::CreepCmsx6::at_980_c();
        let body = describe_one(card);
        let bounds = |field: &Value| {
            (
                field["minimum"].as_f64().unwrap().to_bits(),
                field["maximum"].as_f64().unwrap().to_bits(),
                field["minimum_exclusive"].as_bool().unwrap(),
                field["maximum_exclusive"].as_bool().unwrap(),
            )
        };
        assert_eq!(body["phase_fields"].as_array().unwrap().len(), 1);
        assert_eq!(body["load_fields"].as_array().unwrap().len(), 1);
        let extension = &body["phase_fields"][0];
        assert_eq!(extension["name"], "creep_extension_percent");
        assert_eq!(extension["unit"], "%");
        assert_eq!(
            bounds(extension),
            (
                0.0_f64.to_bits(),
                creep_cmsx6::CREEP_EXTENSION_MAX_PERCENT.to_bits(),
                false,
                false
            )
        );
        assert_eq!(creep_cmsx6::CREEP_EXTENSION_MIN_PERCENT, 0.1);
        let bound = extension["bound"].as_str().unwrap_or_else(|| panic!("{extension}"));
        assert!(bound.contains("to 5 percent, inclusive, at every stress"), "{bound}");
        assert!(bound.contains("CreepExtensionBelowZero"), "{bound}");
        assert!(bound.contains("FLAG_OUTSIDE_MET_RANGE"), "{bound}");
        let stress = &body["load_fields"][0];
        assert_eq!(stress["name"], "stress_mpa");
        assert_eq!(stress["unit"], "MPa");
        assert_eq!(
            bounds(stress),
            (
                creep_cmsx6::STRESS_MIN_MPA.to_bits(),
                creep_cmsx6::STRESS_MAX_MPA.to_bits(),
                false,
                false
            )
        );
        // The ends land in the kernel and one spacing outside does not, at
        // every stress.
        for stress in [creep_cmsx6::STRESS_MIN_MPA, creep_cmsx6::FULL_REACH_STRESS_MIN_MPA] {
            for edge in [0.0, creep_cmsx6::CREEP_EXTENSION_MIN_PERCENT, creep_cmsx6::CREEP_EXTENSION_MAX_PERCENT] {
                assert!(creep_cmsx6::CreepState::try_new(&alloy, stress, edge).is_some());
            }
            let above = f64::from_bits(creep_cmsx6::CREEP_EXTENSION_MAX_PERCENT.to_bits() + 1);
            assert!(creep_cmsx6::CreepState::try_new(&alloy, stress, above).is_none());
            assert!(creep_cmsx6::CreepState::try_new(&alloy, stress, -f64::MIN_POSITIVE).is_none());
        }

        let dt = &body["dt_s"];
        assert_eq!(dt["ceiling_s"].as_f64().unwrap().to_bits(), alloy.gate_dt_s().unwrap().to_bits());
        assert!((dt["ceiling_s"].as_f64().unwrap() - 49.974).abs() < 1.0e-3);
        assert_eq!(dt["floor_s"].as_f64().unwrap(), 1.0);
        assert!(dt["nyquist_s"].is_null());
        assert!(dt["rule"].as_str().unwrap().contains("CreepExtensionBeyondCard"));
        assert!(dt["rule"].as_str().unwrap().contains("Below 0.1 percent the law is slow to start against every measured curve at every stress"));
        assert!(dt["rule"].as_str().unwrap().contains("from 0.1 to 5 percent at 170, 200 and 230 MPa it is within it on all nine tests at every one of 171 levels, the worst 0.289"));
        assert!(dt["rule"].as_str().unwrap().contains("1.70 to 1.78 percent against 0.79 to 0.83 measured, about a factor of 2"));
        assert!(dt["rule"].as_str().unwrap().contains("every frame there carries FLAG_OUTSIDE_MET_RANGE"));
        assert!(dt["rule"].as_str().unwrap().contains("StressNotHeld"));
        assert!(dt["rule"].as_str().unwrap().contains("Do not subtract one frame's time from another's"));

        assert!(dt["rule"].as_str().unwrap().contains("38.9 against 15.4 to 26.7 at 140 MPa, 7.8 against 2.7 to 3.5 at 170 MPa, 1.69 against 0.61 to 0.66 at 200 MPa and 0.245 against 0.06 to 0.08 at 230 MPa"));
        assert!(!dt["rule"].as_str().unwrap().contains("FLAG_BELOW_COMPARED_EXTENSION"));

        let flags = body["advisory_flags"].as_array().unwrap();
        assert_eq!(flags.len(), 2);
        assert_eq!(flags[0]["name"], "FLAG_CREEP_RATE_RISING");
        assert_eq!(flags[0]["mask"], 1);
        assert_eq!(flags[0]["seals"], true);
        assert_eq!(flags[0]["state"], "creep_rate_rising");
        assert_eq!(flags[1]["name"], "FLAG_OUTSIDE_MET_RANGE");
        assert_eq!(flags[1]["mask"], 2);
        assert_eq!(flags[1]["state"], "outside_met_range");
        let flags_slot = body["frame_slots"]
            .as_array()
            .unwrap()
            .iter()
            .find(|slot| slot["slot"] == "flags")
            .unwrap()
            .to_string();
        assert!(!flags_slot.contains("FLAG_BELOW_COMPARED_EXTENSION"), "{flags_slot}");
        assert!(flags_slot.contains("marks no failure of it"), "{flags_slot}");
        assert!(body["specimen"].as_str().is_some_and(|line| line.contains("Gap: below 0.1 percent it starts late, 1.5 to 4 times the hours to 0.05 percent") && line.ends_with("and a stress changed during a run (StressNotHeld).")), "{}", body["specimen"]);
        let slot = |name: &str| {
            body["frame_slots"]
                .as_array()
                .unwrap()
                .iter()
                .find(|slot| slot["slot"] == name)
                .unwrap()
                .clone()
        };
        for (name, state, unit) in [
            ("pos[0]", "creep_extension_percent", "%"),
            ("pos[1]", "creep_rate_percent_per_h", "%/h"),
            ("pos[2]", "stress_mpa", "MPa"),
            ("vel[0]", "work_by_load_mj_per_m3", "MJ/m^3"),
            ("vel[1]", "heat_dissipated_mj_per_m3", "MJ/m^3"),
            ("vel[2]", "work_defect_mj_per_m3", "MJ/m^3"),
            ("force_torque", "stress_mpa", "MPa"),
            ("residual", "residual_conservation_error", "1"),
        ] {
            assert_eq!(slot(name)["state"], state, "{name}");
            assert_eq!(slot(name)["unit"], unit, "{name}");
        }
        let step = serde_json::to_string(&body["example_compute_step"]).unwrap();
        assert!(step.starts_with(r#"{"domain":"materials","card":"creep_cmsx6","parameters":{"creep_extension_percent":0.1,"stress_mpa":200.0,"dt_s":49.97"#), "{step}");
        let run = &body["example_run_trajectory"];
        assert_eq!(run["segments"][0]["steps"], 720);
        assert_eq!(run["record_every"], 72);
        assert_eq!(run["initial"]["creep_extension_percent"], 0.1);
        assert_eq!(run["segments"][0]["load"]["stress_mpa"], 200.0);
    }

    /// The creep card's validation record: the source, the split, the record's
    /// hash with the three targets, the numbers beside them, and the limits.
    #[test]
    fn the_creep_validation_record_states_the_data_the_lines_and_the_limits() {
        let card = find("materials", "creep_cmsx6").expect("the creep card is served");
        let record = &describe_one(card)["validation"];
        let part = |name: &str| record[name].as_str().unwrap().to_string();
        for words in ["CMSX-6", "980 degrees Celsius", "140, 170, 200 and 230 MPa", "6 mm in diameter"] {
            assert!(part("object").contains(words), "{words}");
        }
        for words in [
            "doi 10.5281/zenodo.20132712",
            "Fitted on six, two at each of 140, 170 and 230 MPa: six constants",
            "all three at 200 MPa, a stress no fitted test has",
        ] {
            assert!(part("data").contains(words), "{words}");
        }
        for words in [
            "Record SHA-256 f20f99b0e3419d66201a3e124b5c216c4d138d41f976b19d6a536daf15316d87.",
            "over the six tests not in the fit",
            "RMS at most 0.15, worst at most 0.30, mean at most 0.10 in size",
        ] {
            assert!(part("pass_line").contains(words), "{words}");
        }
        for words in [
            "One cast, one temperature, one laboratory.",
            "Six constants are fitted on six tests of this cast",
            "the law was chosen with the fitted tests read, and the range at 140 MPa was set with all twelve read",
            "the 170 MPa tests predicted from 140 and 230 MPa alone: 0.118, 0.229, -0.039",
            "each of the twelve predicted from the other two at its stress gives RMS 0.075, the card 0.093",
            "gives 0.098, 0.237 and -0.013 on the same 33 times",
            "200 MPa lies where the stress dependence is straight",
            "a Norton power law gives 0.436 RMS and 0.952 worst",
            "Below 0.1 percent the card starts later than every measured curve at all four stresses",
            "38.9 against 15.4 to 26.7 at 140 MPa, 7.8 against 2.7 to 3.5 at 170 MPa, 1.69 against 0.61 to 0.66 at 200 MPa and 0.245 against 0.06 to 0.08 at 230 MPa",
            "The card answers there, and each frame carries FLAG_OUTSIDE_MET_RANGE",
            "The card answers past 0.2 percent below 170 MPa with this gap",
            "the worst 0.289, at 170 MPa and 0.33 percent on a test not in the fit, a margin of 0.011",
            "the card's 1 to 2.09 percent at 140 MPa is beyond any measurement",
            "it reaches 0.8 percent at 683 h where the two tests that got there took about 1000 h",
            "not physical constants of the alloy",
            "A second program, written apart, recomputes the same 33 values to 1e-12",
            "on the same machine",
        ] {
            assert!(part("limits").contains(words), "{words}");
        }
        assert_eq!(part("result"), CREEP_CMSX6_RESULT);
        assert_ne!(part("result"), NOT_COMPARED);
        for words in [
            "Over the 33 times to a level the six tests reached: RMS 0.093 in log time (target 0.15), worst 0.240 (target 0.30), mean +0.019 (target 0.10).",
            "On the three tests at 200 MPa, a stress no fitted test has: RMS 0.085, worst 0.170, mean +0.028 over 18 times.",
            "the card is 27 percent late",
            "over 32 times since load",
            "RMS 0.108, worst 0.249, mean +0.029",
        ] {
            assert!(part("result").contains(words), "{words}");
        }
        for record_word in ["met", "missed", "passed", "failed", "held-back", "blind", "registered", "amendment"] {
            for name in ["data", "pass_line", "result", "limits"] {
                assert!(!crate::spec_sheet::says(&part(name).replace("FLAG_OUTSIDE_MET_RANGE", ""), record_word), "{name} says {record_word}");
            }
        }
        assert!(!record.to_string().contains("seal"));
    }

    /// The capsule's description carries the locked entry state, and its
    /// examples start from it: one step, and the first 20 s at the ceiling.
    #[test]
    fn the_capsule_description_publishes_the_entry_state_and_starts_from_it() {
        let card = find("aerospace", "capsule_stardust").expect("the capsule is served");
        let body = describe_one(card);
        assert_eq!(
            serde_json::to_string(&body["entry_state"]).unwrap(),
            r#"{"radius_m":6503140.0,"radius_m_hex":"4158ceb900000000","radial_speed_m_s":-1842.1416729435698,"radial_speed_m_s_hex":"c09cc89112b64d71","transverse_speed_m_s":12767.792058801891,"transverse_speed_m_s_hex":"40c8efe5622ecd51"}"#
        );
        for (name, locked) in [
            ("radius_m", capsule_stardust::ENTRY_RADIUS_M),
            ("radial_speed_m_s", capsule_stardust::ENTRY_RADIAL_SPEED_M_S),
            (
                "transverse_speed_m_s",
                capsule_stardust::ENTRY_TRANSVERSE_SPEED_M_S,
            ),
        ] {
            let hex = format!("{:016x}", locked.to_bits());
            assert_eq!(body["entry_state"][format!("{name}_hex")], hex, "{name}");
            for (example, at) in [
                ("example_compute_step", "parameters"),
                ("example_run_trajectory", "initial"),
            ] {
                let sent = body[example][at][name].as_f64().expect(name);
                assert_eq!(sent.to_bits(), locked.to_bits(), "{example} {name}");
            }
        }
        assert_eq!(
            serde_json::to_string(&body["example_run_trajectory"]).unwrap(),
            r#"{"domain":"aerospace","card":"capsule_stardust","initial":{"radius_m":6503140.0,"radial_speed_m_s":-1842.1416729435698,"transverse_speed_m_s":12767.792058801891},"dt_s":0.05,"segments":[{"steps":400,"load":{}}],"record_every":10}"#
        );
        assert!(body["load_fields"].as_array().unwrap().is_empty());
        assert_eq!(card.ceiling_s, 0.05);
        assert!(card.nyquist_s.is_none());
        // Only the two capsules have a documented start.
        for other in all()
            .iter()
            .filter(|other| other.card != "capsule_stardust" && other.card != "capsule_insight")
        {
            assert!(describe_one(other).get("entry_state").is_none(), "{}", other.card);
        }
    }

    /// The InSight capsule's refusals, each with the sentence from its doc
    /// comment in the kernel source, word for word.
    #[test]
    fn the_insight_capsule_refusals_carry_the_sentences_from_the_card() {
        let documented = documented_rejects(CAPSULE_INSIGHT_SOURCE);
        let card = find("aerospace", "capsule_insight").expect("the InSight capsule is served");
        assert_eq!(documented.len(), 12);
        assert_eq!(card.rejects.len(), documented.len());
        for ((name, sentence), listed) in documented.iter().zip(card.rejects) {
            assert_eq!(name, listed);
            assert_eq!(
                reject_meaning("capsule_insight", name),
                Some(sentence.as_str()),
                "{name}"
            );
        }
    }

    /// The InSight capsule publishes the locked entry state and starts its
    /// examples from it; its one load field carries the kernel's limit on
    /// lift, both ends inclusive; its flags and frame slots name the state
    /// they come from; and its validation record names the data, both sets
    /// of marks with their hashes, and what the opened rows were used for.
    #[test]
    fn the_insight_capsule_publishes_the_entry_state_the_lift_bound_and_its_record() {
        let card = find("aerospace", "capsule_insight").expect("the InSight capsule is served");
        let body = describe_one(card);
        for (name, locked) in [
            ("radius_m", capsule_insight::ENTRY_RADIUS_M),
            ("radial_speed_m_s", capsule_insight::ENTRY_RADIAL_SPEED_M_S),
            (
                "transverse_speed_m_s",
                capsule_insight::ENTRY_TRANSVERSE_SPEED_M_S,
            ),
        ] {
            let hex = format!("{:016x}", locked.to_bits());
            assert_eq!(body["entry_state"][format!("{name}_hex")], hex, "{name}");
            for (example, at) in [
                ("example_compute_step", "parameters"),
                ("example_run_trajectory", "initial"),
            ] {
                let sent = body[example][at][name].as_f64().expect(name);
                assert_eq!(sent.to_bits(), locked.to_bits(), "{example} {name}");
            }
        }
        assert_eq!(
            serde_json::to_string(&body["example_run_trajectory"]).unwrap(),
            r#"{"domain":"aerospace","card":"capsule_insight","initial":{"radius_m":3522199.9999655997,"radial_speed_m_s":-1202.1585438056352,"transverse_speed_m_s":5633.705477158609},"dt_s":0.1,"segments":[{"steps":2000,"load":{"lift_over_drag":-0.031}}],"record_every":10}"#
        );
        assert_eq!(body["example_compute_step"]["parameters"]["lift_over_drag"], -0.031);
        assert_eq!(body["phase_fields"].as_array().unwrap().len(), 3);
        let load = body["load_fields"].as_array().unwrap();
        assert_eq!(load.len(), 1);
        assert_eq!(load[0]["name"], "lift_over_drag");
        assert_eq!(load[0]["unit"], "1");
        let limit = capsule_insight::LIFT_OVER_DRAG_LIMIT;
        assert_eq!(load[0]["minimum"].as_f64().unwrap().to_bits(), (-limit).to_bits());
        assert_eq!(load[0]["maximum"].as_f64().unwrap().to_bits(), limit.to_bits());
        assert_eq!(load[0]["minimum_exclusive"], false);
        assert_eq!(load[0]["maximum_exclusive"], false);
        let words = load[0]["bound"].as_str().unwrap();
        for part in ["from -0.08 to 0.08, both inclusive", "positive away from Mars", "Zero is a capsule with no lift", "-0.031"] {
            assert!(words.contains(part), "{part}: {words}");
        }
        assert_eq!(card.ceiling_s, 0.1);
        assert_eq!(card.floor_s, 0.1 / 1_048_576.0);
        assert!(card.nyquist_s.is_none());

        // The one flag is the lift.
        let flags = body["advisory_flags"].as_array().unwrap();
        assert_eq!(flags.len(), 1);
        assert_eq!(flags[0]["name"], "FLAG_LIFT_ACTING");
        assert_eq!(flags[0]["mask"], 1);
        assert_eq!(flags[0]["state"], "lift_acting");
        let served = body.to_string();
        for gone in ["6176", "FLAG_PAST_FLIGHT_DEPLOYMENT", "243.5 s", "219.8 s", "[4000, 128000] m and"] {
            assert!(!served.contains(gone), "{gone}");
        }
        for rule in [
            "ended as PastFlightDeployment, the end the card declares",
            "with no lift at step 2321, 232.1 s",
            "with a lift over drag of -0.031 at step 2106, 210.6 s, 5793.3 m and Mach 1.470",
            "Compared with the flight: runs from the locked entry state at 0.1 s",
            "any other start or path is computed on the same equations",
            "A restart from a returned state lands where the run does",
            "no limit depends on them",
        ] {
            assert!(served.contains(rule), "{rule}");
        }
        let slot = |name: &str| {
            body["frame_slots"]
                .as_array()
                .unwrap()
                .iter()
                .find(|slot| slot["slot"] == name)
                .unwrap()
                .clone()
        };
        for (name, state, unit) in [
            ("pos[0]", "altitude_m", "m"),
            ("pos[1]", "ground_distance_m", "m"),
            ("pos[2]", "mach", "1"),
            ("vel[0]", "air_speed_m_s", "m/s"),
            ("vel[1]", "radial_speed_m_s", "m/s"),
            ("vel[2]", "dynamic_pressure_pa", "Pa"),
            ("force_torque", "load_g", "g (Earth)"),
            ("residual", "residual_conservation_error", "1"),
        ] {
            assert_eq!(slot(name)["state"], state, "{name}");
            assert_eq!(slot(name)["unit"], unit, "{name}");
        }

        let record = &body["validation"];
        let part = |name: &str| record[name].as_str().unwrap().to_string();
        assert!(part("object").contains("26 November 2018"));
        for words in [
            "insight_edl",
            "MROM_2147",
            "the lander's barometer",
            "No number in the card comes from the accelerometer",
            "the last 59 s before the parachute are outside it",
        ] {
            assert!(part("data").contains(words), "{words}");
        }
        for words in [
            "ddafbbe8299da589e95a674bb31150f869d0f85d21415036bdbedfc4d7fa0a54",
            "9bb5b953e4ae325eb954fb3979b3942a08dc19bd5ff80ae2f6bbeda37b137146",
            "peak deceleration within 0.5 g",
            "radius at the parachute within 3.5 km",
            "twice the root sum of squares",
        ] {
            assert!(part("pass_line").contains(words), "{words}");
        }
        assert_eq!(part("result"), CAPSULE_INSIGHT_RESULT);
        assert_ne!(part("result"), NOT_COMPARED);
        for words in [
            "On the 59 s not in the fit, two runs",
            "at the flight's deployment time the radius 114 m low (target 3.5 km) and the Mach number 1.501 against 1.522 (target 0.2)",
            "With no lift: 17.1 s and 22.8 s late (target 15.1 s), 4.24 km high (target 3.5 km) and Mach 1.91",
            "the time of the peak is 4.02 s off (target 3.27)",
            "7.60 g against 8.12 g",
        ] {
            assert!(part("result").contains(words), "{words}");
        }
        for words in [
            "a single number measured on the first 150 s",
            "does not forecast an entry whose lift is not known",
            "The air and the lift were both chosen with the first 150 s read",
            "8 to 13 percent denser",
            "sideways lift",
            "13 have a compared value on a state below 5700 m or Mach 1.46",
            "The card ends a run at the flown parachute deployment",
            "no one outside the company has repeated them",
        ] {
            assert!(part("limits").contains(words), "{words}");
        }
        assert!(!part("limits").contains("not yet repeated by a second party"));
        assert!(!record.to_string().contains("seal"));
    }

    /// The generator's refusals, each with the sentence from its doc comment
    /// in the kernel source, word for word.
    #[test]
    fn the_generator_refusals_carry_the_sentences_from_the_card() {
        let documented = documented_rejects(GENERATOR_SOURCE);
        let card = find("energy", "generator_mmrtg_f1").expect("the generator is served");
        assert_eq!(documented.len(), 9);
        assert_eq!(card.rejects.len(), documented.len());
        for ((name, sentence), listed) in documented.iter().zip(card.rejects) {
            assert_eq!(name, listed);
            assert_eq!(
                reject_meaning("generator_mmrtg_f1", name),
                Some(sentence.as_str()),
                "{name}"
            );
        }
        let account = reject_meaning("generator_mmrtg_f1", "ConverterAccountBreach").unwrap();
        assert!(
            account.ends_with("bookkeeping that follows from the model and not a conservation law."),
            "{account}"
        );
    }

    const TI64_SOURCE: &str = include_str!("../../genesis_kernel/src/ti64_lattice.rs");
    const SWING_SOURCE: &str = include_str!("../../genesis_kernel/src/swing.rs");
    const BATTERY_SOURCE: &str = include_str!("../../genesis_kernel/src/battery_sag.rs");
    const RESONANCE_SOURCE: &str = include_str!("../../genesis_kernel/src/resonance.rs");
    const HYDRAULIC_SOURCE: &str = include_str!("../../genesis_kernel/src/hydraulic_fluid.rs");

    /// The five cards that gained refusals of their own list every variant of
    /// their `StepReject`, in the kernel's order, and carry its sentence word
    /// for word. The titanium card has no advisory flag: yield and a residual
    /// past the limit are refusals.
    #[test]
    fn the_refusals_of_the_five_bounded_cards_carry_the_sentences_from_the_card() {
        for (domain, name, source, count) in [
            ("materials", "ti64_lattice", TI64_SOURCE, 9),
            ("energy", "swing", SWING_SOURCE, 9),
            ("energy", "battery_sag", BATTERY_SOURCE, 10),
            ("signal", "resonance", RESONANCE_SOURCE, 10),
            ("materials", "hydraulic_fluid", HYDRAULIC_SOURCE, 10),
        ] {
            let documented = documented_rejects(source);
            let card = find(domain, name).expect(name);
            assert_eq!(documented.len(), count, "{name}");
            assert_eq!(card.rejects.len(), documented.len(), "{name}");
            for ((variant, sentence), listed) in documented.iter().zip(card.rejects) {
                assert_eq!(variant, listed, "{name}");
                assert_eq!(
                    reject_meaning(name, variant),
                    Some(sentence.as_str()),
                    "{name} {variant}"
                );
            }
        }
        let titanium = find("materials", "ti64_lattice").expect("ti64_lattice");
        assert!(titanium.advisory.is_empty());
    }

    /// The generator's examples stay on the card: one day from landing, and
    /// thirty daily steps from landing.
    #[test]
    fn the_generator_examples_are_one_day_and_thirty_days_from_landing() {
        let card = find("energy", "generator_mmrtg_f1").expect("the generator is served");
        let body = describe_one(card);
        assert_eq!(
            serde_json::to_string(&body["example_compute_step"]).unwrap(),
            r#"{"domain":"energy","card":"generator_mmrtg_f1","parameters":{"mission_time_s":0.0,"dt_s":86400.0}}"#
        );
        assert_eq!(
            serde_json::to_string(&body["example_run_trajectory"]).unwrap(),
            r#"{"domain":"energy","card":"generator_mmrtg_f1","initial":{"mission_time_s":0.0},"dt_s":86400.0,"segments":[{"steps":30,"load":{}}],"record_every":1}"#
        );
        assert!(body["load_fields"].as_array().unwrap().is_empty());
        assert_eq!(card.load_names.len(), 0);
        assert_eq!(card.phase_names, vec!["mission_time_s"]);
        assert!(card.ceiling_s > 4.2e6 && card.ceiling_s < 4.3e6);
    }

    #[tokio::test]
    async fn worked_examples_round_trip_and_land() {
        let state = crate::new_gateway_state();
        let mut examples = 0_u32;
        for item in all() {
            let body = describe_one(item);
            let rejects = body["rejects"].as_array().expect("rejects");
            assert!(!rejects.is_empty(), "{}", item.card);
            for reject in rejects {
                let name = reject["name"].as_str().expect("name");
                assert!(
                    reject["meaning"].is_string(),
                    "{} {name} has no doc comment",
                    item.card
                );
            }
            let step_text = serde_json::to_string(&body["example_compute_step"]).unwrap();
            let step: Value = serde_json::from_str(&step_text).unwrap();
            assert_eq!(step["domain"], item.domain);
            assert_eq!(step["card"], item.card);
            match crate::compute_step(&state, step).await {
                Ok(response) => assert_eq!(response.sha384_seal.len(), 96, "{}", item.card),
                Err(error) => panic!("{} step: {}", item.card, error.message()),
            }
            examples += 1;
            let run_text = serde_json::to_string(&body["example_run_trajectory"]).unwrap();
            let run: Value = serde_json::from_str(&run_text).unwrap();
            assert_eq!(run["domain"], item.domain);
            assert_eq!(run["card"], item.card);
            match crate::trajectory::run(&state, run).await {
                Ok(response) => assert_eq!(
                    response.outcome, "completed",
                    "{} {:?}",
                    item.card, response.reject_detail
                ),
                Err(error) => panic!("{} trajectory: {}", item.card, error.message()),
            }
            examples += 1;
        }
        // 0.15.0: 36 cards, two worked examples each.
        assert_eq!(examples, 72);
    }
}
