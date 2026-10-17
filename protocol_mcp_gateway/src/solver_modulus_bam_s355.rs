//! The entry for `modulus_bam_s355`: Young's modulus and shear modulus of one
//! of four steel bars, worked back from a series of the bar's measured
//! resonant frequencies. The numbers come from `genesis_solvers` constants.
//!
//! The question is a series. `frequencies_hz` holds the measured frequencies
//! in a fixed order: flatwise, torsion, edgewise. Each has its own declared
//! range, and the three ranges do not meet, so a frequency in the wrong place
//! is refused by name. A series too short to fix a modulus is refused by
//! name before a physics permit is taken.

use genesis_solvers::modulus_bam_s355::{
    self as modulus, Bar, BarModuli, Mode, ModulusReject, Reading,
};
use serde_json::{Value, json};

use crate::solvers::{
    Answer, Given, Input, Kind, Output, Published, Refusal, Refused, Sections, Shape, Solved,
    Solver,
};

/// The bars, in the solver's order.
const BAR_NAMES: &[&str] = &["ERX1", "ERX2", "ERY1", "ERY2"];
/// The mode each place of `frequencies_hz` holds.
const SERIES_ORDER: [Mode; 3] = [Mode::Flatwise, Mode::Torsion, Mode::Edgewise];
/// The labels of the residual family, in the order a solution publishes them.
const RESIDUAL_MODES: &[&str] = &["flatwise", "torsion", "edgewise"];

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

pub const MODULUS_BAM_S355: Solver = Solver {
    name: "modulus_bam_s355",
    summary: "Young's modulus and shear modulus of one of four test bars, each about 100 mm long, cut from one sheet of S355J2+N steel at BAM Berlin, worked back from the bar's measured resonant frequencies, with the edgewise frequency they predict: a property of those bars, not the strength or the load of a steel structure; the comparison with the independent tensile measurement missed its line, and no independent measurement confirms the returned moduli.",
    inputs: &[
        Input {
            name: "bar",
            kind: Kind::Word { allowed: BAR_NAMES },
            refusal: Some("UnknownBar"),
            bound: "The bar that was measured: ERX1 and ERX2 were cut along the rolling direction of the sheet, ERY1 and ERY2 across it. The bar's length, width, thickness and mass are constants of the solver.",
            only_when: None,
            asked_by_check: false,
        },
        Input {
            name: "frequencies_hz",
            kind: Kind::Numbers {
                fewest: 0,
                most: 3,
                minimum: f64::MIN_POSITIVE,
                maximum: f64::MAX,
                unit: "Hz",
            },
            refusal: Some("FrequencyNotPositive"),
            bound: "The measured resonant frequencies of the free bar, in this order: flatwise (first bending mode across the thickness), torsion (first twisting mode), edgewise (first bending mode across the width). Each is a finite number above zero (FrequencyNotPositive otherwise); the recovery stops by name where no isotropic bar rings at the pair (PoissonRatioOutOfRange), where the search leaves 1 GPa to 2 TPa in E or 0.3 GPa to 1 TPa in G (OutsideSearchRange), or where the frequency equation has no root (FrequencyEquationNotBracketed). The eight scans compared with measurement span flatwise 1587.428 to 1608.28 Hz, torsion 8899.446 to 8966.189 Hz and edgewise 4695.035 to 4755.856 Hz; there the solver reproduced the laboratory's own values within 0.053 percent and predicted the edgewise frequency within 0.114 percent. Outside those spans nothing was compared. Two frequencies fix the two moduli. The third is never used to find them: when it is given, its distance from the predicted edgewise frequency is returned as it is; 0.25 percent is line P2 of the validation, the line the prediction was compared at. A series of fewer than two cannot fix both moduli and is refused by name. Through engine 0.8.0 a frequency outside its compared span, a predicted edgewise frequency outside the edgewise span and a supplied edgewise frequency past line P2 were refused by name.",
            only_when: None,
            asked_by_check: true,
        },
    ],
    kept_inside: "The bar's size and mass, the start and the tolerances stay inside the solver.",
    result: &[
        number("youngs_modulus_pa", "Pa", "Young's modulus along the bar, the fast (adiabatic) value."),
        number("shear_modulus_pa", "Pa", ""),
        number(
            "youngs_modulus_method_uncertainty_pa",
            "Pa",
            "The uncertainty the data set's own summary sheet states for these measurements at room temperature, ±1 percent at 95 percent confidence, as 1 percent of the value. The laboratory's later paper states at most 1 percent for the method.",
        ),
        number(
            "shear_modulus_method_uncertainty_pa",
            "Pa",
            "The same for the shear modulus: the summary sheet states the ±1 percent for both moduli, so 1 percent of the value.",
        ),
        number(
            "shear_modulus_method_uncertainty_ceiling_pa",
            "Pa",
            "The most the laboratory's later paper states for the shear modulus by this method at room temperature, on other alloys: max. 3 percent, as 3 percent of the value (Rehmer et al., Sci Data 10, 474 (2023), Technical Validation).",
        ),
        number(
            "edgewise_predicted_hz",
            "Hz",
            "The edgewise frequency the recovered moduli predict.",
        ),
        Output {
            name: "{mode}_residual_fraction",
            unit: "fraction",
            shape: Shape::Number,
            each: RESIDUAL_MODES,
            note: "Model over measured frequency, less one. Flatwise and torsion are the two the moduli were fitted to. Edgewise is in a solution only when the series held an edgewise frequency.",
            asked_by: None,
        },
        number(
            "last_step",
            "natural logarithm",
            "The larger step in ln E and ln G at the last iteration.",
        ),
    ],
    invariants: &[
        number("flatwise_energy_balance", "fraction", ""),
        number("edgewise_energy_balance", "fraction", ""),
        number("flatwise_free_end", "fraction", ""),
        number("edgewise_free_end", "fraction", ""),
    ],
    iterations: "Gauss–Newton steps taken. On a refusal it is the count for `NotConverged` and null otherwise.",
    field: field_words,
    answer: "the moduli",
    refusals: REFUSALS,
    door_refusals: DOOR_REFUSALS,
    sections,
    validation: Some(validation),
    example,
    further_examples: &[],
    parts: &[],
    check,
    solve,
};

