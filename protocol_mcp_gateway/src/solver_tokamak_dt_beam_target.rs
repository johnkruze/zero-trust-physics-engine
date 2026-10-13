//! The entry for `tokamak_dt_beam_target`: the deuterium-tritium fusion rate
//! of one time slice of a tokamak pulse from its profiles and its beams. The
//! numbers come from `genesis_solvers` constants, the sentence of each
//! refusal from the solver's source, and the validation record from the
//! solver's research record.

use genesis_solvers::bosch_hale::{self, Reaction};
use genesis_solvers::tokamak_dt_beam_target::{self as slice, SliceFusion, SliceQuestion, SliceReject};
use serde_json::{Value, json};

use crate::solvers::{
    Answer, Given, Input, Kind, Output, Published, Refusal, Refused, Sections, Shape, Solved, Solver,
};

/// Where the record of the comparison is kept.
pub const EVIDENCE: &str = "evidence/research/tokamak_dt_beam_target/";

/// When the pass marks were fixed, and the file that holds them.
pub const PASS_MARKS_FIXED: &str = "2026-10-10T01:32:38Z, SHA-256 b507ab0a291ca266ce0a9293f39d3d7efbdb60b8a7a3ee093e68ee87603fad91 of validation_protocol.md, before any TFTR file was fetched";

const fn number(name: &'static str, unit: &'static str, note: &'static str) -> Output {
    Output { name, unit, shape: Shape::Number, each: &[], note, asked_by: None }
}

const fn per_shell(name: &'static str, minimum: f64, maximum: f64, unit: &'static str, refusal: &'static str, bound: &'static str) -> Input {
    Input {
        name,
        kind: Kind::Numbers { fewest: 1, most: slice::SHELLS_MAX, minimum, maximum, unit },
        refusal: Some(refusal),
        bound,
        only_when: None,
        asked_by_check: true,
    }
}

const fn per_component(name: &'static str, minimum: f64, maximum: f64, unit: &'static str, refusal: &'static str, bound: &'static str) -> Input {
    Input {
        name,
        kind: Kind::Numbers { fewest: 1, most: slice::BEAM_COMPONENTS_MAX, minimum, maximum, unit },
        refusal: Some(refusal),
        bound,
        only_when: None,
        asked_by_check: true,
    }
}

const fn one(name: &'static str, minimum: f64, maximum: f64, unit: &'static str, refusal: &'static str, bound: &'static str) -> Input {
    Input { name, kind: Kind::Number { minimum, maximum, unit }, refusal: Some(refusal), bound, only_when: None, asked_by_check: true }
}

pub const TOKAMAK_DT_BEAM_TARGET: Solver = Solver {
    name: "tokamak_dt_beam_target",
    summary: SUMMARY,
    inputs: &[
        per_shell("shell_volume_m3", slice::SHELL_VOLUME_MIN_M3, slice::SHELL_VOLUME_MAX_M3, "m^3", "ShellVolumeOutOfRange", "Volume of each nested radial shell, from the magnetic axis outward: one time slice of one pulse is 1 to 128 shells. Every per-shell list holds one value for each shell. The shell count, the volume and the whole beam power are capacities and no range of the physics: each shell is solved alone, a shell split in two gives the same rates and the same fast-ion share, and what the physics sees is the densities and the power over the volume, which the fast-ion share and the charge balance hold. The shells compared were 0.0070 to 7.0 m^3, 20 to 80 to a slice. The floor, 1e-6 m^3, keeps the power over the volume finite."),
        per_shell("electron_temperature_kev", slice::ELECTRON_TEMPERATURE_MIN_KEV, slice::ELECTRON_TEMPERATURE_MAX_KEV, "keV", "ElectronTemperatureOutOfRange", "Electron temperature of each shell: the span of the slices compared and answered on engine 0.6.0, rounded outward in the third figure. The electron over ion temperature of each shell is held to 0.0730 to 2.0, the span of the shells compared (TemperatureRatioOutOfRange)."),
        per_shell("ion_temperature_kev", slice::ION_TEMPERATURE_MIN_KEV, slice::ION_TEMPERATURE_MAX_KEV, "keV", "IonTemperatureOutOfRange", "Ion temperature of each shell: the span of the slices compared and answered on engine 0.6.0, rounded outward in the third figure. It holds TFTR's supershots, to 41.8 keV."),
        per_shell("electron_density_per_m3", slice::ELECTRON_DENSITY_MIN_PER_M3, slice::ELECTRON_DENSITY_MAX_PER_M3, "m^-3", "ElectronDensityOutOfRange", "Electron density of each shell: the span of the slices compared and answered on engine 0.6.0, rounded outward in the third figure."),
        per_shell("deuterium_density_per_m3", 0.0, slice::ELECTRON_DENSITY_MAX_PER_M3, "m^-3", "IonDensityNegative", "Thermal deuterium density of each shell."),
        per_shell("tritium_density_per_m3", 0.0, slice::ELECTRON_DENSITY_MAX_PER_M3, "m^-3", "IonDensityNegative", "Thermal tritium density of each shell. The tritium share of the fuel, tritium over deuterium plus tritium, is held in every shell to 0.00467 to 0.998, the span of the shells compared (TritiumShareOutOfRange): no slice compared is free of tritium or of deuterium."),
        per_shell("impurity_density_per_m3", 0.0, slice::ELECTRON_DENSITY_MAX_PER_M3, "m^-3", "IonDensityNegative", "Density of the one impurity in each shell. Quasi-neutrality: at the door the thermal ions alone, deuterium plus tritium plus the impurity's charge, may exceed the electrons of a shell by 2.5 percent and no more (QUASI_NEUTRALITY_ALLOWANCE; ThermalChargeExceedsElectrons), the closure error of the profile database's own densities, 7.53 percent under to 2.40 percent over, rounded up. After the solve the thermal ions and the fast ions this solve finds must stand between 21.5 percent under the electrons (QUASI_NEUTRALITY_DEFICIT_ALLOWANCE; IonChargeBelowElectrons) and 12 percent over them (QUASI_NEUTRALITY_FAST_ION_ALLOWANCE; IonChargeExceedsElectrons), the extremes reached by the slices compared and answered on engine 0.6.0 (TFTR 74898 at 3.12 s, 21.09 percent under; TFTR 83526 at 4.73 s, 11.86 percent over), rounded outward; both are sealed refusals. The band is that wide for two reasons found on TFTR and not mended: the solver's fast ions are 1.1 to 3.1 times the database's in the shell of largest excess (local deposition, no orbit spread, no drag on other fast ions), and the question takes no thermal hydrogen, which the database's analysis holds at 4 to 22 percent of the electrons at the edge. Inside the band the solve uses the densities as given: nothing is trimmed and nothing is filled in. What an excess of the thermal ions does: on the eight slices of JET 43002 the database's own excess, up to 1.37 percent in up to 17 of 50 shells, moves the fusion power by at most 0.11 percent against the same slice made neutral; thermal charge worth the whole 2.5 percent of the electrons added in every shell moves it by +1.8 to +4.0 percent when added as fuel and by -1.1 to -3.1 percent when added as impurity (four slices; analysis/out/13_quasi_neutrality_effect.log)."),
        one("impurity_charge", slice::IMPURITY_CHARGE_MIN, slice::IMPURITY_CHARGE_MAX, "1", "ImpurityChargeOutOfRange", "Charge number of the impurity: 6 for carbon. A mean over several impurities need not be whole. The span of the slices compared: no other impurity is in the data on disk."),
        one("impurity_mass_u", slice::IMPURITY_MASS_MIN_U, slice::IMPURITY_MASS_MAX_U, "u", "ImpurityMassOutOfRange", "Mass number of the impurity: 12 for carbon. The span of the slices compared."),
        per_shell("beam_power_w", 0.0, slice::BEAM_POWER_MAX_W, "W", "BeamPowerOutOfRange", BEAM_POWER_BOUND),
        per_component("beam_mass_number", 2.0, 3.0, "u", "BeamSpeciesUnknown", "One value for each beam component, 1 to 12 of them: 2 for deuterium, 3 for tritium, and nothing between. A beam line is three components, at its full, half and third energy. The component count is a capacity: a component split in two gives the same answer."),
        per_component("beam_energy_kev", slice::BEAM_ENERGY_MIN_KEV, slice::BEAM_ENERGY_MAX_KEV, "keV", "BeamEnergyOutOfRange", "Energy of each component: the span of the slices compared. No saved question holds a component outside it."),
        per_component("beam_power_share", 0.0, 1.0, "1", "BeamPowerShareOutOfRange", "Share of the deposited beam power in each component, the same in every shell. Above zero, and the shares add to one within 1e-9."),
        one("time_since_beam_change_s", 0.0, slice::TIME_SINCE_BEAM_CHANGE_MAX_S, "s", "TimeSinceBeamChangeOutOfRange", TIME_SINCE_BEAM_CHANGE_BOUND),
    ],
    kept_inside: "The cross-sections, the slowing-down rates, the stopping energy and the tolerances stay inside the solver. Plasma current, field, minor radius and any limit of a machine's operating point (Greenwald, Troyon) are not inputs: the question is one time slice of profiles.",
    result: &[
        number("thermal_dt_rate_per_s", "1/s", "D-T reactions between thermal ions."),
        number("beam_target_dt_rate_per_s", "1/s", "D-T reactions of beam ions with thermal ions."),
        number("beam_beam_dt_rate_per_s", "1/s", "D-T reactions of fast deuterons with fast tritons, taken as isotropic."),
        number("dt_rate_per_s", "1/s", "All D-T reactions: the 14 MeV neutrons per second."),
        number("dt_fusion_power_w", "W", "D-T reactions per second times 17.589 MeV."),
        number("dd_neutron_rate_per_s", "1/s", "The neutron branch of D-D, thermal and beam-target."),
        number("dd_fusion_power_w", "W", "Both branches of D-D, thermal and beam-target."),
        number("neutron_rate_per_s", "1/s", "D-T and D-D neutrons."),
        number("fast_ion_count", "1", "Beam ions still slowing, in the whole plasma."),
        number("fast_ion_energy_j", "J", "Their kinetic energy."),
        number("fast_ion_energy_time_s", "s", "Their kinetic energy over the deposited beam power. The caller's time_since_beam_change_s is held to five of it (BeamChangeTooRecent)."),
        number("beam_power_to_electrons_w", "W", ""),
        number("beam_power_to_ions_w", "W", "By drag on deuterium, tritium and the impurity."),
        number("beam_power_thermalised_w", "W", "What the ions still hold when they join the thermal plasma at 1.5 ion temperatures."),
        number("beam_target_fused_fraction", "1", "Share of beam ions that fuse with thermal ions before they stop."),
        number("beam_beam_fused_fraction", "1", "Beam-beam reactions over beam ions born. Outside the ledgers."),
        number("fast_ion_fraction_max", "1", "Largest fast-ion density over electron density of any shell. Above 0.439, the largest share on a slice compared and answered, no answer is returned (FastIonShareTooLarge)."),
        number("charge_excess_max", "1", "Largest of (thermal ion charge + fast ions - electrons) / electrons over the shells. Above 0.12, the largest on a slice compared and answered, no answer is returned (IonChargeExceedsElectrons): the fast ions the solver finds cannot be there beside the measured electrons."),
        number("charge_excess_min", "1", "Smallest of the same over the shells. Below -0.215, the lowest on a slice compared and answered, no answer is returned (IonChargeBelowElectrons): the question leaves out ions, such as hydrogen, that would slow and meet the beam."),
        Output { name: "dt_rate_density_per_m3_s", unit: "1/(m^3 s)", shape: Shape::Numbers, each: &[], note: "D-T reactions of all three kinds in each shell, from the axis outward.", asked_by: None },
    ],
    invariants: &[
        number("particle_ledger_residual", "1", "Largest over shells and components."),
        number("energy_ledger_residual", "fraction of the birth energy", "Largest over shells and components."),
        number("power_ledger_residual", "fraction of the beam power", "Of the whole plasma."),
    ],
    iterations: "Refinements taken by the shell that took the most. On a refusal it is the count for `NotSettled` and null otherwise.",
    field: field_words,
    answer: "a fusion rate",
    refusals: REFUSALS,
    sections,
    validation: Some(validation),
    example,
    further_examples: &[],
    parts: &[],
    check,
    solve,
};

