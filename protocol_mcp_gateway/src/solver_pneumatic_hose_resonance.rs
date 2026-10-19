//! The entry for `pneumatic_hose_resonance`: the company's own polyamide
//! hose feeding a closed volume, its modes and its response. The numbers
//! come from `genesis_solvers` constants.

use genesis_solvers::pneumatic_hose_resonance::{self as hose, HoseQuestion, HoseReject, HoseResponse};
use serde_json::{Value, json};

use crate::solvers::{
    Answer, Given, Input, Kind, Output, Published, Refusal, Refused, Sections, Shape, Solved,
    Solver,
};

const PRESSURE_MIN_PA: f64 = hose::PRESSURE_MIN_PA;
const FREQUENCY_MAX_HZ: f64 = hose::FREQUENCY_MAX_HZ;

const fn one(name: &'static str, unit: &'static str, note: &'static str) -> Output {
    Output { name, unit, shape: Shape::Number, each: &[], note, asked_by: None }
}

const fn many(name: &'static str, unit: &'static str, note: &'static str) -> Output {
    Output { name, unit, shape: Shape::Numbers, each: &[], note, asked_by: None }
}

pub const PNEUMATIC_HOSE_RESONANCE: Solver = Solver {
    name: "pneumatic_hose_resonance",
    summary: SUMMARY,
    inputs: &[
        Input {
            name: "mean_pressure_pa",
            kind: Kind::Number { minimum: PRESSURE_MIN_PA, maximum: hose::ATMOSPHERE_PA + hose::TUBE_RATING_GAUGE_PA, unit: "Pa" },
            refusal: Some("PressureAboveTubeRating"),
            bound: "Absolute mean pressure of the air in the hose and the volume, at rest at 293.15 K. At most 1,101,325 Pa, the tube's rating of 1.0 MPa over the atmosphere (PressureAboveTubeRating); at least 170.01457281119065 Pa, the published minimum to the 64-bit number, where the air's mean free path is a hundredth of the 4 mm bore, Knudsen number 0.01, and slip flow begins at the wall: the model's wall holds the air with no slip (RarefiedFlow).",
            only_when: None,
            asked_by_check: true,
        },
        Input {
            name: "frequencies_hz",
            kind: Kind::NumbersWithin {
                fewest: 0,
                most: hose::FREQUENCIES_MAX,
                minimum: 0.0,
                minimum_exclusive: true,
                maximum: FREQUENCY_MAX_HZ,
                maximum_exclusive: true,
                unit: "Hz",
            },
            refusal: Some("AboveFirstCrossMode"),
            bound: "Frequencies at which the volume's pressure over the inlet's is asked, up to 64 (TooManyFrequencies), each above 0 (FrequencyNotPositive) and below the cut-on of the first mode across the bore, 1.8412 c/(pi D) = 50289.2338551663 Hz, the published maximum to the 64-bit number, itself refused, where the waves stop being plane (AboveFirstCrossMode); and at most 1.0544 Hz for each pascal of the mean pressure, 2 pi mu R Kn^2 d^4 p/(k_B^2 T) with Kn 0.01, where the oscillating viscous layer at the wall, sqrt(2 nu/omega), is a hundred mean free paths thick and slip flow begins in it: the model's wall holds the air with no slip (StokesLayerSlip). That limit is the lower of the two below 47,696 Pa: 179 Hz at the lowest pressure, 106.8 kHz at the atmosphere's. May be empty: the modes and peaks are returned either way.",
            only_when: None,
            asked_by_check: true,
        },
    ],
    kept_inside: "The hose, its wall, the volume, the air's constants and the tolerances stay inside the solver.",
    result: &[
        one("sound_speed_m_s", "m/s", "Speed of sound in the air at 293.15 K."),
        one("first_cross_mode_hz", "Hz", "Cut-on of the first mode across the bore: the top of the plane-wave model."),
        many("lossless_frequencies_hz", "Hz", "The six modes of the lossless, adiabatic line and volume: k L tan(k L) = V_line / V."),
        many("mode_numbers", "1", "The number, from 1, of each mode that still rings at this pressure: the highest ones, all six above 8,625 Pa, the sixth alone from 604.7 to 751.8 Pa; below 604.7 Pa every mode is overdamped and the list is empty."),
        many("natural_frequencies_hz", "Hz", "Each ringing mode's damped natural frequency, the imaginary part of its pole over 2 pi."),
        many("damping_ratios", "1", "Each ringing mode's damping ratio, minus the real part of its pole over its modulus."),
        many("peak_frequencies_hz", "Hz", "The frequencies of the peaks of the volume's pressure over the inlet's, up to six, lowest first."),
        many("peak_gains", "1", "The gain at each peak."),
        many("gains", "1", "The volume's pressure amplitude over the inlet's at each asked frequency."),
        many("phases_rad", "rad", "Its phase at each asked frequency."),
        many("inlet_power_w", "W", "Mean power into the inlet for 1 Pa of inlet pressure, at each asked frequency."),
        many("line_dissipation_w", "W", "Mean power the hose dissipates by viscosity and heat exchange, at each asked frequency."),
        many("volume_power_w", "W", "Mean power the volume takes by heat exchange with its wall, at each asked frequency."),
    ],
    invariants: &[
        one("largest_energy_ledger_residual", "1", "Largest, over the peaks and the asked frequencies, of the inlet's power less the line's dissipation and the volume's, over half the inlet pressure times the inlet flow."),
        one("largest_volume_ledger_residual", "1", "Largest, over the same frequencies, of the inlet flow less the flow into the volume and the flow the line's air takes up, over the inlet flow."),
    ],
    iterations: "Evaluations of the line's transfer function: the Newton iterations of the poles, the scan for the peaks and the ledgers. On a refusal it is null.",
    field: field_words,
    answer: "a response",
    refusals: REFUSALS,
    door_refusals: &[],
    sections,
    validation: Some(validation),
    example,
    further_examples: &[crate::solvers::FurtherExample { key: "example_solve_low_pressure", parameters: example_low }],
    parts: &[],
    check,
    solve,
};

