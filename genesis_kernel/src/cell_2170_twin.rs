#![no_implicit_prelude]

//! One Tesla Model 3 2170 lithium-ion cell as one physical model with
//! memory, under a terminal current and a measured case temperature, from
//! -20 to 40 degC: the twin of the three cells `m80`, `m1000` and `m448-N`,
//! calibrated on all their published data.
//!
//! Measured by Kollmeyer, Khanum, Naguib and Emadi, "Tesla Model 3 2170
//! Li-ion Cell Dataset and Battery SOC Estimation Blind Modeling Tool",
//! Borealis, doi: 10.5683/SP3/ZVTR4B, version 3.0, CC BY 4.0.
//!
//! The card returns the voltage at the cell's terminals and the temperature
//! of its core, given the current and the temperature measured on the
//! cell's case. It is `cell_2170_model3`'s structure with what the cold
//! needs added:
//!
//! - **the core temperature.** One heat capacity behind one thermal
//!   resistance from the measured case. The step's own heat, every part the
//!   energy ledger counts, warms the core above the case, and every
//!   temperature law below is read at the core, not at the case. At -20 degC
//!   a discharge at the rated current warms the core several kelvin above the
//!   case, and the cold resistances fall with it.
//! - **charge-transfer kinetics with temperature.** Butler-Volmer, in its
//!   exact solution with the double layer beside it for a held current, with
//!   an Arrhenius exchange current and a slope `2RT/F` read at the core.
//! - **the salt's transport limit.** The slow electrolyte drop of
//!   `cell_2170_model3` is the concentration difference across the cell in
//!   the volts of its small-signal limit. The drop at the terminals grows as
//!   `atanh` of it over `(2RT/F) / k_e`: as the salt at one electrode runs
//!   out the drop grows without bound. Its resistance and its time follow a
//!   Vogel law in the core temperature, which bends from Arrhenius toward the
//!   cold. A step whose concentration difference reaches the limit is refused
//!   as [`StepReject::ElectrolyteDepleted`]: the salt at one electrode is
//!   gone and the cell cannot carry the current.
//! - **the measured case temperature,** each step, as the boundary of the
//!   core.
//!
//! The fitted numbers, the cells' rows, the curves and the compared range
//! are in `cell_2170_twin_curves.rs`, written by the study's
//! `code/export_twin_card.py` from its fit. Each is named there with its
//! meaning and the fit it came from.
//!
//! # Which cell
//!
//! The equations, the curves and the fitted numbers describe the cell type.
//! Three numbers belong to one cell, each a locked row of [`CELLS`], picked
//! by `cell_number` in the starting state: 80 for `m80`, 1000 for `m1000` and
//! 448 for `m448-N`. A row holds the cell's capacity at its first capacity
//! test (a slow discharge at 40 degC to 2.8 V), measured; its charge-return
//! ratio, measured at that test and the recharge after it; and one fitted
//! resistance scale, which multiplies the two immediate resistances and the
//! electrolyte's and divides the two exchange currents. Any other
//! `cell_number` is refused as [`StepReject::CellNotRecognized`].
//!
//! # What a caller supplies
//!
//! The current, the measured case temperature and the step, at every step;
//! and the starting state: `cell_number`, the state of charge and the
//! capacity from the last full charge (the cell ages, so the capacity on the
//! day is the caller's, read from a recharge record as `cell_2170_model3`
//! describes). A rested start has every memory at zero and its core at the
//! case temperature of its first step.
//!
//! # The step
//!
//! `current_a > 0` discharges and `current_a < 0` charges. Over a step the
//! current and the case temperature are held. Each line is the exact
//! solution for a held current and uses what the lines above it have just
//! produced. `exp`, `log` and `sqrt` are the only functions.
//!
//! ```text
//! i current, Tc case temperature in degC, Tk core temperature in degC at the start of the step
//! (the case temperature of the first step at a rested start), TK = Tk + 273.15,
//! x = 1/TK - 1/298.15,   xv = 1/(TK - T0) - 1/(298.15 - T0)   (the Vogel form; T0 the glass temperature)
//! Q capacity_ah, age = 1 - Q / reference capacity of the cell, s resistance scale
//!
//! 1 charge        rate = i / (3600 Q)                     z' = z - rate dt
//! 2 diffusion     tD = diffusion time * exp(E x)
//!                 each mode m:  g = weight[m] tD rate
//!                               w'[m] = g + (w[m] - g) exp(-dt / (tD time[m]))
//!                 surface = z' - (sum of w'[m] + weight at once * tD rate)
//!                 zc = surface held to 0 ... 1    used by lines 3 and 4 only
//! 3 resistance    R0 = s R_fixed + s R_cold exp(E xv)
//!   reactions     I0 = exchange current / s * exp(-E x)
//!                 I0t = near-full exchange current / s * exp(-E x + (1 - zc) / range)
//!                 b = 2 (gas constant / Faraday constant) TK
//!                 the surface reaction and the near-full step: Butler-Volmer with the double
//!                 layer beside it, the exact solution of C d(eta)/dt = i - 2 I sinh(eta / B),
//!                 as cell_2170_model3 takes it
//! 4 hysteresis    as cell_2170_model3, at TK
//! 5 electrolyte   ae = exp(E xv);  vs = s R_el ae i
//!                 v' = vs + (v - vs) exp(-dt / (time ae))
//!                 Vl = b / k_e;  u = v' / Vl;  refused unless -1 < u < 1
//!                 drop = Vl atanh(u) = (Vl / 2) log((1 + u) / (1 - u))
//! 6 voltage       U = rest_v_new_40c(surface) + age * rest_v_per_capacity_lost(surface)
//!                     + (Tk - 40) * rest_v_per_k(surface)
//!                 terminal = U + h' - i R0 - eta' - eta_top' - drop
//! 7 running mean  mean' = i + (mean - i) exp(-dt / 60)
//! 8 heat          q = the energy ledger's heat of the step (below) over dt
//! 9 core          tau = R_th C_th;  Ts = Tc + R_th q
//!                 Tk' = Ts + (Tk - Ts) exp(-dt / tau)
//! ```
//!
//! # The two ledgers
//!
//! An energy ledger at the terminals, as `cell_2170_model3` keeps it, with
//! the electrolyte's part the drop at the terminals:
//!
//! ```text
//! chemical = E(z) - E(z')            E: 3600 Q times the exact integral of the enthalpy voltage
//! work    += i * terminal * dt
//! heat    += i^2 R0 dt + i (eta' + eta_top') dt + i drop dt - i h' dt + (chemical - i U dt)
//! defect   = E(z') - E_anchor + work + heat;   residual = |defect| / (E(1) - E(0))
//! ```
//!
//! and a heat ledger of the core: the heat the energy ledger counts is held
//! in the core or has left it through the thermal resistance to the case.
//! The heat that left over a step is the exact integral of
//! `(T(t) - Tc) / R_th` along the step's own exponential, computed apart:
//!
//! ```text
//! stored  += C_th (Tk' - Tk)
//! to_case += dt (Ts - Tc) / R_th + (Tk - Ts) (tau / R_th) (1 - exp(-dt / tau))
//! thermal defect = heat - stored - to_case
//! ```
//!
//! A defect above the limit, in either ledger, is
//! [`StepReject::SpontaneousGeneration`] and one below its negative is
//! [`StepReject::FirstLawBreach`].
//!
//! # What it was compared with
//!
//! Every held-current discharge and drive cycle of the three cells, 234 runs,
//! computed from rest at one-second steps with the logged current and case
//! temperature and the capacity read from the full charge before each run;
//! millivolts RMS pooled over every compared second, C/3, C/2, 1C and drive
//! cycles:
//!
//! ```text
//!            fitted on all three cells        each cell left out in turn     cell_2170_model3
//!  40 degC    9.3  24.6  18.4  13.5             9.3  26.7  19.7  14.2           9.6  30.0  25.1  15.6
//!  25 degC   16.2  18.9  22.5  12.4            15.7  20.8  22.7  12.4          14.2  22.6  25.0  13.6
//!  10 degC   21.2  21.2  16.6  15.5            20.5  24.0  21.8  15.8          18.9  20.6  19.1  16.0
//!   0 degC   19.7  20.6  23.5  20.4            18.4  24.1  31.9  21.3          15.0  26.8  24.0  23.0
//! -10 degC   31.1  38.1  35.9  24.9            29.6  41.0  50.9  25.2          15.1  38.4  51.5  28.7
//! -20 degC   67.4  51.7  49.6  43.3            67.9  47.9  68.7  47.0          88.0  78.0 126.0  55.2
//! all runs   19.4                              20.2                            22.9
//! ```
//!
//! Left out, a cell's type numbers and curves come from the other two cells
//! and its own resistance scale from its pulse tests alone. The fit is the
//! study's `evidence/research/tesla_model3_2170_twin/out/fit_B.json`; the comparison
//! `code/twin.py` and `code/summary.py` there.
//!
//! # The step ceiling and floor
//!
//! Both are 1 s, as `cell_2170_model3`'s: every fit and every comparison was
//! made at one second on data logged once a second, and the rule for the
//! reaction's overpotential changing sign takes the whole step again from
//! zero, so the answer depends on the step where the current reverses.
//!
//! `core` only: no heap and no standard prelude. The state is `Copy`. An
//! accepted step writes time, terminal voltage, charge removed, surface
//! state of charge, the reaction's overpotential, the core temperature, the
//! electrolyte drop at the terminals, the running mean of the current and
//! the larger of the two ledgers' residuals into [`LastStateFrame64`]. The
//! 16-byte frame proof is not touched.

extern crate core;

use core::prelude::v1::*;

use crate::frame::LastStateFrame64;
use crate::math::{exp, log};
use crate::normal::{normal_or_zero, normal_or_zero_each};

::core::include!("cell_2170_twin_curves.rs");

