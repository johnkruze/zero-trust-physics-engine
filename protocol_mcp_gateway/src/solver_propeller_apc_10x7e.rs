//! The entry for `propeller_apc_10x7e`: thrust and shaft power of the APC
//! Thin Electric 10x7 propeller by blade-element momentum theory. The bounds
//! and the constants come from `genesis_solvers`, the sentence of each
//! refusal from the solver's source, and the validation record from the
//! research record of the propeller.
//!
//! The solver is the judge of a request: every limit a request itself shows
//! is asked of the solver's own `check_question` before a physics permit is
//! taken, so the name and the sentence are the solver's.

use genesis_solvers::propeller_apc_10x7e::{
    self as propeller, PropellerQuestion, PropellerReject, PropellerSolution,
};
use serde_json::{Value, json};

use crate::solvers::{
    Answer, FurtherExample, Given, Input, Kind, Output, Published, Refusal, Refused, Sections,
    Shape, Solved, Solver,
};

/// What the record says of the comparison with the held-back measurements.
pub const HELD_BACK_RESULT: &str = "Line A, 64 points of the sweeps at 6015, 6020, 6519 and 6531 rev/min, not in the fit: mean error 4.85 percent of full scale (target 4.0), worst 10.16 (target 10.0); the solver is low at every point, by 5.0 on average in thrust coefficient and 4.7 in power coefficient. Line B, 9 rows at zero airspeed between 4144 and 6542 rev/min: mean 6.02 (target 4.0), worst 11.76 (target 10.0); the thrust coefficient is within 3.0 on average and the power coefficient is 9.0 low, low in every row. With the two constants set by hand instead of fitted (−4.0 degrees and 0.020) the error on line A is mean 3.77 and worst 10.03, and on line B 6.06 and 11.85: the unfitted model errs high where the fitted one errs low. It is above the measurement at every value of the fitted sweeps, by 7.6 in thrust coefficient and 9.2 in power coefficient on average, and above it at 119 of the 128 values of line A, by 2.8 and 4.5 on average, while the fitted model sits on the fitted sweeps and below every value of line A: the measurement rose with speed between the two, and the fit at 4007 to 5018 rev/min does not carry to the higher speeds. Thirteen points outside advance ratio 0.10 to 0.75 and seven static rows below 4000 rev/min are in no line. A recomputation in 50-digit arithmetic agrees with the solver to 2.2e-14: the solver computes the model it states. A second program, with its own code for the statistics, the proofs and the model, recomputes every statistic to 1e-12 and all 93 rows from the same saved answers.";

const fn number(
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
        asked_by_check: true,
    }
}

const fn one(name: &'static str, unit: &'static str, note: &'static str) -> Output {
    Output {
        name,
        unit,
        shape: Shape::Number,
        each: &[],
        note,
        asked_by: None,
    }
}

const fn rings(name: &'static str, unit: &'static str, note: &'static str) -> Output {
    Output {
        name,
        unit,
        shape: Shape::Numbers,
        each: &[],
        note,
        asked_by: None,
    }
}