/// What `beam_power_w` must be, what the comparisons fed it instead, and
/// what the ceiling refused.
const BEAM_POWER_BOUND: &str = "Beam power deposited in each shell in steady state, all components together: the power the beams inject less shine-through, charge-exchange and orbit loss. The whole must be above zero and at most 4.5e7 W, a property of the two machines: no pulse of the profile database injects more than 39.6 MW, and a steady plasma takes up less than is injected. What the comparisons of the validation record fed it is not that quantity but TRANSP's beam heating profile (QNBIE + QNBII), the power the fast ions hand to the plasma at that instant. It lags the injection (0.18 of it 0.05 s after the beams come on, 0.53 at 0.15 s, 0.87 in the median while the beams are on) and exceeds it while fast ions give up stored energy in a collapse. At the peak of TFTR's record pulse 80539, 3.81 to 3.91 s, it sums to 46 to 57 MW against 39.5 MW injected, and those slices are refused by the ceiling. Heating above the injected power but under the ceiling is not seen by the solver (JET 42976 at 13.41 s: 36 MW against 21 MW injected). Each beam-target rate is proportional to this input and the beam-beam rate to its square.";

/// The required statement of how long the beams have been steady.
const TIME_SINCE_BEAM_CHANGE_BOUND: &str = "Seconds since the injected beam power last differed from its present value by more than a fifth of it (BEAM_CHANGE_FRACTION, 0.2). Required, because the model is steady state. The solve finds the fast ions' energy time, their kinetic energy over the deposited beam power (returned as fast_ion_energy_time_s; 0.03 to 0.18 s on the slices compared at beam power), and returns no answer when this input is less than five of them (SETTLED_AFTER_ENERGY_TIMES; refusal BeamChangeTooRecent, sealed, with both times in its sentence); exactly five is answered. The solver cannot check this statement: it is given one slice and no history. It also cannot see a plasma that is changing under steady beams, such as a collapse. From 0 to 100 s; the upper end bounds the input and is no limit of the physics. No other input is accepted.";

/// The one line an agent reads in the list: what it is, that nothing is
/// fitted, how the held-back comparison came out, and the limits written
/// in after it.
const SUMMARY: &str = "The deuterium-tritium fusion rate of one time slice of a JET or TFTR pulse in steady state, from its analysed profiles (temperatures, densities, fuel mix in up to 128 radial shells) and its neutral beams: thermal, beam-target from classical slowing-down with the Bosch-Hale cross-section, and beam-beam reactions, the fusion power and the neutron rate. No constant is fitted. Its held-back comparison failed: on the 34 slices of every TFTR tritium pulse of the profile database, another machine, the median of solver over database fusion power was 1.02, but 30 of the 34 lay inside 0.75 to 1.25 where nine in ten were asked (second scoring V-42: the same numbers). The reference is TRANSP's calculation on the same profiles and beam heating the solver is given, so agreement shows the solver's slowing-down arithmetic matches TRANSP's on TRANSP's own plasma and nothing about the neutron counters; against four measured values the solver is +7, +21, -11 and about +8 percent. After that failure limits were written in, on data by then spent and so tested blind by nothing, and are enforced by name. Since engine 0.7.0 the fast ions the solver finds are counted in each shell's charge balance and held from both sides at the extremes the slices compared and answered reach (no more than 12 percent over or 21.5 percent under the electrons), and the ranges of the physics are the span of those slices. The band is wide, and that is a finding: on TFTR the solver's own fast ions are 1.1 to 3.1 times the database's in the shell of largest excess, and the questions carry no thermal hydrogen, which TRANSP holds at 4 to 22 percent of the electrons at the edge. Of the 34 TFTR slices, 23 are answered on 0.7.0 as on 0.6.0, with the same bits: median 1.088, 21 of 23 inside 0.75 to 1.25 (1.015 and 23 of 23 with pulse 73268 against five times its database value, an alpha power). On JET the same 53 slices are answered, median 0.934, all inside the band.";

