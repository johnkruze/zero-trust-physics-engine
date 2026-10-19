//! The blast wave of a real charge of TNT in still air at sea level: a
//! sphere of cast TNT in free air, or a hemisphere on the ground. The
//! successor of `blast_wave_tnt_air`, whose point source of the charge's
//! energy reads 6 to 33 percent under the TNT tables: here the charge is a
//! body of detonation products with its own equation of state, the wave it
//! drives into the air is carried by the same equations, and two constants
//! are fitted on J. M. Dewey's measurement-derived TNT tables.
//!
//! **The question.** A charge of `charge_mass_kg` of TNT, a point at
//! `distance_m` from its centre, and the burst: `FreeAir` (a sphere) or
//! `Surface` (a hemisphere resting on flat ground). The answer is the
//! incident (side-on) blast wave at the point: the arrival of the shock, its
//! peak overpressure, the duration and impulse of the positive phase, the
//! Friedlander form and the history, and how far the products reached.
//!
//! **The charge.** Cast TNT at 1,630 kg/m^3, detonated at one instant
//! through its volume (a constant-volume detonation): its products start at
//! rest at the charge's density with the detonation energy E0 of the
//! Jones-Wilkins-Lee form, times [`PRODUCTS_ENERGY_FACTOR`]. Their pressure
//! is the JWL equation of state of TNT (Dobratz and Crawford, LLNL
//! Explosives Handbook, UCRL-52997 Change 2, 1985):
//! p = A (1 - w/(R1 V)) exp(-R1 V) + B (1 - w/(R2 V)) exp(-R2 V) + w rho e,
//! V = rho0/rho, with A = 373.8 GPa, B = 3.747 GPa, R1 = 4.15, R2 = 0.90,
//! w = 0.35 and E0 = 6.00 GPa (3.681 MJ/kg); its Chapman-Jouguet state,
//! 21.0 GPa at 6,930 m/s, lies on it (a test). The products are a material
//! of their own, cells of fixed mass apart from the air's, and their face
//! with the air moves with the flow.
//!
//! **The air.** As in `blast_wave_tnt_air`: the International Standard
//! Atmosphere at sea level, still and uniform, an ideal gas of gamma 1.4.
//!
//! **The wave.** From the instant of detonation the spherical Euler
//! equations of products and air together carry the wave: Lagrangian finite
//! volumes, limited linear reconstruction, two-stage Runge-Kutta, a
//! two-shock acoustic Riemann solver at every node (the solver of
//! `blast_wave_tnt_air`, with each cell's own equation of state). Beyond
//! Sachs distance 4 of the free-air charge's 4.184 MJ/kg (Z = 13.8 in free
//! air) the positive phase is carried by weak-shock theory, as there. The
//! peak overpressure is the Rankine-Hugoniot jump of the shock's Mach number
//! in the air, from seven gauges within 5 percent of the point.
//!
//! **The ground.** A hemisphere on the ground is half of a sphere of twice
//! its mass in free air where the ground is rigid. Real ground gives: it
//! cracks, craters and carries a ground shock away. The twin takes a
//! surface burst of W as a free-air burst of [`GROUND_FACTOR`] W, the
//! reflection factor, fitted on Dewey's surface-burst peaks.
//!
//! **What is held.** At every step the energy of the products plus the
//! energy of the air above ambient equals the products' energy at the
//! instant of detonation, to rounding (`ENERGY_LEDGER_TOLERANCE`). The
//! Friedlander form's integral equals the impulse, and through the
//! weak-shock regime r times the impulse is carried unchanged. The whole
//! answer is computed on two grids, one twice as fine; it is returned only
//! when the two agree within `SETTLE_*`, and the fine grid's values are
//! returned.
//!
//! **Fitted.** Two constants, on 1 kg of TNT, Dewey's tables (CC0),
//! least squares in the log of the twin over the table
//! (evidence/research/blast_overpressure/twin/fit.py and fit_ground.py): the products' energy factor 1.234 (4.542 MJ/kg, near
//! TNT's measured heat of detonation) and the ground factor 1.825:
//! [`PRODUCTS_ENERGY_FACTOR`] on the free-air interface (TNTFFI 1.1c), its
//! peak overpressures from Z = 1 to 14.95 m/kg^(1/3) and its positive-phase
//! impulses and durations from 1.31 to 14.94; [`GROUND_FACTOR`] on the
//! surface-burst interface's peak overpressures (TNTSBI 1.0c) at the 16 Z
//! from 1.1 to 14.94 outside the fireball. Kept out of the fit and compared: the surface-burst interface's
//! positive-phase table (its impulses and durations from 1.31 to 14.94),
//! and Kingery and Bulmash's surface burst as Swisdak simplified it.
//!
//! **Limits.** A charge mass or a distance of zero or less has no blast.
//! The fireball: the products' face with the air runs out to its farthest
//! radius, [`FIREBALL_RADIUS_M_PER_KG13`] for each kg^(1/3) of the free-air
//! charge, and comes back; as it slows, the heavier products pushing the
//! lighter hot air, the face is Rayleigh-Taylor unstable and products and
//! air mix there (and the products' carbon monoxide and carbon burn with
//! the air's oxygen). A model in the radius alone carries a sharp face
//! between them, so a point within that radius is refused
//! (`PointInsideFireball`). The uniform air, the continuum and the sharp
//! shock are held as in `blast_wave_tnt_air`, by the same names.

use crate::blast_wave_tnt_air::{
    self as base, Burst, Fault, Record, Wave, AMBIENT_PRESSURE_PA, DISTANCE_MAX_M, GAMMA, P0, RHO0, TNT_ENERGY_J_PER_KG,
};
use crate::normal::NormalOrZero;
use genesis_kernel::math;
use std::fmt;

// ------------------------------------------------------------ the charge

/// Density of cast TNT, kg/m^3 (Dobratz and Crawford 1985).
pub const TNT_DENSITY_KG_PER_M3: f64 = 1630.0;
/// JWL A of TNT, Pa (Dobratz and Crawford 1985).
pub const JWL_A_PA: f64 = 373.8e9;
/// JWL B of TNT, Pa.
pub const JWL_B_PA: f64 = 3.747e9;
/// JWL R1 of TNT.
pub const JWL_R1: f64 = 4.15;
/// JWL R2 of TNT.
pub const JWL_R2: f64 = 0.90;
/// JWL omega of TNT, the products' Grueneisen coefficient at large expansion.
pub const JWL_OMEGA: f64 = 0.35;
/// JWL E0 of TNT, the detonation energy per unit volume of the charge, J/m^3.
pub const JWL_E0_J_PER_M3: f64 = 6.00e9;
/// Detonation velocity of TNT at 1,630 kg/m^3, m/s.
pub const TNT_DETONATION_VELOCITY_M_PER_S: f64 = 6930.0;
/// Chapman-Jouguet pressure of TNT at 1,630 kg/m^3, Pa.
pub const TNT_CJ_PRESSURE_PA: f64 = 21.0e9;

// --------------------------------------------------- the fitted constants

/// The products' starting energy over the JWL E0, fitted on Dewey's
/// free-air table.
pub const PRODUCTS_ENERGY_FACTOR: f64 = 1.234;
/// A surface burst of W is a free-air burst of this many W (2 on rigid
/// ground), fitted on Dewey's surface-burst peaks.
pub const GROUND_FACTOR: f64 = 1.825;

/// The twin's constants: the fitted pair, served at [`CALIBRATION`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Calibration {
    pub products_energy_factor: f64,
    pub ground_factor: f64,
}

/// The served constants.
pub const CALIBRATION: Calibration = Calibration { products_energy_factor: PRODUCTS_ENERGY_FACTOR, ground_factor: GROUND_FACTOR };

// --------------------------------------------------------- the numerics

