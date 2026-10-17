//! Plane-stress finite elements of eight nodes, von Mises plasticity with no
//! hardening, and the profile solver they share. Not a solver: the field
//! solver `stress_plate_hole_s235` is built on it, and its tests hold this
//! file to exact solutions of elasticity and plasticity.
//!
//! **The element.** The eight-node serendipity quadrilateral, isoparametric,
//! integrated by the 3 x 3 Gauss rule. Its displacement is quadratic along
//! each edge, so a curved edge whose nodes lie on a circle follows it to the
//! square of the element's length.
//!
//! **The material.** Plane stress, `sigma_zz = tau_xz = tau_yz = 0`, with
//! Hooke's law `sigma = C (epsilon - epsilon_p)` and the von Mises yield
//! condition `f = (sigma_xx^2 - sigma_xx sigma_yy + sigma_yy^2 + 3 tau_xy^2)
//! - sigma_y^2 <= 0` with no hardening: the yield plateau of a mild steel.
//! The flow is associated: `d epsilon_p = d gamma P sigma`, `P` the deviator
//! in the plane-stress vector form. A step is integrated by the closest-point
//! return of Simo and Taylor (Comput. Methods Appl. Mech. Engrg. 48 (1985)
//! 101; Simo and Hughes, Computational Inelasticity, 1998, section 3.4): the
//! plastic multiplier solves one scalar equation, convex and falling, by
//! Newton's method from zero, and the tangent of the step is the consistent
//! one, `Xi - (Xi P sigma)(Xi P sigma)^T / (sigma^T P Xi P sigma)` with
//! `Xi = (C^-1 + d gamma P)^-1`. Vectors are `[xx, yy, xy]`, the strain's
//! third entry the engineering shear `2 epsilon_xy`.
//!
//! **The load path.** The elastic field of the full load is solved once. If
//! it stays below yield it is the answer. Otherwise it is scaled to the load
//! at which the first integration point reaches yield, exactly, and the load
//! is carried on in steps, each settled by Newton's method on the nodal
//! forces with the consistent tangent factored directly. A step that does
//! not settle is halved, up to a set number of times.
//!
//! **What is held.** At every Newton iteration the largest nodal force
//! imbalance of a free node, over the largest nodal force of the field's
//! boundaries (load and reactions). At every settled step the energy ledger:
//! the work of the boundary forces, each step's displacement increment
//! against the mean of the forces at its two ends (the trapezoid), equals the
//! strain energy stored plus the plastic dissipation, each step's plastic
//! strain increment against the mean of the stresses at its two ends. With
//! Hooke's law at both ends of a step the two sides are the same sum, so the
//! ledger closes to rounding and to the imbalance; a move of the field that
//! either side does not book breaks it. With no yielding and one step from
//! rest it is Clapeyron's theorem: the strain energy is half the final forces
//! times the displacement, the work of the forces rising from zero.

use genesis_kernel::math;
/// The Gauss abscissa `sqrt(3/5)`.
pub const GAUSS_POINT: f64 = 0.7745966692414834;
/// The three Gauss abscissae and weights.
pub const GAUSS: [f64; 3] = [-GAUSS_POINT, 0.0, GAUSS_POINT];
pub const GAUSS_WEIGHTS: [f64; 3] = [5.0 / 9.0, 8.0 / 9.0, 5.0 / 9.0];
/// Integration points of one element.
pub const POINTS_PER_ELEMENT: usize = 9;

/// The natural coordinates of the eight nodes: four corners counter-clockwise
/// from `(-1, -1)`, then the four mid-sides from the bottom one.
pub const NODE_XI: [f64; 8] = [-1.0, 1.0, 1.0, -1.0, 0.0, 1.0, 0.0, -1.0];
pub const NODE_ETA: [f64; 8] = [-1.0, -1.0, 1.0, 1.0, -1.0, 0.0, 1.0, 0.0];

/// Newton iterations of the return to the yield surface: the equation is
/// convex and falling, and Newton's method from a lower bound of its root
/// converges from below in a few.
pub const RETURN_MAX_ITERATIONS: u32 = 60;

/// The shape functions and their derivatives in natural coordinates.
pub fn shape(xi: f64, eta: f64) -> ([f64; 8], [f64; 8], [f64; 8]) {
    let mut n = [0.0; 8];
    let mut dxi = [0.0; 8];
    let mut deta = [0.0; 8];
    for i in 0..4 {
        let (a, b) = (NODE_XI[i], NODE_ETA[i]);
        n[i] = 0.25 * (1.0 + a * xi) * (1.0 + b * eta) * (a * xi + b * eta - 1.0);
        dxi[i] = 0.25 * a * (1.0 + b * eta) * (2.0 * a * xi + b * eta);
        deta[i] = 0.25 * b * (1.0 + a * xi) * (a * xi + 2.0 * b * eta);
    }
    for i in [4, 6] {
        let b = NODE_ETA[i];
        n[i] = 0.5 * (1.0 - xi * xi) * (1.0 + b * eta);
        dxi[i] = -xi * (1.0 + b * eta);
        deta[i] = 0.5 * b * (1.0 - xi * xi);
    }
    for i in [5, 7] {
        let a = NODE_XI[i];
        n[i] = 0.5 * (1.0 + a * xi) * (1.0 - eta * eta);
        dxi[i] = 0.5 * a * (1.0 - eta * eta);
        deta[i] = -eta * (1.0 + a * xi);
    }
    (n, dxi, deta)
}

/// The weights that carry values at the 3 x 3 Gauss points of an element to
/// a point `(xi, eta)` of it: the biquadratic through the nine values.
pub fn gauss_interpolation(xi: f64, eta: f64) -> [f64; 9] {
    let g2 = GAUSS_POINT * GAUSS_POINT;
    let line = |s: f64| -> [f64; 3] {
        [s * (s - GAUSS_POINT) / (2.0 * g2), (g2 - s * s) / g2, s * (s + GAUSS_POINT) / (2.0 * g2)]
    };
    let (a, b) = (line(xi), line(eta));
    let mut w = [0.0; 9];
    for j in 0..3 {
        for i in 0..3 {
            w[j * 3 + i] = a[i] * b[j];
        }
    }
    w
}

/// Nodes and elements. Each element lists its eight nodes in the order of
/// [`NODE_XI`].
#[derive(Clone, Debug, PartialEq)]
pub struct Mesh {
    pub x: Vec<f64>,
    pub y: Vec<f64>,
    pub elements: Vec<[usize; 8]>,
}

/// The constants of the material.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Material {
    pub youngs_modulus: f64,
    pub poisson_ratio: f64,
    /// The von Mises stress at which the material yields. `None`: it does
    /// not yield (used by tests and by the elastic field reported beside).
    pub yield_strength: Option<f64>,
}

impl Material {
    /// Plane-stress Hooke matrix.
    pub fn hooke(&self) -> [[f64; 3]; 3] {
        let (e, nu) = (self.youngs_modulus, self.poisson_ratio);
        let c = e / (1.0 - nu * nu);
        [[c, c * nu, 0.0], [c * nu, c, 0.0], [0.0, 0.0, c * (1.0 - nu) / 2.0]]
    }

    /// `C^-1 sigma`: the elastic strain of a stress.
    pub fn compliance(&self, s: &[f64; 3]) -> [f64; 3] {
        let (e, nu) = (self.youngs_modulus, self.poisson_ratio);
        [(s[0] - nu * s[1]) / e, (s[1] - nu * s[0]) / e, 2.0 * (1.0 + nu) * s[2] / e]
    }
}

