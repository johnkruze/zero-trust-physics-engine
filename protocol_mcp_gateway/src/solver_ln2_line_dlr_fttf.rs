//! The entry for `ln2_line_dlr_fttf`: fluid hammer in the liquid-nitrogen
//! line of DLR's Fluid Transient Test Facility when its fast valve closes, a
//! named real object compared with the facility's nine published tests. The
//! numbers come from `genesis_solvers` constants.

use genesis_solvers::line_transient as line;
use genesis_solvers::ln2_line_dlr_fttf::{self as ln2, LineHammer, LineQuestion, LineReject};
use genesis_solvers::ln2_nitrogen as n2;
use serde_json::{Value, json};

use crate::solvers::{
    Answer, Given, Input, Kind, Output, Published, Refusal, Refused, Sections, Shape, Solved,
    Solver,
};

const ASK_HISTORIES: &str = "return_histories";

const fn number(name: &'static str, minimum: f64, maximum: f64, unit: &'static str, refusal: &'static str, bound: &'static str) -> Input {
    Input {
        name,
        kind: Kind::Number { minimum, maximum, unit },
        refusal: Some(refusal),
        bound,
        only_when: None,
        asked_by_check: true,
    }
}

const fn above(name: &'static str, floor: f64, maximum: f64, unit: &'static str, refusal: &'static str, bound: &'static str) -> Input {
    Input {
        name,
        kind: Kind::NumberAbove { above: floor, maximum, unit },
        refusal: Some(refusal),
        bound,
        only_when: None,
        asked_by_check: true,
    }
}

const fn one(name: &'static str, unit: &'static str, note: &'static str) -> Output {
    Output { name, unit, shape: Shape::Number, each: &[], note, asked_by: None }
}

const fn many(name: &'static str, unit: &'static str, note: &'static str) -> Output {
    Output { name, unit, shape: Shape::Numbers, each: &[], note, asked_by: None }
}

const fn history(name: &'static str, unit: &'static str, note: &'static str) -> Output {
    Output { name, unit, shape: Shape::Numbers, each: &[], note, asked_by: Some(ASK_HISTORIES) }
}

