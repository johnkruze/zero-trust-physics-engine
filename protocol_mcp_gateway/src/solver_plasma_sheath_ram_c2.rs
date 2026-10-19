//! The entry for `plasma_sheath_ram_c2`: the plasma sheath of the RAM C-II
//! blunt cone at one flight point, its electron density and whether a radio
//! signal is blacked out. The numbers come from the `genesis_solvers`
//! constants and the sentence of each refusal from the solver's source.
//! Stamped from the kit.

use genesis_solvers::air11;
use genesis_solvers::plasma_sheath_ram_c2::{self as sheath, PlasmaSheathRamC2Field, PlasmaSheathRamC2Question, PlasmaSheathRamC2Reject};
use serde_json::{Value, json};

use crate::solvers::{Answer, FurtherExample, Given, Input, Kind, Output, Published, Refusal, Refused, Sections, Shape, Solved, Solver};

const fn number(name: &'static str, unit: &'static str, note: &'static str) -> Output {
    Output { name, unit, shape: Shape::Number, each: &[], note, asked_by: None }
}

pub const PLASMA_SHEATH_RAM_C2: Solver = Solver {
    name: "plasma_sheath_ram_c2",
    summary: SUMMARY,
    inputs: &[
        // Every limit is asked by the solver's own admit(), in its order:
        // the altitude has a name at each end, and the bow shock hangs on
        // the altitude's speed of sound.
        Input {
            name: "altitude_m",
            kind: Kind::Number { minimum: 0.0, maximum: sheath::KNUDSEN_ALTITUDE_M, unit: "m" },
            refusal: Some("RarefiedFlow"),
            bound: "The geometric altitude, 0 to 80,637 m: below sea level is AltitudeBelowSeaLevel; above 80,637 m, where the freestream Knudsen number (the 1962 standard's mean free path over the 0.1524 m nose radius) reaches 0.03, the shock is thicker than the layer it bounds and the question is RarefiedFlow. From 73.6 km (Knudsen number 0.01) the shock is already as thick as the layer: answered, and the answer says so (shock_as_thick_as_layer). Compared with flight from 70 to 80 km.",
            only_when: None,
            asked_by_check: true,
        },
        Input {
            name: "velocity_m_s",
            kind: Kind::NumberAbove { above: 0.0, maximum: sheath::VELOCITY_CEILING_M_S, unit: "m/s" },
            refusal: Some("NoBowShock"),
            bound: "The flight speed through the air, above the speed of sound at that altitude (a Mach number of 1 or less is NoBowShock) and at most 20,000 m/s (above it VelocityPastFits: the gas behind the shock is past 20,000 K, the top of the NASA Glenn fits, at every altitude). Below that the reach is the air's: a state behind the shock above 20,000 K is TemperatureAboveFits (from about 12.3 km/s at sea level, 18 km/s at 50 km, 19.3 km/s at 80 km), and a gas cooled below 298.15 K on the body, the floor of the ions' fits, is TemperatureBelowFits (near 1 km/s and below). Compared with flight at 7.65 km/s.",
            only_when: None,
            asked_by_check: true,
        },
        Input {
            name: "signal_frequency_hz",
            kind: Kind::Number { minimum: sheath::SIGNAL_FREQUENCY_FLOOR_HZ, maximum: sheath::SIGNAL_FREQUENCY_CEILING_HZ, unit: "Hz" },
            refusal: Some("SignalFrequencyBelowResolution"),
            bound: "The frequency of the signal whose blackout is asked, 1 MHz to 1e150 Hz: below 1 MHz its critical density, 1.24e10 per cubic metre, nears the 1e8 per cubic metre to which electrons are resolved (SignalFrequencyBelowResolution); above 1e150 Hz the critical density nears the largest number 64-bit arithmetic carries (SignalFrequencyBeyondArithmetic). It does not enter the electron density.",
            only_when: None,
            asked_by_check: true,
        },
    ],
    kept_inside: "The vehicle (RAM C-II's 15.24 cm hemispherical nose and 9 degree cone, its two reflectometer stations), the 1962 standard atmosphere, the air's composition, its thermodynamics and its 32 reactions, and the tolerances stay inside the solver.",
    result: &[
        number("freestream_temperature_k", "K", "The 1962 standard's temperature at the altitude."),
        number("freestream_pressure_pa", "Pa", "The pressure of the air the shock meets: the standard's density and temperature, with N2 and O2 in the standard's ratio."),
        number("freestream_density_kg_m3", "kg/m^3", "The standard's density at the altitude."),
        number("mach_number", "1", "The flight speed over the speed of sound of the air ahead."),
        number("knudsen_number", "1", "The standard's mean free path over the 0.1524 m nose radius."),
        number("shock_as_thick_as_layer", "1", "1 where the Knudsen number is above 0.01 (above about 73.6 km): the shock, a few mean free paths thick, is as thick as the shock layer, and the boundary layer fills much of it; 0 below. Where it is 1, as at 77.7 and 80.0 km in flight, the density reads about 3 times the measured."),
        number("shock_temperature_k", "K", "Behind the normal shock on the axis, the air in chemical equilibrium."),
        number("shock_pressure_pa", "Pa", "Behind the normal shock."),
        number("shock_density_kg_m3", "kg/m^3", "Behind the normal shock."),
        number("shock_density_ratio", "1", "The density ahead over the density behind the shock."),
        number("shock_electron_density_m3", "m^-3", "Electrons per cubic metre behind the normal shock, in equilibrium."),
        number("stagnation_temperature_k", "K", "At the stagnation point: the gas behind the shock brought to rest without loss, in equilibrium."),
        number("stagnation_pressure_pa", "Pa", "At the stagnation point."),
        number("stagnation_density_kg_m3", "kg/m^3", "At the stagnation point."),
        number("stagnation_electron_density_m3", "m^-3", "Electrons per cubic metre at the stagnation point: the most in the inviscid layer."),
        number("stagnation_plasma_frequency_hz", "Hz", "The plasma frequency there, sqrt(n_e e^2 / (eps0 m_e)) / (2 pi)."),
        number("stagnation_velocity_gradient_per_s", "1/s", "The Newtonian speed gradient along the wall at the stagnation point, sqrt(2 (p0 - p_inf) / rho0) / R."),
        number("signal_critical_density_m3", "m^-3", "The electron density at which the asked signal is the plasma frequency, 4 pi^2 eps0 m_e f^2 / e^2."),
        number("signal_blacked_out_at_stagnation", "1", "1 where the stagnation point's electron density is above the signal's critical density (the signal is cut off), 0 where it is not."),
        number("station1_pressure_pa", "Pa", "Station 1, x/D 0.15, 4.47 cm along the axis, 45.0 degrees around the nose: the modified Newtonian pressure."),
        number("station1_temperature_k", "K", "The gas from the stagnation point at station 1, at finite rate."),
        number("station1_velocity_m_s", "m/s", "Its speed there."),
        number("station1_electron_density_m3", "m^-3", "Its electrons per cubic metre: the answer at station 1."),
        number("station1_electron_density_resolved", "1", "1 where that density is at or above 1e8 per cubic metre, the resolution; 0 where it is below, reported as computed, small and of either sign."),
        number("station1_plasma_frequency_hz", "Hz", "The plasma frequency at station 1."),
        number("station1_equilibrium_electron_density_m3", "m^-3", "The same expansion held in chemical equilibrium: the fast-chemistry bound."),
        number("station1_equilibrium_temperature_k", "K", "Its temperature."),
        number("station1_frozen_electron_density_m3", "m^-3", "The same expansion with the stagnation point's composition frozen: the no-chemistry bound."),
        number("station1_frozen_temperature_k", "K", "Its temperature."),
        number("station1_transit_time_s", "s", "The gas's time from the start of the streamline, a thousandth of the nose radius from the axis."),
        number("station1_signal_blacked_out", "1", "1 where station 1's electron density is above the signal's critical density, 0 where it is not."),
        number("station2_pressure_pa", "Pa", "Station 2, x/D 0.76, 23.16 cm along the axis, on the cone: the modified Newtonian pressure."),
        number("station2_temperature_k", "K", "The gas from the stagnation point at station 2, at finite rate."),
        number("station2_velocity_m_s", "m/s", "Its speed there."),
        number("station2_electron_density_m3", "m^-3", "Its electrons per cubic metre: the answer at station 2."),
        number("station2_electron_density_resolved", "1", "1 where that density is at or above 1e8 per cubic metre, the resolution; 0 where it is below."),
        number("station2_plasma_frequency_hz", "Hz", "The plasma frequency at station 2."),
        number("station2_equilibrium_electron_density_m3", "m^-3", "The same expansion held in chemical equilibrium."),
        number("station2_equilibrium_temperature_k", "K", "Its temperature."),
        number("station2_frozen_electron_density_m3", "m^-3", "The same expansion with the stagnation point's composition frozen."),
        number("station2_frozen_temperature_k", "K", "Its temperature."),
        number("station2_transit_time_s", "s", "The gas's time from the start of the streamline."),
        number("station2_signal_blacked_out", "1", "1 where station 2's electron density is above the signal's critical density, 0 where it is not."),
        number("step_tolerance", "1", "The relative step tolerance whose streamline was accepted: the first of 1e-5, 1e-6, 1e-7 and 1e-8 to agree with the one before it."),
    ],
    invariants: &[
        number("shock_mass_residual", "1", ""),
        number("shock_momentum_residual", "1", ""),
        number("shock_energy_residual", "1", ""),
        number("stagnation_enthalpy_residual", "1", ""),
        number("stagnation_entropy_residual", "1", ""),
        number("streamline_nuclei_residual", "1", ""),
        number("streamline_charge_residual", "1", ""),
        number("streamline_energy_residual", "1", ""),
        number("streamline_entropy_fall", "1", ""),
    ],
    iterations: "The accepted steps of the accepted streamline, plus every Newton iteration of every equilibrium solved. On a refusal it is null.",
    field: field_words,
    answer: "a settled shock layer",
    refusals: REFUSALS,
    door_refusals: &[],
    sections,
    validation: Some(validation),
    example,
    further_examples: &[
        FurtherExample { key: "example_solve_s_band", parameters: s_band_example },
        FurtherExample { key: "example_solve_glide_gps", parameters: glide_example },
    ],
    parts: &[],
    check,
    solve,
};