pub const PROPELLER_APC_10X7E: Solver = Solver {
    name: "propeller_apc_10x7e",
    summary: "Thrust and shaft power of the APC Thin Electric 10x7 propeller at one rotational speed, airspeed and air density, by blade-element momentum theory on its measured blade geometry, with two blade-section constants fitted to this propeller's own measured thrust and power; on the measurements held back from the fit it missed both of its lines, by up to 10.2 percent of full scale at speeds never fitted and 11.8 at rest.",
    inputs: &[
        Input {
            name: "rotational_speed_rev_per_min",
            kind: Kind::NumberAbove {
                above: propeller::ROTATIONAL_SPEED_ABOVE_REV_PER_MIN,
                maximum: propeller::ROTATIONAL_SPEED_MAX_REV_PER_MIN,
                unit: "rev/min",
            },
            refusal: Some("RotationalSpeedNotPositive"),
            bound: "How fast the propeller turns, above zero (RotationalSpeedNotPositive otherwise: the coefficients are per turn of the blades). The section's coefficients depend on the advance ratio alone, so a rotational speed acts on it through two limits and nothing else: the chord Reynolds number, held inside 44399 to 75516, the band the section's two constants are taken over (ReynoldsNumberOutOfRange), and the tip Mach number, held at or below 0.3 because the section law is incompressible (TipMachAboveIncompressible). The maximum here, about 7676 rev/min, is that Mach number with the air still; at 1.225 kg/m³ and zero airspeed the band holds about 4014 to 6828 rev/min, and thinner or denser air, or moving air, moves it. The compared measurements span 4007 to 6531 rev/min with the air moving (the slowest and fastest measured sweep) and 4144 to 6542 rev/min at zero airspeed (the slowest and fastest static row); outside them it computes the same equations. At the speeds not in the fit, 6015 to 6531 rev/min, the answer is low at every point, mean 4.85 percent of full scale and worst 10.16 (targets 4.0 and 10.0), and at rest mean 6.02 and worst 11.76, the static shaft power about 9 percent of full scale low; the error grows with speed above the fitted 4007 to 5018 rev/min.",
            only_when: None,
            asked_by_check: true,
        },
        number(
            "airspeed_m_s",
            0.0,
            propeller::AIRSPEED_MAX_M_S,
            "m/s",
            "TipMachAboveIncompressible",
            "How fast the air meets the propeller along its axis. Exactly 0 is the propeller at rest in still air. Any airspeed is answered, from zero up, until the relative speed of the air at the tip exceeds Mach 0.3 at 340.29 m/s (dry air at 15 degC), where the incompressible blade section no longer holds: that is refused as TipMachAboveIncompressible, and the maximum here, 102.087 m/s, is that Mach number with the blades still, so every turning propeller meets it lower (at 6531 rev/min, near 54 m/s). The advance ratio, airspeed / (revolutions per second × 0.254 m), is computed and used as computed. The compared measurements span advance ratio 0.111 to 0.748 with the air moving (at 6015 to 6531 rev/min, low at every point, mean 4.85 percent of full scale); below 0.111 and above 0.748 it computes the same equations. Past zero thrust, near advance ratio 0.83, the air drives the blades: thrust and then shaft power turn negative, and the efficiency returned is not an efficiency there. A ring whose equations have no root is refused as NoBalance. A negative airspeed is refused as AirspeedNegative.",
        ),
        Input {
            name: "air_density_kg_m3",
            kind: Kind::NumberAbove {
                above: propeller::AIR_DENSITY_ABOVE_KG_M3,
                maximum: f64::MAX,
                unit: "kg/m³",
            },
            refusal: Some("AirDensityNotPositive"),
            bound: "Density of the air, above zero (AirDensityNotPositive otherwise). Thrust and power are proportional to it; the model's coefficients do not depend on it. No density was compared: the measurements give thrust and power coefficients, with no record of the tunnel air's density, and every comparison was sent at 1.225 kg/m³. Density moves the chord Reynolds number at three quarters of the radius (density × geometric speed there × 16.383 mm chord / 1.81e-5 Pa·s), on which a blade section's coefficients depend, and the section has one zero-lift angle and one minimum drag with no law for how they move with it, so that Reynolds number is held inside 44399 to 75516, the band the two constants are taken over (the band the 116 compared points span at 1.225 kg/m³), and outside it a question is refused as ReynoldsNumberOutOfRange: at 5000 rev/min and advance ratio 0.4 that is 0.97 to 1.65 kg/m³. The band assumes the tunnel air was 1.225 kg/m³.",
            only_when: None,
            asked_by_check: true,
        },
    ],
    kept_inside: "The blade geometry, the blade section, the number of rings and the tolerances stay inside the solver.",
    result: &[
        one("advance_ratio", "", "J = airspeed / (revolutions per second × diameter), as computed from the question. Compared with measurement from 0.111 to 0.748 with the air moving."),
        one("thrust_coefficient", "", "CT = thrust / (density × (rev/s)² × diameter⁴)."),
        one("power_coefficient", "", "CP = shaft power / (density × (rev/s)³ × diameter⁵)."),
        one("thrust_n", "N", ""),
        one("torque_n_m", "N·m", ""),
        one("shaft_power_w", "W", "Torque times angular speed."),
        one("efficiency", "", "Thrust times airspeed over shaft power. 0 at zero airspeed. An efficiency while thrust and shaft power are both positive; past zero thrust, near advance ratio 0.83, the air drives the blades."),
        rings("ring_inflow_angle_rad", "rad", "The settled inflow angle of each of the 34 rings, from the hub outward."),
        rings("ring_thrust_n", "N", "Thrust of each ring. They add up to `thrust_n`."),
        rings("ring_torque_n_m", "N·m", "Torque of each ring. They add up to `torque_n_m`."),
    ],
    invariants: &[
        one(
            "thrust_balance_gap",
            "",
            "Blade thrust less the axial momentum given to the air, whole disc, as a fraction of the sum over the rings of each ring's larger side.",
        ),
        one(
            "worst_ring_balance_gap",
            "",
            "The largest gap of any one ring between its axial momentum and its blade force, as a fraction of the larger.",
        ),
    ],
    iterations: "Halvings of the inflow angle's bracket, all 34 rings together. On a refusal it is the count for `NotSettled` and null otherwise.",
    field: field_words,
    answer: "a settled propeller",
    refusals: REFUSALS,
    door_refusals: &[],
    sections,
    validation: Some(validation),
    example,
    further_examples: &[FurtherExample {
        key: "example_solve_at_rest",
        parameters: at_rest_example,
    }],
    parts: &[],
    check,
    solve,
};

/// Each `PropellerReject` variant with the sentence from its doc comment.
/// The request itself shows the first six, so they are not sealed.
const REFUSALS: &[Refusal] = &[
    Refusal {
        name: "NonFiniteInput",
        meaning: "A number of the question is NaN or infinite.",
        sealed: false,
    },
    Refusal {
        name: "RotationalSpeedNotPositive",
        meaning: "The rotational speed is not above `ROTATIONAL_SPEED_ABOVE_REV_PER_MIN`: the coefficients are per turn of the blades, and the rings' angular momentum is that of blades turning forward.",
        sealed: false,
    },
    Refusal {
        name: "AirDensityNotPositive",
        meaning: "The air density is not above `AIR_DENSITY_ABOVE_KG_M3`: no air.",
        sealed: false,
    },
    Refusal {
        name: "AirspeedNegative",
        meaning: "The airspeed is negative.",
        sealed: false,
    },
    Refusal {
        name: "TipMachAboveIncompressible",
        meaning: "The relative speed of the air at the tip over `SPEED_OF_SOUND_M_S` is above `TIP_MACH_MAX`: the flow over the blade is compressible, and the section is not.",
        sealed: false,
    },
    Refusal {
        name: "ReynoldsNumberOutOfRange",
        meaning: "The chord Reynolds number at three quarters of the radius, at the question's density, is outside `REYNOLDS_NUMBER_MIN` to `REYNOLDS_NUMBER_MAX`, the band the blade section's two constants are taken over: the section has one zero-lift angle and one minimum drag and no law for how they move with the Reynolds number, and across this band alone the zero-lift angle that fits each sweep alone drifts from about −2.6 to −3.8 degrees.",
        sealed: false,
    },
    Refusal {
        name: "NonFiniteValue",
        meaning: "An intermediate value in a ring was NaN or infinite.",
        sealed: true,
    },
    Refusal {
        name: "NoBalance",
        meaning: "No inflow angle balances a ring's momentum and blade force.",
        sealed: true,
    },
    Refusal {
        name: "NotSettled",
        meaning: "A ring's inflow angle was still not bracketed within `ANGLE_TOLERANCE_RAD` after the halvings allowed.",
        sealed: true,
    },
    Refusal {
        name: "MomentumBalanceBroken",
        meaning: "At the settled angle a ring's axial momentum and blade force differ by more than `BALANCE_TOLERANCE`.",
        sealed: true,
    },
    Refusal {
        name: "DiscBalanceBroken",
        meaning: "At acceptance the blade thrust and the axial momentum given to the air over the whole disc differ by more than `BALANCE_TOLERANCE` of the sum of the rings' larger sides.",
        sealed: true,
    },
];