pub const LN2_LINE_DLR_FTTF: Solver = Solver {
    name: "ln2_line_dlr_fttf",
    summary: SUMMARY,
    inputs: &[
        number(
            "temperature_k",
            ln2::COLDEST_K,
            ln2::WARMEST_K,
            "K",
            "LiquidFreezesBelowWallYield",
            "Temperature of the liquid nitrogen in the line, from 68.5614 K to 109.90876941859975 K. At or below absolute zero it is no temperature (TemperatureNotPositive). Below nitrogen's triple point, 63.151 K, nitrogen is solid at every pressure; from there to 68.5614 K its melting line (Span et al. 2000) lies under the bore's first-yield pressure, so a surge the tube could still hold might freeze the liquid, and every such question is refused, whatever its own surge: the conservative rule (LiquidFreezesBelowWallYield). Above 109.90876941859975 K, the largest temperature at which the table's saturated vapour is at most a tenth as dense as its saturated liquid, the vapour cavity model, which gives the cavity no mass, does not hold (VapourNotLightAgainstLiquid); at and above the critical temperature, 126.192 K, there is no liquid. The DLR tests ran at 82.7 to 86.4 K, taken where the WebBook's density equals the Coriolis meter's.",
        ),
        number(
            "mass_flow_kg_s",
            0.0,
            100.0,
            "kg/s",
            "MassFlowNegative",
            "Steady mass flow toward the valve before it moves. Zero or more (MassFlowNegative). The flow speed it gives in the 19 mm bore, at the liquid's density at S1, must be at most a tenth of the wave speed there (InitialFlowMachAboveAcoustic): the line equations drop the flow's own convection. Friction to the valve must leave the steady pressure there above the vapour pressure (SteadyBelowVapour) and, with a flow, above the return pressure (SupplyCannotDriveFlow). At zero the valve's coefficient, which the steady flow sets, is zero: the valve is shut from the start, the line is at rest and stays so to the bit, and the return pressure does not act on it. Below a Reynolds number of about 1.6e-25 the friction is Poiseuille's, the limit of Churchill's factor. The DLR tests ran 1.06 to 2.82 kg/s.",
        ),
        above(
            "s1_pressure_pa",
            0.0,
            ln2::WALL_YIELD_PRESSURE_PA,
            "Pa",
            "S1PressureNotAboveVapour",
            "Steady absolute pressure at station S1, 0.60 m before the valve, before the valve moves: above the liquid's vapour pressure at the asked temperature (S1PressureNotAboveVapour), and with the friction to the tank at most 25,413,223.14049587 Pa, where the bore first yields (SupplyAboveWallYield): at zero flow S1 may be that pressure itself; with a flow, the supply then needed is above it. The DLR tests ran 4.50 to 22.40 bar absolute (the records are gauge).",
        ),
        number(
            "return_pressure_pa",
            n2::TRIPLE_POINT_PA,
            ln2::WALL_YIELD_PRESSURE_PA,
            "Pa",
            "ReturnBelowTriplePoint",
            "Absolute pressure behind the valve and the circuit after it, at the low-pressure tank, held. The circuit ends in a tank of liquid nitrogen, which exists only from nitrogen's triple-point pressure, 12,523 Pa, up (ReturnBelowTriplePoint); with a flow, it must be below the steady pressure at the valve (SupplyCannotDriveFlow). It sets, with the steady flow, the circuit's coefficient, so the valve's law, Q = c(tau) sqrt(2 (p_valve - p_return)/rho_0), holds at the asked return pressure as the steady flow there fixes it; below the liquid's vapour pressure the liquid flashes as it passes into the circuit, as it did in the DLR tests whose tank read under it, and the law stands for that circuit as its steady flow measured it. At zero flow the valve is shut and the return pressure does not act on the line. The DLR tests' low-pressure tank read 0.50 to 5.13 bar gauge before closure (0.53 to 0.89 in the three traced tests); 1.70 bar absolute was taken for every test compared, as the protocol fixed.",
        ),
        number(
            "stroke_time_s",
            ln2::STROKE_TIME_MIN_S,
            ln2::DURATION_MAX_S,
            "s",
            "StrokeFasterThanGrid",
            "The valve's stroke from 0.9 to 0.1 of open: its measured stroke shape is scaled to this time. 7.2555 ms is the facility's own (the mean of three traced tests). At least ten steps of the grid, which a faster stroke rings at (StrokeFasterThanGrid): the step is a section's length over the sections' wave speed, so the least stroke is 0.9524182150414855 ms at the coldest liquid just over its vapour pressure, about 1.049 ms at 83.7 K and 11 bar and 1.255 ms at the warmest liquid. At most the duration (StrokeTimeOutOfRange).",
        ),
        above(
            "duration_s",
            0.0,
            ln2::DURATION_MAX_S,
            "s",
            "DurationOutOfRange",
            "Time computed from the start of the stroke, above 0 and at most 10 s (DurationOutOfRange), a budget of compute and not a physical limit: one step is about 0.1 ms, so 10 s is about 95,000 steps, one to two seconds of computing on one core. The facility's records run 2 s after closure.",
        ),
        Input {
            name: ASK_HISTORIES,
            kind: Kind::Flag,
            refusal: None,
            bound: "true returns the pressure histories at S1, S2, S3 and the valve, the valve's flow and vapour, and the energy ledger's history, at most 501 samples each; false leaves them out.",
            only_when: None,
            asked_by_check: false,
        },
    ],
    kept_inside: "The line, its wall, its stations, its friction, the valve's stroke and seat, the liquid's table, the grid of 93 sections and the tolerances stay inside the solver.",
    result: &[
        one("vapour_pressure_pa", "Pa", "The liquid's vapour pressure at the asked temperature (NIST WebBook)."),
        one("density_kg_m3", "kg/m^3", "The liquid's density at S1's steady state: the inertia of the line."),
        one("sound_speed_m_s", "m/s", "The liquid's speed of sound at S1's steady state."),
        one("viscosity_pa_s", "Pa s", "The liquid's viscosity at S1's steady state."),
        one("steel_modulus_pa", "Pa", "Young's modulus of the steel at the liquid's temperature (NIST's fit for 304 stainless)."),
        one("wave_speed_m_s", "m/s", "The wave speed at S1's steady pressure: the liquid's compressibility and the wall's stretch."),
        one("section_wave_speed_m_s", "m/s", "The wave speed the sections carry: the liquid at the tube's yield pressure, its stiffest. The softer liquid at lower pressures is lumped at the nodes."),
        one("velocity_m_s", "m/s", "Steady mean velocity in the bore."),
        one("reynolds_number", "1", "Steady Reynolds number."),
        one("friction_factor", "1", "Churchill's Darcy factor at the steady flow, with the line's equivalent roughness."),
        one("supply_pressure_pa", "Pa", "Absolute pressure held at the tank's end of the line: S1's steady pressure and the friction between."),
        one("joukowsky_rise_pa", "Pa", "m a / A with the wave speed at S1's steady pressure: the rise of an instant closure on a liquid that does not stiffen."),
        one("time_step_s", "s", "One section's length over the sections' wave speed."),
        one("steady_s1_pa", "Pa", "Absolute steady pressure at S1, 6 sections (0.599 m) from the valve."),
        one("steady_s2_pa", "Pa", "Absolute steady pressure at S2, 44 sections (4.395 m) from the valve."),
        one("steady_s3_pa", "Pa", "Absolute steady pressure at S3, 82 sections (8.191 m) from the valve."),
        one("steady_valve_pa", "Pa", "Absolute steady pressure at the valve."),
        one("s1_max_pa", "Pa", "Highest absolute pressure at S1 over the run."),
        one("s1_max_time_s", "s", "When it was reached, from the start of the stroke."),
        one("s2_max_pa", "Pa", "Highest absolute pressure at S2 over the run."),
        one("s2_max_time_s", "s", "When it was reached."),
        one("s3_max_pa", "Pa", "Highest absolute pressure at S3 over the run."),
        one("s3_max_time_s", "s", "When it was reached."),
        one("line_pressure_max_pa", "Pa", "Highest absolute pressure anywhere on the line."),
        one("line_pressure_min_pa", "Pa", "Lowest absolute pressure anywhere on the line: the vapour pressure where it cavitated."),
        many("first_vapour_time_s", "s", "When each node that held vapour first did, in the order they did; empty when the line never cavitated."),
        many("first_vapour_position_m", "m", "Distance from the tank of each of those nodes; the valve is at 9.29 m."),
        one("vapour_volume_max_m3", "m^3", "The most vapour in the whole line at once."),
        one("largest_line_mach", "1", "Largest flow speed over the local wave speed, at nodes with no vapour."),
        one("stored_energy_start_j", "J", "Energy stored at the start, from the line at rest at the supply pressure: kinetic, the liquid's compression and the wall's strain."),
        one("stored_energy_end_j", "J", "The same at the end, with the vapour cavities'."),
        one("friction_loss_j", "J", "Energy friction took, force times velocity in every section and step."),
        one("valve_work_j", "J", "The sum of (p_valve - p_supply) Q_valve dt: the work through the valve, counted from the supply pressure."),
        one("cavity_loss_j", "J", "Energy the nodes' updates took: the backward step of the lumped stiffening and the impact of the columns where a vapour cavity closes. Each node's update is held to take energy and not give it, to a part in 1e9 of the energy scale or the rounding of its own terms where that is more, and to what its Newton root leaves unsettled, so the sum is zero or more to that; a line at rest gives exactly zero."),
        history("sample_times_s", "s", "Times of the samples, from 0."),
        history("s1_history_pa", "Pa", "Absolute pressure at S1 at each sample."),
        history("s2_history_pa", "Pa", "Absolute pressure at S2 at each sample."),
        history("s3_history_pa", "Pa", "Absolute pressure at S3 at each sample."),
        history("valve_history_pa", "Pa", "Absolute pressure at the valve at each sample."),
        history("valve_flow_history_m3_s", "m^3/s", "Flow through the valve at each sample."),
        history("valve_vapour_history_m3", "m^3", "Vapour at the valve at each sample."),
        history("kinetic_energy_history_j", "J", "Kinetic energy of the liquid at each sample."),
        history("liquid_energy_history_j", "J", "Elastic energy of the liquid's compression, in the sections and the lumps, at each sample, from the supply pressure."),
        history("wall_energy_history_j", "J", "Elastic energy of the wall's strain at each sample, from the supply pressure."),
        history("vapour_energy_history_j", "J", "Stored energy of the vapour cavities at each sample, from the supply pressure."),
        history("friction_loss_history_j", "J", "Friction's loss so far at each sample."),
        history("valve_work_history_j", "J", "The work through the valve so far at each sample."),
        history("cavity_loss_history_j", "J", "The nodes' loss so far at each sample."),
    ],
    invariants: &[
        one("largest_energy_ledger_residual", "1", "Largest, over every step, of the stored energy plus friction's loss, the valve's work and the nodes' loss, less the energy at the start, over the energy scale."),
        one("largest_volume_ledger_residual", "1", "Largest, over every step, of the liquid stored by compression less the vapour, less what the supply put in and plus what the valve let out, against the start, over the volume scale: a mass ledger over the reference density."),
    ],
    iterations: "Steps of the grid, each one time step: every section's friction and every node settled once. On a refusal it is null.",
    field: field_words,
    answer: "a transient",
    refusals: REFUSALS,
    door_refusals: &[],
    sections,
    validation: Some(validation),
    example,
    further_examples: &[
        crate::solvers::FurtherExample { key: "example_solve_dlr_20200805_10", parameters: example_held_back },
        crate::solvers::FurtherExample { key: "example_solve_low_pressure_cavitation", parameters: example_low_pressure },
    ],
    parts: &[],
    check,
    solve,
};