const SUMMARY: &str = "Company specimen: the pneumatic resonance of a 2.0 m polyamide hose (6 x 4 mm) feeding a closed 20 cm^3 volume, air at 293.15 K about a mean pressure: the dissipative laminar line of Nichols and Brown (viscous and thermal Bessel functions, the tube's stretch, the volume's heat exchange), its natural modes as poles with their damping ratios, the response's peaks, and the volume's pressure over the inlet's at asked frequencies; energy and volume-flow ledgers closed by quadrature along the standing wave and passivity held at the peaks and the asked frequencies; stops by name at the first mode across the bore (50,289 Hz), where slip flow begins at the wall, in the bore or in the oscillating viscous layer (1.0544 Hz for each pascal of mean pressure), and at the tube's rating; held to Poiseuille and to Kirchhoff's wide-tube attenuation.";

const REFUSALS: &[Refusal] = &[
    Refusal { name: "InputNotFinite", meaning: "An input was NaN or infinite.", sealed: false },
    Refusal { name: "PressureAboveTubeRating", meaning: "The mean pressure is above the tube's rating, `TUBE_RATING_GAUGE_PA` over the atmosphere.", sealed: false },
    Refusal { name: "RarefiedFlow", meaning: "The mean pressure is below `PRESSURE_MIN_PA`, where the mean free path is a hundredth of the bore, Knudsen number `KNUDSEN_LIMIT`, and slip flow begins at the wall: the model's wall holds the air with no slip.", sealed: false },
    Refusal { name: "TooManyFrequencies", meaning: "More frequencies were asked than `FREQUENCIES_MAX`.", sealed: false },
    Refusal { name: "FrequencyNotPositive", meaning: "An asked frequency is not above zero.", sealed: false },
    Refusal { name: "AboveFirstCrossMode", meaning: "An asked frequency is at or above `FREQUENCY_MAX_HZ`, the cut-on of the first mode across the bore, where the waves are no longer plane.", sealed: false },
    Refusal { name: "StokesLayerSlip", meaning: "An asked frequency is above `stokes_slip_hz` at the mean pressure, 1.0544 Hz for each pascal, where the oscillating viscous layer at the wall is a hundred mean free paths thick and slip flow begins in it: the model's wall holds the air with no slip.", sealed: false },
    Refusal { name: "ModeAboveFirstCrossMode", meaning: "A natural mode lies at or above the cut-on of the first mode across the bore.", sealed: true },
    Refusal { name: "ModeAboveStokesLayerSlip", meaning: "A ringing mode's pole, by its modulus over 2 pi, lies above `stokes_slip_hz` at the mean pressure. No admitted pressure reaches it: no mode rings below 604.7 Pa, and every ringing pole's modulus lies under 0.53 of the limit.", sealed: true },
    Refusal { name: "PoleNotSettled", meaning: "A pole of the response did not settle in `MAX_POLE_ITERATIONS` Newton iterations.", sealed: true },
    Refusal { name: "PoleNotFollowed", meaning: "A mode's pole could not be followed from the tube's rating to the mean pressure: at a step of pressure of `TRACK_STEP_MIN` in its logarithm it still left the pole foreseen by more than `TRACK_SHARE` of its modulus.", sealed: true },
    Refusal { name: "ModesOnOnePole", meaning: "Two modes, followed from the tube's rating, settled on one pole, within `DISTINCT_POLE_SHARE` of its modulus: the following lost one of them.", sealed: true },
    Refusal { name: "NonFiniteValue", meaning: "A value was NaN or infinite.", sealed: true },
    Refusal { name: "PassivityBroken", meaning: "The line or the volume would give energy at a frequency: a resistance or a conductance below zero.", sealed: true },
    Refusal { name: "EnergyLedgerBroken", meaning: "The energy ledger did not close to `LEDGER_RESIDUAL_LIMIT` at a frequency.", sealed: true },
    Refusal { name: "VolumeLedgerBroken", meaning: "The volume ledger did not close to `LEDGER_RESIDUAL_LIMIT` at a frequency.", sealed: true },
];

