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
            bound: "Bulk velocity times the bore (129.36 mm) over the kinematic viscosity, answered for fully turbulent flow from 4000 to 10,000,000,000. Below 2300 the flow is laminar and is refused as LaminarFlow; from 2300 to below 4000 it is transitional and is refused as TransitionalFlow; above 1e10 the grid's first node leaves the viscous sublayer and it is refused as ReynoldsAboveGrid. The equations depend on the Reynolds number alone. Compared on the Superpipe at 31,577 to 35,724,000. On the 19 friction factors not used to choose the model, 74,345 to 35,724,000: within 1.38 percent RMS, the friction factor falling below the measured one as the Reynolds number rises, 2.7 percent low at 35.7 million. From 31,577 to 74,345 the comparison is with the 12 Zagarola-Smits profiles used to choose the outer shape: friction within 1.31 percent RMS, the Prandtl-Karman law 0.77. Below 31,577 and above 35,724,000 it computes the same equations.",
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
            bound: "Absolute static pressure of the air in the pipe, above zero (PressureNotPositive otherwise). It does not enter the solve: with the temperature it gives the density of ideal-gas air, the bulk velocity and its Mach number, and a bulk Mach number above 0.3 is refused as MachAboveIncompressible. The compared runs were at 100,090 to 18,928,000 Pa, at bulk velocities of about 3.9 to 30.8 m/s; below the 101,325 Pa of NSTAP case 1 the comparison is with the 12 profiles used to choose the outer shape. At the Superpipe's highest pressures air is about 3 percent denser than ideal and more viscous, so the Mach number given there is low by up to about a fifth.",
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
            bound: "Temperature of the air, 170 to 1900 K, where Sutherland's law gives its viscosity within 2 percent; outside it the bulk velocity cannot be known and it is refused as TemperatureOutsideViscosityLaw. It enters the Mach number and not the solve. The compared runs were at 291.79 to 300.13 K; above 298.4 K, the warmest run not used to choose the model, the comparison is with the 12 profiles used to choose the outer shape.",
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
            bound: "Root-mean-square roughness of the wall. The Superpipe's smooth wall is 0.15 um rms, and every compared run was on it; a smoother wall is answered as hydraulically smooth. A rougher wall is a rough-wall flow, refused as RoughWall; a negative roughness is RoughnessNegative.",
            only_when: None,
            asked_by_check: true,
        },
    ],
    kept_inside: "The bore, the closure and its constants (kappa 0.41, A+ 26, Nikuradse's outer shape), the grid and the tolerances stay inside the solver.",
    result: &[
        output("darcy_friction_factor", "1", Shape::Number, "Pressure drop per length times the bore over the bulk dynamic pressure, rho U_b^2 / 2. Compared with 19 measured friction factors: within 1.38 percent RMS, worst 2.69."),
        output("centreline_to_bulk_velocity_ratio", "1", Shape::Number, "Velocity on the axis over the bulk velocity. Compared with 21 measured profiles: within 1.33 percent RMS, low on 20 by 0.75 to 1.79 percent and on the NSTAP profile at 81,330 by 2.99 percent."),
        output("friction_to_bulk_velocity_ratio", "1", Shape::Number, "Friction velocity over the bulk velocity, sqrt(f/8), from the friction factor."),
        output("karman_number", "1", Shape::Number, "Radius in viscous units, u_tau R / nu, from the friction factor and the Reynolds number."),
        output("pressure_drop_per_m_over_bulk_dynamic_pressure", "m^-1", Shape::Number, "f / D: the pressure drop per metre is this times rho U_b^2 / 2, with the caller's density and bulk velocity."),
        output("bulk_mach_number", "1", Shape::Number, "Bulk velocity over the speed of sound of ideal-gas air with Sutherland's viscosity at the asked pressure and temperature: Re mu / (rho D) over sqrt(1.4 R T). At most 0.3."),
        output("profile_y_over_radius", "1", Shape::Numbers, "Wall distance over the radius of each profile station: 25 fixed stations from 0.0002 to 1 (the axis), less those nearer the wall than was compared at this Reynolds number: below 50 viscous units up to 5,975,000 (the highest NSTAP profile), and above it below 0.010591 of the radius, the nearest compared point of the Pitot profiles (y+ 1054 at 6.11 million, 5444 at 35.7 million)."),
        output("profile_y_plus", "1", Shape::Numbers, "The same distances in viscous units, each at least 50, and above Reynolds number 5,975,000 at least the y+ of 0.010591 of the radius."),
        output("profile_u_plus", "1", Shape::Numbers, "Mean velocity over the friction velocity at each station. Compared with 827 measured points of 21 profiles: within 1.59 percent pooled RMS, worst profile 2.58."),
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
            "mixing_length": "l = kappa R [0.35 - 0.20 (1 - y/R)^2 - 0.15 (1 - y/R)^4] (1 - exp(-y+/A+)): Nikuradse's pipe mixing length (0.14, 0.08, 0.06 over R) rescaled by 0.41/0.40 to wall slope kappa, with van Driest's damping. The rescaled form is this solver's own.",
            "kappa": pipe::KAPPA,
            "van_driest_a_plus": pipe::VAN_DRIEST_A_PLUS,
            "bulk_reynolds_number": "U_b D / nu, U_b the bulk (volume-averaged) velocity and D the bore.",
            "grid_intervals": pipe::GRID_INTERVALS,
            "grid_stretch": pipe::GRID_STRETCH,
            "grid": "Wall distance over the radius at node j of N: (exp(14 j/N) - 1)/(exp(14) - 1). Finite volumes about each node.",
            "fitted": "Nothing. kappa and A+ are the literature's. The outer shape is the one of five forms with no fitted constant, four published and Nikuradse's distribution rescaled to wall slope kappa, with the least friction error on the 12 Zagarola-Smits profiles below 1e6: 1.3 percent RMS against 2.2 for the form as published, which had the smaller centreline error on the same profiles, 1.2 percent against 1.4. The grid was set against itself: from 1600 to 6400 intervals the friction factor moves by 1.1e-5 and the centreline ratio by 1.3e-5, relative, at either end of the compared range; doubling the grid moves the friction factor by 2.6e-5 at 1e10, the top served, where the first node is at y+ 0.78.",
            "mach_number": "Ideal-gas air, R 287.05 J/(kg K), ratio of heat capacities 1.4, Sutherland's viscosity 1.716e-5 Pa s (T/273 K)^1.5 (384 K)/(T + 111 K): bulk velocity Re mu R T / (p D), over sqrt(1.4 R T). It holds the incompressible answer to a bulk Mach number of 0.3, where the density of the air brought to rest changes by about 4.5 percent.",
            "solver_units": "The radius and the bulk velocity as one.",
        }),
        invariants: json!([
            {
                "name": "largest_imbalance",
                "rule": "At every node the shear stress carried across its two faces balances the pressure gradient over its volume: the momentum balance. The largest imbalance over every node, over the pressure force on the whole section. It confirms the equations were solved; how closely they describe the air is in validation.",
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
            "rule": "Newton's method on every node's momentum balance and the volume flux, the pressure gradient the extra unknown; each iteration solves the tridiagonal Jacobian bordered by one row and one column directly. The van Driest damping takes the friction velocity of the previous iterate. A result is returned when the largest imbalance is within its tolerance, the friction velocity changed by no more than its tolerance in the last iteration, the momentum ledger closes and the velocity rises from wall to axis; otherwise the solve is refused by name.",
            "refusal": "NotSettled, ImbalanceGrew, SingularSystem, FluxBroken, MomentumLedgerBroken or VelocityNotMonotone",
            "run_time": "14 to 18 Newton iterations over the range, about 10 ms in a debug build.",
        }),
        reference: json!({
            "name": pipe::REFERENCE_NAME,
            "url": pipe::REFERENCE_URL,
            "retrieved": pipe::REFERENCE_RETRIEVED,
            "tolerance_fraction": 0.03,
            "test": "zagarola_smits_1998_friction_factors",
            "covers": "The Darcy friction factor at the 12 Zagarola-Smits Reynolds numbers, 31,577 to 751,820: each within 3 percent; RMS 1.31 percent, worst 2.26 percent (at 31,577).",
            "kind": "Measurements of the Superpipe itself: the 12 profiles used to choose the closure's outer shape. The comparison with the profiles not used to choose it is in validation.",
        }),
    }
}

