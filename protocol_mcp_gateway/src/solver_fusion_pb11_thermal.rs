//! The entry for `fusion_pb11_thermal`: the energy balance of a uniform
//! thermal proton-boron-11 plasma. No such plasma has been made: the solver
//! is held to published calculations, never to a measurement. The numbers
//! come from `genesis_solvers` constants, the sentence of each refusal from
//! the solver's source, and the record of the comparisons from the solver's
//! research record.

use genesis_solvers::fusion_pb11_thermal::{
    self as thermal, ThermalBalance, ThermalQuestion, ThermalReject,
};
use genesis_solvers::pb11_reaction as reaction;
use serde_json::{Value, json};

use crate::solver_fusion_pb11_shared::{
    EVIDENCE, PASS_LINES_FIXED, RUN_TWICE, SECOND_SCORING, cross_section_record, number,
};
use crate::solvers::{
    Answer, Given, Input, Kind, Published, Refusal, Refused, Sections, Solved, Solver,
};

/// One number of the question, with both ends of its range inside it.
const fn within(
    name: &'static str,
    minimum: f64,
    maximum: f64,
    unit: &'static str,
    refusal: &'static str,
    bound: &'static str,
) -> Input {
    Input {
        name,
        kind: Kind::Number {
            minimum,
            maximum,
            unit,
        },
        refusal: Some(refusal),
        bound,
        only_when: None,
        asked_by_check: false,
    }
}

