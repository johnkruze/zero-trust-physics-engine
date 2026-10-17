#![no_implicit_prelude]
//! The nitrogen charge of one hydraulic diaphragm accumulator, HYDAC
//! SBO450-0.6 with no foam insert, as a piston pushes oil into it and draws
//! it out: the accumulator on the test rig of the Chair of Fluid Systems,
//! Technische Universität Darmstadt, March 2024.
//!
//! The specimen and its data. Rexer and Lefemmine, "accumulator 600ml empty",
//! Zenodo records 11265458 and 15478932 (precharge 40 bar, load 60 bar),
//! 11265391 (20 / 40), 11265436 (20 / 60), 11265491 and 11265583 (40 / 80)
//! and 11265625 (40 / 120), CC BY 4.0: 399 runs. In each run a single-acting
//! hydraulic cylinder on a servo-hydraulic test machine moves in a sine of
//! 2 mm or 5 mm at one frequency from 0.01 to 20 Hz, and a file gives the
//! piston's deflection, the force, the gas pressure, the oil pressure and
//! three temperatures at 1 000 samples a second. The record in the research
//! folder is `evidence/research/accumulator_darmstadt_sbo450/`.
//!
//! What the files do not give: the bore of the cylinder, the true gas volume
//! of the vessel, and the response time of the gas temperature sensor. The
//! precharge is absolute: the metadata give both pressure sensors a range of
//! 1 to 201 bar with an offset of 1 bar, and the records at 20 bar precharge
//! agree with the card read that way and not read as gauge.
//!
//! This card was scored on 2026-10-09, failed, was scored a second time by
//! another party (`evidence/verification/second_party/V-39/`), and was amended the
//! same day. The amendment is a correction made after every one of the 399
//! runs had been opened. It is not a new test of the card.
//!
//! The registered comparison, which stands. Held back: all 285 runs of the
//! five records other than the two at 40 / 60 bar. The lines were fixed
//! beforehand (protocol SHA-256
//! 721d88a5264b0e15342824290e6305db1df27fab76011333b49d5ed253b38323) and the
//! comparison failed: two of six were met. The error of the pressure swing
//! per unit of piston travel was 0.93 percent in the median against a line of
//! 2.0, and 13.96 percent at most against 5.0; in the record at 120 bar the
//! median was 2.72 percent against 2.0; three runs at 120 bar that passed the
//! ceiling of 130 bar the card then had were refused. The error of the
//! pressure's lead over the piston was 0.27 degrees in the median against
//! 0.5, which every baseline also met.
//!
//! What the second scoring found and this amendment does.
//!
//! The geometry. The card as scored took its piston area, 19.2157 cm², from
//! the mean of force over oil pressure at one pressure. The force carries an
//! offset: every file's metadata give the force transducer −0.202 kN, and
//! over the seven records the force is 19.569 cm² times the oil gauge
//! pressure less 211 N, a bore of 49.92 mm. The bore is taken as 50 mm,
//! [`PISTON_BORE_M`]; no source prints it. The gas pressure depends on the
//! area and the vessel volume only through their ratio, so the fit of the
//! opened runs fixed the ratio, [`FITTED_AREA_OVER_VOLUME_PER_M`], and not
//! the volume; at a 50 mm bore that ratio is a vessel of 0.58879 l,
//! [`VESSEL_GAS_VOLUME_M3`], where the catalogue gives 0.6 l and 36 cubic
//! inches (0.58993 l). A run asked with the charge of the new vessel has the
//! gas pressure and temperature of the same run as it was scored, to
//! rounding; its gas volume, gas mass, force, work and heat are 2.18 percent
//! larger. What is fitted to this accumulator's own runs at 40 bar precharge
//! and 60 bar is that ratio and [`THERMAL_TIME_CONSTANT_S`]. The nitrogen
//! constants are from Span et al. 2000 and CODATA and are not fitted.
//!
//! The compared range. The card holds its own marks on the measured runs
//! (median error of the swing at most 2 percent in each record, every run
//! within 5 percent and 3 degrees) at the two measured charges, each at the
//! gas pressures it was measured at, and for motion at 0.2 Hz and faster:
//! 37 to 43 and 55 to 67 bar for the 20 bar charge
//! ([`PRESSURE_BANDS_20_BAR_PA`]), 57 to 64 and 75 to 86 bar for the 40 bar
//! charge ([`PRESSURE_BANDS_40_BAR_PA`]), each the whole bars outside the
//! gas pressures measured in one record; the gas mass at those two charges,
//! each within a thousandth; the wall at room temperature, 293.4 to 293.7 K;
//! the gas from 285.9 to 301.8 K; the piston to 0.7 m/s. At 109 to 132 bar,
//! the record at 40 bar precharge and 120 bar load, the card as scored was
//! compared and missed: the error of the swing was 2.72 percent in the
//! median against the line of 2.0, and three runs above 130 bar went
//! unanswered. Through engine 0.9.0 each edge of that range was a refusal.
//! It is now the range the card was compared on: the Peng-Robinson gas, the
//! one time constant and the ledger are the same outside it, and a step
//! there lands with [`FLAG_OUTSIDE_COMPARED_RUNS`]. The flag that marked
//! pressures outside the fitted 57 to 63 bar is gone: it followed neither
//! the runs that failed nor the ones that held.
//!
//! What stops the card is physical. The gas mass must be above zero
//! ([`StepReject::GasMassNotPositive`]) and the wall above absolute zero
//! ([`StepReject::WallTemperatureNotPositive`]). At or below nitrogen's
//! critical temperature, 126.192 K, the gas can condense and one phase of
//! the equation of state is not this charge
//! ([`StepReject::GasBelowCriticalTemperature`]). Above the vessel's rated
//! working pressure, 330 bar ([`PRESSURE_RATED_PA`]), the vessel is outside
//! its rating ([`StepReject::PressureAboveRated`]). A gas volume larger than
//! the vessel's leaves no oil to draw out
//! ([`StepReject::OilSideEmpty`]), which also ends the card below the
//! precharge. The card is never told a frequency, so slow motion is refused
//! on what the card carries, the share of heat in the work
//! ([`StepReject::HeatShareAboveCard`], below).
//!
//! The amended card on the measured runs, run after the fact on 2026-10-09
//! (amendment SHA-256
//! ba073e2a1c070aa1282bcea8b330b765a833c53b0ab63367a61fb46722a68bb7). Of the
//! 285 formerly held-back runs it answers 196 and refuses 89: the 57 at
//! 120 bar by the pressure, before the first step, and the 32 at 0.1 Hz and
//! under by the share of heat, part way. On the 196 the error of the swing
//! is 0.49 percent in the median, 1.37 in the worst record and 4.05 at most,
//! and the error of the lead 0.21 degrees in the median and 1.79 at most.
//! Those are not lines met: every run was open when the range was drawn.
//! Gas pressure and temperature equal the registered run's in every frame
//! both landed; volume, mass, force, work and heat are 1.021818 times it.
//!
//! What is left out. The oil, the diaphragm, the line between the cylinder
//! and the vessel, and any leak: the gas mass never changes and the gas
//! volume changes only by the piston's area times its travel. The wall is at
//! one temperature for a whole run. The gas has one temperature. The
//! exchange of heat is one time constant, and that form is known to fail:
//! fitted to each record it runs from 8 s to 18 s, within one record it falls
//! threefold from 0.01 Hz to 1 Hz, and the group that made these
//! measurements has published, for two other HYDAC diaphragm accumulators,
//! that "Newton's law is incapable of representing this physical phenomenon"
//! (Hartig, Depp, Rexer and Pelz, arXiv:2012.06526). No orifice and no flow
//! of gas: this card is not a vessel charged or emptied through a nozzle, and
//! choked flow does not arise.
//!
//! The equations. Nitrogen of mass `m` in a volume `V`, with `v = V / m`,
//! piston speed `s` (positive pushes oil in and compresses the gas), wall
//! temperature `T_w`:
//!
//! ```text
//! p      = R T / (v − b) − a α(T) / (v² + 2 b v − b²)       Peng-Robinson
//! α(T)   = (1 + κ (1 − √(T / T_c)))²
//! c_v    = c_v0 + T a α''(T) L(v)
//! u      = c_v0 T − a (α − T α') L(v)
//! L(v)   = ln((v + (1 + √2) b) / (v + (1 − √2) b)) / (2 √2 b)
//! dV/dt  = −A s
//! m c_v dT/dt = −T (∂p/∂T)_v dV/dt − (m c_v0 / τ)(T − T_w)
//! dW/dt  = −p dV/dt                 work of the oil on the gas
//! dQ/dt  = (m c_v0 / τ)(T − T_w)    heat from the gas to the wall
//! defect = m u(T, v) − m u(start) − W + Q
//! M      = M₀ + Σ |ΔW|,  H = H₀ + Σ |ΔQ|   M₀, H₀ carried into the start
//! share  = H / max(M, 100 J)
//! ```
//!
//! One step is classical fourth-order Runge-Kutta over `T`, `V`, `W` and `Q`
//! with the speed held. The defect is held to
//! [`RESIDUAL_CONSERVATION_LIMIT`] of the ledger scale, the pressure ceiling
//! times the vessel volume, about 5.06 kJ. A positive defect is energy from
//! nowhere and is [`StepReject::SpontaneousGeneration`]; a negative one is
//! [`StepReject::FirstLawBreach`]. The ledger closes for any time constant
//! and any vessel volume: it shows that the pressure, the heat capacity and
//! the energy are one equation of state and that the step integrates them,
//! and it does not check that the constants are this accumulator's. The gas
//! mass is a number the step never changes; that it is conserved is not a
//! finding.
//!
//! The share of heat in the work. `ΔW` and `ΔQ` are the work and the heat of
//! one step, and `M` and `H` are the sums of their absolute values over the
//! motion so far. A start carries the sums of the motion before it, `M₀` and
//! `H₀`: zero for gas that has not moved, and the `work_moved_j` and
//! `heat_moved_j` of the returned state when a run or a single step is
//! carried on from it. So a motion asked as one run, as runs carried on from
//! each other's returned states, or as single steps handed back is weighed in
//! the same way. A start whose sums are already above the share is refused as
//! [`StepReject::HeatShareAboveCard`]; a sum below zero is
//! [`StepReject::NegativeMovedSum`]. The card cannot see motion before its
//! start, so it holds the sums it is given and cannot confirm them. The time,
//! the work `W`, the heat `Q` and the energy at the start are the anchor of
//! the ledger and begin again at each start; the ledger's conservation check
//! is held from each start, and no limit of the card's range depends on them.
//! A step lands only if `H` is at most
//! [`HEAT_SHARE_LIMIT`], 0.065, of the larger of `M` and
//! [`HEAT_SHARE_WORK_FLOOR_J`], 100 J; otherwise it is
//! [`StepReject::HeatShareAboveCard`] and the state and the frame stay at the
//! last step that landed. Over one period of a sine the share is 0.05 at
//! 0.2 Hz, 0.10 at 0.1 Hz and 0.8 at 0.01 Hz, nearly the same at every
//! pressure, charge and amplitude measured. The floor is what gives the share
//! a meaning before the oil has done work: until `M` passes 100 J the heat is
//! weighed against 100 J, so a run may exchange 6.5 J with the wall before
//! any work is done. A piston at rest with the gas at the wall's temperature
//! exchanges nothing and may rest for any time. A piston at rest with the gas
//! off the wall's temperature relaxes with the one time constant, which no
//! measurement here tests (every run is a sine) and which the slow runs show
//! to be wrong by a factor of two to three; from the start of a run such a
//! rest is inside the card for its first 6.5 J, about 0.3 K of the 40 bar
//! charge and 0.6 K of the 20 bar charge. The sums run over the whole motion
//! so far: a rest that follows motion is weighed against all the work before
//! it, and is refused later than the same rest alone, or not at all.
//!
//! [`AccumulatorSbo450::gate_dt_s`] is the step ceiling, the time constant
//! divided by 50, 0.3504 s. [`AccumulatorSbo450::floor_dt_s`] is the ceiling
//! divided by 2^20, 3.34e-7 s. The public step applies neither. The gateway
//! refuses a step outside them. The step itself refuses piston travel above
//! [`PISTON_TRAVEL_PER_STEP_M`], 1 mm, which is at most 1.5 percent of the
//! smallest gas volume the card holds, the 0.136 l of the 20 bar charge at
//! 86 bar.
//!
//! `core` only: no heap and no standard prelude. The card, the state and the
//! load are `Copy`. An accepted step writes time, gas pressure, gas
//! temperature, gas volume, the work, the heat, the gas mass, the force of
//! the gas pressure on the piston area and the conservation residual into
//! [`LastStateFrame64`], with [`FLAG_OUTSIDE_COMPARED_RUNS`]. The 16-byte
//! frame proof is not touched.

extern crate core;

use core::prelude::v1::*;

use crate::frame::LastStateFrame64;
use crate::normal::normal_or_zero;

