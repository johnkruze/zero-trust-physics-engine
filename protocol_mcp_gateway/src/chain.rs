//! A chained request: a run or a step that starts from a sealed upstream run.
//!
//! The engine keeps nothing between requests, so the upstream proof travels
//! as the upstream request: `from` holds the upstream `run_trajectory`
//! request as it was sent, the `run_seal` its answer gave, and, for a
//! coupling, the coupling's name. The upstream request may carry its own
//! `from`, so a chain of runs is sent whole. Before the downstream piece
//! runs, every upstream run is computed again and its seal compared with the
//! seal given (`UpstreamProofBroken`); each handed value is read from the
//! upstream's sealed answer and compared bit for bit with the downstream
//! request (`ContinuationNotFromUpstream`, `CouplingLedgerBreach`); a
//! coupling's conditions on the two requests it joins are decided before
//! any of them runs (`CouplingConditionNotMet`); and a coupled value outside
//! the downstream card's declared range is refused (`CouplingOutsideRange`).
//! Then the downstream request, with `from` taken
//! out, is run exactly as the same request typed by hand: its proof, state
//! and seal are that request's. The answer adds `chain`, whose `chain_seal`
//! commits the downstream seal to the upstream's (`genesis_seal::chain`).

use genesis_seal::chain::{
    Handed, KIND_CONTINUATION, KIND_COUPLING, MAX_CHAIN_LINKS, chain_seal, chain_tail, request_value_hex,
    values_at,
};
use serde_json::{Map, Value};

use crate::couplings::{self, Coupling};
use crate::fresh_start::Carried;
use crate::trajectory::{self, Prepared, TrajectoryResponse};
use crate::view::View;
use crate::DoorError;

/// The key of a run or step request that names its upstream.
pub(crate) const FROM: &str = "from";
const FROM_KEYS: &[&str] = &["request", "run_seal", "coupling"];

/// The most steps of every run of a chain together, the downstream run
/// included: the engine's limit for one run.
pub const MAX_CHAIN_STEPS: u64 = trajectory::MAX_TRAJECTORY_STEPS;

/// The downstream piece of a chain: a run, a step, or a solve (only through
/// a coupling into a solver).
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Target {
    Run,
    Step,
    Solve,
}

/// Take `from` out of a request. The request without it is the one that is
/// prepared, run and sealed.
pub(crate) fn take(mut request: Value) -> (Value, Option<Value>) {
    let from = request.as_object_mut().and_then(|object| object.shift_remove(FROM));
    (request, from)
}

fn refused(sentence: impl std::fmt::Display, name: &str) -> String {
    format!("Physical Violation: {sentence} ({name}).")
}

fn violation(sentence: impl std::fmt::Display, name: &str) -> DoorError {
    DoorError::Unprocessable(refused(sentence, name))
}

/// Every step a chained request asks for, upstream runs included, read from
/// the text alone: the open connection counts it before a permit is taken.
pub fn chained_steps(request: &Value) -> u64 {
    let mut total = own_steps(request);
    let mut from = request.get(FROM);
    let mut depth = 0;
    while let Some(link) = from {
        depth += 1;
        let Some(upstream) = link.get("request").filter(|_| depth <= MAX_CHAIN_LINKS) else {
            break;
        };
        total = total.saturating_add(own_steps(upstream));
        from = upstream.get(FROM);
    }
    total
}

fn own_steps(request: &Value) -> u64 {
    request
        .get("segments")
        .and_then(Value::as_array)
        .map(|segments| {
            segments
                .iter()
                .filter_map(|segment| segment.get("steps").and_then(Value::as_u64))
                .fold(0_u64, u64::saturating_add)
        })
        .unwrap_or(0)
}

/// The declaration of `from` in the schemas of `run_trajectory` and
/// `compute_step`.
pub(crate) fn from_schema(full: bool) -> Value {
    let description = if full {
        "Optional: start from a sealed run. request: that run_trajectory request as sent, less its readout if it had one: a readout is not part of a run's seal, and a request here that carries one is refused (it may hold its own from); run_seal: its seal; coupling: absent to continue the same card (describe_card says how), or a coupling's name. The upstream is computed again; each carried value must equal its sealed value bit for bit, and a request whose upstream seal or carried value does not hold is refused by name. The run_seal is that of this request without from; chain.chain_seal commits it to the upstream."
    } else {
        "Optional: a sealed run to start from, as run_trajectory takes it."
    };
    serde_json::json!({
        "type": "object",
        "description": description,
        "properties": {
            "request": { "type": "object" },
            "run_seal": { "type": "string", "pattern": "^[0-9a-f]{96}$" },
            "coupling": { "type": "string" }
        },
        "required": ["request", "run_seal"],
        "additionalProperties": false
    })
}

/// The declaration of `from` in the schema of `solve`: a sealed card run
/// handed in through a coupling into a solver.
pub(crate) fn solve_from_schema(full: bool) -> Value {
    let description = if full {
        "Optional: a sealed card run handed in through a coupling into this solver (describe_solver lists them under couplings). request: that run_trajectory request as sent, less its readout; run_seal: its seal; coupling: the coupling's name. The run is computed again and its seal compared, and each handed value must equal its sealed value bit for bit in parameters. The solution_seal is that of this request without from; chain.chain_seal commits it to the upstream run."
    } else {
        "Optional: a sealed card run handed in through a coupling, as run_trajectory takes it."
    };
    serde_json::json!({
        "type": "object",
        "description": description,
        "properties": {
            "request": { "type": "object" },
            "run_seal": { "type": "string", "pattern": "^[0-9a-f]{96}$" },
            "coupling": { "type": "string" }
        },
        "required": ["request", "run_seal", "coupling"],
        "additionalProperties": false
    })
}