/// Each `ModulusReject` variant with the sentence from its doc comment. The
/// first four are shown by the series itself and are not sealed.
const REFUSALS: &[Refusal] = &[
    Refusal {
        name: "RepeatedMode",
        meaning: "The series names one mode twice.",
        sealed: false,
    },
    Refusal {
        name: "FrequencyNotPositive",
        meaning: "A frequency of the series is not a finite number above zero.",
        sealed: false,
    },
    Refusal {
        name: "YoungsModulusNotIdentifiable",
        meaning: "Not identifiable: the series holds no flatwise frequency, and Young's modulus is recovered from that mode.",
        sealed: false,
    },
    Refusal {
        name: "ShearModulusNotIdentifiable",
        meaning: "Not identifiable: the series holds no torsion frequency, and bending alone fixes the shear modulus only to 19 to 27 % of its value.",
        sealed: false,
    },
    Refusal {
        name: "OutsideSearchRange",
        meaning: "A trial `E` or `G` left `YOUNGS_MODULUS_SEARCH_PA` or `SHEAR_MODULUS_SEARCH_PA`.",
        sealed: true,
    },
    Refusal {
        name: "FrequencyEquationNotBracketed",
        meaning: "Outside the forward model's range: at a trial `E` and `G` the frequency equation of a bending mode has no root between `LAMBDA_BRACKET_LOW` and `LAMBDA_BRACKET_HIGH` of the Euler–Bernoulli value.",
        sealed: true,
    },
    Refusal {
        name: "NonFiniteValue",
        meaning: "An intermediate value was NaN or infinite.",
        sealed: true,
    },
    Refusal {
        name: "EnergyBalanceBroken",
        meaning: "Peak strain energy and peak kinetic energy of a computed bending mode differ by more than `ENERGY_BALANCE_TOLERANCE`.",
        sealed: true,
    },
    Refusal {
        name: "FreeEndBroken",
        meaning: "A computed bending mode leaves more than `FREE_END_TOLERANCE` of moment or shear force at the far end.",
        sealed: true,
    },
    Refusal {
        name: "NotConverged",
        meaning: "The iteration cap was reached with the step tolerance still open.",
        sealed: true,
    },
    Refusal {
        name: "RoundTripBroken",
        meaning: "The model at the recovered `E` and `G` does not give back a frequency it was fitted to within `ROUND_TRIP_TOLERANCE`.",
        sealed: true,
    },
    Refusal {
        name: "PoissonRatioOutOfRange",
        meaning: "`E / (2 G) − 1` is not strictly between 0 and 0.5: the frequencies are not those of a uniform bar that is the same in every direction.",
        sealed: true,
    },
];

