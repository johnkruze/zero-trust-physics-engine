//! The entry for `fusion_pb11_lhd`: proton-boron-11 fusion of one hydrogen
//! neutral beam in the boron-seeded plasma of the Large Helical Device. The
//! numbers come from `genesis_solvers` constants, the sentence of each
//! refusal from the solver's source, and the validation record from the
//! solver's research record.

use genesis_solvers::fusion_pb11_lhd::{self as lhd, LhdBeamFusion, LhdQuestion, LhdReject};
use genesis_solvers::pb11_reaction as reaction;
use serde_json::{Value, json};

use crate::solver_fusion_pb11_shared::{
    EVIDENCE, PASS_LINES_FIXED, RUN_TWICE, SECOND_SCORING, cross_section_record, number,
};
use crate::solvers::{
    Answer, Given, Input, Kind, Published, Refusal, Refused, Sections, Solved, Solver,
};

pub const FUSION_PB11_LHD: Solver = Solver {
    name: "fusion_pb11_lhd",
    summary: "Proton-boron-11 fusion in the boron-seeded hydrogen plasma of the Large Helical Device under one hydrogen neutral beam of 45 keV or of 127.45 to 160.25 keV: alpha particles per second, fusion power, the share of beam protons that fuse, and the alpha emissivity profile. No constant is fitted. It answers only inside the envelope of the inputs compared with measured alpha count rates, each beam family inside its own, and refuses outside it by name. Compared with one published calculation and, in shape only, with measured alpha count rates: it missed three of its four pass lines. It is not validated against measurement.",
    inputs: &[
        // Every limit is asked by the solver's own check, in its order: the
        // beam energy has a gap no range with two ends can say, and so has
        // the boron density, between zero and one nucleus in the plasma.
        Input {
            name: "beam_energy_kev",
            kind: Kind::Number {
                minimum: lhd::POSITIVE_ION_BEAM_ENERGY_KEV,
                maximum: lhd::BEAM_ENERGY_MAX_KEV,
                unit: "keV",
            },
            refusal: Some("BeamEnergyOutOfRange"),
            bound: "Energy of the beam protons. Only the beam energies compared with measurement are accepted: the negative-ion beams at 127.45 to 160.25 keV (Ogawa et al. 2024, Figures 6 and 10) and the positive-ion beam at exactly 45 keV (Figure 6, 7.8 s), compared in that one plasma only. Every energy between 45 and 127.45 keV is refused by name, and so are the 164 to 180 keV beams of the published calculation. One beam is one question; the alphas of several beams add.",
            only_when: None,
            asked_by_check: true,
        },
        Input {
            name: "beam_power_w",
            kind: Kind::Number {
                minimum: lhd::BEAM_POWER_MIN_W,
                maximum: lhd::POSITIVE_ION_BEAM_POWER_W,
                unit: "W",
            },
            refusal: Some("BeamPowerOutOfRange"),
            bound: "Beam power deposited in the plasma: injected less shine-through. The alpha rate is proportional to it. Each beam family is held to the powers compared with measurement under it: a negative-ion beam from 0.9389 MW, the smallest deposited power of Figure 10, to 3.97 MW, the injected power of Figure 6 at 4.5 and 5.0 s; the 45 keV beam at exactly 4.515 MW, its injected power in Figure 6 at 7.8 s. A negative-ion beam above 3.97 MW is inside the published maximum and is refused as BeamPowerOutOfRange: no negative-ion beam was compared there. The runs of Figure 6 gave the injected power, the only power that figure prints.",
            only_when: None,
            asked_by_check: true,
        },
        Input {
            name: "electron_density_per_m3",
            kind: Kind::Number {
                minimum: lhd::ELECTRON_DENSITY_MIN_PER_M3,
                maximum: lhd::ELECTRON_DENSITY_MAX_PER_M3,
                unit: "m^-3",
            },
            refusal: Some("ElectronDensityOutOfRange"),
            bound: "Line-averaged electron density. Each beam family is held to the densities compared with measurement under it, Ogawa et al. 2024, Figures 6 and 10: a negative-ion beam from 2.261e18 to 6.872e18 m^-3, the 45 keV beam at exactly 4.502e18 m^-3. The 1.96e19 m^-3 of the published calculation is outside it.",
            only_when: None,
            asked_by_check: true,
        },
        Input {
            name: "electron_temperature_kev",
            kind: Kind::Number {
                minimum: lhd::POSITIVE_ION_ELECTRON_TEMPERATURE_KEV,
                maximum: lhd::ELECTRON_TEMPERATURE_MAX_KEV,
                unit: "keV",
            },
            refusal: Some("ElectronTemperatureOutOfRange"),
            bound: "Central electron temperature. Each beam family is held to the temperatures compared with measurement under it, Ogawa et al. 2024, Figures 6 and 10: a negative-ion beam from 2.9929 to 3.7872 keV, the 45 keV beam at exactly 2.1252 keV. A negative-ion beam below 2.9929 keV is inside the published minimum and is refused as ElectronTemperatureOutOfRange: no negative-ion beam was compared there. The 5.4 keV of the published calculation is outside it.",
            only_when: None,
            asked_by_check: true,
        },
        Input {
            name: "boron_density_per_m3",
            kind: Kind::Number {
                minimum: 0.0,
                maximum: lhd::BORON_DENSITY_MAX_PER_M3,
                unit: "m^-3",
            },
            refusal: Some("BoronDensityOutOfRange"),
            bound: "Boron density at the peak of its profile, all isotopes. Under a negative-ion beam, zero, or from 0.04973892441153873 m^-3 to the largest compared with measurement: the boron line of Ogawa et al. 2024, Figure 10, times 6.0e13. At 0.04973892441153873 m^-3 the solver's twenty shells hold one boron nucleus in the 25.582 m^3 plasma. A density that is not zero and is below it is refused as BoronBelowOneNucleus: less than one nucleus is no boron, and no boron is asked as zero, which is answered. The smallest density above zero that was compared is 5.1354e16 m^-3. Under the 45 keV beam, exactly 8.1234e16 m^-3, the boron of the one plasma it was compared in. The boron over the line-averaged electron density may not exceed 0.0414, the largest compared (BoronShareOutOfRange). Quasi-neutrality is held in every shell: five times the boron density may not exceed the electron density there. No other input is accepted.",
            only_when: None,
            asked_by_check: true,
        },
    ],
    kept_inside: "The profiles, the cross-section and the tolerances stay inside the solver.",
    result: &[
        number(
            "alpha_emission_rate_per_s",
            "1/s",
            "In the whole plasma: three per reaction.",
        ),
        number("fusion_power_w", "W", "Reactions per second times 8.68 MeV."),
        number("fusion_gain", "1", "Fusion power over deposited beam power, computed as the fused share times 8.68 MeV over the beam energy: it does not pass through the power."),
        number(
            "fused_fraction",
            "1",
            "Share of beam protons that fuse before they stop.",
        ),
        number(
            "peak_alpha_emissivity_per_m3_s",
            "1/(m^3 s)",
            "Of the shell with the most.",
        ),
        number(
            "peak_emissivity_radius",
            "1",
            "rho at the middle of that shell. With no boron, the first shell.",
        ),
        number(
            "slowing_time_s",
            "s",
            "Mean time for a beam proton to slow to 10 keV.",
        ),
        number(
            "energy_fraction_to_electrons",
            "1",
            "Share of the beam energy.",
        ),
        number(
            "energy_fraction_to_ions",
            "1",
            "Share of the beam energy, to protons and boron.",
        ),
    ],
    invariants: &[
        number("proton_ledger_residual", "1", ""),
        number("energy_ledger_residual", "fraction of the beam energy", ""),
    ],
    iterations: "Refinements taken by the shell that took the most. On a refusal it is the count for `NotSettled` and null otherwise.",
    field: field_words,
    answer: "an alpha rate",
    refusals: REFUSALS,
    sections,
    validation: Some(validation),
    example,
    further_examples: &[],
    parts: &[],
    check,
    solve,
};