const SUMMARY: &str = "Fluid hammer in the liquid-nitrogen line of DLR's Fluid Transient Test Facility, Lampoldshausen (9.29 m of 1.4541 tube, 19 x 1.5 mm, in a vacuum jacket), when its fast valve closes on a steady flow: liquid nitrogen's density, speed of sound, viscosity and vapour pressure at the asked temperature from the NIST WebBook, the liquid's stiffening under pressure carried along the line, Churchill's friction, the valve's measured stroke, and vapour cavities where the line cavitates; pressure at the facility's stations S1, S2 and S3; energy and mass ledgers closed at every step; stops by name at the tube's first yield (25.41 MPa) and at the liquid's freezing and near-critical bounds. Scored once against six held-back DLR tests on eleven marks committed beforehand to the engine's own record, not to the public pass-marks repository: see validation.";

/// Each `LineReject` variant with the sentence from its doc comment. A
/// refusal the request itself shows is not sealed.
const REFUSALS: &[Refusal] = &[
    Refusal { name: "InputNotFinite", meaning: "An input was NaN or infinite.", sealed: false },
    Refusal { name: "TemperatureNotPositive", meaning: "The temperature is not above absolute zero.", sealed: false },
    Refusal { name: "LiquidFreezesBelowWallYield", meaning: "Below `COLDEST_K` liquid nitrogen freezes at a pressure the tube holds: its melting pressure (Span et al. 2000) is under the bore's first-yield pressure, and below the triple point it is solid at every pressure.", sealed: false },
    Refusal { name: "VapourNotLightAgainstLiquid", meaning: "Above `WARMEST_K` the saturated vapour is more than a tenth as dense as the liquid, and the vapour cavity model, which gives the cavity no mass, does not hold.", sealed: false },
    Refusal { name: "S1PressureNotAboveVapour", meaning: "The steady pressure at S1 is not above the liquid's vapour pressure: the line does not hold liquid.", sealed: false },
    Refusal { name: "SupplyAboveWallYield", meaning: "The supply pressure the steady flow needs is above the bore's first-yield pressure.", sealed: false },
    Refusal { name: "MassFlowNegative", meaning: "The mass flow is negative: the line runs from the tank to the valve.", sealed: false },
    Refusal { name: "ReturnBelowTriplePoint", meaning: "The return pressure is below nitrogen's triple-point pressure: the circuit behind the valve ends in a tank of liquid nitrogen, and below it there is none.", sealed: false },
    Refusal { name: "SupplyCannotDriveFlow", meaning: "Friction to the valve takes the pressure below the return pressure: the steady flow cannot run.", sealed: false },
    Refusal { name: "SteadyBelowVapour", meaning: "The steady pressure somewhere on the line is not above the vapour pressure.", sealed: false },
    Refusal { name: "InitialFlowMachAboveAcoustic", meaning: "The steady flow is faster than `MACH_LIMIT` of the wave speed.", sealed: false },
    Refusal { name: "DurationOutOfRange", meaning: "The duration is not above zero or is above `DURATION_MAX_S`.", sealed: false },
    Refusal { name: "StrokeTimeOutOfRange", meaning: "The stroke time is negative or longer than the duration.", sealed: false },
    Refusal { name: "StrokeFasterThanGrid", meaning: "The stroke is faster than `STROKE_STEPS_MIN` steps of the grid, which it does not resolve.", sealed: false },
    Refusal { name: "WallYield", meaning: "The transient reached the bore's first-yield pressure.", sealed: true },
    Refusal { name: "LineMachAboveAcoustic", meaning: "The flow at a node reached `MACH_LIMIT` of the local wave speed.", sealed: true },
    Refusal { name: "VapourOverLineShare", meaning: "The vapour cavities together took more than `VAPOUR_SHARE_LIMIT` of the line's volume.", sealed: true },
    Refusal { name: "FrictionNotSettled", meaning: "The friction of a section did not settle.", sealed: true },
    Refusal { name: "NodeNotSettled", meaning: "A node did not settle.", sealed: true },
    Refusal { name: "NonFiniteValue", meaning: "A value was NaN or infinite.", sealed: true },
    Refusal { name: "EnergyLedgerBroken", meaning: "The energy ledger did not close.", sealed: true },
    Refusal { name: "VolumeLedgerBroken", meaning: "The volume ledger did not close.", sealed: true },
    Refusal { name: "FrictionGaveEnergy", meaning: "A section's friction gave energy to the flow.", sealed: true },
    Refusal { name: "CavityGaveEnergy", meaning: "A node gave energy to the waves.", sealed: true },
];

