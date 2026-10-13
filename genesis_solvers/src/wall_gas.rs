//! The gas at a carbon wall in chemical equilibrium, computed from NASA's
//! open thermodynamic records (`wall_gas_species.rs`), and the surface table
//! made from it for a stream of oxygen, nitrogen and argon.
//!
//! This is the calculation that made the locked surface table for air
//! (`surface_air_tacotgas.bin`), written here so that the same table can be
//! made for any other stream when a question asks for one. It follows the
//! surface work's own program step for step
//! (`evidence/research/heatshield_surface_model/analysis/equil.py`,
//! `tables.py`, `rates.py`), and the tests hold it against that program's
//! results.
//!
//! # The calculation
//!
//! An ideal-gas mixture at temperature `T` and pressure `p`. At equilibrium
//! each species' mole fraction is
//!
//! ```text
//! x_j = exp( −g_j(T)/RT − ln(p/p0) + Σ_k a_jk π_k )
//! ```
//!
//! with `g_j` the species' standard Gibbs energy (`p0` = 1 bar), `a_jk` its
//! atoms of element `k`, and `π_k` one unknown per element. Two cases:
//!
//! - **At the wall.** The gas touches solid carbon, which fixes carbon's
//!   number: `π_C = g_graphite(T)/RT`. The other elements are present in
//!   stated amounts `b_k`. The unknowns are their `π_k` and the total moles
//!   of gas `N`; the equations are `N Σ_j a_jk x_j = b_k` and `Σ_j x_j = 1`.
//! - **Gas alone.** No solid; carbon is an element like the others.
//!
//! Solved by Newton's method on `π_k` and `ln N` with a step no longer than
//! 4 and halved until the largest residual falls, to a residual of 10⁻¹¹.
//!
//! # The surface relations
//!
//! For each kilogram of stream gas reaching the wall, `B'g` kg of
//! decomposition gas and `B'c` kg of char carbon join the wall gas (all
//! species diffusing alike, heat- and mass-transfer coefficients equal). The
//! elements other than carbon are then known,
//! `b_k = (Y_k,stream + B'g Y_k,gas)/M_k`; the wall equilibrium gives the gas
//! they make in contact with carbon, whose mass is `1 + B'g + B'c`. Where
//! `B'c` would be negative (carbon would deposit) it is given as zero and
//! the wall-gas enthalpy stays that of the carbon-saturated gas, as in the
//! published TACOT table. Where carbon vapour alone reaches 0.999 of the
//! wall pressure the wall boils and there is no finite `B'c`.

use crate::wall_gas_species::{Species, GASES, GRAPHITE};

/// Gas constant, J/mol/K.
const GAS_CONSTANT_J_PER_MOL_K: f64 = 8.31446261815324;
/// Standard pressure of the thermodynamic records, Pa.
const STANDARD_PRESSURE_PA: f64 = 1.0e5;
/// Elements, in the order of a record's `atoms`.
const ELEMENTS: usize = 5;
const CARBON: usize = 0;
const OXYGEN: usize = 2;
const NITROGEN: usize = 3;
const ARGON: usize = 4;
/// Atomic masses as used with `thermo.inp`, g/mol: C, H, O, N, Ar.
const ATOMIC_MASS_G_PER_MOL: [f64; ELEMENTS] = [12.0107, 1.00794, 15.9994, 14.0067, 39.948];
/// The decomposition gas of TACOT, mole fractions of carbon, hydrogen and
/// oxygen (TACOT_3.0.xls, pyrolysis model).
const DECOMPOSITION_GAS_MOLE_FRACTIONS: [f64; ELEMENTS] = [0.206, 0.679, 0.115, 0.0, 0.0];
/// Newton's method stops at this largest residual.
const RESIDUAL_TOLERANCE: f64 = 1.0e-11;
const NEWTON_PASSES_MAX: usize = 400;
/// The wall boils where carbon vapour alone reaches this share of the pressure.
const BOILING_SHARE_OF_PRESSURE: f64 = 0.999;
/// The five carbon vapours, by name.
const CARBON_VAPOURS: [&str; 5] = ["C", "C2", "C3", "C4", "C5"];
/// The gases of a stream brought to rest, for the share of its oxygen that is atomic.
const STREAM_AT_REST: [&str; 6] = ["N2", "O2", "NO", "N", "O", "Ar"];
/// Pressures, Pa, of the atomic-oxygen table.
const ATOM_PRESSURE_PA: [f64; 6] = [100.0, 1.0e3, 3.0e3, 1.0e4, 3.0e4, 1.0e5];
/// The atomic-oxygen table runs over 161 enthalpies from zero to this, J/kg.
const ATOM_ENTHALPY_TOP_J_PER_KG: f64 = 40.0e6;
const ATOM_ENTHALPIES: usize = 161;

