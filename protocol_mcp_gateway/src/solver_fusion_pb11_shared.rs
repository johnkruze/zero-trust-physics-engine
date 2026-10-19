//! What the entries for `fusion_pb11_lhd` and `fusion_pb11_thermal` share.
//! It is not an entry: the record of the cross-section both solvers take
//! from `genesis_solvers::pb11_reaction`, and the fingerprint of the record
//! of their targets. The numbers are those of the research record of the two solvers
//! and of the reaction rates.

use genesis_solvers::pb11_reaction as reaction;
use serde_json::{Value, json};

use crate::solvers::{Output, Shape};

/// The fingerprint of the record that holds the targets of both solvers.
pub const PASS_LINES_FIXED: &str = "Record SHA-256 8bd91c5c1328f461a268ccca6dc0f31c61b11ab95051da1fe260fe5c2841db90.";

/// The second run of the comparison of both solvers.
pub const SECOND_SCORING: &str = "A second run of the comparison on the same machine recomputes the same numbers.";

/// How often the comparison runs were made.
pub const RUN_TWICE: &str = "twice, the second time on a rebuilt gateway with these results written into this description, and all 58 answers of the two solvers were identical";

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
        "fit": "The S-factor fit of Tentori and Belloni, Nucl. Fusion 63, 086001, Table 1: measured points from 22 keV (Becker's 1987 data) to 3481 keV (Sikora and Weller's 2016 data) and a digitised curve from 5.7 to 9.76 MeV.",
        "range_kev": [0.0, reaction::CROSS_SECTION_ENERGY_MAX_KEV],
        "uncertainty": "The evaluations differ: the Maxwellian reactivity is known to about plus or minus 10 percent from 10 to 60 keV and to about minus 15, plus 5 percent from 70 to 500 keV. This fit is the highest of the three compared. On the low side from 70 to 500 keV the other two evaluations are lower still: the low bound of NACRE II is 16 to 19 percent under this fit and Nevins-Swain 7 to 27 percent under.",
        "against_its_own_integral": { "temperatures": 22, "worst_relative": 1.9e-10, "pass_line": 1.0e-6, "met": true },
        "against_the_published_reactivity_fit": { "temperatures_70_to_500_kev": 11, "worst_abs": 0.0062, "pass_line": 0.01, "met": true },
        "against_nacre_ii": { "points": 34, "inside_its_stated_band": 30, "pass_line": "inside at every point", "met": false, "note": "Above the band's high value by 0.36, 0.65, 0.43 and 0.18 percent at 69, 78, 86 and 108 keV; the record states these four points and keeps the target at every point." },
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