/// The cards a continuation through `from` is refused on, each with why:
/// their start fields do not carry the state a run goes on from, so a
/// "continuation" would be another run and not the same run cut in two.
pub(crate) static NOT_CONTINUED: &[(&str, &str)] = &[
    ("reactor_triga_jsi", "its start fields are the start temperature and the rod table, which its final state carries unchanged, so a run sent from a returned state is the same pulse again from its start; the power, the precursors and the fuel temperature a pulse reaches are not start fields"),
    ("reactor_spert3", "its start fields are the inlet temperature, the system pressure, the flow and the initial power, from which it starts the steady reactor, so a run sent from a returned state is the same ejection again from the steady start; the power, the precursors, the ring, clad and water temperatures and the rod's travel are not start fields"),
    ("reactor_spert3_ccore", "its start fields are the initial temperature and the system pressure, which a transient does not change, so a run sent from a returned state is the same transient again from the step in reactivity; the power, the precursors, the plate, film and water temperature rises and the vapour are not start fields"),
    ("los_jwst_fine_guide", "its start fields are the lock point and the quiet time; the pointing filter's estimates, covariances and disturbance modes are not start fields, so a run cut in two is not the uncut run"),
    ("cell_2170_model3", "its start fields are the cell, the state of charge, the capacity and the span of case temperature; the core temperature, the surface state of charge, the diffusion lags, the hysteresis and the salt's concentration difference are not start fields, so a run cut in two is not the uncut run"),
    ("grid_gb", "the card takes only the start of its event: its responses delivered and its ledger are not start fields, so a returned state is no start of it"),
    ("fingerpad_p02_glass", "the card takes only a twist from zero: a returned state is refused as a start by the card itself"),
    ("reactor_msre_pump", "its one start field is the pump speed of the starting equilibrium; the precursor content of the 252 parcels of salt, the fuel flow behind the pump and the ledger are not start fields, so a run sent from a returned state starts again from an equilibrium"),
    ("grasp_lift_glass_4tu", "its one start field is the hand's velocity at the grip; the object's height and velocity, the force of each of the 32 elements of a pad's contact and the ledgers are not start fields, so a run sent from a returned state is a new grip on an object at rest on the table"),
];

/// Why a card is not continued through `from`, or `None` when it is.
pub(crate) fn not_continued(card: &str) -> Option<&'static str> {
    NOT_CONTINUED.iter().find(|(name, _)| *name == card).map(|(_, why)| *why)
}

/// The one card continued through `from` from some of its states only: the
/// crater, whose start field, the radius, carries its whole state only once
/// the crater has stopped. While it grows the card sets the wall's speed from
/// the radius at a start, so the speed the upstream integrated and the energy
/// accounts are not carried: from 1.75 m at 0.25 s, a run of 1,000 steps cut
/// at 500 ends 27,385 units in the last place from the run uncut in its
/// radius (the breadth tests of 16 October 2026). Stopped, it is at
/// `FINAL_RADIUS_M` with its wall at rest, which the radius carries bit for
/// bit, and a run cut in two is the run uncut but for its clock.
pub(crate) const CRATER_CONTINUED_ONLY: &str = "It is served only from a crater that has stopped, at 8.571304329477922 m with its wall at rest: while the crater grows, the wall's speed and the energy accounts are not start fields (the card sets the speed from the radius at a start), so a run cut in two before the stop is not the uncut run (from 1.75 m at 0.25 s, a run of 1,000 steps cut at 500 ends 27,385 units in the last place from the uncut run's radius), and a continuation from a radius under 8.571304329477922 m is refused before any physics (ContinuationNotOnCard).";

/// Why a continuation of `card` into `request` (a run's `initial`, a step's
/// `parameters`) is not served though the card continues, or `None`. Read
/// from the request before any physics: a continuation states every start
/// field bit for bit as the upstream returned it, so its radius is the
/// upstream's, and a radius that is not the stopped crater's is refused
/// whatever the upstream holds.
fn continued_only_from(card: &str, request: &Value, target: Target) -> Option<String> {
    if card != "crater_ryugu_sci" {
        return None;
    }
    let section = match target {
        Target::Run => "initial",
        Target::Step | Target::Solve => "parameters",
    };
    let stated = request.get(section)?.get("radius_m")?;
    let bits = request_value_hex(stated)?;
    let stopped = genesis_kernel::crater_ryugu_sci::FINAL_RADIUS_M;
    if bits.as_str() == Some(format!("{:016x}", stopped.to_bits()).as_str()) {
        return None;
    }
    Some(format!(
        "aerospace/crater_ryugu_sci is continued through from only from a crater that has stopped, at {stopped} m with its wall at rest, and '{section}.radius_m' is {stated}: while the crater grows, the wall's speed and the energy accounts are not start fields, so a run cut in two there is not the uncut run"
    ))
}

/// For each card continued through `from`: the published values in which
/// its worked run, cut in two through `from`, ends otherwise than the same
/// run uncut (`differ`), and of those the ones that differ by at most one
/// unit in the last place (`within_one_ulp`). Measured by
/// `tests/chain_router.rs`, which holds this table to the engine.
pub(crate) struct Continued {
    pub card: &'static str,
    pub differ: &'static [&'static str],
    pub within_one_ulp: &'static [&'static str],
}

const LEDGER: &str = "residual_conservation_error";