/// The `LineReject` variant name, without its fields.
pub fn reject_name(reject: &LineReject) -> &'static str {
    match reject {
        LineReject::InputNotFinite { .. } => "InputNotFinite",
        LineReject::TemperatureNotPositive { .. } => "TemperatureNotPositive",
        LineReject::LiquidFreezesBelowWallYield { .. } => "LiquidFreezesBelowWallYield",
        LineReject::VapourNotLightAgainstLiquid { .. } => "VapourNotLightAgainstLiquid",
        LineReject::S1PressureNotAboveVapour { .. } => "S1PressureNotAboveVapour",
        LineReject::SupplyAboveWallYield { .. } => "SupplyAboveWallYield",
        LineReject::MassFlowNegative { .. } => "MassFlowNegative",
        LineReject::ReturnBelowTriplePoint { .. } => "ReturnBelowTriplePoint",
        LineReject::SupplyCannotDriveFlow { .. } => "SupplyCannotDriveFlow",
        LineReject::SteadyBelowVapour { .. } => "SteadyBelowVapour",
        LineReject::InitialFlowMachAboveAcoustic { .. } => "InitialFlowMachAboveAcoustic",
        LineReject::DurationOutOfRange { .. } => "DurationOutOfRange",
        LineReject::StrokeTimeOutOfRange { .. } => "StrokeTimeOutOfRange",
        LineReject::StrokeFasterThanGrid { .. } => "StrokeFasterThanGrid",
        LineReject::WallYield { .. } => "WallYield",
        LineReject::LineMachAboveAcoustic { .. } => "LineMachAboveAcoustic",
        LineReject::VapourOverLineShare { .. } => "VapourOverLineShare",
        LineReject::FrictionNotSettled { .. } => "FrictionNotSettled",
        LineReject::NodeNotSettled { .. } => "NodeNotSettled",
        LineReject::NonFiniteValue { .. } => "NonFiniteValue",
        LineReject::EnergyLedgerBroken { .. } => "EnergyLedgerBroken",
        LineReject::VolumeLedgerBroken { .. } => "VolumeLedgerBroken",
        LineReject::FrictionGaveEnergy { .. } => "FrictionGaveEnergy",
        LineReject::CavityGaveEnergy { .. } => "CavityGaveEnergy",
    }
}

fn field_words() -> String {
    "SHA-256 of the count of the numbers that follow as 8 little-endian bytes, then the absolute pressure at S1 at the start and after every step, then the same at S2, then at S3, then the absolute pressure at every node at the end from the tank to the valve, then the velocity at every node at the end in the same order, all as little-endian f64 bytes.".to_string()
}

