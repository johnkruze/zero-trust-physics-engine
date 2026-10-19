//! The entry for `argon_pair_gas`: argon gas at low density from one pair
//! potential. The numbers come from `genesis_solvers` constants.

use genesis_solvers::argon_pair_gas::{self as argon, ArgonGas, ArgonReject, Question};
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

pub const ARGON_PAIR_GAS: Solver = Solver {
    name: "argon_pair_gas",
    summary: "Argon gas at low density from one Lennard-Jones pair potential whose two constants are fitted to the measured speed of sound in argon: second virial coefficient, speed of sound and viscosity at zero density. Every property is answered from 250 K to 6000 K, where argon begins to ionize, at 0 to 500 kPa, where the pressure series cut after its first term is written. Compared with measurement, all five lines met: the speed of sound from 250 K to 349.868 K at 30.5 kPa to 467.2 kPa (held-back rms 0.0074 m/s), the viscosity from 291.09 K to 681.96 K at zero pressure (mean 1.47 percent high).",
    inputs: &[
        Input {
            name: "temperature_k",
            kind: Kind::Number {
                minimum: argon::TEMPERATURE_MIN_K,
                maximum: argon::TEMPERATURE_MAX_K,
                unit: "K",
            },
            refusal: Some("TemperatureOutOfRange"),
            bound: "Temperature of the gas, 250 K to 6000 K. Below 250 K, the lowest isotherm compared, the term the pressure series leaves out grows as the gas cools toward condensation, and its size is not known here; above 6000 K argon begins to ionize (an ionized fraction of 2e-5 at 6000 K and 100 kPa, 2 percent at 10,000 K), which a pair potential of neutral atoms does not hold. Both are TemperatureOutOfRange. The speed of sound is compared with measurement from 250 K to 349.868 K and the viscosity from 291.09 K to 681.96 K, where it reads 3.0 percent high at 291 K falling to 0.3 percent at 682 K; outside those spans every property is computed with the same equations.",
            only_when: None,
            asked_by_check: false,
        },
        Input {
            name: "pressure_kpa",
            kind: Kind::Number {
                minimum: argon::PRESSURE_MIN_KPA,
                maximum: argon::PRESSURE_MAX_KPA,
                unit: "kPa",
            },
            refusal: Some("PressureOutOfRange"),
            bound: "Pressure of the gas, 0 kPa to 500 kPa, where the pressure series cut after its first term is written; below 0 kPa or above 500 kPa, where above 500 kPa the left-out term is larger than the measurement uncertainty, it is refused as PressureOutOfRange. Every property is returned at every pressure: at zero the speed of sound is the ideal gas's. The speed of sound was compared at 30.5 kPa to 467.2 kPa, and the viscosity at zero pressure, the limit of zero density. The viscosity and the second virial coefficient do not depend on the pressure: every pressure answered gives the same ones.",
            only_when: None,
            asked_by_check: true,
        },
    ],
    kept_inside: "The two constants of the pair potential, the molar mass, the quadrature rules and their tolerances stay inside the solver.",
    result: &[
        number("reduced_temperature", "1", "Temperature over the well depth of the pair potential."),
        number("second_virial_cm3_per_mol", "cm3/mol", "B(T). Compared with measurement through the speed of sound, which carries it in the acoustic virial coefficient, from 250 K to 349.868 K at 30.5 kPa to 467.2 kPa."),
        number("second_virial_first_derivative_cm3_per_mol", "cm3/mol", "T dB/dT."),
        number("second_virial_second_derivative_cm3_per_mol", "cm3/mol", "T² d²B/dT²."),
        number("acoustic_virial_cm3_per_mol", "cm3/mol", "2B + (4/3) T dB/dT + (4/15) T² d²B/dT²."),
        number("sound_speed_m_per_s", "m/s", "At the asked pressure, the pressure series cut after its first term. Compared with measurement from 250 K to 349.868 K at 30.5 kPa to 467.2 kPa; at zero pressure it is the ideal gas's."),
        number("ideal_gas_sound_speed_m_per_s", "m/s", "The same gas with no forces between its atoms."),
        number("collision_integral_22", "1", "Reduced collision integral Ω(2,2)*."),
        number("collision_integral_23", "1", "Reduced collision integral Ω(2,3)*."),
        number("viscosity_second_approximation_factor", "1", "Kihara's factor, 1 + (3/196)(8 Ω(2,3)*/Ω(2,2)* − 7)²."),
        number("viscosity_pa_s", "Pa s", "In the limit of zero density; it does not depend on the asked pressure. Compared with measurement from 291.09 K to 681.96 K, asked at zero pressure; below 291.09 K and above 681.96 K it was never compared."),
    ],
    invariants: &[
        number("virial_forms_residual_cm3_per_mol", "cm3/mol", ""),
        number("least_turning_point_margin", "1", ""),
        number("collision_integral_rule_gap", "1", ""),
    ],
    iterations: "Panels halved in the second virial integrals and in the thermal average of the cross-section, together. On a refusal it is the number of times the open panel had been halved, for the three refusals that carry it, and null otherwise.",
    field: field_words,
    answer: "a settled gas",
    refusals: REFUSALS,
    door_refusals: &[],
    sections,
    validation: Some(validation),
    example,
    further_examples: &[],
    parts: &[],
    check,
    solve,
};

