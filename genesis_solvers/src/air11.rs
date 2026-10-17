//! Eleven-species air: its thermodynamics, its chemical equilibrium, and the
//! Dunn-Kang reaction set. Not a solver: the physics `plasma_sheath_ram_c2`
//! is built on.
//!
//! **Species.** N2, O2, NO, N, O, N2+, O2+, NO+, N+, O+ and the electron, the
//! eleven-species air of Gupta, Yos, Thompson and Lee (NASA RP-1232, 1990)
//! and of Dunn and Kang (NASA CR-2232, 1973).
//!
//! **Thermodynamics.** Each species' heat capacity, enthalpy and entropy are
//! the NASA Glenn nine-coefficient fits of McBride, Zehe and Gordon (NASA
//! TP-2002-211556), as distributed with NASA's chemical-equilibrium program
//! CEA (`data/thermo.inp`, github.com/nasa/cea at commit `985fd33`):
//!
//! ```text
//! cp/R  = a1 T^-2 + a2 T^-1 + a3 + a4 T + a5 T^2 + a6 T^3 + a7 T^4
//! h/RT  = -a1 T^-2 + a2 ln(T)/T + a3 + a4 T/2 + a5 T^2/3 + a6 T^3/4 + a7 T^4/5 + b1/T
//! s/R   = -a1 T^-2/2 - a2/T + a3 ln T + a4 T + a5 T^2/2 + a6 T^3/3 + a7 T^4/4 + b2
//! ```
//!
//! in three ranges, from 200 K (298.15 K for the ions and the electron) to
//! 1000 K, to 6000 K and to 20 000 K. Enthalpies include the heats of
//! formation at 298.15 K, with N2, O2 and the electron gas as the reference
//! states; entropies are at the standard pressure of 1 bar. The fits stop at
//! 20 000 K and at their floors, and a state outside them is refused by name:
//! a polynomial is not defined past its range. One extension is declared:
//! undissociated air (N2 and O2 alone, the freestream) between 180 K and
//! 200 K holds its 200 K values with cp = 7/2 R, the rigid rotor with
//! vibration in its ground state. O2's first vibrational quantum is 2270 K,
//! N2's 3390 K; at 200 K vibration holds under 1e-5 of the energy. The
//! coldest air of the 1962 standard atmosphere is 180.65 K.
//!
//! The 11-species fits of NASA RP-1232 (Gupta et al., Table IV, to 30 000 K)
//! were transcribed first and set against these. They were not used: the
//! printed A7 of atomic oxygen from 6000 K to 15 000 K (0.50490E+01) puts
//! oxygen's entropy 0.155 R above its value from statistical mechanics
//! (Sackur-Tetrode and oxygen's electronic levels give s/R = 28.401 at
//! 10 000 K and 1 atm; the fit gives 28.556) and jumps by +0.156 R and
//! -0.152 R at the ends of that range; N2+'s heat of formation is 17.4 kJ/mol
//! above the modern value; and the molecules' enthalpies above 8000 K differ
//! from the NASA Glenn values by several percent.
//!
//! **Equilibrium.** At a temperature and a density, the composition that
//! minimises the Gibbs energy with the nuclei of nitrogen and of oxygen held
//! and the charge zero. It is solved by element potentials: every species'
//! concentration is `(p0/RT) exp(-g_i/RT + a_iN L_N + a_iO L_O + q_i L_q)`,
//! the charge potential `L_q` closes neutrality exactly in logarithms, and
//! Newton's method on `L_N` and `L_O` holds the two nuclei totals to 1e-13.
//!
//! **Kinetics.** The 26 reactions of Dunn and Kang's Table I (NASA CR-2232,
//! p. 52), their forward rate coefficients `C T^n exp(-theta/T)` as printed
//! (the central value where an uncertainty is printed), each third body
//! listed as its own reaction. One temperature: Dunn and Kang hold
//! vibration, electronic excitation and the free electrons at the
//! heavy-particle temperature (CR-2232 p. 10). Dunn and Kang printed both
//! directions of every reaction, and the two do not always agree with the
//! thermodynamics: for N + O = NO+ + e their two rates imply an equilibrium
//! constant 3.7 times below the thermodynamic one at 6000 K. Here one
//! direction of each reaction is taken as printed and the other is its
//! partner by detailed balance, `kf / kb = Kc` from these thermodynamics, so
//! the kinetics relax to the equilibrium above and the entropy the reactions
//! produce is never negative. The direction taken is the one Dunn and Kang
//! name as measured: the deionization (backward) rates of reactions 7, 8, 9,
//! 10 and 13, "taken from the experimental results" (CR-2232 p. 9), and the
//! forward rate of every other reaction. A test states, row by row, how far
//! the derived direction is from the one they printed.

#![allow(clippy::needless_range_loop)]

use genesis_kernel::math;
use std::sync::OnceLock;

/// The species, in this order everywhere.
pub const SPECIES: usize = 11;
pub const SPECIES_NAMES: [&str; SPECIES] = ["N2", "O2", "NO", "N", "O", "N2+", "O2+", "NO+", "N+", "O+", "e-"];
pub const N2: usize = 0;
pub const O2: usize = 1;
pub const NO: usize = 2;
pub const N: usize = 3;
pub const O: usize = 4;
pub const N2_ION: usize = 5;
pub const O2_ION: usize = 6;
pub const NO_ION: usize = 7;
pub const N_ION: usize = 8;
pub const O_ION: usize = 9;
pub const ELECTRON: usize = 10;

/// Molar masses, kg/mol, as `thermo.inp` lists them (an ion is its neutral
/// less one electron).
pub const MOLAR_MASS_KG_PER_MOL: [f64; SPECIES] = [
    0.0280134, 0.0319988, 0.0300061, 0.0140067, 0.0159994,
    0.0280128514, 0.0319982514, 0.0300055514, 0.0140061514, 0.0159988514,
    5.48579903e-7,
];

/// Nuclei of nitrogen and of oxygen, and the charge, in one particle.
pub const COMPOSITION: [[f64; 3]; SPECIES] = [
    [2.0, 0.0, 0.0], [0.0, 2.0, 0.0], [1.0, 1.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0],
    [2.0, 0.0, 1.0], [0.0, 2.0, 1.0], [1.0, 1.0, 1.0], [1.0, 0.0, 1.0], [0.0, 1.0, 1.0],
    [0.0, 0.0, -1.0],
];

