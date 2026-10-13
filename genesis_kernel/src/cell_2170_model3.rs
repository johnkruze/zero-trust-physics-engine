#![no_implicit_prelude]

//! One Tesla Model 3 2170 lithium-ion cell, as a physical model with memory,
//! under a terminal current and a measured case temperature, from -10 to
//! 40 degC.
//!
//! Measured by Kollmeyer, Khanum, Naguib and Emadi, "Tesla Model 3 2170
//! Li-ion Cell Dataset and Battery SOC Estimation Blind Modeling Tool",
//! Borealis, doi: 10.5683/SP3/ZVTR4B, version 3.0, CC BY 4.0.
//!
//! The card returns the voltage at the cell's terminals, given the current
//! and the temperature measured on the cell's case. It has no model of
//! temperature and does not predict it. It holds one measured rest-voltage
//! curve with its two corrections and 22 fitted numbers, each with a
//! physical meaning. The card `cell_2170_m80` describes the same cell with
//! tables read off the same tests; this card describes it with equations.
//!
//! # The numbers, by group
//!
//! ```text
//! immediate resistance     a part that does not change with temperature and a part that
//!                          grows in the cold                                      3 numbers
//! surface reaction         an exchange current that rises with temperature, a slope
//!                          factor while discharging, and the double-layer capacitance
//!                          beside it; its overpotential eases as the current grows  4 numbers
//! near-full step           a second reaction that is slow only near full charge and holds
//!                          back lithium leaving, not lithium arriving              3 numbers
//! diffusion                the time lithium takes to cross a particle, and how that time
//!                          lengthens in the cold; the surface of a particle lags its mean
//!                          as eleven decaying modes of a sphere plus a part that acts at
//!                          once, and those twelve weights are a sphere's, not fitted  2 numbers
//! hysteresis               a half-gap near full that grows in the cold, a half-gap near
//!                          empty, the ranges over which each fades, how much charge must
//!                          pass before the gap follows the current's direction, and how
//!                          fast it fades at rest                                   7 numbers
//! electrolyte              a slow voltage drop under sustained current: its size per
//!                          ampere, the time it takes to form and fade, and how both grow
//!                          in the cold                                             3 numbers
//! rest voltage             three measured curves on 211 points of state of charge: the
//!                          rest voltage at 40 degC when the cell was new, its change per
//!                          unit of capacity lost, and its change per kelvin
//! ```
//!
//! All 22 come from one fit to every test of the first cell, `m80`, from
//! -10 to 40 degC (`round3/fit_r3_final.json` of the study, constants file
//! SHA-256 b8c4a3bcaa6a15a821da3e56b9af4483dfad7b5332a54a05379087b8f94f4d2b).
//! The curves are in `cell_2170_model3_curves.rs`. The study worked with
//! 2,101 points and found that keeping every tenth changes no verdict on any
//! of its 78 runs: the curve moves by at most 2.96 mV, a run's voltage misfit
//! by at most 0.08 mV and a capacity error by at most 0.03 percent.
//!
//! # Which cell
//!
//! The equations, the curves and the 22 numbers describe the cell type. Three
//! numbers belong to one cell, and each cell's are a locked row of
//! [`CELL_TABLE`], picked by `cell_number` in the starting state: 80 for the
//! first cell, `m80`, 1000 for the second, `m1000`, and 448 for the third,
//! `m448-N`. A row holds the
//! cell's capacity at its first capacity test (a slow discharge at 40 degC to
//! 2.8 V), measured; its charge-return ratio, measured at that test and the
//! recharge after it; and one fitted resistance scale, which multiplies the
//! two immediate resistances and the electrolyte's and divides the two
//! exchange currents. The scale is 1 for `m80`, 0.9119920763410333 for
//! `m1000` and 0.8243415837166522 for `m448-N`, each fitted on that cell's
//! own capacity test, discharge at a third of the capacity per hour, pulse
//! test and slow loop at each temperature from -10 to 40 degC. Another cell
//! is one more row. Any other `cell_number` is refused as
//! [`StepReject::CellNotRecognized`].
//!
//! # What a caller supplies
//!
//! The current, the measured case temperature and the step, at every step;
//! and the starting state: `cell_number`, the state of charge and the
//! capacity from the last full charge.
//!
//! The cell ages: over these tests the capacity of `m80` went from 4.687 to
//! 4.174 Ah. So the capacity on the day is part of the starting state and
//! not a constant. A caller reads it from a recharge record:
//!
//! ```text
//! capacity_ah = ratio * (ampere-hours put in by the full recharge) / (1 - state of charge before it)
//! ratio       = the cell's charge-return ratio: charge delivered / charge put back, measured once,
//!               at the cell's first capacity test and the recharge after it;
//!               0.9977038043683101 for m80, 0.9972171037869162 for m1000
//!               and 0.9976972129208563 for m448-N
//! state of charge before the recharge:
//!               the value at which the voltage after a rest of an hour or more equals
//!               rest_v_new_40c + age * rest_v_per_capacity_lost + (T - 40) * rest_v_per_k
//! ```
//!
//! A full recharge is the dataset's: constant current, then constant voltage
//! at 4.2 V. The reading is good only when the state of charge before the
//! recharge is below 0.08, where the curve is steep. After a shallower
//! discharge the caller keeps the last good reading, or the straight line in
//! time between the readings either side. Read this way after the capacity
//! test of each block of tests, at 40, 25, 10, 0 and -10 degC in turn, the
//! capacity of `m80` was 4.6768, 4.5006, 4.2661, 4.2529 and 4.2106 Ah, that
//! of `m1000` 4.6747, 4.5121, 4.2899, 4.2874 and 4.2423 Ah, and that of
//! `m448-N` 4.6696, 4.4980, 4.2680, 4.2653 and 4.2179 Ah. The card holds
//! none of these: the capacity is the caller's.
//!
//! The state of charge is 1 after a full recharge, and 1 plus the top-up
//! divided by the capacity when the tester topped the cell up afterwards.
//!
//! The memories start at zero, which is a cell that has rested. Hysteresis
//! fades in 5 minutes; the electrolyte drop in 3 minutes at 25 degC and 11
//! at -10 degC; the slowest diffusion mode in 80 seconds at 25 degC and 16
//! minutes at -10 degC. An hour's rest is enough anywhere on the card. A
//! cell that has not rested is not in this state: step the card through the
//! load that came before.
//!
//! # The step
//!
//! `current_a > 0` discharges and `current_a < 0` charges. Over a step the
//! current and the case temperature are held. Each line is the exact
//! solution for a held current and uses what the lines above it have just
//! produced. `exp`, `log` and `sqrt` are the only functions.
//!
//! ```text
//! i current, T case temperature in degC, TK = T + 273.15, x = 1/TK - 1/298.15,
//! Q capacity_ah, age = 1 - Q / reference capacity of the cell, s resistance scale
//!
//! 1 charge        rate = i / (3600 Q)                     z' = z - rate dt
//! 2 diffusion     tD = diffusion time * exp(E x)
//!                 each mode m:  g = weight[m] tD rate
//!                               w'[m] = g + (w[m] - g) exp(-dt / (tD time[m]))
//!                 surface = z' - (sum of w'[m] + weight at once * tD rate)
//!                 zc = surface held to 0 ... 1    used by lines 3 and 4 only
//! 3 resistance    R0 = s R_fixed + s R_cold exp(E x)
//!   reactions     I0 = exchange current / s * exp(-E x)
//!                 I0t = near-full exchange current / s * exp(-E x + (1 - zc) / range)
//!                 b = 2 (gas constant / Faraday constant) TK
//!   react(eta, i, I, B):  root = sqrt(i^2 + 4 I^2);  up = (i + root) / (2 I);  um = -1 / up
//!                         u = exp(eta / B held to -60 ... 60)
//!                         rho = (u - up) / (u - um) * exp(-root dt / (B C))
//!                         B log((up - rho um) / (1 - rho))
//!                 the exact solution of  C d(eta)/dt = i - 2 I sinh(eta / B)  for i and I held
//!   surface reaction, slope factor a while eta has the discharging sign, 1 while charging:
//!                 if eta > 0, or eta = 0 and i >= 0:  e = react(eta, i, I0, b / a)
//!                                                     if e < 0:  e = react(0, i, I0 a, b)
//!                 otherwise:                          e = react(eta, i, I0 a, b)
//!                                                     if e > 0:  e = react(0, i, I0, b / a)
//!   near-full:    e = react(eta_top, i, I0t, b);  if e < 0:  e = react(0, i, 1000 I0t, b)
//! 4 hysteresis    H = H_full exp(-(1 - zc) / range) exp(E x) + H_empty exp(-zc / range)
//!                 a = rate constant * |rate|;  target = +H if i < 0, otherwise -H
//!                 k = a + 1 / fade time;  hs = a target / k
//!                 h' = hs + (h - hs) exp(-k dt)
//! 5 electrolyte   ae = exp(E x);  vs = s R_el ae i
//!                 v' = vs + (v - vs) exp(-dt / (time ae))
//! 6 voltage       U = rest_v_new_40c(surface) + age * rest_v_per_capacity_lost(surface)
//!                     + (T - 40) * rest_v_per_k(surface)       straight lines between points
//!                 terminal = U + h' - i R0 - eta' - eta_top' - v'
//! 7 running mean  mean' = i + (mean - i) exp(-dt / 60)
//! ```
//!
//! When the surface reaction's overpotential would pass through zero inside
//! a step, the step is taken again from zero with the other law, for the
//! whole step. Every fit and score was made this way at one second, so it is
//! part of the card, and the answer depends on the step from the second the
//! current reverses and for the seconds after it. The card therefore steps
//! by one second only.
//!
//! # The ledger
//!
//! An energy ledger at the terminals. The rest voltage is a straight line in
//! temperature, so the enthalpy voltage does not depend on temperature.
//!
//! ```text
//! U_H(z) = rest_v_new_40c(z) + age * rest_v_per_capacity_lost(z) - 313.15 * rest_v_per_k(z)
//! E(z)   = 3600 Q * integral of U_H from the first point to z     exact: a trapezoid per interval
//! chemical = E(z) - E(z')
//! work    += i * terminal * dt
//! heat    += i^2 R0 dt + i (eta' + eta_top') dt + i v' dt - i h' dt + (chemical - i U dt)
//! defect   = E(z') - E_anchor + work + heat
//! residual = |defect| / (E(1) - E(0))
//! ```
//!
//! A defect above the limit is [`StepReject::SpontaneousGeneration`] and one
//! below its negative is [`StepReject::FirstLawBreach`]. The energy held in
//! the double layer, in the concentration gradients and in the hysteresis is
//! not kept as a store: it is counted as heat when it is put in. The ledger
//! therefore tests the charge step, the chemical energy and the voltage
//! against its own parts. It does not test how a memory is stepped; the
//! reference states of the tests do that.
//!
//! # The declared range
//!
//! Each limit is set from everything the model has been scored on with the
//! chamber set from -10 to 40 degC: the 80 records of `m80` that are not
//! recharges, and the 36 opened records of `m1000` that are not recharges.
//! Past a limit the card refuses by name. Nothing is pulled back to a limit.
//!
//! ```text
//!                               m80        m1000      limit     how the limit is set
//! discharge current, A          13.506     13.506     13.524    the tester's programmed 13.5 A pulse + 0.024
//! charging current, A            9.737     13.960     13.984    the largest scored, to a milliampere + 0.024
//! running mean, discharging, A   8.698      4.501      8.722    the largest scored + 0.024
//! running mean, charging, A      2.968      0.831      2.993    the largest scored + 0.024
//! case temperature, least, degC -12.586    -12.095    -12.9     the least scored - 0.3, to a tenth
//! case temperature, most, degC   45.535     44.923     45.9     the most scored + 0.3, to a tenth
//! capacity, least, Ah            4.174      4.217      4.16     the least scored - 0.01, to a hundredth
//! capacity, most, Ah             4.687      4.689      4.70     the most scored + 0.01, to a hundredth
//! state of charge at the start   1.012      1.000      1.02
//! ```
//!
//! The allowances are for what a measured number does around the value it
//! was meant to have. 0.024 A is the accuracy the dataset states for the
//! tester's current on its 60 A range: the pulse programmed at 13.5 A, three
//! times the nominal 4.5 Ah, reads from 13.500 to 13.506 A at its top in the
//! ten pulse tests of the two cells. 0.3 K is the accuracy the dataset states
//! for the chamber. 0.01 Ah is a fifth of a percent of the capacity: the
//! capacity is itself read from a recharge and a rested voltage. A logged
//! sample that sits on a limit by the tester's own noise does not end a run.
//!
//! The largest charging currents are the second cell's: its drive cycles
//! charge it harder than the first cell's, with peaks from 9.8 to 13.96 A
//! against 0.2 to 9.74 A. The two sustained-current limits are the first
//! cell's alone. The second cell's opened records hold no hill cycle. The
//! third cell's fitting files lie inside every limit: its pulse reads up to
//! 13.504 A, its case from -11.9 to 41.8 degC, and its capacities run from
//! 4.218 to 4.670 Ah.
//!
//! The state of charge is held to 0 ... 1.02 and the surface state of charge
//! to the ends of the curves, -0.05 ... 1.05. The state of charge is a
//! running sum, z' = z - rate dt in 64-bit arithmetic, and each subtraction
//! rounds by up to 1.11e-16, half the spacing of 64-bit numbers between 1
//! and 2. The comparison of a step's incoming and landing state of charge
//! with 0 and 1.02 therefore carries a named allowance,
//! [`STATE_OF_CHARGE_ALLOWANCE_PER_STEP`]: one full spacing, 2.22e-16, for
//! each step of the run so far including this one, and two more for the
//! start and the current read from decimals. It is 6.7e-16 on the first step
//! and 2.2e-10 after 1,000,000 steps. A run or a single step whose end in
//! decimal is exactly 0 or 1.02 lands its last step, and the state of charge
//! is not pulled back, so it can read up to the allowance past the end. A
//! rested start is held to 0 ... 1.02 itself.
//!
//! # Capacity with case temperature
//!
//! Each block of tests was measured at one age of the cell: the blocks ran
//! in time order, 40, 25, 10, 0 and -10 degC, while the capacity fell. A new
//! cell was never measured in the cold and an aged cell never warm, and the
//! resistances and exchange currents carry no age term. So the capacity is
//! held to the band that was scored at the case temperatures each block
//! spanned. A capacity and a case temperature are on the card when one
//! block holds both:
//!
//! ```text
//! block    capacity scored, Ah     case scored, degC     capacity band, Ah   case band, degC
//! -10 C    4.1742 ... 4.2423       -12.449 ... 12.223    4.16 ... 4.26       -12.9 ... 12.6
//!   0 C    4.2174 ... 4.2874         0.379 ... 20.068    4.20 ... 4.30         0.0 ... 20.4
//!  10 C    4.1842 ... 4.2899        10.183 ... 26.479    4.17 ... 4.30         9.8 ... 26.8
//!  25 C    4.3669 ... 4.5130        23.633 ... 31.071    4.35 ... 4.53        23.3 ... 31.4
//!  40 C    4.5725 ... 4.6871        28.466 ... 45.535    4.56 ... 4.70        28.1 ... 45.9
//! ```
//!
//! The capacities are those of every scored run of the three cells from -10
//! to 40 degC (the 65 of `m80` and the 15 of `m1000` scored by the study,
//! and the 93 of `m1000` and `m448-N` scored in its final check) and the
//! block capacities of the three cells' fitting files; the band allows
//! 0.01 Ah beyond them, to a hundredth. The case temperatures are those
//! the sensor read in the same scored runs, with the chamber's 0.3 K beyond
//! them, to a tenth; the -10 degC block takes the card's least, -12.9 degC,
//! read in a fitting file of `m80`. Outside every band the step is
//! [`StepReject::CapacityOutsideCardAtTemperature`].
//!
//! # Case temperature through a run
//!
//! The memories carry the history of the load from step to step, and the
//! case temperature moved slowly in every scored run. The largest change
//! between two consecutive one-second samples of a scored run was 5.023 K
//! (`m80`, 25 degC, `REORDERED1`), and the largest span of the case
//! temperature inside one scored run 18.811 K (`m1000`, -10 degC, the
//! discharge at half the rated current). A step whose case temperature
//! differs from the last step's by more than 5.4 K is
//! [`StepReject::CaseTemperatureChangeAboveCard`], and one that would take
//! the span since the start past 19.2 K is
//! [`StepReject::CaseTemperatureSpanAboveCard`]: each the measured value
//! plus the chamber's 0.3 K, to a tenth above. The span depends on every
//! case temperature since the rested start, so a run that continues from a
//! returned state carries the least and the most case temperature of the run
//! so far ([`CellState::with_case_span`]); a fresh start states both equal to
//! its first case temperature. The first step of a run has no last step to
//! differ from.
//!
//! -20 degC is outside the card. The measurements there are irregular: the
//! discharges at a held current contradict each other, and no mechanism could
//! be separated from them. Scored there for the record, the model met its
//! marks on none of the first cell's 13 runs.
//!
//! The sustained current is held because the hill cycle is the only test of
//! sustained current above the rated current (about 4.5 A), the model's
//! worst misses are in it, and nothing measured supports a longer or harder
//! climb. The running mean of the current has a 60 s time constant; past its
//! limit the step is [`StepReject::SustainedCurrentOutsideCard`]. Above
//! 4.6 A discharging the step lands and carries [`FLAG_SUSTAINED_ABOVE_1C`]:
//! a discharge at the rated current sits at 4.50 A.
//!
//! The card accepts charging current. It was fitted to the charging inside
//! the first cell's drive cycles, to 9.74 A for seconds and a running mean to
//! 2.97 A, and to the slow charge at a twentieth of the capacity per hour;
//! and it was scored on the second cell's drive cycles, which charge to
//! 13.96 A for seconds. It was not scored on any charging run.
//!
//! # What it was compared with, and what that shows
//!
//! The marks: on a discharge at a held current, voltage error at most 42 mV
//! root mean square and delivered capacity within 3 percent; on a drive
//! cycle, 42 mV root mean square, still running at the last sample and within
//! 126 mV there. On the first cell the model meets them on 62 of 65 runs in
//! range, and on 14 of the second cell's 15 opened runs in range.
//!
//! The model was developed on every test of the first cell. Its scores there
//! are not a blind test. On the second and the third cell one number was
//! fitted, each on that cell's own fitting files, and the rest carried.
//!
//! The card was then run once, on 2026-10-09, on runs of those two cells
//! that no model had seen, from -10 to 40 degC. On the third cell 51 of 60
//! runs were inside the marks and all five conditions written beforehand
//! were met. On the second cell 31 of 45 were inside the marks, below the 36
//! to 41 expected, and three of the five conditions were not met: its
//! discharge at half the rated current delivered 3.3 to 4.0 percent too much
//! charge at four temperatures, and three drive cycles at 40 degC ended 128
//! to 145 mV high. No ordinary drive cycle missed the voltage mark on either
//! cell. All ten hill cycles were refused as outside the declared range,
//! nine for current and one for case temperature, so the hill cycle is
//! untested on new cells. The record is the study's `final_check/` folder:
//! `RESULT.md`, `DECLARATION.md` and `final/TABLES.md`.
//!
//! Known misses:
//!
//! - Three hill cycles of the first cell, at 40, 25 and -10 degC. At 40 and
//!   25 degC the card stops at its 2.8 V limit seconds to minutes before the
//!   real cell, which is riding the same limit there: the card is 15 mV and
//!   2 mV under the cell at that moment.
//! - Steep climbs at low charge in the cold. In the hill cycle at 0 and
//!   -10 degC, at the end of its steepest climbs (7 to 10 A for four minutes,
//!   below 45 percent charge), the card reads 130 to 180 mV high. Three
//!   physical causes were tested and none stands.
//! - Charging in the cold: in the descents of the hill cycle at 0 and
//!   -10 degC the card reads 40 to 70 mV high while the cell is charged at 1
//!   to 2 A.
//! - Capacity up to 2 percent long under held current at 10 and 0 degC.
//! - The cold edge is predicted poorly when it is left out of the fit:
//!   fitted without the -10 degC tests the model met its marks on 3 of the 13
//!   runs at -10 degC. The temperature laws do not reach to a colder
//!   temperature. Each of 25, 10 and 0 degC is predicted as well without its
//!   own tests as with them.
//!
//! # What is left out
//!
//! Temperature: the card is given the case temperature and has no heat
//! balance. Ageing: the capacity is the caller's number and the card never
//! changes it; the age it uses is one minus the capacity over the cell's
//! capacity when first measured. A temperature difference between the case
//! and the inside of the cell. Anything inside one second: the card steps by
//! one second only. The energy stored in the double layer, the concentration gradients and the
//! hysteresis, which the ledger counts as heat. A pack: this is one cell.
//!
//! # The step ceiling and floor
//!
//! [`Cell2170Model3::gate_dt_s`] and [`Cell2170Model3::floor_dt_s`] are both
//! 1 s: every fit and every score was made at one second on data logged once
//! a second, and no measurement on disk can check a shorter step. For a held
//! 4.5 A discharge at 25 degC, the one load checked, a step of 0.1 s gave the
//! one-second answer to within 0.3 mV. When the current reverses it does not: the rule for the reaction's
//! overpotential changing sign takes the whole step again from zero, so the
//! answer depends on the step. From 4.5 A to -4.5 A at 0 degC the 0.1 s
//! answer differs from the one-second answer by 49 mV in the second of the
//! reversal, 5 mV in the next and 0.3 mV in the one after; on the first
//! 6,000 seconds of a drive cycle of `m448-N` at -10 degC, by up to 32 mV.
//! A shorter step can come back only after the sign change is split at the
//! zero crossing and the card is fitted and scored again. [`step`] applies
//! neither.
//!
//! `core` only: no heap and no standard prelude. The state is `Copy`. An
//! accepted step writes time, terminal voltage, charge removed, surface
//! state of charge, the reaction's overpotential, hysteresis, the
//! electrolyte drop, the running mean of the current and the conservation
//! residual into [`LastStateFrame64`]. The 16-byte frame proof is not
//! touched.
//!
//! The study's symbols for the state: `z` is `state_of_charge`, `z_surface`
//! `surface_state_of_charge`, `w` `diffusion_lag`, `eta` and `eta_top`
//! `reaction_overpotential_v` and `near_full_overpotential_v`, `h`
//! `hysteresis_v`, `v_el` `electrolyte_v`.