/// The von Mises stress of a plane stress.
pub fn von_mises(s: &[f64; 3]) -> f64 {
    (s[0] * s[0] - s[0] * s[1] + s[1] * s[1] + 3.0 * s[2] * s[2]).max(0.0).sqrt()
}

trait SubOne {
    fn sub_one(self) -> f64;
}

impl SubOne for f64 {
    fn sub_one(self) -> f64 {
        self - 1.0
    }
}

fn dot3(a: &[f64; 3], b: &[f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn mat_vec(m: &[[f64; 3]; 3], v: &[f64; 3]) -> [f64; 3] {
    [dot3(&m[0], v), dot3(&m[1], v), dot3(&m[2], v)]
}

/// `P sigma`: the deviator in the plane-stress vector form, so that
/// `sigma^T P sigma` is two thirds of the von Mises stress squared.
fn deviator(s: &[f64; 3]) -> [f64; 3] {
    [(2.0 * s[0] - s[1]) / 3.0, (2.0 * s[1] - s[0]) / 3.0, 2.0 * s[2]]
}

/// What one integration point returns for a total strain.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PointState {
    pub stress: [f64; 3],
    pub plastic_strain: [f64; 3],
    pub equivalent_plastic_strain: f64,
    /// The plastic multiplier of the step; zero for an elastic step.
    pub multiplier: f64,
}

/// Why a return to the yield surface failed.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ReturnFailure {
    NotFinite,
    NotConverged,
}

/// The closest-point return of a step: from the plastic strain of the last
/// settled state and the trial total strain, the stress, the new plastic
/// strain, and the consistent tangent. `plastic_tangent_at_rest` asks for
/// the continuum tangent of plastic loading where the trial is on the yield
/// surface or inside it, for the first iteration of a step at a point that
/// yielded in the last.
pub fn return_map(
    material: &Material,
    strain: &[f64; 3],
    old_plastic: &[f64; 3],
    old_equivalent: f64,
    plastic_tangent_at_rest: bool,
) -> Result<(PointState, [[f64; 3]; 3]), ReturnFailure> {
    let c = material.hooke();
    let elastic = [strain[0] - old_plastic[0], strain[1] - old_plastic[1], strain[2] - old_plastic[2]];
    let trial = mat_vec(&c, &elastic);
    if !(trial.iter().all(|v| v.is_finite())) {
        return Err(ReturnFailure::NotFinite);
    }
    let rest = PointState { stress: trial, plastic_strain: *old_plastic, equivalent_plastic_strain: old_equivalent, multiplier: 0.0 };
    let Some(yield_strength) = material.yield_strength else {
        return Ok((rest, c));
    };
    let (e, nu) = (material.youngs_modulus, material.poisson_ratio);
    let sum = trial[0] + trial[1];
    let diff = trial[0] - trial[1];
    let a1 = sum * sum / 12.0;
    let a2 = diff * diff / 4.0 + trial[2] * trial[2];
    let limit = yield_strength * yield_strength / 3.0;
    if a1 + a2 <= limit {
        if plastic_tangent_at_rest && a1 + a2 > 0.0 {
            let n = mat_vec(&c, &deviator(&trial));
            let denominator = dot3(&deviator(&trial), &n);
            let mut tangent = c;
            for i in 0..3 {
                for j in 0..3 {
                    tangent[i][j] -= n[i] * n[j] / denominator;
                }
            }
            return Ok((rest, tangent));
        }
        return Ok((rest, c));
    }
    let k1 = e / (3.0 * (1.0 - nu));
    let k2 = e / (1.0 + nu);
    // A lower bound of the multiplier: with the faster of the two rates for
    // both terms, the function would reach zero there first. The function
    // is not below zero at it, and Newton's method from it stays below the
    // root and converges to it.
    let mut gamma = ((a1 + a2) / limit).sqrt().max(1.0).sub_one() / k1.max(k2);
    let mut converged = false;
    for _ in 0..RETURN_MAX_ITERATIONS {
        let d1 = 1.0 + k1 * gamma;
        let d2 = 1.0 + k2 * gamma;
        let f = a1 / (d1 * d1) + a2 / (d2 * d2) - limit;
        let slope = -2.0 * a1 * k1 / (d1 * d1 * d1) - 2.0 * a2 * k2 / (d2 * d2 * d2);
        if !(f.is_finite() && slope.is_finite()) || slope >= 0.0 {
            return Err(ReturnFailure::NotFinite);
        }
        if f.abs() <= 1.0e-14 * limit {
            converged = true;
            break;
        }
        let step = -f / slope;
        gamma += step;
        if step.abs() <= 1.0e-14 * gamma.abs() {
            converged = true;
            break;
        }
    }
    if !converged {
        return Err(ReturnFailure::NotConverged);
    }
    let d1 = 1.0 + k1 * gamma;
    let d2 = 1.0 + k2 * gamma;
    let new_sum = sum / d1;
    let new_diff = diff / d2;
    let stress = [0.5 * (new_sum + new_diff), 0.5 * (new_sum - new_diff), trial[2] / d2];
    let elastic_strain = material.compliance(&stress);
    let plastic = [strain[0] - elastic_strain[0], strain[1] - elastic_strain[1], strain[2] - elastic_strain[2]];
    let equivalent = old_equivalent + gamma * (2.0 / 3.0) * von_mises(&stress);
    // Xi in the common eigenbasis of C and P.
    let shear = e / (2.0 * (1.0 + nu));
    let xi1 = e / (1.0 - nu) / d1;
    let xi2 = 2.0 * shear / d2;
    let xi3 = shear / d2;
    let xi = [[0.5 * (xi1 + xi2), 0.5 * (xi1 - xi2), 0.0], [0.5 * (xi1 - xi2), 0.5 * (xi1 + xi2), 0.0], [0.0, 0.0, xi3]];
    let n = mat_vec(&xi, &deviator(&stress));
    let denominator = dot3(&deviator(&stress), &n);
    let mut tangent = xi;
    for i in 0..3 {
        for j in 0..3 {
            tangent[i][j] -= n[i] * n[j] / denominator;
        }
    }
    if !(stress.iter().chain(plastic.iter()).all(|v| v.is_finite()) && equivalent.is_finite()) {
        return Err(ReturnFailure::NotFinite);
    }
    Ok((PointState { stress, plastic_strain: plastic, equivalent_plastic_strain: equivalent, multiplier: gamma }, tangent))
}

/// The dot product of two slices of one length, summed in eight fixed
/// lanes and then lane by lane: the same bits on every run, and a loop the
/// compiler can carry out several products at a time.
fn dot(a: &[f64], b: &[f64]) -> f64 {
    let n = a.len().min(b.len());
    let (a, b) = (&a[..n], &b[..n]);
    let mut lanes = [0.0_f64; 8];
    let chunks = n / 8;
    for c in 0..chunks {
        let (x, y) = (&a[c * 8..c * 8 + 8], &b[c * 8..c * 8 + 8]);
        for l in 0..8 {
            lanes[l] += x[l] * y[l];
        }
    }
    let mut tail = 0.0;
    for k in chunks * 8..n {
        tail += a[k] * b[k];
    }
    ((lanes[0] + lanes[1]) + (lanes[2] + lanes[3])) + ((lanes[4] + lanes[5]) + (lanes[6] + lanes[7])) + tail
}