/// The molar gas constant, J/(mol K): the value the fits were made with
/// (Cohen and Taylor 1987, NASA TP-2002-211556 appendix A), so that each fit
/// gives its listed heat of formation at 298.15 K. CODATA 2018 is 5.7e-6 lower.
pub const GAS_CONSTANT_J_PER_MOL_K: f64 = 8.314510;
/// The Avogadro constant, 1/mol: SI 2019, exact.
pub const AVOGADRO_PER_MOL: f64 = 6.02214076e23;
/// The standard pressure of the fits' entropies, 1 bar.
pub const STANDARD_PRESSURE_PA: f64 = 1.0e5;
/// The top of the fits.
pub const FIT_CEILING_K: f64 = 20_000.0;
/// The floor of each species' fit: 200 K for the neutrals, 298.15 K for the
/// ions and the electron.
pub const FIT_FLOOR_K: [f64; SPECIES] = [200.0, 200.0, 200.0, 200.0, 200.0, 298.15, 298.15, 298.15, 298.15, 298.15, 298.15];
/// The floor of every species at once: a state with ions in it.
pub const ALL_SPECIES_FLOOR_K: f64 = 298.15;
/// The bottom of the rigid-rotor extension of N2 and O2 below their fits.
pub const RIGID_ROTOR_FLOOR_K: f64 = 180.0;
/// The two ends inside the fits where one range hands to the next.
pub const RANGE_ENDS_K: [f64; 2] = [1000.0, 6000.0];

/// NASA Glenn coefficients a1..a7, b1, b2 for each species and range.
pub const NASA_GLENN: [[[f64; 9]; 3]; SPECIES] = [
    // N2: 200.0 to 1000.0 K, 1000.0 to 6000.0 K, 6000.0 to 20000.0 K
    [
        [22103.71497, -381.846182, 6.08273836, -0.00853091441, 1.384646189e-05, -9.62579362e-09, 2.519705809e-12, 710.846086, -10.76003744],
        [587712.406, -2239.249073, 6.06694922, -0.00061396855, 1.491806679e-07, -1.923105485e-11, 1.061954386e-15, 12832.10415, -15.86640027],
        [831013916.0, -642073.354, 202.0264635, -0.03065092046, 2.486903333e-06, -9.70595411e-11, 1.437538881e-15, 4938707.04, -1672.09974],
    ],
    // O2: 200.0 to 1000.0 K, 1000.0 to 6000.0 K, 6000.0 to 20000.0 K
    [
        [-34255.6342, 484.700097, 1.119010961, 0.00429388924, -6.83630052e-07, -2.0233727e-09, 1.039040018e-12, -3391.45487, 18.4969947],
        [-1037939.022, 2344.830282, 1.819732036, 0.001267847582, -2.188067988e-07, 2.053719572e-11, -8.19346705e-16, -16890.10929, 17.38716506],
        [497529430.0, -286610.6874, 66.9035225, -0.00616995902, 3.016396027e-07, -7.4214166e-12, 7.27817577e-17, 2293554.027, -553.062161],
    ],
    // NO: 200.0 to 1000.0 K, 1000.0 to 6000.0 K, 6000.0 to 20000.0 K
    [
        [-11439.16503, 153.6467592, 3.43146873, -0.002668592368, 8.48139912e-06, -7.68511105e-09, 2.386797655e-12, 9098.21441, 6.72872549],
        [223901.8716, -1289.651623, 5.43393603, -0.00036560349, 9.88096645e-08, -1.416076856e-11, 9.38018462e-16, 17503.17656, -8.50166909],
        [-957530354.0, 591243.448, -138.4566826, 0.01694339403, -1.007351096e-06, 2.912584076e-11, -3.29510935e-16, -4677501.24, 1242.081216],
    ],
    // N: 200.0 to 1000.0 K, 1000.0 to 6000.0 K, 6000.0 to 20000.0 K
    [
        [0.0, 0.0, 2.5, 0.0, 0.0, 0.0, 0.0, 56104.6378, 4.193905036],
        [88765.0138, -107.12315, 2.362188287, 0.0002916720081, -1.7295151e-07, 4.01265788e-11, -2.677227571e-15, 56973.5133, 4.865231506],
        [547518105.0, -310757.498, 69.1678274, -0.00684798813, 3.8275724e-07, -1.098367709e-11, 1.277986024e-16, 2550585.618, -584.8769753],
    ],
    // O: 200.0 to 1000.0 K, 1000.0 to 6000.0 K, 6000.0 to 20000.0 K
    [
        [-7953.6113, 160.7177787, 1.966226438, 0.00101367031, -1.110415423e-06, 6.5175075e-10, -1.584779251e-13, 28403.62437, 8.40424182],
        [261902.0262, -729.872203, 3.31717727, -0.000428133436, 1.036104594e-07, -9.43830433e-12, 2.725038297e-16, 33924.2806, -0.667958535],
        [177900426.4, -108232.8257, 28.10778365, -0.002975232262, 1.854997534e-07, -5.79623154e-12, 7.191720164e-17, 889094.263, -218.1728151],
    ],
    // N2+: 298.15 to 1000.0 K, 1000.0 to 6000.0 K, 6000.0 to 20000.0 K
    [
        [-34740.4747, 269.6222703, 3.16491637, -0.002132239781, 6.7304764e-06, -5.63730497e-09, 1.621756e-12, 179000.4424, 6.832974166],
        [-2845599.002, 7058.89303, -2.884886385, 0.003068677059, -4.36165231e-07, 2.102514545e-11, 5.41199647e-16, 134038.8483, 50.90897022],
        [-371282977.0, 313928.7234, -96.0351805, 0.01571193286, -1.175065525e-06, 4.14444123e-11, -5.62189309e-16, -2217361.867, 843.6270947],
    ],
    // O2+: 298.15 to 1000.0 K, 1000.0 to 6000.0 K, 6000.0 to 20000.0 K
    [
        [-86072.0545, 1051.875934, -0.543238047, 0.00657116654, -3.27426375e-06, 5.94064534e-11, 3.23878479e-13, 134554.4668, 29.0270975],
        [73846.5488, -845.955954, 4.98516416, -0.000161101089, 6.42708399e-08, -1.504939874e-11, 1.578465409e-15, 144632.1044, -5.81123065],
        [-1562125524.0, 1161406.778, -330.250472, 0.0471093752, -3.35446138e-06, 1.167968599e-10, -1.589754791e-15, -8857866.27, 2852.035602],
    ],
    // NO+: 298.15 to 1000.0 K, 1000.0 to 6000.0 K, 6000.0 to 20000.0 K
    [
        [1398.106635, -159.0446941, 5.1228954, -0.00639438862, 1.123918342e-05, -7.98858126e-09, 2.107383677e-12, 118749.5132, -4.39843381],
        [606987.69, -2278.395427, 6.08032467, -0.000606684758, 1.432002611e-07, -1.747990522e-11, 8.93501406e-16, 132270.9615, -15.19880037],
        [2676400347.0, -1832948.69, 509.924939, -0.0711381928, 5.31765988e-06, -1.963208212e-10, 2.80526823e-15, 14433089.39, -4324.044462],
    ],
    // N+: 298.15 to 1000.0 K, 1000.0 to 6000.0 K, 6000.0 to 20000.0 K
    [
        [5237.07921, 2.299958315, 2.487488821, 2.737490756e-05, -3.134447576e-08, 1.850111332e-11, -4.447350984e-15, 225628.4738, 5.076830786],
        [290497.0374, -855.790861, 3.47738929, -0.000528826719, 1.352350307e-07, -1.389834122e-11, 5.046166279e-16, 231080.9984, -1.994146545],
        [16460921.48, -11131.65218, 4.97698664, -0.0002005393583, 1.022481356e-08, -2.691430863e-13, 3.539931593e-18, 313628.4696, -17.0664638],
    ],
    // O+: 298.15 to 1000.0 K, 1000.0 to 6000.0 K, 6000.0 to 20000.0 K
    [
        [0.0, 0.0, 2.5, 0.0, 0.0, 0.0, 0.0, 187935.2842, 4.39337676],
        [-216651.3208, 666.545615, 1.702064364, 0.000471499281, -1.427131823e-07, 2.016595903e-11, -9.107157762e-16, 183719.1966, 10.05690382],
        [-214383538.3, 146951.8523, -36.8086454, 0.00503616454, -3.087873854e-07, 9.18683487e-12, -1.074163268e-16, -961420.896, 342.619308],
    ],
    // e-: 298.15 to 1000.0 K, 1000.0 to 6000.0 K, 6000.0 to 20000.0 K
    [
        [0.0, 0.0, 2.5, 0.0, 0.0, 0.0, 0.0, -745.375, -11.72081224],
        [0.0, 0.0, 2.5, 0.0, 0.0, 0.0, 0.0, -745.375, -11.72081224],
        [0.0, 0.0, 2.5, 0.0, 0.0, 0.0, 0.0, -745.375, -11.72081224],
    ],
];