/// Molar gas constant, J/(mol K). CODATA 2018, exact.
pub const GAS_CONSTANT_J_MOL_K: f64 = 8.314_462_618;
/// Molar mass of nitrogen N2, kg/mol. Span et al. 2000.
pub const NITROGEN_MOLAR_MASS_KG_MOL: f64 = 0.028_013_4;
/// Critical temperature of nitrogen, K. Span et al. 2000.
pub const CRITICAL_TEMPERATURE_K: f64 = 126.192;
/// Critical pressure of nitrogen, Pa. Span et al. 2000.
pub const CRITICAL_PRESSURE_PA: f64 = 3.3958e6;
/// Acentric factor of nitrogen. Span et al. 2000.
pub const ACENTRIC_FACTOR: f64 = 0.0372;
/// Heat capacity of nitrogen at constant volume in the limit of zero
/// pressure at 293 K, J/(mol K): 29.12 less the gas constant. NIST WebBook.
pub const IDEAL_HEAT_CAPACITY_J_MOL_K: f64 = 20.80;
/// Bore of the cylinder whose piston moves the oil, m: 50 mm. No source
/// prints it. Measured force against measured oil gauge pressure in the runs
/// at or under 1 Hz of all seven records is a line of slope 19.569 cm², a
/// bore of 49.92 mm, with an intercept of −211 N beside the −0.202 kN that
/// every file's metadata give as the force transducer's offset; 50 mm is the
/// standard bore that slope is 0.33 percent under.
pub const PISTON_BORE_M: f64 = 0.05;
/// Area of the piston that moves the oil, m²: `π/4` times the square of
/// [`PISTON_BORE_M`], 19.635 cm².
pub const PISTON_AREA_M2: f64 = 1.963_495_408_493_621e-3;
/// Piston area over the vessel's gas volume as fitted to this accumulator's
/// opened runs, 1/m: the 1.92157e-3 m² over 5.7622e-4 m³ of the card as it
/// was scored. The gas pressure fixes this ratio and neither number alone.
pub const FITTED_AREA_OVER_VOLUME_PER_M: f64 = 1.921_57e-3 / 5.762_2e-4;
/// Gas volume of the vessel with no oil in it, m³: [`PISTON_AREA_M2`] over
/// [`FITTED_AREA_OVER_VOLUME_PER_M`], 0.58879 l. The catalogue gives 0.6 l
/// and 36 cubic inches, which is 0.58993 l.
pub const VESSEL_GAS_VOLUME_M3: f64 = 5.887_921_461_524_661e-4;
/// Time constant of the exchange of heat between the gas and the wall at
/// constant volume, s. Fitted to this accumulator's opened runs.
pub const THERMAL_TIME_CONSTANT_S: f64 = 17.52;
/// Lowest gas pressure of the compared range, Pa: 37 bar. The lowest gas
/// pressure measured in the four conditions in which the card met its marks
/// is 37.76 bar (20 bar precharge, 40 bar load, 5 mm); 37 bar is the whole
/// bar under it. An edge of the compared range, not a limit.
pub const PRESSURE_FLOOR_PA: f64 = 3.7e6;
/// Highest gas pressure of the compared range, Pa: 86 bar. The highest gas
/// pressure measured in the four conditions in which the card met its marks
/// is 85.56 bar (40 bar precharge, 80 bar load, 5 mm); 86 bar is the whole
/// bar over it. The next measured condition, 109 to 132 bar, missed its mark
/// at every frequency, and between the two there is no measurement. An edge
/// of the compared range, not a limit; it also sets the ledger's scale.
pub const PRESSURE_CEILING_PA: f64 = 8.6e6;
/// The vessel's rated maximum working pressure, Pa: 330 bar. HYDAC,
/// Diaphragm Accumulators SBO (USA catalogue): its dimension table gives
/// 4700 psi (330 bar) for the type SBO450-0.6, while the two 0.6 l parts
/// listed, 2121077 and 3042074, are rated 3600 psi (250 bar). Which version
/// the rig carried is not on disk; the higher rating of the type is taken
/// (John Kruze, 14 October 2026), so that no part's lower rating stops a
/// run the type itself is rated for.
pub const PRESSURE_RATED_PA: f64 = 3.3e7;
/// The gas pressures compared with measurement for the 20 bar charge, Pa:
/// 37 to 43 bar and 55 to 67 bar. The answered runs of that charge measured
/// 37.76 to 42.71 bar (20 / 40) and 55.26 to 66.37 bar (20 / 60); each band
/// is the whole bars outside one record. Between and beyond them nothing of
/// this charge was measured.
pub const PRESSURE_BANDS_20_BAR_PA: [[f64; 2]; 2] = [[3.7e6, 4.3e6], [5.5e6, 6.7e6]];
/// The gas pressures compared with measurement for the 40 bar charge, Pa:
/// 57 to 64 bar and 75 to 86 bar. The answered runs of that charge measured
/// 57.36 to 63.14 bar (40 / 60) and 75.31 to 85.55 bar (40 / 80), drawn by the
/// same rule.
pub const PRESSURE_BANDS_40_BAR_PA: [[f64; 2]; 2] = [[5.7e6, 6.4e6], [7.5e6, 8.6e6]];
/// Lowest gas temperature of the compared range, K. The compared runs
/// reached 285.97 to 301.78 K over every step they landed; the range is the
/// next tenth of a kelvin outside that.
pub const GAS_TEMPERATURE_FLOOR_K: f64 = 285.9;
/// Highest gas temperature of the compared range, K.
pub const GAS_TEMPERATURE_CEILING_K: f64 = 301.8;
/// Lowest wall temperature of the compared range, K. The wall of every compared
/// run, the mean room temperature of its record, was 293.46 to 293.60 K; the
/// range is the next tenth of a kelvin outside that. No other wall
/// temperature was measured.
pub const WALL_TEMPERATURE_FLOOR_K: f64 = 293.4;
/// Highest wall temperature of the compared range, K.
pub const WALL_TEMPERATURE_CEILING_K: f64 = 293.7;
/// The charge of a 20 bar precharge, kg: the vessel's gas volume full of
/// nitrogen at 20 bar absolute and 293.15 K by the card's own equation of
/// state. One of the two charges measured.
pub const CHARGE_20_BAR_KG: f64 = 0.013_652_934_755_883_66;
/// The charge of a 40 bar precharge, kg: the same at 40 bar absolute. The
/// other charge measured.
pub const CHARGE_40_BAR_KG: f64 = 0.027_467_339_693_405_615;
/// Smallest gas mass compared as the 20 bar charge, kg: a thousandth
/// under [`CHARGE_20_BAR_KG`]. No charge but the two was measured; the band
/// lets a charge typed to four figures in, and the pressure swing moves by
/// the same share as the mass, a tenth of a percent at most.
pub const GAS_MASS_20_BAR_FLOOR_KG: f64 = 0.013_639;
/// Largest gas mass compared as the 20 bar charge, kg.
pub const GAS_MASS_20_BAR_CEILING_KG: f64 = 0.013_667;
/// Smallest gas mass compared as the 40 bar charge, kg.
pub const GAS_MASS_40_BAR_FLOOR_KG: f64 = 0.027_44;
/// Largest gas mass compared as the 40 bar charge, kg.
pub const GAS_MASS_40_BAR_CEILING_KG: f64 = 0.027_495;
/// Largest piston speed compared, m/s, either way. The fastest
/// compared run, 5 mm at 20 Hz, reached 0.69999 m/s as the mean over a step.
pub const PISTON_SPEED_CEILING_M_S: f64 = 0.7;
/// Largest piston travel in one step, m: 1 mm.
pub const PISTON_TRAVEL_PER_STEP_M: f64 = 1.0e-3;
/// Allowance on the four limits the card works out itself: the travel
/// `|s| dt`, the pressure and the volume worked back from the starting
/// pressure, and the heat set against the work. A relative 1e-12, about
/// 4 500 units in the last place: a start, a step or a run stated or landing
/// exactly at a limit is inside it.
pub const COMPUTED_LIMIT_ALLOWANCE: f64 = 1.0e-12;
/// Largest share of heat in the work that the card answers: 0.065. The sums
/// are `H = Σ|ΔQ|` and `M = Σ|ΔW|` over the steps of the run so far, and a
/// step lands only if `H ≤ 0.065 · max(M, 100 J)`. Run as they were scored,
/// the measured runs at 0.2 Hz, the slowest that met the card's marks, reach
/// 0.062 at most, and those at 0.1 Hz, which did not, 0.104 at least; 0.065
/// is the next two-hundredth over what held.
pub const HEAT_SHARE_LIMIT: f64 = 0.065;
/// The work from which the heat share is counted, J: 100. Until the oil has
/// done this much work on the gas, either way, the heat is set against this
/// much. It is a little over the work of a quarter of the largest measured
/// swing inside the card (91 J: 80 bar, the 5 mm run at 18 Hz), so a swing
/// that starts from rest at its outermost point is not refused for its first
/// quarter, and it is under the work of the smallest measured run (125 J: two
/// periods of 2 mm at 40 bar), so every measured run was weighed on its own
/// work.
pub const HEAT_SHARE_WORK_FLOOR_J: f64 = 100.0;
/// Step ceiling as a share of the thermal time constant: one fiftieth.
pub const GATE_SHARE_OF_TIME_CONSTANT: f64 = 0.02;
/// Step ceiling divided by step floor: 2^20.
pub const GATE_OVER_FLOOR: f64 = 1_048_576.0;
/// `|defect| / (PRESSURE_CEILING_PA · VESSEL_GAS_VOLUME_M3)` must stay at or
/// under this.
pub const RESIDUAL_CONSERVATION_LIMIT: f64 = 1.0e-9;

/// The step lies outside the range compared with measurement: a gas mass
/// other than the two charges, a wall outside 293.4 to 293.7 K, a piston
/// faster than 0.7 m/s, a gas temperature outside 285.9 to 301.8 K, or a gas
/// pressure outside its charge's compared bands, 109 to 132 bar among them,
/// where the card as scored missed (2.72 percent median against 2.0). The
/// step lands.
pub const FLAG_OUTSIDE_COMPARED_RUNS: u64 = 1 << 0;

/// Why a start was refused or a step did not advance the gas or the frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StepReject {
    /// The piston speed, a starting value or `dt` was NaN or infinite.
    NonFiniteInput,
    /// The incoming state, or the trial step, was not finite.
    NonFiniteState,
    /// `dt` was zero or negative.
    NonPositiveDt,
    /// A recorded constant is not this accumulator's.
    InvalidAccumulator,
    /// The gas mass is zero or less.
    GasMassNotPositive,
    /// The wall temperature is at or below absolute zero.
    WallTemperatureNotPositive,
    /// The piston would travel more than 1 mm in the step.
    PistonTravelAboveStep,
    /// The gas temperature, incoming or landing, is at or below 126.192 K, nitrogen's critical temperature: the gas can condense, and one phase of the equation of state is not this charge.
    GasBelowCriticalTemperature,
    /// The gas pressure, incoming or landing, is above 330 bar, the vessel's rated maximum working pressure.
    PressureAboveRated,
    /// The work moved or the heat moved carried into a start is below zero.
    NegativeMovedSum,
    /// The gas volume, incoming or landing, is above the vessel's 0.58879 l: there is no oil left to draw out.
    OilSideEmpty,
    /// The heat exchanged with the wall over the motion so far, with the sums carried into the start, incoming or landing, is more than 0.065 of the work done either way, with the work counted as 100 J until it is more: the motion is too slow, or the rest too long, for the card's one time constant.
    HeatShareAboveCard,
    /// The trial step created energy: the gas gained more than the work on it less the heat it gave.
    SpontaneousGeneration,
    /// The trial step destroyed energy: the gas gained less than the work on it less the heat it gave.
    FirstLawBreach,
}

/// What the rig holds over a step.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AccumulatorLoad {
    /// Speed of the piston: positive pushes oil into the accumulator and
    /// compresses the gas.
    pub piston_speed_m_s: f64,
}

/// The locked accumulator. Every field is a recorded copy of a crate constant
/// and none is a caller input.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AccumulatorSbo450 {
    /// Recorded copy of [`GAS_CONSTANT_J_MOL_K`].
    pub gas_constant_j_mol_k: f64,
    /// Recorded copy of [`NITROGEN_MOLAR_MASS_KG_MOL`].
    pub molar_mass_kg_mol: f64,
    /// Recorded copy of [`CRITICAL_TEMPERATURE_K`].
    pub critical_temperature_k: f64,
    /// Recorded copy of [`CRITICAL_PRESSURE_PA`].
    pub critical_pressure_pa: f64,
    /// Recorded copy of [`ACENTRIC_FACTOR`].
    pub acentric_factor: f64,
    /// Recorded copy of [`IDEAL_HEAT_CAPACITY_J_MOL_K`].
    pub ideal_heat_capacity_j_mol_k: f64,
    /// Recorded copy of [`PISTON_AREA_M2`].
    pub piston_area_m2: f64,
    /// Recorded copy of [`VESSEL_GAS_VOLUME_M3`].
    pub vessel_gas_volume_m3: f64,
    /// Recorded copy of [`THERMAL_TIME_CONSTANT_S`].
    pub thermal_time_constant_s: f64,
    /// Recorded copy of [`PRESSURE_FLOOR_PA`].
    pub pressure_floor_pa: f64,
    /// Recorded copy of [`PRESSURE_CEILING_PA`].
    pub pressure_ceiling_pa: f64,
    pub pressure_rated_pa: f64,
    /// Recorded copy of [`PRESSURE_BANDS_20_BAR_PA`].
    pub pressure_bands_20_bar_pa: [[f64; 2]; 2],
    /// Recorded copy of [`PRESSURE_BANDS_40_BAR_PA`].
    pub pressure_bands_40_bar_pa: [[f64; 2]; 2],
    /// Recorded copy of [`GAS_TEMPERATURE_FLOOR_K`].
    pub gas_temperature_floor_k: f64,
    /// Recorded copy of [`GAS_TEMPERATURE_CEILING_K`].
    pub gas_temperature_ceiling_k: f64,
    /// Recorded copy of [`WALL_TEMPERATURE_FLOOR_K`].
    pub wall_temperature_floor_k: f64,
    /// Recorded copy of [`WALL_TEMPERATURE_CEILING_K`].
    pub wall_temperature_ceiling_k: f64,
    /// Recorded copy of [`GAS_MASS_20_BAR_FLOOR_KG`].
    pub gas_mass_20_bar_floor_kg: f64,
    /// Recorded copy of [`GAS_MASS_20_BAR_CEILING_KG`].
    pub gas_mass_20_bar_ceiling_kg: f64,
    /// Recorded copy of [`GAS_MASS_40_BAR_FLOOR_KG`].
    pub gas_mass_40_bar_floor_kg: f64,
    /// Recorded copy of [`GAS_MASS_40_BAR_CEILING_KG`].
    pub gas_mass_40_bar_ceiling_kg: f64,
    /// Recorded copy of [`HEAT_SHARE_LIMIT`].
    pub heat_share_limit: f64,
    /// Recorded copy of [`HEAT_SHARE_WORK_FLOOR_J`].
    pub heat_share_work_floor_j: f64,
    /// Recorded copy of [`PISTON_SPEED_CEILING_M_S`].
    pub piston_speed_ceiling_m_s: f64,
    /// Recorded copy of [`PISTON_TRAVEL_PER_STEP_M`].
    pub piston_travel_per_step_m: f64,
    /// Recorded copy of [`COMPUTED_LIMIT_ALLOWANCE`].
    pub computed_limit_allowance: f64,
    /// Recorded copy of [`GATE_SHARE_OF_TIME_CONSTANT`].
    pub gate_share_of_time_constant: f64,
    /// Recorded copy of [`GATE_OVER_FLOOR`].
    pub gate_over_floor: f64,
    /// Recorded copy of [`RESIDUAL_CONSERVATION_LIMIT`].
    pub residual_conservation_limit: f64,
}

/// The Peng-Robinson constants of the gas per kilogram, worked from the
/// card's recorded copies each time they are needed.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Gas {
    /// `R`, J/(kg K).
    gas_constant: f64,
    /// `c_v0`, J/(kg K).
    ideal_cv: f64,
    /// `κ`.
    kappa: f64,
    /// `a`, Pa m⁶/kg².
    attraction: f64,
    /// `b`, m³/kg.
    covolume: f64,
    /// `T_c`, K.
    critical_temperature_k: f64,
}

/// `α`, `dα/dT` and `d²α/dT²` at one temperature.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Alpha {
    value: f64,
    first: f64,
    second: f64,
}

impl Gas {
    fn alpha(&self, temperature_k: f64) -> Alpha {
        let root = (temperature_k / self.critical_temperature_k).sqrt();
        let q = 1.0 + self.kappa * (1.0 - root);
        Alpha {
            value: q * q,
            first: -self.kappa * q * root / temperature_k,
            second: self.kappa * (1.0 + self.kappa) * root / (2.0 * temperature_k * temperature_k),
        }
    }

    fn denominator(&self, v: f64) -> f64 {
        v * v + 2.0 * self.covolume * v - self.covolume * self.covolume
    }