pub const FUSION_PB11_THERMAL: Solver = Solver {
    name: "fusion_pb11_thermal",
    summary: "Energy balance of a uniform thermal proton-boron-11 plasma, ion temperature 200 to 450 keV: the electron temperature at which the electrons balance, fusion, bremsstrahlung, synchrotron and exchange power, and the ignition margin. No constant is fitted. No such plasma has been made: it is checked against published calculations only, it answers only inside the inputs of that comparison, and of its eight gated pass lines seven were met and one was missed.",
    inputs: &[
        within(
            "ion_temperature_kev",
            thermal::ION_TEMPERATURE_MIN_KEV,
            thermal::ION_TEMPERATURE_MAX_KEV,
            "keV",
            "IonTemperatureOutOfRange",
            "Temperature of protons, boron and ash. The range is the ion temperatures compared with published calculations: the scan of 200 to 450 keV in steps of 10.",
        ),
        within(
            "electron_density_per_m3",
            thermal::ELECTRON_DENSITY_MIN_PER_M3,
            thermal::ELECTRON_DENSITY_MAX_PER_M3,
            "m^-3",
            "ElectronDensityOutOfRange",
            "Electron density. Ions follow from quasi-neutrality: n_e = n_p + 5 n_B + 2 n_alpha. The range is the electron densities compared: 1.52e20, which is 1e20 fuel ions at 13 percent boron, and 1.6e20.",
        ),
        within(
            "boron_fraction",
            thermal::BORON_FRACTION_MIN,
            thermal::BORON_FRACTION_MAX,
            "",
            "BoronFractionOutOfRange",
            "Boron share of the fuel ions, n_B / (n_p + n_B). The range is the boron shares compared: 0.13 and 0.15.",
        ),
        within(
            "ash_fraction",
            0.0,
            thermal::ASH_FRACTION_MAX,
            "",
            "AshFractionOutOfRange",
            "Thermal alpha ash per fuel ion. No plasma with ash was compared, so only 0 is accepted.",
        ),
        within(
            "magnetic_field_t",
            thermal::MAGNETIC_FIELD_MIN_T,
            thermal::MAGNETIC_FIELD_MAX_T,
            "T",
            "MagneticFieldOutOfRange",
            "Field applied from outside the plasma. Pressure balance is held: the plasma pressure may not exceed B^2 / (2 mu0). The range is the fields compared: the field that leaves 5 T inside the synchrotron case with the kinetic enhancement off, to 20 T.",
        ),
        within(
            "minor_radius_m",
            thermal::MINOR_RADIUS_MIN_M,
            thermal::MINOR_RADIUS_MAX_M,
            "m",
            "MinorRadiusOutOfRange",
            "Minor radius, for the synchrotron loss. Every run of the comparison took 2 m, so only 2 m is accepted.",
        ),
        within(
            "wall_reflectivity",
            thermal::WALL_REFLECTIVITY_MIN,
            thermal::WALL_REFLECTIVITY_MAX,
            "",
            "WallReflectivityOutOfRange",
            "Share of the synchrotron radiation the wall sends back. It has no default: it is poorly known, and the answer turns on it. At 1 there is no synchrotron loss. The range is the reflectivities compared: 0.95 in the one synchrotron case, 1 in every other run.",
        ),
        within(
            "energy_confinement_time_s",
            thermal::CONFINEMENT_TIME_MIN_S,
            thermal::CONFINEMENT_TIME_MAX_S,
            "s",
            "ConfinementTimeOutOfRange",
            "Energy confinement time of the ions. It enters the auxiliary power only. Every run of the comparison took 100 s, and no pass line compares the auxiliary power: only 100 s is accepted.",
        ),
        Input {
            name: "kinetic_enhancement",
            kind: Kind::Flag,
            refusal: None,
            bound: "Required, true or false. It has no default, and a request that leaves it out is refused by name: give false for the Maxwellian reactivity alone. When true the Maxwellian reactivity is multiplied by the factor Ochs et al. (2022) use to stand in for Putvinski's kinetic proton tail, 1.16 at 0 keV falling linearly to 1 at 700 keV. It is theory: a fit to another model's output that nothing has measured. No other input is accepted.",
            only_when: None,
            asked_by_check: false,
        },
    ],
    kept_inside: "The cross-section, the radiation and exchange forms and the tolerances stay inside the solver.",
    result: &[
        number(
            "electron_temperature_kev",
            "keV",
            "Where the electrons' heating equals their radiation. Where more than one temperature does, the first at which the balance, sampled every electron_grid_step_kev upward from 1 keV, turns from heating to loss: the temperature electrons heated from cold reach. A higher balance is not reported.",
        ),
        number(
            "reactivity_m3_per_s",
            "m^3/s",
            "As used: Maxwellian, times the kinetic factor when asked.",
        ),
        number("fusion_power_w_per_m3", "W/m^3", ""),
        number(
            "bremsstrahlung_power_w_per_m3",
            "W/m^3",
            "Svensson (1982), the fit in the balance.",
        ),
        number(
            "bremsstrahlung_power_xie_w_per_m3",
            "W/m^3",
            "Xie (2024) at the same temperature. Not in the balance: it shows how far two published fits differ, which is as much as the ignition margin.",
        ),
        number("synchrotron_power_w_per_m3", "W/m^3", ""),
        number(
            "ion_electron_power_w_per_m3",
            "W/m^3",
            "From ions to electrons.",
        ),
        number("alpha_energy_fraction_to_electrons", "1", ""),
        number(
            "auxiliary_power_w_per_m3",
            "W/m^3",
            "What the ions must be given to hold their temperature. Negative: a surplus. Fusion gain is fusion power over this when it is positive.",
        ),
        number(
            "ignition_margin",
            "1",
            "Fusion power less both radiation losses, over fusion power.",
        ),
        number(
            "ignition_loss_rate_per_s",
            "1/s",
            "The ion energy surplus over the stored energy of ions and electrons. Its inverse is the total-energy confinement time at which the plasma holds itself. It is not one over the ion-only energy_confinement_time_s at which the auxiliary power is zero: the auxiliary power is the ion stored energy over energy_confinement_time_s less the surplus. Zero or below: it cannot. Reported, never held to a published value: it is a small difference of large powers.",
        ),
        number("pressure_pa", "Pa", ""),
        number(
            "beta",
            "1",
            "Plasma pressure over the pressure of the applied field.",
        ),
        number("internal_field_t", "T", ""),
        number("effective_charge", "1", ""),
    ],
    invariants: &[
        number(
            "electron_energy_residual",
            "fraction of the electron heating",
            "",
        ),
        number("total_energy_residual", "fraction of the fusion power", ""),
        number("charge_residual", "fraction of the electron density", ""),
    ],
    iterations: "Halvings of the electron temperature bracket, after the grid that finds it. On a refusal it is the count for `NotSettled` and null otherwise.",
    field: field_words,
    answer: "a balance",
    refusals: REFUSALS,
    sections,
    validation: Some(validation),
    example,
    further_examples: &[],
    parts: &[],
    check: nothing_more_to_check,
    solve,
};

