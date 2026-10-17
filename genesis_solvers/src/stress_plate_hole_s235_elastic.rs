//! The elastic stress and displacement field of the S235 steel plate with a
//! hole of Tröger, Hartmann, Anton and Wessels, pulled: the companion of
//! `stress_plate_hole_s235`, calibrated to the record.
//!
//! **Calibrated to the record, not a blind test.** `stress_plate_hole_s235`
//! was compared blind, once, with the held-back ring of the measured field
//! (`evidence/research/stress_plate_s235/validation_protocol.md`, SHA-256
//! 000657104b0a61e8c6f86935e73ac456315d3ec4fe94139f0bed258c62471880): its
//! displacement lines were met and its strain line 1.5 mm from the hole's
//! edge missed. This solver's structure, the same field with the steel not
//! yielding, was chosen on 2026-10-15, after that ring had been scored and
//! read: the strain line separates the elastic field from the return to the
//! yield surface at 235 MPa (the displacement lines do not), and the record
//! does not give this sheet's yield strength. So its comparison below is
//! after the fact. The record:
//! `evidence/research/stress_plate_s235/companion_elastic/CALIBRATION.md`.
//!
//! The plate is the tensile bar of `stress_plate_hole_s235` (Zenodo,
//! <https://doi.org/10.5281/zenodo.11257192>, version 1.0, CC BY 4.0): S235
//! low-carbon steel 3.05 mm thick, its parallel length 20.05 mm wide and 90
//! mm long with a hole of radius 4 mm at its centre, fillets of radius 25 mm
//! to grip ends 30 mm wide. The question is the traction on the gross
//! section of the parallel length and the points at which the field is
//! wanted; the answer is the settled field: the displacement, the full
//! in-plane stress tensor and the strain everywhere, read at those points,
//! the stress at the hole's edge, the largest von Mises stress, and the
//! ledger.
//!
//! **The physics.** Plane stress, small strain, Hooke's law with no yield,
//! with `stress_plate_hole_s235`'s own Young's modulus and Poisson's ratio,
//! fitted on the opened band before its scoring: nothing is fitted here. Its
//! bar, its meshes of 80, 320 and 1,280 elements, its settling fractions, its
//! ledger and its scaled units are that solver's (`specimen_layout`, `build`,
//! `plane_stress_fe`); with the yield taken out, the load path is one direct
//! solve of the full traction on each mesh. The field is linear in the
//! traction: the stress at the top of the hole's edge is 3.742 times the
//! gross traction (2.249 times the net), 397.6 MPa at the laboratory's 106.26
//! MPa, and every other number scales with it. At every traction this
//! solver's field is, bit for bit, `stress_plate_hole_s235`'s field with the
//! steel not yielding, the one its record reported and did not gate (a
//! test).
//!
//! **Held every solve.** The largest nodal force imbalance of a free node,
//! over the largest boundary nodal force, after the direct solve
//! (`IMBALANCE_TOLERANCE`). The energy ledger on every mesh: the work of the
//! boundary forces, rising from zero to the full traction, half the final
//! forces times the displacement, equals the strain energy (Clapeyron's
//! theorem) to `ENERGY_TOLERANCE`; it closes from the two numbers served. At
//! acceptance the force the stresses carry across five vertical cuts equals
//! the applied force to `SECTION_FORCE_TOLERANCE` (found 1.09e-3, at the end
//! of the fillet). Each ledger set past what the arithmetic holds is refused
//! by name (a test).
//!
//! **Limits.** A traction that is not a pull: the strip between the clamps
//! would be a column. The largest principal strain passing 1 percent, where
//! the linear measure of strain departs from the Green-Lagrange measure by
//! half a percent of itself: the field is linear, so that is one traction,
//! 590.04 MPa the last answered on a scan by 0.01 MPa (a test). A point
//! outside the parallel length or inside the hole. Nothing else is refused:
//! the mathematics of the elastic field holds at every traction above zero.
//!
//! **Past the compared traction.** The record compares one load, 106.26 MPa.
//! Above it every stress of the field is above any the sheet was compared
//! at, and the answer carries `past_compared_traction`. The sheet's own yield
//! strength is not in the record (the authors state that the machine's
//! stress-strain curve stayed straight and that no permanent deformation was
//! found after the test, which says the sheet stayed elastic in the large at
//! 106.26 MPa, not where it yields). Past that yield strength, wherever it
//! is, this elastic field is not that steel's: the steel yields first at the
//! hole's edge, where the stress is highest. The solver cannot say where
//! that is, so it does not stop there: it serves the largest von Mises
//! stress of the field (393.3 MPa over the integration points at 106.26 MPa)
//! and the stress at every asked point, for a caller to hold against a yield
//! strength it knows. `stress_plate_hole_s235` is the same plate yielding at
//! 235 MPa, the least EN 10025-2 guarantees for S235.
//!
//! **The comparison of 2026-10-15, after the fact.** On the original
//! protocol's three lines, at 106.26 MPa, by the original's own scoring
//! functions (`analysis/scoring.py`, `analysis/03_score.py`) on the field
//! this solver serves through the engine
//! (`companion_elastic/score_companion.py`), over the same 1,186 held-back
//! points: P1, rms 0.638 um in u_x and 0.486 um in u_y against 2.0 and 1.5:
//! met; P2, pooled rms 0.567 um, against 1.818 um for a plate without a hole
//! and 1.015 um for Kirsch's infinite plate: met; P3, the ratio R of the
//! strain read by a plane fitted within 1.5 mm of (0, +-5.5) mm to the far
//! strain read from the two opened bands, 1.889 measured and 1.917 by this
//! solver, the measured 1.4 percent below the solver's, against a line of
//! 20: met. These are the numbers `stress_plate_hole_s235`'s record already
//! reported, not gated, for its field with the steel not yielding, and they
//! were read before this structure was chosen: they are not a blind test.
//! The solver compared blind is `stress_plate_hole_s235`, and by its rule it
//! missed.
//!
//! What a result is not. Plane stress in a sheet 3.05 mm thick about a hole
//! of radius 4 mm (t/R = 0.76): near the hole the surface the camera sees
//! may differ from the plane-stress field by a few percent. The sheet is
//! taken as uniform and the same in every direction, and the hole as
//! centred. The two opened bands strain 11 percent apart at the same force,
//! and one modulus sits between them. The ledger shows the equations were
//! solved, not that they describe the steel: the comparison does, and it
//! was made with the record open.

use genesis_kernel::math;
use crate::normal::NormalOrZero;
use std::collections::HashMap;

use crate::plane_stress_fe::{self as fe, LedgerFault, Material, PathFailure, PathNumerics, PointState, Problem, Settled};
use crate::stress_plate_hole_s235::{self as plate, Built, PointField, Side};

/// Radius of the hole, mm: `stress_plate_hole_s235`'s.
pub const HOLE_RADIUS_MM: f64 = plate::HOLE_RADIUS_MM;
/// Width of the parallel section, mm.
pub const WIDTH_MM: f64 = plate::WIDTH_MM;
/// Thickness of the sheet, mm.
pub const THICKNESS_MM: f64 = plate::THICKNESS_MM;
/// Half the length of the parallel section, mm.
pub const PARALLEL_HALF_LENGTH_MM: f64 = plate::PARALLEL_HALF_LENGTH_MM;
/// Where the load is put on the model, mm from the hole's centre.
pub const LOADED_END_MM: f64 = plate::LOADED_END_MM;

/// Young's modulus, MPa: `stress_plate_hole_s235`'s, fitted to the opened
/// band of the laboratory's field before its scoring. Not fitted here.
pub const YOUNGS_MODULUS_MPA: f64 = plate::YOUNGS_MODULUS_MPA;
/// Poisson's ratio, fitted with it.
pub const POISSON_RATIO: f64 = plate::POISSON_RATIO;

/// The one traction the record compares, MPa (Anton et al., arXiv
/// 2405.18311, section 6.1): above it the answer carries
/// `past_compared_traction`.
pub const COMPARED_TRACTION_MPA: f64 = plate::MEASURED_TRACTION_MPA;

/// The public measurement this solver is set against: the same record as
/// `stress_plate_hole_s235`'s.
pub const REFERENCE_NAME: &str = plate::REFERENCE_NAME;
pub const REFERENCE_URL: &str = plate::REFERENCE_URL;
pub const REFERENCE_RETRIEVED: &str = plate::REFERENCE_RETRIEVED;

