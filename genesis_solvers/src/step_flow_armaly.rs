//! Steady laminar flow of air over the backward-facing step of one rig.
//!
//! The rig is the channel of Armaly, Durst, Pereira and Schönung (J. Fluid
//! Mech. 127 (1983) 473): an inlet channel 5.2 mm high opens over a step
//! 4.9 mm high into a channel 10.1 mm high. The flow separates at the edge
//! of the step and reattaches to the floor further down.
//!
//! The solver settles the steady two-dimensional incompressible
//! Navier-Stokes flow in that geometry at one Reynolds number, as a stream
//! function `psi` and a vorticity `omega` on a fixed grid of nodes:
//!
//! ```text
//! u = d(psi)/dy,  v = -d(psi)/dx,  omega = dv/dx - du/dy
//! laplacian(psi) + omega = 0
//! nu * laplacian(omega) - (u * d(omega)/dx + v * d(omega)/dy) = 0
//! ```
//!
//! with second-order central differences, and Newton's method on the whole
//! field with a direct banded solve. The Reynolds number is `V D / nu`, `V`
//! the mean inlet speed (two-thirds of the largest) and `D` twice the inlet
//! height. That is the paper's own definition (page 478): "Re = VD/ν, where V
//! is two-thirds of the measured maximum inlet velocity ... D = 2h". Lengths
//! are in millimetres and `V` is one.
//!
//! **Nothing is fitted to a measurement.** The grid was fixed by comparing
//! the model with itself on finer and coarser grids: halving every spacing
//! moves the reattachment length by 0.8 percent at Reynolds number 100 and
//! 0.4 percent at 389. Against an independent solver on finer grids and
//! against Erturk's published table the reattachment length is 1.3 to 1.5
//! percent low at Reynolds number 100 and within 0.6 percent from 200 up.
//!
//! **What the checks are.** A stream function conserves mass by its
//! construction, so no check of mass is made: it would be arithmetic. The
//! vorticity equation is the momentum balance with the pressure taken out.
//! Its largest imbalance over every node is computed on every Newton
//! iteration, and the solver stops by name if it is not finite, if it grows
//! past a cap, or if it has not fallen below the tolerance within the
//! iteration cap. These checks say the equations were solved. They do not
//! say the equations describe the air: the comparison with measurement does.
//!
//! **The comparison of 2026-10-09**, made once through the gateway with the
//! build of this file's first commit. On the held-back velocity profiles at
//! Reynolds number 389 (174 circles at nine stations, as the frozen digitiser
//! read them) all three lines fixed beforehand were met: root mean square
//! difference 0.0437 of the largest inlet speed against a line of 0.05, worst
//! station 0.052 against 0.08, and 0.24 of the Reynolds-100 flow's difference
//! against 0.5. Reading the circles contributes about 0.005. The data's own
//! floor is about 0.03: the circles scatter about a smooth curve and do not
//! conserve volume flux from station to station (0.90 to 1.12 of the exact
//! flux). The solver's 0.043 is above that floor, with systematic station
//! means of +0.037 at 3.57 step heights, -0.043 at 6.12 and -0.031 at 7.76.
//! The three lines are met for any Reynolds number from about 350 to 445, a
//! reattachment length from 7.5 to 8.8 step heights: they reject the other
//! reading of the Reynolds number and the Reynolds-100 flow, and cannot tell a
//! reattachment at 8.07 from one at 8.6.
//!
//! Not blind: the reattachment length is 2.83 step heights at Reynolds
//! number 100 against the sentence of NASA Technical Memorandum 106248,
//! "approximately 3.3", 14 percent low, a miss of the 10 percent mark; and
//! 8.07 at 389 against its "approximately 8".
//!
//! **The second scoring of 2026-10-09** (`evidence/verification/second_party/V-38`,
//! same machine) reached the same three verdicts on its own digitisation, 187
//! circles: 0.0430, 0.0514 and 0.238. Of the builder's 174 circles 7 are
//! false and 20 were missed. Its independent solver agrees with this one
//! within 1.3 percent in reattachment length. It read the paper, which is
//! open, and Armaly's own Figure 14, after the fact and not blind (reading
//! error about 0.05 in length and 3 in Reynolds number): 3.01 step heights at
//! Reynolds number 97, 4.00 at 149, 6.60 at 298 and 8.60 at 387. This solver
//! is 5.8 percent low, 3.2 percent low, 0.4 percent high and 6.5 percent low
//! there, all inside the 10 percent mark. So the miss at 100 is a miss of the
//! memorandum's sentence, and against Armaly's plot the solver is 6 to 7
//! percent low at the top of its range, where the paper reports "small
//! deviations from two-dimensionality" between 7 and 12 step heights. It
//! agrees with the measured reattachment within 1 percent only near
//! Reynolds number 300.
//!
//! **The far stations at Reynolds number 100, 2026-10-10.** The 0.6.0
//! solver served stations out to 30 step heights at every Reynolds number,
//! though the profiles compared ended at 7.76 at 100 and at 13.57 at 389.
//! Armaly's own Figure 15 has measured profiles at Reynolds number 100 at
//! 12.04, 16.33, 20.92, 30.31 and 44.90 step heights. Its circles at the
//! first four were read by a program from the 300 dots-per-inch scan of the
//! paper, first-hand and digitised, and scored by the protocol's three lines,
//! opened and not blind, the third taken against the solver at Reynolds
//! number 389, and the speeds over the largest inlet speed of the parabola
//! fitted to the circles on the step: 80 circles, root mean square difference
//! 0.019 against 0.05, worst station 0.023 against 0.08, and 0.43 of the
//! Reynolds-389 flow's 0.044 against 0.5. All three were met. The flow there
//! is the developed parabola; the parabola itself is 0.019 from the circles
//! and the solver at 259.33 is 0.023, so this shows that the solver relaxes
//! to it, not more. The circles at 30.31 were scored against the profile at
//! 30, which differs from the one at 20.92 by less than 1e-6. Above 100 no
//! profile past 13.57 step heights was compared (Armaly's Figure 16 at 389
//! ends there); a station past it is answered, out to 30 step heights, where
//! the grid ends, at every Reynolds number. Through engine 0.8.0 it was
//! refused by name.
//!
//! **Below Reynolds number 100**, the lower measured condition, the same
//! equations are settled with Newton's method started at the asked Reynolds
//! number, down to creeping flow: the viscosity `2 h / Re` stays finite for
//! any Reynolds number above zero, and a Reynolds number of zero or less is
//! refused by name. Nothing below 100 was compared with measurement.
//!
//! **The top, Reynolds number 400**, is where the flow over this step stops
//! being two-dimensional. Armaly and his colleagues, page 482: "At Reynolds
//! numbers smaller than 400 the flow maintained its two-dimensionality", as
//! their Figure 8 shows at 397, and from 400 to 6600 the flow near the step
//! is three-dimensional; their own two-dimensional calculation first finds
//! the eddy on the upper wall above 420. Above 400 the solver refuses by
//! name. From 389, the higher compared condition, to 400 the same equations
//! are settled and nothing was compared; at 389 the rig was already showing
//! small deviations from two-dimensionality between 7 and 12 step heights.
//! Until this version the top was 389.
//!
//! The protocol, the data and the comparison are in
//! `evidence/research/fluid_armaly_backward_step/`, and the Figure 15 reading
//! and its score in its `fix_0.7.0/`. The measured profiles are
//! the ones replotted in NASA Technical Memorandum 106248 (1993). The paper of
//! Armaly and his colleagues is readable at
//! <https://courses.washington.edu/me431/handouts/armaly-jfm-83.pdf>; the
//! builder had not read it when the comparison was made.

