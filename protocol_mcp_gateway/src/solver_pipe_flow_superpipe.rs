//! The entry for `pipe_flow_superpipe`: fully developed, smooth-wall,
//! turbulent air flow in the Princeton Superpipe. The numbers come from
//! `genesis_solvers` constants.

use genesis_solvers::pipe_flow_superpipe::{self as pipe, PipeFlow, PipeQuestion, PipeReject};
use serde_json::{Value, json};

use crate::solvers::{
    Answer, Given, Input, Kind, Output, Published, Refusal, Refused, Sections, Shape, Solved,
    Solver,
};

const fn output(name: &'static str, unit: &'static str, shape: Shape, note: &'static str) -> Output {
    Output {
        name,
        unit,
        shape,
        each: &[],
        note,
        asked_by: None,
    }
}

pub const PIPE_FLOW_SUPERPIPE: Solver = Solver {
    name: "pipe_flow_superpipe",
    summary: SUMMARY,
    inputs: &[
        // Every limit is asked by the solver's own check, in its order: the
        // Reynolds number below the range has two names, and the Mach number
        // spans three fields.
        Input {
            name: "bulk_reynolds_number",
            kind: Kind::Number {
                minimum: pipe::TRANSITIONAL_BELOW_REYNOLDS,
                maximum: pipe::REYNOLDS_MAX,
                unit: "1",
            },
            refusal: Some("ReynoldsAboveGrid"),
            bound: "Bulk velocity times the bore (129.36 mm) over the kinematic viscosity, answered for fully turbulent flow from 4000 to 10,000,000,000. Below 2300 the flow is laminar and is refused as LaminarFlow; from 2300 to below 4000 it is transitional and is refused as TransitionalFlow; above 1e10 the grid's first node leaves the viscous sublayer and it is refused as ReynoldsAboveGrid. The equations depend on the Reynolds number alone. The Superpipe profiles compared ran from 31,577 (Zagarola and Smits 1998) to 35,724,000 (McKeon et al. 2004): friction rms 1.38 percent, the friction factor falling below the measured one as the Reynolds number rises, 2.7 percent low at 35.7 million. The held-back comparison starts at 74,345: below it the comparison is only with the opened Zagarola-Smits set used to choose the outer shape (friction rms 1.31 percent, where Prandtl-Karman's 0.77 was better). Below 31,577 and above 35,724,000 nothing was compared.",
            only_when: None,
            asked_by_check: true,
        },
        Input {
            name: "static_pressure_pa",
            kind: Kind::NumberAbove {
                above: 0.0,
                maximum: f64::MAX,
                unit: "Pa",
            },
            refusal: Some("PressureNotPositive"),
            bound: "Absolute static pressure of the air in the pipe, above zero (PressureNotPositive otherwise). It does not enter the solve: with the temperature it gives the density of ideal-gas air, the bulk velocity and its Mach number, and a bulk Mach number above 0.3 is refused as MachAboveIncompressible. The compared runs were at 100,090 to 18,928,000 Pa, at bulk velocities of about 3.9 to 30.8 m/s; the lowest pressure, and every pressure below the 101,325 Pa of NSTAP case 1, rests only on the opened set used to choose the outer shape. At the Superpipe's highest pressures air is about 3 percent denser than ideal and more viscous, so the Mach number given there is low by up to about a fifth.",
            only_when: None,
            asked_by_check: true,
        },
        Input {
            name: "temperature_k",
            kind: Kind::Number {
                minimum: pipe::AIR_VISCOSITY_LAW_K.0,
                maximum: pipe::AIR_VISCOSITY_LAW_K.1,
                unit: "K",
            },
            refusal: Some("TemperatureOutsideViscosityLaw"),
            bound: "Temperature of the air, 170 to 1900 K, where Sutherland's law gives its viscosity within 2 percent (F. M. White, Viscous Fluid Flow, Table 1-2); outside it the bulk velocity cannot be known and it is refused as TemperatureOutsideViscosityLaw. It does not enter the solve, only the Mach number. The compared runs were at 291.79 to 300.13 K; above 298.4 K, the warmest held-back run, the temperature rests only on the opened set used to choose the outer shape.",
            only_when: None,
            asked_by_check: true,
        },
        Input {
            name: "wall_roughness_rms_um",
            kind: Kind::Number {
                minimum: 0.0,
                maximum: pipe::WALL_ROUGHNESS_RMS_MAX_UM,
                unit: "um",
            },
            refusal: Some("RoughWall"),
            bound: "Root-mean-square roughness of the wall. The Superpipe's smooth wall is 0.15 um rms (Zagarola and Smits 1998), and every compared run was on it; a smoother wall is answered as hydraulically smooth. A rougher wall is a rough-wall flow, refused as RoughWall; a negative roughness is RoughnessNegative.",
            only_when: None,
            asked_by_check: true,
        },
    ],
    kept_inside: "The bore, the closure and its constants (kappa 0.41, A+ 26, Nikuradse's outer shape), the grid and the tolerances stay inside the solver.",
    result: &[
        output("darcy_friction_factor", "1", Shape::Number, "Pressure drop per length times the bore over the bulk dynamic pressure, rho U_b^2 / 2. Compared with measurement (line P1, met)."),
        output("centreline_to_bulk_velocity_ratio", "1", Shape::Number, "Velocity on the axis over the bulk velocity. Compared with measurement (line P2, missed at one of 21 profiles)."),
        output("friction_to_bulk_velocity_ratio", "1", Shape::Number, "Friction velocity over the bulk velocity, sqrt(f/8), from the friction factor."),
        output("karman_number", "1", Shape::Number, "Radius in viscous units, u_tau R / nu, from the friction factor and the Reynolds number."),
        output("pressure_drop_per_m_over_bulk_dynamic_pressure", "m^-1", Shape::Number, "f / D: the pressure drop per metre is this times rho U_b^2 / 2, with the caller's density and bulk velocity."),
        output("bulk_mach_number", "1", Shape::Number, "Bulk velocity over the speed of sound of ideal-gas air with Sutherland's viscosity at the asked pressure and temperature: Re mu / (rho D) over sqrt(1.4 R T). At most 0.3."),
        output("profile_y_over_radius", "1", Shape::Numbers, "Wall distance over the radius of each profile station: 25 fixed stations from 0.0002 to 1 (the axis), less those nearer the wall than was compared at this Reynolds number: below 50 viscous units up to 5,975,000 (the highest NSTAP profile), and above it below 0.010591 of the radius, the nearest compared point of the Pitot profiles (y+ 1054 at 6.11 million, 5444 at 35.7 million)."),
        output("profile_y_plus", "1", Shape::Numbers, "The same distances in viscous units, each at least 50, and above Reynolds number 5,975,000 at least the y+ of 0.010591 of the radius."),
        output("profile_u_plus", "1", Shape::Numbers, "Mean velocity over the friction velocity at each station. Compared with measurement (line P3, met)."),
        output("profile_u_over_bulk", "1", Shape::Numbers, "Mean velocity over the bulk velocity at each station; the last is the centreline ratio."),
    ],
    invariants: &[
        output("largest_imbalance", "1", Shape::Number, ""),
        output("flux_residual", "1", Shape::Number, ""),
        output("momentum_ledger_residual", "1", Shape::Number, ""),
    ],
    iterations: "Newton iterations, each one bordered tridiagonal solve of the whole radius. On a refusal it is the count reached, for NotSettled, and null otherwise.",
    field: field_words,
    answer: "a settled flow",
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

const SUMMARY: &str = "Fully developed, smooth-wall, turbulent air flow in the Princeton Superpipe (129.36 mm bore), a mixing length with van Driest damping with nothing fitted (kappa 0.41, A+ 26): Darcy friction factor, centreline-to-bulk velocity ratio and the mean velocity profile, answered for fully turbulent flow at bulk Reynolds number 4000 to 1e10 and bulk Mach number up to 0.3, laminar, transitional, rough-wall and compressible flow refused by name; compared at bulk Reynolds number 31,577 to 35,724,000 and compared once with held-back Superpipe measurements of A. J. Smits' group: the friction line met (rms 1.38 percent, worst 2.69, against Prandtl-Karman's 2.17 rms) and the profile line met (1.59 percent), the centreline line missed at one of 21 profiles (2.99 percent low against 2 percent, the NSTAP profile at 81,330), so by the rule fixed beforehand the comparison failed; the friction factor falls below the measured one as the Reynolds number rises, 2.7 percent low at 35.7 million.";

/// Each `PipeReject` variant with the sentence from its doc comment. A
/// refusal the request itself shows is not sealed.
const REFUSALS: &[Refusal] = &[
    Refusal {
        name: "InputNotFinite",
        meaning: "An input was NaN or infinite.",
        sealed: false,
    },
    Refusal {
        name: "LaminarFlow",
        meaning: "The bulk Reynolds number is below `LAMINAR_BELOW_REYNOLDS`: the flow is laminar, and this solver is of turbulent flow.",
        sealed: false,
    },
    Refusal {
        name: "TransitionalFlow",
        meaning: "The bulk Reynolds number is from `LAMINAR_BELOW_REYNOLDS` to below `TRANSITIONAL_BELOW_REYNOLDS`: the flow is transitional.",
        sealed: false,
    },
    Refusal {
        name: "ReynoldsAboveGrid",
        meaning: "The bulk Reynolds number is above `REYNOLDS_MAX`, where the grid's first node leaves the viscous sublayer.",
        sealed: false,
    },
    Refusal {
        name: "RoughnessNegative",
        meaning: "The wall roughness is negative.",
        sealed: false,
    },
    Refusal {
        name: "RoughWall",
        meaning: "The wall is rougher than the Superpipe's `WALL_ROUGHNESS_RMS_MAX_UM`: a rough-wall flow, which this solver is not of.",
        sealed: false,
    },
    Refusal {
        name: "PressureNotPositive",
        meaning: "The static pressure is not above zero: no gas.",
        sealed: false,
    },
    Refusal {
        name: "TemperatureOutsideViscosityLaw",
        meaning: "The temperature is outside 170 to 1900 K (`AIR_VISCOSITY_LAW_K`), where Sutherland's law gives the viscosity of air, so the bulk velocity cannot be known.",
        sealed: false,
    },
    Refusal {
        name: "MachAboveIncompressible",
        meaning: "The bulk Mach number is above `MACH_MAX`: the flow is compressible, and this solver's density is constant.",
        sealed: false,
    },
    Refusal {
        name: "NonFiniteValue",
        meaning: "A value of the field or its imbalance was NaN or infinite.",
        sealed: true,
    },
    Refusal {
        name: "SingularSystem",
        meaning: "The linear system of a Newton iteration had no usable pivot.",
        sealed: true,
    },
    Refusal {
        name: "ImbalanceGrew",
        meaning: "The largest imbalance rose above `IMBALANCE_CAP`.",
        sealed: true,
    },
    Refusal {
        name: "NotSettled",
        meaning: "The flow had not settled after `MAX_NEWTON_ITERATIONS` iterations.",
        sealed: true,
    },
    Refusal {
        name: "FluxBroken",
        meaning: "The volume flux departed from the asked one by more than `FLUX_TOLERANCE` after a Newton iteration.",
        sealed: true,
    },
    Refusal {
        name: "MomentumLedgerBroken",
        meaning: "The shear force on the grid face nearest the wall departs from the pressure force it encloses by more than `MOMENTUM_LEDGER_TOLERANCE`.",
        sealed: true,
    },
    Refusal {
        name: "VelocityNotMonotone",
        meaning: "The settled velocity does not rise from the wall to the axis.",
        sealed: true,
    },
];

/// The `PipeReject` variant name, without its fields.
pub fn reject_name(reject: &PipeReject) -> &'static str {
    match reject {
        PipeReject::InputNotFinite { .. } => "InputNotFinite",
        PipeReject::LaminarFlow { .. } => "LaminarFlow",
        PipeReject::TransitionalFlow { .. } => "TransitionalFlow",
        PipeReject::ReynoldsAboveGrid { .. } => "ReynoldsAboveGrid",
        PipeReject::RoughnessNegative { .. } => "RoughnessNegative",
        PipeReject::RoughWall { .. } => "RoughWall",
        PipeReject::PressureNotPositive { .. } => "PressureNotPositive",
        PipeReject::TemperatureOutsideViscosityLaw { .. } => "TemperatureOutsideViscosityLaw",
        PipeReject::MachAboveIncompressible { .. } => "MachAboveIncompressible",
        PipeReject::NonFiniteValue => "NonFiniteValue",
        PipeReject::SingularSystem => "SingularSystem",
        PipeReject::ImbalanceGrew { .. } => "ImbalanceGrew",
        PipeReject::NotSettled { .. } => "NotSettled",
        PipeReject::FluxBroken { .. } => "FluxBroken",
        PipeReject::MomentumLedgerBroken { .. } => "MomentumLedgerBroken",
        PipeReject::VelocityNotMonotone { .. } => "VelocityNotMonotone",
    }
}

