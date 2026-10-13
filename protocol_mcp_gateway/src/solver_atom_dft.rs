//! The entry for `atom_dft`: the spin-unpolarized, non-relativistic
//! local-density ground state of one spherical neutral atom. The numbers
//! come from `genesis_solvers` constants.

use genesis_solvers::atom_dft::{self, AtomGroundState, AtomReject};
use serde_json::{Value, json};

use crate::solvers::{
    Answer, Given, Input, Kind, Output, Published, Refusal, Refused, Sections, Shape, Solved,
    Solver,
};

/// Occupied orbitals a solution may name, for atomic numbers 1 through 18.
const ORBITALS: &[&str] = &["1s", "2s", "2p", "3s", "3p"];

const fn hartree(name: &'static str) -> Output {
    Output {
        name,
        unit: "Hartree",
        shape: Shape::Number,
        each: &[],
        note: "",
        asked_by: None,
    }
}

pub const ATOM_DFT: Solver = Solver {
    name: "atom_dft",
    summary: "Spin-unpolarized, non-relativistic local-density ground state of one spherical neutral atom, atomic number 1 through 18: total energy, its four parts, and each orbital eigenvalue. It reproduces the NIST spin-unpolarized LDA tables and is not the measured energy of the atom.",
    inputs: &[Input {
        name: "atomic_number",
        kind: Kind::Count {
            minimum: atom_dft::ATOMIC_NUMBER_MIN as u64,
            maximum: atom_dft::ATOMIC_NUMBER_MAX as u64,
            spoken: "atomic number",
        },
        refusal: None,
        bound: "The neutral atom. No other input is accepted: the grid, the mixing weight, the tolerances and the iteration cap are constants of the solver.",
        only_when: None,
        asked_by_check: false,
    }],
    kept_inside: "The grid, the mixing and the tolerances stay inside the solver.",
    result: &[
        hartree("total_energy_ha"),
        hartree("kinetic_energy_ha"),
        hartree("electron_nucleus_energy_ha"),
        hartree("hartree_energy_ha"),
        hartree("exchange_correlation_energy_ha"),
        Output {
            name: "eigenvalue_{orbital}_ha",
            unit: "Hartree",
            shape: Shape::Number,
            each: ORBITALS,
            note: "One per occupied orbital: 1s, 2s, 2p, 3s, 3p.",
            asked_by: None,
        },
        Output {
            name: "occupation_{orbital}",
            unit: "electrons",
            shape: Shape::Number,
            each: ORBITALS,
            note: "",
            asked_by: None,
        },
    ],
    invariants: &[
        Output {
            name: "electron_count",
            unit: "electrons",
            shape: Shape::Number,
            each: &[],
            note: "",
            asked_by: None,
        },
        hartree("virial_residual_ha"),
    ],
    iterations: "Density updates taken. On a refusal it is the count for `NotConverged` and null otherwise.",
    field: field_words,
    answer: "a ground state",
    refusals: REFUSALS,
    sections,
    validation: None,
    example,
    further_examples: &[],
    parts: &[],
    check: nothing_more_to_check,
    solve,
};

/// Each `AtomReject` variant with the sentence from its doc comment. The
/// request itself shows an atomic number outside the range, so that refusal
/// is not sealed.
const REFUSALS: &[Refusal] = &[
    Refusal {
        name: "AtomicNumberOutOfRange",
        meaning: "`atomic_number` is outside `ATOMIC_NUMBER_MIN`..=`ATOMIC_NUMBER_MAX`.",
        sealed: false,
    },
    Refusal {
        name: "ShellsDoNotHoldElectrons",
        meaning: "The fixed subshell list could not place every electron.",
        sealed: true,
    },
    Refusal {
        name: "ElectronCountBroken",
        meaning: "The density integral left `ELECTRON_COUNT_TOLERANCE`.",
        sealed: true,
    },
    Refusal {
        name: "NegativeDensity",
        meaning: "A density sample was negative.",
        sealed: true,
    },
    Refusal {
        name: "NonFiniteValue",
        meaning: "An intermediate value was NaN or infinite.",
        sealed: true,
    },
    Refusal {
        name: "OrbitalNotBound",
        meaning: "No bound radial solution with the required node count.",
        sealed: true,
    },
    Refusal {
        name: "EigenvalueStepOutOfRange",
        meaning: "A correction to a bisected eigenvalue was larger than `EIGENVALUE_STEP_LIMIT` of that eigenvalue.",
        sealed: true,
    },
    Refusal {
        name: "NotConverged",
        meaning: "The iteration cap was reached with the tolerances still open.",
        sealed: true,
    },
    Refusal {
        name: "VirialBroken",
        meaning: "The virial residual at an otherwise converged density was too large.",
        sealed: true,
    },
];

