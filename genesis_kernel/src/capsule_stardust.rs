#![no_implicit_prelude]

//! The Stardust sample return capsule on its entry of 15 January 2006, from
//! entry interface down to the drogue-deployment condition.
//!
//! A 45.8 kg blunt 60° cone, 0.81 m across, flying nose first, unpowered. The
//! card runs from a geocentric radius of 6503.14 km, which is 133.9 km above
//! the ground under the path, down to the drogue-deployment condition the
//! radar recorded: altitude under 31 030 m and Mach number under 1.23 at once.
//! A state past both is refused as [`StepReject::PastFlightDeployment`], the
//! card's end: the drogue opened there; the step that first crosses both
//! still lands. Below that the capsule flew on a parachute, and nothing there
//! was compared with the flight. The card's ceiling is 134.7 km, the top of
//! the drag table: the aerodynamics paper gives no drag above it.
//!
//! What was compared: the one entry from entry interface to the
//! radar-recorded drogue deployment, against NASA's reconstruction, and all
//! three lines were met: peak load 32.17 g against 32.89 g (a line of
//! 3.64 g); 31.03 km reached at 138.909 s and Mach 1.23 at 136.953 s, against
//! 137.9 s recorded (a line of 7.4 s). The card computes any other start
//! inside its tables on the same equations and ledger: down to sea level, to
//! Mach 0.16, and at air speeds above 68.96 km up to the 12 620 m/s its drag
//! table was computed at. None of that was compared with a flight.
//!
//! Where the locked numbers come from:
//!
//! - Mitcheltree, Wilmoth, Cheatwood, Brauckmann, Greene, "Aerodynamics of
//!   Stardust Sample Return Capsule", AIAA 97-2304, NTRS 20040105538: the
//!   reference area and both drag tables.
//! - Desai, Lyons, Tooley, Kangas, "Entry, Descent, and Landing Operations
//!   Analysis for the Stardust Re-Entry Capsule", AIAA-2006-6410, NTRS
//!   20060028186, and its journal preprint, NTRS 20090004445: mass, entry
//!   radius, entry speed and angle, landing-target latitude.
//! - Desai and Qualls, "Stardust Entry Reconstruction", AIAA-2008-1198, NTRS
//!   20080008567: the entry azimuth, 102.9°, and the radar altitude and Mach
//!   number at drogue deployment, past which the card refuses a state.
//! - U.S. Standard Atmosphere, 1976, NOAA-S/T 76-1562: air density, speed of
//!   sound, standard gravity.
//! - World Geodetic System 1984, NGA.STND.0036_1.0.0: the ellipsoid, the
//!   gravitational parameter, the rotation rate.
//!
//! The card has no attitude, no lift, no wind, no weather and no loss of mass,
//! and it uses a yearly standard atmosphere, not the air of that morning.
//!
//! `core` only: no heap and no standard prelude. The state is `Copy`. An
//! accepted step writes time, altitude, stagnation pressure, Mach number,
//! recovery enthalpy, heat-transfer coefficient, heat flux, load, and the
//! conservation residual into [`LastStateFrame64`]: the stagnation-point
//! environment along the entry, what a heat-shield model takes. Through
//! engine 0.14.3 the frame held ground distance, air speed and radial speed
//! in those three places; the state still publishes them. The 16-byte frame
//! proof is not touched.
//!
//! The stagnation-point environment, from the card's own air and speed, at
//! the nose:
//!
//! ```text
//! p∞  = ρ c² / γ                 static pressure of the 1976 air, γ = 1.4
//! p_s = p∞ (1 + (γ−1)/2 M²)^(γ/(γ−1))                         M < 1
//! p_s = p∞ ((γ+1)² M² / (4γM² − 2(γ−1)))^(γ/(γ−1)) (1 − γ + 2γM²)/(γ+1)
//!                                Rayleigh's pitot formula, M ≥ 1
//! H   = u²/2                     recovery enthalpy above the free stream's
//! C   = 2 k sqrt(ρ / R_n) u      ρ_e u_e C_H, so that C H = q
//! ```
//!
//! `q = k sqrt(ρ/R_n) u³` is the Sutton–Graves stagnation-point heat flux to
//! a cold wall the card reported before; `C` is the heat-transfer coefficient
//! it is made of, `q / (u²/2)`, and `H` the recovery enthalpy it takes. The
//! exponent 3.5 is `x³ sqrt(x)`. Above 86 km the speed of sound is held at
//! its 86 km value, so `p∞` there is not the real air's; `p_s` there is
//! within 0.1 percent of its hypersonic limit `0.92 ρ u²`, which does not
//! depend on it. `γ` is the perfect gas's: in the real shock layer at 12 km/s
//! the stagnation pressure is about 5 percent higher. `H` leaves out the free
//! stream's own enthalpy, under 0.1 MJ/kg from 0 to 134.7 km against 0.1 to
//! 79 MJ/kg of `H` on the compared entry.
//!
//! Air density and speed of sound are locked tables against geometric
//! altitude: 551 knots, one every 200 m from 30 km to 140 km, and below them
//! 151 knots from sea level to 30 km, [`AIR_DENSITY_LOW_KG_M3`] and
//! [`SOUND_SPEED_LOW_M_S`], read by straight lines between knots. Below sea
//! level the state is [`StepReject::BelowStandardSeaLevel`]: the capsule is on
//! or under the ground, and the card has no terrain of its own. The card reads them up to its ceiling, 134.7 km; the
//! 26 knots above 134.8 km are carried as copied and are never read. The
//! tables were computed once, from the standard's formulas
//! below 86 km and its printed density table from 86 km up, and rounded to
//! nine significant digits. The file is
//! `stardust_entry/analysis/out/card_tables.csv` in the research folder,
//! written by `07_tables_for_the_card.py`, SHA-256
//! `0ba3fd8c6fcd28238881b427f629fb2e492b15ab335d01bf63b1ac1d4c5483eb`. The
//! straight line differs from the standard's formula by at most 4.5 parts in
//! 10 000, in the one interval that holds the 47.35 km corner of the
//! standard's temperature profile, and by 0.7 parts in 10 000 on average. The
//! standard's own table steps by 0.003 percent at 86 km; the card inherits it.
//! Above 86 km the speed of sound holds its 86 km value, 274.096 m/s, where
//! the drag is read by altitude and the Mach number moves nothing. The Mach
//! number reported above 86 km is therefore not the Mach number of the real
//! air. Above 68.96 km the band of air speeds below takes the place of the
//! Mach floor: its bottom is more than thirty times the speed of sound
//! anywhere above 68.96 km.
//!
//! Drag is two tables from the aerodynamics paper, also straight lines: by
//! altitude from 68.96 km to 134.7 km, and by Mach number below 68.96 km. The
//! altitude table spans exactly the altitudes it is read at, so nothing is
//! held at either end of it. The Mach table is read from its first entry,
//! Mach 0.16; below it the state is [`StepReject::MachOutsideCard`]. Its four
//! entries below Mach 1.0 were read from the paper's Figure 15, and until
//! engine 0.9 the card's floor was Mach 1.0.
//!
//! No table is read past its end. The Mach table stops at Mach 40, and below
//! 68.96 km a Mach number above 40 is refused as
//! [`StepReject::MachOutsideCard`]. The locked entry never comes near it: the
//! Mach number falls through 40 at 43.36 s, at 69.23 km, while drag is still
//! read by altitude, and the capsule reaches 68.96 km at 43.60 s at Mach
//! 39.84. Through engine 0.6.0 the table's top entry was held above Mach 40.
//!
//! The drag table by altitude is the paper's Table 1, computed by direct
//! simulation at points of the 1997 design trajectory, at speeds from
//! 12 487 to 12 620 m/s, of a kind the paper does not state; its bottom row,
//! at 68.96 km, is from the paper's Table 4, at a design speed of 12 173 m/s.
//! The table gives drag by altitude alone, so it holds only near the speeds it
//! was computed at. Above 68.96 km the card takes an air speed from
//! [`AIR_SPEED_BAND_MIN_M_S`], 11 900 m/s, to [`AIR_SPEED_BAND_MAX_M_S`],
//! 12 620 m/s, the highest speed the table was computed at, and refuses any
//! other as [`StepReject::AirSpeedOutsideCard`]. The locked entry, which met
//! all three lines, passed through 11 911.98 to 12 573.44 m/s there, at every
//! step from 0.05 s to 0.0005 s; from 12 575 to 12 620 m/s the card holds the
//! source's own design speeds, which were not compared with the flight. Engines
//! 0.7 to 0.9 stopped at 12 575 m/s; through engine 0.6.0 any air speed was
//! taken there.
//!
//! Only the entry from the locked entry state was compared with the flight.
//! At each altitude below 69 km it flew the air speeds and path angles of the
//! corridor kept in [`CORRIDOR_AIR_SPEED_LOW_M_S`] to
//! [`CORRIDOR_PATH_SINE_HIGH`]: the lowest and highest air speed and sine of
//! the path angle it had within 500 m of each knot, every 500 m from 31 km to
//! 69 km, widened by 1 m/s and by 0.001 (at 40 km 1147.51 to 1350.11 m/s).
//! Engines 0.7 to 0.9 refused a state off that corridor as
//! `OffComparedCorridor`. The card now computes any start inside its tables on
//! the same RK4 and ledger; a state deep in the air at entry speed is held by
//! the ledger at the 0.05 s step and lands at a smaller one. Other paths were
//! not compared with a flight.
//!
//! The drag is continuous in every place but one. Where the read by altitude
//! hands over to the read by Mach number, at 68.96 km, the value steps. On
//! the locked entry it steps from 1.4959 down to about 1.49545, 3 parts in
//! 10 000, because the capsule arrives at Mach 39.84 and the Mach table is a
//! straight line from 1.4828 at Mach 35.4 to 1.4959 at Mach 40.
//!
//! The step is classical RK4 on five numbers. Its arithmetic is `+ − × ÷` and
//! `sqrt`.
//!
//! ```text
//! h   = r − R                     altitude
//! u_r = v_r                       velocity through the air, radial
//! u_t = v_t − Ω r                 velocity through the air, transverse
//! u   = sqrt(u_r² + u_t²)
//! M   = u / c(h)
//! a   = ½ ρ(h) u² C(h, M) A / m   drag acceleration
//!
//! d r/dt   = v_r
//! d v_r/dt = v_t²/r − GM/r² − a u_r/u
//! d v_t/dt = −v_r v_t/r − a u_t/u
//! d θ/dt   = v_t/r
//! d Q/dt   = a u                  energy per kilogram removed by drag
//!
//! J = ½ (v_r² + v_t²) − GM/r − Ω r v_t
//! residual = |J − J_anchor + Q| / ENERGY_SCALE_J_KG
//! ```
//!
//! `r`, `v_r` and `v_t` are in a frame that does not turn with the Earth. The
//! air turns with the Earth at rate `Ω` in the flight plane. `J` is the energy
//! per kilogram as seen from the frame that turns with the air: kinetic plus
//! gravitational energy, less `Ω` times angular momentum. Without air it is
//! constant. With air only drag changes it, by exactly `−a u`.
//!
//! The ledger shows that the drag loss booked is the work of the force that
//! was applied. It catches a force that does not match its booking. It does
//! not measure how accurate the step is: RK4 is not exact for these
//! equations, and at the ceiling step the error in the drag loss, measured
//! against a step 64 times finer, is tens of thousands of times the reported
//! residual through the heating peak, 57 000 times at 40 s. The ledger is
//! blind to a wrong table used consistently, to a wrong mass, and to a
//! mis-timed step. Accuracy rests on the step ceiling and on the
//! reference-state tests.
//!
//! [`CapsuleStardust::gate_dt_s`] is 0.05 s, and
//! [`CapsuleStardust::floor_dt_s`] is that divided by 2^20. The public step
//! applies neither. A Mach number from 1.2 to 1.6 sets
//! [`FLAG_DROGUE_MACH_WINDOW`], which does not refuse the step. An incoming
//! state under 31 030 m and under Mach 1.23 at once is refused as
//! [`StepReject::PastFlightDeployment`]; through engine 0.6.0 it only set a
//! flag, and the card ran on to 30 km or Mach 1.0. The step that first meets
//! both conditions lands, so a run at a smaller step can end earlier, by less
//! than one of the larger steps: from the locked entry at 0.05 s the last landed state is at
//! 31 020.2 m, and at 0.005 s at 31 029.67 m.

extern crate core;

use core::prelude::v1::*;

use crate::exceedance::{Exceedance, ExceedanceKind, Side};
use crate::frame::LastStateFrame64;
use crate::normal::normal_or_zero;

/// Capsule mass. Operations paper p.2. Held fixed.
pub const CAPSULE_MASS_KG: f64 = 45.8;
/// The area the drag coefficients were made with. Aerodynamics paper p.2.
pub const REFERENCE_AREA_M2: f64 = 0.51887;
/// Geocentric radius of entry interface. Operations paper, journal preprint p.3.
pub const ENTRY_RADIUS_M: f64 = 6_503_140.0;
/// `12 900 m/s × sin(−8.21°)`. Speed on p.2 and angle on p.5 of the operations paper.
pub const ENTRY_RADIAL_SPEED_M_S: f64 = -1842.1416729435698;
/// `12 900 m/s × cos(−8.21°)`. The same two numbers.
pub const ENTRY_TRANSVERSE_SPEED_M_S: f64 = 12767.792058801891;
/// WGS 84 ellipsoid radius at geodetic latitude 40.3167°, the landing target
/// (operations paper p.7). One sphere of this radius stands for the Earth.
pub const LOCAL_EARTH_RADIUS_M: f64 = 6369228.511478454;
/// WGS 84 Table 3.1.
pub const EARTH_GRAVITATIONAL_PARAMETER_M3_S2: f64 = 3.986004418e14;
/// `7.292115e-5 rad/s × cos(40.3167°) × sin(102.9°)`: the WGS 84 rotation rate
/// seen in the flight plane. The azimuth is from the reconstruction paper p.7.
pub const AIR_ROTATION_RATE_RAD_S: f64 = 5.4197601852574874e-05;
/// One Earth g. 1976 standard atmosphere, printed p.2. The unit of the reported load.
pub const STANDARD_GRAVITY_M_S2: f64 = 9.80665;

/// Top of the card: the top of the drag table read by altitude. The
/// aerodynamics paper gives no drag above it.
pub const ALTITUDE_CEILING_M: f64 = 134_700.0;
/// Bottom of the air tables of the research record, [`AIR_DENSITY_KG_M3`] and
/// [`SOUND_SPEED_M_S`]: one kilometre under the radar altitude at drogue
/// deployment. Below it the card reads [`AIR_DENSITY_LOW_KG_M3`] and
/// [`SOUND_SPEED_LOW_M_S`], the same standard's formulas from sea level.
pub const ALTITUDE_FLOOR_M: f64 = 30_000.0;
/// The sea level of the 1976 standard, the bottom of its air, and the
/// ellipsoid under the path: the lowest altitude the card takes. Below it the
/// capsule is on or under the ground, and the state is
/// [`StepReject::BelowStandardSeaLevel`].
pub const SEA_LEVEL_M: f64 = 0.0;
/// Bottom of the card in Mach number: the first entry of the drag table read
/// by Mach number, read from the aerodynamics paper's Figure 15.
pub const MACH_FLOOR: f64 = 0.16;
/// Lowest air speed taken above 68.96 km, where drag is read by altitude. The
/// locked entry's lowest there is 11 911.98 m/s, at 68.96 km.
pub const AIR_SPEED_BAND_MIN_M_S: f64 = 11_900.0;
/// Highest air speed taken above 68.96 km: the highest speed of the 1997
/// design trajectory at which the drag table by altitude was computed. The
/// locked entry's highest there is 12 573.44 m/s.
pub const AIR_SPEED_BAND_MAX_M_S: f64 = 12_620.0;

/// Knots of the corridor the locked entry flew: 31 km to 69 km every 500 m.
pub const CORRIDOR_KNOTS: usize = 77;
/// Lowest knot of the corridor. The locked entry's lowest landed state, at
/// 0.05 s, is 31 020.2 m.
pub const CORRIDOR_BOTTOM_M: f64 = 31_000.0;
/// Distance between corridor knots.
pub const CORRIDOR_KNOT_SPACING_M: f64 = 500.0;
/// Top of the corridor, its last knot, just above the switch to the drag
/// table by altitude.
pub const CORRIDOR_TOP_M: f64 = 69_000.0;
// The four corridor tables, which describe what was compared and are not a
// limit of the card. Each knot holds the lowest or highest value that
// the locked entry had within 500 m of its altitude, widened by 1 m/s or by
// 0.001 and rounded outward: runs from the locked entry state at 0.05, 0.04,
// 0.03, 0.025, 0.02, 0.01, 0.005, 0.0025, 0.001 and 0.0005 s, each to the
// first state under both 31 030 m and Mach 1.23. Written by the 0.7.0 fix,
// `stardust_entry/fix_0.7.0/corridor` in the research folder, from the 0.6.0
// kernel's runs.
/// Lowest speed through the air at each corridor knot, m/s.
pub const CORRIDOR_AIR_SPEED_LOW_M_S: [f64; CORRIDOR_KNOTS] = [
    353.56, 353.56, 371.56, 392.31, 415.31, 440.69, 468.79, 499.96, 534.59, 572.87,
    615.26, 662.25, 714.36, 771.54, 834.11, 902.56, 977.38, 1058.88, 1147.51, 1243.68,
    1348.13, 1462.32, 1586.97, 1722.89, 1870.74, 2031.20, 2204.78, 2391.86, 2591.94, 2804.70,
    3029.62, 3266.22, 3513.33, 3768.99, 4031.48, 4300.66, 4576.02, 4856.18, 5139.63, 5424.83,
    5710.50, 5994.93, 6277.10, 6556.59, 6832.68, 7104.73, 7371.34, 7632.12, 7886.10, 8132.90,
    8372.15, 8603.44, 8826.45, 9040.99, 9247.13, 9444.40, 9633.06, 9813.13, 9984.56, 10147.65,
    10302.46, 10449.14, 10588.09, 10719.40, 10843.28, 10960.12, 11070.23, 11173.92, 11271.46, 11363.07,
    11449.02, 11529.69, 11605.14, 11675.88, 11741.97, 11803.70, 11861.39,
];
/// Highest speed through the air at each corridor knot, m/s.
pub const CORRIDOR_AIR_SPEED_HIGH_M_S: [f64; CORRIDOR_KNOTS] = [
    373.57, 394.31, 417.31, 442.70, 470.79, 501.97, 536.59, 574.87, 617.26, 664.25,
    716.36, 773.53, 836.11, 904.55, 979.37, 1060.87, 1149.49, 1245.66, 1350.11, 1464.30,
    1588.95, 1724.86, 1872.71, 2033.16, 2206.74, 2393.82, 2593.89, 2806.64, 3031.56, 3268.15,
    3515.26, 3770.91, 4033.39, 4302.57, 4577.92, 4858.07, 5141.51, 5426.72, 5712.38, 5996.81,
    6278.97, 6558.45, 6834.54, 7106.59, 7373.19, 7633.98, 7887.95, 8134.75, 8374.00, 8605.29,
    8828.31, 9042.85, 9248.99, 9446.26, 9634.93, 9815.00, 9986.43, 10149.52, 10304.35, 10451.03,
    10589.98, 10721.29, 10845.18, 10962.03, 11072.13, 11175.83, 11273.38, 11364.99, 11450.94, 11531.62,
    11607.07, 11677.82, 11743.91, 11805.64, 11863.34, 11917.14, 11967.28,
];
/// Lowest sine of the path angle through the air, radial air speed over air
/// speed, at each corridor knot.
pub const CORRIDOR_PATH_SINE_LOW: [f64; CORRIDOR_KNOTS] = [
    -0.6690, -0.6690, -0.6382, -0.6056, -0.5729, -0.5404, -0.5082, -0.4767, -0.4460, -0.4165,
    -0.3881, -0.3612, -0.3357, -0.3117, -0.2893, -0.2685, -0.2491, -0.2312, -0.2147, -0.1995,
    -0.1856, -0.1729, -0.1613, -0.1509, -0.1415, -0.1331, -0.1256, -0.1189, -0.1131, -0.1080,
    -0.1035, -0.0997, -0.0963, -0.0934, -0.0910, -0.0889, -0.0871, -0.0856, -0.0844, -0.0834,
    -0.0825, -0.0819, -0.0814, -0.0810, -0.0807, -0.0806, -0.0805, -0.0806, -0.0808, -0.0810,
    -0.0813, -0.0816, -0.0819, -0.0822, -0.0826, -0.0830, -0.0834, -0.0839, -0.0843, -0.0848,
    -0.0852, -0.0857, -0.0862, -0.0867, -0.0872, -0.0877, -0.0882, -0.0887, -0.0892, -0.0897,
    -0.0902, -0.0908, -0.0913, -0.0918, -0.0923, -0.0929, -0.0934,
];
/// Highest sine of the path angle through the air at each corridor knot.
pub const CORRIDOR_PATH_SINE_HIGH: [f64; CORRIDOR_KNOTS] = [
    -0.6361, -0.6035, -0.5708, -0.5383, -0.5061, -0.4746, -0.4439, -0.4144, -0.3860, -0.3591,
    -0.3336, -0.3097, -0.2872, -0.2664, -0.2470, -0.2291, -0.2126, -0.1974, -0.1835, -0.1708,
    -0.1592, -0.1488, -0.1394, -0.1310, -0.1235, -0.1168, -0.1110, -0.1059, -0.1014, -0.0976,
    -0.0942, -0.0913, -0.0889, -0.0868, -0.0850, -0.0835, -0.0823, -0.0813, -0.0804, -0.0798,
    -0.0793, -0.0789, -0.0786, -0.0785, -0.0784, -0.0784, -0.0784, -0.0784, -0.0784, -0.0785,
    -0.0787, -0.0789, -0.0792, -0.0795, -0.0798, -0.0801, -0.0805, -0.0809, -0.0813, -0.0818,
    -0.0822, -0.0827, -0.0831, -0.0836, -0.0841, -0.0846, -0.0851, -0.0856, -0.0861, -0.0866,
    -0.0871, -0.0876, -0.0881, -0.0887, -0.0892, -0.0897, -0.0902,
];

