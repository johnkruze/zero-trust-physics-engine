//! The blast wave of a TNT-equivalent charge in still air at sea level: a
//! spherical charge in free air, or a hemispherical charge on rigid ground.
//! The company's own derivation, a field refined until it settles.
//!
//! **The question.** A charge of `charge_mass_kg` of TNT equivalent, a point
//! at `distance_m` from its centre, and the burst: `FreeAir` (a sphere) or
//! `Surface` (a hemisphere resting on rigid, flat ground). The answer is the
//! incident blast wave at the point: the time of arrival of the shock, its
//! peak overpressure, the duration and impulse of the positive phase, the
//! Friedlander form of the pressure history, and the history itself.
//!
//! **The air.** The International Standard Atmosphere at sea level, still
//! and uniform: 101,325 Pa, 288.15 K, an ideal gas of specific gas constant
//! 287.05287 J/(kg K) and ratio of specific heats 1.4 (density 1.225 kg/m^3,
//! sound speed 340.294 m/s).
//!
//! **The charge as energy.** A kilogram of TNT equivalent is 4.184 MJ, the
//! convention by which energy is stated in TNT (the ton of TNT is
//! 4.184e9 J). The blast is driven by that energy, E = W x 4.184 MJ/kg. On
//! rigid ground the ground plane is a plane of symmetry: a hemispherical
//! burst of W is half of a spherical burst of 2W in free air, so its energy
//! is taken as 2 E. The charge enters the air as energy only: it has no mass
//! and no volume of its own (a point source), which holds once the shock has
//! engulfed more air than the charge weighs (the near-field limit below).
//!
//! **Similarity.** For a point release of energy E in an ideal gas at rest
//! (p0, rho0, gamma), dimensional analysis leaves one length, the Sachs
//! length L = (E/p0)^(1/3), and one time, L/c0. Every overpressure over p0
//! is a function of the scaled distance lambda = R/L and the scaled time
//! t c0/L alone (Sachs scaling; at one ambient state it is Hopkinson-Cranz
//! scaling in Z = R/W^(1/3), with lambda = Z (p0/e_TNT)^(1/3) in free air and
//! Z (p0/(2 e_TNT))^(1/3) on the ground). The solver computes that one
//! dimensionless blast wave and scales it to the question.
//!
//! **Where the shock is strong: Sedov-Taylor.** While p0 is negligible
//! against the overpressure, p0 drops out and the blast is self-similar in
//! r/R(t) with R = xi0 (E t^2/rho0)^(1/5). The similarity equations for
//! gamma = 1.4 have Sedov's closed-form solution in the reduced velocity V
//! (Sedov 1959; Landau and Lifshitz, Fluid Mechanics, section 106), which
//! `sedov_taylor` evaluates; the energy integral
//! E = alpha rho0 R^5/t^2 gives alpha = 0.85107 and xi0 = alpha^(-1/5) =
//! 1.03278. Behind a strong shock the overpressure is
//! 8 xi0^5/(25 (gamma + 1)) E/R^3 = 0.1567 E/R^3. The solver starts the
//! wave from this exact profile at lambda = 0.05, where the overpressure is
//! 1254 p0 and p0 changes the shock jump by 0.1 percent.
//!
//! **Where p0 matters: the Euler equations.** From there the spherical Euler
//! equations of the air, with p0 ahead of the shock, carry the wave out to
//! the point: Lagrangian finite volumes in the radius (each cell a fixed mass
//! of air), second-order in space and time (limited linear reconstruction of
//! pressure and velocity, two-stage strong-stability-preserving Runge-Kutta),
//! a two-shock acoustic Riemann solver at every node (Dukowicz's impedances),
//! the face areas averaged over the step so that the volume a node sweeps is
//! exact. The shock is captured. The peak overpressure is the Rankine-Hugoniot
//! jump of the shock's Mach number, which a least-squares quadratic through
//! the arrival times at seven radii within 5 percent of the point gives; the
//! history is the pressure at the point, its positive phase ending where it
//! first falls to p0; the impulse is the integral of the history over that
//! positive phase.
//!
//! **Where the shock becomes acoustic: weak-shock theory.** Beyond lambda = 4
//! the overpressure is under 0.065 p0 and the wave is a weak shock heading a
//! nearly linear spherical pulse, r p' = F(t - r/c0). Each level of F travels
//! at c0 + (gamma + 1) u/2 with u = p'/(rho0 c0), so it gains
//! (gamma + 1)/(2 rho0 c0^3) F ln(r/r1) in retarded time on its way out, and
//! the shock sits where the overrun areas balance (Whitham's equal-area
//! rule). The solver carries the Friedlander form of the positive phase at
//! lambda = 4 out to the point this way. This is a regime of the same wave,
//! not a limit.
//!
//! **What is held.** At every step of the Euler run the energy of the air
//! above ambient (kinetic, and internal less the ambient's) equals the
//! charge's energy E: the fluxes telescope, the area of each face is the
//! one whose swept volume is exact, and cells added ahead of the shock are at
//! ambient, so the ledger closes to rounding (`ENERGY_LEDGER_TOLERANCE`). The
//! Friedlander form p = Ps (1 - tau/td) exp(-a tau/td) takes the decay
//! coefficient a for which its integral, Ps td (1/a - (1 - exp(-a))/a^2),
//! is the impulse, solved to rounding (`IMPULSE_CLOSURE_TOLERANCE`). In the
//! weak-shock regime the area of r p' over the positive phase, r times the
//! impulse, is carried unchanged to rounding (`AREA_TOLERANCE`). The whole
//! answer is computed on two grids, one twice as fine as the other; it is
//! returned only when the two agree within `SETTLE_*`, and the fine grid's
//! values are returned.
//!
//! **Limits.** A charge mass or a distance of zero or less has no blast. The
//! near field: inside the radius at which the air the shock has engulfed
//! weighs as much as the charge, (4/3) pi R^3 rho0 = W in free air and
//! (2/3) pi R^3 rho0 = W on the ground, the charge's own detonation products
//! carry the blast and a point source of energy alone cannot describe it;
//! that is Z = 0.5798 m/kg^(1/3) in free air and 0.7305 on the ground, both
//! lambda = 0.1677 (`ChargeProductsNearField`). There the shock is at Mach
//! 5.5067, 34.21 p0, about 1,970 K behind it in gamma = 1.4 air, where the
//! air's own vibrational excitation also begins to change the jump. The
//! uniform atmosphere: the air's pressure falls by a factor e in a scale
//! height, R T/g = 8,434.5 m, and by 1 - e^-0.1 = 9.52 percent in a tenth of
//! one, 843.45 m. The model's air is uniform, so one criterion holds the
//! wave to that tenth: the shock's radius when the positive phase at the
//! point ends, the air the answer rests on, is at most 843.45 m
//! (`WaveBeyondUniformAtmosphere`; served as `shock_radius_at_end_m`). Two
//! cases of it show in the question and are refused at the door: a point at
//! 843.45 m or beyond, which the shock passes before its phase ends
//! (`DistanceBeyondUniformAtmosphere`), and a charge above 106,342,834 kg
//! in free air or half that on the ground, whose shock is past 843.45 m as
//! the phase ends at every point (`ChargeBeyondUniformAtmosphere`): from the
//! near-field limit out to Sachs distance 0.2 the phases end at nearly one
//! time and the radius then is 0.51507 to 0.51522 Sachs lengths, least
//! ([`LEAST_SHOCK_REACH_SACHS`]) at 0.1835, and it rises beyond. The continuum: a gas is a continuum, and the Euler
//! equations hold, where its mean free path, 6.37e-8 m at sea level, is
//! less than 0.01 of the flow's length; a distance under 100 mean free paths,
//! 6.37e-6 m, is `DistanceBelowContinuum`. The sharp shock: the model's
//! shock is a jump, and the air's own viscosity and heat conduction give a
//! weak shock Taylor's rise, tau_s = 2 rho0 delta/(beta Ps) with delta the
//! diffusivity of sound, 2.771e-5 m^2/s (served as `shock_rise_time_s`).
//! Joined to the Friedlander decay behind it the rise lowers the peak by
//! (e/4)(1 + ln(4/e)), e = 2 (1 + a) tau_s/td; where that passes 2.5
//! percent, the line the peak is settled to, the shock is not sharp against
//! the phase it heads and the answer is refused as `ShockNotSharp`: a small
//! charge far out (a gram at 840 m; near 843 m a microgram's rise is about
//! 147 times its phase). Molecular relaxation of oxygen and nitrogen, which hangs on the
//! humidity, lengthens a real weak shock's rise beyond Taylor's, so a gauge
//! in moist air can read a lower peak than a sharp shock where this limit
//! answers.
//!
//! **What the model is.** A point source of the charge's energy in uniform
//! ideal air; the answer is the incident (side-on) wave of the positive
//! phase over rigid ground.
//!
//! **Compared with TNT** (tests `dewey_tnt_free_field_comparison` and
//! `kingery_bulmash_surface_burst_comparison`). Against Dewey's TNT
//! free-field interface (TNTFFI 1.1c, CC0, measurement-derived from 4 to
//! 500 kg shots), 1 kg in free air: the peak 33 to 22 percent below from
//! Z = 1.3 to 2 m/kg^(1/3) and 17 to 6 percent below from 3 to 15; the
//! impulse 22 percent above at 1.3 and 3 to 9 percent below from 2 to 15;
//! the duration 79 percent long at 1.3, 2 to 7 percent long from 4 to 15;
//! the arrival 1 to 8 percent late from 1.3 to 15. TNT's products sustain its shock where the
//! point source's energy, spent heating the air it first shocks, cannot.
//! Against Kingery-Bulmash for a surface burst, as simplified by Swisdak
//! (1994) and coded by fcento100 (MIT; neither original was obtained, so the
//! coefficients have not been compared with them; the coder's metric and
//! imperial sets agree within 1 percent): the peak 5 to 16 percent below
//! from Z = 2 to 40, the impulse 15 to 20 percent below, the arrival within
//! 4 percent; from Z = 100 to 198.5 the peak is 16 to 39 percent above it.
//! Against Kinney-Graham's free-air peak (as Karlos and Solomos, JRC EUR
//! 26456 EN, 2013, Eq. 8, print it): within 6 percent from Z = 5 to 15, 26
//! to 33 percent below from 1 to 2, 9 to 37 percent below from 20 to 198.5.
//!
//! **Compared with gauge records** (test
//! `dewey_sochet_propane_oxygen_held_back_comparison`; record
//! `evidence/research/blast_overpressure/`). Dewey and Sochet (2020,
//! Borealis 10.5683/SP2/VHN1UQ, CC BY 4.0): hemispherical surface bursts of
//! 0.37 g of propane and oxygen in 5 cm soap bubbles, gas micro-charges, not
//! high explosive, given the model as their heat of reaction in TNT at
//! 4.184 MJ/kg (stoichiometric 0.8841 g, rich 0.8576 g, lean 0.7317 g) on
//! rigid ground. Pass marks fixed before scoring (the scout's, unchanged;
//! commit cb528d1): rich and lean held back, 0.255 to 0.904 m, peak within
//! 15 percent, impulse within 20, arrival within 5, 20 of 24 traces. Missed:
//! 0 of 24 meet every line; the peak 33 to 72 percent above the measured,
//! the impulse 43 to 78 percent above (13 traces can be scored), the arrival
//! 5 to 25 percent early. After the scoring, and no constant of the model:
//! the point source meets these peaks at 38 to 58 percent of the charges'
//! heat of reaction (median 46) and the impulses at 39 to 57 (median 47).

use genesis_kernel::math;
use crate::normal::NormalOrZero;
use std::fmt;

// ---------------------------------------------------------------- the air

/// Ratio of specific heats of the air.
pub const GAMMA: f64 = 1.4;
/// Ambient pressure, the International Standard Atmosphere at sea level, Pa.
pub const AMBIENT_PRESSURE_PA: f64 = 101_325.0;
/// Ambient temperature, the International Standard Atmosphere at sea level, K.
pub const AMBIENT_TEMPERATURE_K: f64 = 288.15;
/// Specific gas constant of dry air in the International Standard Atmosphere, J/(kg K).
pub const AIR_GAS_CONSTANT_J_PER_KG_K: f64 = 287.052_87;
/// Standard acceleration of gravity, m/s^2, for the scale height.
pub const STANDARD_GRAVITY_M_PER_S2: f64 = 9.806_65;
/// Energy of one kilogram of TNT equivalent, J: the convention of TNT equivalence.
pub const TNT_ENERGY_J_PER_KG: f64 = 4.184e6;
/// The fraction of a scale height over which the model's uniform air stands:
/// across it the ambient pressure changes by 1 - e^-0.1 = 9.52 percent.
pub const UNIFORM_AIR_FRACTION_OF_SCALE_HEIGHT: f64 = 0.1;
/// Dynamic viscosity of the air at sea level, Pa s: the U.S. Standard
/// Atmosphere 1976 (Sutherland's law at 288.15 K).
pub const AIR_VISCOSITY_PA_S: f64 = 1.7894e-5;
/// Thermal conductivity of the air at sea level, W/(m K): the U.S. Standard
/// Atmosphere 1976.
pub const AIR_THERMAL_CONDUCTIVITY_W_PER_M_K: f64 = 2.5326e-2;
/// The Knudsen number, the air's mean free path over the distance, past
/// which a gas is not a continuum (0.01, the edge of the continuum regime).
pub const CONTINUUM_KNUDSEN_MAX: f64 = 0.01;

/// Ambient density, kg/m^3: p0/(R T0).
pub fn ambient_density_kg_per_m3() -> f64 {
    AMBIENT_PRESSURE_PA / (AIR_GAS_CONSTANT_J_PER_KG_K * AMBIENT_TEMPERATURE_K)
}

