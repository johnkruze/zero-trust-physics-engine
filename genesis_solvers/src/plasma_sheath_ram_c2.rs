//! The plasma sheath of the RAM C-II blunt cone at one flight point: the
//! shock layer, its electron density, and whether a radio signal is blacked
//! out.
//!
//! **The vehicle.** RAM C-II, NASA Langley's re-entry probe flown on 22
//! August 1968 (Grantham, NASA TN D-6062, 1970): a 15.24 cm (6 in) radius
//! hemispherical nose on a 9 degree half-angle cone, 30.48 cm reference
//! diameter, a beryllium heat-sink nose cap that kept the nose free of
//! ablation products down to 56.39 km, where the cap was ejected. Two body
//! stations of its microwave reflectometers are served: station 1 on the
//! nose, 4.47 cm along the axis from the nose (x/D 0.15, 45.0 degrees around
//! the sphere), and station 2 on the cone, 23.16 cm along the axis (x/D 0.76)
//! (TN D-6062 Tables I and III).
//!
//! **The question.** An altitude, a flight speed through the air, and a
//! signal frequency. The air is the U.S. Standard Atmosphere, 1962, the
//! source's own choice (TN D-6062 p. 16): molecular-scale temperature
//! straight in geopotential altitude through its layers to 90 km, the
//! pressure from the hydrostatic law, molecular weight 28.9644, R* 8.31432,
//! the mean free path `1/(sqrt(2) pi sigma^2 n)` with sigma = 3.65e-10 m.
//! The air entering the shock is N2 and O2 in the standard's ratio, 78.084
//! to 20.9476 by volume, argon and the trace gases left out; its density is
//! the standard's, so the mass, momentum and energy carried into the shock
//! are the standard's.
//!
//! **The physics, in order.**
//!
//! 1. *The normal shock* on the axis, with the air in chemical equilibrium
//!    behind it: the Rankine-Hugoniot conservation of mass, momentum and
//!    energy, `rho1 V = rho2 u2`, `p1 + rho1 V^2 = p2 + rho2 u2^2`,
//!    `h1 + V^2/2 = h2 + u2^2/2`, with `h2`, `p2` from equilibrium air
//!    (`air11`). The density ratio is the root of one equation, bracketed
//!    and settled to 1e-14.
//! 2. *The stagnation point*: the gas behind the shock brought to rest
//!    without loss, in equilibrium: the state with the total enthalpy and
//!    the entropy behind the shock, settled by Newton's method to 1e-13.
//! 3. *The body streamline*: the gas that passed the stagnation point flows
//!    along the wall, outside the boundary layer. Its pressure is the
//!    modified Newtonian `p = p_inf + (p0 - p_inf) cos^2(phi)` on the sphere
//!    (phi the angle from the axis) and `p_inf + (p0 - p_inf) sin^2(9 deg)` on
//!    the cone, continuous at the tangent point (Lees 1955). It leaves the
//!    stagnation point in equilibrium and reacts at finite rate as it
//!    expands, by the 32 reactions of Dunn and Kang (`air11`), one
//!    temperature. Along it, in time:
//!
//!    ```text
//!    dz_i/dt = w_i(T, rho z) / rho                 species, mol/kg
//!    du/dt   = -(dp/ds) / rho                       momentum
//!    dT/dt   = [(dp/ds) u / rho - sum h_i dz_i/dt] / cp_frozen   energy, dh = dp/rho
//!    ds/dt   = u
//!    ```
//!
//!    started at one thousandth of the nose radius from the stagnation
//!    point, where the speed is the Newtonian `beta s`,
//!    `beta = sqrt(2 (p0 - p_inf)/rho0) / R`. Where it starts does not move
//!    the answer: from a hundredth to a ten-thousandth of the radius the
//!    stations' electron density moves by less than 1e-4. The electrons are
//!    the ions' sum, so the gas is neutral by construction; the speed and
//!    the surface distance are carried as logarithms, since near the
//!    stagnation point both grow as `exp(beta t)`. The equations are stiff,
//!    and are stepped by the two-stage L-stable Rosenbrock method ROS2
//!    (Verwer, Spee, Blom and Hundsdorfer, SIAM J. Sci. Comput. 20, 1999),
//!    second order with any Jacobian, the Jacobian by forward differences,
//!    each step held by the embedded first-order solution to a relative
//!    tolerance and to moving the parcel's distance and speed by at most a
//!    tenth. The tolerance is lowered from 1e-5 by factors of ten, to 1e-8,
//!    until two successive tolerances agree on each station's electron
//!    density and temperature to 1e-4; a tolerance whose ledger breaks is
//!    refined. At a tolerance of 1e-6 the streamline resolves electrons to
//!    1e8 per cubic metre (`ELECTRON_RESOLUTION_M3`); a density below it is
//!    reported as computed, and is not resolved.
//! 4. *The electron density and the blackout.* The plasma frequency is
//!    `f_p = sqrt(n_e e^2/(eps0 m_e)) / (2 pi)`. A signal of frequency `f` is
//!    blacked out where `f < f_p`: its electron density is above the critical
//!    density `n_c = 4 pi^2 eps0 m_e f^2 / e^2`. That is the collisionless
//!    cut-off; collisions blur it where the electron collision frequency
//!    nears the wave's (TN D-6062 places this below 30.48 km).
//!
//! The stations also carry two bounds of the expansion: the gas held in
//! equilibrium (the fastest chemistry) and its composition frozen at the
//! stagnation point's (no chemistry), both isentropic.
//!
//! **What holds, and where it is held.**
//!
//! - Across the shock: mass, momentum and energy fluxes, each to
//!   `SHOCK_LEDGER_TOLERANCE` of its own size, at acceptance.
//! - At the stagnation point: total enthalpy and entropy, to
//!   `STAGNATION_LEDGER_TOLERANCE`.
//! - At every accepted step of the streamline: the nuclei of nitrogen and
//!   of oxygen per kilogram, to `NUCLEI_TOLERANCE`; the charge: the gas is
//!   neutral by construction, its electrons the ions' sum, and the 32
//!   reactions must make as many electrons as ions, to `CHARGE_TOLERANCE` of
//!   the gross rate of the reactions of charged particles; the total enthalpy
//!   `h + u^2/2`, to `ENERGY_LEDGER_TOLERANCE` of the flight's kinetic energy
//!   per kilogram, `V^2/2`, which the energy
//!   equation (in `dh = dp/rho`) and the momentum equation hold together
//!   only if both are stepped right; and the second law: the entropy of the
//!   gas never falls below the highest it has reached, to
//!   `ENTROPY_TOLERANCE` of itself.
//!
//! **Its limits, each a physical or mathematical one.**
//!
//! - *Rarefied flow*: the freestream Knudsen number `lambda/R_n` above 0.03,
//!   above 80 637 m (`KNUDSEN_ALTITUDE_M`, where it reaches 0.03, to the
//!   metre below). The shock is a few mean free paths thick and the layer
//!   behind it about the density ratio times the nose radius, 5 to 6
//!   percent of it here: past 0.03 the shock is thicker than the layer it
//!   bounds, and the jump the model rests on has no room. From 0.01 (about
//!   73.6 km) the shock is already as thick as the layer, and the answer says
//!   so (`shock_as_thick_as_layer`).
//! - *No bow shock*: a flight Mach number of 1 or less.
//! - *Past the fits at every altitude*: a flight speed above 20 000 m/s.
//! - *The electrons' resolution*: a signal below 1 MHz, whose critical
//!   density nears the 1e8 per cubic metre to which electrons are resolved.
//! - *The fits*: any state above 20 000 K or below the ions' floor of
//!   298.15 K, where the NASA Glenn polynomials stop.
//! - *Below sea level*.
//!
//! **What it leaves out, said plainly.** The model is inviscid: the cold
//! wall's boundary layer, where electrons diffuse to the wall and recombine,
//! is not in it, nor ambipolar diffusion across the layer. The stagnation
//! point is in equilibrium: the gas there is taken to have had time to
//! react. It is one-temperature, as Dunn and Kang's model is. The air is
//! clean: no ablation products. Each of these matters most at high altitude,
//! where the layer is thin and viscous and the chemistry slow.
//!
//! **The comparison of 2026-10-15**, run once with this file as
//! fingerprinted (protocol SHA-256 `9f858090...`, committed with the solver
//! as `36c069b` before the measurement was opened). Against the onsets of
//! RAM C-II's reflectometers at stations 1 and 2 (TN D-6062 Table X: where
//! the reflection crossed 20 percent, the layer's peak density being 0.63 of
//! the band's critical density), gated where the measured altitude lies
//! from 56.39 to 80.64 km, at 7.65 km/s: station 2's X band at 70.0 km, the
//! solver 0.94 of the measured density; station 2's S band at 77.7 km, 3.28
//! times; station 1's X band at 80.0 km, 2.92 times. One of the three within
//! a factor of 2, against two needed, and none beyond 4: by its rule the
//! comparison missed. The two missed lie above Kn 0.01, where the shock is as
//! thick as the layer. Not gated: station 1's Ka onset at 72 km (not read
//! from the 20 percent criterion) 0.81; station 2's Ka onset at 38 km, below
//! the cap's ejection, 0.57; the decays, 22 to 31 km, overstated 1.1 to 2000
//! times where the flown nose ablated and the antennas detuned. The onsets at
//! 84 km (station 1's S band, station 2's L band) are refused as
//! `RarefiedFlow`. Record: `evidence/research/hgv_plasma_ramc2/`.
//!
//! **Sources** (the files are in `evidence/research/hgv_plasma_ramc2/raw`
//! and `evidence/research/scout_2026-10-14/hgv_plasma/raw`):
//! W. L. Grantham, NASA TN D-6062 (1970), the vehicle and its stations;
//! U.S. Standard Atmosphere, 1962 (NTRS 19630003300), the air; M. G. Dunn and
//! S.-W. Kang, NASA CR-2232 (1973), Table I, the reactions; B. J. McBride,
//! M. J. Zehe and S. Gordon, NASA TP-2002-211556, and NASA's CEA
//! `thermo.inp`, the thermodynamics; P. W. Huber, NASA TR R-163 (1963), the
//! equilibrium shock against which a test holds this one.

#![allow(clippy::needless_range_loop)]

use genesis_kernel::math;
use crate::normal::NormalOrZero;
use crate::air11::{self, AirLimit, ELECTRON, SPECIES};

/// Nose radius, 6 in (TN D-6062 p. 5).
pub const NOSE_RADIUS_M: f64 = 0.1524;
/// Cone half-angle (TN D-6062 p. 5).
pub const CONE_HALF_ANGLE_DEG: f64 = 9.0;
/// Reference diameter D, 12 in (TN D-6062 Table I).
pub const REFERENCE_DIAMETER_M: f64 = 0.3048;
/// The reflectometer stations' distances along the axis from the nose,
/// 1.76 in and 9.12 in (TN D-6062 Tables I and III): x/D 0.15 and 0.76.
pub const STATION_X_M: [f64; 2] = [0.044704, 0.231648];
/// The stations' names as the source gives them.
pub const STATION_NAMES: [&str; 2] = ["station 1, x/D 0.15, on the nose", "station 2, x/D 0.76, on the cone"];
/// The altitude the beryllium nose cap was ejected at, below which the flown
/// nose ablated (TN D-6062 p. 5). Stated, not used.
pub const NOSE_CAP_EJECTION_ALTITUDE_M: f64 = 56_390.0;

