//! The entry for `step_flow_armaly`: steady laminar flow over the
//! backward-facing step of one rig. The numbers come from `genesis_solvers`
//! constants.

use genesis_solvers::step_flow_armaly::{self as step, StepFlow, StepQuestion, StepReject};
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

pub const STEP_FLOW_ARMALY: Solver = Solver {
    name: "step_flow_armaly",
    summary: SUMMARY,
    inputs: &[
        Input {
            name: "reynolds_number",
            kind: Kind::NumberAbove {
                above: step::REYNOLDS_ABOVE,
                maximum: step::REYNOLDS_MAX,
                unit: "1",
            },
            refusal: Some("ReynoldsOutOfRange"),
            bound: "Mean inlet speed times twice the inlet height (10.4 mm) over the kinematic viscosity. The definition is the paper's own (page 478): V two-thirds of the measured maximum inlet velocity, D twice the inlet height. Any Reynolds number above 0 up to 400 is answered; 0 or less, where the viscosity would be infinite or negative, is refused as ReynoldsNotPositive, and above 400 as ReynoldsOutOfRange: the paper, page 482, finds the flow over this step two-dimensional below 400 and three-dimensional from 400 to 6600, and the solver's equations are those of steady two-dimensional flow. The two conditions with measured profiles are 100 and 389; between them the reattachment length is compared with Armaly's plot, and below 100 at 97 alone (5.8 percent low). Below 100 the same equations are settled down to creeping flow, and from 389 to 400 they are settled the same way. At 389 the rig is already leaving two-dimensionality: the paper reports small deviations from two-dimensionality between 7 and 12 step heights there, and the reattachment length it measured, about 8.6 step heights, is 6 to 7 percent longer than this solver's. Solver and measured reattachment agree within 1 percent near Reynolds number 300.",
            only_when: None,
            asked_by_check: true,
        },
        Input {
            name: "station_x_over_s",
            kind: Kind::Number {
                minimum: step::STATION_MIN_X_OVER_S,
                maximum: step::STATION_MAX_X_OVER_S,
                unit: "step heights",
            },
            refusal: Some("StationOutOfRange"),
            bound: "Where the velocity profile is returned: distance downstream of the step over the step height (4.9 mm). Measured profiles were compared at Reynolds number 100 at stations from 0 to 7.76 (Figure 3 of NASA Technical Memorandum 106248) and from 12.04 to 30.31 (Armaly's Figure 15, first-hand, compared at 30 for the circles at 30.31), and at Reynolds number 389 at stations from 0 to 13.57. Out to 30, where the grid ends, is answered at every Reynolds number; above 100 past 13.57 it computes the same equations with no measured profile. No measured profile at any station was compared at a Reynolds number strictly between 100 and 389, or below 100.",
            only_when: None,
            asked_by_check: false,
        },
    ],
    kept_inside: "The geometry of the rig, the grid, the ladder of Reynolds numbers and the tolerances stay inside the solver.",
    result: &[
        output("reattachment_x_over_s", "step heights", Shape::Number, "Where the flow reattaches to the floor, from the step."),
        output("reattachment_mm", "mm", Shape::Number, "The same length."),
        output("floor_shear_sign_changes_x_over_s", "step heights", Shape::Numbers, "Every place downstream of the step where the shear on the floor changes sign, rising: the reattachment last, and before it the end of the eddy in the corner when the grid resolves one."),
        output("upper_wall_shear_sign_changes_x_over_s", "step heights", Shape::Numbers, "Every place where the shear on the upper wall changes sign. Empty when the flow stays attached to it."),
        output("profile_y_over_h", "1", Shape::Numbers, "Height of each of the 51 grid nodes over the channel height (10.1 mm), floor to upper wall."),
        output("profile_u_over_umax", "1", Shape::Numbers, "Streamwise speed over the largest inlet speed at the asked station, at each of those heights."),
    ],
    invariants: &[
        output("largest_imbalance", "solver units", Shape::Number, ""),
        output("outlet_departure", "1", Shape::Number, ""),
    ],
    iterations: "Newton iterations, each one direct solve of the whole field, over every rung of the ladder of Reynolds numbers. On a refusal it is the count at the rung that did not settle, for NotSettled, and null otherwise.",
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

const SUMMARY: &str = "Steady laminar air flow over the backward-facing step of the rig of Armaly and others (1983; step 4.9 mm, channel 5.2 mm opening to 10.1 mm), two-dimensional Navier-Stokes with nothing fitted: reattachment length and the velocity profile at one station, Reynolds number above 0 to 400, where the flow over this step turns three-dimensional, stations out to 30 step heights; compared with measured velocity profiles at 100 and at 389 only, second-hand and digitised, all three lines met at 389 and confirmed by a second scoring, and at 100 also first-hand from Armaly's Figure 15 out to 30.31 step heights, above 100 to 13.57 step heights, and below 100 not at all; the reattachment length at 100 is 14 percent below the sentence of NASA Technical Memorandum 106248 that quotes the measurement, a miss of that sentence, and 6 percent below Armaly's own plot as read first-hand afterwards, inside the mark; at 389 it is 6 to 7 percent below that plot.";

/// Each `StepReject` variant with the sentence from its doc comment. The
/// request itself shows a Reynolds number or a station outside the range, so
/// those three are not sealed.
const REFUSALS: &[Refusal] = &[
    Refusal {
        name: "ReynoldsNotPositive",
        meaning: "`reynolds_number` is not above `REYNOLDS_ABOVE`: the viscosity `2 h / Re` is infinite or negative.",
        sealed: false,
    },
    Refusal {
        name: "ReynoldsOutOfRange",
        meaning: "`reynolds_number` is above `REYNOLDS_MAX`: above 400 the flow over this step is three-dimensional (Armaly et al. 1983, page 482: two-dimensional below 400, three-dimensional from 400 to 6600), and the solver's equations are those of steady two-dimensional flow.",
        sealed: false,
    },
    Refusal {
        name: "StationOutOfRange",
        meaning: "`station_x_over_s` is outside `STATION_MIN_X_OVER_S` to `STATION_MAX_X_OVER_S`, where the grid ends.",
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
        meaning: "The largest imbalance rose above `IMBALANCE_CAP` in the viscous scale.",
        sealed: true,
    },
    Refusal {
        name: "NotSettled",
        meaning: "The largest imbalance was still above the tolerance after `MAX_NEWTON_ITERATIONS` iterations at one Reynolds number.",
        sealed: true,
    },
    Refusal {
        name: "OutletNotDeveloped",
        meaning: "The outlet profile departs from the developed parabola by more than `OUTLET_TOLERANCE`.",
        sealed: true,
    },
    Refusal {
        name: "FlowPatternNotRecognised",
        meaning: "The shear on the floor does not show one reattachment, with at most a corner eddy before it.",
        sealed: true,
    },
];

/// The `StepReject` variant name, without its fields.
pub fn reject_name(reject: &StepReject) -> &'static str {
    match reject {
        StepReject::ReynoldsNotPositive { .. } => "ReynoldsNotPositive",
        StepReject::ReynoldsOutOfRange { .. } => "ReynoldsOutOfRange",
        StepReject::StationOutOfRange { .. } => "StationOutOfRange",
        StepReject::NonFiniteValue => "NonFiniteValue",
        StepReject::SingularSystem => "SingularSystem",
        StepReject::ImbalanceGrew { .. } => "ImbalanceGrew",
        StepReject::NotSettled { .. } => "NotSettled",
        StepReject::OutletNotDeveloped { .. } => "OutletNotDeveloped",
        StepReject::FlowPatternNotRecognised { .. } => "FlowPatternNotRecognised",
    }
}

