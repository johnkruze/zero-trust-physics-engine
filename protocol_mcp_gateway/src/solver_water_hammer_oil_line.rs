//! The entry for `water_hammer_oil_line`: the company's own steel line of
//! mineral oil when the valve at its end closes fast. The numbers come from
//! `genesis_solvers` constants.

use genesis_solvers::line_transient as line;
use genesis_solvers::water_hammer_oil_line::{self as hammer, Hammer, HammerQuestion, HammerReject};
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

pub const WATER_HAMMER_OIL_LINE: Solver = Solver {
    name: "water_hammer_oil_line",
    summary: SUMMARY,
    inputs: &[
        above(
            "supply_pressure_pa",
            hammer::OIL_VAPOUR_PRESSURE_PA,
            hammer::WALL_YIELD_PRESSURE_PA,
            "Pa",
            "SupplyAboveWallYield",
            "Absolute pressure of the supply at the line's inlet, held through the run (a pump with an accumulator, or a pressure line). Up to 51,507,575 Pa, where the tube's bore first yields (Tresca on Lamé's stresses, E235 steel at 235 MPa, 51.41 MPa over the atmosphere outside), that pressure itself taken; above it is SupplyAboveWallYield. It must be above the return pressure (ReturnNotBelowSupply).",
        ),
        above(
            "return_pressure_pa",
            hammer::OIL_VAPOUR_PRESSURE_PA,
            hammer::WALL_YIELD_PRESSURE_PA,
            "Pa",
            "ReturnNotAboveVapour",
            "Absolute pressure downstream of the valve, held: the tank or the return line. Above the oil's vapour pressure, 10 Pa (ReturnNotAboveVapour), and below the supply pressure (ReturnNotBelowSupply). With the asked velocity, friction along the line must leave the steady pressure at the valve above it (SupplyCannotDriveFlow).",
        ),
        number(
            "initial_velocity_m_s",
            0.0,
            hammer::VELOCITY_MAX_M_S,
            "m/s",
            "VelocityNegative",
            "Steady mean velocity of the oil toward the valve before it moves, in the 9 mm bore. Zero or more (VelocityNegative). At most a tenth of Wood's wave speed of the oil and its air at the valve's steady pressure, 136.59 m/s with no air (InitialFlowMachAboveAcoustic): the line equations drop the flow's own convection.",
        ),
        number(
            "closure_time_s",
            0.0,
            hammer::DURATION_MAX_S,
            "s",
            "ClosureTimeOutOfRange",
            "Time over which the valve's open area falls linearly to zero, from the start of the run; zero closes it within the first step. At most the duration (ClosureTimeOutOfRange). With free air in the oil, at least the 6.75e-29 the line resolves, it must span at least ten steps of the grid, 0.732 ms (ClosureFasterThanGrid): a faster closure rings at the grid's own scale, where the air is lumped at the nodes.",
        ),
        number(
            "free_air_fraction",
            0.0,
            hammer::FREE_AIR_MAX,
            "1",
            "FreeAirNegative",
            "Volume of free air carried in the oil as bubbles, per volume of oil, at one standard atmosphere (101,325 Pa); zero for oil with none. The air is isothermal and is lumped at the grid's nodes. At the valve's steady pressure it must be a void fraction of at most 0.25, past which the flow is no longer bubbly (FreeAirAboveBubblyFlow), and in the transient, where the air expands as the pressure falls, the same at every node and step (LineVoidAboveBubblyFlow). The most any pressure allows is 169.44674726540012, the largest number whose void fraction at the bore's yield pressure is at most 0.25, taken there with the line at rest; one number above it is FreeAirAboveBubblyFlow at every pressure. Air under 6.75e-29 is run as none: even at the oil's vapour pressure its compliance is under 2^-53 of the oil's, below the resolution of every number the line computes. Negative is FreeAirNegative.",
        ),
        above(
            "duration_s",
            0.0,
            hammer::DURATION_MAX_S,
            "s",
            "DurationOutOfRange",
            "Time computed from the start of the closure, above 0 and at most 1 s (DurationOutOfRange), a budget of compute and not a physical limit: 1 s is 13,660 steps of 73.210 us, about 0.3 s of computing on one core with every history; by then friction has taken all but a few millionths of the wave's energy in every run tried.",
        ),
        Input {
            name: ASK_HISTORIES,
            kind: Kind::Flag,
            refusal: None,
            bound: "true returns the histories at the valve and the midpoint and the energy ledger's history, at most 501 samples each; false leaves them out.",
            only_when: None,
            asked_by_check: false,
        },
    ],
    kept_inside: "The tube, the oil, the grid of forty sections, the friction law and the tolerances stay inside the solver.",
    result: &[
        one("liquid_wave_speed_m_s", "m/s", "Wave speed of the oil in the tube with no air: the oil's compressibility and the wall's stretch."),
        one("wood_wave_speed_m_s", "m/s", "Wood's wave speed of the oil with its air at the valve's steady pressure, with the wall."),
        one("void_fraction_at_valve", "1", "The air's share of the volume at the valve's steady pressure."),
        one("joukowsky_rise_liquid_pa", "Pa", "rho a_l V0: the rise an instant closure gives in oil with no air."),
        one("joukowsky_rise_wood_pa", "Pa", "rho_m a_w V0: the rise from Wood's wave speed at the steady pressure, the tangent of the gas law."),
        one("joukowsky_rise_chord_pa", "Pa", "The rise when the rise itself compresses the air: the chord of the gas law from the steady pressure to the risen one. The line's first rise follows this one."),
        one("round_trip_time_s", "s", "2 L over Wood's wave speed: the time for the wave to reach the supply and come back."),
        one("time_step_s", "s", "One section's length over the oil's wave speed with no air."),
        one("steady_valve_pressure_pa", "Pa", "Absolute pressure at the valve before it moves: the supply less the friction of the steady flow."),
        one("valve_pressure_first_rise_pa", "Pa", "The highest pressure at the valve, over its steady pressure, until the closure ends plus 2 L over the oil's wave speed with no air: the first rise, with the line packing that friction adds."),
        one("valve_pressure_max_pa", "Pa", "Highest absolute pressure at the valve over the run."),
        one("valve_pressure_max_time_s", "s", "When it was reached."),
        one("valve_pressure_min_pa", "Pa", "Lowest absolute pressure at the valve; the vapour pressure, 10 Pa, where the column separated."),
        one("line_pressure_max_pa", "Pa", "Highest absolute pressure anywhere on the line."),
        one("line_pressure_min_pa", "Pa", "Lowest absolute pressure anywhere on the line."),
        one("cavity_volume_max_m3", "m^3", "Largest volume of gas and vapour at any one node."),
        one("largest_line_mach", "1", "Largest flow speed over the local wave speed of oil and air, at nodes with no vapour."),
        many("first_vapour_time_s", "s", "When each node that held vapour first did, in the order they did; empty when the column never separated."),
        many("first_vapour_position_m", "m", "Distance from the supply of each of those nodes."),
        many("node_positions_m", "m", "Distance from the supply of every node, the supply at 0 and the valve at 4."),
        many("pressure_max_along_line_pa", "Pa", "Highest absolute pressure at each node over the run."),
        many("pressure_min_along_line_pa", "Pa", "Lowest absolute pressure at each node over the run."),
        one("stored_energy_start_j", "J", "Energy stored at the start, from the line at rest at the supply pressure: kinetic, the oil's compression, the wall's strain and the air's."),
        one("stored_energy_end_j", "J", "The same at the end."),
        one("friction_loss_j", "J", "Energy friction took, force times velocity in every section and step."),
        one("valve_work_j", "J", "The sum of (p_valve - p_supply) Q_valve dt: the work through the valve, counted from the supply pressure."),
        one("cavity_loss_j", "J", "Energy the nodes' cavity updates took: the backward step of the lumped air and the last step of each closing vapour cavity. Each node's update is held to take energy and not give it, to a part in 1e9 of the energy scale or the rounding of its own terms where that is more, so the sum is zero or more to that; a line at rest gives exactly zero."),
        history("sample_times_s", "s", "Times of the samples, from 0."),
        history("valve_pressure_history_pa", "Pa", "Absolute pressure at the valve at each sample."),
        history("midpoint_pressure_history_pa", "Pa", "Absolute pressure at the line's midpoint, 2 m from the supply, at each sample."),
        history("valve_flow_history_m3_s", "m^3/s", "Flow through the valve at each sample."),
        history("valve_cavity_history_m3", "m^3", "Volume of gas and vapour at the valve at each sample."),
        history("kinetic_energy_history_j", "J", "Kinetic energy of the oil at each sample."),
        history("oil_compression_energy_history_j", "J", "Elastic energy of the oil's compression at each sample, from the supply pressure."),
        history("wall_strain_energy_history_j", "J", "Elastic energy of the wall's strain at each sample, from the supply pressure."),
        history("cavity_energy_history_j", "J", "Stored energy of the air and vapour at the nodes at each sample, from the supply pressure."),
        history("friction_loss_history_j", "J", "Friction's loss so far at each sample."),
        history("valve_work_history_j", "J", "The work through the valve so far at each sample."),
        history("cavity_loss_history_j", "J", "The cavity loss so far at each sample."),
    ],
    invariants: &[
        one("largest_energy_ledger_residual", "1", "Largest, over every step, of the stored energy plus friction's loss, the valve's work and the cavity loss, less the energy at the start, over the energy scale."),
        one("largest_volume_ledger_residual", "1", "Largest, over every step, of the oil stored by compression less the cavities, less what the supply put in and plus what the valve let out, against the start, over the volume scale."),
    ],
    iterations: "Steps of the grid, each one time step: every section's friction and every node's cavity settled once. On a refusal it is null.",
    field: field_words,
    answer: "a transient",
    refusals: REFUSALS,
    door_refusals: &[],
    sections,
    validation: Some(validation),
    example,
    further_examples: &[
        crate::solvers::FurtherExample { key: "example_solve_column_separation", parameters: example_separation },
        crate::solvers::FurtherExample { key: "example_solve_aerated", parameters: example_aerated },
    ],
    parts: &[],
    check,
    solve,
};