/// The `PropellerReject` variant name, without its fields.
pub fn reject_name(reject: &PropellerReject) -> &'static str {
    match reject {
        PropellerReject::NonFiniteInput => "NonFiniteInput",
        PropellerReject::RotationalSpeedNotPositive { .. } => "RotationalSpeedNotPositive",
        PropellerReject::AirDensityNotPositive { .. } => "AirDensityNotPositive",
        PropellerReject::AirspeedNegative { .. } => "AirspeedNegative",
        PropellerReject::TipMachAboveIncompressible { .. } => "TipMachAboveIncompressible",
        PropellerReject::ReynoldsNumberOutOfRange { .. } => "ReynoldsNumberOutOfRange",
        PropellerReject::NonFiniteValue { .. } => "NonFiniteValue",
        PropellerReject::NoBalance { .. } => "NoBalance",
        PropellerReject::NotSettled { .. } => "NotSettled",
        PropellerReject::MomentumBalanceBroken { .. } => "MomentumBalanceBroken",
        PropellerReject::DiscBalanceBroken { .. } => "DiscBalanceBroken",
    }
}

fn field_words() -> String {
    format!(
        "SHA-384 of the count of values ({}) as 8 little-endian bytes, then, as little-endian f64 bytes, the inflow angle of each of the {} rings in radians, the thrust of each ring in newtons and the torque of each ring in newton metres. These are the values of `ring_inflow_angle_rad`, `ring_thrust_n` and `ring_torque_n_m`, in that order.",
        3 * propeller::ANNULI,
        propeller::ANNULI
    )
}

fn sections() -> Sections {
    let mut locked = json!({
        "blades": propeller::BLADES,
        "diameter_m": propeller::DIAMETER_M,
        "rings": propeller::ANNULI,
        "hub_station_r_over_R": propeller::HUB_STATION,
        "geometry_r_over_R_chord_over_R_blade_angle_deg": propeller::GEOMETRY
            .iter()
            .map(|(x, chord, beta)| json!([x, chord, beta]))
            .collect::<Vec<_>>(),
        "geometry_source": "UIUC Propeller Data Site, volume 1, apce_10x7_geom.txt; linear between the rows",
        "lift_slope_per_rad": propeller::LIFT_SLOPE_PER_RAD,
        "alpha_zero_lift_deg": propeller::ALPHA_ZERO_LIFT_DEG,
        "cd0": propeller::CD0,
        "fitted": "alpha_zero_lift_deg and cd0, to this propeller's measured thrust and power coefficients at 4007, 5001 and 5018 rev/min",
        "cl_max": propeller::CL_MAX,
        "drag_due_to_lift": propeller::DRAG_DUE_TO_LIFT,
        "cd_flat_plate": propeller::CD_FLAT_PLATE,
        "scan_steps": propeller::SCAN_STEPS,
        "angle_low_rad": propeller::ANGLE_LOW_RAD,
        "max_bisections": propeller::MAX_BISECTIONS,
        "compared_advance_ratio_min": propeller::ADVANCE_RATIO_MIN,
        "compared_advance_ratio_max": propeller::ADVANCE_RATIO_MAX,
        "tip_mach_number": "The relative speed of the air at the tip, sqrt((2π × rev/s × 0.127 m)² + airspeed²), over 340.29 m/s, dry air at 15 degC: the question holds no temperature. At 6531 rev/min and zero airspeed it is 0.255.",
        "constants_named_in_refusals": {
            "ROTATIONAL_SPEED_ABOVE_REV_PER_MIN": propeller::ROTATIONAL_SPEED_ABOVE_REV_PER_MIN,
            "AIR_DENSITY_ABOVE_KG_M3": propeller::AIR_DENSITY_ABOVE_KG_M3,
            "SPEED_OF_SOUND_M_S": propeller::SPEED_OF_SOUND_M_S,
            "TIP_MACH_MAX": propeller::TIP_MACH_MAX,
            "REYNOLDS_NUMBER_MIN": propeller::REYNOLDS_NUMBER_MIN,
            "REYNOLDS_NUMBER_MAX": propeller::REYNOLDS_NUMBER_MAX,
            "ANGLE_TOLERANCE_RAD": propeller::ANGLE_TOLERANCE_RAD,
            "BALANCE_TOLERANCE": propeller::BALANCE_TOLERANCE,
        },
    });
    let envelope = json!({
        "compared_rotational_speed_min_rev_per_min": propeller::COMPARED_ROTATIONAL_SPEED_MIN_REV_PER_MIN,
        "compared_rotational_speed_max_rev_per_min": propeller::COMPARED_ROTATIONAL_SPEED_MAX_REV_PER_MIN,
        "static_rotational_speed_min_rev_per_min": propeller::COMPARED_STATIC_ROTATIONAL_SPEED_MIN_REV_PER_MIN,
        "static_rotational_speed_max_rev_per_min": propeller::COMPARED_STATIC_ROTATIONAL_SPEED_MAX_REV_PER_MIN,
        "rotational_speed_max_rev_per_min": propeller::ROTATIONAL_SPEED_MAX_REV_PER_MIN,
        "airspeed_max_m_s": propeller::AIRSPEED_MAX_M_S,
        "reynolds_number": "Chord Reynolds number at three quarters of the radius: density × sqrt((2π × rev/s × 0.75 × 0.127 m)² + airspeed²) × chord / viscosity. The band is the lowest and highest over the 116 compared points (43 in the fit, 64 outside it, 9 static) at 1.225 kg/m³, each rounded outward to a whole number: 44399.03 at 4007 rev/min and advance ratio 0.144, 75515.76 at 6519 rev/min and 0.734.",
        "chord_at_three_quarters_m": propeller::CHORD_AT_THREE_QUARTERS_M,
        "air_viscosity_pa_s": propeller::AIR_VISCOSITY_PA_S,
        "compared_air_density_kg_m3": propeller::COMPARED_AIR_DENSITY_KG_M3,
    });
    if let (Some(locked), Value::Object(envelope)) = (locked.as_object_mut(), envelope) {
        locked.extend(envelope);
    }
    Sections {
        locked,
        invariants: json!([
            {
                "name": "worst_ring_balance_gap",
                "rule": "In every ring the axial momentum given to the air equals the axial force of the blade elements.",
                "tolerance": propeller::BALANCE_TOLERANCE,
                "unit": "fraction of the larger side",
                "compared": "every ring, at its settled angle; every trial angle is held to finite values, a tip loss factor in (0, 1] and a forward blade speed",
                "refusal": "MomentumBalanceBroken",
            },
            {
                "name": "thrust_balance_gap",
                "rule": "The thrust of the blades equals the axial momentum given to the air over the whole disc.",
                "tolerance": propeller::BALANCE_TOLERANCE,
                "unit": "fraction of the sum of the rings' larger sides",
                "compared": "at acceptance, against the same tolerance, after every ring has balanced",
                "refusal": "DiscBalanceBroken",
            },
        ]),
        convergence: json!({
            "angle_tolerance_rad": propeller::ANGLE_TOLERANCE_RAD,
            "rule": "Each ring's inflow angle is bracketed at the first change of sign of its imbalance and halved until the bracket is within the tolerance. A result is returned when all 34 rings have settled and balance; otherwise the solve is refused by name.",
            "refusal": "NotSettled",
        }),
        reference: json!({
            "name": propeller::REFERENCE_NAME,
            "url": propeller::REFERENCE_URL,
            "retrieved": propeller::REFERENCE_RETRIEVED,
            "tolerance_of_full_scale": propeller::REFERENCE_TOLERANCE_OF_FULL_SCALE,
            "full_scale": {
                "thrust_coefficient": propeller::THRUST_COEFFICIENT_FULL_SCALE,
                "power_coefficient": propeller::POWER_COEFFICIENT_FULL_SCALE,
            },
            "test": "uiuc_measured_sweep_at_5018_rev_per_min",
            "covers": "Thrust coefficient and power coefficient at the 17 measured advance ratios of that sweep, 0.112 to 0.575.",
            "kind": "A measurement, and one of the three sweeps the two fitted constants were fitted to: the test shows the solver is the fitted model. The comparison with measurements it never saw is in `validation`.",
        }),
    }
}