fn field_words() -> String {
    "SHA-256 of the count of the numbers that follow as 8 little-endian bytes, which is twice the number of grid nodes (321 columns of 51), then the stream function at every node, column by column from the inlet and floor to upper wall within a column, in mm times the mean inlet speed, then the vorticity at every node in the same order, in mean inlet speeds per mm, all as little-endian f64 bytes. Nodes inside the step hold zero.".to_string()
}

fn sections() -> Sections {
    Sections {
        locked: json!({
            "step_height_mm": step::STEP_HEIGHT_MM,
            "inlet_height_mm": step::INLET_HEIGHT_MM,
            "channel_height_mm": step::CHANNEL_HEIGHT_MM,
            "equations": "Steady two-dimensional incompressible Navier-Stokes, as stream function psi and vorticity omega: u = d(psi)/dy, v = -d(psi)/dx, laplacian(psi) + omega = 0, nu laplacian(omega) = u d(omega)/dx + v d(omega)/dy. Constant density and viscosity.",
            "reynolds_number": "V D / nu: V the mean inlet speed, two-thirds of the largest; D twice the inlet height.",
            "boundaries": "The developed parabola 10 mm upstream of the step. No slip on every wall, the wall vorticity from the stream function one node inside (Thom); at the edge of the step each neighbour sees the wall it faces. No change along the channel at the outlet, 150 mm downstream of the step.",
            "differences": "Second-order central differences on nodes.",
            "intervals_over_step_height": step::INTERVALS_OVER_STEP,
            "intervals_over_inlet_height": step::INTERVALS_OVER_INLET,
            "dx_mm": step::DX_MM,
            "inlet_columns": step::INLET_COLUMNS,
            "outlet_columns": step::OUTLET_COLUMNS,
            "ladder_of_reynolds_numbers": step::LADDER,
            "rung_tolerance": step::RUNG_TOLERANCE,
            "fitted": "Nothing. The grid was fixed by comparing the model with itself: with every spacing halved the reattachment length moves by 0.8 percent at Reynolds number 100 and 0.4 percent at 389. Against an independent solver carried to finer grids and against Erturk's table the reattachment length is 1.3 to 1.5 percent low at Reynolds number 100 and within 0.6 percent from 200 up.",
            "solver_units": "Millimetres, and the mean inlet speed as one.",
        }),
        invariants: json!([
            {
                "name": "largest_imbalance",
                "rule": "At every node the vorticity carried by the flow is the vorticity spread by viscosity, which is the momentum balance with the pressure taken out, and the vorticity is that of the stream function. The largest imbalance of either over every node. It says the equations were solved, for any viscosity; it does not say they describe the air. Mass is conserved by the stream function's construction and is not tested.",
                "tolerance": step::IMBALANCE_TOLERANCE,
                "cap": step::IMBALANCE_CAP,
                "unit": "solver units",
                "compared": "every Newton iteration",
                "refusal": "NonFiniteValue, ImbalanceGrew or NotSettled",
            },
            {
                "name": "outlet_departure",
                "rule": "One grid column before the outlet the speed is the developed parabola of the whole channel: the outlet is far enough downstream.",
                "tolerance": step::OUTLET_TOLERANCE,
                "unit": "largest inlet speed",
                "compared": "at acceptance",
                "refusal": "OutletNotDeveloped",
            },
        ]),
        convergence: json!({
            "imbalance_tolerance": step::IMBALANCE_TOLERANCE,
            "max_newton_iterations": step::MAX_NEWTON_ITERATIONS,
            "rule": "Newton's method on the whole field, each iteration one direct banded solve with row exchanges. The flow is first settled at each rung of the ladder below the asked Reynolds number, to the rung tolerance, then at the asked one to the imbalance tolerance. Below Reynolds number 100 the viscous term, and with it the imbalance of any field, grows as 1/Re, so the tolerance and the cap are multiplied by 100/Re there; from 100 up they are as stated. Far below, near Reynolds number 1e-12, the rounding of the viscous term outgrows the flow and the solver stops by name (FlowPatternNotRecognised, ImbalanceGrew or NonFiniteValue); at 1e-6 the reattachment is 0.3507 step heights, the corner eddy of creeping flow. A result is returned when the largest imbalance is within it, the outlet is developed, and the floor shows one reattachment with at most a corner eddy before it; otherwise the solve is refused by name.",
            "refusal": "NotSettled, ImbalanceGrew, SingularSystem, OutletNotDeveloped or FlowPatternNotRecognised",
            "run_time": "The heaviest question, Reynolds number 400, takes 2.2 s on one core of an Apple M4 Pro (2.0 s at 389 in the same measurement), and about 82 MB for the banded matrix. Every question in range is at most that work: three rungs and the asked Reynolds number.",
        }),
        reference: json!({
            "name": step::REFERENCE_NAME,
            "url": step::REFERENCE_URL,
            "retrieved": step::REFERENCE_RETRIEVED,
            "tolerance_fraction": step::REFERENCE_TOLERANCE_FRACTION,
            "test": "erturk_2008_table_1_reattachment",
            "covers": "The reattachment length at Reynolds numbers 100, 200 and 300. Found: 2.835, 4.874 and 6.663 step heights here against 2.878, 4.900 and 6.659: lower by 1.5 and 0.5 percent and higher by 0.1. The table has no row at 389. The rows at 200 and 300 are held by a test that is skipped by default for its run time.",
            "kind": "A reference calculation of the same two-dimensional equations at the rig's expansion ratio, 1.942, with an inlet channel of 20 step heights. NASA Technical Memorandum 106248, page 3, a compressible code at Mach 0.1 on a 161 by 65 grid, gives 3.3 and 8.7 step heights, what a calculation without an inlet channel gives; this solver is 14 and 7 percent below them.",
        }),
    }
}