/// Each `LhdReject` variant with the sentence from its doc comment. The
/// request itself shows a value outside its range, boron under one nucleus
/// in the plasma, boron too great for its electrons and boron that
/// outweighs the electrons, so those eight refusals are not sealed.
const REFUSALS: &[Refusal] = &[
    Refusal {
        name: "BeamEnergyOutOfRange",
        meaning: "`beam_energy_kev` is neither `POSITIVE_ION_BEAM_ENERGY_KEV` nor inside `BEAM_ENERGY_MIN_KEV`..=`BEAM_ENERGY_MAX_KEV`: the beam energies compared with measurement.",
        sealed: false,
    },
    Refusal {
        name: "BeamPowerOutOfRange",
        meaning: "`beam_power_w` is not a power compared with measurement for the beam asked: outside `BEAM_POWER_MIN_W`..=`BEAM_POWER_MAX_W` for a negative-ion beam, or not `POSITIVE_ION_BEAM_POWER_W` for the 45 keV beam.",
        sealed: false,
    },
    Refusal {
        name: "ElectronDensityOutOfRange",
        meaning: "`electron_density_per_m3` is not a density compared with measurement for the beam asked: outside `ELECTRON_DENSITY_MIN_PER_M3`..=`ELECTRON_DENSITY_MAX_PER_M3` for a negative-ion beam, or not `POSITIVE_ION_ELECTRON_DENSITY_PER_M3` for the 45 keV beam.",
        sealed: false,
    },
    Refusal {
        name: "ElectronTemperatureOutOfRange",
        meaning: "`electron_temperature_kev` is not a temperature compared with measurement for the beam asked: outside `ELECTRON_TEMPERATURE_MIN_KEV`..=`ELECTRON_TEMPERATURE_MAX_KEV` for a negative-ion beam, or not `POSITIVE_ION_ELECTRON_TEMPERATURE_KEV` for the 45 keV beam.",
        sealed: false,
    },
    Refusal {
        name: "BoronDensityOutOfRange",
        meaning: "`boron_density_per_m3` is not a boron density compared with measurement for the beam asked: below zero or above `BORON_DENSITY_MAX_PER_M3` for a negative-ion beam, or not `POSITIVE_ION_BORON_DENSITY_PER_M3` for the 45 keV beam.",
        sealed: false,
    },
    Refusal {
        name: "BoronBelowOneNucleus",
        meaning: "`boron_density_per_m3` is not zero and is below `BORON_DENSITY_ONE_NUCLEUS_PER_M3`, the peak density at which the plasma holds one boron nucleus: less than one nucleus is no boron, and zero is answered.",
        sealed: false,
    },
    Refusal {
        name: "BoronShareOutOfRange",
        meaning: "The peak boron density over the line-averaged electron density is above `BORON_OVER_ELECTRON_DENSITY_MAX`, the largest compared with measurement.",
        sealed: false,
    },
    Refusal {
        name: "BoronChargeExceedsElectrons",
        meaning: "Quasi-neutrality: in one shell the boron alone carries more charge than the electrons there.",
        sealed: false,
    },
    Refusal {
        name: "NonFiniteValue",
        meaning: "An intermediate value was NaN or infinite, or an input was when the solver is called directly: through the gateway no input can be, since a number that is not finite is not JSON.",
        sealed: true,
    },
    Refusal {
        name: "NotSettled",
        meaning: "The refinement cap was reached with the fused share of a shell still moving.",
        sealed: true,
    },
    Refusal {
        name: "ProtonLedgerBroken",
        meaning: "In one shell the slowed and the fused shares left `PROTON_LEDGER_TOLERANCE` of one.",
        sealed: true,
    },
    Refusal {
        name: "EnergyLedgerBroken",
        meaning: "In one shell the energy ledger left `ENERGY_LEDGER_TOLERANCE` of the beam energy.",
        sealed: true,
    },
];