/// A limit of the air model, named by the caller in its own terms.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum AirLimit {
    /// A temperature above the fits' 20 000 K.
    TemperatureAboveFits,
    /// A temperature below a fit's floor.
    TemperatureBelowFits,
    /// The element potentials did not settle.
    EquilibriumNotFound,
    /// A value was NaN or infinite.
    NonFiniteValue,
}

/// Each species' cp/R, h/RT and s/R (1 bar) at one temperature.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Thermo {
    pub temperature_k: f64,
    pub cp_r: [f64; SPECIES],
    pub h_rt: [f64; SPECIES],
    pub s_r: [f64; SPECIES],
}

fn nasa(i: usize, t: f64) -> (f64, f64, f64) {
    let k = if t <= RANGE_ENDS_K[0] { 0 } else if t <= RANGE_ENDS_K[1] { 1 } else { 2 };
    let a = &NASA_GLENN[i][k];
    let ln_t = math::log(t);
    let t2 = t * t;
    let t3 = t2 * t;
    let t4 = t3 * t;
    let cp = a[0] / t2 + a[1] / t + a[2] + a[3] * t + a[4] * t2 + a[5] * t3 + a[6] * t4;
    let h = -a[0] / t2 + a[1] * ln_t / t + a[2] + a[3] * t / 2.0 + a[4] * t2 / 3.0 + a[5] * t3 / 4.0 + a[6] * t4 / 5.0 + a[7] / t;
    let s = -a[0] / t2 / 2.0 - a[1] / t + a[2] * ln_t + a[3] * t + a[4] * t2 / 2.0 + a[5] * t3 / 3.0 + a[6] * t4 / 4.0 + a[8];
    (cp, h, s)
}

/// Every species at `t`, which must lie inside every fit (298.15 K to 20 000 K).
pub fn thermo(t: f64) -> Result<Thermo, AirLimit> {
    if !t.is_finite() {
        return Err(AirLimit::NonFiniteValue);
    }
    if t > FIT_CEILING_K {
        return Err(AirLimit::TemperatureAboveFits);
    }
    if t < ALL_SPECIES_FLOOR_K {
        return Err(AirLimit::TemperatureBelowFits);
    }
    let mut out = Thermo { temperature_k: t, cp_r: [0.0; SPECIES], h_rt: [0.0; SPECIES], s_r: [0.0; SPECIES] };
    for i in 0..SPECIES {
        let (cp, h, s) = nasa(i, t);
        out.cp_r[i] = cp;
        out.h_rt[i] = h;
        out.s_r[i] = s;
    }
    Ok(out)
}

/// N2 or O2 alone, from 180 K: the fit from 200 K, the rigid rotor below.
pub fn diatomic_thermo(i: usize, t: f64) -> Result<(f64, f64, f64), AirLimit> {
    debug_assert!(i == N2 || i == O2);
    if !t.is_finite() {
        return Err(AirLimit::NonFiniteValue);
    }
    if t > FIT_CEILING_K {
        return Err(AirLimit::TemperatureAboveFits);
    }
    if t < RIGID_ROTOR_FLOOR_K {
        return Err(AirLimit::TemperatureBelowFits);
    }
    if t >= FIT_FLOOR_K[i] {
        return Ok(nasa(i, t));
    }
    let t0 = FIT_FLOOR_K[i];
    let (_, h0, s0) = nasa(i, t0);
    let cp = 3.5;
    let h = (h0 * t0 + cp * (t - t0)) / t;
    let s = s0 + cp * math::log(t / t0);
    Ok((cp, h, s))
}

