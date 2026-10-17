//! Oil film of the University of Sheffield journal bearing test platform.
//!
//! The specimen is one plain full journal bearing, flooded with mineral oil and
//! held at 65 °C, on the bespoke test platform of the Leonardo Centre for
//! Tribology: Beamish, Li, Brunskill, Hunter and Dwyer-Joyce, "Circumferential
//! film thickness measurement in journal bearings via the ultrasonic
//! technique", Tribology International 148 (2020) 106295, with its data at
//! Mendeley Data, doi 10.17632/txdrvd382p.2 (CC BY 4.0).
//!
//! The caller gives the steady load and the shaft speed. The solver refines
//! the film pressure over the whole bearing surface until it settles, and
//! moves the shaft until the film carries the load. It returns the Sommerfeld
//! number, the eccentricity ratio, the attitude angle, the thinnest film, the
//! peak film pressure and the settled pressure field, or a named limit.
//!
//! Only the thinnest film was compared with measurement, on fifty runs at 2 to
//! 20 kN and 100 to 800 rpm, and the Sommerfeld number with the published
//! table. The attitude angle and the film pressure come from the same settled
//! field and were compared with neither. Any load and speed above zero is
//! answered: the film depends on them only through the Sommerfeld number, and
//! stops by name where the shaft would have to sit outside the eccentricity
//! ratios searched or where the film is no longer laminar. Through engine
//! 0.8.0 a load or speed outside the tested envelope was refused by name, and
//! the attitude angle and the peak pressure were not returned.
//!
//! The film is thin, so the pressure `p(θ, z)` obeys the Reynolds equation for
//! an oil of one viscosity between a rigid shaft and a rigid bush:
//!
//! ```text
//! ∂/∂θ (h³ ∂p/∂θ) + R² ∂/∂z (h³ ∂p/∂z) = 6 μ ω R² dh/dθ
//! h(θ) = c (1 + ε cos θ)
//! ```
//!
//! `θ` is measured from the widest gap in the direction of rotation, `z` along
//! the shaft, `ε` is the eccentricity ratio. The pressure is ambient at both
//! ends of the bush. The film cannot pull: a cell whose balance asks for a
//! pressure below ambient is at ambient, and the film ruptures where the
//! pressure and its gradient reach zero together (the Reynolds condition,
//! found by Christopherson's method). That floor is the rupture condition of
//! the model, not a limit of the solver, and the accepted field is checked
//! against it cell by cell.
//!
//! In the solver the pressure is `P = p c² / (μ ω R²)`, the film `H = h / c`
//! and the axial coordinate `z / (L/2)` on the half of the bush from its
//! middle to one end.
//!
//! Source of each number: the diameter, the length and the radial clearance
//! are `System_Parameters` of the dataset (the paper's Table I gives 112.08 mm,
//! 50.55 mm and 50 µm +0 −5 µm). The viscosity at 65 °C is the mean of the
//! dataset's fifty stored values. The viscosity multiplier is fitted: it is
//! the one constant fitted to the thirty-four opened runs of the validation
//! protocol in `evidence/research/tribology_sheffield_journal_bearing/`.
//!
//! Left out: the oil inlet hole and its supply pressure, elastic and thermal
//! deformation of the shell and the shaft, the change of viscosity with
//! pressure and along the film, roughness, misalignment, and any motion of the
//! shaft centre.

use genesis_kernel::math;
use crate::normal::NormalOrZero;
use std::fmt;

const PI: f64 = std::f64::consts::PI;

/// Internal diameter of the bush, metres.
pub const BEARING_DIAMETER_M: f64 = 0.112;
/// Length of the bush, metres.
pub const BEARING_LENGTH_M: f64 = 0.05055;
/// Radial clearance between bush and shaft, metres.
pub const RADIAL_CLEARANCE_M: f64 = 45.0e-6;
/// Dynamic viscosity of the oil at the bearing temperature of 65 °C, pascal seconds.
pub const OIL_VISCOSITY_AT_65C_PA_S: f64 = 0.02722;
/// Fitted. The film's viscosity is the viscosity at 65 °C times this.
pub const FILM_VISCOSITY_MULTIPLIER: f64 = 0.8201;

/// The tested envelope of the specimen: the loads and speeds of the fifty
/// compared runs. Stated, not enforced.
pub const LOAD_MIN_N: f64 = 2000.0;
pub const LOAD_MAX_N: f64 = 20000.0;
pub const SPEED_MIN_RPM: f64 = 100.0;
pub const SPEED_MAX_RPM: f64 = 800.0;

/// Density of the oil in the film Reynolds number, kg/m³: a mineral oil near
/// 65 °C. The dataset gives none; it enters only the laminar limit.
pub const OIL_DENSITY_KG_M3: f64 = 860.0;
/// Taylor's number for a narrow gap with the inner cylinder turning: the film
/// is laminar and free of Taylor vortices while `ρ ω R c / μ` is below this
/// times `sqrt(R / c)`. At 800 rpm the film Reynolds number is about 8, and
/// the limit is about 1450, near 143,000 rpm.
pub const TAYLOR_CRITICAL_NUMBER: f64 = 41.2;

/// The shaft position is searched between these eccentricity ratios.
pub const ECCENTRICITY_RATIO_MIN: f64 = 0.05;
pub const ECCENTRICITY_RATIO_MAX: f64 = 0.95;