/// The `AtomReject` variant name, without its fields.
pub fn reject_name(reject: &AtomReject) -> &'static str {
    match reject {
        AtomReject::AtomicNumberOutOfRange { .. } => "AtomicNumberOutOfRange",
        AtomReject::ShellsDoNotHoldElectrons { .. } => "ShellsDoNotHoldElectrons",
        AtomReject::ElectronCountBroken { .. } => "ElectronCountBroken",
        AtomReject::NegativeDensity => "NegativeDensity",
        AtomReject::NonFiniteValue => "NonFiniteValue",
        AtomReject::OrbitalNotBound { .. } => "OrbitalNotBound",
        AtomReject::EigenvalueStepOutOfRange { .. } => "EigenvalueStepOutOfRange",
        AtomReject::NotConverged { .. } => "NotConverged",
        AtomReject::VirialBroken { .. } => "VirialBroken",
    }
}

fn field_words() -> String {
    format!(
        "SHA-256 of the grid point count ({}) as 8 little-endian bytes, then the converged electron density at each grid point as little-endian f64 bytes.",
        atom_dft::GRID_POINTS
    )
}

fn sections() -> Sections {
    Sections {
        locked: json!({
            "grid_points": atom_dft::GRID_POINTS,
            "r_min_bohr": atom_dft::R_MIN_BOHR,
            "r_max_bohr": atom_dft::R_MAX_BOHR,
            "mixing_alpha": atom_dft::MIXING_ALPHA,
            "max_scf_iterations": atom_dft::MAX_SCF_ITERATIONS,
            "eigenvalue_step_limit": atom_dft::EIGENVALUE_STEP_LIMIT,
        }),
        invariants: json!([
            {
                "name": "electron_count",
                "rule": "The density integrates to the atomic number.",
                "tolerance": atom_dft::ELECTRON_COUNT_TOLERANCE,
                "unit": "electrons",
                "checked": "every iteration",
                "refusal": "ElectronCountBroken",
            },
            {
                "name": "virial_residual_ha",
                "rule": "The virial identity for this functional holds.",
                "tolerance": atom_dft::VIRIAL_TOLERANCE_HA,
                "unit": "Hartree",
                "checked": "at acceptance",
                "refusal": "VirialBroken",
            },
        ]),
        convergence: json!({
            "energy_tolerance_ha": atom_dft::ENERGY_TOLERANCE_HA,
            "kinetic_tolerance_ha": atom_dft::KINETIC_TOLERANCE_HA,
            "density_residual_tolerance": atom_dft::DENSITY_RESIDUAL_TOLERANCE,
            "rule": "The change in total energy, the change in kinetic energy and the unmixed density must all be inside their tolerances before a result is returned.",
            "refusal": "NotConverged",
        }),
        reference: json!({
            "name": atom_dft::REFERENCE_NAME,
            "url": atom_dft::REFERENCE_URL,
            "retrieved": atom_dft::REFERENCE_RETRIEVED,
            "tolerance_ha": atom_dft::REFERENCE_TOLERANCE_HA,
            "test": "nist_lda_from_hydrogen_to_argon",
            "covers": "Total energy, its four parts and every orbital eigenvalue, for atomic numbers 1 through 18.",
            "kind": "A reference calculation in the spin-unpolarized, non-relativistic local-density approximation, the NIST LDA table. It is not a measurement, and it is not the spin-polarized or the relativistic table.",
        }),
    }
}

fn example() -> Value {
    json!({ "atomic_number": 10 })
}

/// The atomic number is the only input, and its bounds are in the field list.
fn nothing_more_to_check(_given: &Given) -> Result<(), String> {
    Ok(())
}

fn solve(given: &Given) -> Result<Solved, String> {
    let count = given.count("atomic_number")?;
    let atomic_number = u32::try_from(count).map_err(|_| {
        format!(
            "Physical Violation: atomic number {count} is outside {}..={}.",
            atom_dft::ATOMIC_NUMBER_MIN,
            atom_dft::ATOMIC_NUMBER_MAX
        )
    })?;
    Ok(solved(atom_dft::solve(atomic_number)))
}

fn orbital_label(n: u32, l: u32) -> String {
    match l {
        0 => format!("{n}s"),
        1 => format!("{n}p"),
        2 => format!("{n}d"),
        3 => format!("{n}f"),
        _ => format!("{n}_l{l}"),
    }
}