/// The numerical choices, `stress_plate_hole_s235`'s.
pub const IMBALANCE_TOLERANCE: f64 = plate::IMBALANCE_TOLERANCE;
pub const IMBALANCE_CAP: f64 = plate::IMBALANCE_CAP;
pub const ENERGY_TOLERANCE: f64 = plate::ENERGY_TOLERANCE;
pub const MAX_LEVEL: u32 = plate::MAX_LEVEL;
/// The largest principal strain of a settled field, in size: past it the
/// linear measure of strain departs from the Green-Lagrange measure by more
/// than half a percent of itself.
pub const SMALL_STRAIN_LIMIT: f64 = plate::SMALL_STRAIN_LIMIT;
/// Two successive levels are settled when the largest change of a
/// displacement at their common nodes is at most this fraction of the
/// largest displacement...
pub const DISPLACEMENT_SETTLE: f64 = plate::DISPLACEMENT_SETTLE;
/// ... and the stress at the top of the hole's edge changes by at most this
/// fraction of itself.
pub const STRESS_SETTLE: f64 = plate::STRESS_SETTLE;
/// The force carried across every vertical cut of the settled field, from
/// its stresses, is the applied force to this fraction.
pub const SECTION_FORCE_TOLERANCE: f64 = plate::SECTION_FORCE_TOLERANCE;
/// The most points one question may ask the field at.
pub const MAX_POINTS: usize = plate::MAX_POINTS;

/// One question: the traction on the gross section of the parallel length,
/// and the points at which the field is wanted, in mm from the hole's
/// centre, `x` along the bar and `y` across it.
#[derive(Clone, Debug, PartialEq)]
pub struct ElasticPlateQuestion {
    pub traction_mpa: f64,
    pub points_x_mm: Vec<f64>,
    pub points_y_mm: Vec<f64>,
}

/// Why no field was returned.
#[derive(Clone, Debug, PartialEq)]
pub enum ElasticPlateReject {
    /// An input was NaN or infinite.
    InputNotFinite,
    /// The traction is not a pull: pushed, the strip between the clamps is a column that can bow out of its plane, and the field in the plane holds only while it stays flat.
    TractionNotTensile { traction_mpa: f64 },
    /// The lists of the points' `x` and `y` are not the same length.
    PointCountsDiffer { x_count: usize, y_count: usize },
    /// More points than `MAX_POINTS`.
    TooManyPoints { count: usize },
    /// A point is outside the parallel length: past its end, where the field depends on how the clamps carry the load, or beyond its sides.
    PointOutsidePlate { x_mm: f64, y_mm: f64 },
    /// A point is inside the hole, where there is no steel.
    PointInsideHole { x_mm: f64, y_mm: f64 },
    /// A value of the field was NaN or infinite.
    NonFiniteValue,
    /// An element of the mesh is turned inside out.
    MeshInverted,
    /// The stiffness of the elastic field has a pivot that is not positive.
    SingularStiffness,
    /// The largest principal strain passed `SMALL_STRAIN_LIMIT`, where the linear measure of strain no longer holds.
    StrainBeyondSmallStrain { traction_mpa: f64, strain: f64, level: u32 },
    /// The largest nodal force imbalance of the settled field is above `IMBALANCE_TOLERANCE`.
    EquilibriumBroken { imbalance: f64 },
    /// The work of the boundary forces departs from the strain energy by more than `ENERGY_TOLERANCE` of it.
    EnergyLedgerBroken { residual: f64 },
    /// The force the settled field carries across a vertical cut departs from the applied force by more than `SECTION_FORCE_TOLERANCE`.
    SectionForceBroken { x_mm: f64, residual: f64 },
    /// Two successive meshes did not agree within the settling fractions by the finest, `MAX_LEVEL`.
    NotSettled { change: f64 },
}

impl ElasticPlateReject {
    pub fn name(&self) -> &'static str {
        match self {
            Self::InputNotFinite => "InputNotFinite",
            Self::TractionNotTensile { .. } => "TractionNotTensile",
            Self::PointCountsDiffer { .. } => "PointCountsDiffer",
            Self::TooManyPoints { .. } => "TooManyPoints",
            Self::PointOutsidePlate { .. } => "PointOutsidePlate",
            Self::PointInsideHole { .. } => "PointInsideHole",
            Self::NonFiniteValue => "NonFiniteValue",
            Self::MeshInverted => "MeshInverted",
            Self::SingularStiffness => "SingularStiffness",
            Self::StrainBeyondSmallStrain { .. } => "StrainBeyondSmallStrain",
            Self::EquilibriumBroken { .. } => "EquilibriumBroken",
            Self::EnergyLedgerBroken { .. } => "EnergyLedgerBroken",
            Self::SectionForceBroken { .. } => "SectionForceBroken",
            Self::NotSettled { .. } => "NotSettled",
        }
    }
}

impl std::fmt::Display for ElasticPlateReject {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InputNotFinite => write!(f, "an input was not finite"),
            Self::TractionNotTensile { traction_mpa } => write!(f, "traction {} MPa is not a pull: pushed, the strip between the clamps is a column that can bow out of its plane, and the field in the plane holds only while it stays flat", said(*traction_mpa)),
            Self::PointCountsDiffer { x_count, y_count } => write!(f, "{x_count} values of x and {y_count} of y: a point is one of each"),
            Self::TooManyPoints { count } => write!(f, "{count} points, more than {MAX_POINTS}"),
            Self::PointOutsidePlate { x_mm, y_mm } => write!(f, "the point ({}, {}) mm is outside the parallel length, which runs {PARALLEL_HALF_LENGTH_MM} mm each way from the hole's centre and {} mm each way across", said(*x_mm), said(*y_mm), WIDTH_MM / 2.0),
            Self::PointInsideHole { x_mm, y_mm } => write!(f, "the point ({}, {}) mm is inside the hole, of radius {HOLE_RADIUS_MM} mm", said(*x_mm), said(*y_mm)),
            Self::NonFiniteValue => write!(f, "a value of the field was not finite"),
            Self::MeshInverted => write!(f, "an element of the mesh is turned inside out"),
            Self::SingularStiffness => write!(f, "the stiffness of the elastic field has a pivot that is not positive"),
            Self::StrainBeyondSmallStrain { traction_mpa, strain, level } => write!(f, "at traction {} MPa on mesh level {level} the largest principal strain is {}, past {SMALL_STRAIN_LIMIT}, where the linear measure of strain no longer holds", said(*traction_mpa), said(*strain)),
            Self::EquilibriumBroken { imbalance } => write!(f, "the largest nodal force imbalance of the settled field is {}, above {IMBALANCE_TOLERANCE}", said(*imbalance)),
            Self::EnergyLedgerBroken { residual } => write!(f, "the work of the boundary forces departs from the strain energy by {} of it, above {ENERGY_TOLERANCE}", said(*residual)),
            Self::SectionForceBroken { x_mm, residual } => write!(f, "the force carried across the cut at x = {} mm departs from the applied force by {} of it, above {SECTION_FORCE_TOLERANCE}", said(*x_mm), said(*residual)),
            Self::NotSettled { change } => write!(f, "the two finest meshes, level {} and {MAX_LEVEL}, differ by {} times the settling fraction of the measure that differs most", MAX_LEVEL - 1, said(*change)),
        }
    }
}

impl std::error::Error for ElasticPlateReject {}

/// A number in a sentence: the shortest decimal that reads back to the same
/// 64-bit number, written with an exponent when it would otherwise run past
/// 24 characters.
fn said(value: f64) -> String {
    let plain = format!("{value}");
    if plain.len() > 24 { format!("{value:e}") } else { plain }
}

/// `value` times two to the power `exponent`, rounded once: every factor
/// but the last is exact. The same arithmetic as `stress_plate_hole_s235`'s.
fn times_two_to(value: f64, exponent: i32) -> f64 {
    fn power(e: i32) -> f64 {
        // 2^e for e from -1074 to 1023, exactly.
        if e >= -1022 { f64::from_bits(((e + 1023) as u64) << 52) } else { f64::from_bits(1_u64 << (e + 1074)) }
    }
    let (mut value, mut exponent) = (value, exponent);
    while exponent > 1023 {
        value *= power(1023);
        exponent -= 1023;
    }
    while exponent < -1074 {
        let step = (exponent + 1074).max(-1022);
        let next = value * power(step);
        if next.abs() < f64::MIN_POSITIVE {
            return 0.0 * value.signum();
        }
        value = next;
        exponent -= step;
    }
    value * power(exponent)
}

/// The power of two that carries `traction` into [1, 2): the field is solved
/// in MPa times two to this power, so that no product of the arithmetic
/// leaves the range of 64-bit numbers whatever the traction.
fn unit_exponent(traction: f64) -> i32 {
    let (mut scaled, mut exponent) = (traction, 0_i32);
    while scaled >= 2.0 {
        scaled *= 0.5;
        exponent -= 1;
    }
    while scaled < 1.0 {
        scaled *= 2.0;
        exponent += 1;
    }
    exponent
}