pub(crate) static CONTINUED: &[Continued] = &[
    Continued { card: "ti64_lattice", differ: &[], within_one_ulp: &[] },
    Continued { card: "resonance", differ: &[], within_one_ulp: &[] },
    Continued { card: "grid_node", differ: &["t_s", "boundary_work_j", "energy_anchor_j", "energy_defect_j", "residual_conservation_error", "voltage_spike_v"], within_one_ulp: &[] },
    Continued { card: "swing", differ: &["t_s"], within_one_ulp: &[] },
    Continued { card: "battery_sag", differ: &["t_s"], within_one_ulp: &[] },
    Continued { card: "thermal", differ: &["t_s"], within_one_ulp: &[] },
    Continued { card: "tesseract", differ: &["t_s"], within_one_ulp: &[] },
    Continued { card: "hydraulic_fluid", differ: &["t_s"], within_one_ulp: &[] },
    Continued { card: "cable_stretch", differ: &["t_s"], within_one_ulp: &[] },
    Continued { card: "crater_ryugu_sci", differ: &["t_s"], within_one_ulp: &[] },
    Continued { card: "tokamak_jet_dte1", differ: &["energy_anchor_j", "energy_defect_j", "energy_lost_j", "energy_supplied_j", "fusion_energy_j", LEDGER], within_one_ulp: &[] },
    Continued {
        card: "runaway_18650_lco",
        differ: &["energy_defect_j", "heat_anode_j", "heat_cathode_j", "heat_electrolyte_j", "heat_lost_j", "heat_released_j", "heat_sei_j", "heat_separator_j", "heat_stored_j", LEDGER, "start_temperature_k", "t_s"],
        within_one_ulp: &[],
    },
    Continued {
        card: "generator_mmrtg_seebeck",
        differ: &["converter_account_anchor_j", "converter_account_defect_j", "converter_account_error", "electrical_energy_j", "energy_anchor_j", "energy_defect_j", "fading_electrical_energy_j", "heat_rejected_j", LEDGER],
        within_one_ulp: &[],
    },
    Continued {
        card: "momentum_jwst_dihedral",
        differ: &["h_anchor_eci_nms", "impulse_eci_nms", "momentum_defect_nms", LEDGER, "t_s"],
        within_one_ulp: &[],
    },
    Continued {
        card: "momentum_jwst_srp",
        differ: &["h_anchor_eci_nms", "impulse_eci_nms", "momentum_defect_nms", LEDGER, "t_s"],
        within_one_ulp: &[],
    },
    Continued { card: "solar_system", differ: &["t_s"], within_one_ulp: &[] },
    Continued { card: "cell_18650pf", differ: &["electrical_work_j", "energy_anchor_j", "energy_defect_j", "joule_j", LEDGER, "t_s"], within_one_ulp: &[] },
    Continued {
        card: "generator_mmrtg_f1",
        differ: &["conversion_efficiency", "converter_account_anchor_j", "converter_account_defect_j", "converter_account_error", "converter_fading_fraction", "converter_fading_yield_j", "electrical_energy_j", "electrical_power_w", "energy_anchor_j", "energy_defect_j", "fading_electrical_energy_j", "heat_rejected_j", LEDGER],
        within_one_ulp: &["conversion_efficiency", "converter_fading_fraction", "converter_fading_yield_j", "electrical_power_w"],
    },
    Continued { card: "capsule_stardust", differ: &["angle_rad", "drag_loss_j_kg", "energy_anchor_j_kg", "energy_defect_j_kg", "ground_distance_m", LEDGER, "t_s"], within_one_ulp: &[] },
    Continued { card: "satellite_lageos1", differ: &["angular_momentum_anchor_m2_s", "angular_momentum_defect_m2_s", "energy_defect_j_per_kg", "external_torque_m2_s", "external_work_j_per_kg", "jacobi_anchor_j_per_kg", LEDGER, "t_s"], within_one_ulp: &[] },
    Continued { card: "cantilever_eth", differ: &["energy_anchor_j_per_kg", "energy_defect_j_per_kg", "heat_j_per_kg", LEDGER, "t_s"], within_one_ulp: &[] },
    Continued { card: "creep_cmsx6", differ: &["heat_dissipated_mj_per_m3", LEDGER, "t_s", "work_by_load_mj_per_m3", "work_defect_mj_per_m3"], within_one_ulp: &[] },
    Continued { card: "capsule_insight", differ: &["angle_rad", "drag_loss_j_kg", "energy_anchor_j_kg", "energy_defect_j_kg", "ground_distance_m", LEDGER, "t_s"], within_one_ulp: &[] },
    Continued {
        card: "link_gothenburg_10010",
        differ: &["antenna_account_anchor_db", "antenna_account_defect_db", "antenna_account_error", "antenna_built_db", "antenna_energy_j", "dry_path_energy_j", "energy_defect_j", "rain_energy_j", "received_energy_j", LEDGER, "t_s"],
        within_one_ulp: &[],
    },
    Continued { card: "tokamak_jet_dte1", differ: &["energy_anchor_j", "energy_defect_j", "energy_lost_j", "energy_supplied_j", "fusion_energy_j", LEDGER], within_one_ulp: &[] },
    Continued { card: "tokamak_vde_mast", differ: &["energy_anchor_j", "energy_defect_j", "energy_dissipated_j", LEDGER, "stored_scale_anchor_j", "t_s"], within_one_ulp: &[] },
    Continued {
        card: "accumulator_sbo450",
        differ: &["energy_defect_j", "gas_force_n", "gas_pressure_pa", "gas_volume_m3", "heat_moved_j", "heat_share", "heat_to_wall_j", LEDGER, "start_energy_j", "t_s", "work_moved_j", "work_on_gas_j"],
        within_one_ulp: &["gas_force_n", "gas_pressure_pa", "gas_volume_m3", "heat_moved_j", "heat_share", "work_moved_j"],
    },
    Continued {
        card: "runaway_18650_lco",
        differ: &["energy_defect_j", "heat_anode_j", "heat_cathode_j", "heat_electrolyte_j", "heat_lost_j", "heat_released_j", "heat_sei_j", "heat_stored_j", LEDGER, "start_temperature_k", "t_s"],
        within_one_ulp: &[],
    },
    Continued { card: "tire_eve_r19_ice", differ: &["drawbar_work_j", "energy_defect_j", LEDGER, "rim_travel_m", "slip_heat_j", "t_s", "wheel_work_j"], within_one_ulp: &[] },
];

