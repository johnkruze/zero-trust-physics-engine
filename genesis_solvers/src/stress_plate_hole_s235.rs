//! The stress and displacement field of an S235 steel plate with a hole,
//! pulled.
//!
//! The plate is the tensile bar of Tröger, Hartmann, Anton and Wessels
//! (Clausthal University of Technology and TU Braunschweig, measured
//! 2023-11-16; Zenodo, <https://doi.org/10.5281/zenodo.11257192>, version
//! 1.0, CC BY 4.0): S235 low-carbon steel 3.05 mm thick, its parallel length
//! 20.05 mm wide and 90 mm long with a hole of radius 4 mm at its centre
//! (d/W = 0.399), fillets of radius 25 mm to grip ends 30 mm wide. The
//! question is the traction on the gross section of the parallel length and
//! the points at which the field is wanted; the answer is the settled field:
//! the displacement and the full in-plane stress tensor everywhere, read at
//! those points, the stress at the hole's edge, the yielded zone, and the
//! ledger.
//!
//! **The physics.** Plane stress, small strain, Hooke's law with Young's
//! modulus and Poisson's ratio fitted to this sheet, and von Mises yield at
//! 235 MPa with no hardening (the yield plateau of a mild steel), integrated
//! by the closest-point return with its consistent tangent
//! (`plane_stress_fe`). 235 MPa is the least upper yield strength that
//! EN 10025-2 asks of S235 up to 16 mm thick; the record does not give this
//! sheet's own. The elastic field puts 3.742 times the gross traction at the
//! top of the hole's edge (2.249 times the net): the edge's stress, carried
//! to its node, reaches the yield strength at 62.80 MPa of traction, and the
//! integration points, where the material law is evaluated, from 63.49 MPa,
//! where the solve's yielding begins and the traction served as first
//! yield. At the laboratory's 106.26 MPa the elastic stress at the edge
//! would be 397.6 MPa: the solver carries the field past yield instead of
//! stopping there. The field is solved in MPa times the power of two that
//! carries the traction into [1, 2), and each value carried back once, so
//! that no product of the arithmetic leaves the range of 64-bit numbers at
//! any traction; at every traction where none did, the bits are the
//! unscaled solve's (a test).
//!
//! **The field.** The quarter `x, y >= 0` of the bar with its two mirror
//! planes, out to 75 mm from the hole's centre: the parallel length, the
//! fillet and 15 mm of the grip end, where a uniform traction carries the
//! same force. Moving that end to 105 mm moves the stress of the parallel
//! length by at most 0.05 MPa (a test). Eight-node quadrilaterals, 3 x 3
//! Gauss points, graded toward the hole, on three meshes of 80, 320 and 1,280
//! elements; each is solved along the whole load path, and the finest of two
//! successive meshes that agree is accepted: the displacement of the
//! parallel length to `DISPLACEMENT_SETTLE` of its largest, the elastic
//! stress at the top of the hole's edge to `STRESS_SETTLE`, the plastic
//! dissipation to `DISSIPATION_SETTLE` of the strain energy. Over the whole
//! range answered the accepted mesh is the finest; the two finest differ by
//! 1.4e-4 in displacement where the field stays elastic, 6.6e-4 at 106.26
//! MPa and 2.98e-3 at 139.81 MPa, and in the plastic dissipation by 4.7e-5 of
//! the strain energy at 106.26 MPa and 1.90e-3 at 139.81 MPa, the highest
//! traction answered. After first yield the load rises in steps of at
//! most `LARGEST_STEP_MPA`, each settled by Newton's method on the nodal
//! forces; halving the step moves the displacement by less than 1e-4 of its
//! largest (a test).
//!
//! **Held every iteration.** The largest nodal force imbalance of a free
//! node, over the largest boundary nodal force, at every Newton iteration
//! (`IMBALANCE_TOLERANCE` to settle a step). The energy ledger at every
//! settled step of every mesh: the work of the boundary forces, each step's
//! displacement against the mean of its two ends' forces, equals the strain
//! energy plus the plastic dissipation, each step's plastic strain against
//! the mean of its two ends' stresses, to `ENERGY_TOLERANCE`; it closes from
//! the three numbers served. While the field stays elastic this is
//! Clapeyron's theorem: the strain energy is the work of the forces rising
//! from zero, half the final forces times the displacement. At acceptance, the
//! force the stresses carry across five vertical cuts (the net section
//! through the hole, the edge of the square around it, the middle of the
//! parallel length, the start and the end of the fillet) equals the applied
//! force to `SECTION_FORCE_TOLERANCE`; found at most 1.09e-3, at the end of
//! the fillet. Each ledger set past what the arithmetic holds is refused by
//! name (a test).
//!
//! **Limits.** A traction that is not a pull: the strip between the clamps
//! would be a column. A traction above the yield strength: the whole section
//! yields. Past the collapse load of the net section, 141.2 MPa of gross
//! traction, the strain at the hole runs away: the largest principal strain
//! passing 1 percent, where the linear measure of strain departs from the
//! Green-Lagrange measure by half a percent of itself, is refused by name.
//! On a scan by 0.01 MPa the last traction answered is 139.81 MPa (largest
//! strain 7.84e-3); from 139.82 to 139.98 MPa the two finest meshes differ in
//! the plastic dissipation by more than `DISSIPATION_SETTLE` of the strain
//! energy, near the collapse, and the question is refused as `NotSettled`;
//! from 139.99 MPa the strain passes 1 percent. A point outside the parallel
//! length or inside the hole.
//!
//! **References, in the tests.** Kirsch's displacement imposed on a square
//! of 3 radii around the hole: the settled field meets Kirsch's to 2.6e-6 of
//! its largest displacement, his hoop stress along the edge to 2.0e-4 of
//! the peak, and his stress concentration of 3 at 2.9996. A
//! strip ten holes wide concentrates 3.035 on the gross section, against
//! 3.025 by the cubic of Peterson's Stress Concentration Factors fitted to
//! Howland's solution and 3.032 by Heywood's formula. This plate, d/W =
//! 0.399, concentrates 2.2489 on the net section, against 2.2366 and 2.2171.
//! The exact plane-stress solution of a hole pulled equally in every
//! direction, yielded out to 1.5 radii, is met to 0.11 percent of the yield
//! strength along the radius, and its yielded zone reaches its radius to
//! 0.15 mm.
//!
//! **Fitted.** Two constants, on the opened band of the laboratory's field
//! only (the points 16 mm or more from the hole along the bar), each band
//! with its own rigid motion: `YOUNGS_MODULUS_MPA` and `POISSON_RATIO`. The
//! two bands do not strain alike (each alone gives 206,500 and 230,000 MPa).
//!
//! **The comparison of 2026-10-14**, fixed beforehand
//! (`evidence/research/stress_plate_s235/validation_protocol.md`, SHA-256
//! 000657104b0a61e8c6f86935e73ac456315d3ec4fe94139f0bed258c62471880, this
//! file's numerics at commit 2752748) and run once, on the 1,186 held-back
//! points 4.5 to 12 mm from the hole's centre at 106.26 MPa: the
//! displacement line P1 was met (rms 0.675 um in u_x against 2.0, 0.444 um in
//! u_y against 1.5); the line against the textbook P2 was met (pooled rms
//! 0.571 um, against 1.818 um for a plate without a hole and 1.015 um for
//! Kirsch's infinite plate); the line of the strain 1.5 mm from the hole's
//! edge P3 was missed: its ratio to the far strain is 1.889 measured and
//! 2.380 by this solver, the measured 20.6 percent below the solver's (the
//! solver 26.0 percent above the measured) against a line of 20. By its rule the
//! comparison missed. The same field with the steel not yielding meets all
//! three (P3 within 1.4 percent): at the laboratory's load this sheet shows
//! no yielded zone at the hole where yield at 235 MPa puts one 1.53 mm deep,
//! so it likely yields well above the grade's least value. The solver keeps
//! 235 MPa, the value the grade guarantees, and states the miss. The record
//! and the result are in `evidence/research/stress_plate_s235/`.
//!
//! What a result is not. Plane stress in a sheet 3.05 mm thick about a hole
//! of radius 4 mm (t/R = 0.76): near the hole the surface the camera sees
//! may differ from the plane-stress field by a few percent. The sheet is
//! taken as uniform and the same in every direction, and the hole as
//! centred. The ledger shows the equations were solved, not that they
//! describe the steel: the comparison does.

use genesis_kernel::math;
use crate::normal::NormalOrZero;
use std::collections::HashMap;
use std::f64::consts::FRAC_PI_4;

use crate::plane_stress_fe::{self as fe, LedgerFault, Material, Mesh, PathFailure, PathNumerics, PointState, Problem, Settled};

/// Radius of the hole, mm (the drawing).
pub const HOLE_RADIUS_MM: f64 = 4.0;
/// Width of the parallel section, mm, as the laboratory measured it.
pub const WIDTH_MM: f64 = 20.05;
/// Thickness of the sheet, mm, as the laboratory measured it.
pub const THICKNESS_MM: f64 = 3.05;
/// Half the length of the parallel section, mm: it is 90 mm long (the drawing).
pub const PARALLEL_HALF_LENGTH_MM: f64 = 45.0;
/// Radius of the fillets from the parallel section to the grip ends, mm.
pub const FILLET_RADIUS_MM: f64 = 25.0;
/// How far each fillet rises, mm: the grip ends are 30 mm wide where the
/// parallel section is 20.
pub const FILLET_RISE_MM: f64 = 5.0;
/// Where the load is put on the model, mm from the hole's centre along the
/// bar: 15 mm into each grip end, as a uniform traction.
pub const LOADED_END_MM: f64 = 75.0;

/// Young's modulus, MPa, fitted to the opened band of the laboratory's
/// field (evidence/research/stress_plate_s235/analysis/02_calibrate.py).
pub const YOUNGS_MODULUS_MPA: f64 = 218_540.0;
/// Poisson's ratio, fitted with it.
pub const POISSON_RATIO: f64 = 0.2835;
/// The yield strength, MPa: the least upper yield strength EN 10025-2 asks
/// of S235 of nominal thickness up to 16 mm.
pub const YIELD_STRENGTH_MPA: f64 = 235.0;

/// The traction the laboratory applied, MPa (Anton et al., arXiv 2405.18311).
pub const MEASURED_TRACTION_MPA: f64 = 106.26;

/// The public measurement this solver is set against.
pub const REFERENCE_NAME: &str = "Tröger, Hartmann, Anton, Wessels, Digital image correlation measurement of linear elastic steel specimen, Zenodo, version 1.0 (2024): the displacement of the surface of an S235 plate with a hole at 5,475 points, at the traction of 106.26 N/mm² that Anton et al., Adv. Model. Simul. Eng. Sci. 12 (2025) 12 (arXiv 2405.18311, section 6.1), give";
pub const REFERENCE_URL: &str = "https://doi.org/10.5281/zenodo.11257192";
pub const REFERENCE_RETRIEVED: &str = "2026-10-14";

/// Elements of the coarsest mesh, per quarter of the plate: around each 45
/// degrees of the hole, out from the hole to the square around it, along the
/// rest of the parallel section, along the fillet, along the grip end.
pub const ARC_ELEMENTS: usize = 4;
pub const RADIAL_ELEMENTS: usize = 4;
pub const ALONG_ELEMENTS: usize = 8;
pub const FILLET_ELEMENTS: usize = 2;
pub const GRIP_ELEMENTS: usize = 2;
/// Ratio of the last to the first element's length out from the hole, over
/// one element, on the coarsest mesh; each finer mesh takes its square root.
pub const RADIAL_RATIO: f64 = 1.6;
/// The same along the parallel section.
pub const ALONG_RATIO: f64 = 1.2;
/// The finest mesh: each level halves every element of the one before.
pub const MAX_LEVEL: u32 = 2;

/// The path's numerical choices.
pub const LARGEST_STEP_MPA: f64 = 4.0;
pub const IMBALANCE_TOLERANCE: f64 = 1.0e-10;
pub const IMBALANCE_CAP: f64 = 1.0e3;
pub const MAX_NEWTON_ITERATIONS: u32 = 25;
pub const MAX_STEP_HALVINGS: u32 = 6;
pub const ENERGY_TOLERANCE: f64 = 1.0e-9;