fn field_words() -> String {
    format!(
        "SHA-256 of the count of the numbers that follow as 8 little-endian bytes, which is twice the number of grid nodes ({} intervals, {} nodes), then the wall distance over the radius of every node from the wall to the axis, then the mean velocity over the bulk velocity at every node in the same order, all as little-endian f64 bytes.",
        pipe::GRID_INTERVALS,
        pipe::GRID_INTERVALS + 1
    )
}

fn sections() -> Sections {
    Sections {
        locked: json!({
            "diameter_m": pipe::DIAMETER_M,
            "equations": "Steady, axially uniform, incompressible mean flow in a round pipe: (1/r) d/dr [ r (nu + l^2 |dU/dr|) dU/dr ] = (1/rho) dp/dx, with no slip at the wall, symmetry on the axis and the bulk velocity fixed. Constant density and viscosity.",
            "mixing_length": "l = kappa R [0.35 - 0.20 (1 - y/R)^2 - 0.15 (1 - y/R)^4] (1 - exp(-y+/A+)): Nikuradse's pipe mixing length (0.14, 0.08, 0.06 over R) rescaled by 0.41/0.40 to wall slope kappa, with van Driest's damping. The rescaled form is not a published one.",
            "kappa": pipe::KAPPA,
            "van_driest_a_plus": pipe::VAN_DRIEST_A_PLUS,
            "bulk_reynolds_number": "U_b D / nu, U_b the bulk (volume-averaged) velocity and D the bore.",
            "grid_intervals": pipe::GRID_INTERVALS,
            "grid_stretch": pipe::GRID_STRETCH,
            "grid": "Wall distance over the radius at node j of N: (exp(14 j/N) - 1)/(exp(14) - 1). Finite volumes about each node.",
            "fitted": "Nothing. kappa and A+ are the literature's. The outer shape was chosen on the opened set's friction factors among five forms with no fitted constant, four published and Nikuradse's distribution rescaled to wall slope kappa (chosen; 1.3 percent rms against 2.2 for the form as published, which had the better opened centreline error, 1.2 percent against 1.4), before any held-back file was opened. The grid was fixed against itself: from 1600 to 6400 intervals the friction factor moves by 1.1e-5 and the centreline ratio by 1.3e-5, relative, at either end of the compared range; doubling the grid moves the friction factor by 2.6e-5 at 1e10, the top served, where the first node is at y+ 0.78.",
            "mach_number": "Ideal-gas air, R 287.05 J/(kg K), ratio of heat capacities 1.4, Sutherland's viscosity 1.716e-5 Pa s (T/273 K)^1.5 (384 K)/(T + 111 K): bulk velocity Re mu R T / (p D), over sqrt(1.4 R T). It holds the incompressible answer to a bulk Mach number of 0.3, where the density of the air brought to rest changes by about 4.5 percent.",
            "solver_units": "The radius and the bulk velocity as one.",
        }),
        invariants: json!([
            {
                "name": "largest_imbalance",
                "rule": "At every node the shear stress carried across its two faces balances the pressure gradient over its volume: the momentum balance. The largest imbalance over every node, over the pressure force on the whole section. It says the equations were solved, not that they describe the air.",
                "tolerance": pipe::IMBALANCE_TOLERANCE,
                "cap": pipe::IMBALANCE_CAP,
                "unit": "1",
                "compared": "every Newton iteration",
                "refusal": "NonFiniteValue, ImbalanceGrew or NotSettled",
            },
            {
                "name": "flux_residual",
                "rule": "The volume flux of the settled profile equals the asked bulk velocity times the section. It is a linear constraint of every Newton step.",
                "tolerance": pipe::FLUX_TOLERANCE,
                "unit": "1",
                "compared": "after every Newton iteration",
                "refusal": "FluxBroken",
            },
            {
                "name": "momentum_ledger_residual",
                "rule": "The shear force on the grid face nearest the wall equals the pressure force on the section it encloses: wall shear times perimeter equals pressure drop times area. It is the sum of the node balances, so it holds their total to the tolerance where each node is held to the imbalance tolerance alone.",
                "tolerance": pipe::MOMENTUM_LEDGER_TOLERANCE,
                "unit": "pressure force on the section",
                "compared": "at acceptance",
                "refusal": "MomentumLedgerBroken",
            },
        ]),
        convergence: json!({
            "imbalance_tolerance": pipe::IMBALANCE_TOLERANCE,
            "friction_velocity_tolerance": pipe::FRICTION_VELOCITY_TOLERANCE,
            "max_newton_iterations": pipe::MAX_NEWTON_ITERATIONS,
            "rule": "Newton's method on every node's momentum balance and the volume flux, the pressure gradient the extra unknown; each iteration solves the tridiagonal Jacobian bordered by one row and one column directly. The van Driest damping takes the friction velocity of the previous iterate. A result is returned only when the largest imbalance is within its tolerance, the friction velocity changed by no more than its tolerance in the last iteration, the momentum ledger closes and the velocity rises from wall to axis.",
            "refusal": "NotSettled, ImbalanceGrew, SingularSystem, FluxBroken, MomentumLedgerBroken or VelocityNotMonotone",
            "run_time": "14 to 18 Newton iterations over the range, about 10 ms in a debug build.",
        }),
        reference: json!({
            "name": pipe::REFERENCE_NAME,
            "url": pipe::REFERENCE_URL,
            "retrieved": pipe::REFERENCE_RETRIEVED,
            "tolerance_fraction": 0.03,
            "test": "zagarola_smits_1998_friction_factors",
            "covers": "The Darcy friction factor at the twelve opened Reynolds numbers, 31,577 to 751,820: each within 3 percent; found rms 1.31 percent, worst 2.26 percent (at 31,577).",
            "kind": "Measurements of the specimen itself: the opened set, used to choose the closure's outer shape. The held-back comparison is in validation.",
        }),
    }
}