/// Each `SliceReject` variant with the sentence from its doc comment. The
/// request itself shows the first eighteen, which are not sealed.
const REFUSALS: &[Refusal] = &[
    Refusal {
        name: "ShellCountOutOfRange",
        meaning: "There is no shell, or more than `SHELLS_MAX`.",
        sealed: false,
    },
    Refusal {
        name: "ProfileLengthsDiffer",
        meaning: "A profile does not hold one value for each shell.",
        sealed: false,
    },
    Refusal {
        name: "BeamComponentCountOutOfRange",
        meaning: "There is no beam component, or more than `BEAM_COMPONENTS_MAX`, or the three beam lists differ in length.",
        sealed: false,
    },
    Refusal {
        name: "ShellVolumeOutOfRange",
        meaning: "A shell volume is outside `SHELL_VOLUME_MIN_M3`..=`SHELL_VOLUME_MAX_M3`.",
        sealed: false,
    },
    Refusal {
        name: "ElectronTemperatureOutOfRange",
        meaning: "An electron temperature is outside `ELECTRON_TEMPERATURE_MIN_KEV`..=`ELECTRON_TEMPERATURE_MAX_KEV`, the span of the slices compared.",
        sealed: false,
    },
    Refusal {
        name: "IonTemperatureOutOfRange",
        meaning: "An ion temperature is outside `ION_TEMPERATURE_MIN_KEV`..=`ION_TEMPERATURE_MAX_KEV`, the span of the slices compared.",
        sealed: false,
    },
    Refusal {
        name: "TemperatureRatioOutOfRange",
        meaning: "The electron temperature of a shell over its ion temperature is outside `TEMPERATURE_RATIO_MIN`..=`TEMPERATURE_RATIO_MAX`, the span of the slices compared.",
        sealed: false,
    },
    Refusal {
        name: "ElectronDensityOutOfRange",
        meaning: "An electron density is outside `ELECTRON_DENSITY_MIN_PER_M3`..=`ELECTRON_DENSITY_MAX_PER_M3`, the span of the slices compared.",
        sealed: false,
    },
    Refusal {
        name: "IonDensityNegative",
        meaning: "A deuterium, tritium or impurity density is below zero.",
        sealed: false,
    },
    Refusal {
        name: "TritiumShareOutOfRange",
        meaning: "The tritium share of the fuel of a shell, n_T / (n_D + n_T), is outside `TRITIUM_SHARE_MIN`..=`TRITIUM_SHARE_MAX`, the span of the slices compared: no slice compared is free of tritium or of deuterium.",
        sealed: false,
    },
    Refusal {
        name: "ImpurityChargeOutOfRange",
        meaning: "`impurity_charge` is outside `IMPURITY_CHARGE_MIN`..=`IMPURITY_CHARGE_MAX`, the span of the slices compared.",
        sealed: false,
    },
    Refusal {
        name: "ImpurityMassOutOfRange",
        meaning: "`impurity_mass_u` is outside `IMPURITY_MASS_MIN_U`..=`IMPURITY_MASS_MAX_U`, the span of the slices compared.",
        sealed: false,
    },
    Refusal {
        name: "ThermalChargeExceedsElectrons",
        meaning: "Quasi-neutrality, at the door: in one shell the thermal ions alone carry more charge than the electrons there by more than `QUASI_NEUTRALITY_ALLOWANCE` of the electron density, with `QUASI_NEUTRALITY_END_ALLOWANCE_ULP` units in the last place for rounding. Fast ions only add to that charge.",
        sealed: false,
    },
    Refusal {
        name: "BeamPowerOutOfRange",
        meaning: "A shell's deposited beam power is below zero, or the whole is not above zero or is above `BEAM_POWER_MAX_W`.",
        sealed: false,
    },
    Refusal {
        name: "BeamSpeciesUnknown",
        meaning: "A beam mass number is neither 2 nor 3.",
        sealed: false,
    },
    Refusal {
        name: "BeamEnergyOutOfRange",
        meaning: "A beam energy is outside `BEAM_ENERGY_MIN_KEV`..=`BEAM_ENERGY_MAX_KEV`, the span of the slices compared.",
        sealed: false,
    },
    Refusal {
        name: "BeamPowerShareOutOfRange",
        meaning: "A beam power share is not above zero, or the shares do not add to one within `SHARE_SUM_ALLOWANCE`.",
        sealed: false,
    },
    Refusal {
        name: "TimeSinceBeamChangeOutOfRange",
        meaning: "`time_since_beam_change_s` is outside 0..=`TIME_SINCE_BEAM_CHANGE_MAX_S`.",
        sealed: false,
    },
    Refusal {
        name: "NonFiniteValue",
        meaning: "An intermediate value was NaN or infinite, or an input was when the solver is called directly: through the gateway no input can be, since a number that is not finite is not JSON.",
        sealed: true,
    },
    Refusal {
        name: "SlowingRateNotPositive",
        meaning: "At some energy between birth and stopping a beam ion gains energy from the plasma of a shell: the electrons there heat it more than the thermal ions slow it.",
        sealed: true,
    },
    Refusal {
        name: "NotSettled",
        meaning: "The refinement cap was reached with a fused share or the beam-beam rate of a shell still moving.",
        sealed: true,
    },
    Refusal {
        name: "ParticleLedgerBroken",
        meaning: "For one component in one shell the slowed and the fused shares left `PARTICLE_LEDGER_TOLERANCE` of one.",
        sealed: true,
    },
    Refusal {
        name: "EnergyLedgerBroken",
        meaning: "For one component in one shell the energy ledger left `ENERGY_LEDGER_TOLERANCE` of the birth energy.",
        sealed: true,
    },
    Refusal {
        name: "PowerLedgerBroken",
        meaning: "The beam power of the whole plasma and where it went differ by more than `POWER_LEDGER_TOLERANCE` of it.",
        sealed: true,
    },
    Refusal {
        name: "BeamBeamShareTooLarge",
        meaning: "Beam-beam reactions take more than `BEAM_BEAM_SHARE_MAX` of the beam ions, and the ledgers do not count them.",
        sealed: true,
    },
    Refusal {
        name: "FastIonShareTooLarge",
        meaning: "In one shell the fast ions this solve finds are more than `FAST_ION_SHARE_MAX` of the electrons, the largest share on a slice compared: a beam ion is slowed on thermal ions only, and other fast ions are not there for it.",
        sealed: true,
    },
    Refusal {
        name: "BeamChangeTooRecent",
        meaning: "The beam power changed less than `SETTLED_AFTER_ENERGY_TIMES` fast-ion energy times before this slice, by the caller's `time_since_beam_change_s` and the energy time this solve finds: the fast ions have not settled, and the model is steady state.",
        sealed: true,
    },
    Refusal {
        name: "IonChargeExceedsElectrons",
        meaning: "Quasi-neutrality, after the solve: in one shell the thermal ions and the fast ions this solve finds carry more charge than the electrons there by more than `QUASI_NEUTRALITY_FAST_ION_ALLOWANCE` of the electron density, the largest excess on a slice compared: the fast ions the model finds cannot be there beside the measured electrons and the given thermal ions.",
        sealed: true,
    },
    Refusal {
        name: "IonChargeBelowElectrons",
        meaning: "Quasi-neutrality, after the solve: in one shell the thermal ions and the fast ions this solve finds carry less charge than the electrons there by more than `QUASI_NEUTRALITY_DEFICIT_ALLOWANCE` of the electron density, the largest shortfall on a slice compared: the question leaves out ions, such as hydrogen, that would slow and meet the beam.",
        sealed: true,
    },
];