/// The geometry of one mesh: a square of half-side `patch` around the hole,
/// and optionally the rest of a strip.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Layout {
    pub radius: f64,
    pub patch: f64,
    pub arc: usize,
    pub radial: usize,
    pub radial_ratio: f64,
    pub strip: Option<Strip>,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct Strip {
    pub parallel_end: f64,
    pub along: usize,
    pub along_ratio: f64,
    pub fillet_rise: f64,
    pub fillet_radius: f64,
    pub fillet: usize,
    pub grip: usize,
    pub loaded_end: f64,
}

impl Strip {
    fn fillet_length(&self) -> f64 {
        if self.fillet_rise == 0.0 {
            0.0
        } else {
            (self.fillet_radius * self.fillet_radius - math::powi(self.fillet_radius - self.fillet_rise, 2)).sqrt()
        }
    }

    /// Half the width of the strip at `x`.
    fn half_width(&self, patch: f64, x: f64) -> f64 {
        let start = self.parallel_end;
        let length = self.fillet_length();
        if x <= start || length == 0.0 {
            patch
        } else if x >= start + length {
            patch + self.fillet_rise
        } else {
            let d = x - start;
            patch + self.fillet_radius - (self.fillet_radius * self.fillet_radius - d * d).sqrt()
        }
    }
}

/// The specimen's layout at one level of refinement.
pub(crate) fn specimen_layout(level: u32) -> Layout {
    let k = 1usize << level;
    let root = 1.0 / k as f64;
    Layout {
        radius: HOLE_RADIUS_MM,
        patch: WIDTH_MM / 2.0,
        arc: ARC_ELEMENTS * k,
        radial: RADIAL_ELEMENTS * k,
        radial_ratio: math::powf(RADIAL_RATIO, root),
        strip: Some(Strip {
            parallel_end: PARALLEL_HALF_LENGTH_MM,
            along: ALONG_ELEMENTS * k,
            along_ratio: math::powf(ALONG_RATIO, root),
            fillet_rise: FILLET_RISE_MM,
            fillet_radius: FILLET_RADIUS_MM,
            fillet: FILLET_ELEMENTS * k,
            grip: GRIP_ELEMENTS * k,
            loaded_end: LOADED_END_MM,
        }),
    }
}

/// Where a graded division of `[0, 1]` into `n` elements, each `ratio`
/// times the one before, puts the point `k` (which may be a half). The two
/// ends are 0 and 1 exactly: `powf(ratio, n)` and `powi(ratio, n)` round
/// differently, so the quotient at `k = n` was a unit or two in the last
/// place from 1, and the parallel length's last node fell short of 45 mm
/// (G^G's recomputation of 16 October 2026).
fn graded(k: f64, n: usize, ratio: f64) -> f64 {
    if k == 0.0 {
        0.0
    } else if k == n as f64 {
        1.0
    } else if ratio == 1.0 {
        k / n as f64
    } else {
        (math::powf(ratio, k) - 1.0) / (math::powi(ratio, n as i32) - 1.0)
    }
}

/// A built mesh and the places on it the solver reads.
#[derive(Clone, Debug)]
pub(crate) struct Built {
    pub mesh: Mesh,
    /// Each node's place in its block's grid: block, column, row.
    pub key: Vec<(u8, u32, u32)>,
    /// Nodes on `y = 0` and on `x = 0`.
    pub bottom: Vec<usize>,
    pub left: Vec<usize>,
    /// The hole's edge, from `theta = 0` to 90 degrees.
    pub hole: Vec<usize>,
    pub top_of_hole: usize,
    /// The edges of three nodes at the loaded end, or on the patch's outer
    /// sides when there is no strip.
    pub end_edges: Vec<[usize; 3]>,
    /// Nodes on the patch's outer sides `x = patch` and `y = patch`, when
    /// there is no strip: the tests hold an exact field there.
    #[cfg_attr(not(test), allow(dead_code))]
    pub outer: Vec<usize>,
    /// Vertical cuts: their `x`, and the element edges along them, each an
    /// element with the side of it on the cut.
    pub cuts: Vec<(f64, Vec<(usize, Side)>)>,
}

/// A side of an element in natural coordinates.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Side {
    /// `xi = -1`, along `eta`.
    Left,
    /// `eta = +1`, along `xi`.
    Top,
}

pub(crate) fn build(layout: &Layout) -> Built {
    let (radius, a, m, nr) = (layout.radius, layout.patch, layout.arc, layout.radial);
    let strip = layout.strip;
    let (nb, nc, nd) = match strip {
        Some(s) => (s.along, if s.fillet_rise > 0.0 { s.fillet } else { 0 }, s.grip),
        None => (0, 0, 0),
    };
    let columns = nr + nb + nc + nd;
    let mut x = Vec::new();
    let mut y = Vec::new();
    let mut key = Vec::new();
    // Block 0: the patch's lower half and the strip. Block 1: the patch's
    // upper half.
    let width0 = 2 * columns + 1;
    let height = 2 * m + 1;
    let mut id0 = vec![usize::MAX; width0 * height];
    let ray = |theta: f64, s: f64, outer: (f64, f64)| -> (f64, f64) {
        let (ax, ay) = (radius * math::cos(theta), radius * math::sin(theta));
        (ax + s * (outer.0 - ax), ay + s * (outer.1 - ay))
    };
    for q in 0..height {
        let theta = q as f64 / (2 * m) as f64 * FRAC_PI_4;
        let slope = math::tan(theta);
        for p in 0..width0 {
            if p % 2 == 1 && q % 2 == 1 {
                continue;
            }
            let half = p as f64 / 2.0;
            let (px, py) = if p <= 2 * nr {
                let s = graded(half, nr, layout.radial_ratio);
                if q == 2 * m {
                    let s = graded(half, nr, layout.radial_ratio);
                    let d = radius * std::f64::consts::FRAC_1_SQRT_2;
                    (d + s * (a - d), d + s * (a - d))
                } else {
                    ray(theta, s, (a, a * slope))
                }
            } else {
                let strip = strip.expect("a strip");
                let k = half - nr as f64;
                let fillet_length = strip.fillet_length();
                let px = if k == nb as f64 {
                    // The parallel length ends where it is declared to.
                    strip.parallel_end
                } else if k <= nb as f64 {
                    a + (strip.parallel_end - a) * graded(k, nb, strip.along_ratio)
                } else if k <= (nb + nc) as f64 {
                    strip.parallel_end + fillet_length * (k - nb as f64) / nc as f64
                } else {
                    let start = strip.parallel_end + fillet_length;
                    start + (strip.loaded_end - start) * (k - (nb + nc) as f64) / nd as f64
                };
                let top = strip.half_width(a, px);
                (px, if q == 2 * m { top } else { slope * top })
            };
            id0[q * width0 + p] = x.len();
            x.push(px);
            y.push(if q == 0 { 0.0 } else { py });
            key.push((0, p as u32, q as u32));
        }
    }
    let width1 = 2 * nr + 1;
    let mut id1 = vec![usize::MAX; width1 * height];
    for q in 0..height {
        for p in 0..width1 {
            if p % 2 == 1 && q % 2 == 1 {
                continue;
            }
            if q == 0 {
                id1[p] = id0[p + 2 * m * width0];
                continue;
            }
            let theta = FRAC_PI_4 + q as f64 / (2 * m) as f64 * FRAC_PI_4;
            let s = graded(p as f64 / 2.0, nr, layout.radial_ratio);
            let (mut px, py) = if q == 2 * m {
                (0.0, radius + s * (a - radius))
            } else {
                ray(theta, s, (a * math::cos(theta) / math::sin(theta), a))
            };
            if q == 2 * m {
                px = 0.0;
            }
            id1[q * width1 + p] = x.len();
            x.push(px);
            y.push(py);
            key.push((1, p as u32, q as u32));
        }
    }
    let element = |ids: &[usize], width: usize, i: usize, j: usize| -> [usize; 8] {
        let at = |p: usize, q: usize| ids[q * width + p];
        let (p, q) = (2 * i, 2 * j);
        [at(p, q), at(p + 2, q), at(p + 2, q + 2), at(p, q + 2), at(p + 1, q), at(p + 2, q + 1), at(p + 1, q + 2), at(p, q + 1)]
    };
    let mut elements = Vec::new();
    let mut cuts: Vec<(f64, Vec<(usize, Side)>)> = Vec::new();
    for i in 0..columns {
        for j in 0..m {
            elements.push(element(&id0, width0, i, j));
        }
    }
    let first_upper = elements.len();
    for i in 0..nr {
        for j in 0..m {
            elements.push(element(&id1, width1, i, j));
        }
    }
    // The net section, x = 0: the upper block's top side.
    cuts.push((0.0, (0..nr).map(|i| (first_upper + i * m + (m - 1), Side::Top)).collect()));
    if let Some(strip) = strip {
        let at_column = |column: usize, cuts: &mut Vec<(f64, Vec<(usize, Side)>)>| {
            let edge: Vec<(usize, Side)> = (0..m).map(|j| (column * m + j, Side::Left)).collect();
            let node = id0[2 * column];
            cuts.push((x[node], edge));
        };
        at_column(nr, &mut cuts);
        at_column(nr + nb / 2, &mut cuts);
        at_column(nr + nb, &mut cuts);
        if nc > 0 {
            at_column(nr + nb + nc, &mut cuts);
        }
        let _ = strip;
    }
    let bottom: Vec<usize> = (0..width0).filter(|p| id0[*p] != usize::MAX).map(|p| id0[p]).collect();
    let left: Vec<usize> = (0..width1).map(|p| id1[2 * m * width1 + p]).collect();
    let mut hole: Vec<usize> = (0..height).map(|q| id0[q * width0]).collect();
    hole.extend((1..height).map(|q| id1[q * width1]));
    let top_of_hole = id1[2 * m * width1];
    let mut end_edges = Vec::new();
    let mut outer = Vec::new();
    if strip.is_some() {
        let p = 2 * columns;
        for j in 0..m {
            end_edges.push([id0[2 * j * width0 + p], id0[(2 * j + 1) * width0 + p], id0[(2 * j + 2) * width0 + p]]);
        }
    } else {
        let p = 2 * nr;
        for j in 0..m {
            end_edges.push([id0[2 * j * width0 + p], id0[(2 * j + 1) * width0 + p], id0[(2 * j + 2) * width0 + p]]);
        }
        for q in 0..height {
            outer.push(id0[q * width0 + p]);
        }
        for q in 1..height {
            outer.push(id1[q * width1 + p]);
        }
    }
    Built { mesh: Mesh { x, y, elements }, key, bottom, left, hole, top_of_hole, end_edges, outer, cuts }
}

/// The largest principal strain of a settled field, in size: past it the
/// linear measure of strain departs from the Green-Lagrange measure by more
/// than half a percent of itself.
pub const SMALL_STRAIN_LIMIT: f64 = 0.01;
/// Two successive levels are settled when the largest change of a
/// displacement at their common nodes is at most this fraction of the
/// largest displacement...
pub const DISPLACEMENT_SETTLE: f64 = 5.0e-3;
/// ... the stress at the top of the hole's edge in the elastic field
/// changes by at most this fraction of itself...
pub const STRESS_SETTLE: f64 = 1.0e-3;
/// ... and the plastic dissipation by at most this fraction of the strain
/// energy.
pub const DISSIPATION_SETTLE: f64 = 2.0e-3;
/// The force carried across every vertical cut of the settled field, from
/// its stresses, is the applied force to this fraction.
pub const SECTION_FORCE_TOLERANCE: f64 = 2.0e-3;
/// The most points one question may ask the field at.
pub const MAX_POINTS: usize = 1000;

/// One question: the traction on the gross section of the parallel length,
/// and the points at which the field is wanted, in mm from the hole's
/// centre, `x` along the bar and `y` across it.
#[derive(Clone, Debug, PartialEq)]
pub struct PlateQuestion {
    pub traction_mpa: f64,
    pub points_x_mm: Vec<f64>,
    pub points_y_mm: Vec<f64>,
}

