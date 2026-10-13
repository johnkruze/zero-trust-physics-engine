//! Spin-unpolarized local-density ground state of one spherical atom.
//!
//! The radial Kohn–Sham equation is integrated on a fixed logarithmic grid.
//! Exchange is the Dirac–Slater term in Hartree units. Correlation is the
//! paramagnetic Vosko–Wilk–Nusair fit, which is the ζ = 0 case of the
//! functional on the NIST reference page. Self-consistency is linear density
//! mixing. The density that enters the first mix is a sum of hydrogenic
//! shells solved in `-Z_n/r`, with `Z_n = (Z − N_inner)^{1/2}`. The bare
//! charge puts those shells on top of the nucleus, and one undamped Hartree
//! update then cancels `-Z/r` before a valence orbital is bound. A run that
//! misses the electron count, the virial identity, or the energy tolerance
//! is refused.
//!
//! NIST prints the exchange formula with the Rydberg prefactor, twice the
//! Hartree term used here. The correlation parameters on that same page are
//! already the Hartree values (`A = 0.0310907` for the paramagnetic gas).
//! The Hartree Schrödinger equation and the Hartree tables both take the
//! Hartree exchange:
//! `ε_x = -(3/4) (3/π)^{1/3} n^{1/3} = -(3/2) (9/(32 π²))^{1/3} r_s^{-1}`.

use std::fmt;

const PI: f64 = std::f64::consts::PI;

/// Radial samples. Odd, so the Simpson intervals are even.
pub const GRID_POINTS: usize = 4001;
/// Innermost radius, Bohr. The same value is used for every atom.
pub const R_MIN_BOHR: f64 = 1.0e-7;
/// Outermost radius, Bohr.
pub const R_MAX_BOHR: f64 = 50.0;
/// Weight of the new density in one linear mix. The previous density keeps `1 - α`.
pub const MIXING_ALPHA: f64 = 0.5;
/// The public solver stops after this many density updates.
pub const MAX_SCF_ITERATIONS: u32 = 80;
/// `|E_n − E_{n−1}|` must fall to this before a result is returned. Hartree.
pub const ENERGY_TOLERANCE_HA: f64 = 1.0e-9;
/// `|T_n − T_{n−1}|` must fall to this as well. The total can sit still while
/// the kinetic and nuclear parts are still exchanging.
pub const KINETIC_TOLERANCE_HA: f64 = 1.0e-9;
/// Electrons of `|ρ_out − ρ_in|` that still count as unmixed.
pub const DENSITY_RESIDUAL_TOLERANCE: f64 = 1.0e-8;
/// `|∫ρ dV − Z|` above this refuses the step.
pub const ELECTRON_COUNT_TOLERANCE: f64 = 1.0e-8;
/// Shell charge `Z_n = (Z − N_inner)` is raised to this power before the
/// hydrogenic orbital for that shell is built. See the module note.
const SHELL_CHARGE_POWER: f64 = 0.5;
/// Absolute residual of the virial identity for this functional, Hartree.
pub const VIRIAL_TOLERANCE_HA: f64 = 1.0e-5;
pub const ATOMIC_NUMBER_MIN: u32 = 1;
pub const ATOMIC_NUMBER_MAX: u32 = 18;
/// One correction to a bisected eigenvalue may move it by at most this
/// fraction of its own magnitude. A larger correction refuses the orbital.
pub const EIGENVALUE_STEP_LIMIT: f64 = 0.25;

/// The published calculation this solver is checked against.
pub const REFERENCE_NAME: &str =
    "NIST Atomic Reference Data for Electronic Structure Calculations, local-density approximation";
pub const REFERENCE_URL: &str = "https://www.nist.gov/pml/atomic-reference-data-electronic-structure-calculations/atomic-reference-data-electronic-7";
/// Date the reference tables were read.
pub const REFERENCE_RETRIEVED: &str = "2026-10-05";
/// Every energy and eigenvalue in the reference test agrees to this. Hartree.
pub const REFERENCE_TOLERANCE_HA: f64 = 1.0e-5;

/// Paramagnetic row of the NIST VWN table. Ferromagnetic and spin-stiffness
/// rows are not used: ζ is fixed at 0.
const VWN_A: f64 = 0.0310907;
const VWN_X0: f64 = -0.10498;
const VWN_B: f64 = 3.72744;
const VWN_C: f64 = 12.9352;

/// Subshells filled in order for Z = 1..=18. Capacity is `2 (2ℓ+1)`.
const SUBSHELLS: [(u32, u32, f64); 5] = [
    (1, 0, 2.0),
    (2, 0, 2.0),
    (2, 1, 6.0),
    (3, 0, 2.0),
    (3, 1, 6.0),
];

/// Why a ground state was not returned.
#[derive(Clone, Debug, PartialEq)]
pub enum AtomReject {
    /// `atomic_number` is outside [`ATOMIC_NUMBER_MIN`]..=[`ATOMIC_NUMBER_MAX`].
    AtomicNumberOutOfRange { atomic_number: u32 },
    /// The fixed subshell list could not place every electron.
    ShellsDoNotHoldElectrons { atomic_number: u32, unplaced: f64 },
    /// The density integral left [`ELECTRON_COUNT_TOLERANCE`].
    ElectronCountBroken {
        electron_count: f64,
        atomic_number: u32,
    },
    /// A density sample was negative.
    NegativeDensity,
    /// An intermediate value was NaN or infinite.
    NonFiniteValue,
    /// No bound radial solution with the required node count.
    OrbitalNotBound { n: u32, l: u32 },
    /// A correction to a bisected eigenvalue was larger than [`EIGENVALUE_STEP_LIMIT`] of that eigenvalue.
    EigenvalueStepOutOfRange {
        n: u32,
        l: u32,
        eigenvalue_ha: f64,
        step_ha: f64,
    },
    /// The iteration cap was reached with the tolerances still open.
    NotConverged { iterations: u32 },
    /// The virial residual at an otherwise converged density was too large.
    VirialBroken { residual_ha: f64 },
}

impl fmt::Display for AtomReject {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::AtomicNumberOutOfRange { atomic_number } => write!(
                f,
                "atomic number {atomic_number} is outside {ATOMIC_NUMBER_MIN}..={ATOMIC_NUMBER_MAX}"
            ),
            Self::ShellsDoNotHoldElectrons {
                atomic_number,
                unplaced,
            } => write!(
                f,
                "atomic number {atomic_number} left {unplaced} electrons unplaced"
            ),
            Self::ElectronCountBroken {
                electron_count,
                atomic_number,
            } => write!(
                f,
                "electron count {electron_count} is not atomic number {atomic_number}"
            ),
            Self::NegativeDensity => write!(f, "electron density was negative"),
            Self::NonFiniteValue => write!(f, "a density or an energy was not finite"),
            Self::OrbitalNotBound { n, l } => {
                write!(f, "no bound orbital for n={n} l={l}")
            }
            Self::EigenvalueStepOutOfRange {
                n,
                l,
                eigenvalue_ha,
                step_ha,
            } => write!(
                f,
                "eigenvalue correction {step_ha} Ha for n={n} l={l} is more than {EIGENVALUE_STEP_LIMIT} of {eigenvalue_ha} Ha"
            ),
            Self::NotConverged { iterations } => {
                write!(f, "not converged after {iterations} iterations")
            }
            Self::VirialBroken { residual_ha } => {
                write!(f, "virial residual {residual_ha} Ha")
            }
        }
    }
}

impl std::error::Error for AtomReject {}

/// One occupied radial orbital.
#[derive(Clone, Debug, PartialEq)]
pub struct Orbital {
    pub n: u32,
    pub l: u32,
    pub occupation: f64,
    pub eigenvalue_ha: f64,
}