/// Each `ArgonReject` variant with the sentence from its doc comment. The
/// request itself shows a temperature or a pressure outside the range, so
/// those two are not sealed.
const REFUSALS: &[Refusal] = &[
    Refusal {
        name: "TemperatureOutOfRange",
        meaning: "`temperature_k` is outside `TEMPERATURE_MIN_K` to `TEMPERATURE_MAX_K`: below it the pressure series' left-out term is not known, above it argon begins to ionize.",
        sealed: false,
    },
    Refusal {
        name: "PressureOutOfRange",
        meaning: "`pressure_kpa` is outside `PRESSURE_MIN_KPA` to `PRESSURE_MAX_KPA`, where the pressure series cut after its first term is written.",
        sealed: false,
    },
    Refusal {
        name: "NonFiniteValue",
        meaning: "An intermediate value was NaN or infinite.",
        sealed: true,
    },
    Refusal {
        name: "VirialNotSettled",
        meaning: "A second virial integral was still open after a panel had been halved `MAX_REFINEMENTS` times over.",
        sealed: true,
    },
    Refusal {
        name: "VirialFormsDisagree",
        meaning: "The Mayer form and the force form of the second virial coefficient differ by more than `VIRIAL_FORMS_TOLERANCE_CM3_PER_MOL`.",
        sealed: true,
    },
    Refusal {
        name: "TurningPointNotOutermost",
        meaning: "A collision was asked at a closest approach that the atoms do not reach first.",
        sealed: true,
    },
    Refusal {
        name: "DeflectionNotSettled",
        meaning: "A deflection angle was still open after a panel had been halved `MAX_REFINEMENTS` times over.",
        sealed: true,
    },
    Refusal {
        name: "CrossSectionNotSettled",
        meaning: "A cross-section was still open after a panel had been halved `MAX_REFINEMENTS` times over.",
        sealed: true,
    },
    Refusal {
        name: "CollisionIntegralNotSettled",
        meaning: "A collision integral was still open after a panel of the thermal average had been halved `MAX_ENERGY_REFINEMENTS` times over.",
        sealed: true,
    },
];

/// The `ArgonReject` variant name, without its fields.
pub fn reject_name(reject: &ArgonReject) -> &'static str {
    match reject {
        ArgonReject::TemperatureOutOfRange { .. } => "TemperatureOutOfRange",
        ArgonReject::PressureOutOfRange { .. } => "PressureOutOfRange",
        ArgonReject::NonFiniteValue => "NonFiniteValue",
        ArgonReject::VirialNotSettled { .. } => "VirialNotSettled",
        ArgonReject::VirialFormsDisagree { .. } => "VirialFormsDisagree",
        ArgonReject::TurningPointNotOutermost { .. } => "TurningPointNotOutermost",
        ArgonReject::DeflectionNotSettled { .. } => "DeflectionNotSettled",
        ArgonReject::CrossSectionNotSettled { .. } => "CrossSectionNotSettled",
        ArgonReject::CollisionIntegralNotSettled { .. } => "CollisionIntegralNotSettled",
    }
}

fn field_words() -> String {
    "SHA-256 of the count of the numbers that follow as 8 little-endian bytes, which is twice the number of collision energies of the settled thermal average, then each reduced collision energy in rising order, then the reduced viscosity cross-section Q(2)* at each of those energies, all as little-endian f64 bytes.".to_string()
}