/// The `LhdReject` variant name, without its fields.
pub fn reject_name(reject: &LhdReject) -> &'static str {
    match reject {
        LhdReject::BeamEnergyOutOfRange { .. } => "BeamEnergyOutOfRange",
        LhdReject::BeamPowerOutOfRange { .. } => "BeamPowerOutOfRange",
        LhdReject::ElectronDensityOutOfRange { .. } => "ElectronDensityOutOfRange",
        LhdReject::ElectronTemperatureOutOfRange { .. } => "ElectronTemperatureOutOfRange",
        LhdReject::BoronDensityOutOfRange { .. } => "BoronDensityOutOfRange",
        LhdReject::BoronBelowOneNucleus { .. } => "BoronBelowOneNucleus",
        LhdReject::BoronShareOutOfRange { .. } => "BoronShareOutOfRange",
        LhdReject::BoronChargeExceedsElectrons { .. } => "BoronChargeExceedsElectrons",
        LhdReject::NonFiniteValue => "NonFiniteValue",
        LhdReject::NotSettled { .. } => "NotSettled",
        LhdReject::ProtonLedgerBroken { .. } => "ProtonLedgerBroken",
        LhdReject::EnergyLedgerBroken { .. } => "EnergyLedgerBroken",
    }
}

fn field_words() -> String {
    format!(
        "SHA-256 of the shell count ({}) as 8 little-endian bytes, then the alpha emissivity of each shell from the axis outward, per cubic metre per second, as little-endian f64 bytes.",
        lhd::SHELLS
    )
}

fn sections() -> Sections {
    Sections {
        locked: json!({
            "model": "A beam proton is born at the beam energy in one of twenty radial shells and slows there on electrons, protons and boron by the test-particle rates of the NRL Plasma Formulary. At each energy it may fuse with boron-11, with the cross-section averaged over the thermal motion of the boron. Steady state. Profile shapes are fixed: electron temperature Te0 (1 - rho^2), ions at the electron temperature, electron density (9/8) n (1 - rho^8), boron the hollow profile digitised from Ogawa 2022 Figure 3, beam ions born per unit volume as (1 - rho^2). The boron is natural boron, 80 percent boron-11. Left out: charge-exchange and orbit loss of beam ions, radial motion while slowing, the time before steady state, beam-beam and thermal reactions, 10B(p,alpha)7Be, the beam's energy spread, the detector.",
            "magnetic_axis_m": lhd::MAGNETIC_AXIS_M,
            "minor_radius_m": lhd::MINOR_RADIUS_M,
            "toroidal_field_t": lhd::TOROIDAL_FIELD_T,
            "plasma_volume_m3": lhd::plasma_volume_m3(),
            "boron11_fraction": lhd::BORON11_FRACTION,
            "boron_profile_polynomial": lhd::BORON_PROFILE,
            "boron_profile_source": "DIGITISED from the fitted curve of Ogawa et al. 2022, Figure 3 (discharge 169626), and fitted by a polynomial of degree six in rho, highest power first, one at its peak. Not fitted to any alpha rate.",
            "central_over_line_averaged_density": lhd::CENTRAL_OVER_LINE_AVERAGED_DENSITY,
            "profile_shapes_source": "The shapes of temperature, density and beam deposition are assumptions. The papers print only the central temperature and the line-averaged density.",
            "shells": lhd::SHELLS,
            "stopping_energy_kev": lhd::STOPPING_ENERGY_KEV,
            "first_panel_kev": lhd::FIRST_PANEL_KEV,
            "max_refinements": lhd::MAX_REFINEMENTS,
            "reaction_energy_kev": reaction::REACTION_ENERGY_KEV,
            "cross_section": cross_section_record(),
        }),
        invariants: json!([
            {
                "name": "proton_ledger_residual",
                "rule": "Every beam proton is counted once: the share that slowed to the stopping energy and the share that fused add to one.",
                "tolerance": lhd::PROTON_LEDGER_TOLERANCE,
                "unit": "1",
                "checked": "in every shell at every refinement",
                "refusal": "ProtonLedgerBroken",
            },
            {
                "name": "energy_ledger_residual",
                "rule": "Every keV of the beam energy is counted once: given to electrons, to protons or to boron, carried into fusion, or still held at the stopping energy.",
                "tolerance": lhd::ENERGY_LEDGER_TOLERANCE,
                "unit": "fraction of the beam energy",
                "checked": "in every shell at every refinement",
                "refusal": "EnergyLedgerBroken",
            },
        ]),
        convergence: json!({
            "settle_tolerance": lhd::SETTLE_TOLERANCE,
            "rule": "In each shell the energy panels are halved until the fused share moves by less than the settle tolerance.",
            "refusal": "NotSettled",
        }),
        reference: json!({
            "name": lhd::REFERENCE_NAME,
            "url": lhd::REFERENCE_URL,
            "retrieved": lhd::REFERENCE_RETRIEVED,
            "published_alpha_rate_per_s": lhd::REFERENCE_ALPHA_RATE_PER_S,
            "test": "ogawa_2022_three_beams_of_discharge_168156",
            "covers": "The total alpha emission rate of one case. The test prints the solver's rate over the published one for both readings of the paper's boron density and holds the solver to neither. The served solver refuses this case by name: its beam energies, density and temperature lie outside the envelope compared with measurement. The test runs the solver's physics on it directly.",
            "kind": "A reference calculation. It is not a measurement.",
        }),
    }
}