/// Pressure cells around the bush.
pub const CELLS_AROUND: usize = 180;
/// Pressure cells from the middle of the bush to one end.
pub const CELLS_ALONG_HALF: usize = 20;
/// Over-relaxation weight of one sweep.
pub const RELAXATION_WEIGHT: f64 = 1.8;
/// A film has settled when no cell moved by more than this fraction of the peak pressure in one sweep.
pub const SETTLE_TOLERANCE: f64 = 1.0e-13;
/// The public solver stops one film after this many sweeps.
pub const MAX_SWEEPS: u32 = 20_000;
/// Largest flow imbalance of any cell of an accepted film, as a fraction of the peak pressure.
pub const FLOW_BALANCE_TOLERANCE: f64 = 1.0e-9;
/// The film force must equal the load to this fraction of the load.
pub const LOAD_TOLERANCE: f64 = 1.0e-10;
/// The public solver stops after this many shaft positions.
pub const MAX_SHAFT_POSITIONS: u32 = 60;

/// The published calculation this solver is checked against.
pub const REFERENCE_NAME: &str = "Raimondi and Boyd (1958), A Solution for the Finite Journal Bearing and Its Application to Analysis and Design, ASLE Transactions 1, 159-209: Sommerfeld number of the full journal bearing, as tabulated in the open-source PDSim library";
pub const REFERENCE_URL: &str =
    "https://pdsim.readthedocs.io/en/stable/_modules/PDSim/core/bearings.html";
/// Date the reference table was read.
pub const REFERENCE_RETRIEVED: &str = "2026-10-09";
/// Every Sommerfeld number in the reference test agrees to this fraction.
pub const REFERENCE_TOLERANCE: f64 = 0.015;

/// Why a film was not returned.
#[derive(Clone, Debug, PartialEq)]
pub enum BearingReject {
    /// `load_n` is not above zero: a film carries no load of zero or less.
    LoadNotPositive { load_n: f64 },
    /// `speed_rpm` is not above zero: a shaft at rest makes no film.
    SpeedNotPositive { speed_rpm: f64 },
    /// The film Reynolds number is above [`TAYLOR_CRITICAL_NUMBER`] times the square root of the radius over the clearance: Taylor vortices form, and the film is not the laminar film of the Reynolds equation.
    FilmNotLaminar { film_reynolds_number: f64 },
    /// An input or an intermediate value was NaN or infinite.
    NonFiniteValue,
    /// The film carries the load only at an eccentricity ratio outside [`ECCENTRICITY_RATIO_MIN`] to [`ECCENTRICITY_RATIO_MAX`].
    EccentricityRatioOutOfRange { sommerfeld_number: f64 },
    /// The sweep cap was reached with the pressure still moving.
    FilmNotSettled { sweeps: u32 },
    /// A cell of an otherwise settled film left [`FLOW_BALANCE_TOLERANCE`].
    FlowBalanceBroken { residual: f64 },
    /// The cap on shaft positions was reached with the film force still off the load.
    LoadNotCarried { shaft_positions: u32 },
}

impl fmt::Display for BearingReject {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::LoadNotPositive { load_n } => {
                write!(f, "load {load_n} N is not above zero")
            }
            Self::SpeedNotPositive { speed_rpm } => {
                write!(f, "speed {speed_rpm} rpm is not above zero")
            }
            Self::FilmNotLaminar { film_reynolds_number } => write!(
                f,
                "the film Reynolds number {film_reynolds_number} is above {}, where Taylor vortices form: the film is not laminar",
                laminar_limit()
            ),
            Self::NonFiniteValue => write!(f, "an input or a pressure was not finite"),
            Self::EccentricityRatioOutOfRange { sommerfeld_number } => write!(
                f,
                "Sommerfeld number {sommerfeld_number} is carried only at an eccentricity ratio outside {ECCENTRICITY_RATIO_MIN} to {ECCENTRICITY_RATIO_MAX}"
            ),
            Self::FilmNotSettled { sweeps } => {
                write!(
                    f,
                    "the film pressure was still moving after {sweeps} sweeps"
                )
            }
            Self::FlowBalanceBroken { residual } => write!(
                f,
                "a cell's flow imbalance was {residual} of the peak pressure"
            ),
            Self::LoadNotCarried { shaft_positions } => write!(
                f,
                "the film force was still off the load after {shaft_positions} shaft positions"
            ),
        }
    }
}

impl std::error::Error for BearingReject {}

/// The settled film at the shaft position that carries the load.
#[derive(Clone, Debug, PartialEq)]
pub struct BearingFilm {
    /// `(R/c)² μ N L D / W`, with `N` in revolutions per second and the film's viscosity.
    pub sommerfeld_number: f64,
    /// Shaft offset divided by the radial clearance.
    pub eccentricity_ratio: f64,
    /// Angle from the load line to the line of centres, degrees, from the
    /// film force of the settled field. Compared with no measurement.
    pub attitude_angle_deg: f64,
    /// Thinnest film, `c (1 − ε)`, metres.
    pub minimum_film_thickness_m: f64,
    /// Highest film pressure above ambient in the settled field, pascals.
    /// Compared with no measurement.
    pub peak_film_pressure_pa: f64,
    /// Largest flow imbalance of any cell, as a fraction of the peak pressure.
    pub flow_balance_residual: f64,
    /// Film force minus the load, newtons.
    pub load_residual_n: f64,
    /// Shaft positions tried.
    pub shaft_positions: u32,
    /// Sweeps taken over all shaft positions.
    pub sweeps: u32,
    /// Film pressure above ambient, pascals: [`CELLS_AROUND`] rows from the
    /// widest gap in the direction of rotation, each of [`CELLS_ALONG_HALF`]
    /// samples from the middle of the bush toward one end. No pressure was
    /// compared with measurement.
    pub film_pressure_pa: Vec<f64>,
}