// ---- fixed, not fitted ------------------------------------------------------

/// The temperature the Arrhenius and Vogel laws are anchored at, 25 degC.
pub const REFERENCE_TEMPERATURE_K: f64 = 298.15;
/// Kelvin at 0 degC.
pub const KELVIN_OFFSET: f64 = 273.15;
/// The case temperature of the slow discharge the rest-voltage curve was read from.
pub const CURVE_TEMPERATURE_C: f64 = 40.0;
/// The gas constant over the Faraday constant, in volts per kelvin.
pub const GAS_OVER_FARADAY_V_PER_K: f64 = 8.617333262281981e-05;
/// The laboratory's lower voltage limit.
pub const LOWER_VOLTAGE_LIMIT_V: f64 = 2.8;
/// The laboratory's upper voltage limit.
pub const UPPER_VOLTAGE_LIMIT_V: f64 = 4.242;
/// The near-full step holds back lithium leaving only; charging multiplies its exchange current by this.
pub const NEAR_FULL_CHARGING_FACTOR: f64 = 1000.0;
/// The reaction's overpotential over its slope voltage is read within this, either sign.
pub const OVERPOTENTIAL_RATIO_LIMIT: f64 = 60.0;
/// Time constant of the running mean of the current.
pub const CURRENT_MEAN_TIME_S: f64 = 60.0;
/// Seconds per hour.
pub const SECONDS_PER_HOUR: f64 = 3_600.0;
/// Modes of the diffusion lag: a sphere's step response as eleven decaying exponentials.
pub const DIFFUSION_MODES: usize = 11;
/// The time of each mode as a share of the diffusion time. Mathematics, not fitted.
pub const DIFFUSION_MODE_TIME_FRACTION: [f64; DIFFUSION_MODES] = [
    4.9527682827557200e-02,
    1.3757689674321444e-02,
    3.8215804650892898e-03,
    1.0615501291914692e-03,
    2.9487503588651929e-04,
    8.1909732190699799e-05,
    2.2752703386305498e-05,
    6.3201953850848606e-06,
    1.7556098291902389e-06,
    4.8766939699728860e-07,
    1.3546372138813572e-07,
];
/// The weight of each mode. Mathematics, not fitted.
pub const DIFFUSION_MODE_WEIGHT: [f64; DIFFUSION_MODES] = [
    0.03342768701014374,
    0.0156812997984314,
    0.00798028983180283,
    0.00487275926154771,
    0.00190045923189665,
    0.00161442759157927,
    0.00033927895075345,
    0.00053533700939262,
    0.00014378322075908,
    0.0,
    0.00015360359191147,
];
/// The part of the lag that acts at once. Mathematics, not fitted.
pub const DIFFUSION_AT_ONCE_WEIGHT: f64 = 1.77411684484381e-05;

/// Largest step the card stands behind.
pub const GATE_DT_S: f64 = 1.0;
/// Smallest step the card stands behind.
pub const FLOOR_DT_S: f64 = 1.0;
/// One spacing of 64-bit numbers between 1 and 2, per step of the running sum of the state of charge.
pub const STATE_OF_CHARGE_ALLOWANCE_PER_STEP: f64 = 2.220_446_049_250_313e-16;
/// Two more spacings: the start and the current, read from decimals.
pub const STATE_OF_CHARGE_ALLOWANCE_EXTRA_STEPS: f64 = 2.0;
/// Either ledger's defect, as a share of the cell's energy from empty to full.
pub const RESIDUAL_CONSERVATION_LIMIT: f64 = 1.0e-10;
/// The state of charge compared at a start: from empty.
pub const STATE_OF_CHARGE_MIN: f64 = 0.0;

/// The running mean of the current is above this, discharging: the rated current, 4.5 A, and a margin.
pub const FLAG_SUSTAINED_ABOVE_1C: u64 = 1 << 0;
/// The step lies outside the range the card was compared on: current, running mean, case
/// temperature, a jump or a span of it, capacity with case temperature, state of charge, or
/// a core more than the largest compared rise above its case.
pub const FLAG_OUTSIDE_COMPARED_RUNS: u64 = 1 << 1;

/// Why a step was refused. A refused step leaves the state and the frame as they were.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StepReject {
    /// Current, case temperature or `dt` was NaN or infinite.
    NonFiniteInput,
    /// The incoming state, or the trial step, was non-finite.
    NonFiniteState,
    /// `dt` was zero or negative.
    NonPositiveDt,
    /// A recorded constant or curve entry is not this cell type's.
    InvalidCell,
    /// The cell number of the state names none of the cells the card holds.
    CellNotRecognized,
    /// Capacity is zero or less: the charge is counted against it.
    CapacityNotPositive,
    /// Case temperature is at or below absolute zero, -273.15 degC.
    CaseTemperatureBelowAbsoluteZero,
    /// The core is at or below the electrolyte's glass temperature, where its Vogel law has no
    /// value: the electrolyte no longer conducts.
    CoreAtElectrolyteGlassTemperature,
    /// The least case temperature the state carries is above the most it carries.
    CaseTemperatureSpanInverted,
    /// State of charge, incoming or landing, is outside the ends of the curves, -0.05 and 1.05,
    /// by more than the rounding allowance of its running sum.
    StateOfChargeOutsideCard,
    /// Landing surface state of charge is outside the ends of the curves.
    SurfaceOutsideCurve,
    /// The salt's concentration difference reached its limit: the electrolyte at one electrode
    /// is depleted and the cell cannot carry the current.
    ElectrolyteDepleted,
    /// Landing terminal voltage is below 2.8 V.
    UndervoltageLimit,
    /// Landing terminal voltage is above 4.242 V.
    OvervoltageLimit,
    /// The trial step created energy from nothing, in the energy ledger or the core's heat ledger.
    SpontaneousGeneration,
    /// The trial step destroyed energy, in the energy ledger or the core's heat ledger.
    FirstLawBreach,
}

/// Held over the step. Positive current discharges the cell, negative charges it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CellLoad {
    pub current_a: f64,
    /// The temperature measured on the cell's case at the end of the step.
    pub case_temperature_c: f64,
}

/// What is locked for one cell of the type.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CellNumbers {
    /// The dataset's name for the cell.
    pub name: &'static str,
    /// The number in that name: 80 for cell `m80`.
    pub cell_number: f64,
    /// Capacity at the cell's first capacity test, a slow discharge at 40 degC to 2.8 V, in ampere-hours.
    pub reference_capacity_ah: f64,
    /// Charge delivered at that test over the charge put back by the recharge after it. A caller
    /// uses it to read the capacity from a recharge record. [`step`] does not use it.
    pub charge_return_ratio: f64,
    /// Multiplies the three resistances and divides the two exchange currents.
    pub resistance_scale: f64,
}

/// How many cells the card holds.
pub const CELL_COUNT: usize = CELL_TABLE.len();

const fn cell_rows() -> [CellNumbers; CELL_COUNT] {
    let mut rows = [CELL_TABLE[0]; CELL_COUNT];
    let mut index = 0;
    while index < CELL_COUNT {
        rows[index] = CELL_TABLE[index];
        index += 1;
    }
    rows
}

/// The locked row of each cell.
pub const CELLS: [CellNumbers; CELL_COUNT] = cell_rows();

/// The locked cell type. Every number is a recorded copy.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Cell2170Twin {
    pub fitted: [f64; FITTED_NUMBERS],
    pub reference_temperature_k: f64,
    pub kelvin_offset: f64,
    pub curve_temperature_c: f64,
    pub gas_over_faraday_v_per_k: f64,
    pub lower_voltage_limit_v: f64,
    pub upper_voltage_limit_v: f64,
    pub near_full_charging_factor: f64,
    pub overpotential_ratio_limit: f64,
    pub current_mean_time_s: f64,
    pub diffusion_mode_time_fraction: [f64; DIFFUSION_MODES],
    pub diffusion_mode_weight: [f64; DIFFUSION_MODES],
    pub diffusion_at_once_weight: f64,
    pub curve_first_state_of_charge: f64,
    pub curve_last_state_of_charge: f64,
    pub curve_spacing: f64,
    pub rest_v_new_40c: [f64; CURVE_POINTS],
    pub rest_v_per_capacity_lost: [f64; CURVE_POINTS],
    pub rest_v_per_k: [f64; CURVE_POINTS],
    pub cells: [CellNumbers; CELL_COUNT],
    /// The compared range, in the order of [`COMPARED_RANGE`].
    pub compared: [f64; COMPARED_RANGE_COUNT],
    pub block_case_temperature_least_c: [f64; BLOCK_COUNT],
    pub block_case_temperature_most_c: [f64; BLOCK_COUNT],
    pub block_capacity_least_ah: [f64; BLOCK_COUNT],
    pub block_capacity_most_ah: [f64; BLOCK_COUNT],
    pub gate_dt_s: f64,
    pub floor_dt_s: f64,
    pub residual_conservation_limit: f64,
    pub state_of_charge_allowance_per_step: f64,
    pub state_of_charge_allowance_extra_steps: f64,
}

/// The fitted numbers, by name, in the order of [`FITTED`].
#[derive(Clone, Copy, Debug, PartialEq)]
struct Fitted {
    resistance_fixed_ohm: f64,
    resistance_cold_ohm: f64,
    resistance_cold_activation_k: f64,
    exchange_current_a: f64,
    exchange_current_activation_k: f64,
    discharge_slope_factor: f64,
    double_layer_f: f64,
    near_full_exchange_current_a: f64,
    near_full_activation_k: f64,
    near_full_range: f64,
    diffusion_time_s: f64,
    diffusion_activation_k: f64,
    hysteresis_full_v: f64,
    hysteresis_full_range: f64,
    hysteresis_full_activation_k: f64,
    hysteresis_empty_v: f64,
    hysteresis_empty_range: f64,
    hysteresis_rate: f64,
    hysteresis_fade_s: f64,
    electrolyte_ohm: f64,
    electrolyte_time_s: f64,
    electrolyte_activation_k: f64,
    electrolyte_glass_temperature_k: f64,
    salt_limit_factor: f64,
    core_thermal_resistance_k_per_w: f64,
    core_heat_capacity_j_per_k: f64,
}