/// A symmetric matrix stored by the profile of its lower triangle, and its
/// factors `L D L^T` in place.
pub struct Profile {
    n: usize,
    first: Vec<usize>,
    start: Vec<usize>,
    a: Vec<f64>,
}

impl Profile {
    /// `first[i]`: the first column of row `i` that may be filled.
    pub fn new(first: Vec<usize>) -> Self {
        let n = first.len();
        let mut start = Vec::with_capacity(n + 1);
        let mut at = 0;
        for (i, f) in first.iter().enumerate() {
            start.push(at);
            at += i - f + 1;
        }
        start.push(at);
        Profile { n, first, start, a: vec![0.0; at] }
    }

    pub fn len(&self) -> usize {
        self.n
    }

    pub fn is_empty(&self) -> bool {
        self.n == 0
    }

    /// Entries stored.
    pub fn stored(&self) -> usize {
        self.a.len()
    }

    pub fn clear(&mut self) {
        self.a.fill(0.0);
    }

    /// Adds to entry `(i, j)` of the lower triangle, `j <= i`.
    pub fn add(&mut self, i: usize, j: usize, value: f64) {
        debug_assert!(j <= i && j >= self.first[i]);
        self.a[self.start[i] + j - self.first[i]] += value;
    }

    /// Factors in place. `Err(row)` at the first pivot that is not positive
    /// and finite: the matrix is not positive definite.
    pub fn factor(&mut self) -> Result<(), usize> {
        for i in 0..self.n {
            let fi = self.first[i];
            let si = self.start[i];
            for j in fi..i {
                let fj = self.first[j];
                let sj = self.start[j];
                let k0 = fi.max(fj);
                if k0 < j {
                    let (head, tail) = self.a.split_at_mut(si);
                    let row_j = &head[sj + k0 - fj..sj + j - fj];
                    let row_i = &tail[k0 - fi..j - fi];
                    let s = dot(row_i, row_j);
                    tail[j - fi] -= s;
                }
            }
            let mut diagonal = self.a[si + i - fi];
            for j in fi..i {
                let g = self.a[si + j - fi];
                let l = g / self.a[self.start[j] + j - self.first[j]];
                diagonal -= g * l;
                self.a[si + j - fi] = l;
            }
            if !(diagonal > 0.0 && diagonal.is_finite()) {
                return Err(i);
            }
            self.a[si + i - fi] = diagonal;
        }
        Ok(())
    }

    /// Solves `A x = b` in place of `b`, from the factors.
    pub fn solve(&self, b: &mut [f64]) {
        for i in 0..self.n {
            let fi = self.first[i];
            let si = self.start[i];
            let row = &self.a[si..si + i - fi];
            let s = dot(row, &b[fi..i]);
            b[i] -= s;
        }
        for i in 0..self.n {
            b[i] /= self.a[self.start[i] + i - self.first[i]];
        }
        for i in (0..self.n).rev() {
            let fi = self.first[i];
            let si = self.start[i];
            let xi = b[i];
            if xi != 0.0 {
                for (k, l) in (fi..i).zip(&self.a[si..si + i - fi]) {
                    b[k] -= l * xi;
                }
            }
        }
    }
}

/// The reverse Cuthill-McKee order of the nodes: neighbours by their count
/// of neighbours, then by number, from a node at the far end of the mesh.
pub fn node_order(mesh: &Mesh) -> Vec<usize> {
    let count = mesh.x.len();
    let mut neighbours: Vec<Vec<usize>> = vec![Vec::new(); count];
    for element in &mesh.elements {
        for &a in element {
            for &b in element {
                if a != b {
                    neighbours[a].push(b);
                }
            }
        }
    }
    for list in neighbours.iter_mut() {
        list.sort_unstable();
        list.dedup();
    }
    let degree = |node: usize| neighbours[node].len();
    let used: Vec<bool> = {
        let mut used = vec![false; count];
        for element in &mesh.elements {
            for &a in element {
                used[a] = true;
            }
        }
        used
    };
    let levels = |start: usize| -> Vec<Vec<usize>> {
        let mut seen = vec![false; count];
        let mut out = vec![vec![start]];
        seen[start] = true;
        loop {
            let mut next = Vec::new();
            for &node in out.last().expect("a level") {
                for &other in &neighbours[node] {
                    if !seen[other] {
                        seen[other] = true;
                        next.push(other);
                    }
                }
            }
            if next.is_empty() {
                return out;
            }
            out.push(next);
        }
    };
    let mut order = Vec::with_capacity(count);
    let mut placed = vec![false; count];
    loop {
        let Some(mut start) = (0..count).filter(|&n| used[n] && !placed[n]).min_by_key(|&n| (degree(n), n)) else {
            break;
        };
        // One pass toward a node of greatest distance.
        let last = levels(start);
        if let Some(far) = last.last().and_then(|level| level.iter().copied().min_by_key(|&n| (degree(n), n))) {
            start = far;
        }
        let mut queue = std::collections::VecDeque::new();
        queue.push_back(start);
        placed[start] = true;
        let mut block = Vec::new();
        while let Some(node) = queue.pop_front() {
            block.push(node);
            let mut next: Vec<usize> = neighbours[node].iter().copied().filter(|&o| !placed[o]).collect();
            next.sort_by_key(|&o| (degree(o), o));
            for o in next {
                placed[o] = true;
                queue.push_back(o);
            }
        }
        block.reverse();
        order.extend(block);
    }
    order
}

/// One integration point's geometry: the derivatives of the shape functions
/// in `x` and `y` and the weight times the Jacobian.
#[derive(Clone, Copy, Debug)]
pub struct PointGeometry {
    pub dx: [f64; 8],
    pub dy: [f64; 8],
    pub weight: f64,
    pub x: f64,
    pub y: f64,
}

/// Why the geometry of a mesh was refused.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum MeshFailure {
    /// An element's Jacobian is not positive at an integration point.
    Inverted { element: usize },
}

/// The geometry of every integration point, element by element.
pub fn point_geometry(mesh: &Mesh) -> Result<Vec<PointGeometry>, MeshFailure> {
    let mut out = Vec::with_capacity(mesh.elements.len() * POINTS_PER_ELEMENT);
    for (index, element) in mesh.elements.iter().enumerate() {
        for (j, eta) in GAUSS.iter().enumerate() {
            for (i, xi) in GAUSS.iter().enumerate() {
                let (n, dxi, deta) = shape(*xi, *eta);
                let (mut x_xi, mut x_eta, mut y_xi, mut y_eta, mut x, mut y) = (0.0, 0.0, 0.0, 0.0, 0.0, 0.0);
                for k in 0..8 {
                    let (px, py) = (mesh.x[element[k]], mesh.y[element[k]]);
                    x_xi += dxi[k] * px;
                    x_eta += deta[k] * px;
                    y_xi += dxi[k] * py;
                    y_eta += deta[k] * py;
                    x += n[k] * px;
                    y += n[k] * py;
                }
                let det = x_xi * y_eta - x_eta * y_xi;
                if !(det > 0.0 && det.is_finite()) {
                    return Err(MeshFailure::Inverted { element: index });
                }
                let mut dx = [0.0; 8];
                let mut dy = [0.0; 8];
                for k in 0..8 {
                    dx[k] = (y_eta * dxi[k] - y_xi * deta[k]) / det;
                    dy[k] = (-x_eta * dxi[k] + x_xi * deta[k]) / det;
                }
                out.push(PointGeometry { dx, dy, weight: GAUSS_WEIGHTS[i] * GAUSS_WEIGHTS[j] * det, x, y });
            }
        }
    }
    Ok(out)
}