fn sections() -> Sections {
    Sections {
        locked: json!({
            "specimen": "The liquid-nitrogen line of DLR's Fluid Transient Test Facility at Lampoldshausen (Klein, Traudt and Deeken, Zenodo 15526459): 9.29 m of stainless steel 1.4541 tube, 19 mm bore and 1.5 mm wall, in one and a half spiral turns in a vacuum jacket, from the high-pressure tank to a fast valve and the circuit to the low-pressure tank; stations S1, S2 and S3 at 6.46, 47.3 and 88.2 percent of the length from the valve (the overview's Table 1 and Figure 1: PS408, PD409 and TI407 at S1).",
            "line_length_m": ln2::LINE_LENGTH_M,
            "bore_m": ln2::BORE_M,
            "wall_thickness_m": ln2::WALL_THICKNESS_M,
            "stations_from_valve": [ln2::S1_FROM_VALVE, ln2::S2_FROM_VALVE, ln2::S3_FROM_VALVE],
            "sections": ln2::SECTIONS,
            "station_sections_from_valve": [ln2::S1_SECTIONS_FROM_VALVE, ln2::S2_SECTIONS_FROM_VALVE, ln2::S3_SECTIONS_FROM_VALVE],
            "steel_poisson_ratio": ln2::STEEL_POISSON_RATIO,
            "steel_modulus_fit_gpa": ln2::STEEL_MODULUS_FIT_GPA,
            "steel_proof_strength_pa": ln2::STEEL_PROOF_STRENGTH_PA,
            "wall_yield_pressure_pa": ln2::WALL_YIELD_PRESSURE_PA,
            "roughness_m": ln2::ROUGHNESS_M,
            "valve_series_ratio": ln2::VALVE_SERIES_RATIO,
            "valve_seat": ln2::VALVE_SEAT,
            "stroke_reference_s": ln2::STROKE_REFERENCE_S,
            "stroke_step_s": ln2::STROKE_STEP_S,
            "stroke_open_fraction": ln2::STROKE.to_vec(),
            "coldest_k": ln2::COLDEST_K,
            "warmest_k": ln2::WARMEST_K,
            "mach_limit": ln2::MACH_LIMIT,
            "vapour_share_limit": ln2::VAPOUR_SHARE_LIMIT,
            "liquid": "Liquid nitrogen from the NIST Chemistry WebBook (SRD 69; the equation of state of Span, Lemmon, Jacobsen, Wagner and Yokozeki 2000 and the viscosity of Lemmon and Jacobsen 2004), retrieved 2026-10-14: 43 isotherms, 68.561 K and every kelvin from 69 to 110 K, from the vapour pressure in steps of 2.5 bar to 262.5 bar over it, interpolated at the same pressure over the vapour pressure; within 1.6e-4 of the WebBook's speed of sound, 7.3e-5 of its density and 4.2e-4 of its viscosity half a kelvin off the grid.",
            "equations": "In each section rho_0 dV/dt + dp/dx = -rho_0 f V|V|/(2D) and the liquid's compliance 1/(rho_0 c(p)^2) + D psi/(E e), with rho_0 the density at S1's steady state, c(p) the WebBook's speed of sound along the asked temperature (so the stored volume is the mass gained over rho_0, since rho beta = 1/c^2), psi = 2(e/D)(1 + nu) + D(1 - nu^2)/(D + e) (thick-walled, anchored; Wylie and Streeter 1993) and E at the liquid's temperature. The sections carry the compliance at the yield pressure, the stiffest, as C+ = p + ZQ and C- = p - ZQ one section a step with nothing interpolated; the rest, 1/(rho_0 c(p)^2) - 1/(rho_0 c_top^2), linear in p between 2.5 bar steps, is lumped at the nodes, each holding it for its reach (volume and energy in closed form, convex; a node takes its pressure at the step's end). Where the pressure would fall below the vapour pressure a node holds a vapour cavity at it (the discrete vapour cavity model). f is Churchill's (1977) factor at the equivalent roughness, read from a table made for the run within 2e-6 of the formula. The valve: Q = c(tau) sqrt(2 (p_valve - p_return)/rho_0), c(tau) = c_v t c_d/sqrt((c_v t)^2 + c_d^2) with t = (tau - seat)/(1 - seat), tau the measured stroke, c_v/c_d the series ratio, normalised to the steady flow.",
            "fitted": "Three numbers, on the three traced tests only (20200806_13, 20200803_11, 20200806_16), before the six held-back tests were opened: the roughness, 12.53 um, to their steady pressure drop from S3 to S1; and the valve's seat, 0.04 of the stroke, and series ratio, 2.0, to the times of their first rise at S1. The stroke's shape is their mean position trace. Every other constant is the record's, the WebBook's, NIST's, a standard's or a published law's.",
        }),
        invariants: json!([
            {
                "name": "largest_energy_ledger_residual",
                "rule": "Energy is counted from the line at rest at the supply pressure. The waves in flight hold dt (C+^2 + C-^2)/(4Z) a section (kinetic dt (C+ - C-)^2/(8Z), elastic dt (C+ + C-)^2/(8Z), split between the liquid and the wall by their shares of the sections' compliance), each node the energy of its lump and of its vapour cavity. That, plus friction's loss, the valve's work and the nodes' loss summed step by step, equals the energy at the start.",
                "tolerance": line::ENERGY_RESIDUAL_LIMIT,
                "scale": "The line's volume times (rho_0 V0^2/2 + (p_supply - p_vapour)^2 (C_top + e(p_vapour))/2), or, where that is smaller, 1e-5 of the sizes of the terms the ledger adds at that step (every section's waves, every node's terms, what has gone) and 1e-6 of the nodes' second-law terms over every step so far, which each lump's Newton root, settled to a part in 1e12 of its volume, carries on: a line creeping just over its vapour pressure shrinks the first and not the others, and below them the ledger would judge their rounding, not the physics.",
                "compared": "every step",
                "refusal": "EnergyLedgerBroken",
            },
            {
                "name": "largest_volume_ledger_residual",
                "rule": "The waves hold dt (C+ + C-)/(2Z) a section and each node's lump its volume, the liquid's mass gained by compression over the supply pressure, over rho_0; the vapour takes volume from the liquid. Stored less vapour rises by what the supply put in and falls by what the valve let out.",
                "tolerance": line::VOLUME_RESIDUAL_LIMIT,
                "scale": "The line's volume times (p_supply - p_vapour) times the most compliant liquid's compliance, or, where that is smaller, 1e-5 of the sizes of the volumes the ledger adds at that step and 1e-6 of the nodes' lump volumes over every step so far.",
                "compared": "every step",
                "refusal": "VolumeLedgerBroken",
            },
            {
                "name": "second law",
                "rule": "Each section's friction takes energy and never gives it, and each node's update takes energy from the waves and never gives it: held to a part in 1e9 of the energy scale, or to the rounding of the node's own terms (its pressure times its volumes, its energy's terms) where that is more, and to what its root leaves unsettled, its own and the one its energy was carried from, and to nothing else.",
                "compared": "every section and node of every step",
                "refusal": "FrictionGaveEnergy or CavityGaveEnergy",
            },
        ]),
        convergence: json!({
            "rule": "A transient is a march of fixed steps, one section's crossing time each. Each section's friction is a fixed point settled to a part in 1e9 at that step's factor; each node between two sections is the root of a concave rising function, by Newton's method to a part in 1e12 of its volume; the valve's node, and any node Newton's method hands over, is found by false position on a function that rises strictly, and past its passes by halving to the bracket's last bit. A node whose lump empties within a step is at the top of its law, the yield pressure, and the rest of the inflow raises it against the sections alone; past the yield the run stops there by name. A node at rest at the supply pressure whose flows balance there keeps that state exactly, so a line at rest stays at rest to the bit. Nothing is returned that has not held at every step.",
            "max_friction_passes": line::MAX_FRICTION_PASSES,
            "max_node_passes": line::MAX_NODE_PASSES,
            "max_bisection_passes": line::MAX_BISECTION_PASSES,
            "refusal": "FrictionNotSettled or NodeNotSettled",
            "run_time": "With every history, on one core of an Apple M4 Pro: about 0.8 s for a run of 10 s at 84 K and 11 bar, up to 1.9 s near the yield pressure or with the line cavitating; a run of 2 s takes 0.2 to 0.5 s.",
        }),
        reference: json!({
            "name": ln2::REFERENCE_NAME,
            "url": ln2::REFERENCE_URL,
            "retrieved": ln2::REFERENCE_RETRIEVED,
            "test": "dlr_scored_once",
            "covers": RESULT,
            "kind": "Measurements of this line: the facility's own records, three tests opened to fix three constants and six held back and scored once on marks committed before they were opened, in the engine's own record and not in the public pass-marks repository. Record: evidence/research/hydraulics_robotics/dlr_ln2_line/.",
        }),
    }
}