fn validation() -> [String; 5] {
    let object = "The Princeton Superpipe: a closed-loop compressed-air pipe flow facility whose test section has a bore of 129.36 mm and a smooth wall, 0.15 um rms, run from atmospheric pressure to about 19 MPa to reach bulk Reynolds numbers from 31,577 to 35,724,000. The solver settles the fully developed mean flow at one bulk Reynolds number with a mixing length and van Driest damping. Nothing is fitted.".to_string();
    let data = "Pitot and NSTAP mean-velocity profiles of the Superpipe, tabulated first-hand by A. J. Smits' group, Princeton University. Used to choose the outer shape of the mixing length: the 12 Zagarola-Smits 1998 Pitot profiles below Reynolds number 1e6 (0.9 mm probe). Not used to choose it, and compared: the 25 McKeon 2004 Pitot profiles (0.3 mm probe, and three larger at two Reynolds numbers, with the McKeon-Smits static-pressure correction), 19 distinct Reynolds numbers 74,345 to 35,724,000, each file stating its friction factor, bulk and maximum velocities; the 9 smooth-pipe NSTAP 2012 and 2013 profiles, 81,330 to 5,975,000, whose friction factor is stated as a quarter of Darcy's; and the 14 Zagarola-Smits profiles above 1e6, given beside the targets. The McKeon and Zagarola-Smits headers state the measured pressure drop, from which their friction factors come; the NSTAP files state a friction velocity and a friction factor and do not state how the friction velocity was obtained.".to_string();
    let pass_line = "Three error measures, on the profiles not used to choose the model. Friction (P1), on the McKeon 2004 friction factors: RMS relative error at most 3 percent, worst at most 5 percent, and RMS below the Prandtl-Karman law's. Centreline (P2), on the McKeon profiles above 1e6 and every NSTAP profile: every centreline-to-bulk ratio within 2 percent, and RMS below the log law integrated over the section (1 + 3.75 sqrt(f_PK/8)). Profile (P3), on the same profiles, at every point from y+ = 50 to the axis and at least two Pitot diameters from the wall: pooled RMS of u+ at most 2 percent, worst profile at most 3 percent, and pooled RMS below the log law u+ = ln(y+)/0.40 + 5.5. Record SHA-256 704700a99d27cdd566d47a92182d363f31ad8d1ee774c1e75c84a8de75b26893.".to_string();
    let limits = "Every compared run is of air in the one 129.36 mm smooth pipe at 291.8 to 300.1 K, at 100,090 to 18,928,000 Pa and bulk velocities of about 3.9 to 30.8 m/s; the runs lie along a line from low Reynolds number at low pressure to high at high pressure. The solver computes past that envelope with the same equations wherever they hold: fully turbulent flow from Reynolds number 4000 to 1e10, any pressure above zero, 170 to 1900 K, up to a bulk Mach number of 0.3. Below 31,577 and above 35,724,000 the friction error, which grows with the Reynolds number inside the compared range, has no measurement. The model has no compressibility, no roughness and no development length: the flow is fully developed and the density and viscosity constant. Its friction factor falls below the measured one as the Reynolds number rises (-0.7 percent at 1e6, -1.4 at 1e7, -2.7 at 3.57e7), so its error is largest at the top of the compared range. The centreline ratio reads low on 20 of 21 profiles not used to choose the model, by 0.75 to 1.8 percent, and 2.99 percent low on NSTAP case 1 (Reynolds number 81,330); whether that profile's bulk velocity was measured independently or integrated from its profile, and how uncertain it is, is a question put to the data's authors. From 31,577 to 74,345, below 101,325 Pa and above 298.4 K the comparison is with the 12 profiles used to choose the outer shape. The profile is served where it was compared: from 50 viscous units out up to Reynolds number 5,975,000, and above that from 0.010591 of the radius out (y+ 1054 at 6.11 million, 5444 at 35.7 million), the nearest point of the Pitot profiles. The ledgers confirm the equations were solved and a proof confirms an answer is unchanged; how closely the model matches the air is the comparison above.".to_string();
    [object, data, pass_line, RESULT.to_string(), limits]
}