/// The `SliceReject` variant name, without its fields.
pub fn reject_name(reject: &SliceReject) -> &'static str {
    match reject {
        SliceReject::ShellCountOutOfRange { .. } => "ShellCountOutOfRange",
        SliceReject::ProfileLengthsDiffer { .. } => "ProfileLengthsDiffer",
        SliceReject::BeamComponentCountOutOfRange { .. } => "BeamComponentCountOutOfRange",
        SliceReject::ShellVolumeOutOfRange { .. } => "ShellVolumeOutOfRange",
        SliceReject::ElectronTemperatureOutOfRange { .. } => "ElectronTemperatureOutOfRange",
        SliceReject::IonTemperatureOutOfRange { .. } => "IonTemperatureOutOfRange",
        SliceReject::TemperatureRatioOutOfRange { .. } => "TemperatureRatioOutOfRange",
        SliceReject::ElectronDensityOutOfRange { .. } => "ElectronDensityOutOfRange",
        SliceReject::IonDensityNegative { .. } => "IonDensityNegative",
        SliceReject::TritiumShareOutOfRange { .. } => "TritiumShareOutOfRange",
        SliceReject::ImpurityChargeOutOfRange { .. } => "ImpurityChargeOutOfRange",
        SliceReject::ImpurityMassOutOfRange { .. } => "ImpurityMassOutOfRange",
        SliceReject::ThermalChargeExceedsElectrons { .. } => "ThermalChargeExceedsElectrons",
        SliceReject::BeamPowerOutOfRange { .. } => "BeamPowerOutOfRange",
        SliceReject::BeamSpeciesUnknown { .. } => "BeamSpeciesUnknown",
        SliceReject::BeamEnergyOutOfRange { .. } => "BeamEnergyOutOfRange",
        SliceReject::BeamPowerShareOutOfRange { .. } => "BeamPowerShareOutOfRange",
        SliceReject::TimeSinceBeamChangeOutOfRange { .. } => "TimeSinceBeamChangeOutOfRange",
        SliceReject::NonFiniteValue => "NonFiniteValue",
        SliceReject::SlowingRateNotPositive { .. } => "SlowingRateNotPositive",
        SliceReject::NotSettled { .. } => "NotSettled",
        SliceReject::ParticleLedgerBroken { .. } => "ParticleLedgerBroken",
        SliceReject::EnergyLedgerBroken { .. } => "EnergyLedgerBroken",
        SliceReject::PowerLedgerBroken { .. } => "PowerLedgerBroken",
        SliceReject::BeamBeamShareTooLarge { .. } => "BeamBeamShareTooLarge",
        SliceReject::FastIonShareTooLarge { .. } => "FastIonShareTooLarge",
        SliceReject::BeamChangeTooRecent { .. } => "BeamChangeTooRecent",
        SliceReject::IonChargeExceedsElectrons { .. } => "IonChargeExceedsElectrons",
        SliceReject::IonChargeBelowElectrons { .. } => "IonChargeBelowElectrons",
    }
}

fn field_words() -> String {
    "SHA-256 of the shell count as 8 little-endian bytes, then the D-T reaction rate density of each shell from the axis outward, per cubic metre per second, as little-endian f64 bytes.".to_string()
}

/// What one reaction's cross-section fit was held to.
fn reaction_record(reaction: Reaction, tolerance: f64, worst: f64, endf: &str) -> Value {
    json!({
        "gamow_constant_sqrt_kev": reaction.gamow_constant(),
        "cross_section_range_kev": [bosch_hale::CROSS_SECTION_ENERGY_MIN_KEV, reaction.cross_section_energy_max_kev()],
        "reactivity_range_kev": [bosch_hale::REACTIVITY_TEMPERATURE_MIN_KEV, bosch_hale::REACTIVITY_TEMPERATURE_MAX_KEV],
        "energy_kev": reaction.energy_kev(),
        "maxwellian_integral_against_the_published_reactivity_fit": { "temperatures_0p2_to_100_kev": 16, "worst_relative": worst, "tolerance": tolerance, "met": true },
        "against_endf_b_viii_0": endf,
    })
}