/// The discrete Reynolds equation at one eccentricity ratio.
struct Film {
    around: usize,
    along: usize,
    /// East-face `H³ / Δθ²` of each row. The west face of row `i` is the east face of row `i − 1`.
    east: Vec<f64>,
    /// `(D/L)² H³ / Δz²` of each row.
    axial: Vec<f64>,
    /// `6 ΔH / Δθ` across each row.
    wedge: Vec<f64>,
    /// When set, the first row is held at ambient: the film starts at the widest gap.
    starts_at_widest_gap: bool,
}

impl Film {
    fn new(
        eccentricity_ratio: f64,
        length_over_diameter: f64,
        around: usize,
        along: usize,
        starts_at_widest_gap: bool,
    ) -> Self {
        let d_theta = 2.0 * PI / around as f64;
        let d_z = 1.0 / along as f64;
        let aspect = 1.0 / (length_over_diameter * length_over_diameter);
        let gap = |theta: f64| 1.0 + eccentricity_ratio * math::cos(theta);
        let mut east = Vec::with_capacity(around);
        let mut axial = Vec::with_capacity(around);
        let mut wedge = Vec::with_capacity(around);
        for row in 0..around {
            let theta = row as f64 * d_theta;
            let face = gap(theta + 0.5 * d_theta);
            east.push(face * face * face / (d_theta * d_theta));
            let centre = gap(theta);
            axial.push(aspect * centre * centre * centre / (d_z * d_z));
            wedge.push(6.0 * (face - gap(theta - 0.5 * d_theta)) / d_theta);
        }
        Self {
            around,
            along,
            east,
            axial,
            wedge,
            starts_at_widest_gap,
        }
    }

    /// The pressure that balances the flow of cell `(row, col)` given its neighbours.
    fn balanced(&self, pressure: &[f64], row: usize, col: usize) -> f64 {
        let along = self.along;
        let next = if row + 1 == self.around { 0 } else { row + 1 };
        let previous = if row == 0 { self.around - 1 } else { row - 1 };
        let east = self.east[row];
        let west = self.east[previous];
        let axial = self.axial[row];
        // Symmetric about the middle of the bush; ambient at its end.
        let inward = pressure[row * along + if col == 0 { 1 } else { col - 1 }];
        let outward = if col + 1 == along {
            0.0
        } else {
            pressure[row * along + col + 1]
        };
        (east * pressure[next * along + col]
            + west * pressure[previous * along + col]
            + axial * (inward + outward)
            - self.wedge[row])
            / (east + west + 2.0 * axial)
    }

    /// Sweeps `pressure` until it settles. With `ruptures` the film cannot pull.
    /// Returns the sweeps taken.
    fn settle(
        &self,
        pressure: &mut [f64],
        ruptures: bool,
        sweep_cap: u32,
    ) -> Result<u32, BearingReject> {
        let first_row = if self.starts_at_widest_gap { 1 } else { 0 };
        for sweep in 1..=sweep_cap {
            let mut moved: f64 = 0.0;
            let mut peak: f64 = 0.0;
            for row in first_row..self.around {
                for col in 0..self.along {
                    let index = row * self.along + col;
                    let old = pressure[index];
                    let mut new =
                        old + RELAXATION_WEIGHT * (self.balanced(pressure, row, col) - old);
                    if !new.is_finite() {
                        return Err(BearingReject::NonFiniteValue);
                    }
                    if ruptures && new < 0.0 {
                        new = 0.0;
                    }
                    pressure[index] = new;
                    moved = moved.max((new - old).abs());
                    peak = peak.max(new.abs());
                }
            }
            if moved <= SETTLE_TOLERANCE * peak {
                return Ok(sweep);
            }
        }
        Err(BearingReject::FilmNotSettled { sweeps: sweep_cap })
    }

    /// Largest flow imbalance of any cell as a fraction of the peak pressure.
    /// A cell under pressure must balance. A cell at ambient must be one the
    /// balance would put at or below ambient.
    fn flow_balance_residual(&self, pressure: &[f64]) -> Result<f64, BearingReject> {
        let mut peak: f64 = 0.0;
        for sample in pressure {
            if !sample.is_finite() {
                return Err(BearingReject::NonFiniteValue);
            }
            peak = peak.max(sample.abs());
        }
        if peak == 0.0 {
            return Ok(f64::INFINITY);
        }
        let first_row = if self.starts_at_widest_gap { 1 } else { 0 };
        let mut worst: f64 = 0.0;
        for row in first_row..self.around {
            for col in 0..self.along {
                let held = pressure[row * self.along + col];
                let balanced = self.balanced(pressure, row, col);
                let off = if held > 0.0 {
                    (balanced - held).abs()
                } else if held < 0.0 {
                    f64::INFINITY
                } else {
                    balanced.max(0.0)
                };
                worst = worst.max(off / peak);
            }
        }
        Ok(worst)
    }