use crate::normal::NormalOrZero;
use std::fmt;

/// Step height `S`, millimetres.
pub const STEP_HEIGHT_MM: f64 = 4.9;
/// Inlet channel height `h`, millimetres.
pub const INLET_HEIGHT_MM: f64 = 5.2;
/// Channel height downstream of the step `H`, millimetres.
pub const CHANNEL_HEIGHT_MM: f64 = STEP_HEIGHT_MM + INLET_HEIGHT_MM;

/// A Reynolds number must be above this: at zero the viscosity `2 h / Re`
/// is infinite, and below it negative.
pub const REYNOLDS_ABOVE: f64 = 0.0;
/// The lower of the two measured conditions compared with measurement.
pub const COMPARED_REYNOLDS_MIN: f64 = 100.0;
/// Highest Reynolds number served: the end of two-dimensional flow over
/// this step. Armaly et al. (1983), page 482: "At Reynolds numbers smaller
/// than 400 the flow maintained its two-dimensionality", shown at 397 by
/// their Figure 8, and "the flow downstream, in the immediate vicinity of the
/// step, was found to be three-dimensional within the Reynolds-number range
/// 400 < Re < 6600". The solver's equations are those of steady
/// two-dimensional flow. Until this version it was 389, the higher of the
/// two conditions compared ([`COMPARED_REYNOLDS_MAX`]).
pub const REYNOLDS_MAX: f64 = 400.0;
/// The higher of the two conditions compared with measured profiles. Stated,
/// not enforced: from it to [`REYNOLDS_MAX`] the flow is answered and was not
/// compared.
pub const COMPARED_REYNOLDS_MAX: f64 = 389.0;
/// First station served, in step heights downstream of the step.
pub const STATION_MIN_X_OVER_S: f64 = 0.0;
/// Last station served, in step heights downstream of the step, at every
/// Reynolds number: the grid ends at 30.6 step heights. At
/// [`COMPARED_REYNOLDS_MIN`] Armaly's Figure 15 has measured profiles out to
/// 30.31 step heights, and they were compared out to there.
pub const STATION_MAX_X_OVER_S: f64 = 30.0;
/// Last station compared with a measured profile above
/// [`COMPARED_REYNOLDS_MIN`]: the last station of the measured profiles at
/// Reynolds number 389 (Armaly's Figure 16 and Figure 4 of NASA Technical
/// Memorandum 106248 both end there). A station past it is answered; no
/// measured profile past it was compared at any Reynolds number above 100.
pub const COMPARED_STATION_MAX_ABOVE_100_X_OVER_S: f64 = 13.57;

/// Grid intervals over the step height.
pub const INTERVALS_OVER_STEP: usize = 24;
/// Grid intervals over the inlet height.
pub const INTERVALS_OVER_INLET: usize = 26;
/// Grid spacing along the channel, millimetres.
pub const DX_MM: f64 = 0.5;
/// Grid columns upstream of the step: the inlet channel is 10 mm long.
pub const INLET_COLUMNS: usize = 20;
/// Grid columns downstream of the step: the outlet is 150 mm from the step.
pub const OUTLET_COLUMNS: usize = 300;

/// The Reynolds numbers settled on the way to a higher one, in order.
pub const LADDER: [f64; 3] = [100.0, 200.0, 300.0];
/// Largest imbalance at which a rung of the ladder is left.
pub const RUNG_TOLERANCE: f64 = 1.0e-4;
/// Largest imbalance of an accepted field, in solver units, times
/// [`viscous_scale`] below Reynolds number 100.
pub const IMBALANCE_TOLERANCE: f64 = 1.0e-9;
/// An imbalance above this, times [`viscous_scale`] below Reynolds number
/// 100, stops the solver.
pub const IMBALANCE_CAP: f64 = 1.0e4;
/// Newton iterations allowed at one Reynolds number.
pub const MAX_NEWTON_ITERATIONS: u32 = 12;
/// Largest departure of the outlet profile from the developed parabola,
/// over the largest inlet speed.
pub const OUTLET_TOLERANCE: f64 = 0.02;

/// The public calculation the test compares with.
pub const REFERENCE_NAME: &str = "E. Erturk, Numerical solutions of 2-D steady incompressible flow over a backward-facing step, Part I: High Reynolds number solutions, Computers & Fluids 37 (2008) 633-655, Table 1, expansion ratio 1.942 with an inlet channel of 20 step heights: reattachment at 2.878 step heights for Reynolds number 100, 4.900 for 200 and 6.659 for 300";
pub const REFERENCE_URL: &str = "https://doi.org/10.1016/j.compfluid.2007.09.003";
pub const REFERENCE_RETRIEVED: &str = "2026-10-09";
/// The solver's reattachment length is within this fraction of the
/// reference calculation's.
pub const REFERENCE_TOLERANCE_FRACTION: f64 = 0.02;

/// One question: a Reynolds number, and the station of the velocity profile.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StepQuestion {
    /// Mean inlet speed times twice the inlet height over the kinematic viscosity.
    pub reynolds_number: f64,
    /// Where the velocity profile is taken, in step heights downstream of the step.
    pub station_x_over_s: f64,
}

/// A settled flow.
#[derive(Clone, Debug, PartialEq)]
pub struct StepFlow {
    /// Where the flow reattaches to the floor, in step heights from the step.
    pub reattachment_x_over_s: f64,
    /// The same length in millimetres.
    pub reattachment_mm: f64,
    /// Every place downstream of the step where the shear on the floor
    /// changes sign, in step heights, rising.
    pub floor_shear_sign_changes_x_over_s: Vec<f64>,
    /// Every place where the shear on the upper wall changes sign, in step
    /// heights, rising. Empty when the flow stays attached to it.
    pub upper_wall_shear_sign_changes_x_over_s: Vec<f64>,
    /// Height of each grid node over the channel height, floor to upper wall.
    pub profile_y_over_h: Vec<f64>,
    /// Streamwise speed over the largest inlet speed at the asked station, at
    /// each of those heights.
    pub profile_u_over_umax: Vec<f64>,
    /// Largest imbalance of the two equations over every node, solver units.
    pub largest_imbalance: f64,
    /// Largest departure of the outlet profile from the developed parabola,
    /// over the largest inlet speed.
    pub outlet_departure: f64,
    /// Linear solves taken, over every rung of the ladder.
    pub newton_iterations: u32,
    /// The stream function at every node, column by column from the inlet,
    /// each column floor to upper wall; mm times the mean inlet speed.
    pub stream_function: Vec<f64>,
    /// The vorticity at every node in the same order; mean inlet speed per mm.
    pub vorticity: Vec<f64>,
}