fn validation() -> [String; 5] {
    let object = "The Princeton Superpipe of A. J. Smits' group, Princeton University: a closed-loop compressed-air pipe flow facility whose test section has a bore of 129.36 mm and a smooth wall, 0.15 um rms, run from atmospheric pressure to about 19 MPa to reach bulk Reynolds numbers from 3.16e4 to 3.57e7. The solver settles the fully developed mean flow at one bulk Reynolds number with a mixing length and van Driest damping. Nothing is fitted.".to_string();
    let data = "Downloaded 2026-10-11 from the Smits group's pages (https://smits.princeton.edu/superpipe-pitot-data, https://smits.princeton.edu/zagarola/, https://smits.princeton.edu/mckeon/, https://smits.princeton.edu/superpipe-turbulence-data); the pages state no licence and carry the footer of the Trustees of Princeton University. Opened: the twelve Pitot profiles of M. V. Zagarola and A. J. Smits (J. Fluid Mech. 373, 1998; 0.9 mm probe) below Reynolds number 1e6, used to choose the outer shape of the mixing length. Held back until the protocol was fingerprinted: the 25 Pitot profiles of B. J. McKeon, J. Li, W. Jiang, J. F. Morrison and A. J. Smits (J. Fluid Mech. 501, 2004; 0.3 mm probe and three larger at two Reynolds numbers, with the McKeon-Smits static-pressure correction), 19 distinct Reynolds numbers 7.4e4 to 3.57e7, each file stating its friction factor, bulk and maximum velocities; the nine smooth-pipe NSTAP profiles of M. Hultmark, M. Vallikivi, S. C. C. Bailey and A. J. Smits (Phys. Rev. Lett. 108, 2012; J. Fluid Mech. 728, 2013), 8.1e4 to 6.0e6, whose friction factor is stated as a quarter of Darcy's; and the 14 Zagarola-Smits profiles above 1e6, reported but not gated. The McKeon and Zagarola-Smits headers state the measured pressure drop, from which their friction factors come; the NSTAP files state a friction velocity and a friction factor and do not state how the friction velocity was obtained. The profiles are first-hand, as tabulated by the group.".to_string();
    let pass_line = "Fixed on 2026-10-11 before any held-back file was opened (protocol SHA-256 704700a99d27cdd566d47a92182d363f31ad8d1ee774c1e75c84a8de75b26893). P1: on the McKeon 2004 friction factors, rms relative error at most 3 percent and worst at most 5 percent, and rms below the Prandtl-Karman law's. P2: on the McKeon profiles above 1e6 and every NSTAP profile, every centreline-to-bulk ratio within 2 percent, and rms below the log law integrated over the section (1 + 3.75 sqrt(f_PK/8)). P3: on the same profiles, at every point from y+ = 50 to the axis (and at least two Pitot diameters from the wall), pooled rms of u+ at most 2 percent, worst profile at most 3 percent, and pooled rms below the log law u+ = ln(y+)/0.40 + 5.5. The comparison passes only if all three do.".to_string();
    let limits = "Every compared run is of air in the one 129.36 mm smooth pipe at 291.8 to 300.1 K, at 100,090 to 18,928,000 Pa and bulk velocities of about 3.9 to 30.8 m/s; the runs lie along a line from low Reynolds number at low pressure to high at high pressure. The solver answers past that envelope wherever its equations hold: fully turbulent flow from Reynolds number 4000 to 1e10, any pressure above zero, 170 to 1900 K, up to a bulk Mach number of 0.3. Through engine 0.8.0 a question outside the compared Reynolds numbers, pressures, temperatures and bulk velocities was refused by name. Below 31,577 and above 35,724,000 nothing was compared, and the friction error, growing with the Reynolds number inside the compared range, is unmeasured there. The model has no compressibility, no roughness and no development length: the flow is fully developed and the density and viscosity constant. Its friction factor falls below the measured one as the Reynolds number rises (-0.7 percent at 1e6, -1.4 at 1e7, -2.7 at 3.57e7), so its error is largest at the top of the compared range. The question put to the data's authors is about the one missed line: whether the bulk velocity of NSTAP case 1 (Reynolds number 81,330) was measured independently or integrated from its profile, and how uncertain it is. The centreline ratio is low throughout, by 0.75 to 1.8 percent on 20 of 21 held-back profiles. Below Reynolds number 74,345, at the lowest pressure and at the highest temperature the only comparison is with the opened set that chose the outer shape. The profile is served only where it was compared: from 50 viscous units out up to Reynolds number 5,975,000, and above that from 0.010591 of the radius out (y+ 1054 at 6.11 million, 5444 at 35.7 million), the nearest point of the Pitot profiles. The ledgers show the equations were solved, not that they describe the air. A proof shows that an answer was not changed, not that the model matches the air.".to_string();
    [object, data, pass_line, RESULT.to_string(), limits]
}

