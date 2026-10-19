//! The entry for `blast_wave_tnt_air_twin`: the incident blast wave of a
//! real charge of TNT in still sea-level air, from its detonation products,
//! the successor of `blast_wave_tnt_air`. The numbers come from the
//! `genesis_solvers` constants, the sentence of each refusal from the
//! solver's source, and the comparison from its record
//! (`evidence/research/blast_overpressure/twin/`).

use genesis_solvers::blast_wave_tnt_air::{self as air, Burst};
use genesis_solvers::blast_wave_tnt_air_twin::{self as model, TwinQuestion, TwinReject, TwinWave};
use serde_json::{Value, json};

use crate::solvers::{
    Answer, FurtherExample, Given, Input, Kind, Output, Published, Refusal, Refused, Sections, Shape, Solved, Solver,
};

const fn number(name: &'static str, unit: &'static str, note: &'static str) -> Output {
    Output { name, unit, shape: Shape::Number, each: &[], note, asked_by: None }
}

const fn numbers(name: &'static str, unit: &'static str, note: &'static str) -> Output {
    Output { name, unit, shape: Shape::Numbers, each: &[], note, asked_by: None }
}

const BURSTS: &[&str] = &["free_air", "surface"];

pub const BLAST_WAVE_TNT_AIR_TWIN: Solver = Solver {
    name: "blast_wave_tnt_air",
    summary: SUMMARY,
    inputs: &[
        Input {
            name: "charge_mass_kg",
            kind: Kind::NumberAbove { above: 0.0, maximum: model::CHARGE_MASS_MAX_KG, unit: "kg" },
            refusal: Some("ChargeBeyondUniformAtmosphere"),
            bound: CHARGE_BOUND,
            only_when: None,
            asked_by_check: true,
        },
        Input {
            name: "distance_m",
            kind: Kind::NumberWithin {
                minimum: air::DISTANCE_MIN_M,
                minimum_exclusive: false,
                maximum: air::DISTANCE_MAX_M,
                maximum_exclusive: true,
                unit: "m",
            },
            refusal: Some("DistanceBeyondUniformAtmosphere"),
            bound: DISTANCE_BOUND,
            only_when: None,
            asked_by_check: true,
        },
        Input {
            name: "burst",
            kind: Kind::Word { allowed: BURSTS },
            refusal: Some("UnknownBurst"),
            bound: BURST_BOUND,
            only_when: None,
            asked_by_check: false,
        },
    ],
    kept_inside: "The air (ISA sea level), TNT's density and JWL constants, the two fitted constants, the grids and the tolerances stay inside the solver.",
    result: &[
        number("scaled_distance_m_per_kg13", "m/kg^(1/3)", "Z = R/W^(1/3), Hopkinson-Cranz scaled distance, W the charge asked."),
        number("free_air_charge_kg", "kg", "The free-air sphere of TNT the burst is taken as: W in free air, the ground factor times W on the ground."),
        number("charge_radius_m", "m", "The radius of that sphere at 1,630 kg/m^3."),
        number("arrival_time_s", "s", "Time of arrival of the shock at the point after the detonation (half-height of its rise)."),
        number("peak_overpressure_pa", "Pa", "The incident (side-on) peak overpressure: the Rankine-Hugoniot jump of the shock's Mach number in the air."),
        number("shock_mach_number", "1", "The shock's speed over the ambient sound speed, 340.29 m/s, at the point."),
        number("positive_phase_duration_s", "s", "From the arrival to the first fall of the overpressure to zero."),
        number("positive_impulse_pa_s", "Pa s", "The overpressure history integrated over the positive phase."),
        number("friedlander_decay", "1", "a in p = Ps (1 - tau/td) exp(-a tau/td), the Friedlander form whose integral is the impulse."),
        number("scaled_arrival_time_ms_per_kg13", "ms/kg^(1/3)", "The arrival time over W^(1/3)."),
        number("scaled_positive_phase_duration_ms_per_kg13", "ms/kg^(1/3)", "The duration over W^(1/3)."),
        number("scaled_positive_impulse_kpa_ms_per_kg13", "kPa ms/kg^(1/3)", "The impulse over W^(1/3)."),
        number("products_radius_max_m", "m", "The farthest the face of the detonation products and the air reached in the run: the fireball's 1D reach, at most 0.8402 m for each kg^(1/3) of the free-air charge, reached when the run lasts long enough."),
        number("euler_reach_m_per_kg13", "m/kg^(1/3)", "The farthest scaled distance the Euler equations carried the wave: the point's own, or Sachs distance 4 of the free-air charge (Z = 13.8 in free air), beyond which weak-shock theory carried it."),
        number("shock_radius_at_end_m", "m", "How far from the charge the shock is when the positive phase at the point ends: the air the answer rests on. At most 843.45 m, a tenth of the scale height (WaveBeyondUniformAtmosphere)."),
        number("shock_rise_time_s", "s", "The shock's thermoviscous rise time at the point, Taylor's 2 rho0 delta/(beta Ps); where it lowers the peak by more than 2.5 percent the answer is refused (ShockNotSharp)."),
        number("scaled_distance_outside_compared", "1", "1 where Z is outside 1 to 14.95 m/kg^(1/3), the range of Dewey's tables it was fitted on and compared with, 0 inside it. The answer is the same arithmetic either way."),
        numbers("history_time_after_arrival_s", "s", "33 times from the arrival to the end of the positive phase, evenly spaced."),
        numbers("history_overpressure_pa", "Pa", "The computed overpressure at the point at those times. In the Euler range the shock's jump is spread over a few cells, so the first values rise to the peak; in the weak-shock range the jump is sharp."),
        numbers("friedlander_overpressure_pa", "Pa", "The Friedlander form at those times, from the peak, the duration and the decay."),
    ],
    invariants: &[
        number("energy_ledger_residual", "1", ""),
        number("impulse_closure_residual", "1", ""),
        number("weak_shock_area_residual", "1", ""),
        number("grid_change_peak", "1", ""),
        number("grid_change_impulse", "1", ""),
        number("grid_change_duration", "1", ""),
        number("grid_change_arrival", "1", ""),
    ],
    iterations: "Time steps of the two Euler runs together, the coarse grid's and the fine grid's.",
    field: field_words,
    answer: "a blast wave",
    refusals: REFUSALS,
    door_refusals: &[Refusal { name: "UnknownBurst", meaning: "The burst is neither free_air nor surface.", sealed: false }],
    sections,
    validation: Some(validation),
    example,
    further_examples: &[
        FurtherExample { key: "example_solve_surface_burst", parameters: surface_example },
        FurtherExample { key: "example_solve_acoustic_far_field", parameters: far_example },
    ],
    parts: &[],
    check,
    solve,
};