/// The validation record, in the five parts a card's description gives: the
/// object, the data, the pass line, the result and the limits. It states
/// each comparison as it came out: three of the four lines were missed.
fn validation() -> [String; 5] {
    let object = "The boron-seeded hydrogen plasma of the Large Helical Device (National Institute for Fusion Science, Toki), toroidal field 2.75 T, magnetic axis 3.60 m, heated by tangential hydrogen neutral beams: the only magnetically confined plasma in which proton-boron-11 alpha particles have been counted. Ogawa et al., Fusion Sci. Technol. 78 (2022) 175; Magee et al., Nat. Commun. 14 (2023) 955; Ogawa et al., Nucl. Fusion 64 (2024) 096028.".to_string();
    let data = format!(
        "One published calculation: Ogawa et al. 2022: 1.5e14 alpha particles per second for the three negative-ion beams of discharge 168156 (180, 165 and 164 keV; 4.33, 2.39 and 3.37 MW deposited; 1.96e19 m^-3; 5.4 keV) with the boron profile of discharge 169626. A reference calculation. It is not a measurement. Measured: Alpha count rate of one silicon detector under the plasma, discharges 178062 to 178072 (Ogawa et al. 2024; Magee et al. 2023). The values were DIGITISED from the papers' figures; the device's data service did not answer. Beam energy dependence: Ogawa et al. 2024, Figure 10: the count rate over boron signal, beam particle rate and slowing-down time at six points, three discharges at two times, beam voltage 127 to 160 kV. Time shape: Ogawa et al. 2024, Figure 6, discharge 178063: the count rate at 4.5, 5.0, 5.5, 6.0, 6.5 and 7.0 s. Record: {EVIDENCE}."
    );
    let pass_line = format!(
        "Four lines. Against the published calculation: solver over published inside 0.5 to 2.0. Beam energy dependence: after one common scale factor, every point within a factor of 2. Time shape: after one common scale factor, every point within a factor of 2. Collapse under the 45 keV beam: solver ratio at most the measured ratio. Fixed {PASS_LINES_FIXED}, before any forecast was run and before any value was read off a figure of Ogawa et al. 2024."
    );
    let result = format!("Three of the four lines were missed and one was met. Run: Through the gateway on 2026-10-09, {RUN_TWICE}. Each answer is saved with its seal and its receipt. Published calculation, with the boron of the paper's text, 6.3e16 per m^3: alpha rate 1.921e13 per second, solver over published 0.128, not met. With the boron of the figure's axis, 6.3e17 per m^3, not gated: alpha rate 1.950e14 per second, solver over published 1.300. The paper gives its boron density twice and the two differ by ten: about 7e16 m^-3 in its text and colour bar, 6.3e17 by the axis of the profile it says it used. The protocol registered the text's value, and by it the pass line is missed. Which value the published calculation used is not known. The reference contradicts itself in a second place: its own emissivity map integrates to about 0.4 of its own stated total. The registered band of 0.5 to 2.0 is tighter than the reference's agreement with itself; the line stays missed. Beam energy dependence: worst factor 2.94, Spearman rank correlation 0.943, rise from 128 to 160 kV measured 2.10 and solver 11.1, not met. These runs took the beam voltage of each circle of the bottom panel of Figure 10, where the protocol names the top panel; with the top panel's voltages the worst factor is 2.953 and the rise 10.7, and the line is still missed. The solver puts the six points in the measured order but rises five times more steeply with beam energy than the measurement does. The measured rise from 128 to 160 kV, 2.1, is below the 4.4 that a cross-section with no resonance at all gives, so no cross-section shape or averaging in the solver can produce it. The cause is open: it lies on the measurement side or in something that neither this solver nor the experimenters' own calculation contains. Time shape: worst factor 2.42, not met. Run as registered with the injected beam power, the only power the figure gives; the solver takes deposited power. With the power scaled by the density (not gated, chosen after the figure was opened) the worst factor is 1.82. Collapse under the 45 keV beam: solver ratio 1.26e-5, measured ratio 0.062, met. One-sided: the measured floor is noise. Absolute count rate, not gated: At 6.0 s with 1.3 MW deposited the solver gives 7.0e12 alpha particles per second: 1.28 times the paper's own calculation (about 5.5e12), and through the paper's detection efficiency of 1.2e-7 counts per alpha 845 kcps against a measured 179 kcps, 4.7 times above. The paper's calculation is itself about three times above its measurement. {SECOND_SCORING}");
    let limits = "Fitted constants: None. Every number is from the two Ogawa papers or is a declared assumption of the solver. What the measurement tests: The shape of the answer in beam energy, in time and across the step to a 45 keV beam, each after one free scale factor. It does not test the absolute rate: no absolute alpha rate has been measured in a magnetically confined plasma. The declared ranges are the envelope of the inputs compared with measurement, and each beam family is its own island. The negative-ion beams, 127.45 to 160.25 keV, are held to the envelope of their own runs, each input on its own: power 0.9389 to 3.97 MW, line-averaged electron density 2.261e18 to 6.872e18 m^-3, central electron temperature 2.9929 to 3.7872 keV, peak boron density zero, or from 0.04973892441153873 m^-3, one boron nucleus in the plasma, to 2.84682e17 m^-3, and at most 0.0414 of the electron density. The 45 keV beam was compared in one plasma only, Figure 6 at 7.8 s, and is answered only there: 4.515 MW, 4.502e18 m^-3, 2.1252 keV and 8.1234e16 m^-3 exactly. Outside its island the solver refuses by name, so the 45 keV plasma's lower temperature and higher power are refused under a negative-ion beam. Inside the negative-ion island, combinations of inputs that no single measured point had are answered: each input lies inside the span its runs reached, and the runs span that box only in part. The published calculation's plasma, 164 to 180 keV at 1.96e19 m^-3 and 5.4 keV, is hotter and denser than every measured one and is refused: it was compared with a calculation, not a measurement. The runs against it were made with engine 0.6.0 and earlier, before this envelope. The record says nothing of: Any other device or beam; beam ions lost by charge exchange or orbits; the time before steady state; how many alphas reach a detector; thermal fusion, which is nil at these temperatures. What the shared cross-section was held to is under locked.cross_section.".to_string();
    [object, data, pass_line, result, limits]
}