    /// `L(v)`, kg/m³.
    fn log_term(&self, v: f64) -> f64 {
        let root2 = 2.0_f64.sqrt();
        crate::math::log((v + (1.0 + root2) * self.covolume) / (v + (1.0 - root2) * self.covolume))
            / (2.0 * root2 * self.covolume)
    }

    fn pressure_pa(&self, temperature_k: f64, v: f64) -> f64 {
        self.gas_constant * temperature_k / (v - self.covolume)
            - self.attraction * self.alpha(temperature_k).value / self.denominator(v)
    }

    /// `(∂p/∂T)` at constant volume, Pa/K.
    fn pressure_per_kelvin(&self, temperature_k: f64, v: f64) -> f64 {
        self.gas_constant / (v - self.covolume)
            - self.attraction * self.alpha(temperature_k).first / self.denominator(v)
    }

    /// `(∂p/∂v)` at constant temperature, Pa kg/m³.
    fn pressure_per_volume(&self, temperature_k: f64, v: f64) -> f64 {
        let free = v - self.covolume;
        let denominator = self.denominator(v);
        -self.gas_constant * temperature_k / (free * free)
            + self.attraction * self.alpha(temperature_k).value * (2.0 * v + 2.0 * self.covolume)
                / (denominator * denominator)
    }

    /// `c_v`, J/(kg K).
    fn cv(&self, temperature_k: f64, v: f64) -> f64 {
        self.ideal_cv
            + temperature_k * self.attraction * self.alpha(temperature_k).second * self.log_term(v)
    }

    /// `u`, J/kg, zero at zero temperature on the ideal-gas line.
    fn energy_j_kg(&self, temperature_k: f64, v: f64) -> f64 {
        let alpha = self.alpha(temperature_k);
        self.ideal_cv * temperature_k
            - self.attraction * (alpha.value - temperature_k * alpha.first) * self.log_term(v)
    }

    /// The specific volume at which the gas has this pressure at this
    /// temperature, by Newton's method from the ideal-gas volume. `None` if
    /// forty passes do not land within 1e-13 of the pressure.
    fn volume_m3_kg(&self, temperature_k: f64, pressure_pa: f64) -> Option<f64> {
        let ideal = self.gas_constant * temperature_k / pressure_pa;
        if let Some(v) = self.newton_volume(temperature_k, pressure_pa, ideal) {
            return Some(v);
        }
        // Dense gas near the critical point, where Newton's method from the
        // ideal-gas volume does not land: above the critical temperature the
        // pressure falls with the volume on every isotherm, so the volume is
        // bracketed between the covolume and the ideal-gas volume plus the
        // covolume, halved down, and polished by Newton's method.
        if !(temperature_k > self.critical_temperature_k) {
            return None;
        }
        let mut low = self.covolume;
        let mut high = ideal + self.covolume;
        let mut pass = 0;
        while pass < 200 {
            let middle = 0.5 * (low + high);
            if !(middle > low && middle < high) {
                break;
            }
            if self.pressure_pa(temperature_k, middle) > pressure_pa {
                low = middle;
            } else {
                high = middle;
            }
            pass += 1;
        }
        self.newton_volume(temperature_k, pressure_pa, 0.5 * (low + high))
    }

    /// Newton's method on the pressure from `start`; `None` if it leaves the
    /// gas or does not meet the pressure to one part in 1e13.
    fn newton_volume(&self, temperature_k: f64, pressure_pa: f64, start: f64) -> Option<f64> {
        let mut v = start;
        let mut pass = 0;
        while pass < 40 {
            let slope = self.pressure_per_volume(temperature_k, v);
            if !(slope.is_finite() && slope < 0.0) {
                return None;
            }
            let next = v - (self.pressure_pa(temperature_k, v) - pressure_pa) / slope;
            if !(next.is_finite() && next > self.covolume) {
                return None;
            }
            if next == v {
                break;
            }
            v = next;
            pass += 1;
        }
        let miss = (self.pressure_pa(temperature_k, v) - pressure_pa).abs();
        if miss <= 1.0e-13 * pressure_pa { Some(v) } else { None }
    }
}

impl AccumulatorSbo450 {
    /// Card: the empty SBO450-0.6 on the Darmstadt rig. The constructor takes
    /// no arguments.
    #[must_use]
    pub fn on_darmstadt_rig() -> Self {
        let accumulator = Self {
            gas_constant_j_mol_k: GAS_CONSTANT_J_MOL_K,
            molar_mass_kg_mol: NITROGEN_MOLAR_MASS_KG_MOL,
            critical_temperature_k: CRITICAL_TEMPERATURE_K,
            critical_pressure_pa: CRITICAL_PRESSURE_PA,
            acentric_factor: ACENTRIC_FACTOR,
            ideal_heat_capacity_j_mol_k: IDEAL_HEAT_CAPACITY_J_MOL_K,
            piston_area_m2: PISTON_AREA_M2,
            vessel_gas_volume_m3: VESSEL_GAS_VOLUME_M3,
            thermal_time_constant_s: THERMAL_TIME_CONSTANT_S,
            pressure_floor_pa: PRESSURE_FLOOR_PA,
            pressure_ceiling_pa: PRESSURE_CEILING_PA,
            pressure_rated_pa: PRESSURE_RATED_PA,
            pressure_bands_20_bar_pa: PRESSURE_BANDS_20_BAR_PA,
            pressure_bands_40_bar_pa: PRESSURE_BANDS_40_BAR_PA,
            gas_temperature_floor_k: GAS_TEMPERATURE_FLOOR_K,
            gas_temperature_ceiling_k: GAS_TEMPERATURE_CEILING_K,
            wall_temperature_floor_k: WALL_TEMPERATURE_FLOOR_K,
            wall_temperature_ceiling_k: WALL_TEMPERATURE_CEILING_K,
            gas_mass_20_bar_floor_kg: GAS_MASS_20_BAR_FLOOR_KG,
            gas_mass_20_bar_ceiling_kg: GAS_MASS_20_BAR_CEILING_KG,
            gas_mass_40_bar_floor_kg: GAS_MASS_40_BAR_FLOOR_KG,
            gas_mass_40_bar_ceiling_kg: GAS_MASS_40_BAR_CEILING_KG,
            heat_share_limit: HEAT_SHARE_LIMIT,
            heat_share_work_floor_j: HEAT_SHARE_WORK_FLOOR_J,
            piston_speed_ceiling_m_s: PISTON_SPEED_CEILING_M_S,
            piston_travel_per_step_m: PISTON_TRAVEL_PER_STEP_M,
            computed_limit_allowance: COMPUTED_LIMIT_ALLOWANCE,
            gate_share_of_time_constant: GATE_SHARE_OF_TIME_CONSTANT,
            gate_over_floor: GATE_OVER_FLOOR,
            residual_conservation_limit: RESIDUAL_CONSERVATION_LIMIT,
        };
        if accumulator.is_valid() {
            accumulator
        } else {
            ::core::panic!("accumulator failed is_valid");
        }
    }

    /// Every recorded copy equals its crate constant bit for bit.
    #[must_use]
    pub fn is_valid(&self) -> bool {
        self.gas_constant_j_mol_k.to_bits() == GAS_CONSTANT_J_MOL_K.to_bits()
            && self.molar_mass_kg_mol.to_bits() == NITROGEN_MOLAR_MASS_KG_MOL.to_bits()
            && self.critical_temperature_k.to_bits() == CRITICAL_TEMPERATURE_K.to_bits()
            && self.critical_pressure_pa.to_bits() == CRITICAL_PRESSURE_PA.to_bits()
            && self.acentric_factor.to_bits() == ACENTRIC_FACTOR.to_bits()
            && self.ideal_heat_capacity_j_mol_k.to_bits() == IDEAL_HEAT_CAPACITY_J_MOL_K.to_bits()
            && self.piston_area_m2.to_bits() == PISTON_AREA_M2.to_bits()
            && self.vessel_gas_volume_m3.to_bits() == VESSEL_GAS_VOLUME_M3.to_bits()
            && self.thermal_time_constant_s.to_bits() == THERMAL_TIME_CONSTANT_S.to_bits()
            && self.pressure_floor_pa.to_bits() == PRESSURE_FLOOR_PA.to_bits()
            && self.pressure_ceiling_pa.to_bits() == PRESSURE_CEILING_PA.to_bits()
            && self.pressure_rated_pa.to_bits() == PRESSURE_RATED_PA.to_bits()
            && bands_are(&self.pressure_bands_20_bar_pa, &PRESSURE_BANDS_20_BAR_PA)
            && bands_are(&self.pressure_bands_40_bar_pa, &PRESSURE_BANDS_40_BAR_PA)
            && self.gas_temperature_floor_k.to_bits() == GAS_TEMPERATURE_FLOOR_K.to_bits()
            && self.gas_temperature_ceiling_k.to_bits() == GAS_TEMPERATURE_CEILING_K.to_bits()
            && self.wall_temperature_floor_k.to_bits() == WALL_TEMPERATURE_FLOOR_K.to_bits()
            && self.wall_temperature_ceiling_k.to_bits() == WALL_TEMPERATURE_CEILING_K.to_bits()
            && self.gas_mass_20_bar_floor_kg.to_bits() == GAS_MASS_20_BAR_FLOOR_KG.to_bits()
            && self.gas_mass_20_bar_ceiling_kg.to_bits() == GAS_MASS_20_BAR_CEILING_KG.to_bits()
            && self.gas_mass_40_bar_floor_kg.to_bits() == GAS_MASS_40_BAR_FLOOR_KG.to_bits()
            && self.gas_mass_40_bar_ceiling_kg.to_bits() == GAS_MASS_40_BAR_CEILING_KG.to_bits()
            && self.heat_share_limit.to_bits() == HEAT_SHARE_LIMIT.to_bits()
            && self.heat_share_work_floor_j.to_bits() == HEAT_SHARE_WORK_FLOOR_J.to_bits()
            && self.piston_speed_ceiling_m_s.to_bits() == PISTON_SPEED_CEILING_M_S.to_bits()
            && self.piston_travel_per_step_m.to_bits() == PISTON_TRAVEL_PER_STEP_M.to_bits()
            && self.computed_limit_allowance.to_bits() == COMPUTED_LIMIT_ALLOWANCE.to_bits()
            && self.gate_share_of_time_constant.to_bits() == GATE_SHARE_OF_TIME_CONSTANT.to_bits()
            && self.gate_over_floor.to_bits() == GATE_OVER_FLOOR.to_bits()
            && self.residual_conservation_limit.to_bits() == RESIDUAL_CONSERVATION_LIMIT.to_bits()
    }

    fn gas(&self) -> Gas {
        let gas_constant = self.gas_constant_j_mol_k / self.molar_mass_kg_mol;
        let w = self.acentric_factor;
        Gas {
            gas_constant,
            ideal_cv: self.ideal_heat_capacity_j_mol_k / self.molar_mass_kg_mol,
            kappa: 0.37464 + 1.54226 * w - 0.26992 * w * w,
            attraction: 0.45724 * gas_constant * gas_constant * self.critical_temperature_k
                * self.critical_temperature_k
                / self.critical_pressure_pa,
            covolume: 0.07780 * gas_constant * self.critical_temperature_k
                / self.critical_pressure_pa,
            critical_temperature_k: self.critical_temperature_k,
        }
    }

    /// Pressure of nitrogen at this temperature and specific volume, Pa.
    #[must_use]
    pub fn pressure_pa(&self, temperature_k: f64, specific_volume_m3_kg: f64) -> f64 {
        self.gas().pressure_pa(temperature_k, specific_volume_m3_kg)
    }

    /// Heat capacity of nitrogen at constant volume, J/(kg K).
    #[must_use]
    pub fn heat_capacity_cv_j_kg_k(&self, temperature_k: f64, specific_volume_m3_kg: f64) -> f64 {
        self.gas().cv(temperature_k, specific_volume_m3_kg)
    }

    /// Internal energy of nitrogen, J/kg, zero at zero temperature on the
    /// ideal-gas line.
    #[must_use]
    pub fn internal_energy_j_kg(&self, temperature_k: f64, specific_volume_m3_kg: f64) -> f64 {
        self.gas().energy_j_kg(temperature_k, specific_volume_m3_kg)
    }

    /// Specific volume of nitrogen at this temperature and pressure, m³/kg.
    /// `None` when the search does not land.
    #[must_use]
    pub fn specific_volume_m3_kg(&self, temperature_k: f64, pressure_pa: f64) -> Option<f64> {
        if !(temperature_k.is_finite() && pressure_pa.is_finite())
            || !(temperature_k > 0.0 && pressure_pa > 0.0)
        {
            return None;
        }
        self.gas().volume_m3_kg(temperature_k, pressure_pa)
    }

    /// The ledger scale: the pressure ceiling times the vessel volume, J.
    /// About 5 064 J.
    #[must_use]
    pub fn ledger_scale_j(&self) -> f64 {
        self.pressure_ceiling_pa * self.vessel_gas_volume_m3
    }

    /// Largest step the card stands behind: the thermal time constant over
    /// 50, 0.3504 s. Not applied by [`step`].
    #[must_use]
    pub fn gate_dt_s(&self) -> Option<f64> {
        let gate = self.thermal_time_constant_s * self.gate_share_of_time_constant;
        if gate.is_finite() && gate > 0.0 { Some(gate) } else { None }
    }

    /// Smallest step the card stands behind: [`Self::gate_dt_s`] divided by
    /// 2^20, about 3.34e-7 s. Not applied by [`step`].
    #[must_use]
    pub fn floor_dt_s(&self) -> Option<f64> {
        match self.gate_dt_s() {
            Some(gate_dt_s) if self.gate_over_floor.is_finite() && self.gate_over_floor > 0.0 => {
                Some(gate_dt_s / self.gate_over_floor)
            }
            _ => None,
        }
    }
}

/// The gas. The ledger fields are part of the state; write them only through
/// [`step`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GasState {
    /// Time since the start of the run.
    pub t_s: f64,
    /// Mass of nitrogen. Never changes.
    pub gas_mass_kg: f64,
    /// Temperature of the wall. Never changes during a run.
    pub wall_temperature_k: f64,
    /// Temperature of the gas.
    pub gas_temperature_k: f64,
    /// Volume of the gas.
    pub gas_volume_m3: f64,
    /// Pressure of the gas at that temperature and volume.
    pub gas_pressure_pa: f64,
    /// That pressure times the piston area: the force the gas pressure puts
    /// on the piston through the oil.
    pub gas_force_n: f64,
    /// `W`, the work of the oil on the gas since the start of the run.
    pub work_on_gas_j: f64,
    /// `Q`, the heat from the gas to the wall since the start of the run.
    pub heat_to_wall_j: f64,
    /// Internal energy of the gas at the start of the run, `m u(start)`.
    pub start_energy_j: f64,
    /// Signed `m u − m u(start) − W + Q`. Positive is created energy.
    pub energy_defect_j: f64,
    /// `|energy_defect_j|` over the ledger scale.
    pub residual_conservation_error: f64,
    /// `M = Σ|ΔW|`: the work of the oil on the gas, either way, summed step
    /// by step over the motion so far: the sum carried into the start plus
    /// the steps of this run.
    pub work_moved_j: f64,
    /// `H = Σ|ΔQ|`: the heat between the gas and the wall, either way, summed
    /// step by step over the motion so far: the sum carried into the start
    /// plus the steps of this run.
    pub heat_moved_j: f64,
    /// `H / max(M, 100 J)`: the share of heat in the work over the motion so
    /// far. A step lands only if it is at most [`HEAT_SHARE_LIMIT`].
    pub heat_share: f64,
}