const SUMMARY: &str = "The incident blast wave of a charge of cast TNT in still sea-level air, a sphere in free air or a hemisphere on the ground, at a point outside its fireball up to 843.45 m from it: arrival time, peak overpressure, positive-phase duration and impulse, the Friedlander form and the pressure history, and the fireball's reach. The charge is a sphere of TNT's detonation products, its pressure the JWL form of Dobratz and Crawford (1985), detonated through its volume; the spherical Euler equations carry products and air together to Sachs distance 4 and weak-shock theory beyond, the energy of products and air held to the products' at every step. Two constants fitted on Dewey's TNT tables: the products' energy, 1.234 times the JWL E0 (4.542 MJ/kg), and the ground factor, a surface burst of W taken as a free-air burst of 1.825 W. Against Dewey's free-air table from Z 1 to 14.94 m/kg^(1/3) the peak is within 6.6 percent, and the impulse and duration within 4.3 percent from Z 1.75 out; against the surface-burst positive-phase table, kept out of the fit, the impulse within 3.9 percent and the duration within 8.0 from Z 1.75 out. Within the fireball, Z 0.8402 in free air and 1.0268 on the ground, it is refused by name.";
const CHARGE_BOUND: &str = "Mass of the charge in kilograms of cast TNT, a sphere at 1,630 kg/m^3 in free air or a hemisphere on the ground. Above zero (ChargeMassNotPositive) and at most 194,741,349 kg in free air, 106,707,589 kg on the ground (ChargeBeyondUniformAtmosphere): above it, at every point, the shock is beyond a tenth of the air's scale height, 843.45 m, when the positive phase there ends. For another explosive, its TNT equivalence, which differs by quantity and distance, is the caller's.";
const DISTANCE_BOUND: &str = "Distance of the point from the charge's centre: from 6.37e-6 m, 100 mean free paths of the air (DistanceBelowContinuum; zero or less, DistanceNotPositive), to below 843.45 m, a tenth of the air's scale height (DistanceBeyondUniformAtmosphere). The point must be outside the fireball, the farthest reach of the detonation products, Z = 0.8402 m/kg^(1/3) in free air and 1.0268 on the ground (PointInsideFireball). Compared from Z 1 to 14.94 in free air and 1.1 to 40 on the ground; outside 1 to 14.95 the answer carries scaled_distance_outside_compared. Beyond Sachs distance 4, Z = 13.83 in free air and 16.90 on the ground, weak-shock theory carries the wave: a regime of the same wave. The point is answered where the shock's radius when its positive phase ends is within 843.45 m (WaveBeyondUniformAtmosphere), and where the shock is sharp against that phase (ShockNotSharp; a gram at 840 m is not).";
const BURST_BOUND: &str = "`free_air`: a spherical charge, the point anywhere around it. `surface`: a hemispherical charge resting on flat ground, the point on or above the ground, taken as a free-air sphere of 1.825 times the charge (the ground factor, fitted on Dewey's surface-burst peaks; rigid ground would be 2).";