/// Ambient sound speed, m/s: (gamma R T0)^(1/2).
pub fn sound_speed_m_per_s() -> f64 {
    (GAMMA * AIR_GAS_CONSTANT_J_PER_KG_K * AMBIENT_TEMPERATURE_K).sqrt()
}

/// The isothermal scale height of the air, R T0/g, m: 8,434.5 m.
pub const SCALE_HEIGHT_M: f64 = AIR_GAS_CONSTANT_J_PER_KG_K * AMBIENT_TEMPERATURE_K / STANDARD_GRAVITY_M_PER_S2;

/// The farthest distance served, a tenth of a scale height, m: 843.45 m.
pub const DISTANCE_MAX_M: f64 = UNIFORM_AIR_FRACTION_OF_SCALE_HEIGHT * SCALE_HEIGHT_M;

/// The Sachs distance at which the shock's radius at the end of the
/// positive phase is least: from the near-field limit, 0.1677, out to about
/// 0.2 the phases end at nearly one time, the overpressure falling to
/// ambient everywhere there as the expansion behind the shock arrives, and
/// the radius is 0.51507 to 0.51522 Sachs lengths, least here on a scan by
/// 0.0005 (a test holds it below its neighbours'). Beyond, it rises.
pub const LEAST_REACH_SACHS_DISTANCE: f64 = 0.1835;
/// That least radius, Sachs lengths, on the fine grid. A test computes it
/// again.
pub const LEAST_SHOCK_REACH_SACHS: f64 = 0.5150689976238669;

/// The largest free-air charge any point is answered at, kg of TNT
/// equivalent: the one whose shock is a tenth of a scale height out as the
/// positive phase ends at [`LEAST_REACH_SACHS_DISTANCE`], where that radius
/// is least. On the ground, half of it.
pub const CHARGE_MASS_MAX_KG: f64 = {
    let reach = DISTANCE_MAX_M / LEAST_SHOCK_REACH_SACHS;
    AMBIENT_PRESSURE_PA * reach * reach * reach / TNT_ENERGY_J_PER_KG
};

/// The diffusivity of sound of the air, m^2/s: (4/3 mu + (gamma - 1) k / c_p)
/// / rho0, from its shear viscosity and its heat conduction (Pierce,
/// Acoustics, 1989, section 10-6; Hamilton and Blackstock, Nonlinear
/// Acoustics, 1998, chapter 4). 2.771e-5 m^2/s.
pub fn sound_diffusivity_m2_per_s() -> f64 {
    let specific_heat = GAMMA * AIR_GAS_CONSTANT_J_PER_KG_K / (GAMMA - 1.0);
    (4.0 / 3.0 * AIR_VISCOSITY_PA_S + (GAMMA - 1.0) * AIR_THERMAL_CONDUCTIVITY_W_PER_M_K / specific_heat) / ambient_density_kg_per_m3()
}

/// The thermoviscous rise time of a weak shock of overpressure `jump_pa`, s:
/// Taylor's steady shock, the solution of Burgers' equation
/// p = (Ps/2)(1 + tanh(tau/tau_s)), tau_s = 2 rho0 delta/(beta Ps), with
/// beta = (gamma + 1)/2 the coefficient of nonlinearity.
pub fn shock_rise_time_s(jump_pa: f64) -> f64 {
    2.0 * ambient_density_kg_per_m3() * sound_diffusivity_m2_per_s() / (0.5 * (GAMMA + 1.0) * jump_pa)
}

/// How much a shock of rise time `rise_s` at the head of a Friedlander
/// positive phase of duration `duration_s` and decay `decay` lowers its
/// peak, as a fraction: the Taylor shock joined to the decay behind it,
/// whose slope is Ps (1 + a)/td, peaks below the jump by
/// (e/4)(1 + ln(4/e)), e = 2 (1 + a) tau_s/td, to first order in e.
/// One for e of 4 or more, where the rise spans the phase.
pub fn peak_smoothing(rise_s: f64, duration_s: f64, decay: f64) -> f64 {
    let e = 2.0 * (1.0 + decay).abs() * rise_s / duration_s;
    if !(e < 4.0) {
        return 1.0;
    }
    if e == 0.0 {
        return 0.0;
    }
    0.25 * e * (1.0 + math::log(4.0 / e))
}

/// The air's mean free path at sea level, m: (mu/p0)(pi R T0/2)^(1/2),
/// the viscosity's own. 6.37e-8 m.
pub fn mean_free_path_m() -> f64 {
    AIR_VISCOSITY_PA_S / AMBIENT_PRESSURE_PA * (core::f64::consts::PI * AIR_GAS_CONSTANT_J_PER_KG_K * AMBIENT_TEMPERATURE_K / 2.0).sqrt()
}

/// The nearest distance served, m: the mean free path over
/// [`CONTINUUM_KNUDSEN_MAX`], 6.37e-6 m (a test computes it again).
pub const DISTANCE_MIN_M: f64 = 6.365624517364194e-6;

/// The nearest distance served, m.
pub fn distance_min_m() -> f64 {
    DISTANCE_MIN_M
}

/// The isothermal scale height of the air, R T0/g, m.
pub fn scale_height_m() -> f64 {
    SCALE_HEIGHT_M
}

/// The farthest distance served, a tenth of a scale height, m.
pub fn distance_max_m() -> f64 {
    DISTANCE_MAX_M
}

/// The largest free-air charge any point is answered at, kg. On the ground,
/// half of it.
pub fn charge_mass_max_kg() -> f64 {
    CHARGE_MASS_MAX_KG
}

/// The Sachs scaled distance of the near-field limit: the radius at which
/// the air engulfed weighs as much as the charge, over the Sachs length,
/// (3 p0/(4 pi rho0 e_TNT))^(1/3). The same number in free air and on the
/// ground.
pub fn near_field_sachs_distance() -> f64 {
    math::cbrt(3.0 * AMBIENT_PRESSURE_PA / (4.0 * core::f64::consts::PI * ambient_density_kg_per_m3() * TNT_ENERGY_J_PER_KG))
}

/// The scaled distance Z of the near-field limit, m/kg^(1/3).
pub fn near_field_scaled_distance(burst: Burst) -> f64 {
    near_field_sachs_distance() * math::cbrt(burst.energy_factor() * TNT_ENERGY_J_PER_KG / AMBIENT_PRESSURE_PA)
}

// --------------------------------------------------------- the numerics

/// The Sachs scaled shock radius at which the wave starts from the
/// Sedov-Taylor profile.
pub const SEDOV_START_SACHS_RADIUS: f64 = 0.05;
/// Points of the tabulated Sedov-Taylor profile.
pub const SEDOV_TABLE_POINTS: usize = 20_000;
/// Beyond this Sachs scaled distance the wave is carried by weak-shock theory.
pub const WEAK_SHOCK_FROM_SACHS_DISTANCE: f64 = 4.0;
/// The fine grid's cell, in Sachs lengths, at most.
pub const FINE_CELL_SACHS: f64 = 0.001_25;
/// The fine grid has at least this many cells across the distance of the point.
pub const FINE_CELLS_ACROSS_DISTANCE: f64 = 800.0;
/// The coarse grid's cells are this many times the fine grid's.
pub const COARSE_FACTOR: f64 = 2.0;
/// Inside `UNIFORM_FROM_FRACTION` of the point's distance the cells widen
/// toward the centre as cell x (that radius)/r, but are never wider than the
/// radius over this many (fine grid; half as many on the coarse).
pub const FINE_CELLS_PER_RADIUS: f64 = 40.0;
/// From this fraction of the point's distance outward the cells are uniform.
pub const UNIFORM_FROM_FRACTION: f64 = 0.6;
/// The innermost cell of the Sedov-Taylor start holds the air from the centre
/// to this fraction of the start radius.
pub const SEDOV_CORE_FRACTION: f64 = 0.2;
/// Inside the start radius the cells widen inward by this factor per cell.
pub const SEDOV_STRETCH: f64 = 1.2;
/// Courant number of the step.
pub const COURANT_NUMBER: f64 = 0.4;
/// The seven gauges for the shock's speed span this fraction of the distance either side of the point.
pub const SPEED_GAUGE_SPREAD: f64 = 0.05;
/// Points of the Friedlander positive phase carried by weak-shock theory.
pub const WEAK_SHOCK_POINTS: usize = 4096;
/// Points of the history and the Friedlander form returned.
pub const HISTORY_POINTS: usize = 33;
/// The most steps of one Euler run.
pub const MAX_STEPS: usize = 400_000;
/// Largest departure of the air's energy above ambient from the charge's energy, as a fraction of it, at any step.
pub const ENERGY_LEDGER_TOLERANCE: f64 = 1.0e-10;
/// Largest departure of the Friedlander form's integral from the impulse, as a fraction.
pub const IMPULSE_CLOSURE_TOLERANCE: f64 = 1.0e-12;
/// Largest change of r times the impulse through the weak-shock regime, as a fraction.
pub const AREA_TOLERANCE: f64 = 1.0e-12;
/// Largest change of the peak overpressure from the coarse grid to the fine.
pub const SETTLE_PEAK: f64 = 0.025;
/// Largest change of the impulse from the coarse grid to the fine.
pub const SETTLE_IMPULSE: f64 = 0.01;
/// Largest change of the positive phase duration from the coarse grid to the fine.
pub const SETTLE_DURATION: f64 = 0.015;
/// Largest change of the arrival time from the coarse grid to the fine.
pub const SETTLE_ARRIVAL: f64 = 0.005;

// ------------------------------------------------------- the question

/// Where the charge is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Burst {
    /// A spherical charge in free air.
    FreeAir,
    /// A hemispherical charge resting on rigid, flat ground: half of a spherical burst of twice the charge.
    Surface,
}

impl Burst {
    /// The charge's energy over W x 4.184 MJ/kg as a free-air point source: 1, or 2 on rigid ground.
    pub fn energy_factor(self) -> f64 {
        match self {
            Burst::FreeAir => 1.0,
            Burst::Surface => 2.0,
        }
    }
}

/// What the caller asks.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BlastQuestion {
    /// Mass of the charge, kg of TNT equivalent.
    pub charge_mass_kg: f64,
    /// Distance of the point from the charge's centre, m.
    pub distance_m: f64,
    pub burst: Burst,
}

/// The blast wave at the point.
#[derive(Clone, Debug, PartialEq)]
pub struct BlastWave {
    /// Z = R/W^(1/3), m/kg^(1/3).
    pub scaled_distance_m_per_kg13: f64,
    /// lambda = R/(E/p0)^(1/3).
    pub sachs_scaled_distance: f64,
    /// The charge's energy as a free-air point source, J (twice W x 4.184 MJ/kg on the ground).
    pub charge_energy_j: f64,
    pub arrival_time_s: f64,
    pub peak_overpressure_pa: f64,
    pub shock_mach_number: f64,
    pub positive_phase_duration_s: f64,
    pub positive_impulse_pa_s: f64,
    /// a of p = Ps (1 - tau/td) exp(-a tau/td).
    pub friedlander_decay: f64,
    /// Hopkinson-scaled: over W^(1/3).
    pub scaled_arrival_time_ms_per_kg13: f64,
    pub scaled_positive_phase_duration_ms_per_kg13: f64,
    pub scaled_positive_impulse_kpa_ms_per_kg13: f64,
    /// What the strong-shock Sedov-Taylor law alone gives at this distance, 0.1567 E/R^3, Pa.
    pub sedov_taylor_overpressure_pa: f64,
    /// The farthest Sachs scaled distance the Euler equations carried the wave; beyond it weak-shock theory.
    pub euler_reach_sachs_distance: f64,
    /// How far from the charge the shock is when the positive phase at the point ends, m: the air the answer rests on.
    pub shock_radius_at_end_m: f64,
    /// The thermoviscous rise time of the shock at the point, s: Taylor's 2 rho0 delta/(beta Ps).
    pub shock_rise_time_s: f64,
    /// Times after arrival of the history, s.
    pub history_time_after_arrival_s: Vec<f64>,
    /// The overpressure computed at the point at those times, Pa.
    pub history_overpressure_pa: Vec<f64>,
    /// The Friedlander form at those times, Pa.
    pub friedlander_overpressure_pa: Vec<f64>,
    /// Largest departure of the air's energy above ambient from the charge's energy over the fine grid's run, as a fraction.
    pub energy_ledger_residual: f64,
    pub impulse_closure_residual: f64,
    /// Change of r times the impulse through the weak-shock regime, as a fraction; zero where it was not used.
    pub weak_shock_area_residual: f64,
    pub grid_change_peak: f64,
    pub grid_change_impulse: f64,
    pub grid_change_duration: f64,
    pub grid_change_arrival: f64,
    /// Steps of the two Euler runs together.
    pub steps: u64,
}