/// What `describe_card` says of a continuation through `from` on a card.
pub(crate) fn continuation_statement(card: &str) -> Value {
    if let Some(why) = not_continued(card) {
        return serde_json::json!({
            "served": false,
            "sentence": format!("A continuation through from is refused on this card (ContinuationNotOnCard): {why}."),
        });
    }
    let Some(entry) = CONTINUED.iter().find(|entry| entry.card == card) else {
        return serde_json::json!({ "served": false, "sentence": "A continuation through from is refused on this card (ContinuationNotOnCard)." });
    };
    let mut sentence = String::from("A continuation through from starts from the start fields this card takes, each equal to the upstream's sealed value bit for bit. ");
    if card == "crater_ryugu_sci" {
        sentence.push_str(CRATER_CONTINUED_ONLY);
        sentence.push_str(" Measured on this card's worked run, which stops at step 1,597 of 1,600, cut in two through from after the stop: ");
    } else {
        sentence.push_str("Measured on this card's worked run cut in two through from: ");
    }
    if entry.differ.is_empty() {
        sentence.push_str("the continued run ends on the uncut run's final state, every published value bit for bit.");
    } else {
        sentence.push_str("the continued run's final state equals the uncut run's bit for bit except in ");
        sentence.push_str(&entry.differ.join(", "));
        if !entry.within_one_ulp.is_empty() {
            sentence.push_str("; of these, ");
            sentence.push_str(&entry.within_one_ulp.join(", "));
            sentence.push_str(" differ by one unit in the last place");
        }
        sentence.push('.');
    }
    serde_json::json!({ "served": true, "sentence": sentence })
}

/// How a link follows the run before it.
#[derive(Clone, Copy)]
enum Kind {
    Continuation,
    Coupling(&'static Coupling),
}

impl Kind {
    fn kind(self) -> &'static str {
        match self {
            Kind::Continuation => KIND_CONTINUATION,
            Kind::Coupling(_) => KIND_COUPLING,
        }
    }

    fn coupling(self) -> Option<&'static str> {
        match self {
            Kind::Continuation => None,
            Kind::Coupling(coupling) => Some(coupling.name),
        }
    }

    /// The start fields of the downstream request this link carries: every
    /// one for a continuation, which carries the upstream's whole start bit
    /// for bit, and for a coupling only the fields it hands.
    fn carried(self) -> Carried {
        match self {
            Kind::Continuation => Carried::Every,
            Kind::Coupling(coupling) => Carried::Only(
                coupling
                    .mappings
                    .iter()
                    .map(|mapping| {
                        mapping
                            .to
                            .strip_prefix("initial.")
                            .or_else(|| mapping.to.strip_prefix("segments[*].load."))
                            .unwrap_or(mapping.to)
                    })
                    .collect(),
            ),
        }
    }
}

/// What a link written in `from` carries, read from its 'coupling' alone; a
/// name that is not a coupling is refused when that link is read.
fn carried_by(link: &Value) -> Carried {
    match link.get("coupling") {
        None => Carried::Every,
        Some(Value::String(name)) => match couplings::find(name) {
            Some(coupling) => Kind::Coupling(coupling).carried(),
            None => Carried::Every,
        },
        Some(_) => Carried::Every,
    }
}

/// One upstream run, read and prepared before any permit is taken.
struct Level {
    prepared: Prepared,
    request: Value,
    pair: (String, String),
    given_seal: String,
    /// How the next link (the next level, or the downstream piece) follows this one.
    next: Kind,
}

/// A chain read and held to its limits: the upstream runs from the first,
/// and the downstream request.
pub(crate) struct Plan {
    levels: Vec<Level>,
    downstream: Value,
    pair: (String, String),
    target: Target,
}

fn pair_of(request: &Value) -> Option<(String, String)> {
    Some((
        request.get("domain")?.as_str()?.to_string(),
        request.get("card")?.as_str()?.to_string(),
    ))
}