const SUMMARY: &str = "The plasma sheath around NASA's RAM C-II re-entry probe (a 15.24 cm hemisphere on a 9 degree cone) at one flight point from sea level to 80,637 m: the equilibrium normal shock and stagnation point of 11-species air, the gas carried along the body to the nose station (x/D 0.15) and the cone station (x/D 0.76) with Dunn and Kang's 26 reactions at finite rate, the electron density, the plasma frequency, and whether a radio signal of the asked frequency is cut off (blacked out) at each; mass, momentum and energy held across the shock, and nuclei, the reactions' charge, total enthalpy and the second law at every step of the streamline; no constant fitted; compared once, forecast first, with RAM C-II's reflectometer onsets at 7.65 km/s: met at 70 km (0.94 of the measured density at the cone station's X band) and missed at 77.7 and 80 km (3.28 and 2.92 times), where the shock is as thick as the layer, so by the rule fixed beforehand the comparison missed.";

/// Each `PlasmaSheathRamC2Reject` variant with the sentence from its doc
/// comment. The limits the question shows by itself (the solver's `admit`)
/// are answered at the door.
const REFUSALS: &[Refusal] = &[
    Refusal { name: "NonFiniteValue", meaning: "An input or an intermediate value was NaN or infinite.", sealed: true },
    Refusal { name: "AltitudeBelowSeaLevel", meaning: "The altitude is below sea level.", sealed: false },
    Refusal {
        name: "RarefiedFlow",
        meaning: "The altitude is above 80 637 m, where the freestream Knudsen number, mean free path over nose radius, reaches 0.03: the shock is thicker than the shock layer it bounds.",
        sealed: false,
    },
    Refusal { name: "NoBowShock", meaning: "The flight Mach number is 1 or less: there is no bow shock.", sealed: false },
    Refusal {
        name: "VelocityPastFits",
        meaning: "The flight speed is above 20 000 m/s: behind the shock the gas is past 20 000 K, the top of the NASA Glenn fits, at every altitude from sea level to 80 637 m.",
        sealed: false,
    },
    Refusal {
        name: "SignalFrequencyBelowResolution",
        meaning: "The signal frequency is below 1 MHz, whose critical density, 1.24e10 per cubic metre, is within two orders of the 1e8 per cubic metre to which the streamline resolves electrons.",
        sealed: false,
    },
    Refusal {
        name: "SignalFrequencyBeyondArithmetic",
        meaning: "The signal frequency is above 1e150 Hz, whose critical density, 1.24e298 per cubic metre, is near the largest number 64-bit arithmetic carries.",
        sealed: false,
    },
    Refusal { name: "TemperatureAboveFits", meaning: "A state of the gas is above 20 000 K, the top of the NASA Glenn fits.", sealed: true },
    Refusal { name: "TemperatureBelowFits", meaning: "A state of the gas is below 298.15 K, the floor of the ions' fits.", sealed: true },
    Refusal { name: "EquilibriumNotFound", meaning: "The element potentials of an equilibrium did not settle.", sealed: true },
    Refusal { name: "ShockNotFound", meaning: "No density ratio closes the Rankine-Hugoniot equations.", sealed: true },
    Refusal { name: "StagnationStateNotFound", meaning: "No equilibrium state has the stagnation enthalpy and entropy.", sealed: true },
    Refusal { name: "StreamlineStalled", meaning: "The streamline's step fell below 1e-18 s or its step count passed 400 000.", sealed: true },
    Refusal { name: "NotSettled", meaning: "The smallest step tolerance, 1e-9, still did not agree with the one before it.", sealed: true },
    Refusal { name: "ShockLedgerBroken", meaning: "The mass, momentum or energy flux is not the same on both sides of the shock.", sealed: true },
    Refusal { name: "StagnationLedgerBroken", meaning: "The stagnation state does not hold the shock's total enthalpy and entropy.", sealed: true },
    Refusal { name: "NucleiNotConserved", meaning: "The nuclei of nitrogen or oxygen per kilogram moved along the streamline.", sealed: true },
    Refusal { name: "ChargeNotNeutral", meaning: "The reactions made electrons and ions in unequal numbers.", sealed: true },
    Refusal { name: "EnergyLedgerBroken", meaning: "The total enthalpy moved along the streamline.", sealed: true },
    Refusal { name: "EntropyFell", meaning: "The entropy of the gas fell along the streamline below the highest it had reached.", sealed: true },
];