/// What this entry names itself: a bar that is not one of the four, and a
/// series with a fourth frequency, which has no mode to be. Neither reaches
/// the solver.
const DOOR_REFUSALS: &[Refusal] = &[
    Refusal {
        name: "UnknownBar",
        meaning: "`bar` is not one of the four bars the solver holds: ERX1, ERX2, ERY1, ERY2.",
        sealed: false,
    },
    Refusal {
        name: "TooManyFrequencies",
        meaning: "`frequencies_hz` holds more than three frequencies: a series is flatwise, torsion, edgewise.",
        sealed: false,
    },
];

/// The `ModulusReject` variant name, without its fields.
pub fn reject_name(reject: &ModulusReject) -> &'static str {
    match reject {
        ModulusReject::RepeatedMode { .. } => "RepeatedMode",
        ModulusReject::FrequencyNotPositive { .. } => "FrequencyNotPositive",
        ModulusReject::YoungsModulusNotIdentifiable => "YoungsModulusNotIdentifiable",
        ModulusReject::ShearModulusNotIdentifiable => "ShearModulusNotIdentifiable",
        ModulusReject::OutsideSearchRange { .. } => "OutsideSearchRange",
        ModulusReject::FrequencyEquationNotBracketed { .. } => "FrequencyEquationNotBracketed",
        ModulusReject::NonFiniteValue => "NonFiniteValue",
        ModulusReject::EnergyBalanceBroken { .. } => "EnergyBalanceBroken",
        ModulusReject::FreeEndBroken { .. } => "FreeEndBroken",
        ModulusReject::NotConverged { .. } => "NotConverged",
        ModulusReject::RoundTripBroken { .. } => "RoundTripBroken",
        ModulusReject::PoissonRatioOutOfRange { .. } => "PoissonRatioOutOfRange",
    }
}

fn field_words() -> String {
    format!(
        "SHA-256 of the sample count ({}) as 8 little-endian bytes, then the two bending modes at the recovered moduli, flatwise then edgewise, as little-endian f64 bytes: deflection over length, rotation, scaled moment and scaled shear force at each of {} stations along the bar.",
        2 * 4 * (modulus::MODE_INTERVALS + 1),
        modulus::MODE_INTERVALS + 1
    )
}

fn range(bounds: (f64, f64)) -> Value {
    json!({ "minimum": bounds.0, "maximum": bounds.1 })
}

