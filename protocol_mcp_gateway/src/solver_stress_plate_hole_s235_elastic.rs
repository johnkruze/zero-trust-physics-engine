//! The entry for `stress_plate_hole_s235_elastic`: the elastic stress and
//! displacement field of the S235 steel plate with a hole of Tröger,
//! Hartmann, Anton and Wessels, the companion of `stress_plate_hole_s235`,
//! calibrated to the record. The numbers come from the `genesis_solvers`
//! constants and the sentence of each refusal from the solver's source.
//! Stamped from the kit.

use genesis_solvers::stress_plate_hole_s235_elastic::{self as plate, ElasticPlateField, ElasticPlateQuestion, ElasticPlateReject};
use serde_json::{Value, json};

use crate::solvers::{Answer, Given, Input, Kind, Output, Published, Refusal, Refused, Sections, Shape, Solved, Solver};

const fn output(name: &'static str, unit: &'static str, shape: Shape, note: &'static str) -> Output {
    Output { name, unit, shape, each: &[], note, asked_by: None }
}

const fn points(name: &'static str, minimum: f64, maximum: f64, bound: &'static str) -> Input {
    Input {
        name,
        kind: Kind::Numbers { fewest: 0, most: plate::MAX_POINTS, minimum, maximum, unit: "mm" },
        refusal: Some("PointOutsidePlate"),
        bound,
        only_when: None,
        asked_by_check: true,
    }
}