/// A point's field, solved in the units of `unit`, in MPa and mm.
fn point_in_mpa(p: PointField, unit: i32) -> PointField {
    let down = |value: f64| times_two_to(value, -unit);
    PointField {
        displacement_x_mm: down(p.displacement_x_mm),
        displacement_y_mm: down(p.displacement_y_mm),
        stress_xx_mpa: down(p.stress_xx_mpa),
        stress_yy_mpa: down(p.stress_yy_mpa),
        stress_xy_mpa: down(p.stress_xy_mpa),
        von_mises_mpa: down(p.von_mises_mpa),
        strain_xx: down(p.strain_xx),
        strain_yy: down(p.strain_yy),
        strain_xy: down(p.strain_xy),
    }
}

/// A settled elastic field and what is read from it.
#[derive(Clone, Debug, PartialEq)]
pub struct ElasticPlateField {
    pub traction_mpa: f64,
    pub axial_force_n: f64,
    pub net_section_stress_mpa: f64,
    /// The stress along the bar at the top of the hole's edge, where the
    /// net section meets it: the peak stress, carried to that node.
    pub hole_edge_stress_mpa: f64,
    pub stress_concentration_gross: f64,
    pub stress_concentration_net: f64,
    /// The largest von Mises stress of the field over its integration
    /// points, where the material law is evaluated.
    pub peak_von_mises_mpa: f64,
    pub largest_strain: f64,
    /// The change of length of the parallel section along its axis.
    pub parallel_section_elongation_mm: f64,
    pub strain_energy_j: f64,
    pub boundary_work_j: f64,
    /// The traction is above the one the record compares, 106.26 MPa.
    pub past_compared_traction: bool,
    pub points: Vec<PointField>,
    /// The ledger.
    pub equilibrium_residual: f64,
    pub energy_residual: f64,
    pub section_force_residual: f64,
    /// The largest traction on the hole's edge, normal or shear, over the
    /// largest stress along it.
    pub hole_traction_residual: f64,
    /// The mesh level accepted and its elements per quarter.
    pub level: u32,
    pub elements: usize,
    /// The settling measures between the accepted level and the one before.
    pub displacement_change: f64,
    pub stress_change: f64,
    /// Every node of the quarter `x, y >= 0` at the accepted level: `x`,
    /// `y`, `u_x`, `u_y` in mm and `sigma_xx`, `sigma_yy`, `tau_xy` in MPa,
    /// seven numbers each, node after node.
    pub nodes: Vec<f64>,
}

/// The numerical choices; the served solver uses [`SERVED`], the tests
/// break one at a time.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Numerics {
    pub path: PathNumerics,
    pub max_level: u32,
    pub displacement_settle: f64,
    pub stress_settle: f64,
    pub section_force_tolerance: f64,
    pub small_strain_limit: f64,
    pub loaded_end: f64,
    /// Solve in MPa times the power of two that carries the traction into
    /// [1, 2); the tests hold it to the unscaled solve.
    pub scaled_units: bool,
}

pub(crate) const SERVED: Numerics = Numerics {
    path: PathNumerics {
        // No yield: the path is one elastic step, and takes no load step.
        largest_step: f64::NAN,
        imbalance_tolerance: IMBALANCE_TOLERANCE,
        imbalance_cap: IMBALANCE_CAP,
        max_newton_iterations: plate::MAX_NEWTON_ITERATIONS,
        max_step_halvings: plate::MAX_STEP_HALVINGS,
        energy_tolerance: ENERGY_TOLERANCE,
        fault: LedgerFault::None,
    },
    max_level: MAX_LEVEL,
    displacement_settle: DISPLACEMENT_SETTLE,
    stress_settle: STRESS_SETTLE,
    section_force_tolerance: SECTION_FORCE_TOLERANCE,
    small_strain_limit: SMALL_STRAIN_LIMIT,
    loaded_end: LOADED_END_MM,
    scaled_units: true,
};

/// The steel: Hooke's law with `stress_plate_hole_s235`'s constants, and no
/// yield.
const STEEL: Material = Material { youngs_modulus: YOUNGS_MODULUS_MPA, poisson_ratio: POISSON_RATIO, yield_strength: None };

/// The limits the question itself shows, in the order the solver asks them.
pub fn admit(question: &ElasticPlateQuestion) -> Result<(), ElasticPlateReject> {
    let t = question.traction_mpa;
    if !t.is_finite() || question.points_x_mm.iter().chain(&question.points_y_mm).any(|v| !v.is_finite()) {
        return Err(ElasticPlateReject::InputNotFinite);
    }
    if !(t > 0.0) {
        return Err(ElasticPlateReject::TractionNotTensile { traction_mpa: t });
    }
    let (nx, ny) = (question.points_x_mm.len(), question.points_y_mm.len());
    if nx != ny {
        return Err(ElasticPlateReject::PointCountsDiffer { x_count: nx, y_count: ny });
    }
    if nx > MAX_POINTS {
        return Err(ElasticPlateReject::TooManyPoints { count: nx });
    }
    for (&x, &y) in question.points_x_mm.iter().zip(&question.points_y_mm) {
        if x.abs() > PARALLEL_HALF_LENGTH_MM || y.abs() > WIDTH_MM / 2.0 {
            return Err(ElasticPlateReject::PointOutsidePlate { x_mm: x, y_mm: y });
        }
        if x * x + y * y < HOLE_RADIUS_MM * HOLE_RADIUS_MM {
            return Err(ElasticPlateReject::PointInsideHole { x_mm: x, y_mm: y });
        }
    }
    Ok(())
}

/// Settles the elastic field at one traction and reads it at the asked
/// points, or names the limit that stopped it.
///
/// Each value of the answer below the normal range is written as zero
/// with its sign ([`crate::normal`]).
pub fn solve(question: &ElasticPlateQuestion) -> Result<ElasticPlateField, ElasticPlateReject> {
    crate::normal::held(solve_as_computed(question))
}

/// The answer as the equations compute it, before a value below the
/// normal range is written as zero.
fn solve_as_computed(question: &ElasticPlateQuestion) -> Result<ElasticPlateField, ElasticPlateReject> {
    solve_with(question, &SERVED)
}

/// The point field of `stress_plate_hole_s235`, which this solver returns
/// at each point asked for, held to the rule here.
impl NormalOrZero for PointField {
    fn normal_or_zero(&mut self) {
        self.displacement_x_mm.normal_or_zero();
        self.displacement_y_mm.normal_or_zero();
        self.stress_xx_mpa.normal_or_zero();
        self.stress_yy_mpa.normal_or_zero();
        self.stress_xy_mpa.normal_or_zero();
        self.von_mises_mpa.normal_or_zero();
        self.strain_xx.normal_or_zero();
        self.strain_yy.normal_or_zero();
        self.strain_xy.normal_or_zero();
    }
}

impl NormalOrZero for ElasticPlateField {
    fn normal_or_zero(&mut self) {
        self.traction_mpa.normal_or_zero();
        self.axial_force_n.normal_or_zero();
        self.net_section_stress_mpa.normal_or_zero();
        self.hole_edge_stress_mpa.normal_or_zero();
        self.stress_concentration_gross.normal_or_zero();
        self.stress_concentration_net.normal_or_zero();
        self.peak_von_mises_mpa.normal_or_zero();
        self.largest_strain.normal_or_zero();
        self.parallel_section_elongation_mm.normal_or_zero();
        self.strain_energy_j.normal_or_zero();
        self.boundary_work_j.normal_or_zero();
        self.points.normal_or_zero();
        self.equilibrium_residual.normal_or_zero();
        self.energy_residual.normal_or_zero();
        self.section_force_residual.normal_or_zero();
        self.hole_traction_residual.normal_or_zero();
        self.displacement_change.normal_or_zero();
        self.stress_change.normal_or_zero();
        self.nodes.normal_or_zero();
    }
}

/// One level's outcome.
enum LevelOutcome {
    Settled(Box<(Built, Settled)>),
    Strain { traction: f64, strain: f64 },
}