fn h_over_rt(species: &Species, t: f64) -> f64 {
    let i = interval(species, t);
    let a = &i.a;
    -a[0] / (t * t)
        + a[1] * t.ln() / t
        + a[2]
        + a[3] * t / 2.0
        + a[4] * (t * t) / 3.0
        + a[5] * t.powi(3) / 4.0
        + a[6] * t.powi(4) / 5.0
        + i.b1 / t
}

fn s_over_r(species: &Species, t: f64) -> f64 {
    let i = interval(species, t);
    let a = &i.a;
    -a[0] / (2.0 * (t * t)) - a[1] / t
        + a[2] * t.ln()
        + a[3] * t
        + a[4] * (t * t) / 2.0
        + a[5] * t.powi(3) / 3.0
        + a[6] * t.powi(4) / 4.0
        + i.b2
}

fn interval(species: &Species, t: f64) -> &'static crate::wall_gas_species::Interval {
    let all = species.intervals;
    if t <= species.lowest_k {
        return &all[0];
    }
    for one in all {
        if t <= one.highest_k {
            return one;
        }
    }
    &all[all.len() - 1]
}

fn molar_mass_kg(species: &Species) -> f64 {
    species.molar_mass_g_per_mol * 1.0e-3
}

/// A set of gas species.
struct Mixture {
    species: Vec<&'static Species>,
    /// Which are carbon alone (C, C2, ...).
    carbon_only: Vec<bool>,
}

/// The species' numbers at one temperature.
struct AtTemperature {
    g_over_rt: Vec<f64>,
    enthalpy_j_per_mol: Vec<f64>,
    graphite_g_over_rt: f64,
}

/// One equilibrium.
struct Equilibrium {
    /// Mole fractions of every species of the mixture.
    mole_fraction: Vec<f64>,
    /// Total moles of gas.
    total_mol: f64,
    /// The unknowns: `π` of each element that is present, then `ln N`.
    unknowns: Vec<f64>,
    converged: bool,
}

impl Mixture {
    fn of(names: impl Iterator<Item = &'static str>) -> Self {
        let species: Vec<&'static Species> = names
            .map(|name| {
                GASES
                    .iter()
                    .find(|s| s.name == name)
                    .expect("a species of the locked records")
            })
            .collect();
        let carbon_only = species
            .iter()
            .map(|s| s.atoms[CARBON] > 0.0 && s.atoms.iter().sum::<f64>() == s.atoms[CARBON])
            .collect();
        Self {
            species,
            carbon_only,
        }
    }

    /// Every gas of the records.
    fn wide() -> Self {
        Self::of(GASES.iter().map(|s| s.name))
    }

    /// The same without the five carbon vapours: oxidation alone.
    fn without_carbon_vapour() -> Self {
        Self::of(
            GASES
                .iter()
                .map(|s| s.name)
                .filter(|name| !CARBON_VAPOURS.contains(name)),
        )
    }

    fn at(&self, temperature_k: f64) -> AtTemperature {
        let t = temperature_k;
        AtTemperature {
            g_over_rt: self
                .species
                .iter()
                .map(|s| h_over_rt(s, t) - s_over_r(s, t))
                .collect(),
            enthalpy_j_per_mol: self
                .species
                .iter()
                .map(|s| h_over_rt(s, t) * GAS_CONSTANT_J_PER_MOL_K * t)
                .collect(),
            graphite_g_over_rt: h_over_rt(&GRAPHITE, t) - s_over_r(&GRAPHITE, t),
        }
    }

    /// Sum of the equilibrium pressures of the carbon-only gases over graphite, Pa.
    fn carbon_vapour_pressure_pa(&self, at: &AtTemperature) -> f64 {
        let mut sum = 0.0;
        for (j, s) in self.species.iter().enumerate() {
            if self.carbon_only[j] {
                sum += (-at.g_over_rt[j] + s.atoms[CARBON] * at.graphite_g_over_rt).exp();
            }
        }
        STANDARD_PRESSURE_PA * sum
    }

    /// Mass (kg) and enthalpy (J) of `total_mol` of gas with these mole fractions.
    fn mass_and_enthalpy(&self, e: &Equilibrium, at: &AtTemperature) -> (f64, f64) {
        let mut mass = 0.0;
        let mut enthalpy = 0.0;
        for (j, s) in self.species.iter().enumerate() {
            mass += e.mole_fraction[j] * molar_mass_kg(s);
            enthalpy += e.mole_fraction[j] * at.enthalpy_j_per_mol[j];
        }
        (e.total_mol * mass, e.total_mol * enthalpy)
    }