/// Why no field was returned.
#[derive(Clone, Debug, PartialEq)]
pub enum PlateReject {
    /// An input was NaN or infinite.
    InputNotFinite,
    /// The traction is not a pull: pushed, the strip between the clamps is a column that can bow out of its plane, and the field in the plane holds only while it stays flat.
    TractionNotTensile { traction_mpa: f64 },
    /// The traction is above the yield strength: the whole section of the parallel length would yield, and a plate that does not harden carries no more.
    TractionAboveYield { traction_mpa: f64 },
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
    /// The return of a stress to the yield surface did not converge.
    YieldReturnNotSettled,
    /// No equilibrium was found past a traction, the load step halved `MAX_STEP_HALVINGS` times: near the collapse load of the net section the yielded steel carries little more.
    EquilibriumNotFound { traction_reached_mpa: f64, level: u32 },
    /// The largest principal strain passed `SMALL_STRAIN_LIMIT`, where the linear measure of strain no longer holds.
    StrainBeyondSmallStrain { traction_mpa: f64, strain: f64, level: u32 },
    /// The largest nodal force imbalance of the settled field is above `IMBALANCE_TOLERANCE`.
    EquilibriumBroken { imbalance: f64 },
    /// The work of the boundary forces departs from the strain energy plus the dissipation by more than `ENERGY_TOLERANCE` of it.
    EnergyLedgerBroken { traction_mpa: f64, residual: f64 },
    /// The force the settled field carries across a vertical cut departs from the applied force by more than `SECTION_FORCE_TOLERANCE`.
    SectionForceBroken { x_mm: f64, residual: f64 },
    /// Two successive meshes did not agree within the settling fractions by the finest, `MAX_LEVEL`.
    NotSettled { change: f64 },
}

impl PlateReject {
    pub fn name(&self) -> &'static str {
        match self {
            Self::InputNotFinite => "InputNotFinite",
            Self::TractionNotTensile { .. } => "TractionNotTensile",
            Self::TractionAboveYield { .. } => "TractionAboveYield",
            Self::PointCountsDiffer { .. } => "PointCountsDiffer",
            Self::TooManyPoints { .. } => "TooManyPoints",
            Self::PointOutsidePlate { .. } => "PointOutsidePlate",
            Self::PointInsideHole { .. } => "PointInsideHole",
            Self::NonFiniteValue => "NonFiniteValue",
            Self::MeshInverted => "MeshInverted",
            Self::SingularStiffness => "SingularStiffness",
            Self::YieldReturnNotSettled => "YieldReturnNotSettled",
            Self::EquilibriumNotFound { .. } => "EquilibriumNotFound",
            Self::StrainBeyondSmallStrain { .. } => "StrainBeyondSmallStrain",
            Self::EquilibriumBroken { .. } => "EquilibriumBroken",
            Self::EnergyLedgerBroken { .. } => "EnergyLedgerBroken",
            Self::SectionForceBroken { .. } => "SectionForceBroken",
            Self::NotSettled { .. } => "NotSettled",
        }
    }
}

impl std::fmt::Display for PlateReject {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InputNotFinite => write!(f, "an input was not finite"),
            Self::TractionNotTensile { traction_mpa } => write!(f, "traction {} MPa is not a pull: pushed, the strip between the clamps is a column that can bow out of its plane, and the field in the plane holds only while it stays flat", said(*traction_mpa)),
            Self::TractionAboveYield { traction_mpa } => write!(f, "traction {} MPa is above the yield strength, {YIELD_STRENGTH_MPA} MPa: the whole section of the parallel length would yield", said(*traction_mpa)),
            Self::PointCountsDiffer { x_count, y_count } => write!(f, "{x_count} values of x and {y_count} of y: a point is one of each"),
            Self::TooManyPoints { count } => write!(f, "{count} points, more than {MAX_POINTS}"),
            Self::PointOutsidePlate { x_mm, y_mm } => write!(f, "the point ({}, {}) mm is outside the parallel length, which runs {PARALLEL_HALF_LENGTH_MM} mm each way from the hole's centre and {} mm each way across", said(*x_mm), said(*y_mm), WIDTH_MM / 2.0),
            Self::PointInsideHole { x_mm, y_mm } => write!(f, "the point ({}, {}) mm is inside the hole, of radius {HOLE_RADIUS_MM} mm", said(*x_mm), said(*y_mm)),
            Self::NonFiniteValue => write!(f, "a value of the field was not finite"),
            Self::MeshInverted => write!(f, "an element of the mesh is turned inside out"),
            Self::SingularStiffness => write!(f, "the stiffness of the elastic field has a pivot that is not positive"),
            Self::YieldReturnNotSettled => write!(f, "the return of a stress to the yield surface did not converge"),
            Self::EquilibriumNotFound { traction_reached_mpa, level } => write!(f, "no equilibrium was found past traction {} MPa on mesh level {level}, the load step halved {MAX_STEP_HALVINGS} times: near the collapse load of the net section, {:.2} MPa on the gross section, the yielded steel carries little more", said(*traction_reached_mpa), YIELD_STRENGTH_MPA * (WIDTH_MM - 2.0 * HOLE_RADIUS_MM) / WIDTH_MM),
            Self::StrainBeyondSmallStrain { traction_mpa, strain, level } => write!(f, "at traction {} MPa on mesh level {level} the largest principal strain is {}, past {SMALL_STRAIN_LIMIT}, where the linear measure of strain no longer holds", said(*traction_mpa), said(*strain)),
            Self::EquilibriumBroken { imbalance } => write!(f, "the largest nodal force imbalance of the settled field is {}, above {IMBALANCE_TOLERANCE}", said(*imbalance)),
            Self::EnergyLedgerBroken { traction_mpa, residual } => write!(f, "at traction {} MPa the work of the boundary forces departs from the strain energy plus the plastic dissipation by {} of it, above {ENERGY_TOLERANCE}", said(*traction_mpa), said(*residual)),
            Self::SectionForceBroken { x_mm, residual } => write!(f, "the force carried across the cut at x = {} mm departs from the applied force by {} of it, above {SECTION_FORCE_TOLERANCE}", said(*x_mm), said(*residual)),
            Self::NotSettled { change } => write!(f, "the two finest meshes, level {} and {MAX_LEVEL}, differ by {} times the settling fraction of the measure that differs most", MAX_LEVEL - 1, said(*change)),
        }
    }
}

/// A number in a sentence: the shortest decimal that reads back to the same
/// 64-bit number, written with an exponent when it would otherwise run past
/// 24 characters.
fn said(value: f64) -> String {
    let plain = format!("{value}");
    if plain.len() > 24 { format!("{value:e}") } else { plain }
}

/// `value` times two to the power `exponent`, rounded once: every factor
/// but the last is exact.
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
    // Down past 2^-1074 in steps that leave the value a normal number, so
    // exact; when one would not, what is left is below every subnormal.
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
/// in MPa times two to this power, and so are its stresses and its
/// displacement, so that no product of the arithmetic leaves the range of
/// 64-bit numbers whatever the traction. A power of two is exact both ways.
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

impl std::error::Error for PlateReject {}

/// The field at one asked point.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PointField {
    pub displacement_x_mm: f64,
    pub displacement_y_mm: f64,
    pub stress_xx_mpa: f64,
    pub stress_yy_mpa: f64,
    pub stress_xy_mpa: f64,
    pub von_mises_mpa: f64,
    pub strain_xx: f64,
    pub strain_yy: f64,
    /// The tensor shear strain, half the engineering shear.
    pub strain_xy: f64,
}

impl PointField {
    /// The point's field, solved in the units of `unit`, in MPa and mm.
    fn in_mpa(self, unit: i32) -> PointField {
        let down = |value: f64| times_two_to(value, -unit);
        PointField {
            displacement_x_mm: down(self.displacement_x_mm),
            displacement_y_mm: down(self.displacement_y_mm),
            stress_xx_mpa: down(self.stress_xx_mpa),
            stress_yy_mpa: down(self.stress_yy_mpa),
            stress_xy_mpa: down(self.stress_xy_mpa),
            von_mises_mpa: down(self.von_mises_mpa),
            strain_xx: down(self.strain_xx),
            strain_yy: down(self.strain_yy),
            strain_xy: down(self.strain_xy),
        }
    }
}

/// A settled field and what is read from it.
#[derive(Clone, Debug, PartialEq)]
pub struct PlateField {
    pub traction_mpa: f64,
    pub axial_force_n: f64,
    pub net_section_stress_mpa: f64,
    /// The stress along the bar at the top of the hole's edge, where the
    /// net section meets it.
    pub hole_edge_stress_mpa: f64,
    /// The same in the elastic field of this traction: what it would be if
    /// the steel did not yield.
    pub elastic_hole_edge_stress_mpa: f64,
    pub stress_concentration_gross: f64,
    pub stress_concentration_net: f64,
    /// The traction at which the solve's yielding begins: the elastic
    /// field's largest von Mises stress over the integration points reaches
    /// the yield strength there.
    pub first_yield_traction_mpa: f64,
    pub peak_von_mises_mpa: f64,
    /// The area of the plate's face that has yielded, all four quarters.
    pub plastic_zone_area_mm2: f64,
    /// The largest distance from the hole's edge of a point that has yielded.
    pub plastic_zone_reach_mm: f64,
    pub largest_equivalent_plastic_strain: f64,
    pub largest_strain: f64,
    /// The change of length of the parallel section along its axis.
    pub parallel_section_elongation_mm: f64,
    pub strain_energy_j: f64,
    pub plastic_dissipation_j: f64,
    pub boundary_work_j: f64,
    pub points: Vec<PointField>,
    /// The ledger.
    pub equilibrium_residual: f64,
    pub energy_residual: f64,
    pub section_force_residual: f64,
    /// The largest traction on the hole's edge, normal or shear, over the
    /// largest stress along it.
    pub hole_traction_residual: f64,
    /// The mesh level accepted, its elements per quarter, and the Newton
    /// iterations over every level.
    pub level: u32,
    pub elements: usize,
    pub newton_iterations: u32,
    /// The settling measures between the accepted level and the one before.
    pub displacement_change: f64,
    pub stress_change: f64,
    pub dissipation_change: f64,
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
    pub dissipation_settle: f64,
    pub section_force_tolerance: f64,
    pub small_strain_limit: f64,
    pub youngs_modulus: f64,
    pub poisson_ratio: f64,
    pub yield_strength: Option<f64>,
    pub loaded_end: f64,
    pub largest_step_mpa: f64,
    /// Solve in MPa times the power of two that carries the traction into
    /// [1, 2) (`unit_exponent`); the tests hold it to the unscaled solve.
    pub scaled_units: bool,
}

pub(crate) const SERVED: Numerics = Numerics {
    path: PathNumerics {
        largest_step: f64::NAN,
        imbalance_tolerance: IMBALANCE_TOLERANCE,
        imbalance_cap: IMBALANCE_CAP,
        max_newton_iterations: MAX_NEWTON_ITERATIONS,
        max_step_halvings: MAX_STEP_HALVINGS,
        energy_tolerance: ENERGY_TOLERANCE,
        fault: LedgerFault::None,
    },
    max_level: MAX_LEVEL,
    displacement_settle: DISPLACEMENT_SETTLE,
    stress_settle: STRESS_SETTLE,
    dissipation_settle: DISSIPATION_SETTLE,
    section_force_tolerance: SECTION_FORCE_TOLERANCE,
    small_strain_limit: SMALL_STRAIN_LIMIT,
    youngs_modulus: YOUNGS_MODULUS_MPA,
    poisson_ratio: POISSON_RATIO,
    yield_strength: Some(YIELD_STRENGTH_MPA),
    loaded_end: LOADED_END_MM,
    largest_step_mpa: LARGEST_STEP_MPA,
    scaled_units: true,
};

