//! The entry for `icf_hotspot_burn`: the burn of a stagnated deuterium-tritium
//! hot spot of inertial confinement fusion, from peak compression through
//! ignition to disassembly. The numbers come from `genesis_solvers`
//! constants, the sentence of each refusal from the solver's source, and the
//! record of the comparison with the published NIF shots from the research
//! record.

use genesis_solvers::icf_hotspot_burn::{self as burn, HotSpotBurn, HotSpotQuestion, HotSpotReject};
use serde_json::{Value, json};

use crate::solvers::{
    Answer, FurtherExample, Given, Input, Kind, Output, Published, Refusal, Refused, Sections, Shape, Solved, Solver,
};

/// The fingerprint of the record of the comparison.
pub const PASS_MARKS_FIXED: &str = "Record SHA-256 a3ef39766b8a527947f4b8c37d3cb2563849a9da3abdabc40fe2dd4b805d8553.";

const fn number(name: &'static str, unit: &'static str, note: &'static str) -> Output {
    Output { name, unit, shape: Shape::Number, each: &[], note, asked_by: None }
}

const fn numbers(name: &'static str, unit: &'static str, note: &'static str) -> Output {
    Output { name, unit, shape: Shape::Numbers, each: &[], note, asked_by: None }
}

const fn above_zero(name: &'static str, maximum: f64, unit: &'static str, refusal: &'static str, bound: &'static str) -> Input {
    Input {
        name,
        kind: Kind::NumberAbove { above: 0.0, maximum, unit },
        refusal: Some(refusal),
        bound,
        only_when: None,
        asked_by_check: true,
    }
}