const SUMMARY: &str = "Water hammer in the company's own specimen, a 4.0 m steel hydraulic line (12 x 1.5 mm tube, 9 mm bore) of mineral oil (870 kg/m^3, 1.72 GPa) between a held supply and a valve that closes: the steady flow, then the transient by the method of characteristics on 40 sections with Churchill's friction, free air lumped at the nodes as isothermal gas (the wave speed falls by Wood's relation) and column separation where the pressure would fall below the vapour pressure; the Joukowsky rise as tangent and chord of the gas law; energy (kinetic, oil compression, wall strain, cavities, friction, valve) and volume ledgers closed at every step; stops by name at the tube's first yield (51.41 MPa gauge), at Mach 0.1 and where the free air passes a void fraction of 0.25. Its method was run on two measured lines with their own constants. On the Gdansk copper water rig, given the rig's measured wave speeds, it gave the free-oscillation period within 0.28 percent in four runs; its own wave speed law is 6.1 to 8.6 percent high there, and with it the period is 5.5 to 7.7 percent short. On DLR's liquid-nitrogen line Joukowsky's closed form missed the mark posted for it (6 of 9 within 5 percent, low by up to 6.9 percent); that line is served, and was scored blind, as ln2_line_dlr_fttf.";

/// Each `HammerReject` variant with the sentence from its doc comment. A
/// refusal the request itself shows is not sealed.
const REFUSALS: &[Refusal] = &[
    Refusal { name: "InputNotFinite", meaning: "An input was NaN or infinite.", sealed: false },
    Refusal { name: "SupplyAboveWallYield", meaning: "The supply pressure is above the pressure at which the tube's bore first yields, `WALL_YIELD_PRESSURE_PA`, 51.41 MPa over the atmosphere outside.", sealed: false },
    Refusal { name: "ReturnNotAboveVapour", meaning: "The return pressure is not above the oil's vapour pressure, `OIL_VAPOUR_PRESSURE_PA`.", sealed: false },
    Refusal { name: "ReturnNotBelowSupply", meaning: "The return pressure is not below the supply pressure: no flow runs toward the valve.", sealed: false },
    Refusal { name: "VelocityNegative", meaning: "The initial velocity is negative: the line runs from the supply to the valve.", sealed: false },
    Refusal { name: "DurationOutOfRange", meaning: "The duration is not above zero or is above `DURATION_MAX_S`.", sealed: false },
    Refusal { name: "ClosureTimeOutOfRange", meaning: "The closure time is negative or longer than the duration.", sealed: false },
    Refusal { name: "FreeAirNegative", meaning: "The free air fraction is negative.", sealed: false },
    Refusal { name: "ClosureFasterThanGrid", meaning: "With free air in the oil the closure is faster than `CLOSURE_STEPS_MIN_WITH_AIR` steps of the grid, which it does not resolve.", sealed: false },
    Refusal { name: "FreeAirAboveBubblyFlow", meaning: "The free air at the valve's steady pressure is a void fraction above `BUBBLY_VOID_LIMIT`: past it the air and oil no longer flow as bubbles (Taitel, Bornea and Dukler 1980).", sealed: false },
    Refusal { name: "SupplyCannotDriveFlow", meaning: "Friction along the line takes the whole supply pressure before the valve: the steady pressure at the valve is not above the return pressure.", sealed: false },
    Refusal { name: "InitialFlowMachAboveAcoustic", meaning: "The steady flow is faster than `MACH_LIMIT` of Wood's wave speed at the valve: the line equations, which drop the flow's own convection, do not hold there.", sealed: false },
    Refusal { name: "WallYield", meaning: "The transient reached the pressure at which the tube's bore first yields: past it the wall is not elastic.", sealed: true },
    Refusal { name: "LineMachAboveAcoustic", meaning: "The flow at a node reached `MACH_LIMIT` of the local wave speed of the oil and its air during the transient.", sealed: true },
    Refusal { name: "LineVoidAboveBubblyFlow", meaning: "The free air at a node expanded, as the pressure there fell, to a void fraction above `BUBBLY_VOID_LIMIT` during the transient: past it the air and oil no longer flow as bubbles.", sealed: true },
    Refusal { name: "CavityLargerThanReach", meaning: "A node's cavity of gas and vapour grew past the oil it is lumped from: between two sections larger than its reach, at the valve longer than one section, so that the separated column has left the grid.", sealed: true },
    Refusal { name: "FrictionNotSettled", meaning: "The friction of a section did not settle in `MAX_FRICTION_PASSES` passes.", sealed: true },
    Refusal { name: "NodeNotSettled", meaning: "A node's cavity did not settle in `MAX_NODE_PASSES` passes of false position and `MAX_BISECTION_PASSES` of halving.", sealed: true },
    Refusal { name: "NonFiniteValue", meaning: "A value of the transient was NaN or infinite.", sealed: true },
    Refusal { name: "EnergyLedgerBroken", meaning: "The energy ledger did not close to `ENERGY_RESIDUAL_LIMIT` of the energy scale at a step.", sealed: true },
    Refusal { name: "VolumeLedgerBroken", meaning: "The volume ledger did not close to `VOLUME_RESIDUAL_LIMIT` of the volume scale at a step.", sealed: true },
    Refusal { name: "FrictionGaveEnergy", meaning: "A section's friction gave energy to the flow.", sealed: true },
    Refusal { name: "CavityGaveEnergy", meaning: "A node's cavity gave energy to the waves.", sealed: true },
];