/// Each `TwinReject` variant with the sentence above it in the solver's
/// source. A refusal the request itself shows is answered at the door.
const REFUSALS: &[Refusal] = &[
    Refusal { name: "InputNotFinite", meaning: "An input is not a finite number.", sealed: false },
    Refusal { name: "ChargeMassNotPositive", meaning: "The charge mass is zero or less: there is no charge.", sealed: false },
    Refusal { name: "DistanceNotPositive", meaning: "The distance is zero or less.", sealed: false },
    Refusal { name: "ChargeBeyondUniformAtmosphere", meaning: "The charge is above 194,741,349 kg of TNT in free air, or 106,707,589 kg on the ground: at every point of it the shock is beyond a tenth of the air's scale height, 843.45 m, when the positive phase there ends, that radius being least, 0.421 Sachs lengths, at the fireball's edge.", sealed: false },
    Refusal { name: "DistanceBeyondUniformAtmosphere", meaning: "The distance is a tenth of the air's scale height, 843.45 m, or beyond, where the ambient pressure differs from the charge's by 1 - e^-0.1 = 9.52 percent or more: the shock is past it before the positive phase at the point ends.", sealed: false },
    Refusal { name: "DistanceBelowContinuum", meaning: "The distance is under 100 mean free paths of the air, 6.37e-6 m: the mean free path over the distance, the Knudsen number, passes 0.01, the edge of the continuum, where the Euler equations of a gas stop holding.", sealed: false },
    Refusal { name: "PointInsideFireball", meaning: "The point is within the farthest reach of the detonation products, 0.8402 m/kg^(1/3) of the free-air charge: there products and air mix, their face being unstable as it slows, and the sharp face the model carries between them is not the real one.", sealed: false },
    Refusal { name: "WaveBeyondUniformAtmosphere", meaning: "When the positive phase at the point ends, the shock is beyond a tenth of the air's scale height, 843.45 m, from the charge: the air the answer rests on reaches where the ambient pressure differs from the charge's by more than 1 - e^-0.1 = 9.52 percent, and the model's air is uniform.", sealed: true },
    Refusal { name: "ShockNotSharp", meaning: "The shock's thermoviscous rise time, Taylor's 2 rho0 delta/(beta Ps), lowers the peak of the positive phase it heads by more than 2.5 percent, the peak's own settling line: the shock is not sharp against its phase, and the inviscid wave the model carries is not the air's.", sealed: true },
    Refusal { name: "NonFiniteValue", meaning: "A value of the flow was not finite.", sealed: true },
    Refusal { name: "InternalEnergyNotPositive", meaning: "A cell's internal energy fell to zero or below.", sealed: true },
    Refusal { name: "ProductsSoundSpeedNotReal", meaning: "A cell of the products has no real sound speed in the JWL equation of state.", sealed: true },
    Refusal { name: "CellInverted", meaning: "A cell's outer face crossed its inner face.", sealed: true },
    Refusal { name: "EnergyLedgerBroken", meaning: "The energy of products and air above ambient departed from the products' energy at detonation by more than 1e-10 of it at a step.", sealed: true },
    Refusal { name: "StepCapReached", meaning: "The Euler run reached 400,000 steps before the positive phase at the point ended.", sealed: true },
    Refusal { name: "ImpulseClosureBroken", meaning: "The Friedlander form's integral departs from the impulse by more than 1e-12 of it.", sealed: true },
    Refusal { name: "AreaNotCarried", meaning: "Weak-shock theory changed r times the impulse by more than 1e-12 of it.", sealed: true },
    Refusal { name: "NotSettled", meaning: "The coarse and fine grids disagree by more than 2.5 percent in the peak, 1 percent in the impulse, 1.5 percent in the duration or 0.5 percent in the arrival time.", sealed: true },
];