fn sections() -> Sections {
    Sections {
        locked: json!({
            "model": "The plasma is the caller's nested shells. Thermal rate: n_D n_T times the Bosch-Hale reactivity at the shell's ion temperature. Beam-target rate: a beam ion is born in a shell at its component's energy and slows there on electrons, deuterium, tritium and the impurity by the test-particle rates of the NRL Plasma Formulary; at each energy it may fuse with the thermal fuel, with the Bosch-Hale cross-section averaged over the thermal motion of the target; it is followed down to 1.5 ion temperatures, and a component born at or below that joins the thermal plasma at once. Beam-beam rate: fast deuterons on fast tritons of the same shell, each with its settled slowing-down spectrum, taken as isotropic in direction. Steady state: the settled spectrum of a constant source; a slice less than five fast-ion energy times after the beam power changed, by the caller's statement, is refused. Fast ions are not a background for other beam ions; where they exceed 0.439 of the electrons of a shell, the largest share compared and answered, the answer is refused. Quasi-neutrality: the thermal ions may exceed the electrons of a shell by 2.5 percent at the door, and the thermal ions and the fast ions the solve finds must carry the electrons' charge within 21.5 percent under and 12 percent over, the extremes compared and answered, or the answer is refused. The tritium share of the fuel is held to the span compared. Thermal hydrogen is not an input. Left out: motion of a beam ion across shells while it slows, its loss by charge exchange or orbit (the caller gives the power deposited), the rotation of the plasma, ion-cyclotron acceleration of beam ions, the direction of the beams, reactions of fast deuterons with each other and of tritons with tritons, fusion alphas. Greenwald's and Troyon's limits belong to a machine's operating point; the question holds no current, field or minor radius, and neither is asked.",
            "stopping_energy_over_ion_temperature": slice::STOPPING_ENERGY_OVER_ION_TEMPERATURE,
            "first_panel_kev": slice::FIRST_PANEL_KEV,
            "thermal_average_panel_kev": slice::THERMAL_AVERAGE_PANEL_KEV,
            "max_refinements": slice::MAX_REFINEMENTS,
            "beam_beam_share_max": slice::BEAM_BEAM_SHARE_MAX,
            "share_sum_allowance": slice::SHARE_SUM_ALLOWANCE,
            "quasi_neutrality_allowance": slice::QUASI_NEUTRALITY_ALLOWANCE,
            "quasi_neutrality_end_allowance_ulp": slice::QUASI_NEUTRALITY_END_ALLOWANCE_ULP,
            "thermal_charge_over_electrons_highest_answered": slice::THERMAL_CHARGE_OVER_ELECTRONS_HIGHEST_ANSWERED,
            "quasi_neutrality_fast_ion_allowance": slice::QUASI_NEUTRALITY_FAST_ION_ALLOWANCE,
            "ion_charge_over_electrons_highest_answered": slice::ION_CHARGE_OVER_ELECTRONS_HIGHEST_ANSWERED,
            "quasi_neutrality_deficit_allowance": slice::QUASI_NEUTRALITY_DEFICIT_ALLOWANCE,
            "quasi_neutrality_deficit_end_allowance_ulp": slice::QUASI_NEUTRALITY_DEFICIT_END_ALLOWANCE_ULP,
            "ion_charge_over_electrons_lowest_answered": slice::ION_CHARGE_OVER_ELECTRONS_LOWEST_ANSWERED,
            "temperature_ratio_range": [slice::TEMPERATURE_RATIO_MIN, slice::TEMPERATURE_RATIO_MAX],
            "tritium_share_range": [slice::TRITIUM_SHARE_MIN, slice::TRITIUM_SHARE_MAX],
            "fast_ion_share_max": slice::FAST_ION_SHARE_MAX,
            "settled_after_energy_times": slice::SETTLED_AFTER_ENERGY_TIMES,
            "beam_change_fraction": slice::BEAM_CHANGE_FRACTION,
            "time_since_beam_change_max_s": slice::TIME_SINCE_BEAM_CHANGE_MAX_S,
            "limits_set_after_the_held_back_comparison": "quasi_neutrality_allowance, fast_ion_share_max, settled_after_energy_times and beam_change_fraction were set on 2026-10-09 after the held-back comparison had failed, on data by then spent (evidence/research/tokamak_dt_beam_target/protocol_amendment_2.md). On engine 0.7.0 the charge balance of the thermal and the fast ions was held from both sides after the solve, at the extremes the slices compared and answered on engine 0.6.0 reach (quasi_neutrality_fast_ion_allowance and quasi_neutrality_deficit_allowance added), fast_ion_share_max was brought down to the largest share compared and answered, and the ranges of temperature, temperature ratio, electron density, tritium share, impurity and beam energy were brought to the span of those slices. None has been tested on data it was not found on.",
            "shells_max": slice::SHELLS_MAX,
            "beam_components_max": slice::BEAM_COMPONENTS_MAX,
            "cross_sections": {
                "fit": "Bosch and Hale (1992), Nucl. Fusion 32, 611: the S-factor fits of Tables IV and VI and the reactivity fits of Table VII. The paper is not open: the cross-section coefficients were read from two open codes that carry its tables and agree, ASCOT5 (src/boschhale.c) and FIDASIM (tables/atomic_tables.f90); the reactivity coefficients from UKAEA PROCESS and cfspopcon. The ranges are as those codes state them, not confirmed against the paper.",
                "deuterium_tritium": reaction_record(Reaction::DeuteriumTritium, bosch_hale::REFERENCE_TOLERANCE[0], 0.0126, "71 points from 5 to 300 keV in the centre of mass: within 3.2 percent; within 1.5 percent to 150 keV. Above 550 keV, where no beam of this solver reaches, the fit is up to 12 percent above."),
                "deuterium_deuterium_neutron": reaction_record(Reaction::DeuteriumDeuteriumNeutron, bosch_hale::REFERENCE_TOLERANCE[1], 0.0193, "Within 1.7 percent from 5 to 150 keV and 2.8 percent to 300 keV."),
                "deuterium_deuterium_proton": reaction_record(Reaction::DeuteriumDeuteriumProton, bosch_hale::REFERENCE_TOLERANCE[2], 0.0250, "Within 2.0 percent from 5 to 150 keV and 4.6 percent to 300 keV."),
            },
            "slowing_down": "genesis_solvers::pb11_reaction: the test-particle energy-loss rates and Coulomb logarithms of the NRL Plasma Formulary and the relativistic factor of Putvinski, Ryutov and Yushmanov (2019) on the electron channel; the same functions the two proton-boron-11 solvers call.",
        }),
        invariants: json!([
            {
                "name": "particle_ledger_residual",
                "rule": "Every beam ion of every component in every shell is counted once: the share that slowed to the stopping energy and the share that fused with thermal ions add to one. An account of the arithmetic: it holds for any cross-section.",
                "tolerance": slice::PARTICLE_LEDGER_TOLERANCE,
                "unit": "1",
                "checked": "for every component in every shell at every refinement",
                "refusal": "ParticleLedgerBroken",
            },
            {
                "name": "energy_ledger_residual",
                "rule": "Every keV of a beam ion's birth energy is counted once: given to electrons, to ions, carried into fusion, or still held at the stopping energy. Each channel's energy is the integral of its own rate over the total, so a channel counted in one place and not the other shows. An account of the arithmetic: it holds for any slowing-down rate, and says nothing of whether the rates are right.",
                "tolerance": slice::ENERGY_LEDGER_TOLERANCE,
                "unit": "fraction of the birth energy",
                "checked": "for every component in every shell at every refinement",
                "refusal": "EnergyLedgerBroken",
            },
            {
                "name": "power_ledger_residual",
                "rule": "The beam power of the whole plasma equals the power given to electrons and to ions, carried into fusion and still held at the stopping energy, summed over shells and components. Beam-beam reactions are outside all three ledgers; their share of the beam ions is returned and bounded.",
                "tolerance": slice::POWER_LEDGER_TOLERANCE,
                "unit": "fraction of the beam power",
                "checked": "once, on the settled answer",
                "refusal": "PowerLedgerBroken",
            },
        ]),
        convergence: json!({
            "settle_tolerance": slice::SETTLE_TOLERANCE,
            "rule": "In each shell the energy panels are halved until every component's fused shares and the shell's beam-beam rate move by less than the settle tolerance. The average over the target's thermal motion is taken in panels of fixed width and is not refined; the outside recomputation holds it.",
            "refusal": "NotSettled",
        }),
        reference: json!({
            "name": slice::REFERENCE_NAME,
            "url": slice::REFERENCE_URL,
            "retrieved": slice::REFERENCE_RETRIEVED,
            "test": "jet_pulse_42976_at_13_35_s",
            "covers": "The D-T fusion power of one time slice of one JET pulse, fifty shells and six beam components. The test prints the solver's power over the database's, 0.983, and holds the solver to its own value. This pulse was not held back.",
            "kind": "The output of the TRANSP analysis of the pulse from its measured profiles, which its analyst compared with the measured neutron rate. It is not a raw neutron count.",
        }),
    }
}

/// The validation record, in the five parts a card's description gives.
fn validation() -> [String; 5] {
    let object = "The deuterium-tritium pulses of two tokamaks as the International Multi-Tokamak Confinement Profile Database, 2008 public release, holds them (https://tokamak-profiledb.ccfe.ac.uk/PR08/; Nucl. Fusion 40 (2000) 1955): JET (Culham), six pulses of 1997, and TFTR (Princeton), 1993 to 1997. For each pulse: radial profiles of temperature and density at each time, the beam heating profile, and the fusion power, all from the TRANSP analysis of the pulse from its measurements.".to_string();
    let data = format!("{DATA} Record: {EVIDENCE}.");
    let pass_line = format!("{PASS_LINE} Fixed {PASS_MARKS_FIXED}.");
    [object, data, pass_line, RESULT.to_string(), LIMITS.to_string()]
}