pub fn reject_name(reject: &HoseReject) -> &'static str {
    match reject {
        HoseReject::InputNotFinite { .. } => "InputNotFinite",
        HoseReject::PressureAboveTubeRating { .. } => "PressureAboveTubeRating",
        HoseReject::RarefiedFlow { .. } => "RarefiedFlow",
        HoseReject::TooManyFrequencies { .. } => "TooManyFrequencies",
        HoseReject::FrequencyNotPositive { .. } => "FrequencyNotPositive",
        HoseReject::AboveFirstCrossMode { .. } => "AboveFirstCrossMode",
        HoseReject::StokesLayerSlip { .. } => "StokesLayerSlip",
        HoseReject::ModeAboveFirstCrossMode { .. } => "ModeAboveFirstCrossMode",
        HoseReject::ModeAboveStokesLayerSlip { .. } => "ModeAboveStokesLayerSlip",
        HoseReject::PoleNotSettled { .. } => "PoleNotSettled",
        HoseReject::PoleNotFollowed { .. } => "PoleNotFollowed",
        HoseReject::ModesOnOnePole { .. } => "ModesOnOnePole",
        HoseReject::NonFiniteValue { .. } => "NonFiniteValue",
        HoseReject::PassivityBroken { .. } => "PassivityBroken",
        HoseReject::EnergyLedgerBroken { .. } => "EnergyLedgerBroken",
        HoseReject::VolumeLedgerBroken { .. } => "VolumeLedgerBroken",
    }
}

fn field_words() -> String {
    format!(
        "SHA-384 of the count of the numbers that follow as 8 little-endian bytes, then the gain at each of the {} points of the logarithmic scan from {} Hz to 1.25 times the sixth lossless mode, or to where slip flow begins in the viscous layer at the mean pressure where that is lower, as little-endian f64 bytes.",
        hose::SCAN_POINTS,
        hose::SCAN_LOW_HZ
    )
}