/// Undissociated air of mass fractions `y_n2` and `y_o2` (they sum to one):
/// enthalpy J/kg, frozen cp J/(kg K), gas constant J/(kg K), entropy J/(kg K) at `p`.
pub fn undissociated_air(y_n2: f64, y_o2: f64, t: f64, p: f64) -> Result<(f64, f64, f64, f64), AirLimit> {
    let (cp1, h1, s1) = diatomic_thermo(N2, t)?;
    let (cp2, h2, s2) = diatomic_thermo(O2, t)?;
    let z1 = y_n2 / MOLAR_MASS_KG_PER_MOL[N2];
    let z2 = y_o2 / MOLAR_MASS_KG_PER_MOL[O2];
    let zt = z1 + z2;
    let r = GAS_CONSTANT_J_PER_MOL_K;
    let h = r * t * (z1 * h1 + z2 * h2);
    let cp = r * (z1 * cp1 + z2 * cp2);
    let mix = |z: f64, s: f64| if z > 0.0 { z * (s - math::log(z / zt * p / STANDARD_PRESSURE_PA)) } else { 0.0 };
    let s = r * (mix(z1, s1) + mix(z2, s2));
    Ok((h, cp, r * zt, s))
}

/// Mixture enthalpy, J/kg, of `z` mol/kg.
pub fn enthalpy(z: &[f64; SPECIES], th: &Thermo) -> f64 {
    let mut sum = 0.0;
    for i in 0..SPECIES {
        sum += z[i] * th.h_rt[i];
    }
    GAS_CONSTANT_J_PER_MOL_K * th.temperature_k * sum
}

/// Frozen heat capacity at constant pressure, J/(kg K).
pub fn frozen_cp(z: &[f64; SPECIES], th: &Thermo) -> f64 {
    let mut sum = 0.0;
    for i in 0..SPECIES {
        sum += z[i] * th.cp_r[i];
    }
    GAS_CONSTANT_J_PER_MOL_K * sum
}

/// Moles per kilogram.
pub fn moles(z: &[f64; SPECIES]) -> f64 {
    z.iter().sum()
}

/// Mixture entropy, J/(kg K), at pressure `p`. A species absent adds nothing.
pub fn entropy(z: &[f64; SPECIES], th: &Thermo, p: f64) -> f64 {
    let zt = moles(z);
    let mut sum = 0.0;
    for i in 0..SPECIES {
        if z[i] > 0.0 {
            sum += z[i] * (th.s_r[i] - math::log(z[i] / zt * p / STANDARD_PRESSURE_PA));
        }
    }
    GAS_CONSTANT_J_PER_MOL_K * sum
}

/// Nuclei of nitrogen and of oxygen, mol/kg, and the net charge, mol/kg.
pub fn nuclei_and_charge(z: &[f64; SPECIES]) -> [f64; 3] {
    let mut out = [0.0; 3];
    for i in 0..SPECIES {
        for e in 0..3 {
            out[e] += COMPOSITION[i][e] * z[i];
        }
    }
    out
}

/// Concentrations of every charged species, mol/kg: the ions' sum and the electrons.
pub fn charged(z: &[f64; SPECIES]) -> (f64, f64) {
    (z[N2_ION] + z[O2_ION] + z[NO_ION] + z[N_ION] + z[O_ION], z[ELECTRON])
}

/// An equilibrium composition.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Equilibrium {
    /// mol/kg of each species.
    pub z: [f64; SPECIES],
    /// The element potentials of nitrogen and oxygen, a start for the next solve.
    pub potentials: [f64; 2],
    pub iterations: u32,
}

/// The largest Newton iterations of one equilibrium solve.
pub const EQUILIBRIUM_MAX_ITERATIONS: u32 = 300;
/// The nuclei totals are held to this fraction of each.
pub const EQUILIBRIUM_TOLERANCE: f64 = 1.0e-13;

fn log_sum_exp(values: &[f64]) -> f64 {
    let top = values.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    if top == f64::NEG_INFINITY {
        return top;
    }
    top + math::log(values.iter().map(|v| math::exp(v - top)).sum::<f64>())
}