const fn fitted_by_name(f: &[f64; FITTED_NUMBERS]) -> Fitted {
    Fitted {
        resistance_fixed_ohm: f[0],
        resistance_cold_ohm: f[1],
        resistance_cold_activation_k: f[2],
        exchange_current_a: f[3],
        exchange_current_activation_k: f[4],
        discharge_slope_factor: f[5],
        double_layer_f: f[6],
        near_full_exchange_current_a: f[7],
        near_full_activation_k: f[8],
        near_full_range: f[9],
        diffusion_time_s: f[10],
        diffusion_activation_k: f[11],
        hysteresis_full_v: f[12],
        hysteresis_full_range: f[13],
        hysteresis_full_activation_k: f[14],
        hysteresis_empty_v: f[15],
        hysteresis_empty_range: f[16],
        hysteresis_rate: f[17],
        hysteresis_fade_s: f[18],
        electrolyte_ohm: f[19],
        electrolyte_time_s: f[20],
        electrolyte_activation_k: f[21],
        electrolyte_glass_temperature_k: f[22],
        salt_limit_factor: f[23],
        core_thermal_resistance_k_per_w: f[24],
        core_heat_capacity_j_per_k: f[25],
    }
}

/// The places of the compared range in [`COMPARED_RANGE`] and [`Cell2170Twin::compared`].
const CAPACITY_MIN: usize = 0;
const CAPACITY_MAX: usize = 1;
const CASE_MIN: usize = 2;
const CASE_MAX: usize = 3;
const DISCHARGE_MAX: usize = 4;
const CHARGE_MAX: usize = 5;
const SUSTAINED_DISCHARGE_MAX: usize = 6;
const SUSTAINED_CHARGE_MAX: usize = 7;
const SUSTAINED_FLAG: usize = 8;
const STATE_OF_CHARGE_MAX: usize = 9;
const CASE_CHANGE_MAX: usize = 10;
const CASE_SPAN_MAX: usize = 11;
const CORE_RISE_MAX: usize = 12;
/// How many numbers the compared range holds.
pub const COMPARED_RANGE_COUNT: usize = 13;

impl Cell2170Twin {
    /// The fitted cell type. The constructor takes no arguments.
    #[must_use]
    pub fn cell() -> Self {
        let cell = Self {
            fitted: FITTED,
            reference_temperature_k: REFERENCE_TEMPERATURE_K,
            kelvin_offset: KELVIN_OFFSET,
            curve_temperature_c: CURVE_TEMPERATURE_C,
            gas_over_faraday_v_per_k: GAS_OVER_FARADAY_V_PER_K,
            lower_voltage_limit_v: LOWER_VOLTAGE_LIMIT_V,
            upper_voltage_limit_v: UPPER_VOLTAGE_LIMIT_V,
            near_full_charging_factor: NEAR_FULL_CHARGING_FACTOR,
            overpotential_ratio_limit: OVERPOTENTIAL_RATIO_LIMIT,
            current_mean_time_s: CURRENT_MEAN_TIME_S,
            diffusion_mode_time_fraction: DIFFUSION_MODE_TIME_FRACTION,
            diffusion_mode_weight: DIFFUSION_MODE_WEIGHT,
            diffusion_at_once_weight: DIFFUSION_AT_ONCE_WEIGHT,
            curve_first_state_of_charge: CURVE_FIRST_STATE_OF_CHARGE,
            curve_last_state_of_charge: CURVE_LAST_STATE_OF_CHARGE,
            curve_spacing: CURVE_SPACING,
            rest_v_new_40c: REST_V_NEW_40C,
            rest_v_per_capacity_lost: REST_V_PER_CAPACITY_LOST,
            rest_v_per_k: REST_V_PER_K,
            cells: CELLS,
            compared: COMPARED_RANGE,
            block_case_temperature_least_c: BLOCK_CASE_TEMPERATURE_LEAST_C,
            block_case_temperature_most_c: BLOCK_CASE_TEMPERATURE_MOST_C,
            block_capacity_least_ah: BLOCK_CAPACITY_LEAST_AH,
            block_capacity_most_ah: BLOCK_CAPACITY_MOST_AH,
            gate_dt_s: GATE_DT_S,
            floor_dt_s: FLOOR_DT_S,
            residual_conservation_limit: RESIDUAL_CONSERVATION_LIMIT,
            state_of_charge_allowance_per_step: STATE_OF_CHARGE_ALLOWANCE_PER_STEP,
            state_of_charge_allowance_extra_steps: STATE_OF_CHARGE_ALLOWANCE_EXTRA_STEPS,
        };
        if cell.is_valid() {
            cell
        } else {
            ::core::panic!("2170 twin cell type failed is_valid");
        }
    }

    /// Every recorded copy equals its constant, bit for bit.
    #[must_use]
    pub fn is_valid(&self) -> bool {
        let mut index = 0;
        while index < CELL_COUNT {
            if self.cells[index].name != CELLS[index].name
                || self.cells[index].cell_number.to_bits() != CELLS[index].cell_number.to_bits()
                || self.cells[index].reference_capacity_ah.to_bits()
                    != CELLS[index].reference_capacity_ah.to_bits()
                || self.cells[index].charge_return_ratio.to_bits()
                    != CELLS[index].charge_return_ratio.to_bits()
                || self.cells[index].resistance_scale.to_bits()
                    != CELLS[index].resistance_scale.to_bits()
            {
                return false;
            }
            index += 1;
        }
        same_bits(&self.fitted, &FITTED)
            && same_bits(&self.diffusion_mode_time_fraction, &DIFFUSION_MODE_TIME_FRACTION)
            && same_bits(&self.diffusion_mode_weight, &DIFFUSION_MODE_WEIGHT)
            && same_bits(&self.rest_v_new_40c, &REST_V_NEW_40C)
            && same_bits(&self.rest_v_per_capacity_lost, &REST_V_PER_CAPACITY_LOST)
            && same_bits(&self.rest_v_per_k, &REST_V_PER_K)
            && same_bits(&self.compared, &COMPARED_RANGE)
            && same_bits(&self.block_case_temperature_least_c, &BLOCK_CASE_TEMPERATURE_LEAST_C)
            && same_bits(&self.block_case_temperature_most_c, &BLOCK_CASE_TEMPERATURE_MOST_C)
            && same_bits(&self.block_capacity_least_ah, &BLOCK_CAPACITY_LEAST_AH)
            && same_bits(&self.block_capacity_most_ah, &BLOCK_CAPACITY_MOST_AH)
            && same_bits(
                &[
                    self.reference_temperature_k,
                    self.kelvin_offset,
                    self.curve_temperature_c,
                    self.gas_over_faraday_v_per_k,
                    self.lower_voltage_limit_v,
                    self.upper_voltage_limit_v,
                    self.near_full_charging_factor,
                    self.overpotential_ratio_limit,
                    self.current_mean_time_s,
                    self.diffusion_at_once_weight,
                    self.curve_first_state_of_charge,
                    self.curve_last_state_of_charge,
                    self.curve_spacing,
                    self.gate_dt_s,
                    self.floor_dt_s,
                    self.residual_conservation_limit,
                    self.state_of_charge_allowance_per_step,
                    self.state_of_charge_allowance_extra_steps,
                ],
                &[
                    REFERENCE_TEMPERATURE_K,
                    KELVIN_OFFSET,
                    CURVE_TEMPERATURE_C,
                    GAS_OVER_FARADAY_V_PER_K,
                    LOWER_VOLTAGE_LIMIT_V,
                    UPPER_VOLTAGE_LIMIT_V,
                    NEAR_FULL_CHARGING_FACTOR,
                    OVERPOTENTIAL_RATIO_LIMIT,
                    CURRENT_MEAN_TIME_S,
                    DIFFUSION_AT_ONCE_WEIGHT,
                    CURVE_FIRST_STATE_OF_CHARGE,
                    CURVE_LAST_STATE_OF_CHARGE,
                    CURVE_SPACING,
                    GATE_DT_S,
                    FLOOR_DT_S,
                    RESIDUAL_CONSERVATION_LIMIT,
                    STATE_OF_CHARGE_ALLOWANCE_PER_STEP,
                    STATE_OF_CHARGE_ALLOWANCE_EXTRA_STEPS,
                ],
            )
    }

    /// The largest step the card stands behind: one second.
    #[must_use]
    pub fn gate_dt_s(&self) -> Option<f64> {
        if self.is_valid() { Some(self.gate_dt_s) } else { None }
    }

    /// The smallest step the card stands behind: the same one second.
    #[must_use]
    pub fn floor_dt_s(&self) -> Option<f64> {
        if self.is_valid() { Some(self.floor_dt_s) } else { None }
    }

    /// Whether one block of tests holds both this capacity and this case temperature.
    #[must_use]
    pub fn capacity_compared_at(&self, capacity_ah: f64, case_temperature_c: f64) -> bool {
        let mut block = 0;
        while block < BLOCK_COUNT {
            if case_temperature_c >= self.block_case_temperature_least_c[block]
                && case_temperature_c <= self.block_case_temperature_most_c[block]
                && capacity_ah >= self.block_capacity_least_ah[block]
                && capacity_ah <= self.block_capacity_most_ah[block]
            {
                return true;
            }
            block += 1;
        }
        false
    }

    /// Which row a cell number names, or `None` when it names none.
    #[must_use]
    pub fn cell_of(&self, cell_number: f64) -> Option<usize> {
        let mut index = 0;
        while index < CELL_COUNT {
            if cell_number == self.cells[index].cell_number {
                return Some(index);
            }
            index += 1;
        }
        None
    }