/// Why no flow was returned.
#[derive(Clone, Debug, PartialEq)]
pub enum StepReject {
    /// `reynolds_number` is not above [`REYNOLDS_ABOVE`]: the viscosity `2 h / Re` is infinite or negative.
    ReynoldsNotPositive { reynolds_number: f64 },
    /// `reynolds_number` is above [`REYNOLDS_MAX`]: above 400 the flow over this step is three-dimensional (Armaly et al. 1983, page 482: two-dimensional below 400, three-dimensional from 400 to 6600), and the solver's equations are those of steady two-dimensional flow.
    ReynoldsOutOfRange { reynolds_number: f64 },
    /// `station_x_over_s` is outside [`STATION_MIN_X_OVER_S`] to [`STATION_MAX_X_OVER_S`], where the grid ends.
    StationOutOfRange { station_x_over_s: f64 },
    /// A value of the field or its imbalance was NaN or infinite.
    NonFiniteValue,
    /// The linear system of a Newton iteration had no usable pivot.
    SingularSystem,
    /// The largest imbalance rose above [`IMBALANCE_CAP`] in the viscous scale.
    ImbalanceGrew { reynolds_number: f64, imbalance: f64 },
    /// The largest imbalance was still above the tolerance after [`MAX_NEWTON_ITERATIONS`] iterations at one Reynolds number.
    NotSettled { reynolds_number: f64, imbalance: f64, iterations: u32 },
    /// The outlet profile departs from the developed parabola by more than [`OUTLET_TOLERANCE`].
    OutletNotDeveloped { departure: f64 },
    /// The shear on the floor does not show one reattachment, with at most a corner eddy before it.
    FlowPatternNotRecognised { sign_changes: usize },
}

impl fmt::Display for StepReject {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ReynoldsNotPositive { reynolds_number } => write!(
                f,
                "Reynolds number {reynolds_number} is not above {REYNOLDS_ABOVE}: the viscosity would be infinite or negative"
            ),
            Self::ReynoldsOutOfRange { reynolds_number } => write!(
                f,
                "Reynolds number {reynolds_number} is above {REYNOLDS_MAX}: above 400 the flow over this step is three-dimensional (Armaly et al. 1983, page 482: two-dimensional below 400, three-dimensional from 400 to 6600), and the solver's equations are those of steady two-dimensional flow"
            ),
            Self::StationOutOfRange { station_x_over_s } => write!(
                f,
                "station {station_x_over_s} step heights is outside {STATION_MIN_X_OVER_S} to {STATION_MAX_X_OVER_S} step heights, where the grid ends"
            ),
            Self::NonFiniteValue => write!(f, "a value of the field was not finite"),
            Self::SingularSystem => write!(f, "a Newton iteration had no usable pivot"),
            Self::ImbalanceGrew {
                reynolds_number,
                imbalance,
            } => write!(
                f,
                "at Reynolds number {reynolds_number} the largest imbalance rose to {imbalance}, above {IMBALANCE_CAP}"
            ),
            Self::NotSettled {
                reynolds_number,
                imbalance,
                iterations,
            } => write!(
                f,
                "at Reynolds number {reynolds_number} the largest imbalance was {imbalance} after {iterations} iterations"
            ),
            Self::OutletNotDeveloped { departure } => write!(
                f,
                "the outlet profile departs from the developed parabola by {departure} of the largest inlet speed, above {OUTLET_TOLERANCE}"
            ),
            Self::FlowPatternNotRecognised { sign_changes } => write!(
                f,
                "the shear on the floor changes sign {sign_changes} times in a pattern that is not one reattachment"
            ),
        }
    }
}

impl std::error::Error for StepReject {}

/// The numerical choices. The served solver uses [`SERVED`]; the tests break
/// one at a time.
#[derive(Clone, Copy, Debug)]
struct Numerics {
    inlet_columns: usize,
    outlet_columns: usize,
    ladder: &'static [f64],
    max_newton_iterations: u32,
    imbalance_tolerance: f64,
    imbalance_cap: f64,
}

const SERVED: Numerics = Numerics {
    inlet_columns: INLET_COLUMNS,
    outlet_columns: OUTLET_COLUMNS,
    ladder: &LADDER,
    max_newton_iterations: MAX_NEWTON_ITERATIONS,
    imbalance_tolerance: IMBALANCE_TOLERANCE,
    imbalance_cap: IMBALANCE_CAP,
};

/// What a node is.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Node {
    Solid,
    Inlet,
    Outlet,
    /// On the floor, the upper wall, or the floor of the inlet channel.
    WallFlat,
    /// On the face of the step.
    WallFace,
    /// The edge of the step.
    Corner,
    Fluid,
}

struct Grid {
    /// Nodes in a column.
    m: usize,
    /// Index of the last column.
    nx: usize,
    /// Column of the step face.
    i0: usize,
    /// Row of the step edge.
    n1: usize,
    /// Row of the upper wall.
    ny: usize,
    y: Vec<f64>,
}

impl Grid {
    fn new(numerics: &Numerics) -> Self {
        let n1 = INTERVALS_OVER_STEP;
        let n2 = INTERVALS_OVER_INLET;
        let ny = n1 + n2;
        let mut y = Vec::with_capacity(ny + 1);
        for j in 0..n1 {
            y.push(j as f64 * (STEP_HEIGHT_MM / n1 as f64));
        }
        for j in 0..=n2 {
            y.push(STEP_HEIGHT_MM + j as f64 * (INLET_HEIGHT_MM / n2 as f64));
        }
        y[n1] = STEP_HEIGHT_MM;
        y[ny] = CHANNEL_HEIGHT_MM;
        Grid {
            m: ny + 1,
            nx: numerics.inlet_columns + numerics.outlet_columns,
            i0: numerics.inlet_columns,
            n1,
            ny,
            y,
        }
    }

    fn node(&self, i: usize, j: usize) -> Node {
        if i < self.i0 {
            if j < self.n1 {
                Node::Solid
            } else if i == 0 {
                Node::Inlet
            } else if j == self.n1 || j == self.ny {
                Node::WallFlat
            } else {
                Node::Fluid
            }
        } else if i == self.i0 {
            if j == self.ny {
                Node::WallFlat
            } else if j == self.n1 {
                Node::Corner
            } else if j < self.n1 {
                Node::WallFace
            } else {
                Node::Fluid
            }
        } else if j == 0 || j == self.ny {
            Node::WallFlat
        } else if i == self.nx {
            Node::Outlet
        } else {
            Node::Fluid
        }
    }

    /// Index of the stream function at a node; the vorticity is the next.
    fn psi(&self, i: usize, j: usize) -> usize {
        (i * self.m + j) * 2
    }

    fn x_mm(&self, i: usize) -> f64 {
        (i as f64 - self.i0 as f64) * DX_MM
    }
}

/// Stream function, speed over the mean and vorticity of the inlet parabola.
fn inlet_profile(y_mm: f64) -> (f64, f64) {
    let eta = (y_mm - STEP_HEIGHT_MM) / INLET_HEIGHT_MM;
    let psi = INLET_HEIGHT_MM * (3.0 * eta * eta - 2.0 * eta * eta * eta);
    let omega = -6.0 * (1.0 - 2.0 * eta) / INLET_HEIGHT_MM;
    (psi, omega)
}

/// A square matrix whose entries lie within `half` of the diagonal, with
/// room for the fill of row exchanges.
struct Band {
    n: usize,
    half: usize,
    width: usize,
    a: Vec<f64>,
}

impl Band {
    fn new(n: usize, half: usize) -> Self {
        let width = 3 * half + 1;
        Band {
            n,
            half,
            width,
            a: vec![0.0; n * width],
        }
    }

    fn clear(&mut self) {
        self.a.fill(0.0);
    }

    fn add(&mut self, row: usize, col: usize, value: f64) {
        debug_assert!(col + self.half >= row && col <= row + self.half);
        self.a[row * self.width + col + self.half - row] += value;
    }