/// Distance between knots of the air tables.
pub const TABLE_KNOT_SPACING_M: f64 = 200.0;
/// Knots in each air table below [`ALTITUDE_FLOOR_M`]: sea level to 30 km
/// every 200 m. The top knot is the first knot of [`AIR_DENSITY_KG_M3`] and
/// [`SOUND_SPEED_M_S`], equal to it bit for bit.
pub const LOW_KNOTS: usize = 151;
/// Air density from sea level to 30 km every 200 m, kg/m³: the U.S. Standard
/// Atmosphere 1976's formulas for its first three layers (geopotential
/// altitude, base temperatures 288.15, 216.65 and 216.65 K, lapse rates
/// -6.5, 0 and +1 K/km, 101 325 Pa at sea level, g0 9.80665 m/s², molar mass
/// 0.0289644 kg/mol, gas constant 8.31432 J/(mol K), Earth radius 6 356 766 m),
/// rounded to nine significant digits, as the research record's table was
/// above 30 km. The same formulas give that table's knots from 30 km to
/// 50 km to the last digit. A test recomputes every knot.
#[cfg_attr(rustfmt, rustfmt::skip)]
pub const AIR_DENSITY_LOW_KG_M3: [f64; LOW_KNOTS] = [
    1.22499916, 1.20165139, 1.17864703, 1.15598252, 1.13365434, 1.11165899,
    1.08999296, 1.06865281, 1.04763508, 1.02693635, 1.00655322, 0.986482305,
    0.96672025, 0.947263714, 0.928109377, 0.909253941, 0.890694129, 0.872426686,
    0.854448375, 0.83675598, 0.819346309, 0.802216186, 0.785362458, 0.768781993,
    0.752471678, 0.736428421, 0.72064915, 0.705130815, 0.689870383, 0.674864845,
    0.660111211, 0.645606509, 0.631347789, 0.617332123, 0.603556599, 0.590018328,
    0.576714441, 0.563642086, 0.550798435, 0.538180678, 0.525786023, 0.513611702,
    0.501654962, 0.489913074, 0.478383326, 0.467063026, 0.455949504, 0.445040106,
    0.434332199, 0.423823172, 0.413510429, 0.403391397, 0.39346352, 0.383724263,
    0.37417111, 0.364801564, 0.353716771, 0.342773765, 0.332169962, 0.321894825,
    0.311938149, 0.302290044, 0.292940928, 0.283881518, 0.27510282, 0.266596119,
    0.25835297, 0.250365193, 0.242624862, 0.235124295, 0.227856052, 0.220812923,
    0.213987922, 0.20737428, 0.20096544, 0.194755046, 0.188736944, 0.182905166,
    0.177253933, 0.171777645, 0.166470876, 0.161328368, 0.156345027, 0.151515917,
    0.146836255, 0.142301408, 0.137906885, 0.133648335, 0.129521545, 0.125522428,
    0.121647028, 0.117891509, 0.114252157, 0.11072537, 0.107307661, 0.103995649,
    0.10078606, 0.0976757195, 0.0946615527, 0.0917405803, 0.0889099151, 0.0861132775,
    0.0833829721, 0.080741778, 0.0781867042, 0.0757148627, 0.073323465, 0.0710098187,
    0.0687713239, 0.06660547, 0.0645098327, 0.0624820709, 0.0605199235, 0.0586212069,
    0.0567838121, 0.0550057024, 0.0532849104, 0.0516195357, 0.0500077429, 0.0484477589,
    0.046937871, 0.0454764245, 0.0440618211, 0.0426925165, 0.0413670189, 0.0400838867,
    0.0388417274, 0.0376391952, 0.0364749901, 0.0353478555, 0.0342565774, 0.0331999827,
    0.0321769375, 0.0311863463, 0.03022715, 0.0292983254, 0.0283988833, 0.0275278677,
    0.0266843546, 0.0258674508, 0.0250762929, 0.0243100463, 0.0235679042, 0.0228490869,
    0.0221528401, 0.0214784351, 0.020825167, 0.0201923544, 0.0195793387, 0.0189854826,
    0.0184101704,
];
/// Speed of sound from sea level to 30 km every 200 m, m/s: the same
/// standard, `sqrt(1.4 R* T / M0)`, rounded to nine significant digits.
#[cfg_attr(rustfmt, rustfmt::skip)]
pub const SOUND_SPEED_LOW_M_S: [f64; LOW_KNOTS] = [
    340.294108, 339.525639, 338.755476, 337.983606, 337.210018, 336.434701,
    335.657641, 334.878827, 334.098247, 333.315888, 332.531738, 331.745784,
    330.958014, 330.168413, 329.376969, 328.583669, 327.788499, 326.991446,
    326.192495, 325.391633, 324.588846, 323.784118, 322.977437, 322.168787,
    321.358153, 320.54552, 319.730873, 318.914196, 318.095474, 317.274691,
    316.451831, 315.626878, 314.799815, 313.970626, 313.139293, 312.3058,
    311.470129, 310.632262, 309.792182, 308.949871, 308.105311, 307.258482,
    306.409367, 305.557945, 304.704198, 303.848106, 302.989649, 302.128807,
    301.26556, 300.399886, 299.531766, 298.661176, 297.788097, 296.912505,
    296.034379, 295.153695, 295.069597, 295.069597, 295.069597, 295.069597,
    295.069597, 295.069597, 295.069597, 295.069597, 295.069597, 295.069597,
    295.069597, 295.069597, 295.069597, 295.069597, 295.069597, 295.069597,
    295.069597, 295.069597, 295.069597, 295.069597, 295.069597, 295.069597,
    295.069597, 295.069597, 295.069597, 295.069597, 295.069597, 295.069597,
    295.069597, 295.069597, 295.069597, 295.069597, 295.069597, 295.069597,
    295.069597, 295.069597, 295.069597, 295.069597, 295.069597, 295.069597,
    295.069597, 295.069597, 295.069597, 295.069597, 295.069597, 295.162206,
    295.297463, 295.43265, 295.567766, 295.702813, 295.837789, 295.972695,
    296.107531, 296.242298, 296.376995, 296.511622, 296.646179, 296.780668,
    296.915086, 297.049436, 297.183716, 297.317928, 297.45207, 297.586144,
    297.720148, 297.854084, 297.987952, 298.121751, 298.255482, 298.389144,
    298.522738, 298.656264, 298.789722, 298.923112, 299.056434, 299.189688,
    299.322875, 299.455994, 299.589045, 299.72203, 299.854946, 299.987796,
    300.120579, 300.253294, 300.385943, 300.518525, 300.651039, 300.783488,
    300.915869, 301.048185, 301.180433, 301.312616, 301.444732, 301.576782,
    301.708766,
];
/// Knots in each air table: 30 km to 140 km every 200 m. The card reads them
/// up to its ceiling.
pub const KNOTS: usize = 551;

/// Air density at each knot, from 30 km up, as decimal literals.
pub const AIR_DENSITY_KG_M3: [f64; KNOTS] = [
    1.84101704e-2,
    1.78528064e-2,
    1.73128146e-2,
    1.67896382e-2,
    1.62827386e-2,
    1.57915948e-2,
    1.53157032e-2,
    1.48545765e-2,
    1.44077437e-2,
    1.39747488e-2,
    1.35551512e-2,
    1.31446335e-2,
    1.27312736e-2,
    1.23318885e-2,
    1.19459731e-2,
    1.15730416e-2,
    1.12126268e-2,
    1.08642794e-2,
    1.05275674e-2,
    1.02020752e-2,
    9.88740289e-3,
    9.58316588e-3,
    9.28899406e-3,
    9.00453132e-3,
    8.72943493e-3,
    8.46337508e-3,
    8.20603430e-3,
    7.95710700e-3,
    7.71629898e-3,
    7.48332701e-3,
    7.25791838e-3,
    7.03981045e-3,
    6.82875034e-3,
    6.62449445e-3,
    6.42680818e-3,
    6.23546551e-3,
    6.05024872e-3,
    5.87094804e-3,
    5.69736135e-3,
    5.52929387e-3,
    5.36655791e-3,
    5.20897256e-3,
    5.05636344e-3,
    4.90856248e-3,
    4.76540763e-3,
    4.62674267e-3,
    4.49241696e-3,
    4.36228526e-3,
    4.23620748e-3,
    4.11404854e-3,
    3.99567814e-3,
    3.88097059e-3,
    3.76980464e-3,
    3.66206331e-3,
    3.55763376e-3,
    3.45640708e-3,
    3.35827818e-3,
    3.26314565e-3,
    3.17091162e-3,
    3.08148159e-3,
    2.99476439e-3,
    2.91067197e-3,
    2.82911934e-3,
    2.75002443e-3,
    2.67330800e-3,
    2.59889353e-3,
    2.52670714e-3,
    2.45667743e-3,
    2.38873548e-3,
    2.32281469e-3,
    2.25885074e-3,
    2.19678148e-3,
    2.13654686e-3,
    2.07808886e-3,
    2.02135142e-3,
    1.96628035e-3,
    1.91282329e-3,
    1.86092962e-3,
    1.81055044e-3,
    1.76163844e-3,
    1.71414790e-3,
    1.66803463e-3,
    1.62325589e-3,
    1.57977034e-3,
    1.53753804e-3,
    1.49652033e-3,
    1.45667984e-3,
    1.41869951e-3,
    1.38384854e-3,
    1.34985580e-3,
    1.31670010e-3,
    1.28436077e-3,
    1.25281768e-3,
    1.22205116e-3,
    1.19204205e-3,
    1.16277166e-3,
    1.13422176e-3,
    1.10637457e-3,
    1.07921276e-3,
    1.05271941e-3,
    1.02687803e-3,
    1.00167255e-3,
    9.77087263e-4,
    9.53106884e-4,
    9.29716491e-4,
    9.06901534e-4,
    8.84647820e-4,
    8.62941511e-4,
    8.43360283e-4,
    8.24290929e-4,
    8.05616211e-4,
    7.87328719e-4,
    7.69421167e-4,
    7.51886393e-4,
    7.34717357e-4,
    7.17907138e-4,
    7.01448932e-4,
    6.85336050e-4,
    6.69561920e-4,
    6.54120079e-4,
    6.39004177e-4,
    6.24207972e-4,
    6.09725330e-4,
    5.95550221e-4,
    5.81676721e-4,
    5.68099007e-4,
    5.54811359e-4,
    5.41808154e-4,
    5.29083868e-4,
    5.16633073e-4,
    5.04450435e-4,
    4.92530716e-4,
    4.80868767e-4,
    4.69459532e-4,
    4.58298041e-4,
    4.47379414e-4,
    4.36698858e-4,
    4.26251662e-4,
    4.16033201e-4,
    4.06038932e-4,
    3.96264393e-4,
    3.86705200e-4,
    3.77357050e-4,
    3.68215715e-4,
    3.59277045e-4,
    3.50536964e-4,
    3.41991469e-4,
    3.33636629e-4,
    3.25468587e-4,
    3.17483553e-4,
    3.09677808e-4,
    3.02047699e-4,
    2.94589641e-4,
    2.87300115e-4,
    2.80175667e-4,
    2.73212904e-4,
    2.66408498e-4,
    2.59759182e-4,
    2.53261749e-4,
    2.46913052e-4,
    2.40710003e-4,
    2.34649569e-4,
    2.28728777e-4,
    2.22944708e-4,
    2.17294498e-4,
    2.11775336e-4,
    2.06384466e-4,
    2.01119181e-4,
    1.95976830e-4,
    1.90954806e-4,
    1.86050557e-4,
    1.81261577e-4,
    1.76585409e-4,
    1.72019641e-4,
    1.67561909e-4,
    1.63209895e-4,
    1.58961324e-4,
    1.54813966e-4,
    1.50765633e-4,
    1.46814181e-4,
    1.42957506e-4,
    1.39193547e-4,
    1.35520282e-4,
    1.31935729e-4,
    1.28437943e-4,
    1.25025021e-4,
    1.21695093e-4,
    1.18446330e-4,
    1.15276937e-4,
    1.12185155e-4,
    1.09169260e-4,
    1.06227563e-4,
    1.03358408e-4,
    1.00560171e-4,
    9.78312638e-5,
    9.51701274e-5,
    9.25752354e-5,
    9.00450918e-5,
    8.75782314e-5,
    8.51732182e-5,
    8.28286459e-5,
    8.05431367e-5,
    7.83153409e-5,
    7.61439366e-5,
    7.40276291e-5,
    7.19651504e-5,
    6.99552584e-5,
    6.79967371e-5,
    6.60883954e-5,
    6.42290673e-5,
    6.23731822e-5,
    6.05673528e-5,
    5.88107512e-5,
    5.71021188e-5,
    5.54402271e-5,
    5.38238767e-5,
    5.22518974e-5,
    5.07231467e-5,
    4.92365098e-5,
    4.77908985e-5,
    4.63852511e-5,
    4.50185315e-5,
    4.36897285e-5,
    4.23978556e-5,
    4.11419503e-5,
    3.99210733e-5,
    3.87343085e-5,
    3.75807619e-5,
    3.64595615e-5,
    3.53698568e-5,
    3.43108179e-5,
    3.32816354e-5,
    3.22815201e-5,
    3.13097020e-5,
    3.03654301e-5,
    2.94479723e-5,
    2.85566145e-5,
    2.76906603e-5,
    2.68494307e-5,
    2.60322636e-5,
    2.52385137e-5,
    2.44675514e-5,
    2.37187634e-5,
    2.29915515e-5,
    2.22853325e-5,
    2.15995383e-5,
    2.09336147e-5,
    2.02870219e-5,
    1.96592336e-5,
    1.90497369e-5,
    1.84580320e-5,
    1.78836319e-5,
    1.73260619e-5,
    1.67848595e-5,
    1.62595742e-5,
    1.57497669e-5,
    1.52550099e-5,
    1.47748866e-5,
    1.43089910e-5,
    1.38569277e-5,
    1.34183116e-5,
    1.29927677e-5,
    1.25799304e-5,
    1.21794440e-5,
    1.17909619e-5,
    1.14141468e-5,
    1.10486700e-5,
    1.06942116e-5,
    1.03504601e-5,
    1.00171121e-5,
    9.69387241e-6,
    9.38045355e-6,
    9.07657569e-6,
    8.78196646e-6,
    8.49636075e-6,
    8.21950050e-6,
    7.95113460e-6,
    7.69101863e-6,
    7.43891475e-6,
    7.19459153e-6,
    6.95800000e-6,
    6.71478038e-6,
    6.48006260e-6,
    6.25354947e-6,
    6.03495419e-6,
    5.82400000e-6,
    5.62046202e-6,
    5.42403730e-6,
    5.23447727e-6,
    5.05154201e-6,
    4.87500000e-6,
    4.70471169e-6,
    4.54037172e-6,
    4.38177229e-6,
    4.22871289e-6,
    4.08100000e-6,
    3.93837303e-6,
    3.80073072e-6,
    3.66789889e-6,
    3.53970940e-6,
    3.41600000e-6,
    3.29676125e-6,
    3.18168463e-6,
    3.07062488e-6,
    2.96344178e-6,
    2.86000000e-6,
    2.75982387e-6,
    2.66315657e-6,
    2.56987519e-6,
    2.47986115e-6,
    2.39300000e-6,
    2.30866095e-6,
    2.22729436e-6,
    2.14879546e-6,
    2.07306317e-6,
    2.00000000e-6,
    1.92915575e-6,
    1.86082095e-6,
    1.79490671e-6,
    1.73132730e-6,
    1.67000000e-6,
    1.61050991e-6,
    1.55313901e-6,
    1.49781184e-6,
    1.44445557e-6,
    1.39300000e-6,
    1.34339002e-6,
    1.29554685e-6,
    1.24940754e-6,
    1.20491143e-6,
    1.16200000e-6,
    1.12043025e-6,
    1.08034764e-6,
    1.04169895e-6,
    1.00443289e-6,
    9.68500000e-7,
    9.33824297e-7,
    9.00390106e-7,
    8.68152976e-7,
    8.37070049e-7,
    8.07100000e-7,
    7.78180485e-7,
    7.50297197e-7,
    7.23413005e-7,
    6.97492112e-7,
    6.72500000e-7,
    6.48415602e-7,
    6.25193744e-7,
    6.02803536e-7,
    5.81215194e-7,
    5.60400000e-7,
    5.40910760e-7,
    5.22099304e-7,
    5.03942062e-7,
    4.86416281e-7,
    4.69500000e-7,
    4.53207851e-7,
    4.37481057e-7,
    4.22299999e-7,
    4.07645740e-7,
    3.93500000e-7,
    3.79890626e-7,
    3.66751938e-7,
    3.54067658e-7,
    3.41822070e-7,
    3.30000000e-7,
    3.18621988e-7,
    3.07636277e-7,
    2.97029339e-7,
    2.86788117e-7,
    2.76900000e-7,
    2.67388642e-7,
    2.58203994e-7,
    2.49334834e-7,
    2.40770324e-7,
    2.32500000e-7,
    2.24555283e-7,
    2.16882044e-7,
    2.09471006e-7,
    2.02313209e-7,
    1.95400000e-7,
    1.88741395e-7,
    1.82309694e-7,
    1.76097164e-7,
    1.70096338e-7,
    1.64300000e-7,
    1.58689717e-7,
    1.53271007e-7,
    1.48037326e-7,
    1.42982358e-7,
    1.38100000e-7,
    1.33389421e-7,
    1.28839519e-7,
    1.24444814e-7,
    1.20200012e-7,
    1.16100000e-7,
    1.12019010e-7,
    1.08081469e-7,
    1.04282335e-7,
    1.00616744e-7,
    9.70800000e-8,
    9.36523532e-8,
    9.03457279e-8,
    8.71558510e-8,
    8.40786006e-8,
    8.11100000e-8,
    7.83872341e-8,
    7.57558681e-8,
    7.32128340e-8,
    7.07551665e-8,
    6.83800000e-8,
    6.61901633e-8,
    6.40704550e-8,
    6.20186294e-8,
    6.00325125e-8,
    5.81100000e-8,
    5.63325379e-8,
    5.46094446e-8,
    5.29390570e-8,
    5.13197631e-8,
    4.97500000e-8,
    4.82953900e-8,
    4.68833104e-8,
    4.55125178e-8,
    4.41818049e-8,
    4.28900000e-8,
    4.16863072e-8,
    4.05163957e-8,
    3.93793173e-8,
    3.82741506e-8,
    3.72000000e-8,
    3.61996236e-8,
    3.52261492e-8,
    3.42788533e-8,
    3.33570319e-8,
    3.24600000e-8,
    3.16195952e-8,
    3.08009489e-8,
    3.00034977e-8,
    2.92266929e-8,
    2.84700000e-8,
    2.77594020e-8,
    2.70665402e-8,
    2.63909719e-8,
    2.57322654e-8,
    2.50900000e-8,
    2.44877758e-8,
    2.39000065e-8,
    2.33263451e-8,
    2.27664532e-8,
    2.22200000e-8,
    2.17068386e-8,
    2.12055285e-8,
    2.07157959e-8,
    2.02373735e-8,
    1.97700000e-8,
    1.93309254e-8,
    1.89016023e-8,
    1.84818141e-8,
    1.80713490e-8,
    1.76700000e-8,
    1.72900040e-8,
    1.69181798e-8,
    1.65543518e-8,
    1.61983479e-8,
    1.58500000e-8,
    1.55227640e-8,
    1.52022840e-8,
    1.48884207e-8,
    1.45810372e-8,
    1.42800000e-8,
    1.39948360e-8,
    1.37153666e-8,
    1.34414781e-8,
    1.31730589e-8,
    1.29100000e-8,
    1.26605442e-8,
    1.24159085e-8,
    1.21759998e-8,
    1.19407269e-8,
    1.17100000e-8,
    1.14898785e-8,
    1.12738949e-8,
    1.10619712e-8,
    1.08540312e-8,
    1.06500000e-8,
    1.04564948e-8,
    1.02665055e-8,
    1.00799683e-8,
    9.89682028e-9,
    9.71700000e-9,
    9.54544860e-9,
    9.37692590e-9,
    9.21137843e-9,
    9.04875366e-9,
    8.88900000e-9,
    8.73645329e-9,
    8.58652447e-9,
    8.43916863e-9,
    8.29434161e-9,
    8.15200000e-9,
    8.01593293e-9,
    7.88213698e-9,
    7.75057426e-9,
    7.62120748e-9,
    7.49400000e-9,
    7.37209813e-9,
    7.25217919e-9,
    7.13421092e-9,
    7.01816160e-9,
    6.90400000e-9,
    6.79458660e-9,
    6.68690716e-9,
    6.58093420e-9,
    6.47664069e-9,
    6.37400000e-9,
    6.27560894e-9,
    6.17873667e-9,
    6.08335975e-9,
    5.98945511e-9,
    5.89700000e-9,
    5.80795089e-9,
    5.72024649e-9,
    5.63386649e-9,
    5.54879089e-9,
    5.46500000e-9,
    5.38446088e-9,
    5.30510868e-9,
    5.22692591e-9,
    5.14989535e-9,
    5.07400000e-9,
    5.00092528e-9,
    4.92890297e-9,
    4.85791791e-9,
    4.78795516e-9,
    4.71900000e-9,
    4.65255503e-9,
    4.58704563e-9,
    4.52245861e-9,
    4.45878101e-9,
    4.39600000e-9,
    4.33534936e-9,
    4.27553551e-9,
    4.21654690e-9,
    4.15837214e-9,
    4.10100000e-9,
    4.04551901e-9,
    3.99078859e-9,
    3.93679861e-9,
    3.88353904e-9,
    3.83100000e-9,
];