fn sections() -> Sections {
    let bars: Vec<Value> = modulus::BARS
        .iter()
        .map(|bar| {
            let constants = bar.constants();
            json!({
                "bar": bar.name(),
                "mass_kg": constants.mass_kg,
                "length_m": constants.length_m,
                "width_m": constants.width_m,
                "thickness_m": constants.thickness_m,
            })
        })
        .collect();
    Sections {
        locked: json!({
            "bars": bars,
            "series_order": SERIES_ORDER.iter().map(|mode| mode.name()).collect::<Vec<_>>(),
            "compared_frequency_range_hz": {
                "flatwise": range(modulus::FLATWISE_RANGE_HZ),
                "torsion": range(modulus::TORSION_RANGE_HZ),
                "edgewise": range(modulus::EDGEWISE_RANGE_HZ),
            },
            "model": "Bending is a Timoshenko beam with free ends and the shear coefficient 5 (1 + ν) / (6 + 5 ν), ν = E / (2 G) − 1. Torsion is Saint-Venant's for a rectangle with the end correction of ASTM E1875. The model holds no density, no modulus and no Poisson's ratio of steel.",
            "not_returned": "Poisson's ratio. It is a small difference of two larger numbers; on these bars its 95 percent interval is 13 percent of its value, so it is not reported as a property.",
            "start_youngs_modulus_pa": modulus::START_YOUNGS_MODULUS_PA,
            "start_shear_modulus_pa": modulus::START_SHEAR_MODULUS_PA,
            "max_iterations": modulus::MAX_ITERATIONS,
            "jacobian_step": modulus::JACOBIAN_STEP,
            "youngs_modulus_search_pa": range(modulus::YOUNGS_MODULUS_SEARCH_PA),
            "shear_modulus_search_pa": range(modulus::SHEAR_MODULUS_SEARCH_PA),
            "euler_bernoulli_free_free_root": modulus::EULER_BERNOULLI_FREE_FREE_ROOT,
            "lambda_bracket_low": modulus::LAMBDA_BRACKET_LOW,
            "lambda_bracket_high": modulus::LAMBDA_BRACKET_HIGH,
            "lambda_bisections": modulus::LAMBDA_BISECTIONS,
            "torsion_series_terms": modulus::TORSION_SERIES_TERMS,
            "mode_intervals": modulus::MODE_INTERVALS,
            "edgewise_line_p2_fraction": modulus::EDGEWISE_LIMIT_FRACTION,
            "youngs_modulus_method_uncertainty": modulus::YOUNGS_MODULUS_METHOD_UNCERTAINTY,
            "shear_modulus_method_uncertainty": modulus::SHEAR_MODULUS_METHOD_UNCERTAINTY,
            "shear_modulus_method_uncertainty_ceiling": modulus::SHEAR_MODULUS_METHOD_UNCERTAINTY_CEILING,
        }),
        invariants: json!([
            {
                "name": "flatwise_energy_balance",
                "rule": "Peak strain energy equals peak kinetic energy of the computed flatwise mode.",
                "tolerance": modulus::ENERGY_BALANCE_TOLERANCE,
                "unit": "fraction of the strain energy",
                "compared": "every iteration",
                "refusal": "EnergyBalanceBroken",
            },
            {
                "name": "flatwise_free_end",
                "rule": "The far end of the computed flatwise mode carries no moment and no shear force.",
                "tolerance": modulus::FREE_END_TOLERANCE,
                "unit": "fraction of the largest value in the bar",
                "compared": "every iteration",
                "refusal": "FreeEndBroken",
            },
            {
                "name": "edgewise_energy_balance",
                "rule": "The same energy balance for the edgewise mode.",
                "tolerance": modulus::ENERGY_BALANCE_TOLERANCE,
                "unit": "fraction of the strain energy",
                "compared": "at acceptance",
                "refusal": "EnergyBalanceBroken",
            },
            {
                "name": "edgewise_free_end",
                "rule": "The same free end for the edgewise mode.",
                "tolerance": modulus::FREE_END_TOLERANCE,
                "unit": "fraction of the largest value in the bar",
                "compared": "at acceptance",
                "refusal": "FreeEndBroken",
            },
        ]),
        convergence: json!({
            "step_tolerance": modulus::STEP_TOLERANCE,
            "round_trip_tolerance": modulus::ROUND_TRIP_TOLERANCE,
            "rule": "The larger step in ln E and ln G must be inside the step tolerance, and the model at the returned moduli must give back the flatwise and the torsion frequency inside the round-trip tolerance, before a result is returned.",
            "refusal": "NotConverged",
        }),
        reference: json!({
            "name": modulus::REFERENCE_NAME,
            "url": modulus::REFERENCE_URL,
            "retrieved": modulus::REFERENCE_RETRIEVED,
            "tolerance_fraction": modulus::REFERENCE_TOLERANCE_FRACTION,
            "test": "bam_published_moduli_of_erx1_and_erx2",
            "covers": "Young's modulus and shear modulus of ERX1 and ERX2, two scans each, against the values the laboratory itself computed from the same frequencies with the formulas of ASTM E1875; and the measured edgewise frequency, which the recovery never uses, within 0.25 percent.",
            "kind": "Measured frequencies and an accredited laboratory's own reduction of them. The laboratory's values use the same frequencies, so they test the calculation and are not an independent measurement of the modulus.",
        }),
    }
}

