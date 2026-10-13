//! What the entries for `fusion_pb11_lhd` and `fusion_pb11_thermal` share.
//! It is not an entry: the record of the cross-section both solvers take
//! from `genesis_solvers::pb11_reaction`, and when their pass lines were
//! fixed. The numbers are those of the research record of the two solvers
//! and of the reaction rates.

use genesis_solvers::pb11_reaction as reaction;
use serde_json::{Value, json};

use crate::solvers::{Output, Shape};

/// When the pass lines of both solvers were fixed, and the file that holds
/// them.
pub const PASS_LINES_FIXED: &str = "2026-10-09T20:51:55Z, SHA-256 8bd91c5c1328f461a268ccca6dc0f31c61b11ab95051da1fe260fe5c2841db90 of validation_protocol.md";

/// The second scoring of both solvers, and where its record is kept.
pub const SECOND_SCORING: &str = "Second scoring of 2026-10-09, same machine, same verdicts: evidence/checks/second_party/V-26/.";

/// How often the comparison runs were made.
pub const RUN_TWICE: &str = "twice, both times after the pass lines were fixed: the second time after the results had been written into this description and the gateway rebuilt, and all 58 answers of the two solvers were identical";

/// Where the record of both comparisons is kept.
pub const EVIDENCE: &str = "evidence/research/fusion_pb11/solver/";

/// One published number.
pub const fn number(name: &'static str, unit: &'static str, note: &'static str) -> Output {
    Output {
        name,
        unit,
        shape: Shape::Number,
        each: &[],
        note,
        asked_by: None,
    }
}

/// What the shared cross-section was held to.
pub fn cross_section_record() -> Value {
    json!({
        "fit": "Tentori and Belloni (2023), Nucl. Fusion 63, 086001, Table 1, CC BY 4.0: an S-factor fit to measured points from 22 keV (Becker 1987) to 3481 keV (Sikora and Weller 2016) and a digitised curve from 5.7 to 9.76 MeV.",
        "range_kev": [0.0, reaction::CROSS_SECTION_ENERGY_MAX_KEV],
        "uncertainty": "The evaluations differ: the Maxwellian reactivity is known to about plus or minus 10 percent from 10 to 60 keV and to about minus 15, plus 5 percent from 70 to 500 keV. This fit is the highest of the three compared. On the low side from 70 to 500 keV the other two evaluations are lower still: the low bound of NACRE II is 16 to 19 percent under this fit and Nevins-Swain 7 to 27 percent under.",
        "against_its_own_integral": { "temperatures": 22, "worst_relative": 1.9e-10, "pass_line": 1.0e-6, "met": true },
        "against_the_published_reactivity_fit": { "temperatures_70_to_500_kev": 11, "worst_abs": 0.0062, "pass_line": 0.01, "met": true },
        "against_nacre_ii": { "points": 34, "inside_its_stated_band": 30, "pass_line": "inside at every point", "met": false, "note": "Above the band's high value by 0.36, 0.65, 0.43 and 0.18 percent at 69, 78, 86 and 108 keV. Known and written into the protocol before scoring; the pass line was not moved." },
    })
}

/// `(variant, doc sentence)` for every variant of the enum that `opening`
/// starts, read from the solver's source. An intra-doc link is written as
/// its bare code span.
#[cfg(test)]
pub fn documented_rejects_of(source: &str, opening: &str) -> Vec<(String, String)> {
    let start = source.find(opening).expect("the enum is in the solver");
    let mut found = Vec::new();
    let mut doc = String::new();
    for line in source[start..].lines().skip(1) {
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