impl GasState {
    /// The gas at this mass, pressure and temperature beside a wall at this
    /// temperature, at time 0 with the ledger empty. The volume is worked
    /// back from the pressure. `work_moved_j` and `heat_moved_j` are the sums
    /// `M` and `H` of the motion before this start: zero for gas that has not
    /// moved, and the `work_moved_j` and `heat_moved_j` of the returned state
    /// when a run or a step is carried on. A sum below zero is
    /// [`StepReject::NegativeMovedSum`], and sums whose share is already
    /// above the limit are [`StepReject::HeatShareAboveCard`]. A value outside
    /// the card is named.
    pub fn try_start(
        accumulator: &AccumulatorSbo450,
        gas_mass_kg: f64,
        gas_pressure_pa: f64,
        gas_temperature_k: f64,
        wall_temperature_k: f64,
        work_moved_j: f64,
        heat_moved_j: f64,
    ) -> Result<Self, StepReject> {
        if !(gas_mass_kg.is_finite()
            && gas_pressure_pa.is_finite()
            && gas_temperature_k.is_finite()
            && wall_temperature_k.is_finite()
            && work_moved_j.is_finite()
            && heat_moved_j.is_finite())
        {
            return Err(StepReject::NonFiniteInput);
        }
        if !accumulator.is_valid() {
            return Err(StepReject::InvalidAccumulator);
        }
        if !(gas_mass_kg > 0.0) {
            return Err(StepReject::GasMassNotPositive);
        }
        if !(wall_temperature_k > 0.0) {
            return Err(StepReject::WallTemperatureNotPositive);
        }
        if !(gas_temperature_k > accumulator.critical_temperature_k) {
            return Err(StepReject::GasBelowCriticalTemperature);
        }
        if !(gas_pressure_pa > 0.0) {
            return Err(StepReject::OilSideEmpty);
        }
        if !(gas_pressure_pa <= accumulator.pressure_rated_pa) {
            return Err(StepReject::PressureAboveRated);
        }
        let gas = accumulator.gas();
        let v = match gas.volume_m3_kg(gas_temperature_k, gas_pressure_pa) {
            Some(v) => v,
            None => return Err(StepReject::NonFiniteState),
        };
        let gas_volume_m3 = gas_mass_kg * v;
        if !volume_in_vessel(accumulator, gas_volume_m3) {
            return Err(StepReject::OilSideEmpty);
        }
        if !(work_moved_j >= 0.0 && heat_moved_j >= 0.0) {
            return Err(StepReject::NegativeMovedSum);
        }
        if !heat_share_in_card(accumulator, work_moved_j, heat_moved_j) {
            return Err(StepReject::HeatShareAboveCard);
        }
        let pressure = gas.pressure_pa(gas_temperature_k, v);
        let state = Self {
            t_s: 0.0,
            gas_mass_kg,
            wall_temperature_k,
            gas_temperature_k,
            gas_volume_m3,
            gas_pressure_pa: pressure,
            gas_force_n: pressure * accumulator.piston_area_m2,
            work_on_gas_j: 0.0,
            heat_to_wall_j: 0.0,
            start_energy_j: gas_mass_kg * gas.energy_j_kg(gas_temperature_k, v),
            energy_defect_j: 0.0,
            residual_conservation_error: 0.0,
            work_moved_j,
            heat_moved_j,
            heat_share: heat_moved_j / heat_share_base_j(accumulator, work_moved_j),
        };
        if state_is_finite(&state) { Ok(state) } else { Err(StepReject::NonFiniteState) }
    }
}

fn volume_in_vessel(accumulator: &AccumulatorSbo450, gas_volume_m3: f64) -> bool {
    gas_volume_m3 > 0.0
        && gas_volume_m3
            <= accumulator.vessel_gas_volume_m3 * (1.0 + accumulator.computed_limit_allowance)
}

fn pressure_in_rating(accumulator: &AccumulatorSbo450, pressure_pa: f64) -> bool {
    pressure_pa <= accumulator.pressure_rated_pa * (1.0 + accumulator.computed_limit_allowance)
}

/// The step lies inside the range compared with measurement: the charge,
/// the wall, the speed, the gas temperature and the pressure in its
/// charge's bands.
fn inside_compared_runs(
    accumulator: &AccumulatorSbo450,
    gas_mass_kg: f64,
    wall_temperature_k: f64,
    piston_speed_m_s: f64,
    gas_temperature_k: f64,
    pressure_pa: f64,
) -> bool {
    mass_is_a_measured_charge(accumulator, gas_mass_kg)
        && wall_temperature_k >= accumulator.wall_temperature_floor_k
        && wall_temperature_k <= accumulator.wall_temperature_ceiling_k
        && piston_speed_m_s.abs() <= accumulator.piston_speed_ceiling_m_s
        && gas_temperature_k >= accumulator.gas_temperature_floor_k
        && gas_temperature_k <= accumulator.gas_temperature_ceiling_k
        && pressure_in_charge_bands(accumulator, gas_mass_kg, pressure_pa, accumulator.computed_limit_allowance)
}

fn bands_are(recorded: &[[f64; 2]; 2], constant: &[[f64; 2]; 2]) -> bool {
    recorded[0][0].to_bits() == constant[0][0].to_bits()
        && recorded[0][1].to_bits() == constant[0][1].to_bits()
        && recorded[1][0].to_bits() == constant[1][0].to_bits()
        && recorded[1][1].to_bits() == constant[1][1].to_bits()
}

/// The pressure lies in one of the bands compared with measurement for the
/// charge this mass is nearer, each end widened by `allowance`, relative.
fn pressure_in_charge_bands(
    accumulator: &AccumulatorSbo450,
    gas_mass_kg: f64,
    pressure_pa: f64,
    allowance: f64,
) -> bool {
    let bands = if gas_mass_kg <= accumulator.gas_mass_20_bar_ceiling_kg {
        &accumulator.pressure_bands_20_bar_pa
    } else {
        &accumulator.pressure_bands_40_bar_pa
    };
    bands.iter().any(|band| {
        pressure_pa >= band[0] * (1.0 - allowance) && pressure_pa <= band[1] * (1.0 + allowance)
    })
}

/// The mass is one of the two charges measured, each within its band.
fn mass_is_a_measured_charge(accumulator: &AccumulatorSbo450, gas_mass_kg: f64) -> bool {
    (gas_mass_kg >= accumulator.gas_mass_20_bar_floor_kg
        && gas_mass_kg <= accumulator.gas_mass_20_bar_ceiling_kg)
        || (gas_mass_kg >= accumulator.gas_mass_40_bar_floor_kg
            && gas_mass_kg <= accumulator.gas_mass_40_bar_ceiling_kg)
}

/// The work the heat is set against: the work moved, or the floor until the
/// work moved is more.
fn heat_share_base_j(accumulator: &AccumulatorSbo450, work_moved_j: f64) -> f64 {
    if work_moved_j > accumulator.heat_share_work_floor_j {
        work_moved_j
    } else {
        accumulator.heat_share_work_floor_j
    }
}

/// The heat moved is at most the limit's share of the work it is set against.
fn heat_share_in_card(accumulator: &AccumulatorSbo450, work_moved_j: f64, heat_moved_j: f64) -> bool {
    heat_moved_j
        <= accumulator.heat_share_limit
            * heat_share_base_j(accumulator, work_moved_j)
            * (1.0 + accumulator.computed_limit_allowance)
}

/// The four numbers a step integrates.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Trial {
    gas_temperature_k: f64,
    gas_volume_m3: f64,
    work_on_gas_j: f64,
    heat_to_wall_j: f64,
}

/// Their rates of change with the piston speed held.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Rates {
    temperature_k_s: f64,
    volume_m3_s: f64,
    work_w: f64,
    heat_w: f64,
}

fn rates(
    accumulator: &AccumulatorSbo450,
    gas: &Gas,
    gas_mass_kg: f64,
    wall_temperature_k: f64,
    piston_speed_m_s: f64,
    at: Trial,
) -> Rates {
    let v = at.gas_volume_m3 / gas_mass_kg;
    let volume_m3_s = -accumulator.piston_area_m2 * piston_speed_m_s;
    let pressure = gas.pressure_pa(at.gas_temperature_k, v);
    let heat_w = gas_mass_kg * gas.ideal_cv / accumulator.thermal_time_constant_s
        * (at.gas_temperature_k - wall_temperature_k);
    let temperature_k_s = (-at.gas_temperature_k
        * gas.pressure_per_kelvin(at.gas_temperature_k, v)
        * volume_m3_s
        - heat_w)
        / (gas_mass_kg * gas.cv(at.gas_temperature_k, v));
    Rates { temperature_k_s, volume_m3_s, work_w: -pressure * volume_m3_s, heat_w }
}

fn moved(at: Trial, by: Rates, dt_s: f64) -> Trial {
    Trial {
        gas_temperature_k: at.gas_temperature_k + dt_s * by.temperature_k_s,
        gas_volume_m3: at.gas_volume_m3 + dt_s * by.volume_m3_s,
        work_on_gas_j: at.work_on_gas_j + dt_s * by.work_w,
        heat_to_wall_j: at.heat_to_wall_j + dt_s * by.heat_w,
    }
}

/// One classical fourth-order Runge-Kutta step with the piston speed held.
fn runge_kutta(
    accumulator: &AccumulatorSbo450,
    state: &GasState,
    load: AccumulatorLoad,
    dt_s: f64,
) -> Trial {
    let gas = accumulator.gas();
    let at = Trial {
        gas_temperature_k: state.gas_temperature_k,
        gas_volume_m3: state.gas_volume_m3,
        work_on_gas_j: state.work_on_gas_j,
        heat_to_wall_j: state.heat_to_wall_j,
    };
    let rate = |point: Trial| {
        rates(
            accumulator,
            &gas,
            state.gas_mass_kg,
            state.wall_temperature_k,
            load.piston_speed_m_s,
            point,
        )
    };
    let k1 = rate(at);
    let k2 = rate(moved(at, k1, 0.5 * dt_s));
    let k3 = rate(moved(at, k2, 0.5 * dt_s));
    let k4 = rate(moved(at, k3, dt_s));
    let blend = |a: f64, b: f64, c: f64, d: f64| dt_s / 6.0 * (a + 2.0 * b + 2.0 * c + d);
    Trial {
        gas_temperature_k: at.gas_temperature_k
            + blend(k1.temperature_k_s, k2.temperature_k_s, k3.temperature_k_s, k4.temperature_k_s),
        gas_volume_m3: at.gas_volume_m3
            + blend(k1.volume_m3_s, k2.volume_m3_s, k3.volume_m3_s, k4.volume_m3_s),
        work_on_gas_j: at.work_on_gas_j + blend(k1.work_w, k2.work_w, k3.work_w, k4.work_w),
        heat_to_wall_j: at.heat_to_wall_j + blend(k1.heat_w, k2.heat_w, k3.heat_w, k4.heat_w),
    }
}

/// Hold the piston speed for one step and write the frame.
///
/// A rejected step leaves `state` and `frame` as they were.
///
/// A value of the landed state below the normal range is written as zero
/// with its sign ([`crate::normal`]).
#[must_use]
pub fn step(
    accumulator: &AccumulatorSbo450,
    state: &mut GasState,
    frame: &mut LastStateFrame64,
    load: AccumulatorLoad,
    dt_s: f64,
) -> Result<(), StepReject> {
    step_as_computed(accumulator, state, frame, load, dt_s)?;
    normal_or_zero_state(state);
    Ok(())
}

/// The step as its equations compute it, before a value of the landed
/// state below the normal range is written as zero.
fn step_as_computed(
    accumulator: &AccumulatorSbo450,
    state: &mut GasState,
    frame: &mut LastStateFrame64,
    load: AccumulatorLoad,
    dt_s: f64,
) -> Result<(), StepReject> {
    validate(accumulator, state, load, dt_s)?;
    let trial = runge_kutta(accumulator, state, load, dt_s);
    commit(accumulator, state, frame, dt_s, trial)?;
    // The step has landed: the flag reports where it lies against the range
    // compared with measurement.
    if !inside_compared_runs(
        accumulator,
        state.gas_mass_kg,
        state.wall_temperature_k,
        load.piston_speed_m_s,
        state.gas_temperature_k,
        state.gas_pressure_pa,
    ) {
        frame.flags = FLAG_OUTSIDE_COMPARED_RUNS;
    }
    Ok(())
}

/// Each value of a landed state below the normal range written as zero
/// with its sign ([`crate::normal`]): the state carried to the next step
/// and published holds no subnormal number.
fn normal_or_zero_state(state: &mut GasState) {
    state.t_s = normal_or_zero(state.t_s);
    state.gas_mass_kg = normal_or_zero(state.gas_mass_kg);
    state.wall_temperature_k = normal_or_zero(state.wall_temperature_k);
    state.gas_temperature_k = normal_or_zero(state.gas_temperature_k);
    state.gas_volume_m3 = normal_or_zero(state.gas_volume_m3);
    state.gas_pressure_pa = normal_or_zero(state.gas_pressure_pa);
    state.gas_force_n = normal_or_zero(state.gas_force_n);
    state.work_on_gas_j = normal_or_zero(state.work_on_gas_j);
    state.heat_to_wall_j = normal_or_zero(state.heat_to_wall_j);
    state.start_energy_j = normal_or_zero(state.start_energy_j);
    state.energy_defect_j = normal_or_zero(state.energy_defect_j);
    state.residual_conservation_error = normal_or_zero(state.residual_conservation_error);
    state.work_moved_j = normal_or_zero(state.work_moved_j);
    state.heat_moved_j = normal_or_zero(state.heat_moved_j);
    state.heat_share = normal_or_zero(state.heat_share);
}

fn validate(
    accumulator: &AccumulatorSbo450,
    state: &GasState,
    load: AccumulatorLoad,
    dt_s: f64,
) -> Result<(), StepReject> {
    if !(dt_s.is_finite() && load.piston_speed_m_s.is_finite()) {
        return Err(StepReject::NonFiniteInput);
    }
    if !state_is_finite(state) {
        return Err(StepReject::NonFiniteState);
    }
    if dt_s <= 0.0 {
        return Err(StepReject::NonPositiveDt);
    }
    if !accumulator.is_valid() {
        return Err(StepReject::InvalidAccumulator);
    }
    if !(state.gas_mass_kg > 0.0) {
        return Err(StepReject::GasMassNotPositive);
    }
    if !(state.wall_temperature_k > 0.0) {
        return Err(StepReject::WallTemperatureNotPositive);
    }
    let speed = load.piston_speed_m_s.abs();
    if !(speed * dt_s
        <= accumulator.piston_travel_per_step_m * (1.0 + accumulator.computed_limit_allowance))
    {
        return Err(StepReject::PistonTravelAboveStep);
    }
    landing_in_card(
        accumulator,
        state.gas_mass_kg,
        state.gas_temperature_k,
        state.gas_volume_m3,
    )?;
    if !(state.work_moved_j >= 0.0 && state.heat_moved_j >= 0.0) {
        return Err(StepReject::NonFiniteState);
    }
    if !heat_share_in_card(accumulator, state.work_moved_j, state.heat_moved_j) {
        return Err(StepReject::HeatShareAboveCard);
    }
    Ok(())
}