/// The `HammerReject` variant name, without its fields.
pub fn reject_name(reject: &HammerReject) -> &'static str {
    match reject {
        HammerReject::InputNotFinite { .. } => "InputNotFinite",
        HammerReject::SupplyAboveWallYield { .. } => "SupplyAboveWallYield",
        HammerReject::ReturnNotAboveVapour { .. } => "ReturnNotAboveVapour",
        HammerReject::ReturnNotBelowSupply { .. } => "ReturnNotBelowSupply",
        HammerReject::VelocityNegative { .. } => "VelocityNegative",
        HammerReject::DurationOutOfRange { .. } => "DurationOutOfRange",
        HammerReject::ClosureTimeOutOfRange { .. } => "ClosureTimeOutOfRange",
        HammerReject::FreeAirNegative { .. } => "FreeAirNegative",
        HammerReject::ClosureFasterThanGrid { .. } => "ClosureFasterThanGrid",
        HammerReject::FreeAirAboveBubblyFlow { .. } => "FreeAirAboveBubblyFlow",
        HammerReject::SupplyCannotDriveFlow { .. } => "SupplyCannotDriveFlow",
        HammerReject::InitialFlowMachAboveAcoustic { .. } => "InitialFlowMachAboveAcoustic",
        HammerReject::WallYield { .. } => "WallYield",
        HammerReject::LineMachAboveAcoustic { .. } => "LineMachAboveAcoustic",
        HammerReject::LineVoidAboveBubblyFlow { .. } => "LineVoidAboveBubblyFlow",
        HammerReject::CavityLargerThanReach { .. } => "CavityLargerThanReach",
        HammerReject::FrictionNotSettled { .. } => "FrictionNotSettled",
        HammerReject::NodeNotSettled { .. } => "NodeNotSettled",
        HammerReject::NonFiniteValue { .. } => "NonFiniteValue",
        HammerReject::EnergyLedgerBroken { .. } => "EnergyLedgerBroken",
        HammerReject::VolumeLedgerBroken { .. } => "VolumeLedgerBroken",
        HammerReject::FrictionGaveEnergy { .. } => "FrictionGaveEnergy",
        HammerReject::CavityGaveEnergy { .. } => "CavityGaveEnergy",
    }
}