fn validation() -> [String; 5] {
    let object = "The air channel of Armaly, Durst, Pereira and Schönung (J. Fluid Mech. 127, 473): an inlet channel 5.2 mm high opens over a backward-facing step 4.9 mm high into a channel 10.1 mm high, expansion ratio 1.94, with developed laminar flow at the step and laser-Doppler measurements in the centre plane. The solver settles the steady two-dimensional incompressible flow in that geometry. Nothing is fitted.".to_string();
    let data = "The paper is Experimental and theoretical investigation of backward-facing step flow, Journal of Fluid Mechanics 127, 473-496, doi 10.1017/S0022112083002839. Its page 478 gives the Reynolds number as this solver takes it: \"Re = VD/ν, where V is two-thirds of the measured maximum inlet velocity\", with \"D = 2h\". The measurements at 100 and 389 are the ones replotted and quoted in NASA Technical Memorandum 106248 (NTRS 19930021958): circles of u/Umax against y/H at stations x/S in its Figure 3 (Reynolds number 100) and Figure 4 (389), and one sentence giving the measured reattachment at approximately 3.3 step heights for 100 and approximately 8 for 389. The circles were digitised from a 300 dots-per-inch one-bit scan by a program: second-hand and digitised, two steps from the instrument. At the station on the step, where the flow is the inlet parabola, the circles as read differ from the parabola by 0.041 of the largest inlet speed RMS: reading contributes about 0.005, and the data's own floor is about 0.03, from the scatter of the circles about a smooth curve and from a volume flux that is not conserved from station to station (0.90 to 1.12 of the exact flux at Reynolds number 389). The digitiser read 174 circles downstream of the step in Figure 4; a careful reading has about 187, and of the 174, 7 are false and 20 were not found. Read when the solver was built: Figure 3 and both reattachment lengths. Compared outside it: Figure 4, the whole condition at Reynolds number 389. Read and compared later, first-hand: Armaly's own Figure 15 (page 491 of the paper), circles of the speed in cm/s against the height in mm at Reynolds number 100, read by a program from the 300 dots-per-inch one-bit scan of the paper; 80 circles at the stations 12.04, 16.33, 20.92 and 30.31 step heights. Each speed is taken over the largest inlet speed of the parabola fitted to the 19 circles on the step, 21.19 cm/s; over the largest of those circles, 21.28 cm/s, every figure stays on the same side of its target. Its station 44.90 is past the grid.".to_string();
    let pass_line = "Error measures and targets. On every circle of Figure 4 at a station downstream of the step: RMS difference of u/Umax at most 0.05; at no single station more than 0.08; and at most half that of this solver run at Reynolds number 100 on the same circles. Beside it: the developed parabola, the solver at Reynolds number 100, and the solver at 259.33, the other reading of the memorandum's Reynolds number. The two reattachment lengths were read before the solver was built; their target is 10 percent of the quoted value. Record SHA-256 dd3abc9bbc833852f0a53c3201994edaf23a2bd45edccf3ae3016136c3afce1a.".to_string();
    let limits = "The profiles at 389 are the same rig and instrument at a Reynolds number 3.9 times the one read when the solver was built, the top of the compared range. No measured velocity profile between 100 and 389 was compared, and the solver serves that whole span; the reattachment length is compared there with Armaly's Figure 14 at 149 and 298, read after the solver was built. Velocity profiles were compared at stations from 0 to 7.76 and from 12.04 to 30.31 step heights at Reynolds number 100, and from 0 to 13.57 at 389. A station out to 30 is answered at every Reynolds number with the same equations; above 100 past 13.57 it has no measured profile. Below 100 the flow is answered down to creeping flow, with the reattachment length compared at 97 alone. Between 7.76 and 12.04 at 100 no circle lies, a gap narrower than the 9.39 step heights between the compared stations 20.92 and 30.31. The flow is taken as two-dimensional and steady: above a Reynolds number of 400 the flow over this step is three-dimensional (the paper, page 482), and the solver refuses there; from 389 to 400 it answers with the same equations. At 389 itself the rig is leaving two-dimensionality (the paper: small deviations from two-dimensionality between 7 and 12 step heights) and the measured reattachment is 6 to 7 percent longer than the solver's; the two agree within 1 percent near Reynolds number 300. The three targets hold for any Reynolds number from about 350 to 445, a reattachment length from 7.5 to 8.8 step heights: they separate this flow from the other reading of the Reynolds number and from the Reynolds-100 flow, and do not separate a reattachment at 8.07 from one at 8.6. The side walls of the rig, the compressibility of air and any unsteadiness are left out. The largest_imbalance balance closes for any viscosity: it confirms the equations were solved; how closely they describe the air is the comparison above. The grid error of the reattachment length is 1.3 to 1.5 percent at Reynolds number 100 and under 0.6 percent from 200 up. A proof confirms an answer is unchanged.".to_string();
    [object, data, pass_line, RESULT.to_string(), limits]
}