/// Speed of sound at each knot, from 30 km up, as decimal literals.
pub const SOUND_SPEED_M_S: [f64; KNOTS] = [
    3.01708766e2,
    3.01840684e2,
    3.01972536e2,
    3.02104323e2,
    3.02236043e2,
    3.02367698e2,
    3.02499288e2,
    3.02630812e2,
    3.02762271e2,
    3.02893664e2,
    3.03024992e2,
    3.03201247e2,
    3.03568405e2,
    3.03935095e2,
    3.04301321e2,
    3.04667084e2,
    3.05032386e2,
    3.05397227e2,
    3.05761611e2,
    3.06125538e2,
    3.06489010e2,
    3.06852029e2,
    3.07214596e2,
    3.07576714e2,
    3.07938382e2,
    3.08299604e2,
    3.08660381e2,
    3.09020714e2,
    3.09380605e2,
    3.09740055e2,
    3.10099066e2,
    3.10457639e2,
    3.10815776e2,
    3.11173479e2,
    3.11530749e2,
    3.11887587e2,
    3.12243995e2,
    3.12599974e2,
    3.12955527e2,
    3.13310653e2,
    3.13665356e2,
    3.14019635e2,
    3.14373493e2,
    3.14726931e2,
    3.15079951e2,
    3.15432554e2,
    3.15784741e2,
    3.16136513e2,
    3.16487873e2,
    3.16838821e2,
    3.17189358e2,
    3.17539487e2,
    3.17889209e2,
    3.18238524e2,
    3.18587434e2,
    3.18935941e2,
    3.19284046e2,
    3.19631750e2,
    3.19979055e2,
    3.20325961e2,
    3.20672470e2,
    3.21018584e2,
    3.21364303e2,
    3.21709629e2,
    3.22054563e2,
    3.22399107e2,
    3.22743262e2,
    3.23087028e2,
    3.23430408e2,
    3.23773401e2,
    3.24116011e2,
    3.24458237e2,
    3.24800082e2,
    3.25141546e2,
    3.25482630e2,
    3.25823336e2,
    3.26163665e2,
    3.26503617e2,
    3.26843195e2,
    3.27182400e2,
    3.27521231e2,
    3.27859692e2,
    3.28197782e2,
    3.28535504e2,
    3.28872857e2,
    3.29209844e2,
    3.29546465e2,
    3.29798847e2,
    3.29798847e2,
    3.29798847e2,
    3.29798847e2,
    3.29798847e2,
    3.29798847e2,
    3.29798847e2,
    3.29798847e2,
    3.29798847e2,
    3.29798847e2,
    3.29798847e2,
    3.29798847e2,
    3.29798847e2,
    3.29798847e2,
    3.29798847e2,
    3.29798847e2,
    3.29798847e2,
    3.29798847e2,
    3.29798847e2,
    3.29798847e2,
    3.29798847e2,
    3.29483916e2,
    3.29147714e2,
    3.28811190e2,
    3.28474342e2,
    3.28137169e2,
    3.27799670e2,
    3.27461844e2,
    3.27123691e2,
    3.26785209e2,
    3.26446397e2,
    3.26107255e2,
    3.25767780e2,
    3.25427973e2,
    3.25087832e2,
    3.24747355e2,
    3.24406543e2,
    3.24065393e2,
    3.23723905e2,
    3.23382078e2,
    3.23039911e2,
    3.22697402e2,
    3.22354551e2,
    3.22011356e2,
    3.21667816e2,
    3.21323931e2,
    3.20979698e2,
    3.20635118e2,
    3.20290188e2,
    3.19944908e2,
    3.19599276e2,
    3.19253292e2,
    3.18906954e2,
    3.18560262e2,
    3.18213213e2,
    3.17865807e2,
    3.17518042e2,
    3.17169918e2,
    3.16821433e2,
    3.16472587e2,
    3.16123377e2,
    3.15773802e2,
    3.15423862e2,
    3.15073555e2,
    3.14722881e2,
    3.14371837e2,
    3.14020422e2,
    3.13668636e2,
    3.13316476e2,
    3.12963943e2,
    3.12611033e2,
    3.12257747e2,
    3.11904083e2,
    3.11550040e2,
    3.11195615e2,
    3.10840809e2,
    3.10485620e2,
    3.10130046e2,
    3.09774086e2,
    3.09417738e2,
    3.09061002e2,
    3.08703876e2,
    3.08346359e2,
    3.07988449e2,
    3.07630145e2,
    3.07271445e2,
    3.06912349e2,
    3.06552854e2,
    3.06192960e2,
    3.05832665e2,
    3.05471967e2,
    3.05110865e2,
    3.04749358e2,
    3.04387444e2,
    3.04025122e2,
    3.03662390e2,
    3.03299247e2,
    3.02935692e2,
    3.02571722e2,
    3.02207336e2,
    3.01842534e2,
    3.01477313e2,
    3.01111671e2,
    3.00745608e2,
    3.00379121e2,
    3.00012210e2,
    2.99644872e2,
    2.99277107e2,
    2.98908911e2,
    2.98540285e2,
    2.98171226e2,
    2.97801732e2,
    2.97431803e2,
    2.97061436e2,
    2.96690630e2,
    2.96319382e2,
    2.95947693e2,
    2.95575559e2,
    2.95202979e2,
    2.94829951e2,
    2.94456475e2,
    2.94082547e2,
    2.93708166e2,
    2.93439420e2,
    2.93171501e2,
    2.92903354e2,
    2.92634977e2,
    2.92366371e2,
    2.92097534e2,
    2.91828467e2,
    2.91559168e2,
    2.91289637e2,
    2.91019873e2,
    2.90749876e2,
    2.90479644e2,
    2.90209178e2,
    2.89938476e2,
    2.89667538e2,
    2.89396363e2,
    2.89124951e2,
    2.88853301e2,
    2.88581412e2,
    2.88309283e2,
    2.88036914e2,
    2.87764305e2,
    2.87491454e2,
    2.87218360e2,
    2.86945024e2,
    2.86671444e2,
    2.86397620e2,
    2.86123551e2,
    2.85849236e2,
    2.85574675e2,
    2.85299867e2,
    2.85024810e2,
    2.84749506e2,
    2.84473951e2,
    2.84198147e2,
    2.83922092e2,
    2.83645786e2,
    2.83369227e2,
    2.83092416e2,
    2.82815351e2,
    2.82538031e2,
    2.82260456e2,
    2.81982625e2,
    2.81704538e2,
    2.81426193e2,
    2.81147589e2,
    2.80868727e2,
    2.80589605e2,
    2.80310222e2,
    2.80030578e2,
    2.79750672e2,
    2.79470503e2,
    2.79190070e2,
    2.78909372e2,
    2.78628410e2,
    2.78347181e2,
    2.78065685e2,
    2.77783922e2,
    2.77501889e2,
    2.77219588e2,
    2.76937016e2,
    2.76654174e2,
    2.76371059e2,
    2.76087672e2,
    2.75804011e2,
    2.75520076e2,
    2.75235865e2,
    2.74951379e2,
    2.74666615e2,
    2.74381574e2,
    2.74096254e2,
    2.74096254e2,
    2.74096254e2,
    2.74096254e2,
    2.74096254e2,
    2.74096254e2,
    2.74096254e2,
    2.74096254e2,
    2.74096254e2,
    2.74096254e2,
    2.74096254e2,
    2.74096254e2,
    2.74096254e2,
    2.74096254e2,
    2.74096254e2,
    2.74096254e2,
    2.74096254e2,
    2.74096254e2,
    2.74096254e2,
    2.74096254e2,
    2.74096254e2,
    2.74096254e2,
    2.74096254e2,
    2.74096254e2,
    2.74096254e2,
    2.74096254e2,
    2.74096254e2,
    2.74096254e2,
    2.74096254e2,
    2.74096254e2,
    2.74096254e2,
    2.74096254e2,
    2.74096254e2,
    2.74096254e2,
    2.74096254e2,
    2.74096254e2,
    2.74096254e2,
    2.74096254e2,
    2.74096254e2,
    2.74096254e2,
    2.74096254e2,
    2.74096254e2,
    2.74096254e2,
    2.74096254e2,
    2.74096254e2,
    2.74096254e2,
    2.74096254e2,
    2.74096254e2,
    2.74096254e2,
    2.74096254e2,
    2.74096254e2,
    2.74096254e2,
    2.74096254e2,
    2.74096254e2,
    2.74096254e2,
    2.74096254e2,
    2.74096254e2,
    2.74096254e2,
    2.74096254e2,
    2.74096254e2,
    2.74096254e2,
    2.74096254e2,
    2.74096254e2,
    2.74096254e2,
    2.74096254e2,
    2.74096254e2,
    2.74096254e2,
    2.74096254e2,
    2.74096254e2,
    2.74096254e2,
    2.74096254e2,
    2.74096254e2,
    2.74096254e2,
    2.74096254e2,
    2.74096254e2,
    2.74096254e2,
    2.74096254e2,
    2.74096254e2,
    2.74096254e2,
    2.74096254e2,
    2.74096254e2,
    2.74096254e2,
    2.74096254e2,
    2.74096254e2,
    2.74096254e2,
    2.74096254e2,
    2.74096254e2,
    2.74096254e2,
    2.74096254e2,
    2.74096254e2,
    2.74096254e2,
    2.74096254e2,
    2.74096254e2,
    2.74096254e2,
    2.74096254e2,
    2.74096254e2,
    2.74096254e2,
    2.74096254e2,
    2.74096254e2,
    2.74096254e2,
    2.74096254e2,
    2.74096254e2,
    2.74096254e2,
    2.74096254e2,
    2.74096254e2,
    2.74096254e2,
    2.74096254e2,
    2.74096254e2,
    2.74096254e2,
    2.74096254e2,
    2.74096254e2,
    2.74096254e2,
    2.74096254e2,
    2.74096254e2,
    2.74096254e2,
    2.74096254e2,
    2.74096254e2,
    2.74096254e2,
    2.74096254e2,
    2.74096254e2,
    2.74096254e2,
    2.74096254e2,
    2.74096254e2,
    2.74096254e2,
    2.74096254e2,
    2.74096254e2,
    2.74096254e2,
    2.74096254e2,
    2.74096254e2,
    2.74096254e2,
    2.74096254e2,
    2.74096254e2,
    2.74096254e2,
    2.74096254e2,
    2.74096254e2,
    2.74096254e2,
    2.74096254e2,
    2.74096254e2,
    2.74096254e2,
    2.74096254e2,
    2.74096254e2,
    2.74096254e2,
    2.74096254e2,
    2.74096254e2,
    2.74096254e2,
    2.74096254e2,
    2.74096254e2,
    2.74096254e2,
    2.74096254e2,
    2.74096254e2,
    2.74096254e2,
    2.74096254e2,
    2.74096254e2,
    2.74096254e2,
    2.74096254e2,
    2.74096254e2,
    2.74096254e2,
    2.74096254e2,
    2.74096254e2,
    2.74096254e2,
    2.74096254e2,
    2.74096254e2,
    2.74096254e2,
    2.74096254e2,
    2.74096254e2,
    2.74096254e2,
    2.74096254e2,
    2.74096254e2,
    2.74096254e2,
    2.74096254e2,
    2.74096254e2,
    2.74096254e2,
    2.74096254e2,
    2.74096254e2,
    2.74096254e2,
    2.74096254e2,
    2.74096254e2,
    2.74096254e2,
    2.74096254e2,
    2.74096254e2,
    2.74096254e2,
    2.74096254e2,
    2.74096254e2,
    2.74096254e2,
    2.74096254e2,
    2.74096254e2,
    2.74096254e2,
    2.74096254e2,
    2.74096254e2,
    2.74096254e2,
    2.74096254e2,
    2.74096254e2,
    2.74096254e2,
    2.74096254e2,
    2.74096254e2,
    2.74096254e2,
    2.74096254e2,
    2.74096254e2,
    2.74096254e2,
    2.74096254e2,
    2.74096254e2,
    2.74096254e2,
    2.74096254e2,
    2.74096254e2,
    2.74096254e2,
    2.74096254e2,
    2.74096254e2,
    2.74096254e2,
    2.74096254e2,
    2.74096254e2,
    2.74096254e2,
    2.74096254e2,
    2.74096254e2,
    2.74096254e2,
    2.74096254e2,
    2.74096254e2,
    2.74096254e2,
    2.74096254e2,
    2.74096254e2,
    2.74096254e2,
    2.74096254e2,
    2.74096254e2,
    2.74096254e2,
    2.74096254e2,
    2.74096254e2,
    2.74096254e2,
    2.74096254e2,
    2.74096254e2,
    2.74096254e2,
    2.74096254e2,
    2.74096254e2,
    2.74096254e2,
    2.74096254e2,
    2.74096254e2,
    2.74096254e2,
    2.74096254e2,
    2.74096254e2,
    2.74096254e2,
    2.74096254e2,
    2.74096254e2,
    2.74096254e2,
    2.74096254e2,
    2.74096254e2,
    2.74096254e2,
    2.74096254e2,
    2.74096254e2,
    2.74096254e2,
    2.74096254e2,
    2.74096254e2,
    2.74096254e2,
    2.74096254e2,
    2.74096254e2,
    2.74096254e2,
    2.74096254e2,
    2.74096254e2,
    2.74096254e2,
    2.74096254e2,
    2.74096254e2,
    2.74096254e2,
    2.74096254e2,
    2.74096254e2,
    2.74096254e2,
    2.74096254e2,
    2.74096254e2,
    2.74096254e2,
    2.74096254e2,
    2.74096254e2,
    2.74096254e2,
    2.74096254e2,
    2.74096254e2,
    2.74096254e2,
];

/// Entries in the drag table read by altitude.
pub const DRAG_ALTITUDE_ENTRIES: usize = 7;
/// Entries in the drag table read by Mach number.
pub const DRAG_MACH_ENTRIES: usize = 19;
/// Altitudes of the thin-air drag table. Aerodynamics paper Table 1 (p.2),
/// and the first row of Table 4 (p.4).
pub const DRAG_ALTITUDE_KM: [f64; DRAG_ALTITUDE_ENTRIES] =
    [68.96, 76.0, 83.7, 92.0, 100.9, 120.5, 134.7];
/// Drag coefficient at those altitudes, zero angle of attack.
pub const DRAG_BY_ALTITUDE: [f64; DRAG_ALTITUDE_ENTRIES] =
    [1.4959, 1.525, 1.543, 1.64, 1.822, 1.974, 2.019];
/// Mach numbers of the dense-air drag table. Mach 7.15 and up: aerodynamics
/// paper Table 4 (p.4). Below: read from the graph of Figure 15 (p.10). The
/// card reads this table from Mach 1.0 up.
pub const DRAG_MACH: [f64; DRAG_MACH_ENTRIES] = [
    0.16, 0.59, 0.76, 0.85, 1.0, 1.155, 1.31, 1.67, 2.22, 3.03, 4.0, 7.15, 8.5, 10.5, 12.2, 17.1,
    24.7, 35.4, 40.0,
];
/// Drag coefficient at those Mach numbers.
pub const DRAG_BY_MACH: [f64; DRAG_MACH_ENTRIES] = [
    0.876, 0.923, 0.959, 1.0, 1.094, 1.328, 1.373, 1.45, 1.509, 1.495, 1.462, 1.506, 1.51, 1.515,
    1.4992, 1.4939, 1.4816, 1.4828, 1.4959,
];
/// Drag is read by altitude at and above this, by Mach number below it.
pub const DRAG_SWITCH_ALTITUDE_M: f64 = 68_960.0;

/// Pre-flight requirement on the Mach number at drogue deployment. Operations
/// paper p.4 Table 1. Advisory only.
pub const DROGUE_MACH_REQUIRED_MIN: f64 = 1.2;
/// Upper end of the same requirement.
pub const DROGUE_MACH_REQUIRED_MAX: f64 = 1.6;
/// Radar altitude when the drogue opened. Reconstruction paper p.5. An
/// incoming state under it and under [`FLIGHT_DROGUE_MACH`] is refused.
pub const FLIGHT_DROGUE_ALTITUDE_M: f64 = 31_030.0;
/// Mach number from the radar data when the drogue opened. Reconstruction
/// paper p.5. An incoming state under it and under [`FLIGHT_DROGUE_ALTITUDE_M`]
/// is refused.
pub const FLIGHT_DROGUE_MACH: f64 = 1.23;

/// Constant of the reported heat flux, `k sqrt(ρ / R_n) u³`, SI units.
pub const HEAT_FLUX_CONSTANT: f64 = 1.7415e-4;
/// Nose radius used by the reported heat flux. Three radii are in print.
pub const NOSE_RADIUS_M: f64 = 0.23;
/// Ratio of specific heats of the air, the 1976 standard's, from which its
/// speed of sound is made: the static pressure `ρ c² / γ` and the
/// stagnation pressure.
pub const RATIO_OF_SPECIFIC_HEATS: f64 = 1.4;

/// Half the square of 12 900 m/s. Scale of the residual.
pub const ENERGY_SCALE_J_KG: f64 = 83_205_000.0;
/// `|J − J_anchor + Q| / ENERGY_SCALE_J_KG` must stay at or under this.
pub const RESIDUAL_CONSERVATION_LIMIT: f64 = 1.0e-9;
/// Largest step the card stands behind. At the locked entry state the capsule
/// falls 92 m in one such step, under half a knot interval of the air tables,
/// and over the whole entry the residual stays under a thousandth of its limit.
pub const GATE_DT_S: f64 = 0.05;
/// Step ceiling divided by step floor: 2^20.
pub const GATE_OVER_FLOOR: f64 = 1_048_576.0;

/// Mach number is from 1.2 to 1.6 on this frame.
pub const FLAG_DROGUE_MACH_WINDOW: u64 = 1 << 0;

/// Why a step did not advance the capsule or the frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StepReject {
    /// `dt` was NaN or infinite.
    NonFiniteInput,
    /// The incoming state, or the trial step, was non-finite.
    NonFiniteState,
    /// `dt` was zero or negative.
    NonPositiveDt,
    /// A recorded constant or table entry is not this capsule.
    InvalidCapsule,
    /// Altitude, incoming, at a stage of the step, or landing, is above 134.7 km, the top of the drag table read by altitude.
    AltitudeOutsideCard,
    /// Altitude, incoming, at a stage of the step, or landing, is below 0 m, the sea level of the 1976 standard's air and the ellipsoid under the path: the capsule is on or under the ground.
    BelowStandardSeaLevel,
    /// Mach number, incoming, at a stage of the step, or landing, is below 0.16, the first entry of the drag table read by Mach number, or is above 40 below 68.96 km, where that table ends.
    MachOutsideCard,
    /// Air speed, incoming, at a stage of the step, or landing, above 68.96 km is below 11900 m/s or above 12620 m/s, the highest speed the drag table by altitude was computed at.
    AirSpeedOutsideCard,
    /// The incoming state is under 31030 m and under Mach 1.23 at once: past the flown drogue deployment.
    PastFlightDeployment,
    /// The trial step gained energy that drag did not account for.
    SpontaneousGeneration,
    /// The trial step lost more energy than drag accounts for.
    FirstLawBreach,
}

/// Nothing acts on the capsule that is not in the card. The load is empty.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct CapsuleLoad {}

/// The locked capsule, its air, and its Earth. The tables are recorded copies.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CapsuleStardust {
    pub capsule_mass_kg: f64,
    pub reference_area_m2: f64,
    pub entry_radius_m: f64,
    pub entry_radial_speed_m_s: f64,
    pub entry_transverse_speed_m_s: f64,
    pub local_earth_radius_m: f64,
    pub earth_gravitational_parameter_m3_s2: f64,
    pub air_rotation_rate_rad_s: f64,
    pub standard_gravity_m_s2: f64,
    pub altitude_ceiling_m: f64,
    pub altitude_floor_m: f64,
    pub mach_floor: f64,
    pub air_speed_band_min_m_s: f64,
    pub air_speed_band_max_m_s: f64,
    pub sea_level_m: f64,
    pub table_knot_spacing_m: f64,
    pub air_density_kg_m3: [f64; KNOTS],
    pub sound_speed_m_s: [f64; KNOTS],
    pub air_density_low_kg_m3: [f64; LOW_KNOTS],
    pub sound_speed_low_m_s: [f64; LOW_KNOTS],
    pub drag_altitude_km: [f64; DRAG_ALTITUDE_ENTRIES],
    pub drag_by_altitude: [f64; DRAG_ALTITUDE_ENTRIES],
    pub drag_mach: [f64; DRAG_MACH_ENTRIES],
    pub drag_by_mach: [f64; DRAG_MACH_ENTRIES],
    pub drag_switch_altitude_m: f64,
    pub drogue_mach_required_min: f64,
    pub drogue_mach_required_max: f64,
    pub flight_drogue_altitude_m: f64,
    pub flight_drogue_mach: f64,
    pub heat_flux_constant: f64,
    pub nose_radius_m: f64,
    pub energy_scale_j_kg: f64,
    pub residual_conservation_limit: f64,
}

impl CapsuleStardust {
    /// The Stardust sample return capsule. The constructor takes no arguments.
    #[must_use]
    pub fn sample_return_capsule() -> Self {
        let capsule = Self {
            capsule_mass_kg: CAPSULE_MASS_KG,
            reference_area_m2: REFERENCE_AREA_M2,
            entry_radius_m: ENTRY_RADIUS_M,
            entry_radial_speed_m_s: ENTRY_RADIAL_SPEED_M_S,
            entry_transverse_speed_m_s: ENTRY_TRANSVERSE_SPEED_M_S,
            local_earth_radius_m: LOCAL_EARTH_RADIUS_M,
            earth_gravitational_parameter_m3_s2: EARTH_GRAVITATIONAL_PARAMETER_M3_S2,
            air_rotation_rate_rad_s: AIR_ROTATION_RATE_RAD_S,
            standard_gravity_m_s2: STANDARD_GRAVITY_M_S2,
            altitude_ceiling_m: ALTITUDE_CEILING_M,
            altitude_floor_m: ALTITUDE_FLOOR_M,
            mach_floor: MACH_FLOOR,
            air_speed_band_min_m_s: AIR_SPEED_BAND_MIN_M_S,
            air_speed_band_max_m_s: AIR_SPEED_BAND_MAX_M_S,
            sea_level_m: SEA_LEVEL_M,
            table_knot_spacing_m: TABLE_KNOT_SPACING_M,
            air_density_kg_m3: AIR_DENSITY_KG_M3,
            sound_speed_m_s: SOUND_SPEED_M_S,
            air_density_low_kg_m3: AIR_DENSITY_LOW_KG_M3,
            sound_speed_low_m_s: SOUND_SPEED_LOW_M_S,
            drag_altitude_km: DRAG_ALTITUDE_KM,
            drag_by_altitude: DRAG_BY_ALTITUDE,
            drag_mach: DRAG_MACH,
            drag_by_mach: DRAG_BY_MACH,
            drag_switch_altitude_m: DRAG_SWITCH_ALTITUDE_M,
            drogue_mach_required_min: DROGUE_MACH_REQUIRED_MIN,
            drogue_mach_required_max: DROGUE_MACH_REQUIRED_MAX,
            flight_drogue_altitude_m: FLIGHT_DROGUE_ALTITUDE_M,
            flight_drogue_mach: FLIGHT_DROGUE_MACH,
            heat_flux_constant: HEAT_FLUX_CONSTANT,
            nose_radius_m: NOSE_RADIUS_M,
            energy_scale_j_kg: ENERGY_SCALE_J_KG,
            residual_conservation_limit: RESIDUAL_CONSERVATION_LIMIT,
        };
        if capsule.is_valid() {
            capsule
        } else {
            ::core::panic!("Stardust sample return capsule failed is_valid");
        }
    }