fn sections() -> Sections {
    Sections {
        locked: json!({
            "pair_potential": "Lennard-Jones 12-6: u(r) = 4 ε [(σ/r)^12 − (σ/r)^6]. Classical mechanics and statistics, no quantum correction, no three-body force.",
            "well_depth_k": argon::WELL_DEPTH_K,
            "zero_distance_angstrom": argon::ZERO_DISTANCE_ANGSTROM,
            "fitted": "Both constants of the pair potential, by least squares on the measured speed of sound of this same gas: 42 points at or below 500 kPa on eleven isotherms from 250 K to 300.117 K.",
            "molar_mass_kg_per_mol": argon::MOLAR_MASS_KG_PER_MOL,
            "gas_constant_j_per_mol_k": argon::GAS_CONSTANT_J_PER_MOL_K,
            "boltzmann_j_per_k": argon::BOLTZMANN_J_PER_K,
            "avogadro_per_mol": argon::AVOGADRO_PER_MOL,
            "heat_capacity_ratio": argon::HEAT_CAPACITY_RATIO,
            "sound_speed": "u² = (γ/M)(R T + β p), β the second acoustic virial coefficient.",
            "viscosity": "η = (5/16) sqrt(π m k T) / (π σ² Ω(2,2)*) × Kihara's factor.",
            "max_refinements": argon::MAX_REFINEMENTS,
            "max_energy_refinements": argon::MAX_ENERGY_REFINEMENTS,
        }),
        invariants: json!([
            {
                "name": "virial_forms_residual_cm3_per_mol",
                "rule": "The second virial coefficient from the Mayer integral and from the force integral, which are one another's integration by parts, are the same number.",
                "tolerance": argon::VIRIAL_FORMS_TOLERANCE_CM3_PER_MOL,
                "unit": "cm3/mol",
                "compared": "at acceptance",
                "refusal": "VirialFormsDisagree",
            },
            {
                "name": "least_turning_point_margin",
                "rule": "Every collision is computed at the closest approach the two atoms reach first: the margin is positive for every deflection angle.",
                "tolerance": 0.0,
                "unit": "1",
                "compared": "every deflection angle",
                "refusal": "TurningPointNotOutermost",
            },
            {
                "name": "collision_integral_rule_gap",
                "rule": "Over the settled thermal average, the fifteen-point and the seven-point quadrature rules give each collision integral to the same fraction.",
                "tolerance": argon::COLLISION_INTEGRAL_SETTLE_TOLERANCE,
                "unit": "1",
                "compared": "every panel of the thermal average",
                "refusal": "CollisionIntegralNotSettled",
            },
        ]),
        convergence: json!({
            "virial_settle_tolerance": argon::VIRIAL_SETTLE_TOLERANCE,
            "deflection_settle_tolerance": argon::DEFLECTION_SETTLE_TOLERANCE,
            "cross_section_settle_tolerance": argon::CROSS_SECTION_SETTLE_TOLERANCE,
            "collision_integral_settle_tolerance": argon::COLLISION_INTEGRAL_SETTLE_TOLERANCE,
            "rule": "Every integral is taken panel by panel with a fifteen-point and a seven-point rule. A panel on which the two differ by more than its share of the tolerance is halved, and halved again, up to the cap. A result is returned once every panel of every integral has closed: the four second virial integrals, each deflection angle, each cross-section, and the thermal average.",
            "refusal": "VirialNotSettled, DeflectionNotSettled, CrossSectionNotSettled or CollisionIntegralNotSettled",
        }),
        reference: json!({
            "name": argon::REFERENCE_NAME,
            "url": argon::REFERENCE_URL,
            "url_collision_integrals": argon::REFERENCE_URL_COLLISION,
            "retrieved": argon::REFERENCE_RETRIEVED,
            "tolerance_second_virial_reduced": argon::REFERENCE_TOLERANCE_VIRIAL,
            "tolerance_collision_integral_fraction": argon::REFERENCE_TOLERANCE_COLLISION,
            "test": "nbs_tables_inside_the_served_range",
            "covers": "The reduced second virial coefficient, Ω(2,2)* and Ω(2,3)* at the thirteen tabulated reduced temperatures inside the temperature range, 2.2 to 5.5. Found: the second virial coefficient to the table's last printed digit, the collision integrals within three parts in ten thousand.",
            "kind": "Reference calculations for the 12-6 potential itself: they hold the solver's integrals of the potential, and the comparison with argon is the validation record.",
        }),
    }
}