/// Equilibrium at temperature `t` and density `rho`, with `nuclei` mol/kg of
/// nitrogen and of oxygen nuclei, the charge zero.
pub fn equilibrium_t_rho(t: f64, rho: f64, nuclei: [f64; 2], start: Option<[f64; 2]>) -> Result<Equilibrium, AirLimit> {
    if !(rho.is_finite() && rho > 0.0) {
        return Err(AirLimit::NonFiniteValue);
    }
    let th = thermo(t)?;
    let r = GAS_CONSTANT_J_PER_MOL_K;
    let base = math::log(STANDARD_PRESSURE_PA / (r * t));
    let mut lnref = [0.0; SPECIES];
    for i in 0..SPECIES {
        lnref[i] = base - (th.h_rt[i] - th.s_r[i]);
    }
    let b = [nuclei[0] * rho, nuclei[1] * rho];
    let mut lam = match start {
        Some(l) => l,
        None => {
            // All nitrogen as N2 and all oxygen as O2 when cool, as atoms when hot.
            if t < 4000.0 {
                [0.5 * (math::log(0.5 * b[0]) - lnref[N2]), 0.5 * (math::log(0.5 * b[1]) - lnref[O2])]
            } else {
                [math::log(b[0]) - lnref[N], math::log(b[1]) - lnref[O]]
            }
        }
    };
    let mut c = [0.0; SPECIES];
    let mut iterations = 0;
    loop {
        iterations += 1;
        // Concentrations without the charge potential, in logarithms.
        let mut ln0 = [0.0; SPECIES];
        for i in 0..SPECIES {
            ln0[i] = lnref[i] + COMPOSITION[i][0] * lam[0] + COMPOSITION[i][1] * lam[1];
        }
        let ions = [ln0[N2_ION], ln0[O2_ION], ln0[NO_ION], ln0[N_ION], ln0[O_ION]];
        let lq = 0.5 * (ln0[ELECTRON] - log_sum_exp(&ions));
        for i in 0..SPECIES {
            c[i] = math::exp(ln0[i] + COMPOSITION[i][2] * lq);
        }
        let ion_sum: f64 = (N2_ION..=O_ION).map(|i| c[i]).sum();
        // d L_q / d L_k: neutrality moves the charge potential with the ions' nuclei.
        let mut dq = [0.0; 2];
        if ion_sum > 0.0 {
            for k in 0..2 {
                let mut s = 0.0;
                for i in N2_ION..=O_ION {
                    s += COMPOSITION[i][k] * c[i];
                }
                dq[k] = -0.5 * s / ion_sum;
            }
        }
        let mut res = [0.0; 2];
        let mut jac = [[0.0; 2]; 2];
        for i in 0..SPECIES {
            for j in 0..2 {
                res[j] += COMPOSITION[i][j] * c[i];
                for k in 0..2 {
                    jac[j][k] += COMPOSITION[i][j] * c[i] * (COMPOSITION[i][k] + COMPOSITION[i][2] * dq[k]);
                }
            }
        }
        res[0] -= b[0];
        res[1] -= b[1];
        if !(res[0].is_finite() && res[1].is_finite()) {
            return Err(AirLimit::EquilibriumNotFound);
        }
        let det = jac[0][0] * jac[1][1] - jac[0][1] * jac[1][0];
        if !(det.is_finite() && det != 0.0) {
            return Err(AirLimit::EquilibriumNotFound);
        }
        let mut d = [(-res[0] * jac[1][1] + res[1] * jac[0][1]) / det, (-res[1] * jac[0][0] + res[0] * jac[1][0]) / det];
        let largest = d[0].abs().max(d[1].abs());
        if largest > 1.0 {
            d[0] /= largest;
            d[1] /= largest;
        }
        let settled = (res[0] / b[0]).abs() <= EQUILIBRIUM_TOLERANCE && (res[1] / b[1]).abs() <= EQUILIBRIUM_TOLERANCE;
        if settled && largest <= 1.0e-10 {
            break;
        }
        lam[0] += d[0];
        lam[1] += d[1];
        if iterations >= EQUILIBRIUM_MAX_ITERATIONS {
            return Err(AirLimit::EquilibriumNotFound);
        }
    }
    let mut z = [0.0; SPECIES];
    for i in 0..SPECIES {
        z[i] = c[i] / rho;
    }
    Ok(Equilibrium { z, potentials: lam, iterations })
}

/// The pressure of an equilibrium held to this fraction: above the noise of
/// the nuclei totals, held to 1e-13.
pub const PRESSURE_TOLERANCE: f64 = 1.0e-12;

/// Equilibrium at temperature `t` and pressure `p`: the composition and the
/// density, by Newton's method on the logarithm of the density.
pub fn equilibrium_t_p(t: f64, p: f64, nuclei: [f64; 2], start: Option<[f64; 2]>) -> Result<(Equilibrium, f64), AirLimit> {
    if !(p.is_finite() && p > 0.0) {
        return Err(AirLimit::NonFiniteValue);
    }
    let r = GAS_CONSTANT_J_PER_MOL_K;
    // Undissociated air's density first.
    let mut ln_rho = math::log(p / (r * t * 0.5 * (nuclei[0] + nuclei[1])));
    let mut potentials = start;
    let mut total = 0;
    for _ in 0..60 {
        let rho = math::exp(ln_rho);
        let eq = equilibrium_t_rho(t, rho, nuclei, potentials)?;
        total += eq.iterations;
        potentials = Some(eq.potentials);
        let f = math::log(rho * r * t * moles(&eq.z) / p);
        if f.abs() <= PRESSURE_TOLERANCE {
            let mut eq = eq;
            eq.iterations = total;
            return Ok((eq, rho));
        }
        let d = 1.0e-6;
        let eq2 = equilibrium_t_rho(t, math::exp(ln_rho + d), nuclei, potentials)?;
        total += eq2.iterations;
        let f2 = math::log(math::exp(ln_rho + d) * r * t * moles(&eq2.z) / p);
        let slope = (f2 - f) / d;
        // d ln p / d ln rho lies between 1/2 and 1 for dissociating, ionizing air.
        let step = if slope.is_finite() && slope > 0.1 { -f / slope } else { -f };
        ln_rho += step.clamp(-2.0, 2.0);
    }
    Err(AirLimit::EquilibriumNotFound)
}

/// One reaction of the expanded Dunn-Kang set.
#[derive(Clone, Copy, Debug)]
pub struct Reaction {
    /// Its row in Dunn and Kang's Table I.
    pub row: u8,
    /// The third body named for this copy of a row that lists several.
    pub third_body: Option<usize>,
    pub reactants: [u8; SPECIES],
    pub products: [u8; SPECIES],
    /// ln of the forward coefficient in SI, m^3/(mol s) or m^6/(mol^2 s).
    pub ln_a_si: f64,
    pub temperature_exponent: f64,
    pub activation_k: f64,
    /// The backward rate as printed, cm-mole units: C, n, theta.
    pub printed_backward: (f64, f64, f64),
    /// The printed backward rate is the measured one (rows 7, 8, 9, 10, 13)
    /// and the forward rate is its partner by detailed balance.
    pub backward_measured: bool,
    /// ln of the printed backward coefficient in SI.
    pub ln_b_si: f64,
}

/// Rows whose deionization (backward) rate Dunn and Kang took from
/// experiment (CR-2232 p. 9).
pub const MEASURED_BACKWARD_ROWS: [u8; 5] = [7, 8, 9, 10, 13];