    /// Rest voltage at a state of charge between the ends of the curves, a temperature and a
    /// capacity of the cell in row `row`, in volts. `None` outside the curves.
    #[must_use]
    pub fn rest_v(&self, row: usize, capacity_ah: f64, state_of_charge: f64, temperature_c: f64) -> Option<f64> {
        if row >= CELL_COUNT
            || !(state_of_charge >= self.curve_first_state_of_charge
                && state_of_charge <= self.curve_last_state_of_charge)
        {
            return None;
        }
        let age = 1.0 - capacity_ah / self.cells[row].reference_capacity_ah;
        let rest_v = rest_v_at(self, age, state_of_charge, temperature_c);
        if rest_v.is_finite() { Some(rest_v) } else { None }
    }

    /// The compared range, by its place.
    #[must_use]
    pub fn compared(&self, place: usize) -> f64 {
        self.compared[place]
    }
}

/// Integrated cell. The ledger fields are part of the state; write them only through [`step`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CellState {
    pub t_s: f64,
    /// The number in the dataset's name for the cell: 80, 1000 or 448.
    pub cell_number: f64,
    /// Capacity from the last full charge, in ampere-hours. Never changed by [`step`].
    pub capacity_ah: f64,
    /// State of charge, mean over the particles. 1 after a full recharge.
    pub state_of_charge: f64,
    /// The eleven modes of the lag of the particle surface behind the mean.
    pub diffusion_lag: [f64; DIFFUSION_MODES],
    /// Overpotential of the surface reaction. Positive while discharging.
    pub reaction_overpotential_v: f64,
    /// Overpotential of the step that is slow near full charge.
    pub near_full_overpotential_v: f64,
    /// Negative after discharge, positive after charge.
    pub hysteresis_v: f64,
    /// The salt's concentration difference across the cell, in the volts of its small-signal
    /// limit. Positive while discharging.
    pub electrolyte_v: f64,
    /// The electrolyte's drop at the terminals in the last step: `electrolyte_v` through the
    /// transport limit. Zero before the first step.
    pub electrolyte_drop_v: f64,
    /// Running mean of the current, 60 s time constant.
    pub current_mean_a: f64,
    /// Temperature of the core at the end of the last step. Zero before the first step.
    pub core_temperature_c: f64,
    /// The core has a temperature: after the first step. A rested start takes the case
    /// temperature of its first step.
    pub core_known: bool,
    /// State of charge at the particle surface, as the last step left it.
    pub surface_state_of_charge: f64,
    /// `(1 - state_of_charge) * capacity_ah`.
    pub charge_removed_ah: f64,
    /// Current held over the last step. Zero before the first step.
    pub current_a: f64,
    /// Case temperature held over the last step. Zero before the first step.
    pub case_temperature_c: f64,
    /// Least case temperature of the steps since the rested start.
    pub case_temperature_least_c: f64,
    /// Most case temperature of the steps since the rested start.
    pub case_temperature_most_c: f64,
    /// The least and most case temperature hold the run's span: after the first step.
    pub case_span_known: bool,
    /// Terminal voltage at the end of the last step. Zero before the first step.
    pub terminal_v: f64,
    /// Rest voltage at the surface state of charge and the core temperature of the last step.
    pub rest_v: f64,
    /// Immediate resistance at the core temperature of the last step.
    pub resistance_at_once_ohm: f64,
    /// Heat of the last step per second, in watts: the energy ledger's heat of the step over its length.
    pub heat_rate_w: f64,
    pub chemical_energy_j: f64,
    pub energy_anchor_j: f64,
    /// Work the cell has done at its terminals. Positive while discharging.
    pub electrical_work_j: f64,
    /// Heat: ohmic, the two reactions, the electrolyte, hysteresis, and mixing with the reversible part.
    pub heat_j: f64,
    /// Heat held in the core above where the run began: its heat capacity times its rise.
    pub heat_stored_j: f64,
    /// Heat that has left the core through its thermal resistance to the case.
    pub heat_to_case_j: f64,
    pub energy_defect_j: f64,
    /// The heat ledger's defect: heat minus heat stored minus heat to the case.
    pub thermal_defect_j: f64,
    /// The larger of the two ledgers' defects over the cell's energy from empty to full.
    pub residual_conservation_error: f64,
    /// The running mean of the current is above the flag's current, discharging.
    pub sustained_above_1c: bool,
}

impl CellState {
    /// A rested cell: this cell, this state of charge, this capacity, every memory at zero, the
    /// core at the case of its first step, and both ledgers anchored there. `None` unless the
    /// cell number names a row and both numbers are finite and on the card.
    #[must_use]
    pub fn try_rested(cell: &Cell2170Twin, cell_number: f64, state_of_charge: f64, capacity_ah: f64) -> Option<Self> {
        if !cell.is_valid() || !cell_number.is_finite() || !state_of_charge.is_finite() || !capacity_ah.is_finite() {
            return None;
        }
        let row = cell.cell_of(cell_number)?;
        if !(capacity_ah > 0.0) {
            return None;
        }
        if !(state_of_charge >= cell.curve_first_state_of_charge && state_of_charge <= cell.curve_last_state_of_charge) {
            return None;
        }
        let age = 1.0 - capacity_ah / cell.cells[row].reference_capacity_ah;
        let chemical_energy_j = SECONDS_PER_HOUR * capacity_ah * enthalpy_integral_v(cell, age, state_of_charge);
        let state = Self {
            t_s: 0.0,
            cell_number: cell.cells[row].cell_number,
            capacity_ah,
            state_of_charge,
            diffusion_lag: [0.0; DIFFUSION_MODES],
            reaction_overpotential_v: 0.0,
            near_full_overpotential_v: 0.0,
            hysteresis_v: 0.0,
            electrolyte_v: 0.0,
            electrolyte_drop_v: 0.0,
            current_mean_a: 0.0,
            core_temperature_c: 0.0,
            core_known: false,
            surface_state_of_charge: state_of_charge,
            charge_removed_ah: (1.0 - state_of_charge) * capacity_ah,
            current_a: 0.0,
            case_temperature_c: 0.0,
            case_temperature_least_c: 0.0,
            case_temperature_most_c: 0.0,
            case_span_known: false,
            terminal_v: 0.0,
            rest_v: 0.0,
            resistance_at_once_ohm: 0.0,
            heat_rate_w: 0.0,
            chemical_energy_j,
            energy_anchor_j: chemical_energy_j,
            electrical_work_j: 0.0,
            heat_j: 0.0,
            heat_stored_j: 0.0,
            heat_to_case_j: 0.0,
            energy_defect_j: 0.0,
            thermal_defect_j: 0.0,
            residual_conservation_error: 0.0,
            sustained_above_1c: false,
        };
        if !state_is_finite(&state) {
            return None;
        }
        Some(state)
    }

    /// The same start carrying the least and the most case temperature of the run so far. `None`
    /// unless both are finite, above absolute zero, and the least is not above the most.
    #[must_use]
    pub fn with_case_span(self, cell: &Cell2170Twin, case_temperature_least_c: f64, case_temperature_most_c: f64) -> Option<Self> {
        if !case_temperature_least_c.is_finite()
            || !case_temperature_most_c.is_finite()
            || !(case_temperature_least_c + cell.kelvin_offset > 0.0)
            || case_temperature_least_c > case_temperature_most_c
        {
            return None;
        }
        Some(Self { case_temperature_least_c, case_temperature_most_c, case_span_known: true, ..self })
    }
}

/// Advance one step and write the frame.
///
/// A rejected step leaves `state` and `frame` as they were. A value of the landed state below
/// the normal range is written as zero with its sign ([`crate::normal`]).
#[must_use]
pub fn step(
    cell: &Cell2170Twin,
    state: &mut CellState,
    frame: &mut LastStateFrame64,
    load: CellLoad,
    dt_s: f64,
) -> Result<(), StepReject> {
    #[cfg(any(test, debug_assertions, feature = "check_exceedances"))]
    let before = *state;
    let stepped = step_landed(cell, state, frame, load, dt_s);
    #[cfg(any(test, debug_assertions, feature = "check_exceedances"))]
    if stepped.is_ok() {
        crate::exceedance::debug_check("cell_2170_twin", exceedances(cell, &before, state, load, dt_s), frame.flags);
    }
    stepped
}

/// The step itself; [`step`] is it with, in a build that checks
/// (`crate::exceedance::CHECKED`), the check that [`exceedances`] stands for
/// the flags it wrote.
fn step_landed(
    cell: &Cell2170Twin,
    state: &mut CellState,
    frame: &mut LastStateFrame64,
    load: CellLoad,
    dt_s: f64,
) -> Result<(), StepReject> {
    validate(cell, state, load, dt_s)?;
    let trial = integrate(cell, state, load, dt_s)?;
    commit(cell, state, frame, load, dt_s, trial)?;
    normal_or_zero_state(state);
    Ok(())
}