/// The temperature, the volume and the pressure checked against the card, in
/// that order. Returns the pressure.
fn landing_in_card(
    accumulator: &AccumulatorSbo450,
    gas_mass_kg: f64,
    gas_temperature_k: f64,
    gas_volume_m3: f64,
) -> Result<f64, StepReject> {
    if !(gas_temperature_k > accumulator.critical_temperature_k) {
        return Err(StepReject::GasBelowCriticalTemperature);
    }
    if !volume_in_vessel(accumulator, gas_volume_m3) {
        return Err(StepReject::OilSideEmpty);
    }
    let pressure = accumulator.gas().pressure_pa(gas_temperature_k, gas_volume_m3 / gas_mass_kg);
    if !pressure.is_finite() {
        return Err(StepReject::NonFiniteState);
    }
    if !(pressure > 0.0) {
        return Err(StepReject::NonFiniteState);
    }
    if !pressure_in_rating(accumulator, pressure) {
        return Err(StepReject::PressureAboveRated);
    }
    Ok(pressure)
}

fn commit(
    accumulator: &AccumulatorSbo450,
    state: &mut GasState,
    frame: &mut LastStateFrame64,
    dt_s: f64,
    trial: Trial,
) -> Result<(), StepReject> {
    if !(trial.gas_temperature_k.is_finite()
        && trial.gas_volume_m3.is_finite()
        && trial.work_on_gas_j.is_finite()
        && trial.heat_to_wall_j.is_finite())
    {
        return Err(StepReject::NonFiniteState);
    }
    let pressure = landing_in_card(
        accumulator,
        state.gas_mass_kg,
        trial.gas_temperature_k,
        trial.gas_volume_m3,
    )?;
    let scale = accumulator.ledger_scale_j();
    if !(scale.is_finite() && scale > 0.0) {
        return Err(StepReject::InvalidAccumulator);
    }
    let work_moved_j = state.work_moved_j + (trial.work_on_gas_j - state.work_on_gas_j).abs();
    let heat_moved_j = state.heat_moved_j + (trial.heat_to_wall_j - state.heat_to_wall_j).abs();
    let energy_j = state.gas_mass_kg
        * accumulator
            .gas()
            .energy_j_kg(trial.gas_temperature_k, trial.gas_volume_m3 / state.gas_mass_kg);
    let defect_j = energy_j - state.start_energy_j - trial.work_on_gas_j + trial.heat_to_wall_j;
    let landed = GasState {
        t_s: state.t_s + dt_s,
        gas_mass_kg: state.gas_mass_kg,
        wall_temperature_k: state.wall_temperature_k,
        gas_temperature_k: trial.gas_temperature_k,
        gas_volume_m3: trial.gas_volume_m3,
        gas_pressure_pa: pressure,
        gas_force_n: pressure * accumulator.piston_area_m2,
        work_on_gas_j: trial.work_on_gas_j,
        heat_to_wall_j: trial.heat_to_wall_j,
        start_energy_j: state.start_energy_j,
        energy_defect_j: defect_j,
        residual_conservation_error: defect_j.abs() / scale,
        work_moved_j,
        heat_moved_j,
        heat_share: heat_moved_j / heat_share_base_j(accumulator, work_moved_j),
    };
    if !state_is_finite(&landed) {
        return Err(StepReject::NonFiniteState);
    }
    let gate = accumulator.residual_conservation_limit * scale;
    if defect_j > gate {
        return Err(StepReject::SpontaneousGeneration);
    }
    if defect_j < -gate {
        return Err(StepReject::FirstLawBreach);
    }
    if !heat_share_in_card(accumulator, work_moved_j, heat_moved_j) {
        return Err(StepReject::HeatShareAboveCard);
    }
    *state = landed;
    publish(state, frame);
    Ok(())
}

fn publish(state: &GasState, frame: &mut LastStateFrame64) {
    frame.t = state.t_s;
    frame.pos[0] = state.gas_pressure_pa as f32;
    frame.pos[1] = state.gas_temperature_k as f32;
    frame.pos[2] = state.gas_volume_m3 as f32;
    frame.vel[0] = state.work_on_gas_j as f32;
    frame.vel[1] = state.heat_to_wall_j as f32;
    frame.vel[2] = state.gas_mass_kg as f32;
    frame.force_torque = state.gas_force_n as f32;
    frame.residual = state.residual_conservation_error as f32;
    frame.flags = 0;
}

fn state_is_finite(state: &GasState) -> bool {
    state.t_s.is_finite()
        && state.gas_mass_kg.is_finite()
        && state.wall_temperature_k.is_finite()
        && state.gas_temperature_k.is_finite()
        && state.gas_volume_m3.is_finite()
        && state.gas_pressure_pa.is_finite()
        && state.gas_force_n.is_finite()
        && state.work_on_gas_j.is_finite()
        && state.heat_to_wall_j.is_finite()
        && state.start_energy_j.is_finite()
        && state.energy_defect_j.is_finite()
        && state.residual_conservation_error.is_finite()
        && state.work_moved_j.is_finite()
        && state.heat_moved_j.is_finite()
        && state.heat_share.is_finite()
}

#[cfg(test)]
mod tests {
    extern crate core;
    extern crate std;

    use core::prelude::v1::*;
    use std::assert;
    use std::assert_eq;

    use super::{
        AccumulatorLoad, AccumulatorSbo450, CHARGE_20_BAR_KG, CHARGE_40_BAR_KG, CRITICAL_TEMPERATURE_K, FLAG_OUTSIDE_COMPARED_RUNS, PRESSURE_RATED_PA,
        COMPUTED_LIMIT_ALLOWANCE, FITTED_AREA_OVER_VOLUME_PER_M, GAS_MASS_20_BAR_CEILING_KG,
        GAS_MASS_20_BAR_FLOOR_KG, GAS_MASS_40_BAR_CEILING_KG, GAS_MASS_40_BAR_FLOOR_KG, GasState,
        HEAT_SHARE_LIMIT, HEAT_SHARE_WORK_FLOOR_J, PISTON_AREA_M2, PISTON_BORE_M,
        GAS_TEMPERATURE_CEILING_K, GAS_TEMPERATURE_FLOOR_K, PISTON_SPEED_CEILING_M_S,
        PRESSURE_BANDS_20_BAR_PA, PRESSURE_BANDS_40_BAR_PA, WALL_TEMPERATURE_CEILING_K,
        WALL_TEMPERATURE_FLOOR_K,
        PRESSURE_CEILING_PA, PRESSURE_FLOOR_PA, RESIDUAL_CONSERVATION_LIMIT, StepReject,
        THERMAL_TIME_CONSTANT_S, Trial, VESSEL_GAS_VOLUME_M3, commit, rates, runge_kutta, step,
    };
    use crate::frame::LastStateFrame64;

    const PROOF: [u8; 16] = [0xA5; 16];
    /// Room temperature of the measured runs, K.
    const ROOM_K: f64 = 293.55;

    fn blank() -> LastStateFrame64 {
        LastStateFrame64 {
            t: 0.0,
            pos: [0.0; 3],
            vel: [0.0; 3],
            force_torque: 0.0,
            residual: 0.0,
            flags: 0,
            proof: PROOF,
        }
    }

    fn card() -> AccumulatorSbo450 {
        AccumulatorSbo450::on_darmstadt_rig()
    }

    /// A start of gas that has not moved: both sums carried in are zero.
    fn unmoved(
        card: &AccumulatorSbo450,
        mass: f64,
        pressure: f64,
        temperature: f64,
        wall: f64,
    ) -> Result<GasState, StepReject> {
        GasState::try_start(card, mass, pressure, temperature, wall, 0.0, 0.0)
    }

    fn start(mass: f64, pressure: f64, temperature: f64) -> GasState {
        unmoved(&card(), mass, pressure, temperature, ROOM_K).unwrap()
    }

    fn push(speed: f64) -> AccumulatorLoad {
        AccumulatorLoad { piston_speed_m_s: speed }
    }

    fn relative(a: f64, b: f64) -> f64 {
        ((a - b) / b).abs()
    }

    /// Steps the segments until one is refused. Returns the steps that landed
    /// and the refusal, if any.
    fn run(
        state: &mut GasState,
        frame: &mut LastStateFrame64,
        dt: f64,
        segments: &[(usize, f64)],
    ) -> (usize, Option<StepReject>) {
        let mut landed = 0;
        for &(steps, speed) in segments {
            for _ in 0..steps {
                if let Err(reject) = step(&card(), state, frame, push(speed), dt) {
                    return (landed, Some(reject));
                }
                landed += 1;
            }
        }
        (landed, None)
    }

    /// The speed the scoring holds over step `k` of a sine of this amplitude
    /// from the piston's outermost point, 400 steps to a period.
    fn sine_speed(frequency_hz: f64, amplitude_m: f64, k: usize) -> f64 {
        let dt = 1.0 / (frequency_hz * 400.0);
        let w = 2.0 * core::f64::consts::PI * frequency_hz;
        -amplitude_m * (crate::math::cos(w * (k + 1) as f64 * dt) - crate::math::cos(w * k as f64 * dt)) / dt
    }

    /// The piston is a 50 mm bore, and the vessel volume is that area over
    /// the ratio of area to volume fitted to the opened runs (the frozen fit
    /// of the research record, `analysis/card_constants.json`: 1.92157e-3 m²
    /// and 5.7622e-4 m³). The time constant is that fit's.
    #[test]
    fn the_geometry_is_a_50_mm_bore_and_the_fitted_ratio() {
        assert_eq!(PISTON_BORE_M, 0.05);
        let area = core::f64::consts::PI * PISTON_BORE_M * PISTON_BORE_M / 4.0;
        assert!(relative(PISTON_AREA_M2, area) < 4.0e-16, "{area:e}");
        assert_eq!(FITTED_AREA_OVER_VOLUME_PER_M, 1.92157e-3 / 5.7622e-4);
        assert!(relative(PISTON_AREA_M2 / VESSEL_GAS_VOLUME_M3, FITTED_AREA_OVER_VOLUME_PER_M) < 4.0e-16);
        assert!(relative(VESSEL_GAS_VOLUME_M3, 5.7622e-4 * PISTON_AREA_M2 / 1.92157e-3) < 4.0e-16);
        // 36 cubic inches, the catalogue's effective gas volume, is 0.2 percent more.
        let catalogue = 36.0 * 0.0254 * 0.0254 * 0.0254;
        assert!(catalogue / VESSEL_GAS_VOLUME_M3 - 1.0 > 0.0015 && catalogue / VESSEL_GAS_VOLUME_M3 - 1.0 < 0.0025);
        assert_eq!(THERMAL_TIME_CONSTANT_S, 17.52);
        assert!(relative(THERMAL_TIME_CONSTANT_S, 17.520216456494445) < 2.0e-5);
        let card = card();
        assert!(card.is_valid());
        assert!(relative(card.gate_dt_s().unwrap(), 0.3504) < 1.0e-15);
        assert!(relative(card.floor_dt_s().unwrap(), 0.3504 / 1_048_576.0) < 1.0e-15);
        assert!(relative(card.ledger_scale_j(), 5_063.612_456_911_208) < 1.0e-14);
    }

    /// The two charges are the vessel full of nitrogen at 20 and 40 bar and
    /// 293.15 K. Each lands with no flag at the exact ends of its band, and a
    /// mass a little outside either band, or between the two, lands with the
    /// flag and closes the ledger.
    #[test]
    fn the_gas_mass_is_one_of_the_two_measured_charges() {
        let card = card();
        for (pressure, charge, floor, ceiling) in [
            (2.0e6, CHARGE_20_BAR_KG, GAS_MASS_20_BAR_FLOOR_KG, GAS_MASS_20_BAR_CEILING_KG),
            (4.0e6, CHARGE_40_BAR_KG, GAS_MASS_40_BAR_FLOOR_KG, GAS_MASS_40_BAR_CEILING_KG),
        ] {
            let full = VESSEL_GAS_VOLUME_M3 / card.specific_volume_m3_kg(293.15, pressure).unwrap();
            assert!(relative(charge, full) < 1.0e-13, "{full:e}");
            assert!(1.0 - floor / charge > 0.000_9 && 1.0 - floor / charge < 0.001_1);
            assert!(ceiling / charge - 1.0 > 0.000_9 && ceiling / charge - 1.0 < 0.001_1);
            for mass in [floor, charge, ceiling] {
                let mut state = unmoved(&card, mass, 6.0e6, ROOM_K, ROOM_K).unwrap();
                step(&card, &mut state, &mut blank(), push(0.01), 0.001).unwrap();
            }
            for mass in [f64::from_bits(floor.to_bits() - 1), f64::from_bits(ceiling.to_bits() + 1), floor * 0.999, ceiling * 1.001] {
                let mut state = unmoved(&card, mass, 6.0e6, ROOM_K, ROOM_K).unwrap();
                let mut frame = blank();
                step(&card, &mut state, &mut frame, push(0.01), 0.001).unwrap();
                assert_eq!(frame.flags, FLAG_OUTSIDE_COMPARED_RUNS);
            }
        }
        // The charges as the card was scored, and the charges between the two measured, land
        // with the flag; a mass of zero or less is no charge.
        for mass in [0.01336, 0.02688, 0.015, 0.02, 0.025, 0.013, 0.027, 0.028] {
            let mut state = unmoved(&card, mass, 6.0e6, ROOM_K, ROOM_K).unwrap();
            let mut frame = blank();
            for _ in 0..100 {
                step(&card, &mut state, &mut frame, push(0.1), 0.001).unwrap();
                assert_eq!(frame.flags, FLAG_OUTSIDE_COMPARED_RUNS);
                assert!(state.residual_conservation_error <= RESIDUAL_CONSERVATION_LIMIT);
            }
        }
        for mass in [0.0, -0.0, -0.01] {
            assert_eq!(unmoved(&card, mass, 6.0e6, ROOM_K, ROOM_K), Err(StepReject::GasMassNotPositive));
        }
    }