fn field_words() -> String {
    "SHA-256 of the count of the numbers that follow as 8 little-endian bytes, then the absolute pressure at the valve at the start and after every step, then the absolute pressure at every node at the end from the supply to the valve, then the velocity at every node at the end in the same order, all as little-endian f64 bytes.".to_string()
}

fn sections() -> Sections {
    Sections {
        locked: json!({
            "specimen": "The company's own specimen: 4.0 m of seamless steel tube, 12 mm outside and 1.5 mm wall (9 mm bore, E235, Young's modulus 210 GPa, Poisson 0.3, bore roughness 1.5 um), anchored against axial movement, level, from a supply held at one pressure to a valve whose open area falls linearly to zero; filled with the mineral oil of the hydraulic_fluid card (870 kg/m^3, bulk modulus 1.72 GPa, vapour pressure 10 Pa) at the viscosity of ISO VG 32 at 40 degC, 32 mm^2/s.",
            "line_length_m": hammer::LINE_LENGTH_M,
            "bore_m": hammer::BORE_M,
            "wall_thickness_m": hammer::WALL_THICKNESS_M,
            "oil_density_kg_m3": hammer::OIL_DENSITY_KG_M3,
            "oil_bulk_modulus_pa": hammer::OIL_BULK_MODULUS_PA,
            "oil_kinematic_viscosity_m2_s": hammer::OIL_KINEMATIC_VISCOSITY_M2_S,
            "oil_vapour_pressure_pa": hammer::OIL_VAPOUR_PRESSURE_PA,
            "wall_yield_pressure_pa": hammer::WALL_YIELD_PRESSURE_PA,
            "sections": hammer::SECTIONS,
            "mach_limit": hammer::MACH_LIMIT,
            "bubbly_void_limit": hammer::BUBBLY_VOID_LIMIT,
            "equations": "In each section rho dV/dt + dp/dx = -rho f V|V|/(2D) and dp/dt + rho a^2 dV/dx = 0, with 1/(rho a^2) = 1/K + D psi/(E e) and psi = 2(e/D)(1 + nu) + D(1 - nu^2)/(D + e) (thick-walled, anchored against axial movement; Wylie and Streeter 1993), a = 1365.9 m/s. C+ = p + ZQ and C- = p - ZQ, Z = rho a/A, are carried one section per step with nothing interpolated. f is Churchill's (1977) friction factor, one formula for laminar, transitional and turbulent flow; each section's loss is delta = dx F(V_mid) at the velocity halfway through it, so friction's energy is force times velocity and never negative. Free air: at each node an isothermal gas cavity p V = alpha0 p_atm V_reach (Wylie's discrete gas cavity model); vapour: where the pressure would fall below 10 Pa a vapour cavity at 10 Pa (the discrete vapour cavity model); a node takes its cavity's pressure at the step's end, the weighting of one. Wood's relation, reported: 1/(rho_m a_w^2) = (1 - alpha)/K + alpha/p + D psi/(E e).",
            "fitted": "Nothing. Every constant is the specimen's declared value or a published law's.",
        }),
        invariants: json!([
            {
                "name": "largest_energy_ledger_residual",
                "rule": "Energy is counted from the line at rest at the supply pressure. The waves in flight hold dt (C+^2 + C-^2)/(4Z) a section (kinetic dt (C+ - C-)^2/(8Z), elastic dt (C+ + C-)^2/(8Z), split between the oil and the wall by their shares of the compliance), the nodes the stored energy of their air and vapour. That, plus friction's loss, the valve's work and the cavity loss summed step by step, equals the energy at the start. The supply does no work in this count.",
                "tolerance": line::ENERGY_RESIDUAL_LIMIT,
                "scale": "The line's volume times (rho V0^2/2 + (p_supply - p_vapour)^2/(2 rho a^2)), or, where that is smaller, 1e-5 of the sizes of the terms the ledger adds at that step (every section's waves, every node's terms, what has gone): a line creeping just over the vapour pressure shrinks the first and not the second, and below it the ledger would judge their rounding, not the physics.",
                "compared": "every step",
                "refusal": "EnergyLedgerBroken",
            },
            {
                "name": "largest_volume_ledger_residual",
                "rule": "The waves hold dt (C+ + C-)/(2Z) a section, the oil stored by compression over the line at the supply pressure; the cavities take volume from the oil. Stored oil less cavities rises by what the supply put in and falls by what the valve let out.",
                "tolerance": line::VOLUME_RESIDUAL_LIMIT,
                "scale": "The line's volume times ((p_supply - p_vapour)/(rho a^2) + alpha0), or, where that is smaller, 1e-5 of the sizes of the volumes the ledger adds at that step.",
                "compared": "every step",
                "refusal": "VolumeLedgerBroken",
            },
            {
                "name": "second law",
                "rule": "Each section's friction takes energy and never gives it, and each node's cavity update takes energy from the waves and never gives it: held to a part in 1e9 of the energy scale, or to the rounding of the node's own terms (its pressure times its volumes, its energy's terms) where that is more, and to what its root leaves unsettled, and to nothing else.",
                "compared": "every section and node of every step",
                "refusal": "FrictionGaveEnergy or CavityGaveEnergy",
            },
        ]),
        convergence: json!({
            "rule": "A transient is a march of fixed steps, one section's crossing time each. Each section's friction is a fixed point settled to a part in 1e13 (it contracts by under 1e-3 a pass); a node with air between two sections is the root of a quadratic; the valve's node is found by false position on a function that rises strictly, and past its passes by halving to the bracket's last bit. A node at rest at the supply pressure whose flows balance there keeps that state exactly, so a line at rest stays at rest to the bit. Nothing is returned that has not held at every step.",
            "time_step_s": hammer::time_step_s(),
            "friction_settled": line::FRICTION_SETTLED,
            "max_friction_passes": line::MAX_FRICTION_PASSES,
            "max_node_passes": line::MAX_NODE_PASSES,
            "max_bisection_passes": line::MAX_BISECTION_PASSES,
            "refusal": "FrictionNotSettled or NodeNotSettled",
            "run_time": "About 0.26 s for a run of 1 s with every history, and 0.2 s without, on one core of an Apple M4 Pro, with or without air.",
        }),
        reference: json!({
            "name": hammer::REFERENCE_NAME,
            "url": hammer::REFERENCE_URL,
            "retrieved": hammer::REFERENCE_RETRIEVED,
            "test": "gdansk_free_oscillation_period",
            "covers": hammer::GDANSK_FINDINGS,
            "second_name": "S. Klein, T. Traudt and J. Deeken, Liquid Nitrogen Fluid Hammer: PS-LN2-Set1, DLR Institute of Space Propulsion, Zenodo 15526459 (CC BY 4.0), with nitrogen's properties from the NIST Chemistry WebBook (Span et al. 2000)",
            "second_url": "https://doi.org/10.5281/zenodo.15526459",
            "second_retrieved": "2026-10-14",
            "second_tests": "dlr_first_surge_by_joukowsky, dlr_first_surge_by_the_transient, dlr_front_speed",
            "second_covers": hammer::DLR_FINDINGS,
            "kind": "Measurements of two other lines, a copper water rig and a stainless liquid-nitrogen line, run through this solver's method and wave speed law with each line's own constants. This line's own comparison comes later. Record: evidence/research/hydraulics_robotics/.",
        }),
    }
}