/// Why no blast wave is returned.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum BlastReject {
    /// An input is not a finite number.
    InputNotFinite,
    /// The charge mass is zero or less: there is no charge.
    ChargeMassNotPositive,
    /// The distance is zero or less.
    DistanceNotPositive,
    /// The charge is above 106,342,834 kg of TNT equivalent in free air, or half that on the ground: at every point of it the shock is beyond a tenth of the air's scale height, 843.45 m, when the positive phase there ends, that radius being least, 0.51507 Sachs lengths, near the charge, where the phases end at nearly one time.
    ChargeBeyondUniformAtmosphere,
    /// The distance is a tenth of the air's scale height, 843.45 m, or beyond, where the ambient pressure differs from the charge's by 1 - e^-0.1 = 9.52 percent or more: the shock is past it before the positive phase at the point ends.
    DistanceBeyondUniformAtmosphere,
    /// The distance is under 100 mean free paths of the air, 6.37e-6 m: the mean free path over the distance, the Knudsen number, passes 0.01, the edge of the continuum, where the Euler equations of a gas stop holding.
    DistanceBelowContinuum,
    /// The point is inside the radius at which the air the shock has engulfed weighs as much as the charge, Z = 0.5798 m/kg^(1/3) in free air and 0.7305 on the ground: the charge's detonation products carry the blast there, and a point source of its energy cannot.
    ChargeProductsNearField,
    /// When the positive phase at the point ends, the shock is beyond a tenth of the air's scale height, 843.45 m, from the charge: the air the answer rests on reaches where the ambient pressure differs from the charge's by more than 1 - e^-0.1 = 9.52 percent, and the model's air is uniform.
    WaveBeyondUniformAtmosphere,
    /// The shock's thermoviscous rise time, Taylor's 2 rho0 delta/(beta Ps), lowers the peak of the positive phase it heads by more than 2.5 percent, the peak's own settling line: the shock is not sharp against its phase, and the inviscid wave the model carries is not the air's.
    ShockNotSharp,
    /// A value of the flow was not finite.
    NonFiniteValue,
    /// A cell's internal energy fell to zero or below.
    InternalEnergyNotPositive,
    /// A cell's outer face crossed its inner face.
    CellInverted,
    /// The energy of the air above ambient departed from the charge's energy by more than 1e-10 of it at a step.
    EnergyLedgerBroken,
    /// The Euler run reached 400,000 steps before the positive phase at the point ended.
    StepCapReached,
    /// The Friedlander form's integral departs from the impulse by more than 1e-12 of it.
    ImpulseClosureBroken,
    /// Weak-shock theory changed r times the impulse by more than 1e-12 of it.
    AreaNotCarried,
    /// The coarse and fine grids disagree by more than 2.5 percent in the peak, 1 percent in the impulse, 1.5 percent in the duration or 0.5 percent in the arrival time.
    NotSettled,
}

/// Every refusal, in the order of the enum.
pub const ALL_REJECTS: [BlastReject; 17] = [
    BlastReject::InputNotFinite,
    BlastReject::ChargeMassNotPositive,
    BlastReject::DistanceNotPositive,
    BlastReject::ChargeBeyondUniformAtmosphere,
    BlastReject::DistanceBeyondUniformAtmosphere,
    BlastReject::DistanceBelowContinuum,
    BlastReject::ChargeProductsNearField,
    BlastReject::WaveBeyondUniformAtmosphere,
    BlastReject::ShockNotSharp,
    BlastReject::NonFiniteValue,
    BlastReject::InternalEnergyNotPositive,
    BlastReject::CellInverted,
    BlastReject::EnergyLedgerBroken,
    BlastReject::StepCapReached,
    BlastReject::ImpulseClosureBroken,
    BlastReject::AreaNotCarried,
    BlastReject::NotSettled,
];

impl BlastReject {
    pub fn name(&self) -> &'static str {
        match self {
            Self::InputNotFinite => "InputNotFinite",
            Self::ChargeMassNotPositive => "ChargeMassNotPositive",
            Self::DistanceNotPositive => "DistanceNotPositive",
            Self::ChargeBeyondUniformAtmosphere => "ChargeBeyondUniformAtmosphere",
            Self::DistanceBeyondUniformAtmosphere => "DistanceBeyondUniformAtmosphere",
            Self::DistanceBelowContinuum => "DistanceBelowContinuum",
            Self::ChargeProductsNearField => "ChargeProductsNearField",
            Self::WaveBeyondUniformAtmosphere => "WaveBeyondUniformAtmosphere",
            Self::ShockNotSharp => "ShockNotSharp",
            Self::NonFiniteValue => "NonFiniteValue",
            Self::InternalEnergyNotPositive => "InternalEnergyNotPositive",
            Self::CellInverted => "CellInverted",
            Self::EnergyLedgerBroken => "EnergyLedgerBroken",
            Self::StepCapReached => "StepCapReached",
            Self::ImpulseClosureBroken => "ImpulseClosureBroken",
            Self::AreaNotCarried => "AreaNotCarried",
            Self::NotSettled => "NotSettled",
        }
    }
}

impl BlastReject {
    /// The sentence above the variant.
    pub fn meaning(&self) -> &'static str {
        match self {
            Self::InputNotFinite => "An input is not a finite number.",
            Self::ChargeMassNotPositive => "The charge mass is zero or less: there is no charge.",
            Self::DistanceNotPositive => "The distance is zero or less.",
            Self::ChargeBeyondUniformAtmosphere => "The charge is above 106,342,834 kg of TNT equivalent in free air, or half that on the ground: at every point of it the shock is beyond a tenth of the air's scale height, 843.45 m, when the positive phase there ends, that radius being least, 0.51507 Sachs lengths, near the charge, where the phases end at nearly one time.",
            Self::DistanceBeyondUniformAtmosphere => "The distance is a tenth of the air's scale height, 843.45 m, or beyond, where the ambient pressure differs from the charge's by 1 - e^-0.1 = 9.52 percent or more: the shock is past it before the positive phase at the point ends.",
            Self::DistanceBelowContinuum => "The distance is under 100 mean free paths of the air, 6.37e-6 m: the mean free path over the distance, the Knudsen number, passes 0.01, the edge of the continuum, where the Euler equations of a gas stop holding.",
            Self::ChargeProductsNearField => "The point is inside the radius at which the air the shock has engulfed weighs as much as the charge, Z = 0.5798 m/kg^(1/3) in free air and 0.7305 on the ground: the charge's detonation products carry the blast there, and a point source of its energy cannot.",
            Self::WaveBeyondUniformAtmosphere => "When the positive phase at the point ends, the shock is beyond a tenth of the air's scale height, 843.45 m, from the charge: the air the answer rests on reaches where the ambient pressure differs from the charge's by more than 1 - e^-0.1 = 9.52 percent, and the model's air is uniform.",
            Self::ShockNotSharp => "The shock's thermoviscous rise time, Taylor's 2 rho0 delta/(beta Ps), lowers the peak of the positive phase it heads by more than 2.5 percent, the peak's own settling line: the shock is not sharp against its phase, and the inviscid wave the model carries is not the air's.",
            Self::NonFiniteValue => "A value of the flow was not finite.",
            Self::InternalEnergyNotPositive => "A cell's internal energy fell to zero or below.",
            Self::CellInverted => "A cell's outer face crossed its inner face.",
            Self::EnergyLedgerBroken => "The energy of the air above ambient departed from the charge's energy by more than 1e-10 of it at a step.",
            Self::StepCapReached => "The Euler run reached 400,000 steps before the positive phase at the point ended.",
            Self::ImpulseClosureBroken => "The Friedlander form's integral departs from the impulse by more than 1e-12 of it.",
            Self::AreaNotCarried => "Weak-shock theory changed r times the impulse by more than 1e-12 of it.",
            Self::NotSettled => "The coarse and fine grids disagree by more than 2.5 percent in the peak, 1 percent in the impulse, 1.5 percent in the duration or 0.5 percent in the arrival time.",
        }
    }
}

impl fmt::Display for BlastReject {
    /// The sentence, with its first letter lowered and no full stop.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let meaning = self.meaning();
        let body = &meaning[..meaning.len() - 1];
        let mut letters = body.chars();
        match letters.next() {
            Some(first) => write!(f, "{}{}", first.to_lowercase(), letters.as_str()),
            None => Ok(()),
        }
    }
}

// ------------------------------------------------------ Sedov-Taylor

/// Sedov's closed-form solution of the strong point blast in a uniform gas,
/// spherical, tabulated from the centre to the shock.
#[derive(Clone, Debug)]
pub struct SedovTaylor {
    /// r/R, from 0 at the centre to 1 at the shock.
    pub xi: Vec<f64>,
    /// rho/rho0.
    pub density: Vec<f64>,
    /// V, with u = (2 r/(5 t)) V.
    pub velocity: Vec<f64>,
    /// Z, with c^2 = (2 r/(5 t))^2 Z.
    pub sound: Vec<f64>,
    /// The mass inside xi per steradian, in rho0 R^3 (1/3 at the shock).
    pub mass: Vec<f64>,
    /// E = alpha rho0 R^5 / t^2.
    pub alpha: f64,
    /// R = xi0 (E t^2/rho0)^(1/5).
    pub xi0: f64,
}

/// The Sedov-Taylor profile for `gamma`, with `points` points from the
/// centre to the shock (Landau and Lifshitz, section 106).
pub fn sedov_taylor(gamma: f64, points: usize) -> SedovTaylor {
    let g = gamma;
    let nu1 = -(13.0 * g * g - 7.0 * g + 12.0) / ((3.0 * g - 1.0) * (2.0 * g + 1.0));
    let nu2 = 5.0 * (g - 1.0) / (2.0 * g + 1.0);
    let nu3 = 3.0 / (2.0 * g + 1.0);
    let nu4 = -nu1 / (2.0 - g);
    let nu5 = -2.0 / (2.0 - g);
    let v_shock = 2.0 / (g + 1.0);
    let mut xi = vec![0.0];
    let mut density = vec![0.0];
    let mut velocity = vec![1.0 / g];
    let mut sound = vec![f64::INFINITY];
    for k in 1..=points {
        let s = k as f64 / points as f64;
        // w = g V - 1, from 0 at the centre to (g - 1)/(g + 1) at the shock,
        // clustered at the centre where xi^5 goes as w^nu2.
        let w = (g - 1.0) / (g + 1.0) * math::powi(s, 10);
        let v = if k == points { v_shock } else { (1.0 + w) / g };
        let a = (g + 1.0) / (7.0 - g) * (5.0 - (3.0 * g - 1.0) * v);
        let b = (g + 1.0) / (g - 1.0) * w;
        let c = (g + 1.0) / (g - 1.0) * (1.0 - v);
        if k == points {
            xi.push(1.0);
            density.push((g + 1.0) / (g - 1.0));
        } else {
            xi.push(math::powf(math::powf(0.5 * (g + 1.0) * v, -2.0) * math::powf(a, nu1) * math::powf(b, nu2), 0.2));
            density.push((g + 1.0) / (g - 1.0) * math::powf(b, nu3) * math::powf(a, nu4) * math::powf(c, nu5));
        }
        velocity.push(v);
        sound.push(g * (g - 1.0) * (1.0 - v) * v * v / (2.0 * w));
    }
    let mut mass = vec![0.0; xi.len()];
    let mut energy = 0.0;
    let energy_density = |k: usize| -> f64 {
        if k == 0 { 0.0 } else { density[k] * (velocity[k] * velocity[k] / 2.0 + sound[k] / (g * (g - 1.0))) * math::powi(xi[k], 4) }
    };
    for k in 1..xi.len() {
        let dx = xi[k] - xi[k - 1];
        mass[k] = mass[k - 1] + 0.5 * dx * (density[k] * xi[k] * xi[k] + density[k - 1] * xi[k - 1] * xi[k - 1]);
        energy += 0.5 * dx * (energy_density(k) + energy_density(k - 1));
    }
    let alpha = 16.0 * core::f64::consts::PI / 25.0 * energy;
    SedovTaylor { xi, density, velocity, sound, mass, alpha, xi0: math::powf(alpha, -0.2) }
}

/// The strong-shock Sedov-Taylor overpressure at radius `r_m` of a point
/// release of `energy_j`: 2 rho0 U^2/(gamma + 1) with U = dR/dt, which is
/// 8 xi0^5/(25 (gamma + 1)) E/R^3, Pa.
pub fn sedov_taylor_overpressure_pa(sedov: &SedovTaylor, energy_j: f64, r_m: f64) -> f64 {
    8.0 * math::powi(sedov.xi0, 5) / (25.0 * (GAMMA + 1.0)) * energy_j / (r_m * r_m * r_m)
}

// ------------------------------------------------ the Lagrangian run
//
// Units of the run: p0 = 1, rho0 = gamma, c0 = 1, E = 1 (so lengths are in
// Sachs lengths and times in Sachs lengths over c0). Everything is per
// steradian: a cell between radii a and b holds (b^3 - a^3)/3 of volume.

const P0: f64 = 1.0;
const RHO0: f64 = GAMMA;

fn ambient_specific_energy() -> f64 {
    P0 / ((GAMMA - 1.0) * RHO0)
}

fn shell_volume(a: f64, b: f64) -> f64 {
    (b * b * b - a * a * a) / 3.0
}

#[derive(Clone, Copy, Debug)]
struct Grid {
    /// Uniform cell, Sachs lengths.
    cell: f64,
    /// At most this many cells per radius inside `uniform_from`.
    per_radius: f64,
    uniform_from: f64,
}

impl Grid {
    fn width_at(&self, r0: f64) -> f64 {
        if r0 >= self.uniform_from {
            self.cell
        } else {
            (self.cell * self.uniform_from / r0).min(r0 / self.per_radius).max(self.cell)
        }
    }
}

#[derive(Clone, Debug)]
struct Flow {
    /// Node radii, one more than the cells.
    r: Vec<f64>,
    /// Cell masses.
    m: Vec<f64>,
    /// Cell velocities.
    u: Vec<f64>,
    /// Cell specific total energies.
    et: Vec<f64>,
}

struct Work {
    rho: Vec<f64>,
    p: Vec<f64>,
    c: Vec<f64>,
    node_u: Vec<f64>,
    node_p: Vec<f64>,
    area: Vec<f64>,
    slope_p: Vec<f64>,
    slope_u: Vec<f64>,
}