pub const ICF_HOTSPOT_BURN: Solver = Solver {
    name: "icf_hotspot_burn",
    summary: SUMMARY,
    inputs: &[
        Input {
            name: "hot_spot_temperature_kev",
            kind: Kind::Number { minimum: burn::TEMPERATURE_MIN_KEV, maximum: burn::TEMPERATURE_MAX_KEV, unit: "keV" },
            refusal: Some("TemperatureOutsideReactivityFit"),
            bound: "Temperature of the hot spot's ions and electrons (one temperature) at peak compression under ideal hydrodynamics. 0.2 to 100 keV, where the D-T reactivity fit of Bosch and Hale, 1992, is defined; the same limit holds at every step of the burn (TemperatureLeftReactivityFit). A published burn-averaged ion temperature is a different quantity: the solver returns its own neutron-averaged temperature beside the yield.",
            only_when: None,
            asked_by_check: true,
        },
        above_zero("hot_spot_pressure_gbar", burn::PRESSURE_MAX_GBAR, "Gbar", "PressureNotPositive", "Hot-spot pressure at peak compression, gigabar (1e14 Pa). Above zero. A hot spot whose electrons are degenerate at peak compression, their Fermi energy at n = p/2T at or above T, is refused (StagnationDegenerate); above 4.6018767e7 Gbar that is so at every temperature the reactivity fit holds, which is the upper end."),
        above_zero("hot_spot_radius_um", burn::RADIUS_MAX_UM, "um", "RadiusNotPositive", "Hot-spot radius, micrometres. Above zero. With the pressure and temperature it fixes the hot-spot mass: p / 2T fuel ions per volume, each with its electron; the fuel mass must exceed it (FuelMassNotAboveHotSpotMass). For a NIF shot, the radius of the sphere with the neutron image's volume. The upper end, 1e9 um, is where the model's volumes and energies would leave 64-bit arithmetic (InputTooLargeForArithmetic)."),
        above_zero("shell_areal_density_g_per_cm2", burn::SHELL_AREAL_DENSITY_MAX_G_PER_CM2, "g/cm^2", "ShellArealDensityNotPositive", "Areal density of the cold fuel around the hot spot, g/cm^2. Above zero. The fuel left for the shell must be at least 4 pi R^2 times it, the least a uniform shell of that areal density outside the hot spot holds (ShellMassBelowArealDensity); the shell's density follows from the two, and must be above the hot spot's (ShellNotDenserThanHotSpot): the model is of a hot spot inside colder, denser fuel. The upper end, 1e12 g/cm^2, is arithmetic's (InputTooLargeForArithmetic)."),
        above_zero("fuel_mass_mg", burn::FUEL_MASS_MAX_MG, "mg", "FuelMassNotAboveHotSpotMass", "The deuterium-tritium fuel at peak compression, hot spot and shell, milligrams. It must exceed the hot spot's own mass. The NIF Hybrid-E capsules hold about 0.2 to 0.22 mg of DT, of which part is in the dense shell during the burn (the down-scatter ratio sees 35 to 60 percent of it in a shell of 255 to 510 g/cm^3). The upper end, 1e12 mg, is arithmetic's (InputTooLargeForArithmetic)."),
        Input {
            name: "ablator_mass_mg",
            kind: Kind::Number { minimum: 0.0, maximum: burn::FUEL_MASS_MAX_MG, unit: "mg" },
            refusal: Some("AblatorMassNegative"),
            bound: "The ablator remaining outside the fuel at peak compression, milligrams; zero for none. On NIF about 5 percent of the initial ablator, close to the DT mass (PRL 132, 065103): about 0.2 mg. It is carbon, fully ionized, at the fuel shell's pressure and temperature, and tamps the shell. The upper end is arithmetic's (InputTooLargeForArithmetic).",
            only_when: None,
            asked_by_check: true,
        },
    ],
    kept_inside: "The shell zones, the Courant number and its halvings, the start and end of the burn, the tolerances, the deuterium-tritium mix (50:50) and every physical constant stay inside the solver.",
    result: &[
        number("yield_mj", "MJ", "Fusion yield, the D-T reactions times 17.589 MeV."),
        number("neutron_yield", "1", "D-T reactions, which is the 14 MeV neutron yield."),
        number("yield_without_alpha_heating_mj", "MJ", "Yield of the same implosion with the alpha energy switched off and everything else on."),
        number("yield_amplification", "1", "Yield over the yield without alpha heating."),
        number("burn_up_fraction", "1", "Twice the reactions over the deuterium and tritium ions of the whole fuel."),
        number("burn_width_ps", "ps", "Full width at half maximum of the fusion rate."),
        number("bang_time_ps", "ps", "Time of the peak fusion rate, from the ideal peak compression."),
        number("peak_fusion_power_w", "W", "Fusion power at bang time."),
        number("burn_averaged_temperature_kev", "keV", "Neutron-averaged hot-spot temperature."),
        number("burn_averaged_pressure_gbar", "Gbar", "Neutron-averaged hot-spot pressure."),
        number("burn_averaged_radius_um", "um", "Neutron-averaged hot-spot radius."),
        number("burn_averaged_hot_spot_areal_density_g_per_cm2", "g/cm^2", "Neutron-averaged hot-spot density times radius."),
        number("burn_averaged_shell_areal_density_g_per_cm2", "g/cm^2", "Neutron-averaged areal density of the shell."),
        number("hot_spot_mass_at_bang_mg", "mg", "Hot-spot mass at bang time, its stagnation mass and the fuel ablated into it, less what burned."),
        number("hot_spot_energy_at_bang_kj", "kJ", "Hot-spot thermal energy, (3/2) p V, at bang time."),
        number("peak_temperature_kev", "keV", "Highest hot-spot temperature during the burn."),
        number("peak_pressure_gbar", "Gbar", "Highest hot-spot pressure during the burn."),
        number("fuel_ablated_fraction", "1", "Share of the fuel mass ablated from the shell into the hot spot by the end of the burn: the burn's propagation into the dense fuel."),
        number("burned_through", "1", "1 when the whole shell was ablated into the hot spot before the burn ended, else 0."),
        number("self_heating_ratio_at_bang", "1", "The static power balance at bang time: alpha heating deposited in the hot spot over bremsstrahlung leaving it plus conduction, 5 T times the conduction ablation rate per fuel ion. Above one the hot spot heats itself faster than it radiates and conducts."),
        number("self_heating_ratio_peak", "1", "The largest value of that ratio during the burn."),
        number("alpha_energy_over_compression_work_at_bang", "1", "Alpha energy deposited in the hot spot, directly and through the shell, up to bang time, over the compression work done on the hot spot up to bang time. Above one: the burning-plasma regime of the hot spot."),
        number("alpha_energy_over_compression_work", "1", "The same over the whole burn."),
        number("lawson_parameter_no_alpha", "1", "The generalised Lawson parameter of Betti et al., PRL 114, 255003, 2015, equation 11, from the run without alpha heating: (rho R)^0.61 (0.24 Y16 / M)^0.34, rho R its neutron-averaged total areal density in g/cm^2, Y16 its yield in 1e16 neutrons, M the fuel mass in mg. One is the ignition condition."),
        number("lawson_parameter_pressure_time", "1", "p tau / (24 T^2 / (<sigma v> E_alpha)) at the neutron-averaged pressure and temperature, tau the burn width: the Lawson product of Betti et al., Phys. Plasmas 17, 058102, 2010, over its ignition value."),
        number("lawson_criterion_lindl", "1", "The generalised Lawson criterion of Lindl et al. as PRL 129, 075001 evaluates it (its equation 2): (p/420 Gbar)(R/50 um) at the neutron-averaged pressure and radius. Above one: ignition by that criterion."),
        number("alpha_energy_over_twice_hot_spot_energy", "1", "E_alpha / (2 E_hs), the criterion of Christopherson et al. as PRL 129, 075001 evaluates it (its equation 4): the yield over five over twice the hot-spot energy at bang time. Above 1.4: ignition by that criterion."),
        number("q_alpha_hot_spot", "1", "Q_alpha of the hot spot (Betti et al., 2015, equation 14): half the alpha energy released over the compression work done on the hot spot up to bang time. Above one: the hot spot's burning-plasma regime."),
        number("alpha_energy_mj", "MJ", "Alpha energy released, 3560.80 keV a reaction."),
        number("alpha_energy_in_hot_spot_mj", "MJ", "Alpha energy deposited directly in the hot spot (the Krokhin-Rozanov share)."),
        number("alpha_energy_through_shell_mj", "MJ", "Alpha energy deposited in the shell and returned with the fuel it ablated."),
        number("alpha_energy_escaped_mj", "MJ", "Alpha energy that left the fuel after burn-through, with no ablator outside it."),
        number("alpha_energy_in_ablator_mj", "MJ", "Alpha energy deposited in the ablator after the fuel was burned through."),
        number("neutron_energy_mj", "MJ", "Neutron energy, 14028.20 keV a reaction, all of it leaving."),
        number("bremsstrahlung_mj", "MJ", "Bremsstrahlung that left the fuel."),
        number("bremsstrahlung_absorbed_in_shell_mj", "MJ", "Bremsstrahlung absorbed in the shell, by free-free absorption."),
        number("conduction_returned_mj", "MJ", "Heat conducted from the hot spot into the shell and returned with the fuel it ablated. Not a loss of the hot spot's energy."),
        number("compression_work_mj", "MJ", "Work done on the hot spot by the shell while it compressed it."),
        number("expansion_work_mj", "MJ", "Work done by the hot spot on the shell while it expanded."),
        number("energy_at_start_mj", "MJ", "Thermal, internal and kinetic energy of the hot spot and the shell at the start of the burn."),
        number("energy_at_end_mj", "MJ", "The same at the end of the burn."),
        number("start_time_ps", "ps", "Start of the burn before the ideal peak compression."),
        number("shell_density_g_per_cm3", "g/cm^3", "Density of the uniform shell at the ideal peak compression, from its mass and areal density."),
        number("ablator_density_g_per_cm3", "g/cm^3", "Density of the ablator at the ideal peak compression: the shell's pressure and temperature, carbon fully ionized, 1.37 times the shell's density."),
        number("stagnation_hot_spot_mass_mg", "mg", "Hot-spot mass at the ideal peak compression, from its pressure, temperature and radius."),
        numbers("history_time_ps", "ps", "The burn history: at most 200 steps of the answering run (every n-th, the bang step and the last), time."),
        numbers("history_fusion_power_w", "W", "Fusion power at each history step."),
        numbers("history_temperature_kev", "keV", "Hot-spot temperature at each history step."),
        numbers("history_pressure_gbar", "Gbar", "Hot-spot pressure at each history step."),
        numbers("history_radius_um", "um", "Hot-spot radius at each history step."),
    ],
    invariants: &[
        number("energy_residual", "1", ""),
        number("particle_residual", "1", ""),
    ],
    iterations: "Steps of the answering run, the one at the finer step of the last pair that agreed.",
    field: field_words,
    answer: "a burn",
    refusals: REFUSALS,
    door_refusals: &[],
    sections,
    validation: Some(validation),
    example,
    further_examples: &[
        FurtherExample { key: "example_solve_igniting", parameters: igniting_example },
        FurtherExample { key: "example_solve_n221204_stagnated_255", parameters: n221204_stagnated_255 },
        FurtherExample { key: "example_solve_n221204_stagnated_510", parameters: n221204_stagnated_510 },
    ],
    parts: &[],
    check,
    solve,
};