/// Dunn and Kang's Table I as printed: reactants, products, the forward
/// C (cm^3/(mole s)), n, theta (K), the printed backward C, n, theta, and the
/// third bodies where the row is `+ M`.
pub const DUNN_KANG_TABLE: [(u8, &[usize], &[usize], f64, f64, f64, (f64, f64, f64), &[usize]); 26] = [
    (1, &[O2], &[O, O], 3.6e18, -1.0, 5.95e4, (3.0e15, -0.5, 0.0), &[N, NO]),
    (2, &[N2], &[N, N], 1.9e17, -0.5, 1.13e5, (1.1e16, -0.5, 0.0), &[O, NO, O2]),
    (3, &[NO], &[N, O], 3.9e20, -1.5, 7.55e4, (1.0e20, -1.5, 0.0), &[O2, N2]),
    (4, &[O, NO], &[N, O2], 3.2e9, 1.0, 1.97e4, (1.3e10, 1.0, 3.58e3), &[]),
    (5, &[O, N2], &[N, NO], 7.0e13, 0.0, 3.8e4, (1.56e13, 0.0, 0.0), &[]),
    (6, &[N, N2], &[N, N, N], 4.085e22, -1.5, 1.13e5, (2.27e21, -1.5, 0.0), &[]),
    (7, &[O, N], &[NO_ION, ELECTRON], 1.4e6, 1.5, 3.19e4, (6.7e21, -1.5, 0.0), &[]),
    (8, &[O, ELECTRON], &[O_ION, ELECTRON, ELECTRON], 3.6e31, -2.91, 1.58e5, (2.2e40, -4.5, 0.0), &[]),
    (9, &[N, ELECTRON], &[N_ION, ELECTRON, ELECTRON], 1.1e32, -3.14, 1.69e5, (2.2e40, -4.5, 0.0), &[]),
    (10, &[O, O], &[O2_ION, ELECTRON], 1.6e17, -0.98, 8.08e4, (8.0e21, -1.5, 0.0), &[]),
    (11, &[O, O2_ION], &[O2, O_ION], 2.92e18, -1.11, 2.8e4, (7.8e11, 0.5, 0.0), &[]),
    (12, &[N2, N_ION], &[N, N2_ION], 2.02e11, 0.81, 1.3e4, (7.8e11, 0.5, 0.0), &[]),
    (13, &[N, N], &[N2_ION, ELECTRON], 1.4e13, 0.0, 6.78e4, (1.5e22, -1.5, 0.0), &[]),
    (14, &[O2, N2], &[NO, NO_ION, ELECTRON], 1.38e20, -1.84, 1.41e5, (1.0e24, -2.5, 0.0), &[]),
    (15, &[NO, N2], &[NO_ION, ELECTRON, N2], 2.2e15, -0.35, 1.08e5, (2.2e26, -2.5, 0.0), &[]),
    (16, &[O, NO_ION], &[NO, O_ION], 3.63e15, -0.6, 5.08e4, (1.5e13, 0.0, 0.0), &[]),
    (17, &[N2, O_ION], &[O, N2_ION], 3.4e19, -2.0, 2.3e4, (2.48e19, -2.2, 0.0), &[]),
    (18, &[N, NO_ION], &[NO, N_ION], 1.0e19, -0.93, 6.1e4, (4.8e14, 0.0, 0.0), &[]),
    (19, &[O2, NO_ION], &[NO, O2_ION], 1.8e15, 0.17, 3.3e4, (1.8e13, 0.5, 0.0), &[]),
    (20, &[O, NO_ION], &[O2, N_ION], 1.34e13, 0.31, 7.727e4, (1.0e14, 0.0, 0.0), &[]),
    (21, &[NO, O2], &[NO_ION, ELECTRON, O2], 8.8e15, -0.35, 1.08e5, (8.8e26, -2.5, 0.0), &[]),
    (22, &[O2, O], &[O, O, O], 9.0e19, -1.0, 5.95e4, (7.5e16, -0.5, 0.0), &[]),
    (23, &[O2, O2], &[O, O, O2], 3.24e19, -1.0, 5.95e4, (2.7e16, -0.5, 0.0), &[]),
    (24, &[O2, N2], &[O, O, N2], 7.2e18, -1.0, 5.95e4, (6.0e15, -0.5, 0.0), &[]),
    (25, &[N2, N2], &[N, N, N2], 4.7e17, -0.5, 1.13e5, (2.72e16, -0.5, 0.0), &[]),
    (26, &[NO], &[N, O], 7.8e20, -1.5, 7.55e4, (2.0e20, -1.5, 0.0), &[O, N, NO]),
];

/// The reactions, each third body of a `+ M` row its own: 32 in all.
pub fn reactions() -> &'static [Reaction] {
    static SET: OnceLock<Vec<Reaction>> = OnceLock::new();
    SET.get_or_init(|| {
        let mut out = Vec::new();
        for (row, reactants, products, c, n, theta, back, bodies) in DUNN_KANG_TABLE.iter() {
            let copies: Vec<Option<usize>> = if bodies.is_empty() { vec![None] } else { bodies.iter().map(|m| Some(*m)).collect() };
            for m in copies {
                let mut nr = [0u8; SPECIES];
                let mut np = [0u8; SPECIES];
                for s in reactants.iter() {
                    nr[*s] += 1;
                }
                for s in products.iter() {
                    np[*s] += 1;
                }
                if let Some(m) = m {
                    nr[m] += 1;
                    np[m] += 1;
                }
                let order: u8 = nr.iter().sum();
                let order_b: u8 = np.iter().sum();
                // cm^3 = 1e-6 m^3: a reaction of order k carries (1e-6)^(k-1).
                let ln_a_si = math::log(*c) + f64::from(order - 1) * math::log(1.0e-6_f64);
                let ln_b_si = math::log(back.0) + f64::from(order_b - 1) * math::log(1.0e-6_f64);
                out.push(Reaction {
                    row: *row,
                    third_body: m,
                    reactants: nr,
                    products: np,
                    ln_a_si,
                    temperature_exponent: *n,
                    activation_k: *theta,
                    printed_backward: *back,
                    backward_measured: MEASURED_BACKWARD_ROWS.contains(row),
                    ln_b_si,
                });
            }
        }
        out
    })
}

/// ln of the equilibrium constant in concentrations (SI) of one reaction.
pub fn ln_kc(reaction: &Reaction, th: &Thermo) -> f64 {
    let mut d_g = 0.0;
    let mut d_nu = 0.0;
    for i in 0..SPECIES {
        let nu = f64::from(reaction.products[i]) - f64::from(reaction.reactants[i]);
        d_g += nu * (th.h_rt[i] - th.s_r[i]);
        d_nu += nu;
    }
    -d_g + d_nu * math::log(STANDARD_PRESSURE_PA / (GAS_CONSTANT_J_PER_MOL_K * th.temperature_k))
}