    /// Equilibrium at `at` and `pressure_pa` of the elements in the amounts
    /// `element_mol` (zero: absent). At a carbon wall any carbon in
    /// `element_mol` is ignored: the wall supplies it.
    fn equilibrium(
        &self,
        at: &AtTemperature,
        pressure_pa: f64,
        element_mol: &[f64; ELEMENTS],
        carbon_wall: bool,
        guess: Option<&[f64]>,
    ) -> Equilibrium {
        let present = |k: usize| element_mol[k] > 0.0;
        let fixed = |k: usize| carbon_wall && k == CARBON;
        let free: Vec<usize> = (0..ELEMENTS).filter(|&k| present(k) && !fixed(k)).collect();
        let nf = free.len();
        let active: Vec<usize> = (0..self.species.len())
            .filter(|&j| {
                (0..ELEMENTS).all(|k| present(k) || fixed(k) || self.species[j].atoms[k] == 0.0)
            })
            .collect();
        let na = active.len();
        // Atoms of each free element in each active species, row by row.
        let mut atoms = vec![0.0; na * nf];
        let mut base = vec![0.0; na];
        let log_pressure = (pressure_pa / STANDARD_PRESSURE_PA).ln();
        for (row, &j) in active.iter().enumerate() {
            for (column, &k) in free.iter().enumerate() {
                atoms[row * nf + column] = self.species[j].atoms[k];
            }
            base[row] = -at.g_over_rt[j] - log_pressure;
            if carbon_wall {
                base[row] += self.species[j].atoms[CARBON] * at.graphite_g_over_rt;
            }
        }
        let amount: Vec<f64> = free.iter().map(|&k| element_mol[k]).collect();
        let mut y: Vec<f64> = match guess {
            Some(guess) if guess.len() == nf + 1 => guess.to_vec(),
            _ => first_guess(&atoms, &base, &amount, na, nf),
        };
        // Residuals, mole numbers and element sums at `y`.
        let residual = |y: &[f64], f: &mut [f64], x: &mut [f64], el: &mut [f64]| {
            el.iter_mut().for_each(|v| *v = 0.0);
            let mut sum = 0.0;
            for row in 0..na {
                let mut log_x = base[row];
                for column in 0..nf {
                    log_x += atoms[row * nf + column] * y[column];
                }
                let value = log_x.min(700.0).exp();
                x[row] = value;
                sum += value;
                for column in 0..nf {
                    el[column] += atoms[row * nf + column] * value;
                }
            }
            for column in 0..nf {
                f[column] = y[nf] + el[column].max(1.0e-300).ln() - amount[column].ln();
            }
            f[nf] = sum.max(1.0e-300).ln();
            sum
        };
        let largest = |f: &[f64]| f.iter().fold(0.0f64, |a, v| a.max(v.abs()));
        let n = nf + 1;
        let (mut f, mut x, mut el) = (vec![0.0; n], vec![0.0; na], vec![0.0; nf]);
        let (mut f2, mut x2, mut el2) = (vec![0.0; n], vec![0.0; na], vec![0.0; nf]);
        let mut sum = residual(&y, &mut f, &mut x, &mut el);
        let mut converged = false;
        let mut jacobian = vec![0.0; n * n];
        let mut step = vec![0.0; n];
        let mut trial = vec![0.0; n];
        for _ in 0..NEWTON_PASSES_MAX {
            let norm = largest(&f);
            if norm < RESIDUAL_TOLERANCE {
                converged = true;
                break;
            }
            if !norm.is_finite() {
                break;
            }
            // d ln(el_k)/d π_l = Σ_j a_jk a_jl x_j / el_k
            jacobian.iter_mut().for_each(|v| *v = 0.0);
            for row in 0..na {
                for k in 0..nf {
                    let a_k = atoms[row * nf + k] * x[row];
                    if a_k != 0.0 {
                        for l in 0..nf {
                            jacobian[k * n + l] += a_k * atoms[row * nf + l];
                        }
                    }
                }
            }
            for k in 0..nf {
                for l in 0..nf {
                    jacobian[k * n + l] /= el[k];
                }
                jacobian[k * n + nf] = 1.0;
                jacobian[nf * n + k] = el[k] / sum;
            }
            jacobian[nf * n + nf] = 0.0;
            for k in 0..n {
                step[k] = -f[k];
            }
            if !solve_small(&mut jacobian, &mut step, n) {
                break;
            }
            let longest = largest(&step);
            let mut length = if longest < 4.0 { 1.0 } else { 4.0 / longest };
            let mut sum2 = sum;
            for _ in 0..40 {
                for k in 0..n {
                    trial[k] = y[k] + length * step[k];
                }
                sum2 = residual(&trial, &mut f2, &mut x2, &mut el2);
                if f2.iter().all(|v| v.is_finite())
                    && largest(&f2) < (1.0 - 1.0e-4 * length) * norm + 1.0e-14
                {
                    break;
                }
                length *= 0.5;
            }
            for k in 0..n {
                y[k] += length * step[k];
            }
            std::mem::swap(&mut f, &mut f2);
            std::mem::swap(&mut x, &mut x2);
            std::mem::swap(&mut el, &mut el2);
            sum = sum2;
        }
        let mut mole_fraction = vec![0.0; self.species.len()];
        for (row, &j) in active.iter().enumerate() {
            mole_fraction[j] = x[row] / sum;
        }
        Equilibrium {
            mole_fraction,
            total_mol: y[nf].exp(),
            unknowns: y,
            converged,
        }
    }