/// Converged ground state. Energies are Hartree.
#[derive(Clone, Debug, PartialEq)]
pub struct AtomGroundState {
    pub atomic_number: u32,
    pub iterations: u32,
    pub total_energy_ha: f64,
    pub kinetic_energy_ha: f64,
    pub electron_nucleus_energy_ha: f64,
    pub hartree_energy_ha: f64,
    pub exchange_correlation_energy_ha: f64,
    pub electron_count: f64,
    pub virial_residual_ha: f64,
    pub orbitals: Vec<Orbital>,
    /// Radius of each grid point, Bohr. The same grid for every atom.
    pub radial_grid_bohr: Vec<f64>,
    /// Electrons per cubic Bohr at each grid point. The energies above were
    /// computed from this density.
    pub electron_density: Vec<f64>,
}

/// Ground state of the neutral atom. The grid, the mix, and the iteration cap
/// are the constants in this file.
pub fn solve(atomic_number: u32) -> Result<AtomGroundState, AtomReject> {
    run(atomic_number, MAX_SCF_ITERATIONS, 1.0)
}

fn run(
    atomic_number: u32,
    iteration_cap: u32,
    density_scale: f64,
) -> Result<AtomGroundState, AtomReject> {
    if !(ATOMIC_NUMBER_MIN..=ATOMIC_NUMBER_MAX).contains(&atomic_number) {
        return Err(AtomReject::AtomicNumberOutOfRange { atomic_number });
    }
    if !density_scale.is_finite() {
        return Err(AtomReject::NonFiniteValue);
    }
    let shells = configuration(atomic_number)?;
    let z = atomic_number as f64;
    let grid = Grid::new();
    let mut scratch = Scratch::new(grid.r.len());
    hydrogenic_density(&grid, &mut scratch, z, atomic_number, &shells)?;
    for i in 0..grid.r.len() {
        scratch.v[i] = -z / grid.r[i];
    }

    let mut rho_in = scratch.rho.clone();
    let mut guesses = vec![None; shells.len()];
    let mut previous_total: Option<f64> = None;
    let mut previous_kinetic: Option<f64> = None;
    let mut last_iterations = 0u32;

    for iteration in 0..iteration_cap {
        last_iterations = iteration + 1;
        let solved = solve_shells(&grid, &mut scratch, z, &shells, &guesses)?;
        for (slot, orbital) in solved.iter().enumerate() {
            guesses[slot] = Some(orbital.orbital.eigenvalue_ha);
        }
        density_from_orbitals(&grid, &solved, &mut scratch.rho_out);
        let count_out = electron_count(&grid, &scratch.rho_out);
        ensure_count(count_out, atomic_number)?;
        if !scratch.rho_out.iter().all(|p| p.is_finite()) {
            return Err(AtomReject::NonFiniteValue);
        }

        let parts = energy_parts(&grid, z, &scratch.rho_out, &scratch.v, &solved)?;
        let residual = density_residual(&grid, &rho_in, &scratch.rho_out);
        let energy_settled = previous_total
            .map(|e| (parts.total - e).abs() <= ENERGY_TOLERANCE_HA)
            .unwrap_or(false);
        let kinetic_settled = previous_kinetic
            .map(|t| (parts.kinetic - t).abs() <= KINETIC_TOLERANCE_HA)
            .unwrap_or(false);
        if energy_settled && kinetic_settled && residual <= DENSITY_RESIDUAL_TOLERANCE {
            if parts.virial.abs() > VIRIAL_TOLERANCE_HA {
                return Err(AtomReject::VirialBroken {
                    residual_ha: parts.virial,
                });
            }
            return Ok(AtomGroundState {
                atomic_number,
                iterations: last_iterations,
                total_energy_ha: parts.total,
                kinetic_energy_ha: parts.kinetic,
                electron_nucleus_energy_ha: parts.electron_nucleus,
                hartree_energy_ha: parts.hartree,
                exchange_correlation_energy_ha: parts.exchange_correlation,
                electron_count: count_out,
                virial_residual_ha: parts.virial,
                orbitals: solved.into_iter().map(|o| o.orbital).collect(),
                radial_grid_bohr: grid.r,
                electron_density: scratch.rho_out,
            });
        }

        mix_density(&rho_in, &scratch.rho_out, MIXING_ALPHA, &mut scratch.rho);
        if density_scale != 1.0 {
            for sample in &mut scratch.rho {
                *sample *= density_scale;
            }
        }
        if scratch.rho.iter().any(|p| *p < 0.0) {
            return Err(AtomReject::NegativeDensity);
        }
        let count_mixed = electron_count(&grid, &scratch.rho);
        ensure_count(count_mixed, atomic_number)?;
        potential_from_density(&grid, z, &scratch.rho, &mut scratch.v, &mut scratch.work)?;
        previous_total = Some(parts.total);
        previous_kinetic = Some(parts.kinetic);
        rho_in.clone_from(&scratch.rho);
    }

    Err(AtomReject::NotConverged {
        iterations: last_iterations,
    })
}

fn configuration(atomic_number: u32) -> Result<Vec<(u32, u32, f64)>, AtomReject> {
    let mut remain = atomic_number as f64;
    let mut shells = Vec::with_capacity(SUBSHELLS.len());
    for &(n, l, capacity) in &SUBSHELLS {
        if remain <= 0.0 {
            break;
        }
        let occupation = remain.min(capacity);
        shells.push((n, l, occupation));
        remain -= occupation;
    }
    if remain > 1.0e-12 {
        return Err(AtomReject::ShellsDoNotHoldElectrons {
            atomic_number,
            unplaced: remain,
        });
    }
    Ok(shells)
}

/// Hydrogenic shells whose nuclear charge is `(Z − N_inner)` to the power
/// [`SHELL_CHARGE_POWER`]. Each orbital is normalized on this grid, and the
/// sum is stored only when it integrates to the atomic number. Later mixes
/// are not rescaled.
fn hydrogenic_density(
    grid: &Grid,
    scratch: &mut Scratch,
    z: f64,
    atomic_number: u32,
    shells: &[(u32, u32, f64)],
) -> Result<(), AtomReject> {
    let n_points = grid.r.len();
    let mut rho = vec![0.0; n_points];
    for &(n, l, occupation) in shells {
        let mut inner = 0.0;
        for &(shell_n, _, shell_occupation) in shells {
            if shell_n < n {
                inner += shell_occupation;
            }
        }
        let excess = z - inner;
        if !excess.is_finite() || excess <= 0.0 {
            return Err(AtomReject::NonFiniteValue);
        }
        let z_shell = excess.powf(SHELL_CHARGE_POWER);
        if !z_shell.is_finite() || z_shell <= 0.0 {
            return Err(AtomReject::NonFiniteValue);
        }
        for i in 0..n_points {
            scratch.v[i] = -z_shell / grid.r[i];
        }
        let v_shell = scratch.v.clone();
        let orbital = bound_orbital(grid, scratch, z_shell, n, l, occupation, None, &v_shell)?;
        for i in 0..n_points {
            let r = grid.r[i];
            rho[i] += occupation * orbital.u[i] * orbital.u[i] / (4.0 * PI * r * r);
        }
    }
    ensure_count(electron_count(grid, &rho), atomic_number)?;
    scratch.rho.copy_from_slice(&rho);
    Ok(())
}

fn ensure_count(electron_count: f64, atomic_number: u32) -> Result<(), AtomReject> {
    if !electron_count.is_finite() {
        return Err(AtomReject::NonFiniteValue);
    }
    if (electron_count - atomic_number as f64).abs() > ELECTRON_COUNT_TOLERANCE {
        return Err(AtomReject::ElectronCountBroken {
            electron_count,
            atomic_number,
        });
    }
    Ok(())
}

struct Grid {
    r: Vec<f64>,
    h: f64,
    h2: f64,
}

impl Grid {
    fn new() -> Self {
        let h = (R_MAX_BOHR / R_MIN_BOHR).ln() / (GRID_POINTS - 1) as f64;
        let r = (0..GRID_POINTS)
            .map(|i| R_MIN_BOHR * (i as f64 * h).exp())
            .collect();
        Self { r, h, h2: h * h }
    }
}

