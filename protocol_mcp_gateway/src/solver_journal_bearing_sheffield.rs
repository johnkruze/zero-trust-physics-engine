//! The entry for `journal_bearing_sheffield`: the oil film of the University
//! of Sheffield journal bearing test platform. The numbers come from
//! `genesis_solvers` constants, the sentence of each refusal from the
//! solver's source, and the validation record from the research record of
//! the bearing.

use genesis_solvers::journal_bearing_sheffield::{self as bearing, BearingFilm, BearingReject};
use serde_json::{Value, json};

use crate::solvers::{
    Answer, Given, Input, Kind, Output, Published, Refusal, Refused, Sections, Shape, Solved,
    Solver,
};

const fn number(name: &'static str, unit: &'static str, note: &'static str) -> Output {
    Output {
        name,
        unit,
        shape: Shape::Number,
        each: &[],
        note,
        asked_by: None,
    }
}

pub const JOURNAL_BEARING_SHEFFIELD: Solver = Solver {
    name: "journal_bearing_sheffield",
    summary: "Oil film of the University of Sheffield journal bearing test platform at 65 °C, load 2 to 20 kN, speed 100 to 800 rpm: Sommerfeld number, eccentricity ratio and minimum film thickness. One constant, a viscosity multiplier, is fitted to measurements of this bearing.",
    inputs: &[
        Input {
            name: "load_n",
            kind: Kind::Number {
                minimum: bearing::LOAD_MIN_N,
                maximum: bearing::LOAD_MAX_N,
                unit: "N",
            },
            refusal: Some("LoadOutOfRange"),
            bound: "The steady load on the bearing. The range is the tested envelope of the specimen.",
            only_when: None,
            asked_by_check: false,
        },
        Input {
            name: "speed_rpm",
            kind: Kind::Number {
                minimum: bearing::SPEED_MIN_RPM,
                maximum: bearing::SPEED_MAX_RPM,
                unit: "rpm",
            },
            refusal: Some("SpeedOutOfRange"),
            bound: "The steady shaft speed. No other input is accepted: the bearing, the oil, the grid and the tolerances are constants of the solver.",
            only_when: None,
            asked_by_check: false,
        },
    ],
    kept_inside: "The bearing, the oil, the grid and the tolerances stay inside the solver.",
    result: &[
        number(
            "sommerfeld_number",
            "1",
            "(R/c)^2 mu N L D / W, with N in revolutions per second and the film's viscosity.",
        ),
        number(
            "eccentricity_ratio",
            "1",
            "Shaft offset over the radial clearance.",
        ),
        number("minimum_film_thickness_m", "m", ""),
    ],
    invariants: &[
        number("flow_balance_residual", "fraction of the peak pressure", ""),
        number("load_residual_n", "N", ""),
    ],
    iterations: "Shaft positions tried. On a refusal it is the count for `LoadNotCarried` and null otherwise.",
    field: field_words,
    answer: "a film",
    refusals: REFUSALS,
    sections,
    validation: Some(validation),
    example,
    further_examples: &[],
    parts: &[],
    check: nothing_more_to_check,
    solve,
};

/// Each `BearingReject` variant with the sentence from its doc comment. The
/// request itself shows a load or a speed outside the range, so those two
/// refusals are not sealed.
const REFUSALS: &[Refusal] = &[
    Refusal {
        name: "LoadOutOfRange",
        meaning: "`load_n` is outside `LOAD_MIN_N`..=`LOAD_MAX_N`.",
        sealed: false,
    },
    Refusal {
        name: "SpeedOutOfRange",
        meaning: "`speed_rpm` is outside `SPEED_MIN_RPM`..=`SPEED_MAX_RPM`.",
        sealed: false,
    },
    Refusal {
        name: "NonFiniteValue",
        meaning: "An input or an intermediate value was NaN or infinite.",
        sealed: true,
    },
    Refusal {
        name: "EccentricityRatioOutOfRange",
        meaning: "The film carries the load only at an eccentricity ratio outside `ECCENTRICITY_RATIO_MIN`..=`ECCENTRICITY_RATIO_MAX`.",
        sealed: true,
    },
    Refusal {
        name: "FilmNotSettled",
        meaning: "The sweep cap was reached with the pressure still moving.",
        sealed: true,
    },
    Refusal {
        name: "FlowBalanceBroken",
        meaning: "A cell of an otherwise settled film left `FLOW_BALANCE_TOLERANCE`.",
        sealed: true,
    },
    Refusal {
        name: "LoadNotCarried",
        meaning: "The cap on shaft positions was reached with the film force still off the load.",
        sealed: true,
    },
];