/// What is asked of a field: the mesh, the displacements held at nodes
/// (each a degree of freedom, `2 node + 0` for `x` and `+ 1` for `y`, with
/// its value at the full load), and the nodal forces of the full load, per
/// unit thickness.
pub struct Problem<'a> {
    pub mesh: &'a Mesh,
    pub held: Vec<(usize, f64)>,
    pub load: Vec<f64>,
}

/// The numerical choices of a load path.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PathNumerics {
    /// The largest load step after first yield, as a fraction of the full load.
    pub largest_step: f64,
    /// The largest nodal force imbalance of a settled step, over the largest
    /// boundary nodal force.
    pub imbalance_tolerance: f64,
    /// An imbalance above this ends the step as not settled.
    pub imbalance_cap: f64,
    pub max_newton_iterations: u32,
    /// How many times a step that does not settle may be halved, over the path.
    pub max_step_halvings: u32,
    /// The energy ledger's largest residual, as a fraction of the work.
    pub energy_tolerance: f64,
    /// A fault put in on purpose by the tests; [`LedgerFault::None`] when served.
    pub fault: LedgerFault,
}

/// A move of the field put in on purpose by the tests, that the energy ledger
/// does not book, to show that it is held.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LedgerFault {
    None,
    /// The work of a step against its end forces alone, the rectangle: the
    /// half of the step's force increment times its displacement increment is
    /// booked nowhere.
    RectangleWork,
    /// The plastic dissipation of one yielding integration point, at every
    /// step, booked nowhere.
    UnbookedDissipation,
}

/// One settled step of the path, for the record.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StepRecord {
    pub load_factor: f64,
    pub newton_iterations: u32,
    pub imbalance: f64,
    pub energy_residual: f64,
    pub largest_strain: f64,
}

/// A settled field.
#[derive(Clone, Debug, PartialEq)]
pub struct Settled {
    /// Displacement at every degree of freedom.
    pub displacement: Vec<f64>,
    /// The state at every integration point, element by element.
    pub points: Vec<PointState>,
    /// The load factor reached: 1 for the full load.
    pub load_factor: f64,
    /// The load factor at which the first integration point reached yield;
    /// `None` when the full load leaves every point below it.
    pub first_yield_factor: Option<f64>,
    /// The largest von Mises stress of the elastic field of the full load,
    /// over every integration point: what the field would carry if the
    /// material did not yield.
    pub elastic_peak_von_mises: f64,
    /// The elastic field of the full load at every degree of freedom.
    pub elastic_displacement: Vec<f64>,
    pub steps: Vec<StepRecord>,
    pub newton_iterations: u32,
    /// The largest imbalance of the settled steps.
    pub imbalance: f64,
    /// The work of the boundary forces, by the trapezoid over each step, per
    /// unit thickness.
    pub boundary_work: f64,
    pub strain_energy: f64,
    /// The plastic dissipation, by the trapezoid over each step.
    pub plastic_dissipation: f64,
    /// The largest energy residual of the settled steps.
    pub energy_residual: f64,
    /// Reaction at every held degree of freedom, in the order of `held`.
    pub reactions: Vec<f64>,
}

/// Why a path stopped.
#[derive(Clone, Debug, PartialEq)]
pub enum PathFailure {
    Mesh(MeshFailure),
    /// A number was NaN or infinite.
    NotFinite,
    /// The elastic stiffness had a pivot that is not positive.
    ElasticSingular { row: usize },
    /// A return to the yield surface did not converge.
    ReturnNotConverged,
    /// After the allowed halvings a step still did not settle: the last
    /// settled load factor.
    NoEquilibrium { load_factor: f64, halvings: u32 },
    /// The energy ledger broke at a settled step.
    EnergyLedger { load_factor: f64, residual: f64 },
    /// The caller's stop: a settled step past a limit the caller sets.
    Stopped { load_factor: f64, value: f64 },
}

struct System {
    /// Equation of each degree of freedom, `usize::MAX` when held.
    equation: Vec<usize>,
    first: Vec<usize>,
    count: usize,
}

fn system(mesh: &Mesh, held: &[(usize, f64)]) -> System {
    let order = node_order(mesh);
    let mut is_held = vec![false; mesh.x.len() * 2];
    for (dof, _) in held {
        is_held[*dof] = true;
    }
    let mut equation = vec![usize::MAX; mesh.x.len() * 2];
    let mut count = 0;
    for node in order {
        for c in 0..2 {
            let dof = 2 * node + c;
            if !is_held[dof] {
                equation[dof] = count;
                count += 1;
            }
        }
    }
    let mut first: Vec<usize> = (0..count).collect();
    for element in &mesh.elements {
        let eqs: Vec<usize> = element.iter().flat_map(|&n| [equation[2 * n], equation[2 * n + 1]]).filter(|&e| e != usize::MAX).collect();
        let low = eqs.iter().copied().min().unwrap_or(0);
        for &e in &eqs {
            first[e] = first[e].min(low);
        }
    }
    System { equation, first, count }
}

/// The strain at one integration point.
fn strain_at(element: &[usize; 8], g: &PointGeometry, u: &[f64]) -> [f64; 3] {
    let mut e = [0.0; 3];
    for k in 0..8 {
        let (ux, uy) = (u[2 * element[k]], u[2 * element[k] + 1]);
        e[0] += g.dx[k] * ux;
        e[1] += g.dy[k] * uy;
        e[2] += g.dy[k] * ux + g.dx[k] * uy;
    }
    e
}

/// The nodal forces of the stresses, at every degree of freedom.
fn internal_forces(mesh: &Mesh, geometry: &[PointGeometry], stresses: &[[f64; 3]]) -> Vec<f64> {
    let mut f = vec![0.0; mesh.x.len() * 2];
    for (index, element) in mesh.elements.iter().enumerate() {
        for p in 0..POINTS_PER_ELEMENT {
            let g = &geometry[index * POINTS_PER_ELEMENT + p];
            let s = &stresses[index * POINTS_PER_ELEMENT + p];
            for k in 0..8 {
                f[2 * element[k]] += g.weight * (g.dx[k] * s[0] + g.dy[k] * s[2]);
                f[2 * element[k] + 1] += g.weight * (g.dy[k] * s[1] + g.dx[k] * s[2]);
            }
        }
    }
    f
}