fn validation() -> [String; 5] {
    [
        "The company's own specimen: 4.0 m of seamless steel tube, 12 x 1.5 mm, of mineral oil between a held supply and a valve that closes. The solver computes the steady flow and the transient of the closure, with free air and column separation.".to_string(),
        "The company's own specimen, its constants declared in its description; its own comparison with measurement comes later. Its method and its wave speed law were run, each with that line's own constants, on two measured lines: the copper water rig of Lewandowski and Adamkowski (2024) and the liquid-nitrogen line of DLR Lampoldshausen (Klein, Traudt and Deeken, Zenodo 15526459). What they found is under reference.".to_string(),
        "For this line, the ledgers stated under invariants: energy and volume closed at every step within 1e-9 of their scales, and friction and the cavities never giving energy. For the DLR line a mark was posted before it was scored: Joukowsky's rho a v0 with Korteweg's wave speed, nitrogen at the line thermometer's temperature, within 5 percent of the measured first surge in at least 8 of 9 tests.".to_string(),
        crate::descriptor::OWN_SPECIMEN_SOLVER_RESULT.to_string(),
        "The line's constants are the company's, declared in its description. The oil's modulus is one value at every pressure; friction is quasi-steady, which damps a water hammer more slowly than it is measured to damp; air neither leaves nor enters solution; the tube does not move; the line is level. The lumped air is damped at the grid's scale by the backward step of its cavities, and that energy is reported as cavity_loss_j: with 1 percent of air at 2 MPa it is 12 percent of the starting energy over the first 50 ms, against friction's 40. On the DLR line the closed form missed its posted mark (6 of 9 within 5 percent) and the transient, after the fact and with one fitted friction roughness, met it on all 9, read 1.10 m before the valve and with the record's pressures as absolute, where the sensor is at 0.60 m and the pressures are gauge (ln2_line_dlr_fttf reads both right); on the Gdansk rig the wave speed law was 6.1 to 8.6 percent high. A proof shows that an answer was not changed, not that the model matches a line.".to_string(),
    ]
}