const SUMMARY: &str = "Burn of a stagnated deuterium-tritium hot spot of inertial confinement fusion, the company's own derivation: a 0-D hot spot (one temperature, Bosch-Hale burn, Krokhin-Rozanov alpha deposition, Betti's conduction ablation, bremsstrahlung with free-free absorption) inside a 64-zone Lagrangian shell of cold fuel, run from the mirror image of the ideal implosion through peak compression and ignition to disassembly, with an energy, particle and mass ledger at every step: yield, burn-up, burn width, the neutron-averaged hot spot, the energy accounts and the field's ignition measures (Betti's generalised Lawson parameter, Lindl's criterion, Q_alpha, the static self-heating ratio). Nothing is fitted. Compared once with seven published NIF shots, N201101 to N221204, under pass marks fixed beforehand, it met no shot and three marks of twenty-four: on the published numbers it ignited the four sub-ignited burning-plasma shots (yield 4.3 to 6.1 MJ against 0.10 to 0.17 MJ measured) and answered N221204 at 2.3 MJ against 3.1 but with a burn width of 113 ps against 75; the published volume, down-scatter ratio and fuel mass, put into one uniform shell, leave a shell less dense than the hot spot, which it now refuses by name (ShellNotDenserThanHotSpot, its worked refusal). Re-evaluated after the fact and not gated, with an independent second party's consistent reading (the stagnated mass: the hot spot and a shell of the same areal density at 255 to 510 g/cm^3), N221204 yields 2.19 to 2.97 MJ against 3.1 and burns 2.9 to 4.0 percent of its fuel against 4.33, in 28 ps against 75.";

/// Each `HotSpotReject` variant with the sentence from its doc comment. A
/// refusal the request itself shows is not sealed.
const REFUSALS: &[Refusal] = &[
    Refusal { name: "InputNotFinite", meaning: "An input was NaN or infinite.", sealed: false },
    Refusal {
        name: "TemperatureOutsideReactivityFit",
        meaning: "The hot-spot temperature is outside 0.2 to 100 keV (`TEMPERATURE_MIN_KEV` to `TEMPERATURE_MAX_KEV`), where the reactivity fit of Bosch and Hale is defined.",
        sealed: false,
    },
    Refusal { name: "PressureNotPositive", meaning: "The hot-spot pressure is not above zero.", sealed: false },
    Refusal { name: "AblatorMassNegative", meaning: "The ablator mass is below zero.", sealed: false },
    Refusal {
        name: "InputTooLargeForArithmetic",
        meaning: "The radius, the fuel mass or the shell's areal density is above `RADIUS_MAX_UM`, `FUEL_MASS_MAX_MG` or `SHELL_AREAL_DENSITY_MAX_G_PER_CM2`, where the model's volumes and energies would leave the range of 64-bit arithmetic.",
        sealed: false,
    },
    Refusal {
        name: "StagnationDegenerate",
        meaning: "The hot spot at peak compression is degenerate, its electrons' Fermi energy at or above its temperature; above `PRESSURE_MAX_GBAR` it is so at every temperature the reactivity fit holds.",
        sealed: false,
    },
    Refusal { name: "RadiusNotPositive", meaning: "The hot-spot radius is not above zero.", sealed: false },
    Refusal {
        name: "ShellArealDensityNotPositive",
        meaning: "The shell's areal density is not above zero: with no shell nothing confines the hot spot.",
        sealed: false,
    },
    Refusal {
        name: "FuelMassNotAboveHotSpotMass",
        meaning: "The fuel mass is not more than the hot spot holds at this pressure, temperature and radius: no fuel is left for the shell.",
        sealed: false,
    },
    Refusal {
        name: "ShellMassBelowArealDensity",
        meaning: "The fuel left for the shell is less than any uniform shell outside the hot spot with this areal density holds, `4π R² (ρR)_s`.",
        sealed: false,
    },
    Refusal {
        name: "ShellNotDenserThanHotSpot",
        meaning: "The shell is not denser than the hot spot: at one pressure it would be as hot or hotter, and the model is of a hot spot inside colder, denser fuel.",
        sealed: false,
    },
    Refusal {
        name: "HotSpotDegenerate",
        meaning: "The hot spot's electrons are degenerate, their Fermi energy at or above the temperature: the ideal-gas pressure and the Maxwellian rates of the model do not hold.",
        sealed: true,
    },
    Refusal {
        name: "TemperatureLeftReactivityFit",
        meaning: "During the burn the hot-spot temperature left 0.2 to 100 keV (`TEMPERATURE_MIN_KEV` to `TEMPERATURE_MAX_KEV`), where the reactivity fit is defined.",
        sealed: true,
    },
    Refusal { name: "ShellMeshTangled", meaning: "A shell zone's volume was not above zero or its nodes crossed.", sealed: true },
    Refusal {
        name: "InternalEnergyNotPositive",
        meaning: "The internal energy of the hot spot or of a shell zone was not above zero.",
        sealed: true,
    },
    Refusal { name: "NonFiniteValue", meaning: "An intermediate value was NaN or infinite.", sealed: true },
    Refusal { name: "EnergyLedgerBroken", meaning: "At a step the energy ledger left `ENERGY_TOLERANCE`.", sealed: true },
    Refusal {
        name: "ParticleLedgerBroken",
        meaning: "At a step the particle or mass ledger left `PARTICLE_TOLERANCE`, or the burn-up exceeded the fuel.",
        sealed: true,
    },
    Refusal { name: "StepLimitReached", meaning: "A run reached `MAX_STEPS` with the burn not over.", sealed: true },
    Refusal {
        name: "NotSettled",
        meaning: "The yield or the burn width at the step and at half the step differ by more than `SETTLE_TOLERANCE`.",
        sealed: true,
    },
];

/// The `HotSpotReject` variant name, without its fields.
pub fn reject_name(reject: &HotSpotReject) -> &'static str {
    match reject {
        HotSpotReject::InputNotFinite => "InputNotFinite",
        HotSpotReject::TemperatureOutsideReactivityFit { .. } => "TemperatureOutsideReactivityFit",
        HotSpotReject::PressureNotPositive { .. } => "PressureNotPositive",
        HotSpotReject::AblatorMassNegative { .. } => "AblatorMassNegative",
        HotSpotReject::InputTooLargeForArithmetic { .. } => "InputTooLargeForArithmetic",
        HotSpotReject::StagnationDegenerate { .. } => "StagnationDegenerate",
        HotSpotReject::RadiusNotPositive { .. } => "RadiusNotPositive",
        HotSpotReject::ShellArealDensityNotPositive { .. } => "ShellArealDensityNotPositive",
        HotSpotReject::FuelMassNotAboveHotSpotMass { .. } => "FuelMassNotAboveHotSpotMass",
        HotSpotReject::ShellMassBelowArealDensity { .. } => "ShellMassBelowArealDensity",
        HotSpotReject::ShellNotDenserThanHotSpot { .. } => "ShellNotDenserThanHotSpot",
        HotSpotReject::HotSpotDegenerate { .. } => "HotSpotDegenerate",
        HotSpotReject::TemperatureLeftReactivityFit { .. } => "TemperatureLeftReactivityFit",
        HotSpotReject::ShellMeshTangled { .. } => "ShellMeshTangled",
        HotSpotReject::InternalEnergyNotPositive { .. } => "InternalEnergyNotPositive",
        HotSpotReject::NonFiniteValue { .. } => "NonFiniteValue",
        HotSpotReject::EnergyLedgerBroken { .. } => "EnergyLedgerBroken",
        HotSpotReject::ParticleLedgerBroken { .. } => "ParticleLedgerBroken",
        HotSpotReject::StepLimitReached { .. } => "StepLimitReached",
        HotSpotReject::NotSettled { .. } => "NotSettled",
    }
}