/// Beyond this Sachs distance of the free-air charge's 4.184 MJ/kg the wave is carried by weak-shock theory.
pub const WEAK_SHOCK_FROM_SACHS_DISTANCE: f64 = base::WEAK_SHOCK_FROM_SACHS_DISTANCE;
/// The fine grid's air cell, in Sachs lengths, at most.
pub const FINE_CELL_SACHS: f64 = 0.001_25;
/// The fine grid has at least this many cells across the distance of the point.
pub const FINE_CELLS_ACROSS_DISTANCE: f64 = 800.0;
/// Cells across the charge's radius on the fine grid, at least.
pub const FINE_PRODUCTS_CELLS: usize = 64;
/// Near the charge an air cell is at most its radius over this many (fine grid).
pub const FINE_CELLS_PER_RADIUS: f64 = 40.0;
/// The coarse grid's cells are this many times the fine grid's.
pub const COARSE_FACTOR: f64 = 2.0;
/// Courant number of the step.
pub const COURANT_NUMBER: f64 = 0.4;
/// The seven gauges for the shock's speed span this fraction of the distance either side of the point.
pub const SPEED_GAUGE_SPREAD: f64 = 0.05;
/// Cells are joined only inside this fraction of the innermost gauge's radius.
pub const MERGE_INSIDE_GAUGE: f64 = 0.9;
/// Points of the history and the Friedlander form returned.
pub const HISTORY_POINTS: usize = 33;
/// The most steps of one Euler run.
pub const MAX_STEPS: usize = 400_000;
/// Largest departure of the energy of products and air above ambient from the products' energy at detonation, as a fraction of it, at any step.
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

// -------------------------------------------------------- the question

/// What the caller asks.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TwinQuestion {
    /// Mass of the charge, kg of TNT.
    pub charge_mass_kg: f64,
    /// Distance of the point from the charge's centre, m.
    pub distance_m: f64,
    pub burst: Burst,
}

/// The blast wave at the point.
#[derive(Clone, Debug, PartialEq)]
pub struct TwinWave {
    /// Z = R/W^(1/3), m/kg^(1/3).
    pub scaled_distance_m_per_kg13: f64,
    /// The free-air charge the burst is taken as, kg: W in free air, the ground factor times W on the ground.
    pub free_air_charge_kg: f64,
    /// The radius of that free-air sphere of TNT, m.
    pub charge_radius_m: f64,
    pub arrival_time_s: f64,
    pub peak_overpressure_pa: f64,
    pub shock_mach_number: f64,
    pub positive_phase_duration_s: f64,
    pub positive_impulse_pa_s: f64,
    /// a of p = Ps (1 - tau/td) exp(-a tau/td).
    pub friedlander_decay: f64,
    pub scaled_arrival_time_ms_per_kg13: f64,
    pub scaled_positive_phase_duration_ms_per_kg13: f64,
    pub scaled_positive_impulse_kpa_ms_per_kg13: f64,
    /// The farthest the face of the products and the air reached in the run, m.
    pub products_radius_max_m: f64,
    /// The farthest scaled distance the Euler equations carried the wave, m/kg^(1/3); beyond it weak-shock theory.
    pub euler_reach_m_per_kg13: f64,
    /// How far from the charge the shock is when the positive phase at the point ends, m.
    pub shock_radius_at_end_m: f64,
    /// The thermoviscous rise time of the shock at the point, s.
    pub shock_rise_time_s: f64,
    /// 1 where Z is outside the range compared with Dewey's tables, 1 to 14.95 m/kg^(1/3), 0 inside it.
    pub scaled_distance_outside_compared: bool,
    pub history_time_after_arrival_s: Vec<f64>,
    pub history_overpressure_pa: Vec<f64>,
    pub friedlander_overpressure_pa: Vec<f64>,
    /// Largest departure of the energy of products and air above ambient from the products' energy at detonation, over the fine grid's run, as a fraction.
    pub energy_ledger_residual: f64,
    pub impulse_closure_residual: f64,
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
pub enum TwinReject {
    /// An input is not a finite number.
    InputNotFinite,
    /// The charge mass is zero or less: there is no charge.
    ChargeMassNotPositive,
    /// The distance is zero or less.
    DistanceNotPositive,
    /// The charge is above 194,741,349 kg of TNT in free air, or 106,707,589 kg on the ground: at every point of it the shock is beyond a tenth of the air's scale height, 843.45 m, when the positive phase there ends, that radius being least, 0.421 Sachs lengths, at the fireball's edge.
    ChargeBeyondUniformAtmosphere,
    /// The distance is a tenth of the air's scale height, 843.45 m, or beyond, where the ambient pressure differs from the charge's by 1 - e^-0.1 = 9.52 percent or more: the shock is past it before the positive phase at the point ends.
    DistanceBeyondUniformAtmosphere,
    /// The distance is under 100 mean free paths of the air, 6.37e-6 m: the mean free path over the distance, the Knudsen number, passes 0.01, the edge of the continuum, where the Euler equations of a gas stop holding.
    DistanceBelowContinuum,
    /// The point is within the farthest reach of the detonation products, 0.8402 m/kg^(1/3) of the free-air charge: there products and air mix, their face being unstable as it slows, and the sharp face the model carries between them is not the real one.
    PointInsideFireball,
    /// When the positive phase at the point ends, the shock is beyond a tenth of the air's scale height, 843.45 m, from the charge: the air the answer rests on reaches where the ambient pressure differs from the charge's by more than 1 - e^-0.1 = 9.52 percent, and the model's air is uniform.
    WaveBeyondUniformAtmosphere,
    /// The shock's thermoviscous rise time, Taylor's 2 rho0 delta/(beta Ps), lowers the peak of the positive phase it heads by more than 2.5 percent, the peak's own settling line: the shock is not sharp against its phase, and the inviscid wave the model carries is not the air's.
    ShockNotSharp,
    /// A value of the flow was not finite.
    NonFiniteValue,
    /// A cell's internal energy fell to zero or below.
    InternalEnergyNotPositive,
    /// A cell of the products has no real sound speed in the JWL equation of state.
    ProductsSoundSpeedNotReal,
    /// A cell's outer face crossed its inner face.
    CellInverted,
    /// The energy of products and air above ambient departed from the products' energy at detonation by more than 1e-10 of it at a step.
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
pub const ALL_REJECTS: [TwinReject; 18] = [
    TwinReject::InputNotFinite,
    TwinReject::ChargeMassNotPositive,
    TwinReject::DistanceNotPositive,
    TwinReject::ChargeBeyondUniformAtmosphere,
    TwinReject::DistanceBeyondUniformAtmosphere,
    TwinReject::DistanceBelowContinuum,
    TwinReject::PointInsideFireball,
    TwinReject::WaveBeyondUniformAtmosphere,
    TwinReject::ShockNotSharp,
    TwinReject::NonFiniteValue,
    TwinReject::InternalEnergyNotPositive,
    TwinReject::ProductsSoundSpeedNotReal,
    TwinReject::CellInverted,
    TwinReject::EnergyLedgerBroken,
    TwinReject::StepCapReached,
    TwinReject::ImpulseClosureBroken,
    TwinReject::AreaNotCarried,
    TwinReject::NotSettled,
];

impl TwinReject {
    pub fn name(&self) -> &'static str {
        match self {
            Self::InputNotFinite => "InputNotFinite",
            Self::ChargeMassNotPositive => "ChargeMassNotPositive",
            Self::DistanceNotPositive => "DistanceNotPositive",
            Self::ChargeBeyondUniformAtmosphere => "ChargeBeyondUniformAtmosphere",
            Self::DistanceBeyondUniformAtmosphere => "DistanceBeyondUniformAtmosphere",
            Self::DistanceBelowContinuum => "DistanceBelowContinuum",
            Self::PointInsideFireball => "PointInsideFireball",
            Self::WaveBeyondUniformAtmosphere => "WaveBeyondUniformAtmosphere",
            Self::ShockNotSharp => "ShockNotSharp",
            Self::NonFiniteValue => "NonFiniteValue",
            Self::InternalEnergyNotPositive => "InternalEnergyNotPositive",
            Self::ProductsSoundSpeedNotReal => "ProductsSoundSpeedNotReal",
            Self::CellInverted => "CellInverted",
            Self::EnergyLedgerBroken => "EnergyLedgerBroken",
            Self::StepCapReached => "StepCapReached",
            Self::ImpulseClosureBroken => "ImpulseClosureBroken",
            Self::AreaNotCarried => "AreaNotCarried",
            Self::NotSettled => "NotSettled",
        }
    }