impl Work {
    fn new(cells: usize) -> Work {
        Work {
            rho: vec![0.0; cells],
            p: vec![0.0; cells],
            c: vec![0.0; cells],
            node_u: vec![0.0; cells + 1],
            node_p: vec![0.0; cells + 1],
            area: vec![0.0; cells + 1],
            slope_p: vec![0.0; cells],
            slope_u: vec![0.0; cells],
        }
    }
}

/// The run's start: nodes evenly spaced in the air's undisturbed radius
/// (widening inward inside the start radius), moved to their places in the
/// Sedov-Taylor profile, each cell holding the mass-weighted mean velocity
/// and energy of the profile over its mass, with p0 added to the strong-shock
/// pressure; then the profile's speeds and pressures are scaled together so
/// the energy above ambient is exactly the charge's.
fn sedov_start(sedov: &SedovTaylor, grid: &Grid) -> Result<(Flow, f64), BlastReject> {
    let g = GAMMA;
    let rs = SEDOV_START_SACHS_RADIUS;
    let target = 1.0 / (4.0 * core::f64::consts::PI);
    let time = (sedov.alpha * RHO0 * math::powi(rs, 5)).sqrt();
    // undisturbed radii of the nodes inside the start radius
    let mut inner = vec![rs];
    let mut w = grid.width_at(rs).min(rs / grid.per_radius);
    loop {
        let next = inner[inner.len() - 1] - w;
        if next <= SEDOV_CORE_FRACTION * rs + 0.5 * w {
            break;
        }
        inner.push(next);
        w *= SEDOV_STRETCH;
    }
    inner.push(0.0);
    inner.reverse();
    let shocked = inner.len() - 1;
    let mut r0 = inner;
    for _ in 0..64 {
        let last = r0[r0.len() - 1];
        r0.push(last + grid.width_at(last));
    }
    let cells = r0.len() - 1;
    let total = sedov.mass[sedov.mass.len() - 1];
    // the table index and fraction at which the mass inside is `q` (in rho0 R^3, 1/3 at the shock)
    let locate = |q: f64| -> (usize, f64) {
        let target = q * total * 3.0;
        let mut lo = 0;
        let mut hi = sedov.mass.len() - 1;
        while hi - lo > 1 {
            let mid = (lo + hi) / 2;
            if sedov.mass[mid] <= target { lo = mid } else { hi = mid }
        }
        let span = sedov.mass[hi] - sedov.mass[lo];
        (lo, if span > 0.0 { (target - sedov.mass[lo]) / span } else { 0.0 })
    };
    let mut r = r0.clone();
    for j in 1..shocked {
        let (k, x) = locate(math::powi(r0[j] / rs, 3) / 3.0);
        r[j] = rs * (sedov.xi[k] + x * (sedov.xi[k + 1] - sedov.xi[k]));
    }
    let mut m = vec![0.0; cells];
    let mut u = vec![0.0; cells];
    let mut kinetic = vec![0.0; cells];
    let mut internal = vec![0.0; cells];
    for i in 0..cells {
        m[i] = RHO0 * shell_volume(r0[i], r0[i + 1]);
        if i >= shocked {
            continue;
        }
        let lo = math::powi(r0[i] / rs, 3) * total;
        let hi = math::powi(r0[i + 1] / rs, 3) * total;
        let (k0, _) = locate(math::powi(r0[i] / rs, 3) / 3.0);
        let (k1, _) = locate(math::powi(r0[i + 1] / rs, 3) / 3.0);
        let (mut mm, mut mu, mut mk, mut me) = (0.0, 0.0, 0.0, 0.0);
        for k in k0.max(1)..=(k1 + 1).min(sedov.xi.len() - 1) {
            let a = sedov.mass[k - 1].max(lo);
            let b = sedov.mass[k].min(hi);
            if b <= a {
                continue;
            }
            let dm = b - a;
            let xi = 0.5 * (sedov.xi[k] + sedov.xi[k - 1]);
            let v = 0.5 * (sedov.velocity[k] + sedov.velocity[k - 1]);
            let z = if k == 1 { sedov.sound[k] } else { 0.5 * (sedov.sound[k] + sedov.sound[k - 1]) };
            let speed = 0.4 * xi * rs / time;
            let uu = speed * v;
            mm += dm;
            mu += uu * dm;
            mk += 0.5 * uu * uu * dm;
            me += speed * speed * z / (g * (g - 1.0)) * dm;
        }
        if mm > 0.0 {
            u[i] = mu / mm;
            kinetic[i] = mk / mm;
            internal[i] = me / mm;
        }
    }
    // energy above ambient with the profile scaled by k: k A + B
    let e0 = ambient_specific_energy();
    let (mut a_sum, mut b_sum) = (0.0, 0.0);
    let mut rho = vec![0.0; cells];
    for i in 0..cells {
        rho[i] = m[i] / shell_volume(r[i], r[i + 1]);
        if i < shocked {
            a_sum += m[i] * (internal[i] + kinetic[i]);
            b_sum += m[i] * (P0 / ((g - 1.0) * rho[i]) - e0);
        }
    }
    let k = (target - b_sum) / a_sum;
    if !(k.is_finite() && k > 0.0) {
        return Err(BlastReject::NonFiniteValue);
    }
    let mut et = vec![e0; cells];
    for i in 0..shocked {
        // the spread of the profile's velocity within the cell goes into its internal energy
        let e_int = k * (internal[i] + kinetic[i] - 0.5 * u[i] * u[i]) + P0 / ((g - 1.0) * rho[i]);
        u[i] *= k.sqrt();
        et[i] = e_int + 0.5 * u[i] * u[i];
    }
    Ok((Flow { r, m, u, et }, time / k.sqrt()))
}

fn energy_above_ambient(flow: &Flow) -> f64 {
    let e0 = ambient_specific_energy();
    let mut sum = 0.0;
    for i in 0..flow.m.len() {
        sum += flow.m[i] * (flow.et[i] - e0);
    }
    sum
}

fn primitives(flow: &Flow, w: &mut Work) -> Result<(), BlastReject> {
    for i in 0..flow.m.len() {
        let volume = shell_volume(flow.r[i], flow.r[i + 1]);
        if !(volume > 0.0) {
            return Err(if volume.is_finite() { BlastReject::CellInverted } else { BlastReject::NonFiniteValue });
        }
        let rho = flow.m[i] / volume;
        let e = flow.et[i] - 0.5 * flow.u[i] * flow.u[i];
        if !e.is_finite() {
            return Err(BlastReject::NonFiniteValue);
        }
        if !(e > 0.0) {
            return Err(BlastReject::InternalEnergyNotPositive);
        }
        let p = (GAMMA - 1.0) * rho * e;
        w.rho[i] = rho;
        w.p[i] = p;
        w.c[i] = (GAMMA * p / rho).sqrt();
    }
    Ok(())
}

/// Monotonized-central limited slope.
fn limited(left: f64, right: f64) -> f64 {
    if left * right <= 0.0 {
        0.0
    } else {
        left.signum() * (2.0 * left.abs()).min(2.0 * right.abs()).min(0.5 * (left + right).abs())
    }
}

fn between(x: f64, a: f64, b: f64) -> f64 {
    x.max(a.min(b)).min(a.max(b))
}

/// Node velocities and pressures from the two-shock acoustic Riemann solver.
fn node_states(flow: &Flow, w: &mut Work) {
    let n = flow.m.len();
    let shock_factor = 0.5 * (GAMMA + 1.0);
    let centre = |i: usize| 0.5 * (flow.r[i] + flow.r[i + 1]);
    w.slope_p[0] = 0.0;
    w.slope_u[0] = 0.0;
    w.slope_p[n - 1] = 0.0;
    w.slope_u[n - 1] = 0.0;
    for i in 1..n - 1 {
        let dl = centre(i) - centre(i - 1);
        let dr = centre(i + 1) - centre(i);
        w.slope_p[i] = limited((w.p[i] - w.p[i - 1]) / dl, (w.p[i + 1] - w.p[i]) / dr);
        w.slope_u[i] = limited((flow.u[i] - flow.u[i - 1]) / dl, (flow.u[i + 1] - flow.u[i]) / dr);
    }
    w.node_u[0] = 0.0;
    w.node_p[0] = w.p[0];
    for j in 1..n {
        let (l, r) = (j - 1, j);
        let pl = between(w.p[l] + w.slope_p[l] * (flow.r[j] - centre(l)), w.p[l], w.p[r]);
        let pr = between(w.p[r] - w.slope_p[r] * (centre(r) - flow.r[j]), w.p[l], w.p[r]);
        let ul = between(flow.u[l] + w.slope_u[l] * (flow.r[j] - centre(l)), flow.u[l], flow.u[r]);
        let ur = between(flow.u[r] - w.slope_u[r] * (centre(r) - flow.r[j]), flow.u[l], flow.u[r]);
        let mut zl = w.rho[l] * w.c[l];
        let mut zr = w.rho[r] * w.c[r];
        let (mut us, mut ps) = (0.0, 0.0);
        for _ in 0..3 {
            us = (zl * ul + zr * ur - (pr - pl)) / (zl + zr);
            ps = (zr * pl + zl * pr - zl * zr * (ur - ul)) / (zl + zr);
            zl = w.rho[l] * (w.c[l] + shock_factor * (ul - us).max(0.0));
            zr = w.rho[r] * (w.c[r] + shock_factor * (us - ur).max(0.0));
        }
        w.node_u[j] = us;
        w.node_p[j] = ps;
    }
    // the outer wall: at rest
    let l = n - 1;
    w.node_u[n] = 0.0;
    w.node_p[n] = w.p[l] + w.rho[l] * w.c[l] * flow.u[l];
}

/// One forward-Euler stage of length dt from `flow` into `out`. With
/// `fresh`, `w` already holds the primitives of `flow`.
fn stage(flow: &Flow, w: &mut Work, dt: f64, out: &mut Flow, fresh: bool) -> Result<(), BlastReject> {
    if !fresh {
        primitives(flow, w)?;
    }
    node_states(flow, w);
    let n = flow.m.len();
    for j in 0..=n {
        let a = flow.r[j];
        let b = a + dt * w.node_u[j];
        out.r[j] = b;
        // the area whose swept volume over the step is exact
        w.area[j] = (a * a + a * b + b * b) / 3.0;
    }
    out.r[0] = 0.0;
    w.area[0] = 0.0;
    for i in 0..n {
        let force = w.area[i + 1] * w.node_p[i + 1] - w.area[i] * w.node_p[i];
        let geometric = w.p[i] * (w.area[i + 1] - w.area[i]);
        out.u[i] = flow.u[i] + dt / flow.m[i] * (geometric - force);
        let work = w.area[i + 1] * w.node_p[i + 1] * w.node_u[i + 1] - w.area[i] * w.node_p[i] * w.node_u[i];
        out.et[i] = flow.et[i] - dt / flow.m[i] * work;
    }
    Ok(())
}

/// The pressure over ambient at radius `r`, linear between cell centres.
fn overpressure_at(flow: &Flow, w: &Work, r: f64) -> f64 {
    let n = flow.m.len();
    let (mut lo, mut hi) = (0, n);
    while hi - lo > 1 {
        let mid = (lo + hi) / 2;
        if flow.r[mid] <= r { lo = mid } else { hi = mid }
    }
    let centre = |i: usize| 0.5 * (flow.r[i] + flow.r[i + 1]);
    let (a, b) = if r >= centre(lo) { (lo, (lo + 1).min(n - 1)) } else { (lo.saturating_sub(1), lo) };
    let p = if a == b {
        w.p[a]
    } else {
        w.p[a] + (r - centre(a)) / (centre(b) - centre(a)) * (w.p[b] - w.p[a])
    };
    p - P0
}

/// The shock's radius in the flow: where its overpressure, from the
/// outside in, first rises through half the largest (the half-height of its
/// rise, as the arrival times are read), between cell centres.
fn front_of(flow: &Flow, w: &Work) -> f64 {
    let n = flow.m.len();
    let mut largest = 0.0f64;
    for i in 0..n {
        largest = largest.max(w.p[i] - P0);
    }
    let half = 0.5 * largest;
    let centre = |i: usize| 0.5 * (flow.r[i] + flow.r[i + 1]);
    for i in (0..n).rev() {
        let over = w.p[i] - P0;
        if over >= half {
            if i + 1 == n {
                return centre(i);
            }
            let next = w.p[i + 1] - P0;
            return centre(i) + (over - half) / (over - next) * (centre(i + 1) - centre(i));
        }
    }
    centre(0)
}

/// What one Euler run records: the overpressure at the point and at the
/// seven speed gauges at every step, and the shock's radius from the
/// arrival at the point on.
#[derive(Clone, Debug)]
struct Record {
    t: Vec<f64>,
    point: Vec<f64>,
    front: Vec<f64>,
    gauge_r: [f64; 7],
    gauges: [Vec<f64>; 7],
    worst_energy: f64,
    steps: usize,
}

/// A fault put in on purpose by the tests, to show that what must hold is held.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Fault {
    None,
    /// Heat added to one cell at every step, booked nowhere.
    UnbookedHeat,
    /// The level of the pulse behind the shock lowered by one part in a billion in the weak-shock regime.
    LostArea,
}