    /// Solves `A x = b` in place of `b` by elimination with row exchanges.
    /// The matrix is destroyed. `false` when a column has no pivot.
    fn solve(&mut self, b: &mut [f64]) -> bool {
        let (n, half, width) = (self.n, self.half, self.width);
        for k in 0..n {
            let last_row = (k + half).min(n - 1);
            let mut pivot_row = k;
            let mut pivot = self.a[k * width + half].abs();
            for r in k + 1..=last_row {
                let value = self.a[r * width + k + half - r].abs();
                if value > pivot {
                    pivot = value;
                    pivot_row = r;
                }
            }
            if !(pivot > 0.0) || !pivot.is_finite() {
                return false;
            }
            let last_col = (k + 2 * half).min(n - 1);
            if pivot_row != k {
                for c in k..=last_col {
                    let at_k = k * width + c + half - k;
                    let at_p = pivot_row * width + c + half - pivot_row;
                    // Row `pivot_row` holds nothing beyond its own band.
                    let from_p = if c <= pivot_row + 2 * half { self.a[at_p] } else { 0.0 };
                    let from_k = self.a[at_k];
                    self.a[at_k] = from_p;
                    if c <= pivot_row + 2 * half {
                        self.a[at_p] = from_k;
                    }
                }
                b.swap(k, pivot_row);
            }
            let diagonal = self.a[k * width + half];
            let (upper, lower) = self.a.split_at_mut((k + 1) * width);
            let pivot_line = &upper[k * width + half + 1..k * width + half + 1 + (last_col - k)];
            for r in k + 1..=last_row {
                let row = &mut lower[(r - k - 1) * width..(r - k) * width];
                let at = k + half - r;
                let factor = row[at] / diagonal;
                if factor == 0.0 {
                    continue;
                }
                row[at] = 0.0;
                for (target, source) in row[at + 1..at + 1 + (last_col - k)].iter_mut().zip(pivot_line) {
                    *target -= factor * source;
                }
                b[r] -= factor * b[k];
            }
        }
        for k in (0..n).rev() {
            let last_col = (k + 2 * half).min(n - 1);
            let mut sum = b[k];
            for c in k + 1..=last_col {
                sum -= self.a[k * width + c + half - k] * b[c];
            }
            b[k] = sum / self.a[k * width + half];
        }
        true
    }
}

/// The imbalance of both equations at every node into `r`, their largest
/// size as the return value, and the matrix of their derivatives into `jac`.
fn assemble(grid: &Grid, nu: f64, z: &[f64], r: &mut [f64], jac: &mut Band) -> f64 {
    let (m, nx, ny) = (grid.m, grid.nx, grid.ny);
    let y = &grid.y;
    let flow = INLET_HEIGHT_MM;
    let cx = 1.0 / (DX_MM * DX_MM);
    let two_dx = 2.0 * DX_MM;
    jac.clear();
    for i in 0..=nx {
        for j in 0..m {
            let rp = grid.psi(i, j);
            let rw = rp + 1;
            match grid.node(i, j) {
                Node::Solid => {
                    r[rp] = z[rp];
                    jac.add(rp, rp, 1.0);
                    r[rw] = z[rw];
                    jac.add(rw, rw, 1.0);
                }
                Node::Inlet => {
                    let (psi, omega) = inlet_profile(y[j]);
                    r[rp] = z[rp] - psi;
                    jac.add(rp, rp, 1.0);
                    r[rw] = z[rw] - omega;
                    jac.add(rw, rw, 1.0);
                }
                Node::Outlet => {
                    let west = grid.psi(i - 1, j);
                    r[rp] = z[rp] - z[west];
                    jac.add(rp, rp, 1.0);
                    jac.add(rp, west, -1.0);
                    r[rw] = z[rw] - z[west + 1];
                    jac.add(rw, rw, 1.0);
                    jac.add(rw, west + 1, -1.0);
                }
                Node::WallFlat => {
                    let wall = if j == ny { flow } else { 0.0 };
                    r[rp] = z[rp] - wall;
                    jac.add(rp, rp, 1.0);
                    let inside = if j == ny { j - 1 } else { j + 1 };
                    let gap = y[inside] - y[j];
                    let c = 2.0 / (gap * gap);
                    let neighbour = grid.psi(i, inside);
                    r[rw] = z[rw] + c * (z[neighbour] - wall);
                    jac.add(rw, rw, 1.0);
                    jac.add(rw, neighbour, c);
                }
                Node::WallFace => {
                    r[rp] = z[rp];
                    jac.add(rp, rp, 1.0);
                    let c = 2.0 * cx;
                    let neighbour = grid.psi(i + 1, j);
                    r[rw] = z[rw] + c * z[neighbour];
                    jac.add(rw, rw, 1.0);
                    jac.add(rw, neighbour, c);
                }
                Node::Corner => {
                    r[rp] = z[rp];
                    jac.add(rp, rp, 1.0);
                    let gap = y[j + 1] - y[j];
                    let ca = 1.0 / (gap * gap);
                    let above = grid.psi(i, j + 1);
                    let right = grid.psi(i + 1, j);
                    r[rw] = z[rw] + ca * z[above] + cx * z[right];
                    jac.add(rw, rw, 1.0);
                    jac.add(rw, above, ca);
                    jac.add(rw, right, cx);
                }
                Node::Fluid => {
                    let hm = y[j] - y[j - 1];
                    let hp = y[j + 1] - y[j];
                    let ym = -hp / (hm * (hm + hp));
                    let y0 = (hp - hm) / (hm * hp);
                    let yp = hm / (hp * (hm + hp));
                    let ymm = 2.0 / (hm * (hm + hp));
                    let y00 = -2.0 / (hm * hp);
                    let ypp = 2.0 / (hp * (hm + hp));
                    let pe = grid.psi(i + 1, j);
                    let pw = grid.psi(i - 1, j);
                    let pn = grid.psi(i, j + 1);
                    let ps = grid.psi(i, j - 1);
                    // The stream function equation.
                    r[rp] = cx * z[pe] + cx * z[pw] + ypp * z[pn] + ymm * z[ps]
                        + (-2.0 * cx + y00) * z[rp]
                        + z[rw];
                    jac.add(rp, pe, cx);
                    jac.add(rp, pw, cx);
                    jac.add(rp, pn, ypp);
                    jac.add(rp, ps, ymm);
                    jac.add(rp, rp, -2.0 * cx + y00);
                    jac.add(rp, rw, 1.0);
                    // The vorticity at a neighbour: its own unknown, or at
                    // the edge of the step the wall value on the side this
                    // node sees, which is a multiple of this node's own
                    // stream function.
                    let seen = |ni: usize, nj: usize| -> (usize, f64) {
                        if grid.node(ni, nj) == Node::Corner {
                            if j > nj {
                                let gap = y[j] - y[nj];
                                (rp, -2.0 / (gap * gap))
                            } else {
                                (rp, -2.0 * cx)
                            }
                        } else {
                            (grid.psi(ni, nj) + 1, 1.0)
                        }
                    };
                    let (ie, fe) = seen(i + 1, j);
                    let (iw, fw) = seen(i - 1, j);
                    let (inn, fnn) = seen(i, j + 1);
                    let (is, fs) = seen(i, j - 1);
                    let (ve, vw, vn, vs, vp) = (fe * z[ie], fw * z[iw], fnn * z[inn], fs * z[is], z[rw]);
                    let lap = cx * (ve + vw - 2.0 * vp) + ymm * vs + y00 * vp + ypp * vn;
                    let wx = (ve - vw) / two_dx;
                    let wy = ym * vs + y0 * vp + yp * vn;
                    let px = (z[pe] - z[pw]) / two_dx;
                    let py = ym * z[ps] + y0 * z[rp] + yp * z[pn];
                    r[rw] = nu * lap - (py * wx - px * wy);
                    jac.add(rw, ie, fe * (nu * cx - py / two_dx));
                    jac.add(rw, iw, fw * (nu * cx + py / two_dx));
                    jac.add(rw, inn, fnn * (nu * ypp + px * yp));
                    jac.add(rw, is, fs * (nu * ymm + px * ym));
                    jac.add(rw, rw, nu * (-2.0 * cx + y00) + px * y0);
                    jac.add(rw, pe, wy / two_dx);
                    jac.add(rw, pw, -wy / two_dx);
                    jac.add(rw, pn, -wx * yp);
                    jac.add(rw, ps, -wx * ym);
                    jac.add(rw, rp, -wx * y0);
                }
            }
        }
    }
    let mut largest = 0.0_f64;
    for value in r.iter() {
        if !value.is_finite() {
            return f64::NAN;
        }
        largest = largest.max(value.abs());
    }
    largest
}