fn example() -> Value {
    json!({
        "beam_energy_kev": 160, "beam_power_w": 2.424e6, "electron_density_per_m3": 3.137e18,
        "electron_temperature_kev": 3.2438, "boron_density_per_m3": 8.6706e16
    })
}

/// The solver's own question, from the values of `parameters`.
fn question(given: &Given) -> Result<LhdQuestion, String> {
    Ok(LhdQuestion {
        beam_energy_kev: given.number("beam_energy_kev")?,
        beam_power_w: given.number("beam_power_w")?,
        electron_density_per_m3: given.number("electron_density_per_m3")?,
        electron_temperature_kev: given.number("electron_temperature_kev")?,
        boron_density_per_m3: given.number("boron_density_per_m3")?,
    })
}

/// The limits the question itself shows, asked of the solver's own check
/// before a physics permit is taken: the five ranges, the boron for its
/// electrons, and quasi-neutrality in every shell. The sentence is the solver's, with
/// its name for the refusal.
fn check(given: &Given) -> Result<(), String> {
    lhd::admit(&question(given)?)
        .map_err(|reject| format!("Physical Violation: {reject} ({}).", reject_name(&reject)))
}

fn solve(given: &Given) -> Result<Solved, String> {
    Ok(solved(lhd::solve(&question(given)?)))
}