/// ln of the printed forward coefficient, SI.
pub fn ln_kf_printed(reaction: &Reaction, t: f64) -> f64 {
    reaction.ln_a_si + reaction.temperature_exponent * math::log(t) - reaction.activation_k / t
}

/// ln of the printed backward coefficient, SI.
pub fn ln_kb_printed(reaction: &Reaction, t: f64) -> f64 {
    let (_, n, theta) = reaction.printed_backward;
    reaction.ln_b_si + n * math::log(t) - theta / t
}

/// ln of the forward and backward coefficients used: the measured direction
/// as printed, the other by detailed balance.
pub fn ln_rates(reaction: &Reaction, th: &Thermo) -> (f64, f64) {
    let t = th.temperature_k;
    let kc = ln_kc(reaction, th);
    if reaction.backward_measured {
        let b = ln_kb_printed(reaction, t);
        (b + kc, b)
    } else {
        let f = ln_kf_printed(reaction, t);
        (f, f - kc)
    }
}

fn power(c: f64, k: u8) -> f64 {
    match k {
        0 => 1.0,
        1 => c,
        2 => c * c,
        _ => c * c * c,
    }
}

/// Net production of each species, mol/(m^3 s), at concentrations `c`
/// mol/m^3; with each reaction's forward and backward rates when asked.
pub fn production(th: &Thermo, c: &[f64; SPECIES], each: Option<&mut Vec<(f64, f64)>>) -> [f64; SPECIES] {
    let mut w = [0.0; SPECIES];
    let mut each = each;
    for r in reactions() {
        let (lf, lb) = ln_rates(r, th);
        let kf = math::exp(lf);
        let kb = math::exp(lb);
        let mut rf = kf;
        let mut rb = kb;
        for i in 0..SPECIES {
            rf *= power(c[i], r.reactants[i]);
            rb *= power(c[i], r.products[i]);
        }
        let net = rf - rb;
        for i in 0..SPECIES {
            w[i] += (f64::from(r.products[i]) - f64::from(r.reactants[i])) * net;
        }
        if let Some(v) = each.as_deref_mut() {
            v.push((rf, rb));
        }
    }
    w
}

#[cfg(test)]
mod tests {
    use super::*;

    const AIR_NUCLEI: [f64; 2] = [54.648212681, 14.660479740];

    /// The fits are exact at 298.15 K (McBride et al., constraint 1): each
    /// species' enthalpy there is its heat of formation as `thermo.inp` lists it.
    #[test]
    fn each_fit_gives_its_heat_of_formation_at_298_15_k() {
        let listed = [0.0, 0.0, 91271.310, 472680.000, 249175.003, 1509508.424, 1171828.436, 990809.704, 1882127.624, 1568787.228, 0.0];
        let th = thermo(298.15).unwrap();
        for i in 0..SPECIES {
            let h = th.h_rt[i] * GAS_CONSTANT_J_PER_MOL_K * 298.15;
            assert!((h - listed[i]).abs() < 2.0, "{}: {h} J/mol against {}", SPECIES_NAMES[i], listed[i]);
        }
    }

    /// Two contiguous ranges give the same values where they meet (constraint 2).
    #[test]
    fn the_ranges_meet() {
        for i in 0..SPECIES {
            for (k, t) in RANGE_ENDS_K.iter().enumerate() {
                let eval = |row: usize| {
                    let a = &NASA_GLENN[i][row];
                    let ln_t = t.ln();
                    let h = -a[0] / (t * t) + a[1] * ln_t / t + a[2] + a[3] * t / 2.0 + a[4] * t * t / 3.0 + a[5] * t.powi(3) / 4.0 + a[6] * t.powi(4) / 5.0 + a[7] / t;
                    let s = -a[0] / (t * t) / 2.0 - a[1] / t + a[2] * ln_t + a[3] * t + a[4] * t * t / 2.0 + a[5] * t.powi(3) / 3.0 + a[6] * t.powi(4) / 4.0 + a[8];
                    (h * t, s)
                };
                let (h0, s0) = eval(k);
                let (h1, s1) = eval(k + 1);
                assert!((h1 - h0).abs() < 0.01, "{} at {t} K: h/R jumps {} K", SPECIES_NAMES[i], h1 - h0);
                assert!((s1 - s0).abs() < 1.0e-5, "{} at {t} K: s/R jumps {}", SPECIES_NAMES[i], s1 - s0);
            }
        }
    }

    /// The company's own derivation: atomic oxygen at 10 000 K and 1 bar from
    /// the Sackur-Tetrode equation and its electronic levels (3P2,1,0 at 0,
    /// 158.265, 226.977 cm^-1; 1D at 15867.862; 1S at 33792.583; 5S at
    /// 73768.200; 3S at 76794.978; NIST Atomic Spectra Database, retrieved 2026-10-15), against the fit.
    #[test]
    fn oxygen_entropy_is_statistical_mechanics() {
        let t: f64 = 10_000.0;
        let c2 = 1.438776877; // cm K, second radiation constant
        let levels = [(5.0, 0.0), (3.0, 158.265), (1.0, 226.977), (5.0, 15867.862), (1.0, 33792.583), (5.0, 73768.2), (3.0, 76794.978)];
        let q: f64 = levels.iter().map(|(g, e)| g * (-c2 * e / t).exp()).sum();
        let mean: f64 = levels.iter().map(|(g, e)| g * (c2 * e / t) * (-c2 * e / t).exp()).sum::<f64>() / q;
        // Sackur-Tetrode at 1 bar: S/R = 5/2 ln T + 3/2 ln M(g/mol) + S0/R, S0/R = -1.15170753706 (CODATA 2018).
        let st = 2.5 * t.ln() + 1.5 * 15.9994_f64.ln() - 1.151_707_537_06;
        let s = st + q.ln() + mean;
        let th = thermo(t).unwrap();
        assert!((th.s_r[O] - s).abs() < 2.0e-3, "fit {} against {s}", th.s_r[O]);
    }