    /// The same, started from the neighbouring state's unknowns and, if that
    /// does not converge, from scratch.
    fn equilibrium_from(
        &self,
        at: &AtTemperature,
        pressure_pa: f64,
        element_mol: &[f64; ELEMENTS],
        carbon_wall: bool,
        guess: &mut Option<Vec<f64>>,
    ) -> Equilibrium {
        let mut e = self.equilibrium(at, pressure_pa, element_mol, carbon_wall, guess.as_deref());
        if !e.converged && guess.is_some() {
            e = self.equilibrium(at, pressure_pa, element_mol, carbon_wall, None);
        }
        *guess = if e.converged {
            Some(e.unknowns.clone())
        } else {
            None
        };
        e
    }
}

/// A start for Newton's method: each element's number from the one species
/// that carries it most stably at a mole fraction of 0.3, lowered until the
/// mole fractions sum to less than one.
fn first_guess(atoms: &[f64], base: &[f64], amount: &[f64], na: usize, nf: usize) -> Vec<f64> {
    let mut y = vec![0.0; nf + 1];
    let third = 0.3f64.ln();
    let row_sum = |row: usize| (0..nf).map(|c| atoms[row * nf + c]).sum::<f64>();
    for k in 0..nf {
        let alone: Vec<usize> = (0..na)
            .filter(|&row| atoms[row * nf + k] > 0.0 && row_sum(row) == atoms[row * nf + k])
            .collect();
        let mut lowest = f64::INFINITY;
        if alone.is_empty() {
            for row in (0..na).filter(|&row| atoms[row * nf + k] > 0.0) {
                lowest = lowest.min((third - base[row]) / row_sum(row));
            }
        } else {
            for &row in &alone {
                lowest = lowest.min((third - base[row]) / atoms[row * nf + k]);
            }
        }
        y[k] = lowest;
    }
    let log_x = |y: &[f64], row: usize| {
        let mut v = base[row];
        for c in 0..nf {
            v += atoms[row * nf + c] * y[c];
        }
        v
    };
    for _ in 0..60 {
        let sum: f64 = (0..na).map(|row| log_x(&y, row).min(700.0).exp()).sum();
        if sum < 1.0 {
            break;
        }
        for value in y.iter_mut().take(nf) {
            *value -= 0.5;
        }
    }
    let mut elements = 0.0;
    for row in 0..na {
        elements += row_sum(row) * log_x(&y, row).exp();
    }
    y[nf] = (amount.iter().sum::<f64>() / elements.max(1.0e-300)).ln();
    y
}

/// Solves the `n × n` system in place by elimination with row exchange.
/// False if the matrix is singular.
fn solve_small(matrix: &mut [f64], rhs: &mut [f64], n: usize) -> bool {
    for column in 0..n {
        let mut pivot = column;
        for row in column + 1..n {
            if matrix[row * n + column].abs() > matrix[pivot * n + column].abs() {
                pivot = row;
            }
        }
        if !(matrix[pivot * n + column].abs() > 0.0) || !matrix[pivot * n + column].is_finite() {
            return false;
        }
        if pivot != column {
            for k in 0..n {
                matrix.swap(column * n + k, pivot * n + k);
            }
            rhs.swap(column, pivot);
        }
        for row in column + 1..n {
            let factor = matrix[row * n + column] / matrix[column * n + column];
            if factor != 0.0 {
                for k in column..n {
                    matrix[row * n + k] -= factor * matrix[column * n + k];
                }
                rhs[row] -= factor * rhs[column];
            }
        }
    }
    for row in (0..n).rev() {
        let mut value = rhs[row];
        for k in row + 1..n {
            value -= matrix[row * n + k] * rhs[k];
        }
        rhs[row] = value / matrix[row * n + row];
    }
    rhs.iter().all(|v| v.is_finite())
}