/// U.S. Standard Atmosphere, 1962, Table I.2.1 and Table I.4(e).
pub const SEA_LEVEL_PRESSURE_PA: f64 = 101_325.0;
pub const STANDARD_GRAVITY_M_S2: f64 = 9.80665;
pub const AIR_MOLECULAR_WEIGHT: f64 = 28.9644;
pub const UNIVERSAL_GAS_CONSTANT_1962: f64 = 8.31432;
pub const COLLISION_DIAMETER_M: f64 = 3.65e-10;
pub const AVOGADRO_1962_PER_KMOL: f64 = 6.02257e26;
/// The radius in `H = r0 Z / (r0 + Z)`.
pub const GEOPOTENTIAL_RADIUS_M: f64 = 6_356_766.0;
/// Layer bases in geopotential metres, their molecular-scale temperatures,
/// and the gradients above them, K per geopotential metre, to 90 km.
pub const LAYERS: [(f64, f64, f64); 8] = [
    (0.0, 288.15, -6.5e-3),
    (11_000.0, 216.65, 0.0),
    (20_000.0, 216.65, 1.0e-3),
    (32_000.0, 228.65, 2.8e-3),
    (47_000.0, 270.65, 0.0),
    (52_000.0, 270.65, -2.0e-3),
    (61_000.0, 252.65, -4.0e-3),
    (79_000.0, 180.65, 0.0),
];
/// The top of the atmosphere as computed here, 90 km geometric: the
/// standard's molecular weight is constant below it.
pub const ATMOSPHERE_TOP_M: f64 = 90_000.0;
/// The air entering the shock, mole fractions of N2 and O2 (the standard's
/// Table I.2.7), before argon and the trace gases are left out.
pub const AIR_MOLE_FRACTIONS: [f64; 2] = [0.78084, 0.209476];

/// Past this freestream Knudsen number the shock is thicker than the layer.
pub const KNUDSEN_LIMIT: f64 = 0.03;
/// The altitude at which the 1962 standard's Knudsen number reaches 0.03,
/// 80 637.77 m, to the metre below: the solver answers up to it.
pub const KNUDSEN_ALTITUDE_M: f64 = 80_637.0;
/// From this one the shock is as thick as the layer: reported.
pub const KNUDSEN_SHOCK_AS_THICK_AS_LAYER: f64 = 0.01;

/// CODATA 2018: elementary charge (exact), electron mass, vacuum permittivity.
pub const ELEMENTARY_CHARGE_C: f64 = 1.602176634e-19;
pub const ELECTRON_MASS_KG: f64 = 9.1093837015e-31;
pub const VACUUM_PERMITTIVITY_F_M: f64 = 8.8541878128e-12;

/// The reflectometer bands of RAM C-II (TN D-6062 Table I), Hz.
pub const REFLECTOMETER_BANDS_HZ: [(&str, f64); 4] = [("L", 1.116e9), ("S", 3.348e9), ("X", 10.044e9), ("Ka", 35.0e9)];
/// Where a reflectometer's reflection coefficient crosses 20 percent, the
/// layer's peak electron density is this fraction of the critical density
/// (TN D-6062).
pub const REFLECTOMETER_CRITERION: f64 = 0.63;

/// The streamline starts this fraction of the nose radius from the axis.
pub const START_FRACTION: f64 = 1.0e-3;
/// The step tolerances tried, in order.
pub const STEP_TOLERANCES: [f64; 4] = [1.0e-5, 1.0e-6, 1.0e-7, 1.0e-8];
/// Two successive tolerances that agree on each station's electron density
/// and temperature to this fraction settle the answer.
pub const SETTLE_TOLERANCE: f64 = 1.0e-4;
/// ...or on the electron density to within the resolution below.
pub const SETTLE_FLOOR_M3: f64 = ELECTRON_RESOLUTION_M3;
/// The electron density the streamline resolves at a step tolerance of
/// 1e-6, per cubic metre: two orders below the critical density of a 1 MHz
/// wave, 1.24e10 per cubic metre. Below it a density is reported as
/// computed and is not resolved.
pub const ELECTRON_RESOLUTION_M3: f64 = 1.0e8;
/// The lowest signal frequency: its critical density is 124 times the
/// resolution.
pub const SIGNAL_FREQUENCY_FLOOR_HZ: f64 = 1.0e6;
/// The fastest flight answered: above it the gas behind the shock is past
/// the fits' 20 000 K at every altitude from sea level to 80 637 m (at the
/// top, where the shock is coolest, from between 19 250 and 19 500 m/s).
pub const VELOCITY_CEILING_M_S: f64 = 20_000.0;
/// The highest: its critical density, 1.24e298 per cubic metre, is near the
/// largest number 64-bit arithmetic carries.
pub const SIGNAL_FREQUENCY_CEILING_HZ: f64 = 1.0e150;
/// The streamline's absolute tolerance of the neutral species, mol/kg, per
/// unit of step tolerance.
pub const NEUTRAL_ABSOLUTE_TOLERANCE: f64 = 1.0e-6;
/// Across the shock, each flux to this fraction of itself.
pub const SHOCK_LEDGER_TOLERANCE: f64 = 1.0e-10;
/// At the stagnation point, enthalpy and entropy to this fraction.
pub const STAGNATION_LEDGER_TOLERANCE: f64 = 1.0e-10;
/// Nuclei per kilogram held to this fraction at every step.
pub const NUCLEI_TOLERANCE: f64 = 1.0e-10;
/// The electrons the reactions make against the ions they make, held to
/// this fraction of the gross rate of the reactions of charged particles at
/// every step.
pub const CHARGE_TOLERANCE: f64 = 1.0e-12;
/// Total enthalpy held to this fraction of the flight's kinetic energy per
/// kilogram, V^2/2, at every step.
pub const ENERGY_LEDGER_TOLERANCE: f64 = 1.0e-5;
/// The entropy falls below the highest it has reached by no more than this
/// fraction of itself: the chemistry only raises it, so a fall is the
/// steps' own error.
pub const ENTROPY_TOLERANCE: f64 = 1.0e-6;
/// The most accepted steps of one integration.
pub const MAX_STEPS: u32 = 400_000;
/// The largest change of ln s or ln u in one step.
pub const MAX_LOG_STRIDE: f64 = 0.1;
/// The smallest step, s.
pub const MIN_STEP_S: f64 = 1.0e-18;

pub const REFERENCE_NAME: &str = "W. L. Grantham, Flight results of a 25 000-foot-per-second reentry experiment using microwave reflectometers to measure plasma electron density and standoff distance, NASA TN D-6062 (1970): the RAM C-II vehicle, its reflectometer stations and bands, and its trajectory";
pub const REFERENCE_URL: &str = "https://ntrs.nasa.gov/citations/19710004000";
pub const REFERENCE_RETRIEVED: &str = "2026-10-14";

/// The streamline carries the ten heavy species; the electrons are the ions'
/// sum, so the gas is neutral by construction, and what is held instead is
/// that the reactions make as many electrons as ions.
const HEAVY: usize = SPECIES - 1;
const N_STATE: usize = HEAVY + 3;
const T_IX: usize = HEAVY;
const U_IX: usize = HEAVY + 1;
const S_IX: usize = HEAVY + 2;

fn species_of(y: &[f64; N_STATE]) -> [f64; SPECIES] {
    let mut z = [0.0; SPECIES];
    z[..HEAVY].copy_from_slice(&y[..HEAVY]);
    z[ELECTRON] = z[air11::N2_ION] + z[air11::O2_ION] + z[air11::NO_ION] + z[air11::N_ION] + z[air11::O_ION];
    z
}

/// What the caller asks.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PlasmaSheathRamC2Question {
    /// Geometric altitude, m.
    pub altitude_m: f64,
    /// Flight speed through the air, m/s.
    pub velocity_m_s: f64,
    /// The signal whose blackout is asked, Hz.
    pub signal_frequency_hz: f64,
}

/// One body station.
#[derive(Clone, Debug, PartialEq)]
pub struct Station {
    pub x_m: f64,
    pub surface_distance_m: f64,
    pub pressure_pa: f64,
    /// Finite-rate expansion from the stagnation point: the answer.
    pub temperature_k: f64,
    pub velocity_m_s: f64,
    pub electron_density_m3: f64,
    /// The density is at or above `ELECTRON_RESOLUTION_M3`; below it the
    /// number is reported as computed and is not resolved, and may be small
    /// and of either sign.
    pub electron_density_resolved: bool,
    pub plasma_frequency_hz: f64,
    /// The same expansion held in equilibrium.
    pub equilibrium_electron_density_m3: f64,
    pub equilibrium_temperature_k: f64,
    /// The same expansion with the stagnation point's composition frozen.
    pub frozen_electron_density_m3: f64,
    pub frozen_temperature_k: f64,
    /// Time from the start of the streamline to the station.
    pub transit_time_s: f64,
    pub signal_blacked_out: bool,
}

/// The settled answer.
#[derive(Clone, Debug, PartialEq)]
pub struct PlasmaSheathRamC2Field {
    pub freestream_temperature_k: f64,
    pub freestream_pressure_pa: f64,
    pub freestream_density_kg_m3: f64,
    pub mach_number: f64,
    pub knudsen_number: f64,
    pub shock_as_thick_as_layer: bool,
    pub shock_temperature_k: f64,
    pub shock_pressure_pa: f64,
    pub shock_density_kg_m3: f64,
    /// Freestream over post-shock density.
    pub shock_density_ratio: f64,
    pub shock_electron_density_m3: f64,
    pub stagnation_temperature_k: f64,
    pub stagnation_pressure_pa: f64,
    pub stagnation_density_kg_m3: f64,
    pub stagnation_electron_density_m3: f64,
    pub stagnation_plasma_frequency_hz: f64,
    pub stagnation_velocity_gradient_per_s: f64,
    pub signal_frequency_hz: f64,
    pub signal_critical_density_m3: f64,
    pub signal_blacked_out_at_stagnation: bool,
    pub stations: [Station; 2],
    /// Ledgers, each the largest found.
    pub shock_mass_residual: f64,
    pub shock_momentum_residual: f64,
    pub shock_energy_residual: f64,
    pub stagnation_enthalpy_residual: f64,
    pub stagnation_entropy_residual: f64,
    pub streamline_nuclei_residual: f64,
    pub streamline_charge_residual: f64,
    pub streamline_energy_residual: f64,
    pub streamline_entropy_fall: f64,
    /// The tolerance whose answer was accepted, and the one before it agreed.
    pub step_tolerance: f64,
    pub steps: u32,
    pub equilibrium_iterations: u32,
    /// Each accepted step of the accepted streamline: surface distance m,
    /// temperature K, electron density m^-3.
    pub streamline: Vec<f64>,
}