struct Scratch {
    g: Vec<f64>,
    y_out: Vec<f64>,
    y_in: Vec<f64>,
    y: Vec<f64>,
    u: Vec<f64>,
    rho: Vec<f64>,
    rho_out: Vec<f64>,
    v: Vec<f64>,
    work: Vec<f64>,
    work_b: Vec<f64>,
}

impl Scratch {
    fn new(n: usize) -> Self {
        Self {
            g: vec![0.0; n],
            y_out: vec![0.0; n],
            y_in: vec![0.0; n],
            y: vec![0.0; n],
            u: vec![0.0; n],
            rho: vec![0.0; n],
            rho_out: vec![0.0; n],
            v: vec![0.0; n],
            work: vec![0.0; n],
            work_b: vec![0.0; n],
        }
    }
}

struct SolvedOrbital {
    orbital: Orbital,
    u: Vec<f64>,
}

struct EnergyParts {
    total: f64,
    kinetic: f64,
    electron_nucleus: f64,
    hartree: f64,
    exchange_correlation: f64,
    virial: f64,
}

/// `Y'' = [(ℓ+1/2)² + 2 r² (V − ε)] Y`, with `r = e^x` and `u = √r Y`.
fn fill_g(l: u32, v: &[f64], energy: f64, r: &[f64], g: &mut [f64]) {
    let lp = l as f64 + 0.5;
    let lp2 = lp * lp;
    for i in 0..r.len() {
        let ri = r[i];
        g[i] = lp2 + 2.0 * ri * ri * (v[i] - energy);
    }
}

fn series_y(r: f64, l: u32, z: f64) -> f64 {
    r.powf(l as f64 + 0.5) * (1.0 - z * r / (l as f64 + 1.0))
}

enum Side {
    Below,
    Above,
    Unusable,
}

fn outward(grid: &Grid, l: u32, z: f64, y: &mut [f64], g: &[f64]) -> Result<(usize, u32), Side> {
    let n = grid.r.len();
    if g[n - 1] <= 0.0 {
        return Err(Side::Above);
    }
    let mut turn = None;
    for i in 0..n - 1 {
        if g[i] <= 0.0 && g[i + 1] > 0.0 {
            turn = Some(i);
        }
    }
    let Some(turn) = turn else {
        return Err(Side::Below);
    };
    if turn < 8 || turn > n - 12 {
        return Err(if turn < 8 { Side::Below } else { Side::Above });
    }
    y[0] = series_y(grid.r[0], l, z);
    y[1] = series_y(grid.r[1], l, z);
    if !y[0].is_finite() || !y[1].is_finite() || y[0] == 0.0 {
        return Err(Side::Unusable);
    }
    let h2 = grid.h2;
    for i in 1..turn {
        let c_next = 1.0 - h2 * g[i + 1] / 12.0;
        let c_mid = 2.0 * (1.0 + 5.0 * h2 * g[i] / 12.0);
        let c_prev = 1.0 - h2 * g[i - 1] / 12.0;
        if c_next.abs() < 1.0e-14 {
            return Err(Side::Unusable);
        }
        y[i + 1] = (c_mid * y[i] - c_prev * y[i - 1]) / c_next;
        if !y[i + 1].is_finite() {
            return Err(Side::Above);
        }
    }
    let mut nodes = 0u32;
    for i in 1..=turn {
        if y[i - 1] * y[i] < 0.0 {
            nodes += 1;
        }
    }
    Ok((turn, nodes))
}

fn deriv_backward(y: &[f64], m: usize, h: f64) -> f64 {
    (25.0 * y[m] - 48.0 * y[m - 1] + 36.0 * y[m - 2] - 16.0 * y[m - 3] + 3.0 * y[m - 4])
        / (12.0 * h)
}

fn deriv_forward(y: &[f64], m: usize, h: f64) -> f64 {
    (-25.0 * y[m] + 48.0 * y[m + 1] - 36.0 * y[m + 2] + 16.0 * y[m + 3] - 3.0 * y[m + 4])
        / (12.0 * h)
}

struct Match {
    mismatch: f64,
    /// `Y (Y'_out − Y'_in) / (2 ∫ u² dr)`, the Cooley step.
    delta_energy: f64,
}

/// Outward and inward Numerov waves matched at one radius.
fn matched_wave(grid: &Grid, l: u32, z: f64, scratch: &mut Scratch) -> Result<Match, Side> {
    let (turn, _) = outward(grid, l, z, &mut scratch.y_out, &scratch.g)?;
    let mut m = turn;
    for shift in 0..12 {
        let j = turn - shift;
        if j >= 6 && scratch.y_out[j].abs() > scratch.y_out[m].abs() {
            m = j;
        }
    }
    if scratch.y_out[m].abs() < 1.0e-18 {
        return Err(Side::Unusable);
    }
    let n = grid.r.len();
    let mut ib = m + 1;
    let mut acc = 0.0;
    while ib < n - 2 && acc < 25.0 {
        acc += scratch.g[ib].max(0.0).sqrt() * grid.h;
        ib += 1;
    }
    if ib < m + 6 {
        ib = (m + 6).min(n - 2);
    }
    if ib < m + 4 || ib >= n {
        return Err(Side::Unusable);
    }
    let s_b = scratch.g[ib].max(0.0).sqrt();
    let s_b1 = scratch.g[ib - 1].max(0.0).sqrt();
    scratch.y_in[ib] = 1.0;
    scratch.y_in[ib - 1] = (0.5 * (s_b + s_b1) * grid.h).exp();
    let h2 = grid.h2;
    for i in (m + 1..ib).rev() {
        if scratch.y_in[i].abs() > 1.0e200 {
            for k in i..=ib {
                scratch.y_in[k] *= 1.0e-100;
            }
        }
        let c_next = 1.0 - h2 * scratch.g[i + 1] / 12.0;
        let c_mid = 2.0 * (1.0 + 5.0 * h2 * scratch.g[i] / 12.0);
        let c_prev = 1.0 - h2 * scratch.g[i - 1] / 12.0;
        if c_prev.abs() < 1.0e-14 {
            return Err(Side::Unusable);
        }
        scratch.y_in[i - 1] = (c_mid * scratch.y_in[i] - c_next * scratch.y_in[i + 1]) / c_prev;
        if !scratch.y_in[i - 1].is_finite() {
            return Err(Side::Unusable);
        }
    }
    if scratch.y_in[m] == 0.0 || !scratch.y_in[m].is_finite() {
        return Err(Side::Unusable);
    }
    let scale = scratch.y_out[m] / scratch.y_in[m];
    for i in m..=ib {
        scratch.y_in[i] *= scale;
    }
    let yp_out = deriv_backward(&scratch.y_out, m, grid.h);
    let yp_in = deriv_forward(&scratch.y_in, m, grid.h);
    if !yp_out.is_finite() || !yp_in.is_finite() {
        return Err(Side::Unusable);
    }
    for i in 0..m {
        scratch.y[i] = scratch.y_out[i];
    }
    for i in m..=ib {
        scratch.y[i] = scratch.y_in[i];
    }
    for i in ib + 1..n {
        scratch.y[i] = 0.0;
    }
    for i in 0..n {
        let root = scratch.y[i] * grid.r[i].sqrt();
        scratch.u[i] = root * root;
    }
    let weight = integrate_dr(&scratch.u, &grid.r, grid.h);
    if !(weight > 0.0) || !weight.is_finite() {
        return Err(Side::Unusable);
    }
    let ym = scratch.y_out[m];
    Ok(Match {
        mismatch: (yp_out - yp_in) / ym,
        delta_energy: ym * (yp_out - yp_in) / (2.0 * weight),
    })
}