/// Element mass fractions (C, H, O, N, Ar) of the decomposition gas.
fn decomposition_gas_mass_fractions() -> [f64; ELEMENTS] {
    let mut mass = [0.0; ELEMENTS];
    let mut sum = 0.0;
    for k in 0..ELEMENTS {
        mass[k] = DECOMPOSITION_GAS_MOLE_FRACTIONS[k] * ATOMIC_MASS_G_PER_MOL[k];
        sum += mass[k];
    }
    mass.iter_mut().for_each(|m| *m /= sum);
    mass
}

/// Element mass fractions of a stream of oxygen, argon and nitrogen.
fn stream_mass_fractions(oxygen: f64, argon: f64) -> [f64; ELEMENTS] {
    let mut mass = [0.0; ELEMENTS];
    mass[OXYGEN] = oxygen;
    mass[ARGON] = argon;
    mass[NITROGEN] = 1.0 - oxygen - argon;
    mass
}

/// Moles of each element for one kilogram of stream gas with `blowing` kg of
/// decomposition gas.
fn element_mol(stream: &[f64; ELEMENTS], gas: &[f64; ELEMENTS], blowing: f64) -> [f64; ELEMENTS] {
    let mut mol = [0.0; ELEMENTS];
    for k in 0..ELEMENTS {
        let mass = stream[k] + blowing * gas[k];
        if mass > 0.0 {
            mol[k] = mass / (ATOMIC_MASS_G_PER_MOL[k] * 1.0e-3);
        }
    }
    mol
}

/// The surface tables of one stream at a list of pressures, on a grid of
/// B'g and temperature: what a surface bundle holds that depends on the
/// stream. Pressure outermost, then B'g, then temperature.
pub(crate) struct StreamTables {
    /// `B'c`; infinite where the wall boils.
    pub char_rate: Vec<f64>,
    pub wall_enthalpy: Vec<f64>,
    /// The same two with the five carbon vapours left out.
    pub char_rate_without_vapour: Vec<f64>,
    pub wall_enthalpy_without_vapour: Vec<f64>,
    /// Stream gas and decomposition gas in equilibrium with each other, no
    /// carbon exchanged with the wall.
    pub wall_enthalpy_no_exchange: Vec<f64>,
    /// The decomposition gas alone, by pressure and temperature.
    pub gas_alone_enthalpy: Vec<f64>,
    /// Share of the stream's oxygen that is atomic when the stream is
    /// brought to rest, by pressure and enthalpy.
    pub atom_pressure_pa: Vec<f64>,
    pub atom_enthalpy_j_per_kg: Vec<f64>,
    pub atom_fraction: Vec<f64>,
}

/// A state the equilibrium did not converge at.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct NotConverged {
    pub pressure_pa: f64,
    pub blowing: f64,
    pub temperature_k: f64,
}

/// Straight lines through rising `(grid, values)`, held at the ends.
fn along(grid: &[f64], values: &[f64], at: f64) -> f64 {
    let n = grid.len();
    if at <= grid[0] {
        return values[0];
    }
    if at >= grid[n - 1] {
        return values[n - 1];
    }
    let mut i = 0;
    while grid[i + 1] < at {
        i += 1;
    }
    values[i] + (at - grid[i]) / (grid[i + 1] - grid[i]) * (values[i + 1] - values[i])
}

/// Gives every state of a row that did not converge (left as not-a-number)
/// the value on the straight line between its converged neighbours in
/// temperature, held at the ends. False if no state of the row converged.
///
/// This is what the surface work's program does, and it matters at a few
/// cold states: with about a tenth as much decomposition gas as stream and
/// the wall below about 475 K, the gas-phase equilibrium with no carbon
/// exchanged does not converge by this method (the oxygen is used up almost
/// exactly), and the enthalpy there is the one at the coldest converged
/// temperature. The locked table for air has the same.
fn from_neighbours(temperature_k: &[f64], row: &mut [f64]) -> bool {
    let good: Vec<usize> = (0..row.len()).filter(|&i| row[i].is_finite()).collect();
    if good.is_empty() {
        return false;
    }
    if good.len() < row.len() {
        let grid: Vec<f64> = good.iter().map(|&i| temperature_k[i]).collect();
        let values: Vec<f64> = good.iter().map(|&i| row[i]).collect();
        for i in 0..row.len() {
            if !row[i].is_finite() {
                row[i] = along(&grid, &values, temperature_k[i]);
            }
        }
    }
    true
}