/// The limits the question itself shows, in the order the solver asks them.
pub fn admit(question: &PlateQuestion) -> Result<(), PlateReject> {
    let t = question.traction_mpa;
    if !t.is_finite() || question.points_x_mm.iter().chain(&question.points_y_mm).any(|v| !v.is_finite()) {
        return Err(PlateReject::InputNotFinite);
    }
    if !(t > 0.0) {
        return Err(PlateReject::TractionNotTensile { traction_mpa: t });
    }
    if t > YIELD_STRENGTH_MPA {
        return Err(PlateReject::TractionAboveYield { traction_mpa: t });
    }
    let (nx, ny) = (question.points_x_mm.len(), question.points_y_mm.len());
    if nx != ny {
        return Err(PlateReject::PointCountsDiffer { x_count: nx, y_count: ny });
    }
    if nx > MAX_POINTS {
        return Err(PlateReject::TooManyPoints { count: nx });
    }
    for (&x, &y) in question.points_x_mm.iter().zip(&question.points_y_mm) {
        if x.abs() > PARALLEL_HALF_LENGTH_MM || y.abs() > WIDTH_MM / 2.0 {
            return Err(PlateReject::PointOutsidePlate { x_mm: x, y_mm: y });
        }
        if x * x + y * y < HOLE_RADIUS_MM * HOLE_RADIUS_MM {
            return Err(PlateReject::PointInsideHole { x_mm: x, y_mm: y });
        }
    }
    Ok(())
}

/// Settles the field at one traction and reads it at the asked points, or
/// names the limit that stopped it.
///
/// Each value of the answer below the normal range is written as zero
/// with its sign ([`crate::normal`]).
pub fn solve(question: &PlateQuestion) -> Result<PlateField, PlateReject> {
    crate::normal::held(solve_as_computed(question))
}

/// The answer as the equations compute it, before a value below the
/// normal range is written as zero.
fn solve_as_computed(question: &PlateQuestion) -> Result<PlateField, PlateReject> {
    solve_with(question, &SERVED)
}

impl NormalOrZero for PlateField {
    fn normal_or_zero(&mut self) {
        self.traction_mpa.normal_or_zero();
        self.axial_force_n.normal_or_zero();
        self.net_section_stress_mpa.normal_or_zero();
        self.hole_edge_stress_mpa.normal_or_zero();
        self.elastic_hole_edge_stress_mpa.normal_or_zero();
        self.stress_concentration_gross.normal_or_zero();
        self.stress_concentration_net.normal_or_zero();
        self.first_yield_traction_mpa.normal_or_zero();
        self.peak_von_mises_mpa.normal_or_zero();
        self.plastic_zone_area_mm2.normal_or_zero();
        self.plastic_zone_reach_mm.normal_or_zero();
        self.largest_equivalent_plastic_strain.normal_or_zero();
        self.largest_strain.normal_or_zero();
        self.parallel_section_elongation_mm.normal_or_zero();
        self.strain_energy_j.normal_or_zero();
        self.plastic_dissipation_j.normal_or_zero();
        self.boundary_work_j.normal_or_zero();
        self.points.normal_or_zero();
        self.equilibrium_residual.normal_or_zero();
        self.energy_residual.normal_or_zero();
        self.section_force_residual.normal_or_zero();
        self.hole_traction_residual.normal_or_zero();
        self.displacement_change.normal_or_zero();
        self.stress_change.normal_or_zero();
        self.dissipation_change.normal_or_zero();
        self.nodes.normal_or_zero();
    }
}

/// One level's outcome.
enum LevelOutcome {
    Settled(Box<(Built, Settled)>),
    Strain { traction: f64, strain: f64 },
    Collapse { traction: f64 },
}