fn classify(
    grid: &Grid,
    l: u32,
    z: f64,
    energy: f64,
    target_nodes: u32,
    v: &[f64],
    scratch: &mut Scratch,
) -> Side {
    fill_g(l, v, energy, &grid.r, &mut scratch.g);
    let outward_result = outward(grid, l, z, &mut scratch.y_out, &scratch.g);
    let turn_nodes = match outward_result {
        Err(side) => return side,
        Ok(pair) => pair,
    };
    let nodes = turn_nodes.1;
    let _turn = turn_nodes.0;
    if nodes < target_nodes {
        return Side::Below;
    }
    if nodes > target_nodes {
        return Side::Above;
    }
    match matched_wave(grid, l, z, scratch) {
        Err(side) => side,
        Ok(matched) => {
            if !matched.mismatch.is_finite() {
                Side::Unusable
            } else if matched.mismatch > 0.0 {
                Side::Below
            } else {
                Side::Above
            }
        }
    }
}

fn bisect(
    grid: &Grid,
    l: u32,
    z: f64,
    target_nodes: u32,
    v: &[f64],
    mut lo: f64,
    mut hi: f64,
    scratch: &mut Scratch,
) -> Result<f64, ()> {
    if !(lo < hi && lo.is_finite() && hi.is_finite()) {
        return Err(());
    }
    for _ in 0..64 {
        let mid = 0.5 * (lo + hi);
        if (hi - lo).abs() <= 1.0e-13 {
            return Ok(mid);
        }
        match classify(grid, l, z, mid, target_nodes, v, scratch) {
            Side::Below => lo = mid,
            Side::Above | Side::Unusable => hi = mid,
        }
    }
    Ok(0.5 * (lo + hi))
}

fn full_bracket(
    grid: &Grid,
    l: u32,
    z: f64,
    target_nodes: u32,
    v: &[f64],
    scratch: &mut Scratch,
) -> Result<(f64, f64), ()> {
    let mut lo = -0.8 * (z + 1.0) * (z + 1.0);
    let mut lo_ok = false;
    for _ in 0..30 {
        match classify(grid, l, z, lo, target_nodes, v, scratch) {
            Side::Below => {
                lo_ok = true;
                break;
            }
            Side::Above | Side::Unusable => lo *= 1.4,
        }
    }
    if !lo_ok {
        return Err(());
    }
    let mut hi = -1.0e-8;
    let mut hi_ok = false;
    for _ in 0..40 {
        match classify(grid, l, z, hi, target_nodes, v, scratch) {
            Side::Above => {
                hi_ok = true;
                break;
            }
            Side::Below | Side::Unusable => {
                hi *= 0.3;
                if hi > -1.0e-14 {
                    hi = -1.0e-14;
                    break;
                }
            }
        }
    }
    if !hi_ok {
        if matches!(
            classify(grid, l, z, hi, target_nodes, v, scratch),
            Side::Above
        ) {
            hi_ok = true;
        }
    }
    if !hi_ok || !(lo < hi) {
        return Err(());
    }
    Ok((lo, hi))
}

fn narrow_bracket(
    grid: &Grid,
    l: u32,
    z: f64,
    target_nodes: u32,
    v: &[f64],
    guess: f64,
    scratch: &mut Scratch,
) -> Option<(f64, f64)> {
    if !(guess < 0.0 && guess.is_finite()) {
        return None;
    }
    let mut span = (0.05 * guess.abs()).max(0.02);
    for _ in 0..14 {
        let lo = guess - span;
        let hi = (guess + span).min(-1.0e-10);
        if lo < hi {
            let lo_below = matches!(
                classify(grid, l, z, lo, target_nodes, v, scratch),
                Side::Below
            );
            let hi_above = matches!(
                classify(grid, l, z, hi, target_nodes, v, scratch),
                Side::Above
            );
            if lo_below && hi_above {
                return Some((lo, hi));
            }
        }
        span *= 2.0;
    }
    None
}

fn normalize_u(grid: &Grid, scratch: &mut Scratch) -> Result<(), ()> {
    for i in 0..grid.r.len() {
        scratch.u[i] = scratch.y[i] * grid.r[i].sqrt();
    }
    let mut square = scratch.work_b.clone();
    for i in 0..grid.r.len() {
        square[i] = scratch.u[i] * scratch.u[i];
    }
    let norm = integrate_dr(&square, &grid.r, grid.h);
    if !(norm > 0.0) || !norm.is_finite() {
        return Err(());
    }
    let scale = 1.0 / norm.sqrt();
    for sample in &mut scratch.u {
        *sample *= scale;
    }
    Ok(())
}

fn bound_orbital(
    grid: &Grid,
    scratch: &mut Scratch,
    z: f64,
    n: u32,
    l: u32,
    occupation: f64,
    guess: Option<f64>,
    v: &[f64],
) -> Result<SolvedOrbital, AtomReject> {
    let target_nodes = n - l - 1;
    let bracket = guess
        .and_then(|g| narrow_bracket(grid, l, z, target_nodes, v, g, scratch))
        .or_else(|| full_bracket(grid, l, z, target_nodes, v, scratch).ok());
    let Some((lo, hi)) = bracket else {
        return Err(AtomReject::OrbitalNotBound { n, l });
    };
    let mut energy = bisect(grid, l, z, target_nodes, v, lo, hi, scratch)
        .map_err(|_| AtomReject::OrbitalNotBound { n, l })?;
    for _ in 0..5 {
        fill_g(l, v, energy, &grid.r, &mut scratch.g);
        let Ok(matched) = matched_wave(grid, l, z, scratch) else {
            break;
        };
        if !matched.delta_energy.is_finite() {
            break;
        }
        if matched.delta_energy.abs() < 1.0e-13 {
            break;
        }
        let next = energy + accepted_correction(n, l, energy, matched.delta_energy)?;
        if next >= -1.0e-14 {
            break;
        }
        let nodes_ok = {
            fill_g(l, v, next, &grid.r, &mut scratch.g);
            matches!(outward(grid, l, z, &mut scratch.y_out, &scratch.g), Ok((_, nodes)) if nodes == target_nodes)
        };
        if !nodes_ok {
            break;
        }
        energy = next;
    }
    fill_g(l, v, energy, &grid.r, &mut scratch.g);
    matched_wave(grid, l, z, scratch).map_err(|_| AtomReject::OrbitalNotBound { n, l })?;
    normalize_u(grid, scratch).map_err(|_| AtomReject::OrbitalNotBound { n, l })?;
    if scratch.u.iter().any(|s| !s.is_finite()) || !energy.is_finite() {
        return Err(AtomReject::NonFiniteValue);
    }
    Ok(SolvedOrbital {
        orbital: Orbital {
            n,
            l,
            occupation,
            eigenvalue_ha: energy,
        },
        u: scratch.u.clone(),
    })
}

/// The correction itself, or a refusal when it is outside
/// [`EIGENVALUE_STEP_LIMIT`]. The correction is never shortened.
fn accepted_correction(
    n: u32,
    l: u32,
    eigenvalue_ha: f64,
    step_ha: f64,
) -> Result<f64, AtomReject> {
    if step_ha.abs() > EIGENVALUE_STEP_LIMIT * eigenvalue_ha.abs() {
        return Err(AtomReject::EigenvalueStepOutOfRange {
            n,
            l,
            eigenvalue_ha,
            step_ha,
        });
    }
    Ok(step_ha)
}

fn solve_shells(
    grid: &Grid,
    scratch: &mut Scratch,
    z: f64,
    shells: &[(u32, u32, f64)],
    guesses: &[Option<f64>],
) -> Result<Vec<SolvedOrbital>, AtomReject> {
    let v = scratch.v.clone();
    let mut solved = Vec::with_capacity(shells.len());
    for (index, &(n, l, occupation)) in shells.iter().enumerate() {
        solved.push(bound_orbital(
            grid,
            scratch,
            z,
            n,
            l,
            occupation,
            guesses[index],
            &v,
        )?);
    }
    Ok(solved)
}

fn density_from_orbitals(grid: &Grid, orbitals: &[SolvedOrbital], rho: &mut [f64]) {
    rho.fill(0.0);
    for orbital in orbitals {
        for i in 0..grid.r.len() {
            let r = grid.r[i];
            rho[i] += orbital.orbital.occupation * orbital.u[i] * orbital.u[i] / (4.0 * PI * r * r);
        }
    }
}