    #[must_use]
    pub fn is_valid(&self) -> bool {
        if !same_bits(self.capsule_mass_kg, CAPSULE_MASS_KG)
            || !same_bits(self.reference_area_m2, REFERENCE_AREA_M2)
            || !same_bits(self.entry_radius_m, ENTRY_RADIUS_M)
            || !same_bits(self.entry_radial_speed_m_s, ENTRY_RADIAL_SPEED_M_S)
            || !same_bits(self.entry_transverse_speed_m_s, ENTRY_TRANSVERSE_SPEED_M_S)
            || !same_bits(self.local_earth_radius_m, LOCAL_EARTH_RADIUS_M)
            || !same_bits(
                self.earth_gravitational_parameter_m3_s2,
                EARTH_GRAVITATIONAL_PARAMETER_M3_S2,
            )
            || !same_bits(self.air_rotation_rate_rad_s, AIR_ROTATION_RATE_RAD_S)
            || !same_bits(self.standard_gravity_m_s2, STANDARD_GRAVITY_M_S2)
            || !same_bits(self.altitude_ceiling_m, ALTITUDE_CEILING_M)
            || !same_bits(self.altitude_floor_m, ALTITUDE_FLOOR_M)
            || !same_bits(self.mach_floor, MACH_FLOOR)
            || !same_bits(self.air_speed_band_min_m_s, AIR_SPEED_BAND_MIN_M_S)
            || !same_bits(self.air_speed_band_max_m_s, AIR_SPEED_BAND_MAX_M_S)
            || !same_bits(self.sea_level_m, SEA_LEVEL_M)
            || !same_bits(self.table_knot_spacing_m, TABLE_KNOT_SPACING_M)
            || !same_bits(self.drag_switch_altitude_m, DRAG_SWITCH_ALTITUDE_M)
            || !same_bits(self.drogue_mach_required_min, DROGUE_MACH_REQUIRED_MIN)
            || !same_bits(self.drogue_mach_required_max, DROGUE_MACH_REQUIRED_MAX)
            || !same_bits(self.flight_drogue_altitude_m, FLIGHT_DROGUE_ALTITUDE_M)
            || !same_bits(self.flight_drogue_mach, FLIGHT_DROGUE_MACH)
            || !same_bits(self.heat_flux_constant, HEAT_FLUX_CONSTANT)
            || !same_bits(self.nose_radius_m, NOSE_RADIUS_M)
            || !same_bits(self.energy_scale_j_kg, ENERGY_SCALE_J_KG)
            || !same_bits(
                self.residual_conservation_limit,
                RESIDUAL_CONSERVATION_LIMIT,
            )
        {
            return false;
        }
        let mut index = 0;
        while index < KNOTS {
            if !same_bits(self.air_density_kg_m3[index], AIR_DENSITY_KG_M3[index])
                || !same_bits(self.sound_speed_m_s[index], SOUND_SPEED_M_S[index])
            {
                return false;
            }
            if !(self.air_density_kg_m3[index] > 0.0 && self.sound_speed_m_s[index] > 0.0) {
                return false;
            }
            if index > 0 && !(self.air_density_kg_m3[index] < self.air_density_kg_m3[index - 1]) {
                return false;
            }
            index += 1;
        }
        let mut index = 0;
        while index < DRAG_ALTITUDE_ENTRIES {
            if !same_bits(self.drag_altitude_km[index], DRAG_ALTITUDE_KM[index])
                || !same_bits(self.drag_by_altitude[index], DRAG_BY_ALTITUDE[index])
            {
                return false;
            }
            if !(self.drag_by_altitude[index] > 0.0) {
                return false;
            }
            if index > 0 && !(self.drag_altitude_km[index] > self.drag_altitude_km[index - 1]) {
                return false;
            }
            index += 1;
        }
        let mut index = 0;
        while index < DRAG_MACH_ENTRIES {
            if !same_bits(self.drag_mach[index], DRAG_MACH[index])
                || !same_bits(self.drag_by_mach[index], DRAG_BY_MACH[index])
            {
                return false;
            }
            if !(self.drag_by_mach[index] > 0.0) {
                return false;
            }
            if index > 0 && !(self.drag_mach[index] > self.drag_mach[index - 1]) {
                return false;
            }
            index += 1;
        }
        let mut index = 0;
        while index < LOW_KNOTS {
            if !same_bits(self.air_density_low_kg_m3[index], AIR_DENSITY_LOW_KG_M3[index])
                || !same_bits(self.sound_speed_low_m_s[index], SOUND_SPEED_LOW_M_S[index])
            {
                return false;
            }
            if index > 0
                && !(self.air_density_low_kg_m3[index] < self.air_density_low_kg_m3[index - 1])
            {
                return false;
            }
            index += 1;
        }
        // The low tables end where the tables of the record begin, bit for bit.
        if !(same_bits(self.air_density_low_kg_m3[LOW_KNOTS - 1], self.air_density_kg_m3[0])
            && same_bits(self.sound_speed_low_m_s[LOW_KNOTS - 1], self.sound_speed_m_s[0])
            && same_bits(
                self.sea_level_m + ((LOW_KNOTS - 1) as f64) * self.table_knot_spacing_m,
                self.altitude_floor_m,
            ))
        {
            return false;
        }
        let top_knot_m = self.altitude_floor_m + ((KNOTS - 1) as f64) * self.table_knot_spacing_m;
        let entry_altitude_m = self.entry_radius_m - self.local_earth_radius_m;
        // The air tables reach past the ceiling, the altitude drag table spans
        // the switch altitude to the ceiling exactly, and the Mach drag table
        // starts at or below the Mach floor. So no read can fall off a table's end.
        top_knot_m > self.altitude_ceiling_m
            && same_bits(
                self.drag_altitude_km[DRAG_ALTITUDE_ENTRIES - 1] * 1000.0,
                self.altitude_ceiling_m,
            )
            && same_bits(
                self.drag_altitude_km[0] * 1000.0,
                self.drag_switch_altitude_m,
            )
            && self.drag_mach[0] <= self.mach_floor
            && self.air_speed_band_min_m_s < self.air_speed_band_max_m_s
            && entry_altitude_m > self.altitude_floor_m
            && entry_altitude_m < self.altitude_ceiling_m
            && self.capsule_mass_kg > 0.0
            && self.reference_area_m2 > 0.0
            && self.nose_radius_m > 0.0
            && self.energy_scale_j_kg > 0.0
            && self.residual_conservation_limit > 0.0
    }

    /// [`GATE_DT_S`]. Not applied by [`step`].
    #[must_use]
    pub fn gate_dt_s(&self) -> Option<f64> {
        if self.is_valid() {
            Some(GATE_DT_S)
        } else {
            None
        }
    }

    /// Smallest step the card stands behind: [`GATE_DT_S`] divided by 2^20,
    /// 4.8e-8 s. At the ceiling the capsule falls 92 m at the locked entry
    /// state. A million times smaller it falls 88 micrometres, which the
    /// 64-bit radius, 6503 km, still resolves to five figures. Under that a
    /// step is mostly rounding. Not applied by [`step`].
    #[must_use]
    pub fn floor_dt_s(&self) -> Option<f64> {
        match self.gate_dt_s() {
            Some(gate_dt_s) => Some(gate_dt_s / GATE_OVER_FLOOR),
            None => None,
        }
    }
}

/// Integrated capsule. The ledger fields are part of the state; write them only through [`step`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CapsuleState {
    /// Time since the state the ledger started from.
    pub t_s: f64,
    /// Distance from the Earth's centre.
    pub radius_m: f64,
    /// Negative going down. Fixed frame.
    pub radial_speed_m_s: f64,
    /// Fixed frame.
    pub transverse_speed_m_s: f64,
    /// Angle travelled about the Earth's centre. Fixed frame.
    pub angle_rad: f64,
    /// Energy per kilogram removed by drag, integrated by the step.
    pub drag_loss_j_kg: f64,
    /// `J` at the state the ledger started from.
    pub energy_anchor_j_kg: f64,
    /// Signed `J − J_anchor + Q`. Positive is created energy.
    pub energy_defect_j_kg: f64,
    pub residual_conservation_error: f64,
    pub altitude_m: f64,
    /// `angle_rad × LOCAL_EARTH_RADIUS_M`.
    pub ground_distance_m: f64,
    pub air_speed_m_s: f64,
    pub mach: f64,
    /// Drag acceleration in Earth g.
    pub load_g: f64,
    /// Reported only.
    pub heat_flux_w_m2: f64,
    /// Pressure at the stagnation point, Pa. Reported only.
    pub stagnation_pressure_pa: f64,
    /// Recovery enthalpy at the stagnation point, `u²/2`, J/kg. Reported only.
    pub recovery_enthalpy_j_kg: f64,
    /// Heat-transfer coefficient `ρ_e u_e C_H` of the reported heat flux,
    /// kg/m²/s: the heat flux over the recovery enthalpy. Reported only.
    pub heat_transfer_coefficient_kg_m2_s: f64,
    pub drogue_mach_window: bool,
}

impl CapsuleState {
    /// The locked entry state: radius 6503.14 km, 12.9 km/s, −8.21°.
    #[must_use]
    pub fn try_entry_interface(capsule: &CapsuleStardust) -> Option<Self> {
        Self::try_state(
            capsule,
            capsule.entry_radius_m,
            capsule.entry_radial_speed_m_s,
            capsule.entry_transverse_speed_m_s,
        )
    }

    /// Anchor the ledger at this radius and these two speeds. `None` where
    /// [`CapsuleState::anchor`] refuses.
    #[must_use]
    pub fn try_state(
        capsule: &CapsuleStardust,
        radius_m: f64,
        radial_speed_m_s: f64,
        transverse_speed_m_s: f64,
    ) -> Option<Self> {
        match Self::anchor(capsule, radius_m, radial_speed_m_s, transverse_speed_m_s) {
            Ok(state) => Some(state),
            Err(_) => None,
        }
    }

    /// Anchor the ledger at this radius and these two speeds, or say why not.
    /// The state must be one a step would take: altitude from sea level to
    /// 134.7 km, Mach number at least 0.16 and at most 40 below 68.96 km, air
    /// speed from 11 900 to 12 620 m/s above 68.96 km, and not under 31 030 m
    /// and under Mach 1.23 at once.
    pub fn anchor(
        capsule: &CapsuleStardust,
        radius_m: f64,
        radial_speed_m_s: f64,
        transverse_speed_m_s: f64,
    ) -> Result<Self, StepReject> {
        if !capsule.is_valid() {
            return Err(StepReject::InvalidCapsule);
        }
        if !radius_m.is_finite() || !radial_speed_m_s.is_finite() || !transverse_speed_m_s.is_finite()
        {
            return Err(StepReject::NonFiniteState);
        }
        let air = air_at(capsule, radius_m, radial_speed_m_s, transverse_speed_m_s)?;
        if past_flight_deployment(capsule, &air) {
            return Err(StepReject::PastFlightDeployment);
        }
        let energy =
            rotating_frame_energy_j_kg(capsule, radius_m, radial_speed_m_s, transverse_speed_m_s);
        let mut state = Self {
            t_s: 0.0,
            radius_m,
            radial_speed_m_s,
            transverse_speed_m_s,
            angle_rad: 0.0,
            drag_loss_j_kg: 0.0,
            energy_anchor_j_kg: energy,
            energy_defect_j_kg: 0.0,
            residual_conservation_error: 0.0,
            altitude_m: 0.0,
            ground_distance_m: 0.0,
            air_speed_m_s: 0.0,
            mach: 0.0,
            load_g: 0.0,
            heat_flux_w_m2: 0.0,
            stagnation_pressure_pa: 0.0,
            recovery_enthalpy_j_kg: 0.0,
            heat_transfer_coefficient_kg_m2_s: 0.0,
            drogue_mach_window: false,
        };
        refresh(capsule, &mut state, &air);
        if state_is_finite(&state) {
            Ok(state)
        } else {
            Err(StepReject::NonFiniteState)
        }
    }
}

/// Advance one RK4 step and write the frame.
///
/// The capsule is driven by the clock alone; `load` is empty. A rejected step
/// leaves `state` and `frame` as they were.
///
/// A value of the landed state below the normal range is written as zero
/// with its sign ([`crate::normal`]).
#[must_use]
pub fn step(
    capsule: &CapsuleStardust,
    state: &mut CapsuleState,
    frame: &mut LastStateFrame64,
    load: CapsuleLoad,
    dt_s: f64,
) -> Result<(), StepReject> {
    #[cfg(any(test, debug_assertions, feature = "check_exceedances"))]
    let before = *state;
    let stepped = step_landed(capsule, state, frame, load, dt_s);
    #[cfg(any(test, debug_assertions, feature = "check_exceedances"))]
    if stepped.is_ok() {
        crate::exceedance::debug_check("capsule_stardust", exceedances(capsule, &before, state, load, dt_s), frame.flags);
    }
    stepped
}

/// The step itself; [`step`] is it with, in a build that checks
/// (`crate::exceedance::CHECKED`), the check that [`exceedances`] stands for
/// the flags it wrote.
fn step_landed(
    capsule: &CapsuleStardust,
    state: &mut CapsuleState,
    frame: &mut LastStateFrame64,
    load: CapsuleLoad,
    dt_s: f64,
) -> Result<(), StepReject> {
    step_as_computed(capsule, state, frame, load, dt_s)?;
    normal_or_zero_state(state);
    Ok(())
}

/// The step as its equations compute it, before a value of the landed
/// state below the normal range is written as zero.
fn step_as_computed(
    capsule: &CapsuleStardust,
    state: &mut CapsuleState,
    frame: &mut LastStateFrame64,
    load: CapsuleLoad,
    dt_s: f64,
) -> Result<(), StepReject> {
    validate(capsule, state, load, dt_s)?;
    let predicted = integrate(capsule, recorded(state), dt_s)?;
    commit(capsule, state, frame, dt_s, predicted)
}

/// Each value of a landed state below the normal range written as zero
/// with its sign ([`crate::normal`]): the state carried to the next step
/// and published holds no subnormal number.
fn normal_or_zero_state(state: &mut CapsuleState) {
    state.t_s = normal_or_zero(state.t_s);
    state.radius_m = normal_or_zero(state.radius_m);
    state.radial_speed_m_s = normal_or_zero(state.radial_speed_m_s);
    state.transverse_speed_m_s = normal_or_zero(state.transverse_speed_m_s);
    state.angle_rad = normal_or_zero(state.angle_rad);
    state.drag_loss_j_kg = normal_or_zero(state.drag_loss_j_kg);
    state.energy_anchor_j_kg = normal_or_zero(state.energy_anchor_j_kg);
    state.energy_defect_j_kg = normal_or_zero(state.energy_defect_j_kg);
    state.residual_conservation_error = normal_or_zero(state.residual_conservation_error);
    state.altitude_m = normal_or_zero(state.altitude_m);
    state.ground_distance_m = normal_or_zero(state.ground_distance_m);
    state.air_speed_m_s = normal_or_zero(state.air_speed_m_s);
    state.mach = normal_or_zero(state.mach);
    state.load_g = normal_or_zero(state.load_g);
    state.heat_flux_w_m2 = normal_or_zero(state.heat_flux_w_m2);
    state.stagnation_pressure_pa = normal_or_zero(state.stagnation_pressure_pa);
    state.recovery_enthalpy_j_kg = normal_or_zero(state.recovery_enthalpy_j_kg);
    state.heat_transfer_coefficient_kg_m2_s = normal_or_zero(state.heat_transfer_coefficient_kg_m2_s);
}

fn validate(
    capsule: &CapsuleStardust,
    state: &CapsuleState,
    _load: CapsuleLoad,
    dt_s: f64,
) -> Result<(), StepReject> {
    if !dt_s.is_finite() {
        return Err(StepReject::NonFiniteInput);
    }
    if !state_is_finite(state) {
        return Err(StepReject::NonFiniteState);
    }
    if dt_s <= 0.0 {
        return Err(StepReject::NonPositiveDt);
    }
    if !capsule.is_valid() {
        return Err(StepReject::InvalidCapsule);
    }
    let air = air_at(
        capsule,
        state.radius_m,
        state.radial_speed_m_s,
        state.transverse_speed_m_s,
    )?;
    if past_flight_deployment(capsule, &air) {
        return Err(StepReject::PastFlightDeployment);
    }
    Ok(())
}

/// Under the radar altitude and under the radar Mach number of drogue
/// deployment at once. Both are strict, so a state exactly on either is not past.
fn past_flight_deployment(capsule: &CapsuleStardust, air: &Air) -> bool {
    air.altitude_m < capsule.flight_drogue_altitude_m && air.mach < capsule.flight_drogue_mach
}

/// One classical RK4 step on `(r, v_r, v_t, θ, Q)`. A stage outside the card
/// is refused; no density and no drag is made up outside the tables.
fn integrate(capsule: &CapsuleStardust, y: [f64; 5], dt_s: f64) -> Result<[f64; 5], StepReject> {
    let k1 = rates(capsule, y)?;
    let k2 = rates(capsule, axpy(y, dt_s * 0.5, k1))?;
    let k3 = rates(capsule, axpy(y, dt_s * 0.5, k2))?;
    let k4 = rates(capsule, axpy(y, dt_s, k3))?;
    let mut next = y;
    let mut index = 0;
    while index < 5 {
        let increment = k1[index] + 2.0 * k2[index] + 2.0 * k3[index] + k4[index];
        if increment != 0.0 {
            next[index] += (dt_s / 6.0) * increment;
        }
        index += 1;
    }
    if finite5(next) {
        Ok(next)
    } else {
        Err(StepReject::NonFiniteState)
    }
}

fn commit(
    capsule: &CapsuleStardust,
    state: &mut CapsuleState,
    frame: &mut LastStateFrame64,
    dt_s: f64,
    predicted: [f64; 5],
) -> Result<(), StepReject> {
    let saved = *state;
    if !finite5(predicted) {
        return Err(StepReject::NonFiniteState);
    }
    let air = air_at(capsule, predicted[0], predicted[1], predicted[2])?;
    let scale = capsule.energy_scale_j_kg;
    if !(scale > 0.0 && scale.is_finite()) {
        return Err(StepReject::InvalidCapsule);
    }
    let energy = rotating_frame_energy_j_kg(capsule, predicted[0], predicted[1], predicted[2]);
    let defect = energy - state.energy_anchor_j_kg + predicted[4];
    if !defect.is_finite() {
        return Err(StepReject::NonFiniteState);
    }
    let gate = capsule.residual_conservation_limit * scale;
    if defect > gate {
        return Err(StepReject::SpontaneousGeneration);
    }
    if defect < -gate {
        return Err(StepReject::FirstLawBreach);
    }
    state.radius_m = predicted[0];
    state.radial_speed_m_s = predicted[1];
    state.transverse_speed_m_s = predicted[2];
    state.angle_rad = predicted[3];
    state.drag_loss_j_kg = predicted[4];
    state.energy_defect_j_kg = defect;
    state.residual_conservation_error = abs_f64(defect) / scale;
    state.t_s += dt_s;
    refresh(capsule, state, &air);
    if !state_is_finite(state) {
        *state = saved;
        return Err(StepReject::NonFiniteState);
    }
    publish(state, frame);
    Ok(())
}

/// Write the fields that follow from the phase.
fn refresh(capsule: &CapsuleStardust, state: &mut CapsuleState, air: &Air) {
    let u = air.air_speed_m_s;
    state.altitude_m = air.altitude_m;
    state.ground_distance_m = state.angle_rad * capsule.local_earth_radius_m;
    state.air_speed_m_s = u;
    state.mach = air.mach;
    state.load_g = air.drag_acceleration_m_s2 / capsule.standard_gravity_m_s2;
    state.heat_flux_w_m2 =
        capsule.heat_flux_constant * (air.density_kg_m3 / capsule.nose_radius_m).sqrt() * u * u * u;
    state.stagnation_pressure_pa = stagnation_pressure_pa(air);
    state.recovery_enthalpy_j_kg = 0.5 * u * u;
    state.heat_transfer_coefficient_kg_m2_s =
        2.0 * capsule.heat_flux_constant * (air.density_kg_m3 / capsule.nose_radius_m).sqrt() * u;
    state.drogue_mach_window = air.mach >= capsule.drogue_mach_required_min
        && air.mach <= capsule.drogue_mach_required_max;
}

fn publish(state: &CapsuleState, frame: &mut LastStateFrame64) {
    let mut flags = 0;
    if state.drogue_mach_window {
        flags |= FLAG_DROGUE_MACH_WINDOW;
    }
    frame.t = state.t_s;
    frame.pos[0] = state.altitude_m as f32;
    frame.pos[1] = state.stagnation_pressure_pa as f32;
    frame.pos[2] = state.mach as f32;
    frame.vel[0] = state.recovery_enthalpy_j_kg as f32;
    frame.vel[1] = state.heat_transfer_coefficient_kg_m2_s as f32;
    frame.vel[2] = state.heat_flux_w_m2 as f32;
    frame.force_torque = state.load_g as f32;
    frame.residual = state.residual_conservation_error as f32;
    frame.flags = flags;
}

/// `x^(γ/(γ−1))` for `γ = 1.4`: `x³ sqrt(x)`.
fn power_three_and_a_half(x: f64) -> f64 {
    x * x * x * x.sqrt()
}

/// Pressure at the stagnation point in the card's air: the air brought to
/// rest isentropically below Mach 1, and behind a normal shock and then
/// isentropically at and above it (Rayleigh's pitot formula), for a perfect
/// gas of `γ = 1.4` at the static pressure `ρ c² / γ`. The two meet at Mach 1.
fn stagnation_pressure_pa(air: &Air) -> f64 {
    let gamma = RATIO_OF_SPECIFIC_HEATS;
    let static_pa = air.density_kg_m3 * air.sound_speed_m_s * air.sound_speed_m_s / gamma;
    let m2 = air.mach * air.mach;
    if air.mach < 1.0 {
        static_pa * power_three_and_a_half(1.0 + 0.5 * (gamma - 1.0) * m2)
    } else {
        let shock = (gamma + 1.0) * (gamma + 1.0) * m2 / (4.0 * gamma * m2 - 2.0 * (gamma - 1.0));
        static_pa * power_three_and_a_half(shock) * (1.0 - gamma + 2.0 * gamma * m2) / (gamma + 1.0)
    }
}

/// The air at one phase.
struct Air {
    altitude_m: f64,
    radial_m_s: f64,
    transverse_m_s: f64,
    air_speed_m_s: f64,
    density_kg_m3: f64,
    sound_speed_m_s: f64,
    mach: f64,
    drag_acceleration_m_s2: f64,
}

/// The air the capsule meets at this radius and these two speeds. An
/// altitude, a Mach number or an air speed outside the card is refused by name.
fn air_at(
    capsule: &CapsuleStardust,
    radius_m: f64,
    radial_speed_m_s: f64,
    transverse_speed_m_s: f64,
) -> Result<Air, StepReject> {
    let altitude_m = radius_m - capsule.local_earth_radius_m;
    if !altitude_m.is_finite() || !radial_speed_m_s.is_finite() || !transverse_speed_m_s.is_finite()
    {
        return Err(StepReject::NonFiniteState);
    }
    if !(altitude_m >= capsule.sea_level_m) {
        return Err(StepReject::BelowStandardSeaLevel);
    }
    if !(altitude_m <= capsule.altitude_ceiling_m) {
        return Err(StepReject::AltitudeOutsideCard);
    }
    let radial_m_s = radial_speed_m_s;
    let transverse_m_s = transverse_speed_m_s - capsule.air_rotation_rate_rad_s * radius_m;
    let air_speed_m_s = (radial_m_s * radial_m_s + transverse_m_s * transverse_m_s).sqrt();
    // Below the record's tables, the standard's formulas tabulated from sea
    // level; at and above 30 km, the record's own tables, as before.
    let (density, sound) = if altitude_m < capsule.altitude_floor_m {
        (
            between_low_knots(capsule, &capsule.air_density_low_kg_m3, altitude_m),
            between_low_knots(capsule, &capsule.sound_speed_low_m_s, altitude_m),
        )
    } else {
        (
            between_knots(capsule, &capsule.air_density_kg_m3, altitude_m),
            between_knots(capsule, &capsule.sound_speed_m_s, altitude_m),
        )
    };
    let Some(density_kg_m3) = density else {
        return Err(StepReject::AltitudeOutsideCard);
    };
    let Some(sound_speed_m_s) = sound else {
        return Err(StepReject::AltitudeOutsideCard);
    };
    let mach = air_speed_m_s / sound_speed_m_s;
    if !(air_speed_m_s.is_finite() && mach.is_finite()) {
        return Err(StepReject::NonFiniteState);
    }
    let read_by_altitude = altitude_m >= capsule.drag_switch_altitude_m;
    // Above the switch the band of air speeds is the limit, and it lies far
    // above Mach 1.0, so the Mach floor is never what refuses there.
    if read_by_altitude
        && !(air_speed_m_s >= capsule.air_speed_band_min_m_s
            && air_speed_m_s <= capsule.air_speed_band_max_m_s)
    {
        return Err(StepReject::AirSpeedOutsideCard);
    }
    if !(mach >= capsule.mach_floor) {
        return Err(StepReject::MachOutsideCard);
    }
    let drag_coefficient = if read_by_altitude {
        let Some(value) = between_entries(
            &capsule.drag_altitude_km,
            &capsule.drag_by_altitude,
            altitude_m / 1000.0,
        ) else {
            return Err(StepReject::AltitudeOutsideCard);
        };
        value
    } else {
        let Some(value) = drag_at_mach(capsule, mach) else {
            return Err(StepReject::MachOutsideCard);
        };
        value
    };
    let drag_acceleration_m_s2 = 0.5
        * density_kg_m3
        * air_speed_m_s
        * air_speed_m_s
        * drag_coefficient
        * capsule.reference_area_m2
        / capsule.capsule_mass_kg;
    if !drag_acceleration_m_s2.is_finite() {
        return Err(StepReject::NonFiniteState);
    }
    Ok(Air {
        altitude_m,
        radial_m_s,
        transverse_m_s,
        air_speed_m_s,
        density_kg_m3,
        sound_speed_m_s,
        mach,
        drag_acceleration_m_s2,
    })
}

/// Straight line between the two knots either side of `altitude_m`. `None`
/// when the altitude is not inside the table: nothing is held at its ends.
fn between_knots(capsule: &CapsuleStardust, table: &[f64; KNOTS], altitude_m: f64) -> Option<f64> {
    let knots_up = (altitude_m - capsule.altitude_floor_m) / capsule.table_knot_spacing_m;
    if !(knots_up >= 0.0) {
        return None;
    }
    let index = knots_up as usize;
    if index + 1 >= KNOTS {
        return None;
    }
    Some(table[index] + (table[index + 1] - table[index]) * (knots_up - (index as f64)))
}