/// Read `from`, prepare every upstream run, and hold the chain to its limits,
/// before any physics. `downstream` is the request without `from`;
/// `downstream_steps` the steps it asks for itself.
pub(crate) fn read(
    view: &View,
    downstream: &Value,
    from: Value,
    target: Target,
    downstream_steps: u64,
) -> Result<Plan, DoorError> {
    let pair = if target == Target::Solve {
        let name = downstream.get("solver").and_then(Value::as_str).ok_or_else(|| {
            DoorError::Unprocessable("Physical Violation: a solve request with 'from' names its 'solver' as a string.".to_string())
        })?;
        if !couplings::sees(view, (couplings::SOLVER, name)) {
            return Err(DoorError::NotFound(crate::SOLVER_NOT_RECOGNIZED.to_string()));
        }
        (couplings::SOLVER.to_string(), name.to_string())
    } else {
        let pair = pair_of(downstream).ok_or_else(|| {
            DoorError::Unprocessable("Physical Violation: a request with 'from' names its 'domain' and 'card' as strings.".to_string())
        })?;
        if view.find(&pair.0, &pair.1).is_none() {
            return Err(DoorError::NotFound(crate::CARD_NOT_RECOGNIZED.to_string()));
        }
        pair
    };
    // Nearest upstream first, as `from` nests.
    let mut levels = Vec::new();
    let mut next = Some(from);
    let mut total_steps = downstream_steps;
    while let Some(link) = next.take() {
        let depth = levels.len() + 1;
        if depth + 1 > MAX_CHAIN_LINKS {
            return Err(violation(
                format!("the chain holds more than {MAX_CHAIN_LINKS} runs, a step at its end counted as one, the most one request may carry"),
                "ChainTooLong",
            ));
        }
        let at = if depth == 1 { "'from'".to_string() } else { format!("'from' at depth {depth}") };
        let object = link
            .as_object()
            .ok_or_else(|| DoorError::Unprocessable(format!("Physical Violation: {at} must be a JSON object.")))?;
        for key in object.keys() {
            if !FROM_KEYS.contains(&key.as_str()) {
                return Err(DoorError::Unprocessable(format!(
                    "Physical Violation: '{key}' is not a field of {at}: it takes 'request', 'run_seal' and, for a coupling, 'coupling'."
                )));
            }
        }
        let request = object
            .get("request")
            .ok_or_else(|| DoorError::Unprocessable(format!("Missing required parameter: 'request' in {at} (InputMissing).")))?
            .clone();
        if !request.is_object() {
            return Err(DoorError::Unprocessable(format!(
                "Physical Violation: 'request' in {at} must be a run_trajectory request, a JSON object."
            )));
        }
        let given_seal = match object.get("run_seal") {
            Some(Value::String(text)) if crate::json_text::is_lower_hex(text, 96) => text.clone(),
            Some(_) => {
                return Err(DoorError::Unprocessable(format!(
                    "Physical Violation: 'run_seal' in {at} must be 96 lowercase hex digits."
                )));
            }
            None => return Err(DoorError::Unprocessable(format!("Missing required parameter: 'run_seal' in {at} (InputMissing)."))),
        };
        let kind = match object.get("coupling") {
            None => Kind::Continuation,
            Some(Value::String(name)) => match couplings::find(name) {
                Some(coupling) => Kind::Coupling(coupling),
                None => {
                    return Err(violation(
                        format!("'{name}' in {at} is not a coupling the engine serves; describe_card lists a card's couplings under couplings"),
                        "UnknownCoupling",
                    ));
                }
            },
            Some(_) => {
                return Err(DoorError::Unprocessable(format!(
                    "Physical Violation: 'coupling' in {at} must be a string."
                )));
            }
        };
        let (request, upstream_from) = take(request);
        if request.get(crate::readout::READOUT).is_some() {
            return Err(DoorError::Unprocessable(format!(
                "Physical Violation: the request in {at} carries a readout; a readout is not part of a run's seal, so send the upstream request without it."
            )));
        }
        // The first run of a chain is a fresh start; a run with its own
        // upstream carries that upstream's values, all of them through a
        // continuation and the ones it hands through a coupling.
        let prepared = match &upstream_from {
            Some(link) => trajectory::prepare_continued(view, request.clone(), &carried_by(link)),
            None => trajectory::prepare_in(view, request.clone()),
        }
        .map_err(|error| match error {
            DoorError::Unprocessable(sentence) => DoorError::Unprocessable(format!(
                "{sentence} That is the upstream run in {at}, refused before it runs."
            )),
            other => other,
        })?;
        total_steps = total_steps.saturating_add(prepared.steps_requested());
        if total_steps > MAX_CHAIN_STEPS {
            return Err(violation(
                format!("the runs of the chain ask for more than {MAX_CHAIN_STEPS} steps together, the engine's limit for one run"),
                "ChainStepsAboveLimit",
            ));
        }
        let up_pair = pair_of(&request).expect("a prepared request names its pair");
        levels.push(Level {
            prepared,
            request,
            pair: up_pair,
            given_seal,
            next: kind,
        });
        next = upstream_from;
    }
    levels.reverse();
    // Each link names pieces it may join, and a coupling's conditions hold,
    // read before any physics.
    for index in 0..levels.len() {
        let up = &levels[index].pair;
        let down = if index + 1 < levels.len() { &levels[index + 1].pair } else { &pair };
        match levels[index].next {
            Kind::Continuation => {
                if up != down {
                    return Err(violation(
                        format!(
                            "a continuation goes on with the same card, and the upstream run is {}/{} where this request is {}/{}; a run of another card follows only through a named coupling",
                            up.0, up.1, down.0, down.1
                        ),
                        "ContinuationNotFromUpstream",
                    ));
                }
                if let Some(why) = not_continued(&down.1) {
                    return Err(violation(
                        format!("{}/{} is not continued through from: {why}", down.0, down.1),
                        "ContinuationNotOnCard",
                    ));
                }
                let (next_request, next_target) = if index + 1 < levels.len() {
                    (&levels[index + 1].request, Target::Run)
                } else {
                    (downstream, target)
                };
                if let Some(sentence) = continued_only_from(&down.1, next_request, next_target) {
                    return Err(violation(sentence, "ContinuationNotOnCard"));
                }
            }
            Kind::Coupling(coupling) => {
                let names = |pair: (&str, &str)| (pair.0.to_string(), pair.1.to_string());
                if names(coupling.upstream) != *up || names(coupling.downstream) != *down {
                    return Err(violation(
                        format!(
                            "the coupling '{}' joins {}/{} to {}/{}, and this link joins {}/{} to {}/{}",
                            coupling.name,
                            coupling.upstream.0,
                            coupling.upstream.1,
                            coupling.downstream.0,
                            coupling.downstream.1,
                            up.0,
                            up.1,
                            down.0,
                            down.1
                        ),
                        "CouplingNotOfThesePieces",
                    ));
                }
                // The coupling's conditions on the two requests it joins.
                let last = index + 1 == levels.len();
                let joined = couplings::Joined {
                    upstream: &levels[index].request,
                    downstream: if last { downstream } else { &levels[index + 1].request },
                    step: last && target != Target::Run,
                };
                if let Some(sentence) = coupling.unmet(&joined) {
                    return Err(violation(sentence, "CouplingConditionNotMet"));
                }
            }
        }
    }
    Ok(Plan {
        levels,
        downstream: downstream.clone(),
        pair,
        target,
    })
}