fn mix_density(previous: &[f64], updated: &[f64], alpha: f64, out: &mut [f64]) {
    let keep = 1.0 - alpha;
    for i in 0..out.len() {
        out[i] = keep * previous[i] + alpha * updated[i];
    }
}

/// Dirac–Slater exchange in Hartree units. Returns `(ε_x, v_x)`.
fn slater_exchange(rho: f64) -> (f64, f64) {
    if !(rho > 0.0) {
        return (0.0, 0.0);
    }
    let v_x = -((3.0 / PI) * rho).cbrt();
    (0.75 * v_x, v_x)
}

/// Paramagnetic Vosko–Wilk–Nusair correlation. Returns `(ε_c, v_c)`.
fn vwn_correlation(rho: f64) -> (f64, f64) {
    if !(rho > 0.0) {
        return (0.0, 0.0);
    }
    let rs = (3.0 / (4.0 * PI * rho)).cbrt();
    let x = rs.sqrt();
    if !x.is_finite() || x == 0.0 {
        return (0.0, 0.0);
    }
    let q = (4.0 * VWN_C - VWN_B * VWN_B).sqrt();
    let x0_poly = VWN_X0 * VWN_X0 + VWN_B * VWN_X0 + VWN_C;
    let xx = x * x + VWN_B * x + VWN_C;
    let ln1 = -(1.0 + VWN_B / x + VWN_C / (x * x)).ln();
    let dx0 = x - VWN_X0;
    let ln3 = ((dx0 * dx0) / xx).ln();
    let ar = (q / (2.0 * x + VWN_B)).atan();
    let pref = (VWN_B * VWN_X0) / x0_poly;
    let angle_weight = 2.0 * (VWN_B + 2.0 * VWN_X0) / q;
    let eps = VWN_A * (ln1 + (2.0 * VWN_B / q) * ar - pref * (ln3 + angle_weight * ar));
    let df1 = (VWN_B * x + 2.0 * VWN_C) / (x * xx);
    let num3 = (VWN_B + 2.0 * VWN_X0) * x + 2.0 * VWN_C + VWN_B * VWN_X0;
    let df3 = num3 / (dx0 * xx);
    // d/dx atan(Q/(2x+b)) = -Q / (2 X). The 2 is d(2x+b)/dx.
    let df2 = -q / (2.0 * xx);
    let deps_dx = VWN_A * (df1 + (2.0 * VWN_B / q) * df2 - pref * (df3 + angle_weight * df2));
    let v_c = eps - (x / 6.0) * deps_dx;
    if !eps.is_finite() || !v_c.is_finite() {
        (0.0, 0.0)
    } else {
        (eps, v_c)
    }
}

fn prefix_quadratic(f: &[f64], h: f64, out: &mut [f64]) {
    let n = f.len();
    out[0] = 0.0;
    let mut acc = 0.0;
    for i in 0..n - 1 {
        if i + 2 < n {
            let (a, b, c) = (f[i], f[i + 1], f[i + 2]);
            acc += h * (5.0 * a + 8.0 * b - c) / 12.0;
        } else {
            acc += 0.5 * (f[i] + f[i + 1]) * h;
        }
        out[i + 1] = acc;
    }
}

fn hartree(grid: &Grid, rho: &[f64], vh: &mut [f64], fq: &mut [f64], ft: &mut [f64]) {
    let n = grid.r.len();
    for i in 0..n {
        let r = grid.r[i];
        fq[i] = 4.0 * PI * r * r * r * rho[i];
        ft[i] = 4.0 * PI * r * r * rho[i];
    }
    let mut charge = ft.to_vec();
    prefix_quadratic(fq, grid.h, &mut charge);
    let mut tail_prefix = fq.to_vec();
    prefix_quadratic(ft, grid.h, &mut tail_prefix);
    let total_tail = tail_prefix[n - 1];
    for i in 0..n {
        vh[i] = charge[i] / grid.r[i] + (total_tail - tail_prefix[i]);
    }
}

fn potential_from_density(
    grid: &Grid,
    z: f64,
    rho: &[f64],
    v: &mut [f64],
    work: &mut [f64],
) -> Result<(), AtomReject> {
    let mut fq = work.to_vec();
    let mut ft = vec![0.0; grid.r.len()];
    hartree(grid, rho, v, &mut fq, &mut ft);
    for i in 0..grid.r.len() {
        let rho_i = rho[i];
        if !rho_i.is_finite() {
            return Err(AtomReject::NonFiniteValue);
        }
        if rho_i < 0.0 {
            return Err(AtomReject::NegativeDensity);
        }
        let (_, v_x) = slater_exchange(rho_i);
        let (_, v_c) = vwn_correlation(rho_i);
        v[i] += -z / grid.r[i] + v_x + v_c;
        if !v[i].is_finite() {
            return Err(AtomReject::NonFiniteValue);
        }
    }
    Ok(())
}

/// Composite Simpson rule for `∫ f(r) dr` on the logarithmic grid.
fn integrate_dr(f: &[f64], r: &[f64], h: f64) -> f64 {
    let n = f.len();
    let mut acc = f[0] * r[0] + f[n - 1] * r[n - 1];
    for i in 1..n - 1 {
        let weight = if i % 2 == 0 { 2.0 } else { 4.0 };
        acc += weight * f[i] * r[i];
    }
    acc * h / 3.0
}

fn electron_count(grid: &Grid, rho: &[f64]) -> f64 {
    let mut f = vec![0.0; grid.r.len()];
    for i in 0..grid.r.len() {
        let r = grid.r[i];
        f[i] = 4.0 * PI * r * r * rho[i];
    }
    integrate_dr(&f, &grid.r, grid.h)
}

fn density_residual(grid: &Grid, previous: &[f64], updated: &[f64]) -> f64 {
    let mut f = vec![0.0; grid.r.len()];
    for i in 0..grid.r.len() {
        let r = grid.r[i];
        f[i] = 4.0 * PI * r * r * (updated[i] - previous[i]).abs();
    }
    integrate_dr(&f, &grid.r, grid.h)
}