fn field_words() -> String {
    "SHA-256 of the count of the numbers that follow as 8 little-endian bytes, which is 3 times one more than the accepted steps, then for the streamline's start and each accepted step in turn the surface distance from the stagnation point in m, the temperature in K and the electron density per cubic metre, all as little-endian f64 bytes.".to_string()
}

fn sections() -> Sections {
    Sections {
        locked: json!({
            "vehicle": "RAM C-II (NASA TN D-6062): a hemispherical nose of 0.1524 m radius, a 9 degree half-angle cone tangent to it, reference diameter 0.3048 m; reflectometer station 1 at 4.47 cm along the axis (x/D 0.15, 45.0 degrees around the nose), station 2 at 23.16 cm (x/D 0.76, on the cone).",
            "nose_radius_m": sheath::NOSE_RADIUS_M,
            "cone_half_angle_deg": sheath::CONE_HALF_ANGLE_DEG,
            "station_x_m": sheath::STATION_X_M,
            "atmosphere": "U.S. Standard Atmosphere, 1962: molecular-scale temperature straight in geopotential altitude (H = r0 Z / (r0 + Z), r0 6,356,766 m) through its layers to 90 km, the hydrostatic pressure from 101,325 Pa, molecular weight 28.9644, R* 8.31432 J/(mol K), the mean free path 1/(sqrt(2) pi sigma^2 n) with sigma 3.65e-10 m. It reproduces its printed tables to 3e-4 in density at 61, 71, 81 and 85 km.",
            "air": "N2 and O2 alone, in the standard's ratio of 78.084 to 20.9476 by volume, at the standard's density.",
            "species": air11::SPECIES_NAMES,
            "thermodynamics": "NASA Glenn nine-coefficient fits (McBride, Zehe and Gordon, NASA TP-2002-211556; NASA CEA thermo.inp at github.com/nasa/cea 985fd33), 200 K (298.15 K for the ions and the electron) to 20,000 K, standard state 1 bar, gas constant 8.314510 J/(mol K), the value they were made with. N2 and O2 alone, the freestream, from 180 K to 200 K: the rigid rotor, cp = 7/2 R.",
            "equilibrium": "The Gibbs minimum at a temperature and density with the nuclei of nitrogen and oxygen held and the charge zero, by element potentials, the nuclei to 1e-13.",
            "reactions": "The 26 reactions of Dunn and Kang, NASA CR-2232 (1973) Table I, as 32 with their third bodies, rates C T^n exp(-theta/T) as printed (central values). One temperature. Each reaction's measured direction as printed, the deionization rates of reactions 7, 8, 9, 10 and 13 and the forward rate of the rest, the other by detailed balance with these thermodynamics.",
            "shock": "The Rankine-Hugoniot equations with equilibrium air behind the shock, the density ratio settled by false position to 1e-12 in the pressure balance.",
            "stagnation_point": "The equilibrium state with the shock's total enthalpy and entropy, by Newton's method to 1e-12.",
            "streamline": "From one thousandth of the nose radius off the axis, at the Newtonian speed beta s, the gas at the stagnation point's equilibrium composition; pressure p_inf + (p0 - p_inf) cos^2(phi) on the sphere and p_inf + (p0 - p_inf) sin^2(9 deg) on the cone (modified Newtonian); dz/dt = w/rho, du/dt = -(dp/ds)/rho, dT/dt = ((dp/ds) u/rho - sum h_i dz_i/dt)/cp, ds/dt = u; the electrons the ions' sum; the speed and the distance as logarithms.",
            "blackout": "A signal of frequency f is cut off where the electron density is above 4 pi^2 eps0 m_e f^2 / e^2: the collisionless cut-off. Collisions blur it where the electrons' collision frequency nears the wave's, which TN D-6062 places below 30.48 km.",
            "electron_resolution_m3": sheath::ELECTRON_RESOLUTION_M3,
            "knudsen_limit": sheath::KNUDSEN_LIMIT,
            "knudsen_altitude_m": sheath::KNUDSEN_ALTITUDE_M,
            "fitted": "Nothing.",
            "assumptions": "Inviscid: the gas outside the cold wall's boundary layer, where in flight electrons diffuse to the wall and recombine. One temperature for translation, rotation, vibration, electronic excitation and the free electrons, as Dunn and Kang hold. Clean air, as the flown beryllium nose kept it to 56.39 km. The stagnation point in chemical equilibrium. Adiabatic: the gas radiates nothing away. The body at zero angle of attack.",
        }),
        invariants: json!([
            {
                "name": "shock_mass_residual",
                "rule": "The mass flux ahead of the shock equals the flux behind it, rho1 V = rho2 u2, over rho1 V.",
                "tolerance": sheath::SHOCK_LEDGER_TOLERANCE,
                "unit": "1",
                "held": "at acceptance of the shock",
                "refusal": "ShockLedgerBroken",
            },
            {
                "name": "shock_momentum_residual",
                "rule": "p1 + rho1 V^2 = p2 + rho2 u2^2, over p1 + rho1 V^2.",
                "tolerance": sheath::SHOCK_LEDGER_TOLERANCE,
                "unit": "1",
                "held": "at acceptance of the shock",
                "refusal": "ShockLedgerBroken",
            },
            {
                "name": "shock_energy_residual",
                "rule": "h1 + V^2/2 = h2 + u2^2/2, over h1 + V^2/2.",
                "tolerance": sheath::SHOCK_LEDGER_TOLERANCE,
                "unit": "1",
                "held": "at acceptance of the shock",
                "refusal": "ShockLedgerBroken",
            },
            {
                "name": "stagnation_enthalpy_residual",
                "rule": "The stagnation state's enthalpy equals the shock's total enthalpy, over it.",
                "tolerance": sheath::STAGNATION_LEDGER_TOLERANCE,
                "unit": "1",
                "held": "at acceptance of the stagnation state",
                "refusal": "StagnationLedgerBroken",
            },
            {
                "name": "stagnation_entropy_residual",
                "rule": "The stagnation state's entropy equals the entropy behind the shock, over it.",
                "tolerance": sheath::STAGNATION_LEDGER_TOLERANCE,
                "unit": "1",
                "held": "at acceptance of the stagnation state",
                "refusal": "StagnationLedgerBroken",
            },
            {
                "name": "streamline_nuclei_residual",
                "rule": "The nuclei of nitrogen and of oxygen per kilogram stay those of the stagnation point: the largest change of either over itself.",
                "tolerance": sheath::NUCLEI_TOLERANCE,
                "unit": "1",
                "held": "every accepted step of the streamline",
                "refusal": "NucleiNotConserved",
            },
            {
                "name": "streamline_charge_residual",
                "rule": "The gas is neutral by construction, its electrons the ions' sum; the 32 reactions make as many electrons as ions: the difference over the gross rate of the reactions of charged particles.",
                "tolerance": sheath::CHARGE_TOLERANCE,
                "unit": "1",
                "held": "every accepted step of the streamline",
                "refusal": "ChargeNotNeutral",
            },
            {
                "name": "streamline_energy_residual",
                "rule": "The total enthalpy h + u^2/2 stays that of the start, over the flight's kinetic energy per kilogram, V^2/2: the energy equation (dh = dp/rho) and the momentum equation are stepped apart, and hold it together when both are stepped right.",
                "tolerance": sheath::ENERGY_LEDGER_TOLERANCE,
                "unit": "1",
                "held": "every accepted step of the streamline",
                "refusal": "EnergyLedgerBroken",
            },
            {
                "name": "streamline_entropy_fall",
                "rule": "The second law: the entropy of the gas never falls below the highest it has reached; the reactions never lower it. The largest fall over itself.",
                "tolerance": sheath::ENTROPY_TOLERANCE,
                "unit": "1",
                "held": "every accepted step of the streamline",
                "refusal": "EntropyFell",
            },
        ]),
        convergence: json!({
            "step_tolerances": sheath::STEP_TOLERANCES,
            "settle_tolerance": sheath::SETTLE_TOLERANCE,
            "settle_floor_m3": sheath::SETTLE_FLOOR_M3,
            "start_fraction_of_nose_radius": sheath::START_FRACTION,
            "largest_log_stride": sheath::MAX_LOG_STRIDE,
            "rule": "The streamline is stepped by ROS2, the two-stage L-stable Rosenbrock method of Verwer, Spee, Blom and Hundsdorfer (1999), its Jacobian by forward differences, each step held to the relative tolerance by its embedded first-order solution and to moving the parcel's distance and speed by at most a tenth. The tolerance is lowered from 1e-5 by factors of ten to 1e-8; the first that agrees with the one before it on each station's electron density (to 1e-4 of it, or 1e8 per cubic metre) and temperature (to 1e-4) is accepted. A tolerance whose ledger breaks is refined; the finest's broken ledger is the refusal. Where the streamline starts, from a hundredth to a ten-thousandth of the nose radius, moves the stations by less than 2e-4.",
            "refusal": "NotSettled",
            "run_time": "0.1 to 0.45 s in a release build over most of the range; 0.94 s at sea level and 12 km/s, the slowest of a 90-point sweep from sea level to 80 km and 2 to 14 km/s.",
        }),
        reference: json!({
            "name": sheath::REFERENCE_NAME,
            "url": sheath::REFERENCE_URL,
            "retrieved": sheath::REFERENCE_RETRIEVED,
            "test": "the_comparison_with_the_reflectometers_missed_by_its_rule",
            "covers": "The three compared onsets of TN D-6062 Table X at 7.65 km/s: station 2's X band at 70.0 km (the solver 0.940 of the measured density), station 2's S band at 77.7 km (3.284 times) and station 1's X band at 80.0 km (2.925 times); and the onsets at 84 km refused as RarefiedFlow. The whole comparison is in the validation.",
            "kind": "Measurements of the vehicle itself. The solver's tests also hold its equilibrium shock and stagnation point to Huber's published calculation (NASA TR R-163, 1963: at 230,400 ft, temperature within 0.4 percent, density ratio 0.6, pressure ratio 0.2, stagnation temperature 0.3, electron density 6), its atmosphere to the 1962 standard's printed tables, its worked question to an independent integration of the same equations (SciPy's Radau, 1e-4), and its air to statistical mechanics (atomic oxygen's entropy at 10,000 K) and to detailed balance.",
        }),
    }
}