    /// The sentence above the variant.
    pub fn meaning(&self) -> &'static str {
        match self {
            Self::InputNotFinite => "An input is not a finite number.",
            Self::ChargeMassNotPositive => "The charge mass is zero or less: there is no charge.",
            Self::DistanceNotPositive => "The distance is zero or less.",
            Self::ChargeBeyondUniformAtmosphere => "The charge is above 194,741,349 kg of TNT in free air, or 106,707,589 kg on the ground: at every point of it the shock is beyond a tenth of the air's scale height, 843.45 m, when the positive phase there ends, that radius being least, 0.421 Sachs lengths, at the fireball's edge.",
            Self::DistanceBeyondUniformAtmosphere => "The distance is a tenth of the air's scale height, 843.45 m, or beyond, where the ambient pressure differs from the charge's by 1 - e^-0.1 = 9.52 percent or more: the shock is past it before the positive phase at the point ends.",
            Self::DistanceBelowContinuum => "The distance is under 100 mean free paths of the air, 6.37e-6 m: the mean free path over the distance, the Knudsen number, passes 0.01, the edge of the continuum, where the Euler equations of a gas stop holding.",
            Self::PointInsideFireball => "The point is within the farthest reach of the detonation products, 0.8402 m/kg^(1/3) of the free-air charge: there products and air mix, their face being unstable as it slows, and the sharp face the model carries between them is not the real one.",
            Self::WaveBeyondUniformAtmosphere => "When the positive phase at the point ends, the shock is beyond a tenth of the air's scale height, 843.45 m, from the charge: the air the answer rests on reaches where the ambient pressure differs from the charge's by more than 1 - e^-0.1 = 9.52 percent, and the model's air is uniform.",
            Self::ShockNotSharp => "The shock's thermoviscous rise time, Taylor's 2 rho0 delta/(beta Ps), lowers the peak of the positive phase it heads by more than 2.5 percent, the peak's own settling line: the shock is not sharp against its phase, and the inviscid wave the model carries is not the air's.",
            Self::NonFiniteValue => "A value of the flow was not finite.",
            Self::InternalEnergyNotPositive => "A cell's internal energy fell to zero or below.",
            Self::ProductsSoundSpeedNotReal => "A cell of the products has no real sound speed in the JWL equation of state.",
            Self::CellInverted => "A cell's outer face crossed its inner face.",
            Self::EnergyLedgerBroken => "The energy of products and air above ambient departed from the products' energy at detonation by more than 1e-10 of it at a step.",
            Self::StepCapReached => "The Euler run reached 400,000 steps before the positive phase at the point ended.",
            Self::ImpulseClosureBroken => "The Friedlander form's integral departs from the impulse by more than 1e-12 of it.",
            Self::AreaNotCarried => "Weak-shock theory changed r times the impulse by more than 1e-12 of it.",
            Self::NotSettled => "The coarse and fine grids disagree by more than 2.5 percent in the peak, 1 percent in the impulse, 1.5 percent in the duration or 0.5 percent in the arrival time.",
        }
    }
}

impl fmt::Display for TwinReject {
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

/// The base solver's numerical refusals, in the twin's names.
fn from_base(reject: base::BlastReject) -> TwinReject {
    match reject {
        base::BlastReject::ImpulseClosureBroken => TwinReject::ImpulseClosureBroken,
        base::BlastReject::AreaNotCarried => TwinReject::AreaNotCarried,
        _ => TwinReject::NonFiniteValue,
    }
}

// ------------------------------------------------ units and the charge
//
// The run's units are blast_wave_tnt_air's: p0 = 1, rho_air = gamma,
// c0 = 1, lengths in Sachs lengths L = (M e_TNT/p0)^(1/3) of the free-air
// charge M at 4.184 MJ/kg (a unit only: the products carry their own
// energy), times in L/c0. Everything is per steradian.

/// Density in kg/m^3 of one unit of the run.
fn density_unit() -> f64 {
    base::ambient_density_kg_per_m3() / GAMMA
}

/// Specific energy in J/kg of one unit of the run.
fn energy_unit() -> f64 {
    AMBIENT_PRESSURE_PA / density_unit()
}

/// The charge's radius in Sachs lengths: (3 p0/(4 pi rho_TNT e_TNT))^(1/3), the same for every charge.
pub fn charge_radius_sachs() -> f64 {
    math::cbrt(3.0 * AMBIENT_PRESSURE_PA / (4.0 * core::f64::consts::PI * TNT_DENSITY_KG_PER_M3 * TNT_ENERGY_J_PER_KG))
}

/// The radius of a sphere of `mass_kg` of TNT, m.
pub fn charge_radius_m(mass_kg: f64) -> f64 {
    math::cbrt(3.0 * mass_kg / (4.0 * core::f64::consts::PI * TNT_DENSITY_KG_PER_M3))
}

/// The JWL equation of state of the products, in the run's units.
#[derive(Clone, Copy, Debug)]
struct Jwl {
    a: f64,
    b: f64,
    rho0: f64,
}

impl Jwl {
    fn new() -> Jwl {
        Jwl { a: JWL_A_PA / AMBIENT_PRESSURE_PA, b: JWL_B_PA / AMBIENT_PRESSURE_PA, rho0: TNT_DENSITY_KG_PER_M3 / density_unit() }
    }