/// One level's elastic solve, in the units of `unit`.
fn run_level(level: u32, traction: f64, unit: i32, numerics: &Numerics) -> Result<LevelOutcome, ElasticPlateReject> {
    let mut layout = plate::specimen_layout(level);
    if let Some(strip) = layout.strip.as_mut() {
        strip.loaded_end = numerics.loaded_end;
    }
    let built = plate::build(&layout);
    let mesh = &built.mesh;
    let mut load = vec![0.0; mesh.x.len() * 2];
    let strip = layout.strip.expect("the specimen has a strip");
    let grip_half_width = layout.patch + strip.fillet_rise;
    let end_traction = times_two_to(traction, unit) * (WIDTH_MM / 2.0) / grip_half_width;
    for edge in &built.end_edges {
        fe::edge_traction(mesh, *edge, [end_traction, 0.0], &mut load);
    }
    let mut held: Vec<(usize, f64)> = built.bottom.iter().map(|&n| (2 * n + 1, 0.0)).collect();
    held.extend(built.left.iter().map(|&n| (2 * n, 0.0)));
    let problem = Problem { mesh, held, load };
    let limit = times_two_to(numerics.small_strain_limit, unit);
    let stop = |_: f64, _: &[f64], points: &[PointState]| -> Option<f64> {
        let strain = fe::largest_point_strain(&STEEL, points);
        (strain > limit).then_some(strain)
    };
    match fe::settle(&problem, &STEEL, &numerics.path, &stop) {
        Ok(settled) => Ok(LevelOutcome::Settled(Box::new((built, settled)))),
        Err(PathFailure::Stopped { load_factor, value }) => Ok(LevelOutcome::Strain { traction: load_factor * traction, strain: times_two_to(value, -unit) }),
        Err(PathFailure::Mesh(_)) => Err(ElasticPlateReject::MeshInverted),
        Err(PathFailure::NotFinite) => Err(ElasticPlateReject::NonFiniteValue),
        Err(PathFailure::ElasticSingular { .. }) => Err(ElasticPlateReject::SingularStiffness),
        Err(PathFailure::EnergyLedger { residual, .. }) => Err(ElasticPlateReject::EnergyLedgerBroken { residual }),
        // A material with no yield strength never leaves Hooke's law: the
        // path is the one elastic step from rest, which takes no return to
        // a yield surface and no load step (a test).
        Err(PathFailure::ReturnNotConverged | PathFailure::NoEquilibrium { .. }) => unreachable!("an elastic path takes no plastic step"),
    }
}

/// The stresses of a field at every integration point.
fn point_stresses(settled: &Settled) -> [Vec<f64>; 3] {
    let mut out = [Vec::new(), Vec::new(), Vec::new()];
    for p in &settled.points {
        for c in 0..3 {
            out[c].push(p.stress[c]);
        }
    }
    out
}

/// The force per unit thickness each cut carries, from the stresses.
fn cut_forces(built: &Built, stresses: &[Vec<f64>; 3]) -> Vec<(f64, f64)> {
    // Five Gauss points along each edge.
    const S: [f64; 5] = [-0.906_179_845_938_664, -0.538_469_310_105_683, 0.0, 0.538_469_310_105_683, 0.906_179_845_938_664];
    const W: [f64; 5] = [0.236_926_885_056_189, 0.478_628_670_499_366, 0.568_888_888_888_889, 0.478_628_670_499_366, 0.236_926_885_056_189];
    let mesh = &built.mesh;
    built
        .cuts
        .iter()
        .map(|(x, edges)| {
            let mut force = 0.0;
            for (element, side) in edges {
                for (s, w) in S.iter().zip(W) {
                    let (xi, eta) = match side {
                        Side::Left => (-1.0, *s),
                        Side::Top => (*s, 1.0),
                    };
                    let (_, dxi, deta) = fe::shape(xi, eta);
                    let nodes = &mesh.elements[*element];
                    let mut dy = 0.0;
                    for k in 0..8 {
                        dy += match side {
                            Side::Left => deta[k],
                            Side::Top => dxi[k],
                        } * mesh.y[nodes[k]];
                    }
                    let sxx = fe::point_value_in(&stresses[0], *element, xi, eta);
                    force += w * sxx * dy;
                }
            }
            (*x, force)
        })
        .collect()
}

fn solve_with(question: &ElasticPlateQuestion, numerics: &Numerics) -> Result<ElasticPlateField, ElasticPlateReject> {
    admit(question)?;
    let unit = if numerics.scaled_units { unit_exponent(question.traction_mpa) } else { 0 };
    let ladder = settle_ladder(question.traction_mpa, unit, numerics)?;
    read_field(question, numerics, ladder, unit)
}

/// The accepted level of the ladder.
struct Ladder {
    level: u32,
    built: Built,
    settled: Settled,
    measures: (f64, f64),
}

/// Settles the field on meshes of rising level until two successive ones
/// agree, or names the limit that stopped it.
fn settle_ladder(traction: f64, unit: i32, numerics: &Numerics) -> Result<Ladder, ElasticPlateReject> {
    let mut previous: Option<LevelOutcome> = None;
    let mut last_change = f64::NAN;
    for level in 0..=numerics.max_level {
        let outcome = run_level(level, traction, unit, numerics)?;
        let last = level == numerics.max_level;
        match (&previous, &outcome) {
            (Some(LevelOutcome::Settled(coarse)), LevelOutcome::Settled(fine)) => {
                let (change, measures) = settle_measures(&coarse.0, &coarse.1, &fine.0, &fine.1, numerics)?;
                last_change = change;
                if measures.0 <= numerics.displacement_settle && measures.1 <= numerics.stress_settle {
                    let LevelOutcome::Settled(fine) = outcome else { unreachable!() };
                    let (built, settled) = *fine;
                    return Ok(Ladder { level, built, settled, measures });
                }
            }
            // A limit is named when the finest mesh reaches it, or two
            // successive meshes do.
            (previous, LevelOutcome::Strain { traction: reached, strain }) if last || matches!(previous, Some(LevelOutcome::Strain { .. })) => {
                return Err(ElasticPlateReject::StrainBeyondSmallStrain { traction_mpa: *reached, strain: *strain, level });
            }
            _ => {}
        }
        previous = Some(outcome);
    }
    Err(ElasticPlateReject::NotSettled { change: last_change })
}

/// The settling measures between two levels: the change of displacement at
/// their common nodes of the parallel section over its largest
/// displacement, and of the hole's edge stress over itself. The first
/// number is the larger of the two over its settling fraction.
fn settle_measures(coarse: &Built, coarse_field: &Settled, fine: &Built, fine_field: &Settled, numerics: &Numerics) -> Result<(f64, (f64, f64)), ElasticPlateReject> {
    let index: HashMap<(u8, u32, u32), usize> = fine.key.iter().enumerate().map(|(i, k)| (*k, i)).collect();
    let largest = (0..fine.mesh.x.len()).filter(|&n| fine.mesh.x[n] <= PARALLEL_HALF_LENGTH_MM).fold(0.0_f64, |a, n| a.max(fine_field.displacement[2 * n].abs()).max(fine_field.displacement[2 * n + 1].abs()));
    let mut worst = 0.0_f64;
    for (node, (block, p, q)) in coarse.key.iter().enumerate() {
        if coarse.mesh.x[node] > PARALLEL_HALF_LENGTH_MM {
            continue;
        }
        let Some(&other) = index.get(&(*block, 2 * p, 2 * q)) else { continue };
        for c in 0..2 {
            worst = worst.max((coarse_field.displacement[2 * node + c] - fine_field.displacement[2 * other + c]).abs());
        }
    }
    let displacement = if largest > 0.0 { worst / largest } else { worst };
    let (c_edge, f_edge) = (top_of_hole_value(coarse, &point_stresses(coarse_field)[0]), top_of_hole_value(fine, &point_stresses(fine_field)[0]));
    let stress = (c_edge - f_edge).abs() / f_edge.abs();
    if !(displacement.is_finite() && stress.is_finite()) {
        return Err(ElasticPlateReject::NonFiniteValue);
    }
    let change = (displacement / numerics.displacement_settle).max(stress / numerics.stress_settle);
    Ok((change, (displacement, stress)))
}

/// A field of the integration points at the top of the hole's edge,
/// averaged over the elements that share that node.
fn top_of_hole_value(built: &Built, values: &[f64]) -> f64 {
    let node = built.top_of_hole;
    let mut sum = 0.0;
    let mut count = 0;
    for (index, element) in built.mesh.elements.iter().enumerate() {
        for k in 0..8 {
            if element[k] == node {
                sum += fe::point_value_in(values, index, fe::NODE_XI[k], fe::NODE_ETA[k]);
                count += 1;
            }
        }
    }
    sum / count as f64
}