fn sections() -> Sections {
    Sections {
        locked: json!({
            "specimen": "Company specimen: 2.0 m of polyamide 12 tube, 4 mm bore and 1 mm wall (Young's modulus 1.4 GPa, Poisson 0.4), rated to 1.0 MPa over the atmosphere, closed at its end by a 20 cm^3 volume whose wall stays at the air's 293.15 K; dry air, R 287.05 J/(kg K), gamma 1.4, viscosity 1.8205e-5 Pa s, Prandtl number 0.713.",
            "hose_length_m": hose::HOSE_LENGTH_M,
            "hose_bore_m": hose::HOSE_BORE_M,
            "hose_wall_m": hose::HOSE_WALL_M,
            "end_volume_m3": hose::END_VOLUME_M3,
            "end_sphere_radius_m": hose::end_sphere_radius_m(),
            "temperature_k": hose::TEMPERATURE_K,
            "air_viscosity_pa_s": hose::AIR_VISCOSITY_PA_S,
            "air_prandtl": hose::AIR_PRANDTL,
            "wall_compliance_per_pa": hose::wall_compliance_per_pa(),
            "modes": hose::MODES,
            "stokes_slip_hz_per_pa": hose::stokes_slip_hz(1.0),
            "equations": "Per length: Z(s) = rho s / (A (1 - F(r sqrt(-s/nu)))) and Y(s) = A s/(gamma p) [1 + (gamma - 1) F(r sqrt(-s Pr/nu))] + A s D psi/(E e), F(z) = 2 J1(z)/(z J0(z)) (Nichols 1962; Brown 1962), J1/J0 and J2/J1 as a continued fraction for any complex z; Z taken as (8 mu/(A r^2)) over (1 - F)/(-z^2/8), with 1 - F = -J2/J0, so that it is Poiseuille's at low frequency with no difference of near-equal numbers. Gamma = sqrt(Z Y). The volume: Y_v = V s/(gamma p) [1 + (gamma - 1) F_s(a sqrt(s Pr/nu))], F_s(x) = 3(x coth x - 1)/x^2, a sphere of the same volume. G(s) = 1/(cosh(Gamma L) + Z Y_v L sinh(Gamma L)/(Gamma L)), the volume's pressure over the inlet's, the same function as 1/(cosh(Gamma L) + Z_c Y_v sinh(Gamma L)) with Z_c = Z/Gamma. On the frequency axis the flows and powers are taken over omega, so that every term keeps its own size at any frequency asked.",
            "fitted": "Nothing.",
        }),
        invariants: json!([
            {
                "name": "largest_energy_ledger_residual",
                "rule": "For 1 Pa at the inlet: (1/2) Re(P_in Q_in*) = (1/2) integral of Re(Z)|Q|^2 + Re(Y)|P|^2 dx + (1/2) Re(Y_v)|P_v|^2, the integral by eight-point Gauss-Legendre on at least 32 segments and four for each radian of |Gamma L| over pi.",
                "tolerance": hose::LEDGER_RESIDUAL_LIMIT,
                "compared": "at every peak and every asked frequency",
                "refusal": "EnergyLedgerBroken",
            },
            {
                "name": "largest_volume_ledger_residual",
                "rule": "Q_in - Q_v = integral of Y P dx, the volume flow the line's air and wall take up.",
                "tolerance": hose::LEDGER_RESIDUAL_LIMIT,
                "compared": "at every peak and every asked frequency",
                "refusal": "VolumeLedgerBroken",
            },
            {
                "name": "passivity",
                "rule": "Re Z, Re Y and Re Y_v are not negative: the line and the volume take energy and never give it.",
                "compared": "at every peak and every asked frequency",
                "refusal": "PassivityBroken",
            },
        ]),
        convergence: json!({
            "rule": "Each pole by Newton's method on the denominator of G with a central-difference derivative, settled when a step moves it by 1e-13 of its modulus, followed from the tube's rating to the asked pressure in steps of the logarithm of the pressure of at most ln 1.25: each step foresees the pole from the last two, and is taken when the settled pole lies within 2 percent of its modulus of the foreseen; otherwise it is halved and taken again, so that the following does not jump to another mode's pole. A pole whose imaginary part falls under 1e-4 of its modulus has stopped ringing. Two ringing modes on one pole are refused (ModesOnOnePole). The peaks: a 4000-point logarithmic scan, each local maximum refined by 80 steps of golden section.",
            "max_pole_iterations": hose::MAX_POLE_ITERATIONS,
            "pole_settled": hose::POLE_SETTLED,
            "overdamped_imaginary_share": hose::OVERDAMPED_IMAGINARY_SHARE,
            "track_step_max": hose::TRACK_STEP_MAX,
            "track_step_min": hose::TRACK_STEP_MIN,
            "track_share": hose::TRACK_SHARE,
            "distinct_pole_share": hose::DISTINCT_POLE_SHARE,
            "refusal": "PoleNotSettled",
            "run_time": "About 40 ms for 64 frequencies up to the cut-on at the tube's rating, on one core of an Apple M4 Pro.",
        }),
        reference: json!({
            "name": hose::REFERENCE_NAME,
            "url": hose::REFERENCE_URL,
            "retrieved": hose::REFERENCE_RETRIEVED,
            "test": "the_line_meets_poiseuille_and_kirchhoff",
            "covers": "The test holds the line's resistance at 1 mHz to Poiseuille's 8 mu/(pi r^4) within 1e-6, its compliance to the isothermal A/p, and its attenuation at 5 and 20 kHz to Kirchhoff's wide-tube closed form within 1.2 and 0.6 percent.",
            "kind": "Published closed forms: Poiseuille's resistance and Kirchhoff's attenuation in a wide tube.",
        }),
    }
}