/// Computes the tables for a stream with these mass fractions of oxygen and
/// argon (the rest nitrogen) and TACOT's decomposition gas.
pub(crate) fn stream_tables(
    oxygen_mass_fraction: f64,
    argon_mass_fraction: f64,
    pressure_pa: &[f64],
    blowing: &[f64],
    temperature_k: &[f64],
) -> Result<StreamTables, NotConverged> {
    let stream = stream_mass_fractions(oxygen_mass_fraction, argon_mass_fraction);
    let gas = decomposition_gas_mass_fractions();
    let (np, nb, nt) = (pressure_pa.len(), blowing.len(), temperature_k.len());
    let wide = Mixture::wide();
    let plain = Mixture::without_carbon_vapour();
    let wide_at: Vec<AtTemperature> = temperature_k.iter().map(|&t| wide.at(t)).collect();
    let plain_at: Vec<AtTemperature> = temperature_k.iter().map(|&t| plain.at(t)).collect();
    let nodes = np * nb * nt;
    let mut out = StreamTables {
        char_rate: vec![f64::INFINITY; nodes],
        wall_enthalpy: vec![f64::NAN; nodes],
        char_rate_without_vapour: vec![f64::INFINITY; nodes],
        wall_enthalpy_without_vapour: vec![f64::NAN; nodes],
        wall_enthalpy_no_exchange: vec![f64::NAN; nodes],
        gas_alone_enthalpy: vec![f64::NAN; np * nt],
        atom_pressure_pa: ATOM_PRESSURE_PA.to_vec(),
        atom_enthalpy_j_per_kg: Vec::new(),
        atom_fraction: Vec::new(),
    };
    for (ip, &p) in pressure_pa.iter().enumerate() {
        for (ib, &bg) in blowing.iter().enumerate() {
            let mol = element_mol(&stream, &gas, bg);
            let first = (ip * nb + ib) * nt;
            let failed = |it: usize| NotConverged {
                pressure_pa: p,
                blowing: bg,
                temperature_k: temperature_k[it],
            };
            // In contact with carbon, rising in temperature, with and without the carbon vapours.
            for (mixture, at_all, with_vapour) in
                [(&wide, &wide_at, true), (&plain, &plain_at, false)]
            {
                let (rate, enthalpy) = if with_vapour {
                    (&mut out.char_rate, &mut out.wall_enthalpy)
                } else {
                    (
                        &mut out.char_rate_without_vapour,
                        &mut out.wall_enthalpy_without_vapour,
                    )
                };
                let mut guess = None;
                let mut last = f64::NAN;
                for it in 0..nt {
                    let at = &at_all[it];
                    if mixture.carbon_vapour_pressure_pa(at) >= BOILING_SHARE_OF_PRESSURE * p {
                        for later in it..nt {
                            enthalpy[first + later] = last;
                        }
                        break;
                    }
                    let e = mixture.equilibrium_from(at, p, &mol, true, &mut guess);
                    if !e.converged {
                        return Err(failed(it));
                    }
                    let (mass, heat) = mixture.mass_and_enthalpy(&e, at);
                    rate[first + it] = (mass - 1.0 - bg).max(0.0);
                    enthalpy[first + it] = heat / mass;
                    last = heat / mass;
                }
            }
            // No carbon exchanged with the wall, falling in temperature.
            let mut guess = None;
            for it in (0..nt).rev() {
                let at = &wide_at[it];
                let e = wide.equilibrium_from(at, p, &mol, false, &mut guess);
                if e.converged {
                    let (mass, heat) = wide.mass_and_enthalpy(&e, at);
                    out.wall_enthalpy_no_exchange[first + it] = heat / mass;
                }
            }
            if !from_neighbours(
                temperature_k,
                &mut out.wall_enthalpy_no_exchange[first..first + nt],
            ) {
                return Err(failed(0));
            }
        }
        // The decomposition gas alone, falling in temperature.
        let mol = element_mol(&[0.0; ELEMENTS], &gas, 1.0);
        let mut guess = None;
        for it in (0..nt).rev() {
            let at = &wide_at[it];
            let e = wide.equilibrium_from(at, p, &mol, false, &mut guess);
            if e.converged {
                let (mass, heat) = wide.mass_and_enthalpy(&e, at);
                out.gas_alone_enthalpy[ip * nt + it] = heat / mass;
            }
        }
        if !from_neighbours(
            temperature_k,
            &mut out.gas_alone_enthalpy[ip * nt..(ip + 1) * nt],
        ) {
            return Err(NotConverged {
                pressure_pa: p,
                blowing: f64::INFINITY,
                temperature_k: temperature_k[0],
            });
        }
    }
    let (enthalpies, fraction) = atomic_oxygen_share(&stream)?;
    out.atom_enthalpy_j_per_kg = enthalpies;
    out.atom_fraction = fraction;
    Ok(out)
}