fn normal_or_zero_state(state: &mut CellState) {
    state.t_s = normal_or_zero(state.t_s);
    state.cell_number = normal_or_zero(state.cell_number);
    state.capacity_ah = normal_or_zero(state.capacity_ah);
    state.state_of_charge = normal_or_zero(state.state_of_charge);
    normal_or_zero_each(&mut state.diffusion_lag);
    state.reaction_overpotential_v = normal_or_zero(state.reaction_overpotential_v);
    state.near_full_overpotential_v = normal_or_zero(state.near_full_overpotential_v);
    state.hysteresis_v = normal_or_zero(state.hysteresis_v);
    state.electrolyte_v = normal_or_zero(state.electrolyte_v);
    state.electrolyte_drop_v = normal_or_zero(state.electrolyte_drop_v);
    state.current_mean_a = normal_or_zero(state.current_mean_a);
    state.core_temperature_c = normal_or_zero(state.core_temperature_c);
    state.surface_state_of_charge = normal_or_zero(state.surface_state_of_charge);
    state.charge_removed_ah = normal_or_zero(state.charge_removed_ah);
    state.current_a = normal_or_zero(state.current_a);
    state.case_temperature_c = normal_or_zero(state.case_temperature_c);
    state.case_temperature_least_c = normal_or_zero(state.case_temperature_least_c);
    state.case_temperature_most_c = normal_or_zero(state.case_temperature_most_c);
    state.terminal_v = normal_or_zero(state.terminal_v);
    state.rest_v = normal_or_zero(state.rest_v);
    state.resistance_at_once_ohm = normal_or_zero(state.resistance_at_once_ohm);
    state.heat_rate_w = normal_or_zero(state.heat_rate_w);
    state.chemical_energy_j = normal_or_zero(state.chemical_energy_j);
    state.energy_anchor_j = normal_or_zero(state.energy_anchor_j);
    state.electrical_work_j = normal_or_zero(state.electrical_work_j);
    state.heat_j = normal_or_zero(state.heat_j);
    state.heat_stored_j = normal_or_zero(state.heat_stored_j);
    state.heat_to_case_j = normal_or_zero(state.heat_to_case_j);
    state.energy_defect_j = normal_or_zero(state.energy_defect_j);
    state.thermal_defect_j = normal_or_zero(state.thermal_defect_j);
    state.residual_conservation_error = normal_or_zero(state.residual_conservation_error);
}

/// Every input and the incoming state, before any curve is read.
fn validate(cell: &Cell2170Twin, state: &CellState, load: CellLoad, dt_s: f64) -> Result<(), StepReject> {
    if !dt_s.is_finite() || !load.current_a.is_finite() || !load.case_temperature_c.is_finite() {
        return Err(StepReject::NonFiniteInput);
    }
    if !state_is_finite(state) {
        return Err(StepReject::NonFiniteState);
    }
    if dt_s <= 0.0 {
        return Err(StepReject::NonPositiveDt);
    }
    if !cell.is_valid() {
        return Err(StepReject::InvalidCell);
    }
    if cell.cell_of(state.cell_number).is_none() {
        return Err(StepReject::CellNotRecognized);
    }
    if !(state.capacity_ah > 0.0) {
        return Err(StepReject::CapacityNotPositive);
    }
    if !(load.case_temperature_c + cell.kelvin_offset > 0.0) {
        return Err(StepReject::CaseTemperatureBelowAbsoluteZero);
    }
    if state.case_span_known && !(state.case_temperature_least_c <= state.case_temperature_most_c) {
        return Err(StepReject::CaseTemperatureSpanInverted);
    }
    Ok(())
}

/// The flags of a step from `state` under `load` that lands at the running mean `current_mean_a`,
/// the state of charge `state_of_charge` and the core temperature `core_c`.
fn compared_flags(
    cell: &Cell2170Twin,
    state: &CellState,
    load: CellLoad,
    current_mean_a: f64,
    state_of_charge: f64,
    core_c: f64,
    allowance: f64,
) -> u64 {
    let r = &cell.compared;
    let case_c = load.case_temperature_c;
    let changed = state.t_s > 0.0 && !(abs_f64(case_c - state.case_temperature_c) <= r[CASE_CHANGE_MAX]);
    let spread = if state.case_span_known {
        let least = if case_c < state.case_temperature_least_c { case_c } else { state.case_temperature_least_c };
        let most = if case_c > state.case_temperature_most_c { case_c } else { state.case_temperature_most_c };
        !(most - least <= r[CASE_SPAN_MAX])
    } else {
        false
    };
    let outside = !(state.capacity_ah >= r[CAPACITY_MIN] && state.capacity_ah <= r[CAPACITY_MAX])
        || case_c < r[CASE_MIN]
        || case_c > r[CASE_MAX]
        || load.current_a > r[DISCHARGE_MAX]
        || load.current_a < -r[CHARGE_MAX]
        || !cell.capacity_compared_at(state.capacity_ah, case_c)
        || changed
        || spread
        || !(current_mean_a <= r[SUSTAINED_DISCHARGE_MAX] && current_mean_a >= -r[SUSTAINED_CHARGE_MAX])
        || !(abs_f64(core_c - case_c) <= r[CORE_RISE_MAX])
        || !(state_of_charge >= STATE_OF_CHARGE_MIN - allowance && state_of_charge <= r[STATE_OF_CHARGE_MAX] + allowance);
    let mut flags = 0;
    if current_mean_a > r[SUSTAINED_FLAG] {
        flags |= FLAG_SUSTAINED_ABOVE_1C;
    }
    if outside {
        flags |= FLAG_OUTSIDE_COMPARED_RUNS;
    }
    flags
}

/// What a trial step hands to [`commit`].
#[derive(Clone, Copy, Debug, PartialEq)]
struct Trial {
    state_of_charge: f64,
    diffusion_lag: [f64; DIFFUSION_MODES],
    surface_state_of_charge: f64,
    reaction_overpotential_v: f64,
    near_full_overpotential_v: f64,
    hysteresis_v: f64,
    electrolyte_v: f64,
    electrolyte_drop_v: f64,
    current_mean_a: f64,
    core_start_c: f64,
    resistance_at_once_ohm: f64,
    rest_v: f64,
    terminal_v: f64,
    chemical_step_j: f64,
}

fn state_of_charge_allowance(cell: &Cell2170Twin, t_s: f64, dt_s: f64) -> f64 {
    cell.state_of_charge_allowance_per_step * (t_s / dt_s + 1.0 + cell.state_of_charge_allowance_extra_steps)
}

fn state_of_charge_on_card(cell: &Cell2170Twin, state_of_charge: f64, allowance: f64) -> bool {
    state_of_charge >= cell.curve_first_state_of_charge - allowance
        && state_of_charge <= cell.curve_last_state_of_charge + allowance
}

fn surface_on_curve(cell: &Cell2170Twin, surface_state_of_charge: f64) -> bool {
    surface_state_of_charge >= cell.curve_first_state_of_charge
        && surface_state_of_charge <= cell.curve_last_state_of_charge
}