/// What the upstream runs settled, for the downstream answer's `chain`.
pub(crate) struct Settled {
    kind: Kind,
    upstream_run_seal: String,
    upstream_chain_seal: Option<String>,
    handed: Vec<Handed>,
}

impl Plan {
    /// The start fields of the downstream request that its `from` carries.
    pub(crate) fn carried(&self) -> Carried {
        self.levels.last().map_or(Carried::Nothing, |level| level.next.carried())
    }

    /// Every upstream run computed again, checked against its seal, and each
    /// link's handed values checked. Runs on the physics pool.
    pub(crate) fn settle(self) -> Result<(Settled, Value), String> {
        let Plan { levels, downstream, pair, target } = self;
        let mut previous: Option<(TrajectoryResponse, (String, String), Kind)> = None;
        let mut commitment: Option<String> = None;
        let mut chained: Option<String> = None;
        for (index, level) in levels.into_iter().enumerate() {
            let depth_name = format!("the upstream run {} of the chain", index + 1);
            // This run's start is checked against the run before it before it runs.
            let handed = match &previous {
                Some((up, up_pair, kind)) => {
                    Some(handed_values(up, up_pair, *kind, &level.request, &level.pair, Target::Run)?)
                }
                None => None,
            };
            let response = trajectory::execute(level.prepared)?;
            if response.run_seal != level.given_seal {
                return Err(refused(
                    format!(
                        "{depth_name} was computed again and its run_seal is not the {} given; the upstream answer is not the engine's answer to that request",
                        level.given_seal
                    ),
                    "UpstreamProofBroken",
                ));
            }
            // The chain seal of this run over the one before it.
            match (&previous, handed) {
                (Some((_, _, kind)), Some(handed)) => {
                    let tail = chain_tail(kind.kind(), kind.coupling(), &handed);
                    let upstream_commitment = commitment.clone().expect("a commitment after the first run");
                    let seal = chain_seal(&upstream_commitment, &tail, &response.run_seal);
                    chained = Some(seal.clone());
                    commitment = Some(seal);
                }
                _ => commitment = Some(response.run_seal.clone()),
            }
            previous = Some((response, level.pair, level.next));
        }
        let (up, up_pair, kind) = previous.expect("a chain has an upstream run");
        let handed = handed_values(&up, &up_pair, kind, &downstream, &pair, target)?;
        Ok((
            Settled {
                kind,
                upstream_run_seal: up.run_seal.clone(),
                upstream_chain_seal: chained,
                handed,
            },
            downstream,
        ))
    }
}

impl Settled {
    /// The answer's `chain`, with the downstream seal committed.
    pub(crate) fn chain_value(&self, downstream_seal: &str) -> Value {
        let commitment = self.upstream_chain_seal.as_deref().unwrap_or(&self.upstream_run_seal);
        let tail = chain_tail(self.kind.kind(), self.kind.coupling(), &self.handed);
        let seal = chain_seal(commitment, &tail, downstream_seal);
        let handed: Vec<Value> = self
            .handed
            .iter()
            .map(|entry| {
                let mut object = Map::new();
                object.insert("from".into(), Value::String(entry.from.clone()));
                object.insert("to".into(), Value::String(entry.to.clone()));
                object.insert(
                    "unit".into(),
                    entry.unit.clone().map(Value::String).unwrap_or(Value::Null),
                );
                object.insert("value_hex".into(), entry.value_hex.clone());
                Value::Object(object)
            })
            .collect();
        let mut object = Map::new();
        object.insert("kind".into(), Value::String(self.kind.kind().to_string()));
        object.insert(
            "coupling".into(),
            self.kind.coupling().map(|name| Value::String(name.to_string())).unwrap_or(Value::Null),
        );
        object.insert("upstream_run_seal".into(), Value::String(self.upstream_run_seal.clone()));
        object.insert(
            "upstream_chain_seal".into(),
            self.upstream_chain_seal.clone().map(Value::String).unwrap_or(Value::Null),
        );
        object.insert("handed".into(), Value::Array(handed));
        object.insert("chain_seal".into(), Value::String(seal));
        Value::Object(object)
    }
}

/// The path a run request's value stands at, in the downstream request.
fn target_path(to: &str, target: Target) -> String {
    match target {
        Target::Run | Target::Solve => to.to_string(),
        Target::Step => {
            let name = to
                .strip_prefix("initial.")
                .or_else(|| to.strip_prefix("segments[*].load."))
                .unwrap_or(to);
            format!("parameters.{name}")
        }
    }
}

fn hex_of_bits(value: f64) -> String {
    format!("{:016x}", value.to_bits())
}

fn value_of_hex(hex: &Value) -> Option<f64> {
    hex.as_str()
        .and_then(|text| u64::from_str_radix(text, 16).ok())
        .map(f64::from_bits)
}

/// The values one link hands on, checked against the upstream's sealed
/// answer and the downstream request.
fn handed_values(
    up: &TrajectoryResponse,
    up_pair: &(String, String),
    kind: Kind,
    down: &Value,
    down_pair: &(String, String),
    target: Target,
) -> Result<Vec<Handed>, String> {
    let carried = match kind {
        Kind::Continuation => up.outcome == "completed",
        Kind::Coupling(_) => up.outcome == "completed" || up.outcome == "ended",
    };
    let Some(state) = up.final_state.as_ref().and_then(Value::as_object).filter(|_| carried) else {
        let what = match kind {
            Kind::Continuation => "a continuation goes on only from a run that completed",
            Kind::Coupling(_) => "a coupling hands on only from a run that completed or ended at an end its card declares, with a step landed",
        };
        return Err(refused(
            format!(
                "the upstream {}/{} run has outcome '{}' with {} steps landed; {what}",
                up_pair.0, up_pair.1, up.outcome, up.steps_landed
            ),
            "UpstreamOutcomeNotCarried",
        ));
    };
    match kind {
        Kind::Continuation => continuation(up, state, down, down_pair, target),
        Kind::Coupling(coupling) => coupled(coupling, up, state, down, target),
    }
}