fn validation() -> [String; 5] {
    let object = "NASA Langley's RAM C-II re-entry probe, flown on 22 August 1968: a 15.24 cm hemispherical nose with a beryllium heat-sink cap on a 9 degree cone, re-entering at 7.59 to 7.65 km/s; microwave reflectometers at four body stations in four bands, of which the solver reaches stations 1 (x/D 0.15) and 2 (x/D 0.76). The solver computes the equilibrium shock and stagnation point and the finite-rate expansion of the gas from the stagnation point to each station. No constant is fitted.".to_string();
    let data = "NASA TN D-6062: Table X, the altitudes at which each reflectometer's reflection coefficient crossed 20 percent on the way down (onset) and back (decay), where the source states the layer's peak electron density is 0.63 of the band's critical density (L 1116, S 3348, X 10,044, Ka 35,000 MHz); Table IX, the trajectory, the flight speed at each altitude. Both transcribed by hand from the report. The source bounds the reflectometers' error: differences above a factor of 3 or 4 are unreasonable. Nothing is fitted, so all of Table X is outside the fit; before the comparison, the span of the onset altitudes (84 to 38 km) and which onsets the source flags had been read, and no value of Table X.".to_string();
    let pass_line = "Compared: the onsets of station 1 at S and X and of station 2 at L, S, X and Ka whose measured altitude lies from 56.39 km (the nose cap's ejection, below which the flown nose ablated) to 80.637 km (the Knudsen limit), each at Table IX's speed. With r = log10(n_e / 0.63 n_c): P1, within a factor of 2 on at least two thirds of these onsets; P2, none beyond a factor of 4. Record SHA-256 9f8580902488f60b806ba73f55722d15f35eb080cfb43968af16394231508c17, with the forecast along the flown trajectory.".to_string();
    let limits = "Compared at 7.65 km/s from 70 to 80 km on two stations, three onsets; the onsets at 84 km are past the Knudsen limit, 80,637 m. The model is inviscid, starts the streamline from equilibrium at the stagnation point, holds one temperature and clean air: above 73.6 km the shock is as thick as the layer, the boundary layer fills much of it and electrons are lost to the cold wall, and there it reads the density about 3 times high. Below 56.39 km the flown nose ablated Teflon and the reflectometers detuned: the decays there read 1.1 to 2000 times the measured, outside what clean air describes. The blackout answer is the collisionless one. The ledgers confirm the equations were solved, and a proof confirms an answer is unchanged; how closely the model matches the flight is the comparison above.".to_string();
    [object, data, pass_line, RESULT.to_string(), limits]
}