/// The places along one wall where its vorticity changes sign, downstream
/// of the step, in millimetres, each with the sign before it.
fn sign_changes(grid: &Grid, z: &[f64], row: usize) -> Vec<(f64, bool)> {
    let mut found = Vec::new();
    for i in grid.i0 + 1..grid.nx {
        let here = z[grid.psi(i, row) + 1];
        let next = z[grid.psi(i + 1, row) + 1];
        if (here > 0.0) != (next > 0.0) {
            let x = grid.x_mm(i) + DX_MM * here / (here - next);
            found.push((x, here > 0.0));
        }
    }
    found
}

/// Streamwise speed over the largest inlet speed at every node of a column.
fn column_speed(grid: &Grid, z: &[f64], i: usize) -> Vec<f64> {
    let y = &grid.y;
    let mut u = vec![0.0; grid.m];
    if grid.node(i, 0) == Node::Solid {
        // Upstream of the step the floor is at the step height.
        for j in grid.n1 + 1..grid.ny {
            u[j] = speed_at(grid, z, i, j, y);
        }
    } else {
        for j in 1..grid.ny {
            u[j] = speed_at(grid, z, i, j, y);
        }
    }
    if i == grid.i0 {
        for value in u.iter_mut().take(grid.n1 + 1) {
            *value = 0.0;
        }
    }
    u
}

fn speed_at(grid: &Grid, z: &[f64], i: usize, j: usize, y: &[f64]) -> f64 {
    let hm = y[j] - y[j - 1];
    let hp = y[j + 1] - y[j];
    (-hp / (hm * (hm + hp)) * z[grid.psi(i, j - 1)]
        + (hp - hm) / (hm * hp) * z[grid.psi(i, j)]
        + hm / (hp * (hm + hp)) * z[grid.psi(i, j + 1)])
        / 1.5
}

/// The scale of the viscous term against its scale at
/// [`COMPARED_REYNOLDS_MIN`]: `100 / Re` below 100, one from 100 up. The
/// imbalance tolerance and cap are multiplied by it.
pub fn viscous_scale(reynolds_number: f64) -> f64 {
    (COMPARED_REYNOLDS_MIN / reynolds_number).max(1.0)
}

/// Settles the flow at one Reynolds number and returns it with the velocity
/// profile at one station, or names the limit that stopped it.
///
/// Each value of the answer below the normal range is written as zero
/// with its sign ([`crate::normal`]).
pub fn solve(question: &StepQuestion) -> Result<StepFlow, StepReject> {
    crate::normal::held(solve_as_computed(question))
}

/// The answer as the equations compute it, before a value below the
/// normal range is written as zero.
fn solve_as_computed(question: &StepQuestion) -> Result<StepFlow, StepReject> {
    solve_with(question, &SERVED)
}

impl NormalOrZero for StepFlow {
    fn normal_or_zero(&mut self) {
        self.reattachment_x_over_s.normal_or_zero();
        self.reattachment_mm.normal_or_zero();
        self.floor_shear_sign_changes_x_over_s.normal_or_zero();
        self.upper_wall_shear_sign_changes_x_over_s.normal_or_zero();
        self.profile_y_over_h.normal_or_zero();
        self.profile_u_over_umax.normal_or_zero();
        self.largest_imbalance.normal_or_zero();
        self.outlet_departure.normal_or_zero();
        self.stream_function.normal_or_zero();
        self.vorticity.normal_or_zero();
    }
}

/// The limits the question itself shows, in the order the solver asks them:
/// the Reynolds number, then the station. No flow is settled.
pub fn admit(question: &StepQuestion) -> Result<(), StepReject> {
    let reynolds_number = question.reynolds_number;
    let station = question.station_x_over_s;
    if reynolds_number.is_nan() {
        return Err(StepReject::NonFiniteValue);
    }
    if !(reynolds_number > REYNOLDS_ABOVE) {
        return Err(StepReject::ReynoldsNotPositive { reynolds_number });
    }
    if !(reynolds_number <= REYNOLDS_MAX) {
        return Err(StepReject::ReynoldsOutOfRange { reynolds_number });
    }
    if !(station >= STATION_MIN_X_OVER_S && station <= STATION_MAX_X_OVER_S) {
        return Err(StepReject::StationOutOfRange {
            station_x_over_s: station,
        });
    }
    Ok(())
}