/// The outcome of the comparison with the held-back measurements.
const RESULT: &str = "Compared once, on 2026-10-11, with the solver as fingerprinted (evidence/research/fluid_princeton_superpipe/analysis/out/03_scored.json). P1, friction, on 19 McKeon friction factors: rms 1.38 percent, worst 2.69 percent (at 35.7 million) against 3 and 5, and below the Prandtl-Karman law's rms 2.17 (worst 3.64): met. Colebrook's smooth law, reported beside it, rms 2.18; Blasius 23.4. P3, profile, on 827 points of 21 profiles: pooled rms of u+ 1.59 percent, worst profile 2.58 percent (NSTAP at 3.95 million) against 2 and 3, and below the log law's 3.59: met. P2, centreline, on the same 21 profiles: rms 1.33 percent, below the baseline's 2.10, but the NSTAP profile at 81,330 is 2.99 percent low against the 2 percent line; the other 20 are 0.75 to 1.79 percent low: missed, on one profile. By the rule fixed beforehand the comparison failed. That profile's measured ratio, 11.69/9.48 = 1.233, is above the McKeon profile at 74,345 (1.218) and the Zagarola-Smits profile at 74,293 (1.220). Reported, not gated: the 14 Zagarola-Smits profiles above 1e6, friction rms 2.75 percent, worst 4.52 (Prandtl-Karman 3.62, 5.45), centreline worst 1.21 percent, profile pooled 1.48 percent; the McKeon profiles at or below 1e6, centreline worst 2.02 percent, profile pooled 1.28 percent; the nine NSTAP friction factors, rms 1.13 percent. Opened set, not blind: friction rms 1.31 percent, where Prandtl-Karman's 0.77 was better.";