/// What the solver returned, in the words every solver shares. A refusal the
/// request itself shows is answered with no seal and with the solver's
/// sentence alone, so its sentence carries the solver's name for it.
pub fn solved(returned: Result<LhdBeamFusion, LhdReject>) -> Solved {
    let answer = match returned {
        Ok(answer) => answer,
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
                    LhdReject::NotSettled { refinements } => Some(u64::from(refinements)),
                    _ => None,
                },
            });
        }
    };
    let named = |name: &str, value: f64| (name.to_string(), Published::Number(value));
    Solved::Answer(Answer {
        iterations: u64::from(answer.refinements),
        result: vec![
            named("alpha_emission_rate_per_s", answer.alpha_emission_rate_per_s),
            named("fusion_power_w", answer.fusion_power_w),
            named("fusion_gain", answer.fusion_gain),
            named("fused_fraction", answer.fused_fraction),
            named(
                "peak_alpha_emissivity_per_m3_s",
                answer.peak_alpha_emissivity_per_m3_s,
            ),
            named("peak_emissivity_radius", answer.peak_emissivity_radius),
            named("slowing_time_s", answer.slowing_time_s),
            named(
                "energy_fraction_to_electrons",
                answer.energy_fraction_to_electrons,
            ),
            named("energy_fraction_to_ions", answer.energy_fraction_to_ions),
        ],
        invariants: vec![
            named("proton_ledger_residual", answer.proton_ledger_residual),
            named("energy_ledger_residual", answer.energy_ledger_residual),
        ],
        field: answer.alpha_emissivity_per_m3_s,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::solver_fusion_pb11_shared::documented_rejects_of;
    use crate::solvers::{describe_value, solve_schema_in};
    use crate::view::View;

    const LHD_SOURCE: &str = include_str!("../../genesis_solvers/src/fusion_pb11_lhd.rs");

    fn read(parameters: &Value) -> Result<Given, String> {
        crate::solution::read_parameters(
            &FUSION_PB11_LHD,
            parameters.as_object().expect("an object"),
        )
        .map_err(|error| error.message().to_string())
    }

    #[test]
    fn every_refusal_carries_the_sentence_from_its_doc_comment() {
        let documented = documented_rejects_of(LHD_SOURCE, "pub enum LhdReject {");
        assert_eq!(documented.len(), REFUSALS.len());
        for ((variant, sentence), refusal) in documented.iter().zip(REFUSALS) {
            assert_eq!(variant, refusal.name);
            assert_eq!(sentence, refusal.meaning, "{}", refusal.name);
        }
        let body = describe_value(FUSION_PB11_LHD.name).expect("described");
        let refusals = body["refusals"].as_array().expect("refusals");
        assert_eq!(refusals.len(), documented.len());
        // The eight refusals the request itself shows are answered with no
        // seal, as the solver's own check names them.
        for (place, refusal) in refusals.iter().enumerate() {
            let reported = if place < 8 { "422, no seal" } else { "200, outcome refused, sealed" };
            assert_eq!(refusal["reported_as"], reported, "{}", refusal["name"]);
        }
    }

    #[test]
    fn the_reference_is_the_one_stored_in_the_solver_test() {
        let body = describe_value(FUSION_PB11_LHD.name).expect("described");
        let url = body["reference"]["url"].as_str().expect("url");
        let retrieved = body["reference"]["retrieved"].as_str().expect("retrieved");
        assert!(LHD_SOURCE.contains(&format!("<{url}>")), "{url}");
        assert!(LHD_SOURCE.contains(&format!("read {retrieved}")), "{retrieved}");
        let test = body["reference"]["test"].as_str().expect("test");
        assert!(LHD_SOURCE.contains(&format!("fn {test}()")), "{test}");
        assert_eq!(
            body["reference"]["kind"],
            "A reference calculation. It is not a measurement."
        );
    }

    #[test]
    fn the_description_names_every_input_and_output_once() {
        let body = describe_value(FUSION_PB11_LHD.name).expect("described");
        let names = |section: &Value| -> Vec<String> {
            section
                .as_array()
                .expect("list")
                .iter()
                .map(|entry| entry["name"].as_str().expect("name").to_string())
                .collect()
        };
        let inputs = [
            "beam_energy_kev",
            "beam_power_w",
            "electron_density_per_m3",
            "electron_temperature_kev",
            "boron_density_per_m3",
        ];
        let result = [
            "alpha_emission_rate_per_s",
            "fusion_power_w",
            "fusion_gain",
            "fused_fraction",
            "peak_alpha_emissivity_per_m3_s",
            "peak_emissivity_radius",
            "slowing_time_s",
            "energy_fraction_to_electrons",
            "energy_fraction_to_ions",
        ];
        let invariants = ["proton_ledger_residual", "energy_ledger_residual"];
        assert_eq!(names(&body["inputs"]), inputs);
        assert_eq!(names(&body["returns"]["result"]), result);
        assert_eq!(names(&body["returns"]["invariants"]), invariants);
        assert_eq!(names(&body["invariants"]), invariants);
        for name in result {
            assert!(FUSION_PB11_LHD.published_shape("result", name).is_some());
            assert!(FUSION_PB11_LHD.published_shape("invariants", name).is_none());
        }
        assert!(FUSION_PB11_LHD.summary.contains("No constant is fitted"));
        // The other proton-boron solver's names are not this one's, and the
        // atom publishes neither's.
        assert!(FUSION_PB11_LHD.published_shape("result", "electron_temperature_kev").is_none());
        assert!(
            crate::solver_atom_dft::ATOM_DFT
                .published_shape("result", "fusion_gain")
                .is_none()
        );
        // The power's range is the powers compared with measurement, both
        // ends inside it.
        assert_eq!(body["inputs"][1]["minimum"], 938_900.0);
        assert_eq!(body["inputs"][1]["maximum"], 4_515_000.0);
        assert!(body["inputs"][1]["bound"].as_str().unwrap().contains("compared with measurement"));
        // Every bound sentence says what was compared.
        for input in body["inputs"].as_array().unwrap() {
            assert!(input["bound"].as_str().unwrap().contains("compared"), "{}", input["name"]);
        }
        let limits = &validation()[4];
        assert!(!limits.contains("both lie inside the declared ranges"));
        assert!(limits.contains("envelope of the inputs compared with measurement"));
        // Each beam family is its own island: the 45 keV plasma opens nothing to a negative-ion beam.
        assert!(limits.contains("power 0.9389 to 3.97 MW"));
        assert!(limits.contains("central electron temperature 2.9929 to 3.7872 keV"));
        assert!(limits.contains("4.515 MW, 4.502e18 m^-3, 2.1252 keV and 8.1234e16 m^-3 exactly"));
        assert!(!limits.contains("2.1252 to 3.7872 keV"));
        assert!(!limits.contains("0.9389 to 4.515 MW"));
        assert_eq!(body["inputs"][3]["minimum"], 2.1252);
        assert!(body["inputs"][3]["bound"].as_str().unwrap().contains("a negative-ion beam from 2.9929 to 3.7872 keV"));
        assert!(body["inputs"][1]["bound"].as_str().unwrap().contains("to 3.97 MW"));
    }

    #[test]
    fn a_question_is_read_whole_or_named() {
        let parameters = example();
        assert!(read(&parameters).is_ok());
        for input in FUSION_PB11_LHD.inputs {
            let mut missing = parameters.clone();
            missing.as_object_mut().unwrap().remove(input.name);
            let Err(message) = read(&missing) else {
                panic!("{} was filled in", input.name)
            };
            assert_eq!(message, format!("Missing required parameter: '{}'", input.name));
        }
    }

    /// Each limit the question shows is refused before a permit, by the
    /// solver's name for it: the beam energy as the field list reads it, the
    /// rest in the solver's own words and order.
    #[test]
    fn each_limit_of_the_question_is_named_at_the_door() {
        let refused = |changes: &[(&str, Value)]| {
            let mut parameters = example();
            for (field, value) in changes {
                parameters[*field] = value.clone();
            }
            match read(&parameters) {
                Ok(_) => panic!("{changes:?} was not refused"),
                Err(message) => message,
            }
        };
        assert_eq!(
            refused(&[("beam_energy_kev", json!(39.5))]),
            "Physical Violation: beam energy 39.5 keV is neither 45 nor inside 127.45..=160.25, the beam energies compared with measurement (BeamEnergyOutOfRange)."
        );
        assert_eq!(
            refused(&[("beam_energy_kev", json!(100))]),
            "Physical Violation: beam energy 100 keV is neither 45 nor inside 127.45..=160.25, the beam energies compared with measurement (BeamEnergyOutOfRange)."
        );
        assert_eq!(
            refused(&[("beam_power_w", json!(1))]),
            "Physical Violation: beam power 1 W is not a power compared with measurement for the beam asked: 938900..=3970000 for a negative-ion beam, 4515000 for the 45 keV beam (BeamPowerOutOfRange)."
        );
        assert_eq!(
            refused(&[("electron_density_per_m3", json!(4.0e18)), ("boron_density_per_m3", json!(2.0e17))]),
            "Physical Violation: peak boron density over line-averaged electron density 0.05 is above 0.04142636786961583, the largest compared with measurement (BoronShareOutOfRange)."
        );
        // A boron density that is not zero and is under one nucleus in the
        // plasma, in the solver's own words, with the value by its exponent.
        assert_eq!(
            refused(&[("boron_density_per_m3", json!(1.0e-297))]),
            "Physical Violation: boron density 1e-297 m^-3 is not zero and is below 0.04973892441153873 m^-3, the peak density at which the plasma holds one boron nucleus: less than one nucleus is no boron, and zero is answered (BoronBelowOneNucleus)."
        );
        assert_eq!(
            refused(&[("boron_density_per_m3", json!(1.8008101798042446e-289))]),
            "Physical Violation: boron density 1.8008101798042446e-289 m^-3 is not zero and is below 0.04973892441153873 m^-3, the peak density at which the plasma holds one boron nucleus: less than one nucleus is no boron, and zero is answered (BoronBelowOneNucleus)."
        );
        let floor = lhd::BORON_DENSITY_ONE_NUCLEUS_PER_M3;
        assert_eq!(floor, 0.04973892441153873);
        assert!(refused(&[("boron_density_per_m3", json!(f64::from_bits(floor.to_bits() - 1)))]).ends_with("(BoronBelowOneNucleus)."));
        assert!(refused(&[("boron_density_per_m3", json!(0.04))]).ends_with("(BoronBelowOneNucleus)."));
        // Zero, the floor itself and the worked example are read whole.
        for boron in [json!(0.0), json!(0), json!(floor), json!(8.6706e16)] {
            let mut parameters = example();
            parameters["boron_density_per_m3"] = boron.clone();
            assert!(read(&parameters).is_ok(), "{boron}");
        }
        // The bound and the record give the floor as the solver holds it.
        let bound = FUSION_PB11_LHD.inputs[4].bound;
        assert_eq!(FUSION_PB11_LHD.inputs[4].name, "boron_density_per_m3");
        assert!(bound.contains(&format!("zero, or from {floor} m^-3 to the largest compared with measurement")), "{bound}");
        assert!(bound.contains("is refused as BoronBelowOneNucleus"), "{bound}");
        assert!(bound.contains(&format!("in the {:.3} m^3 plasma", lhd::plasma_volume_m3())), "{bound}");
        assert!(validation()[4].contains(&format!("peak boron density zero, or from {floor} m^-3, one boron nucleus in the plasma, to 2.84682e17 m^-3")));
        // Two limits at once: the solver asks the power before the density.
        let both = refused(&[("electron_density_per_m3", json!(1.0e17)), ("beam_power_w", json!(0))]);
        assert!(both.ends_with("(BeamPowerOutOfRange)."), "{both}");
        for (field, value, name) in [
            ("beam_energy_kev", json!(180), "BeamEnergyOutOfRange"),
            ("beam_power_w", json!(1.6e7), "BeamPowerOutOfRange"),
            ("beam_power_w", json!(6.0e6), "BeamPowerOutOfRange"),
            ("electron_density_per_m3", json!(1.0e18), "ElectronDensityOutOfRange"),
            ("electron_density_per_m3", json!(1.96e19), "ElectronDensityOutOfRange"),
            ("electron_temperature_kev", json!(2.0), "ElectronTemperatureOutOfRange"),
            ("electron_temperature_kev", json!(5.4), "ElectronTemperatureOutOfRange"),
            ("electron_temperature_kev", json!(2.1252), "ElectronTemperatureOutOfRange"),
            ("beam_power_w", json!(4.515e6), "BeamPowerOutOfRange"),
            ("boron_density_per_m3", json!(-1.0), "BoronDensityOutOfRange"),
            ("boron_density_per_m3", json!(3.0e17), "BoronDensityOutOfRange"),
        ] {
            let message = refused(&[(field, value)]);
            assert!(message.starts_with("Physical Violation: "), "{message}");
            assert!(message.ends_with(&format!("({name}).")), "{message}");
        }
    }

    #[test]
    fn the_schema_requires_a_lone_solver_s_fields_and_lists_them_otherwise() {
        let lone = Value::Object(solve_schema_in(&View::limited(
            [],
            [FUSION_PB11_LHD.name.to_string()],
        )));
        assert_eq!(lone["properties"]["solver"], json!({ "const": "fusion_pb11_lhd" }));
        let parameters = &lone["properties"]["parameters"];
        assert_eq!(parameters["required"].as_array().unwrap().len(), 5);
        assert_eq!(parameters["properties"].as_object().unwrap().len(), 5);
        assert_eq!(parameters["properties"]["beam_energy_kev"]["maximum"], 160.25);
        assert_eq!(parameters["properties"]["beam_energy_kev"]["minimum"], 45.0);
        assert_eq!(parameters["additionalProperties"], false);
        // With the other proton-boron solver in the view no field is
        // required by the schema, and each solver's fields are in words:
        // electron_density_per_m3 is a field of both, with different bounds.
        let both = Value::Object(solve_schema_in(&View::limited(
            [],
            [
                FUSION_PB11_LHD.name.to_string(),
                crate::solver_fusion_pb11_thermal::FUSION_PB11_THERMAL.name.to_string(),
            ],
        )));
        let parameters = &both["properties"]["parameters"];
        assert!(parameters.get("required").is_none());
        assert!(parameters.get("properties").is_none());
        // The inputs of each are left to describe_solver.
        let words = parameters["description"].as_str().expect("the inputs in words");
        assert!(words.contains("describe_solver"), "{words}");
        assert!(!words.contains("beam_energy_kev"), "{words}");
        let atom = Value::Object(solve_schema_in(&View::limited([], ["atom_dft".to_string()])));
        assert!(!atom.to_string().contains("beam_energy_kev"));
    }

    #[test]
    fn a_refusal_the_solver_finds_is_sealed_and_a_range_refusal_is_not() {
        let request = json!({ "solver": FUSION_PB11_LHD.name, "parameters": example() });
        let response = crate::solution::seal(
            &FUSION_PB11_LHD,
            request.clone(),
            solved(Err(LhdReject::NotSettled { refinements: 6 })),
        )
        .expect("sealed");
        let body: Value = serde_json::from_str(&serde_json::to_string(&response).unwrap()).unwrap();
        assert_eq!(body["outcome"], "refused");
        assert_eq!(body["reject"], "NotSettled");
        assert_eq!(
            body["reject_detail"],
            "Solver Refusal: fusion_pb11_lhd did not return an alpha rate (NotSettled). the fused share of a shell was still moving after 6 refinements."
        );
        assert_eq!(body["iterations"], 6);
        assert!(body["result"].is_null());
        assert!(
            crate::solution::check(&body)
                .unwrap_or_else(|_| panic!("refused at the door"))
                .holds
        );
        let Err(unsealed) = crate::solution::seal(
            &FUSION_PB11_LHD,
            request,
            solved(Err(LhdReject::BoronChargeExceedsElectrons { shell: 19 })),
        ) else {
            panic!("a limit the request shows was sealed")
        };
        assert_eq!(
            unsealed,
            "Physical Violation: in shell 19 of 20 the boron carries more charge than the electrons: quasi-neutrality cannot hold (BoronChargeExceedsElectrons)."
        );
    }

    /// The validation record states each comparison as it came out, the
    /// three that missed included.
    #[test]
    fn the_validation_record_holds_every_number_of_the_comparison() {
        let [object, data, pass_line, result, limits] = validation();
        assert!(object.contains("Large Helical Device"));
        assert!(data.contains("It is not a measurement") && data.contains("DIGITISED"));
        for mark in ["0.5 to 2.0", "within a factor of 2", "at most the measured ratio", "8bd91c5c1328f461a268ccca6dc0f31c61b11ab95051da1fe260fe5c2841db90", "2026-10-09T20:51:55Z"] {
            assert!(pass_line.contains(mark), "{mark}");
        }
        for number in [
            "1.921e13", "0.128", "1.950e14", "1.300", "2.94", "0.943", "2.10", "11.1", "2.42",
            "1.82", "1.26e-5", "0.062", "7.0e12", "1.28", "5.5e12", "1.2e-7", "845", "179",
            "4.7 times above", "about 0.4 of its own stated total", "below the 4.4",
            "evidence/checks/second_party/V-26/", "twice", "all 58 answers",
        ] {
            assert!(result.contains(number), "{number}");
        }
        assert!(result.starts_with("Three of the four lines were missed"));
        assert_eq!(result.matches("not met").count(), 3);
        assert!(limits.starts_with("Fitted constants: None."));
        assert!(limits.contains("It does not test the absolute rate"));
        let body = describe_value(FUSION_PB11_LHD.name).expect("described");
        let cross_section = &body["locked"]["cross_section"];
        assert_eq!(cross_section["against_nacre_ii"]["met"], false);
        assert_eq!(cross_section["against_nacre_ii"]["inside_its_stated_band"], 30);
    }
}