/// The field read from the accepted level, solved in the units of `unit`
/// and returned in MPa and mm: every stress, strain and displacement times
/// two to the power `-unit`, every energy times two to `-2 unit`, each
/// rounded once; every ratio is taken in the units it was solved in.
fn read_field(question: &ElasticPlateQuestion, numerics: &Numerics, ladder: Ladder, unit: i32) -> Result<ElasticPlateField, ElasticPlateReject> {
    let Ladder { level, built, settled, measures } = ladder;
    let traction = question.traction_mpa;
    let scaled_traction = times_two_to(traction, unit);
    let down = |value: f64| times_two_to(value, -unit);
    let down_twice = |value: f64| times_two_to(value, -2 * unit);
    let mesh = &built.mesh;
    if !(settled.imbalance <= numerics.path.imbalance_tolerance) {
        return Err(ElasticPlateReject::EquilibriumBroken { imbalance: settled.imbalance });
    }
    let stresses = point_stresses(&settled);
    // The ledger of the cuts.
    let quarter_force = scaled_traction * WIDTH_MM / 2.0;
    let mut section_force_residual = 0.0_f64;
    for (x, force) in cut_forces(&built, &stresses) {
        let residual = (force / quarter_force - 1.0).abs();
        if !residual.is_finite() {
            return Err(ElasticPlateReject::NonFiniteValue);
        }
        if residual > numerics.section_force_tolerance {
            return Err(ElasticPlateReject::SectionForceBroken { x_mm: x, residual });
        }
        section_force_residual = section_force_residual.max(residual);
    }
    let nodal: Vec<Vec<f64>> = stresses.iter().map(|s| fe::nodal_average(mesh, s)).collect();
    // The hole's edge carries no traction.
    let mut hole_traction = 0.0_f64;
    let mut hoop_largest = 0.0_f64;
    for &n in &built.hole {
        let theta = math::atan2(mesh.y[n], mesh.x[n]);
        let (c, s) = (math::cos(theta), math::sin(theta));
        let (sxx, syy, sxy) = (nodal[0][n], nodal[1][n], nodal[2][n]);
        let srr = sxx * c * c + syy * s * s + 2.0 * sxy * s * c;
        let srt = (syy - sxx) * s * c + sxy * (c * c - s * s);
        let stt = sxx * s * s + syy * c * c - 2.0 * sxy * s * c;
        hole_traction = hole_traction.max(srr.abs()).max(srt.abs());
        hoop_largest = hoop_largest.max(stt.abs());
    }
    let hole_traction_residual = hole_traction / hoop_largest;
    let hole_edge = top_of_hole_value(&built, &stresses[0]);
    let net = traction * WIDTH_MM / (WIDTH_MM - 2.0 * HOLE_RADIUS_MM);
    let scaled_net = scaled_traction * WIDTH_MM / (WIDTH_MM - 2.0 * HOLE_RADIUS_MM);
    let peak = settled.points.iter().fold(0.0_f64, |m, p| m.max(fe::von_mises(&p.stress)));
    let largest_strain = fe::largest_point_strain(&STEEL, &settled.points);
    let end_node = built
        .key
        .iter()
        .position(|k| *k == (0, (2 * (plate::RADIAL_ELEMENTS + plate::ALONG_ELEMENTS) << level) as u32, 0))
        .expect("the node at the end of the parallel section");
    let elongation = 2.0 * settled.displacement[2 * end_node];
    let to_joules = 4.0 * THICKNESS_MM * 1.0e-3;
    let mut points = Vec::with_capacity(question.points_x_mm.len());
    for (&x, &y) in question.points_x_mm.iter().zip(&question.points_y_mm) {
        points.push(point_in_mpa(field_at(&built, &settled, &stresses, x, y)?, unit));
    }
    let mut nodes = Vec::with_capacity(mesh.x.len() * 7);
    for n in 0..mesh.x.len() {
        nodes.extend_from_slice(&[mesh.x[n], mesh.y[n], down(settled.displacement[2 * n]), down(settled.displacement[2 * n + 1]), down(nodal[0][n]), down(nodal[1][n]), down(nodal[2][n])]);
    }
    let field = ElasticPlateField {
        traction_mpa: traction,
        axial_force_n: traction * WIDTH_MM * THICKNESS_MM,
        net_section_stress_mpa: net,
        hole_edge_stress_mpa: down(hole_edge),
        stress_concentration_gross: hole_edge / scaled_traction,
        stress_concentration_net: hole_edge / scaled_net,
        peak_von_mises_mpa: down(peak),
        largest_strain: down(largest_strain),
        parallel_section_elongation_mm: down(elongation),
        strain_energy_j: down_twice(settled.strain_energy * to_joules),
        boundary_work_j: down_twice(settled.boundary_work * to_joules),
        past_compared_traction: traction > COMPARED_TRACTION_MPA,
        points,
        equilibrium_residual: settled.imbalance,
        energy_residual: settled.energy_residual,
        section_force_residual,
        hole_traction_residual,
        level,
        elements: mesh.elements.len(),
        displacement_change: measures.0,
        stress_change: measures.1,
        nodes,
    };
    let numbers = [
        field.hole_edge_stress_mpa,
        field.peak_von_mises_mpa,
        field.largest_strain,
        field.parallel_section_elongation_mm,
        field.strain_energy_j,
        field.boundary_work_j,
        field.hole_traction_residual,
    ];
    if numbers.iter().chain(&field.nodes).any(|v| !v.is_finite()) {
        return Err(ElasticPlateReject::NonFiniteValue);
    }
    Ok(field)
}