/// The validation record: the object, the data, the pass line, the result
/// and the limits. The comparison of 2026-10-09 is stated as it came out.
fn validation() -> [String; 5] {
    let object = "Four bars about 100 mm x 9 mm x 3 mm and 21.3 g, cut from one hot-rolled sheet of structural steel S355J2+N (material number 1.0577) and rung at 24 °C at the Bundesanstalt für Materialforschung und -prüfung (BAM), Berlin, order 5.22/430: ERX1 and ERX2 along the rolling direction, ERY1 and ERY2 across it. The solver holds each bar's size and mass and a model of a free ringing bar; it holds no property of steel.".to_string();
    let data = "Schilling, Glaubitz, Matzak, Rehmer, Skrotzki, Zenodo, doi 10.5281/zenodo.6778336, version 1.0.0, CC BY 4.0: for each bar two scans of three resonant frequencies (ASTM E1875), and tensile tests of the same sheet in both directions (ISO 6892-1). The model was chosen on ERX1, ERX2 and the tensile tests along the rolling direction. ERY1, ERY2 and the tensile tests across it were held back and opened once, on 2026-10-09, after the solver was served.".to_string();
    let pass_line = "Fixed before the held-back data were opened (analysis/prereg.json, SHA-256 e606a12456dfb449387a8a3c627401ebbc087166264b885b336b466f46dcec2d, and its amendment 2, SHA-256 ea32c1e8b46f5191316dc9b53dc30f2105e1e86a7600d422b4fa8da5744589da). P1: Young's modulus against the elastic slope of the tensile tests in the same direction, within 2 sqrt(0.5² + (s/√n)² + 0.577² + 0.115² + 0.577²) percent. P2: the edgewise frequency, which the recovery never uses, predicted within 0.25 percent on all four scans. P3: the moduli of the four scans within 1.41 percent (E) and 4.24 percent (G) of each other. P5: energy balance and free end within 1e-8. P6: E and G within 0.10 percent of the values the laboratory computed from the same frequencies. P7: every answer converged and its proof verified. The model passes only if P1 to P5 are all met.".to_string();
    let result = "Compared with the held-back bars on 2026-10-09, and by the rule fixed beforehand the comparison failed: P1 was missed and every other line was met. P1: 215.68 GPa recovered against 210.25 GPa from the tensile slope, 2.58 percent apart against a line of 2.49. The miss is 0.094 of a percentage point, 0.2 GPa, and it depends on the stress band the tensile slope is taken over: over 100 to 140, 140 to 180 or 100 to 200 MPa the line is met (2.10, 2.26 and 2.24 percent against 2.48, 2.38 and 2.43), and over 60 to 100, 50 to 150, 180 to 220 or 50 to 250 MPa it is missed. By the rule as written P1 across the rolling direction missed; in substance it is neither a miss nor a pass. P2: worst of four 0.114 percent. Two of the four edgewise readings behind P2 are the same number, 4755.130 Hz for both ERY1 and ERY2 in the laboratory's whole-spectrum column, most likely one entry carried from one workbook to the other; against each bar's own single-scan reading the two predictions are −0.129 percent (ERY1) and −0.027 percent (ERY2), inside the line. P3: 0.46 percent in E, 0.60 percent in G. P5: 9.2e-11. P6: worst 0.053 percent. P6 is arithmetic: the laboratory's values are the standard's formulas on the same frequencies, sizes and mass, and the solver's distance from them is fixed by the bar's shape, 0.032 percent in G and 0.051 to 0.053 percent in E, whatever the bar rang at. The sizes were opened by amendment 2 before the frequencies, so P6 was not a blind test. P7: four of four. On the bars the model was chosen on, P1 had missed by more: 211.6 GPa against 197.0 GPa, 7.4 percent apart against a line of 2.29. The scored runs were made with a build of the uncommitted tree, 19 minutes before the commits the record names for it existed and with the same solver source; later builds re-made every answer bit for bit. A second scoring of 2026-10-09, on the same machine and from the scorer's own cut of the workbooks, found the same numbers, with one difference in the third digit (P3 for G, 0.598 against 0.601 percent): evidence/verification/second_party/V-36/.".to_string();
    let limits = "No agreement with an independent measurement of the modulus is claimed. The only other measurement of this sheet's modulus is the slope of strength tests, taken over 25 to 27 micrometres of extensometer travel: the line of 2.3 to 2.5 percent is 0.6 micrometres there, the miss along the rolling direction is 2.0 micrometres and across it 0.65, and along the rolling direction the slope itself moves by up to 8 percent within one test. Across the rolling direction the slope moves 1.4 to 2.0 percent within a test, between 60 and 220 MPa, and the scatter is between test pieces: 204.5 to 213.4 GPa. The tensile slopes of the two directions differ by 6.7 percent where the resonances differ by 1.9. That reference cannot decide at the line it was given, in either direction. With the extensometer term rebuilt from a fixed 0.6 micrometres at each end of a 25 micrometre window, the same formula gives a line near 4.6 percent, next to the protocol's own 5 percent \"too loose to decide\". A handbook 210 GPa is 0.12 percent from the mean of the tensile slopes across the rolling direction, where the solver is 2.58 percent from it. The comparison supports nothing either way. What was shown is narrower: the solver reproduces an accredited laboratory's own values on two bars whose frequencies had been held back, and predicts a resonance it was not given. The first is arithmetic: the same formula on the same readings, with the offset fixed by the bar's shape; it is not a blind test. The second, the edgewise frequency within 0.114 percent, tests the shear and rotary-inertia correction of the bending model and the width and thickness readings. It does not test the absolute value of Young's modulus: all three frequencies scaled by one factor leave the residual unchanged. It does not test the shear modulus (3 percent in G moves the prediction 0.027 percent), the torsion end correction (0.006 percent) or the choice of shear coefficient (5/6 also stays inside 0.25 percent). The standard's formula with a handbook Poisson's ratio of 0.29 predicts the same four readings within 0.082 percent. The solver cannot tell the bar name from the frequencies: ERY1's frequencies sent under the name ERX1 converge, to 216.32 GPa, because the bars differ by less than 0.2 percent in size. The compared ranges are the envelope of the eight scans compared with measurement, two on each of the four bars: flatwise 1587.428 to 1608.28 Hz, torsion 8899.446 to 8966.189 Hz, and edgewise from 4695.035 Hz, the lowest edgewise frequency those scans predict, to 4755.856 Hz, the highest one measured. Inside them the answers run from 210.7 to 216.3 GPa in E and from 79.0 to 80.8 GPa in G. The ranges are one envelope for all four bars: a flatwise reading of one bar paired with a torsion reading of another, under either name, was not itself compared. Any finite frequency above zero is answered, wherever the beam and torsion models find an isotropic bar that rings at it, and a supplied edgewise frequency is answered with its residual as it is, past the 0.25 percent of line P2 too. Through engine 0.6.0 the ranges were flatwise 1200 to 2000 Hz, torsion 6700 to 11200 Hz and edgewise 3600 to 5900 Hz, which answered E from 120 to 334 GPa, and a supplied edgewise frequency was accepted up to 1 percent from the prediction; engines 0.7.0 and 0.8.0 refused outside the compared ranges and past line P2. Four bars of one sheet at 24 °C. The shear modulus has no second measurement of any kind. The data set's own summary sheet states ±1 percent (95 percent confidence) for both E and G at room temperature, worked out by the GUM from the uncertainties of the measured inputs, and that is the uncertainty returned for both moduli. The laboratory's later paper on three other alloys, Rehmer et al., Sci Data 10, 474 (2023), states for the same method at most 1 percent for E and at most 3 percent for G at room temperature; the 3 percent is returned beside the shear modulus as shear_modulus_method_uncertainty_ceiling_pa. Through engine 0.5.0 the 3 percent was returned as the shear modulus's uncertainty. Poisson's ratio is not returned. The modulus is the fast (adiabatic) one, about 0.24 percent above the slow one. A proof shows that an answer was not changed, not that the model matches the steel.".to_string();
    [object, data, pass_line, result, limits]
}