fn field_words() -> String {
    format!(
        "SHA-256 of the count of the numbers that follow ({}) as 8 little-endian bytes, then the {} times after arrival of the history in seconds, then its {} overpressures in pascals, all as little-endian f64 bytes.",
        2 * model::HISTORY_POINTS,
        model::HISTORY_POINTS,
        model::HISTORY_POINTS
    )
}

fn sections() -> Sections {
    Sections {
        locked: json!({
            "air": "The International Standard Atmosphere at sea level, still and uniform: an ideal gas of ratio of specific heats 1.4.",
            "ambient_pressure_pa": air::AMBIENT_PRESSURE_PA,
            "ambient_temperature_k": air::AMBIENT_TEMPERATURE_K,
            "gamma": air::GAMMA,
            "ambient_density_kg_per_m3": air::ambient_density_kg_per_m3(),
            "sound_speed_m_per_s": air::sound_speed_m_per_s(),
            "charge": "Cast TNT at 1,630 kg/m^3, a sphere in free air; detonated at one instant through its volume, its products at rest at the charge's density with the JWL detonation energy E0 times the fitted products_energy_factor.",
            "tnt_density_kg_per_m3": model::TNT_DENSITY_KG_PER_M3,
            "jwl": "p = A (1 - w/(R1 V)) exp(-R1 V) + B (1 - w/(R2 V)) exp(-R2 V) + w rho e, V = rho0/rho: TNT's Jones-Wilkins-Lee equation of state of the detonation products, B. M. Dobratz and P. C. Crawford, LLNL Explosives Handbook, UCRL-52997 Change 2 (1985). Its Chapman-Jouguet state, 21.0 GPa at 6,930 m/s, lies on it within 0.3 percent (a test).",
            "jwl_a_pa": model::JWL_A_PA,
            "jwl_b_pa": model::JWL_B_PA,
            "jwl_r1": model::JWL_R1,
            "jwl_r2": model::JWL_R2,
            "jwl_omega": model::JWL_OMEGA,
            "jwl_e0_j_per_m3": model::JWL_E0_J_PER_M3,
            "tnt_detonation_velocity_m_per_s": model::TNT_DETONATION_VELOCITY_M_PER_S,
            "tnt_cj_pressure_pa": model::TNT_CJ_PRESSURE_PA,
            "products_energy_factor": model::PRODUCTS_ENERGY_FACTOR,
            "products_energy_j_per_kg": model::PRODUCTS_ENERGY_FACTOR * model::JWL_E0_J_PER_M3 / model::TNT_DENSITY_KG_PER_M3,
            "ground_factor": model::GROUND_FACTOR,
            "surface_burst": "A hemisphere of W on the ground is taken as a free-air sphere of ground_factor W: 2 is rigid ground, the plane a mirror; the fitted factor holds what real ground takes away.",
            "euler": "From the instant of detonation the spherical Euler equations of products and air together: Lagrangian finite volumes in the radius, the products and the air cells of their own materials with a face between them that moves with the flow, limited linear reconstruction of pressure and velocity, two-stage strong-stability-preserving Runge-Kutta, a two-shock acoustic Riemann solver at every node, face areas averaged over the step so a node's swept volume is exact. Cells are added at ambient ahead of the wave; pairs of air cells far behind the shock, or rarefied below the ambient density, are joined, mass, momentum and energy summed; the products' cells are never joined.",
            "peak": "The Rankine-Hugoniot jump 2 gamma (M^2 - 1)/(gamma + 1) p0 of the shock's Mach number M in the air, from a least-squares quadratic of arrival time in radius over seven gauges within 5 percent of the point.",
            "weak_shock": "Beyond Sachs distance 4 of the free-air charge's 4.184 MJ/kg the Friedlander form of the positive phase there is carried out by weak-shock theory (Whitham's equal-area rule).",
            "weak_shock_from_sachs_distance": model::WEAK_SHOCK_FROM_SACHS_DISTANCE,
            "fireball_radius_m_per_kg13": model::FIREBALL_RADIUS_M_PER_KG13,
            "fireball": "The products' face with the air runs out to 0.8402 m for each kg^(1/3) of the free-air charge and turns back. As it slows, the heavier products pushing the lighter hot air, the face is Rayleigh-Taylor unstable and products and air mix there, and the products' carbon monoxide and carbon burn with the air's oxygen; a model in the radius alone carries a sharp face between them. A point within that radius is refused (PointInsideFireball).",
            "uniform_air": "One criterion holds the model's uniform air: the shock's radius when the positive phase at the point ends, at most a tenth of the scale height, 843.45 m. That radius is least at the fireball's edge, 0.421 Sachs lengths of the free-air charge at 4.184 MJ/kg (Sachs distance 0.2431), and rises outward, so a charge above 194,741,349 kg in free air (106,707,589 on the ground) is refused at the door.",
            "least_shock_reach_sachs": model::LEAST_SHOCK_REACH_SACHS,
            "least_reach_sachs_distance": model::LEAST_REACH_SACHS_DISTANCE,
            "charge_mass_max_kg": model::CHARGE_MASS_MAX_KG,
            "distance_min_m": air::DISTANCE_MIN_M,
            "distance_max_m": air::DISTANCE_MAX_M,
            "fine_cell_sachs": model::FINE_CELL_SACHS,
            "fine_cells_across_distance": model::FINE_CELLS_ACROSS_DISTANCE,
            "fine_products_cells": model::FINE_PRODUCTS_CELLS,
            "coarse_factor": model::COARSE_FACTOR,
            "courant_number": model::COURANT_NUMBER,
            "history_points": model::HISTORY_POINTS,
            "fitted": "Two constants, on 1 kg of TNT, least squares in the log of model over table. products_energy_factor, 1.234: on Dewey's free-air interface (TNTFFI 1.1c), its peaks at 15 Z from 1 to 14.94 m/kg^(1/3) and its impulses and durations at 14 Z from 1.31 to 14.94. ground_factor, 1.825: on Dewey's surface-burst interface's peaks (TNTSBI 1.0c) at 16 Z from 1.1 to 14.94, those outside the fireball, with the energy factor fixed. Kept out: the surface-burst interface's positive-phase table, every arrival, and Kingery-Bulmash.",
        }),
        invariants: json!([
            {
                "name": "energy_ledger_residual",
                "rule": "The energy of the products, kinetic and internal, plus the energy of the air above ambient, over the whole domain, equals the products' energy at the instant of detonation at every step: the largest departure over the fine grid's run, as a fraction of it. The fluxes telescope, the face between products and air is a node like any other, and the cells added ahead are at ambient, so it closes to rounding, about 1e-14.",
                "tolerance": model::ENERGY_LEDGER_TOLERANCE,
                "unit": "fraction of the products' energy",
                "held": "every step of both runs",
                "refusal": "EnergyLedgerBroken",
            },
            {
                "name": "impulse_closure_residual",
                "rule": "The Friedlander form's integral, Ps td (1/a - (1 - exp(-a))/a^2), equals the impulse: a is solved by bisection to adjacent 64-bit numbers.",
                "tolerance": model::IMPULSE_CLOSURE_TOLERANCE,
                "unit": "fraction of the impulse",
                "held": "at acceptance",
                "refusal": "ImpulseClosureBroken",
            },
            {
                "name": "weak_shock_area_residual",
                "rule": "Through the weak-shock regime the area of r p' over the positive phase, r times the impulse, is carried unchanged. Zero where the Euler run reached the point.",
                "tolerance": model::AREA_TOLERANCE,
                "unit": "fraction of r times the impulse",
                "held": "at acceptance",
                "refusal": "AreaNotCarried",
            },
            {
                "name": "grid_change_peak",
                "rule": "The change of the peak overpressure from the coarse grid to the fine.",
                "tolerance": model::SETTLE_PEAK,
                "unit": "fraction",
                "held": "at acceptance",
                "refusal": "NotSettled",
            },
            {
                "name": "grid_change_impulse",
                "rule": "The change of the impulse from the coarse grid to the fine.",
                "tolerance": model::SETTLE_IMPULSE,
                "unit": "fraction",
                "held": "at acceptance",
                "refusal": "NotSettled",
            },
            {
                "name": "grid_change_duration",
                "rule": "The change of the positive-phase duration from the coarse grid to the fine.",
                "tolerance": model::SETTLE_DURATION,
                "unit": "fraction",
                "held": "at acceptance",
                "refusal": "NotSettled",
            },
            {
                "name": "grid_change_arrival",
                "rule": "The change of the arrival time from the coarse grid to the fine.",
                "tolerance": model::SETTLE_ARRIVAL,
                "unit": "fraction",
                "held": "at acceptance",
                "refusal": "NotSettled",
            },
        ]),
        convergence: json!({
            "rule": "The whole wave is computed on two grids: the fine one's air cell the smaller of 0.00125 Sachs lengths and the point's distance over 800 (never under the charge's radius over 64), at least 64 cells across the charge, the coarse one's twice as wide with half as many. The fine grid's values are returned when the two agree within the grid_change tolerances, every step held the energy ledger, and the Euler run ended the positive phase at the point within 400,000 steps. From the fireball's edge to Z 100 in free air the changes are at most about 0.7 percent in the peak, 0.45 in the impulse, 0.9 in the duration and 0.2 in the arrival.",
            "refusal": "NotSettled, StepCapReached, EnergyLedgerBroken, InternalEnergyNotPositive, ProductsSoundSpeedNotReal, CellInverted or NonFiniteValue; once settled, WaveBeyondUniformAtmosphere or ShockNotSharp",
            "run_time": "3 to 12 s for one solve on one core of an Apple M4 Pro in a release build, measured with the machine under heavy load (load average near 100): about 3 s from Z 2 to 10, 6 to 12 s from Z 12 out, up to 45 s at the fireball's edge under that load.",
        }),
        reference: json!({
            "name": model::REFERENCE_NAME,
            "url": model::REFERENCE_URL,
            "retrieved": model::REFERENCE_RETRIEVED,
            "test": "dewey_tnt_free_field_fit",
            "covers": "1 kg of TNT in free air at Z = 2, 5 and 10 m/kg^(1/3), the table the energy factor was fitted on: the peak -0.7, +0.3 and +0.8 percent from Dewey's, the impulse -1.6, -2.1 and -0.8, the duration -0.4, -3.0 and -1.1, the arrival (not fitted) +0.9, +0.4 and +0.2, each held to half a percentage point. dewey_tnt_surface_burst_positive_phase_left_out holds the surface burst against the surface-burst table's impulse and duration, kept out of the fit (impulse +0.8, -3.6 and -3.5 percent at Z 2, 5 and 10; duration +3.1, -1.8 and -2.8), and kingery_bulmash_surface_burst_left_out against Kingery-Bulmash as Swisdak simplified it and fcento100 coded it (peak within 8 percent from Z 2 to 40, the impulse 16 to 23 percent below and the duration 15 to 37 percent below it).",
            "kind": "Measurement-derived tables of TNT explosions (Dewey and McMillin's particle-trajectory analyses of 4 to 500 kg shots), the free-air one and the surface-burst one; and a published correlation of TNT surface bursts.",
        }),
    }
}