fn validation() -> [String; 5] {
    let object = format!(
        "The APC Thin Electric 10x7 propeller: two blades, {diameter} m (10 inch) diameter, 7 inch nominal pitch, a moulded model-aircraft propeller. The model is blade-element momentum theory on the chord and blade angle the University of Illinois measured at 18 stations of this propeller, with one blade section for the whole blade and Prandtl's tip loss. Two constants of that section are fitted to this propeller's own measured thrust and power: its zero-lift angle ({alpha} degrees) and its minimum drag coefficient ({cd0}). The lift slope (2π per radian), the stall lift coefficient ({cl_max}), the drag due to lift ({kd}) and the broadside drag coefficient ({flat}) are set, not fitted.",
        diameter = propeller::DIAMETER_M,
        alpha = propeller::ALPHA_ZERO_LIFT_DEG,
        cd0 = propeller::CD0,
        cl_max = propeller::CL_MAX,
        kd = propeller::DRAG_DUE_TO_LIFT,
        flat = propeller::CD_FLAT_PLATE,
    );
    let data = "Wind-tunnel measurements of thrust and power coefficient from the UIUC Propeller Data Site, volume 1 (University of Illinois at Urbana-Champaign): seven sweeps of advance ratio at fixed rotational speed and one run at zero airspeed. In the fit: the sweeps at 4007, 5001 and 5018 rev/min, 43 points with advance ratio 0.112 to 0.726. Outside the fit, compared: the sweeps at 6015, 6020, 6519 and 6531 rev/min, 64 points with advance ratio 0.111 to 0.748, and the run at zero airspeed, 9 rows at 4144 to 6542 rev/min. The measurements give coefficients; the density of the tunnel air is not in them, and every comparison was sent at 1.225 kg/m³.".to_string();
    let pass_line = format!(
        "An error is the solver less the measurement as a percentage of full scale, full scale being the largest measured value in the fitted sweeps (thrust coefficient {ct}, power coefficient {cp}); a percentage of the reading is not used because the thrust crosses zero. Line A, the four sweeps outside the fit: mean absolute error over thrust and power coefficients together at most 4.0 percent of full scale and no value beyond 10.0. Line B, the run at zero airspeed between 4000 and 6600 rev/min: the same two targets.",
        ct = propeller::THRUST_COEFFICIENT_FULL_SCALE,
        cp = propeller::POWER_COEFFICIENT_FULL_SCALE,
    );
    let result = HELD_BACK_RESULT.to_string();
    let limits = "1. Two constants are fitted to this propeller's own measurements. On the fitted sweeps the error is 2.1 percent of full scale on average and 5.3 at worst; with the two constants set by hand instead (−4.0 degrees and 0.020) it is 8.4 and 16.3. 2. The thrust and power coefficients depend on the advance ratio alone: there is no Reynolds-number or Mach-number dependence. The chord Reynolds number at 75 percent of the radius is about 44,000 to 76,000 over the measurements (air of 1.225 kg/m³ and 1.81e-5 Pa·s, chord 16.4 mm, geometric speed): 44,000 to 58,000 in the fitted sweeps and 67,000 to 76,000 in the sweeps outside the fit. The measured thrust and power coefficients rise with rotational speed at the same advance ratio, and the model cannot follow that. It is what line A shows, and it is why the answer is low above about 5000 rev/min, by 5 percent of full scale on average at 6000 to 6500 rev/min and by up to 10. The fitted measurements themselves, interpolated in advance ratio with no model at all, give 4.45 on average and 10.24 at worst on line A (63 of its 64 points lie between their advance ratios): no model of advance ratio alone that is faithful to the fitted data reaches the targets of line A. The shape of the model holds at the higher speeds when refitted there: with the two constants fitted to line A itself, which puts line A in the fit, the error on line A is 2.14 and 5.31 (−3.51 degrees and 0.0206). What does not carry is the constants: fitted sweep by sweep, the zero-lift angle drifts from −2.6 degrees at 4007 rev/min to about −3.5 to −3.8 at 6519 and 6531. 3. Line B has a different cause. At zero airspeed the measured thrust coefficient rises with speed, from 0.1051 at 4144 rev/min to 0.1139 at 6542, but the measured power coefficient falls, from 0.0536 to 0.0512, while the solver's static power coefficient is 0.0474 at every speed: 11.8 percent of full scale low at 4144 rev/min, the speed nearest the fitted ones, 7.2 low at 6542, and about 9 low throughout. No choice of the two constants moves it: line B is 6.0 to 6.2 on average with the fitted pair, the pair set by hand and the pair fitted to line A. The model's shaft power at rest reads low, an effect apart from the speed dependence of limit 2. 4. Line A tests extrapolation in rotational speed (fitted to 5018, compared from 6015) and interpolation in advance ratio. Line B tests extrapolation in advance ratio to zero. The compared points span 4007 to 6531 rev/min and advance ratio 0.111 to 0.748 with the air moving, and 4144 to 6542 rev/min at rest. The rotational speed, the advance ratio and the density are answered past that span with the same equations, to the Reynolds band, a tip Mach number of 0.3 and the roots of the ring equations. Inside the span, the sweeps cover the plane of speed and advance ratio unevenly: the 4007 sweep runs from 0.144 to 0.721, and the 6531 sweep from 0.111 to 0.440, so a question between sweeps is interpolated. No density was compared: the chord Reynolds number is held inside the band the compared points span at 1.225 kg/m³, and that band assumes the tunnel air was 1.225 kg/m³. The record holds one propeller, with the air on its axis. 5. One blade section stands for the whole blade; the hub inside 15 percent of the radius carries no load. 6. The measurement floor is far below the targets, so the errors are not measurement noise. The repeated sweeps outside the fit (6015 against 6020, 6519 against 6531) agree with each other to 0.15 to 0.18 percent of full scale on average and 0.29 to 0.35 at worst over the narrow band of advance ratio both cover, 0.38 to 0.48. Brandt's thesis, Table 3.1, gives the uncertainties of the rig: thrust 0.504 percent, power 0.240, torque 0.218, propeller speed 0.100, freestream velocity 0.207. 7. The five files outside the fit are now read: a further comparison needs other measurements.".to_string();
    [object, data, pass_line, result, limits]
}