    /// The film force along the line of centres and across it, as the
    /// integrals of `P cos θ` and `P sin θ` over the surface.
    fn force(&self, pressure: &[f64]) -> (f64, f64) {
        let d_theta = 2.0 * PI / self.around as f64;
        let d_z = 1.0 / self.along as f64;
        let mut along_centres = 0.0;
        let mut across = 0.0;
        for row in 0..self.around {
            let mut column = 0.5 * pressure[row * self.along];
            for col in 1..self.along {
                column += pressure[row * self.along + col];
            }
            let theta = row as f64 * d_theta;
            along_centres += column * math::cos(theta);
            across += column * math::sin(theta);
        }
        (along_centres * d_theta * d_z, across * d_theta * d_z)
    }
}

/// A film settled and checked at one eccentricity ratio.
struct Settled {
    sommerfeld_number: f64,
    attitude_angle_deg: f64,
    peak: f64,
    flow_balance_residual: f64,
    sweeps: u32,
}

/// Settles the film at one eccentricity ratio, starting from `pressure`, and
/// holds it to the flow balance.
fn settle_and_check(
    film: &Film,
    pressure: &mut [f64],
    sweep_cap: u32,
) -> Result<Settled, BearingReject> {
    let sweeps = film.settle(pressure, true, sweep_cap)?;
    accept(film, pressure, sweeps)
}

/// Holds a pressure field to the flow balance and reads its force.
fn accept(film: &Film, pressure: &[f64], sweeps: u32) -> Result<Settled, BearingReject> {
    let residual = film.flow_balance_residual(pressure)?;
    if !(residual <= FLOW_BALANCE_TOLERANCE) {
        return Err(BearingReject::FlowBalanceBroken { residual });
    }
    let (along_centres, across) = film.force(pressure);
    let size = (along_centres * along_centres + across * across).sqrt();
    if !size.is_finite() || size <= 0.0 {
        return Err(BearingReject::NonFiniteValue);
    }
    let mut peak: f64 = 0.0;
    for sample in pressure {
        peak = peak.max(*sample);
    }
    Ok(Settled {
        sommerfeld_number: 1.0 / (PI * size),
        attitude_angle_deg: math::atan2(across, -along_centres) * 180.0 / PI,
        peak,
        flow_balance_residual: residual,
        sweeps,
    })
}

/// The Sommerfeld number a full bearing of this length-to-diameter ratio
/// carries at one eccentricity ratio. For the reference test.
#[cfg(test)]
fn sommerfeld_at(
    eccentricity_ratio: f64,
    length_over_diameter: f64,
    starts_at_widest_gap: bool,
) -> Result<Settled, BearingReject> {
    let film = Film::new(
        eccentricity_ratio,
        length_over_diameter,
        CELLS_AROUND,
        CELLS_ALONG_HALF,
        starts_at_widest_gap,
    );
    let mut pressure = vec![0.0; CELLS_AROUND * CELLS_ALONG_HALF];
    settle_and_check(&film, &mut pressure, MAX_SWEEPS)
}

/// The film Reynolds number `ρ ω R c / μ` at a speed, with the film's viscosity.
pub fn film_reynolds_number(speed_rpm: f64) -> f64 {
    let viscosity_pa_s = OIL_VISCOSITY_AT_65C_PA_S * FILM_VISCOSITY_MULTIPLIER;
    let omega = 2.0 * PI * speed_rpm / 60.0;
    OIL_DENSITY_KG_M3 * omega * 0.5 * BEARING_DIAMETER_M * RADIAL_CLEARANCE_M / viscosity_pa_s
}

/// The film Reynolds number above which Taylor vortices form:
/// [`TAYLOR_CRITICAL_NUMBER`] times `sqrt(R / c)`.
pub fn laminar_limit() -> f64 {
    TAYLOR_CRITICAL_NUMBER * (0.5 * BEARING_DIAMETER_M / RADIAL_CLEARANCE_M).sqrt()
}

/// The limits the question itself shows, in the order the solver asks them.
/// No film is settled.
pub fn admit(load_n: f64, speed_rpm: f64) -> Result<(), BearingReject> {
    if !load_n.is_finite() || !speed_rpm.is_finite() {
        return Err(BearingReject::NonFiniteValue);
    }
    if !(load_n > 0.0) {
        return Err(BearingReject::LoadNotPositive { load_n });
    }
    if !(speed_rpm > 0.0) {
        return Err(BearingReject::SpeedNotPositive { speed_rpm });
    }
    let film_reynolds_number = film_reynolds_number(speed_rpm);
    if film_reynolds_number > laminar_limit() {
        return Err(BearingReject::FilmNotLaminar { film_reynolds_number });
    }
    Ok(())
}

/// The film of the Sheffield bearing under `load_n` newtons at `speed_rpm`.
///
/// Each value of the answer below the normal range is written as zero
/// with its sign ([`crate::normal`]).
pub fn solve(load_n: f64, speed_rpm: f64) -> Result<BearingFilm, BearingReject> {
    crate::normal::held(solve_as_computed(load_n, speed_rpm))
}