/// Straight line between the two knots of a table below 30 km either side of
/// `altitude_m`. `None` when the altitude is not inside it.
fn between_low_knots(
    capsule: &CapsuleStardust,
    table: &[f64; LOW_KNOTS],
    altitude_m: f64,
) -> Option<f64> {
    let knots_up = (altitude_m - capsule.sea_level_m) / capsule.table_knot_spacing_m;
    if !(knots_up >= 0.0) {
        return None;
    }
    let index = knots_up as usize;
    if index + 1 >= LOW_KNOTS {
        return None;
    }
    Some(table[index] + (table[index + 1] - table[index]) * (knots_up - (index as f64)))
}

/// Straight line between two entries of a drag table. `None` outside the
/// table: nothing is held at its ends.
fn between_entries(arguments: &[f64], values: &[f64], x: f64) -> Option<f64> {
    let last = arguments.len() - 1;
    if !(x >= arguments[0] && x <= arguments[last]) {
        return None;
    }
    if x == arguments[last] {
        return Some(values[last]);
    }
    let mut index = 0;
    while x >= arguments[index + 1] {
        index += 1;
    }
    Some(
        values[index]
            + (values[index + 1] - values[index]) / (arguments[index + 1] - arguments[index])
                * (x - arguments[index]),
    )
}

/// Drag coefficient by Mach number. `None` above the table's top Mach number,
/// 40, and below its first: nothing is held at either end.
fn drag_at_mach(capsule: &CapsuleStardust, mach: f64) -> Option<f64> {
    between_entries(&capsule.drag_mach, &capsule.drag_by_mach, mach)
}

/// Rates of `(r, v_r, v_t, θ, Q)`.
fn rates(capsule: &CapsuleStardust, y: [f64; 5]) -> Result<[f64; 5], StepReject> {
    let radius_m = y[0];
    let radial_speed_m_s = y[1];
    let transverse_speed_m_s = y[2];
    let air = air_at(capsule, radius_m, radial_speed_m_s, transverse_speed_m_s)?;
    let drag = air.drag_acceleration_m_s2;
    let drag_radial = -drag * air.radial_m_s / air.air_speed_m_s;
    let drag_transverse = -drag * air.transverse_m_s / air.air_speed_m_s;
    let out = [
        radial_speed_m_s,
        transverse_speed_m_s * transverse_speed_m_s / radius_m
            - capsule.earth_gravitational_parameter_m3_s2 / (radius_m * radius_m)
            + drag_radial,
        -radial_speed_m_s * transverse_speed_m_s / radius_m + drag_transverse,
        transverse_speed_m_s / radius_m,
        drag * air.air_speed_m_s,
    ];
    if finite5(out) {
        Ok(out)
    } else {
        Err(StepReject::NonFiniteState)
    }
}

/// `J = ½ (v_r² + v_t²) − GM/r − Ω r v_t`.
fn rotating_frame_energy_j_kg(
    capsule: &CapsuleStardust,
    radius_m: f64,
    radial_speed_m_s: f64,
    transverse_speed_m_s: f64,
) -> f64 {
    0.5 * (radial_speed_m_s * radial_speed_m_s + transverse_speed_m_s * transverse_speed_m_s)
        - capsule.earth_gravitational_parameter_m3_s2 / radius_m
        - capsule.air_rotation_rate_rad_s * radius_m * transverse_speed_m_s
}

fn axpy(y: [f64; 5], scale: f64, k: [f64; 5]) -> [f64; 5] {
    [
        mul_dt(y[0], scale, k[0]),
        mul_dt(y[1], scale, k[1]),
        mul_dt(y[2], scale, k[2]),
        mul_dt(y[3], scale, k[3]),
        mul_dt(y[4], scale, k[4]),
    ]
}

fn mul_dt(y: f64, scale: f64, k: f64) -> f64 {
    if k == 0.0 { y } else { y + scale * k }
}

fn recorded(state: &CapsuleState) -> [f64; 5] {
    [
        state.radius_m,
        state.radial_speed_m_s,
        state.transverse_speed_m_s,
        state.angle_rad,
        state.drag_loss_j_kg,
    ]
}

fn finite5(x: [f64; 5]) -> bool {
    x[0].is_finite() && x[1].is_finite() && x[2].is_finite() && x[3].is_finite() && x[4].is_finite()
}

fn same_bits(recorded: f64, locked: f64) -> bool {
    recorded.to_bits() == locked.to_bits()
}

fn abs_f64(value: f64) -> f64 {
    if value < 0.0 { -value } else { value }
}

fn state_is_finite(state: &CapsuleState) -> bool {
    state.t_s.is_finite()
        && state.radius_m.is_finite()
        && state.radial_speed_m_s.is_finite()
        && state.transverse_speed_m_s.is_finite()
        && state.angle_rad.is_finite()
        && state.drag_loss_j_kg.is_finite()
        && state.energy_anchor_j_kg.is_finite()
        && state.energy_defect_j_kg.is_finite()
        && state.residual_conservation_error.is_finite()
        && state.altitude_m.is_finite()
        && state.ground_distance_m.is_finite()
        && state.air_speed_m_s.is_finite()
        && state.mach.is_finite()
        && state.load_g.is_finite()
        && state.heat_flux_w_m2.is_finite()
        && state.stagnation_pressure_pa.is_finite()
        && state.recovery_enthalpy_j_kg.is_finite()
        && state.heat_transfer_coefficient_kg_m2_s.is_finite()
}

/// Every limit a flag of this card can mark: what [`exceedances`] reports
/// beside a flag's bit (`crate::exceedance`).
pub const EXCEEDANCE_KINDS: &[crate::exceedance::ExceedanceKind] = &[
    WINDOW_LOW_KIND,
    WINDOW_HIGH_KIND,
];

const WINDOW_WORDS: &str = "the Mach window the drogue had to open in, 1.2 to 1.6: the flag is set inside it";
const WINDOW_LOW_KIND: ExceedanceKind = ExceedanceKind::fixed(
    "FLAG_DROGUE_MACH_WINDOW",
    FLAG_DROGUE_MACH_WINDOW,
    "mach",
    "1",
    Side::AtOrAbove,
    DROGUE_MACH_REQUIRED_MIN,
    WINDOW_WORDS,
);
const WINDOW_HIGH_KIND: ExceedanceKind = ExceedanceKind::fixed(
    "FLAG_DROGUE_MACH_WINDOW",
    FLAG_DROGUE_MACH_WINDOW,
    "mach",
    "1",
    Side::AtOrBelow,
    DROGUE_MACH_REQUIRED_MAX,
    WINDOW_WORDS,
);

/// The limits the step from `before` to `after` under `load` over `dt_s`
/// is past: one record for each limit a flag set on its frame marks.
/// The window flag is set inside its two edges, so a flagged step has one
/// record for each: how far above the lower edge and below the upper.
pub fn exceedances(
    capsule: &CapsuleStardust,
    before: &CapsuleState,
    after: &CapsuleState,
    load: CapsuleLoad,
    dt_s: f64,
) -> crate::exceedance::Exceedances {
    let _ = (before, load, dt_s);
    let mut records = crate::exceedance::Exceedances::new();
    if after.drogue_mach_window {
        records.push(Exceedance::at(WINDOW_LOW_KIND, capsule.drogue_mach_required_min, after.mach));
        records.push(Exceedance::at(WINDOW_HIGH_KIND, capsule.drogue_mach_required_max, after.mach));
    }
    records
}

#[cfg(test)]
mod tests {
    extern crate core;
    extern crate std;

    use core::prelude::v1::*;
    use std::assert;
    use std::assert_eq;
    use std::panic;

    use super::{
        AIR_DENSITY_KG_M3, AIR_SPEED_BAND_MAX_M_S, AIR_SPEED_BAND_MIN_M_S, ALTITUDE_CEILING_M,
        ALTITUDE_FLOOR_M, CORRIDOR_AIR_SPEED_HIGH_M_S, CORRIDOR_AIR_SPEED_LOW_M_S,
        CORRIDOR_BOTTOM_M, CORRIDOR_KNOTS, CORRIDOR_KNOT_SPACING_M, CORRIDOR_PATH_SINE_HIGH,
        CORRIDOR_PATH_SINE_LOW, CORRIDOR_TOP_M, CapsuleLoad, CapsuleStardust, CapsuleState,
        DRAG_ALTITUDE_KM,
        DRAG_BY_ALTITUDE, DRAG_BY_MACH, DRAG_MACH, DRAG_SWITCH_ALTITUDE_M, ENERGY_SCALE_J_KG,
        FLAG_DROGUE_MACH_WINDOW, FLIGHT_DROGUE_ALTITUDE_M, FLIGHT_DROGUE_MACH, GATE_DT_S,
        AIR_DENSITY_LOW_KG_M3, KNOTS, LOCAL_EARTH_RADIUS_M, LOW_KNOTS, MACH_FLOOR,
        RESIDUAL_CONSERVATION_LIMIT, SOUND_SPEED_LOW_M_S,
        SOUND_SPEED_M_S, StepReject, TABLE_KNOT_SPACING_M, abs_f64, axpy,
        between_entries, between_knots, between_low_knots, commit, drag_at_mach, integrate, rates,
        recorded,
        rotating_frame_energy_j_kg, step, validate,
    };
    use crate::frame::LastStateFrame64;

    const DT: f64 = GATE_DT_S;
    const NO_LOAD: CapsuleLoad = CapsuleLoad {};

    /// The standard's printed density from 86 km to 140 km, one per kilometre
    /// (U.S. Standard Atmosphere, 1976, Table I, printed p.68).
    const PRINTED_UPPER_DENSITY_KG_M3: [f64; 55] = [
        6.958e-6, 5.824e-6, 4.875e-6, 4.081e-6, 3.416e-6, 2.860e-6, 2.393e-6, 2.000e-6, 1.670e-6,
        1.393e-6, 1.162e-6, 9.685e-7, 8.071e-7, 6.725e-7, 5.604e-7, 4.695e-7, 3.935e-7, 3.300e-7,
        2.769e-7, 2.325e-7, 1.954e-7, 1.643e-7, 1.381e-7, 1.161e-7, 9.708e-8, 8.111e-8, 6.838e-8,
        5.811e-8, 4.975e-8, 4.289e-8, 3.720e-8, 3.246e-8, 2.847e-8, 2.509e-8, 2.222e-8, 1.977e-8,
        1.767e-8, 1.585e-8, 1.428e-8, 1.291e-8, 1.171e-8, 1.065e-8, 9.717e-9, 8.889e-9, 8.152e-9,
        7.494e-9, 6.904e-9, 6.374e-9, 5.897e-9, 5.465e-9, 5.074e-9, 4.719e-9, 4.396e-9, 4.101e-9,
        3.831e-9,
    ];

    // Places in a row of `REFERENCE_STATES`.
    const TIME: usize = 0;
    const RADIUS: usize = 1;
    const RADIAL: usize = 2;
    const TRANSVERSE: usize = 3;
    const ANGLE: usize = 4;
    const DRAG_LOSS: usize = 5;
    const ALTITUDE: usize = 6;
    const AIR_SPEED: usize = 7;
    const MACH: usize = 8;
    const LOAD: usize = 9;
    const HEAT_FLUX: usize = 10;
    const DEFECT: usize = 11;

    /// The stand-in in table form, RK4 at 0.05 s from the locked entry state
    /// (`card_reference_table_form.json`). Each row is the step number, then
    /// `t_s`, `radius_m`, `radial_speed_m_s`, `transverse_speed_m_s`,
    /// `angle_rad`, `drag_loss_j_kg`, `altitude_m`, `air_speed_m_s`, `mach`,
    /// `load_g`, `heat_flux_w_m2`, `energy_defect_j_kg`, then the flags.
    const REFERENCE_STATES: [(u32, [f64; 12], u64); 15] = [
        (
            0,
            [
                0.0,
                6503140.0,
                -1842.1416729435698,
                12767.792058801891,
                0.0,
                0.0,
                133911.48852154613,
                12551.258516382646,
                45.79142667299148,
                0.0010895559730245907,
                55327.16728784448,
                0.0,
            ],
            0,
        ),
        (
            1,
            [
                0.05,
                6503047.912471315,
                -1841.3594677413757,
                12767.972328993394,
                9.816775132675825e-05,
                6.729048836515585,
                133819.40099286102,
                12551.326992957847,
                45.79167649973738,
                0.0010972203247764422,
                55526.04627053919,
                -7.911284960471221e-09,
            ],
            0,
        ),
        (
            200,
            [
                10.000000000000007,
                6485503.428663513,
                -1684.9077438788404,
                12802.241896136746,
                0.01968730847452982,
                3429.649676654391,
                116274.91718505882,
                12564.231748449185,
                45.83875760830057,
                0.006343066739426476,
                136339.2565799553,
                1.3028829926042818e-07,
            ],
            0,
        ),
        (
            600,
            [
                30.00000000000029,
                6454999.797645938,
                -1362.1362268223736,
                12821.353833584102,
                0.05933521151477472,
                526277.4213528895,
                85771.28616748378,
                12545.673955224978,
                45.71663798671643,
                1.0300897351195748,
                1927972.132031199,
                1.6030389815568924e-07,
            ],
            0,
        ),
        (
            1000,
            [
                49.9999999999993,
                6431725.43899581,
                -923.091653995828,
                11152.731577568427,
                0.09766332555715147,
                20655409.764482323,
                62496.92751735635,
                10843.509627242747,
                34.903775750023854,
                22.75170103090352,
                6959080.990214281,
                2.0448118448257446e-05,
            ],
            0,
        ),
        (
            1188,
            [
                59.39999999999876,
                6424337.859871674,
                -651.8536273842465,
                8508.965098036437,
                0.11214859037642867,
                46005754.023006335,
                55109.348393220454,
                8186.773851058324,
                25.303979146138655,
                32.170160635699425,
                4718621.529485093,
                4.011392593383789e-05,
            ],
            0,
        ),
        (
            1189,
            [
                59.44999999999876,
                6424305.300586308,
                -650.5181604514844,
                8493.28416088623,
                0.11221475397468694,
                46134768.881608516,
                55076.78910785448,
                8171.038125706197,
                25.250999432398853,
                32.17008884439047,
                4700503.900934352,
                4.010647535324097e-05,
            ],
            0,
        ),
        (
            1400,
            [
                69.99999999999817,
                6418722.812257599,
                -428.19710285635074,
                5467.343237076745,
                0.1235751998465857,
                66375426.498141415,
                49494.30077914521,
                5137.340061568258,
                15.57719230464216,
                24.932441232351902,
                1628160.9536446147,
                4.07099723815918e-05,
            ],
            0,
        ),
        (
            2000,
            [
                99.99999999999646,
                6409602.587967617,
                -248.39059677288233,
                1645.7796273040703,
                0.13800500105519414,
                78785984.58671765,
                40374.07648916356,
                1321.9403410948582,
                4.1590869885952335,
                5.593050686704173,
                51603.95114894905,
                5.345046520233154e-05,
            ],
            0,
        ),
        (
            2400,
            [
                119.99999999999532,
                6404759.811949636,
                -238.5567336916978,
                918.3687969805424,
                0.14185109810359048,
                79515062.51033406,
                35531.30047118198,
                619.0569499929655,
                2.0017557676740183,
                2.5647315413925535,
                7608.013534968867,
                5.391240119934082e-05,
            ],
            0,
        ),
        (
            2543,
            [
                127.14999999999492,
                6403055.512755154,
                -238.25712314440605,
                774.9741571462879,
                0.14279253012822365,
                79603260.06270088,
                33827.001276699826,
                489.79836826078514,
                1.59973542481584,
                2.020210142150712,
                4300.8045407098025,
                5.383789539337158e-05,
            ],
            1,
        ),
        (
            2739,
            [
                136.9499999999969,
                6400722.924513078,
                -237.36254687655153,
                633.7779989665727,
                0.143864193195799,
                79676534.62209931,
                31494.41303462442,
                372.34092706848463,
                1.2300948087524461,
                1.5828619967630162,
                2268.3310824755513,
                5.3763389587402344e-05,
            ],
            1,
        ),
        (
            2740,
            [
                136.9999999999969,
                6400711.056672601,
                -237.35105832167528,
                633.1817481969132,
                0.1438691417022133,
                79676823.27839905,
                31482.545194147155,
                371.8748957648749,
                1.2285868528449406,
                1.5812480090206913,
                2261.869402196408,
                5.373358726501465e-05,
            ],
            1,
        ),
        (
            2758,
            [
                137.8999999999971,
                6400497.53831334,
                -237.12902896119078,
                622.6344589546702,
                0.14395742887877053,
                79681910.03973158,
                31269.02683488652,
                363.68157931495307,
                1.2020755995425243,
                1.553567851718309,
                2150.4219757811525,
                5.3748488426208496e-05,
            ],
            1,
        ),
        (
            2778,
            [
                138.89999999999733,
                6400260.548434607,
                -236.8432223156414,
                611.3117699173176,
                0.14405381992225116,
                79687333.95233452,
                31032.03695615288,
                354.99228419555374,
                1.1739598541045693,
                1.5254541789740474,
                2036.4723932841678,
                5.3763389587402344e-05,
            ],
            0,
        ),
    ];

    fn reference(step_number: u32) -> [f64; 12] {
        let mut index = 0;
        while index < REFERENCE_STATES.len() {
            if REFERENCE_STATES[index].0 == step_number {
                return REFERENCE_STATES[index].1;
            }
            index += 1;
        }
        panic!("no reference state at step {step_number}");
    }

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

    fn state_bits(state: &CapsuleState) -> [u64; 16] {
        [
            state.t_s.to_bits(),
            state.radius_m.to_bits(),
            state.radial_speed_m_s.to_bits(),
            state.transverse_speed_m_s.to_bits(),
            state.angle_rad.to_bits(),
            state.drag_loss_j_kg.to_bits(),
            state.energy_anchor_j_kg.to_bits(),
            state.energy_defect_j_kg.to_bits(),
            state.residual_conservation_error.to_bits(),
            state.altitude_m.to_bits(),
            state.ground_distance_m.to_bits(),
            state.air_speed_m_s.to_bits(),
            state.mach.to_bits(),
            state.load_g.to_bits(),
            state.heat_flux_w_m2.to_bits(),
            u64::from(state.drogue_mach_window),
        ]
    }

    /// A start at this altitude with this speed through the air and this
    /// sine of the path angle through the air.
    fn through_the_air(
        capsule: &CapsuleStardust,
        altitude_m: f64,
        air_speed_m_s: f64,
        sine: f64,
    ) -> Result<CapsuleState, StepReject> {
        let radius_m = LOCAL_EARTH_RADIUS_M + altitude_m;
        let radial_m_s = sine * air_speed_m_s;
        let along_m_s = (air_speed_m_s * air_speed_m_s - radial_m_s * radial_m_s).sqrt();
        CapsuleState::anchor(
            capsule,
            radius_m,
            radial_m_s,
            along_m_s + capsule.air_rotation_rate_rad_s * radius_m,
        )
    }

    /// The corridor's four bounds at this altitude, by straight lines between knots.
    fn corridor_at(altitude_m: f64) -> [f64; 4] {
        let knots_up = (altitude_m - CORRIDOR_BOTTOM_M) / CORRIDOR_KNOT_SPACING_M;
        let mut index = knots_up as usize;
        if index == CORRIDOR_KNOTS - 1 {
            index -= 1;
        }
        let share = knots_up - (index as f64);
        let read = |table: &[f64; CORRIDOR_KNOTS]| table[index] + (table[index + 1] - table[index]) * share;
        [
            read(&CORRIDOR_AIR_SPEED_LOW_M_S),
            read(&CORRIDOR_AIR_SPEED_HIGH_M_S),
            read(&CORRIDOR_PATH_SINE_LOW),
            read(&CORRIDOR_PATH_SINE_HIGH),
        ]
    }

    fn knot_at(altitude_m: f64) -> usize {
        ((altitude_m - ALTITUDE_FLOOR_M) / TABLE_KNOT_SPACING_M) as usize
    }

    /// `½ (v_r² + v_t²) − GM/r`: the energy without the term for the turning air.
    fn fixed_frame_energy_j_kg(capsule: &CapsuleStardust, y: [f64; 5]) -> f64 {
        0.5 * (y[1] * y[1] + y[2] * y[2]) - capsule.earth_gravitational_parameter_m3_s2 / y[0]
    }

    fn flags_of(state: &CapsuleState) -> u64 {
        let mut flags = 0;
        if state.drogue_mach_window {
            flags |= FLAG_DROGUE_MACH_WINDOW;
        }
        flags
    }

    /// Explicit Euler on `(r, v_r, v_t, θ, Q)`.
    fn step_euler(
        capsule: &CapsuleStardust,
        state: &mut CapsuleState,
        frame: &mut LastStateFrame64,
        dt_s: f64,
    ) -> Result<(), StepReject> {
        validate(capsule, state, NO_LOAD, dt_s)?;
        let y = recorded(state);
        let start = rates(capsule, y)?;
        commit(capsule, state, frame, dt_s, axpy(y, dt_s, start))
    }

    /// Honest RK4, then `extra_j_kg` added to the booked drag loss.
    fn step_bias_drag_loss(
        capsule: &CapsuleStardust,
        state: &mut CapsuleState,
        frame: &mut LastStateFrame64,
        dt_s: f64,
        extra_j_kg: f64,
    ) -> Result<(), StepReject> {
        validate(capsule, state, NO_LOAD, dt_s)?;
        let mut predicted = integrate(capsule, recorded(state), dt_s)?;
        predicted[4] += extra_j_kg;
        commit(capsule, state, frame, dt_s, predicted)
    }

    /// Honest RK4 on the phase. The drag loss is booked at the rate the step
    /// started with, as if the air tables were read once and not at each stage.
    fn step_booked_at_the_starting_rate(
        capsule: &CapsuleStardust,
        state: &mut CapsuleState,
        frame: &mut LastStateFrame64,
        dt_s: f64,
    ) -> Result<(), StepReject> {
        validate(capsule, state, NO_LOAD, dt_s)?;
        let y = recorded(state);
        let start = rates(capsule, y)?;
        let mut predicted = integrate(capsule, y, dt_s)?;
        predicted[4] = y[4] + dt_s * start[4];
        commit(capsule, state, frame, dt_s, predicted)
    }

    /// The real `validate` against the real capsule, then the RK4 step of
    /// `mover` and the ledger of `keeper`. Either may be a changed copy.
    fn step_with_copies(
        capsule: &CapsuleStardust,
        mover: &CapsuleStardust,
        keeper: &CapsuleStardust,
        state: &mut CapsuleState,
        frame: &mut LastStateFrame64,
        dt_s: f64,
    ) -> Result<(), StepReject> {
        validate(capsule, state, NO_LOAD, dt_s)?;
        let predicted = integrate(mover, recorded(state), dt_s)?;
        commit(keeper, state, frame, dt_s, predicted)
    }

