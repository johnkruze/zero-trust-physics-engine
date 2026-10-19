//! The entry for `fuel_rod_triga_jsi`: the instrumented fuel element 6753 of
//! the JSI TRIGA Mark II reactor through a pulse, heated by the core's energy
//! release, and the reading of its fuel thermocouple. Stamped from the kit; the
//! numbers come from the `genesis_solvers` constants and the sentence of each
//! refusal from the solver's source.

use genesis_solvers::fuel_rod_triga_jsi::{self as rod, ElementPulse, ElementQuestion, FuelRodReject};
use serde_json::{Value, json};

use crate::solvers::{Answer, Given, Input, Kind, Output, Published, Refusal, Refused, Sections, Shape, Solved, Solver};

const fn output(name: &'static str, unit: &'static str, shape: Shape, note: &'static str) -> Output {
    Output { name, unit, shape, each: &[], note, asked_by: None }
}

pub const FUEL_ROD_TRIGA_JSI: Solver = Solver {
    name: "fuel_rod_triga_jsi",
    summary: SUMMARY,
    inputs: &[
        // Every limit is asked by the solver's own check, in its order: the
        // history's limits span its two arrays.
        Input {
            name: "start_temperature_k",
            kind: Kind::NumberWithin {
                minimum: rod::POOL_FREEZING_K,
                minimum_exclusive: true,
                maximum: rod::SATURATION_TEMPERATURE_K,
                maximum_exclusive: true,
                unit: "K",
            },
            refusal: Some("StartTemperatureOutsideLiquidPool"),
            bound: "The temperature of the whole element and of the pool at the history's start, above 273.15 K, where the pool freezes, and below 385 K, where the water at the element boils (StartTemperatureOutsideLiquidPool). The pool stays at it through the history. The histories compared started from 293.14 to 305.53 K; outside that the answer carries start_temperature_outside_compared.",
            only_when: None,
            asked_by_check: true,
        },
        Input {
            name: "time_s",
            kind: Kind::NumbersWithin {
                fewest: 1,
                most: rod::HISTORY_POINTS_MAX,
                minimum: 0.0,
                minimum_exclusive: true,
                maximum: rod::HISTORY_SPAN_MAX_S,
                maximum_exclusive: false,
                unit: "s",
            },
            refusal: Some("HistoryTimesNotRising"),
            bound: "The times of the history, s from its start, each after the one before and the first after 0 (HistoryTimesNotRising), to 600 s, the capacity of one solve (HistoryBeyondCapacity); 1 to 100,000 points, as many as core_energy_released_mws (HistoryShape). The compared histories ran in intervals of 1 ms and 10 ms to 14 s; an interval over 10.1 ms or under 1e-5 s, or a span past 14 s, sets history_outside_compared, and the power over a longer interval is its mean.",
            only_when: None,
            asked_by_check: true,
        },
        Input {
            name: "core_energy_released_mws",
            kind: Kind::Numbers {
                fewest: 1,
                most: rod::HISTORY_POINTS_MAX,
                minimum: 0.0,
                maximum: f64::MAX,
                unit: "MW*s",
            },
            refusal: Some("EnergyFalls"),
            bound: "The core's thermal energy released since the history's start, MW s, at each time of the history: from 0 and never falling (EnergyFalls). Over each interval the core's power is the energy released in it over its length, and the element takes 1.387/56 of it. The histories compared released 6.85 to 19.41 MW s by their end, at up to 994 MW over an interval; outside that the answer carries core_energy_outside_compared or core_power_outside_compared. A pulse that heats the meat past 1150 degC is refused as FuelAboveSafetyLimit.",
            only_when: None,
            asked_by_check: true,
        },
    ],
    kept_inside: "The element's geometry and materials, the hot-rod factor 1.387 of ring B, the 56 elements of the core, the water side, the two fitted constants, the grid, the step and the tolerances stay inside the solver.",
    result: &[
        output("thermocouple_temperature_k", "K", Shape::Numbers, "The thermocouple's reading at each time of the history: the meat's temperature at the thermocouple's radius through a first-order lag of the fitted time constant, 1.03 s. Compared with channel T1 of the institute's pulse records."),
        output("fuel_temperature_at_thermocouple_k", "K", Shape::Numbers, "The meat's temperature 0.3 in from the axis at each time of the history, before the lag."),
        output("peak_thermocouple_temperature_k", "K", Shape::Number, "The highest of the readings at the history's times."),
        output("peak_thermocouple_time_s", "s", Shape::Number, "The history time of that reading."),
        output("peak_fuel_temperature_k", "K", Shape::Number, "The highest temperature of any cell of the element at any step: the meat's peak, near its outer radius during the pulse."),
        output("peak_clad_surface_temperature_k", "K", Shape::Number, "The highest temperature of the clad's surface at any step."),
        output("element_energy_deposited_j", "J", Shape::Number, "The element's share of the core's energy over the history: 1.387/56 of it."),
        output("energy_to_water_j", "J", Shape::Number, "Energy that crossed the clad's surface into the pool over the history."),
        output("energy_stored_j", "J", Shape::Number, "Energy stored in the element at the history's end over its start."),
        output("start_temperature_outside_compared", "1", Shape::Number, "1 where the start temperature is outside 293.14 to 305.53 K, the starts of the histories compared, 0 inside. The answer is the same arithmetic either way."),
        output("core_energy_outside_compared", "1", Shape::Number, "1 where the energy released by the history's end is outside 6.85 to 19.41 MW s, that of the histories compared, 0 inside."),
        output("core_power_outside_compared", "1", Shape::Number, "1 where the core power over an interval of the history is above 994 MW, the highest of the histories compared, 0 otherwise."),
        output("history_outside_compared", "1", Shape::Number, "1 where an interval of the history is under 1e-5 s or over 10.1 ms, or the history runs past 14 s, 0 otherwise."),
    ],
    invariants: &[
        output("largest_step_energy_residual", "1", Shape::Number, ""),
        output("run_energy_residual", "1", Shape::Number, ""),
    ],
    iterations: "Newton iterations, all steps together, each one tridiagonal solve across the radius. On a refusal it is the count of the step refused, for NotSettled, and null otherwise.",
    field: field_words,
    answer: "the element through the history",
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

const SUMMARY: &str = "The instrumented fuel element 6753 of the JSI TRIGA Mark II research reactor (Jozef Stefan Institute, Ljubljana) through a pulse: radial conduction through its zirconium rod, U-ZrH1.6 meat, gas gap and steel clad into the pool, heated by 1.387/56 of the core's energy release, and the reading of its fuel thermocouple, with the peak fuel and clad temperatures; every step's energy account closed to 1e-9. Two constants fitted on 39 pulses; compared with 16 pulses left out of the fit: the thermocouple's maximum rise within 10 percent on all 16 (9.2 percent low to 0.3 high), its time within 20 percent on all 16, its rise 10 s after the peak within 10 percent on all 8 that run that long. Fed by the reactor_triga_jsi card through the coupling triga_pulse_to_element_6753, under one chain proof. Refuses above 1150 degC in the meat and above the critical heat flux.";

/// Each refusal with the sentence above its variant in the solver's source.
/// A refusal the request itself shows is not sealed.
const REFUSALS: &[Refusal] = &[
    Refusal { name: "InputNotFinite", meaning: "An input was NaN or infinite.", sealed: false },
    Refusal {
        name: "HistoryShape",
        meaning: "The history has no point, more than 100,000 points, or a different count of times and energies.",
        sealed: false,
    },
    Refusal {
        name: "StartTemperatureOutsideLiquidPool",
        meaning: "The start temperature is at or below 273.15 K, where the pool freezes, or at or above 385 K, where the water at the element boils: the pool is liquid water at the start temperature.",
        sealed: false,
    },
    Refusal {
        name: "HistoryTimesNotRising",
        meaning: "A time of the history is not after the one before it, the first not after zero: time runs forward.",
        sealed: false,
    },
    Refusal {
        name: "HistoryBeyondCapacity",
        meaning: "The history runs past 600 s, the capacity of one solve (300,000 steps of 2 ms).",
        sealed: false,
    },
    Refusal {
        name: "EnergyFalls",
        meaning: "The energy released falls from one time to the next, or is negative: energy released does not fall.",
        sealed: false,
    },
    Refusal {
        name: "FuelAboveSafetyLimit",
        meaning: "The meat passed 1423.15 K (1150 degC), the safety limit of U-ZrH1.6 fuel in a steel clad, where its hydrogen pressure loads the clad past its strength.",
        sealed: true,
    },
    Refusal {
        name: "CriticalHeatFluxPassed",
        meaning: "The heat flux from the clad passed the critical heat flux of saturated water at 385 K, 1.47 MW per m2 (Zuber): there the surface leaves nucleate boiling.",
        sealed: true,
    },
    Refusal { name: "NonFiniteValue", meaning: "A temperature or an energy was NaN or infinite.", sealed: true },
    Refusal { name: "SingularSystem", meaning: "A Newton system had no usable pivot.", sealed: true },
    Refusal { name: "NotSettled", meaning: "Newton's method had not settled after 30 iterations of one step.", sealed: true },
    Refusal {
        name: "EnergyLedgerBroken",
        meaning: "A step's energy account was open by more than 1e-9 of its scale, or the run's by more than 1e-9 of the energy deposited.",
        sealed: true,
    },
];

fn field_words() -> String {
    format!(
        "SHA-384 of the count of the numbers that follow as 8 little-endian bytes, which is twice the {} cells and the clad's surface node ({} numbers), then the radius of every cell centre from the axis out and of the clad's surface, m, then the temperature there at the history's end, K, all as little-endian f64 bytes.",
        rod::ROD_CELLS + rod::MEAT_CELLS + rod::CLAD_CELLS,
        2 * (rod::ROD_CELLS + rod::MEAT_CELLS + rod::CLAD_CELLS + 1)
    )
}

fn sections() -> Sections {
    Sections {
        locked: json!({
            "equations": "Radial conduction in each layer, rho c(T) dT/dt = (1/r) d/dr (k r dT/dr) + q''', from the axis out: the zirconium rod, the U-ZrH1.6 meat (the only layer heated), a gap of conductance h_gap on the meat's surface, the SS-304 clad; at the clad's surface the heat flux to the pool is natural convection (Churchill and Chu, vertical surface, water at 300 K) plus nucleate boiling above 385 K (Rohsenow, C_sf 0.013, n 1); the pool stays at the start temperature. The thermocouple: dT_tc/dt = (T_meat(r_tc) - T_tc)/tau.",
            "heat_source": "q''' = 1.387 P_core / (56 V_meat), spread evenly over the meat. P_core over each interval of the history is the core energy released in it over its length, so the element receives 1.387/56 of the energy given, however coarse the history. 1.387 is the hot-rod factor of ring B of a core loaded uniformly with 12 wt% fuel (I. Vavtar, master thesis, University of Ljubljana, 2019, Table 4), taken as element 6753's own at B1; 56 is the core's elements. The thermocouple's plane carries the element's average power density: the axial peak at mid-height and the radial dip towards the axis are folded into the fitted gap conductance.",
            "rod_radius_m": rod::ROD_RADIUS_M,
            "meat_radius_m": rod::MEAT_RADIUS_M,
            "gap_outer_radius_m": rod::GAP_RADIUS_M,
            "clad_radius_m": rod::CLAD_RADIUS_M,
            "meat_length_m": rod::MEAT_LENGTH_M,
            "thermocouple_radius_m": rod::THERMOCOUPLE_RADIUS_M,
            "geometry_source": "Radii: Svajger et al., Nucl. Eng. Technol. 56 (2024) 1165, Table 1. Meat length 15 in: M. Ravnik, Description of TRIGA Reactor, Table 4. Thermocouple radius 0.3 in: General Atomics' instrumented element (University of Texas NETL TRIGA safety analysis report, 1991, section 4.4.5.1).",
            "meat_density_kg_per_m3": rod::MEAT_DENSITY_KG_PER_M3,
            "meat_heat_capacity_j_per_kg_k": "333 + 0.678 (T - 274.15): Svajger et al. 2024, eq. 7.2, as in the reactor_triga_jsi card.",
            "meat_conductivity_w_per_m_k": rod::MEAT_CONDUCTIVITY_W_PER_M_K,
            "meat_conductivity_source": "13 Btu per h ft degF, NETL TRIGA safety analysis report, Table 4-1 (delta-phase U-ZrH, 93 to 650 degC).",
            "rod": { "density_kg_per_m3": rod::ROD_DENSITY_KG_PER_M3, "heat_capacity_j_per_kg_k": rod::ROD_HEAT_CAPACITY_J_PER_KG_K, "conductivity_w_per_m_k": rod::ROD_CONDUCTIVITY_W_PER_M_K, "source": "density Svajger et al. 2024, Table 1; zirconium at 300 K, Incropera and DeWitt, Table A.1." },
            "clad": { "density_kg_per_m3": rod::CLAD_DENSITY_KG_PER_M3, "heat_capacity_j_per_kg_k": rod::CLAD_HEAT_CAPACITY_J_PER_KG_K, "conductivity_w_per_m_k": rod::CLAD_CONDUCTIVITY_W_PER_M_K, "source": "density Svajger et al. 2024, Table 1; SS-304 at 300 K, Incropera and DeWitt, Table A.1." },
            "element_power_factor": rod::ELEMENT_POWER_FACTOR,
            "core_elements": rod::CORE_ELEMENTS,
            "saturation_temperature_k": rod::SATURATION_TEMPERATURE_K,
            "water_side_source": "Water at 300 K and saturated water at 385 K (1.52 bar, about 5 m of water over the element): Incropera and DeWitt, Tables A.6 and 10.1.",
            "fuel_temperature_limit_k": rod::FUEL_TEMPERATURE_LIMIT_K,
            "critical_heat_flux_w_per_m2": rod::critical_heat_flux_w_per_m2(),
            "gap_conductance_w_per_m2_k": rod::GAP_CONDUCTANCE_W_PER_M2_K,
            "thermocouple_time_constant_s": rod::THERMOCOUPLE_TIME_CONSTANT_S,
            "fitted": "Two constants, fitted on 39 pulses of the institute's pulse database (cores 215, 217 and 231, 1.4 to 3.0 printed dollars) to the thermocouple's maximum rise, its time after the power peak and its rise 10 s after the peak: the gap conductance 1780 W/m2 K and the thermocouple's time constant 1.03 s. The gap conductance also carries the water side and the power density at the thermocouple's plane; the time constant every lag of the thermocouple and its recording chain.",
            "cells": { "rod": rod::ROD_CELLS, "meat": rod::MEAT_CELLS, "clad": rod::CLAD_CELLS },
            "step_max_s": rod::STEP_MAX_S,
            "grid": "Finite volumes of equal width in each layer, the temperature at each cell's mid-radius, a surface node of no heat capacity; the conductance between neighbours from the logarithm of their radii, the gap's in series. Each interval of the history in equal steps of at most step_max_s. Quartering the step moved the compared quantities by at most 0.003 K and doubling the cells by at most 0.03 K.",
        }),
        invariants: json!([
            {
                "name": "largest_step_energy_residual",
                "rule": "Each step: the energy stored in the element, cell by cell, against the energy deposited less the energy to the water, each from the step's own values; over the largest of the stored change, the energy deposited, the energy to the water, the cells' changes without sign and 100 J.",
                "tolerance": rod::STEP_ENERGY_TOLERANCE,
                "unit": "1",
                "held": "every step",
                "refusal": "EnergyLedgerBroken",
            },
            {
                "name": "run_energy_residual",
                "rule": "At the end: the energy stored over the start against the energy deposited less the energy to the water, summed over the steps, over the energy deposited (at least 100 J).",
                "tolerance": rod::RUN_ENERGY_TOLERANCE,
                "unit": "1",
                "held": "at the end",
                "refusal": "EnergyLedgerBroken",
            },
        ]),
        convergence: json!({
            "rule": "Backward Euler in the enthalpy, Newton's method on the tridiagonal system of the cells and the surface node; a step is accepted when every equation holds to 1e-13 of the step's energy scale or to 1e-8 J, whichever is larger.",
            "imbalance_tolerance": rod::NEWTON_IMBALANCE_TOLERANCE,
            "imbalance_floor_j": rod::NEWTON_IMBALANCE_FLOOR_J,
            "max_newton_iterations": rod::MAX_NEWTON_ITERATIONS,
            "refusal": "NotSettled, SingularSystem or NonFiniteValue",
        }),
        reference: json!({
            "name": "Svajger, Calic, Pungercic, Trkov, Snoj, Evaluation of reactor pulse experiments, Nucl. Eng. Technol. 56 (2024) 1165, eq. 7.6 (adiabatic heating, E = m integral of c dT), and the closed form of steady conduction through the layers",
            "url": "https://doi.org/10.1016/j.net.2023.11.021",
            "retrieved": "2026-10-09",
            "test": "adiabatic_meat_against_the_article",
            "covers": "With the gap closed to 1e-12 W/m2 K, a pulse of 12 MW s: the element receives 1.387/56 of it to 1e-12, the meat and rod keep it to 1e-9, and the outer meat is within 0.05 K of the temperature the article's relation gives. A second test holds steady conduction to the closed form of the layers within 2 percent of the rise at the thermocouple.",
            "kind": "A published relation and a closed form. The comparison with the thermocouple of element 6753 is in validation.",
        }),
    }
}

/// The record in the five parts: the object, the data, how it is scored, the
/// result in numbers and the limits.
fn validation() -> [String; 5] {
    let object = "The instrumented standard fuel element 6753 at position B1 (ring B) of the 56-element core of the TRIGA Mark II research reactor of the Jozef Stefan Institute, Ljubljana, cores 215 (November 2014), 217 (March 2015) and 231 (March and April 2018): U-ZrH1.6 meat, 12 wt% uranium enriched to 20 percent, around a zirconium rod, a gas gap, an SS-304 clad, in the pool; its thermocouple, channel T1 of the pulse records, in the meat.".to_string();
    let data = "The JSI TRIGA Mark II pulse database, https://trigapulse.ijs.si/ (Ingrid Svajger, nee Vavtar, Luka Snoj and the reactor's operators, Reactor Physics Department, Jozef Stefan Institute); published account: Svajger et al., Nucl. Eng. Technol. 56 (2024) 1165. Each pulse: power, T1, T2 and rod position at 20 kHz for 10 or 20 s. The site states no licence; the article is CC BY-NC-ND 4.0. The engine stores no measured value: two constants are fitted to the records. Fitted on 39 pulses of cores 215, 217 and 231, 1.4 to 3.0 printed dollars; compared with the 19 pulses of cores 217 and 231 at 2.0 to 2.6 printed dollars left out of the fit, each fetched and its start temperature read from its own record; 16 were fetched whole and compared. The history given is the record's power summed from its first sample (every 10 ms, every 1 ms about the peak), to 11 s after the power peak.".to_string();
    let scoring = "Three quantities per pulse, each against the measured T1 (its 0.1 s running mean): the maximum rise above the start, within 10 percent; the time of that maximum after the power peak, within 20 percent; the rise 10 s after the peak, within 10 percent, where the record runs that long. Beside it a lumped baseline: the reactor_triga_jsi card's core-average fuel temperature through the hot-rod factor 1.387 and a fitted lag of 0.77 s, on the same history. Two upstreams: the record's own power (the element alone), and the reactor_triga_jsi card run on the pulse, its energy handed through the coupling triga_pulse_to_element_6753.".to_string();
    let limits = "Fitted: the gap conductance, 1780 W/m2 K, and the thermocouple's time constant, 1.03 s, on 39 pulses at 1.4 to 3.0 printed dollars. Compared span: start 293.14 to 305.53 K, core energy released 6.85 to 19.41 MW s, core power to 994 MW over an interval, histories in 1 ms and 10 ms intervals to 14 s; outside it the same equations answer and the answer carries start_temperature_outside_compared, core_energy_outside_compared, core_power_outside_compared or history_outside_compared. One element at one position, B1: the power density at the thermocouple's plane is the element's mean, the axial peak and the radial dip folded into the fitted gap conductance, so another element or another core loading needs its own power factor. On all 55 pulses, the 39 fitted and the 16 compared, the 2014 and 2015 cores (215, 217) read 6.3 and 7.7 percent low on the maximum rise at the mean and the 2018 core (231) 2.2; refitted on the 55 the pair is the same, a gap conductance per core leaves core 217 at 7.4 percent low, and 8 to 10 percent more of the core's energy in the element closes it (record analysis/06_refit_55.py and 06b_power_share_probe.py). The pool stays at the start temperature; the meat's conductivity is one number and its heat capacity linear in temperature. It refuses where its equations stop holding: the meat above 1150 degC (FuelAboveSafetyLimit), a clad heat flux above Zuber's critical heat flux of 1.47 MW/m2 (CriticalHeatFluxPassed), a pool that is not liquid at the start. A proof shows that an answer was not changed, not that the model matches the element.".to_string();
    [object, data, scoring, RESULT.to_string(), limits]
}

/// The comparison with the 19 pulses left out of the fit, in numbers.
pub(crate) const RESULT: &str = "Compared with the pulses of cores 217 and 231 at 2.0 to 2.6 printed dollars left out of the fit, from each record's own power: 11 pulses (224 to 231, 324, 325, 338, started 294.06 to 297.11 K), and 5 more started 293.15 to 298.47 K (315, 320 to 323). The thermocouple's maximum rise: within 10 percent on all 16, from 9.2 percent low to 0.3 percent high, mean absolute error 5.5 percent (on the 11: 9.2 to 1.4 percent low, mean 7.0); the 2015 core-217 pulses read 8.7 to 9.2 percent low and the 2018 core-231 pulses 0.3 high to 3.0 low. The time of that maximum after the power peak: within 20 percent on all 16, from 19.6 percent early to 11.4 percent late, mean 11.9 percent. The rise 10 s after the peak, on the 8 records that run that long: within 10 percent on all 8, 4.3 to 1.5 percent low, mean 3.1 percent. Beside it, the card's lumped core-average fuel temperature through the hot-rod factor 1.387 and a fitted 0.77 s lag: closer on the maximum rise (mean 4.0 percent), further on its time (17.7) and on the 10 s rise (6.4). Through the coupling triga_pulse_to_element_6753 from the reactor_triga_jsi card run on the same pulses (its peak power 149 to 525 MW against the records' 158 to 558): maximum rise 9.8 to 4.3 percent low, time 19.9 to 4.2 percent early, 10 s rise 11.2 to 5.0 percent low (7 of 8 within 10 percent). Computed on the 16: peak fuel temperature 561 to 632 K (288 to 359 degC), peak clad surface 392.2 to 394.5 K, every step's energy account closed within 1e-10 of its scale and every run's within 2e-12. The model is closer than that baseline on the time and on the 10 s rise, and further on the maximum rise. The printed thermocouple maxima of these pulses, in the database's pulse table, had been read for the reactor_triga_jsi card before this comparison; the time of the maximum and the rise 10 s on are in no table. Every answer was sealed and verified by verify_solution, every card run by verify_tape. Pulses 336, 337 and 339 were fetched in part and are not compared.";

/// A worked pulse: 14 MW s released evenly over 20 ms from 0.3 s, a history
/// at 10 ms to 3 s, at 297 K: the fuel's peak, and the thermocouple still
/// rising at the end.
fn example() -> Value {
    let mut time_s = Vec::new();
    let mut energy = Vec::new();
    for k in 1..=300 {
        let t = k as f64 * 0.01;
        let share = (t - 0.3) / 0.02;
        let share = if share < 0.0 { 0.0 } else if share > 1.0 { 1.0 } else { share };
        time_s.push(json!(k as f64 / 100.0));
        energy.push(json!(14.0 * share));
    }
    json!({
        "start_temperature_k": 297.0,
        "time_s": time_s,
        "core_energy_released_mws": energy,
    })
}

/// The solver's own question, from the values of `parameters`.
fn question(given: &Given) -> Result<ElementQuestion, String> {
    Ok(ElementQuestion {
        start_temperature_k: given.number("start_temperature_k")?,
        time_s: given.numbers("time_s")?.to_vec(),
        core_energy_released_mws: given.numbers("core_energy_released_mws")?.to_vec(),
    })
}

/// Every limit the request shows, asked of the solver's own check before a
/// physics permit is taken. The sentence is the solver's, with its name for
/// the refusal.
fn check(given: &Given) -> Result<(), String> {
    rod::admit(&question(given)?).map_err(|reject| format!("Physical Violation: {reject} ({}).", reject.name()))
}

fn solve(given: &Given) -> Result<Solved, String> {
    Ok(solved(rod::solve(&question(given)?)))
}

/// What the solver returned, in the words every solver shares.
pub fn solved(returned: Result<ElementPulse, FuelRodReject>) -> Solved {
    let pulse = match returned {
        Ok(pulse) => pulse,
        Err(reject) => {
            let name = reject.name();
            let sealed = REFUSALS.iter().any(|refusal| refusal.name == name && refusal.sealed);
            return Solved::Refused(Refused {
                name,
                sentence: if sealed { reject.to_string() } else { format!("{reject} ({name})") },
                iterations: match reject {
                    FuelRodReject::NotSettled { iterations, .. } => Some(u64::from(iterations)),
                    _ => None,
                },
            });
        }
    };
    let one = |name: &str, value: f64| (name.to_string(), Published::Number(value));
    let many = |name: &str, values: Vec<f64>| (name.to_string(), Published::Numbers(values));
    let mut field = Vec::with_capacity(pulse.field_radius_m.len() * 2);
    field.extend_from_slice(&pulse.field_radius_m);
    field.extend_from_slice(&pulse.field_temperature_k);
    Solved::Answer(Answer {
        iterations: pulse.newton_iterations,
        result: vec![
            many("thermocouple_temperature_k", pulse.thermocouple_temperature_k),
            many("fuel_temperature_at_thermocouple_k", pulse.fuel_temperature_at_thermocouple_k),
            one("peak_thermocouple_temperature_k", pulse.peak_thermocouple_temperature_k),
            one("peak_thermocouple_time_s", pulse.peak_thermocouple_time_s),
            one("peak_fuel_temperature_k", pulse.peak_fuel_temperature_k),
            one("peak_clad_surface_temperature_k", pulse.peak_clad_surface_temperature_k),
            one("element_energy_deposited_j", pulse.element_energy_deposited_j),
            one("energy_to_water_j", pulse.energy_to_water_j),
            one("energy_stored_j", pulse.energy_stored_j),
            one("start_temperature_outside_compared", pulse.start_temperature_outside_compared),
            one("core_energy_outside_compared", pulse.core_energy_outside_compared),
            one("core_power_outside_compared", pulse.core_power_outside_compared),
            one("history_outside_compared", pulse.history_outside_compared),
        ],
        invariants: vec![
            one("largest_step_energy_residual", pulse.largest_step_energy_residual),
            one("run_energy_residual", pulse.run_energy_residual),
        ],
        field,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_published_name_is_in_the_entry() {
        let given = crate::solution::read_parameters(&FUEL_ROD_TRIGA_JSI, example().as_object().expect("an object"))
            .unwrap_or_else(|error| panic!("the example is admitted: {}", error.message()));
        let answer = match solve(&given).expect("solved") {
            Solved::Answer(answer) => answer,
            Solved::Refused(refused) => panic!("{}", refused.sentence),
        };
        assert_eq!(answer.result.len(), FUEL_ROD_TRIGA_JSI.result.len());
        for ((name, _), output) in answer.result.iter().zip(FUEL_ROD_TRIGA_JSI.result) {
            assert_eq!(name, output.name);
        }
        assert_eq!(answer.invariants.len(), FUEL_ROD_TRIGA_JSI.invariants.len());
        assert_eq!(answer.field.len(), 2 * (rod::ROD_CELLS + rod::MEAT_CELLS + rod::CLAD_CELLS + 1));
    }
}