fn solve_with(question: &StepQuestion, numerics: &Numerics) -> Result<StepFlow, StepReject> {
    admit(question)?;
    let reynolds_number = question.reynolds_number;
    let station = question.station_x_over_s;
    let grid = Grid::new(numerics);
    let (m, nx) = (grid.m, grid.nx);
    let n = (nx + 1) * m * 2;
    let flow = INLET_HEIGHT_MM;

    // The starting field: the inlet parabola as far as the step, and the
    // developed parabola across the whole channel after it.
    let mut z = vec![0.0; n];
    for i in 0..=nx {
        for j in 0..m {
            if grid.node(i, j) == Node::Solid {
                continue;
            }
            let at = grid.psi(i, j);
            if i <= grid.i0 {
                if j >= grid.n1 {
                    let (psi, omega) = inlet_profile(grid.y[j]);
                    z[at] = psi;
                    z[at + 1] = omega;
                }
            } else {
                let eta = grid.y[j] / CHANNEL_HEIGHT_MM;
                z[at] = flow * (3.0 * eta * eta - 2.0 * eta * eta * eta);
                z[at + 1] = -flow * (6.0 - 12.0 * eta) / (CHANNEL_HEIGHT_MM * CHANNEL_HEIGHT_MM);
            }
        }
    }

    let mut jac = Band::new(n, 2 * m + 1);
    let mut r = vec![0.0; n];
    let mut newton_iterations = 0_u32;
    let mut largest_imbalance = f64::INFINITY;
    let rungs = numerics
        .ladder
        .iter()
        .copied()
        .filter(|rung| *rung < reynolds_number)
        .map(|rung| (rung, RUNG_TOLERANCE))
        .chain(std::iter::once((reynolds_number, numerics.imbalance_tolerance)));
    for (rung, tolerance) in rungs {
        let nu = 2.0 * INLET_HEIGHT_MM / rung;
        // Below the lower measured condition the viscous term, and with it
        // the imbalance of any field, grows as 1/Re: the tolerance and the
        // cap are held in that scale. From 100 up the scale is one.
        let scale = viscous_scale(rung);
        let (tolerance, cap) = (tolerance * scale, numerics.imbalance_cap * scale);
        let mut iterations = 0_u32;
        loop {
            let imbalance = assemble(&grid, nu, &z, &mut r, &mut jac);
            if !imbalance.is_finite() {
                return Err(StepReject::NonFiniteValue);
            }
            if imbalance > cap {
                return Err(StepReject::ImbalanceGrew {
                    reynolds_number: rung,
                    imbalance,
                });
            }
            if imbalance <= tolerance {
                largest_imbalance = imbalance;
                break;
            }
            if iterations == numerics.max_newton_iterations {
                return Err(StepReject::NotSettled {
                    reynolds_number: rung,
                    imbalance,
                    iterations,
                });
            }
            if !jac.solve(&mut r) {
                return Err(StepReject::SingularSystem);
            }
            for (value, step) in z.iter_mut().zip(&r) {
                *value -= step;
            }
            iterations += 1;
            newton_iterations += 1;
        }
    }

    // The outlet against the developed parabola.
    let outlet = column_speed(&grid, &z, nx - 1);
    let mut outlet_departure = 0.0_f64;
    for (j, speed) in outlet.iter().enumerate() {
        let eta = grid.y[j] / CHANNEL_HEIGHT_MM;
        let developed = 6.0 * eta * (1.0 - eta) * (INLET_HEIGHT_MM / CHANNEL_HEIGHT_MM) / 1.5;
        outlet_departure = outlet_departure.max((speed - developed).abs());
    }
    if !outlet_departure.is_finite() {
        return Err(StepReject::NonFiniteValue);
    }
    if outlet_departure > OUTLET_TOLERANCE {
        return Err(StepReject::OutletNotDeveloped {
            departure: outlet_departure,
        });
    }

    // On the floor a positive vorticity is flow back toward the step.
    let floor = sign_changes(&grid, &z, 0);
    let recognised = match floor.as_slice() {
        [(_, true)] => true,
        [(_, false), (_, true)] => true,
        _ => false,
    };
    if !recognised {
        return Err(StepReject::FlowPatternNotRecognised {
            sign_changes: floor.len(),
        });
    }
    let reattachment_mm = floor[floor.len() - 1].0;
    let roof = sign_changes(&grid, &z, grid.ny);

    // The profile at the station, read linearly between the two columns
    // around it.
    let place = (station * STEP_HEIGHT_MM - grid.x_mm(0)) / DX_MM;
    let column = (place.floor() as usize).min(nx - 1);
    let share = place - column as f64;
    let left = column_speed(&grid, &z, column);
    let right = column_speed(&grid, &z, column + 1);
    let profile_u_over_umax: Vec<f64> = left
        .iter()
        .zip(&right)
        .map(|(a, b)| (1.0 - share) * a + share * b)
        .collect();
    let profile_y_over_h: Vec<f64> = grid.y.iter().map(|y| y / CHANNEL_HEIGHT_MM).collect();

    let mut stream_function = Vec::with_capacity(n / 2);
    let mut vorticity = Vec::with_capacity(n / 2);
    for pair in z.chunks_exact(2) {
        stream_function.push(pair[0]);
        vorticity.push(pair[1]);
    }
    if stream_function.iter().chain(&vorticity).any(|value| !value.is_finite()) {
        return Err(StepReject::NonFiniteValue);
    }
    Ok(StepFlow {
        reattachment_x_over_s: reattachment_mm / STEP_HEIGHT_MM,
        reattachment_mm,
        floor_shear_sign_changes_x_over_s: floor.iter().map(|(x, _)| x / STEP_HEIGHT_MM).collect(),
        upper_wall_shear_sign_changes_x_over_s: roof.iter().map(|(x, _)| x / STEP_HEIGHT_MM).collect(),
        profile_y_over_h,
        profile_u_over_umax,
        largest_imbalance,
        outlet_departure,
        newton_iterations,
        stream_function,
        vorticity,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The flows at the two ends of the range, at the station 3.06, settled
    /// once for the tests that read them.
    fn low() -> &'static StepFlow {
        static FLOW: std::sync::OnceLock<StepFlow> = std::sync::OnceLock::new();
        FLOW.get_or_init(|| solve(&ask(COMPARED_REYNOLDS_MIN, 3.06)).expect("settles"))
    }

    fn high() -> &'static StepFlow {
        static FLOW: std::sync::OnceLock<StepFlow> = std::sync::OnceLock::new();
        FLOW.get_or_init(|| solve(&ask(COMPARED_REYNOLDS_MAX, 3.06)).expect("settles"))
    }

    fn ask(reynolds_number: f64, station_x_over_s: f64) -> StepQuestion {
        StepQuestion {
            reynolds_number,
            station_x_over_s,
        }
    }

    /// E. Erturk, Computers & Fluids 37 (2008) 633-655, Table 1,
    /// <https://doi.org/10.1016/j.compfluid.2007.09.003>.
    /// Retrieved 2026-10-09. A calculation, not a measurement: the same
    /// two-dimensional equations at the rig's expansion ratio, 1.942, with an
    /// inlet channel of 20 step heights, which gives 2.878, 4.900 and 6.659
    /// step heights at Reynolds numbers 100, 200 and 300. This solver gives
    /// 2.835, 4.874 and 6.663: lower by 1.5 and 0.5 percent and higher by 0.1.
    /// The table has no row at 389.
    #[test]
    fn erturk_2008_table_1_reattachment() {
        let fraction = low().reattachment_x_over_s / 2.878 - 1.0;
        assert!(fraction.abs() <= REFERENCE_TOLERANCE_FRACTION, "{fraction}");
        assert!(fraction < -0.010 && fraction > -0.020, "{fraction}");
    }

    /// The two higher rows of the table are heavy in an unoptimised build, so
    /// this is skipped by default: `cargo test --release -- --ignored`.
    #[test]
    #[ignore]
    fn erturk_2008_table_1_reattachment_at_200_and_300() {
        for (reynolds_number, erturk) in [(200.0, 4.900), (300.0, 6.659)] {
            let flow = solve(&ask(reynolds_number, 3.06)).expect("settles");
            let fraction = flow.reattachment_x_over_s / erturk - 1.0;
            assert!(fraction.abs() <= REFERENCE_TOLERANCE_FRACTION, "{reynolds_number}: {fraction}");
            assert!(fraction.abs() <= 0.006, "{reynolds_number}: {fraction}");
        }
    }

    /// NASA Technical Memorandum 106248 (1993), page 3,
    /// <https://ntrs.nasa.gov/api/citations/19930021958/downloads/19930021958.pdf>:
    /// a compressible code at Mach 0.1 on a 161 by 65 grid gives 3.3 and 8.7
    /// step heights. That is what a calculation without an inlet channel
    /// gives, and it was this solver's reference until its second scoring.
    /// This solver, with the channel, gives 2.83 and 8.07, lower by 14 and 7
    /// percent: held here as a recorded difference, not as agreement.
    #[test]
    fn nasa_tm_106248_two_dimensional_reattachment_is_longer_by_the_inlet_channel() {
        let fraction = low().reattachment_x_over_s / 3.3 - 1.0;
        assert!(fraction < -0.13 && fraction > -0.15, "{fraction}");
    }

    /// The upper compared condition is heavy in an unoptimised build, so this
    /// is skipped by default: `cargo test --release -- --ignored`.
    #[test]
    #[ignore]
    fn nasa_tm_106248_two_dimensional_reattachment_at_the_upper_end() {
        let fraction = high().reattachment_x_over_s / 8.7 - 1.0;
        assert!(fraction < -0.06 && fraction > -0.08, "{fraction}");
    }

    /// The plain-Python model of `evidence/research/fluid_armaly_backward_step/analysis/bfs.py`,
    /// which solves the same difference equations with a different linear solver.
    #[test]
    fn agrees_with_the_plain_python_model() {
        let low = low();
        assert!((low.reattachment_x_over_s - 2.8349935962908743).abs() < 1e-8, "{}", low.reattachment_x_over_s);
        assert_eq!(low.floor_shear_sign_changes_x_over_s.len(), 1);
        for (node, expect) in [(5, 0.0653348040733973), (12, 0.2521553503410923), (24, 0.614511862454308), (30, 0.6296573738689354), (40, 0.3489804004725631)] {
            assert!((low.profile_u_over_umax[node] - expect).abs() < 1e-9, "{node}");
        }
    }

    /// The upper compared condition is heavy in an unoptimised build, so this
    /// is skipped by default: `cargo test --release -- --ignored`.
    #[test]
    #[ignore]
    fn agrees_with_the_plain_python_model_at_the_upper_end() {
        let low = low();
        let high = high();
        assert!(high.newton_iterations > low.newton_iterations);
        assert!((high.reattachment_x_over_s - 8.070951412957976).abs() < 1e-8, "{}", high.reattachment_x_over_s);
        assert_eq!(high.floor_shear_sign_changes_x_over_s.len(), 2);
        assert!((high.floor_shear_sign_changes_x_over_s[0] - 0.11791886374173041).abs() < 1e-8);
        assert!(high.upper_wall_shear_sign_changes_x_over_s.is_empty());
        for (node, expect) in [(5, -0.094695019560393), (12, -0.05338088174448002), (24, 0.47404782127970974), (30, 0.8172791244413204), (40, 0.7332519120725091)] {
            assert!((high.profile_u_over_umax[node] - expect).abs() < 1e-9, "{node}");
        }
        assert!(high.largest_imbalance <= IMBALANCE_TOLERANCE);
        assert!((high.outlet_departure - 0.003640121827386489).abs() < 1e-8);
    }

    #[test]
    fn the_inlet_station_is_the_parabola_and_the_walls_do_not_slip() {
        let flow = solve(&ask(100.0, 0.0)).expect("settles");
        assert_eq!(flow.profile_y_over_h.len(), INTERVALS_OVER_STEP + INTERVALS_OVER_INLET + 1);
        assert_eq!(flow.profile_y_over_h[0], 0.0);
        assert_eq!(*flow.profile_y_over_h.last().expect("a node"), 1.0);
        for (j, u) in flow.profile_u_over_umax.iter().enumerate() {
            if j <= INTERVALS_OVER_STEP {
                assert_eq!(*u, 0.0, "{j}");
            }
        }
        let peak = flow.profile_u_over_umax.iter().fold(0.0_f64, |a, b| a.max(*b));
        assert!((peak - 1.0).abs() < 0.03, "{peak}");
        assert_eq!(*flow.profile_u_over_umax.last().expect("a node"), 0.0);
    }

    #[test]
    fn the_volume_flow_is_the_same_at_every_station() {
        let flow = low();
        let m = INTERVALS_OVER_STEP + INTERVALS_OVER_INLET + 1;
        for column in flow.stream_function.chunks_exact(m) {
            assert!((column[m - 1] - INLET_HEIGHT_MM).abs() < 1e-12);
            assert!(column[0].abs() < 1e-12);
        }
        // The speed profile integrates to the same flow, to the accuracy of
        // the trapezoid rule on this grid.
        let mut sum = 0.0;
        for j in 1..m {
            let dy = (flow.profile_y_over_h[j] - flow.profile_y_over_h[j - 1]) * CHANNEL_HEIGHT_MM;
            sum += 0.5 * (flow.profile_u_over_umax[j] + flow.profile_u_over_umax[j - 1]) * 1.5 * dy;
        }
        assert!((sum / INLET_HEIGHT_MM - 1.0).abs() < 5e-3, "{sum}");
    }

    /// A long sweep, skipped by default: `cargo test --release -- --ignored`.
    #[test]
    #[ignore]
    fn the_reattachment_length_rises_with_reynolds_number() {
        let mut last = 0.0;
        for reynolds in [100.0, 150.0, 200.0, 250.0, 300.0, 350.0, 389.0] {
            let flow = solve(&ask(reynolds, 1.0)).expect("settles");
            assert!(flow.reattachment_x_over_s > last);
            last = flow.reattachment_x_over_s;
        }
    }

    #[test]
    fn same_question_same_bits() {
        let a = low();
        let b = solve(&ask(COMPARED_REYNOLDS_MIN, 3.06)).expect("settles");
        assert_eq!(*a, b);
        assert_eq!(a.reattachment_x_over_s.to_bits(), b.reattachment_x_over_s.to_bits());
    }

    /// Past the old top, 389, to the new one, 400, where the flow over this
    /// step stops being two-dimensional: the field settles with its balance
    /// closed, the outlet developed, one reattachment on the floor and the
    /// upper wall still attached (Armaly's own calculation first finds the
    /// eddy there above 420), and the reattachment lies past the one at 389.
    /// The next number above 400 is refused by name.
    #[test]
    fn the_top_of_two_dimensional_flow_settles_with_its_balance_closed() {
        let top = solve(&ask(REYNOLDS_MAX, COMPARED_STATION_MAX_ABOVE_100_X_OVER_S)).expect("settles");
        assert!(top.largest_imbalance <= IMBALANCE_TOLERANCE, "{}", top.largest_imbalance);
        assert!(top.outlet_departure <= OUTLET_TOLERANCE, "{}", top.outlet_departure);
        assert!(top.upper_wall_shear_sign_changes_x_over_s.is_empty(), "{:?}", top.upper_wall_shear_sign_changes_x_over_s);
        assert!(top.reattachment_x_over_s > 8.070951412957976 && top.reattachment_x_over_s < 9.0, "{}", top.reattachment_x_over_s);
        let m = INTERVALS_OVER_STEP + INTERVALS_OVER_INLET + 1;
        for column in top.stream_function.chunks_exact(m) {
            assert!((column[m - 1] - INLET_HEIGHT_MM).abs() < 1e-12);
            assert!(column[0].abs() < 1e-12);
        }
        let past = f64::from_bits(REYNOLDS_MAX.to_bits() + 1);
        assert_eq!(admit(&ask(past, 1.0)), Err(StepReject::ReynoldsOutOfRange { reynolds_number: past }));
    }

    #[test]
    fn both_ends_of_both_ranges_are_inside_and_the_next_number_is_not() {
        // The compared lower condition is settled in `low`, the upper end in
        // `the_top_of_two_dimensional_flow_settles_with_its_balance_closed`,
        // and the first station in the inlet test.
        assert!(low().largest_imbalance <= IMBALANCE_TOLERANCE);
        assert!(solve(&ask(COMPARED_REYNOLDS_MIN, STATION_MAX_X_OVER_S)).is_ok());
        assert!(admit(&ask(REYNOLDS_MAX, STATION_MAX_X_OVER_S)).is_ok());
        assert!(admit(&ask(f64::from_bits(1), 1.0)).is_ok());
        for reynolds in [400.00000000000006, 401.0, 1.0e9, f64::INFINITY] {
            assert_eq!(solve(&ask(reynolds, 1.0)), Err(StepReject::ReynoldsOutOfRange { reynolds_number: reynolds }), "{reynolds}");
        }
        for reynolds in [0.0, -0.0, -100.0, f64::NEG_INFINITY] {
            assert_eq!(solve(&ask(reynolds, 1.0)), Err(StepReject::ReynoldsNotPositive { reynolds_number: reynolds }), "{reynolds}");
        }
        assert_eq!(solve(&ask(f64::NAN, 1.0)), Err(StepReject::NonFiniteValue));
        for station in [-1e-300, 30.000000000000004, f64::NAN, f64::NEG_INFINITY] {
            assert!(matches!(solve(&ask(100.0, station)), Err(StepReject::StationOutOfRange { .. })), "{station}");
        }
        assert_eq!(
            StepReject::ReynoldsNotPositive { reynolds_number: 0.0 }.to_string(),
            "Reynolds number 0 is not above 0: the viscosity would be infinite or negative"
        );
        assert_eq!(
            StepReject::StationOutOfRange { station_x_over_s: 31.0 }.to_string(),
            "station 31 step heights is outside 0 to 30 step heights, where the grid ends"
        );
    }

    /// Below Reynolds number 100, the lower measured condition, where engine
    /// 0.8.0 refused by name: the flow is settled down to creeping flow, to
    /// the same imbalance tolerance with the outlet developed and one
    /// reattachment, the volume flow the same at every station, and the
    /// eddy behind the step shrinking as the Reynolds number falls.
    #[test]
    fn below_the_lower_measured_condition_the_flow_settles_down_to_creeping_flow() {
        let m = INTERVALS_OVER_STEP + INTERVALS_OVER_INLET + 1;
        let mut last = low().reattachment_x_over_s;
        for reynolds in [99.99999999999999, 50.0, 10.0, 1.0, 1.0e-3, 1.0e-6] {
            let flow = solve(&ask(reynolds, 30.0)).unwrap_or_else(|reject| panic!("{reynolds}: {reject}"));
            assert!(flow.largest_imbalance <= IMBALANCE_TOLERANCE * viscous_scale(reynolds), "{reynolds}");
            assert!(flow.outlet_departure <= OUTLET_TOLERANCE, "{reynolds}");
            assert!(flow.reattachment_x_over_s < last, "{reynolds}: {} then {last}", flow.reattachment_x_over_s);
            assert!(flow.reattachment_x_over_s > 0.0);
            last = flow.reattachment_x_over_s;
            for column in flow.stream_function.chunks_exact(m) {
                assert!((column[m - 1] - INLET_HEIGHT_MM).abs() < 1e-12, "{reynolds}");
                assert!(column[0].abs() < 1e-12, "{reynolds}");
            }
        }
        // From 100 up the scale is one, so no answer served before moves.
        assert_eq!(viscous_scale(COMPARED_REYNOLDS_MIN), 1.0);
        assert_eq!(viscous_scale(REYNOLDS_MAX), 1.0);
        // Creeping flow: the reattachment tends to the corner eddy of Stokes
        // flow, 0.3507 step heights, and stops moving.
        assert!((last - 0.35066).abs() < 1.0e-4, "{last}");
        // Far below, the viscous term is so large that the rounding of its
        // 64-bit numbers is larger than the flow, and a limit of the
        // arithmetic names it: the floor shear is noise, the imbalance
        // cannot close, or the viscosity overflows.
        for reynolds in [1.0e-12, 1.0e-100] {
            match solve(&ask(reynolds, 1.0)) {
                Err(StepReject::FlowPatternNotRecognised { .. } | StepReject::ImbalanceGrew { .. } | StepReject::NotSettled { .. }) => {}
                other => panic!("{reynolds}: {:?}", other.map(|flow| flow.reattachment_x_over_s)),
            }
        }
        assert_eq!(solve(&ask(5.0e-324, 1.0)), Err(StepReject::NonFiniteValue));
    }

    /// Past 13.57 step heights above Reynolds number 100, where engine 0.8.0
    /// refused by name: the profile is answered out to 30 step heights, the
    /// same settled field at every station, and it relaxes to the developed
    /// parabola of the whole channel.
    #[test]
    fn a_station_past_the_last_compared_one_above_reynolds_100_is_answered() {
        let at = solve(&ask(150.0, COMPARED_STATION_MAX_ABOVE_100_X_OVER_S)).expect("settles");
        for station in [13.570000000000002, 20.0, STATION_MAX_X_OVER_S] {
            let flow = solve(&ask(150.0, station)).unwrap_or_else(|reject| panic!("{station}: {reject}"));
            assert_eq!(flow.stream_function, at.stream_function, "{station}");
            assert_eq!(flow.reattachment_x_over_s, at.reattachment_x_over_s);
            if station == STATION_MAX_X_OVER_S {
                for (y, u) in flow.profile_y_over_h.iter().zip(&flow.profile_u_over_umax) {
                    let developed = 6.0 * y * (1.0 - y) * (INLET_HEIGHT_MM / CHANNEL_HEIGHT_MM) / 1.5;
                    assert!((u - developed).abs() < OUTLET_TOLERANCE, "{y} {u} {developed}");
                }
            }
        }
        assert!(matches!(
            admit(&ask(REYNOLDS_MAX, 30.000000000000004)),
            Err(StepReject::StationOutOfRange { .. })
        ));
    }

    #[test]
    fn too_few_iterations_is_named() {
        let broken = Numerics {
            max_newton_iterations: 2,
            ..SERVED
        };
        match solve_with(&ask(100.0, 1.0), &broken) {
            Err(StepReject::NotSettled { iterations, imbalance, .. }) => {
                assert_eq!(iterations, 2);
                assert!(imbalance > RUNG_TOLERANCE);
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn without_the_ladder_the_highest_reynolds_number_is_refused_by_name() {
        let broken = Numerics {
            max_newton_iterations: 4,
            ladder: &[],
            ..SERVED
        };
        match solve_with(&ask(389.0, 1.0), &broken) {
            Err(StepReject::ImbalanceGrew { .. } | StepReject::NotSettled { .. }) => {}
            other => panic!("{:?}", other.map(|flow| flow.reattachment_x_over_s)),
        }
    }

    #[test]
    fn a_cap_below_the_first_imbalance_is_named() {
        let broken = Numerics {
            imbalance_cap: 1.0,
            ..SERVED
        };
        assert!(matches!(
            solve_with(&ask(100.0, 1.0), &broken),
            Err(StepReject::ImbalanceGrew { .. })
        ));
    }

    #[test]
    fn an_outlet_inside_the_separated_flow_is_named() {
        let broken = Numerics {
            outlet_columns: 60,
            ..SERVED
        };
        match solve_with(&ask(389.0, 1.0), &broken) {
            Err(
                StepReject::OutletNotDeveloped { .. }
                | StepReject::FlowPatternNotRecognised { .. }
                | StepReject::NotSettled { .. }
                | StepReject::ImbalanceGrew { .. },
            ) => {}
            other => panic!("{:?}", other.map(|flow| flow.reattachment_x_over_s)),
        }
    }

    #[test]
    fn a_matrix_with_an_empty_column_has_no_pivot() {
        let mut band = Band::new(3, 1);
        band.add(0, 0, 1.0);
        band.add(1, 0, 1.0);
        band.add(2, 2, 1.0);
        let mut b = [1.0, 1.0, 1.0];
        assert!(!band.solve(&mut b));
    }

    #[test]
    fn the_banded_solve_exchanges_rows() {
        // [[0, 2, 0], [1, 1, 1], [0, 3, 1]] x = [2, 3, 4] has x = [1, 1, 1].
        let mut band = Band::new(3, 1);
        band.add(0, 1, 2.0);
        band.add(1, 0, 1.0);
        band.add(1, 1, 1.0);
        band.add(1, 2, 1.0);
        band.add(2, 1, 3.0);
        band.add(2, 2, 1.0);
        let mut b = [2.0, 3.0, 4.0];
        assert!(band.solve(&mut b));
        for value in b {
            assert!((value - 1.0).abs() < 1e-14);
        }
    }
}