fn validation() -> [String; 5] {
    let object = "The liquid-nitrogen line of DLR's Fluid Transient Test Facility at Lampoldshausen, 9.29 m of 1.4541 tube, 19 x 1.5 mm, a fast valve at its end. The solver computes the steady flow and the fluid hammer of the valve's closure, with the liquid's stiffening and cavitation, at the facility's stations S1, S2 and S3. Three constants are fitted, on the three traced tests only: the roughness and the valve's seat and series ratio.".to_string();
    let data = "S. Klein, T. Traudt and J. Deeken, DLR Institute of Space Propulsion, Liquid Nitrogen Fluid Hammer: PS-LN2-Set1, Zenodo 15526459, CC BY 4.0: nine tests, 1.06 to 2.82 kg/s at 3.5 to 21.4 bar gauge at S1, the liquid at 82.7 to 86.4 K by the Coriolis meter's density; static pressures at 10 kHz, dynamic at 150 kHz, the valve's position, the line thermometer and the Coriolis meter. Opened: the overview and every test's steady sheets, and the traces of 20200806_13, 20200803_11 and 20200806_16. Held back until the protocol was fingerprinted: the traces of 20200806_5, 20200803_8, 20200806_15, 20200805_9, 20200803_15 and 20200805_10.".to_string();
    let pass_line = "Fixed on 2026-10-15 UTC before any held-back trace was downloaded (protocol SHA-256 9742e60eb828783056b423e960c65b1d62bcc4142fdb5324a5fcfb24dd4cdf99, with a forecast of every quantity). Eleven marks, each met when at least five of the six held-back tests are inside its band: the first rise at S1 within 5 percent (published for all nine as dP, so not held back), at S2 within 5, at S3 within 10; the front's speed from S1 to S2 and from S2 to S3 within 3; the plateau at S1 within 3; the second surge's time within 15 and its height within 20; the late free oscillation's period within 5 and its decay rate within 30; the steady drop from S3 to S1 within 10. The front is timed on the dynamic sensors, sampled at 150 kHz.".to_string();
    let limits = LIMITS.to_string();
    [object, data, pass_line, RESULT.to_string(), limits]
}