/// Each `ThermalReject` variant with the sentence from its doc comment. The
/// request itself shows a value outside its range, so those eight refusals
/// are not sealed.
const REFUSALS: &[Refusal] = &[
    Refusal {
        name: "IonTemperatureOutOfRange",
        meaning: "`ion_temperature_kev` is outside `ION_TEMPERATURE_MIN_KEV`..=`ION_TEMPERATURE_MAX_KEV`, the ion temperatures compared with published calculations.",
        sealed: false,
    },
    Refusal {
        name: "ElectronDensityOutOfRange",
        meaning: "`electron_density_per_m3` is outside `ELECTRON_DENSITY_MIN_PER_M3`..=`ELECTRON_DENSITY_MAX_PER_M3`, the electron densities compared with published calculations.",
        sealed: false,
    },
    Refusal {
        name: "BoronFractionOutOfRange",
        meaning: "`boron_fraction` is outside `BORON_FRACTION_MIN`..=`BORON_FRACTION_MAX`, the boron fractions compared with published calculations.",
        sealed: false,
    },
    Refusal {
        name: "AshFractionOutOfRange",
        meaning: "`ash_fraction` is below zero or above `ASH_FRACTION_MAX`: no plasma with ash was compared with published calculations.",
        sealed: false,
    },
    Refusal {
        name: "MagneticFieldOutOfRange",
        meaning: "`magnetic_field_t` is outside `MAGNETIC_FIELD_MIN_T`..=`MAGNETIC_FIELD_MAX_T`, the applied fields compared with published calculations.",
        sealed: false,
    },
    Refusal {
        name: "MinorRadiusOutOfRange",
        meaning: "`minor_radius_m` is outside `MINOR_RADIUS_MIN_M`..=`MINOR_RADIUS_MAX_M`, the one minor radius compared with published calculations.",
        sealed: false,
    },
    Refusal {
        name: "WallReflectivityOutOfRange",
        meaning: "`wall_reflectivity` is outside `WALL_REFLECTIVITY_MIN`..=`WALL_REFLECTIVITY_MAX`, the wall reflectivities compared with published calculations.",
        sealed: false,
    },
    Refusal {
        name: "ConfinementTimeOutOfRange",
        meaning: "`energy_confinement_time_s` is outside `CONFINEMENT_TIME_MIN_S`..=`CONFINEMENT_TIME_MAX_S`, the one confinement time compared with published calculations.",
        sealed: false,
    },
    Refusal {
        name: "NonFiniteValue",
        meaning: "An intermediate value was NaN or infinite, or an input was when the solver is called directly: through the gateway no input can be, since a number that is not finite is not JSON.",
        sealed: true,
    },
    Refusal {
        name: "ChargeNotNeutral",
        meaning: "Quasi-neutrality: the ion charges left `CHARGE_TOLERANCE` of the electron density.",
        sealed: true,
    },
    Refusal {
        name: "PressureExceedsField",
        meaning: "Pressure balance: the ions, with the electrons at `ELECTRON_TEMPERATURE_MIN_KEV`, are more than the applied field can hold, or the electron balance, sampled every `ELECTRON_GRID_STEP_KEV` from `ELECTRON_TEMPERATURE_MIN_KEV`, heats the electrons at every point up to the highest electron temperature the field can hold.",
        sealed: true,
    },
    Refusal {
        name: "ElectronTemperatureOutOfRange",
        meaning: "The electron energy balance, sampled every `ELECTRON_GRID_STEP_KEV`, does not turn from heating to loss between `ELECTRON_TEMPERATURE_MIN_KEV` and the lower of the ion temperature and `ELECTRON_TEMPERATURE_MAX_KEV`, the range of the radiation and exchange forms.",
        sealed: true,
    },
    Refusal {
        name: "AlphaSharesBroken",
        meaning: "At a trial temperature the shares of the alpha energy to ions and to electrons left `ALPHA_SHARE_TOLERANCE` of one, or a share was not positive.",
        sealed: true,
    },
    Refusal {
        name: "NotSettled",
        meaning: "The iteration cap was reached with the electron temperature still open.",
        sealed: true,
    },
    Refusal {
        name: "EnergyBalanceBroken",
        meaning: "At the settled temperature an energy balance left `ENERGY_TOLERANCE`.",
        sealed: true,
    },
];

/// The `ThermalReject` variant name, without its fields.
pub fn reject_name(reject: &ThermalReject) -> &'static str {
    match reject {
        ThermalReject::IonTemperatureOutOfRange { .. } => "IonTemperatureOutOfRange",
        ThermalReject::ElectronDensityOutOfRange { .. } => "ElectronDensityOutOfRange",
        ThermalReject::BoronFractionOutOfRange { .. } => "BoronFractionOutOfRange",
        ThermalReject::AshFractionOutOfRange { .. } => "AshFractionOutOfRange",
        ThermalReject::MagneticFieldOutOfRange { .. } => "MagneticFieldOutOfRange",
        ThermalReject::MinorRadiusOutOfRange { .. } => "MinorRadiusOutOfRange",
        ThermalReject::WallReflectivityOutOfRange { .. } => "WallReflectivityOutOfRange",
        ThermalReject::ConfinementTimeOutOfRange { .. } => "ConfinementTimeOutOfRange",
        ThermalReject::NonFiniteValue => "NonFiniteValue",
        ThermalReject::ChargeNotNeutral { .. } => "ChargeNotNeutral",
        ThermalReject::PressureExceedsField { .. } => "PressureExceedsField",
        ThermalReject::ElectronTemperatureOutOfRange { .. } => "ElectronTemperatureOutOfRange",
        ThermalReject::AlphaSharesBroken { .. } => "AlphaSharesBroken",
        ThermalReject::NotSettled { .. } => "NotSettled",
        ThermalReject::EnergyBalanceBroken { .. } => "EnergyBalanceBroken",
    }
}

fn field_words() -> String {
    "SHA-256 of the term count (9) as 8 little-endian bytes, then the terms of the settled balance in W/m^3 as little-endian f64 bytes: fusion, alpha heating of ions, alpha heating of electrons, reactant energy returned to the ions, exchange, bremsstrahlung, synchrotron, conduction, auxiliary.".to_string()
}