/// Where a continuation reads a start field when the card's returned state
/// gives it under another name, as the card's own description of the field
/// says. Every other start field is read from the final state under its own
/// name.
pub(crate) fn continuation_source(card: &str, field: &str) -> Option<&'static str> {
    Some(match (card, field) {
        // "A state the card returned is handed back with its t_s as
        // start_t_s"; each other start value "a returned state gives as" the
        // named field.
        ("ti64_lattice", "start_t_s") => "final_state.t_s",
        ("ti64_lattice", "start_force_n") => "final_state.applied_force_n",
        ("ti64_lattice", "start_temperature_k") => "final_state.temperature_k",
        ("ti64_lattice", "start_energy_anchor_j") => "final_state.energy_anchor_j",
        ("ti64_lattice", "start_boundary_work_j") => "final_state.boundary_work_j",
        // "for a run from a state an earlier run returned, that run's held current".
        ("cell_18650pf", "held_current_a") => "inputs.initial.held_current_a",
        // The generator's t_s is the mission time since landing.
        ("generator_mmrtg_f1", "mission_time_s") => "final_state.t_s",
        ("generator_mmrtg_seebeck", "mission_time_s") => "final_state.t_s",
        // "a run or a single step continued from a returned state states that state's t_s".
        ("tokamak_jet_dte1", "elapsed_time_s") => "final_state.t_s",
        _ => return None,
    })
}

/// The exact value a continuation reads at `source` in the upstream answer.
fn upstream_value<'a>(up: &'a TrajectoryResponse, state: &'a Map<String, Value>, source: &str) -> Option<Value> {
    if let Some(name) = source.strip_prefix("final_state.") {
        return state.get(&format!("{name}_hex")).cloned();
    }
    let path = source.strip_prefix("inputs.")?;
    let values = values_at(&up.inputs, path).ok()?;
    let first = values.first().copied().flatten().and_then(request_value_hex)?;
    values
        .iter()
        .all(|value| value.and_then(request_value_hex).as_ref() == Some(&first))
        .then_some(first)
}

fn continuation(
    up: &TrajectoryResponse,
    state: &Map<String, Value>,
    down: &Value,
    pair: &(String, String),
    target: Target,
) -> Result<Vec<Handed>, String> {
    let section = match target {
        Target::Run => "initial",
        Target::Step | Target::Solve => "parameters",
    };
    let stated = down.get(section).and_then(Value::as_object);
    let phases = crate::descriptor::phase_names(&pair.0, &pair.1);
    let not_from = |sentence: String| refused(sentence, "ContinuationNotFromUpstream");
    let mut handed = Vec::new();
    for name in phases {
        let source = continuation_source(&pair.1, name)
            .map(str::to_string)
            .unwrap_or_else(|| format!("final_state.{name}"));
        let upstream = upstream_value(up, state, &source);
        let given = stated.and_then(|object| object.get(*name));
        match (upstream, given) {
            (None, None) => {}
            (None, Some(_)) => {
                return Err(not_from(format!(
                    "'{section}.{name}' is stated, and the upstream run carries no '{source}', so it cannot be carried from it"
                )));
            }
            (Some(_), None) => {
                return Err(not_from(format!(
                    "'{section}.{name}' is not stated, and the upstream run carries it as '{source}'; a continuation states every start field the upstream carries, as returned"
                )));
            }
            (Some(upstream), Some(given)) => {
                let bits = request_value_hex(given);
                if bits.as_ref() != Some(&upstream) {
                    return Err(not_from(format!(
                        "'{section}.{name}' is {} and the upstream run carries {} as '{source}'; a continuation starts from the upstream's sealed values bit for bit",
                        bits.map(|value| value.to_string()).unwrap_or_else(|| "not a number".to_string()),
                        upstream
                    )));
                }
                handed.push(Handed {
                    from: source,
                    to: format!("{section}.{name}"),
                    unit: None,
                    value_hex: upstream,
                });
            }
        }
    }
    if handed.is_empty() {
        return Err(not_from(format!(
            "the upstream run carries none of the start fields of {}/{}",
            pair.0, pair.1
        )));
    }
    Ok(handed)
}