fn integrate(cell: &Cell2170Twin, state: &CellState, load: CellLoad, dt_s: f64) -> Result<Trial, StepReject> {
    let row = match cell.cell_of(state.cell_number) {
        Some(row) => row,
        None => return Err(StepReject::CellNotRecognized),
    };
    let f = fitted_by_name(&cell.fitted);
    let numbers = cell.cells[row];
    let scale = numbers.resistance_scale;

    let current_a = load.current_a;
    // the core at the start of the step: a rested start takes the case of its first step
    let core_start_c = if state.core_known { state.core_temperature_c } else { load.case_temperature_c };
    let kelvin = core_start_c + cell.kelvin_offset;
    if !(kelvin > f.electrolyte_glass_temperature_k) {
        return Err(StepReject::CoreAtElectrolyteGlassTemperature);
    }
    let x = 1.0 / kelvin - 1.0 / cell.reference_temperature_k;
    let xv = 1.0 / (kelvin - f.electrolyte_glass_temperature_k)
        - 1.0 / (cell.reference_temperature_k - f.electrolyte_glass_temperature_k);
    let capacity_ah = state.capacity_ah;
    let age = 1.0 - capacity_ah / numbers.reference_capacity_ah;

    // 7 the running mean of the current
    let current_mean_a = current_a + (state.current_mean_a - current_a) * exp(-dt_s / cell.current_mean_time_s);
    if !current_mean_a.is_finite() {
        return Err(StepReject::NonFiniteState);
    }

    // 1 charge
    let allowance = state_of_charge_allowance(cell, state.t_s, dt_s);
    if !allowance.is_finite() {
        return Err(StepReject::NonFiniteState);
    }
    if !state_of_charge_on_card(cell, state.state_of_charge, allowance) {
        return Err(StepReject::StateOfChargeOutsideCard);
    }
    let rate = current_a / (SECONDS_PER_HOUR * capacity_ah);
    let state_of_charge = state.state_of_charge - rate * dt_s;
    if !state_of_charge.is_finite() {
        return Err(StepReject::NonFiniteState);
    }
    if !state_of_charge_on_card(cell, state_of_charge, allowance) {
        return Err(StepReject::StateOfChargeOutsideCard);
    }

    // 2 diffusion in the particles: eleven modes, each exact for a held current
    let diffusion_time_s = f.diffusion_time_s * exp(f.diffusion_activation_k * x);
    let mut diffusion_lag = [0.0; DIFFUSION_MODES];
    let mut lag_sum = 0.0;
    let mut mode = 0;
    while mode < DIFFUSION_MODES {
        let settled = cell.diffusion_mode_weight[mode] * diffusion_time_s * rate;
        diffusion_lag[mode] = settled
            + (state.diffusion_lag[mode] - settled)
                * exp(-dt_s / (diffusion_time_s * cell.diffusion_mode_time_fraction[mode]));
        lag_sum += diffusion_lag[mode];
        mode += 1;
    }
    let surface_state_of_charge = state_of_charge - (lag_sum + cell.diffusion_at_once_weight * diffusion_time_s * rate);
    if !surface_state_of_charge.is_finite() {
        return Err(StepReject::NonFiniteState);
    }
    if !surface_on_curve(cell, surface_state_of_charge) {
        return Err(StepReject::SurfaceOutsideCurve);
    }
    let surface_held = if surface_state_of_charge < 0.0 {
        0.0
    } else if surface_state_of_charge > 1.0 {
        1.0
    } else {
        surface_state_of_charge
    };

    // 3 immediate resistance and the two surface reactions, at the core
    let resistance_at_once_ohm = scale * f.resistance_fixed_ohm + scale * f.resistance_cold_ohm * exp(f.resistance_cold_activation_k * xv);
    let exchange_a = f.exchange_current_a / scale * exp(-f.exchange_current_activation_k * x);
    let near_full_exchange_a = f.near_full_exchange_current_a / scale
        * exp(-f.near_full_activation_k * x + (1.0 - surface_held) / f.near_full_range);
    let slope_v = 2.0 * cell.gas_over_faraday_v_per_k * kelvin;
    let slope_factor = f.discharge_slope_factor;
    let layer = f.double_layer_f;
    let incoming = state.reaction_overpotential_v;
    let mut reaction_overpotential_v;
    if incoming > 0.0 || (incoming == 0.0 && current_a >= 0.0) {
        reaction_overpotential_v = react(cell, layer, incoming, current_a, exchange_a, slope_v / slope_factor, dt_s);
        if reaction_overpotential_v < 0.0 {
            reaction_overpotential_v = react(cell, layer, 0.0, current_a, exchange_a * slope_factor, slope_v, dt_s);
        }
    } else {
        reaction_overpotential_v = react(cell, layer, incoming, current_a, exchange_a * slope_factor, slope_v, dt_s);
        if reaction_overpotential_v > 0.0 {
            reaction_overpotential_v = react(cell, layer, 0.0, current_a, exchange_a, slope_v / slope_factor, dt_s);
        }
    }
    let mut near_full_overpotential_v =
        react(cell, layer, state.near_full_overpotential_v, current_a, near_full_exchange_a, slope_v, dt_s);
    if !(near_full_overpotential_v >= 0.0) {
        near_full_overpotential_v = react(
            cell,
            layer,
            0.0,
            current_a,
            near_full_exchange_a * cell.near_full_charging_factor,
            slope_v,
            dt_s,
        );
    }

    // 4 hysteresis
    let half_gap_v = f.hysteresis_full_v
        * exp(-(1.0 - surface_held) / f.hysteresis_full_range)
        * exp(f.hysteresis_full_activation_k * x)
        + f.hysteresis_empty_v * exp(-surface_held / f.hysteresis_empty_range);
    let follow_per_s = f.hysteresis_rate * abs_f64(rate);
    let target_v = if current_a < 0.0 { half_gap_v } else { -half_gap_v };
    let fade_per_s = follow_per_s + 1.0 / f.hysteresis_fade_s;
    let settled_v = follow_per_s * target_v / fade_per_s;
    let hysteresis_v = settled_v + (state.hysteresis_v - settled_v) * exp(-fade_per_s * dt_s);

    // 5 the salt in the electrolyte, and its transport limit
    let cold = exp(f.electrolyte_activation_k * xv);
    let electrolyte_settled_v = scale * f.electrolyte_ohm * cold * current_a;
    let electrolyte_v = electrolyte_settled_v
        + (state.electrolyte_v - electrolyte_settled_v) * exp(-dt_s / (f.electrolyte_time_s * cold));
    let limit_v = slope_v / f.salt_limit_factor;
    let share = electrolyte_v / limit_v;
    if !share.is_finite() {
        return Err(StepReject::NonFiniteState);
    }
    if !(share > -1.0 && share < 1.0) {
        return Err(StepReject::ElectrolyteDepleted);
    }
    let electrolyte_drop_v = 0.5 * limit_v * log((1.0 + share) / (1.0 - share));

    // 6 terminal voltage
    let rest_v = rest_v_at(cell, age, surface_state_of_charge, core_start_c);
    let terminal_v = rest_v + hysteresis_v
        - current_a * resistance_at_once_ohm
        - reaction_overpotential_v
        - near_full_overpotential_v
        - electrolyte_drop_v;

    // the chemical energy the step released: the exact integral of the enthalpy voltage
    let chemical_step_j = SECONDS_PER_HOUR
        * capacity_ah
        * (enthalpy_integral_v(cell, age, state.state_of_charge) - enthalpy_integral_v(cell, age, state_of_charge));

    let trial = Trial {
        state_of_charge,
        diffusion_lag,
        surface_state_of_charge,
        reaction_overpotential_v,
        near_full_overpotential_v,
        hysteresis_v,
        electrolyte_v,
        electrolyte_drop_v,
        current_mean_a,
        core_start_c,
        resistance_at_once_ohm,
        rest_v,
        terminal_v,
        chemical_step_j,
    };
    if !trial_is_finite(&trial) {
        return Err(StepReject::NonFiniteState);
    }
    if terminal_v < cell.lower_voltage_limit_v {
        return Err(StepReject::UndervoltageLimit);
    }
    if terminal_v > cell.upper_voltage_limit_v {
        return Err(StepReject::OvervoltageLimit);
    }
    Ok(trial)
}

fn commit(
    cell: &Cell2170Twin,
    state: &mut CellState,
    frame: &mut LastStateFrame64,
    load: CellLoad,
    dt_s: f64,
    trial: Trial,
) -> Result<(), StepReject> {
    let row = match cell.cell_of(state.cell_number) {
        Some(row) => row,
        None => return Err(StepReject::CellNotRecognized),
    };
    if !trial_is_finite(&trial) {
        return Err(StepReject::NonFiniteState);
    }
    let allowance = state_of_charge_allowance(cell, state.t_s, dt_s);
    if !(allowance.is_finite() && state_of_charge_on_card(cell, trial.state_of_charge, allowance)) {
        return Err(StepReject::StateOfChargeOutsideCard);
    }
    if !surface_on_curve(cell, trial.surface_state_of_charge) {
        return Err(StepReject::SurfaceOutsideCurve);
    }
    if trial.terminal_v < cell.lower_voltage_limit_v {
        return Err(StepReject::UndervoltageLimit);
    }
    if trial.terminal_v > cell.upper_voltage_limit_v {
        return Err(StepReject::OvervoltageLimit);
    }
    let f = fitted_by_name(&cell.fitted);
    let current_a = load.current_a;
    let capacity_ah = state.capacity_ah;
    let age = 1.0 - capacity_ah / cell.cells[row].reference_capacity_ah;

    // the energy ledger at the terminals
    let work_j = current_a * trial.terminal_v * dt_s;
    let ohmic_j = current_a * current_a * trial.resistance_at_once_ohm * dt_s;
    let reactions_j = current_a * (trial.reaction_overpotential_v + trial.near_full_overpotential_v) * dt_s;
    let electrolyte_j = current_a * trial.electrolyte_drop_v * dt_s;
    let hysteresis_j = -current_a * trial.hysteresis_v * dt_s;
    let mixing_j = trial.chemical_step_j - current_a * trial.rest_v * dt_s;
    let step_heat_j = ohmic_j + reactions_j + electrolyte_j + hysteresis_j + mixing_j;
    let electrical_work_j = state.electrical_work_j + work_j;
    let heat_j = state.heat_j + step_heat_j;

    let chemical_energy_j = SECONDS_PER_HOUR * capacity_ah * enthalpy_integral_v(cell, age, trial.state_of_charge);
    let cell_energy_j = SECONDS_PER_HOUR
        * capacity_ah
        * (enthalpy_integral_v(cell, age, 1.0) - enthalpy_integral_v(cell, age, 0.0));
    if !(cell_energy_j > 0.0 && cell_energy_j.is_finite()) {
        return Err(StepReject::InvalidCell);
    }
    // Subtracted from zero, so that books that balance exactly read +0.
    let defect = 0.0 - (state.energy_anchor_j - chemical_energy_j - electrical_work_j - heat_j);
    if !defect.is_finite() {
        return Err(StepReject::NonFiniteState);
    }
    let gate = cell.residual_conservation_limit * cell_energy_j;
    if defect > gate {
        return Err(StepReject::SpontaneousGeneration);
    }
    if defect < -gate {
        return Err(StepReject::FirstLawBreach);
    }

    // 8 and 9: the heat of the step warms the core behind its thermal resistance to the case
    let heat_rate_w = step_heat_j / dt_s;
    let resistance = f.core_thermal_resistance_k_per_w;
    let capacity_j_per_k = f.core_heat_capacity_j_per_k;
    let tau_s = resistance * capacity_j_per_k;
    let case_c = load.case_temperature_c;
    let settled_c = case_c + resistance * heat_rate_w;
    let fade = exp(-dt_s / tau_s);
    let core_c = settled_c + (trial.core_start_c - settled_c) * fade;
    let stored_j = capacity_j_per_k * (core_c - trial.core_start_c);
    let to_case_j = dt_s * (settled_c - case_c) / resistance + (trial.core_start_c - settled_c) * (tau_s / resistance) * (1.0 - fade);
    let heat_stored_j = state.heat_stored_j + stored_j;
    let heat_to_case_j = state.heat_to_case_j + to_case_j;
    let thermal_defect = 0.0 - (heat_stored_j + heat_to_case_j - heat_j);
    if !(thermal_defect.is_finite() && core_c.is_finite() && heat_rate_w.is_finite()) {
        return Err(StepReject::NonFiniteState);
    }
    if thermal_defect > gate {
        return Err(StepReject::SpontaneousGeneration);
    }
    if thermal_defect < -gate {
        return Err(StepReject::FirstLawBreach);
    }
    let energy_residual = abs_f64(defect) / cell_energy_j;
    let thermal_residual = abs_f64(thermal_defect) / cell_energy_j;

    let landed = CellState {
        t_s: state.t_s + dt_s,
        cell_number: state.cell_number,
        capacity_ah,
        state_of_charge: trial.state_of_charge,
        diffusion_lag: trial.diffusion_lag,
        reaction_overpotential_v: trial.reaction_overpotential_v,
        near_full_overpotential_v: trial.near_full_overpotential_v,
        hysteresis_v: trial.hysteresis_v,
        electrolyte_v: trial.electrolyte_v,
        electrolyte_drop_v: trial.electrolyte_drop_v,
        current_mean_a: trial.current_mean_a,
        core_temperature_c: core_c,
        core_known: true,
        surface_state_of_charge: trial.surface_state_of_charge,
        charge_removed_ah: (1.0 - trial.state_of_charge) * capacity_ah,
        current_a,
        case_temperature_c: case_c,
        case_temperature_least_c: if state.case_span_known && state.case_temperature_least_c < case_c {
            state.case_temperature_least_c
        } else {
            case_c
        },
        case_temperature_most_c: if state.case_span_known && state.case_temperature_most_c > case_c {
            state.case_temperature_most_c
        } else {
            case_c
        },
        case_span_known: true,
        terminal_v: trial.terminal_v,
        rest_v: trial.rest_v,
        resistance_at_once_ohm: trial.resistance_at_once_ohm,
        heat_rate_w,
        chemical_energy_j,
        energy_anchor_j: state.energy_anchor_j,
        electrical_work_j,
        heat_j,
        heat_stored_j,
        heat_to_case_j,
        energy_defect_j: defect,
        thermal_defect_j: thermal_defect,
        residual_conservation_error: if thermal_residual > energy_residual { thermal_residual } else { energy_residual },
        sustained_above_1c: trial.current_mean_a > cell.compared[SUSTAINED_FLAG],
    };
    if !state_is_finite(&landed) {
        return Err(StepReject::NonFiniteState);
    }
    let flags = compared_flags(cell, state, load, trial.current_mean_a, trial.state_of_charge, core_c, allowance);
    *state = landed;
    publish(state, frame, flags);
    Ok(())
}