const DATA: &str = "Six JET D-T pulses (42976, 42982, 42762, 43002, 42794, 42997), on which the structure was chosen, and every TFTR pulse of the database, held back: another machine, circular, with tangential beams near 100 kV and ion temperatures above JET's. One question is one time slice: the database's radial zones as shells, the beam heating profile as the deposited power, the beam species and energies from the pulse's comment and 0D files. Slices are taken every 0.3 s from 0.4 s after the beam power first reaches 0.8 of its maximum. Every number of a question and the fusion power it is compared with come from one TRANSP run of that pulse: the temperatures, densities, fuel mix, impurity, zone volumes and beam heating are TRANSP's analysed profiles, and the reference, PFUSION, is TRANSP's fusion power, which each JET pulse's comment file places within about 10 to 20 percent of the measured neutron rate. No measured neutron trace is in the database. The stated calibration accuracy of the neutron detectors is 7 percent on TFTR (Johnson et al. 1995) and about 10 percent on JET (Batistoni et al. 2018). Since 2026-10-09 each question also carries time_since_beam_change_s, read from the database's injected power trace PNBI: the slice time less the time of the latest sample that differs from the trace's value at the slice by more than a fifth (protocol_amendment_2.md; the value for every saved slice is in analysis/out/08_time_since_beam_change.json).";
const PASS_LINE: &str = "Four marks on the TFTR slices whose database fusion power is at least 0.2 MW. L1: the median of solver over database fusion power inside 0.85 to 1.15. L2: at least 0.90 of the slices inside 0.75 to 1.25. L3: pulse 80539 at the slice nearest the peak of its trace, solver over the published measured 10.7 MW, inside 0.80 to 1.20. L4: every scored slice solved, with both ledgers inside tolerance. Baselines scored beside: the first card's fusion formulas with its fitted constants, thermal-only at the central values, thermal-only with profiles, and a constant times the beam power.";
const RESULT: &str = "The registered comparison failed, and that stands: one of the four marks was missed and three were met. Run: through the gateway's local connection on 2026-10-09, once, by the build of solver commit 0302810 and gateway commit b73b7b7 (binary SHA-256 2c88f6e3...0d7da), which held none of the three limits described under limits. 103 slices of 27 TFTR pulses were asked and all were solved; 34 slices, of 15 of the 16 pulses with tritium beams, reach 0.2 MW and are scored (73268, 74898, 76539, 76770, 77801, 79089, 79099, 79100, 79101, 79122, 80539, 83526, 84243, 88742, 95555). L1, median of solver over database fusion power: 1.022, met. L2, share inside 0.75 to 1.25: 30 of 34, 0.882 against 0.90, missed. L3, pulse 80539 at 3.74 s: 11.45 MW, 1.070 of the published 10.7 MW, met, at the edge of the detectors' 7 percent; that slice is 0.07 s before the peak of the database's trace (11.49 MW at 3.81 s, itself 1.074 of 10.7 MW), and at 3.81, 3.86 and 3.91 s the solver returns no answer (BeamPowerOutOfRange: the heating profile there sums to 48, 57 and 46 MW against 39.5 MW injected), so the instant of the record is outside what the solver answers. L4: 34 of 34 solved, every ledger inside tolerance, every seal verified, met. The four slices outside the band: both slices of pulse 73268, 5.68 and 5.07, and 79100 at 3.92 s, 1.338, and 83526 at 4.43 s, 1.340. Pulse 73268's database value is an alpha power: it is below the power of its own thermal neutrons, and five times it is 1.06 of the measured 6.2 MW; against five times it the two slices are 1.14 and 1.01. It was flagged in amendment 1 before the solver was run, with the value on screen. The other two are slices 0.4 s after the beams came on, 3.8 and 2.3 fast-ion energy times, with fast ions at 0.59 and 0.63 of the electrons in one shell. Counts that are not the registered one: without the two flagged slices 30 of 32, median 1.013; against five times 73268's value 32 of 34. Amendment 1 read the beam energies the comment files state as mean energies, after the TFTR files were opened: with them read as full energies the count is 29 of 34 (29 of 32), with the 0D voltage for both species 28 of 34 (28 of 32); the amendment was worth one or two slices, on pulse 95555, in the solver's favour. Second scoring V-42 (evidence/checks/second_party/V-42/), a second agent with its own readers on the saved binary: all 175 questions rebuilt to rounding, all 278 saved answers verified and asked again with the same bytes, the same four marks to the printed digit, the same verdict. What the agreement shows. Inputs and reference come from one TRANSP run. The solver's thermal part equals TRANSP's thermal neutrons to 2 percent on most slices because both put one reactivity fit on the same profiles. What is left to differ is one slowing-down calculation (local, steady, isotropic, analytic) against another (Monte Carlo, orbit-following, time-dependent) with the same cross-section. So a median of 1.02 shows that the solver's slowing-down arithmetic matches TRANSP's on TRANSP's own plasma; it shows nothing about the neutron counters, the profiles or the deposition. Against measured values, four numbers of which only the first was registered, an indication and no test: TFTR 80539, 10.7 MW: solver 1.070 at 3.74 s (PFUSION 1.074 at its peak, where the solver refuses). TFTR 73268, 6.2 +- 0.4 MW: solver 1.21 at the peak of the database's trace, 7.5 MW (five times the database value: 1.06). JET 42976, 16.1 MW: solver 0.89 at the peak of the trace, 14.4 MW, the same as PFUSION. JET 42982, 21.7 MJ: about 1.08, the solver being 0.939 of PFUSION's 24.9 MJ over the pulse. Calibration uncertainty: TFTR +-7 percent, JET about +-10 percent; two of the four are inside it, and the reference itself is outside it on two and at its edge on a third. The measured values of 73268 and 42982 were read from abstracts and search summaries, not from the papers. What was found of the model on these slices, all after the fact. The transient: within about five energy times of the beams coming on the solver is 0.6 to 1.55 of the reference, low at first because the heating profile it is fed lags the injection, then high. Fast ions: the solver's fast-ion number is 1.08 to 1.44 of the database's on the scored TFTR slices (1.01 to 1.19 on JET's), and its thermal ions plus its own fast ions exceed the electrons by up to 57 percent in a shell (79122 at 3.92 s) and by more than 5 percent on 15 of the 34. Slowing the beam ions on the database's fast tritons as well as on the thermal ions lowers the beam-target rate by 4 to 17 percent on the 19 slices with tritium beams only and brings the fast-ion number to 0.96 to 1.26 of the database's; the median on those slices then falls from 0.96 to 0.88. The agreement in the median is therefore partly a cancellation: the missing drag raises the answer and something else, not identified, lowers it. Beam-beam: with pitch-angle distributions of tangential injection in place of isotropy the rate is 0.52 to 1.19 of the isotropic one (0.78 to 1.00 for balanced or two-to-one injection), which for near-balanced injection is about 5 percent of the total at most on the two-gas slices, where beam-beam is 18 to 25 percent of the rate, and about 10 percent on 83526, where it is 42 to 50 percent. Its size: the fast-ion number is 1.2 times the database's in the median and beam-beam goes as its square, 1.45; a fit of TRANSP's power to the solver's parts weights the beam-beam part by 0.33 to 1.03 depending on the rows taken and how they are weighted (0.67 in the second scoring), so the term lies between right and three times too large and this data does not fix it. Without beam-beam reactions the median is 0.93 and 28 of 34 are inside. Baselines on the same 34 slices, median and share inside the band: the first card's fusion formulas with its fitted constants 0.32 and none; thermal at the central values over the whole volume 2.93 and 3 of 34; thermal with profiles alone 0.16 and 1 of 34; a constant times the beam power 0.70 and 12 of 34 (0.96 and 10 of 34 with the constant taken on the 63 JET slices the solver answered: the median mark alone does not tell the solver from a constant, the spread mark does). On the JET slices, not held back, by the same build: 72 were asked and 8, all of pulse 43002, were refused by the quasi-neutrality limit as it then stood, 1e-9, the database's own deuterium, tritium and impurity there carrying up to 1.37 percent more charge than its electrons; of the 71 above 0.2 MW, 63 were solved: median 0.934, 0.815 to 1.046, all inside. With the energy fractions of the 1996 to 1997 TFTR files in place of the registered ones, reported and not gated: median 1.007, 80539 at 0.988 of 10.7 MW, the same four slices outside. Outside recomputation in 50-digit arithmetic: evidence/checks/crosscheck/tokamak_dt_beam_target/. After the fact, and no test of anything, since each of the three limits was found on these same slices: all 175 saved questions were asked again on 2026-10-09 of the build that holds the limits (solver commit feeca9b, gateway commit f4a175b, binary SHA-256 4b7f2faa...ed6cad), each with time_since_beam_change_s read from the injected power trace. Of the 34 scored TFTR slices 23 were answered and 11 refused by name: 8 by BeamChangeTooRecent (the first slice of 79089, 79099 and 88742, all three of 79101, 80539 at 4.04 s, 84243) and 3 by FastIonShareTooLarge (79100 and 79122 at 3.92 s, 83526 at 4.43 s, each also under five energy times). Nine of the 11 refused had been inside the band. Over the 23 answered: median 1.088 and 21 of 23 inside 0.75 to 1.25, the two outside being pulse 73268's; against five times its database value, median 1.015, 23 of 23, 0.850 to 1.243; 80539 at 3.74 s is answered, 1.070 of 10.7 MW; every ledger and seal holds. Counted as the protocol counts a refusal, outside every mark: 21 of 34. Of the 71 scored JET slices 53 were answered and 18 refused, all by BeamChangeTooRecent; the eight of pulse 43002 that the old allowance refused are answered, 0.878 to 0.909; over the 53, median 0.934, 0.832 to 1.046, all inside. On the 135 slices both builds answer, every number both return is the same bits. The slices answered on that build were not quasi-neutral with their own fast ions: thermal plus fast charge exceeded the electrons by up to 12 percent (83526 at 4.73 s, where beam-beam is 42 percent of the rate), and the largest fast-ion share was 0.44. Record: after_the_fact_rescoring.md, runs_tftr_amended_after_the_fact/, runs_jet_amended_after_the_fact/. Engine 0.7.0, on 2026-10-10, again after the fact and no test: the fast ions the solve finds are counted in the charge balance of every shell, held from both sides at the extremes the slices compared and answered on 0.6.0 reach, and the ranges are brought to the span of those slices, the 143 answered in runs_tftr_amended_after_the_fact_v060/ and runs_jet_amended_after_the_fact_v060/ and the 23 held-back TFTR slices answered. Asked of the 0.7.0 solver source were the 34 scored TFTR slices as a later check of 0.6.0 rebuilt them from the protocol alone (evidence/checks/fresh_eyes_0.6.0/tokamak_dt_beam_target/requests/), and the 176 saved questions (the 175 of those two folders and the JET reference slice). Of the 34, 23 are answered, the same slices as on 0.6.0 with the same bits: median 1.088, 0.850 to 5.68, 21 of 23 inside 0.75 to 1.25 (against five times 73268's value, median 1.015 and 23 of 23); 80539 at 3.74 s is answered, 1.070 of 10.7 MW. The 11 refused, all refused on 0.6.0, are now refused by 4 FastIonShareTooLarge, 4 BeamChangeTooRecent, 1 IonTemperatureOutOfRange (79100 at 3.92 s, 41.93 keV), 1 TemperatureRatioOutOfRange (80539 at 4.04 s, 7.90, after its collapse) and 1 ElectronDensityOutOfRange (84243 at 2.86 s, 2.46e18 at the edge). Of the 103 saved TFTR slices the same 90 are answered as on 0.6.0, and of the 72 JET slices the same 53, all with the same bits: median 0.934, 0.832 to 1.046, all inside; the JET reference slice is answered. What the wide band holds, read from the database's own densities shell by shell on the 34: over the electrons, the solver's fast ions, 1.1 to 3.1 times the database's fast ions in the shell of largest excess (local deposition, no orbit spread, no drag on other fast ions); under them, the thermal hydrogen of TRANSP's analysis, 4 to 22 percent of the electrons in the outer shells, which the protocol's questions leave out because the solver takes no hydrogen. With that hydrogen counted, the lowest balance of each of the 34 lies between 9.0 percent under and 0.9 percent over; the database's own whole balance on them is within 3.2 percent under and 2.1 percent over. A band at the database's own closure, 8 percent under and 2.5 percent over, would refuse 21 of the 23 TFTR slices answered (14 over, 7 under), and was not set: no slice compared may be refused. Record: fix_0.7.0/.";
const LIMITS: &str = "Fitted constants: None. The choices made on the JET pulses are structural: the deposited power is the beam heating profile, beam-beam reactions are counted, and a beam ion is followed to 1.5 ion temperatures. Three limits were set on 2026-10-09 after the held-back comparison had failed, each from data already spent, and on engine 0.7.0 the charge balance was taken over the fast ions and the ranges brought to the span compared; none has been tested on data it was not found on (protocol_amendment_2.md; fix_0.7.0/). QUASI_NEUTRALITY_ALLOWANCE, 0.025, held at the door on the thermal ions alone: the database's own whole charge balance over the 4930 shells of the 175 saved slices runs from 7.53 percent under (JET 42982) to 2.40 percent over (JET 43002), the largest excess rounded up to the half percent. The thermal ions alone exceed the electrons by up to 1.37 percent on JET 43002, which moves the fusion power of those slices by at most 0.11 percent; the allowance used in full moves it by up to 4 percent. QUASI_NEUTRALITY_FAST_ION_ALLOWANCE, 0.12, and QUASI_NEUTRALITY_DEFICIT_ALLOWANCE, 0.215, held since engine 0.7.0 after the solve on the thermal ions and the fast ions the solve finds: the largest excess and the largest shortfall on a slice compared and answered on engine 0.6.0 (TFTR 83526 at 4.73 s, 0.1186; TFTR 74898 at 3.12 s, 0.2109), rounded outward to the half percent. They are no closure error. On the registered run thermal plus fast charge exceeds the electrons by more than 2.5 percent on 22 of the 34 scored TFTR slices, 20 of them inside the band (analysis/out/11_score_amended.json, registered_charge_excess), because the solver's fast ions are 1.1 to 3.1 times the database's; the shortfall is the thermal hydrogen the question cannot carry. FAST_ION_SHARE_MAX, 0.439 since engine 0.7.0, the largest share on a slice compared and answered on engine 0.6.0 (TFTR 80539 at 3.74 s, 0.4383), rounded up; until then 0.5, under which the 31 scored TFTR slices at or below it were inside the band (73268 against five times its value) and two of the three above were the outliers. The ranges of electron and ion temperature, of their ratio, of electron density, of the tritium share of the fuel, of the impurity and of the beam energy are the span of the 166 slices compared and answered on engine 0.6.0 (90 saved TFTR slices, 23 held-back TFTR slices, 53 of JET), rounded outward in the third figure; the shell count, the shell volume, the component count and the whole beam power are capacities. SETTLED_AFTER_ENERGY_TIMES, 5, with BEAM_CHANGE_FRACTION, 0.2: the 27 scored slices at five energy times or more after the first beam power are all inside (73268 against five times its value), and the two outliers are at 3.8 and 2.3; in the second scoring's time scans a change of 10 to 20 percent inside five energy times left 17 of 17 TFTR rows inside and a change of 20 to 50 percent 31 of 39. The solver cannot check time_since_beam_change_s and cannot see a plasma changing under steady beams. What the comparison tests: whether classical slowing-down on TRANSP's profiles with the Bosch-Hale cross-section reproduces the fusion power TRANSP computes from them, on a machine none of the structural choices were made on. What it does not test: the temperatures and densities, which are inputs, so the solver predicts no pulse; the beam deposition, an input from the same analysis; the neutron counters, which stand one step behind the reference; the three limits. The record says nothing of: plasmas outside the declared ranges, beams above 152 keV, plasmas whose thermal hydrogen would slow the beam, since the question takes none, radio-frequency-accelerated ions, any device that is not a tokamak with neutral beams.";