/// 5000 rev/min at 8 m/s in sea-level air: advance ratio 0.378.
fn example() -> Value {
    json!({
        "rotational_speed_rev_per_min": 5000.0,
        "airspeed_m_s": 8.0,
        "air_density_kg_m3": 1.225,
    })
}

/// The propeller at rest in still air, 6000 rev/min.
fn at_rest_example() -> Value {
    json!({
        "rotational_speed_rev_per_min": 6000.0,
        "airspeed_m_s": 0.0,
        "air_density_kg_m3": 1.225,
    })
}

fn question(given: &Given) -> Result<PropellerQuestion, String> {
    Ok(PropellerQuestion {
        rotational_speed_rev_per_min: given.number("rotational_speed_rev_per_min")?,
        airspeed_m_s: given.number("airspeed_m_s")?,
        air_density_kg_m3: given.number("air_density_kg_m3")?,
    })
}

/// Every limit the request itself shows, asked of the solver's own check of
/// a question before a physics permit is taken.
fn check(given: &Given) -> Result<(), String> {
    propeller::check_question(&question(given)?)
        .map_err(|reject| format!("Physical Violation: {reject} ({}).", reject_name(&reject)))
}

fn solve(given: &Given) -> Result<Solved, String> {
    Ok(solved(propeller::solve(&question(given)?)))
}