/// The comparison with the reflectometer onsets.
const RESULT: &str = "Three onsets compared: station 2's X band at 70.0 km, the solver 0.94 of the measured density; station 2's S band at 77.7 km, 3.28 times; station 1's X band at 80.0 km, 2.92 times. P1, within a factor of 2: one of three (target two of three). P2, beyond a factor of 4: none (target none). The two above a factor of 2 lie above Knudsen number 0.01, where the shock is as thick as the layer, as the forecast expected. Beside these: station 1's Ka onset at 72 km (the source: not read from the 20 percent criterion), 0.81; station 2's Ka onset at 38 km, below the cap's ejection, 0.57; the onsets at 84 km (station 1's S band, station 2's L band) refused as RarefiedFlow; the seven decays, 22 to 31 km, 1.1 to 2000 times the measured. The forecast onset of station 2's X band was 69.43 km, against 70.0 measured.";

fn example() -> Value {
    json!({ "altitude_m": 71_000.0, "velocity_m_s": 7650.0, "signal_frequency_hz": 9.21e9 })
}

/// Apollo's unified S band, 2287.5 MHz, at RAM C-II's speed near the top
/// of the range.
fn s_band_example() -> Value {
    json!({ "altitude_m": 80_000.0, "velocity_m_s": 7650.0, "signal_frequency_hz": 2.2875e9 })
}