fn validation() -> [String; 5] {
    let object = "Argon gas, 99.999 percent pure, at low density. Two atoms are given one Lennard-Jones 12-6 pair potential, its two constants fitted to this gas's own measured speed of sound. Everything else follows from the potential with no further constant: the second virial coefficient and the speed of sound by statistical mechanics, the viscosity by the kinetic theory of Chapman and Enskog.".to_string();
    let data = "Speed of sound: two spherical resonators, J. Chem. Eng. Data 54, 2758, 15 isotherms from 250 K to 349.868 K, stated expanded uncertainty under 0.03 m/s; the archive's compiler gives its own 95 percent uncertainty beside it, 0.50 to 0.59 m/s. Fitted on the 42 points at or below 500 kPa of the eleven isotherms from 250 K to 300.117 K. Compared with the 24 points at or below 500 kPa of the four isotherms at 310 K, 320 K, 325.046 K and 349.868 K, not in the fit; they are 17 distinct states, seven of them printed twice with different speeds. Viscosity, not in the fit: an oscillating-disk viscometer, Int. J. Thermophys. 31, 447, 49 points from 291.09 K to 681.96 K, stated uncertainty 0.15 to 0.2 percent; the archive's compiler gives 0.30 to 0.31 percent at 95 percent. The viscosity is at a density of 1.1 kg/m3, not at zero density. The viscometer is relative: what is measured is the ratio of the viscosity to its value at 298.15 K, and the absolute level of all 49 values rests on that one value, which was calculated from an ab initio potential, not measured. Both sets are from the NIST ThermoML Archive, doi 10.18434/mds2-2422.".to_string();
    let pass_line = "Speed of sound, on the 24 points not in the fit: RMS error at most 0.030 m/s, worst at most 0.060 m/s, RMS at most a quarter of the ideal gas's. Viscosity, on all 49 points: mean error at most 2.0 percent, worst at most 4.0 percent. The two viscosity targets are an allowance for a two-constant potential, about ten times the measurement's uncertainty, set knowing that textbook constants give about 2 percent.".to_string();
    let result = RESULT.to_string();
    let limits = "The speed of sound outside the fit extends the fit in temperature by up to 50 K, on the same property and instruments. The viscosity is a different property from a different laboratory, up to 382 K above the highest fitted temperature. The split and the targets were chosen with 29 of the 163 speed-of-sound rows read, from all 15 isotherms, and 11 of the 49 viscosity rows. Answered with the same equations and compared with no measurement: a pressure above zero outside 30.5 to 467.2 kPa; the speed of sound above 349.868 K; the viscosity below 291.09 K; every property above 681.96 K up to 6000 K; the second virial coefficient itself, of which no measurement is used. Refused: temperatures below 250 K, where the size of the series' left-out term is not known, and above 6000 K, where argon begins to ionize; pressures above 500 kPa. Not served: the liquid; any mixture. The returned second virial coefficient is shifted: the fitted constants absorb the higher pressure term the series drops, so B is higher by roughly 0.1 to 0.4 cm3/mol at 300 K, -15.05 cm3/mol here against -15.18 from the reference equation of state. The 12-6 potential is a fitted form: its two constants stand in for what the form leaves out of the true potential of argon. A proof shows that an answer is the one the engine computed; how close it is to the gas is the comparison above.".to_string();
    [object, data, pass_line, result, limits]
}