/// The `BearingReject` variant name, without its fields.
pub fn reject_name(reject: &BearingReject) -> &'static str {
    match reject {
        BearingReject::LoadOutOfRange { .. } => "LoadOutOfRange",
        BearingReject::SpeedOutOfRange { .. } => "SpeedOutOfRange",
        BearingReject::NonFiniteValue => "NonFiniteValue",
        BearingReject::EccentricityRatioOutOfRange { .. } => "EccentricityRatioOutOfRange",
        BearingReject::FilmNotSettled { .. } => "FilmNotSettled",
        BearingReject::FlowBalanceBroken { .. } => "FlowBalanceBroken",
        BearingReject::LoadNotCarried { .. } => "LoadNotCarried",
    }
}

fn field_words() -> String {
    format!(
        "SHA-256 of the sample count ({}) as 8 little-endian bytes, then the settled film pressure above ambient in pascals as little-endian f64 bytes: {} rows around the bush from the widest gap in the direction of rotation, each of {} samples from the middle of the bush toward one end.",
        bearing::CELLS_AROUND * bearing::CELLS_ALONG_HALF,
        bearing::CELLS_AROUND,
        bearing::CELLS_ALONG_HALF
    )
}

fn sections() -> Sections {
    Sections {
        locked: json!({
            "model": "The Reynolds equation for an oil of one viscosity between a rigid aligned shaft and a rigid bush, film thickness c (1 + e cos(theta)), ambient pressure at both ends, rupture where pressure and its gradient reach zero together. The shaft is moved until the film carries the load. Left out: the oil inlet and its supply pressure, elastic and thermal deformation, the change of viscosity with pressure and along the film, roughness, misalignment, any motion of the shaft centre.",
            "bearing_diameter_m": bearing::BEARING_DIAMETER_M,
            "bearing_length_m": bearing::BEARING_LENGTH_M,
            "radial_clearance_m": bearing::RADIAL_CLEARANCE_M,
            "oil_viscosity_at_65c_pa_s": bearing::OIL_VISCOSITY_AT_65C_PA_S,
            "film_viscosity_multiplier": bearing::FILM_VISCOSITY_MULTIPLIER,
            "film_viscosity_multiplier_source": "Fitted to the thirty-four opened runs of the validation protocol. Every other constant is the source's.",
            "eccentricity_ratio_min": bearing::ECCENTRICITY_RATIO_MIN,
            "eccentricity_ratio_max": bearing::ECCENTRICITY_RATIO_MAX,
            "cells_around": bearing::CELLS_AROUND,
            "cells_along_half": bearing::CELLS_ALONG_HALF,
            "relaxation_weight": bearing::RELAXATION_WEIGHT,
            "max_sweeps": bearing::MAX_SWEEPS,
            "max_shaft_positions": bearing::MAX_SHAFT_POSITIONS,
        }),
        invariants: json!([
            {
                "name": "flow_balance_residual",
                "rule": "Oil is conserved in every cell: a cell under pressure balances the flow through its four faces, and a cell at ambient is one that balance would put at or below ambient.",
                "tolerance": bearing::FLOW_BALANCE_TOLERANCE,
                "unit": "fraction of the peak pressure",
                "checked": "at every shaft position",
                "refusal": "FlowBalanceBroken",
            },
            {
                "name": "load_residual_n",
                "rule": "The film force equals the load.",
                "tolerance": bearing::LOAD_TOLERANCE,
                "unit": "fraction of the load",
                "checked": "at acceptance",
                "refusal": "LoadNotCarried",
            },
        ]),
        convergence: json!({
            "settle_tolerance": bearing::SETTLE_TOLERANCE,
            "load_tolerance": bearing::LOAD_TOLERANCE,
            "rule": "At each shaft position no cell may move by more than the settle tolerance times the peak pressure in one sweep, and the film force must equal the load to the load tolerance, before a result is returned.",
            "refusal": "FilmNotSettled",
        }),
        reference: json!({
            "name": bearing::REFERENCE_NAME,
            "url": bearing::REFERENCE_URL,
            "retrieved": bearing::REFERENCE_RETRIEVED,
            "tolerance": bearing::REFERENCE_TOLERANCE,
            "test": "raimondi_boyd_full_bearing_at_three_length_ratios",
            "covers": "The Sommerfeld number at eccentricity ratios 0.1 to 0.9 for length-to-diameter ratios 1, 1/2 and 1/4, with the film started at the widest gap as the table's authors did.",
            "kind": "A reference calculation. It is not a measurement.",
        }),
    }
}