fn validation() -> [String; 5] {
    let object = "TNT, cast at 1,630 kg/m^3, detonated as a sphere in free air or a hemisphere on the ground in still air near sea level: the incident blast wave as J. M. Dewey's interfaces give it for 1 kg, from Dewey and McMillin's particle-trajectory analyses of TNT shots of 4 to 500 kg (Department of Physics and Astronomy, University of Victoria). They are measurement-derived tables, scaled by Hopkinson-Cranz to 1 kg at 101.3 kPa and 15 degC.".to_string();
    let data = "Dewey (2021), TNT Free-Field Blast Wave Properties Interface 1.1c, Borealis doi 10.5683/SP2/WB8JKO, and Dewey (2022), TNT Surface-Burst Blast Wave Interface 1.0c, doi 10.5683/SP3/CFPFRH, both CC0 1.0: peak overpressure and arrival at radii from 1 to 14.95 m, and modified-Friedlander coefficients (peak, decay, positive duration) as cubics in ln R from 1.31 to 14.94 m, the impulse their integral. Beside them Kingery and Bulmash's hemispherical surface burst (ARBRL-TR-02555, 1984) as Swisdak simplified it (1994) and fcento100 coded it (MIT), not compared with either original. Fitted on the free-air peaks at 15 Z from 1 to 14.94 and its impulses and durations at 14 Z from 1.31 to 14.94 (one constant), and on the surface-burst peaks at 16 Z from 1.1 to 14.94 (a second). Kept out of the fit: the surface-burst positive-phase table, 17 Z from 1.1 to 14.94, every arrival, Kingery-Bulmash from Z 1.1 to 40, and the peaks at 5 more Z in free air and 3 on the ground.".to_string();
    let scoring = "Per quantity, the solver's value over the table's, in percent, at each Z; the root mean square and the largest.".to_string();
    [object, data, scoring, RESULT.to_string(), LIMITS.to_string()]
}