    /// The equation of state against Python mpmath at 50 digits from the same
    /// constants as 64-bit numbers (`evidence/verification/crosscheck/
    /// accumulator_sbo450/reference_values_amended.txt`): the volume at 20,
    /// 40, 60 and 86 bar at 293.15 K, and the heat capacity and the energy
    /// there.
    #[test]
    fn the_equation_of_state_matches_fifty_digit_arithmetic() {
        let card = card();
        for (pressure, v, cv, u) in [
            (2.0e6, 0.043_125_683_721_496_525, 749.402_339_652_662_83, 212_906.048_999_698_0),
            (4.0e6, 0.021_436_082_005_925_891, 756.125_757_006_605_53, 208_269.829_638_553_19),
            (6.0e6, 0.014_246_161_386_391_950, 762.634_541_286_551_55, 203_781.613_288_499_44),
            (8.6e6, 0.009_937_789_461_532_766_2, 770.727_590_545_752_85, 198_200.946_539_587_17),
        ] {
            let got = card.specific_volume_m3_kg(293.15, pressure).unwrap();
            assert!(relative(got, v) < 1.0e-13, "{pressure}: {got}");
            assert!(relative(card.pressure_pa(293.15, v), pressure) < 1.0e-13);
            assert!(relative(card.heat_capacity_cv_j_kg_k(293.15, v), cv) < 1.0e-13);
            assert!(relative(card.internal_energy_j_kg(293.15, v), u) < 1.0e-13);
        }
    }

    /// Against the NIST WebBook nitrogen isotherm at 293.15 K (Span et al.
    /// 2000; `raw/nist_webbook_nitrogen_isotherm_293.15K_10_150bar.tsv`): the
    /// card's density is 0.5 to 1.5 percent high from 20 to 80 bar, and its
    /// heat capacity 0.2 to 1.0 percent high. Those are the equation's, and
    /// are stated in the card's description.
    #[test]
    fn the_equation_of_state_is_within_two_percent_of_the_reference_equation() {
        let card = card();
        for (pressure, density_mol_l, cv_j_mol_k) in
            [(2.0e6, 0.823_752, 20.9532), (6.0e6, 2.475_80, 21.2207), (8.0e6, 3.293_79, 21.3430)]
        {
            let v = card.specific_volume_m3_kg(293.15, pressure).unwrap();
            let density = density_mol_l * 1.0e3 * 0.028_013_4;
            let high = (1.0 / v) / density - 1.0;
            assert!(high > 0.004 && high < 0.016, "{pressure}: density {high}");
            let cv = card.heat_capacity_cv_j_kg_k(293.15, v) * 0.028_013_4;
            let high = cv / cv_j_mol_k - 1.0;
            assert!(high > 0.001 && high < 0.011, "{pressure}: cv {high}");
        }
    }

    /// Eleven runs against the same 50-digit recomputation
    /// (`recompute_amended.py`): every stepped number within 1e-11, the start
    /// worked back from the pressure within 1e-13, and each run that is
    /// refused lands the same number of steps and ends in the same refusal.
    #[test]
    fn eleven_runs_match_fifty_digit_arithmetic() {
        struct Case {
            mass: f64,
            pressure: f64,
            temperature: f64,
            wall: f64,
            dt: f64,
            segments: &'static [(usize, f64)],
            sine: Option<(f64, f64)>,
            start_volume: f64,
            start_energy: f64,
            landed: usize,
            refusal: Option<StepReject>,
            /// Temperature, volume, pressure, work, heat, work moved, heat moved.
            end: [f64; 7],
        }
        let cases = [
            Case {
                mass: CHARGE_40_BAR_KG, pressure: 6.0e6, temperature: 293.15, wall: ROOM_K, dt: 0.0025,
                segments: &[(40, 0.03)], sine: None,
                start_volume: 0.000_391_304_154_127_105_98, start_energy: 5_597.338_795_465_433_8,
                landed: 40, refusal: None,
                end: [295.116_010_165_196_27, 0.000_385_413_667_901_625_12, 6_138_438.002_182_535_8,
                    35.748_107_347_770_703, 0.067_533_835_028_413_760, 35.748_107_347_770_703,
                    0.077_073_402_042_197_209],
            },
            Case {
                mass: CHARGE_40_BAR_KG, pressure: 6.0e6, temperature: 293.15, wall: ROOM_K, dt: 0.0025,
                segments: &[(20, 0.03), (40, -0.03), (20, 0.03)], sine: None,
                start_volume: 0.000_391_304_154_127_105_98, start_energy: 5_597.338_795_465_433_8,
                landed: 80, refusal: None,
                end: [293.154_427_507_168_02, 0.000_391_304_154_127_105_98, 6_000_104.561_142_492_7,
                    0.000_250_165_980_257_275_82, -0.092_495_217_165_974_412, 70.688_672_760_342_840,
                    0.132_170_474_050_095_81],
            },
            Case {
                mass: CHARGE_40_BAR_KG, pressure: 5.9e6, temperature: 293.9, wall: ROOM_K, dt: 0.3504,
                segments: &[(10, 0.0003), (10, -0.0003)], sine: None,
                start_volume: 0.000_399_151_670_701_603_67, start_energy: 5_620.150_234_869_706_4,
                landed: 20, refusal: None,
                end: [293.679_429_652_705_17, 0.000_399_151_670_701_603_67, 5_894_905.555_799_699_9,
                    0.006_291_106_840_733_302_0, 4.624_293_450_419_182_4, 24.439_131_776_224_719,
                    4.624_293_450_419_182_4],
            },
            // The piston at rest with the gas a quarter of a kelvin over the wall: 5.2 J, inside the share.
            Case {
                mass: CHARGE_40_BAR_KG, pressure: 6.2e6, temperature: 293.8, wall: ROOM_K, dt: 0.25,
                segments: &[(400, 0.0)], sine: None,
                start_volume: 0.000_379_595_984_095_533_44, start_energy: 5_599.841_081_172_446_1,
                landed: 400, refusal: None,
                end: [293.550_969_028_945_72, 0.000_379_595_984_095_533_44, 6_193_916.514_380_923_1,
                    0.0, 5.220_477_192_700_454_5, 0.0, 5.220_477_192_700_454_5],
            },
            // The same rest from 0.95 K over the wall is refused after 28 steps and 6.41 J.
            Case {
                mass: CHARGE_40_BAR_KG, pressure: 6.2e6, temperature: 294.5, wall: ROOM_K, dt: 0.25,
                segments: &[(400, 0.0)], sine: None,
                start_volume: 0.000_380_647_148_555_851_44, start_energy: 5_615.538_187_942_209_8,
                landed: 28, refusal: Some(StepReject::HeatShareAboveCard),
                end: [294.194_011_539_063_36, 0.000_380_647_148_555_851_44, 6_192_549.180_350_575_7,
                    0.0, 6.413_833_830_639_123_1, 0.0, 6.413_833_830_639_123_1],
            },
            Case {
                mass: CHARGE_40_BAR_KG, pressure: 8.0e6, temperature: ROOM_K, wall: ROOM_K, dt: 0.000_125,
                segments: &[(40, 0.5), (40, -0.5)], sine: None,
                start_volume: 0.000_293_814_542_592_066_97, start_energy: 5_487.890_115_592_784_0,
                landed: 80, refusal: None,
                end: [293.549_384_010_175_25, 0.000_293_814_542_592_066_97, 7_999_979.889_464_751_6,
                    0.000_066_490_553_472_666_720, 0.013_074_976_620_664_041, 79.570_262_055_049_204,
                    0.013_074_976_620_664_041],
            },
            Case {
                mass: CHARGE_20_BAR_KG, pressure: 4.0e6, temperature: ROOM_K, wall: ROOM_K, dt: 0.001,
                segments: &[(30, 0.2), (30, -0.2)], sine: None,
                start_volume: 0.000_293_110_818_417_236_15, start_energy: 2_847.811_374_968_040_1,
                landed: 60, refusal: None,
                end: [293.541_458_860_596_26, 0.000_293_110_818_417_236_15, 3_999_871.373_795_923_8,
                    0.001_034_897_663_938_292_7, 0.089_204_550_814_216_381, 97.115_786_056_407_109,
                    0.089_204_550_814_216_381],
            },
            // A measured run at 0.2 Hz as the scoring asks it (record 15478932, 5 mm): two periods land.
            Case {
                mass: CHARGE_40_BAR_KG, pressure: 5_775_825.720_407_209, temperature: 290.379_543_337_863_65,
                wall: 293.563_020_463_611_85, dt: 1.0 / (0.2 * 400.0),
                segments: &[], sine: Some((0.2, 0.004_995_669_974_272_598)),
                start_volume: 0.000_402_133_073_375_126_49, start_energy: 5_549.080_515_815_609_4,
                landed: 800, refusal: None,
                end: [290.379_543_337_863_50, 0.000_402_133_073_375_126_49, 5_775_825.720_407_205_9,
                    0.207_422_386_988_606_17, 0.207_422_386_991_735_12, 470.543_609_255_738_63,
                    23.970_012_384_788_028],
            },
            // The same record's run at 0.05 Hz is refused after 41 steps.
            Case {
                mass: CHARGE_40_BAR_KG, pressure: 5_777_959.075_854_575, temperature: 290.471_087_359_594_95,
                wall: 293.564_905_776_357_65, dt: 1.0 / (0.05 * 400.0),
                segments: &[], sine: Some((0.05, 0.004_997_515_779_679_947)),
                start_volume: 0.000_402_130_614_145_548_29, start_energy: 5_550.994_912_003_603_9,
                landed: 41, refusal: Some(StepReject::HeatShareAboveCard),
                end: [291.408_736_093_848_16, 0.000_400_164_999_968_188_27, 5_827_748.778_894_198_7,
                    11.408_756_827_041_802, -6.486_140_273_482_238_3, 11.408_756_827_041_802,
                    6.486_140_273_482_238_3],
            },
        ];
        for (index, case) in cases.iter().enumerate() {
            let mut state =
                unmoved(&card(), case.mass, case.pressure, case.temperature, case.wall).unwrap();
            assert!(relative(state.gas_volume_m3, case.start_volume) < 1.0e-13, "{index}");
            assert!(relative(state.start_energy_j, case.start_energy) < 1.0e-13, "{index}");
            let mut frame = blank();
            let (landed, refusal) = match case.sine {
                None => run(&mut state, &mut frame, case.dt, case.segments),
                Some((frequency, amplitude)) => {
                    let mut landed = 0;
                    let mut refusal = None;
                    for k in 0..800 {
                        let speed = sine_speed(frequency, amplitude, k);
                        if let Err(reject) = step(&card(), &mut state, &mut frame, push(speed), case.dt) {
                            refusal = Some(reject);
                            break;
                        }
                        landed += 1;
                    }
                    (landed, refusal)
                }
            };
            assert_eq!(landed, case.landed, "{index}");
            assert_eq!(refusal, case.refusal, "{index}");
            let got = [
                state.gas_temperature_k, state.gas_volume_m3, state.gas_pressure_pa,
                state.work_on_gas_j, state.heat_to_wall_j, state.work_moved_j, state.heat_moved_j,
            ];
            for (slot, (got, reference)) in got.iter().zip(case.end).enumerate() {
                if reference == 0.0 {
                    assert_eq!(*got, 0.0, "{index} {slot}");
                } else {
                    // The work and the heat of a closed swing are differences of
                    // sums a hundred thousand times larger.
                    let allowed = if slot >= 3 { 2.0e-9 } else { 1.0e-11 };
                    assert!(relative(*got, reference) < allowed, "{index} {slot}: {got} {reference}");
                }
            }
            let base = if state.work_moved_j > HEAT_SHARE_WORK_FLOOR_J { state.work_moved_j } else { HEAT_SHARE_WORK_FLOOR_J };
            assert_eq!(state.heat_share, state.heat_moved_j / base, "{index}");
            assert!(state.heat_share <= HEAT_SHARE_LIMIT, "{index}");
            assert!(state.residual_conservation_error < 1.0e-13, "{index}");
            assert_eq!(frame.proof, PROOF);
            assert_eq!(frame.flags, 0);
        }
    }

    /// A push far quicker than the time constant follows the constant-entropy
    /// line of the same equation of state. The same travel made slowly would
    /// follow the constant-temperature line, and is refused: the heat passes
    /// its share of the work.
    #[test]
    fn a_quick_push_is_adiabatic_and_a_slow_one_is_refused() {
        let card = card();
        let gas = card.gas();
        let mut quick = unmoved(&card, CHARGE_40_BAR_KG, 6.0e6, ROOM_K, ROOM_K).unwrap();
        let mut slow = quick;
        let mut frame = blank();
        let v = quick.gas_volume_m3 / quick.gas_mass_kg;
        let isothermal = -gas.pressure_per_volume(ROOM_K, v);
        let per_kelvin = gas.pressure_per_kelvin(ROOM_K, v);
        let adiabatic = isothermal + ROOM_K * per_kelvin * per_kelvin / gas.cv(ROOM_K, v);
        // 0.02 mm in 0.4 ms.
        assert_eq!(run(&mut quick, &mut frame, 1.0e-5, &[(40, 0.05)]), (40, None));
        let stiffness = (quick.gas_pressure_pa - 6.0e6) / (v - quick.gas_volume_m3 / quick.gas_mass_kg);
        assert!(relative(stiffness, adiabatic) < 2.0e-4, "{}", stiffness / adiabatic);
        // The ratio of the two is the ratio of the heat capacities, 1.51 at 60 bar by the reference equation.
        assert!(adiabatic / isothermal > 1.48 && adiabatic / isothermal < 1.54, "{}", adiabatic / isothermal);
        // 20 mm at 0.1 mm/s: 200 s, eleven time constants.
        let (landed, refusal) = run(&mut slow, &mut frame, 0.35, &[(572, 1.0e-4)]);
        assert_eq!(refusal, Some(StepReject::HeatShareAboveCard));
        assert!(landed > 10 && landed < 572, "{landed}");
        assert!(slow.heat_share <= HEAT_SHARE_LIMIT);
        assert_eq!(frame.t, slow.t_s);
    }

    /// Gas a quarter of a kelvin warmer than the wall with the piston still:
    /// the difference falls by 1/e in the time constant times `c_v / c_v0`,
    /// all the heat comes out of the gas's energy, and the 5.2 J are inside
    /// the share. With the gas at the wall's temperature a rest exchanges
    /// nothing and lands for as long as it is asked.
    #[test]
    fn still_gas_cools_to_the_wall_with_the_time_constant() {
        let card = card();
        let mut state = start(CHARGE_40_BAR_KG, 6.0e6, ROOM_K + 0.25);
        let mut frame = blank();
        let ratio = card.heat_capacity_cv_j_kg_k(ROOM_K, state.gas_volume_m3 / state.gas_mass_kg)
            / card.gas().ideal_cv;
        let steps = 200;
        let dt = THERMAL_TIME_CONSTANT_S * ratio / steps as f64;
        assert_eq!(run(&mut state, &mut frame, dt, &[(steps, 0.0)]), (steps, None));
        let left = (state.gas_temperature_k - ROOM_K) / 0.25;
        assert!((left - (-1.0_f64).exp()).abs() < 2.0e-4, "{left}");
        assert_eq!(state.work_on_gas_j, 0.0);
        assert_eq!(state.work_moved_j, 0.0);
        assert!(state.heat_to_wall_j > 3.2 && state.heat_to_wall_j < 3.4, "{}", state.heat_to_wall_j);
        assert_eq!(state.heat_moved_j, state.heat_to_wall_j);
        assert_eq!(state.heat_share, state.heat_moved_j / HEAT_SHARE_WORK_FLOOR_J);
        assert!(state.residual_conservation_error < 1.0e-13);
        assert_eq!(frame.flags, 0);

        let mut level = start(CHARGE_40_BAR_KG, 6.0e6, ROOM_K);
        assert_eq!(run(&mut level, &mut frame, 0.35, &[(10_000, 0.0)]), (10_000, None));
        assert_eq!(level.heat_moved_j, 0.0);
        assert_eq!(level.gas_temperature_k, ROOM_K);
    }