/// Why no answer is returned.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PlasmaSheathRamC2Reject {
    /// An input or an intermediate value was NaN or infinite.
    NonFiniteValue,
    /// The altitude is below sea level.
    AltitudeBelowSeaLevel,
    /// The altitude is above 80 637 m, where the freestream Knudsen number, mean free path over nose radius, reaches 0.03: the shock is thicker than the shock layer it bounds.
    RarefiedFlow,
    /// The flight Mach number is 1 or less: there is no bow shock.
    NoBowShock,
    /// The flight speed is above 20 000 m/s: behind the shock the gas is past 20 000 K, the top of the NASA Glenn fits, at every altitude from sea level to 80 637 m.
    VelocityPastFits,
    /// The signal frequency is below 1 MHz, whose critical density, 1.24e10 per cubic metre, is within two orders of the 1e8 per cubic metre to which the streamline resolves electrons.
    SignalFrequencyBelowResolution,
    /// The signal frequency is above 1e150 Hz, whose critical density, 1.24e298 per cubic metre, is near the largest number 64-bit arithmetic carries.
    SignalFrequencyBeyondArithmetic,
    /// A state of the gas is above 20 000 K, the top of the NASA Glenn fits.
    TemperatureAboveFits,
    /// A state of the gas is below 298.15 K, the floor of the ions' fits.
    TemperatureBelowFits,
    /// The element potentials of an equilibrium did not settle.
    EquilibriumNotFound,
    /// No density ratio closes the Rankine-Hugoniot equations.
    ShockNotFound,
    /// No equilibrium state has the stagnation enthalpy and entropy.
    StagnationStateNotFound,
    /// The streamline's step fell below 1e-18 s or its step count passed 400 000.
    StreamlineStalled,
    /// The smallest step tolerance, 1e-9, still did not agree with the one before it.
    NotSettled,
    /// The mass, momentum or energy flux is not the same on both sides of the shock.
    ShockLedgerBroken,
    /// The stagnation state does not hold the shock's total enthalpy and entropy.
    StagnationLedgerBroken,
    /// The nuclei of nitrogen or oxygen per kilogram moved along the streamline.
    NucleiNotConserved,
    /// The reactions made electrons and ions in unequal numbers.
    ChargeNotNeutral,
    /// The total enthalpy moved along the streamline.
    EnergyLedgerBroken,
    /// The entropy of the gas fell along the streamline below the highest it had reached.
    EntropyFell,
}

impl PlasmaSheathRamC2Reject {
    pub fn name(&self) -> &'static str {
        match self {
            Self::NonFiniteValue => "NonFiniteValue",
            Self::AltitudeBelowSeaLevel => "AltitudeBelowSeaLevel",
            Self::RarefiedFlow => "RarefiedFlow",
            Self::NoBowShock => "NoBowShock",
            Self::VelocityPastFits => "VelocityPastFits",
            Self::SignalFrequencyBelowResolution => "SignalFrequencyBelowResolution",
            Self::SignalFrequencyBeyondArithmetic => "SignalFrequencyBeyondArithmetic",
            Self::TemperatureAboveFits => "TemperatureAboveFits",
            Self::TemperatureBelowFits => "TemperatureBelowFits",
            Self::EquilibriumNotFound => "EquilibriumNotFound",
            Self::ShockNotFound => "ShockNotFound",
            Self::StagnationStateNotFound => "StagnationStateNotFound",
            Self::StreamlineStalled => "StreamlineStalled",
            Self::NotSettled => "NotSettled",
            Self::ShockLedgerBroken => "ShockLedgerBroken",
            Self::StagnationLedgerBroken => "StagnationLedgerBroken",
            Self::NucleiNotConserved => "NucleiNotConserved",
            Self::ChargeNotNeutral => "ChargeNotNeutral",
            Self::EnergyLedgerBroken => "EnergyLedgerBroken",
            Self::EntropyFell => "EntropyFell",
        }
    }
}

impl core::fmt::Display for PlasmaSheathRamC2Reject {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let sentence = match self {
            Self::NonFiniteValue => "an input or an intermediate value was not finite",
            Self::AltitudeBelowSeaLevel => "the altitude is below sea level",
            Self::RarefiedFlow => "the altitude is above 80 637 m, where the freestream Knudsen number, the mean free path over the 0.1524 m nose radius, reaches 0.03: the shock, a few mean free paths thick, is thicker than the shock layer it bounds, and the Rankine-Hugoniot jump the model rests on has no room",
            Self::NoBowShock => "the flight Mach number is 1 or less: there is no bow shock",
            Self::VelocityPastFits => "the flight speed is above 20 000 m/s: behind the shock the gas is past 20 000 K, the top of the NASA Glenn fits, at every altitude from sea level to 80 637 m",
            Self::SignalFrequencyBelowResolution => "the signal frequency is below 1 MHz, whose critical density, 1.24e10 per cubic metre, is within two orders of the 1e8 per cubic metre to which the streamline resolves electrons",
            Self::SignalFrequencyBeyondArithmetic => "the signal frequency is above 1e150 Hz, whose critical density, 1.24e298 per cubic metre, is near the largest number 64-bit arithmetic carries",
            Self::TemperatureAboveFits => "a state of the gas is above 20 000 K, the top of the NASA Glenn thermodynamic fits",
            Self::TemperatureBelowFits => "a state of the gas is below 298.15 K, the floor of the ions' NASA Glenn thermodynamic fits",
            Self::EquilibriumNotFound => "the element potentials of an equilibrium did not settle",
            Self::ShockNotFound => "no density ratio closes the Rankine-Hugoniot equations with equilibrium air",
            Self::StagnationStateNotFound => "no equilibrium state holds the stagnation enthalpy and entropy",
            Self::StreamlineStalled => "the streamline's step fell below 1e-18 s or its count passed 400 000",
            Self::NotSettled => "the step tolerance of 1e-9 still did not agree with 1e-8 on the stations",
            Self::ShockLedgerBroken => "the mass, momentum or energy flux differs across the shock by more than 1e-10 of itself",
            Self::StagnationLedgerBroken => "the stagnation state's total enthalpy or entropy differs from the shock's by more than 1e-10",
            Self::NucleiNotConserved => "the nuclei of nitrogen or oxygen per kilogram moved by more than 1e-10 along the streamline",
            Self::ChargeNotNeutral => "the reactions along the streamline made electrons and ions in unequal numbers, by more than 1e-12 of the gross rate of the reactions of charged particles",
            Self::EnergyLedgerBroken => "the total enthalpy moved along the streamline by more than 1e-5 of the flight's kinetic energy per kilogram",
            Self::EntropyFell => "the entropy of the gas fell along the streamline below the highest it had reached, by more than 1e-6 of itself",
        };
        f.write_str(sentence)
    }
}

type Reject = PlasmaSheathRamC2Reject;

fn air(limit: AirLimit) -> Reject {
    match limit {
        AirLimit::TemperatureAboveFits => Reject::TemperatureAboveFits,
        AirLimit::TemperatureBelowFits => Reject::TemperatureBelowFits,
        AirLimit::EquilibriumNotFound => Reject::EquilibriumNotFound,
        AirLimit::NonFiniteValue => Reject::NonFiniteValue,
    }
}

/// The 1962 standard at geometric altitude `z` (0 to 90 km): temperature K,
/// pressure Pa, density kg/m^3, mean free path m.
pub fn atmosphere(z: f64) -> (f64, f64, f64, f64) {
    let h = GEOPOTENTIAL_RADIUS_M * z / (GEOPOTENTIAL_RADIUS_M + z);
    let gmr = STANDARD_GRAVITY_M_S2 * AIR_MOLECULAR_WEIGHT / (UNIVERSAL_GAS_CONSTANT_1962 * 1000.0);
    let mut p = SEA_LEVEL_PRESSURE_PA;
    let mut t = LAYERS[0].1;
    for k in 0..LAYERS.len() {
        let (hb, tb, l) = LAYERS[k];
        let top = if k + 1 < LAYERS.len() { LAYERS[k + 1].0 } else { f64::INFINITY };
        let hh = h.min(top);
        let pn = if l == 0.0 { p * math::exp(-gmr * (hh - hb) / tb) } else { p * math::powf(tb / (tb + l * (hh - hb)), gmr / l) };
        if h <= top {
            t = tb + l * (h - hb);
            p = pn;
            break;
        }
        p = pn;
    }
    let rho = p * AIR_MOLECULAR_WEIGHT / (UNIVERSAL_GAS_CONSTANT_1962 * 1000.0 * t);
    let n = AVOGADRO_1962_PER_KMOL * p / (UNIVERSAL_GAS_CONSTANT_1962 * 1000.0 * t);
    let lambda = 1.0 / (2.0_f64.sqrt() * core::f64::consts::PI * COLLISION_DIAMETER_M * COLLISION_DIAMETER_M * n);
    (t, p, rho, lambda)
}

/// Mass fractions of N2 and O2 of the air entering the shock.
pub fn air_mass_fractions() -> (f64, f64) {
    let m1 = AIR_MOLE_FRACTIONS[0] * air11::MOLAR_MASS_KG_PER_MOL[air11::N2];
    let m2 = AIR_MOLE_FRACTIONS[1] * air11::MOLAR_MASS_KG_PER_MOL[air11::O2];
    (m1 / (m1 + m2), m2 / (m1 + m2))
}

/// Nuclei of nitrogen and oxygen in a kilogram of that air.
pub fn air_nuclei() -> [f64; 2] {
    let (y1, y2) = air_mass_fractions();
    [2.0 * y1 / air11::MOLAR_MASS_KG_PER_MOL[air11::N2], 2.0 * y2 / air11::MOLAR_MASS_KG_PER_MOL[air11::O2]]
}

/// Plasma frequency of `n_e` electrons per cubic metre, Hz.
pub fn plasma_frequency_hz(n_e: f64) -> f64 {
    (n_e.max(0.0) * ELEMENTARY_CHARGE_C * ELEMENTARY_CHARGE_C / (VACUUM_PERMITTIVITY_F_M * ELECTRON_MASS_KG)).sqrt() / (2.0 * core::f64::consts::PI)
}

/// The electron density at which `f` is the plasma frequency, m^-3.
pub fn critical_density_m3(f: f64) -> f64 {
    4.0 * core::f64::consts::PI * core::f64::consts::PI * VACUUM_PERMITTIVITY_F_M * ELECTRON_MASS_KG * f * f / (ELEMENTARY_CHARGE_C * ELEMENTARY_CHARGE_C)
}

/// The freestream.
#[derive(Clone, Copy, Debug)]
pub struct Freestream {
    pub temperature_k: f64,
    pub pressure_pa: f64,
    pub density_kg_m3: f64,
    pub enthalpy_j_kg: f64,
    pub sound_speed_m_s: f64,
    pub mean_free_path_m: f64,
}

/// The freestream at altitude `z`: the standard's temperature and density,
/// the model gas's pressure and enthalpy.
pub fn freestream(z: f64) -> Result<Freestream, Reject> {
    let (t, _, rho, lambda) = atmosphere(z);
    freestream_of(t, rho, lambda)
}

fn freestream_of(t: f64, rho: f64, lambda: f64) -> Result<Freestream, Reject> {
    let (y1, y2) = air_mass_fractions();
    let (h, cp, r, _) = air11::undissociated_air(y1, y2, t, 1.0).map_err(air)?;
    let p = rho * r * t;
    let gamma = cp / (cp - r);
    Ok(Freestream { temperature_k: t, pressure_pa: p, density_kg_m3: rho, enthalpy_j_kg: h, sound_speed_m_s: (gamma * r * t).sqrt(), mean_free_path_m: lambda })
}

/// An equilibrium state.
#[derive(Clone, Copy, Debug)]
pub struct State {
    pub temperature_k: f64,
    pub pressure_pa: f64,
    pub density_kg_m3: f64,
    pub enthalpy_j_kg: f64,
    pub entropy_j_kg_k: f64,
    pub z: [f64; SPECIES],
    pub potentials: [f64; 2],
}