fn assemble(mesh: &Mesh, geometry: &[PointGeometry], tangents: &[[[f64; 3]; 3]], sys: &System, k: &mut Profile) {
    k.clear();
    for (index, element) in mesh.elements.iter().enumerate() {
        let eqs: [usize; 16] = std::array::from_fn(|d| sys.equation[2 * element[d / 2] + d % 2]);
        let mut ke = [[0.0_f64; 16]; 16];
        for p in 0..POINTS_PER_ELEMENT {
            let g = &geometry[index * POINTS_PER_ELEMENT + p];
            let d = &tangents[index * POINTS_PER_ELEMENT + p];
            // Rows of B for each degree of freedom: [e_xx, e_yy, g_xy].
            let mut b = [[0.0_f64; 3]; 16];
            for n in 0..8 {
                b[2 * n] = [g.dx[n], 0.0, g.dy[n]];
                b[2 * n + 1] = [0.0, g.dy[n], g.dx[n]];
            }
            for a in 0..16 {
                let db = mat_vec(d, &b[a]);
                let db = [db[0] * g.weight, db[1] * g.weight, db[2] * g.weight];
                for c in 0..=a {
                    ke[a][c] += dot3(&b[c], &db);
                }
            }
        }
        for a in 0..16 {
            for c in 0..=a {
                let (ea, ec) = (eqs[a], eqs[c]);
                if ea == usize::MAX || ec == usize::MAX {
                    continue;
                }
                if ea >= ec {
                    k.add(ea, ec, ke[a][c]);
                } else {
                    k.add(ec, ea, ke[a][c]);
                }
            }
        }
    }
}

/// The force each held degree of freedom exerts on the field when the held
/// values move, carried to the right-hand side: `- K_fh u_h`.
fn held_forces(mesh: &Mesh, geometry: &[PointGeometry], tangents: &[[[f64; 3]; 3]], sys: &System, u_held: &[f64]) -> Vec<f64> {
    let mut rhs = vec![0.0; sys.count];
    for (index, element) in mesh.elements.iter().enumerate() {
        let dofs: [usize; 16] = std::array::from_fn(|d| 2 * element[d / 2] + d % 2);
        if dofs.iter().all(|&d| sys.equation[d] != usize::MAX) {
            continue;
        }
        let mut ue = [0.0; 16];
        for (a, &d) in dofs.iter().enumerate() {
            if sys.equation[d] == usize::MAX {
                ue[a] = u_held[d];
            }
        }
        if ue.iter().all(|v| *v == 0.0) {
            continue;
        }
        for p in 0..POINTS_PER_ELEMENT {
            let g = &geometry[index * POINTS_PER_ELEMENT + p];
            let d = &tangents[index * POINTS_PER_ELEMENT + p];
            let mut e = [0.0; 3];
            for n in 0..8 {
                e[0] += g.dx[n] * ue[2 * n];
                e[1] += g.dy[n] * ue[2 * n + 1];
                e[2] += g.dy[n] * ue[2 * n] + g.dx[n] * ue[2 * n + 1];
            }
            let s = mat_vec(d, &e);
            for n in 0..8 {
                for (c, value) in [(0, g.dx[n] * s[0] + g.dy[n] * s[2]), (1, g.dy[n] * s[1] + g.dx[n] * s[2])] {
                    let eq = sys.equation[2 * element[n] + c];
                    if eq != usize::MAX {
                        rhs[eq] -= g.weight * value;
                    }
                }
            }
        }
    }
    rhs
}

/// The energy of each part of the ledger, per unit thickness.
fn strain_energy(material: &Material, geometry: &[PointGeometry], points: &[PointState]) -> f64 {
    geometry.iter().zip(points).map(|(g, p)| 0.5 * g.weight * dot3(&p.stress, &material.compliance(&p.stress))).sum()
}

