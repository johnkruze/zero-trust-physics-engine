//! The table of couplings: one entry each. A coupling names one card whose
//! sealed run hands values on, one card that takes them, and for each value
//! where it is read in the upstream run, where it stands in the downstream
//! request, its unit, and the downstream card's declared range for it; and
//! the conditions on the two requests under which the values handed carry
//! one meaning in both cards.
//!
//! A coupling is served only where both pieces were compared with
//! measurement on the same object and the value handed is one both pieces
//! take with the same meaning. No physics is added here: a value is carried
//! bit for bit, never converted. A coupling's conditions are decided on the
//! two requests before any physics, and a link that does not meet one is
//! refused as `CouplingConditionNotMet`. A value outside the downstream
//! card's declared range is refused as `CouplingOutsideRange` before
//! anything of the downstream piece runs.
//!
//! A unit is the unit both cards' descriptions give the value, spelled as
//! they spell it, except for the spellings in [`UNIT_SPELLINGS`]: a unit is
//! in every chain seal its coupling gives, so a spelling once served stays.
//!
//! Served: one pulse of the JSI TRIGA Mark II reactor from the card
//! reactor_triga_jsi into the solver fuel_rod_triga_jsi, its instrumented
//! element 6753: the start temperature, and from every recorded frame of the
//! card's sealed run its time and the core energy released, each carried bit
//! for bit (the energy is the frame's 32-bit slot, widened to 64 bits, which
//! is exact) into the solve's history. The element's share of the core's
//! energy, 1.387/56, is the solver's own constant and is not applied here.
//! Both pieces were compared with the institute's pulse records. And the
//! Stardust capsule's entry from the card capsule_stardust into the solver
//! ablator_slab on a PICA model: from every recorded frame of the card's
//! sealed run its time and the stagnation-point environment the card
//! computes from its own air and speed (the stagnation pressure, the
//! recovery enthalpy u²/2 and the heat-transfer coefficient of its
//! Sutton-Graves heat flux), each carried bit for bit into rows 1 to n of
//! the slab's heating history. Row 0 and the rows after the handed ones are
//! the request's own, held by the coupling's conditions to carry no
//! heating. Both pieces were compared with the Stardust flight: the card
//! with NASA's reconstruction of the entry, the slab with the cores cut
//! from the recovered shield. The card computes no radiative heating, so
//! the history carries none.

use genesis_solvers::ablator_slab as slab;
use genesis_solvers::fuel_rod_triga_jsi as rod;
use serde_json::Value;