/// The first published scan of ERX1: flatwise, torsion, edgewise.
fn example() -> Value {
    json!({ "bar": "ERX1", "frequencies_hz": [1588.627, 8950.895, 4700.015] })
}

/// The bar and the series of a request, as the solver takes them.
fn question(given: &Given) -> Result<(Bar, Vec<Reading>), String> {
    let name = given.word("bar")?;
    let bar = Bar::from_name(name).ok_or_else(|| {
        format!("Physical Violation: the solver's entry lists the bar '{name}', and the solver has no such bar. Nothing is computed.")
    })?;
    let frequencies = given.numbers("frequencies_hz")?;
    if frequencies.len() > SERIES_ORDER.len() {
        return Err(format!(
            "Physical Violation: 'frequencies_hz' holds {} frequencies, and a series is at most {}: flatwise, torsion, edgewise (TooManyFrequencies).",
            frequencies.len(),
            SERIES_ORDER.len()
        ));
    }
    let series = frequencies
        .iter()
        .zip(SERIES_ORDER)
        .map(|(frequency_hz, mode)| Reading {
            mode,
            frequency_hz: *frequency_hz,
        })
        .collect();
    Ok((bar, series))
}

fn door_sentence(reject: &ModulusReject) -> String {
    format!("Physical Violation: {reject} ({}).", reject_name(reject))
}