extern crate core;

use core::prelude::v1::*;

use crate::frame::LastStateFrame64;
use crate::math::{exp, log};

::core::include!("cell_2170_model3_curves.rs");

// ---- fitted on every test of cell m80 from -10 to 40 degC ----------------

/// Immediate resistance, the part that does not change with temperature.
pub const RESISTANCE_FIXED_OHM: f64 = 0.009808192734973114;
/// Immediate resistance, the part that grows in the cold, at 25 degC.
pub const RESISTANCE_COLD_OHM: f64 = 0.003808703729236238;
/// Activation temperature of the part that grows in the cold.
pub const RESISTANCE_COLD_ACTIVATION_K: f64 = 4662.3507187309315;
/// Exchange current of the surface reaction at 25 degC.
pub const EXCHANGE_CURRENT_A: f64 = 7.071674568238423;
/// Activation temperature of the exchange current.
pub const EXCHANGE_CURRENT_ACTIVATION_K: f64 = 5585.755376007683;
/// Slope factor of the surface reaction while its overpotential has the discharging sign.
pub const DISCHARGE_SLOPE_FACTOR: f64 = 0.4391459868803579;
/// Double-layer capacitance beside each surface reaction.
pub const DOUBLE_LAYER_F: f64 = 32.522796339896;
/// Exchange current, at full charge and 25 degC, of the step that is slow only near full charge.
pub const NEAR_FULL_EXCHANGE_CURRENT_A: f64 = 0.6112030975920181;
/// Activation temperature of the near-full exchange current.
pub const NEAR_FULL_ACTIVATION_K: f64 = 6556.635544986871;
/// Range of surface state of charge below full over which the near-full step fades.
pub const NEAR_FULL_RANGE: f64 = 0.08428161227052938;
/// Diffusion time of lithium in the particles at 25 degC.
pub const DIFFUSION_TIME_S: f64 = 1609.9270615985909;
/// Activation temperature of the diffusion time.
pub const DIFFUSION_ACTIVATION_K: f64 = 5611.902700436816;
/// Hysteresis half-gap at full charge at 25 degC.
pub const HYSTERESIS_FULL_V: f64 = 0.04475462198708377;
/// Range of surface state of charge below full over which the half-gap at full fades.
pub const HYSTERESIS_FULL_RANGE: f64 = 0.2552619174037047;
/// Activation temperature of the half-gap at full.
pub const HYSTERESIS_FULL_ACTIVATION_K: f64 = 3947.5906521610236;
/// Hysteresis half-gap at empty. No temperature dependence.
pub const HYSTERESIS_EMPTY_V: f64 = 0.15712567231710017;
/// Range of surface state of charge above empty over which the half-gap at empty fades.
pub const HYSTERESIS_EMPTY_RANGE: f64 = 0.13487617354135945;
/// Hysteresis follows the current's direction after one part in this many of the capacity has passed.
pub const HYSTERESIS_RATE: f64 = 238.4025932699597;
/// Time constant with which hysteresis fades at rest.
pub const HYSTERESIS_FADE_S: f64 = 325.9211027041617;
/// Slow drop of the electrolyte under sustained current, per ampere, at 25 degC.
pub const ELECTROLYTE_OHM: f64 = 0.013822066978543074;
/// Time for the electrolyte drop to form and to fade at 25 degC.
pub const ELECTROLYTE_TIME_S: f64 = 171.6346219712238;
/// Activation temperature of the electrolyte's resistance and time.
pub const ELECTROLYTE_ACTIVATION_K: f64 = 3075.5261524017415;
/// How many numbers were fitted.
pub const FITTED_NUMBERS: usize = 22;

// ---- fixed -----------------------------------------------------------------

/// 25 degC, where the temperature laws are anchored.
pub const REFERENCE_TEMPERATURE_K: f64 = 298.15;
/// Kelvin at 0 degC.
pub const KELVIN_OFFSET: f64 = 273.15;
/// The temperature the rest-voltage curve was measured at.
pub const CURVE_TEMPERATURE_C: f64 = 40.0;
/// Gas constant divided by the Faraday constant: 8.314462618 / 96485.33212.
pub const GAS_OVER_FARADAY_V_PER_K: f64 = 8.617333262281981e-05;
/// The laboratory ends every discharge here.
pub const LOWER_VOLTAGE_LIMIT_V: f64 = 2.8;
/// The laboratory's charging limit of 4.2 V plus 0.042 V.
pub const UPPER_VOLTAGE_LIMIT_V: f64 = 4.242;
/// Factor on the near-full exchange current while lithium arrives: the step vanishes.
pub const NEAR_FULL_CHARGING_FACTOR: f64 = 1000.0;
/// An overpotential divided by its slope voltage is held to this size before `exp`.
pub const OVERPOTENTIAL_RATIO_LIMIT: f64 = 60.0;
/// Time constant of the running mean of the current.
pub const CURRENT_MEAN_TIME_S: f64 = 60.0;
/// Seconds in an hour: ampere-hours to ampere-seconds.
pub const SECONDS_PER_HOUR: f64 = 3_600.0;

/// Modes of the surface lag of a sphere.
pub const DIFFUSION_MODES: usize = 11;
/// Time of each mode as a fraction of the diffusion time. A sphere's, not this cell's.
pub const DIFFUSION_MODE_TIME_FRACTION: [f64; DIFFUSION_MODES] = [
    0.0495276828275572,
    0.013757689674321444,
    0.0038215804650892898,
    0.0010615501291914692,
    0.0002948750358865193,
    8.19097321906998e-05,
    2.2752703386305498e-05,
    6.320195385084861e-06,
    1.7556098291902389e-06,
    4.876693969972886e-07,
    1.3546372138813572e-07,
];
/// Weight of each mode. A sphere's, not this cell's.
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
/// Weight of the part of the surface lag that acts at once.
pub const DIFFUSION_AT_ONCE_WEIGHT: f64 = 1.77411684484381e-05;

// ---- one row per cell ------------------------------------------------------

/// What is locked for one cell of the type.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CellNumbers {
    /// The dataset's name for the cell.
    pub name: &'static str,
    /// The number in that name: 80 for cell `m80`.
    pub cell_number: f64,
    /// Capacity at the cell's first capacity test, a slow discharge at 40 degC to 2.8 V, in ampere-hours.
    pub reference_capacity_ah: f64,
    /// Charge delivered at that test divided by the charge put back by the
    /// recharge after it. A caller uses it to read the capacity from a
    /// recharge record. [`step`] does not use it.
    pub charge_return_ratio: f64,
    /// Multiplies the three resistances and divides the two exchange currents.
    pub resistance_scale: f64,
}

/// The cells the card holds. A further cell is one more row here and nothing else.
const CELL_TABLE: &[CellNumbers] = &[
    // m80: the cell every one of the 22 numbers was fitted on. Its capacity
    // and ratio are `round3/fitted_range.json` of the study.
    CellNumbers {
        name: "m80",
        cell_number: 80.0,
        reference_capacity_ah: 4.67683635335689,
        charge_return_ratio: 0.9977038043683101,
        resistance_scale: 1.0,
    },
    // m1000: the scale fitted on its discharge at a third of the capacity per
    // hour, its pulse test, its slow loop and its capacity test at each
    // temperature from -10 to 40 degC, and its capacity at 40 degC
    // (`round3/per_cell_m1000_r3.json`). Its ratio is in no file of the
    // study: it is what the study's own function, `backbone.block_capacities`,
    // returns for this cell from its opened capacity test at 40 degC and the
    // recharge after it, 4.674655 Ah delivered and 4.687700 Ah put back, and
    // with it that function returns the six capacities of the file bit for bit.
    CellNumbers {
        name: "m1000",
        cell_number: 1000.0,
        reference_capacity_ah: 4.674654617421927,
        charge_return_ratio: 0.9972171037869162,
        resistance_scale: 0.9119920763410333,
    },
    // m448-N, the dataset's third cell: its capacity and its ratio measured
    // and its scale fitted from its own 35 fitting files, with every other
    // number and the curves held (`final_check/per_cell_m448n.json` of the
    // study, SHA-256
    // f95fb75362492d8783e638c41a3a1ce83ab7745ddee0992e6f1bb84eb4cebf30).
    // None of its other records had been opened when this row was written.
    CellNumbers {
        name: "m448-N",
        cell_number: 448.0,
        reference_capacity_ah: 4.6696220353547755,
        charge_return_ratio: 0.9976972129208563,
        resistance_scale: 0.8243415837166522,
    },
];

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

// ---- the declared range ----------------------------------------------------

/// The accuracy the dataset states for the tester's current on its 60 A range.
pub const TESTER_CURRENT_ACCURACY_A: f64 = 0.024;
/// The accuracy the dataset states for the chamber.
pub const CHAMBER_ACCURACY_K: f64 = 0.3;
/// The capacities the two cells had in the scored tests, 4.174 to 4.689 Ah, less 0.01 Ah, to a hundredth.
pub const CAPACITY_MIN_AH: f64 = 4.16;
/// The same, plus 0.01 Ah.
pub const CAPACITY_MAX_AH: f64 = 4.70;
/// The least the case sensor read in a scored test, -12.586 degC, less the chamber's accuracy, to a tenth.
pub const CASE_TEMPERATURE_MIN_C: f64 = -12.9;
/// The most it read, 45.535 degC, plus the chamber's accuracy, to a tenth.
pub const CASE_TEMPERATURE_MAX_C: f64 = 45.9;
/// The tester's programmed pulse of 13.5 A, three times the nominal 4.5 Ah, plus the tester's accuracy.
pub const DISCHARGE_CURRENT_MAX_A: f64 = 13.524;
/// The largest charging current in a scored drive cycle, 13.960 A on the second cell, plus the tester's accuracy.
pub const CHARGE_CURRENT_MAX_A: f64 = 13.984;
/// The largest value the running mean of the current reaches discharging in a scored test, 8.698 A, plus the tester's accuracy.
pub const SUSTAINED_DISCHARGE_MAX_A: f64 = 8.722;
/// The largest value it reaches charging, 2.969 A, plus the tester's accuracy.
pub const SUSTAINED_CHARGE_MAX_A: f64 = 2.993;
/// Above this running mean, discharging, a landed step carries the flag. A run at the rated current sits at 4.50 A.
pub const SUSTAINED_FLAG_A: f64 = 4.6;
pub const STATE_OF_CHARGE_MIN: f64 = 0.0;
/// The cell was never started above 1.012.
pub const STATE_OF_CHARGE_MAX: f64 = 1.02;
/// The largest step the card stands behind: the sampling of the data it was fitted on.
pub const GATE_DT_S: f64 = 1.0;
/// The smallest step the card stands behind: the same one second. A shorter
/// step changes the answer where the current reverses, and nothing measured
/// checks it.
pub const FLOOR_DT_S: f64 = 1.0;
/// The rounding allowance of the running sum of the state of charge, for each
/// step of a run so far including this one and two more: 2^-52, the spacing
/// of 64-bit numbers between 1 and 2.
pub const STATE_OF_CHARGE_ALLOWANCE_PER_STEP: f64 = 2.220_446_049_250_313e-16;
/// Steps added to the count in the state-of-charge allowance: the start and
/// the current, each read from a decimal.
pub const STATE_OF_CHARGE_ALLOWANCE_EXTRA_STEPS: f64 = 2.0;
/// Blocks of tests, by the chamber's setting.
pub const BLOCK_COUNT: usize = 5;
/// The chamber's setting of each block, in degC.
pub const BLOCK_CHAMBER_C: [f64; BLOCK_COUNT] = [-10.0, 0.0, 10.0, 25.0, 40.0];
/// The least case temperature of each block's scored runs, less the chamber's
/// accuracy, to a tenth below. The -10 degC block takes the card's least.
pub const BLOCK_CASE_TEMPERATURE_LEAST_C: [f64; BLOCK_COUNT] = [-12.9, 0.0, 9.8, 23.3, 28.1];
/// The most case temperature of each block's scored runs, plus the chamber's
/// accuracy, to a tenth above.
pub const BLOCK_CASE_TEMPERATURE_MOST_C: [f64; BLOCK_COUNT] = [12.6, 20.4, 26.8, 31.4, 45.9];
/// The least capacity of each block, in its scored runs and its fitting
/// files, less 0.01 Ah, to a hundredth below.
pub const BLOCK_CAPACITY_LEAST_AH: [f64; BLOCK_COUNT] = [4.16, 4.20, 4.17, 4.35, 4.56];
/// The most capacity of each block, plus 0.01 Ah, to a hundredth above.
pub const BLOCK_CAPACITY_MOST_AH: [f64; BLOCK_COUNT] = [4.26, 4.30, 4.30, 4.53, 4.70];
/// The largest change of case temperature between two consecutive
/// one-second samples of a scored run, 5.023 K, plus the chamber's accuracy,
/// to a tenth above.
pub const CASE_TEMPERATURE_CHANGE_MAX_K: f64 = 5.4;
/// The largest span of case temperature inside one scored run, 18.811 K,
/// plus the chamber's accuracy, to a tenth above.
pub const CASE_TEMPERATURE_SPAN_MAX_K: f64 = 19.2;
/// `|defect| / (E(1) - E(0))` must stay at or under this.
pub const RESIDUAL_CONSERVATION_LIMIT: f64 = 1.0e-10;

/// The running mean of the current is above 4.6 A, discharging. Only the
/// hill cycle exercises this region, and below 10 degC the card is known to
/// read high in it. The step lands.
pub const FLAG_SUSTAINED_ABOVE_1C: u64 = 1 << 0;

/// Why a step did not advance the cell or the frame.
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
    /// Capacity is below 4.16 Ah or above 4.70 Ah.
    CapacityOutsideCard,
    /// Case temperature is below -12.9 degC or above 45.9 degC.
    CaseTemperatureOutsideCard,
    /// The current is above 13.524 A discharging or above 13.984 A charging.
    CurrentOutsideCard,
    /// No block of tests holds both the capacity and the case temperature: the capacity is outside the band scored at the case temperatures that block spanned.
    CapacityOutsideCardAtTemperature,
    /// Case temperature differs from the last step's by more than 5.4 K.
    CaseTemperatureChangeAboveCard,
    /// Case temperature would take its span since the rested start, carried across a continued start, past 19.2 K.
    CaseTemperatureSpanAboveCard,
    /// The landing running mean of the current is above 8.722 A discharging or above 2.993 A charging.
    SustainedCurrentOutsideCard,
    /// State of charge, incoming or landing, is below 0 or above 1.02 by more than the rounding allowance of its running sum.
    StateOfChargeOutsideCard,
    /// Landing surface state of charge is below -0.05 or above 1.05, the ends of the measured curves.
    SurfaceOutsideCurve,
    /// Landing terminal voltage is below 2.8 V.
    UndervoltageLimit,
    /// Landing terminal voltage is above 4.242 V.
    OvervoltageLimit,
    /// The trial step created energy from nothing.
    SpontaneousGeneration,
    /// The trial step destroyed energy.
    FirstLawBreach,
}

/// Held over the step. Positive current discharges the cell, negative charges it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CellLoad {
    pub current_a: f64,
    /// The temperature measured on the cell's case at the end of the step.
    pub case_temperature_c: f64,
}

/// The locked cell type. Every number is a recorded copy.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Cell2170Model3 {
    pub resistance_fixed_ohm: f64,
    pub resistance_cold_ohm: f64,
    pub resistance_cold_activation_k: f64,
    pub exchange_current_a: f64,
    pub exchange_current_activation_k: f64,
    pub discharge_slope_factor: f64,
    pub double_layer_f: f64,
    pub near_full_exchange_current_a: f64,
    pub near_full_activation_k: f64,
    pub near_full_range: f64,
    pub diffusion_time_s: f64,
    pub diffusion_activation_k: f64,
    pub hysteresis_full_v: f64,
    pub hysteresis_full_range: f64,
    pub hysteresis_full_activation_k: f64,
    pub hysteresis_empty_v: f64,
    pub hysteresis_empty_range: f64,
    pub hysteresis_rate: f64,
    pub hysteresis_fade_s: f64,
    pub electrolyte_ohm: f64,
    pub electrolyte_time_s: f64,
    pub electrolyte_activation_k: f64,
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
    pub capacity_min_ah: f64,
    pub capacity_max_ah: f64,
    pub case_temperature_min_c: f64,
    pub case_temperature_max_c: f64,
    pub discharge_current_max_a: f64,
    pub charge_current_max_a: f64,
    pub sustained_discharge_max_a: f64,
    pub sustained_charge_max_a: f64,
    pub sustained_flag_a: f64,
    pub state_of_charge_min: f64,
    pub state_of_charge_max: f64,
    pub gate_dt_s: f64,
    pub floor_dt_s: f64,
    pub residual_conservation_limit: f64,
    pub state_of_charge_allowance_per_step: f64,
    pub state_of_charge_allowance_extra_steps: f64,
    pub block_chamber_c: [f64; BLOCK_COUNT],
    pub block_case_temperature_least_c: [f64; BLOCK_COUNT],
    pub block_case_temperature_most_c: [f64; BLOCK_COUNT],
    pub block_capacity_least_ah: [f64; BLOCK_COUNT],
    pub block_capacity_most_ah: [f64; BLOCK_COUNT],
    pub case_temperature_change_max_k: f64,
    pub case_temperature_span_max_k: f64,
}