    #[test]
    fn equilibrium_holds_nuclei_and_charge() {
        for &(t, rho) in &[(300.0, 1.2), (2000.0, 1.0e-3), (6200.0, 1.4e-3), (9000.0, 1.0e-4), (15000.0, 1.0e-2), (19000.0, 1.0e-5)] {
            let eq = equilibrium_t_rho(t, rho, AIR_NUCLEI, None).unwrap();
            let n = nuclei_and_charge(&eq.z);
            assert!((n[0] / AIR_NUCLEI[0] - 1.0).abs() < 1.0e-12, "{t}: {n:?}");
            assert!((n[1] / AIR_NUCLEI[1] - 1.0).abs() < 1.0e-12, "{t}: {n:?}");
            let (ions, e) = charged(&eq.z);
            assert!((ions - e).abs() <= 1.0e-12 * e.max(1.0e-300), "{t}: {ions} against {e}");
        }
    }

    /// Detailed balance: at the equilibrium composition each reaction's
    /// forward and backward rates are equal, so the kinetics and the
    /// equilibrium are one model.
    #[test]
    fn at_equilibrium_every_reaction_balances() {
        for &(t, rho) in &[(3000.0, 1.0e-2), (6200.0, 1.4e-3), (12000.0, 1.0e-4)] {
            let eq = equilibrium_t_rho(t, rho, AIR_NUCLEI, None).unwrap();
            let mut c = [0.0; SPECIES];
            for i in 0..SPECIES {
                c[i] = eq.z[i] * rho;
            }
            let th = thermo(t).unwrap();
            let mut each = Vec::new();
            production(&th, &c, Some(&mut each));
            for (k, (rf, rb)) in each.iter().enumerate() {
                assert!((rf - rb).abs() <= 1.0e-9 * rf.max(*rb).max(1.0e-300), "reaction {k} at {t} K: {rf} against {rb}");
            }
        }
    }

    /// Equilibrium is the minimum of the Gibbs energy: moving along any
    /// reaction from it raises the Gibbs energy at the same T and volume.
    #[test]
    fn equilibrium_is_the_gibbs_minimum() {
        let (t, rho) = (6500.0, 2.0e-3);
        let eq = equilibrium_t_rho(t, rho, AIR_NUCLEI, None).unwrap();
        let th = thermo(t).unwrap();
        let r = GAS_CONSTANT_J_PER_MOL_K;
        // Helmholtz energy per volume / RT at fixed T, V: sum c (g/RT + ln(c RT/p0) - 1).
        let a = |c: &[f64; SPECIES]| {
            let mut s = 0.0;
            for i in 0..SPECIES {
                if c[i] > 0.0 {
                    s += c[i] * ((th.h_rt[i] - th.s_r[i]) + (c[i] * r * t / STANDARD_PRESSURE_PA).ln() - 1.0);
                }
            }
            s
        };
        let mut c0 = [0.0; SPECIES];
        for i in 0..SPECIES {
            c0[i] = eq.z[i] * rho;
        }
        let f0 = a(&c0);
        for re in reactions() {
            for sign in [-1.0, 1.0] {
                let mut c = c0;
                let mut step = f64::MAX;
                for i in 0..SPECIES {
                    let nu = f64::from(re.products[i]) - f64::from(re.reactants[i]);
                    if nu != 0.0 {
                        step = step.min(1.0e-4 * c0[i] / nu.abs());
                    }
                }
                for i in 0..SPECIES {
                    c[i] += sign * step * (f64::from(re.products[i]) - f64::from(re.reactants[i]));
                }
                assert!(a(&c) >= f0 - 1.0e-12 * f0.abs(), "row {}", re.row);
            }
        }
    }

    /// The direction not taken as printed, against Dunn and Kang's printed
    /// value, from 4000 K to 10 000 K: printed over derived, in the bounds
    /// below. Most rows agree within a factor of 1.5. The rows that do not:
    /// 7 (N + O = NO+ + e: the derived forward rate is 6.4, 3.7, 2.5 and 1.8
    /// times the printed one at 4000, 6000, 8000 and 10 000 K), 9 (the derived
    /// forward rate of electron-impact ionization of N is about 12 times the
    /// printed), 10 (O + O = O2+ + e: the printed forward is 4 to 15 times the
    /// derived), 13 (the derived forward is 1.2 to 2.1 times the printed), 14
    /// (4.2 at 4000 K), 16 and 18 (to 3), 20 (1.6), and 12, 15 and 21 (to 0.36).
    #[test]
    fn the_direction_derived_is_set_against_the_one_printed() {
        for r in reactions() {
            for &t in &[4000.0, 6000.0, 8000.0, 10_000.0] {
                let th = thermo(t).unwrap();
                let (lf, lb) = ln_rates(r, &th);
                let ratio = if r.backward_measured { (ln_kf_printed(r, t) - lf).exp() } else { (ln_kb_printed(r, t) - lb).exp() };
                let (low, high) = match r.row {
                    7 => (0.15, 0.6),
                    13 => (0.45, 0.9),
                    20 => (1.2, 1.7),
                    9 => (1.0 / 13.0, 1.0 / 11.0),
                    10 => (4.0, 16.0),
                    14 => (1.2, 4.3),
                    12 | 15 | 21 => (0.35, 0.8),
                    16 | 18 => (1.2, 3.0),
                    _ => (0.66, 1.6),
                };
                assert!(ratio >= low && ratio <= high, "row {} ({:?}) at {t} K: printed over derived {ratio}", r.row, r.third_body);
            }
        }
    }

    #[test]
    fn the_set_has_thirty_two_reactions_that_conserve_nuclei_and_charge() {
        assert_eq!(reactions().len(), 32);
        for r in reactions() {
            for e in 0..3 {
                let mut before = 0.0;
                let mut after = 0.0;
                for i in 0..SPECIES {
                    before += COMPOSITION[i][e] * f64::from(r.reactants[i]);
                    after += COMPOSITION[i][e] * f64::from(r.products[i]);
                }
                assert_eq!(before, after, "row {}", r.row);
            }
        }
    }

    #[test]
    fn a_state_past_the_fits_is_named() {
        assert_eq!(thermo(20_000.000_001), Err(AirLimit::TemperatureAboveFits));
        assert_eq!(thermo(298.0), Err(AirLimit::TemperatureBelowFits));
        assert_eq!(diatomic_thermo(N2, 179.9), Err(AirLimit::TemperatureBelowFits));
        assert!(diatomic_thermo(O2, 180.65).is_ok());
        assert_eq!(thermo(f64::NAN), Err(AirLimit::NonFiniteValue));
    }
}

