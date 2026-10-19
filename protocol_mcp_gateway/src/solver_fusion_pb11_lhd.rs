//! The entry for `fusion_pb11_lhd`: proton-boron-11 fusion of one hydrogen
//! neutral beam in the boron-seeded plasma of the Large Helical Device. The
//! numbers come from `genesis_solvers` constants, the sentence of each
//! refusal from the solver's source, and the validation record from the
//! solver's research record.

use genesis_solvers::fusion_pb11_lhd::{self as lhd, LhdBeamFusion, LhdQuestion, LhdReject};
use genesis_solvers::pb11_reaction as reaction;
use serde_json::{Value, json};

use crate::solver_fusion_pb11_shared::{
    PASS_LINES_FIXED, RUN_TWICE, SECOND_SCORING, cross_section_record, number,
};
use crate::solvers::{
    Answer, Given, Input, Kind, Published, Refusal, Refused, Sections, Solved, Solver,
};

pub const FUSION_PB11_LHD: Solver = Solver {
    name: "fusion_pb11_lhd",
    summary: "Proton-boron-11 fusion in the boron-seeded hydrogen plasma of the Large Helical Device under one hydrogen neutral beam above 10 to 190 keV: alpha particles per second, fusion power, the share of beam protons that fuse, the alpha emissivity profile, and the fast protons over the electrons. No constant is fitted. Compared with the measured alpha count rates of Ogawa et al. (2024) and the published calculation of Ogawa et al. (2022): one of its four lines met (the collapse under a 45 keV beam); it put six beam energies in the measured order but rose 11.1 times from 128 to 160 kV where the measurement rose 2.10.",
    inputs: &[
        // Every limit is asked by the solver's own check, in its order: the
        // boron density has a gap no range with two ends can say, between
        // zero and one nucleus in the plasma.
        Input {
            name: "beam_energy_kev",
            kind: Kind::NumberAbove {
                above: lhd::BEAM_ENERGY_MIN_KEV,
                maximum: lhd::BEAM_ENERGY_MAX_KEV,
                unit: "keV",
            },
            refusal: Some("BeamEnergyOutOfRange"),
            bound: "Energy of the beam protons, above 10 keV to 190 keV. A proton is followed down to 10 keV, so at or below it there is nothing to follow; 190 keV is the rating of the device's negative-ion beams, the most energetic beams the object has (the equations go further: the cross-section's fit reaches 10.6 MeV in the laboratory). Compared with measured alpha count rates for negative-ion beams of 127.45 to 160.25 keV and one 45 keV beam at 7.8 s: the six beam-energy points come in the measured order (Spearman 0.943), with a rise of 11.1 times from 128 to 160 kV where the measurement rises 2.10, and under the 45 keV beam the rate falls to 1.26e-5 of its value before, against a measured 0.062. One beam is one question; the alphas of several beams add.",
            only_when: None,
            asked_by_check: true,
        },
        Input {
            name: "beam_power_w",
            kind: Kind::NumberAbove {
                above: 0.0,
                maximum: lhd::BEAM_POWER_MAX_W,
                unit: "W",
            },
            refusal: Some("BeamPowerOutOfRange"),
            bound: "Beam power deposited in the plasma, injected less shine-through: any power above zero (to 1e300 W, a capacity of the input and no physics). The alpha rate is proportional to it, and so are the fast protons; where in any shell the beam protons still slowing and the boron would carry more charge than the electrons, no thermal proton could be left, and the solver stops by name (FastProtonChargeExceedsElectrons, sealed). Compared at deposited powers of 0.9389 to 1.4056 MW for the negative-ion beams of the beam-energy scan, answered, and at the injected powers of the time trace, 2.254 to 3.97 MW and 4.515 MW for the 45 keV beam, which leave 1.25 to 7.25 fast protons per electron at the axis and are refused by that limit; computed with the injected power before that limit was held, the time shape came within a worst factor of 2.42.",
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
            bound: "Line-averaged electron density, 1e18 to 3e19 m^-3, the span of the device's proton-boron discharges. Up to the top the ionisation length of a 45 to 190 keV hydrogen beam stays above the minor radius (an estimate), so the core-peaked deposition the solver takes holds. Compared with measurement at line-averaged electron densities of 2.261e18 to 6.872e18 m^-3, and with the published calculation at its 1.96e19 m^-3.",
            only_when: None,
            asked_by_check: true,
        },
        Input {
            name: "electron_temperature_kev",
            kind: Kind::Number {
                minimum: lhd::ELECTRON_TEMPERATURE_MIN_KEV,
                maximum: lhd::ELECTRON_TEMPERATURE_MAX_KEV,
                unit: "keV",
            },
            refusal: Some("ElectronTemperatureOutOfRange"),
            bound: "Central electron temperature, 0.5 to 6.67 keV; the ions are at the electron temperature. Below 0.5 keV boron at the axis is not fully stripped (the last electron of B4+ is bound by 0.340 keV), and the solver takes boron as a bare nucleus of charge 5. Above two thirds of 10 keV the 10 keV a proton is followed to is less than 1.5 ion temperatures, and the proton would be thermal. Compared with measurement at central electron temperatures of 2.1252 keV (the 45 keV beam) and 2.9929 to 3.7872 keV (the negative-ion beams), and with the published calculation at its 5.4 keV.",
            only_when: None,
            asked_by_check: true,
        },
        Input {
            name: "boron_density_per_m3",
            kind: Kind::Number {
                minimum: 0.0,
                maximum: lhd::ELECTRON_DENSITY_MAX_PER_M3,
                unit: "m^-3",
            },
            refusal: Some("BoronDensityOutOfRange"),
            bound: "Boron density at the peak of its profile, all isotopes: zero, or from 0.04973892441153873 m^-3 to what quasi-neutrality allows: five times the boron may not exceed the electrons in any shell, so the published maximum, the top electron density, is an outer bound. At 0.04973892441153873 m^-3 the solver's twenty shells hold one boron nucleus in the 25.582 m^3 plasma. A density that is not zero and is below it is refused as BoronBelowOneNucleus: less than one nucleus is no boron, and no boron is asked as zero, which is answered. Quasi-neutrality is held in every shell: five times the boron density may not exceed the electron density there (BoronChargeExceedsElectrons). Compared at peak boron densities of zero and 5.1354e16 to 2.84682e17 m^-3; the largest boron over electron density compared was 0.0414. No other input is accepted.",
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
        number("fusion_gain", "1", "Fusion power over deposited beam power, computed as the fused share times 8.68 MeV over the beam energy, independent of the power."),
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
            "fast_proton_fraction_max",
            "1",
            "Largest density of beam protons still slowing over the electron density, of any shell: born per second per cubic metre times the shell's slowing time. The solver takes the thermal protons as n_e - 5 n_B and slows each beam proton on them alone, so the drag on protons is that of this share more protons than quasi-neutrality leaves. Where the fast protons and the boron would carry more than the electrons' charge the answer is refused (FastProtonChargeExceedsElectrons).",
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
    door_refusals: &[],
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
/// in the plasma and boron that outweighs the electrons, so those seven
/// refusals are not sealed.
const REFUSALS: &[Refusal] = &[
    Refusal {
        name: "BeamEnergyOutOfRange",
        meaning: "`beam_energy_kev` is not above the 10 keV a proton is followed down to, where there is nothing to follow, or is above 190 keV, the rating of the device's negative-ion beams.",
        sealed: false,
    },
    Refusal {
        name: "BeamPowerOutOfRange",
        meaning: "`beam_power_w` is not above zero, which is no beam, or is above the 1e300 W capacity of the input.",
        sealed: false,
    },
    Refusal {
        name: "ElectronDensityOutOfRange",
        meaning: "`electron_density_per_m3` is outside 1e18 to 3e19 per cubic metre, the device's proton-boron discharges: above it the beam's deposition is no longer core-peaked.",
        sealed: false,
    },
    Refusal {
        name: "ElectronTemperatureOutOfRange",
        meaning: "`electron_temperature_kev` is outside 0.5 to 6.67 keV: below it boron at the axis is not fully stripped, above it the 10 keV a proton is followed to is less than 1.5 ion temperatures and the proton would be thermal.",
        sealed: false,
    },
    Refusal {
        name: "BoronDensityOutOfRange",
        meaning: "`boron_density_per_m3` is below zero: no density.",
        sealed: false,
    },
    Refusal {
        name: "BoronBelowOneNucleus",
        meaning: "`boron_density_per_m3` is not zero and is below `BORON_DENSITY_ONE_NUCLEUS_PER_M3`, the peak density at which the plasma holds one boron nucleus: less than one nucleus is no boron, and zero is answered.",
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
        name: "SlowingRateNotPositive",
        meaning: "At some energy between the beam energy and the stopping energy a beam proton gains energy from the plasma of a shell: it is no longer slowing.",
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
    Refusal {
        name: "FastProtonChargeExceedsElectrons",
        meaning: "Quasi-neutrality with the fast protons counted: in one shell the beam protons still slowing, born per second times the time each takes to slow, and the boron carry more charge than the electrons there, so no thermal proton could be left.",
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
        LhdReject::BoronChargeExceedsElectrons { .. } => "BoronChargeExceedsElectrons",
        LhdReject::NonFiniteValue => "NonFiniteValue",
        LhdReject::SlowingRateNotPositive { .. } => "SlowingRateNotPositive",
        LhdReject::NotSettled { .. } => "NotSettled",
        LhdReject::ProtonLedgerBroken { .. } => "ProtonLedgerBroken",
        LhdReject::EnergyLedgerBroken { .. } => "EnergyLedgerBroken",
        LhdReject::FastProtonChargeExceedsElectrons { .. } => "FastProtonChargeExceedsElectrons",
    }
}

fn field_words() -> String {
    format!(
        "SHA-384 of the shell count ({}) as 8 little-endian bytes, then the alpha emissivity of each shell from the axis outward, per cubic metre per second, as little-endian f64 bytes.",
        lhd::SHELLS
    )
}

fn sections() -> Sections {
    Sections {
        locked: json!({
            "model": "A beam proton is born at the beam energy in one of twenty radial shells and slows there on electrons, protons and boron by the test-particle rates of the NRL Plasma Formulary. At each energy it may fuse with boron-11, with the cross-section averaged over the thermal motion of the boron. Steady state. The beam protons still slowing in a shell, born per second times the slowing time, are positive charge beside the thermal protons and the boron; where with the boron they would carry more than the electrons' charge, no thermal proton could be left, and the answer is refused (FastProtonChargeExceedsElectrons). Profile shapes are fixed: electron temperature Te0 (1 - rho^2), ions at the electron temperature, electron density (9/8) n (1 - rho^8), boron the hollow profile digitised for discharge 169626, beam ions born per unit volume as (1 - rho^2). The boron is natural boron, 80 percent boron-11. Left out: charge-exchange and orbit loss of beam ions, radial motion while slowing, the time before steady state, beam-beam and thermal reactions, 10B(p,alpha)7Be, the beam's energy spread, the detector.",
            "magnetic_axis_m": lhd::MAGNETIC_AXIS_M,
            "minor_radius_m": lhd::MINOR_RADIUS_M,
            "toroidal_field_t": lhd::TOROIDAL_FIELD_T,
            "plasma_volume_m3": lhd::plasma_volume_m3(),
            "boron11_fraction": lhd::BORON11_FRACTION,
            "boron_profile_polynomial": lhd::BORON_PROFILE,
            "boron_profile_source": "Digitised from the fitted boron curve of discharge 169626 in the published calculation (Fusion Sci. Technol. 78, 175), and fitted by a polynomial of degree six in rho, highest power first, one at its peak. Fitted to no alpha rate.",
            "central_over_line_averaged_density": lhd::CENTRAL_OVER_LINE_AVERAGED_DENSITY,
            "profile_shapes_source": "The shapes of temperature, density and beam deposition are assumptions. The papers print the central temperature and the line-averaged density, and no profile.",
            "shells": lhd::SHELLS,
            "stopping_energy_kev": lhd::STOPPING_ENERGY_KEV,
            "fast_proton_charge_over_electrons_max": lhd::FAST_PROTON_CHARGE_OVER_ELECTRONS_MAX,
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
                "compared": "in every shell at every refinement",
                "refusal": "ProtonLedgerBroken",
            },
            {
                "name": "energy_ledger_residual",
                "rule": "Every keV of the beam energy is counted once: given to electrons, to protons or to boron, carried into fusion, or still held at the stopping energy.",
                "tolerance": lhd::ENERGY_LEDGER_TOLERANCE,
                "unit": "fraction of the beam energy",
                "compared": "in every shell at every refinement",
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
            "covers": "The total alpha emission rate of one case. The test prints the solver's rate over the published one for both readings of the paper's boron density and holds the solver to neither. The served solver answers this case.",
            "kind": "A reference calculation. It is not a measurement.",
        }),
    }
}

/// The validation record, in the five parts a card's description gives: the
/// object, the data, the pass line, the result and the limits. It states
/// each comparison in numbers beside its target.
fn validation() -> [String; 5] {
    let object = "The boron-seeded hydrogen plasma of the Large Helical Device (National Institute for Fusion Science, Toki), toroidal field 2.75 T, magnetic axis 3.60 m, heated by tangential hydrogen neutral beams: the one magnetically confined plasma in which proton-boron-11 alpha particles have been counted. Published in Fusion Sci. Technol. 78, 175; Nat. Commun. 14, 955; Nucl. Fusion 64, 096028.".to_string();
    let data = format!(
        "One published calculation, a reference and not a measurement: 1.5e14 alpha particles per second for the three negative-ion beams of discharge 168156 (180, 165 and 164 keV; 4.33, 2.39 and 3.37 MW deposited; 1.96e19 m^-3; 5.4 keV) with the boron profile of discharge 169626 (Fusion Sci. Technol. 78, 175). Measured: the alpha count rate of one silicon detector under the plasma, discharges 178062 to 178072 (Nucl. Fusion 64, 096028; Nat. Commun. 14, 955), digitised from the papers' figures; the device's data service did not answer. Beam energy dependence: the count rate over boron signal, beam particle rate and slowing-down time at six points, three discharges at two times, beam voltage 127 to 160 kV. Time shape: discharge 178063, the count rate at 4.5, 5.0, 5.5, 6.0, 6.5 and 7.0 s. Nothing fitted."
    );
    let pass_line = format!(
        "Four targets. Against the published calculation: solver over published inside 0.5 to 2.0. Beam energy dependence: after one common scale factor, every point within a factor of 2. Time shape: after one common scale factor, every point within a factor of 2. Collapse under the 45 keV beam: solver ratio at most the measured ratio. {PASS_LINES_FIXED}"
    );
    let result = format!("Against the published calculation, with the boron of the paper's text, 6.3e16 per m^3: alpha rate 1.921e13 per second, solver over published 0.128 (target 0.5 to 2.0). With the boron of the figure's axis, 6.3e17 per m^3: alpha rate 1.950e14 per second, solver over published 1.300. The paper gives its boron density twice and the two differ by ten: about 7e16 m^-3 in its text and colour bar, 6.3e17 by the axis of the profile it says it used; the record takes the text's value. Which value the published calculation used is unknown. The reference disagrees with itself in a second place: its own emissivity map integrates to about 0.4 of its own stated total, so the band of 0.5 to 2.0 is tighter than the reference's agreement with itself. Beam energy dependence: worst factor 2.94 (target 2), Spearman rank correlation 0.943, rise from 128 to 160 kV measured 2.10 and solver 11.1. These runs take the beam voltage of each circle of the bottom panel of the beam-energy figure, where the record names the top panel; with the top panel's voltages the worst factor is 2.953 and the rise 10.7. The solver puts the six points in the measured order and rises five times more steeply with beam energy than the measurement does. The measured rise from 128 to 160 kV, 2.1, is below the 4.4 that a cross-section with no resonance at all gives, so no cross-section shape or averaging in the solver can produce it: the cause lies on the measurement side or in something that neither this solver nor the experimenters' own calculation contains. Time shape: worst factor 2.42 (target 2), run with the injected beam power, the one power the figure gives, where the solver takes deposited power; with the power scaled by the density, a choice made with the figure read, the worst factor is 1.82. Collapse under the 45 keV beam: solver ratio 1.26e-5, measured ratio 0.062 (target: at most the measured ratio); one-sided, since the measured floor is noise. Absolute count rate, with no target: at 6.0 s with 1.3 MW deposited the solver gives 7.0e12 alpha particles per second, 1.28 times the paper's own calculation (about 5.5e12), and through the paper's detection efficiency of 1.2e-7 counts per alpha 845 kcps against a measured 179 kcps, 4.7 times above; the paper's calculation is itself about three times above its measurement. Run through the gateway {RUN_TWICE}. Each answer is saved with its seal and its receipt. {SECOND_SCORING} Fast protons: the fast protons each run leaves in steady state are counted against the electrons. The seven runs of the beam-energy scan, given the deposited power, are answered with the bits they had, the most fast protons 0.78 of the electrons at the axis. The fourteen of the time trace, the 45 keV beam among them, given the injected power or an estimate of the deposited one, leave 1.25 to 7.25 beam protons per electron at the axis, which no plasma can hold beside its electrons, and are refused by name (FastProtonChargeExceedsElectrons): the time shape and the collapse are compared on answers computed before that limit was held. The published calculation's plasma is answered, its fast protons at most 0.16 of the electrons.");
    let limits = "Fitted constants: None. Every number is from the published LHD papers or is a declared assumption of the solver. What the measurement tests: the shape of the answer in beam energy, in time and across the step to a 45 keV beam, each after one free scale factor. The absolute rate is outside what it tests: no absolute alpha rate has been measured in a magnetically confined plasma. The declared range is where the solver's equations hold, each end with its physical reason in its input: a beam above the 10 keV stopping energy to 190 keV, any deposited power above zero, line-averaged electron density 1e18 to 3e19 m^-3, central temperature 0.5 to 6.67 keV, peak boron density zero, or from 0.04973892441153873 m^-3, one boron nucleus in the plasma, to what quasi-neutrality allows in every shell. Inside it the solver stops by name where the fast protons and the boron would outweigh the electrons. Both beam families, the negative-ion beams and the 45 keV beam, share one range, and the published calculation's plasma is inside it. The comparison covered beams of 45 and 127.45 to 160.25 keV, 0.9389 to 4.515 MW, 2.261e18 to 6.872e18 m^-3, 2.1252 to 3.7872 keV and boron of zero and 5.1354e16 to 2.84682e17 m^-3; outside it the same equations answer, compared with no measurement. At every corner of the declared range, 64 of them, the ledgers close or a physical limit stops the solve by name (42 answered). Not in the solver: any other device or beam; beam ions lost by charge exchange or orbits; the time before steady state; how many alphas reach a detector; thermal fusion, which is nil at these temperatures; drag of beam protons on one another. What the shared cross-section was held to is under locked.cross_section.".to_string();
    [object, data, pass_line, result, limits]
}

/// The first measured point of Figure 10 (run A3), the deposited power as given.
fn example() -> Value {
    json!({
        "beam_energy_kev": 159.56, "beam_power_w": 1.3111e6, "electron_density_per_m3": 4.268e18,
        "electron_temperature_kev": 3.7872, "boron_density_per_m3": 1.4298e17
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
/// before a physics permit is taken: the five ranges, the boron floor, and
/// quasi-neutrality in every shell. The sentence is the solver's, with
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
            named("fast_proton_fraction_max", answer.fast_proton_fraction_max),
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
        // The seven refusals the request itself shows are answered with no
        // seal, as the solver's own check names them.
        for (place, refusal) in refusals.iter().enumerate() {
            let reported = if place < 7 { "422, no seal" } else { "200, outcome refused, sealed" };
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
            "fast_proton_fraction_max",
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
        let summary = crate::spec_sheet::summary_of(FUSION_PB11_LHD.name, FUSION_PB11_LHD.summary);
        assert!(summary.contains("Nothing fitted"), "{summary}");
        // The other proton-boron solver's names are not this one's, and the
        // atom publishes neither's.
        assert!(FUSION_PB11_LHD.published_shape("result", "electron_temperature_kev").is_none());
        assert!(
            crate::solver_atom_dft::ATOM_DFT
                .published_shape("result", "fusion_gain")
                .is_none()
        );
        // The power is any above zero; every bound sentence says what was compared and how it did.
        assert_eq!(body["inputs"][1]["minimum"], 0.0);
        assert_eq!(body["inputs"][1]["minimum_exclusive"], true);
        for input in body["inputs"].as_array().unwrap() {
            assert!(input["bound"].as_str().unwrap().contains("Compared"), "{}", input["name"]);
        }
        let limits = &validation()[4];
        assert!(limits.contains("where the solver's equations hold"));
        assert!(limits.contains("Both beam families, the negative-ion beams and the 45 keV beam, share one range"));
        assert!(limits.contains("central temperature 0.5 to 6.67 keV"));
        assert_eq!(body["inputs"][3]["minimum"], 0.5);
        assert!(body["inputs"][1]["bound"].as_str().unwrap().contains("FastProtonChargeExceedsElectrons"));
        assert!(!summary.contains("missed"));
        assert!(summary.contains("Compared with measured alpha count rates under negative-ion beams of 127.45 to 160.25 keV"), "{summary}");
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
            assert_eq!(message, format!("Missing required parameter: '{}' (InputMissing).", input.name));
        }
    }

    /// Each limit the question shows is refused before a permit, by the
    /// solver's name for it, in the solver's own words and order.
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
            refused(&[("beam_energy_kev", json!(10))]),
            "Physical Violation: beam energy 10 keV is not above 10 keV, the energy a proton is followed down to, or is above 190 keV, the rating of the device's negative-ion beams (BeamEnergyOutOfRange)."
        );
        assert_eq!(
            refused(&[("beam_energy_kev", json!(190.5))]),
            "Physical Violation: beam energy 190.5 keV is not above 10 keV, the energy a proton is followed down to, or is above 190 keV, the rating of the device's negative-ion beams (BeamEnergyOutOfRange)."
        );
        assert_eq!(
            refused(&[("beam_power_w", json!(0))]),
            "Physical Violation: beam power 0 W is not above 0 W, which is no beam, or is above 1e300 W, the capacity of the input (BeamPowerOutOfRange)."
        );
        assert_eq!(
            refused(&[("electron_density_per_m3", json!(4.0e19))]),
            "Physical Violation: electron density 40000000000000000000 m^-3 is outside 1e18 to 3e19 m^-3, the device's proton-boron discharges (ElectronDensityOutOfRange)."
        );
        assert_eq!(
            refused(&[("electron_temperature_kev", json!(7))]),
            "Physical Violation: electron temperature 7 keV is outside 0.5 to 6.666666666666667 keV, two thirds of the 10 keV stopping energy: below it boron at the axis is not fully stripped, above it a proton at the stopping energy would be thermal (ElectronTemperatureOutOfRange)."
        );
        // A boron density that is not zero and is under one nucleus in the
        // plasma, in the solver's own words, with the value by its exponent.
        assert_eq!(
            refused(&[("boron_density_per_m3", json!(1.0e-297))]),
            "Physical Violation: boron density 1e-297 m^-3 is not zero and is below 0.04973892441153873 m^-3, the peak density at which the plasma holds one boron nucleus: less than one nucleus is no boron, and zero is answered (BoronBelowOneNucleus)."
        );
        let floor = lhd::BORON_DENSITY_ONE_NUCLEUS_PER_M3;
        assert_eq!(floor, 0.04973892441153873);
        assert!(refused(&[("boron_density_per_m3", json!(f64::from_bits(floor.to_bits() - 1)))]).ends_with("(BoronBelowOneNucleus)."));
        assert!(refused(&[("boron_density_per_m3", json!(-1.0))]).ends_with("(BoronDensityOutOfRange)."));
        // Boron that outweighs the electrons in a shell.
        assert_eq!(
            refused(&[("electron_density_per_m3", json!(1.0e18)), ("boron_density_per_m3", json!(3.0e17))]),
            "Physical Violation: in shell 7 of 20 the boron carries more charge than the electrons: quasi-neutrality cannot hold (BoronChargeExceedsElectrons)."
        );
        // Zero, the floor itself, the worked example, and what 0.7.0 to this version refused at the door are read whole.
        for (field, value) in [
            ("boron_density_per_m3", json!(0)),
            ("boron_density_per_m3", json!(floor)),
            ("boron_density_per_m3", json!(2.0e17)),
            ("beam_energy_kev", json!(45)),
            ("beam_energy_kev", json!(100)),
            ("beam_energy_kev", json!(180)),
            ("beam_power_w", json!(1)),
            ("beam_power_w", json!(6.0e6)),
            ("electron_density_per_m3", json!(1.96e19)),
            ("electron_temperature_kev", json!(5.4)),
            ("electron_temperature_kev", json!(0.5)),
        ] {
            let mut parameters = example();
            parameters[field] = value.clone();
            assert!(read(&parameters).is_ok(), "{field} {value}");
        }
        // The bound and the record give the floor as the solver holds it.
        let bound = FUSION_PB11_LHD.inputs[4].bound;
        assert_eq!(FUSION_PB11_LHD.inputs[4].name, "boron_density_per_m3");
        assert!(bound.contains(&format!("zero, or from {floor} m^-3 to what quasi-neutrality allows")), "{bound}");
        assert!(bound.contains("is refused as BoronBelowOneNucleus"), "{bound}");
        assert!(bound.contains(&format!("in the {:.3} m^3 plasma", lhd::plasma_volume_m3())), "{bound}");
        assert!(validation()[4].contains(&format!("peak boron density zero, or from {floor} m^-3, one boron nucleus in the plasma, to what quasi-neutrality allows")));
        // Two limits at once: the solver asks the power before the density.
        let both = refused(&[("electron_density_per_m3", json!(1.0e17)), ("beam_power_w", json!(0))]);
        assert!(both.ends_with("(BeamPowerOutOfRange)."), "{both}");
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
        // Five fields, and signed, an instrument's signature beside them.
        assert_eq!(parameters["properties"].as_object().unwrap().len(), 6);
        assert_eq!(parameters["properties"]["beam_energy_kev"]["maximum"], 190.0);
        assert_eq!(parameters["properties"]["beam_energy_kev"]["exclusiveMinimum"], 10.0);
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

    /// The validation record states each comparison in numbers beside its
    /// target.
    #[test]
    fn the_validation_record_holds_every_number_of_the_comparison() {
        let [object, data, pass_line, result, limits] = validation();
        assert!(object.contains("Large Helical Device"));
        assert!(data.contains("a reference and not a measurement") && data.contains("digitised from the papers' figures"));
        for mark in ["0.5 to 2.0", "within a factor of 2", "at most the measured ratio", "Record SHA-256 8bd91c5c1328f461a268ccca6dc0f31c61b11ab95051da1fe260fe5c2841db90."] {
            assert!(pass_line.contains(mark), "{mark}");
        }
        for number in [
            "1.921e13", "0.128", "1.950e14", "1.300", "2.94", "0.943", "2.10", "11.1", "2.42",
            "1.82", "1.26e-5", "0.062", "7.0e12", "1.28", "5.5e12", "1.2e-7", "845", "179",
            "4.7 times above", "about 0.4 of its own stated total", "below the 4.4",
            "A second run of the comparison on the same machine recomputes the same numbers.", "twice", "all 58 answers",
        ] {
            assert!(result.contains(number), "{number}");
        }
        assert!(result.starts_with("Against the published calculation"));
        assert_eq!(result.matches("(target 2)").count(), 2);
        assert!(result.contains("solver over published 0.128 (target 0.5 to 2.0)"));
        assert!(limits.starts_with("Fitted constants: None."));
        assert!(limits.contains("The absolute rate is outside what it tests"));
        let body = describe_value(FUSION_PB11_LHD.name).expect("described");
        let cross_section = &body["locked"]["cross_section"];
        assert_eq!(cross_section["against_nacre_ii"]["met"], false);
        assert_eq!(cross_section["against_nacre_ii"]["inside_its_stated_band"], 30);
    }
}