/// Share of the stream's oxygen that is atomic when the stream is brought to
/// rest in equilibrium, by pressure ([`ATOM_PRESSURE_PA`]) and enthalpy: the
/// stream's own equilibrium from 300 K to 12,000 K every 100 K, read against
/// its enthalpy.
fn atomic_oxygen_share(stream: &[f64; ELEMENTS]) -> Result<(Vec<f64>, Vec<f64>), NotConverged> {
    let rest = Mixture::of(STREAM_AT_REST.iter().copied());
    let atom = rest
        .species
        .iter()
        .position(|s| s.name == "O")
        .expect("atomic oxygen");
    let mol = element_mol(stream, &[0.0; ELEMENTS], 0.0);
    let enthalpies: Vec<f64> = (0..ATOM_ENTHALPIES)
        .map(|i| i as f64 * (ATOM_ENTHALPY_TOP_J_PER_KG / (ATOM_ENTHALPIES - 1) as f64))
        .collect();
    let mut fraction = Vec::with_capacity(ATOM_PRESSURE_PA.len() * ATOM_ENTHALPIES);
    for &p in &ATOM_PRESSURE_PA {
        let (mut h, mut share) = (Vec::new(), Vec::new());
        let mut guess: Option<Vec<f64>> = None;
        for step in 0..118 {
            let t = 300.0 + 100.0 * step as f64;
            let at = rest.at(t);
            let e = rest.equilibrium(&at, p, &mol, false, guess.as_deref());
            if !e.converged {
                return Err(NotConverged {
                    pressure_pa: p,
                    blowing: 0.0,
                    temperature_k: t,
                });
            }
            let (mass, heat) = rest.mass_and_enthalpy(&e, &at);
            h.push(heat / mass);
            share.push(e.total_mol * e.mole_fraction[atom] / mol[OXYGEN]);
            guess = Some(e.unknowns);
        }
        for &at in &enthalpies {
            fraction.push(along(&h, &share, at));
        }
    }
    Ok((enthalpies, fraction))
}