fn field_words() -> String {
    "SHA-256 of the count of the numbers that follow as 8 little-endian bytes, then, for each row of the burn history in order, its time in ps, fusion power in W, hot-spot temperature in keV, pressure in Gbar and radius in um, all as little-endian f64 bytes.".to_string()
}

fn sections() -> Sections {
    Sections {
        locked: json!({
            "model": "The question is the state of peak compression under ideal hydrodynamics (no burn, no alpha heating, no conduction, no radiation), everything at rest and the shell at the hot-spot pressure. Hot spot: one uniform zone of radius a, D and T in equal numbers, alpha ash, one temperature for ions and electrons; E_h = (3/2)(N_ions + N_e) T, p = (2/3) E_h / V. Shell: the rest of the fuel as a uniform shell of the given areal density, cut into 64 zones of equal mass, followed by 1-D Lagrangian hydrodynamics with p = (2/3) rho epsilon (an ideal gas and a degenerate electron gas alike) and a quadratic artificial viscosity that acts where a zone is compressed and nowhere else; the outer surface is free, so a rarefaction runs in from it, unless an ablator is given: then 32 zones of carbon, fully ionized, sit outside the fuel at the shell's pressure and temperature (1.37 times its density) and tamp it, with carbon's free-free absorption. Ablation is of fuel alone; once the fuel shell is ablated, the heat conducted stays in the hot spot and the alpha energy not deposited there heats the ablator, with no carbon mixed in. The hot spot's own inertia is that of homologous flow, (3/10) M_h u^2. Fusion: (N_f/2V)^2 <sigma v> V reactions a second, Bosch-Hale; each releases 17.589 MeV, an alpha of 3560.80 keV and a neutron of 14028.20 keV (two-body kinematics), the neutron leaving. Alpha deposition: the Krokhin-Rozanov fraction f(tau) = 3tau/2 - 4tau^2/5 (tau <= 1/2), 1 - 1/(4tau) + 1/(160 tau^3) (tau >= 1/2), tau = a / l, l three times the mean depth at which the alpha's energy is deposited by the NRL Formulary's test-particle rates on electrons, D, T and ash (which makes the fraction right to first order in 1/tau for the real law of energy loss); the range by the same rates is within 15 percent of the Maynard-Deutsch fit of Zylstra and Hurricane, 2019, over 30 to 300 g/cm^3 and 5 to 40 keV. The rest of the alpha energy enters the shell, which stops it, and returns with the fuel it ablates. Conduction: the heat conducted out does not leave the hot spot; it ablates the shell's inner face into it at m_dot = (16 pi/25)/0.85^(5/2) m kappa_0 a T^(5/2)/ln Lambda, the edge of the isobaric profile of Betti et al. (LLE Review 85, 2000, equation 41), with Braginskii's conductivity; it reproduces Betti's ablation speed, his equation 32, to 1 percent. Radiation: Svensson's bremsstrahlung with the spectrum e^(-h nu/T), leaving the hot spot with the escape probability of a uniform sphere at its free-free optical radius and crossing the shell zone by zone, each absorbing at its own temperature (Rybicki and Lightman 5.18a, Gaunt factor one). The Coulomb logarithms are the Formulary's in the form (1/2) ln(1 + Lambda^2). The burn before peak compression: the ideal motion is run forward from the question until its fusion rate is 1e-4 of its value there, every velocity is reversed, and the whole model is run forward from that state through peak compression until the fusion rate has fallen to 1e-3 of its peak. Time zero is the ideal peak compression. When the last shell zone has been ablated the hot spot is the whole fuel and expands freely, and the alpha energy not deposited in it leaves. Left out: two temperatures, profiles in the hot spot, mix of ablator into the hot spot, asymmetry and residual kinetic energy at stagnation, burn in the dense shell itself, neutron heating, magnetic fields.",
            "shell_zones": burn::SHELL_ZONES,
            "courant": burn::COURANT,
            "max_refinements": burn::MAX_REFINEMENTS,
            "viscosity_quadratic": burn::VISCOSITY_QUADRATIC,
            "zone_join_fraction": burn::ZONE_JOIN_FRACTION,
            "start_rate_fraction": burn::START_RATE_FRACTION,
            "end_rate_fraction": burn::END_RATE_FRACTION,
            "alpha_birth_energy_kev": burn::alpha_birth_energy_kev(),
            "neutron_birth_energy_kev": burn::neutron_birth_energy_kev(),
            "alpha_panels": burn::ALPHA_PANELS,
            "photon_energy_max_over_t": burn::PHOTON_ENERGY_MAX_OVER_T,
            "photon_panels": burn::PHOTON_PANELS,
            "max_steps": burn::MAX_STEPS,
            "history_rows_max": burn::HISTORY_ROWS_MAX,
            "fitted": "Nothing. Every coefficient is the published one: Bosch and Hale's reactivity, Krokhin and Rozanov's deposition fraction, the NRL Formulary's slowing-down rates and Coulomb logarithms, Braginskii's conductivity, Betti's ablation profile, Svensson's bremsstrahlung, Rybicki and Lightman's free-free absorption. The numerics were set on generic hot spots, with no NIF shot run.",
        }),
        invariants: json!([
            {
                "name": "energy_residual",
                "rule": "The hot spot's heat, the shell's internal and kinetic energy and the hot spot's kinetic energy, plus the bremsstrahlung and alpha energy that left the fuel and the kinetic energy carried off with the reaction products' mass, less all alpha energy released, equal their value at the start of the burn. The equations conserve it exactly; the residual is the time integrator's truncation. The largest over every step of every run, as a share of the larger of the starting energy and the alpha energy released.",
                "tolerance": burn::ENERGY_TOLERANCE,
                "unit": "1",
                "compared": "at every step of every run",
                "refusal": "EnergyLedgerBroken",
            },
            {
                "name": "particle_residual",
                "rule": "Deuterium and tritium ions in the hot spot and the shell, plus twice the reactions, equal the fuel at the start; the alphas equal the reactions; the burn-up is at most one and the hot spot's fuel stays above zero. The largest over every step, as a share of the fuel ions.",
                "tolerance": burn::PARTICLE_TOLERANCE,
                "unit": "1",
                "compared": "at every step of every run",
                "refusal": "ParticleLedgerBroken",
            },
        ]),
        convergence: json!({
            "settle_tolerance": burn::SETTLE_TOLERANCE,
            "rule": "Classical fourth-order Runge-Kutta in time, each step the Courant share of the shortest time of the state (a sound crossing of the hot spot or of a shell zone, the hot spot's boundary moving or turning, the burn-out, the heating, the ablation of the innermost zone). The whole burn is run at the Courant number and at half of it; where yield or burn width differ by more than the settle tolerance, both are halved again, at most max_refinements times. The finer run is the answer. A third run, with the alpha energy switched off, at the finer step, gives the yield without alpha heating.",
            "refusal": "NotSettled",
            "run_time": "About 0.05 to 2 s in a release build; most where the hot spot sits on the edge of ignition and the step is halved.",
        }),
        reference: json!({
            "name": burn::REFERENCE_NAME,
            "url": burn::REFERENCE_URL,
            "retrieved": burn::REFERENCE_RETRIEVED,
            "test": "nif_n221204_as_published",
            "covers": "N221204 as PRL 132 065102 Table I prints it. Compared once as the record sets it: no shot within all its targets, 3 of 24 targets reached. Made into the solver's question by the record's construction, all the fuel outside the hot spot in one uniform shell holding the down-scatter ratio's column alone: the hot spot it makes holds 0.085 mg against the 0.084 +- 0.010 mg inferred in PRE 109, 025203, and has an areal density of 0.446 g/cm^2 against 0.44 published; the shell it leaves is 8.3 g/cm^3 against a hot spot of 66 g/cm^3, and the question is refused by name, ShellNotDenserThanHotSpot: the worked refusal, example_refused. With no target, the consistent reading an independent reviewer proposed, the same temperature, pressure, radius and shell areal density, with the fuel mass the stagnated mass, the hot spot's and a uniform shell of that areal density at 255 or 510 g/cm^3 (0.1317 and 0.1307 mg, 60 and 59 percent of the 0.220 mg of fuel): yield 2.19 and 2.97 MJ against 3.1, burn-up over the whole fuel 2.9 and 4.0 percent against 4.33, but a burn width of 27.7 and 28.0 ps against 75, burning through (example_solve_n221204_stagnated_255 and example_solve_n221204_stagnated_510, held bit for bit by the test nif_n221204_stagnated_mass). With a shell three times the hot spot's density holding all the fuel, also with no target, the clean burn goes through the whole shell, with a yield above the measured 3.1 MJ (5.9 MJ) and a burn width far below the measured 75 ps (8.6 ps).",
            "kind": "Published inferences and measurements of one NIF shot: the yield, the neutron-image volume, the DD and DT ion temperatures, the down-scatter ratio and the burn width are measured; the pressure, the hot-spot energy and areal density are LLNL's inferences from them.",
        }),
    }
}