fn example() -> Value {
    json!({
        "supply_pressure_pa": 10.0e6,
        "return_pressure_pa": 0.2e6,
        "initial_velocity_m_s": 2.0,
        "closure_time_s": 0.001,
        "free_air_fraction": 0.0,
        "duration_s": 0.05,
        "return_histories": false
    })
}

fn example_separation() -> Value {
    json!({
        "supply_pressure_pa": 1.0e6,
        "return_pressure_pa": 0.2e6,
        "initial_velocity_m_s": 1.5,
        "closure_time_s": 0.0,
        "free_air_fraction": 0.0,
        "duration_s": 0.2,
        "return_histories": true
    })
}

fn example_aerated() -> Value {
    json!({
        "supply_pressure_pa": 2.0e6,
        "return_pressure_pa": 0.2e6,
        "initial_velocity_m_s": 1.0,
        "closure_time_s": 0.001,
        "free_air_fraction": 0.01,
        "duration_s": 0.05,
        "return_histories": false
    })
}

/// The solver's own question, from the values of `parameters`.
fn question(given: &Given) -> Result<HammerQuestion, String> {
    Ok(HammerQuestion {
        supply_pressure_pa: given.number("supply_pressure_pa")?,
        return_pressure_pa: given.number("return_pressure_pa")?,
        initial_velocity_m_s: given.number("initial_velocity_m_s")?,
        closure_time_s: given.number("closure_time_s")?,
        free_air_fraction: given.number("free_air_fraction")?,
        duration_s: given.number("duration_s")?,
    })
}