/// The answer as the equations compute it, before a value below the
/// normal range is written as zero.
fn solve_as_computed(load_n: f64, speed_rpm: f64) -> Result<BearingFilm, BearingReject> {
    solve_with(load_n, speed_rpm, MAX_SWEEPS, MAX_SHAFT_POSITIONS)
}

impl NormalOrZero for BearingFilm {
    fn normal_or_zero(&mut self) {
        self.sommerfeld_number.normal_or_zero();
        self.eccentricity_ratio.normal_or_zero();
        self.attitude_angle_deg.normal_or_zero();
        self.minimum_film_thickness_m.normal_or_zero();
        self.peak_film_pressure_pa.normal_or_zero();
        self.flow_balance_residual.normal_or_zero();
        self.load_residual_n.normal_or_zero();
        self.film_pressure_pa.normal_or_zero();
    }
}

fn solve_with(
    load_n: f64,
    speed_rpm: f64,
    sweep_cap: u32,
    position_cap: u32,
) -> Result<BearingFilm, BearingReject> {
    admit(load_n, speed_rpm)?;
    let radius_m = 0.5 * BEARING_DIAMETER_M;
    let radius_over_clearance = radius_m / RADIAL_CLEARANCE_M;
    let viscosity_pa_s = OIL_VISCOSITY_AT_65C_PA_S * FILM_VISCOSITY_MULTIPLIER;
    let turns_per_s = speed_rpm / 60.0;
    let sommerfeld_number = radius_over_clearance
        * radius_over_clearance
        * viscosity_pa_s
        * turns_per_s
        * BEARING_LENGTH_M
        * BEARING_DIAMETER_M
        / load_n;
    let pressure_scale_pa =
        viscosity_pa_s * 2.0 * PI * turns_per_s * radius_over_clearance * radius_over_clearance;
    let target = math::log(sommerfeld_number);
    let length_over_diameter = BEARING_LENGTH_M / BEARING_DIAMETER_M;

    let mut pressure = vec![0.0; CELLS_AROUND * CELLS_ALONG_HALF];
    let mut sweeps: u32 = 0;
    let mut shaft_positions: u32 = 0;
    // The Sommerfeld number carried falls as the eccentricity ratio rises.
    let at = |eccentricity_ratio: f64,
              pressure: &mut Vec<f64>,
              sweeps: &mut u32,
              shaft_positions: &mut u32|
     -> Result<(f64, Settled), BearingReject> {
        let film = Film::new(
            eccentricity_ratio,
            length_over_diameter,
            CELLS_AROUND,
            CELLS_ALONG_HALF,
            false,
        );
        let settled = settle_and_check(&film, pressure, sweep_cap)?;
        *sweeps += settled.sweeps;
        *shaft_positions += 1;
        Ok((math::log(settled.sommerfeld_number) - target, settled))
    };

    let mut low = ECCENTRICITY_RATIO_MIN;
    let mut high = ECCENTRICITY_RATIO_MAX;
    let (mut miss_low, _) = at(low, &mut pressure, &mut sweeps, &mut shaft_positions)?;
    let (mut miss_high, _) = at(high, &mut pressure, &mut sweeps, &mut shaft_positions)?;
    if !(miss_low >= 0.0 && miss_high <= 0.0) {
        return Err(BearingReject::EccentricityRatioOutOfRange { sommerfeld_number });
    }
    // False position with the Illinois weight: the bracket always holds the answer.
    let mut last_side = 0i8;
    loop {
        if shaft_positions >= position_cap {
            return Err(BearingReject::LoadNotCarried { shaft_positions });
        }
        let eccentricity_ratio = (low * miss_high - high * miss_low) / (miss_high - miss_low);
        if !eccentricity_ratio.is_finite() {
            return Err(BearingReject::NonFiniteValue);
        }
        let (miss, settled) = at(
            eccentricity_ratio,
            &mut pressure,
            &mut sweeps,
            &mut shaft_positions,
        )?;
        if miss.abs() <= LOAD_TOLERANCE {
            let film_force_n = load_n * sommerfeld_number / settled.sommerfeld_number;
            for sample in pressure.iter_mut() {
                *sample *= pressure_scale_pa;
            }
            return Ok(BearingFilm {
                sommerfeld_number,
                eccentricity_ratio,
                attitude_angle_deg: settled.attitude_angle_deg,
                minimum_film_thickness_m: RADIAL_CLEARANCE_M * (1.0 - eccentricity_ratio),
                peak_film_pressure_pa: settled.peak * pressure_scale_pa,
                flow_balance_residual: settled.flow_balance_residual,
                load_residual_n: film_force_n - load_n,
                shaft_positions,
                sweeps,
                film_pressure_pa: pressure,
            });
        }
        if miss > 0.0 {
            low = eccentricity_ratio;
            miss_low = miss;
            if last_side == 1 {
                miss_high *= 0.5;
            }
            last_side = 1;
        } else {
            high = eccentricity_ratio;
            miss_high = miss;
            if last_side == -1 {
                miss_low *= 0.5;
            }
            last_side = -1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Sommerfeld number of the full journal bearing from Raimondi and Boyd
    /// (1958), read 2026-10-09 ([`REFERENCE_RETRIEVED`]) from the source of the
    /// open-source PDSim library, function `journal_bearing_design`
    /// (<https://pdsim.readthedocs.io/en/stable/_modules/PDSim/core/bearings.html>),
    /// lists `_hm_c`, `_L_D` and `_S`. Each row is a length-to-diameter ratio;
    /// each pair is the thinnest film over the clearance and the Sommerfeld
    /// number. Raimondi and Boyd start the film at the widest gap. The table
    /// is a calculation, not a measurement. Its last column, an eccentricity
    /// ratio of 0.97, is beyond [`ECCENTRICITY_RATIO_MAX`], where the solver
    /// never returns a film. There this solver is 1.2, 1.9 and 3.2 percent
    /// above the table, and four times the cells gives the same numbers. That
    /// column is printed and not held.
    const RAIMONDI_BOYD: [(f64, [(f64, f64); 7]); 3] = [
        (
            1.0,
            [
                (0.9, 1.33),
                (0.8, 0.631),
                (0.6, 0.264),
                (0.4, 0.121),
                (0.2, 0.0446),
                (0.1, 0.0188),
                (0.03, 0.00474),
            ],
        ),
        (
            0.5,
            [
                (0.9, 4.31),
                (0.8, 2.03),
                (0.6, 0.779),
                (0.4, 0.319),
                (0.2, 0.0923),
                (0.1, 0.0313),
                (0.03, 0.00609),
            ],
        ),
        (
            0.25,
            [
                (0.9, 16.2),
                (0.8, 7.57),
                (0.6, 2.83),
                (0.4, 1.07),
                (0.2, 0.261),
                (0.1, 0.0736),
                (0.03, 0.0101),
            ],
        ),
    ];

    #[test]
    fn raimondi_boyd_full_bearing_at_three_length_ratios() {
        println!("Raimondi and Boyd retrieved {REFERENCE_RETRIEVED} from {REFERENCE_URL}");
        let mut worst: f64 = 0.0;
        for (length_over_diameter, row) in RAIMONDI_BOYD {
            for (film_over_clearance, published) in row {
                let settled = sommerfeld_at(1.0 - film_over_clearance, length_over_diameter, true)
                    .expect("settles");
                let off = (settled.sommerfeld_number - published) / published;
                println!(
                    "L/D {length_over_diameter} h/c {film_over_clearance}: {} against {published}, {off:+.4}",
                    settled.sommerfeld_number
                );
                let inside = 1.0 - film_over_clearance <= ECCENTRICITY_RATIO_MAX;
                if inside {
                    worst = worst.max(off.abs());
                    assert!(off.abs() <= REFERENCE_TOLERANCE, "{off}");
                }
            }
        }
        println!("worst inside the eccentricity range {worst}");
    }

    #[test]
    fn the_film_carries_the_load_and_every_cell_balances() {
        let film = solve(10_000.0, 400.0).expect("solves");
        assert!(film.load_residual_n.abs() <= LOAD_TOLERANCE * 10_000.0 * 1.01);
        assert!(film.flow_balance_residual <= FLOW_BALANCE_TOLERANCE);
        assert!(film.eccentricity_ratio > 0.7 && film.eccentricity_ratio < 0.85);
        assert_eq!(film.film_pressure_pa.len(), CELLS_AROUND * CELLS_ALONG_HALF);
        assert!(film.film_pressure_pa.iter().all(|sample| *sample >= 0.0));
        // The force read back from the returned field in pascals is the load.
        let d_theta = 2.0 * PI / CELLS_AROUND as f64;
        let mut along_centres = 0.0;
        let mut across = 0.0;
        for row in 0..CELLS_AROUND {
            let mut column = 0.5 * film.film_pressure_pa[row * CELLS_ALONG_HALF];
            for col in 1..CELLS_ALONG_HALF {
                column += film.film_pressure_pa[row * CELLS_ALONG_HALF + col];
            }
            along_centres += column * (row as f64 * d_theta).cos();
            across += column * (row as f64 * d_theta).sin();
        }
        let area = 0.5 * BEARING_DIAMETER_M * d_theta * BEARING_LENGTH_M / CELLS_ALONG_HALF as f64;
        let force = (along_centres * along_centres + across * across).sqrt() * area;
        assert!((force - 10_000.0).abs() < 1.0e-6, "{force}");
        // The mean pressure on the projected area sits below the field's peak.
        let projected = 10_000.0 / (BEARING_DIAMETER_M * BEARING_LENGTH_M);
        let peak = film.film_pressure_pa.iter().copied().fold(0.0, f64::max);
        assert!(peak > projected);
        assert!(peak < 10.0 * projected);
        // The peak returned is the field's own, and the attitude angle is
        // the direction of the film force read back from it.
        assert_eq!(film.peak_film_pressure_pa, peak);
        let attitude = across.atan2(-along_centres) * 180.0 / PI;
        assert!((film.attitude_angle_deg - attitude).abs() < 1.0e-9, "{} {attitude}", film.attitude_angle_deg);
        assert!(film.attitude_angle_deg > 0.0 && film.attitude_angle_deg < 90.0);
    }

    #[test]
    fn the_same_question_gives_the_same_bits() {
        let first = solve(6_000.0, 200.0).expect("solves");
        let second = solve(6_000.0, 200.0).expect("solves");
        assert_eq!(
            first.eccentricity_ratio.to_bits(),
            second.eccentricity_ratio.to_bits()
        );
        assert_eq!(first.sweeps, second.sweeps);
        for (left, right) in first.film_pressure_pa.iter().zip(&second.film_pressure_pa) {
            assert_eq!(left.to_bits(), right.to_bits());
        }
    }

    #[test]
    fn more_load_thins_the_film_and_more_speed_thickens_it() {
        let light = solve(4_000.0, 400.0).expect("solves");
        let heavy = solve(16_000.0, 400.0).expect("solves");
        let fast = solve(16_000.0, 800.0).expect("solves");
        assert!(heavy.minimum_film_thickness_m < light.minimum_film_thickness_m);
        assert!(fast.minimum_film_thickness_m > heavy.minimum_film_thickness_m);
        // A heavier load moves the shaft closer to the load line.
        assert!(heavy.attitude_angle_deg < light.attitude_angle_deg);
        assert!(heavy.peak_film_pressure_pa > light.peak_film_pressure_pa);
        // The film depends on speed over load only: 4 kN at 400 rpm is 8 kN at 800 rpm.
        let twin = solve(8_000.0, 800.0).expect("solves");
        assert!((twin.eccentricity_ratio - light.eccentricity_ratio).abs() < 1.0e-9);
    }

    #[test]
    fn a_question_past_a_physical_limit_is_named() {
        for load_n in [0.0, -0.0, -10_000.0] {
            assert_eq!(solve(load_n, 400.0), Err(BearingReject::LoadNotPositive { load_n }));
        }
        for speed_rpm in [0.0, -400.0] {
            assert_eq!(solve(10_000.0, speed_rpm), Err(BearingReject::SpeedNotPositive { speed_rpm }));
        }
        assert_eq!(solve(f64::NAN, 400.0), Err(BearingReject::NonFiniteValue));
        assert_eq!(
            solve(10_000.0, f64::INFINITY),
            Err(BearingReject::NonFiniteValue)
        );
        // The laminar limit: about 1450 in film Reynolds number, near
        // 143,000 rpm; at 800 rpm the film Reynolds number is about 8.
        assert!((film_reynolds_number(800.0) - 8.1).abs() < 0.1, "{}", film_reynolds_number(800.0));
        assert!((laminar_limit() - 1453.4).abs() < 0.1, "{}", laminar_limit());
        match solve(1.0e9, 150_000.0) {
            Err(BearingReject::FilmNotLaminar { film_reynolds_number }) => assert!(film_reynolds_number > laminar_limit()),
            other => panic!("{other:?}"),
        }
        assert_eq!(admit(1.0e9, 140_000.0), Ok(()));
        assert_eq!(
            BearingReject::EccentricityRatioOutOfRange { sommerfeld_number: 5.0 }.to_string(),
            "Sommerfeld number 5 is carried only at an eccentricity ratio outside 0.05 to 0.95"
        );
    }

    /// Past the tested envelope, where engine 0.8.0 refused by name: the same
    /// film at the Sommerfeld number the load and speed give, every cell
    /// balanced and the load carried, until the shaft would sit outside the
    /// eccentricity ratios searched. A load and speed with the same ratio as
    /// a tested run give the same film.
    #[test]
    fn past_the_tested_envelope_the_film_carries_the_load_to_the_eccentricity_limits() {
        for (load_n, speed_rpm) in [(1_000.0, 400.0), (40_000.0, 800.0), (30_000.0, 200.0), (2_000.0, 1_600.0), (100.0, 100.0), (500_000.0, 100_000.0)] {
            let film = solve(load_n, speed_rpm).unwrap_or_else(|reject| panic!("{load_n} {speed_rpm}: {reject}"));
            assert!(film.flow_balance_residual <= FLOW_BALANCE_TOLERANCE);
            assert!(film.load_residual_n.abs() <= LOAD_TOLERANCE * load_n * 1.01);
            assert!(film.eccentricity_ratio > ECCENTRICITY_RATIO_MIN && film.eccentricity_ratio < ECCENTRICITY_RATIO_MAX);
        }
        let tested = solve(4_000.0, 400.0).expect("solves");
        let past = solve(40_000.0, 4_000.0).expect("solves");
        assert!((past.eccentricity_ratio - tested.eccentricity_ratio).abs() < 1.0e-9);
        // Too heavy for the film: the shaft would touch; too light: it would
        // sit at the centre, where the steady position loses its meaning.
        assert!(matches!(solve(1.0e6, 100.0), Err(BearingReject::EccentricityRatioOutOfRange { .. })));
        assert!(matches!(solve(10.0, 800.0), Err(BearingReject::EccentricityRatioOutOfRange { .. })));
    }

    #[test]
    fn the_corners_of_the_envelope_solve_inside_the_eccentricity_range() {
        let thinnest = solve(LOAD_MAX_N, SPEED_MIN_RPM).expect("solves");
        let thickest = solve(LOAD_MIN_N, SPEED_MAX_RPM).expect("solves");
        assert!(thinnest.eccentricity_ratio < ECCENTRICITY_RATIO_MAX);
        assert!(thickest.eccentricity_ratio > ECCENTRICITY_RATIO_MIN);
        assert!(thinnest.minimum_film_thickness_m > 2.0e-6);
        assert!(thickest.minimum_film_thickness_m < 35.0e-6);
    }

    /// Attack: a film stopped before it has settled is named, not returned.
    #[test]
    fn a_film_stopped_early_is_named() {
        assert_eq!(
            solve_with(10_000.0, 400.0, 50, MAX_SHAFT_POSITIONS),
            Err(BearingReject::FilmNotSettled { sweeps: 50 })
        );
    }

    /// Attack: a search for the shaft position stopped early is named.
    #[test]
    fn a_shaft_search_stopped_early_is_named() {
        assert_eq!(
            solve_with(10_000.0, 400.0, MAX_SWEEPS, 3),
            Err(BearingReject::LoadNotCarried { shaft_positions: 3 })
        );
    }

    /// Attack: the wrong rupture method. The half-Sommerfeld film is the full
    /// film with its negative half cut off afterwards. It settles, it is never
    /// negative, and it does not conserve oil where it was cut.
    #[test]
    fn a_film_cut_off_after_the_fact_breaks_the_flow_balance() {
        let film = Film::new(0.6, 0.5, CELLS_AROUND, CELLS_ALONG_HALF, false);
        let mut pressure = vec![0.0; CELLS_AROUND * CELLS_ALONG_HALF];
        let sweeps = film
            .settle(&mut pressure, false, MAX_SWEEPS)
            .expect("the full film settles");
        assert!(pressure.iter().any(|sample| *sample < 0.0));
        for sample in pressure.iter_mut() {
            if *sample < 0.0 {
                *sample = 0.0;
            }
        }
        match accept(&film, &pressure, sweeps) {
            Err(BearingReject::FlowBalanceBroken { residual }) => {
                assert!(residual > 1.0e-3, "{residual}")
            }
            other => panic!(
                "accepted a film cut off after the fact: {:?}",
                other.is_ok()
            ),
        }
    }

    /// Attack: a falsified field. A settled film with every pressure raised by
    /// one part in a million, a single cell raised, a cell pulled below
    /// ambient, and a cell that is not a number are each named.
    #[test]
    fn a_falsified_film_is_named() {
        let film = Film::new(0.6, 0.5, CELLS_AROUND, CELLS_ALONG_HALF, false);
        let mut pressure = vec![0.0; CELLS_AROUND * CELLS_ALONG_HALF];
        let sweeps = film
            .settle(&mut pressure, true, MAX_SWEEPS)
            .expect("settles");
        assert!(accept(&film, &pressure, sweeps).is_ok());

        let peak_at = (0..pressure.len())
            .max_by(|a, b| pressure[*a].total_cmp(&pressure[*b]))
            .expect("cells");
        let mut raised = pressure.clone();
        raised[peak_at] *= 1.000_001;
        assert!(matches!(
            accept(&film, &raised, sweeps),
            Err(BearingReject::FlowBalanceBroken { .. })
        ));

        let ambient_at = pressure
            .iter()
            .position(|sample| *sample == 0.0)
            .expect("a ruptured cell");
        let mut pulled = pressure.clone();
        pulled[ambient_at] = -1.0e-6;
        assert!(matches!(
            accept(&film, &pulled, sweeps),
            Err(BearingReject::FlowBalanceBroken { .. })
        ));

        let mut pushed = pressure.clone();
        // A cell deep in the ruptured region, given pressure the wedge does not supply.
        let deep = (CELLS_AROUND * 3 / 4) * CELLS_ALONG_HALF;
        assert_eq!(pushed[deep], 0.0);
        pushed[deep] = 1.0e-3;
        assert!(matches!(
            accept(&film, &pushed, sweeps),
            Err(BearingReject::FlowBalanceBroken { .. })
        ));

        let mut broken = pressure.clone();
        broken[peak_at] = f64::NAN;
        assert_eq!(
            accept(&film, &broken, sweeps).err(),
            Some(BearingReject::NonFiniteValue)
        );

        let empty = vec![0.0; pressure.len()];
        assert!(matches!(
            accept(&film, &empty, sweeps),
            Err(BearingReject::FlowBalanceBroken { .. })
        ));
    }

    /// The grid is fine enough: twice the cells each way moves the Sommerfeld
    /// number carried by less than half a percent across the range.
    #[test]
    fn twice_the_cells_moves_the_answer_by_less_than_half_a_percent() {
        for eccentricity_ratio in [0.3, 0.7, 0.94] {
            let coarse = sommerfeld_at(
                eccentricity_ratio,
                BEARING_LENGTH_M / BEARING_DIAMETER_M,
                false,
            )
            .expect("settles");
            let film = Film::new(
                eccentricity_ratio,
                BEARING_LENGTH_M / BEARING_DIAMETER_M,
                2 * CELLS_AROUND,
                2 * CELLS_ALONG_HALF,
                false,
            );
            let mut pressure = vec![0.0; 4 * CELLS_AROUND * CELLS_ALONG_HALF];
            let fine = settle_and_check(&film, &mut pressure, 4 * MAX_SWEEPS).expect("settles");
            let off = (coarse.sommerfeld_number - fine.sommerfeld_number) / fine.sommerfeld_number;
            assert!(off.abs() < 0.005, "{eccentricity_ratio}: {off}");
        }
    }
}