    /// The edges of the compared range: a start or a step exactly on one
    /// lands with no flag, and one a thousandth beyond lands with the flag.
    /// What stops the card is physical: nitrogen's critical temperature, the
    /// vessel's rated 330 bar, the oil side empty, and the piston's millimetre
    /// a step.
    #[test]
    fn the_edges_of_the_compared_range_flag_and_the_physical_limits_refuse() {
        let card = card();
        let mut frame = blank();
        assert_eq!(PRESSURE_FLOOR_PA, 3.7e6);
        assert_eq!(PRESSURE_CEILING_PA, 8.6e6);
        assert_eq!(PRESSURE_RATED_PA, 3.3e7);
        assert_eq!(PRESSURE_BANDS_20_BAR_PA, [[3.7e6, 4.3e6], [5.5e6, 6.7e6]]);
        assert_eq!(PRESSURE_BANDS_40_BAR_PA, [[5.7e6, 6.4e6], [7.5e6, 8.6e6]]);
        for (mass, bands) in [(CHARGE_20_BAR_KG, PRESSURE_BANDS_20_BAR_PA), (CHARGE_40_BAR_KG, PRESSURE_BANDS_40_BAR_PA)] {
            for band in bands {
                for pressure in band {
                    let mut state = unmoved(&card, mass, pressure, ROOM_K, ROOM_K).unwrap();
                    assert_eq!(run(&mut state, &mut frame, 0.01, &[(3, 0.0)]), (3, None));
                    assert_eq!(frame.flags, 0);
                    assert!(relative(state.gas_pressure_pa, pressure) < 1.0e-13);
                }
            }
        }
        // Never measured for the charge given, the gaps, the far side and the
        // record at 120 bar that missed: each lands with the flag.
        for (pressure, mass) in [
            (8.0e6, CHARGE_20_BAR_KG), (4.5e6, CHARGE_40_BAR_KG), (7.0e6, CHARGE_40_BAR_KG),
            (5.0e6, CHARGE_20_BAR_KG), (7.5e6, CHARGE_20_BAR_KG), (8.6e6, CHARGE_20_BAR_KG),
            (4.2e6, CHARGE_40_BAR_KG), (5.6e6, CHARGE_40_BAR_KG),
            (8.7e6, CHARGE_20_BAR_KG), (1.09e7, CHARGE_40_BAR_KG), (1.32e7, CHARGE_40_BAR_KG),
            (2.0e7, CHARGE_40_BAR_KG), (3.0e6, CHARGE_20_BAR_KG), (2.4e7, CHARGE_20_BAR_KG),
        ] {
            let mut state = unmoved(&card, mass, pressure, ROOM_K, ROOM_K).unwrap();
            step(&card, &mut state, &mut frame, push(0.0), 0.01).unwrap();
            assert_eq!(frame.flags, FLAG_OUTSIDE_COMPARED_RUNS, "{mass} {pressure}");
        }
        // Above the rated 330 bar, and at or below nothing, is refused;
        // between 250 and 330 bar, the lower rating of two listed parts, a
        // start lands, flagged.
        for pressure in [2.6e7, 3.0e7, 3.29e7] {
            let mut state = unmoved(&card, CHARGE_40_BAR_KG, pressure, ROOM_K, ROOM_K).unwrap();
            step(&card, &mut state, &mut frame, push(0.0), 0.01).unwrap();
            assert_eq!(frame.flags, FLAG_OUTSIDE_COMPARED_RUNS, "{pressure}");
        }
        for pressure in [3.3e7 * 1.001, 3.5e7, 4.5e7] {
            assert_eq!(unmoved(&card, CHARGE_40_BAR_KG, pressure, ROOM_K, ROOM_K), Err(StepReject::PressureAboveRated));
        }
        // The gas and the wall temperatures compared, to the next tenth of a kelvin.
        assert_eq!((GAS_TEMPERATURE_FLOOR_K, GAS_TEMPERATURE_CEILING_K), (285.9, 301.8));
        assert_eq!((WALL_TEMPERATURE_FLOOR_K, WALL_TEMPERATURE_CEILING_K), (293.4, 293.7));
        assert_eq!(PISTON_SPEED_CEILING_M_S, 0.7);
        for temperature in [285.0, 302.5, 283.15, 303.15, 200.0, 400.0, 130.0] {
            let mut state = unmoved(&card, CHARGE_40_BAR_KG, 6.0e6, temperature, ROOM_K).unwrap();
            step(&card, &mut state, &mut frame, push(0.0), 0.01).unwrap();
            assert_eq!(frame.flags, FLAG_OUTSIDE_COMPARED_RUNS, "{temperature}");
        }
        for temperature in [CRITICAL_TEMPERATURE_K, 120.0, 77.0, 0.0, -1.0] {
            assert_eq!(
                unmoved(&card, CHARGE_40_BAR_KG, 6.0e6, temperature, ROOM_K),
                Err(StepReject::GasBelowCriticalTemperature),
                "{temperature}"
            );
        }
        for wall in [293.399, 293.701, 288.15, 293.15, 298.15, 350.0] {
            let mut state = unmoved(&card, CHARGE_40_BAR_KG, 6.0e6, ROOM_K, wall).unwrap();
            step(&card, &mut state, &mut frame, push(0.0), 0.01).unwrap();
            assert_eq!(frame.flags, FLAG_OUTSIDE_COMPARED_RUNS, "{wall}");
        }
        for wall in [0.0, -1.0] {
            assert_eq!(unmoved(&card, CHARGE_40_BAR_KG, 6.0e6, ROOM_K, wall), Err(StepReject::WallTemperatureNotPositive));
        }
        // One millimetre in a step, stated four ways, and the fastest compared speed.
        for (speed, dt) in [(0.5, 0.002), (0.25, 0.004), (0.1, 0.01), (-0.004, 0.25), (0.7, 1.0e-4), (-0.7, 1.0e-4)] {
            let mut state = start(CHARGE_40_BAR_KG, 8.0e6, ROOM_K);
            step(&card, &mut state, &mut frame, push(speed), dt).unwrap();
            assert_eq!(frame.flags, 0);
        }
        let mut state = start(CHARGE_40_BAR_KG, 8.0e6, ROOM_K);
        let saved = state;
        assert_eq!(
            step(&card, &mut state, &mut frame, push(0.5), 0.002_002),
            Err(StepReject::PistonTravelAboveStep)
        );
        assert_eq!(state, saved);
        // Faster than any compared run lands with the flag, within the millimetre.
        for speed in [0.7007, 0.75, -0.75, 0.8, -0.8, 5.0] {
            let mut state = start(CHARGE_40_BAR_KG, 8.0e6, ROOM_K);
            step(&card, &mut state, &mut frame, push(speed), 1.0e-4).unwrap();
            assert_eq!(frame.flags, FLAG_OUTSIDE_COMPARED_RUNS, "{speed}");
        }
    }

    /// The record at 109 to 132 bar, compared and missed, and pressures far
    /// beyond it: a push from 85.5 bar runs on through 109 and 132 bar, every
    /// step flagged above 86 bar and the ledger closing, until the vessel's
    /// rated 330 bar stops it by name with nothing moved. A draw from 40 bar
    /// on the 40 bar charge ends where the oil side empties.
    #[test]
    fn a_push_past_the_compared_pressures_runs_to_the_rated_pressure() {
        let card = card();
        let mut state = start(CHARGE_40_BAR_KG, 8.55e6, ROOM_K);
        let mut frame = blank();
        let mut landed = 0u32;
        let mut worst = 0.0_f64;
        let mut passed_132_bar = false;
        let reject = loop {
            let before = state;
            match step(&card, &mut state, &mut frame, push(0.1), 0.001) {
                Ok(()) => {
                    landed += 1;
                    worst = worst.max(state.residual_conservation_error);
                    if state.gas_pressure_pa > 8.6e6 * (1.0 + 1.0e-9) {
                        assert_eq!(frame.flags, FLAG_OUTSIDE_COMPARED_RUNS);
                    }
                    passed_132_bar |= state.gas_pressure_pa > 1.32e7;
                }
                Err(reject) => {
                    assert_eq!(state, before);
                    break reject;
                }
            }
            assert!(landed < 100_000);
        };
        assert!(passed_132_bar);
        assert_eq!(reject, StepReject::PressureAboveRated);
        assert!(state.gas_pressure_pa <= 3.3e7 && state.gas_pressure_pa > 3.2e7, "{}", state.gas_pressure_pa);
        assert!(worst <= RESIDUAL_CONSERVATION_LIMIT, "{worst:e}");
        // A draw ends where the oil side empties.
        let mut state = start(CHARGE_40_BAR_KG, 4.2e6, ROOM_K);
        let reject = loop {
            match step(&card, &mut state, &mut frame, push(-0.1), 0.001) {
                Ok(()) => assert!(state.residual_conservation_error <= RESIDUAL_CONSERVATION_LIMIT),
                Err(reject) => break reject,
            }
        };
        assert_eq!(reject, StepReject::OilSideEmpty);
    }

    /// The heat share at its exact end is inside, under the work floor and
    /// over it, and a run that reaches the limit exactly keeps that step. A
    /// part in a thousand million over is named.
    #[test]
    fn the_exact_end_of_the_heat_share_is_inside() {
        let card = card();
        assert_eq!(HEAT_SHARE_LIMIT, 0.065);
        assert_eq!(HEAT_SHARE_WORK_FLOOR_J, 100.0);
        for work_moved in [0.0, 40.0, 100.0, 100.000_001, 1_000.0, 123_456.789] {
            let base = if work_moved > 100.0 { work_moved } else { 100.0 };
            let mut state = start(CHARGE_40_BAR_KG, 6.0e6, ROOM_K);
            state.work_moved_j = work_moved;
            state.heat_moved_j = HEAT_SHARE_LIMIT * base;
            let mut frame = blank();
            // At rest at the wall's temperature a step moves no heat: the run stays exactly on the limit.
            assert_eq!(run(&mut state, &mut frame, 0.1, &[(5, 0.0)]), (5, None), "{work_moved}");
            assert_eq!(state.heat_share, HEAT_SHARE_LIMIT * base / base);
            assert_eq!(frame.t, state.t_s);
            let mut over = state;
            over.heat_moved_j = HEAT_SHARE_LIMIT * base * (1.0 + 1.0e-9);
            let saved = over;
            assert_eq!(
                step(&card, &mut over, &mut frame, push(0.0), 0.1),
                Err(StepReject::HeatShareAboveCard),
                "{work_moved}"
            );
            assert_eq!(over, saved);
            let mut within = state;
            within.heat_moved_j = HEAT_SHARE_LIMIT * base * (1.0 + 0.5 * COMPUTED_LIMIT_ALLOWANCE);
            step(&card, &mut within, &mut frame, push(0.0), 0.1).unwrap();
        }
        // A step whose own heat carries the run over the limit is refused and the step before it is kept.
        let mut state = start(CHARGE_40_BAR_KG, 6.0e6, ROOM_K + 1.0);
        let mut frame = blank();
        let (landed, refusal) = run(&mut state, &mut frame, 0.3504, &[(1_000, 0.0)]);
        assert_eq!(refusal, Some(StepReject::HeatShareAboveCard));
        assert!(landed > 5 && landed < 40, "{landed}");
        assert!(state.heat_moved_j <= HEAT_SHARE_LIMIT * HEAT_SHARE_WORK_FLOOR_J);
        assert!(relative(state.t_s, landed as f64 * 0.3504) < 1.0e-12);
        assert_eq!(frame.t, state.t_s);
        assert_eq!(frame.vel[1], state.heat_to_wall_j as f32);
    }

    /// The measured runs of the record at 20 bar precharge and 60 bar, 5 mm,
    /// as the scoring asks them (`runs/held_back/11265436.jsonl`, with the
    /// charge of the amended vessel): at 0.2 Hz two periods land with a share
    /// of 0.060 at most, and at 0.1 Hz the run is refused in its first period.
    #[test]
    fn a_sine_at_a_fifth_of_a_hertz_lands_and_one_at_a_tenth_is_refused() {
        let card = card();
        let mut frame = blank();
        let mut state = unmoved(
            &card, CHARGE_20_BAR_KG, 5_562_310.731_761_613, 287.254_261_423_625, 293.552_334_697_530_4,
        )
        .unwrap();
        let mut largest: f64 = 0.0;
        for k in 0..800 {
            let speed = sine_speed(0.2, 0.004_989_864_801_862_285, k);
            step(&card, &mut state, &mut frame, push(speed), 1.0 / 80.0).unwrap();
            largest = largest.max(state.heat_share);
        }
        assert!(largest > 0.055 && largest < 0.062, "{largest}");
        let mut state = unmoved(
            &card, CHARGE_20_BAR_KG, 5_566_029.968_360_627, 287.285_217_526_049_34, 293.551_841_507_571_4,
        )
        .unwrap();
        let mut landed = 0;
        let mut refusal = None;
        for k in 0..800 {
            let speed = sine_speed(0.1, 0.004_991_137_886_577_627, k);
            match step(&card, &mut state, &mut frame, push(speed), 1.0 / 40.0) {
                Ok(()) => landed += 1,
                Err(reject) => {
                    refusal = Some(reject);
                    break;
                }
            }
        }
        assert_eq!(refusal, Some(StepReject::HeatShareAboveCard));
        assert!(landed > 20 && landed < 400, "{landed}");
        assert!(state.heat_share <= HEAT_SHARE_LIMIT);
    }