fn run(sedov: &SedovTaylor, grid: &Grid, point: f64, fault: Fault) -> Result<Record, BlastReject> {
    let (mut flow, mut t) = sedov_start(sedov, grid)?;
    let target = 1.0 / (4.0 * core::f64::consts::PI);
    let e0 = ambient_specific_energy();
    let gauge_r: [f64; 7] = core::array::from_fn(|k| point * (1.0 + SPEED_GAUGE_SPREAD * (k as f64 - 3.0) / 3.0));
    let mut rec = Record { t: Vec::new(), point: Vec::new(), front: Vec::new(), gauge_r, gauges: Default::default(), worst_energy: 0.0, steps: 0 };
    let mut a = flow.clone();
    let mut b = flow.clone();
    let mut w = Work::new(flow.m.len());
    let mut arrived = false;
    let mut peak = 0.0f64;
    let mut outer_steps = 0usize;
    let merge_inside = 0.5 * gauge_r[0];
    // whether `w` holds the primitives of `flow`
    let mut current = false;
    loop {
        // keep at least 40 undisturbed cells ahead of the wave
        let n = flow.m.len();
        let mut front = 0;
        for i in (0..n).rev() {
            if (flow.et[i] - e0).abs() > 1.0e-12 || flow.u[i].abs() > 1.0e-12 {
                front = i;
                break;
            }
        }
        if n - front < 40 {
            for _ in 0..256 {
                let last = flow.r[flow.r.len() - 1];
                let next = last + grid.width_at(last);
                flow.m.push(RHO0 * shell_volume(last, next));
                flow.u.push(0.0);
                flow.et.push(e0);
                flow.r.push(next);
            }
            a = flow.clone();
            b = flow.clone();
            w = Work::new(flow.m.len());
            current = false;
        }
        if !current {
            primitives(&flow, &mut w)?;
        }
        // merge pairs of hot cells far behind the shock whose own step is
        // shorter than the step the uniform cells allow: mass, momentum and
        // energy are summed, so the ledger is untouched
        let n = flow.m.len();
        let mut largest = 0.0f64;
        for i in 0..n {
            largest = largest.max(w.p[i] - P0);
        }
        let mut shock = 0;
        for i in (0..n).rev() {
            if w.p[i] - P0 > 1.0e-3 * largest {
                shock = i;
                break;
            }
        }
        let limit = (0.5 * flow.r[shock + 1]).min(merge_inside);
        let short = grid.cell / 1.3;
        // a pass over the pairs first, so the flow is built anew only when one merges
        let pairs = |flow: &Flow, w: &Work, i: usize| i + 1 < n && flow.r[i + 2] < limit && (flow.r[i + 1] - flow.r[i]) / w.c[i] < short;
        let mut any_pair = false;
        if flow.r[2] < limit {
            let mut i = 0;
            while i < n && flow.r[i + 1] < limit {
                if pairs(&flow, &w, i) {
                    any_pair = true;
                    break;
                }
                i += 1;
            }
        }
        if any_pair {
            let mut merged = Flow { r: vec![0.0], m: Vec::with_capacity(n), u: Vec::with_capacity(n), et: Vec::with_capacity(n) };
            let mut any = false;
            let mut i = 0;
            while i < n {
                let (mut mm, mut mu, mut me, mut next) = (flow.m[i], flow.m[i] * flow.u[i], flow.m[i] * flow.et[i], i + 1);
                if pairs(&flow, &w, i) {
                    mm += flow.m[i + 1];
                    mu += flow.m[i + 1] * flow.u[i + 1];
                    me += flow.m[i + 1] * flow.et[i + 1];
                    next = i + 2;
                    any = true;
                }
                merged.m.push(mm);
                merged.u.push(mu / mm);
                merged.et.push(me / mm);
                merged.r.push(flow.r[next]);
                i = next;
            }
            if any {
                flow = merged;
                a = flow.clone();
                b = flow.clone();
                w = Work::new(flow.m.len());
                primitives(&flow, &mut w)?;
            }
        }
        let n = flow.m.len();
        let mut dt = f64::INFINITY;
        for i in 0..n {
            let du = if i + 1 < n { (flow.u[i + 1] - flow.u[i]).abs() } else { 0.0 };
            dt = dt.min(COURANT_NUMBER * (flow.r[i + 1] - flow.r[i]) / (w.c[i] + du));
        }
        if !(dt.is_finite() && dt > 0.0) {
            return Err(BlastReject::NonFiniteValue);
        }
        stage(&flow, &mut w, dt, &mut a, true)?;
        stage(&a, &mut w, dt, &mut b, false)?;
        for j in 0..flow.r.len() {
            flow.r[j] = 0.5 * (flow.r[j] + b.r[j]);
        }
        for i in 0..n {
            flow.u[i] = 0.5 * (flow.u[i] + b.u[i]);
            flow.et[i] = 0.5 * (flow.et[i] + b.et[i]);
        }
        t += dt;
        rec.steps += 1;
        if fault == Fault::UnbookedHeat {
            flow.et[n / 2] *= 1.0 + 1.0e-9;
        }
        let residual = ((energy_above_ambient(&flow) - target) / target).abs();
        if !residual.is_finite() {
            return Err(BlastReject::NonFiniteValue);
        }
        rec.worst_energy = rec.worst_energy.max(residual);
        if residual > ENERGY_LEDGER_TOLERANCE {
            return Err(BlastReject::EnergyLedgerBroken);
        }
        primitives(&flow, &mut w)?;
        current = true;
        let here = overpressure_at(&flow, &w, point);
        rec.t.push(t);
        rec.point.push(here);
        for k in 0..7 {
            let v = overpressure_at(&flow, &w, gauge_r[k]);
            rec.gauges[k].push(v);
        }
        if here > 1.0e-6 {
            arrived = true;
        }
        rec.front.push(if arrived { front_of(&flow, &w) } else { f64::NAN });
        if rec.gauges[6][rec.gauges[6].len() - 1] > 1.0e-6 {
            outer_steps += 1;
        }
        peak = peak.max(here);
        if arrived && peak > 0.0 && here <= 0.0 && outer_steps > 50 {
            return Ok(rec);
        }
        if rec.steps >= MAX_STEPS {
            return Err(BlastReject::StepCapReached);
        }
    }
}

/// The time at which a record first rises through half its largest value.
fn arrival_of(t: &[f64], p: &[f64]) -> f64 {
    let mut top = 0;
    for k in 0..p.len() {
        if p[k] > p[top] {
            top = k;
        }
    }
    let half = 0.5 * p[top];
    for k in (1..=top).rev() {
        if p[k - 1] < half && p[k] >= half {
            return t[k - 1] + (half - p[k - 1]) / (p[k] - p[k - 1]) * (t[k] - t[k - 1]);
        }
    }
    t[0]
}

/// The positive phase at one Sachs distance, in the run's units.
#[derive(Clone, Debug)]
struct Wave {
    /// Arrival time.
    arrival: f64,
    peak: f64,
    mach: f64,
    duration: f64,
    impulse: f64,
    /// (time after arrival, overpressure) of the history, ending at zero.
    history: Vec<(f64, f64)>,
    /// The shock's radius when the positive phase ends.
    front_at_end: f64,
}

fn wave_of(rec: &Record, point: f64) -> Result<Wave, BlastReject> {
    // the shock's Mach number: a least-squares quadratic of arrival time in radius
    let mut s = [0.0f64; 5];
    let mut ty = [0.0f64; 3];
    for k in 0..7 {
        let x = rec.gauge_r[k] - point;
        let y = arrival_of(&rec.t, &rec.gauges[k]);
        let mut xp = 1.0;
        for q in 0..5 {
            s[q] += xp;
            if q < 3 {
                ty[q] += xp * y;
            }
            xp *= x;
        }
    }
    let det = |m: [[f64; 3]; 3]| {
        m[0][0] * (m[1][1] * m[2][2] - m[1][2] * m[2][1]) - m[0][1] * (m[1][0] * m[2][2] - m[1][2] * m[2][0])
            + m[0][2] * (m[1][0] * m[2][1] - m[1][1] * m[2][0])
    };
    let matrix = [[s[0], s[1], s[2]], [s[1], s[2], s[3]], [s[2], s[3], s[4]]];
    let mut with_y = matrix;
    for row in 0..3 {
        with_y[row][1] = ty[row];
    }
    let slowness = det(with_y) / det(matrix);
    let mach = 1.0 / slowness;
    let peak = 2.0 * GAMMA / (GAMMA + 1.0) * (mach * mach - 1.0);
    let arrival = arrival_of(&rec.t, &rec.point);
    let n = rec.point.len();
    let mut top = 0;
    for k in 0..n {
        if rec.point[k] > rec.point[top] {
            top = k;
        }
    }
    let mut zero = rec.t[n - 1];
    let mut front_at_end = rec.front[n - 1];
    for k in top..n - 1 {
        if rec.point[k] > 0.0 && rec.point[k + 1] <= 0.0 {
            let x = rec.point[k] / (rec.point[k] - rec.point[k + 1]);
            zero = rec.t[k] + x * (rec.t[k + 1] - rec.t[k]);
            front_at_end = rec.front[k] + x * (rec.front[k + 1] - rec.front[k]);
            break;
        }
    }
    // the history from the arrival to the zero crossing, and its integral
    let at = |k: usize, time: f64| rec.point[k] + (rec.point[k + 1] - rec.point[k]) * (time - rec.t[k]) / (rec.t[k + 1] - rec.t[k]);
    let mut history = Vec::new();
    let mut impulse = 0.0;
    for k in 0..n - 1 {
        let lo = rec.t[k].max(arrival);
        let hi = rec.t[k + 1].min(zero);
        if hi <= lo {
            continue;
        }
        if history.is_empty() {
            history.push((lo - arrival, at(k, lo)));
        }
        let end = if hi >= zero { 0.0 } else { at(k, hi) };
        impulse += 0.5 * (at(k, lo) + end) * (hi - lo);
        history.push((hi - arrival, end));
    }
    let wave = Wave { arrival, peak, mach, duration: zero - arrival, impulse, history, front_at_end };
    if !(wave.peak.is_finite() && wave.mach.is_finite() && wave.duration.is_finite() && wave.impulse.is_finite() && wave.front_at_end.is_finite() && wave.peak > 0.0 && wave.duration > 0.0 && wave.impulse > 0.0) {
        return Err(BlastReject::NonFiniteValue);
    }
    Ok(wave)
}

// -------------------------------------------------- the Friedlander form

/// The integral of the Friedlander form over its positive phase per Ps td:
/// 1/a - (1 - exp(-a))/a^2, which is the sum of (-a)^n/(n + 2)! over n; the
/// series is summed where |a| < 1/2, where the closed form loses digits.
pub fn friedlander_fraction(a: f64) -> f64 {
    if a.abs() < 0.5 {
        let mut term = 0.5;
        let mut sum = 0.5;
        for n in 1..=24 {
            term *= -a / (n as f64 + 2.0);
            sum += term;
        }
        sum
    } else {
        1.0 / a - (-math::exp_m1(-a)) / (a * a)
    }
}

/// The decay coefficient a whose Friedlander integral per Ps td is
/// `fraction`, by bisection to adjacent 64-bit numbers; the fraction falls
/// from above 1/2 as a falls below zero, through 1/2 at zero, to zero as a
/// rises. None for a fraction that is not a positive finite number or that
/// needs a below -700.
pub fn friedlander_decay(fraction: f64) -> Option<f64> {
    if !(fraction.is_finite() && fraction > 0.0) {
        return None;
    }
    let (mut lo, mut hi) = if fraction < 0.5 {
        (0.0, 1.0 / fraction)
    } else {
        let mut lo = -1.0;
        while friedlander_fraction(lo) < fraction {
            lo *= 2.0;
            if lo < -700.0 {
                return None;
            }
        }
        (lo, 0.0)
    };
    for _ in 0..2000 {
        let mid = 0.5 * (lo + hi);
        if !(mid > lo && mid < hi) {
            break;
        }
        if friedlander_fraction(mid) > fraction { lo = mid } else { hi = mid }
    }
    let error = |a: f64| (friedlander_fraction(a) - fraction).abs();
    Some(if error(lo) <= error(hi) { lo } else { hi })
}

// ------------------------------------------------- weak-shock theory

/// The positive phase carried from Sachs distance `from` to `to` by
/// weak-shock theory, starting from the Friedlander form of `wave`.
/// Returns the wave there and the change of r times the impulse.
fn carried(wave: &Wave, decay: f64, from: f64, to: f64, fault: Fault) -> (Wave, f64) {
    // r p' over retarded time from the shock, sharp at the front
    let mut tau = Vec::with_capacity(WEAK_SHOCK_POINTS + 1);
    let mut level = Vec::with_capacity(WEAK_SHOCK_POINTS + 1);
    for k in 0..=WEAK_SHOCK_POINTS {
        let x = k as f64 / WEAK_SHOCK_POINTS as f64;
        tau.push(x * wave.duration);
        level.push(if k == WEAK_SHOCK_POINTS { 0.0 } else { from * wave.peak * (1.0 - x) * math::exp(-decay * x) });
    }
    let mut area_before = 0.0;
    for k in 0..WEAK_SHOCK_POINTS {
        area_before += 0.5 * (level[k] + level[k + 1]) * (tau[k + 1] - tau[k]);
    }
    let kappa = (GAMMA + 1.0) / (2.0 * RHO0) * math::log(to / from);
    let moved: Vec<f64> = tau.iter().zip(&level).map(|(t, f)| t - kappa * f).collect();
    // signed area along the distorted curve: the jump at the front, then the decay
    let mut area = -0.5 * kappa * level[0] * level[0];
    let mut shock_tau = moved[0];
    let mut shock_level = level[0];
    let mut start = 1;
    if area < 0.0 {
        for k in 0..WEAK_SHOCK_POINTS {
            let piece = 0.5 * (level[k] + level[k + 1]) * (moved[k + 1] - moved[k]);
            if area + piece >= 0.0 {
                // within this segment, level is linear in moved time
                let (t1, t2, f1, f2) = (moved[k], moved[k + 1], level[k], level[k + 1]);
                let half_slope = 0.5 * (f2 - f1) / (t2 - t1);
                let x = if half_slope.abs() * (t2 - t1) < 1.0e-12 * f1.abs() {
                    -area / f1
                } else {
                    (-f1 + (f1 * f1 - 4.0 * half_slope * area).sqrt()) / (2.0 * half_slope)
                };
                shock_tau = t1 + x;
                shock_level = f1 + 2.0 * half_slope * x;
                start = k + 1;
                break;
            }
            area += piece;
        }
    }
    let mut profile = vec![(shock_tau, shock_level)];
    for k in start..=WEAK_SHOCK_POINTS {
        let lowered = if fault == Fault::LostArea { 1.0 - 1.0e-9 } else { 1.0 };
        profile.push((moved[k], level[k] * lowered));
    }
    let mut area_after = 0.0;
    for k in 0..profile.len() - 1 {
        area_after += 0.5 * (profile[k].1 + profile[k + 1].1) * (profile[k + 1].0 - profile[k].0);
    }
    let end = profile[profile.len() - 1].0;
    let history = profile.iter().map(|&(t, f)| (t - shock_tau, f / to)).collect();
    let arrival = wave.arrival - from + shock_tau + to;
    let peak = shock_level / to;
    let mach = (1.0 + (GAMMA + 1.0) / (2.0 * GAMMA) * peak).sqrt();
    (
        Wave { arrival, peak, mach, duration: end - shock_tau, impulse: area_after / to, history, front_at_end: f64::NAN },
        ((area_after - area_before) / area_before).abs(),
    )
}