    #[test]
    fn constants_and_tables_are_locked() {
        assert_copy::<CapsuleStardust>();
        assert_copy::<CapsuleState>();
        assert_copy::<CapsuleLoad>();
        assert_copy::<StepReject>();
        let capsule = CapsuleStardust::sample_return_capsule();
        assert!(capsule.is_valid());
        assert_eq!(capsule.gate_dt_s(), Some(0.05));
        let mut state = CapsuleState::try_entry_interface(&capsule).unwrap();
        let saved = state_bits(&state);
        let mut frame = blank_frame();
        let held = frame.to_bytes();
        let mut refuse = |tampered: CapsuleStardust, what: &str| {
            assert!(
                !tampered.is_valid(),
                "{what} changed and the card is still valid"
            );
            assert!(tampered.gate_dt_s().is_none(), "{what}");
            assert!(
                CapsuleState::try_entry_interface(&tampered).is_none(),
                "{what}"
            );
            let got = step(&tampered, &mut state, &mut frame, NO_LOAD, DT);
            assert_eq!(got, Err(StepReject::InvalidCapsule), "{what}");
            assert_eq!(state_bits(&state), saved, "{what}");
            assert_eq!(frame.to_bytes(), held, "{what}");
            proof_intact(&frame);
        };

        let mut tampered = capsule;
        tampered.air_density_kg_m3[125] *= 1.000_001;
        refuse(tampered, "one air density entry");
        let mut tampered = capsule;
        tampered.sound_speed_m_s[60] += 0.001;
        refuse(tampered, "one speed of sound entry");
        let mut tampered = capsule;
        tampered.drag_by_mach[16] = 1.48;
        refuse(tampered, "one drag entry by Mach number");
        let mut tampered = capsule;
        tampered.drag_by_altitude[0] = 1.65;
        refuse(tampered, "one drag entry by altitude");
        let mut tampered = capsule;
        tampered.drag_mach[4] = 1.01;
        refuse(tampered, "one Mach number of the drag table");
        let mut tampered = capsule;
        tampered.capsule_mass_kg = 46.0;
        refuse(tampered, "the mass");
        let mut tampered = capsule;
        tampered.air_rotation_rate_rad_s = 0.0;
        refuse(tampered, "the air rotation rate");
        let mut tampered = capsule;
        tampered.local_earth_radius_m = 6_378_137.0;
        refuse(tampered, "the Earth radius under the path");
        let mut tampered = capsule;
        tampered.entry_radial_speed_m_s = -1_839.9;
        refuse(tampered, "the entry state");
        let mut tampered = capsule;
        tampered.residual_conservation_limit = 1.0e-3;
        refuse(tampered, "the ledger limit");
        let mut tampered = capsule;
        tampered.altitude_floor_m = 0.0;
        refuse(tampered, "the floor");
        let mut tampered = capsule;
        tampered.altitude_ceiling_m = 140_000.0;
        refuse(tampered, "the ceiling");
        let mut tampered = capsule;
        tampered.drag_switch_altitude_m = 70_000.0;
        refuse(tampered, "the altitude at which the drag read changes");
        let mut tampered = capsule;
        tampered.air_speed_band_min_m_s = 0.0;
        refuse(tampered, "the bottom of the air-speed band");
        let mut tampered = capsule;
        tampered.air_speed_band_max_m_s = 20_000.0;
        refuse(tampered, "the top of the air-speed band");
        let mut tampered = capsule;
        tampered.air_density_low_kg_m3[20] *= 1.0001;
        refuse(tampered, "one knot of the low density table");
        let mut tampered = capsule;
        tampered.sound_speed_low_m_s[150] += 0.001;
        refuse(tampered, "the top knot of the low sound-speed table");
        let mut tampered = capsule;
        tampered.sea_level_m = -100.0;
        refuse(tampered, "the sea level");
        let mut tampered = capsule;
        tampered.flight_drogue_altitude_m = 30_000.0;
        refuse(tampered, "the radar altitude of drogue deployment");
        let mut tampered = capsule;
        tampered.flight_drogue_mach = 1.0;
        refuse(tampered, "the radar Mach number of drogue deployment");

        step(&capsule, &mut state, &mut frame, NO_LOAD, DT).unwrap();
        proof_intact(&frame);
    }

    #[test]
    fn the_air_tables_were_copied_correctly() {
        let capsule = CapsuleStardust::sample_return_capsule();
        assert_eq!(KNOTS, 551);
        assert_eq!(AIR_DENSITY_KG_M3.len(), 551);
        assert_eq!(SOUND_SPEED_M_S.len(), 551);
        // The tables run from 30 km to 140 km. The card's ceiling is inside them.
        assert_eq!(
            (ALTITUDE_FLOOR_M + 550.0 * TABLE_KNOT_SPACING_M).to_bits(),
            140_000.0_f64.to_bits()
        );
        assert_eq!(ALTITUDE_CEILING_M.to_bits(), 134_700.0_f64.to_bits());
        assert_eq!(AIR_DENSITY_KG_M3[0].to_bits(), 1.84101704e-2_f64.to_bits());
        assert_eq!(
            AIR_DENSITY_KG_M3[KNOTS - 1].to_bits(),
            3.831e-9_f64.to_bits()
        );
        assert_eq!(SOUND_SPEED_M_S[0].to_bits(), 3.01708766e2_f64.to_bits());
        assert_eq!(
            SOUND_SPEED_M_S[KNOTS - 1].to_bits(),
            2.74096254e2_f64.to_bits()
        );

        // Each column added up in knot order, against the same sum of card_tables.csv.
        let mut density_sum = 0.0;
        let mut sound_sum = 0.0;
        let mut index = 0;
        while index < KNOTS {
            density_sum += AIR_DENSITY_KG_M3[index];
            sound_sum += SOUND_SPEED_M_S[index];
            if index > 0 {
                assert!(
                    AIR_DENSITY_KG_M3[index] < AIR_DENSITY_KG_M3[index - 1],
                    "density does not fall at knot {index}"
                );
            }
            assert!(AIR_DENSITY_KG_M3[index] > 0.0 && SOUND_SPEED_M_S[index] > 0.0);
            index += 1;
        }
        assert_eq!(density_sum.to_bits(), 0.6267140557308454_f64.to_bits());
        assert_eq!(sound_sum.to_bits(), 160286.88597100094_f64.to_bits());

        // Six densities printed in the standard (printed pp.65 and 67), within 0.01 percent.
        let printed = [
            (40_000.0, 3.9957e-3),
            (50_000.0, 1.0269e-3),
            (60_000.0, 3.0968e-4),
            (70_000.0, 8.2829e-5),
            (80_000.0, 1.8458e-5),
            (85_000.0, 8.2196e-6),
        ];
        let mut index = 0;
        while index < printed.len() {
            let (altitude_m, density) = printed[index];
            let at_knot = AIR_DENSITY_KG_M3[knot_at(altitude_m)];
            let through_lookup =
                between_knots(&capsule, &capsule.air_density_kg_m3, altitude_m).unwrap();
            assert_eq!(at_knot.to_bits(), through_lookup.to_bits());
            assert!(
                abs_f64(at_knot / density - 1.0) < 1.0e-4,
                "{altitude_m} m: table {at_knot}, printed {density}"
            );
            index += 1;
        }

        // The 55 densities printed from 86 km to 140 km sit at knots, digit for digit.
        let mut kilometre = 0;
        while kilometre < 55 {
            let altitude_m = 86_000.0 + 1_000.0 * (kilometre as f64);
            assert_eq!(
                AIR_DENSITY_KG_M3[knot_at(altitude_m)].to_bits(),
                PRINTED_UPPER_DENSITY_KG_M3[kilometre].to_bits(),
                "{altitude_m} m"
            );
            kilometre += 1;
        }

        // The standard's formula gives 6.95782378e-6 at 86 km and its table prints 6.958e-6.
        let join = AIR_DENSITY_KG_M3[knot_at(86_000.0)];
        assert_eq!(join.to_bits(), 6.958e-6_f64.to_bits());
        assert!(abs_f64(6.95782378e-6 / join - 1.0) < 1.0e-4);

        // The speed of sound holds its 86 km value above 86 km.
        let mut index = knot_at(86_000.0);
        while index < KNOTS {
            assert_eq!(SOUND_SPEED_M_S[index].to_bits(), 2.74096254e2_f64.to_bits());
            index += 1;
        }

        // Straight lines: the knot value at a knot, the mean of two knots half way.
        let density =
            |altitude_m: f64| between_knots(&capsule, &capsule.air_density_kg_m3, altitude_m);
        let low = AIR_DENSITY_KG_M3[100];
        let high = AIR_DENSITY_KG_M3[101];
        let half_way = density(50_100.0).unwrap();
        assert!(abs_f64(half_way / (0.5 * (low + high)) - 1.0) < 1.0e-15);
        // The first knot, and the last knot under the ceiling, are returned exactly.
        assert_eq!(
            density(ALTITUDE_FLOOR_M).unwrap().to_bits(),
            AIR_DENSITY_KG_M3[0].to_bits()
        );
        assert_eq!(
            density(134_600.0).unwrap().to_bits(),
            AIR_DENSITY_KG_M3[523].to_bits()
        );
        // The ceiling sits half way between knots 523 and 524.
        let at_ceiling = density(ALTITUDE_CEILING_M).unwrap();
        let mean = 0.5 * (AIR_DENSITY_KG_M3[523] + AIR_DENSITY_KG_M3[524]);
        assert!(abs_f64(at_ceiling / mean - 1.0) < 1.0e-15);
        // Nothing is held at either end of the table.
        assert!(density(29_999.0).is_none());
        assert!(density(140_000.0).is_none());
        assert!(density(150_000.0).is_none());
        assert!(density(f64::NAN).is_none());
    }

    #[test]
    fn the_drag_tables_return_their_end_entries_exactly_and_hold_nothing_outside() {
        let by_altitude =
            |altitude_km: f64| between_entries(&DRAG_ALTITUDE_KM, &DRAG_BY_ALTITUDE, altitude_km);
        let by_mach = |mach: f64| between_entries(&DRAG_MACH, &DRAG_BY_MACH, mach);
        let bits = |value: Option<f64>| value.unwrap().to_bits();

        // First and last entry of each table, exactly at its argument.
        assert_eq!(bits(by_altitude(68.96)), 1.4959_f64.to_bits());
        assert_eq!(bits(by_altitude(134.7)), 2.019_f64.to_bits());
        assert_eq!(bits(by_mach(0.16)), 0.876_f64.to_bits());
        assert_eq!(bits(by_mach(40.0)), 1.4959_f64.to_bits());
        // The card's own ends land on those entries: the switch altitude and the ceiling.
        assert_eq!(
            bits(by_altitude(DRAG_SWITCH_ALTITUDE_M / 1000.0)),
            1.4959_f64.to_bits()
        );
        assert_eq!(
            bits(by_altitude(ALTITUDE_CEILING_M / 1000.0)),
            2.019_f64.to_bits()
        );

        // Outside a table there is no value. Nothing is held.
        assert!(by_altitude(68.95).is_none());
        assert!(by_altitude(60.0).is_none());
        assert!(by_altitude(134.8).is_none());
        assert!(by_altitude(140.0).is_none());
        assert!(by_mach(0.1).is_none());
        assert!(by_mach(40.001).is_none());
        assert!(by_mach(f64::NAN).is_none());

        // Inside: the entry at an entry, the mean of two entries half way.
        assert_eq!(bits(by_altitude(92.0)), 1.64_f64.to_bits());
        assert!(abs_f64(by_altitude(96.45).unwrap() - 0.5 * (1.64 + 1.822)) < 1.0e-12);
        assert_eq!(bits(by_mach(1.0)), 1.094_f64.to_bits());
        assert_eq!(bits(by_mach(35.4)), 1.4828_f64.to_bits());
        assert!(abs_f64(by_mach(1.0775).unwrap() - 0.5 * (1.094 + 1.328)) < 1.0e-12);
        assert!(abs_f64(by_mach(30.05).unwrap() - 0.5 * (1.4816 + 1.4828)) < 1.0e-12);
        let mut index = 1;
        while index < DRAG_MACH.len() {
            assert!(DRAG_MACH[index] > DRAG_MACH[index - 1]);
            index += 1;
        }
        let mut index = 1;
        while index < DRAG_ALTITUDE_KM.len() {
            assert!(DRAG_ALTITUDE_KM[index] > DRAG_ALTITUDE_KM[index - 1]);
            index += 1;
        }
    }

    #[test]
    fn above_mach_40_below_the_switch_is_refused_and_nothing_is_held() {
        // The table by Mach number stops at Mach 40. Its top entry is read at
        // Mach 40 exactly and nothing is read above it. Through engine 0.6.0
        // that entry was held above Mach 40.
        let capsule = CapsuleStardust::sample_return_capsule();
        let top_entry = DRAG_BY_MACH[DRAG_MACH.len() - 1];
        assert_eq!(top_entry.to_bits(), 1.4959_f64.to_bits());
        let at = |mach: f64| drag_at_mach(&capsule, mach);
        assert_eq!(at(40.0).unwrap().to_bits(), top_entry.to_bits());
        assert!(at(f64::from_bits(40.0_f64.to_bits() + 1)).is_none());
        assert!(at(40.001).is_none());
        assert!(at(45.8).is_none());
        assert!(at(39.999).unwrap() < top_entry);
        assert_eq!(at(1.0).unwrap().to_bits(), 1.094_f64.to_bits());
        assert!(at(0.1).is_none());

        // 65 km with the entry's two speeds is Mach 41: refused, as a start and as a step.
        let fast_at = |altitude_m: f64| {
            CapsuleState::anchor(
                &capsule,
                LOCAL_EARTH_RADIUS_M + altitude_m,
                capsule.entry_radial_speed_m_s,
                capsule.entry_transverse_speed_m_s,
            )
        };
        assert_eq!(fast_at(65_000.0), Err(StepReject::MachOutsideCard));
        assert!(CapsuleState::try_state(
            &capsule,
            LOCAL_EARTH_RADIUS_M + 65_000.0,
            capsule.entry_radial_speed_m_s,
            capsule.entry_transverse_speed_m_s,
        )
        .is_none());
        let mut entry = CapsuleState::try_entry_interface(&capsule).unwrap();
        let saved = state_bits(&entry);
        entry.radius_m = LOCAL_EARTH_RADIUS_M + 65_000.0;
        let moved = state_bits(&entry);
        let mut frame = blank_frame();
        let held = frame.to_bytes();
        assert_eq!(
            step(&capsule, &mut entry, &mut frame, NO_LOAD, DT),
            Err(StepReject::MachOutsideCard)
        );
        assert_eq!(state_bits(&entry), moved);
        assert!(moved != saved);
        assert_eq!(frame.to_bytes(), held);

        // A run started at 60 km at 20 km/s was sealed through engine 0.6.0
        // at Mach 62. It is refused as a start.
        let audit = CapsuleState::anchor(&capsule, 6429228.511478454, -2000.0, 20000.0);
        assert_eq!(audit, Err(StepReject::MachOutsideCard));
        // A single step at the same speed started at 68.96 km, where drag is
        // read by altitude: 20 km/s is above the band of air speeds there.
        let audit = CapsuleState::anchor(&capsule, 6438188.511478454, -2000.0, 20000.0);
        assert_eq!(audit, Err(StepReject::AirSpeedOutsideCard));

        // Mach 40.1 at 60 km is refused as MachOutsideCard. Mach 39.9 there,
        // falling straight down, is inside the table and is computed: until
        // engine 0.9 it was off the corridor the locked entry flew.
        let radius_m = LOCAL_EARTH_RADIUS_M + 60_000.0;
        let sound_m_s = between_knots(&capsule, &capsule.sound_speed_m_s, 60_000.0).unwrap();
        let at_mach = |mach: f64| {
            CapsuleState::anchor(
                &capsule,
                radius_m,
                -mach * sound_m_s,
                capsule.air_rotation_rate_rad_s * radius_m,
            )
        };
        assert!(at_mach(39.9).is_ok());
        assert_eq!(at_mach(40.1), Err(StepReject::MachOutsideCard));

        // The locked entry never reaches it. The Mach number falls through 40
        // at 43.36 s, at 69.23 km, while drag is still read by altitude. The
        // first landed state under 68.96 km is at Mach 39.81.
        let mut state = CapsuleState::try_entry_interface(&capsule).unwrap();
        let mut frame = blank_frame();
        let mut first_under_the_switch = None;
        let mut last_at_mach_40 = state;
        loop {
            match step(&capsule, &mut state, &mut frame, NO_LOAD, DT) {
                Ok(()) => {}
                Err(StepReject::PastFlightDeployment) => break,
                Err(other) => panic!("refused with {other:?}"),
            }
            if state.mach >= 40.0 {
                last_at_mach_40 = state;
            }
            if state.altitude_m < DRAG_SWITCH_ALTITUDE_M {
                assert!(
                    state.mach < 40.0,
                    "Mach {} at {} m",
                    state.mach,
                    state.altitude_m
                );
                if first_under_the_switch.is_none() {
                    first_under_the_switch = Some(state);
                }
            }
        }
        let first = first_under_the_switch.unwrap();
        assert_eq!(first.mach.to_bits(), 39.812109311728015_f64.to_bits());
        assert_eq!(first.altitude_m.to_bits(), 68910.30920205079_f64.to_bits());
        assert!(last_at_mach_40.altitude_m > 69_200.0 && last_at_mach_40.altitude_m < 69_300.0);
        assert!(
            abs_f64(last_at_mach_40.t_s - 43.35) < 1.0e-9,
            "{}",
            last_at_mach_40.t_s
        );
        proof_intact(&frame);
    }

    #[test]
    fn above_the_switch_an_air_speed_outside_the_band_is_refused() {
        let capsule = CapsuleStardust::sample_return_capsule();
        // Straight down through the air: the transverse speed is the air's own turning.
        let falling = |altitude_m: f64, air_speed_m_s: f64| {
            let radius_m = LOCAL_EARTH_RADIUS_M + altitude_m;
            CapsuleState::anchor(
                &capsule,
                radius_m,
                -air_speed_m_s,
                capsule.air_rotation_rate_rad_s * radius_m,
            )
        };
        // Both ends of the band are taken, exactly; one ulp outside is refused.
        for altitude_m in [DRAG_SWITCH_ALTITUDE_M, 100_000.0, ALTITUDE_CEILING_M] {
            let low = falling(altitude_m, AIR_SPEED_BAND_MIN_M_S).unwrap();
            assert_eq!(low.air_speed_m_s.to_bits(), AIR_SPEED_BAND_MIN_M_S.to_bits());
            let high = falling(altitude_m, AIR_SPEED_BAND_MAX_M_S).unwrap();
            assert_eq!(high.air_speed_m_s.to_bits(), AIR_SPEED_BAND_MAX_M_S.to_bits());
            let under = f64::from_bits(AIR_SPEED_BAND_MIN_M_S.to_bits() - 1);
            let over = f64::from_bits(AIR_SPEED_BAND_MAX_M_S.to_bits() + 1);
            assert_eq!(falling(altitude_m, under), Err(StepReject::AirSpeedOutsideCard));
            assert_eq!(falling(altitude_m, over), Err(StepReject::AirSpeedOutsideCard));
            // The bottom of the band is more than thirty times the speed of sound there.
            let sound_m_s = between_knots(&capsule, &capsule.sound_speed_m_s, altitude_m).unwrap();
            assert!(AIR_SPEED_BAND_MIN_M_S > 30.0 * sound_m_s);
        }
        // Under the switch the band does not apply. 600 m/s straight down at
        // 60 km, off the corridor the locked entry flew, is computed.
        assert!(falling(60_000.0, 600.0).is_ok());
        // Where the card stood at 12 575 m/s, the source's own design speeds
        // to 12 620 m/s are taken and a step from each lands on the ledger.
        assert!(falling(100_000.0, AIR_SPEED_BAND_MAX_M_S).is_ok());
        for air_speed_m_s in [12_575.5, 12_600.0, 12_619.0] {
            let mut state = falling(100_000.0, air_speed_m_s).unwrap();
            let mut frame = blank_frame();
            step(&capsule, &mut state, &mut frame, NO_LOAD, DT).unwrap();
            assert!(state.residual_conservation_error < RESIDUAL_CONSERVATION_LIMIT);
        }

        // Two states sealed through engine 0.6.0: at rest in the fixed
        // frame at 100 km, and 300 m/s through the air at 130 km.
        let at_rest = CapsuleState::anchor(&capsule, 6469228.511478454, 0.0, 0.0);
        assert_eq!(at_rest, Err(StepReject::AirSpeedOutsideCard));
        let slow = CapsuleState::anchor(&capsule, 6499228.511478454, -300.0, 352.2425992140121);
        assert_eq!(slow, Err(StepReject::AirSpeedOutsideCard));
        // Retrograde at the entry speed meets the air at about 13 250 m/s.
        let retrograde = CapsuleState::anchor(
            &capsule,
            LOCAL_EARTH_RADIUS_M + 100_000.0,
            capsule.entry_radial_speed_m_s,
            -capsule.entry_transverse_speed_m_s,
        );
        assert_eq!(retrograde, Err(StepReject::AirSpeedOutsideCard));
        // As a step, the refusal leaves the state and the frame as they were.
        let mut state = CapsuleState::try_entry_interface(&capsule).unwrap();
        state.radial_speed_m_s = 0.0;
        state.transverse_speed_m_s = 0.0;
        let saved = state_bits(&state);
        let mut frame = blank_frame();
        let held = frame.to_bytes();
        assert_eq!(
            step(&capsule, &mut state, &mut frame, NO_LOAD, DT),
            Err(StepReject::AirSpeedOutsideCard)
        );
        assert_eq!(state_bits(&state), saved);
        assert_eq!(frame.to_bytes(), held);

        // The locked entry stays inside the band above 68.96 km, from 11 912
        // to 12 573 m/s, at the ceiling step and at a tenth of it. The band is
        // that span joined with the design trajectory's, rounded outward to
        // 25 m/s and no further.
        for dt_s in [DT, DT / 10.0] {
            let mut state = CapsuleState::try_entry_interface(&capsule).unwrap();
            let mut frame = blank_frame();
            let mut lowest = f64::MAX;
            let mut highest = 0.0_f64;
            while state.altitude_m >= DRAG_SWITCH_ALTITUDE_M {
                lowest = if state.air_speed_m_s < lowest { state.air_speed_m_s } else { lowest };
                highest = if state.air_speed_m_s > highest { state.air_speed_m_s } else { highest };
                step(&capsule, &mut state, &mut frame, NO_LOAD, dt_s).unwrap();
            }
            assert!(lowest > 11_912.0 && lowest < 11_913.0, "{lowest}");
            assert!(highest > 12_573.0 && highest < 12_574.0, "{highest}");
            assert!(lowest - AIR_SPEED_BAND_MIN_M_S > 0.0 && lowest - AIR_SPEED_BAND_MIN_M_S < 25.0);
            assert!(AIR_SPEED_BAND_MAX_M_S - highest > 0.0 && AIR_SPEED_BAND_MAX_M_S - highest < 50.0);
            // The band's top is the design trajectory's top speed, at which
            // the drag table was computed and which was not compared with the flight.
            assert_eq!(AIR_SPEED_BAND_MAX_M_S, 12_620.0);
        }
    }