fn state_t_rho(t: f64, rho: f64, start: Option<[f64; 2]>, iters: &mut u32) -> Result<State, Reject> {
    let eq = air11::equilibrium_t_rho(t, rho, air_nuclei(), start).map_err(air)?;
    *iters += eq.iterations;
    let th = air11::thermo(t).map_err(air)?;
    let p = rho * air11::GAS_CONSTANT_J_PER_MOL_K * t * air11::moles(&eq.z);
    Ok(State {
        temperature_k: t,
        pressure_pa: p,
        density_kg_m3: rho,
        enthalpy_j_kg: air11::enthalpy(&eq.z, &th),
        entropy_j_kg_k: air11::entropy(&eq.z, &th, p),
        z: eq.z,
        potentials: eq.potentials,
    })
}

/// The temperature at density `rho` whose equilibrium enthalpy is `h`.
fn state_h_rho(h: f64, rho: f64, guess: f64, iters: &mut u32) -> Result<State, Reject> {
    let mut lo = air11::ALL_SPECIES_FLOOR_K;
    let mut hi = air11::FIT_CEILING_K;
    let top = state_t_rho(hi, rho, None, iters)?;
    if top.enthalpy_j_kg < h {
        return Err(Reject::TemperatureAboveFits);
    }
    let bottom = state_t_rho(lo, rho, None, iters)?;
    if bottom.enthalpy_j_kg > h {
        return Err(Reject::TemperatureBelowFits);
    }
    let mut t = guess.clamp(lo, hi);
    let mut start = None;
    for _ in 0..200 {
        let s = state_t_rho(t, rho, start, iters)?;
        start = Some(s.potentials);
        let f = s.enthalpy_j_kg - h;
        if f > 0.0 {
            hi = t;
        } else {
            lo = t;
        }
        if (f / h.abs().max(1.0)).abs() <= 1.0e-12 || hi - lo <= 1.0e-12 * t {
            return Ok(s);
        }
        let dt = 1.0e-6 * t;
        let s2 = state_t_rho(t + dt, rho, start, iters)?;
        let slope = (s2.enthalpy_j_kg - s.enthalpy_j_kg) / dt;
        let mut next = t - f / slope;
        if !(next > lo && next < hi && next.is_finite()) {
            next = 0.5 * (lo + hi);
        }
        t = next;
    }
    Err(Reject::EquilibriumNotFound)
}

/// The equilibrium state at pressure `p` with entropy `s0`.
fn state_p_s(p: f64, s0: f64, guess: f64, iters: &mut u32) -> Result<State, Reject> {
    let at = |t: f64, iters: &mut u32| -> Result<State, Reject> {
        let (eq, rho) = air11::equilibrium_t_p(t, p, air_nuclei(), None).map_err(air)?;
        *iters += eq.iterations;
        let th = air11::thermo(t).map_err(air)?;
        Ok(State {
            temperature_k: t,
            pressure_pa: p,
            density_kg_m3: rho,
            enthalpy_j_kg: air11::enthalpy(&eq.z, &th),
            entropy_j_kg_k: air11::entropy(&eq.z, &th, p),
            z: eq.z,
            potentials: eq.potentials,
        })
    };
    let mut lo = air11::ALL_SPECIES_FLOOR_K;
    let mut hi = air11::FIT_CEILING_K;
    if at(lo, iters)?.entropy_j_kg_k > s0 {
        return Err(Reject::TemperatureBelowFits);
    }
    if at(hi, iters)?.entropy_j_kg_k < s0 {
        return Err(Reject::TemperatureAboveFits);
    }
    let mut t = guess.clamp(lo, hi);
    for _ in 0..200 {
        let s = at(t, iters)?;
        let f = s.entropy_j_kg_k - s0;
        if f > 0.0 {
            hi = t;
        } else {
            lo = t;
        }
        if (f / s0).abs() <= 1.0e-12 || hi - lo <= 1.0e-12 * t {
            return Ok(s);
        }
        let dt = 1.0e-6 * t;
        let slope = (at(t + dt, iters)?.entropy_j_kg_k - s.entropy_j_kg_k) / dt;
        let mut next = t - f / slope;
        if !(next > lo && next < hi && next.is_finite()) {
            next = 0.5 * (lo + hi);
        }
        t = next;
    }
    Err(Reject::EquilibriumNotFound)
}

/// The frozen expansion: composition `z`, entropy `s0`, pressure `p`.
fn frozen_p_s(z: &[f64; SPECIES], p: f64, s0: f64, guess: f64) -> Result<(f64, f64), Reject> {
    let s_at = |t: f64| -> Result<f64, Reject> {
        let th = air11::thermo(t).map_err(air)?;
        Ok(air11::entropy(z, &th, p))
    };
    let mut lo = air11::ALL_SPECIES_FLOOR_K;
    let mut hi = air11::FIT_CEILING_K;
    if s_at(lo)? > s0 {
        return Err(Reject::TemperatureBelowFits);
    }
    if s_at(hi)? < s0 {
        return Err(Reject::TemperatureAboveFits);
    }
    let mut t = guess.clamp(lo, hi);
    for _ in 0..200 {
        let f = s_at(t)? - s0;
        if f > 0.0 {
            hi = t;
        } else {
            lo = t;
        }
        if (f / s0).abs() <= 1.0e-13 || hi - lo <= 1.0e-12 * t {
            let rho = p / (air11::GAS_CONSTANT_J_PER_MOL_K * t * air11::moles(z));
            return Ok((t, rho));
        }
        let dt = 1.0e-6 * t;
        let slope = (s_at(t + dt)? - s_at(t)?) / dt;
        let mut next = t - f / slope;
        if !(next > lo && next < hi && next.is_finite()) {
            next = 0.5 * (lo + hi);
        }
        t = next;
    }
    Err(Reject::EquilibriumNotFound)
}

/// The equilibrium normal shock: the post-shock state, its density ratio,
/// and the three flux residuals.
pub fn equilibrium_shock(fs: &Freestream, v: f64, iters: &mut u32) -> Result<(State, f64, [f64; 3]), Reject> {
    let rho1 = fs.density_kg_m3;
    let p1 = fs.pressure_pa;
    let h1 = fs.enthalpy_j_kg;
    let mut guess = 3000.0;
    let eval = |eps: f64, guess: &mut f64, iters: &mut u32| -> Result<(f64, State), Reject> {
        let rho2 = rho1 / eps;
        let p2 = p1 + rho1 * v * v * (1.0 - eps);
        let h2 = h1 + 0.5 * v * v * (1.0 - eps * eps);
        let s = state_h_rho(h2, rho2, *guess, iters)?;
        *guess = s.temperature_k;
        Ok((s.pressure_pa / p2 - 1.0, s))
    };
    // The ideal-gas ratio at gamma 1.41, above any air here, bounds the root
    // from above, and a ratio of 1e-3 from below. Where the state behind that
    // far trial is past the fits' 20 000 K, ratios are tried downward from
    // the top by a factor of 0.8 until the pressure balance changes sign:
    // the temperature behind a trial shock rises as its ratio falls, so a
    // trial past the fits lies on the dense side of any root inside them,
    // and the bracket is closed by halving between it and the last trial.
    // Only a root itself past 20 000 K is refused.
    let m2 = math::powi(v / fs.sound_speed_m_s, 2);
    let gamma_hi = 1.41;
    let mut e_hi = ((gamma_hi - 1.0) * m2 + 2.0) / ((gamma_hi + 1.0) * m2);
    let (mut f_hi, mut s_hi) = eval(e_hi, &mut guess, iters)?;
    if !(f_hi < 0.0) {
        return Err(Reject::ShockNotFound);
    }
    // The far end first: a compression ratio of 1e-3.
    let mut e_lo = 1.0e-3;
    let mut f_lo;
    let mut past: Option<f64> = None;
    match eval(e_lo, &mut guess, iters) {
        Ok((f, _)) => f_lo = f,
        Err(Reject::TemperatureAboveFits) => {
            e_lo = e_hi;
            f_lo = f_hi;
            for _ in 0..200 {
                let trial = match past {
                    None => e_lo * 0.8,
                    Some(e_past) => 0.5 * (e_past + e_hi),
                };
                if trial < 1.0e-3 || (past.is_some() && (e_hi - trial).abs() <= 1.0e-13 * e_hi) {
                    return Err(if past.is_some() { Reject::TemperatureAboveFits } else { Reject::ShockNotFound });
                }
                match eval(trial, &mut guess, iters) {
                    Ok((f, _)) if f > 0.0 => {
                        e_lo = trial;
                        f_lo = f;
                        break;
                    }
                    Ok((f, s)) => {
                        e_hi = trial;
                        f_hi = f;
                        s_hi = s;
                        if past.is_none() {
                            e_lo = trial;
                        }
                    }
                    Err(Reject::TemperatureAboveFits) => past = Some(trial),
                    Err(e) => return Err(e),
                }
            }
        }
        Err(e) => return Err(e),
    }
    if !(f_lo > 0.0 && f_hi < 0.0) {
        return Err(Reject::ShockNotFound);
    }
    // Illinois false position.
    let mut side = 0i32;
    let mut best = s_hi;
    let mut eps = e_hi;
    for _ in 0..200 {
        eps = (e_lo * f_hi - e_hi * f_lo) / (f_hi - f_lo);
        let (f, s) = eval(eps, &mut guess, iters)?;
        best = s;
        if f.abs() <= 1.0e-12 || (e_hi - e_lo).abs() <= 1.0e-15 {
            break;
        }
        if f > 0.0 {
            e_lo = eps;
            f_lo = f;
            if side == 1 {
                f_hi *= 0.5;
            }
            side = 1;
        } else {
            e_hi = eps;
            f_hi = f;
            s_hi = s;
            if side == -1 {
                f_lo *= 0.5;
            }
            side = -1;
        }
    }
    let _ = s_hi;
    let s2 = best;
    let u2 = v * eps;
    let mass = (rho1 * v - s2.density_kg_m3 * u2).abs() / (rho1 * v);
    let momentum = ((p1 + rho1 * v * v) - (s2.pressure_pa + s2.density_kg_m3 * u2 * u2)).abs() / (p1 + rho1 * v * v);
    let energy = ((h1 + 0.5 * v * v) - (s2.enthalpy_j_kg + 0.5 * u2 * u2)).abs() / (h1 + 0.5 * v * v).abs();
    Ok((s2, eps, [mass, momentum, energy]))
}