fn validation() -> [String; 5] {
    [
        "Company specimen: 2.0 m of polyamide 12 tube, 6 x 4 mm, closed by a 20 cm^3 volume, air at 293.15 K about a mean pressure, driven by a small pressure at the inlet.".to_string(),
        "Company specimen, every constant declared in its description. The model is held to two published closed forms, Poiseuille's resistance and Kirchhoff's attenuation in a wide tube; reference gives the numbers.".to_string(),
        "The ledgers stated under invariants: energy and volume flow closed at every peak and every asked frequency, and the line and the volume never giving energy.".to_string(),
        crate::descriptor::OWN_SPECIMEN_SOLVER_RESULT.to_string(),
        "The hose's constants are the company's, declared in its description. The air is at rest about its mean pressure, the oscillation small and laminar, the waves plane; the tube's wall is elastic with no loss of its own; the volume is taken as a sphere of the same volume for its heat exchange. A proof confirms an answer is unchanged; this hose is a company specimen, with no measurement of its own.".to_string(),
    ]
}

fn example() -> Value {
    json!({ "mean_pressure_pa": 6.0e5, "frequencies_hz": [1.0, 24.0, 50.0, 92.0, 200.0, 1000.0] })
}

fn example_low() -> Value {
    json!({ "mean_pressure_pa": 1.5e5, "frequencies_hz": [] })
}

fn question(given: &Given) -> Result<HoseQuestion, String> {
    Ok(HoseQuestion { mean_pressure_pa: given.number("mean_pressure_pa")?, frequencies_hz: given.numbers("frequencies_hz")?.to_vec() })
}

fn check(given: &Given) -> Result<(), String> {
    hose::admit(&question(given)?).map_err(|reject| format!("Physical Violation: {reject} ({}).", reject_name(&reject)))
}

fn solve(given: &Given) -> Result<Solved, String> {
    Ok(solved(hose::solve(&question(given)?)))
}