fn sections() -> Sections {
    Sections {
        locked: json!({
            "model": "Steady energy balance per species, as set up by Putvinski, Ryutov and Yushmanov (2019) and Ochs et al. (2022). Fusion power from the Maxwellian reactivity; bremsstrahlung by Svensson (1982); synchrotron loss by Trubnikov's formula with the field inside the plasma, B sqrt(1 - beta); ion-electron exchange by Spitzer's rate with the Formulary's Coulomb logarithm and Putvinski's relativistic factor; the alphas' energy split between ions and electrons by slowing-down integrals. The electron temperature is the one at which the electrons' heating equals their radiation; the balance need not fall with temperature, since near the field's limit the field inside and the synchrotron loss go to zero, and where it turns from heating to loss more than once the first turn upward from 1 keV is returned. Heat is conducted away through the ions only. Left out: profiles, particle balance, side reactions, reabsorption, and every limit that belongs to a device.",
            "reaction_energy_kev": reaction::REACTION_ENERGY_KEV,
            "bremsstrahlung_coefficient_w_m3": thermal::bremsstrahlung_coefficient_w_m3(),
            "synchrotron_coeff_mw_per_m3": thermal::SYNCHROTRON_COEFF_MW_PER_M3,
            "kinetic_factor_at_zero": thermal::KINETIC_FACTOR_AT_ZERO,
            "kinetic_factor_ends_kev": thermal::KINETIC_FACTOR_ENDS_KEV,
            "electron_temperature_min_kev": thermal::ELECTRON_TEMPERATURE_MIN_KEV,
            "electron_temperature_max_kev": thermal::ELECTRON_TEMPERATURE_MAX_KEV,
            "electron_grid_step_kev": thermal::ELECTRON_GRID_STEP_KEV,
            "alpha_panels": thermal::ALPHA_PANELS,
            "max_iterations": thermal::MAX_ITERATIONS,
            "not_in_the_solver": "Alpha channeling, avalanche multiplication and spin polarisation. Each is theoretical or disputed and none is offered as an input.",
            "cross_section": cross_section_record(),
        }),
        invariants: json!([
            {
                "name": "electron_energy_residual",
                "rule": "The electrons' energy balances: alpha heating and exchange equal bremsstrahlung and synchrotron loss.",
                "tolerance": thermal::ENERGY_TOLERANCE,
                "unit": "fraction of the electron heating",
                "checked": "at acceptance",
                "refusal": "EnergyBalanceBroken",
            },
            {
                "name": "total_energy_residual",
                "rule": "Energy is conserved over ions and electrons together: the exchange leaves the ions as it reaches the electrons, and the two alpha shares, each from its own integral, add to one. The shares are held to one at every trial temperature (refusal AlphaSharesBroken).",
                "tolerance": thermal::ENERGY_TOLERANCE,
                "unit": "fraction of the fusion power",
                "checked": "every iteration for the shares, at acceptance for the total",
                "refusal": "EnergyBalanceBroken",
            },
            {
                "name": "charge_residual",
                "rule": "Charge is neutral: n_e = n_p + 5 n_B + 2 n_alpha.",
                "tolerance": thermal::CHARGE_TOLERANCE,
                "unit": "fraction of the electron density",
                "checked": "before the first iteration",
                "refusal": "ChargeNotNeutral",
            },
        ]),
        convergence: json!({
            "rule": "The electron balance is sampled every electron_grid_step_kev from electron_temperature_min_kev up to the top of the search, the lowest of the ion temperature, electron_temperature_max_kev and the highest electron temperature the applied field can hold, which is itself sampled. The first interval in which the balance turns from heating to loss is the bracket, and it is halved until its two ends are neighbouring 64-bit numbers.",
            "refusal": "NotSettled",
        }),
        reference: json!({
            "name": thermal::REFERENCE_NAME,
            "url": thermal::REFERENCE_URL,
            "retrieved": thermal::REFERENCE_RETRIEVED,
            "temperature_tolerance": thermal::REFERENCE_TEMPERATURE_TOLERANCE,
            "ratio_tolerance": thermal::REFERENCE_RATIO_TOLERANCE,
            "test": "ochs_2022_electron_temperature_and_ignition_margin",
            "covers": "The electron temperature at 300 keV and 13 percent boron, and the largest fusion power over bremsstrahlung at 15 percent boron with its ion temperature.",
            "kind": "A reference calculation. It is not a measurement.",
        }),
    }
}