fn publish(state: &CellState, frame: &mut LastStateFrame64, flags: u64) {
    frame.t = state.t_s;
    frame.pos[0] = state.terminal_v as f32;
    frame.pos[1] = state.charge_removed_ah as f32;
    frame.pos[2] = state.surface_state_of_charge as f32;
    frame.vel[0] = state.reaction_overpotential_v as f32;
    frame.vel[1] = state.core_temperature_c as f32;
    frame.vel[2] = state.electrolyte_drop_v as f32;
    frame.force_torque = state.current_mean_a as f32;
    frame.residual = state.residual_conservation_error as f32;
    frame.flags = flags;
}

/// One surface reaction over a step: the exact solution of `C d(eta)/dt = i - 2 I sinh(eta / B)`
/// for a held current `i`, exchange current `I` and slope voltage `B`.
fn react(cell: &Cell2170Twin, layer_f: f64, overpotential_v: f64, current_a: f64, exchange_a: f64, slope_v: f64, dt_s: f64) -> f64 {
    let root = (current_a * current_a + 4.0 * exchange_a * exchange_a).sqrt();
    let up = (current_a + root) / (2.0 * exchange_a);
    let um = -1.0 / up;
    let mut ratio = overpotential_v / slope_v;
    if ratio < -cell.overpotential_ratio_limit {
        ratio = -cell.overpotential_ratio_limit;
    }
    if ratio > cell.overpotential_ratio_limit {
        ratio = cell.overpotential_ratio_limit;
    }
    let u = exp(ratio);
    let rho = (u - up) / (u - um) * exp(-root * dt_s / (slope_v * layer_f));
    slope_v * log((up - rho * um) / (1.0 - rho))
}

fn curve_place(cell: &Cell2170Twin, state_of_charge: f64) -> (usize, f64) {
    let along = (state_of_charge - cell.curve_first_state_of_charge) / cell.curve_spacing;
    let mut index = if along > 0.0 { along as usize } else { 0 };
    if index > CURVE_POINTS - 2 {
        index = CURVE_POINTS - 2;
    }
    (index, along - index as f64)
}

fn curve_at(values: &[f64; CURVE_POINTS], place: (usize, f64)) -> f64 {
    values[place.0] + (values[place.0 + 1] - values[place.0]) * place.1
}

/// Rest voltage at a state of charge, a temperature and an age.
fn rest_v_at(cell: &Cell2170Twin, age: f64, state_of_charge: f64, temperature_c: f64) -> f64 {
    let place = curve_place(cell, state_of_charge);
    curve_at(&cell.rest_v_new_40c, place)
        + age * curve_at(&cell.rest_v_per_capacity_lost, place)
        + (temperature_c - cell.curve_temperature_c) * curve_at(&cell.rest_v_per_k, place)
}

fn enthalpy_v_at_point(cell: &Cell2170Twin, age: f64, index: usize) -> f64 {
    cell.rest_v_new_40c[index] + age * cell.rest_v_per_capacity_lost[index]
        - (cell.curve_temperature_c + cell.kelvin_offset) * cell.rest_v_per_k[index]
}

/// Integral of the enthalpy voltage from the first point of the curves to a state of charge, in
/// volts. Exact for straight lines.
fn enthalpy_integral_v(cell: &Cell2170Twin, age: f64, state_of_charge: f64) -> f64 {
    let (index, share) = curve_place(cell, state_of_charge);
    let mut sum = 0.0;
    let mut point = 0;
    while point < index {
        sum += 0.5 * (enthalpy_v_at_point(cell, age, point) + enthalpy_v_at_point(cell, age, point + 1)) * cell.curve_spacing;
        point += 1;
    }
    let low = enthalpy_v_at_point(cell, age, index);
    let here = low + (enthalpy_v_at_point(cell, age, index + 1) - low) * share;
    sum + 0.5 * (low + here) * share * cell.curve_spacing
}

fn same_bits<const N: usize>(recorded: &[f64; N], locked: &[f64; N]) -> bool {
    let mut index = 0;
    while index < N {
        if recorded[index].to_bits() != locked[index].to_bits() {
            return false;
        }
        index += 1;
    }
    true
}

fn abs_f64(value: f64) -> f64 {
    if value < 0.0 { -value } else { value }
}

fn all_finite<const N: usize>(values: &[f64; N]) -> bool {
    let mut index = 0;
    while index < N {
        if !values[index].is_finite() {
            return false;
        }
        index += 1;
    }
    true
}

fn trial_is_finite(trial: &Trial) -> bool {
    all_finite(&trial.diffusion_lag)
        && all_finite(&[
            trial.state_of_charge,
            trial.surface_state_of_charge,
            trial.reaction_overpotential_v,
            trial.near_full_overpotential_v,
            trial.hysteresis_v,
            trial.electrolyte_v,
            trial.electrolyte_drop_v,
            trial.current_mean_a,
            trial.core_start_c,
            trial.resistance_at_once_ohm,
            trial.rest_v,
            trial.terminal_v,
            trial.chemical_step_j,
        ])
}

fn state_is_finite(state: &CellState) -> bool {
    all_finite(&state.diffusion_lag)
        && all_finite(&[
            state.t_s,
            state.cell_number,
            state.capacity_ah,
            state.state_of_charge,
            state.reaction_overpotential_v,
            state.near_full_overpotential_v,
            state.hysteresis_v,
            state.electrolyte_v,
            state.electrolyte_drop_v,
            state.current_mean_a,
            state.core_temperature_c,
            state.surface_state_of_charge,
            state.charge_removed_ah,
            state.current_a,
            state.case_temperature_c,
            state.case_temperature_least_c,
            state.case_temperature_most_c,
            state.terminal_v,
            state.rest_v,
            state.resistance_at_once_ohm,
            state.heat_rate_w,
            state.chemical_energy_j,
            state.energy_anchor_j,
            state.electrical_work_j,
            state.heat_j,
            state.heat_stored_j,
            state.heat_to_case_j,
            state.energy_defect_j,
            state.thermal_defect_j,
            state.residual_conservation_error,
        ])
}


/// Every limit a flag of this card can mark: what [`exceedances`] reports
/// beside a flag's bit (`crate::exceedance`).
pub const EXCEEDANCE_KINDS: &[crate::exceedance::ExceedanceKind] = &[
    SUSTAINED_KIND,
    CAPACITY_LOW,
    CAPACITY_HIGH,
    CASE_LOW,
    CASE_HIGH,
    DISCHARGE_HIGH,
    CHARGE_HIGH,
    BLOCK_CASE_LOW,
    BLOCK_CASE_HIGH,
    BLOCK_CAPACITY_LOW,
    BLOCK_CAPACITY_HIGH,
    CASE_CHANGE,
    CASE_SPAN,
    MEAN_DISCHARGE_HIGH,
    MEAN_CHARGE_HIGH,
    CORE_RISE,
    SOC_LOW,
    SOC_HIGH,
];

use crate::exceedance::{Exceedance, ExceedanceKind, Side};