const RESULT: &str = "Free air against Dewey's TNTFFI, Z 1 to 14.94 m/kg^(1/3), 20 points: the peak within 6.6 percent (-6.6 at 1.31, +4.2 at 1; RMS 2.6 on the 15 fitted, 2.4 on the 5 not fitted); the impulse within 4.0 percent and the duration within 4.3 from Z 1.75 out, at 1.5 +10.1 and +8.3, at 1.31 +25.3 and +22.2; the arrival, not fitted, within 2.0 percent at every point. On the ground against the TNTSBI peaks, Z 1.1 to 14.94: within 6.6 percent on the 16 fitted (RMS 3.2; +6.6 at 1.2, within 3.9 from 1.31 out), within 6.2 on the 3 not. Kept out of the fit, the TNTSBI positive phase, 17 points: the impulse within 3.9 percent and the duration within 8.0 from Z 1.75 out (within 3.2 from 2 out), at 1.5 +10.2 and +20.8, at 1.31 +19.3 and +41.7; the arrival 0.3 to 5.3 percent late. Kingery-Bulmash's surface burst, kept out, Z 1.75 to 40: the peak within 7.8 percent (+24 at Z 1.1, +18.8 at 100 and +42 at 198.5, where its line falls as Z^-1.41), the impulse 15 to 27 percent below and the duration 14 to 61 percent below it, the arrival within 2.7 percent from Z 3 out; Kingery-Bulmash's own impulses and durations stand 14 to 63 and 14 to 255 percent above Dewey's. The fitted energy, 4.542 MJ/kg, sits by TNT's measured heat of detonation; the ground factor, 1.825, by the 1.8 the surface-burst literature gives.";