/// One level's load path. `material` is in the units of `unit` (its yield
/// strength scaled with the traction), and so is the field it settles.
fn run_level(level: u32, traction: f64, unit: i32, numerics: &Numerics, material: &Material) -> Result<LevelOutcome, PlateReject> {
    let mut layout = specimen_layout(level);
    if let Some(strip) = layout.strip.as_mut() {
        strip.loaded_end = numerics.loaded_end;
    }
    let built = build(&layout);
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
    let mut path = numerics.path;
    path.largest_step = numerics.largest_step_mpa / traction;
    let limit = times_two_to(numerics.small_strain_limit, unit);
    let stop = |_: f64, _: &[f64], points: &[PointState]| -> Option<f64> {
        let strain = fe::largest_point_strain(material, points);
        (strain > limit).then_some(strain)
    };
    match fe::settle(&problem, material, &path, &stop) {
        Ok(settled) => Ok(LevelOutcome::Settled(Box::new((built, settled)))),
        Err(PathFailure::Stopped { load_factor, value }) => Ok(LevelOutcome::Strain { traction: load_factor * traction, strain: times_two_to(value, -unit) }),
        Err(PathFailure::NoEquilibrium { load_factor, .. }) => Ok(LevelOutcome::Collapse { traction: load_factor * traction }),
        Err(PathFailure::Mesh(_)) => Err(PlateReject::MeshInverted),
        Err(PathFailure::NotFinite) => Err(PlateReject::NonFiniteValue),
        Err(PathFailure::ElasticSingular { .. }) => Err(PlateReject::SingularStiffness),
        Err(PathFailure::ReturnNotConverged) => Err(PlateReject::YieldReturnNotSettled),
        Err(PathFailure::EnergyLedger { load_factor, residual }) => Err(PlateReject::EnergyLedgerBroken { traction_mpa: load_factor * traction, residual }),
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

fn elastic_point_stresses(mesh: &Mesh, material: &Material, u: &[f64]) -> Result<[Vec<f64>; 3], PlateReject> {
    let geometry = fe::point_geometry(mesh).map_err(|_| PlateReject::MeshInverted)?;
    let hooke = material.hooke();
    let mut out = [Vec::new(), Vec::new(), Vec::new()];
    for (index, element) in mesh.elements.iter().enumerate() {
        for p in 0..fe::POINTS_PER_ELEMENT {
            let g = &geometry[index * fe::POINTS_PER_ELEMENT + p];
            let mut e = [0.0; 3];
            for k in 0..8 {
                let (ux, uy) = (u[2 * element[k]], u[2 * element[k] + 1]);
                e[0] += g.dx[k] * ux;
                e[1] += g.dy[k] * uy;
                e[2] += g.dy[k] * ux + g.dx[k] * uy;
            }
            for c in 0..3 {
                out[c].push(hooke[c][0] * e[0] + hooke[c][1] * e[1] + hooke[c][2] * e[2]);
            }
        }
    }
    Ok(out)
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

fn solve_with(question: &PlateQuestion, numerics: &Numerics) -> Result<PlateField, PlateReject> {
    admit(question)?;
    let unit = unit_of(question.traction_mpa, numerics);
    let material = material_of(numerics, unit);
    let ladder = settle_ladder(question.traction_mpa, unit, numerics, &material)?;
    read_field(question, numerics, &material, ladder, unit)
}

fn unit_of(traction: f64, numerics: &Numerics) -> i32 {
    if numerics.scaled_units { unit_exponent(traction) } else { 0 }
}

/// The steel in the units of `unit`: Young's modulus and Poisson's ratio as
/// they are, the yield strength scaled with the traction (infinite where the
/// scaling carries it past the largest 64-bit number, which only a traction
/// far below yield does).
fn material_of(numerics: &Numerics, unit: i32) -> Material {
    Material {
        youngs_modulus: numerics.youngs_modulus,
        poisson_ratio: numerics.poisson_ratio,
        yield_strength: numerics.yield_strength.map(|strength| times_two_to(strength, unit)),
    }
}

/// The accepted level of the ladder.
struct Ladder {
    level: u32,
    built: Built,
    settled: Settled,
    newton_iterations: u32,
    measures: (f64, f64, f64),
}

/// Settles the field on meshes of rising level until two successive ones
/// agree, or names the limit that stopped it.
fn settle_ladder(traction: f64, unit: i32, numerics: &Numerics, material: &Material) -> Result<Ladder, PlateReject> {
    let mut previous: Option<LevelOutcome> = None;
    let mut newton_iterations = 0_u32;
    let mut last_change = f64::NAN;
    for level in 0..=numerics.max_level {
        let outcome = run_level(level, traction, unit, numerics, material)?;
        let last = level == numerics.max_level;
        match (&previous, &outcome) {
            (Some(LevelOutcome::Settled(coarse)), LevelOutcome::Settled(fine)) => {
                newton_iterations += fine.1.newton_iterations;
                let (change, measures) = settle_measures(&coarse.0, &coarse.1, &fine.0, &fine.1, material)?;
                last_change = change;
                if measures.0 <= numerics.displacement_settle && measures.1 <= numerics.stress_settle && measures.2 <= numerics.dissipation_settle {
                    let LevelOutcome::Settled(fine) = outcome else { unreachable!() };
                    let (built, settled) = *fine;
                    return Ok(Ladder { level, built, settled, newton_iterations, measures });
                }
            }
            (_, LevelOutcome::Settled(fine)) => newton_iterations += fine.1.newton_iterations,
            // A limit is named when the finest mesh reaches it, or two
            // successive meshes do.
            (previous, LevelOutcome::Strain { traction: reached, strain }) if last || matches!(previous, Some(LevelOutcome::Strain { .. })) => {
                return Err(PlateReject::StrainBeyondSmallStrain { traction_mpa: *reached, strain: *strain, level });
            }
            (previous, LevelOutcome::Collapse { traction: reached }) if last || matches!(previous, Some(LevelOutcome::Collapse { .. })) => {
                return Err(PlateReject::EquilibriumNotFound { traction_reached_mpa: *reached, level });
            }
            _ => {}
        }
        previous = Some(outcome);
    }
    Err(PlateReject::NotSettled { change: last_change })
}

/// The field at any number of points of the plate, with Young's modulus and
/// Poisson's ratio of the caller's and the steel yielding or not: for the
/// record's programs (the calibration on the opened band, and the scoring),
/// never served. A point outside the plate or inside the hole is refused as
/// in [`admit`]; the count of points is not limited.
#[doc(hidden)]
pub fn probe_with_moduli(traction_mpa: f64, youngs_modulus_mpa: f64, poisson_ratio: f64, yields: bool, points: &[(f64, f64)]) -> Result<(PlateField, Vec<PointField>), PlateReject> {
    let numerics = Numerics {
        youngs_modulus: youngs_modulus_mpa,
        poisson_ratio,
        yield_strength: if yields { Some(YIELD_STRENGTH_MPA) } else { None },
        ..SERVED
    };
    let question = PlateQuestion { traction_mpa, points_x_mm: vec![], points_y_mm: vec![] };
    admit(&question)?;
    for &(x, y) in points {
        admit(&PlateQuestion { traction_mpa, points_x_mm: vec![x], points_y_mm: vec![y] })?;
    }
    let unit = unit_of(traction_mpa, &numerics);
    let material = material_of(&numerics, unit);
    let ladder = settle_ladder(traction_mpa, unit, &numerics, &material)?;
    let stresses = point_stresses(&ladder.settled);
    let mut out = Vec::with_capacity(points.len());
    for &(x, y) in points {
        out.push(field_at(&ladder.built, &ladder.settled, &stresses, x, y)?.in_mpa(unit));
    }
    let field = read_field(&question, &numerics, &material, ladder, unit)?;
    Ok((field, out))
}

/// The settling measures between two levels: the change of displacement at
/// their common nodes of the parallel section over its largest
/// displacement, of the hole's edge stress in the elastic field over
/// itself, and of the plastic dissipation over the strain energy. The first
/// number is the largest of the three over its settling fraction.
fn settle_measures(coarse: &Built, coarse_field: &Settled, fine: &Built, fine_field: &Settled, material: &Material) -> Result<(f64, (f64, f64, f64)), PlateReject> {
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
    let edge = |built: &Built, field: &Settled| -> Result<f64, PlateReject> {
        let e = elastic_point_stresses(&built.mesh, material, &field.elastic_displacement)?;
        Ok(top_of_hole_value(built, &e[0]))
    };
    let (c_elastic, f_elastic) = (edge(coarse, coarse_field)?, edge(fine, fine_field)?);
    let stress = (c_elastic - f_elastic).abs() / f_elastic.abs();
    let dissipation = (coarse_field.plastic_dissipation - fine_field.plastic_dissipation).abs() / fine_field.strain_energy;
    let measures = (displacement, stress, dissipation);
    if !(displacement.is_finite() && stress.is_finite() && dissipation.is_finite()) {
        return Err(PlateReject::NonFiniteValue);
    }
    let change = (displacement / DISPLACEMENT_SETTLE).max(stress / STRESS_SETTLE).max(dissipation / DISSIPATION_SETTLE);
    Ok((change, measures))
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
fn read_field(question: &PlateQuestion, numerics: &Numerics, material: &Material, ladder: Ladder, unit: i32) -> Result<PlateField, PlateReject> {
    let Ladder { level, built, settled, newton_iterations, measures } = ladder;
    let traction = question.traction_mpa;
    let scaled_traction = times_two_to(traction, unit);
    let down = |value: f64| times_two_to(value, -unit);
    let down_twice = |value: f64| times_two_to(value, -2 * unit);
    let mesh = &built.mesh;
    if !(settled.imbalance <= numerics.path.imbalance_tolerance) {
        return Err(PlateReject::EquilibriumBroken { imbalance: settled.imbalance });
    }
    let geometry = fe::point_geometry(mesh).map_err(|_| PlateReject::MeshInverted)?;
    let stresses = point_stresses(&settled);
    let elastic = elastic_point_stresses(mesh, material, &settled.elastic_displacement)?;
    // The ledger of the cuts.
    let quarter_force = scaled_traction * WIDTH_MM / 2.0;
    let mut section_force_residual = 0.0_f64;
    for (x, force) in cut_forces(&built, &stresses) {
        let residual = (force / quarter_force - 1.0).abs();
        if !residual.is_finite() {
            return Err(PlateReject::NonFiniteValue);
        }
        if residual > numerics.section_force_tolerance {
            return Err(PlateReject::SectionForceBroken { x_mm: x, residual });
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
    let elastic_edge = top_of_hole_value(&built, &elastic[0]);
    let net = traction * WIDTH_MM / (WIDTH_MM - 2.0 * HOLE_RADIUS_MM);
    let scaled_net = scaled_traction * WIDTH_MM / (WIDTH_MM - 2.0 * HOLE_RADIUS_MM);
    let mut peak = 0.0_f64;
    let mut area = 0.0;
    let mut reach = 0.0_f64;
    let mut largest_plastic = 0.0_f64;
    for (g, p) in geometry.iter().zip(&settled.points) {
        peak = peak.max(fe::von_mises(&p.stress));
        if p.equivalent_plastic_strain > 0.0 {
            area += g.weight;
            reach = reach.max((g.x * g.x + g.y * g.y).sqrt() - HOLE_RADIUS_MM);
            largest_plastic = largest_plastic.max(p.equivalent_plastic_strain);
        }
    }
    let largest_strain = fe::largest_point_strain(material, &settled.points);
    let end_node = built
        .key
        .iter()
        .position(|k| *k == (0, (2 * (RADIAL_ELEMENTS + ALONG_ELEMENTS) << level) as u32, 0))
        .expect("the node at the end of the parallel section");
    let elongation = 2.0 * settled.displacement[2 * end_node];
    let to_joules = 4.0 * THICKNESS_MM * 1.0e-3;
    let mut points = Vec::with_capacity(question.points_x_mm.len());
    for (&x, &y) in question.points_x_mm.iter().zip(&question.points_y_mm) {
        points.push(field_at(&built, &settled, &stresses, x, y)?.in_mpa(unit));
    }
    let mut nodes = Vec::with_capacity(mesh.x.len() * 7);
    for n in 0..mesh.x.len() {
        nodes.extend_from_slice(&[mesh.x[n], mesh.y[n], down(settled.displacement[2 * n]), down(settled.displacement[2 * n + 1]), down(nodal[0][n]), down(nodal[1][n]), down(nodal[2][n])]);
    }
    let field = PlateField {
        traction_mpa: traction,
        axial_force_n: traction * WIDTH_MM * THICKNESS_MM,
        net_section_stress_mpa: net,
        hole_edge_stress_mpa: down(hole_edge),
        elastic_hole_edge_stress_mpa: down(elastic_edge),
        stress_concentration_gross: elastic_edge / scaled_traction,
        stress_concentration_net: elastic_edge / scaled_net,
        // Where the solve's own yielding begins: the elastic field's largest
        // von Mises stress over the integration points reaches the yield
        // strength, the same peak the load path scales to.
        first_yield_traction_mpa: YIELD_STRENGTH_MPA * (scaled_traction / settled.elastic_peak_von_mises),
        peak_von_mises_mpa: down(peak),
        plastic_zone_area_mm2: 4.0 * area,
        plastic_zone_reach_mm: reach,
        largest_equivalent_plastic_strain: down(largest_plastic),
        largest_strain: down(largest_strain),
        parallel_section_elongation_mm: down(elongation),
        strain_energy_j: down_twice(settled.strain_energy * to_joules),
        plastic_dissipation_j: down_twice(settled.plastic_dissipation * to_joules),
        boundary_work_j: down_twice(settled.boundary_work * to_joules),
        points,
        equilibrium_residual: settled.imbalance,
        energy_residual: settled.energy_residual,
        section_force_residual,
        hole_traction_residual,
        level,
        elements: mesh.elements.len(),
        newton_iterations,
        displacement_change: measures.0,
        stress_change: measures.1,
        dissipation_change: measures.2,
        nodes,
    };
    let numbers = [
        field.hole_edge_stress_mpa,
        field.elastic_hole_edge_stress_mpa,
        field.peak_von_mises_mpa,
        field.plastic_zone_area_mm2,
        field.plastic_zone_reach_mm,
        field.largest_strain,
        field.parallel_section_elongation_mm,
        field.strain_energy_j,
        field.plastic_dissipation_j,
        field.hole_traction_residual,
    ];
    if numbers.iter().chain(&field.nodes).any(|v| !v.is_finite()) {
        return Err(PlateReject::NonFiniteValue);
    }
    Ok(field)
}

/// The element of the parallel length that holds a point of the quarter, and
/// its natural coordinates. Every point the solver answers at lies in the
/// parallel length, so it is read from an element of it, every node at
/// `PARALLEL_HALF_LENGTH_MM` or less: a point at the end, x = 45 mm, lies on
/// the side the last parallel element shares with the shoulder fillet's
/// first, and was read from the fillet's (G^G's recomputation of 16 October
/// 2026: 122.557 MPa where the parallel section gives 118.703 at 106.26 MPa).
pub(crate) fn locate_in_parallel_length(mesh: &Mesh, x: f64, y: f64) -> Option<(usize, f64, f64)> {
    fe::locate_among(mesh, x, y, |element| element.iter().all(|&n| mesh.x[n] <= PARALLEL_HALF_LENGTH_MM))
}

/// The field at a point of the whole plate, from the quarter by its two
/// mirror planes.
fn field_at(built: &Built, settled: &Settled, stresses: &[Vec<f64>; 3], x: f64, y: f64) -> Result<PointField, PlateReject> {
    let (sx, sy) = (if x < 0.0 { -1.0 } else { 1.0 }, if y < 0.0 { -1.0 } else { 1.0 });
    let (qx, qy) = (x.abs(), y.abs());
    let mesh = &built.mesh;
    let (element, xi, eta) = locate_in_parallel_length(mesh, qx, qy).ok_or(PlateReject::PointOutsidePlate { x_mm: x, y_mm: y })?;
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

    fn ask(traction: f64, xs: &[f64], ys: &[f64]) -> PlateQuestion {
        PlateQuestion { traction_mpa: traction, points_x_mm: xs.to_vec(), points_y_mm: ys.to_vec() }
    }

    fn served_material(yields: bool) -> Material {
        Material { youngs_modulus: YOUNGS_MODULUS_MPA, poisson_ratio: POISSON_RATIO, yield_strength: yields.then_some(YIELD_STRENGTH_MPA) }
    }

    const PATH: PathNumerics = PathNumerics {
        largest_step: 0.02,
        imbalance_tolerance: IMBALANCE_TOLERANCE,
        imbalance_cap: IMBALANCE_CAP,
        max_newton_iterations: MAX_NEWTON_ITERATIONS,
        max_step_halvings: MAX_STEP_HALVINGS,
        energy_tolerance: ENERGY_TOLERANCE,
        fault: LedgerFault::None,
    };

    /// The mesh reaches its own ends: graded(0, n, r) is 0 and graded(n, n, r)
    /// is 1 exactly, so the parallel length's last node is at 45 mm, the end
    /// the solver declares, at every level of refinement (G^G's
    /// recomputation of 16 October 2026; it was 1 to 2 units in the last
    /// place short). A point at either end of the parallel length is read
    /// from the parallel section's element, as the points just inside it are.
    #[test]
    fn the_mesh_reaches_the_end_of_the_parallel_length() {
        for level in 0..3 {
            let layout = specimen_layout(level);
            let strip = layout.strip.unwrap();
            for (n, ratio) in [(layout.radial, layout.radial_ratio), (strip.along, strip.along_ratio)] {
                assert_eq!(graded(0.0, n, ratio).to_bits(), 0.0_f64.to_bits(), "{level} {n} {ratio}");
                assert_eq!(graded(n as f64, n, ratio).to_bits(), 1.0_f64.to_bits(), "{level} {n} {ratio}");
            }
            let built = build(&layout);
            let end = built.key.iter().zip(&built.mesh.x).filter(|(key, _)| key.0 == 0 && key.1 as usize == 2 * (layout.radial + strip.along)).map(|(_, x)| *x);
            let mut count = 0;
            for x in end {
                assert_eq!(x.to_bits(), PARALLEL_HALF_LENGTH_MM.to_bits(), "{level}: {x}");
                count += 1;
            }
            assert_eq!(count, 2 * layout.arc + 1, "{level}");
        }
        let inside = 44.999999999999;
        let (xs, ys): (Vec<f64>, Vec<f64>) = [-10.025, 10.025].iter().flat_map(|&y| [45.0, -45.0, inside, -inside].map(|x| (x, y))).unzip();
        let field = solve(&ask(100.0, &xs, &ys)).expect("settles");
        for (k, chunk) in field.points.chunks(4).enumerate() {
            for (end, near) in [(&chunk[0], &chunk[2]), (&chunk[1], &chunk[3])] {
                let at = (xs[4 * k], ys[4 * k]);
                assert!((end.stress_xx_mpa / near.stress_xx_mpa - 1.0).abs() < 1.0e-9, "{at:?}: {} {}", end.stress_xx_mpa, near.stress_xx_mpa);
                assert!((end.stress_yy_mpa - near.stress_yy_mpa).abs() < 1.0e-9, "{at:?}: {} {}", end.stress_yy_mpa, near.stress_yy_mpa);
            }
        }
    }

    /// The worked question, settled once for the tests that read it.
    fn measured() -> &'static PlateField {
        static FIELD: std::sync::OnceLock<PlateField> = std::sync::OnceLock::new();
        FIELD.get_or_init(|| solve(&ask(MEASURED_TRACTION_MPA, &[0.0, 0.0, 4.5, -20.0, 20.0], &[4.0, 10.025, 0.0, 5.0, -5.0])).expect("settles"))
    }

    /// Kirsch's plate: an infinite plane-stress plate with a hole of radius
    /// `a`, pulled by `s` along `x`. The displacement, up to a rigid motion.
    fn kirsch_displacement(x: f64, y: f64, s: f64, e: f64, nu: f64, a: f64) -> (f64, f64) {
        let r = x.hypot(y);
        let t = y.atan2(x);
        let mu = e / (2.0 * (1.0 + nu));
        let kappa = (3.0 - nu) / (1.0 + nu);
        let (c2, s2) = ((2.0 * t).cos(), (2.0 * t).sin());
        let a2 = a * a;
        let ur = s / (4.0 * mu) * (r * ((kappa - 1.0) / 2.0 + c2) + a2 / r * (1.0 + (1.0 + kappa) * c2) - a2 * a2 / r.powi(3) * c2);
        let ut = s / (4.0 * mu) * ((1.0 - kappa) * a2 / r - r - a2 * a2 / r.powi(3)) * s2;
        (ur * t.cos() - ut * t.sin(), ur * t.sin() + ut * t.cos())
    }

    /// Kirsch's hoop stress on the hole's edge: `s (1 - 2 cos 2 theta)`.
    fn kirsch_edge_hoop(theta: f64, s: f64) -> f64 {
        s * (1.0 - 2.0 * (2.0 * theta).cos())
    }

    fn hole_patch(radius: f64, patch: f64, arc: usize, radial: usize, radial_ratio: f64) -> Built {
        build(&Layout { radius, patch, arc, radial, radial_ratio, strip: None })
    }

    /// Kirsch's solution as its own reference: its displacement imposed on
    /// the sides of a square of 3 radii around the hole, the field inside is
    /// settled and held to it. The hoop stress at the top of the hole is 3
    /// times the pull, the stress concentration of the infinite plate.
    #[test]
    fn kirsch_inside_a_square_of_its_own_displacement() {
        let (s, e, nu, a) = (100.0, YOUNGS_MODULUS_MPA, POISSON_RATIO, HOLE_RADIUS_MM);
        let built = hole_patch(a, 3.0 * a, 16, 16, RADIAL_RATIO.powf(0.25));
        let mesh = &built.mesh;
        let mut held: Vec<(usize, f64)> = built.bottom.iter().map(|&n| (2 * n + 1, 0.0)).collect();
        held.extend(built.left.iter().map(|&n| (2 * n, 0.0)));
        for &n in &built.outer {
            let (ux, uy) = kirsch_displacement(mesh.x[n], mesh.y[n], s, e, nu, a);
            held.push((2 * n, ux));
            held.push((2 * n + 1, uy));
        }
        let problem = Problem { mesh, held, load: vec![0.0; mesh.x.len() * 2] };
        let settled = fe::settle(&problem, &served_material(false), &PATH, &|_, _, _| None).expect("settles");
        let stresses = point_stresses(&settled);
        let top = top_of_hole_value(&built, &stresses[0]);
        assert!((top / s - 3.0).abs() < 5e-4, "{}", top / s);
        let nodal: Vec<Vec<f64>> = stresses.iter().map(|v| fe::nodal_average(mesh, v)).collect();
        let mut worst = 0.0_f64;
        for &n in &built.hole {
            let theta = mesh.y[n].atan2(mesh.x[n]);
            let (c, si) = (theta.cos(), theta.sin());
            let hoop = nodal[0][n] * si * si + nodal[1][n] * c * c - 2.0 * nodal[2][n] * si * c;
            worst = worst.max((hoop - kirsch_edge_hoop(theta, s)).abs());
        }
        assert!(worst / (3.0 * s) < 2e-3, "{}", worst / (3.0 * s));
        let mut largest = 0.0_f64;
        let mut error = 0.0_f64;
        for n in 0..mesh.x.len() {
            let (ux, uy) = kirsch_displacement(mesh.x[n], mesh.y[n], s, e, nu, a);
            largest = largest.max(ux.abs()).max(uy.abs());
            error = error.max((settled.displacement[2 * n] - ux).abs()).max((settled.displacement[2 * n + 1] - uy).abs());
        }
        assert!(error / largest < 1e-5, "{}", error / largest);
        // Clapeyron: the strain energy is half the work of the boundary forces.
        assert!(settled.energy_residual < 1e-12, "{}", settled.energy_residual);
    }

    /// The finite width raises the concentration above Kirsch's 3. On a
    /// strip ten holes wide (d/W = 0.1): the cubic fit of Peterson's Stress
    /// Concentration Factors to Howland's solution,
    /// K_tn = 3.00 - 3.13 (d/W) + 3.66 (d/W)^2 - 1.53 (d/W)^3, gives 3.0246 on
    /// the gross section, and Heywood's formula, K_tn = 2 + (1 - d/W)^3,
    /// gives 3.0321. This strip gives 3.0351.
    #[test]
    fn a_strip_ten_holes_wide_concentrates_a_little_above_three() {
        let layout = Layout {
            radius: HOLE_RADIUS_MM,
            patch: 40.0,
            arc: 16,
            radial: 24,
            radial_ratio: 1.17,
            strip: Some(Strip { parallel_end: 200.0, along: 16, along_ratio: 1.15, fillet_rise: 0.0, fillet_radius: 0.0, fillet: 0, grip: 2, loaded_end: 240.0 }),
        };
        let built = build(&layout);
        let mesh = &built.mesh;
        let mut load = vec![0.0; mesh.x.len() * 2];
        for edge in &built.end_edges {
            fe::edge_traction(mesh, *edge, [100.0, 0.0], &mut load);
        }
        let mut held: Vec<(usize, f64)> = built.bottom.iter().map(|&n| (2 * n + 1, 0.0)).collect();
        held.extend(built.left.iter().map(|&n| (2 * n, 0.0)));
        let problem = Problem { mesh, held, load };
        let settled = fe::settle(&problem, &served_material(false), &PATH, &|_, _, _| None).expect("settles");
        let gross = top_of_hole_value(&built, &point_stresses(&settled)[0]) / 100.0;
        assert!(gross > 3.0, "{gross}");
        assert!((gross / 3.0246 - 1.0).abs() < 5e-3, "{gross}");
        assert!((gross / 3.0321 - 1.0).abs() < 5e-3, "{gross}");
    }

    /// The specimen, d/W = 0.399: the same cubic gives K_tn = 2.2366, and
    /// Heywood's formula 2 + (1 - d/W)^3 gives 2.2171. The plate's own,
    /// with its fillets and its finite length, is 2.2489.
    #[test]
    fn the_specimen_concentrates_as_howland_and_heywood_say() {
        let field = solve(&ask(50.0, &[], &[])).expect("settles");
        let d_over_w = 2.0 * HOLE_RADIUS_MM / WIDTH_MM;
        let howland = 3.00 - 3.13 * d_over_w + 3.66 * d_over_w.powi(2) - 1.53 * d_over_w.powi(3);
        let heywood = 2.0 + (1.0 - d_over_w).powi(3);
        assert!((field.stress_concentration_net / howland - 1.0).abs() < 1e-2, "{} {howland}", field.stress_concentration_net);
        assert!((field.stress_concentration_net / heywood - 1.0).abs() < 2e-2, "{} {heywood}", field.stress_concentration_net);
        assert!((field.stress_concentration_net - 2.2489).abs() < 5e-4, "{}", field.stress_concentration_net);
        assert!((howland - 2.2366).abs() < 5e-5 && (heywood - 2.2171).abs() < 5e-5, "{howland} {heywood}");
        // The two finest meshes, elastic: 1.4e-4 in displacement.
        assert!((field.displacement_change - 1.4e-4).abs() < 0.05e-4, "{}", field.displacement_change);
        assert_eq!(field.plastic_zone_area_mm2, 0.0);
        assert!((field.hole_edge_stress_mpa - field.elastic_hole_edge_stress_mpa).abs() < 1e-9);
    }

    /// The plate with a hole pulled equally in every direction, yielding
    /// around the hole: the exact solution of plane-stress von Mises
    /// plasticity. In the yielded ring `a <= r <= c` the stresses are fixed by
    /// equilibrium and the yield condition alone: with
    /// `sigma_r = (2/sqrt 3) Y sin w` and `sigma_theta = (2/sqrt 3) Y sin(w + pi/3)`,
    /// `dw / d ln r = cos(w + pi/6) / cos w` from `w = 0` at the free edge.
    /// Outside, the elastic `S -/+ B / r^2` meets them at `c`. The
    /// displacement of that elastic field is imposed on the sides of a square
    /// of 5 radii, and the settled field is held to the exact one.
    #[test]
    fn the_yielded_ring_around_a_hole_pulled_every_way_is_the_exact_one() {
        let (a, c, y) = (HOLE_RADIUS_MM, 6.0, YIELD_STRENGTH_MPA);
        let big = 2.0 * y / 3.0_f64.sqrt();
        let exact_w = |r: f64| -> f64 {
            let steps = 4000;
            let h = (r / a).ln() / steps as f64;
            let f = |w: f64| (w + std::f64::consts::FRAC_PI_6).cos() / w.cos();
            let mut w = 0.0;
            for _ in 0..steps {
                let k1 = f(w);
                let k2 = f(w + 0.5 * h * k1);
                let k3 = f(w + 0.5 * h * k2);
                let k4 = f(w + h * k3);
                w += h / 6.0 * (k1 + 2.0 * k2 + 2.0 * k3 + k4);
            }
            w
        };
        let wc = exact_w(c);
        let (sr_c, st_c) = (big * wc.sin(), big * (wc + std::f64::consts::FRAC_PI_3).sin());
        let s = 0.5 * (sr_c + st_c);
        let b = 0.5 * c * c * (st_c - sr_c);
        let exact = |r: f64| -> (f64, f64) {
            if r <= c {
                let w = exact_w(r);
                (big * w.sin(), big * (w + std::f64::consts::FRAC_PI_3).sin())
            } else {
                (s - b / (r * r), s + b / (r * r))
            }
        };
        let material = served_material(true);
        let (e, nu) = (material.youngs_modulus, material.poisson_ratio);
        let built = hole_patch(a, 5.0 * a, 16, 24, 1.12);
        let mesh = &built.mesh;
        let mut held: Vec<(usize, f64)> = built.bottom.iter().map(|&n| (2 * n + 1, 0.0)).collect();
        held.extend(built.left.iter().map(|&n| (2 * n, 0.0)));
        for &n in &built.outer {
            let r = mesh.x[n].hypot(mesh.y[n]);
            let ur = (s * (1.0 - nu) * r + b * (1.0 + nu) / r) / e;
            held.push((2 * n, ur * mesh.x[n] / r));
            held.push((2 * n + 1, ur * mesh.y[n] / r));
        }
        let problem = Problem { mesh, held, load: vec![0.0; mesh.x.len() * 2] };
        let settled = fe::settle(&problem, &material, &PATH, &|_, _, _| None).expect("settles");
        let stresses = point_stresses(&settled);
        let nodal: Vec<Vec<f64>> = stresses.iter().map(|v| fe::nodal_average(mesh, v)).collect();
        let mut worst = 0.0_f64;
        let mut worst_away = 0.0_f64;
        for &n in &built.bottom {
            let r = mesh.x[n];
            let (sr, st) = exact(r);
            let error = (nodal[0][n] - sr).abs().max((nodal[1][n] - st).abs()) / y;
            worst = worst.max(error);
            if (r - c).abs() > 0.5 {
                worst_away = worst_away.max(error);
            }
        }
        println!("equibiaxial: S {s} B {b} worst {worst} away from c {worst_away}");
        assert!(worst < 1e-2, "{worst}");
        assert!(worst_away < 3e-3, "{worst_away}");
        // The yielded ring reaches c, to the spacing of the integration points.
        let geometry = fe::point_geometry(mesh).unwrap();
        let reach = geometry.iter().zip(&settled.points).filter(|(_, p)| p.equivalent_plastic_strain > 0.0).map(|(g, _)| g.x.hypot(g.y)).fold(0.0_f64, f64::max);
        assert!((reach - c).abs() < 0.15, "{reach}");
        // The energy ledger closed at every step of the path.
        assert!(settled.steps.len() > 10);
        assert!(settled.steps.iter().all(|step| step.energy_residual <= ENERGY_TOLERANCE));
        assert!(settled.plastic_dissipation > 0.0);
    }

    #[test]
    fn the_ledger_closes_at_the_worked_question() {
        let field = measured();
        assert!(field.equilibrium_residual <= IMBALANCE_TOLERANCE, "{}", field.equilibrium_residual);
        assert!(field.energy_residual <= ENERGY_TOLERANCE, "{}", field.energy_residual);
        assert!(field.section_force_residual <= SECTION_FORCE_TOLERANCE, "{}", field.section_force_residual);
        assert_eq!(field.level, MAX_LEVEL);
        // The work of the boundary forces is the strain energy plus the
        // dissipation, from the three numbers served.
        let closure = (field.boundary_work_j - field.strain_energy_j - field.plastic_dissipation_j) / field.boundary_work_j;
        assert!(closure.abs() < 1e-12, "{closure}");
        assert!(field.plastic_dissipation_j > 0.0);
        // The edge of the hole has yielded: its stress is the yield strength,
        // to the reach of the recovery of stress at the nodes.
        assert!((field.hole_edge_stress_mpa / YIELD_STRENGTH_MPA - 1.0).abs() < 5e-3, "{}", field.hole_edge_stress_mpa);
        assert!(field.peak_von_mises_mpa <= YIELD_STRENGTH_MPA * (1.0 + 1e-12));
        assert!(field.elastic_hole_edge_stress_mpa > 390.0 && field.elastic_hole_edge_stress_mpa < 400.0);
        assert!((field.first_yield_traction_mpa - 63.494).abs() < 1e-3, "{}", field.first_yield_traction_mpa);
        assert!(field.plastic_zone_reach_mm > 1.0 && field.plastic_zone_reach_mm < 2.0, "{}", field.plastic_zone_reach_mm);
        // The force on the gross section.
        assert!((field.axial_force_n - 106.26 * 20.05 * 3.05).abs() < 1e-9);
    }

    /// A ledger set past what the arithmetic holds is named, never passed.
    #[test]
    fn each_ledger_broken_on_purpose_is_named() {
        let q = ask(MEASURED_TRACTION_MPA, &[], &[]);
        let energy = Numerics { path: PathNumerics { energy_tolerance: 1e-18, ..SERVED.path }, ..SERVED };
        assert!(matches!(solve_with(&q, &energy), Err(PlateReject::EnergyLedgerBroken { .. })));
        // A move the ledger does not book: the work of each step against its
        // end forces alone (the rectangle, twice the strain energy while
        // elastic), and the dissipation of one yielding point left out.
        for fault in [LedgerFault::RectangleWork, LedgerFault::UnbookedDissipation] {
            let broken = Numerics { path: PathNumerics { fault, ..SERVED.path }, ..SERVED };
            assert!(matches!(solve_with(&q, &broken), Err(PlateReject::EnergyLedgerBroken { .. })), "{fault:?}");
        }
        let rectangle = Numerics { path: PathNumerics { fault: LedgerFault::RectangleWork, ..SERVED.path }, ..SERVED };
        match solve_with(&ask(30.0, &[], &[]), &rectangle) {
            Err(PlateReject::EnergyLedgerBroken { residual, .. }) => assert!((residual - 0.5).abs() < 1e-9, "{residual}"),
            other => panic!("{:?}", other.map(|f| f.energy_residual)),
        }
        let cut = Numerics { section_force_tolerance: 1e-6, ..SERVED };
        assert!(matches!(solve_with(&q, &cut), Err(PlateReject::SectionForceBroken { .. })));
        // In an elastic field the equilibrium is a direct solve, held after it.
        let elastic = ask(30.0, &[], &[]);
        let tight = Numerics { path: PathNumerics { imbalance_tolerance: 1e-20, ..SERVED.path }, ..SERVED };
        assert!(matches!(solve_with(&elastic, &tight), Err(PlateReject::EquilibriumBroken { .. })));
        // A plastic step that cannot settle is named, after its halvings.
        let short = Numerics { path: PathNumerics { max_newton_iterations: 1, ..SERVED.path }, ..SERVED };
        assert!(matches!(solve_with(&q, &short), Err(PlateReject::EquilibriumNotFound { .. })));
        // A ladder too short to settle is named.
        let one = Numerics { max_level: 1, ..SERVED };
        assert!(matches!(solve_with(&q, &one), Err(PlateReject::NotSettled { .. })));
        let strict = Numerics { displacement_settle: 1e-9, ..SERVED };
        assert!(matches!(solve_with(&q, &strict), Err(PlateReject::NotSettled { .. })));
        // The small-strain limit, set low, is reached at the worked question.
        let small = Numerics { small_strain_limit: 1e-3, ..SERVED };
        assert!(matches!(solve_with(&q, &small), Err(PlateReject::StrainBeyondSmallStrain { .. })));
    }

    /// Past the collapse load of the net section the strain runs away and
    /// the small-strain limit is named; the last traction answered is below it.
    #[test]
    fn past_the_net_section_collapse_load_the_strain_limit_is_named() {
        let collapse = YIELD_STRENGTH_MPA * (WIDTH_MM - 2.0 * HOLE_RADIUS_MM) / WIDTH_MM;
        for t in [141.0, 150.0, YIELD_STRENGTH_MPA] {
            match solve(&ask(t, &[], &[])) {
                Err(PlateReject::StrainBeyondSmallStrain { traction_mpa, strain, .. }) => {
                    assert!(traction_mpa < collapse * 1.01 && traction_mpa > collapse * 0.97, "{t}: {traction_mpa}");
                    assert!(strain > SMALL_STRAIN_LIMIT);
                }
                other => panic!("{t}: {:?}", other.map(|f| f.largest_strain)),
            }
        }
    }

    /// The upper end of what is answered, heavy in an unoptimised build:
    /// `cargo test --release -- --ignored`. 139.81 MPa is the last traction
    /// answered on a 0.01 MPa scan, its two finest meshes 1.90e-3 apart in the
    /// dissipation over the strain energy and 2.98e-3 in the displacement;
    /// from 139.82 to 139.98 MPa they differ by more than the settling
    /// fraction, NotSettled; from 139.99 MPa the strain passes 1 percent.
    #[test]
    #[ignore]
    fn the_last_tractions_answered_and_the_first_refused() {
        let field = solve(&ask(139.81, &[], &[])).expect("settles");
        assert_eq!(field.level, MAX_LEVEL);
        assert!(field.largest_strain < SMALL_STRAIN_LIMIT && field.largest_strain > 7.8e-3, "{}", field.largest_strain);
        assert!((field.dissipation_change - 1.896e-3).abs() < 1e-5, "{}", field.dissipation_change);
        assert!((field.displacement_change - 2.983e-3).abs() < 1e-5, "{}", field.displacement_change);
        for t in [139.82, 139.9, 139.98] {
            assert!(matches!(solve(&ask(t, &[], &[])), Err(PlateReject::NotSettled { .. })), "{t}");
        }
        for t in [139.99, 140.0] {
            assert!(matches!(solve(&ask(t, &[], &[])), Err(PlateReject::StrainBeyondSmallStrain { level: 2, .. })), "{t}");
        }
    }

    /// The field is solved in MPa times a power of two that carries the
    /// traction into [1, 2): at every traction the unscaled arithmetic holds,
    /// the bits are the unscaled solve's.
    #[test]
    fn the_scaled_units_give_the_unscaled_bits() {
        let plain = Numerics { scaled_units: false, ..SERVED };
        let q = ask(MEASURED_TRACTION_MPA, &[0.0, 0.0, 4.5, -20.0, 20.0], &[4.0, 10.025, 0.0, 5.0, -5.0]);
        assert_eq!(&solve_with(&q, &plain).expect("settles"), measured());
        let elastic = ask(50.0, &[10.0, -30.0], &[5.0, 2.0]);
        assert_eq!(solve_with(&elastic, &plain).expect("settles"), solve(&elastic).expect("settles"));
    }

    /// The smallest tractions are answered, their ledger closed: the field
    /// is linear there, and solved in scaled units it is the same field
    /// whatever the power of two, each value of it scaled back once, and
    /// one that scaling takes under the normal range answered as zero
    /// (`crate::normal`; through engine 0.13.0's first candidate it was
    /// answered as the subnormal number).
    #[test]
    fn the_smallest_tractions_are_answered_with_their_ledger_closed() {
        let one = solve(&ask(1.0, &[20.0], &[5.0])).expect("settles");
        let scaled = |value: f64, power: i32| crate::normal::normal_or_zero(times_two_to(value, power));
        for power in [-500, -1000, -1022] {
            let t = times_two_to(1.0, power);
            let small = solve(&ask(t, &[20.0], &[5.0])).expect("settles");
            assert_eq!(small.energy_residual, one.energy_residual, "{power}");
            assert_eq!(small.stress_concentration_gross, one.stress_concentration_gross, "{power}");
            assert_eq!(small.first_yield_traction_mpa, one.first_yield_traction_mpa, "{power}");
            assert_eq!(small.points[0].displacement_x_mm.to_bits(), scaled(one.points[0].displacement_x_mm, power).to_bits(), "{power}");
            assert_eq!(small.strain_energy_j.to_bits(), scaled(one.strain_energy_j, 2 * power).to_bits(), "{power}");
        }
        // The tractions that once came back NonFiniteValue and EnergyLedgerBroken.
        let fifty = solve(&ask(50.0, &[20.0], &[5.0])).expect("settles");
        for t in [1.0e-155, 1.0e-158, 1.0e-200, 2.2e-308, f64::MIN_POSITIVE] {
            let small = solve(&ask(t, &[20.0], &[5.0])).expect("settles");
            assert!(small.energy_residual <= ENERGY_TOLERANCE, "{t:e}");
            assert!((small.stress_concentration_gross / fifty.stress_concentration_gross - 1.0).abs() < 1e-13, "{t:e}");
            let stress = small.points[0].stress_xx_mpa;
            if stress == 0.0 {
                // Under the normal range as computed: answered as zero.
                assert!((t * fifty.points[0].stress_xx_mpa / 50.0).abs() < f64::MIN_POSITIVE, "{t:e}");
            } else {
                assert!((stress / t - fifty.points[0].stress_xx_mpa / 50.0).abs() < 1e-12, "{t:e}");
            }
        }
    }

    /// The red team's traction of engine 0.13.0 (D1), as on the elastic
    /// companion: at 1e-154 MPa the strain energy and the boundary work come
    /// to 2.06e-313 J as computed and are answered as zero; at 1e-300 MPa and
    /// at the smallest normal traction the field's values under the normal
    /// range are. Every number of the field is normal or zero, and every
    /// other number is the computed one bit for bit.
    #[test]
    fn a_value_under_the_smallest_normal_number_is_answered_as_zero() {
        use crate::normal::NormalOrZero;
        for traction in [1.0e-154, 1.0e-300, f64::MIN_POSITIVE] {
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
        }
        let computed = solve_as_computed(&ask(1.0e-154, &[0.0, 4.5], &[10.025, 0.0])).expect("settles");
        assert!(computed.strain_energy_j.is_subnormal() && computed.boundary_work_j.is_subnormal());
    }

    /// Two to a power, rounded once, against what the format holds.
    #[test]
    fn a_power_of_two_is_carried_exactly() {
        assert_eq!(times_two_to(1.5, 10), 1536.0);
        assert_eq!(times_two_to(1.0, -1074), f64::from_bits(1));
        assert_eq!(times_two_to(1.0, -1075), 0.0);
        assert_eq!(times_two_to(1.5, -1074), f64::from_bits(2));
        assert_eq!(times_two_to(3.0, -2148), 0.0);
        assert_eq!(times_two_to(f64::from_bits(1), 1074), 1.0);
        assert_eq!(times_two_to(times_two_to(1.25, 1000), -1000), 1.25);
        assert_eq!(times_two_to(235.0, 1100), f64::INFINITY);
        for t in [5e-324, 2.2e-308, 1e-200, 0.7, 1.0, 106.26, 235.0] {
            let k = unit_exponent(t);
            let scaled = times_two_to(t, k);
            assert!((1.0..2.0).contains(&scaled), "{t:e}");
            assert_eq!(times_two_to(scaled, -k), t, "{t:e}");
        }
    }

    /// Where the solve's yielding begins is the traction it serves as first
    /// yield: the integration points' peak, 63.494 MPa; the edge's own
    /// stress, carried to its node, reaches the yield strength at 62.80 MPa.
    #[test]
    fn first_yield_is_where_the_solve_yields() {
        let elastic = solve(&ask(50.0, &[], &[])).expect("settles");
        let first = elastic.first_yield_traction_mpa;
        assert!((first - 63.494).abs() < 1e-3, "{first}");
        let edge = YIELD_STRENGTH_MPA * 50.0 / elastic.elastic_hole_edge_stress_mpa;
        assert!((edge - 62.801).abs() < 1e-3, "{edge}");
        let below = solve(&ask(first * (1.0 - 1e-9), &[], &[])).expect("settles");
        assert_eq!(below.plastic_zone_area_mm2, 0.0);
        assert_eq!(below.newton_iterations, 0);
        let above = solve(&ask(first * (1.0 + 1e-6), &[], &[])).expect("settles");
        assert!(above.plastic_zone_area_mm2 > 0.0);
        assert!((above.first_yield_traction_mpa / first - 1.0).abs() < 1e-13);
        let at_edge = solve(&ask(62.9, &[], &[])).expect("settles");
        assert_eq!(at_edge.plastic_zone_area_mm2, 0.0);
    }

    /// A number in a sentence is the shortest decimal that reads back, with
    /// an exponent where the plain form is long.
    #[test]
    fn a_sentence_prints_a_number_short() {
        let text = PlateReject::TractionNotTensile { traction_mpa: -1e300 }.to_string();
        assert!(text.starts_with("traction -1e300 MPa is not a pull"), "{text}");
        let text = PlateReject::PointOutsidePlate { x_mm: 1e-200, y_mm: 2.5e20 }.to_string();
        assert!(text.contains("(1e-200, 250000000000000000000) mm"), "{text}");
        assert!(PlateReject::NotSettled { change: 1.2397221085110006 }.to_string().contains("by 1.2397221085110006 times"));
    }

    #[test]
    fn the_question_s_own_limits_are_named_before_any_field() {
        let t = MEASURED_TRACTION_MPA;
        for bad in [0.0, -1.0, -0.0] {
            assert!(matches!(admit(&ask(bad, &[], &[])), Err(PlateReject::TractionNotTensile { .. })), "{bad}");
        }
        assert!(admit(&ask(f64::from_bits(1), &[], &[])).is_ok());
        assert!(admit(&ask(YIELD_STRENGTH_MPA, &[], &[])).is_ok());
        assert!(matches!(admit(&ask(f64::from_bits(YIELD_STRENGTH_MPA.to_bits() + 1), &[], &[])), Err(PlateReject::TractionAboveYield { .. })));
        for bad in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            assert_eq!(admit(&ask(bad, &[], &[])), Err(PlateReject::InputNotFinite));
            assert_eq!(admit(&ask(t, &[bad], &[0.0])), Err(PlateReject::InputNotFinite));
            assert_eq!(admit(&ask(t, &[10.0], &[bad])), Err(PlateReject::InputNotFinite));
        }
        assert!(matches!(admit(&ask(t, &[1.0, 2.0], &[8.0])), Err(PlateReject::PointCountsDiffer { x_count: 2, y_count: 1 })));
        let many = vec![20.0; MAX_POINTS + 1];
        assert!(matches!(admit(&ask(t, &many, &many.iter().map(|_| 0.0).collect::<Vec<_>>())), Err(PlateReject::TooManyPoints { .. })));
        assert!(admit(&ask(t, &[PARALLEL_HALF_LENGTH_MM, -PARALLEL_HALF_LENGTH_MM, 0.0, 0.0, 4.0], &[WIDTH_MM / 2.0, -WIDTH_MM / 2.0, 4.0, -4.0, 0.0])).is_ok());
        for (x, y) in [(45.00000000000001, 0.0), (0.0, 10.025000000000002), (-46.0, 0.0), (0.0, -11.0)] {
            assert!(matches!(admit(&ask(t, &[x], &[y])), Err(PlateReject::PointOutsidePlate { .. })), "{x} {y}");
        }
        for (x, y) in [(0.0, 0.0), (0.0, 3.9999999999999996), (2.8, 2.8), (-3.0, -2.0)] {
            assert!(matches!(admit(&ask(t, &[x], &[y])), Err(PlateReject::PointInsideHole { .. })), "{x} {y}");
        }
        let sentence = PlateReject::TractionNotTensile { traction_mpa: -5.0 }.to_string();
        assert!(sentence.starts_with("traction -5 MPa is not a pull"));
    }

    /// The field at a point mirrors across both planes of the plate.
    #[test]
    fn the_field_mirrors_across_both_planes() {
        let field = measured();
        let (a, b) = (field.points[3], field.points[4]);
        // (-20, 5) and (20, -5) are the same point turned half round.
        assert!((a.displacement_x_mm + b.displacement_x_mm).abs() < 1e-15);
        assert!((a.displacement_y_mm + b.displacement_y_mm).abs() < 1e-15);
        assert_eq!(a.stress_xx_mpa, b.stress_xx_mpa);
        assert_eq!(a.stress_xy_mpa, b.stress_xy_mpa);
        // The top of the hole carries the edge stress; the side of the net
        // section is in pure tension along the bar.
        let top = field.points[0];
        assert!((top.stress_xx_mpa - field.hole_edge_stress_mpa).abs() < 1e-9);
        assert_eq!(top.displacement_x_mm, 0.0);
        let side = field.points[1];
        assert!(side.stress_yy_mpa.abs() < 2.0 && side.stress_xx_mpa > MEASURED_TRACTION_MPA * 0.6);
        // Beside the hole on the axis the hoop stress is a compression.
        assert!(field.points[2].stress_yy_mpa < -80.0);
    }

    /// Below first yield the field is linear in the traction.
    #[test]
    fn below_first_yield_the_field_is_linear() {
        let a = solve(&ask(20.0, &[10.0], &[5.0])).expect("settles");
        let b = solve(&ask(40.0, &[10.0], &[5.0])).expect("settles");
        assert!((b.points[0].displacement_x_mm / a.points[0].displacement_x_mm - 2.0).abs() < 1e-12);
        assert!((b.hole_edge_stress_mpa / a.hole_edge_stress_mpa - 2.0).abs() < 1e-12);
        assert!((a.stress_concentration_gross - b.stress_concentration_gross).abs() < 1e-12);
        assert!(a.first_yield_traction_mpa > 40.0);
        assert_eq!(a.newton_iterations, 0);
    }

    /// The load step after first yield is a numerical choice: halving it
    /// moves the field by far less than the settling fractions.
    #[test]
    fn halving_the_load_step_moves_the_field_by_little() {
        let q = ask(MEASURED_TRACTION_MPA, &[0.0, 5.0, -20.0], &[5.5, 6.0, 5.0]);
        let half = Numerics { largest_step_mpa: LARGEST_STEP_MPA / 2.0, ..SERVED };
        let a = measured();
        let a = solve(&q).map(|f| (f, a.clone())).expect("settles").0;
        let b = solve_with(&q, &half).expect("settles");
        let largest = a.nodes.chunks(7).fold(0.0_f64, |m, n| m.max(n[2].abs()).max(n[3].abs()));
        let moved = a.nodes.chunks(7).zip(b.nodes.chunks(7)).fold(0.0_f64, |m, (p, q)| m.max((p[2] - q[2]).abs()).max((p[3] - q[3]).abs()));
        assert!(moved / largest < 1e-4, "{}", moved / largest);
        assert!((a.plastic_dissipation_j / b.plastic_dissipation_j - 1.0).abs() < 1e-3);
    }

    /// Saint-Venant: moving the place the load goes in from 15 mm into the
    /// grip end to 45 mm moves the field of the parallel length by little:
    /// at most 0.05 MPa of stress (found 0.023 MPa at 40 mm from the hole's
    /// centre, 5 mm short of the fillet) and 3e-6 mm of displacement.
    #[test]
    fn where_the_load_goes_in_does_not_reach_the_parallel_length() {
        let q = ask(MEASURED_TRACTION_MPA, &[0.0, 40.0, -44.0], &[5.5, 9.0, -3.0]);
        // Its grip end is three times as long in the same elements, so its
        // own cut at the fillet's end is not held here.
        let far = Numerics { loaded_end: 105.0, section_force_tolerance: 1.0, ..SERVED };
        let a = solve(&q).expect("settles");
        let b = solve_with(&q, &far).expect("settles");
        for (p, q) in a.points.iter().zip(&b.points) {
            assert!((p.displacement_x_mm - q.displacement_x_mm).abs() < 1e-4 * 0.03, "{p:?} {q:?}");
            assert!((p.stress_xx_mpa - q.stress_xx_mpa).abs() < 0.05, "{p:?} {q:?}");
        }
        assert!((a.hole_edge_stress_mpa - b.hole_edge_stress_mpa).abs() < 1e-6 * a.hole_edge_stress_mpa, "{} {}", a.hole_edge_stress_mpa, b.hole_edge_stress_mpa);
    }

    #[test]
    fn the_same_question_gives_the_same_bits() {
        let q = ask(MEASURED_TRACTION_MPA, &[0.0, 0.0, 4.5, -20.0, 20.0], &[4.0, 10.025, 0.0, 5.0, -5.0]);
        let again = solve(&q).expect("settles");
        assert_eq!(&again, measured());
        assert_eq!(again.nodes.len(), 7 * 4033);
    }

    /// The measured points of the two windows of line P3 of the comparison
    /// (`evidence/research/stress_plate_s235/validation_protocol.md`):
    /// held-back points within 1.5 mm of (0, 5.5) and (0, -5.5) mm from the
    /// hole's centre, as the screen kept them, each with its measured u_x in
    /// mm. From Tröger, Hartmann, Anton, Wessels, Zenodo
    /// 10.5281/zenodo.11257192, CC BY 4.0, file
    /// 20231116_displacements_raw.csv, retrieved 2026-10-14; the positions
    /// are moved to the hole's centre found in the record's registration,
    /// (39.756875, 10.275) mm of the camera's frame. 22 points above the hole,
    /// then 18 below.
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
    /// slopes of u_x along the bar, measured and this solver's
    /// (`analysis/out/03_scored.json`, over 3,269 points).
    const FAR_STRAIN_MEASURED: f64 = 0.0004868755195684971;
    const FAR_STRAIN_SOLVER: f64 = 0.00048469007311628264;
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
        // Cramer's rule for the slope.
        let det = |m: &[[f64; 3]; 3]| m[0][0] * (m[1][1] * m[2][2] - m[1][2] * m[2][1]) - m[0][1] * (m[1][0] * m[2][2] - m[1][2] * m[2][0]) + m[0][2] * (m[1][0] * m[2][1] - m[1][1] * m[2][0]);
        let mut m1 = n;
        for i in 0..3 {
            m1[i][1] = b[i];
        }
        det(&m1) / det(&n)
    }

    /// Line P3 of the comparison, held here: the strain 1.5 mm from the
    /// hole's edge over the far strain is 1.889 measured and 2.380 by this
    /// solver, the measured 20.6 percent below the solver's (the solver 26.0
    /// percent above the measured), against a line of 20: missed. The field
    /// with the steel not yielding is 1.4 percent from the measured.
    #[test]
    fn the_comparison_s_strain_near_the_hole_missed_by_its_line() {
        let (top, bottom) = WINDOW_POINTS.split_at(22);
        let measured = 0.5 * (window_slope(top, 5.5) + window_slope(bottom, -5.5)) / FAR_STRAIN_MEASURED;
        assert!((measured - 1.8894827826341272).abs() < 1e-9, "{measured}");
        let xs: Vec<f64> = WINDOW_POINTS.iter().map(|p| p.0).collect();
        let ys: Vec<f64> = WINDOW_POINTS.iter().map(|p| p.1).collect();
        let at = |field: &PlateField| -> f64 {
            let with: Vec<(f64, f64, f64)> = WINDOW_POINTS.iter().zip(&field.points).map(|(p, f)| (p.0, p.1, f.displacement_x_mm)).collect();
            let (top, bottom) = with.split_at(22);
            0.5 * (window_slope(top, 5.5) + window_slope(bottom, -5.5))
        };
        let served = solve(&ask(MEASURED_TRACTION_MPA, &xs, &ys)).expect("settles");
        let solver = at(&served) / FAR_STRAIN_SOLVER;
        assert!((solver - 2.379840177321265).abs() < 1e-6, "{solver}");
        let miss = measured / solver - 1.0;
        assert!(miss < -0.20 && miss > -0.21, "{miss}");
        assert!((solver / measured - 1.0 - 0.260).abs() < 5e-4, "{}", solver / measured - 1.0);
        let elastic = Numerics { yield_strength: None, ..SERVED };
        let unyielding = solve_with(&ask(MEASURED_TRACTION_MPA, &xs, &ys), &elastic).expect("settles");
        let ratio = at(&unyielding) / FAR_STRAIN_ELASTIC;
        assert!((measured / ratio - 1.0).abs() < 0.02, "{ratio}");
    }

}