impl Cell2170Model3 {
    /// The fitted cell type. The constructor takes no arguments.
    #[must_use]
    pub fn cell() -> Self {
        let cell = Self {
            resistance_fixed_ohm: RESISTANCE_FIXED_OHM,
            resistance_cold_ohm: RESISTANCE_COLD_OHM,
            resistance_cold_activation_k: RESISTANCE_COLD_ACTIVATION_K,
            exchange_current_a: EXCHANGE_CURRENT_A,
            exchange_current_activation_k: EXCHANGE_CURRENT_ACTIVATION_K,
            discharge_slope_factor: DISCHARGE_SLOPE_FACTOR,
            double_layer_f: DOUBLE_LAYER_F,
            near_full_exchange_current_a: NEAR_FULL_EXCHANGE_CURRENT_A,
            near_full_activation_k: NEAR_FULL_ACTIVATION_K,
            near_full_range: NEAR_FULL_RANGE,
            diffusion_time_s: DIFFUSION_TIME_S,
            diffusion_activation_k: DIFFUSION_ACTIVATION_K,
            hysteresis_full_v: HYSTERESIS_FULL_V,
            hysteresis_full_range: HYSTERESIS_FULL_RANGE,
            hysteresis_full_activation_k: HYSTERESIS_FULL_ACTIVATION_K,
            hysteresis_empty_v: HYSTERESIS_EMPTY_V,
            hysteresis_empty_range: HYSTERESIS_EMPTY_RANGE,
            hysteresis_rate: HYSTERESIS_RATE,
            hysteresis_fade_s: HYSTERESIS_FADE_S,
            electrolyte_ohm: ELECTROLYTE_OHM,
            electrolyte_time_s: ELECTROLYTE_TIME_S,
            electrolyte_activation_k: ELECTROLYTE_ACTIVATION_K,
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
            capacity_min_ah: CAPACITY_MIN_AH,
            capacity_max_ah: CAPACITY_MAX_AH,
            case_temperature_min_c: CASE_TEMPERATURE_MIN_C,
            case_temperature_max_c: CASE_TEMPERATURE_MAX_C,
            discharge_current_max_a: DISCHARGE_CURRENT_MAX_A,
            charge_current_max_a: CHARGE_CURRENT_MAX_A,
            sustained_discharge_max_a: SUSTAINED_DISCHARGE_MAX_A,
            sustained_charge_max_a: SUSTAINED_CHARGE_MAX_A,
            sustained_flag_a: SUSTAINED_FLAG_A,
            state_of_charge_min: STATE_OF_CHARGE_MIN,
            state_of_charge_max: STATE_OF_CHARGE_MAX,
            gate_dt_s: GATE_DT_S,
            floor_dt_s: FLOOR_DT_S,
            residual_conservation_limit: RESIDUAL_CONSERVATION_LIMIT,
            state_of_charge_allowance_per_step: STATE_OF_CHARGE_ALLOWANCE_PER_STEP,
            state_of_charge_allowance_extra_steps: STATE_OF_CHARGE_ALLOWANCE_EXTRA_STEPS,
            block_chamber_c: BLOCK_CHAMBER_C,
            block_case_temperature_least_c: BLOCK_CASE_TEMPERATURE_LEAST_C,
            block_case_temperature_most_c: BLOCK_CASE_TEMPERATURE_MOST_C,
            block_capacity_least_ah: BLOCK_CAPACITY_LEAST_AH,
            block_capacity_most_ah: BLOCK_CAPACITY_MOST_AH,
            case_temperature_change_max_k: CASE_TEMPERATURE_CHANGE_MAX_K,
            case_temperature_span_max_k: CASE_TEMPERATURE_SPAN_MAX_K,
        };
        if cell.is_valid() {
            cell
        } else {
            ::core::panic!("2170 cell type failed is_valid");
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
        same_bits(&self.diffusion_mode_time_fraction, &DIFFUSION_MODE_TIME_FRACTION)
            && same_bits(&self.diffusion_mode_weight, &DIFFUSION_MODE_WEIGHT)
            && same_bits(&self.rest_v_new_40c, &REST_V_NEW_40C)
            && same_bits(&self.rest_v_per_capacity_lost, &REST_V_PER_CAPACITY_LOST)
            && same_bits(&self.rest_v_per_k, &REST_V_PER_K)
            && same_bits(&self.block_chamber_c, &BLOCK_CHAMBER_C)
            && same_bits(&self.block_case_temperature_least_c, &BLOCK_CASE_TEMPERATURE_LEAST_C)
            && same_bits(&self.block_case_temperature_most_c, &BLOCK_CASE_TEMPERATURE_MOST_C)
            && same_bits(&self.block_capacity_least_ah, &BLOCK_CAPACITY_LEAST_AH)
            && same_bits(&self.block_capacity_most_ah, &BLOCK_CAPACITY_MOST_AH)
            && same_bits(
                &[
                    self.resistance_fixed_ohm,
                    self.resistance_cold_ohm,
                    self.resistance_cold_activation_k,
                    self.exchange_current_a,
                    self.exchange_current_activation_k,
                    self.discharge_slope_factor,
                    self.double_layer_f,
                    self.near_full_exchange_current_a,
                    self.near_full_activation_k,
                    self.near_full_range,
                    self.diffusion_time_s,
                    self.diffusion_activation_k,
                    self.hysteresis_full_v,
                    self.hysteresis_full_range,
                    self.hysteresis_full_activation_k,
                    self.hysteresis_empty_v,
                    self.hysteresis_empty_range,
                    self.hysteresis_rate,
                    self.hysteresis_fade_s,
                    self.electrolyte_ohm,
                    self.electrolyte_time_s,
                    self.electrolyte_activation_k,
                ],
                &[
                    RESISTANCE_FIXED_OHM,
                    RESISTANCE_COLD_OHM,
                    RESISTANCE_COLD_ACTIVATION_K,
                    EXCHANGE_CURRENT_A,
                    EXCHANGE_CURRENT_ACTIVATION_K,
                    DISCHARGE_SLOPE_FACTOR,
                    DOUBLE_LAYER_F,
                    NEAR_FULL_EXCHANGE_CURRENT_A,
                    NEAR_FULL_ACTIVATION_K,
                    NEAR_FULL_RANGE,
                    DIFFUSION_TIME_S,
                    DIFFUSION_ACTIVATION_K,
                    HYSTERESIS_FULL_V,
                    HYSTERESIS_FULL_RANGE,
                    HYSTERESIS_FULL_ACTIVATION_K,
                    HYSTERESIS_EMPTY_V,
                    HYSTERESIS_EMPTY_RANGE,
                    HYSTERESIS_RATE,
                    HYSTERESIS_FADE_S,
                    ELECTROLYTE_OHM,
                    ELECTROLYTE_TIME_S,
                    ELECTROLYTE_ACTIVATION_K,
                ],
            )
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
                    self.capacity_min_ah,
                    self.capacity_max_ah,
                    self.case_temperature_min_c,
                    self.case_temperature_max_c,
                    self.discharge_current_max_a,
                    self.charge_current_max_a,
                    self.sustained_discharge_max_a,
                    self.sustained_charge_max_a,
                    self.sustained_flag_a,
                    self.state_of_charge_min,
                    self.state_of_charge_max,
                    self.gate_dt_s,
                    self.floor_dt_s,
                    self.residual_conservation_limit,
                    self.state_of_charge_allowance_per_step,
                    self.state_of_charge_allowance_extra_steps,
                    self.case_temperature_change_max_k,
                    self.case_temperature_span_max_k,
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
                    CAPACITY_MIN_AH,
                    CAPACITY_MAX_AH,
                    CASE_TEMPERATURE_MIN_C,
                    CASE_TEMPERATURE_MAX_C,
                    DISCHARGE_CURRENT_MAX_A,
                    CHARGE_CURRENT_MAX_A,
                    SUSTAINED_DISCHARGE_MAX_A,
                    SUSTAINED_CHARGE_MAX_A,
                    SUSTAINED_FLAG_A,
                    STATE_OF_CHARGE_MIN,
                    STATE_OF_CHARGE_MAX,
                    GATE_DT_S,
                    FLOOR_DT_S,
                    RESIDUAL_CONSERVATION_LIMIT,
                    STATE_OF_CHARGE_ALLOWANCE_PER_STEP,
                    STATE_OF_CHARGE_ALLOWANCE_EXTRA_STEPS,
                    CASE_TEMPERATURE_CHANGE_MAX_K,
                    CASE_TEMPERATURE_SPAN_MAX_K,
                ],
            )
    }

    /// The largest step the card stands behind: one second, the sampling of
    /// the measurements every fit and every score was made on. The rule for
    /// the reaction changing sign was fitted at this step, and a load that
    /// changes within a second is not represented by a longer one.
    #[must_use]
    pub fn gate_dt_s(&self) -> Option<f64> {
        if self.is_valid() { Some(self.gate_dt_s) } else { None }
    }

    /// The smallest step the card stands behind: the same one second. Where
    /// the current reverses, the rule for the reaction changing sign makes the
    /// answer depend on the step, by 49 mV between 0.1 s and 1 s in the reversal that was checked, and
    /// data logged once a second cannot check a shorter step.
    #[must_use]
    pub fn floor_dt_s(&self) -> Option<f64> {
        if self.is_valid() { Some(self.floor_dt_s) } else { None }
    }

    /// Whether one block of tests holds both this capacity and this case
    /// temperature, each inside that block's band.
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

    /// Rest voltage at a state of charge between the ends of the curves, a
    /// case temperature and a capacity of the cell in row `row`, in volts.
    /// `None` outside the curves.
    #[must_use]
    pub fn rest_v(
        &self,
        row: usize,
        capacity_ah: f64,
        state_of_charge: f64,
        case_temperature_c: f64,
    ) -> Option<f64> {
        if row >= CELL_COUNT
            || !(state_of_charge >= self.curve_first_state_of_charge
                && state_of_charge <= self.curve_last_state_of_charge)
        {
            return None;
        }
        let age = 1.0 - capacity_ah / self.cells[row].reference_capacity_ah;
        let rest_v = rest_v_at(self, age, state_of_charge, case_temperature_c);
        if rest_v.is_finite() { Some(rest_v) } else { None }
    }
}

/// Integrated cell. The ledger fields are part of the state; write them only through [`step`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CellState {
    pub t_s: f64,
    /// The number in the dataset's name for the cell: 80 or 1000.
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
    /// The slow drop of the electrolyte. Positive while discharging.
    pub electrolyte_v: f64,
    /// Running mean of the current, 60 s time constant.
    pub current_mean_a: f64,
    /// State of charge at the particle surface, as the last step left it.
    pub surface_state_of_charge: f64,
    /// `(1 - state_of_charge) * capacity_ah`.
    pub charge_removed_ah: f64,
    /// Current held over the last step. Zero before the first step.
    pub current_a: f64,
    /// Case temperature held over the last step. Zero before the first step.
    pub case_temperature_c: f64,
    /// Least case temperature of the steps since the rested start, or the least a continued
    /// start carried. Zero before the first step of a start that carried none.
    pub case_temperature_least_c: f64,
    /// Most case temperature of the steps since the rested start, or the most a continued start
    /// carried. Zero before the first step of a start that carried none.
    pub case_temperature_most_c: f64,
    /// The least and most case temperature hold the run's span: after the first step, or from a
    /// start that carried them. False only at a start that carried none, whose span begins
    /// with its first step.
    pub case_span_known: bool,
    /// Terminal voltage at the end of the last step. Zero before the first step.
    pub terminal_v: f64,
    /// Rest voltage at the surface state of charge and the case temperature of the last step. Zero before the first step.
    pub rest_v: f64,
    /// Immediate resistance at the case temperature of the last step. Zero before the first step.
    pub resistance_at_once_ohm: f64,
    pub chemical_energy_j: f64,
    pub energy_anchor_j: f64,
    /// Work the cell has done at its terminals. Positive while discharging.
    pub electrical_work_j: f64,
    /// Heat: ohmic, the two reactions, the electrolyte, hysteresis, and mixing with the reversible part.
    pub heat_j: f64,
    pub energy_defect_j: f64,
    pub residual_conservation_error: f64,
    /// The running mean of the current is above 4.6 A, discharging.
    pub sustained_above_1c: bool,
}

impl CellState {
    /// A rested cell: this cell, this state of charge, this capacity, every
    /// memory at zero, and the ledger anchored there. `None` unless the cell
    /// number names a row and both numbers are finite and on the card.
    #[must_use]
    pub fn try_rested(
        cell: &Cell2170Model3,
        cell_number: f64,
        state_of_charge: f64,
        capacity_ah: f64,
    ) -> Option<Self> {
        if !cell.is_valid()
            || !cell_number.is_finite()
            || !state_of_charge.is_finite()
            || !capacity_ah.is_finite()
        {
            return None;
        }
        let row = cell.cell_of(cell_number)?;
        if !(capacity_ah >= cell.capacity_min_ah && capacity_ah <= cell.capacity_max_ah) {
            return None;
        }
        if !(state_of_charge >= cell.state_of_charge_min
            && state_of_charge <= cell.state_of_charge_max)
        {
            return None;
        }
        let age = 1.0 - capacity_ah / cell.cells[row].reference_capacity_ah;
        let chemical_energy_j =
            SECONDS_PER_HOUR * capacity_ah * enthalpy_integral_v(cell, age, state_of_charge);
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
            current_mean_a: 0.0,
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
            chemical_energy_j,
            energy_anchor_j: chemical_energy_j,
            electrical_work_j: 0.0,
            heat_j: 0.0,
            energy_defect_j: 0.0,
            residual_conservation_error: 0.0,
            sustained_above_1c: false,
        };
        if !state_is_finite(&state) {
            return None;
        }
        Some(state)
    }

    /// The same start carrying the least and the most case temperature of the run so far, so
    /// that a run continued from a returned state is weighed against the span of the whole run.
    /// A fresh start states both equal to its first case temperature. `None` unless both are
    /// finite, on the card's case temperatures, the least is not above the most, and their
    /// span is at most [`CASE_TEMPERATURE_SPAN_MAX_K`].
    #[must_use]
    pub fn with_case_span(
        self,
        cell: &Cell2170Model3,
        case_temperature_least_c: f64,
        case_temperature_most_c: f64,
    ) -> Option<Self> {
        if !case_temperature_least_c.is_finite()
            || !case_temperature_most_c.is_finite()
            || case_temperature_least_c < cell.case_temperature_min_c
            || case_temperature_most_c > cell.case_temperature_max_c
            || case_temperature_least_c > case_temperature_most_c
            || !(case_temperature_most_c - case_temperature_least_c <= cell.case_temperature_span_max_k)
        {
            return None;
        }
        Some(Self {
            case_temperature_least_c,
            case_temperature_most_c,
            case_span_known: true,
            ..self
        })
    }
}

/// Advance one step and write the frame.
///
/// A rejected step leaves `state` and `frame` as they were.
#[must_use]
pub fn step(
    cell: &Cell2170Model3,
    state: &mut CellState,
    frame: &mut LastStateFrame64,
    load: CellLoad,
    dt_s: f64,
) -> Result<(), StepReject> {
    validate(cell, state, load, dt_s)?;
    capacity_with_temperature(cell, state, load)?;
    let trial = match integrate(cell, state, load, dt_s) {
        Ok(trial) => trial,
        Err(reject) => return Err(reject),
    };
    commit(cell, state, frame, load, dt_s, trial)
}

/// Every input and the incoming state, before any curve is read.
fn validate(
    cell: &Cell2170Model3,
    state: &CellState,
    load: CellLoad,
    dt_s: f64,
) -> Result<(), StepReject> {
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
    if !(state.capacity_ah >= cell.capacity_min_ah && state.capacity_ah <= cell.capacity_max_ah) {
        return Err(StepReject::CapacityOutsideCard);
    }
    if load.case_temperature_c < cell.case_temperature_min_c
        || load.case_temperature_c > cell.case_temperature_max_c
    {
        return Err(StepReject::CaseTemperatureOutsideCard);
    }
    if load.current_a > cell.discharge_current_max_a || load.current_a < -cell.charge_current_max_a
    {
        return Err(StepReject::CurrentOutsideCard);
    }
    // A step has landed since the rested start exactly when the clock has moved.
    if state.t_s > 0.0
        && !(abs_f64(load.case_temperature_c - state.case_temperature_c)
            <= cell.case_temperature_change_max_k)
    {
        return Err(StepReject::CaseTemperatureChangeAboveCard);
    }
    // The span is known after the first step, or from a start that carried it.
    if state.case_span_known {
        if !(state.case_temperature_least_c <= state.case_temperature_most_c) {
            return Err(StepReject::CaseTemperatureSpanAboveCard);
        }
        let least = if load.case_temperature_c < state.case_temperature_least_c {
            load.case_temperature_c
        } else {
            state.case_temperature_least_c
        };
        let most = if load.case_temperature_c > state.case_temperature_most_c {
            load.case_temperature_c
        } else {
            state.case_temperature_most_c
        };
        if !(most - least <= cell.case_temperature_span_max_k) {
            return Err(StepReject::CaseTemperatureSpanAboveCard);
        }
    }
    Ok(())
}

/// The capacity against the band compared at the step's case temperature,
/// after [`validate`] has held each to its own range.
fn capacity_with_temperature(
    cell: &Cell2170Model3,
    state: &CellState,
    load: CellLoad,
) -> Result<(), StepReject> {
    if cell.capacity_compared_at(state.capacity_ah, load.case_temperature_c) {
        Ok(())
    } else {
        Err(StepReject::CapacityOutsideCardAtTemperature)
    }
}

/// What a trial step hands to [`commit`]: where the charge and every memory
/// land, the voltage there with the two readings it was made from, and the
/// chemical energy the step released.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Trial {
    state_of_charge: f64,
    diffusion_lag: [f64; DIFFUSION_MODES],
    surface_state_of_charge: f64,
    reaction_overpotential_v: f64,
    near_full_overpotential_v: f64,
    hysteresis_v: f64,
    electrolyte_v: f64,
    current_mean_a: f64,
    resistance_at_once_ohm: f64,
    rest_v: f64,
    terminal_v: f64,
    chemical_step_j: f64,
}

/// The landing running mean of the current against its two limits.
fn sustained_current_on_card(cell: &Cell2170Model3, current_mean_a: f64) -> bool {
    current_mean_a <= cell.sustained_discharge_max_a
        && current_mean_a >= -cell.sustained_charge_max_a
}

/// The rounding allowance of the running sum of the state of charge for the
/// step that lands at `t_s + dt_s`: one spacing for each step of the run so
/// far, this one included, and two more.
fn state_of_charge_allowance(cell: &Cell2170Model3, t_s: f64, dt_s: f64) -> f64 {
    cell.state_of_charge_allowance_per_step
        * (t_s / dt_s + 1.0 + cell.state_of_charge_allowance_extra_steps)
}

/// A state of charge against 0 and 1.02, with the rounding allowance.
fn state_of_charge_on_card(cell: &Cell2170Model3, state_of_charge: f64, allowance: f64) -> bool {
    state_of_charge >= cell.state_of_charge_min - allowance
        && state_of_charge <= cell.state_of_charge_max + allowance
}

fn surface_on_curve(cell: &Cell2170Model3, surface_state_of_charge: f64) -> bool {
    surface_state_of_charge >= cell.curve_first_state_of_charge
        && surface_state_of_charge <= cell.curve_last_state_of_charge
}