/// Where the weak shock carried from `from` is when the positive phase at
/// `far` (carried to `to`) ends: the radius whose arrival is that time, by
/// bisection to adjacent 64-bit numbers. The shock is supersonic and slows
/// toward c0, so its arrival rises with the radius.
fn reach_at_end(wave: &Wave, decay: f64, from: f64, to: f64, far: &Wave) -> f64 {
    let end = far.arrival + far.duration;
    let arrival = |r: f64| carried(wave, decay, from, r, Fault::None).0.arrival;
    let (mut lo, mut hi) = (to, to + 2.0 * far.mach * far.duration);
    while arrival(hi) < end {
        lo = hi;
        hi = to + 2.0 * (hi - to);
    }
    for _ in 0..200 {
        let mid = 0.5 * (lo + hi);
        if !(mid > lo && mid < hi) {
            break;
        }
        if arrival(mid) < end { lo = mid } else { hi = mid }
    }
    0.5 * (lo + hi)
}

// ------------------------------------------------------------ solve

/// Every limit the question itself shows, in the order the solver asks them.
pub fn admit(question: &BlastQuestion) -> Result<(), BlastReject> {
    let BlastQuestion { charge_mass_kg: w, distance_m: r, burst } = *question;
    if !(w.is_finite() && r.is_finite()) {
        return Err(BlastReject::InputNotFinite);
    }
    if !(w > 0.0) {
        return Err(BlastReject::ChargeMassNotPositive);
    }
    if !(r > 0.0) {
        return Err(BlastReject::DistanceNotPositive);
    }
    // The uniform air, at the charge's most favourable point.
    if !(w * burst.energy_factor() <= charge_mass_max_kg()) {
        return Err(BlastReject::ChargeBeyondUniformAtmosphere);
    }
    // At a tenth of the scale height itself the shock is past the point
    // before the phase there ends: the bound is exclusive.
    if !(r < distance_max_m()) {
        return Err(BlastReject::DistanceBeyondUniformAtmosphere);
    }
    if !(r >= distance_min_m()) {
        return Err(BlastReject::DistanceBelowContinuum);
    }
    let length = math::cbrt(w * burst.energy_factor() * TNT_ENERGY_J_PER_KG / AMBIENT_PRESSURE_PA);
    if !(r / length >= near_field_sachs_distance()) {
        return Err(BlastReject::ChargeProductsNearField);
    }
    Ok(())
}

/// The wave at Sachs distance `point` on one grid: Euler to the point, or to
/// `WEAK_SHOCK_FROM_SACHS_DISTANCE` and weak-shock theory beyond.
fn wave_on(sedov: &SedovTaylor, grid: &Grid, point: f64, fault: Fault) -> Result<(Wave, f64, f64, usize), BlastReject> {
    let reach = point.min(WEAK_SHOCK_FROM_SACHS_DISTANCE);
    let rec = run(sedov, grid, reach, fault)?;
    let wave = wave_of(&rec, reach)?;
    if point <= reach {
        return Ok((wave, 0.0, rec.worst_energy, rec.steps));
    }
    let decay = friedlander_decay(wave.impulse / (wave.peak * wave.duration)).ok_or(BlastReject::ImpulseClosureBroken)?;
    let (mut far, area) = carried(&wave, decay, reach, point, fault);
    if !(far.peak.is_finite() && far.duration.is_finite() && far.impulse.is_finite() && area.is_finite()) {
        return Err(BlastReject::NonFiniteValue);
    }
    if area > AREA_TOLERANCE {
        return Err(BlastReject::AreaNotCarried);
    }
    far.front_at_end = reach_at_end(&wave, decay, reach, point, &far);
    if !far.front_at_end.is_finite() {
        return Err(BlastReject::NonFiniteValue);
    }
    Ok((far, area, rec.worst_energy, rec.steps))
}

fn grids(point: f64, coarse_factor: f64) -> (Grid, Grid) {
    let reach = point.min(WEAK_SHOCK_FROM_SACHS_DISTANCE);
    let fine = FINE_CELL_SACHS.min(reach / FINE_CELLS_ACROSS_DISTANCE);
    let uniform_from = UNIFORM_FROM_FRACTION * reach;
    (
        Grid { cell: fine * coarse_factor, per_radius: FINE_CELLS_PER_RADIUS / coarse_factor, uniform_from },
        Grid { cell: fine, per_radius: FINE_CELLS_PER_RADIUS, uniform_from },
    )
}

/// The blast wave at the point, or the limit that stopped it.
///
/// Each value of the answer below the normal range is written as zero
/// with its sign ([`crate::normal`]).
pub fn solve(question: &BlastQuestion) -> Result<BlastWave, BlastReject> {
    crate::normal::held(solve_as_computed(question))
}

/// The answer as the equations compute it, before a value below the
/// normal range is written as zero.
fn solve_as_computed(question: &BlastQuestion) -> Result<BlastWave, BlastReject> {
    solve_with(question, Fault::None, COARSE_FACTOR)
}

impl NormalOrZero for BlastWave {
    fn normal_or_zero(&mut self) {
        self.scaled_distance_m_per_kg13.normal_or_zero();
        self.sachs_scaled_distance.normal_or_zero();
        self.charge_energy_j.normal_or_zero();
        self.arrival_time_s.normal_or_zero();
        self.peak_overpressure_pa.normal_or_zero();
        self.shock_mach_number.normal_or_zero();
        self.positive_phase_duration_s.normal_or_zero();
        self.positive_impulse_pa_s.normal_or_zero();
        self.friedlander_decay.normal_or_zero();
        self.scaled_arrival_time_ms_per_kg13.normal_or_zero();
        self.scaled_positive_phase_duration_ms_per_kg13.normal_or_zero();
        self.scaled_positive_impulse_kpa_ms_per_kg13.normal_or_zero();
        self.sedov_taylor_overpressure_pa.normal_or_zero();
        self.euler_reach_sachs_distance.normal_or_zero();
        self.shock_radius_at_end_m.normal_or_zero();
        self.shock_rise_time_s.normal_or_zero();
        self.history_time_after_arrival_s.normal_or_zero();
        self.history_overpressure_pa.normal_or_zero();
        self.friedlander_overpressure_pa.normal_or_zero();
        self.energy_ledger_residual.normal_or_zero();
        self.impulse_closure_residual.normal_or_zero();
        self.weak_shock_area_residual.normal_or_zero();
        self.grid_change_peak.normal_or_zero();
        self.grid_change_impulse.normal_or_zero();
        self.grid_change_duration.normal_or_zero();
        self.grid_change_arrival.normal_or_zero();
    }
}

fn solve_with(question: &BlastQuestion, fault: Fault, coarse_factor: f64) -> Result<BlastWave, BlastReject> {
    admit(question)?;
    let BlastQuestion { charge_mass_kg: w, distance_m: r, burst } = *question;
    let energy = w * burst.energy_factor() * TNT_ENERGY_J_PER_KG;
    let length = math::cbrt(energy / AMBIENT_PRESSURE_PA);
    let time_unit = length / sound_speed_m_per_s();
    let point = r / length;
    let sedov = sedov_taylor(GAMMA, SEDOV_TABLE_POINTS);
    let (coarse_grid, fine_grid) = grids(point, coarse_factor);
    let (coarse, _, _, coarse_steps) = wave_on(&sedov, &coarse_grid, point, fault)?;
    let (fine, area, worst_energy, fine_steps) = wave_on(&sedov, &fine_grid, point, fault)?;
    let change = |a: f64, b: f64| ((b - a) / b).abs();
    let (dp, di, dd, da) = (
        change(coarse.peak, fine.peak),
        change(coarse.impulse, fine.impulse),
        change(coarse.duration, fine.duration),
        change(coarse.arrival, fine.arrival),
    );
    if !(dp <= SETTLE_PEAK && di <= SETTLE_IMPULSE && dd <= SETTLE_DURATION && da <= SETTLE_ARRIVAL) {
        return Err(BlastReject::NotSettled);
    }
    let fraction = fine.impulse / (fine.peak * fine.duration);
    let decay = friedlander_decay(fraction).ok_or(BlastReject::ImpulseClosureBroken)?;
    let closure = ((fine.peak * fine.duration * friedlander_fraction(decay) - fine.impulse) / fine.impulse).abs();
    if !(closure <= IMPULSE_CLOSURE_TOLERANCE) {
        return Err(BlastReject::ImpulseClosureBroken);
    }
    // The air the answer rests on: the shock's radius when the positive
    // phase at the point ends.
    let shock_radius_at_end_m = fine.front_at_end * length;
    if !(shock_radius_at_end_m <= DISTANCE_MAX_M) {
        return Err(BlastReject::WaveBeyondUniformAtmosphere);
    }
    // The shock's own thickness against the phase it heads.
    let rise_s = shock_rise_time_s(fine.peak * AMBIENT_PRESSURE_PA);
    if !(peak_smoothing(rise_s, fine.duration * time_unit, decay) <= SETTLE_PEAK) {
        return Err(BlastReject::ShockNotSharp);
    }
    // the history and the Friedlander form at HISTORY_POINTS times
    let mut times = Vec::with_capacity(HISTORY_POINTS);
    let mut history = Vec::with_capacity(HISTORY_POINTS);
    let mut friedlander = Vec::with_capacity(HISTORY_POINTS);
    let mut j = 0;
    for k in 0..HISTORY_POINTS {
        let x = k as f64 / (HISTORY_POINTS - 1) as f64;
        let tau = x * fine.duration;
        while j + 2 < fine.history.len() && fine.history[j + 1].0 < tau {
            j += 1;
        }
        let (t1, p1) = fine.history[j];
        let (t2, p2) = fine.history[(j + 1).min(fine.history.len() - 1)];
        let value = if k == HISTORY_POINTS - 1 { 0.0 } else if t2 > t1 { p1 + (p2 - p1) * (tau - t1) / (t2 - t1) } else { p1 };
        times.push(tau * time_unit);
        history.push(value * AMBIENT_PRESSURE_PA);
        friedlander.push(fine.peak * (1.0 - x) * math::exp(-decay * x) * AMBIENT_PRESSURE_PA);
    }
    let cube_root_w = math::cbrt(w);
    let arrival_time_s = fine.arrival * time_unit;
    let duration_s = fine.duration * time_unit;
    let impulse_pa_s = fine.impulse * AMBIENT_PRESSURE_PA * time_unit;
    let sedov_peak = sedov_taylor_overpressure_pa(&sedov, energy, r);
    let answer = BlastWave {
        scaled_distance_m_per_kg13: r / cube_root_w,
        sachs_scaled_distance: point,
        charge_energy_j: energy,
        arrival_time_s,
        peak_overpressure_pa: fine.peak * AMBIENT_PRESSURE_PA,
        shock_mach_number: fine.mach,
        positive_phase_duration_s: duration_s,
        positive_impulse_pa_s: impulse_pa_s,
        friedlander_decay: decay,
        scaled_arrival_time_ms_per_kg13: arrival_time_s * 1.0e3 / cube_root_w,
        scaled_positive_phase_duration_ms_per_kg13: duration_s * 1.0e3 / cube_root_w,
        scaled_positive_impulse_kpa_ms_per_kg13: impulse_pa_s / cube_root_w,
        sedov_taylor_overpressure_pa: sedov_peak,
        euler_reach_sachs_distance: point.min(WEAK_SHOCK_FROM_SACHS_DISTANCE),
        shock_radius_at_end_m,
        shock_rise_time_s: rise_s,
        history_time_after_arrival_s: times,
        history_overpressure_pa: history,
        friedlander_overpressure_pa: friedlander,
        energy_ledger_residual: worst_energy,
        impulse_closure_residual: closure,
        weak_shock_area_residual: area,
        grid_change_peak: dp,
        grid_change_impulse: di,
        grid_change_duration: dd,
        grid_change_arrival: da,
        steps: (coarse_steps + fine_steps) as u64,
    };
    let finite = [
        answer.arrival_time_s,
        answer.peak_overpressure_pa,
        answer.positive_phase_duration_s,
        answer.positive_impulse_pa_s,
        answer.friedlander_decay,
        answer.sedov_taylor_overpressure_pa,
        answer.shock_radius_at_end_m,
        answer.shock_rise_time_s,
    ];
    if finite.iter().any(|v| !v.is_finite()) || answer.history_overpressure_pa.iter().chain(&answer.friedlander_overpressure_pa).any(|v| !v.is_finite()) {
        return Err(BlastReject::NonFiniteValue);
    }
    Ok(answer)
}

