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
    capsule_insight,
    crater_ryugu_sci,
    link_gothenburg_10010,
    grid_gb,
    solar_system,
    tire_eve_r19_ice,
    accumulator_sbo450,
    los_jwst_fine_guide,
    tokamak_jet_dte1,
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
/// two forms: that the short one exists, and when to read the full one.
pub(crate) const SHORT_FORM_SAYS: &str = "With form \"short\" the answer is a page, enough to write a first request: what the model is of, how it did against measurement, each input with its unit and range, the names of its stops, and a worked request. Read the full form, the answer when form is left out, when a request is refused or before relying on a result: only it has each bound's reason, the validation record and each stop's meaning.";

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
    described.ok_or_else(|| DoorError::NotFound("Card not recognized".to_string()))
}

fn required_string(object: &Map<String, Value>, key: &str) -> Result<String, DoorError> {
    match object.get(key) {
        None => Err(DoorError::Unprocessable(format!(
            "Missing required parameter: '{key}'"
        ))),
        Some(Value::String(text)) if !text.is_empty() => Ok(text.clone()),
        Some(Value::String(_)) => Err(DoorError::Unprocessable(format!(
            "Missing required parameter: '{key}'"
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

pub fn verify_step_schema_in(view: &View) -> Map<String, Value> {
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
        "description": "One anchored step. The same request as POST /verify_physics. The shape of parameters depends on the pair: read describe_card for it."
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
        "One anchored run. Total steps must be <= {}. Recorded frames must be <= {}. The shape of initial and of each segment's load depends on the pair: read describe_card for it.",
        super::trajectory::MAX_TRAJECTORY_STEPS,
        super::trajectory::MAX_RECORDED_FRAMES
    );
    let (domain, card) = pair_properties(view);
    let initial = json!({
        "type": "object",
        "description": "The phase fields of the card at the start of the run, each a number (a vector field is an array of three numbers), and no other key. describe_card gives them, with each unit and bound."
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
                        "load": load
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
                "description": "With run_seal alone: the run_trajectory request as it was sent, with its readout and its from when it had them (the answer's inputs holds neither). It is run again, under a physics permit and the limits of run_trajectory, and its seal is compared with run_seal. Its fields are run_trajectory's."
            },
            "inputs": {
                "type": "object",
                "description": "The inputs of the run_trajectory answer, sent back as returned: the request that was run. It is hashed, not run. A card's fields, units and bounds are in describe_card."
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
        "description": "Either request and run_seal alone, the short form: the run is computed again and its seal compared with run_seal. Or the whole run_trajectory answer sent back as it was returned, without request: inputs, frames, outcome, steps_landed, reject, run_seal, final_state, final_residual_conservation_error_hex, max_residual_conservation_error_hex and state_digest are then all required, and the trajectory chain, the state digest and the run seal are recomputed with no physics run."
    }))
}

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
    ]
}

/// The one line an agent reads in `list_cards`.
pub(crate) const ACCUMULATOR_SBO450_SPECIMEN: &str = "The nitrogen charge of one HYDAC SBO450-0.6 diaphragm accumulator on the test rig at TU Darmstadt, as a piston pushes oil in and draws it out: gas pressure, temperature and an energy ledger. The ratio of piston area to vessel volume and the thermal time constant are fitted to this accumulator's own runs at 40 bar precharge and 60 bar. Compared with 285 held-back runs at 40, 80 and 120 bar and at half the charge, 0.01 to 20 Hz: the comparison failed, two of six lines met, and a second scoring confirmed it. Amended after that with every run opened, which is not a new test: the card now refuses by name outside the range in which it held: a precharge of 20 bar at gas pressures of 37 to 43 or 55 to 67 bar, or of 40 bar at 57 to 64 or 75 to 86 bar, the bands in which each charge was measured, a wall at 293.4 to 293.7 K, gas at 285.9 to 301.8 K and a piston at most 0.7 m/s, as in the measured runs, and heat at most 0.065 of the work over the whole motion, which in the measured sines is 0.2 Hz and faster. Run after the fact it answers 196 of those 285 runs and refuses 89; on the 196 the swing of gas pressure per piston travel is within 0.49 percent in the median, 1.37 in the worst record and 4.05 at most.";

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
                Some(accumulator_sbo450::GAS_MASS_20_BAR_FLOOR_KG),
                false,
                Some(accumulator_sbo450::GAS_MASS_40_BAR_CEILING_KG),
                false,
                "inclusive [0.013639, 0.027495] kg, and inside that only the two charges measured: 0.013639 to 0.013667 kg, a precharge of 20 bar, or 0.02744 to 0.027495 kg, a precharge of 40 bar. A mass between the two bands is inside the published minimum and maximum and is refused as GasMassOutsideCard: no charge between the two was measured. The nitrogen in the accumulator, which never changes. A charge is the vessel's 0.58879 l full of nitrogen at the precharge, absolute, and 293.15 K by the card's own equation of state: 0.013653 kg for 20 bar and 0.027467 kg for 40 bar. Each band is a thousandth either side, so a charge typed to four figures is inside, and the pressure swing moves by the same share as the mass. The card as it was scored on 2026-10-09 held a smaller vessel and its charges were 0.01336 and 0.02688 kg; those are now refused".into(),
            ),
            scalar(
                "gas_pressure_pa",
                Role::Phase,
                "Pa",
                Some(accumulator_sbo450::PRESSURE_FLOOR_PA),
                false,
                Some(accumulator_sbo450::PRESSURE_CEILING_PA),
                false,
                "inclusive [3700000, 8600000] Pa, absolute, and inside that only the bands in which the charge given was compared with measurement: 3700000 to 4300000 Pa or 5500000 to 6700000 Pa for the 20 bar charge, 5700000 to 6400000 Pa or 7500000 to 8600000 Pa for the 40 bar charge. The gas pressure at the start; the card works the gas volume back from it. A start or a step landing below 37 or above 86 bar is refused as PressureOutsideCard. A start or a step landing inside the published minimum and maximum and outside the bands of its charge is refused as PressureOutsideChargeBands: that charge was never measured there. Each band is the whole bars outside the gas pressures measured in one record whose runs the card answers: 37.76 to 42.71 bar at 20 / 40, 55.26 to 66.37 bar at 20 / 60, 57.36 to 63.14 bar at 40 / 60 and 75.31 to 85.55 bar at 40 / 80. The bands lie above both precharges, so inside them the vessel always holds oil".into(),
            ),
            scalar(
                "gas_temperature_k",
                Role::Phase,
                "K",
                Some(accumulator_sbo450::GAS_TEMPERATURE_FLOOR_K),
                false,
                Some(accumulator_sbo450::GAS_TEMPERATURE_CEILING_K),
                false,
                "inclusive [285.9, 301.8] K: the gas temperature at the start. A start, or a step that would land, outside the range is refused as GasTemperatureOutsideCard. The compared runs reached 285.97 to 301.78 K over every step they landed, and the range is the next tenth of a kelvin outside that; no other gas temperature was compared with measurement".into(),
            ),
            scalar(
                "wall_temperature_k",
                Role::Phase,
                "K",
                Some(accumulator_sbo450::WALL_TEMPERATURE_FLOOR_K),
                false,
                Some(accumulator_sbo450::WALL_TEMPERATURE_CEILING_K),
                false,
                "inclusive [293.4, 293.7] K: the temperature of the wall, held for the whole run. It is part of the start and no segment can change it. The wall of every compared run, the mean room temperature of its record, was 293.46 to 293.60 K, and the range is the next tenth of a kelvin outside that. A wall outside it is refused as WallTemperatureOutsideCard: no other wall temperature was measured".into(),
            ),
            scalar(
                "work_moved_j",
                Role::Phase,
                "J",
                Some(0.0),
                false,
                None,
                false,
                "at least 0 J, required: M at the start, the work of the oil on the gas, either way, summed step by step over the motion before this start. State 0 for gas that has not moved. To carry on a run or a single step, state the work_moved_j of its returned state; the run then weighs its heat against the whole motion, as one run would. A start whose heat_moved_j is already more than 0.065 of the larger of work_moved_j and 100 J is refused as HeatShareAboveCard, and a sum below zero as NegativeMovedSum. The card cannot see motion before its start, so it holds the sum it is given and cannot check it; a caller who states a sum that is not so gets answers the comparison does not cover".into(),
            ),
            scalar(
                "heat_moved_j",
                Role::Phase,
                "J",
                Some(0.0),
                false,
                None,
                false,
                "at least 0 J, required: H at the start, the heat between the gas and the wall, either way, summed step by step over the motion before this start. State 0 for gas that has not moved. To carry on a run or a single step, state the heat_moved_j of its returned state. A start whose sum is already more than 0.065 of the larger of work_moved_j and 100 J is refused as HeatShareAboveCard, and a sum below zero as NegativeMovedSum. The card cannot check it".into(),
            ),
            scalar(
                "piston_speed_m_s",
                Role::Load,
                "m/s",
                Some(-accumulator_sbo450::PISTON_SPEED_CEILING_M_S),
                false,
                Some(accumulator_sbo450::PISTON_SPEED_CEILING_M_S),
                false,
                "inclusive [-0.7, 0.7] m/s: the speed of the piston, held over a step. Positive pushes oil in and compresses the gas. The fastest compared run, 5 mm at 20 Hz, reached 0.69999 m/s as the mean over a step; a faster speed is refused as PistonSpeedOutsideCard. A speed times dt_s above 1 mm is refused as PistonTravelAboveStep".into(),
            ),
        ],
        floor,
        "the ceiling divided by 2^20, about 3.34e-7 s",
        gate,
        "0.3504 s, a fiftieth of the thermal time constant of 17.52 s",
        None,
        "Over a step the piston speed is held, and the gas volume changes by the piston area, 19.635 cm2 (a 50 mm bore), times the travel. The gas is nitrogen by the Peng-Robinson equation of state; it exchanges heat with the wall with one time constant, 17.52 s; the vessel's gas volume is 0.58879 l. The time constant and the ratio of the area to the volume are fitted to this accumulator's own runs. A run starts at time zero with the work and the heat at zero; the time, the work, the heat and the energy at the start are the anchor of the ledger and begin again at each start; the ledger's conservation check is held from each start, and no limit of the card's range depends on them. The card keeps two more sums over the steps of the motion: M, the absolute work of each step, and H, the absolute heat of each step. A start carries them in as work_moved_j and heat_moved_j, 0 for gas that has not moved; each step adds to them; the state gives them back under the same names, and heat_share is H over the larger of M and 100 J. A step lands only if H is at most 0.065 of the larger of M and 100 J; otherwise it is refused as HeatShareAboveCard and the run ends there, sealed, with its proof up to the last step that landed. In the measured sines that refuses 0.1 Hz and slower and answers 0.2 Hz and faster. A piston at rest lands for as long as the gas is at the wall's temperature; off it, a rest from the start of a run is answered for its first 6.5 J of heat. A single step or a run is carried on by handing back its returned gas_pressure_pa, gas_temperature_k, work_moved_j and heat_moved_j as a new start: the two sums travel with the motion, so single steps handed back, runs carried on from each other and one run are weighed alike, and the step a run refuses is refused when it is asked alone from that run's returned state. A start without the two sums is refused as a missing input. The four limits the card works out itself (the travel in a step, the pressure and the volume worked back from the starting pressure, and the heat against the work) carry an allowance of a relative 1e-12, so a start, a step or a run stated or landing exactly at a limit is inside it. A piston speed outside its range in any segment of a run refuses the whole run before its first step. A run ends in a sealed refusal at the step that would land outside the pressure, the pressure bands of its charge, the temperature, the vessel volume or the share of heat.".to_string(),
        &[
            "NonFiniteInput",
            "NonFiniteState",
            "NonPositiveDt",
            "InvalidAccumulator",
            "GasMassOutsideCard",
            "WallTemperatureOutsideCard",
            "PistonSpeedOutsideCard",
            "PistonTravelAboveStep",
            "GasTemperatureOutsideCard",
            "PressureOutsideCard",
            "PressureOutsideChargeBands",
            "NegativeMovedSum",
            "OilSideEmpty",
            "HeatShareAboveCard",
            "SpontaneousGeneration",
            "FirstLawBreach",
        ],
        vec![],
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
                "Always zero. The card has no advisory condition: a gas pressure, a gas mass or a share of heat outside the card is a refusal.",
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
        "The crater made on asteroid Ryugu by the 2 kg copper impactor of Hayabusa2 on 5 April 2019: its radius growing from 1.75 m until it stops, by the law of crater size for dry sand held by gravity alone, fitted in the laboratory on Earth and not to Ryugu, with no load; its predicted radius is 18 percent above the measured crater, a miss of its own line, and its clock is not compared with Ryugu.",
        vec![scalar(
            "radius_m",
            Role::Phase,
            "m",
            Some(crater_ryugu_sci::RADIUS_FLOOR_M),
            false,
            Some(crater_ryugu_sci::FINAL_RADIUS_M),
            false,
            format!(
                "inclusive [{}, {}] m: the crater's radius at the level of the undisturbed surface, from 0.917 s after the impact by the card's clock to the radius at which this crater stops. The speed of the wall is the one this impact gives it at that radius; the caller does not set it",
                token(crater_ryugu_sci::RADIUS_FLOOR_M),
                token(crater_ryugu_sci::FINAL_RADIUS_M)
            ),
        )],
        floor,
        "the ceiling divided by 2^20: a step that moves the wall by about a tenth of a micrometre at its fastest. Under it a step is mostly rounding",
        gate,
        "0.25 s, a locked constant: about 1,600 steps from 1.75 m to the stop. Over that whole growth the largest conservation residual at this step is 5.8e-8 against a limit of 1e-6; at 0.5 s it is 8.8e-7 and at 1 s the first step is refused",
        None,
        "No load: nothing acts on the crater but the impact and Ryugu, and a run segment carries an empty load object. A run starts at time zero at the radius given, with the wall at the speed this impact gives it there; 1.75 m is 0.917 s after the impact by the card's clock. A single step starts in the same way at each call: its radius_m handed back as a new start continues the same growth, and t_s begins again at zero. The radius only grows. On the step in which the wall would turn back, the crater is set at 8.571304329477922 m with the wall at rest, the frame carries FLAG_STOPPED, and from then on a step moves only the time. From 1.75 m at the step ceiling that is step 1,597, at 399.25 s. The time is the clock of Gault and Wedekind (1977), which runs until the ejecta near the rim have landed, and is not compared with Ryugu.".into(),
        &[
            "NonFiniteInput",
            "NonFiniteState",
            "NonPositiveDt",
            "InvalidCrater",
            "RadiusBelowCard",
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
        "An annealed Ti-6Al-4V lattice column under nodal force, torque, and temperature.",
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
        "A copper fuse link between an upstream voltage and a load current.",
        vec![
            within(
                "current_a",
                Role::Phase,
                "A",
                melt_current_a,
                format!(
                    "from -{0} to {0} A, both inclusive. The bound is the specimen's melting point: the current whose energy in the feeder inductance equals the heat that takes the link from the reference temperature, 293.15 K, to its melting point, 1357.77 K. A feeder that holds more has more energy than the link can take as heat without melting. A step that lands with |current_a| above the bound is refused as CurrentOutsideCard and that state is not sealed",
                    token(melt_current_a)
                ),
            ),
            within(
                "voltage_v",
                Role::Phase,
                "V",
                melt_voltage_v,
                format!(
                    "from -{0} to {0} V, both inclusive. The bound is the specimen's melting point: the voltage whose energy in the shunt capacitance equals the heat that takes the link from the reference temperature, 293.15 K, to its melting point, 1357.77 K. A step that lands with |voltage_v| above the bound is refused as VoltageOutsideCard and that state is not sealed. Inside the range a node voltage at or above 120 V is flagged FLAG_GRID_SURGE and still seals",
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
        "A synchronous 60 Hz machine under mechanical power.",
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
        "A 12S lithium-ion pack under a terminal current.",
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
        "One Panasonic NCR18650PF cell in a held 1C discharge from full charge, with no temperature and no memory of the load before (a real cell's voltage relaxes for minutes after a change of load); it takes only the 1C currents compared with measurement, 2.89817 to 2.90065 A.",
        vec![
            scalar(
                "soc",
                Role::Phase,
                "1",
                Some(cell_18650pf::SOC_MIN),
                false,
                Some(1.0),
                false,
                "inclusive minimum 0.03, inclusive maximum 1; below 1 the run is the continuation of the 1C discharge whose held current held_current_a states".into(),
            ),
            scalar(
                "held_current_a",
                Role::Phase,
                "A",
                Some(cell_18650pf::CURRENT_COMPARED_MIN_A),
                false,
                Some(cell_18650pf::CURRENT_COMPARED_MAX_A),
                false,
                "the held current of the 1C discharge this run belongs to: for a run from full charge, the current it is held at; for a run from a state an earlier run returned, that run's held current. Inclusive minimum 2.89817 A, inclusive maximum 2.90065 A (CurrentNotCompared)".into(),
            ),
            scalar(
                "current_a",
                Role::Load,
                "A",
                Some(cell_18650pf::CURRENT_COMPARED_MIN_A),
                false,
                Some(cell_18650pf::CURRENT_COMPARED_MAX_A),
                false,
                "inclusive minimum 2.89817 A, inclusive maximum 2.90065 A: the 1C currents compared with measurement, 2.899 to 2.89982 A, widened on each side by the 0.83 mA band (CurrentNotCompared)".into(),
            ),
        ],
        floor,
        "the ceiling divided by 2^20: at 3.0 A, five parts in a thousand million of state of charge. Under it a step is mostly rounding",
        gate,
        "the time to cross one table interval at 3.0 A",
        None,
        "current_a is in [2.89817, 2.90065] A: the measured current fed to every compared discharge moves between 2.899 and 2.89982 A, and the card takes that span widened on each side by the 0.83 mA band. Any other current, a rest at 0 A among them, is refused as CurrentNotCompared; the card's tables were fitted at other currents. One discharge at another current was left out of the fitting: 04-27-17 3541_dis5_10p.mat, the same cell at 25 degrees Celsius discharged at a held 0.86888 to 0.86970 A from full charge to 2.5 V in 14 holds with rests at 0 A between them. Its totals and its voltage trace had been read before it was compared; no output of the card had. Compared on 2026-10-10, each hold run as a continuation of the one before and the rests not compared, at steps of 0.1 s, it failed: the voltage error during the holds was 56.7 millivolts root mean square against the line of 50, rising to 204 and 206 in the last two holds, and the card did not end by its own cutoff but left its table at soc 0.03 and 2.66 V (StateOfChargeOutsideCard), 71 s after the cell reached 2.5 V, having delivered 0.60 percent more than the cell. 0.87 A is therefore refused: evidence/research/panasonic_18650pf/held_0p87A_comparison. A current outside that range in any segment of a run refuses the whole run before its first step, so a later segment cannot be tried after earlier ones have landed. A run is one discharge at one held current: every segment's current_a must lie within 0.00083 A (HELD_CURRENT_BAND_A) of the held current, and a segment outside that band refuses the whole run before its first step as CurrentChangedDuringDischarge, with nothing sealed. The held current is held_current_a, which the request states. A returned state does not carry held_current_a; to continue from it, the request states the same held_current_a as the run that returned it. Held means what the tester held: the measured current moves 0.82 mA within a run, one step of the 0.81 mA in which the tester logs its current; the dataset states no accuracy for the tester, so the band is that spread, written as 0.83 mA, and the test allows four units in the last place at the size of the currents, so that two decimal currents exactly 0.83 mA apart are inside it at every current. A rest, a second rate, a pulse and a drive cycle are refused: the card has no memory, the real cell's voltage relaxes for minutes after a change of load, and no changing current was compared with a measurement of this cell. Every discharge that passed the comparison started at full charge, soc 1. A run or a step from soc below 1 is taken only as the continuation of a 1C discharge: initial states held_current_a, the held current of the earlier run, and every segment is held to its band, so a restart from a returned state cannot change the current. The card cannot check that the earlier run happened; it holds the continuation to the caller's statement. Inside the card a state below full charge that carries no held current is refused as StartNotAtFullCharge; every state the door builds carries held_current_a. A run continued from a state another run returned, at the same held current, gives the same state of charge and terminal voltage as the straight run; the ledger's anchor, time and work start again at the continuation, and no limit depends on them. A single step takes the state of charge and held_current_a only: time, work and the ledger's anchor start again at each call. A run whose end in decimal is exactly the bottom table point, soc 0.03, lands its last step: a state of charge summed in binary that lands under 0.03 by no more than one unit in the last place at 1 for each step since the run or continuation began, plus four, lands on 0.03. At 1C the 2.5 V cutoff stops a discharge near soc 0.05, before that point. A step of zero or less is refused as NonPositiveDt before the step floor is tested.".into(),
        &[
            "NonFiniteInput",
            "NonFiniteState",
            "NonPositiveDt",
            "InvalidCell",
            "CurrentNotCompared",
            "CurrentChangedDuringDischarge",
            "StartNotAtFullCharge",
            "StateOfChargeOutsideCard",
            "UndervoltageCutoff",
            "SpontaneousGeneration",
            "FirstLawBreach",
        ],
        vec![Advisory {
            name: "FLAG_SAG_WARNING",
            mask: cell_18650pf::FLAG_SAG_WARNING,
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
        "An avionics cold plate under heater power and a bath temperature.",
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
        "A 10 Hz mount oscillator under an imposed acceleration.",
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
                    "from 0 J, inclusive, to {} J, inclusive, four stroke energies: the mechanical energy the ledger of the run is anchored at. A new start states the mechanical energy of its own displacement and velocity, 0.5 m v^2 + 0.5 k x^2 with m 1 kg and k = m (20 pi)^2 N/m. A landed state handed back as a new start carries its own energy_anchor_j, so the anchor of the run stays where it was. A start whose own mechanical energy is above four stroke energies is refused as AmplitudeAboveCard; otherwise an anchor below 0 or above four stroke energies is refused as LedgerNotOfThisState",
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
            "AmplitudeAboveCard",
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
        "A drive-mode Duffing resonator under acceleration, an external rate, and sensed velocity.",
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
        "A mineral-oil column under a commanded flow.",
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
        "A Dyneema tendon under a commanded force.",
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
    let last_start_s = unit.last_start_mission_time_s().expect("latest start");
    let floor = unit.floor_dt_s().expect("specimen floor");
    finish(
        "energy",
        "generator_mmrtg_f1",
        "One radioisotope generator, flight unit F1, on the Curiosity rover: its daily-average electrical output from landing to mission day 4,087, with no load; on the 1,279 held-back days, 2,809 to 4,087, it was inside its line of 0.010 of 115.4 W by root mean square (0.008129, about 0.94 W) and outside it in its mean over the final 365 days (+0.010094, about 1.16 W high), so it failed its comparison by the rule fixed beforehand and is served with that stated. Its worst day read 2.60 W high.",
        vec![scalar(
            "mission_time_s",
            Role::Phase,
            "s",
            Some(0.0),
            false,
            Some(last_start_s),
            false,
            format!(
                "inclusive [0, {}] s. The maximum is the latest start, not the last day: it lies one smallest step of the card, about 4.03 s, before the end of mission day 4,087 ({} s), so that the smallest step still lands. A later start lands no step. The returned state gives the mission time as t_s",
                token(last_start_s),
                token(last_s)
            ),
        )],
        floor,
        "the ceiling divided by 2^20: one part in a thousand million of thermal power. Under it a step is mostly rounding",
        gate,
        "0.02 rad on its fastest rate, fuel decay plus converter fading",
        nyquist,
        "No load. A run segment carries an empty load object. A step that would land after mission day 4,087, the last day of the published record, is refused as MissionTimeOutsideCard. Mission days 2,809 to 4,087 are the held-back days the card was compared with and failed on; it answers them as the model that was scored, with no constant changed since. The mission clock is a compensated sum: each step adds dt_s to t_s in 64-bit arithmetic, keeps what the addition rounded off and folds it back in, so t_s stays within half a spacing of 64-bit numbers of the exact sum of the start and the steps at any length of run. Every step, the first included, compares its landing with mission day 4,087 plus one tolerance, MISSION_END_TOLERANCE_S, 2^-22 s or about 2.38e-7 s: four spacings of 64-bit numbers between 2^28 s and 2^29 s, where the last day lies, enough for a start and a step read from decimals and for the clock's own rounding, whether a run starts after 2^28 s, mission day 3,106.9, or before it, where the spacing is finer. A run or a single step whose end in decimal is exactly mission day 4,087 lands its last step, and its last t_s can read up to the tolerance past the last day, where range_margin_s is zero. A step whose landing, as the clock reads it, is later than mission day 4,087 by more than the tolerance is refused, whether it is the first step or the millionth: a step whose end in decimal is later than the day by more than twice the tolerance, 4.8e-7 s, is refused by every route. A restart from a returned t_s begins with no carry. A run whose end in decimal is on or before mission day 4,087 lands its last step whether it is run straight through or restarted from any returned state, and one whose end is later than the day by more than 4.8e-7 s is refused by both; between those, inside the clock's rounding, the two routes can differ. Through engine 0.6.0, when the card ended at mission day 2,808, the comparison allowed 1.49e-8 s for each step a run of that step takes from landing, up to 0.90 s at the step floor, so a run of two steps could land 0.89 s past the card's end with a proof; since then no step lands past the end by more than the tolerance. electrical_energy_j, heat_rejected_j and fading_electrical_energy_j count from the start of this run, the anchor of the ledger, and not from landing: energy_anchor_j is the decay heat held at that start. A restart from a returned state starts each again at zero. No refusal depends on these totals: every limit of the card is on the mission time or on a single step's ledgers.".into(),
        &[
            "NonFiniteInput",
            "NonFiniteState",
            "NonPositiveDt",
            "InvalidGenerator",
            "MissionTimeOutsideCard",
            "SpontaneousGeneration",
            "FirstLawBreach",
            "ConverterAccountBreach",
        ],
        vec![Advisory {
            name: "FLAG_PAST_FITTING_SPAN",
            mask: generator_mmrtg_f1::FLAG_PAST_FITTING_SPAN,
            seals: true,
        }],
        slots(&[
            ("t", "Mission time since landing, in seconds."),
            ("pos[0]", "electrical_power_w"),
            ("pos[1]", "thermal_power_w"),
            ("pos[2]", "converter_fading_fraction"),
            ("vel[0]", "electrical_energy_j"),
            ("vel[1]", "heat_rejected_j"),
            ("vel[2]", "fuel_heat_j"),
            ("force_torque", "Range margin, in seconds."),
            ("residual", "Conservation residual."),
            (
                "flags",
                "FLAG_PAST_FITTING_SPAN after mission day 2,060. It seals.",
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
    let coolant_min = reactor_spert3::COOLANT_TEMPERATURE_MIN_K;
    let coolant_max = reactor_spert3::COOLANT_TEMPERATURE_MAX_K;
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
                    "inclusive [{}, {}] K, the primary coolant at the start, 56 to 92 degrees Fahrenheit, the span tested. The card records it and its feedback does not depend on it",
                    coolant_min,
                    coolant_max
                ),
            ),
            scalar(
                "reactivity_inserted_dollar",
                Role::Load,
                "dollar",
                Some(reactivity_min),
                false,
                Some(reactivity_max),
                false,
                format!(
                    "inclusive [{}, {}] dollars, a step in reactivity held from the start of the run: the inhour values for the card's set of the longest and the shortest tested reactor period, 1010 ms (test 22) and 10 ms (test 43), not rounded; a reactivity outside them is refused as ReactivityOutsideCard",
                    reactivity_min,
                    reactivity_max
                ),
            ),
        ],
        floor,
        "a locked 1e-9 s: the smallest step that still moves the clock at 20 s and moves the power at the start by a part that 64-bit numbers resolve",
        gate,
        "0.2 of the time constant of the fastest pole the card can reach, the prompt pole at the smallest reactivity with the feedback of the largest energy the card allows",
        None,
        "reactivity_inserted_dollar is in the published span. A run is one burst, with one reactivity: every segment carries the reactivity of the first, and segments divide a run without changing its load. A reactivity outside that span in any segment of a run refuses the whole run before its first step, and so does a segment whose reactivity is not the first segment's, as ReactivityChangedDuringBurst, with nothing sealed. The card is one step in reactivity at time zero, held to the first maximum of power, which is what the report tested: a second insertion, a withdrawal or a jump in the inserted reactivity is not on the card. The card's own step refuses the same change once the first step has landed, and keeps the state. NegativeState and InconsistentState are refusals of the card's step for a state handed to it that the card did not write; this door takes the coolant temperature only and hands no state in, so no request reaches them. coolant_temperature_k is in kelvin. The reactor starts at 5e-5 MW in equilibrium. A single step takes the coolant temperature and starts again from there at each call, so single steps cannot be chained. A run ends at the first maximum of power: the step after it is refused as PastFirstMaximum and the run is sealed with that refusal. Its last landed state carries peak_power_mw, peak_energy_mws and peak_time_s, the highest landed sample, which lies within one step of the true maximum; the card does not interpolate. Over the published span, runs at the ceiling and at half of it all ended at the first maximum before 20 s, 10 MW s and 1000 MW, so TimeBeyondCard, EnergyBeyondCard and PowerBeyondCard are named for the kernel and were not reached. At the ceiling the slowest reactivity takes 17,098 steps to the first maximum, inside the 1,000,000 steps of a run; a run of 1,000,000 steps at the floor stops at one millisecond, completed, before any maximum. A step of zero or less is refused as NonPositiveDt before the step floor is tested.".into(),
        &[
            "NonFiniteInput",
            "NonFiniteState",
            "NonPositiveDt",
            "InvalidCard",
            "ReactivityOutsideCard",
            "CoolantTemperatureOutsideCard",
            "PastFirstMaximum",
            "TimeBeyondCard",
            "EnergyBeyondCard",
            "PowerBeyondCard",
            "ReactivityChangedDuringBurst",
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
const GRID_GB_RESULT: &str = "Compared with the held-back trips on 2026-10-09, run once through the engine: all seven lines were met on 12 trips. One of the 13 was dropped before scoring by the quality rule written beforehand, the trip of 2 September 2021, whose recorded frequency moves 0.275 Hz in the ten seconds before the fall. That was two opposite losses at once, an export link and an import link, and not one trip. Depth of the fall: root mean square relative error 0.166 and mean -0.018, against 0.182 for the rule that depth is proportional to loss on the same trips; the worst was 37 percent too shallow, on 9 May 2019. Steepest second: 0.103 and mean +0.043. Frequency near 60 s: 0.046 Hz. The whole trace: median 0.036 Hz. No trip was refused by the card. The held-back twelve were the easiest third of three: on the other two thirds, each with constants refitted on the rest, the depth error is 0.258 and 0.326 with three lines missed each, and 18.7 percent of random twelves of the forty meet all seven lines. The card's margin over one-constant rules on the twelve is 0.015: depth proportional to loss 0.182, a fall to one fixed frequency 0.180, a straight line 0.171, the card 0.166. The loss and the lowest frequency of every trip, the held-back ones included, were on screen before the protocol was fingerprinted. A further 39 trips of the same kind, which the scan's rule had dropped because it reads the fall at the first second to reach 0.12 Hz, were found by the second scorer; 36 were inside the card as served through engine 0.6.0 and were scored after the fact, and six of them, of 500 to 692 MW, are under the losses the card takes from engine 0.7.0. All 36: depth 0.269 and near 60 s 0.067 Hz, two lines missed. May 2018 to February 2025, 23 trips: depth 0.136, all lines met. 2024 to 2026, 15 trips: depth 0.345, six lines missed. The six trips since November 2025 run 39 to 80 percent too deep. The binary that made the scored runs (SHA-256 da8ee131279452aa4f1f56afb71e801f622a050882554fb6d632dd23f9b7384c) was overwritten by a later build, which returned all 41 saved answers byte for byte. Since engine 0.7.0 the final state no longer carries below_statutory_limit, so the saved answers no longer come back byte for byte. A second scoring of 2026-10-09, on the same machine and from the scorer's own scan of the monthly files, found the same numbers: evidence/checks/second_party/V-32/.";

fn grid_gb_card() -> Card {
    let specimen = grid_gb::GridGb::interconnector_loss();
    // The ceiling is a fiftieth of the fastest time scale of the card, so no `π/ω`.
    let gate = specimen.gate_dt_s().expect("specimen gate");
    let floor = specimen.floor_dt_s().expect("specimen floor");
    finish(
        "energy",
        "grid_gb",
        "The Great Britain synchronous area, the 50 Hz transmission system of England, Scotland and Wales as one rotating mass, in the 60 seconds after an importing interconnector trips. It is the served model of that event at any date: on trips from mid-2018 to early 2025 it met its lines, and on the six compared trips since November 2025 its fall is 39 to 80 percent deeper than the measured fall.",
        vec![
            scalar(
                "frequency_hz",
                Role::Phase,
                "Hz",
                Some(grid_gb::START_FREQUENCY_MIN_HZ),
                false,
                Some(grid_gb::START_FREQUENCY_MAX_HZ),
                false,
                format!(
                    "inclusive [{}, {}] Hz, the system frequency at the start, with the area in balance: the span of the opened trips, 49.853 to 50.126 Hz, rounded outward to 0.05 Hz",
                    grid_gb::START_FREQUENCY_MIN_HZ,
                    grid_gb::START_FREQUENCY_MAX_HZ
                ),
            ),
            scalar(
                "inertia_mw_s",
                Role::Phase,
                "MW*s",
                Some(grid_gb::INERTIA_MIN_MW_S),
                false,
                Some(grid_gb::INERTIA_MAX_MW_S),
                false,
                format!(
                    "inclusive [{}, {}] MW s, the kinetic energy the area stores at 50 Hz: the operator's published outturn inertia for the half hour, in GVA s, times 1000. The span of the opened trips, 111 to 337 GVA s",
                    grid_gb::INERTIA_MIN_MW_S,
                    grid_gb::INERTIA_MAX_MW_S
                ),
            ),
            scalar(
                "demand_mw",
                Role::Phase,
                "MW",
                Some(grid_gb::DEMAND_MIN_MW),
                false,
                Some(grid_gb::DEMAND_MAX_MW),
                false,
                format!(
                    "inclusive [{}, {}] MW, generation and net imports on the transmission system just before the loss: the span of the opened trips",
                    grid_gb::DEMAND_MIN_MW,
                    grid_gb::DEMAND_MAX_MW
                ),
            ),
            scalar(
                "loss_mw",
                Role::Load,
                "MW",
                Some(grid_gb::LOSS_MIN_MW),
                false,
                Some(grid_gb::LOSS_MAX_MW),
                false,
                format!(
                    "inclusive [{}, {}] MW, the power lost, held through each step: the losses compared with measurement, the smallest of all 40 scored trips and the largest of the opened trips. The six trips of 500 to 692 MW inside the other spans, among those the scan lost, were run through the engine on 2026-10-10 by the protocol's lines and missed three of the seven, so a smaller loss is refused as LossOutsideCard; through engine 0.6.0 the card took any loss from 0 MW. No trip under 500 MW was found: the event rule takes only flows that fall by 500 MW or more. A zero loss is no event, and a loss of demand, a negative number, is not on the card",
                    grid_gb::LOSS_MIN_MW,
                    grid_gb::LOSS_MAX_MW
                ),
            ),
        ],
        floor,
        "a locked 1e-6 s: a step of that size still moves the clock at 60 s, and a loss of 1 MW on the largest inertia moves the frequency by 500 times the spacing of 64-bit numbers at 50",
        gate,
        "0.02 of the fastest time scale the card can reach, the swing of the rotating mass against the response at the smallest inertia and at 48.8 Hz, under the lowest frequency of the card",
        None,
        "frequency_hz, inertia_mw_s and demand_mw are the area's condition at the start and are held through the run; loss_mw is the one loss of the run: stepped in at time zero and held. Every segment carries the loss of the first, and a segment with another loss refuses the whole run before its first step as LossChangedDuringRun, with nothing sealed: a second loss, a link that comes back and a quiet period before the trip are other events, a loss of zero is outside the card, no changing loss was compared with a measurement, and the response the card holds was fitted to single trips. The card's own step refuses the same change once the first step has landed. A value outside its published span in any segment refuses the whole run before its first step. A run starts at time zero with the area in balance at frequency_hz and no response delivered. A single step starts in the same way at each call, so single steps cannot be chained: a run is the way to follow the fall. The card ends 60 s after the start: the step that would pass it is refused as TimeBeyondCard and the run is sealed up to the last step that landed. The clock is a running sum, t_s += dt_s in 64-bit arithmetic, and each addition rounds by up to 3.55e-15 s, half the spacing of 64-bit numbers between 32 s and 64 s. The comparison with 60 s therefore carries a named allowance, TIME_END_ALLOWANCE_PER_STEP_S: 3.55e-15 s for each step a run of this step takes to reach 60 s, and two more for the step read from a decimal, and never more than half the step. It is 1.0e-11 s at the step ceiling, 2.1e-10 s at a step of 1 ms and 2.1e-7 s at the step floor. A run of exactly 60 s in decimal lands its last step. The clock is stepped at its own value and is not pulled back, so the last t_s of such a run can read up to the allowance past 60 s. A frequency under 49.55 Hz, incoming or landing, is refused as FrequencyBelowCompared, and a run is sealed up to the last step that landed. 49.55 Hz is the lowest frequency compared with measurement: the lowest measured single-loss frequency among the 41 trips, 49.553 Hz, rounded outward. The lowest among the 36 trips the scan lost is 49.596 Hz, and the lowest the card reached in any compared run is 49.563 Hz. With the largest loss on the smallest inertia and the smallest demand from the lowest start the card would fall to 49.46 Hz; no trip combined those, and below the knee the response the grid held changed over the years, so such a run is refused on the step that would land under 49.55 Hz. Through engine 0.6.0 a landed frequency under 49.5 Hz set FLAG_BELOW_STATUTORY_LIMIT and the run went on; the card has no flag now and the frame's flags are zero. A frequency at or below 48.8 Hz, where low-frequency demand disconnection begins, is refused as LowFrequencyDemandDisconnection; that refusal and FrequencyAboveCard are named for the kernel and are not reached by a run inside the published spans. The state carries lowest_frequency_hz and lowest_time_s, the lowest landed sample; the card does not interpolate. The measured frequency is one value a second, and a loss falls between two samples: the protocol reads the card at half a second, one and a half seconds and so on after the loss begins. The response is one fitted function of the frequency for nine years of days: the response the operator held on a given day is not an input. A step of zero or less is refused as NonPositiveDt".to_string(),
        &[
            "NonFiniteInput",
            "NonFiniteState",
            "NonPositiveDt",
            "InvalidCard",
            "LossOutsideCard",
            "LossChangedDuringRun",
            "StartFrequencyOutsideCard",
            "InertiaOutsideCard",
            "DemandOutsideCard",
            "TimeBeyondCard",
            "LowFrequencyDemandDisconnection",
            "FrequencyBelowCompared",
            "FrequencyAboveCard",
            "SpontaneousGeneration",
            "FirstLawBreach",
        ],
        vec![],
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
                "Always zero. The card has no advisory condition: a loss outside 693 to 1402 MW and a frequency under 49.55 Hz are refusals.",
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
    // Only the blocks the card serves: the comparison showed the others wrong.
    let served: Vec<usize> = (0..cell_2170_m80::BLOCK_COUNT)
        .filter(|block| !specimen.block_shown_wrong[*block])
        .collect();
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
        "One Tesla Model 3 2170 lithium-ion cell, cell m80 alone, from 0 to 40 degC, under a terminal current and a measured case temperature.",
        vec![
            scalar(
                "block_chamber_c",
                Role::Phase,
                "degC",
                Some(coldest_block),
                false,
                Some(warmest_block),
                false,
                "exactly one of 40, 25, 10 and 0 degC: the chamber setting that names the block of tests the run belongs to. It picks the capacity, the rest-voltage table, the range of charge and the band of case temperatures, and in this dataset it is also the cell's age. -10 and -20 degC name the two blocks the comparison with measurement showed wrong, where no held-back test met its marks, and are refused as BlockShownWrong. Any other value is refused as BlockNotRecognized; nothing is rounded to a block".into(),
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
                    "ampere-hours removed since the recharge: 0 for a cell that has just been recharged. Each block has its own inclusive range, where its tables end: {ranges}. The published minimum and maximum are the widest of the four; a start outside the block's own range is refused as ChargeOutsideTables. One exception, for rounding: a start past an end of the block's tables by no more than 2^-49 Ah, the rounding allowance of a start, is taken and read at the end point, so a run that landed its last step a spacing or two past an end can be started again from the charge it returned. A run that ended further past an end by rounding returns a charge from which no new run starts; start it from the end itself"
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
                    "inclusive [-11.375, 9.737] A. A negative current discharges the cell and a positive current charges it, as in the measurements. The bounds are the largest currents in the held-back tests the card was compared with: 11.374 A discharging, in a drive cycle at 0 degC, and 9.737 A charging, in a drive cycle at 40 degC, each rounded outward to the milliampere. Each block takes only the currents of its own held-back tests, from the largest discharging to the largest charging sample, rounded outward to the milliampere: {currents}. A current outside the card's bounds is refused as CurrentOutsideCard, and one inside them but outside the block's currents as CurrentOutsideBlock"
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
                    "inclusive [0.808, 45.535] degC: the temperature measured on the cell's case, at the middle. The card is given it and does not predict it. Each block takes only the case temperatures at which its own held-back tests were run, from the coldest to the warmest sample, rounded outward to the millikelvin: {bands}. A case temperature outside the card's bounds is refused as CaseTemperatureOutsideCard, and one inside them but outside the block's band as CaseTemperatureOutsideBlock"
                ),
            ),
        ],
        floor,
        "a locked 1 s: the step is exactly one second, the sampling of the measurements the card was fitted to",
        gate,
        "the same locked 1 s: the first-second resistance has no meaning at another step, and the card refuses any other as StepNotOneSecond",
        None,
        "Over a step the current and the case temperature are held, and every table is read at the charge the step lands on. A current or a case temperature outside its range in any segment of a run refuses the whole run before its first step. The range is the range compared with measurement: the four blocks from 0 to 40 degC, each at the case temperatures and the currents of its own held-back tests. The current is bounded in size only: the card does not track how long a current is held. The held currents compared were a third of the rated current, about 1.5 A, and the rated current, about 4.5 A; the drive cycles reach the larger currents only for seconds at a time, and the held discharge at the rated current missed its voltage mark at every temperature. A run starts at rest: charge_removed_ah as given, nothing held by the relaxing branch, and the ledger anchored there. The first measured sample of a record is that start and each later sample is one step. A single step starts from rest in the same way at each call, so single steps cannot be chained into a run. A run ends in a sealed refusal when the terminal voltage would fall below 2.8 V (LowerVoltageLimit), would rise above 4.242 V (UpperVoltageLimit), or when the charge removed would leave the tables of the block (ChargeOutsideTables); the last landed state is the last second on the card. The charge removed is a running sum in 64-bit arithmetic, so the comparison with either end of the tables carries a rounding allowance: 2^-50 Ah for each landed step and the step being taken, and two more for the start and the end of the table, never more than half the charge of the step; a rest, or a step back toward the tables, from a state past an end lands. A run whose end in decimal is exactly an end of the tables, at the top or the bottom, lands its last step, and every table is read at the end point there. When the charge leaves the tables on the same step on which the voltage would pass a limit, the refusal named is ChargeOutsideTables, because no voltage is read outside the tables. A discharge slower than about a tenth of the capacity per hour can leave the tables before 2.8 V. The card sets no flag. A step of zero or less is refused as NonPositiveDt before the step floor is tested.".into(),
        &[
            "NonFiniteInput",
            "NonFiniteState",
            "NonPositiveDt",
            "InvalidCell",
            "StepNotOneSecond",
            "BlockNotRecognized",
            "BlockShownWrong",
            "CaseTemperatureOutsideCard",
            "CaseTemperatureOutsideBlock",
            "CurrentOutsideCard",
            "CurrentOutsideBlock",
            "ChargeOutsideTables",
            "LowerVoltageLimit",
            "UpperVoltageLimit",
            "SpontaneousGeneration",
            "FirstLawBreach",
        ],
        vec![],
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
                "Always zero. The card has no advisory condition: a voltage limit, the end of the tables and a residual past the limit are refusals.",
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
    let (reactivity_least, _) = span_of(&reactivity_min);
    let (_, reactivity_most) = span_of(&reactivity_max);
    let (flow_least, _) = span_of(&flow_min);
    let (_, flow_most) = span_of(&flow_max);
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
                    "the coolant at the core inlet at the start, in one of three inclusive bands, the three the report tested: {inlet_bands}. The published minimum and maximum are the lowest and the highest of the three; a temperature between two bands is refused as InletTemperatureOutsideCard, and nothing is rounded to a band. The band picks the generation time, the insertion ramp, the moderator coefficient, the heat capacity of the water, and the spans of the reactivity and of the flow"
                ),
            ),
            scalar(
                "reactivity_inserted_dollar",
                Role::Load,
                "dollar",
                Some(reactivity_least),
                false,
                Some(reactivity_most),
                false,
                format!(
                    "the reactivity the transient rod inserts in all, ramped in from zero as K t^2 and then held. Each band has its own inclusive span, that of its tested reactor periods: {reactivity_bands}. The published minimum and maximum are the widest of the three; a reactivity outside the span of the run's own band is refused as ReactivityOutsideCard. A run is one burst and carries one reactivity: a segment with another is refused as ReactivityChangedDuringBurst"
                ),
            ),
            scalar(
                "coolant_flow_m_per_s",
                Role::Load,
                "m/s",
                Some(flow_least),
                false,
                Some(flow_most),
                false,
                format!(
                    "the average coolant flow rate through the core. Each band has its own inclusive span, from the lowest to the highest flow tested in that band: {flow_bands}. The cold band was tested with no flow, the 260 degF band at 2.4 to 22 ft/s and the 500 degF band at 4.8 to 24 ft/s. The published minimum and maximum are the widest of the three; a flow outside the span of the run's own band is refused as FlowOutsideCard. A run carries one flow rate, as every tested burst did: a segment with another is refused as FlowChangedDuringBurst"
                ),
            ),
        ],
        floor,
        "a locked 1e-9 s: the card's own constant, a step that still moves the clock at 20 s",
        gate,
        "0.2 of the reduced generation time of the cold band, the time constant of the prompt pole at zero net reactivity, where every run starts; the same ceiling in all three bands",
        None,
        format!(
            "reactivity_inserted_dollar and coolant_flow_m_per_s are in the span of the band inlet_temperature_k falls in. A run is one burst, with one reactivity and one flow rate: every segment carries the load of the first, and segments divide a run without changing its load. A reactivity or a flow outside that span in any segment of a run refuses the whole run before its first step, and so does a segment whose reactivity or flow is not the first segment's, as ReactivityChangedDuringBurst or FlowChangedDuringBurst, with nothing sealed. The card is one insertion, ramped in and then held, which is what the report tested: a second insertion, a withdrawal or a jump in the inserted reactivity is not on the card, and no tested burst changed its flow. The card's own step refuses the same change once the first step has landed, and keeps the state. inlet_temperature_k is in kelvin. The reactor starts at {} MW in equilibrium, with zero net reactivity and both temperature rises zero, at the moment the transient rod begins to move. The reactivity is ramped in as K t^2 up to the load and held there, with K the band's own; FLAG_ROD_MOVING is set while the inserted reactivity is still below the load. fuel_temperature_rise_k and water_temperature_rise_k are rises above the inlet temperature. A single step takes the inlet temperature and starts again from there at each call, so single steps cannot be chained. A run ends at the first maximum of power after the insertion is complete: the step after it is refused as PastFirstMaximum and the run is sealed with that refusal. Its last landed state carries peak_power_mw, peak_energy_mws and peak_time_s, the highest landed sample, which lies within one step of the true maximum; the card does not interpolate. A run that would pass {} s, {} MW s or {} MW before that maximum is refused there as TimeBeyondCard, EnergyBeyondCard or PowerBeyondCard and sealed with that refusal. A run of 1,000,000 steps at the floor stops at one millisecond, completed, with the rod still moving. NegativeState and InconsistentState are refusals of the card's step for a state handed to it that the card did not write; this door takes the inlet temperature and nothing else of a state, and starts every run and every step from the card's own start, so no request reaches them. A step of zero or less is refused as NonPositiveDt before the step floor is tested.",
            format!("{:e}", specimen.initial_power_mw),
            specimen.time_max_s,
            specimen.energy_max_mws,
            specimen.power_max_mw,
        ),
        &[
            "NonFiniteInput",
            "NonFiniteState",
            "NonPositiveDt",
            "InvalidCard",
            "InletTemperatureOutsideCard",
            "ReactivityOutsideCard",
            "FlowOutsideCard",
            "PastFirstMaximum",
            "TimeBeyondCard",
            "NegativeState",
            "EnergyBeyondCard",
            "PowerBeyondCard",
            "ReactivityChangedDuringBurst",
            "FlowChangedDuringBurst",
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
                "inclusive [0, 0] degrees: the twist of the plate against the finger at the start, which is zero. Every measured twist began with the pad untwisted, and every run and every single step starts there: a pad already twisted was brought there under a load the card cannot see, so any other start is refused as TwistNotStartedFromZero, a twist such as 1e-46 that would read as zero in a 32-bit frame slot included. The plate turned 80 degrees in every trial and slowed to a stop between 77 and 80 degrees; the torque and the stuck fraction were compared with measurement from 1 to 75 degrees, and a step of a run that would end beyond 75 degrees is refused as TwistBeyondMeasured".into(),
            ),
            scalar(
                "normal_force_n",
                Role::Load,
                "N",
                Some(fingerpad_p02_glass::NORMAL_FORCE_FLOOR_N),
                false,
                Some(fingerpad_p02_glass::NORMAL_FORCE_CEILING_N),
                false,
                format!("inclusive [1.93, 2.07] N: the force of the plate on the pad, 2 N less and plus {} N. The card holds its slip-torque mark only in the block of trials at 2 N; at 0.5, 1, 5 and 10 N it was off by +14, +9, +22 and +29 percent, and those forces are refused as NormalForceOutsideCard. The band is the spread of the robot's force controller: the root-mean-square departure from 2 N of the normal force measured in the 60 trials of this finger at 2 N and 5 to 50 degrees a second, 0.072 N over 4,427 samples (column NormalForceExp2 of the video_out files, Zenodo 7646119). Both ends are inside. Across the band the law moves the slip torque by 2.7 percent each way", fingerpad_p02_glass::NORMAL_FORCE_BAND_N),
            ),
            scalar(
                "twist_rate_deg_s",
                Role::Load,
                "deg/s",
                Some(fingerpad_p02_glass::TWIST_RATE_FLOOR_DEG_S),
                false,
                Some(fingerpad_p02_glass::TWIST_RATE_CEILING_DEG_S),
                false,
                "inclusive [5, 50] degrees a second: the rate at which the plate turns about the normal, the rates the constants were fitted at (5, 10, 20 and 50). At 100 degrees a second, the fastest measured, the card missed its mark by 13 percent, and a rate above 50 is refused as TwistRateOutsideCard. Both ends are inside. The direction of the turn is not an input: clockwise and counter-clockwise trials were pooled".into(),
            ),
        ],
        floor,
        "the ceiling divided by 2^8, a twenty-fifth of the sampling interval of the force sensors. The slowest whole twist is then at most 384,000 steps",
        gate,
        "0.01 s, one camera frame at the fastest twist measured, 100 degrees a second. At 50 degrees a second, the fastest of the card, that is half a degree, 0.05 of the characteristic twist, where the four-point rule that sums the work is exact to rounding",
        None,
        format!(
            "Over a step the normal force and the twist rate are held, and the twist grows by rate x dt_s. A bed of springs under a parabolic friction limit, twisted about its centre: with c = 2 / (u + sqrt(u^2 + 4)), the torque is T_limit x (1 - (5c^3 + 3c^5)/8) at u = twist / twist_T, and the stuck fraction is c^2 at u = twist / twist_S. T_limit = {} N m x (F / 2 N)^{} x (rate / 20)^{}; twist_T = {} deg x (F / 2 N)^{} x (rate / 20)^{}; twist_S = {} deg x (rate / 20)^{}. All eight numbers are fitted to this finger's own trials; the level, {} N m, is that of the block of trials at 2 N, and the card answers only within {} N of 2 N, where the two powers of F move the slip torque by 2.7 percent each way. A run is one twist from zero under one load: it starts at time zero with the pad untwisted and the work and the heat at zero, and a start at any other twist is refused as TwistNotStartedFromZero. A single step is the first step of such a twist, from zero, at each call: single steps cannot be chained, and a twist is followed with run_trajectory. The force and the rate cannot change during a twist: no measured twist changed either, and a later segment of a run whose load differs from the first is refused as LoadChangedDuringTwist. Because no run and no step can start part way, no request can take up a twist under another load. The card's own step refuses the same change for as long as the state lives: the state carries the force and the rate it was twisted under, held_normal_force_n and held_twist_rate_deg_s, which are answers and not inputs. A value outside its range in any segment of a run refuses the whole run before its first step. The frame flag FLAG_FULL_SLIP is an event, the one the card exists to find, and not a warning. A run ends in a sealed refusal, TwistBeyondMeasured, at the step that would end beyond 75 degrees, the end of the twist compared with measurement; a run that ends exactly on 75 degrees keeps its last step, because the step accepts a twist up to {} degrees past the end, the most the sum of the steps can round by.",
            fingerpad_p02_glass::SLIP_TORQUE_REFERENCE_N_M,
            fingerpad_p02_glass::LOAD_EXPONENT,
            fingerpad_p02_glass::RATE_EXPONENT,
            fingerpad_p02_glass::TORQUE_TWIST_REFERENCE_DEG,
            fingerpad_p02_glass::TORQUE_TWIST_LOAD_EXPONENT,
            fingerpad_p02_glass::TORQUE_TWIST_RATE_EXPONENT,
            fingerpad_p02_glass::STUCK_TWIST_REFERENCE_DEG,
            fingerpad_p02_glass::STUCK_TWIST_RATE_EXPONENT,
            fingerpad_p02_glass::SLIP_TORQUE_REFERENCE_N_M,
            fingerpad_p02_glass::NORMAL_FORCE_BAND_N,
            fingerpad_p02_glass::TWIST_END_ALLOWANCE_DEG,
        ),
        &[
            "NonFiniteInput",
            "NonFiniteState",
            "NonPositiveDt",
            "InvalidFingerPad",
            "NormalForceOutsideCard",
            "TwistRateOutsideCard",
            "LoadChangedDuringTwist",
            "TwistNotStartedFromZero",
            "TwistBeyondMeasured",
            "SpontaneousGeneration",
            "FirstLawBreach",
        ],
        vec![
            Advisory {
                name: "FLAG_FULL_SLIP",
                mask: fingerpad_p02_glass::FLAG_FULL_SLIP,
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
                "FLAG_FULL_SLIP, an event and not a warning: the model's stuck fraction is under 0.03, the threshold of the measurement, so the whole contact slides. It is the event the card exists to find. The model's torque is then 1.1 to 6.9 percent under its limit across the card's range, 2.4 percent at 2 N and 20 degrees a second, and goes on rising toward it. The card puts the event at 39 to 40 degrees of twist; on this finger at 2 N it was measured at 44 to 49 degrees. It seals. The card writes no other flag: where it does not hold it refuses.",
            ),
            ("proof", "Not written by the step."),
        ]),
    )
}

fn tokamak_jet_dte1_card() -> Card {
    let tokamak = tokamak_jet_dte1::TokamakJetDte1::elmy_h_mode();
    // The ceiling and the floor are locked constants of the card; it has no
    // frequency, so no `π/ω` is published.
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
                Some(tokamak_jet_dte1::THERMAL_ENERGY_MIN_J),
                false,
                Some(tokamak_jet_dte1::THERMAL_ENERGY_MAX_J),
                false,
                format!(
                    "inclusive [{}, {}] J: the thermal energy of the plasma at the start, the span of the card's state over the nine compared runs of the card's regime; a thermal energy outside it, at the start or after any step, is refused as EnergyOutsideComparison. Under the first load the start must also give a temperature of 0.777 to 4.30 keV (TemperatureOutsideComparison), a normalised beta of the two stores of 0.224 to 2.21 (BetaOutsideComparison) and a confinement time of 0.150 to 5.72 s (ConfinementTimeOutsideComparison), and the limits of physics are tested before them: a temperature from 0.2 keV with at most 100 keV at the centre of the profile, the range of the reactivity fit, and a normalised beta of at most 2.8, the Troyon limit (BetaAboveTroyon)",
                    token(tokamak_jet_dte1::THERMAL_ENERGY_MIN_J),
                    token(tokamak_jet_dte1::THERMAL_ENERGY_MAX_J)
                ),
            ),
            scalar(
                "fast_ion_energy_j",
                Role::Phase,
                "J",
                Some(tokamak_jet_dte1::FAST_ION_ENERGY_MIN_J),
                false,
                Some(tokamak_jet_dte1::FAST_ION_ENERGY_MAX_J),
                false,
                format!(
                    "inclusive [{}, {}] J: the energy of the beam ions, the ion-cyclotron tail and the fusion alphas at the start, the span of the card's state over the nine compared runs of the card's regime; outside it, at the start or after any step, it is refused as EnergyOutsideComparison. With the thermal energy it is held to the normalised beta of the thermal_energy_j field",
                    token(tokamak_jet_dte1::FAST_ION_ENERGY_MIN_J),
                    token(tokamak_jet_dte1::FAST_ION_ENERGY_MAX_J)
                ),
            ),
            scalar(
                "elapsed_time_s",
                Role::Phase,
                "s",
                Some(0.0),
                false,
                Some(tokamak_jet_dte1::TIME_MAX_S),
                false,
                format!(
                    "inclusive [0, {}] s: the time since the start of the heated window at the start of this request, and t_s of the state it starts from. It is required. A new pulse states 0; a run or a single step continued from a returned state states that state's t_s, so the end of the card, {} s after the start of the heated window, the longest compared window, cannot be passed by starting again. The card cannot see an earlier run, so it cannot check this number. A start at {} s is legal and its first step is refused as TimeBeyondCard",
                    token(tokamak_jet_dte1::TIME_MAX_S),
                    token(tokamak_jet_dte1::TIME_MAX_S),
                    token(tokamak_jet_dte1::TIME_MAX_S)
                ),
            ),
            scalar(
                "heating_power_w",
                Role::Load,
                "W",
                Some(tokamak_jet_dte1::HEATING_POWER_MIN_W),
                false,
                Some(tokamak_jet_dte1::HEATING_POWER_MAX_W),
                false,
                format!(
                    "inclusive [{}, {}] W: the neutral-beam power less shine-through plus the coupled ion-cyclotron power, the span of the nine compared pulses of the card's regime",
                    token(tokamak_jet_dte1::HEATING_POWER_MIN_W),
                    token(tokamak_jet_dte1::HEATING_POWER_MAX_W)
                ),
            ),
            scalar(
                "ohmic_power_w",
                Role::Load,
                "W",
                Some(tokamak_jet_dte1::OHMIC_POWER_MIN_W),
                false,
                Some(tokamak_jet_dte1::OHMIC_POWER_MAX_W),
                false,
                format!(
                    "inclusive [{}, {}] W: the ohmic power, the span of the nine compared pulses of the card's regime",
                    token(tokamak_jet_dte1::OHMIC_POWER_MIN_W),
                    token(tokamak_jet_dte1::OHMIC_POWER_MAX_W)
                ),
            ),
            scalar(
                "line_density_per_m3",
                Role::Load,
                "1/m^3",
                Some(tokamak_jet_dte1::LINE_DENSITY_MIN_PER_M3),
                false,
                Some(tokamak_jet_dte1::LINE_DENSITY_MAX_PER_M3),
                false,
                format!(
                    "from {} to {} per cubic metre, both inclusive: the line-averaged electron density, the span of the nine compared pulses of the card's regime; above it the request is refused as DensityAboveComparison. It is also held to the Greenwald density of the segment's current, 1e20 x Ip[MA] / (pi x 0.9386^2) per cubic metre, refused above it as DensityAboveGreenwald, and to 0.834 of it, the largest fraction compared, refused above that as GreenwaldFractionAboveComparison",
                    token(tokamak_jet_dte1::LINE_DENSITY_MIN_PER_M3),
                    token(tokamak_jet_dte1::LINE_DENSITY_MAX_PER_M3)
                ),
            ),
            scalar(
                "plasma_current_a",
                Role::Load,
                "A",
                Some(tokamak_jet_dte1::PLASMA_CURRENT_MIN_A),
                false,
                Some(tokamak_jet_dte1::PLASMA_CURRENT_MAX_A),
                false,
                format!(
                    "inclusive [{}, {}] A: the plasma current, as a size, the span of the nine compared pulses of the card's regime. With the toroidal field it sets the edge safety factor, 5 a^2 Bt / (R Ip[MA]) times a shape factor of 2.428: a pair under 2 is refused as SafetyFactorBelowLimit, and a pair outside 3.66 to 4.14, the span compared, as SafetyFactorOutsideComparison",
                    token(tokamak_jet_dte1::PLASMA_CURRENT_MIN_A),
                    token(tokamak_jet_dte1::PLASMA_CURRENT_MAX_A)
                ),
            ),
            scalar(
                "toroidal_field_t",
                Role::Load,
                "T",
                Some(tokamak_jet_dte1::TOROIDAL_FIELD_MIN_T),
                false,
                Some(tokamak_jet_dte1::TOROIDAL_FIELD_MAX_T),
                false,
                format!(
                    "inclusive [{}, {}] T: the toroidal field at the major radius, as a size, the span of the nine compared pulses of the card's regime. With the plasma current it must keep the edge safety factor from 3.66 to 4.14",
                    token(tokamak_jet_dte1::TOROIDAL_FIELD_MIN_T),
                    token(tokamak_jet_dte1::TOROIDAL_FIELD_MAX_T)
                ),
            ),
            scalar(
                "tritium_fraction",
                Role::Load,
                "1",
                Some(0.0),
                false,
                Some(tokamak_jet_dte1::TRITIUM_FRACTION_EVEN_MAX),
                false,
                format!(
                    "tritium atoms per fuel atom, in one of two spans, both inclusive: 0 to {}, deuterium and the trace-tritium pulse 43134, and {} to {}, the held-back pulse 42982 at 0.451 and the fitted pulse 42762 at 0.503 (42976, outside the card's regime, was at 0.501), rounded up by 0.001. Those are where the fusion power was compared. Above {} and below {} nothing was compared, and three training pulses at 0.75 to 0.93 showed the fusion power wrong, by 33 to 65 percent of its peak; a fraction there is refused as FuelMixOutsideComparison, the whole step and not the fusion power alone, since the alpha heating feeds the fast-ion store. 0 is deuterium and gives no D-T fusion power",
                    token(tokamak_jet_dte1::TRITIUM_FRACTION_TRACE_MAX),
                    token(tokamak_jet_dte1::TRITIUM_FRACTION_EVEN_MIN),
                    token(tokamak_jet_dte1::TRITIUM_FRACTION_EVEN_MAX),
                    token(tokamak_jet_dte1::TRITIUM_FRACTION_TRACE_MAX),
                    token(tokamak_jet_dte1::TRITIUM_FRACTION_EVEN_MIN)
                ),
            ),
        ],
        floor,
        "a locked 1e-6 s: at 5.475 s a step of that size still moves the clock, and at 1 MW it moves a store of 20 MJ by one joule, which 64-bit numbers resolve to nine figures",
        gate,
        "a locked 1 ms: under an eighteenth of the shortest slowing-down time inside the declared range, 18.0 ms at 0.777 keV and 7.85e19 per cubic metre, and under a fortieth of the shortest time in which the loss power moves the thermal energy, 0.31 times the confinement time, 46.5 ms at the smallest compared confinement time of 0.150 s. Over the eight training pulses at this step the largest conservation residual is 5.1e-10 against a limit of 2e-8. In 0.6.0 the ceiling rested on the slowing-down time alone, and states with a confinement time near 37 ms, outside the compared envelope, were refused as FirstLawBreach although no energy was lost; the confinement time is now held to the envelope, and FirstLawBreach and SpontaneousGeneration name a defect of the arithmetic or the integrator only",
        None,
        "The declared range is the envelope of the comparison inside the card's regime: the nine ELMy H-mode pulses of the research record, the eight fitted and the held-back 42982, run through the card at 1 ms by the protocol's rule, each bound the smallest or largest value of those runs rounded outward in the third figure. The held-back pulse 42976 is not in it: a hot-ion pulse declared outside the card's regime before it was run, it missed all four marks. Every load field is held through a segment and a later segment may change any of them: a run of a pulse is one segment for each interval of its measured traces. A value outside its published span, a density above the Greenwald density of the segment's current or above 0.834 of it, a current and field whose safety factor is under 2 or outside 3.66 to 4.14, or a tritium fraction between 0.034 and 0.451, in any segment of a run, refuses the whole run before its first step. The state is held at the start and after every step to the compared spans of the thermal energy, the fast-ion energy, the temperature, the normalised beta and the confinement time, after the limits of physics, the reactivity fit and the Troyon limit; these depend on the state, so a run that reaches one of them part way is sealed with that refusal on its last landed step. At the door the published bounds of the thermal energy and the density keep the temperature under 17 keV, so TemperatureAboveReactivityFit is a refusal of the card's own step and no request reaches it here. A run starts at elapsed_time_s with its ledger anchored at the two energies given and no fusion energy released. A single step starts in the same way at each call: its thermal_energy_j, fast_ion_energy_j and t_s handed back as a new start, t_s as elapsed_time_s, continue the same plasma on the same clock, and fusion_energy_j and the ledger begin again at zero; no limit depends on either. The state reports the temperature, the loss power, the confinement time, the slowing-down time, the fusion power and its thermal part, the normalised beta, the Greenwald fraction and the safety factor under the load of the last step; they are zero at the start. A frame carries FLAG_NEAR_DENSITY_LIMIT while the density is above 0.85 of the Greenwald density, which the compared span of 0.834 no longer reaches. A step of zero or less is refused as NonPositiveDt before the step floor is tested. The card ends 5.475 s after the start of the heated window, the longest compared window: the step that would pass it is refused as TimeBeyondCard and the run is sealed up to the last step that landed. The clock is a running sum from elapsed_time_s, t_s += dt_s in 64-bit arithmetic, and each addition rounds by up to 4.44e-16 s, half the spacing of 64-bit numbers between 4 s and 8 s. The comparison with 5.475 s therefore carries a named allowance, TIME_END_ALLOWANCE_PER_STEP_S: 4.44e-16 s for each step a run of this step takes to reach 5.475 s, and two more for the step read from a decimal, and never more than half the step: 2.4e-12 s at the step ceiling and 2.4e-9 s at the step floor. A run that ends exactly on 5.475 s in decimal lands its last step, and its last t_s can read up to the allowance past 5.475 s.".into(),
        &[
            "NonFiniteInput",
            "NonFiniteState",
            "NonPositiveDt",
            "InvalidCard",
            "HeatingPowerOutsideCard",
            "OhmicPowerOutsideCard",
            "PlasmaCurrentOutsideCard",
            "ToroidalFieldOutsideCard",
            "FuelMixOutsideCard",
            "FuelMixOutsideComparison",
            "DensityBelowCard",
            "DensityAboveComparison",
            "DensityAboveGreenwald",
            "GreenwaldFractionAboveComparison",
            "SafetyFactorBelowLimit",
            "SafetyFactorOutsideComparison",
            "EnergyOutsideCard",
            "TemperatureBelowReactivityFit",
            "TemperatureAboveReactivityFit",
            "BetaAboveTroyon",
            "EnergyOutsideComparison",
            "TemperatureOutsideComparison",
            "BetaOutsideComparison",
            "ConfinementTimeOutsideComparison",
            "TimeBeyondCard",
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
                "FLAG_NEAR_DENSITY_LIMIT: the line-averaged density is above 0.85 of the Greenwald density. It seals. The compared span of the density, at most 0.834 of the Greenwald density, does not reach it.",
            ),
            ("proof", "Not written by the step."),
        ]),
    )
}

/// The data of the JSI TRIGA reactor's validation record.
pub(crate) const REACTOR_TRIGA_JSI_DATA: &str = "The pulse database of the Jozef Stefan Institute, trigapulse.ijs.si (I. Svajger and L. Snoj): a table of 757 pulses from 1991 to 2023 and, for each pulse, a record of power and two fuel thermocouples at 20,000 samples a second. The site states no licence and no terms. The constants taken from publications are from Svajger et al., Nuclear Engineering and Technology 56 (2024) 1165, an open-access article under the CC BY-NC-ND licence, and the hot-rod factor is from I. Vavtar, master thesis, University of Ljubljana, 2019, which states no licence. None of the site's measured values is stored in the card. 44 records are on disk, 43 of them of cores 215, 217 and 231. All 43 were opened and scored under the card's first protocol and by its second scoring, so none is unseen. The ten fitted constants were taken from 36 of them: 1.4 to 3.0 printed dollars, a peak at least ten times the noise, a scram found, a printed value that follows the rod table. Held back, by rule: the table's peak power, energy within 50 ms of the peak and thermocouple maximum for the 20 rows of the same cores whose records were never fetched, 12 of 22 and 28 March 2018 and 8 repeats of 24 March 2015. Their peak powers had been printed beside the first card before this protocol; their energies and thermocouple maxima had not been read.";

/// The pass line of the JSI TRIGA reactor's validation record.
pub(crate) const REACTOR_TRIGA_JSI_PASS_LINE: &str = "Protocol evidence/research/reactor_triga_jsi/validation_protocol_v2.md, fixed before anything was fitted (SHA-256 of its machine-readable copy 9a0720f1822161c3b53cbc351d862ec4c077da51521ee1a96d7ceb85d8275ddc), with two amendments. The uncertainty of a comparison: the repeat scatter of one pulse within a day (peak power 1.3 percent, width 0.5 percent, energy within 50 ms of the peak 0.7 percent, energy to the scram 0.8 percent, thermocouple maximum 0.9 K) and the change for 0.03 printed dollars, the shift of the day's reference; for a row without a record also the start temperature and the scram instant, which are set by rule. On the opened pulses, in four bins of reactivity: root mean square of the error in units of the uncertainty at most 1.5 for peak power, for width and for energy to the scram; the card's range is the unbroken run of bins that hold, and outside it the card refuses. On the rows held back, for each group and each of peak power, energy within 50 ms of the peak and thermocouple maximum: root mean square at most 1.5, worst at most 3, mean at most 2 over the square root of the number of days. A straight line in printed reactivity, the nearest opened pulse, the Fuchs-Nordheim relations with the institute's constants, the unfitted model and the first card are scored beside the card; a mark that the straight line or the nearest pulse also meets is not evidence for the model.";

/// The one-line summary of the JSI TRIGA reactor, as the list shows it.
pub(crate) const REACTOR_TRIGA_JSI_SUMMARY: &str = "The TRIGA Mark II reactor of the Jozef Stefan Institute, Ljubljana, with its 56-element core of standard fuel, in pulse mode from 2.0 to 3.0 printed dollars on the rod-worth tables of cores 215, 217 and 231: power, energy released and core-average fuel temperature from the fire signal through the pulse to the scram. Ten constants are fitted to 36 pulses of this reactor, in a form added after the twelve fixed beforehand had failed; by the rule fixed beforehand the comparison with the 20 rows held back failed, four of six lines missed, with peak power 23 and 44 percent low on a day the card was not fitted to; the fuel temperature was compared only through one thermocouple of one element. After that scoring the lower limit was raised from 1.4 to 2.0 dollars, where the held-back rows had read up to 29 percent low: below 2.0 dollars the card refuses by name.";

/// The result of the JSI TRIGA reactor's validation record.
pub(crate) const REACTOR_TRIGA_JSI_RESULT: &str = "On the opened pulses: the twelve forms fixed beforehand all read the power after the pulse too low, and by the rule fixed beforehand none held in any bin of reactivity. A thirteenth form, with the generation time fitted, was added after that result was seen (amendment 2) and holds in all four bins, 1.4 to 3.0 dollars, on the 36 pulses it was fitted to: peak power 4.1 percent root mean square (0.49 in units of the uncertainty), width 2.6 percent (0.80), energy to the scram 2.3 percent (0.96), each after the pulse channel's factor for that core's sessions. The thermocouple maximum, through the hot-rod factor 1.387 and a delay fitted to each recording chain: 4.9 K root mean square on 29 pulses, 1.8 percent of the rise, worst 8.6 K low. All of that is in sample. On the rows held back, run once on 2026-10-09, by the rule fixed beforehand the comparison failed: two of the six lines were met and four missed, and the two that were met are also met by the nearest opened pulse taken unchanged, so they are not evidence for the model. Core 231 on 22 and 28 March 2018, twelve rows on a fitted table and on days that were not fitted: all three quantities missed, each on its worst row. Peak power read 10.9 percent low on average, 18.0 percent root mean square, worst 44 percent low; in units of the uncertainty 1.21 root mean square, 3.02 worst against a line of 3, mean -0.74. Energy within 50 ms of the peak: 10.2 percent low, 15.7 percent root mean square; 1.47, 3.75 and -1.11. Thermocouple maximum, eleven rows: 2.6 K low on average, 8.0 K root mean square, worst 20.3 K low; 1.29, 3.10 and -0.39. The two rows of 22 March read 44 and 23 percent low in peak power: that day's printed 1.5 and 2.0 dollars sit at rod positions where the fitted day's printing gives 1.75 and 2.25, and the measured pulses lie between the two readings. On 28 March the six rows at 2.0 dollars and above read within 3.7 percent in peak power, 2.7 percent in energy and 5.8 K; the four below 2.0 dollars read from 29 percent low to level. Core 217 on 24 March 2015, eight repeats at 2.5 dollars between two fitted pulses: peak power met (5.6 percent high; 1.09, 1.33 and +1.07 against a mean line of 2), thermocouple maximum met (2.1 K low; 0.42 root mean square), energy within 50 ms missed (5.4 percent high; 2.01 against a line of 1.5). Beside the card on the twelve rows of 2018: the first card met the thermocouple maximum (0.97 root mean square, 1.59 worst) where this card missed, and was worse on peak power and energy (2.54 and 4.18 root mean square); a straight line in printed reactivity did as well as this card on energy and on the thermocouple maximum (1.45 and 1.33 root mean square, each missed on its worst row); the nearest opened pulse, the Fuchs-Nordheim relations and the unfitted model missed every line. One row, at 1.2 dollars, was refused as ReactivityOutsideCard. Every number is in evidence/research/reactor_triga_jsi/analysis_v2/RESULTS_v2.md. The first protocol's result, on the first card: four of eight lines met and four missed. Its second scoring (evidence/checks/second_party/V-37) confirmed the count and found that card's energy to the scram 7 to 28 percent under the record's, its thermocouple maximum right only because a fitted ratio of 1.885 had absorbed the deficit, its four met lines met only with 5 percent of the reactivity charged to each pulse, and a scram after the rod had stopped refused under a false name. This card was rebuilt for those findings. Second scoring of this card under its second protocol, 2026-10-09 (evidence/checks/second_party/V-43): the count stands, two of six. What it adds. Of the six held-back lines three were on numbers nobody had read, the energy within 50 ms and the thermocouple maximum of the 2018 rows and the energy of the 2015 repeats, and all three were missed; both lines that were met are on numbers printed or bracketed before, and both are also met by the nearest opened pulse. The lines met depend on the 0.03 dollars charged for the day's reference: on the rows held back one of six with none or with 0.015 dollars, two with 0.03 and six with 0.06; in sample nine of the twelve bin lines with 0.015 dollars, which would have declared a range of 2.25 to 2.75 dollars only, and twelve with 0.03. The energy within 50 ms of the peak is 0.99 correlated with peak power on the 2018 rows and is not the energy to the scram; no held-back column tests the energy to the scram. After the held-back rows were scored the card's lower limit of reactivity was raised from 1.4 to 2.0 printed dollars, and a request below 2.0 is refused as ReactivityOutsideCard. The reason is in the rows themselves: of the four below 2.0 dollars, all of 28 March 2018, the card read 29, 21 and 9 percent low in peak power at 1.4, 1.6 and 1.8 dollars and level on the repeat at 1.8, and on that day's six rows from 2.0 dollars up it was within 3.7 percent in peak power, 2.7 percent in energy and 5.8 K. This floor was chosen with the held-back rows seen: it is not a blind result, it changes no line of the registered result above, and no unseen row tests it. The ten constants are unchanged and are still the fit to the 36 pulses from 1.4 to 3.0 dollars. The second scoring confirmed the floor as the right refusal: below 2.0 dollars the card is ill-conditioned and not biased, since 0.03 printed dollars moves the peak power by 44 percent at 1.4 dollars, 21 at 1.6, 13 at 1.8 and 9 at 2.0. With the limit at 2.0 dollars, worked after the fact: in sample 27 pulses at 3.3 percent root mean square in peak power, 1.6 percent in energy to the scram and 4.7 K; held back, seven rows of 2018, with peak power and energy still missed on the row of 22 March at 2.0 dollars, 23 percent low, and the thermocouple maximum met. Support the card was not fitted to exists from 2.0 to 2.6 dollars on one day and at 2.5 dollars on core 217; nothing unfitted lies above 2.6 dollars.";

/// The limits of the JSI TRIGA reactor's validation record.
pub(crate) const REACTOR_TRIGA_JSI_LIMITS: &str = "Ten constants are fitted in the card: for each of the three rod-worth tables a scale and an offset on the printed dollars (scale 0.6921, 0.6677 and 0.7191, offset -0.1204, -0.1366 and -0.04949 dollars, for cores 215, 217 and 231); the feedback coefficient, 6.241e-5 per K at 294 K, and its rise, 8.718e-8 per K2; the generation time, 29.59 microseconds, where the article gives 40 with 8 percent; and the heat flow from fuel to water, 890.6 W/K. More are fitted outside the card, to instruments: the pulse channel's factor for each core's sessions (0.9838, 0.9743 and 1.043; their geometric mean is 1 by convention, so two are free), the thermocouple's delay in the recording chain of 2014 and 2015 (1.336 s), and two delays in series in the chain of 2018 (1.152 s each): five free numbers outside the card and fifteen fitted in all. For a session whose channel factor is not fitted the floor on power or energy is about 4 percent at one sigma, and 7 to 8 percent between two sessions (pulses 219 and 243). Sourced and not fitted: the delayed fraction and the six groups, the fuel mass, the heat capacity, the rod's rate, and the hot-rod factor 1.387. What is not determined: a pulse shows only products. A generation time of 29.59 microseconds with a printed dollar worth 0.69, and a feedback that falls back by a fifth with a time constant of 1.1 s after the pulse with a printed dollar worth 0.92 and the article's 40 microseconds, fit the opened pulses equally (pooled error 3.04 and 3.06 percent); the rule took the one with fewer constants. No constant of the card is therefore the reactor's own value of that quantity. What the pulses do determine: the inverse period for each printed dollar, 158 to 170 per second, 3 to 10 percent under the published 175, and prompt criticality at 1.27 to 1.32 printed dollars. The fitted generation time of 29.59 microseconds stands for the delayed fraction over the generation time, 237 per second against the published 175, and is not the reactor's generation time. The card cannot see the day. A printed dollar belongs to a day's reference as well as to a table; the validation charged 0.03 dollars for it, from two pairs of days, and 22 March 2018 needed about three times that. 0.03 printed dollars moves the peak power by 44 percent at 1.4 dollars, 31 at 1.5, 21 at 1.6, 13 at 1.8 and 9 at 2.0 (core 231), so below 2.0 dollars the uncertainty is as wide as the answer and a comparison passed inside it says little; the card refuses below 2.0 dollars. From 2.0 dollars up the day is still unseen: the two rows of 22 March 2018, printed 1.5 and 2.0 dollars, read 44 and 23 percent low, and a caller's printed 2.0 dollars of such a day is answered as the fitted day's. The power, and with it the energy and the rise of the fuel temperature, rests on the institute's pulse channel, whose calibration differed by 7.7 percent between November 2014 and March 2015 on two pulses of the same width and the same thermocouple reading; the card's power is the mean of three calibrations. The fuel temperature is the core average. It was compared with one thermocouple at the centre of one element, 6753 at position B1, with the instrument's response in between: the hot-rod factor is a calculated ratio of element powers (1.387 with 0.003, Table 4 of Appendix B of the thesis), the factors along the element and inside it at the thermocouple are not sourced and were taken as 1, and the delay is fitted. The traces are consistent with 1.387 and do not confirm it: with the factor left free they prefer 1.585 with a heat flow of 2930 W/K, and any factor from 1.39 to 1.73 fits as well or better. The card's heat flow, 890.6 W/K, is conditional on 1.387, and the thermocouple constrains the core-average rise only to roughly 5 percent above and 20 percent below. That comparison tests the product of energy, heat capacity and those factors seconds after the pulse. It does not test the temperature during the pulse, in which the thermocouple shows under 4 percent of its rise, nor the temperature of the hottest fuel. The dependence on the starting temperature was not tested apart. At 2.0 printed dollars and above the opened pulses started at 20.9 to 29.2 degC, core 215 from 24.0 to 27.9, core 217 from 21.3 to 29.2 and core 231 only from 20.9 to 22.8; the card refuses, as StartTemperatureOutsideCard, a start outside the starts compared on its own table: 24.00 to 27.94 degC on core 215, 21.31 to 29.23 on core 217 and 20.90 to 22.81 on core 231. Every held-back row at 2.0 printed dollars and above started inside its own table's span. On those pulses the rods dropped 1.51 to 7.45 s after the peak; the card refuses a scram earlier than 1.50 s after its peak as ScramBeforeCompared and rods held later than 7.45 s after it as RodsHeldBeyondCompared. Both limits were set after the second protocol's scoring, from the envelope of what was compared; they change no line of the registered result. The rows held back test other days on fitted tables, an interpolation in reactivity; the start temperature and the scram instant of each were set by rule and not measured, and those at 2.0 printed dollars and above lie inside both envelopes. The scram's worth, 8 dollars, and the rod's rate, from the table of core 253, are not fitted and not compared. Nothing tests another core loading, a second pulse on warm fuel, or a rod-worth table other than the three; the card refuses those it can see. The two second scorings, V-37 of the first card and V-43 of this one, were made by a second agent with programs of its own on the same machine; neither repeated the measurements. Not repeated on a second machine or by a second organisation.";

/// The one line of the C-core reactor in `list_cards`.
pub(crate) const REACTOR_SPERT3_CCORE_SPECIMEN: &str = "The SPERT III C-core test reactor (stainless-steel-clad, highly enriched plates) in step transients from room temperature at 0 to 2500 psig with no flow, or 200 to 20,000 gpm at 230 psig only, through the first maximum of power to the first maximum of the plate surface temperature at the hot spot; two constants are fitted, and by the rule fixed beforehand the comparison with its 39 held-back transients failed on all three lines.";

/// What the C-core reactor card is.
pub(crate) const REACTOR_SPERT3_CCORE_WHAT: &str = "The SPERT III reactor at the National Reactor Testing Station, Idaho, with the C-19/52 core: 93 percent enriched UO2-stainless-steel plates clad in stainless steel, 16.8 g of U-235 a plate, 30.6 kg in all, 36 in active length. Step insertions of reactivity from room temperature (21 to 36 degrees Celsius) at system pressures of 0 to 2500 psig with no forced flow, and at 230 psig with 200 to 20,000 gpm. Point kinetics with six delayed groups, and four lumped temperatures: the core-average plate and water, and a plate and water at the hot spot where the thermocouples sat, with the power density multiplied by the measured flux peaking 2.04 x 1.30. The temperature the card answers for is the plate SURFACE temperature at the hot spot, not a fuel temperature. A run ends at the first maximum of that temperature after the first maximum of power.";

/// The data behind the C-core reactor card.
pub(crate) const REACTOR_SPERT3_CCORE_DATA: &str = "IDO-16918, C. R. Toole, The Effects of Pressure and Flow on Room Temperature Power Excursions in Spert III, Phillips Petroleum Co. for the US Atomic Energy Commission, June 1964, OSTI 4042264: its appendix tabulates 126 step transients with pressure, initial temperature, flow, period, peak power, energy to peak and the maximum plate surface temperature read by thermocouples spot-welded to a plate at the flux peak of a central assembly. Core constants from IDO-17138 (Condit, Scott and Johnson, 1967), Table B-1. 15 rows were excluded before any run (11 with an abnormal thermocouple reading, two likely misprints, two with an initial temperature missing or in doubt); every third remaining row and the whole 1000 psig series were held back: 72 rows fitted, 39 held back.";

/// The pass lines of the C-core reactor card.
pub(crate) const REACTOR_SPERT3_CCORE_PASS_LINE: &str = "Fixed before any constant was fitted and before any model was run on any row (evidence/research/reactor_spert3_ccore/validation_protocol.md, SHA-256 8e2e56cf3e93099f19c6a17537acb76708dc99da682915ea602d84005cbae72f, with amendment 1). On the held-back rows, for each of peak power, energy to peak and the maximum plate surface temperature rise: exp(root mean square of ln(card / measured)) - 1 at most 20 percent, and below each of three baselines fitted on the same rows: Fuchs-Nordheim on the measured reciprocal period, the nearest fitted row, and a straight line in the logarithm of the reciprocal period. The whole table, held-back rows included, had been read to check its transcription before the protocol was fixed.";

/// The result of the C-core reactor card's comparison.
pub(crate) const REACTOR_SPERT3_CCORE_RESULT: &str = "Compared once with the 39 held-back transients on 2026-10-11, and by the rule fixed beforehand the comparison failed on all three lines. Peak power: 33.3 percent (38 rows), against Fuchs-Nordheim 176 percent, the nearest fitted row 16.7 percent and the straight line 70.4 percent. Energy to peak: 56.7 percent, against 204, 17.2 and 61.6. Maximum plate surface temperature rise: 71.0 percent (36 rows), against 181, 18.1 and 36.8. The card beats Fuchs-Nordheim and the straight line on power and energy and loses to the nearest fitted row on every line. On the held-back 1000 psig series: peak power 13.5 percent, energy 11.7 percent, temperature rise 67.9 percent, low.";

/// The limits of the C-core reactor card.
pub(crate) const REACTOR_SPERT3_CCORE_LIMITS: &str = "A miss, and it stays on the record. Two constants are fitted on 72 rows: the plate-to-coolant conductance with no flow, 6403.73 W/m^2 K (Dittus-Boelter adds the flow), and the share of fission energy deposited directly in the water, 0.551915. This is not the physical share, which is a few percent from gamma rays and neutron slowing: it is an effective value that absorbs the boiling and void feedback the card leaves out, and it should not be read as a property of the core. The card has no boiling, no void and no pressure term: pressure is a start field held to the compared span, so the card answers the same transient at every pressure, where the measured energy to peak doubles from 0 to 2500 psig. Where the hot-spot plate temperature has no maximum before 200 s (long periods, and flow), the run is refused there and the rise it reached is what was compared; those 11 held-back rows read 2.2 times high on average. The power and energy rows at flow and long period whose measured power still rose at the scram are on no line. The delayed groups are Keepin's for U-235, not measured for this core. The ledger names a wrong method; it does not show the physics matches the reactor.";

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
                Some(reactor_spert3_ccore::INITIAL_TEMPERATURE_MIN_K),
                false,
                Some(reactor_spert3_ccore::INITIAL_TEMPERATURE_MAX_K),
                false,
                "the temperature of plates and water at the step, inclusive, the span of the compared transients: 21 to 36 degC. Outside it the run is refused as InitialTemperatureOutsideCard".to_string(),
            ),
            scalar(
                "system_pressure_psig",
                Role::Phase,
                "psig",
                Some(0.0),
                false,
                Some(reactor_spert3_ccore::PRESSURE_MAX_PSIG),
                false,
                format!(
                    "the system pressure in pounds per square inch above the atmosphere, inclusive, the span of the compared transients: 0 to {} psig with no flow, and exactly {} psig with flow (a flow at any other pressure is refused as FlowOutsideCard). Outside it the run is refused as PressureOutsideCard. No equation of the card reads it: the card answers the same transient at every pressure, and the comparison measured what that costs",
                    reactor_spert3_ccore::PRESSURE_MAX_PSIG,
                    reactor_spert3_ccore::FLOW_PRESSURE_MIN_PSIG,
                ),
            ),
            scalar(
                "reactivity_inserted_dollar",
                Role::Load,
                "dollar",
                Some(reactor_spert3_ccore::REACTIVITY_MIN_NO_FLOW_DOLLAR),
                false,
                Some(reactor_spert3_ccore::REACTIVITY_MAX_DOLLAR),
                false,
                format!(
                    "the reactivity stepped in at time zero, inclusive: with no flow [{}, {}] dollars, with flow [{}, {}] dollars, the inhour values (with the card's generation time and groups, summed in group order) of the longest and the shortest compared period, 9.3 s or 516 ms to 9.7 ms. The bounds are the exact doubles shown; an inhour value summed in another order can differ in the last place and fall outside. Outside the span of the run's flow it is refused as ReactivityOutsideCard. A run carries one reactivity: a segment with another is refused as ReactivityChangedDuringRun",
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
                Some(reactor_spert3_ccore::FLOW_MAX_M_PER_S),
                false,
                format!(
                    "the coolant velocity through the core, 18 ft/s at 20,000 gpm: exactly 0, or [{}, {}] m/s (200 to 20,000 gpm) at a system pressure of exactly {} psig, the one pressure at which flow was compared. Any other flow is refused as FlowOutsideCard. A run carries one flow: a segment with another is refused as FlowChangedDuringRun",
                    reactor_spert3_ccore::FLOW_MIN_M_PER_S,
                    reactor_spert3_ccore::FLOW_MAX_M_PER_S,
                    reactor_spert3_ccore::FLOW_PRESSURE_MIN_PSIG,
                ),
            ),
        ],
        floor,
        "a locked 1e-9 s: the card's own constant, a step that still moves the clock at 200 s",
        gate,
        "0.2 of the reduced generation time, 3 ms, the time constant of the prompt pole at zero net reactivity, where every run starts",
        None,
        format!(
            "A run is one transient with one reactivity and one flow: every segment carries the load of the first. The reactor starts at {} MW in equilibrium with every temperature rise zero, and the reactivity is a step at time zero. hot_spot_plate_rise_k is the rise of the plate surface temperature at the hot spot above the initial temperature, the quantity the thermocouples measured; plate_rise_k and water_rise_k are the core averages. The run passes the first maximum of power (FLAG_PAST_POWER_PEAK is set from that step on; peak_power_mw, peak_energy_mws and peak_time_s hold the highest landed sample before it, within one step of the true maximum; the card does not interpolate) and ends at the first maximum of the hot-spot plate temperature after it: the step after is refused as PastPlateTemperatureMaximum and the run is sealed with that refusal, the outcome ended. plate_rise_max_k holds the highest landed hot-spot rise. A run that would pass {} s, {} MW s or {} MW first is refused there as TimeBeyondCard, EnergyBeyondCard or PowerBeyondCard; at long periods and with flow the hot-spot temperature can still be rising at {} s, and the run is refused there. A single step starts again from the card's start at each call, so single steps cannot be chained. NegativeState and InconsistentState are refusals of the card's step for a state handed to it that the card did not write; this door takes the initial temperature and the pressure and nothing else of a state, so no request reaches them. A step of zero or less is refused as NonPositiveDt before the step floor is tested.",
            format!("{:e}", reactor_spert3_ccore::INITIAL_POWER_MW),
            reactor_spert3_ccore::TIME_MAX_S,
            reactor_spert3_ccore::ENERGY_MAX_MWS,
            reactor_spert3_ccore::POWER_MAX_MW,
            reactor_spert3_ccore::TIME_MAX_S,
        ),
        &[
            "NonFiniteInput",
            "NonFiniteState",
            "NonPositiveDt",
            "InvalidCard",
            "InitialTemperatureOutsideCard",
            "PressureOutsideCard",
            "FlowOutsideCard",
            "ReactivityOutsideCard",
            "PastPlateTemperatureMaximum",
            "TimeBeyondCard",
            "NegativeState",
            "EnergyBeyondCard",
            "PowerBeyondCard",
            "ReactivityChangedDuringRun",
            "FlowChangedDuringRun",
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
                Some(specimen.start_temperature_min_k),
                false,
                Some(specimen.start_temperature_max_k),
                false,
                format!(
                    "the fuel temperature at the fire signal, which is also the water temperature, inclusive. Each rod table accepts only the starts compared on it with a record at 2.0 printed dollars and above, the first thermocouple's reading at the fire signal on its opened pulses, rounded outward to the hundredth of a kelvin: {}. Core 215 was compared from 24.0029 to 27.9375 degC (pulses 216 and 206), core 217 from 21.3171 to 29.2207 (222 and 240) and core 231 only from 20.9023 to 22.8024 (331 and 340). The minimum and maximum published here, 294.05 to 302.38 K (20.90 to 29.23 degC), are the union of the three; a start inside them but outside its own table's span was never compared on that table and is refused as StartTemperatureOutsideCard, as is a start outside them. The pulses that started at 20.3 and 32.4 degC were at 1.5 printed dollars, below the card",
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
                Some(specimen.reactivity_min_dollar),
                false,
                Some(specimen.reactivity_max_dollar),
                false,
                "the reactivity the transient rod inserts in all, as the institute's pulse table prints it, in its dollars of 700 pcm; ramped in from zero over the printed value divided by 52 dollars per second, and then held. Inclusive. The caller sends the printed value; the card applies the scale and the offset of the table named by rod_table_core. The day's reference moves a printed dollar by about 0.03 dollars between days on one table. Outside the span the card refuses as ReactivityOutsideCard. It cannot change during a run: a later segment with another value is refused before the first step as ReactivityChangedDuringRun".to_string(),
            ),
            scalar(
                "rods_dropped",
                Role::Load,
                "1",
                Some(0.0),
                false,
                Some(1.0),
                false,
                "0 while the rods are held, 1 from the segment in which they are dropped (the scram, 8 dollars removed at once from the first step of that segment; the 8 dollars is an assumption). The card does not decide when the rods drop: the caller does. In the records the rods dropped when the thermocouple read about 260 degC (2014 and 2015) or about 290 degC (2018), or about 7.3 s after the peak of a weaker pulse; the validation took the interval from each record and dropped the rods that long after the card's own peak. On the 27 opened pulses compared at 2.0 printed dollars and above that interval ran from 1.510 to 7.446 s (pulses 219 and 330); no scram earlier or later was compared. The card holds the scram to that envelope, rounded outward to the hundredth of a second, counted from peak_time_s: a scram from a step that starts less than 1.50 s after the peak is refused as ScramBeforeCompared, and while the power still rises the peak is the present step, so a scram before the pulse is refused so too; a step with the rods held that would end more than 7.45 s after the peak is refused as RodsHeldBeyondCompared. Any other value is refused as ScramValueOutsideCard; 1 while the transient rod is still moving as ScramBeforeRodStopped; 0 after 1 as RodsRaisedAfterScram".to_string(),
            ),
        ],
        floor,
        "a locked 1e-9 s: the card's own constant, a step that still moves the clock at 14 s",
        gate,
        "0.2 of the time constant of the prompt pole after the scram, the fitted generation time over the delayed fraction times nine; the same ceiling before the scram",
        None,
        format!(
            "start_temperature_k is in kelvin. The reactor starts at {} MW at zero net reactivity, with the fuel at the starting temperature, at the fire signal, and each precursor sum at its equilibrium with that power. The start is not stationary: the six printed group fractions sum to 0.00699891 and the power equation uses the delayed fraction 0.00700, so at zero net reactivity the power falls at 0.037 per second, as if 1.6e-4 dollar were taken out. FLAG_ROD_MOVING is set while the inserted reactivity is still below the load; the rod stops at reactivity_inserted_dollar / 52 seconds. A run is one pulse: its first segment holds rods_dropped 0, and a later segment with the same reactivity_inserted_dollar and rods_dropped 1 is the scram, which lands at every step from the floor to the ceiling from 1.50 to 7.45 s after the peak; the rods may be held until 7.45 s after the peak and no longer. A run is capped at 1,000,000 steps and the earliest scram the card accepts is 1.6 to 1.7 s after the fire signal, so a run reaches its scram only at steps of about 1.6e-6 to 1.7e-6 s and above. fuel_temperature_k is the core-average fuel temperature: the energy released, less the heat that has left to the water, over the heat capacity of the fuel. It is not the temperature of any one place in the fuel, and there is no thermocouple on the card. peak_fuel_temperature_k is its highest landed sample. energy_at_scram_mws is the energy released at the instant the rods dropped, and is 0 until then. peak_power_mw, peak_energy_mws and peak_time_s are the highest landed sample of the power, which lies within one step of the true maximum; the card does not interpolate. power_mw is the mean of three calibrations of the institute's pulse channel, one for each core's sessions; they differ by up to 7 percent. A single step takes the starting temperature and the rod table and starts again from there at each call, so single steps cannot be chained. A run that would pass {} s is refused there as TimeBeyondCard; a run whose steps make exactly {} s keeps its last step. A run that would pass {} MW s, {} MW or a core-average fuel temperature of {} K is refused there as EnergyBeyondCard, PowerBeyondCard or FuelTemperatureBeyondCard and sealed with that refusal: those are past the largest values compared with a record. A pulse of 3.0 dollars with the rods held, at the ends of its own table's starts, reaches the fuel temperature limit and is refused as FuelTemperatureBeyondCard after 5.2 to 6.1 s on the table of core 215 and after 5.9 to 6.4 s on the table of core 231; on the table of core 217 it is refused first as RodsHeldBeyondCompared, at 7.56 s, 7.45 s after its peak. NegativeEnergy, ImpossiblePeak and StateOutsideCard are refusals of the card's step for a state handed to it that the card did not write; this door takes the starting temperature and the rod table and nothing else of a state, so no request reaches them. A step of zero or less is refused as NonPositiveDt before the step floor is tested.",
            format!("{:e}", specimen.initial_power_mw),
            specimen.time_max_s,
            specimen.time_max_s,
            specimen.energy_max_mws,
            specimen.power_max_mw,
            specimen.fuel_temperature_max_k,
        ),
        &[
            "NonFiniteInput",
            "NonFiniteState",
            "NonPositiveDt",
            "InvalidCard",
            "StartTemperatureOutsideCard",
            "RodTableOutsideCard",
            "ReactivityOutsideCard",
            "ScramValueOutsideCard",
            "ReactivityChangedDuringRun",
            "ScramBeforeRodStopped",
            "ScramBeforeCompared",
            "RodsHeldBeyondCompared",
            "RodsRaisedAfterScram",
            "NegativeEnergy",
            "ImpossiblePeak",
            "StateOutsideCard",
            "TimeBeyondCard",
            "EnergyBeyondCard",
            "PowerBeyondCard",
            "FuelTemperatureBeyondCard",
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
        "The line of sight of the James Webb Space Telescope in quiet fine guide, as its Fine Guidance Sensor reads it every 64 ms: a Kalman filter stepped through the flight centroids of one fine-guide file, with eighteen constants fitted to this observatory's own centroids of April 2022. On quiet frames of held-back visits it met its five marks. It is for quiet fine guide only: a centroid more than 5 mas from the lock point, or more than four stated standard deviations from where it was expected, is refused and ends the run, and a new run starts no earlier than 30 s later.",
        vec![
            scalar(
                "lock_x_arcsec",
                Role::Phase,
                "arcsec",
                Some(los_jwst_fine_guide::LOCK_X_MIN_ARCSEC),
                false,
                Some(los_jwst_fine_guide::LOCK_X_MAX_ARCSEC),
                false,
                format!(
                    "from {} to {} arcsec, both inclusive: the lock point on x in the sensor's ideal frame, where the guide star is held. It is not in the file and is the caller's. The bounds are the span of the lock points of the files the comparison answered, fitted and held-back together; the held-back files alone span -61.1512947 to 49.05617. The sensor's field reaches about 70 arcsec from its centre, and no lock point beyond the span was compared, so one outside it is refused as LockOutsideCard. The comparison took the median of the good centroids of the whole run, which uses centroids later than the frame being predicted. With a lock point that uses no future, the median of the first 10 s, the second scoring found the last-centroid, variance, correlation and per-file marks still met on quiet frames (0.793 and 0.821, 0.999 and 1.023, 0.071 and 0.095, 100 percent), and the lock-point mark no longer a test; with that lock point more frames lie beyond 5 mas, and the card refuses more of the file",
                    token(los_jwst_fine_guide::LOCK_X_MIN_ARCSEC),
                    token(los_jwst_fine_guide::LOCK_X_MAX_ARCSEC)
                ),
            ),
            scalar(
                "lock_y_arcsec",
                Role::Phase,
                "arcsec",
                Some(los_jwst_fine_guide::LOCK_Y_MIN_ARCSEC),
                false,
                Some(los_jwst_fine_guide::LOCK_Y_MAX_ARCSEC),
                false,
                format!(
                    "from {} to {} arcsec, both inclusive: the lock point on y in the sensor's ideal frame. The bounds are the span of the lock points of the files the comparison answered, fitted and held-back together; the held-back files alone span -62.0922546 to 50.1812668. No lock point beyond the span was compared, so one outside it is refused as LockOutsideCard",
                    token(los_jwst_fine_guide::LOCK_Y_MIN_ARCSEC),
                    token(los_jwst_fine_guide::LOCK_Y_MAX_ARCSEC)
                ),
            ),
            scalar(
                "quiet_before_start_s",
                Role::Phase,
                "s",
                Some(los_jwst_fine_guide::RECOVERY_S - los_jwst_fine_guide::RECOVERY_ALLOWANCE_S),
                false,
                None,
                false,
                format!(
                    "at least {} s: the time from the last centroid that was refused as PointingDisturbance or InnovationBeyondFourSigma, or that the caller knows was more than {} mas from the lock point, to the first frame of this run; {} frames or more. The card cannot see a centroid that came before its run, so the caller states this and the card holds it to {} s; it cannot check it, and a caller who states a time that is not so gets answers the comparison does not cover. Where no earlier centroid is known, as at the first frame of a fine-guide file, state {}. The published minimum is {} s less an allowance of 2^-36 s, the most a difference of two times under 36 hours, each read from a decimal, can differ from the exact difference; a time further under {} s is refused as RecoveryNotElapsed",
                    token(los_jwst_fine_guide::RECOVERY_S),
                    token(los_jwst_fine_guide::QUIET_OFFSET_MAS),
                    469,
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
                "0 or 1 and nothing between: 1 for a frame with a good centroid (centroid_time above 0 and bad_centroid_dq_flag GOOD in the table FGS CENTROID PACKET), 0 for a frame with none. A frame with measured 0 is predicted and not updated. At most 10 frames in a row may have measured 0; the eleventh is refused as MeasurementGapOutsideCard and the run ends at the frame before it".into(),
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
                    "guide_star_position_x of the table, unchanged, arcsec in the sensor's ideal frame. No bound of its own: on a measured frame more than 5 mas from lock_x_arcsec the frame is refused as PointingDisturbance and the run ends there, and more than {} mas from it the whole run is refused before its first step as OffsetOutsideCard. {LOS_UNMEASURED_RULE}",
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
                    "guide_star_position_y of the table, unchanged, arcsec in the sensor's ideal frame. No bound of its own: on a measured frame more than 5 mas from lock_y_arcsec the frame is refused as PointingDisturbance and the run ends there, and more than {} mas from it the whole run is refused before its first step as OffsetOutsideCard. {LOS_UNMEASURED_RULE}",
                    token(los_jwst_fine_guide::OFFSET_MAX_MAS)
                ),
            ),
            scalar(
                "counts_per_s",
                Role::Load,
                "counts/s",
                Some(0.0),
                false,
                Some(los_jwst_fine_guide::COUNTS_MAX_PER_S),
                false,
                format!(
                    "guide_star_instrument_counts_per_sec of the table, unchanged. On a measured frame from {} to {} counts per second, both inclusive, the range the constants were fitted on; below it the frame is refused as CountRateOutsideCard. {LOS_UNMEASURED_RULE}",
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
        "One segment of one step for each frame of the table, in order, with the table's own numbers as the load; the validation used dt_s 0.064. A run starts at time zero at the lock point given, with nothing known: every part at zero and each covariance at its unmeasured value. A single step starts in the same way at each call, so single steps cannot be chained; a run carries the filter from frame to frame. A frame the request itself shows to be malformed or off the card, a count rate outside the range on a measured frame, a centroid more than 250 mas from the lock point, a measured that is neither 0 nor 1, or numbers on a frame with measured 0, refuses the whole run before its first step. Where the card refuses, and how a caller goes on. The card describes quiet fine guide. When the pointing is disturbed the offset jumps by many mas and rings down, the filter follows slowly, and there its prediction is worse than the last centroid and its stated variance too small. So a measured centroid more than 5 mas from the lock point on x or y is refused as PointingDisturbance, and a measured centroid more than four stated standard deviations from where it was expected is refused as InnovationBeyondFourSigma. Either ends the run at that frame: the run is returned as refused, with its frames and its proof up to the last frame that landed, the refused centroid is not used, and the state is as the last landed frame left it. These two are met at their frame and not before the first step, though the request shows the first of them: a disturbance is a state of the telescope that a run reaches, and the frames before it stand. To go on, start a new run whose first frame is at least 30 s, 469 frames, after the refused centroid and after any later centroid more than 5 mas from the lock point, and state that time as quiet_before_start_s; a start stated to be sooner is refused as RecoveryNotElapsed. The new run knows nothing of the old one. The card cannot see centroids before its run, so it cannot check that statement. A centroid beyond four standard deviations also comes by chance where the stated variance is right, once in about 7,900 measured frames, 8.4 minutes: the card refuses it all the same. Each frame's slots carry where the centroid was expected before the frame was used and the variance stated for the difference, so the comparison with the centroid is the caller's: (centroid - lock) x 1000 - predicted, in mas, and its square over the variance, which averages 1 where the stated variance is right. The final state carries the sums nis_sum_x and nis_sum_y over measured_frames. The lock point is held to the span of the lock points the comparison answered, from -61.1512947 to 60.4564934 arcsec on x and from -64.2767 to 50.1812668 arcsec on y, the fitted and held-back files together; one outside it is refused as LockOutsideCard before the first step. No lock point nearer the edge of the sensor's field was compared. A frame with measured 0 that would make more than 10 such frames in a row is refused as MeasurementGapOutsideCard, and the run ends at the frame before it, as at a disturbance, though the request shows it. With no measurement the filter only predicts and its stated variance grows, and the longest run of frames with no measurement that a run of the comparison crossed was 10 frames on the fitted files and 2 on the held-back files; the tables hold longer gaps only where the comparison refused or did not run. The state counts those frames as unmeasured_in_row, and a measured frame sets it to zero. Every run starts with the count at zero, as it starts with nothing known, so a new run carries nothing over. To go on after the gap, start a new run at the next measured frame; a gap is not a disturbance, and quiet_before_start_s is still the time since the last one. The card has no limit in time of its own; a run records at most 10,000 frames. A step of zero or less is refused as NonPositiveDt before the step floor is tested.".into(),
        &[
            "NonFiniteInput",
            "NonFiniteState",
            "NonPositiveDt",
            "InvalidObservatory",
            "LockOutsideCard",
            "RecoveryNotElapsed",
            "InvalidCovariance",
            "MeasuredNotZeroOrOne",
            "UnmeasuredFrameCarriesNumbers",
            "MeasurementGapOutsideCard",
            "CountRateOutsideCard",
            "OffsetOutsideCard",
            "PointingDisturbance",
            "InnovationBeyondFourSigma",
            "SpontaneousGeneration",
            "FirstLawBreach",
        ],
        vec![Advisory {
            name: "FLAG_NO_MEASUREMENT",
            mask: los_jwst_fine_guide::FLAG_NO_MEASUREMENT,
            seals: true,
        }],
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
                "FLAG_NO_MEASUREMENT, bit 0: the table had no good centroid for the frame, and it was predicted only. It marks no failure of the model, and it seals. Bits 1 and 2 were the flags of a centroid more than 5 mas from the lock point and of one beyond four standard deviations; such a frame is now refused, and no frame sets them.",
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
const CANTILEVER_ENVELOPE_RULE: &str = "The deflection and the speed together must keep the envelope, sqrt(deflection_m^2 + (velocity_m_s / omega0)^2) with omega0 = 2 pi x 8.29235 Hz, at or below 0.01579 m and at or above 5.8e-5 m, the largest and the smallest envelope the card held on the run compared with the held-back ring-down";

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
                    "from -{0} to {0} m, both inclusive, with the beam at rest: the deflection at the laser-1 station, 268 mm from the clamp. {CANTILEVER_ENVELOPE_RULE}, so a deflection inside this range is refused as DeflectionAboveCard when the speed is large enough, and as DeflectionBelowCard when the deflection and the speed are both small enough, rest at zero included",
                    token(largest_deflection_m)
                ),
            ),
            within(
                "velocity_m_s",
                Role::Phase,
                "m/s",
                largest_speed_m_s,
                format!(
                    "from -{0} to {0} m/s, both inclusive, with the deflection at zero: the speed of that deflection, signed. {CANTILEVER_ENVELOPE_RULE}, so a speed inside this range is refused as DeflectionAboveCard when the deflection is large enough, and as DeflectionBelowCard when the deflection and the speed are both small enough, rest at zero included",
                    token(largest_speed_m_s)
                ),
            ),
        ],
        floor,
        "the ceiling divided by 2^18, set where the deflection moves least: from rest at the amplitude ceiling, a turning point, one step at the floor moves the deflection by five units in the last place toward rest, and no step from one to four floors is refused there. At a quarter of the floor the deflection there moves by less than one unit in the last place while the speed grows, and such steps were refused as DeflectionAboveCard by rounding alone",
        gate,
        "0.25 ms, about 482 steps per period, a locked constant set by measurement and not by a formula. The damping term has a sharp corner at zero deflection, which the beam crosses twice a period, so the error falls more slowly with the step than fourth-order Runge-Kutta usually gives. On the card's map over the 24 s of the compared run the largest conservation residual at this step is 5.6e-8 against a limit of 1e-6, and through the card's own step from the start of the scored ring-down to the floor it is 7.4e-8. Ten steps make one frame at 400 per second, the rate at which the reading procedure of the validation protocol was checked",
        None,
        "No load: the beam rings free, and a run segment carries an empty load object. A run starts at time zero with its ledger anchored at the deflection and the speed given. A single step starts in the same way at each call: its deflection_m and velocity_m_s handed back as a new start continue the same motion, and t_s and the heat begin again at zero. The envelope range is what the comparison with the held-back ring-down spanned. The compared run started 0.150 s before the start of the ring-down, on the model's own trajectory, at an envelope of 15.786 mm, and its readings were scored from the start crossing, 0.14873 s into the run, to the end of the last scored block, 23.469 s after it; over the run the card's envelope fell from 15.786 mm at its start, the largest it held, through 14.404 mm, at the first turning point after the start crossing, to 0.0587 mm. The ceiling is the first rounded up to 0.01579 m and the floor is the last rounded down to 5.8e-5 m. The envelopes from 14.41 to 15.79 mm are the ones the compared run passed through before its readings were scored; the ceiling keeps the compared run's own start, and that run lands 94,731 steps before it is refused as DeflectionBelowCard. From its state after 595 steps of 0.25 ms, at 0.14875 s, deflection -0.00104914043915228 m and speed 0.7408030982848752 m/s, the card gives the same deflections and speeds bit for bit, and it lands 94,136 steps, to 23.534 s, before it is refused as DeflectionBelowCard. A free decay only shrinks, so every run is refused as DeflectionBelowCard once its envelope would fall under 5.8e-5 m, and that run keeps its proof up to the last step that landed; rest at zero is refused. The envelope is measured with the frequency at vanishing amplitude and the softened spring is weaker, so within one period the envelope is smallest at the rest position and rises toward the turning point, by about 1 percent at the largest swing: a start at zero deflection with its envelope within 1 percent of the ceiling is accepted and is refused as DeflectionAboveCard a little under a quarter of a period later, and that run keeps its proof up to the last step that landed. With the envelope at 0.99 of the ceiling the step is refused after 104 steps of 0.25 ms; at 0.98 the beam rings through. The worked examples start at that state after 595 steps, the start of the scored ring-down. A step of zero or less is refused as NonPositiveDt before the step floor is tested.".into(),
        &[
            "NonFiniteInput",
            "NonFiniteState",
            "NonPositiveDt",
            "InvalidBeam",
            "DeflectionAboveCard",
            "DeflectionBelowCard",
            "SpontaneousGeneration",
            "FirstLawBreach",
        ],
        vec![],
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
                "Margin to the amplitude ceiling, in metres: 0.01579 − envelope_m.",
            ),
            ("residual", "Conservation residual."),
            (
                "flags",
                "Always zero. The card has no advisory condition: an envelope above the ceiling or below the floor and a residual past the limit are refusals.",
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
        "One 32.445 GHz microwave link across 1.26 km of Gothenburg in rain, OpenMRG link 10010 sublink 587: its path loss from the rain rate at the gauge beside it. The rain term is Recommendation ITU-R P.838-3 with nothing fitted; the three constants of the water on the antennas are fitted to this link's own readings of June and July 2015.",
        vec![
            scalar(
                "dry_path_loss_db",
                Role::Phase,
                "dB",
                Some(link_gothenburg_10010::DRY_PATH_LOSS_MIN_DB),
                false,
                Some(link_gothenburg_10010::DRY_PATH_LOSS_MAX_DB),
                false,
                "from 50 to 54 dB, both inclusive: the sent level minus the received level with no rain and dry antennas. It is the caller's, read from the link before the rain, as the median over the hour before the first wet minute; the card holds it through the run and does not fit it. The 18 fitted events lie between 51.75 and 52.55 dB".to_string(),
            ),
            scalar(
                "antenna_water_db",
                Role::Phase,
                "dB",
                Some(0.0),
                false,
                Some(link_gothenburg_10010::ANTENNA_WATER_MAX_DB),
                false,
                "from 0 to 2.13 dB, both inclusive: the attenuation of the water on the two antenna covers at the start. A link that has been dry for an hour starts at 0; covers still wet from earlier rain start above it, and a run's final state returns the value it reached. The card cannot see the rain that came before its run, so the caller states this; it cannot check it, and a value other than 0 is taken as stated, without from, as an honest start the card cannot tell from a continuation; a caller who states a value that is not so gets answers the comparison does not cover. 2.13 dB is the most the card gives it, a constant fitted to this link".to_string(),
            ),
            scalar(
                "gauge_minute_elapsed_s",
                Role::Phase,
                "s",
                Some(0.0),
                false,
                Some(link_gothenburg_10010::GAUGE_STEP_S),
                true,
                "from 0 s inclusive to 60 s exclusive: the seconds already spent in the current gauge minute at the start. A new link starts at 0, on a minute's boundary. A link continued from a returned state takes that state's gauge_minute_elapsed_s, so a restart cannot change the rain inside a minute. Outside the range it is refused as GaugeMinuteOutsideCard".to_string(),
            ),
            scalar(
                "gauge_minute_rain_mm_h",
                Role::Phase,
                "mm/h",
                Some(0.0),
                false,
                Some(link_gothenburg_10010::RAIN_RATE_MAX_MM_H),
                false,
                "0, or from 6 to 84 mm/h, both inclusive: the rain rate held since the current gauge minute began. While gauge_minute_elapsed_s is above 0 the first step must carry this rate, or it is refused as RainNotHeldForGaugeMinute. At 0 s it is not used, and it must still be on the card. A link continued from a returned state takes that state's gauge_minute_rain_mm_h".to_string(),
            ),
            scalar(
                "rain_rate_mm_h",
                Role::Load,
                "mm/h",
                Some(0.0),
                false,
                Some(link_gothenburg_10010::RAIN_RATE_MAX_MM_H),
                false,
                "0, or from 6 to 84 mm/h, both inclusive: the rain rate at the Barlastplatsen gauge, held through the step. 84 mm/h is the highest one-minute rate in the fitting data; nothing is extrapolated past it. The gauge counts 0.1 mm, so one count in a minute is 6 mm/h, and every one-minute rate the card was fitted and compared on is 0 or at least 6 mm/h. A rate above 0 and below 6 mm/h is refused as RainRateBelowGaugeCount, in any segment of a run before its first step; a rate averaged over longer than a minute is fed as the gauge's own minutes".to_string(),
            ),
        ],
        floor,
        "the ceiling divided by 2^20: a step that moves the water on the antennas by at most seven parts in ten million of its ceiling. Under it a step is mostly rounding",
        gate,
        "60 s, the gauge's minute and the step the three fitted constants were fitted at, a locked constant set by the measurement and not by a formula. The map of the water on the antennas is exact at any step, and each energy is its power integrated exactly over the step, so one step and many steps over the same span give the same state and the same energies to rounding, within one part in 10^13",
        None,
        "No load Nyquist: the rain rate is held through each step and a run segment carries one rain rate. The fit and the comparison fed rain held through whole gauge minutes, so the rain may change only on a minute's boundary. The card keeps the clock of the gauge minute: gauge_minute_elapsed_s, the seconds already spent in the current minute, and gauge_minute_rain_mm_h, the rate held since it began. A minute is complete when its clock is within half the step floor, about 2.9e-5 s, of 60 s. A step that starts inside a minute with another rate is refused as RainNotHeldForGaugeMinute and keeps the state: in a run, at the first step of the segment that changes the rate, and the run is sealed with that refusal after the steps that landed. Steps shorter than 60 s land while the rain is held for whole minutes, and give the same water as whole minutes. A run starts at time zero with its ledgers anchored at the dry level and the antenna water given, and with the gauge minute given. A single step starts in the same way at each call. Its dry_path_loss_db, antenna_water_db, gauge_minute_elapsed_s and gauge_minute_rain_mm_h handed back as a new start continue the same link and the same gauge minute. t_s and the ledgers start again from zero, and no limit depends on them.".into(),
        &[
            "NonFiniteInput",
            "NonFiniteState",
            "NonPositiveDt",
            "InvalidLink",
            "RainRateOutsideCard",
            "RainRateBelowGaugeCount",
            "RainNotHeldForGaugeMinute",
            "GaugeMinuteOutsideCard",
            "DryPathLossOutsideCard",
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
                "Margin to the highest rain rate of the fitting data, in mm/h: 84 − rain_rate_mm_h.",
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
        "One Tesla Model 3 2170 lithium-ion cell, any of the three cells m80, m1000 and m448-N of its dataset, as a physical model with memory, from -10 to 40 degC, under a terminal current and a measured case temperature.",
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
                Some(cell_2170_model3::STATE_OF_CHARGE_MIN),
                false,
                Some(cell_2170_model3::STATE_OF_CHARGE_MAX),
                false,
                "inclusive [0, 1.02]: 1 after a full recharge, and 1 plus the top-up in ampere-hours divided by capacity_ah when the tester topped the cell up afterwards. The largest start in the tests was 1.012. The cell is taken as rested: every memory starts at zero. A run lands a step whose state of charge ends within the rounding allowance of the running sum past 0 or 1.02, STATE_OF_CHARGE_ALLOWANCE_PER_STEP, 2.22e-16 for each step of the run so far and two more; handed back as a new start, such a value is past this bound and is refused".into(),
            ),
            scalar(
                "capacity_ah",
                Role::Phase,
                "Ah",
                Some(cell_2170_model3::CAPACITY_MIN_AH),
                false,
                Some(cell_2170_model3::CAPACITY_MAX_AH),
                false,
                format!(
                    "inclusive [4.16, 4.7] Ah: the capacity from the last full charge, which falls as the cell ages. The two cells had 4.174 to 4.689 Ah in the scored tests, and the range allows 0.01 Ah beyond that. It is the caller's number and the card never changes it. Each block of tests was measured at one age of the cell, so the capacity is also held to the band each block was scored with, at the case temperatures that block spanned: 4.16 to 4.26 Ah from -12.9 to 12.6 degC, 4.20 to 4.30 Ah from 0.0 to 20.4 degC, 4.17 to 4.30 Ah from 9.8 to 26.8 degC, 4.35 to 4.53 Ah from 23.3 to 31.4 degC and 4.56 to 4.70 Ah from 28.1 to 45.9 degC. These are the capacities of every run scored from -10 to 40 degC on the three cells and the block capacities of their fitting files, with 0.01 Ah beyond them, and the case temperatures the sensor read in those runs, with the chamber's 0.3 K beyond them. A capacity and a case temperature that no one block holds were never compared, a new cell in the cold or an aged cell when warm, and are refused as CapacityOutsideCardAtTemperature. {CELL_2170_MODEL3_CAPACITY_RULE}"
                ),
            ),
            scalar(
                "case_temperature_least_c",
                Role::Phase,
                "degC",
                Some(cell_2170_model3::CASE_TEMPERATURE_MIN_C),
                false,
                Some(cell_2170_model3::CASE_TEMPERATURE_MAX_C),
                false,
                "inclusive [-12.9, 45.9] degC, required: the least case temperature of the run so far. The card refuses a case temperature that spans more than 19.2 K since the rested start, and that span is accumulated over the steps of one request: a run starts rested and states case_temperature_least_c equal to its first segment's case_temperature_c, and a single step equal to its case_temperature_c. A start without it is refused. A value above case_temperature_most_c, or one more than 19.2 K below it, is refused as CaseTemperatureSpanAboveCard; one outside the card's case temperatures as CaseTemperatureOutsideCard".into(),
            ),
            scalar(
                "case_temperature_most_c",
                Role::Phase,
                "degC",
                Some(cell_2170_model3::CASE_TEMPERATURE_MIN_C),
                false,
                Some(cell_2170_model3::CASE_TEMPERATURE_MAX_C),
                false,
                "inclusive [-12.9, 45.9] degC, required: the most case temperature of the run so far: a run starts rested and states it equal to its first segment's case_temperature_c, and a single step equal to its case_temperature_c. A start without it is refused. The first segment's case temperature and every later one are weighed with both before the first step, and a run whose case temperatures and the stated pair span more than 19.2 K is refused whole as CaseTemperatureSpanAboveCard".into(),
            ),
            scalar(
                "current_a",
                Role::Load,
                "A",
                Some(-cell_2170_model3::CHARGE_CURRENT_MAX_A),
                false,
                Some(cell_2170_model3::DISCHARGE_CURRENT_MAX_A),
                false,
                "inclusive [-13.984, 13.524] A. A positive current discharges the cell and a negative current charges it: the opposite sign to the measurements and to the cell_2170_m80 card. 13.524 A is the tester's programmed pulse of 13.5 A plus 0.024 A, the accuracy the dataset states for the tester's current; the pulse reads up to 13.506 A in the logs. 13.984 A is the largest charging current in a scored drive cycle, 13.960 A on cell m1000, plus the same 0.024 A; on cell m80 the largest was 9.74 A".into(),
            ),
            scalar(
                "case_temperature_c",
                Role::Load,
                "degC",
                Some(cell_2170_model3::CASE_TEMPERATURE_MIN_C),
                false,
                Some(cell_2170_model3::CASE_TEMPERATURE_MAX_C),
                false,
                "inclusive [-12.9, 45.9] degC: the temperature measured on the cell's case at the end of the step. With the chamber set from -10 to 40 degC the sensor read from -12.59 to 45.54 degC in the scored tests, and the range allows 0.3 K beyond that, the accuracy the dataset states for the chamber. The card is given it and does not predict it. -20 degC is outside the card. It is paired with capacity_ah, as that field states. In the scored runs the case temperature changed by at most 5.023 K between two consecutive seconds and spanned at most 18.811 K inside one run: a step that differs from the last step's by more than 5.4 K is refused as CaseTemperatureChangeAboveCard, and one that takes the span since the rested start past 19.2 K as CaseTemperatureSpanAboveCard, each the measured value with 0.3 K beyond it".into(),
            ),
        ],
        floor,
        "a locked 1 s, the ceiling itself: every fit and every score was made at one second, and measurements logged once a second cannot check a shorter step. For a held 4.5 A discharge at 25 degC, the one load checked, a step of 0.1 s gave the one-second answer to within 0.3 mV, but where the current reverses it does not. When the surface reaction's overpotential would change sign inside a step the step is taken again from zero with the other law, so the answer depends on the step: from 4.5 A to -4.5 A at 0 degC the 0.1 s answer differs from the one-second answer by 49 mV in the second of the reversal, 5 mV in the next and 0.3 mV in the one after, and over the first 6,000 seconds of a drive cycle of cell m448-N at -10 degC by up to 32 mV. A shorter step can come back only after that sign change is split at the zero crossing and the card is fitted and scored again",
        gate,
        "a locked 1 s, the sampling of the measurements every fit and every score was made on. A load that changes within a second is not represented by a longer step, and the constants were fitted with the one-second rule for the reaction changing sign",
        None,
        "Over a step the current and the case temperature are held. A current or a case temperature outside its range, a capacity not compared at a segment's case temperature (CapacityOutsideCardAtTemperature), or case temperatures whose span over the segments passes 19.2 K (CaseTemperatureSpanAboveCard), in any segment of a run, refuses the whole run before its first step. A change of more than 5.4 K between the case temperatures of two consecutive steps ends the run in a sealed refusal at that step (CaseTemperatureChangeAboveCard). The change counts from the start: the first step has no last step to differ from. The span counts from the rested start and is accumulated over the steps of one request: a run, or a single step, starts rested and states case_temperature_least_c and case_temperature_most_c both equal to its first case temperature, and a start without them is refused. A start from a returned state is not served on this card: through from it is refused as ContinuationNotOnCard, and without from a pair other than a fresh start's is refused as ContinuationWithoutUpstream, because the card's other memories are not start fields and would start again at zero, among them the 60 s running mean of the current that SustainedCurrentOutsideCard holds and the last case temperature that CaseTemperatureChangeAboveCard counts the 5.4 K change from. A run starts from a rested cell: the state of charge and the capacity as given, the least and most case temperature as given, every other memory at zero, and the ledger anchored there. An hour's rest is enough anywhere on the card; a cell that has not rested is not in that state, and the run should start earlier and go through the load that came before. The first measured sample of a record is the start and each later sample is one step, so a logged drive cycle is one run with a segment of one step for each second. A single step starts from rest in the same way at each call, so single steps cannot be chained into a run. A run ends in a sealed refusal when the terminal voltage would fall below 2.8 V (UndervoltageLimit) or rise above 4.242 V (OvervoltageLimit), when the state of charge would leave 0 to 1.02 by more than the rounding allowance of its running sum (StateOfChargeOutsideCard), when the state of charge at the particle surface would leave the measured curves (SurfaceOutsideCurve), or when the running mean of the current, with a 60 s time constant, would pass 8.722 A discharging or 2.993 A charging (SustainedCurrentOutsideCard); the last landed state is the last step on the card. Those two sustained currents are the largest that running mean reaches in any scored test, 8.698 A and 2.969 A in the hill cycle of cell m80, plus 0.024 A: the hill cycle is the only test of sustained current above the rated current of about 4.5 A, and the model's largest misses are in it. Above a running mean of 4.6 A discharging a frame carries FLAG_SUSTAINED_ABOVE_1C and the run goes on. Charging current is accepted: it was fitted inside the drive cycles of cell m80 and in a slow charge and scored inside the drive cycles of cell m1000, which charge harder, and no charging run was scored. A step of zero or less is refused as NonPositiveDt before the step floor is tested.".into(),
        &[
            "NonFiniteInput",
            "NonFiniteState",
            "NonPositiveDt",
            "InvalidCell",
            "CellNotRecognized",
            "CapacityOutsideCard",
            "CaseTemperatureOutsideCard",
            "CurrentOutsideCard",
            "CapacityOutsideCardAtTemperature",
            "CaseTemperatureChangeAboveCard",
            "CaseTemperatureSpanAboveCard",
            "SustainedCurrentOutsideCard",
            "StateOfChargeOutsideCard",
            "SurfaceOutsideCurve",
            "UndervoltageLimit",
            "OvervoltageLimit",
            "SpontaneousGeneration",
            "FirstLawBreach",
        ],
        vec![Advisory {
            name: "FLAG_SUSTAINED_ABOVE_1C",
            mask: cell_2170_model3::FLAG_SUSTAINED_ABOVE_1C,
            seals: true,
        }],
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
                "FLAG_SUSTAINED_ABOVE_1C: the running mean of the current is above 4.6 A, discharging. Only the hill cycle of the tests is in this region, and below 10 degC the card is known to read high in it. It seals.",
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
                Some(tire_eve_r19_ice::TRACK_LENGTH_M),
                false,
                "inclusive [0, 6] m: where the wheel centre stands on the track at the start. The track ends at 6 m, the length rolled in a measured run, and a step that would end beyond it by more than the rounding allowance of the running sum, 5.5e-11 m at every step, is refused as TravelBeyondTrack".into(),
            ),
            scalar(
                "slip_held_s",
                Role::Phase,
                "s",
                Some(tire_eve_r19_ice::SLIP_SETTLE_S),
                false,
                Some(tire_eve_r19_ice::HOLD_CEILING_S),
                false,
                "inclusive [5, 21] s: the time the slip of the first step has already been held at the start. The card stands behind the force from 5 s to 21 s after a slip is set: a time below 5 s is refused as SlipNotSettled and one above 21 s as HoldBeyondMeasured. A run carries it from step to step while the slip is held, and its final state returns it; to go on from a returned state, hand back its slip_held_s with its distance_m. The card cannot see a hold that came before its run, so the caller states this; it cannot check it, and a caller who states a time that is not so gets answers the comparison does not cover".into(),
            ),
            scalar(
                "normal_load_n",
                Role::Load,
                "N",
                Some(tire_eve_r19_ice::NORMAL_LOAD_FLOOR_N),
                false,
                Some(tire_eve_r19_ice::NORMAL_LOAD_CEILING_N),
                false,
                "inclusive [3500, 6500] N: the normal load on the wheel. The constants were fitted at 3500 and 5000 N; above 5000 N a frame carries FLAG_LOAD_ABOVE_FITTED".into(),
            ),
            scalar(
                "inflation_pressure_pa",
                Role::Load,
                "Pa",
                Some(tire_eve_r19_ice::INFLATION_PRESSURE_FLOOR_PA),
                false,
                Some(tire_eve_r19_ice::INFLATION_PRESSURE_CEILING_PA),
                false,
                "inclusive [150000, 350000] Pa, gauge: the pressures measured. The pressure does not enter the force; it is asked for so that a run outside the measured pressures is named".into(),
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
        "the ceiling divided by 2^14, 2^-10 s, about a sixth of the sampling interval of the measurements. A run at one slip is then at most 16,384 steps, and rounding in the three sums of work stays under a hundredth of the conservation limit; the whole track at the floor and 0.05 m/s is 122,880 steps",
        gate,
        "16 s: the time from 5 s to 21 s after a slip is set, the hold the card stands behind. The force has no dynamics, so a step of any size inside the range gives the same force",
        None,
        "Over a step the normal load, the pressure, the slip and the carriage speed are held. The force is the same at every step of a held load: sign(slip) x MU x (|slip| / 0.10)^-0.132823 x normal_load_n, with MU 0.228109 in traction and 0.24162 in braking, all three numbers fitted to this tyre's own runs. The card stands behind the force from 5 s to 21 s after a slip is set. The registered comparison scored each slip stage on the window from 10 s to 1 s before it ended. Run again on 2026-10-10 by the same cells and the same five lines on the time after each slip change, with the change placed where the wheel speed steps, the force ratio met all five lines in every one-second window from 5 s to 20 s, and every one-second window before 5 s failed a line: the first second failed the line of 0.045 at 6.5 kN, with 0.048, and every second from 1 s to 5 s did no better than the constant comparator. With the protocol's own stage cuts the window from 5 s to 11 s met all five lines as well. No scored slip was held longer than one stage of about 20 s, except the last slip of each run, 7 or 20 percent, which the rig held for 21 to 33 s. So a start states slip_held_s, the time the slip of the first step has already been held, from 5 s to 21 s; a run carries it from step to step while the slip is held, and it starts again at zero when the slip changes. A step that starts before 5 s is refused as SlipNotSettled: a start that states less, and the first step after any change of slip, so a run holds one slip. A step that ends beyond 21 s is refused as HoldBeyondMeasured. The rise of the force from rest is not modelled. A run starts at time zero at distance_m with the three works at zero. A single step starts in the same way at each call: its distance_m and its slip_held_s handed back as a new start continue along the track and along the hold, and the works begin again at zero. The card cannot see a hold that came before its run, so it cannot check a stated slip_held_s. A value outside its range in any segment of a run refuses the whole run before its first step. A run ends in a sealed refusal at the step that would take its slip beyond 21 s, HoldBeyondMeasured, at the step that changes its slip, SlipNotSettled, or at the step that would end beyond 6 m, TravelBeyondTrack. The distance is a running sum, distance_m += carriage_speed_m_s x dt_s in 64-bit arithmetic, and each addition rounds by up to 4.44e-16 m, half the spacing of 64-bit numbers between 4 m and 8 m; a distance handed back as a new start carries the rounding of the steps before it. The comparison with 6 m therefore carries a named allowance, TRACK_END_ALLOWANCE_PER_STEP_M: 4.44e-16 m for each of the 122,880 steps the whole track holds at the step floor and 0.05 m/s, and six more for the start, the speed and the step read from decimals. It is 5.5e-11 m at every step, and never more than half the step, so one step and many steps to the same end, from a decimal start or from a returned one, meet the same allowance. The time held is summed in the same way, and the comparison with 21 s carries HOLD_END_ALLOWANCE_PER_STEP_S, 1.78e-15 s, half the spacing of 64-bit numbers between 16 s and 32 s, for each of the 16,384 steps of the floor in 16 s and six more: 2.9e-11 s. A run whose end in decimal is exactly 6 m, or exactly 21 s at one slip, lands its last step. The distance is stepped at its own value and is not pulled back, so the last distance_m of such a run can read up to the allowance past 6 m; handed back as a new start, that value is past the track and is refused as TravelBeyondTrack.".to_string(),
        &[
            "NonFiniteInput",
            "NonFiniteState",
            "NonPositiveDt",
            "InvalidTire",
            "NormalLoadOutsideCard",
            "InflationPressureOutsideCard",
            "SlipOutsideCard",
            "CarriageSpeedOutsideCard",
            "TravelBeyondTrack",
            "SlipNotSettled",
            "HoldBeyondMeasured",
            "SpontaneousGeneration",
            "FirstLawBreach",
        ],
        vec![Advisory {
            name: "FLAG_LOAD_ABOVE_FITTED",
            mask: tire_eve_r19_ice::FLAG_LOAD_ABOVE_FITTED,
            seals: true,
        }],
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
                "FLAG_LOAD_ABOVE_FITTED: the normal load is above 5000 N, the largest load the constants were fitted on. It seals.",
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
        "One cast of the single-crystal nickel alloy CMSX-6, loaded along [001] at 980 degrees Celsius, creeping in tension under a held load: the percentage creep extension of a round test piece 6 mm in diameter against the time since the full load, from 0.1 percent to 5 percent at 170 to 230 MPa and from 0.1 to 0.2 percent at 140 to under 170 MPa. Six constants fitted to six creep tests of this cast; the time to a level met its three lines on six held-back tests. Below 0.1 percent, and past 0.2 percent under 170 MPa, the law misses the measured curves, and there the card refuses.",
        vec![
            scalar(
                "creep_extension_percent",
                Role::Phase,
                "%",
                Some(extension_min),
                false,
                Some(extension_max),
                false,
                format!(
                    "from {extension_min} percent, inclusive, to the limit for the stress, inclusive: {extension_max_low} percent at a stress below {full_reach} MPa and {extension_max} percent from {full_reach} MPa. It is the extension since the full load was applied, over the reference length, as DIN EN ISO 204 defines the percentage creep extension; the elastic and the initial plastic extension of loading are not in it. A start below {extension_min} percent is refused as CreepExtensionBelowCard: there the card's law is slow to start against every measured curve. A start above the limit for its stress is refused as CreepExtensionOutsideCard. {extension_max} percent is the largest level the constants were fitted on; {extension_max_low} percent is the largest registered level up to which the law held on all three tests at 140 MPa at every level"
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
        "0.2 over the largest rate of change of the creep rate with the creep extension that the law can reach from zero to 5 percent, at 230 MPa: about 50 s. The bound was taken from zero extension and is kept, so it is smaller than the range from 0.1 percent needs",
        None,
        "The card answers one question: at a held stress, how far has the test piece crept at a given time since the full load, from 0.1 percent creep extension up. t_s is the time since the full load was applied, as the card's law gives it. A run starts at the creep extension given, with empty books and the clock at the time the law itself takes to reach that extension from zero at the stress of the first segment: 0.1 percent at 50.79 h at 140 MPa, 11.36 h at 170 MPa, 2.96 h at 200 MPa and 0.668 h at 230 MPa. That time since load is what was compared with measurement. The time between two extensions was not, and it does not hold: against the measured curves the law's time from one level to the next is off by up to 1.06 in the natural logarithm, and a clock started at 0.1 percent is outside the registered worst line of 0.30 at 5 to 16 of 170 levels at 170, 200 and 230 MPa. Do not subtract one frame's time from another's. A single step starts in the same way at each call: its creep_extension_percent handed back as a new start continues the same test, and the clock continues with it, while the work and the heat begin again at zero. stress_mpa is held: every segment of a run carries the stress of the first, and a segment with another stress refuses the whole run before its first step as StressNotHeld; no test with a changed stress is in the record. A stress outside the span in any segment refuses the whole run before its first step. The temperature is 980 degrees Celsius and is not an input. Where the card refuses: a start below 0.1 percent creep extension, zero among them, is refused as CreepExtensionBelowCard, because below it the law is slow to start against every measured curve at every stress (in hours to 0.05 percent, the law against the three test pieces, 38.9 against 15.4 to 26.7 at 140 MPa, 7.8 against 2.7 to 3.5 at 170 MPa, 1.69 against 0.61 to 0.66 at 200 MPa and 0.245 against 0.06 to 0.08 at 230 MPa); the upper limit is 5 percent from 170 MPa and 0.2 percent below 170 MPa, because at 140 MPa the law leaves the worst line from 0.26 percent on one test and runs ahead of all three past 0.66 percent, and no test lies between 140 and 170 MPa. Under a held stress the extension only grows, and the step that would pass the limit is refused as CreepExtensionBeyondCard: a run that asks for more steps than that ends there, sealed with that refusal, and its last landed state is within one step of the limit. A step that lands exactly on the limit lands. At the ceiling from 0.1 percent that is 4,462 steps at 230 MPa, 11,031 at 200 MPa and 30,506 at 170 MPa, each to 5 percent, and 5,159 at 140 MPa, to 0.2 percent at 122.4 h. All are inside the 1,000,000 steps of a run. The card has no limit in time: the latest hour it can reach is 434.84 h, at 170 MPa and 5 percent. A step of zero or less is refused as NonPositiveDt before the step floor is tested.".into(),
        &[
            "NonFiniteInput",
            "NonFiniteState",
            "NonPositiveDt",
            "InvalidCard",
            "StressOutsideCard",
            "StressNotHeld",
            "CreepExtensionBelowCard",
            "CreepExtensionOutsideCard",
            "CreepExtensionBeyondCard",
            "StepTooSmallToResolve",
            "SpontaneousGeneration",
            "FirstLawBreach",
        ],
        vec![Advisory {
            name: "FLAG_CREEP_RATE_RISING",
            mask: creep_cmsx6::FLAG_CREEP_RATE_RISING,
            seals: true,
        }],
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
                "FLAG_CREEP_RATE_RISING: the law's creep rate at the end of the step is above its rate at the start. The law's rate falls from 0.1 percent to a minimum near 0.26 percent and rises from there; the flag says where on its own curve the law is and marks no failure of it. Below 170 MPa the card ends at 0.2 percent and the flag is never set. It seals.",
            ),
            ("proof", "Not written by the step."),
        ]),
    )
}

/// What the capsule says of its two speed fields: the speed through the air
/// they make, its band above 68.96 km, and the corridor below it.
const CAPSULE_SPEED_REACH: &str = "The speed through the air is the square root of the sum of the squares of radial_speed_m_s and of transverse_speed_m_s less the air's own turning, 5.4197601852574874e-05 rad/s times radius_m. Above 68.96 km, where drag is read by altitude from a table computed along the 1997 design trajectory at 12173 to 12620 m/s (12487 to 12620 m/s in Table 1, 12173 m/s at the 68.96 km row from Table 4, of Mitcheltree, Wilmoth, Cheatwood, Brauckmann and Greene, Aerodynamics of Stardust Sample Return Capsule, AIAA 97-2304, 1997; that paper does not say whether these speeds are inertial or through the air), the speed through the air must be from 11900 to 12575 m/s, and a state outside is refused as AirSpeedOutsideCard: the band is the span compared, the air speeds the locked entry passes through there, 11911.98 to 12573.44 m/s at every step from 0.05 s to 0.0005 s, rounded outward to 25 m/s. Below 68.96 km, where drag is read by Mach number, the Mach number must be from 1.0 to 40, the ends of that table, and a state outside is refused as MachOutsideCard. Below 68.96 km, and for a state moving upward above it, the speed through the air and the sine of the path angle through the air, radial_speed_m_s over the speed through the air, must be inside the corridor the locked entry flew at that altitude, and a state outside is refused as OffComparedCorridor: the corridor is the lowest and highest of each that entry had within 500 m of the altitude, run at ten steps from 0.05 s to 0.0005 s down to the first state past both deployment conditions, widened by 1 m/s and by 0.001, at knots every 500 m from 31 km to 69 km, read by straight lines between knots. At 40 km the speed through the air must be from 1147.51 to 1350.11 m/s. Under 31 km, and moving upward above 69 km, no state is inside it. Above 86 km the speed of sound is held at its 86 km value, 274.096 m/s, so the Mach number reported there is not the real air's; the band, not the Mach number, is the limit there. A state under 31030 m and under Mach 1.23 at once, past the flown drogue deployment, is refused as PastFlightDeployment at the start; a run that reaches it is ended there, at the end the card declares. A start deep in the atmosphere at the entry speed, which engine 0.6.0 answered and the largest step could refuse on the ledger, is off the corridor and is refused as OffComparedCorridor before any step";

/// What the InSight capsule says of a speed above its entry: the two speed
/// fields publish no bound of their own; the speed through the air is held.
const CAPSULE_INSIGHT_SPEED_REACH: &str = "The speed has no bound of its own. Its reach: the speed through the air, which is this speed less the air's own turning with Mars, must be at or above Mach 1.46, the lowest the compared run reached (1.470) rounded outward, or the state is refused as PastComparedFlight; with the path angle through the air it must be inside the corridor the compared runs flew at that altitude, which is never above 5546.4 m/s and never climbing, or the state is refused as OffComparedCorridor. Below Mach 1.0 the drag table read by Mach number ends and the state is refused as MachOutsideCard; above 5850 m/s the drag table read by speed ends and it is refused as AirSpeedOutsideCard";

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
        "The InSight entry capsule on its entry at Mars of 26 November 2018, from the top of the atmosphere to just past the flight's parachute deployment, down to 5700 m or Mach 1.46: no wind, the air of orbiter soundings set on the lander's barometer, and the capsule's lift as the load. Only runs from the locked entry state were compared with the flight, and only states on the corridor they flew are answered. It does not forecast an entry whose lift is not known.",
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
                    "distance from the centre of Mars. Altitude is radius_m - {} and is in [5700, 126500] m: below 5700 m, the lowest altitude the run compared with the flight reached (5793 m) rounded outward, a state is refused as PastComparedFlight, and above 126500 m, over the entry altitude of 126486 m, as OffComparedCorridor. Outside [4000, 128000] m, the span of the air tables, it is refused as AltitudeOutsideCard",
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
        "the largest step the card stands behind, a locked constant, stated for the entry from the locked entry state: there the capsule falls 120 m in one step, under one interval of the air tables, and over the whole entry the conservation residual stays near a thousandth of its limit. A start deep in the atmosphere at entry speed, a state the entry itself never passes through, is off the compared corridor and is refused as OffComparedCorridor before any step",
        None,
        "lift_over_drag is in the published span, and a later segment of a run may change it. A lift outside that span in any segment refuses the whole run before its first step. A run from the entry state is ended as PastComparedFlight, the end the card declares, at 5700 m or Mach 1.46, whichever comes first: the lowest state the run with the flight's lift reached in its comparison, 5793 m and Mach 1.470, rounded outward: with no lift at 227.3 s, 7.06 km up; with a lift over drag of -0.031 at 211.0 s, 5.72 km up. The flight record ends at parachute deployment, 209.27 s after the entry state, 6164.6 m above the datum and 352.1 m/s through the air. Only runs from the locked entry state at 0.1 s were compared with the flight: with one lift over drag from -0.08 to 0.08 held through the run, or with -0.031 or 0 to 150 s and one lift in that span after. The corridor is the lowest and highest speed through the air and sine of the path angle those runs had within 500 m of each altitude, widened by 1 m/s and by 0.001, at knots every 500 m from 5.5 km to 126.5 km. A start or a landed state outside it is refused as OffComparedCorridor, so a run whose lift changes in a way not compared can be refused part way. A restart from a state such a run returns lands. The corridor and the floor depend on the state alone. A new start sets the time, the angle and the drag-loss ledger to zero and anchors the ledger there; no limit depends on them.".into(),
        &[
            "NonFiniteInput",
            "NonFiniteState",
            "NonPositiveDt",
            "InvalidCapsule",
            "LiftOutsideCard",
            "AltitudeOutsideCard",
            "MachOutsideCard",
            "AirSpeedOutsideCard",
            "PastComparedFlight",
            "OffComparedCorridor",
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
        "The Stardust sample return capsule on its entry of 15 January 2006, from entry interface down to the drogue-deployment condition the radar recorded, under 31030 m and under Mach 1.23 at once, past which a state is refused: no lift, no wind, a yearly standard atmosphere, and no load. One entry was compared with the flight, the locked entry from entry interface; off that path the card holds the limits each field states.",
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
                    "distance from the Earth's centre. Altitude is radius_m - {} and is in [30000, 134700] m. Under 31000 m a state is off the corridor the locked entry flew and is refused as OffComparedCorridor",
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
        "the largest step the card stands behind, a locked constant, stated for the entry from entry interface: over the whole entry the conservation residual stays under a thousandth of its limit. Its reach: a start deep in the atmosphere at entry speed, a state the entry itself never passes through, is where the step method's own error at this step is larger than the conservation limit; it is off the corridor the locked entry flew and is refused as OffComparedCorridor before any step. Through engine 0.6.0 a start at 40 km at the entry speed was refused at this step as SpontaneousGeneration or FirstLawBreach and completed at 0.005 s",
        None,
        "No load. A run segment carries an empty load object. A run from entry interface at the ceiling step lands 2779 steps, the last the first under both 31030 m and Mach 1.23, and is ended at the next as PastFlightDeployment, the end the card declares. The step that first meets both drogue conditions lands, so a run at a smaller step can end earlier, by less than one of the larger steps: at 0.005 s the run lands 27782 steps, to 31029.67 m at 138.91 s. A start or a landed state off the corridor the locked entry flew is refused as OffComparedCorridor; every state of that entry is inside it, so a restart from a state it returns lands. The corridor depends on the state alone. A new start sets the time, the angle and the drag-loss ledger to zero and anchors the ledger there; no limit depends on them. Through engine 0.6.0 a run from entry interface ran on to 30 km or Mach 1.0 with an advisory flag.".into(),
        &[
            "NonFiniteInput",
            "NonFiniteState",
            "NonPositiveDt",
            "InvalidCapsule",
            "AltitudeOutsideCard",
            "MachOutsideCard",
            "AirSpeedOutsideCard",
            "PastFlightDeployment",
            "OffComparedCorridor",
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
            "UTC at the start of the run, in seconds after 2000-01-01 12:00 UTC: 2026-08-23 00:00 ({} s), the first start of the compared runs, to the latest start from which the card's smallest step still lands on or before 2026-09-20 00:00 ({} s), the end of the last compared run. These 28 days are the dates the card was compared on. A later start is refused at every step",
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
        Some(satellite_lageos1::FORECAST_HORIZON_S),
        false,
        format!(
            "from 0 to {} s, seven days, both inclusive: the time since the state was taken from a published orbit. Required. State 0 for a state turned from a published orbit, and the forecast_elapsed_s of a returned state for a state handed back. The card holds this plus t_s to seven days, the length of the compared runs, so a forecast carried on through restarts stops where a run straight through stops; a step that would end later is refused as HorizonOutsideCard. A start at seven days is accepted and its step refused. The card cannot check the number stated",
            token(satellite_lageos1::FORECAST_HORIZON_S)
        ),
    );
    let position = vec3_stated(
        "position_m",
        "m",
        "in the card's turning frame. Its length is from 11 997 km to 12 327 km, the radii the compared runs reached".into(),
    );
    let velocity = vec3_stated(
        "velocity_m_s",
        "m/s",
        "in the card's turning frame. The orbit is one of the two compared orbits: LAGEOS-1, semi-major axis 12 265 to 12 275 km, eccentricity 0.0034 to 0.0044 and inclination 109.80 to 109.83 degrees, or LAGEOS-2, 12 158 to 12 166 km, 0.0132 to 0.0138 and 52.63 to 52.66 degrees, each worked out from the position and the velocity seen from a frame that does not turn, with the card's z axis as the pole".into(),
    );
    finish(
        "aerospace",
        "satellite_lageos1",
        "The laser-ranging satellite LAGEOS-1 in free flight, in a frame turning with the Earth: the Earth's field to degree and order 12, the Sun and the Moon, the solid tide, sunlight pressure and relativity, with no load.",
        vec![epoch, forecast, position, velocity],
        floor,
        "the ceiling divided by 2^20: 0.054 m of flight. Under it a step is mostly rounding",
        gate,
        "the largest step the card stands behind, a locked constant. Measured against an 8th-order reference from the worked example of engine 0.6.0, a circular polar orbit that the card now refuses, the step error at this step is 11.2 mm after one day and 13.9 cm after seven days, and 0.4 mm to 21 mm after one day from four other start states inside the 0.6.0 bands, none of them on a date and an orbit the card now serves. The measurement is an outside recomputation in 50-digit arithmetic, 2026-10-07, one checker, not yet reviewed",
        None,
        "No load. A run segment carries an empty load object. What was compared with published orbits, and is all the card serves: two orbits, LAGEOS-1 and LAGEOS-2, on 28 calendar days, 2026-08-23 to 2026-09-19, in 56 runs of 10 s steps of at most seven days each, 42 of them on the held-back weeks that were scored against the pass line, the last ending 2026-09-20 00:00 UTC. The radius band, the two orbit bands, the dates and the seven days of a forecast are applied to the incoming state and to every landing, and each refuses by name outside: RadiusOutsideCard, OrbitOutsideCard, EpochOutsideCard and HorizonOutsideCard. The bands of each orbit hold every start and every landing of its compared runs; an orbit of any other inclination, eccentricity or size, polar and equatorial orbits among them, was never compared and is refused. The last date and rounding: the clock of a run is a running sum, t_s += dt_s in 64-bit arithmetic, and the time of a state is the start epoch plus that clock, rounded to the spacing of 64-bit numbers at these dates, 1.19e-7 s. The comparison with 2026-09-20 00:00 therefore carries a named allowance: EPOCH_END_ALLOWANCE_S, 1.19e-7 s, and CLOCK_ROUNDING_PER_STEP, one part in 2^53 of the clock, for each step taken so far at this step and two more, and never more than half the step. It is 1.2e-7 s on the first step of a run and 1.1e-5 s after 1,000,000 steps of 0.1 s. A run whose end in decimal is exactly the last date lands its last step, and its last epoch_utc_s can read up to the allowance past it; handed back as a new start, that value is past the card and is refused. The allowance is in the comparison a step makes and not in the published maximum of epoch_utc_s. The seven days of a forecast carry the same allowance, with the steps counted from the published state over every run, so a run whose end in decimal is exactly seven days lands its last step. A single first step that ends exactly on the last date or on seven days lands.".into(),
        &[
            "NonFiniteInput",
            "NonFiniteState",
            "NonPositiveDt",
            "InvalidSatellite",
            "RadiusOutsideCard",
            "OrbitOutsideCard",
            "EpochOutsideCard",
            "HorizonOutsideCard",
            "SpontaneousGeneration",
            "FirstLawBreach",
        ],
        vec![Advisory {
            name: "FLAG_EARTH_SHADOW",
            mask: satellite_lageos1::FLAG_EARTH_SHADOW,
            seals: true,
        }],
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
            ("flags", "FLAG_EARTH_SHADOW or 0"),
            ("proof", "Not written by the step."),
        ]),
    )
}

/// What a caller must do to a start state before this card takes it, in the
/// words of the card's header.
const SATELLITE_START_STATE: &str = "The published orbit files are in a frame fixed to the Earth's crust, which does not turn uniformly. A state must be turned into the card's frame, the true pole of the start epoch, before it is given to the card. That is done outside the engine. A crust-fixed state fed straight in is 21 to 22 m off within one revolution for that reason alone. `epoch_utc_s` in a request is UTC at the start of the run in seconds after 2000-01-01 12:00 UTC. The `epoch_utc_s` of a returned state is the UTC of that state: the start epoch plus `t_s`, worked out from the two at each state with one rounding and not kept as a sum that gains a step at a time. `t_s` counts from the start of the run. A final state handed back as a new start, with its `epoch_utc_s`, `position_m` and `velocity_m_s`, continues the same orbit, and the new run's `t_s` begins again at zero. `forecast_elapsed_s` in a request is the time since the state was taken from a published orbit: zero for a state turned from a published orbit, and the `forecast_elapsed_s` of a returned state for a state handed back. It is required. The card holds it, plus `t_s`, to seven days, so a forecast carried on through restarts stops where a run straight through stops. The card cannot check the number a caller states: a state stated as fresh that is not gets answers the comparison does not cover.";

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
        vec3_stated("sun_position_m", "m", "of the Sun from the barycentre of the Solar System, ICRF axes. No band of its own: every other body's band is measured from it. The centre of mass of the thirteen bodies must be no more than 120000 m from the origin; in the compared run, to the last range compared with measurement, it went from 98409 m to 114929 m".into()),
        vec3_stated("sun_velocity_m_s", "m/s", "of the Sun. The centre of mass of the thirteen bodies must move no faster than 0.01 m/s".into()),
        vec3_stated("mercury_position_m", "m", "of Mercury from the barycentre of the Solar System, ICRF axes. Its distance from the Sun is from 0.29 to 0.48 au, and within the distances the compared run went through, 0.30 to 0.47 au".into()),
        vec3_stated("mercury_velocity_m_s", "m/s", "of Mercury. The semi-major axis of its orbit about the Sun is from 0.29 to 0.48 au".into()),
        vec3_stated("venus_position_m", "m", "of Venus from the barycentre of the Solar System, ICRF axes. Its distance from the Sun is from 0.70 to 0.74 au, and within the distances the compared run went through, 0.71 to 0.73 au".into()),
        vec3_stated("venus_velocity_m_s", "m/s", "of Venus. The semi-major axis of its orbit about the Sun is from 0.70 to 0.74 au".into()),
        vec3_stated("earth_position_m", "m", "of the Earth from the barycentre of the Solar System, ICRF axes. Its distance from the Sun is from 0.97 to 1.03 au, and within the distances the compared run went through, 0.98 to 1.02 au".into()),
        vec3_stated("earth_velocity_m_s", "m/s", "of the Earth. The semi-major axis of its orbit about the Sun is from 0.97 to 1.03 au".into()),
        vec3_stated("moon_position_m", "m", "of the Moon from the barycentre of the Solar System, ICRF axes. Its distance from the Earth is from 340 000 km to 420 000 km, and within the distances the compared run went through, 356 000 km to 407 000 km".into()),
        vec3_stated("moon_velocity_m_s", "m/s", "of the Moon. The semi-major axis of its orbit about the Earth is from 340 000 km to 420 000 km".into()),
        vec3_stated("mars_position_m", "m", "of the Mars system barycentre from the barycentre of the Solar System, ICRF axes. Its distance from the Sun is from 1.35 to 1.70 au, and within the distances the compared run went through, 1.38 to 1.67 au".into()),
        vec3_stated("mars_velocity_m_s", "m/s", "of the Mars system barycentre. The semi-major axis of its orbit about the Sun is from 1.35 to 1.70 au".into()),
        vec3_stated("jupiter_position_m", "m", "of the Jupiter system barycentre from the barycentre of the Solar System, ICRF axes. Its distance from the Sun is from 4.8 to 5.6 au, and within the distances the compared run went through, 4.94 to 5.46 au".into()),
        vec3_stated("jupiter_velocity_m_s", "m/s", "of the Jupiter system barycentre. The semi-major axis of its orbit about the Sun is from 4.8 to 5.6 au".into()),
        vec3_stated("saturn_position_m", "m", "of the Saturn system barycentre from the barycentre of the Solar System, ICRF axes. Its distance from the Sun is from 8.8 to 10.3 au, and within the distances the compared run went through, 9.03 to 9.97 au".into()),
        vec3_stated("saturn_velocity_m_s", "m/s", "of the Saturn system barycentre. The semi-major axis of its orbit about the Sun is from 8.8 to 10.3 au".into()),
        vec3_stated("uranus_position_m", "m", "of the Uranus system barycentre from the barycentre of the Solar System, ICRF axes. Its distance from the Sun is from 18.0 to 20.4 au, and within the distances the compared run went through, 19.88 to 20.10 au".into()),
        vec3_stated("uranus_velocity_m_s", "m/s", "of the Uranus system barycentre. The semi-major axis of its orbit about the Sun is from 18.0 to 20.4 au".into()),
        vec3_stated("neptune_position_m", "m", "of the Neptune system barycentre from the barycentre of the Solar System, ICRF axes. Its distance from the Sun is from 29.5 to 30.7 au, and within the distances the compared run went through, 29.96 to 30.14 au".into()),
        vec3_stated("neptune_velocity_m_s", "m/s", "of the Neptune system barycentre. The semi-major axis of its orbit about the Sun is from 29.5 to 30.7 au".into()),
        vec3_stated("ceres_position_m", "m", "of Ceres from the barycentre of the Solar System, ICRF axes. Its distance from the Sun is from 2.4 to 3.1 au, and within the distances the compared run went through, 2.54 to 2.99 au".into()),
        vec3_stated("ceres_velocity_m_s", "m/s", "of Ceres. The semi-major axis of its orbit about the Sun is from 2.4 to 3.1 au".into()),
        vec3_stated("pallas_position_m", "m", "of Pallas from the barycentre of the Solar System, ICRF axes. Its distance from the Sun is from 2.0 to 3.6 au, and within the distances the compared run went through, 2.13 to 3.42 au".into()),
        vec3_stated("pallas_velocity_m_s", "m/s", "of Pallas. The semi-major axis of its orbit about the Sun is from 2.0 to 3.6 au".into()),
        vec3_stated("vesta_position_m", "m", "of Vesta from the barycentre of the Solar System, ICRF axes. Its distance from the Sun is from 2.0 to 2.7 au, and within the distances the compared run went through, 2.14 to 2.58 au".into()),
        vec3_stated("vesta_velocity_m_s", "m/s", "of Vesta. The semi-major axis of its orbit about the Sun is from 2.0 to 2.7 au".into()),
        scalar("energy_anchor", Role::Phase, "m^5/s^4", None, false, None, false, "E_anchor of the run's ledger: Newton's energy of the thirteen bodies times G, 1/2 sum mu_i v_i.v_i - sum_{i<j} mu_i mu_j / rho_ij, at the state the ledger started from. A returned state hands back its own; a new start gives its own E. Required".into()),
        scalar("booked_work", Role::Phase, "m^5/s^4", None, false, None, false, "W of the run's ledger: the work of relativity and of the Earth's oblateness, times G, since the ledger started. A returned state hands back its own; a new start gives 0. Required. E - energy_anchor - booked_work at the start must be within 1e-12 times 1.32e25, or the start is refused by the ledger's names".into()),
        vec3_stated("angular_momentum_anchor", "m^5/s^3", "L_anchor of the run's ledger: the angular momentum of the thirteen bodies times G, sum mu_i r_i x v_i, at the state the ledger started from. A returned state hands back its own; a new start gives its own L. Required".into()),
        vec3_stated("booked_torque", "m^5/s^3", "T of the run's ledger: the torque of relativity and of the Earth's oblateness, times G, since the ledger started. A returned state hands back its own; a new start gives [0, 0, 0]. Required. |L - angular_momentum_anchor - booked_torque| at the start must be within 1e-12 times 2.09e33, or the start is refused as AngularMomentumBreach".into()),
    ];
    finish(
        "aerospace",
        "solar_system",
        "The Solar System as thirteen point masses in free motion: the Sun, Mercury, Venus, the Earth, the Moon, the barycentres of the Mars, Jupiter, Saturn, Uranus and Neptune systems, and Ceres, Pallas and Vesta, under Newton's and the relativistic forces between every pair and the Earth's oblateness on the Moon, with the constants of JPL's DE440 and nothing fitted. The caller supplies every position and velocity. The comparison started from JPL's own state of 1999 and was made with range measurements that state was fitted to.",
        fields,
        floor,
        "the ceiling divided by 4096: Neptune moves 4.8 km. Under it a step is mostly rounding",
        gate,
        "the largest step the card stands behind, a locked constant. Measured with this card from the 1999 state over 2830 days against the same run at 1800 s, the largest distance at the end of a day, the step error at this step is 359 m for the Moon, 90 m for Mercury, 4.3 m for the Earth, 0.30 m for Venus, 0.21 m for Mars and under 0.2 m for each other body, the three asteroids among them",
        None,
        "No load. A run segment carries an empty load object. Each body's distance band, its orbit band, the distances the compared run went through, the speed of the centre of mass and the distance of the centre of mass from the origin are applied to the incoming state and to every landing. A body whose distance from its primary is outside the distances the compared run went through up to the last range compared with measurement, 16.2 years after the 1999 state, each end rounded outward to 0.01 au (Mercury 0.30 to 0.47 au, Venus 0.71 to 0.73, the Earth 0.98 to 1.02, the Moon 356 000 to 407 000 km, Mars 1.38 to 1.67, Jupiter 4.94 to 5.46, Saturn 9.03 to 9.97, Uranus 19.88 to 20.10, Neptune 29.96 to 30.14, Ceres 2.54 to 2.99, Pallas 2.13 to 3.42, Vesta 2.14 to 2.58 au), incoming or landing, is refused as DistanceOutsideComparedRun: the run swept the whole orbit of each inner body, of Jupiter and of the asteroids, and only part of the orbits of Saturn, Uranus and Neptune. From 16.2 to 21 years the run was compared only with DE441, a calculation, and is not in these distances. No date is held; carried on past the last measured range, day 5,920.4, in steps of 1800 s, the compared run is refused 5,934.6 days from the 1999 state, Saturn farther from the Sun than 9.97 au. Two bodies at exactly one point are refused as BodiesCoincide. Two bodies other than the Earth and the Moon closer than 0.26 au, or two of the three asteroids closer than 0.38 au, incoming or landing, are refused as BodiesTooClose: the compared run's closest such pair was Mercury and Venus at 0.2631 au, and its closest two asteroids Ceres and Vesta at 0.3885 au. A centre of mass more than 120000 m from the origin, incoming or landing, is refused as OriginOutsideCard: the compared run's was within 114929 m. The ledger of the incoming state, energy_anchor, booked_work, angular_momentum_anchor and booked_torque, is closed at the start as at a landing and refused by the ledger's names when it is out of balance. The axes are checked by the one thing the card can tell them by: a state whose Earth-Moon barycentre does not orbit the Sun in a plane 21 to 27 degrees from the z axis is refused as AxesOutsideCard, which names a state in ecliptic axes; a turn about the z axis is not seen.".into(),
        &[
            "NonFiniteInput",
            "NonFiniteState",
            "NonPositiveDt",
            "InvalidSystem",
            "DistanceOutsideCard",
            "OrbitOutsideCard",
            "DistanceOutsideComparedRun",
            "BodiesCoincide",
            "BodiesTooClose",
            "FrameOutsideCard",
            "OriginOutsideCard",
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
    match state {
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
        | "surface_state_of_charge" | "radius_over_final" | "slip_ratio" => "1",
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
        _ => unreachable!("a packed quantity with no unit"),
    }
}

/// The state quantity that a card's `force_torque` slot holds.
fn force_torque_state(card: &str) -> &'static str {
    match card {
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
        "link_gothenburg_10010" => "rain_rate_margin_mm_h",
        "solar_system" => "earth_mars_distance_m",
        "tire_eve_r19_ice" => "longitudinal_force_n",
        "accumulator_sbo450" => "gas_force_n",
        "los_jwst_fine_guide" => "nis",
        "tokamak_jet_dte1" => "loss_power_w",
        "fingerpad_p02_glass" => "torque_n_m",
        _ => unreachable!("a card with no force_torque quantity"),
    }
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
const CRATER_RESULT: &str = "Compared on 2026-10-09, and by the rule fixed beforehand the comparison with Ryugu failed. In the laboratory both lines were met: fitted on the nine craters made at 1 g, the law put each of the four centrifuge craters inside its 95 percent prediction interval, 6.9 to 9.5 percent low, 7.9 percent on average against a mark of 10. Those intervals are +/- 31 to 38 percent, so the first laboratory line could not have failed. The second, 7.87 percent, was foreseeable from Schmidt's published line, which was fitted through the centrifuge craters and was known beforehand, and it is fragile: with any one of four of the nine opened shots left out it is 10.4 to 11.8 percent, a miss. The four errors are all of one sign, a real bias of the nine-shot line. On Ryugu the line was missed: the card stops at 8.571 m and the measured apparent radius is 7.25 m, so the card is 1.32 m, or 18.2 percent, too large, against an allowance of 0.97 m (0.81 m from the fit, 0.40 m from the measurement, 0.34 m from the local gravity). 7.25 m is the apparent radius, which is what the law predicts: the apparent diameter is 14.5 +/- 0.8 m and the rim-to-rim diameter 17.6 +/- 0.7 m. The same team's submitted manuscript gave 6.5 +/- 0.4 m by another reduction. The published team's own range for sand scaling, 6.9 to 7.7 m by the dry-sand line of Housen and Holsapple, \"agrees well with the apparent radius\". The radius at which the engine stops was scored against the closed form of the thirteen-shot law with a mark of 1 part in 10^9, and that line was missed: the card scored on 2026-10-09 typed the law's two constants to six figures and stopped at 8.571304624905444 m, 3.45 parts in 10^8 from the closed form of the fitted law, 8.571304329477925 m. The record said the line was met because it compared the engine with the closed form of the same six-figure constants. Since engine 0.7.0 the card types the constants as the fit gave them and stops at 8.571304329477922 m, 4 parts in 10^16 from the closed form. That agreement is by construction: on the stopping step the card sets the radius to a typed constant, so it shows only that the constant was typed as computed. The miss on Ryugu is the same either way, 1.321 m. Without a mark: the two lines Schmidt published for Ottawa sand and for number 24 quartz sand give 8.60 m and 8.89 m; the fitted exponent, 0.1661, is inside the 0.165 +/- 0.005 that Gault and Wedekind measured between 0.073 and 1 g; the card stops 400 s after the impact, where the crater was seen to grow for about 200 s, the team's submitted manuscript says growth would stop by about 300 s, and the whole event lasted about 500 s; the constant that would put the law on Ryugu's crater is 0.737 against the 0.871 fitted in sand. The binary that made the scored run (SHA-256 425ef2b93d69be6f05da11eaf6ad4c4debecc37d548672950465d68ef2bf3b04) was overwritten by a later build; two later builds, 05efbd06… and 043d5765…, returned the answer byte for byte (evidence/checks/second_party/V-30/), and engines 0.4.0 to 0.6.0 give the same run proof. Engine 0.7.0, with the constants retyped, does not: its stop, its states and its proofs differ from the scored run in the eighth figure and beyond. A second scoring of 2026-10-09, on the same machine, found the same numbers: evidence/checks/second_party/V-30/.";

/// What a card that has not been compared with a measurement says.
pub(crate) const NOT_COMPARED: &str = "not compared with a measurement";

/// The pass line of the InSight capsule's validation record.
const CAPSULE_INSIGHT_PASS_LINE: &str = "Two sets, both fixed before the card was compared with any flight row (protocol SHA-256 ddafbbe8299da589e95a674bb31150f869d0f85d21415036bdbedfc4d7fa0a54, Amendment 2 SHA-256 9bb5b953e4ae325eb954fb3979b3942a08dc19bd5ff80ae2f6bbeda37b137146). The first lines are NASA's own printed spread for its own forecast: peak deceleration within 0.5 g, each time within 6.946 percent of elapsed time (7.68 s on the peak, 10.42 s at 150 s, 15.1 s at the parachute), radius at the parachute within 3.5 km, Mach number there within 0.2. The second set is built from the stated spread of the card's own inputs, twice the root sum of squares of the shift each input gives when moved by one standard deviation: the air density, the drag coefficient and the lift.";

/// The result of the InSight capsule's validation record: the comparison of
/// 2026-10-09 with the rows held back, as scored from the engine's frames.
const CAPSULE_INSIGHT_RESULT: &str = "Compared with the held-back rows on 2026-10-09: two runs, each against both sets of marks. With the lift measured on the opened rows, a lift over drag of -0.031: all four first lines met and all six stated-spread marks met. The flight's speed at parachute deployment was reached 1.31 s late and its radius 0.58 s early, against lines of 15.1 s; at the flight's deployment time the radius was 114 m low against a line of 3.5 km, and the Mach number 1.501 against 1.522. Over the 58 whole seconds held back the speed differed by 7.2 m/s root mean square, the radius by 245 m and the deceleration by 0.031 g. With no lift: none of the four first lines met, 17.1 s and 22.8 s late, 4.24 km high and Mach 1.91; all six stated-spread marks met, because those marks carry a lift the capsule could take either way and are wide: 6.8 km on the radius at the parachute, 27 s and 37 s on the two times and 0.72 on the Mach number, which is no forecast. On the opened rows, with the lift: all four first lines met, the peak 7.95 g against 8.12 g and 4.0 s early, and one stated-spread mark missed, the time of the peak, 4.02 s against 3.27. With no lift: the peak missed its first line, 7.60 g against 8.12 g with a line of 0.5 g, and the 150 s radius was reached 17.0 s late against 10.42; two stated-spread marks missed, the peak, 0.517 g against 0.489, and its time, 5.81 s against 1.82.";

/// The limits of the InSight capsule's validation record.
const CAPSULE_INSIGHT_LIMITS: &str = "The run that meets the first lines takes one number from the flight: its lift, a single number measured on the opened 150 s and held through the run. The run that takes nothing from the flight but its starting state misses them. So the card does not forecast an entry whose lift is not known; it reproduces this entry given its lift, and the 59 s it had not seen came out as stated. Those 59 s do not test the lift: the run is already right at 150 s, where the lift was measured, and on the engine before 0.7.0, which carried a run on to 4 km, it then met the four first lines with any lift from -0.08 to 0.08 over the 59 s; the flight's own lift over those 59 s was -0.016, half the number the run holds. They test the air below 18 km and the drag below Mach 6, to which nothing was fitted. The 114 m at the parachute is finer than the flight record's own one standard deviation there, 694 m, and is not the card's accuracy. The air and the lift were both chosen with the opened rows in view, so agreement on those rows is a check and not a test. The card's air is the mean of soundings one to seven sols old: between 36 km and 65 km it is 8 to 13 percent denser than the density reconstructed from the flight, more than its own stated spread allows, and the peak of the load comes 4 s early for it; below 18 km it is within 7 percent. The first lines are NASA's spread for its own forecast and are wide: 0.5 g is 6 percent of the peak. The comparison was not blind in the full sense: NASA's printed values at the peak and at the parachute were known. The capsule's sideways lift, wind, and the rise of drag in thin air are not on the card. One entry of one capsule, down to parachute deployment only. From 0.7.0 the card refuses below 5700 m or Mach 1.46, the lowest state the run with the flight's lift reached in its scores (5793 m, Mach 1.470) rounded outward, and every one of that run's scores is on landed steps. The run with no lift now stops at 227.3 s, 7.06 km up, after it reaches the flight's speed at the parachute and before it reaches the flight's radius there: its 22.8 s on that radius was scored on 0.3.0, which carried it on to 4 km and Mach 1.20, and is not on a landed step now. Of the second scorer's 20 runs with other lifts, 13 have a score on a state below 5700 m or Mach 1.46, made on 0.3.0 and not on a landed step now. The same aeroshell on Phoenix flew with its lift pointing the other way and has not been run. The runs were scored again from the protocol alone by a second scorer on the same machine on 2026-10-09, with the same verdicts (evidence/checks/second_party/V-23/); not yet repeated on another machine or by anyone outside the company. The held-back runs were made with a debug build of the gateway.";

/// The solar_system card's validation record.
const SOLAR_SYSTEM_DATA: &str = "Round-trip light times between stations of NASA's Deep Space Network and spacecraft at Mars, Mercury, Jupiter and Saturn, as reduced and published by JPL's Solar System Dynamics group (ssd.jpl.nasa.gov/planets/obs_data.html; Konopliv et al. 2011, Icarus 211, 401; Park et al. 2017, AJ 153, 121; Hees et al. 2014, Phys. Rev. D 89, 102002). The constants are those of DE440 and the start state is DE441's, through JPL Horizons: Park et al. 2021 for DE440 and DE441, Giorgini et al. for Horizons. Nothing is fitted. The Mars Global Surveyor ranges of 1999 to 2006 were opened and used to choose the card's structure and to set the pass lines. Held back: Mars Odyssey and Mars Reconnaissance Orbiter (Mars), MESSENGER (Mercury), Juno (Jupiter) and Cassini (Saturn).";
const SOLAR_SYSTEM_PASS_LINE: &str = "Fixed before any held-back measurement was read (protocol SHA-256 2d79e65df9fb8ad036444896e776e98d18cb98fc7b582f861747f27d79fd9679). For every measurement of a held-back file, measured minus computed as a one-way distance at most B x max(1, T / 7.75 years)^2, T counted from the start: B is 0.70 km for Mars, 1.0 km for Mercury, 1.5 km for Jupiter and 0.5 km for Saturn. One run from the 1999 state in steps of 1800 s.";
const SOLAR_SYSTEM_RESULT: &str = "Compared on 2026-10-09 with the held-back measurements, in one run of 7,670 days from the 1999 state, none refused. Two files met their lines, two missed and two comparisons were not made. The measured range residual equals the card's distance from JPL's DE441 ephemeris to about 5 m, so every verdict follows from DE441 without a measurement. What the result supports is that this simpler model, carried forward from JPL's own 1999 state, stays within 0.69 km of JPL's Earth-to-Mars distance over 7.75 years and 1.9 km over 21, within 2.7 km for Mercury, 3.2 km for Jupiter and 2.1 km for Saturn. The registered figures, file by file: Mars Odyssey up to 2006, the opened span (2,389 ranges): met, 270 m rms and 695 m at most against 700 m. Mars Odyssey after 2006 (4,939 ranges, out to 15.0 years): missed, 612 m rms and 1,312 m at most; 44 ranges were outside the line, all in mid-2007, by at most 8 percent. Mars Reconnaissance Orbiter (1,993 ranges, out to 15.0 years): missed, 619 m rms and 1,314 m at most; 34 ranges outside, all in mid-2007, by at most 10 percent. MESSENGER at Mercury (813 ranges, 12.3 to 16.2 years out): met, 1.42 km rms and 2.08 km at most against a line of 2.5 to 4.4 km. Juno at Jupiter (4 ranges) and Cassini at Saturn (196): not made, because the check on the reduction fixed beforehand failed; with DE441's own positions the reduction left 20 km and 10 km rms on those two files. The registered reduction left out the station term of TDB - TT, up to 48 m one way and 22 m rms. Under a reduction that has it the verdicts are the same: Odyssey up to 2006-10 met, 689 m at most against 700 m; Odyssey after missed, 45 of 4,939 ranges outside, the worst 1.095 of the line; Mars Reconnaissance Orbiter missed, 33 of 1,993 outside, the worst 1.093; MESSENGER met, 2,054 m at most, 0.640 of its line. The Odyssey line was met thinly: inside by 5 to 11 m of a line that is 678.3 m rounded up to 700, and with 678.3 m, 11 to 13 ranges are outside. The causes: the Mars error is the 340 asteroids the card leaves out, 730 of the 933 m at the misses of mid-2007, and no single asteroid does it; Mercury's is the Sun's oblateness, 2,591 of 2,661 m; the Earth's remaining 69 to 183 m is the Moon's error of 6 to 16 km, from the figure of the Moon, the tides and the Earth's figure beyond J2. Newton alone would be 351 km off by 2006-10. The engine's daily positions were also to agree with the outside model to 10 m: every body was within 3 m but the Moon, 14.6 m apart after 21 years, a miss, with the two growing apart smoothly from nothing as rounding does. It is rounding: two 64-bit runs of these equations that differ in the order of operations, or by one unit in the last place at the start, end 5 to 24 m apart on the Moon after 21 years, so the 10 m line could not have been held. Against a 32-digit run the engine is 9.5 m off on the Moon and 0.92 m on the Earth. The truncation of the 1800 s step over 21 years is 78 m on the Moon, 17.9 m on Mercury and under 1 m on the Earth. A second scoring of 2026-10-09, on the same machine, with an independent reduction and an independent integrator, found the same verdicts: evidence/checks/second_party/V-31/.";
const SOLAR_SYSTEM_CAVEATS: &str = "The start state is JPL's DE441 through Horizons. DE441 is a calculation whose start values JPL fitted to observations, these range measurements among them, so the start already carries information from the held-back measurements; what the comparison tests is whether this card's far simpler dynamics carry that one state forward for up to 16.2 years to where the stations measured (the run went on to 21 years, compared only with DE441). The held-back Odyssey ranges up to 2006 are interpolation: the same planet over the opened span. Odyssey and Mars Reconnaissance Orbiter after 2006 are extrapolation in time. Mercury, Jupiter and Saturn had no opened measurement, and their pass lines rest on a comparison with DE441, a calculation. The light time is worked out from the card's positions outside the engine: station position from the IAU 2006/2000A Earth orientation and the IERS series, the Sun's relativistic delay, no solar plasma. With DE441's own positions that reduction left 18 m rms on the opened file, the floor of the comparison. That floor of 18 m is the reduction's omission of the station term of TDB - TT; a reduction that has the term has a floor of about 5 m, and 1.3 m at Mercury. Left out of the card: every other asteroid, Pluto, the Sun's oblateness, the figure of the Moon, every tide. The square in the pass line was a guess at how the error grows beyond the opened span; the Mars misses lie just past that span, where the line had barely risen. The square is not physical: the envelope of the error grows about in proportion to time (T^0.96), so the line is too tight just after the opened span and twice too loose at 15 years. The Juno and Cassini files as published leave about 20 km and 10 km with JPL's own ephemeris through this reduction. That is consistent with a station delay left twice in the round-trip time; this is an inference from DE441 and is not stated in the files' headers. No registered comparison was made for Jupiter or Saturn, and after that correction the Cassini file still fails the check on the reduction, at 590 m rms. FrameOutsideCard checks the speed of the centre of mass and nothing else; OriginOutsideCard checks where the centre of mass is, no more than 120000 m from the origin. A state moved bodily is the same motion in other numbers with other rounding: moved by 6e13 m it put the Moon 98.5 m from the unmoved run after one year. The axes have a check of their own, AxesOutsideCard: the orbit of the Earth-Moon barycentre about the Sun must be inclined 21 to 27 degrees to the z axis, as the ecliptic is to the ICRF equator. In DE441, as Horizons serves it, that angle is 23.43 to 23.45 degrees from 1900 to 2100 and falls from 24.94 to 22.56 degrees from 9999 BC to AD 9999; a state given in ecliptic axes instead of ICRF axes has about 0 degrees and is refused, where through engine 0.5.0 it was accepted with the Earth's pole in the J2 term 23.4 degrees wrong. What that check cannot tell: a turn of the axes about the z axis, by any angle; the equator of another date in place of the ICRF's, which keeps the angle and moves the true pole from the z axis by the precession of those years; and two bodies whose distance bands overlap, Ceres and Pallas, given in each other's place, which are taken without a refusal. Two bodies at exactly one point are refused by a name of the card, BodiesCoincide, before anything divides by the distance between them; through engine 0.5.0 they were refused with no number and no name of the card. Two bodies closer than the compared run's closest pair of their kind are refused as BodiesTooClose: any pair but the Earth and the Moon under 0.26 au, any two asteroids under 0.38 au; through engine 0.6.0 Pallas 100 km from Ceres's centre, inside Ceres, was accepted and answered with a proof. A body where the compared run never had it is refused as DistanceOutsideComparedRun: each body's distance from its primary must lie within the distances the run went through up to the last range compared with measurement, 16.2 years after the 1999 state, each end rounded outward to 0.01 au. From 16.2 to 21 years the run was compared only with DE441, a calculation. The run to 16.2 years swept the whole orbit of each inner body, of Jupiter and of the asteroids, and only part of the orbits of Saturn (9.03 to 9.97 au), Uranus (19.88 to 20.10 au) and Neptune (29.96 to 30.14 au); through engine 0.6.0 a state with those three anywhere in their bands was answered with a proof. This limit also ends long runs, since the card holds no date: carried on past the last measured range, day 5,920.4, in steps of 1800 s, the compared run is refused 5,934.6 days from the 1999 state, Saturn farther from the Sun than 9.97 au. The scoring was run in the same pass as the check of the engine against the outside model, not after it as the protocol had it. Every result is of this one start; no other start state has been scored. What the card takes is the envelope of that one run up to the last range compared with measurement: each body within the distances the run went through, no pair closer than the run's closest of its kind, the centre of mass within 120000 m of the origin; a start state inside that envelope but not on the run, Neptune at another longitude for instance, was not itself compared.";

/// The result of the cantilever's validation record: the comparison of
/// 2026-10-09 with the half of the held-back recording that had been kept
/// unread.
const CANTILEVER_RESULT: &str = "Compared with the held-back half on 2026-10-09, and by the rule fixed beforehand the comparison failed: three of the four lines were met and the amplitude line was missed. On the eleven blocks of eight periods between 12 s and 24 s the amplitude error was 7.03 percent against a line of 5.0. The frequency error was 0.0031 Hz against 0.010 and the timing error 0.021 of a period against 0.20, and both the amplitude error and the frequency error were under one third of a plain damped oscillator's, which missed by 40.8 percent and 0.048 Hz. The card's swing is too large in every block and the gap widens, from 3.6 percent at 13 s to 11.7 percent at 23 s: it dies away too slowly once the swing is small.";

/// The result of the link's validation record: the comparison of 2026-10-09
/// with the month that had been kept unread.
const LINK_RESULT: &str = "Compared with the rain events of August 2015 on 2026-10-09, in one run of each event through the engine, and all three lines were met. Root mean square error 0.581 dB, against 0.90. That is 0.585 of the error of P.838 alone, 0.994 dB, against 0.80. The summed attenuation was 1.069 of the measured sum, inside 0.75 to 1.25. No attenuation at all scores 1.621 dB. The rule fixed beforehand finds 21 gauge events in August: eleven are under 1.0 mm and by the rule are not scored, nine were scored and none was refused: 773 five-minute blocks; and one of 15.8 mm that starts 2015-08-31T17:37Z was not scored because its window runs past the end of the archive. A second scorer scored that event to the last minute of data, after the fact: the month then reads 0.622 dB, 0.528 and 0.974, all inside the lines. In two of the nine events P.838 alone was closer than the card, 0.644 against 0.675 dB and 0.346 against 0.400 dB. August's rain reached 54 mm/h, inside the 84 mm/h fitted. Its dry levels, 52.60 to 52.90 dB, were above every fitted event's, 51.75 to 52.55 dB, and inside the card's limit. The card's five-minute peak is below the measured five-minute peak in seven of the nine events, by up to 2.2 dB. On the one-minute values the card returns, its peak is below the measured peak in six of the nine events, by up to 2.8 dB, and above it in three, by up to 3.3 dB. In light rain under 2 mm/h the card is about 0.3 dB high. The link leads the gauge by one to two minutes: moving the link series one minute later lowers the error from 0.580 to 0.516 dB, and this is not corrected. The other direction of the same link, sublink 588, scored after the fact with the same constants, does not transfer under the protocol's rules: 0.794 dB, no better than P.838 alone, with a summed attenuation of 2.30 of the measured sum, because that receiver's dry level wanders by more than 1 dB within a dry day; in the events where its level was steady the card does about as well as on the scored direction. A second scoring of 2026-10-09, on the same machine and from the scorer's own cut of the archive, found the same verdicts: evidence/checks/second_party/V-28/. The binary that was scored served this record with the words \"not yet compared with the held-back month\"; this text was written afterwards and changes no number the engine computes.";

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
pub(crate) const REACTOR_SPERT3_FUEL_RESULT: &str = "Compared with the 30 held-back tests on 2026-10-09, and by the rule fixed beforehand the comparison failed in each of the three groups and for the 30 together: of the 24 lines of the three groups 18 were met and 6 were missed, and of the 8 lines for the 30 together 5 were met. Each triple is root mean square, worst and mean, in units of the report's standard deviation; the times are relative errors. Cold group, 10 tests: peak power met its three lines (0.514, 0.937, +0.204), energy released to the peak met its three (0.540, 1.087, +0.023), and time of the peak missed both of its lines: root mean square 5.22 percent against 5 and worst 12.24 percent against 10. That miss is at the floor of the measurement: in this series the report's own plots and its own table disagree on the time of the peak by about 3 to 6 percent. 260 degree Fahrenheit group, 13 tests, the one group with nothing fitted to it: peak power met its three lines (0.643, 1.051, and a mean of +0.479 against a line of 0.5 in size, met by 0.02), energy met its three (0.846, 1.334, -0.152), and time of the peak missed both: root mean square 8.46 percent and worst 14.88 percent, twelve of the thirteen peaks early. That miss is about twice the floor of the measurement there and is the card's. 500 degree Fahrenheit group, 7 tests: peak power met its three lines (0.796, 1.650, and a mean of -0.490 against 0.5, met by 0.01), time of the peak met both (4.49 and 7.45 percent), and energy met one line and missed two: root mean square 1.336, worst 3.212 against 3, and mean -0.890 against 0.5 in size; the card's energy is low on all seven tests. The 30 together: peak power met (0.644, 1.650, +0.161); energy met two lines and missed one (0.908, mean -0.266, worst 3.212); time of the peak missed both (6.69 and 14.88 percent). The scored step was 0.43 ms, the card's ceiling, where the protocol said 0.5 ms, a step above that ceiling, which the door refuses. A second scoring on 2026-10-09, written from the protocol by a second agent with its own reading of the report's tables and a model of its own (evidence/checks/second_party/V-25/), on the same machine, reached the same figures and the same verdicts.";

/// The limits of the fuel-temperature reactor's validation record.
pub(crate) const REACTOR_SPERT3_FUEL_LIMITS: &str = "A miss, and it stays on the record. Four constants are fitted on 28 tests: the power at the start, 11.6113 W; the strength of the Doppler feedback, 0.0371601 dollar per square root of a kelvin; the heat capacity of the fuel, 0.0260616 MW s per K; and one weight on the water-temperature feedback for all three bands, 0.28586. The starting power is a fitted effective value and not a measurement: across what the second scoring varied it came out at about 7 to 11 W, the report's 50 W is excluded under this model, and the difference from the report is unexplained. The insertion ramp is the report's design figures, a constant rod acceleration and a constant worth per inch, and not a recorded rod motion; the recorded power histories prefer a somewhat faster insertion, and 10 to 15 ms of fixed-time error remains, where the card is late on the fastest tests. The fuel temperature is compared with no measurement, because the report measures none: fuel_temperature_rise_k is the temperature of one fitted lump, about one fourteenth of the heat capacity of the whole fuel, and was 69 to 402 K at the peak on the held-back tests; it is tested only through what it does to power, energy and time, and it is not a temperature a thermocouple in the fuel would read. Heat flow from fuel to water is left out. The delayed-neutron constants are the report's values stated for 500 degrees Fahrenheit, used in all three bands. The ten cold-startup tests held back here had been scored once before, by the first card of this reactor: they are held back from this card's fitting and are not unseen. Before the marks were fixed, the whole of Table V, its held-back rows included, had been printed to the fitting agent's screen, the plotted time of peak of held-back test 70 had been read, and the power history of one held-back cold test had been looked at; that no model had been run on a hot test rests on the builder's word and on file times of one machine, which agree with it and cannot prove it. The report's standard deviations share a part from the power calibration, so the errors of two tests are not independent. The conservation residual names a wrong method and is not a bound on accuracy. On the fastest run of each band, by a ledger the second scorer wrote, it named explicit Euler, the midpoint and Heun's second-order methods, Kutta's third-order method, Runge-Kutta with one stage weight wrong by 1e-3 and by 1e-5, a delayed group dropped from the power equation, the Doppler strength wrong by 1e-3 and by 1e-5, a group fraction wrong by 1e-4, and Runge-Kutta at two and at three times the step ceiling; it did not name a group fraction wrong by 1e-6 or Runge-Kutta at 1.5 times the ceiling. Its limit, 3e-7, is calibrated on the card's own method, at 11 times the largest residual that method leaves, and the residual reads the true error of the state low by about 11 to 15 times: on two of the three fast runs the true error, 3.3e-7 and 3.8e-7 of the neutron population, is above the limit while the residual is 2.3e-8 and 2.6e-8. A run is one burst with one reactivity and one flow rate; a load changed during a burst is refused by name. Between the three bands nothing was measured and the card refuses. The card stops at the first power maximum: hot standby, operating power, boiling, void, pressure and any other core are not on the card. The reactivity is the report's reactor period converted by the caller with the band's own generation time; the card takes dollars. One party wrote the card and ran it; the second scoring was on the same machine and did not rebuild the engine.";

/// The result of the tyre's validation record: the one scoring of
/// 2026-10-09 on the 60 held-back runs that hold a scored slip, of the 90
/// held back, as its record states it.
pub(crate) const TIRE_EVE_R19_ICE_RESULT: &str = "Ninety runs were held back. On 2026-10-09 the card was compared with the 60 of them that hold a scored slip, as 60 cells of three repeats each, and all five lines fixed beforehand were met; the other 30 held-back runs step through slips of 0 to 3 percent, which the protocol does not score. Root-mean-square error of the friction ratio: 0.034 over the 60 held-back cells against a line of 0.045, 0.040 over the 24 cells at 2.5 bar and 0.029 over the 36 cells at 6.5 kN. A ratio of 0.1 that uses no measurement of this tyre gave 0.122, and the opened runs' geometric mean with no slip term (least squares on the logarithm) gave 0.036. The mean measured ratio was 0.218, and the card read high by 0.019 on average. Two held-back conditions taken alone were above 0.045, traction at 2.5 bar at 3.5 kN and at 5 kN, with errors of opposite sign; no line was set on a single condition. One held-back run, number 153, is a faulty record: it rolled for about 90 s like the others, and its channels are on other scales. Its vertical force reads 1,849 N at a nominal 3,500 N, and its wheel angle spans 0 to 11.75 where every other file spans 0 to 360. It was scored with the rest, as the protocol says; without it the three errors are 0.031, 0.034 and 0.029, and those are the figures that describe the tyre. The card read high on all six conditions at 6.5 kN, by 0.023 on average. On the 48 opened cells the error was 0.030. The binary that made the scored runs (SHA-256 7a2f3cc8e75aadbbdaf4f74289803ddf1149dbe882d94cadc96f668061b943c7) was overwritten by a later build, which returned all 108 saved answers byte for byte. Since engine 0.7.0 a request states slip_held_s, so those saved requests are refused as they were sent. A second scoring of 2026-10-09, on the same machine and on the scorer's own reduction of the raw files, found the same verdicts: evidence/checks/second_party/V-33/.";

/// The result of the line-of-sight card's validation record: the comparison
/// of 2026-10-09 with the held-back visits, as its research record states
/// it.
pub(crate) const LOS_RESULT: &str = "Compared with the held-back visits on 2026-10-09, and by the rule fixed beforehand the comparison passed: all five marks were met on both axes. The marks are on quiet frames. The card as it was scored answered every frame of a file, and on a whole file the outcome was the other way. Over all 973,388 measured held-back frames, quiet and disturbed together, the card's error one frame ahead was 1.35 on x and 1.26 on y of the error of taking the last centroid, and the squared error over the variance it states was 3.9 and 3.0. Disturbances are in 121 of the 134 files and make up 18.3 percent of the measured frames. The card's own whole-file figure on x, nis_sum_x over measured_frames in the final state, lies in 0.5 to 2.0 in only 39 of the 134 files. The card as scored marked the 5.8 percent of frames above 5 mas with a flag and answered them, and answered the 30 s after them with no mark. It now refuses those frames; what that gives on the same files is at the end of this record. On the 795,021 quiet frames of the 134 held-back files, one frame ahead, the card's error was 0.510 mas on x and 0.472 mas on y, root mean square, against 0.644 and 0.584 for taking the last centroid (0.790 and 0.807 of it) and 0.660 and 0.748 for taking the lock point (0.772 and 0.631 of it). The mean of the squared error over the variance the card states was 1.004 and 0.986, the correlation from one frame to the next 0.074 and 0.074, and in every one of the 134 files that mean was between 0.5 and 2.0: from 0.68 to 1.27 on x and from 0.70 to 1.75 on y. All 134 runs completed and none was outside the card. Inside the disturbances, 178,367 frames within 2 s before and 30 s after a centroid more than 5 mas from the lock point, the card was worse than taking the last centroid: 2.03 mas against 1.00 on x and 1.57 against 0.78 on y, and the variance it stated was too small by a factor of 17 on x and 12 on y. No mark was set on those frames. A random walk with the same power-law sensor noise, three constants an axis fitted on the same April data, meets all five marks, at 0.808 and 0.837 of the last centroid's error. The card's two modes and twelve further constants lower the error by 2 and 4 percent: 0.510 and 0.472 mas against 0.521 and 0.489. The marks do not test the mode frequencies: with both moved 20 percent up or down every mark is still met. The error is near the floor the sensor's noise sets. The sensor-noise law is what carries the variance marks: a second-order autoregression with one variance misses the per-file mark. The y noise exponent was not reproduced on the held-back guide stars: 1.045 fitted there against the card's 1.273, where x gave 1.134 against 1.151. Inside the tested count rates the two y laws differ by +27 to -21 percent in variance, which the variance mark cannot resolve. With a lock point that uses no future, the median of the first 10 s or a running median of past frames, the last-centroid, variance, correlation and per-file marks still hold, each number within 0.04 of the scored one, and the lock-point mark is no longer a test. Held-back files longer than 10,000 frames were cut at the gateway's limit: 43 files, which lose 12 percent of the set's frames. The second scorer's own filter on whole files still meets the marks. A second scoring of 2026-10-09, on the same machine and with the scorer's own FITS reader, found the same numbers: evidence/checks/second_party/V-35/. All of that is the registered result, of the card as it then was. What follows was done after the scoring and is not a blind test (amendment 1 of the research record, SHA-256 e0e4b86fcc18c2cd868f587627a0ac2ed0d5dd692292b1febbcc6358976dd849). The card was changed to refuse where it is wrong: a centroid more than 5 mas from the lock point, a centroid more than four stated standard deviations from where it was expected, and a start less than 30 s after either. The three numbers are the protocol's own and none was fitted to a held-back file. The second rule was decided on the fitted files of April 2022: there, in the 30 s after a centroid beyond four standard deviations, the x axis missed four of the five marks (0.921, a squared error over stated variance of 1.641, a correlation of 0.272, 74 percent of files), and on the held-back files three (0.933, 1.513, 0.316); such a centroid came 31 times as often on x as a right variance gives on the fitted files and 21 times on the held-back files. The 134 held-back files run again as the card now runs them, each file as a chain of runs that end at a refused centroid and start again 30 s later with nothing known, with the registered lock point: of 973,388 measured frames the card answered 721,727, 74.1 percent, in 428 runs, and refused 251,661: 56,910 centroids above 5 mas, 305 beyond four standard deviations, where a right variance gives about 91 by chance, and 194,446 in the 30 s after one. 130 of the 134 files hold a refused frame. Over the answered frames the five marks, x and y: 0.787 and 0.805 of the last centroid's error, 0.779 and 0.640 of the lock point's, a squared error over stated variance of 0.987 and 0.970, a frame-to-frame correlation of 0.064 and 0.072, and that mean between 0.5 and 2.0 in 100 and 100 percent of the files with 500 answered frames or more. Beside the registered 0.790 and 0.807, 0.772 and 0.631, 1.004 and 0.986, 0.074 and 0.074, 100 and 100 percent on the 795,021 quiet frames, which were chosen with 2 s of look-ahead that the card does not have. The card's own figure, nis_sum_x and nis_sum_y over measured_frames in the final state of a run, lies in 0.5 to 2.0 on both axes in every one of the 312 runs that landed 500 frames or more, where on x it did in 39 of the 134 whole files. The run through the engine and the same chain in plain Python answer the same frames, and their numbers agree to 2.4e-7 mas (evidence/research/pointing_jwst_fine_guidance/analysis/out/11_enforced/).";

/// The one line an agent sees in the list for the finger pad: what it is,
/// that its constants are fitted to the finger's own trials, and how the
/// comparison stands.
pub(crate) const FINGERPAD_P02_GLASS_SUMMARY: &str = "One human finger pad pressed on flat glass at 2 N (1.93 to 2.07 N) and twisted about the normal at 5 to 50 degrees a second until it slips, the right index finger of participant P02 of the torsion experiment at UCLouvain: the torque, the stuck fraction of the contact, the work and the heat, from eight constants fitted to this finger's own trials. Compared on 2026-10-09 with its held-back trials at 1 N, 10 N and 100 degrees a second and missed: the slip torque was off by +9, +29 and -13 percent against a mark of 5.6 percent, and those conditions and every other load are now refused by name. What is left is the one block of trials the fit took its level from: there this finger slipped at 10 to 11 N mm and the card gives that to about 8 percent, on trials it was fitted to, which is not a test. Another block of the same finger differs by 13 to 15 percent, another finger by about a fifth.";

/// The result of the finger pad's validation record.
pub(crate) const FINGERPAD_P02_GLASS_RESULT: &str = "Compared with the 30 held-back trials of this finger on 2026-10-09, and by the rule fixed beforehand the comparison failed: of the twelve lines three were met and nine missed, and the slip torque was missed at all three held-back conditions. Slip torque, card against the mean of ten trials: 6.03 against 5.51 N mm at 1 N (+9.4 percent), 35.63 against 27.55 at 10 N (+29.3 percent), 10.15 against 11.66 at 2 N and 100 degrees a second (-13.0 percent); the mark was 5.62 percent. Torque curve from 1 to 75 degrees, as a fraction of the slip torque: 0.073 (met), 0.263 and 0.115 against 0.0832. Twist at full slip: 40 against 29.0 degrees, 40 against 40.4 (met) and 39 against 48.1, with a mark of 5.75 degrees. Stuck-fraction curve: 0.036 (met), 0.081 and 0.135 against 0.0595. A simpler rule did better than the card on the slip torque at each condition: at 1 N the same model with the constants of another finger, P01, +1.9 percent; at 10 N a straight line in load, +18.4 percent; at 100 degrees a second no effect of rate, -4.7 percent. The card did better than torque proportional to load (-25 and +50 percent) and than the nearest fitted load (-44 and -38 percent). The scoring stands, and all three held-back conditions are now refused: the card that was scored answered from 0.5 to 10 N and from 5 to 100 degrees a second, and this one answers from 1.93 to 2.07 N and from 5 to 50 degrees a second. Where the card holds its own slip-torque mark, on the trials it was fitted to: at 2 N only, at 5 degrees a second (10.32 against 10.24 N mm, +0.7 percent), at 50 (10.23 against 10.68, -4.2 percent) and at 20 against the ten trials twisted about the centre of the contact (10.30 against 10.84, -5.0 percent). At 2 N and 10 degrees a second it is just outside (10.33 against 11.08, -6.8 percent), and at 20 with the 20 off-centre trials pooled, as the fit took them, it is -7.4 percent (11.12). On its fitted trials at the other loads it is off by +13.6 percent at 0.5 N and +21.7 percent at 5 N. Why: this finger's slip torque at its five loads was 3.11, 5.51, 11.12, 17.15 and 27.55 N mm; 70 of the 80 fitted trials are the block at 2 N, the fit takes its level from them, and that block stands 24 percent above the power law that the other four loads follow to within 4 percent. Inside the range that remains, the other lines on the four fitted conditions, 5, 10, 20 and 50 degrees a second: torque curve 0.028, 0.060, 0.089 and 0.048 of the slip torque against 0.0832; twist at full slip 40, 40, 39 and 39 degrees against a measured 45.0, 44.1, 46.4 and 48.8, so 4 to 10 degrees early against a mark of 5.75; stuck-fraction curve 0.110, 0.113, 0.132 and 0.141 against 0.0595, missed at all four. The card's slip-torque law is numerically a two-constant power law of load, 6.03 N mm x F^0.772, fitted to the same 80 trials weighted alike: it errs by +9.4 and +29.4 percent at 1 and 10 N where the card errs by +9.4 and +29.3, and the rate term and the five twist constants change the slip torque by 0.1 percent at those loads and by 0.6 percent at most inside the range. The paper's own fit of that law over the seven fingers is n = 0.76 +/- 0.06 and k = 6.74 +/- 1.31 mm. With the paper's exponent and this finger's level from its three opened condition means the held-back loads read +1.4 and +16.7 percent, closer than the card at both; with the paper's own k, the mean of seven fingers, they read +22.3 and +40.8 percent, further than the card, because this finger is one of the two of lowest friction. The five other fingers, with no mark: the card is off by 24 percent, root mean square, over their 45 condition means and reads low by 18 percent on average; between the seven fingers the slip torque at one load spreads by 17 to 26 percent. A second scoring of 2026-10-09, on the same machine, from the scorer's own reduction of all 768 files, found the same twelve verdicts, and the same against the authors' own per-trial summaries, where the slip torque is off by +9.4, +29.3 and -12.4 percent: evidence/checks/second_party/V-40/. The scored binary is kept (SHA-256 9b7ad4088cd7af1b3255eba36cc99cc2f7297c779f76bbdc8432920f13ff7c0a). The build that carries this text differs from it in the range it accepts, in the start it accepts, in the two warning flags it no longer writes and in this description; no fitted constant moved, and the four saved runs of the scoring that lie inside the range come back with every frame the same, byte for byte. Record: evidence/research/tactile_slip_fingerpad_p02_glass/RESULTS.md.";

/// The limits of the finger pad's validation record.
pub(crate) const FINGERPAD_P02_GLASS_LIMITS: &str = "All eight constants are fitted to this finger's own trials; nothing is derived from skin or glass, and the form (two characteristic twists, no load term in the twist of the stuck area) was chosen on another finger of the same experiment, P01. What the card holds: this finger, in one block of 70 trials of one sitting, slipped at 10.2 to 11.1 N mm at 2 N from 5 to 50 degrees a second (the means of its four conditions; single trials ran from 9.3 to 13.2), and the card gives that to about 8 percent. Those are trials the constants were fitted to. Inside the range the card accepts there is no held-back trial, so nothing inside it is a test; the only test, at held-back conditions, failed, and those conditions are refused. A miss, and it stays on the record. The floors of the slip torque, which no card of one finger can go under: one trial about the mean of its condition inside a block, 5.2 percent on this finger; block to block, 13 to 15 percent; finger to finger, 17 to 26 percent. The block-to-block figure is how far the block at 2 N sits from the power law through the other four loads over the seven fingers: +24 percent on this finger, from -12 to +24 over the seven, 13 percent as a standard deviation and 15 as a root mean square. The paper gives the order of the blocks, 0.5, 1, 5, 10 and then 2 N, not randomised because the camera was refocused for each force, and names the confound itself: total experiment time, with moisture increasing with occlusion time. The card takes its level from the last block and does not know the block. On this finger on another occasion a user should carry 13 to 15 percent, and on another finger about a fifth; nothing here tests another day, another surface, a wet or a treated finger, or a finger sliding along the glass: the motion is a twist about the normal only. The force band, 2 N less and plus 0.07 N, is the spread of the robot's force controller: the root-mean-square departure of the measured normal force from 2 N in the 60 trials at 5 to 50 degrees a second, 0.072 N; every trial's mean force was inside it, 2.006 to 2.045 N, and single samples ran from 1.80 to 2.28 N. Across the band the law moves the slip torque by 2.7 percent each way, with a load exponent that failed at every other load; no trial held a force at the ends of the band, so that movement is the law's and was not measured. The mean at 2 N and 20 degrees a second pools 20 trials twisted about a point off the centre of the contact, which the paper does not count among its 90 trials a finger and which read 3.9 percent higher on this finger: 11.12 N mm pooled, 10.84 from the ten centred trials. On this finger the slip torque shows no trend with rate from 5 to 50 degrees a second (10.24, 11.08, 10.84 and 10.68 N mm) and the card's rate exponent is -0.009; at 100 degrees a second it was 11.66, the card missed it by 13 percent, and that rate is refused. On three of the other fingers the slip torque rose by 60 to 80 percent from 5 to 100 degrees a second. This finger and P01 have the lowest friction of the seven. The twist at full slip is the card's weakest output: 39 to 40 degrees at every load and rate, against a measured 19 to 49 degrees over this finger's nine conditions; inside the range it is 39 to 40 against a measured 44 to 49 degrees, and between fingers it spreads by 11 to 17 degrees. FLAG_FULL_SLIP marks that event, the one the card exists to find, and is not a warning; the card writes no other flag. The stuck fraction is a second fitted curve of the same form as the torque's, with its own characteristic twist; a bed of springs on a rigid finger would have one; the curve missed its mark at seven of the nine conditions. The stuck fraction and the torque were never measured in one trial. The paper (J. R. Soc. Interface 20: 20220809, sections 2.2.1 and 4.3.1, read in its open text) says that the tangential force of the first experiment, the one with the camera, was corrupted, that the torque was acquired in a second experiment with one centred sensor and no camera, and that \\\"the experiments were conducted on separate days\\\", so that \\\"the moisture of the participant's finger could have impacted the friction\\\". The card's torque is fitted to the second experiment and its stuck fraction to the first; an earlier text of this record said the two were taken in the same session, and that was wrong. The trial key of the second experiment numbers this finger's 110 trials in one sequence with the block at 2 N last, trials 41 to 110; that the sequence was one sitting is this record's reading of the key, and the paper does not say so. The twist was compared from 1 to 75 degrees, and the card answers from zero to 75. The plate turned 80 degrees in every trial: at 2 N it held its rate to 77.0 degrees at 50 degrees a second and to 78.9, 79.4 and 79.7 degrees at 20, 10 and 5, and then slowed to a stop at 80, with the torque falling. The twist from 75 to 80 degrees was never compared, and a step that ends beyond 75 degrees is refused as TwistBeyondMeasured; the build of engine 0.6.0 answered to 80. The direction of the twist is not an input. The overshoot of the torque before it settles, a few percent, and the shrinking of the contact area during the twist are left out. No heat and no stored energy was measured: the ledger of work, stored energy and heat is that of the model's bed of springs and closes for any value of the eight constants, so it can catch a wrong stepping rule or a corrupted step and cannot catch a wrong constant. Its scale is fixed at 0.0508 J, the scale of the build that was scored (the law's slip-torque limit at 10 N and 5 degrees a second over the whole turn of 80 degrees), where the largest work inside the range is 0.0126 J, a twist to 75 degrees at 2.07 N and 50 degrees a second, so the gate of the ledger is 4.0e-9 of it. The slip torque and the full-slip twist are this record's own reductions of the published per-frame files. The slip-torque window here is the frames whose twist lies within rate x 0.5 s below 75 degrees; the authors' is the 0.5 s before 75 degrees. The two agree to 0.02 percent from 5 to 50 degrees a second and differ by 0.67 percent at 100. The authors' own per-trial summaries, not used in the scoring, have since been compared, by the second scorer and again by the builder: on this finger's 90 centred trials the values here agree with theirs to 0.04 percent in the condition means at 5 to 50 degrees a second. The raw 1 kHz files were not used. The values given here are this record's reductions of the authors' data (CC BY 4.0), not the authors' own figures, except where the paper is quoted. A run is one twist from zero under one load. A run whose later segment changes the force or the rate is refused whole, with no proof, as LoadChangedDuringTwist, and every run and every single step starts untwisted, so that no request can take up a twist under another load: a start at any twist but zero is refused as TwistNotStartedFromZero. The card that was scored answered a start at any twist under any load. Every trial of all seven fingers in the archive has now been opened, so a card rebuilt on this finger could not be tested on it. One party wrote the card and ran it; it was scored twice, the second time by a second scorer, on one machine.";

/// The result of the tokamak's validation record.
/// The one-line summary of the JET tokamak, as the list shows it.
pub(crate) const TOKAMAK_SUMMARY: &str = "The plasma of the JET tokamak in an ELMy H-mode pulse of 1996 to 1997, deuterium or deuterium-tritium: its thermal energy, its fast-ion energy and its D-T fusion power under the heating power, inside the envelope of the nine JET pulses of its regime it was compared with, 1.65 to 3.84 MA, 1.80 to 4.04 T, up to 24.3 MW of heating, the first 5.475 s of the heated window and a tritium fraction of 0 to 0.034 or 0.451 to 0.504; outside it the card refuses by name. Compared on 2026-10-09 with two held-back JET pulses, and by the rule fixed beforehand the comparison failed: on 42982 the fusion energy released was 0.743 of the measured, under the 0.75 mark.";

const TOKAMAK_RESULT: &str = "Compared with the two held-back pulses on 2026-10-09, and by the rule fixed beforehand the comparison failed: on the primary pulse, 42982, three of the four marks were met and the fusion-energy mark was missed. Pulse 42982, 219 samples over 5.475 s: thermal energy error 7.9 percent of its peak against a mark of 14.5; fast-ion energy error 22.5 percent against 25; fusion power error 23.4 percent against 25; fusion energy released 18.4 MJ against 24.8 MJ, a ratio of 0.743 against a mark of 0.75 to 1.25. The card's fusion power is too low throughout: its peak is 3.7 MW against 6.1 MW. Pulse 42976, the hot-ion pulse declared outside the card before it was run, missed all four: thermal energy 19.2 percent, fast-ion energy 25.1 percent, fusion power 34.7 percent, and fusion energy 5.4 MJ against 10.3 MJ, with a peak fusion power of 5.1 MW against 14.4 MW.";

/// The lines of the accumulator's validation record, fixed before a held-back
/// file was downloaded.
const ACCUMULATOR_SBO450_PASS_LINE: &str = "Fixed before any held-back file was downloaded (protocol SHA-256 721d88a5264b0e15342824290e6305db1df27fab76011333b49d5ed253b38323). For each held-back run the card is driven by a sine of the measured amplitude from the cyclic steady state at the measured mean pressure, and two numbers are compared: the stiffness, which is the swing of the gas pressure per unit of piston travel, and the lead of the pressure over the piston. Stiffness error: median at most 2.0 percent over the 285 runs and in each of the five records, and at most 5.0 percent in any run. Lead error: median at most 0.5 degrees, and at most 3.0 degrees in any run. And the card's two medians lower than those of five baselines: an isothermal and an adiabatic ideal gas, this model unfitted, the measured stiffness of the fitted condition scaled by the ideal-gas law, and this model with the ideal-gas law fitted to the same runs. The mean pressure of a run is taken from the measurement and is not tested.";

/// The result of the accumulator's validation record.
pub(crate) const ACCUMULATOR_SBO450_RESULT: &str = "Compared with the 285 held-back runs on 2026-10-09, run once through the engine, and by the rule fixed beforehand the comparison failed: two of six lines were met. The card answered 282 runs and refused three, the 5 mm runs at 12, 15 and 18 Hz at 120 bar, whose pressure passes the card's ceiling of 130 bar as the measured pressure does. Stiffness error over all runs: 0.93 percent in the median against a line of 2.0, met. By record: 0.26 percent at 20 / 40 bar, 1.45 at 20 / 60, 0.68 and 0.33 at 40 / 80, and 2.72 at 40 / 120 against 2.0, missed. Largest: 13.96 percent against 5.0, missed; 27 runs are above 5 percent, all at 0.1 Hz or under. Lead error: 0.27 degrees in the median against 0.5, met; 2.82 degrees at most against 3.0 on the runs answered, and not met over all runs because three have no answer. Against the baselines: the card's median stiffness error is the lowest, 0.93 percent against 2.20 for the fitted condition's measurement scaled by the ideal-gas law, 2.46 for this model with the ideal-gas law, 3.50 for this model unfitted, 12.2 for an adiabatic and 37.2 for an isothermal ideal gas; its median lead error is not the lowest, 0.27 degrees against 0.12, 0.15 and 0.25, missed. Above 1 Hz, 212 runs: stiffness within 0.59 percent in the median and 3.49 at most, lead within 0.18 and 0.99 degrees; the two ideal-gas baselines are at 2.2 and 2.4 percent there. At 0.05 Hz and under, 30 runs: the card is too stiff by 7.7 percent on average and 7.5 in the median, and the scaled measurement, 2.4 percent, and the fitted ideal gas, 3.2, are closer. The median lead line is met by every baseline as well, an ideal gas with no heat exchange among them at 0.30 degrees, so it is no evidence for the card's exchange of heat. On the 114 opened runs the stiffness error was 0.45 percent in the median and 3.17 at most and the lead error 0.17 and 2.20 degrees; two records of one condition differ in measured stiffness by 0.87 percent in the median. The binary that made the runs has SHA-256 a3d268ca094ba37d1edc91eb3e46d9748deb365d48b9295453e1fa45a8235428. Record: evidence/research/accumulator_darmstadt_sbo450/RESULTS.md. That is the registered result and it stands; the card it describes is the card as it was then, with a pressure range of 30 to 130 bar, a piston area of 19.2157 cm2 and a vessel of 0.57622 l. Second scoring, 2026-10-09, by another party (evidence/checks/second_party/V-39/): all 399 files reduced anew, all 798 requests rebuilt and asked again of the same binary, every proof checked; every number above was reproduced and the verdict stands. Amendment, 2026-10-09, made after every one of the 399 runs had been opened. It is not a new test and no line above is changed by it: the geometry was corrected, the range was drawn in to where the card held its marks, and the card refuses by name outside it. Run after the fact through the amended card (binary SHA-256 92e33dfeaacb2c2fceb0bed273e24424ed13d16fa68592c107e70c3171fbe56e): of the 285 formerly held-back runs it answers 196 and refuses 89, the 57 runs at 40 / 120 bar as PressureOutsideCard before the first step and the 32 runs at 0.1 Hz and under of the other four records as HeatShareAboveCard after 9 to 414 of 800 steps. Every run at 0.2 Hz and faster of the four records at 37 to 86 bar is answered. The pressure bands of each charge were added after this run, drawn by the same whole-bar rule from the measured pressures of those four records: on a transcription of the card's equations that a later check found equal to the engine's numbers in the three runs it compared (evidence/checks/fresh_eyes_0.6.0/accumulator_sbo450/), every one of the 196 held-back runs and the 98 opened runs answered here stays inside the bands of its charge at every step, so none of the numbers that follow changes. On the 196 answered, worked as the six lines were: stiffness error 0.49 percent in the median; by record 0.21 at 20 / 40 bar, 1.37 at 20 / 60, 0.60 and 0.26 at 40 / 80; 4.05 percent at most; lead error 0.21 degrees in the median and 1.79 at most; the median stiffness error is lower than every baseline's on the same runs, 2.06 percent for the best of them, and the median lead error is not, two baselines being at 0.10 and 0.11 degrees. None of these is a line met: every run was open when the range was drawn, and 89 runs have no answer. Of the 255 held-back runs that were within 5 percent and 3 degrees in the registered run, 59 are no longer answered, 46 at 40 / 120 bar and 13 at 0.1 Hz and under; none of the 27 that were above 5 percent is answered. Of the 114 opened runs 98 are answered, 0.42 percent in the median and 1.26 at most. On every step both binaries landed, 479,346 frames of 684 runs, the gas pressure and the gas temperature are the registered run's to the last bit of the 32-bit slot, and within 4e-15 in the 64-bit state at the end; gas volume, gas mass, force, work and heat are 1.021818 times the registered ones, and the flags are zero. Record: evidence/research/accumulator_darmstadt_sbo450/RESULTS.md under the heading of the amendment, and amendment_2026-10-09.md, SHA-256 ba073e2a1c070aa1282bcea8b330b765a833c53b0ab63367a61fb46722a68bb7.";

/// The limits of the accumulator's validation record.
const ACCUMULATOR_SBO450_LIMITS: &str = "The card was scored, failed, was scored again by a second party, and was amended afterwards with every run opened. What the amended card does on the measured runs is stated after the fact. It is not a new test. What the card gives: the swing of the gas pressure per unit of piston travel, for the two charges measured, each at the gas pressures it was measured at, and motion quick enough that the heat exchanged with the wall is a small share of the work. What it refuses by name: any other gas mass (GasMassOutsideCard: no charge between the two was measured), a pressure under 37 or over 86 bar (PressureOutsideCard: 85.56 bar is the highest pressure measured in the four conditions in which the card met its marks, the record at 109 to 132 bar missed its mark at every frequency, and between the two there is no measurement), a pressure inside 37 to 86 bar that the charge given was never measured at (PressureOutsideChargeBands: the 20 bar charge was measured at 37.76 to 42.71 and 55.26 to 66.37 bar and is held to 37 to 43 and 55 to 67 bar; the 40 bar charge was measured at 57.36 to 63.14 and 75.31 to 85.55 bar and is held to 57 to 64 and 75 to 86 bar), a wall temperature outside 293.4 to 293.7 K (WallTemperatureOutsideCard: the wall of every compared run was 293.46 to 293.60 K), a gas temperature outside 285.9 to 301.8 K (GasTemperatureOutsideCard: the compared runs reached 285.97 to 301.78 K), a piston speed above 0.7 m/s (PistonSpeedOutsideCard: the fastest compared run reached 0.69999 m/s as the mean over a step), and a motion in which the heat passes 0.065 of the work (HeatShareAboveCard). The card is never told a frequency, so slow motion is refused on what the card carries. M is the sum over the steps of the motion so far of the absolute work of each step and H the same sum of the heat; a start carries in the sums of the motion before it as work_moved_j and heat_moved_j, so a motion carried on from a returned state, by runs or by single steps, is weighed as one run; a step lands only if H is at most 0.065 of the larger of M and 100 J. Over one period of a sine the share is 0.05 at 0.2 Hz, 0.10 at 0.1 Hz and 0.8 at 0.01 Hz, nearly the same at every pressure, charge and amplitude measured. Run as they were scored, two periods from the cyclic steady state at the piston's outermost point, the measured runs at 0.2 Hz reach 0.062 at most and those at 0.1 Hz 0.104 at least; 0.065 is the next two-hundredth over what held, and nothing between 0.1 and 0.2 Hz was measured. The 100 J give the share a meaning before the oil has done work: it is a little over the work of a quarter of the largest measured swing inside the card, 91 J, and under the work of the smallest measured run, 125 J. A piston at rest with the gas at the wall's temperature exchanges nothing and may rest for any time. A piston at rest with the gas off the wall's temperature relaxes with the one time constant; no measurement here is of a rest, and the slow runs show that time constant wrong by a factor of two to three, so from the start of a run such a rest is answered only for its first 6.5 J, about 0.3 K of the 40 bar charge. The sums run over the whole motion, so a rest that follows motion is weighed against all the work before it and is refused later than the same rest alone, or not at all; a share over a trailing window of work would weigh it alone, and it has not been compared with the measured runs. The card cannot see motion before its start: it holds the sums a start states and cannot check them. Two constants are fitted to this accumulator's own runs at one condition, 40 bar precharge and 60 bar load: the ratio of the piston area to the vessel's gas volume, 3.3348 per metre, and the thermal time constant, 17.52 s. The gas pressure fixes that ratio and neither number alone. The piston area is that of a 50 mm bore, 19.635 cm2, which no source prints: over the seven records the measured force is 19.569 cm2 times the oil gauge pressure less 211 N, a bore of 49.92 mm, and every file's metadata give the force transducer an offset of -0.202 kN. At that area the fitted ratio is a vessel of 0.58879 l, where the catalogue gives 0.6 l and 36 cubic inches, which is 0.58993 l. The card as it was scored held 19.2157 cm2 and 0.57622 l: force over oil pressure at one pressure with the offset in it, 2.1 percent low. Its pressure and temperature were not affected; its gas volume, gas mass, force, work and heat were 2.1 percent low. With the 50 mm bore, the 36 cubic inches and no fitted geometry the same model's median stiffness error on the held-back runs is 0.72 percent, against 0.92 with the fitted ratio. A third fitted number belongs to the measurement and not to the card: the deflection reading is taken as a lag of 1.52 ms behind the piston, and the measured stiffness and lead are stated with that correction. Fitted to each record alone the lag is 1.47 to 1.53 ms in all seven: it is a property of the acquisition, in which the deflection passes through the test machine's own conditioner and the pressures do not. Above 2 Hz the lead lines compare that fitted lag with a model lead of under 0.1 degrees and test nothing: every baseline meets them. At 0.5 Hz and under, where the lead says something about the exchange of heat, the card's median lead error on the held-back runs is 1.24 degrees, and two baselines are closer, the scaled measurement at 0.94 and the fitted ideal gas at 0.89. The held-back runs are at load pressures of 40, 80 and 120 bar and at half the gas mass: extrapolation in pressure and in charge, at the same frequencies and amplitudes as the fitted runs, on the same accumulator, rig, gas and room temperature. Nothing here tests another vessel, another gas, another temperature, a motion that is not a sine, a rest, or the mean pressure, which is taken from each run. The precharge is absolute: the metadata give both pressure sensors a range of 1 to 201 bar, and the records at 20 bar precharge agree with the card read that way and not as gauge. The exchange of heat is one time constant, and that form fails here. Fitted afterwards to each record's own runs it is 17 to 18 s at 40 / 60 bar, 13 s at 40 / 80, 9.3 s at 40 / 120, 10.6 s at 20 / 40 and 8.1 s at 20 / 60, about 45 s per litre of gas volume. A time constant proportional to the gas volume, fitted on the one opened condition, would still have failed the comparison: 2.56 percent in the worst record, 6.73 percent at most and 4.03 degrees at most. Within one record the time constant that each frequency asks for falls threefold, from 20 s at 0.01 Hz to 6.6 s at 1 Hz at 40 / 60 bar. The group that made these measurements has published the same of two other HYDAC diaphragm accumulators, SBO500-0,1 and SBO330-0,75, at 20 and 40 bar precharge: \"Newton's law is incapable of representing this physical phenomenon\" and \"Experimental data confirm the failure of Newton's approach\" (Hartig, Depp, Rexer and Pelz, Effects of unsteady heat transfer on behaviour of commercial hydro-pneumatic accumulators, arXiv:2012.06526). So at 0.1 Hz and under the card as scored was up to 14 percent too stiff, and there the amended card refuses. The miss at 40 / 120 bar is not the equation of state. Against the NIST WebBook isotherm at 293.15 K the Peng-Robinson density is 0.5 percent high at 20 bar and 1.9 at 120, and its isentropic stiffness 0.2 to 1.8 percent low; in the card these make the charge 0.8 percent softer at 40 / 120 bar than the reference equation would, so with the reference equation the card would be further off there, 2.9 percent too stiff above 1 Hz against 2.1. What the card leaves out is the give of the oil side, in series with the gas: the second scoring found that one series compliance, fitted afterwards to all seven records, brings every record within 0.33 percent above 1 Hz. That is a lead for another card and is not in this one. Two records of one condition differ in measured stiffness by 0.87 percent in the median, most of it a difference of mean pressure between two fills: with that removed through the model it is 0.35 percent. The unfitted baseline takes its time constant from Rotthäuser's correlation, which the source the second scoring found for it gives for piston accumulators: a weak comparison for a diaphragm accumulator. The gas temperature sensor reads a fifth of the card's swing at 0.01 Hz and nothing above 1 Hz; its response time is not stated and the temperature was not scored. The oil, the diaphragm and the line are left out. This is a closed gas spring: no orifice, no flow of gas, and choked flow does not arise. The ledger closes for any time constant and any vessel volume: it checks that the pressure, the heat capacity and the energy are one equation of state and that the step integrates them, and it cannot catch a wrong constant. The gas mass is a number the step never changes. One party wrote the card, ran it and scored it once, on one machine. A second party scored it again from the protocol (V-39) and found the score right. The amendment and its run after the fact are the first party's and have not been scored by a second party.";

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
            "A forecast is not a result. The forecast, made outside the engine with constants fitted on the first nine April visits and scored on the other ten: 0.774 and 0.785 against the last centroid on x and y, 0.779 and 0.636 against the lock point, a mean normalised squared error of 0.680 and 0.938, a frame-to-frame correlation of 0.113 and 0.067, and 84 and 100 percent of files. The constants are fitted to this observatory's own centroids, so the held-back visits test extrapolation in time and to other guide stars and nothing else: one observatory, one season, one guiding mode. In brightness they are interpolation: their count rates, 62,000 to 816,000 counts per second, lie inside the fitted 22,900 to 2,820,000, so the noise law was not tested at either end of the card's range. One party wrote the card and ran it. It was scored twice, the second time by a second scorer, on one machine. The prediction is one frame, 64 ms, ahead. The estimate of the line of sight with the noise taken out is a model output that no measurement here checks. The lock point is not in the file and is the caller's; the validation took the median of the run's good centroids, which uses the whole run. The lock points of the files the comparison answered span -61.1512947 to 60.4564934 arcsec on x and -64.2767 to 50.1812668 arcsec on y, fitted and held-back files together, and the card refuses a lock point outside that span as LockOutsideCard; no lock point nearer the edge of the sensor's field was compared. The longest run of frames with no measurement that a run of the comparison crossed was 10 frames on the fitted files and 2 on the held-back files, and the card refuses an eleventh such frame in a row as MeasurementGapOutsideCard. Nothing above 7.8 Hz is resolved by a 64 ms frame: the Nyquist frequency is 7.81 Hz, and the card cannot see or state jitter above it, nor tell a folded line from a true one. The card describes quiet fine guide: inside a disturbance, a centroid more than 5 mas from the lock point and the 30 s after it, the filter follows slowly, and on the fitted visits as on the held-back ones its error was larger than that of taking the last centroid. The card refuses there. It holds a new run to 30 s after the last disturbance only by the caller's statement, which it cannot check; and a caller's lock point that is off by a few mas makes quiet frames read as disturbed, so the card then refuses frames it could have answered. It says nothing of the line of sight during a disturbance: the visits of March 2022 are unopened and kept for a card that follows one. The y axis shows lines near 0.51, 1.02 and 1.53 Hz that the model has no part for. In the held-back spectra the y feature is a narrow line at 0.290 to 0.294 Hz with a shoulder at 0.32 to 0.34 Hz; the card's 0.3025 Hz is one damped mode laid across both. The commissioning report of the telescope (Rigby et al., arXiv:2207.05632, section 3) names a 0.3 Hz oscillation of a vibration damper between the telescope and the spacecraft bus and a 0.045 Hz oscillation attributed to fuel slosh; the card has no part for the 0.045 Hz one. The x feature is a broad hump at 0.50 to 0.60 Hz, for which no published attribution was found. The ledger is an account of variance that follows from the model; it is not a conservation law, and it cannot detect a wrong constant. It checks that each update is a Kalman update with the optimal gain: it passes with a wrong mode frequency, wrong noise or the two axes swapped, and it says nothing about the telescope.".into(),
        ],
        "cell_18650pf" => [
            "One Panasonic NCR18650PF lithium-ion cell, new, at 25 degrees Celsius, discharged from full charge at a held current of 1C.".into(),
            "Kollmeyer (2018), Mendeley Data, doi 10.17632/wykht8y7tg.1, CC BY 4.0: the laboratory's tests of that cell. The card's constants were taken from its low-rate and pulse tests. Eleven 1C discharges were held back from that fitting.".into(),
            "Fixed before any result was seen: terminal voltage within 50 millivolts and delivered capacity within 3 percent.".into(),
            "Eleven of eleven discharges met the line. The voltage error was 27.8 to 30.0 millivolts root mean square (25.7 to 32.4 across the three onset times tried), and the largest single error was 57 millivolts. On the one test that ran to cutoff the delivered capacity differed by +0.04 percent at steps of 0.1 s and by +0.014 percent at the protocol's step of 1 s in the second scoring, with the discharge ended by the card's own cutoff.".into(),
            "One cell, new, at 25 degrees Celsius, at a held current of 1C. The comparison that passed covered discharges from full charge only, at 1C only, 2.899 to 2.89982 A. The card takes 2.89817 to 2.90065 A, that span widened by its 0.83 mA band, and refuses any other current as CurrentNotCompared; through engine 0.6.0 it took any held current from 0 to 3.0 A. The dataset's other held currents are the C/20 test (0.145 A), which set the rest-voltage curve, and the 0.87 A holds of the pulse test, which set the resistance; both are fitting data and neither is a test of the card. One discharge at another current was left out of the fitting: 04-27-17 3541_dis5_10p.mat, the same cell at 25 degrees Celsius discharged at a held 0.86888 to 0.86970 A from full charge to 2.5 V in 14 holds with rests at 0 A between them. Its totals and its voltage trace had been read before it was compared; no output of the card had. Compared on 2026-10-10, each hold run as a continuation of the one before and the rests not compared, at steps of 0.1 s, it failed: the voltage error during the holds was 56.7 millivolts root mean square against the line of 50, rising to 204 and 206 in the last two holds, and the card did not end by its own cutoff but left its table at soc 0.03 and 2.66 V (StateOfChargeOutsideCard), 71 s after the cell reached 2.5 V, having delivered 0.60 percent more than the cell. 0.87 A is therefore refused: evidence/research/panasonic_18650pf/held_0p87A_comparison. A start below full charge is taken only as the continuation of a 1C discharge, whose held current the request states; through engine 0.6.0 any start from 0.03 to 1 was accepted at any current. A current that changes during a run is refused as CurrentChangedDuringDischarge; through engine 0.5.0 it was accepted. Not a drive cycle, another temperature or a pack; the twelve-cell pack card is a different card and is not validated by this. The capacity total of the full discharge had been seen before the comparison; the voltage traces had not. The card has no temperature and no memory. Run by one party and scored twice, the second time by a second party on 2026-10-07, with the same verdicts on all eleven discharges: evidence/checks/second_party/V-07.".into(),
        ],
        "capsule_stardust" => [
            "The Stardust sample return capsule on its entry of 15 January 2006, from entry interface to parachute opening.".into(),
            "NASA's reconstruction of the entry (peak load 32.89 g) and the radar record of the altitude and speed at parachute opening. The card is fixed to the capsule's pre-flight design and navigation values and to published standards (U.S. Standard Atmosphere 1976, World Geodetic System 1984).".into(),
            "Fixed before any trajectory was computed: peak load within 3.64 g of 32.89 g, and the altitude and the Mach number of parachute opening each reached within 7.4 s of the recorded time. The lines are NASA's printed three-sigma pre-entry spreads.".into(),
            "All three lines met. Peak load 32.17 g, 2.2 percent under 32.89 g. The altitude of 31.03 km was reached at 138.909 s and Mach 1.23 at 136.953 s, against 137.9 s recorded.".into(),
            "The comparison is not blind: 32.89 g was known to everyone involved. It is NASA's own fitted simulation, not an instrument reading; no accelerometer flew, and the two times rest on range radar. The structure used was recommended after scores were seen; the rule fixed in advance named a simpler one, which passes the peak-load line by 0.12 g. The air is a yearly standard atmosphere, not the air of that morning, which is worth about 1 g either way. One entry, down to parachute opening only. Heat flux is not validated: the card gives 696 W/cm2 against NASA's computed 762 to 1000. The run repeats a forecast made outside the engine, so it is not a new test of that forecast; a second capsule would be. Nothing past the later of the two deployment crossings was compared, and the card refuses a state there. Run once and scored twice. Repeated on 2026-10-07 by Grok, another developer's model, with its own comparison written from the protocol: the same three numbers and the same run proof. It had read the headline numbers first, and a directory search showed it lines of the first agent's comparison scripts before its run. Grok's other sessions write the engine, and no other organisation has repeated this result.".into(),
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
            "A miss, and it stays on the record. One generator; the daily-average electrical output only; the record holds no heat measurement. The card reads high on the held-back days and the gap grows: its mean error was +0.0031, +0.0045 and +0.0096 of 115.4 W over three successive spans of 365 days and +0.0107 over the last 184, about 0.36, 0.52, 1.11 and 1.23 W. Days 2,061 to 2,808 were used to choose the card's structure and are not an independent test of it. A different structure would have passed the held-back days, which was seen after they were opened and is information, not a result. The card serves mission days 2,809 to 4,087, the days it was scored on, as the model that was scored, with no constant changed since: a card that failed its comparison, served with the failure stated. The card ends at mission day 4,087, the last day of the published record, and refuses a later day by name. A corrected card needs days no model has seen: the record after October 2023. Run by one party and scored twice by it, the second time with a separate script. Repeated by a second party, Grok, on 2026-10-07, with its own comparison written from the protocol and run on the cut engine of version 0.2 on the same machine: the same root mean square error, 0.008129, the same final-year mean, +0.010094, the same run proof and the same miss (evidence/checks/second_party/V-11). Grok is also the SI whose other sessions write the engine, and it had read the record's numbers before it repeated them.".into(),
        ],
        "link_gothenburg_10010" => [
            "One commercial microwave link in Gothenburg, Sweden: link 10010, sublink 587, of the OpenMRG data set, 32.445 GHz, vertical polarisation, 1.25841 km, a 0.3 m antenna at each end, sent level 5 dBm, with the weighing rain gauge at Barlastplatsen 37 m from the path.".into(),
            "Andersson, Olsson, van de Beek, Hansryd, Andersson and Persson, The OpenMRG data set, Zenodo, doi 10.5281/zenodo.7107689, CC BY-SA 4.0: the sent and the received level every 10 s and the gauge every minute, June to August 2015. The rain term is Recommendation ITU-R P.838-3 at this frequency and polarisation, with nothing fitted: k = 0.27282714286921533 and alpha = 0.8947805998167349 at 32.445 GHz, vertical. The three constants of the water on the antennas were fitted to this link's own readings in the 18 rain events of June and July: a ceiling of 2.13 dB, a wetting time of 81 s and a drying time of 404 s. Every reading of August was held back and was not opened until the protocol's fingerprint was recorded.".into(),
            "Fixed before any reading of August was opened (pre-registration SHA-256 f60bfcc70553f95fef5d35963b6550120de3c67e712c057e40c5d37c51784625), on five-minute means of the attenuation above the dry level over every August rain event of at least 1 mm: root mean square error at most 0.90 dB, at most 0.80 of the error of P.838 alone on the same blocks, and the summed attenuation between 0.75 and 1.25 of the measured sum.".into(),
            LINK_RESULT.into(),
            "A forecast is not a result. On the 18 events the constants were fitted to, the card's error is 0.667 dB, against 1.028 dB for P.838 alone and 1.589 dB for no attenuation; with each event left out of the fit in turn, 0.677 dB. The structure and the link were chosen on June and July. August tests new rain events on the same link in the same summer, inside the rain rates fitted: interpolation in rain rate, later in time. It does not test another link, frequency, polarisation or season, or snow. One link, one direction, one gauge at one end of the path: rain on the path that differs from rain at the gauge is not in the card, and the fitted event with the heaviest rain, 84 mm/h, has the largest error, 1.84 dB. The dry level is the caller's, read from the link itself before each event, and the result depends on it: in the comparison it was the median of the hour before each event; one receiver step, 0.3 dB, of error in it puts the summed attenuation outside its band, at 1.69 of the measured sum, and a single dry level for the whole month, 52.30 dB, gives 0.644. An agent that has only a rain forecast, and no reading of the link's level just before the rain, does not have what this comparison used. An agent sizing a fade margin must not take the card's peak as the measured peak: the card's five-minute peak was the lower in seven of the nine events, by up to 2.2 dB, and its one-minute peak was the lower in six, by up to 2.8 dB, and the higher in three, by up to 3.3 dB. The dry level's median, 52.30 dB, is 7.98 dB above free space between two ideal 0.3 m apertures (124.667 dB less 2 x 40.172 dB); the data set gives no antenna gain or feeder loss, so that difference is not divided. The received power in watts and the energies are the card's arithmetic on the levels; no energy was compared with a measurement. The comparison fed only one-minute gauge rates of 0 or multiples of 6 mm/h, held through whole minutes in steps of 60 s from dry antennas; a rate above 0 and below 6 mm/h and a rain changed inside a gauge minute are refused by name.".into(),
        ],
        "reactor_spert3" => [
            "The SPERT III E-core, a small light-water test reactor, in its cold-startup reactivity-insertion tests: primary coolant at 56 to 92 degrees Fahrenheit, no flow, 5e-5 MW at the start, reactivity inserted in a step.".into(),
            "IDO-17281 (Phillips Petroleum Company for the US Atomic Energy Commission, March 1969), Table III: 30 tests, each value with a stated standard deviation. Every third test by the order in which the tests were run was held back and sealed; the other 20 were opened. The card's one fitted constant was taken from the 20 opened tests. None of the report's measured values is stored in the card. The ten held-back tests span 0.890 to 1.128 dollars, 4.3 to 110 MW at the peak and 56 to 82 degrees Fahrenheit. Outside that span the card was compared only with the opened tests its constant was fitted on, in sample and each left out of the fit in turn: test 22 below it, tests 42 and 43 above it. Tests 22 and 43 set the ends of the span the card accepts.".into(),
            "Fixed before any model was run (protocol SHA-256 47458dc27bcd9a60fe6c16ac613b2be3e15b30c7e4bb75d7624c80707e4e7ccd). Peak power and energy released to the peak: the root mean square of the error, in units of the report's standard deviation, at most 1.5; the worst error at most 3; the mean error at most 0.5 in size. Time of the peak: the root mean square of the relative error at most 5 percent and the worst at most 10 percent.".into(),
            "Compared with the ten held-back tests on 2026-10-08, and by the rule fixed beforehand the comparison failed: five of the eight lines were met and three were missed. Peak power met all three of its lines: root mean square 0.58 of the report's standard deviation, worst 1.13, mean +0.29. Energy released to the peak met two and missed one: root mean square 1.01, worst 1.99, and a mean of -0.705 against a line of 0.5 in size: the card reads low. Time of the peak missed both of its lines: root mean square 23 percent against 5 and worst 29 percent against 10, every peak early.".into(),
            "One constant is fitted, the strength of the feedback with energy released, on the 20 opened tests. The delayed-neutron constants are the report's values stated for 500 degrees Fahrenheit, used here on cold tests; the report prints none for cold. A forecast made outside the engine on the opened tests, each left out of the fit in turn, met the peak-power and energy lines and missed the time-of-peak line, with peaks about 21 percent early. The card stops at the first power maximum: boiling, heat flow from fuel to water, hot startup and operating power are not on the card. The reactivity is the report's reactor period converted by the caller; the card takes dollars. Only the peak power, the energy released to the peak and the time of the peak were compared with the held-back tests. The states the card seals before the peak were compared only with digitised plots of 19 opened tests; on 18 of them the card reaches a tenth of its peak about 26 percent early. The card steps the reactivity in at 5e-5 MW; the report's rod ramped it in over 50 to 65 ms, and a fit to the plots puts the starting power near 9 W (evidence/research/reactor_spert3/analysis/10_peak_timing_cause.md). energy/reactor_spert3_fuel carries the ramp.".into(),
        ],
        "reactor_spert3_fuel" => [
            "The SPERT III E-core, a small light-water test reactor, in its low-initial-power reactivity-insertion tests in three bands of inlet temperature: 56 to 92, 251 to 259 and 498 to 501 degrees Fahrenheit, with no coolant flow in the cold band, 2.4 to 22 ft/s in the 260 degF band and 4.8 to 24 ft/s in the 500 degF band. The reactivity is ramped in as K t^2 by the transient rod; the fuel temperature and the water temperature are state, and the fuel feeds back by a square-root Doppler law.".into(),
            "IDO-17281 (Phillips Petroleum Company for the US Atomic Energy Commission, March 1969), OSTI 4792676, Table III (cold startup) and Table V (hot startup at about 260 and 500 degrees Fahrenheit): 58 scored tests, each value with a stated standard deviation. The card's four fitted constants were taken from 28 tests: the 20 cold-startup tests already opened and eight tests at 500 degrees Fahrenheit. 30 tests were held back from that fitting: ten cold-startup tests, seven at 500 degrees Fahrenheit, and the whole 260 degree Fahrenheit series of 13, at which nothing in the card is fitted. None of the report's measured values is stored in the card.".into(),
            "Fixed before any model was run on a hot-startup test and before any held-back comparison (protocol evidence/research/reactor_spert3/validation_protocol_v2.md; SHA-256 of its machine-readable copy 413ace16d5cc3d622c0ef44c0ea2b9526ac7526dfeee4b43ae601d6018e51db4), for each held-back group and for the 30 together. Peak power and energy released to the peak: the root mean square of the error, in units of the report's standard deviation, at most 1.5; the worst error at most 3; the mean error at most 0.5 in size. Time of the peak: the root mean square of the relative error at most 5 percent and the worst at most 10 percent.".into(),
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
        "reactor_triga_jsi" => [
            "The TRIGA Mark II research reactor of the Jozef Stefan Institute, Ljubljana, pulsed by pneumatic ejection of its transient rod, with the 56-element core of standard fuel (12 wt% uranium, 20 percent enriched, U-ZrH) loaded as cores 215 (November 2014), 217 (March 2015) and 231 (March and April 2018): the power through the pulse and the seconds after it to the scram, the energy released, and the core-average fuel temperature that energy makes.".into(),
            REACTOR_TRIGA_JSI_DATA.into(),
            REACTOR_TRIGA_JSI_PASS_LINE.into(),
            REACTOR_TRIGA_JSI_RESULT.into(),
            REACTOR_TRIGA_JSI_LIMITS.into(),
        ],
        "cell_2170_m80" => [
            "One Tesla Model 3 2170 lithium-ion cell, cell m80 of the dataset, as it stood in each of six blocks of tests at chamber settings of 40, 25, 10, 0, -10 and -20 degrees Celsius: its terminal voltage, given the current and the temperature measured on its case. The four blocks from 0 to 40 degrees Celsius are served; the -10 and -20 degree blocks, shown wrong by the comparison, are refused.".into(),
            "Kollmeyer, Khanum, Naguib and Emadi, Tesla Model 3 2170 Li-ion Cell Dataset and Battery SOC Estimation Blind Modeling Tool, Borealis, doi 10.5683/SP3/ZVTR4B, version 3.0; and P. J. Kollmeyer, M. Naguib, F. Khanum and A. Emadi, A Blind Modeling Tool for Standardized Evaluation of Battery State of Charge Estimation Algorithms, 2022 IEEE ITEC, pp. 243-248, doi 10.1109/ITEC53557.2022.9813996. The card's 2,307 fitted numbers were taken from 17 characterisation files of that cell: the slow discharges and the pulse test of each block. 18 tests were held back and opened for a forecast outside the engine. 60 more were held back until the engine's card was compared with them: 12 discharges at a held current and 48 drive cycles.".into(),
            "Fixed before any constant was fitted (protocol SHA-256 488fe0af60bbb68be465106ae5fe368b7780f331e8ea5c87e8ac523e6ab984fa, with its Amendment 1). On each held-back discharge at a held current: voltage error at most 42 millivolts root mean square, delivered capacity within 3 percent, and the run ended by the card's own 2.8 V limit. On each held-back drive cycle: voltage error at most 42 millivolts root mean square, the card still running at the last sample, and the error at the last sample at most 126 millivolts. The same marks at all six temperatures, with a verdict for each temperature.".into(),
            "Compared with the 60 held-back tests on 2026-10-08, and by the rule fixed beforehand the comparison failed: 27 of 60 runs met their marks and no temperature met its verdict. The slow discharge, at a third of the rated current, met its marks at 40, 25, 10 and 0 degrees Celsius: voltage error 5.0, 6.4, 15.0 and 29.2 millivolts root mean square. The discharge at the rated current missed at every temperature: 48.2, 44.1, 51.2 and 88.2 millivolts at 40, 25, 10 and 0 degrees Celsius, against a mark of 42. Of the 32 drive cycles from 0 to 40 degrees Celsius, 23 passed and 29 met the voltage mark. At -10 and -20 degrees Celsius no run met its marks: voltage errors of 55.6 to 425 millivolts. From engine 0.7.0 those two blocks are refused as BlockShownWrong, each served block is held to the case temperatures and the currents of its own held-back tests. A third scoring, written from the protocol's text alone by an agent that did not write the card, through engine 0.6.0 on 2026-10-10, reached the same 27 of 60 with every error and every call the same to within a millionth of a millivolt.".into(),
            "A miss, and it stays on the record. The forecast made outside the engine on the 18 opened tests, one discharge at 2.25 A and two drive cycles at each temperature, had this card's shape meet every mark at 25, 10 and 0 degrees Celsius, miss one drive cycle at 40 degrees Celsius by 2 millivolts at the last sample, and miss at -10 and -20 degrees Celsius. It held no discharge at the rated current and could not have shown that miss. The held-back tests of this cell are spent: a corrected card needs a cell no model has seen. One party wrote the card, ran it and scored it twice, on one machine; a third scoring, from the protocol's text alone by an agent that did not write the card, agreed with it. No one outside Protocol Company, which makes this engine, has repeated it. One cell. One capacity per block, while the cell went on ageing inside each block. Each temperature was measured at one age, so a block is also an age of the cell. The card is given the case temperature and does not predict it. Fitted from pulses of up to 13.5 A discharging and 2.25 A charging: a charging current above 2.25 A is served by the discharge side of the tables. Served up to 11.375 A discharging and 9.737 A charging, the largest currents of the held-back tests, and in each block only up to the largest currents of that block's own tests; held currents were compared at about 1.5 A and 4.5 A only, and the card bounds the size of the current, not how long it is held. The tables end where a slow discharge at the block's temperature ended. The step is one second.".into(),
        ],
        "cantilever_eth" => [
            "One aluminium strip, 326 mm long, 40 mm wide and 1 mm thick, clamped at one end under a bolted steel plate with nothing between plate and strip, at ETH Zürich: its first bending mode in free decay in room air, as the deflection at the laser-1 station, 268 mm from the clamp.".into(),
            "Lai, Alzugaray, Chli and Chatzi, Zenodo, doi 10.5281/zenodo.22723484, CC BY 4.0: recordings of the strip ringing down. The card's six constants were fitted to one recording, test1a, from 0.863 s to 24 s after its start. A second ring-down of the same strip in the same clamp, test1b, was held back from that fitting: its first 12 s were opened for a forecast made outside the engine, and its next 12 s were kept unread until the card was compared with them.".into(),
            "Fixed before any scoring (pre-registration SHA-256 d081cc5ae51b5de4a028cd17b4e1ae80b402f71fdc786f82876713f3b925d4a6), on the blocks between 12 s and 24 s of the held-back ring-down: amplitude error at most 5.0 percent, frequency error at most 0.010 Hz, timing error at most 0.20 of a period by 24 s, and the amplitude error and the frequency error each at most one third of a plain damped oscillator's.".into(),
            CANTILEVER_RESULT.into(),
            "A miss, and it stays on the record. A second scoring, written from the protocol's text alone by a second agent with a reader of its own, reached the same verdict on the same eleven blocks with the same frequency and timing errors; it took each period's amplitude from a fitted sine where the first took half the peak-to-peak value, and found 7.94 percent where the first found 7.03. The forecast for this structure, made outside the engine on the opened 12 s, was 2.22 percent in amplitude, 0.0047 Hz in frequency and 0.024 of a period in timing, and the card run through the engine gave the same figures there. The rule fixed in advance named a smaller structure of five constants; this card is the larger one of six, chosen after the opened scores were seen because the smaller was expected to miss by more. Scored outside the engine on the same blocks, the smaller structure missed the amplitude line too, at 6.48 percent, closer than this card. The last two blocks lie below the smallest amplitude the constants were fitted on, 0.072 mm; worked out after the scores were seen, the amplitude error without them is 5.7 percent, still over the line. The card accepts the envelopes this comparison spanned, from 0.058 to 15.79 mm, and refuses outside them by name: the compared run's own start, at 15.786 mm, is the top of that range, and its readings were scored from 14.404 mm. The held-back half of this ring-down is spent: a corrected card needs a ring-down no model has seen. One party wrote the card, ran it and scored it twice, on one machine; not yet repeated by a second party. One strip, one clamp, one afternoon. The laser was logged in volts: the scale in metres rests on one line of the authors' script, cross-checked to 3 percent, and every compared quantity is a ratio or a frequency. The card has no mass, so its energies are per kilogram. The softening spring is a fitted power law: it is not derived from the strip's dimensions and says nothing outside the fitted amplitudes. A start inside the amplitude ceiling with nearly all its energy as speed can be refused some steps later, because the softened spring carries the deflection envelope past the ceiling: from zero deflection with the envelope at 0.99 of the ceiling the step is refused after 104 steps of 0.25 ms, and at 0.98 the beam rings through.".into(),
        ],
        "cell_2170_model3" => [
            "One Tesla Model 3 2170 lithium-ion cell from -10 to 40 degrees Celsius: its terminal voltage, given the current and the temperature measured on its case. A physical model with memory: one measured rest-voltage curve and 22 fitted numbers for the immediate resistance, a surface reaction that eases with current and temperature, lithium diffusing in the particles, hysteresis, and a slow voltage drop in the electrolyte under sustained current. Three cells of the dataset are held: m80, m1000 and m448-N.".into(),
            "Kollmeyer, Khanum, Naguib and Emadi, Tesla Model 3 2170 Li-ion Cell Dataset and Battery SOC Estimation Blind Modeling Tool, Borealis, doi 10.5683/SP3/ZVTR4B, version 3.0, CC BY 4.0. The curve and the 22 numbers were fitted on every test of cell m80 from -10 to 40 degrees Celsius. For cell m1000 one number was fitted, a resistance scale, on its discharge at a third of the rated current, its pulse test, its slow charge and discharge and its capacity test at each of those temperatures, and its capacities were measured from its recharges. For cell m448-N the same one number was fitted, on the same four tests of its own at each temperature. 45 runs of cell m1000 and 60 runs of cell m448-N from -10 to 40 degrees Celsius were held back unopened until the card was run on them: discharges at a held current and drive cycles.".into(),
            CELL_2170_MODEL3_PASS_LINE.into(),
            CELL_2170_MODEL3_RESULT.into(),
            "The first cell is not a blind test: the model was developed on all of its tests. Scored outside the engine by the study that fitted it, the model met the marks on 62 of the first cell's 65 runs in range and on 14 of the second cell's 15 opened runs in range. Known misses. Three hill cycles of the first cell, at 40, 25 and -10 degrees Celsius; at 40 and 25 the card stops at its 2.8 V limit shortly before the real cell, which is riding the same limit there. Steep climbs at low charge in the cold: at 0 and -10 degrees Celsius, at the end of climbs of 7 to 10 A for four minutes below 45 percent charge, the card reads 130 to 180 millivolts high, and three physical causes were tested and none stands. Charging in the cold: in the descents of the hill cycle at 0 and -10 degrees Celsius the card reads 40 to 70 millivolts high. Capacity is up to 2 percent long under held current at 10 and 0 degrees Celsius. The cold edge is predicted poorly when it is left out of the fit: fitted without the tests at -10 degrees Celsius the model met its marks on 3 of the 13 runs there, so the temperature laws do not reach to a colder temperature. The range: a case temperature from -12.9 to 45.9 degrees Celsius, which is the chamber set from -10 to 40 with 0.3 kelvin allowed either side of what the sensor read; a current up to 13.524 A discharging and 13.984 A charging; each limit is set from the tests the model was scored on before that check, on the first two cells, with the accuracy the dataset states for the tester or the chamber allowed beyond them. -20 degrees Celsius is outside the card, because the measurements there are irregular and no mechanism could be separated from them; scored there for the record, the model met its marks on none of the first cell's 13 runs. Sustained current above the rated current was tested by the hill cycle alone, and the card refuses a running mean above 8.722 A. Charging current is accepted and was fitted inside drive cycles; no charging run was scored. The card is given the case temperature and does not predict it. The capacity is the caller's, read from the last full charge. Each block of tests was measured at one age of the cell, so a new cell was never measured in the cold nor an aged cell warm, and the resistances carry no age term: the card refuses a capacity outside the band its block was scored with at that case temperature (CapacityOutsideCardAtTemperature). It also refuses a case temperature that changes by more than 5.4 K between steps or spans more than 19.2 K in a run, past the 5.023 K and 18.811 K of the scored runs. Every fit and score was made at a step of one second, and the card steps by one second only: where the current reverses, a step of 0.1 s gave an answer 49 mV from the one-second answer in the reversal that was checked, and no measurement on disk checks it. Every run scored in the record lies inside these limits, so none of them is refused by them. The unseen runs of the second and the third cell inside the range are now spent: a corrected card needs a cell no model has seen. The hill-climb cycles of those cells lie outside the card's range and were not scored. One developer's SI wrote the model, wrote the card and ran the check, on one machine. A second agent, which did not write the card, scored it again on engine 0.6.0 from the protocol alone (evidence/checks/fresh_eyes_0.6.0/cell_2170_model3): with the record's own starts 50 of 60 runs of cell m448-N and 33 of 45 of cell m1000 were inside the marks, the same verdict as the record on 102 of 105 runs, the three others turning on where the scored span ends; with its own simpler starts, full charge and each block's capacity, 48 and 27. It is the same developer's SI, and no one outside Protocol Company, which makes this engine, has repeated the result.".into(),
        ],
        "creep_cmsx6" => [
            "One cast of the single-crystal nickel-base superalloy CMSX-6, cast by PCC Airfoils as [001] bars, solution annealed and aged: twelve round test pieces 6 mm in diameter, within 1.9 to 7.5 degrees of [001], in tension creep at 980 degrees Celsius under a held load at initial stresses of 140, 170, 200 and 230 MPa, three at each.".into(),
            "Avila Calderon, Schriever, Han, Olbricht, Dolabella Portella and Skrotzki, BAM Reference Data: Creep of Single-Crystal Ni-Based Superalloy CMSX-6, version 2.1, Zenodo, doi 10.5281/zenodo.20132712, CC BY 4.0: creep tests to DIN EN ISO 204 in a laboratory accredited to ISO 17025. The card's six constants were fitted to six tests, two at each of 140, 170 and 230 MPa. Six were held back and not read: all three at 200 MPa, a stress no fitted test has, and one more test piece at each of the other three stresses.".into(),
            "Fixed before any curve was fitted (pre-registration SHA-256 f20f99b0e3419d66201a3e124b5c216c4d138d41f976b19d6a536daf15316d87, with two numbered amendments made before the held-back tests were read, and a third, a correction of the forecast, made after they were scored): over the six held-back tests, the natural logarithm of the card's time over the measured time to reach 0.1, 0.2, 0.5, 1, 2 and 5 percent creep extension, root mean square at most 0.15, worst at most 0.30, mean at most 0.10 in size.".into(),
            CREEP_CMSX6_RESULT.into(),
            "One cast, one temperature, one laboratory. The card's six constants are fitted to six tests of this cast, and the law was chosen after the fitted tests were seen. Of the five structures registered beforehand it is the first to meet the three lines in both forecasts registered on the fitted tests: each fitted test predicted from the other five (0.112, 0.279, -0.001), and the 170 MPa tests predicted from 140 and 230 MPa alone (0.118, 0.229, -0.039). The record first gave that second forecast as a miss (0.229, 0.538, -0.113) and the first amendment dropped it as a gate. The miss did not happen: the forecast's integral was wrong at the constants that fit reached, the second scoring found it, and amendment 3 (SHA-256 99f461ffb0f589334d841590c456ba7c3027651f7a0249bf166ebcfe57e2e60f), written after the held-back tests were scored and so not a pre-registration, corrects it. The first amendment was not needed, the rule as first registered names this law, and no constant of the card changed. What the pass means: from 0.1 to 5 percent creep extension, inside 140 to 230 MPa, for this cast, on the six registered levels, the card is about as good as the measured test pieces are at predicting each other. Each of the twelve pieces predicted from the other two at its stress gives a root mean square of 0.075; the card's is 0.093. A straight-line interpolation of the fitted measurements, with no creep law, also meets the three lines on the same 33 times (0.098, 0.237, -0.013). 200 MPa lies where the stress dependence is straight: the minimum creep rate has a stress exponent near 6 from 170 to 200 MPa and from 200 to 230 MPa, and over 10 from 140 to 170 MPa, where no test was held back. The pass does not show that the law predicts a stress away from the fitted ones. The simplest registered structure, a Norton power law, misses on the same six tests (0.436 root mean square, 0.952 worst). At 0.5 percent the card is late by a mean of 0.095. Below 0.1 percent creep extension the card is not compared with measurement, and it starts later than every measured curve at all four stresses: in hours to 0.05 percent, the card against the three test pieces, 38.9 against 15.4 to 26.7 at 140 MPa, 7.8 against 2.7 to 3.5 at 170 MPa, 1.69 against 0.61 to 0.66 at 200 MPa and 0.245 against 0.06 to 0.08 at 230 MPa. The measured curves gain 0.02 to 0.04 percent within the first hour and the card stays near zero; its extension is under the measured one by at most 0.07 percent. The card refuses a state there as CreepExtensionBelowCard; until amendment 4 it answered there with a flag. At 140 MPa the three tests were stopped at 1010 to 1030 h, at 0.79 to 0.83 percent, so only the levels 0.1, 0.2 and 0.5 percent are compared there and the card's 1 to 2.09 percent at 140 MPa is beyond any measurement. Between 0.5 percent and where those tests stopped the card runs ahead of them: worked out after the scoring and not a registered level, it reaches 0.8 percent at 683 h where the two tests that got there took about 1000 h, and at the hour each test stopped it gives 1.70 to 1.78 percent. The card now ends at 0.2 percent below 170 MPa and refuses the step past it as CreepExtensionBeyondCard. How the range was drawn, with all twelve tests read after the scoring, on 171 levels from 0.10 to 5.00 percent in place of the six registered: at 170, 200 and 230 MPa the law's time since load is inside the registered worst line of 0.30 at every level on all nine tests, and the worst is 0.289, at 170 MPa and 0.33 percent on a held-back test, a margin of 0.011. At 140 MPa all three tests are inside the line at every level from 0.10 to 0.25 percent, where the worst is 0.296; one is outside it from 0.26 percent, a second from 0.31, and all three from 0.66 percent. The limit below 170 MPa is 0.2 percent, the largest registered level under that, where the worst is 0.196. No test lies between 140 and 170 MPa, so 5 percent is promised only from 170 MPa, the lowest stress at which it was measured and held, and the stress exponent between 140 and 170 MPa is where the law is least tested. The card's clock is the time since load because that is what holds: the law's time between two neighbouring levels is off by up to 1.06 in the logarithm against the measured curves, and its errors cancel only in the sum from zero. Its exponents, 33.4 and -27.2, are fitted numbers of this law and not physical constants of the alloy. Between repeat test pieces at one stress the measured time to 0.1 percent differs by up to 0.27 in the logarithm. No rupture, no extension beyond 5 percent, no other temperature, no changed or removed load (a changed stress is refused as StressNotHeld), no other crystal direction. Scored a second time on 2026-10-09 by a second agent with programs of its own (evidence/checks/second_party/V-24/): the same 33 values to 1e-12 and the same verdict, on the same machine. Not repeated on a second machine or by a second organisation.".into(),
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
            "The held-back trips are now spent, and they are an easier set than the opened ones: the proportional rule scores 0.18 on them and 0.337 on the opened 28 with the card's constants (0.35 with each trip left out of the fit in turn), which hold a trip followed by a second loss (22 December 2023) and the shallowest fall of all (9 May 2026), so the card's margin over that rule on the held-back trips is small. On the opened trips the card missed its mark: with each trip left out of the fit in turn the depth error was 0.302 against 0.25 and the error near 60 s was 0.097 Hz against 0.06, while the steepest second, 0.154, and the trace, 0.050 Hz, met their lines and the depth beat the proportional rule, 0.350. No structure of the protocol met every line; this is the one with the smallest depth error. The response the operator held on each day is not in the open data and is not an input: the card carries one response for nine years. Its errors follow the calendar: before 2021 its falls are about a quarter too shallow, after 2023 too deep. The depth error moves by +0.066 a year (standard error 0.013), from -0.30 in 2018 to +0.17 in 2025, nearly twice the 0.25 mark across the span. The system changed: Dynamic Containment from October 2020, 500 MW rising to 1,576 MW by July 2023; Dynamic Moderation and Dynamic Regulation from March 2022; the minimum inertia lowered in 2024. The card has one gain and one lag for all of it. The inertia is the operator's estimate, which leaves out the Stability Pathfinder units. It is the sharper input: 0.9 or 1.2 times the published inertia misses the line on the mean of the steepest second. The size of the loss is steady to 0.35 percent by the steadiness rule. The range was narrowed on 2026-10-10, after the scoring and with every trip seen, so the narrowing is not a new test. Losses: every scored trip lost 693 MW or more. The six trips of 500 to 692 MW inside the other spans, among those the scan lost, run through the engine by the protocol's lines, missed three of the seven: depth 0.2512 against 0.25, and against 0.2509 for the proportional rule, steepest second 0.273 against 0.25; the mean depth error -0.032, the steepest second's mean +0.057, near 60 s 0.054 Hz and the trace median 0.030 Hz met theirs. A loss under 693 MW is refused as LossOutsideCard. Frequency: no compared trip went under 49.553 Hz, so a frequency under 49.55 Hz is refused as FrequencyBelowCompared, where through engine 0.6.0 the card went on to 49.46 Hz with a flag. Two numbers are not fitted. They were first written from memory, and the second scoring found their sources. Demand's relief of approximately 2.5 percent per hertz: National Grid, System Operability Framework 2016, p. 70, and the NESO webinar of 12 March 2025; the documented range is 2.0 to 2.5. The 49.8 Hz knee is Dynamic Containment's (NESO, Response Services Service Terms v6; Dynamic Containment testing guidelines, August 2020): right for that one service from October 2020, and before it there were static blocks at 49.7 and 49.6 Hz. With the three constants refitted, a relief from 0 to 5 percent per hertz changes no verdict, and the pass holds for a knee of 49.80 to 49.90 Hz and is lost at 49.75. A run is one loss: a loss_mw in a later segment that is not the first segment's is refused as LossChangedDuringRun, where through engine 0.5.0 it was taken as the total level of loss from then on. No changing loss was compared with a measurement: the one measured trip with a second loss, that of 22 December 2023, is the card's worst. The recomputation outside the engine failed against its criterion as first written, in 39 of 80 runs: only the response and the two integrals were outside, the response by at most 4.2e-10 MW, and the frequency was never above 0.05 of the bound. The yardstick was changed afterwards by the card's author. A run of exactly 60 s in decimal lands every step: 3,000 steps of 0.02 s land 3,000 and read 60.00000000000378 s. Through engine 0.5.0 the last was refused at some step sizes, because the clock is summed in binary; the comparison with 60 s now carries the published rounding allowance, and no answer that landed before has changed. Known defect of the scan that found the trips: its rule of 0.15 Hz or more lost four falls of exactly 0.150 Hz to rounding. One frequency for the whole area; no network, no generator that trips as a consequence, nothing after 60 s. One party wrote the card and ran it. It was scored twice, the second time by a second scorer, on one machine.".into(),
        ],
        "tire_eve_r19_ice" => [
            "One 235/55 R19 all-season sport-utility tyre of the European project EVE, rolled on a prepared ice track by the Terramechanics Rig of the Advanced Vehicle Dynamics Laboratory at Virginia Tech in April 2016, at a creep of about 0.06 m/s: the ratio of its longitudinal force to its normal load at a held slip, driven and retarded.".into(),
            "Schleinin, Savitski, Jimenez, Sandu and Ivanov, Dataset of tyre tests on ice, parts 1 to 3, Zenodo, doi 10.5281/zenodo.1009017, 10.5281/zenodo.1009023 and 10.5281/zenodo.1009025, CC BY 4.0: 252 runs, each stepping the wheel through four slips, the first three for about 21 s each and the last until the wheel stopped, 21 to 33 s. The card's three constants were fitted to 72 runs at 3.5 and 5 kN and at 1.5 and 3.5 bar. Held back from that fitting: all 54 runs at 6.5 kN, a load above every fitted load, and all 36 runs at 2.5 bar at the two fitted loads, a pressure between the two fitted pressures. The 90 runs on a water film with a steered wheel were left out.".into(),
            "Fixed before any held-back run was opened (pre-registration SHA-256 84a380a5c948cd741ac91845fbfeac7ed1b60ed3c6a01855bcb99c996e7782c1), on the 60 held-back cells, each the mean over three repeats of the last nine seconds of a slip stage at 5, 6, 7, 12, 15 or 20 percent slip: root-mean-square error of the friction ratio at most 0.045 over all 60 cells, over the 24 cells at 2.5 bar and over the 36 cells at 6.5 kN, and over the 60 cells smaller than the error of a ratio of 0.1 that uses no measurement of this tyre and smaller than the error of the opened runs' mean with no slip term. That mean is the geometric mean of each mode, least squares on the logarithm, as the fit is; the arithmetic mean would have given 0.038, and the line is met either way.".into(),
            TIRE_EVE_R19_ICE_RESULT.into(),
            "The three constants are fitted to this tyre's own runs; nothing is derived from the rubber or the ice. The held-back runs at 2.5 bar test interpolation in pressure and those at 6.5 kN test extrapolation in load, on the same rig, the same ice and in the same campaign as the fitted runs: nothing here tests another ice temperature, another speed, another tyre or another laboratory. The files record no ice temperature, no carriage speed and no rolling radius, and the tyre's make is not published. The load and the pressure do not enter the friction ratio: on the opened runs a ratio that varied with them predicted a left-out condition worse than one that did not, and the differences between conditions, about 0.03, were three times the scatter between repeats, so part of what a condition shows is the state of the ice on the day. Slips under 5 percent and the rise of the force from rest are outside the card. The speed is a creep of 0.06 m/s: the friction of ice changes with sliding speed, and the card says nothing of a tyre on a road. The error is between conditions and not between repeats: a condition sits 0.01 to 0.06 from the card as a block, and the card cannot say which way the ice of a given session will sit. The error follows the test block, not the load or the pressure: the 18 condition means of the measured ratio, in six blocks, run from 0.185 to 0.287, and the standard deviation between them is 0.029, against 0.005 to 0.011 between repeats. The ice changed between sessions and within the first block, where the ratio rose by 0.010 to 0.024 from the first repeat to the third. Any model that does not know the block lands near 0.03, and the card is at that floor. A user should carry about ±0.03 (one standard deviation) on the ratio for the session. The figure of 0.23 to 0.24 is this tyre on that rig's ice in that fortnight at 0.06 m/s; it is not a figure for a tyre on ice at a stated temperature or at road speed. The slip term is real on the held-back runs and small: the exponent fitted to them alone is -0.107 ± 0.014, where the card holds -0.133, and the term is worth 0.002 to 0.003 of the 0.03 error. The longitudinal force at rest before a run, about -62 N with a standard deviation of 49 N, is not subtracted from the measured forces. The ledger of the three works closes for any constant, because each work is proportional to the force: it can catch a corrupted step, and it cannot catch a wrong constant, a wrong exponent or a wrong ice. A run whose end in decimal is exactly the track's 6 m lands every step: 1,000 steps of 0.1 s at 0.06 m/s, rolled as runs of 16 s each started from the distance the last returned, land 1,000 and read 6.000000000000005 m. Through engine 0.5.0 the last was refused, because the distance is summed in binary, and through engine 0.6.0 a start handed back from a run of small steps could be refused on one step to the end that small steps reached; the allowance at the end of the track is now the same at every step. The time after a slip change was not part of the registered comparison. Compared on 2026-10-10 by the protocol's cells and lines, with the change placed where the wheel speed steps, every one-second window from 5 s to 20 s after a change met all five lines, and every one before 5 s failed one; the card refuses a step before 5 s and after 21 s (evidence/research/tire_eve_235_55r19_ice/RESULTS.md). Engine 0.7.0 changed every answer of this card: a start states slip_held_s, the state returns it, and the step ceiling is 16 s. One party wrote the card and ran it. It was scored twice, the second time by a second scorer, on one machine.".into(),
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
            "A forecast is not a result. Predicted with constants fitted without it, six of the eight training pulses met the thermal-energy mark and all eight the fast-ion mark; the fusion constants rest on one pulse, 42762, and on the four training pulses that are not an even mix the fusion power met its mark on one, 43134 at a tritium fraction of 0.034 (16.5 percent of its peak), and missed on three, 42794, 42997 and 43002 at 0.75 to 0.93 (33, 65 and 40 percent); above a fraction of 0.504 the card therefore refuses, as FuelMixOutsideComparison, and so it does from 0.034 to 0.451, where no pulse was compared. The declared range is the envelope of the nine pulses of the card's regime run through the card, and every other region JET reached is refused by name: above 24.3 MW of heating, 2.44 MW of ohmic power, 3.84 MA, 4.04 T, 0.834 of the Greenwald density, 9.57 MJ of thermal and 1.62 MJ of fast-ion energy, 4.30 keV, a normalised beta of 2.21 and a confinement time of 5.72 s; below a density of 1.65e19 per cubic metre, a safety factor of 3.66, a normalised beta of 0.224 and a confinement time of 0.150 s; above a density of 7.85e19 per cubic metre; and after 5.475 s of the heated window. The states and loads only pulse 42976 reached are outside the declared range and refused by name, because it is a hot-ion pulse that was declared outside the card's regime before it was run and it missed all four marks; a run of it is refused before its first step, since its first load already has 3.1 MW of ohmic power, 4.13 MA and 1.50e19 per cubic metre. The card has one temperature, no profiles of its own, no ELMs and no radiation term; the density, the current and the field are the caller's. It does not know the species of the beams. The compared values are a reconstruction by the TRANSP code, whose own fusion power is within 10 to 20 percent of the measured neutron rate. Pulse 42976 is not an ELMy H-mode: it was declared outside the card's regime before it was run. Much of both pulses had been seen before the marks were fixed, as the pre-registration lists. One tokamak, one campaign, one maker. The comparison was scored a second time by a second agent of the same maker, given the protocol, the raw files and the saved answers and not the first scorer's scripts (evidence/checks/tokamak_jet_dte1_scoring/): the same eight results and the same two verdicts, every number agreeing to 15 figures; on engine 0.6.0 a third agent rebuilt it from the protocol and the raw files and reached the same numbers (evidence/checks/fresh_eyes_0.6.0/tokamak_jet_dte1/). No outside party has repeated it. The card's fusion power is superseded by the solver tokamak_dt_beam_target, which computes the D-T rate of one time slice from the plasma's profiles and its beams.".into(),
        ],
        "satellite_lageos1" => [
            "The laser-ranging satellite LAGEOS-1 in free flight, started from its published position and velocity at one instant. The same card, unchanged, was also run on the satellite LAGEOS-2.".into(),
            "International Laser Ranging Service combined orbits (product doi 10.5067/SLR/SLR_ILRSORB_001; Pearlman, Noll, Pavlis et al., 2019, J Geod 93, 2161-2180). Two LAGEOS-1 weeks and four LAGEOS-2 weeks were held back from the card's design.".into(),
            "Fixed before any model was scored: within 1 m of the published orbit after one revolution, 10 m after one day and 100 m after seven days.".into(),
            "Every line met on 42 runs, none refused. LAGEOS-1: worst distance 0.65 m after one revolution, 3.8 m after one day, 32 m after seven days. LAGEOS-2: 0.34 m, 2.9 m and 22 m.".into(),
            "The card's structure was chosen after a ladder of structures fixed in advance failed on two opened weeks; the tide, sunlight pressure and relativity were added afterwards. The held-back weeks are what make it a test: one LAGEOS-1 week and all four LAGEOS-2 weeks were fully blind; the other LAGEOS-1 week had been downloaded earlier and its analysis centres compared with each other, with no model involved. The published orbit is itself a fit by analysis centres to laser ranges, good to a few centimetres; it is not a raw measurement. The start state is turned into the card's frame outside the engine; a crust-fixed state fed in directly is about 20 m off within one revolution. Free flight only; compared on 28 calendar days, 2026-08-23 to 2026-09-19, for at most seven days from a published state, on the two orbits of LAGEOS-1 and LAGEOS-2 and nowhere between or beside them; the card refuses other dates, other orbits and longer forecasts by name. No drag, no light reflected from the Earth, no ocean tide; the Sun and the Moon are approximate. Run once and checked twice. Repeated independently on 2026-10-07 by Grok, another developer's model, with its own comparison and its own Earth-orientation code: the same six worst distances to the millimetre. Scored a third time from the protocol alone on 2026-10-10 by an agent that did not write the card, with its own Earth-orientation code: worst distances within 7.8 cm of the record.".into(),
        ],
        "crater_ryugu_sci" => [
            "The crater made on asteroid 162173 Ryugu on 5 April 2019 by the Small Carry-on Impactor of JAXA's Hayabusa2: a 2 kg copper projectile at 2 km/s into a rubble surface under a gravity of 9.3e-5 m/s2. The compared quantity is the crater's apparent radius, at the level of the original surface.".into(),
            "The law of crater size and its two constants: thirteen impact craters in dense dry Ottawa Flintshot sand at 1, 111 and 464 times Earth's gravity, Schmidt (1980), Proc. Lunar Planet. Sci. Conf. 11, 2099, Table 1, transcribed from the scan at the NASA Astrophysics Data System. The clock: seven points read by eye from Fig. 4 of Gault and Wedekind (1977), Impact and Explosion Cratering, 1231. The projectile and the target point: JAXA's press briefing of 2019-03-18, before the impact. Ryugu's mass, density, size and rotation: the JPL Small-Body Database, citing Watanabe et al. (2019). Held back, and no part of any constant: the measured crater, apparent diameter 14.5 +/- 0.8 m, Arakawa et al. (2020), Science 368, 67, as given in JAXA's summary of 2020-03-20.".into(),
            "Fixed before any centrifuge crater was fitted or predicted and before the report of the Ryugu crater was opened (protocol SHA-256 ee2793f1ce97b42ed07210121d4fb7cc2f3599db1db1a46a9530efd3cb9da9b3). In the laboratory: fitted on the nine craters made at 1 g, the law must put each of the four centrifuge craters, 2.5 to 4.5 decades away in scaled gravity, inside its 95 percent prediction interval, with a mean error of at most 10 percent. On Ryugu: the radius at which the card stops must be within an allowance worked out by rule from the fit's 95 percent prediction interval at Ryugu, the measurement's stated uncertainty and the uncertainty of the local gravity.".into(),
            CRATER_RESULT.into(),
            "Nothing is fitted to Ryugu, and Ryugu lies 2.5 decades below the smallest laboratory crater in scaled gravity: the comparison is an extrapolation, in gravity and in material. The crater's measured diameter has been public since 2019 and was known to the writer of the protocol, which is why the Ryugu line is a rule and not a chosen number. Nothing in this comparison was blind: one known measured number and a rough prediction of 8.6 m were in hand before the protocol's hash was recorded. One crater, measured once. The measured crater is a half circle, stopped on its south side by a block, and the projectile came in at 60 degrees to the surface; the card is a full circle from a square hit. The impactor was a hollow copper shell 13 cm across, of 2 kg; the card models it as a solid copper sphere of the same mass, of radius 3.76 cm, at the full 2 km/s and under gravity reduced by the rotation. With the projectile taken as the 13 cm shell the same law gives 7.83 m, 8.0 percent above the measurement and inside the allowance; with the published team's own inputs (the shell, 2 km/s, g = 1.2e-4 m/s2) it gives 7.50 m, 3.5 percent above. Across twenty reasonable readings of the inputs the prediction runs from 7.15 to 8.80 m, wider than the miss. The projectile's radius and the 60-degree angle each move the prediction more than the rotation term does, and neither is in the allowance. The miss stands as registered: it is a miss of the scaling of dense Ottawa Flintshot sand with these inputs, and it does not show that Ryugu's crater departs from sand scaling. The measurement was read from JAXA's summary of the paper, not from the paper. The card has no cohesion, no boulders, no rim, no depth and no ejecta. Its time is a laboratory clock that runs until the ejecta near the rim have landed, read by eye from a printed figure; no line was set on time. Its time constant, 1.318295773573555, comes from seven points read by eye at 1 g; the card stops at 400 s, where the open sources say growth for about 200 s and a stop by about 300 s. The clock is untested. The first 1.75 m of growth is outside the card. One party wrote the card and ran it. It was scored twice, the second time by a second scorer, on one machine.".into(),
        ],
        "solar_system" => [
            "The Solar System: the Sun, the eight planets, the Moon and the asteroids Ceres, Pallas and Vesta, carried forward from one state, that of 1999-01-01 00:00 TDB.".into(),
            SOLAR_SYSTEM_DATA.into(),
            SOLAR_SYSTEM_PASS_LINE.into(),
            SOLAR_SYSTEM_RESULT.into(),
            SOLAR_SYSTEM_CAVEATS.into(),
        ],
        _ => [
            format!("{specimen} A generic specimen, not a named real object."),
            "none".into(),
            "none".into(),
            NOT_COMPARED.into(),
            "The run obeys the card's own equations and its conservation ledger; that has been checked by an outside recomputation in arithmetic of 50 digits or more. Nothing is stated about how closely it matches any real object.".into(),
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
    Value::Object(record)
}

/// A defect of a card that is known, stated in the limits of its validation
/// record, and not fixed in this engine.
pub(crate) fn known_defect(card: &str) -> Option<&'static str> {
    match card {
        _ => None,
    }
}

fn describe_one(item: &Card) -> Value {
    let (example_verify_step, example_run_trajectory) =
        worked_examples(item.domain, item.card);
    let mut body = describe_fields(item, example_verify_step, example_run_trajectory);
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

fn describe_fields(item: &Card, example_verify_step: Value, example_run_trajectory: Value) -> Value {
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
        "rejects": item.rejects.iter().map(|name| json!({
            "name": name,
            "meaning": reject_meaning(item.card, name),
        })).collect::<Vec<_>>(),
        "declared_ends": crate::ends::ENDS.iter().filter(|end| end.card == item.card).map(|end| json!({
            "reject": end.reject,
            "end": end.end,
            "outcome": "ended",
        })).collect::<Vec<_>>(),
        "example_verify_step": example_verify_step,
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
/// finger pad. Every other flag of the engine is a warning.
fn flag_is_event(card: &str, name: &str) -> bool {
    matches!((card, name), ("fingerpad_p02_glass", "FLAG_FULL_SLIP"))
}

/// The boolean in the state that sets each advisory flag.
fn flag_state(name: &str) -> &'static str {
    match name {
        "FLAG_GRID_SURGE" => "surge",
        "FLAG_PLL_SLIP" => "pll_slipped",
        "FLAG_SAG_WARNING" => "sag_warning",
        "FLAG_THERMAL_DERATE" => "derated",
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
        "FLAG_NEAR_DENSITY_LIMIT" => "near_density_limit",
        "FLAG_FULL_SLIP" => "full_slip",
        "FLAG_PRESSURE_OUTSIDE_FITTED" => "pressure_outside_fitted",
        _ => unreachable!("a flag with no state"),
    }
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

/// What the time label is, said once and the same on every card.
pub(crate) const TIME_LABEL_SENTENCE: &str = "The time label t_s is the running sum of the steps taken, t_s += dt_s in 64-bit arithmetic, not the step count times the step: ten steps of 0.1 s read 0.9999999999999999 s. The t slot of the frame and every hash over the frame carry this sum.";

fn time_label(item: &Card) -> String {
    if item.card == "satellite_lageos1" {
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
        ("aerospace", "crater_ryugu_sci") => crater_examples(),
        ("signal", "link_gothenburg_10010") => link_examples(),
        ("energy", "grid_gb") => grid_gb_examples(),
        ("aerospace", "solar_system") => solar_system_examples(),
        ("materials", "tire_eve_r19_ice") => tire_eve_r19_ice_examples(),
        ("aerospace", "los_jwst_fine_guide") => los_examples(),
        ("energy", "tokamak_jet_dte1") => tokamak_examples(),
        ("materials", "fingerpad_p02_glass") => fingerpad_examples(),
        ("energy", "accumulator_sbo450") => accumulator_sbo450_examples(),
        _ => unreachable!("descriptor pair"),
    }
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

fn grid_examples() -> (Value, Value) {
    let node = grid_node::GridNode::copper_fuse_link();
    let state = grid_node::GridNodeState::try_rest(&node).expect("grid rest");
    pair_examples(
        "energy",
        "grid_node",
        obj(&[
            ("current_a", scalar_value(state.current_a)),
            ("voltage_v", scalar_value(state.voltage_v)),
            ("temperature_k", scalar_value(state.temperature_k)),
        ]),
        obj(&[
            ("upstream_voltage_v", scalar_value(state.upstream_voltage_v)),
            ("load_current_a", scalar_value(state.load_current_a)),
            ("ambient_k", scalar_value(state.ambient_k)),
        ]),
        card_dt(node.gate_dt_s()),
    )
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
fn crater_examples() -> (Value, Value) {
    let crater = crater_ryugu_sci::CraterRyuguSci::sci_impact();
    pair_examples_recorded(
        "aerospace",
        "crater_ryugu_sci",
        obj(&[("radius_m", scalar_value(crater_ryugu_sci::RADIUS_FLOOR_M))]),
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
        card_dt(link.gate_dt_s()),
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
    let tire = tire_eve_r19_ice::TireEveR19Ice::on_rig_ice();
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
        // Four steps of a quarter of the ceiling: the hold from 5 s to 21 s.
        scalar_value(tire.gate_dt_s().expect("specimen gate") / 4.0),
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
        ("solar_system", "DistanceOutsideComparedRun") => {
            "A body's distance from its primary, incoming or landing, is outside the distances the compared run went through up to the last range compared with measurement, 16.2 years after the 1999 state: Mercury 0.30 to 0.47 au, Venus 0.71 to 0.73, the Earth 0.98 to 1.02, the Moon 356 000 to 407 000 km, Mars 1.38 to 1.67, Jupiter 4.94 to 5.46, Saturn 9.03 to 9.97, Uranus 19.88 to 20.10, Neptune 29.96 to 30.14, Ceres 2.54 to 2.99, Pallas 2.13 to 3.42, Vesta 2.14 to 2.58 au."
        }
        ("solar_system", "BodiesCoincide") => {
            "Two bodies, incoming or landing, are at exactly one point, where the attraction between them has no value."
        }
        ("solar_system", "BodiesTooClose") => {
            "Two bodies other than the Earth and the Moon, incoming or landing, are closer than 0.26 au, or two of the three asteroids closer than 0.38 au: closer than any pair in the compared run."
        }
        ("solar_system", "OriginOutsideCard") => {
            "The centre of mass of the thirteen bodies, incoming or landing, is more than 120 000 m from the origin."
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
        ("cell_18650pf", "CurrentNotCompared") => {
            "Current is outside 2.89817 to 2.90065 A, the 1C currents compared with measurement widened by 0.83 mA. Any other current is refused by this name, the held 0.87 A among them, which was compared and failed."
        }
        ("cell_18650pf", "CurrentChangedDuringDischarge") => {
            "The current is more than 0.83 mA from the held current, set by the run's first step or carried in from the discharge it continues: the card is of one held current."
        }
        ("cell_18650pf", "StartNotAtFullCharge") => {
            "State of charge is below 1 with no held current: every compared discharge started at full charge."
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
        ("resonance", "AmplitudeAboveCard") => {
            "Mechanical energy, incoming or landing, is above that of a free ring of twice the stroke."
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
        ("generator_mmrtg_f1", "MissionTimeOutsideCard") => {
            "Mission time, incoming or landing, is before landing or after mission day 4,087; the landing by more than the rounding tolerance of the clock, 2.38e-7 s, the same for every step."
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
            "Altitude, incoming, at a stage of the step, or landing, is above 134.7 km or below 30 km."
        }
        ("capsule_stardust", "MachOutsideCard") => {
            "Mach number, incoming, at a stage of the step, or landing, is below 1.0, or is above 40 below 68.96 km, where the drag table by Mach number ends."
        }
        ("capsule_stardust", "AirSpeedOutsideCard") => {
            "Air speed, incoming, at a stage of the step, or landing, above 68.96 km is below 11900 m/s or above 12575 m/s."
        }
        ("capsule_stardust", "PastFlightDeployment") => {
            "The incoming state is under 31030 m and under Mach 1.23 at once: past the flown drogue deployment."
        }
        ("capsule_stardust", "OffComparedCorridor") => {
            "The state, incoming or landing, below 68.96 km or moving upward above it, is off the corridor the locked entry flew: under 31 km, moving upward above 69 km, or its air speed or path angle is outside the corridor at its altitude."
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
            "Altitude, incoming, at a stage of the step, or landing, is above 128 km or below 4 km."
        }
        ("capsule_insight", "MachOutsideCard") => {
            "Mach number, incoming, at a stage of the step, or landing, is below 1.0."
        }
        ("capsule_insight", "AirSpeedOutsideCard") => {
            "Air speed, incoming, at a stage of the step, or landing, is above 5850 m/s."
        }
        ("capsule_insight", "PastComparedFlight") => {
            "Altitude, incoming, at a stage of the step, or landing, is below 5700 m, or Mach number is below 1.46: past the lowest state the compared run reached."
        }
        ("capsule_insight", "OffComparedCorridor") => {
            "The state, incoming or landing, is off the corridor the compared runs from the entry state flew: above 126.5 km, or its air speed or path angle is outside the corridor at its altitude."
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
        ("satellite_lageos1", "RadiusOutsideCard") => {
            "Distance from the Earth's centre, incoming or landing, is below 11 997 km or above 12 327 km."
        }
        ("satellite_lageos1", "OrbitOutsideCard") => {
            "The orbit, incoming or landing, is not closed, or is in neither of the two compared orbits: LAGEOS-1, semi-major axis 12 265 to 12 275 km, eccentricity 0.0034 to 0.0044 and inclination 109.80 to 109.83 degrees, and LAGEOS-2, 12 158 to 12 166 km, 0.0132 to 0.0138 and 52.63 to 52.66 degrees."
        }
        ("satellite_lageos1", "EpochOutsideCard") => {
            "The time, incoming or landing, is before 2026-08-23 00:00 or after 2026-09-20 00:00 UTC; the landing by more than the rounding allowance of the running sum of the clock: 1.19e-7 s, and one part in 2^53 of the clock for each step taken at this step and two more."
        }
        ("satellite_lageos1", "HorizonOutsideCard") => {
            "The forecast, incoming or landing, is more than seven days, 604 800 s, from the state taken from a published orbit, or the stated time since that state is below zero; the landing by more than the same rounding allowance, counted over the whole forecast."
        }
        ("satellite_lageos1", "SpontaneousGeneration") => {
            "The trial step gained energy that the work of the Sun, the Moon, the tide, sunlight and relativity does not account for."
        }
        ("satellite_lageos1", "FirstLawBreach") => {
            "The trial step lost energy that the work of the Sun, the Moon, the tide, sunlight and relativity does not account for."
        }
        ("grid_gb", "NonFiniteInput") => "The power lost or `dt` was NaN or infinite.",
        ("grid_gb", "NonFiniteState") => "The incoming state, or the trial step, was non-finite.",
        ("grid_gb", "NonPositiveDt") => "`dt` was zero or negative.",
        ("grid_gb", "InvalidCard") => "A constant is not this card's.",
        ("grid_gb", "LossOutsideCard") => {
            "The power lost is outside 693 to 1402 MW, the losses compared with measurement."
        }
        ("grid_gb", "StartFrequencyOutsideCard") => {
            "The starting frequency is outside 49.85 to 50.15 Hz."
        }
        ("grid_gb", "InertiaOutsideCard") => {
            "The stored kinetic energy at 50 Hz is outside 111,000 to 337,000 MW s."
        }
        ("grid_gb", "DemandOutsideCard") => "The demand is outside 18,163 to 40,829 MW.",
        ("grid_gb", "LossChangedDuringRun") => {
            "The power lost is not the loss of the run's first step: a run is one loss at time zero, held for the 60 seconds."
        }
        ("grid_gb", "TimeBeyondCard") => {
            "The step would end later than 60 s after the start, by more than the rounding allowance of the running sum of the clock: 3.55e-15 s for each step a run of this step takes to reach 60 s, and two more."
        }
        ("grid_gb", "LowFrequencyDemandDisconnection") => {
            "The frequency, incoming or landing, is at or below 48.8 Hz, where low-frequency demand disconnection begins."
        }
        ("grid_gb", "FrequencyBelowCompared") => {
            "The frequency, incoming or landing, is below 49.55 Hz, the lowest frequency compared with measurement."
        }
        ("grid_gb", "FrequencyAboveCard") => {
            "The frequency, incoming or landing, is above 50.5 Hz, the upper statutory limit."
        }
        ("grid_gb", "SpontaneousGeneration") => "The trial step created kinetic energy from nothing.",
        ("grid_gb", "FirstLawBreach") => "The trial step destroyed kinetic energy.",
        ("reactor_spert3", "NonFiniteInput") => {
            "`dt` or the held reactivity was NaN or infinite."
        }
        ("reactor_spert3", "NonFiniteState") => {
            "The incoming state, or the trial step, was non-finite."
        }
        ("reactor_spert3", "NonPositiveDt") => "`dt` was zero or negative.",
        ("reactor_spert3", "InvalidCard") => "A recorded constant is not this reactor's.",
        ("reactor_spert3", "ReactivityOutsideCard") => {
            "The held reactivity is below 0.7686211839115998 dollar or above 1.2102315121058187 dollars, the inhour values of the longest and the shortest tested reactor period."
        }
        ("reactor_spert3", "CoolantTemperatureOutsideCard") => {
            "The recorded primary coolant temperature is outside 56 to 92 degF."
        }
        ("reactor_spert3", "PastFirstMaximum") => {
            "The state is past the first maximum of power; nothing after it is on the card."
        }
        ("reactor_spert3", "TimeBeyondCard") => "The step would end after 20 s.",
        ("reactor_spert3", "EnergyBeyondCard") => "The trial step released more than 10 MW s.",
        ("reactor_spert3", "PowerBeyondCard") => "The trial step reached more than 1000 MW.",
        ("reactor_spert3", "ReactivityChangedDuringBurst") => {
            "The reactivity of the load is not the one of the first step of this burst; a burst is one step in reactivity, held to the first maximum of power."
        }
        ("reactor_spert3", "NegativeState") => {
            "The incoming state holds an energy released or a precursor sum below zero, or a power that is not above zero."
        }
        ("reactor_spert3", "InconsistentState") => {
            "The incoming state is not one this card writes: at time zero it is not the card's start, or a field the card derives from the rest of the state does not hold the value the card derives."
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
        ("reactor_spert3_ccore", "InitialTemperatureOutsideCard") => {
            "The initial temperature is outside the compared 21 to 36 degC."
        }
        ("reactor_spert3_ccore", "PressureOutsideCard") => {
            "The system pressure is outside the compared 0 to 2500 psig."
        }
        ("reactor_spert3_ccore", "FlowOutsideCard") => {
            "The coolant flow is outside the compared: none, or 200 to 20,000 gpm (0.054864 to 5.4864 m/s) at 230 psig only."
        }
        ("reactor_spert3_ccore", "ReactivityOutsideCard") => {
            "The reactivity is outside the compared: the inhour values of 9.3 s to 9.7 ms with no flow, 516 ms to 9.7 ms with flow."
        }
        ("reactor_spert3_ccore", "PastPlateTemperatureMaximum") => {
            "The state is past the first maximum of the hot-spot plate temperature after the power peak; nothing after it is on the card."
        }
        ("reactor_spert3_ccore", "TimeBeyondCard") => "The step would end after 200 s.",
        ("reactor_spert3_ccore", "NegativeState") => {
            "The incoming state holds an energy, a temperature rise or a precursor sum below zero, or a power not above zero."
        }
        ("reactor_spert3_ccore", "EnergyBeyondCard") => {
            "The incoming state or the trial step holds more than 1e5 MW s released."
        }
        ("reactor_spert3_ccore", "PowerBeyondCard") => {
            "The incoming state or the trial step holds a power, or a peak power, above 2000 MW."
        }
        ("reactor_spert3_ccore", "ReactivityChangedDuringRun") => {
            "The reactivity of the load is not the one of the first step of this run; a run is one step of reactivity."
        }
        ("reactor_spert3_ccore", "FlowChangedDuringRun") => {
            "The coolant flow of the load is not the one of the first step of this run; no compared test changed its flow."
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
        ("reactor_triga_jsi", "NonFiniteInput") => {
            "`dt`, the reactivity or the scram value was NaN or infinite."
        }
        ("reactor_triga_jsi", "NonFiniteState") => {
            "The incoming state, or the trial step, was non-finite."
        }
        ("reactor_triga_jsi", "NonPositiveDt") => "`dt` was zero or negative.",
        ("reactor_triga_jsi", "InvalidCard") => "A recorded constant is not this reactor's.",
        ("reactor_triga_jsi", "StartTemperatureOutsideCard") => {
            "The recorded starting fuel temperature is outside the starts compared on its rod table at 2.0 printed dollars and above: 24.00 to 27.94 degC on core 215, 21.31 to 29.23 on core 217, 20.90 to 22.81 on core 231."
        }
        ("reactor_triga_jsi", "RodTableOutsideCard") => {
            "The recorded rod-worth table is not that of core 215, 217 or 231."
        }
        ("reactor_triga_jsi", "ReactivityOutsideCard") => {
            "The inserted reactivity is outside 2.0 to 3.0 printed dollars."
        }
        ("reactor_triga_jsi", "ScramValueOutsideCard") => {
            "The scram value is neither 0 (rods held) nor 1 (rods dropped)."
        }
        ("reactor_triga_jsi", "ReactivityChangedDuringRun") => {
            "The inserted reactivity differs from the one this run started with; the model defines no change of it but the scram."
        }
        ("reactor_triga_jsi", "ScramBeforeRodStopped") => {
            "The rods were dropped while the transient rod was still moving."
        }
        ("reactor_triga_jsi", "ScramBeforeCompared") => {
            "The rods were dropped earlier than 1.50 s after the peak power, earlier than any scram compared with a record."
        }
        ("reactor_triga_jsi", "RodsHeldBeyondCompared") => {
            "The step would end with the rods still held more than 7.45 s after the peak power, later than any scram compared with a record."
        }
        ("reactor_triga_jsi", "RodsRaisedAfterScram") => "The rods were raised after the scram.",
        ("reactor_triga_jsi", "NegativeEnergy") => {
            "The energy released in the handed-in state is negative."
        }
        ("reactor_triga_jsi", "ImpossiblePeak") => {
            "A peak field of the handed-in state cannot have come from a run: a peak power below the power, a peak time or peak energy outside the run so far, or a highest fuel temperature below the fuel temperature."
        }
        ("reactor_triga_jsi", "StateOutsideCard") => {
            "The handed-in state has a power that is not positive, a negative precursor sum, a negative time, a fuel temperature below the starting temperature, or a scram time or scram energy outside the run so far."
        }
        ("reactor_triga_jsi", "TimeBeyondCard") => "The step would end after 14 s.",
        ("reactor_triga_jsi", "EnergyBeyondCard") => {
            "The trial step released more than 20 MW s, beyond what was compared with a record."
        }
        ("reactor_triga_jsi", "PowerBeyondCard") => {
            "The trial step reached more than 1000 MW, beyond what was compared with a record."
        }
        ("reactor_triga_jsi", "FuelTemperatureBeyondCard") => {
            "The trial step took the core-average fuel temperature above 600 K, beyond what was compared with a record."
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
        ("reactor_spert3_fuel", "NonPositiveDt") => "`dt` was zero or negative.",
        ("reactor_spert3_fuel", "InvalidCard") => "A recorded constant is not this reactor's.",
        ("reactor_spert3_fuel", "InletTemperatureOutsideCard") => {
            "The recorded inlet temperature is in none of the three tested bands: 56 to 92, 251 to 259, 498 to 501 degF."
        }
        ("reactor_spert3_fuel", "ReactivityOutsideCard") => {
            "The reactivity is outside the span tested in this band."
        }
        ("reactor_spert3_fuel", "FlowOutsideCard") => {
            "The coolant flow rate is outside the span tested in this band: none in the cold band, 2.4 to 22 ft/s at 260 degF and 4.8 to 24 ft/s at 500 degF."
        }
        ("reactor_spert3_fuel", "PastFirstMaximum") => {
            "The state is past the first maximum of power; nothing after it is on the card."
        }
        ("reactor_spert3_fuel", "TimeBeyondCard") => "The step would end after 20 s.",
        ("reactor_spert3_fuel", "NegativeState") => {
            "The incoming state holds an energy released, a temperature rise or a precursor sum below zero, or a power that is not above zero."
        }
        ("reactor_spert3_fuel", "EnergyBeyondCard") => {
            "The incoming state or the trial step holds more than 30 MW s released."
        }
        ("reactor_spert3_fuel", "PowerBeyondCard") => {
            "The incoming state or the trial step holds a power, or a peak power, above 1000 MW."
        }
        ("reactor_spert3_fuel", "ReactivityChangedDuringBurst") => {
            "The reactivity of the load is not the one the first step of this burst sent the rod to; a burst is one insertion, ramped in and then held."
        }
        ("reactor_spert3_fuel", "FlowChangedDuringBurst") => {
            "The coolant flow rate of the load is not the one of the first step of this burst; no tested burst changed its flow."
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
        ("cell_2170_m80", "BlockShownWrong") => {
            "The block of the state is the -10 or the -20 degC block, which the comparison with measurement showed wrong."
        }
        ("cell_2170_m80", "CaseTemperatureOutsideCard") => {
            "Case temperature is below 0.808 degC or above 45.535 degC."
        }
        ("cell_2170_m80", "CaseTemperatureOutsideBlock") => {
            "Case temperature is outside the band at which the block of the state was compared with measurement."
        }
        ("cell_2170_m80", "CurrentOutsideCard") => {
            "The current is above 9.737 A charging or above 11.375 A discharging."
        }
        ("cell_2170_m80", "CurrentOutsideBlock") => {
            "The current is outside the currents at which the block of the state was compared with measurement."
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
        ("cantilever_eth", "DeflectionAboveCard") => {
            "The deflection envelope is above 15.79 mm, the largest the compared run held, at the start of the step or where the trial step lands."
        }
        ("cantilever_eth", "DeflectionBelowCard") => {
            "The deflection envelope is below 0.058 mm, the smallest the card was compared at, at the start of the step or where the trial step lands."
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
        ("link_gothenburg_10010", "RainRateOutsideCard") => {
            "The rain rate is below zero or above 84 mm/h, the highest one-minute rate in the fitting data."
        }
        ("link_gothenburg_10010", "RainRateBelowGaugeCount") => {
            "The rain rate is above zero and below 6 mm/h, one count of the gauge in a minute: the gauge gives no such one-minute rate, and none was compared with the link."
        }
        ("link_gothenburg_10010", "RainNotHeldForGaugeMinute") => {
            "The rain rate changed inside a gauge minute: it is not the rate held since the minute began, and only rain held through whole gauge minutes was compared with the link."
        }
        ("link_gothenburg_10010", "GaugeMinuteOutsideCard") => {
            "The seconds already spent in the current gauge minute are below zero or not below 60 s."
        }
        ("link_gothenburg_10010", "DryPathLossOutsideCard") => {
            "The dry level is outside 50 to 54 dB."
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
        ("los_jwst_fine_guide", "LockOutsideCard") => {
            "The lock point is outside the span the comparison covered: from -61.1512947 to 60.4564934 arcsec on x, from -64.2767 to 50.1812668 arcsec on y."
        }
        ("los_jwst_fine_guide", "RecoveryNotElapsed") => {
            "The run starts less than 30 s after the last centroid the caller states was refused or more than 5 mas from the lock point."
        }
        ("los_jwst_fine_guide", "InvalidCovariance") => {
            "A variance in the incoming state, or the variance stated for a centroid, was zero or negative."
        }
        ("los_jwst_fine_guide", "MeasuredNotZeroOrOne") => "`measured` was neither 0 nor 1.",
        ("los_jwst_fine_guide", "UnmeasuredFrameCarriesNumbers") => {
            "A frame with no measurement carried a centroid or a count rate that was not zero."
        }
        ("los_jwst_fine_guide", "MeasurementGapOutsideCard") => {
            "The frame has no measurement and would make more than 10 frames in a row with none, the longest run of them the comparison crossed."
        }
        ("los_jwst_fine_guide", "CountRateOutsideCard") => {
            "The guide star's count rate is below 22,900 or above 2,820,000 counts per second, the range the constants were fitted on."
        }
        ("los_jwst_fine_guide", "OffsetOutsideCard") => {
            "The centroid is more than 250 mas from the lock point on either axis."
        }
        ("los_jwst_fine_guide", "PointingDisturbance") => {
            "A measured centroid is more than 5 mas from the lock point on either axis: the pointing is disturbed, and there the card is worse than the last centroid."
        }
        ("los_jwst_fine_guide", "InnovationBeyondFourSigma") => {
            "A measured centroid is more than four stated standard deviations from where it was expected, on either axis."
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
        ("cell_2170_model3", "CapacityOutsideCard") => "Capacity is below 4.16 Ah or above 4.70 Ah.",
        ("cell_2170_model3", "CaseTemperatureOutsideCard") => {
            "Case temperature is below -12.9 degC or above 45.9 degC."
        }
        ("cell_2170_model3", "CurrentOutsideCard") => {
            "The current is above 13.524 A discharging or above 13.984 A charging."
        }
        ("cell_2170_model3", "CapacityOutsideCardAtTemperature") => {
            "No block of tests holds both the capacity and the case temperature: the capacity is outside the band scored at the case temperatures that block spanned."
        }
        ("cell_2170_model3", "CaseTemperatureChangeAboveCard") => {
            "Case temperature differs from the last step's by more than 5.4 K."
        }
        ("cell_2170_model3", "CaseTemperatureSpanAboveCard") => {
            "Case temperature would take its span since the rested start, carried across a continued start, past 19.2 K."
        }
        ("cell_2170_model3", "SustainedCurrentOutsideCard") => {
            "The landing running mean of the current is above 8.722 A discharging or above 2.993 A charging."
        }
        ("cell_2170_model3", "StateOfChargeOutsideCard") => {
            "State of charge, incoming or landing, is below 0 or above 1.02 by more than the rounding allowance of its running sum."
        }
        ("cell_2170_model3", "SurfaceOutsideCurve") => {
            "Landing surface state of charge is below -0.05 or above 1.05, the ends of the measured curves."
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
        ("creep_cmsx6", "CreepExtensionBelowCard") => {
            "The incoming creep extension is below 0.1 percent, where the law is slow to start against every measured curve."
        }
        ("creep_cmsx6", "CreepExtensionOutsideCard") => {
            "The incoming creep extension is above the limit for the held stress: 0.2 percent below 170 MPa, 5 percent from 170 MPa."
        }
        ("creep_cmsx6", "CreepExtensionBeyondCard") => {
            "The trial step passed the limit of creep extension for the held stress: 0.2 percent below 170 MPa, 5 percent from 170 MPa."
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
        ("crater_ryugu_sci", "RadiusBelowCard") => {
            "The radius is under 1.75 m, the smallest the card covers."
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
        ("tire_eve_r19_ice", "NormalLoadOutsideCard") => {
            "The normal load is below 3 500 N or above 6 500 N, the loads measured."
        }
        ("tire_eve_r19_ice", "InflationPressureOutsideCard") => {
            "The inflation pressure is below 150 kPa or above 350 kPa, the pressures measured."
        }
        ("tire_eve_r19_ice", "SlipOutsideCard") => {
            "The slip magnitude is below 0.05 or above 0.20, the slips the constants cover."
        }
        ("tire_eve_r19_ice", "CarriageSpeedOutsideCard") => {
            "The carriage speed is below 0.05 m/s or above 0.07 m/s, the creep of the rig."
        }
        ("tire_eve_r19_ice", "TravelBeyondTrack") => {
            "The distance along the track is below zero, or the step ends beyond 6 m, the length rolled in a run, by more than the rounding allowance of the running sum: 4.44e-16 m for each of the 122,880 steps of the step floor at 0.05 m/s along 6 m, and six more, 5.5e-11 m at every step."
        }
        ("tire_eve_r19_ice", "SlipNotSettled") => {
            "The slip has been held less than 5 s at the start of the step: a start that states less, or the first step after a change of slip. The force in the first 5 s after a slip is set failed the comparison with the measurements."
        }
        ("tire_eve_r19_ice", "HoldBeyondMeasured") => {
            "The step ends more than 21 s after the slip was set, beyond the rounding allowance of the running sum: no scored slip was compared beyond one 21 s stage."
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
        ("fingerpad_p02_glass", "NormalForceOutsideCard") => {
            "The normal force is below 1.93 N or above 2.07 N: the card holds its mark only in the block of trials at 2 N, within the spread of the robot's force controller."
        }
        ("fingerpad_p02_glass", "TwistRateOutsideCard") => {
            "The twist rate is below 5 or above 50 degrees a second: the card holds its mark at 2 N only at those rates."
        }
        ("fingerpad_p02_glass", "LoadChangedDuringTwist") => {
            "The normal force or the twist rate differs from the one this twist began with. No measured twist changed either."
        }
        ("fingerpad_p02_glass", "TwistNotStartedFromZero") => {
            "The twist at the start is not zero. Every measured twist began with the pad untwisted, and the card cannot see the load a pad already twisted was brought there under."
        }
        ("fingerpad_p02_glass", "TwistBeyondMeasured") => {
            "The twist is below zero, or the step ends beyond 75 degrees, the end of the twist compared with measurement, past which the measured plate slowed to a stop at 80."
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
        ("accumulator_sbo450", "GasMassOutsideCard") => {
            "The gas mass is neither of the two charges measured: 0.013639 to 0.013667 kg, a precharge of 20 bar, or 0.02744 to 0.027495 kg, a precharge of 40 bar."
        }
        ("accumulator_sbo450", "WallTemperatureOutsideCard") => {
            "The wall temperature is below 293.4 K or above 293.7 K: no other wall temperature was compared with measurement."
        }
        ("accumulator_sbo450", "PistonSpeedOutsideCard") => {
            "The piston speed is above 0.7 m/s, either way: no faster motion was compared with measurement."
        }
        ("accumulator_sbo450", "PistonTravelAboveStep") => {
            "The piston would travel more than 1 mm in the step."
        }
        ("accumulator_sbo450", "GasTemperatureOutsideCard") => {
            "The gas temperature, incoming or landing, is below 285.9 K or above 301.8 K: no other gas temperature was compared with measurement."
        }
        ("accumulator_sbo450", "PressureOutsideCard") => {
            "The gas pressure, incoming or landing, is below 37 bar or above 86 bar."
        }
        ("accumulator_sbo450", "PressureOutsideChargeBands") => {
            "The gas pressure, incoming or landing, is inside 37 to 86 bar and outside the bands compared with measurement for the charge given: 37 to 43 or 55 to 67 bar for the 20 bar charge, 57 to 64 or 75 to 86 bar for the 40 bar charge."
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
        ("tokamak_jet_dte1", "HeatingPowerOutsideCard") => {
            "The heating power is below 0.124 MW or above 24.3 MW, the span of the nine compared pulses of the card's regime."
        }
        ("tokamak_jet_dte1", "OhmicPowerOutsideCard") => {
            "The ohmic power is below 0.173 MW or above 2.44 MW, the span of the nine compared pulses of the card's regime."
        }
        ("tokamak_jet_dte1", "PlasmaCurrentOutsideCard") => {
            "The plasma current is below 1.65 MA or above 3.84 MA, the span of the nine compared pulses of the card's regime."
        }
        ("tokamak_jet_dte1", "ToroidalFieldOutsideCard") => {
            "The toroidal field is below 1.80 T or above 4.04 T, the span of the nine compared pulses of the card's regime."
        }
        ("tokamak_jet_dte1", "FuelMixOutsideCard") => {
            "The tritium fraction of the fuel is below 0."
        }
        ("tokamak_jet_dte1", "FuelMixOutsideComparison") => {
            "The tritium fraction of the fuel is above 0.034 and below 0.451, where the fusion power was never compared, or above 0.504, where three training pulses at 0.75 to 0.93 showed it wrong."
        }
        ("tokamak_jet_dte1", "DensityBelowCard") => {
            "The line-averaged density is below 1.65e19 per cubic metre, the smallest compared."
        }
        ("tokamak_jet_dte1", "DensityAboveComparison") => {
            "The line-averaged density is above 7.85e19 per cubic metre, the largest compared."
        }
        ("tokamak_jet_dte1", "DensityAboveGreenwald") => {
            "The line-averaged density is above the Greenwald density of the plasma current."
        }
        ("tokamak_jet_dte1", "GreenwaldFractionAboveComparison") => {
            "The line-averaged density is above 0.834 of the Greenwald density of the plasma current, the largest fraction compared."
        }
        ("tokamak_jet_dte1", "SafetyFactorBelowLimit") => {
            "The edge safety factor of the current and the field is below 2."
        }
        ("tokamak_jet_dte1", "SafetyFactorOutsideComparison") => {
            "The edge safety factor of the current and the field is below 3.66 or above 4.14, the span of the nine compared pulses of the card's regime."
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
        ("tokamak_jet_dte1", "EnergyOutsideComparison") => {
            "The thermal energy is outside 0.462 to 9.57 MJ, or the fast-ion energy outside 5.59 kJ to 1.62 MJ, the spans of the nine compared runs of the card's regime, at the start of the step or where the trial step lands."
        }
        ("tokamak_jet_dte1", "TemperatureOutsideComparison") => {
            "The plasma temperature is below 0.777 keV or above 4.30 keV, the span of the nine compared runs of the card's regime, at the start of the step or where the trial step lands."
        }
        ("tokamak_jet_dte1", "BetaOutsideComparison") => {
            "The normalised beta of the two stores is below 0.224 or above 2.21, the span of the nine compared runs of the card's regime, at the start of the step or where the trial step lands."
        }
        ("tokamak_jet_dte1", "ConfinementTimeOutsideComparison") => {
            "The confinement time, the thermal energy over the loss power, is below 0.150 s or above 5.72 s, the span of the nine compared runs of the card's regime, at the start of the step or where the trial step lands."
        }
        ("tokamak_jet_dte1", "TimeBeyondCard") => {
            "The clock is negative, or the step would end after 5.475 s, the longest compared window, by more than the rounding allowance of the running sum of the clock: 4.44e-16 s for each step a run of this step takes to reach 5.475 s, and two more."
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

#[cfg(test)]
mod tests {
    use super::*;

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
                // The floor still moves the clock at the end of the card.
                assert!(reactor_triga_jsi::TIME_MAX_S + floor > reactor_triga_jsi::TIME_MAX_S);
                // The ceiling follows the fitted generation time: 0.094 ms.
                assert!((ceiling - 9.39365e-5).abs() < 1.0e-9);
            } else if card == "reactor_spert3_ccore" {
                assert_eq!(floor.to_bits(), reactor_spert3_ccore::FLOOR_DT_S.to_bits());
                assert!(reactor_spert3_ccore::TIME_MAX_S + floor > reactor_spert3_ccore::TIME_MAX_S);
                assert_eq!(ceiling.to_bits(), (0.2_f64 * 0.003).to_bits());
            } else if card == "reactor_spert3_fuel" {
                assert_eq!(floor.to_bits(), reactor_spert3_fuel::FLOOR_DT_S.to_bits());
                assert_eq!(floor.to_bits(), 1.0e-9_f64.to_bits());
                // The floor still moves the clock at the end of the card.
                assert!(reactor_spert3_fuel::TIME_MAX_S + floor > reactor_spert3_fuel::TIME_MAX_S);
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
                // The tyre's floor is its ceiling of 16 s divided by 2^14.
                assert_eq!(floor.to_bits(), (ceiling / 16_384.0).to_bits());
                assert_eq!(ceiling, 16.0);
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
                    "the ceiling divided by 2^14"
                } else if card == "fingerpad_p02_glass" {
                    "the ceiling divided by 2^8"
                } else if card == "cantilever_eth" {
                    "the ceiling divided by 2^18"
                } else if card == "los_jwst_fine_guide" {
                    "a locked 0.0635 s"
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
        assert_eq!(documented.len(), 13);
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
                field["maximum"].as_f64().unwrap().to_bits(),
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
        assert_eq!(bounds(minute), (0.0_f64.to_bits(), 60.0_f64.to_bits(), false, true));
        let held = &body["phase_fields"][3];
        assert_eq!(held["name"], "gauge_minute_rain_mm_h");
        assert_eq!(held["unit"], "mm/h");
        assert_eq!(bounds(held), (0.0_f64.to_bits(), 84.0_f64.to_bits(), false, false));
        assert!(held.to_string().contains("refused as RainNotHeldForGaugeMinute"), "{held}");
        assert!(minute.to_string().contains("a restart cannot change the rain inside a minute"), "{minute}");

        let dry = &body["phase_fields"][0];
        assert_eq!(dry["name"], "dry_path_loss_db");
        assert_eq!(dry["unit"], "dB");
        assert_eq!(bounds(dry), (50.0_f64.to_bits(), 54.0_f64.to_bits(), false, false));
        let water = &body["phase_fields"][1];
        assert_eq!(water["name"], "antenna_water_db");
        assert_eq!(water["unit"], "dB");
        assert_eq!(bounds(water), (0.0_f64.to_bits(), 2.13_f64.to_bits(), false, false));
        let rain = &body["load_fields"][0];
        assert_eq!(rain["name"], "rain_rate_mm_h");
        assert_eq!(rain["unit"], "mm/h");
        assert_eq!(bounds(rain), (0.0_f64.to_bits(), 84.0_f64.to_bits(), false, false));
        assert!(rain.to_string().contains("refused as RainRateBelowGaugeCount"), "{rain}");

        let next = |value: f64| f64::from_bits(value.to_bits() + 1);
        let below = |value: f64| f64::from_bits(value.to_bits() - 1);
        let start = link_gothenburg_10010::LinkState::try_new;
        assert!(start(&link, 50.0, 0.0).is_some());
        assert!(start(&link, 54.0, 2.13).is_some());
        assert!(start(&link, next(54.0), 0.0).is_none());
        assert!(start(&link, f64::from_bits(50.0_f64.to_bits() - 1), 0.0).is_none());
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
            Err(link_gothenburg_10010::StepReject::RainRateOutsideCard)
        );
        assert_eq!(
            link_gothenburg_10010::step(&link, &mut state, &mut frame, load(84.0), 60.0),
            Ok(())
        );
        assert_eq!(
            link_gothenburg_10010::step(&link, &mut state, &mut frame, load(below(6.0)), 60.0),
            Err(link_gothenburg_10010::StepReject::RainRateBelowGaugeCount)
        );
        assert_eq!(
            link_gothenburg_10010::step(&link, &mut state, &mut frame, load(6.0), 60.0),
            Ok(())
        );

        let dt = &body["dt_s"];
        assert_eq!(dt["ceiling_s"].as_f64().unwrap(), 60.0);
        assert_eq!(dt["floor_s"].as_f64().unwrap().to_bits(), (60.0_f64 / 1_048_576.0).to_bits());
        assert!(dt["nyquist_s"].is_null());
        assert!(dt["rule"].as_str().unwrap().contains("the gauge's minute"));
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
            serde_json::to_string(&body["example_verify_step"]).unwrap(),
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
            "a rate above 0 and below 6 mm/h and a rain changed inside a gauge minute are refused by name",
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
            "evidence/checks/second_party/V-28/",
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
        assert_eq!(documented.len(), 16);
        assert_eq!(card.rejects.len(), documented.len());
        for ((name, sentence), listed) in documented.iter().zip(card.rejects) {
            assert_eq!(name, listed);
            assert_eq!(
                reject_meaning("los_jwst_fine_guide", name),
                Some(sentence.as_str()),
                "{name}"
            );
        }
        assert_eq!(card.advisory.len(), 1);
        assert_eq!(card.advisory[0].mask, los_jwst_fine_guide::FLAG_NO_MEASUREMENT);
        let field = |name: &str| card.fields.iter().find(|f| f.name == name).expect("field");
        for (lock, minimum, maximum) in [
            (
                "lock_x_arcsec",
                los_jwst_fine_guide::LOCK_X_MIN_ARCSEC,
                los_jwst_fine_guide::LOCK_X_MAX_ARCSEC,
            ),
            (
                "lock_y_arcsec",
                los_jwst_fine_guide::LOCK_Y_MIN_ARCSEC,
                los_jwst_fine_guide::LOCK_Y_MAX_ARCSEC,
            ),
        ] {
            assert_eq!(field(lock).minimum, Some(minimum));
            assert_eq!(field(lock).maximum, Some(maximum));
            assert!(field(lock).bound.contains("LockOutsideCard"));
            assert!(field(lock).bound.contains(&token(minimum)));
            assert!(field(lock).bound.contains(&token(maximum)));
        }
        assert!(field("measured").bound.contains("MeasurementGapOutsideCard"));
        assert!(card.nyquist_rule.contains("MeasurementGapOutsideCard"));
        assert!(!card.nyquist_rule.contains("no limit in time or in frames"));
        let quiet = field("quiet_before_start_s");
        assert_eq!(
            quiet.minimum,
            Some(los_jwst_fine_guide::RECOVERY_S - los_jwst_fine_guide::RECOVERY_ALLOWANCE_S)
        );
        assert_eq!(quiet.maximum, None);
        assert!(quiet.bound.contains("RecoveryNotElapsed") && quiet.bound.contains("it cannot check it"));
        assert_eq!(field("measured").minimum, Some(0.0));
        assert_eq!(field("measured").maximum, Some(1.0));
        assert_eq!(field("counts_per_s").minimum, Some(0.0));
        assert_eq!(field("counts_per_s").maximum, Some(los_jwst_fine_guide::COUNTS_MAX_PER_S));
        assert!(field("counts_per_s").bound.contains("CountRateOutsideCard"));
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
        let step = &body["example_verify_step"];
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
        assert_eq!(documented.len(), 15);
        assert_eq!(card.rejects.len(), documented.len());
        for ((name, sentence), listed) in documented.iter().zip(card.rejects) {
            assert_eq!(name, listed);
            assert_eq!(reject_meaning("grid_gb", name), Some(sentence.as_str()), "{name}");
        }
        assert!(card.advisory.is_empty());
        let body = describe_one(card);
        let expected = [
            ("frequency_hz", grid_gb::START_FREQUENCY_MIN_HZ, grid_gb::START_FREQUENCY_MAX_HZ),
            ("inertia_mw_s", grid_gb::INERTIA_MIN_MW_S, grid_gb::INERTIA_MAX_MW_S),
            ("demand_mw", grid_gb::DEMAND_MIN_MW, grid_gb::DEMAND_MAX_MW),
        ];
        for (index, (name, low, high)) in expected.iter().enumerate() {
            let field = &body["phase_fields"][index];
            assert_eq!(field["name"], *name);
            assert_eq!(field["minimum"].as_f64().unwrap().to_bits(), low.to_bits());
            assert_eq!(field["maximum"].as_f64().unwrap().to_bits(), high.to_bits());
        }
        let load = &body["load_fields"][0];
        assert_eq!(load["name"], "loss_mw");
        assert_eq!(load["minimum"].as_f64().unwrap().to_bits(), 693.0_f64.to_bits());
        assert_eq!(load["minimum"].as_f64().unwrap().to_bits(), grid_gb::LOSS_MIN_MW.to_bits());
        assert_eq!(load["maximum"].as_f64().unwrap().to_bits(), 1402.0_f64.to_bits());
        let ceiling = grid_gb::GridGb::interconnector_loss().gate_dt_s().unwrap();
        assert_eq!(
            body["example_verify_step"]["parameters"]["dt_s"].as_f64().unwrap().to_bits(),
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
        assert_eq!(documented.len(), 15);
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

    /// The reactor publishes the kernel's bounds exactly, and its examples
    /// are one step at the ceiling and a run to the first maximum.
    #[test]
    fn the_reactor_publishes_the_kernel_bounds_and_its_examples_start_cold() {
        let card = find("energy", "reactor_spert3").expect("the reactor is served");
        let body = describe_one(card);
        let phase = &body["phase_fields"][0];
        assert_eq!(phase["name"], "coolant_temperature_k");
        assert_eq!(phase["unit"], "K");
        assert_eq!(
            phase["minimum"].as_f64().unwrap().to_bits(),
            reactor_spert3::COOLANT_TEMPERATURE_MIN_K.to_bits()
        );
        assert_eq!(
            phase["maximum"].as_f64().unwrap().to_bits(),
            reactor_spert3::COOLANT_TEMPERATURE_MAX_K.to_bits()
        );
        let load = &body["load_fields"][0];
        assert_eq!(load["name"], "reactivity_inserted_dollar");
        assert_eq!(load["minimum"].as_f64().unwrap().to_bits(), 0.7686211839115998_f64.to_bits());
        assert_eq!(load["maximum"].as_f64().unwrap().to_bits(), 1.2102315121058187_f64.to_bits());
        let ceiling = reactor_spert3::ReactorSpert3::e_core_cold_startup()
            .gate_dt_s()
            .unwrap();
        assert_eq!(
            body["example_verify_step"]["parameters"]["dt_s"].as_f64().unwrap().to_bits(),
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
        assert_eq!(documented.len(), 17);
        assert_eq!(card.rejects.len(), documented.len());
        for ((name, sentence), listed) in documented.iter().zip(card.rejects) {
            assert_eq!(name, listed);
            assert_eq!(
                reject_meaning("reactor_spert3_fuel", name),
                Some(sentence.as_str()),
                "{name}"
            );
        }
        // The three numbers the sentences carry are the card's own limits.
        for (name, limit) in [
            ("TimeBeyondCard", reactor_spert3_fuel::TIME_MAX_S),
            ("EnergyBeyondCard", reactor_spert3_fuel::ENERGY_MAX_MWS),
            ("PowerBeyondCard", reactor_spert3_fuel::POWER_MAX_MW),
        ] {
            let sentence = reject_meaning("reactor_spert3_fuel", name).unwrap();
            assert!(sentence.contains(&format!(" {limit} ")), "{name}: {sentence}");
        }
    }

    /// The JSI TRIGA reactor lists every variant of its `StepReject`, in the
    /// kernel's order, with the kernel's sentence word for word.
    #[test]
    fn the_triga_reactor_refusals_carry_the_sentences_from_the_card() {
        let documented = documented_rejects(REACTOR_TRIGA_SOURCE);
        let card = find("energy", "reactor_triga_jsi").expect("the TRIGA reactor is served");
        assert_eq!(documented.len(), 22);
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
            ("TimeBeyondCard", reactor_triga_jsi::TIME_MAX_S),
            ("EnergyBeyondCard", reactor_triga_jsi::ENERGY_MAX_MWS),
            ("PowerBeyondCard", reactor_triga_jsi::POWER_MAX_MW),
            ("FuelTemperatureBeyondCard", reactor_triga_jsi::FUEL_TEMPERATURE_MAX_K),
        ] {
            let sentence = reject_meaning("reactor_triga_jsi", name).unwrap();
            assert!(sentence.contains(&format!(" {limit} ")), "{name}: {sentence}");
        }
        // and the envelopes compared with the records, to the hundredth
        for (name, limits) in [
            ("ScramBeforeCompared", vec![reactor_triga_jsi::SCRAM_AFTER_PEAK_MIN_S]),
            ("RodsHeldBeyondCompared", vec![reactor_triga_jsi::SCRAM_AFTER_PEAK_MAX_S]),
            (
                "StartTemperatureOutsideCard",
                reactor_triga_jsi::START_TEMPERATURE_K_BY_TABLE
                    .iter()
                    .flat_map(|(low, high)| [low - 273.15, high - 273.15])
                    .collect(),
            ),
        ] {
            let sentence = reject_meaning("reactor_triga_jsi", name).unwrap();
            for limit in limits {
                assert!(sentence.contains(&format!(" {limit:.2} ")), "{name}: {sentence}");
            }
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
        assert!(cell.specimen.ends_with("it takes only the 1C currents compared with measurement, 2.89817 to 2.90065 A."));
    }

    #[test]
    fn the_triga_reactor_publishes_the_kernel_bounds_tables_flags_frame_and_record() {
        let card = find("energy", "reactor_triga_jsi").expect("the TRIGA reactor is served");
        assert_eq!(all()[29].card, "reactor_triga_jsi");
        assert_eq!(all().last().map(|last| last.card), Some("reactor_spert3_ccore"));
        assert_eq!(all().len(), 31);
        for words in [
            "Ten constants are fitted to 36 pulses of this reactor",
            "added after the twelve fixed beforehand had failed",
            "one thermocouple of one element",
            "2.0 to 3.0 printed dollars",
            "below 2.0 dollars the card refuses by name",
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
        assert_eq!(bits(&phase[0]["minimum"]), reactor_triga_jsi::START_TEMPERATURE_MIN_K.to_bits());
        assert_eq!(bits(&phase[0]["maximum"]), reactor_triga_jsi::START_TEMPERATURE_MAX_K.to_bits());
        let start_words = phase[0]["bound"].as_str().unwrap();
        for (core, (low, high)) in reactor_triga_jsi::ROD_TABLE_CORE.iter().zip(reactor_triga_jsi::START_TEMPERATURE_K_BY_TABLE) {
            assert!(start_words.contains(&format!("{low:.2} to {high:.2} K")), "{start_words}");
            assert!(start_words.contains(&format!("on core {core}")), "{start_words}");
        }
        assert!(start_words.contains("StartTemperatureOutsideCard"), "{start_words}");
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
        assert_eq!(bits(&load[0]["minimum"]), reactor_triga_jsi::REACTIVITY_MIN_DOLLAR.to_bits());
        assert_eq!(bits(&load[0]["maximum"]), reactor_triga_jsi::REACTIVITY_MAX_DOLLAR.to_bits());
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
        assert!(
            rule.contains(&format!(
                "would pass {} MW s, {} MW or a core-average fuel temperature of {} K",
                reactor.energy_max_mws, reactor.power_max_mw, reactor.fuel_temperature_max_k
            )),
            "{rule}"
        );
        assert!(rule.contains("there is no thermocouple on the card"));
        assert!(rule.contains(&format!("a run whose steps make exactly {} s keeps its last step", reactor.time_max_s)));
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
            "evidence/checks/second_party/V-37",
            "9a0720f1822161c3b53cbc351d862ec4c077da51521ee1a96d7ceb85d8275ddc",
            "by the rule fixed beforehand none held in any bin of reactivity",
            "the comparison failed: two of the six lines were met and four missed",
            "are also met by the nearest opened pulse taken unchanged, so they are not evidence for the model",
            "the first card met the thermocouple maximum",
            "The card cannot see the day",
            "evidence/checks/second_party/V-43",
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
        assert_eq!(all().len(), 31);
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
        let least = |values: [f64; 3]| values.into_iter().fold(f64::INFINITY, f64::min);
        let most = |values: [f64; 3]| values.into_iter().fold(f64::NEG_INFINITY, f64::max);
        assert_eq!(bits(&load[0]["minimum"]), least(reactor_spert3_fuel::BAND_REACTIVITY_MIN_DOLLAR).to_bits());
        assert_eq!(bits(&load[0]["maximum"]), most(reactor_spert3_fuel::BAND_REACTIVITY_MAX_DOLLAR).to_bits());
        assert_eq!(bits(&load[1]["minimum"]), least(reactor_spert3_fuel::BAND_FLOW_MIN_M_PER_S).to_bits());
        assert_eq!(bits(&load[1]["maximum"]), most(reactor_spert3_fuel::BAND_FLOW_MAX_M_PER_S).to_bits());
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
            // The band's name is the band's own ends in degrees Fahrenheit; the
            // highest is rounded outward, so either conversion of it is inside.
            let (low, high) = name.split_once(" to ").unwrap();
            let kelvin = |degf: &str| (degf.parse::<f64>().unwrap() - 32.0) / 1.8 + 273.15;
            let rankine = |degf: &str| (degf.parse::<f64>().unwrap() + 459.67) / 1.8;
            assert_eq!(kelvin(low).to_bits(), reactor.band_inlet_min_k[band].to_bits());
            assert_eq!(rankine(high).to_bits(), reactor.band_inlet_max_k[band].to_bits());
            assert!(kelvin(high) <= reactor.band_inlet_max_k[band]);
        }
        for name in ["InletTemperatureOutsideCard", "ReactivityOutsideCard", "FlowOutsideCard"] {
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
        assert_eq!(bits(&body["example_verify_step"]["parameters"]["dt_s"]), ceiling.to_bits());
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
        // The rule names the card's own starting power and its three ends.
        let rule = dt["rule"].as_str().unwrap();
        assert!(rule.contains(&format!("starts at {:e} MW in equilibrium", reactor.initial_power_mw)), "{rule}");
        assert!(
            rule.contains(&format!(
                "would pass {} s, {} MW s or {} MW",
                reactor.time_max_s, reactor.energy_max_mws, reactor.power_max_mw
            )),
            "{rule}"
        );
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
            "the whole 260 degree Fahrenheit series of 13",
        ] {
            assert!(data.contains(words), "{words}: {data}");
        }
        let pass_line = part("pass_line");
        for words in [
            "evidence/research/reactor_spert3/validation_protocol_v2.md",
            "413ace16d5cc3d622c0ef44c0ea2b9526ac7526dfeee4b43ae601d6018e51db4",
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
            // 260 degF
            "the one group with nothing fitted to it",
            "(0.643, 1.051, and a mean of +0.479 against a line of 0.5 in size, met by 0.02)",
            "(0.846, 1.334, -0.152)",
            "root mean square 8.46 percent and worst 14.88 percent, twelve of the thirteen peaks early",
            // 500 degF
            "(0.796, 1.650, and a mean of -0.490 against 0.5, met by 0.01)",
            "(4.49 and 7.45 percent)",
            "root mean square 1.336, worst 3.212 against 3, and mean -0.890 against 0.5 in size",
            "the card's energy is low on all seven tests",
            // the 30 together
            "(0.644, 1.650, +0.161)",
            "(0.908, mean -0.266, worst 3.212)",
            "(6.69 and 14.88 percent)",
            "The scored step was 0.43 ms, the card's ceiling, where the protocol said 0.5 ms",
            "evidence/checks/second_party/V-25/",
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
            "the whole of Table V, its held-back rows included, had been printed",
            "are not unseen",
            "The conservation residual names a wrong method and is not a bound on accuracy",
            "reads the true error of the state low by about 11 to 15 times",
            "3.3e-7 and 3.8e-7 of the neutron population, is above the limit while the residual is 2.3e-8 and 2.6e-8",
            "a load changed during a burst is refused by name",
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
        assert_eq!(documented.len(), 16);
        assert_eq!(card.rejects.len(), documented.len());
        for ((name, sentence), listed) in documented.iter().zip(card.rejects) {
            assert_eq!(name, listed);
            assert_eq!(
                reject_meaning("cell_2170_m80", name),
                Some(sentence.as_str()),
                "{name}"
            );
        }
        assert!(card.advisory.is_empty());
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
        // The coldest served block is the 0 degC block: the -10 and -20 degC
        // blocks were shown wrong and are refused.
        assert_eq!(bounds(&block), (0.0_f64.to_bits(), 40.0_f64.to_bits(), false, false));
        assert!(block["bound"].as_str().unwrap().contains("BlockShownWrong"));
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
        ] {
            let chamber = token(chamber.parse::<f64>().unwrap());
            let deep = token(deep.parse::<f64>().unwrap());
            assert!(
                words.contains(&format!("{chamber} degC: [{}, {deep}] Ah", token(-0.02))),
                "{chamber}: {words}"
            );
        }
        for (chamber, deep) in [("-10", "3.9413"), ("-20", "3.7886")] {
            let chamber = token(chamber.parse::<f64>().unwrap());
            let deep = token(deep.parse::<f64>().unwrap());
            assert!(!words.contains(&format!("{chamber} degC: [{}, {deep}] Ah", token(-0.02))), "{chamber}: {words}");
        }
        // The current and the case temperature are the ranges compared with measurement.
        let current = field("load_fields", 0);
        assert_eq!(current["name"], "current_a");
        assert_eq!(bounds(&current), ((-11.375_f64).to_bits(), 9.737_f64.to_bits(), false, false));
        let case = field("load_fields", 1);
        assert_eq!(case["name"], "case_temperature_c");
        assert_eq!(case["unit"], "degC");
        assert_eq!(bounds(&case), (0.808_f64.to_bits(), 45.535_f64.to_bits(), false, false));
        let bands = case["bound"].as_str().unwrap();
        for block in 0..cell_2170_m80::BLOCK_COUNT {
            let band = format!(
                "{} degC block: [{}, {}] degC",
                cell_2170_m80::BLOCK_CHAMBER_C[block],
                cell_2170_m80::BLOCK_CASE_MIN_C[block],
                cell_2170_m80::BLOCK_CASE_MAX_C[block]
            );
            assert_eq!(bands.contains(&band), !cell_2170_m80::BLOCK_SHOWN_WRONG[block], "{band}: {bands}");
        }
        // Each served block's currents are published, from envelope.json of the 0.7.0 fix.
        let currents = current["bound"].as_str().unwrap();
        assert!(currents.contains("CurrentOutsideBlock"));
        for block in 0..cell_2170_m80::BLOCK_COUNT {
            let band = format!(
                "{} degC block: [{}, {}] A",
                cell_2170_m80::BLOCK_CHAMBER_C[block],
                -cell_2170_m80::BLOCK_CURRENT_DISCHARGE_MAX_A[block],
                cell_2170_m80::BLOCK_CURRENT_CHARGE_MAX_A[block]
            );
            assert_eq!(currents.contains(&band), !cell_2170_m80::BLOCK_SHOWN_WRONG[block], "{band}: {currents}");
        }
        assert!(currents.contains("40 degC block: [-9.809, 9.737] A"), "{currents}");
        assert!(currents.contains("0 degC block: [-11.375, 8.304] A"), "{currents}");
        // A start within the rounding allowance of a start past an end is stated.
        assert!(charge["bound"].as_str().unwrap().contains("2^-49 Ah"));
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

        assert_eq!(body["advisory_flags"].as_array().unwrap().len(), 0);
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

        let step = &body["example_verify_step"]["parameters"];
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
        assert_eq!(documented.len(), 11);
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
        assert_eq!(ends("normal_force_n"), (1.93, 2.07));
        assert_eq!(ends("twist_rate_deg_s"), (5.0, 50.0));
        assert!(field("normal_force_n").bound.contains("the spread of the robot's force controller"));
        assert!(card.specimen.contains("every other load are now refused by name"));
        assert!(card.specimen.contains("which is not a test"));
        assert_eq!(
            ends("normal_force_n"),
            (
                fingerpad_p02_glass::NORMAL_FORCE_FLOOR_N,
                fingerpad_p02_glass::NORMAL_FORCE_CEILING_N
            )
        );
        assert_eq!(
            ends("twist_rate_deg_s"),
            (
                fingerpad_p02_glass::TWIST_RATE_FLOOR_DEG_S,
                fingerpad_p02_glass::TWIST_RATE_CEILING_DEG_S
            )
        );
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
        // The one flag left is the event of full slip, and it is named as an
        // event; the two warnings of the scored build are gone.
        let flags = described["advisory_flags"].as_array().unwrap();
        assert_eq!(flags.len(), 1);
        assert_eq!(flags[0]["name"], "FLAG_FULL_SLIP");
        assert_eq!(flags[0]["kind"], "event");
        assert_eq!(flags[0]["state"], "full_slip");
        let record = &described["validation"];
        assert!(record["data"].as_str().unwrap().contains("10.5281/zenodo.7646119"));
        assert!(record["pass_line"].as_str().unwrap().contains(
            "49a4788d1eb6e5d5881d4f3b2195ad248a2a543c2da45fb1c8ffb7dc27672081"
        ));
        assert_eq!(record["result"], FINGERPAD_P02_GLASS_RESULT);
        let limits = record["limits"].as_str().unwrap();
        for words in [
            "nothing inside it is a test",
            "block to block, 13 to 15 percent",
            "the experiments were conducted on separate days",
            "The twist at full slip is the card's weakest output",
            "closes for any value of the eight constants",
            "TwistNotStartedFromZero",
            "where the largest work inside the range is 0.0126 J",
            "The twist was compared from 1 to 75 degrees, and the card answers from zero to 75",
        ] {
            assert!(limits.contains(words), "{words}");
        }
        assert!(!limits.contains("0.015 J"));
        assert!(field("twist_deg").bound.contains("would end beyond 75 degrees is refused as TwistBeyondMeasured"));
        assert!(card.nyquist_rule.contains("a run that ends exactly on 75 degrees keeps its last step"));
        assert_eq!(fingerpad_p02_glass::TWIST_CEILING_DEG, 75.0);
        // No other card's flag is called an event.
        for other in all().iter().filter(|other| other.card != "fingerpad_p02_glass") {
            let body = describe_one(other);
            for flag in body["advisory_flags"].as_array().unwrap() {
                assert!(flag.get("kind").is_none(), "{}", other.card);
            }
        }
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
        assert_eq!(ends("distance_m"), (0.0, tire_eve_r19_ice::TRACK_LENGTH_M));
        // The hold the comparison covers: 5 s to 21 s after a slip is set.
        assert_eq!(
            ends("slip_held_s"),
            (tire_eve_r19_ice::SLIP_SETTLE_S, tire_eve_r19_ice::HOLD_CEILING_S)
        );
        assert!(field("slip_held_s").bound.contains("refused as SlipNotSettled"));
        assert!(field("slip_held_s").bound.contains("HoldBeyondMeasured"));
        // No speed was measured: the bound says the speed is inferred.
        assert!(field("carriage_speed_m_s").bound.contains("The files record no carriage speed"));
        assert!(!field("carriage_speed_m_s").bound.contains("no other speed was measured"));
        assert_eq!(
            ends("normal_load_n"),
            (tire_eve_r19_ice::NORMAL_LOAD_FLOOR_N, tire_eve_r19_ice::NORMAL_LOAD_CEILING_N)
        );
        assert_eq!(
            ends("inflation_pressure_pa"),
            (
                tire_eve_r19_ice::INFLATION_PRESSURE_FLOOR_PA,
                tire_eve_r19_ice::INFLATION_PRESSURE_CEILING_PA
            )
        );
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
        assert!(card.nyquist_rule.contains("from 5 s to 21 s after a slip is set"));
        assert!(card.nyquist_rule.contains("5.5e-11 m at every step"));
    }

    const ACCUMULATOR_SOURCE: &str = include_str!("../../genesis_kernel/src/accumulator_sbo450.rs");

    /// The accumulator lists every variant of its `StepReject`, in the
    /// kernel's order, with the kernel's sentence word for word.
    #[test]
    fn the_accumulator_refusals_carry_the_sentences_from_the_card() {
        let documented = documented_rejects(ACCUMULATOR_SOURCE);
        let card = find("energy", "accumulator_sbo450").expect("the accumulator is served");
        assert_eq!(documented.len(), 16);
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
        let ends = |name: &str| (field(name).minimum.unwrap(), field(name).maximum.unwrap());
        assert_eq!(
            ends("gas_mass_kg"),
            (
                accumulator_sbo450::GAS_MASS_20_BAR_FLOOR_KG,
                accumulator_sbo450::GAS_MASS_40_BAR_CEILING_KG
            )
        );
        assert_eq!(
            ends("gas_pressure_pa"),
            (accumulator_sbo450::PRESSURE_FLOOR_PA, accumulator_sbo450::PRESSURE_CEILING_PA)
        );
        assert_eq!(
            ends("gas_temperature_k"),
            (
                accumulator_sbo450::GAS_TEMPERATURE_FLOOR_K,
                accumulator_sbo450::GAS_TEMPERATURE_CEILING_K
            )
        );
        assert_eq!(
            ends("wall_temperature_k"),
            (
                accumulator_sbo450::WALL_TEMPERATURE_FLOOR_K,
                accumulator_sbo450::WALL_TEMPERATURE_CEILING_K
            )
        );
        assert_eq!(
            ends("piston_speed_m_s"),
            (
                -accumulator_sbo450::PISTON_SPEED_CEILING_M_S,
                accumulator_sbo450::PISTON_SPEED_CEILING_M_S
            )
        );
        assert!(field("gas_pressure_pa").bound.contains("refused as PressureOutsideChargeBands"));
        assert!(field("gas_pressure_pa").bound.contains("3700000 to 4300000 Pa or 5500000 to 6700000 Pa for the 20 bar charge, 5700000 to 6400000 Pa or 7500000 to 8600000 Pa for the 40 bar charge"));
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
        // The wall, the gas temperature and the speed are held to what the compared runs reached.
        assert_eq!(ends("wall_temperature_k"), (293.4, 293.7));
        assert_eq!(ends("gas_temperature_k"), (285.9, 301.8));
        assert_eq!(ends("piston_speed_m_s"), (-0.7, 0.7));
        assert!(field("wall_temperature_k").bound.contains("293.46 to 293.60 K"));
        assert!(field("wall_temperature_k").bound.contains("refused as WallTemperatureOutsideCard"));
        assert!(field("gas_temperature_k").bound.contains("285.97 to 301.78 K"));
        assert!(field("piston_speed_m_s").bound.contains("refused as PistonSpeedOutsideCard"));
        assert!(ACCUMULATOR_SBO450_LIMITS.contains("(WallTemperatureOutsideCard: the wall of every compared run was 293.46 to 293.60 K)"));
        // The conservation gate depends on the ledger's anchor; no limit of the range does.
        assert!(card.nyquist_rule.contains("the ledger's conservation check is held from each start, and no limit of the card's range depends on them"));
        assert!(!card.nyquist_rule.contains("begin again at each start, and no limit depends on them"));
        assert!(field("gas_mass_kg").bound.contains("0.013639 to 0.013667 kg"));
        assert!(field("gas_mass_kg").bound.contains("0.02744 to 0.027495 kg"));
        assert!(field("gas_mass_kg").bound.contains("refused as GasMassOutsideCard"));
        assert_eq!(accumulator_sbo450::PISTON_BORE_M, 0.05);
        assert!(card.nyquist_rule.contains("19.635 cm2"));
        assert_eq!(accumulator_sbo450::HEAT_SHARE_LIMIT, 0.065);
        assert_eq!(accumulator_sbo450::HEAT_SHARE_WORK_FLOOR_J, 100.0);
        assert!(card.nyquist_rule.contains("at most 0.065 of the larger of M and 100 J"));
        assert!(card.nyquist_rule.contains("refused as HeatShareAboveCard"));
        assert!(card.advisory.is_empty());
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
        assert_eq!(documented.len(), 27);
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

    /// The tokamak publishes the kernel's bounds exactly: each one the
    /// envelope of the nine compared runs of its regime, without 42976, the
    /// clock to the longest compared
    /// window, and the tritium fraction to the upper of its two spans. Its
    /// validation record carries the hash of the pre-registration and names
    /// both held-back pulses, and its summary states the failure.
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
        for (name, minimum, maximum) in [
            ("thermal_energy_j", 0.462e6_f64, 9.57e6_f64),
            ("fast_ion_energy_j", 5.59e3, 1.62e6),
            ("elapsed_time_s", 0.0, 5.475),
            ("heating_power_w", 0.124e6, 24.3e6),
            ("ohmic_power_w", 0.173e6, 2.44e6),
            ("line_density_per_m3", 1.65e19, 7.85e19),
            ("plasma_current_a", 1.65e6, 3.84e6),
            ("toroidal_field_t", 1.80, 4.04),
            ("tritium_fraction", 0.0, 0.504),
        ] {
            let published = field(name);
            assert_eq!(published["minimum"].as_f64().unwrap().to_bits(), minimum.to_bits(), "{name}");
            assert_eq!(published["maximum"].as_f64().unwrap().to_bits(), maximum.to_bits(), "{name}");
            assert_eq!(published["minimum_exclusive"], false, "{name}");
            assert_eq!(published["maximum_exclusive"], false, "{name}");
        }
        assert!(field("tritium_fraction")["bound"].as_str().unwrap().contains("FuelMixOutsideComparison"));
        assert!(field("elapsed_time_s")["bound"].as_str().unwrap().contains("It is required"));
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
        // The pulse outside the card's regime is outside its envelope, and the limits say why.
        assert!(limits.contains("The states and loads only pulse 42976 reached are outside the declared range and refused by name"));
        assert!(limits.contains("declared outside the card's regime before it was run and it missed all four marks"));
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
        assert_eq!(documented.len(), 8);
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
    /// envelope is the condition they share, with its floor. The step ceiling
    /// and floor are the card's, no load field and no flag is published, the
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
        // 0.01579 m times 2 pi times 8.29235 Hz, to the last few 64-bit numbers.
        let product = ceiling_m * beam.omega0_rad_s();
        assert!((largest_m_s - product).abs() <= 4.0 * f64::EPSILON * product, "{largest_m_s}");
        assert!(largest_m_s > 0.8226 && largest_m_s < 0.8228, "{largest_m_s}");
        for sign in [1.0, -1.0] {
            assert!(cantilever_eth::CantileverState::try_phase(&beam, 0.0, sign * largest_m_s).is_some());
            let beyond = sign * f64::from_bits(largest_m_s.to_bits() + 1);
            assert!(cantilever_eth::CantileverState::try_phase(&beam, 0.0, beyond).is_none());
        }
        for field in [deflection, speed] {
            let words = field["bound"].as_str().unwrap();
            assert!(words.contains("sqrt(deflection_m^2 + (velocity_m_s / omega0)^2)"), "{words}");
            assert!(words.contains("at or below 0.01579 m and at or above 5.8e-5 m"), "{words}");
            assert!(words.contains("DeflectionAboveCard"), "{words}");
            assert!(words.contains("DeflectionBelowCard"), "{words}");
        }
        // The floor of the envelope: rest at zero and anything under 5.8e-5 m
        // makes no start; the floor itself does.
        let floor_m = cantilever_eth::AMPLITUDE_FLOOR_M;
        assert_eq!(floor_m, 5.8e-5);
        assert!(cantilever_eth::CantileverState::try_phase(&beam, floor_m, 0.0).is_some());
        let under = f64::from_bits(floor_m.to_bits() - 1);
        assert!(cantilever_eth::CantileverState::try_phase(&beam, under, 0.0).is_none());
        assert!(cantilever_eth::CantileverState::try_phase(&beam, 0.0, 0.0).is_none());

        let dt = &body["dt_s"];
        assert_eq!(dt["ceiling_s"].as_f64().unwrap().to_bits(), cantilever_eth::GATE_DT_S.to_bits());
        assert_eq!(dt["ceiling_s"].as_f64().unwrap(), 2.5e-4);
        assert_eq!(dt["floor_s"].as_f64().unwrap().to_bits(), (2.5e-4_f64 / 262_144.0).to_bits());
        assert!(dt["nyquist_s"].is_null());
        assert!(dt["rule"].as_str().unwrap().contains("No load: the beam rings free"));

        assert!(body["advisory_flags"].as_array().unwrap().is_empty());
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
        assert!(slot("flags")["holds"].as_str().unwrap().starts_with("Always zero."));

        assert_eq!(
            serde_json::to_string(&body["example_verify_step"]).unwrap(),
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
            "The envelope range is what the comparison with the held-back ring-down spanned.",
            "from 15.786 mm at its start, the largest it held, through 14.404 mm, at the first turning point after the start crossing, to 0.0587 mm",
            "the ceiling keeps the compared run's own start, and that run lands 94,731 steps before it is refused as DeflectionBelowCard",
            "lands 94,136 steps, to 23.534 s, before it is refused as DeflectionBelowCard",
            "rest at zero is refused",
        ] {
            assert!(rule.contains(words), "{words}: {rule}");
        }
        assert!(!rule.contains("FLAG_"), "{rule}");
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
            "next 12 s were kept unread",
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
            "cross-checked to 3 percent",
            "every compared quantity is a ratio or a frequency",
            "no mass, so its energies are per kilogram",
            "a fitted power law",
            "refused after 104 steps of 0.25 ms",
            "from 0.058 to 15.79 mm, and refuses outside them by name",
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
    /// flag.
    #[test]
    fn the_physical_2170_cell_refusals_carry_the_sentences_from_the_card() {
        let documented = documented_rejects(CELL_2170_MODEL3_SOURCE);
        let card = find("energy", "cell_2170_model3").expect("the physical 2170 cell is served");
        assert_eq!(documented.len(), 18);
        assert_eq!(card.rejects.len(), documented.len());
        for ((name, sentence), listed) in documented.iter().zip(card.rejects) {
            assert_eq!(name, listed);
            assert_eq!(
                reject_meaning("cell_2170_model3", name),
                Some(sentence.as_str()),
                "{name}"
            );
        }
        assert_eq!(card.advisory.len(), 1);
        assert_eq!(card.advisory[0].name, "FLAG_SUSTAINED_ABOVE_1C");
        assert_eq!(card.advisory[0].mask, cell_2170_model3::FLAG_SUSTAINED_ABOVE_1C);
        assert!(card.advisory[0].seals);
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
                field["minimum"].as_f64().unwrap().to_bits(),
                field["maximum"].as_f64().unwrap().to_bits(),
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
        assert_eq!(bounds(&charge), (0.0_f64.to_bits(), 1.02_f64.to_bits(), false, false));
        let capacity = field("phase_fields", 2);
        assert_eq!(capacity["name"], "capacity_ah");
        assert_eq!(capacity["unit"], "Ah");
        assert_eq!(bounds(&capacity), (4.16_f64.to_bits(), 4.70_f64.to_bits(), false, false));
        let words = capacity["bound"].as_str().unwrap();
        assert!(words.contains("capacity_ah = ratio x (ampere-hours put in by the full recharge) / (1 - state of charge before that recharge)"));
        assert!(words.contains("it is listed for each cell under cell_number"));
        assert!(words.contains("below 0.08"));
        let current = field("load_fields", 0);
        assert_eq!(current["name"], "current_a");
        assert_eq!(bounds(&current), ((-13.984_f64).to_bits(), 13.524_f64.to_bits(), false, false));
        assert_eq!(13.524_f64.to_bits(), (13.5_f64 + cell_2170_model3::TESTER_CURRENT_ACCURACY_A).to_bits());
        assert!(current["bound"].as_str().unwrap().contains("A positive current discharges the cell"));
        let case = field("load_fields", 1);
        assert_eq!(case["name"], "case_temperature_c");
        assert_eq!(case["unit"], "degC");
        assert_eq!(bounds(&case), ((-12.9_f64).to_bits(), 45.9_f64.to_bits(), false, false));
        assert_eq!(body["phase_fields"].as_array().unwrap().len(), 5);
        assert_eq!(body["load_fields"].as_array().unwrap().len(), 2);
        for (index, name) in [(3, "case_temperature_least_c"), (4, "case_temperature_most_c")] {
            let carried = field("phase_fields", index);
            assert_eq!(carried["name"], name);
            assert_eq!(carried["unit"], "degC");
            assert_eq!(bounds(&carried), ((-12.9_f64).to_bits(), 45.9_f64.to_bits(), false, false));
            assert!(carried["bound"].as_str().unwrap().contains("A start without it is refused"));
        }

        let dt = &body["dt_s"];
        assert_eq!(dt["floor_s"].as_f64().unwrap().to_bits(), 1.0_f64.to_bits());
        assert_eq!(dt["ceiling_s"].as_f64().unwrap().to_bits(), 1.0_f64.to_bits());
        assert!(dt["nyquist_s"].is_null());
        let rule = dt["rule"].as_str().unwrap();
        assert!(rule.starts_with(&format!("dt_s is in [{}, {}] s.", token(1.0), token(1.0))));
        assert!(dt["floor_from"].as_str().unwrap().contains("49 mV in the second of the reversal"));
        assert!(dt["floor_from"].as_str().unwrap().contains("For a held 4.5 A discharge at 25 degC, the one load checked, a step of 0.1 s gave the one-second answer to within 0.3 mV"));
        assert!(!dt["floor_from"].as_str().unwrap().contains("Under a held discharge"));
        assert!(!rule.contains("up to 9 mV") && !dt["ceiling_from"].as_str().unwrap().contains("9 mV"));
        let capacity_words = capacity["bound"].as_str().unwrap();
        for words in [
            "4.16 to 4.26 Ah from -12.9 to 12.6 degC",
            "4.20 to 4.30 Ah from 0.0 to 20.4 degC",
            "4.17 to 4.30 Ah from 9.8 to 26.8 degC",
            "4.35 to 4.53 Ah from 23.3 to 31.4 degC",
            "4.56 to 4.70 Ah from 28.1 to 45.9 degC",
            "CapacityOutsideCardAtTemperature",
        ] {
            assert!(capacity_words.contains(words), "{words}");
        }
        let case_words = case["bound"].as_str().unwrap();
        assert!(case_words.contains("CaseTemperatureChangeAboveCard") && case_words.contains("CaseTemperatureSpanAboveCard"));
        assert!(charge["bound"].as_str().unwrap().contains("STATE_OF_CHARGE_ALLOWANCE_PER_STEP"));
        for name in [
            "CapacityOutsideCardAtTemperature",
            "CaseTemperatureChangeAboveCard",
            "CaseTemperatureSpanAboveCard",
            "UndervoltageLimit",
            "OvervoltageLimit",
            "StateOfChargeOutsideCard",
            "SurfaceOutsideCurve",
            "SustainedCurrentOutsideCard",
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
            "One developer's SI wrote the model, wrote the card and ran the check, on one machine",
            "A second agent, which did not write the card, scored it again",
            "the same verdict as the record on 102 of 105 runs",
            "the card steps by one second only",
            "49 mV from the one-second answer in the reversal that was checked",
            "It is the same developer's SI, and no one outside Protocol Company, which makes this engine, has repeated the result.",
        ] {
            assert!(limits.contains(words), "{words}");
        }
        assert!(!limits.contains("not yet repeated by a second party"));
        assert!(!limits.contains("Known defect"));
        assert!(!record.to_string().contains("seal"));
        assert!(!record.to_string().contains("build"));

        assert_eq!(body["advisory_flags"].as_array().unwrap().len(), 1);
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

        let step = &body["example_verify_step"]["parameters"];
        assert_eq!(step["dt_s"].as_f64().unwrap().to_bits(), 1.0_f64.to_bits());
        assert_eq!(step["cell_number"].as_f64().unwrap(), 80.0);
        assert_eq!(step["state_of_charge"].as_f64().unwrap(), 1.0);
        assert_eq!(step["capacity_ah"].as_f64().unwrap(), 4.5006);
        assert_eq!(step["current_a"].as_f64().unwrap(), 4.5);
        let run = &body["example_run_trajectory"];
        assert_eq!(run["segments"][0]["steps"], 600);
        assert_eq!(run["record_every"], 60);
        // The earlier card of the same cell is served beside it, unchanged.
        let earlier = find("energy", "cell_2170_m80").expect("the earlier card is served");
        assert_eq!(earlier.rejects.len(), 16);
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
                creep_cmsx6::CREEP_EXTENSION_MIN_PERCENT.to_bits(),
                creep_cmsx6::CREEP_EXTENSION_MAX_PERCENT.to_bits(),
                false,
                false
            )
        );
        assert_eq!(creep_cmsx6::CREEP_EXTENSION_MIN_PERCENT, 0.1);
        let bound = extension["bound"].as_str().unwrap_or_else(|| panic!("{extension}"));
        assert!(bound.contains("0.2 percent at a stress below 170 MPa and 5 percent from 170 MPa"), "{bound}");
        assert!(bound.contains("CreepExtensionBelowCard"), "{bound}");
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
        // The ends land in the kernel and one spacing outside does not.
        let full = creep_cmsx6::FULL_REACH_STRESS_MIN_MPA;
        let low_limit = creep_cmsx6::CREEP_EXTENSION_MAX_BELOW_FULL_REACH_PERCENT;
        for edge in [creep_cmsx6::CREEP_EXTENSION_MIN_PERCENT, creep_cmsx6::CREEP_EXTENSION_MAX_PERCENT] {
            assert!(creep_cmsx6::CreepState::try_new(&alloy, full, edge).is_some());
        }
        for edge in [creep_cmsx6::CREEP_EXTENSION_MIN_PERCENT, low_limit] {
            assert!(creep_cmsx6::CreepState::try_new(&alloy, creep_cmsx6::STRESS_MIN_MPA, edge).is_some());
        }
        let above = f64::from_bits(creep_cmsx6::CREEP_EXTENSION_MAX_PERCENT.to_bits() + 1);
        assert!(creep_cmsx6::CreepState::try_new(&alloy, full, above).is_none());
        let above_low = f64::from_bits(low_limit.to_bits() + 1);
        assert!(creep_cmsx6::CreepState::try_new(&alloy, creep_cmsx6::STRESS_MIN_MPA, above_low).is_none());
        let below = f64::from_bits(creep_cmsx6::CREEP_EXTENSION_MIN_PERCENT.to_bits() - 1);
        assert!(creep_cmsx6::CreepState::try_new(&alloy, full, below).is_none());
        assert!(creep_cmsx6::CreepState::try_new(&alloy, full, 0.0).is_none());

        let dt = &body["dt_s"];
        assert_eq!(dt["ceiling_s"].as_f64().unwrap().to_bits(), alloy.gate_dt_s().unwrap().to_bits());
        assert!((dt["ceiling_s"].as_f64().unwrap() - 49.974).abs() < 1.0e-3);
        assert_eq!(dt["floor_s"].as_f64().unwrap(), 1.0);
        assert!(dt["nyquist_s"].is_null());
        assert!(dt["rule"].as_str().unwrap().contains("CreepExtensionBeyondCard"));
        assert!(dt["rule"].as_str().unwrap().contains("a start below 0.1 percent creep extension, zero among them, is refused as CreepExtensionBelowCard"));
        assert!(dt["rule"].as_str().unwrap().contains("the upper limit is 5 percent from 170 MPa and 0.2 percent below 170 MPa"));
        assert!(dt["rule"].as_str().unwrap().contains("StressNotHeld"));
        assert!(dt["rule"].as_str().unwrap().contains("Do not subtract one frame's time from another's"));

        assert!(dt["rule"].as_str().unwrap().contains("38.9 against 15.4 to 26.7 at 140 MPa, 7.8 against 2.7 to 3.5 at 170 MPa, 1.69 against 0.61 to 0.66 at 200 MPa and 0.245 against 0.06 to 0.08 at 230 MPa"));
        assert!(!dt["rule"].as_str().unwrap().contains("FLAG_BELOW_COMPARED_EXTENSION"));

        let flags = body["advisory_flags"].as_array().unwrap();
        assert_eq!(flags.len(), 1);
        assert_eq!(flags[0]["name"], "FLAG_CREEP_RATE_RISING");
        assert_eq!(flags[0]["mask"], 1);
        assert_eq!(flags[0]["seals"], true);
        assert_eq!(flags[0]["state"], "creep_rate_rising");
        let flags_slot = body["frame_slots"]
            .as_array()
            .unwrap()
            .iter()
            .find(|slot| slot["slot"] == "flags")
            .unwrap()
            .to_string();
        assert!(!flags_slot.contains("FLAG_BELOW_COMPARED_EXTENSION"), "{flags_slot}");
        assert!(flags_slot.contains("marks no failure of it"), "{flags_slot}");
        assert!(body["specimen"].as_str().is_some_and(|line| line.ends_with("the law misses the measured curves, and there the card refuses.")), "{}", body["specimen"]);
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
        let step = serde_json::to_string(&body["example_verify_step"]).unwrap();
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
            "starts later than every measured curve at all four stresses",
            "38.9 against 15.4 to 26.7 at 140 MPa, 7.8 against 2.7 to 3.5 at 170 MPa, 1.69 against 0.61 to 0.66 at 200 MPa and 0.245 against 0.06 to 0.08 at 230 MPa",
            "The card refuses a state there as CreepExtensionBelowCard",
            "The card now ends at 0.2 percent below 170 MPa",
            "the worst is 0.289, at 170 MPa and 0.33 percent on a held-back test, a margin of 0.011",
            "the card's 1 to 2.09 percent at 140 MPa is beyond any measurement",
            "it reaches 0.8 percent at 683 h where the two tests that got there took about 1000 h",
            "not physical constants of the alloy",
            "not compared with measurement",
            "Scored a second time on 2026-10-09 by a second agent with programs of its own (evidence/checks/second_party/V-24/)",
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
                ("example_verify_step", "parameters"),
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
                ("example_verify_step", "parameters"),
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
        assert_eq!(body["example_verify_step"]["parameters"]["lift_over_drag"], -0.031);
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

        // The one flag is the lift. Past the compared flight the card refuses;
        // it does not flag.
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
            "ended as PastComparedFlight, the end the card declares, at 5700 m or Mach 1.46",
            "with no lift at 227.3 s",
            "with a lift over drag of -0.031 at 211.0 s",
            "Only runs from the locked entry state at 0.1 s were compared with the flight",
            "refused as OffComparedCorridor",
            "A restart from a state such a run returns lands",
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
            "a check and not a test",
            "8 to 13 percent denser",
            "sideways lift",
            "From 0.7.0 the card refuses below 5700 m or Mach 1.46",
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
        assert_eq!(documented.len(), 8);
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
            serde_json::to_string(&body["example_verify_step"]).unwrap(),
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
            let step_text = serde_json::to_string(&body["example_verify_step"]).unwrap();
            let step: Value = serde_json::from_str(&step_text).unwrap();
            assert_eq!(step["domain"], item.domain);
            assert_eq!(step["card"], item.card);
            match crate::verify_step(&state, step).await {
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
        assert_eq!(examples, 62);
    }
}