/// The validation record, in the five parts a card's description gives: the
/// object, the data, the pass line, the result and the limits. The numbers
/// are those of the research record of the bearing.
fn validation() -> [String; 5] {
    let object = "The plain full journal bearing of the bespoke test platform of the Leonardo Centre for Tribology, University of Sheffield, flooded with mineral oil, bearing temperature 65 °C, shaft aligned, steady load and speed. Beamish, Li, Brunskill, Hunter and Dwyer-Joyce, Tribology International 148 (2020) 106295.".to_string();
    let data = "Minimum film thickness by the ultrasonic phase-change method, one value per steady run, fifty runs: five speeds from 100 to 800 rpm at ten loads from 2 to 20 kN. Mendeley Data, doi 10.17632/txdrvd382p.2, CC BY 4.0. Thirty-four runs were opened and the viscosity multiplier fitted to them; sixteen were held back by rank before any measurement was read. Record: evidence/research/tribology_sheffield_journal_bearing/.".to_string();
    let pass_line = "On the residual (solver - measured) / measured over the held-back runs: mean of the absolute residual at most 0.15, worst absolute residual at most 0.35, absolute value of the mean at most 0.10. Fixed 2026-10-09T20:11:00Z, SHA-256 5319a2f190dece027bd0f19cb339b9d950827a2e7dcde7241b251380a21de01a of analysis/prereg.json, before any measured film thickness was read.".to_string();
    let result = "All three lines met on the sixteen held-back runs: mean absolute 0.0580, worst absolute 0.1563, mean 0.0199, computed by this solver through the gateway on 2026-10-09, once, each answer saved with its seal and its receipt. With no fitted constant the solver misses: on the sixteen held-back runs mean absolute 0.1409, worst absolute 0.2574, mean 0.1409, the film thicker than measured on all sixteen (by the second scoring, outside the engine); on the thirty-four opened runs mean absolute 0.1293, worst absolute 0.4761, mean 0.1277, not met. On the opened runs with the fitted multiplier, leaving one out, in plain Python outside the engine: mean absolute 0.0636, worst absolute 0.3339, mean 0.0085, met; through the engine: mean absolute 0.0617, worst absolute 0.3235, mean 0.0085, met, and the fitted multiplier came from these runs. A second scorer recomputed the sixteen residuals, the seals and the receipts from the protocol alone on 2026-10-09 and found the same numbers and the same verdict: evidence/checks/second_party/V-22/. Not gated: against the ultrasonic amplitude method on the same sixteen runs, mean absolute 0.2106, worst absolute 0.9356, mean -0.0648; the source restricts that method to films under 12.5 micrometres.".to_string();
    let limits = "One constant is fitted, the viscosity multiplier. The held-back runs are every third run of the same fifty and lie among the fitted ones, inside the same loads and speeds: interpolation, not extrapolation. The model depends on load and speed only through their ratio, and eight of the sixteen share that ratio with a fitted run. That is a property of the model, not a measurement: a load or speed outside the range is refused, and a request inside the range with the same ratio is not an answer for it. The test is whether the measurements fall on one curve with one constant. Nothing is shown outside 2 to 20 kN and 100 to 800 rpm. The record says nothing of any other bearing, oil or temperature; a misaligned shaft; start-up, shut-down or a moving load; the film away from its thinnest point; the pressure; the attitude angle; friction and oil flow. The solver returns no attitude angle, no pressure, no friction and no oil flow. The settled pressure field enters the proof only through field_sha256, and no pressure was compared with measurement.".to_string();
    [object, data, pass_line, result, limits]
}

fn example() -> Value {
    json!({ "load_n": 10000, "speed_rpm": 400 })
}