/// Every limit the series itself shows, asked of the solver's own check
/// before a physics permit is taken: each frequency inside the range of its
/// mode, and enough of them to fix both moduli.
fn check(given: &Given) -> Result<(), String> {
    let (_, series) = question(given)?;
    modulus::check_series(&series).map_err(|reject| door_sentence(&reject))
}

fn solve(given: &Given) -> Result<Solved, String> {
    let (bar, series) = question(given)?;
    Ok(solved(modulus::solve(bar, &series)))
}

/// What the solver returned, in the words every solver shares. A refusal the
/// series itself shows is answered with no seal and with the solver's
/// sentence alone, so its sentence carries the solver's name for it.
pub fn solved(returned: Result<BarModuli, ModulusReject>) -> Solved {
    let moduli = match returned {
        Ok(moduli) => moduli,
        Err(reject) => {
            let name = reject_name(&reject);
            let sealed = REFUSALS
                .iter()
                .any(|refusal| refusal.name == name && refusal.sealed);
            return Solved::Refused(Refused {
                name,
                sentence: if sealed {
                    reject.to_string()
                } else {
                    format!("{reject} ({name})")
                },
                iterations: match reject {
                    ModulusReject::NotConverged { iterations } => Some(u64::from(iterations)),
                    _ => None,
                },
            });
        }
    };
    let number = |name: &str, value: f64| (name.to_string(), Published::Number(value));
    let mut result = vec![
        number("youngs_modulus_pa", moduli.youngs_modulus_pa),
        number("shear_modulus_pa", moduli.shear_modulus_pa),
        number(
            "youngs_modulus_method_uncertainty_pa",
            moduli.youngs_modulus_method_uncertainty_pa,
        ),
        number(
            "shear_modulus_method_uncertainty_pa",
            moduli.shear_modulus_method_uncertainty_pa,
        ),
        number(
            "shear_modulus_method_uncertainty_ceiling_pa",
            moduli.shear_modulus_method_uncertainty_ceiling_pa,
        ),
        number("edgewise_predicted_hz", moduli.edgewise_predicted_hz),
        number("flatwise_residual_fraction", moduli.flatwise_residual_fraction),
        number("torsion_residual_fraction", moduli.torsion_residual_fraction),
    ];
    if let Some(residual) = moduli.edgewise_residual_fraction {
        result.push(number("edgewise_residual_fraction", residual));
    }
    result.push(number("last_step", moduli.last_step));
    Solved::Answer(Answer {
        iterations: u64::from(moduli.iterations),
        result,
        invariants: vec![
            number("flatwise_energy_balance", moduli.flatwise_energy_balance),
            number("edgewise_energy_balance", moduli.edgewise_energy_balance),
            number("flatwise_free_end", moduli.flatwise_free_end),
            number("edgewise_free_end", moduli.edgewise_free_end),
        ],
        field: moduli.mode_shapes,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::solvers::describe_value;

    const SOLVER_SOURCE: &str = include_str!("../../genesis_solvers/src/modulus_bam_s355.rs");

    /// `(variant, doc sentence)` for every variant of `ModulusReject`, read
    /// from the solver's source. An intra-doc link is written as its bare
    /// code span.
    fn documented_rejects() -> Vec<(String, String)> {
        let start = SOLVER_SOURCE
            .find("pub enum ModulusReject {")
            .expect("ModulusReject is in the solver");
        let mut found = Vec::new();
        let mut doc = String::new();
        for line in SOLVER_SOURCE[start..].lines().skip(1) {
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
        assert_eq!(documented.len(), 12);
        assert_eq!(documented.len(), REFUSALS.len());
        for ((variant, sentence), refusal) in documented.iter().zip(REFUSALS) {
            assert_eq!(variant, refusal.name);
            assert_eq!(sentence, refusal.meaning, "{}", refusal.name);
        }
        // The four the series itself shows are answered with no seal.
        let body = describe_value(MODULUS_BAM_S355.name).expect("described");
        let refusals = body["refusals"].as_array().expect("refusals");
        // After the solver's 12 the description lists
        // the two this entry names itself, each answered with no seal.
        assert_eq!(refusals.len(), 14);
        for (index, refusal) in refusals.iter().enumerate() {
            let reported = if index < 4 || index >= 12 { "422, no seal" } else { "200, outcome refused, sealed" };
            assert_eq!(refusal["reported_as"], reported, "{}", refusal["name"]);
        }
        for (refusal, own) in refusals[12..].iter().zip(DOOR_REFUSALS) {
            assert_eq!(refusal["name"], own.name);
            assert_eq!(refusal["meaning"], own.meaning);
        }
        assert_eq!(
            DOOR_REFUSALS.iter().map(|own| own.name).collect::<Vec<_>>(),
            ["UnknownBar", "TooManyFrequencies"]
        );
    }

    #[test]
    fn the_reference_is_the_one_stored_in_the_solver_test() {
        let body = describe_value(MODULUS_BAM_S355.name).expect("described");
        let url = body["reference"]["url"].as_str().expect("url");
        let retrieved = body["reference"]["retrieved"].as_str().expect("retrieved");
        assert_eq!(url, "https://doi.org/10.5281/zenodo.6778336");
        assert!(SOLVER_SOURCE.contains(&format!("<{url}>")), "{url}");
        assert!(SOLVER_SOURCE.contains(&format!("Read on {retrieved}")), "{retrieved}");
        let test = body["reference"]["test"].as_str().expect("test");
        assert!(SOLVER_SOURCE.contains(&format!("fn {test}()")), "{test}");
    }

    #[test]
    fn the_bars_and_the_ranges_published_are_the_solvers() {
        let names: Vec<&str> = modulus::BARS.iter().map(|bar| bar.name()).collect();
        assert_eq!(names, BAR_NAMES);
        // The compared ranges of the three modes do not meet; the field list
        // takes every finite frequency above zero.
        let (flat, edge, torsion) = (
            modulus::FLATWISE_RANGE_HZ,
            modulus::EDGEWISE_RANGE_HZ,
            modulus::TORSION_RANGE_HZ,
        );
        assert!(flat.1 < edge.0 && edge.1 < torsion.0);
        match MODULUS_BAM_S355.inputs[1].kind {
            Kind::Numbers { minimum, maximum, fewest, most, .. } => {
                assert_eq!((minimum, maximum), (f64::MIN_POSITIVE, f64::MAX));
                assert_eq!((fewest, most), (0, 3));
            }
            _ => panic!("the series is an array of numbers"),
        }
        for mode in RESIDUAL_MODES {
            assert_eq!(
                MODULUS_BAM_S355.published_shape("result", &format!("{mode}_residual_fraction_hex")),
                Some(Shape::Number)
            );
        }
        assert_eq!(MODULUS_BAM_S355.published_shape("result", "poisson_ratio"), None);
        assert_eq!(MODULUS_BAM_S355.published_shape("result", "longitudinal_residual_fraction"), None);
    }
}