const LIMITS: &str = "Fitted constants: two, the products' energy (1.234 times the JWL E0) and the ground factor (1.825), on Dewey's tables for 1 kg; every other constant is TNT's (Dobratz and Crawford 1985), the air's, or a numerical choice held to the solver's own grid refinement. The charge is detonated at one instant through its volume, so the near field of a charge detonated from a point and its shape are outside it, and so is a charge of another explosive except as the caller's TNT equivalence. The air is an ideal gas of gamma 1.4 throughout: the air the products first shock is hotter than real air there, and the fitted energy holds that. The positive phase near the fireball is long: at Z 1.31 the impulse is 25 percent and the duration 22 percent above Dewey's free-air table (19 and 42 percent on the ground), where Dewey's own Friedlander fits begin, 1.31; the second shock of the products, which a real fireball's mixing spreads, ends the phase late there. Within the fireball, Z 0.8402 in free air and 1.0268 on the ground, it is refused. The ground factor is one number for every Z and every soil. The answer is the incident (side-on) wave of the positive phase in still, uniform air; a surface the wave strikes, wind and temperature gradients change it, and the negative phase follows it. Far out the weak shock's peak falls about 2 percent faster than the Euler equations carry it. A proof shows that an answer was not changed, not that the model matches a charge.";

fn example() -> Value {
    json!({ "charge_mass_kg": 1.0, "distance_m": 5.0, "burst": "free_air" })
}

fn surface_example() -> Value {
    json!({ "charge_mass_kg": 100.0, "distance_m": 30.0, "burst": "surface" })
}

fn far_example() -> Value {
    json!({ "charge_mass_kg": 1.0, "distance_m": 200.0, "burst": "free_air" })
}

/// The solver's own question, from the values of `parameters`.
fn question(given: &Given) -> Result<TwinQuestion, String> {
    let burst = match given.word("burst")? {
        "surface" => Burst::Surface,
        _ => Burst::FreeAir,
    };
    Ok(TwinQuestion { charge_mass_kg: given.number("charge_mass_kg")?, distance_m: given.number("distance_m")?, burst })
}

/// Every limit the request shows, asked of the solver's own `admit` before
/// a physics permit is taken. The sentence is the solver's, with its name.
fn check(given: &Given) -> Result<(), String> {
    model::admit(&question(given)?).map_err(|reject| format!("Physical Violation: {reject} ({}).", reject.name()))
}

fn solve(given: &Given) -> Result<Solved, String> {
    Ok(solved(model::solve(&question(given)?)))
}