/// The outcome of the one scoring of the six held-back tests.
pub(crate) const RESULT: &str = "Scored once, on 2026-10-15 UTC, with the solver as forecast (evidence/research/hydraulics_robotics/dlr_ln2_line/scored.json). Four marks met, seven missed. Met, each in all six held-back tests: the first rise at S2, 1.7 percent low to 0.9 high; the front's speed from S1 to S2, 0.4 percent low to 0.7 high; the second surge's time, 8.1 percent short to 1.4 long; the steady drop from S3 to S1, 9.3 percent low to 2.1 high. Missed: the first rise at S1, 4.8 to 6.8 percent high (2 of 6 within 5; it is the overview's dP, published for all nine tests, and was not held back); the first rise at S3, 2.6 to 14.9 percent low (1 of 6 within 10); the front from S2 to S3, 2.3 percent low to 0.4 high in the four tests where the dynamic sensor at S3 recorded and unmeasured in the two where it recorded no signal (4 of 6); the plateau at S1, 4.2 to 7.4 percent short (0 of 6 within 3), the wave reflected from the tank coming back 0.9 to 1.7 ms early; the second surge's height, 6.9 to 22.6 percent high at the higher pressures and 39 and 111 percent at the two lowest (3 of 6 within 20); the late free oscillation (0 of 6 within 5 and 30): the rule's fit failed on the computed traces of the two lowest-pressure tests, 20200806_5 and 20200803_8, its leftover 3.5 and 1.0 times its amplitude, so the decay rates it gave there, -1.68 and -0.62 per s, are not rates (a fit is taken where its leftover is under half its amplitude, as it is on every other trace, computed and measured, at most 0.16: a rule read after the scoring, which moves no mark); on the other four the computed period is 10.0 to 13.1 percent short and the decay 78 to 85 percent under the measured, and on the two, read from the computed trace's envelope (half its swing every 50 ms, 0.5 to 1.5 s), 84 and 91 percent under: after it cavitates the measured line rings slower and dies faster than a liquid of quasi-steady friction whose cavities hold vapour alone. The protocol forecast ten of the eleven outcomes; it forecast the front from S2 to S3 met. On the three traced tests that fixed the three constants, not blind: the first rise at S1 4.4 to 7.3 percent high, at S2 2.2 low to 2.9 high, the front from S1 to S2 within 0.6 percent.";

/// What the comparison does not cover, and what the model leaves out.
const LIMITS: &str = "Every compared test is of this one line, at 82.7 to 86.4 K, 3.5 to 21.4 bar gauge at S1 and 1.06 to 2.82 kg/s. What the model holds: the front runs at the wave speed, not at the wave speed less the flow's (0.6 to 1.7 percent in time at these flows); the liquid holds the asked temperature through its compression; a cavity holds vapour at the vapour pressure and nothing else; friction is quasi-steady, Churchill's factor at the instantaneous velocity, which damps a hammer more slowly than it is measured to damp; the tube is anchored, the spiral's curvature is in the fitted roughness alone, and the line meets the tank at 9.29 m. The valve's seat and series ratio and the roughness were fitted to three tests and stand for this valve and this line only. A proof shows that an answer was not changed, not that the model matches the line.";

fn example() -> Value {
    json!({
        "temperature_k": 83.7,
        "mass_flow_kg_s": 1.9739,
        "s1_pressure_pa": 1144325.0,
        "return_pressure_pa": 170000.0,
        "stroke_time_s": 0.0072555,
        "duration_s": 0.05,
        "return_histories": false
    })
}

fn example_held_back() -> Value {
    json!({
        "temperature_k": 83.7,
        "mass_flow_kg_s": 2.625,
        "s1_pressure_pa": 2107325.0,
        "return_pressure_pa": 170000.0,
        "stroke_time_s": 0.0072555,
        "duration_s": 0.3,
        "return_histories": true
    })
}

fn example_low_pressure() -> Value {
    json!({
        "temperature_k": 82.7,
        "mass_flow_kg_s": 1.0642,
        "s1_pressure_pa": 450325.0,
        "return_pressure_pa": 170000.0,
        "stroke_time_s": 0.0072555,
        "duration_s": 0.25,
        "return_histories": false
    })
}

/// The solver's own question, from the values of `parameters`.
fn question(given: &Given) -> Result<LineQuestion, String> {
    Ok(LineQuestion {
        temperature_k: given.number("temperature_k")?,
        mass_flow_kg_s: given.number("mass_flow_kg_s")?,
        s1_pressure_pa: given.number("s1_pressure_pa")?,
        return_pressure_pa: given.number("return_pressure_pa")?,
        stroke_time_s: given.number("stroke_time_s")?,
        duration_s: given.number("duration_s")?,
    })
}

/// Every limit the request shows, asked of the solver's own check before a
/// physics permit is taken.
fn check(given: &Given) -> Result<(), String> {
    ln2::admit(&question(given)?).map(|_| ()).map_err(|reject| format!("Physical Violation: {reject} ({}).", reject_name(&reject)))
}

fn solve(given: &Given) -> Result<Solved, String> {
    let histories = given.flag(ASK_HISTORIES)?;
    Ok(solved(ln2::solve(&question(given)?), histories))
}