const SUSTAINED_KIND: ExceedanceKind = ExceedanceKind::fixed(
    "FLAG_SUSTAINED_ABOVE_1C",
    FLAG_SUSTAINED_ABOVE_1C,
    "current_mean_a",
    "A",
    Side::Above,
    COMPARED_RANGE[SUSTAINED_FLAG],
    "a running mean discharge current of 4.6 A: the rated current, 4.5 A, and a margin",
);
const fn outside(quantity: &'static str, unit: &'static str, side: Side, limit: f64, words: &'static str) -> ExceedanceKind {
    ExceedanceKind::fixed("FLAG_OUTSIDE_COMPARED_RUNS", FLAG_OUTSIDE_COMPARED_RUNS, quantity, unit, side, limit, words)
}
const fn outside_varying(quantity: &'static str, unit: &'static str, side: Side, words: &'static str) -> ExceedanceKind {
    ExceedanceKind::varying("FLAG_OUTSIDE_COMPARED_RUNS", FLAG_OUTSIDE_COMPARED_RUNS, quantity, unit, side, words)
}
const CAPACITY_WORDS: &str = "the capacities of the compared cells, 4.16 to 4.7 Ah";
const CAPACITY_LOW: ExceedanceKind = outside("capacity_ah", "Ah", Side::Below, COMPARED_RANGE[CAPACITY_MIN], CAPACITY_WORDS);
const CAPACITY_HIGH: ExceedanceKind = outside("capacity_ah", "Ah", Side::Above, COMPARED_RANGE[CAPACITY_MAX], CAPACITY_WORDS);
const CASE_WORDS: &str = "the case temperatures of the compared runs, -23.3 to 50.2 degC";
const CASE_LOW: ExceedanceKind = outside("case_temperature_c", "degC", Side::Below, COMPARED_RANGE[CASE_MIN], CASE_WORDS);
const CASE_HIGH: ExceedanceKind = outside("case_temperature_c", "degC", Side::Above, COMPARED_RANGE[CASE_MAX], CASE_WORDS);
const DISCHARGE_HIGH: ExceedanceKind = outside(
    "current_a",
    "A",
    Side::Above,
    COMPARED_RANGE[DISCHARGE_MAX],
    "the largest discharging current compared, 16.152 A (discharge is positive)",
);
const CHARGE_HIGH: ExceedanceKind = outside(
    "current_a",
    "A",
    Side::Below,
    -COMPARED_RANGE[CHARGE_MAX],
    "the largest charging current compared, 13.984 A, as the negative current -13.984 A",
);
const BLOCK_CASE_WORDS: &str = "the nearer edge of the case temperatures of the compared blocks of tests that hold the step's capacity: no block held this capacity and case temperature together";
const BLOCK_CASE_LOW: ExceedanceKind = outside_varying("case_temperature_c", "degC", Side::Below, BLOCK_CASE_WORDS);
const BLOCK_CASE_HIGH: ExceedanceKind = outside_varying("case_temperature_c", "degC", Side::Above, BLOCK_CASE_WORDS);
const BLOCK_CAPACITY_WORDS: &str = "the nearer edge of the capacities of the compared blocks of tests: no block held this capacity at any case temperature";
const BLOCK_CAPACITY_LOW: ExceedanceKind = outside_varying("capacity_ah", "Ah", Side::Below, BLOCK_CAPACITY_WORDS);
const BLOCK_CAPACITY_HIGH: ExceedanceKind = outside_varying("capacity_ah", "Ah", Side::Above, BLOCK_CAPACITY_WORDS);
const CASE_CHANGE: ExceedanceKind = outside(
    "case_temperature_change_k",
    "K",
    Side::Above,
    COMPARED_RANGE[CASE_CHANGE_MAX],
    "the largest change of case temperature from one step to the next in the compared runs, 13.0 K",
);
const CASE_SPAN: ExceedanceKind = outside(
    "case_temperature_span_k",
    "K",
    Side::Above,
    COMPARED_RANGE[CASE_SPAN_MAX],
    "the widest span of case temperature within one compared run, 25.4 K",
);
const MEAN_DISCHARGE_HIGH: ExceedanceKind = outside(
    "current_mean_a",
    "A",
    Side::Above,
    COMPARED_RANGE[SUSTAINED_DISCHARGE_MAX],
    "the largest running mean discharging current compared, 11.949 A",
);
const MEAN_CHARGE_HIGH: ExceedanceKind = outside(
    "current_mean_a",
    "A",
    Side::Below,
    -COMPARED_RANGE[SUSTAINED_CHARGE_MAX],
    "the largest running mean charging current compared, 4.971 A, as the negative current -4.971 A",
);
const CORE_RISE: ExceedanceKind = outside(
    "core_rise_k",
    "K",
    Side::Above,
    COMPARED_RANGE[CORE_RISE_MAX],
    "the largest rise of the core above its case compared, 7.9 K, either way",
);
const SOC_WORDS: &str = "the states of charge of the compared runs, 0 to 1.04; set past them by more than the running sum's rounding allowance";
const SOC_LOW: ExceedanceKind = outside("state_of_charge", "1", Side::Below, STATE_OF_CHARGE_MIN, SOC_WORDS);
const SOC_HIGH: ExceedanceKind = outside("state_of_charge", "1", Side::Above, COMPARED_RANGE[STATE_OF_CHARGE_MAX], SOC_WORDS);

/// The capacity and case temperature that no block held together, read
/// against the nearer edge: the case temperature against the blocks that
/// hold the capacity, or, where none does, the capacity against every block.
fn block_record(cell: &Cell2170Twin, capacity_ah: f64, case_c: f64) -> Exceedance {
    fn keep(nearest: &mut Option<(f64, Exceedance)>, distance: f64, record: Exceedance) {
        if nearest.is_none_or(|(held, _)| distance < held) {
            *nearest = Some((distance, record));
        }
    }
    let mut nearest: Option<(f64, Exceedance)> = None;
    let mut block = 0;
    while block < BLOCK_COUNT {
        let least_ah = cell.block_capacity_least_ah[block];
        let most_ah = cell.block_capacity_most_ah[block];
        if capacity_ah >= least_ah && capacity_ah <= most_ah {
            let least_c = cell.block_case_temperature_least_c[block];
            let most_c = cell.block_case_temperature_most_c[block];
            if case_c < least_c {
                keep(&mut nearest, least_c - case_c, Exceedance::at(BLOCK_CASE_LOW, least_c, case_c));
            } else if case_c > most_c {
                keep(&mut nearest, case_c - most_c, Exceedance::at(BLOCK_CASE_HIGH, most_c, case_c));
            }
        }
        block += 1;
    }
    if let Some((_, record)) = nearest {
        return record;
    }
    let mut block = 0;
    while block < BLOCK_COUNT {
        let least_ah = cell.block_capacity_least_ah[block];
        let most_ah = cell.block_capacity_most_ah[block];
        if capacity_ah < least_ah {
            keep(&mut nearest, least_ah - capacity_ah, Exceedance::at(BLOCK_CAPACITY_LOW, least_ah, capacity_ah));
        } else if capacity_ah > most_ah {
            keep(&mut nearest, capacity_ah - most_ah, Exceedance::at(BLOCK_CAPACITY_HIGH, most_ah, capacity_ah));
        }
        block += 1;
    }
    match nearest {
        Some((_, record)) => record,
        None => Exceedance::at(BLOCK_CAPACITY_LOW, capacity_ah, capacity_ah),
    }
}

/// The limits the step from `before` to `after` under `load` over `dt_s`
/// is past: one record for each limit a flag set on its frame marks.
/// The conditions are those of `compared_flags`, from the state before the
/// step, its load, and the running mean, state of charge and core
/// temperature it landed at.
pub fn exceedances(
    cell: &Cell2170Twin,
    before: &CellState,
    after: &CellState,
    load: CellLoad,
    dt_s: f64,
) -> crate::exceedance::Exceedances {
    let mut records = crate::exceedance::Exceedances::new();
    let r = &cell.compared;
    let state = before;
    let current_mean_a = after.current_mean_a;
    let state_of_charge = after.state_of_charge;
    let allowance = state_of_charge_allowance(cell, state.t_s, dt_s);
    records.push_if(current_mean_a > r[SUSTAINED_FLAG], Exceedance::at(SUSTAINED_KIND, r[SUSTAINED_FLAG], current_mean_a));
    let capacity = state.capacity_ah;
    records.push_if(!(capacity >= r[CAPACITY_MIN]), Exceedance::at(CAPACITY_LOW, r[CAPACITY_MIN], capacity));
    records.push_if(!(capacity <= r[CAPACITY_MAX]), Exceedance::at(CAPACITY_HIGH, r[CAPACITY_MAX], capacity));
    let case_c = load.case_temperature_c;
    records.push_if(case_c < r[CASE_MIN], Exceedance::at(CASE_LOW, r[CASE_MIN], case_c));
    records.push_if(case_c > r[CASE_MAX], Exceedance::at(CASE_HIGH, r[CASE_MAX], case_c));
    records.push_if(load.current_a > r[DISCHARGE_MAX], Exceedance::at(DISCHARGE_HIGH, r[DISCHARGE_MAX], load.current_a));
    records.push_if(load.current_a < -r[CHARGE_MAX], Exceedance::at(CHARGE_HIGH, -r[CHARGE_MAX], load.current_a));
    if !cell.capacity_compared_at(capacity, case_c) {
        records.push(block_record(cell, capacity, case_c));
    }
    let change = abs_f64(case_c - state.case_temperature_c);
    records.push_if(
        state.t_s > 0.0 && !(change <= r[CASE_CHANGE_MAX]),
        Exceedance::at(CASE_CHANGE, r[CASE_CHANGE_MAX], change),
    );
    if state.case_span_known {
        let least = if case_c < state.case_temperature_least_c { case_c } else { state.case_temperature_least_c };
        let most = if case_c > state.case_temperature_most_c { case_c } else { state.case_temperature_most_c };
        let span = most - least;
        records.push_if(!(span <= r[CASE_SPAN_MAX]), Exceedance::at(CASE_SPAN, r[CASE_SPAN_MAX], span));
    }
    records.push_if(
        !(current_mean_a <= r[SUSTAINED_DISCHARGE_MAX]),
        Exceedance::at(MEAN_DISCHARGE_HIGH, r[SUSTAINED_DISCHARGE_MAX], current_mean_a),
    );
    records.push_if(
        !(current_mean_a >= -r[SUSTAINED_CHARGE_MAX]),
        Exceedance::at(MEAN_CHARGE_HIGH, -r[SUSTAINED_CHARGE_MAX], current_mean_a),
    );
    let rise = abs_f64(after.core_temperature_c - case_c);
    records.push_if(!(rise <= r[CORE_RISE_MAX]), Exceedance::at(CORE_RISE, r[CORE_RISE_MAX], rise));
    records.push_if(
        !(state_of_charge >= STATE_OF_CHARGE_MIN - allowance),
        Exceedance::at(SOC_LOW, STATE_OF_CHARGE_MIN, state_of_charge),
    );
    records.push_if(
        !(state_of_charge <= r[STATE_OF_CHARGE_MAX] + allowance),
        Exceedance::at(SOC_HIGH, r[STATE_OF_CHARGE_MAX], state_of_charge),
    );
    records
}

#[cfg(test)]
#[path = "cell_2170_twin_tests.rs"]
mod tests;