/// Settles the field along the path from rest to the full load. `stop`
/// is asked after every settled step with the field so far, and a `Some`
/// ends the path as [`PathFailure::Stopped`] with its value.
pub fn settle(
    problem: &Problem,
    material: &Material,
    numerics: &PathNumerics,
    stop: &dyn Fn(f64, &[f64], &[PointState]) -> Option<f64>,
) -> Result<Settled, PathFailure> {
    let mesh = problem.mesh;
    let geometry = point_geometry(mesh).map_err(PathFailure::Mesh)?;
    let dofs = mesh.x.len() * 2;
    let sys = system(mesh, &problem.held);
    let points_count = geometry.len();
    let hooke = material.hooke();
    let mut k = Profile::new(sys.first.clone());

    // The elastic field of the full load.
    let elastic_tangents = vec![hooke; points_count];
    assemble(mesh, &geometry, &elastic_tangents, &sys, &mut k);
    k.factor().map_err(|row| PathFailure::ElasticSingular { row })?;
    let mut u_full_held = vec![0.0; dofs];
    for (dof, value) in &problem.held {
        u_full_held[*dof] = *value;
    }
    let mut rhs = held_forces(mesh, &geometry, &elastic_tangents, &sys, &u_full_held);
    for d in 0..dofs {
        let eq = sys.equation[d];
        if eq != usize::MAX {
            rhs[eq] += problem.load[d];
        }
    }
    k.solve(&mut rhs);
    let mut elastic = u_full_held.clone();
    for d in 0..dofs {
        let eq = sys.equation[d];
        if eq != usize::MAX {
            elastic[d] = rhs[eq];
        }
    }
    if elastic.iter().any(|v| !v.is_finite()) {
        return Err(PathFailure::NotFinite);
    }
    let elastic_stress: Vec<[f64; 3]> = mesh
        .elements
        .iter()
        .enumerate()
        .flat_map(|(index, element)| {
            let geometry = &geometry;
            let elastic = &elastic;
            (0..POINTS_PER_ELEMENT).map(move |p| mat_vec(&hooke, &strain_at(element, &geometry[index * POINTS_PER_ELEMENT + p], elastic)))
        })
        .collect();
    let elastic_peak = elastic_stress.iter().map(von_mises).fold(0.0_f64, f64::max);
    if !elastic_peak.is_finite() {
        return Err(PathFailure::NotFinite);
    }
    let first_yield_factor = match material.yield_strength {
        Some(yield_strength) if elastic_peak > yield_strength => Some(yield_strength / elastic_peak),
        _ => None,
    };
    let elastic_factor = first_yield_factor.unwrap_or(1.0);

    // The elastic part of the path, in one step from rest.
    let mut u: Vec<f64> = elastic.iter().map(|v| v * elastic_factor).collect();
    let mut points: Vec<PointState> = elastic_stress
        .iter()
        .map(|s| PointState { stress: [s[0] * elastic_factor, s[1] * elastic_factor, s[2] * elastic_factor], plastic_strain: [0.0; 3], equivalent_plastic_strain: 0.0, multiplier: 0.0 })
        .collect();
    let mut boundary_work = 0.0;
    let mut plastic_dissipation = 0.0;
    let mut steps = Vec::new();
    let mut newton_iterations = 0_u32;
    let mut largest_imbalance = 0.0_f64;
    let mut largest_energy_residual = 0.0_f64;

    let stresses: Vec<[f64; 3]> = points.iter().map(|p| p.stress).collect();
    let f_int = internal_forces(mesh, &geometry, &stresses);
    let scale_of = |f_int: &[f64], factor: f64| -> f64 {
        let mut scale = 0.0_f64;
        for d in 0..dofs {
            if sys.equation[d] == usize::MAX {
                scale = scale.max(f_int[d].abs());
            } else {
                scale = scale.max((problem.load[d] * factor).abs());
            }
        }
        scale
    };
    let imbalance_of = |f_int: &[f64], factor: f64| -> f64 {
        let scale = scale_of(f_int, factor);
        let mut worst = 0.0_f64;
        for d in 0..dofs {
            if sys.equation[d] != usize::MAX {
                let r = f_int[d] - problem.load[d] * factor;
                if !r.is_finite() {
                    return f64::NAN;
                }
                worst = worst.max(r.abs());
            }
        }
        if scale > 0.0 { worst / scale } else { worst }
    };
    let imbalance = imbalance_of(&f_int, elastic_factor);
    if !imbalance.is_finite() {
        return Err(PathFailure::NotFinite);
    }
    largest_imbalance = largest_imbalance.max(imbalance);
    // The force on the field's boundary at every freedom: the load at a free
    // one, the reaction at a held one.
    let boundary_forces = |f_int: &[f64], factor: f64| -> Vec<f64> {
        (0..dofs).map(|d| if sys.equation[d] == usize::MAX { f_int[d] } else { problem.load[d] * factor }).collect()
    };
    // Work of the boundary forces over a step: the displacement increment
    // against the mean of the forces at the step's two ends.
    let work_of = |before: &[f64], after: &[f64], du: &[f64]| -> f64 {
        let mut w = 0.0;
        for d in 0..dofs {
            let force = if numerics.fault == LedgerFault::RectangleWork { after[d] } else { 0.5 * (before[d] + after[d]) };
            w += force * du[d];
        }
        w
    };
    // From rest: no force before the first step.
    let mut forces = boundary_forces(&f_int, elastic_factor);
    boundary_work += work_of(&vec![0.0; dofs], &forces, &u);
    let energy = strain_energy(material, &geometry, &points);
    let residual_of = |work: f64, stored: f64, plastic: f64| -> f64 {
        if work == 0.0 && stored == 0.0 { 0.0 } else { (work - (stored + plastic)).abs() / work.abs().max(stored.abs()) }
    };
    let energy_residual = residual_of(boundary_work, energy, plastic_dissipation);
    if !energy_residual.is_finite() {
        return Err(PathFailure::NotFinite);
    }
    if energy_residual > numerics.energy_tolerance {
        return Err(PathFailure::EnergyLedger { load_factor: elastic_factor, residual: energy_residual });
    }
    largest_energy_residual = largest_energy_residual.max(energy_residual);
    let largest_strain = largest_point_strain(material, &points);
    steps.push(StepRecord { load_factor: elastic_factor, newton_iterations: 0, imbalance, energy_residual, largest_strain });
    if let Some(value) = stop(elastic_factor, &u, &points) {
        return Err(PathFailure::Stopped { load_factor: elastic_factor, value });
    }

    // The plastic part of the path.
    let mut factor = elastic_factor;
    let mut step = if first_yield_factor.is_some() {
        let span = 1.0 - elastic_factor;
        span / (span / numerics.largest_step).ceil().max(1.0)
    } else {
        0.0
    };
    let mut halvings = 0_u32;
    let mut was_plastic = vec![false; points_count];
    while factor < 1.0 {
        let target = if 1.0 - factor <= step * (1.0 + 1e-12) { 1.0 } else { factor + step };
        let mut trial_u = u.clone();
        for (dof, value) in &problem.held {
            trial_u[*dof] = value * target;
        }
        let mut iterations = 0_u32;
        let mut tangents = vec![hooke; points_count];
        let mut trial_points = points.clone();
        let outcome: Result<f64, ()> = loop {
            // The state of every point at the trial displacement.
            let mut failed = false;
            for (index, element) in mesh.elements.iter().enumerate() {
                for p in 0..POINTS_PER_ELEMENT {
                    let at = index * POINTS_PER_ELEMENT + p;
                    let strain = strain_at(element, &geometry[at], &trial_u);
                    let old = &points[at];
                    match return_map(material, &strain, &old.plastic_strain, old.equivalent_plastic_strain, iterations == 0 && was_plastic[at]) {
                        Ok((state, tangent)) => {
                            trial_points[at] = state;
                            tangents[at] = tangent;
                        }
                        Err(ReturnFailure::NotConverged) => return Err(PathFailure::ReturnNotConverged),
                        Err(ReturnFailure::NotFinite) => {
                            failed = true;
                        }
                    }
                }
            }
            if failed {
                break Err(());
            }
            let stresses: Vec<[f64; 3]> = trial_points.iter().map(|p| p.stress).collect();
            let f_int = internal_forces(mesh, &geometry, &stresses);
            let imbalance = imbalance_of(&f_int, target);
            if !imbalance.is_finite() || imbalance > numerics.imbalance_cap {
                break Err(());
            }
            if imbalance <= numerics.imbalance_tolerance && iterations > 0 {
                break Ok(imbalance);
            }
            if iterations >= numerics.max_newton_iterations {
                break Err(());
            }
            assemble(mesh, &geometry, &tangents, &sys, &mut k);
            if k.factor().is_err() {
                break Err(());
            }
            let mut r = vec![0.0; sys.count];
            for d in 0..dofs {
                let eq = sys.equation[d];
                if eq != usize::MAX {
                    r[eq] = problem.load[d] * target - f_int[d];
                }
            }
            k.solve(&mut r);
            for d in 0..dofs {
                let eq = sys.equation[d];
                if eq != usize::MAX {
                    trial_u[d] += r[eq];
                }
            }
            iterations += 1;
            newton_iterations += 1;
        };
        match outcome {
            Ok(imbalance) => {
                let stresses: Vec<[f64; 3]> = trial_points.iter().map(|p| p.stress).collect();
                let f_int = internal_forces(mesh, &geometry, &stresses);
                let du: Vec<f64> = trial_u.iter().zip(&u).map(|(a, b)| a - b).collect();
                let after = boundary_forces(&f_int, target);
                boundary_work += work_of(&forces, &after, &du);
                forces = after;
                let mut left_out = numerics.fault == LedgerFault::UnbookedDissipation;
                for ((g, new), old) in geometry.iter().zip(&trial_points).zip(&points) {
                    let dp = [new.plastic_strain[0] - old.plastic_strain[0], new.plastic_strain[1] - old.plastic_strain[1], new.plastic_strain[2] - old.plastic_strain[2]];
                    if left_out && new.multiplier > 0.0 {
                        left_out = false;
                        continue;
                    }
                    let mean = [0.5 * (old.stress[0] + new.stress[0]), 0.5 * (old.stress[1] + new.stress[1]), 0.5 * (old.stress[2] + new.stress[2])];
                    plastic_dissipation += g.weight * dot3(&mean, &dp);
                }
                for (flag, state) in was_plastic.iter_mut().zip(&trial_points) {
                    *flag = state.multiplier > 0.0;
                }
                u = trial_u;
                points = trial_points;
                factor = target;
                let energy = strain_energy(material, &geometry, &points);
                let energy_residual = residual_of(boundary_work, energy, plastic_dissipation);
                if !energy_residual.is_finite() {
                    return Err(PathFailure::NotFinite);
                }
                if energy_residual > numerics.energy_tolerance {
                    return Err(PathFailure::EnergyLedger { load_factor: factor, residual: energy_residual });
                }
                largest_energy_residual = largest_energy_residual.max(energy_residual);
                largest_imbalance = largest_imbalance.max(imbalance);
                let largest_strain = largest_point_strain(material, &points);
                steps.push(StepRecord { load_factor: factor, newton_iterations: iterations, imbalance, energy_residual, largest_strain });
                if let Some(value) = stop(factor, &u, &points) {
                    return Err(PathFailure::Stopped { load_factor: factor, value });
                }
            }
            Err(()) => {
                if halvings >= numerics.max_step_halvings {
                    return Err(PathFailure::NoEquilibrium { load_factor: factor, halvings });
                }
                halvings += 1;
                step *= 0.5;
            }
        }
    }
    let stresses: Vec<[f64; 3]> = points.iter().map(|p| p.stress).collect();
    let f_int = internal_forces(mesh, &geometry, &stresses);
    let reactions = problem.held.iter().map(|(dof, _)| f_int[*dof]).collect();
    let strain_energy = strain_energy(material, &geometry, &points);
    Ok(Settled {
        displacement: u,
        points,
        load_factor: factor,
        first_yield_factor,
        elastic_peak_von_mises: elastic_peak,
        elastic_displacement: elastic,
        steps,
        newton_iterations,
        imbalance: largest_imbalance,
        boundary_work,
        strain_energy,
        plastic_dissipation,
        energy_residual: largest_energy_residual,
        reactions,
    })
}