pub const STRESS_PLATE_HOLE_S235_ELASTIC: Solver = Solver {
    name: "stress_plate_hole_s235_elastic",
    summary: SUMMARY,
    inputs: &[
        // Every limit is asked by the solver's own check, in its order: a
        // point's limits span both lists.
        Input {
            name: "traction_mpa",
            kind: Kind::NumberAbove { above: 0.0, maximum: f64::MAX, unit: "MPa" },
            refusal: Some("TractionNotTensile"),
            bound: "The axial traction on the gross section of the parallel length, 20.05 mm by 3.05 mm (the force over 61.15 mm²), any above 0 (TractionNotTensile at 0 or below). The field is elastic and linear in the traction. From 590.05 MPa, on a scan by 0.01 MPa, the strain at the hole passes 1 percent and the question is refused as StrainBeyondSmallStrain; 590.04 MPa is the last traction answered. Compared, after the fact, at 106.26 MPa, the laboratory's one load; above it the answer carries past_compared_traction. The record gives no yield strength of this sheet: past it, wherever it is, this field is not that steel's, the steel yielding first at the hole's edge, where the elastic stress is 3.742 times the traction (397.6 MPa at 106.26 MPa; the grade's least yield strength, 235 MPa, is reached there at 62.80 MPa). Every traction above 0 is answered on the same equations, solved in units scaled to it; below about 3.3e-152 MPa the energies in joules, and below about 1e-304 MPa the displacements, are smaller than the smallest normal 64-bit number, 2.2e-308, and are served as the format rounds them, to a subnormal number or zero.",
            only_when: None,
            asked_by_check: true,
        },
        points(
            "points_x_mm",
            -plate::PARALLEL_HALF_LENGTH_MM,
            plate::PARALLEL_HALF_LENGTH_MM,
            "Where the field is read, along the bar, in mm from the hole's centre: -45 to 45 mm, the parallel length (PointOutsidePlate past it, where the field depends on how the clamps carry the load). One entry for each point, its y at the same place of points_y_mm (PointCountsDiffer when the two lists differ in length), at most 1,000 points (TooManyPoints); a point nearer the hole's centre than 4 mm is PointInsideHole. The lists may be empty.",
        ),
        points(
            "points_y_mm",
            -plate::WIDTH_MM / 2.0,
            plate::WIDTH_MM / 2.0,
            "Where the field is read, across the bar, in mm from the hole's centre: -10.025 to 10.025 mm, the bar's two sides.",
        ),
    ],
    kept_inside: "The bar's shape, the steel's constants (Young's modulus 218,540 MPa and Poisson's ratio 0.2835, stress_plate_hole_s235's, fitted to the opened band of the laboratory's field; no yield), the meshes and the tolerances stay inside the solver.",
    result: &[
        output("axial_force_n", "N", Shape::Number, "The force the bar carries: the traction times 20.05 mm times 3.05 mm."),
        output("net_section_stress_mpa", "MPa", Shape::Number, "The force over the net section through the hole, 12.05 mm by 3.05 mm."),
        output("hole_edge_stress_mpa", "MPa", Shape::Number, "The stress along the bar at the top of the hole's edge, where the net section meets it, carried from the integration points to that node: the peak stress, 397.6 MPa at 106.26 MPa."),
        output("stress_concentration_gross", "1", Shape::Number, "The stress at the hole's edge over the traction: 3.742, a property of the bar's shape (Kirsch's infinite plate gives 3)."),
        output("stress_concentration_net", "1", Shape::Number, "The same over the net section stress: 2.2489 (Howland's solution, as Peterson's cubic gives it, 2.2366; Heywood's formula 2.2171)."),
        output("peak_von_mises_mpa", "MPa", Shape::Number, "The largest von Mises stress of the field over its integration points, where the material law is evaluated: 393.3 MPa at 106.26 MPa, 3.701 times the traction. Where it is above this sheet's yield strength, which the record does not give, the steel there yields and the field is not that steel's."),
        output("largest_strain", "1", Shape::Number, "The largest principal strain in size over the integration points, in the plane and through the thickness: the measure of the small-strain limit, 0.01."),
        output("parallel_section_elongation_mm", "mm", Shape::Number, "The change of length of the 90 mm parallel length along its axis."),
        output("strain_energy_j", "J", Shape::Number, "The elastic strain energy stored in the bar out to 75 mm each side of the hole."),
        output("boundary_work_j", "J", Shape::Number, "The work of the load and the reactions rising from zero to the full traction, half the final forces times the displacement. It is the strain energy, the ledger closed from these two numbers (energy_residual): Clapeyron's theorem."),
        output("hole_edge_traction_fraction", "1", Shape::Number, "The largest normal or shear stress the field leaves on the hole's free edge, over the largest hoop stress there: zero in the exact field, it measures the carrying of stress to the edge: 6.3e-4."),
        output("past_compared_traction", "1", Shape::Number, "1 where the traction is above 106.26 MPa, the one load the record compares, 0 at or below it. The answer is the same arithmetic either way; above it every stress of the field is above any the sheet was compared at."),
        output("point_displacement_x_mm", "mm", Shape::Numbers, "The displacement along the bar at each asked point, with the hole's centre fixed."),
        output("point_displacement_y_mm", "mm", Shape::Numbers, "The displacement across the bar at each asked point."),
        output("point_stress_xx_mpa", "MPa", Shape::Numbers, "The stress along the bar at each asked point."),
        output("point_stress_yy_mpa", "MPa", Shape::Numbers, "The stress across the bar at each asked point."),
        output("point_stress_xy_mpa", "MPa", Shape::Numbers, "The shear stress at each asked point."),
        output("point_von_mises_mpa", "MPa", Shape::Numbers, "The von Mises stress at each asked point, of the stresses above; through the thickness the stress is zero (plane stress)."),
        output("point_strain_xx", "1", Shape::Numbers, "The strain along the bar at each asked point, from the displacement."),
        output("point_strain_yy", "1", Shape::Numbers, "The strain across the bar at each asked point."),
        output("point_strain_xy", "1", Shape::Numbers, "The shear strain at each asked point, half the engineering shear."),
    ],
    invariants: &[
        output("equilibrium_residual", "1", Shape::Number, ""),
        output("energy_residual", "1", Shape::Number, ""),
        output("section_force_residual", "1", Shape::Number, ""),
        output("displacement_change", "1", Shape::Number, ""),
        output("edge_stress_change", "1", Shape::Number, ""),
    ],
    iterations: "0: the elastic field is one direct solve of the full traction on each mesh. On a refusal it is null.",
    field: field_words,
    answer: "a settled field",
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

const SUMMARY: &str = "Calibrated to the record, not a blind test: the companion of stress_plate_hole_s235, the S235 steel plate with a 4 mm hole of Tröger, Hartmann, Anton and Wessels, with the steel not yielding, its structure chosen after stress_plate_hole_s235's held-back ring had been scored, because the strain line separates the elastic field from yield at 235 MPa; on that solver's three lines at 106.26 MPa, after the fact, it meets all three (displacement rms 0.638 and 0.486 um against 2.0 and 1.5; 0.567 um pooled against Kirsch's 1.015; the strain 1.5 mm from the hole's edge over the far strain 1.917 against 1.889 measured, the measured 1.4 percent below, against 20), the numbers stress_plate_hole_s235's record had reported beside its blind miss. The full in-plane stress tensor and displacement field of the plate (parallel length 20.05 by 3.05 mm, d/W 0.399) pulled at any traction above 0: plane stress, Hooke's law with stress_plate_hole_s235's own Young's modulus and Poisson's ratio and nothing fitted here, settled on its finite-element meshes with its force and energy ledger, stress concentration 2.249 on the net section; answered to 590.04 MPa, where the strain at the hole reaches 1 percent; above 106.26 MPa the answer carries past_compared_traction. The record gives no yield strength of this sheet: past it this field is not that steel's.";

/// Each `ElasticPlateReject` variant with the sentence from its doc
/// comment. A refusal the request itself shows is not sealed.
const REFUSALS: &[Refusal] = &[
    Refusal { name: "InputNotFinite", meaning: "An input was NaN or infinite.", sealed: false },
    Refusal {
        name: "TractionNotTensile",
        meaning: "The traction is not a pull: pushed, the strip between the clamps is a column that can bow out of its plane, and the field in the plane holds only while it stays flat.",
        sealed: false,
    },
    Refusal { name: "PointCountsDiffer", meaning: "The lists of the points' `x` and `y` are not the same length.", sealed: false },
    Refusal { name: "TooManyPoints", meaning: "More points than `MAX_POINTS`.", sealed: false },
    Refusal {
        name: "PointOutsidePlate",
        meaning: "A point is outside the parallel length: past its end, where the field depends on how the clamps carry the load, or beyond its sides.",
        sealed: false,
    },
    Refusal { name: "PointInsideHole", meaning: "A point is inside the hole, where there is no steel.", sealed: false },
    Refusal { name: "NonFiniteValue", meaning: "A value of the field was NaN or infinite.", sealed: true },
    Refusal { name: "MeshInverted", meaning: "An element of the mesh is turned inside out.", sealed: true },
    Refusal { name: "SingularStiffness", meaning: "The stiffness of the elastic field has a pivot that is not positive.", sealed: true },
    Refusal {
        name: "StrainBeyondSmallStrain",
        meaning: "The largest principal strain passed `SMALL_STRAIN_LIMIT`, where the linear measure of strain no longer holds.",
        sealed: true,
    },
    Refusal { name: "EquilibriumBroken", meaning: "The largest nodal force imbalance of the settled field is above `IMBALANCE_TOLERANCE`.", sealed: true },
    Refusal { name: "EnergyLedgerBroken", meaning: "The work of the boundary forces departs from the strain energy by more than `ENERGY_TOLERANCE` of it.", sealed: true },
    Refusal {
        name: "SectionForceBroken",
        meaning: "The force the settled field carries across a vertical cut departs from the applied force by more than `SECTION_FORCE_TOLERANCE`.",
        sealed: true,
    },
    Refusal { name: "NotSettled", meaning: "Two successive meshes did not agree within the settling fractions by the finest, `MAX_LEVEL`.", sealed: true },
];

fn field_words() -> String {
    "SHA-256 of the count of the numbers that follow as 8 little-endian bytes, which is 7 times the nodes of the accepted mesh (4,033 nodes of 1,280 elements in the quarter x, y >= 0), then for each node in turn its x and y in mm from the hole's centre, its displacement u_x and u_y in mm, and its stresses sigma_xx, sigma_yy and tau_xy in MPa (carried from the integration points and averaged over the elements that share the node), all as little-endian f64 bytes. It is the field hash stress_plate_hole_s235 serves for the same traction wherever its steel has not yielded, below 63.49 MPa.".to_string()
}

fn sections() -> Sections {
    Sections {
        locked: json!({
            "equations": "Plane stress, small strain: div sigma = 0 in the bar, sigma . n = 0 on its sides and the hole's edge, a uniform traction on its end. Hooke's law sigma = C epsilon, with no yield.",
            "companion_to": "stress_plate_hole_s235: the same bar, constants, meshes and ledger, with von Mises yield at 235 MPa. This solver's field is, bit for bit, that solver's field with the steel not yielding, the one its record reported and did not gate.",
            "calibrated": "Calibrated to the record, not a blind test: the structure, no yield, was chosen on 2026-10-15 after stress_plate_hole_s235's held-back ring had been scored and its result read (evidence/research/stress_plate_s235/companion_elastic/CALIBRATION.md). Nothing was fitted to the ring: the two constants are stress_plate_hole_s235's, fitted on the opened band before its scoring.",
            "hole_radius_mm": plate::HOLE_RADIUS_MM,
            "width_mm": plate::WIDTH_MM,
            "thickness_mm": plate::THICKNESS_MM,
            "parallel_length_mm": 2.0 * plate::PARALLEL_HALF_LENGTH_MM,
            "fillet_radius_mm": genesis_solvers::stress_plate_hole_s235::FILLET_RADIUS_MM,
            "grip_width_mm": plate::WIDTH_MM + 2.0 * genesis_solvers::stress_plate_hole_s235::FILLET_RISE_MM,
            "loaded_end_mm": plate::LOADED_END_MM,
            "youngs_modulus_mpa": plate::YOUNGS_MODULUS_MPA,
            "poisson_ratio": plate::POISSON_RATIO,
            "yield_strength": "None: the steel does not yield in this field. The record gives no yield strength of this sheet. Its authors state that the machine's stress-strain curve stayed straight and that no permanent deformation was found after the test: the sheet stayed elastic in the large at 106.26 MPa. Where this sheet yields is not in the record.",
            "domain": "The quarter x, y >= 0 of the bar with its two mirror planes, out to 75 mm from the hole's centre: the parallel length, the fillet of 25 mm radius and 15 mm of the 30 mm wide grip end, where a uniform traction carries the bar's force. Moving that end to 105 mm moves the stress of the parallel length by at most 0.05 MPa.",
            "elements": "Eight-node quadrilaterals, 3 by 3 Gauss points, mapped about the hole and graded toward it; meshes of 80, 320 and 1,280 elements per quarter, each halving every element of the one before. On the finest the first element at the hole is 0.13 to 0.23 mm deep and 0.20 mm along the edge.",
            "fitted": "Nothing is fitted here. Young's modulus 218,540 MPa and Poisson's ratio 0.2835 are stress_plate_hole_s235's, fitted by that solver on the opened band of the laboratory's field only (the points 16 mm or more from the hole along the bar, each of its two bands with its own rigid motion), before its held-back ring was scored. The bands alone give 206,500 and 230,000 MPa.",
        }),
        invariants: json!([
            {
                "name": "equilibrium_residual",
                "rule": "The nodal forces of the stresses balance the applied traction at every free node after the direct solve: the largest imbalance over the largest boundary nodal force (load and reactions), 7.8e-13 at 106.26 MPa. It says the equations were solved, not that they describe the steel.",
                "tolerance": plate::IMBALANCE_TOLERANCE,
                "unit": "1",
                "held": "at the solve of every mesh, and at acceptance",
                "refusal": "EquilibriumBroken",
            },
            {
                "name": "energy_residual",
                "rule": "The work of the boundary forces rising from zero to the full traction, half the final forces times the displacement, equals the strain energy stored: boundary_work_j = strain_energy_j (Clapeyron's theorem). Booked against the end forces alone the work would be twice the strain energy, a residual of one half. The largest residual over every mesh, over the work: about 1.7e-13.",
                "tolerance": plate::ENERGY_TOLERANCE,
                "unit": "1",
                "held": "on every mesh",
                "refusal": "EnergyLedgerBroken",
            },
            {
                "name": "section_force_residual",
                "rule": "The force the stresses carry across a vertical cut equals the applied force: five cuts, the net section through the hole, the side of the square about it at 10.025 mm, the middle of the parallel length, the start and the end of the fillet at 45 and 60 mm. The largest departure over the applied force; 1.09e-3, at the end of the fillet.",
                "tolerance": plate::SECTION_FORCE_TOLERANCE,
                "unit": "1",
                "held": "at acceptance",
                "refusal": "SectionForceBroken",
            },
            {
                "name": "displacement_change",
                "rule": "The largest change of displacement at the nodes the two finest meshes share in the parallel length, over its largest displacement: 1.37e-4 at every traction.",
                "tolerance": plate::DISPLACEMENT_SETTLE,
                "unit": "1",
                "held": "between the two finest meshes",
                "refusal": "NotSettled",
            },
            {
                "name": "edge_stress_change",
                "rule": "The change of the stress at the top of the hole's edge between the two finest meshes, over itself: 3.8e-4.",
                "tolerance": plate::STRESS_SETTLE,
                "unit": "1",
                "held": "between the two finest meshes",
                "refusal": "NotSettled",
            },
        ]),
        convergence: json!({
            "displacement_settle": plate::DISPLACEMENT_SETTLE,
            "stress_settle": plate::STRESS_SETTLE,
            "max_level": plate::MAX_LEVEL,
            "imbalance_tolerance": plate::IMBALANCE_TOLERANCE,
            "rule": "Each mesh is solved once, the elastic field of the full traction by one direct solve. The finest of two successive meshes is accepted when they agree within the two settling fractions; a limit is named when the finest mesh reaches it, or two successive meshes do. At every traction answered the accepted mesh is the finest, 1,280 elements. The field is solved in MPa times the power of two that carries the traction into [1, 2), each value carried back once, so that no product leaves the range of 64-bit numbers.",
            "refusal": "NotSettled or StrainBeyondSmallStrain",
            "run_time": "On one core of an Apple M4 Pro in a release build, through the stdio connection, measured on 2026-10-15: 0.16 s with 1,000 points and 0.02 s with none, the same at every traction answered; a refusal past the strain limit in 0.02 s or less.",
        }),
        reference: json!({
            "name": plate::REFERENCE_NAME,
            "url": plate::REFERENCE_URL,
            "retrieved": plate::REFERENCE_RETRIEVED,
            "test": "the_strain_near_the_hole_after_the_fact",
            "covers": "Line P3 of stress_plate_hole_s235's comparison, after the fact, from the 40 measured points of its two windows: the strain 1.5 mm from the hole's edge over the far strain, 1.889 measured and 1.917 by this solver, 1.4 percent apart. The whole after-the-fact comparison is in the validation; calibrated to the record, not a blind test.",
            "kind": "Measurements of the specimen itself, read with the record open. The solver's tests also hold its field, bit for bit, to stress_plate_hole_s235's with the steel not yielding, whose tests hold it to Kirsch's field imposed on a square around the hole (stress concentration 2.9996 against 3) and to a strip ten holes wide (3.035 against Howland's 3.025 and Heywood's 3.032).",
        }),
    }
}

fn validation() -> [String; 5] {
    let object = "A flat tensile bar of S235 low-carbon steel, 3.05 mm thick, its parallel length 20.05 mm wide and 90 mm long with a hole of radius 4 mm at its centre, fillets of radius 25 mm to grip ends 30 mm wide, pulled in a ZwickRoell Z100 machine at Clausthal University of Technology on 2023-11-16 to a traction of 106.26 N/mm² on its gross section (6,498 N). The solver settles the plane-stress field of the bar with the steel not yielding, on stress_plate_hole_s235's constants: nothing is fitted here.".to_string();
    let data = "Tröger, Hartmann, Anton, Wessels (Clausthal University of Technology, TU Braunschweig), Digital image correlation measurement of linear elastic steel specimen, Zenodo 10.5281/zenodo.11257192, version 1.0, CC BY 4.0, retrieved 2026-10-14: the surface displacement u_x and u_y at 5,475 points of the parallel length, by stereo digital image correlation (GOM ARAMIS 12M, 25 px facets), one load state, no uncertainty given for each point. The traction is the one Anton et al. give (arXiv 2405.18311, section 6.1; Adv. Model. Simul. Eng. Sci. 12 (2025) 12) from the machine's force. The same sets as stress_plate_hole_s235's comparison: the 3,269 opened points 16 mm or more from the hole's centre along the bar, and the 1,186 held-back points 4.5 to 12 mm from the hole's centre, all of them read before this solver was built.".to_string();
    let pass_line = "stress_plate_hole_s235's lines, fixed on 2026-10-14 before its held-back ring was opened (protocol SHA-256 000657104b0a61e8c6f86935e73ac456315d3ec4fe94139f0bed258c62471880), applied unchanged, by its own scoring functions: after one rigid motion fitted to the opened points, P1, rms of the u_x error at most 2.0 um and of u_y at most 1.5 um over the held-back ring; P2, the pooled rms below that of a plate without a hole and below that of Kirsch's infinite plate with the same constants; P3, the strain read by a plane fitted within 1.5 mm of (0, 5.5) and (0, -5.5) mm, over the far strain, within 20 percent of the solver's same reading. They were fixed for stress_plate_hole_s235, not for this solver.".to_string();
    let limits = "The comparison is at the one load state the record holds, 106.26 MPa; the field is elastic and scales with the traction from 0 to 590.04 MPa, and above 106.26 MPa the answer carries past_compared_traction. The record gives no yield strength of this sheet: its measured strain 1.5 mm from the hole's edge is the elastic field's, and its authors found the machine's curve straight and no permanent deformation after the test, but the elastic stress at the edge, 397.6 MPa at 106.26 MPa, is not shown to be carried there, since the ring read starts 0.5 mm from the edge. Past the sheet's yield strength, wherever it is, this field is not that steel's. The two bands of the opened field strain 11 percent apart at the same force, so with one modulus the u_x error is a shift, larger left of the hole. Plane stress in a sheet 3.05 mm thick about a hole of radius 4 mm (t/R = 0.76): the surface the camera sees may differ by a few percent near the hole. The sheet is taken as uniform and the same in every direction, the hole as centred. Points are served in the parallel length only. The ledger shows the equations were solved, not that they describe the steel. A proof shows that an answer was not changed, not that the model matches the steel.".to_string();
    [object, data, pass_line, RESULT.to_string(), limits]
}

/// The outcome of the after-the-fact comparison with the held-back ring.
pub const RESULT: &str = "Calibrated to the record, not a blind test: this companion of stress_plate_hole_s235 was built on 2026-10-15 with that solver's held-back ring already scored and read, and its structure, no yield, chosen because the strain line separated the elastic field from yield at 235 MPa; stress_plate_hole_s235 is the scored original, compared before this companion was built, and by its rule it missed; its own record says that comparison was not blind in the ring. After the fact, on stress_plate_hole_s235's three lines at 106.26 MPa, over the same 1,186 held-back points 4.5 to 12 mm from the hole's centre, by that comparison's own scoring functions on this solver's field as the engine serves it (evidence/research/stress_plate_s235/companion_elastic/CALIBRATION.md, out/scored_companion.json): P1, displacement: rms 0.638 um in u_x and 0.486 um in u_y against 2.0 and 1.5: met. P2: pooled rms 0.567 um, against 1.818 um for the plate without a hole and 1.015 um for Kirsch's infinite plate: met. P3, the strain 1.5 mm from the hole's edge over the far strain: 1.889 measured, 1.917 by this solver, the measured 1.4 percent below the solver's, against a line of 20: met, well inside the 9 percent scatter of the read. These are the numbers stress_plate_hole_s235's record reported, not gated, for its field with the steel not yielding, read before this solver was built. In displacement the elastic and the yielding fields agree within 0.02 um in every ring from 4.5 to 12 mm: the strain line alone tells them apart.";

fn example() -> Value {
    json!({ "traction_mpa": plate::COMPARED_TRACTION_MPA, "points_x_mm": [0.0, 0.0, 4.5, -20.0], "points_y_mm": [4.0, 10.025, 0.0, 5.0] })
}

/// The `ElasticPlateReject` variant name, without its fields.
pub fn reject_name(reject: &ElasticPlateReject) -> &'static str {
    reject.name()
}