/// The validation record, in the five parts a card's description gives.
fn validation() -> [String; 5] {
    let object = "The stagnated deuterium-tritium hot spots of the indirect-drive implosions of the National Ignition Facility, Lawrence Livermore National Laboratory: Hybrid-E high-density-carbon capsules about 2.1 mm across holding about 0.2 mg of DT ice, imploded by the x-rays of a laser-heated hohlraum at about 380 to 400 km/s to a hot spot of 35 to 70 um radius, 5 to 13 keV and 300 to 600 Gbar for about 100 ps, from the first burning plasmas (N201101, N201122, N210207, N210220, yield amplification 3.5 to 5.7) to the Lawson criterion for ignition (N210808, 1.3 MJ from 1.89 MJ of laser) and the first target gain above one (N221204, 3.1 MJ from 2.05 MJ). The solver computes the burn from a stagnation state; it does not model the laser, the hohlraum or the ablator.".to_string();
    let data = DATA.to_string();
    let pass_line = format!("{PASS_LINE} {PASS_MARKS_FIXED}");
    [object, data, pass_line, RESULT.to_string(), LIMITS.to_string()]
}

const DATA: &str = "Published tables, each used whole for its shots so that one shot's numbers come from one analysis. PRL 132, 065102, Table I, for N210808, N220919 and N221204: T_DD 8.94 +- 0.4, 8.7 +- 0.2, 12 +- 1 keV; hot-spot pressure 561 +61/-53, 518 +76/-62, 608 +56/-59 Gbar; neutron-image volume enclosing 80 percent of the yield 6.4 +- 0.75, 6.7 +- 0.9, 12.9 +- 1.5 x 1e5 um^3; DSR 2.72 +- 0.24, 3.06 +- 0.14, 2.83 +- 0.17 percent; yield 1.3 +- 0.07, 1.2 +- 0.06, 3.1 +- 0.16 MJ; burn width 89 +- 15, 92 +- 20, 75 +- 15 ps; burn-up 1.77, 1.60, 4.33 percent. Nature 601, 542, Extended Data Table 1, for N201101, N201122, N210207, N210220: T_DD 4.61, 4.65, 5.23, 5.13 keV; pressure 319, 297, 339, 371 Gbar; volumes of 2.5, 2.5, 3.3, 2.2 x 1e5 um^3; burn width 118, 137, 107, 135 ps; yield 98.4, 106.2, 170.9, 160.6 kJ; yield amplification 3.5, 4.2, 4.7, 5.7; their DSR from arXiv:2111.04640, Table 1, 3.44, 3.33, 3.16, 3.31 percent. The papers revise one shot's numbers between publications, and the comparison names the one it used. N210808: PRL 129, 075001, Table I gives pressure 569 +- 61 Gbar, DSR 2.87 +- 0.24 percent, burn width 89 +- 5 ps and yield 1.37 MJ; PRL 132 gives 561, 2.72, 89 +- 15 and 1.3 +- 0.07. PRL 132 was used: it is the later analysis and the one table that holds both N210808 and N221204. N210207: pressure 339 (Nature 601), 351 (PRL 129) and 304 Gbar (PRL 132); burn width 107, 103 and 137 ps. Nature 601 was used for the four burning-plasma shots: it is the table that states their yield amplification. N221204's yield: 3.1 +- 0.16 MJ (PRL 132) and 3.1 +- 0.3 (PRE 109, 025203); PRL 132 was used. Fuel mass: 0.200 mg for N210808 (PRE 106, 025202, '~200 ug'), 0.220 mg for N221204 (PRL 132); not stated for the others, taken as 0.215 mg (the Hybrid-E layer of 66 um inside 1050 um holds 0.219 mg at 0.255 g/cm^3, our arithmetic). The DSR is made an areal density by rho R = 18.5 x DSR (PRE 109, 025203: a coefficient between 18 and 19). Every temperature is apparent and burn-averaged, from neutron spectra; the pressures, hot-spot energies and areal densities are LLNL's inferences, which a 0-D isobaric hot spot at T_DD reproduces from the yield, volume and burn width. No shot after N221204 has a stated stagnation pressure: a search of the 2023 to 2026 sources (sixteen shots, listed in the record) found pressures as points on one slide alone, inferred from the yields, and most of those shots lack a temperature or a DSR; none is compared. Nothing fitted. Compared with all seven shots.";
const PASS_LINE: &str = "For the solver as committed at 9c6fb1f. The question: T_DD, the pressure, the equivalent radius of the volume, the shell areal density the DSR leaves after the hot spot's own rho R, the fuel mass. Procedure A gives these as the stagnation state; procedure C finds the stagnation temperature and pressure whose neutron-averaged values equal T_DD and the pressure within 1 percent (Newton's method, at most 12 iterations). Targets on the central inputs, for A and for C each: Y, the yield within a factor of 2 of the measured; W, the burn width within 30 percent of the measured; B, for the ignition shots, the burn-up within a factor of 2; G, for the burning-plasma shots, the yield amplification within a factor of 1.5. A shot is within its targets when it is within each of them. Uncertainty: 64 seeded draws per shot under A.";
const RESULT: &str = "With the solver at 9c6fb1f, 3 of 24 targets reached and no shot within all of its own. Procedure A. N221204: yield 2.32 MJ against 3.1 (Y, within a factor of 2), burn-up 3.1 percent against 4.33 (B, within a factor of 2), burn width 113 ps against 75 (W, outside 30 percent), burned through, neutron-averaged temperature 43 keV against T_DD 12. N220919: yield 5.0 MJ against 1.2, burn width 110 ps against 92 (W, within 30 percent), burn-up 6.9 percent against 1.6. N210808: refused, NonFiniteValue at 19 ps, a fault of the integrator since mended. The four burning-plasma shots: yield 4.3 to 6.1 MJ against 0.10 to 0.17 MJ measured, yield amplification 61 to 139 against 3.5 to 5.7, burn width 13 to 17 ps against 107 to 137: the solver ignited and burned through every one of them, which NIF did not. Procedure C found no state on any shot: the solver's burn has no state whose neutron-averaged temperature and pressure are the published ones between fizzling (neutron-averaged temperature near 2 to 3 keV) and burning through (above 25 keV). Under the 64 draws of A: N221204 answered on 29, yield 2.4 to 6.3 MJ (16th to 84th percentile), burn width 85 to 119 ps; N210808 answered on 28, 1.9 to 5.1 MJ, 88 to 115 ps; the burning-plasma shots answered on all 64, 2.9 to 6.7 MJ. The refusals among the draws were ShellArealDensityNotPositive (the DSR left less than the hot spot's own rho R) and integrator faults since mended. The record's forecast put the ignition shots outside their targets under A and C, as they came out, and the burning-plasma shots within some, where they came out a factor of 25 to 60 high in yield. The cause. The published volume, DSR and fuel mass leave every one of the seven shots a shell less dense than its hot spot (4.5 to 46 g/cm^3 against 66 to 94), so at one pressure hotter than it: ablated, that hot fuel heated the hot spot and ignited it. The model's premise is a hot spot inside colder, denser fuel, and the solver refuses such a question by name (ShellNotDenserThanHotSpot); on the served build every question of this comparison is refused so. Two more changes of the model since: the conduction ablation was raised by 1.50 to Betti's own published profile, and two integrator faults found on a grid of 2400 questions were mended. With no target, the same hot spots with a shell of 1.5, 3, 10 or 30 times the hot spot's density: the first burning-plasma shots N201101 and N201122 then stay sub-ignited: at three times, yield 67 and 61 kJ against 98 and 106 measured, yield amplification 3.5 and 3.4 against 3.5 and 4.2, burn width 120 and 125 ps against 118 and 137; at ten times 41 and 37 kJ, 2.7 and 2.6, 88 and 90 ps. N210207 and N210220 ignite and burn through at every density (9 to 13 MJ against 0.17 and 0.16). N221204 burns through at every density, 4.2 to 12.7 MJ against 3.1, burn width 8.6 to 19.5 ps against 75, burn-up 5.7 to 17 percent against 4.3; N210808 5.5 to 13 MJ against 1.3. So the clean, spherical burn of this model crosses ignition between the N201122 and the N210207 stagnation states, where NIF's implosions crossed it between N210220 and N210808; past that the model burns the whole fuel in tens of picoseconds where NIF's ignition shots burned 2 to 4 percent over 75 to 90 ps. With no target, the questions built the way an independent reviewer proposed: the light shell comes from putting all the fuel into one uniform shell with the DSR's column alone, and NIF's own analysis reads the same DSR as a thin dense shell holding part of the fuel. The temperature, pressure, radius and shell areal density as before; the fuel mass given is the stagnated mass, the hot spot's and a shell of that areal density at 255 or 510 g/cm^3, 35 to 60 percent of the fuel; burn-up over the whole fuel. With no ablator: N210808 1.12 and 1.40 MJ against 1.3, burn-up 1.5 and 1.9 percent against 1.77; N221204 2.19 and 2.97 MJ against 3.1, burn-up 2.9 and 4.0 percent against 4.33 (served as example_solve_n221204_stagnated_255 and example_solve_n221204_stagnated_510); but burn widths 18 to 28 ps against 75 to 89, burning through; N220919 3.3 and 4.1 MJ against 1.2; N210207 and N210220 1.2 and 1.1 MJ at 255 g/cm^3, 0.33 and 0.24 at 510, against 0.17 and 0.16; N201101 and N201122 46 to 64 kJ against 98 and 106, yield amplification 2.7 to 3.3 against 3.5 and 4.2, burn widths 63 to 78 ps against 118 and 137. With the remaining ablator (5 percent of the capsule where stated, else the DT mass): N201101 and N201122 burn 106 and 115 ps against 118 and 137, yield amplification 3.6 and 3.4 against 3.5 and 4.2, yield 0.58 to 0.70 of the measured at 255 g/cm^3; every later shot ignites further, N221204 10.4 to 10.7 MJ against 3.1 with burn widths 32 to 36 ps, N210808 6.5 MJ at 255 g/cm^3 (StepLimitReached at 510, its fuel shell 1.3 um thick). No reading reproduces both the yield and the burn width of the ignition shots.";
const LIMITS: &str = "Fitted constants: none. What the comparison tests: whether a clean, spherical, one-temperature hot spot in a uniform shell, started from the published stagnation numbers, burns as NIF's hot spots burned. Outside what it tests: the stagnation itself, which is an input, so the solver predicts no implosion; the published pressures, energies and areal densities, which are LLNL's inferences. The physical limits enforced by name: the D-T reactivity fit of Bosch and Hale, 0.2 to 100 keV, at the door and at every step (an ignited hot spot that would exceed 100 keV is refused, TemperatureLeftReactivityFit); a degenerate hot spot (HotSpotDegenerate), which radiative collapse can reach; a geometry the fuel cannot fill (FuelMassNotAboveHotSpotMass, ShellMassBelowArealDensity); a shell no denser than the hot spot (ShellNotDenserThanHotSpot). What the model leaves out and NIF has: two temperatures, the hot spot's temperature profile (the conduction loss is that of Betti's profile with its centre at the zone's temperature), mix of ablator into the hot spot, low-mode asymmetry and residual kinetic energy at stagnation (PRE 106 025202 measures a hot-spot velocity of 68 +- 7 km/s on N210808), burn in the dense shell other than by ablation into the hot spot, deposition of alpha energy in the shell other than by ablation, neutron heating. Most make the model's burn larger and faster than a real one, which is the direction of its gap. The shell's temperature is read from the ideal gas, which overstates it where the cold fuel is degenerate and so understates its absorption of bremsstrahlung. The alpha deposition fraction is Krokhin and Rozanov's straight-line one. The solver's own numerics: a burn on the edge of ignition can need the step halved (MAX_REFINEMENTS) and is refused, NotSettled, if it still differs. The measured yields, burn widths and burn-ups are published and were read before the targets were set, and the record's forecast quotes them; the solver had not been run on the shots' inputs. The solver served is the one compared with the changes the result states. A proof shows that an answer is the one the engine computed; how close it is to the hot spot is the comparison above.";