    /// Each named limit is reached, and the state, the frame and its proof
    /// are as they were.
    #[test]
    fn each_limit_is_named_and_leaves_the_state_and_the_proof() {
        let card = card();
        let fresh = start(CHARGE_40_BAR_KG, 6.0e6, ROOM_K);
        let refused = |state: GasState, card: &AccumulatorSbo450, speed: f64, dt: f64| {
            let mut trial = state;
            let mut frame = blank();
            frame.t = 7.0;
            let answer = step(card, &mut trial, &mut frame, push(speed), dt);
            assert_eq!(trial.gas_temperature_k.to_bits(), state.gas_temperature_k.to_bits());
            assert_eq!(trial.gas_volume_m3.to_bits(), state.gas_volume_m3.to_bits());
            assert_eq!(trial.work_on_gas_j.to_bits(), state.work_on_gas_j.to_bits());
            assert_eq!(trial.work_moved_j.to_bits(), state.work_moved_j.to_bits());
            assert_eq!(trial.heat_moved_j.to_bits(), state.heat_moved_j.to_bits());
            assert_eq!(trial.t_s.to_bits(), state.t_s.to_bits());
            assert_eq!(frame.t, 7.0);
            assert_eq!(frame.proof, PROOF);
            answer
        };
        assert_eq!(refused(fresh, &card, f64::NAN, 0.01), Err(StepReject::NonFiniteInput));
        assert_eq!(refused(fresh, &card, 0.01, f64::INFINITY), Err(StepReject::NonFiniteInput));
        assert_eq!(refused(fresh, &card, 0.01, 0.0), Err(StepReject::NonPositiveDt));
        assert_eq!(refused(fresh, &card, 0.01, -0.01), Err(StepReject::NonPositiveDt));
        let mut broken = fresh;
        broken.heat_to_wall_j = f64::NAN;
        assert_eq!(refused(broken, &card, 0.01, 0.01), Err(StepReject::NonFiniteState));
        let mut broken = fresh;
        broken.heat_moved_j = f64::NAN;
        assert_eq!(refused(broken, &card, 0.01, 0.01), Err(StepReject::NonFiniteState));
        let mut broken = fresh;
        broken.work_moved_j = -1.0;
        assert_eq!(refused(broken, &card, 0.01, 0.01), Err(StepReject::NonFiniteState));
        let mut other = card;
        other.thermal_time_constant_s = 5.0;
        assert_eq!(refused(fresh, &other, 0.01, 0.01), Err(StepReject::InvalidAccumulator));
        let mut other = card;
        other.vessel_gas_volume_m3 = 5.7622e-4;
        assert_eq!(refused(fresh, &other, 0.01, 0.01), Err(StepReject::InvalidAccumulator));
        let mut other = card;
        other.piston_area_m2 = 1.92157e-3;
        assert_eq!(refused(fresh, &other, 0.01, 0.01), Err(StepReject::InvalidAccumulator));
        let mut other = card;
        other.heat_share_limit = 1.0;
        assert_eq!(refused(fresh, &other, 0.01, 0.01), Err(StepReject::InvalidAccumulator));
        let mut other = card;
        other.pressure_ceiling_pa = 1.3e7;
        assert_eq!(refused(fresh, &other, 0.01, 0.01), Err(StepReject::InvalidAccumulator));
        let mut other = card;
        other.pressure_bands_20_bar_pa[1][1] = 8.6e6;
        assert_eq!(refused(fresh, &other, 0.01, 0.01), Err(StepReject::InvalidAccumulator));
        let mut other = card;
        other.pressure_bands_40_bar_pa[0][0] = 4.0e6;
        assert_eq!(refused(fresh, &other, 0.01, 0.01), Err(StepReject::InvalidAccumulator));
        let mut weightless = fresh;
        weightless.gas_mass_kg = 0.0;
        assert_eq!(refused(weightless, &card, 0.01, 0.01), Err(StepReject::GasMassNotPositive));
        let mut cold_wall = fresh;
        cold_wall.wall_temperature_k = 0.0;
        assert_eq!(refused(cold_wall, &card, 0.01, 0.01), Err(StepReject::WallTemperatureNotPositive));
        assert_eq!(refused(fresh, &card, 0.5, 0.01), Err(StepReject::PistonTravelAboveStep));
        let mut condensing = fresh;
        condensing.gas_temperature_k = 120.0;
        assert_eq!(refused(condensing, &card, 0.0, 0.01), Err(StepReject::GasBelowCriticalTemperature));
        // Near the rated pressure a push lands above 330 bar.
        let high = start(CHARGE_40_BAR_KG, 3.299e7, ROOM_K);
        assert_eq!(refused(high, &card, 0.7, 0.001_25), Err(StepReject::PressureAboveRated));
        // A state written with more gas than the vessel holds: a pull has no oil to draw.
        let mut brim = fresh;
        brim.gas_volume_m3 = VESSEL_GAS_VOLUME_M3 * 1.001;
        assert_eq!(refused(brim, &card, -0.1, 0.01), Err(StepReject::OilSideEmpty));
        // Gas four kelvin over the wall at rest with 6.4 J already moved: the next step passes 6.5 J.
        let mut spent = start(CHARGE_40_BAR_KG, 6.0e6, ROOM_K + 4.0);
        spent.heat_moved_j = 6.4;
        assert_eq!(refused(spent, &card, 0.0, 0.3504), Err(StepReject::HeatShareAboveCard));
        for (mass, pressure, temperature, wall, name) in [
            (f64::NAN, 6.0e6, ROOM_K, ROOM_K, StepReject::NonFiniteInput),
            (CHARGE_40_BAR_KG, 6.0e6, 100.0, ROOM_K, StepReject::GasBelowCriticalTemperature),
            (CHARGE_40_BAR_KG, 6.0e6, ROOM_K, -5.0, StepReject::WallTemperatureNotPositive),
            (CHARGE_40_BAR_KG, 3.4e7, ROOM_K, ROOM_K, StepReject::PressureAboveRated),
            (CHARGE_40_BAR_KG, 2.0e6, ROOM_K, ROOM_K, StepReject::OilSideEmpty),
            (-0.02, 6.0e6, ROOM_K, ROOM_K, StepReject::GasMassNotPositive),
        ] {
            assert_eq!(unmoved(&card, mass, pressure, temperature, wall), Err(name));
        }
        // The sums carried into a start.
        for (work, heat, name) in [
            (f64::NAN, 0.0, StepReject::NonFiniteInput),
            (0.0, f64::INFINITY, StepReject::NonFiniteInput),
            (-1.0e-9, 0.0, StepReject::NegativeMovedSum),
            (100.0, -1.0, StepReject::NegativeMovedSum),
            (0.0, 6.5 * (1.0 + 1.0e-9), StepReject::HeatShareAboveCard),
            (1_000.0, 65.0 * (1.0 + 1.0e-9), StepReject::HeatShareAboveCard),
        ] {
            assert_eq!(
                GasState::try_start(&card, CHARGE_40_BAR_KG, 6.0e6, ROOM_K, ROOM_K, work, heat),
                Err(name),
                "{work} {heat}"
            );
        }
    }

    /// The sums of the motion before a start travel with it. A slow push
    /// asked as one run is refused at step 20; carried on from the state of
    /// step 19 with its sums, step 20 is refused in the same way, as a single
    /// step and as the first step of a run; carried on with the sums at zero
    /// it would land. Where the share does not bind, a run cut in two and
    /// carried on lands where the run in one piece lands, with the same sums.
    #[test]
    fn the_heat_share_travels_with_a_continuation() {
        let card = card();
        // 50 bar is outside both bands of the 20 bar charge and a wall at 293.15 K was never
        // compared, so the run is asked at 60 bar with gas and wall at 293.5 K.
        let begin = unmoved(&card, CHARGE_20_BAR_KG, 6.0e6, 293.5, 293.5).unwrap();
        let mut straight = begin;
        let mut frame = blank();
        let (landed, refusal) = run(&mut straight, &mut frame, 0.35, &[(30, 0.0005)]);
        assert_eq!(refusal, Some(StepReject::HeatShareAboveCard));
        assert!(landed > 5 && landed < 30, "{landed}");
        let carried = GasState::try_start(
            &card,
            straight.gas_mass_kg,
            straight.gas_pressure_pa,
            straight.gas_temperature_k,
            straight.wall_temperature_k,
            straight.work_moved_j,
            straight.heat_moved_j,
        )
        .unwrap();
        assert_eq!(carried.work_moved_j, straight.work_moved_j);
        assert_eq!(carried.heat_moved_j, straight.heat_moved_j);
        assert_eq!(carried.heat_share, straight.heat_share);
        assert_eq!(carried.t_s, 0.0);
        assert_eq!(carried.work_on_gas_j, 0.0);
        let mut again = carried;
        assert_eq!(step(&card, &mut again, &mut frame, push(0.0005), 0.35), Err(StepReject::HeatShareAboveCard));
        assert_eq!(again, carried);
        // The same state with its sums dropped: the step the run refused would land.
        let mut dropped = unmoved(
            &card,
            straight.gas_mass_kg,
            straight.gas_pressure_pa,
            straight.gas_temperature_k,
            straight.wall_temperature_k,
        )
        .unwrap();
        step(&card, &mut dropped, &mut frame, push(0.0005), 0.35).unwrap();

        // A swing at 0.5 Hz where the share does not bind, in one run and in two.
        let segments: [(usize, f64); 4] = [(25, 0.01), (50, -0.01), (50, 0.01), (25, -0.01)];
        let mut whole = begin;
        assert_eq!(run(&mut whole, &mut frame, 0.01, &segments), (150, None));
        let mut first = begin;
        assert_eq!(run(&mut first, &mut frame, 0.01, &segments[..2]), (75, None));
        let mut second = GasState::try_start(
            &card,
            first.gas_mass_kg,
            first.gas_pressure_pa,
            first.gas_temperature_k,
            first.wall_temperature_k,
            first.work_moved_j,
            first.heat_moved_j,
        )
        .unwrap();
        assert_eq!(run(&mut second, &mut frame, 0.01, &segments[2..]), (75, None));
        assert!(relative(second.gas_temperature_k, whole.gas_temperature_k) < 1.0e-13);
        assert!(relative(second.gas_pressure_pa, whole.gas_pressure_pa) < 1.0e-12);
        assert!(relative(second.work_moved_j, whole.work_moved_j) < 1.0e-12);
        assert!(relative(second.heat_moved_j, whole.heat_moved_j) < 1.0e-9);
        assert!(relative(second.heat_share, whole.heat_share) < 1.0e-9);
    }

    /// Rule 10. A step by explicit Euler, and a Runge-Kutta step whose heat
    /// was halved, are each named by the ledger, and the state is kept.
    #[test]
    fn a_wrong_stepping_method_and_a_falsified_ledger_are_named() {
        let card = card();
        let fresh = start(CHARGE_40_BAR_KG, 6.0e6, 296.0);
        let load = push(0.05);
        let dt = 0.02;
        let gas = card.gas();
        let at = Trial {
            gas_temperature_k: fresh.gas_temperature_k,
            gas_volume_m3: fresh.gas_volume_m3,
            work_on_gas_j: 0.0,
            heat_to_wall_j: 0.0,
        };
        let slope = rates(&card, &gas, fresh.gas_mass_kg, ROOM_K, load.piston_speed_m_s, at);
        let euler = Trial {
            gas_temperature_k: at.gas_temperature_k + dt * slope.temperature_k_s,
            gas_volume_m3: at.gas_volume_m3 + dt * slope.volume_m3_s,
            work_on_gas_j: dt * slope.work_w,
            heat_to_wall_j: dt * slope.heat_w,
        };
        let mut state = fresh;
        let mut frame = blank();
        let named = commit(&card, &mut state, &mut frame, dt, euler);
        assert!(
            named == Err(StepReject::SpontaneousGeneration) || named == Err(StepReject::FirstLawBreach),
            "{named:?}"
        );
        assert_eq!(state, fresh);
        assert_eq!(frame.proof, PROOF);
        assert_eq!(frame.t, 0.0);

        let honest = runge_kutta(&card, &fresh, load, dt);
        let mut forged = honest;
        forged.heat_to_wall_j *= 0.5;
        assert_eq!(
            commit(&card, &mut state, &mut frame, dt, forged),
            Err(StepReject::FirstLawBreach)
        );
        let mut forged = honest;
        forged.work_on_gas_j *= 0.999;
        assert_eq!(
            commit(&card, &mut state, &mut frame, dt, forged),
            Err(StepReject::SpontaneousGeneration)
        );
        assert_eq!(state, fresh);
        commit(&card, &mut state, &mut frame, dt, honest).unwrap();
        assert!(state.residual_conservation_error < 1.0e-13);
    }

    /// The frame carries the pressure, the temperature, the volume, the
    /// work, the heat, the mass, the force on the 50 mm piston and the
    /// residual, and its flags are zero at every pressure: the card raises
    /// no flag.
    #[test]
    fn the_frame_carries_the_state_and_no_flag() {
        let card = card();
        let mut frame = blank();
        for (mass, pressure) in [
            (CHARGE_40_BAR_KG, 6.0e6),
            (CHARGE_40_BAR_KG, 6.3e6),
            (CHARGE_40_BAR_KG, 8.0e6),
            (CHARGE_20_BAR_KG, 4.0e6),
            (CHARGE_20_BAR_KG, 6.5e6),
        ] {
            let mut state = start(mass, pressure, ROOM_K);
            frame.flags = u64::MAX;
            step(&card, &mut state, &mut frame, push(0.01), 0.001).unwrap();
            assert_eq!(frame.flags, 0);
            assert_eq!(frame.t, state.t_s);
            assert_eq!(frame.pos[0], state.gas_pressure_pa as f32);
            assert_eq!(frame.pos[1], state.gas_temperature_k as f32);
            assert_eq!(frame.pos[2], state.gas_volume_m3 as f32);
            assert_eq!(frame.vel[0], state.work_on_gas_j as f32);
            assert_eq!(frame.vel[1], state.heat_to_wall_j as f32);
            assert_eq!(frame.vel[2], mass as f32);
            assert_eq!(frame.force_torque, (state.gas_pressure_pa * PISTON_AREA_M2) as f32);
            assert_eq!(frame.residual, state.residual_conservation_error as f32);
        }
    }

    /// Two hundred thousand steps of the measured 5 mm, 18 Hz run at 40 bar
    /// precharge and 80 bar (`runs/held_back/11265583.jsonl`), the run with
    /// the highest pressure inside the card: the residual stays a thousand
    /// times under its limit, the pressure stays under 86 bar and the heat
    /// share stays under a thousandth.
    #[test]
    fn a_long_fast_swing_keeps_its_ledger() {
        let card = card();
        let mut state = unmoved(
            &card, CHARGE_40_BAR_KG, 7_523_342.015_237_681, 288.377_611_384_481_04, 293.597_813_650_145_8,
        )
        .unwrap();
        let mut frame = blank();
        let dt = 1.0 / (18.0 * 400.0);
        let mut worst: f64 = 0.0;
        let mut highest: f64 = 0.0;
        for index in 0..200_000_usize {
            let speed = sine_speed(18.0, 0.006_004_701_850_574_627_5, index % 400);
            step(&card, &mut state, &mut frame, push(speed), dt).unwrap();
            worst = worst.max(state.residual_conservation_error);
            highest = highest.max(state.gas_pressure_pa);
        }
        assert!(worst < RESIDUAL_CONSERVATION_LIMIT * 1.0e-3, "{worst}");
        assert!(highest > 8.5e6 && highest < 8.53e6, "{highest}");
        assert!(state.heat_share < 1.0e-3, "{}", state.heat_share);
        // Five hundred whole periods: the gas is back where it started and the work equals the heat.
        let gained = state.gas_mass_kg
            * card.internal_energy_j_kg(state.gas_temperature_k, state.gas_volume_m3 / state.gas_mass_kg)
            - state.start_energy_j;
        assert!((state.work_on_gas_j - state.heat_to_wall_j - gained).abs() < 1.0e-9 * card.ledger_scale_j());
        assert!(state.heat_to_wall_j > 0.0);
    }
}