    #[test]
    fn past_the_flown_drogue_deployment_a_state_is_refused() {
        let capsule = CapsuleStardust::sample_return_capsule();
        // The run from entry lands step 2779, the first under both 31 030 m
        // and Mach 1.23, and refuses step 2780 by name.
        let mut state = CapsuleState::try_entry_interface(&capsule).unwrap();
        let mut frame = blank_frame();
        let mut landed = 0u32;
        loop {
            let before = state;
            let held = state_bits(&state);
            let held_frame = frame.to_bytes();
            match step(&capsule, &mut state, &mut frame, NO_LOAD, DT) {
                Ok(()) => {
                    landed += 1;
                    assert!(landed < 4_000);
                    assert!(
                        !(before.altitude_m < FLIGHT_DROGUE_ALTITUDE_M
                            && before.mach < FLIGHT_DROGUE_MACH),
                        "step {landed} started past deployment"
                    );
                }
                Err(StepReject::PastFlightDeployment) => {
                    assert_eq!(state_bits(&state), held);
                    assert_eq!(frame.to_bytes(), held_frame);
                    proof_intact(&frame);
                    break;
                }
                Err(other) => panic!("refused with {other:?} after {landed} steps"),
            }
        }
        assert_eq!(landed, 2_779);
        assert!(state.altitude_m < FLIGHT_DROGUE_ALTITUDE_M && state.mach < FLIGHT_DROGUE_MACH);
        assert!(abs_f64(state.t_s - 138.95) < 1.0e-9, "{}", state.t_s);
        // The refusal repeats, and a start from the returned state is refused too.
        assert_eq!(
            step(&capsule, &mut state, &mut frame, NO_LOAD, DT),
            Err(StepReject::PastFlightDeployment)
        );
        assert_eq!(
            CapsuleState::anchor(
                &capsule,
                state.radius_m,
                state.radial_speed_m_s,
                state.transverse_speed_m_s
            ),
            Err(StepReject::PastFlightDeployment)
        );
        // A smaller step does not get further.
        assert_eq!(
            step(&capsule, &mut state, &mut frame, NO_LOAD, DT / 1_000.0),
            Err(StepReject::PastFlightDeployment)
        );

        // A single step at 30.5 km and Mach 1.14, sealed through engine 0.6.0.
        let audit = CapsuleState::anchor(&capsule, 6399728.511478454, -200.0, 626.8541052549);
        assert_eq!(audit, Err(StepReject::PastFlightDeployment));

        // Under one of the two only is not past: low and fast, or slow and
        // high, on the corridor's path angle.
        let at_mach = |altitude_m: f64, mach: f64| {
            let sound_m_s = between_knots(&capsule, &capsule.sound_speed_m_s, altitude_m).unwrap();
            through_the_air(&capsule, altitude_m, mach * sound_m_s, -0.65)
        };
        assert!(at_mach(31_010.0, 1.234).is_ok());
        assert!(at_mach(31_100.0, 1.18).is_ok());
        assert_eq!(at_mach(31_010.0, 1.18), Err(StepReject::PastFlightDeployment));
        // Exactly on the altitude is not under it.
        assert!(at_mach(FLIGHT_DROGUE_ALTITUDE_M, 1.18).is_ok());
        assert_eq!(
            at_mach(FLIGHT_DROGUE_ALTITUDE_M - 0.001, 1.18),
            Err(StepReject::PastFlightDeployment)
        );
        // Past deployment is named before the corridor: under 31 km at Mach 1.1.
        assert_eq!(at_mach(30_500.0, 1.1), Err(StepReject::PastFlightDeployment));
    }

    #[test]
    fn free_fall_conserves_energy_and_angular_momentum_to_rounding() {
        let capsule = CapsuleStardust::sample_return_capsule();
        let mut vacuum = capsule;
        vacuum.drag_by_altitude = [0.0; 7];
        vacuum.drag_by_mach = [0.0; 19];
        // Free fall from entry passes 68.96 km at Mach 44, which the card
        // refuses. The copy reads its zero drag past Mach 40 and past the band
        // of air speeds, and the step is taken without the card's own checks.
        vacuum.drag_mach[DRAG_MACH.len() - 1] = 1.0e3;
        vacuum.air_speed_band_max_m_s = 1.0e9;
        assert!(!vacuum.is_valid());
        let mut state = CapsuleState::try_entry_interface(&capsule).unwrap();
        let mut frame = blank_frame();
        let start = recorded(&state);
        let energy_0 = rotating_frame_energy_j_kg(&capsule, start[0], start[1], start[2]);
        let fixed_0 = fixed_frame_energy_j_kg(&capsule, start);
        let momentum_0 = start[0] * start[2];
        let mut worst_energy = 0.0;
        let mut worst_fixed = 0.0;
        let mut worst_momentum = 0.0;
        let mut n = 0u32;
        while n < 1_600 {
            let predicted = integrate(&vacuum, recorded(&state), DT).unwrap();
            commit(&vacuum, &mut state, &mut frame, DT, predicted).unwrap();
            n += 1;
            let y = recorded(&state);
            assert_eq!(
                y[4].to_bits(),
                0.0_f64.to_bits(),
                "step {n} booked a drag loss"
            );
            let energy = rotating_frame_energy_j_kg(&capsule, y[0], y[1], y[2]);
            let drift = abs_f64(energy - energy_0) / ENERGY_SCALE_J_KG;
            if drift > worst_energy {
                worst_energy = drift;
            }
            let drift = abs_f64(fixed_frame_energy_j_kg(&capsule, y) - fixed_0) / ENERGY_SCALE_J_KG;
            if drift > worst_fixed {
                worst_fixed = drift;
            }
            let drift = abs_f64(y[0] * y[2] / momentum_0 - 1.0);
            if drift > worst_momentum {
                worst_momentum = drift;
            }
        }
        assert!(
            worst_energy < 1.0e-13,
            "rotating-frame energy drift {worst_energy}"
        );
        assert!(
            worst_fixed < 1.0e-13,
            "fixed-frame energy drift {worst_fixed}"
        );
        assert!(
            worst_momentum < 1.0e-13,
            "angular momentum drift {worst_momentum}"
        );
        assert_eq!(state.load_g.to_bits(), 0.0_f64.to_bits());
        // The stand-in's free fall after the same 1,600 steps.
        assert_eq!(state.radius_m.to_bits(), 6406972.490887238_f64.to_bits());
        assert_eq!(
            state.transverse_speed_m_s.to_bits(),
            12959.434329923115_f64.to_bits()
        );
        assert_eq!(state.altitude_m.to_bits(), 37743.97940878384_f64.to_bits());
        proof_intact(&frame);
    }

    #[test]
    fn the_entry_reproduces_the_table_form_reference_states_bit_for_bit() {
        let capsule = CapsuleStardust::sample_return_capsule();
        let mut state = CapsuleState::try_entry_interface(&capsule).unwrap();
        let mut frame = blank_frame();
        let mut row = 0;
        let mut n = 0u32;
        loop {
            if row < REFERENCE_STATES.len() && REFERENCE_STATES[row].0 == n {
                let (_, expect, flags) = REFERENCE_STATES[row];
                let got = [
                    state.t_s,
                    state.radius_m,
                    state.radial_speed_m_s,
                    state.transverse_speed_m_s,
                    state.angle_rad,
                    state.drag_loss_j_kg,
                    state.altitude_m,
                    state.air_speed_m_s,
                    state.mach,
                    state.load_g,
                    state.heat_flux_w_m2,
                    state.energy_defect_j_kg,
                ];
                let mut place = 0;
                while place < 12 {
                    assert_eq!(
                        got[place].to_bits(),
                        expect[place].to_bits(),
                        "step {n}, place {place}: card {}, stand-in {}",
                        got[place],
                        expect[place]
                    );
                    place += 1;
                }
                assert_eq!(flags_of(&state), flags, "step {n}");
                row += 1;
            }
            if n == 2_778 {
                break;
            }
            step(&capsule, &mut state, &mut frame, NO_LOAD, DT).unwrap();
            n += 1;
            assert!(state.residual_conservation_error <= 1.0e-12, "step {n}");
        }
        assert_eq!(row, REFERENCE_STATES.len());
        proof_intact(&frame);
    }

    #[test]
    fn the_full_entry_reproduces_the_judged_quantities_of_the_table_form_reference() {
        let capsule = CapsuleStardust::sample_return_capsule();
        let mut state = CapsuleState::try_entry_interface(&capsule).unwrap();
        let mut frame = blank_frame();
        let mut peak_load_g = 0.0;
        let mut peak_load_step = 0u32;
        let mut peak_load_time_s = 0.0;
        let mut peak_load_altitude_m = 0.0;
        let mut peak_heat_flux_w_m2 = 0.0;
        let mut peak_heat_flux_step = 0u32;
        let mut time_to_drogue_altitude_s = None;
        let mut time_to_drogue_mach_s = None;
        let mut worst_residual = 0.0;
        let mut landed = 0u32;
        loop {
            let before = state;
            match step(&capsule, &mut state, &mut frame, NO_LOAD, DT) {
                Ok(()) => landed += 1,
                Err(StepReject::PastFlightDeployment) => break,
                Err(other) => panic!("refused with {other:?} after {landed} steps"),
            }
            assert!(landed < 4_000, "the entry did not end");
            if state.residual_conservation_error > worst_residual {
                worst_residual = state.residual_conservation_error;
            }
            if state.load_g > peak_load_g {
                peak_load_g = state.load_g;
                peak_load_step = landed;
                peak_load_time_s = state.t_s;
                peak_load_altitude_m = state.altitude_m;
            }
            if state.heat_flux_w_m2 > peak_heat_flux_w_m2 {
                peak_heat_flux_w_m2 = state.heat_flux_w_m2;
                peak_heat_flux_step = landed;
            }
            // A straight line between the two steps either side of each crossing.
            if time_to_drogue_altitude_s.is_none()
                && before.altitude_m > FLIGHT_DROGUE_ALTITUDE_M
                && FLIGHT_DROGUE_ALTITUDE_M >= state.altitude_m
            {
                time_to_drogue_altitude_s = Some(
                    before.t_s
                        + DT * (before.altitude_m - FLIGHT_DROGUE_ALTITUDE_M)
                            / (before.altitude_m - state.altitude_m),
                );
            }
            if time_to_drogue_mach_s.is_none()
                && before.mach > FLIGHT_DROGUE_MACH
                && FLIGHT_DROGUE_MACH >= state.mach
            {
                time_to_drogue_mach_s = Some(
                    before.t_s
                        + DT * (before.mach - FLIGHT_DROGUE_MACH) / (before.mach - state.mach),
                );
            }
        }
        assert_eq!(
            peak_load_g.to_bits(),
            32.170160635699425_f64.to_bits(),
            "{peak_load_g}"
        );
        assert_eq!(peak_load_step, 1_188);
        assert_eq!(peak_load_time_s.to_bits(), 59.39999999999876_f64.to_bits());
        assert_eq!(
            peak_load_altitude_m.to_bits(),
            55109.348393220454_f64.to_bits()
        );
        let time_to_drogue_altitude_s = time_to_drogue_altitude_s.unwrap();
        let time_to_drogue_mach_s = time_to_drogue_mach_s.unwrap();
        assert_eq!(
            time_to_drogue_altitude_s.to_bits(),
            138.90860072165526_f64.to_bits(),
            "{time_to_drogue_altitude_s}"
        );
        assert_eq!(
            time_to_drogue_mach_s.to_bits(),
            136.95314361818805_f64.to_bits(),
            "{time_to_drogue_mach_s}"
        );
        assert_eq!(
            peak_heat_flux_w_m2.to_bits(),
            6961388.6908901455_f64.to_bits()
        );
        assert_eq!(peak_heat_flux_step, 995);
        assert_eq!(
            worst_residual.to_bits(),
            6.483048317015626e-13_f64.to_bits()
        );
        assert!(worst_residual < RESIDUAL_CONSERVATION_LIMIT / 1_000.0);
        // The last landed step is the first past both crossings, so both are
        // placed between two landed frames.
        assert_eq!(landed, 2_779);
        proof_intact(&frame);
    }

    /// The low tables are the 1976 standard's formulas, knot by knot, and end
    /// on the record's first knot.
    #[test]
    fn the_air_below_30_km_is_the_standards_formulas() {
        let g0 = 9.80665;
        let m0 = 0.0289644;
        let gas = 8.31432;
        let r0 = 6_356_766.0;
        let layers = [(0.0, 288.15, -0.0065), (11_000.0, 216.65, 0.0), (20_000.0, 216.65, 0.001)];
        let mut base_pa = [101_325.0, 0.0, 0.0];
        let mut index = 1;
        while index < 3 {
            let (hb, tb, lapse) = layers[index - 1];
            let h = layers[index].0;
            base_pa[index] = if lapse == 0.0 {
                base_pa[index - 1] * crate::math::exp(-g0 * m0 * (h - hb) / (gas * tb))
            } else {
                base_pa[index - 1]
                    * crate::math::exp(
                        (g0 * m0 / (gas * lapse)) * crate::math::log(tb / (tb + lapse * (h - hb))),
                    )
            };
            index += 1;
        }
        let mut knot = 0;
        while knot < LOW_KNOTS {
            let z = (knot as f64) * TABLE_KNOT_SPACING_M;
            let h = r0 * z / (r0 + z);
            let layer = if h >= 20_000.0 { 2 } else if h >= 11_000.0 { 1 } else { 0 };
            let (hb, tb, lapse) = layers[layer];
            let t = tb + lapse * (h - hb);
            let p = if lapse == 0.0 {
                base_pa[layer] * crate::math::exp(-g0 * m0 * (h - hb) / (gas * tb))
            } else {
                base_pa[layer] * crate::math::exp((g0 * m0 / (gas * lapse)) * crate::math::log(tb / t))
            };
            let rho = p * m0 / (gas * t);
            let c = (1.4 * gas * t / m0).sqrt();
            // Nine significant digits: within half a unit of the ninth.
            assert!(abs_f64(rho - AIR_DENSITY_LOW_KG_M3[knot]) <= 5.1e-9 * rho, "{z} m: {rho}");
            assert!(abs_f64(c - SOUND_SPEED_LOW_M_S[knot]) <= 5.1e-9 * c, "{z} m: {c}");
            knot += 1;
        }
        assert_eq!(AIR_DENSITY_LOW_KG_M3[0], 1.22499916);
        assert_eq!(SOUND_SPEED_LOW_M_S[0], 340.294108);
        assert_eq!(AIR_DENSITY_LOW_KG_M3[LOW_KNOTS - 1].to_bits(), AIR_DENSITY_KG_M3[0].to_bits());
        assert_eq!(SOUND_SPEED_LOW_M_S[LOW_KNOTS - 1].to_bits(), SOUND_SPEED_M_S[0].to_bits());
    }

    /// Where the card stood at 30 km and at the corridor's bottom of 31 km, a
    /// state low and fast is computed: from 31 010 m at Mach 1.234 the step of
    /// 0.05 s lands under 31 km; a start at 29 990 m and at 10 km lands; a
    /// dive at 1000 m/s from 50 m reaches sea level still above Mach 1.23 and
    /// the step that would go under it is refused by name, the state kept.
    #[test]
    fn a_low_fast_state_is_computed_down_to_sea_level() {
        let capsule = CapsuleStardust::sample_return_capsule();
        let sound_m_s = between_knots(&capsule, &capsule.sound_speed_m_s, 31_010.0).unwrap();
        let mut state = through_the_air(&capsule, 31_010.0, 1.234 * sound_m_s, -0.65).unwrap();
        let mut frame = blank_frame();
        step(&capsule, &mut state, &mut frame, NO_LOAD, DT).unwrap();
        assert!(state.altitude_m < CORRIDOR_BOTTOM_M);
        assert!(state.residual_conservation_error < RESIDUAL_CONSERVATION_LIMIT);
        for altitude_m in [29_990.0, 10_000.0, 1_000.0] {
            let sound_m_s = between_low_knots(&capsule, &capsule.sound_speed_low_m_s, altitude_m).unwrap();
            let mut state = through_the_air(&capsule, altitude_m, 2.0 * sound_m_s, -0.65).unwrap();
            let mut frame = blank_frame();
            step(&capsule, &mut state, &mut frame, NO_LOAD, 1.0e-4).unwrap();
            assert!(state.residual_conservation_error < RESIDUAL_CONSERVATION_LIMIT, "{altitude_m}");
            assert!(state.mach > 1.9 && state.mach < 2.0, "{altitude_m}: {}", state.mach);
        }
        // The dive.
        let mut state = through_the_air(&capsule, 50.0, 1000.0, -1.0).unwrap();
        let mut frame = blank_frame();
        let named = loop {
            let before = state_bits(&state);
            match step(&capsule, &mut state, &mut frame, NO_LOAD, 1.0e-4) {
                Ok(()) => assert!(state.residual_conservation_error < RESIDUAL_CONSERVATION_LIMIT),
                Err(reject) => {
                    assert_eq!(state_bits(&state), before);
                    break reject;
                }
            }
        };
        assert_eq!(named, StepReject::BelowStandardSeaLevel);
        assert!(state.altitude_m >= 0.0 && state.altitude_m < 1.0, "{}", state.altitude_m);
        assert!(state.mach > FLIGHT_DROGUE_MACH);
        assert_eq!(through_the_air(&capsule, -1.0, 1000.0, -1.0), Err(StepReject::BelowStandardSeaLevel));
        assert!(through_the_air(&capsule, 0.0, 1000.0, -1.0).is_ok());
    }

    /// Where the card stood at the corridor the locked entry flew, a state
    /// off it is computed on the same RK4 and ledger. 40 km at the entry's two
    /// speeds, the start engine 0.6.0 could not take on the ledger, lands a
    /// hundred steps of 0.005 s through a load of more than 100 g;
    /// 600 m/s straight down at 60 km lands; and a state moving upward above
    /// 69 km lands.
    #[test]
    fn off_the_compared_corridor_a_state_is_computed_on_the_ledger() {
        let capsule = CapsuleStardust::sample_return_capsule();
        let deep = CapsuleState::anchor(
            &capsule,
            LOCAL_EARTH_RADIUS_M + 40_000.0,
            capsule.entry_radial_speed_m_s,
            capsule.entry_transverse_speed_m_s,
        )
        .unwrap();
        let mut frame = blank_frame();
        let mut trial = deep;
        for _ in 0..100 {
            step(&capsule, &mut trial, &mut frame, NO_LOAD, 0.005).unwrap();
            assert!(trial.residual_conservation_error < RESIDUAL_CONSERVATION_LIMIT);
        }
        assert!(trial.load_g > 100.0, "{}", trial.load_g);
        let mut state = CapsuleState::try_entry_interface(&capsule).unwrap();
        state.radius_m = LOCAL_EARTH_RADIUS_M + 60_000.0;
        state.radial_speed_m_s = -600.0;
        state.transverse_speed_m_s = capsule.air_rotation_rate_rad_s * state.radius_m;
        let mut fresh = CapsuleState::anchor(&capsule, state.radius_m, state.radial_speed_m_s, state.transverse_speed_m_s).unwrap();
        step(&capsule, &mut fresh, &mut frame, NO_LOAD, DT).unwrap();
        for altitude_m in [DRAG_SWITCH_ALTITUDE_M, 68_990.0, CORRIDOR_TOP_M, 100_000.0] {
            let mut up = through_the_air(&capsule, altitude_m, 12_000.0, 0.01).unwrap();
            step(&capsule, &mut up, &mut frame, NO_LOAD, DT).unwrap();
            assert!(up.residual_conservation_error < RESIDUAL_CONSERVATION_LIMIT);
        }
        // The corridor is still the description of what was compared: at 40 km
        // 1147.51 to 1350.11 m/s.
        let [low, high, _, _] = corridor_at(40_000.0);
        assert!(abs_f64(low - 1147.51) < 1.0e-9 && abs_f64(high - 1350.11) < 1.0e-9);
    }

    /// Where the card stood at Mach 1.0, a state above 31 030 m at Mach 0.5
    /// is computed with the drag table's entries below Mach 1.0; Mach 0.16 is
    /// its first entry and is taken, and under it the state is refused by name.
    #[test]
    fn subsonic_states_above_the_drogue_altitude_are_computed_to_the_tables_first_entry() {
        let capsule = CapsuleStardust::sample_return_capsule();
        let sound_m_s = between_knots(&capsule, &capsule.sound_speed_m_s, 40_000.0).unwrap();
        for mach in [0.99, 0.5, MACH_FLOOR] {
            let mut state = through_the_air(&capsule, 40_000.0, mach * sound_m_s, -0.5).unwrap();
            let mut frame = blank_frame();
            step(&capsule, &mut state, &mut frame, NO_LOAD, DT).unwrap();
            assert!(state.residual_conservation_error < RESIDUAL_CONSERVATION_LIMIT, "{mach}");
        }
        assert_eq!(
            through_the_air(&capsule, 40_000.0, 0.159 * sound_m_s, -0.5),
            Err(StepReject::MachOutsideCard)
        );
    }

    #[test]
    fn the_locked_entry_lands_every_step_at_every_step_size() {
        // The compared entry, and the same entry at other steps the derivation
        // did not use, land every step to the first state past both deployment
        // conditions and refuse the next as PastFlightDeployment. Each landed
        // state is also a start the card takes.
        let capsule = CapsuleStardust::sample_return_capsule();
        for dt_s in [DT, 0.0371, 0.025, 0.0137, 0.005, 0.003] {
            let mut state = CapsuleState::try_entry_interface(&capsule).unwrap();
            let mut frame = blank_frame();
            let mut landed = 0u32;
            loop {
                match step(&capsule, &mut state, &mut frame, NO_LOAD, dt_s) {
                    Ok(()) => landed += 1,
                    Err(StepReject::PastFlightDeployment) => break,
                    Err(other) => panic!("dt {dt_s}: refused with {other:?} after {landed} steps"),
                }
                assert!(
                    CapsuleState::anchor(
                        &capsule,
                        state.radius_m,
                        state.radial_speed_m_s,
                        state.transverse_speed_m_s
                    )
                    .is_ok()
                        || (state.altitude_m < FLIGHT_DROGUE_ALTITUDE_M
                            && state.mach < FLIGHT_DROGUE_MACH),
                    "dt {dt_s}: step {landed} is not a start"
                );
            }
            assert!(state.altitude_m < FLIGHT_DROGUE_ALTITUDE_M && state.mach < FLIGHT_DROGUE_MACH);
            assert!(state.altitude_m > CORRIDOR_BOTTOM_M, "dt {dt_s}");
            let t_end = state.t_s;
            assert!(t_end > 138.9 && t_end < 138.9 + dt_s + 0.01, "dt {dt_s}: {t_end}");
        }
    }

    #[test]
    fn explicit_euler_is_refused_on_the_first_step() {
        let capsule = CapsuleStardust::sample_return_capsule();
        let mut state = CapsuleState::try_entry_interface(&capsule).unwrap();
        let saved = state_bits(&state);
        let mut frame = blank_frame();
        let held = frame.to_bytes();
        let got = step_euler(&capsule, &mut state, &mut frame, DT);
        assert_eq!(got, Err(StepReject::SpontaneousGeneration));
        assert_eq!(state_bits(&state), saved);
        assert_eq!(frame.to_bytes(), held);
        proof_intact(&frame);

        // How far over the limit: +3.7e-9 against 1e-9.
        let y = recorded(&state);
        let trial = axpy(y, DT, rates(&capsule, y).unwrap());
        let defect = rotating_frame_energy_j_kg(&capsule, trial[0], trial[1], trial[2])
            - state.energy_anchor_j_kg
            + trial[4];
        let residual = defect / ENERGY_SCALE_J_KG;
        assert_eq!(
            residual.to_bits(),
            3.73494792736566e-09_f64.to_bits(),
            "{residual}"
        );

        // The honest step from the same state lands.
        step(&capsule, &mut state, &mut frame, NO_LOAD, DT).unwrap();
        assert!(state.residual_conservation_error < 1.0e-15);
        proof_intact(&frame);
    }