/// Where a handed value is read in the upstream run: `final_state.<name>`
/// (its sealed final value), `inputs.initial.<name>` /
/// `inputs.segments[*].load.<name>` (its sealed request, every segment
/// holding the same value), or `frames[*].<slot>` (that slot of every
/// recorded frame of the sealed run, in order: an array, into a solver).
pub(crate) struct Mapping {
    pub from: &'static str,
    /// Where it stands in a downstream run request: `initial.<name>` or
    /// `segments[*].load.<name>`. A step request takes it at
    /// `parameters.<name>`, and a solve request at `parameters.<name>` as
    /// written here, or, for an array from the frames, at
    /// `parameters.<name>[k..]`: the handed values stand at places k to
    /// k + n - 1 of that array, n the frames handed, and the places before
    /// and after them are the request's own, held by the coupling's
    /// [`RowCondition`]s.
    pub to: &'static str,
    pub unit: &'static str,
    /// `Some(sentence)` when the value is outside the downstream card's
    /// declared range, given the values already handed by this coupling. A
    /// frame slot's array is held to it value by value.
    pub outside: fn(f64, &[(&'static str, f64)]) -> Option<String>,
}

/// The two requests a link joins, as a coupling's conditions read them: the
/// upstream run_trajectory request and the downstream request, each without
/// its `from`, and whether the downstream piece is one step, whose start
/// fields stand in its `parameters`.
// The upstream request and the downstream piece's kind are read by the
// conditions of a coupling between two cards; the couplings served now go
// from a card into a solver.
#[allow(dead_code)]
pub(crate) struct Joined<'a> {
    pub upstream: &'a Value,
    pub downstream: &'a Value,
    pub step: bool,
}

impl Joined<'_> {

}

/// A condition a coupling holds on the two requests it joins, decided before
/// any physics: where it is not met, the values handed do not carry one
/// meaning in both cards.
pub(crate) struct Condition {
    /// What the condition holds, as `describe_card` states it.
    pub says: &'static str,
    /// `Some(what the requests hold instead)` when it is not met. A field it
    /// reads that a request leaves out does not decide it: that request is
    /// refused by name when it is read.
    pub unmet: fn(&Joined) -> Option<String>,
}

/// A condition a coupling into a solver holds on the places of a downstream
/// array that no frame was handed to, decided once the upstream run is
/// computed again and its values handed, before the solver runs: the
/// request alone does not say how many frames the upstream records.
pub(crate) struct RowCondition {
    /// What the condition holds, as `describe_solver` states it.
    pub says: &'static str,
    /// `Some(what the request holds instead)` when it is not met, given the
    /// downstream `parameters` and the first place after the handed ones.
    pub unmet: fn(&Value, usize) -> Option<String>,
}

/// The downstream piece of a coupling into a solver is named `("solver",
/// <name>)`.
pub(crate) const SOLVER: &str = "solver";

pub(crate) struct Coupling {
    pub name: &'static str,
    pub upstream: (&'static str, &'static str),
    pub downstream: (&'static str, &'static str),
    pub mappings: &'static [Mapping],
    /// The conditions on the two requests, each decided before any physics.
    pub conditions: &'static [Condition],
    /// The conditions on the places of a downstream array no frame was
    /// handed to, decided after the values are handed.
    pub row_conditions: &'static [RowCondition],
    /// What is handed and why the pairing is served, in one sentence.
    pub meaning: &'static str,
}

impl Coupling {
    /// The first condition the two requests do not meet, as the sentence of
    /// the refusal `CouplingConditionNotMet`; `None` when they meet every one.
    pub(crate) fn unmet(&self, joined: &Joined) -> Option<String> {
        self.conditions.iter().find_map(|condition| {
            (condition.unmet)(joined).map(|instead| {
                format!(
                    "the coupling '{}' joins two runs only where {}; {instead}, and outside its conditions the values it hands do not carry one meaning in both cards, so neither run is computed",
                    self.name, condition.says
                )
            })
        })
    }