/// Enthalpy, J/kg, of a stream of oxygen, argon and nitrogen brought to a
/// wall at `temperature_k` in equilibrium with itself at 1 bar, no carbon:
/// the cold-wall enthalpy an arc-jet heating level is referred to.
pub(crate) fn stream_enthalpy_at_wall(
    oxygen_mass_fraction: f64,
    argon_mass_fraction: f64,
    temperature_k: f64,
) -> Option<f64> {
    let stream = stream_mass_fractions(oxygen_mass_fraction, argon_mass_fraction);
    let wide = Mixture::wide();
    let at = wide.at(temperature_k);
    let mol = element_mol(&stream, &[0.0; ELEMENTS], 0.0);
    let e = wide.equilibrium(&at, STANDARD_PRESSURE_PA, &mol, false, None);
    if !e.converged {
        return None;
    }
    let (mass, heat) = wide.mass_and_enthalpy(&e, &at);
    Some(heat / mass)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Enthalpy of one gas as it is, J/kg.
    fn of(name: &str, temperature_k: f64) -> f64 {
        let s = GASES.iter().find(|s| s.name == name).unwrap();
        h_over_rt(s, temperature_k) * GAS_CONSTANT_J_PER_MOL_K * temperature_k / molar_mass_kg(s)
    }

    #[test]
    fn the_records_give_the_textbook_numbers() {
        // Elements in their reference states have zero enthalpy at 298.15 K.
        for name in ["N2", "O2", "H2", "Ar"] {
            assert!(of(name, 298.15).abs() < 1.0, "{name}");
        }
        assert!(h_over_rt(&GRAPHITE, 298.15).abs() * 8.314 * 298.15 < 1.0);
        // Heats of formation, kJ/mol: CO −110.5, CO2 −393.5, H2O −241.8.
        let molar = |name: &str| {
            let s = GASES.iter().find(|s| s.name == name).unwrap();
            h_over_rt(s, 298.15) * GAS_CONSTANT_J_PER_MOL_K * 298.15 / 1000.0
        };
        assert!((molar("CO") + 110.5).abs() < 0.1);
        assert!((molar("CO2") + 393.5).abs() < 0.1);
        assert!((molar("H2O") + 241.8).abs() < 0.1);
        assert_eq!(GASES.len(), 52);
        // Every record's atoms weigh what the record says, to the file's rounding.
        for s in GASES.iter().chain(std::iter::once(&GRAPHITE)) {
            let weighed: f64 = (0..ELEMENTS)
                .map(|k| s.atoms[k] * ATOMIC_MASS_G_PER_MOL[k])
                .sum();
            assert!(
                (weighed - s.molar_mass_g_per_mol).abs() < 2.0e-3,
                "{} {weighed} {}",
                s.name,
                s.molar_mass_g_per_mol
            );
        }
    }

    #[test]
    fn hot_air_alone_comes_apart_and_keeps_its_atoms() {
        let wide = Mixture::wide();
        let stream = stream_mass_fractions(0.2314, 0.1);
        let mol = element_mol(&stream, &[0.0; ELEMENTS], 0.0);
        for &(t, p) in &[
            (300.0, 1.0e5),
            (2000.0, 1.0e4),
            (6000.0, 1.0e3),
            (9000.0, 100.0),
        ] {
            let at = wide.at(t);
            let e = wide.equilibrium(&at, p, &mol, false, None);
            assert!(e.converged, "{t} {p}");
            let sum: f64 = e.mole_fraction.iter().sum();
            assert!((sum - 1.0).abs() < 1.0e-12);
            // One kilogram went in.
            let (mass, _) = wide.mass_and_enthalpy(&e, &at);
            assert!((mass - 1.0).abs() < 1.0e-9, "{mass}");
            // Atoms of each element are kept.
            for k in 0..ELEMENTS {
                let held: f64 = wide
                    .species
                    .iter()
                    .zip(&e.mole_fraction)
                    .map(|(s, x)| s.atoms[k] * x * e.total_mol)
                    .sum();
                assert!(
                    (held - mol[k]).abs() <= 1.0e-9 * mol[k] + 1.0e-300,
                    "{k} {held}"
                );
            }
            // No carbon or hydrogen species appear from nowhere.
            for (s, x) in wide.species.iter().zip(&e.mole_fraction) {
                if s.atoms[CARBON] > 0.0 || s.atoms[1] > 0.0 {
                    assert_eq!(*x, 0.0, "{}", s.name);
                }
            }
            let x = |name: &str| {
                e.mole_fraction[wide.species.iter().position(|s| s.name == name).unwrap()]
            };
            if t == 300.0 {
                assert!(x("O") < 1.0e-30 && x("NO") < 1.0e-12);
            }
            if t == 9000.0 {
                assert!(x("O2") < 1.0e-6 && x("N2") < 0.01 && x("O") > 0.1 && x("N") > 0.5);
            }
        }
    }

    #[test]
    fn the_cold_wall_enthalpy_is_that_of_oxygen_nitrogen_and_argon_as_they_are() {
        let (o2, n2, ar) = (of("O2", 300.0), of("N2", 300.0), of("Ar", 300.0));
        eprintln!("enthalpy at 300 K, J/kg: oxygen {o2:?}, nitrogen {n2:?}, argon {ar:?}");
        for &(oxygen, argon) in &[(0.2314, 0.0), (0.1675336, 0.276), (0.1, 0.0), (0.4, 0.5)] {
            let mixed = stream_enthalpy_at_wall(oxygen, argon, 300.0).expect("converged");
            let apart = oxygen * o2 + (1.0 - oxygen - argon) * n2 + argon * ar;
            // In equilibrium the cold gas holds a trace of nitrogen oxides:
            // a ten-thousandth of a joule per kilogram.
            assert!((mixed - apart).abs() < 1.0e-3, "{mixed} {apart}");
        }
    }

    #[test]
    fn the_atomic_share_of_a_streams_oxygen_rises_from_none_to_all() {
        let (enthalpies, fraction) =
            atomic_oxygen_share(&stream_mass_fractions(0.2, 0.2)).expect("converged");
        assert_eq!(enthalpies.len(), ATOM_ENTHALPIES);
        assert_eq!(enthalpies[1], 250_000.0);
        for row in fraction.chunks(ATOM_ENTHALPIES) {
            assert!(row[0] < 1.0e-12 && row[ATOM_ENTHALPIES - 1] > 0.99);
            assert!(row.windows(2).all(|pair| pair[1] >= pair[0] - 1.0e-9));
            assert!(row.iter().all(|v| (0.0..=1.0 + 1.0e-9).contains(v)));
        }
    }
}