fn example() -> Value {
    json!({
        "hot_spot_temperature_kev": 4.0,
        "hot_spot_pressure_gbar": 150.0,
        "hot_spot_radius_um": 35.0,
        "shell_areal_density_g_per_cm2": 0.8,
        "fuel_mass_mg": 0.2,
        "ablator_mass_mg": 0.0
    })
}

/// N221204 as PRL 132 065102 Table I prints it, made into the solver's
/// question by the protocol's rule (evidence/research/icf_nif_hotspot/
/// analysis/inputs.py): T_DD, the pressure, the radius of the neutron-image
/// volume, the shell areal density 18.5 DSR less the hot spot's own, and the
/// 0.220 mg of fuel. Every number but the fuel mass is that of the two
/// stagnated-mass examples.
fn n221204(fuel_mass_mg: f64) -> Value {
    json!({
        "hot_spot_temperature_kev": 12.0,
        "hot_spot_pressure_gbar": 608.0,
        "hot_spot_radius_um": 67.53056246593466,
        "shell_areal_density_g_per_cm2": 0.0776038256167414,
        "fuel_mass_mg": fuel_mass_mg,
        "ablator_mass_mg": 0.0
    })
}

/// The second party's consistent reading of N221204, after the fact and not
/// gated: the fuel mass is the stagnated mass, the hot spot's and a uniform
/// shell of the same areal density at 255 g/cm^3
/// (analysis/reevaluate_consistent.py, out/consistent.json).
fn n221204_stagnated_255() -> Value {
    n221204(0.13169370648325554)
}