    #[test]
    fn explicit_euler_at_a_fine_step_adds_up_to_a_refusal() {
        let capsule = CapsuleStardust::sample_return_capsule();
        let mut state = CapsuleState::try_entry_interface(&capsule).unwrap();
        let mut frame = blank_frame();
        let mut previous = 0.0;
        let mut landed = 0u32;
        loop {
            let held = state_bits(&state);
            let held_frame = frame.to_bytes();
            match step_euler(&capsule, &mut state, &mut frame, 0.001) {
                Ok(()) => {
                    landed += 1;
                    assert!(
                        state.energy_defect_j_kg > previous,
                        "step {landed} defect {}",
                        state.energy_defect_j_kg
                    );
                    previous = state.energy_defect_j_kg;
                    assert!(landed < 5_000, "euler did not refuse");
                }
                Err(StepReject::SpontaneousGeneration) => {
                    assert!(landed > 100, "refused after only {landed} steps");
                    assert_eq!(state_bits(&state), held);
                    assert_eq!(frame.to_bytes(), held_frame);
                    proof_intact(&frame);
                    break;
                }
                Err(other) => panic!("euler refused with {other:?}"),
            }
        }
    }

    #[test]
    fn a_ledger_that_forgets_the_air_turns_is_refused() {
        let capsule = CapsuleStardust::sample_return_capsule();
        // The same commit, read from a copy whose air does not turn, anchored
        // at the fixed-frame energy. The step itself is the honest one.
        let mut still = capsule;
        still.air_rotation_rate_rad_s = 0.0;
        // Without the turning air the copy sees the entry at 12 900 m/s, above
        // the band; the copy's band is opened so its ledger is what refuses.
        still.air_speed_band_max_m_s = 13_000.0;
        let mut state = CapsuleState::try_entry_interface(&capsule).unwrap();
        let y = recorded(&state);
        state.energy_anchor_j_kg = fixed_frame_energy_j_kg(&capsule, y);
        let saved = state_bits(&state);
        let mut frame = blank_frame();
        let held = frame.to_bytes();
        let got = step_with_copies(&capsule, &capsule, &still, &mut state, &mut frame, DT);
        assert_eq!(got, Err(StepReject::FirstLawBreach));
        assert_eq!(state_bits(&state), saved);
        assert_eq!(frame.to_bytes(), held);
        proof_intact(&frame);

        // How far: −2.2e-9 after one step of 0.05 s.
        let trial = integrate(&capsule, y, DT).unwrap();
        let defect = fixed_frame_energy_j_kg(&capsule, trial) - state.energy_anchor_j_kg + trial[4];
        let residual = defect / ENERGY_SCALE_J_KG;
        assert_eq!(
            residual.to_bits(),
            (-2.2464110355634345e-09_f64).to_bits(),
            "{residual}"
        );

        // The ledger that counts the turning air lands the same step.
        let mut honest = CapsuleState::try_entry_interface(&capsule).unwrap();
        step(&capsule, &mut honest, &mut frame, NO_LOAD, DT).unwrap();
        assert_eq!(honest.radius_m.to_bits(), trial[0].to_bits());
        assert!(honest.residual_conservation_error < 1.0e-15);
        proof_intact(&frame);
    }

    #[test]
    fn a_biased_drag_loss_is_refused_both_ways() {
        let capsule = CapsuleStardust::sample_return_capsule();
        let mut state = CapsuleState::try_entry_interface(&capsule).unwrap();
        let mut frame = blank_frame();
        step(&capsule, &mut state, &mut frame, NO_LOAD, DT).unwrap();
        let held = state_bits(&state);
        let held_frame = frame.to_bytes();
        let extra = ENERGY_SCALE_J_KG * 1.0e-3;
        let got = step_bias_drag_loss(&capsule, &mut state, &mut frame, DT, extra);
        assert_eq!(got, Err(StepReject::SpontaneousGeneration));
        assert_eq!(state_bits(&state), held);
        assert_eq!(frame.to_bytes(), held_frame);
        let got = step_bias_drag_loss(&capsule, &mut state, &mut frame, DT, -extra);
        assert_eq!(got, Err(StepReject::FirstLawBreach));
        assert_eq!(state_bits(&state), held);
        assert_eq!(frame.to_bytes(), held_frame);
        proof_intact(&frame);
        // No bias: the same helper lands.
        step_bias_drag_loss(&capsule, &mut state, &mut frame, DT, 0.0).unwrap();
        proof_intact(&frame);
    }

    #[test]
    fn drag_loss_booked_at_the_starting_rate_is_refused() {
        let capsule = CapsuleStardust::sample_return_capsule();
        let near_peak = reference(1_189);
        let mut state = CapsuleState::try_state(
            &capsule,
            near_peak[RADIUS],
            near_peak[RADIAL],
            near_peak[TRANSVERSE],
        )
        .unwrap();
        assert_eq!(state.load_g.to_bits(), near_peak[LOAD].to_bits());
        let saved = state_bits(&state);
        let mut frame = blank_frame();
        let held = frame.to_bytes();
        let got = step_booked_at_the_starting_rate(&capsule, &mut state, &mut frame, DT);
        assert_eq!(got, Err(StepReject::SpontaneousGeneration));
        assert_eq!(state_bits(&state), saved);
        assert_eq!(frame.to_bytes(), held);
        proof_intact(&frame);
        step(&capsule, &mut state, &mut frame, NO_LOAD, DT).unwrap();
        assert!(state.residual_conservation_error < 1.0e-12);
    }

    #[test]
    fn a_step_ten_times_the_ceiling_is_stopped_by_the_ledger() {
        let capsule = CapsuleStardust::sample_return_capsule();
        let mut state = CapsuleState::try_entry_interface(&capsule).unwrap();
        let mut frame = blank_frame();
        let mut landed = 0u32;
        loop {
            let held = state_bits(&state);
            let held_frame = frame.to_bytes();
            match step(&capsule, &mut state, &mut frame, NO_LOAD, 10.0 * DT) {
                Ok(()) => {
                    landed += 1;
                    assert!(landed < 286, "honest RK4 at 0.5 s reached the floor");
                    assert!(state.residual_conservation_error <= RESIDUAL_CONSERVATION_LIMIT);
                }
                Err(StepReject::SpontaneousGeneration) => {
                    assert_eq!(state_bits(&state), held);
                    assert_eq!(frame.to_bytes(), held_frame);
                    proof_intact(&frame);
                    break;
                }
                Err(other) => panic!("refused with {other:?} after {landed} steps"),
            }
        }
        assert!(landed > 10, "refused after only {landed} steps");
        assert!(state.altitude_m > 60_000.0);
    }

    #[test]
    fn every_edge_of_the_card_is_refused_by_name() {
        let capsule = CapsuleStardust::sample_return_capsule();
        let mut frame = blank_frame();
        let held = frame.to_bytes();
        let mut refuse = |mut state: CapsuleState, dt_s: f64, expect: StepReject, what: &str| {
            let saved = state_bits(&state);
            let got = step(&capsule, &mut state, &mut frame, NO_LOAD, dt_s);
            assert_eq!(got, Err(expect), "{what}");
            assert_eq!(state_bits(&state), saved, "{what}");
            assert_eq!(frame.to_bytes(), held, "{what}");
            proof_intact(&frame);
        };
        let entry = CapsuleState::try_entry_interface(&capsule).unwrap();

        refuse(entry, 0.0, StepReject::NonPositiveDt, "a zero step");
        refuse(entry, -DT, StepReject::NonPositiveDt, "a negative step");
        refuse(entry, f64::NAN, StepReject::NonFiniteInput, "a NaN step");
        refuse(
            entry,
            f64::INFINITY,
            StepReject::NonFiniteInput,
            "an infinite step",
        );
        refuse(
            entry,
            f64::NEG_INFINITY,
            StepReject::NonFiniteInput,
            "a step of minus infinity",
        );

        let mut broken = entry;
        broken.transverse_speed_m_s = f64::NAN;
        refuse(broken, DT, StepReject::NonFiniteState, "NaN in the state");
        let mut broken = entry;
        broken.radius_m = f64::INFINITY;
        refuse(broken, DT, StepReject::NonFiniteState, "an infinite radius");
        let mut broken = entry;
        broken.drag_loss_j_kg = f64::NAN;
        refuse(broken, DT, StepReject::NonFiniteState, "NaN in the ledger");

        let mut above = entry;
        above.radius_m = LOCAL_EARTH_RADIUS_M + 141_000.0;
        refuse(
            above,
            DT,
            StepReject::AltitudeOutsideCard,
            "a state at 141 km",
        );
        // 134.8 km is above the drag table. It is refused; no drag value is held there.
        let mut above = entry;
        above.radius_m = LOCAL_EARTH_RADIUS_M + 134_800.0;
        refuse(
            above,
            DT,
            StepReject::AltitudeOutsideCard,
            "a state at 134.8 km",
        );
        let mut below = entry;
        below.radius_m = LOCAL_EARTH_RADIUS_M - 100.0;
        refuse(
            below,
            DT,
            StepReject::BelowStandardSeaLevel,
            "a state 100 m under sea level",
        );
        let mut slow = entry;
        slow.radius_m = LOCAL_EARTH_RADIUS_M + 35_000.0;
        slow.radial_speed_m_s = -20.0;
        slow.transverse_speed_m_s = capsule.air_rotation_rate_rad_s * slow.radius_m + 10.0;
        refuse(
            slow,
            DT,
            StepReject::MachOutsideCard,
            "a state below Mach 0.16",
        );

        // A stage of one step of 100 s from entry is thrown far outside the
        // band of air speeds before it leaves the altitudes.
        refuse(
            entry,
            100.0,
            StepReject::AirSpeedOutsideCard,
            "one step of 100 s from entry",
        );
        let late = reference(2_778);
        let low = CapsuleState::try_state(&capsule, late[RADIUS], late[RADIAL], late[TRANSVERSE])
            .unwrap();
        assert_eq!(low.altitude_m.to_bits(), late[ALTITUDE].to_bits());
        // A step far too long from the last landed state is refused by name.
        let mut trial = low;
        let mut scratch = blank_frame();
        let named = step(&capsule, &mut trial, &mut scratch, NO_LOAD, 10.0);
        assert!(named.is_err(), "{named:?}");
        assert_eq!(trial, low);

        let at = |altitude_m: f64, radial_speed_m_s: f64, transverse_speed_m_s: f64| {
            CapsuleState::try_state(
                &capsule,
                LOCAL_EARTH_RADIUS_M + altitude_m,
                radial_speed_m_s,
                transverse_speed_m_s,
            )
        };
        assert!(at(141_000.0, -1_800.0, 12_700.0).is_none());
        assert!(at(134_800.0, -1_800.0, 12_700.0).is_none());
        assert!(at(134_600.0, -1_800.0, 12_700.0).is_some());
        // 29 km and 31 km at 425 m/s through the air, Mach 1.41 and 1.40, off
        // the corridor the locked entry flew and, at 29 km, under the record's
        // air tables, are computed.
        assert!(at(29_000.0, -236.0, 700.0).is_some());
        assert!(CapsuleState::anchor(&capsule, LOCAL_EARTH_RADIUS_M + 31_000.0, -236.0, 700.0).is_ok());
        let sound_m_s = between_knots(&capsule, &capsule.sound_speed_m_s, 31_200.0).unwrap();
        assert!(through_the_air(&capsule, 31_200.0, 1.2 * sound_m_s, -0.63).is_ok());
        // 35 km at Mach 0.38, where the card stood at Mach 1.0, is computed.
        assert!(at(35_000.0, -100.0, 400.0).is_some());
        assert!(at(35_000.0, -20.0, 357.0).is_none());
        assert!(at(35_000.0, f64::NAN, 400.0).is_none());
        assert!(at(f64::INFINITY, -100.0, 700.0).is_none());
        assert!(at(35_000.0, -100.0, f64::NEG_INFINITY).is_none());

        // 30.5 km at Mach 1.01 is past deployment and is refused as a start.
        assert!(
            CapsuleState::try_state(&capsule, 6399728.511478454, -236.0, 540.1499116372644)
                .is_none()
        );
        assert_eq!(
            CapsuleState::anchor(&capsule, 6399728.511478454, -236.0, 540.1499116372644),
            Err(StepReject::PastFlightDeployment)
        );
        // From 40 km at Mach 1.01 and Mach 0.99, falling at 20 m/s: off the
        // corridor the locked entry flew, 1147.51 to 1350.11 m/s at 40 km, and
        // computed; engines 0.7 to 0.9 refused both.
        let radius_m = LOCAL_EARTH_RADIUS_M + 40_000.0;
        let sound_m_s = between_knots(&capsule, &capsule.sound_speed_m_s, 40_000.0).unwrap();
        let along_m_s = ((1.01 * sound_m_s) * (1.01 * sound_m_s) - 20.0 * 20.0).sqrt();
        assert_eq!(
            CapsuleState::anchor(
                &capsule,
                radius_m,
                -20.0,
                along_m_s + capsule.air_rotation_rate_rad_s * radius_m,
            )
            .is_ok(),
            true
        );
        let along_m_s = ((0.99 * sound_m_s) * (0.99 * sound_m_s) - 20.0 * 20.0).sqrt();
        assert!(CapsuleState::anchor(
            &capsule,
            radius_m,
            -20.0,
            along_m_s + capsule.air_rotation_rate_rad_s * radius_m,
        )
        .is_ok());
    }

    #[test]
    fn flags_are_only_flags() {
        let capsule = CapsuleStardust::sample_return_capsule();
        let mut state = CapsuleState::try_entry_interface(&capsule).unwrap();
        assert_eq!(flags_of(&state), 0);
        let mut frame = blank_frame();
        let mut first_in_window = None;
        let mut n = 0u32;
        while n < 2_779 {
            let before = state;
            step(&capsule, &mut state, &mut frame, NO_LOAD, DT).unwrap();
            n += 1;
            assert_eq!(frame.flags, flags_of(&state), "step {n}");
            assert_eq!(frame.flags & !FLAG_DROGUE_MACH_WINDOW, 0);
            if first_in_window.is_none() && state.drogue_mach_window {
                first_in_window = Some(n);
                assert!(before.mach > 1.6 && state.mach <= 1.6 && state.mach >= 1.2);
                assert_eq!(frame.flags, FLAG_DROGUE_MACH_WINDOW);
            }
        }
        assert_eq!(first_in_window, Some(2_543));
        // Past Mach 1.2 the window flag clears. The step that crosses both
        // deployment conditions still lands; the next is refused by name.
        assert!(state.mach < 1.2);
        assert_eq!(frame.flags, 0);
        assert_eq!(
            step(&capsule, &mut state, &mut frame, NO_LOAD, DT),
            Err(StepReject::PastFlightDeployment)
        );
        proof_intact(&frame);
    }

    #[test]
    fn a_frame_is_not_a_state() {
        // A sealed run is continued from the full 64-bit state, never from a
        // frame. This test shows the difference. It is expected.
        let capsule = CapsuleStardust::sample_return_capsule();
        let mut running = CapsuleState::try_entry_interface(&capsule).unwrap();
        let mut frame = blank_frame();
        let mut n = 0u32;
        while n < 1_189 {
            step(&capsule, &mut running, &mut frame, NO_LOAD, DT).unwrap();
            n += 1;
        }
        // The frame holds the altitude and the recovery enthalpy in 32 bits:
        // neither the radius nor the air speed comes back exactly, and the
        // radial speed, the angle and the drag loss are not in it at all.
        let radius_m = f64::from(frame.pos[0]) + LOCAL_EARTH_RADIUS_M;
        let air_speed_m_s = (2.0 * f64::from(frame.vel[0])).sqrt();
        assert!(radius_m != running.radius_m);
        assert!(air_speed_m_s != running.air_speed_m_s);
        let apart_m_s = abs_f64(air_speed_m_s - running.air_speed_m_s);
        assert!(apart_m_s > 1.0e-6 && apart_m_s < 1.0e-2, "{apart_m_s} m/s apart");
        assert!(running.drag_loss_j_kg > 4.6e7);
        proof_intact(&frame);
    }

    #[test]
    fn the_frame_carries_the_documented_slots() {
        let capsule = CapsuleStardust::sample_return_capsule();
        let mut state = CapsuleState::try_entry_interface(&capsule).unwrap();
        let mut frame = blank_frame();
        let mut n = 0u32;
        while n < 1_000 {
            step(&capsule, &mut state, &mut frame, NO_LOAD, DT).unwrap();
            n += 1;
        }
        let expect = reference(1_000);
        assert_eq!(frame.t.to_bits(), expect[TIME].to_bits());
        assert_eq!(frame.pos[0], expect[ALTITUDE] as f32);
        assert_eq!(frame.pos[1], state.stagnation_pressure_pa as f32);
        assert_eq!(frame.pos[2], expect[MACH] as f32);
        assert_eq!(frame.vel[0], (0.5 * expect[AIR_SPEED] * expect[AIR_SPEED]) as f32);
        assert_eq!(frame.vel[1], state.heat_transfer_coefficient_kg_m2_s as f32);
        assert_eq!(frame.vel[2], expect[HEAT_FLUX] as f32);
        assert_eq!(frame.force_torque, expect[LOAD] as f32);
        assert_eq!(
            frame.residual,
            (abs_f64(expect[DEFECT]) / ENERGY_SCALE_J_KG) as f32
        );
        assert_eq!(frame.flags, 0);
        assert_eq!(state.drag_loss_j_kg.to_bits(), expect[DRAG_LOSS].to_bits());
        assert_eq!(state.recovery_enthalpy_j_kg.to_bits(), (0.5 * expect[AIR_SPEED] * expect[AIR_SPEED]).to_bits());
        // The coefficient times the recovery enthalpy is the heat flux.
        let product = state.heat_transfer_coefficient_kg_m2_s * state.recovery_enthalpy_j_kg;
        assert!(abs_f64(product - state.heat_flux_w_m2) < 1.0e-14 * state.heat_flux_w_m2);
        // At Mach 34 the stagnation pressure is near its hypersonic limit,
        // 0.92 of rho u^2 for gamma 1.4.
        let rho = state.heat_transfer_coefficient_kg_m2_s / (2.0 * capsule.heat_flux_constant * state.air_speed_m_s);
        let rho = rho * rho * capsule.nose_radius_m;
        let ratio = state.stagnation_pressure_pa / (rho * state.air_speed_m_s * state.air_speed_m_s);
        assert!(ratio > 0.919 && ratio < 0.921, "{ratio}");
        // 32 bits hold the altitude to a few millimetres and the load to a few millionths of a g.
        assert!(abs_f64(f64::from(frame.pos[0]) - state.altitude_m) < 0.005);
        assert!(abs_f64(f64::from(frame.force_torque) - state.load_g) < 2.0e-6);
        proof_intact(&frame);
    }

    /// The stagnation pressure over the static pressure, against the
    /// isentropic and normal-shock tables for gamma 1.4 (NACA Report 1135):
    /// 1.0283 at Mach 0.2, 1.8929 at Mach 1 from both sides, 5.6404 at Mach
    /// 2, 129.22 at Mach 10.
    #[test]
    fn the_stagnation_pressure_is_the_isentropic_and_pitot_pressure() {
        let at = |mach: f64| {
            let air = super::Air {
                altitude_m: 40_000.0,
                radial_m_s: 0.0,
                transverse_m_s: 0.0,
                air_speed_m_s: mach * 300.0,
                density_kg_m3: 1.4,
                sound_speed_m_s: 1.0,
                mach,
                drag_acceleration_m_s2: 0.0,
            };
            super::stagnation_pressure_pa(&air)
        };
        for (mach, table, within) in [(0.2, 1.0283, 1.0e-4), (2.0, 5.6404, 1.0e-4), (10.0, 129.22, 1.0e-2)] {
            assert!(abs_f64(at(mach) - table) < within, "Mach {mach}: {}", at(mach));
        }
        let below = at(1.0 - 1.0e-12);
        let above = at(1.0);
        assert!(abs_f64(below - 1.892929158737854) < 1.0e-9, "{below}");
        assert!(abs_f64(above - below) < 1.0e-9, "{above} {below}");
    }

    #[test]
    fn the_proof_bytes_are_never_touched() {
        let capsule = CapsuleStardust::sample_return_capsule();
        let mut state = CapsuleState::try_entry_interface(&capsule).unwrap();
        let mut frame = blank_frame();
        let mut n = 0;
        while n < 4 {
            step(&capsule, &mut state, &mut frame, NO_LOAD, DT).unwrap();
            proof_intact(&frame);
            let bytes = frame.to_bytes();
            let mut index = 48;
            while index < 64 {
                assert_eq!(bytes[index], 0xA5);
                index += 1;
            }
            n += 1;
        }
        let got = step(&capsule, &mut state, &mut frame, NO_LOAD, 0.0);
        assert_eq!(got, Err(StepReject::NonPositiveDt));
        proof_intact(&frame);
        let got = step(&capsule, &mut state, &mut frame, NO_LOAD, f64::NAN);
        assert_eq!(got, Err(StepReject::NonFiniteInput));
        proof_intact(&frame);
        let got = step(&capsule, &mut state, &mut frame, NO_LOAD, 100.0);
        assert_eq!(got, Err(StepReject::AirSpeedOutsideCard));
        proof_intact(&frame);
        let got = step_euler(&capsule, &mut state, &mut frame, DT);
        assert_eq!(got, Err(StepReject::SpontaneousGeneration));
        proof_intact(&frame);
        let mut tampered = capsule;
        tampered.capsule_mass_kg = 46.0;
        let got = step(&tampered, &mut state, &mut frame, NO_LOAD, DT);
        assert_eq!(got, Err(StepReject::InvalidCapsule));
        proof_intact(&frame);
    }

    #[test]
    fn a_step_at_the_floor_moves_the_clock_and_every_integrated_quantity() {
        let card = super::CapsuleStardust::sample_return_capsule();
        let ceiling_s = card.gate_dt_s().unwrap();
        let floor_s = card.floor_dt_s().unwrap();
        assert_eq!(super::GATE_OVER_FLOOR.to_bits(), 1_048_576.0_f64.to_bits());
        assert_eq!(
            floor_s.to_bits(),
            (ceiling_s / super::GATE_OVER_FLOOR).to_bits()
        );

        // The locked entry state.
        let mut state = super::CapsuleState::try_entry_interface(&card).unwrap();
        let load = super::CapsuleLoad {};
        let before = state;
        let mut frame = crate::frame::LastStateFrame64 {
            t: 0.0,
            pos: [0.0; 3],
            vel: [0.0; 3],
            force_torque: 0.0,
            residual: 0.0,
            flags: 0,
            proof: [0; 16],
        };
        super::step(&card, &mut state, &mut frame, load, floor_s).unwrap();
        assert_eq!(state.t_s.to_bits(), floor_s.to_bits());
        assert!(state.radius_m != before.radius_m);
        assert!(state.radial_speed_m_s != before.radial_speed_m_s);
        assert!(state.transverse_speed_m_s != before.transverse_speed_m_s);
        assert!(state.angle_rad != before.angle_rad);
        assert!(state.drag_loss_j_kg != before.drag_loss_j_kg);
        assert!(state.residual_conservation_error <= super::RESIDUAL_CONSERVATION_LIMIT);

        // A thousand more steps at the floor land: the ledger holds an
        // honest step this small.
        let mut taken = 0;
        while taken < 1_000 {
            super::step(&card, &mut state, &mut frame, load, floor_s).unwrap();
            taken += 1;
        }
        assert!(state.residual_conservation_error <= super::RESIDUAL_CONSERVATION_LIMIT);

        // At the largest time a run reaches, the clock still moves by a
        // floor step, to within a thousandth of it.
        let late_s = 1.0e6 * ceiling_s;
        let moved_s = (late_s + floor_s) - late_s;
        assert!(
            (moved_s - floor_s).abs() <= 1.0e-3 * floor_s,
            "the clock moved {moved_s} for a step of {floor_s}"
        );
    }
}