/// The equilibrium stagnation state with total enthalpy `h0` and entropy `s0`.
pub fn stagnation(h0: f64, s0: f64, from: &State, iters: &mut u32) -> Result<(State, [f64; 2]), Reject> {
    let mut lt = math::log(from.temperature_k);
    let mut lr = math::log(from.density_kg_m3);
    let mut start = Some(from.potentials);
    for _ in 0..100 {
        let f = |lt: f64, lr: f64, start: Option<[f64; 2]>, iters: &mut u32| -> Result<([f64; 2], State), Reject> {
            let s = state_t_rho(math::exp(lt), math::exp(lr), start, iters)?;
            Ok(([s.enthalpy_j_kg / h0 - 1.0, s.entropy_j_kg_k / s0 - 1.0], s))
        };
        let (r, s) = f(lt, lr, start, iters)?;
        start = Some(s.potentials);
        if r[0].abs() <= 1.0e-12 && r[1].abs() <= 1.0e-12 {
            return Ok((s, [r[0].abs(), r[1].abs()]));
        }
        let d = 1.0e-7;
        let (rt, _) = f(lt + d, lr, start, iters)?;
        let (rr, _) = f(lt, lr + d, start, iters)?;
        let j = [[(rt[0] - r[0]) / d, (rr[0] - r[0]) / d], [(rt[1] - r[1]) / d, (rr[1] - r[1]) / d]];
        let det = j[0][0] * j[1][1] - j[0][1] * j[1][0];
        if !(det.is_finite() && det != 0.0) {
            return Err(Reject::StagnationStateNotFound);
        }
        let mut dt = (-r[0] * j[1][1] + r[1] * j[0][1]) / det;
        let mut dr = (-r[1] * j[0][0] + r[0] * j[1][0]) / det;
        let big = dt.abs().max(dr.abs());
        if big > 0.5 {
            dt *= 0.5 / big;
            dr *= 0.5 / big;
        }
        lt += dt;
        lr += dr;
        if dt.abs() <= 1.0e-13 && dr.abs() <= 1.0e-13 {
            let (r, s) = f(lt, lr, start, iters)?;
            return Ok((s, [r[0].abs(), r[1].abs()]));
        }
    }
    Err(Reject::StagnationStateNotFound)
}

/// The body: Newtonian pressure and its gradient at surface distance `s`.
#[derive(Clone, Copy, Debug)]
pub struct Body {
    pub p0: f64,
    pub p_inf: f64,
}

/// The angle of the tangent point from the axis, rad.
pub fn tangent_angle() -> f64 {
    core::f64::consts::FRAC_PI_2 - CONE_HALF_ANGLE_DEG.to_radians()
}

/// Surface distance from the stagnation point of a point `x` along the axis.
pub fn surface_distance(x: f64) -> f64 {
    let phi_t = tangent_angle();
    let x_t = NOSE_RADIUS_M * (1.0 - math::cos(phi_t));
    if x <= x_t {
        NOSE_RADIUS_M * math::acos(1.0 - x / NOSE_RADIUS_M)
    } else {
        NOSE_RADIUS_M * phi_t + (x - x_t) / math::cos(CONE_HALF_ANGLE_DEG.to_radians())
    }
}

impl Body {
    pub fn pressure(&self, s: f64) -> (f64, f64) {
        let phi = s / NOSE_RADIUS_M;
        if phi <= tangent_angle() {
            let c = math::cos(phi);
            let sn = math::sin(phi);
            (self.p_inf + (self.p0 - self.p_inf) * c * c, -(self.p0 - self.p_inf) * 2.0 * c * sn / NOSE_RADIUS_M)
        } else {
            let sc = math::sin(CONE_HALF_ANGLE_DEG.to_radians());
            (self.p_inf + (self.p0 - self.p_inf) * sc * sc, 0.0)
        }
    }
}

fn rhs_with(y: &[f64; N_STATE], body: &Body, inverted_balance: bool) -> Result<[f64; N_STATE], AirLimit> {
    let t = y[T_IX];
    let th = air11::thermo(t)?;
    let s = math::exp(y[S_IX]);
    let u = math::exp(y[U_IX]);
    let (p, dpds) = body.pressure(s);
    let z = species_of(y);
    let r = air11::GAS_CONSTANT_J_PER_MOL_K;
    let rho = p / (r * air11::moles(&z) * t);
    let mut c = [0.0; SPECIES];
    for i in 0..SPECIES {
        c[i] = rho * z[i];
    }
    let mut w = air11::production(&th, &c, None);
    if inverted_balance {
        // Reaction 7, N + O = NO+ + e, its backward rate from the inverse of
        // its equilibrium constant: a wrong balance, its heat still booked.
        let r7 = air11::reactions().iter().find(|r| r.row == 7).expect("row 7");
        let (lf, lb) = air11::ln_rates(r7, &th);
        let kc = air11::ln_kc(r7, &th);
        let pair = c[air11::NO_ION] * c[ELECTRON];
        let shift = math::exp(lb) * pair - math::exp(lf + kc) * pair;
        w[air11::NO_ION] += shift;
        w[ELECTRON] += shift;
        w[air11::N] -= shift;
        w[air11::O] -= shift;
    }
    let mut out = [0.0; N_STATE];
    let mut heat = 0.0;
    let mut electrons = 0.0;
    for i in 0..HEAVY {
        out[i] = w[i] / rho;
        heat += r * t * th.h_rt[i] * out[i];
        if i >= air11::N2_ION {
            electrons += out[i];
        }
    }
    heat += r * t * th.h_rt[ELECTRON] * electrons;
    let cp = air11::frozen_cp(&z, &th);
    out[U_IX] = -dpds / (rho * u);
    out[T_IX] = (dpds * u / rho - heat) / cp;
    out[S_IX] = u / s;
    if out.iter().all(|v| v.is_finite()) { Ok(out) } else { Err(AirLimit::NonFiniteValue) }
}

/// Solve `a x = b` in place by Gaussian elimination with partial pivoting.
fn lu_solve(a: &mut [[f64; N_STATE]; N_STATE], piv: &mut [usize; N_STATE]) -> bool {
    for k in 0..N_STATE {
        let mut best = k;
        for i in k + 1..N_STATE {
            if a[i][k].abs() > a[best][k].abs() {
                best = i;
            }
        }
        piv[k] = best;
        if a[best][k] == 0.0 || !a[best][k].is_finite() {
            return false;
        }
        a.swap(k, best);
        for i in k + 1..N_STATE {
            let f = a[i][k] / a[k][k];
            a[i][k] = f;
            for j in k + 1..N_STATE {
                a[i][j] -= f * a[k][j];
            }
        }
    }
    true
}

fn lu_apply(a: &[[f64; N_STATE]; N_STATE], piv: &[usize; N_STATE], b: &mut [f64; N_STATE]) {
    // The rows were swapped whole, multipliers and all: every interchange
    // goes to b first, then the two triangular solves.
    for k in 0..N_STATE {
        b.swap(k, piv[k]);
    }
    for k in 0..N_STATE {
        for i in k + 1..N_STATE {
            b[i] -= a[i][k] * b[k];
        }
    }
    for k in (0..N_STATE).rev() {
        for j in k + 1..N_STATE {
            b[k] -= a[k][j] * b[j];
        }
        b[k] /= a[k][k];
    }
}

/// The streamline as integrated at one tolerance.
#[derive(Clone, Debug)]
struct Line {
    stations: [[f64; N_STATE + 1]; 2],
    steps: u32,
    nuclei: f64,
    charge: f64,
    energy: f64,
    entropy_fall: f64,
    record: Vec<f64>,
}

fn total_enthalpy(y: &[f64; N_STATE]) -> Result<f64, AirLimit> {
    let th = air11::thermo(y[T_IX])?;
    let z = species_of(y);
    let u = math::exp(y[U_IX]);
    Ok(air11::enthalpy(&z, &th) + 0.5 * u * u)
}

fn entropy_of(y: &[f64; N_STATE], body: &Body) -> Result<f64, AirLimit> {
    let th = air11::thermo(y[T_IX])?;
    let z = species_of(y);
    Ok(air11::entropy(&z, &th, body.pressure(math::exp(y[S_IX])).0))
}

fn electron_density(y: &[f64; N_STATE], body: &Body) -> f64 {
    let z = species_of(y);
    let rho = body.pressure(math::exp(y[S_IX])).0 / (air11::GAS_CONSTANT_J_PER_MOL_K * air11::moles(&z) * y[T_IX]);
    z[ELECTRON] * rho * air11::AVOGADRO_PER_MOL
}

/// The reactions' electrons made against the ions they make, over the
/// gross rate of every reaction that touches a charged particle, each
/// counted once for each charged particle it touches: zero but for round-off.
fn charge_closure(y: &[f64; N_STATE], body: &Body, fault: Fault) -> Result<f64, AirLimit> {
    let th = air11::thermo(y[T_IX])?;
    let z = species_of(y);
    let rho = body.pressure(math::exp(y[S_IX])).0 / (air11::GAS_CONSTANT_J_PER_MOL_K * air11::moles(&z) * y[T_IX]);
    let mut c = [0.0; SPECIES];
    for i in 0..SPECIES {
        c[i] = rho * z[i];
    }
    let mut each = Vec::new();
    let mut w = air11::production(&th, &c, Some(&mut each));
    let mut gross = 0.0;
    for (r, (rf, rb)) in air11::reactions().iter().zip(each.iter()) {
        let mut touched = 0.0;
        for i in air11::N2_ION..SPECIES {
            touched += f64::from(r.products[i]) + f64::from(r.reactants[i]);
        }
        gross += touched * (rf + rb);
    }
    if fault == Fault::Charge {
        w[air11::NO_ION] += 1.0e-6 * gross;
    }
    let ions = w[air11::N2_ION] + w[air11::O2_ION] + w[air11::NO_ION] + w[air11::N_ION] + w[air11::O_ION];
    Ok(if gross > 0.0 { (w[ELECTRON] - ions).abs() / gross } else { 0.0 })
}

/// A fault put into the streamline on purpose, so a test can show that the
/// ledger meant to catch it does.
#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(not(test), allow(dead_code))]
enum Fault {
    None,
    /// Electrons made from nothing.
    Charge,
    /// Nitrogen atoms made from nothing.
    Nuclei,
    /// Heat from nothing: the gas warmed at 1e4 K/s.
    Energy,
    /// Reaction 7, N + O = NO+ + e, given the inverse of its equilibrium
    /// constant: the kinetics relax toward a state of lower entropy, the
    /// energy still booked.
    Entropy,
}

fn faulted(y: &[f64; N_STATE], body: &Body, fault: Fault) -> Result<[f64; N_STATE], AirLimit> {
    let mut f = rhs_with(y, body, fault == Fault::Entropy)?;
    match fault {
        Fault::None => {}
        Fault::Charge => {}
        Fault::Nuclei => f[air11::N] += 1.0e-3 * y[air11::N].abs().max(1.0e-6),
        Fault::Energy => f[T_IX] += 1.0e4,
        Fault::Entropy => {}
    }
    Ok(f)
}