/// The outcome of the comparison with the held-back measurements.
const RESULT: &str = "Speed of sound, 24 points not in the fit: RMS 0.0074 m/s (target 0.030), worst 0.0146 m/s (target 0.060), RMS 0.023 of the ideal gas's 0.327 m/s (target 0.25). Viscosity, 49 points: mean error 1.47 percent (target 2.0), worst 2.97 percent (target 4.0). The viscosity reads above the measurement at all 49 points, by 2.95 percent at 291 K falling to 0.28 percent at 682 K: 1.4 to 20 times the measurement's uncertainty, 9 on average, and no point is within it. Its change with temperature differs from argon's. The 12-6 form follows this viscosity with other constants: 143.84 K and 3.3379 angstrom fit all 49 points within 0.11 percent, and that pair is 0.22 m/s off on the speed of sound outside the fit; one pair of constants does not carry both properties. On the speed of sound outside the fit, the 12-6 form is one of several: a square well, van der Waals' second virial coefficient, a hard sphere with an r^-6 attraction, and Mie potentials from 9-6 to 30-6, each fitted on the same 42 points, give 0.0073 to 0.0107 m/s RMS. The floor is about 0.007 m/s: the reference equation of state gives 0.0076, and repeated states differ by up to 0.016 m/s. What carries is the second acoustic virial coefficient: fitted at 250 to 300 K, it extrapolates 50 K. The textbook constants of argon (119.8 K, 3.405 angstrom), fitted on none of these data, give 0.0068 m/s and 0.0136 m/s on the same speed-of-sound points and 0.80 percent and 2.05 percent on the viscosity. On the 42 fitted points the solver gives 0.0042 m/s RMS and 0.0122 m/s worst, the textbook constants 0.0061 and 0.0196, the ideal gas 0.149 and 0.377. The 115 answers of the comparison were computed by the binary SHA-256 90dfab953a94d199db146cfad89a4ce4bf750288c3b33b299fb2fed49e722066; a later build returns all 115 byte for byte, and engine 0.7.0 answers all 115, the 49 viscosity answers asked at zero pressure among them, the same in every field but the engine version and the receipt. A second run on the same machine recomputes the same numbers.";

fn example() -> Value {
    json!({ "temperature_k": 300.0, "pressure_kpa": 101.325 })
}

/// The temperature's bounds are in the field list. The pressure has two
/// limits with two names, the pressure series' range and the span compared
/// with measurement, so the solver's own check asks them, in its order,
/// before a permit is taken.
fn check(given: &Given) -> Result<(), String> {
    let question = Question {
        temperature_k: given.number("temperature_k")?,
        pressure_kpa: given.number("pressure_kpa")?,
    };
    argon::check(question)
        .map_err(|reject| format!("Physical Violation: {reject} ({}).", reject_name(&reject)))
}

fn solve(given: &Given) -> Result<Solved, String> {
    let question = Question {
        temperature_k: given.number("temperature_k")?,
        pressure_kpa: given.number("pressure_kpa")?,
    };
    Ok(solved(argon::solve(question)))
}