// ------------------------------------------------------ the references

/// The published reference the tests compare with: J. M. Dewey's TNT
/// free-field blast wave properties interface, built from particle-trajectory
/// analyses of TNT explosions of 4 kg to 500 kg (Dewey and McMillin 1987,
/// 1989); measurement-derived tables, not raw measurements.
pub const REFERENCE_NAME: &str = "J. M. Dewey (2021), TNT Free-Field Blast Wave Properties Interface (TNTFFI 1.1c), Borealis, CC0 1.0: peak overpressure and arrival time of 1 kg of TNT in free air at 100 radii from 1 to 14.95 m, and Friedlander coefficients from 1.31 to 14.94 m";
pub const REFERENCE_URL: &str = "https://doi.org/10.5683/SP2/WB8JKO";
pub const REFERENCE_RETRIEVED: &str = "2026-10-14";

#[cfg(test)]
mod tests {
    use super::*;

    fn free(w: f64, r: f64) -> BlastQuestion {
        BlastQuestion { charge_mass_kg: w, distance_m: r, burst: Burst::FreeAir }
    }

    fn surface(w: f64, r: f64) -> BlastQuestion {
        BlastQuestion { charge_mass_kg: w, distance_m: r, burst: Burst::Surface }
    }

    fn ratio(model: f64, reference: f64) -> f64 {
        model / reference - 1.0
    }

    /// Sedov's closed form, held to its own conservation of mass (the air
    /// inside the shock is the air it engulfed, 1/3 of rho0 R^3 per
    /// steradian) and to the published energy constant for gamma = 1.4,
    /// alpha = 0.851072 (Kamm and Timmes 2007, LA-UR-07-2849).
    #[test]
    fn the_sedov_taylor_profile_holds_its_mass_and_its_energy_constant() {
        let sedov = sedov_taylor(GAMMA, SEDOV_TABLE_POINTS);
        let mass = sedov.mass[sedov.mass.len() - 1];
        assert!((mass - 1.0 / 3.0).abs() < 1.0e-7, "{mass}");
        assert!((sedov.alpha - 0.851_072).abs() < 2.0e-6, "{}", sedov.alpha);
        assert!((sedov.xi0 - 1.032_777).abs() < 2.0e-6, "{}", sedov.xi0);
        // the jump conditions at the shock
        let last = sedov.xi.len() - 1;
        assert_eq!(sedov.xi[last], 1.0);
        assert!((sedov.density[last] - 6.0).abs() < 1.0e-12);
        assert!((sedov.velocity[last] - 2.0 / 2.4).abs() < 1.0e-12);
    }

    /// Where the shock is strong the Euler run is the Sedov-Taylor wave: at
    /// Sachs distance 0.12, where the jump is 90 p0, it is within 2 percent
    /// of 0.1567 E/R^3 (1.8 percent above it: p0, which Sedov-Taylor leaves
    /// out, and what remains of the start at 0.05) and the arrival within
    /// 0.6 percent of (rho0 R^5/(xi0^5 E))^(1/2).
    #[test]
    fn the_euler_run_is_the_sedov_taylor_wave_while_the_shock_is_strong() {
        let sedov = sedov_taylor(GAMMA, SEDOV_TABLE_POINTS);
        let point = 0.12;
        let grid = Grid { cell: 0.0002, per_radius: 40.0, uniform_from: 0.6 * point };
        let rec = run(&sedov, &grid, point, Fault::None).unwrap();
        let wave = wave_of(&rec, point).unwrap();
        let strong = 8.0 * sedov.xi0.powi(5) / (25.0 * (GAMMA + 1.0)) / (point * point * point);
        let arrival = (RHO0 * point.powi(5) / sedov.xi0.powi(5)).sqrt();
        assert!(ratio(wave.peak, strong).abs() < 0.02, "{} against {strong}", wave.peak);
        assert!(ratio(wave.arrival, arrival).abs() < 0.006, "{} against {arrival}", wave.arrival);
        assert!(rec.worst_energy < 1.0e-13, "{}", rec.worst_energy);
    }

    #[test]
    fn the_worked_question_settles_with_its_ledgers_closed() {
        let wave = solve(&free(1.0, 5.0)).unwrap();
        assert!(wave.energy_ledger_residual < 1.0e-13, "{}", wave.energy_ledger_residual);
        assert!(wave.impulse_closure_residual < 1.0e-14, "{}", wave.impulse_closure_residual);
        assert_eq!(wave.weak_shock_area_residual, 0.0);
        assert!(wave.grid_change_peak <= SETTLE_PEAK && wave.grid_change_impulse <= SETTLE_IMPULSE);
        assert_eq!(wave.history_overpressure_pa.len(), HISTORY_POINTS);
        assert_eq!(wave.history_overpressure_pa[HISTORY_POINTS - 1], 0.0);
        // the Friedlander form starts at the peak and closes on the impulse
        assert_eq!(wave.friedlander_overpressure_pa[0], wave.peak_overpressure_pa);
        let a = wave.friedlander_decay;
        let integral = wave.peak_overpressure_pa * wave.positive_phase_duration_s * (1.0 / a - (1.0 - (-a).exp()) / (a * a));
        assert!(ratio(integral, wave.positive_impulse_pa_s).abs() < 1.0e-13);
        // Rankine-Hugoniot between the Mach number and the peak
        let m = wave.shock_mach_number;
        assert!(ratio(wave.peak_overpressure_pa, AMBIENT_PRESSURE_PA * 2.0 * GAMMA / (GAMMA + 1.0) * (m * m - 1.0)).abs() < 1.0e-12);
    }

    /// Rule 8: heat put into one cell at every step and booked nowhere is named.
    #[test]
    fn unbooked_heat_breaks_the_energy_ledger_by_name() {
        assert_eq!(solve_with(&free(1.0, 3.0), Fault::UnbookedHeat, COARSE_FACTOR), Err(BlastReject::EnergyLedgerBroken));
    }

    /// Rule 8: a weak-shock regime that loses area is named.
    #[test]
    fn a_weak_shock_regime_that_loses_area_is_named() {
        assert_eq!(solve_with(&free(1.0, 20.0), Fault::LostArea, COARSE_FACTOR), Err(BlastReject::AreaNotCarried));
        let wave = solve(&free(1.0, 20.0)).unwrap();
        assert!(wave.weak_shock_area_residual < 1.0e-13, "{}", wave.weak_shock_area_residual);
        assert_eq!(wave.euler_reach_sachs_distance, WEAK_SHOCK_FROM_SACHS_DISTANCE);
    }

    /// Rule 2: a coarse grid twelve times the fine one does not agree with
    /// it, and the answer is refused, not returned.
    #[test]
    fn grids_that_disagree_are_named_not_settled() {
        assert_eq!(solve_with(&free(1.0, 1.5), Fault::None, 12.0), Err(BlastReject::NotSettled));
    }

    #[test]
    fn each_limit_is_named() {
        assert_eq!(solve(&free(0.0, 5.0)), Err(BlastReject::ChargeMassNotPositive));
        assert_eq!(solve(&free(-1.0, 5.0)), Err(BlastReject::ChargeMassNotPositive));
        assert_eq!(solve(&free(1.0, 0.0)), Err(BlastReject::DistanceNotPositive));
        assert_eq!(solve(&free(f64::NAN, 5.0)), Err(BlastReject::InputNotFinite));
        assert_eq!(solve(&free(1.0, f64::INFINITY)), Err(BlastReject::InputNotFinite));
        // the continuum: 100 mean free paths of the air
        let nearest = distance_min_m();
        assert_eq!(nearest, mean_free_path_m() / CONTINUUM_KNUDSEN_MAX);
        assert!((mean_free_path_m() - 6.366e-8).abs() < 1.0e-11, "{nearest}");
        assert_eq!(admit(&free(1.0e-18, nearest)), Ok(()));
        assert_eq!(admit(&free(1.0e-18, f64::from_bits(nearest.to_bits() - 1))), Err(BlastReject::DistanceBelowContinuum));
        assert_eq!(admit(&surface(3.0e-308, 1.0e-102)), Err(BlastReject::DistanceBelowContinuum));
        let farthest = distance_max_m();
        assert!((farthest - 843.451).abs() < 0.001, "{farthest}");
        assert_eq!(admit(&free(1.0, f64::from_bits(farthest.to_bits() - 1))), Ok(()));
        assert_eq!(admit(&free(1.0, farthest)), Err(BlastReject::DistanceBeyondUniformAtmosphere));
        // the near field: just inside is refused, just outside is answered
        let z_free = near_field_scaled_distance(Burst::FreeAir);
        let z_ground = near_field_scaled_distance(Burst::Surface);
        assert!((z_free - 0.5798).abs() < 1.0e-4 && (z_ground - 0.7305).abs() < 1.0e-4, "{z_free} {z_ground}");
        assert_eq!(admit(&free(1.0, 0.999_999 * z_free)), Err(BlastReject::ChargeProductsNearField));
        assert_eq!(admit(&surface(1.0, 0.999_999 * z_ground)), Err(BlastReject::ChargeProductsNearField));
        assert!(solve(&free(1.0, 1.000_001 * z_free)).is_ok());
        assert!(solve(&surface(1.0, 1.000_001 * z_ground)).is_ok());
    }

    /// The near field, on a shot that was measured there: Clarke et al.
    /// (2020), Proc. R. Soc. A 476: 20190791 (CC BY 4.0), Series A, 78 g of
    /// PE4 (taken by the authors as 1.2 TNT, 93.6 g), a 3:1 cylinder 19 mm
    /// high, 168 mm from its top to the target: 177.5 mm from its centre,
    /// Z = 0.39 m/kg^(1/3), refused.
    #[test]
    fn the_clarke_series_a_near_field_shot_is_refused() {
        let question = free(0.078 * 1.2, 0.168 + 0.0095);
        assert!(question.distance_m / question.charge_mass_kg.cbrt() < 0.40);
        assert_eq!(solve(&question), Err(BlastReject::ChargeProductsNearField));
    }

    /// Hopkinson-Cranz scaling is exact in the model: a thousand times the
    /// charge at ten times the distance gives the same overpressure, and ten
    /// times the times and the impulse.
    #[test]
    fn a_thousand_times_the_charge_at_ten_times_the_distance_is_the_same_wave_scaled() {
        let small = solve(&free(1.0, 3.0)).unwrap();
        let large = solve(&free(1000.0, 30.0)).unwrap();
        assert!(ratio(large.peak_overpressure_pa, small.peak_overpressure_pa).abs() < 1.0e-9);
        assert!(ratio(large.arrival_time_s, 10.0 * small.arrival_time_s).abs() < 1.0e-9);
        assert!(ratio(large.positive_impulse_pa_s, 10.0 * small.positive_impulse_pa_s).abs() < 1.0e-9);
        assert!(ratio(large.scaled_positive_impulse_kpa_ms_per_kg13, small.scaled_positive_impulse_kpa_ms_per_kg13).abs() < 1.0e-9);
    }

    /// On rigid ground the burst is half of a free-air burst of twice the charge.
    #[test]
    fn a_surface_burst_is_a_free_air_burst_of_twice_the_charge() {
        let ground = solve(&surface(10.0, 12.0)).unwrap();
        let air = solve(&free(20.0, 12.0)).unwrap();
        assert_eq!(ground.peak_overpressure_pa, air.peak_overpressure_pa);
        assert_eq!(ground.positive_impulse_pa_s, air.positive_impulse_pa_s);
        assert_eq!(ground.arrival_time_s, air.arrival_time_s);
        assert!(ratio(ground.scaled_distance_m_per_kg13, air.scaled_distance_m_per_kg13 * 2.0f64.cbrt()).abs() < 1.0e-15);
    }

    /// The sentence above each variant is the one `meaning` serves.
    #[test]
    fn every_meaning_is_the_sentence_above_its_variant() {
        let source = include_str!("blast_wave_tnt_air.rs");
        let start = source.find("pub enum BlastReject {").unwrap();
        let mut doc = String::new();
        let mut seen = 0;
        for line in source[start..].lines().skip(1) {
            if line == "}" {
                break;
            }
            let text = line.trim_start();
            if let Some(sentence) = text.strip_prefix("/// ") {
                doc.push_str(sentence);
            } else if let Some(name) = text.strip_suffix(',') {
                let reject = ALL_REJECTS.iter().find(|r| r.name() == name).unwrap_or_else(|| panic!("{name}"));
                assert_eq!(reject.meaning(), doc, "{name}");
                doc.clear();
                seen += 1;
            }
        }
        assert_eq!(seen, ALL_REJECTS.len());
        assert_eq!(BlastReject::NotSettled.to_string(), "the coarse and fine grids disagree by more than 2.5 percent in the peak, 1 percent in the impulse, 1.5 percent in the duration or 0.5 percent in the arrival time");
    }

    #[test]
    fn the_same_question_gives_the_same_bits() {
        assert_eq!(solve(&free(2.0, 7.0)), solve(&free(2.0, 7.0)));
    }

    #[test]
    fn the_friedlander_decay_inverts_its_integral() {
        for &a in &[-3.0, -0.5, -1.0e-4, 0.0, 1.0e-5, 0.3, 1.0, 5.0, 40.0, 300.0] {
            let fraction = friedlander_fraction(a);
            let back = friedlander_decay(fraction).unwrap();
            assert!(((friedlander_fraction(back) - fraction) / fraction).abs() < 1.0e-14, "{a} {back}");
        }
        assert_eq!(friedlander_fraction(0.0), 0.5);
        assert_eq!(friedlander_decay(0.0), None);
        assert_eq!(friedlander_decay(f64::NAN), None);
    }