/// The comparison with the measured profiles and reattachment lengths.
const RESULT: &str = "On the 174 circles of Figure 4 at Reynolds number 389, at nine stations from 2.55 to 13.57 step heights, computed through the gateway (mcp_stdio SHA-256 042bb92d94119f5d684998d7d7ff9b88134188dda06baa6e2aea8be7884bd616): RMS difference 0.0437 of the largest inlet speed (target 0.05); worst station 0.052 (target 0.08); 0.24 of the Reynolds-100 solver's 0.181 (target 0.5). The margin on the first is 0.006. Reading the circles contributes about 0.005 and the data's own floor is about 0.03, so the solver's 0.043 is above the floor, with systematic station means of +0.037 at 3.57 step heights, -0.043 at 6.12 and -0.031 at 7.76. The baselines, each above 0.05: the developed parabola 0.206, the solver at Reynolds number 100 0.181, the solver at 259.33 (the other reading of the memorandum's Reynolds number) 0.088. The digitiser found 186 circles in Figure 4; by eye it leaves out some that the memorandum's line crosses and places a few on the frame. On the conditions read when the solver was built: at Reynolds number 100 the profiles differ by 0.035 over 161 circles (the developed parabola 0.063), and the reattachment length is 2.83 step heights against a quoted approximately 3.3, 14 percent low (target 10); at 389 it is 8.07 against approximately 8, where Armaly's own plot has about 8.6, so that the solver is 6 to 7 percent low there, and where the memorandum's own calculation gives 8.7, the other reading of the Reynolds number 5.96 and a length proportional to Reynolds number 12.8. A second program carrying the same difference equations to 50 digits gives the engine's reattachment length within 1.4e-8 step heights at Reynolds number 389, where the engine accepts the field at an imbalance of 2.7e-10, and within 3e-13 at 100 and 259.33; the profiles agree within 2e-9. The heaviest answer took 2.5 s. A second program on its own digitisation of 187 circles gives 0.0430, 0.0514 and 0.238, and an independent solver is within 1.3 percent of this one in reattachment length. Armaly's own Figure 14 as read from the scan (about 0.05 in length and 3 in Reynolds number): 3.01 step heights at Reynolds number 97, 4.00 at 149, 6.60 at 298 and 8.60 at 387; this solver is 5.8 percent low, 3.2 percent low, 0.4 percent high and 6.5 percent low, each within 10 percent. The 14 percent at Reynolds number 100 is against the memorandum's sentence, whose 3.3 is what a calculation without an inlet channel gives; against the measurement as plotted it is 5.8 percent. The far stations at Reynolds number 100, on 80 circles of Armaly's Figure 15 at 12.04, 16.33, 20.92 and 30.31 step heights, read after the solver was built, the third measure taken against the solver at Reynolds number 389, the other end of the range: RMS difference 0.019 (target 0.05), worst station 0.023 at 20.92 (target 0.08), and 0.43 of the Reynolds-389 flow's 0.044 (target 0.5). The flow there is the developed parabola, and the parabola itself is 0.019 from the circles and the solver at 259.33 is 0.023: the comparison confirms the solver reaches the developed flow. On the same reading the solver is 0.019 from the 19 circles on the step and 0.029 from 149 circles at the stations 2.55 to 7.76, where the memorandum's replot gave 0.035. The circles at 30.31 are compared against the profile at 30, which differs from the one at 20.92 by less than 1e-6.";