/// What the solver returned, in the words every solver shares.
pub fn solved(returned: Result<ArgonGas, ArgonReject>) -> Solved {
    let gas = match returned {
        Ok(gas) => gas,
        Err(reject) => {
            return Solved::Refused(Refused {
                name: reject_name(&reject),
                sentence: reject.to_string(),
                iterations: match reject {
                    ArgonReject::VirialNotSettled { refinements }
                    | ArgonReject::CollisionIntegralNotSettled { refinements } => {
                        Some(u64::from(refinements))
                    }
                    _ => None,
                },
            });
        }
    };
    let one = |name: &str, value: f64| (name.to_string(), Published::Number(value));
    let mut field = Vec::with_capacity(2 * gas.collision_energies.len());
    field.extend_from_slice(&gas.collision_energies);
    field.extend_from_slice(&gas.cross_sections);
    Solved::Answer(Answer {
        iterations: u64::from(gas.refinements),
        result: vec![
            one("reduced_temperature", gas.reduced_temperature),
            one("second_virial_cm3_per_mol", gas.second_virial_cm3_per_mol),
            one(
                "second_virial_first_derivative_cm3_per_mol",
                gas.second_virial_first_derivative_cm3_per_mol,
            ),
            one(
                "second_virial_second_derivative_cm3_per_mol",
                gas.second_virial_second_derivative_cm3_per_mol,
            ),
            one("acoustic_virial_cm3_per_mol", gas.acoustic_virial_cm3_per_mol),
            one("sound_speed_m_per_s", gas.sound_speed_m_per_s),
            one("ideal_gas_sound_speed_m_per_s", gas.ideal_gas_sound_speed_m_per_s),
            one("collision_integral_22", gas.collision_integral_22),
            one("collision_integral_23", gas.collision_integral_23),
            one(
                "viscosity_second_approximation_factor",
                gas.viscosity_second_approximation_factor,
            ),
            one("viscosity_pa_s", gas.viscosity_pa_s),
        ],
        invariants: vec![
            one(
                "virial_forms_residual_cm3_per_mol",
                gas.virial_forms_residual_cm3_per_mol,
            ),
            one("least_turning_point_margin", gas.least_turning_point_margin),
            one("collision_integral_rule_gap", gas.collision_integral_rule_gap),
        ],
        field,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::solvers::describe_value;

    const SOLVER_SOURCE: &str = include_str!("../../genesis_solvers/src/argon_pair_gas.rs");

    /// `(variant, doc sentence)` for every variant of `ArgonReject`, read from
    /// the solver's source. An intra-doc link is written as its bare code span.
    fn documented_rejects() -> Vec<(String, String)> {
        let start = SOLVER_SOURCE
            .find("pub enum ArgonReject {")
            .expect("ArgonReject is in the solver");
        let body = &SOLVER_SOURCE[start..];
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
    fn every_refusal_carries_the_sentence_from_its_doc_comment() {
        let documented = documented_rejects();
        assert_eq!(documented.len(), REFUSALS.len());
        for ((variant, sentence), refusal) in documented.iter().zip(REFUSALS) {
            assert_eq!(variant, refusal.name);
            assert_eq!(sentence, refusal.meaning, "{}", refusal.name);
        }
        let body = describe_value(ARGON_PAIR_GAS.name).expect("described");
        let refusals = body["refusals"].as_array().expect("refusals");
        assert_eq!(refusals.len(), documented.len());
        // Only the two refusals the request itself shows are answered with no seal.
        for refusal in refusals {
            let at_the_door = refusal["name"] == "TemperatureOutOfRange"
                || refusal["name"] == "PressureOutOfRange";
            let reported = if at_the_door { "422, no seal" } else { "200, outcome refused, sealed" };
            assert_eq!(refusal["reported_as"], reported, "{}", refusal["name"]);
        }
    }

    #[test]
    fn the_references_are_the_ones_stored_in_the_solver_test() {
        let body = describe_value(ARGON_PAIR_GAS.name).expect("described");
        for key in ["url", "url_collision_integrals"] {
            let url = body["reference"][key].as_str().expect("url");
            assert!(url.starts_with("https://nvlpubs.nist.gov/"), "{url}");
            assert!(SOLVER_SOURCE.contains(&format!("<{url}>")), "{url}");
        }
        let retrieved = body["reference"]["retrieved"].as_str().expect("retrieved");
        assert!(SOLVER_SOURCE.contains(&format!("Retrieved {retrieved}")), "{retrieved}");
        let test = body["reference"]["test"].as_str().expect("test");
        assert!(SOLVER_SOURCE.contains(&format!("fn {test}()")), "{test}");
    }

    /// The list an agent reads first says that the constants are fitted to
    /// the gas's own measurements, and the description says what the
    /// held-back data does and does not test.
    #[test]
    fn the_summary_says_the_constants_are_fitted() {
        assert!(ARGON_PAIR_GAS.summary.contains("fitted to the measured speed of sound in argon"));
        let body = describe_value(ARGON_PAIR_GAS.name).expect("described");
        let limits = body["validation"]["limits"].as_str().expect("limits");
        assert!(limits.contains("extends the fit in temperature by up to 50 K"));
        assert!(limits.contains("Answered with the same equations and compared with no measurement"));
        assert_eq!(body["locked"]["well_depth_k"], 119.33);
        assert_eq!(body["locked"]["zero_distance_angstrom"], 3.3918);
    }

    #[test]
    fn every_published_name_is_in_the_entry() {
        let gas = match solved(argon::solve(Question {
            temperature_k: 300.0,
            pressure_kpa: 101.325,
        })) {
            Solved::Answer(answer) => answer,
            Solved::Refused(refused) => panic!("{}", refused.sentence),
        };
        assert_eq!(gas.result.len(), ARGON_PAIR_GAS.result.len());
        for ((name, value), output) in gas.result.iter().zip(ARGON_PAIR_GAS.result) {
            assert_eq!(name, output.name);
            assert_eq!(value.shape(), output.shape);
        }
        assert_eq!(gas.invariants.len(), ARGON_PAIR_GAS.invariants.len());
        for ((name, _), output) in gas.invariants.iter().zip(ARGON_PAIR_GAS.invariants) {
            assert_eq!(name, output.name);
        }
        assert_eq!(gas.field.len() % 2, 0);
        assert!(!gas.field.is_empty());
    }
}