    /// The held-back comparison as it was scored on 2026-10-14
    /// (`evidence/research/blast_overpressure/analysis/out/scored.json`): five
    /// of the 24 held-back traces, each answer held to the scored error within
    /// 0.1 percentage point. The measured values are Dewey and Sochet's (2020,
    /// CC BY 4.0): the largest value of the record (atm), its integral from
    /// the arrival to its first zero crossing (atm ms; none where the record
    /// ends before it), and the time of arrival after ignition (ms).
    #[test]
    fn dewey_sochet_propane_oxygen_held_back_comparison() {
        let atm = AMBIENT_PRESSURE_PA;
        // mixture TNT equivalent (kg), radius (m), measured peak, impulse, arrival, scored errors
        let traces = [
            (0.000_857_646_372_538_848_9, 0.255, 0.7225, Some(0.045_286_022_603_007_424), 0.372, (0.7153, Some(0.7762), -0.2542)),
            (0.000_857_646_372_538_848_9, 0.592, 0.1788, None, 1.217, (0.5470, None, -0.0985)),
            (0.000_731_663_208_705_421_6, 0.255, 0.6763, Some(0.050_699_379_413_899_84), 0.379, (0.6465, Some(0.4438), -0.2337)),
            (0.000_731_663_208_705_421_6, 0.592, 0.18, Some(0.022_091_062_319_807_76), 1.235, (0.4199, Some(0.5433), -0.0919)),
            (0.000_731_663_208_705_421_6, 0.904, 0.1032, Some(0.015_430_681_633_828_006), 2.081, (0.3268, Some(0.4570), -0.0517)),
        ];
        for (w, r, peak, impulse, arrival, (dp, di, da)) in traces {
            let wave = solve(&surface(w, r)).unwrap();
            let e_peak = ratio(wave.peak_overpressure_pa / atm, peak);
            let e_arrival = ratio(wave.arrival_time_s * 1.0e3, arrival);
            assert!((e_peak - dp).abs() < 1.0e-3, "{r}: {e_peak}");
            assert!((e_arrival - da).abs() < 1.0e-3, "{r}: {e_arrival}");
            // every one misses: the peak line is 15 percent, the arrival line 5
            assert!(e_peak > 0.15 && e_arrival < -0.05);
            if let (Some(impulse), Some(di)) = (impulse, di) {
                let e_impulse = ratio(wave.positive_impulse_pa_s * 1.0e3 / atm, impulse);
                assert!((e_impulse - di).abs() < 1.0e-3, "{r}: {e_impulse}");
                assert!(e_impulse > 0.20);
            }
        }
    }

    /// The comparison with the published reference (`REFERENCE_NAME`), as the
    /// header states it: 1 kg of TNT in free air, Dewey's TNTFFI 1.1c,
    /// overpressure and arrival interpolated in its 100-radius table, duration
    /// and impulse from its Friedlander coefficients. The point source of the
    /// charge's 4.184 MJ is below TNT: 22 percent in the peak at Z = 2,
    /// 11 at 5, 8 at 10; the impulse 5 to 9 percent below; the arrival 2 to
    /// 8 percent late. Retrieved 2026-10-14.
    #[test]
    fn dewey_tnt_free_field_comparison() {
        // Z, peak kPa, arrival ms, duration ms, impulse kPa ms (TNTFFI, 1 kg)
        let reference = [
            (2.0, 182.6425, 1.907667, 1.329653, 75.551427),
            (5.0, 31.056189, 8.950615, 2.783318, 34.591422),
            (10.0, 10.970094, 22.642857, 3.616525, 17.30983),
        ];
        let stated = [(-0.217, 0.078, 0.258, -0.053), (-0.111, 0.040, 0.045, -0.084), (-0.078, 0.020, 0.025, -0.076)];
        for ((z, p, ta, td, imp), (dp, da, dd, di)) in reference.iter().zip(stated) {
            let wave = solve(&free(1.0, *z)).unwrap();
            let found = (
                ratio(wave.peak_overpressure_pa / 1.0e3, *p),
                ratio(wave.arrival_time_s * 1.0e3, *ta),
                ratio(wave.positive_phase_duration_s * 1.0e3, *td),
                ratio(wave.positive_impulse_pa_s, *imp),
            );
            for (f, s) in [(found.0, dp), (found.1, da), (found.2, dd), (found.3, di)] {
                assert!((f - s).abs() < 0.01, "Z {z}: {found:?} against {stated:?}");
            }
        }
    }

    /// Kingery and Bulmash (1984), as simplified by Swisdak (1994, DTIC
    /// ADA526744) and coded by fcento100 (github.com/fcento100/kingery-bulmash,
    /// MIT, commit 194c3c7, retrieved 2026-10-14; not yet compared with the
    /// original report): the hemispherical surface burst of 1 kg of TNT.
    /// The model's surface burst is below it: the peak 16 percent at
    /// Z = 2, 7 at 5, 6 at 10, 11 at 20, 6 at 40; the impulse 15 to 20
    /// percent; the arrival within 2 percent from Z = 2 out.
    #[test]
    fn kingery_bulmash_surface_burst_comparison() {
        fn kb(z: f64, c: &[f64]) -> f64 {
            let l = z.ln();
            c.iter().enumerate().map(|(i, ci)| ci * l.powi(i as i32)).sum::<f64>().exp()
        }
        let peak = |z: f64| {
            if z <= 2.9 { kb(z, &[7.2106, -2.1069, -0.3229, 0.1117, 0.0685]) } else if z <= 23.8 { kb(z, &[7.5938, -3.0523, 0.40977, 0.0261, -0.01267]) } else { kb(z, &[6.0536, -1.4066]) }
        };
        let arrival = |z: f64| {
            if z <= 1.5 { kb(z, &[-0.7604, 1.8058, 0.1257, -0.0437, -0.0310, -0.00669]) } else { kb(z, &[-0.7137, 1.5732, 0.5561, -0.4213, 0.1054, -0.00929]) }
        };
        let impulse = |z: f64| {
            if z <= 2.38 { kb(z, &[5.465, -0.308, -1.464, 1.362, -0.432]) } else if z <= 33.7 { kb(z, &[5.2749, -0.4677, -0.2499, 0.0588, -0.00554]) } else { kb(z, &[5.9825, -1.062]) }
        };
        let stated = [(2.0, -0.163, -0.036, -0.205), (5.0, -0.074, 0.014, -0.161), (10.0, -0.065, 0.007, -0.183), (20.0, -0.110, 0.005, -0.200), (40.0, -0.055, 0.003, -0.194)];
        for (z, dp, da, di) in stated {
            let wave = solve(&surface(1.0, z)).unwrap();
            let found = (
                ratio(wave.peak_overpressure_pa / 1.0e3, peak(z)),
                ratio(wave.arrival_time_s * 1.0e3, arrival(z)),
                ratio(wave.positive_impulse_pa_s, impulse(z)),
            );
            for (f, s) in [(found.0, dp), (found.1, da), (found.2, di)] {
                assert!((f - s).abs() < 0.01, "Z {z}: {found:?}");
            }
        }
    }


    /// One criterion holds the air uniform: the shock's radius when the
    /// positive phase at the point ends, at most a tenth of a scale height.
    /// It measures the wave, not the charge: 16,000 tonnes at 300 m, whose
    /// shock is 482 m out by then, is answered; the charge whose Sachs length
    /// is 843.45 m, at 843 m, is refused. The largest charge answered
    /// anywhere is the one whose shock reaches 843.45 m as the phase ends
    /// where that radius is least, Sachs distance 0.1835, held here below its
    /// neighbours' from the near-field limit out.
    #[test]
    fn the_uniform_air_is_held_on_the_wave_at_the_end_of_its_positive_phase() {
        let big = solve(&free(1.6e7, 300.0)).unwrap();
        assert!((big.shock_radius_at_end_m - 482.0).abs() < 1.0, "{}", big.shock_radius_at_end_m);
        let old_cap = AMBIENT_PRESSURE_PA * DISTANCE_MAX_M * DISTANCE_MAX_M * DISTANCE_MAX_M / TNT_ENERGY_J_PER_KG;
        assert_eq!(solve(&free(old_cap, 843.0)), Err(BlastReject::WaveBeyondUniformAtmosphere));
        let sedov = sedov_taylor(GAMMA, SEDOV_TABLE_POINTS);
        let reach = |lambda: f64| {
            let (_, fine) = grids(lambda, COARSE_FACTOR);
            wave_on(&sedov, &fine, lambda, Fault::None).unwrap().0.front_at_end
        };
        assert_eq!(reach(LEAST_REACH_SACHS_DISTANCE), LEAST_SHOCK_REACH_SACHS);
        for lambda in [near_field_sachs_distance(), 0.175, 0.18, 0.183, 0.184, 0.19, 0.2, 0.25, 0.4, 1.0, 4.0] {
            assert!(reach(lambda) >= LEAST_SHOCK_REACH_SACHS, "{lambda}");
        }
        assert!((CHARGE_MASS_MAX_KG / 106342834.30636403 - 1.0).abs() < 1.0e-12, "{CHARGE_MASS_MAX_KG}");
        let at = |w: f64, lambda: f64, burst: Burst| {
            let length = (w * burst.energy_factor() * TNT_ENERGY_J_PER_KG / AMBIENT_PRESSURE_PA).cbrt();
            BlastQuestion { charge_mass_kg: w, distance_m: lambda * length, burst }
        };
        let inside = solve(&at(CHARGE_MASS_MAX_KG * (1.0 - 1.0e-4), LEAST_REACH_SACHS_DISTANCE, Burst::FreeAir)).unwrap();
        assert!(inside.shock_radius_at_end_m <= DISTANCE_MAX_M && inside.shock_radius_at_end_m > 0.9999 * DISTANCE_MAX_M, "{}", inside.shock_radius_at_end_m);
        assert!(solve(&at(0.5 * CHARGE_MASS_MAX_KG * (1.0 - 1.0e-4), LEAST_REACH_SACHS_DISTANCE, Burst::Surface)).is_ok());
        // Above it no point is answered, refused at the door; below it a
        // point farther out is past the uniform air by then, sealed.
        assert_eq!(admit(&at(CHARGE_MASS_MAX_KG, LEAST_REACH_SACHS_DISTANCE, Burst::FreeAir)), Ok(()));
        assert_eq!(admit(&at(CHARGE_MASS_MAX_KG.next_up(), LEAST_REACH_SACHS_DISTANCE, Burst::FreeAir)), Err(BlastReject::ChargeBeyondUniformAtmosphere));
        assert_eq!(admit(&at((0.5 * CHARGE_MASS_MAX_KG).next_up(), LEAST_REACH_SACHS_DISTANCE, Burst::Surface)), Err(BlastReject::ChargeBeyondUniformAtmosphere));
        assert_eq!(solve(&at(CHARGE_MASS_MAX_KG * (1.0 - 1.0e-4), 0.25, Burst::FreeAir)), Err(BlastReject::WaveBeyondUniformAtmosphere));
        // In the weak-shock range the shock has travelled on by a little more
        // than the sound speed times the duration.
        let far = solve(&free(1.0, 200.0)).unwrap();
        let sound = far.positive_phase_duration_s * sound_speed_m_per_s();
        assert!(far.shock_radius_at_end_m > 200.0 + sound && far.shock_radius_at_end_m < 200.0 + 1.01 * sound, "{}", far.shock_radius_at_end_m);
    }

    /// The thermoviscous rise of a weak shock: Taylor's profile joined to the
    /// decay behind it lowers the peak by (e/4)(1 + ln(4/e)) to first order,
    /// held here against the composite profile's own maximum.
    #[test]
    fn the_peak_smoothing_of_a_taylor_shock_is_its_composite_maximum() {
        assert!((sound_diffusivity_m2_per_s() - 2.7708e-5).abs() < 1.0e-8, "{}", sound_diffusivity_m2_per_s());
        for e in [1.0e-4, 1.0e-3, 1.0e-2, 0.02] {
            // p/Ps = 1/(1 + exp(-2 x)) - (e/2) x, x = tau/tau_s, the slope behind
            // the shock Ps (1 + a)/td = (e/2) Ps/tau_s
            let profile = |x: f64| 1.0 / (1.0 + (-2.0 * x).exp()) - 0.5 * e * x;
            let mut best = 0.0f64;
            for k in 0..200_000 {
                best = best.max(profile(k as f64 * 1.0e-4));
            }
            let stated = peak_smoothing(e, 2.0, 0.0);
            assert!(((1.0 - best) - stated).abs() < 2.0 * e * e + 1.0e-9, "{e}: {} against {stated}", 1.0 - best);
        }
        assert_eq!(peak_smoothing(5.0, 1.0, 0.0), 1.0);
    }

    /// Where the shock is not sharp the inviscid wave is refused by name: a
    /// microgram at 840 m, whose rise would be about 147 times its phase, and
    /// a gram; a kilogram there is answered, its rise 1.45e-4 of its phase.
    #[test]
    fn a_shock_that_is_not_sharp_is_refused() {
        assert_eq!(solve(&free(1.0e-9, 840.0)), Err(BlastReject::ShockNotSharp));
        assert_eq!(solve(&free(1.0e-3, 840.0)), Err(BlastReject::ShockNotSharp));
        let kilogram = solve(&free(1.0, 840.0)).unwrap();
        let fraction = kilogram.shock_rise_time_s / kilogram.positive_phase_duration_s;
        assert!((fraction - 1.45e-4).abs() < 0.01e-4, "{fraction}");
        assert!(peak_smoothing(kilogram.shock_rise_time_s, kilogram.positive_phase_duration_s, kilogram.friedlander_decay) < SETTLE_PEAK);
    }

}