/// The comparison with the measurements not used to choose the model.
const RESULT: &str = "On the profiles not used to choose the model. Friction (P1), on 19 McKeon friction factors: RMS 1.38 percent (target 3), worst 2.69 percent at 35.7 million (target 5), against the Prandtl-Karman law's RMS 2.17 (worst 3.64); Colebrook's smooth law RMS 2.18, Blasius 23.4. Profile (P3), on 827 points of 21 profiles: pooled RMS of u+ 1.59 percent (target 2), worst profile 2.58 percent, NSTAP at 3.95 million (target 3), against the log law's 3.59. Centreline (P2), on the same 21 profiles: RMS 1.33 percent, against the baseline's 2.10; 20 profiles are 0.75 to 1.79 percent low (target 2), and the NSTAP profile at 81,330 is 2.99 percent low (target 2). That profile's measured ratio, 11.69/9.48 = 1.233, is above the McKeon profile at 74,345 (1.218) and the Zagarola-Smits profile at 74,293 (1.220). Beside the targets: the 14 Zagarola-Smits profiles above 1e6, friction RMS 2.75 percent, worst 4.52 (Prandtl-Karman 3.62, 5.45), centreline worst 1.21 percent, profile pooled 1.48 percent; the McKeon profiles at or below 1e6, centreline worst 2.02 percent, profile pooled 1.28 percent; the nine NSTAP friction factors, RMS 1.13 percent. On the 12 profiles used to choose the outer shape: friction RMS 1.31 percent, the Prandtl-Karman law 0.77.";

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
        let summary = crate::spec_sheet::summary_of(PIPE_FLOW_SUPERPIPE.name, PIPE_FLOW_SUPERPIPE.summary);
        assert!(summary.contains("Nothing fitted"));
        assert!(summary.contains("31,577 to 35,724,000"));
        assert!(summary.contains("4000 to 1e10"));
        assert!(!summary.contains("..="));
        assert!(summary.contains("friction factor within 1.38 percent RMS, worst 2.69"));
        assert!(summary.contains("2.99 percent low on the NSTAP profile at 81,330"));
        assert!(summary.contains("2.7 percent at 35.7 million."));
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