fn integrate(start: &[f64; N_STATE], body: &Body, energy_scale: f64, density: f64, rtol: f64, fault: Fault) -> Result<Line, Reject> {
    let gamma = 1.0 + core::f64::consts::FRAC_1_SQRT_2;
    // Absolute tolerances, mol/kg: the neutrals' 1e-6 of the step
    // tolerance; the ions' that of ELECTRON_RESOLUTION_M3 at the stagnation
    // density, scaled the same way.
    let ion_atol = ELECTRON_RESOLUTION_M3 * (rtol / 1.0e-6) / (density * air11::AVOGADRO_PER_MOL);
    let atol = |i: usize| if i >= air11::N2_ION { ion_atol } else { NEUTRAL_ABSOLUTE_TOLERANCE * rtol };
    let targets = [math::log(surface_distance(STATION_X_M[0])), math::log(NOSE_RADIUS_M * tangent_angle()), math::log(surface_distance(STATION_X_M[1]))];
    let station_of_target = [Some(0usize), None, Some(1usize)];
    let mut y = *start;
    let mut t_now = 0.0;
    let nuclei0 = air11::nuclei_and_charge(&species_of(&y));
    let h_line = total_enthalpy(&y).map_err(air)?;
    let mut s_top = entropy_of(&y, body).map_err(air)?;
    let mut h = 1.0e-9;
    let mut next = 0usize;
    let mut out = Line { stations: [[0.0; N_STATE + 1]; 2], steps: 0, nuclei: 0.0, charge: 0.0, energy: 0.0, entropy_fall: 0.0, record: Vec::new() };
    let mut f0 = faulted(&y, body, fault).map_err(air)?;
    out.record.extend_from_slice(&[math::exp(y[S_IX]), y[T_IX], electron_density(&y, body)]);
    while next < targets.len() {
        if out.steps >= MAX_STEPS {
            return Err(Reject::StreamlineStalled);
        }
        // The Jacobian by forward differences.
        let mut jac = [[0.0; N_STATE]; N_STATE];
        for j in 0..N_STATE {
            let d = if j < HEAVY { (1.0e-7 * y[j].abs()).max(1.0e-12) } else if j == T_IX { 1.0e-7 * y[j] } else { 1.0e-7 };
            let mut yp = y;
            yp[j] += d;
            let fp = faulted(&yp, body, fault).map_err(air)?;
            for i in 0..N_STATE {
                jac[i][j] = (fp[i] - f0[i]) / d;
            }
        }
        // Steps with this Jacobian, shortened until one is accepted. A trial
        // that leaves the fits is shortened; if the step shrinks to nothing
        // against that wall, the wall is the answer.
        let mut wall: Option<AirLimit> = None;
        let (yn, fnew, err) = loop {
            if h < MIN_STEP_S {
                return Err(wall.map(air).unwrap_or(Reject::StreamlineStalled));
            }
            let mut w = [[0.0; N_STATE]; N_STATE];
            for i in 0..N_STATE {
                for j in 0..N_STATE {
                    w[i][j] = -gamma * h * jac[i][j];
                }
                w[i][i] += 1.0;
            }
            let mut piv = [0usize; N_STATE];
            if !lu_solve(&mut w, &mut piv) {
                h *= 0.5;
                continue;
            }
            let mut k1 = f0;
            lu_apply(&w, &piv, &mut k1);
            let mut y1 = y;
            for i in 0..N_STATE {
                y1[i] += h * k1[i];
            }
            let f1 = match faulted(&y1, body, fault) {
                Ok(f) => f,
                Err(e) => {
                    wall = Some(e);
                    h *= 0.5;
                    continue;
                }
            };
            let mut k2 = [0.0; N_STATE];
            for i in 0..N_STATE {
                k2[i] = f1[i] - 2.0 * k1[i];
            }
            lu_apply(&w, &piv, &mut k2);
            let mut yn = y;
            // The local error: the difference from the embedded first-order
            // solution.
            let mut est = [0.0; N_STATE];
            for i in 0..N_STATE {
                yn[i] += h * (1.5 * k1[i] + 0.5 * k2[i]);
                est[i] = 0.5 * h * (k1[i] + k2[i]);
            }
            let mut err = 0.0_f64;
            for i in 0..N_STATE {
                let scale = if i < HEAVY { atol(i) + rtol * y[i].abs().max(yn[i].abs()) } else if i == T_IX { rtol * y[i].max(yn[i]) } else { rtol };
                err = err.max(est[i].abs() / scale);
            }
            let negative = (0..HEAVY).any(|i| yn[i] < -100.0 * atol(i));
            // No step moves the parcel's surface distance or speed by more
            // than a tenth of itself: near the stagnation point both grow
            // as exp(beta t), and a longer step outruns its own error estimate.
            let stride = (yn[S_IX] - y[S_IX]).abs().max((yn[U_IX] - y[U_IX]).abs());
            if stride > MAX_LOG_STRIDE {
                h *= (0.9 * MAX_LOG_STRIDE / stride).clamp(0.1, 0.9);
                continue;
            }
            if !err.is_finite() || negative || err > 1.0 {
                h *= if err.is_finite() { (0.9 / err.sqrt()).clamp(0.1, 0.5) } else { 0.1 };
                continue;
            }
            // Land on the next target, not past it.
            if yn[S_IX] > targets[next] + 1.0e-12 {
                h *= ((targets[next] - y[S_IX]) / (yn[S_IX] - y[S_IX])).clamp(0.05, 0.999_999);
                continue;
            }
            let fnew = match faulted(&yn, body, fault) {
                Ok(f) => f,
                Err(e) => {
                    wall = Some(e);
                    h *= 0.5;
                    continue;
                }
            };
            break (yn, fnew, err);
        };
        // The ledgers of the accepted step.
        let nc = air11::nuclei_and_charge(&species_of(&yn));
        let nuclei = ((nc[0] - nuclei0[0]) / nuclei0[0]).abs().max(((nc[1] - nuclei0[1]) / nuclei0[1]).abs());
        let charge = charge_closure(&yn, body, fault).map_err(air)?;
        let energy = ((total_enthalpy(&yn).map_err(air)? - h_line) / energy_scale).abs();
        let s_new = entropy_of(&yn, body).map_err(air)?;
        s_top = s_top.max(s_new);
        let fall = ((s_top - s_new) / s_top.abs()).max(0.0);
        out.nuclei = out.nuclei.max(nuclei);
        out.charge = out.charge.max(charge);
        out.energy = out.energy.max(energy);
        out.entropy_fall = out.entropy_fall.max(fall);
        if !(nuclei <= NUCLEI_TOLERANCE) {
            return Err(Reject::NucleiNotConserved);
        }
        if !(charge <= CHARGE_TOLERANCE) {
            return Err(Reject::ChargeNotNeutral);
        }
        if !(energy <= ENERGY_LEDGER_TOLERANCE) {
            return Err(Reject::EnergyLedgerBroken);
        }
        if !(fall <= ENTROPY_TOLERANCE) {
            return Err(Reject::EntropyFell);
        }
        let reached = yn[S_IX] >= targets[next] - 1.0e-12;
        t_now += h;
        y = yn;
        f0 = fnew;
        out.steps += 1;
        out.record.extend_from_slice(&[math::exp(y[S_IX]), y[T_IX], electron_density(&y, body)]);
        h *= (0.9 / err.max(1.0e-10).sqrt()).clamp(0.2, 4.0);
        if reached {
            if let Some(k) = station_of_target[next] {
                out.stations[k][..N_STATE].copy_from_slice(&y);
                out.stations[k][N_STATE] = t_now;
            }
            next += 1;
        }
    }
    Ok(out)
}

/// The limits the question shows by itself, before anything is computed:
/// its numbers finite, the altitude from sea level to the Knudsen limit, a
/// bow shock, the signal inside what the arithmetic resolves.
pub fn admit(question: &PlasmaSheathRamC2Question) -> Result<(), PlasmaSheathRamC2Reject> {
    let PlasmaSheathRamC2Question { altitude_m: z, velocity_m_s: v, signal_frequency_hz: f_sig } = *question;
    if !(z.is_finite() && v.is_finite() && f_sig.is_finite()) {
        return Err(Reject::NonFiniteValue);
    }
    if z < 0.0 {
        return Err(Reject::AltitudeBelowSeaLevel);
    }
    if z > KNUDSEN_ALTITUDE_M {
        return Err(Reject::RarefiedFlow);
    }
    let fs = freestream(z)?;
    if !(v / fs.sound_speed_m_s > 1.0) {
        return Err(Reject::NoBowShock);
    }
    if v > VELOCITY_CEILING_M_S {
        return Err(Reject::VelocityPastFits);
    }
    if !(f_sig >= SIGNAL_FREQUENCY_FLOOR_HZ) {
        return Err(Reject::SignalFrequencyBelowResolution);
    }
    if f_sig > SIGNAL_FREQUENCY_CEILING_HZ {
        return Err(Reject::SignalFrequencyBeyondArithmetic);
    }
    Ok(())
}

/// The settled answer, or the limit that stopped it.
///
/// Each value of the answer below the normal range is written as zero
/// with its sign ([`crate::normal`]).
pub fn solve(question: &PlasmaSheathRamC2Question) -> Result<PlasmaSheathRamC2Field, PlasmaSheathRamC2Reject> {
    crate::normal::held(solve_as_computed(question))
}

/// The answer as the equations compute it, before a value below the
/// normal range is written as zero.
fn solve_as_computed(question: &PlasmaSheathRamC2Question) -> Result<PlasmaSheathRamC2Field, PlasmaSheathRamC2Reject> {
    solve_with(question, &STEP_TOLERANCES, START_FRACTION, Fault::None)
}

impl NormalOrZero for PlasmaSheathRamC2Field {
    fn normal_or_zero(&mut self) {
        self.freestream_temperature_k.normal_or_zero();
        self.freestream_pressure_pa.normal_or_zero();
        self.freestream_density_kg_m3.normal_or_zero();
        self.mach_number.normal_or_zero();
        self.knudsen_number.normal_or_zero();
        self.shock_temperature_k.normal_or_zero();
        self.shock_pressure_pa.normal_or_zero();
        self.shock_density_kg_m3.normal_or_zero();
        self.shock_density_ratio.normal_or_zero();
        self.shock_electron_density_m3.normal_or_zero();
        self.stagnation_temperature_k.normal_or_zero();
        self.stagnation_pressure_pa.normal_or_zero();
        self.stagnation_density_kg_m3.normal_or_zero();
        self.stagnation_electron_density_m3.normal_or_zero();
        self.stagnation_plasma_frequency_hz.normal_or_zero();
        self.stagnation_velocity_gradient_per_s.normal_or_zero();
        self.signal_frequency_hz.normal_or_zero();
        self.signal_critical_density_m3.normal_or_zero();
        self.stations.normal_or_zero();
        self.shock_mass_residual.normal_or_zero();
        self.shock_momentum_residual.normal_or_zero();
        self.shock_energy_residual.normal_or_zero();
        self.stagnation_enthalpy_residual.normal_or_zero();
        self.stagnation_entropy_residual.normal_or_zero();
        self.streamline_nuclei_residual.normal_or_zero();
        self.streamline_charge_residual.normal_or_zero();
        self.streamline_energy_residual.normal_or_zero();
        self.streamline_entropy_fall.normal_or_zero();
        self.step_tolerance.normal_or_zero();
        self.streamline.normal_or_zero();
    }
}

impl NormalOrZero for Station {
    fn normal_or_zero(&mut self) {
        self.x_m.normal_or_zero();
        self.surface_distance_m.normal_or_zero();
        self.pressure_pa.normal_or_zero();
        self.temperature_k.normal_or_zero();
        self.velocity_m_s.normal_or_zero();
        self.electron_density_m3.normal_or_zero();
        self.plasma_frequency_hz.normal_or_zero();
        self.equilibrium_electron_density_m3.normal_or_zero();
        self.equilibrium_temperature_k.normal_or_zero();
        self.frozen_electron_density_m3.normal_or_zero();
        self.frozen_temperature_k.normal_or_zero();
        self.transit_time_s.normal_or_zero();
    }
}