pub fn solved(returned: Result<HoseResponse, HoseReject>) -> Solved {
    let h = match returned {
        Ok(h) => h,
        Err(reject) => {
            let name = reject_name(&reject);
            let sealed = REFUSALS.iter().any(|refusal| refusal.name == name && refusal.sealed);
            return Solved::Refused(Refused {
                name,
                sentence: if sealed { reject.to_string() } else { format!("{reject} ({name})") },
                iterations: None,
            });
        }
    };
    let one = |name: &str, value: f64| (name.to_string(), Published::Number(value));
    let many = |name: &str, values: &[f64]| (name.to_string(), Published::Numbers(values.to_vec()));
    Solved::Answer(Answer {
        iterations: h.frequencies_computed,
        result: vec![
            one("sound_speed_m_s", h.sound_speed_m_s),
            one("first_cross_mode_hz", h.first_cross_mode_hz),
            many("lossless_frequencies_hz", &h.lossless_frequencies_hz),
            many("mode_numbers", &h.mode_numbers),
            many("natural_frequencies_hz", &h.natural_frequencies_hz),
            many("damping_ratios", &h.damping_ratios),
            many("peak_frequencies_hz", &h.peak_frequencies_hz),
            many("peak_gains", &h.peak_gains),
            many("gains", &h.gains),
            many("phases_rad", &h.phases_rad),
            many("inlet_power_w", &h.inlet_power_w),
            many("line_dissipation_w", &h.line_dissipation_w),
            many("volume_power_w", &h.volume_power_w),
        ],
        invariants: vec![
            one("largest_energy_ledger_residual", h.largest_energy_ledger_residual),
            one("largest_volume_ledger_residual", h.largest_volume_ledger_residual),
        ],
        field: h.field,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const SOLVER_SOURCE: &str = include_str!("../../genesis_solvers/src/pneumatic_hose_resonance.rs");

    fn documented_rejects() -> Vec<(String, String)> {
        let start = SOLVER_SOURCE.find("pub enum HoseReject {").expect("HoseReject is in the solver");
        let body = &SOLVER_SOURCE[start..];
        let mut found = Vec::new();
        let mut doc = String::new();
        for line in body.lines().skip(1) {
            if line == "}" {
                break;
            }
            let Some(text) = line.strip_prefix("    ") else { continue };
            if let Some(sentence) = text.strip_prefix("/// ") {
                if !doc.is_empty() {
                    doc.push(' ');
                }
                doc.push_str(sentence);
            } else if text.starts_with(|first: char| first.is_ascii_uppercase()) {
                let name: String = text.chars().take_while(|letter| letter.is_ascii_alphanumeric()).collect();
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
    }

    #[test]
    fn the_published_bounds_are_the_solvers_limits() {
        // Each published bound is the computed limit to the bit, and its
        // words print it. The floor is taken and the number below it refused
        // by the name the input lists; the cut-on is exclusive, so it is
        // itself refused, and the number below it taken where the viscous
        // layer allows it; zero is exclusive too.
        assert_eq!(PRESSURE_MIN_PA.to_bits(), hose::rarefied_below_pa().to_bits());
        assert_eq!(FREQUENCY_MAX_HZ.to_bits(), hose::first_cross_mode_hz().to_bits());
        assert!(PNEUMATIC_HOSE_RESONANCE.inputs[0].bound.contains(&format!("at least {PRESSURE_MIN_PA} Pa")));
        assert!(PNEUMATIC_HOSE_RESONANCE.inputs[1].bound.contains(&format!("= {FREQUENCY_MAX_HZ} Hz")));
        let ask = |p: f64, f: Vec<f64>| hose::admit(&HoseQuestion { mean_pressure_pa: p, frequencies_hz: f }).map_err(|e| reject_name(&e));
        let rating = hose::tube_rating_pa();
        ask(PRESSURE_MIN_PA, vec![]).unwrap();
        assert_eq!(ask(PRESSURE_MIN_PA.next_down(), vec![]), Err("RarefiedFlow"));
        assert_eq!(ask(rating, vec![FREQUENCY_MAX_HZ]), Err("AboveFirstCrossMode"));
        ask(rating, vec![FREQUENCY_MAX_HZ.next_down(), 50_289.2, f64::MIN_POSITIVE]).unwrap();
        assert_eq!(ask(rating, vec![0.0]), Err("FrequencyNotPositive"));
        assert_eq!(ask(1.0e4, vec![hose::stokes_slip_hz(1.0e4).next_up()]), Err("StokesLayerSlip"));
        ask(1.0e4, vec![hose::stokes_slip_hz(1.0e4)]).unwrap();
        assert_eq!(ask(PRESSURE_MIN_PA, vec![170.5]), Ok(()));
    }

    #[test]
    fn every_published_name_is_in_the_entry() {
        let answer = match solved(hose::solve(&HoseQuestion { mean_pressure_pa: 6.0e5, frequencies_hz: vec![10.0] })) {
            Solved::Answer(answer) => answer,
            Solved::Refused(refused) => panic!("{}", refused.sentence),
        };
        assert_eq!(answer.result.len(), PNEUMATIC_HOSE_RESONANCE.result.len());
        for ((name, value), output) in answer.result.iter().zip(PNEUMATIC_HOSE_RESONANCE.result) {
            assert_eq!(name, output.name);
            assert_eq!(value.shape(), output.shape);
        }
    }
}
