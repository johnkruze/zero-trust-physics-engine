//! The entry for `stress_plate_hole_s235`: the stress and displacement field
//! of the S235 steel plate with a hole of Tröger, Hartmann, Anton and
//! Wessels. The numbers come from the `genesis_solvers` constants and the
//! sentence of each refusal from the solver's source. Stamped from the kit.

use genesis_solvers::stress_plate_hole_s235::{self as plate, PlateField, PlateQuestion, PlateReject};
use serde_json::{Value, json};

use crate::solvers::{
    Answer, FurtherExample, Given, Input, Kind, Output, Published, Refusal, Refused, Sections, Shape, Solved, Solver,
};

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

pub const STRESS_PLATE_HOLE_S235: Solver = Solver {
    name: "stress_plate_hole_s235",
    summary: SUMMARY,
    inputs: &[
        // Every limit is asked by the solver's own check, in its order: the
        // traction has a name at each end, and a point's limits span both
        // lists.
        Input {
            name: "traction_mpa",
            kind: Kind::NumberAbove { above: 0.0, maximum: plate::YIELD_STRENGTH_MPA, unit: "MPa" },
            refusal: Some("TractionAboveYield"),
            bound: "The axial traction on the gross section of the parallel length, 20.05 mm by 3.05 mm (the force over 61.15 mm²), above 0 and at most 235 MPa, the yield strength: 0 or below is TractionNotTensile, above 235 MPa TractionAboveYield. The steel yields from 63.49 MPa and the field is carried past it. Near the collapse load of the net section (141.2 MPa), on a scan by 0.01 MPa, 139.81 MPa is the last traction answered; from 139.82 to 139.98 MPa the two finest meshes differ in the plastic dissipation by more than 2e-3 of the strain energy (NotSettled), and from 139.99 MPa the strain at the hole exceeds 1 percent (StrainBeyondSmallStrain). Every traction above 0 is answered on the same equations, solved in units scaled to it; below about 3.3e-152 MPa the energies in joules, and below about 1e-304 MPa the displacements, are smaller than the smallest normal 64-bit number, 2.2e-308, and are served as the format rounds them, to a subnormal number or zero. Compared with measurement at 106.26 MPa, the laboratory's one load; at every other traction it computes the same equations.",
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
    kept_inside: "The bar's shape, the steel's constants (Young's modulus 218,540 MPa and Poisson's ratio 0.2835, fitted to the band of the laboratory's field 16 mm or more from the hole; the yield strength 235 MPa), the meshes and the tolerances stay inside the solver.",
    result: &[
        output("axial_force_n", "N", Shape::Number, "The force the bar carries: the traction times 20.05 mm times 3.05 mm."),
        output("net_section_stress_mpa", "MPa", Shape::Number, "The force over the net section through the hole, 12.05 mm by 3.05 mm."),
        output("hole_edge_stress_mpa", "MPa", Shape::Number, "The stress along the bar at the top of the hole's edge, where the net section meets it, carried from the integration points to that node. In an elastic field it is the peak stress. Once the edge has yielded it is the yield strength, to within about 1 percent: the carrying of stress to an edge in a partly yielded element."),
        output("elastic_hole_edge_stress_mpa", "MPa", Shape::Number, "The same stress in the elastic field of this traction: what it would be if the steel did not yield. At 106.26 MPa it is 397.6 MPa."),
        output("stress_concentration_gross", "1", Shape::Number, "The elastic stress at the hole's edge over the traction: 3.742, a property of the bar's shape (Kirsch's infinite plate gives 3)."),
        output("stress_concentration_net", "1", Shape::Number, "The same over the net section stress: 2.2489 (Howland's solution, as Peterson's cubic gives it, 2.2366; Heywood's formula 2.2171)."),
        output("first_yield_traction_mpa", "MPa", Shape::Number, "The traction at which the solve's yielding begins: the elastic field's largest von Mises stress over the integration points, where the material law is evaluated, reaches the yield strength, 63.49 MPa. Below it no point has yielded; the edge's own stress, carried to its node, reaches the yield strength 1.1 percent earlier, at 62.80 MPa."),
        output("peak_von_mises_mpa", "MPa", Shape::Number, "The largest von Mises stress of the field over its integration points: at most the yield strength."),
        output("plastic_zone_area_mm2", "mm^2", Shape::Number, "The area of the plate's face that has yielded, all four quarters, from the integration points that have. Its edge is resolved to their spacing: 5.32 mm² at 106.26 MPa, where the two finest meshes differ by about 9 percent."),
        output("plastic_zone_reach_mm", "mm", Shape::Number, "The largest distance from the hole's edge of an integration point that has yielded: 1.53 mm at 106.26 MPa."),
        output("largest_equivalent_plastic_strain", "1", Shape::Number, "The largest equivalent plastic strain over the integration points."),
        output("largest_strain", "1", Shape::Number, "The largest principal strain in size over the integration points, in the plane and through the thickness: the measure of the small-strain limit, 0.01."),
        output("parallel_section_elongation_mm", "mm", Shape::Number, "The change of length of the 90 mm parallel length along its axis."),
        output("strain_energy_j", "J", Shape::Number, "The elastic strain energy stored in the bar out to 75 mm each side of the hole."),
        output("plastic_dissipation_j", "J", Shape::Number, "The work turned to heat by yielding: each load step's plastic strain against the mean of the stresses at its two ends, summed."),
        output("boundary_work_j", "J", Shape::Number, "The work of the load and the reactions: each load step's displacement against the mean of the forces at its two ends, summed from rest. It is the strain energy plus the plastic dissipation, the ledger closed from these three numbers (energy_residual); while the field stays elastic it equals the strain energy (Clapeyron)."),
        output("hole_edge_traction_fraction", "1", Shape::Number, "The largest normal or shear stress the field leaves on the hole's free edge, over the largest hoop stress there: zero in the exact field, it measures the carrying of stress to the edge: 6e-4 while elastic, 8e-3 at 106.26 MPa, up to 2.6e-2 near the strain limit."),
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
        output("dissipation_change", "1", Shape::Number, ""),
    ],
    iterations: "Newton iterations over every mesh and every load step after first yield: 0 for a field that stays elastic. On a refusal it is null.",
    field: field_words,
    answer: "a settled field",
    refusals: REFUSALS,
    door_refusals: &[],
    sections,
    validation: Some(validation),
    example,
    further_examples: &[FurtherExample { key: "example_solve_elastic", parameters: elastic_example }],
    parts: &[],
    check,
    solve,
};

const SUMMARY: &str = "The full in-plane stress tensor and displacement field of the S235 steel plate with a 4 mm hole of Tröger, Hartmann, Anton and Wessels (parallel length 20.05 by 3.05 mm, d/W 0.399), pulled at a traction from above 0 to 235 MPa on its gross section: plane stress with von Mises yield at 235 MPa (the least EN 10025-2 guarantees), carried past first yield at 63.49 MPa, settled on finite-element meshes of rising refinement with its force and energy ledger held every iteration, stress concentration 2.249 on the net section; answered to 139.81 MPa, near the net section's collapse, the meshes not settling from 139.82 MPa and the strain past 1 percent refused by name from 139.99; two constants fitted, Young's modulus and Poisson's ratio, on the opened band of the measured field; compared once with its held-back ring at 106.26 MPa: the displacement lines met (rms 0.675 and 0.444 um against 2.0 and 1.5; 0.571 um pooled against Kirsch's 1.015), the strain line 1.5 mm from the hole's edge missed (the solver 26.0 percent above the measured, the measured 20.6 percent below the solver's, against 20), so by the rule fixed beforehand the comparison missed; reported and not gated, the same field with the steel not yielding meets all three, a reading of the field that this sheet likely yields well above 235 MPa, not a measurement.";

/// Each `PlateReject` variant with the sentence from its doc comment. A
/// refusal the request itself shows is not sealed.
const REFUSALS: &[Refusal] = &[
    Refusal { name: "InputNotFinite", meaning: "An input was NaN or infinite.", sealed: false },
    Refusal {
        name: "TractionNotTensile",
        meaning: "The traction is not a pull: pushed, the strip between the clamps is a column that can bow out of its plane, and the field in the plane holds only while it stays flat.",
        sealed: false,
    },
    Refusal {
        name: "TractionAboveYield",
        meaning: "The traction is above the yield strength: the whole section of the parallel length would yield, and a plate that does not harden carries no more.",
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
    Refusal { name: "YieldReturnNotSettled", meaning: "The return of a stress to the yield surface did not converge.", sealed: true },
    Refusal {
        name: "EquilibriumNotFound",
        meaning: "No equilibrium was found past a traction, the load step halved `MAX_STEP_HALVINGS` times: near the collapse load of the net section the yielded steel carries little more.",
        sealed: true,
    },
    Refusal {
        name: "StrainBeyondSmallStrain",
        meaning: "The largest principal strain passed `SMALL_STRAIN_LIMIT`, where the linear measure of strain no longer holds.",
        sealed: true,
    },
    Refusal { name: "EquilibriumBroken", meaning: "The largest nodal force imbalance of the settled field is above `IMBALANCE_TOLERANCE`.", sealed: true },
    Refusal {
        name: "EnergyLedgerBroken",
        meaning: "The work of the boundary forces departs from the strain energy plus the dissipation by more than `ENERGY_TOLERANCE` of it.",
        sealed: true,
    },
    Refusal {
        name: "SectionForceBroken",
        meaning: "The force the settled field carries across a vertical cut departs from the applied force by more than `SECTION_FORCE_TOLERANCE`.",
        sealed: true,
    },
    Refusal { name: "NotSettled", meaning: "Two successive meshes did not agree within the settling fractions by the finest, `MAX_LEVEL`.", sealed: true },
];

fn field_words() -> String {
    "SHA-256 of the count of the numbers that follow as 8 little-endian bytes, which is 7 times the nodes of the accepted mesh (4,033 nodes of 1,280 elements in the quarter x, y >= 0), then for each node in turn its x and y in mm from the hole's centre, its displacement u_x and u_y in mm, and its stresses sigma_xx, sigma_yy and tau_xy in MPa (carried from the integration points and averaged over the elements that share the node), all as little-endian f64 bytes.".to_string()
}

fn sections() -> Sections {
    Sections {
        locked: json!({
            "equations": "Plane stress, small strain: div sigma = 0 in the bar, sigma . n = 0 on its sides and the hole's edge, a uniform traction on its end. Hooke's law sigma = C (epsilon - epsilon_p) with von Mises yield (sigma_xx^2 - sigma_xx sigma_yy + sigma_yy^2 + 3 tau_xy^2 <= Y^2) and no hardening, the flow associated; each step by the closest-point return of Simo and Taylor with its consistent tangent.",
            "hole_radius_mm": plate::HOLE_RADIUS_MM,
            "width_mm": plate::WIDTH_MM,
            "thickness_mm": plate::THICKNESS_MM,
            "parallel_length_mm": 2.0 * plate::PARALLEL_HALF_LENGTH_MM,
            "fillet_radius_mm": plate::FILLET_RADIUS_MM,
            "grip_width_mm": plate::WIDTH_MM + 2.0 * plate::FILLET_RISE_MM,
            "loaded_end_mm": plate::LOADED_END_MM,
            "youngs_modulus_mpa": plate::YOUNGS_MODULUS_MPA,
            "poisson_ratio": plate::POISSON_RATIO,
            "yield_strength_mpa": plate::YIELD_STRENGTH_MPA,
            "yield_strength_source": "EN 10025-2, the least upper yield strength of S235 of nominal thickness up to 16 mm. The record gives no yield strength of this sheet; its measured field near the hole is what a sheet that has not yielded shows (the validation's result).",
            "domain": "The quarter x, y >= 0 of the bar with its two mirror planes, out to 75 mm from the hole's centre: the parallel length, the fillet of 25 mm radius and 15 mm of the 30 mm wide grip end, where a uniform traction carries the bar's force. Moving that end to 105 mm moves the stress of the parallel length by at most 0.05 MPa.",
            "elements": "Eight-node quadrilaterals, 3 by 3 Gauss points, mapped about the hole and graded toward it; meshes of 80, 320 and 1,280 elements per quarter, each halving every element of the one before. On the finest the first element at the hole is 0.13 to 0.23 mm deep and 0.20 mm along the edge.",
            "largest_step_mpa": plate::LARGEST_STEP_MPA,
            "fitted": "Two constants, on the band of the laboratory's field 16 mm or more from the hole along the bar (each of its two bands with its own rigid motion), by the solver itself: Young's modulus 218,540 MPa and Poisson's ratio 0.2835. The bands alone give 206,500 and 230,000 MPa. The yield strength is the grade's, not fitted.",
        }),
        invariants: json!([
            {
                "name": "equilibrium_residual",
                "rule": "The nodal forces of the stresses balance the applied traction at every free node: the largest imbalance over the largest boundary nodal force (load and reactions). It confirms the equations were solved; how closely they describe the steel is in validation.",
                "tolerance": plate::IMBALANCE_TOLERANCE,
                "cap": plate::IMBALANCE_CAP,
                "unit": "1",
                "held": "every Newton iteration, and at acceptance",
                "refusal": "EquilibriumBroken, or EquilibriumNotFound when a step cannot reach it",
            },
            {
                "name": "energy_residual",
                "rule": "The work of the boundary forces, each step's displacement increment against the mean of the forces at its two ends, equals the strain energy stored plus the plastic dissipation, each step's plastic strain increment against the mean of the stresses at its two ends: boundary_work_j = strain_energy_j + plastic_dissipation_j. With Hooke's law at both ends of every step the two sides are one sum, so a move of the field either side leaves unbooked breaks it. While the field stays elastic, one step from rest, it is Clapeyron's theorem: the strain energy is the work of the forces rising from zero, half the final forces times the displacement. The largest residual over every settled step of every mesh, over the work: about 1.7e-13.",
                "tolerance": plate::ENERGY_TOLERANCE,
                "unit": "1",
                "held": "every settled load step of every mesh",
                "refusal": "EnergyLedgerBroken",
            },
            {
                "name": "section_force_residual",
                "rule": "The force the stresses carry across a vertical cut equals the applied force: five cuts, the net section through the hole, the side of the square about it at 10.025 mm, the middle of the parallel length, the start and the end of the fillet at 45 and 60 mm. The largest departure over the applied force; at most 1.09e-3, at the end of the fillet.",
                "tolerance": plate::SECTION_FORCE_TOLERANCE,
                "unit": "1",
                "held": "at acceptance",
                "refusal": "SectionForceBroken",
            },
            {
                "name": "displacement_change",
                "rule": "The largest change of displacement at the nodes the two finest meshes share in the parallel length, over its largest displacement: 1.4e-4 while elastic, 6.6e-4 at 106.26 MPa, 2.98e-3 at 139.81 MPa, the highest traction answered.",
                "tolerance": plate::DISPLACEMENT_SETTLE,
                "unit": "1",
                "held": "between the two finest meshes",
                "refusal": "NotSettled",
            },
            {
                "name": "edge_stress_change",
                "rule": "The change of the elastic stress at the top of the hole's edge between the two finest meshes, over itself: 3.8e-4.",
                "tolerance": plate::STRESS_SETTLE,
                "unit": "1",
                "held": "between the two finest meshes",
                "refusal": "NotSettled",
            },
            {
                "name": "dissipation_change",
                "rule": "The change of the plastic dissipation between the two finest meshes, over the strain energy: 0 while elastic, 4.7e-5 at 106.26 MPa, 1.90e-3 at 139.81 MPa, the highest traction answered; from 139.82 to 139.98 MPa it exceeds 2e-3, and the question is refused as NotSettled.",
                "tolerance": plate::DISSIPATION_SETTLE,
                "unit": "1",
                "held": "between the two finest meshes",
                "refusal": "NotSettled",
            },
        ]),
        convergence: json!({
            "displacement_settle": plate::DISPLACEMENT_SETTLE,
            "stress_settle": plate::STRESS_SETTLE,
            "dissipation_settle": plate::DISSIPATION_SETTLE,
            "max_level": plate::MAX_LEVEL,
            "imbalance_tolerance": plate::IMBALANCE_TOLERANCE,
            "max_newton_iterations": plate::MAX_NEWTON_ITERATIONS,
            "max_step_halvings": plate::MAX_STEP_HALVINGS,
            "rule": "Each mesh is solved along the whole load path: the elastic field of the full traction by one direct solve; past first yield the load rises in steps of at most 4 MPa, each settled by Newton's method with the consistent tangent factored directly, a step that does not settle halved. The finest of two successive meshes is accepted when they agree within the three settling fractions; a limit is named when the finest mesh reaches it, or two successive meshes do. Over every traction answered the accepted mesh is the finest, 1,280 elements; from 139.82 to 139.98 MPa, by 0.01 MPa, the two finest do not agree in the dissipation, NotSettled. The field is solved in MPa times the power of two that carries the traction into [1, 2), each value carried back once, so that no product leaves the range of 64-bit numbers.",
            "refusal": "NotSettled, EquilibriumNotFound or StrainBeyondSmallStrain",
            "run_time": "On one core of an Apple M4 Pro in a release build, measured under load: 1.66 s at 139.81 MPa, the highest traction answered, with 1,000 points (1.58 s at 139.5 MPa), about 1.0 s at 106.26 MPa and 0.26 s while elastic with 1,000 points, 0.02 s while elastic with none.",
        }),
        reference: json!({
            "name": plate::REFERENCE_NAME,
            "url": plate::REFERENCE_URL,
            "retrieved": plate::REFERENCE_RETRIEVED,
            "test": "the_comparison_s_strain_near_the_hole_missed_by_its_line",
            "covers": "Measure P3 of the comparison, from the 40 measured points of its two windows: the strain 1.5 mm from the hole's edge over the far strain, 1.889 measured and 2.380 by the solver, 20.6 percent apart. The whole comparison is in the validation.",
            "kind": "Measurements of the specimen itself. The solver's tests also hold it to Kirsch's field imposed on a square around the hole (stress concentration 2.9996 against 3), to a strip ten holes wide (3.035 against Howland's 3.025 and Heywood's 3.032), and to the exact plane-stress yielded ring around a hole pulled equally every way (0.11 percent of the yield strength).",
        }),
    }
}

fn validation() -> [String; 5] {
    let object = "A flat tensile bar of S235 low-carbon steel, 3.05 mm thick, its parallel length 20.05 mm wide and 90 mm long with a hole of radius 4 mm at its centre, fillets of radius 25 mm to grip ends 30 mm wide, pulled in a ZwickRoell Z100 machine at Clausthal University of Technology to a traction of 106.26 N/mm² on its gross section (6,498 N). The solver settles the plane-stress field of the bar with the steel yielding at 235 MPa. Two constants are fitted, Young's modulus and Poisson's ratio, on the band 16 mm or more from the hole.".to_string();
    let data = "Digital image correlation measurement of linear elastic steel specimen, Zenodo 10.5281/zenodo.11257192, version 1.0 (Clausthal University of Technology, TU Braunschweig): the surface displacement u_x and u_y at 5,475 points of the parallel length, by stereo digital image correlation (GOM ARAMIS 12M, 25 px facets), one load state, no uncertainty given for each point. The traction is the one the record's paper gives from the machine's force (arXiv 2405.18311, section 6.1). The hole's centre was found from the points' positions alone. In the fit: the 3,372 points 16 mm or more from the hole's centre along the bar (3,269 used, less the first and last 1.5 mm of the window and 7 bad points). Outside the fit: the 1,189 points 4.5 to 12 mm from the hole's centre (1,186 compared). Of that ring, local strains of 1.3e-3 to 3.0e-3, from fits within 1.5 mm, 0.5 to 1 mm from the hole's edge, were read before the error measures were set, and the strain measure P3, its window and its 20 percent, was proposed with those strains in view; no other value of the ring was read.".to_string();
    let pass_line = "After one rigid motion fitted to the points in the fit: P1, RMS of the u_x error at most 2.0 um and of u_y at most 1.5 um over the ring outside the fit; P2, the pooled RMS below that of a plate without a hole (a uniform strain fitted to the band in the fit) and below that of Kirsch's infinite plate with the same constants; P3, the strain read by a plane fitted within 1.5 mm of (0, 5.5) and (0, -5.5) mm, over the far strain, within 20 percent of the solver's same reading. Record SHA-256 000657104b0a61e8c6f86935e73ac456315d3ec4fe94139f0bed258c62471880.".to_string();
    let limits = "The comparison is at the one load state the record holds, 106.26 MPa. Below 63.49 MPa the field is elastic and scales with the traction; from 63.49 to 139.81 MPa the yielded zone grows, on the same equations and constants. The yield strength is the grade's guaranteed least value, 235 MPa; this sheet's own is not in the record, and its measured field near the hole points to one well above (stress_plate_hole_s235_elastic serves that field). The two bands of the field in the fit strain 11 percent apart at the same force, so with one modulus the u_x error is a shift, larger left of the hole. Plane stress in a sheet 3.05 mm thick about a hole of radius 4 mm (t/R = 0.76): the surface the camera sees may differ by a few percent near the hole. The sheet is taken as uniform and the same in every direction, the hole as centred. Points are served in the parallel length, -45 to 45 mm. The ledger confirms the equations were solved, and a proof confirms an answer is unchanged; how closely the model matches the steel is the comparison above.".to_string();
    [object, data, pass_line, RESULT.to_string(), limits]
}

/// The comparison with the ring outside the fit.
const RESULT: &str = "On the 1,186 points of the ring 4.5 to 12 mm from the hole, outside the fit. P1, displacement: RMS 0.675 um in u_x (target 2.0) and 0.444 um in u_y (target 1.5). P2: pooled RMS 0.571 um, against 1.818 um for the plate without a hole and 1.015 um for Kirsch's infinite plate. P3, the strain 1.5 mm from the hole's edge over the far strain: 1.889 measured, 2.380 by the solver, the measured 20.6 percent below the solver's (the solver 26.0 percent above the measured), target 20, 0.6 points past it; the 20.6 percent itself is about 2.3 standard deviations of the read (9 percent). Beside these: the same field with the steel not yielding gives P1 0.638 and 0.486 um, P2 0.567 um, P3 within 1.4 percent; yielding at 235 MPa puts a yielded zone 1.53 mm deep at the hole, which is absent from the measured strain. In displacement the two fields agree within 0.02 um in every ring from 4.5 to 12 mm. By half of the ring the u_x error is mostly a shift, +0.69 um left of the hole and +0.27 um right.";

fn example() -> Value {
    json!({ "traction_mpa": plate::MEASURED_TRACTION_MPA, "points_x_mm": [0.0, 0.0, 4.5, -20.0], "points_y_mm": [4.0, 10.025, 0.0, 5.0] })
}

fn elastic_example() -> Value {
    json!({ "traction_mpa": 50.0, "points_x_mm": [], "points_y_mm": [] })
}

/// The `PlateReject` variant name, without its fields.
pub fn reject_name(reject: &PlateReject) -> &'static str {
    reject.name()
}

/// The solver's own question, from the values of `parameters`.
fn question(given: &Given) -> Result<PlateQuestion, String> {
    Ok(PlateQuestion {
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
pub fn solved(returned: Result<PlateField, PlateReject>) -> Solved {
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
    let many = |name: &str, values: Vec<f64>| (name.to_string(), Published::Numbers(values));
    let at = |read: fn(&plate::PointField) -> f64| -> Vec<f64> { field.points.iter().map(read).collect() };
    Solved::Answer(Answer {
        iterations: u64::from(field.newton_iterations),
        result: vec![
            one("axial_force_n", field.axial_force_n),
            one("net_section_stress_mpa", field.net_section_stress_mpa),
            one("hole_edge_stress_mpa", field.hole_edge_stress_mpa),
            one("elastic_hole_edge_stress_mpa", field.elastic_hole_edge_stress_mpa),
            one("stress_concentration_gross", field.stress_concentration_gross),
            one("stress_concentration_net", field.stress_concentration_net),
            one("first_yield_traction_mpa", field.first_yield_traction_mpa),
            one("peak_von_mises_mpa", field.peak_von_mises_mpa),
            one("plastic_zone_area_mm2", field.plastic_zone_area_mm2),
            one("plastic_zone_reach_mm", field.plastic_zone_reach_mm),
            one("largest_equivalent_plastic_strain", field.largest_equivalent_plastic_strain),
            one("largest_strain", field.largest_strain),
            one("parallel_section_elongation_mm", field.parallel_section_elongation_mm),
            one("strain_energy_j", field.strain_energy_j),
            one("plastic_dissipation_j", field.plastic_dissipation_j),
            one("boundary_work_j", field.boundary_work_j),
            one("hole_edge_traction_fraction", field.hole_traction_residual),
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
            one("dissipation_change", field.dissipation_change),
        ],
        field: field.nodes,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::solvers::describe_value;

    const SOLVER_SOURCE: &str = include_str!("../../genesis_solvers/src/stress_plate_hole_s235.rs");

    /// `(variant, doc sentence)` for every variant of `PlateReject`, read
    /// from the solver's source.
    fn documented_rejects() -> Vec<(String, String)> {
        let start = SOLVER_SOURCE.find("pub enum PlateReject {").expect("PlateReject is in the solver");
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
        let body = describe_value(STRESS_PLATE_HOLE_S235.name).expect("described");
        let refusals = body["refusals"].as_array().expect("refusals");
        assert_eq!(refusals.len(), documented.len());
        for (refusal, listed) in refusals.iter().zip(REFUSALS) {
            let reported = if listed.sealed { "200, outcome refused, sealed" } else { "422, no seal" };
            assert_eq!(refusal["reported_as"], reported, "{}", refusal["name"]);
        }
    }

    #[test]
    fn the_reference_is_the_one_stored_in_the_solver_and_its_test() {
        let body = describe_value(STRESS_PLATE_HOLE_S235.name).expect("described");
        assert_eq!(body["reference"]["url"], plate::REFERENCE_URL);
        assert!(SOLVER_SOURCE.contains("10.5281/zenodo.11257192"));
        let test = body["reference"]["test"].as_str().expect("test");
        assert!(SOLVER_SOURCE.contains(&format!("fn {test}()")), "{test}");
        // The protocol's fingerprint is the one the solver's header carries.
        assert!(SOLVER_SOURCE.contains("000657104b0a61e8c6f86935e73ac456315d3ec4fe94139f0bed258c62471880"));
        assert!(body["validation"].to_string().contains("000657104b0a61e8c6f86935e73ac456315d3ec4fe94139f0bed258c62471880"));
    }

    /// The list an agent reads first carries the standing.
    #[test]
    fn the_summary_carries_the_standing() {
        let summary = crate::spec_sheet::summary_of(STRESS_PLATE_HOLE_S235.name, SUMMARY);
        assert!(summary.contains("2.380 against 1.889 measured, the solver 26.0 percent above the measured, the measured 20.6 percent below the solver's (target 20)"));
        assert!(!summary.contains("20.6 percent above"));
        assert!(RESULT.contains("about 2.3 standard deviations of the read (9 percent)") && !RESULT.contains("about two standard deviations"));
        assert!(summary.contains("two constants, Young's modulus 218,540 MPa and Poisson's ratio 0.2835"));
        assert!(summary.contains("within 0.675 um RMS in u_x (target 2.0) and 0.444 um in u_y (target 1.5)"));
        assert!(summary.contains("the same field with the steel not yielding comes within 1.4 percent (stress_plate_hole_s235_elastic)"));
    }

    #[test]
    fn every_published_name_is_in_the_entry() {
        let field = match solved(plate::solve(&PlateQuestion { traction_mpa: 50.0, points_x_mm: vec![10.0], points_y_mm: vec![5.0] })) {
            Solved::Answer(answer) => answer,
            Solved::Refused(refused) => panic!("{}", refused.sentence),
        };
        assert_eq!(field.result.len(), STRESS_PLATE_HOLE_S235.result.len());
        for ((name, value), output) in field.result.iter().zip(STRESS_PLATE_HOLE_S235.result) {
            assert_eq!(name, output.name);
            assert_eq!(value.shape(), output.shape);
        }
        assert_eq!(field.invariants.len(), STRESS_PLATE_HOLE_S235.invariants.len());
        for ((name, _), output) in field.invariants.iter().zip(STRESS_PLATE_HOLE_S235.invariants) {
            assert_eq!(name, output.name);
        }
        assert_eq!(field.field.len(), 7 * 4033);
        assert_eq!(field.iterations, 0);
    }
}