/// GPS L1, 1575.42 MHz, on this body at 40 km and 6 km/s, the speed and
/// height of a glide.
fn glide_example() -> Value {
    json!({ "altitude_m": 40_000.0, "velocity_m_s": 6000.0, "signal_frequency_hz": 1.57542e9 })
}

/// The solver's own question, from the values of `parameters`.
fn question(given: &Given) -> Result<PlasmaSheathRamC2Question, String> {
    Ok(PlasmaSheathRamC2Question {
        altitude_m: given.number("altitude_m")?,
        velocity_m_s: given.number("velocity_m_s")?,
        signal_frequency_hz: given.number("signal_frequency_hz")?,
    })
}

/// Every limit the request shows, asked of the solver's own admit() before a
/// physics permit is taken. The sentence is the solver's, with its name for
/// the refusal.
fn check(given: &Given) -> Result<(), String> {
    sheath::admit(&question(given)?).map_err(|reject| format!("Physical Violation: {reject} ({}).", reject.name()))
}

fn solve(given: &Given) -> Result<Solved, String> {
    Ok(solved(sheath::solve(&question(given)?)))
}

/// What the solver returned, in the words every solver shares.
pub fn solved(returned: Result<PlasmaSheathRamC2Field, PlasmaSheathRamC2Reject>) -> Solved {
    let a = match returned {
        Ok(field) => field,
        Err(reject) => {
            let name = reject.name();
            let sealed = REFUSALS.iter().any(|refusal| refusal.name == name && refusal.sealed);
            return Solved::Refused(Refused { name, sentence: if sealed { reject.to_string() } else { format!("{reject} ({name})") }, iterations: None });
        }
    };
    let one = |name: &str, value: f64| (name.to_string(), Published::Number(value));
    let flag = |name: &str, value: bool| (name.to_string(), Published::Number(if value { 1.0 } else { 0.0 }));
    let mut result = vec![
        one("freestream_temperature_k", a.freestream_temperature_k),
        one("freestream_pressure_pa", a.freestream_pressure_pa),
        one("freestream_density_kg_m3", a.freestream_density_kg_m3),
        one("mach_number", a.mach_number),
        one("knudsen_number", a.knudsen_number),
        flag("shock_as_thick_as_layer", a.shock_as_thick_as_layer),
        one("shock_temperature_k", a.shock_temperature_k),
        one("shock_pressure_pa", a.shock_pressure_pa),
        one("shock_density_kg_m3", a.shock_density_kg_m3),
        one("shock_density_ratio", a.shock_density_ratio),
        one("shock_electron_density_m3", a.shock_electron_density_m3),
        one("stagnation_temperature_k", a.stagnation_temperature_k),
        one("stagnation_pressure_pa", a.stagnation_pressure_pa),
        one("stagnation_density_kg_m3", a.stagnation_density_kg_m3),
        one("stagnation_electron_density_m3", a.stagnation_electron_density_m3),
        one("stagnation_plasma_frequency_hz", a.stagnation_plasma_frequency_hz),
        one("stagnation_velocity_gradient_per_s", a.stagnation_velocity_gradient_per_s),
        one("signal_critical_density_m3", a.signal_critical_density_m3),
        flag("signal_blacked_out_at_stagnation", a.signal_blacked_out_at_stagnation),
    ];
    for (k, st) in a.stations.iter().enumerate() {
        let name = |part: &str| format!("station{}_{part}", k + 1);
        result.push((name("pressure_pa"), Published::Number(st.pressure_pa)));
        result.push((name("temperature_k"), Published::Number(st.temperature_k)));
        result.push((name("velocity_m_s"), Published::Number(st.velocity_m_s)));
        result.push((name("electron_density_m3"), Published::Number(st.electron_density_m3)));
        result.push((name("electron_density_resolved"), Published::Number(if st.electron_density_resolved { 1.0 } else { 0.0 })));
        result.push((name("plasma_frequency_hz"), Published::Number(st.plasma_frequency_hz)));
        result.push((name("equilibrium_electron_density_m3"), Published::Number(st.equilibrium_electron_density_m3)));
        result.push((name("equilibrium_temperature_k"), Published::Number(st.equilibrium_temperature_k)));
        result.push((name("frozen_electron_density_m3"), Published::Number(st.frozen_electron_density_m3)));
        result.push((name("frozen_temperature_k"), Published::Number(st.frozen_temperature_k)));
        result.push((name("transit_time_s"), Published::Number(st.transit_time_s)));
        result.push((name("signal_blacked_out"), Published::Number(if st.signal_blacked_out { 1.0 } else { 0.0 })));
    }
    result.push(one("step_tolerance", a.step_tolerance));
    Solved::Answer(Answer {
        iterations: u64::from(a.steps) + u64::from(a.equilibrium_iterations),
        result,
        invariants: vec![
            one("shock_mass_residual", a.shock_mass_residual),
            one("shock_momentum_residual", a.shock_momentum_residual),
            one("shock_energy_residual", a.shock_energy_residual),
            one("stagnation_enthalpy_residual", a.stagnation_enthalpy_residual),
            one("stagnation_entropy_residual", a.stagnation_entropy_residual),
            one("streamline_nuclei_residual", a.streamline_nuclei_residual),
            one("streamline_charge_residual", a.streamline_charge_residual),
            one("streamline_energy_residual", a.streamline_energy_residual),
            one("streamline_entropy_fall", a.streamline_entropy_fall),
        ],
        field: a.streamline,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::solvers::describe_value;

    const SOLVER_SOURCE: &str = include_str!("../../genesis_solvers/src/plasma_sheath_ram_c2.rs");

    /// `(variant, doc sentence)` for every variant of the reject enum, read
    /// from the solver's source.
    fn documented_rejects() -> Vec<(String, String)> {
        let start = SOLVER_SOURCE.find("pub enum PlasmaSheathRamC2Reject {").expect("the reject enum is in the solver");
        let mut found = Vec::new();
        let mut doc = String::new();
        for line in SOLVER_SOURCE[start..].lines().skip(1) {
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
        let body = describe_value(PLASMA_SHEATH_RAM_C2.name).expect("described");
        assert_eq!(body["solver"], "plasma_sheath_ram_c2");
    }

    #[test]
    fn the_worked_question_is_answered_with_every_published_name() {
        let Solved::Answer(answer) = solved(sheath::solve(&PlasmaSheathRamC2Question { altitude_m: 71_000.0, velocity_m_s: 7650.0, signal_frequency_hz: 9.21e9 })) else {
            panic!("refused")
        };
        let names: Vec<&str> = answer.result.iter().map(|(name, _)| name.as_str()).collect();
        let declared: Vec<&str> = PLASMA_SHEATH_RAM_C2.result.iter().map(|output| output.name).collect();
        assert_eq!(names, declared);
        let invariants: Vec<&str> = answer.invariants.iter().map(|(name, _)| name.as_str()).collect();
        let declared: Vec<&str> = PLASMA_SHEATH_RAM_C2.invariants.iter().map(|output| output.name).collect();
        assert_eq!(invariants, declared);
    }
}