/// The largest principal strain in size at any integration point: in the
/// plane, and through the thickness.
pub fn largest_point_strain(material: &Material, points: &[PointState]) -> f64 {
    let mut worst = 0.0_f64;
    for p in points {
        let e = material.compliance(&p.stress);
        let total = [e[0] + p.plastic_strain[0], e[1] + p.plastic_strain[1], e[2] + p.plastic_strain[2]];
        let mean = 0.5 * (total[0] + total[1]);
        let radius = (0.25 * math::powi(total[0] - total[1], 2) + 0.25 * total[2] * total[2]).sqrt();
        // Through the thickness: elastic, and the plastic flow keeps volume.
        let nu = material.poisson_ratio;
        let zz = -nu / material.youngs_modulus * (p.stress[0] + p.stress[1]) - (p.plastic_strain[0] + p.plastic_strain[1]);
        worst = worst.max((mean + radius).abs()).max((mean - radius).abs()).max(zz.abs());
    }
    worst
}

/// The nodal forces of a uniform traction on a straight or curved edge of
/// three nodes `[end, middle, end]`, per unit thickness, into `load`.
pub fn edge_traction(mesh: &Mesh, nodes: [usize; 3], traction: [f64; 2], load: &mut [f64]) {
    // Shape functions along the edge, s from -1 to 1: end, middle, end.
    for (s, w) in GAUSS.iter().zip(GAUSS_WEIGHTS) {
        let n = [0.5 * s * (s - 1.0), 1.0 - s * s, 0.5 * s * (s + 1.0)];
        let dn = [s - 0.5, -2.0 * s, s + 0.5];
        let mut tx = 0.0;
        let mut ty = 0.0;
        for k in 0..3 {
            tx += dn[k] * mesh.x[nodes[k]];
            ty += dn[k] * mesh.y[nodes[k]];
        }
        let length = (tx * tx + ty * ty).sqrt();
        for k in 0..3 {
            load[2 * nodes[k]] += w * n[k] * length * traction[0];
            load[2 * nodes[k] + 1] += w * n[k] * length * traction[1];
        }
    }
}

/// The element and natural coordinates of a point, by Newton's method on
/// the map of each element whose box holds it. `None` outside the mesh.
pub fn locate(mesh: &Mesh, x: f64, y: f64) -> Option<(usize, f64, f64)> {
    locate_among(mesh, x, y, |_| true)
}

/// [`locate`] among the elements `keep` holds to: a point on the side two
/// elements share is read from the one kept.
pub fn locate_among(mesh: &Mesh, x: f64, y: f64, keep: impl Fn(&[usize; 8]) -> bool) -> Option<(usize, f64, f64)> {
    const SLACK: f64 = 1.0e-9;
    let mut best: Option<(usize, f64, f64, f64)> = None;
    for (index, element) in mesh.elements.iter().enumerate() {
        if !keep(element) {
            continue;
        }
        let (mut x0, mut x1, mut y0, mut y1) = (f64::MAX, f64::MIN, f64::MAX, f64::MIN);
        for &n in element {
            x0 = x0.min(mesh.x[n]);
            x1 = x1.max(mesh.x[n]);
            y0 = y0.min(mesh.y[n]);
            y1 = y1.max(mesh.y[n]);
        }
        let pad = 0.05 * ((x1 - x0) + (y1 - y0));
        if x < x0 - pad || x > x1 + pad || y < y0 - pad || y > y1 + pad {
            continue;
        }
        let (mut xi, mut eta) = (0.0_f64, 0.0_f64);
        for _ in 0..30 {
            let (n, dxi, deta) = shape(xi, eta);
            let (mut px, mut py, mut a, mut b, mut c, mut d) = (0.0, 0.0, 0.0, 0.0, 0.0, 0.0);
            for k in 0..8 {
                let (ex, ey) = (mesh.x[element[k]], mesh.y[element[k]]);
                px += n[k] * ex;
                py += n[k] * ey;
                a += dxi[k] * ex;
                b += deta[k] * ex;
                c += dxi[k] * ey;
                d += deta[k] * ey;
            }
            let det = a * d - b * c;
            if det == 0.0 || !det.is_finite() {
                break;
            }
            let (rx, ry) = (x - px, y - py);
            let dxi_step = (d * rx - b * ry) / det;
            let deta_step = (-c * rx + a * ry) / det;
            xi += dxi_step;
            eta += deta_step;
            if !(xi.is_finite() && eta.is_finite()) || xi.abs() > 3.0 || eta.abs() > 3.0 {
                break;
            }
            if dxi_step.abs() + deta_step.abs() < 1.0e-14 {
                break;
            }
        }
        let outside = (xi.abs() - 1.0).max(eta.abs() - 1.0).max(0.0);
        if xi.is_finite() && eta.is_finite() && outside <= SLACK && best.is_none_or(|b| outside < b.3) {
            best = Some((index, xi.clamp(-1.0, 1.0), eta.clamp(-1.0, 1.0), outside));
            if outside == 0.0 {
                break;
            }
        }
    }
    best.map(|(e, xi, eta, _)| (e, xi, eta))
}

/// The displacement at natural coordinates of an element.
pub fn displacement_in(mesh: &Mesh, u: &[f64], element: usize, xi: f64, eta: f64) -> [f64; 2] {
    let (n, _, _) = shape(xi, eta);
    let nodes = &mesh.elements[element];
    let mut out = [0.0; 2];
    for k in 0..8 {
        out[0] += n[k] * u[2 * nodes[k]];
        out[1] += n[k] * u[2 * nodes[k] + 1];
    }
    out
}

/// A value of the integration points carried to natural coordinates of an
/// element by the biquadratic through its nine points.
pub fn point_value_in(values: &[f64], element: usize, xi: f64, eta: f64) -> f64 {
    let w = gauss_interpolation(xi, eta);
    (0..POINTS_PER_ELEMENT).map(|p| w[p] * values[element * POINTS_PER_ELEMENT + p]).sum()
}