    /// Pressure and the square of the sound speed at density `rho` and specific internal energy `e`.
    fn state(&self, rho: f64, e: f64) -> (f64, f64) {
        let v = self.rho0 / rho;
        let x1 = math::exp(-JWL_R1 * v);
        let x2 = math::exp(-JWL_R2 * v);
        let w = JWL_OMEGA;
        let p = self.a * (1.0 - w / (JWL_R1 * v)) * x1 + self.b * (1.0 - w / (JWL_R2 * v)) * x2 + w * rho * e;
        // (dp/drho) at constant e: d/drho = (V^2/rho0) d/dV ... with dV/drho = -V/rho
        let dpdv = self.a * x1 * (w / (JWL_R1 * v * v) - JWL_R1 * (1.0 - w / (JWL_R1 * v)))
            + self.b * x2 * (w / (JWL_R2 * v * v) - JWL_R2 * (1.0 - w / (JWL_R2 * v)));
        let dpdrho_e = -v / rho * dpdv + w * e;
        (p, dpdrho_e + w * p / rho)
    }
}

/// The JWL pressure of TNT's products, Pa, at relative volume `v` = rho0/rho and energy per unit initial volume `e_per_v0` J/m^3.
pub fn jwl_pressure_pa(v: f64, e_per_v0: f64) -> f64 {
    let w = JWL_OMEGA;
    JWL_A_PA * (1.0 - w / (JWL_R1 * v)) * math::exp(-JWL_R1 * v) + JWL_B_PA * (1.0 - w / (JWL_R2 * v)) * math::exp(-JWL_R2 * v) + w * e_per_v0 / v
}

// ------------------------------------------------ the Lagrangian run

#[derive(Clone, Copy, Debug)]
struct Grid {
    /// Uniform air cell far out, Sachs lengths.
    cell: f64,
    /// Near the charge an air cell is at most r over this many.
    per_radius: f64,
    /// Cells across the charge.
    products_cells: usize,
    /// The charge's radius.
    charge: f64,
}

impl Grid {
    fn width_at(&self, r0: f64) -> f64 {
        (r0 / self.per_radius).max(self.charge / self.products_cells as f64).min(self.cell)
    }
}

#[derive(Clone, Debug)]
struct Flow {
    r: Vec<f64>,
    m: Vec<f64>,
    u: Vec<f64>,
    et: Vec<f64>,
    /// Cells 0..products are the products, the rest air.
    products: usize,
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

/// The charge detonated through its volume: products at rest at the
/// charge's density and the calibrated detonation energy, air at rest
/// around it.
fn detonated(grid: &Grid, jwl: &Jwl, calibration: &Calibration) -> (Flow, f64) {
    let a = grid.charge;
    let n = grid.products_cells;
    let mut r = Vec::with_capacity(n + 128);
    for k in 0..=n {
        r.push(a * k as f64 / n as f64);
    }
    for _ in 0..128 {
        let last = r[r.len() - 1];
        r.push(last + grid.width_at(last));
    }
    let cells = r.len() - 1;
    let e_products = calibration.products_energy_factor * JWL_E0_J_PER_M3 / TNT_DENSITY_KG_PER_M3 / energy_unit();
    let e_air = base::ambient_specific_energy();
    let mut m = Vec::with_capacity(cells);
    let mut et = Vec::with_capacity(cells);
    let mut target = 0.0;
    for i in 0..cells {
        let volume = base::shell_volume(r[i], r[i + 1]);
        if i < n {
            m.push(jwl.rho0 * volume);
            et.push(e_products);
            target += jwl.rho0 * volume * e_products;
        } else {
            m.push(RHO0 * volume);
            et.push(e_air);
        }
    }
    (Flow { r, u: vec![0.0; cells], m, et, products: n }, target)
}

fn energy_above_ambient(flow: &Flow) -> f64 {
    let e0 = base::ambient_specific_energy();
    let mut sum = 0.0;
    for i in 0..flow.m.len() {
        sum += flow.m[i] * if i < flow.products { flow.et[i] } else { flow.et[i] - e0 };
    }
    sum
}

fn primitives(flow: &Flow, jwl: &Jwl, w: &mut Work) -> Result<(), TwinReject> {
    for i in 0..flow.m.len() {
        let volume = base::shell_volume(flow.r[i], flow.r[i + 1]);
        if !(volume > 0.0) {
            return Err(if volume.is_finite() { TwinReject::CellInverted } else { TwinReject::NonFiniteValue });
        }
        let rho = flow.m[i] / volume;
        let e = flow.et[i] - 0.5 * flow.u[i] * flow.u[i];
        if !e.is_finite() {
            return Err(TwinReject::NonFiniteValue);
        }
        if !(e > 0.0) {
            return Err(TwinReject::InternalEnergyNotPositive);
        }
        let (p, c2) = if i < flow.products { jwl.state(rho, e) } else { ((GAMMA - 1.0) * rho * e, GAMMA * (GAMMA - 1.0) * e) };
        if !(p.is_finite() && c2.is_finite()) {
            return Err(TwinReject::NonFiniteValue);
        }
        if !(c2 > 0.0) {
            return Err(TwinReject::ProductsSoundSpeedNotReal);
        }
        w.rho[i] = rho;
        w.p[i] = p;
        w.c[i] = c2.sqrt();
    }
    Ok(())
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
        w.slope_p[i] = base::limited((w.p[i] - w.p[i - 1]) / dl, (w.p[i + 1] - w.p[i]) / dr);
        w.slope_u[i] = base::limited((flow.u[i] - flow.u[i - 1]) / dl, (flow.u[i + 1] - flow.u[i]) / dr);
    }
    w.node_u[0] = 0.0;
    w.node_p[0] = w.p[0];
    for j in 1..n {
        let (l, r) = (j - 1, j);
        let pl = base::between(w.p[l] + w.slope_p[l] * (flow.r[j] - centre(l)), w.p[l], w.p[r]);
        let pr = base::between(w.p[r] - w.slope_p[r] * (centre(r) - flow.r[j]), w.p[l], w.p[r]);
        let ul = base::between(flow.u[l] + w.slope_u[l] * (flow.r[j] - centre(l)), flow.u[l], flow.u[r]);
        let ur = base::between(flow.u[r] - w.slope_u[r] * (centre(r) - flow.r[j]), flow.u[l], flow.u[r]);
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
    let l = n - 1;
    w.node_u[n] = 0.0;
    w.node_p[n] = w.p[l] + w.rho[l] * w.c[l] * flow.u[l];
}

fn stage(flow: &Flow, jwl: &Jwl, w: &mut Work, dt: f64, out: &mut Flow, fresh: bool) -> Result<(), TwinReject> {
    if !fresh {
        primitives(flow, jwl, w)?;
    }
    node_states(flow, w);
    let n = flow.m.len();
    for j in 0..=n {
        let a = flow.r[j];
        let b = a + dt * w.node_u[j];
        out.r[j] = b;
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
    out.products = flow.products;
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
    let p = if a == b { w.p[a] } else { w.p[a] + (r - centre(a)) / (centre(b) - centre(a)) * (w.p[b] - w.p[a]) };
    p - P0
}

/// The shock's radius: where the overpressure in the air, from the outside
/// in, first rises through half the largest in the air.
fn front_of(flow: &Flow, w: &Work) -> f64 {
    let n = flow.m.len();
    let mut largest = 0.0f64;
    for i in flow.products..n {
        largest = largest.max(w.p[i] - P0);
    }
    let half = 0.5 * largest;
    let centre = |i: usize| 0.5 * (flow.r[i] + flow.r[i + 1]);
    for i in (flow.products..n).rev() {
        let over = w.p[i] - P0;
        if over >= half {
            if i + 1 == n {
                return centre(i);
            }
            let next = w.p[i + 1] - P0;
            return centre(i) + (over - half) / (over - next) * (centre(i + 1) - centre(i));
        }
    }
    flow.r[flow.products]
}

/// One Euler run to the end of the positive phase at `point`; the record and the farthest the products reached.
fn run(grid: &Grid, jwl: &Jwl, calibration: &Calibration, point: f64, fault: Fault) -> Result<(Record, f64), TwinReject> {
    let (mut flow, target) = detonated(grid, jwl, calibration);
    let mut t = 0.0;
    let e0 = base::ambient_specific_energy();
    let gauge_r: [f64; 7] = core::array::from_fn(|k| point * (1.0 + SPEED_GAUGE_SPREAD * (k as f64 - 3.0) / 3.0));
    let mut rec = Record { t: Vec::new(), point: Vec::new(), front: Vec::new(), gauge_r, gauges: Default::default(), worst_energy: 0.0, steps: 0 };
    let mut a = flow.clone();
    let mut b = flow.clone();
    let mut w = Work::new(flow.m.len());
    let mut arrived = false;
    let mut peak = 0.0f64;
    let mut outer_steps = 0usize;
    let mut products_max = grid.charge;
    let merge_inside = MERGE_INSIDE_GAUGE * gauge_r[0];
    let mut current = false;
    loop {
        // keep at least 40 undisturbed cells ahead of the wave
        let n = flow.m.len();
        let mut front = 0;
        for i in (flow.products..n).rev() {
            if (flow.et[i] - e0).abs() > 1.0e-12 || flow.u[i].abs() > 1.0e-12 {
                front = i;
                break;
            }
        }
        if n - front < 40 {
            for _ in 0..256 {
                let last = flow.r[flow.r.len() - 1];
                let next = last + grid.width_at(last);
                flow.m.push(RHO0 * base::shell_volume(last, next));
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
            primitives(&flow, jwl, &mut w)?;
        }
        // join pairs of air cells whose own step is shorter than a uniform
        // cell's, far behind the shock or rarefied below the ambient
        // density, inside the innermost gauge: mass, momentum and energy
        // summed, so the ledger is untouched; the products' cells are never
        // joined, their resolution being what settles the wave
        let n = flow.m.len();
        let mut largest = 0.0f64;
        for i in flow.products..n {
            largest = largest.max(w.p[i] - P0);
        }
        let mut shock = flow.products;
        for i in (flow.products..n).rev() {
            if w.p[i] - P0 > 1.0e-3 * largest {
                shock = i;
                break;
            }
        }
        let behind = 0.5 * flow.r[shock + 1];
        let limit = flow.r[shock + 1].min(merge_inside);
        let short = grid.cell / 1.3;
        let np = flow.products;
        // far behind the shock, or rarefied below the ambient density
        let pairs = |flow: &Flow, w: &Work, i: usize| {
            i + 1 < n
                && i >= np
                && flow.r[i + 2] < limit
                && (flow.r[i + 2] < behind || (w.rho[i] < RHO0 && w.rho[i + 1] < RHO0))
                && (flow.r[i + 1] - flow.r[i]) / w.c[i] < short
        };
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
            let mut merged = Flow { r: vec![0.0], m: Vec::with_capacity(n), u: Vec::with_capacity(n), et: Vec::with_capacity(n), products: 0 };
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
                if next <= np {
                    merged.products = merged.m.len();
                }
                i = next;
            }
            if any {
                flow = merged;
                a = flow.clone();
                b = flow.clone();
                w = Work::new(flow.m.len());
                primitives(&flow, jwl, &mut w)?;
            }
        }
        let n = flow.m.len();
        let mut dt = f64::INFINITY;
        for i in 0..n {
            let du = if i + 1 < n { (flow.u[i + 1] - flow.u[i]).abs() } else { 0.0 };
            dt = dt.min(COURANT_NUMBER * (flow.r[i + 1] - flow.r[i]) / (w.c[i] + du));
        }
        if !(dt.is_finite() && dt > 0.0) {
            return Err(TwinReject::NonFiniteValue);
        }
        stage(&flow, jwl, &mut w, dt, &mut a, true)?;
        stage(&a, jwl, &mut w, dt, &mut b, false)?;
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
            return Err(TwinReject::NonFiniteValue);
        }
        rec.worst_energy = rec.worst_energy.max(residual);
        if residual > ENERGY_LEDGER_TOLERANCE {
            return Err(TwinReject::EnergyLedgerBroken);
        }
        primitives(&flow, jwl, &mut w)?;
        current = true;
        products_max = products_max.max(flow.r[flow.products]);
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
            return Ok((rec, products_max));
        }
        if rec.steps >= MAX_STEPS {
            return Err(TwinReject::StepCapReached);
        }
    }
}