/// Every limit the request shows, asked of the solver's own check before a
/// physics permit is taken.
fn check(given: &Given) -> Result<(), String> {
    hammer::admit(&question(given)?).map_err(|reject| format!("Physical Violation: {reject} ({}).", reject_name(&reject)))
}

fn solve(given: &Given) -> Result<Solved, String> {
    let histories = given.flag(ASK_HISTORIES)?;
    Ok(solved(hammer::solve(&question(given)?), histories))
}

/// What the solver returned, in the words every solver shares.
pub fn solved(returned: Result<Hammer, HammerReject>, histories: bool) -> Solved {
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
    let mut result = vec![
        one("liquid_wave_speed_m_s", h.liquid_wave_speed_m_s),
        one("wood_wave_speed_m_s", h.wood_wave_speed_m_s),
        one("void_fraction_at_valve", h.void_fraction_at_valve),
        one("joukowsky_rise_liquid_pa", h.joukowsky_rise_liquid_pa),
        one("joukowsky_rise_wood_pa", h.joukowsky_rise_wood_pa),
        one("joukowsky_rise_chord_pa", h.joukowsky_rise_chord_pa),
        one("round_trip_time_s", h.round_trip_time_s),
        one("time_step_s", h.time_step_s),
        one("steady_valve_pressure_pa", h.steady_valve_pressure_pa),
        one("valve_pressure_first_rise_pa", h.valve_pressure_first_rise_pa),
        one("valve_pressure_max_pa", h.valve_pressure_max_pa),
        one("valve_pressure_max_time_s", h.valve_pressure_max_time_s),
        one("valve_pressure_min_pa", h.valve_pressure_min_pa),
        one("line_pressure_max_pa", h.line_pressure_max_pa),
        one("line_pressure_min_pa", h.line_pressure_min_pa),
        one("cavity_volume_max_m3", h.cavity_volume_max_m3),
        one("largest_line_mach", h.largest_line_mach),
        many("first_vapour_time_s", &h.first_vapour_time_s),
        many("first_vapour_position_m", &h.first_vapour_position_m),
        many("node_positions_m", &h.node_positions_m),
        many("pressure_max_along_line_pa", &h.pressure_max_along_line_pa),
        many("pressure_min_along_line_pa", &h.pressure_min_along_line_pa),
        one("stored_energy_start_j", h.stored_energy_start_j),
        one("stored_energy_end_j", h.stored_energy_end_j),
        one("friction_loss_j", h.friction_loss_j),
        one("valve_work_j", h.valve_work_j),
        one("cavity_loss_j", h.cavity_loss_j),
    ];
    if histories {
        result.extend([
            many("sample_times_s", &h.sample_times_s),
            many("valve_pressure_history_pa", &h.valve_pressure_history_pa),
            many("midpoint_pressure_history_pa", &h.midpoint_pressure_history_pa),
            many("valve_flow_history_m3_s", &h.valve_flow_history_m3_s),
            many("valve_cavity_history_m3", &h.valve_cavity_history_m3),
            many("kinetic_energy_history_j", &h.kinetic_energy_history_j),
            many("oil_compression_energy_history_j", &h.oil_compression_energy_history_j),
            many("wall_strain_energy_history_j", &h.wall_strain_energy_history_j),
            many("cavity_energy_history_j", &h.cavity_energy_history_j),
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

    const SOLVER_SOURCE: &str = include_str!("../../genesis_solvers/src/water_hammer_oil_line.rs");

    /// `(variant, doc sentence)` for every variant of `HammerReject`, read
    /// from the solver's source. An intra-doc link is written as its bare
    /// code span.
    fn documented_rejects() -> Vec<(String, String)> {
        let start = SOLVER_SOURCE.find("pub enum HammerReject {").expect("HammerReject is in the solver");
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
    fn the_reference_is_the_one_stored_in_the_solver_test() {
        let body = describe_value(WATER_HAMMER_OIL_LINE.name).expect("described");
        let url = body["reference"]["url"].as_str().expect("url");
        assert!(SOLVER_SOURCE.contains(&format!("<{url}>")), "{url}");
        let retrieved = body["reference"]["retrieved"].as_str().expect("retrieved");
        assert!(SOLVER_SOURCE.contains(&format!("Retrieved\n//! {retrieved}")) || SOLVER_SOURCE.contains(&format!("Retrieved {retrieved}")), "{retrieved}");
        let test = body["reference"]["test"].as_str().expect("test");
        assert!(SOLVER_SOURCE.contains(&format!("fn {test}()")), "{test}");
    }

    #[test]
    fn every_published_name_is_in_the_entry_and_the_histories_only_when_asked() {
        let q = HammerQuestion { supply_pressure_pa: 10.0e6, return_pressure_pa: 0.2e6, initial_velocity_m_s: 2.0, closure_time_s: 0.001, free_air_fraction: 0.0, duration_s: 0.01 };
        for histories in [false, true] {
            let answer = match solved(hammer::solve(&q), histories) {
                Solved::Answer(answer) => answer,
                Solved::Refused(refused) => panic!("{}", refused.sentence),
            };
            let listed: Vec<&Output> = WATER_HAMMER_OIL_LINE.result.iter().filter(|o| histories || o.asked_by.is_none()).collect();
            assert_eq!(answer.result.len(), listed.len());
            for ((name, value), output) in answer.result.iter().zip(listed) {
                assert_eq!(name, output.name);
                assert_eq!(value.shape(), output.shape);
            }
            for ((name, _), output) in answer.invariants.iter().zip(WATER_HAMMER_OIL_LINE.invariants) {
                assert_eq!(name, output.name);
            }
        }
    }
}