/// What the solver returned, in the words every solver shares.
pub fn solved(returned: Result<LineHammer, LineReject>, histories: bool) -> Solved {
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
    let s = &h.state;
    let mut result = vec![
        one("vapour_pressure_pa", s.vapour_pressure_pa),
        one("density_kg_m3", s.density_kg_m3),
        one("sound_speed_m_s", s.sound_speed_m_s),
        one("viscosity_pa_s", s.viscosity_pa_s),
        one("steel_modulus_pa", s.steel_modulus_pa),
        one("wave_speed_m_s", s.wave_speed_m_s),
        one("section_wave_speed_m_s", s.section_wave_speed_m_s),
        one("velocity_m_s", s.velocity_m_s),
        one("reynolds_number", s.reynolds_number),
        one("friction_factor", s.friction_factor),
        one("supply_pressure_pa", s.supply_pressure_pa),
        one("joukowsky_rise_pa", s.joukowsky_rise_pa),
        one("time_step_s", h.time_step_s),
        one("steady_s1_pa", h.steady_s1_pa),
        one("steady_s2_pa", h.steady_s2_pa),
        one("steady_s3_pa", h.steady_s3_pa),
        one("steady_valve_pa", h.steady_valve_pa),
        one("s1_max_pa", h.s1_max_pa),
        one("s1_max_time_s", h.s1_max_time_s),
        one("s2_max_pa", h.s2_max_pa),
        one("s2_max_time_s", h.s2_max_time_s),
        one("s3_max_pa", h.s3_max_pa),
        one("s3_max_time_s", h.s3_max_time_s),
        one("line_pressure_max_pa", h.line_pressure_max_pa),
        one("line_pressure_min_pa", h.line_pressure_min_pa),
        many("first_vapour_time_s", &h.first_vapour_time_s),
        many("first_vapour_position_m", &h.first_vapour_position_m),
        one("vapour_volume_max_m3", h.vapour_volume_max_m3),
        one("largest_line_mach", h.largest_line_mach),
        one("stored_energy_start_j", h.stored_energy_start_j),
        one("stored_energy_end_j", h.stored_energy_end_j),
        one("friction_loss_j", h.friction_loss_j),
        one("valve_work_j", h.valve_work_j),
        one("cavity_loss_j", h.cavity_loss_j),
    ];
    if histories {
        result.extend([
            many("sample_times_s", &h.sample_times_s),
            many("s1_history_pa", &h.s1_history_pa),
            many("s2_history_pa", &h.s2_history_pa),
            many("s3_history_pa", &h.s3_history_pa),
            many("valve_history_pa", &h.valve_history_pa),
            many("valve_flow_history_m3_s", &h.valve_flow_history_m3_s),
            many("valve_vapour_history_m3", &h.valve_vapour_history_m3),
            many("kinetic_energy_history_j", &h.kinetic_energy_history_j),
            many("liquid_energy_history_j", &h.liquid_energy_history_j),
            many("wall_energy_history_j", &h.wall_energy_history_j),
            many("vapour_energy_history_j", &h.vapour_energy_history_j),
            many("friction_loss_history_j", &h.friction_loss_history_j),
            many("valve_work_history_j", &h.valve_work_history_j),
            many("cavity_loss_history_j", &h.cavity_loss_history_j),
        ]);
    }
    Solved::Answer(Answer {
        iterations: h.steps,
        result,
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
    use crate::solvers::describe_value;

    const SOLVER_SOURCE: &str = include_str!("../../genesis_solvers/src/ln2_line_dlr_fttf.rs");

    /// `(variant, doc sentence)` for every variant of `LineReject`, read
    /// from the solver's source. An intra-doc link is written as its bare
    /// code span.
    fn documented_rejects() -> Vec<(String, String)> {
        let start = SOLVER_SOURCE.find("pub enum LineReject {").expect("LineReject is in the solver");
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
    fn the_reference_is_the_one_stored_in_the_solver() {
        let body = describe_value(LN2_LINE_DLR_FTTF.name).expect("described");
        assert_eq!(body["reference"]["url"], ln2::REFERENCE_URL);
        assert_eq!(body["reference"]["retrieved"], ln2::REFERENCE_RETRIEVED);
        let test = body["reference"]["test"].as_str().expect("test");
        assert!(SOLVER_SOURCE.contains(&format!("fn {test}()")), "{test}");
        // The protocol's fingerprint served is the one the solver holds.
        let pass_line = body["validation"]["pass_line"].as_str().expect("pass line");
        assert!(pass_line.contains(ln2::PROTOCOL_SHA256), "{pass_line}");
    }

    #[test]
    fn every_published_name_is_in_the_entry_and_the_histories_only_when_asked() {
        let q = LineQuestion { temperature_k: 83.7, mass_flow_kg_s: 1.9739, s1_pressure_pa: 1144325.0, return_pressure_pa: 1.7e5, stroke_time_s: 0.0072555, duration_s: 0.02 };
        for histories in [false, true] {
            let answer = match solved(ln2::solve(&q), histories) {
                Solved::Answer(answer) => answer,
                Solved::Refused(refused) => panic!("{}", refused.sentence),
            };
            let listed: Vec<&Output> = LN2_LINE_DLR_FTTF.result.iter().filter(|o| histories || o.asked_by.is_none()).collect();
            assert_eq!(answer.result.len(), listed.len());
            for ((name, value), output) in answer.result.iter().zip(listed) {
                assert_eq!(name, output.name);
                assert_eq!(value.shape(), output.shape);
            }
            for ((name, _), output) in answer.invariants.iter().zip(LN2_LINE_DLR_FTTF.invariants) {
                assert_eq!(name, output.name);
            }
        }
    }
}