/// The load and the speed are the only inputs, and their bounds are in the
/// field list.
fn nothing_more_to_check(_given: &Given) -> Result<(), String> {
    Ok(())
}

fn solve(given: &Given) -> Result<Solved, String> {
    let load_n = given.number("load_n")?;
    let speed_rpm = given.number("speed_rpm")?;
    Ok(solved(bearing::solve(load_n, speed_rpm)))
}

/// What the solver returned, in the words every solver shares.
pub fn solved(returned: Result<BearingFilm, BearingReject>) -> Solved {
    let film = match returned {
        Ok(film) => film,
        Err(reject) => {
            return Solved::Refused(Refused {
                name: reject_name(&reject),
                sentence: reject.to_string(),
                iterations: match reject {
                    BearingReject::LoadNotCarried { shaft_positions } => {
                        Some(u64::from(shaft_positions))
                    }
                    _ => None,
                },
            });
        }
    };
    let named = |name: &str, value: f64| (name.to_string(), Published::Number(value));
    Solved::Answer(Answer {
        iterations: u64::from(film.shaft_positions),
        result: vec![
            named("sommerfeld_number", film.sommerfeld_number),
            named("eccentricity_ratio", film.eccentricity_ratio),
            named("minimum_film_thickness_m", film.minimum_film_thickness_m),
        ],
        invariants: vec![
            named("flow_balance_residual", film.flow_balance_residual),
            named("load_residual_n", film.load_residual_n),
        ],
        field: film.film_pressure_pa,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::solvers::{ALL, describe_value, solve_schema_in};
    use crate::view::View;

    const BEARING_SOURCE: &str =
        include_str!("../../genesis_solvers/src/journal_bearing_sheffield.rs");

    /// `(variant, doc sentence)` for every variant of `BearingReject`, read
    /// from the solver's source. An intra-doc link is written as its bare
    /// code span.
    fn documented_rejects() -> Vec<(String, String)> {
        let start = BEARING_SOURCE
            .find("pub enum BearingReject {")
            .expect("the enum is in the solver");
        let body = &BEARING_SOURCE[start..];
        let mut found = Vec::new();
        let mut doc = String::new();
        for line in body.lines().skip(1) {
            if line == "}" {
                break;
            }
            let Some(text) = line.strip_prefix("    ") else {
                continue;
            };
            if let Some(sentence) = text.strip_prefix("/// ") {
                if !doc.is_empty() {
                    doc.push(' ');
                }
                doc.push_str(sentence);
            } else if text.starts_with(|first: char| first.is_ascii_uppercase()) {
                let name: String = text
                    .chars()
                    .take_while(|letter| letter.is_ascii_alphanumeric())
                    .collect();
                found.push((name, doc.replace("[`", "`").replace("`]", "`")));
                doc.clear();
            }
        }
        found
    }

    #[test]
    fn every_bearing_refusal_carries_the_sentence_from_its_doc_comment() {
        let documented = documented_rejects();
        assert_eq!(documented.len(), REFUSALS.len());
        for ((variant, sentence), refusal) in documented.iter().zip(REFUSALS) {
            assert_eq!(variant, refusal.name);
            assert_eq!(sentence, refusal.meaning, "{}", refusal.name);
        }
        let body = describe_value(JOURNAL_BEARING_SHEFFIELD.name).expect("described");
        let refusals = body["refusals"].as_array().expect("refusals");
        assert_eq!(refusals.len(), documented.len());
        // Only the two refusals the request itself shows are answered with no seal.
        for refusal in refusals {
            let at_the_door =
                refusal["name"] == "LoadOutOfRange" || refusal["name"] == "SpeedOutOfRange";
            let reported = if at_the_door { "422, no seal" } else { "200, outcome refused, sealed" };
            assert_eq!(refusal["reported_as"], reported, "{}", refusal["name"]);
        }
    }

    #[test]
    fn the_bearing_reference_is_the_one_stored_in_the_solver_test() {
        let body = describe_value(JOURNAL_BEARING_SHEFFIELD.name).expect("described");
        let url = body["reference"]["url"].as_str().expect("url");
        let retrieved = body["reference"]["retrieved"].as_str().expect("retrieved");
        assert!(BEARING_SOURCE.contains(&format!("<{url}>")), "{url}");
        assert!(BEARING_SOURCE.contains(&format!("read {retrieved}")), "{retrieved}");
        let test = body["reference"]["test"].as_str().expect("test");
        assert!(BEARING_SOURCE.contains(&format!("fn {test}()")), "{test}");
    }

    #[test]
    fn each_solver_publishes_its_own_names_and_no_other() {
        let journal = &JOURNAL_BEARING_SHEFFIELD;
        let atom = &crate::solver_atom_dft::ATOM_DFT;
        assert!(journal.published_shape("result", "eccentricity_ratio_hex").is_some());
        assert!(journal.published_shape("invariants", "load_residual_n").is_some());
        assert!(journal.published_shape("result", "total_energy_ha").is_none());
        // Compared with no measurement, so not returned.
        assert!(journal.published_shape("result", "attitude_angle_deg").is_none());
        assert!(journal.published_shape("result", "peak_film_pressure_pa").is_none());
        let names: Vec<&str> = journal.result.iter().map(|output| output.name).collect();
        assert_eq!(names, ["sommerfeld_number", "eccentricity_ratio", "minimum_film_thickness_m"]);
        assert!(!journal.summary.contains("attitude") && !journal.summary.contains("pressure"));
        assert!(atom.published_shape("result", "eccentricity_ratio").is_none());
        assert!(atom.published_shape("invariants", "flow_balance_residual").is_none());
        assert!(crate::solvers::find("atom").is_none());
    }

    #[test]
    fn the_solve_schema_names_the_solvers_of_the_view() {
        let full = Value::Object(solve_schema_in(&View::full()));
        assert_eq!(full["properties"]["solver"]["type"], "string");
        // The solvers are left to list_solvers, and their inputs to describe_solver.
        assert!(full["properties"]["solver"].get("enum").is_none());
        let names: Vec<&str> = ALL.iter().map(|solver| solver.name).collect();
        assert!(names.contains(&JOURNAL_BEARING_SHEFFIELD.name));
        let parameters = &full["properties"]["parameters"];
        assert!(parameters.get("required").is_none());
        assert!(!parameters.to_string().contains("load_n"));
        let atom = Value::Object(solve_schema_in(&View::limited([], ["atom_dft".to_string()])));
        assert_eq!(atom["properties"]["solver"], json!({ "const": "atom_dft" }));
        assert!(!atom.to_string().contains("load_n"));
        let journal = Value::Object(solve_schema_in(&View::limited(
            [],
            [JOURNAL_BEARING_SHEFFIELD.name.to_string()],
        )));
        assert_eq!(
            journal["properties"]["solver"],
            json!({ "const": JOURNAL_BEARING_SHEFFIELD.name })
        );
        assert_eq!(
            journal["properties"]["parameters"]["required"],
            json!(["load_n", "speed_rpm"])
        );
        assert_eq!(
            journal["properties"]["parameters"]["properties"]
                .as_object()
                .expect("properties")
                .len(),
            2
        );
        assert!(!journal.to_string().contains("atomic_number"));
        let none = Value::Object(solve_schema_in(&View::limited([], [])));
        assert_eq!(none["properties"]["solver"], full["properties"]["solver"]);
    }

    /// The validation record states each comparison as it came out, the ones
    /// that missed included.
    #[test]
    fn the_validation_record_holds_every_number_of_the_comparison() {
        let [object, data, pass_line, result, limits] = validation();
        assert!(object.contains("University of Sheffield"));
        assert!(data.contains("fifty runs") && data.contains("doi 10.17632/txdrvd382p.2"));
        for mark in ["0.15", "0.35", "0.10", "5319a2f190dece027bd0f19cb339b9d950827a2e7dcde7241b251380a21de01a"] {
            assert!(pass_line.contains(mark), "{mark}");
        }
        for number in [
            "0.0580", "0.1563", "0.0199", "0.1409", "0.2574", "0.1293", "0.4761", "0.1277",
            "0.0636", "0.3339", "0.0085", "0.0617", "0.3235", "0.2106", "0.9356", "-0.0648",
        ] {
            assert!(result.contains(number), "{number}");
        }
        assert!(limits.contains("interpolation, not extrapolation"));
        assert!(limits.contains("One constant is fitted"));
        assert!(limits.contains("The solver returns no attitude angle, no pressure"));
        assert!(limits.contains("no pressure was compared with measurement"));
    }
}