fn coupled(
    coupling: &'static Coupling,
    up: &TrajectoryResponse,
    state: &Map<String, Value>,
    down: &Value,
    target: Target,
) -> Result<Vec<Handed>, String> {
    let mut so_far: Vec<(&'static str, f64)> = Vec::new();
    let mut handed = Vec::new();
    // The first place of a downstream array after the values handed to it.
    let mut after_handed: Option<usize> = None;
    for mapping in coupling.mappings {
        let value_hex = if let Some(name) = mapping.from.strip_prefix("final_state.") {
            state.get(&format!("{name}_hex")).cloned()
        } else if let Some(slot) = mapping.from.strip_prefix("frames[*].") {
            frame_values(up, slot).map(|values| Value::Array(values.iter().map(|value| Value::String(hex_of_bits(*value))).collect()))
        } else {
            let path = mapping.from.strip_prefix("inputs.").unwrap_or(mapping.from);
            let values = values_at(&up.inputs, path).map_err(|error| refused(error, "CouplingLedgerBreach"))?;
            let hexes: Vec<Option<Value>> = values.iter().map(|value| value.and_then(request_value_hex)).collect();
            match hexes.first() {
                Some(Some(first)) if hexes.iter().all(|hex| hex.as_ref() == Some(first)) => Some(first.clone()),
                _ => {
                    return Err(refused(
                        format!(
                            "the coupling '{}' hands on '{}', and the upstream request does not hold one value there in every segment",
                            coupling.name, mapping.from
                        ),
                        "CouplingSourceNotOneValue",
                    ));
                }
            }
        };
        let Some(value_hex) = value_hex else {
            return Err(refused(
                format!("the upstream answer carries no '{}' for the coupling '{}'", mapping.from, coupling.name),
                "CouplingSourceNotOneValue",
            ));
        };
        let values: Vec<f64> = match &value_hex {
            Value::Array(items) => items.iter().map(|item| value_of_hex(item).expect("a handed value is an exact number")).collect(),
            one => vec![value_of_hex(one).expect("a handed value is one exact number")],
        };
        let value = values[0];
        if let Some(sentence) = values.iter().find_map(|value| (mapping.outside)(*value, &so_far)) {
            return Err(refused(
                format!(
                    "the coupling '{}' hands '{}' on to '{}': {sentence}. That value is outside the downstream card's range, so the downstream card is not run",
                    coupling.name, mapping.from, mapping.to
                ),
                "CouplingOutsideRange",
            ));
        }
        let to = target_path(mapping.to, target);
        // `parameters.<name>[k..]`: the handed array stands at places k to
        // k + n - 1 of the request's array.
        let (path, first) = rows_from(&to);
        let received = values_at(down, path).map_err(|error| refused(error, "CouplingLedgerBreach"))?;
        let received: Vec<Option<Value>> = match first {
            None => received.into_iter().map(|given| given.cloned()).collect(),
            Some(first) => {
                let count = values.len();
                after_handed = Some(after_handed.map_or(first + count, |at: usize| at.max(first + count)));
                vec![received.first().copied().flatten().and_then(Value::as_array).map(|array| {
                    Value::Array(array.iter().skip(first).take(count).cloned().collect())
                })]
            }
        };
        for (at, given) in received.iter().enumerate() {
            let bits = given.as_ref().and_then(request_value_hex);
            if bits.as_ref() != Some(&value_hex) {
                let place = if received.len() > 1 { format!(" in segment {at}") } else { String::new() };
                if values.len() > 1 || value_hex.is_array() {
                    return Err(refused(
                        format!(
                            "the coupling '{}' hands {} values in {} from '{}' to '{}', and the request holds {}{place}; each value handed over and the value received must be one number, bit for bit",
                            coupling.name,
                            values.len(),
                            mapping.unit,
                            mapping.from,
                            to,
                            first_difference(&value_hex, bits.as_ref())
                        ),
                        "CouplingLedgerBreach",
                    ));
                }
                return Err(refused(
                    format!(
                        "the coupling '{}' hands {} {} ({}) from '{}' to '{}', and the request holds {}{place}; the value handed over and the value received must be one number, bit for bit",
                        coupling.name,
                        value,
                        mapping.unit,
                        hex_of_bits(value),
                        mapping.from,
                        to,
                        bits.map(|value| value.to_string()).unwrap_or_else(|| "nothing".to_string())
                    ),
                    "CouplingLedgerBreach",
                ));
            }
        }
        so_far.push((mapping.to, value));
        handed.push(Handed {
            from: mapping.from.to_string(),
            to,
            unit: Some(mapping.unit.to_string()),
            value_hex,
        });
    }
    // The places no frame was handed to, held to the coupling's row conditions.
    if let (Some(after), Some(parameters)) = (after_handed, down.get("parameters")) {
        if let Some(sentence) = coupling.rows_unmet(parameters, after) {
            return Err(refused(sentence, "CouplingConditionNotMet"));
        }
    }
    Ok(handed)
}

/// A downstream path and, for `<path>[k..]`, the first place k of the
/// handed values in the request's array.
fn rows_from(to: &str) -> (&str, Option<usize>) {
    if let Some(open) = to.strip_suffix("..]").and_then(|head| head.rfind('[').map(|at| (at, head))) {
        let (at, head) = open;
        if let Ok(first) = head[at + 1..].parse::<usize>() {
            return (&to[..at], Some(first));
        }
    }
    (to, None)
}

/// The exact values of one slot of every recorded frame of the upstream run,
/// in order: a 32-bit slot is widened to 64 bits, which is exact.
fn frame_values(up: &TrajectoryResponse, slot: &str) -> Option<Vec<f64>> {
    let row = genesis_kernel::frame::FRAME_LAYOUT.iter().find(|row| row.slot == slot)?;
    let mut values = Vec::with_capacity(up.frames.len());
    for frame in &up.frames {
        let bytes = hex::decode(&frame.frame_hex).ok()?;
        let at = bytes.get(row.offset..row.offset + row.width)?;
        let value = match row.kind {
            "f64" => f64::from_le_bytes(at.try_into().ok()?),
            "f32" => f64::from(f32::from_le_bytes(at.try_into().ok()?)),
            _ => return None,
        };
        values.push(value);
    }
    (!values.is_empty()).then_some(values)
}

/// What the request holds where an array was handed, for a sentence: its
/// length, or where it first differs.
fn first_difference(handed: &Value, received: Option<&Value>) -> String {
    match (handed, received) {
        (Value::Array(left), Some(Value::Array(right))) => match left.iter().zip(right).position(|(a, b)| a != b) {
            Some(at) => format!("another value at index {at} ({} handed, {} received)", left[at], right[at]),
            None => format!("{} values where {} were handed", right.len(), left.len()),
        },
        (_, Some(other)) => format!("{other}"),
        (_, None) => "nothing".to_string(),
    }
}