fn solve_with(question: &PlasmaSheathRamC2Question, tolerances: &[f64], start_fraction: f64, fault: Fault) -> Result<PlasmaSheathRamC2Field, Reject> {
    admit(question)?;
    let PlasmaSheathRamC2Question { altitude_m: z, velocity_m_s: v, signal_frequency_hz: f_sig } = *question;
    let fs = freestream(z)?;
    let kn = fs.mean_free_path_m / NOSE_RADIUS_M;
    let mach = v / fs.sound_speed_m_s;
    let mut iters = 0u32;
    let (shock, eps, shock_ledger) = equilibrium_shock(&fs, v, &mut iters)?;
    if shock_ledger.iter().any(|r| !(*r <= SHOCK_LEDGER_TOLERANCE)) {
        return Err(Reject::ShockLedgerBroken);
    }
    let u2 = v * eps;
    let h0 = shock.enthalpy_j_kg + 0.5 * u2 * u2;
    let s0 = shock.entropy_j_kg_k;
    let (stag, stag_ledger) = stagnation(h0, s0, &shock, &mut iters)?;
    if stag_ledger.iter().any(|r| !(*r <= STAGNATION_LEDGER_TOLERANCE)) {
        return Err(Reject::StagnationLedgerBroken);
    }
    let body = Body { p0: stag.pressure_pa, p_inf: fs.pressure_pa };
    let beta = (2.0 * (body.p0 - body.p_inf) / stag.density_kg_m3).sqrt() / NOSE_RADIUS_M;
    let s_start = start_fraction * NOSE_RADIUS_M;
    let mut start = [0.0; N_STATE];
    start[..HEAVY].copy_from_slice(&stag.z[..HEAVY]);
    start[T_IX] = stag.temperature_k;
    start[U_IX] = math::log(beta * s_start);
    start[S_IX] = math::log(s_start);

    // The tolerance ladder. A rung whose ledger breaks was stepped too
    // coarsely and the next rung is tried; a limit of the physics stops the
    // solve at once; the finest rung's broken ledger is the answer.
    let mut previous: Option<Line> = None;
    let mut accepted: Option<(Line, f64)> = None;
    let mut broken: Option<Reject> = None;
    for &rtol in tolerances {
        let line = match integrate(&start, &body, 0.5 * v * v, stag.density_kg_m3, rtol, fault) {
            Ok(line) => line,
            Err(e @ (Reject::NucleiNotConserved | Reject::ChargeNotNeutral | Reject::EnergyLedgerBroken | Reject::EntropyFell)) => {
                broken = Some(e);
                previous = None;
                continue;
            }
            Err(e) => return Err(e),
        };
        broken = None;
        if let Some(prev) = &previous {
            let agree = (0..2).all(|k| {
                let a = electron_density(&station_state(&line, k), &body);
                let b = electron_density(&station_state(prev, k), &body);
                let ta = line.stations[k][T_IX];
                let tb = prev.stations[k][T_IX];
                (a - b).abs() <= SETTLE_TOLERANCE * a.abs() + SETTLE_FLOOR_M3 && ((ta - tb) / ta).abs() <= SETTLE_TOLERANCE
            });
            if agree {
                accepted = Some((line, rtol));
                break;
            }
        }
        previous = Some(line);
    }
    if let Some(e) = broken {
        return Err(e);
    }
    let (line, rtol) = accepted.ok_or(Reject::NotSettled)?;

    let n_c = critical_density_m3(f_sig);
    let n0 = stag.z[ELECTRON] * stag.density_kg_m3 * air11::AVOGADRO_PER_MOL;
    let mut stations = Vec::with_capacity(2);
    for k in 0..2 {
        let y = station_state(&line, k);
        let s = math::exp(y[S_IX]);
        let (p, _) = body.pressure(s);
        let n_e = electron_density(&y, &body);
        let eq = state_p_s(p, s0, y[T_IX], &mut iters)?;
        let (tf, rf) = frozen_p_s(&stag.z, p, s0, y[T_IX])?;
        stations.push(Station {
            x_m: STATION_X_M[k],
            surface_distance_m: s,
            pressure_pa: p,
            temperature_k: y[T_IX],
            velocity_m_s: math::exp(y[U_IX]),
            electron_density_m3: n_e,
            electron_density_resolved: n_e >= ELECTRON_RESOLUTION_M3,
            plasma_frequency_hz: plasma_frequency_hz(n_e),
            equilibrium_electron_density_m3: eq.z[ELECTRON] * eq.density_kg_m3 * air11::AVOGADRO_PER_MOL,
            equilibrium_temperature_k: eq.temperature_k,
            frozen_electron_density_m3: stag.z[ELECTRON] * rf * air11::AVOGADRO_PER_MOL,
            frozen_temperature_k: tf,
            transit_time_s: line.stations[k][N_STATE],
            signal_blacked_out: n_e > n_c,
        });
    }
    let stations: [Station; 2] = [stations[0].clone(), stations[1].clone()];
    let field = PlasmaSheathRamC2Field {
        freestream_temperature_k: fs.temperature_k,
        freestream_pressure_pa: fs.pressure_pa,
        freestream_density_kg_m3: fs.density_kg_m3,
        mach_number: mach,
        knudsen_number: kn,
        shock_as_thick_as_layer: kn > KNUDSEN_SHOCK_AS_THICK_AS_LAYER,
        shock_temperature_k: shock.temperature_k,
        shock_pressure_pa: shock.pressure_pa,
        shock_density_kg_m3: shock.density_kg_m3,
        shock_density_ratio: eps,
        shock_electron_density_m3: shock.z[ELECTRON] * shock.density_kg_m3 * air11::AVOGADRO_PER_MOL,
        stagnation_temperature_k: stag.temperature_k,
        stagnation_pressure_pa: stag.pressure_pa,
        stagnation_density_kg_m3: stag.density_kg_m3,
        stagnation_electron_density_m3: n0,
        stagnation_plasma_frequency_hz: plasma_frequency_hz(n0),
        stagnation_velocity_gradient_per_s: beta,
        signal_frequency_hz: f_sig,
        signal_critical_density_m3: n_c,
        signal_blacked_out_at_stagnation: n0 > n_c,
        stations,
        shock_mass_residual: shock_ledger[0],
        shock_momentum_residual: shock_ledger[1],
        shock_energy_residual: shock_ledger[2],
        stagnation_enthalpy_residual: stag_ledger[0],
        stagnation_entropy_residual: stag_ledger[1],
        streamline_nuclei_residual: line.nuclei,
        streamline_charge_residual: line.charge,
        streamline_energy_residual: line.energy,
        streamline_entropy_fall: line.entropy_fall,
        step_tolerance: rtol,
        steps: line.steps,
        equilibrium_iterations: iters,
        streamline: line.record,
    };
    let finite = [
        field.shock_temperature_k,
        field.stagnation_electron_density_m3,
        field.stations[0].electron_density_m3,
        field.stations[1].electron_density_m3,
        field.stations[0].equilibrium_electron_density_m3,
        field.stations[1].frozen_electron_density_m3,
    ];
    if finite.iter().all(|v| v.is_finite()) && field.streamline.iter().all(|v| v.is_finite()) {
        Ok(field)
    } else {
        Err(Reject::NonFiniteValue)
    }
}

fn station_state(line: &Line, k: usize) -> [f64; N_STATE] {
    let mut y = [0.0; N_STATE];
    y.copy_from_slice(&line.stations[k][..N_STATE]);
    y
}

#[cfg(test)]
mod tests {
    use super::*;

    fn question() -> PlasmaSheathRamC2Question {
        PlasmaSheathRamC2Question { altitude_m: 71_000.0, velocity_m_s: 7650.0, signal_frequency_hz: 9.21e9 }
    }

    fn relative(a: f64, b: f64) -> f64 {
        (a / b - 1.0).abs()
    }

    /// The worked question settles, holds every ledger, and agrees with the
    /// record's independent integration of the same equations (Python,
    /// SciPy's Radau at a relative tolerance of 1e-8:
    /// `evidence/research/hgv_plasma_ramc2/analysis/prototype`) to 1e-4.
    #[test]
    fn the_worked_question_settles_and_agrees_with_an_independent_integration() {
        let a = solve(&question()).unwrap();
        assert!(relative(a.stations[0].electron_density_m3, 8.9162e18) < 1.0e-4, "{}", a.stations[0].electron_density_m3);
        assert!(relative(a.stations[1].electron_density_m3, 6.6311e17) < 1.0e-4, "{}", a.stations[1].electron_density_m3);
        assert!(relative(a.stagnation_electron_density_m3, 2.6869e19) < 1.0e-4, "{}", a.stagnation_electron_density_m3);
        assert!(a.shock_mass_residual.max(a.shock_momentum_residual).max(a.shock_energy_residual) <= SHOCK_LEDGER_TOLERANCE);
        assert!(a.streamline_energy_residual <= ENERGY_LEDGER_TOLERANCE);
        assert!(a.streamline_charge_residual <= CHARGE_TOLERANCE);
        assert!(a.streamline_nuclei_residual <= NUCLEI_TOLERANCE);
        assert!(a.streamline_entropy_fall <= ENTROPY_TOLERANCE);
        // The stagnation point holds the layer's most electrons; they fall
        // along the body.
        assert!(a.stagnation_electron_density_m3 > a.stations[0].electron_density_m3);
        assert!(a.stations[0].electron_density_m3 > a.stations[1].electron_density_m3);
        // The verdict is the comparison it says it is.
        for st in &a.stations {
            assert_eq!(st.signal_blacked_out, st.electron_density_m3 > a.signal_critical_density_m3);
            assert!(relative(st.plasma_frequency_hz, (st.electron_density_m3 / a.signal_critical_density_m3).sqrt() * a.signal_frequency_hz) < 1.0e-12);
        }
        assert!(a.stations[0].signal_blacked_out && !a.stations[1].signal_blacked_out);
        assert!(!a.shock_as_thick_as_layer && relative(a.knudsen_number, 1.0629e-3 / NOSE_RADIUS_M) < 1.0e-3);
    }

    /// Huber, NASA TR R-163 (1963), Table IV(i): equilibrium normal shock and
    /// stagnation point at 230 400 ft, T1 = 205 K, p1 = 5.042e-5 atm, a
    /// published calculation from air tables of the 1950s with argon in.
    /// Rows at T2 = 5000, 6000 and 6500 K: flight speed, p2/p1, rho2/rho1,
    /// stagnation temperature and pressure ratio, and the electron
    /// concentration behind the shock (printed to two figures).
    #[test]
    fn the_equilibrium_shock_meets_huber_1963() {
        let t1 = 205.0;
        let (y1, y2) = air_mass_fractions();
        let (_, _, r, _) = air11::undissociated_air(y1, y2, t1, 1.0).unwrap();
        let p1 = 5.042e-5 * 101_325.0;
        let fs = freestream_of(t1, p1 / (r * t1), 0.0).unwrap();
        let rows = [
            (5000.0, 17.00e3, 420.8, 13.10, 5016.0, 437.0, 3.4e18),
            (6000.0, 23.71e3, 833.6, 17.29, 6013.0, 858.0, 2.1e19),
            (6500.0, 26.68e3, 1059.0, 18.29, 6516.0, 1088.0, 5.0e19),
        ];
        for (t2, u_ft, p_ratio, rho_ratio, ts, ps_ratio, ne) in rows {
            let mut iters = 0;
            let (shock, eps, _) = equilibrium_shock(&fs, u_ft * 0.3048, &mut iters).unwrap();
            let v = u_ft * 0.3048;
            let h0 = shock.enthalpy_j_kg + 0.5 * (v * eps).powi(2);
            let (stag, _) = stagnation(h0, shock.entropy_j_kg_k, &shock, &mut iters).unwrap();
            let n2 = shock.z[ELECTRON] * shock.density_kg_m3 * air11::AVOGADRO_PER_MOL;
            assert!(relative(shock.temperature_k, t2) < 0.004, "{t2}: T2 {}", shock.temperature_k);
            assert!(relative(1.0 / eps, rho_ratio) < 0.006, "{t2}: rho ratio {}", 1.0 / eps);
            assert!(relative(shock.pressure_pa / fs.pressure_pa, p_ratio) < 0.002, "{t2}: p ratio {}", shock.pressure_pa / fs.pressure_pa);
            assert!(relative(stag.temperature_k, ts) < 0.003, "{t2}: Ts {}", stag.temperature_k);
            assert!(relative(stag.pressure_pa / fs.pressure_pa, ps_ratio) < 0.005, "{t2}: ps ratio {}", stag.pressure_pa / fs.pressure_pa);
            assert!(relative(n2, ne) < 0.06, "{t2}: ne {n2}");
        }
    }