fn example() -> Value {
    json!({ "reynolds_number": 100.0, "station_x_over_s": 3.06 })
}

/// The solver's own question, from the values of `parameters`.
fn question(given: &Given) -> Result<StepQuestion, String> {
    Ok(StepQuestion {
        reynolds_number: given.number("reynolds_number")?,
        station_x_over_s: given.number("station_x_over_s")?,
    })
}

/// The Reynolds number's two limits, asked of the solver's own check before
/// a physics permit is taken, so each end is refused by its own name. The
/// sentence is the solver's, with its name for the refusal.
fn check(given: &Given) -> Result<(), String> {
    step::admit(&question(given)?)
        .map_err(|reject| format!("Physical Violation: {reject} ({}).", reject_name(&reject)))
}

fn solve(given: &Given) -> Result<Solved, String> {
    Ok(solved(step::solve(&question(given)?)))
}

/// What the solver returned, in the words every solver shares.
pub fn solved(returned: Result<StepFlow, StepReject>) -> Solved {
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
                    StepReject::NotSettled { iterations, .. } => Some(u64::from(iterations)),
                    _ => None,
                },
            });
        }
    };
    let one = |name: &str, value: f64| (name.to_string(), Published::Number(value));
    let many = |name: &str, values: Vec<f64>| (name.to_string(), Published::Numbers(values));
    let mut field = Vec::with_capacity(flow.stream_function.len() * 2);
    field.extend_from_slice(&flow.stream_function);
    field.extend_from_slice(&flow.vorticity);
    Solved::Answer(Answer {
        iterations: u64::from(flow.newton_iterations),
        result: vec![
            one("reattachment_x_over_s", flow.reattachment_x_over_s),
            one("reattachment_mm", flow.reattachment_mm),
            many("floor_shear_sign_changes_x_over_s", flow.floor_shear_sign_changes_x_over_s),
            many(
                "upper_wall_shear_sign_changes_x_over_s",
                flow.upper_wall_shear_sign_changes_x_over_s,
            ),
            many("profile_y_over_h", flow.profile_y_over_h),
            many("profile_u_over_umax", flow.profile_u_over_umax),
        ],
        invariants: vec![
            one("largest_imbalance", flow.largest_imbalance),
            one("outlet_departure", flow.outlet_departure),
        ],
        field,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::solvers::describe_value;

    const SOLVER_SOURCE: &str = include_str!("../../genesis_solvers/src/step_flow_armaly.rs");

    /// `(variant, doc sentence)` for every variant of `StepReject`, read from
    /// the solver's source. An intra-doc link is written as its bare code span.
    fn documented_rejects() -> Vec<(String, String)> {
        let start = SOLVER_SOURCE
            .find("pub enum StepReject {")
            .expect("StepReject is in the solver");
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
        let body = describe_value(STEP_FLOW_ARMALY.name).expect("described");
        let refusals = body["refusals"].as_array().expect("refusals");
        assert_eq!(refusals.len(), documented.len());
        for refusal in refusals {
            let at_the_door = refusal["name"] == "ReynoldsOutOfRange"
                || refusal["name"] == "StationOutOfRange"
                || refusal["name"] == "ReynoldsNotPositive";
            let reported = if at_the_door { "422, no seal" } else { "200, outcome refused, sealed" };
            assert_eq!(refusal["reported_as"], reported, "{}", refusal["name"]);
        }
    }

    #[test]
    fn the_reference_is_the_one_stored_in_the_solver_test() {
        let body = describe_value(STEP_FLOW_ARMALY.name).expect("described");
        let url = body["reference"]["url"].as_str().expect("url");
        assert!(url.starts_with("https://doi.org/10.1016/"), "{url}");
        assert!(SOLVER_SOURCE.contains(&format!("<{url}>")), "{url}");
        let retrieved = body["reference"]["retrieved"].as_str().expect("retrieved");
        assert!(SOLVER_SOURCE.contains(&format!("Retrieved {retrieved}")), "{retrieved}");
        let test = body["reference"]["test"].as_str().expect("test");
        assert!(SOLVER_SOURCE.contains(&format!("fn {test}()")), "{test}");
    }

    /// The list an agent reads first carries the standing: that nothing is
    /// fitted, the range, and how the comparison came out.
    #[test]
    fn the_summary_carries_the_standing() {
        let summary = crate::spec_sheet::summary_of(STEP_FLOW_ARMALY.name, STEP_FLOW_ARMALY.summary);
        assert!(summary.contains("Nothing fitted"));
        assert!(summary.contains("above 0 to 400"));
        assert!(summary.contains("174 digitised circles"));
        assert!(summary.contains("within 0.0437 of the largest inlet speed RMS (target 0.05)"));
        assert!(summary.contains("6 to 7 percent short of Armaly's plot at 389"));
        assert!(summary.ends_with("(StationOutOfRange)."));
        assert_eq!(body_reference_tolerance(), 0.02);
        let body = describe_value(STEP_FLOW_ARMALY.name).expect("described");
        let limits = body["validation"]["limits"].as_str().expect("limits");
        assert!(limits.contains("No measured velocity profile between 100 and 389 was compared"));
        assert!(limits.contains("with Armaly's Figure 14 at 149 and 298"));
        assert!(!limits.contains("No measurement between"));
        assert!(limits.contains("closes for any viscosity"));
        assert_eq!(body["locked"]["fitted"].as_str().expect("fitted").split('.').next(), Some("Nothing"));
    }

    fn body_reference_tolerance() -> f64 {
        describe_value(STEP_FLOW_ARMALY.name).expect("described")["reference"]["tolerance_fraction"].as_f64().expect("a number")
    }

    #[test]
    fn every_published_name_is_in_the_entry() {
        let flow = match solved(step::solve(&StepQuestion {
            reynolds_number: 100.0,
            station_x_over_s: 3.06,
        })) {
            Solved::Answer(answer) => answer,
            Solved::Refused(refused) => panic!("{}", refused.sentence),
        };
        assert_eq!(flow.result.len(), STEP_FLOW_ARMALY.result.len());
        for ((name, value), output) in flow.result.iter().zip(STEP_FLOW_ARMALY.result) {
            assert_eq!(name, output.name);
            assert_eq!(value.shape(), output.shape);
        }
        assert_eq!(flow.invariants.len(), STEP_FLOW_ARMALY.invariants.len());
        for ((name, _), output) in flow.invariants.iter().zip(STEP_FLOW_ARMALY.invariants) {
            assert_eq!(name, output.name);
        }
        assert_eq!(flow.field.len(), 2 * 321 * 51);
    }
}