fn example() -> Value {
    json!({
        "shell_volume_m3": [
            0.185232,
            1.13996,
            2.13983,
            2.78539
        ],
        "electron_temperature_kev": [
            10.9616,
            10.2851,
            7.36622,
            4.37061
        ],
        "ion_temperature_kev": [
            19.2543,
            18.2776,
            8.04433,
            4.37061
        ],
        "electron_density_per_m3": [
            4.06983e+19,
            4.47374e+19,
            5.06836e+19,
            4.15438e+19
        ],
        "deuterium_density_per_m3": [
            1.69228e+19,
            2.04297e+19,
            1.82594e+19,
            1.18337e+19
        ],
        "tritium_density_per_m3": [
            1.37276e+19,
            1.62253e+19,
            1.76105e+19,
            1.52491e+19
        ],
        "impurity_density_per_m3": [
            8.09588e+17,
            7.77101e+17,
            1.92707e+18,
            2.19865e+18
        ],
        "impurity_charge": 6.0,
        "impurity_mass_u": 12.0,
        "beam_power_w": [
            77277.5,
            319378.0,
            593792.0,
            150108.0
        ],
        "beam_mass_number": [
            2.0,
            3.0
        ],
        "beam_energy_kev": [
            80.0,
            152.0
        ],
        "beam_power_share": [
            0.5,
            0.5
        ],
        "time_since_beam_change_s": 0.5503
    })
}