/// The wave at Sachs distance `point` on one grid: Euler to the point, or to
/// Sachs distance 4 and weak-shock theory beyond.
fn wave_on(grid: &Grid, jwl: &Jwl, calibration: &Calibration, point: f64, fault: Fault) -> Result<(Wave, f64, f64, usize, f64), TwinReject> {
    let reach = point.min(WEAK_SHOCK_FROM_SACHS_DISTANCE);
    let (rec, products_max) = run(grid, jwl, calibration, reach, fault)?;
    let wave = base::wave_of(&rec, reach).map_err(from_base)?;
    if point <= reach {
        return Ok((wave, 0.0, rec.worst_energy, rec.steps, products_max));
    }
    let decay = base::friedlander_decay(wave.impulse / (wave.peak * wave.duration)).ok_or(TwinReject::ImpulseClosureBroken)?;
    let (mut far, area) = base::carried(&wave, decay, reach, point, fault);
    if !(far.peak.is_finite() && far.duration.is_finite() && far.impulse.is_finite() && area.is_finite()) {
        return Err(TwinReject::NonFiniteValue);
    }
    if area > AREA_TOLERANCE {
        return Err(TwinReject::AreaNotCarried);
    }
    far.front_at_end = base::reach_at_end(&wave, decay, reach, point, &far);
    if !far.front_at_end.is_finite() {
        return Err(TwinReject::NonFiniteValue);
    }
    Ok((far, area, rec.worst_energy, rec.steps, products_max))
}

fn grids(point: f64, coarse_factor: f64) -> (Grid, Grid) {
    let reach = point.min(WEAK_SHOCK_FROM_SACHS_DISTANCE);
    let charge = charge_radius_sachs();
    let fine = FINE_CELL_SACHS.min((reach / FINE_CELLS_ACROSS_DISTANCE).max(charge / FINE_PRODUCTS_CELLS as f64));
    let fine_products = FINE_PRODUCTS_CELLS.max((charge / fine).ceil() as usize);
    let coarse_products = ((fine_products as f64 / coarse_factor).ceil() as usize).max(1);
    (
        Grid { cell: fine * coarse_factor, per_radius: FINE_CELLS_PER_RADIUS / coarse_factor, products_cells: coarse_products, charge },
        Grid { cell: fine, per_radius: FINE_CELLS_PER_RADIUS, products_cells: fine_products, charge },
    )
}

// ------------------------------------------------------------ solve

/// The free-air charge a burst of `w` kg is taken as, kg.
pub fn free_air_charge_kg(w: f64, burst: Burst, calibration: &Calibration) -> f64 {
    match burst {
        Burst::FreeAir => w,
        Burst::Surface => calibration.ground_factor * w,
    }
}

/// The scaled distance of the charge's surface, m/kg^(1/3): the free-air sphere's radius over W^(1/3).
pub fn charge_surface_scaled_distance(burst: Burst, calibration: &Calibration) -> f64 {
    charge_radius_m(free_air_charge_kg(1.0, burst, calibration))
}

/// The farthest the face of the products and the air reaches from a free-air
/// charge of 1 kg, m: the served constants' fine grid at Z = 14.94, where the
/// run lasts past the products' largest radius (a test computes it again).
pub const FIREBALL_RADIUS_M_PER_KG13: f64 = 0.8402;

/// The shock's radius when the positive phase ends, least over the points
/// served, in Sachs lengths of the free-air charge, and where (a test holds
/// it below its neighbours').
pub const LEAST_SHOCK_REACH_SACHS: f64 = 0.421;
/// The Sachs distance of that least reach.
pub const LEAST_REACH_SACHS_DISTANCE: f64 = 0.2431;

/// The largest free-air charge any point is answered at, kg: the one whose
/// shock is a tenth of a scale height out when the positive phase ends
/// where that radius is least. On the ground, that over the ground factor.
pub const CHARGE_MASS_MAX_KG: f64 = {
    let reach = DISTANCE_MAX_M / LEAST_SHOCK_REACH_SACHS;
    AMBIENT_PRESSURE_PA * reach * reach * reach / TNT_ENERGY_J_PER_KG
};

/// Every limit the question itself shows, in the order the solver asks them.
pub fn admit(question: &TwinQuestion) -> Result<(), TwinReject> {
    admit_with(question, &CALIBRATION)
}

fn admit_with(question: &TwinQuestion, calibration: &Calibration) -> Result<(), TwinReject> {
    let TwinQuestion { charge_mass_kg: w, distance_m: r, burst } = *question;
    if !(w.is_finite() && r.is_finite()) {
        return Err(TwinReject::InputNotFinite);
    }
    if !(w > 0.0) {
        return Err(TwinReject::ChargeMassNotPositive);
    }
    if !(r > 0.0) {
        return Err(TwinReject::DistanceNotPositive);
    }
    if !(free_air_charge_kg(w, burst, calibration) <= CHARGE_MASS_MAX_KG) {
        return Err(TwinReject::ChargeBeyondUniformAtmosphere);
    }
    if !(r < DISTANCE_MAX_M) {
        return Err(TwinReject::DistanceBeyondUniformAtmosphere);
    }
    if !(r >= base::DISTANCE_MIN_M) {
        return Err(TwinReject::DistanceBelowContinuum);
    }
    if !(r > FIREBALL_RADIUS_M_PER_KG13 * math::cbrt(free_air_charge_kg(w, burst, calibration))) {
        return Err(TwinReject::PointInsideFireball);
    }
    Ok(())
}

/// The blast wave at the point, or the limit that stopped it.
pub fn solve(question: &TwinQuestion) -> Result<TwinWave, TwinReject> {
    crate::normal::held(solve_with(question, &CALIBRATION, Fault::None, COARSE_FACTOR))
}