/// The field at a point of the whole plate, from the quarter by its two
/// mirror planes.
fn field_at(built: &Built, settled: &Settled, stresses: &[Vec<f64>; 3], x: f64, y: f64) -> Result<PointField, ElasticPlateReject> {
    let (sx, sy) = (if x < 0.0 { -1.0 } else { 1.0 }, if y < 0.0 { -1.0 } else { 1.0 });
    let (qx, qy) = (x.abs(), y.abs());
    let mesh = &built.mesh;
    let (element, xi, eta) = plate::locate_in_parallel_length(mesh, qx, qy).ok_or(ElasticPlateReject::PointOutsidePlate { x_mm: x, y_mm: y })?;
    let u = fe::displacement_in(mesh, &settled.displacement, element, xi, eta);
    let s = [
        fe::point_value_in(&stresses[0], element, xi, eta),
        fe::point_value_in(&stresses[1], element, xi, eta),
        fe::point_value_in(&stresses[2], element, xi, eta),
    ];
    // The strain from the displacement's derivatives at the point.
    let (_, dxi, deta) = fe::shape(xi, eta);
    let nodes = &mesh.elements[element];
    let (mut a, mut b, mut c, mut d) = (0.0, 0.0, 0.0, 0.0);
    for k in 0..8 {
        a += dxi[k] * mesh.x[nodes[k]];
        b += deta[k] * mesh.x[nodes[k]];
        c += dxi[k] * mesh.y[nodes[k]];
        d += deta[k] * mesh.y[nodes[k]];
    }
    let det = a * d - b * c;
    let (mut exx, mut eyy, mut gxy) = (0.0, 0.0, 0.0);
    for k in 0..8 {
        let dx = (d * dxi[k] - c * deta[k]) / det;
        let dy = (-b * dxi[k] + a * deta[k]) / det;
        let (ux, uy) = (settled.displacement[2 * nodes[k]], settled.displacement[2 * nodes[k] + 1]);
        exx += dx * ux;
        eyy += dy * uy;
        gxy += dy * ux + dx * uy;
    }
    Ok(PointField {
        displacement_x_mm: sx * u[0],
        displacement_y_mm: sy * u[1],
        stress_xx_mpa: s[0],
        stress_yy_mpa: s[1],
        stress_xy_mpa: sx * sy * s[2],
        von_mises_mpa: fe::von_mises(&s),
        strain_xx: exx,
        strain_yy: eyy,
        strain_xy: sx * sy * 0.5 * gxy,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ask(traction: f64, xs: &[f64], ys: &[f64]) -> ElasticPlateQuestion {
        ElasticPlateQuestion { traction_mpa: traction, points_x_mm: xs.to_vec(), points_y_mm: ys.to_vec() }
    }

    /// The worked question, settled once for the tests that read it.
    fn measured() -> &'static ElasticPlateField {
        static FIELD: std::sync::OnceLock<ElasticPlateField> = std::sync::OnceLock::new();
        FIELD.get_or_init(|| solve(&ask(COMPARED_TRACTION_MPA, &[0.0, 0.0, 4.5, -20.0, 20.0], &[4.0, 10.025, 0.0, 5.0, -5.0])).expect("settles"))
    }

    /// G^G's recomputation of 16 October 2026 (evidence/research/gg_argon_2026-10-16/
    /// s1_recompute/stress_plate_hole_s235_elastic/NOTES.md, 3.1): the parallel
    /// length's last node fell 1 to 2 units in the last place short of 45 mm,
    /// since graded(n, n, ratio) was not exactly 1, and a point on the declared
    /// end was read from the shoulder fillet's element: at 106.26 MPa and
    /// y = -10.025 mm, sigma_xx 122.557 MPa at x = 45.0 against 118.703 at
    /// 44.999999999999, and sigma_yy +0.710 against -0.383. A point at either
    /// end of the parallel length, on either edge or inside, is read from the
    /// parallel section's element, as the points just inside it are.
    #[test]
    fn a_point_at_the_end_of_the_parallel_length_is_read_from_its_element() {
        let inside = 44.999999999999;
        let mut xs = Vec::new();
        let mut ys = Vec::new();
        for y in [-10.025, 10.025, -5.0, 5.0] {
            for x in [45.0, -45.0, inside, -inside] {
                xs.push(x);
                ys.push(y);
            }
        }
        let field = solve(&ask(106.26, &xs, &ys)).expect("settles");
        for (k, chunk) in field.points.chunks(4).enumerate() {
            for (end, near) in [(&chunk[0], &chunk[2]), (&chunk[1], &chunk[3])] {
                let at = (xs[4 * k], ys[4 * k]);
                assert!((end.stress_xx_mpa / near.stress_xx_mpa - 1.0).abs() < 1.0e-9, "{at:?}: {} {}", end.stress_xx_mpa, near.stress_xx_mpa);
                assert!((end.stress_yy_mpa - near.stress_yy_mpa).abs() < 1.0e-9, "{at:?}: {} {}", end.stress_yy_mpa, near.stress_yy_mpa);
                assert!((end.stress_xy_mpa - near.stress_xy_mpa).abs() < 1.0e-9, "{at:?}: {} {}", end.stress_xy_mpa, near.stress_xy_mpa);
            }
        }
    }

    /// The field is `stress_plate_hole_s235`'s with the steel not yielding,
    /// bit for bit: the field its record reported and did not gate
    /// (`analysis/out/03_scored.json`, "elastic"), at its constants.
    #[test]
    fn the_field_is_the_original_s_with_the_steel_not_yielding() {
        let points = [(0.0, 4.0), (0.0, 10.025), (4.5, 0.0), (-20.0, 5.0), (20.0, -5.0), (-0.5, 5.4), (1.0, -6.3), (30.0, 9.0)];
        let (xs, ys): (Vec<f64>, Vec<f64>) = points.iter().copied().unzip();
        for t in [COMPARED_TRACTION_MPA, 50.0, 1.0e-3, 200.0] {
            let field = solve(&ask(t, &xs, &ys)).expect("settles");
            let (original, at) = plate::probe_with_moduli(t, YOUNGS_MODULUS_MPA, POISSON_RATIO, false, &points).expect("settles");
            assert_eq!(field.points, at, "{t}");
            assert_eq!(field.nodes, original.nodes, "{t}");
            assert_eq!(field.hole_edge_stress_mpa, original.hole_edge_stress_mpa, "{t}");
            assert_eq!(field.hole_edge_stress_mpa, original.elastic_hole_edge_stress_mpa, "{t}");
            assert_eq!(field.peak_von_mises_mpa, original.peak_von_mises_mpa, "{t}");
            assert_eq!(field.strain_energy_j, original.strain_energy_j, "{t}");
            assert_eq!(field.boundary_work_j, original.boundary_work_j, "{t}");
            assert_eq!(field.largest_strain, original.largest_strain, "{t}");
            assert_eq!(field.parallel_section_elongation_mm, original.parallel_section_elongation_mm, "{t}");
            assert_eq!(field.section_force_residual, original.section_force_residual, "{t}");
            assert_eq!(field.hole_traction_residual, original.hole_traction_residual, "{t}");
            assert_eq!((field.displacement_change, field.stress_change), (original.displacement_change, original.stress_change), "{t}");
            assert_eq!(original.plastic_dissipation_j, 0.0);
            assert_eq!(field.level, MAX_LEVEL);
        }
        // The numbers the record reported for that field at 106.26 MPa, as
        // genesis_kernel::math computes them (since genesis_solvers 0.12.0):
        // the record's, from the platform's math, are within 2e-15 of these,
        // and its settling measures within 2e-10. Since the fix pass of
        // 0.14.0 the parallel length's last node is at 45 mm exactly, where it
        // was a unit or two in the last place short: the stresses and the
        // strain moved by under 5e-14 of themselves (397.6221454436819,
        // 2.2489169756750447 and 0.0018008893006263005 before), the residual
        // and the settling measures, differences of nearly equal numbers, by
        // under 2e-10 (0.0010855396220219582, 0.00013732361200618053 and
        // 0.0003762198852329818 before).
        let field = measured();
        assert_eq!(field.hole_edge_stress_mpa, 397.62214544369976);
        assert_eq!(field.stress_concentration_net, 2.2489169756751455);
        assert_eq!(field.largest_strain, 0.0018008893006263727);
        assert_eq!(field.section_force_residual, 0.001085539622060594);
        assert_eq!(field.displacement_change, 0.0001373236120169337);
        assert_eq!(field.stress_change, 0.0003762198852914349);
    }

    /// Below 63.49 MPa, where `stress_plate_hole_s235`'s steel has not
    /// yielded, the two solvers serve the same field, bit for bit.
    #[test]
    fn below_first_yield_the_original_serves_the_same_field() {
        let (xs, ys) = ([0.0, 1.0, -20.0, 30.0], [5.5, -6.3, 5.0, 9.0]);
        for t in [50.0, 63.0, 1.0e-200] {
            let field = solve(&ask(t, &xs, &ys)).expect("settles");
            let served = plate::solve(&plate::PlateQuestion { traction_mpa: t, points_x_mm: xs.to_vec(), points_y_mm: ys.to_vec() }).expect("settles");
            assert_eq!(served.plastic_zone_area_mm2, 0.0, "{t}");
            assert_eq!(field.points, served.points, "{t}");
            assert_eq!(field.nodes, served.nodes, "{t}");
            assert_eq!(field.hole_edge_stress_mpa, served.hole_edge_stress_mpa, "{t}");
            assert_eq!(field.strain_energy_j, served.strain_energy_j, "{t}");
            assert_eq!(field.boundary_work_j, served.boundary_work_j, "{t}");
        }
    }

    /// The ledger closes at the worked question: the force balance, the
    /// work of the forces against the strain energy, the force across every
    /// cut, and a free hole.
    #[test]
    fn the_ledger_closes_at_the_worked_question() {
        let field = measured();
        assert!(field.equilibrium_residual <= IMBALANCE_TOLERANCE, "{}", field.equilibrium_residual);
        assert!(field.energy_residual <= ENERGY_TOLERANCE, "{}", field.energy_residual);
        assert!(field.section_force_residual <= SECTION_FORCE_TOLERANCE, "{}", field.section_force_residual);
        assert!(field.hole_traction_residual < 1e-3, "{}", field.hole_traction_residual);
        let closure = (field.boundary_work_j - field.strain_energy_j) / field.boundary_work_j;
        assert!(closure.abs() < 1e-12, "{closure}");
        assert!((field.axial_force_n - 106.26 * 20.05 * 3.05).abs() < 1e-9);
        assert!((field.stress_concentration_gross - 3.742).abs() < 5e-4, "{}", field.stress_concentration_gross);
        assert!((field.peak_von_mises_mpa - 393.3).abs() < 0.05, "{}", field.peak_von_mises_mpa);
        assert!(!field.past_compared_traction);
        assert_eq!(field.elements, 1280);
        assert_eq!(field.nodes.len(), 7 * 4033);
    }

    /// A ledger set past what the arithmetic holds is named, never passed.
    #[test]
    fn each_ledger_broken_on_purpose_is_named() {
        let q = ask(COMPARED_TRACTION_MPA, &[], &[]);
        let energy = Numerics { path: PathNumerics { energy_tolerance: 1e-18, ..SERVED.path }, ..SERVED };
        assert!(matches!(solve_with(&q, &energy), Err(ElasticPlateReject::EnergyLedgerBroken { .. })));
        // The work of the step against its end forces alone, the rectangle:
        // twice the strain energy, a residual of one half.
        let rectangle = Numerics { path: PathNumerics { fault: LedgerFault::RectangleWork, ..SERVED.path }, ..SERVED };
        match solve_with(&q, &rectangle) {
            Err(ElasticPlateReject::EnergyLedgerBroken { residual }) => assert!((residual - 0.5).abs() < 1e-9, "{residual}"),
            other => panic!("{:?}", other.map(|f| f.energy_residual)),
        }
        let cut = Numerics { section_force_tolerance: 1e-6, ..SERVED };
        assert!(matches!(solve_with(&q, &cut), Err(ElasticPlateReject::SectionForceBroken { .. })));
        let tight = Numerics { path: PathNumerics { imbalance_tolerance: 1e-20, ..SERVED.path }, ..SERVED };
        assert!(matches!(solve_with(&q, &tight), Err(ElasticPlateReject::EquilibriumBroken { .. })));
        let one = Numerics { max_level: 1, ..SERVED };
        assert!(matches!(solve_with(&q, &one), Err(ElasticPlateReject::NotSettled { .. })));
        let strict = Numerics { displacement_settle: 1e-9, ..SERVED };
        assert!(matches!(solve_with(&q, &strict), Err(ElasticPlateReject::NotSettled { .. })));
        let small = Numerics { small_strain_limit: 1e-3, ..SERVED };
        assert!(matches!(solve_with(&q, &small), Err(ElasticPlateReject::StrainBeyondSmallStrain { .. })));
    }

    /// The path is the one elastic step from rest on every mesh: no return
    /// to a yield surface, no load step, no Newton iteration.
    #[test]
    fn the_path_takes_no_plastic_step() {
        for level in 0..=MAX_LEVEL {
            match run_level(level, 1.5, unit_exponent(1.5), &SERVED).expect("solved") {
                LevelOutcome::Settled(found) => {
                    let settled = &found.1;
                    assert_eq!(settled.steps.len(), 1, "{level}");
                    assert_eq!(settled.newton_iterations, 0, "{level}");
                    assert_eq!(settled.first_yield_factor, None, "{level}");
                    assert_eq!(settled.plastic_dissipation, 0.0, "{level}");
                    assert_eq!(settled.load_factor, 1.0, "{level}");
                }
                LevelOutcome::Strain { .. } => panic!("{level}"),
            }
        }
    }

    /// The field is linear in the traction, past the compared traction and
    /// past 235 MPa: nothing yields.
    #[test]
    fn the_field_is_linear_in_the_traction() {
        let a = solve(&ask(100.0, &[10.0, 1.0], &[5.0, 5.5])).expect("settles");
        let b = solve(&ask(400.0, &[10.0, 1.0], &[5.0, 5.5])).expect("settles");
        for (p, q) in a.points.iter().zip(&b.points) {
            assert!((q.displacement_x_mm / p.displacement_x_mm - 4.0).abs() < 1e-12);
            assert!((q.stress_xx_mpa / p.stress_xx_mpa - 4.0).abs() < 1e-12);
            assert!((q.strain_xx / p.strain_xx - 4.0).abs() < 1e-12);
        }
        assert!((b.hole_edge_stress_mpa / a.hole_edge_stress_mpa - 4.0).abs() < 1e-12);
        assert!((a.stress_concentration_gross - b.stress_concentration_gross).abs() < 1e-12);
        assert!((b.strain_energy_j / a.strain_energy_j - 16.0).abs() < 1e-11);
        assert!(!a.past_compared_traction && b.past_compared_traction);
        // Above 235 MPa of traction, where the original refuses, the
        // elastic field is answered.
        assert!(matches!(plate::admit(&plate::PlateQuestion { traction_mpa: 300.0, points_x_mm: vec![], points_y_mm: vec![] }), Err(plate::PlateReject::TractionAboveYield { .. })));
        assert!(solve(&ask(300.0, &[], &[])).is_ok());
    }

    /// The flag is set above the compared traction and only there.
    #[test]
    fn past_the_compared_traction_the_answer_says_so() {
        let at = solve(&ask(COMPARED_TRACTION_MPA, &[], &[])).expect("settles");
        assert!(!at.past_compared_traction);
        let next = f64::from_bits(COMPARED_TRACTION_MPA.to_bits() + 1);
        assert!(solve(&ask(next, &[], &[])).expect("settles").past_compared_traction);
        assert!(!solve(&ask(1.0, &[], &[])).expect("settles").past_compared_traction);
    }

    /// The small-strain limit: the field is linear, so it is one traction.
    /// 590.04 MPa is answered and 590.05 MPa refused, on the finest mesh.
    #[test]
    fn the_small_strain_limit_is_one_traction() {
        let last = solve(&ask(590.04, &[], &[])).expect("settles");
        assert!(last.largest_strain < SMALL_STRAIN_LIMIT && last.largest_strain > 0.99995 * SMALL_STRAIN_LIMIT, "{}", last.largest_strain);
        match solve(&ask(590.05, &[], &[])) {
            Err(ElasticPlateReject::StrainBeyondSmallStrain { traction_mpa, strain, level }) => {
                assert_eq!(traction_mpa, 590.05);
                assert!(strain > SMALL_STRAIN_LIMIT);
                assert_eq!(level, MAX_LEVEL);
            }
            other => panic!("{:?}", other.map(|f| f.largest_strain)),
        }
        // Far past it two meshes reach it, and it is named on the second.
        assert!(matches!(solve(&ask(1.0e6, &[], &[])), Err(ElasticPlateReject::StrainBeyondSmallStrain { level: 1, .. })));
        assert!(matches!(solve(&ask(f64::MAX, &[], &[])), Err(ElasticPlateReject::StrainBeyondSmallStrain { level: 1, .. })));
    }

    /// The scaled units give the unscaled bits wherever the unscaled
    /// arithmetic holds.
    #[test]
    fn the_scaled_units_give_the_unscaled_bits() {
        let plain = Numerics { scaled_units: false, ..SERVED };
        let q = ask(COMPARED_TRACTION_MPA, &[0.0, 0.0, 4.5, -20.0, 20.0], &[4.0, 10.025, 0.0, 5.0, -5.0]);
        assert_eq!(&solve_with(&q, &plain).expect("settles"), measured());
    }

    /// The smallest tractions are answered, their ledger closed, each value
    /// scaled back once, and a value that scaling takes under the normal
    /// range answered as zero (`crate::normal`).
    #[test]
    fn the_smallest_tractions_are_answered_with_their_ledger_closed() {
        let one = solve(&ask(1.0, &[20.0], &[5.0])).expect("settles");
        for power in [-500, -1000, -1022] {
            let t = times_two_to(1.0, power);
            let small = solve(&ask(t, &[20.0], &[5.0])).expect("settles");
            assert_eq!(small.energy_residual, one.energy_residual, "{power}");
            assert_eq!(small.stress_concentration_gross, one.stress_concentration_gross, "{power}");
            let scaled = |value: f64, power: i32| crate::normal::normal_or_zero(times_two_to(value, power));
            assert_eq!(small.points[0].displacement_x_mm.to_bits(), scaled(one.points[0].displacement_x_mm, power).to_bits(), "{power}");
            assert_eq!(small.strain_energy_j.to_bits(), scaled(one.strain_energy_j, 2 * power).to_bits(), "{power}");
        }
        let small = solve(&ask(f64::from_bits(1), &[20.0], &[5.0])).expect("settles");
        assert!(small.energy_residual <= ENERGY_TOLERANCE);
    }

    #[test]
    fn the_question_s_own_limits_are_named_before_any_field() {
        let t = COMPARED_TRACTION_MPA;
        for bad in [0.0, -1.0, -0.0] {
            assert!(matches!(admit(&ask(bad, &[], &[])), Err(ElasticPlateReject::TractionNotTensile { .. })), "{bad}");
        }
        assert!(admit(&ask(f64::from_bits(1), &[], &[])).is_ok());
        assert!(admit(&ask(f64::MAX, &[], &[])).is_ok());
        for bad in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            assert_eq!(admit(&ask(bad, &[], &[])), Err(ElasticPlateReject::InputNotFinite));
            assert_eq!(admit(&ask(t, &[bad], &[0.0])), Err(ElasticPlateReject::InputNotFinite));
            assert_eq!(admit(&ask(t, &[10.0], &[bad])), Err(ElasticPlateReject::InputNotFinite));
        }
        assert!(matches!(admit(&ask(t, &[1.0, 2.0], &[8.0])), Err(ElasticPlateReject::PointCountsDiffer { x_count: 2, y_count: 1 })));
        let many = vec![20.0; MAX_POINTS + 1];
        assert!(matches!(admit(&ask(t, &many, &many.iter().map(|_| 0.0).collect::<Vec<_>>())), Err(ElasticPlateReject::TooManyPoints { .. })));
        assert!(admit(&ask(t, &[PARALLEL_HALF_LENGTH_MM, -PARALLEL_HALF_LENGTH_MM, 0.0, 0.0, 4.0], &[WIDTH_MM / 2.0, -WIDTH_MM / 2.0, 4.0, -4.0, 0.0])).is_ok());
        for (x, y) in [(45.00000000000001, 0.0), (0.0, 10.025000000000002), (-46.0, 0.0), (0.0, -11.0)] {
            assert!(matches!(admit(&ask(t, &[x], &[y])), Err(ElasticPlateReject::PointOutsidePlate { .. })), "{x} {y}");
        }
        for (x, y) in [(0.0, 0.0), (0.0, 3.9999999999999996), (2.8, 2.8), (-3.0, -2.0)] {
            assert!(matches!(admit(&ask(t, &[x], &[y])), Err(ElasticPlateReject::PointInsideHole { .. })), "{x} {y}");
        }
        let sentence = ElasticPlateReject::TractionNotTensile { traction_mpa: -5.0 }.to_string();
        assert!(sentence.starts_with("traction -5 MPa is not a pull"));
    }

    /// The field at a point mirrors across both planes of the plate.
    #[test]
    fn the_field_mirrors_across_both_planes() {
        let field = measured();
        let (a, b) = (field.points[3], field.points[4]);
        assert!((a.displacement_x_mm + b.displacement_x_mm).abs() < 1e-15);
        assert!((a.displacement_y_mm + b.displacement_y_mm).abs() < 1e-15);
        assert_eq!(a.stress_xx_mpa, b.stress_xx_mpa);
        assert_eq!(a.stress_xy_mpa, b.stress_xy_mpa);
        let top = field.points[0];
        assert!((top.stress_xx_mpa - field.hole_edge_stress_mpa).abs() < 1e-9);
        assert_eq!(top.displacement_x_mm, 0.0);
        assert!(field.points[2].stress_yy_mpa < -80.0);
    }

    #[test]
    fn the_same_question_gives_the_same_bits() {
        let q = ask(COMPARED_TRACTION_MPA, &[0.0, 0.0, 4.5, -20.0, 20.0], &[4.0, 10.025, 0.0, 5.0, -5.0]);
        assert_eq!(&solve(&q).expect("settles"), measured());
    }

    /// The measured points of the two windows of line P3 of the original's
    /// comparison, with their measured u_x in mm: the same 40 points
    /// `stress_plate_hole_s235`'s test holds (Tröger, Hartmann, Anton,
    /// Wessels, Zenodo 10.5281/zenodo.11257192, CC BY 4.0, file
    /// 20231116_displacements_raw.csv, moved to the hole's centre of the
    /// record's registration). 22 points above the hole, then 18 below.
    const WINDOW_POINTS: [(f64, f64, f64); 40] = [
        (-1.0338880899999978, 4.547907629999999, 0.0389632400000011),
        (-1.0376856299999986, 5.13284073, 0.0395422799999992),
        (-1.041093679999996, 5.729291949999999, 0.0404711500000019),
        (-1.044551779999999, 6.324831259999998, 0.0405924000000013),
        (-0.5347639899999947, 4.84066707, 0.0397775899999999),
        (-0.5372751899999955, 5.43028196, 0.0400844300000003),
        (-0.5399779799999962, 6.027265139999999, 0.0399873599999978),
        (-0.5431898799999999, 6.623868920000001, 0.0403092600000008),
        (-0.03285826999999841, 4.5712103299999995, 0.0406904800000021),
        (-0.032936189999993815, 5.132350279999999, 0.0406131899999949),
        (-0.03510329999999584, 5.728221849999999, 0.0403099299999994),
        (-0.039080359999999814, 6.325875259999998, 0.0408639600000029),
        (-0.04177504999999826, 6.923806739999998, 0.0406370900000042),
        (0.4735982900000053, 4.83805078, 0.0407936100000015),
        (0.4698386499999998, 5.428635569999999, 0.0409144900000058),
        (0.46588953000000544, 6.02622742, 0.0408719699999978),
        (0.4632854900000041, 6.62413422, 0.0412667599999992),
        (0.9721713200000011, 4.5585131400000005, 0.0413977299999999),
        (0.9735509700000051, 5.13260938, 0.0418354600000015),
        (0.9703575000000058, 5.728728630000001, 0.0414452799999978),
        (0.9680948600000008, 6.325289969999998, 0.0417281799999998),
        (1.4756679000000048, 5.430187779999999, 0.0424246899999972),
        (-1.4787501899999995, -5.2496755, 0.038969740000006),
        (-0.9720555399999995, -6.13286142, 0.0399545300000028),
        (-0.9764489399999974, -5.543197040000001, 0.0397734200000031),
        (-0.9810050299999986, -4.9500804700000005, 0.0395415999999997),
        (-0.4710460699999999, -6.429292670000001, 0.0395754700000026),
        (-0.47394721999999945, -5.83889073, 0.0399244400000001),
        (-0.4775923199999994, -5.2469969, 0.040119770000004),
        (-0.4756168099999982, -4.641797840000001, 0.0397771100000028),
        (0.031725010000002385, -6.724053280000001, 0.0402440100000021),
        (0.028655950000000985, -6.133369500000001, 0.0408131699999998),
        (0.02475424000000004, -5.542184430000001, 0.0411149100000046),
        (0.022839260000004913, -4.95055666, 0.040889270000001),
        (0.5314055199999999, -6.42866956, 0.0406761900000063),
        (0.527748840000001, -5.837406550000001, 0.0412397500000026),
        (0.5238154500000007, -5.246867, 0.0408338400000048),
        (1.0290942400000063, -6.13143197, 0.0411645999999948),
        (1.026462880000004, -5.54186303, 0.0414498800000018),
        (1.0231286100000005, -4.9491248500000005, 0.0414370200000036),
    ];

    /// The far strain of the comparison: the mean of the two opened bands'
    /// slopes of u_x along the bar, measured and this field's
    /// (`analysis/out/03_scored.json`, over 3,269 points).
    const FAR_STRAIN_MEASURED: f64 = 0.0004868755195684971;
    const FAR_STRAIN_ELASTIC: f64 = 0.0004847822930590952;

    /// The slope in x of a plane fitted to u_x at the points.
    fn window_slope(points: &[(f64, f64, f64)], centre_y: f64) -> f64 {
        let mut n = [[0.0_f64; 3]; 3];
        let mut b = [0.0_f64; 3];
        for &(x, y, u) in points {
            let row = [1.0, x, y - centre_y];
            for i in 0..3 {
                for j in 0..3 {
                    n[i][j] += row[i] * row[j];
                }
                b[i] += row[i] * u;
            }
        }
        let det = |m: &[[f64; 3]; 3]| m[0][0] * (m[1][1] * m[2][2] - m[1][2] * m[2][1]) - m[0][1] * (m[1][0] * m[2][2] - m[1][2] * m[2][0]) + m[0][2] * (m[1][0] * m[2][1] - m[1][1] * m[2][0]);
        let mut m1 = n;
        for i in 0..3 {
            m1[i][1] = b[i];
        }
        det(&m1) / det(&n)
    }

    /// Line P3 of the original's comparison, after the fact: the strain 1.5
    /// mm from the hole's edge over the far strain is 1.889 measured and
    /// 1.917 by this solver, the measured 1.4 percent below the solver's,
    /// against a line of 20. Calibrated to the record, not a blind test.
    #[test]
    fn the_strain_near_the_hole_after_the_fact() {
        let (top, bottom) = WINDOW_POINTS.split_at(22);
        let measured = 0.5 * (window_slope(top, 5.5) + window_slope(bottom, -5.5)) / FAR_STRAIN_MEASURED;
        assert!((measured - 1.8894827826341272).abs() < 1e-9, "{measured}");
        let xs: Vec<f64> = WINDOW_POINTS.iter().map(|p| p.0).collect();
        let ys: Vec<f64> = WINDOW_POINTS.iter().map(|p| p.1).collect();
        let field = solve(&ask(COMPARED_TRACTION_MPA, &xs, &ys)).expect("settles");
        let with: Vec<(f64, f64, f64)> = WINDOW_POINTS.iter().zip(&field.points).map(|(p, f)| (p.0, p.1, f.displacement_x_mm)).collect();
        let (top, bottom) = with.split_at(22);
        let ratio = 0.5 * (window_slope(top, 5.5) + window_slope(bottom, -5.5)) / FAR_STRAIN_ELASTIC;
        assert!((ratio - 1.9171593285977524).abs() < 1e-9, "{ratio}");
        let difference = measured / ratio - 1.0;
        assert!((difference - -0.014436226322341428).abs() < 1e-8, "{difference}");
    }

    /// The red team's tractions of engine 0.13.0 (D1). At 1e-154 MPa the
    /// strain energy and the boundary work, which go with the square of the
    /// traction, come to 2.06e-313 J as computed; at 1e-300 MPa the shear
    /// strain at the points does. Each is answered as zero with its sign,
    /// every number of the field is normal or zero, and every other number
    /// is the computed one bit for bit.
    #[test]
    fn a_value_under_the_smallest_normal_number_is_answered_as_zero() {
        for traction in [1.0e-154, 1.0e-155, 1.0e-300, f64::MIN_POSITIVE] {
            let question = ask(traction, &[0.0, 4.5], &[10.025, 0.0]);
            let computed = solve_as_computed(&question).expect("settles");
            let answered = solve(&question).expect("settles");
            let mut held = computed.clone();
            held.normal_or_zero();
            assert_eq!(held, answered, "{traction:e}");
            let mut again = answered.clone();
            again.normal_or_zero();
            assert_eq!(again, answered, "{traction:e}: a subnormal number is answered");
            assert_ne!(computed, answered, "{traction:e}: nothing under the normal range was computed");
            if traction == 1.0e-154 {
                assert!(computed.strain_energy_j.is_subnormal() && computed.boundary_work_j.is_subnormal());
                assert_eq!((answered.strain_energy_j.to_bits(), answered.boundary_work_j.to_bits()), (0, 0));
            }
        }
    }
}