/// What the solver returned, in the words every solver shares.
pub fn solved(returned: Result<AtomGroundState, AtomReject>) -> Solved {
    let state = match returned {
        Ok(state) => state,
        Err(reject) => {
            return Solved::Refused(Refused {
                name: reject_name(&reject),
                sentence: reject.to_string(),
                iterations: match reject {
                    AtomReject::NotConverged { iterations } => Some(u64::from(iterations)),
                    _ => None,
                },
            });
        }
    };
    let mut result = vec![
        ("total_energy_ha".to_string(), Published::Number(state.total_energy_ha)),
        ("kinetic_energy_ha".to_string(), Published::Number(state.kinetic_energy_ha)),
        (
            "electron_nucleus_energy_ha".to_string(),
            Published::Number(state.electron_nucleus_energy_ha),
        ),
        ("hartree_energy_ha".to_string(), Published::Number(state.hartree_energy_ha)),
        (
            "exchange_correlation_energy_ha".to_string(),
            Published::Number(state.exchange_correlation_energy_ha),
        ),
    ];
    for orbital in &state.orbitals {
        let label = orbital_label(orbital.n, orbital.l);
        result.push((
            format!("eigenvalue_{label}_ha"),
            Published::Number(orbital.eigenvalue_ha),
        ));
        result.push((
            format!("occupation_{label}"),
            Published::Number(orbital.occupation),
        ));
    }
    Solved::Answer(Answer {
        iterations: u64::from(state.iterations),
        result,
        invariants: vec![
            ("electron_count".to_string(), Published::Number(state.electron_count)),
            (
                "virial_residual_ha".to_string(),
                Published::Number(state.virial_residual_ha),
            ),
        ],
        field: state.electron_density,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::solvers::describe_value;

    const SOLVER_SOURCE: &str = include_str!("../../genesis_solvers/src/atom_dft.rs");

    /// `(variant, doc sentence)` for every variant of `AtomReject`, read from
    /// the solver's source. An intra-doc link is written as its bare code span.
    fn documented_rejects() -> Vec<(String, String)> {
        let start = SOLVER_SOURCE
            .find("pub enum AtomReject {")
            .expect("AtomReject is in the solver");
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
        let body = describe_value(ATOM_DFT.name).expect("described");
        let refusals = body["refusals"].as_array().expect("refusals");
        assert_eq!(refusals.len(), documented.len());
        for refusal in refusals {
            assert!(refusal["meaning"].is_string());
        }
        // Only the refusal the request itself shows is answered with no seal.
        for refusal in refusals {
            let at_the_door = refusal["name"] == "AtomicNumberOutOfRange";
            let reported = if at_the_door { "422, no seal" } else { "200, outcome refused, sealed" };
            assert_eq!(refusal["reported_as"], reported, "{}", refusal["name"]);
        }
    }

    #[test]
    fn the_reference_is_the_one_stored_in_the_solver_test() {
        let body = describe_value(ATOM_DFT.name).expect("described");
        let url = body["reference"]["url"].as_str().expect("url");
        let retrieved = body["reference"]["retrieved"].as_str().expect("retrieved");
        assert!(url.starts_with("https://www.nist.gov/"), "{url}");
        assert!(SOLVER_SOURCE.contains(&format!("<{url}>")), "{url}");
        assert!(
            SOLVER_SOURCE.contains(&format!("Retrieved {retrieved}")),
            "{retrieved}"
        );
        assert!(describe_value("atom").is_none());
    }

    /// The note is the list of orbitals, and every orbital the solver fills
    /// for atomic numbers 1 through 18 is a name the entry publishes.
    #[test]
    fn the_orbitals_named_are_the_orbitals_published() {
        assert_eq!(
            ATOM_DFT.result[5].note,
            format!("One per occupied orbital: {}.", ORBITALS.join(", "))
        );
        for (n, l) in [(1, 0), (2, 0), (2, 1), (3, 0), (3, 1)] {
            let label = orbital_label(n, l);
            assert!(ORBITALS.contains(&label.as_str()), "{label}");
            for key in [format!("eigenvalue_{label}_ha"), format!("occupation_{label}_hex")] {
                assert_eq!(ATOM_DFT.published_shape("result", &key), Some(Shape::Number), "{key}");
            }
        }
        for key in ["eigenvalue_4s_ha", "occupation_", "eigenvalue__ha", "occupation_3d", "total_energy"] {
            assert_eq!(ATOM_DFT.published_shape("result", key), None, "{key}");
        }
        assert_eq!(ATOM_DFT.published_shape("invariants", "total_energy_ha"), None);
        assert_eq!(ATOM_DFT.published_shape("inputs", "electron_count"), None);
    }
}