/// Every node's value of a field of the integration points: each element's
/// biquadratic carried to its nodes, averaged over the elements that share
/// the node.
pub fn nodal_average(mesh: &Mesh, values: &[f64]) -> Vec<f64> {
    let mut sum = vec![0.0; mesh.x.len()];
    let mut count = vec![0_u32; mesh.x.len()];
    for (index, element) in mesh.elements.iter().enumerate() {
        for k in 0..8 {
            sum[element[k]] += point_value_in(values, index, NODE_XI[k], NODE_ETA[k]);
            count[element[k]] += 1;
        }
    }
    sum.iter().zip(&count).map(|(s, c)| if *c > 0 { s / *c as f64 } else { 0.0 }).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn steel() -> Material {
        Material { youngs_modulus: 200_000.0, poisson_ratio: 0.3, yield_strength: Some(250.0) }
    }

    #[test]
    fn the_shape_functions_sum_to_one_and_are_one_at_their_node() {
        for (xi, eta) in [(0.3, -0.7), (-1.0, 0.2), (0.0, 0.0)] {
            let (n, dxi, deta) = shape(xi, eta);
            assert!((n.iter().sum::<f64>() - 1.0).abs() < 1e-15);
            assert!(dxi.iter().sum::<f64>().abs() < 1e-14);
            assert!(deta.iter().sum::<f64>().abs() < 1e-14);
        }
        for k in 0..8 {
            let (n, _, _) = shape(NODE_XI[k], NODE_ETA[k]);
            for (j, v) in n.iter().enumerate() {
                assert!((v - if j == k { 1.0 } else { 0.0 }).abs() < 1e-15);
            }
        }
        let w = gauss_interpolation(GAUSS[2], GAUSS[0]);
        for (p, v) in w.iter().enumerate() {
            assert!((v - if p == 2 { 1.0 } else { 0.0 }).abs() < 1e-14, "{p} {v}");
        }
    }

    /// A trial stress outside the yield surface returns onto it, the plastic
    /// strain closes Hooke's law exactly, and the consistent tangent is the
    /// derivative of the return, by central differences.
    #[test]
    fn the_return_lands_on_the_surface_and_its_tangent_is_its_derivative() {
        let m = steel();
        let strain = [2.0e-3, -0.4e-3, 1.1e-3];
        let (state, tangent) = return_map(&m, &strain, &[0.0; 3], 0.0, false).unwrap();
        assert!((von_mises(&state.stress) / 250.0 - 1.0).abs() < 1e-14);
        let elastic = m.compliance(&state.stress);
        for i in 0..3 {
            assert!((elastic[i] + state.plastic_strain[i] - strain[i]).abs() < 1e-17);
        }
        // The plastic strain is along the deviator: no change of volume in the plane-stress sense.
        let d = deviator(&state.stress);
        let ratio = state.plastic_strain[0] / d[0];
        for i in 0..3 {
            assert!((state.plastic_strain[i] - ratio * d[i]).abs() < 1e-12 * state.plastic_strain[i].abs().max(1e-12), "{i}");
        }
        assert!((ratio - state.multiplier).abs() < 1e-9 * ratio);
        let h = 1.0e-9;
        for j in 0..3 {
            let mut up = strain;
            let mut down = strain;
            up[j] += h;
            down[j] -= h;
            let su = return_map(&m, &up, &[0.0; 3], 0.0, false).unwrap().0.stress;
            let sd = return_map(&m, &down, &[0.0; 3], 0.0, false).unwrap().0.stress;
            for i in 0..3 {
                let numeric = (su[i] - sd[i]) / (2.0 * h);
                assert!((numeric - tangent[i][j]).abs() < 1e-4 * 200_000.0, "{i}{j}: {numeric} against {}", tangent[i][j]);
            }
        }
        // Symmetric.
        for i in 0..3 {
            for j in 0..3 {
                assert!((tangent[i][j] - tangent[j][i]).abs() < 1e-9 * 200_000.0);
            }
        }
    }

    #[test]
    fn a_trial_inside_the_surface_is_elastic() {
        let m = steel();
        let (state, tangent) = return_map(&m, &[0.5e-3, 0.0, 0.0], &[0.0; 3], 0.0, false).unwrap();
        assert_eq!(state.multiplier, 0.0);
        assert_eq!(tangent, m.hooke());
        assert!((state.stress[0] - 0.5e-3 * 200_000.0 / 0.91).abs() < 1e-9);
    }

    #[test]
    fn the_profile_solver_meets_a_dense_solve() {
        // A banded, positive definite matrix of 6 rows with a varying profile.
        let first = vec![0, 0, 1, 0, 2, 3];
        let mut p = Profile::new(first.clone());
        let mut dense = [[0.0_f64; 6]; 6];
        for i in 0..6 {
            for j in first[i]..=i {
                let v = if i == j { 10.0 + i as f64 } else { 1.0 / (1.0 + (i + j) as f64) };
                p.add(i, j, v);
                dense[i][j] = v;
                dense[j][i] = v;
            }
        }
        let x_true = [1.0, -2.0, 0.5, 3.0, -1.0, 2.0];
        let mut b = [0.0; 6];
        for i in 0..6 {
            for j in 0..6 {
                b[i] += dense[i][j] * x_true[j];
            }
        }
        p.factor().unwrap();
        p.solve(&mut b);
        for i in 0..6 {
            assert!((b[i] - x_true[i]).abs() < 1e-13, "{i}");
        }
        let mut bad = Profile::new(vec![0, 0]);
        bad.add(0, 0, 1.0);
        bad.add(1, 0, 2.0);
        bad.add(1, 1, 1.0);
        assert_eq!(bad.factor(), Err(1));
    }

    /// One element, a square of side 2, pulled by a uniform traction: the
    /// patch test. The field is uniform and exact.
    #[test]
    fn a_uniform_pull_on_one_element_is_exact() {
        let mesh = Mesh {
            x: vec![0.0, 2.0, 2.0, 0.0, 1.0, 2.0, 1.0, 0.0],
            y: vec![0.0, 0.0, 2.0, 2.0, 0.0, 1.0, 2.0, 1.0],
            elements: vec![[0, 1, 2, 3, 4, 5, 6, 7]],
        };
        let mut load = vec![0.0; 16];
        edge_traction(&mesh, [1, 5, 2], [100.0, 0.0], &mut load);
        // x held on the left edge, y on the bottom edge.
        let held = vec![(0, 0.0), (1, 0.0), (6, 0.0), (14, 0.0), (9, 0.0), (3, 0.0)];
        let problem = Problem { mesh: &mesh, held, load };
        let m = Material { yield_strength: None, ..steel() };
        let numerics = PathNumerics { largest_step: 0.1, imbalance_tolerance: 1e-10, imbalance_cap: 1e6, max_newton_iterations: 20, max_step_halvings: 4, energy_tolerance: 1e-10, fault: LedgerFault::None };
        let settled = settle(&problem, &m, &numerics, &|_, _, _| None).unwrap();
        assert!((settled.displacement[2] - 100.0 / 200_000.0 * 2.0).abs() < 1e-15);
        assert!((settled.displacement[5] + 0.3 * 100.0 / 200_000.0 * 2.0).abs() < 1e-15);
        for p in &settled.points {
            assert!((p.stress[0] - 100.0).abs() < 1e-10 && p.stress[1].abs() < 1e-10 && p.stress[2].abs() < 1e-10);
        }
        // Clapeyron: the strain energy is the work of the forces rising from
        // zero, half the final forces times the displacement.
        assert!((settled.strain_energy / settled.boundary_work - 1.0).abs() < 1e-13);
        let final_work: f64 = problem.load.iter().zip(&settled.displacement).map(|(f, u)| f * u).sum();
        assert!((settled.strain_energy * 2.0 / final_work - 1.0).abs() < 1e-13);
        assert!(settled.energy_residual < 1e-13);
        // The rectangle of the same step books twice the strain energy: refused by name.
        let rectangle = PathNumerics { fault: LedgerFault::RectangleWork, ..numerics };
        assert!(matches!(settle(&problem, &m, &rectangle, &|_, _, _| None), Err(PathFailure::EnergyLedger { .. })));
    }
}