/// What the solver returned, in the words every solver shares.
pub fn solved(returned: Result<TwinWave, TwinReject>) -> Solved {
    let wave = match returned {
        Ok(wave) => wave,
        Err(reject) => {
            let name = reject.name();
            let sealed = REFUSALS.iter().any(|refusal| refusal.name == name && refusal.sealed);
            return Solved::Refused(Refused {
                name,
                sentence: if sealed { reject.to_string() } else { format!("{reject} ({name})") },
                iterations: None,
            });
        }
    };
    let one = |name: &str, value: f64| (name.to_string(), Published::Number(value));
    let many = |name: &str, values: Vec<f64>| (name.to_string(), Published::Numbers(values));
    let mut field = Vec::with_capacity(2 * wave.history_time_after_arrival_s.len());
    field.extend_from_slice(&wave.history_time_after_arrival_s);
    field.extend_from_slice(&wave.history_overpressure_pa);
    Solved::Answer(Answer {
        iterations: wave.steps,
        result: vec![
            one("scaled_distance_m_per_kg13", wave.scaled_distance_m_per_kg13),
            one("free_air_charge_kg", wave.free_air_charge_kg),
            one("charge_radius_m", wave.charge_radius_m),
            one("arrival_time_s", wave.arrival_time_s),
            one("peak_overpressure_pa", wave.peak_overpressure_pa),
            one("shock_mach_number", wave.shock_mach_number),
            one("positive_phase_duration_s", wave.positive_phase_duration_s),
            one("positive_impulse_pa_s", wave.positive_impulse_pa_s),
            one("friedlander_decay", wave.friedlander_decay),
            one("scaled_arrival_time_ms_per_kg13", wave.scaled_arrival_time_ms_per_kg13),
            one("scaled_positive_phase_duration_ms_per_kg13", wave.scaled_positive_phase_duration_ms_per_kg13),
            one("scaled_positive_impulse_kpa_ms_per_kg13", wave.scaled_positive_impulse_kpa_ms_per_kg13),
            one("products_radius_max_m", wave.products_radius_max_m),
            one("euler_reach_m_per_kg13", wave.euler_reach_m_per_kg13),
            one("shock_radius_at_end_m", wave.shock_radius_at_end_m),
            one("shock_rise_time_s", wave.shock_rise_time_s),
            one("scaled_distance_outside_compared", if wave.scaled_distance_outside_compared { 1.0 } else { 0.0 }),
            many("history_time_after_arrival_s", wave.history_time_after_arrival_s),
            many("history_overpressure_pa", wave.history_overpressure_pa),
            many("friedlander_overpressure_pa", wave.friedlander_overpressure_pa),
        ],
        invariants: vec![
            one("energy_ledger_residual", wave.energy_ledger_residual),
            one("impulse_closure_residual", wave.impulse_closure_residual),
            one("weak_shock_area_residual", wave.weak_shock_area_residual),
            one("grid_change_peak", wave.grid_change_peak),
            one("grid_change_impulse", wave.grid_change_impulse),
            one("grid_change_duration", wave.grid_change_duration),
            one("grid_change_arrival", wave.grid_change_arrival),
        ],
        field,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::solvers::describe_value;

    #[test]
    fn every_refusal_is_the_solvers_own_sentence() {
        assert_eq!(REFUSALS.len(), model::ALL_REJECTS.len());
        for (listed, reject) in REFUSALS.iter().zip(model::ALL_REJECTS) {
            assert_eq!(listed.name, reject.name());
            assert_eq!(listed.meaning, reject.meaning(), "{}", listed.name);
        }
    }

    #[test]
    fn every_published_name_is_in_the_entry() {
        let answer = match solved(model::solve(&TwinQuestion { charge_mass_kg: 1.0, distance_m: 5.0, burst: Burst::FreeAir })) {
            Solved::Answer(answer) => answer,
            Solved::Refused(refused) => panic!("{}", refused.sentence),
        };
        assert_eq!(answer.result.len(), BLAST_WAVE_TNT_AIR_TWIN.result.len());
        for ((name, value), output) in answer.result.iter().zip(BLAST_WAVE_TNT_AIR_TWIN.result) {
            assert_eq!(name, output.name);
            assert_eq!(value.shape(), output.shape);
        }
        assert_eq!(answer.invariants.len(), BLAST_WAVE_TNT_AIR_TWIN.invariants.len());
        for ((name, _), output) in answer.invariants.iter().zip(BLAST_WAVE_TNT_AIR_TWIN.invariants) {
            assert_eq!(name, output.name);
        }
        assert_eq!(answer.field.len(), 2 * model::HISTORY_POINTS);
    }

    #[test]
    fn the_reference_is_the_one_stored_in_the_solver_test() {
        let source = include_str!("../../genesis_solvers/src/blast_wave_tnt_air_twin.rs");
        let body = describe_value(BLAST_WAVE_TNT_AIR_TWIN.name).expect("described");
        let test = body["reference"]["test"].as_str().expect("test");
        assert!(source.contains(&format!("fn {test}()")), "{test}");
        assert!(source.contains("fn dewey_tnt_surface_burst_positive_phase_left_out()"));
        assert!(source.contains("fn kingery_bulmash_surface_burst_left_out()"));
        assert_eq!(body["reference"]["url"], model::REFERENCE_URL);
    }

}