fn example() -> Value {
    json!({
        "bulk_reynolds_number": 1.0e6,
        "static_pressure_pa": 1.47e6,
        "temperature_k": 296.0,
        "wall_roughness_rms_um": 0.15
    })
}

/// The solver's own question, from the values of `parameters`.
fn question(given: &Given) -> Result<PipeQuestion, String> {
    Ok(PipeQuestion {
        bulk_reynolds_number: given.number("bulk_reynolds_number")?,
        static_pressure_pa: given.number("static_pressure_pa")?,
        temperature_k: given.number("temperature_k")?,
        wall_roughness_rms_um: given.number("wall_roughness_rms_um")?,
    })
}

/// Every limit the request shows, asked of the solver's own check before a
/// physics permit is taken. The sentence is the solver's, with its name for
/// the refusal.
fn check(given: &Given) -> Result<(), String> {
    pipe::admit(&question(given)?)
        .map_err(|reject| format!("Physical Violation: {reject} ({}).", reject_name(&reject)))
}

fn solve(given: &Given) -> Result<Solved, String> {
    Ok(solved(pipe::solve(&question(given)?)))
}

/// What the solver returned, in the words every solver shares.
pub fn solved(returned: Result<PipeFlow, PipeReject>) -> Solved {
    let flow = match returned {
        Ok(flow) => flow,
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
                    PipeReject::NotSettled { iterations, .. } => Some(u64::from(iterations)),
                    _ => None,
                },
            });
        }
    };
    let one = |name: &str, value: f64| (name.to_string(), Published::Number(value));
    let many = |name: &str, values: Vec<f64>| (name.to_string(), Published::Numbers(values));
    let mut field = Vec::with_capacity(flow.field_y_over_radius.len() * 2);
    field.extend_from_slice(&flow.field_y_over_radius);
    field.extend_from_slice(&flow.field_u_over_bulk);
    Solved::Answer(Answer {
        iterations: u64::from(flow.newton_iterations),
        result: vec![
            one("darcy_friction_factor", flow.darcy_friction_factor),
            one("centreline_to_bulk_velocity_ratio", flow.centreline_to_bulk_velocity_ratio),
            one("friction_to_bulk_velocity_ratio", flow.friction_to_bulk_velocity_ratio),
            one("karman_number", flow.karman_number),
            one(
                "pressure_drop_per_m_over_bulk_dynamic_pressure",
                flow.pressure_drop_per_m_over_bulk_dynamic_pressure,
            ),
            one("bulk_mach_number", flow.bulk_mach_number),
            many("profile_y_over_radius", flow.profile_y_over_radius),
            many("profile_y_plus", flow.profile_y_plus),
            many("profile_u_plus", flow.profile_u_plus),
            many("profile_u_over_bulk", flow.profile_u_over_bulk),
        ],
        invariants: vec![
            one("largest_imbalance", flow.largest_imbalance),
            one("flux_residual", flow.flux_residual),
            one("momentum_ledger_residual", flow.momentum_ledger_residual),
        ],
        field,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::solvers::describe_value;

    const SOLVER_SOURCE: &str = include_str!("../../genesis_solvers/src/pipe_flow_superpipe.rs");

    /// `(variant, doc sentence)` for every variant of `PipeReject`, read from
    /// the solver's source. An intra-doc link is written as its bare code span.
    fn documented_rejects() -> Vec<(String, String)> {
        let start = SOLVER_SOURCE
            .find("pub enum PipeReject {")
            .expect("PipeReject is in the solver");
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
                let name: String = text
                    .chars()
                    .take_while(|letter| letter.is_ascii_alphanumeric())
                    .collect();
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
        let body = describe_value(PIPE_FLOW_SUPERPIPE.name).expect("described");
        let refusals = body["refusals"].as_array().expect("refusals");
        assert_eq!(refusals.len(), documented.len());
        for (refusal, listed) in refusals.iter().zip(REFUSALS) {
            let reported = if listed.sealed { "200, outcome refused, sealed" } else { "422, no seal" };
            assert_eq!(refusal["reported_as"], reported, "{}", refusal["name"]);
        }
    }

    #[test]
    fn the_reference_is_the_one_stored_in_the_solver_test() {
        let body = describe_value(PIPE_FLOW_SUPERPIPE.name).expect("described");
        let url = body["reference"]["url"].as_str().expect("url");
        assert!(SOLVER_SOURCE.contains(&format!("<{url}>")), "{url}");
        let retrieved = body["reference"]["retrieved"].as_str().expect("retrieved");
        assert!(SOLVER_SOURCE.contains(&format!("Retrieved {retrieved}")), "{retrieved}");
        let test = body["reference"]["test"].as_str().expect("test");
        assert!(SOLVER_SOURCE.contains(&format!("fn {test}()")), "{test}");
    }

    /// The list an agent reads first carries the standing.
    #[test]
    fn the_summary_carries_the_standing() {
        let summary = PIPE_FLOW_SUPERPIPE.summary;
        assert!(summary.contains("nothing fitted"));
        assert!(summary.contains("31,577 to 35,724,000"));
        assert!(summary.contains("4000 to 1e10"));
        assert!(!summary.contains("..="));
        assert!(summary.contains("by the rule fixed beforehand the comparison failed"));
        assert!(summary.contains("the centreline line missed at one of 21 profiles"));
        assert!(summary.ends_with("2.7 percent low at 35.7 million."));
        let body = describe_value(PIPE_FLOW_SUPERPIPE.name).expect("described");
        assert_eq!(body["locked"]["fitted"].as_str().expect("fitted").split('.').next(), Some("Nothing"));
    }

    #[test]
    fn every_published_name_is_in_the_entry() {
        let flow = match solved(pipe::solve(&PipeQuestion {
            bulk_reynolds_number: 1.0e6,
            static_pressure_pa: 1.47e6,
            temperature_k: 296.0,
            wall_roughness_rms_um: 0.15,
        })) {
            Solved::Answer(answer) => answer,
            Solved::Refused(refused) => panic!("{}", refused.sentence),
        };
        assert_eq!(flow.result.len(), PIPE_FLOW_SUPERPIPE.result.len());
        for ((name, value), output) in flow.result.iter().zip(PIPE_FLOW_SUPERPIPE.result) {
            assert_eq!(name, output.name);
            assert_eq!(value.shape(), output.shape);
        }
        assert_eq!(flow.invariants.len(), PIPE_FLOW_SUPERPIPE.invariants.len());
        for ((name, _), output) in flow.invariants.iter().zip(PIPE_FLOW_SUPERPIPE.invariants) {
            assert_eq!(name, output.name);
        }
        assert_eq!(flow.field.len(), 2 * (pipe::GRID_INTERVALS + 1));
    }
}