/// The same with other constants: the fit's own door. Not served.
pub fn solve_calibrated(question: &TwinQuestion, calibration: &Calibration) -> Result<TwinWave, TwinReject> {
    crate::normal::held(solve_with(question, calibration, Fault::None, COARSE_FACTOR))
}

impl NormalOrZero for TwinWave {
    fn normal_or_zero(&mut self) {
        self.scaled_distance_m_per_kg13.normal_or_zero();
        self.free_air_charge_kg.normal_or_zero();
        self.charge_radius_m.normal_or_zero();
        self.arrival_time_s.normal_or_zero();
        self.peak_overpressure_pa.normal_or_zero();
        self.shock_mach_number.normal_or_zero();
        self.positive_phase_duration_s.normal_or_zero();
        self.positive_impulse_pa_s.normal_or_zero();
        self.friedlander_decay.normal_or_zero();
        self.scaled_arrival_time_ms_per_kg13.normal_or_zero();
        self.scaled_positive_phase_duration_ms_per_kg13.normal_or_zero();
        self.scaled_positive_impulse_kpa_ms_per_kg13.normal_or_zero();
        self.products_radius_max_m.normal_or_zero();
        self.euler_reach_m_per_kg13.normal_or_zero();
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

/// The range of Z compared with Dewey's tables, m/kg^(1/3).
pub const COMPARED_Z_MIN: f64 = 1.0;
/// The range of Z compared with Dewey's tables, m/kg^(1/3).
pub const COMPARED_Z_MAX: f64 = 14.95;

fn solve_with(question: &TwinQuestion, calibration: &Calibration, fault: Fault, coarse_factor: f64) -> Result<TwinWave, TwinReject> {
    admit_with(question, calibration)?;
    let TwinQuestion { charge_mass_kg: w, distance_m: r, burst } = *question;
    let mass = free_air_charge_kg(w, burst, calibration);
    let length = math::cbrt(mass * TNT_ENERGY_J_PER_KG / AMBIENT_PRESSURE_PA);
    let time_unit = length / base::sound_speed_m_per_s();
    let point = r / length;
    let jwl = Jwl::new();
    let (coarse_grid, fine_grid) = grids(point, coarse_factor);
    let (coarse, _, _, coarse_steps, _) = wave_on(&coarse_grid, &jwl, calibration, point, fault)?;
    let (fine, area, worst_energy, fine_steps, products_max) = wave_on(&fine_grid, &jwl, calibration, point, fault)?;
    let change = |a: f64, b: f64| ((b - a) / b).abs();
    let (dp, di, dd, da) = (
        change(coarse.peak, fine.peak),
        change(coarse.impulse, fine.impulse),
        change(coarse.duration, fine.duration),
        change(coarse.arrival, fine.arrival),
    );
    if !(dp <= SETTLE_PEAK && di <= SETTLE_IMPULSE && dd <= SETTLE_DURATION && da <= SETTLE_ARRIVAL) {
        return Err(TwinReject::NotSettled);
    }
    let fraction = fine.impulse / (fine.peak * fine.duration);
    let decay = base::friedlander_decay(fraction).ok_or(TwinReject::ImpulseClosureBroken)?;
    let closure = ((fine.peak * fine.duration * base::friedlander_fraction(decay) - fine.impulse) / fine.impulse).abs();
    if !(closure <= IMPULSE_CLOSURE_TOLERANCE) {
        return Err(TwinReject::ImpulseClosureBroken);
    }
    let shock_radius_at_end_m = fine.front_at_end * length;
    if !(shock_radius_at_end_m <= DISTANCE_MAX_M) {
        return Err(TwinReject::WaveBeyondUniformAtmosphere);
    }
    let rise_s = base::shock_rise_time_s(fine.peak * AMBIENT_PRESSURE_PA);
    if !(base::peak_smoothing(rise_s, fine.duration * time_unit, decay) <= SETTLE_PEAK) {
        return Err(TwinReject::ShockNotSharp);
    }
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
    let z = r / cube_root_w;
    let arrival_time_s = fine.arrival * time_unit;
    let duration_s = fine.duration * time_unit;
    let impulse_pa_s = fine.impulse * AMBIENT_PRESSURE_PA * time_unit;
    let answer = TwinWave {
        scaled_distance_m_per_kg13: z,
        free_air_charge_kg: mass,
        charge_radius_m: charge_radius_m(mass),
        arrival_time_s,
        peak_overpressure_pa: fine.peak * AMBIENT_PRESSURE_PA,
        shock_mach_number: fine.mach,
        positive_phase_duration_s: duration_s,
        positive_impulse_pa_s: impulse_pa_s,
        friedlander_decay: decay,
        scaled_arrival_time_ms_per_kg13: arrival_time_s * 1.0e3 / cube_root_w,
        scaled_positive_phase_duration_ms_per_kg13: duration_s * 1.0e3 / cube_root_w,
        scaled_positive_impulse_kpa_ms_per_kg13: impulse_pa_s / cube_root_w,
        products_radius_max_m: products_max * length,
        euler_reach_m_per_kg13: point.min(WEAK_SHOCK_FROM_SACHS_DISTANCE) * length / cube_root_w,
        shock_radius_at_end_m,
        shock_rise_time_s: rise_s,
        scaled_distance_outside_compared: !(COMPARED_Z_MIN..=COMPARED_Z_MAX).contains(&z),
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
        answer.products_radius_max_m,
        answer.shock_radius_at_end_m,
        answer.shock_rise_time_s,
    ];
    if finite.iter().any(|v| !v.is_finite()) || answer.history_overpressure_pa.iter().chain(&answer.friedlander_overpressure_pa).any(|v| !v.is_finite()) {
        return Err(TwinReject::NonFiniteValue);
    }
    Ok(answer)
}

// ------------------------------------------------------ the references

/// The published tables the twin is fitted on and compared with: J. M.
/// Dewey's TNT free-field and surface-burst blast wave interfaces, built
/// from particle-trajectory analyses of TNT explosions of 4 kg to 500 kg
/// (Dewey and McMillin 1987, 1989); measurement-derived tables.
pub const REFERENCE_NAME: &str = "J. M. Dewey (2021), TNT Free-Field Blast Wave Properties Interface (TNTFFI 1.1c), and J. M. Dewey (2022), TNT Surface-Burst Blast Wave Interface (TNTSBI 1.0c), Borealis, CC0 1.0: peak overpressure and arrival of 1 kg of TNT from 1 to 14.95 m, and Friedlander coefficients from 1.31 to 14.94 m";
pub const REFERENCE_URL: &str = "https://doi.org/10.5683/SP2/WB8JKO";
/// The surface-burst interface.
pub const SURFACE_REFERENCE_URL: &str = "https://doi.org/10.5683/SP3/CFPFRH";
pub const REFERENCE_RETRIEVED: &str = "2026-10-14";
/// The source of the JWL constants of TNT.
pub const JWL_SOURCE: &str = "B. M. Dobratz and P. C. Crawford, LLNL Explosives Handbook: Properties of Chemical Explosives and Explosive Simulants, UCRL-52997 Change 2, Lawrence Livermore National Laboratory, 1985: TNT at 1.630 g/cm^3, D 6930 m/s, P_CJ 21.0 GPa, A 373.8 GPa, B 3.747 GPa, R1 4.15, R2 0.90, omega 0.35, E0 6.00 GPa";

#[cfg(test)]
mod tests {
    use super::*;

    fn free(w: f64, r: f64) -> TwinQuestion {
        TwinQuestion { charge_mass_kg: w, distance_m: r, burst: Burst::FreeAir }
    }

    fn surface(w: f64, r: f64) -> TwinQuestion {
        TwinQuestion { charge_mass_kg: w, distance_m: r, burst: Burst::Surface }
    }

    fn ratio(model: f64, reference: f64) -> f64 {
        model / reference - 1.0
    }

    /// TNT's Chapman-Jouguet state, 21.0 GPa at 6,930 m/s and 1,630 kg/m^3,
    /// lies on the JWL form with E0 (the Rayleigh line and the Hugoniot of
    /// the products meet there): V = 1 - P/(rho0 D^2), E = E0 + P (1 - V)/2.
    #[test]
    fn the_chapman_jouguet_state_lies_on_the_jwl_form() {
        let v = 1.0 - TNT_CJ_PRESSURE_PA / (TNT_DENSITY_KG_PER_M3 * TNT_DETONATION_VELOCITY_M_PER_S * TNT_DETONATION_VELOCITY_M_PER_S);
        let e = JWL_E0_J_PER_M3 + 0.5 * TNT_CJ_PRESSURE_PA * (1.0 - v);
        let p = jwl_pressure_pa(v, e);
        assert!(ratio(p, TNT_CJ_PRESSURE_PA).abs() < 0.003, "{p} at V {v}");
        // and the run's own form is the same one, in its units
        let jwl = Jwl::new();
        let (pu, c2) = jwl.state(jwl.rho0 / v, e / TNT_DENSITY_KG_PER_M3 / energy_unit());
        assert!(ratio(pu * AMBIENT_PRESSURE_PA, p).abs() < 1.0e-12);
        assert!(c2 > 0.0);
    }

    /// The products' sound speed is the derivative of the form along an
    /// isentrope, de = -p dv: held against a finite difference.
    #[test]
    fn the_products_sound_speed_is_the_isentropes() {
        let jwl = Jwl::new();
        for (rho, e) in [(jwl.rho0, 30.0), (0.3 * jwl.rho0, 10.0), (0.01 * jwl.rho0, 3.0), (2.0, 1.5)] {
            let (p, c2) = jwl.state(rho, e);
            let h = 1.0e-6 * rho;
            let (p2, _) = jwl.state(rho + h, e + p / (rho * rho) * h);
            let (p1, _) = jwl.state(rho - h, e - p / (rho * rho) * h);
            let fd = (p2 - p1) / (2.0 * h);
            assert!(ratio(c2, fd).abs() < 1.0e-5, "{rho} {e}: {c2} against {fd}");
        }
    }

    #[test]
    fn the_worked_question_settles_with_its_ledgers_closed() {
        let wave = solve(&free(1.0, 5.0)).unwrap();
        assert!(wave.energy_ledger_residual < 1.0e-12, "{}", wave.energy_ledger_residual);
        assert!(wave.impulse_closure_residual <= IMPULSE_CLOSURE_TOLERANCE);
        assert_eq!(wave.weak_shock_area_residual, 0.0);
        assert!(wave.grid_change_peak < 0.01 && wave.grid_change_impulse < SETTLE_IMPULSE && wave.grid_change_duration < SETTLE_DURATION && wave.grid_change_arrival < SETTLE_ARRIVAL);
        assert!(!wave.scaled_distance_outside_compared);
        assert_eq!(wave.history_overpressure_pa.len(), HISTORY_POINTS);
        assert_eq!(wave.history_overpressure_pa[HISTORY_POINTS - 1], 0.0);
    }

    #[test]
    fn unbooked_heat_breaks_the_energy_ledger_by_name() {
        assert_eq!(solve_with(&free(1.0, 3.0), &CALIBRATION, Fault::UnbookedHeat, COARSE_FACTOR), Err(TwinReject::EnergyLedgerBroken));
    }

    #[test]
    fn a_weak_shock_regime_that_loses_area_is_named() {
        assert_eq!(solve_with(&free(1.0, 20.0), &CALIBRATION, Fault::LostArea, COARSE_FACTOR), Err(TwinReject::AreaNotCarried));
    }

    #[test]
    fn grids_that_disagree_are_named_not_settled() {
        assert_eq!(solve_with(&free(1.0, 3.0), &CALIBRATION, Fault::None, 12.0), Err(TwinReject::NotSettled));
    }

    #[test]
    fn each_limit_is_named() {
        assert_eq!(admit(&free(f64::NAN, 5.0)), Err(TwinReject::InputNotFinite));
        assert_eq!(admit(&free(1.0, f64::INFINITY)), Err(TwinReject::InputNotFinite));
        assert_eq!(admit(&free(0.0, 5.0)), Err(TwinReject::ChargeMassNotPositive));
        assert_eq!(admit(&free(1.0, 0.0)), Err(TwinReject::DistanceNotPositive));
        assert_eq!(admit(&free(CHARGE_MASS_MAX_KG * 1.000001, 500.0)), Err(TwinReject::ChargeBeyondUniformAtmosphere));
        assert_eq!(admit(&surface(CHARGE_MASS_MAX_KG / GROUND_FACTOR * 1.000001, 500.0)), Err(TwinReject::ChargeBeyondUniformAtmosphere));
        assert_eq!(admit(&free(1.0, DISTANCE_MAX_M)), Err(TwinReject::DistanceBeyondUniformAtmosphere));
        assert_eq!(admit(&free(1.0e-30, 1.0e-6)), Err(TwinReject::DistanceBelowContinuum));
        // the fireball: just inside it, refused; just outside, admitted
        let edge = FIREBALL_RADIUS_M_PER_KG13;
        assert_eq!(admit(&free(1.0, edge)), Err(TwinReject::PointInsideFireball));
        assert_eq!(admit(&free(1.0, edge * 1.000001)), Ok(()));
        let ground = edge * math::cbrt(GROUND_FACTOR);
        assert_eq!(admit(&surface(1.0, ground * 0.999999)), Err(TwinReject::PointInsideFireball));
        assert_eq!(admit(&surface(1.0, ground * 1.000001)), Ok(()));
        // a far point of a small charge: its shock is not sharp
        assert_eq!(solve(&free(1.0e-3, 840.0)), Err(TwinReject::ShockNotSharp));
    }

    /// A surface burst of W is the twin's free-air burst of the ground
    /// factor times W, bit for bit; and Hopkinson scaling holds.
    #[test]
    fn a_surface_burst_is_a_free_air_burst_of_the_ground_factor_times_the_charge() {
        let ground = solve(&surface(1.0, 5.0)).unwrap();
        let air = solve(&free(GROUND_FACTOR, 5.0)).unwrap();
        assert_eq!(ground.peak_overpressure_pa, air.peak_overpressure_pa);
        assert_eq!(ground.positive_impulse_pa_s, air.positive_impulse_pa_s);
        assert_eq!(ground.free_air_charge_kg, GROUND_FACTOR);
        let big = solve(&free(1000.0, 50.0)).unwrap();
        let small = solve(&free(1.0, 5.0)).unwrap();
        assert!(ratio(big.peak_overpressure_pa, small.peak_overpressure_pa).abs() < 1.0e-9);
        assert!(ratio(big.scaled_positive_impulse_kpa_ms_per_kg13, small.scaled_positive_impulse_kpa_ms_per_kg13).abs() < 1.0e-9);
    }

    /// The sentence above each variant is the one `meaning` serves.
    #[test]
    fn every_meaning_is_the_sentence_above_its_variant() {
        let source = include_str!("blast_wave_tnt_air_twin.rs");
        let start = source.find("pub enum TwinReject {").unwrap();
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
        // the numbers of the sentences are the constants'
        assert_eq!(format!("{:.0}", CHARGE_MASS_MAX_KG), "194741349");
        assert_eq!(format!("{:.0}", CHARGE_MASS_MAX_KG / GROUND_FACTOR), "106707589");
    }

    #[test]
    fn the_same_question_gives_the_same_bits() {
        assert_eq!(solve(&free(2.0, 7.0)), solve(&free(2.0, 7.0)));
    }

    /// Dewey's TNT free-air interface (TNTFFI 1.1c, CC0, retrieved
    /// 2026-10-14), 1 kg: the table the products' energy factor is fitted
    /// on. Peak, impulse and duration from its sheets ("AB Peaks" and the
    /// Friedlander coefficients of "Time Histories"), the arrival beside
    /// them, not fitted. The twin within 3.1 percent at Z = 2, 5 and 10;
    /// each held to half a percentage point
    /// (evidence/research/blast_overpressure/twin/compare.py).
    #[test]
    fn dewey_tnt_free_field_fit() {
        // Z, peak kPa, arrival ms, duration ms, impulse kPa ms (TNTFFI, 1 kg)
        let reference = [
            (2.0, 182.6425, 1.907667, 1.329653, 75.551427),
            (5.0, 31.056189, 8.950615, 2.783318, 34.591422),
            (10.0, 10.970094, 22.642857, 3.616525, 17.30983),
        ];
        let stated = [(-0.0070, 0.0085, -0.0041, -0.0163), (0.0028, 0.0039, -0.0302, -0.0209), (0.0077, 0.0017, -0.0106, -0.0078)];
        for ((z, p, ta, td, imp), (dp, da, dd, di)) in reference.iter().zip(stated) {
            let wave = solve(&free(1.0, *z)).unwrap();
            let found = (
                ratio(wave.peak_overpressure_pa / 1.0e3, *p),
                ratio(wave.arrival_time_s * 1.0e3, *ta),
                ratio(wave.positive_phase_duration_s * 1.0e3, *td),
                ratio(wave.positive_impulse_pa_s, *imp),
            );
            for (f, s) in [(found.0, dp), (found.1, da), (found.2, dd), (found.3, di)] {
                assert!((f - s).abs() < 0.005, "Z {z}: {found:?} against {stated:?}");
            }
        }
    }

    /// Dewey's TNT surface-burst interface (TNTSBI 1.0c, CC0): its peaks
    /// are what the ground factor is fitted on; its positive-phase table,
    /// the impulse and the duration, was kept out of the fit. At Z = 2, 5
    /// and 10 the impulse is within 3.6 percent and the duration within 3.2.
    #[test]
    fn dewey_tnt_surface_burst_positive_phase_left_out() {
        // Z, peak kPa, duration ms, impulse kPa ms (TNTSBI, 1 kg)
        let reference = [(2.0, 297.093349, 1.247467, 103.263885), (5.0, 44.323879, 3.004902, 51.859363), (10.0, 15.238590, 4.196588, 26.534388)];
        let stated = [(-0.0119, 0.0313, 0.0075), (-0.0147, -0.0179, -0.0358), (-0.0368, -0.0276, -0.0353)];
        for ((z, p, td, imp), (dp, dd, di)) in reference.iter().zip(stated) {
            let wave = solve(&surface(1.0, *z)).unwrap();
            let found = (
                ratio(wave.peak_overpressure_pa / 1.0e3, *p),
                ratio(wave.positive_phase_duration_s * 1.0e3, *td),
                ratio(wave.positive_impulse_pa_s, *imp),
            );
            for (f, s) in [(found.0, dp), (found.1, dd), (found.2, di)] {
                assert!((f - s).abs() < 0.005, "Z {z}: {found:?}");
            }
        }
    }

    /// Kingery and Bulmash's hemispherical surface burst as Swisdak (1994)
    /// simplified it and fcento100 coded it (MIT; not compared with either
    /// original), kept out of the fit: the peak within 8 percent from Z = 2
    /// to 40, the impulse 16 to 23 percent below it and the duration 15 to
    /// 37 percent below it, where Kingery-Bulmash's own are 14 to 63 percent
    /// above Dewey's.
    #[test]
    fn kingery_bulmash_surface_burst_left_out() {
        fn kb(z: f64, c: &[f64]) -> f64 {
            let l = math::log(z);
            math::exp(c.iter().enumerate().map(|(i, ci)| ci * math::powi(l, i as i32)).sum::<f64>())
        }
        let peak = |z: f64| {
            if z <= 2.9 { kb(z, &[7.2106, -2.1069, -0.3229, 0.1117, 0.0685]) } else if z <= 23.8 { kb(z, &[7.5938, -3.0523, 0.40977, 0.0261, -0.01267]) } else { kb(z, &[6.0536, -1.4066]) }
        };
        let impulse = |z: f64| {
            if z <= 2.38 { kb(z, &[5.465, -0.308, -1.464, 1.362, -0.432]) } else if z <= 33.7 { kb(z, &[5.2749, -0.4677, -0.2499, 0.0588, -0.00554]) } else { kb(z, &[5.9825, -1.062]) }
        };
        let duration = |z: f64| {
            if z <= 2.8 { kb(z, &[0.5440, 2.7082, -9.7354, 14.3425, -9.7791, 2.8535]) } else { kb(z, &[-2.4608, 7.1639, -5.6215, 2.2711, -0.44994, 0.03486]) }
        };
        let stated = [(2.0, 0.0346, -0.2268, -0.3734), (5.0, 0.0102, -0.1569, -0.2221), (10.0, -0.0142, -0.1752, -0.1461), (20.0, -0.0784, -0.1923, -0.1482), (40.0, -0.0279, -0.1860, -0.1717)];
        for (z, dp, di, dd) in stated {
            let wave = solve(&surface(1.0, z)).unwrap();
            let found = (
                ratio(wave.peak_overpressure_pa / 1.0e3, peak(z)),
                ratio(wave.positive_impulse_pa_s, impulse(z)),
                ratio(wave.positive_phase_duration_s * 1.0e3, duration(z)),
            );
            for (f, s) in [(found.0, dp), (found.1, di), (found.2, dd)] {
                assert!((f - s).abs() < 0.005, "Z {z}: {found:?}");
            }
        }
    }

    /// The twin against the point source it succeeds, at Dewey's free-air
    /// peaks: nearer at every Z of 2, 5 and 10.
    #[test]
    fn the_twin_is_nearer_the_tnt_table_than_the_point_source() {
        for (z, p) in [(2.0, 182.6425), (5.0, 31.056189), (10.0, 10.970094)] {
            let twin = solve(&free(1.0, z)).unwrap().peak_overpressure_pa / 1.0e3;
            let point = base::solve(&base::BlastQuestion { charge_mass_kg: 1.0, distance_m: z, burst: Burst::FreeAir }).unwrap().peak_overpressure_pa / 1.0e3;
            assert!(ratio(twin, p).abs() < ratio(point, p).abs(), "Z {z}: twin {twin}, point source {point}, table {p}");
        }
    }

    /// The fireball's reach, computed again: a run long enough that the
    /// products' face has turned back (Z = 14.94, the Euler run to Sachs
    /// distance 4) reaches FIREBALL_RADIUS_M_PER_KG13 within 0.01 percent;
    /// and the shock's radius when the positive phase ends is least at the
    /// fireball's edge, at or above LEAST_SHOCK_REACH_SACHS and within 0.3
    /// percent of it, rising outward.
    #[test]
    fn the_fireball_and_the_least_reach_are_computed_again() {
        let far = solve(&free(1.0, 14.94)).unwrap();
        assert!(ratio(far.products_radius_max_m, FIREBALL_RADIUS_M_PER_KG13).abs() < 1.0e-4, "{}", far.products_radius_max_m);
        let length = math::cbrt(TNT_ENERGY_J_PER_KG / AMBIENT_PRESSURE_PA);
        let edge = solve(&free(1.0, FIREBALL_RADIUS_M_PER_KG13 * 1.0001)).unwrap();
        let next = solve(&free(1.0, 0.9)).unwrap();
        let least = edge.shock_radius_at_end_m / length;
        assert!(least >= LEAST_SHOCK_REACH_SACHS && least < LEAST_SHOCK_REACH_SACHS * 1.003, "{least}");
        assert!(next.shock_radius_at_end_m > edge.shock_radius_at_end_m);
        assert!(ratio(FIREBALL_RADIUS_M_PER_KG13 / length, LEAST_REACH_SACHS_DISTANCE).abs() < 1.0e-3);
    }
}