/// The solver's own question, from the values of `parameters`.
fn question(given: &Given) -> Result<SliceQuestion, String> {
    Ok(SliceQuestion {
        shell_volume_m3: given.numbers("shell_volume_m3")?.to_vec(),
        electron_temperature_kev: given.numbers("electron_temperature_kev")?.to_vec(),
        ion_temperature_kev: given.numbers("ion_temperature_kev")?.to_vec(),
        electron_density_per_m3: given.numbers("electron_density_per_m3")?.to_vec(),
        deuterium_density_per_m3: given.numbers("deuterium_density_per_m3")?.to_vec(),
        tritium_density_per_m3: given.numbers("tritium_density_per_m3")?.to_vec(),
        impurity_density_per_m3: given.numbers("impurity_density_per_m3")?.to_vec(),
        impurity_charge: given.number("impurity_charge")?,
        impurity_mass_u: given.number("impurity_mass_u")?,
        beam_power_w: given.numbers("beam_power_w")?.to_vec(),
        beam_mass_number: given.numbers("beam_mass_number")?.to_vec(),
        beam_energy_kev: given.numbers("beam_energy_kev")?.to_vec(),
        beam_power_share: given.numbers("beam_power_share")?.to_vec(),
        time_since_beam_change_s: given.number("time_since_beam_change_s")?,
    })
}

/// The limits the question itself shows, asked of the solver's own check
/// before a physics permit is taken, in the solver's order. The sentence is
/// the solver's, with its name for the refusal.
fn check(given: &Given) -> Result<(), String> {
    slice::admit(&question(given)?)
        .map_err(|reject| format!("Physical Violation: {reject} ({}).", reject_name(&reject)))
}

fn solve(given: &Given) -> Result<Solved, String> {
    Ok(solved(slice::solve(&question(given)?)))
}

/// What the solver returned, in the words every solver shares.
pub fn solved(returned: Result<SliceFusion, SliceReject>) -> Solved {
    let answer = match returned {
        Ok(answer) => answer,
        Err(reject) => {
            let name = reject_name(&reject);
            let sealed = REFUSALS.iter().any(|refusal| refusal.name == name && refusal.sealed);
            return Solved::Refused(Refused {
                name,
                sentence: if sealed { reject.to_string() } else { format!("{reject} ({name})") },
                iterations: match reject {
                    SliceReject::NotSettled { refinements } => Some(u64::from(refinements)),
                    _ => None,
                },
            });
        }
    };
    let named = |name: &str, value: f64| (name.to_string(), Published::Number(value));
    Solved::Answer(Answer {
        iterations: u64::from(answer.refinements),
        result: vec![
            named("thermal_dt_rate_per_s", answer.thermal_dt_rate_per_s),
            named("beam_target_dt_rate_per_s", answer.beam_target_dt_rate_per_s),
            named("beam_beam_dt_rate_per_s", answer.beam_beam_dt_rate_per_s),
            named("dt_rate_per_s", answer.dt_rate_per_s),
            named("dt_fusion_power_w", answer.dt_fusion_power_w),
            named("dd_neutron_rate_per_s", answer.dd_neutron_rate_per_s),
            named("dd_fusion_power_w", answer.dd_fusion_power_w),
            named("neutron_rate_per_s", answer.neutron_rate_per_s),
            named("fast_ion_count", answer.fast_ion_count),
            named("fast_ion_energy_j", answer.fast_ion_energy_j),
            named("fast_ion_energy_time_s", answer.fast_ion_energy_time_s),
            named("beam_power_to_electrons_w", answer.beam_power_to_electrons_w),
            named("beam_power_to_ions_w", answer.beam_power_to_ions_w),
            named("beam_power_thermalised_w", answer.beam_power_thermalised_w),
            named("beam_target_fused_fraction", answer.beam_target_fused_fraction),
            named("beam_beam_fused_fraction", answer.beam_beam_fused_fraction),
            named("fast_ion_fraction_max", answer.fast_ion_fraction_max),
            named("charge_excess_max", answer.charge_excess_max),
            named("charge_excess_min", answer.charge_excess_min),
            ("dt_rate_density_per_m3_s".to_string(), Published::Numbers(answer.dt_rate_density_per_m3_s.clone())),
        ],
        invariants: vec![
            named("particle_ledger_residual", answer.particle_ledger_residual),
            named("energy_ledger_residual", answer.energy_ledger_residual),
            named("power_ledger_residual", answer.power_ledger_residual),
        ],
        field: answer.dt_rate_density_per_m3_s,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::solver_fusion_pb11_shared::documented_rejects_of;
    use crate::solvers::describe_value;

    const SOURCE: &str = include_str!("../../genesis_solvers/src/tokamak_dt_beam_target.rs");

    #[test]
    fn every_refusal_carries_the_sentence_from_its_doc_comment() {
        let documented = documented_rejects_of(SOURCE, "pub enum SliceReject {");
        assert_eq!(documented.len(), REFUSALS.len());
        for ((variant, sentence), refusal) in documented.iter().zip(REFUSALS) {
            assert_eq!(variant, refusal.name);
            assert_eq!(sentence, refusal.meaning, "{}", refusal.name);
        }
        let body = describe_value(TOKAMAK_DT_BEAM_TARGET.name).expect("described");
        let refusals = body["refusals"].as_array().expect("refusals");
        assert_eq!(refusals.len(), 29);
        for (place, refusal) in refusals.iter().enumerate() {
            let reported = if place < 18 { "422, no seal" } else { "200, outcome refused, sealed" };
            assert_eq!(refusal["reported_as"], reported, "{}", refusal["name"]);
        }
    }

    #[test]
    fn the_reference_is_the_one_stored_in_the_solver_test() {
        let body = describe_value(TOKAMAK_DT_BEAM_TARGET.name).expect("described");
        let url = body["reference"]["url"].as_str().expect("url");
        let retrieved = body["reference"]["retrieved"].as_str().expect("retrieved");
        assert!(SOURCE.contains(&format!("<{url}>")), "{url}");
        assert!(SOURCE.contains(&format!("read\n    /// {retrieved}")) || SOURCE.contains(&format!("read {retrieved}")), "{retrieved}");
        let test = body["reference"]["test"].as_str().expect("test");
        assert!(SOURCE.contains(&format!("fn {test}()")), "{test}");
    }

    #[test]
    fn the_summary_carries_the_standing() {
        assert!(SUMMARY.contains("No constant is fitted"));
        assert!(SUMMARY.ends_with('.'));
        let body = describe_value(TOKAMAK_DT_BEAM_TARGET.name).expect("described");
        assert_eq!(body["inputs"].as_array().unwrap().len(), 14);
        assert_eq!(body["returns"]["result"].as_array().unwrap().len(), 20);
        assert_eq!(body["returns"]["invariants"].as_array().unwrap().len(), 3);
        assert_eq!(body["validation"].as_object().unwrap().len(), 5);
    }
}