    /// The first row condition the downstream request does not meet, given
    /// the first place after the handed ones, as the sentence of the refusal
    /// `CouplingConditionNotMet`.
    pub(crate) fn rows_unmet(&self, parameters: &Value, after: usize) -> Option<String> {
        self.row_conditions.iter().find_map(|condition| {
            (condition.unmet)(parameters, after).map(|instead| {
                format!(
                    "the coupling '{}' hands on only where {}; {instead}, and outside its conditions the values it hands do not carry one meaning in both pieces, so the solver is not run",
                    self.name, condition.says
                )
            })
        })
    }
}

/// The spellings of a unit that a served coupling carries and that the two
/// cards' descriptions spell otherwise, each with the descriptions'
/// spelling. A coupling's unit is in every chain seal it gives
/// (`handed[].unit`), so a spelling once served stays; a new coupling spells
/// its units as the descriptions do. The table is static, so its test,
/// `every_unit_is_the_unit_both_cards_give_the_value`, is its registration.
#[cfg_attr(not(test), allow(dead_code))]
pub(crate) const UNIT_SPELLINGS: &[(&str, &str)] = &[
    // The SPERT III couplings since engine 0.8.0; both cards' descriptions
    // give the reactivity in "dollar".
    ("dollars", "dollar"),
];

/// A coupling's unit as the cards' descriptions spell it.
#[cfg_attr(not(test), allow(dead_code))]
pub(crate) fn unit_as_described(unit: &str) -> &str {
    UNIT_SPELLINGS
        .iter()
        .find(|(served, _)| *served == unit)
        .map_or(unit, |(_, described)| *described)
}

fn outside_span(value: f64, min: f64, max: f64, unit: &str, what: &str) -> Option<String> {
    if value >= min && value <= max {
        None
    } else {
        Some(format!("{value} {unit} is outside {what}, {min} to {max} {unit}"))
    }
}

fn rod_start(value: f64, _: &[(&'static str, f64)]) -> Option<String> {
    if value > rod::POOL_FREEZING_K && value < rod::SATURATION_TEMPERATURE_K {
        None
    } else {
        Some(format!(
            "{value} K is not above {} K and below {} K, fuel_rod_triga_jsi's liquid pool",
            rod::POOL_FREEZING_K,
            rod::SATURATION_TEMPERATURE_K
        ))
    }
}

fn rod_time(value: f64, _: &[(&'static str, f64)]) -> Option<String> {
    if value > 0.0 && value <= rod::HISTORY_SPAN_MAX_S {
        None
    } else {
        Some(format!("{value} s is not after 0 s and within the {} s of one solve", rod::HISTORY_SPAN_MAX_S))
    }
}

fn rod_energy(value: f64, _: &[(&'static str, f64)]) -> Option<String> {
    (value < 0.0).then(|| format!("{value} MW*s is below zero; energy released does not fall"))
}

/// The PICA model's surface table, which every PICA version reads: the wall
/// pressures it takes.
fn slab_pressure(value: f64, _: &[(&'static str, f64)]) -> Option<String> {
    outside_span(
        value,
        slab::PICA_STREAM_PRESSURE_MIN_PA,
        slab::STREAM_PRESSURE_MAX_PA,
        "Pa",
        "the ends of ablator_slab's PICA surface table",
    )
}

fn slab_time(value: f64, _: &[(&'static str, f64)]) -> Option<String> {
    if value > 0.0 && value <= slab::HISTORY_DURATION_MAX_S {
        None
    } else {
        Some(format!("{value} s is not after 0 s and within the {} s of ablator_slab's heating history", slab::HISTORY_DURATION_MAX_S))
    }
}

fn slab_coefficient(value: f64, _: &[(&'static str, f64)]) -> Option<String> {
    (!(value >= 0.0)).then(|| format!("{value} kg/m²/s is below zero; ablator_slab takes a heat-transfer coefficient of zero or more"))
}

fn slab_enthalpy(value: f64, _: &[(&'static str, f64)]) -> Option<String> {
    (!(value >= 0.0)).then(|| format!("{value} J/kg is below zero; ablator_slab takes a flow enthalpy of zero or more"))
}

/// A word of the downstream solve's parameters.
fn parameter_word<'a>(parameters: &'a Value, name: &str) -> Option<&'a str> {
    parameters.get(name)?.as_str()
}

/// The numbers of a downstream solve's array, as far as they are numbers.
fn parameter_column(parameters: &Value, name: &str) -> Option<Vec<f64>> {
    parameters.get(name)?.as_array()?.iter().map(Value::as_f64).collect()
}

fn solve_parameters<'a>(joined: &'a Joined) -> Option<&'a Value> {
    joined.downstream.get("parameters")
}

fn slab_is_pica(joined: &Joined) -> Option<String> {
    let material = parameter_word(solve_parameters(joined)?, "material")?;
    (!material.starts_with(slab::MATERIAL_PICA)).then(|| format!("the solve states material '{material}'"))
}

fn slab_face_in_air(joined: &Joined) -> Option<String> {
    let parameters = solve_parameters(joined)?;
    let face = parameter_word(parameters, "face")?;
    if face != "gas_stream" {
        return Some(format!("the solve states face '{face}'"));
    }
    let gas = parameter_word(parameters, "stream_gas")?;
    (gas != "air").then(|| format!("the solve states stream_gas '{gas}'"))
}

fn slab_starts_unheated(joined: &Joined) -> Option<String> {
    let parameters = solve_parameters(joined)?;
    let time = parameter_column(parameters, "history_time_s")?;
    let coefficient = parameter_column(parameters, "history_heat_transfer_coefficient_kg_per_m2_s")?;
    match (time.first(), coefficient.first()) {
        (Some(&t), Some(&c)) if t == 0.0 && c == 0.0 => None,
        (Some(&t), Some(&c)) => Some(format!("the history's first row is at {t} s with a coefficient of {c} kg/m²/s")),
        _ => None,
    }
}

fn slab_no_radiation(joined: &Joined) -> Option<String> {
    let radiative = parameter_column(solve_parameters(joined)?, "history_radiative_heating_w_per_m2")?;
    radiative
        .iter()
        .enumerate()
        .find(|(_, value)| **value != 0.0)
        .map(|(row, value)| format!("row {row} of the history holds a radiative heating of {value} W/m²"))
}

fn slab_unheated_after(parameters: &Value, after: usize) -> Option<String> {
    let coefficient = parameter_column(parameters, "history_heat_transfer_coefficient_kg_per_m2_s")?;
    coefficient
        .iter()
        .enumerate()
        .skip(after)
        .find(|(_, value)| **value != 0.0)
        .map(|(row, value)| format!("row {row}, after the {} rows handed, holds a coefficient of {value} kg/m²/s", after - 1))
}

pub(crate) static COUPLINGS: &[Coupling] = &[
    Coupling {
        name: TRIGA_PULSE_TO_ELEMENT_6753,
        upstream: ("energy", "reactor_triga_jsi"),
        downstream: (SOLVER, "fuel_rod_triga_jsi"),
        mappings: &[
            Mapping { from: "inputs.initial.start_temperature_k", to: "parameters.start_temperature_k", unit: "K", outside: rod_start },
            Mapping { from: "frames[*].t", to: "parameters.time_s", unit: "s", outside: rod_time },
            Mapping { from: "frames[*].pos[1]", to: "parameters.core_energy_released_mws", unit: "MW*s", outside: rod_energy },
        ],
        conditions: &[],
        row_conditions: &[],
        meaning: TRIGA_MEANING,
    },
    Coupling {
        name: STARDUST_ENTRY_TO_PICA_SLAB,
        upstream: ("aerospace", "capsule_stardust"),
        downstream: (SOLVER, "ablator_slab"),
        mappings: &[
            Mapping { from: "frames[*].t", to: "parameters.history_time_s[1..]", unit: "s", outside: slab_time },
            Mapping { from: "frames[*].pos[1]", to: "parameters.history_pressure_pa[1..]", unit: "Pa", outside: slab_pressure },
            Mapping {
                from: "frames[*].vel[1]",
                to: "parameters.history_heat_transfer_coefficient_kg_per_m2_s[1..]",
                unit: "kg/m²/s",
                outside: slab_coefficient,
            },
            Mapping { from: "frames[*].vel[0]", to: "parameters.history_flow_enthalpy_j_per_kg[1..]", unit: "J/kg", outside: slab_enthalpy },
        ],
        conditions: &[
            Condition {
                says: "the slab is a PICA model, a material that begins 'PICA, open record'",
                unmet: slab_is_pica,
            },
            Condition {
                says: "the face sits in a gas stream of air, face 'gas_stream' and stream_gas 'air', the air the capsule flew in",
                unmet: slab_face_in_air,
            },
            Condition {
                says: "the history's first row, before the first frame handed, is at 0 s with a heat-transfer coefficient of 0",
                unmet: slab_starts_unheated,
            },
            Condition {
                says: "every row of the history holds a radiative heating of 0, since the card computes none",
                unmet: slab_no_radiation,
            },
        ],
        row_conditions: &[RowCondition {
            says: "every row of the history after the rows handed holds a heat-transfer coefficient of 0: the card's entry ends there, and the shield cools with the stream off",
            unmet: slab_unheated_after,
        }],
        meaning: STARDUST_MEANING,
    },
];

/// The name of the coupling of the Stardust entry into the PICA slab.
pub(crate) const STARDUST_ENTRY_TO_PICA_SLAB: &str = "stardust_entry_to_pica_slab";

const STARDUST_MEANING: &str = "The Stardust entry of 15 January 2006 handed from capsule_stardust to ablator_slab on a PICA model under one chain proof: each recorded frame's time, stagnation pressure, heat-transfer coefficient and recovery enthalpy (32-bit slots widened exactly), bit for bit into rows 1 to n of the history. The card computes them at the nose from its own air and speed: Rayleigh's pitot pressure for gamma 1.4, u²/2, and the coefficient of its Sutton-Graves heat flux, which times u²/2 is that heat flux. Row 0 and the rows after the handed ones are the request's own and carry no heating; the card computes no radiation. Use record_every 30 or more at dt_s 0.05: at 1 s the pressure is 0.99 Pa, below the PICA table's 1.01325. It stands for the near-nose core, not the flank. Against the recovered shield: wear 4.65 mm against 5.7 (6.1 by X-ray); core density within 10.5 kg/m³ RMS over 28 slices; face temperature within 156 K RMS of the airborne spectra over 22 times, against 376 K with NASA's bare-wall heating. No temperature inside the shield was measured in flight. Worked: from entry interface, dt_s 0.05, 2779 steps, record_every 30, 93 frames in a 96-row history, PICA version 5, 0.0582 m from 260 K, thermocouples at 5, 10, 20 and 58.2 mm asked, radiating to 266.48 K, stream off at 139.95 s, cooling to 600 s: wear 4.648 mm, chain_seal 21a8ace2dc1223ed33f32f97173269980e469ca11d23bed4ccbcf41544fee9e55a3228621dd12a52452ffec09ddf80b1.";

/// The name of the coupling of the TRIGA card into the element solver.
pub(crate) const TRIGA_PULSE_TO_ELEMENT_6753: &str = "triga_pulse_to_element_6753";

const TRIGA_MEANING: &str = "One pulse of the JSI TRIGA Mark II reactor handed from the card reactor_triga_jsi to the solver of its instrumented fuel element 6753, under one chain proof: the start temperature, and from every recorded frame of the card's sealed run its time since the fire signal (s) and the core energy released (energy_mws, MW*s, the frame's 32-bit slot widened exactly to 64 bits), each carried bit for bit into the solve's start_temperature_k, time_s and core_energy_released_mws. The card's energy released and the solver's input are one quantity, the core's thermal energy since the fire signal, the integral of the card's power; energy is handed and not the frames' power, because a frame samples the power at one step and the pulse is narrower than the frames, while the energy at a frame is the card's integral of all of it. The solver takes 1.387/56 of it for the element, its own constant; the coupling converts nothing. Record the card's run with frames 10 ms apart (dt_s 5e-5 and record_every 200), the history the element was compared on. On 16 pulses of cores 217 and 231 left out of the element's fit, chained this way: the thermocouple's maximum rise 9.8 to 4.3 percent low, its time after the power peak 19.9 to 4.2 percent early, its rise 10 s after the peak 11.2 to 5.0 percent low (7 of 8 within 10 percent); the card's peak power was 149 to 525 MW against the records' 158 to 558. Worked: reactor_triga_jsi from 296.15 K on core 231's rod table at 2.5 dollars, rods dropped after 60,000 steps of 5e-5 s, 279,000 steps, record_every 200, handed as 1,395 points: peak fuel temperature 598.8 K, chain_seal 3ee0bd22021ae709fec8afa93ea1a709a1e41387c5d1ee1e4d1a7b77edb29effb27d40a9236b1e029df0a01fb21fe527.";

/// Whether the view holds a piece a coupling names: a card, or a solver.
pub(crate) fn sees(view: &crate::view::View, piece: (&str, &str)) -> bool {
    if piece.0 == SOLVER {
        view.sees_solver(piece.1) && crate::solvers::find(piece.1).is_some()
    } else {
        view.find(piece.0, piece.1).is_some()
    }
}

/// The couplings into a solver, as `describe_solver` states them.
pub(crate) fn table_into_solver(view: &crate::view::View, solver: &str) -> Value {
    table_in(view, SOLVER, solver)
}

pub(crate) fn find(name: &str) -> Option<&'static Coupling> {
    COUPLINGS.iter().find(|coupling| coupling.name == name)
}

/// The couplings a card hands from or takes, as `describe_card` states
/// them under `couplings`: only those whose two cards the view holds, so a
/// card outside the view is not named.
pub(crate) fn table_in(view: &crate::view::View, domain: &str, card: &str) -> Value {
    Value::Array(
        COUPLINGS
            .iter()
            .filter(|coupling| coupling.upstream == (domain, card) || coupling.downstream == (domain, card))
            .filter(|coupling| sees(view, coupling.upstream) && sees(view, coupling.downstream))
            // A coupling into a solver is listed by describe_solver alone, so a
            // card's description is the same in every view that holds the card.
            .filter(|coupling| coupling.downstream.0 != SOLVER || domain == SOLVER)
            .map(|coupling| {
                let mut entry = serde_json::Map::new();
                entry.insert("name".into(), coupling.name.into());
                entry.insert("upstream".into(), format!("{}/{}", coupling.upstream.0, coupling.upstream.1).into());
                entry.insert("downstream".into(), format!("{}/{}", coupling.downstream.0, coupling.downstream.1).into());
                entry.insert(
                    "handed".into(),
                    coupling
                        .mappings
                        .iter()
                        .map(|mapping| serde_json::json!({ "from": mapping.from, "to": mapping.to, "unit": mapping.unit }))
                        .collect::<Vec<_>>()
                        .into(),
                );
                // A coupling with no condition is listed as it was before conditions were served.
                if !coupling.conditions.is_empty() || !coupling.row_conditions.is_empty() {
                    entry.insert(
                        "conditions".into(),
                        coupling
                            .conditions
                            .iter()
                            .map(|condition| Value::from(condition.says))
                            .chain(coupling.row_conditions.iter().map(|condition| Value::from(condition.says)))
                            .collect::<Vec<_>>()
                            .into(),
                    );
                }
                entry.insert("meaning".into(), coupling.meaning.into());
                Value::Object(entry)
            })
            .collect(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::descriptor::Role;

    /// The field of a card's description with this name, and its role.
    fn field<'a>(card: &'a crate::descriptor::Card, name: &str) -> &'a crate::descriptor::FieldSpec {
        card.fields.iter().find(|field| field.name == name).unwrap_or_else(|| panic!("{}: no field {name}", card.card))
    }

    /// Every coupling names two cards the engine serves; every source names
    /// a value the upstream card gives (a number of its published state, a
    /// start field or a load of its request) and every place a field of the
    /// downstream card of the kind its path declares (a start field at
    /// `initial.`, a load at `segments[*].load.`, and either among a step's
    /// parameters).
    #[test]
    fn every_coupling_names_cards_the_engine_serves_and_fields_they_take() {
        for coupling in COUPLINGS {
            let up = crate::descriptor::find(coupling.upstream.0, coupling.upstream.1).expect("upstream card");
            let published = crate::state_fields(up.domain, up.card).expect("the upstream publishes a state");
            if coupling.downstream.0 == SOLVER {
                let solver = crate::solvers::find(coupling.downstream.1).expect("downstream solver");
                for mapping in coupling.mappings {
                    if let Some(slot) = mapping.from.strip_prefix("frames[*].") {
                        assert!(
                            genesis_kernel::frame::FRAME_LAYOUT.iter().any(|row| row.slot == slot && (row.kind == "f64" || row.kind == "f32")),
                            "{}",
                            mapping.from
                        );
                    } else {
                        let name = mapping.from.strip_prefix("inputs.initial.").expect("a start field");
                        assert!(up.phase_names.contains(&name), "{}", mapping.from);
                    }
                    let name = mapping.to.strip_prefix("parameters.").expect("a solve takes parameters");
                    let name = name.split_once('[').map_or(name, |(name, _)| name);
                    assert!(solver.inputs.iter().any(|input| input.name == name), "{}", mapping.to);
                }
                assert_eq!(COUPLINGS.iter().filter(|other| other.name == coupling.name).count(), 1);
                continue;
            }
            let down = crate::descriptor::find(coupling.downstream.0, coupling.downstream.1).expect("downstream card");
            for mapping in coupling.mappings {
                if let Some(name) = mapping.from.strip_prefix("inputs.initial.") {
                    assert!(up.phase_names.contains(&name), "{}", mapping.from);
                    assert!(field(up, name).role == Role::Phase, "{}", mapping.from);
                } else if let Some(name) = mapping.from.strip_prefix("inputs.segments[*].load.") {
                    assert!(up.load_names.contains(&name), "{}", mapping.from);
                    assert!(field(up, name).role == Role::Load, "{}", mapping.from);
                } else {
                    let name = mapping.from.strip_prefix("final_state.").unwrap_or_else(|| panic!("{}", mapping.from));
                    assert!(
                        published.iter().any(|(field, kind)| field == name && *kind == crate::QuantityKind::Number),
                        "{}: {} publishes no number {name}",
                        coupling.name,
                        up.card
                    );
                }
                let name = if let Some(name) = mapping.to.strip_prefix("initial.") {
                    assert!(down.phase_names.contains(&name), "{}", mapping.to);
                    assert!(field(down, name).role == Role::Phase, "{}", mapping.to);
                    name
                } else {
                    let name = mapping.to.strip_prefix("segments[*].load.").expect("a load path");
                    assert!(down.load_names.contains(&name), "{}", mapping.to);
                    assert!(field(down, name).role == Role::Load, "{}", mapping.to);
                    name
                };
                // A step takes the value among its parameters.
                assert!(down.parameter_names.contains(&name), "{} parameters.{name}", coupling.name);
                // One number is handed into one number.
                assert_eq!(field(down, name).components, 1, "{}", mapping.to);
            }
            assert_eq!(COUPLINGS.iter().filter(|other| other.name == coupling.name).count(), 1);
        }
        let names: Vec<&str> = COUPLINGS.iter().map(|coupling| coupling.name).collect();
        assert_eq!(
            names,
            [
                "triga_pulse_to_element_6753",
                "stardust_entry_to_pica_slab"
            ]
        );
    }

    /// The unit of a value the upstream card gives: the unit its description
    /// gives a start field or a load, or the unit of a published number (a
    /// number published under a start field's name has that field's unit).
    fn source_unit(card: &crate::descriptor::Card, from: &str) -> &'static str {
        let name = from
            .strip_prefix("inputs.initial.")
            .or_else(|| from.strip_prefix("inputs.segments[*].load."))
            .or_else(|| from.strip_prefix("final_state."))
            .unwrap_or_else(|| panic!("{from}"));
        match card.fields.iter().find(|field| field.name == name) {
            Some(field) => field.unit,
            None => crate::descriptor::unit_of_known(name).unwrap_or_else(|| panic!("{}: no unit for {name}", card.card)),
        }
    }

    /// Each coupling's unit is the unit both cards' descriptions give the
    /// value, as they spell it, or a spelling in UNIT_SPELLINGS. A spelling
    /// once served stays there: since 0.15.0 no served coupling carries
    /// "dollars" (the SPERT III couplings are removed).
    #[test]
    fn every_unit_is_the_unit_both_cards_give_the_value() {
        for coupling in COUPLINGS.iter().filter(|coupling| coupling.downstream.0 != SOLVER) {
            let up = crate::descriptor::find(coupling.upstream.0, coupling.upstream.1).unwrap();
            let down = crate::descriptor::find(coupling.downstream.0, coupling.downstream.1).unwrap();
            for mapping in coupling.mappings {
                let to = mapping.to.strip_prefix("initial.").or_else(|| mapping.to.strip_prefix("segments[*].load.")).unwrap();
                let described = unit_as_described(mapping.unit);
                assert_eq!(described, source_unit(up, mapping.from), "{} {}", coupling.name, mapping.from);
                assert_eq!(described, field(down, to).unit, "{} {}", coupling.name, mapping.to);
            }
        }
        // No other spelling is a descriptions' own.
        for (served, described) in UNIT_SPELLINGS {
            assert_ne!(served, described);
        }
        assert_eq!(unit_as_described("dollars"), "dollar");
        assert_eq!(unit_as_described("Ah"), "Ah");
    }

    fn joined<'a>(upstream: &'a Value, downstream: &'a Value, step: bool) -> Joined<'a> {
        Joined { upstream, downstream, step }
    }

    /// The Stardust coupling's conditions on the slab's question: a PICA
    /// model in air, unheated before the first frame, no radiation, and no
    /// heating after the last frame handed.
    #[test]
    fn the_stardust_conditions_hold_the_slab_s_question() {
        let coupling = find(STARDUST_ENTRY_TO_PICA_SLAB).unwrap();
        let up = serde_json::json!({ "domain": "aerospace", "card": "capsule_stardust" });
        let solve = |change: &dyn Fn(&mut Value)| {
            let mut parameters = serde_json::json!({
                "material": "PICA, open record, version 5", "face": "gas_stream", "stream_gas": "air",
                "history_time_s": [0.0, 1.5, 3.0, 4.0, 600.0],
                "history_heat_transfer_coefficient_kg_per_m2_s": [0.0, 0.001, 0.002, 0.0, 0.0],
                "history_radiative_heating_w_per_m2": [0.0, 0.0, 0.0, 0.0, 0.0]
            });
            change(&mut parameters);
            serde_json::json!({ "solver": "ablator_slab", "parameters": parameters })
        };
        let named = |down: &Value| coupling.unmet(&joined(&up, down, true));
        assert_eq!(named(&solve(&|_| {})), None);
        for material in slab::MATERIAL_NAMES {
            let down = solve(&|p| p["material"] = serde_json::json!(material));
            assert_eq!(named(&down).is_none(), material.starts_with("PICA"), "{material}");
        }
        let cases: [(&dyn Fn(&mut Value), &str); 5] = [
            (&|p| p["material"] = serde_json::json!("TACOT 3.0"), "the solve states material 'TACOT 3.0'"),
            (&|p| p["face"] = serde_json::json!("held_temperature"), "the solve states face 'held_temperature'"),
            (&|p| p["stream_gas"] = serde_json::json!("mixture"), "the solve states stream_gas 'mixture'"),
            (&|p| p["history_time_s"][0] = serde_json::json!(0.5), "the history's first row is at 0.5 s"),
            (&|p| p["history_radiative_heating_w_per_m2"][2] = serde_json::json!(5.0), "row 2 of the history holds a radiative heating of 5 W/m²"),
        ];
        for (change, instead) in cases {
            let sentence = named(&solve(change)).unwrap_or_else(|| panic!("{instead}"));
            assert!(sentence.contains(instead), "{sentence}");
        }
        // After two rows handed (rows 1 and 2), rows 3 and 4 carry no heating.
        let parameters = solve(&|_| {})["parameters"].clone();
        assert_eq!(coupling.rows_unmet(&parameters, 3), None);
        let sentence = coupling.rows_unmet(&parameters, 2).unwrap();
        assert!(sentence.contains("row 2, after the 1 rows handed, holds a coefficient of 0.002"), "{sentence}");
        // A field left out decides nothing.
        let mut bare = solve(&|_| {});
        bare["parameters"].as_object_mut().unwrap().shift_remove("material");
        assert_eq!(named(&bare), None);
        // What is handed is in range from the card's own entry, and the slab's ends are named.
        assert!(slab_pressure(slab::PICA_STREAM_PRESSURE_MIN_PA, &[]).is_none());
        assert!(slab_pressure(0.99, &[]).unwrap().contains("1.01325 to 101325 Pa"));
        assert!(slab_time(0.0, &[]).is_some() && slab_time(1.5, &[]).is_none());
        assert!(slab_coefficient(-1e-9, &[]).is_some() && slab_enthalpy(-1.0, &[]).is_some());
    }

}