    /// The 1962 standard's printed tables (NTRS 19630003300, Table I, by
    /// geometric altitude): temperature, density, mean free path.
    #[test]
    fn the_atmosphere_is_the_1962_standard() {
        for (z, t, rho, lambda) in [(61_000.0, 253.810, 2.7028e-4, 3.006e-4), (71_000.0, 215.788, 7.6442e-5, 1.0629e-3), (81_000.0, 180.65, 1.662e-5, 4.888e-3), (85_000.0, 180.65, 7.955e-6, 1.021e-2)] {
            let (tt, _, rr, ll) = atmosphere(z);
            assert!((tt - t).abs() < 1.0e-3, "{z}: {tt}");
            assert!(relative(rr, rho) < 3.0e-4, "{z}: {rr}");
            assert!(relative(ll, lambda) < 1.0e-3, "{z}: {ll}");
        }
    }

    #[test]
    fn each_ledger_broken_on_purpose_is_named() {
        let q = question();
        for (fault, name) in [
            (Fault::Charge, Reject::ChargeNotNeutral),
            (Fault::Nuclei, Reject::NucleiNotConserved),
            (Fault::Energy, Reject::EnergyLedgerBroken),
            (Fault::Entropy, Reject::EntropyFell),
        ] {
            assert_eq!(solve_with(&q, &STEP_TOLERANCES, START_FRACTION, fault).map(|_| ()), Err(name), "{fault:?}");
        }
    }

    /// Where the streamline starts does not move the answer.
    #[test]
    fn the_start_does_not_move_the_stations() {
        let a = solve_with(&question(), &STEP_TOLERANCES, 1.0e-2, Fault::None).unwrap();
        let b = solve_with(&question(), &STEP_TOLERANCES, 1.0e-4, Fault::None).unwrap();
        for k in 0..2 {
            assert!(relative(a.stations[k].electron_density_m3, b.stations[k].electron_density_m3) < 2.0e-4);
            assert!(relative(a.stations[k].temperature_k, b.stations[k].temperature_k) < 2.0e-4);
        }
    }

    /// A ladder of one tolerance cannot settle, and says so.
    #[test]
    fn a_ladder_cut_short_is_named_not_settled() {
        assert_eq!(solve_with(&question(), &[1.0e-5], START_FRACTION, Fault::None).map(|_| ()), Err(Reject::NotSettled));
    }

    #[test]
    fn each_limit_is_named() {
        let q = question();
        let at = |z: f64, v: f64, f: f64| solve(&PlasmaSheathRamC2Question { altitude_m: z, velocity_m_s: v, signal_frequency_hz: f }).map(|_| ());
        assert_eq!(at(81_000.0, 7650.0, 1.0e9), Err(Reject::RarefiedFlow));
        assert_eq!(at(120_000.0, 7650.0, 1.0e9), Err(Reject::RarefiedFlow));
        assert_eq!(at(-1.0, 7650.0, 1.0e9), Err(Reject::AltitudeBelowSeaLevel));
        assert_eq!(at(71_000.0, 290.0, 1.0e9), Err(Reject::NoBowShock));
        assert_eq!(at(71_000.0, -7650.0, 1.0e9), Err(Reject::NoBowShock));
        assert_eq!(at(71_000.0, 7650.0, 0.999e6), Err(Reject::SignalFrequencyBelowResolution));
        assert_eq!(at(71_000.0, 7650.0, -1.0), Err(Reject::SignalFrequencyBelowResolution));
        assert_eq!(at(71_000.0, 7650.0, 1.0e151), Err(Reject::SignalFrequencyBeyondArithmetic));
        assert_eq!(at(KNUDSEN_ALTITUDE_M + 1.0e-6, 7650.0, 1.0e9), Err(Reject::RarefiedFlow));
        assert!(at(KNUDSEN_ALTITUDE_M, 7650.0, 1.0e9).is_ok());
        assert!(at(71_000.0, 7650.0, SIGNAL_FREQUENCY_CEILING_HZ).is_ok());
        assert_eq!(at(20_000.0, 600.0, 1.0e9), Err(Reject::TemperatureBelowFits));
        assert_eq!(at(60_000.0, 30_000.0, 1.0e9), Err(Reject::VelocityPastFits));
        assert_eq!(at(60_000.0, 19_500.0, 1.0e9), Err(Reject::TemperatureAboveFits));
        assert_eq!(at(f64::NAN, 7650.0, 1.0e9), Err(Reject::NonFiniteValue));
        assert_eq!(at(71_000.0, f64::INFINITY, 1.0e9), Err(Reject::NonFiniteValue));
        assert!(solve(&q).is_ok());
    }

    /// A shock whose far trial ratio, 1e-3, puts the gas past the fits is
    /// still found where its own state is inside them: 14 km/s at 50 km
    /// (about 14 800 K behind the shock); at sea level 14 km/s it is past
    /// 20 000 K and refused by name.
    #[test]
    fn a_far_trial_past_the_fits_does_not_hide_the_shock() {
        let at = |z: f64, v: f64| solve(&PlasmaSheathRamC2Question { altitude_m: z, velocity_m_s: v, signal_frequency_hz: 1.0e9 });
        let a = at(50_000.0, 14_000.0).unwrap();
        assert!(a.shock_temperature_k > 14_000.0 && a.shock_temperature_k < 15_500.0, "{}", a.shock_temperature_k);
        assert!(a.shock_mass_residual.max(a.shock_momentum_residual).max(a.shock_energy_residual) <= SHOCK_LEDGER_TOLERANCE);
        assert_eq!(at(0.0, 14_000.0).map(|_| ()), Err(Reject::TemperatureAboveFits));
    }

    /// Just below the speed ceiling every altitude is already past the fits,
    /// so the ceiling refuses nothing the physics would answer.
    #[test]
    fn below_the_speed_ceiling_the_fits_already_stop_the_shock() {
        for z in [0.0, 40_000.0, 80_000.0, KNUDSEN_ALTITUDE_M] {
            let q = PlasmaSheathRamC2Question { altitude_m: z, velocity_m_s: VELOCITY_CEILING_M_S, signal_frequency_hz: 1.0e9 };
            assert_eq!(solve(&q).map(|_| ()), Err(Reject::TemperatureAboveFits), "{z}");
        }
    }

    /// The Knudsen limits sit where their sentences say: about 80.6 km and 73.6 km.
    #[test]
    fn the_knudsen_limits_are_near_80_6_and_73_6_km() {
        let kn = |z: f64| atmosphere(z).3 / NOSE_RADIUS_M;
        assert!(kn(80_600.0) < KNUDSEN_LIMIT && kn(80_700.0) > KNUDSEN_LIMIT);
        assert!(kn(KNUDSEN_ALTITUDE_M) <= KNUDSEN_LIMIT && kn(KNUDSEN_ALTITUDE_M + 1.0) > KNUDSEN_LIMIT);
        assert!(kn(73_500.0) < KNUDSEN_SHOCK_AS_THICK_AS_LAYER && kn(73_650.0) > KNUDSEN_SHOCK_AS_THICK_AS_LAYER);
    }

    #[test]
    fn the_stations_sit_where_the_source_puts_them() {
        let phi1 = surface_distance(STATION_X_M[0]) / NOSE_RADIUS_M;
        assert!((phi1.to_degrees() - 45.04).abs() < 0.01, "{}", phi1.to_degrees());
        assert!(surface_distance(STATION_X_M[1]) > NOSE_RADIUS_M * tangent_angle());
        assert!((STATION_X_M[0] / REFERENCE_DIAMETER_M - 0.15).abs() < 0.005);
        assert!((STATION_X_M[1] / REFERENCE_DIAMETER_M - 0.76).abs() < 0.005);
        // Modified Newtonian pressure is continuous at the tangent point.
        let body = Body { p0: 1.0e4, p_inf: 10.0 };
        let st = NOSE_RADIUS_M * tangent_angle();
        assert!((body.pressure(st * (1.0 - 1.0e-12)).0 - body.pressure(st * (1.0 + 1.0e-12)).0).abs() < 1.0e-6);
    }

    #[test]
    fn the_critical_density_is_the_sources() {
        // TN D-6062: 0.63 of the critical density at S band is 8.76e10 per cm^3.
        assert!(relative(REFLECTOMETER_CRITERION * critical_density_m3(3.348e9), 8.76e16) < 2.0e-3);
        assert!(relative(plasma_frequency_hz(critical_density_m3(1.0e9)), 1.0e9) < 1.0e-12);
    }

    /// The comparison of 2026-10-15, held: the three gated onsets of TN
    /// D-6062 Table X (REFERENCE_URL), each at Table IX's 25 100 ft/s.
    #[test]
    fn the_comparison_with_the_reflectometers_missed_by_its_rule() {
        let v = 25_100.0 * 0.3048;
        let rows = [(2usize, 10.044e9, 70.0e3, 0.940), (2, 3.348e9, 77.7e3, 3.284), (1, 10.044e9, 80.0e3, 2.925)];
        let mut within_2 = 0;
        let mut beyond_4 = 0;
        for (station, f, z, scored) in rows {
            let a = solve(&PlasmaSheathRamC2Question { altitude_m: z, velocity_m_s: v, signal_frequency_hz: f }).unwrap();
            let ratio = a.stations[station - 1].electron_density_m3 / (REFLECTOMETER_CRITERION * critical_density_m3(f));
            assert!(relative(ratio, scored) < 2.0e-3, "station {station} at {z} m: {ratio}");
            if ratio.log10().abs() <= 2.0_f64.log10() {
                within_2 += 1;
            }
            if ratio.log10().abs() > 4.0_f64.log10() {
                beyond_4 += 1;
            }
        }
        assert_eq!((within_2, beyond_4), (1, 0));
        // Above the Knudsen limit, the onsets at 84 km are refused by name.
        assert_eq!(solve(&PlasmaSheathRamC2Question { altitude_m: 84.0e3, velocity_m_s: v, signal_frequency_hz: 3.348e9 }).map(|_| ()), Err(Reject::RarefiedFlow));
    }

    #[test]
    fn the_same_question_gives_the_same_bits() {
        assert_eq!(solve(&question()), solve(&question()));
    }
}