/// The record of the comparisons, in the five parts a card's description
/// gives: the object, the data, the pass line, the result and the limits.
/// There is no object and no measurement, and the first two parts say so in
/// their first word: every comparison is with another calculation.
fn validation() -> [String; 5] {
    let object = "None: a uniform thermal plasma of protons, boron-11, alpha ash and electrons at one point. No such plasma has been made. This solver is held to published calculations, never to a measurement.".to_string();
    let data = format!(
        "Nothing. No thermal proton-boron-11 plasma has been made, and no experiment exists at these conditions. Every comparison below is with another calculation. Asked with: Ion density 1e20 m^-3, wall reflectivity 1 (no synchrotron loss), no ash, the kinetic enhancement on, as in Putvinski et al. 2019 and Ochs et al. 2022. Record: {EVIDENCE}."
    );
    let pass_line = format!(
        "Eight gated lines. Five against published calculations, each the largest difference from the published value as a fraction of it: electron temperature at 300 keV, 13 percent boron, 0.05; largest fusion power over bremsstrahlung, 15 percent boron, 0.02; ion temperature of that maximum, 0.10; fusion less bremsstrahlung over fusion at 300 keV, 13 percent boron, 0.50; share of the alpha energy given to electrons at 300 keV, 0.50. Two against the research's own plain-Python model: 5 percent on the electron temperature and 5 percent on synchrotron over fusion power. One on energy and charge closure: 1e-12. Fixed {PASS_LINES_FIXED}, before any forecast was run."
    );
    let result = format!("Seven of the eight gated lines were met and one was missed: the five lines against published calculations, the electron temperature against the research's own model and the closure of energy and charge were met; synchrotron over fusion power against the research's own model was missed. Electron temperature at 300 keV, 13 percent boron: published 163.0 keV, solver 159.33, met. Largest fusion power over bremsstrahlung, 15 percent boron: published 1.028, solver 1.0302, met. Ion temperature of that maximum: published 300.0 keV, solver 300.0, met. Fusion less bremsstrahlung over fusion at 300 keV, 13 percent boron: published 0.04, solver 0.0371, met. Share of the alpha energy given to electrons at 300 keV: published 0.10, solver 0.1163, met. Missed: Against the research's own plain-Python model at 5 T inside the plasma, 2 m, wall reflectivity 0.95, 1.6e20 m^-3, 15 percent boron, 300 keV: electron temperature 113.2 keV against 113 (met, line 5 percent); synchrotron over fusion power 1.263 against 1.17, 7.9 percent off (missed, line 5 percent). The protocol registered this case with the kinetic enhancement off; with it on (not gated) the solver gives 1.163. The second scoring found that the reference value 1.17 was computed with the kinetic enhancement on, while the line was registered with it off: the line stays missed as registered, and the miss is a fault in the protocol's line, not shown to be a fault in the solver. Not gated: Total-energy ignition confinement time at 300 keV, the stored energy of ions and electrons over the ion energy surplus: 532 s at 15 percent boron against a published 550 s, 468 s at 13 percent against 460 s. It is the stored energy over a 2 to 3 percent difference of two powers; the choice among three published bremsstrahlung fits moves it from about 480 to about 1540 s, so no pass line was set. Energy and charge closure: Over the 29 converged thermal answers of the comparison (30 thermal answers with the one sealed refusal) the balances close to 1.2e-15 of the fusion power; the pass line was 1e-12. Run: Through the gateway on 2026-10-09, {RUN_TWICE}. Each answer is saved with its seal and its receipt. {SECOND_SCORING}");
    let limits = "Fitted constants: None. Nothing is measured: agreement with another calculation is not a validation against measurement. The record says nothing of: Whether such a plasma can be made or held; profiles; any device's density or beta limit; the hot-ion modes, alpha channeling, avalanche and spin polarisation of the literature, none of which is in the solver. The declared range is the envelope of the inputs of the comparison, each input on its own: ion temperature 200 to 450 keV, electron density 1.52e20 to 1.6e20 m^-3, boron 0.13 to 0.15, no ash, applied field 6.661 to 20 T, minor radius 2 m, wall reflectivity 0.95 to 1, confinement time 100 s, the kinetic enhancement on or off. Outside it every request is refused by name. Inside it, combinations the runs did not take, such as wall reflectivity 0.95 at 20 T or 13 percent boron at 450 keV, were not compared. The 4 T request of the 2026-10-09 run, a sealed PressureExceedsField then, is now refused at the door as MagneticFieldOutOfRange. What the shared cross-section was held to is under locked.cross_section.".to_string();
    [object, data, pass_line, result, limits]
}

fn example() -> Value {
    json!({
        "ion_temperature_kev": 300, "electron_density_per_m3": 1.6e20, "boron_fraction": 0.15, "ash_fraction": 0,
        "magnetic_field_t": 6.661098700751347, "minor_radius_m": 2, "wall_reflectivity": 0.95, "energy_confinement_time_s": 100,
        "kinetic_enhancement": false
    })
}

/// The eight numbers and the switch are the only inputs, and the bounds of
/// each number are in the field list.
fn nothing_more_to_check(_given: &Given) -> Result<(), String> {
    Ok(())
}

/// The solver's own question, from the values of `parameters`.
fn question(given: &Given) -> Result<ThermalQuestion, String> {
    Ok(ThermalQuestion {
        ion_temperature_kev: given.number("ion_temperature_kev")?,
        electron_density_per_m3: given.number("electron_density_per_m3")?,
        boron_fraction: given.number("boron_fraction")?,
        ash_fraction: given.number("ash_fraction")?,
        magnetic_field_t: given.number("magnetic_field_t")?,
        minor_radius_m: given.number("minor_radius_m")?,
        wall_reflectivity: given.number("wall_reflectivity")?,
        energy_confinement_time_s: given.number("energy_confinement_time_s")?,
        kinetic_enhancement: given.flag("kinetic_enhancement")?,
    })
}