fn energy_parts(
    grid: &Grid,
    z: f64,
    rho_out: &[f64],
    v_solved: &[f64],
    orbitals: &[SolvedOrbital],
) -> Result<EnergyParts, AtomReject> {
    let n = grid.r.len();
    let mut vh = vec![0.0; n];
    let mut fq = vec![0.0; n];
    let mut ft = vec![0.0; n];
    hartree(grid, rho_out, &mut vh, &mut fq, &mut ft);

    let mut f_en = vec![0.0; n];
    let mut f_h = vec![0.0; n];
    let mut f_x = vec![0.0; n];
    let mut f_c = vec![0.0; n];
    let mut f_vc = vec![0.0; n];
    let mut f_veff = vec![0.0; n];
    for i in 0..n {
        let r = grid.r[i];
        let rho = rho_out[i];
        if !rho.is_finite() {
            return Err(AtomReject::NonFiniteValue);
        }
        if rho < 0.0 {
            return Err(AtomReject::NegativeDensity);
        }
        let shell = 4.0 * PI * r * r * rho;
        let (eps_x, _) = slater_exchange(rho);
        let (eps_c, v_c) = vwn_correlation(rho);
        f_en[i] = shell * (-z / r);
        f_h[i] = 0.5 * shell * vh[i];
        f_x[i] = shell * eps_x;
        f_c[i] = shell * eps_c;
        f_vc[i] = shell * v_c;
        // The eigenvalues were computed in `v_solved`, which belongs to the
        // input density. At convergence that density is `rho_out`.
        f_veff[i] = shell * v_solved[i];
    }
    let electron_nucleus = integrate_dr(&f_en, &grid.r, grid.h);
    let hartree = integrate_dr(&f_h, &grid.r, grid.h);
    let exchange = integrate_dr(&f_x, &grid.r, grid.h);
    let correlation = integrate_dr(&f_c, &grid.r, grid.h);
    let vc_integral = integrate_dr(&f_vc, &grid.r, grid.h);
    let veff_integral = integrate_dr(&f_veff, &grid.r, grid.h);
    let mut eigenvalue_sum = 0.0;
    for orbital in orbitals {
        eigenvalue_sum += orbital.orbital.occupation * orbital.orbital.eigenvalue_ha;
    }
    let kinetic = eigenvalue_sum - veff_integral;
    let exchange_correlation = exchange + correlation;
    let total = kinetic + electron_nucleus + hartree + exchange_correlation;
    // Exchange is homogeneous of degree 1, so it cancels in `T + E`.
    // What remains is the correlation scaling term `4 E_c − 3 ∫ ρ v_c`.
    let virial = kinetic + total - 4.0 * correlation + 3.0 * vc_integral;
    if ![
        total,
        kinetic,
        electron_nucleus,
        hartree,
        exchange_correlation,
        virial,
    ]
    .iter()
    .all(|value| value.is_finite())
    {
        return Err(AtomReject::NonFiniteValue);
    }
    Ok(EnergyParts {
        total,
        kinetic,
        electron_nucleus,
        hartree,
        exchange_correlation,
        virial,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn atomic_number_outside_1_through_18_is_refused() {
        for z in [0, 19, 92] {
            let err = solve(z).expect_err("out of range");
            assert!(
                matches!(err, AtomReject::AtomicNumberOutOfRange { atomic_number } if atomic_number == z),
                "{err}"
            );
        }
    }

    #[test]
    fn hydrogenic_spectrum_on_the_coulomb_potential() {
        let grid = Grid::new();
        let mut scratch = Scratch::new(grid.r.len());
        for i in 0..grid.r.len() {
            scratch.v[i] = -1.0 / grid.r[i];
        }
        let cases = [
            (1, 0, -0.5),
            (2, 0, -0.125),
            (2, 1, -0.125),
            (3, 0, -1.0 / 18.0),
        ];
        for (n, l, expect) in cases {
            let v = scratch.v.clone();
            let orbital = bound_orbital(&grid, &mut scratch, 1.0, n, l, 1.0, None, &v)
                .unwrap_or_else(|err| panic!("n={n} l={l}: {err}"));
            let err = (orbital.orbital.eigenvalue_ha - expect).abs();
            assert!(
                err < 1.0e-8,
                "n={n} l={l} got {} expect {expect} err {err}",
                orbital.orbital.eigenvalue_ha
            );
        }
    }

    #[test]
    fn vwn_potential_matches_a_density_derivative() {
        let rho = 0.04;
        let (eps, v_c) = vwn_correlation(rho);
        let dn = rho * 1.0e-6;
        let (eps_hi, _) = vwn_correlation(rho + dn);
        let (eps_lo, _) = vwn_correlation(rho - dn);
        let numeric = eps + rho * (eps_hi - eps_lo) / (2.0 * dn);
        assert!(
            (v_c - numeric).abs() < 1.0e-8,
            "analytic {v_c} numeric {numeric}"
        );
    }

    #[test]
    fn slater_exchange_is_the_hartree_prefactor() {
        let rho = 1.0;
        let (eps, v_x) = slater_exchange(rho);
        let rs = (3.0 / (4.0 * PI * rho)).cbrt();
        let from_rs = -1.5 * (9.0 / (32.0 * PI * PI)).powf(1.0 / 3.0) / rs;
        assert!((eps - from_rs).abs() < 1.0e-12);
        assert!((v_x - (4.0 / 3.0) * eps).abs() < 1.0e-12);
    }

    #[test]
    fn a_mix_that_changes_the_electron_count_is_refused() {
        let err = run(6, MAX_SCF_ITERATIONS, 2.0).expect_err("scaled density");
        assert!(
            matches!(err, AtomReject::ElectronCountBroken { .. }),
            "{err:?}"
        );
    }

    #[test]
    fn an_eigenvalue_correction_past_the_limit_is_refused_and_not_shortened() {
        let eigenvalue = -0.5;
        let limit = EIGENVALUE_STEP_LIMIT * 0.5;
        for step in [limit, -limit, 1.0e-12, 0.0] {
            let kept = accepted_correction(2, 1, eigenvalue, step).expect("inside the limit");
            assert_eq!(kept.to_bits(), step.to_bits());
        }
        for step in [limit * (1.0 + 1.0e-12), -0.2, 3.0] {
            let err = accepted_correction(2, 1, eigenvalue, step).expect_err("past the limit");
            assert_eq!(
                err,
                AtomReject::EigenvalueStepOutOfRange {
                    n: 2,
                    l: 1,
                    eigenvalue_ha: eigenvalue,
                    step_ha: step,
                }
            );
        }
    }

    #[test]
    fn too_few_iterations_are_refused() {
        let err = run(2, 2, 1.0).expect_err("short cap");
        assert!(
            matches!(err, AtomReject::NotConverged { iterations: 2 }),
            "{err:?}"
        );
    }

    #[test]
    fn the_same_atom_twice_is_bit_identical() {
        let first = solve(8).expect("oxygen");
        let second = solve(8).expect("oxygen again");
        assert_eq!(first.iterations, second.iterations);
        assert_eq!(
            first.total_energy_ha.to_bits(),
            second.total_energy_ha.to_bits()
        );
        assert_eq!(
            first.kinetic_energy_ha.to_bits(),
            second.kinetic_energy_ha.to_bits()
        );
        assert_eq!(
            first.electron_nucleus_energy_ha.to_bits(),
            second.electron_nucleus_energy_ha.to_bits()
        );
        assert_eq!(
            first.hartree_energy_ha.to_bits(),
            second.hartree_energy_ha.to_bits()
        );
        assert_eq!(
            first.exchange_correlation_energy_ha.to_bits(),
            second.exchange_correlation_energy_ha.to_bits()
        );
        assert_eq!(
            first.virial_residual_ha.to_bits(),
            second.virial_residual_ha.to_bits()
        );
        assert_eq!(first.orbitals.len(), second.orbitals.len());
        for (a, b) in first.orbitals.iter().zip(second.orbitals.iter()) {
            assert_eq!(a.eigenvalue_ha.to_bits(), b.eigenvalue_ha.to_bits());
            assert_eq!(a.occupation.to_bits(), b.occupation.to_bits());
            assert_eq!(a.n, b.n);
            assert_eq!(a.l, b.l);
        }
        assert_eq!(first.radial_grid_bohr.len(), GRID_POINTS);
        assert_eq!(first.electron_density.len(), GRID_POINTS);
        let bits = |field: &[f64]| field.iter().map(|v| v.to_bits()).collect::<Vec<u64>>();
        assert_eq!(
            bits(&first.radial_grid_bohr),
            bits(&second.radial_grid_bohr)
        );
        assert_eq!(
            bits(&first.electron_density),
            bits(&second.electron_density)
        );
    }

    #[test]
    fn the_returned_density_holds_the_electron_count_on_the_returned_grid() {
        let state = solve(10).expect("neon");
        let grid = Grid::new();
        assert_eq!(
            state.radial_grid_bohr.first().map(|r| r.to_bits()),
            Some(R_MIN_BOHR.to_bits())
        );
        for (returned, built) in state.radial_grid_bohr.iter().zip(grid.r.iter()) {
            assert_eq!(returned.to_bits(), built.to_bits());
        }
        let count = electron_count(&grid, &state.electron_density);
        assert_eq!(count.to_bits(), state.electron_count.to_bits());
        assert!((count - 10.0).abs() <= ELECTRON_COUNT_TOLERANCE, "{count}");
    }

    /// LDA column of the neutral-atom table. Retrieved 2026-10-05
    /// ([`REFERENCE_RETRIEVED`]) from
    /// <https://www.nist.gov/pml/atomic-reference-data-electronic-structure-calculations/atomic-reference-data-electronic-7>
    /// ([`REFERENCE_URL`]) and the element pages below. The functional definition is
    /// <https://www.nist.gov/pml/atomic-reference-data-electronic-structure-calculations/atomic-reference-data-electronic-6-3>.
    struct NistAtom {
        z: u32,
        symbol: &'static str,
        url: &'static str,
        total: f64,
        kinetic: f64,
        hartree: f64,
        electron_nucleus: f64,
        exchange_correlation: f64,
        /// `(n, l, eigenvalue)`.
        orbitals: &'static [(u32, u32, f64)],
    }

    fn nist_table() -> &'static [NistAtom] {
        const H: &[(u32, u32, f64)] = &[(1, 0, -0.233471)];
        const HE: &[(u32, u32, f64)] = &[(1, 0, -0.570425)];
        const LI: &[(u32, u32, f64)] = &[(1, 0, -1.878564), (2, 0, -0.105540)];
        const BE: &[(u32, u32, f64)] = &[(1, 0, -3.856411), (2, 0, -0.205744)];
        const B: &[(u32, u32, f64)] = &[(1, 0, -6.564347), (2, 0, -0.344701), (2, 1, -0.136603)];
        const C: &[(u32, u32, f64)] = &[(1, 0, -9.947718), (2, 0, -0.500866), (2, 1, -0.199186)];
        const N: &[(u32, u32, f64)] = &[(1, 0, -14.011501), (2, 0, -0.676151), (2, 1, -0.266297)];
        const O: &[(u32, u32, f64)] = &[(1, 0, -18.758245), (2, 0, -0.871362), (2, 1, -0.338381)];
        const F: &[(u32, u32, f64)] = &[(1, 0, -24.189391), (2, 0, -1.086859), (2, 1, -0.415606)];
        const NE: &[(u32, u32, f64)] = &[(1, 0, -30.305855), (2, 0, -1.322809), (2, 1, -0.498034)];
        const NA: &[(u32, u32, f64)] = &[
            (1, 0, -37.719975),
            (2, 0, -2.063401),
            (2, 1, -1.060636),
            (3, 0, -0.103415),
        ];
        const MG: &[(u32, u32, f64)] = &[
            (1, 0, -45.973167),
            (2, 0, -2.903746),
            (2, 1, -1.718970),
            (3, 0, -0.175427),
        ];
        const AL: &[(u32, u32, f64)] = &[
            (1, 0, -55.156044),
            (2, 0, -3.934827),
            (2, 1, -2.564018),
            (3, 0, -0.286883),
            (3, 1, -0.102545),
        ];
        const SI: &[(u32, u32, f64)] = &[
            (1, 0, -65.184426),
            (2, 0, -5.075056),
            (2, 1, -3.514938),
            (3, 0, -0.398139),
            (3, 1, -0.153293),
        ];
        const P: &[(u32, u32, f64)] = &[
            (1, 0, -76.061897),
            (2, 0, -6.329346),
            (2, 1, -4.576617),
            (3, 0, -0.512364),
            (3, 1, -0.206080),
        ];
        const S: &[(u32, u32, f64)] = &[
            (1, 0, -87.789937),
            (2, 0, -7.699940),
            (2, 1, -5.751257),
            (3, 0, -0.630912),
            (3, 1, -0.261676),
        ];
        const CL: &[(u32, u32, f64)] = &[
            (1, 0, -100.369229),
            (2, 0, -9.187993),
            (2, 1, -7.039982),
            (3, 0, -0.754458),
            (3, 1, -0.320380),
        ];
        const AR: &[(u32, u32, f64)] = &[
            (1, 0, -113.800134),
            (2, 0, -10.794172),
            (2, 1, -8.443439),
            (3, 0, -0.883384),
            (3, 1, -0.382330),
        ];
        const TABLE: &[NistAtom] = &[
            NistAtom {
                z: 1,
                symbol: "H",
                url: "https://www.nist.gov/pml/atomic-reference-data-electronic-structure-calculations/atomic-reference-data-electronic-7-0",
                total: -0.445671,
                kinetic: 0.425027,
                hartree: 0.282827,
                electron_nucleus: -0.920999,
                exchange_correlation: -0.232525,
                orbitals: H,
            },
            NistAtom {
                z: 2,
                symbol: "He",
                url: "https://www.nist.gov/pml/atomic-reference-data-electronic-structure-calculations/atomic-reference-data-electronic-7-1",
                total: -2.834836,
                kinetic: 2.767922,
                hartree: 1.996120,
                electron_nucleus: -6.625564,
                exchange_correlation: -0.973314,
                orbitals: HE,
            },
            NistAtom {
                z: 3,
                symbol: "Li",
                url: "https://www.nist.gov/pml/atomic-reference-data-electronic-structure-calculations/atomic-reference-data-electronic-7-2",
                total: -7.335195,
                kinetic: 7.237484,
                hartree: 3.990755,
                electron_nucleus: -16.911041,
                exchange_correlation: -1.652393,
                orbitals: LI,
            },
            NistAtom {
                z: 4,
                symbol: "Be",
                url: "https://www.nist.gov/pml/atomic-reference-data-electronic-structure-calculations/atomic-reference-data-electronic-7-3",
                total: -14.447209,
                kinetic: 14.309424,
                hartree: 7.115257,
                electron_nucleus: -33.357034,
                exchange_correlation: -2.514856,
                orbitals: BE,
            },
            NistAtom {
                z: 5,
                symbol: "B",
                url: "https://www.nist.gov/pml/atomic-reference-data-electronic-structure-calculations/atomic-reference-data-electronic-7-4",
                total: -24.344198,
                kinetic: 24.161047,
                hartree: 11.503002,
                electron_nucleus: -56.484587,
                exchange_correlation: -3.523660,
                orbitals: B,
            },
            NistAtom {
                z: 6,
                symbol: "C",
                url: "https://www.nist.gov/pml/atomic-reference-data-electronic-structure-calculations/atomic-reference-data-electronic-7-5",
                total: -37.425749,
                kinetic: 37.190391,
                hartree: 17.627997,
                electron_nucleus: -87.515412,
                exchange_correlation: -4.728724,
                orbitals: C,
            },
            NistAtom {
                z: 7,
                symbol: "N",
                url: "https://www.nist.gov/pml/atomic-reference-data-electronic-structure-calculations/atomic-reference-data-electronic-7-91",
                total: -54.025016,
                kinetic: 53.731727,
                hartree: 25.799845,
                electron_nucleus: -127.416229,
                exchange_correlation: -6.140359,
                orbitals: N,
            },
            NistAtom {
                z: 8,
                symbol: "O",
                url: "https://www.nist.gov/pml/atomic-reference-data-electronic-structure-calculations/atomic-reference-data-electronic-7-6",
                total: -74.473077,
                kinetic: 74.116881,
                hartree: 36.331102,
                electron_nucleus: -177.152578,
                exchange_correlation: -7.768482,
                orbitals: O,
            },
            NistAtom {
                z: 9,
                symbol: "F",
                url: "https://www.nist.gov/pml/atomic-reference-data-electronic-structure-calculations/atomic-reference-data-electronic-7-7",
                total: -99.099648,
                kinetic: 98.676131,
                hartree: 49.535385,
                electron_nucleus: -237.688846,
                exchange_correlation: -9.622319,
                orbitals: F,
            },
            NistAtom {
                z: 10,
                symbol: "Ne",
                url: "https://www.nist.gov/pml/atomic-reference-data-electronic-structure-calculations/atomic-reference-data-electronic-7-8",
                total: -128.233481,
                kinetic: 127.738667,
                hartree: 65.726488,
                electron_nucleus: -309.988206,
                exchange_correlation: -11.710430,
                orbitals: NE,
            },
            NistAtom {
                z: 11,
                symbol: "Na",
                url: "https://www.nist.gov/pml/atomic-reference-data-electronic-structure-calculations/atomic-reference-data-electronic-7-9",
                total: -161.440060,
                kinetic: 160.896571,
                hartree: 79.732497,
                electron_nucleus: -388.546372,
                exchange_correlation: -13.522757,
                orbitals: NA,
            },
            NistAtom {
                z: 12,
                symbol: "Mg",
                url: "https://www.nist.gov/pml/atomic-reference-data-electronic-structure-calculations/atomic-reference-data-electronic-7-10",
                total: -199.139406,
                kinetic: 198.541505,
                hartree: 95.673290,
                electron_nucleus: -477.899149,
                exchange_correlation: -15.455051,
                orbitals: MG,
            },
            NistAtom {
                z: 13,
                symbol: "Al",
                url: "https://www.nist.gov/pml/atomic-reference-data-electronic-structure-calculations/atomic-reference-data-electronic-7-11",
                total: -241.315573,
                kinetic: 240.663489,
                hartree: 112.670733,
                electron_nucleus: -577.205757,
                exchange_correlation: -17.444038,
                orbitals: AL,
            },
            NistAtom {
                z: 14,
                symbol: "Si",
                url: "https://www.nist.gov/pml/atomic-reference-data-electronic-structure-calculations/atomic-reference-data-electronic-7-12",
                total: -288.198397,
                kinetic: 287.487740,
                hartree: 131.767814,
                electron_nucleus: -687.900675,
                exchange_correlation: -19.553276,
                orbitals: SI,
            },
            NistAtom {
                z: 15,
                symbol: "P",
                url: "https://www.nist.gov/pml/atomic-reference-data-electronic-structure-calculations/atomic-reference-data-electronic-7-13",
                total: -339.946219,
                kinetic: 339.173459,
                hartree: 153.065007,
                electron_nucleus: -810.400100,
                exchange_correlation: -21.784585,
                orbitals: P,
            },
            NistAtom {
                z: 16,
                symbol: "S",
                url: "https://www.nist.gov/pml/atomic-reference-data-electronic-structure-calculations/atomic-reference-data-electronic-7-14",
                total: -396.716081,
                kinetic: 395.878143,
                hartree: 176.687950,
                electron_nucleus: -945.141155,
                exchange_correlation: -24.141019,
                orbitals: S,
            },
            NistAtom {
                z: 17,
                symbol: "Cl",
                url: "https://www.nist.gov/pml/atomic-reference-data-electronic-structure-calculations/atomic-reference-data-electronic-7-15",
                total: -458.664179,
                kinetic: 457.758293,
                hartree: 202.772337,
                electron_nucleus: -1092.568993,
                exchange_correlation: -26.625816,
                orbitals: CL,
            },
            NistAtom {
                z: 18,
                symbol: "Ar",
                url: "https://www.nist.gov/pml/atomic-reference-data-electronic-structure-calculations/atomic-reference-data-electronic-7-16",
                total: -525.946195,
                kinetic: 524.969812,
                hartree: 231.458124,
                electron_nucleus: -1253.131982,
                exchange_correlation: -29.242149,
                orbitals: AR,
            },
        ];
        TABLE
    }

    fn check_close(misses: &mut Vec<String>, label: &str, got: f64, expect: f64) {
        let diff = got - expect;
        if diff.abs() > REFERENCE_TOLERANCE_HA {
            misses.push(format!(
                "{label}: computed {got:.10} NIST {expect:.6} diff {diff:.3e}"
            ));
        }
    }

    #[test]
    fn nist_lda_from_hydrogen_to_argon() {
        println!("NIST LDA retrieved {REFERENCE_RETRIEVED}");
        println!(
            "{:<4} {:>14} {:>14} {:>12} {:>6} {:>8}",
            "atom", "computed", "NIST", "diff", "iters", "ms"
        );
        let mut misses = Vec::new();
        for row in nist_table() {
            let started = std::time::Instant::now();
            let got = match solve(row.z) {
                Ok(state) => state,
                Err(err) => {
                    misses.push(format!("{} refused: {err}", row.symbol));
                    continue;
                }
            };
            let ms = started.elapsed().as_secs_f64() * 1.0e3;
            let diff = got.total_energy_ha - row.total;
            println!(
                "{:<4} {:>14.8} {:>14.6} {:>12.3e} {:>6} {:>8.1}",
                row.symbol, got.total_energy_ha, row.total, diff, got.iterations, ms
            );
            check_close(
                &mut misses,
                &format!("{} total", row.symbol),
                got.total_energy_ha,
                row.total,
            );
            check_close(
                &mut misses,
                &format!("{} kinetic", row.symbol),
                got.kinetic_energy_ha,
                row.kinetic,
            );
            check_close(
                &mut misses,
                &format!("{} hartree", row.symbol),
                got.hartree_energy_ha,
                row.hartree,
            );
            check_close(
                &mut misses,
                &format!("{} electron-nucleus", row.symbol),
                got.electron_nucleus_energy_ha,
                row.electron_nucleus,
            );
            check_close(
                &mut misses,
                &format!("{} exchange-correlation", row.symbol),
                got.exchange_correlation_energy_ha,
                row.exchange_correlation,
            );
            let parts = got.kinetic_energy_ha
                + got.electron_nucleus_energy_ha
                + got.hartree_energy_ha
                + got.exchange_correlation_energy_ha;
            if (parts - got.total_energy_ha).abs() > 1.0e-8 {
                misses.push(format!(
                    "{} parts {} do not sum to total {}",
                    row.symbol, parts, got.total_energy_ha
                ));
            }
            if got.orbitals.len() != row.orbitals.len() {
                misses.push(format!(
                    "{} orbital count {} vs {}",
                    row.symbol,
                    got.orbitals.len(),
                    row.orbitals.len()
                ));
                continue;
            }
            for (orbital, expect) in got.orbitals.iter().zip(row.orbitals.iter()) {
                if orbital.n != expect.0 || orbital.l != expect.1 {
                    misses.push(format!(
                        "{} orbital n={} l={} expected n={} l={}",
                        row.symbol, orbital.n, orbital.l, expect.0, expect.1
                    ));
                }
                check_close(
                    &mut misses,
                    &format!(
                        "{} {}{}",
                        row.symbol,
                        orbital.n,
                        ["s", "p", "d"][orbital.l as usize]
                    ),
                    orbital.eigenvalue_ha,
                    expect.2,
                );
            }
            println!(
                "     kin {:>14.8} {:>14.6} {:>12.3e}",
                got.kinetic_energy_ha,
                row.kinetic,
                got.kinetic_energy_ha - row.kinetic
            );
            println!(
                "     coul {:>14.8} {:>14.6} {:>12.3e}",
                got.hartree_energy_ha,
                row.hartree,
                got.hartree_energy_ha - row.hartree
            );
            println!(
                "     enuc {:>14.8} {:>14.6} {:>12.3e}",
                got.electron_nucleus_energy_ha,
                row.electron_nucleus,
                got.electron_nucleus_energy_ha - row.electron_nucleus
            );
            println!(
                "     exc {:>14.8} {:>14.6} {:>12.3e}",
                got.exchange_correlation_energy_ha,
                row.exchange_correlation,
                got.exchange_correlation_energy_ha - row.exchange_correlation
            );
            for orbital in &got.orbitals {
                let label = format!("{}{}", orbital.n, ["s", "p", "d"][orbital.l as usize]);
                let expect = row
                    .orbitals
                    .iter()
                    .find(|item| item.0 == orbital.n && item.1 == orbital.l)
                    .map(|item| item.2)
                    .unwrap_or(0.0);
                println!(
                    "     {label:<4} {:>14.8} {:>14.6} {:>12.3e}",
                    orbital.eigenvalue_ha,
                    expect,
                    orbital.eigenvalue_ha - expect
                );
            }
            println!(
                "     virial {:>12.3e}  electrons {:>12.8}  url {}",
                got.virial_residual_ha, got.electron_count, row.url
            );
        }
        assert!(
            misses.is_empty(),
            "NIST 1e-5 Ha missed:\n{}",
            misses.join("\n")
        );
    }
}