/// The same at 510 g/cm^3.
fn n221204_stagnated_510() -> Value {
    n221204(0.13066903769359978)
}

/// The worked refusal: N221204 as the protocol made it into a question, the
/// fuel outside the hot spot all in one uniform shell. Turned away by name
/// before anything is computed, with no seal.
pub(crate) fn example_refused() -> Value {
    let request = json!({ "solver": "icf_hotspot_burn", "parameters": n221204(0.22) });
    let answered = match crate::solution::prepare_in(&crate::view::View::full(), &request) {
        Err(refused) => refused.message().to_string(),
        Ok(_) => unreachable!("N221204's uniform shell is less dense than its hot spot"),
    };
    json!({
        "solve": request,
        "answered": answered,
        "says": "N221204 as PRL 132 065102 Table I prints it, made into a question with all the fuel outside the hot spot in one uniform shell holding the down-scatter ratio's column: a shell less dense than the hot spot. A question outside the model's premise, a hot spot inside colder, denser fuel, is turned away by name before anything is computed: the answer is this one sentence, an error and not a result, and nothing is sealed. The comparison with the seven NIF shots used this construction; example_solve_n221204_stagnated_255 and example_solve_n221204_stagnated_510 are the stagnated reading of the same shot.",
    })
}

fn igniting_example() -> Value {
    json!({
        "hot_spot_temperature_kev": 7.0,
        "hot_spot_pressure_gbar": 300.0,
        "hot_spot_radius_um": 30.0,
        "shell_areal_density_g_per_cm2": 0.8,
        "fuel_mass_mg": 0.2,
        "ablator_mass_mg": 0.2
    })
}

/// The solver's own question, from the values of `parameters`.
fn question(given: &Given) -> Result<HotSpotQuestion, String> {
    Ok(HotSpotQuestion {
        hot_spot_temperature_kev: given.number("hot_spot_temperature_kev")?,
        hot_spot_pressure_gbar: given.number("hot_spot_pressure_gbar")?,
        hot_spot_radius_um: given.number("hot_spot_radius_um")?,
        shell_areal_density_g_per_cm2: given.number("shell_areal_density_g_per_cm2")?,
        fuel_mass_mg: given.number("fuel_mass_mg")?,
        ablator_mass_mg: given.number("ablator_mass_mg")?,
    })
}

/// Every limit the request shows, asked of the solver's own check before a
/// physics permit is taken. The sentence is the solver's, with its name for
/// the refusal.
fn check(given: &Given) -> Result<(), String> {
    burn::admit(&question(given)?).map_err(|reject| format!("Physical Violation: {reject} ({}).", reject_name(&reject)))
}

fn solve(given: &Given) -> Result<Solved, String> {
    Ok(solved(burn::solve(&question(given)?)))
}