fn solve(given: &Given) -> Result<Solved, String> {
    Ok(solved(thermal::solve(&question(given)?)))
}

/// What the solver returned, in the words every solver shares. A refusal the
/// request itself shows is answered with no seal and with the solver's
/// sentence alone, so its sentence carries the solver's name for it.
pub fn solved(returned: Result<ThermalBalance, ThermalReject>) -> Solved {
    let balance = match returned {
        Ok(balance) => balance,
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
                    ThermalReject::NotSettled { iterations } => Some(u64::from(iterations)),
                    _ => None,
                },
            });
        }
    };
    let named = |name: &str, value: f64| (name.to_string(), Published::Number(value));
    Solved::Answer(Answer {
        iterations: u64::from(balance.iterations),
        result: vec![
            named("electron_temperature_kev", balance.electron_temperature_kev),
            named("reactivity_m3_per_s", balance.reactivity_m3_per_s),
            named("fusion_power_w_per_m3", balance.fusion_power_w_per_m3),
            named(
                "bremsstrahlung_power_w_per_m3",
                balance.bremsstrahlung_power_w_per_m3,
            ),
            named(
                "bremsstrahlung_power_xie_w_per_m3",
                balance.bremsstrahlung_power_xie_w_per_m3,
            ),
            named(
                "synchrotron_power_w_per_m3",
                balance.synchrotron_power_w_per_m3,
            ),
            named(
                "ion_electron_power_w_per_m3",
                balance.ion_electron_power_w_per_m3,
            ),
            named(
                "alpha_energy_fraction_to_electrons",
                balance.alpha_energy_fraction_to_electrons,
            ),
            named("auxiliary_power_w_per_m3", balance.auxiliary_power_w_per_m3),
            named("ignition_margin", balance.ignition_margin),
            named("ignition_loss_rate_per_s", balance.ignition_loss_rate_per_s),
            named("pressure_pa", balance.pressure_pa),
            named("beta", balance.beta),
            named("internal_field_t", balance.internal_field_t),
            named("effective_charge", balance.effective_charge),
        ],
        invariants: vec![
            named("electron_energy_residual", balance.electron_energy_residual),
            named("total_energy_residual", balance.total_energy_residual),
            named("charge_residual", balance.charge_residual),
        ],
        field: balance.balance_terms_w_per_m3,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::solver_fusion_pb11_shared::documented_rejects_of;
    use crate::solvers::{describe_value, solve_schema_in};
    use crate::view::View;

    const THERMAL_SOURCE: &str = include_str!("../../genesis_solvers/src/fusion_pb11_thermal.rs");

    fn read(parameters: &Value) -> Result<Given, String> {
        crate::solution::read_parameters(
            &FUSION_PB11_THERMAL,
            parameters.as_object().expect("an object"),
        )
        .map_err(|error| error.message().to_string())
    }

    #[test]
    fn every_refusal_carries_the_sentence_from_its_doc_comment() {
        let documented = documented_rejects_of(THERMAL_SOURCE, "pub enum ThermalReject {");
        assert_eq!(documented.len(), REFUSALS.len());
        for ((variant, sentence), refusal) in documented.iter().zip(REFUSALS) {
            assert_eq!(variant, refusal.name);
            assert_eq!(sentence, refusal.meaning, "{}", refusal.name);
        }
        let body = describe_value(FUSION_PB11_THERMAL.name).expect("described");
        let refusals = body["refusals"].as_array().expect("refusals");
        assert_eq!(refusals.len(), documented.len());
        // The eight ranges the request itself shows are answered with no seal.
        for (place, refusal) in refusals.iter().enumerate() {
            let reported = if place < 8 { "422, no seal" } else { "200, outcome refused, sealed" };
            assert_eq!(refusal["reported_as"], reported, "{}", refusal["name"]);
        }
    }

    #[test]
    fn the_reference_is_the_one_stored_in_the_solver_test() {
        let body = describe_value(FUSION_PB11_THERMAL.name).expect("described");
        let url = body["reference"]["url"].as_str().expect("url");
        let retrieved = body["reference"]["retrieved"].as_str().expect("retrieved");
        assert!(THERMAL_SOURCE.contains(&format!("<{url}>")), "{url}");
        assert!(THERMAL_SOURCE.contains(&format!("read {retrieved}")), "{retrieved}");
        let test = body["reference"]["test"].as_str().expect("test");
        assert!(THERMAL_SOURCE.contains(&format!("fn {test}()")), "{test}");
        assert_eq!(
            body["reference"]["kind"],
            "A reference calculation. It is not a measurement."
        );
    }

    #[test]
    fn the_description_names_every_input_and_output_once() {
        let body = describe_value(FUSION_PB11_THERMAL.name).expect("described");
        let names = |section: &Value| -> Vec<String> {
            section
                .as_array()
                .expect("list")
                .iter()
                .map(|entry| entry["name"].as_str().expect("name").to_string())
                .collect()
        };
        let inputs = [
            "ion_temperature_kev",
            "electron_density_per_m3",
            "boron_fraction",
            "ash_fraction",
            "magnetic_field_t",
            "minor_radius_m",
            "wall_reflectivity",
            "energy_confinement_time_s",
            "kinetic_enhancement",
        ];
        let result = [
            "electron_temperature_kev",
            "reactivity_m3_per_s",
            "fusion_power_w_per_m3",
            "bremsstrahlung_power_w_per_m3",
            "bremsstrahlung_power_xie_w_per_m3",
            "synchrotron_power_w_per_m3",
            "ion_electron_power_w_per_m3",
            "alpha_energy_fraction_to_electrons",
            "auxiliary_power_w_per_m3",
            "ignition_margin",
            "ignition_loss_rate_per_s",
            "pressure_pa",
            "beta",
            "internal_field_t",
            "effective_charge",
        ];
        let invariants = ["electron_energy_residual", "total_energy_residual", "charge_residual"];
        assert_eq!(names(&body["inputs"]), inputs);
        assert_eq!(names(&body["returns"]["result"]), result);
        assert_eq!(names(&body["returns"]["invariants"]), invariants);
        assert_eq!(names(&body["invariants"]), invariants);
        for name in result {
            assert!(FUSION_PB11_THERMAL.published_shape("result", name).is_some());
            assert!(FUSION_PB11_THERMAL.published_shape("invariants", name).is_none());
        }
        assert!(FUSION_PB11_THERMAL.summary.contains("No constant is fitted"));
        assert!(
            FUSION_PB11_THERMAL
                .published_shape("result", "alpha_emission_rate_per_s")
                .is_none()
        );
        // The thermal solver says in its own words that nothing was measured.
        assert!(body["validation"]["data"].as_str().unwrap().starts_with("Nothing."));
        assert!(body["validation"]["object"].as_str().unwrap().starts_with("None:"));
        assert!(body["validation"]["object"].as_str().unwrap().contains("never to a measurement"));
        assert!(body["inputs"][8]["bound"].as_str().unwrap().contains("It is theory"));
    }

    #[test]
    fn a_question_is_read_whole_or_named() {
        let parameters = example();
        let given = read(&parameters).unwrap_or_else(|message| panic!("{message}"));
        assert!(!question(&given).expect("a question").kinetic_enhancement);
        // Every field is named when it is missing, the switch included: it
        // has no default.
        for input in FUSION_PB11_THERMAL.inputs {
            let mut missing = parameters.clone();
            missing.as_object_mut().unwrap().remove(input.name);
            let Err(message) = read(&missing) else {
                panic!("{} was filled in", input.name)
            };
            assert_eq!(message, format!("Missing required parameter: '{}'", input.name));
        }
        let mut on = parameters.clone();
        on["kinetic_enhancement"] = json!(true);
        let given = read(&on).unwrap_or_else(|message| panic!("{message}"));
        assert!(question(&given).expect("a question").kinetic_enhancement);
        on["kinetic_enhancement"] = json!(1);
        assert_eq!(
            read(&on).err().expect("a number is not a switch"),
            "Physical Violation: 'kinetic_enhancement' must be true or false; this request has 1."
        );
    }

    /// Each range is refused at its field, by the solver's name for it.
    #[test]
    fn each_range_of_the_question_is_named_at_the_door() {
        for (field, value, sentence) in [
            ("ion_temperature_kev", json!(451), "'ion_temperature_kev' 451 is outside 200..=450 keV (IonTemperatureOutOfRange)."),
            ("electron_density_per_m3", json!(1.0e20), "'electron_density_per_m3' 1e+20 is outside 152000000000000000000..=160000000000000000000 m^-3 (ElectronDensityOutOfRange)."),
            ("boron_fraction", json!(0.2), "'boron_fraction' 0.2 is outside 0.13..=0.15 (BoronFractionOutOfRange)."),
            ("ash_fraction", json!(0.01), "'ash_fraction' 0.01 is outside 0..=0 (AshFractionOutOfRange)."),
            ("magnetic_field_t", json!(5), "'magnetic_field_t' 5 is outside 6.661098700751347..=20 T (MagneticFieldOutOfRange)."),
            ("minor_radius_m", json!(1.0), "'minor_radius_m' 1.0 is outside 2..=2 m (MinorRadiusOutOfRange)."),
            ("wall_reflectivity", json!(0.9), "'wall_reflectivity' 0.9 is outside 0.95..=1 (WallReflectivityOutOfRange)."),
            ("energy_confinement_time_s", json!(50), "'energy_confinement_time_s' 50 is outside 100..=100 s (ConfinementTimeOutOfRange)."),
        ] {
            let mut parameters = example();
            parameters[field] = value;
            let Err(message) = read(&parameters) else {
                panic!("{field} was not refused")
            };
            assert_eq!(message, format!("Physical Violation: {sentence}"));
        }
    }

    #[test]
    fn the_schema_requires_a_lone_solver_s_fields_and_lists_them_otherwise() {
        let lone = Value::Object(solve_schema_in(&View::limited(
            [],
            [FUSION_PB11_THERMAL.name.to_string()],
        )));
        assert_eq!(lone["properties"]["solver"], json!({ "const": "fusion_pb11_thermal" }));
        let parameters = &lone["properties"]["parameters"];
        // Nine fields and nine required: the switch has no default.
        assert_eq!(parameters["required"].as_array().unwrap().len(), 9);
        assert_eq!(parameters["properties"].as_object().unwrap().len(), 9);
        assert_eq!(parameters["properties"]["wall_reflectivity"]["maximum"], 1.0);
        assert_eq!(parameters["properties"]["kinetic_enhancement"], json!({ "type": "boolean" }));
        assert!(!lone.to_string().contains("beam_energy_kev"));
        let atom = Value::Object(solve_schema_in(&View::limited([], ["atom_dft".to_string()])));
        assert!(!atom.to_string().contains("wall_reflectivity"));
    }

    #[test]
    fn a_refusal_the_solver_finds_is_sealed_and_a_range_refusal_is_not() {
        let request = json!({ "solver": FUSION_PB11_THERMAL.name, "parameters": example() });
        let response = crate::solution::seal(
            &FUSION_PB11_THERMAL,
            request.clone(),
            solved(Err(ThermalReject::PressureExceedsField { pressure_pa: 9.0e6 })),
        )
        .expect("sealed");
        let body: Value = serde_json::from_str(&serde_json::to_string(&response).unwrap()).unwrap();
        assert_eq!(body["outcome"], "refused");
        assert_eq!(body["reject"], "PressureExceedsField");
        assert_eq!(
            body["reject_detail"],
            "Solver Refusal: fusion_pb11_thermal did not return a balance (PressureExceedsField). the plasma pressure, 9000000 Pa or more, is above what the applied field can hold."
        );
        assert!(body["iterations"].is_null());
        assert!(
            crate::solution::check(&body)
                .unwrap_or_else(|_| panic!("refused at the door"))
                .holds
        );
        let response = crate::solution::seal(
            &FUSION_PB11_THERMAL,
            request.clone(),
            solved(Err(ThermalReject::NotSettled { iterations: 200 })),
        )
        .expect("sealed");
        assert_eq!(response.document.iterations, Some(200));
        let Err(unsealed) = crate::solution::seal(
            &FUSION_PB11_THERMAL,
            request,
            solved(Err(ThermalReject::WallReflectivityOutOfRange { wall_reflectivity: 2.0 })),
        ) else {
            panic!("a limit the request shows was sealed")
        };
        assert_eq!(
            unsealed,
            "Physical Violation: wall reflectivity 2 is outside 0.95..=1, the wall reflectivities compared with published calculations (WallReflectivityOutOfRange)."
        );
    }

    /// The record states each comparison as it came out, the one that
    /// missed included.
    #[test]
    fn the_validation_record_holds_every_number_of_the_comparison() {
        let [object, data, pass_line, result, limits] = validation();
        assert!(object.starts_with("None:"));
        assert!(data.starts_with("Nothing."));
        for mark in ["0.05", "0.02", "0.10", "0.50", "5 percent", "1e-12", "8bd91c5c1328f461a268ccca6dc0f31c61b11ab95051da1fe260fe5c2841db90", "2026-10-09T20:51:55Z"] {
            assert!(pass_line.contains(mark), "{mark}");
        }
        for number in [
            "163.0", "159.33", "1.028", "1.0302", "300.0", "0.04", "0.0371", "0.10", "0.1163",
            "113.2", "1.263", "1.17", "7.9 percent off", "1.163", "532", "550", "468", "460",
            "480", "1540", "1.2e-15", "29 converged", "computed with the kinetic enhancement on",
            "evidence/checks/second_party/V-26/", "twice", "all 58 answers",
        ] {
            assert!(result.contains(number), "{number}");
        }
        // Five published comparisons, each met, and the one that missed.
        assert_eq!(result.matches(", met.").count(), 5);
        assert!(result.contains("(missed, line 5 percent)"));
        assert!(result.starts_with("Seven of the eight gated lines were met and one was missed"));
        assert!(result.contains("a fault in the protocol's line, not shown to be a fault in the solver"));
        assert!(FUSION_PB11_THERMAL.summary.contains("of its eight gated pass lines seven were met and one was missed"));
        assert!(limits.starts_with("Fitted constants: None."));
        let body = describe_value(FUSION_PB11_THERMAL.name).expect("described");
        let cross_section = &body["locked"]["cross_section"];
        assert_eq!(cross_section["against_nacre_ii"]["met"], false);
        assert_eq!(cross_section["against_nacre_ii"]["inside_its_stated_band"], 30);
    }
}