/// The solver's own question, from the values of `parameters`.
fn question(given: &Given) -> Result<ElasticPlateQuestion, String> {
    Ok(ElasticPlateQuestion {
        traction_mpa: given.number("traction_mpa")?,
        points_x_mm: given.numbers("points_x_mm")?.to_vec(),
        points_y_mm: given.numbers("points_y_mm")?.to_vec(),
    })
}

/// Every limit the request shows, asked of the solver's own check before a
/// physics permit is taken. The sentence is the solver's, with its name for
/// the refusal.
fn check(given: &Given) -> Result<(), String> {
    plate::admit(&question(given)?).map_err(|reject| format!("Physical Violation: {reject} ({}).", reject_name(&reject)))
}

fn solve(given: &Given) -> Result<Solved, String> {
    Ok(solved(plate::solve(&question(given)?)))
}

/// What the solver returned, in the words every solver shares.
pub fn solved(returned: Result<ElasticPlateField, ElasticPlateReject>) -> Solved {
    let field = match returned {
        Ok(field) => field,
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
    let flag = |name: &str, value: bool| (name.to_string(), Published::Number(if value { 1.0 } else { 0.0 }));
    let many = |name: &str, values: Vec<f64>| (name.to_string(), Published::Numbers(values));
    let at = |read: fn(&genesis_solvers::stress_plate_hole_s235::PointField) -> f64| -> Vec<f64> { field.points.iter().map(read).collect() };
    Solved::Answer(Answer {
        iterations: 0,
        result: vec![
            one("axial_force_n", field.axial_force_n),
            one("net_section_stress_mpa", field.net_section_stress_mpa),
            one("hole_edge_stress_mpa", field.hole_edge_stress_mpa),
            one("stress_concentration_gross", field.stress_concentration_gross),
            one("stress_concentration_net", field.stress_concentration_net),
            one("peak_von_mises_mpa", field.peak_von_mises_mpa),
            one("largest_strain", field.largest_strain),
            one("parallel_section_elongation_mm", field.parallel_section_elongation_mm),
            one("strain_energy_j", field.strain_energy_j),
            one("boundary_work_j", field.boundary_work_j),
            one("hole_edge_traction_fraction", field.hole_traction_residual),
            flag("past_compared_traction", field.past_compared_traction),
            many("point_displacement_x_mm", at(|p| p.displacement_x_mm)),
            many("point_displacement_y_mm", at(|p| p.displacement_y_mm)),
            many("point_stress_xx_mpa", at(|p| p.stress_xx_mpa)),
            many("point_stress_yy_mpa", at(|p| p.stress_yy_mpa)),
            many("point_stress_xy_mpa", at(|p| p.stress_xy_mpa)),
            many("point_von_mises_mpa", at(|p| p.von_mises_mpa)),
            many("point_strain_xx", at(|p| p.strain_xx)),
            many("point_strain_yy", at(|p| p.strain_yy)),
            many("point_strain_xy", at(|p| p.strain_xy)),
        ],
        invariants: vec![
            one("equilibrium_residual", field.equilibrium_residual),
            one("energy_residual", field.energy_residual),
            one("section_force_residual", field.section_force_residual),
            one("displacement_change", field.displacement_change),
            one("edge_stress_change", field.stress_change),
        ],
        field: field.nodes,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::solvers::describe_value;

    const SOLVER_SOURCE: &str = include_str!("../../genesis_solvers/src/stress_plate_hole_s235_elastic.rs");

    /// `(variant, doc sentence)` for every variant of `ElasticPlateReject`,
    /// read from the solver's source.
    fn documented_rejects() -> Vec<(String, String)> {
        let start = SOLVER_SOURCE.find("pub enum ElasticPlateReject {").expect("ElasticPlateReject is in the solver");
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
        let body = describe_value(STRESS_PLATE_HOLE_S235_ELASTIC.name).expect("described");
        let refusals = body["refusals"].as_array().expect("refusals");
        assert_eq!(refusals.len(), documented.len());
        for (refusal, listed) in refusals.iter().zip(REFUSALS) {
            let reported = if listed.sealed { "200, outcome refused, sealed" } else { "422, no seal" };
            assert_eq!(refusal["reported_as"], reported, "{}", refusal["name"]);
        }
    }

    #[test]
    fn the_reference_is_the_one_stored_in_the_solver_and_its_test() {
        let body = describe_value(STRESS_PLATE_HOLE_S235_ELASTIC.name).expect("described");
        assert_eq!(body["reference"]["url"], plate::REFERENCE_URL);
        let test = body["reference"]["test"].as_str().expect("test");
        assert!(SOLVER_SOURCE.contains(&format!("fn {test}()")), "{test}");
        // The protocol it is scored on, after the fact, is the original's.
        assert!(SOLVER_SOURCE.contains("000657104b0a61e8c6f86935e73ac456315d3ec4fe94139f0bed258c62471880"));
        assert!(body["validation"].to_string().contains("000657104b0a61e8c6f86935e73ac456315d3ec4fe94139f0bed258c62471880"));
    }

    /// The list an agent reads first, and the result, say first that it is
    /// calibrated to the record and not a blind test, and name the scored
    /// original, whose own record says its comparison was not blind in the
    /// ring.
    #[test]
    fn the_summary_and_the_result_say_first_that_it_is_calibrated() {
        for text in [SUMMARY, RESULT] {
            assert!(text.starts_with("Calibrated to the record, not a blind test: "), "{text}");
            let first = text.split(". ").next().unwrap();
            assert!(first.contains("stress_plate_hole_s235"), "{first}");
        }
        assert!(RESULT.contains("stress_plate_hole_s235 is the scored original, compared before this companion was built, and by its rule it missed; its own record says that comparison was not blind in the ring."));
        assert!(!RESULT.contains("compared blind") && !SUMMARY.contains("compared blind"));
        assert!(SUMMARY.contains("1.917 against 1.889 measured, the measured 1.4 percent below, against 20"));
        assert!(SUMMARY.contains("nothing fitted here"));
        assert!(SUMMARY.ends_with("past it this field is not that steel's."));
    }

    #[test]
    fn every_published_name_is_in_the_entry() {
        let field = match solved(plate::solve(&ElasticPlateQuestion { traction_mpa: 150.0, points_x_mm: vec![10.0], points_y_mm: vec![5.0] })) {
            Solved::Answer(answer) => answer,
            Solved::Refused(refused) => panic!("{}", refused.sentence),
        };
        assert_eq!(field.result.len(), STRESS_PLATE_HOLE_S235_ELASTIC.result.len());
        for ((name, value), output) in field.result.iter().zip(STRESS_PLATE_HOLE_S235_ELASTIC.result) {
            assert_eq!(name, output.name);
            assert_eq!(value.shape(), output.shape);
        }
        let past = field.result.iter().find(|(name, _)| name == "past_compared_traction").expect("the flag");
        assert!(matches!(past.1, Published::Number(value) if value == 1.0));
        assert_eq!(field.invariants.len(), STRESS_PLATE_HOLE_S235_ELASTIC.invariants.len());
        for ((name, _), output) in field.invariants.iter().zip(STRESS_PLATE_HOLE_S235_ELASTIC.invariants) {
            assert_eq!(name, output.name);
        }
        assert_eq!(field.field.len(), 7 * 4033);
        assert_eq!(field.iterations, 0);
    }
}