/// What the solver returned, in the words every solver shares.
pub fn solved(returned: Result<HotSpotBurn, HotSpotReject>) -> Solved {
    let b = match returned {
        Ok(b) => b,
        Err(reject) => {
            let name = reject_name(&reject);
            let sealed = REFUSALS.iter().any(|refusal| refusal.name == name && refusal.sealed);
            return Solved::Refused(Refused {
                name,
                sentence: if sealed { reject.to_string() } else { format!("{reject} ({name})") },
                iterations: match reject {
                    HotSpotReject::StepLimitReached { steps } => Some(steps as u64),
                    _ => None,
                },
            });
        }
    };
    let one = |name: &str, value: f64| (name.to_string(), Published::Number(value));
    let column = |name: &str, k: usize| (name.to_string(), Published::Numbers(b.history.iter().map(|row| row[k]).collect()));
    let mut field = Vec::with_capacity(b.history.len() * 5);
    for row in &b.history {
        field.extend_from_slice(row);
    }
    Solved::Answer(Answer {
        iterations: b.steps as u64,
        result: vec![
            one("yield_mj", b.yield_mj),
            one("neutron_yield", b.neutron_yield),
            one("yield_without_alpha_heating_mj", b.yield_without_alpha_heating_mj),
            one("yield_amplification", b.yield_amplification),
            one("burn_up_fraction", b.burn_up_fraction),
            one("burn_width_ps", b.burn_width_ps),
            one("bang_time_ps", b.bang_time_ps),
            one("peak_fusion_power_w", b.peak_fusion_power_w),
            one("burn_averaged_temperature_kev", b.burn_averaged_temperature_kev),
            one("burn_averaged_pressure_gbar", b.burn_averaged_pressure_gbar),
            one("burn_averaged_radius_um", b.burn_averaged_radius_um),
            one("burn_averaged_hot_spot_areal_density_g_per_cm2", b.burn_averaged_hot_spot_areal_density_g_per_cm2),
            one("burn_averaged_shell_areal_density_g_per_cm2", b.burn_averaged_shell_areal_density_g_per_cm2),
            one("hot_spot_mass_at_bang_mg", b.hot_spot_mass_at_bang_mg),
            one("hot_spot_energy_at_bang_kj", b.hot_spot_energy_at_bang_kj),
            one("peak_temperature_kev", b.peak_temperature_kev),
            one("peak_pressure_gbar", b.peak_pressure_gbar),
            one("fuel_ablated_fraction", b.fuel_ablated_fraction),
            one("burned_through", b.burned_through),
            one("self_heating_ratio_at_bang", b.self_heating_ratio_at_bang),
            one("self_heating_ratio_peak", b.self_heating_ratio_peak),
            one("alpha_energy_over_compression_work_at_bang", b.alpha_energy_over_compression_work_at_bang),
            one("alpha_energy_over_compression_work", b.alpha_energy_over_compression_work),
            one("lawson_parameter_no_alpha", b.lawson_parameter_no_alpha),
            one("lawson_parameter_pressure_time", b.lawson_parameter_pressure_time),
            one("lawson_criterion_lindl", b.lawson_criterion_lindl),
            one("alpha_energy_over_twice_hot_spot_energy", b.alpha_energy_over_twice_hot_spot_energy),
            one("q_alpha_hot_spot", b.q_alpha_hot_spot),
            one("alpha_energy_mj", b.alpha_energy_mj),
            one("alpha_energy_in_hot_spot_mj", b.alpha_energy_in_hot_spot_mj),
            one("alpha_energy_through_shell_mj", b.alpha_energy_through_shell_mj),
            one("alpha_energy_escaped_mj", b.alpha_energy_escaped_mj),
            one("alpha_energy_in_ablator_mj", b.alpha_energy_in_ablator_mj),
            one("neutron_energy_mj", b.neutron_energy_mj),
            one("bremsstrahlung_mj", b.bremsstrahlung_mj),
            one("bremsstrahlung_absorbed_in_shell_mj", b.bremsstrahlung_absorbed_in_shell_mj),
            one("conduction_returned_mj", b.conduction_returned_mj),
            one("compression_work_mj", b.compression_work_mj),
            one("expansion_work_mj", b.expansion_work_mj),
            one("energy_at_start_mj", b.energy_at_start_mj),
            one("energy_at_end_mj", b.energy_at_end_mj),
            one("start_time_ps", b.start_time_ps),
            one("shell_density_g_per_cm3", b.shell_density_g_per_cm3),
            one("ablator_density_g_per_cm3", b.ablator_density_g_per_cm3),
            one("stagnation_hot_spot_mass_mg", b.stagnation_hot_spot_mass_mg),
            column("history_time_ps", 0),
            column("history_fusion_power_w", 1),
            column("history_temperature_kev", 2),
            column("history_pressure_gbar", 3),
            column("history_radius_um", 4),
        ],
        invariants: vec![one("energy_residual", b.energy_residual), one("particle_residual", b.particle_residual)],
        field,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::solvers::describe_value;

    const SOLVER_SOURCE: &str = include_str!("../../genesis_solvers/src/icf_hotspot_burn.rs");

    /// `(variant, doc sentence)` for every variant of `HotSpotReject`, read
    /// from the solver's source. An intra-doc link is written as its bare code span.
    fn documented_rejects() -> Vec<(String, String)> {
        let start = SOLVER_SOURCE.find("pub enum HotSpotReject {").expect("HotSpotReject is in the solver");
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
        let body = describe_value(ICF_HOTSPOT_BURN.name).expect("described");
        let refusals = body["refusals"].as_array().expect("refusals");
        assert_eq!(refusals.len(), documented.len());
        for (refusal, listed) in refusals.iter().zip(REFUSALS) {
            let reported = if listed.sealed { "200, outcome refused, sealed" } else { "422, no seal" };
            assert_eq!(refusal["reported_as"], reported, "{}", refusal["name"]);
        }
    }

    #[test]
    fn the_reference_is_the_one_stored_in_the_solver_test() {
        let body = describe_value(ICF_HOTSPOT_BURN.name).expect("described");
        let url = body["reference"]["url"].as_str().expect("url");
        assert!(SOLVER_SOURCE.contains(&format!("<{url}>")), "{url}");
        let retrieved = body["reference"]["retrieved"].as_str().expect("retrieved");
        assert!(SOLVER_SOURCE.contains(&format!("Retrieved {retrieved}")), "{retrieved}");
        let test = body["reference"]["test"].as_str().expect("test");
        assert!(SOLVER_SOURCE.contains(&format!("fn {test}()")), "{test}");
        // The after-the-fact reading beside it is held by a test of its own.
        assert!(body["reference"]["covers"].as_str().unwrap().contains("the test nif_n221204_stagnated_mass"));
        assert!(SOLVER_SOURCE.contains("fn nif_n221204_stagnated_mass()"));
    }

    #[test]
    fn every_published_name_is_in_the_entry() {
        let b = match solved(burn::solve(&HotSpotQuestion {
            hot_spot_temperature_kev: 4.0,
            hot_spot_pressure_gbar: 150.0,
            hot_spot_radius_um: 35.0,
            shell_areal_density_g_per_cm2: 0.8,
            fuel_mass_mg: 0.2,
            ablator_mass_mg: 0.0,
        })) {
            Solved::Answer(answer) => answer,
            Solved::Refused(refused) => panic!("{}", refused.sentence),
        };
        assert_eq!(b.result.len(), ICF_HOTSPOT_BURN.result.len());
        for ((name, value), output) in b.result.iter().zip(ICF_HOTSPOT_BURN.result) {
            assert_eq!(name, output.name);
            assert_eq!(value.shape(), output.shape);
        }
        assert_eq!(b.invariants.len(), ICF_HOTSPOT_BURN.invariants.len());
        for ((name, _), output) in b.invariants.iter().zip(ICF_HOTSPOT_BURN.invariants) {
            assert_eq!(name, output.name);
        }
        assert_eq!(b.field.len() % 5, 0);
        assert!(b.field.len() / 5 <= burn::HISTORY_ROWS_MAX);
    }
}
