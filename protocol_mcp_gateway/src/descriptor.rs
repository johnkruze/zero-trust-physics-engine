//! One description of each domain/card pair.
//!
//! Request parsing reads [`parameter_names`], [`phase_names`], and [`load_names`]
//! from this list. The numbers come from `genesis_kernel` helpers and constants.

use std::sync::OnceLock;

use genesis_kernel::{
    battery_sag, cable_stretch, cantilever_eth, capsule_stardust, cell_18650pf, cell_2170_m80,
    cell_2170_model3, generator_mmrtg_f1, grid_node, hydraulic_fluid, reactor_spert3,
    reactor_spert3_fuel, resonance, satellite_lageos1, swing, tesseract, thermal, ti64_lattice,
    reactor_triga_jsi,
    reactor_spert3_ccore,
    creep_cmsx6,
    runaway_18650_lco,
    capsule_insight,
    crater_ryugu_sci,
    link_gothenburg_10010,
    grid_gb,
    solar_system,
    tire_eve_r19_ice,
    accumulator_sbo450,
    los_jwst_fine_guide,
    tokamak_jet_dte1,
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
pub(crate) const SHORT_FORM_SAYS: &str = "With form \"short\" the answer is a page, enough to write a first request: what the model is of, how it did against measurement, each input with its unit and range, the names of its stops, and a worked request. Only the full form, the answer when form is left out, has each bound's reason, the validation record and each stop's meaning: it is the form that explains a refusal and the form a result is relied on with.";

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
            // how the model did against its own measurements, in one word, before its sentence
            if let Some((word, _)) = crate::find_model::verdict_of_card(item.domain, item.card) {
                row.insert("against_measurement".to_string(), json!(word));
            }
            row.insert("specimen".to_string(), json!(item.specimen));
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
/// written twice; the verdict is the one `find_model` and `list_cards` give.
pub fn describe_short_in(view: &View, domain: &str, card: &str) -> Option<Value> {
    let full = describe_value_in(view, domain, card)?;
    let fields = |key: &str| Value::Array(full[key].as_array().into_iter().flatten().map(short_field).collect());
    let names = |key: &str, name: &str| Value::Array(full[key].as_array().into_iter().flatten().map(|row| row[name].clone()).collect());
    let mut short = Map::new();
    short.insert("domain".to_string(), full["domain"].clone());
    short.insert("card".to_string(), full["card"].clone());
    short.insert(FORM.to_string(), json!("short"));
    short.insert("specimen".to_string(), full["specimen"].clone());
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
            "This is the short form: phase_fields are the keys of initial, load_fields the keys of each segment's load, a null minimum or maximum is a bound the full form gives in words, and stops are the names a refusal carries. describe_card with form \"full\", or with no form, has {FULL_FORM_HAS}."
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
            "run_seal": { "type": "string", "pattern": "^[0-9a-f]{64}$", "description": "The run_seal of the run_trajectory answer: with request, or as part of the whole answer." },
            "final_state": {},
            "final_residual_conservation_error_hex": { "type": ["string", "null"] },
            "max_residual_conservation_error_hex": { "type": ["string", "null"] },
            "state_digest": { "type": ["string", "null"] },
            "chain_seal": { "type": "string", "pattern": "^[0-9a-f]{64}$", "description": "With request and run_seal: a chained answer's chain_seal." },
            "links": { "type": "array", "items": { "type": "object" }, "description": "Alone: a chain's answers in order." }
        },
        "additionalProperties": true,
        "description": "Either request and run_seal alone, the short form: the run is computed again and its seal compared with run_seal. Or the whole run_trajectory answer sent back as it was returned, without request, the long form: inputs, frames, outcome, steps_landed, reject, run_seal, final_state, final_residual_conservation_error_hex, max_residual_conservation_error_hex and state_digest are then all required, along with the rest of the answer. The trajectory chain, the state digest and the run seal are recomputed from its exact hex values; then its request is computed again under a physics permit and the limits of run_trajectory, and the engine's whole answer to it compared with the answer sent, value by value and then byte for byte, the receipt included, so an answer that holds is every byte the engine's. A seal has no key: an answer whose values were rewritten with its seals worked out again holds every seal, and only the computation shows it is not the engine's. An answer that continues a sealed upstream run (its chain names upstream_run_seal) carries the upstream's seal and not its request, and its long form is refused (AnswerCarriesNoRequest): its chain's answers in order, as links, carry each request, and each link is computed again from the links before it. Or links alone: a chain's answers in order, held as the long form holds one. A design_run answer sent whole is designed again and held the same way. The long form's answer carries holds, true when the answer sent is every byte the engine's, and values_hold beside it, true when every value of it is the engine's answer to its request computed again, whatever its spelling: each number by its 64-bit value, strings, flags and null exactly, keys and their order, the receipt string among them. A copy whose numbers a client wrote again in its own spelling (0.0001 for 1.0e-4, 12 for 12.0) has holds false and values_hold true, and differs_at names the first place its text differs."
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

fn build() -> Vec<Card> {
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
        reactor_spert3_card(),
        cell_2170_m80_card(),
        cantilever_eth_card(),
        cell_2170_model3_card(),
        creep_cmsx6_card(),
        capsule_insight_card(),
        reactor_spert3_fuel_card(),
        crater_ryugu_sci_card(),
        card_generator_mmrtg_seebeck_descriptor::card(),
        card_momentum_jwst_dihedral_descriptor::card(),
        card_momentum_jwst_srp_descriptor::card(),
        link_gothenburg_10010_card(),
        grid_gb_card(),
        solar_system_card(),
        tire_eve_r19_ice_card(),
        los_jwst_fine_guide_card(),
        tokamak_jet_dte1_card(),
        fingerpad_p02_glass_card(),
        accumulator_sbo450_card(),
        reactor_triga_jsi_card(),
        reactor_spert3_ccore_card(),
        runaway_18650_lco_card(),
        card_grasp_lift_glass_4tu_descriptor::card(),
        tokamak_vde_mast_card(),
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
                "above 0 kg (GasMassNotPositive): the nitrogen in the accumulator, which never changes. A charge is the vessel's 0.58879 l full of nitrogen at the precharge, absolute, and 293.15 K by the card's own equation of state: 0.013653 kg for 20 bar and 0.027467 kg for 40 bar. Each band is a thousandth either side, so a charge typed to four figures is inside, as the worked examples' 0.02747 kg is. Compared at those two charges only, 0.013639 to 0.013667 kg and 0.02744 to 0.027495 kg; any other mass sets FLAG_OUTSIDE_COMPARED_RUNS, and through engine 0.9.0 it was refused as GasMassOutsideCard. The card as it was scored on 2026-10-09 held a smaller vessel and its charges were 0.01336 and 0.02688 kg".into(),
            ),
            scalar(
                "gas_pressure_pa",
                Role::Phase,
                "Pa",
                Some(0.0),
                true,
                Some(accumulator_sbo450::PRESSURE_RATED_PA),
                false,
                "above 0 Pa and at most 33000000 Pa, absolute: the gas pressure at the start; the card works the gas volume back from it. 330 bar is the vessel's rated maximum working pressure, the rating HYDAC's SBO catalogue gives the type SBO450-0.6 in its dimension table, 4700 psi; the two 0.6 l parts it lists are rated 3600 psi, 250 bar, and which the rig carried is not on disk (PressureAboveRated); a gas volume past the vessel's, below the precharge, has no oil left (OilSideEmpty). Compared at 37.76 to 42.71 bar at 20 / 40, 55.26 to 66.37 bar at 20 / 60, 57.36 to 63.14 bar at 40 / 60 and 75.31 to 85.55 bar at 40 / 80, held as 37 to 43 and 55 to 67 bar for the 20 bar charge and 57 to 64 and 75 to 86 bar for the 40 bar charge; a pressure outside its charge's bands sets FLAG_OUTSIDE_COMPARED_RUNS. At 109 to 132 bar, 40 / 120, the card as scored was compared and missed: 2.72 percent median error of the swing against the line of 2.0, three runs above 130 bar unanswered".into(),
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
                "the speed of the piston, held over a step. Positive pushes oil in and compresses the gas. A speed times dt_s above 1 mm, 1.5 percent of the smallest compared gas volume, is refused as PistonTravelAboveStep. The fastest compared run, 5 mm at 20 Hz, reached 0.69999 m/s as the mean over a step, above 1 Hz 212 runs at 0.59 percent median error of the swing; a faster speed sets FLAG_OUTSIDE_COMPARED_RUNS, and through engine 0.9.0 it was refused as PistonSpeedOutsideCard".into(),
            ),
        ],
        floor,
        "the ceiling divided by 2^20, about 3.34e-7 s",
        gate,
        "0.3504 s, a fiftieth of the thermal time constant of 17.52 s",
        None,
        "Over a step the piston speed is held, and the gas volume changes by the piston area, 19.635 cm2 (a 50 mm bore), times the travel. The gas is nitrogen by the Peng-Robinson equation of state; it exchanges heat with the wall with one time constant, 17.52 s; the vessel's gas volume is 0.58879 l. The time constant and the ratio of the area to the volume are fitted to this accumulator's own runs. A run starts at time zero with the work and the heat at zero; the time, the work, the heat and the energy at the start are the anchor of the ledger and begin again at each start; the ledger's conservation balance is held from each start, and no limit of the card's range depends on them. The card keeps two more sums over the steps of the motion: M, the absolute work of each step, and H, the absolute heat of each step. A start carries them in as work_moved_j and heat_moved_j, 0 for gas that has not moved; each step adds to them; the state gives them back under the same names, and heat_share is H over the larger of M and 100 J. A step lands only if H is at most 0.065 of the larger of M and 100 J; otherwise it is refused as HeatShareAboveCard and the run ends there, sealed, with its proof up to the last step that landed. In the measured sines that refuses 0.1 Hz and slower and answers 0.2 Hz and faster. A piston at rest lands for as long as the gas is at the wall's temperature; off it, a rest from the start of a run is answered for its first 6.5 J of heat. A single step or a run is carried on by handing back its returned gas_pressure_pa, gas_temperature_k, work_moved_j and heat_moved_j as a new start: the two sums travel with the motion, so single steps handed back, runs carried on from each other and one run are weighed alike, and the step a run refuses is refused when it is asked alone from that run's returned state. A start without the two sums is refused as a missing input. The four limits the card works out itself (the travel in a step, the pressure and the volume worked back from the starting pressure, and the heat against the work) carry an allowance of a relative 1e-12, so a start, a step or a run stated or landing exactly at a limit is inside it. A run ends in a sealed refusal at the step that would land above the rated 330 bar, at or below nitrogen's critical temperature, past the vessel volume, or past the share of heat. A step outside the range compared with measurement, a gas mass other than the two charges, a pressure outside its charge's bands, a wall or a gas temperature or a piston speed outside the compared, lands with FLAG_OUTSIDE_COMPARED_RUNS: the Peng-Robinson gas, the one time constant and the ledger are the same there, and through engine 0.9.0 each was a refusal.".to_string(),
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
                "inclusive [{}, {}] m: the crater's radius at the level of the undisturbed surface, up to the radius at which this crater stops. The speed of the wall is the one this impact gives it at that radius; the caller does not set it. Below 0.1622 m the energy the wall has yet to give up, over 22 800 J/kg, is so large that one rounding of it in 64 bits passes a tenth of the ledger's limit of 5.064e-11 J/kg, and the start is refused as RadiusBelowLedgerResolution; that radius is 4.3 radii of the projectile's equivalent sphere and the wall there moves at 214 m/s, a tenth of the projectile's speed. Only the final radius was compared with Ryugu, 8.571 m against the measured 7.25 +/- 0.4 m, 18.2 percent above and outside the allowance of 0.97 m fixed beforehand, a miss; the growth and its clock, read by eye from a laboratory figure at 1 g, were not compared at any radius. The card served from 1.75 m until engine 0.9",
                token(crater_ryugu_sci::RADIUS_FLOOR_M),
                token(crater_ryugu_sci::FINAL_RADIUS_M)
            ),
        )],
        floor,
        "the ceiling divided by 2^20: a step that moves the wall by about a tenth of a micrometre at its fastest. Under it a step is mostly rounding",
        gate,
        "0.25 s, a locked constant: about 1,600 steps from 1.75 m to the stop. Over that whole growth the largest conservation residual at this step is 5.8e-8 against a limit of 1e-6; at 0.5 s it is 8.8e-7 and at 1 s the first step is refused. Nearer the impact the wall is faster and a step of 0.25 s is refused by the ledger: from 0.1622 m a run takes steps of a few microseconds at first, growing with the radius",
        None,
        "No load: nothing acts on the crater but the impact and Ryugu, and a run segment carries an empty load object. A run starts at time zero at the radius given, with the wall at the speed this impact gives it there; 1.75 m is 0.917 s after the impact by the card's clock, and the worked example starts there. A single step starts in the same way at each call: its radius_m handed back as a new start goes on with the wall at the speed the card sets from that radius, not the speed the earlier steps integrated, and t_s begins again at zero; through from, a continuation is served only from the crater stopped (ContinuationNotOnCard before). The radius only grows. On the step in which the wall would turn back, the crater is set at 8.571304329477922 m with the wall at rest, the frame carries FLAG_STOPPED, and from then on a step moves only the time. From 1.75 m at the step ceiling that is step 1,597, at 399.25 s. The time is the clock of Gault and Wedekind (1977), which runs until the ejecta near the rim have landed, and is not compared with Ryugu.".into(),
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
        "ti64_lattice, the company's own specimen: a three-strut annealed Ti-6Al-4V lattice column, yield 880 MPa, modulus 113.8 GPa, under nodal force, torque and temperature to 1268.15 K.",
        vec![
            position,
            vec3_within(
                "velocity_m_s",
                "m/s",
                yield_speed_m_s,
                format!(
                    "each component from -{0} to {0} m/s, both inclusive. The bound is the highest speed along a ligament that a run under forces and temperatures inside their ranges can approach, from rest or from a turning point, before that ligament passes 880 MPa: 2 omega sqrt(x_y (x_y + g)), with omega the highest natural frequency, x_y = {1} m the yield position at 293.15 K and g = {2} m the growth of a ligament heated from 250 K to 1268.15 K. It is approached with the largest force pulling and the ligament heated as the node moves, so that its stretch stays at yield in compression up to the beta transus and then runs out to yield in tension. With the temperature held, the highest is 2 omega x_y, {3} m/s; a run of the largest force from -x_y lands 45.2537 m/s. A faster node cannot come to rest before that ligament passes 880 MPa under any force and temperature in the range, and the card refuses it as YieldReached. A position and a velocity each inside its range can together still carry a ligament past yield during a run: that step is refused as YieldReached, on the incoming state, at a turning point of the motion inside the step, or on the landing, and the run is sealed as refused",
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
                "from 0 s up, inclusive: the clock of the start. A new start states 0. A state the card returned is handed back with its t_s as start_t_s, and the clock runs on from it as a running sum, t_s += dt_s. A clock below zero is refused as StartNotWritten. With start_force_n, start_temperature_k, start_energy_anchor_j and start_boundary_work_j it carries the run across a restart: a run handed back its own returned state with these five values steps on exactly as the run straight through, bit for bit in every field but torque_nm, which is the applied couple of the last step, and its ledger is weighed against the conservation limit over the whole of it. The card cannot tell the values a run returned from values a caller states: any start whose start_energy_anchor_j and start_boundary_work_j leave a defect inside the conservation limit is accepted by the card and weighed from that defect, a returned state restated as new (start_t_s 0, start_boundary_work_j 0, start_energy_anchor_j its own energy) among them, and its proof carries the values stated. The ledger is weighed over a whole chain only when each start hands back the values the card returned.".to_string(),
            ),
            vec3_within(
                "start_force_n",
                "N",
                yield_force_n,
                format!(
                    "each component from -{0} to {0} N, both inclusive, the range of force_n: the force the start stands under, which a returned state gives as applied_force_n. A new start states the force its start_energy_anchor_j is taken under; the card's rest has none. It enters only the ledger: the first step books the change from it to the step's own force_n as boundary work",
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
                    "each component from -{0} to {0} N, both inclusive. The bound is the specimen's yield: the yield strength, 880 MPa, times the section of a ligament, the force that holds a ligament at yield at rest. The load rule of dt_s also holds; at every step the card accepts it allows more than this maximum, so the maximum is reached first. A force inside the range, held from rest, carries the node out to twice its rest deflection: above half the maximum that excursion passes yield, the step that holds it is refused as YieldReached, at its turning point if no landing is past yield first, and the run is sealed as refused",
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
                    "inclusive [{}, {}] K. Yield is tested on the incoming state at its start_temperature_k and again at the instant this temperature is applied. From the rest position the cell yields from heat alone above about 1192.32 K, so a higher temperature is refused as YieldReached unless the start position is part of the way out. The published maximum is inclusive and is answered, but never from rest with no force: a step at exactly 1268.15 K lands from a start position of 3e-5 m in each component, where the ligaments are already stretched, and from the rest position the largest temperature that lands is 1192.3203110311847 K. A start position that is at rest only when hot is past yield at 293.15 K: stated with a start_temperature_k of 293.15 K it is refused on the first step, and stated with its own hot start_temperature_k it lands",
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
        "grid_node, the company's own specimen: a 48 V DC-plant node, a 50 uH feeder into a 50 uF shunt that rings at 20 krad/s, with an AWG 34 copper fuse link 0.1601 mm across and 120 mm long that melts at 1357.77 K, between an upstream voltage and a load current. It holds an energy ledger of the feeder, the shunt and the copper's heat.",
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
        "No scalar load Nyquist. The step refuses when the fuse temperature reaches the melt point. energy_anchor_j, boundary_work_j, energy_defect_j and the residual are counted from the state this request starts from, and voltage_spike_v is the largest |voltage_v| since that start: a restart from a returned state starts all of them again, so they can differ from a straight run while current_a, voltage_v and temperature_k agree. No limit depends on voltage_spike_v, and the conservation gate is held on each step from the new start.".into(),
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
        "swing, the company's own specimen: one round-rotor synchronous machine on a 60 Hz bus, inertia constant 5 s, damping 0.05 pu, peak electrical power 1.04 pu and a phase-locked loop of 20 per second, under mechanical power, held in synchronism inside a rotor angle of plus or minus pi/2.",
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
        "No scalar load Nyquist. The rotor must start inside +/- pi/2, with a speed deviation of at most 6 pi rad/s and a dispatch of at most 2 pu either way. The transient energy, transient_energy_pu, is U = 1/2 M delta_omega^2 - p_mech delta - 1.04 cos delta, with M = 2 x 5 s / (2 pi 60 rad/s). A run starts at time zero with its ledger anchored at the angle, the speed and the dispatch given: energy_anchor_pu is U at that angle and speed under the p_mech_pu of the first segment, and boundary_work_pu starts at zero. A first step held at that dispatch books no change of dispatch. boundary_work_pu is the energy that has entered the rotor since the anchor: the damping, minus 0.05 pu per rad/s times delta_omega squared, integrated over each step, and at each later change of dispatch the change of U it makes at the angle before the step, -(new p_mech - old p_mech) x delta. energy_defect_pu is transient_energy_pu - energy_anchor_pu - boundary_work_pu, the energy the integration created (positive) or lost (negative); residual_conservation_error is its size over the synchronism barrier, the gap in U from the stable to the unstable equilibrium at 1 pu. A step whose defect is above 1e-6 of the barrier is refused as SpontaneousGeneration, and one below -1e-6 of it as FirstLawBreach. A single step starts in the same way at each call: its delta_rad, delta_omega_rad_s and pll_err_rad handed back as a new start continue the same motion, and t_s, the anchor, the boundary work and the residual begin again. No refusal depends on the ledger across calls: the conservation gate weighs each step against the run it is in.".into(),
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
        "battery_sag, the company's own specimen: a 12S lithium-ion pack of 22 Ah, 50.4 V full and 36.0 V empty, with a straight-line rest voltage and a 0.060 ohm resistance that doubles as it empties, under a terminal current. It holds a chemical-energy ledger, a 42.0 V warning, a 36.0 V cutoff and a 50.4 V charging limit.",
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
                    "inclusive minimum {}, inclusive maximum 1. The minimum is the lowest state of charge at which the open-circuit terminal voltage, {} V + {} V * soc, is above the {} V cutoff; a lower value is refused. A step from a low state of charge lands only if the landing state of charge, soc - current_a * dt_s / {} C, still has a terminal voltage above the cutoff: with a discharging current the lowest usable soc is higher than the minimum and rises with current_a and dt_s (UndervoltageCutoff). For any current the terminal voltage at the landing state of charge s' is {} V + {} V * s' - current_a * {} ohm * (2 - s'), with current_a positive when discharging",
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
                    "The largest charging current (negative) that is accepted depends on the state of charge and on dt_s. The published exclusive minimum -{} A is approached only as the state of charge goes to zero; at a full pack no charging current is accepted: at soc 1 every negative current_a, however small, is refused as OvervoltageCutoff. The rule: a charging current of size I at state of charge s and step dt_s lands when, with the landing state of charge s' = s + I * dt_s / {} C, {} V + {} V * s' + I * {} ohm * (2 - s') is at most {} V (OvervoltageCutoff). The table charging_current_limit_by_state_of_charge gives the largest accepted size, generated from the card, for a step of {} s. Both signs are bound by the load rule: {}",
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
                "above 0 A, a discharge (CurrentNotDischarging), within 0.83 mA of held_current_a. The 1C span that passed its comparison is 2.89817 to 2.90065 A, the measured 2.899 to 2.89982 A widened on each side by the 0.83 mA band; any other current sets FLAG_CURRENT_OUTSIDE_PASSED_SPAN".into(),
            ),
        ],
        floor,
        "the ceiling divided by 2^20: at 3.0 A, five parts in a thousand million of state of charge. Under it a step is mostly rounding",
        gate,
        "the time to cross one table interval at 3.0 A, the value here, for every held current at or below 3.0 A; above it, the time to cross one at the run's own held current, and a step above that is refused as DtAboveCeiling with the ceiling stated",
        None,
        "current_a is any discharge current above 0 A. The equations, the charge count and the ledger are the same at every current. Compared on eleven held 1C discharges from full charge, fed the tester's logged current, 2.899 to 2.89982 A: all eleven met the line of 50 mV, 27.8 to 30.0 mV rms, largest single error 57 mV, capacity +0.04 percent on the one run that reached cutoff. One discharge at another current was left out of the fitting: 04-27-17 3541_dis5_10p.mat, the same cell at 25 degrees Celsius discharged at a held 0.86888 to 0.86970 A from full charge to 2.5 V in 14 holds with rests at 0 A between them. Compared on 2026-10-10, each hold run as a continuation of the one before and the rests not compared, at steps of 0.1 s, it missed: the voltage error during the holds was 56.7 millivolts root mean square against the line of 50, rising to 204 and 206 in the last two holds, and the card did not end by its own cutoff but left its table at soc 0.03 and 2.66 V (StateOfChargeOutsideCard), 71 s after the cell reached 2.5 V, having delivered 0.60 percent more than the cell: evidence/research/panasonic_18650pf/held_0p87A_comparison. The rest-voltage table was fitted at 0.145 A and the resistance on 0.87 A holds; no other current was compared. Through engine 0.9.0 every current outside the 1C span was refused; now a step at such a current lands with FLAG_CURRENT_OUTSIDE_PASSED_SPAN. From about 1.1 A up the 2.5 V cutoff ends a discharge (UndervoltageCutoff); below it the bottom of the table, soc 0.03, comes first (StateOfChargeOutsideCard). A current of zero or less, a rest or a charge, is refused as CurrentNotDischarging: the card has no charging limit at 4.2 V and no relaxation for a rest after a discharge. A current that is not a discharge in any segment of a run refuses the whole run before its first step. A run is one discharge at one held current: every segment's current_a must lie within 0.00083 A (HELD_CURRENT_BAND_A) of the held current, and a segment outside that band refuses the whole run before its first step as CurrentChangedDuringDischarge, with nothing sealed. The held current is held_current_a, which the request states. A returned state does not carry held_current_a; to continue from it, the request states the same held_current_a as the run that returned it. Held means what the tester held: the measured current moves 0.82 mA within a run, one step of the 0.81 mA in which the tester logs its current; the dataset states no accuracy for the tester, so the band is that spread, written as 0.83 mA, and the test allows four units in the last place at the size of the currents. A pulse and a drive cycle are refused: the card has no memory, the real cell's voltage relaxes for minutes after a change of load, and no changing current was compared with a measurement of this cell. Every discharge that passed the comparison started at full charge, soc 1; the 0.87 A comparison ran its holds as continuations from partial charge. A start below full charge is a rested cell at the rest-voltage table's own value, or a continuation; the two are the same physics on this card, which has no memory. Through engine 0.9.0 a start below full charge was taken only as a continuation. A run continued from a state another run returned, at the same held current, gives the same state of charge and terminal voltage as the straight run; the ledger's anchor, time and work start again at the continuation, and no limit depends on them. A single step takes the state of charge and held_current_a only: time, work and the ledger's anchor start again at each call. A run whose end in decimal is exactly the bottom table point, soc 0.03, lands its last step: a state of charge summed in binary that lands under 0.03 by no more than one unit in the last place at 1 for each step since the run or continuation began, plus four, lands on 0.03. At 1C the 2.5 V cutoff stops a discharge near soc 0.05, before that point. A step of zero or less is refused as NonPositiveDt before the step floor is tested.".into(),
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
        "thermal, the company's own specimen: an aluminum 6061 avionics cold plate, 250 x 125 x 7.8 mm, heat capacity 590.625 J/K and 2 K/W to its bath, under heater power and a bath temperature, stepped by the exact exponential map from -40 to 125 degrees Celsius, the industrial rating of the electronics it carries.",
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
        "resonance, the company's own specimen: a 1 kg mass on a 10 Hz mount, damping ratio 0.05 and stroke 15.625 mm, under an imposed acceleration, up to the energy of a free ring of twice the stroke.",
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
        "tesseract, the company's own specimen: an 80 Hz drive-mode Duffing resonator, damping ratio 1/80, cubic stiffness 6e8 per square metre per square second and proof mass 1e-7 kg, under acceleration, an external rate and sensed velocity, up to an amplitude of 7.8125 mm.",
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
                    "from -{0} to {0} J, both inclusive: the signed ledger defect, mechanical energy less the anchor less the booked work, that the run reaching this state carried, and 0 for a new run. The limit is 1e-6 of the energy of the mode at the amplitude limit, the same limit every step is held to. A state handed back continues its run only with its own energy_defect_j; a run restarted with 0 is a new run, and its ledger begins again",
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
            "{}. A run starts at time zero with its ledger anchored so that the defect is the energy_defect_j given: the anchor is the starting mechanical energy less that defect and the booked work is zero. A run or a single step whose final state is handed back as a new start continues the same motion, and continues the same ledger only when the request carries that state's energy_defect_j; t_s begins again at zero. A start without energy_defect_j is refused as a missing field. A step that would land the displacement or the velocity outside its range is refused as AmplitudeOutsideCard, and a run that reaches one is sealed up to its last landed step",
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
        "hydraulic_fluid, the company's own specimen: a 0.50 m mineral-oil column in a 50 mm bore, bulk modulus 1.72 GPa and density 870 kg/m^3, charged at one atmosphere, under a commanded flow, from its vapour pressure of 10 Pa to 20.74 MPa absolute.",
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
        "cable_stretch, the company's own specimen: a 1.20 m Dyneema tendon 1.50 mm in diameter, modulus 120 GPa, damping ratio 0.01 and ultimate stress 2800 MPa, 4948 N at 28 mm of elongation, under a commanded force, in tension up to the tension at which it snaps.",
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
                    "[0, {}) m, the ultimate elongation, where the elastic stress reaches the ultimate 2800 MPa and k x reaches the ultimate tension of {} N. A rope has no compressive branch: an elongation below 0 is a rope shorter than its free length and is refused as Slack. An elongation at or above the ultimate elongation is a failed braid and is refused as TensileFailure. The damper term does not widen this range. The initial tension k x + c v must also be in [0, {}) N",
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
                "inclusive [0, {}] s. The maximum is where the decay heat the fuel still holds falls to the ledger's resolution, 1e-6 of the heat released from landing to mission day 4,087, about 2,070 years after landing: past it a breach of the ledger the size of its limit could not be seen (DecayHeatBelowLedgerResolution). The published record ends on mission day 4,087 ({} s); a later start is answered with FLAG_PAST_RECORD. The returned state gives the mission time as t_s",
                token(ledger_end_s),
                token(last_s)
            ),
        )],
        floor,
        "the ceiling divided by 2^20: one part in a thousand million of thermal power. Under it a step is mostly rounding",
        gate,
        "0.02 rad on its fastest rate, fuel decay plus converter fading",
        nyquist,
        "No load. A run segment carries an empty load object. Mission days 2,809 to 4,087 are the held-back days the card was compared with and failed on (rms 0.008129 of 115.4 W inside its line of 0.010, final-year mean +0.010094 outside it; +0.0031, +0.0045, +0.0096 over successive years and +0.0107 over the last 184 days); it answers them as the model that was scored, with no constant changed since. Mission day 4,087 is the last day of the published record and not an end of the generator: a step that lands past it by more than the clock's tolerance lands with FLAG_PAST_RECORD set, the same exact map carried forward, and since the card read high and the gap grew each year, later days are expected to read higher still. The card stops where the decay heat still held falls under the ledger's resolution, about 2,070 years after landing, as DecayHeatBelowLedgerResolution, and before landing as MissionTimeBeforeLanding. The mission clock is a compensated sum: each step adds dt_s to t_s in 64-bit arithmetic, keeps what the addition rounded off and folds it back in, so t_s stays within half a spacing of 64-bit numbers of the exact sum of the start and the steps at any length of run. Every step, the first included, compares its landing with mission day 4,087 plus one tolerance, MISSION_END_TOLERANCE_S, 2^-22 s or about 2.38e-7 s: four spacings of 64-bit numbers between 2^28 s and 2^29 s, where the last day lies, enough for a start and a step read from decimals and for the clock's own rounding. A run or a single step whose end in decimal is exactly mission day 4,087 lands its last step without FLAG_PAST_RECORD, and range_margin_s, the time left to the last day of the record, is zero on and past it. A restart from a returned t_s begins with no carry. electrical_energy_j, heat_rejected_j and fading_electrical_energy_j count from the start of this run, the anchor of the ledger, and not from landing: energy_anchor_j is the decay heat held at that start. A restart from a returned state starts each again at zero. No refusal depends on these totals: every limit of the card is on the mission time, the heat still held or on a single step's ledgers".into(),
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

fn reactor_spert3_card() -> Card {
    let specimen = reactor_spert3::ReactorSpert3::e_core_cold_startup();
    // No frequency on the card, so no `π/ω`.
    let gate = specimen.gate_dt_s().expect("specimen gate");
    let floor = specimen.floor_dt_s().expect("specimen floor");
    let coolant_min = reactor_spert3::WATER_FREEZING_K;
    let coolant_max = reactor_spert3::WATER_CRITICAL_K;
    let tested_min = reactor_spert3::COOLANT_TEMPERATURE_MIN_K;
    let tested_max = reactor_spert3::COOLANT_TEMPERATURE_MAX_K;
    let reactivity_min = reactor_spert3::REACTIVITY_MIN_DOLLAR;
    let reactivity_max = reactor_spert3::REACTIVITY_MAX_DOLLAR;
    finish(
        "energy",
        "reactor_spert3",
        "The SPERT III E-core test reactor, cold, from a step in reactivity to the first maximum of power.",
        vec![
            scalar(
                "coolant_temperature_k",
                Role::Phase,
                "K",
                Some(coolant_min),
                false,
                Some(coolant_max),
                false,
                format!(
                    "inclusive [{}, {}] K, the primary coolant at the start: liquid water, from where water freezes to its critical point, above which there is no liquid at any pressure; a temperature outside it is refused as CoolantTemperatureNotLiquidWater. Tested at a primary coolant of 56 to 92 degF ([{}, {}] K). The card's generation time and delayed groups are the report's values for 70 degF, and its feedback does not depend on the coolant temperature, so it gives the same burst at any coolant temperature; the report's hot-startup tests at 260 and 500 degF are in reactor_spert3_fuel",
                    coolant_min, coolant_max, tested_min, tested_max
                ),
            ),
            scalar(
                "reactivity_inserted_dollar",
                Role::Load,
                "dollar",
                None,
                false,
                None,
                false,
                format!(
                    "the reactivity of the step held from the start of a segment. The first segment's is the step in reactivity at time zero and must be above zero: zero or less is refused as ReactivityNotPositive. A later segment may carry any reactivity, and each step is computed at its own. Compared on reactivity 0.769 to 1.210 dollars ([{}, {}], the inhour values for the card's set of the longest and the shortest tested reactor period, 1010 ms (test 22) and 10 ms (test 43), IDO-17281 Table III, not rounded): on the ten held-back tests, 0.890 to 1.128 dollars, it met five of eight lines; peak power and energy to peak were within the report's standard deviations and the peaks came about 21 percent early. Outside that span the card computes the same equations and nothing was compared",
                    reactivity_min,
                    reactivity_max
                ),
            ),
        ],
        floor,
        "a locked 1e-9 s: the smallest step that still moves the clock at 1e7 s and moves the power at the start by a part that 64-bit numbers resolve",
        gate,
        "0.2 of the time constant of the fastest pole the card reaches inside its compared span, the prompt pole at the smallest compared reactivity with the feedback of 10 MW s, above the 7.2 MW s the card reaches there. Past the compared span the step ceiling follows the state: the card's step refuses as DtAboveStateCeiling, the state kept, a step longer than 0.2 of the time constant of the fastest pole at the state it starts from or would land on, or 0.07 of it when that pole is growing. The poles are the prompt pole |1 - rho| / l*, with rho the net reactivity and l* 2.15 ms, and the feedback pole, the square root of a P / l*, with a 0.0369864 dollar per MW s and P the power. At this ceiling the growing pole binds above a net reactivity of 1.2104 dollars and the decaying pole below 0.3988 dollar, which a burst from below the compared span reaches as its feedback grows: a step of 0.2 l*, 0.43 ms, holds at any net reactivity from zero to one dollar, and one of 0.07 l* / (rho - 1) above one dollar",
        None,
        "A run is one burst from one step in reactivity at time zero: the first segment's reactivity is that step and must be above zero, refused before any step as ReactivityNotPositive otherwise. A later segment may carry another reactivity: each step is computed at the reactivity its segment holds, and the ledger books each step at the reactivity it held. Every tested burst was one step in reactivity, held to the first maximum; a burst whose reactivity changes was not compared. A segment raised is a second insertion; a segment lowered below what the burst needs to keep rising, to zero or below it, turns the power down, and that step lands as the first maximum. NegativeState, NegativeTime and InconsistentState are refusals of the card's step for a state handed to it that the card did not write; this door takes the coolant temperature only and hands no state in, so no request reaches them. coolant_temperature_k is in kelvin. The reactor starts at 5e-5 MW in equilibrium. A single step takes the coolant temperature and starts again from there at each call, so single steps cannot be chained. A run ends at the first maximum of power: the step after it is refused as PastFirstMaximum and the run is sealed with that refusal. Its last landed state carries peak_power_mw, peak_energy_mws and peak_time_s, the highest landed sample, which lies within one step of the true maximum; the card does not interpolate. The latest first maximum the card reaches inside its compared span is at 12.2 s; the longest in Table III is 13.7 s. The largest energy to peak in Table III is 6.9 MW s, and the card reaches 7.2 MW s at the bottom of its compared span. The largest peak power in Table III is 280 MW, and the card reaches about 296 MW at the top of its compared span. Past the span the card has no limit of time, energy or power: at 0.3 dollar the first maximum is at 149 s, at 3 dollars it is 25,200 MW, and at 10 dollars 509,000 MW with 245 MW s released. At the ceiling the slowest compared reactivity takes 17,098 steps to the first maximum, inside the 1,000,000 steps of a run; at 0.3 dollar and 0.43 ms a burst takes about 346,000 steps; a run of 1,000,000 steps at the floor stops at one millisecond, completed, before any maximum. A step of zero or less is refused as NonPositiveDt before the step floor is tested.".into(),
        &[
            "NonFiniteInput",
            "NonFiniteState",
            "NonPositiveDt",
            "InvalidCard",
            "ReactivityNotPositive",
            "CoolantTemperatureNotLiquidWater",
            "PastFirstMaximum",
            "NegativeTime",
            "DtAboveStateCeiling",
            "NegativeState",
            "InconsistentState",
            "SpontaneousGeneration",
            "FirstLawBreach",
        ],
        vec![Advisory {
            name: "FLAG_PROMPT_SUPERCRITICAL",
            mask: reactor_spert3::FLAG_PROMPT_SUPERCRITICAL,
            seals: true,
        }],
        slots(&[
            ("t", "Time since the step in reactivity, in seconds."),
            ("pos[0]", "power_mw"),
            ("pos[1]", "energy_mws"),
            ("pos[2]", "reactivity_net_dollar"),
            ("vel[0]", "peak_power_mw"),
            ("vel[1]", "peak_energy_mws"),
            ("vel[2]", "peak_time_s"),
            ("force_torque", "Rate of change of power, in MW per second."),
            ("residual", "Conservation residual."),
            (
                "flags",
                "FLAG_PROMPT_SUPERCRITICAL: net reactivity above one dollar. It seals.",
            ),
            ("proof", "Not written by the step."),
        ]),
    )
}

/// The result of the Great Britain card's validation record: the one run on
/// the held-back trips, as the research record gives its results.
const GRID_GB_RESULT: &str = "Compared with the held-back trips on 2026-10-09, run once through the engine: all seven lines were met on 12 trips. One of the 13 was dropped before scoring by the quality rule written beforehand, the trip of 2 September 2021, whose recorded frequency moves 0.275 Hz in the ten seconds before the fall. That was two opposite losses at once, an export link and an import link, and not one trip. Depth of the fall: root mean square relative error 0.166 and mean -0.018, against 0.182 for the rule that depth is proportional to loss on the same trips; the worst was 37 percent too shallow, on 9 May 2019. Steepest second: 0.103 and mean +0.043. Frequency near 60 s: 0.046 Hz. The whole trace: median 0.036 Hz. No trip was refused by the card. The held-back twelve were the easiest third of three: on the other two thirds, each with constants refitted on the rest, the depth error is 0.258 and 0.326 with three lines missed each, and 18.7 percent of random twelves of the forty meet all seven lines. The card's margin over one-constant rules on the twelve is 0.015: depth proportional to loss 0.182, a fall to one fixed frequency 0.180, a straight line 0.171, the card 0.166. The comparison is not blind: the loss and the lowest frequency of every trip, the held-back ones included, were on screen once, in the scan that found the trips, before the protocol was fingerprinted and the trips were split, and the depth line scores that lowest frequency; no held-back number entered a fit or a choice. A further 39 trips of the same kind, which the scan's rule had dropped because it reads the fall at the first second to reach 0.12 Hz, were found by the second scorer; 36 were inside the card as served through engine 0.6.0 and were scored after the fact, and six of them, of 500 to 692 MW, are under the losses the card takes from engine 0.7.0. All 36: depth 0.269 and near 60 s 0.067 Hz, two lines missed. May 2018 to February 2025, 23 trips: depth 0.136, all lines met. 2024 to 2026, 15 trips: depth 0.345, six lines missed. The six trips since November 2025 run 39 to 80 percent too deep. The binary that made the scored runs (SHA-256 da8ee131279452aa4f1f56afb71e801f622a050882554fb6d632dd23f9b7384c) was overwritten by a later build, which returned all 41 saved answers byte for byte. Since engine 0.7.0 the final state no longer carries below_statutory_limit, so the saved answers no longer come back byte for byte. A second scoring of 2026-10-09, on the same machine and from the scorer's own scan of the monthly files, found the same numbers: evidence/verification/second_party/V-32/.";

fn grid_gb_card() -> Card {
    let specimen = grid_gb::GridGb::interconnector_loss();
    // The ceiling is a fiftieth of the fastest time scale of the card, so no `π/ω`.
    let gate = specimen.gate_dt_s().expect("specimen gate");
    let floor = specimen.floor_dt_s().expect("specimen floor");
    finish(
        "energy",
        "grid_gb",
        "The Great Britain synchronous area, the 50 Hz transmission system of England, Scotland and Wales as one rotating mass, after an importing interconnector trips. Compared on trips of 693 to 1402 MW over their first 60 s: on the 12 held-back trips it met all seven lines; scored after the fact, on six trips of 500 to 692 MW it missed three, and on the six compared trips since November 2025 its fall is 39 to 80 percent deeper than the measured fall. Outside the compared trips a step lands with a flag that says so.",
        vec![
            scalar(
                "frequency_hz",
                Role::Phase,
                "Hz",
                Some(grid_gb::FREQUENCY_MIN_HZ),
                true,
                Some(grid_gb::FREQUENCY_MAX_HZ),
                false,
                format!(
                    "above {} Hz and at most {} Hz, the system frequency at the start, with the area in balance: at or below 48.8 Hz low-frequency demand disconnection has begun (LowFrequencyDemandDisconnection), and above 50.5 Hz, the statutory limit, the card holds no over-frequency response (FrequencyAboveCard). Compared on starts of 49.853 to 50.126 Hz, the opened trips, rounded outward to {} to {} Hz; a start outside sets FLAG_OUTSIDE_COMPARED_TRIPS",
                    grid_gb::FREQUENCY_MIN_HZ,
                    grid_gb::FREQUENCY_MAX_HZ,
                    grid_gb::START_FREQUENCY_MIN_HZ,
                    grid_gb::START_FREQUENCY_MAX_HZ
                ),
            ),
            scalar(
                "inertia_mw_s",
                Role::Phase,
                "MW*s",
                Some(0.0),
                true,
                None,
                false,
                format!(
                    "above 0 MW s, the kinetic energy the area stores at 50 Hz: the operator's published outturn inertia for the half hour, in GVA s, times 1000. The rate of change of frequency divides by it (InertiaNotPositive). Compared on {} to {} MW s, the span of the opened trips, 111 to 337 GVA s; outside it sets FLAG_OUTSIDE_COMPARED_TRIPS, and the run's step ceiling is recomputed from its own inertia and demand",
                    grid_gb::INERTIA_MIN_MW_S,
                    grid_gb::INERTIA_MAX_MW_S
                ),
            ),
            scalar(
                "demand_mw",
                Role::Phase,
                "MW",
                Some(0.0),
                false,
                None,
                false,
                format!(
                    "at least 0 MW (DemandNegative), generation and net imports on the transmission system just before the loss. Compared on {} to {} MW, the span of the opened trips; outside it sets FLAG_OUTSIDE_COMPARED_TRIPS, and the run's step ceiling is recomputed from its own inertia and demand",
                    grid_gb::DEMAND_MIN_MW,
                    grid_gb::DEMAND_MAX_MW
                ),
            ),
            scalar(
                "loss_mw",
                Role::Load,
                "MW",
                None,
                false,
                None,
                false,
                format!(
                    "the power lost, held through each step: a loss of infeed when positive, a loss of demand when negative, never larger in size than demand_mw (LossLargerThanDemand). Compared on {} to {} MW, the smallest of all 40 scored trips and the largest of the opened trips, each one loss held from time zero. Six trips of 500 to 692 MW were run through the engine on 2026-10-10 by the protocol's lines and missed three of the seven (depth rms 0.2512 against 0.25, steepest second rms 0.273 against 0.25). No trip under 500 MW, above 1402 MW or with a loss of demand was compared. A loss outside the compared span, or one that is not the run's first, sets FLAG_OUTSIDE_COMPARED_TRIPS",
                    grid_gb::LOSS_MIN_MW,
                    grid_gb::LOSS_MAX_MW
                ),
            ),
        ],
        floor,
        "a locked 1e-6 s: a step of that size still moves the clock at 60 s, and a loss of 1 MW on the largest compared inertia moves the frequency by 500 times the spacing of 64-bit numbers at 50",
        gate,
        "0.02 of the fastest time scale the card can reach, the swing of the rotating mass against the response, the relief pole of the demand or the response lag, at 48.8 Hz, under the lowest frequency of the card. The value here is at the corner of the compared spans, 111,000 MW s and 40,829 MW, and holds for every run inside them. A run outside them takes the same rule at its own inertia and demand where that gives a smaller ceiling, as a smaller inertia or a larger demand does, and a step above it is refused as DtAboveCeiling with the ceiling stated",
        None,
        "frequency_hz, inertia_mw_s and demand_mw are the area's condition at the start and are held through the run; loss_mw is the load of each segment. The ledger books each step's own loss, so a second loss, a link that comes back and a quiet period before the trip are stepped by the same equations; every compared trip was one loss held from time zero, and the one measured trip that was followed by a second loss, 22 December 2023, is the card's worst, so a step whose loss is not the run's first sets FLAG_OUTSIDE_COMPARED_TRIPS. A run starts at time zero with the area in balance at frequency_hz and no response delivered. A single step starts in the same way at each call, so single steps cannot be chained: a run is the way to follow the fall. The card was compared over the first 60 s after each trip: the frequency near 60 s was within 0.046 Hz on the held-back trips and 0.097 Hz on the opened trips. A run goes on past 60 s until its steps run out or a limit stops it, and a step that lands later than 60 s sets FLAG_PAST_COMPARED_MINUTE; past the first minute the card holds the primary response only, and the operator's instructed actions and the slower reserves are not in it. The clock is a running sum, t_s += dt_s in 64-bit arithmetic, and each addition rounds by up to 3.55e-15 s, half the spacing of 64-bit numbers between 32 s and 64 s. The comparison with 60 s therefore carries a named allowance, TIME_END_ALLOWANCE_PER_STEP_S: 3.55e-15 s for each step a run of this step takes to reach 60 s, and two more for the step read from a decimal, and never more than half the step. A run of exactly 60 s in decimal lands its last step without the flag. 49.55 Hz is the lowest frequency compared with measurement: the lowest measured single-loss frequency among the 41 trips, 49.553 Hz, rounded outward. The lowest among the 36 trips the scan lost is 49.596 Hz, and the lowest the card reached in any compared run is 49.563 Hz. A landing under 49.55 Hz sets FLAG_BELOW_COMPARED_FREQUENCY; below the 49.8 Hz knee the response the grid held changed over the years. Through engine 0.9.0 the edges of the compared trips were refusals. A frequency at or below 48.8 Hz, where low-frequency demand disconnection begins, is refused as LowFrequencyDemandDisconnection, and one above 50.5 Hz, the statutory limit, as FrequencyAboveCard: the card holds no over-frequency response, and only a loss of demand reaches it. The state carries lowest_frequency_hz and lowest_time_s, the lowest landed sample; the card does not interpolate. The measured frequency is one value a second, and a loss falls between two samples: the protocol reads the card at half a second, one and a half seconds and so on after the loss begins. The response is one fitted function of the frequency for nine years of days: the response the operator held on a given day is not an input. A step of zero or less is refused as NonPositiveDt".to_string(),
        &[
            "NonFiniteInput",
            "NonFiniteState",
            "NonPositiveDt",
            "InvalidCard",
            "InertiaNotPositive",
            "DemandNegative",
            "LossLargerThanDemand",
            "NegativeTime",
            "LowFrequencyDemandDisconnection",
            "FrequencyAboveCard",
            "SpontaneousGeneration",
            "FirstLawBreach",
        ],
        vec![
            Advisory {
                name: "FLAG_OUTSIDE_COMPARED_TRIPS",
                mask: grid_gb::FLAG_OUTSIDE_COMPARED_TRIPS,
                seals: true,
            },
            Advisory {
                name: "FLAG_PAST_COMPARED_MINUTE",
                mask: grid_gb::FLAG_PAST_COMPARED_MINUTE,
                seals: true,
            },
            Advisory {
                name: "FLAG_BELOW_COMPARED_FREQUENCY",
                mask: grid_gb::FLAG_BELOW_COMPARED_FREQUENCY,
                seals: true,
            },
        ],
        slots(&[
            ("t", "Time since the start, in seconds."),
            ("pos[0]", "frequency_hz"),
            ("pos[1]", "response_mw"),
            ("pos[2]", "relief_mw"),
            ("vel[0]", "frequency_rate_hz_per_s"),
            ("vel[1]", "lowest_frequency_hz"),
            ("vel[2]", "lowest_time_s"),
            (
                "force_torque",
                "Net power on the rotating mass, in MW: response plus relief less the loss.",
            ),
            ("residual", "Conservation residual."),
            (
                "flags",
                "FLAG_OUTSIDE_COMPARED_TRIPS when the start, the inertia, the demand or the step's loss is outside the compared trips or the loss is not the run's first; FLAG_PAST_COMPARED_MINUTE when the step lands later than 60 s; FLAG_BELOW_COMPARED_FREQUENCY when it lands under 49.55 Hz. Each seals.",
            ),
            ("proof", "Not written by the step."),
        ]),
    )
}

fn cell_2170_m80_card() -> Card {
    let specimen = cell_2170_m80::Cell2170M80::cell();
    // No pole on the card, so no `π/ω`. The floor and the ceiling are the
    // same one second.
    let gate = specimen.gate_dt_s().expect("specimen gate");
    let floor = specimen.floor_dt_s().expect("specimen floor");
    // Every block is served; the two that met no mark carry a flag.
    let served: Vec<usize> = (0..cell_2170_m80::BLOCK_COUNT).collect();
    let least_top = served
        .iter()
        .map(|block| specimen.charge_top_ah[*block])
        .fold(f64::INFINITY, f64::min);
    let most_deep = served
        .iter()
        .map(|block| specimen.charge_deep_ah[*block])
        .fold(f64::NEG_INFINITY, f64::max);
    let coldest_block = served
        .iter()
        .map(|block| specimen.block_chamber_c[*block])
        .fold(f64::INFINITY, f64::min);
    let warmest_block = served
        .iter()
        .map(|block| specimen.block_chamber_c[*block])
        .fold(f64::NEG_INFINITY, f64::max);
    let ranges = served
        .iter()
        .map(|block| {
            format!(
                "{} degC: [{}, {}] Ah",
                token(specimen.block_chamber_c[*block]),
                token(specimen.charge_top_ah[*block]),
                token(specimen.charge_deep_ah[*block])
            )
        })
        .collect::<Vec<_>>()
        .join("; ");
    let bands = served
        .iter()
        .map(|block| {
            format!(
                "{} degC block: [{}, {}] degC",
                specimen.block_chamber_c[*block],
                specimen.block_case_min_c[*block],
                specimen.block_case_max_c[*block]
            )
        })
        .collect::<Vec<_>>()
        .join("; ");
    let currents = served
        .iter()
        .map(|block| {
            format!(
                "{} degC block: [{}, {}] A",
                specimen.block_chamber_c[*block],
                -specimen.block_current_discharge_max_a[*block],
                specimen.block_current_charge_max_a[*block]
            )
        })
        .collect::<Vec<_>>()
        .join("; ");
    finish(
        "energy",
        "cell_2170_m80",
        "One Tesla Model 3 2170 lithium-ion cell, cell m80 alone, in six blocks of tests from -20 to 40 degC, under a terminal current and a measured case temperature. Compared on 60 held-back tests it failed, 27 met: C/3 met from 0 to 40 degC, 1C missed at each, and the -10 and -20 degC blocks met no mark (55.6 to 425 mV). The physical model cell_2170_model3 reads 17.4 mV lower on one 1.5 A discharge at 12 degC.",
        vec![
            scalar(
                "block_chamber_c",
                Role::Phase,
                "degC",
                Some(coldest_block),
                false,
                Some(warmest_block),
                false,
                "exactly one of 40, 25, 10, 0, -10 and -20 degC: the chamber setting that names the block of tests the run belongs to. It picks the capacity, the rest-voltage table and the range of charge, and in this dataset it is also the cell's age. In the 25, 10, 0, -10 and -20 degC blocks the capacity is the charge of a capacity test that began after a top-up, 33, 22, 48, 52 and 39 mAh above the laboratory's full (0.73, 0.52, 1.11, 1.23 and 0.92 percent of the block's capacity); in the 40 degC block that test began at full. In those five blocks charge_level is therefore not the state of charge counted from the laboratory's full: it is 1 at full and reads up to that share high toward empty. The tables were fitted with these capacities, and the comparison is of the card as fitted. In the -10 and -20 degC blocks no held-back test met its marks: at -10 degC C/3 read 55.6 mV rms and +0.68 percent capacity, 1C 134.2 mV and +6.92 percent, no drive cycle passed; at -20 degC C/3 139.2 mV and +7.09 percent, 1C 207.8 mV and +15.85 percent, no drive cycle passed, reading far too high from the first seconds. Through engine 0.9.0 they were refused as BlockShownWrong; now every step in them carries FLAG_BLOCK_MET_NO_MARK. Any other value is refused as BlockNotRecognized; nothing is rounded to a block".into(),
            ),
            scalar(
                "charge_removed_ah",
                Role::Phase,
                "Ah",
                Some(least_top),
                false,
                Some(most_deep),
                false,
                format!(
                    "ampere-hours removed since the recharge: 0 for a cell that has just been recharged. Each block has its own inclusive range, where its tables end: {ranges}. The published minimum and maximum are the widest of the six; a start outside the block's own range is refused as ChargeOutsideTables. A start one 64-bit number past an end is refused too: no start is read at an end point it is not at. A run that landed its last step past an end, within the rounding allowance of its running sum, returns a charge from which no new run starts, and a new run starts from the end itself"
                ),
            ),
            scalar(
                "current_a",
                Role::Load,
                "A",
                Some(-cell_2170_m80::CURRENT_DISCHARGE_MAX_A),
                false,
                Some(cell_2170_m80::CURRENT_CHARGE_MAX_A),
                false,
                format!(
                    "inclusive [-13.5, 13.5] A, the top of the pulse test and the end of the ten-second table's current axis; past it the table would hold its end value (CurrentOutsideCard). A negative current discharges the cell and a positive current charges it, as in the measurements; a charging current above 2.25 A is read on the discharge side of the tables, which were fitted on pulses to 13.5 A discharging and 2.25 A charging. Compared up to 11.374 A discharging and 9.737 A charging, for seconds in drive cycles; held currents compared at about 1.5 A (met from 0 to 40 degC) and 4.5 A (missed at every temperature, 44.1 to 88.2 mV). Each block's own held-back tests ran from the largest discharging to the largest charging sample, rounded outward to the milliampere: {currents}. A current outside its block's tests sets FLAG_OUTSIDE_COMPARED_TESTS; through engine 0.9.0 it was refused as CurrentOutsideBlock"
                ),
            ),
            scalar(
                "case_temperature_c",
                Role::Load,
                "degC",
                Some(cell_2170_m80::CASE_TEMPERATURE_MIN_C),
                false,
                Some(cell_2170_m80::CASE_TEMPERATURE_MAX_C),
                false,
                format!(
                    "inclusive [-21.898, 45.535] degC: the temperature measured on the cell's case, at the middle. The card is given it and does not predict it. The bounds are the coldest and the warmest case temperature of any block's own tests; past them the resistances and the time constant would hold the end block's value, and the card has no temperature law of its own (CaseTemperatureOutsideCard). Each block's own held-back tests were run at, from the coldest to the warmest sample, rounded outward to the millikelvin: {bands}. A case temperature outside its block's band sets FLAG_OUTSIDE_COMPARED_TESTS; through engine 0.9.0 it was refused as CaseTemperatureOutsideBlock. From 0 to 40 degC 27 of 40 runs met their marks, and in the -10 and -20 degC blocks none of 20"
                ),
            ),
        ],
        floor,
        "a locked 1 s: the step is exactly one second, the sampling of the measurements the card was fitted to",
        gate,
        "the same locked 1 s: the first-second resistance has no meaning at another step, and the card refuses any other as StepNotOneSecond",
        None,
        "Over a step the current and the case temperature are held, and every table is read at the charge the step lands on. A current or a case temperature outside its range in any segment of a run refuses the whole run before its first step. The range compared with measurement is each block at the case temperatures and the currents of its own held-back tests, and a step outside it lands with FLAG_OUTSIDE_COMPARED_TESTS. The same cell type is served as a physical model with memory, cell_2170_model3; on one discharge, cell m80 from the recharge of its 10 degC block at 1.5 A for 600 s with the case at 12 degC, this card gives 3.934 V and that one 3.917 V, 17.4 mV apart. The current is bounded in size only: the card does not track how long a current is held. The held currents compared were a third of the rated current, about 1.5 A, and the rated current, about 4.5 A; the drive cycles reach the larger currents only for seconds at a time, and the held discharge at the rated current missed its voltage mark at every temperature. A run starts at rest: charge_removed_ah as given, nothing held by the relaxing branch, and the ledger anchored there. The first measured sample of a record is that start and each later sample is one step. A single step starts from rest in the same way at each call, so single steps cannot be chained into a run. A run ends in a sealed refusal when the terminal voltage would fall below 2.8 V (LowerVoltageLimit), would rise above 4.242 V (UpperVoltageLimit), or when the charge removed would leave the tables of the block (ChargeOutsideTables); the last landed state is the last second on the card. The charge removed is a running sum in 64-bit arithmetic, so the comparison with either end of the tables carries a rounding allowance: 2^-50 Ah for each landed step and the step being taken, and two more for the start and the end of the table, never more than half the charge of the step; a rest, or a step back toward the tables, from a state past an end lands. A run whose end in decimal is exactly an end of the tables, at the top or the bottom, lands its last step, and every table is read at the end point there. When the charge leaves the tables on the same step on which the voltage would pass a limit, the refusal named is ChargeOutsideTables, because no voltage is read outside the tables. A discharge slower than about a tenth of the capacity per hour can leave the tables before 2.8 V. A step of zero or less is refused as NonPositiveDt before the step floor is tested.".into(),
        &[
            "NonFiniteInput",
            "NonFiniteState",
            "NonPositiveDt",
            "InvalidCell",
            "StepNotOneSecond",
            "BlockNotRecognized",
            "CaseTemperatureOutsideCard",
            "CurrentOutsideCard",
            "ChargeOutsideTables",
            "LowerVoltageLimit",
            "UpperVoltageLimit",
            "SpontaneousGeneration",
            "FirstLawBreach",
        ],
        vec![
            Advisory {
                name: "FLAG_BLOCK_MET_NO_MARK",
                mask: cell_2170_m80::FLAG_BLOCK_MET_NO_MARK,
                seals: true,
            },
            Advisory {
                name: "FLAG_OUTSIDE_COMPARED_TESTS",
                mask: cell_2170_m80::FLAG_OUTSIDE_COMPARED_TESTS,
                seals: true,
            },
        ],
        slots(&[
            ("t", "Time since the anchor, in seconds."),
            ("pos[0]", "terminal_v"),
            ("pos[1]", "charge_removed_ah"),
            ("pos[2]", "current_a"),
            ("vel[0]", "open_circuit_v"),
            ("vel[1]", "branch_v"),
            ("vel[2]", "case_temperature_c"),
            ("force_torque", "Margin to the lower voltage limit, in volts."),
            ("residual", "Conservation residual."),
            (
                "flags",
                "FLAG_BLOCK_MET_NO_MARK in the -10 and -20 degC blocks; FLAG_OUTSIDE_COMPARED_TESTS when the case temperature or the current is outside its block's own tests. Both seal.",
            ),
            ("proof", "Not written by the step."),
        ]),
    )
}

/// The three inlet-temperature bands of the fuel-temperature reactor, as the
/// report names them, in degrees Fahrenheit.
pub(crate) const REACTOR_FUEL_BAND_DEGF: [&str; 3] = ["56 to 92", "251 to 259", "498 to 501"];

/// The smallest and the largest of three numbers.
fn span_of(values: &[f64; 3]) -> (f64, f64) {
    (
        values.iter().copied().fold(f64::INFINITY, f64::min),
        values.iter().copied().fold(f64::NEG_INFINITY, f64::max),
    )
}

fn reactor_spert3_fuel_card() -> Card {
    let specimen = reactor_spert3_fuel::ReactorSpert3Fuel::e_core_low_initial_power();
    // No frequency on the card, so no `π/ω`.
    let gate = specimen.gate_dt_s().expect("specimen gate");
    let floor = specimen.floor_dt_s().expect("specimen floor");
    let inlet_min = specimen.band_inlet_min_k;
    let inlet_max = specimen.band_inlet_max_k;
    let reactivity_min = specimen.band_reactivity_min_dollar;
    let reactivity_max = specimen.band_reactivity_max_dollar;
    let flow_min = specimen.band_flow_min_m_per_s;
    let flow_max = specimen.band_flow_max_m_per_s;
    let (inlet_least, _) = span_of(&inlet_min);
    let (_, inlet_most) = span_of(&inlet_max);
    let per_band = |low: &[f64; 3], high: &[f64; 3], unit: &str| {
        (0..3)
            .map(|band| {
                // Written as the shortest decimal that reads back to the same bits.
                let span = if low[band] == high[band] {
                    format!("exactly {} {unit}", low[band])
                } else {
                    format!("[{}, {}] {unit}", low[band], high[band])
                };
                format!("{} degF: {span}", REACTOR_FUEL_BAND_DEGF[band])
            })
            .collect::<Vec<_>>()
            .join("; ")
    };
    let inlet_bands = per_band(&inlet_min, &inlet_max, "K");
    let reactivity_bands = per_band(&reactivity_min, &reactivity_max, "dollars");
    let flow_bands = per_band(&flow_min, &flow_max, "m/s");
    finish(
        "energy",
        "reactor_spert3_fuel",
        "The SPERT III E-core test reactor in low-initial-power bursts at three inlet temperatures, from the start of the transient rod's motion to the first maximum of power, with the fuel temperature and the water temperature as state; four constants are fitted, and by the rule fixed beforehand the comparison with its 30 held-back tests failed.",
        vec![
            scalar(
                "inlet_temperature_k",
                Role::Phase,
                "K",
                Some(inlet_least),
                false,
                Some(inlet_most),
                false,
                format!(
                    "the coolant at the core inlet at the start, in one of three inclusive bands, the three the report tested: {inlet_bands}. The published minimum and maximum are the lowest and the highest of the three; a temperature between two bands is refused as InletTemperatureOutsideCard, and nothing is rounded to a band: 260 degF itself, 399.8167 K, the report's round name for the middle series, is above that band and is refused. The band picks the generation time, the insertion ramp, the moderator coefficient, the heat capacity of the water, and the spans of the reactivity and of the flow"
                ),
            ),
            scalar(
                "reactivity_inserted_dollar",
                Role::Load,
                "dollar",
                Some(0.0),
                true,
                None,
                false,
                format!(
                    "the reactivity the transient rod inserts in all, ramped in from zero as K t^2 and then held; above zero, and zero or less is refused as ReactivityNotPositive. Compared per band on the tested periods: {reactivity_bands} (0.769 to 1.210 dollars at 56 to 92 degF, 0.919 to 1.207 at 251 to 259 degF, 0.837 to 1.227 at 498 to 501 degF); on the 30 held-back tests peak power met its lines in every band and the time of the peak was missed in the two cooler bands: root mean square 5.22 percent against 5 and worst 12.24 percent against 10 at 56 to 92 degF, 8.46 and 14.88 percent at 251 to 259 degF. Outside a band's span the card computes the same ramp and feedback at any reactivity above zero, and nothing was compared. A run is one burst and carries one reactivity: a segment with another is refused as ReactivityChangedDuringBurst"
                ),
            ),
            scalar(
                "coolant_flow_m_per_s",
                Role::Load,
                "m/s",
                Some(0.0),
                false,
                None,
                false,
                format!(
                    "the average coolant flow rate through the core, zero or more in any band, the cold band included; below zero is refused as NegativeFlow. Tested per band: {flow_bands}. The cold band was tested with no flow, 251 to 259 degF at 2.4 to 22 ft/s and 498 to 501 degF at 4.8 to 24 ft/s; the flow enters only through the sweep of the water temperature. Outside those flows the card computes the same equations and nothing was compared. Each segment may carry its own flow rate, booked per step in the ledger at that step's flow; no tested burst changed its flow, and a burst whose flow changes was not compared"
                ),
            ),
        ],
        floor,
        "a locked 1e-9 s: the card's own constant, a step that still moves the clock at 1e7 s",
        gate,
        "0.2 of the reduced generation time of the cold band, the time constant of the prompt pole at zero net reactivity, where every run starts; the same ceiling in all three bands. Past the compared spans the step ceiling follows the state: the card's step refuses as DtAboveStateCeiling, the state kept, a step longer than 0.2 of the time constant of the fastest pole at the state it starts from or would land on, or 0.045 of it when that pole is growing. The poles are the prompt pole |1 - rho| / l*, with rho the net reactivity and l* the band's, 2.15, 2.18 or 2.25 ms; the feedback pole of the fuel and the water temperature; and the sweep of the water, 2 v / L, with L 0.9728 m. At this ceiling the growing pole binds above a net reactivity of about 1.22 dollars, and the sweep above a flow of about 226 m/s: a step of at most 0.045 l* / (rho - 1) holds through the ramp to rho dollars, 97 microseconds at 2 dollars in the cold band",
        None,
        format!(
            "inlet_temperature_k picks the band. A run is one burst, with one reactivity: every segment carries the reactivity of the first, and a segment whose reactivity is not the first segment's refuses the whole run before its first step as ReactivityChangedDuringBurst, with nothing sealed. The card is one insertion, ramped in and then held, which is what the report tested: a second insertion, a withdrawal or a jump in the inserted reactivity is not on the card. The card's own step refuses the same change once the first step has landed, and keeps the state. Each segment may carry its own coolant flow rate. inlet_temperature_k is in kelvin. The reactor starts at {} MW in equilibrium, with zero net reactivity and both temperature rises zero, at the moment the transient rod begins to move. The reactivity is ramped in as K t^2 up to the load and held there, with K the band's own; FLAG_ROD_MOVING is set while the inserted reactivity is still below the load. fuel_temperature_rise_k and water_temperature_rise_k are rises above the inlet temperature. A single step takes the inlet temperature and starts again from there at each call, so single steps cannot be chained. A run ends at the first maximum of power after the insertion is complete: the step after it is refused as PastFirstMaximum and the run is sealed with that refusal. Its last landed state carries peak_power_mw, peak_energy_mws and peak_time_s, the highest landed sample, which lies within one step of the true maximum; the card does not interpolate. The longest scored time of peak is 13.7 s; the largest energy to peak in Tables III and V is 15 MW s; the largest peak power is 410 MW. Past them the card computes the same burst to its first maximum, with no limit of time, energy or power: the fuel lump is about a fourteenth of the fuel and has no melting limit on the card. At 0.3 dollar in the cold band the first maximum is at 177 s; at 4 dollars it is 57,900 MW with 157 MW s released and a fuel_temperature_rise_k of 13,300 K. A run of 1,000,000 steps at the floor stops at one millisecond, completed, with the rod still moving. NegativeState, NegativeTime and InconsistentState are refusals of the card's step for a state handed to it that the card did not write; this door takes the inlet temperature and nothing else of a state, and starts every run and every step from the card's own start, so no request reaches them. A step of zero or less is refused as NonPositiveDt before the step floor is tested.",
            format!("{:e}", specimen.initial_power_mw),
        ),
        &[
            "NonFiniteInput",
            "NonFiniteState",
            "NonPositiveDt",
            "InvalidCard",
            "InletTemperatureOutsideCard",
            "ReactivityNotPositive",
            "NegativeFlow",
            "PastFirstMaximum",
            "NegativeTime",
            "DtAboveStateCeiling",
            "NegativeState",
            "ReactivityChangedDuringBurst",
            "InconsistentState",
            "SpontaneousGeneration",
            "FirstLawBreach",
        ],
        vec![
            Advisory {
                name: "FLAG_PROMPT_SUPERCRITICAL",
                mask: reactor_spert3_fuel::FLAG_PROMPT_SUPERCRITICAL,
                seals: true,
            },
            Advisory {
                name: "FLAG_ROD_MOVING",
                mask: reactor_spert3_fuel::FLAG_ROD_MOVING,
                seals: true,
            },
        ],
        slots(&[
            ("t", "Time since the transient rod began to move, in seconds."),
            ("pos[0]", "power_mw"),
            ("pos[1]", "energy_mws"),
            ("pos[2]", "reactivity_net_dollar"),
            ("vel[0]", "fuel_temperature_rise_k"),
            ("vel[1]", "water_temperature_rise_k"),
            ("vel[2]", "peak_power_mw"),
            ("force_torque", "Rate of change of power, in MW per second."),
            ("residual", "Conservation residual."),
            (
                "flags",
                "FLAG_PROMPT_SUPERCRITICAL: net reactivity above one dollar. FLAG_ROD_MOVING: the inserted reactivity is still below the load. Both seal.",
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
                "inclusive [0, 0] degrees: the twist of the plate against the finger at the start, which is zero, and every run and every single step starts there: the card's closed forms are of a pad twisted from zero under the load it holds, and a pad already twisted holds springs set by the load and the path it was brought there under, which a start does not carry, so any other start is refused as TwistNotStartedFromZero, a twist such as 1e-46 that would read as zero in a 32-bit frame slot included. The plate turned 80 degrees in every trial and slowed to a stop between 77 and 80 degrees; the torque and the stuck fraction were compared with measurement from 1 to 75 degrees, and past 75 degrees a run goes on, each frame carrying FLAG_BEYOND_COMPARED; through engine 0.9 a step past 75 degrees was refused".into(),
            ),
            scalar(
                "normal_force_n",
                Role::Load,
                "N",
                Some(0.0),
                true,
                None,
                false,
                "above 0 N: the force of the plate on the pad. A force of zero or less is no contact and is refused as NoContact. Measured on this finger at 0.5, 1, 2, 5 and 10 N; the card's slip torque was off by +13.6 percent at 0.5 N and +21.7 percent at 5 N on trials it was fitted to, by +9.4 percent at 1 N and +29.3 percent at 10 N on trials held back, against a mark of 5.62 percent, and within 0.7 to 7.4 percent at 2 N. Below 0.5 N or above 10 N, where nothing was measured, a frame carries FLAG_BEYOND_COMPARED".into(),
            ),
            scalar(
                "twist_rate_deg_s",
                Role::Load,
                "deg/s",
                Some(0.0),
                true,
                None,
                false,
                "above 0 degrees a second: the rate at which the plate turns about the normal. A rate of zero or less is refused as TwistRateNotPositive. The constants were fitted at 5, 10, 20 and 50 degrees a second at 2 N, where the slip torque is off by +0.7, -6.8, -5.0 and -4.2 percent, and compared at 100, held back, where it was off by -13.0 percent. Below 5 or above 100 degrees a second, where nothing was measured, a frame carries FLAG_BEYOND_COMPARED; the characteristic twists go as the rate to the powers -0.341 and -0.015 and grow without limit as the rate falls. The direction of the turn is not an input: clockwise and counter-clockwise trials were pooled".into(),
            ),
        ],
        floor,
        "the ceiling divided by 2^8, a twenty-fifth of the sampling interval of the force sensors",
        gate,
        "0.01 s, one camera frame at the fastest twist measured, 100 degrees a second. The card holds the four-point rule that sums the work and the heat to its own step: a step that turns the plate by more than 0.05 of the characteristic twist of the torque is refused as TwistStepAboveQuadrature. At 2 N that is 0.73 degrees at 20 degrees a second and 0.42 at 100, so a step of 0.01 s holds to 52.6 degrees a second and a faster twist takes a shorter step",
        None,
        format!(
            "Over a step the normal force and the twist rate are held, and the twist grows by rate x dt_s. A bed of springs under a parabolic friction limit, twisted about its centre: with c = 2 / (u + sqrt(u^2 + 4)), the torque is T_limit x (1 - (5c^3 + 3c^5)/8) at u = twist / twist_T, and the stuck fraction is c^2 at u = twist / twist_S. T_limit = {} N m x (F / 2 N)^{} x (rate / 20)^{}; twist_T = {} deg x (F / 2 N)^{} x (rate / 20)^{}; twist_S = {} deg x (rate / 20)^{}. All eight numbers are fitted to this finger's own trials; the level, {} N m, is that of the block of trials at 2 N. The card answers at every force and every rate above zero, with the misses at the other forces and at 100 degrees a second stated in its validation record, and a frame beyond 0.5 to 10 N, 5 to 100 degrees a second or 75 degrees of twist carries FLAG_BEYOND_COMPARED. A run is one twist from zero under one load: it starts at time zero with the pad untwisted and the work and the heat at zero, and a start at any other twist is refused as TwistNotStartedFromZero. A single step is the first step of such a twist, from zero, at each call: single steps cannot be chained, and a twist is followed with run_trajectory. The force and the rate cannot change during a twist: under a changed load the bed's springs hold a state set by the path, which the card's state does not carry, and a later segment of a run whose load differs from the first is refused as LoadChangedDuringTwist. Every measured twist began untwisted and held its load. Because no run and no step can start part way, no request can take up a twist under another load. The card's own step refuses the same change for as long as the state lives: the state carries the force and the rate it was twisted under, held_normal_force_n and held_twist_rate_deg_s, which are answers and not inputs. A value outside its range in any segment of a run refuses the whole run before its first step. The frame flag FLAG_FULL_SLIP is an event, the one the card exists to find, and not a warning. A run goes on past 75 degrees, the end of the twist compared with measurement, with the torque rising toward its slip limit and the heat growing with the twist; a frame past 75 degrees by more than {} degrees, the most the sum of the steps can round by, carries FLAG_BEYOND_COMPARED. A step that turns the plate by more than 0.05 of the characteristic twist of the torque is refused as TwistStepAboveQuadrature. The ledger is held to 1e-9 of the larger of 0.0508 J and the plate work of the run.",
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

fn tokamak_jet_dte1_card() -> Card {
    let tokamak = tokamak_jet_dte1::TokamakJetDte1::elmy_h_mode();
    // The ceiling and the floor are locked constants of the card; it has no
    // frequency, so no `π/ω` is published. Past the compared envelope the
    // card's own step holds dt to the ceiling of the state.
    let gate = tokamak.gate_dt_s().expect("specimen gate");
    let floor = tokamak.floor_dt_s().expect("specimen floor");
    finish(
        "energy",
        "tokamak_jet_dte1",
        TOKAMAK_SUMMARY,
        vec![
            scalar(
                "thermal_energy_j",
                Role::Phase,
                "J",
                Some(0.0),
                true,
                None,
                false,
                "above 0 J: the thermal energy of the plasma at the start; zero or less is refused as EnergyOutsideCard. Compared with thermal energies of 0.462 to 9.57 MJ and fast-ion energies of 5.59 kJ to 1.62 MJ; on 42982 the thermal energy error was 7.9 percent and the fast-ion 22.5 percent of peak. Compared at plasma temperatures of 0.777 to 4.30 keV (42976 reached 6.3 keV), at normalised beta 0.224 to 2.21 and at confinement times of 0.150 to 5.72 s (42976 reached 8.5 s). Outside those spans the card computes the same equations, and nothing was compared there. Under the first load the start, and every step, is held to the limits of physics: a temperature from 0.2 keV with at most 100 keV at the centre of the profile, the range of the reactivity fit (TemperatureBelowReactivityFit, TemperatureAboveReactivityFit), and a normalised beta of the two stores of at most 2.8, the Troyon limit (BetaAboveTroyon)".into(),
            ),
            scalar(
                "fast_ion_energy_j",
                Role::Phase,
                "J",
                Some(0.0),
                false,
                None,
                false,
                "at least 0 J: the energy of the beam ions, the ion-cyclotron tail and the fusion alphas at the start; below zero is refused as EnergyOutsideCard. Compared at 5.59 kJ to 1.62 MJ (42976 reached 3.5 MJ); outside it the card computes the same equations. With the thermal energy it is held to the Troyon limit of the thermal_energy_j field".into(),
            ),
            scalar(
                "elapsed_time_s",
                Role::Phase,
                "s",
                Some(0.0),
                false,
                None,
                false,
                "at least 0 s: the time since the start of the heated window at the start of this request, and t_s of the state it starts from. It is required. A new pulse states 0; a run or a single step continued from a returned state states that state's t_s. A clock below zero is refused as NegativeTime. The card has no end in time: the longest compared heated window is 5.475 s, pulse 42982, and past it the card computes the same equations with nothing compared. A step too small to move the clock at its time, which a step of 1e-6 s first is at 2^34 s, is refused as StepBelowClockResolution".into(),
            ),
            scalar(
                "heating_power_w",
                Role::Load,
                "W",
                Some(0.0),
                false,
                None,
                false,
                "at least 0 W: the neutral-beam power less shine-through plus the coupled ion-cyclotron power; below zero is refused as HeatingPowerNegative. Compared at 0.124 to 24.3 MW of heating; the held-back hot-ion pulse 42976 reached 25.1 MW and missed all four marks. Outside that span the card computes the same equations. Lifting the compared span opens loads below the L-H power threshold, where the plasma is in L-mode and the H-mode scaling the card uses was not compared; the card has no limit there, and the compared runs start their heated window at 0.124 MW".into(),
            ),
            scalar(
                "ohmic_power_w",
                Role::Load,
                "W",
                Some(0.0),
                false,
                None,
                false,
                "at least 0 W: the ohmic power; below zero is refused as OhmicPowerNegative. Compared at 0.173 to 2.44 MW of ohmic power; outside it the card computes the same equations".into(),
            ),
            scalar(
                "line_density_per_m3",
                Role::Load,
                "1/m^3",
                Some(0.0),
                true,
                None,
                false,
                "above 0 per cubic metre: the line-averaged electron density; zero or less is refused as DensityNotPositive, since the slowing-down time divides by it. It is held to the Greenwald density of the segment's current, 1e20 x Ip[MA] / (pi x 0.9386^2) per cubic metre, and refused above it as DensityAboveGreenwald. Compared at line-averaged densities of 1.65e19 to 7.85e19 per cubic metre and at up to 0.834 of the Greenwald density; outside them the card computes the same equations. A frame carries FLAG_NEAR_DENSITY_LIMIT above 0.85 of the Greenwald density".into(),
            ),
            scalar(
                "plasma_current_a",
                Role::Load,
                "A",
                Some(0.0),
                true,
                None,
                false,
                "above 0 A: the plasma current, as a size; zero or less is refused as PlasmaCurrentNotPositive. With the toroidal field it sets the edge safety factor, 5 a^2 Bt / (R Ip[MA]) times a shape factor of 2.428, and a pair under 2 is refused as SafetyFactorBelowLimit. Compared at 1.65 to 3.84 MA; 42976 at 4.16 MA, outside the regime, missed all four marks. Compared at edge safety factors of 3.66 to 4.14 (the card's formula; JET's own equilibrium value is about a fifth higher); 42976 reached 3.39. Outside these the card computes the same equations".into(),
            ),
            scalar(
                "toroidal_field_t",
                Role::Load,
                "T",
                Some(0.0),
                true,
                None,
                false,
                "above 0 T: the toroidal field at the major radius, as a size; zero or less is refused as ToroidalFieldNotPositive. With the plasma current it must keep the edge safety factor at 2 or more. Compared at 1.80 to 4.04 T; outside it the card computes the same equations".into(),
            ),
            scalar(
                "tritium_fraction",
                Role::Load,
                "1",
                Some(0.0),
                false,
                Some(1.0),
                false,
                "from 0 to 1, both inclusive: tritium atoms per fuel atom; outside it is refused as FuelMixOutsideCard. 0 is deuterium and 1 is tritium, and neither gives D-T fusion power. Fusion power was compared at tritium fractions 0 to 0.034 (deuterium and the trace-tritium pulse 43134) and 0.451 to 0.504 (the held-back 42982 at 0.451 and the fitted 42762 at 0.503, rounded up by 0.001); between them nothing was compared, and at 0.75 to 0.93 three training pulses showed it wrong by 33 to 65 percent of its peak. On held-back 42982 the fusion energy was 0.743 of the measured. At every fraction the card computes the same D-T power, in proportion to the deuterium fraction times the tritium fraction, and its alpha heating feeds the fast-ion store".into(),
            ),
        ],
        floor,
        "a locked 1e-6 s: at 1 MW it moves a store of 20 MJ by one joule, which 64-bit numbers resolve to nine figures, and it moves the clock up to 2^34 s",
        gate,
        "a locked 1 ms, the largest step the card stands behind anywhere; past the compared envelope the step ceiling follows the state. The card's own step refuses as DtAboveStateCeiling, with nothing changed, a step longer than the smallest of an eighteenth of the slowing-down time, a forty-sixth of the loss time, 0.31 times the confinement time, and a forty-eighth of the coupling time, 2 times the slowing-down time times the thermal energy over the fast-ion energy, at the state the step starts from and where its trial step lands. Each fraction is the largest whole one at which 1 ms still fits at the fastest corner of the compared envelope: a slowing-down time of 18.0 ms at 0.777 keV and 7.85e19 per cubic metre, a loss time of 46.5 ms at the smallest compared confinement time of 0.150 s, and a coupling time of 48.1 ms at 0.777 keV with 1.62 MJ of fast ions. Inside the envelope the state ceiling is at least 1.0018 ms, so every compared run lands as before. Over the eight training pulses at 1 ms the largest conservation residual is 5.1e-10 against a limit of 2e-8, and past every old wall of the envelope, at the state ceiling, at most 1.5e-9. A start with eight times the thermal energy in its fast ions at 2e20 per cubic metre and 1 keV passes the ledger's limit after about a hundred steps at the state ceiling and is refused as FirstLawBreach; at a quarter of the ceiling it closes. In 0.6.0 the ceiling rested on the slowing-down time alone, and states with a confinement time near 37 ms were refused as FirstLawBreach although no energy was lost; the loss time now holds that step",
        None,
        "The compared envelope is that of the nine ELMy H-mode pulses of the research record, the eight fitted and the held-back 42982, run through the card at 1 ms by the protocol's rule, each bound the smallest or largest value of those runs rounded outward in the third figure. The held-back pulse 42976, a hot-ion pulse declared outside the card's regime before it was run, is not in it and missed all four marks. Outside the envelope the card computes the same equations and nothing was compared there; each field's bound states the compared span and how the card did. Every load field is held through a segment and a later segment may change any of them: a run of a pulse is one segment for each interval of its measured traces. A value outside its published bound, a density above the Greenwald density of the segment's current, or a current and field whose safety factor is under 2, in any segment of a run, refuses the whole run before its first step. The state is held at the start and after every step to the limits of physics, a thermal energy above zero, the range of the reactivity fit and the Troyon limit, and every step to the ceiling of its state (DtAboveStateCeiling, under dt_s); these depend on the state, so a run that reaches one of them part way is sealed with that refusal on its last landed step. A run starts at elapsed_time_s with its ledger anchored at the two energies given and no fusion energy released. A single step starts in the same way at each call: its thermal_energy_j, fast_ion_energy_j and t_s handed back as a new start, t_s as elapsed_time_s, continue the same plasma on the same clock, and fusion_energy_j and the ledger begin again at zero; no limit depends on either. The state reports the temperature, the loss power, the confinement time, the slowing-down time, the fusion power and its thermal part, the normalised beta, the Greenwald fraction and the safety factor under the load of the last step; they are zero at the start. A frame carries FLAG_NEAR_DENSITY_LIMIT while the density is above 0.85 of the Greenwald density; the compared span reached 0.834. A step of zero or less is refused as NonPositiveDt before the step floor is tested. The clock is a running sum from elapsed_time_s, t_s += dt_s in 64-bit arithmetic, and the card has no end in time.".into(),
        &[
            "NonFiniteInput",
            "NonFiniteState",
            "NonPositiveDt",
            "InvalidCard",
            "HeatingPowerNegative",
            "OhmicPowerNegative",
            "PlasmaCurrentNotPositive",
            "ToroidalFieldNotPositive",
            "FuelMixOutsideCard",
            "DensityNotPositive",
            "DensityAboveGreenwald",
            "SafetyFactorBelowLimit",
            "EnergyOutsideCard",
            "TemperatureBelowReactivityFit",
            "TemperatureAboveReactivityFit",
            "BetaAboveTroyon",
            "NegativeTime",
            "StepBelowClockResolution",
            "DtAboveStateCeiling",
            "SpontaneousGeneration",
            "FirstLawBreach",
        ],
        vec![Advisory {
            name: "FLAG_NEAR_DENSITY_LIMIT",
            mask: tokamak_jet_dte1::FLAG_NEAR_DENSITY_LIMIT,
            seals: true,
        }],
        slots(&[
            ("t", "Time since the start of the heated window, in seconds."),
            ("pos[0]", "thermal_energy_j"),
            ("pos[1]", "fast_ion_energy_j"),
            ("pos[2]", "fusion_energy_j"),
            ("vel[0]", "fusion_power_w"),
            ("vel[1]", "temperature_kev"),
            ("vel[2]", "beta_normalised"),
            ("force_torque", "Loss power of the thermal plasma, in watts."),
            ("residual", "Conservation residual."),
            (
                "flags",
                "FLAG_NEAR_DENSITY_LIMIT: the line-averaged density is above 0.85 of the Greenwald density. It seals. The compared span of the density reached 0.834 of the Greenwald density; from 0.85 to the Greenwald density itself the frame carries it.",
            ),
            ("proof", "Not written by the step."),
        ]),
    )
}

/// The data of the JSI TRIGA reactor's validation record.
pub(crate) const REACTOR_TRIGA_JSI_DATA: &str = "The pulse database of the Jozef Stefan Institute, trigapulse.ijs.si (I. Svajger and L. Snoj): a table of 757 pulses from 1991 to 2023 and, for each pulse, a record of power and two fuel thermocouples at 20,000 samples a second. The site states no licence and no terms. The constants taken from publications are from Svajger et al., Nuclear Engineering and Technology 56 (2024) 1165, an open-access article under the CC BY-NC-ND licence, and the hot-rod factor is from I. Vavtar, master thesis, University of Ljubljana, 2019, which states no licence. None of the site's measured values is stored in the card. 44 records are on disk, 43 of them of cores 215, 217 and 231. All 43 were opened and scored under the card's first protocol and by its second scoring, so none is unseen. The ten fitted constants were taken from 36 of them: 1.4 to 3.0 printed dollars, a peak at least ten times the noise, a scram found, a printed value that follows the rod table. Held back, by rule: the table's peak power, energy within 50 ms of the peak and thermocouple maximum for the 20 rows of the same cores whose records had not been fetched when they were scored, 12 of 22 and 28 March 2018 and 8 repeats of 24 March 2015; 15 of those records were fetched on 2026-10-11, after the scoring, for the study of element 6753. The comparison is not blind on three of its six lines: before this protocol was fixed on 2026-10-09, the peak power of every row held back had been printed beside the first card, in its result and in its second scoring, and that second scoring had printed the spread and range of the thermocouple maxima of the 2015 repeats; this card was rebuilt after the first card's result, and its thirteenth form was chosen after the twelve fixed beforehand had failed on the opened pulses. The energies within 50 ms of both groups and the thermocouple maxima of 2018 had not been read.";

/// The pass line of the JSI TRIGA reactor's validation record.
pub(crate) const REACTOR_TRIGA_JSI_PASS_LINE: &str = "Protocol evidence/research/reactor_triga_jsi/validation_protocol_v2.md, fixed before anything was fitted (SHA-256 of its machine-readable copy 9a0720f1822161c3b53cbc351d862ec4c077da51521ee1a96d7ceb85d8275ddc), with two amendments. The uncertainty of a comparison: the repeat scatter of one pulse within a day (peak power 1.3 percent, width 0.5 percent, energy within 50 ms of the peak 0.7 percent, energy to the scram 0.8 percent, thermocouple maximum 0.9 K) and the change for 0.03 printed dollars, the shift of the day's reference; for a row without a record also the start temperature and the scram instant, which are set by rule. On the opened pulses, in four bins of reactivity: root mean square of the error in units of the uncertainty at most 1.5 for peak power, for width and for energy to the scram; the card's range is the unbroken run of bins that hold, and outside it the card refuses. On the rows held back, for each group and each of peak power, energy within 50 ms of the peak and thermocouple maximum: root mean square at most 1.5, worst at most 3, mean at most 2 over the square root of the number of days. A straight line in printed reactivity, the nearest opened pulse, the Fuchs-Nordheim relations with the institute's constants, the unfitted model and the first card are scored beside the card; a mark that the straight line or the nearest pulse also meets is not evidence for the model.";

/// The one-line summary of the JSI TRIGA reactor, as the list shows it.
pub(crate) const REACTOR_TRIGA_JSI_SUMMARY: &str = "The TRIGA Mark II reactor of the Jozef Stefan Institute, Ljubljana, with its 56-element core of standard fuel, in pulse mode on the rod-worth tables of cores 215, 217 and 231: power, energy released and core-average fuel temperature from the fire signal through the pulse, the scram and after it. Ten constants are fitted to 36 pulses of this reactor, in a form added after the twelve fixed beforehand had failed; by the rule fixed beforehand the comparison with the 20 rows held back failed, four of six lines missed, with peak power 23 and 44 percent low on a day the card was not fitted to; the fuel temperature was compared only through one thermocouple of one element. It was compared at 2.0 to 3.0 printed dollars; below 2.0 dollars it read 29, 21 and 9 percent low in peak power at 1.4, 1.6 and 1.8 dollars. Outside that it computes the same equations with nothing compared: from any start in liquid water, at any reactivity above zero up to the rod's whole worth of 4.16 printed dollars, with the scram at any time or none.";

/// The result of the JSI TRIGA reactor's validation record.
pub(crate) const REACTOR_TRIGA_JSI_RESULT: &str = "On the opened pulses: the twelve forms fixed beforehand all read the power after the pulse too low, and by the rule fixed beforehand none held in any bin of reactivity. A thirteenth form, with the generation time fitted, was added after that result was seen (amendment 2) and holds in all four bins, 1.4 to 3.0 dollars, on the 36 pulses it was fitted to: peak power 4.1 percent root mean square (0.49 in units of the uncertainty), width 2.6 percent (0.80), energy to the scram 2.3 percent (0.96), each after the pulse channel's factor for that core's sessions. The thermocouple maximum, through the hot-rod factor 1.387 and a delay fitted to each recording chain: 4.9 K root mean square on 29 pulses, 1.8 percent of the rise, worst 8.6 K low. All of that is in sample. On the rows held back, run once on 2026-10-09, by the rule fixed beforehand the comparison failed: two of the six lines were met and four missed, and the two that were met are also met by the nearest opened pulse taken unchanged, so they are not evidence for the model. Core 231 on 22 and 28 March 2018, twelve rows on a fitted table and on days that were not fitted: all three quantities missed, each on its worst row. Peak power read 10.9 percent low on average, 18.0 percent root mean square, worst 44 percent low; in units of the uncertainty 1.21 root mean square, 3.02 worst against a line of 3, mean -0.74. Energy within 50 ms of the peak: 10.2 percent low, 15.7 percent root mean square; 1.47, 3.75 and -1.11. Thermocouple maximum, eleven rows: 2.6 K low on average, 8.0 K root mean square, worst 20.3 K low; 1.29, 3.10 and -0.39. The two rows of 22 March read 44 and 23 percent low in peak power: that day's printed 1.5 and 2.0 dollars sit at rod positions where the fitted day's printing gives 1.75 and 2.25, and the measured pulses lie between the two readings. On 28 March the six rows at 2.0 dollars and above read within 3.7 percent in peak power, 2.7 percent in energy and 5.8 K; the four below 2.0 dollars read from 29 percent low to level. Core 217 on 24 March 2015, eight repeats at 2.5 dollars between two fitted pulses: peak power met (5.6 percent high; 1.09, 1.33 and +1.07 against a mean line of 2), thermocouple maximum met (2.1 K low; 0.42 root mean square), energy within 50 ms missed (5.4 percent high; 2.01 against a line of 1.5). Beside the card on the twelve rows of 2018: the first card met the thermocouple maximum (0.97 root mean square, 1.59 worst) where this card missed, and was worse on peak power and energy (2.54 and 4.18 root mean square); a straight line in printed reactivity did as well as this card on energy and on the thermocouple maximum (1.45 and 1.33 root mean square, each missed on its worst row); the nearest opened pulse, the Fuchs-Nordheim relations and the unfitted model missed every line. One row, at 1.2 dollars, was below the card's range of that scoring and was not scored. Every number is in evidence/research/reactor_triga_jsi/analysis_v2/RESULTS_v2.md. The first protocol's result, on the first card: four of eight lines met and four missed. Its second scoring (evidence/verification/second_party/V-37) confirmed the count and found that card's energy to the scram 7 to 28 percent under the record's, its thermocouple maximum right only because a fitted ratio of 1.885 had absorbed the deficit, its four met lines met only with 5 percent of the reactivity charged to each pulse, and a scram after the rod had stopped refused under a false name. This card was rebuilt for those findings. Second scoring of this card under its second protocol, 2026-10-09 (evidence/verification/second_party/V-43): the count stands, two of six. What it adds. Of the six held-back lines three were on numbers nobody had read, the energy within 50 ms and the thermocouple maximum of the 2018 rows and the energy of the 2015 repeats, and all three were missed; both lines that were met are on numbers printed or bracketed before, and both are also met by the nearest opened pulse. The lines met depend on the 0.03 dollars charged for the day's reference: on the rows held back one of six with none or with 0.015 dollars, two with 0.03 and six with 0.06; in sample nine of the twelve bin lines with 0.015 dollars, which would have declared a range of 2.25 to 2.75 dollars only, and twelve with 0.03. The energy within 50 ms of the peak is 0.99 correlated with peak power on the 2018 rows and is not the energy to the scram; no held-back column tests the energy to the scram. After the held-back rows were scored the compared range of reactivity was set at 2.0 to 3.0 printed dollars. The reason is in the rows themselves: of the four below 2.0 dollars, all of 28 March 2018, the card read 29, 21 and 9 percent low in peak power at 1.4, 1.6 and 1.8 dollars and level on the repeat at 1.8, and on that day's six rows from 2.0 dollars up it was within 3.7 percent in peak power, 2.7 percent in energy and 5.8 K. This range was chosen with the held-back rows seen: it is not a blind result, it changes no line of the registered result above, and no unseen row tests it. The ten constants are unchanged and are still the fit to the 36 pulses from 1.4 to 3.0 dollars. The second scoring found the card below 2.0 dollars ill-conditioned and not biased, since 0.03 printed dollars moves the peak power by 44 percent at 1.4 dollars, 21 at 1.6, 13 at 1.8 and 9 at 2.0; engine 0.9.0 refused there, and the card now computes there with those misses stated. On 2.0 dollars and above, worked after the fact: in sample 27 pulses at 3.3 percent root mean square in peak power, 1.6 percent in energy to the scram and 4.7 K; held back, seven rows of 2018, with peak power and energy still missed on the row of 22 March at 2.0 dollars, 23 percent low, and the thermocouple maximum met. Support the card was not fitted to exists from 2.0 to 2.6 dollars on one day and at 2.5 dollars on core 217; nothing unfitted lies above 2.6 dollars.";

/// The limits of the JSI TRIGA reactor's validation record.
pub(crate) const REACTOR_TRIGA_JSI_LIMITS: &str = "Ten constants are fitted in the card: for each of the three rod-worth tables a scale and an offset on the printed dollars (scale 0.6921, 0.6677 and 0.7191, offset -0.1204, -0.1366 and -0.04949 dollars, for cores 215, 217 and 231); the feedback coefficient, 6.241e-5 per K at 294 K, and its rise, 8.718e-8 per K2; the generation time, 29.59 microseconds, where the article gives 40 with 8 percent; and the heat flow from fuel to water, 890.6 W/K. More are fitted outside the card, to instruments: the pulse channel's factor for each core's sessions (0.9838, 0.9743 and 1.043; their geometric mean is 1 by convention, so two are free), the thermocouple's delay in the recording chain of 2014 and 2015 (1.336 s), and two delays in series in the chain of 2018 (1.152 s each): five free numbers outside the card and fifteen fitted in all. For a session whose channel factor is not fitted the floor on power or energy is about 4 percent at one sigma, and 7 to 8 percent between two sessions (pulses 219 and 243). Sourced and not fitted: the delayed fraction and the six groups, the fuel mass, the heat capacity, the rod's rate, and the hot-rod factor 1.387. What is not determined: a pulse shows only products. A generation time of 29.59 microseconds with a printed dollar worth 0.69, and a feedback that falls back by a fifth with a time constant of 1.1 s after the pulse with a printed dollar worth 0.92 and the article's 40 microseconds, fit the opened pulses equally (pooled error 3.04 and 3.06 percent); the rule took the one with fewer constants. No constant of the card is therefore the reactor's own value of that quantity. What the pulses do determine: the inverse period for each printed dollar, 158 to 170 per second, 3 to 10 percent under the published 175, and prompt criticality at 1.27 to 1.32 printed dollars. The fitted generation time of 29.59 microseconds stands for the delayed fraction over the generation time, 237 per second against the published 175, and is not the reactor's generation time. The card cannot see the day. A printed dollar belongs to a day's reference as well as to a table; the validation charged 0.03 dollars for it, from two pairs of days, and 22 March 2018 needed about three times that. 0.03 printed dollars moves the peak power by 44 percent at 1.4 dollars, 31 at 1.5, 21 at 1.6, 13 at 1.8 and 9 at 2.0 (core 231), so below 2.0 dollars the uncertainty is as wide as the answer and a comparison passed inside it says little; the card computes there with nothing more compared. From 2.0 dollars up the day is still unseen: the two rows of 22 March 2018, printed 1.5 and 2.0 dollars, read 44 and 23 percent low, and a caller's printed 2.0 dollars of such a day is answered as the fitted day's. The power, and with it the energy and the rise of the fuel temperature, rests on the institute's pulse channel, whose calibration differed by 7.7 percent between November 2014 and March 2015 on two pulses of the same width and the same thermocouple reading; the card's power is the mean of three calibrations. The fuel temperature is the core average. It was compared with one thermocouple at the centre of one element, 6753 at position B1, with the instrument's response in between: the hot-rod factor is a calculated ratio of element powers (1.387 with 0.003, Table 4 of Appendix B of the thesis), the factors along the element and inside it at the thermocouple are not sourced and were taken as 1, and the delay is fitted. The traces are consistent with 1.387 and do not confirm it: with the factor left free they prefer 1.585 with a heat flow of 2930 W/K, and any factor from 1.39 to 1.73 fits as well or better. The card's heat flow, 890.6 W/K, is conditional on 1.387, and the thermocouple constrains the core-average rise only to roughly 5 percent above and 20 percent below. That comparison tests the product of energy, heat capacity and those factors seconds after the pulse. It does not test the temperature during the pulse, in which the thermocouple shows under 4 percent of its rise, nor the temperature of the hottest fuel. The dependence on the starting temperature was not tested apart. At 2.0 printed dollars and above the opened pulses started at 20.9 to 29.2 degC: compared on starts of 24.00 to 27.94 degC on core 215, 21.31 to 29.23 on core 217 and 20.90 to 22.81 on core 231. Every held-back row at 2.0 printed dollars and above started inside its own table's span. On the compared pulses the rods dropped 1.51 to 7.45 s after the peak; an earlier scram was not compared, no compared scram came while the transient rod was moving, and the longest compared hold was 7.45 s after the peak. The records compared run to 14 s after the fire signal, to 18.3 MW s released, 954 MW and a core-average fuel temperature of 585 K; the quadratic feedback was fitted from 294 to about 585 K. Outside all of that the card computes the same equations with nothing compared; it refuses only a start that is not liquid water, a reactivity of zero or less and one above the rod's whole worth. The rows held back test other days on fitted tables, an interpolation in reactivity; the start temperature and the scram instant of each were set by rule and not measured, and those at 2.0 printed dollars and above lie inside both envelopes. The scram's worth, 8 dollars, and the rod's rate and whole worth, 4.16 dollars from the table of core 253, are not fitted and not compared. Nothing tests another core loading, a second pulse on warm fuel, or a rod-worth table other than the three; the card refuses those it can see. The two second scorings, V-37 of the first card and V-43 of this one, were made by a second agent with programs of its own on the same machine; neither repeated the measurements. Not repeated on a second machine or by a second organisation.";

/// The one line of the C-core reactor in `list_cards`.
pub(crate) const REACTOR_SPERT3_CCORE_SPECIMEN: &str = "The SPERT III C-core test reactor (stainless-steel-clad, highly enriched plates) in step transients from room temperature at 0 to 2500 psig with no flow, or 200 to 20,000 gpm at 230 psig only, through the first maximum of power to the first maximum of the plate surface temperature at the hot spot; two constants are fitted, and by the rule fixed beforehand the comparison with its 39 held-back transients failed on all three lines.";

/// What the C-core reactor card is.
pub(crate) const REACTOR_SPERT3_CCORE_WHAT: &str = "The SPERT III reactor at the National Reactor Testing Station, Idaho, with the C-19/52 core: 93 percent enriched UO2-stainless-steel plates clad in stainless steel, 16.8 g of U-235 a plate, 30.6 kg in all, 36 in active length. Step insertions of reactivity from room temperature (21 to 36 degrees Celsius) at system pressures of 0 to 2500 psig with no forced flow, and at 230 psig with 200 to 20,000 gpm. Point kinetics with six delayed groups, and four lumped temperatures: the core-average plate and water, and a plate and water at the hot spot where the thermocouples sat, with the power density multiplied by the measured flux peaking 2.04 x 1.30. The temperature the card answers for is the plate SURFACE temperature at the hot spot, not a fuel temperature. A run ends at the first maximum of that temperature after the first maximum of power. Outside the compared starts, pressures, flows, reactivities and times the card computes the same equations, with nothing compared: from any start of liquid water at any absolute pressure above zero, any flow of zero or more, changed from segment to segment, any reactivity above zero, for as long as a run asks.";

/// The data behind the C-core reactor card.
pub(crate) const REACTOR_SPERT3_CCORE_DATA: &str = "IDO-16918, C. R. Toole, The Effects of Pressure and Flow on Room Temperature Power Excursions in Spert III, Phillips Petroleum Co. for the US Atomic Energy Commission, June 1964, OSTI 4042264: its appendix tabulates 126 step transients with pressure, initial temperature, flow, period, peak power, energy to peak and the maximum plate surface temperature read by thermocouples spot-welded to a plate at the flux peak of a central assembly. Core constants from IDO-17138 (Condit, Scott and Johnson, 1967), Table B-1. 15 rows were excluded before any run (11 with an abnormal thermocouple reading, two likely misprints, two with an initial temperature missing or in doubt); every third remaining row and the whole 1000 psig series were held back: 72 rows fitted, 39 held back.";

/// The pass lines of the C-core reactor card.
pub(crate) const REACTOR_SPERT3_CCORE_PASS_LINE: &str = "Fixed before any constant was fitted and before any model was run on any row (evidence/research/reactor_spert3_ccore/validation_protocol.md, SHA-256 8e2e56cf3e93099f19c6a17537acb76708dc99da682915ea602d84005cbae72f, with amendment 1). On the held-back rows, for each of peak power, energy to peak and the maximum plate surface temperature rise: exp(root mean square of ln(card / measured)) - 1 at most 20 percent, and below each of three baselines fitted on the same rows: Fuchs-Nordheim on the measured reciprocal period, the nearest fitted row, and a straight line in the logarithm of the reciprocal period. The comparison is not blind: the whole table, held-back rows included, had been read to confirm its transcription before the protocol was fixed, and the split that holds back the whole 1000 psig series and the rules that exclude rows were chosen with those values seen, before any model was run.";

/// The result of the C-core reactor card's comparison.
pub(crate) const REACTOR_SPERT3_CCORE_RESULT: &str = "Compared once with the 39 held-back transients on 2026-10-11, and by the rule fixed beforehand the comparison failed on all three lines. Peak power: 33.3 percent (38 rows), against Fuchs-Nordheim 176 percent, the nearest fitted row 16.7 percent and the straight line 70.4 percent. Energy to peak: 56.7 percent, against 204, 17.2 and 61.6. Maximum plate surface temperature rise: 71.0 percent (36 rows), against 181, 18.1 and 36.8. The card beats Fuchs-Nordheim and the straight line on power and energy and loses to the nearest fitted row on every line. On the held-back 1000 psig series: peak power 13.5 percent, energy 11.7 percent, temperature rise 67.9 percent, low.";

/// The limits of the C-core reactor card.
pub(crate) const REACTOR_SPERT3_CCORE_LIMITS: &str = "A miss, and it stays on the record. Two constants are fitted on 72 rows: the plate-to-coolant conductance with no flow, 6403.73 W/m^2 K (Dittus-Boelter adds the flow), and the share of fission energy deposited directly in the water, 0.551915. This is not the physical share, which is a few percent from gamma rays and neutron slowing: it is an effective value that absorbs the boiling and void feedback the card leaves out, and it should not be read as a property of the core. Compared at 0 to 2500 psig with no flow. The card has no boiling, no void, no melting of the plate and no pressure term, and gives the same transient at every pressure, where the measured energy to peak doubles from 0 to 2500 psig; on the held-back 1000 psig series peak power read 13.5 percent, energy 11.7 percent and temperature rise 67.9 percent low. The pressure sets only where the water at the start boils. Compared on starts at 21 to 36 degC; the water properties of the heat transfer are those of 25 degC; outside that span nothing was compared. The isothermal temperature coefficient is measured to 204.44 degC and continues straight past it. Flow was compared at 200 to 20,000 gpm (0.055 to 5.49 m/s) at 230 psig only; the Dittus-Boelter term is a turbulent correlation and is blended with the fitted stagnant conductance. No compared transient changed its flow. Compared with reactivities of 0.412 to 1.305 dollars with no flow and 0.870 to 1.305 with flow; on the 39 held-back transients it failed all three lines, peak power 33.3 percent, energy 56.7 percent, plate surface temperature rise 71.0 percent. Where the hot-spot plate temperature has no maximum before 200 s (long periods, and flow), the comparison took the rise reached at 200 s; those 11 held-back rows read 2.2 times high on average. The largest energy the card reaches inside its compared range is about 58,000 MW s (53,000 from a start at 36 degC), at the slowest compared flow insertion with the most flow, by 200 s; the largest power about 1050 MW. Past those the card computes the same transient, and nothing stops it in their place: it has no melting of the plate, no boiling and no void. The power and energy rows at flow and long period whose measured power still rose at the scram are on no line. The delayed groups are Keepin's for U-235, not measured for this core. The ledger names a wrong method; it does not show the physics matches the reactor.";

/// The one line an agent reads in `list_cards`.
pub(crate) const RUNAWAY_18650_LCO_SPECIMEN: &str = "Thermal runaway of one 18650 lithium-ion cell, LiCoO2 against graphite, in a held oven, under a heater or in an adiabatic calorimeter: the four decomposition stages of Hatchard et al. (2001) with every constant as Kim et al. (2007) publish it, the cell's heat capacity and its loss to the oven, an energy ledger every step. Compared after it was built, by lines fixed before any measured number was read, with open records of LiCoO2 18650 cells: 8 of 23 lines met. It self-heats at 83 degC where measured cells start at 100 to 150 degC, runs away in a 155 degC oven in 24 minutes where Hatchard's cell took 51, and runs away at 150 degC where that cell did not; its highest temperatures lie 30 to 85 K under six adiabatic measurements.";

/// What the card is.
pub(crate) const RUNAWAY_18650_LCO_WHAT: &str = "One 18650 lithium-ion cell, 18 mm across and 65 mm high, LiCoO2 cathode and graphite anode, as one lumped body in thermal abuse: SEI decomposition, the anode with the electrolyte (slowed by the SEI it grows), the cathode with the electrolyte (autocatalytic in its converted part) and the decomposition of the electrolyte, each an Arrhenius stage, heating the cell, which loses heat to an oven by convection and radiation or, in a calorimeter, none.";

/// The sources of the card's numbers and of the measurements it was compared with.
pub(crate) const RUNAWAY_18650_LCO_DATA: &str = "The card's constants, all published, none fitted: the kinetics, heats, contents and starting amounts of Ostanek (2025), Simulink Model for Thermal Runaway of a Single Li-ion Cell using the Hatchard-Kim Reaction Mechanism, Purdue University Research Repository, doi 10.4231/NWZX-2675, Table 2 (CC BY 4.0), which agree in the frequency factors, activation energies, heats and starting amounts with Lopez, Jeevarajan and Mukherjee (2015), J. Electrochem. Soc. 162, A2163, Table II; Lopez prints other contents (1.39e3, 1.3e3 and 5.0e2 kg/m3) and the card takes Ostanek's (610.4, 1221.0 and 406.9). The jelly roll's density, 2580 kg/m3, and specific heat, 830 J/(kg K), of Hatchard et al. (2001), as Coman et al. (2017), J. Electrochem. Soc. 164, A587, Table I, reprint them; the convective coefficient 7.17 W/(m2 K) and emissivity 0.8 of the oven test, from Lopez et al. (2015) and Ostanek (2025). Hatchard et al. (2001), J. Electrochem. Soc. 148, A755, and Kim, Pesaran and Spotnitz (2007), J. Power Sources 170, 476, are not open and were not read. Measurements compared, all open: Hatchard's oven tests of a LiCoO2 18650 at 145, 150 and 155 degC from 28 degC, as Lopez et al. (2015), Fig. 2a, reprint them and Ostanek et al. (2023), J. Electrochem. Soc. 170, 090512, state their timing (in sample: the oven constants are Hatchard's own); Golubkov et al. (2014), RSC Adv. 4, 3633, Table 3, an LCO/NMC 2:1 cell heated at 2 degC/min (CC BY 3.0); Mendoza-Hernandez et al. (2017), E3S Web Conf. 16, 07001, Table 1, a fresh LiCoO2 2550 mAh cell at 100 percent state of charge by heat-wait-seek (CC BY 4.0); and seven adiabatic LiCoO2 rows that Lei et al. (2017), Batteries 3, 14, Table 1, reprint from other papers (CC BY). Record: evidence/research/thermal_runaway_18650/, with every source, its licence and every run of the comparison.";

/// The lines of the comparison, fixed before any measured number was read.
pub(crate) const RUNAWAY_18650_LCO_PASS_LINE: &str = "Fixed after the card was built and before any measured number of the sources was read (evidence/research/thermal_runaway_18650/PROTOCOL.md, SHA-256 b704cdf7d0d8bdde297c8ffa7e4b157f4edf633f26890c94e6f4fbfe65a60ad4). For each scored test of an 18650 cell with a LiCoO2 cathode, alone or blended: the onset of self-heating within 15 K; the onset of runaway within 20 K; the highest temperature within 100 K; in a held oven, runaway if and only if the cell ran away, and the time from the start of the exposure to runaway within 25 percent; in a calorimeter, the time from the onset of self-heating to the onset of runaway within a factor of 2. Each line is met or missed alone; nothing is averaged across cells. Tests of NMC, NCA and LFP cells are reported as context and not scored.";

/// The result of the comparison.
pub(crate) const RUNAWAY_18650_LCO_RESULT: &str = "Compared once on 2026-10-14, through the gateway, by the lines fixed beforehand: 8 of 23 lines met, 2 of 10 on the primary records and 6 of 13 on the reprinted rows. Hatchard's oven at 155 degC: the card runs away, met, but reaches 20 K per minute at 24.3 minutes against 51.1 (Ostanek et al. 2023; 40.5 to 48 by eye from Lopez et al. 2015, Fig. 2a), 52 percent early, missed. At 150 degC the card runs away at 34.7 minutes and peaks at 481.7 degC where the cell did not run away in 120 minutes, missed; at 145 degC the card reaches 10 K per minute at 129 minutes and peaks at 193.9 degC where the cell held near 148 degC, missed by the rule, though inside the 120 minutes the record covers the card does not run away either. Golubkov et al. 2014, LCO/NMC heated at 2 degC/min: self-heating onset 83.1 degC against 149, runaway onset 177.4 degC against 208, highest temperature 700.7 degC against 853, all missed. Mendoza-Hernandez et al. 2017, fresh LiCoO2 by heat-wait-seek: self-heating onset 83.7 degC against 100.0, missed; runaway onset at 1 degC/min 154.0 degC against 144.9, met; self-heating to runaway 11.5 hours against about 5.7, a factor of 2.03, missed. The seven reprinted LiCoO2 rows: every onset missed, 32.8 to 58.8 K early; every one of the six highest temperatures met, 624.4 degC against 654.3 to 708.8. Before the comparison the card's equations were run outside the engine against an independent integration (within 0.002 K before the runaway and 0.2 K at its peak) and, with Ostanek's own constants, against his published Simulink run (peak within 0.5 K, its time 41 s early). The same equations with Ostanek's constants also run away at 150 degC: the early runaway is the published model's, not this card's arithmetic.";

/// What the comparison does not test, and what the card leaves out.
pub(crate) const RUNAWAY_18650_LCO_LIMITS: &str = "A miss on four of five primary records, and it stays on the record. The card is the lumped Hatchard-Kim model as published: one temperature for the whole cell, with the jelly roll's density and specific heat over the can's whole volume, 35.42 J/K, where a measured 44 g LiCoO2 cell holds about 42 J/K (FAA DOT/FAA/TC-TN15/17); the can's whole surface, ends included, loses heat. Left out: venting and the heat and electrolyte the vented gas carries, its combustion, the internal short circuit, the electrical energy of the charge, the melting of the separator and of the aluminium collector, a heat capacity that changes with temperature, the temperature inside the cell, the calorimeter's own heat capacity and its tracking. The card is a fully charged cell; it has no state of charge and no ageing. The anode stage slows itself as its SEI grows: in a cell that cools, about a quarter of the anode's lithium is left, and the end EveryReactantConsumed, the heat held falling to 0.0205 J, is met by a cell held at 1,200 K after 1.6 hours and at 900 K after 6.2 days; the runaway's peak is in the frames. The commercial cells compared are not Hatchard's cell: their loadings, electrolytes, vents and makers differ, and the measured onsets use thresholds and fits the card reads at its own 0.02 K per minute (FLAG_SELF_HEATING) and 10 K per minute (FLAG_RUNAWAY) or at the paper's stated rate. Hatchard's oven records are read by eye from a reprinted figure, and two readings of the 155 degC timing differ by 3 to 10 minutes. The secondary rows are reprints whose heat-wait-seek settings and thresholds are not stated. The ledger holds the heat stored to the heat released, the heater's heat and the heat lost, within 1e-6 of the cell's chemical heat: it catches a step that does not follow the reactions or a falsified state, and it cannot catch a wrong constant or a wrong mechanism. One party wrote the card, fixed the lines and ran the comparison, on one machine.";

fn runaway_18650_lco_card() -> Card {
    let cell = runaway_18650_lco::Runaway18650Lco::hatchard_kim();
    let gate = cell.gate_dt_s().expect("specimen gate");
    let floor = cell.floor_dt_s().expect("specimen floor");
    finish(
        "energy",
        "runaway_18650_lco",
        RUNAWAY_18650_LCO_SPECIMEN,
        vec![
            scalar(
                "temperature_k",
                Role::Phase,
                "K",
                Some(0.0),
                true,
                Some(runaway_18650_lco::CAN_MELTING_K),
                true,
                "exclusive (0, 1811) K: the temperature of the cell at the start, one for the whole body. At zero kelvin and below it is refused as TemperatureNotPositive; at 1811 K, the melting point of iron, where the steel can melts, and above, as CanMelting. A step whose temperature would reach either inside one of its substeps is refused by the same name, and the run ends there, sealed".into(),
            ),
            scalar(
                "sei_fraction",
                Role::Phase,
                "1",
                Some(0.0),
                false,
                Some(runaway_18650_lco::AMOUNT_LIMITS[0]),
                false,
                "from 0 to 1, inclusive: the lithium in the solid-electrolyte interphase at the start, c_sei, a fraction. A fresh cell is 0.15; a start above it is a cell with more of its lithium in the SEI, and the stage releases its heat in proportion. Below 0 or above 1 it is refused as SeiFractionOutsideCell".into(),
            ),
            scalar(
                "anode_lithium_fraction",
                Role::Phase,
                "1",
                Some(0.0),
                false,
                Some(runaway_18650_lco::AMOUNT_LIMITS[1]),
                false,
                "from 0 to 0.783, inclusive: the lithium in the graphite at the start, c_ne. A fresh cell is 0.75. The SEI thickness the anode stage grows and is slowed by is 0.783 minus this, so a start below 0.75 is a cell whose anode has already reacted that much, and a start above it a cell with a thinner SEI, down to none at 0.783. Below 0 or above 0.783, where the SEI thickness would be below zero, it is refused as AnodeLithiumOutsideCell".into(),
            ),
            scalar(
                "cathode_unreacted_fraction",
                Role::Phase,
                "1",
                Some(0.0),
                false,
                Some(runaway_18650_lco::AMOUNT_LIMITS[2]),
                false,
                "from 0 to 1 - 2^-33 (0.99999999988), inclusive, or exactly 1: the unreacted part of the cathode at the start, 1 minus the converted part alpha. A fresh cell is 0.96, alpha 0.04. The cathode stage is autocatalytic, alpha (1 - alpha): at exactly 1 it has no converted part and never starts. The card carries the unreacted part in 64 bits, which resolve alpha to 2^-53, so above 1 - 2^-33 and under 1, where alpha is under 1.16e-10 and carried to fewer than 20 bits, its growth would be lost to rounding: there, below 0 and above 1 it is refused as CathodeUnreactedOutsideCell".into(),
            ),
            scalar(
                "electrolyte_fraction",
                Role::Phase,
                "1",
                Some(0.0),
                false,
                Some(runaway_18650_lco::ELECTROLYTE_START),
                false,
                "from 0 to 1, inclusive: the electrolyte at the start, c_e, a fraction of the cell's fill. A fresh cell is 1. Below 0 or above 1 it is refused as ElectrolyteOutsideCell. A start whose four amounts together hold 0.0205 J of heat or less, 1e-6 of the cell's chemical heat, is a spent cell: it is stepped as a body that gains and loses heat, under the oven and the heater, and meets no end".into(),
            ),
            scalar(
                "oven_temperature_k",
                Role::Load,
                "K",
                None,
                false,
                None,
                false,
                "above 0 K when adiabatic is 0: the temperature of the oven, or of the surroundings, held over a step. The cell exchanges heat with it by convection, 7.17 W/(m2 K) over the can's 41.85 cm2, and by radiation with an emissivity of 0.8; at zero kelvin and below it is refused as OvenTemperatureNotPositive. When adiabatic is 1 no heat crosses the can and the oven temperature does not enter the step or the ledger: it is still required, any temperature of 0 K or more is taken, and the step is the same at every one. A negative number of kelvin is no temperature, and the door refuses it as OvenTemperatureNotPositive either way".into(),
            ),
            scalar(
                "heater_power_w",
                Role::Load,
                "W",
                Some(0.0),
                false,
                None,
                false,
                "at least 0 W: heat a heater puts into the cell, held over a step. 35.42 W warms the cell by 1 K a second with no reaction. Below zero it is refused as HeaterPowerNegative".into(),
            ),
            scalar(
                "adiabatic",
                Role::Load,
                "1",
                Some(0.0),
                false,
                Some(1.0),
                false,
                "0 or 1 and nothing between: 1 for a cell in an accelerating-rate calorimeter that tracks it, so that no heat crosses the can and the oven temperature does not enter; 0 for a cell that loses heat to the oven. Any other value is refused as AdiabaticNotZeroOrOne".into(),
            ),
        ],
        floor,
        "the ceiling divided by 2^20, about 1.13e-5 s",
        gate,
        "11.805 s, a hundredth of the cell's convective time constant C / (h A), 1180.5 s",
        None,
        "Over a step the oven temperature, the heater power and the adiabatic switch are held. Inside a step the card takes substeps, each classical fourth-order Runge-Kutta over the temperature, the natural logarithm of each of the four amounts and the heat lost, and each no longer than 0.1 over the fastest live stage's rate in its logarithm (its stiffness) nor than 1 K of temperature change at the substep's starting rate. A stage whose remaining heat is under 2^-52 of the cell's whole chemical heat no longer sets the substep. At the peak of a runaway one step takes several hundred substeps; a step that would need more than 1,048,576 is refused as SubstepsAboveBudget. The ceiling and the floor bound the step a caller asks for, not the substeps. A run starts at time zero with every heat sum at zero and the start temperature as the anchor of the heat stored; they begin again at each start, and no limit of the card depends on them. The heat each stage released is booked in closed form from its amount, the heater's heat as its power times the step, the heat lost with the temperature: the ledger holds the heat stored, the heat capacity times the rise from the start, to the heat released plus the heater's less the heat lost, within 1e-6 of the cell's whole chemical heat, 20,499 J. The onset of self-heating (FLAG_SELF_HEATING, chemical heat warming the cell at 0.02 K per minute or more) and of runaway (FLAG_RUNAWAY, 10 K per minute or more) are flags on the frame, and the step lands. The end of the runaway is EveryReactantConsumed: the heat the four amounts still hold falls to 1e-6 of the cell's chemical heat or under, 0.0205 J, the ledger's own limit, heat that would warm the cell by under 0.6 mK. It is tested at the end of every substep, so whether a run meets it, and when, does not follow the step asked for; the step that would carry the cell there is not taken, and the run ends there, sealed to the last step that landed. A start that holds no more than that heat is a spent cell: the card steps it as a body that gains and loses heat, under the oven and the heater, and no end is met. In this model the anode stage slows itself as its SEI grows, to A exp(-E / R T) exp(-0.783 / 0.033) per second as its lithium runs out: in every oven run worked here about a quarter of the anode's lithium is left when the cell cools, and the runaway's peak is in the frames and the state. A cell held at a temperature after the 155 degC oven run's runaway meets the end after 1.6 hours at 1,200 K, 5.6 hours at 1,100 K, 24.6 hours at 1,000 K, 6.2 days at 900 K and 60 days at 800 K; at 700 K it has not met it after 410 days.".to_string(),
        &[
            "NonFiniteInput",
            "NonFiniteState",
            "NonPositiveDt",
            "InvalidCell",
            "OvenTemperatureNotPositive",
            "HeaterPowerNegative",
            "AdiabaticNotZeroOrOne",
            "TemperatureNotPositive",
            "CanMelting",
            "SeiFractionOutsideCell",
            "AnodeLithiumOutsideCell",
            "CathodeUnreactedOutsideCell",
            "ElectrolyteOutsideCell",
            "EveryReactantConsumed",
            "SubstepsAboveBudget",
            "SpontaneousGeneration",
            "FirstLawBreach",
        ],
        vec![
            Advisory {
                name: "FLAG_SELF_HEATING",
                mask: runaway_18650_lco::FLAG_SELF_HEATING,
                seals: true,
            },
            Advisory {
                name: "FLAG_RUNAWAY",
                mask: runaway_18650_lco::FLAG_RUNAWAY,
                seals: true,
            },
        ],
        slots(&[
            ("t", "Time since the start of the run, in seconds."),
            ("pos[0]", "temperature_k"),
            ("pos[1]", "chemical_heat_w"),
            ("pos[2]", "heat_released_j"),
            ("vel[0]", "anode_lithium_fraction"),
            ("vel[1]", "cathode_unreacted_fraction"),
            ("vel[2]", "electrolyte_fraction"),
            (
                "force_torque",
                "The heat the cell has lost to the oven since the start, in joules; negative while the oven warms the cell.",
            ),
            ("residual", "Conservation residual: the heat ledger's defect over the cell's whole chemical heat, 20,499 J."),
            (
                "flags",
                "FLAG_SELF_HEATING: the chemical heat warms the cell at 0.02 K per minute or more. FLAG_RUNAWAY: at 10 K per minute or more. Both seal.",
            ),
            ("proof", "Not written by the step."),
        ]),
    )
}

/// One step at 10 s of a fresh cell at 298.15 K put in an oven at 155 degC,
/// and a run of 200 such steps, through its runaway near 1,500 s.
fn runaway_18650_lco_examples() -> (Value, Value) {
    pair_examples_of(
        "energy",
        "runaway_18650_lco",
        obj(&[
            ("temperature_k", scalar_value(298.15)),
            ("sei_fraction", scalar_value(runaway_18650_lco::SEI_FRACTION_START)),
            ("anode_lithium_fraction", scalar_value(runaway_18650_lco::ANODE_LITHIUM_START)),
            ("cathode_unreacted_fraction", scalar_value(1.0 - runaway_18650_lco::CATHODE_CONVERSION_START)),
            ("electrolyte_fraction", scalar_value(runaway_18650_lco::ELECTROLYTE_START)),
        ]),
        obj(&[
            ("oven_temperature_k", scalar_value(428.15)),
            ("heater_power_w", scalar_value(0.0)),
            ("adiabatic", scalar_value(0.0)),
        ]),
        scalar_value(10.0),
        200,
    )
}

fn reactor_spert3_ccore_card() -> Card {
    let specimen = reactor_spert3_ccore::ReactorSpert3Ccore::c_core_step_transients();
    let gate = specimen.gate_dt_s().expect("specimen gate");
    let floor = specimen.floor_dt_s().expect("specimen floor");
    finish(
        "energy",
        "reactor_spert3_ccore",
        REACTOR_SPERT3_CCORE_SPECIMEN,
        vec![
            scalar(
                "initial_temperature_k",
                Role::Phase,
                "K",
                Some(reactor_spert3_ccore::FREEZING_K),
                false,
                Some(reactor_spert3_ccore::CRITICAL_TEMPERATURE_K),
                true,
                format!(
                    "the temperature of plates and water at the step: liquid water, at or above {} K and below the saturation temperature of water at system_pressure_psig (IAPWS-IF97 region 4, backward equation; 373.12 K at 0 psig, 627.06 K at 2500 psig), and below the critical temperature, {} K, above the critical pressure, 22.064 MPa. A start that is not liquid water is refused as InitialTemperatureNotLiquidWater. Compared on starts at 21 to 36 degC; the water properties of the heat transfer are those of 25 degC; outside that span nothing was compared",
                    reactor_spert3_ccore::FREEZING_K,
                    reactor_spert3_ccore::CRITICAL_TEMPERATURE_K,
                ),
            ),
            scalar(
                "system_pressure_psig",
                Role::Phase,
                "psig",
                Some(-reactor_spert3_ccore::ATMOSPHERE_PSI),
                true,
                None,
                false,
                format!(
                    "the system pressure in pounds per square inch above the atmosphere, above absolute zero, -{} psig; at or below it the run is refused as PressureNotPositive. It sets only where the water at the start boils. Compared at 0 to {} psig with no flow, and at {} psig with flow. The card has no pressure term and gives the same transient at every pressure, where the measured energy to peak doubles from 0 to 2500 psig; on the held-back 1000 psig series peak power read 13.5 percent, energy 11.7 percent and temperature rise 67.9 percent low",
                    reactor_spert3_ccore::ATMOSPHERE_PSI,
                    reactor_spert3_ccore::PRESSURE_MAX_PSIG,
                    reactor_spert3_ccore::FLOW_PRESSURE_MIN_PSIG,
                ),
            ),
            scalar(
                "reactivity_inserted_dollar",
                Role::Load,
                "dollar",
                Some(0.0),
                true,
                None,
                false,
                format!(
                    "the reactivity stepped in at time zero, above zero; zero or less is refused as ReactivityNotPositive. Compared with reactivities of [{}, {}] dollars with no flow and [{}, {}] with flow, the inhour values (with the card's generation time and groups, summed in group order) of the longest and the shortest compared period, 9.3 s or 516 ms to 9.7 ms; on the 39 held-back transients it failed all three lines, peak power 33.3 percent, energy 56.7 percent, plate surface temperature rise 71.0 percent. Above about 1.31 dollars the step ceiling follows the state (DtAboveStateCeiling). A run carries one reactivity: a segment with another is refused as ReactivityChangedDuringRun",
                    reactor_spert3_ccore::REACTIVITY_MIN_NO_FLOW_DOLLAR,
                    reactor_spert3_ccore::REACTIVITY_MAX_DOLLAR,
                    reactor_spert3_ccore::REACTIVITY_MIN_FLOW_DOLLAR,
                    reactor_spert3_ccore::REACTIVITY_MAX_DOLLAR,
                ),
            ),
            scalar(
                "coolant_flow_m_per_s",
                Role::Load,
                "m/s",
                Some(0.0),
                false,
                None,
                false,
                format!(
                    "the coolant velocity through the core, 18 ft/s at 20,000 gpm, zero or more; below zero it is refused as NegativeFlow. Flow was compared at [{}, {}] m/s (200 to 20,000 gpm) at {} psig only; the Dittus-Boelter term is a turbulent correlation and is blended with the fitted stagnant conductance. Each segment may carry its own flow, and each step is taken at its own; no compared transient changed its flow. Above about 5.5 m/s the step ceiling follows the state (DtAboveStateCeiling)",
                    reactor_spert3_ccore::FLOW_MIN_M_PER_S,
                    reactor_spert3_ccore::FLOW_MAX_M_PER_S,
                    reactor_spert3_ccore::FLOW_PRESSURE_MIN_PSIG,
                ),
            ),
        ],
        floor,
        "a locked 1e-9 s: the card's own constant, a step that still moves the clock at 600 s, the longest a run of 1,000,000 steps at the ceiling reaches",
        gate,
        "0.2 of the reduced generation time, 3 ms, the time constant of the prompt pole at zero net reactivity, where every run starts; it holds every state of the compared range. Past the compared range the step ceiling follows the state: at the state a step starts from and at its trial end, the card refuses as DtAboveStateCeiling a step that would advance the growing prompt pole (net reactivity - 1) / l* by more than 0.0611, the decaying prompt pole (1 - net reactivity) / l* by more than 0.5, or the plate and water heat transfer and sweep H/C_p + H/C_w + 2v/L by more than 0.0184, the most a step at this ceiling advances each inside the compared range (at 1.305 dollars, at -1.48 dollars net, at 5.4864 m/s). At 3 dollars the longest step at the start is 0.092 ms, at 10 dollars 0.020 ms, at 100 m/s 0.046 ms. Measured with the card's own step at the state ceiling: the ledger closes on bursts to 1.5 dollars and on flows to 300 m/s; on bursts of 2 dollars and more it names a step after the power peak, where the feedback turns the power faster than the prompt poles say and the decaying prompt pole carries the power's fall, and there a shorter step closes it to the plate maximum, 0.1 ms at 2 dollars, 0.023 ms at 5 dollars and 0.001 ms at 30 dollars. The three phases are not yet worked out from the step's own error",
        None,
        format!(
            "A run is one transient with one reactivity: every segment carries the reactivity of the first, and each segment may carry its own flow. The reactor starts at {} MW in equilibrium with every temperature rise zero, and the reactivity is a step at time zero. hot_spot_plate_rise_k is the rise of the plate surface temperature at the hot spot above the initial temperature, the quantity the thermocouples measured; plate_rise_k and water_rise_k are the core averages. The run passes the first maximum of power (FLAG_PAST_POWER_PEAK is set from that step on; peak_power_mw, peak_energy_mws and peak_time_s hold the highest landed sample before it, within one step of the true maximum; the card does not interpolate) and ends at the first maximum of the hot-spot plate temperature after it: the step after is refused as PastPlateTemperatureMaximum and the run is sealed with that refusal, the outcome ended. plate_rise_max_k holds the highest landed hot-spot rise. At long periods and with flow the hot-spot temperature can have no maximum before {} s, where the comparison took the rise reached; the run goes on, and with flow the temperatures settle and a run with no plate maximum is completed. With no flow and a reactivity just under prompt critical the net reactivity falls below -1.48 dollars after 200 s and the ledger or the state ceiling names where the run stops. A single step starts again from the card's start at each call, so single steps cannot be chained. NegativeState and InconsistentState are refusals of the card's step for a state handed to it that the card did not write; this door takes the initial temperature and the pressure and nothing else of a state, so no request reaches them. A step of zero or less is refused as NonPositiveDt before the step floor is tested.",
            format!("{:e}", reactor_spert3_ccore::INITIAL_POWER_MW),
            reactor_spert3_ccore::COMPARED_TIME_S,
        ),
        &[
            "NonFiniteInput",
            "NonFiniteState",
            "NonPositiveDt",
            "InvalidCard",
            "PressureNotPositive",
            "InitialTemperatureNotLiquidWater",
            "NegativeFlow",
            "ReactivityNotPositive",
            "PastPlateTemperatureMaximum",
            "NegativeTime",
            "NegativeState",
            "DtAboveStateCeiling",
            "ReactivityChangedDuringRun",
            "InconsistentState",
            "SpontaneousGeneration",
            "FirstLawBreach",
        ],
        vec![
            Advisory {
                name: "FLAG_PROMPT_SUPERCRITICAL",
                mask: reactor_spert3_ccore::FLAG_PROMPT_SUPERCRITICAL,
                seals: true,
            },
            Advisory {
                name: "FLAG_PAST_POWER_PEAK",
                mask: reactor_spert3_ccore::FLAG_PAST_POWER_PEAK,
                seals: true,
            },
        ],
        slots(&[
            ("t", "Time since the step of reactivity, in seconds."),
            ("pos[0]", "power_mw"),
            ("pos[1]", "energy_mws"),
            ("pos[2]", "reactivity_net_dollar"),
            ("vel[0]", "hot_spot_plate_rise_k"),
            ("vel[1]", "water_rise_k"),
            ("vel[2]", "peak_power_mw"),
            ("force_torque", "Rate of change of power, in MW per second."),
            ("residual", "Conservation residual: the largest of the neutron, energy and hot-spot energy balances, each relative to its own scale."),
            (
                "flags",
                "FLAG_PROMPT_SUPERCRITICAL: net reactivity above one dollar. FLAG_PAST_POWER_PEAK: the first maximum of power has been passed. Both seal.",
            ),
            ("proof", "Not written by the step."),
        ]),
    )
}

/// One step at the card's ceiling from 25 degC at atmospheric pressure with no
/// flow, and a run of a 1.1 dollar step through its power peak to the first
/// maximum of the hot-spot plate temperature.
fn reactor_ccore_examples() -> (Value, Value) {
    let reactor = reactor_spert3_ccore::ReactorSpert3Ccore::c_core_step_transients();
    pair_examples_recorded(
        "energy",
        "reactor_spert3_ccore",
        obj(&[
            ("initial_temperature_k", scalar_value(298.15)),
            ("system_pressure_psig", scalar_value(0.0)),
        ]),
        obj(&[
            ("reactivity_inserted_dollar", scalar_value(1.1)),
            ("coolant_flow_m_per_s", scalar_value(0.0)),
        ]),
        card_dt(reactor.gate_dt_s()),
        REACTOR_CCORE_EXAMPLE_STEPS,
        100,
    )
}

/// The steps the worked run of the C-core reactor asks for: 0.6 s, past the
/// first maximum of the hot-spot plate temperature of a 1.1 dollar step.
pub(crate) const REACTOR_CCORE_EXAMPLE_STEPS: u64 = 1_000;

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
                    "the fuel temperature at the fire signal, which is also the pool water's temperature, inclusive: liquid water at one standard atmosphere, from {} K, where water freezes, to {} K, where it boils (IAPWS-IF97, the pool open to the air). A start outside it is refused as StartTemperatureNotLiquidWater. Compared on starts of {}: the first thermocouple's reading at the fire signal on the opened pulses compared at 2.0 printed dollars and above, rounded outward to the hundredth of a kelvin. Core 215 was compared from 24.0029 to 27.9375 degC (pulses 216 and 206), core 217 from 21.3171 to 29.2207 (222 and 240) and core 231 only from 20.9023 to 22.8024 (331 and 340); the dependence on the starting temperature was not tested apart. Outside those starts, on any of the three tables, the card computes the same equations with nothing compared",
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
                "0 while the rods are held, 1 from the segment in which they are dropped (the scram, 8 dollars removed at once from the first step of that segment). The card does not decide when the rods drop: the caller does, at any time after the fire signal, or never. In the records the rods dropped when the thermocouple read about 260 degC (2014 and 2015) or about 290 degC (2018), or about 7.3 s after the peak of a weaker pulse; the validation took the interval from each record and dropped the rods that long after the card's own peak. On the compared pulses the rods dropped 1.51 to 7.45 s after the peak (1.510 to 7.446 s, pulses 219 and 330); an earlier scram was not compared. No compared scram came while the transient rod was moving; the 8 dollars of the scram is an assumption. A scram while the rod moves takes 8 dollars from its ramp, which goes on to the rod's stop. The longest compared hold was 7.45 s after the peak; a 3.0 dollar pulse with the rods held reaches 600 K after 5.2 to 6.4 s on cores 215 and 231. Any other value is refused as ScramValueOutsideCard; 0 after 1 as RodsRaisedAfterScram".to_string(),
            ),
        ],
        floor,
        "a locked 1e-9 s: the card's own constant, a step that still moves the clock at the end of the longest run, a million steps at the ceiling, 93.9 s",
        gate,
        "0.2 of the time constant of the prompt pole after the scram, the fitted generation time over the delayed fraction times nine; the same ceiling before the scram. The feedback after a pulse takes the net reactivity after the scram below -8 dollars: to -9.08 dollars on the compared runs, where a step at this ceiling advances 0.224 of the time constant of that pole, and on any run the card computes to -9.87 dollars (4.16 dollars on core 231 from 373.124 K, the rods dropped at the hottest fuel, 8.1 s after the fire signal), where it advances 0.241. Past the compared span the step itself also holds a ceiling that follows the state: a step longer than 0.25 of the fitted generation time over the delayed fraction times the distance of the net reactivity from one dollar, at the step's start or where it lands, is refused as DtAboveStateCeiling. No run from a start this door takes meets it at or under this ceiling. In the rise of a pulse stronger than any compared a step is taken in pieces of at most 0.03 of the time constant of the growing pole, and after the scram in pieces of 0.025 of the prompt time constant, each with its own ledger integral",
        None,
        format!(
            "start_temperature_k is in kelvin. The reactor starts at {} MW at zero net reactivity, with the fuel at the starting temperature, at the fire signal, and each precursor sum at its equilibrium with that power. The start is not stationary: the six printed group fractions sum to 0.00699891 and the power equation uses the delayed fraction 0.00700, so at zero net reactivity the power falls at 0.037 per second, as if 1.6e-4 dollar were taken out. FLAG_ROD_MOVING is set while the inserted reactivity is still below the load; the rod stops at reactivity_inserted_dollar / 52 seconds. A run is one pulse: a segment holds one reactivity_inserted_dollar, rods_dropped 0 while the rods are held, and the first segment with rods_dropped 1 is the scram, at any time after the fire signal, also while the rod moves, or never. A run that never scrams is no longer refused 7.45 s after its peak: the held pulse is computed on, and its power settles where the feedback and the heat flow to the water balance. A run is capped at 1,000,000 steps, 93.9 s at the ceiling. fuel_temperature_k is the core-average fuel temperature: the energy released, less the heat that has left to the water, over the heat capacity of the fuel. It is not the temperature of any one place in the fuel, and there is no thermocouple on the card. peak_fuel_temperature_k is its highest landed sample. energy_at_scram_mws is the energy released at the instant the rods dropped, and is 0 until then. peak_power_mw, peak_energy_mws and peak_time_s are the highest landed sample of the power, which lies within one step of the true maximum; the card does not interpolate. power_mw is the mean of three calibrations of the institute's pulse channel, one for each core's sessions; they differ by up to 7 percent. A single step takes the starting temperature and the rod table and starts again from there at each call, so single steps cannot be chained. The records compared run to {} s after the fire signal; a run goes on past it. The largest energy released in a compared run is {} MW s, the largest compared peak power {} MW and the highest compared core-average fuel temperature {} K, the fuel temperature compared only through one thermocouple of one element, through the hot-rod factor 1.387; past them the card computes the same equations, the quadratic feedback carried past the 294 to about 585 K it was fitted on. At the rod's whole worth the peak power is 2280 to 2700 MW and a held pulse releases 41 to 45 MW s in 93.9 s. The hottest core-average fuel any run reaches is 769.7 K (496.6 degC), at 4.16 dollars on core 231 from 373.124 K with the rods held, 8.1 s after the fire signal, 923 K (650 degC) in the hottest element through the hot-rod factor: far below 1150 degC, the limit of U-ZrH1.6 fuel in stainless clad. The card still holds that limit on the core-average fuel temperature, the one it carries: above 1423.15 K, in a state handed to its step or where a step would land, it is refused as FuelTemperatureAboveFuelLimit; no run from a start this door takes reaches it. Held on, that pulse settles at 636.6 K, where the feedback takes back the whole insertion. Past the compared span, with 3.25 printed dollars and more and the rods dropped in the rise of the pulse or at its peak, the error a step at the ceiling makes in the rise is carried by a population the scram leaves tens of times smaller, and a run at the ceiling can be stopped by the ledger more than 75 s after the fire signal, as SpontaneousGeneration or FirstLawBreach; over 14 s the largest residual found there is 3.2e-8 against the limit of 2e-7, and at half the ceiling every such run tried lands to 93.9 s. NegativeEnergy, ImpossiblePeak, ImpossibleState, FuelTemperatureAboveFuelLimit and DtAboveStateCeiling are refusals of the card's step for a state handed to it that the card did not write; this door takes the starting temperature and the rod table and nothing else of a state, and holds the step to the ceiling, so no request reaches them. A step of zero or less is refused as NonPositiveDt before the step floor is tested.",
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
                    "from -70 to 70 arcsec, both inclusive: the lock point on x in the sensor's ideal frame, where the guide star is held. It is not in the file and is the caller's. The guider's field is 2.3 by 2.3 arcmin, 69 arcsec either side of its centre; a lock point beyond 70 arcsec is refused as LockOutsideSensorField, since no star can be held there. The lock points of the files the comparison answered span {} to {} arcsec, fitted and held-back together, and the held-back files alone -61.1512947 to 49.05617, where all five marks were met on both axes; a lock point between that span and the field's edge is computed and was not compared. The arithmetic only takes the lock point away from the centroid. The comparison took the median of the good centroids of the whole run, which uses centroids later than the frame being predicted. With a lock point that uses no future, the median of the first 10 s, the second scoring found the last-centroid, variance, correlation and per-file marks still met on quiet frames (0.793 and 0.821, 0.999 and 1.023, 0.071 and 0.095, 100 percent), and the lock-point mark no longer a test; with that lock point more frames lie beyond 5 mas, and more carry FLAG_POINTING_DISTURBED",
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
                    "from -70 to 70 arcsec, both inclusive: the lock point on y in the sensor's ideal frame; beyond it, outside the guider's 2.3 by 2.3 arcmin field, it is refused as LockOutsideSensorField. The lock points of the files the comparison answered span {} to {} arcsec, fitted and held-back together, and the held-back files alone -62.0922546 to 50.1812668; a lock point between that span and the field's edge is computed and was not compared",
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
                    "0 s or more: the time from the last centroid before this run that was more than {} mas from the lock point or more than four stated standard deviations from where it was expected, to the first frame of this run. The card cannot see a centroid that came before its run, so the caller states this; it cannot confirm it. Where no earlier centroid is known, as at the first frame of a fine-guide file, state {} or more. It starts the state's since_disturbance_s, and the frames that end less than {} s, 469 frames, after such a centroid carry FLAG_RECOVERING: the protocol's 30 s after an event, where on the fitted files the x axis missed four of its five marks and on the held-back files three. The comparison with {} s carries an allowance of 2^-36 s, the most a difference of two times under 36 hours, each read from a decimal, can differ from the exact difference. Below zero it is refused as QuietBeforeStartBelowZero; through engine 0.9 a time under {} s was refused as RecoveryNotElapsed",
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
                "0 or 1 and nothing between: 1 for a frame with a good centroid (centroid_time above 0 and bad_centroid_dq_flag GOOD in the table FGS CENTROID PACKET), 0 for a frame with none. A frame with measured 0 is predicted and not updated, and its stated variance grows toward its unmeasured value, 1.52574 mas squared over both axes, for any number of such frames in a row. The longest run of them a compared run crossed was 10 frames on the fitted files and 2 on the held-back files; through engine 0.9 an eleventh was refused as MeasurementGapOutsideCard".into(),
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
                    "guide_star_instrument_counts_per_sec of the table, unchanged. On a measured frame any count rate above zero: the noise variance of a centroid is the power law r_ref x (counts / 1e6)^(-r_power), which has no value at zero, and a count rate of zero or below, or under about 7e-237 counts per second where the y axis's variance passes the largest 64-bit number, is refused as CountRateNotPositive. The noise law was fitted on count rates from 22,910 to 2,813,637 counts per second, and every held-back file lay inside them, from 62,000 to 816,000; outside {} to {} a measured frame lands with FLAG_COUNT_RATE_BEYOND_COMPARED, the noise law carried on past where it was fitted. On the held-back guide stars the y noise exponent came out 1.045 against the card's 1.273, so the y variance is the less certain the farther the count rate is from 1,000,000. {LOS_UNMEASURED_RULE}",
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
        "One segment of one step for each frame of the table, in order, with the table's own numbers as the load; the validation used dt_s 0.064. A run starts at time zero at the lock point given, with nothing known: every part at zero and each covariance at its unmeasured value. A single step starts in the same way at each call, so single steps cannot be chained; a run carries the filter from frame to frame. A frame the request itself shows to be malformed or off the card, a count rate of zero or below on a measured frame, a centroid more than 250 mas from the lock point, a measured that is neither 0 nor 1, or numbers on a frame with measured 0, refuses the whole run before its first step. Where the card was measured worse. The card describes quiet fine guide. When the pointing is disturbed the offset jumps by many mas and rings down, the filter follows slowly, and there its prediction is worse than the last centroid and its stated variance too small: inside disturbances on the held-back files its error one frame ahead was 2.03 mas against 1.00 for the last centroid on x and 1.57 against 0.78 on y, with the stated variance 12 to 17 times too small; over all 973,388 measured held-back frames, quiet and disturbed together, it was 1.35 on x and 1.26 on y of the last centroid's error, with a squared error over the stated variance of 3.9 and 3.0. The card answers those frames, with the filter's equations and its ledger closed, and says so with three flags that seal: FLAG_POINTING_DISTURBED on a measured centroid more than 5 mas from the lock point on x or y, the protocol's event sample; FLAG_BEYOND_FOUR_SIGMA on a measured centroid more than four stated standard deviations from where it was expected, which also comes by chance where the stated variance is right, once in about 7,900 measured frames, 8.4 minutes, and on the held-back files came 21 times as often on x; and FLAG_RECOVERING on a frame that ends less than 30 s after the last centroid that carried either, counted from quiet_before_start_s at the start, where on the held-back files the x axis missed three of the protocol's five marks. Through engine 0.9 the first two ended the run as PointingDisturbance and InnovationBeyondFourSigma, and a run starting within 30 s was refused as RecoveryNotElapsed. A measured frame whose count rate is outside 22,900 to 2,820,000 counts per second, the count rates the noise law was fitted on, lands with FLAG_COUNT_RATE_BEYOND_COMPARED, which seals; through engine 0.11 such a frame was refused as CountRateOutsideCard. Each frame's slots carry where the centroid was expected before the frame was used and the variance stated for the difference, so the comparison with the centroid is the caller's: (centroid - lock) x 1000 - predicted, in mas, and its square over the variance, which averages 1 where the stated variance is right. The final state carries the sums nis_sum_x and nis_sum_y over measured_frames, and since_disturbance_s. The lock point is held to the guider's field, 70 arcsec from its centre on x and y; one beyond it is refused as LockOutsideSensorField before the first step. The comparison answered lock points from -61.1512947 to 60.4564934 arcsec on x and from -64.2767 to 50.1812668 arcsec on y, the fitted and held-back files together; a lock point nearer the field's edge is computed and was not compared. A frame with measured 0 is predicted for any number of frames in a row; the longest run of such frames a compared run crossed was 10 frames on the fitted files and 2 on the held-back files. The state counts them as unmeasured_in_row, and a measured frame sets it to zero. The card has no limit in time of its own; a run records at most 10,000 frames. A step of zero or less is refused as NonPositiveDt before the step floor is tested.".into(),
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
                "FLAG_NO_MEASUREMENT, bit 0: the table had no good centroid for the frame, and it was predicted only; it marks no failure of the model. FLAG_POINTING_DISTURBED, bit 1: the measured centroid is more than 5 mas from the lock point. FLAG_BEYOND_FOUR_SIGMA, bit 2: it is more than four stated standard deviations from where it was expected. FLAG_RECOVERING, bit 3: the frame ends less than 30 s after the last centroid that carried bit 1 or 2. FLAG_COUNT_RATE_BEYOND_COMPARED, bit 4: the measured frame's count rate is outside 22,900 to 2,820,000 counts per second, the count rates the noise law was fitted on; the state holds no boolean of its own for it. Each seals.",
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
const CANTILEVER_ENVELOPE_RULE: &str = "The deflection and the speed together must keep the envelope, sqrt(deflection_m^2 + (velocity_m_s / omega0)^2) with omega0 = 2 pi x 8.29235 Hz, at or below 0.01875 m, the largest amplitude the softening and the damping were fitted on (3.119 V x 0.006012 m/V); above it those laws were not fitted, and the strip's own limit, its yield at the clamp, needs its alloy, which the dataset does not state. The envelope is a size: a deflection or a speed of either sign past it is refused as EnvelopeAboveCard. Above 0.01579 m, the largest envelope the card held on the run compared with the held-back ring-down, the card answers with the same equations and ledger, and a frame there carries FLAG_ENVELOPE_ABOVE_COMPARED; through engine 0.11 such a start was refused as EnvelopeAboveCard. Below 5.8e-5 m, the smallest envelope of the compared run, the card goes on to rest with the same equations and ledger, and a frame there carries FLAG_ENVELOPE_BELOW_COMPARED";

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
        "0.25 ms, about 482 steps per period, a locked constant set by measurement and not by a formula. The damping term has a sharp corner at zero deflection, which the beam crosses twice a period, so the error falls more slowly with the step than fourth-order Runge-Kutta usually gives. On the card's map over the 24 s of the compared run the largest conservation residual at this step is 5.6e-8 against a limit of 1e-6, and through the card's own step from the start of the scored ring-down to the compared floor, 5.8e-5 m, it is 7.4e-8, and over 200 s from 0.1 mm to 4.6e-13 m, 4.6e-12. Ten steps make one frame at 400 per second, the rate at which the reading procedure of the validation protocol was confirmed",
        None,
        "No load: the beam rings free, and a run segment carries an empty load object. A run starts at time zero with its ledger anchored at the deflection and the speed given. A single step starts in the same way at each call: its deflection_m and velocity_m_s handed back as a new start continue the same motion, and t_s and the heat begin again at zero. The envelope range compared with the held-back ring-down, in numbers: the compared run started 0.150 s before the start of the ring-down, on the model's own trajectory, at an envelope of 15.786 mm, and its readings were scored from the start crossing, 0.14873 s into the run, to the end of the last scored block, 23.469 s after it; over the run the card's envelope fell from 15.786 mm at its start, the largest it held, through 14.404 mm, at the first turning point after the start crossing, to 0.0587 mm. There the amplitude error was 7.03 percent against a line of 5 percent, missed, and the frequency error 0.0031 Hz against 0.010 Hz and the timing error 0.021 of a period, met. The largest envelope compared, rounded up, is 0.01579 m. The card's ceiling is 0.01875 m: 3.119 V, the largest amplitude of the fitted recording, times 0.006012 m per volt, 18.7514 mm, rounded down; the softening and the damping are power laws fitted on amplitudes up to it, above it those laws were not fitted, and the strip's own limit, its yield at the clamp, needs its alloy, which the dataset does not state, so above it a start and a step are refused as EnvelopeAboveCard. From 0.01579 m to 0.01875 m the card answers with the same equations and the same ledger, and every frame there carries FLAG_ENVELOPE_ABOVE_COMPARED: the fitted laws, inside the amplitudes they were fitted on and above the largest envelope compared; through engine 0.11 such a start or step was refused as EnvelopeAboveCard. From rest at 0.01875 m the ring-down lands with 1,149 frames flagged, the last at 0.317 s, its largest conservation residual 1.04e-7 against a limit of 1e-6. Below 0.0587 mm, rounded down to 5.8e-5 m, the card goes on toward rest with the same equations and the same ledger, and every frame there carries FLAG_ENVELOPE_BELOW_COMPARED: the fitted damping and the 8.29235 Hz frequency at vanishing amplitude are carried on below where they were compared, and the lowest frequency read on the beam was 8.22 Hz. Through engine 0.9 such a step was refused as DeflectionBelowCard. The compared run lands 94,731 steps inside the compared envelope. From its state after 595 steps of 0.25 ms, at 0.14875 s, deflection -0.00104914043915228 m and speed 0.7408030982848752 m/s, the card gives the same deflections and speeds bit for bit, and it lands 94,136 steps, to 23.534 s, inside the compared envelope, and goes on below it. A free decay only shrinks and ends only when the request's steps end; rest at zero is a start that does not move. The envelope is measured with the frequency at vanishing amplitude and the softened spring is weaker, so within one period the envelope is smallest at the rest position and rises toward the turning point, by about 1 percent at the largest swing: a start at zero deflection with its envelope within 1 percent of the ceiling is accepted and is refused as EnvelopeAboveCard a little under a quarter of a period later, and that run keeps its proof up to the last step that landed. With the envelope at 0.99 of the ceiling the step is refused after 107 steps of 0.25 ms; at 0.98 the beam rings through. For the same reason, as a ring-down passes 0.01579 m, frames with FLAG_ENVELOPE_ABOVE_COMPARED and frames without it alternate within a period. The worked examples start at that state after 595 steps, the start of the scored ring-down. A step of zero or less is refused as NonPositiveDt before the step floor is tested.".into(),
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
                "Margin to the largest envelope compared, in metres: 0.01579 − envelope_m, below zero on a frame with FLAG_ENVELOPE_ABOVE_COMPARED. Through engine 0.11 0.01579 m was the card's ceiling; the ceiling is now 0.01875 m.",
            ),
            ("residual", "Conservation residual."),
            (
                "flags",
                "FLAG_ENVELOPE_BELOW_COMPARED, bit 0: the envelope is below 5.8e-5 m, the smallest the card was compared at on the held-back ring-down. FLAG_ENVELOPE_ABOVE_COMPARED, bit 1: the envelope is above 0.01579 m, the largest the card held on the compared run, and at or below 0.01875 m, the largest amplitude the constants were fitted on; the state holds no boolean of its own for it. Each seals. An envelope above the ceiling, 0.01875 m, and a residual past the limit are refusals.",
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
                "from 44.32325388 dB up, inclusive: the sent level minus the received level with no rain and dry antennas. It is the caller's, read from the link before the rain, as the median over the hour before the first wet minute; the card holds it through the run and does not fit it. 44.32 dB is the free-space loss of this path, 124.667 dB, less the gain of two ideal 0.3 m apertures at this frequency, 40.172 dB each: a lower level is a gain no pair of these antennas can have, and is refused as DryPathLossBelowFreeSpace. Rain and antenna attenuation add to the dry level in dB, and the ledger holds at any level. The 18 fitted events lay between 51.75 and 52.55 dB, the monthly median moved 0.6 dB from June to July, and the nine held-back events of August, which met all three lines, lay between 52.60 and 52.90 dB".to_string(),
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
                "0 mm/h or more: the rain rate at the Barlastplatsen gauge, held through the step. Below zero it is refused as NegativeRainRate. Any rate from zero up is computed: the rain term is ITU-R P.838-3, k R^alpha L, at any rate, the water on the antennas builds toward 2.13 dB at any rate above zero, and the map and the power ledger are exact at any rate. Fitted on the one-minute rates of June and July 2015, 0 or 6 to 84 mm/h; compared with the nine rain events of August 2015, whose rain reached 54 mm/h, where all three lines were met: RMS error 0.581 dB against a line of 0.90, 0.585 of the error of P.838 alone against 0.80, and 1.069 of the measured summed attenuation, inside 0.75 to 1.25; in light rain under 2 mm/h the card was about 0.3 dB high. No rate above 84 mm/h was measured at this gauge. The gauge counts 0.1 mm, so every rate the card was fitted and compared on is 0 or at least 6 mm/h; a rate between them is computed with the antenna water fitted only at rates of 6 mm/h and up. The frame's force_torque, 84 - rain_rate_mm_h, is below zero above the fitted rates".to_string(),
            ),
        ],
        floor,
        "the gauge's minute, 60 s, divided by 2^20: a step that moves the water on the antennas by at most seven parts in ten million of its ceiling. Under it a step is mostly rounding",
        gate,
        "364791569.8170102 s, about 11.6 years, set by the step's arithmetic and not by the data. The map of the water on the antennas is exact at any step, and each energy is its power integrated exactly over the step, so one step and many steps over the same span give the same state and the same energies to rounding, within one part in 10^13. The one number that loses figures as the step grows is the water built in a wet step, a difference of two numbers near the antennas' most water times the step; its rounding, one part in 2^52 of the step over the 81 s wetting time, is a thousandth of the antenna account's limit of one part in a million at this ceiling. The three antenna constants were fitted at the gauge's 60 s step, and the held-back month was run at 60 s",
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

const CELL_2170_MODEL3_CAPACITY_RULE: &str = "Read it from a recharge record: capacity_ah = ratio x (ampere-hours put in by the full recharge) / (1 - state of charge before that recharge). A full recharge is constant current, then constant voltage at 4.2 V. The ratio is the cell's charge-return ratio, the charge delivered divided by the charge put back at the cell's first capacity test and the recharge after it; it is listed for each cell under cell_number. The state of charge before the recharge is the value at which the voltage after a rest of an hour or more equals the card's rest voltage, which a step at zero current from that state of charge returns as terminal_v. The reading is good only when that state of charge is below 0.08, where the curve is steep; after a shallower discharge keep the last good reading, or the straight line in time between the readings either side";

fn cell_2170_model3_card() -> Card {
    let specimen = cell_2170_model3::Cell2170Model3::cell();
    // No pole on the card, so no `π/ω`. The floor and the ceiling are locked
    // constants of the card.
    let gate = specimen.gate_dt_s().expect("specimen gate");
    let floor = specimen.floor_dt_s().expect("specimen floor");
    let rows = cell_2170_model3::CELLS;
    let numbers = rows.iter().map(|row| row.cell_number);
    let least = numbers.clone().fold(f64::INFINITY, f64::min);
    let most = numbers.fold(f64::NEG_INFINITY, f64::max);
    let per_cell = rows
        .iter()
        .map(|row| {
            format!(
                "cell {}: capacity at its first capacity test {} Ah, charge-return ratio {}, resistance scale {}",
                row.name,
                crate::message_number(row.reference_capacity_ah),
                crate::message_number(row.charge_return_ratio),
                crate::message_number(row.resistance_scale)
            )
        })
        .collect::<Vec<_>>()
        .join("; ");
    finish(
        "energy",
        "cell_2170_model3",
        "One Tesla Model 3 2170 lithium-ion cell, any of the three cells m80, m1000 and m448-N of its dataset, as a physical model with memory, under a terminal current and a measured case temperature. Compared from -10 to 40 degC on runs of two cells no model had seen: m448-N 51 of 60, m1000 31 of 45; on m80, whose runs it was built on and not a blind test, 62 of 65, and at -20 degC none of m80's 13. The table card cell_2170_m80 reads 17.4 mV higher on one 1.5 A discharge at 12 degC.",
        vec![
            scalar(
                "cell_number",
                Role::Phase,
                "1",
                Some(least),
                false,
                Some(most),
                false,
                format!(
                    "exactly one of {}: the number in the dataset's name for the cell. It picks the numbers that belong to one cell; the equations, the curves and the 22 fitted numbers are the cell type's. {per_cell}. The scale multiplies the three resistances and divides the two exchange currents; the ratio is for reading capacity_ah from a recharge and the step does not use it. Any other value is refused as CellNotRecognized; nothing is rounded to a cell",
                    crate::cell_2170_model3_cells_in_words()
                ),
            ),
            scalar(
                "state_of_charge",
                Role::Phase,
                "1",
                Some(cell_2170_model3::CURVE_FIRST_STATE_OF_CHARGE),
                false,
                Some(cell_2170_model3::CURVE_LAST_STATE_OF_CHARGE),
                false,
                "inclusive [-0.05, 1.05], the ends of the measured rest-voltage curves (StateOfChargeOutsideCard): 1 after a full recharge, and 1 plus the top-up in ampere-hours divided by capacity_ah when the tester topped the cell up afterwards. Compared from 0 to 1.02; the largest start in the tests was 1.012, and a state of charge outside 0 to 1.02 sets FLAG_OUTSIDE_COMPARED_RUNS. The cell is taken as rested: every memory starts at zero. A run lands a step whose state of charge ends within the rounding allowance of the running sum past either end, STATE_OF_CHARGE_ALLOWANCE_PER_STEP, 2.22e-16 for each step of the run so far and two more".into(),
            ),
            scalar(
                "capacity_ah",
                Role::Phase,
                "Ah",
                Some(0.0),
                true,
                None,
                false,
                format!(
                    "above 0 Ah (CapacityNotPositive): the capacity from the last full charge, which falls as the cell ages. Compared on 4.16 to 4.7 Ah: the two cells had 4.174 to 4.689 Ah in the scored tests, and the range allows 0.01 Ah beyond that; outside it sets FLAG_OUTSIDE_COMPARED_RUNS. The resistances carry no age term. It is the caller's number and the card never changes it. Each block of tests was measured at one age of the cell, so the capacity was compared only in the band each block was scored with, at the case temperatures that block spanned: 4.16 to 4.26 Ah from -12.9 to 12.6 degC, 4.20 to 4.30 Ah from 0.0 to 20.4 degC, 4.17 to 4.30 Ah from 9.8 to 26.8 degC, 4.35 to 4.53 Ah from 23.3 to 31.4 degC and 4.56 to 4.70 Ah from 28.1 to 45.9 degC. These are the capacities of every run scored from -10 to 40 degC on the three cells and the block capacities of their fitting files, with 0.01 Ah beyond them, and the case temperatures the sensor read in those runs, with the chamber's 0.3 K beyond them. A capacity and a case temperature that no one block holds were never compared, a new cell in the cold or an aged cell when warm, and set FLAG_OUTSIDE_COMPARED_RUNS; through engine 0.9.0 they were refused as CapacityOutsideCardAtTemperature. {CELL_2170_MODEL3_CAPACITY_RULE}"
                ),
            ),
            scalar(
                "case_temperature_least_c",
                Role::Phase,
                "degC",
                Some(-cell_2170_model3::KELVIN_OFFSET),
                true,
                None,
                false,
                "above -273.15 degC, absolute zero (CaseTemperatureBelowAbsoluteZero), required: the least case temperature of the run so far. A span of case temperature past 19.2 K since the rested start, the scored 18.811 K and the chamber's 0.3 K, sets FLAG_OUTSIDE_COMPARED_RUNS, and that span is accumulated over the steps of one request: a run starts rested and states case_temperature_least_c equal to its first segment's case_temperature_c, and a single step equal to its case_temperature_c. A start without it is refused. A value above case_temperature_most_c is refused as CaseTemperatureSpanInverted; through engine 0.9.0 it was named CaseTemperatureSpanAboveCard".into(),
            ),
            scalar(
                "case_temperature_most_c",
                Role::Phase,
                "degC",
                Some(-cell_2170_model3::KELVIN_OFFSET),
                true,
                None,
                false,
                "above -273.15 degC, required: the most case temperature of the run so far: a run starts rested and states it equal to its first segment's case_temperature_c, and a single step equal to its case_temperature_c. A start without it is refused. A step that takes the span of the run past 19.2 K lands with FLAG_OUTSIDE_COMPARED_RUNS".into(),
            ),
            scalar(
                "current_a",
                Role::Load,
                "A",
                None,
                false,
                None,
                false,
                "any current: each line of the card is the exact solution for a held current at any size, and a large one ends at a voltage limit. A positive current discharges the cell and a negative current charges it: the opposite sign to the measurements and to the cell_2170_m80 card. Compared from -13.984 to 13.524 A: 13.524 A is the tester's programmed pulse of 13.5 A plus 0.024 A, the accuracy the dataset states for the tester's current, and the pulse reads up to 13.506 A in the logs; 13.984 A is the largest charging current in a scored drive cycle, 13.960 A on cell m1000, plus the same 0.024 A; on cell m80 the largest was 9.74 A. A current outside sets FLAG_OUTSIDE_COMPARED_RUNS; not scored on any charging run".into(),
            ),
            scalar(
                "case_temperature_c",
                Role::Load,
                "degC",
                Some(-cell_2170_model3::KELVIN_OFFSET),
                true,
                None,
                false,
                "above -273.15 degC, absolute zero, where the Arrhenius laws have no value (CaseTemperatureBelowAbsoluteZero): the temperature measured on the cell's case at the end of the step. The card is given it and does not predict it. Compared from -12.9 to 45.9 degC: with the chamber set from -10 to 40 degC the sensor read from -12.59 to 45.54 degC in the scored tests, and 0.3 K beyond that is the chamber's stated accuracy; m80 met 62 of 65 runs, m448-N 51 of 60, m1000 31 of 45. Fitted without its -10 degC tests the model met 3 of 13 there, so the temperature laws are not shown to reach colder, and at -20 degC it met none of m80's 13 runs. Physical bounds of the cell, its electrolyte freezing or the maker's operating range, are not on the card. It is paired with capacity_ah, as that field states. In the scored runs the case temperature changed by at most 5.023 K between two consecutive seconds and spanned at most 18.811 K inside one run; a step past 5.4 K or 19.2 K, each the measured value with 0.3 K beyond it, sets FLAG_OUTSIDE_COMPARED_RUNS. Outside -12.9 to 45.9 degC it sets the same flag".into(),
            ),
        ],
        floor,
        "a locked 1 s, the ceiling itself: every fit and every score was made at one second, and measurements logged once a second cannot test a shorter step. For a held 4.5 A discharge at 25 degC, the one load compared, a step of 0.1 s gave the one-second answer to within 0.3 mV, but where the current reverses it does not. When the surface reaction's overpotential would change sign inside a step the step is taken again from zero with the other law, so the answer depends on the step: from 4.5 A to -4.5 A at 0 degC the 0.1 s answer differs from the one-second answer by 49 mV in the second of the reversal, 5 mV in the next and 0.3 mV in the one after, and over the first 6,000 seconds of a drive cycle of cell m448-N at -10 degC by up to 32 mV. A shorter step can come back only after that sign change is split at the zero crossing and the card is fitted and scored again",
        gate,
        "a locked 1 s, the sampling of the measurements every fit and every score was made on. A load that changes within a second is not represented by a longer step, and the constants were fitted with the one-second rule for the reaction changing sign",
        None,
        "Over a step the current and the case temperature are held. The range compared with measurement is a range, not a wall: through engine 0.9.0 a capacity, a case temperature or a current outside it, a capacity not compared at a segment's case temperature, a jump of case temperature past 5.4 K, a span past 19.2 K, a running mean past 8.722 A discharging or 2.993 A charging and a state of charge past 1.02 were refusals, and now each step there lands with FLAG_OUTSIDE_COMPARED_RUNS, its every line the card's own equation and its ledger closed. The same cell type is served as tables, cell_2170_m80; on one discharge, cell m80 from the recharge of its 10 degC block, 4.2661 Ah, at 1.5 A for 600 s with the case at 12 degC, that card gives 3.934 V and this one 3.917 V, 17.4 mV apart. A case temperature at or below absolute zero in any segment of a run refuses the whole run before its first step. The change of case temperature counts from the start: the first step has no last step to differ from. The span counts from the rested start and is accumulated over the steps of one request: a run, or a single step, starts rested and states case_temperature_least_c and case_temperature_most_c both equal to its first case temperature, and a start without them is refused. A start from a returned state is not served on this card: through from it is refused as ContinuationNotOnCard, and without from a pair other than a fresh start's is refused as ContinuationWithoutUpstream, because the card's other memories are not start fields and would start again at zero, among them the 60 s running mean of the current and the last case temperature the 5.4 K change is counted from. A run starts from a rested cell: the state of charge and the capacity as given, the least and most case temperature as given, every other memory at zero, and the ledger anchored there. A rest of an hour is enough from about -7 degC up. Colder, the slowest diffusion mode, 16 minutes at -10 degC, 21 at -12.9 and 38 at -20, needs five of its times to fall under 1 percent: 81, 103 and 189 minutes. At -12.9 degC an hour after a 1C discharge of 1400 s leaves the surface 0.85 percent of capacity below the bulk and the terminal 8.8 mV under its rested voltage; a cell that has not rested is not in that state, and the run should start earlier and go through the load that came before. The first measured sample of a record is the start and each later sample is one step, so a logged drive cycle is one run with a segment of one step for each second. A single step starts from rest in the same way at each call, so single steps cannot be chained into a run. A run ends in a sealed refusal when the terminal voltage would fall below 2.8 V (UndervoltageLimit) or rise above 4.242 V (OvervoltageLimit), when the state of charge would leave the measured curves, -0.05 to 1.05, by more than the rounding allowance of its running sum (StateOfChargeOutsideCard), or when the state of charge at the particle surface would leave them (SurfaceOutsideCurve); the last landed state is the last step on the card. The sustained currents compared, 8.722 A discharging and 2.993 A charging, are the largest that the 60 s running mean reaches in any scored test, 8.698 A and 2.969 A in the hill cycle of cell m80, plus 0.024 A: the hill cycle is the only test of sustained current above the rated current of about 4.5 A, and the model's largest misses are in it; at 40 and 25 degC the card stops at 2.8 V seconds to minutes before the cell, and at the end of the steepest climbs at 0 and -10 degC it reads 130 to 180 mV high. Above a running mean of 4.6 A discharging a frame carries FLAG_SUSTAINED_ABOVE_1C and the run goes on. Charging current is accepted: it was fitted inside the drive cycles of cell m80 and in a slow charge and scored inside the drive cycles of cell m1000, which charge harder, and no charging run was scored. A step of zero or less is refused as NonPositiveDt before the step floor is tested.".into(),
        &[
            "NonFiniteInput",
            "NonFiniteState",
            "NonPositiveDt",
            "InvalidCell",
            "CellNotRecognized",
            "CapacityNotPositive",
            "CaseTemperatureBelowAbsoluteZero",
            "CaseTemperatureSpanInverted",
            "StateOfChargeOutsideCard",
            "SurfaceOutsideCurve",
            "UndervoltageLimit",
            "OvervoltageLimit",
            "SpontaneousGeneration",
            "FirstLawBreach",
        ],
        vec![
            Advisory {
                name: "FLAG_SUSTAINED_ABOVE_1C",
                mask: cell_2170_model3::FLAG_SUSTAINED_ABOVE_1C,
                seals: true,
            },
            Advisory {
                name: "FLAG_OUTSIDE_COMPARED_RUNS",
                mask: cell_2170_model3::FLAG_OUTSIDE_COMPARED_RUNS,
                seals: true,
            },
        ],
        slots(&[
            ("t", "Time since the anchor, in seconds."),
            ("pos[0]", "terminal_v"),
            ("pos[1]", "charge_removed_ah"),
            ("pos[2]", "surface_state_of_charge"),
            ("vel[0]", "reaction_overpotential_v"),
            ("vel[1]", "hysteresis_v"),
            ("vel[2]", "electrolyte_v"),
            (
                "force_torque",
                "Running mean of the current with a 60 s time constant, in amperes, positive discharging.",
            ),
            ("residual", "Conservation residual."),
            (
                "flags",
                "FLAG_SUSTAINED_ABOVE_1C: the running mean of the current is above 4.6 A, discharging. Only the hill cycle of the tests is in this region, and below 10 degC the card is known to read high in it. FLAG_OUTSIDE_COMPARED_RUNS: the step lies outside the range compared with measurement. Both seal.",
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
                "from 0 s, inclusive: the time the slip of the first step has already been held at the start. A time below zero is no time a slip can have been held and is refused as SlipHeldBelowZero. The measured force rises over the first seconds after a slip is set, and the card's force law has no rise: it gives the steady force from the instant the slip is set, and there it was compared and missed, so a step that starts before 5 s lands with FLAG_SLIP_SETTLING; through engine 0.11 it was refused as SlipNotSettled. From 5 s to 21 s after a slip was set it was compared and met, and past 21 s a frame carries FLAG_BEYOND_COMPARED. A run carries it from step to step while the slip is held, and its final state returns it; to go on from a returned state, hand back its slip_held_s with its distance_m. The card cannot see a hold that came before its run, so the caller states this; it cannot confirm it, and a caller who states a time that is not so gets answers the comparison does not cover".into(),
            ),
            scalar(
                "normal_load_n",
                Role::Load,
                "N",
                Some(0.0),
                true,
                None,
                false,
                "above 0 N: the normal load on the wheel. A load of zero or less is no contact and is refused as NoContact. The constants were fitted at 3500 and 5000 N and compared at 6500 N, held back: above 5000 N a frame carries FLAG_LOAD_ABOVE_FITTED, and below 3500 N or above 6500 N, where nothing was measured, FLAG_BEYOND_COMPARED as well".into(),
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
        "2^22 s, 4,194,304 s, from the step's arithmetic. The force has no dynamics, so every work of a step is exact at any size. Below 2^42 s the spacing of 64-bit numbers is at most 2^-11 s, half the floor, so a floor step still moves the clock, the time held and the distance there; a run of 2^20 steps, more than the 1,000,000 one request takes, at the ceiling ends at 2^42 s. A step that a sum would not move is refused as StepTooSmallToResolve. Through engine 0.9 the ceiling was 16 s, the compared hold from 5 s to 21 s",
        None,
        "Over a step the normal load, the pressure, the slip and the carriage speed are held. The force is the same at every step of a held load: sign(slip) x MU x (|slip| / 0.10)^-0.132823 x normal_load_n, with MU 0.228109 in traction and 0.24162 in braking, all three numbers fitted to this tyre's own runs. The registered comparison scored each slip stage on the window from 10 s to 1 s before it ended. Run again on 2026-10-10 by the same cells and the same five lines on the time after each slip change, with the change placed where the wheel speed steps, the force ratio met all five lines in every one-second window from 5 s to 20 s, and every one-second window before 5 s failed a line: the first second missed the line of 0.045 at 6.5 kN, with 0.0477, and the comparator, with 0.0449 against a constant ratio's 0.0441 over the 60 held-back cells, and the seconds from 1 s to 5 s did no better than the constant ratio, 0.0430 against 0.0424 from 1 s to 2 s and 0.0399 against 0.0395 from 2 s to 5 s, where from 5 s to 11 s the card had 0.0365 and from 11 s to 20 s 0.0337, every line met. With the protocol's own stage cuts the window from 5 s to 11 s met all five lines as well. No scored slip was held longer than one stage of about 20 s, except the last slip of each run, 7 or 20 percent, which the rig held for 21 to 33 s. So a start states slip_held_s, the time the slip of the first step has already been held, from 0 s; a run carries it from step to step while the slip is held, and it starts again at zero when the slip changes. The measured force rises over the first seconds after a slip is set and the card's force law has no rise, so in those seconds the card gives the steady force where the measured force was still rising: a step that starts before 5 s, after a start that states less or after any change of slip, lands with the same force and the same ledger and carries FLAG_SLIP_SETTLING, and through engine 0.11 it was refused as SlipNotSettled. A time held below zero is refused as SlipHeldBelowZero. From 5 s the card holds the steady force for as long as the slip is held; past 21 s, the end of the compared hold, a frame carries FLAG_BEYOND_COMPARED, and through engine 0.9 such a step was refused. The rise of the force from rest is not modelled. A run starts at time zero at distance_m with the three works at zero. A single step starts in the same way at each call: its distance_m and its slip_held_s handed back as a new start continue along the track and along the hold, and the works begin again at zero. The card cannot see a hold that came before its run, so it cannot confirm a stated slip_held_s. A value outside its range in any segment of a run refuses the whole run before its first step. The end of the rig's track is not an end of the tyre: the force does not depend on where the wheel is, and a run rolls on past 6 m, where through engine 0.9 it ended. The ledger of the three works is held to 1e-9 of the larger of 10.3 kJ, the heaviest load measured at the highest friction ratio over 6 m, and the largest of the three works of the run, so it closes at the same relative size at every load and every distance.".to_string(),
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
                "FLAG_LOAD_ABOVE_FITTED: the normal load is above 5000 N, the largest load the constants were fitted on. FLAG_BEYOND_COMPARED: the frame is beyond what was compared with the measurements, a normal load below 3500 N or above 6500 N, an inflation pressure below 150000 Pa or above 350000 Pa, or a slip held longer than 21 s. FLAG_SLIP_SETTLING: the step starts less than 5 s after its slip was set, where the measured force was still rising and the card's steady force missed it; the state holds no boolean of its own for it. All three seal.",
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
                    "from 0 percent, inclusive, the moment the full load is applied, to {extension_max} percent, inclusive, at every stress. It is the extension since the full load was applied, over the reference length, as DIN EN ISO 204 defines the percentage creep extension; the elastic and the initial plastic extension of loading are not in it. A start below zero is refused as CreepExtensionBelowZero, and one above {extension_max} percent, the largest registered level, as CreepExtensionOutsideCard. Below {extension_min} percent, and above {extension_max_low} percent at a stress below {full_reach} MPa, the law was compared with the measured curves and missed, and a frame there carries FLAG_OUTSIDE_MET_RANGE"
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
        "The card answers one question: at a held stress, how far has the test piece crept at a given time since the full load, from zero creep extension up. t_s is the time since the full load was applied, as the card's law gives it. A run starts at the creep extension given, with empty books and the clock at the time the law itself takes to reach that extension from zero at the stress of the first segment: zero at zero, and 0.1 percent at 50.79 h at 140 MPa, 11.36 h at 170 MPa, 2.96 h at 200 MPa and 0.668 h at 230 MPa. That time since load is what was compared with measurement. The time between two extensions was not, and it does not hold: against the measured curves the law's time from one level to the next is off by up to 1.06 in the natural logarithm, and a clock started at 0.1 percent is outside the registered worst line of 0.30 at 5 to 16 of 170 levels at 170, 200 and 230 MPa. Do not subtract one frame's time from another's. A single step starts in the same way at each call: its creep_extension_percent handed back as a new start continues the same test, and the clock continues with it, while the work and the heat begin again at zero. stress_mpa is held: every segment of a run carries the stress of the first, and a segment with another stress refuses the whole run before its first step as StressNotHeld; no test with a changed stress is in the record. A stress outside the span in any segment refuses the whole run before its first step. The temperature is 980 degrees Celsius and is not an input. Where the law met the registered worst line of 0.30 in the logarithm of the time to a level, and where it missed, in numbers: from 0.1 to 5 percent at 170, 200 and 230 MPa it met it on all nine tests at every one of 171 levels, the worst 0.289; at 140 MPa from 0.1 to 0.25 percent, the worst 0.296. Below 0.1 percent it missed at every stress: the law is slow to start against every measured curve (in hours to 0.05 percent, the law against the three test pieces, 38.9 against 15.4 to 26.7 at 140 MPa, 7.8 against 2.7 to 3.5 at 170 MPa, 1.69 against 0.61 to 0.66 at 200 MPa and 0.245 against 0.06 to 0.08 at 230 MPa). Past 0.2 percent at 140 MPa it missed: the law leaves the worst line from 0.26 percent on one test and runs ahead of all three past 0.66 percent, and at the hour each test was stopped, 1010 to 1030 h, it gives 1.70 to 1.78 percent against 0.79 to 0.83 measured, about a factor of 2; no 140 MPa test went past 0.83 percent, and no test lies between 140 and 170 MPa. The card answers in both regions, and every frame there carries FLAG_OUTSIDE_MET_RANGE; through engine 0.9 it refused a start below 0.1 percent as CreepExtensionBelowCard and ended at 0.2 percent below 170 MPa. Under a held stress the extension only grows, and the step that would pass 5 percent, the largest registered level, is refused as CreepExtensionBeyondCard: a run that asks for more steps than that ends there, sealed with that refusal, and its last landed state is within one step of 5 percent. A step that lands exactly on it lands. At the ceiling from 0.1 percent that is 4,462 steps at 230 MPa, 11,031 at 200 MPa, 30,506 at 170 MPa and 102,748 at 140 MPa, each to 5 percent, at 62.6, 156.1, 434.8 and 1477.1 h. All are inside the 1,000,000 steps of a run. The card has no limit in time: the latest hour it can reach is 1477.11 h, at 140 MPa and 5 percent. A step of zero or less is refused as NonPositiveDt before the step floor is tested.".into(),
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
                "FLAG_CREEP_RATE_RISING: the law's creep rate at the end of the step is above its rate at the start. The law's rate falls from 0.1 percent to a minimum near 0.26 percent and rises from there; the flag says where on its own curve the law is and marks no failure of it. FLAG_OUTSIDE_MET_RANGE: the step covers creep extension where the law was compared with the measured curves and missed, below 0.1 percent at any stress or past 0.2 percent below 170 MPa. Both seal.",
            ),
            ("proof", "Not written by the step."),
        ]),
    )
}

/// What the capsule says of its two speed fields: the speed through the air
/// they make, its band above 68.96 km, and the corridor below it.
const CAPSULE_SPEED_REACH: &str = "The speed through the air is the square root of the sum of the squares of radial_speed_m_s and of transverse_speed_m_s less the air's own turning, 5.4197601852574874e-05 rad/s times radius_m. Above 68.96 km, where drag is read by altitude from a table computed along the 1997 design trajectory at 12173 to 12620 m/s (12487 to 12620 m/s in Table 1, 12173 m/s at the 68.96 km row from Table 4, of Mitcheltree, Wilmoth, Cheatwood, Brauckmann and Greene, Aerodynamics of Stardust Sample Return Capsule, AIAA 97-2304, 1997; that paper does not say whether these speeds are inertial or through the air), the speed through the air must be from 11900 to 12620 m/s, the top the highest speed the table was computed at, and a state outside is refused as AirSpeedOutsideCard. The locked entry, which met all three lines, passed through 11911.98 to 12573.44 m/s there at every step from 0.05 s to 0.0005 s; from 12573.44 to 12620 m/s the card holds the source's design speeds, not compared with the flight (engines 0.7 to 0.9 stopped at 12575 m/s). Below 68.96 km, where drag is read by Mach number, the Mach number must be from 0.16 to 40, the ends of that table, and a state outside is refused as MachOutsideCard; the four entries below Mach 1.0 were read from the paper's Figure 15. Any path is computed on the same equations and ledger. The locked entry flew a corridor, the lowest and highest speed through the air and sine of the path angle it had within 500 m of each altitude, widened by 1 m/s and by 0.001, at knots every 500 m from 31 km to 69 km (at 40 km 1147.51 to 1350.11 m/s); a state off it was not compared, and engines 0.7 to 0.9 refused it as OffComparedCorridor. Above 86 km the speed of sound is held at its 86 km value, 274.096 m/s, so the Mach number reported there is not the real air's; the band, not the Mach number, is the limit there. A state under 31030 m and under Mach 1.23 at once, past the flown drogue deployment, is refused as PastFlightDeployment at the start; a run that reaches it is ended there, at the end the card declares. A start deep in the atmosphere at the entry speed can be refused by the ledger at the largest step and lands at a smaller one";

/// What the InSight capsule says of a speed above its entry: the two speed
/// fields publish no bound of their own; the speed through the air is held.
const CAPSULE_INSIGHT_SPEED_REACH: &str = "The speed has no bound of its own. Its reach: the speed through the air, which is this speed less the air's own turning with Mars, must be at or above Mach 0.9, the first entry of the drag table read by Mach number, or the state is refused as MachOutsideCard, and at most 5850 m/s, where the drag table read by speed ends, or it is refused as AirSpeedOutsideCard. A state under 6164.62 m and under 352.1169 m/s at once is past the flown parachute deployment and is refused as PastFlightDeployment at the start; a run that reaches it is ended there. Any path is computed on the same equations and ledger; the compared runs flew a corridor, never above 5546.4 m/s and never climbing, and a state off it was not compared (engines 0.7 to 0.9 refused it as OffComparedCorridor)";

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
                    "distance from the centre of Mars. Altitude is radius_m - {} and is in [-2634.5, 128000] m: below -2634.5 m, the landing site's radius of 3393079.5 m, the capsule is at or under the ground and the state is refused as BelowLandingSite, and above 128 km as AltitudeOutsideCard. The air from 4 km up is the research record's table; under 4 km the same law, the hydrostatic column from the lander's barometer, 733.724 Pa at the landing site, is carried down in 35 knots. The compared run with the flight's lift reached down to 5793 m; under that, and under the flown deployment at 6164.62 m, nothing was compared",
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
        "the largest step the card stands behind, a locked constant, stated for the entry from the locked entry state: there the capsule falls 120 m in one step, under one interval of the air tables, and over the whole entry the conservation residual stays near a thousandth of its limit. A start deep in the atmosphere at entry speed, a state the entry itself never passes through, can be refused by the ledger at this step and lands at a smaller one",
        None,
        "lift_over_drag is in the published span, and a later segment of a run may change it. A lift outside that span in any segment refuses the whole run before its first step. A run from the entry state is ended as PastFlightDeployment, the end the card declares, at the first landed state under both 6164.62 m and 352.1169 m/s, the last row of the flight's record, the flown parachute deployment 209.27 s after the entry state: with no lift at step 2321, 232.1 s, 6163.7 m and Mach 1.373; with a lift over drag of -0.031 at step 2106, 210.6 s, 5793.3 m and Mach 1.470, the lowest state the compared run reached. Engines 0.7 to 0.9 ended both earlier, at 5700 m or Mach 1.46, the compared floor, as PastComparedFlight. Only runs from the locked entry state at 0.1 s were compared with the flight: with one lift over drag from -0.08 to 0.08 held through the run, or with -0.031 or 0 to 150 s and one lift in that span after. They flew a corridor, the lowest and highest speed through the air and sine of the path angle within 500 m of each altitude, widened by 1 m/s and by 0.001, at knots every 500 m from 5.5 km to 126.5 km; any other start or path is computed and was not compared, and a run whose lift changes in a way not compared goes on. A restart from a returned state lands where the run does. A new start sets the time, the angle and the drag-loss ledger to zero and anchors the ledger there; no limit depends on them.".into(),
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
                    "distance from the Earth's centre. Altitude is radius_m - {} and is in [0, 134700] m: above 134.7 km, the top of the drag table read by altitude, a state is refused as AltitudeOutsideCard, and below 0 m, the sea level of the 1976 standard and the ellipsoid under the path, as BelowStandardSeaLevel; the card has no terrain. The air from 30 km up is the research record's table of the 1976 standard; under 30 km the same standard's formulas in 151 knots from sea level. The flight was compared down to 31.03 km",
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
        "the largest step the card stands behind, a locked constant, stated for the entry from entry interface: over the whole entry the conservation residual stays under a thousandth of its limit. Its reach: a start deep in the atmosphere at entry speed, a state the entry itself never passes through, is where the step method's own error at this step can pass the conservation limit, and the ledger refuses it by name; a smaller step takes it, and from 40 km at the entry speed a hundred steps of 0.005 s land",
        None,
        "No load. A run segment carries an empty load object. A run from entry interface at the ceiling step lands 2779 steps, the last the first under both 31030 m and Mach 1.23, and is ended there as PastFlightDeployment, the end the card declares, the step past it not taken: a run asked for exactly those 2779 steps is ended there too. The step that first meets both drogue conditions lands, so a run at a smaller step can end earlier, by less than one of the larger steps: at 0.005 s the run lands 27782 steps, to 31029.67 m at 138.91 s. A start or a landed state off the corridor the locked entry flew is computed; a restart from a state the entry returns lands where the run does. A new start sets the time, the angle and the drag-loss ledger to zero and anchors the ledger there; no limit depends on them. Through engine 0.6.0 a run from entry interface ran on to 30 km or Mach 1.0 with an advisory flag.".into(),
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
            ("pos[1]", "ground_distance_m"),
            ("pos[2]", "mach"),
            ("vel[0]", "air_speed_m_s"),
            ("vel[1]", "radial_speed_m_s"),
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
            "UTC at the start of the run, in seconds after 2000-01-01 12:00 UTC: 2017-01-01 00:00 ({} s), the last leap second, from which TT - UTC is the card's locked 69.184 s, to the latest start from which the card's smallest step still lands on or before 2027-06-28 00:00 ({} s), when the IERS leap-second file of Bulletin C 72 (July 2026) expires; Bulletin C 72 announced no leap second at the end of December 2026, and the next the IERS can announce is at the end of June 2027. Outside those dates TT - UTC is not the card's, or not yet known, and a step is refused as EpochOutsideLeapSecondSpan. The card was compared on 28 days, 2026-08-23 to 2026-09-19, the last run ending 2026-09-20 00:00; a date outside them is computed. The Sun and the Moon are placed by a short analytic series to about a tenth of a degree at any of these dates, and the field's yearly rates are not applied",
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
            "0 s or more: the time since the state was taken from a published orbit. Required. State 0 for a state turned from a published orbit, and the forecast_elapsed_s of a returned state for a state handed back; below zero it is refused as ForecastElapsedBelowZero. It is reported plus t_s and no limit depends on it. The compared forecasts were at most {} s, seven days, from a published state: the worst distance after seven days was 32 m for LAGEOS-1 and 22 m for LAGEOS-2, against a line of 100 m. A longer forecast is computed; the card has no drag, no light reflected from the Earth and no ocean tide, so its error grows with the length of the forecast. The card cannot confirm the number stated",
            token(satellite_lageos1::FORECAST_HORIZON_S)
        ),
    );
    let position = vec3_stated(
        "position_m",
        "m",
        "in the card's turning frame. Its length is 6 378.1363 km or more, the Earth's equatorial radius, to which the field is scaled; below it the satellite is under the sphere of the Earth's equator, in the ground at the equator, where the card's field, the series of the Earth's field outside the Earth, is not the field, and the start is refused as RadiusBelowEarthRadius. The compared runs reached radii from 11 997.57 km (LAGEOS-2 near perigee) to 12 326.40 km (LAGEOS-2 near apogee); a larger radius is computed, and a smaller one down to the equatorial radius, each landing under 11 997 km carrying FLAG_BELOW_COMPARED_RADIUS. The card carries no air, so the lower the orbit, the larger the drag on the sphere that it does not carry; through engine 0.11 a radius under 11 997 km was refused as RadiusOutsideCard".into(),
    );
    let velocity = vec3_stated(
        "velocity_m_s",
        "m/s",
        "in the card's turning frame. Any orbit is computed, closed or not. Compared on two orbits only: LAGEOS-1, semi-major axis 12 265.14 to 12 274.83 km, eccentricity 0.003438 to 0.004375, inclination 109.8039 to 109.8235 degrees; LAGEOS-2, 12 158.51 to 12 165.68 km, 0.013232 to 0.013761, 52.6315 to 52.6508 degrees, each worked out from the position and the velocity seen from a frame that does not turn, with the card's z axis as the pole. On the 42 held-back runs every line was met: worst 0.65 m after one revolution, 3.8 m after one day, 32 m after seven days".into(),
    );
    finish(
        "aerospace",
        "satellite_lageos1",
        "The laser-ranging satellite LAGEOS-1 in free flight, in a frame turning with the Earth: the Earth's field to degree and order 12, the Sun and the Moon, the solid tide, sunlight pressure and relativity, with no load. Compared with the published orbits of LAGEOS-1 and LAGEOS-2 on 28 days of 2026 for up to seven days, it met every line on 42 held-back runs: worst 0.65 m after one revolution, 3.8 m after one day, 32 m after seven days.",
        vec![epoch, forecast, position, velocity],
        floor,
        "the ceiling divided by 2^20: 0.054 m of flight. Under it a step is mostly rounding",
        gate,
        "the largest step the card stands behind, a locked constant. Measured against an 8th-order reference from the worked example of engine 0.6.0, a circular polar orbit, the step error at this step is 11.2 mm after one day and 13.9 cm after seven days, and 0.4 mm to 21 mm after one day from four other start states inside the 0.6.0 bands. The step was measured for orbits near LAGEOS's; a lower or faster orbit needs a smaller step, and the two ledgers refuse one taken too large. The measurement is an outside recomputation in 50-digit arithmetic, 2026-10-07, one reader, not yet reviewed. Below the compared radii a lower orbit is faster and needs a smaller step, and the two ledgers refuse one taken too large: measured on circular orbits inclined 51.6 degrees, at 10 s the energy ledger reaches its limit and the step is refused by name after about 1.5 days at 6 400 km, 2.5 days at 6 700 km and 3.9 days at 7 000 km, and at 5 s a week at 6 700 km leaves 8.8e-11 and at 6 400 km 1.4e-10; over one revolution from 6 700 km to 11 000 km the 10 s step lands within 1.7 cm of steps of 2.5 s",
        None,
        "No load. A run segment carries an empty load object. What was compared with published orbits: two orbits, LAGEOS-1 and LAGEOS-2, on 28 calendar days, 2026-08-23 to 2026-09-19, in 56 runs of 10 s steps of at most seven days each, 42 of them on the held-back weeks that were scored against the pass line, the last ending 2026-09-20 00:00 UTC; every line was met on all 42 (1 m after one revolution, 10 m after one day, 100 m after seven days): LAGEOS-1 worst 0.65 m, 3.8 m and 32 m, LAGEOS-2 worst 0.34 m, 2.9 m and 22 m. The card computes past all of it: any orbit with a radius of 6 378.1363 km or more, the Earth's equatorial radius, any forecast length, any date from 2017-01-01 00:00 to 2027-06-28 00:00 UTC; none of that was compared. A landing under 11 997 km, the lowest point a compared run reached, carries FLAG_BELOW_COMPARED_RADIUS, which seals; the card carries no air, so the lower the orbit, the larger the drag on the sphere that it does not carry. The lowest radius and the dates are applied to the incoming state and to every landing and refuse by name outside: RadiusBelowEarthRadius and EpochOutsideLeapSecondSpan. Two ledgers are closed at every step: the energy seen from the turning frame against the work of the Sun, the Moon, the tide, sunlight and relativity, and the angular momentum about the Earth's axis against the torque of the field and those forces. The second is the one a Coriolis term with the wrong sign breaks, which does no work and which the energy ledger cannot see; through engine 0.9 the bands of the two compared orbits were the only guard against it. residual_conservation_error is the larger of the two residuals, each against a limit of 1e-9: over a week at 10 s from the test state they reach 1.5e-11 and 1.1e-11. The last date and rounding: the clock of a run is a running sum, t_s += dt_s in 64-bit arithmetic, and the time of a state is the start epoch plus that clock, rounded to the spacing of 64-bit numbers at these dates, at most 1.19e-7 s. The comparison with 2027-06-28 00:00 therefore carries a named allowance: EPOCH_END_ALLOWANCE_S, 1.19e-7 s, and CLOCK_ROUNDING_PER_STEP, one part in 2^53 of the clock, for each step taken so far at this step and two more, and never more than half the step. It is 1.2e-7 s on the first step of a run and 1.1e-5 s after 1,000,000 steps of 0.1 s. A run whose end in decimal is exactly the last date lands its last step, and its last epoch_utc_s can read up to the allowance past it; handed back as a new start, that value is past the card and is refused. The allowance is in the comparison a step makes and not in the published maximum of epoch_utc_s. A single first step that ends exactly on the last date lands.".into(),
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
        "No load. A run segment carries an empty load object. Each body's distance band, its orbit band and the speed of the centre of mass are applied to the incoming state and to every landing. Compared with Deep Space Network ranges for up to 16.2 years from JPL's 1999 state, in one run of 1800 s steps: Mars Odyssey to 2006 met its line (695 m at most against 700 m) and MESSENGER at Mercury met (2.08 km at most); Odyssey after 2006 and Mars Reconnaissance Orbiter missed, by at most 8 to 10 percent, in mid-2007; Juno and Cassini were not compared. In that run, to the last range compared with measurement, the bodies went through these distances from their primaries, each end rounded outward to 0.01 au: Mercury 0.30 to 0.47 au, Venus 0.71 to 0.73, the Earth 0.98 to 1.02, the Moon 356 000 to 407 000 km, Mars 1.38 to 1.67, Jupiter 4.94 to 5.46, Saturn 9.03 to 9.97, Uranus 19.88 to 20.10, Neptune 29.96 to 30.14, Ceres 2.54 to 2.99, Pallas 2.13 to 3.42, Vesta 2.14 to 2.58 au. The run swept the whole orbit of each inner body, of Jupiter and of the asteroids, and only part of the orbits of Saturn, Uranus and Neptune. A state outside those distances, inside the bands, is computed on the same forces and both ledgers; it was not compared. From 16.2 to 21 years the run was compared only with DE441, a calculation, and stayed within 1.9 km of its Earth-to-Mars distance. No date is held; carried on past the last measured range, the compared run goes on (until engine 0.9 it was refused 5,934.6 days from the 1999 state, Saturn farther from the Sun than 9.97 au). Two bodies at exactly one point are refused as BodiesCoincide; any other separation is computed. The compared run's closest pair other than the Earth and the Moon was Mercury and Venus at 0.2631 au, and its closest two asteroids Ceres and Vesta at 0.3885 au; the card has no figures and no tides of the bodies other than the Earth's oblateness on the Moon, so two bodies closer than their radii are still point masses to it, and a close encounter needs a smaller step, which the ledger asks for by refusing one taken too large. The centre of mass may be at any distance from the origin; the compared run's stayed 98 409 to 114 929 m from it, and a state moved bodily is the same motion with other rounding, 98.5 m on the Moon after one year when moved by 6e13 m. The ledger of the incoming state, energy_anchor, booked_work, angular_momentum_anchor and booked_torque, is closed at the start as at a landing and refused by the ledger's names when it is out of balance. The axes are held to the one thing the card can tell them by: a state whose Earth-Moon barycentre does not orbit the Sun in a plane 21 to 27 degrees from the z axis is refused as AxesOutsideCard, which names a state in ecliptic axes; a turn about the z axis is not seen.".into(),
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
const SOLAR_SYSTEM_START_STATE: &str = "The card holds no ephemeris and no date: the forces depend on where the bodies are and how they move, and on nothing else. Positions and velocities are barycentric, on the axes of the ICRF, in metres and metres per second; time is TDB. The worked example is the state of 1999-01-01 00:00 TDB (JD 2451179.5) from JPL Horizons, the state the card was scored from. Horizons names its source as DE441 for the planets and its own orbit solutions for the three asteroids; DE441 is a calculation whose start values JPL fitted to observations, not a measurement. `t_s` counts from the start of the run. Every start carries a ledger: `energy_anchor`, `booked_work`, `angular_momentum_anchor` and `booked_torque`, and a start without them is refused. A final state handed back as a new start, with its twenty-six vectors and its ledger, continues the same motion and the same ledger bit for bit, and the new run's `t_s` begins again at zero; no limit depends on `t_s`. A new start gives its own energy and angular momentum as the anchors and zero work and torque, as the worked example does; the card cannot tell that from a returned state given a new ledger, and such a start begins a new ledger. What a station measures is not in the state: the light time between a station on the turning Earth and a planet is worked out from these positions outside the engine.";

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
        | "electrolyte_v" => "V",
        "charge_removed_ah" => "Ah",
        "case_temperature_c" => "degC",
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
        "energy_mws" | "peak_energy_mws" => "MW*s",
        "power_rate_mw_per_s" => "MW/s",
        "reactivity_net_dollar" => "dollar",
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
        "reactor_spert3" | "reactor_spert3_fuel" | "reactor_triga_jsi" | "reactor_spert3_ccore" => {
            "power_rate_mw_per_s"
        }
        "grid_gb" => "net_power_mw",
        "cell_2170_m80" => "lower_limit_margin_v",
        // The slot holds the amplitude ceiling less the envelope; its row says so.
        "cantilever_eth" => "envelope_m",
        "cell_2170_model3" => "current_mean_a",
        "creep_cmsx6" => "stress_mpa",
        "crater_ryugu_sci" => "radius_to_go_m",
        "generator_mmrtg_seebeck" => "range_margin_s",
        "momentum_jwst_dihedral" => "h_magnitude_nms",
        "grasp_lift_glass_4tu" => "tangential_force_n",
        "momentum_jwst_srp" => "h_magnitude_nms",
        "link_gothenburg_10010" => "rain_rate_margin_mm_h",
        "solar_system" => "earth_mars_distance_m",
        "tire_eve_r19_ice" => "longitudinal_force_n",
        "accumulator_sbo450" => "gas_force_n",
        "runaway_18650_lco" => "heat_lost_j",
        "los_jwst_fine_guide" => "nis",
        "tokamak_jet_dte1" => "loss_power_w",
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
pub(crate) const CREEP_CMSX6_RESULT: &str = "Compared with the six held-back tests on 2026-10-09, and by the rule fixed beforehand the comparison passed. Over the 33 times to a level that the six tests reached, the logarithm of the card's time over the measured time had a root mean square of 0.093 against a line of 0.15, a worst of 0.240 against 0.30 and a mean of +0.019 against 0.10. On the three tests at 200 MPa, the stress no fitted test has: 0.085, 0.170 and +0.028 over 18 times. On the three new test pieces at fitted stresses: 0.102, 0.240 and +0.008 over 15 times. The worst single time was 0.5 percent at 170 MPa, where the card is 27 percent late. In hours at 200 MPa the card gives 2.96, 10.6, 48.9, 81.1, 113.4 and 156.1 to the six levels, and the three test pieces 3.1 to 3.2, 11.2 to 12.2, 41.3 to 43.9, 74.6 to 79.3, 106.3 to 114.0 and 138.1 to 151.8. That is the registered result, of the card as it then was: it stepped from zero extension to 5 percent at every stress and marked the range below 0.1 percent with a flag. What follows was done after the scoring and is not a blind test (amendment 4 of the research record, SHA-256 59b0f0bc055dd9336b151e8d0321fd9bcf0ceab6de3107d87cef99caf3501950). The card's range was narrowed to where its law held on the measured curves, and it now refuses outside it. The same six tests, run as the card now runs them from 0.1 percent, on the registered levels inside its range: over 32 times since load, the 33 less 0.5 percent at 140 MPa, which is now outside the card, a root mean square of 0.094, a worst of 0.240 and a mean of +0.021; each of the 32 is the registered value to 2 parts in 100,000 of the logarithm. The time from 0.1 percent to each higher level, which is not the card's clock and is given for comparison, over 26 times: 0.108, 0.249 and +0.029. The four saved requests of the registered run, which start at zero extension, are refused when sent again (evidence/research/materials_cmsx6_creep/analysis/out/16_enforced/).";

/// The result of the crater's validation record.
const CRATER_RESULT: &str = "Compared on 2026-10-09, and by the rule fixed beforehand the comparison with Ryugu failed. In the laboratory both lines were met: fitted on the nine craters made at 1 g, the law put each of the four centrifuge craters inside its 95 percent prediction interval, 6.9 to 9.5 percent low, 7.9 percent on average against a mark of 10. Those intervals are +/- 31 to 38 percent, so the first laboratory line could not have failed. The second, 7.87 percent, was foreseeable from Schmidt's published line, which was fitted through the centrifuge craters and was known beforehand, and it is fragile: with any one of four of the nine opened shots left out it is 10.4 to 11.8 percent, a miss. The four errors are all of one sign, a real bias of the nine-shot line. On Ryugu the line was missed: the card stops at 8.571 m and the measured apparent radius is 7.25 m, so the card is 1.32 m, or 18.2 percent, too large, against an allowance of 0.97 m (0.81 m from the fit, 0.40 m from the measurement, 0.34 m from the local gravity). 7.25 m is the apparent radius, which is what the law predicts: the apparent diameter is 14.5 +/- 0.8 m and the rim-to-rim diameter 17.6 +/- 0.7 m. The same team's submitted manuscript gave 6.5 +/- 0.4 m by another reduction. The published team's own range for sand scaling, 6.9 to 7.7 m by the dry-sand line of Housen and Holsapple, \"agrees well with the apparent radius\". The radius at which the engine stops was scored against the closed form of the thirteen-shot law with a mark of 1 part in 10^9, and that line was missed: the card scored on 2026-10-09 typed the law's two constants to six figures and stopped at 8.571304624905444 m, 3.45 parts in 10^8 from the closed form of the fitted law, 8.571304329477925 m. The record said the line was met because it compared the engine with the closed form of the same six-figure constants. Since engine 0.7.0 the card types the constants as the fit gave them and stops at 8.571304329477922 m, 4 parts in 10^16 from the closed form. That agreement is by construction: on the stopping step the card sets the radius to a typed constant, so it shows only that the constant was typed as computed. The miss on Ryugu is the same either way, 1.321 m. Without a mark: the two lines Schmidt published for Ottawa sand and for number 24 quartz sand give 8.60 m and 8.89 m; the fitted exponent, 0.1661, is inside the 0.165 +/- 0.005 that Gault and Wedekind measured between 0.073 and 1 g; the card stops 400 s after the impact, where the crater was seen to grow for about 200 s, the team's submitted manuscript says growth would stop by about 300 s, and the whole event lasted about 500 s; the constant that would put the law on Ryugu's crater is 0.737 against the 0.871 fitted in sand. The binary that made the scored run (SHA-256 425ef2b93d69be6f05da11eaf6ad4c4debecc37d548672950465d68ef2bf3b04) was overwritten by a later build; two later builds, 05efbd06… and 043d5765…, returned the answer byte for byte (evidence/verification/second_party/V-30/), and engines 0.4.0 to 0.6.0 give the same run proof. Engine 0.7.0, with the constants retyped, does not: its stop, its states and its proofs differ from the scored run in the eighth figure and beyond. A second scoring of 2026-10-09, on the same machine, found the same numbers: evidence/verification/second_party/V-30/.";

/// What a card that has not been compared with a measurement says.
pub(crate) const NOT_COMPARED: &str = "not compared with a measurement";

/// The pass line of the InSight capsule's validation record.
const CAPSULE_INSIGHT_PASS_LINE: &str = "Two sets, both fixed before the card was compared with any flight row (protocol SHA-256 ddafbbe8299da589e95a674bb31150f869d0f85d21415036bdbedfc4d7fa0a54, Amendment 2 SHA-256 9bb5b953e4ae325eb954fb3979b3942a08dc19bd5ff80ae2f6bbeda37b137146). The first lines are NASA's own printed spread for its own forecast: peak deceleration within 0.5 g, each time within 6.946 percent of elapsed time (7.68 s on the peak, 10.42 s at 150 s, 15.1 s at the parachute), radius at the parachute within 3.5 km, Mach number there within 0.2. The second set is built from the stated spread of the card's own inputs, twice the root sum of squares of the shift each input gives when moved by one standard deviation: the air density, the drag coefficient and the lift.";

/// The result of the InSight capsule's validation record: the comparison of
/// 2026-10-09 with the rows held back, as scored from the engine's frames.
const CAPSULE_INSIGHT_RESULT: &str = "Compared with the held-back rows on 2026-10-09: two runs, each against both sets of marks. With the lift measured on the opened rows, a lift over drag of -0.031: all four first lines met and all six stated-spread marks met. The flight's speed at parachute deployment was reached 1.31 s late and its radius 0.58 s early, against lines of 15.1 s; at the flight's deployment time the radius was 114 m low against a line of 3.5 km, and the Mach number 1.501 against 1.522. Over the 58 whole seconds held back the speed differed by 7.2 m/s root mean square, the radius by 245 m and the deceleration by 0.031 g. With no lift: none of the four first lines met, 17.1 s and 22.8 s late, 4.24 km high and Mach 1.91; all six stated-spread marks met, because those marks carry a lift the capsule could take either way and are wide: 6.8 km on the radius at the parachute, 27 s and 37 s on the two times and 0.72 on the Mach number, which is no forecast. On the opened rows, with the lift: all four first lines met, the peak 7.95 g against 8.12 g and 4.0 s early, and one stated-spread mark missed, the time of the peak, 4.02 s against 3.27. With no lift: the peak missed its first line, 7.60 g against 8.12 g with a line of 0.5 g, and the 150 s radius was reached 17.0 s late against 10.42; two stated-spread marks missed, the peak, 0.517 g against 0.489, and its time, 5.81 s against 1.82.";

/// The limits of the InSight capsule's validation record.
const CAPSULE_INSIGHT_LIMITS: &str = "The run that meets the first lines takes one number from the flight: its lift, a single number measured on the opened 150 s and held through the run. The run that takes nothing from the flight but its starting state misses them. So the card does not forecast an entry whose lift is not known; it reproduces this entry given its lift, and the 59 s it had not seen came out as stated. Those 59 s do not test the lift: the run is already right at 150 s, where the lift was measured, and on the engine before 0.7.0, which carried a run on to 4 km, it then met the four first lines with any lift from -0.08 to 0.08 over the 59 s; the flight's own lift over those 59 s was -0.016, half the number the run holds. They test the air below 18 km and the drag below Mach 6, to which nothing was fitted. The 114 m at the parachute is finer than the flight record's own one standard deviation there, 694 m, and is not the card's accuracy. The air and the lift were both chosen with the opened rows in view, so agreement on those rows is a comparison and not a test. The card's air is the mean of soundings one to seven sols old: between 36 km and 65 km it is 8 to 13 percent denser than the density reconstructed from the flight, more than its own stated spread allows, and the peak of the load comes 4 s early for it; below 18 km it is within 7 percent. The first lines are NASA's spread for its own forecast and are wide: 0.5 g is 6 percent of the peak. The comparison is not blind: NASA's printed values at the peak and at the parachute, Mach 1.522 among them, were read before the first protocol was written, and Amendment 2, which fixed the card's structure and added the line on the Mach number at the parachute against that printed 1.522, was written on 2026-10-09 after the first forecast had been scored on the opened 150 s. The capsule's sideways lift, wind, and the rise of drag in thin air are not on the card. One entry of one capsule, down to parachute deployment only. Engines 0.7 to 0.9 refused below 5700 m or Mach 1.46, the lowest state the run with the flight's lift reached in its scores (5793 m, Mach 1.470) rounded outward; the card now ends a run at the flown parachute deployment, and every one of the scores of the run with the flight's lift is on a landed step. The run with no lift ends at 232.1 s, 6163.7 m up, after it reaches the flight's speed at the parachute and before it reaches the flight's radius there: its 22.8 s on that radius was scored on 0.3.0, which carried it on to 4 km and Mach 1.20, and is not on a landed step now. Of the second scorer's 20 runs with other lifts, 13 have a score on a state below 5700 m or Mach 1.46, made on 0.3.0. The same aeroshell on Phoenix flew with its lift pointing the other way and has not been run. The runs were scored again from the protocol alone by a second scorer on the same machine on 2026-10-09, with the same verdicts (evidence/verification/second_party/V-23/); not yet repeated on another machine or by anyone outside the company. The held-back runs were made with a debug build of the gateway.";

/// The solar_system card's validation record.
const SOLAR_SYSTEM_DATA: &str = "Round-trip light times between stations of NASA's Deep Space Network and spacecraft at Mars, Mercury, Jupiter and Saturn, as reduced and published by JPL's Solar System Dynamics group (ssd.jpl.nasa.gov/planets/obs_data.html; Konopliv et al. 2011, Icarus 211, 401; Park et al. 2017, AJ 153, 121; Hees et al. 2014, Phys. Rev. D 89, 102002). The constants are those of DE440 and the start state is DE441's, through JPL Horizons: Park et al. 2021 for DE440 and DE441, Giorgini et al. for Horizons. Nothing is fitted. The Mars Global Surveyor ranges of 1999 to 2006 were opened and used to choose the card's structure and to set the pass lines. Held back: Mars Odyssey and Mars Reconnaissance Orbiter (Mars), MESSENGER (Mercury), Juno (Jupiter) and Cassini (Saturn).";
const SOLAR_SYSTEM_PASS_LINE: &str = "Fixed before any held-back measurement was compared with a model (protocol SHA-256 2d79e65df9fb8ad036444896e776e98d18cb98fc7b582f861747f27d79fd9679); of the held-back files only their first lines had been printed, to learn their layout, two rows of data among them in four files. For every measurement of a held-back file, measured minus computed as a one-way distance at most B x max(1, T / 7.75 years)^2, T counted from the start: B is 0.70 km for Mars, 1.0 km for Mercury, 1.5 km for Jupiter and 0.5 km for Saturn. One run from the 1999 state in steps of 1800 s.";
const SOLAR_SYSTEM_RESULT: &str = "Compared on 2026-10-09 with the held-back measurements, in one run of 7,670 days from the 1999 state, none refused. Two files met their lines, two missed and two comparisons were not made. The measured range residual equals the card's distance from JPL's DE441 ephemeris to about 5 m, so every verdict follows from DE441 without a measurement. What the result supports is that this simpler model, carried forward from JPL's own 1999 state, stays within 0.69 km of JPL's Earth-to-Mars distance over 7.75 years and 1.9 km over 21, within 2.7 km for Mercury, 3.2 km for Jupiter and 2.1 km for Saturn. The registered figures, file by file: Mars Odyssey up to 2006, the opened span (2,389 ranges): met, 270 m rms and 695 m at most against 700 m. Mars Odyssey after 2006 (4,939 ranges, out to 15.0 years): missed, 612 m rms and 1,312 m at most; 44 ranges were outside the line, all in mid-2007, by at most 8 percent. Mars Reconnaissance Orbiter (1,993 ranges, out to 15.0 years): missed, 619 m rms and 1,314 m at most; 34 ranges outside, all in mid-2007, by at most 10 percent. MESSENGER at Mercury (813 ranges, 12.3 to 16.2 years out): met, 1.42 km rms and 2.08 km at most against a line of 2.5 to 4.4 km. Juno at Jupiter (4 ranges) and Cassini at Saturn (196): not made, because the test of the reduction fixed beforehand failed; with DE441's own positions the reduction left 20 km and 10 km rms on those two files. The registered reduction left out the station term of TDB - TT, up to 48 m one way and 22 m rms. Under a reduction that has it the verdicts are the same: Odyssey up to 2006-10 met, 689 m at most against 700 m; Odyssey after missed, 45 of 4,939 ranges outside, the worst 1.095 of the line; Mars Reconnaissance Orbiter missed, 33 of 1,993 outside, the worst 1.093; MESSENGER met, 2,054 m at most, 0.640 of its line. The Odyssey line was met thinly: inside by 5 to 11 m of a line that is 678.3 m rounded up to 700, and with 678.3 m, 11 to 13 ranges are outside. The causes: the Mars error is the 340 asteroids the card leaves out, 730 of the 933 m at the misses of mid-2007, and no single asteroid does it; Mercury's is the Sun's oblateness, 2,591 of 2,661 m; the Earth's remaining 69 to 183 m is the Moon's error of 6 to 16 km, from the figure of the Moon, the tides and the Earth's figure beyond J2. Newton alone would be 351 km off by 2006-10. The engine's daily positions were also to agree with the outside model to 10 m: every body was within 3 m but the Moon, 14.6 m apart after 21 years, a miss, with the two growing apart smoothly from nothing as rounding does. It is rounding: two 64-bit runs of these equations that differ in the order of operations, or by one unit in the last place at the start, end 5 to 24 m apart on the Moon after 21 years, so the 10 m line could not have been held. Against a 32-digit run the engine is 9.5 m off on the Moon and 0.92 m on the Earth. The truncation of the 1800 s step over 21 years is 78 m on the Moon, 17.9 m on Mercury and under 1 m on the Earth. A second scoring of 2026-10-09, on the same machine, with an independent reduction and an independent integrator, found the same verdicts: evidence/verification/second_party/V-31/.";
const SOLAR_SYSTEM_CAVEATS: &str = "The comparison is not blind. The start state is JPL's DE441 through Horizons. DE441 is a calculation whose start values JPL fitted, before this work, to observations, these range measurements among them, so the start already carries information from the held-back measurements; what the comparison tests is whether this card's far simpler dynamics carry that one state forward for up to 16.2 years to where the stations measured (the run went on to 21 years, compared only with DE441). The held-back Odyssey ranges up to 2006 are interpolation: the same planet over the opened span. Odyssey and Mars Reconnaissance Orbiter after 2006 are extrapolation in time. Mercury, Jupiter and Saturn had no opened measurement, and their pass lines rest on a comparison with DE441, a calculation. The light time is worked out from the card's positions outside the engine: station position from the IAU 2006/2000A Earth orientation and the IERS series, the Sun's relativistic delay, no solar plasma. With DE441's own positions that reduction left 18 m rms on the opened file, the floor of the comparison. That floor of 18 m is the reduction's omission of the station term of TDB - TT; a reduction that has the term has a floor of about 5 m, and 1.3 m at Mercury. Left out of the card: every other asteroid, Pluto, the Sun's oblateness, the figure of the Moon, every tide. The square in the pass line was a guess at how the error grows beyond the opened span; the Mars misses lie just past that span, where the line had barely risen. The square is not physical: the envelope of the error grows about in proportion to time (T^0.96), so the line is too tight just after the opened span and twice too loose at 15 years. The Juno and Cassini files as published leave about 20 km and 10 km with JPL's own ephemeris through this reduction. That is consistent with a station delay left twice in the round-trip time; this is an inference from DE441 and is not stated in the files' headers. No registered comparison was made for Jupiter or Saturn, and after that correction the Cassini file still fails the test of the reduction, at 590 m rms. FrameOutsideCard tests the speed of the centre of mass and nothing else. Where the centre of mass is, is not held: the compared run's stayed within 114 929 m of the origin, and a state moved bodily is the same motion in other numbers with other rounding: moved by 6e13 m it put the Moon 98.5 m from the unmoved run after one year. Through engine 0.9 such a state was refused as OriginOutsideCard past 120000 m. The axes have a limit of their own, AxesOutsideCard: the orbit of the Earth-Moon barycentre about the Sun must be inclined 21 to 27 degrees to the z axis, as the ecliptic is to the ICRF equator. In DE441, as Horizons serves it, that angle is 23.43 to 23.45 degrees from 1900 to 2100 and falls from 24.94 to 22.56 degrees from 9999 BC to AD 9999; a state given in ecliptic axes instead of ICRF axes has about 0 degrees and is refused, where through engine 0.5.0 it was accepted with the Earth's pole in the J2 term 23.4 degrees wrong. What that limit cannot tell: a turn of the axes about the z axis, by any angle; the equator of another date in place of the ICRF's, which keeps the angle and moves the true pole from the z axis by the precession of those years; and two bodies whose distance bands overlap, Ceres and Pallas, given in each other's place, which are taken without a refusal. Two bodies at exactly one point are refused by a name of the card, BodiesCoincide, before anything divides by the distance between them; through engine 0.5.0 they were refused with no number and no name of the card. Two bodies closer than the compared run's closest pair of their kind, any pair but the Earth and the Moon under 0.26 au or any two asteroids under 0.38 au, are computed as point masses, held by the ledger; through engine 0.9 they were refused as BodiesTooClose. The card has no figures: Pallas 100 km from Ceres's centre, inside Ceres, is still two point masses to it, and an agent asking about bodies in contact is asking about another object. A body where the compared run never had it is computed inside its band; through engine 0.9 it was refused as DistanceOutsideComparedRun, past the distances the run went through up to the last range compared with measurement, 16.2 years after the 1999 state, each end rounded outward to 0.01 au. From 16.2 to 21 years the run was compared only with DE441, a calculation. The run to 16.2 years swept the whole orbit of each inner body, of Jupiter and of the asteroids, and only part of the orbits of Saturn (9.03 to 9.97 au), Uranus (19.88 to 20.10 au) and Neptune (29.96 to 30.14 au); through engine 0.6.0 a state with those three anywhere in their bands was answered with a proof. Long runs go on, since the card holds no date: carried on past the last measured range, day 5,920.4, in steps of 1800 s, the compared run passes day 5,934.6, where Saturn goes farther from the Sun than 9.97 au, and through engine 0.9 was refused there. The scoring was run in the same pass as the comparison of the engine with the outside model, not after it as the protocol had it. Every result is of this one start; no other start state has been scored. What was compared is the envelope of that one run up to the last range compared with measurement: each body within the distances the run went through, no pair closer than the run's closest of its kind, the centre of mass within 114 929 m of the origin. The card computes past that envelope, inside each body's band; a start state outside it, or inside it but not on the run, Neptune at another longitude for instance, was not itself compared.";

/// The result of the cantilever's validation record: the comparison of
/// 2026-10-09 with the held-back half of the recording.
const CANTILEVER_RESULT: &str = "Compared with the held-back half on 2026-10-09, and by the rule fixed beforehand the comparison failed: three of the four lines were met and the amplitude line was missed. On the eleven blocks of eight periods between 12 s and 24 s the amplitude error was 7.03 percent against a line of 5.0. The frequency error was 0.0031 Hz against 0.010 and the timing error 0.021 of a period against 0.20, and both the amplitude error and the frequency error were under one third of a plain damped oscillator's, which missed by 40.8 percent and 0.048 Hz. The card's swing is too large in every block and the gap widens, from 3.6 percent at 13 s to 11.7 percent at 23 s: it dies away too slowly once the swing is small.";

/// The result of the link's validation record: the comparison of 2026-10-09
/// with the month that had been kept unread.
const LINK_RESULT: &str = "Compared with the rain events of August 2015 on 2026-10-09, in one run of each event through the engine, and all three lines were met. Root mean square error 0.581 dB, against 0.90. That is 0.585 of the error of P.838 alone, 0.994 dB, against 0.80. The summed attenuation was 1.069 of the measured sum, inside 0.75 to 1.25. No attenuation at all scores 1.621 dB. The rule fixed beforehand finds 21 gauge events in August: eleven are under 1.0 mm and by the rule are not scored, nine were scored and none was refused: 773 five-minute blocks; and one of 15.8 mm that starts 2015-08-31T17:37Z was not scored because its window runs past the end of the archive. A second scorer scored that event to the last minute of data, after the fact: the month then reads 0.622 dB, 0.528 and 0.974, all inside the lines. In two of the nine events P.838 alone was closer than the card, 0.644 against 0.675 dB and 0.346 against 0.400 dB. August's rain reached 54 mm/h, inside the 84 mm/h fitted. Its dry levels, 52.60 to 52.90 dB, were above every fitted event's, 51.75 to 52.55 dB, and inside the card's limit. The card's five-minute peak is below the measured five-minute peak in seven of the nine events, by up to 2.2 dB. On the one-minute values the card returns, its peak is below the measured peak in six of the nine events, by up to 2.8 dB, and above it in three, by up to 3.3 dB. In light rain under 2 mm/h the card is about 0.3 dB high. The link leads the gauge by one to two minutes: moving the link series one minute later lowers the error from 0.580 to 0.516 dB, and this is not corrected. The other direction of the same link, sublink 588, scored after the fact with the same constants, does not transfer under the protocol's rules: 0.794 dB, no better than P.838 alone, with a summed attenuation of 2.30 of the measured sum, because that receiver's dry level wanders by more than 1 dB within a dry day; in the events where its level was steady the card does about as well as on the scored direction. A second scoring of 2026-10-09, on the same machine and from the scorer's own cut of the archive, found the same verdicts: evidence/verification/second_party/V-28/. The binary that was scored served this record with the words \"not yet compared with the held-back month\"; this text was written afterwards and changes no number the engine computes.";

/// The marks of the physical 2170 cell: the ones fixed for the cell_2170_m80
/// card, unchanged.
const CELL_2170_MODEL3_PASS_LINE: &str = "The same marks as for the cell_2170_m80 card, unchanged since its protocol. On a discharge at a held current: voltage error at most 42 millivolts root mean square and delivered capacity within 3 percent. On a drive cycle: voltage error at most 42 millivolts root mean square, the card still running at the last sample, and the error at the last sample at most 126 millivolts.";

/// The result of the physical 2170 cell's validation record: the check of
/// 2026-10-09 on runs of the second and the third cell that no model had
/// seen, as the study's record of that check states it.
pub(crate) const CELL_2170_MODEL3_RESULT: &str = "Run once, on 2026-10-09, on runs of two further cells of the same type that no model had seen, from -10 to 40 degrees Celsius. One resistance number was fitted to each cell from its characterisation tests and nothing else was changed. Third cell, m448-N: 51 of 60 runs were inside the marks, within the 48 to 54 expected, and all five conditions written beforehand were met. Second cell, m1000: 31 of 45 runs were inside the marks, below the 36 to 41 expected, and three of the five conditions were not met. No ordinary drive cycle missed the voltage mark on either cell: of 80, the card ran 78, and 72 were inside the marks. The misses on the second cell: its discharge at half the rated current delivered 3.3 to 4.0 percent more charge than was measured, at four of five temperatures; three drive cycles at 40 degrees Celsius ended 128 to 145 millivolts high, against a mark of 126; and one drive cycle at -10 degrees Celsius stopped at the card's upper voltage limit a quarter of the way through. The misses on the third cell: its discharge at the rated current at -10 degrees Celsius, with a voltage error of 73 millivolts against 42 and 7.3 percent short on capacity; and two drive cycles in which the card reached its 2.8 V limit 2 and 13 seconds before the last sample. Twelve runs were refused as outside the card's range and count as not passed. All ten hill-climb cycles are among them, nine refused for current and one for case temperature, so that kind of run is untested on new cells. The other two were one drive cycle with three one-second samples of current above 13.524 A and one that started above 1.02 of full.";

/// The result of the fuel-temperature reactor's validation record: the
/// comparison of 2026-10-09 with the 30 held-back tests as it came out in
/// the research record.
pub(crate) const REACTOR_SPERT3_FUEL_RESULT: &str = "Compared with the 30 held-back tests on 2026-10-09, and by the rule fixed beforehand the comparison failed in each of the three groups and for the 30 together: of the 24 lines of the three groups 18 were met and 6 were missed, and of the 8 lines for the 30 together 5 were met. Each triple is root mean square, worst and mean, in units of the report's standard deviation; the times are relative errors. Cold group, 10 tests: peak power met its three lines (0.514, 0.937, +0.204), energy released to the peak met its three (0.540, 1.087, +0.023), and time of the peak missed both of its lines: root mean square 5.22 percent against 5 and worst 12.24 percent against 10. That miss is at the floor of the measurement: in this series the report's own plots and its own table disagree on the time of the peak by about 3 to 6 percent. The group at 251 to 259 degrees Fahrenheit, 13 tests, the report's series at about 260 degrees Fahrenheit and the one group with nothing fitted to it: peak power met its three lines (0.643, 1.051, and a mean of +0.479 against a line of 0.5 in size, met by 0.02), energy met its three (0.846, 1.334, -0.152), and time of the peak missed both: root mean square 8.46 percent and worst 14.88 percent, twelve of the thirteen peaks early. That miss is about twice the floor of the measurement there and is the card's. The group at 498 to 501 degrees Fahrenheit, 7 tests: peak power met its three lines (0.796, 1.650, and a mean of -0.490 against 0.5, met by 0.01), time of the peak met both (4.49 and 7.45 percent), and energy met one line and missed two: root mean square 1.336, worst 3.212 against 3, and mean -0.890 against 0.5 in size; the card's energy is low on all seven tests. The 30 together: peak power met (0.644, 1.650, +0.161); energy met two lines and missed one (0.908, mean -0.266, worst 3.212); time of the peak missed both (6.69 and 14.88 percent). The scored step was 0.43 ms, the card's ceiling, where the protocol said 0.5 ms, a step above that ceiling, which the door refuses. A second scoring on 2026-10-09, written from the protocol by a second agent with its own reading of the report's tables and a model of its own (evidence/verification/second_party/V-25/), on the same machine, reached the same figures and the same verdicts.";

/// The limits of the fuel-temperature reactor's validation record.
pub(crate) const REACTOR_SPERT3_FUEL_LIMITS: &str = "A miss, and it stays on the record. Four constants are fitted on 28 tests: the power at the start, 11.6113 W; the strength of the Doppler feedback, 0.0371601 dollar per square root of a kelvin; the heat capacity of the fuel, 0.0260616 MW s per K; and one weight on the water-temperature feedback for all three bands, 0.28586. The starting power is a fitted effective value and not a measurement: across what the second scoring varied it came out at about 7 to 11 W, the report's 50 W is excluded under this model, and the difference from the report is unexplained. The insertion ramp is the report's design figures, a constant rod acceleration and a constant worth per inch, and not a recorded rod motion; the recorded power histories prefer a somewhat faster insertion, and 10 to 15 ms of fixed-time error remains, where the card is late on the fastest tests. The fuel temperature is compared with no measurement, because the report measures none: fuel_temperature_rise_k is the temperature of one fitted lump, about one fourteenth of the heat capacity of the whole fuel, and was 69 to 402 K at the peak on the held-back tests; it is tested only through what it does to power, energy and time, and it is not a temperature a thermocouple in the fuel would read. Heat flow from fuel to water is left out. The delayed-neutron constants are the report's values stated for 500 degrees Fahrenheit, used in all three bands. The comparison is not blind. This card's form was built after the first card of this reactor had been compared, on 2026-10-08, with the ten cold-startup tests held back here and had put every peak about 22 percent early: those ten are held back from this card's fitting and are not unseen. Before the marks were fixed on 2026-10-09, the whole of Tables V and VII as transcribed, their held-back rows included, had been printed to the fitting agent's screen, the plotted time of peak of held-back test 70 had been read, and the power history of one held-back cold test had been looked at; that no model had been run on a hot test rests on the builder's word and on file times of one machine, which agree with it and cannot prove it. The report's standard deviations share a part from the power calibration, so the errors of two tests are not independent. The conservation residual names a wrong method and is not a bound on accuracy. On the fastest run of each band, by a ledger the second scorer wrote, it named explicit Euler, the midpoint and Heun's second-order methods, Kutta's third-order method, Runge-Kutta with one stage weight wrong by 1e-3 and by 1e-5, a delayed group dropped from the power equation, the Doppler strength wrong by 1e-3 and by 1e-5, a group fraction wrong by 1e-4, and Runge-Kutta at two and at three times the step ceiling; it did not name a group fraction wrong by 1e-6 or Runge-Kutta at 1.5 times the ceiling. Its limit, 3e-7, is calibrated on the card's own method, at 11 times the largest residual that method leaves, and the residual reads the true error of the state low by about 11 to 15 times: on two of the three fast runs the true error, 3.3e-7 and 3.8e-7 of the neutron population, is above the limit while the residual is 2.3e-8 and 2.6e-8. A run is one burst with one reactivity; a reactivity changed during a burst is refused by name. Each step may carry its own flow rate; no tested burst changed its flow, and a burst whose flow changes was not compared. Outside each band's compared reactivity and flow the card computes the same equations and nothing was compared. Between the three bands nothing was measured and the card refuses. The card stops at the first power maximum: hot standby, operating power, boiling, void, pressure and any other core are not on the card. The reactivity is the report's reactor period converted by the caller with the band's own generation time; the card takes dollars. One party wrote the card and ran it; the second scoring was on the same machine and did not rebuild the engine.";

/// The result of the tyre's validation record: the one scoring of
/// 2026-10-09 on the 60 held-back runs that hold a scored slip, of the 90
/// held back, as its record states it.
pub(crate) const TIRE_EVE_R19_ICE_RESULT: &str = "Ninety runs were held back. On 2026-10-09 the card was compared with the 60 of them that hold a scored slip, as 60 cells of three repeats each, and all five lines fixed beforehand were met; the other 30 held-back runs step through slips of 0 to 3 percent, which the protocol does not score. Root-mean-square error of the friction ratio: 0.034 over the 60 held-back cells against a line of 0.045, 0.040 over the 24 cells at 2.5 bar and 0.029 over the 36 cells at 6.5 kN. A ratio of 0.1 that uses no measurement of this tyre gave 0.122, and the opened runs' geometric mean with no slip term (least squares on the logarithm) gave 0.036. The mean measured ratio was 0.218, and the card read high by 0.019 on average. Two held-back conditions taken alone were above 0.045, traction at 2.5 bar at 3.5 kN and at 5 kN, with errors of opposite sign; no line was set on a single condition. One held-back run, number 153, is a faulty record: it rolled for about 90 s like the others, and its channels are on other scales. Its vertical force reads 1,849 N at a nominal 3,500 N, and its wheel angle spans 0 to 11.75 where every other file spans 0 to 360. It was scored with the rest, as the protocol says; without it, computed after the scoring and no part of the registered result, the three errors are 0.031, 0.034 and 0.029. The card read high on all six conditions at 6.5 kN, by 0.023 on average. On the 48 opened cells the error was 0.030. The binary that made the scored runs (SHA-256 7a2f3cc8e75aadbbdaf4f74289803ddf1149dbe882d94cadc96f668061b943c7) was overwritten by a later build, which returned all 108 saved answers byte for byte. Since engine 0.7.0 a request states slip_held_s, so those saved requests are refused as they were sent. A second scoring of 2026-10-09, on the same machine and on the scorer's own reduction of the raw files, found the same verdicts: evidence/verification/second_party/V-33/.";

/// The result of the line-of-sight card's validation record: the comparison
/// of 2026-10-09 with the held-back visits, as its research record states
/// it.
pub(crate) const LOS_RESULT: &str = "Compared with the held-back visits on 2026-10-09, and by the rule fixed beforehand the comparison passed: all five marks were met on both axes. The marks are on quiet frames. The card as it was scored answered every frame of a file, and on a whole file the outcome was the other way. Over all 973,388 measured held-back frames, quiet and disturbed together, the card's error one frame ahead was 1.35 on x and 1.26 on y of the error of taking the last centroid, and the squared error over the variance it states was 3.9 and 3.0. Disturbances are in 121 of the 134 files and make up 18.3 percent of the measured frames. The card's own whole-file figure on x, nis_sum_x over measured_frames in the final state, lies in 0.5 to 2.0 in only 39 of the 134 files. The card as scored marked the 5.8 percent of frames above 5 mas with a flag and answered them, and answered the 30 s after them with no mark. Engines 0.7 to 0.9 refused those frames; what that gave on the same files is at the end of this record. The card now answers them again and marks them with three flags, FLAG_POINTING_DISTURBED, FLAG_BEYOND_FOUR_SIGMA and FLAG_RECOVERING; the registered whole-file numbers above are what it does there. On the 795,021 quiet frames of the 134 held-back files, one frame ahead, the card's error was 0.510 mas on x and 0.472 mas on y, root mean square, against 0.644 and 0.584 for taking the last centroid (0.790 and 0.807 of it) and 0.660 and 0.748 for taking the lock point (0.772 and 0.631 of it). The mean of the squared error over the variance the card states was 1.004 and 0.986, the correlation from one frame to the next 0.074 and 0.074, and in every one of the 134 files that mean was between 0.5 and 2.0: from 0.68 to 1.27 on x and from 0.70 to 1.75 on y. All 134 runs completed and none was outside the card. Inside the disturbances, 178,367 frames within 2 s before and 30 s after a centroid more than 5 mas from the lock point, the card was worse than taking the last centroid: 2.03 mas against 1.00 on x and 1.57 against 0.78 on y, and the variance it stated was too small by a factor of 17 on x and 12 on y. No mark was set on those frames. A random walk with the same power-law sensor noise, three constants an axis fitted on the same April data, meets all five marks, at 0.808 and 0.837 of the last centroid's error. The card's two modes and twelve further constants lower the error by 2 and 4 percent: 0.510 and 0.472 mas against 0.521 and 0.489. The marks do not test the mode frequencies: with both moved 20 percent up or down every mark is still met. The error is near the floor the sensor's noise sets. The sensor-noise law is what carries the variance marks: a second-order autoregression with one variance misses the per-file mark. The y noise exponent was not reproduced on the held-back guide stars: 1.045 fitted there against the card's 1.273, where x gave 1.134 against 1.151. Inside the tested count rates the two y laws differ by +27 to -21 percent in variance, which the variance mark cannot resolve. With a lock point that uses no future, the median of the first 10 s or a running median of past frames, the last-centroid, variance, correlation and per-file marks still hold, each number within 0.04 of the scored one, and the lock-point mark is no longer a test. Held-back files longer than 10,000 frames were cut at the gateway's limit: 43 files, which lose 12 percent of the set's frames. The second scorer's own filter on whole files still meets the marks. A second scoring of 2026-10-09, on the same machine and with the scorer's own FITS reader, found the same numbers: evidence/verification/second_party/V-35/. All of that is the registered result, of the card as it then was. What follows was done after the scoring and is not a blind test (amendment 1 of the research record, SHA-256 e0e4b86fcc18c2cd868f587627a0ac2ed0d5dd692292b1febbcc6358976dd849). The card was then changed, for engines 0.7 to 0.9, to refuse where it is wrong: a centroid more than 5 mas from the lock point, a centroid more than four stated standard deviations from where it was expected, and a start less than 30 s after either. The three numbers are the protocol's own and none was fitted to a held-back file. The second rule was decided on the fitted files of April 2022: there, in the 30 s after a centroid beyond four standard deviations, the x axis missed four of the five marks (0.921, a squared error over stated variance of 1.641, a correlation of 0.272, 74 percent of files), and on the held-back files three (0.933, 1.513, 0.316); such a centroid came 31 times as often on x as a right variance gives on the fitted files and 21 times on the held-back files. The 134 held-back files run again as engines 0.7 to 0.9 ran them, each file as a chain of runs that end at a refused centroid and start again 30 s later with nothing known, with the registered lock point: of 973,388 measured frames the card answered 721,727, 74.1 percent, in 428 runs, and refused 251,661: 56,910 centroids above 5 mas, 305 beyond four standard deviations, where a right variance gives about 91 by chance, and 194,446 in the 30 s after one. 130 of the 134 files hold a refused frame. Over the answered frames the five marks, x and y: 0.787 and 0.805 of the last centroid's error, 0.779 and 0.640 of the lock point's, a squared error over stated variance of 0.987 and 0.970, a frame-to-frame correlation of 0.064 and 0.072, and that mean between 0.5 and 2.0 in 100 and 100 percent of the files with 500 answered frames or more. Beside the registered 0.790 and 0.807, 0.772 and 0.631, 1.004 and 0.986, 0.074 and 0.074, 100 and 100 percent on the 795,021 quiet frames, which were chosen with 2 s of look-ahead that the card does not have. The card's own figure, nis_sum_x and nis_sum_y over measured_frames in the final state of a run, lies in 0.5 to 2.0 on both axes in every one of the 312 runs that landed 500 frames or more, where on x it did in 39 of the 134 whole files. The run through the engine and the same chain in plain Python answer the same frames, and their numbers agree to 2.4e-7 mas (evidence/research/pointing_jwst_fine_guidance/analysis/out/11_enforced/).";

/// The one line an agent sees in the list for the finger pad: what it is,
/// that its constants are fitted to the finger's own trials, and how the
/// comparison stands.
pub(crate) const FINGERPAD_P02_GLASS_SUMMARY: &str = "One human finger pad pressed on flat glass and twisted about the normal until it slips, the right index finger of participant P02 of the torsion experiment at UCLouvain: the torque, the stuck fraction of the contact, the work and the heat, from eight constants fitted to this finger's own trials. Compared on 2026-10-09 with its held-back trials at 1 N, 10 N and 100 degrees a second and missed: the slip torque was off by +9.4, +29.3 and -13.0 percent against a mark of 5.62 percent. On its fitted trials it is off by +13.6 percent at 0.5 N and +21.7 percent at 5 N, and within 0.7 to 7.4 percent at 2 N from 5 to 50 degrees a second, where its level was taken. It answers at every force and rate above zero with those misses stated; below 0.5 N, above 10 N, below 5 or above 100 degrees a second and past 75 degrees of twist nothing was measured, and a frame there says so. Another block of the same finger differs by 13 to 15 percent, another finger by about a fifth.";

/// The result of the finger pad's validation record.
pub(crate) const FINGERPAD_P02_GLASS_RESULT: &str = "Compared with the 30 held-back trials of this finger on 2026-10-09, and by the rule fixed beforehand the comparison failed: of the twelve lines three were met and nine missed, and the slip torque was missed at all three held-back conditions. Slip torque, card against the mean of ten trials: 6.03 against 5.51 N mm at 1 N (+9.4 percent), 35.63 against 27.55 at 10 N (+29.3 percent), 10.15 against 11.66 at 2 N and 100 degrees a second (-13.0 percent); the mark was 5.62 percent. Torque curve from 1 to 75 degrees, as a fraction of the slip torque: 0.073 (met), 0.263 and 0.115 against 0.0832. Twist at full slip: 40 against 29.0 degrees, 40 against 40.4 (met) and 39 against 48.1, with a mark of 5.75 degrees. Stuck-fraction curve: 0.036 (met), 0.081 and 0.135 against 0.0595. A simpler rule did better than the card on the slip torque at each condition: at 1 N the same model with the constants of another finger, P01, +1.9 percent; at 10 N a straight line in load, +18.4 percent; at 100 degrees a second no effect of rate, -4.7 percent. The card did better than torque proportional to load (-25 and +50 percent) and than the nearest fitted load (-44 and -38 percent). The scoring stands. The card that was scored answered from 0.5 to 10 N and from 5 to 100 degrees a second; engines 0.7 to 0.9 answered only from 1.93 to 2.07 N and from 5 to 50 degrees a second; this one answers at every force and rate above zero, with these misses stated. Where the card holds its own slip-torque mark, on the trials it was fitted to: at 2 N only, at 5 degrees a second (10.32 against 10.24 N mm, +0.7 percent), at 50 (10.23 against 10.68, -4.2 percent) and at 20 against the ten trials twisted about the centre of the contact (10.30 against 10.84, -5.0 percent). At 2 N and 10 degrees a second it is just outside (10.33 against 11.08, -6.8 percent), and at 20 with the 20 off-centre trials pooled, as the fit took them, it is -7.4 percent (11.12). On its fitted trials at the other loads it is off by +13.6 percent at 0.5 N and +21.7 percent at 5 N. Why: this finger's slip torque at its five loads was 3.11, 5.51, 11.12, 17.15 and 27.55 N mm; 70 of the 80 fitted trials are the block at 2 N, the fit takes its level from them, and that block stands 24 percent above the power law that the other four loads follow to within 4 percent. At 2 N, the other lines on the four fitted conditions, 5, 10, 20 and 50 degrees a second: torque curve 0.028, 0.060, 0.089 and 0.048 of the slip torque against 0.0832; twist at full slip 40, 40, 39 and 39 degrees against a measured 45.0, 44.1, 46.4 and 48.8, so 4 to 10 degrees early against a mark of 5.75; stuck-fraction curve 0.110, 0.113, 0.132 and 0.141 against 0.0595, missed at all four. The card's slip-torque law is numerically a two-constant power law of load, 6.03 N mm x F^0.772, fitted to the same 80 trials weighted alike: it errs by +9.4 and +29.4 percent at 1 and 10 N where the card errs by +9.4 and +29.3, and the rate term and the five twist constants change the slip torque by 0.1 percent at those loads and by 0.6 percent at most inside the range. The paper's own fit of that law over the seven fingers is n = 0.76 +/- 0.06 and k = 6.74 +/- 1.31 mm. With the paper's exponent and this finger's level from its three opened condition means the held-back loads read +1.4 and +16.7 percent, closer than the card at both; with the paper's own k, the mean of seven fingers, they read +22.3 and +40.8 percent, further than the card, because this finger is one of the two of lowest friction. The five other fingers, with no mark: the card is off by 24 percent, root mean square, over their 45 condition means and reads low by 18 percent on average; between the seven fingers the slip torque at one load spreads by 17 to 26 percent. A second scoring of 2026-10-09, on the same machine, from the scorer's own reduction of all 768 files, found the same twelve verdicts, and the same against the authors' own per-trial summaries, where the slip torque is off by +9.4, +29.3 and -12.4 percent: evidence/verification/second_party/V-40/. The scored binary is kept (SHA-256 9b7ad4088cd7af1b3255eba36cc99cc2f7297c779f76bbdc8432920f13ff7c0a). The build that carries this text differs from it in the start it accepts, in its flags, in its ledger scale past the scored range and in this description; no fitted constant moved, and the four saved runs of the scoring that lie inside the range come back with every frame the same, byte for byte. Record: evidence/research/tactile_slip_fingerpad_p02_glass/RESULTS.md.";

/// The limits of the finger pad's validation record.
pub(crate) const FINGERPAD_P02_GLASS_LIMITS: &str = "All eight constants are fitted to this finger's own trials; nothing is derived from skin or glass, and the form (two characteristic twists, no load term in the twist of the stuck area) was chosen on another finger of the same experiment, P01. What the card holds: this finger, in one block of 70 trials of one sitting, slipped at 10.2 to 11.1 N mm at 2 N from 5 to 50 degrees a second (the means of its four conditions; single trials ran from 9.3 to 13.2), and the card gives that to about 8 percent. Those are trials the constants were fitted to. At 2 N from 5 to 50 degrees a second there is no held-back trial, so nothing there is a test; the only test, at held-back conditions, failed, and the card answers there with the misses stated. A miss, and it stays on the record. The floors of the slip torque, which no card of one finger can go under: one trial about the mean of its condition inside a block, 5.2 percent on this finger; block to block, 13 to 15 percent; finger to finger, 17 to 26 percent. The block-to-block figure is how far the block at 2 N sits from the power law through the other four loads over the seven fingers: +24 percent on this finger, from -12 to +24 over the seven, 13 percent as a standard deviation and 15 as a root mean square. The paper gives the order of the blocks, 0.5, 1, 5, 10 and then 2 N, not randomised because the camera was refocused for each force, and names the confound itself: total experiment time, with moisture increasing with occlusion time. The card takes its level from the last block and does not know the block. On this finger on another occasion a user should carry 13 to 15 percent, and on another finger about a fifth; nothing here tests another day, another surface, a wet or a treated finger, or a finger sliding along the glass: the motion is a twist about the normal only. Through engines 0.7 to 0.9 the card answered only within 0.07 N of 2 N, the spread of the robot's force controller, and refused every other force. It answers now at every force above zero with the law's two load exponents, which missed at every other load measured: +13.6 percent at 0.5 N, +9.4 at 1 N, +21.7 at 5 N and +29.3 at 10 N. Nothing was measured below 0.5 N or above 10 N, and there a frame carries FLAG_BEYOND_COMPARED. The mean at 2 N and 20 degrees a second pools 20 trials twisted about a point off the centre of the contact, which the paper does not count among its 90 trials a finger and which read 3.9 percent higher on this finger: 11.12 N mm pooled, 10.84 from the ten centred trials. On this finger the slip torque shows no trend with rate from 5 to 50 degrees a second (10.24, 11.08, 10.84 and 10.68 N mm) and the card's rate exponent is -0.009; at 100 degrees a second it was 11.66, the card missed it by 13 percent, and the card answers there with that miss stated. Nothing was measured below 5 or above 100 degrees a second, and there a frame carries FLAG_BEYOND_COMPARED. On three of the other fingers the slip torque rose by 60 to 80 percent from 5 to 100 degrees a second. This finger and P01 have the lowest friction of the seven. The twist at full slip is the card's weakest output: 39 to 40 degrees at every load and rate, against a measured 19 to 49 degrees over this finger's nine conditions; inside the range it is 39 to 40 against a measured 44 to 49 degrees, and between fingers it spreads by 11 to 17 degrees. FLAG_FULL_SLIP marks that event, the one the card exists to find, and is not a warning. FLAG_BEYOND_COMPARED marks a frame beyond the forces, the rates and the twist that were compared. The stuck fraction is a second fitted curve of the same form as the torque's, with its own characteristic twist; a bed of springs on a rigid finger would have one; the curve missed its mark at seven of the nine conditions. The stuck fraction and the torque were never measured in one trial. The paper (J. R. Soc. Interface 20: 20220809, sections 2.2.1 and 4.3.1, read in its open text) says that the tangential force of the first experiment, the one with the camera, was corrupted, that the torque was acquired in a second experiment with one centred sensor and no camera, and that \"the experiments were conducted on separate days\", so that \"the moisture of the participant's finger could have impacted the friction\". The card's torque is fitted to the second experiment and its stuck fraction to the first; an earlier text of this record said the two were taken in the same session, and that was wrong. The trial key of the second experiment numbers this finger's 110 trials in one sequence with the block at 2 N last, trials 41 to 110; that the sequence was one sitting is this record's reading of the key, and the paper does not say so. The twist was compared from 1 to 75 degrees. The plate turned 80 degrees in every trial: at 2 N it held its rate to 77.0 degrees at 50 degrees a second and to 78.9, 79.4 and 79.7 degrees at 20, 10 and 5, and then slowed to a stop at 80, with the torque falling. Past 75 degrees the card goes on with the same bed of springs under the held rate, its torque rising toward its slip limit and its heat growing with the twist, and each frame carries FLAG_BEYOND_COMPARED: the twist from 75 to 80 degrees was never compared, and the measured plate stopped at 80. Engines 0.7 to 0.9 refused a step past 75 degrees, and the build of engine 0.6.0 answered to 80. The direction of the twist is not an input. The overshoot of the torque before it settles, a few percent, and the shrinking of the contact area during the twist are left out. No heat and no stored energy was measured: the ledger of work, stored energy and heat is that of the model's bed of springs and closes for any value of the eight constants, so it can catch a wrong stepping rule or a corrupted step and cannot catch a wrong constant. Its scale is the larger of 0.0508 J, the scale of the build that was scored (the law's slip-torque limit at 10 N and 5 degrees a second over the whole turn of 80 degrees), and the plate work of the run: the work of a twist to 75 degrees at 2.07 N and 50 degrees a second is 0.0126 J, so on every twist at 2 N to 75 degrees the fixed energy is the scale, and a heavier load or a longer twist is held to the same relative closure. A step that turns the plate by more than 0.05 of the characteristic twist of the torque is refused as TwistStepAboveQuadrature: the ledger sees only the difference of the work and the heat, and the card holds the step at which the four-point rule sums each of them exact to rounding. A force of zero or less is refused as NoContact, and a rate of zero or less as TwistRateNotPositive. The slip torque and the full-slip twist are this record's own reductions of the published per-frame files. The slip-torque window here is the frames whose twist lies within rate x 0.5 s below 75 degrees; the authors' is the 0.5 s before 75 degrees. The two agree to 0.02 percent from 5 to 50 degrees a second and differ by 0.67 percent at 100. The authors' own per-trial summaries, not used in the scoring, have since been compared, by the second scorer and again by the builder: on this finger's 90 centred trials the values here agree with theirs to 0.04 percent in the condition means at 5 to 50 degrees a second. The raw 1 kHz files were not used. The values given here are this record's reductions of the authors' data (CC BY 4.0), not the authors' own figures, except where the paper is quoted. A run is one twist from zero under one load. A run whose later segment changes the force or the rate is refused whole, with no proof, as LoadChangedDuringTwist, and every run and every single step starts untwisted, so that no request can take up a twist under another load: a start at any twist but zero is refused as TwistNotStartedFromZero. The card that was scored answered a start at any twist under any load. Every trial of all seven fingers in the archive has now been opened, so a card rebuilt on this finger could not be tested on it. One party wrote the card and ran it; it was scored twice, the second time by a second scorer, on one machine.";

/// The result of the tokamak's validation record.
/// The one-line summary of the JET tokamak, as the list shows it.
pub(crate) const TOKAMAK_SUMMARY: &str = "The plasma of the JET tokamak in an ELMy H-mode pulse of 1996 to 1997, deuterium or deuterium-tritium: its thermal energy, its fast-ion energy and its D-T fusion power under the heating power. Compared with the nine JET pulses of its regime, 1.65 to 3.84 MA, 1.80 to 4.04 T, up to 24.3 MW of heating, heated windows up to 5.475 s and a tritium fraction of 0 to 0.034 or 0.451 to 0.504; outside that envelope it computes the same equations, with nothing compared, up to the Greenwald density, an edge safety factor of 2, the Troyon limit and the span of the reactivity fit. Compared on 2026-10-09 with two held-back JET pulses, and by the rule fixed beforehand the comparison failed: on 42982 the fusion energy released was 0.743 of the measured, under the 0.75 mark.";

const TOKAMAK_RESULT: &str = "Compared with the two held-back pulses on 2026-10-09, and by the rule fixed beforehand the comparison failed: on the primary pulse, 42982, three of the four marks were met and the fusion-energy mark was missed. Pulse 42982, 219 samples over 5.475 s: thermal energy error 7.9 percent of its peak against a mark of 14.5; fast-ion energy error 22.5 percent against 25; fusion power error 23.4 percent against 25; fusion energy released 18.4 MJ against 24.8 MJ, a ratio of 0.743 against a mark of 0.75 to 1.25. The card's fusion power is too low throughout: its peak is 3.7 MW against 6.1 MW. Pulse 42976, the hot-ion pulse declared outside the card before it was run, missed all four: thermal energy 19.2 percent, fast-ion energy 25.1 percent, fusion power 34.7 percent, and fusion energy 5.4 MJ against 10.3 MJ, with a peak fusion power of 5.1 MW against 14.4 MW.";

/// The lines of the accumulator's validation record, fixed before a held-back
/// file was downloaded.
const ACCUMULATOR_SBO450_PASS_LINE: &str = "Fixed before any held-back file was downloaded (protocol SHA-256 721d88a5264b0e15342824290e6305db1df27fab76011333b49d5ed253b38323). For each held-back run the card is driven by a sine of the measured amplitude from the cyclic steady state at the measured mean pressure, and two numbers are compared: the stiffness, which is the swing of the gas pressure per unit of piston travel, and the lead of the pressure over the piston. Stiffness error: median at most 2.0 percent over the 285 runs and in each of the five records, and at most 5.0 percent in any run. Lead error: median at most 0.5 degrees, and at most 3.0 degrees in any run. And the card's two medians lower than those of five baselines: an isothermal and an adiabatic ideal gas, this model unfitted, the measured stiffness of the fitted condition scaled by the ideal-gas law, and this model with the ideal-gas law fitted to the same runs. The mean pressure of a run is taken from the measurement and is not tested.";

/// The result of the accumulator's validation record.
pub(crate) const ACCUMULATOR_SBO450_RESULT: &str = "Compared with the 285 held-back runs on 2026-10-09, run once through the engine, and by the rule fixed beforehand the comparison failed: two of six lines were met. The card answered 282 runs and refused three, the 5 mm runs at 12, 15 and 18 Hz at 120 bar, whose pressure passes the card's ceiling of 130 bar as the measured pressure does. Stiffness error over all runs: 0.93 percent in the median against a line of 2.0, met. By record: 0.26 percent at 20 / 40 bar, 1.45 at 20 / 60, 0.68 and 0.33 at 40 / 80, and 2.72 at 40 / 120 against 2.0, missed. Largest: 13.96 percent against 5.0, missed; 27 runs are above 5 percent, all at 0.1 Hz or under. Lead error: 0.27 degrees in the median against 0.5, met; 2.82 degrees at most against 3.0 on the runs answered, and not met over all runs because three have no answer. Against the baselines: the card's median stiffness error is the lowest, 0.93 percent against 2.20 for the fitted condition's measurement scaled by the ideal-gas law, 2.46 for this model with the ideal-gas law, 3.50 for this model unfitted, 12.2 for an adiabatic and 37.2 for an isothermal ideal gas; its median lead error is not the lowest, 0.27 degrees against 0.12, 0.15 and 0.25, missed. Above 1 Hz, 212 runs: stiffness within 0.59 percent in the median and 3.49 at most, lead within 0.18 and 0.99 degrees; the two ideal-gas baselines are at 2.2 and 2.4 percent there. At 0.05 Hz and under, 30 runs: the card is too stiff by 7.7 percent on average and 7.5 in the median, and the scaled measurement, 2.4 percent, and the fitted ideal gas, 3.2, are closer. The median lead line is met by every baseline as well, an ideal gas with no heat exchange among them at 0.30 degrees, so it is no evidence for the card's exchange of heat. On the 114 opened runs the stiffness error was 0.45 percent in the median and 3.17 at most and the lead error 0.17 and 2.20 degrees; two records of one condition differ in measured stiffness by 0.87 percent in the median. The binary that made the runs has SHA-256 a3d268ca094ba37d1edc91eb3e46d9748deb365d48b9295453e1fa45a8235428. Record: evidence/research/accumulator_darmstadt_sbo450/RESULTS.md. That is the registered result and it stands; the card it describes is the card as it was then, with a pressure range of 30 to 130 bar, a piston area of 19.2157 cm2 and a vessel of 0.57622 l. Second scoring, 2026-10-09, by another party (evidence/verification/second_party/V-39/): all 399 files reduced anew, all 798 requests rebuilt and asked again of the same binary, every proof verified; every number above was reproduced and the verdict stands. Amendment, 2026-10-09, made after every one of the 399 runs had been opened. It is not a new test and no line above is changed by it: the geometry was corrected, the range was drawn in to where the card held its marks, and the card refuses by name outside it. Run after the fact through the amended card (binary SHA-256 92e33dfeaacb2c2fceb0bed273e24424ed13d16fa68592c107e70c3171fbe56e): of the 285 formerly held-back runs it answers 196 and refuses 89, the 57 runs at 40 / 120 bar as PressureOutsideCard before the first step and the 32 runs at 0.1 Hz and under of the other four records as HeatShareAboveCard after 9 to 414 of 800 steps. Every run at 0.2 Hz and faster of the four records at 37 to 86 bar is answered. The pressure bands of each charge were added after this run, drawn by the same whole-bar rule from the measured pressures of those four records: on a transcription of the card's equations that a later recomputation found equal to the engine's numbers in the three runs it compared (evidence/verification/fresh_eyes_0.6.0/accumulator_sbo450/), every one of the 196 held-back runs and the 98 opened runs answered here stays inside the bands of its charge at every step, so none of the numbers that follow changes. On the 196 answered, worked as the six lines were: stiffness error 0.49 percent in the median; by record 0.21 at 20 / 40 bar, 1.37 at 20 / 60, 0.60 and 0.26 at 40 / 80; 4.05 percent at most; lead error 0.21 degrees in the median and 1.79 at most; the median stiffness error is lower than every baseline's on the same runs, 2.06 percent for the best of them, and the median lead error is not, two baselines being at 0.10 and 0.11 degrees. None of these is a line met: every run was open when the range was drawn, and 89 runs have no answer. Of the 255 held-back runs that were within 5 percent and 3 degrees in the registered run, 59 are no longer answered, 46 at 40 / 120 bar and 13 at 0.1 Hz and under; none of the 27 that were above 5 percent is answered. Of the 114 opened runs 98 are answered, 0.42 percent in the median and 1.26 at most. On every step both binaries landed, 479,346 frames of 684 runs, the gas pressure and the gas temperature are the registered run's to the last bit of the 32-bit slot, and within 4e-15 in the 64-bit state at the end; gas volume, gas mass, force, work and heat are 1.021818 times the registered ones, and the flags are zero. Record: evidence/research/accumulator_darmstadt_sbo450/RESULTS.md under the heading of the amendment, and amendment_2026-10-09.md, SHA-256 ba073e2a1c070aa1282bcea8b330b765a833c53b0ab63367a61fb46722a68bb7.";

/// The limits of the accumulator's validation record.
const ACCUMULATOR_SBO450_LIMITS: &str = "The card was scored, failed, was scored again by a second party, and was amended afterwards with every run opened. What the amended card does on the measured runs is stated after the fact. It is not a new test. What the card gives: the swing of the gas pressure per unit of piston travel, for the two charges measured, each at the gas pressures it was measured at, and motion quick enough that the heat exchanged with the wall is a small share of the work. Through engine 0.9.0 it refused by name any other gas mass, a pressure under 37 or over 86 bar, a pressure the charge given was never measured at, a wall outside 293.4 to 293.7 K, a gas outside 285.9 to 301.8 K and a piston faster than 0.7 m/s. Those are now the edges of the range compared with measurement, and a step outside them lands with FLAG_OUTSIDE_COMPARED_RUNS: 85.56 bar is the highest pressure measured in the four conditions in which the card met its marks; the record at 109 to 132 bar, 40 bar precharge and 120 bar load, was compared by the card as scored and missed, 2.72 percent median error of the swing against the line of 2.0, and three runs above 130 bar went unanswered; between 86 and 109 bar and beyond 132 bar there is no measurement; the 20 bar charge was measured at 37.76 to 42.71 and 55.26 to 66.37 bar and the 40 bar charge at 57.36 to 63.14 and 75.31 to 85.55 bar; the wall of every compared run was 293.46 to 293.60 K; the gas reached 285.97 to 301.78 K; the fastest compared run reached 0.69999 m/s as the mean over a step. What the card refuses by name is physical: a gas mass of zero or less (GasMassNotPositive), a wall at or below absolute zero (WallTemperatureNotPositive), a gas at or below nitrogen's critical temperature, 126.192 K, where it can condense (GasBelowCriticalTemperature), a pressure above the vessel's rated 330 bar (PressureAboveRated: HYDAC's SBO catalogue rates the type SBO450-0.6 at 4700 psi, 330 bar, in its dimension table, and the two 0.6 l parts it lists at 3600 psi, 250 bar; which the rig carried is not on disk, and between 250 and 330 bar a step lands with FLAG_OUTSIDE_COMPARED_RUNS like any step past the compared range), a gas volume past the vessel's with no oil left (OilSideEmpty), and a motion in which the heat passes 0.065 of the work (HeatShareAboveCard). The card is never told a frequency, so slow motion is refused on what the card carries. M is the sum over the steps of the motion so far of the absolute work of each step and H the same sum of the heat; a start carries in the sums of the motion before it as work_moved_j and heat_moved_j, so a motion carried on from a returned state, by runs or by single steps, is weighed as one run; a step lands only if H is at most 0.065 of the larger of M and 100 J. Over one period of a sine the share is 0.05 at 0.2 Hz, 0.10 at 0.1 Hz and 0.8 at 0.01 Hz, nearly the same at every pressure, charge and amplitude measured. Run as they were scored, two periods from the cyclic steady state at the piston's outermost point, the measured runs at 0.2 Hz reach 0.062 at most and those at 0.1 Hz 0.104 at least; 0.065 is the next two-hundredth over what held, and nothing between 0.1 and 0.2 Hz was measured. The 100 J give the share a meaning before the oil has done work: it is a little over the work of a quarter of the largest measured swing inside the card, 91 J, and under the work of the smallest measured run, 125 J. A piston at rest with the gas at the wall's temperature exchanges nothing and may rest for any time. A piston at rest with the gas off the wall's temperature relaxes with the one time constant; no measurement here is of a rest, and the slow runs show that time constant wrong by a factor of two to three, so from the start of a run such a rest is answered only for its first 6.5 J, about 0.3 K of the 40 bar charge. The sums run over the whole motion, so a rest that follows motion is weighed against all the work before it and is refused later than the same rest alone, or not at all; a share over a trailing window of work would weigh it alone, and it has not been compared with the measured runs. The card cannot see motion before its start: it holds the sums a start states and cannot confirm them. Two constants are fitted to this accumulator's own runs at one condition, 40 bar precharge and 60 bar load: the ratio of the piston area to the vessel's gas volume, 3.3348 per metre, and the thermal time constant, 17.52 s. The gas pressure fixes that ratio and neither number alone. The piston area is that of a 50 mm bore, 19.635 cm2, which no source prints: over the seven records the measured force is 19.569 cm2 times the oil gauge pressure less 211 N, a bore of 49.92 mm, and every file's metadata give the force transducer an offset of -0.202 kN. At that area the fitted ratio is a vessel of 0.58879 l, where the catalogue gives 0.6 l and 36 cubic inches, which is 0.58993 l. The card as it was scored held 19.2157 cm2 and 0.57622 l: force over oil pressure at one pressure with the offset in it, 2.1 percent low. Its pressure and temperature were not affected; its gas volume, gas mass, force, work and heat were 2.1 percent low. With the 50 mm bore, the 36 cubic inches and no fitted geometry the same model's median stiffness error on the held-back runs is 0.72 percent, against 0.92 with the fitted ratio. A third fitted number belongs to the measurement and not to the card: the deflection reading is taken as a lag of 1.52 ms behind the piston, and the measured stiffness and lead are stated with that correction. Fitted to each record alone the lag is 1.47 to 1.53 ms in all seven: it is a property of the acquisition, in which the deflection passes through the test machine's own conditioner and the pressures do not. Above 2 Hz the lead lines compare that fitted lag with a model lead of under 0.1 degrees and test nothing: every baseline meets them. At 0.5 Hz and under, where the lead says something about the exchange of heat, the card's median lead error on the held-back runs is 1.24 degrees, and two baselines are closer, the scaled measurement at 0.94 and the fitted ideal gas at 0.89. The held-back runs are at load pressures of 40, 80 and 120 bar and at half the gas mass: extrapolation in pressure and in charge, at the same frequencies and amplitudes as the fitted runs, on the same accumulator, rig, gas and room temperature. Nothing here tests another vessel, another gas, another temperature, a motion that is not a sine, a rest, or the mean pressure, which is taken from each run. The precharge is absolute: the metadata give both pressure sensors a range of 1 to 201 bar, and the records at 20 bar precharge agree with the card read that way and not as gauge. The exchange of heat is one time constant, and that form fails here. Fitted afterwards to each record's own runs it is 17 to 18 s at 40 / 60 bar, 13 s at 40 / 80, 9.3 s at 40 / 120, 10.6 s at 20 / 40 and 8.1 s at 20 / 60, about 45 s per litre of gas volume. A time constant proportional to the gas volume, fitted on the one opened condition, would still have failed the comparison: 2.56 percent in the worst record, 6.73 percent at most and 4.03 degrees at most. Within one record the time constant that each frequency asks for falls threefold, from 20 s at 0.01 Hz to 6.6 s at 1 Hz at 40 / 60 bar. The group that made these measurements has published the same of two other HYDAC diaphragm accumulators, SBO500-0,1 and SBO330-0,75, at 20 and 40 bar precharge: \"Newton's law is incapable of representing this physical phenomenon\" and \"Experimental data confirm the failure of Newton's approach\" (Hartig, Depp, Rexer and Pelz, Effects of unsteady heat transfer on behaviour of commercial hydro-pneumatic accumulators, arXiv:2012.06526). So at 0.1 Hz and under the card as scored was up to 14 percent too stiff, and there the amended card refuses. The miss at 40 / 120 bar is not the equation of state. Against the NIST WebBook isotherm at 293.15 K the Peng-Robinson density is 0.5 percent high at 20 bar and 1.9 at 120, and its isentropic stiffness 0.2 to 1.8 percent low; in the card these make the charge 0.8 percent softer at 40 / 120 bar than the reference equation would, so with the reference equation the card would be further off there, 2.9 percent too stiff above 1 Hz against 2.1. What the card leaves out is the give of the oil side, in series with the gas: the second scoring found that one series compliance, fitted afterwards to all seven records, brings every record within 0.33 percent above 1 Hz. That is a lead for another card and is not in this one. Two records of one condition differ in measured stiffness by 0.87 percent in the median, most of it a difference of mean pressure between two fills: with that removed through the model it is 0.35 percent. The unfitted baseline takes its time constant from Rotthäuser's correlation, which the source the second scoring found for it gives for piston accumulators: a weak comparison for a diaphragm accumulator. The gas temperature sensor reads a fifth of the card's swing at 0.01 Hz and nothing above 1 Hz; its response time is not stated and the temperature was not scored. The oil, the diaphragm and the line are left out. This is a closed gas spring: no orifice, no flow of gas, and choked flow does not arise. The ledger closes for any time constant and any vessel volume: it shows that the pressure, the heat capacity and the energy are one equation of state and that the step integrates them, and it cannot catch a wrong constant. The gas mass is a number the step never changes. One party wrote the card, ran it and scored it once, on one machine. A second party scored it again from the protocol (V-39) and found the score right. The amendment and its run after the fact are the first party's and have not been scored by a second party.";

/// One validation record: the object, the data, the pass line, the result and
/// the limits. Fitted cards carry their rows of the claims file; a generic
/// specimen says that it was not compared with a measurement.
fn validation(card: &str, specimen: &str) -> Value {
    let parts: [String; 5] = match card {
        "los_jwst_fine_guide" => [
            "The James Webb Space Telescope holding a guide star in fine guide: the offset of the star's centroid from the lock point on the two axes of the Fine Guidance Sensor's ideal frame, every 64 ms, in quiet stretches.".into(),
            format!(
                "Space Telescope Science Institute, Mikulski Archive for Space Telescopes, programme 1163, OTE-26 Routine WFSC/CC Jitter Maintenance: the table FGS CENTROID PACKET of each calibrated fine-guide file, public. The centroid is the flight software's. The card's eighteen constants were fitted to the 127 files of the 19 visits that started in April 2022: 936,273 frames, six guide stars. The nine constants of each axis, x then y: the slow part {} and {} mas with a memory of {} and {} s; the fast part {} and {} mas with a memory of {} and {} s; the mode {} and {} mas at {} and {} Hz with a memory of {} and {} s; and the noise variance of one centroid, {} and {} mas2 at 1,000,000 counts per second, falling as the count rate to the power {} and {}. The 134 files of the 24 visits that started from 2022-05-02 to 2022-06-21 were held back from that fitting: other dates, 100 of the 134 files from the other guider, and 19 guide stars of which 18 were never fitted. The acknowledgment the institute asks for: \"This work is based in part on observations made with the NASA/ESA/CSA James Webb Space Telescope. The data were obtained from the Mikulski Archive for Space Telescopes at the Space Telescope Science Institute, which is operated by the Association of Universities for Research in Astronomy, Inc., under NASA contract NAS5-03127 for JWST. These observations are associated with program #1163.\"",
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
            "Fixed before any held-back file was downloaded (pre-registration SHA-256 9e160724092e182f847eb0103e6ca7a54aba0f75c5d1ca14dcd275c9a554c8f5), on the quiet frames of the held-back files, for each axis: the error of the prediction one frame ahead at most 0.85 of the error of taking the last centroid and at most 0.85 of the error of taking the lock point; the mean of the squared error over the variance the card states between 0.67 and 1.50; the correlation of that normalised error from one frame to the next at most 0.15 in size; and that mean between 0.5 and 2.0 in at least 80 percent of the files with 500 quiet frames or more.".into(),
            LOS_RESULT.into(),
            "A forecast is not a result. The forecast, made outside the engine with constants fitted on the first nine April visits and scored on the other ten: 0.774 and 0.785 against the last centroid on x and y, 0.779 and 0.636 against the lock point, a mean normalised squared error of 0.680 and 0.938, a frame-to-frame correlation of 0.113 and 0.067, and 84 and 100 percent of files. The constants are fitted to this observatory's own centroids, so the held-back visits test extrapolation in time and to other guide stars and nothing else: one observatory, one season, one guiding mode. In brightness they are interpolation: their count rates, 62,000 to 816,000 counts per second, lie inside the fitted 22,900 to 2,820,000, so the noise law was not tested at either end of the card's range. One party wrote the card and ran it. It was scored twice, the second time by a second scorer, on one machine. The prediction is one frame, 64 ms, ahead. The estimate of the line of sight with the noise taken out is a model output that no measurement here tests. The lock point is not in the file and is the caller's; the validation took the median of the run's good centroids, which uses the whole run. The lock points of the files the comparison answered span -61.1512947 to 60.4564934 arcsec on x and -64.2767 to 50.1812668 arcsec on y, fitted and held-back files together, and the card computes a lock point outside that span up to the guider's field, 70 arcsec from its centre, past which it refuses as LockOutsideSensorField; no lock point nearer the edge of the sensor's field was compared. The longest run of frames with no measurement that a run of the comparison crossed was 10 frames on the fitted files and 2 on the held-back files, and the card predicts any longer run of them, its stated variance growing toward its unmeasured value; none longer was compared. Nothing above 7.8 Hz is resolved by a 64 ms frame: the Nyquist frequency is 7.81 Hz, and the card cannot see or state jitter above it, nor tell a folded line from a true one. The card describes quiet fine guide: inside a disturbance, a centroid more than 5 mas from the lock point and the 30 s after it, the filter follows slowly, and on the fitted visits as on the held-back ones its error was larger than that of taking the last centroid. The card answers there and flags the frames, FLAG_POINTING_DISTURBED, FLAG_BEYOND_FOUR_SIGMA and FLAG_RECOVERING; an agent that needs the line of sight to the quiet frames' accuracy must not use the flagged ones. It counts the 30 s after a disturbance from the caller's statement at the start of a run, which it cannot confirm; and a caller's lock point that is off by a few mas makes quiet frames read as disturbed, so the card then flags frames that are quiet. It says nothing of the line of sight during a disturbance: the visits of March 2022 are unopened and kept for a card that follows one. The y axis shows lines near 0.51, 1.02 and 1.53 Hz that the model has no part for. In the held-back spectra the y feature is a narrow line at 0.290 to 0.294 Hz with a shoulder at 0.32 to 0.34 Hz; the card's 0.3025 Hz is one damped mode laid across both. The commissioning report of the telescope (Rigby et al., arXiv:2207.05632, section 3) names a 0.3 Hz oscillation of a vibration damper between the telescope and the spacecraft bus and a 0.045 Hz oscillation attributed to fuel slosh; the card has no part for the 0.045 Hz one. The x feature is a broad hump at 0.50 to 0.60 Hz, for which no published attribution was found. The ledger is an account of variance that follows from the model; it is not a conservation law, and it cannot detect a wrong constant. It confirms that each update is a Kalman update with the optimal gain: it passes with a wrong mode frequency, wrong noise or the two axes swapped, and it says nothing about the telescope.".into(),
        ],
        "cell_18650pf" => [
            "One Panasonic NCR18650PF lithium-ion cell, new, at 25 degrees Celsius, discharged from full charge at a held current of 1C.".into(),
            "Kollmeyer (2018), Mendeley Data, doi 10.17632/wykht8y7tg.1, CC BY 4.0: the laboratory's tests of that cell. The card's constants were taken from its low-rate and pulse tests. Eleven 1C discharges were held back from that fitting.".into(),
            "Fixed before any result was seen: terminal voltage within 50 millivolts and delivered capacity within 3 percent.".into(),
            "Eleven of eleven discharges met the line. The voltage error was 27.8 to 30.0 millivolts root mean square (25.7 to 32.4 across the three onset times tried), and the largest single error was 57 millivolts. On the one test that ran to cutoff the delivered capacity differed by +0.04 percent at steps of 0.1 s and by +0.014 percent at the protocol's step of 1 s in the second scoring, with the discharge ended by the card's own cutoff.".into(),
            "One cell, new, at 25 degrees Celsius, at a held current of 1C. The comparison that passed covered discharges from full charge only, at 1C only, 2.899 to 2.89982 A. The 1C span that passed is 2.89817 to 2.90065 A, the measured span widened by the card's 0.83 mA band; through engine 0.9.0 the card refused any other current, and now it takes any held discharge current above zero and flags a step outside that span as FLAG_CURRENT_OUTSIDE_PASSED_SPAN. The dataset's other held currents are the C/20 test (0.145 A), which set the rest-voltage curve, and the 0.87 A holds of the pulse test, which set the resistance; both are fitting data and neither is a test of the card. One discharge at another current was left out of the fitting: 04-27-17 3541_dis5_10p.mat, the same cell at 25 degrees Celsius discharged at a held 0.86888 to 0.86970 A from full charge to 2.5 V in 14 holds with rests at 0 A between them. Its totals and its voltage trace had been read before it was compared; no output of the card had. Compared on 2026-10-10, each hold run as a continuation of the one before and the rests not compared, at steps of 0.1 s, it failed: the voltage error during the holds was 56.7 millivolts root mean square against the line of 50, rising to 204 and 206 in the last two holds, and the card did not end by its own cutoff but left its table at soc 0.03 and 2.66 V (StateOfChargeOutsideCard), 71 s after the cell reached 2.5 V, having delivered 0.60 percent more than the cell. 0.87 A is answered with that miss stated: evidence/research/panasonic_18650pf/held_0p87A_comparison. A start below full charge is taken only as the continuation of a 1C discharge, whose held current the request states; through engine 0.6.0 any start from 0.03 to 1 was accepted at any current. A current that changes during a run is refused as CurrentChangedDuringDischarge; through engine 0.5.0 it was accepted. Not a drive cycle, another temperature or a pack; the twelve-cell pack card is a different card and is not validated by this. The capacity total of the full discharge had been seen before the comparison; the voltage traces had not. The card has no temperature and no memory. Run by one party and scored twice, the second time by a second party on 2026-10-07, with the same verdicts on all eleven discharges: evidence/verification/second_party/V-07.".into(),
        ],
        "capsule_stardust" => [
            "The Stardust sample return capsule on its entry of 15 January 2006, from entry interface to parachute opening.".into(),
            "NASA's reconstruction of the entry (peak load 32.89 g) and the radar record of the altitude and speed at parachute opening. The card is fixed to the capsule's pre-flight design and navigation values and to published standards (U.S. Standard Atmosphere 1976, World Geodetic System 1984).".into(),
            "Fixed before any trajectory was computed: peak load within 3.64 g of 32.89 g, and the altitude and the Mach number of parachute opening each reached within 7.4 s of the recorded time. The lines are NASA's printed three-sigma pre-entry spreads.".into(),
            "All three lines met. Peak load 32.17 g, 2.2 percent under 32.89 g. The altitude of 31.03 km was reached at 138.909 s and Mach 1.23 at 136.953 s, against 137.9 s recorded.".into(),
            "The comparison is not blind: 32.89 g was published and known to everyone involved when the marks were fixed, and nothing was held back unseen. It is NASA's own fitted simulation, not an instrument reading; no accelerometer flew, and the two times rest on range radar. The structure used, E, was recommended after the forecast's scores on the ladder of structures were seen; the rule fixed in advance named a simpler one, B1, which passes the peak-load line by 0.12 g and fails it under 8 of the 25 input changes tried on it. The air is a yearly standard atmosphere, not the air of that morning, which is worth about 1 g either way. One entry, down to parachute opening only. Heat flux is not validated: the card gives 696 W/cm2 against NASA's computed 762 to 1000. The run repeats a forecast made outside the engine, so it is not a new test of that forecast; a second capsule would be. Nothing past the later of the two deployment crossings was compared, and the card refuses a state there. Run once and scored twice. Repeated on 2026-10-07 by Grok, another developer's model, with its own comparison written from the protocol: the same three numbers and the same run proof. It had read the headline numbers first, and a directory search showed it lines of the first agent's comparison scripts before its run. Grok's other sessions write the engine, and no other organisation has repeated this result.".into(),
        ],
        "capsule_insight" => [
            "NASA's InSight entry capsule on its entry at Mars of 26 November 2018, from the first sample of the public flight record, 126.5 km up, to parachute deployment 209.3 s later.".into(),
            "The flight's own inertial record, 200 samples a second, and the path added up from it: NASA Planetary Data System bundle insight_edl (Karatekin and Van Hove, Royal Observatory of Belgium). The card's air is the temperature of seven Mars Climate Sounder profiles (PDS volume MROM_2147) set on the surface pressure from the lander's barometer (NTRS 20200003204); its drag is the 2008 Phoenix aerodynamic database (NTRS 20080034648). No number in the card comes from the accelerometer. The first 150 s of the record were opened and used to diagnose a first forecast; the last 59 s before the parachute were held back unread.".into(),
            CAPSULE_INSIGHT_PASS_LINE.into(),
            CAPSULE_INSIGHT_RESULT.into(),
            CAPSULE_INSIGHT_LIMITS.into(),
        ],
        "generator_mmrtg_f1" => [
            "One radioisotope generator, flight unit F1, on NASA's Curiosity rover: its daily-average electrical output.".into(),
            "Whiting and Woerner, Historical RTG Performance Data, 2023 update, Dryad, doi 10.5061/dryad.1zcrjdfw2, CC0: the daily-average electrical output on mission days 0 to 4,087, each as a fraction of 115.4 W, the highest daily average of the first 30 days. Three constants were fitted to the first 2,061 days, mission days 0 to 2,060. Days 2,061 to 2,808, 748 days, were used to choose the card's structure. Days 2,809 to 4,087, 1,279 days, were held back from both and are the days the card was compared with.".into(),
            "Two lines, fixed before any structure was fitted, each 0.010 of 115.4 W, which is 1.154 W: over the held-back days, 2,809 to 4,087, the root mean square error within the line, and the mean error over the final 365 days within the line. Both had to be met.".into(),
            "One line met and one missed, so by the rule fixed beforehand the comparison failed. Compared with the 1,279 held-back days, 2,809 to 4,087, on 2026-10-06, in one run of 4,087 daily steps from landing; an error is the card less the measurement, as a fraction of 115.4 W. The root mean square error was 0.008129, about 0.94 W, inside the line of 0.010. The mean error over the final 365 days was +0.010094, about 1.16 W high, outside the line by 0.0000936, about 0.011 W. The largest error on one day was +0.02251, 2.60 W high, on mission day 3,546.".into(),
            "A miss, and it stays on the record. One generator; the daily-average electrical output only; the record holds no heat measurement. The card reads high on the held-back days and the gap grows: its mean error was +0.0031, +0.0045 and +0.0096 of 115.4 W over three successive spans of 365 days and +0.0107 over the last 184, about 0.36, 0.52, 1.11 and 1.23 W. Days 2,061 to 2,808 were used to choose the card's structure and are not an independent test of it. A different structure would have passed the held-back days, which was seen after they were opened and is information, not a result. The card serves mission days 2,809 to 4,087, the days it was scored on, as the model that was scored, with no constant changed since: a card that failed its comparison, served with the failure stated. Mission day 4,087 is the last day of the published record; the card carries the same exact map past it and flags each later step as FLAG_PAST_RECORD, and since it read high and the gap grew each year, later days are expected to read higher still. A corrected card needs days no model has seen: the record after October 2023. Run by one party and scored twice by it, the second time with a separate script. Repeated by a second party, Grok, on 2026-10-07, with its own comparison written from the protocol and run on the cut engine of version 0.2 on the same machine: the same root mean square error, 0.008129, the same final-year mean, +0.010094, the same run proof and the same miss (evidence/verification/second_party/V-11). Grok is also the SI whose other sessions write the engine, and it had read the record's numbers before it repeated them.".into(),
        ],
        "link_gothenburg_10010" => [
            "One commercial microwave link in Gothenburg, Sweden: link 10010, sublink 587, of the OpenMRG data set, 32.445 GHz, vertical polarisation, 1.25841 km, a 0.3 m antenna at each end, sent level 5 dBm, with the weighing rain gauge at Barlastplatsen 37 m from the path.".into(),
            "Andersson, Olsson, van de Beek, Hansryd, Andersson and Persson, The OpenMRG data set, Zenodo, doi 10.5281/zenodo.7107689, CC BY-SA 4.0: the sent and the received level every 10 s and the gauge every minute, June to August 2015. The rain term is Recommendation ITU-R P.838-3 at this frequency and polarisation, with nothing fitted: k = 0.27282714286921533 and alpha = 0.8947805998167349 at 32.445 GHz, vertical. The three constants of the water on the antennas were fitted to this link's own readings in the 18 rain events of June and July: a ceiling of 2.13 dB, a wetting time of 81 s and a drying time of 404 s. Every reading of August was held back and was not opened until the protocol's fingerprint was recorded.".into(),
            "Fixed before any reading of August was opened (pre-registration SHA-256 f60bfcc70553f95fef5d35963b6550120de3c67e712c057e40c5d37c51784625), on five-minute means of the attenuation above the dry level over every August rain event of at least 1 mm: root mean square error at most 0.90 dB, at most 0.80 of the error of P.838 alone on the same blocks, and the summed attenuation between 0.75 and 1.25 of the measured sum.".into(),
            LINK_RESULT.into(),
            "A forecast is not a result. On the 18 events the constants were fitted to, the card's error is 0.667 dB, against 1.028 dB for P.838 alone and 1.589 dB for no attenuation; with each event left out of the fit in turn, 0.677 dB. The structure and the link were chosen on June and July. August tests new rain events on the same link in the same summer, inside the rain rates fitted: interpolation in rain rate, later in time. It does not test another link, frequency, polarisation or season, or snow. One link, one direction, one gauge at one end of the path: rain on the path that differs from rain at the gauge is not in the card, and the fitted event with the heaviest rain, 84 mm/h, has the largest error, 1.84 dB. The dry level is the caller's, read from the link itself before each event, and the result depends on it: in the comparison it was the median of the hour before each event; one receiver step, 0.3 dB, of error in it puts the summed attenuation outside its band, at 1.69 of the measured sum, and a single dry level for the whole month, 52.30 dB, gives 0.644. An agent that has only a rain forecast, and no reading of the link's level just before the rain, does not have what this comparison used. An agent sizing a fade margin must not take the card's peak as the measured peak: the card's five-minute peak was the lower in seven of the nine events, by up to 2.2 dB, and its one-minute peak was the lower in six, by up to 2.8 dB, and the higher in three, by up to 3.3 dB. The dry level's median, 52.30 dB, is 7.98 dB above free space between two ideal 0.3 m apertures (124.667 dB less 2 x 40.172 dB); the data set gives no antenna gain or feeder loss, so that difference is not divided. The received power in watts and the energies are the card's arithmetic on the levels; no energy was compared with a measurement. The comparison fed only one-minute gauge rates of 0 or multiples of 6 mm/h, held through whole minutes in steps of 60 s from dry antennas. The card computes past all of these, with the map and the ledgers exact: a rate above 84 mm/h, a rate above 0 and below 6 mm/h, rain changed inside a gauge minute, a step longer than 60 s and a dry level outside the fitted 51.75 to 52.90 dB; none of those was compared with a measurement.".into(),
        ],
        "reactor_spert3" => [
            "The SPERT III E-core, a small light-water test reactor, in its cold-startup reactivity-insertion tests: primary coolant at 56 to 92 degrees Fahrenheit, no flow, 5e-5 MW at the start, reactivity inserted in a step.".into(),
            "IDO-17281 (Phillips Petroleum Company for the US Atomic Energy Commission, March 1969), Table III: 30 tests, each value with a stated standard deviation. Every third test by the order in which the tests were run was held back and sealed; the other 20 were opened. The card's one fitted constant was taken from the 20 opened tests. None of the report's measured values is stored in the card. The ten held-back tests span 0.890 to 1.128 dollars, 4.3 to 110 MW at the peak and 56 to 82 degrees Fahrenheit. Outside that span the card was compared only with the opened tests its constant was fitted on, in sample and each left out of the fit in turn: test 22 below it, tests 42 and 43 above it. Tests 22 and 43 set the ends of the compared span, 0.769 to 1.210 dollars; outside it the card computes the same equations and nothing was compared.".into(),
            "Fixed before any model was run (protocol SHA-256 47458dc27bcd9a60fe6c16ac613b2be3e15b30c7e4bb75d7624c80707e4e7ccd). Peak power and energy released to the peak: the root mean square of the error, in units of the report's standard deviation, at most 1.5; the worst error at most 3; the mean error at most 0.5 in size. Time of the peak: the root mean square of the relative error at most 5 percent and the worst at most 10 percent.".into(),
            "Compared with the ten held-back tests on 2026-10-08, and by the rule fixed beforehand the comparison failed: five of the eight lines were met and three were missed. Peak power met all three of its lines: root mean square 0.58 of the report's standard deviation, worst 1.13, mean +0.29. Energy released to the peak met two and missed one: root mean square 1.01, worst 1.99, and a mean of -0.705 against a line of 0.5 in size: the card reads low. Time of the peak missed both of its lines: root mean square 23 percent against 5 and worst 29 percent against 10, every peak early.".into(),
            "The comparison is not blind: the whole of Table III, the ten held-back tests included, was read and transcribed on 2026-10-07 before the protocol was written, and no model was fitted to or scored on any test before its hash. No structure of the ladder fixed in advance met every line on the opened tests, so the rule named none; this card's structure was chosen on the opened tests after that, before the held-back run. One constant is fitted, the strength of the feedback with energy released, on the 20 opened tests. The delayed-neutron constants are the report's values stated for 500 degrees Fahrenheit, used here on cold tests; the report prints none for cold. A forecast made outside the engine on the opened tests, each left out of the fit in turn, met the peak-power and energy lines and missed the time-of-peak line, with peaks about 21 percent early. The card stops at the first power maximum: boiling, heat flow from fuel to water, hot startup and operating power are not on the card. The reactivity is the report's reactor period converted by the caller; the card takes dollars. Only the peak power, the energy released to the peak and the time of the peak were compared with the held-back tests. The states the card seals before the peak were compared only with digitised plots of 19 opened tests; on 18 of them the card reaches a tenth of its peak about 26 percent early. The card steps the reactivity in at 5e-5 MW; the report's rod ramped it in over 50 to 65 ms, and a fit to the plots puts the starting power near 9 W (evidence/research/reactor_spert3/analysis/10_peak_timing_cause.md). energy/reactor_spert3_fuel carries the ramp. Outside the compared span of reactivity, 0.769 to 1.210 dollars, and of coolant temperature, 56 to 92 degrees Fahrenheit, the card computes the same equations and nothing was compared; every tested burst was one step in reactivity, held to the first maximum, and a burst whose reactivity changes was not compared.".into(),
        ],
        "reactor_spert3_fuel" => [
            "The SPERT III E-core, a small light-water test reactor, in its low-initial-power reactivity-insertion tests in three bands of inlet temperature: 56 to 92, 251 to 259 and 498 to 501 degrees Fahrenheit, with no coolant flow in the cold band, 2.4 to 22 ft/s in the 251 to 259 degF band and 4.8 to 24 ft/s in the 498 to 501 degF band. The reactivity is ramped in as K t^2 by the transient rod; the fuel temperature and the water temperature are state, and the fuel feeds back by a square-root Doppler law.".into(),
            "IDO-17281 (Phillips Petroleum Company for the US Atomic Energy Commission, March 1969), OSTI 4792676, Table III (cold startup) and Table V (hot startup at about 260 and 500 degrees Fahrenheit): 58 scored tests, each value with a stated standard deviation. The card's four fitted constants were taken from 28 tests: the 20 cold-startup tests already opened and eight tests at 500 degrees Fahrenheit. 30 tests were held back from that fitting: ten cold-startup tests, seven at 498 to 501 degrees Fahrenheit, and the whole series the report names for about 260 degrees Fahrenheit, 13 tests at inlet temperatures of 251 to 259 degrees Fahrenheit, at which nothing in the card is fitted. None of the report's measured values is stored in the card.".into(),
            "Fixed on 2026-10-09 before any model was run on a hot-startup test and before this card was compared with any held-back test (protocol evidence/research/reactor_spert3/validation_protocol_v2.md; SHA-256 of its machine-readable copy 413ace16d5cc3d622c0ef44c0ea2b9526ac7526dfeee4b43ae601d6018e51db4), for each held-back group and for the 30 together; ten of the 30, the cold-startup tests, had been compared once with the first card of this reactor, on 2026-10-08. Peak power and energy released to the peak: the root mean square of the error, in units of the report's standard deviation, at most 1.5; the worst error at most 3; the mean error at most 0.5 in size. Time of the peak: the root mean square of the relative error at most 5 percent and the worst at most 10 percent.".into(),
            REACTOR_SPERT3_FUEL_RESULT.into(),
            REACTOR_SPERT3_FUEL_LIMITS.into(),
        ],
        "reactor_spert3_ccore" => [
            REACTOR_SPERT3_CCORE_WHAT.into(),
            REACTOR_SPERT3_CCORE_DATA.into(),
            REACTOR_SPERT3_CCORE_PASS_LINE.into(),
            REACTOR_SPERT3_CCORE_RESULT.into(),
            REACTOR_SPERT3_CCORE_LIMITS.into(),
        ],
        "runaway_18650_lco" => [
            RUNAWAY_18650_LCO_WHAT.into(),
            RUNAWAY_18650_LCO_DATA.into(),
            RUNAWAY_18650_LCO_PASS_LINE.into(),
            RUNAWAY_18650_LCO_RESULT.into(),
            RUNAWAY_18650_LCO_LIMITS.into(),
        ],
        "reactor_triga_jsi" => [
            "The TRIGA Mark II research reactor of the Jozef Stefan Institute, Ljubljana, pulsed by pneumatic ejection of its transient rod, with the 56-element core of standard fuel (12 wt% uranium, 20 percent enriched, U-ZrH) loaded as cores 215 (November 2014), 217 (March 2015) and 231 (March and April 2018): the power through the pulse and the seconds after it to the scram, the energy released, and the core-average fuel temperature that energy makes.".into(),
            REACTOR_TRIGA_JSI_DATA.into(),
            REACTOR_TRIGA_JSI_PASS_LINE.into(),
            REACTOR_TRIGA_JSI_RESULT.into(),
            REACTOR_TRIGA_JSI_LIMITS.into(),
        ],
        "cell_2170_m80" => [
            "One Tesla Model 3 2170 lithium-ion cell, cell m80 of the dataset, as it stood in each of six blocks of tests at chamber settings of 40, 25, 10, 0, -10 and -20 degrees Celsius: its terminal voltage, given the current and the temperature measured on its case. All six blocks are served. The -10 and -20 degree blocks met no mark in the comparison and are served with that miss stated and FLAG_BLOCK_MET_NO_MARK on each step. The same cell type is served as a physical model, cell_2170_model3: on one discharge, cell m80 from the recharge of its 10 degC block at 1.5 A for 600 s with the case at 12 degC, this card gives 3.934 V and that one 3.917 V, 17.4 mV apart.".into(),
            "Kollmeyer, Khanum, Naguib and Emadi, Tesla Model 3 2170 Li-ion Cell Dataset and Battery SOC Estimation Blind Modeling Tool, Borealis, doi 10.5683/SP3/ZVTR4B, version 3.0; and P. J. Kollmeyer, M. Naguib, F. Khanum and A. Emadi, A Blind Modeling Tool for Standardized Evaluation of Battery State of Charge Estimation Algorithms, 2022 IEEE ITEC, pp. 243-248, doi 10.1109/ITEC53557.2022.9813996. The card's 2,307 fitted numbers were taken from 17 characterisation files of that cell: the slow discharges and the pulse test of each block. 18 tests were held back and opened for a forecast outside the engine. 60 more were held back until the engine's card was compared with them: 12 discharges at a held current and 48 drive cycles.".into(),
            "Fixed before any constant was fitted (protocol SHA-256 488fe0af60bbb68be465106ae5fe368b7780f331e8ea5c87e8ac523e6ab984fa, with its Amendment 1). On each held-back discharge at a held current: voltage error at most 42 millivolts root mean square, delivered capacity within 3 percent, and the run ended by the card's own 2.8 V limit. On each held-back drive cycle: voltage error at most 42 millivolts root mean square, the card still running at the last sample, and the error at the last sample at most 126 millivolts. The same marks at all six temperatures, with a verdict for each temperature.".into(),
            "Compared with the 60 held-back tests on 2026-10-08, and by the rule fixed beforehand the comparison failed: 27 of 60 runs met their marks and no temperature met its verdict. The slow discharge, at a third of the rated current, met its marks at 40, 25, 10 and 0 degrees Celsius: voltage error 5.0, 6.4, 15.0 and 29.2 millivolts root mean square. The discharge at the rated current missed at every temperature: 48.2, 44.1, 51.2 and 88.2 millivolts at 40, 25, 10 and 0 degrees Celsius, against a mark of 42. Of the 32 drive cycles from 0 to 40 degrees Celsius, 23 passed and 29 met the voltage mark. At -10 and -20 degrees Celsius no run met its marks: voltage errors of 55.6 to 425 millivolts. From engine 0.7.0 to 0.9.0 those two blocks were refused as BlockShownWrong and each served block was held to the case temperatures and the currents of its own held-back tests; now every block is served at the card's case temperatures, -21.898 to 45.535 degC, and currents to 13.5 A either way, and a step outside its block's own tests carries FLAG_OUTSIDE_COMPARED_TESTS. A third scoring, written from the protocol's text alone by an agent that did not write the card, through engine 0.6.0 on 2026-10-10, reached the same 27 of 60 with every error and every call the same to within a millionth of a millivolt.".into(),
            "A miss, and it stays on the record. The forecast made outside the engine on the 18 opened tests, one discharge at 2.25 A and two drive cycles at each temperature, had this card's shape meet every mark at 25, 10 and 0 degrees Celsius, miss one drive cycle at 40 degrees Celsius by 2 millivolts at the last sample, and miss at -10 and -20 degrees Celsius. It held no discharge at the rated current and could not have shown that miss. The held-back tests of this cell are spent: a corrected card needs a cell no model has seen. One party wrote the card, ran it and scored it twice, on one machine; a third scoring, from the protocol's text alone by an agent that did not write the card, agreed with it. No one outside Protocol Company, which makes this engine, has repeated it. One cell. One capacity per block, while the cell went on ageing inside each block. Each temperature was measured at one age, so a block is also an age of the cell. The card is given the case temperature and does not predict it. Fitted from pulses of up to 13.5 A discharging and 2.25 A charging: a charging current above 2.25 A is served by the discharge side of the tables. Served up to 11.375 A discharging and 9.737 A charging, the largest currents of the held-back tests, and in each block only up to the largest currents of that block's own tests; held currents were compared at about 1.5 A and 4.5 A only, and the card bounds the size of the current, not how long it is held. The tables end where a slow discharge at the block's temperature ended. The step is one second.".into(),
        ],
        "cantilever_eth" => [
            "One aluminium strip, 326 mm long, 40 mm wide and 1 mm thick, clamped at one end under a bolted steel plate with nothing between plate and strip, at ETH Zürich: its first bending mode in free decay in room air, as the deflection at the laser-1 station, 268 mm from the clamp.".into(),
            "Lai, Alzugaray, Chli and Chatzi, Zenodo, doi 10.5281/zenodo.22723484, CC BY 4.0: recordings of the strip ringing down. The card's six constants were fitted to one recording, test1a, from 0.863 s to 24 s after its start. A second ring-down of the same strip in the same clamp, test1b, was held back from that fitting: its first 12 s were opened for a forecast made outside the engine, and its next 12 s were held back from the fitting and the forecast and compared with the card once. The comparison is not blind: on 2026-10-06, before the split was written, the coarse envelope of every recording, the peak-to-peak value of Laser_1, the channel the amplitude line scores, in each half second of the first 30 s, was printed by mistake; no fit was made to it.".into(),
            "Fixed before any scoring (pre-registration SHA-256 d081cc5ae51b5de4a028cd17b4e1ae80b402f71fdc786f82876713f3b925d4a6), on the blocks between 12 s and 24 s of the held-back ring-down: amplitude error at most 5.0 percent, frequency error at most 0.010 Hz, timing error at most 0.20 of a period by 24 s, and the amplitude error and the frequency error each at most one third of a plain damped oscillator's.".into(),
            CANTILEVER_RESULT.into(),
            "A miss, and it stays on the record. A second scoring, written from the protocol's text alone by a second agent with a reader of its own, reached the same verdict on the same eleven blocks with the same frequency and timing errors; it took each period's amplitude from a fitted sine where the first took half the peak-to-peak value, and found 7.94 percent where the first found 7.03. The forecast for this structure, made outside the engine on the opened 12 s, was 2.22 percent in amplitude, 0.0047 Hz in frequency and 0.024 of a period in timing, and the card run through the engine gave the same figures there. The rule fixed in advance named a smaller structure of five constants; this card is the larger one of six, chosen after the opened scores were seen because the smaller was expected to miss by more. Scored outside the engine on the same blocks, the smaller structure missed the amplitude line too, at 6.48 percent, closer than this card. The last two blocks lie below the smallest amplitude the constants were fitted on, 0.072 mm; worked out after the scores were seen, the amplitude error without them is 5.7 percent, still over the line. The comparison spanned envelopes from 15.79 mm down to 0.058 mm: the compared run's own start, at 15.786 mm, is the largest envelope compared, and its readings were scored from 14.404 mm. Above 15.79 mm, up to 18.75 mm, the largest amplitude the constants were fitted on, the card answers with the frame flagged FLAG_ENVELOPE_ABOVE_COMPARED, where engine 0.11 refused; nothing there was compared. Above 18.75 mm the card refuses by name, as EnvelopeAboveCard: the fitted laws were not fitted there, and the strip's own yield needs its alloy, which the dataset does not state. Below 0.058 mm the card answers on to rest with the frame flagged, where engine 0.9 refused. The held-back half of this ring-down is spent: a corrected card needs a ring-down no model has seen. One party wrote the card, ran it and scored it twice, on one machine; not yet repeated by a second party. One strip, one clamp, one afternoon. The laser was logged in volts: the scale in metres rests on one line of the authors' script, confirmed by a second reading to 3 percent, and every compared quantity is a ratio or a frequency. The card has no mass, so its energies are per kilogram. The softening spring is a fitted power law: it is not derived from the strip's dimensions and says nothing outside the fitted amplitudes. A start inside the amplitude ceiling with nearly all its energy as speed can be refused some steps later, because the softened spring carries the deflection envelope past the ceiling: from zero deflection with the envelope at 0.99 of the ceiling the step is refused after 107 steps of 0.25 ms, and at 0.98 the beam rings through.".into(),
        ],
        "cell_2170_model3" => [
            "One Tesla Model 3 2170 lithium-ion cell from -10 to 40 degrees Celsius: its terminal voltage, given the current and the temperature measured on its case. A physical model with memory: one measured rest-voltage curve and 22 fitted numbers for the immediate resistance, a surface reaction that eases with current and temperature, lithium diffusing in the particles, hysteresis, and a slow voltage drop in the electrolyte under sustained current. Three cells of the dataset are held: m80, m1000 and m448-N.".into(),
            "Kollmeyer, Khanum, Naguib and Emadi, Tesla Model 3 2170 Li-ion Cell Dataset and Battery SOC Estimation Blind Modeling Tool, Borealis, doi 10.5683/SP3/ZVTR4B, version 3.0, CC BY 4.0. The curve and the 22 numbers were fitted on every test of cell m80 from -10 to 40 degrees Celsius. For cell m1000 one number was fitted, a resistance scale, on its discharge at a third of the rated current, its pulse test, its slow charge and discharge and its capacity test at each of those temperatures, and its capacities were measured from its recharges. For cell m448-N the same one number was fitted, on the same four tests of its own at each temperature. 45 runs of cell m1000 and 60 runs of cell m448-N from -10 to 40 degrees Celsius were held back unopened until the card was run on them: discharges at a held current and drive cycles.".into(),
            CELL_2170_MODEL3_PASS_LINE.into(),
            CELL_2170_MODEL3_RESULT.into(),
            "The first cell is not a blind test: the model was developed on all of its tests. Scored outside the engine by the study that fitted it, the model met the marks on 62 of the first cell's 65 runs in range and on 14 of the second cell's 15 opened runs in range. Known misses. Three hill cycles of the first cell, at 40, 25 and -10 degrees Celsius; at 40 and 25 the card stops at its 2.8 V limit shortly before the real cell, which is riding the same limit there. Steep climbs at low charge in the cold: at 0 and -10 degrees Celsius, at the end of climbs of 7 to 10 A for four minutes below 45 percent charge, the card reads 130 to 180 millivolts high, and three physical causes were tested and none stands. Charging in the cold: in the descents of the hill cycle at 0 and -10 degrees Celsius the card reads 40 to 70 millivolts high. Capacity is up to 2 percent long under held current at 10 and 0 degrees Celsius. The cold edge is predicted poorly when it is left out of the fit: fitted without the tests at -10 degrees Celsius the model met its marks on 3 of the 13 runs there, so the temperature laws do not reach to a colder temperature. The range: a case temperature from -12.9 to 45.9 degrees Celsius, which is the chamber set from -10 to 40 with 0.3 kelvin allowed either side of what the sensor read; a current up to 13.524 A discharging and 13.984 A charging; each limit is set from the tests the model was scored on before that comparison, on the first two cells, with the accuracy the dataset states for the tester or the chamber allowed beyond them. -20 degrees Celsius is outside the card, because the measurements there are irregular and no mechanism could be separated from them; scored there for the record, the model met its marks on none of the first cell's 13 runs. Sustained current above the rated current was tested by the hill cycle alone; a running mean above 8.722 A, the largest scored, lands with FLAG_OUTSIDE_COMPARED_RUNS. Charging current is accepted and was fitted inside drive cycles; no charging run was scored. The card is given the case temperature and does not predict it. The capacity is the caller's, read from the last full charge. Each block of tests was measured at one age of the cell, so a new cell was never measured in the cold nor an aged cell warm, and the resistances carry no age term: a capacity outside the band its block was scored with at that case temperature lands with FLAG_OUTSIDE_COMPARED_RUNS. So does a case temperature that changes by more than 5.4 K between steps or spans more than 19.2 K in a run, past the 5.023 K and 18.811 K of the scored runs. Through engine 0.9.0 each was a refusal. Every fit and score was made at a step of one second, and the card steps by one second only: where the current reverses, a step of 0.1 s gave an answer 49 mV from the one-second answer in the reversal that was compared, and no measurement on disk tests it. Every run scored in the record lies inside these edges, so none of them carries the flag. The unseen runs of the second and the third cell inside the range are now spent: a corrected card needs a cell no model has seen. The hill-climb cycles of those cells lie outside the card's range and were not scored. One developer's SI wrote the model, wrote the card and ran the comparison, on one machine. A second agent, which did not write the card, scored it again on engine 0.6.0 from the protocol alone (evidence/verification/fresh_eyes_0.6.0/cell_2170_model3): with the record's own starts 50 of 60 runs of cell m448-N and 33 of 45 of cell m1000 were inside the marks, the same verdict as the record on 102 of 105 runs, the three others turning on where the scored span ends; with its own simpler starts, full charge and each block's capacity, 48 and 27. It is the same developer's SI, and no one outside Protocol Company, which makes this engine, has repeated the result.".into(),
        ],
        "creep_cmsx6" => [
            "One cast of the single-crystal nickel-base superalloy CMSX-6, cast by PCC Airfoils as [001] bars, solution annealed and aged: twelve round test pieces 6 mm in diameter, within 1.9 to 7.5 degrees of [001], in tension creep at 980 degrees Celsius under a held load at initial stresses of 140, 170, 200 and 230 MPa, three at each.".into(),
            "Avila Calderon, Schriever, Han, Olbricht, Dolabella Portella and Skrotzki, BAM Reference Data: Creep of Single-Crystal Ni-Based Superalloy CMSX-6, version 2.1, Zenodo, doi 10.5281/zenodo.20132712, CC BY 4.0: creep tests to DIN EN ISO 204 in a laboratory accredited to ISO 17025. The card's six constants were fitted to six tests, two at each of 140, 170 and 230 MPa. Six were held back and not read: all three at 200 MPa, a stress no fitted test has, and one more test piece at each of the other three stresses.".into(),
            "Fixed before any curve was fitted (pre-registration SHA-256 f20f99b0e3419d66201a3e124b5c216c4d138d41f976b19d6a536daf15316d87, with two numbered amendments made before the held-back tests were read, and a third, a correction of the forecast, made after they were scored): over the six held-back tests, the natural logarithm of the card's time over the measured time to reach 0.1, 0.2, 0.5, 1, 2 and 5 percent creep extension, root mean square at most 0.15, worst at most 0.30, mean at most 0.10 in size.".into(),
            CREEP_CMSX6_RESULT.into(),
            "One cast, one temperature, one laboratory. The card's six constants are fitted to six tests of this cast, and the law was chosen after the fitted tests were seen. Of the five structures registered beforehand it is the first to meet the three lines in both forecasts registered on the fitted tests: each fitted test predicted from the other five (0.112, 0.279, -0.001), and the 170 MPa tests predicted from 140 and 230 MPa alone (0.118, 0.229, -0.039). The record first gave that second forecast as a miss (0.229, 0.538, -0.113) and the first amendment dropped it as a gate. The miss did not happen: the forecast's integral was wrong at the constants that fit reached, the second scoring found it, and amendment 3 (SHA-256 99f461ffb0f589334d841590c456ba7c3027651f7a0249bf166ebcfe57e2e60f), written after the held-back tests were scored and so not a pre-registration, corrects it. The first amendment was not needed, the rule as first registered names this law, and no constant of the card changed. What the pass means: from 0.1 to 5 percent creep extension, inside 140 to 230 MPa, for this cast, on the six registered levels, the card is about as good as the measured test pieces are at predicting each other. Each of the twelve pieces predicted from the other two at its stress gives a root mean square of 0.075; the card's is 0.093. A straight-line interpolation of the fitted measurements, with no creep law, also meets the three lines on the same 33 times (0.098, 0.237, -0.013). 200 MPa lies where the stress dependence is straight: the minimum creep rate has a stress exponent near 6 from 170 to 200 MPa and from 200 to 230 MPa, and over 10 from 140 to 170 MPa, where no test was held back. The pass does not show that the law predicts a stress away from the fitted ones. The simplest registered structure, a Norton power law, misses on the same six tests (0.436 root mean square, 0.952 worst). At 0.5 percent the card is late by a mean of 0.095. Below 0.1 percent creep extension the card was not compared on a registered level, and compared with the measured curves after the scoring it starts later than every one of them at all four stresses: in hours to 0.05 percent, the card against the three test pieces, 38.9 against 15.4 to 26.7 at 140 MPa, 7.8 against 2.7 to 3.5 at 170 MPa, 1.69 against 0.61 to 0.66 at 200 MPa and 0.245 against 0.06 to 0.08 at 230 MPa. The measured curves gain 0.02 to 0.04 percent within the first hour and the card stays near zero; its extension is under the measured one by at most 0.07 percent. The card answers there, and each frame carries FLAG_OUTSIDE_MET_RANGE; from amendment 4 through engine 0.9 it refused a state there as CreepExtensionBelowCard, and before amendment 4 it answered there with a flag. At 140 MPa the three tests were stopped at 1010 to 1030 h, at 0.79 to 0.83 percent, so only the levels 0.1, 0.2 and 0.5 percent are compared there and the card's 1 to 2.09 percent at 140 MPa is beyond any measurement. Between 0.5 percent and where those tests stopped the card runs ahead of them: worked out after the scoring and not a registered level, it reaches 0.8 percent at 683 h where the two tests that got there took about 1000 h, and at the hour each test stopped it gives 1.70 to 1.78 percent. The card answers past 0.2 percent below 170 MPa with this miss stated, about a factor of 2 in extension at the hour the tests were stopped, and each frame there carries FLAG_OUTSIDE_MET_RANGE; through engine 0.9 it ended at 0.2 percent below 170 MPa. How the range was drawn, with all twelve tests read after the scoring, on 171 levels from 0.10 to 5.00 percent in place of the six registered: at 170, 200 and 230 MPa the law's time since load is inside the registered worst line of 0.30 at every level on all nine tests, and the worst is 0.289, at 170 MPa and 0.33 percent on a held-back test, a margin of 0.011. At 140 MPa all three tests are inside the line at every level from 0.10 to 0.25 percent, where the worst is 0.296; one is outside it from 0.26 percent, a second from 0.31, and all three from 0.66 percent. Below 170 MPa the law met the line to 0.2 percent, the largest registered level under that, where the worst is 0.196. No test lies between 140 and 170 MPa, so the law met its line to 5 percent only from 170 MPa, the lowest stress at which it was measured and held, and the stress exponent between 140 and 170 MPa is where the law is least tested. The card's clock is the time since load because that is what holds: the law's time between two neighbouring levels is off by up to 1.06 in the logarithm against the measured curves, and its errors cancel only in the sum from zero. Its exponents, 33.4 and -27.2, are fitted numbers of this law and not physical constants of the alloy. Between repeat test pieces at one stress the measured time to 0.1 percent differs by up to 0.27 in the logarithm. No rupture, no extension beyond 5 percent, no other temperature, no changed or removed load (a changed stress is refused as StressNotHeld), no other crystal direction. Scored a second time on 2026-10-09 by a second agent with programs of its own (evidence/verification/second_party/V-24/): the same 33 values to 1e-12 and the same verdict, on the same machine. Not repeated on a second machine or by a second organisation.".into(),
        ],
        "grid_gb" => [
            "The Great Britain synchronous area, the 50 Hz transmission system of England, Scotland and Wales, in the 60 seconds after an importing interconnector, or one pole of it, trips: 41 trips of 693 to 1402 MW between July 2017 and May 2026.".into(),
            format!(
                "The system frequency once a second and the half-hourly outturn inertia, published by the National Energy System Operator under the NESO Open Data Licence (supported by National Energy SO Open Data), and the interconnector's flow before and after each trip in Elexon's five-minute generation by fuel type (BMRS dataset FUELINST). The trips were found by a written rule from 114 months of frequency. Every third trip in time order, 13 of 41, was held back; the card's three fitted constants were taken from the other 28: a response of {} MW/Hz below 50 Hz, a further {} MW/Hz below the knee, and a lag of {} s.",
                grid_gb::RESPONSE_GAIN_MW_PER_HZ,
                grid_gb::RESPONSE_GAIN_BELOW_KNEE_MW_PER_HZ,
                grid_gb::RESPONSE_TIME_S,
            ),
            "Fixed before any model was run (protocol SHA-256 35c91fa9fe440ad852390b77bcd6ecddd8e7029630e620a660b11aad1f481ea9, amended once before the structure of this card was fitted, SHA-256 0fd449fc45839ef37928db97995b43f47cb22bb2ae89d531ccf4592e6e9732ba). Depth of the fall and the steepest second: root mean square of the relative error at most 0.25 and mean within 0.10, and the depth error smaller than that of the rule that depth is proportional to loss. Frequency near 60 s: root mean square error at most 0.06 Hz. The whole trace: median root mean square error at most 0.05 Hz.".into(),
            GRID_GB_RESULT.into(),
            "The held-back trips are now spent, and they are an easier set than the opened ones: the proportional rule scores 0.18 on them and 0.337 on the opened 28 with the card's constants (0.35 with each trip left out of the fit in turn), which hold a trip followed by a second loss (22 December 2023) and the shallowest fall of all (9 May 2026), so the card's margin over that rule on the held-back trips is small. On the opened trips the card missed its mark: with each trip left out of the fit in turn the depth error was 0.302 against 0.25 and the error near 60 s was 0.097 Hz against 0.06, while the steepest second, 0.154, and the trace, 0.050 Hz, met their lines and the depth beat the proportional rule, 0.350. No structure of the protocol met every line; this is the one with the smallest depth error. The response the operator held on each day is not in the open data and is not an input: the card carries one response for nine years. Its errors follow the calendar: before 2021 its falls are about a quarter too shallow, after 2023 too deep. The depth error moves by +0.066 a year (standard error 0.013), from -0.30 in 2018 to +0.17 in 2025, nearly twice the 0.25 mark across the span. The system changed: Dynamic Containment from October 2020, 500 MW rising to 1,576 MW by July 2023; Dynamic Moderation and Dynamic Regulation from March 2022; the minimum inertia lowered in 2024. The card has one gain and one lag for all of it. The inertia is the operator's estimate, which leaves out the Stability Pathfinder units. It is the sharper input: 0.9 or 1.2 times the published inertia misses the line on the mean of the steepest second. The size of the loss is steady to 0.35 percent by the steadiness rule. The range was narrowed on 2026-10-10, after the scoring and with every trip seen, so the narrowing is not a new test. Losses: every scored trip lost 693 MW or more. The six trips of 500 to 692 MW inside the other spans, among those the scan lost, run through the engine by the protocol's lines, missed three of the seven: depth 0.2512 against 0.25, and against 0.2509 for the proportional rule, steepest second 0.273 against 0.25; the mean depth error -0.032, the steepest second's mean +0.057, near 60 s 0.054 Hz and the trace median 0.030 Hz met theirs. A loss under 693 MW was refused as LossOutsideCard through engine 0.9.0; now it lands with FLAG_OUTSIDE_COMPARED_TRIPS. Frequency: no compared trip went under 49.553 Hz; through engine 0.9.0 a frequency under 49.55 Hz was refused as FrequencyBelowCompared, and now it lands with FLAG_BELOW_COMPARED_FREQUENCY, down to 48.8 Hz, where low-frequency demand disconnection stops the run. Two numbers are not fitted. They were first written from memory, and the second scoring found their sources. Demand's relief of approximately 2.5 percent per hertz: National Grid, System Operability Framework 2016, p. 70, and the NESO webinar of 12 March 2025; the documented range is 2.0 to 2.5. The 49.8 Hz knee is Dynamic Containment's (NESO, Response Services Service Terms v6; Dynamic Containment testing guidelines, August 2020): right for that one service from October 2020, and before it there were static blocks at 49.7 and 49.6 Hz. With the three constants refitted, a relief from 0 to 5 percent per hertz changes no verdict, and the pass holds for a knee of 49.80 to 49.90 Hz and is lost at 49.75. Every compared trip was one loss: through engine 0.9.0 a later segment's other loss was refused as LossChangedDuringRun; now the ledger books each step's own loss and the step carries FLAG_OUTSIDE_COMPARED_TRIPS. No changing loss was compared with a measurement: the one measured trip with a second loss, that of 22 December 2023, is the card's worst. The recomputation outside the engine failed against its criterion as first written, in 39 of 80 runs: only the response and the two integrals were outside, the response by at most 4.2e-10 MW, and the frequency was never above 0.05 of the bound. The yardstick was changed afterwards by the card's author. A run of exactly 60 s in decimal lands every step: 3,000 steps of 0.02 s land 3,000 and read 60.00000000000378 s. Through engine 0.5.0 the last was refused at some step sizes, because the clock is summed in binary; the comparison with 60 s now carries the published rounding allowance, and no answer that landed before has changed. Known defect of the scan that found the trips: its rule of 0.15 Hz or more lost four falls of exactly 0.150 Hz to rounding. One frequency for the whole area; no network, no generator that trips as a consequence, nothing after 60 s. One party wrote the card and ran it. It was scored twice, the second time by a second scorer, on one machine.".into(),
        ],
        "tire_eve_r19_ice" => [
            "One 235/55 R19 all-season sport-utility tyre of the European project EVE, rolled on a prepared ice track by the Terramechanics Rig of the Advanced Vehicle Dynamics Laboratory at Virginia Tech in April 2016, at a creep of about 0.06 m/s: the ratio of its longitudinal force to its normal load at a held slip, driven and retarded.".into(),
            "Schleinin, Savitski, Jimenez, Sandu and Ivanov, Dataset of tyre tests on ice, parts 1 to 3, Zenodo, doi 10.5281/zenodo.1009017, 10.5281/zenodo.1009023 and 10.5281/zenodo.1009025, CC BY 4.0: 252 runs, each stepping the wheel through four slips, the first three for about 21 s each and the last until the wheel stopped, 21 to 33 s. The card's three constants were fitted to 72 runs at 3.5 and 5 kN and at 1.5 and 3.5 bar. Held back from that fitting: all 54 runs at 6.5 kN, a load above every fitted load, and all 36 runs at 2.5 bar at the two fitted loads, a pressure between the two fitted pressures. The 90 runs on a water film with a steered wheel were left out.".into(),
            "Fixed before any held-back run was opened (pre-registration SHA-256 84a380a5c948cd741ac91845fbfeac7ed1b60ed3c6a01855bcb99c996e7782c1), on the 60 held-back cells, each the mean over three repeats of the last nine seconds of a slip stage at 5, 6, 7, 12, 15 or 20 percent slip: root-mean-square error of the friction ratio at most 0.045 over all 60 cells, over the 24 cells at 2.5 bar and over the 36 cells at 6.5 kN, and over the 60 cells smaller than the error of a ratio of 0.1 that uses no measurement of this tyre and smaller than the error of the opened runs' mean with no slip term. That mean is the geometric mean of each mode, least squares on the logarithm, as the fit is; the arithmetic mean would have given 0.038, and the line is met either way.".into(),
            TIRE_EVE_R19_ICE_RESULT.into(),
            "The three constants are fitted to this tyre's own runs; nothing is derived from the rubber or the ice. The held-back runs at 2.5 bar test interpolation in pressure and those at 6.5 kN test extrapolation in load, on the same rig, the same ice and in the same campaign as the fitted runs: nothing here tests another ice temperature, another speed, another tyre or another laboratory. The files record no ice temperature, no carriage speed and no rolling radius, and the tyre's make is not published. The load and the pressure do not enter the friction ratio: on the opened runs a ratio that varied with them predicted a left-out condition worse than one that did not, and the differences between conditions, about 0.03, were three times the scatter between repeats, so part of what a condition shows is the state of the ice on the day. Slips under 5 percent and the rise of the force from rest are outside the card. The speed is a creep of 0.06 m/s: the friction of ice changes with sliding speed, and the card says nothing of a tyre on a road. The error is between conditions and not between repeats: a condition sits 0.01 to 0.06 from the card as a block, and the card cannot say which way the ice of a given session will sit. The error follows the test block, not the load or the pressure: the 18 condition means of the measured ratio, in six blocks, run from 0.185 to 0.287, and the standard deviation between them is 0.029, against 0.005 to 0.011 between repeats. The ice changed between sessions and within the first block, where the ratio rose by 0.010 to 0.024 from the first repeat to the third. Any model that does not know the block lands near 0.03, and the card is at that floor. A user should carry about ±0.03 (one standard deviation) on the ratio for the session. The figure of 0.23 to 0.24 is this tyre on that rig's ice in that fortnight at 0.06 m/s; it is not a figure for a tyre on ice at a stated temperature or at road speed. The slip term is real on the held-back runs and small: the exponent fitted to them alone is -0.107 ± 0.014, where the card holds -0.133, and the term is worth 0.002 to 0.003 of the 0.03 error. The longitudinal force at rest before a run, about -62 N with a standard deviation of 49 N, is not subtracted from the measured forces. The ledger of the three works closes for any constant, because each work is proportional to the force: it can catch a corrupted step, and it cannot catch a wrong constant, a wrong exponent or a wrong ice. The time after a slip change was not part of the registered comparison. Compared on 2026-10-10 by the protocol's cells and lines, with the change placed where the wheel speed steps, every one-second window from 5 s to 20 s after a change met all five lines, and every one before 5 s failed one: the first second missed the line of 0.045 at 6.5 kN, with 0.0477, and the comparator, with 0.0449 against a constant ratio's 0.0441 over the 60 held-back cells, and the seconds from 1 s to 5 s did no better than the constant ratio, 0.0430 against 0.0424 from 1 s to 2 s and 0.0399 against 0.0395 from 2 s to 5 s, where from 5 s to 11 s the card had 0.0365 and from 11 s to 20 s 0.0337, every line met. The card answers a step before 5 s with the same steady force and the frame carries FLAG_SLIP_SETTLING; through engine 0.11 it refused such a step as SlipNotSettled (evidence/research/tire_eve_235_55r19_ice/RESULTS.md). The range the card was compared on, in numbers: loads of 3.5 to 6.5 kN, fitted at 3.5 and 5 kN and compared at 6.5 kN with a root-mean-square error of 0.029 against a line of 0.045; pressures of 1.5 to 3.5 bar, compared at 2.5 bar with 0.040; slips of 5 to 20 percent; a carriage speed of about 0.06 m/s; and 5 s to 21 s after a slip is set. Past the loads, the pressures and the hold measured, the card answers with the same law and the frame carries FLAG_BEYOND_COMPARED: nothing was measured there. Engine 0.7.0 changed every answer of this card: a start states slip_held_s, the state returns it. Through engine 0.9 the card refused a load outside 3.5 to 6.5 kN, a pressure outside 1.5 to 3.5 bar, a hold past 21 s and a distance past 6 m, and its step ceiling was 16 s. One party wrote the card and ran it. It was scored twice, the second time by a second scorer, on one machine.".into(),
        ],
        "fingerpad_p02_glass" => [
            "The right index finger pad of participant P02, one of seven volunteers of 23 to 35 years, held still with the nail up while a robot presses a flat glass plate on it at a held normal force and turns the plate about the normal through 80 degrees, at a held rate to between 77 and 79.7 degrees and then slowing to a stop at 80, at the Université catholique de Louvain: the torque of the pad on the plate and the stuck fraction of the contact as the twist grows, to full slip.".into(),
            "du Bois de Dunilac, Córdova Bulens, Lefèvre, Redmond and Delhaye, Biomechanics of the finger pad in response to torsion, J. R. Soc. Interface 20 (2023) 20220809, doi 10.1098/rsif.2022.0809, and its data on Zenodo, doi 10.5281/zenodo.7646119, both CC BY 4.0: seven fingers, five normal forces at 20 degrees a second and five rates at 2 N, clockwise and counter-clockwise, five repeats, in two experiments on separate days: the first over a camera, which gives the stuck fraction, and the second over a centred torque sensor with no camera, which gives the torque. The archive's torque experiment has 110 trials a finger: the paper's 90 and 20 more at 2 N and 20 degrees a second twisted about a point off the centre of the contact. The card's eight constants were fitted to 80 trials of P02 at 0.5, 2 and 5 N and at 5, 10, 20 and 50 degrees a second, 70 of them in the block at 2 N. Held back from that fitting: P02's 10 trials at 1 N, 10 at 10 N and 10 at 100 degrees a second, and the fingers P03 to P07 whole. The finger P01 was open whole and gave the form of the model. The paper's own fit of the slip torque over load, T/F = k F^(n-1) for each finger, is k = 6.74 +/- 1.31 mm and n = 0.76 +/- 0.06 over the seven; this finger's own exponent is 0.72 to 0.77 by the weighting. The authors' per-trial summaries and the raw 1 kHz files were not used in the fitting or the scoring; the summaries were compared afterwards.".into(),
            "Fixed before any held-back trial was opened (pre-registration SHA-256 49a4788d1eb6e5d5881d4f3b2195ad248a2a543c2da45fb1c8ffb7dc27672081), twelve lines, four at each of P02's three held-back conditions, each set at the scatter of one trial about its own condition's mean on the fitted conditions: slip torque (the mean over the 0.5 s before 75 degrees) within 5.62 percent of the measured mean; torque curve from 1 to 75 degrees within 8.32 percent of the slip torque, root mean square; twist at full slip (stuck fraction under 0.03) within 5.75 degrees; stuck-fraction curve within 0.0595, root mean square. Beside each slip torque: a constant, torque proportional to load, a straight line in load, the nearest fitted condition, and the same model with the constants of finger P01, which never saw P02. A line that one of those also meets is not evidence for the card. The five other fingers carry no line.".into(),
            FINGERPAD_P02_GLASS_RESULT.into(),
            FINGERPAD_P02_GLASS_LIMITS.into(),
        ],
        "accumulator_sbo450" => [
            "One hydraulic diaphragm accumulator, HYDAC SBO450-0.6 with no foam insert, charged with nitrogen, on the accumulator test rig of the Chair of Fluid Systems at Technische Universität Darmstadt in March 2024: the pressure of its gas as a piston pushes oil into it and draws it out in a sine of 2 mm or 5 mm at 0.01 to 20 Hz.".into(),
            "Rexer and Lefemmine, accumulator 600ml empty, Zenodo, doi 10.5281/zenodo.11265458 and 10.5281/zenodo.15478932 (precharge 40 bar, load 60 bar), 10.5281/zenodo.11265391 (20 / 40), 10.5281/zenodo.11265436 (20 / 60), 10.5281/zenodo.11265491 and 10.5281/zenodo.11265583 (40 / 80) and 10.5281/zenodo.11265625 (40 / 120), CC BY 4.0: 399 runs, each one frequency and one amplitude, with piston deflection, force, gas pressure, oil pressure and three temperatures at 1 kHz. The ratio of piston area to vessel volume and the thermal time constant were fitted to the 100 runs from 0.01 to 10 Hz of the two records at 40 / 60 bar. Held back from that fitting: all 285 runs of the other five records. After the second scoring the piston area was set to a 50 mm bore from the force and the oil pressure of all seven records; by then every run had been opened. Nitrogen by the Peng-Robinson equation with the critical constants of Span et al. 2000; against the NIST WebBook isotherm at 293.15 K its density is 0.5 percent high at 20 bar and 1.5 percent high at 80 bar.".into(),
            ACCUMULATOR_SBO450_PASS_LINE.into(),
            ACCUMULATOR_SBO450_RESULT.into(),
            ACCUMULATOR_SBO450_LIMITS.into(),
        ],
        "tokamak_jet_dte1" => [
            "The JET tokamak at Culham in ELMy H-mode pulses of 1996 to 1997, with the Mark IIa divertor, in deuterium and in deuterium-tritium (the DTE1 experiment): the thermal energy of the plasma, the energy of its fast ions and its D-T fusion power through the heated part of a pulse.".into(),
            "International Multi-Tokamak Confinement Profile Database, 2008 public release (tokamak-profiledb.ccfe.ac.uk; Nucl. Fusion 40 (2000) 1955): the time traces of ten pulses. The energies and the fusion power in it are the TRANSP reconstruction of each pulse from its measured profiles, not raw signals. Four constants were fitted on eight pulses. Two were held back from the fitting: 42982, the steady pulse of the 1997 fusion-energy record, and 42976, the hot-ion pulse of the 16 MW fusion-power record.".into(),
            "Fixed before the card was run on either held-back pulse (pre-registration SHA-256 7badf2b1267219d0e4da2edaad7d50097f9f98ceee1e0ab5fb9f8fd391ba94bb), over the heated window of each pulse, sampled every 25 ms: thermal energy error at most 14.5 percent of its measured peak, root mean square, the error the IPB98(y,2) scaling has on its own data; fast-ion energy error at most 25 percent of its peak; fusion power error at most 25 percent of its peak; and the fusion energy released within 25 percent.".into(),
            TOKAMAK_RESULT.into(),
            "A forecast is not a result. Predicted with constants fitted without it, six of the eight training pulses met the thermal-energy mark and all eight the fast-ion mark; the fusion constants rest on one pulse, 42762, and on the four training pulses that are not an even mix the fusion power met its mark on one, 43134 at a tritium fraction of 0.034 (16.5 percent of its peak), and missed on three, 42794, 42997 and 43002 at 0.75 to 0.93 (33, 65 and 40 percent); above a fraction of 0.504, and from 0.034 to 0.451, where no pulse was compared, the card computes the same D-T power, and the miss above 0.504 is stated here. The compared envelope is that of the nine pulses of the card's regime run through the card: 0.124 to 24.3 MW of heating, 0.173 to 2.44 MW of ohmic power, 1.65 to 3.84 MA, 1.80 to 4.04 T, 1.65e19 to 7.85e19 per cubic metre and up to 0.834 of the Greenwald density, a safety factor of 3.66 to 4.14, 0.462 to 9.57 MJ of thermal and 5.59 kJ to 1.62 MJ of fast-ion energy, 0.777 to 4.30 keV, a normalised beta of 0.224 to 2.21, a confinement time of 0.150 to 5.72 s, and 5.475 s of the heated window. Outside it the card computes the same equations, and nothing was compared there; it stops only at the Greenwald density, an edge safety factor of 2, the Troyon limit, the span of the reactivity fit and the ceiling of each state's step. The states and loads only pulse 42976 reached are outside the compared envelope and are computed, and that pulse, a hot-ion pulse declared outside the card's regime before it was run, missed all four marks. The card has one temperature, no profiles of its own, no ELMs and no radiation term; the density, the current and the field are the caller's. It does not know the species of the beams. The compared values are a reconstruction by the TRANSP code, whose own fusion power is within 10 to 20 percent of the measured neutron rate. Pulse 42976 is not an ELMy H-mode: it was declared outside the card's regime before it was run. The comparison is not blind: before the marks were fixed on 2026-10-09, every file of both held-back pulses had been downloaded and its format read, a table of 28 time samples of 42976 had been printed and a confinement multiplier of about 1.5 estimated from it by hand, and one line of peak values had been printed for each pulse (42982: thermal energy 10.1 MJ and fusion power 6.08 MW; 42976: 13.1 MJ and 14.4 MW); 42976 was declared outside the card's regime after its table was read. No model was run on either pulse and no constant was fitted to either before the scored run. One tokamak, one campaign, one maker. The comparison was scored a second time by a second agent of the same maker, given the protocol, the raw files and the saved answers and not the first scorer's scripts (evidence/verification/tokamak_jet_dte1_scoring/): the same eight results and the same two verdicts, every number agreeing to 15 figures; on engine 0.6.0 a third agent rebuilt it from the protocol and the raw files and reached the same numbers (evidence/verification/fresh_eyes_0.6.0/tokamak_jet_dte1/). No outside party has repeated it. The card's fusion power is superseded by the solver tokamak_dt_beam_target, which computes the D-T rate of one time slice from the plasma's profiles and its beams.".into(),
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
            "International Laser Ranging Service combined orbits (product doi 10.5067/SLR/SLR_ILRSORB_001; Pearlman, Noll, Pavlis et al., 2019, J Geod 93, 2161-2180). Two LAGEOS-1 weeks and four LAGEOS-2 weeks were held back from the card's design.".into(),
            "Fixed before any model was scored: within 1 m of the published orbit after one revolution, 10 m after one day and 100 m after seven days.".into(),
            "Every line met on 42 runs, none refused. LAGEOS-1: worst distance 0.65 m after one revolution, 3.8 m after one day, 32 m after seven days. LAGEOS-2: 0.34 m, 2.9 m and 22 m.".into(),
            "The card's structure was chosen after a ladder of structures fixed in advance failed on two opened weeks: sunlight pressure and relativity were rungs of that ladder, and the solid tide was added afterwards; the rule then named the smallest structure that passed on the opened weeks, the field to degree 8, and the card was built at degree 12, a departure from that rule chosen on the same two weeks and stated before the held-back weeks were opened. The held-back weeks are what make it a test: one LAGEOS-1 week and all four LAGEOS-2 weeks were fully blind; the other LAGEOS-1 week had been downloaded earlier and its analysis centres compared with each other, with no model involved. The published orbit is itself a fit by analysis centres to laser ranges, good to a few centimetres; it is not a raw measurement. The start state is turned into the card's frame outside the engine, as start_state says; a crust-fixed state fed in directly was 69 to 81 m off after one revolution and 374 to 385 m after one day. Free flight only; compared on 28 calendar days, 2026-08-23 to 2026-09-19, for at most seven days from a published state, on the two orbits of LAGEOS-1 and LAGEOS-2 and nowhere between or beside them. The card computes other orbits, longer forecasts and any date from 2017-01-01 to 2027-06-28, the span its locked TT - UTC holds; none of those was compared, and its error grows with the length of a forecast from what it leaves out. It computes any radius down to the Earth's equatorial radius, 6 378.1363 km, where the compared runs went no lower than 11 997.57 km; a landing under 11 997 km carries FLAG_BELOW_COMPARED_RADIUS, and the lower the orbit, the larger the air's drag on the sphere, which the card does not carry. No drag, no light reflected from the Earth, no ocean tide; the Sun and the Moon are approximate. Run once and verified twice. Repeated independently on 2026-10-07 by Grok, another developer's model, with its own comparison and its own Earth-orientation code: the same six worst distances to the millimetre. Scored a third time from the protocol alone on 2026-10-10 by an agent that did not write the card, with its own Earth-orientation code: worst distances within 7.8 cm of the record.".into(),
        ],
        "grasp_lift_glass_4tu" => card_grasp_lift_glass_4tu_descriptor::validation(specimen),
        "momentum_jwst_srp" => card_momentum_jwst_srp_descriptor::validation(specimen),
        "momentum_jwst_dihedral" => card_momentum_jwst_dihedral_descriptor::validation(specimen),
        "generator_mmrtg_seebeck" => card_generator_mmrtg_seebeck_descriptor::validation(specimen),
        "crater_ryugu_sci" => [
            "The crater made on asteroid 162173 Ryugu on 5 April 2019 by the Small Carry-on Impactor of JAXA's Hayabusa2: a 2 kg copper projectile at 2 km/s into a rubble surface under a gravity of 9.3e-5 m/s2. The compared quantity is the crater's apparent radius, at the level of the original surface.".into(),
            "The law of crater size and its two constants: thirteen impact craters in dense dry Ottawa Flintshot sand at 1, 111 and 464 times Earth's gravity, Schmidt (1980), Proc. Lunar Planet. Sci. Conf. 11, 2099, Table 1, transcribed from the scan at the NASA Astrophysics Data System. The clock: seven points read by eye from Fig. 4 of Gault and Wedekind (1977), Impact and Explosion Cratering, 1231. The projectile and the target point: JAXA's press briefing of 2019-03-18, before the impact. Ryugu's mass, density, size and rotation: the JPL Small-Body Database, citing Watanabe et al. (2019). Held back, and no part of any constant: the measured crater, apparent diameter 14.5 +/- 0.8 m, Arakawa et al. (2020), Science 368, 67, as given in JAXA's summary of 2020-03-20.".into(),
            "Fixed before any centrifuge crater was fitted or predicted and before the report of the Ryugu crater was opened (protocol SHA-256 ee2793f1ce97b42ed07210121d4fb7cc2f3599db1db1a46a9530efd3cb9da9b3). In the laboratory: fitted on the nine craters made at 1 g, the law must put each of the four centrifuge craters, 2.5 to 4.5 decades away in scaled gravity, inside its 95 percent prediction interval, with a mean error of at most 10 percent. On Ryugu: the radius at which the card stops must be within an allowance worked out by rule from the fit's 95 percent prediction interval at Ryugu, the measurement's stated uncertainty and the uncertainty of the local gravity.".into(),
            CRATER_RESULT.into(),
            "Nothing is fitted to Ryugu, and Ryugu lies 2.5 decades below the smallest laboratory crater in scaled gravity: the comparison is an extrapolation, in gravity and in material. The crater's measured diameter has been public since 2019 and was known to the writer of the protocol, which is why the Ryugu line is a rule and not a chosen number. The comparison is not blind: the measured diameter and a prediction of 8.6 m worked out by hand were in hand before the protocol's hash was recorded, and all of Schmidt's Table 1, the four held-back centrifuge craters included, was seen while it was transcribed. One crater, measured once. The measured crater is a half circle, stopped on its south side by a block, and the projectile came in at 60 degrees to the surface; the card is a full circle from a square hit. The impactor was a hollow copper shell 13 cm across, of 2 kg; the card models it as a solid copper sphere of the same mass, of radius 3.76 cm, at the full 2 km/s and under gravity reduced by the rotation. With the projectile taken as the 13 cm shell the same law gives 7.83 m, 8.0 percent above the measurement and inside the allowance; with the published team's own inputs (the shell, 2 km/s, g = 1.2e-4 m/s2) it gives 7.50 m, 3.5 percent above. Across twenty reasonable readings of the inputs the prediction runs from 7.15 to 8.80 m, wider than the miss. The projectile's radius and the 60-degree angle each move the prediction more than the rotation term does, and neither is in the allowance. The miss stands as registered: it is a miss of the scaling of dense Ottawa Flintshot sand with these inputs, and it does not show that Ryugu's crater departs from sand scaling. The measurement was read from JAXA's summary of the paper, not from the paper. The card has no cohesion, no boulders, no rim, no depth and no ejecta. Its time is a laboratory clock that runs until the ejecta near the rim have landed, read by eye from a printed figure; no line was set on time. Its time constant, 1.318295773573555, comes from seven points read by eye at 1 g; the card stops at 400 s, where the open sources say growth for about 200 s and a stop by about 300 s. The clock is untested. The first 1.75 m of growth is outside the card. One party wrote the card and ran it. It was scored twice, the second time by a second scorer, on one machine.".into(),
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

/// A model calibrated to the record gives, where its word and its record are
/// given, the served model it stands beside and what of the record was seen
/// before its structure or constants were chosen (`find_model::CALIBRATIONS`).
/// Every other model gives neither.
pub(crate) fn calibration_into(map: &mut Map<String, Value>, model: &str) {
    if let Some(row) = crate::find_model::calibration_of(model) {
        map.insert("companion_to".to_string(), json!(row.companion_to));
        map.insert("not_blind".to_string(), json!(row.not_blind));
    }
}

/// The result of the validation record of a card that is one of the
/// company's own specimens.
pub(crate) const OWN_SPECIMEN_RESULT: &str = "Reproduced: an outside recomputation in arithmetic of 50 digits or more gives the card's runs, and every step holds the card's conservation ledger to its stated limit.";

/// The result of the validation record of a solver that is one of the
/// company's own specimens. It claims no outside recomputation, which no such
/// solver has: what it states is the solver's own ledgers and where what its
/// method was held to is written.
pub(crate) const OWN_SPECIMEN_SOLVER_RESULT: &str = "The company's own specimen: its constants are the company's, stated in its description, and every answer holds the conservation ledgers stated under invariants to their tolerances; what its method was held to, and how it did there, is under reference.";

/// The validation record of one of the company's own specimens: what it is,
/// the constants it holds, and what its runs are held to.
fn own_specimen_record(specimen: &str) -> [String; 5] {
    [
        format!("{specimen} The company's own specimen: an object the company specified, every constant of which is stated in this description and fixed in the card."),
        "The company's own constants for this specimen, fixed in the card and listed in its description and in its step rule.".into(),
        "Every step holds the card's own equations and its conservation ledger to the limit the card states, and a run is reproduced by an outside recomputation in arithmetic of 50 digits or more.".into(),
        OWN_SPECIMEN_RESULT.into(),
        "The company's own specimen: its numbers are those of the object the company specified, and a run is that object's run under the card's equations and inside the card's stated limits. A real object of the same kind is described by its own measurements.".into(),
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
    body
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
    json!({
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
    })
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
        ("energy", "reactor_spert3") => reactor_examples(),
        ("energy", "cell_2170_m80") => cell_2170_examples(),
        ("signal", "cantilever_eth") => cantilever_examples(),
        ("energy", "cell_2170_model3") => cell_2170_model3_examples(),
        ("materials", "creep_cmsx6") => creep_examples(),
        ("aerospace", "capsule_insight") => capsule_insight_examples(),
        ("energy", "reactor_spert3_fuel") => reactor_fuel_examples(),
        ("energy", "reactor_triga_jsi") => reactor_triga_examples(),
        ("energy", "reactor_spert3_ccore") => reactor_ccore_examples(),
        ("energy", "runaway_18650_lco") => runaway_18650_lco_examples(),
        ("aerospace", "crater_ryugu_sci") => crater_examples(),
        ("energy", "generator_mmrtg_seebeck") => card_generator_mmrtg_seebeck_descriptor::examples(),
        ("aerospace", "momentum_jwst_dihedral") => card_momentum_jwst_dihedral_descriptor::examples(),
        ("materials", "grasp_lift_glass_4tu") => card_grasp_lift_glass_4tu_descriptor::examples(),
        ("aerospace", "momentum_jwst_srp") => card_momentum_jwst_srp_descriptor::examples(),
        ("signal", "link_gothenburg_10010") => link_examples(),
        ("energy", "grid_gb") => grid_gb_examples(),
        ("aerospace", "solar_system") => solar_system_examples(),
        ("materials", "tire_eve_r19_ice") => tire_eve_r19_ice_examples(),
        ("aerospace", "los_jwst_fine_guide") => los_examples(),
        ("energy", "tokamak_jet_dte1") => tokamak_examples(),
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

/// One step at the card's ceiling from cold start, and a run to the first
/// maximum of power for a mid-range reactivity: the reactor period of opened
/// test 51 (72.3 ms, coolant 82 degF) converted for the card's own set. The run
/// asks for more steps than it needs and ends in the sealed refusal.
/// The opened trip of 18 April 2019: one pole of the interconnector to France,
/// 996 MW, with 241 GVA s and 27,132 MW on the system, from 49.942 Hz. One
/// step at the card's ceiling, and 2,880 steps, 59.8 s, with a frame every 48.
fn grid_gb_examples() -> (Value, Value) {
    let card = grid_gb::GridGb::interconnector_loss();
    pair_examples_recorded(
        "energy",
        "grid_gb",
        obj(&[
            ("frequency_hz", scalar_value(49.942)),
            ("inertia_mw_s", scalar_value(241_000.0)),
            ("demand_mw", scalar_value(27_132.0)),
        ]),
        obj(&[("loss_mw", scalar_value(996.0))]),
        card_dt(card.gate_dt_s()),
        2_880,
        48,
    )
}

fn reactor_examples() -> (Value, Value) {
    let reactor = reactor_spert3::ReactorSpert3::e_core_cold_startup();
    let coolant_k = (82.0 - 32.0) / 1.8 + 273.15;
    let state = reactor_spert3::ReactorState::try_cold_start(&reactor, coolant_k).expect("cold");
    pair_examples_recorded(
        "energy",
        "reactor_spert3",
        obj(&[("coolant_temperature_k", scalar_value(state.coolant_temperature_k))]),
        obj(&[("reactivity_inserted_dollar", scalar_value(0.9982818460448638))]),
        card_dt(reactor.gate_dt_s()),
        3000,
        100,
    )
}

/// One step at the card's ceiling from the start in the cold band, at 82 degF
/// with no flow, and a run to the first maximum of power for one dollar
/// inserted. The run asks for more steps than it needs and ends in the sealed
/// refusal.
fn reactor_fuel_examples() -> (Value, Value) {
    let reactor = reactor_spert3_fuel::ReactorSpert3Fuel::e_core_low_initial_power();
    let inlet_k = (82.0 - 32.0) / 1.8 + 273.15;
    let state = reactor_spert3_fuel::ReactorState::try_start(&reactor, inlet_k).expect("start");
    let band = reactor.band_of(inlet_k).expect("the cold band");
    pair_examples_recorded(
        "energy",
        "reactor_spert3_fuel",
        obj(&[("inlet_temperature_k", scalar_value(state.inlet_temperature_k))]),
        obj(&[
            ("reactivity_inserted_dollar", scalar_value(1.0)),
            (
                "coolant_flow_m_per_s",
                scalar_value(reactor.band_flow_min_m_per_s[band]),
            ),
        ]),
        card_dt(reactor.gate_dt_s()),
        REACTOR_FUEL_EXAMPLE_STEPS,
        100,
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

/// The steps the worked run of the fuel-temperature reactor asks for: more
/// than the run to the first maximum takes.
pub(crate) const REACTOR_FUEL_EXAMPLE_STEPS: u64 = 5_000;

/// One second of a 2.25 A discharge from a cell just recharged, in the 25 degC
/// block with its case at 25 degC, and ten minutes of the same with a frame
/// every minute.
fn cell_2170_examples() -> (Value, Value) {
    let cell = cell_2170_m80::Cell2170M80::cell();
    pair_examples_recorded(
        "energy",
        "cell_2170_m80",
        obj(&[
            ("block_chamber_c", scalar_value(25.0)),
            ("charge_removed_ah", scalar_value(0.0)),
        ]),
        obj(&[
            ("current_a", scalar_value(-cell_2170_m80::PULSE_CURRENT_A)),
            ("case_temperature_c", scalar_value(25.0)),
        ]),
        card_dt(cell.gate_dt_s()),
        600,
        60,
    )
}

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

fn cell_2170_model3_examples() -> (Value, Value) {
    let cell = cell_2170_model3::Cell2170Model3::cell();
    pair_examples_recorded(
        "energy",
        "cell_2170_model3",
        obj(&[
            ("cell_number", scalar_value(cell_2170_model3::CELLS[0].cell_number)),
            ("state_of_charge", scalar_value(1.0)),
            ("capacity_ah", scalar_value(4.5006)),
            ("case_temperature_least_c", scalar_value(25.0)),
            ("case_temperature_most_c", scalar_value(25.0)),
        ]),
        obj(&[
            ("current_a", scalar_value(4.5)),
            ("case_temperature_c", scalar_value(25.0)),
        ]),
        card_dt(cell.gate_dt_s()),
        600,
        60,
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

/// One step at the card's ceiling of a 4.5 MJ plasma at 3.8 MA and 4.0 T
/// under 20 MW of heating in an even D-T mix, at the start of the heated
/// window, and the first second of it: 1,000 steps, a frame every 25th step.
fn tokamak_examples() -> (Value, Value) {
    let tokamak = tokamak_jet_dte1::TokamakJetDte1::elmy_h_mode();
    pair_examples_recorded(
        "energy",
        "tokamak_jet_dte1",
        obj(&[
            ("thermal_energy_j", scalar_value(4.0e6)),
            ("fast_ion_energy_j", scalar_value(0.5e6)),
            ("elapsed_time_s", scalar_value(0.0)),
        ]),
        obj(&[
            ("heating_power_w", scalar_value(20.0e6)),
            ("ohmic_power_w", scalar_value(0.5e6)),
            ("line_density_per_m3", scalar_value(6.0e19)),
            ("plasma_current_a", scalar_value(3.8e6)),
            ("toroidal_field_t", scalar_value(4.0)),
            ("tritium_fraction", scalar_value(0.5)),
        ]),
        card_dt(tokamak.gate_dt_s()),
        1_000,
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
        ("grid_gb", "NonFiniteInput") => "The power lost or `dt` was NaN or infinite.",
        ("grid_gb", "NonFiniteState") => "The incoming state, or the trial step, was non-finite.",
        ("grid_gb", "NonPositiveDt") => "`dt` was zero or negative.",
        ("grid_gb", "InvalidCard") => "A constant is not this card's.",
        ("grid_gb", "InertiaNotPositive") => {
            "The stored kinetic energy at 50 Hz is zero or less: the rate of change of frequency divides by it."
        }
        ("grid_gb", "DemandNegative") => "The demand is below zero.",
        ("grid_gb", "LossLargerThanDemand") => {
            "The power lost is larger in size than the demand: the area cannot lose more infeed, or more demand, than it carries."
        }
        ("grid_gb", "NegativeTime") => "The clock is below zero.",
        ("grid_gb", "LowFrequencyDemandDisconnection") => {
            "The frequency, incoming or landing, is at or below 48.8 Hz, where low-frequency demand disconnection begins."
        }
        ("grid_gb", "FrequencyAboveCard") => {
            "The frequency, incoming or landing, is above 50.5 Hz, the upper statutory limit, where the card holds no over-frequency response."
        }
        ("grid_gb", "SpontaneousGeneration") => "The trial step created kinetic energy from nothing.",
        ("grid_gb", "FirstLawBreach") => "The trial step destroyed kinetic energy.",
        ("reactor_spert3", "NonFiniteInput") => {
            "`dt` or the held reactivity was NaN or infinite."
        }
        ("reactor_spert3", "NonFiniteState") => {
            "The incoming state, or the trial step, was non-finite."
        }
        ("reactor_spert3", "NonPositiveDt") => {
            "`dt` was zero or negative."
        }
        ("reactor_spert3", "InvalidCard") => {
            "A recorded constant is not this reactor's."
        }
        ("reactor_spert3", "ReactivityNotPositive") => {
            "The held reactivity is zero or less: no step in reactivity is inserted, and there is no burst."
        }
        ("reactor_spert3", "CoolantTemperatureNotLiquidWater") => {
            "The recorded primary coolant temperature is below 273.15 K, where water freezes, or above 647.096 K, the critical point of water, above which there is no liquid at any pressure."
        }
        ("reactor_spert3", "PastFirstMaximum") => {
            "The state is past the first maximum of power; nothing after it is on the card."
        }
        ("reactor_spert3", "NegativeTime") => {
            "The incoming state holds a time below zero."
        }
        ("reactor_spert3", "DtAboveStateCeiling") => {
            "The step is longer than 0.2 of the time constant of the fastest pole at the state it starts from or at the state it would land on, or 0.07 of it when that pole is growing: the prompt pole |1 - net reactivity| / l*, or the feedback pole the square root of a P / l*."
        }
        ("reactor_spert3", "NegativeState") => {
            "The incoming state holds an energy released or a precursor sum below zero, or a power that is not above zero."
        }
        ("reactor_spert3", "InconsistentState") => {
            "The incoming state is not one this card writes: at time zero it is not the card's start, or a field the card derives from the rest of the state, under the reactivity of the last landed step that the state records, does not hold the value the card derives."
        }
        ("reactor_spert3", "SpontaneousGeneration") => {
            "The trial step created neutron population beyond what the reactivity supplies."
        }
        ("reactor_spert3", "FirstLawBreach") => {
            "The trial step destroyed neutron population the reactivity supplies."
        }
        ("reactor_spert3_ccore", "NonFiniteInput") => {
            "`dt`, the reactivity or the flow was NaN or infinite."
        }
        ("reactor_spert3_ccore", "NonFiniteState") => {
            "The incoming state, or the trial step, was non-finite."
        }
        ("reactor_spert3_ccore", "NonPositiveDt") => "`dt` was zero or negative.",
        ("reactor_spert3_ccore", "InvalidCard") => "A recorded constant is not this reactor's.",
        ("reactor_spert3_ccore", "PressureNotPositive") => {
            "The system pressure is at or below absolute zero: a gauge pressure at or below −14.6959488 psig."
        }
        ("reactor_spert3_ccore", "InitialTemperatureNotLiquidWater") => {
            "The initial temperature is not that of liquid water at the system pressure: below 273.15 K, or at or above the saturation temperature of water at that pressure (IAPWS-IF97 region 4), or above the critical pressure, 22.064 MPa, at or above the critical temperature, 647.096 K."
        }
        ("reactor_spert3_ccore", "NegativeFlow") => "The coolant flow is below zero.",
        ("reactor_spert3_ccore", "ReactivityNotPositive") => {
            "The held reactivity is zero or less: no step in reactivity is inserted, and there is no burst."
        }
        ("reactor_spert3_ccore", "PastPlateTemperatureMaximum") => {
            "The state is past the first maximum of the hot-spot plate temperature after the power peak; nothing after it is on the card."
        }
        ("reactor_spert3_ccore", "NegativeTime") => "The incoming state's clock is below zero.",
        ("reactor_spert3_ccore", "NegativeState") => {
            "The incoming state holds an energy, a temperature rise, a swept energy, a precursor sum or a flow below zero, or a power not above zero."
        }
        ("reactor_spert3_ccore", "DtAboveStateCeiling") => {
            "The step is longer than the state lets it be, at the state it starts from or at its trial end: one Runge–Kutta step would advance the growing prompt pole (net reactivity − 1) / l* by more than 0.0611 of its time constant, the decaying prompt pole (1 − net reactivity) / l* by more than 0.5, or the plate and water heat transfer and sweep by more than 0.0184."
        }
        ("reactor_spert3_ccore", "ReactivityChangedDuringRun") => {
            "The reactivity of the load is not the one of the first step of this run; a run is one step of reactivity."
        }
        ("reactor_spert3_ccore", "InconsistentState") => {
            "The incoming state is not one this card writes: at time zero it is not the card's start; later, a derived field, a flag or a peak field is not what the card computes from the rest of the state."
        }
        ("reactor_spert3_ccore", "SpontaneousGeneration") => {
            "The trial step created neutrons or energy beyond what the ledger supplies."
        }
        ("reactor_spert3_ccore", "FirstLawBreach") => {
            "The trial step destroyed neutrons or energy the ledger supplies."
        }
        ("runaway_18650_lco", "NonFiniteInput") => {
            "The oven temperature, the heater power, the adiabatic switch or `dt` was NaN or infinite."
        }
        ("runaway_18650_lco", "NonFiniteState") => {
            "The incoming state, or a substep, was not finite."
        }
        ("runaway_18650_lco", "NonPositiveDt") => {
            "`dt` was zero or negative."
        }
        ("runaway_18650_lco", "InvalidCell") => {
            "A recorded constant is not this cell's."
        }
        ("runaway_18650_lco", "OvenTemperatureNotPositive") => {
            "The oven temperature is zero kelvin or below while the cell loses heat to the oven, adiabatic 0; with adiabatic 1 it does not enter and is not tested."
        }
        ("runaway_18650_lco", "HeaterPowerNegative") => {
            "The heater power is below zero: a heater gives heat and does not take it."
        }
        ("runaway_18650_lco", "AdiabaticNotZeroOrOne") => {
            "The adiabatic switch is neither 0 nor 1."
        }
        ("runaway_18650_lco", "TemperatureNotPositive") => {
            "The cell temperature, incoming or in a substep, is zero kelvin or below."
        }
        ("runaway_18650_lco", "CanMelting") => {
            "The cell temperature, incoming or in a substep, is at or above 1811 K, the melting point of iron: the steel can melts."
        }
        ("runaway_18650_lco", "SeiFractionOutsideCell") => {
            "The lithium in the SEI is below 0 or above 1: a fraction, it is between none and all."
        }
        ("runaway_18650_lco", "AnodeLithiumOutsideCell") => {
            "The lithium in the graphite is below 0 or above 0.783: above it the SEI thickness, 0.783 less this, would be below zero."
        }
        ("runaway_18650_lco", "CathodeUnreactedOutsideCell") => {
            "The unreacted part of the cathode is below 0 or above 1, or above 1 - 2^-33 and under 1: above 1 the converted part would be below zero and the cathode stage would make cathode; just under 1 the converted part, under 1.16e-10, is carried to fewer than 20 bits and its growth is lost to rounding."
        }
        ("runaway_18650_lco", "ElectrolyteOutsideCell") => {
            "The electrolyte is below 0 or above 1: a fraction of the cell's fill, it is between none and all."
        }
        ("runaway_18650_lco", "EveryReactantConsumed") => {
            "The step would carry the cell to every reactant consumed: the heat the lithium in the SEI and in the graphite, the unreacted cathode and the electrolyte still hold falls to 1e-6 of the cell's chemical heat or under, 0.0205 J, and the runaway has finished."
        }
        ("runaway_18650_lco", "SubstepsAboveBudget") => {
            "The step would need more than 1,048,576 substeps to follow its fastest reaction."
        }
        ("runaway_18650_lco", "SpontaneousGeneration") => {
            "The trial step created heat: the heat stored in the cell rose above the heat released, plus the heater's, less the heat lost."
        }
        ("runaway_18650_lco", "FirstLawBreach") => {
            "The trial step destroyed heat: the heat stored in the cell fell below the heat released, plus the heater's, less the heat lost."
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
        ("reactor_spert3_fuel", "NonFiniteInput") => {
            "`dt`, the reactivity or the flow rate was NaN or infinite."
        }
        ("reactor_spert3_fuel", "NonFiniteState") => {
            "The incoming state, or the trial step, was non-finite."
        }
        ("reactor_spert3_fuel", "NonPositiveDt") => {
            "`dt` was zero or negative."
        }
        ("reactor_spert3_fuel", "InvalidCard") => {
            "A recorded constant is not this reactor's."
        }
        ("reactor_spert3_fuel", "InletTemperatureOutsideCard") => {
            "The recorded inlet temperature is in none of the three tested bands: 56 to 92, 251 to 259, 498 to 501 degF."
        }
        ("reactor_spert3_fuel", "ReactivityNotPositive") => {
            "The held reactivity is zero or less: no step in reactivity is inserted, and there is no burst."
        }
        ("reactor_spert3_fuel", "NegativeFlow") => {
            "The coolant flow rate is below zero: the flow runs through the core from the inlet, and a flow the other way is not a flow rate of this core."
        }
        ("reactor_spert3_fuel", "PastFirstMaximum") => {
            "The state is past the first maximum of power; nothing after it is on the card."
        }
        ("reactor_spert3_fuel", "NegativeTime") => {
            "The incoming state holds a time below zero."
        }
        ("reactor_spert3_fuel", "DtAboveStateCeiling") => {
            "The step is longer than 0.2 of the time constant of the fastest pole at the state it starts from or at the state it would land on, or 0.045 of it when that pole is growing: the prompt pole |1 - net reactivity| / l*, the feedback pole of the fuel and the water temperature, or the sweep 2 v / L of the water."
        }
        ("reactor_spert3_fuel", "NegativeState") => {
            "The incoming state holds an energy released, a temperature rise or a precursor sum below zero, or a power that is not above zero."
        }
        ("reactor_spert3_fuel", "ReactivityChangedDuringBurst") => {
            "The reactivity of the load is not the one the first step of this burst sent the rod to; a burst is one insertion, ramped in and then held."
        }
        ("reactor_spert3_fuel", "InconsistentState") => {
            "The incoming state is not one this card writes: at time zero it is not the card's start; later, its clock, inserted reactivity and rod flag are not on the ramp to the burst's reactivity, or its power rate, net reactivity, ledger anchor, defect or residual, a flag or a peak field is not what the card computes from the rest of the state."
        }
        ("reactor_spert3_fuel", "SpontaneousGeneration") => {
            "The trial step created neutron population beyond what the reactivity supplies."
        }
        ("reactor_spert3_fuel", "FirstLawBreach") => {
            "The trial step destroyed neutron population the reactivity supplies."
        }
        ("cell_2170_m80", "NonFiniteInput") => {
            "Current, case temperature or `dt` was NaN or infinite."
        }
        ("cell_2170_m80", "NonFiniteState") => {
            "The incoming state, or the trial step, was non-finite."
        }
        ("cell_2170_m80", "NonPositiveDt") => "`dt` was zero or negative.",
        ("cell_2170_m80", "InvalidCell") => "A recorded constant or table entry is not this cell's.",
        ("cell_2170_m80", "StepNotOneSecond") => "`dt` is not exactly one second.",
        ("cell_2170_m80", "BlockNotRecognized") => {
            "The block of the state is not one of the six chamber settings."
        }
        ("cell_2170_m80", "CaseTemperatureOutsideCard") => {
            "Case temperature is below -21.898 degC or above 45.535 degC, beyond the coldest and the warmest block's own tests, where the tables would hold their end values."
        }
        ("cell_2170_m80", "CurrentOutsideCard") => {
            "The current is above 13.5 A charging or discharging, the end of the ten-second table's current axis."
        }
        ("cell_2170_m80", "ChargeOutsideTables") => {
            "Charge removed, incoming or landing, is outside the tables of the block by more than the rounding allowance."
        }
        ("cell_2170_m80", "LowerVoltageLimit") => "Landing terminal voltage is below 2.8 V.",
        ("cell_2170_m80", "UpperVoltageLimit") => "Landing terminal voltage is above 4.242 V.",
        ("cell_2170_m80", "SpontaneousGeneration") => "The trial step created energy from nothing.",
        ("cell_2170_m80", "FirstLawBreach") => "The trial step destroyed energy.",
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
        ("cell_2170_model3", "NonFiniteInput") => {
            "Current, case temperature or `dt` was NaN or infinite."
        }
        ("cell_2170_model3", "NonFiniteState") => {
            "The incoming state, or the trial step, was non-finite."
        }
        ("cell_2170_model3", "NonPositiveDt") => "`dt` was zero or negative.",
        ("cell_2170_model3", "InvalidCell") => {
            "A recorded constant or curve entry is not this cell type's."
        }
        ("cell_2170_model3", "CellNotRecognized") => {
            "The cell number of the state names none of the cells the card holds."
        }
        ("cell_2170_model3", "CapacityNotPositive") => "Capacity is zero or less: the charge is counted against it.",
        ("cell_2170_model3", "CaseTemperatureBelowAbsoluteZero") => {
            "Case temperature is at or below absolute zero, -273.15 degC, where the temperature laws have no value."
        }
        ("cell_2170_model3", "CaseTemperatureSpanInverted") => {
            "The least case temperature the state carries is above the most it carries."
        }
        ("cell_2170_model3", "StateOfChargeOutsideCard") => {
            "State of charge, incoming or landing, is below -0.05 or above 1.05, the ends of the card's rest-voltage curves, outside which it has no rest voltage, by more than the rounding allowance of its running sum."
        }
        ("cell_2170_model3", "SurfaceOutsideCurve") => {
            "Landing surface state of charge is below -0.05 or above 1.05, the ends of the card's rest-voltage curves, outside which it has no rest voltage."
        }
        ("cell_2170_model3", "UndervoltageLimit") => "Landing terminal voltage is below 2.8 V.",
        ("cell_2170_model3", "OvervoltageLimit") => "Landing terminal voltage is above 4.242 V.",
        ("cell_2170_model3", "SpontaneousGeneration") => {
            "The trial step created energy from nothing."
        }
        ("cell_2170_model3", "FirstLawBreach") => "The trial step destroyed energy.",
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
            "The incoming creep extension is above 5 percent, the largest level of the protocol."
        }
        ("creep_cmsx6", "CreepExtensionBeyondCard") => {
            "The trial step passed 5 percent creep extension, the largest level of the protocol."
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
        ("tokamak_jet_dte1", "NonFiniteInput") => "`dt` or a load value was NaN or infinite.",
        ("tokamak_jet_dte1", "NonFiniteState") => {
            "The incoming state, or the trial step, was non-finite."
        }
        ("tokamak_jet_dte1", "NonPositiveDt") => "`dt` was zero or negative.",
        ("tokamak_jet_dte1", "InvalidCard") => "A recorded constant is not this tokamak's.",
        ("tokamak_jet_dte1", "HeatingPowerNegative") => {
            "The heating power is below zero."
        }
        ("tokamak_jet_dte1", "OhmicPowerNegative") => {
            "The ohmic power is below zero."
        }
        ("tokamak_jet_dte1", "PlasmaCurrentNotPositive") => {
            "The plasma current is zero or less: the confinement law takes its logarithm and the safety factor divides by it."
        }
        ("tokamak_jet_dte1", "ToroidalFieldNotPositive") => {
            "The toroidal field is zero or less: the confinement law takes its logarithm and the normalised beta divides by it."
        }
        ("tokamak_jet_dte1", "FuelMixOutsideCard") => {
            "The tritium fraction of the fuel is below 0 or above 1, which no fraction of the fuel atoms can be."
        }
        ("tokamak_jet_dte1", "DensityNotPositive") => {
            "The line-averaged density is zero or less: the slowing-down time divides by it."
        }
        ("tokamak_jet_dte1", "DensityAboveGreenwald") => {
            "The line-averaged density is above the Greenwald density of the plasma current."
        }
        ("tokamak_jet_dte1", "SafetyFactorBelowLimit") => {
            "The edge safety factor of the current and the field is below 2."
        }
        ("tokamak_jet_dte1", "EnergyOutsideCard") => {
            "The thermal energy is zero or negative, or the fast-ion energy is negative."
        }
        ("tokamak_jet_dte1", "TemperatureBelowReactivityFit") => {
            "The plasma temperature is below 0.2 keV, the floor of the reactivity fit."
        }
        ("tokamak_jet_dte1", "TemperatureAboveReactivityFit") => {
            "The temperature at the centre of the profile is above 100 keV, the ceiling of the reactivity fit."
        }
        ("tokamak_jet_dte1", "BetaAboveTroyon") => {
            "The normalised beta of the two stores is above the Troyon limit of 2.8, at the start of the step or where the trial step lands."
        }
        ("tokamak_jet_dte1", "NegativeTime") => {
            "The clock, the time since the start of the heated window, is negative."
        }
        ("tokamak_jet_dte1", "StepBelowClockResolution") => {
            "The step is too small to move the clock at its time in 64-bit arithmetic."
        }
        ("tokamak_jet_dte1", "DtAboveStateCeiling") => {
            "The step is longer than the ceiling of the state it starts from or where its trial step lands: the smallest of an eighteenth of the slowing-down time, a forty-sixth of 0.31 times the confinement time and a forty-eighth of the coupling time of the fast ions and the thermal plasma."
        }
        ("tokamak_jet_dte1", "SpontaneousGeneration") => {
            "The trial step created energy beyond what the heating and the fusion supply: a defect of the arithmetic or the integrator."
        }
        ("tokamak_jet_dte1", "FirstLawBreach") => {
            "The trial step destroyed energy the heating and the fusion supply: a defect of the arithmetic or the integrator."
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
pub(crate) const TOKAMAK_VDE_MAST_WHAT: &str = "The company's own derivation, from the MAST spherical tokamak (UKAEA, Culham) at 0.250 s of shot 15045, the EFIT time slice just before the vertical-position feedback was cut on purpose to make an upward vertical displacement event. The plasma is rigid: EFIT's toroidal current density at 0.250 s, 878 cells and 755.3 kA, shifted bodily in height, with its current and its mass (deuterium at the line-average density, 5.98e-7 kg) held. It moves in the field of MAST's PF coils at their 0.250 s currents and of the passive currents MAST's AMM signals give at that time. Its motion induces eddy currents in the 126 passive elements of MAST's machine description (the vessel, the centre column's Inconel casing and graphite, the P2 arms and divertor plates, the stainless coil cases), held as one wall circuit in the shape of the growing mode of the whole set, and in the P6 radial-field pair, a circuit of its own driven by the fast amplifier's voltage. The ledger holds the magnetic energy of the two circuits, the plasma's kinetic energy and its energy in the held field against the resistive losses and the amplifier's work. The event ends when EFIT's boundary, shifted, reaches the limiter: at +0.389 m and at -0.391 m, the corner at R 1.04 m, Z 1.033 m, by the P3 coil, where the halo currents of shot 15045 flowed.";

/// The data behind the MAST VDE card.
pub(crate) const TOKAMAK_VDE_MAST_DATA: &str = "FAIR-MAST, the open archive of MAST (S. Jackson, S. Khan, N. Cummings, J. Hodson, S. de Witt, S. Pamela, R. Akers, J. Thiyagalingam and the MAST Team, SoftwareX 27 (2024) 101869; IEEE Transactions on Plasma Science (2025), doi 10.1109/TPS.2025.3583419), level 2, shot 15045: EFIT's equilibrium (current density, flux, boundary, scalars), the PF coil currents and geometry, the passive structure's geometry and AMM currents, the limiter contour, the line-average density, the centre-column Bv array and the flux loops. Licence, in every group's metadata: Creative Commons 4.0 BY-SA. The 692 files read are kept as served in the company's research record, with the time, status and SHA-256 of each. The constants derived from them: every coil element one turn, the channel current of P2 to P6 the coil's ampere-turns and the solenoid's elements half its channel (with them EFIT's flux at 0.250 s outside the plasma is reproduced to 1.1 percent of its variation, root mean square; 0.73 percent with each coil's scale fitted free, which is how the convention was found); resistivities from the AMM currents over the flux-loop voltages where a loop is near an element (Inconel 1.29e-6, graphite 13.5e-6, divertor plates 3.3e-6 ohm m) and stainless steel 304 at 20 degC, 0.72e-6, for the vessel and the coil cases; P6 copper at 20 degC with nothing for the amplifier and feeders. The measurement compared with: the growth rate of shot 15045 published by M. J. Windridge, T. C. Hender, G. Cunningham, J. B. Lister, V. Lukash, R. Khayrutdinov and V. Dokuka, 34th EPS Conference on Plasma Physics (2007) P1.108, 96.3 per second from the centre-column Bv signal, with no window stated; the centre-column Bv probes themselves; and the P6 currents.";

/// The pass lines of the MAST VDE card.
pub(crate) const TOKAMAK_VDE_MAST_PASS_LINE: &str = "Fixed on 2026-10-14 and committed before any growth rate of the card or of the measurement was computed (evidence/research/tokamak_vde_mast/FORECAST.md, SHA-256 3743912cffaa1274685092dd247e5aaf596f7af0f1e8275be7a1e3ccdc4b9f70, with the measurement program analysis/measure_15045.py, b758bd068d44056c3cbb189c4fb18b367769d92094c41e872bd98e33ec9de7fa, in the commit 3e3d2d7 of the kernel). The measured position is the vertex of a least-squares parabola through the centre-column probes within 0.25 m of the one reading the most, as Windridge's signal is the peak of that field; its baseline the mean from 0.200 to 0.240 s; its window from the first sample after 0.250 s at which it is 1 cm above the baseline and stays above, to 0.16 of EFIT's minor radius at 0.250 s (8.97 cm, TCV's linear-phase rule); its growth rate the least-squares slope of the logarithm. Line 1: the card's linear growth rate within 25 percent of Windridge's 96.3 per second. Line 2: the growth rate of the card's own synthetic signal, from a run through the engine started at 1 mm on its growing mode at zero volts, by the same rule, within 25 percent of the measured one. Line 3: the P6 current at the end of the card's window within 25 percent of the measured one at the end of the measured window. All three met is met, one or two met in part, none missed.";

/// The result of the MAST VDE card's comparison.
pub(crate) const TOKAMAK_VDE_MAST_RESULT: &str = "Compared once on 2026-10-14 by the rule fixed beforehand: met in part, one line of three. Line 1: the card's linear growth rate is 6.55 per second against Windridge's 96.3, 93.2 percent low: missed. Line 2: by the forecast's rule the measured probes grow at 121.3 per second, over 0.2608 to 0.2746 s of the shot (1.97 to 9.05 cm); the card's own synthetic signal, from its run through the engine (729 steps of 1 ms from the growing mode at 1 mm, ended at the wall), grows at 7.10 per second over 0.353 to 0.663 s of the run: 94.1 percent low, missed. Line 3: at the end of the two windows P6U carries -2,933 ampere-turns in the card and -3,126 in the shot, 6.2 percent low: met. Line 3 is met with the two events at different speeds: on the card's slow growth P6, whose time is 172 ms, carries 54 percent of the current that would hold its flux, at a centroid height of 8.6 cm; in the shot P6 held its flux while the probes' vertex ran ahead of the magnetic axis. Reported and not scored: the card's run reaches the wall 0.066 to 0.067 s after its window ends, and the shot's current spike came 0.0144 s after its window ended; other windows of the measured signal give 81 to 263 per second, and EFIT's magnetic axis 82 to 155. Where the miss is, found after the scoring and stated as such: at 0.280 s the measured P6 current, 5,011 ampere-turns, is 98.7 percent of the 5,078 that hold its flux for EFIT's axis there, 8.03 cm above its place at 0.250 s, and with that current P6 alone pushes the plasma back harder than the held field's drive at that height, 3,870 N against 2,350 N. With P6 left open the 127 circuits would grow at 90.7 per second; with P6 closed, as the data show it was, they need 2.04 times the rigid shift's stiffness to grow at 96.3. The plasma did not move rigidly: by 0.280 s its boundary's centre had risen 0.175 m and its magnetic axis 0.083 m, and its elongation fell from 1.91 to 1.72. A rigid shift of the 0.250 s current carries about half the drive the event shows.";

/// The limits of the MAST VDE card.
pub(crate) const TOKAMAK_VDE_MAST_LIMITS: &str = "The comparison is not blind. Before the forecast was fixed on 2026-10-14, Windridge's published 96.3 per second, which line 1 scores, was known and quoted in it; the scout's growth rates of the same shot had been seen, 219 per second at onset and 94 later from the centre-column array and 82 to 155 from EFIT's axis; and the PF coil currents had been read to 0.295 s, the P6 pair's among them, about 5 kA by 0.280 s, which line 3, the one line met, scores. The P6 circuit, copper with nothing for its amplifier and feeders, was set as a choice with that trace in view; nothing was fitted to the motion. One shot, one time slice, one tokamak, one maker; no outside party has repeated it. The plasma is rigid: the 0.250 s current density is shifted and not changed, while in shot 15045 the boundary's centre rose 0.175 m by 0.280 s and the magnetic axis 0.083 m, and the elongation fell from 1.91 to 1.72. The held field's force on the unshifted plasma, 113.3 N upward by the derivation's direct sum over EFIT's 878 cells, is set to zero in the card, the plasma taken in equilibrium at z = 0: it is the force of 4.04 mm of the held field's stiffness there, and over the rise to the upper contact its work would be 44.1 J of the 3,209 J the held field releases (44.3 J of 3,245 J down to the lower contact). The PF coils are held at their 0.250 s currents, while P2 fell from 70 to 53 kA-turns by 0.280 s; their up and down halves carry equal currents through the event, as series pairs do, so no part of them is a passive circuit. The wall is one circuit in the shape of the growing mode at z = 0: the card's two circuits have a stability margin of 1.27 there, where all 127 circuits as perfect conductors have 4.13, so the card holds the plasma less firmly against fast motion than the whole wall would, and its ideal-wall limit at +0.238 m and -0.239 m is the card's own and comes farther out for the whole wall. The resistivities of the vessel and the coil cases are a handbook value at 20 degC, those of the other elements are read from the AMM signals, and the P6 circuit's resistance leaves out its amplifier and feeders; P6's ampere-turn convention is the one the other coils' fit established. Halo currents, the thermal quench and the current quench after contact are not in the card. The ledger names a wrong method or a defect of the arithmetic; it does not show the physics matches the tokamak.";

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
        "A run starts at time zero at the displacement, velocity and two currents given, with its ledger anchored there: the energy is the plasma's kinetic energy, its energy in the held field, zero at the equilibrium, and the magnetic energy of the wall mode and P6 with their mutual. Each step holds the amplifier's voltage p6_voltage_v, and a later segment may change it; the ledger books the amplifier's work and the resistive losses of the two circuits over every substep by the corrected trapezoid rule, and holds the defect at every substep to 2e-8 of the energy in play, the size of each term at the anchor or now with the losses and the work, or of 0.01 J when that is smaller. The state reports the held field's force on the plasma, the wall mode's force (the passive structure feels the opposite: the vertical force a VDE puts on the vessel) and P6's, the resistive power of each circuit and the amplifier's, each energy and the ledger's sums, the gap to the nearer contact height and the substeps the last step took. The run ends where the event ends: a step any of whose stages carries EFIT's boundary to the limiter, at +0.389 m or -0.391 m, is refused as PlasmaAtWall and the run is sealed up to the last step that landed, the outcome ended. A step any of whose stages carries the plasma to its Alfven speed is refused as SpeedAtAlfvenSpeed, the run sealed up to the last step that landed. The two are tested at the same stages, so which one a run meets does not follow the step asked for: from rest at the equilibrium with 1 MA in P6 every step from 1e-8 s to 1 ms meets the Alfven speed near 0.30 m, 3.4e-7 s after the start, before the wall. Only where the plasma would reach both within one substep can two steps of different length meet them in a different order, because a step cut otherwise puts the stages of that substep at other times. A frame carries FLAG_PAST_IDEAL_WALL_LIMIT where the landed displacement is past +0.238 m or -0.239 m: there the held field's stiffness, which rises from 28,022.5 N/m at the equilibrium to 109,777 N/m at the upper contact and 109,812 N/m at the lower, is above what the wall mode and P6 as perfect conductors would push back with, and the plasma runs on its inertial time, in microseconds, to the wall. A single step starts again at each call from the state it is given: a returned state's displacement_m, velocity_m_s, wall_current_a and p6_current_a handed back as a new start continue the same motion, with t_s and the ledger begun again. A step of zero or less is refused as NonPositiveDt before the step floor is tested.".to_string(),
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
            ("energy", "battery_sag", vec!["soc", "current_a", "dt_s"]),
            (
                "energy",
                "cell_18650pf",
                vec!["soc", "held_current_a", "current_a", "dt_s"],
            ),
            (
                "energy",
                "thermal",
                vec!["temperature_k", "heat_power_w", "ambient_k", "dt_s"],
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
                vec!["elongation_m", "velocity_m_s", "force_n", "dt_s"],
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
                vec!["epoch_utc_s", "forecast_elapsed_s", "position_m", "velocity_m_s", "dt_s"],
            ),
            (
                "energy",
                "reactor_spert3",
                vec!["coolant_temperature_k", "reactivity_inserted_dollar", "dt_s"],
            ),
            (
                "energy",
                "cell_2170_m80",
                vec![
                    "block_chamber_c",
                    "charge_removed_ah",
                    "current_a",
                    "case_temperature_c",
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
                "energy",
                "reactor_spert3_fuel",
                vec![
                    "inlet_temperature_k",
                    "reactivity_inserted_dollar",
                    "coolant_flow_m_per_s",
                    "dt_s",
                ],
            ),
            ("aerospace", "crater_ryugu_sci", vec!["radius_m", "dt_s"]),
            ("energy", "generator_mmrtg_seebeck", vec!["mission_time_s", "dt_s"]),
            (
                "aerospace",
                "momentum_jwst_dihedral",
                vec!["h_eci_nms", "q1", "q2", "q3", "q4", "sun_x", "sun_y", "sun_z", "sun_distance_au", "dt_s"],
            ),
            ("materials", "grasp_lift_glass_4tu", vec!["hand_velocity_m_s", "grip_force_n", "hand_acceleration_m_s2", "dt_s"]),
            (
                "aerospace",
                "momentum_jwst_srp",
                vec!["h_eci_nms", "q1", "q2", "q3", "q4", "sun_x", "sun_y", "sun_z", "sun_distance_au", "dt_s"],
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
                vec!["frequency_hz", "inertia_mw_s", "demand_mw", "loss_mw", "dt_s"],
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
                    "fast_ion_energy_j",
                    "elapsed_time_s",
                    "heating_power_w",
                    "ohmic_power_w",
                    "line_density_per_m3",
                    "plasma_current_a",
                    "toroidal_field_t",
                    "tritium_fraction",
                    "dt_s",
                ],
            ),
            (
                "materials",
                "fingerpad_p02_glass",
                vec!["twist_deg", "normal_force_n", "twist_rate_deg_s", "dt_s"],
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
                    "oven_temperature_k",
                    "heater_power_w",
                    "adiabatic",
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
                reactor_spert3::ReactorSpert3::e_core_cold_startup().floor_dt_s(),
                reactor_spert3::ReactorSpert3::e_core_cold_startup().gate_dt_s(),
            ),
            (
                "cell_2170_m80",
                cell_2170_m80::Cell2170M80::cell().floor_dt_s(),
                cell_2170_m80::Cell2170M80::cell().gate_dt_s(),
            ),
            (
                "cantilever_eth",
                cantilever_eth::CantileverEth::first_bending_mode().floor_dt_s(),
                cantilever_eth::CantileverEth::first_bending_mode().gate_dt_s(),
            ),
            (
                "cell_2170_model3",
                cell_2170_model3::Cell2170Model3::cell().floor_dt_s(),
                cell_2170_model3::Cell2170Model3::cell().gate_dt_s(),
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
                "reactor_spert3_fuel",
                reactor_spert3_fuel::ReactorSpert3Fuel::e_core_low_initial_power().floor_dt_s(),
                reactor_spert3_fuel::ReactorSpert3Fuel::e_core_low_initial_power().gate_dt_s(),
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
                grid_gb::GridGb::interconnector_loss().floor_dt_s(),
                grid_gb::GridGb::interconnector_loss().gate_dt_s(),
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
                tokamak_jet_dte1::TokamakJetDte1::elmy_h_mode().floor_dt_s(),
                tokamak_jet_dte1::TokamakJetDte1::elmy_h_mode().gate_dt_s(),
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
                reactor_spert3_ccore::ReactorSpert3Ccore::c_core_step_transients().floor_dt_s(),
                reactor_spert3_ccore::ReactorSpert3Ccore::c_core_step_transients().gate_dt_s(),
            ),
            (
                "runaway_18650_lco",
                runaway_18650_lco::Runaway18650Lco::hatchard_kim().floor_dt_s(),
                runaway_18650_lco::Runaway18650Lco::hatchard_kim().gate_dt_s(),
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
                || card == "reactor_spert3_fuel"
                || card == "reactor_spert3_ccore"
                || card == "reactor_triga_jsi"
                || card == "cell_2170_m80"
                || card == "cell_2170_model3"
                || card == "creep_cmsx6"
                || card == "grid_gb"
                || card == "solar_system"
                || card == "tire_eve_r19_ice"
                || card == "fingerpad_p02_glass"
                || card == "los_jwst_fine_guide";
            if card == "grid_gb" {
                assert_eq!(floor.to_bits(), grid_gb::FLOOR_DT_S.to_bits());
            } else if card == "reactor_spert3" {
                assert_eq!(floor.to_bits(), reactor_spert3::FLOOR_DT_S.to_bits());
            } else if card == "reactor_triga_jsi" {
                assert_eq!(floor.to_bits(), reactor_triga_jsi::FLOOR_DT_S.to_bits());
                assert_eq!(floor.to_bits(), 1.0e-9_f64.to_bits());
                // The floor still moves the clock at the end of the longest run, a million
                // steps at the ceiling, 93.9 s.
                assert!(1.0e6 * ceiling + floor > 1.0e6 * ceiling);
                // The ceiling follows the fitted generation time: 0.094 ms.
                assert!((ceiling - 9.39365e-5).abs() < 1.0e-9);
            } else if card == "reactor_spert3_ccore" {
                assert_eq!(floor.to_bits(), reactor_spert3_ccore::FLOOR_DT_S.to_bits());
                assert!(600.0 + floor > 600.0);
                assert_eq!(ceiling.to_bits(), (0.2_f64 * 0.003).to_bits());
            } else if card == "reactor_spert3_fuel" {
                assert_eq!(floor.to_bits(), reactor_spert3_fuel::FLOOR_DT_S.to_bits());
                assert_eq!(floor.to_bits(), 1.0e-9_f64.to_bits());
                // The floor still moves the clock at 1e7 s, far past any first maximum.
                assert!(1.0e7 + floor > 1.0e7);
            } else if card == "cell_2170_m80" {
                assert_eq!(floor.to_bits(), cell_2170_m80::STEP_S.to_bits());
                assert_eq!(ceiling.to_bits(), cell_2170_m80::STEP_S.to_bits());
            } else if card == "cell_2170_model3" {
                assert_eq!(floor.to_bits(), cell_2170_model3::FLOOR_DT_S.to_bits());
                assert_eq!(ceiling.to_bits(), cell_2170_model3::GATE_DT_S.to_bits());
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
            } else if card == "tokamak_jet_dte1" {
                // The tokamak's floor and ceiling are locked constants of the card.
                assert_eq!(floor.to_bits(), tokamak_jet_dte1::FLOOR_DT_S.to_bits());
                assert_eq!(ceiling.to_bits(), tokamak_jet_dte1::GATE_DT_S.to_bits());
            } else {
                assert_eq!(floor.to_bits(), (ceiling / 1_048_576.0).to_bits(), "{card}");
            }
            let body = describe_one(item);
            let dt = &body["dt_s"];
            assert_eq!(dt["floor_s"].as_f64().unwrap().to_bits(), floor.to_bits());
            assert_eq!(dt["ceiling_s"].as_f64().unwrap().to_bits(), ceiling.to_bits());
            assert!(
                dt["floor_from"].as_str().unwrap().starts_with(if card == "cell_2170_m80"
                    || card == "creep_cmsx6"
                {
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
                } else if card == "tokamak_jet_dte1" {
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
    const REACTOR_SOURCE: &str = include_str!("../../genesis_kernel/src/reactor_spert3.rs");
    const REACTOR_TRIGA_SOURCE: &str =
        include_str!("../../genesis_kernel/src/reactor_triga_jsi.rs");
    const REACTOR_FUEL_SOURCE: &str =
        include_str!("../../genesis_kernel/src/reactor_spert3_fuel.rs");
    const GRID_GB_SOURCE: &str = include_str!("../../genesis_kernel/src/grid_gb.rs");
    const CELL_2170_SOURCE: &str = include_str!("../../genesis_kernel/src/cell_2170_m80.rs");
    const CANTILEVER_SOURCE: &str = include_str!("../../genesis_kernel/src/cantilever_eth.rs");
    const CELL_2170_MODEL3_SOURCE: &str =
        include_str!("../../genesis_kernel/src/cell_2170_model3.rs");
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
        assert!(card.specimen.contains("fitted to this link's own readings of June and July 2015"));
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
            "doi 10.5281/zenodo.7107689, CC BY-SA 4.0",
            "Recommendation ITU-R P.838-3 at this frequency and polarisation, with nothing fitted",
            "fitted to this link's own readings in the 18 rain events of June and July",
            "Every reading of August was held back",
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
        assert!(limits.starts_with("A forecast is not a result."), "{limits}");
        for words in [
            "0.667 dB, against 1.028 dB for P.838 alone and 1.589 dB for no attenuation",
            "interpolation in rain rate, later in time",
            "It does not test another link, frequency, polarisation or season, or snow",
            "one gauge at one end of the path",
            "7.98 dB above free space between two ideal 0.3 m apertures",
            "one receiver step, 0.3 dB, of error in it puts the summed attenuation outside its band, at 1.69",
            "a single dry level for the whole month, 52.30 dB, gives 0.644",
            "must not take the card's peak as the measured peak",
            "the card's five-minute peak was the lower in seven of the nine events, by up to 2.2 dB",
            "its one-minute peak was the lower in six, by up to 2.8 dB, and the higher in three, by up to 3.3 dB",
            "no energy was compared with a measurement",
            "a rate above 84 mm/h, a rate above 0 and below 6 mm/h, rain changed inside a gauge minute, a step longer than 60 s",
            "none of those was compared with a measurement",
        ] {
            assert!(limits.contains(words), "{words}: {limits}");
        }
        assert_eq!(part("result"), LINK_RESULT);
        assert_ne!(part("result"), NOT_COMPARED);
        let result = part("result");
        for words in [
            "all three lines were met",
            "0.581 dB, against 0.90",
            "0.585 of the error of P.838 alone, 0.994 dB, against 0.80",
            "1.069 of the measured sum",
            "nine were scored and none was refused",
            "In two of the nine events P.838 alone was closer than the card",
            "finds 21 gauge events in August: eleven are under 1.0 mm",
            "one of 15.8 mm that starts 2015-08-31T17:37Z was not scored",
            "0.622 dB, 0.528 and 0.974",
            "The card's five-minute peak is below the measured five-minute peak in seven of the nine events, by up to 2.2 dB",
            "its peak is below the measured peak in six of the nine events, by up to 2.8 dB, and above it in three, by up to 3.3 dB",
            "lowers the error from 0.580 to 0.516 dB",
            "sublink 588",
            "0.794 dB, no better than P.838 alone, with a summed attenuation of 2.30",
            "evidence/verification/second_party/V-28/",
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
            "all three lines were met",
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
    const TOKAMAK_SOURCE: &str = include_str!("../../genesis_kernel/src/tokamak_jet_dte1.rs");

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

    /// The Great Britain card lists every variant of its `StepReject`, in the
    /// kernel's order, with the kernel's sentence word for word, and
    /// publishes the kernel's bounds exactly.
    #[test]
    fn the_great_britain_card_carries_the_kernel_sentences_and_bounds() {
        let documented = documented_rejects(GRID_GB_SOURCE);
        let card = find("energy", "grid_gb").expect("the area is served");
        assert_eq!(documented.len(), 12);
        assert_eq!(card.rejects.len(), documented.len());
        for ((name, sentence), listed) in documented.iter().zip(card.rejects) {
            assert_eq!(name, listed);
            assert_eq!(reject_meaning("grid_gb", name), Some(sentence.as_str()), "{name}");
        }
        assert_eq!(card.advisory.len(), 3);
        let body = describe_one(card);
        let field = &body["phase_fields"][0];
        assert_eq!(field["name"], "frequency_hz");
        assert_eq!(field["minimum"].as_f64().unwrap().to_bits(), grid_gb::FREQUENCY_MIN_HZ.to_bits());
        assert_eq!(field["maximum"].as_f64().unwrap().to_bits(), grid_gb::FREQUENCY_MAX_HZ.to_bits());
        for (index, name) in [(1, "inertia_mw_s"), (2, "demand_mw")] {
            let field = &body["phase_fields"][index];
            assert_eq!(field["name"], name);
            assert_eq!(field["minimum"].as_f64().unwrap(), 0.0);
            assert!(field["maximum"].is_null(), "{field}");
        }
        let load = &body["load_fields"][0];
        assert_eq!(load["name"], "loss_mw");
        assert!(load["minimum"].is_null() && load["maximum"].is_null(), "{load}");
        let ceiling = grid_gb::GridGb::interconnector_loss().gate_dt_s().unwrap();
        assert_eq!(
            body["example_compute_step"]["parameters"]["dt_s"].as_f64().unwrap().to_bits(),
            ceiling.to_bits()
        );
        assert!(2_880.0 * ceiling < grid_gb::TIME_MAX_S);
    }

    /// The reactor lists every variant of its `StepReject`, in the kernel's
    /// order, with the kernel's sentence word for word.
    #[test]
    fn the_reactor_refusals_carry_the_sentences_from_the_card() {
        let documented = documented_rejects(REACTOR_SOURCE);
        let card = find("energy", "reactor_spert3").expect("the reactor is served");
        assert_eq!(documented.len(), 13);
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

    /// The reactor publishes the kernel's bounds exactly, liquid water for the
    /// coolant and no bound for the reactivity, whose first step the door holds
    /// above zero, and its examples are one step at the ceiling and a run to the
    /// first maximum.
    #[test]
    fn the_reactor_publishes_the_kernel_bounds_and_its_examples_start_cold() {
        let card = find("energy", "reactor_spert3").expect("the reactor is served");
        let body = describe_one(card);
        let phase = &body["phase_fields"][0];
        assert_eq!(phase["name"], "coolant_temperature_k");
        assert_eq!(phase["unit"], "K");
        assert_eq!(
            phase["minimum"].as_f64().unwrap().to_bits(),
            reactor_spert3::WATER_FREEZING_K.to_bits()
        );
        assert_eq!(
            phase["maximum"].as_f64().unwrap().to_bits(),
            reactor_spert3::WATER_CRITICAL_K.to_bits()
        );
        let words = phase["bound"].as_str().unwrap();
        assert!(words.contains("CoolantTemperatureNotLiquidWater"), "{words}");
        assert!(words.contains("Tested at a primary coolant of 56 to 92 degF"), "{words}");
        let load = &body["load_fields"][0];
        assert_eq!(load["name"], "reactivity_inserted_dollar");
        assert!(load["minimum"].is_null() && load["maximum"].is_null(), "{load}");
        let words = load["bound"].as_str().unwrap();
        assert!(words.contains("ReactivityNotPositive"), "{words}");
        assert!(words.contains("Compared on reactivity 0.769 to 1.210 dollars"), "{words}");
        assert!(words.contains("[0.7686211839115998, 1.2102315121058187]"), "{words}");
        let ceiling = reactor_spert3::ReactorSpert3::e_core_cold_startup()
            .gate_dt_s()
            .unwrap();
        assert_eq!(
            body["example_compute_step"]["parameters"]["dt_s"].as_f64().unwrap().to_bits(),
            ceiling.to_bits()
        );
        let rho = body["example_run_trajectory"]["segments"][0]["load"]["reactivity_inserted_dollar"]
            .as_f64()
            .unwrap();
        assert!((0.7686211839115998..=1.2102315121058187).contains(&rho));
    }

    /// The fuel-temperature reactor lists every variant of its `StepReject`,
    /// in the kernel's order, with the kernel's sentence word for word.
    #[test]
    fn the_fuel_reactor_refusals_carry_the_sentences_from_the_card() {
        let documented = documented_rejects(REACTOR_FUEL_SOURCE);
        let card = find("energy", "reactor_spert3_fuel").expect("the fuel reactor is served");
        assert_eq!(documented.len(), 15);
        assert_eq!(card.rejects.len(), documented.len());
        for ((name, sentence), listed) in documented.iter().zip(card.rejects) {
            assert_eq!(name, listed);
            assert_eq!(
                reject_meaning("reactor_spert3_fuel", name),
                Some(sentence.as_str()),
                "{name}"
            );
        }
        // The bands are named by the temperatures they take: 260 degF itself is refused.
        assert_eq!(REACTOR_FUEL_BAND_DEGF, ["56 to 92", "251 to 259", "498 to 501"]);
        let reactor = reactor_spert3_fuel::ReactorSpert3Fuel::e_core_low_initial_power();
        assert!(reactor.band_of((260.0 - 32.0) / 1.8 + 273.15).is_none());
        assert!(reactor.band_of((260.0 + 459.67) / 1.8).is_none());
        // The two fractions the state ceiling's sentence carries are the card's own.
        let sentence = reject_meaning("reactor_spert3_fuel", "DtAboveStateCeiling").unwrap();
        assert!(sentence.contains(&format!("{} of the time constant", reactor_spert3_fuel::GATE_PHASE_STEP)), "{sentence}");
        assert!(sentence.contains(&format!("or {} of it", reactor_spert3_fuel::GROWTH_PHASE_STEP)), "{sentence}");
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
    /// frame as the card's `publish` writes it. Its one-line summary carries
    /// the standing: the fitted constants, that the form was added after the
    /// forms fixed beforehand had failed, and what the fuel temperature was
    /// compared with. Its record names every fitted constant with its value,
    /// what is sourced, the licences, and both scorings of the first card.
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
        assert!(jet.ends_with("The card's fusion power is superseded by the solver tokamak_dt_beam_target, which computes the D-T rate of one time slice from the plasma's profiles and its beams."));
        // The heat share of the accumulator is carried into each start, so its limits state no known limit.
        assert!(!ACCUMULATOR_SBO450_LIMITS.contains("Known limit"));
        assert!(ACCUMULATOR_SBO450_LIMITS.ends_with("The amendment and its run after the fact are the first party's and have not been scored by a second party."));
        let cell = find("energy", "cell_18650pf").unwrap();
        assert!(cell.specimen.ends_with("A step at any other current lands with FLAG_CURRENT_OUTSIDE_PASSED_SPAN."));
    }

    #[test]
    fn the_triga_reactor_publishes_the_kernel_bounds_tables_flags_frame_and_record() {
        let card = find("energy", "reactor_triga_jsi").expect("the TRIGA reactor is served");
        assert_eq!(all()[32].card, "reactor_triga_jsi");
        assert_eq!(all()[33].card, "reactor_spert3_ccore");
        assert_eq!(all().last().map(|last| last.card), Some("tokamak_vde_mast"));
        assert_eq!(all().len(), 37);
        for words in [
            "Ten constants are fitted to 36 pulses of this reactor",
            "added after the twelve fixed beforehand had failed",
            "one thermocouple of one element",
            "2.0 to 3.0 printed dollars",
            "below 2.0 dollars it read 29, 21 and 9 percent low in peak power",
            "up to the rod's whole worth of 4.16 printed dollars",
            "cores 215, 217 and 231",
            "the comparison with the 20 rows held back failed, four of six lines missed",
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
        assert!(start_words.contains("the dependence on the starting temperature was not tested apart"), "{start_words}");
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
        for words in ["No compared scram came while the transient rod was moving; the 8 dollars of the scram is an assumption", "On the compared pulses the rods dropped 1.51 to 7.45 s after the peak", "an earlier scram was not compared", "The longest compared hold was 7.45 s after the peak; a 3.0 dollar pulse with the rods held reaches 600 K after 5.2 to 6.4 s on cores 215 and 231"] {
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
            "A run that never scrams is no longer refused 7.45 s after its peak",
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
            "The site states no licence",
            "CC BY-NC-ND",
            "one thermocouple at the centre of one element, 6753 at position B1, with the instrument's response in between",
            "four of eight lines met and four missed",
            "evidence/verification/second_party/V-37",
            "9a0720f1822161c3b53cbc351d862ec4c077da51521ee1a96d7ceb85d8275ddc",
            "by the rule fixed beforehand none held in any bin of reactivity",
            "the comparison failed: two of the six lines were met and four missed",
            "are also met by the nearest opened pulse taken unchanged, so they are not evidence for the model",
            "the first card met the thermocouple maximum",
            "The card cannot see the day",
            "evidence/verification/second_party/V-43",
            "all three were missed",
            "one of six with none or with 0.015 dollars, two with 0.03 and six with 0.06",
            "they prefer 1.585 with a heat flow of 2930 W/K",
            "158 to 170 per second",
            "fifteen fitted in all",
            "nothing unfitted lies above 2.6 dollars",
            "0.99 correlated with peak power",
            "added after that result was seen",
            "No constant of the card is therefore the reactor's own value of that quantity",
            "Not repeated on a second machine or by a second organisation",
        ] {
            assert!(all_of_it.contains(words), "the record lacks: {words}");
        }
    }

    /// The fuel-temperature reactor publishes the widest of the kernel's three
    /// bands for each field and names each band's own span in words, sets both
    /// flags, packs the frame as the card's `publish` does, and its examples
    /// are one step at the ceiling and a run that starts in the cold band.
    #[test]
    fn the_fuel_reactor_publishes_the_kernel_bands_flags_and_frame() {
        let card = find("energy", "reactor_spert3_fuel").expect("the fuel reactor is served");
        assert_eq!(card.domain, "energy");
        assert_eq!(all()[19].card, "reactor_spert3_fuel");
        assert_eq!(all().len(), 37);
        let reactor = reactor_spert3_fuel::ReactorSpert3Fuel::e_core_low_initial_power();
        let body = describe_one(card);
        let bits = |value: &Value| value.as_f64().unwrap().to_bits();
        let phase = &body["phase_fields"];
        assert_eq!(phase.as_array().unwrap().len(), 1);
        assert_eq!(phase[0]["name"], "inlet_temperature_k");
        assert_eq!(phase[0]["unit"], "K");
        assert_eq!(bits(&phase[0]["minimum"]), reactor_spert3_fuel::BAND_INLET_MIN_K[0].to_bits());
        assert_eq!(bits(&phase[0]["maximum"]), reactor_spert3_fuel::BAND_INLET_MAX_K[2].to_bits());
        let load = &body["load_fields"];
        assert_eq!(load.as_array().unwrap().len(), 2);
        assert_eq!(load[0]["name"], "reactivity_inserted_dollar");
        assert_eq!(load[0]["unit"], "dollar");
        assert_eq!(load[1]["name"], "coolant_flow_m_per_s");
        assert_eq!(load[1]["unit"], "m/s");
        // The reactivity above zero and the flow at or above zero: the physical limits, in every
        // band, with no maximum.
        assert_eq!(bits(&load[0]["minimum"]), 0.0_f64.to_bits());
        assert_eq!(load[0]["minimum_exclusive"], true);
        assert!(load[0]["maximum"].is_null());
        assert_eq!(bits(&load[1]["minimum"]), 0.0_f64.to_bits());
        assert!(load[1]["maximum"].is_null());
        // Each band's own numbers are in the words of the field, from the card's constants.
        for band in 0..3 {
            let name = REACTOR_FUEL_BAND_DEGF[band];
            let words = phase[0]["bound"].as_str().unwrap();
            assert!(
                words.contains(&format!(
                    "{name} degF: [{}, {}] K",
                    reactor.band_inlet_min_k[band], reactor.band_inlet_max_k[band]
                )),
                "{words}"
            );
            let words = load[0]["bound"].as_str().unwrap();
            assert!(
                words.contains(&format!(
                    "{name} degF: [{}, {}] dollars",
                    reactor.band_reactivity_min_dollar[band], reactor.band_reactivity_max_dollar[band]
                )),
                "{words}"
            );
            let words = load[1]["bound"].as_str().unwrap();
            let span = if reactor.band_flow_min_m_per_s[band] == reactor.band_flow_max_m_per_s[band] {
                format!("{name} degF: exactly {} m/s", reactor.band_flow_min_m_per_s[band])
            } else {
                format!(
                    "{name} degF: [{}, {}] m/s",
                    reactor.band_flow_min_m_per_s[band], reactor.band_flow_max_m_per_s[band]
                )
            };
            // Each number in the words reads back to the card's own bits.
            for value in [
                reactor.band_inlet_min_k[band],
                reactor.band_reactivity_max_dollar[band],
                reactor.band_flow_min_m_per_s[band],
                reactor.band_flow_max_m_per_s[band],
            ] {
                assert_eq!(format!("{value}").parse::<f64>().unwrap().to_bits(), value.to_bits());
            }
            assert!(words.contains(&span), "{words}");
            // The band's name is the band's own ends in degrees Fahrenheit. The highest is
            // (F + 459.67) / 1.8, the conversion both E-core cards use; the lowest is the
            // (F - 32) / 1.8 + 273.15 at which the compared requests were sent, below the other,
            // so either conversion of each end is inside.
            let (low, high) = name.split_once(" to ").unwrap();
            let as_compared = |degf: &str| (degf.parse::<f64>().unwrap() - 32.0) / 1.8 + 273.15;
            let kelvin = |degf: &str| (degf.parse::<f64>().unwrap() + 459.67) / 1.8;
            assert_eq!(as_compared(low).to_bits(), reactor.band_inlet_min_k[band].to_bits());
            assert!(kelvin(low) >= reactor.band_inlet_min_k[band]);
            assert_eq!(kelvin(high).to_bits(), reactor.band_inlet_max_k[band].to_bits());
            assert!(as_compared(high) <= reactor.band_inlet_max_k[band]);
        }
        for name in ["InletTemperatureOutsideCard", "ReactivityNotPositive", "NegativeFlow", "DtAboveStateCeiling"] {
            assert!(body.to_string().contains(name), "{name}");
        }

        // Both flags, each with the boolean of the state that sets it.
        let flags = body["advisory_flags"].as_array().unwrap();
        assert_eq!(flags.len(), 2);
        assert_eq!(flags[0]["name"], "FLAG_PROMPT_SUPERCRITICAL");
        assert_eq!(flags[0]["mask"], reactor_spert3_fuel::FLAG_PROMPT_SUPERCRITICAL);
        assert_eq!(flags[0]["state"], "prompt_supercritical");
        assert_eq!(flags[1]["name"], "FLAG_ROD_MOVING");
        assert_eq!(flags[1]["mask"], reactor_spert3_fuel::FLAG_ROD_MOVING);
        assert_eq!(flags[1]["state"], "rod_moving");
        assert!(flags.iter().all(|flag| flag["seals"] == true));

        // The frame, as the card's `publish` writes it.
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
            ("t", "t_s", "s"),
            ("pos[0]", "power_mw", "MW"),
            ("pos[1]", "energy_mws", "MW*s"),
            ("pos[2]", "reactivity_net_dollar", "dollar"),
            ("vel[0]", "fuel_temperature_rise_k", "K"),
            ("vel[1]", "water_temperature_rise_k", "K"),
            ("vel[2]", "peak_power_mw", "MW"),
            ("force_torque", "power_rate_mw_per_s", "MW/s"),
            ("residual", "residual_conservation_error", "1"),
        ] {
            assert_eq!(slot(name)["state"], state, "{name}");
            assert_eq!(slot(name)["unit"], unit, "{name}");
        }
        for (line, state) in [
            ("frame.pos[0] = state.power_mw as f32;", "pos[0]"),
            ("frame.pos[1] = state.energy_mws as f32;", "pos[1]"),
            ("frame.pos[2] = state.reactivity_net_dollar as f32;", "pos[2]"),
            ("frame.vel[0] = state.fuel_temperature_rise_k as f32;", "vel[0]"),
            ("frame.vel[1] = state.water_temperature_rise_k as f32;", "vel[1]"),
            ("frame.vel[2] = state.peak_power_mw as f32;", "vel[2]"),
            ("frame.force_torque = state.power_rate_mw_per_s as f32;", "force_torque"),
        ] {
            assert!(REACTOR_FUEL_SOURCE.contains(line), "{state}: the card packs this slot otherwise");
        }

        // The clock and the examples.
        let ceiling = reactor.gate_dt_s().unwrap();
        let dt = &body["dt_s"];
        assert_eq!(bits(&dt["ceiling_s"]), ceiling.to_bits());
        assert!(dt["nyquist_s"].is_null());
        assert_eq!(bits(&body["example_compute_step"]["parameters"]["dt_s"]), ceiling.to_bits());
        let run = &body["example_run_trajectory"];
        let inlet_k = run["initial"]["inlet_temperature_k"].as_f64().unwrap();
        let band = reactor.band_of(inlet_k).expect("the example starts in a band");
        assert_eq!(band, 0);
        let example_load = &run["segments"][0]["load"];
        let rho = example_load["reactivity_inserted_dollar"].as_f64().unwrap();
        assert!(rho >= reactor.band_reactivity_min_dollar[band] && rho <= reactor.band_reactivity_max_dollar[band]);
        let flow = example_load["coolant_flow_m_per_s"].as_f64().unwrap();
        assert!(flow >= reactor.band_flow_min_m_per_s[band] && flow <= reactor.band_flow_max_m_per_s[band]);
        assert_eq!(run["segments"][0]["steps"], REACTOR_FUEL_EXAMPLE_STEPS);
        // The rule names the card's own starting power and the margins it no longer stops at.
        let rule = dt["rule"].as_str().unwrap();
        assert!(rule.contains(&format!("starts at {:e} MW in equilibrium", reactor.initial_power_mw)), "{rule}");
        assert!(rule.contains("The longest scored time of peak is 13.7 s; the largest energy to peak in Tables III and V is 15 MW s; the largest peak power is 410 MW."), "{rule}");
        assert!(rule.contains("with no limit of time, energy or power"), "{rule}");
    }

    /// The fuel-temperature reactor's validation record: the report, the
    /// three bands, the fit and the held-back tests, the protocol and its
    /// marks, the comparison that failed with every figure of every group,
    /// and the limits with the four fitted values.
    #[test]
    fn the_fuel_reactor_validation_record_states_the_comparison_that_failed() {
        let card = find("energy", "reactor_spert3_fuel").expect("the fuel reactor is served");
        let record = &describe_one(card)["validation"];
        let part = |name: &str| record[name].as_str().unwrap().to_string();
        let object = part("object");
        for words in [
            "SPERT III E-core",
            "56 to 92, 251 to 259 and 498 to 501 degrees Fahrenheit",
            "ramped in as K t^2",
            "the fuel temperature and the water temperature are state",
            "square-root Doppler law",
        ] {
            assert!(object.contains(words), "{words}: {object}");
        }
        let data = part("data");
        for words in [
            "IDO-17281",
            "OSTI 4792676",
            "four fitted constants were taken from 28 tests",
            "30 tests were held back",
            "13 tests at inlet temperatures of 251 to 259 degrees Fahrenheit",
        ] {
            assert!(data.contains(words), "{words}: {data}");
        }
        let pass_line = part("pass_line");
        for words in [
            "evidence/research/reactor_spert3/validation_protocol_v2.md",
            "413ace16d5cc3d622c0ef44c0ea2b9526ac7526dfeee4b43ae601d6018e51db4",
            "before this card was compared with any held-back test",
            "had been compared once with the first card of this reactor, on 2026-10-08",
            "at most 1.5",
            "at most 5 percent",
        ] {
            assert!(pass_line.contains(words), "{words}: {pass_line}");
        }
        let result = part("result");
        assert_eq!(result, REACTOR_SPERT3_FUEL_RESULT);
        assert_ne!(result, NOT_COMPARED);
        for words in [
            "Compared with the 30 held-back tests on 2026-10-09, and by the rule fixed beforehand the comparison failed in each of the three groups and for the 30 together",
            "of the 24 lines of the three groups 18 were met and 6 were missed",
            "of the 8 lines for the 30 together 5 were met",
            // cold: peak power, energy, time
            "(0.514, 0.937, +0.204)",
            "(0.540, 1.087, +0.023)",
            "root mean square 5.22 percent against 5 and worst 12.24 percent against 10",
            "at the floor of the measurement",
            "about 3 to 6 percent",
            // 251 to 259 degF
            "The group at 251 to 259 degrees Fahrenheit, 13 tests, the report's series at about 260 degrees Fahrenheit",
            "the one group with nothing fitted to it",
            "(0.643, 1.051, and a mean of +0.479 against a line of 0.5 in size, met by 0.02)",
            "(0.846, 1.334, -0.152)",
            "root mean square 8.46 percent and worst 14.88 percent, twelve of the thirteen peaks early",
            // 498 to 501 degF
            "The group at 498 to 501 degrees Fahrenheit, 7 tests",
            "(0.796, 1.650, and a mean of -0.490 against 0.5, met by 0.01)",
            "(4.49 and 7.45 percent)",
            "root mean square 1.336, worst 3.212 against 3, and mean -0.890 against 0.5 in size",
            "the card's energy is low on all seven tests",
            // the 30 together
            "(0.644, 1.650, +0.161)",
            "(0.908, mean -0.266, worst 3.212)",
            "(6.69 and 14.88 percent)",
            "The scored step was 0.43 ms, the card's ceiling, where the protocol said 0.5 ms",
            "evidence/verification/second_party/V-25/",
            "the same figures and the same verdicts",
        ] {
            assert!(result.contains(words), "{words}: {result}");
        }
        // The step the result names is the card's ceiling, and the protocol's step is refused.
        let gate = reactor_spert3_fuel::ReactorSpert3Fuel::e_core_low_initial_power()
            .gate_dt_s()
            .unwrap();
        assert!((gate - 0.43e-3).abs() < 1.0e-15 && gate < 0.5e-3);
        let limits = part("limits");
        assert!(limits.starts_with("A miss, and it stays on the record. Four constants are fitted on 28 tests"));
        // The four fitted values are the card's constants, digit for digit.
        for (words, value) in [
            ("the power at the start, 11.6113 W", reactor_spert3_fuel::INITIAL_POWER_MW * 1.0e6),
            (
                "0.0371601 dollar per square root of a kelvin",
                reactor_spert3_fuel::DOPPLER_STRENGTH_DOLLAR_PER_SQRT_K,
            ),
            ("0.0260616 MW s per K", reactor_spert3_fuel::FUEL_HEAT_CAPACITY_MWS_PER_K),
            ("for all three bands, 0.28586", reactor_spert3_fuel::WATER_FEEDBACK_WEIGHT),
        ] {
            assert!(limits.contains(words), "{words}: {limits}");
            let digits: String = words
                .split(|c: char| !(c.is_ascii_digit() || c == '.'))
                .find(|part| part.contains('.'))
                .unwrap()
                .to_string();
            let written: f64 = digits.parse().unwrap();
            assert!((written - value).abs() <= 1.0e-12 * value.abs(), "{words}: {value}");
        }
        for words in [
            "a fitted effective value and not a measurement",
            "about 7 to 11 W, the report's 50 W is excluded under this model",
            "the report's design figures",
            "10 to 15 ms of fixed-time error remains, where the card is late on the fastest tests",
            "The fuel temperature is compared with no measurement, because the report measures none",
            "69 to 402 K at the peak on the held-back tests",
            "The comparison is not blind. This card's form was built after the first card of this reactor had been compared, on 2026-10-08, with the ten cold-startup tests held back here",
            "the whole of Tables V and VII as transcribed, their held-back rows included, had been printed",
            "are not unseen",
            "The conservation residual names a wrong method and is not a bound on accuracy",
            "reads the true error of the state low by about 11 to 15 times",
            "3.3e-7 and 3.8e-7 of the neutron population, is above the limit while the residual is 2.3e-8 and 2.6e-8",
            "a reactivity changed during a burst is refused by name",
            "Each step may carry its own flow rate; no tested burst changed its flow, and a burst whose flow changes was not compared",
            "Outside each band's compared reactivity and flow the card computes the same equations and nothing was compared",
        ] {
            assert!(limits.contains(words), "{words}: {limits}");
        }
        // The limit the record names is the card's, unchanged.
        assert_eq!(reactor_spert3_fuel::RESIDUAL_CONSERVATION_LIMIT, 3.0e-7);
        assert!(limits.contains("Its limit, 3e-7,"));
        // The one line of list_cards says the comparison failed.
        assert!(card.specimen.ends_with(
            "; four constants are fitted, and by the rule fixed beforehand the comparison with its 30 held-back tests failed."
        ));
        // The record quotes no proof of a run.
        assert!(!record.to_string().contains("seal"));
    }

    /// The 2170 cell lists every variant of its `StepReject`, in the kernel's
    /// order, with the kernel's sentence word for word.
    #[test]
    fn the_2170_cell_refusals_carry_the_sentences_from_the_card() {
        let documented = documented_rejects(CELL_2170_SOURCE);
        let card = find("energy", "cell_2170_m80").expect("the 2170 cell is served");
        assert_eq!(documented.len(), 13);
        assert_eq!(card.rejects.len(), documented.len());
        for ((name, sentence), listed) in documented.iter().zip(card.rejects) {
            assert_eq!(name, listed);
            assert_eq!(
                reject_meaning("cell_2170_m80", name),
                Some(sentence.as_str()),
                "{name}"
            );
        }
        assert_eq!(card.advisory.len(), 2);
    }

    /// The 2170 cell publishes the kernel's bounds exactly for every input,
    /// each block's own range of charge, a step of one second as floor and
    /// ceiling, and a validation record that states the comparison that failed.
    #[test]
    fn the_2170_cell_publishes_the_kernel_bounds_and_states_the_comparison_that_failed() {
        let card = find("energy", "cell_2170_m80").expect("the 2170 cell is served");
        let body = describe_one(card);
        let field = |list: &str, index: usize| body[list][index].clone();
        let bounds = |field: &Value| {
            (
                field["minimum"].as_f64().unwrap().to_bits(),
                field["maximum"].as_f64().unwrap().to_bits(),
                field["minimum_exclusive"].as_bool().unwrap(),
                field["maximum_exclusive"].as_bool().unwrap(),
            )
        };
        let block = field("phase_fields", 0);
        assert_eq!(block["name"], "block_chamber_c");
        assert_eq!(block["unit"], "degC");
        // Every block is served: the -10 and -20 degC blocks met no mark and
        // carry a flag.
        assert_eq!(bounds(&block), ((-20.0_f64).to_bits(), 40.0_f64.to_bits(), false, false));
        assert!(block["bound"].as_str().unwrap().contains("FLAG_BLOCK_MET_NO_MARK"));
        let charge = field("phase_fields", 1);
        assert_eq!(charge["name"], "charge_removed_ah");
        assert_eq!(charge["unit"], "Ah");
        assert_eq!(
            bounds(&charge),
            ((-0.02_f64).to_bits(), cell_2170_m80::CHARGE_DEEP_AH[0].to_bits(), false, false)
        );
        let words = charge["bound"].as_str().unwrap();
        for (chamber, deep) in [
            ("40", "4.67683635335689"),
            ("25", "4.4615"),
            ("10", "4.2059"),
            ("0", "4.1048"),
            ("-10", "3.9413"),
            ("-20", "3.7886"),
        ] {
            let chamber = token(chamber.parse::<f64>().unwrap());
            let deep = token(deep.parse::<f64>().unwrap());
            assert!(
                words.contains(&format!("{chamber} degC: [{}, {deep}] Ah", token(-0.02))),
                "{chamber}: {words}"
            );
        }
        // The current and the case temperature are held to where the tables end.
        let current = field("load_fields", 0);
        assert_eq!(current["name"], "current_a");
        assert_eq!(bounds(&current), ((-13.5_f64).to_bits(), 13.5_f64.to_bits(), false, false));
        let case = field("load_fields", 1);
        assert_eq!(case["name"], "case_temperature_c");
        assert_eq!(case["unit"], "degC");
        assert_eq!(bounds(&case), ((-21.898_f64).to_bits(), 45.535_f64.to_bits(), false, false));
        let bands = case["bound"].as_str().unwrap();
        for block in 0..cell_2170_m80::BLOCK_COUNT {
            let band = format!(
                "{} degC block: [{}, {}] degC",
                cell_2170_m80::BLOCK_CHAMBER_C[block],
                cell_2170_m80::BLOCK_CASE_MIN_C[block],
                cell_2170_m80::BLOCK_CASE_MAX_C[block]
            );
            assert!(bands.contains(&band), "{band}: {bands}");
        }
        // Each block's compared currents are published, from envelope.json of the 0.7.0 fix.
        let currents = current["bound"].as_str().unwrap();
        assert!(currents.contains("FLAG_OUTSIDE_COMPARED_TESTS"));
        for block in 0..cell_2170_m80::BLOCK_COUNT {
            let band = format!(
                "{} degC block: [{}, {}] A",
                cell_2170_m80::BLOCK_CHAMBER_C[block],
                -cell_2170_m80::BLOCK_CURRENT_DISCHARGE_MAX_A[block],
                cell_2170_m80::BLOCK_CURRENT_CHARGE_MAX_A[block]
            );
            assert!(currents.contains(&band), "{band}: {currents}");
        }
        assert!(currents.contains("40 degC block: [-9.809, 9.737] A"), "{currents}");
        assert!(currents.contains("0 degC block: [-11.375, 8.304] A"), "{currents}");
        // A start past an end is refused, however near: it is stated.
        assert!(charge["bound"].as_str().unwrap().contains("A start one 64-bit number past an end is refused too"));
        assert_eq!(body["phase_fields"].as_array().unwrap().len(), 2);
        assert_eq!(body["load_fields"].as_array().unwrap().len(), 2);

        let dt = &body["dt_s"];
        assert_eq!(dt["floor_s"].as_f64().unwrap().to_bits(), 1.0_f64.to_bits());
        assert_eq!(dt["ceiling_s"].as_f64().unwrap().to_bits(), 1.0_f64.to_bits());
        assert!(dt["nyquist_s"].is_null());
        assert!(dt["ceiling_from"].as_str().unwrap().contains("StepNotOneSecond"));
        assert!(
            dt["rule"]
                .as_str()
                .unwrap()
                .starts_with(&format!("dt_s is in [{}, {}] s.", token(1.0), token(1.0)))
        );

        let record = &body["validation"];
        assert!(record["result"].as_str().unwrap().starts_with(
            "Compared with the 60 held-back tests on 2026-10-08, and by the rule fixed beforehand the comparison failed: 27 of 60 runs met their marks"
        ));
        let data = record["data"].as_str().unwrap();
        assert!(data.contains("doi 10.5683/SP3/ZVTR4B, version 3.0"));
        assert!(data.contains("doi 10.1109/ITEC53557.2022.9813996"));
        assert!(record["pass_line"].as_str().unwrap().contains(
            "488fe0af60bbb68be465106ae5fe368b7780f331e8ea5c87e8ac523e6ab984fa"
        ));
        assert!(record["limits"].as_str().unwrap().starts_with("A miss, and it stays on the record."));
        assert!(!record.to_string().contains("seal"));
        // The record counts three scorings: the sealed run was scored twice,
        // and a third time from the protocol's text alone. Result and limits
        // say the same.
        let result = record["result"].as_str().unwrap();
        assert!(result.contains("A third scoring, written from the protocol's text alone by an agent that did not write the card"), "{result}");
        assert!(!result.contains("second scoring"), "{result}");
        let limits = record["limits"].as_str().unwrap();
        assert!(limits.contains("scored it twice"));
        assert!(limits.contains("a third scoring, from the protocol's text alone"));
        assert!(!limits.contains("not yet repeated by a second party"));

        assert_eq!(body["advisory_flags"].as_array().unwrap().len(), 2);
        let slot = |name: &str| {
            body["frame_slots"]
                .as_array()
                .unwrap()
                .iter()
                .find(|slot| slot["slot"] == name)
                .unwrap()
                .clone()
        };
        assert_eq!(slot("pos[0]")["state"], "terminal_v");
        assert_eq!(slot("pos[1]")["state"], "charge_removed_ah");
        assert_eq!(slot("pos[1]")["unit"], "Ah");
        assert_eq!(slot("vel[1]")["state"], "branch_v");
        assert_eq!(slot("vel[2]")["state"], "case_temperature_c");
        assert_eq!(slot("vel[2]")["unit"], "degC");
        assert_eq!(slot("force_torque")["state"], "lower_limit_margin_v");

        let step = &body["example_compute_step"]["parameters"];
        assert_eq!(step["dt_s"].as_f64().unwrap().to_bits(), 1.0_f64.to_bits());
        assert_eq!(step["block_chamber_c"].as_f64().unwrap(), 25.0);
        assert_eq!(step["current_a"].as_f64().unwrap(), -2.25);
        let run = &body["example_run_trajectory"];
        assert_eq!(run["segments"][0]["steps"], 600);
        assert_eq!(run["record_every"], 60);
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
        assert_eq!(card.specimen, FINGERPAD_P02_GLASS_SUMMARY);
        assert!(card.specimen.contains("eight constants fitted to this finger's own trials"));
        assert!(card.specimen.contains("and missed"));
        assert!(card.specimen.contains("held-back trials at 1 N, 10 N and 100 degrees a second"));
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
        assert!(card.specimen.contains("It answers at every force and rate above zero with those misses stated"));
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
        assert!(card.specimen.contains("fitted to this tyre's own measurements"));
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
        assert_eq!(card.specimen, ACCUMULATOR_SBO450_SPECIMEN);
        assert!(card.specimen.contains("fitted to this accumulator's own runs"));
        let field = |name: &str| card.fields.iter().find(|f| f.name == name).expect("field");
        let ends = |name: &str| (field(name).minimum, field(name).maximum);
        assert_eq!(ends("gas_mass_kg"), (Some(0.0), None));
        assert_eq!(ends("gas_pressure_pa"), (Some(0.0), Some(accumulator_sbo450::PRESSURE_RATED_PA)));
        assert_eq!(ends("gas_temperature_k"), (Some(accumulator_sbo450::CRITICAL_TEMPERATURE_K), None));
        assert_eq!(ends("wall_temperature_k"), (Some(0.0), None));
        assert_eq!(ends("piston_speed_m_s"), (None, None));
        assert!(field("gas_pressure_pa").bound.contains("PressureAboveRated"));
        assert!(field("gas_pressure_pa").bound.contains("held as 37 to 43 and 55 to 67 bar for the 20 bar charge and 57 to 64 and 75 to 86 bar for the 40 bar charge"));
        assert!(field("gas_pressure_pa").bound.contains("2.72 percent median error of the swing against the line of 2.0"));
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
        assert!(limits.contains("It is not a new test"));
        assert!(limits.contains("The ledger closes for any time constant and any vessel volume"));
        assert!(limits.contains("choked flow does not arise"));
    }

    /// The tokamak lists every variant of its `StepReject`, in the kernel's
    /// order, with the kernel's sentence word for word.
    #[test]
    fn the_tokamak_refusals_carry_the_sentences_from_the_card() {
        let documented = documented_rejects(TOKAMAK_SOURCE);
        let card = find("energy", "tokamak_jet_dte1").expect("the tokamak is served");
        assert_eq!(documented.len(), 21);
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

    /// The tokamak publishes the physical bounds of its fields: each above
    /// or from zero, the tritium fraction from 0 to 1, and no upper bound
    /// elsewhere, since the compared envelope is lifted. Each bound sentence
    /// states the compared span. Its validation record carries the hash of
    /// the pre-registration and names both held-back pulses, and its summary
    /// states the failure.
    #[test]
    fn the_tokamak_publishes_the_kernel_bounds_and_its_validation_record() {
        let card = find("energy", "tokamak_jet_dte1").expect("the tokamak is served");
        let tokamak = tokamak_jet_dte1::TokamakJetDte1::elmy_h_mode();
        let body = describe_one(card);
        let field = |name: &str| {
            body["phase_fields"]
                .as_array()
                .unwrap()
                .iter()
                .chain(body["load_fields"].as_array().unwrap())
                .find(|field| field["name"] == name)
                .unwrap_or_else(|| panic!("{name}"))
                .clone()
        };
        assert_eq!(body["phase_fields"].as_array().unwrap().len(), 3);
        assert_eq!(body["load_fields"].as_array().unwrap().len(), 6);
        for (name, exclusive, compared) in [
            ("thermal_energy_j", true, "0.462 to 9.57 MJ"),
            ("fast_ion_energy_j", false, "5.59 kJ to 1.62 MJ"),
            ("elapsed_time_s", false, "5.475 s"),
            ("heating_power_w", false, "0.124 to 24.3 MW"),
            ("ohmic_power_w", false, "0.173 to 2.44 MW"),
            ("line_density_per_m3", true, "1.65e19 to 7.85e19"),
            ("plasma_current_a", true, "1.65 to 3.84 MA"),
            ("toroidal_field_t", true, "1.80 to 4.04 T"),
            ("tritium_fraction", false, "0.451 to 0.504"),
        ] {
            let published = field(name);
            assert_eq!(published["minimum"].as_f64().unwrap().to_bits(), 0.0_f64.to_bits(), "{name}");
            assert_eq!(published["minimum_exclusive"], exclusive, "{name}");
            if name == "tritium_fraction" {
                assert_eq!(published["maximum"].as_f64().unwrap().to_bits(), 1.0_f64.to_bits(), "{name}");
                assert_eq!(published["maximum_exclusive"], false, "{name}");
            } else {
                assert!(published["maximum"].is_null(), "{name}");
            }
            let bound = published["bound"].as_str().unwrap();
            assert!(bound.contains(compared), "{name}: {bound}");
            assert!(bound.to_lowercase().contains("compared"), "{name}: {bound}");
        }
        assert!(field("tritium_fraction")["bound"].as_str().unwrap().contains("FuelMixOutsideCard"));
        assert!(field("tritium_fraction")["bound"].as_str().unwrap().contains("0.743 of the measured"));
        assert!(field("heating_power_w")["bound"].as_str().unwrap().contains("L-H power threshold"));
        assert!(field("elapsed_time_s")["bound"].as_str().unwrap().contains("It is required"));
        assert!(!field("elapsed_time_s")["bound"].as_str().unwrap().contains("end of the card"));
        for words in [
            "the comparison failed",
            "0.743 of the measured",
            "1.65 to 3.84 MA",
            "nine JET pulses of its regime",
            "5.475 s",
            "0 to 0.034 or 0.451 to 0.504",
        ] {
            assert!(card.specimen.contains(words), "the summary lacks: {words}");
        }
        let limits = validation(card.card, card.specimen)["limits"].as_str().unwrap().to_string();
        assert!(limits.contains("met its mark on one, 43134"));
        assert!(!limits.contains("17 to 65 percent"));
        assert!(!limits.contains("not yet repeated by a second party"));
        assert!(body["dt_s"]["ceiling_from"].as_str().unwrap().contains("46.5 ms"));
        assert!(body["dt_s"]["ceiling_from"].as_str().unwrap().contains("18.0 ms at 0.777 keV and 7.85e19"));
        assert!(body["dt_s"]["ceiling_from"].as_str().unwrap().contains("past the compared envelope the step ceiling follows the state"));
        // The pulse outside the card's regime is outside its envelope and is computed; the limits say how it did.
        assert!(limits.contains("The states and loads only pulse 42976 reached are outside the compared envelope and are computed"));
        assert!(limits.contains("declared outside the card's regime before it was run, missed all four marks"));
        assert!(!limits.contains("refused by name"));
        assert!(!limits.contains("envelope of the ten pulses"));
        // The shape factor the current's bound states is the card's own.
        let shape = tokamak.safety_factor(1.0e6, 1.0)
            / (5.0 * 0.9386 * 0.9386 / 2.899);
        assert!((shape - 2.428).abs() < 5.0e-4, "{shape}");
        let dt = &body["dt_s"];
        assert_eq!(dt["ceiling_s"].as_f64().unwrap().to_bits(), tokamak_jet_dte1::GATE_DT_S.to_bits());
        assert_eq!(dt["floor_s"].as_f64().unwrap().to_bits(), tokamak_jet_dte1::FLOOR_DT_S.to_bits());
        assert!(dt["nyquist_s"].is_null());
        let flags = body["advisory_flags"].as_array().unwrap();
        assert_eq!(flags.len(), 1);
        assert_eq!(flags[0]["name"], "FLAG_NEAR_DENSITY_LIMIT");
        assert_eq!(flags[0]["state"], "near_density_limit");
        let record = &body["validation"];
        let part = |name: &str| record[name].as_str().unwrap().to_string();
        assert!(part("data").contains("42982") && part("data").contains("42976"));
        assert!(part("data").contains("TRANSP reconstruction"));
        assert!(part("pass_line").contains("7badf2b1267219d0e4da2edaad7d50097f9f98ceee1e0ab5fb9f8fd391ba94bb"));
        assert!(part("pass_line").contains("14.5 percent"));
        assert!(part("limits").starts_with("A forecast is not a result."));
        assert_eq!(part("result"), TOKAMAK_RESULT);
        assert!(part("result").starts_with("Compared with the two held-back pulses on 2026-10-09, and by the rule fixed beforehand the comparison failed"));
        for words in ["7.9 percent", "22.5 percent", "23.4 percent", "a ratio of 0.743", "19.2 percent", "34.7 percent"] {
            assert!(part("result").contains(words), "{words}");
        }
        assert_ne!(part("result"), NOT_COMPARED);
        assert!(!record.to_string().contains("seal"));
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
            "The envelope range compared with the held-back ring-down, in numbers:",
            "from 15.786 mm at its start, the largest it held, through 14.404 mm, at the first turning point after the start crossing, to 0.0587 mm",
            "There the amplitude error was 7.03 percent against a line of 5 percent, missed",
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
            "Lai, Alzugaray, Chli and Chatzi, Zenodo, doi 10.5281/zenodo.22723484, CC BY 4.0",
            "six constants were fitted to one recording, test1a",
            "test1b, was held back",
            "first 12 s were opened for a forecast made outside the engine",
            "next 12 s were held back from the fitting and the forecast and compared with the card once",
            "The comparison is not blind: on 2026-10-06, before the split was written, the coarse envelope of every recording",
            "was printed by mistake; no fit was made to it",
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
        assert!(limits.starts_with("A miss, and it stays on the record."), "{limits}");
        for words in [
            "2.22 percent in amplitude, 0.0047 Hz in frequency and 0.024 of a period in timing",
            "a smaller structure of five constants",
            "the larger one of six, chosen after the opened scores were seen",
            "One strip, one clamp, one afternoon.",
            "confirmed by a second reading to 3 percent",
            "every compared quantity is a ratio or a frequency",
            "no mass, so its energies are per kilogram",
            "a fitted power law",
            "refused after 107 steps of 0.25 ms",
            "The comparison spanned envelopes from 15.79 mm down to 0.058 mm",
            "the card answers with the frame flagged FLAG_ENVELOPE_ABOVE_COMPARED, where engine 0.11 refused",
            "Below 0.058 mm the card answers on to rest with the frame flagged",
        ] {
            assert!(limits.contains(words), "{words}: {limits}");
        }
        // The record states the outcome of the comparison.
        assert_eq!(part("result"), CANTILEVER_RESULT);
        assert_ne!(part("result"), NOT_COMPARED);
        assert!(!record.to_string().contains("seal"));
    }

    /// The physical 2170 cell lists every variant of its `StepReject`, in the
    /// kernel's order, with the kernel's sentence word for word, and its one
    /// flag. One sentence is the served card's and not the kernel's: the
    /// kernel's start can carry a span of case temperature from a continued
    /// run, and the served card takes no continued start, so the meaning of
    /// the span's refusal leaves "carried across a continued start" out.
    #[test]
    fn the_physical_2170_cell_refusals_carry_the_sentences_from_the_card() {
        let documented = documented_rejects(CELL_2170_MODEL3_SOURCE);
        let card = find("energy", "cell_2170_model3").expect("the physical 2170 cell is served");
        assert_eq!(documented.len(), 14);
        assert_eq!(card.rejects.len(), documented.len());
        assert!(crate::chain::not_continued("cell_2170_model3").is_some());
        for ((name, sentence), listed) in documented.iter().zip(card.rejects) {
            assert_eq!(name, listed);
            assert_eq!(
                reject_meaning("cell_2170_model3", name),
                Some(sentence.as_str()),
                "{name}"
            );
        }
        assert_eq!(card.advisory.len(), 2);
        assert_eq!(card.advisory[0].name, "FLAG_SUSTAINED_ABOVE_1C");
        assert_eq!(card.advisory[0].mask, cell_2170_model3::FLAG_SUSTAINED_ABOVE_1C);
        assert!(card.advisory[0].seals);
        assert_eq!(card.advisory[1].name, "FLAG_OUTSIDE_COMPARED_RUNS");
        assert!(card.advisory[1].seals);
        assert!(CELL_2170_MODEL3_SOURCE.contains("pub const FLAG_SUSTAINED_ABOVE_1C: u64 = 1 << 0;"));
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
        assert!(card.specimen.contains("fitted in the laboratory on Earth and not to Ryugu"));
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

    /// The physical 2170 cell publishes the kernel's bounds exactly for every
    /// input, the cells it holds with their locked numbers, a floor and a
    /// ceiling of 1 s, the pairing of capacity with case temperature and the
    /// limits on its change and span, its frame, and a validation record that
    /// says what it was developed on, what was compared, and its second
    /// scoring.
    #[test]
    fn the_physical_2170_cell_publishes_the_kernel_bounds_and_says_what_it_was_developed_on() {
        let card = find("energy", "cell_2170_model3").expect("the physical 2170 cell is served");
        let body = describe_one(card);
        let field = |list: &str, index: usize| body[list][index].clone();
        let bounds = |field: &Value| {
            (
                field["minimum"].as_f64().map_or(0, f64::to_bits),
                field["maximum"].as_f64().map_or(0, f64::to_bits),
                field["minimum_exclusive"].as_bool().unwrap(),
                field["maximum_exclusive"].as_bool().unwrap(),
            )
        };
        let number = field("phase_fields", 0);
        assert_eq!(number["name"], "cell_number");
        assert_eq!(bounds(&number), (80.0_f64.to_bits(), 1000.0_f64.to_bits(), false, false));
        let words = number["bound"].as_str().unwrap();
        assert!(words.starts_with("exactly one of 80 (cell m80), 1000 (cell m1000) and 448 (cell m448-N):"), "{words}");
        assert!(
            words.contains("cell m80: capacity at its first capacity test 4.67683635335689 Ah, charge-return ratio 0.9977038043683101, resistance scale 1; cell m1000: capacity at its first capacity test 4.674654617421927 Ah, charge-return ratio 0.9972171037869162, resistance scale 0.9119920763410333; cell m448-N: capacity at its first capacity test 4.6696220353547755 Ah, charge-return ratio 0.9976972129208563, resistance scale 0.8243415837166522."),
            "{words}"
        );
        assert_eq!(cell_2170_model3::CELL_COUNT, 3);
        let third = cell_2170_model3::CELLS[2];
        assert_eq!((third.name, third.cell_number), ("m448-N", 448.0));
        assert_eq!("4.6696220353547755".parse::<f64>().unwrap().to_bits(), third.reference_capacity_ah.to_bits());
        assert_eq!("0.9976972129208563".parse::<f64>().unwrap().to_bits(), third.charge_return_ratio.to_bits());
        assert_eq!("0.8243415837166522".parse::<f64>().unwrap().to_bits(), third.resistance_scale.to_bits());
        assert_eq!("0.9977038043683101".parse::<f64>().unwrap().to_bits(), cell_2170_model3::CELLS[0].charge_return_ratio.to_bits());
        assert_eq!("0.9972171037869162".parse::<f64>().unwrap().to_bits(), cell_2170_model3::CELLS[1].charge_return_ratio.to_bits());
        assert_eq!("4.67683635335689".parse::<f64>().unwrap().to_bits(), cell_2170_model3::CELLS[0].reference_capacity_ah.to_bits());
        assert_eq!("4.674654617421927".parse::<f64>().unwrap().to_bits(), cell_2170_model3::CELLS[1].reference_capacity_ah.to_bits());
        assert_eq!("0.9119920763410333".parse::<f64>().unwrap().to_bits(), cell_2170_model3::CELLS[1].resistance_scale.to_bits());
        assert!(words.contains("CellNotRecognized"));
        let charge = field("phase_fields", 1);
        assert_eq!(charge["name"], "state_of_charge");
        assert_eq!(charge["unit"], "1");
        assert_eq!(bounds(&charge), ((-0.05_f64).to_bits(), 1.05_f64.to_bits(), false, false));
        let capacity = field("phase_fields", 2);
        assert_eq!(capacity["name"], "capacity_ah");
        assert_eq!(capacity["unit"], "Ah");
        assert_eq!(bounds(&capacity), (0.0_f64.to_bits(), 0, true, false));
        let words = capacity["bound"].as_str().unwrap();
        assert!(words.contains("capacity_ah = ratio x (ampere-hours put in by the full recharge) / (1 - state of charge before that recharge)"));
        assert!(words.contains("it is listed for each cell under cell_number"));
        assert!(words.contains("below 0.08"));
        let current = field("load_fields", 0);
        assert_eq!(current["name"], "current_a");
        assert_eq!(bounds(&current), (0, 0, false, false));
        assert!(current["bound"].as_str().unwrap().contains("Compared from -13.984 to 13.524 A"));
        assert_eq!(13.524_f64.to_bits(), (13.5_f64 + cell_2170_model3::TESTER_CURRENT_ACCURACY_A).to_bits());
        assert!(current["bound"].as_str().unwrap().contains("A positive current discharges the cell"));
        let case = field("load_fields", 1);
        assert_eq!(case["name"], "case_temperature_c");
        assert_eq!(case["unit"], "degC");
        assert_eq!(bounds(&case), ((-273.15_f64).to_bits(), 0, true, false));
        assert!(case["bound"].as_str().unwrap().contains("Compared from -12.9 to 45.9 degC"));
        assert_eq!(body["phase_fields"].as_array().unwrap().len(), 5);
        assert_eq!(body["load_fields"].as_array().unwrap().len(), 2);
        for (index, name) in [(3, "case_temperature_least_c"), (4, "case_temperature_most_c")] {
            let carried = field("phase_fields", index);
            assert_eq!(carried["name"], name);
            assert_eq!(carried["unit"], "degC");
            assert_eq!(bounds(&carried), ((-273.15_f64).to_bits(), 0, true, false));
            assert!(carried["bound"].as_str().unwrap().contains("A start without it is refused"));
        }

        let dt = &body["dt_s"];
        assert_eq!(dt["floor_s"].as_f64().unwrap().to_bits(), 1.0_f64.to_bits());
        assert_eq!(dt["ceiling_s"].as_f64().unwrap().to_bits(), 1.0_f64.to_bits());
        assert!(dt["nyquist_s"].is_null());
        let rule = dt["rule"].as_str().unwrap();
        assert!(rule.starts_with(&format!("dt_s is in [{}, {}] s.", token(1.0), token(1.0))));
        assert!(dt["floor_from"].as_str().unwrap().contains("49 mV in the second of the reversal"));
        assert!(dt["floor_from"].as_str().unwrap().contains("For a held 4.5 A discharge at 25 degC, the one load compared, a step of 0.1 s gave the one-second answer to within 0.3 mV"));
        assert!(!dt["floor_from"].as_str().unwrap().contains("Under a held discharge"));
        assert!(!rule.contains("up to 9 mV") && !dt["ceiling_from"].as_str().unwrap().contains("9 mV"));
        let capacity_words = capacity["bound"].as_str().unwrap();
        for words in [
            "4.16 to 4.26 Ah from -12.9 to 12.6 degC",
            "4.20 to 4.30 Ah from 0.0 to 20.4 degC",
            "4.17 to 4.30 Ah from 9.8 to 26.8 degC",
            "4.35 to 4.53 Ah from 23.3 to 31.4 degC",
            "4.56 to 4.70 Ah from 28.1 to 45.9 degC",
            "FLAG_OUTSIDE_COMPARED_RUNS",
        ] {
            assert!(capacity_words.contains(words), "{words}");
        }
        let case_words = case["bound"].as_str().unwrap();
        assert!(case_words.contains("5.4 K") && case_words.contains("19.2 K") && case_words.contains("FLAG_OUTSIDE_COMPARED_RUNS"));
        assert!(charge["bound"].as_str().unwrap().contains("STATE_OF_CHARGE_ALLOWANCE_PER_STEP"));
        for name in [
            "FLAG_OUTSIDE_COMPARED_RUNS",
            "UndervoltageLimit",
            "OvervoltageLimit",
            "StateOfChargeOutsideCard",
            "SurfaceOutsideCurve",
            "FLAG_SUSTAINED_ABOVE_1C",
            "NonPositiveDt",
        ] {
            assert!(rule.contains(name), "{name}");
        }

        let record = &body["validation"];
        let result = record["result"].as_str().unwrap();
        assert_eq!(result, CELL_2170_MODEL3_RESULT);
        assert!(result.starts_with("Run once, on 2026-10-09, on runs of two further cells of the same type that no model had seen"));
        for words in [
            "One resistance number was fitted to each cell from its characterisation tests and nothing else was changed.",
            "Third cell, m448-N: 51 of 60 runs were inside the marks",
            "all five conditions written beforehand were met",
            "Second cell, m1000: 31 of 45 runs were inside the marks, below the 36 to 41 expected, and three of the five conditions were not met.",
            "No ordinary drive cycle missed the voltage mark on either cell: of 80, the card ran 78, and 72 were inside the marks.",
            "3.3 to 4.0 percent more charge than was measured, at four of five temperatures",
            "128 to 145 millivolts high",
            "its discharge at the rated current at -10 degrees Celsius",
            "All ten hill-climb cycles are among them",
            "untested on new cells",
        ] {
            assert!(result.contains(words), "{words}");
        }
        assert!(!result.contains("not yet run"));
        let data = record["data"].as_str().unwrap();
        assert!(data.contains("doi 10.5683/SP3/ZVTR4B, version 3.0, CC BY 4.0"));
        let line = record["pass_line"].as_str().unwrap();
        for mark in ["42 millivolts root mean square", "within 3 percent", "still running at the last sample", "at most 126 millivolts"] {
            assert!(line.contains(mark), "{mark}");
        }
        let limits = record["limits"].as_str().unwrap();
        for words in [
            "not a blind test",
            "Three hill cycles",
            "130 to 180 millivolts high",
            "3 of the 13 runs",
            "-20 degrees Celsius is outside the card",
            "are now spent",
            "One developer's SI wrote the model, wrote the card and ran the comparison, on one machine",
            "A second agent, which did not write the card, scored it again",
            "the same verdict as the record on 102 of 105 runs",
            "the card steps by one second only",
            "49 mV from the one-second answer in the reversal that was compared",
            "It is the same developer's SI, and no one outside Protocol Company, which makes this engine, has repeated the result.",
        ] {
            assert!(limits.contains(words), "{words}");
        }
        assert!(!limits.contains("not yet repeated by a second party"));
        assert!(!limits.contains("Known defect"));
        assert!(!record.to_string().contains("seal"));
        assert!(!record.to_string().contains("build"));

        assert_eq!(body["advisory_flags"].as_array().unwrap().len(), 2);
        assert_eq!(body["advisory_flags"][0]["state"], "sustained_above_1c");
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
            ("pos[0]", "terminal_v", "V"),
            ("pos[1]", "charge_removed_ah", "Ah"),
            ("pos[2]", "surface_state_of_charge", "1"),
            ("vel[0]", "reaction_overpotential_v", "V"),
            ("vel[1]", "hysteresis_v", "V"),
            ("vel[2]", "electrolyte_v", "V"),
            ("force_torque", "current_mean_a", "A"),
            ("residual", "residual_conservation_error", "1"),
        ] {
            assert_eq!(slot(name)["state"], state, "{name}");
            assert_eq!(slot(name)["unit"], unit, "{name}");
        }

        let step = &body["example_compute_step"]["parameters"];
        assert_eq!(step["dt_s"].as_f64().unwrap().to_bits(), 1.0_f64.to_bits());
        assert_eq!(step["cell_number"].as_f64().unwrap(), 80.0);
        assert_eq!(step["state_of_charge"].as_f64().unwrap(), 1.0);
        assert_eq!(step["capacity_ah"].as_f64().unwrap(), 4.5006);
        assert_eq!(step["current_a"].as_f64().unwrap(), 4.5);
        let run = &body["example_run_trajectory"];
        assert_eq!(run["segments"][0]["steps"], 600);
        assert_eq!(run["record_every"], 60);
        // The table card of the same cell is served beside it.
        let earlier = find("energy", "cell_2170_m80").expect("the earlier card is served");
        assert_eq!(earlier.rejects.len(), 13);
        assert_eq!(earlier.ceiling_s, 1.0);
    }

    /// The creep card lists every variant of its `StepReject`, in the kernel's
    /// order, with the kernel's sentence word for word.
    #[test]
    fn the_creep_refusals_carry_the_sentences_from_the_card() {
        let documented = documented_rejects(CREEP_SOURCE);
        let card = find("materials", "creep_cmsx6").expect("the creep card is served");
        assert_eq!(documented.len(), 12);
        assert_eq!(card.rejects.len(), documented.len());
        for ((name, sentence), listed) in documented.iter().zip(card.rejects) {
            assert_eq!(name, listed);
            assert_eq!(reject_meaning("creep_cmsx6", name), Some(sentence.as_str()), "{name}");
        }
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
        assert!(dt["rule"].as_str().unwrap().contains("Below 0.1 percent it missed at every stress"));
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
        assert!(body["specimen"].as_str().is_some_and(|line| line.ends_with("the card answers there with those misses stated, and a frame there says so.")), "{}", body["specimen"]);
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

    /// The creep card's validation record: the citation, the split, the hash
    /// of the pre-registration with the three lines, and the limits.
    #[test]
    fn the_creep_validation_record_states_the_data_the_lines_and_the_limits() {
        let card = find("materials", "creep_cmsx6").expect("the creep card is served");
        let record = &describe_one(card)["validation"];
        let part = |name: &str| record[name].as_str().unwrap().to_string();
        for words in ["CMSX-6", "980 degrees Celsius", "140, 170, 200 and 230 MPa", "6 mm in diameter"] {
            assert!(part("object").contains(words), "{words}");
        }
        for words in [
            "doi 10.5281/zenodo.20132712, CC BY 4.0",
            "six constants were fitted to six tests",
            "all three at 200 MPa, a stress no fitted test has",
        ] {
            assert!(part("data").contains(words), "{words}");
        }
        for words in [
            "f20f99b0e3419d66201a3e124b5c216c4d138d41f976b19d6a536daf15316d87",
            "two numbered amendments made before the held-back tests were read, and a third, a correction of the forecast, made after they were scored",
            "root mean square at most 0.15, worst at most 0.30, mean at most 0.10 in size",
        ] {
            assert!(part("pass_line").contains(words), "{words}");
        }
        for words in [
            "One cast, one temperature, one laboratory.",
            "The card's six constants are fitted to six tests of this cast",
            "chosen after the fitted tests were seen",
            "the 170 MPa tests predicted from 140 and 230 MPa alone (0.118, 0.229, -0.039)",
            "The miss did not happen",
            "amendment 3 (SHA-256 99f461ffb0f589334d841590c456ba7c3027651f7a0249bf166ebcfe57e2e60f), written after the held-back tests were scored and so not a pre-registration",
            "no constant of the card changed",
            "a root mean square of 0.075; the card's is 0.093",
            "with no creep law, also meets the three lines on the same 33 times (0.098, 0.237, -0.013)",
            "200 MPa lies where the stress dependence is straight",
            "a Norton power law, misses on the same six tests (0.436 root mean square, 0.952 worst)",
            "it starts later than every one of them at all four stresses",
            "38.9 against 15.4 to 26.7 at 140 MPa, 7.8 against 2.7 to 3.5 at 170 MPa, 1.69 against 0.61 to 0.66 at 200 MPa and 0.245 against 0.06 to 0.08 at 230 MPa",
            "The card answers there, and each frame carries FLAG_OUTSIDE_MET_RANGE",
            "The card answers past 0.2 percent below 170 MPa with this miss stated, about a factor of 2",
            "the worst is 0.289, at 170 MPa and 0.33 percent on a held-back test, a margin of 0.011",
            "the card's 1 to 2.09 percent at 140 MPa is beyond any measurement",
            "it reaches 0.8 percent at 683 h where the two tests that got there took about 1000 h",
            "not physical constants of the alloy",
            "not compared on a registered level",
            "Scored a second time on 2026-10-09 by a second agent with programs of its own (evidence/verification/second_party/V-24/)",
            "on the same machine",
        ] {
            assert!(part("limits").contains(words), "{words}");
        }
        // What the first record said and the correction withdrew is quoted only as withdrawn.
        assert!(!part("limits").contains("not yet repeated by a second party"));
        assert!(!part("limits").contains("it missed them"));
        assert!(!part("limits").contains("two stresses cannot show"));
        assert_eq!(part("result"), CREEP_CMSX6_RESULT);
        assert_ne!(part("result"), NOT_COMPARED);
        for words in [
            "Compared with the six held-back tests on 2026-10-09, and by the rule fixed beforehand the comparison passed.",
            "a root mean square of 0.093 against a line of 0.15, a worst of 0.240 against 0.30 and a mean of +0.019 against 0.10",
            "On the three tests at 200 MPa, the stress no fitted test has: 0.085, 0.170 and +0.028",
            "the card is 27 percent late",
        ] {
            assert!(part("result").contains(words), "{words}");
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
            "Only runs from the locked entry state at 0.1 s were compared with the flight",
            "any other start or path is computed and was not compared",
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
            "Karatekin and Van Hove",
            "MROM_2147",
            "the lander's barometer",
            "No number in the card comes from the accelerometer",
            "the last 59 s before the parachute were held back unread",
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
            "Compared with the held-back rows on 2026-10-09",
            "a lift over drag of -0.031: all four first lines met and all six stated-spread marks met",
            "With no lift: none of the four first lines met",
            "one stated-spread mark missed, the time of the peak, 4.02 s against 3.27",
            "7.60 g against 8.12 g",
        ] {
            assert!(part("result").contains(words), "{words}");
        }
        for words in [
            "a single number measured on the opened 150 s",
            "does not forecast an entry whose lift is not known",
            "a comparison and not a test",
            "8 to 13 percent denser",
            "sideways lift",
            "Engines 0.7 to 0.9 refused below 5700 m or Mach 1.46",
            "the card now ends a run at the flown parachute deployment",
            "not yet repeated on another machine or by anyone outside the company",
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
                Ok(response) => assert_eq!(response.sha256_seal.len(), 64, "{}", item.card),
                Err(error) => panic!("{} step: {}", item.card, error.message()),
            }
            examples += 1;
            let run_text = serde_json::to_string(&body["example_run_trajectory"]).unwrap();
            let run: Value = serde_json::from_str(&run_text).unwrap();
            assert_eq!(run["domain"], item.domain);
            assert_eq!(run["card"], item.card);
            match crate::trajectory::run(&state, run).await {
                // The run of each reactor card goes to the first maximum of
                // power, the end the card declares, and is sealed as ended.
                Ok(response)
                    if item.card == "reactor_spert3" || item.card == "reactor_spert3_fuel" =>
                {
                    assert_eq!(response.outcome, "ended", "{:?}", response.reject_detail);
                    assert_eq!(response.reject.as_deref(), Some("PastFirstMaximum"));
                    assert!(response.steps_landed < response.steps_requested);
                }
                // The C-core reactor's run goes on past the power peak to the first maximum of
                // the hot-spot plate temperature, the end it declares.
                Ok(response) if item.card == "reactor_spert3_ccore" => {
                    assert_eq!(response.outcome, "ended", "{:?}", response.reject_detail);
                    assert_eq!(response.reject.as_deref(), Some("PastPlateTemperatureMaximum"));
                    assert!(response.steps_landed < response.steps_requested);
                }
                Ok(response) => assert_eq!(
                    response.outcome, "completed",
                    "{} {:?}",
                    item.card, response.reject_detail
                ),
                Err(error) => panic!("{} trajectory: {}", item.card, error.message()),
            }
            examples += 1;
        }
        assert_eq!(examples, 74);
    }
}