/// What the solver returned, in the words every solver shares. A refusal
/// the request itself shows carries the solver's name for it in its sentence.
pub fn solved(returned: Result<PropellerSolution, PropellerReject>) -> Solved {
    let solution = match returned {
        Ok(solution) => solution,
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
                    PropellerReject::NotSettled { iterations, .. } => Some(u64::from(iterations)),
                    _ => None,
                },
            });
        }
    };
    let number = |name: &str, value: f64| (name.to_string(), Published::Number(value));
    let mut field = solution.inflow_angle_rad.clone();
    field.extend_from_slice(&solution.ring_thrust_n);
    field.extend_from_slice(&solution.ring_torque_n_m);
    Solved::Answer(Answer {
        iterations: u64::from(solution.iterations),
        result: vec![
            number("advance_ratio", solution.advance_ratio),
            number("thrust_coefficient", solution.thrust_coefficient),
            number("power_coefficient", solution.power_coefficient),
            number("thrust_n", solution.thrust_n),
            number("torque_n_m", solution.torque_n_m),
            number("shaft_power_w", solution.shaft_power_w),
            number("efficiency", solution.efficiency),
            (
                "ring_inflow_angle_rad".to_string(),
                Published::Numbers(solution.inflow_angle_rad),
            ),
            ("ring_thrust_n".to_string(), Published::Numbers(solution.ring_thrust_n)),
            ("ring_torque_n_m".to_string(), Published::Numbers(solution.ring_torque_n_m)),
        ],
        invariants: vec![
            number("thrust_balance_gap", solution.thrust_balance_gap),
            number("worst_ring_balance_gap", solution.worst_ring_balance_gap),
        ],
        field,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::solution;
    use crate::solvers::{self, describe_value};

    const SOLVER_SOURCE: &str = include_str!("../../genesis_solvers/src/propeller_apc_10x7e.rs");

    /// `(variant, doc sentence)` for every variant of `PropellerReject`, read
    /// from the solver's source. An intra-doc link is written as its bare
    /// code span.
    fn documented_rejects() -> Vec<(String, String)> {
        let start = SOLVER_SOURCE
            .find("pub enum PropellerReject {")
            .expect("PropellerReject is in the solver");
        let mut found = Vec::new();
        let mut doc = String::new();
        for line in SOLVER_SOURCE[start..].lines().skip(1) {
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

    fn request(rev_per_min: f64, airspeed: f64, density: f64) -> Value {
        json!({ "solver": "propeller_apc_10x7e", "parameters": {
            "rotational_speed_rev_per_min": rev_per_min,
            "airspeed_m_s": airspeed,
            "air_density_kg_m3": density,
        }})
    }

    fn the_gateway_says(request: &Value) -> Option<String> {
        solution::prepare(request)
            .err()
            .map(|error| error.message().to_string())
    }

    #[test]
    fn all_eleven_refusals_carry_the_sentence_from_the_solvers_doc_comment() {
        let documented = documented_rejects();
        assert_eq!(documented.len(), 11);
        assert_eq!(documented.len(), REFUSALS.len());
        for ((variant, sentence), refusal) in documented.iter().zip(REFUSALS) {
            assert_eq!(variant, refusal.name);
            assert_eq!(sentence, refusal.meaning, "{}", refusal.name);
        }
        let body = describe_value(PROPELLER_APC_10X7E.name).expect("described");
        let refusals = body["refusals"].as_array().expect("refusals");
        assert_eq!(refusals.len(), 11);
        // Every constant a meaning names is published with its value.
        let named = &body["locked"]["constants_named_in_refusals"];
        for refusal in REFUSALS {
            for word in refusal.meaning.split('`').skip(1).step_by(2) {
                assert!(named[word].is_number(), "{word}");
            }
        }
    }

    /// One of each refusal, as the solver would return it.
    fn one_of_each() -> Vec<PropellerReject> {
        vec![
            PropellerReject::NonFiniteInput,
            PropellerReject::RotationalSpeedNotPositive { rotational_speed_rev_per_min: 0.0 },
            PropellerReject::AirDensityNotPositive { air_density_kg_m3: 0.0 },
            PropellerReject::AirspeedNegative { airspeed_m_s: -1.0 },
            PropellerReject::TipMachAboveIncompressible { tip_mach_number: 0.4 },
            PropellerReject::ReynoldsNumberOutOfRange { reynolds_number: 30000.0 },
            PropellerReject::NonFiniteValue { ring: 3 },
            PropellerReject::NoBalance { ring: 4 },
            PropellerReject::NotSettled { ring: 5, iterations: 64 },
            PropellerReject::MomentumBalanceBroken { ring: 6, gap: 0.5 },
            PropellerReject::DiscBalanceBroken { gap: 0.5 },
        ]
    }

    #[test]
    fn a_refusal_is_sealed_only_when_the_request_does_not_show_it() {
        let rejects = one_of_each();
        assert_eq!(rejects.len(), REFUSALS.len());
        let asked = request(5000.0, 8.0, 1.225);
        for (reject, refusal) in rejects.iter().zip(REFUSALS) {
            assert_eq!(reject_name(reject), refusal.name);
            let Solved::Refused(refused) = solved(Err(*reject)) else {
                panic!("a refusal");
            };
            assert_eq!(refused.name, refusal.name);
            assert_eq!(refused.iterations.is_some(), refusal.name == "NotSettled");
            assert_eq!(refused.sentence.ends_with(&format!("({})", refusal.name)), !refusal.sealed);
            let sealed = solution::seal(&PROPELLER_APC_10X7E, asked.clone(), solved(Err(*reject)));
            if refusal.sealed {
                let response = sealed.expect("sealed");
                assert_eq!(response.document.outcome, "refused");
                assert_eq!(response.document.reject.as_deref(), Some(refusal.name));
                assert!(response.document.result.is_none() && response.document.field_sha384.is_none());
                assert_eq!(
                    response.document.reject_detail.as_deref(),
                    Some(format!(
                        "Solver Refusal: propeller_apc_10x7e did not return a settled propeller ({}). {reject}.",
                        refusal.name
                    ).as_str())
                );
            }
        }
    }

    /// The gateway refuses exactly what the solver's `check_question`
    /// refuses, with its sentence and its name, over a grid that crosses
    /// every limit and every pair of limits.
    #[test]
    fn the_gateway_refuses_exactly_what_check_question_refuses() {
        let speeds = [0.0, 1.0e-300, 1000.0, 3000.0, 4006.999, 4007.0, 4100.0, 4144.0, 5000.0, 6531.0, 6531.001, 6542.0, 6542.001, 6800.0, 7600.0, 7676.0, 7677.0, 1.0e9, -5000.0];
        let airspeeds = [-1.0, -1.0e-300, 0.0, 1.0e-300, 1.0, 1.8, 1.9, 8.0, 12.68, 12.7, 20.6, 20.7, 1.0e9];
        let densities = [0.0, 0.999, 1.0, 1.225, 1.3, 1.301, -1.0];
        let (mut refused, mut taken) = (std::collections::BTreeMap::new(), 0);
        for speed in speeds {
            for airspeed in airspeeds {
                for density in densities {
                    let question = PropellerQuestion {
                        rotational_speed_rev_per_min: speed,
                        airspeed_m_s: airspeed,
                        air_density_kg_m3: density,
                    };
                    let said = the_gateway_says(&request(speed, airspeed, density));
                    match propeller::check_question(&question) {
                        Ok(()) => {
                            assert_eq!(said, None, "{question:?}");
                            taken += 1;
                        }
                        Err(reject) => {
                            let name = reject_name(&reject);
                            assert_eq!(
                                said,
                                Some(format!("Physical Violation: {reject} ({name}).")),
                                "{question:?}"
                            );
                            *refused.entry(name).or_insert(0) += 1;
                        }
                    }
                }
            }
        }
        assert!(taken >= 20, "{taken}");
        let names: Vec<&str> = refused.keys().copied().collect();
        assert_eq!(
            names,
            ["AirDensityNotPositive", "AirspeedNegative", "ReynoldsNumberOutOfRange", "RotationalSpeedNotPositive", "TipMachAboveIncompressible"]
        );
        // An unknown key and a missing key are named and nothing is filled in.
        let mut extra = request(5000.0, 8.0, 1.225);
        extra["parameters"]["blade_pitch_deg"] = json!(10.0);
        assert!(the_gateway_says(&extra).expect("refused").contains("blade_pitch_deg"));
        let mut missing = request(5000.0, 8.0, 1.225);
        missing["parameters"].as_object_mut().unwrap().remove("air_density_kg_m3");
        assert!(the_gateway_says(&missing).expect("refused").contains("air_density_kg_m3"));
        let mut worded = request(5000.0, 8.0, 1.225);
        worded["parameters"]["airspeed_m_s"] = json!("8.0");
        assert!(the_gateway_says(&worded).expect("refused").contains("must be a number"));
    }

    #[test]
    fn the_description_publishes_the_fields_the_record_and_the_reference() {
        let body = describe_value(PROPELLER_APC_10X7E.name).expect("described");
        assert!(body.get("more").is_none());
        let summary = body["summary"].as_str().unwrap();
        assert!(summary.contains("two blade-section constants, the zero-lift angle (-2.77 degrees) and the minimum drag coefficient (0.027)"));
        assert!(summary.contains("within 4.85 percent of full scale on average (target 4.0), worst 10.16 (target 10.0)"));
        let listed = solvers::list_value();
        let row = listed["solvers"]
            .as_array()
            .unwrap()
            .iter()
            .find(|row| row["solver"] == "propeller_apc_10x7e")
            .expect("listed");
        // The row is the sheet's row since 0.14.4: what it computes, the comparison and the gap.
        assert_eq!(row["summary"], crate::spec_sheet::of(PROPELLER_APC_10X7E.name).unwrap().row());

        let inputs = body["inputs"].as_array().unwrap();
        let names: Vec<&str> = inputs.iter().map(|input| input["name"].as_str().unwrap()).collect();
        assert_eq!(names, ["rotational_speed_rev_per_min", "airspeed_m_s", "air_density_kg_m3"]);
        assert_eq!(inputs[0]["minimum"], 0.0);
        assert_eq!(inputs[0]["minimum_exclusive"], true);
        assert_eq!(inputs[0]["maximum"], propeller::ROTATIONAL_SPEED_MAX_REV_PER_MIN);
        assert_eq!(inputs[1]["minimum"], 0.0);
        assert_eq!(inputs[1]["maximum"], propeller::AIRSPEED_MAX_M_S);
        assert!((propeller::AIRSPEED_MAX_M_S - 102.087).abs() < 1e-9);
        assert_eq!(inputs[2]["minimum"], 0.0);
        assert_eq!(inputs[2]["minimum_exclusive"], true);
        // The published maximum airspeed is the tip Mach limit with the
        // blades still: at every served rotational speed it is refused there.
        let above = the_gateway_says(&request(4007.0, propeller::AIRSPEED_MAX_M_S, 0.5)).expect("refused");
        assert!(above.ends_with("(TipMachAboveIncompressible)."), "{above}");
        assert!(inputs[0]["bound"].as_str().unwrap().contains("4007 to 6531 rev/min with the air moving"));
        assert!(inputs[0]["bound"].as_str().unwrap().contains("4144 to 6542 rev/min at zero airspeed"));
        let density = inputs[2]["bound"].as_str().unwrap();
        assert!(density.contains("No density was compared"));
        assert!(density.contains("44399 to 75516"));
        let bound = inputs[1]["bound"].as_str().unwrap();
        assert!(bound.contains("advance ratio 0.111 to 0.748"));
        assert!(bound.contains("TipMachAboveIncompressible"));
        // Past the compared advance ratios and densities the door takes the
        // question: the low and high ends at both speeds, the windmill, and
        // thin and dense air where the Reynolds band holds.
        for (speed, airspeed, density) in [
            (4007.0, 3.0, 1.25),
            (4007.0, 30.0, 1.225),
            (6531.0, 2.0, 1.225),
            (6531.0, 25.0, 1.1),
            (5000.0, 8.47, 1.6),
            (6531.0, 27.6, 0.8),
        ] {
            assert_eq!(the_gateway_says(&request(speed, airspeed, density)), None, "{speed} {airspeed} {density}");
        }
        assert_eq!(body["locked"]["constants_named_in_refusals"]["REYNOLDS_NUMBER_MIN"], 44399.0);
        assert_eq!(body["locked"]["constants_named_in_refusals"]["REYNOLDS_NUMBER_MAX"], 75516.0);
        assert_eq!(body["locked"]["constants_named_in_refusals"]["TIP_MACH_MAX"], 0.3);
        // The disc balance is checked at acceptance, under its own name.
        assert_eq!(body["invariants"][1]["name"], "thrust_balance_gap");
        assert_eq!(body["invariants"][1]["refusal"], "DiscBalanceBroken");
        assert!(!body["invariants"][1]["compared"].as_str().unwrap().contains("follows from the rings"));

        let record = &body["validation"];
        for part in ["object", "data", "pass_line", "result", "limits"] {
            assert!(record[part].as_str().unwrap().ends_with('.'), "{part}");
        }
        assert!(record["object"].as_str().unwrap().contains("-2.77 degrees"));
        assert!(record["object"].as_str().unwrap().contains("(0.027)"));
        assert!(record["limits"].as_str().unwrap().starts_with("1. Two constants are fitted"));
        let data = record["data"].as_str().unwrap();
        assert!(data.contains("43 points with advance ratio 0.112 to 0.726"));
        assert!(data.contains("64 points with advance ratio 0.111 to 0.748"));
        assert!(!data.contains("0.10 to 0.75"));
        let limits = record["limits"].as_str().unwrap();
        assert!(limits.contains("4007 to 6531 rev/min and advance ratio 0.111 to 0.748 with the air moving, and 4144 to 6542 rev/min at rest"));
        for refusal in body["refusals"].as_array().unwrap() {
            assert!(!refusal["meaning"].as_str().unwrap().contains("..="), "{refusal}");
        }
        assert!(limits.contains("No density was compared"));
        assert!(!limits.contains("outside 4000 to 6600 rev/min"));
        assert_eq!(record["result"], HELD_BACK_RESULT);
        let result = record["result"].as_str().unwrap();
        for stated in [
            "mean error 4.85 percent of full scale (target 4.0), worst 10.16 (target 10.0)",
            "mean error 4.85",
            "worst 10.16",
            "mean 6.02",
            "worst 11.76",
            "mean 3.77 and worst 10.03",
            "the unfitted model errs high where the fitted one errs low",
            "recomputes every statistic to 1e-12 and all 93 rows",
            "agrees with the solver to 2.2e-14",
        ] {
            assert!(result.contains(stated), "{stated}");
        }
        assert!(!result.contains("does better"));
        // Each line's miss has its own cause, and the floor of the measurement is stated.
        let limits = record["limits"].as_str().unwrap();
        for stated in [
            "It is what line A shows",
            "3. Line B has a different cause",
            "from 0.0536 to 0.0512",
            "0.0474 at every speed",
            "give 4.45 on average and 10.24 at worst on line A",
            "2.14 and 5.31",
            "from −2.6 degrees at 4007 rev/min to about −3.5 to −3.8",
            "about 44,000 to 76,000",
            "0.15 to 0.18 percent of full scale on average and 0.29 to 0.35 at worst",
            "Table 3.1",
        ] {
            assert!(limits.contains(stated), "{stated}");
        }
        assert!(!limits.contains("both missed lines show"));
        assert!(record["pass_line"].as_str().unwrap().contains("at most 4.0 percent of full scale and no value beyond 10.0"));

        assert_eq!(body["locked"]["alpha_zero_lift_deg"], -2.77);
        assert_eq!(body["locked"]["cd0"], 0.027);
        assert_eq!(body["locked"]["geometry_r_over_R_chord_over_R_blade_angle_deg"].as_array().unwrap().len(), 18);
        assert_eq!(body["invariants"][0]["tolerance"], 1.0e-9);
        assert_eq!(body["convergence"]["angle_tolerance_rad"], 1.0e-14);

        let url = body["reference"]["url"].as_str().unwrap();
        let retrieved = body["reference"]["retrieved"].as_str().unwrap();
        assert!(url.starts_with("https://m-selig.ae.illinois.edu/props/volume-1/data/"));
        assert!(SOLVER_SOURCE.contains(&format!("<{url}>")), "{url}");
        assert!(SOLVER_SOURCE.contains(&format!("Retrieved {retrieved}")), "{retrieved}");
        assert!(SOLVER_SOURCE.contains(&format!("fn {}()", body["reference"]["test"].as_str().unwrap())));
    }

    #[test]
    fn both_examples_settle_and_the_field_is_the_three_ring_arrays() {
        for parameters in [example(), at_rest_example()] {
            let asked = json!({ "solver": "propeller_apc_10x7e", "parameters": parameters });
            assert_eq!(the_gateway_says(&asked), None);
            let question = PropellerQuestion {
                rotational_speed_rev_per_min: parameters["rotational_speed_rev_per_min"].as_f64().unwrap(),
                airspeed_m_s: parameters["airspeed_m_s"].as_f64().unwrap(),
                air_density_kg_m3: parameters["air_density_kg_m3"].as_f64().unwrap(),
            };
            let solution = propeller::solve(&question).expect("settles");
            let response = solution::seal(&PROPELLER_APC_10X7E, asked, solved(Ok(solution.clone())))
                .expect("sealed");
            assert_eq!(response.document.outcome, "converged");
            let result = response.document.result.as_ref().expect("result");
            assert_eq!(result["thrust_n"], solution.thrust_n);
            assert_eq!(result["ring_thrust_n"].as_array().unwrap().len(), propeller::ANNULI);
            assert_eq!(
                result["shaft_power_w_hex"],
                format!("{:016x}", solution.shaft_power_w.to_bits())
            );
            let Solved::Answer(answer) = solved(Ok(solution)) else {
                panic!("an answer");
            };
            assert_eq!(answer.field.len(), 3 * propeller::ANNULI);
            assert!(field_words().contains("(102)"));
        }
    }
}