fn integrate(
    cell: &Cell2170Model3,
    state: &CellState,
    load: CellLoad,
    dt_s: f64,
) -> Result<Trial, StepReject> {
    let row = match cell.cell_of(state.cell_number) {
        Some(row) => row,
        None => return Err(StepReject::CellNotRecognized),
    };
    let numbers = cell.cells[row];
    let scale = numbers.resistance_scale;
    let resistance_fixed_ohm = cell.resistance_fixed_ohm * scale;
    let resistance_cold_ohm = cell.resistance_cold_ohm * scale;
    let electrolyte_ohm = cell.electrolyte_ohm * scale;
    let exchange_current_a = cell.exchange_current_a / scale;
    let near_full_exchange_current_a = cell.near_full_exchange_current_a / scale;

    let current_a = load.current_a;
    let kelvin = load.case_temperature_c + cell.kelvin_offset;
    let x = 1.0 / kelvin - 1.0 / cell.reference_temperature_k;
    let capacity_ah = state.capacity_ah;
    let age = 1.0 - capacity_ah / numbers.reference_capacity_ah;

    // 7, taken first because its limit is tested first: the running mean of the current.
    let current_mean_a =
        current_a + (state.current_mean_a - current_a) * exp(-dt_s / cell.current_mean_time_s);
    if !current_mean_a.is_finite() {
        return Err(StepReject::NonFiniteState);
    }
    if !sustained_current_on_card(cell, current_mean_a) {
        return Err(StepReject::SustainedCurrentOutsideCard);
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
    let diffusion_time_s = cell.diffusion_time_s * exp(cell.diffusion_activation_k * x);
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
    let surface_state_of_charge =
        state_of_charge - (lag_sum + cell.diffusion_at_once_weight * diffusion_time_s * rate);
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

    // 3 immediate resistance and the two surface reactions
    let resistance_at_once_ohm =
        resistance_fixed_ohm + resistance_cold_ohm * exp(cell.resistance_cold_activation_k * x);
    let exchange_a = exchange_current_a * exp(-cell.exchange_current_activation_k * x);
    let near_full_exchange_a = near_full_exchange_current_a
        * exp(-cell.near_full_activation_k * x + (1.0 - surface_held) / cell.near_full_range);
    let slope_v = 2.0 * cell.gas_over_faraday_v_per_k * kelvin;
    let slope_factor = cell.discharge_slope_factor;
    let incoming = state.reaction_overpotential_v;
    let mut reaction_overpotential_v;
    if incoming > 0.0 || (incoming == 0.0 && current_a >= 0.0) {
        reaction_overpotential_v =
            react(cell, incoming, current_a, exchange_a, slope_v / slope_factor, dt_s);
        if reaction_overpotential_v < 0.0 {
            reaction_overpotential_v =
                react(cell, 0.0, current_a, exchange_a * slope_factor, slope_v, dt_s);
        }
    } else {
        reaction_overpotential_v =
            react(cell, incoming, current_a, exchange_a * slope_factor, slope_v, dt_s);
        if reaction_overpotential_v > 0.0 {
            reaction_overpotential_v =
                react(cell, 0.0, current_a, exchange_a, slope_v / slope_factor, dt_s);
        }
    }
    let mut near_full_overpotential_v = react(
        cell,
        state.near_full_overpotential_v,
        current_a,
        near_full_exchange_a,
        slope_v,
        dt_s,
    );
    if !(near_full_overpotential_v >= 0.0) {
        near_full_overpotential_v = react(
            cell,
            0.0,
            current_a,
            near_full_exchange_a * cell.near_full_charging_factor,
            slope_v,
            dt_s,
        );
    }

    // 4 hysteresis
    let half_gap_v = cell.hysteresis_full_v
        * exp(-(1.0 - surface_held) / cell.hysteresis_full_range)
        * exp(cell.hysteresis_full_activation_k * x)
        + cell.hysteresis_empty_v * exp(-surface_held / cell.hysteresis_empty_range);
    let follow_per_s = cell.hysteresis_rate * abs_f64(rate);
    let target_v = if current_a < 0.0 { half_gap_v } else { -half_gap_v };
    let fade_per_s = follow_per_s + 1.0 / cell.hysteresis_fade_s;
    let settled_v = follow_per_s * target_v / fade_per_s;
    let hysteresis_v = settled_v + (state.hysteresis_v - settled_v) * exp(-fade_per_s * dt_s);

    // 5 slow drop of the electrolyte
    let cold = exp(cell.electrolyte_activation_k * x);
    let electrolyte_settled_v = electrolyte_ohm * cold * current_a;
    let electrolyte_v = electrolyte_settled_v
        + (state.electrolyte_v - electrolyte_settled_v)
            * exp(-dt_s / (cell.electrolyte_time_s * cold));

    // 6 terminal voltage
    let rest_v = rest_v_at(cell, age, surface_state_of_charge, load.case_temperature_c);
    let terminal_v = rest_v + hysteresis_v
        - current_a * resistance_at_once_ohm
        - reaction_overpotential_v
        - near_full_overpotential_v
        - electrolyte_v;

    // the chemical energy the step released: the exact integral of the enthalpy voltage
    let chemical_step_j = SECONDS_PER_HOUR
        * capacity_ah
        * (enthalpy_integral_v(cell, age, state.state_of_charge)
            - enthalpy_integral_v(cell, age, state_of_charge));

    let trial = Trial {
        state_of_charge,
        diffusion_lag,
        surface_state_of_charge,
        reaction_overpotential_v,
        near_full_overpotential_v,
        hysteresis_v,
        electrolyte_v,
        current_mean_a,
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
    cell: &Cell2170Model3,
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
    if !sustained_current_on_card(cell, trial.current_mean_a) {
        return Err(StepReject::SustainedCurrentOutsideCard);
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
    let current_a = load.current_a;
    let capacity_ah = state.capacity_ah;
    let age = 1.0 - capacity_ah / cell.cells[row].reference_capacity_ah;

    let work_j = current_a * trial.terminal_v * dt_s;
    let ohmic_j = current_a * current_a * trial.resistance_at_once_ohm * dt_s;
    let reactions_j =
        current_a * (trial.reaction_overpotential_v + trial.near_full_overpotential_v) * dt_s;
    let electrolyte_j = current_a * trial.electrolyte_v * dt_s;
    let hysteresis_j = -current_a * trial.hysteresis_v * dt_s;
    let mixing_j = trial.chemical_step_j - current_a * trial.rest_v * dt_s;
    let electrical_work_j = state.electrical_work_j + work_j;
    let heat_j = state.heat_j + (ohmic_j + reactions_j + electrolyte_j + hysteresis_j + mixing_j);

    let chemical_energy_j =
        SECONDS_PER_HOUR * capacity_ah * enthalpy_integral_v(cell, age, trial.state_of_charge);
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
        current_mean_a: trial.current_mean_a,
        surface_state_of_charge: trial.surface_state_of_charge,
        charge_removed_ah: (1.0 - trial.state_of_charge) * capacity_ah,
        current_a,
        case_temperature_c: load.case_temperature_c,
        case_temperature_least_c: if state.case_span_known
            && state.case_temperature_least_c < load.case_temperature_c
        {
            state.case_temperature_least_c
        } else {
            load.case_temperature_c
        },
        case_temperature_most_c: if state.case_span_known
            && state.case_temperature_most_c > load.case_temperature_c
        {
            state.case_temperature_most_c
        } else {
            load.case_temperature_c
        },
        case_span_known: true,
        terminal_v: trial.terminal_v,
        rest_v: trial.rest_v,
        resistance_at_once_ohm: trial.resistance_at_once_ohm,
        chemical_energy_j,
        energy_anchor_j: state.energy_anchor_j,
        electrical_work_j,
        heat_j,
        energy_defect_j: defect,
        residual_conservation_error: abs_f64(defect) / cell_energy_j,
        sustained_above_1c: trial.current_mean_a > cell.sustained_flag_a,
    };
    if !state_is_finite(&landed) {
        return Err(StepReject::NonFiniteState);
    }
    *state = landed;
    publish(state, frame);
    Ok(())
}

fn publish(state: &CellState, frame: &mut LastStateFrame64) {
    frame.t = state.t_s;
    frame.pos[0] = state.terminal_v as f32;
    frame.pos[1] = state.charge_removed_ah as f32;
    frame.pos[2] = state.surface_state_of_charge as f32;
    frame.vel[0] = state.reaction_overpotential_v as f32;
    frame.vel[1] = state.hysteresis_v as f32;
    frame.vel[2] = state.electrolyte_v as f32;
    frame.force_torque = state.current_mean_a as f32;
    frame.residual = state.residual_conservation_error as f32;
    frame.flags = if state.sustained_above_1c {
        FLAG_SUSTAINED_ABOVE_1C
    } else {
        0
    };
}

/// One surface reaction over a step: the exact solution of
/// `C d(eta)/dt = i - 2 I sinh(eta / B)` for a held current `i`, exchange
/// current `I` and slope voltage `B`.
fn react(
    cell: &Cell2170Model3,
    overpotential_v: f64,
    current_a: f64,
    exchange_a: f64,
    slope_v: f64,
    dt_s: f64,
) -> f64 {
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
    let rho = (u - up) / (u - um) * exp(-root * dt_s / (slope_v * cell.double_layer_f));
    slope_v * log((up - rho * um) / (1.0 - rho))
}

/// A place on the curves: the point at or before it, and the share of the
/// way to the next point. On the last point, the interval before it.
fn curve_place(cell: &Cell2170Model3, state_of_charge: f64) -> (usize, f64) {
    let along = (state_of_charge - cell.curve_first_state_of_charge) / cell.curve_spacing;
    let mut index = if along > 0.0 { along as usize } else { 0 };
    if index > CURVE_POINTS - 2 {
        index = CURVE_POINTS - 2;
    }
    (index, along - index as f64)
}

/// The straight line between two points of a curve, at a place.
fn curve_at(values: &[f64; CURVE_POINTS], place: (usize, f64)) -> f64 {
    values[place.0] + (values[place.0 + 1] - values[place.0]) * place.1
}

/// Rest voltage at a state of charge, a case temperature and an age.
fn rest_v_at(cell: &Cell2170Model3, age: f64, state_of_charge: f64, case_temperature_c: f64) -> f64 {
    let place = curve_place(cell, state_of_charge);
    curve_at(&cell.rest_v_new_40c, place)
        + age * curve_at(&cell.rest_v_per_capacity_lost, place)
        + (case_temperature_c - cell.curve_temperature_c) * curve_at(&cell.rest_v_per_k, place)
}

/// Enthalpy voltage at point `index` of the curves, for an age.
fn enthalpy_v_at_point(cell: &Cell2170Model3, age: f64, index: usize) -> f64 {
    cell.rest_v_new_40c[index] + age * cell.rest_v_per_capacity_lost[index]
        - (cell.curve_temperature_c + cell.kelvin_offset) * cell.rest_v_per_k[index]
}

/// Integral of the enthalpy voltage from the first point of the curves to a
/// state of charge, in volts. Exact for straight lines: a trapezoid for each
/// whole interval and one for the part of the last.
fn enthalpy_integral_v(cell: &Cell2170Model3, age: f64, state_of_charge: f64) -> f64 {
    let (index, share) = curve_place(cell, state_of_charge);
    let mut sum = 0.0;
    let mut point = 0;
    while point < index {
        sum += 0.5
            * (enthalpy_v_at_point(cell, age, point) + enthalpy_v_at_point(cell, age, point + 1))
            * cell.curve_spacing;
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

fn trial_is_finite(trial: &Trial) -> bool {
    let mut mode = 0;
    while mode < DIFFUSION_MODES {
        if !trial.diffusion_lag[mode].is_finite() {
            return false;
        }
        mode += 1;
    }
    trial.state_of_charge.is_finite()
        && trial.surface_state_of_charge.is_finite()
        && trial.reaction_overpotential_v.is_finite()
        && trial.near_full_overpotential_v.is_finite()
        && trial.hysteresis_v.is_finite()
        && trial.electrolyte_v.is_finite()
        && trial.current_mean_a.is_finite()
        && trial.resistance_at_once_ohm.is_finite()
        && trial.rest_v.is_finite()
        && trial.terminal_v.is_finite()
        && trial.chemical_step_j.is_finite()
}

fn state_is_finite(state: &CellState) -> bool {
    let mut mode = 0;
    while mode < DIFFUSION_MODES {
        if !state.diffusion_lag[mode].is_finite() {
            return false;
        }
        mode += 1;
    }
    state.t_s.is_finite()
        && state.cell_number.is_finite()
        && state.capacity_ah.is_finite()
        && state.state_of_charge.is_finite()
        && state.reaction_overpotential_v.is_finite()
        && state.near_full_overpotential_v.is_finite()
        && state.hysteresis_v.is_finite()
        && state.electrolyte_v.is_finite()
        && state.current_mean_a.is_finite()
        && state.surface_state_of_charge.is_finite()
        && state.charge_removed_ah.is_finite()
        && state.current_a.is_finite()
        && state.case_temperature_c.is_finite()
        && state.case_temperature_least_c.is_finite()
        && state.case_temperature_most_c.is_finite()
        && state.terminal_v.is_finite()
        && state.rest_v.is_finite()
        && state.resistance_at_once_ohm.is_finite()
        && state.chemical_energy_j.is_finite()
        && state.energy_anchor_j.is_finite()
        && state.electrical_work_j.is_finite()
        && state.heat_j.is_finite()
        && state.energy_defect_j.is_finite()
        && state.residual_conservation_error.is_finite()
}

#[cfg(test)]
mod tests {
    extern crate core;
    extern crate std;

    use core::prelude::v1::*;
    use std::assert;
    use std::assert_eq;
    use std::panic;
    use std::vec::Vec;

    use super::{
        CAPACITY_MAX_AH, CAPACITY_MIN_AH, CASE_TEMPERATURE_MAX_C, CASE_TEMPERATURE_MIN_C,
        CELL_COUNT, CELLS, CHAMBER_ACCURACY_K, CHARGE_CURRENT_MAX_A, CURRENT_MEAN_TIME_S,
        CURVE_FIRST_STATE_OF_CHARGE, CURVE_LAST_STATE_OF_CHARGE, CURVE_POINTS, CURVE_SPACING,
        CURVE_TEMPERATURE_C, Cell2170Model3, CellLoad, CellState, DIFFUSION_ACTIVATION_K,
        DIFFUSION_AT_ONCE_WEIGHT, DIFFUSION_MODE_TIME_FRACTION, DIFFUSION_MODE_WEIGHT,
        DIFFUSION_MODES, DIFFUSION_TIME_S, DISCHARGE_CURRENT_MAX_A, DISCHARGE_SLOPE_FACTOR,
        DOUBLE_LAYER_F, ELECTROLYTE_ACTIVATION_K, ELECTROLYTE_OHM, ELECTROLYTE_TIME_S,
        EXCHANGE_CURRENT_A, EXCHANGE_CURRENT_ACTIVATION_K, FITTED_NUMBERS, FLAG_SUSTAINED_ABOVE_1C,
        FLOOR_DT_S, GAS_OVER_FARADAY_V_PER_K, GATE_DT_S, HYSTERESIS_EMPTY_RANGE,
        HYSTERESIS_EMPTY_V, HYSTERESIS_FADE_S, HYSTERESIS_FULL_ACTIVATION_K, HYSTERESIS_FULL_RANGE,
        HYSTERESIS_FULL_V, HYSTERESIS_RATE, KELVIN_OFFSET, LOWER_VOLTAGE_LIMIT_V,
        NEAR_FULL_ACTIVATION_K, NEAR_FULL_CHARGING_FACTOR, NEAR_FULL_EXCHANGE_CURRENT_A,
        NEAR_FULL_RANGE, OVERPOTENTIAL_RATIO_LIMIT, REFERENCE_TEMPERATURE_K,
        RESIDUAL_CONSERVATION_LIMIT, RESISTANCE_COLD_ACTIVATION_K, RESISTANCE_COLD_OHM,
        RESISTANCE_FIXED_OHM, REST_V_NEW_40C, REST_V_PER_CAPACITY_LOST, REST_V_PER_K,
        SECONDS_PER_HOUR, STATE_OF_CHARGE_MAX, STATE_OF_CHARGE_MIN, SUSTAINED_CHARGE_MAX_A,
        BLOCK_CAPACITY_LEAST_AH, BLOCK_CAPACITY_MOST_AH, BLOCK_CASE_TEMPERATURE_LEAST_C,
        BLOCK_CASE_TEMPERATURE_MOST_C, BLOCK_CHAMBER_C, BLOCK_COUNT, CASE_TEMPERATURE_CHANGE_MAX_K,
        CASE_TEMPERATURE_SPAN_MAX_K, STATE_OF_CHARGE_ALLOWANCE_EXTRA_STEPS,
        STATE_OF_CHARGE_ALLOWANCE_PER_STEP,
        SUSTAINED_DISCHARGE_MAX_A, SUSTAINED_FLAG_A, StepReject, TESTER_CURRENT_ACCURACY_A, Trial,
        UPPER_VOLTAGE_LIMIT_V,
        abs_f64, commit, enthalpy_integral_v, integrate, rest_v_at, step, validate,
    };
    use crate::frame::LastStateFrame64;

    ::core::include!("cell_2170_model3_reference.rs");

    /// Largest difference from the Python model the tests allow in a voltage
    /// or in the running mean of the current, in volts and amperes. The
    /// largest found anywhere is 2.8e-17 V: one unit in the last place of an
    /// overpotential of a few hundredths of a volt.
    const VOLT_AGREEMENT: f64 = 1.0e-15;
    /// The same for a state of charge and a diffusion mode. Largest found: 1.4e-17.
    const CHARGE_AGREEMENT: f64 = 1.0e-15;
    /// The same for work and heat, as a share of the value. Largest found:
    /// 2.7e-16. The Python adds the five parts of a step's heat with a
    /// compensated sum and the card adds them in order.
    const ENERGY_AGREEMENT: f64 = 1.0e-14;

    fn blank_frame() -> LastStateFrame64 {
        LastStateFrame64 {
            t: -1.0,
            pos: [1.0, 2.0, 3.0],
            vel: [4.0, 5.0, 6.0],
            force_torque: 7.0,
            residual: 8.0,
            flags: 9,
            proof: [0xA5; 16],
        }
    }

    fn proof_intact(frame: &LastStateFrame64) {
        let mut index = 0;
        while index < 16 {
            assert_eq!(frame.proof[index], 0xA5, "proof byte {index} was resealed");
            index += 1;
        }
    }

    fn assert_copy<T: Copy>() {}

    fn state_bits(state: &CellState) -> Vec<u64> {
        let mut bits = Vec::new();
        for value in [
            state.t_s,
            state.cell_number,
            state.capacity_ah,
            state.state_of_charge,
            state.reaction_overpotential_v,
            state.near_full_overpotential_v,
            state.hysteresis_v,
            state.electrolyte_v,
            state.current_mean_a,
            state.surface_state_of_charge,
            state.charge_removed_ah,
            state.current_a,
            state.case_temperature_c,
            state.case_temperature_least_c,
            state.case_temperature_most_c,
            state.terminal_v,
            state.rest_v,
            state.resistance_at_once_ohm,
            state.chemical_energy_j,
            state.energy_anchor_j,
            state.electrical_work_j,
            state.heat_j,
            state.energy_defect_j,
            state.residual_conservation_error,
        ] {
            bits.push(value.to_bits());
        }
        for value in state.diffusion_lag {
            bits.push(value.to_bits());
        }
        bits.push(state.sustained_above_1c as u64);
        bits
    }

    fn load(current_a: f64, case_temperature_c: f64) -> CellLoad {
        CellLoad {
            current_a,
            case_temperature_c,
        }
    }

    fn rested(cell: &Cell2170Model3, state_of_charge: f64, capacity_ah: f64) -> CellState {
        CellState::try_rested(cell, 80.0, state_of_charge, capacity_ah).expect("a rested cell")
    }

    /// The card's step without its pairing of capacity with case
    /// temperature. The reference cases of the specification were chosen
    /// before that pairing was set, and A, C and the reference discharge sit
    /// at a capacity no block of tests measured at their case temperature
    /// (4.6768 Ah at 25 degC, 4.5 Ah at 10 degC). Their equations are checked
    /// through this; the card's own step refuses those starts by name.
    fn step_equations(
        cell: &Cell2170Model3,
        state: &mut CellState,
        frame: &mut LastStateFrame64,
        load: CellLoad,
        dt_s: f64,
    ) -> Result<(), StepReject> {
        validate(cell, state, load, dt_s)?;
        let trial = integrate(cell, state, load, dt_s)?;
        commit(cell, state, frame, load, dt_s, trial)
    }

    fn next_up(value: f64) -> f64 {
        if value > 0.0 {
            f64::from_bits(value.to_bits() + 1)
        } else if value < 0.0 {
            f64::from_bits(value.to_bits() - 1)
        } else {
            f64::from_bits(1)
        }
    }

    fn next_down(value: f64) -> f64 {
        -next_up(-value)
    }

    /// One (current, case temperature) for each one-second step of segments
    /// of (seconds, amperes, start degC, kelvin per second): the case
    /// temperature of step k of a segment is its start plus the slope times
    /// k, with k = 1 for the first step.
    fn series(segments: &[(usize, f64, f64, f64)]) -> Vec<(f64, f64)> {
        let mut out = Vec::new();
        for (seconds, current_a, start_c, slope) in segments {
            let mut k = 0;
            while k < *seconds {
                out.push((*current_a, *start_c + *slope * (k + 1) as f64));
                k += 1;
            }
        }
        out
    }

    const LOAD_A: [(usize, f64, f64, f64); 1] = [(3000, 4.5, 25.0, 0.0)];
    const LOAD_B: [(usize, f64, f64, f64); 1] = [(6000, 1.5, -5.0, 0.0)];
    const LOAD_C: [(usize, f64, f64, f64); 5] = [
        (60, 0.0, 10.0, 0.0),
        (10, 10.0, 10.0, 0.0),
        (120, 0.0, 10.0, 0.0),
        (60, -2.0, 10.0, 0.0),
        (300, 0.0, 10.0, 0.0),
    ];
    const LOAD_D: [(usize, f64, f64, f64); 4] = [
        (270, 7.0, 0.0, 0.02),
        (270, -1.0, 5.4, -0.01),
        (270, 7.0, 2.7, 0.02),
        (270, -1.0, 8.1, -0.01),
    ];
    const LOAD_SECOND_CELL: [(usize, f64, f64, f64); 5] = [
        (60, 0.0, 10.0, 0.0),
        (120, 9.0, 10.0, 0.01),
        (60, 0.0, 11.2, 0.0),
        (120, -2.0, 11.2, -0.01),
        (240, 4.5, 10.0, 0.0),
    ];

    /// The largest differences from the Python model found along a comparison.
    #[derive(Clone, Copy, Debug)]
    struct Worst {
        volt: f64,
        charge: f64,
        energy_share: f64,
        states: usize,
    }

    impl Worst {
        fn none() -> Self {
            Self {
                volt: 0.0,
                charge: 0.0,
                energy_share: 0.0,
                states: 0,
            }
        }

        fn volt(&mut self, got: f64, want: f64, what: &str, at: f64) {
            let off = abs_f64(got - want);
            assert!(off <= VOLT_AGREEMENT, "{what} at {at} s: {got:e} against {want:e}");
            if off > self.volt {
                self.volt = off;
            }
        }

        fn charge(&mut self, got: f64, want: f64, what: &str, at: f64) {
            let off = abs_f64(got - want);
            assert!(off <= CHARGE_AGREEMENT, "{what} at {at} s: {got:e} against {want:e}");
            if off > self.charge {
                self.charge = off;
            }
        }

        fn energy(&mut self, got: f64, want: f64, what: &str, at: f64) {
            let off = abs_f64(got - want);
            let share = if want == 0.0 { off } else { off / abs_f64(want) };
            assert!(share <= ENERGY_AGREEMENT, "{what} at {at} s: {got:e} against {want:e}");
            if share > self.energy_share {
                self.energy_share = share;
            }
        }
    }

    /// Every number of a reference state against the card's state.
    fn against(state: &CellState, want: &ReferenceState, worst: &mut Worst) {
        let at = want.t_s;
        assert_eq!(state.t_s, want.t_s);
        assert_eq!(state.current_a.to_bits(), want.current_a.to_bits(), "current at {at} s");
        assert_eq!(
            state.case_temperature_c.to_bits(),
            want.case_temperature_c.to_bits(),
            "case temperature at {at} s"
        );
        worst.volt(state.terminal_v, want.terminal_v, "terminal voltage", at);
        worst.volt(state.reaction_overpotential_v, want.reaction_overpotential_v, "reaction", at);
        worst.volt(state.near_full_overpotential_v, want.near_full_overpotential_v, "near-full", at);
        worst.volt(state.hysteresis_v, want.hysteresis_v, "hysteresis", at);
        worst.volt(state.electrolyte_v, want.electrolyte_v, "electrolyte", at);
        worst.volt(state.current_mean_a, want.current_mean_a, "running mean", at);
        worst.charge(state.state_of_charge, want.state_of_charge, "state of charge", at);
        worst.charge(state.surface_state_of_charge, want.surface_state_of_charge, "surface", at);
        worst.charge(state.diffusion_lag[0], want.diffusion_first, "first mode", at);
        worst.charge(
            state.diffusion_lag[DIFFUSION_MODES - 1],
            want.diffusion_last,
            "last mode",
            at,
        );
        worst.energy(state.electrical_work_j, want.work_j, "work", at);
        worst.energy(state.heat_j, want.heat_j, "heat", at);
        // The Python's residual is rounding, of either sign; so is the card's.
        assert!(abs_f64(want.residual) < 1.0e-13);
        assert!(state.residual_conservation_error < 1.0e-13, "residual at {at} s");
        worst.states += 1;
    }

    /// Step the card through a series and compare with every reference state
    /// whose time is reached. Returns the last state.
    fn follow(
        cell: &Cell2170Model3,
        mut state: CellState,
        loads: &[(f64, f64)],
        reference: &[ReferenceState],
        worst: &mut Worst,
    ) -> CellState {
        let mut frame = blank_frame();
        let mut seen = 0;
        for (index, (current_a, case_c)) in loads.iter().enumerate() {
            step_equations(cell, &mut state, &mut frame, load(*current_a, *case_c), 1.0)
                .unwrap_or_else(|reject| panic!("step {} refused: {reject:?}", index + 1));
            for want in reference {
                if want.t_s == (index + 1) as f64 {
                    against(&state, want, worst);
                    seen += 1;
                }
            }
        }
        assert_eq!(seen, reference.len(), "a reference time was not reached");
        proof_intact(&frame);
        state
    }

    // ---- a plain reading of the curves, written apart from the card's own ----

    fn plain_curve(values: &[f64; CURVE_POINTS], state_of_charge: f64) -> f64 {
        let mut index = 0;
        while index < CURVE_POINTS - 2 && -0.05 + 1.1 * (index + 1) as f64 / 210.0 <= state_of_charge {
            index += 1;
        }
        let low = -0.05 + 1.1 * index as f64 / 210.0;
        let high = -0.05 + 1.1 * (index + 1) as f64 / 210.0;
        values[index] + (values[index + 1] - values[index]) * (state_of_charge - low) / (high - low)
    }

    fn plain_rest_v(reference_capacity_ah: f64, capacity_ah: f64, z: f64, case_c: f64) -> f64 {
        plain_curve(&REST_V_NEW_40C, z)
            + (1.0 - capacity_ah / reference_capacity_ah) * plain_curve(&REST_V_PER_CAPACITY_LOST, z)
            + (case_c - 40.0) * plain_curve(&REST_V_PER_K, z)
    }

    // ---- constants --------------------------------------------------------------

    macro_rules! each_scalar {
        ($($field:ident),* $(,)?) => {
            [$((
                ::core::stringify!($field),
                (|cell: &mut Cell2170Model3| cell.$field = next_up(cell.$field))
                    as fn(&mut Cell2170Model3),
            )),*]
        };
    }

    #[test]
    fn constants_are_locked_and_a_caller_cannot_set_one() {
        let cell = Cell2170Model3::cell();
        assert!(cell.is_valid());
        assert_eq!(cell.gate_dt_s(), Some(1.0));
        assert_eq!(cell.floor_dt_s(), Some(1.0));
        assert_eq!(FLAG_SUSTAINED_ABOVE_1C, 1);
        assert_eq!(RESIDUAL_CONSERVATION_LIMIT, 1.0e-10);

        let scalars = each_scalar!(
            resistance_fixed_ohm,
            resistance_cold_ohm,
            resistance_cold_activation_k,
            exchange_current_a,
            exchange_current_activation_k,
            discharge_slope_factor,
            double_layer_f,
            near_full_exchange_current_a,
            near_full_activation_k,
            near_full_range,
            diffusion_time_s,
            diffusion_activation_k,
            hysteresis_full_v,
            hysteresis_full_range,
            hysteresis_full_activation_k,
            hysteresis_empty_v,
            hysteresis_empty_range,
            hysteresis_rate,
            hysteresis_fade_s,
            electrolyte_ohm,
            electrolyte_time_s,
            electrolyte_activation_k,
            reference_temperature_k,
            kelvin_offset,
            curve_temperature_c,
            gas_over_faraday_v_per_k,
            lower_voltage_limit_v,
            upper_voltage_limit_v,
            near_full_charging_factor,
            overpotential_ratio_limit,
            current_mean_time_s,
            diffusion_at_once_weight,
            curve_first_state_of_charge,
            curve_last_state_of_charge,
            curve_spacing,
            capacity_min_ah,
            capacity_max_ah,
            case_temperature_min_c,
            case_temperature_max_c,
            discharge_current_max_a,
            charge_current_max_a,
            sustained_discharge_max_a,
            sustained_charge_max_a,
            sustained_flag_a,
            state_of_charge_min,
            state_of_charge_max,
            gate_dt_s,
            floor_dt_s,
            residual_conservation_limit,
            state_of_charge_allowance_per_step,
            state_of_charge_allowance_extra_steps,
            case_temperature_change_max_k,
            case_temperature_span_max_k,
        );
        assert_eq!(scalars.len(), 53);
        let mut altered: Vec<(std::string::String, Cell2170Model3)> = Vec::new();
        for (name, change) in scalars {
            let mut other = cell;
            change(&mut other);
            altered.push((std::string::String::from(name), other));
        }
        let mut mode = 0;
        while mode < DIFFUSION_MODES {
            let mut other = cell;
            other.diffusion_mode_time_fraction[mode] = next_up(other.diffusion_mode_time_fraction[mode]);
            altered.push((std::format!("mode time {mode}"), other));
            let mut other = cell;
            other.diffusion_mode_weight[mode] = next_up(other.diffusion_mode_weight[mode]);
            altered.push((std::format!("mode weight {mode}"), other));
            mode += 1;
        }
        let mut point = 0;
        while point < CURVE_POINTS {
            let mut other = cell;
            other.rest_v_new_40c[point] = next_up(other.rest_v_new_40c[point]);
            altered.push((std::format!("rest voltage {point}"), other));
            let mut other = cell;
            other.rest_v_per_capacity_lost[point] = next_down(other.rest_v_per_capacity_lost[point]);
            altered.push((std::format!("change with capacity lost {point}"), other));
            let mut other = cell;
            other.rest_v_per_k[point] = next_up(other.rest_v_per_k[point]);
            altered.push((std::format!("change with temperature {point}"), other));
            point += 1;
        }
        let mut block = 0;
        while block < BLOCK_COUNT {
            let mut other = cell;
            other.block_chamber_c[block] = next_up(other.block_chamber_c[block]);
            altered.push((std::format!("chamber {block}"), other));
            let mut other = cell;
            other.block_case_temperature_least_c[block] =
                next_down(other.block_case_temperature_least_c[block]);
            altered.push((std::format!("least case {block}"), other));
            let mut other = cell;
            other.block_case_temperature_most_c[block] =
                next_up(other.block_case_temperature_most_c[block]);
            altered.push((std::format!("most case {block}"), other));
            let mut other = cell;
            other.block_capacity_least_ah[block] = next_down(other.block_capacity_least_ah[block]);
            altered.push((std::format!("least capacity {block}"), other));
            let mut other = cell;
            other.block_capacity_most_ah[block] = next_up(other.block_capacity_most_ah[block]);
            altered.push((std::format!("most capacity {block}"), other));
            block += 1;
        }
        let mut row = 0;
        while row < CELL_COUNT {
            let mut other = cell;
            other.cells[row].cell_number += 1.0;
            altered.push((std::format!("cell number {row}"), other));
            let mut other = cell;
            other.cells[row].reference_capacity_ah = next_up(other.cells[row].reference_capacity_ah);
            altered.push((std::format!("reference capacity {row}"), other));
            let mut other = cell;
            other.cells[row].resistance_scale = next_down(other.cells[row].resistance_scale);
            altered.push((std::format!("resistance scale {row}"), other));
            let mut other = cell;
            other.cells[row].charge_return_ratio = next_up(other.cells[row].charge_return_ratio);
            altered.push((std::format!("charge-return ratio {row}"), other));
            let mut other = cell;
            other.cells[row].name = "m448";
            altered.push((std::format!("name {row}"), other));
            row += 1;
        }
        assert_eq!(
            altered.len(),
            53 + 2 * DIFFUSION_MODES + 3 * CURVE_POINTS + 5 * BLOCK_COUNT + 5 * CELL_COUNT
        );

        let start = rested(&cell, 0.8, 4.5);
        for (name, other) in &altered {
            assert!(!other.is_valid(), "{name}");
            assert_eq!(other.gate_dt_s(), None, "{name}");
            assert_eq!(other.floor_dt_s(), None, "{name}");
            assert!(CellState::try_rested(other, 80.0, 0.8, 4.5).is_none(), "{name}");
            let mut state = start;
            let mut frame = blank_frame();
            let before = frame.to_bytes();
            assert_eq!(
                step(other, &mut state, &mut frame, load(1.0, 25.0), 1.0),
                Err(StepReject::InvalidCell),
                "{name}"
            );
            assert_eq!(state_bits(&state), state_bits(&start), "{name}");
            assert_eq!(frame.to_bytes(), before, "{name}");
        }
    }

    /// The card's constants are the numbers of the study's files, bit for bit.
    #[test]
    fn the_constants_are_the_numbers_of_the_study() {
        assert_eq!(FITTED_NUMBERS, 22);
        let fitted = [
            RESISTANCE_FIXED_OHM,
            RESISTANCE_COLD_OHM,
            RESISTANCE_COLD_ACTIVATION_K,
            EXCHANGE_CURRENT_A,
            EXCHANGE_CURRENT_ACTIVATION_K,
            DISCHARGE_SLOPE_FACTOR,
            DOUBLE_LAYER_F,
            NEAR_FULL_EXCHANGE_CURRENT_A,
            NEAR_FULL_ACTIVATION_K,
            NEAR_FULL_RANGE,
            DIFFUSION_TIME_S,
            DIFFUSION_ACTIVATION_K,
            HYSTERESIS_FULL_V,
            HYSTERESIS_FULL_RANGE,
            HYSTERESIS_FULL_ACTIVATION_K,
            HYSTERESIS_EMPTY_V,
            HYSTERESIS_EMPTY_RANGE,
            HYSTERESIS_RATE,
            HYSTERESIS_FADE_S,
            ELECTROLYTE_OHM,
            ELECTROLYTE_TIME_S,
            ELECTROLYTE_ACTIVATION_K,
        ];
        assert_eq!(fitted.len(), FITTED_NUMBERS);
        for (index, (card, study)) in fitted.iter().zip(STUDY_FITTED).enumerate() {
            assert_eq!(card.to_bits(), study.to_bits(), "fitted number {index}");
        }
        let fixed = [
            REFERENCE_TEMPERATURE_K,
            KELVIN_OFFSET,
            CURVE_TEMPERATURE_C,
            GAS_OVER_FARADAY_V_PER_K,
            LOWER_VOLTAGE_LIMIT_V,
            UPPER_VOLTAGE_LIMIT_V,
            NEAR_FULL_CHARGING_FACTOR,
            CELLS[0].reference_capacity_ah,
            DIFFUSION_AT_ONCE_WEIGHT,
        ];
        for (index, (card, study)) in fixed.iter().zip(STUDY_FIXED).enumerate() {
            assert_eq!(card.to_bits(), study.to_bits(), "fixed number {index}");
        }
        assert_eq!(GAS_OVER_FARADAY_V_PER_K.to_bits(), (8.314462618_f64 / 96485.33212).to_bits());
        for mode in 0..DIFFUSION_MODES {
            assert_eq!(
                DIFFUSION_MODE_TIME_FRACTION[mode].to_bits(),
                STUDY_MODE_TIME_FRACTION[mode].to_bits()
            );
            assert_eq!(DIFFUSION_MODE_WEIGHT[mode].to_bits(), STUDY_MODE_WEIGHT[mode].to_bits());
        }
        // The three cells, and the first is the one the 22 numbers were fitted on.
        assert_eq!(CELL_COUNT, 3);
        assert_eq!((CELLS[2].name, CELLS[2].cell_number), ("m448-N", 448.0));
        assert_eq!(CELLS[2].reference_capacity_ah.to_bits(), THIRD_CELL_NUMBERS[0].to_bits());
        assert_eq!(CELLS[2].charge_return_ratio.to_bits(), THIRD_CELL_NUMBERS[1].to_bits());
        assert_eq!(CELLS[2].resistance_scale.to_bits(), THIRD_CELL_NUMBERS[2].to_bits());
        // Its ratio times the charge put back after each capacity test is
        // the capacity its file holds for that block, and each is on the card.
        for (put_back_ah, capacity_ah) in THIRD_CELL_PUT_BACK_AND_CAPACITY_AH {
            assert_eq!((CELLS[2].charge_return_ratio * put_back_ah).to_bits(), capacity_ah.to_bits());
            assert!(capacity_ah >= CAPACITY_MIN_AH + 0.01 && capacity_ah <= CAPACITY_MAX_AH - 0.01);
        }
        // What its fitting files hold lies inside every limit: the largest
        // pulse, and the least and the most the case sensor read under load.
        let [pulse_a, case_min_c, case_max_c] = THIRD_CELL_FITTING_EXTREMES;
        assert!(pulse_a > 13.5 && pulse_a < DISCHARGE_CURRENT_MAX_A);
        assert!(case_min_c > CASE_TEMPERATURE_MIN_C && case_max_c < CASE_TEMPERATURE_MAX_C);
        assert_eq!((CELLS[0].name, CELLS[0].cell_number), ("m80", 80.0));
        assert_eq!(CELLS[0].resistance_scale, 1.0);
        assert_eq!((CELLS[1].name, CELLS[1].cell_number), ("m1000", 1000.0));
        assert_eq!(CELLS[1].resistance_scale.to_bits(), SECOND_CELL_RESISTANCE_SCALE.to_bits());
        assert_eq!(
            CELLS[1].reference_capacity_ah.to_bits(),
            SECOND_CELL_REFERENCE_CAPACITY_AH.to_bits()
        );
        assert_eq!(
            CELLS[1].charge_return_ratio.to_bits(),
            SECOND_CELL_CHARGE_RETURN_RATIO.to_bits()
        );
        // Each cell's ratio times the charge put back after each capacity
        // test is the capacity the study holds for that block.
        assert_eq!(STUDY_RANGE[10].to_bits(), CELLS[0].charge_return_ratio.to_bits());
        for (put_back_ah, capacity_ah) in SECOND_CELL_PUT_BACK_AND_CAPACITY_AH {
            assert_eq!((CELLS[1].charge_return_ratio * put_back_ah).to_bits(), capacity_ah.to_bits());
        }
        assert_eq!(
            SECOND_CELL_PUT_BACK_AND_CAPACITY_AH[0].1.to_bits(),
            CELLS[1].reference_capacity_ah.to_bits()
        );
        let mut seen = Vec::new();
        for row in CELLS {
            assert!(!seen.contains(&row.cell_number.to_bits()), "two cells with one number");
            seen.push(row.cell_number.to_bits());
            assert!(row.charge_return_ratio > 0.99 && row.charge_return_ratio < 1.0);
            assert!(row.resistance_scale > 0.8 && row.resistance_scale < 1.25);
            assert!(row.reference_capacity_ah >= CAPACITY_MIN_AH);
            assert!(row.reference_capacity_ah <= CAPACITY_MAX_AH);
        }

        // The first cell's extremes are the study's own (`fitted_range.json`).
        let [case_min, case_max, capacity_min, capacity_max, start_max, charge_peak, discharge_peak, mean_discharge, mean_charge, mean_rated, _ratio] =
            STUDY_RANGE;
        let first = SCORED_RANGE[0];
        assert_eq!(first.case_min_c.to_bits(), case_min.to_bits());
        assert_eq!(first.case_max_c.to_bits(), case_max.to_bits());
        assert_eq!(first.charge_peak_a.to_bits(), charge_peak.to_bits());
        assert_eq!(first.discharge_peak_a.to_bits(), discharge_peak.to_bits());
        assert_eq!(first.mean_discharge_a.to_bits(), mean_discharge.to_bits());
        assert_eq!(first.mean_charge_a.to_bits(), mean_charge.to_bits());
        assert!(SUSTAINED_FLAG_A > mean_rated && SUSTAINED_FLAG_A - mean_rated < 0.1);
        assert!(STATE_OF_CHARGE_MAX >= start_max && STATE_OF_CHARGE_MAX - start_max < 0.01);
        assert_eq!(STATE_OF_CHARGE_MIN, 0.0);

        // Every limit holds everything either cell was scored on, with the
        // stated allowance and no more than a rounding step beyond it.
        assert_eq!(TESTER_CURRENT_ACCURACY_A, 0.024);
        assert_eq!(CHAMBER_ACCURACY_K, 0.3);
        let most = |pick: fn(&ScoredRange) -> f64| pick(&SCORED_RANGE[0]).max(pick(&SCORED_RANGE[1]));
        let least = |pick: fn(&ScoredRange) -> f64| pick(&SCORED_RANGE[0]).min(pick(&SCORED_RANGE[1]));
        // The discharge limit stands on the programmed pulse, 13.5 A, and
        // every pulse either cell logged is within the tester's accuracy of it.
        assert_eq!(DISCHARGE_CURRENT_MAX_A, 13.5 + TESTER_CURRENT_ACCURACY_A);
        let pulse = most(|range| range.discharge_peak_a);
        assert!(pulse > 13.5 && pulse - 13.5 < 0.007, "{pulse}");
        for (limit, scored) in [
            (CHARGE_CURRENT_MAX_A, most(|range| range.charge_peak_a)),
            (SUSTAINED_DISCHARGE_MAX_A, most(|range| range.mean_discharge_a)),
            (SUSTAINED_CHARGE_MAX_A, most(|range| range.mean_charge_a)),
        ] {
            let margin = limit - scored;
            assert!(
                margin >= TESTER_CURRENT_ACCURACY_A && margin < TESTER_CURRENT_ACCURACY_A + 0.001,
                "{limit} against {scored}"
            );
        }
        // The largest charging current is the second cell's and the largest
        // sustained currents are the first cell's.
        assert!(SCORED_RANGE[1].charge_peak_a > SCORED_RANGE[0].charge_peak_a + 4.0);
        assert!(SCORED_RANGE[0].mean_discharge_a > SCORED_RANGE[1].mean_discharge_a + 4.0);
        let cold = CASE_TEMPERATURE_MIN_C - least(|range| range.case_min_c);
        assert!(cold <= -CHAMBER_ACCURACY_K && cold > -CHAMBER_ACCURACY_K - 0.1, "{cold}");
        let warm = CASE_TEMPERATURE_MAX_C - most(|range| range.case_max_c);
        assert!(warm >= CHAMBER_ACCURACY_K && warm < CHAMBER_ACCURACY_K + 0.1, "{warm}");
        let low = least(|range| range.capacity_min_ah).min(capacity_min) - CAPACITY_MIN_AH;
        assert!(low >= 0.01 && low < 0.02, "{low}");
        let high = CAPACITY_MAX_AH - most(|range| range.capacity_max_ah).max(capacity_max);
        assert!(high >= 0.01 && high < 0.02, "{high}");
        assert_eq!(SCORED_RANGE[0].records, 80);
        assert_eq!(SCORED_RANGE[1].records, 36);
        assert_eq!(CURRENT_MEAN_TIME_S, 60.0);
        assert_eq!(OVERPOTENTIAL_RATIO_LIMIT, 60.0);
        assert_eq!((GATE_DT_S, FLOOR_DT_S), (1.0, 1.0));
        assert_eq!(SECONDS_PER_HOUR, 3600.0);
    }

    /// What the scored runs of each block held, from -10 to 40 degC: the
    /// least and the most capacity of the 173 runs scored (65 of m80 and 15
    /// of m1000 by the study, 93 of m1000 and m448-N in its final check),
    /// with the block capacities of the three cells' fitting files, and the
    /// least and the most case temperature those runs sent to the card, read
    /// from the dataset's files on the one-second grid and from the final
    /// check's saved requests.
    const BLOCK_SCORED: [(f64, f64, f64, f64); BLOCK_COUNT] = [
        (4.1742, 4.2423, -12.449, 12.223),
        (4.217400705699121, 4.2874, 0.3786377142857127, 20.068),
        (4.184235273670695, 4.2899, 10.183, 26.47885650723025),
        (4.3669163015065875, 4.5130, 23.633, 31.071),
        (4.5725074365073946, 4.6871, 28.466, 45.535),
    ];
    /// The largest change of case temperature between two consecutive
    /// one-second samples of those runs (m80, 25 degC, REORDERED1), and the
    /// largest span inside one run (m1000, -10 degC, discharge at half the
    /// rated current).
    const SCORED_CASE_CHANGE_K: f64 = 5.022600000000072;
    const SCORED_CASE_SPAN_K: f64 = 18.8107;

    /// Each band is what its block scored with the stated allowance beyond
    /// it, to the stated rounding and no further; the blocks join without a
    /// gap from the card's least case temperature to its most; and the
    /// change and span limits are the scored values plus the chamber's
    /// accuracy, to a tenth above.
    #[test]
    fn the_bands_of_capacity_with_temperature_are_what_each_block_scored() {
        assert_eq!(BLOCK_CHAMBER_C, [-10.0, 0.0, 10.0, 25.0, 40.0]);
        for block in 0..BLOCK_COUNT {
            let (q_least, q_most, t_least, t_most) = BLOCK_SCORED[block];
            let low = q_least - BLOCK_CAPACITY_LEAST_AH[block];
            assert!(low >= 0.01 && low < 0.02 + 1.0e-12, "{block}: {low}");
            let high = BLOCK_CAPACITY_MOST_AH[block] - q_most;
            assert!(high >= 0.01 && high < 0.02 + 1.0e-12, "{block}: {high}");
            let warm = BLOCK_CASE_TEMPERATURE_MOST_C[block] - t_most;
            assert!(warm >= CHAMBER_ACCURACY_K && warm < CHAMBER_ACCURACY_K + 0.1, "{block}: {warm}");
            let cold = t_least - BLOCK_CASE_TEMPERATURE_LEAST_C[block];
            if block == 0 {
                // The -10 degC block reaches the card's least, read in a fitting file of m80.
                assert_eq!(BLOCK_CASE_TEMPERATURE_LEAST_C[0], CASE_TEMPERATURE_MIN_C);
                assert!(cold > CHAMBER_ACCURACY_K);
            } else {
                assert!(cold >= CHAMBER_ACCURACY_K && cold < CHAMBER_ACCURACY_K + 0.1, "{block}: {cold}");
                assert!(BLOCK_CASE_TEMPERATURE_LEAST_C[block] <= BLOCK_CASE_TEMPERATURE_MOST_C[block - 1]);
            }
            assert!(BLOCK_CAPACITY_LEAST_AH[block] >= CAPACITY_MIN_AH);
            assert!(BLOCK_CAPACITY_MOST_AH[block] <= CAPACITY_MAX_AH);
        }
        assert_eq!(BLOCK_CASE_TEMPERATURE_MOST_C[BLOCK_COUNT - 1], CASE_TEMPERATURE_MAX_C);
        assert_eq!(BLOCK_CAPACITY_LEAST_AH[0], CAPACITY_MIN_AH);
        assert_eq!(BLOCK_CAPACITY_MOST_AH[BLOCK_COUNT - 1], CAPACITY_MAX_AH);
        let change = CASE_TEMPERATURE_CHANGE_MAX_K - SCORED_CASE_CHANGE_K;
        assert!(change >= CHAMBER_ACCURACY_K && change < CHAMBER_ACCURACY_K + 0.1, "{change}");
        let span = CASE_TEMPERATURE_SPAN_MAX_K - SCORED_CASE_SPAN_K;
        assert!(span >= CHAMBER_ACCURACY_K && span < CHAMBER_ACCURACY_K + 0.1, "{span}");
        // The card's own block capacities, in its header, lie in their bands
        // at the chamber's setting.
        let cell = Cell2170Model3::cell();
        for (block, capacities) in [
            [4.2106, 4.2423, 4.2179],
            [4.2529, 4.2874, 4.2653],
            [4.2661, 4.2899, 4.2680],
            [4.5006, 4.5121, 4.4980],
            [4.6768, 4.6747, 4.6696],
        ]
        .iter()
        .enumerate()
        {
            for capacity_ah in capacities {
                assert!(cell.capacity_compared_at(*capacity_ah, BLOCK_CHAMBER_C[block]), "{block}");
            }
        }
        // A new cell in the cold and an aged cell when warm were never measured.
        assert!(!cell.capacity_compared_at(4.70, -12.9));
        assert!(!cell.capacity_compared_at(4.16, 45.9));
        assert!(!cell.capacity_compared_at(4.6768, 25.0));
        assert!(!cell.capacity_compared_at(4.5, 10.0));
        assert!(!cell.capacity_compared_at(4.31, 15.0));
        assert!(cell.capacity_compared_at(4.30, 15.0));
        // Where two blocks meet, either band is on the card and the gap between is not.
        assert!(cell.capacity_compared_at(4.30, 26.8));
        assert!(cell.capacity_compared_at(4.35, 23.3));
        assert!(!cell.capacity_compared_at(4.32, 25.0));
        assert!(!cell.capacity_compared_at(4.30, next_up(26.8)));
        assert!(!cell.capacity_compared_at(4.35, next_down(23.3)));
    }

    /// 211 points each on a grid from -0.05 to 1.05; the rest voltage never
    /// falls from one point to the next; and the cell's energy between empty
    /// and full is the Python model's for each capacity of the reference
    /// cases.
    #[test]
    fn the_curves_are_sane() {
        let cell = Cell2170Model3::cell();
        assert_eq!(CURVE_POINTS, 211);
        assert_eq!(REST_V_NEW_40C.len(), 211);
        assert_eq!(REST_V_PER_CAPACITY_LOST.len(), 211);
        assert_eq!(REST_V_PER_K.len(), 211);
        assert_eq!(CURVE_FIRST_STATE_OF_CHARGE, -0.05);
        assert_eq!(CURVE_LAST_STATE_OF_CHARGE, 1.05);
        assert!(abs_f64(CURVE_FIRST_STATE_OF_CHARGE + 210.0 * CURVE_SPACING - 1.05) < 1.0e-14);
        let mut point = 1;
        while point < CURVE_POINTS {
            assert!(REST_V_NEW_40C[point] >= REST_V_NEW_40C[point - 1], "point {point}");
            point += 1;
        }
        assert!(REST_V_NEW_40C[CURVE_POINTS - 1] > REST_V_NEW_40C[0] + 1.5);
        for value in REST_V_NEW_40C.iter().chain(&REST_V_PER_CAPACITY_LOST).chain(&REST_V_PER_K) {
            assert!(value.is_finite());
        }
        for (capacity_ah, want_j) in CELL_ENERGY_J {
            let age = 1.0 - capacity_ah / CELLS[0].reference_capacity_ah;
            let energy_j = SECONDS_PER_HOUR
                * capacity_ah
                * (enthalpy_integral_v(&cell, age, 1.0) - enthalpy_integral_v(&cell, age, 0.0));
            assert_eq!(energy_j.to_bits(), want_j.to_bits(), "{capacity_ah} Ah");
        }
        // As the study's check prints it for case A.
        assert!(abs_f64(CELL_ENERGY_J[0].1 - 62543.423219) < 1.0e-6);
        // The card's reading of the curves is the plain reading.
        let mut z = -0.05;
        while z <= 1.05 {
            for case_c in [-12.9, 0.0, 25.0, 45.9] {
                for capacity_ah in [4.16, 4.5, 4.70] {
                    let age = 1.0 - capacity_ah / CELLS[0].reference_capacity_ah;
                    let got = rest_v_at(&cell, age, z, case_c);
                    let want = plain_rest_v(CELLS[0].reference_capacity_ah, capacity_ah, z, case_c);
                    assert!(abs_f64(got - want) < 1.0e-11, "{z} {case_c} {capacity_ah}");
                    assert_eq!(cell.rest_v(0, capacity_ah, z, case_c), Some(got));
                }
            }
            z += 0.00317;
        }
        assert_eq!(cell.rest_v(0, 4.5, next_down(-0.05), 25.0), None);
        assert_eq!(cell.rest_v(0, 4.5, next_up(1.05), 25.0), None);
        assert_eq!(cell.rest_v(CELL_COUNT, 4.5, 0.5, 25.0), None);
        // At a point of the curve the reading is the point.
        assert!(abs_f64(rest_v_at(&cell, 0.0, 1.05, 40.0) - REST_V_NEW_40C[210]) < 1.0e-12);
        assert_eq!(rest_v_at(&cell, 0.0, -0.05, 40.0), REST_V_NEW_40C[0]);
    }

    #[test]
    fn a_start_outside_the_card_builds_no_state() {
        let cell = Cell2170Model3::cell();
        for cell_number in [80.0, 1000.0, 448.0] {
            let state = CellState::try_rested(&cell, cell_number, 1.0, 4.5).expect("a full cell");
            assert_eq!(state.cell_number, cell_number);
            assert_eq!(state.t_s, 0.0);
            assert_eq!(state.surface_state_of_charge, 1.0);
            assert_eq!(state.charge_removed_ah, 0.0);
            assert_eq!(state.energy_anchor_j, state.chemical_energy_j);
            assert_eq!(state.electrical_work_j, 0.0);
            assert_eq!(state.heat_j, 0.0);
            assert_eq!(state.diffusion_lag, [0.0; DIFFUSION_MODES]);
            assert!(!state.sustained_above_1c);
        }
        // The edges are on the card.
        for (z, capacity_ah) in [(0.0, 4.16), (1.02, 4.70), (0.0, 4.70), (1.02, 4.16)] {
            assert!(CellState::try_rested(&cell, 80.0, z, capacity_ah).is_some());
        }
        for cell_number in [0.0, 79.0, 81.0, 449.0, 447.5, 1000.5, -80.0, f64::NAN, f64::INFINITY] {
            assert!(CellState::try_rested(&cell, cell_number, 1.0, 4.5).is_none(), "{cell_number}");
        }
        for z in [next_down(0.0), next_up(1.02), -0.05, 1.05, f64::NAN, f64::INFINITY] {
            assert!(CellState::try_rested(&cell, 80.0, z, 4.5).is_none(), "{z}");
        }
        for capacity_ah in [next_down(4.16), next_up(4.70), 4.1, 4.75, 0.0, -4.5, f64::NAN] {
            assert!(CellState::try_rested(&cell, 80.0, 1.0, capacity_ah).is_none(), "{capacity_ah}");
        }
    }

    // ---- refusals ---------------------------------------------------------------

    /// A step that must be refused by this name, with the state and the
    /// frame left bit for bit as they were.
    fn refused(
        cell: &Cell2170Model3,
        start: &CellState,
        current_a: f64,
        case_c: f64,
        dt_s: f64,
        want: StepReject,
    ) {
        let mut state = *start;
        let mut frame = blank_frame();
        let before = frame.to_bytes();
        assert_eq!(
            step(cell, &mut state, &mut frame, load(current_a, case_c), dt_s),
            Err(want),
            "{current_a} A, {case_c} degC, {dt_s} s"
        );
        assert_eq!(state_bits(&state), state_bits(start), "{want:?}");
        assert_eq!(frame.to_bytes(), before, "{want:?}");
    }

    #[test]
    fn every_refusal_is_named_and_rolls_back_state_and_frame() {
        let cell = Cell2170Model3::cell();
        let start = rested(&cell, 0.8, 4.5);

        for (current_a, case_c, dt_s) in [
            (f64::NAN, 25.0, 1.0),
            (f64::INFINITY, 25.0, 1.0),
            (1.0, f64::NAN, 1.0),
            (1.0, f64::NEG_INFINITY, 1.0),
            (1.0, 25.0, f64::NAN),
            (1.0, 25.0, f64::INFINITY),
        ] {
            refused(&cell, &start, current_a, case_c, dt_s, StepReject::NonFiniteInput);
        }
        for dt_s in [0.0, -0.0, -1.0, -1.0e-300] {
            refused(&cell, &start, 1.0, 25.0, dt_s, StepReject::NonPositiveDt);
        }

        // A state that is not finite is named before the step is.
        let mut broken = start;
        broken.hysteresis_v = f64::NAN;
        refused(&cell, &broken, 1.0, 25.0, 1.0, StepReject::NonFiniteState);
        refused(&cell, &broken, 1.0, 25.0, 0.0, StepReject::NonFiniteState);
        let mut broken = start;
        broken.diffusion_lag[7] = f64::INFINITY;
        refused(&cell, &broken, 1.0, 25.0, 1.0, StepReject::NonFiniteState);
        // A step so short that the reaction's own solution divides by zero.
        let mut high = start;
        high.reaction_overpotential_v = 10.0;
        refused(&cell, &high, 1.0, 25.0, 1.0e-300, StepReject::NonFiniteState);

        // A cell the card does not hold.
        for cell_number in [449.0, 81.0, 0.0, -80.0] {
            let mut other = start;
            other.cell_number = cell_number;
            refused(&cell, &other, 1.0, 25.0, 1.0, StepReject::CellNotRecognized);
        }

        // The capacity.
        for capacity_ah in [4.1, 4.75, next_down(4.16), next_up(4.70)] {
            let mut other = start;
            other.capacity_ah = capacity_ah;
            refused(&cell, &other, 1.0, 25.0, 1.0, StepReject::CapacityOutsideCard);
        }

        // The case temperature.
        for case_c in [-13.0, 46.0, next_down(-12.9), next_up(45.9), -20.0, -273.15, 1000.0] {
            refused(&cell, &start, 1.0, case_c, 1.0, StepReject::CaseTemperatureOutsideCard);
        }

        // The current, each way.
        for current_a in [13.6, -14.0, next_up(13.524), next_down(-13.984), 100.0, -13.99] {
            refused(&cell, &start, current_a, 25.0, 1.0, StepReject::CurrentOutsideCard);
        }

        // The state of charge, incoming and landing: past either end by more
        // than the allowance of a first step, three spacings of 2^-52.
        let allowance = 3.0 * STATE_OF_CHARGE_ALLOWANCE_PER_STEP;
        for z in [next_down(-allowance), next_up(1.02 + allowance), -0.01, 1.03] {
            let mut other = start;
            other.state_of_charge = z;
            refused(&cell, &other, 0.0, 25.0, 1.0, StepReject::StateOfChargeOutsideCard);
        }
        let empty = rested(&cell, 0.0, 4.5);
        refused(&cell, &empty, 0.1, 25.0, 1.0, StepReject::StateOfChargeOutsideCard);
        let brim = rested(&cell, 1.02, 4.5);
        refused(&cell, &brim, -0.1, 25.0, 1.0, StepReject::StateOfChargeOutsideCard);

        // The surface off either end of the curves. The mean is on the card;
        // the lag of the surface carries it off.
        let mut lagging = rested(&cell, 0.1, 4.5);
        lagging.diffusion_lag[0] = 0.2;
        refused(&cell, &lagging, 0.0, 25.0, 1.0, StepReject::SurfaceOutsideCurve);
        let mut leading = rested(&cell, 0.95, 4.5);
        leading.diffusion_lag[0] = -0.2;
        refused(&cell, &leading, 0.0, 25.0, 1.0, StepReject::SurfaceOutsideCurve);

        // The voltage limits: a discharge at the bottom and a charge at the top.
        let low = rested(&cell, 0.02, 4.25);
        refused(&cell, &low, 10.0, 0.0, 1.0, StepReject::UndervoltageLimit);
        let full = rested(&cell, 1.0, 4.5);
        refused(&cell, &full, -5.0, 25.0, 1.0, StepReject::OvervoltageLimit);

        // A capacity outside the band compared at the case temperature.
        for (capacity_ah, case_c) in [(4.70, -12.9), (4.16, 45.9), (4.6768, 25.0), (4.5, 10.0), (4.32, 25.0)] {
            let mut other = start;
            other.capacity_ah = capacity_ah;
            refused(&cell, &other, 1.0, case_c, 1.0, StepReject::CapacityOutsideCardAtTemperature);
        }

        // The ledger, both ways: the work on the books is not the work done.
        let mut created = start;
        created.electrical_work_j += 1.0;
        refused(&cell, &created, 1.0, 25.0, 1.0, StepReject::SpontaneousGeneration);
        let mut destroyed = start;
        destroyed.electrical_work_j -= 1.0;
        refused(&cell, &destroyed, 1.0, 25.0, 1.0, StepReject::FirstLawBreach);

        // The order of the names, where two limits are passed at once.
        let mut both = start;
        both.capacity_ah = 4.75;
        refused(&cell, &both, 13.6, 46.0, 1.0, StepReject::CapacityOutsideCard);
        refused(&cell, &start, 13.6, 46.0, 1.0, StepReject::CaseTemperatureOutsideCard);
        both.cell_number = 449.0;
        refused(&cell, &both, 13.6, 46.0, 1.0, StepReject::CellNotRecognized);
        refused(&cell, &both, 13.6, 46.0, 0.0, StepReject::NonPositiveDt);
        refused(&cell, &both, f64::NAN, 46.0, 0.0, StepReject::NonFiniteInput);
        // The range of each input is named before the pairing of two.
        let mut aged = start;
        aged.capacity_ah = 4.16;
        refused(&cell, &aged, 13.6, 45.9, 1.0, StepReject::CurrentOutsideCard);
        let mut mean_and_charge = rested(&cell, 0.0, 4.5);
        mean_and_charge.current_mean_a = 9.0;
        refused(&cell, &mean_and_charge, 9.0, 25.0, 1.0, StepReject::SustainedCurrentOutsideCard);
    }

    /// The edges of the declared range are on the card.
    #[test]
    fn a_step_at_each_edge_of_the_range_lands() {
        let cell = Cell2170Model3::cell();
        for (z, capacity_ah, current_a, case_c) in [
            (0.8, 4.16, 1.0, -12.9),
            (0.8, 4.26, 1.0, -12.9),
            (0.8, 4.56, 1.0, 45.9),
            (0.8, 4.70, 1.0, 45.9),
            (0.8, 4.5, 13.524, 25.0),
            (0.5, 4.5, -13.984, 25.0),
            // The largest pulses the two cells logged, a few milliamperes
            // over the programmed 13.5 A, and the second cell's largest
            // charging current: none of them ends a run.
            (0.8, 4.5, 13.506, 25.0),
            (0.8, 4.5, 13.50649, 25.0),
            (0.5, 4.5, -13.959553446553446, 25.0),
            (0.8, 4.16, 1.0, -10.0),
            (0.8, 4.70, 1.0, 40.0),
            (1.02, 4.5, 1.0, 25.0),
            (0.2, 4.5, 0.0, 25.0),
        ] {
            let mut state = rested(&cell, z, capacity_ah);
            let mut frame = blank_frame();
            assert_eq!(
                step(&cell, &mut state, &mut frame, load(current_a, case_c), 1.0),
                Ok(()),
                "{z} {capacity_ah} {current_a} {case_c}"
            );
            assert_eq!(frame.t, 1.0);
            assert_eq!(frame.flags, 0);
            proof_intact(&frame);
        }
        // Each corner of each block's band.
        for block in 0..BLOCK_COUNT {
            for capacity_ah in [BLOCK_CAPACITY_LEAST_AH[block], BLOCK_CAPACITY_MOST_AH[block]] {
                for case_c in [BLOCK_CASE_TEMPERATURE_LEAST_C[block], BLOCK_CASE_TEMPERATURE_MOST_C[block]] {
                    let mut state = rested(&cell, 0.8, capacity_ah);
                    let mut frame = blank_frame();
                    assert_eq!(
                        step(&cell, &mut state, &mut frame, load(1.0, case_c), 1.0),
                        Ok(()),
                        "{block} {capacity_ah} {case_c}"
                    );
                }
            }
        }
    }

    /// A discharge held at 9 A lands until the running mean of the current
    /// passes 8.722 A, and is flagged from the step on which it passes 4.6 A.
    #[test]
    fn a_held_nine_amperes_is_flagged_and_then_refused_when_the_running_mean_passes_its_limit() {
        let cell = Cell2170Model3::cell();
        let mut state = rested(&cell, 1.0, 4.5006);
        let mut frame = blank_frame();
        let mut landed = 0;
        let mut first_flagged = 0;
        let reject = loop {
            let before = state;
            let frame_before = frame.to_bytes();
            match step(&cell, &mut state, &mut frame, load(9.0, 25.0), 1.0) {
                Ok(()) => {
                    landed += 1;
                    let plain = 9.0 * (1.0 - (-(landed as f64) / 60.0).exp());
                    assert!(abs_f64(state.current_mean_a - plain) < 1.0e-12);
                    assert_eq!(frame.force_torque, state.current_mean_a as f32);
                    assert_eq!(state.sustained_above_1c, state.current_mean_a > 4.6);
                    assert_eq!(
                        frame.flags,
                        if state.sustained_above_1c { FLAG_SUSTAINED_ABOVE_1C } else { 0 }
                    );
                    if state.sustained_above_1c && first_flagged == 0 {
                        first_flagged = landed;
                    }
                    assert!(state.current_mean_a <= 8.722);
                }
                Err(reject) => {
                    assert_eq!(state_bits(&state), state_bits(&before));
                    assert_eq!(frame.to_bytes(), frame_before);
                    break reject;
                }
            }
            assert!(landed < 1000);
        };
        assert_eq!(reject, StepReject::SustainedCurrentOutsideCard);
        // 9 (1 - exp(-t / 60)) passes 4.6 at 42.9 s and 8.722 at 208.6 s.
        assert_eq!(first_flagged, 43);
        assert_eq!(landed, 208);
        proof_intact(&frame);

        // Charging: a held 3.2 A of charge passes 2.993 A at 164.3 s.
        let mut state = rested(&cell, 0.3, 4.5006);
        let mut landed = 0;
        let reject = loop {
            match step(&cell, &mut state, &mut frame, load(-3.2, 25.0), 1.0) {
                Ok(()) => {
                    landed += 1;
                    assert_eq!(frame.flags, 0);
                }
                Err(reject) => break reject,
            }
            assert!(landed < 1000);
        };
        assert_eq!(reject, StepReject::SustainedCurrentOutsideCard);
        assert_eq!(landed, 164);
        // A discharge at the rated current never reaches the flag.
        let mut state = rested(&cell, 1.0, 4.5006);
        for _ in 0..1000 {
            step(&cell, &mut state, &mut frame, load(4.5, 25.0), 1.0).unwrap();
            assert_eq!(frame.flags, 0);
        }
        assert!(state.current_mean_a > 4.4999 && state.current_mean_a <= 4.5);
    }

    // ---- the ends of the state of charge ----------------------------------------

    /// A run or a single step whose end in decimal is exactly 1.02 or 0
    /// lands its last step: the state of charge is a running sum, and its
    /// comparison with each end carries the published rounding allowance. The
    /// value is not pulled back. A step that would end farther than the
    /// allowance past either end is refused by name.
    #[test]
    fn a_run_or_a_single_step_ending_exactly_on_either_end_of_the_charge_lands() {
        let cell = Cell2170Model3::cell();
        // 160 s at 0.225 A of charge from 1.0177777777777777 of 4.5 Ah ends in
        // decimal on 1.02. Before the allowance this run lost its last step.
        let mut state = rested(&cell, 1.0177777777777777, 4.5);
        let mut frame = blank_frame();
        for index in 0..160 {
            assert_eq!(
                step(&cell, &mut state, &mut frame, load(-0.225, 25.0), 1.0),
                Ok(()),
                "step {}",
                index + 1
            );
        }
        let past = state.state_of_charge - STATE_OF_CHARGE_MAX;
        std::println!("run to 1.02 ends at {:e} past it", past);
        assert!(abs_f64(past) <= STATE_OF_CHARGE_ALLOWANCE_PER_STEP * (160.0 + 2.0));
        assert!(past > 0.0, "{past:e}");
        // Restarted as a rested cell, that value is past the published end and builds no state.
        assert!(CellState::try_rested(&cell, 80.0, state.state_of_charge, 4.5).is_none());
        // One more step of charge is past the allowance.
        let before = state;
        assert_eq!(
            step(&cell, &mut state, &mut frame, load(-0.225, 25.0), 1.0),
            Err(StepReject::StateOfChargeOutsideCard)
        );
        assert_eq!(state_bits(&state), state_bits(&before));

        // A single first step that ends in decimal on 0: 1.215 A for 1 s from
        // 0.000075 of 4.5 Ah. In binary it lands 1.36e-20 below 0.
        let start = rested(&cell, 0.000075, 4.5);
        let landing = start.state_of_charge - 1.215 / (SECONDS_PER_HOUR * 4.5);
        assert!(landing < 0.0, "the sum must round past the end for this test");
        let mut state = start;
        let outcome = step(&cell, &mut state, &mut frame, load(1.215, 25.0), 1.0);
        std::println!("single step to 0: {outcome:?} at {:e}, {} V", state.state_of_charge, state.terminal_v);
        assert_eq!(outcome, Ok(()));
        assert!(abs_f64(state.state_of_charge) <= 3.0 * STATE_OF_CHARGE_ALLOWANCE_PER_STEP);
        // A second step of the same current is far past 0.
        assert_eq!(
            step(&cell, &mut state, &mut frame, load(1.215, 25.0), 1.0),
            Err(StepReject::StateOfChargeOutsideCard)
        );
        // The allowance grows with the steps of the run and stays far below
        // any physical amount: 2.2e-10 after 1,000,000 steps.
        assert_eq!(STATE_OF_CHARGE_ALLOWANCE_PER_STEP.to_bits(), (2.0_f64).powi(-52).to_bits());
        assert_eq!(STATE_OF_CHARGE_ALLOWANCE_EXTRA_STEPS, 2.0);
        assert!(STATE_OF_CHARGE_ALLOWANCE_PER_STEP * 1.0e6 < 2.3e-10);
    }

    // ---- case temperature through a run ----------------------------------------

    /// A change of case temperature between two steps of more than 5.4 K is
    /// refused by name, and so is a span since the rested start of more than
    /// 19.2 K. The first step has no last step to differ from.
    #[test]
    fn a_jump_or_a_span_of_case_temperature_beyond_what_was_scored_is_refused_by_name() {
        let cell = Cell2170Model3::cell();
        // 4.25 Ah is in the bands of the -10, 0 and 10 degC blocks: -12.9 to 26.8 degC.
        for first_c in [-12.9, 0.0, 26.8] {
            let mut state = rested(&cell, 0.8, 4.25);
            let mut frame = blank_frame();
            assert_eq!(step(&cell, &mut state, &mut frame, load(1.0, first_c), 1.0), Ok(()));
            assert_eq!(
                (state.case_temperature_least_c, state.case_temperature_most_c),
                (first_c, first_c)
            );
        }
        let mut state = rested(&cell, 0.8, 4.25);
        let mut frame = blank_frame();
        assert_eq!((state.case_temperature_least_c, state.case_temperature_most_c), (0.0, 0.0));
        step(&cell, &mut state, &mut frame, load(1.0, 10.0), 1.0).unwrap();
        refused(&cell, &state, 1.0, next_up(15.4), 1.0, StepReject::CaseTemperatureChangeAboveCard);
        refused(&cell, &state, 1.0, next_down(4.6), 1.0, StepReject::CaseTemperatureChangeAboveCard);
        refused(&cell, &state, 1.0, -12.9, 1.0, StepReject::CaseTemperatureChangeAboveCard);
        // The range of each input is named first.
        refused(&cell, &state, 1.0, 46.0, 1.0, StepReject::CaseTemperatureOutsideCard);
        step(&cell, &mut state, &mut frame, load(1.0, 15.4), 1.0).unwrap();
        // Up to 19.2 K above the least, in steps of at most 5.4 K.
        for case_c in [10.0, 4.6, 0.0, 5.4, 10.8, 16.2, 19.2] {
            step(&cell, &mut state, &mut frame, load(1.0, case_c), 1.0).unwrap();
        }
        assert_eq!((state.case_temperature_least_c, state.case_temperature_most_c), (0.0, 19.2));
        refused(&cell, &state, 1.0, next_up(19.2), 1.0, StepReject::CaseTemperatureSpanAboveCard);
        step(&cell, &mut state, &mut frame, load(1.0, 14.0), 1.0).unwrap();
        step(&cell, &mut state, &mut frame, load(1.0, 9.0), 1.0).unwrap();
        step(&cell, &mut state, &mut frame, load(1.0, 4.0), 1.0).unwrap();
        step(&cell, &mut state, &mut frame, load(1.0, 0.0), 1.0).unwrap();
        refused(&cell, &state, 1.0, -0.001, 1.0, StepReject::CaseTemperatureSpanAboveCard);
        // A run of 300 s at -12.9 degC, then 45.9 degC.
        let mut state = rested(&cell, 0.6, 4.16);
        for _ in 0..300 {
            step(&cell, &mut state, &mut frame, load(2.0, -12.9), 1.0).unwrap();
        }
        let mut aged = state;
        aged.capacity_ah = 4.60;
        refused(&cell, &aged, 2.0, 45.9, 1.0, StepReject::CaseTemperatureChangeAboveCard);
        // The scored extremes are on the card.
        assert!(5.022600000000072 <= CASE_TEMPERATURE_CHANGE_MAX_K);
        assert!(18.8107 <= CASE_TEMPERATURE_SPAN_MAX_K);
    }

    /// The span depends on every case temperature since the rested start, so a run continued
    /// from a returned state carries the least and the most case temperature of the run so far.
    /// A fresh start that states both equal to its first case temperature is the rested start,
    /// to the bit. A continuation that carries the span is refused where the whole run would
    /// pass 19.2 K; restarting without it would have evaded the limit. A carried span that is
    /// off the card, reversed or wider than 19.2 K is no start.
    #[test]
    fn a_continued_run_carries_its_case_temperature_span_and_cannot_evade_the_limit() {
        let cell = Cell2170Model3::cell();
        // A fresh start stating its first case temperature is the rested start.
        let plain = rested(&cell, 0.8, 4.25);
        assert!(!plain.case_span_known);
        let stated = plain.with_case_span(&cell, 10.0, 10.0).expect("a fresh start");
        assert!(stated.case_span_known);
        let (mut a, mut b) = (plain, stated);
        let (mut fa, mut fb) = (blank_frame(), blank_frame());
        for case_c in [10.0, 12.0, 14.0, 9.0] {
            step(&cell, &mut a, &mut fa, load(1.0, case_c), 1.0).unwrap();
            step(&cell, &mut b, &mut fb, load(1.0, case_c), 1.0).unwrap();
        }
        assert_eq!(a, b);
        assert_eq!(fa.to_bytes(), fb.to_bytes());
        // A run from 0 to 18 degC, then handed back.
        let mut state = rested(&cell, 0.8, 4.25);
        let mut frame = blank_frame();
        for case_c in [0.0, 4.5, 9.0, 13.5, 18.0] {
            step(&cell, &mut state, &mut frame, load(1.0, case_c), 1.0).unwrap();
        }
        assert_eq!((state.case_temperature_least_c, state.case_temperature_most_c), (0.0, 18.0));
        let returned = CellState::try_rested(&cell, 80.0, state.state_of_charge, 4.25).unwrap();
        // Without the carried span, 22 degC after a restart is inside its new span of 0 K.
        let mut evading = returned;
        assert_eq!(step(&cell, &mut evading, &mut blank_frame(), load(1.0, 22.0), 1.0), Ok(()));
        // Carrying it, the whole run would span 22 K, and the step is refused by name.
        let continued = returned.with_case_span(&cell, 0.0, 18.0).expect("a carried span");
        refused(&cell, &continued, 1.0, 22.0, 1.0, StepReject::CaseTemperatureSpanAboveCard);
        refused(&cell, &continued, 1.0, next_up(19.2), 1.0, StepReject::CaseTemperatureSpanAboveCard);
        refused(&cell, &continued, 1.0, -1.3, 1.0, StepReject::CaseTemperatureSpanAboveCard);
        // Inside the carried span's reach it lands, and the span goes on from there.
        let mut on = continued;
        assert_eq!(step(&cell, &mut on, &mut blank_frame(), load(1.0, 19.2), 1.0), Ok(()));
        assert_eq!((on.case_temperature_least_c, on.case_temperature_most_c), (0.0, 19.2));
        // A carried span off the card, reversed, or wider than the limit is no start.
        assert!(returned.with_case_span(&cell, next_down(CASE_TEMPERATURE_MIN_C), 0.0).is_none());
        assert!(returned.with_case_span(&cell, 40.0, next_up(CASE_TEMPERATURE_MAX_C)).is_none());
        assert!(returned.with_case_span(&cell, 10.0, 9.0).is_none());
        assert!(returned.with_case_span(&cell, 0.0, next_up(19.2)).is_none());
        assert!(returned.with_case_span(&cell, f64::NAN, 0.0).is_none());
        assert!(returned.with_case_span(&cell, 0.0, 19.2).is_some());
        assert!(returned.with_case_span(&cell, CASE_TEMPERATURE_MIN_C, CASE_TEMPERATURE_MIN_C).is_some());
        // A handed-in state whose carried span is reversed is refused by name.
        let mut forged = continued;
        forged.case_temperature_least_c = 18.0;
        forged.case_temperature_most_c = 0.0;
        refused(&cell, &forged, 1.0, 9.0, 1.0, StepReject::CaseTemperatureSpanAboveCard);
    }

    // ---- rest ---------------------------------------------------------------

    /// From the rested state a zero-current step of any allowed length leaves
    /// every memory at zero and the terminal voltage at the rest voltage.
    #[test]
    fn rest_holds() {
        let cell = Cell2170Model3::cell();
        for row in 0..CELL_COUNT {
            for dt_s in [0.1, 0.5, 1.0] {
                for z in [0.0, 0.05, 0.37, 0.6, 0.999, 1.0, 1.02] {
                    for (case_c, capacity_ah) in [
                        (-12.9, 4.16),
                        (-5.0, 4.26),
                        (10.0, 4.17),
                        (25.0, 4.5),
                        (25.0, 4.30),
                        (45.9, 4.70),
                        (45.9, 4.56),
                    ] {
                        {
                            let numbers = CELLS[row];
                            let start =
                                CellState::try_rested(&cell, numbers.cell_number, z, capacity_ah)
                                    .unwrap();
                            let mut state = start;
                            let mut frame = blank_frame();
                            for _ in 0..25 {
                                step(&cell, &mut state, &mut frame, load(0.0, case_c), dt_s).unwrap();
                            }
                            assert_eq!(state.state_of_charge, z);
                            assert_eq!(state.surface_state_of_charge, z);
                            assert_eq!(state.diffusion_lag, [0.0; DIFFUSION_MODES]);
                            assert_eq!(state.reaction_overpotential_v, 0.0);
                            assert_eq!(state.near_full_overpotential_v, 0.0);
                            assert_eq!(state.hysteresis_v, 0.0);
                            assert_eq!(state.electrolyte_v, 0.0);
                            assert_eq!(state.current_mean_a, 0.0);
                            assert_eq!(state.electrical_work_j, 0.0);
                            assert_eq!(state.heat_j, 0.0);
                            assert_eq!(state.energy_defect_j, 0.0);
                            assert_eq!(state.chemical_energy_j, start.energy_anchor_j);
                            assert_eq!(state.terminal_v, state.rest_v);
                            let plain =
                                plain_rest_v(numbers.reference_capacity_ah, capacity_ah, z, case_c);
                            assert!(abs_f64(state.terminal_v - plain) < 1.0e-11);
                            assert_eq!(frame.pos[0], state.terminal_v as f32);
                            assert_eq!(frame.flags, 0);
                        }
                    }
                }
            }
        }
    }

    // ---- the reference states ---------------------------------------------------

    /// The card reproduces every state the specification prints, on the same
    /// 211-point curves those states were made with: the thinned curves move
    /// no reference state by more than 1e-15 V. Largest differences found on
    /// the 17 states: 2.8e-17 V in a voltage, 1.4e-17 in a state of charge,
    /// 2.2e-16 of the value in work and heat. The card's `exp` and `log` are
    /// its own and the Python's are the platform's; on these runs they gave
    /// the same numbers but for a unit in the last place here and there. The
    /// study's 2,101-point curves are not in its files, so the card is not
    /// compared with them here; the study's own measure of the thinning is in
    /// the header.
    #[test]
    fn the_reference_states_of_the_specification_are_reproduced_on_the_211_point_curves() {
        let cell = Cell2170Model3::cell();
        let mut worst = Worst::none();
        follow(&cell, rested(&cell, 1.0, 4.6768), &series(&LOAD_A), &CASE_A, &mut worst);
        follow(&cell, rested(&cell, 1.0, 4.25), &series(&LOAD_B), &CASE_B, &mut worst);
        follow(&cell, rested(&cell, 0.6, 4.5), &series(&LOAD_C), &CASE_C, &mut worst);
        follow(&cell, rested(&cell, 0.9, 4.3), &series(&LOAD_D), &CASE_D, &mut worst);
        assert_eq!(worst.states, 17);
        std::println!("reference states: {worst:?}");
        assert!(worst.volt < 1.0e-16, "{worst:?}");
        assert!(worst.charge < 1.0e-16, "{worst:?}");
        assert!(worst.energy_share < 1.0e-15, "{worst:?}");
    }

    /// Case D across the reversal of the current after 270 s. The reaction's
    /// overpotential is still positive at 271 s and negative at 272 s: inside
    /// that step it would pass through zero, and the step is taken again from
    /// zero with the charging law.
    #[test]
    fn the_reaction_changes_sign_as_the_reference_does() {
        let cell = Cell2170Model3::cell();
        let mut worst = Worst::none();
        let loads = series(&LOAD_D);
        follow(&cell, rested(&cell, 0.9, 4.3), &loads[..273], &CASE_D_SIGN_CHANGE, &mut worst);
        assert_eq!(worst.states, 4);
        assert!(CASE_D_SIGN_CHANGE[1].reaction_overpotential_v > 0.02);
        assert!(CASE_D_SIGN_CHANGE[2].reaction_overpotential_v < -0.01);
        std::println!("sign change: {worst:?}");
        assert!(worst.volt < 1.0e-16, "{worst:?}");

        // The step taken again from zero is the charging law over the whole
        // step: the same answer as a step that starts at zero.
        let mut state = rested(&cell, 0.9, 4.3);
        let mut frame = blank_frame();
        for (current_a, case_c) in &loads[..271] {
            step(&cell, &mut state, &mut frame, load(*current_a, *case_c), 1.0).unwrap();
        }
        assert!(state.reaction_overpotential_v > 0.0);
        let mut from_zero = state;
        from_zero.reaction_overpotential_v = -0.0;
        let (current_a, case_c) = loads[271];
        step(&cell, &mut state, &mut frame, load(current_a, case_c), 1.0).unwrap();
        // With the incoming overpotential at zero and a charging current the
        // card goes straight to the charging law.
        let trial = integrate(&cell, &from_zero, load(current_a, case_c), 1.0).unwrap();
        assert!(state.reaction_overpotential_v < 0.0);
        assert_eq!(
            trial.reaction_overpotential_v.to_bits(),
            state.reaction_overpotential_v.to_bits()
        );
    }

    /// A discharge at the rated current from full, held until the card
    /// refuses: it lands the same 3,514 steps as the Python model and is
    /// refused at the lower voltage limit on the next, with the voltage of
    /// every 25th step and of the last ten agreeing. The reference is a new
    /// cell at 25 degC, which no block measured: the card's own step refuses
    /// it by name, and the equations are followed without that pairing.
    #[test]
    fn a_discharge_at_the_rated_current_ends_at_the_lower_limit_on_the_reference_step() {
        let cell = Cell2170Model3::cell();
        let mut state = rested(&cell, 1.0, 4.6768);
        let mut frame = blank_frame();
        let before = state;
        assert_eq!(
            step(&cell, &mut state, &mut frame, load(4.5, 25.0), 1.0),
            Err(StepReject::CapacityOutsideCardAtTemperature)
        );
        assert_eq!(state_bits(&state), state_bits(&before));
        let mut voltages = Vec::new();
        let mut largest_residual = 0.0;
        let reject = loop {
            match step_equations(&cell, &mut state, &mut frame, load(4.5, 25.0), 1.0) {
                Ok(()) => {
                    voltages.push(state.terminal_v);
                    if state.residual_conservation_error > largest_residual {
                        largest_residual = state.residual_conservation_error;
                    }
                    assert_eq!(frame.flags, 0);
                }
                Err(reject) => break reject,
            }
            assert!(voltages.len() < 5000);
        };
        assert_eq!(reject, StepReject::UndervoltageLimit);
        assert_eq!(voltages.len(), FULL_DISCHARGE_STEPS);
        assert_eq!(FULL_DISCHARGE_STEPS, 3514);
        assert!(FULL_DISCHARGE_REFUSED_V < 2.8);
        let mut worst = Worst::none();
        for (index, want) in FULL_DISCHARGE_V_EVERY_25.iter().enumerate() {
            let at = 25 * (index + 1);
            worst.volt(voltages[at - 1], *want, "terminal voltage", at as f64);
        }
        assert_eq!(FULL_DISCHARGE_V_EVERY_25.len(), 140);
        for (index, want) in FULL_DISCHARGE_V_LAST_10.iter().enumerate() {
            let at = FULL_DISCHARGE_STEPS - 10 + index + 1;
            worst.volt(voltages[at - 1], *want, "terminal voltage", at as f64);
        }
        against(&state, &FULL_DISCHARGE_END[0], &mut worst);
        std::println!("full discharge: {worst:?}, largest residual {largest_residual:e}");
        assert!(worst.volt < 1.0e-16, "{worst:?}");
        // The trial the card refused is the Python model's next voltage.
        assert!(state.terminal_v >= 2.8);
        assert!(largest_residual < 1.0e-13);
        // Delivered: 3,514 s at 4.5 A.
        assert!(abs_f64(state.charge_removed_ah - 4.5 * 3514.0 / 3600.0) < 1.0e-12);
        proof_intact(&frame);
    }

    /// The first 300 seconds of a logged drive cycle of the first cell, a
    /// different current and case temperature each second: every voltage
    /// against the Python model's.
    #[test]
    fn the_first_minutes_of_a_drive_cycle_follow_the_reference() {
        let cell = Cell2170Model3::cell();
        let mut state = rested(&cell, DRIVE_START_STATE_OF_CHARGE, DRIVE_CAPACITY_AH);
        let mut frame = blank_frame();
        let mut worst = Worst::none();
        let mut charging_steps = 0;
        for index in 0..DRIVE_CURRENT_A.len() {
            let current_a = DRIVE_CURRENT_A[index];
            step(
                &cell,
                &mut state,
                &mut frame,
                load(current_a, DRIVE_CASE_TEMPERATURE_C[index]),
                1.0,
            )
            .unwrap_or_else(|reject| panic!("step {} refused: {reject:?}", index + 1));
            worst.volt(state.terminal_v, DRIVE_TERMINAL_V[index], "terminal voltage", (index + 1) as f64);
            if current_a < 0.0 {
                charging_steps += 1;
            }
        }
        assert_eq!(DRIVE_CURRENT_A.len(), 300);
        assert!(charging_steps > 0, "the stretch has no charging in it");
        against(&state, &DRIVE_END[0], &mut worst);
        std::println!("drive cycle: {worst:?}, charging steps {charging_steps}");
        assert!(worst.volt < 1.0e-16, "{worst:?}");
        proof_intact(&frame);
    }

    /// The third cell: its own reference capacity and its one scale, against
    /// the Python model with those two numbers. It has the least resistance
    /// of the three.
    #[test]
    fn the_third_cell_carries_its_scale_as_the_study_applies_it() {
        let cell = Cell2170Model3::cell();
        let third = CellState::try_rested(&cell, 448.0, 0.95, 4.498).unwrap();
        let loads = series(&LOAD_SECOND_CELL);
        let mut worst = Worst::none();
        let end = follow(&cell, third, &loads, &THIRD_CELL, &mut worst);
        assert_eq!(worst.states, 6);
        std::println!("third cell: {worst:?}");
        assert!(worst.volt < 1.0e-16, "{worst:?}");
        let mut frame = blank_frame();
        let mut ends = Vec::new();
        for cell_number in [80.0, 1000.0] {
            let mut other = CellState::try_rested(&cell, cell_number, 0.95, 4.498).unwrap();
            for (current_a, case_c) in &loads {
                step_equations(&cell, &mut other, &mut frame, load(*current_a, *case_c), 1.0).unwrap();
            }
            ends.push(other);
        }
        assert!(end.resistance_at_once_ohm < ends[1].resistance_at_once_ohm);
        assert!(ends[1].resistance_at_once_ohm < ends[0].resistance_at_once_ohm);
        assert!(end.terminal_v > ends[1].terminal_v && ends[1].terminal_v > ends[0].terminal_v);
        assert_eq!(end.cell_number, 448.0);
    }

    /// The second cell: its own reference capacity, and its one scale applied
    /// as the study applies it. The same load on the first cell gives another
    /// voltage.
    #[test]
    fn the_second_cell_carries_its_scale_as_the_study_applies_it() {
        let cell = Cell2170Model3::cell();
        let second = CellState::try_rested(&cell, 1000.0, 0.95, 4.5121).unwrap();
        let loads = series(&LOAD_SECOND_CELL);
        let mut worst = Worst::none();
        let end = follow(&cell, second, &loads, &SECOND_CELL, &mut worst);
        assert_eq!(worst.states, 6);
        std::println!("second cell: {worst:?}");
        assert!(worst.volt < 1.0e-16, "{worst:?}");

        let mut first = rested(&cell, 0.95, 4.5121);
        let mut frame = blank_frame();
        for (current_a, case_c) in &loads {
            step_equations(&cell, &mut first, &mut frame, load(*current_a, *case_c), 1.0).unwrap();
        }
        // Less resistance: the second cell stands higher under the same discharge.
        assert!(end.terminal_v > first.terminal_v + 0.005);
        assert!(end.resistance_at_once_ohm < first.resistance_at_once_ohm);
        assert!(
            abs_f64(
                end.resistance_at_once_ohm / first.resistance_at_once_ohm
                    - SECOND_CELL_RESISTANCE_SCALE
            ) < 1.0e-14
        );
        // The charge counted is the same; the cell is not.
        assert_eq!(end.state_of_charge.to_bits(), first.state_of_charge.to_bits());
        assert_eq!(end.cell_number, 1000.0);
    }

    // ---- the ledger ---------------------------------------------------------------

    /// Over the two long reference cases, the pulses, the climbs and a full
    /// discharge the residual stays at rounding: below 1e-13 against a limit
    /// of 1e-10. At the end the books balance to the same rounding when added
    /// up by hand.
    #[test]
    fn the_ledger_closes_over_full_runs() {
        let cell = Cell2170Model3::cell();
        let mut largest = 0.0_f64;
        for (z, capacity_ah, segments) in [
            (1.0, 4.6768, &LOAD_A[..]),
            (1.0, 4.25, &LOAD_B[..]),
            (0.6, 4.5, &LOAD_C[..]),
            (0.9, 4.3, &LOAD_D[..]),
        ] {
            let mut state = rested(&cell, z, capacity_ah);
            let mut frame = blank_frame();
            for (current_a, case_c) in series(segments) {
                step_equations(&cell, &mut state, &mut frame, load(current_a, case_c), 1.0).unwrap();
                largest = largest.max(state.residual_conservation_error);
                assert_eq!(frame.residual, state.residual_conservation_error as f32);
            }
            let age = 1.0 - capacity_ah / CELLS[0].reference_capacity_ah;
            let energy_j = SECONDS_PER_HOUR
                * capacity_ah
                * (enthalpy_integral_v(&cell, age, 1.0) - enthalpy_integral_v(&cell, age, 0.0));
            let by_hand = state.chemical_energy_j - state.energy_anchor_j
                + state.electrical_work_j
                + state.heat_j;
            assert!(abs_f64(by_hand - state.energy_defect_j) < 1.0e-9);
            assert!(abs_f64(by_hand) / energy_j < 1.0e-13);
            assert_eq!(
                state.residual_conservation_error,
                abs_f64(state.energy_defect_j) / energy_j
            );
            // The chemical energy spent is the work and the heat.
            assert!(state.heat_j > 0.0);
            assert!(
                abs_f64(
                    (state.energy_anchor_j - state.chemical_energy_j)
                        - (state.electrical_work_j + state.heat_j)
                ) < 1.0e-9
            );
        }
        std::println!("largest residual over the four cases: {largest:e}");
        assert!(largest < 1.0e-13);
        assert!(largest < RESIDUAL_CONSERVATION_LIMIT * 1.0e-3);
    }

    /// The chemical energy of a step taken by the rectangle rule: the
    /// enthalpy voltage at the start of the step times the charge.
    fn rectangle_chemical_j(cell: &Cell2170Model3, state: &CellState, current_a: f64, dt_s: f64) -> f64 {
        let age = 1.0 - state.capacity_ah / CELLS[0].reference_capacity_ah;
        // The enthalpy voltage is the rest voltage read at 0 K: -273.15 degC.
        rest_v_at(cell, age, state.state_of_charge, -KELVIN_OFFSET) * current_a * dt_s
    }

    /// The wrong stepping of the chemical energy is named. With the rectangle
    /// rule in place of the exact integral the card refuses the first
    /// one-second step, and the residuals that rule would give are the
    /// study's: -6.672e-9 after one 1 s step at 4.5 A from full, -6.683e-7
    /// after one 10 s step, -2.428e-5 after 3,000 steps. The study's sign is
    /// the opposite of the card's defect.
    #[test]
    fn the_chemical_energy_by_the_rectangle_rule_is_refused_as_spontaneous_generation() {
        let cell = Cell2170Model3::cell();
        let energy_j = CELL_ENERGY_J[0].1;
        for (dt_s, want) in [(1.0, WRONG_RULE_RESIDUAL[0]), (10.0, WRONG_RULE_RESIDUAL[1])] {
            let start = rested(&cell, 1.0, 4.6768);
            let honest = integrate(&cell, &start, load(4.5, 25.0), dt_s).unwrap();
            let wrong = Trial {
                chemical_step_j: rectangle_chemical_j(&cell, &start, 4.5, dt_s),
                ..honest
            };
            // The residual the rule gives, by the study's formula.
            let work_j = 4.5 * honest.terminal_v * dt_s;
            let heat_j = wrong.chemical_step_j - work_j;
            let age = 1.0 - 4.6768 / CELLS[0].reference_capacity_ah;
            let landing_j =
                SECONDS_PER_HOUR * 4.6768 * enthalpy_integral_v(&cell, age, honest.state_of_charge);
            let residual = (start.energy_anchor_j - landing_j - work_j - heat_j) / energy_j;
            assert!(abs_f64(residual - want) < 2.0e-13, "{dt_s} s: {residual:e} against {want:e}");
            assert!(abs_f64(residual) > RESIDUAL_CONSERVATION_LIMIT);

            let mut state = start;
            let mut frame = blank_frame();
            let before = frame.to_bytes();
            assert_eq!(
                commit(&cell, &mut state, &mut frame, load(4.5, 25.0), dt_s, wrong),
                Err(StepReject::SpontaneousGeneration),
                "{dt_s} s"
            );
            assert_eq!(state_bits(&state), state_bits(&start));
            assert_eq!(frame.to_bytes(), before);
            // The honest trial of the same step lands.
            assert_eq!(commit(&cell, &mut state, &mut frame, load(4.5, 25.0), dt_s, honest), Ok(()));
            assert!(state.residual_conservation_error < 1.0e-15);
        }
        assert!(abs_f64(WRONG_RULE_RESIDUAL[0] + 6.672e-9) < 1.0e-12);
        assert!(abs_f64(WRONG_RULE_RESIDUAL[1] + 6.683e-7) < 1.0e-10);
        assert!(abs_f64(WRONG_RULE_RESIDUAL[2] + 2.428e-5) < 1.0e-8);

        // 3,000 steps of case A with the rule, added up beside the card.
        let mut state = rested(&cell, 1.0, 4.6768);
        let mut frame = blank_frame();
        let mut wrong_heat_j = 0.0;
        for _ in 0..3000 {
            let trial = integrate(&cell, &state, load(4.5, 25.0), 1.0).unwrap();
            let work_j = 4.5 * trial.terminal_v;
            wrong_heat_j += rectangle_chemical_j(&cell, &state, 4.5, 1.0) - work_j;
            step_equations(&cell, &mut state, &mut frame, load(4.5, 25.0), 1.0).unwrap();
        }
        let residual = (state.energy_anchor_j
            - state.chemical_energy_j
            - state.electrical_work_j
            - wrong_heat_j)
            / energy_j;
        assert!(
            abs_f64(residual - WRONG_RULE_RESIDUAL[2]) < 1.0e-11,
            "{residual:e} against {:e}",
            WRONG_RULE_RESIDUAL[2]
        );
    }

    /// A wrong stepping of the charge is named both ways. A step that books
    /// the work and the heat and removes no charge has made energy from
    /// nothing; one that removes the charge twice has destroyed energy.
    #[test]
    fn a_charge_step_that_removes_no_charge_or_removes_it_twice_is_refused_both_ways() {
        let cell = Cell2170Model3::cell();
        for (current_a, case_c) in [(4.5, 25.0), (1.0, -10.0), (-2.0, 10.0)] {
            let start = rested(&cell, 0.7, 4.5);
            let honest = integrate(&cell, &start, load(current_a, case_c), 1.0).unwrap();
            let none = Trial {
                state_of_charge: start.state_of_charge,
                ..honest
            };
            let twice = Trial {
                state_of_charge: start.state_of_charge
                    + 2.0 * (honest.state_of_charge - start.state_of_charge),
                ..honest
            };
            // Discharging, no charge removed is energy created; charging, it
            // is energy destroyed. Twice the charge is the other way about.
            let (for_none, for_twice) = if current_a > 0.0 {
                (StepReject::SpontaneousGeneration, StepReject::FirstLawBreach)
            } else {
                (StepReject::FirstLawBreach, StepReject::SpontaneousGeneration)
            };
            for (trial, want) in [(none, for_none), (twice, for_twice)] {
                let mut state = start;
                let mut frame = blank_frame();
                let before = frame.to_bytes();
                assert_eq!(
                    commit(&cell, &mut state, &mut frame, load(current_a, case_c), 1.0, trial),
                    Err(want),
                    "{current_a} A"
                );
                assert_eq!(state_bits(&state), state_bits(&start));
                assert_eq!(frame.to_bytes(), before);
            }
            // A voltage that is not the sum of its parts is named too: the
            // terminal voltage with the reaction's overpotential left out.
            let no_reaction = Trial {
                terminal_v: honest.terminal_v + honest.reaction_overpotential_v,
                ..honest
            };
            let mut state = start;
            let mut frame = blank_frame();
            assert_eq!(
                commit(&cell, &mut state, &mut frame, load(current_a, case_c), 1.0, no_reaction),
                Err(StepReject::SpontaneousGeneration)
            );
            assert_eq!(state_bits(&state), state_bits(&start));
        }
    }

    /// A trial that is off the card is refused by `commit` itself, whatever
    /// made it.
    #[test]
    fn a_trial_off_the_card_is_refused_by_name() {
        let cell = Cell2170Model3::cell();
        let start = rested(&cell, 0.7, 4.5);
        let honest = integrate(&cell, &start, load(1.0, 25.0), 1.0).unwrap();
        let cases = [
            (Trial { current_mean_a: 8.723, ..honest }, StepReject::SustainedCurrentOutsideCard),
            (Trial { current_mean_a: -2.994, ..honest }, StepReject::SustainedCurrentOutsideCard),
            (Trial { state_of_charge: -0.001, ..honest }, StepReject::StateOfChargeOutsideCard),
            (Trial { state_of_charge: 1.021, ..honest }, StepReject::StateOfChargeOutsideCard),
            (Trial { surface_state_of_charge: -0.051, ..honest }, StepReject::SurfaceOutsideCurve),
            (Trial { surface_state_of_charge: 1.051, ..honest }, StepReject::SurfaceOutsideCurve),
            (Trial { terminal_v: 2.79, ..honest }, StepReject::UndervoltageLimit),
            (Trial { terminal_v: 4.243, ..honest }, StepReject::OvervoltageLimit),
            (Trial { hysteresis_v: f64::NAN, ..honest }, StepReject::NonFiniteState),
            (Trial { chemical_step_j: f64::INFINITY, ..honest }, StepReject::NonFiniteState),
        ];
        for (trial, want) in cases {
            let mut state = start;
            let mut frame = blank_frame();
            let before = frame.to_bytes();
            assert_eq!(
                commit(&cell, &mut state, &mut frame, load(1.0, 25.0), 1.0, trial),
                Err(want)
            );
            assert_eq!(state_bits(&state), state_bits(&start));
            assert_eq!(frame.to_bytes(), before);
        }
        assert_eq!(validate(&cell, &start, load(1.0, 25.0), 1.0), Ok(()));
    }

    /// A falsified ledger rolls back: work, heat or the anchor moved by a
    /// joule between steps, each way, part way through a run.
    #[test]
    fn a_falsified_ledger_rolls_back() {
        let cell = Cell2170Model3::cell();
        let mut state = rested(&cell, 0.9, 4.25);
        let mut frame = blank_frame();
        for _ in 0..120 {
            step(&cell, &mut state, &mut frame, load(3.0, 10.0), 1.0).unwrap();
        }
        let honest = state;
        let frame_before = frame.to_bytes();
        let energy_j = state.electrical_work_j + state.heat_j;
        assert!(energy_j > 1000.0);
        // The limit is 1e-10 of some 60 kJ: six microjoules.
        for joules in [1.0, 1.0e-3, 1.0e-5] {
            for (which, sign, want) in [
                (0, 1.0, StepReject::SpontaneousGeneration),
                (0, -1.0, StepReject::FirstLawBreach),
                (1, 1.0, StepReject::SpontaneousGeneration),
                (1, -1.0, StepReject::FirstLawBreach),
                (2, 1.0, StepReject::FirstLawBreach),
                (2, -1.0, StepReject::SpontaneousGeneration),
            ] {
                let mut forged = honest;
                match which {
                    0 => forged.electrical_work_j += sign * joules,
                    1 => forged.heat_j += sign * joules,
                    _ => forged.energy_anchor_j += sign * joules,
                }
                let mut tried = forged;
                assert_eq!(
                    step(&cell, &mut tried, &mut frame, load(3.0, 10.0), 1.0),
                    Err(want),
                    "{which} {sign} {joules}"
                );
                assert_eq!(state_bits(&tried), state_bits(&forged));
                assert_eq!(frame.to_bytes(), frame_before);
            }
        }
        // A forged state of charge is named too: charge that was never removed.
        let mut forged = honest;
        forged.state_of_charge += 0.001;
        let mut tried = forged;
        assert_eq!(
            step(&cell, &mut tried, &mut frame, load(3.0, 10.0), 1.0),
            Err(StepReject::SpontaneousGeneration)
        );
        assert_eq!(state_bits(&tried), state_bits(&forged));
        // The honest state steps on.
        assert_eq!(step(&cell, &mut state, &mut frame, load(3.0, 10.0), 1.0), Ok(()));
        proof_intact(&frame);
    }

    /// What the ledger does not test, stated: a memory stepped by the wrong
    /// method is counted as heat and the books still balance. The reference
    /// states catch it. Here the electrolyte drop is stepped by explicit
    /// Euler; the step lands, and its voltage is off the reference by more
    /// than a hundred million times the agreement the tests hold.
    #[test]
    fn an_euler_step_of_a_memory_passes_the_ledger_and_fails_the_reference() {
        let cell = Cell2170Model3::cell();
        let start = rested(&cell, 1.0, 4.6768);
        let honest = integrate(&cell, &start, load(4.5, 25.0), 1.0).unwrap();
        // v' = v + dt / time * (settled - v), from rest, at 25 degC.
        let euler_v = 1.0 / ELECTROLYTE_TIME_S * (ELECTROLYTE_OHM * 4.5);
        let euler = Trial {
            electrolyte_v: euler_v,
            terminal_v: honest.terminal_v + honest.electrolyte_v - euler_v,
            ..honest
        };
        let mut state = start;
        let mut frame = blank_frame();
        assert_eq!(commit(&cell, &mut state, &mut frame, load(4.5, 25.0), 1.0, euler), Ok(()));
        assert!(state.residual_conservation_error < 1.0e-15);
        let off = abs_f64(state.terminal_v - CASE_A[0].terminal_v);
        assert!(off > 1.0e-6 && off < 1.0e-5, "{off:e}");
        assert!(off > 1.0e8 * VOLT_AGREEMENT);
    }

    // ---- the contract ---------------------------------------------------------------

    #[test]
    fn types_are_copy_and_the_step_is_core_only() {
        assert_copy::<Cell2170Model3>();
        assert_copy::<CellState>();
        assert_copy::<CellLoad>();
        assert_copy::<StepReject>();
        assert_copy::<Trial>();
        assert_copy::<super::CellNumbers>();
        let source = ::core::include_str!("cell_2170_model3.rs");
        let card = &source[..source.find("#[cfg(test)]").unwrap()];
        assert!(source.starts_with("#![no_implicit_prelude]"));
        assert!(!card.contains("std::"));
        assert!(!card.contains("Vec<"));
        assert!(!card.contains("Box<"));
        assert!(!card.contains(".powf("));
        assert!(!card.contains(".exp()"));
        assert!(!card.contains(".ln()"));
    }

    #[test]
    fn the_proof_is_not_the_cards() {
        let cell = Cell2170Model3::cell();
        let mut state = rested(&cell, 1.0, 4.5006);
        let mut frame = blank_frame();
        for index in 0..16 {
            frame.proof[index] = index as u8 ^ 0x5A;
        }
        let proof = frame.proof;
        for _ in 0..50 {
            step(&cell, &mut state, &mut frame, load(4.5, 25.0), 1.0).unwrap();
            assert_eq!(frame.proof, proof);
        }
        assert_eq!(
            step(&cell, &mut state, &mut frame, load(4.5, 60.0), 1.0),
            Err(StepReject::CaseTemperatureOutsideCard)
        );
        assert_eq!(frame.proof, proof);
        // What a landed step writes.
        step(&cell, &mut state, &mut frame, load(4.5, 25.0), 1.0).unwrap();
        assert_eq!(frame.t, 51.0);
        assert_eq!(frame.pos[0], state.terminal_v as f32);
        assert_eq!(frame.pos[1], state.charge_removed_ah as f32);
        assert_eq!(frame.pos[2], state.surface_state_of_charge as f32);
        assert_eq!(frame.vel[0], state.reaction_overpotential_v as f32);
        assert_eq!(frame.vel[1], state.hysteresis_v as f32);
        assert_eq!(frame.vel[2], state.electrolyte_v as f32);
        assert_eq!(frame.force_torque, state.current_mean_a as f32);
        assert_eq!(frame.residual, state.residual_conservation_error as f32);
        assert_eq!(frame.flags, 0);
        assert_eq!(frame.proof, proof);
        assert!(abs_f64(state.charge_removed_ah - 4.5 * 51.0 / 3600.0) < 1.0e-13);
        assert_eq!(state.capacity_ah, 4.5006);
    }

    #[test]
    fn repeated_runs_are_the_same_to_the_bit() {
        let cell = Cell2170Model3::cell();
        let mut ends = Vec::new();
        for _ in 0..2 {
            let mut state = rested(&cell, 0.9, 4.3);
            let mut frame = blank_frame();
            for (current_a, case_c) in series(&LOAD_D) {
                step(&cell, &mut state, &mut frame, load(current_a, case_c), 1.0).unwrap();
            }
            ends.push((state_bits(&state), frame.to_bytes()));
        }
        assert_eq!(ends[0], ends[1]);
    }

    /// The ceiling and the floor are both one second, locked, and `step`
    /// applies neither. For a held 4.5 A discharge at 25 degC, the one load
    /// checked, a shorter step gave the one-second answer to within 0.3 mV, as
    /// the study found (0.25 mV at 0.1 s and 0.10 mV at 0.5 s on case A). Where
    /// the current reverses it does not: the second of the reversal differs by
    /// 49 mV in the reversal that was checked, which is why nothing shorter
    /// than a second is served.
    #[test]
    fn the_step_is_one_second_because_a_shorter_step_changes_the_answer_where_the_current_reverses() {
        let cell = Cell2170Model3::cell();
        assert_eq!(cell.gate_dt_s(), Some(GATE_DT_S));
        assert_eq!(cell.floor_dt_s(), Some(FLOOR_DT_S));
        assert_eq!(GATE_DT_S, 1.0);
        assert_eq!(FLOOR_DT_S, 1.0);
        let mut one = rested(&cell, 1.0, 4.6768);
        let mut frame = blank_frame();
        let mut at_one = Vec::new();
        for _ in 0..3000 {
            step_equations(&cell, &mut one, &mut frame, load(4.5, 25.0), 1.0).unwrap();
            at_one.push(one.terminal_v);
        }
        for (dt_s, per_second, want_mv) in [(0.1, 10, 0.25), (0.5, 2, 0.10)] {
            let mut state = rested(&cell, 1.0, 4.6768);
            let mut largest = 0.0_f64;
            for second in 0..3000 {
                for _ in 0..per_second {
                    step_equations(&cell, &mut state, &mut frame, load(4.5, 25.0), dt_s).unwrap();
                }
                largest = largest.max(abs_f64(state.terminal_v - at_one[second]));
            }
            std::println!("step {dt_s} s against 1 s: {:.4} mV", 1.0e3 * largest);
            assert!(abs_f64(1.0e3 * largest - want_mv) < 0.01, "{dt_s} s: {largest:e}");
            assert!(state.residual_conservation_error < 1.0e-13);
        }
        // 4.5 A for 5 s, then -4.5 A for 5 s: at 0 degC from half charge.
        // No block measured 4.5 Ah at 0 degC; the same reversal at 4.25 Ah
        // is on the card.
        for (capacity_ah, paired, want_mv) in [(4.5, false, 48.97), (4.25, true, REVERSAL_ON_CARD_MV)] {
            let differences = reversal_differences_mv(&cell, capacity_ah, 0.0, 4.5, -4.5);
            std::println!("reversal at {capacity_ah} Ah: {differences:?}");
            assert!(differences[..5].iter().all(|mv| *mv < 0.05), "{differences:?}");
            assert!(abs_f64(differences[5] - want_mv) < 0.01, "{differences:?}");
            assert!(differences[5] > 9.0 && differences[6] > 1.0, "{differences:?}");
            assert_eq!(cell.capacity_compared_at(capacity_ah, 0.0), paired);
        }
        // `step` itself applies neither: a step above the ceiling and one
        // below the floor land.
        let mut state = rested(&cell, 1.0, 4.5006);
        assert_eq!(step(&cell, &mut state, &mut frame, load(4.5, 25.0), 10.0), Ok(()));
        assert_eq!(step(&cell, &mut state, &mut frame, load(4.5, 25.0), 0.01), Ok(()));
    }

    /// The terminal voltage at 0.1 s steps less that at 1 s steps, in
    /// millivolts, at each whole second of 5 s at one current and 5 s at
    /// another, from half charge, cell m80.
    fn reversal_differences_mv(
        cell: &Cell2170Model3,
        capacity_ah: f64,
        case_c: f64,
        first_a: f64,
        then_a: f64,
    ) -> Vec<f64> {
        let mut coarse = rested(cell, 0.5, capacity_ah);
        let mut fine = coarse;
        let mut frame = blank_frame();
        let mut out = Vec::new();
        for second in 0..10 {
            let current_a = if second < 5 { first_a } else { then_a };
            step_equations(cell, &mut coarse, &mut frame, load(current_a, case_c), 1.0).unwrap();
            for _ in 0..10 {
                step_equations(cell, &mut fine, &mut frame, load(current_a, case_c), 0.1).unwrap();
            }
            out.push(1.0e3 * abs_f64(fine.terminal_v - coarse.terminal_v));
        }
        out
    }

    /// The difference in the second of the reversal at 4.25 Ah, 0 degC.
    const REVERSAL_ON_CARD_MV: f64 = 48.973;

}
