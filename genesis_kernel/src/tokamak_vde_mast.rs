#![no_implicit_prelude]

//! The vertical displacement event of the MAST tokamak's plasma: the plasma
//! column's height and vertical velocity after its position feedback is cut,
//! held back by the eddy currents of MAST's passive structure and by the P6
//! vertical-control coils, until its boundary reaches the wall.
//!
//! The company's own derivation, from the MAST tokamak (UKAEA, Culham) at
//! 0.250 s of shot 15045, the EFIT time slice just before the vertical-position
//! feedback was cut. Every number comes from the FAIR-MAST level-2 data of that
//! shot (S. Jackson et al. and the MAST Team, SoftwareX 27 (2024) 101869; IEEE
//! TPS (2025), doi 10.1109/TPS.2025.3583419), which are under Creative Commons
//! Attribution-ShareAlike 4.0; the derivation is
//! `evidence/research/tokamak_vde_mast/analysis/derive_card.py`, its method
//! fixed in `FORECAST.md` there and committed before any growth rate was
//! computed. No measured trace is stored in the card.
//!
//! What the card is. A rigid plasma: EFIT's toroidal current density at 0.250 s,
//! 878 cells and 755.3 kA, shifted bodily in height by `z`, its current and its
//! mass held. The held field is MAST's PF coils at their 0.250 s currents (the
//! solenoid, P2 to P5; each element of `pf_active` one turn, the channel's
//! current the coil's ampere-turns, the solenoid's elements half its channel)
//! and the passive currents MAST's AMM signals give at that time; with them and
//! the plasma, EFIT's flux on its grid outside the plasma is reproduced to 1.1
//! percent of its variation, root mean square (0.73 percent with each coil's
//! scale fitted free, which is how the convention was found). The up and down halves of P2, P4 and P5 carry the
//! same current through the event, so they are held. The 126 passive elements
//! of `pf_passive` (the vessel, the centre column's Inconel casing and graphite,
//! the P2 arms and divertor plates, and the 48 stainless coil-case sheets), with
//! resistivities from MAST's AMM signals where a flux loop is near (Inconel
//! 1.29e-6, graphite 13.5e-6, divertor plates 3.3e-6 ohm m) and stainless steel
//! 304 (0.72e-6) elsewhere, are one wall circuit in the shape of the growing
//! mode of the 127-circuit system. P6, the radial-field pair of 4 turns each in
//! anti-series, is a circuit of its own, copper at 20 degC, driven by the fast
//! amplifier's voltage: the caller's load. In shot 15045 the amplifier's drive
//! is zero from 0.250 s, and P6 then carries the current of a flux conserver.
//!
//! ```text
//! x          = (2 z − (z_up + z_lo)) / (z_up − z_lo)                     the series' variable
//! m dv/dt    = G(z) + Ip (D_w(z) I_w + D_c(z) I_c)                       G = −dU/dz
//! L_w dI_w/dt + M dI_c/dt + R_w I_w = −Ip D_w(z) v
//! M dI_w/dt + L_c dI_c/dt + R_c I_c = V − Ip D_c(z) v
//! E          = ½ m v² + U(z) + ½ L_w I_w² + M I_w I_c + ½ L_c I_c²
//! dE/dt      = V I_c − R_w I_w² − R_c I_c²
//! ```
//!
//! `G`, `U`, `D_w` and `D_c` are Chebyshev series in `x` from the lower to the
//! upper contact height, each of the lowest degree whose largest deviation from
//! the computed values at 401 heights is under 1e-9 of its range. `D_w` and
//! `D_c` are the height derivatives of the mutual inductance of the shifted
//! plasma with the wall mode and with P6, per ampere of Ip. The interaction of
//! the plasma with the two circuits is work the plasma's force and the induced
//! voltage exchange: it cancels between the first and the last two equations,
//! and the plasma's current is held, so `E` holds the circuits' own energy and
//! the plasma's energy in the held field.
//!
//! The ledger. Over each substep the losses and the supply's work are
//! integrated by the corrected trapezoid rule on the power and its rate at the
//! two ends, which the Runge-Kutta step does not use:
//!
//! ```text
//! defect   = E − E_anchor − (Σ Q_supply − Σ Q_loss)
//! Q        = h/2 (P0 + P1) + h²/12 (P0' − P1')
//! scale    = max(|E|_now, |E|_anchor) + Σ Q_loss + |Σ Q_supply|        |E| the sum of each term's size
//! residual = |defect| / max(scale, 0.01 J)
//! ```
//!
//! The growth comes from the physics, not a fit. At z = 0 the held field pushes
//! the shifted plasma with a stiffness of 28,022.5 N/m; the 127 circuits as
//! perfect conductors would push back 5.13 times as hard (a stability margin of
//! 4.13; P6 alone 0.75); the card's two circuits have a margin of 1.27, the wall
//! mode's time is 14.29 ms and P6's 172 ms. The card's linear system at z = 0
//! has a growing pole at 6.5465 per second, the same as the 127-circuit system
//! to seven figures, a wall pole at −61.40 per second, and the plasma's inertial
//! oscillation at 2.44e5 rad/s (period 25.8 µs). The stiffness rises to
//! 109,777 N/m at the upper contact and 109,812 N/m at the lower, the slope of
//! the force series at either end.
//!
//! The held field's force on the unshifted plasma, 113.3 N upward by the
//! derivation's direct sum, is taken out of the force series
//! (`derive_card.py`, stage 4: set to zero, the plasma in equilibrium at
//! z = 0). It is the force of 4.04 mm of the stiffness at z = 0, and over the
//! rise to the upper contact its work would be 44.1 J against the 3,209 J the
//! held field releases there (44.3 J against 3,245 J down to the lower
//! contact).
//!
//! The comparison, its marks fixed before any growth rate was computed
//! (`FORECAST.md`): the measured growth of shot 15045 is 96.3 per second from
//! the centre-column Bv signal (Windridge et al., 34th EPS Conference (2007)
//! P1.108; no window is stated), and 121.3 per second by the forecast's own
//! rule on the same probes (0.2608 to 0.2746 s, 1.97 to 9.05 cm). The card's
//! linear growth rate, 6.55 per second, is 93 percent under the first: line 1
//! is missed. Line 2: the card's own synthetic signal, from its sealed run
//! (729 steps of 1 ms from the growing mode at 1 mm, ended at the wall), grows
//! at 7.10 per second by the same rule, 94.1 percent under the measured 121.3:
//! missed. Line 3: at the ends of the two windows P6U carries −2,933
//! ampere-turns in the card and −3,126 in the shot, 6.2 percent low: met, with
//! the two events at different speeds (on the card's slow growth P6 carries
//! 54 percent of the current that would hold its flux). One line of three:
//! met in part.
//!
//! Where the card misses, in numbers, found after the scoring. At 0.280 s
//! P6's measured current, 5,011 ampere-turns (−4,970 on P6U, +5,053 on P6L),
//! is 98.7 percent of the 5,078 that hold its flux for EFIT's axis there, 8.03
//! cm above its place at 0.250 s; with that current P6 alone pushes the plasma
//! back harder than the held field's drive at that height, 3,870 N against
//! 2,350 N. With P6 open the 127
//! circuits would grow at 90.7 per second; with P6 closed, as the data show it
//! was, they need 2.04 times the stiffness of the rigid shift to grow at 96.3.
//! The plasma did not move rigidly: by 0.280 s its boundary's centre had risen
//! 0.175 m and its magnetic axis 0.083 m. A rigid shift of the 0.250 s current
//! has half the drive the event shows.
//!
//! What is left out. The plasma's deformation and every change of its current
//! and shape (by 0.280 s the boundary's centre had risen 0.175 m and the
//! magnetic axis 0.083 m); the PF currents' programmed change (P2 falls from 70
//! to 53 kA-turns between 0.250 and 0.280 s); every wall mode but one; the
//! amplifier's and the feeders' resistance; halo currents, the thermal and the
//! current quench after contact; every other time of shot 15045 and every other
//! shot and tokamak.
//!
//! The named limits, each enforced and none pulled back: the start inside the
//! vessel (`StartOutsideVessel`); the plasma's speed under its Alfvén speed at
//! the start and at every stage of every substep (`SpeedAtAlfvenSpeed`); the
//! boundary reaching the limiter during a step, which ends the event
//! (`PlasmaAtWall`, tested at the same stages); the substep budget
//! (`SubstepsAboveBudget`); and the ledger. The speed and the wall are tested
//! at the same points, so which of the two a run meets does not follow the
//! step asked for: from rest at z = 0 with 1 MA in P6 every step from 1e-8 s to
//! 1 ms meets the Alfvén speed at 0.30 m, before the wall. The order can differ
//! between two steps only where the plasma would reach both within one
//! substep, whose stages fall at other times when the step is cut otherwise.
//!
//! The step and its ceiling. A step holds the amplifier's voltage and is cut
//! into substeps of the classical fourth-order Runge-Kutta method, each no
//! longer than [`SUBSTEP_POLE_FRACTION`] of the inverse of Fujiwara's bound on
//! the roots of the characteristic polynomial of the rates' Jacobian at the
//! substep's start: every pole of the step's linearisation lies within that
//! bound, the fastest being the plasma's inertial oscillation, 3.03e5 per
//! second at the most over the span with no current. A step that would take
//! more than [`MAX_SUBSTEPS`] is refused. [`TokamakVdeMast::gate_dt_s`] is
//! [`GATE_DT_S`], 1 ms: about 24,400 substeps at the bound of the growing mode,
//! and under a sixteenth of the time of the wall pole. Over hard starts (at
//! rest 0.30 m off balance, moving at 20,000 m/s, the wall carrying 1 MA, P6
//! carrying 100 kA, under 0, 300 and −3,000 V) the largest residual found at
//! steps of 0.1 and 1 ms is 8.6e-10, and the limit
//! [`RESIDUAL_CONSERVATION_LIMIT`] is 2e-8; a step without substeps at 6.6e-7 s
//! from the same starts gives 0.51. [`TokamakVdeMast::floor_dt_s`] is
//! [`FLOOR_DT_S`], the ceiling divided by 2^20, 9.54e-10 s, a step that still
//! moves a clock of 100 s. The public
//! step applies neither; the gateway refuses a step outside them.
//!
//! `core` only: no heap and no standard prelude. The state is `Copy`. An
//! accepted step writes time, the displacement, the wall-mode and P6 currents,
//! the velocity, the wall's resistive power, the gap to the wall, the wall's
//! force on the plasma and the conservation residual into
//! [`LastStateFrame64`]. The 16-byte frame proof is not touched.

extern crate core;

use core::prelude::v1::*;

use crate::exceedance::{Exceedance, ExceedanceKind, Side};
use crate::frame::LastStateFrame64;
use crate::normal::normal_or_zero;
use crate::math::{exp, log};

/// Plasma current, A: the sum of EFIT's `j_phi` over its 878 cells at 0.250 s (EFIT's own Ip, 755,420.8 A).
pub const PLASMA_CURRENT_A: f64 = 755340.5452248498;
/// Plasma mass, kg: deuterons and electrons at the line-average density 2.4683e19 per m3 over EFIT's volume, 7.2383 m3.
pub const PLASMA_MASS_KG: f64 = 5.975368111347944e-07;
/// Inductance of the wall mode, H: vᵀLv over the 126 passive elements.
pub const WALL_INDUCTANCE_H: f64 = 3.68063695571417e-07;
/// Resistance of the wall mode, ohm: vᵀRv.
pub const WALL_RESISTANCE_OHM: f64 = 2.5753872992470118e-05;
/// Mutual inductance of the wall mode and the P6 circuit, H.
pub const WALL_P6_MUTUAL_H: f64 = 3.80048996118169e-06;
/// Inductance of the P6 circuit, its 8 turns in anti-series, H.
pub const P6_INDUCTANCE_H: f64 = 0.00019595470089027952;
/// Resistance of the P6 circuit, ohm: the copper of its 8 turns at 20 degC; the amplifier and feeders add nothing.
pub const P6_RESISTANCE_OHM: f64 = 0.0011405623808968812;
/// Displacement at which EFIT's boundary, shifted down, meets the limiter (the corner at R 1.04 m, Z -1.033 m), m.
pub const LOWER_CONTACT_M: f64 = -0.3910649269819259;
/// Displacement at which EFIT's boundary, shifted up, meets the limiter (the corner at R 1.04 m, Z 1.033 m), m.
pub const UPPER_CONTACT_M: f64 = 0.38888768851757044;
/// Alfven speed at the magnetic axis, m/s: EFIT's 0.5731 T over the root of mu0 times the mass density.
pub const ALFVEN_SPEED_M_S: f64 = 1779348.1301704806;
/// Chebyshev series of the force of the held field on the shifted plasma, N, less its value at z = 0: positive is upward.
pub const FORCE_SERIES_N: [f64; 14] = [
    -51.83420182896187,
    19279.31950545196,
    -16.84229628316902,
    2717.036148678542,
    4.266482819083641,
    -38.602373830943236,
    -0.21643490088997058,
    1.0051378525610641,
    0.0029983454131904588,
    0.007439274551490927,
    -0.00013462497636111342,
    -0.0009817587636466348,
    -1.2671491910461548e-05,
    -9.83071339257976e-05,
];
/// Chebyshev series of the plasma's energy in the held field, J: minus the integral of [`FORCE_SERIES_N`] from z = 0.
pub const ENERGY_SERIES_J: [f64; 15] = [
    -1479.1006092070531,
    16.930062385145014,
    -1614.7245278449009,
    1.3719872892335896,
    -134.32921706265253,
    -0.17482317003812284,
    1.2871659304573349,
    0.006112411942203415,
    -0.024317425481209215,
    -6.787690137814512e-05,
    -0.00016420017398374837,
    2.1617713446946157e-06,
    1.435521685975124e-05,
    1.9006083188163834e-07,
    1.3691947540658053e-06,
];
/// Chebyshev series of the wall mode's coupling, the height derivative of its mutual inductance with the shifted plasma per ampere of Ip, H/m.
pub const WALL_COUPLING_SERIES_H_PER_M: [f64; 80] = [
    1.6251223683276375e-07,
    1.808973603792173e-10,
    -3.4507953062224693e-09,
    -1.570702586936676e-12,
    4.047246003651773e-11,
    -1.0529121147615502e-13,
    -1.8504745309894998e-12,
    1.1315728895581602e-15,
    2.3335182893005452e-14,
    2.0116009741420365e-14,
    3.834277425457347e-14,
    9.989096897068164e-15,
    3.828371181550227e-15,
    5.376703378754189e-15,
    -1.4029551850746985e-14,
    -2.902090693774788e-16,
    -1.0352859762911499e-14,
    -1.7391607627095788e-15,
    -5.800614921219405e-15,
    -1.8221218922377075e-15,
    5.14229253946569e-16,
    -6.433347301329866e-16,
    1.6581569138583984e-15,
    2.495957865352728e-16,
    1.0128484191849607e-15,
    4.579428592531307e-16,
    -1.5600113181839568e-15,
    1.727639437229327e-16,
    -1.2354841732724027e-16,
    -2.3637725033932698e-17,
    2.4383098357766793e-15,
    2.365989522169772e-16,
    -9.93319450446465e-16,
    -9.785428933230497e-17,
    -2.779234720261505e-15,
    -4.3023309287247564e-16,
    4.562615699677961e-15,
    4.677135191446344e-16,
    -4.150259306998569e-15,
    -3.76250783616795e-16,
    2.6857332392495517e-15,
    2.5162965313786723e-16,
    -1.2811082212774488e-15,
    -8.825832569238503e-17,
    5.374643675325072e-16,
    4.3139409494744505e-17,
    -1.845998594102941e-16,
    -1.7821511924540096e-17,
    3.951379925203413e-17,
    -3.1281714079210903e-18,
    -1.813272970978232e-17,
    -4.580388041328852e-18,
    2.9334329916583997e-18,
    3.7122377169651656e-19,
    3.581610078753826e-19,
    1.4483043359642273e-18,
    9.0031362213739e-19,
    1.2069760041371328e-18,
    3.698103552594036e-18,
    9.734331178216382e-19,
    1.716313521448042e-18,
    5.383392258777305e-19,
    -4.1745113253788934e-18,
    -1.0593926824350886e-18,
    -4.857446107562312e-19,
    -7.559467848264929e-19,
    3.3719062302705735e-18,
    7.716878968674635e-19,
    -3.74242165207111e-18,
    -5.467613811371711e-19,
    2.5996535992861347e-19,
    -2.201947506775478e-19,
    4.757002543025864e-18,
    1.3600561154892397e-18,
    -7.074241007690632e-18,
    -1.5527893660867022e-18,
    7.018787580168739e-18,
    1.3835481620796808e-18,
    -5.507302413329106e-18,
    -1.0534936526434811e-18,
];
/// Chebyshev series of the P6 circuit's coupling, H/m.
pub const P6_COUPLING_SERIES_H_PER_M: [f64; 15] = [
    3.921919206270224e-06,
    -1.011450313659559e-09,
    -1.9553545467906307e-07,
    -2.750926737310909e-10,
    -1.2447752883175043e-08,
    -2.3567518330315337e-13,
    2.0402944957038167e-10,
    2.728822925739802e-13,
    5.5668020486987694e-12,
    3.7670503726210225e-14,
    -5.911407907930018e-14,
    7.330692518049686e-16,
    8.227763537678077e-15,
    -7.626134312716423e-16,
    -1.3481948868425673e-15,
];

/// Each substep spans at most this fraction of the inverse of the bound on the
/// step's fastest pole. Chosen from the ledger: at 0.05 the largest residual
/// over the hard starts is 8.6e-8, at 0.02 it is 8.6e-10, as the fifth power.
pub const SUBSTEP_POLE_FRACTION: f64 = 0.02;
/// Most substeps one step may take, 2^20.
pub const MAX_SUBSTEPS: u32 = 1_048_576;
/// `|defect| / max(scale, 0.01 J)` must stay at or under this: 23 times the
/// largest value found over the hard starts at the step ceiling and a tenth of
/// it, 8.64e-10. A step of the explicit Euler method in place of the
/// Runge-Kutta substeps gives 1 or more.
pub const RESIDUAL_CONSERVATION_LIMIT: f64 = 2.0e-8;
/// Energy below which the ledger is held absolutely rather than relatively, J:
/// the energy of the plasma 0.85 mm off the equilibrium in the stiffness of the
/// held field. The largest rounding of the energy series' sum over the span,
/// against 40-digit arithmetic at 20,000 heights, is 9.5e-13 J, under 1e-10 of it.
pub const ENERGY_SCALE_FLOOR_J: f64 = 0.01;
/// Largest step the card stands behind, s.
pub const GATE_DT_S: f64 = 1.0e-3;
/// Smallest step the card stands behind, s: the ceiling divided by 2^20,
/// 9.54e-10 s, a step that still moves a clock of 100 s.
pub const FLOOR_DT_S: f64 = GATE_DT_S / 1_048_576.0;
/// The growing pole of the card's linear system at z = 0, per second: derived
/// from the constants above (`analysis/card_model.py`), not used by the step.
pub const GROWTH_RATE_PER_S: f64 = 6.546503766672119;

/// The landed displacement is past the card's ideal-wall limit: there the held
/// field's stiffness is above what the wall mode and P6 as perfect conductors
/// would hold back, so the plasma grows on its inertial time and not the
/// wall's. From +0.238 m up and from −0.239 m down.
pub const FLAG_PAST_IDEAL_WALL_LIMIT: u64 = 1 << 0;

/// Why a step did not advance the plasma or the frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StepReject {
    /// `dt` or the amplifier voltage was NaN or infinite.
    NonFiniteInput,
    /// The incoming state, or the trial step, was non-finite.
    NonFiniteState,
    /// `dt` was zero or negative.
    NonPositiveDt,
    /// A recorded constant is not this card's.
    InvalidCard,
    /// The displacement is at or beyond a contact height, −0.391 m or +0.389 m: there EFIT's boundary, shifted, touches or crosses the limiter, and the plasma is not inside the vessel.
    StartOutsideVessel,
    /// The plasma's vertical speed is at or above its Alfvén speed, 1.78e6 m/s, at the start or at any stage of a substep, where the wall is tested too: the plasma cannot move as one body faster than the field carries its force across it.
    SpeedAtAlfvenSpeed,
    /// The step carries EFIT's boundary to the limiter, at −0.391 m or +0.389 m: the plasma reaches the wall and the vertical displacement event ends.
    PlasmaAtWall,
    /// The step would take more than 1,048,576 substeps, each 0.02 of the inverse of the bound on its fastest pole.
    SubstepsAboveBudget,
    /// The trial step created energy beyond what the amplifier supplied: a defect of the arithmetic or the integrator.
    SpontaneousGeneration,
    /// The trial step destroyed energy beyond the resistive losses: a defect of the arithmetic or the integrator.
    FirstLawBreach,
}

/// What is applied, held through the step.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct VdeLoad {
    /// The fast amplifier's voltage across the P6 pair, V. Zero in shot 15045 after 0.250 s.
    pub p6_voltage_v: f64,
}

/// The locked card. Every field is a recorded copy and none is a caller input.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TokamakVdeMast {
    /// Recorded copy of [`PLASMA_CURRENT_A`].
    pub plasma_current_a: f64,
    /// Recorded copy of [`PLASMA_MASS_KG`].
    pub plasma_mass_kg: f64,
    /// Recorded copy of [`WALL_INDUCTANCE_H`].
    pub wall_inductance_h: f64,
    /// Recorded copy of [`WALL_RESISTANCE_OHM`].
    pub wall_resistance_ohm: f64,
    /// Recorded copy of [`WALL_P6_MUTUAL_H`].
    pub wall_p6_mutual_h: f64,
    /// Recorded copy of [`P6_INDUCTANCE_H`].
    pub p6_inductance_h: f64,
    /// Recorded copy of [`P6_RESISTANCE_OHM`].
    pub p6_resistance_ohm: f64,
    /// Recorded copy of [`LOWER_CONTACT_M`].
    pub lower_contact_m: f64,
    /// Recorded copy of [`UPPER_CONTACT_M`].
    pub upper_contact_m: f64,
    /// Recorded copy of [`ALFVEN_SPEED_M_S`].
    pub alfven_speed_m_s: f64,
    /// Recorded copy of [`FORCE_SERIES_N`].
    pub force_series_n: [f64; 14],
    /// Recorded copy of [`ENERGY_SERIES_J`].
    pub energy_series_j: [f64; 15],
    /// Recorded copy of [`WALL_COUPLING_SERIES_H_PER_M`].
    pub wall_coupling_series_h_per_m: [f64; 80],
    /// Recorded copy of [`P6_COUPLING_SERIES_H_PER_M`].
    pub p6_coupling_series_h_per_m: [f64; 15],
    /// Recorded copy of [`SUBSTEP_POLE_FRACTION`].
    pub substep_pole_fraction: f64,
    /// Recorded copy of [`MAX_SUBSTEPS`].
    pub max_substeps: u32,
    /// Recorded copy of [`RESIDUAL_CONSERVATION_LIMIT`].
    pub residual_conservation_limit: f64,
    /// Recorded copy of [`ENERGY_SCALE_FLOOR_J`].
    pub energy_scale_floor_j: f64,
    /// Recorded copy of [`GATE_DT_S`].
    pub gate_dt_s: f64,
    /// Recorded copy of [`FLOOR_DT_S`].
    pub floor_dt_s: f64,
}

fn same_bits(a: &[f64], b: &[f64]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut index = 0;
    while index < a.len() {
        if a[index].to_bits() != b[index].to_bits() {
            return false;
        }
        index += 1;
    }
    true
}

impl TokamakVdeMast {
    /// Card: MAST shot 15045 at 0.250 s. The constructor takes no arguments.
    #[must_use]
    pub fn mast_15045() -> Self {
        let card = Self {
            plasma_current_a: PLASMA_CURRENT_A,
            plasma_mass_kg: PLASMA_MASS_KG,
            wall_inductance_h: WALL_INDUCTANCE_H,
            wall_resistance_ohm: WALL_RESISTANCE_OHM,
            wall_p6_mutual_h: WALL_P6_MUTUAL_H,
            p6_inductance_h: P6_INDUCTANCE_H,
            p6_resistance_ohm: P6_RESISTANCE_OHM,
            lower_contact_m: LOWER_CONTACT_M,
            upper_contact_m: UPPER_CONTACT_M,
            alfven_speed_m_s: ALFVEN_SPEED_M_S,
            force_series_n: FORCE_SERIES_N,
            energy_series_j: ENERGY_SERIES_J,
            wall_coupling_series_h_per_m: WALL_COUPLING_SERIES_H_PER_M,
            p6_coupling_series_h_per_m: P6_COUPLING_SERIES_H_PER_M,
            substep_pole_fraction: SUBSTEP_POLE_FRACTION,
            max_substeps: MAX_SUBSTEPS,
            residual_conservation_limit: RESIDUAL_CONSERVATION_LIMIT,
            energy_scale_floor_j: ENERGY_SCALE_FLOOR_J,
            gate_dt_s: GATE_DT_S,
            floor_dt_s: FLOOR_DT_S,
        };
        if card.is_valid() {
            card
        } else {
            ::core::panic!("MAST VDE card failed is_valid");
        }
    }

    #[must_use]
    pub fn is_valid(&self) -> bool {
        self.plasma_current_a.to_bits() == PLASMA_CURRENT_A.to_bits()
            && self.plasma_mass_kg.to_bits() == PLASMA_MASS_KG.to_bits()
            && self.wall_inductance_h.to_bits() == WALL_INDUCTANCE_H.to_bits()
            && self.wall_resistance_ohm.to_bits() == WALL_RESISTANCE_OHM.to_bits()
            && self.wall_p6_mutual_h.to_bits() == WALL_P6_MUTUAL_H.to_bits()
            && self.p6_inductance_h.to_bits() == P6_INDUCTANCE_H.to_bits()
            && self.p6_resistance_ohm.to_bits() == P6_RESISTANCE_OHM.to_bits()
            && self.lower_contact_m.to_bits() == LOWER_CONTACT_M.to_bits()
            && self.upper_contact_m.to_bits() == UPPER_CONTACT_M.to_bits()
            && self.alfven_speed_m_s.to_bits() == ALFVEN_SPEED_M_S.to_bits()
            && same_bits(&self.force_series_n, &FORCE_SERIES_N)
            && same_bits(&self.energy_series_j, &ENERGY_SERIES_J)
            && same_bits(&self.wall_coupling_series_h_per_m, &WALL_COUPLING_SERIES_H_PER_M)
            && same_bits(&self.p6_coupling_series_h_per_m, &P6_COUPLING_SERIES_H_PER_M)
            && self.substep_pole_fraction.to_bits() == SUBSTEP_POLE_FRACTION.to_bits()
            && self.max_substeps == MAX_SUBSTEPS
            && self.residual_conservation_limit.to_bits() == RESIDUAL_CONSERVATION_LIMIT.to_bits()
            && self.energy_scale_floor_j.to_bits() == ENERGY_SCALE_FLOOR_J.to_bits()
            && self.gate_dt_s.to_bits() == GATE_DT_S.to_bits()
            && self.floor_dt_s.to_bits() == FLOOR_DT_S.to_bits()
            && self.lower_contact_m < 0.0
            && self.upper_contact_m > 0.0
            && self.plasma_mass_kg > 0.0
            && self.wall_inductance_h > 0.0
            && self.p6_inductance_h > 0.0
            && self.wall_inductance_h * self.p6_inductance_h > self.wall_p6_mutual_h * self.wall_p6_mutual_h
    }

    /// Largest step the card stands behind: [`GATE_DT_S`]. Not applied by [`step`].
    #[must_use]
    pub fn gate_dt_s(&self) -> Option<f64> {
        if self.gate_dt_s.is_finite() && self.gate_dt_s > 0.0 {
            Some(self.gate_dt_s)
        } else {
            None
        }
    }

    /// Smallest step the card stands behind: [`FLOOR_DT_S`]. Not applied by [`step`].
    #[must_use]
    pub fn floor_dt_s(&self) -> Option<f64> {
        if self.floor_dt_s.is_finite() && self.floor_dt_s > 0.0 {
            Some(self.floor_dt_s)
        } else {
            None
        }
    }

    /// `true` when a displacement lies strictly between the two contact heights.
    #[must_use]
    pub fn inside_vessel(&self, displacement_m: f64) -> bool {
        let x = unit(self, displacement_m);
        x > -1.0 && x < 1.0
    }

    /// The force of the held field on the plasma at a displacement, N, positive
    /// upward: `G(z)`. `None` outside the vessel.
    #[must_use]
    pub fn field_force_n(&self, displacement_m: f64) -> Option<f64> {
        if self.inside_vessel(displacement_m) {
            Some(clenshaw(&self.force_series_n, unit(self, displacement_m)))
        } else {
            None
        }
    }

    /// The plasma's energy in the held field at a displacement, J: `U(z)`, zero
    /// at the equilibrium. `None` outside the vessel.
    #[must_use]
    pub fn field_energy_j(&self, displacement_m: f64) -> Option<f64> {
        if self.inside_vessel(displacement_m) {
            Some(clenshaw(&self.energy_series_j, unit(self, displacement_m)))
        } else {
            None
        }
    }

    /// The card's stability margin at a displacement: the stiffness the wall mode
    /// and P6 as perfect conductors would push back with, `Ip² Dᵀ L⁻¹ D`, over
    /// the held field's stiffness `G'(z)`, less one. 1.267 at the equilibrium,
    /// zero at +0.238 m and −0.239 m. `None` outside the vessel, or where the
    /// held field does not push away from the equilibrium.
    #[must_use]
    pub fn ideal_wall_margin(&self, displacement_m: f64) -> Option<f64> {
        if !self.inside_vessel(displacement_m) {
            return None;
        }
        let x = unit(self, displacement_m);
        let stiffness = clenshaw_slope(&self.force_series_n, x) * 2.0 / (self.upper_contact_m - self.lower_contact_m);
        if !(stiffness > 0.0) {
            return None;
        }
        let dw = clenshaw(&self.wall_coupling_series_h_per_m, x);
        let dc = clenshaw(&self.p6_coupling_series_h_per_m, x);
        let det = self.wall_inductance_h * self.p6_inductance_h - self.wall_p6_mutual_h * self.wall_p6_mutual_h;
        let held = (self.p6_inductance_h * dw * dw - 2.0 * self.wall_p6_mutual_h * dw * dc + self.wall_inductance_h * dc * dc) / det;
        Some(self.plasma_current_a * self.plasma_current_a * held / stiffness - 1.0)
    }

    /// The wall mode's and P6's couplings at a displacement, H/m: `D_w(z)`, `D_c(z)`.
    #[must_use]
    pub fn couplings_h_per_m(&self, displacement_m: f64) -> Option<(f64, f64)> {
        if self.inside_vessel(displacement_m) {
            let x = unit(self, displacement_m);
            Some((
                clenshaw(&self.wall_coupling_series_h_per_m, x),
                clenshaw(&self.p6_coupling_series_h_per_m, x),
            ))
        } else {
            None
        }
    }
}

/// The plasma and the two circuits. The derived and ledger fields are part of
/// the state; write them only through [`step`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct VdeState {
    /// Time since the start of the run, s.
    pub t_s: f64,
    /// Height of the plasma above its 0.250 s equilibrium, `z`, m.
    pub displacement_m: f64,
    /// Vertical velocity, `v`, m/s.
    pub velocity_m_s: f64,
    /// Current of the wall mode, `I_w`, A.
    pub wall_current_a: f64,
    /// Current in each of P6's 8 turns, `I_c`, A: positive in the upper coil in the sense of Ip, negative in the lower. Each coil carries 4 times it in ampere-turns.
    pub p6_current_a: f64,
    /// The held field's force on the plasma, `G(z)`, N, upward positive.
    pub field_force_n: f64,
    /// The wall mode's force on the plasma, `Ip D_w I_w`, N; the passive structure feels the opposite.
    pub wall_force_n: f64,
    /// P6's force on the plasma, `Ip D_c I_c`, N; P6 feels the opposite.
    pub p6_force_n: f64,
    /// Resistive power in the wall mode, W.
    pub wall_power_w: f64,
    /// Resistive power in P6, W.
    pub p6_power_w: f64,
    /// Power the amplifier delivers to P6, `V I_c`, W. Zero at the start.
    pub supply_power_w: f64,
    /// `½ m v²`, J.
    pub kinetic_energy_j: f64,
    /// `U(z)`, J.
    pub potential_energy_j: f64,
    /// `½ L_w I_w² + M I_w I_c + ½ L_c I_c²`, J.
    pub magnetic_energy_j: f64,
    /// `E`, the sum of the three, J.
    pub energy_j: f64,
    /// `E` at the state the ledger started from, J.
    pub energy_anchor_j: f64,
    /// The sum of each term's size at the anchor, J.
    pub stored_scale_anchor_j: f64,
    /// `Σ Q_supply` since the anchor, J.
    pub energy_supplied_j: f64,
    /// `Σ Q_loss` since the anchor, J.
    pub energy_dissipated_j: f64,
    /// Signed defect of the ledger, J. Positive is created energy.
    pub energy_defect_j: f64,
    /// `|energy_defect_j| / max(scale, 0.01 J)`.
    pub residual_conservation_error: f64,
    /// Distance to the nearer contact height, m.
    pub gap_to_wall_m: f64,
    /// Substeps the last step took; zero at the start.
    pub substeps: u32,
    /// The displacement is past the card's ideal-wall limit ([`FLAG_PAST_IDEAL_WALL_LIMIT`]).
    pub past_ideal_wall_limit: bool,
}

impl VdeState {
    /// The plasma at a displacement and velocity with the two currents, at time
    /// 0, ledger anchored. `None` unless the card is valid, the four numbers are
    /// finite, the displacement is inside the vessel and the speed under the
    /// Alfvén speed; [`start_in_card`] names the reason.
    #[must_use]
    pub fn try_start(
        card: &TokamakVdeMast,
        displacement_m: f64,
        velocity_m_s: f64,
        wall_current_a: f64,
        p6_current_a: f64,
    ) -> Option<Self> {
        if !card.is_valid()
            || !displacement_m.is_finite()
            || !velocity_m_s.is_finite()
            || !wall_current_a.is_finite()
            || !p6_current_a.is_finite()
            || start_in_card(card, displacement_m, velocity_m_s).is_err()
        {
            return None;
        }
        let y = [displacement_m, velocity_m_s, wall_current_a, p6_current_a];
        let (kinetic, potential, magnetic, size) = stored(card, &y);
        let mut state = Self {
            t_s: 0.0,
            displacement_m,
            velocity_m_s,
            wall_current_a,
            p6_current_a,
            field_force_n: 0.0,
            wall_force_n: 0.0,
            p6_force_n: 0.0,
            wall_power_w: 0.0,
            p6_power_w: 0.0,
            supply_power_w: 0.0,
            kinetic_energy_j: kinetic,
            potential_energy_j: potential,
            magnetic_energy_j: magnetic,
            energy_j: kinetic + potential + magnetic,
            energy_anchor_j: kinetic + potential + magnetic,
            stored_scale_anchor_j: size,
            energy_supplied_j: 0.0,
            energy_dissipated_j: 0.0,
            energy_defect_j: 0.0,
            residual_conservation_error: 0.0,
            gap_to_wall_m: 0.0,
            substeps: 0,
            past_ideal_wall_limit: false,
        };
        describe(card, &mut state, 0.0);
        if state_is_finite(&state) { Some(state) } else { None }
    }
}

/// A start held to the card: the displacement strictly inside the vessel, then
/// the speed under the Alfvén speed. The step tests this first; a caller may
/// test a start with it.
pub fn start_in_card(card: &TokamakVdeMast, displacement_m: f64, velocity_m_s: f64) -> Result<(), StepReject> {
    if !card.inside_vessel(displacement_m) {
        return Err(StepReject::StartOutsideVessel);
    }
    if !(velocity_m_s.abs() < card.alfven_speed_m_s) {
        return Err(StepReject::SpeedAtAlfvenSpeed);
    }
    Ok(())
}

/// Advance one step of substeps and write the frame.
///
/// A rejected step leaves `state` and `frame` as they were.
///
/// A value of the landed state below the normal range is written as zero
/// with its sign ([`crate::normal`]).
#[must_use]
pub fn step(
    card: &TokamakVdeMast,
    state: &mut VdeState,
    frame: &mut LastStateFrame64,
    load: VdeLoad,
    dt_s: f64,
) -> Result<(), StepReject> {
    #[cfg(any(test, debug_assertions, feature = "check_exceedances"))]
    let before = *state;
    let stepped = step_landed(card, state, frame, load, dt_s);
    #[cfg(any(test, debug_assertions, feature = "check_exceedances"))]
    if stepped.is_ok() {
        crate::exceedance::debug_check("tokamak_vde_mast", exceedances(card, &before, state, load, dt_s), frame.flags);
    }
    stepped
}

/// The step itself; [`step`] is it with, in a build that checks
/// (`crate::exceedance::CHECKED`), the check that [`exceedances`] stands for
/// the flags it wrote.
fn step_landed(
    card: &TokamakVdeMast,
    state: &mut VdeState,
    frame: &mut LastStateFrame64,
    load: VdeLoad,
    dt_s: f64,
) -> Result<(), StepReject> {
    step_as_computed(card, state, frame, load, dt_s)?;
    normal_or_zero_state(state);
    Ok(())
}

/// The step as its equations compute it, before a value of the landed
/// state below the normal range is written as zero.
fn step_as_computed(
    card: &TokamakVdeMast,
    state: &mut VdeState,
    frame: &mut LastStateFrame64,
    load: VdeLoad,
    dt_s: f64,
) -> Result<(), StepReject> {
    step_with(card, state, frame, load, dt_s, rk4)
}

/// Each value of a landed state below the normal range written as zero
/// with its sign ([`crate::normal`]): the state carried to the next step
/// and published holds no subnormal number.
fn normal_or_zero_state(state: &mut VdeState) {
    state.t_s = normal_or_zero(state.t_s);
    state.displacement_m = normal_or_zero(state.displacement_m);
    state.velocity_m_s = normal_or_zero(state.velocity_m_s);
    state.wall_current_a = normal_or_zero(state.wall_current_a);
    state.p6_current_a = normal_or_zero(state.p6_current_a);
    state.field_force_n = normal_or_zero(state.field_force_n);
    state.wall_force_n = normal_or_zero(state.wall_force_n);
    state.p6_force_n = normal_or_zero(state.p6_force_n);
    state.wall_power_w = normal_or_zero(state.wall_power_w);
    state.p6_power_w = normal_or_zero(state.p6_power_w);
    state.supply_power_w = normal_or_zero(state.supply_power_w);
    state.kinetic_energy_j = normal_or_zero(state.kinetic_energy_j);
    state.potential_energy_j = normal_or_zero(state.potential_energy_j);
    state.magnetic_energy_j = normal_or_zero(state.magnetic_energy_j);
    state.energy_j = normal_or_zero(state.energy_j);
    state.energy_anchor_j = normal_or_zero(state.energy_anchor_j);
    state.stored_scale_anchor_j = normal_or_zero(state.stored_scale_anchor_j);
    state.energy_supplied_j = normal_or_zero(state.energy_supplied_j);
    state.energy_dissipated_j = normal_or_zero(state.energy_dissipated_j);
    state.energy_defect_j = normal_or_zero(state.energy_defect_j);
    state.residual_conservation_error = normal_or_zero(state.residual_conservation_error);
    state.gap_to_wall_m = normal_or_zero(state.gap_to_wall_m);
}

/// One substep's integrator: from `y` with its rates `k1` over `h`, the end
/// state and its rates, or the reject a stage met.
type Integrator = fn(&TokamakVdeMast, &[f64; 4], &[f64; 4], f64, f64) -> Result<([f64; 4], [f64; 4]), StepReject>;

fn step_with(
    card: &TokamakVdeMast,
    state: &mut VdeState,
    frame: &mut LastStateFrame64,
    load: VdeLoad,
    dt_s: f64,
    integrate: Integrator,
) -> Result<(), StepReject> {
    validate(card, state, load, dt_s)?;
    let volts = load.p6_voltage_v;
    let mut y = [state.displacement_m, state.velocity_m_s, state.wall_current_a, state.p6_current_a];
    let mut here = rates(card, &y, volts)?;
    let mut loss = 0.0;
    let mut supply = 0.0;
    let mut remaining = dt_s;
    let mut count: u32 = 0;
    loop {
        let bound = pole_bound(card, &y);
        if !(bound.is_finite() && bound > 0.0) {
            return Err(StepReject::NonFiniteState);
        }
        let largest = card.substep_pole_fraction / bound;
        let left = (remaining / largest).ceil();
        let h = if left <= 1.0 { remaining } else { remaining / left };
        if count >= card.max_substeps {
            return Err(StepReject::SubstepsAboveBudget);
        }
        let (next, there) = integrate(card, &y, &here, volts, h)?;
        if !next[0].is_finite() || !next[1].is_finite() || !next[2].is_finite() || !next[3].is_finite() {
            return Err(StepReject::NonFiniteState);
        }
        let (l0, dl0, s0, ds0) = powers(card, &y, &here, volts);
        let (l1, dl1, s1, ds1) = powers(card, &next, &there, volts);
        loss += 0.5 * h * (l0 + l1) + h * h / 12.0 * (dl0 - dl1);
        supply += 0.5 * h * (s0 + s1) + h * h / 12.0 * (ds0 - ds1);
        // the ledger is held at every substep, so a defect is named before the
        // wall or the speed limit can stop the step
        let (defect, residual) = ledger(card, state, &next, loss, supply);
        if !(residual <= card.residual_conservation_limit) {
            return Err(if !residual.is_finite() {
                StepReject::NonFiniteState
            } else if defect > 0.0 {
                StepReject::SpontaneousGeneration
            } else {
                StepReject::FirstLawBreach
            });
        }
        y = next;
        here = there;
        count += 1;
        if h == remaining {
            break;
        }
        remaining -= h;
    }
    commit(card, state, frame, volts, dt_s, &y, loss, supply, count)
}

/// The ledger's defect and residual at `y`, with the step's losses and supply so far.
fn ledger(card: &TokamakVdeMast, state: &VdeState, y: &[f64; 4], loss: f64, supply: f64) -> (f64, f64) {
    let (kinetic, potential, magnetic, size) = stored(card, y);
    let supplied = state.energy_supplied_j + supply;
    let dissipated = state.energy_dissipated_j + loss;
    let defect = (kinetic + potential + magnetic) - state.energy_anchor_j - (supplied - dissipated);
    let larger = if size > state.stored_scale_anchor_j { size } else { state.stored_scale_anchor_j };
    let scale = larger + dissipated + supplied.abs();
    let floor = if scale > card.energy_scale_floor_j { scale } else { card.energy_scale_floor_j };
    (defect, defect.abs() / floor)
}

fn validate(card: &TokamakVdeMast, state: &VdeState, load: VdeLoad, dt_s: f64) -> Result<(), StepReject> {
    if !dt_s.is_finite() || !load.p6_voltage_v.is_finite() {
        return Err(StepReject::NonFiniteInput);
    }
    if !state_is_finite(state) {
        return Err(StepReject::NonFiniteState);
    }
    if dt_s <= 0.0 {
        return Err(StepReject::NonPositiveDt);
    }
    if !card.is_valid() {
        return Err(StepReject::InvalidCard);
    }
    start_in_card(card, state.displacement_m, state.velocity_m_s)
}

#[allow(clippy::too_many_arguments)]
fn commit(
    card: &TokamakVdeMast,
    state: &mut VdeState,
    frame: &mut LastStateFrame64,
    volts: f64,
    dt_s: f64,
    y: &[f64; 4],
    loss: f64,
    supply: f64,
    count: u32,
) -> Result<(), StepReject> {
    if !card.inside_vessel(y[0]) {
        return Err(StepReject::PlasmaAtWall);
    }
    if !(y[1].abs() < card.alfven_speed_m_s) {
        return Err(StepReject::SpeedAtAlfvenSpeed);
    }
    let saved = *state;
    let (kinetic, potential, magnetic, size) = stored(card, y);
    state.t_s += dt_s;
    state.displacement_m = y[0];
    state.velocity_m_s = y[1];
    state.wall_current_a = y[2];
    state.p6_current_a = y[3];
    state.kinetic_energy_j = kinetic;
    state.potential_energy_j = potential;
    state.magnetic_energy_j = magnetic;
    state.energy_j = kinetic + potential + magnetic;
    state.energy_supplied_j += supply;
    state.energy_dissipated_j += loss;
    state.energy_defect_j =
        (kinetic + potential + magnetic) - state.energy_anchor_j - (state.energy_supplied_j - state.energy_dissipated_j);
    let larger = if size > state.stored_scale_anchor_j { size } else { state.stored_scale_anchor_j };
    let scale = larger + state.energy_dissipated_j + state.energy_supplied_j.abs();
    let floor = if scale > card.energy_scale_floor_j { scale } else { card.energy_scale_floor_j };
    state.residual_conservation_error = state.energy_defect_j.abs() / floor;
    state.substeps = count;
    describe(card, state, volts);
    if !state_is_finite(state) {
        *state = saved;
        return Err(StepReject::NonFiniteState);
    }
    if state.residual_conservation_error > card.residual_conservation_limit {
        let created = state.energy_defect_j > 0.0;
        *state = saved;
        return Err(if created {
            StepReject::SpontaneousGeneration
        } else {
            StepReject::FirstLawBreach
        });
    }
    publish(state, frame);
    Ok(())
}

/// The forces, powers and gap of the state's own numbers.
fn describe(card: &TokamakVdeMast, state: &mut VdeState, volts: f64) {
    let x = unit(card, state.displacement_m);
    let ip = card.plasma_current_a;
    state.field_force_n = clenshaw(&card.force_series_n, x);
    state.wall_force_n = ip * clenshaw(&card.wall_coupling_series_h_per_m, x) * state.wall_current_a;
    state.p6_force_n = ip * clenshaw(&card.p6_coupling_series_h_per_m, x) * state.p6_current_a;
    state.wall_power_w = card.wall_resistance_ohm * state.wall_current_a * state.wall_current_a;
    state.p6_power_w = card.p6_resistance_ohm * state.p6_current_a * state.p6_current_a;
    state.supply_power_w = volts * state.p6_current_a;
    let above = card.upper_contact_m - state.displacement_m;
    let below = state.displacement_m - card.lower_contact_m;
    state.gap_to_wall_m = if above < below { above } else { below };
    state.past_ideal_wall_limit = card.ideal_wall_margin(state.displacement_m).is_some_and(|margin| margin < 0.0);
}

/// The series' variable of a displacement.
fn unit(card: &TokamakVdeMast, displacement_m: f64) -> f64 {
    (2.0 * displacement_m - (card.upper_contact_m + card.lower_contact_m))
        / (card.upper_contact_m - card.lower_contact_m)
}

/// `Σ c[k] T_k(x)` by Clenshaw's recurrence.
fn clenshaw(c: &[f64], x: f64) -> f64 {
    let mut b1 = 0.0;
    let mut b2 = 0.0;
    let mut k = c.len() - 1;
    while k > 0 {
        let b0 = 2.0 * x * b1 - b2 + c[k];
        b2 = b1;
        b1 = b0;
        k -= 1;
    }
    x * b1 - b2 + c[0]
}

/// `d/dx Σ c[k] T_k(x) = Σ k c[k] U_{k−1}(x)`, by Clenshaw's recurrence for `U`.
fn clenshaw_slope(c: &[f64], x: f64) -> f64 {
    let mut b1 = 0.0;
    let mut b2 = 0.0;
    let mut k = c.len() - 1;
    while k > 0 {
        let b0 = 2.0 * x * b1 - b2 + (k as f64) * c[k];
        b2 = b1;
        b1 = b0;
        k -= 1;
    }
    b1
}

/// The rates at `y` under the voltage, or [`StepReject::PlasmaAtWall`] when `y`
/// is not strictly inside the vessel, or [`StepReject::SpeedAtAlfvenSpeed`]
/// when its speed is at or above the Alfvén speed. Every stage of every
/// substep passes through here, so the wall and the speed are tested at the
/// same points and the step asked for does not decide which is met first.
fn rates(card: &TokamakVdeMast, y: &[f64; 4], volts: f64) -> Result<[f64; 4], StepReject> {
    let x = unit(card, y[0]);
    if !(x > -1.0 && x < 1.0) {
        return Err(if x.is_finite() { StepReject::PlasmaAtWall } else { StepReject::NonFiniteState });
    }
    if !(y[1].abs() < card.alfven_speed_m_s) {
        return Err(if y[1].is_finite() { StepReject::SpeedAtAlfvenSpeed } else { StepReject::NonFiniteState });
    }
    let ip = card.plasma_current_a;
    let force = clenshaw(&card.force_series_n, x);
    let dw = clenshaw(&card.wall_coupling_series_h_per_m, x);
    let dc = clenshaw(&card.p6_coupling_series_h_per_m, x);
    let accel = (force + ip * (dw * y[2] + dc * y[3])) / card.plasma_mass_kg;
    let ew = -card.wall_resistance_ohm * y[2] - ip * dw * y[1];
    let ec = volts - card.p6_resistance_ohm * y[3] - ip * dc * y[1];
    let det = card.wall_inductance_h * card.p6_inductance_h - card.wall_p6_mutual_h * card.wall_p6_mutual_h;
    let diw = (card.p6_inductance_h * ew - card.wall_p6_mutual_h * ec) / det;
    let dic = (card.wall_inductance_h * ec - card.wall_p6_mutual_h * ew) / det;
    Ok([y[1], accel, diw, dic])
}

fn advance(y: &[f64; 4], k: &[f64; 4], scale: f64) -> [f64; 4] {
    [y[0] + scale * k[0], y[1] + scale * k[1], y[2] + scale * k[2], y[3] + scale * k[3]]
}

/// One substep of the classical fourth-order Runge-Kutta method. Every stage is
/// held inside the vessel.
fn rk4(
    card: &TokamakVdeMast,
    y: &[f64; 4],
    k1: &[f64; 4],
    volts: f64,
    h: f64,
) -> Result<([f64; 4], [f64; 4]), StepReject> {
    let k2 = rates(card, &advance(y, k1, 0.5 * h), volts)?;
    let k3 = rates(card, &advance(y, &k2, 0.5 * h), volts)?;
    let k4 = rates(card, &advance(y, &k3, h), volts)?;
    let mut out = [0.0; 4];
    let mut index = 0;
    while index < 4 {
        out[index] = y[index] + h / 6.0 * (k1[index] + 2.0 * k2[index] + 2.0 * k3[index] + k4[index]);
        index += 1;
    }
    let there = rates(card, &out, volts)?;
    Ok((out, there))
}

fn multiply(a: &[[f64; 4]; 4], b: &[[f64; 4]; 4]) -> [[f64; 4]; 4] {
    let mut out = [[0.0; 4]; 4];
    let mut row = 0;
    while row < 4 {
        let mut col = 0;
        while col < 4 {
            let mut sum = 0.0;
            let mut k = 0;
            while k < 4 {
                sum += a[row][k] * b[k][col];
                k += 1;
            }
            out[row][col] = sum;
            col += 1;
        }
        row += 1;
    }
    out
}

/// `|x|^(1/n)` for n = 3 by the crate's own `exp` and `log`, zero at zero.
fn cube_root(x: f64) -> f64 {
    if x == 0.0 { 0.0 } else { exp(log(x) / 3.0) }
}

/// Fujiwara's bound on the roots of the characteristic polynomial of the
/// Jacobian of the rates at `y`, `λ⁴ + c3 λ³ + c2 λ² + c1 λ + c0`:
/// `2 max(|c3|, |c2|^(1/2), |c1|^(1/3), |c0 / 2|^(1/4))`, the coefficients by
/// Faddeev and LeVerrier. Every eigenvalue of the linearisation lies within it.
fn pole_bound(card: &TokamakVdeMast, y: &[f64; 4]) -> f64 {
    let x = unit(card, y[0]);
    let ip = card.plasma_current_a;
    let mass = card.plasma_mass_kg;
    let slope = 2.0 / (card.upper_contact_m - card.lower_contact_m);
    let dw = clenshaw(&card.wall_coupling_series_h_per_m, x);
    let dc = clenshaw(&card.p6_coupling_series_h_per_m, x);
    let gz = clenshaw_slope(&card.force_series_n, x) * slope;
    let dwz = clenshaw_slope(&card.wall_coupling_series_h_per_m, x) * slope;
    let dcz = clenshaw_slope(&card.p6_coupling_series_h_per_m, x) * slope;
    let mut j = [[0.0; 4]; 4];
    j[0][1] = 1.0;
    j[1][0] = (gz + ip * (dwz * y[2] + dcz * y[3])) / mass;
    j[1][2] = ip * dw / mass;
    j[1][3] = ip * dc / mass;
    let ew = [-ip * dwz * y[1], -ip * dw, -card.wall_resistance_ohm, 0.0];
    let ec = [-ip * dcz * y[1], -ip * dc, 0.0, -card.p6_resistance_ohm];
    let det = card.wall_inductance_h * card.p6_inductance_h - card.wall_p6_mutual_h * card.wall_p6_mutual_h;
    let mut col = 0;
    while col < 4 {
        j[2][col] = (card.p6_inductance_h * ew[col] - card.wall_p6_mutual_h * ec[col]) / det;
        j[3][col] = (card.wall_inductance_h * ec[col] - card.wall_p6_mutual_h * ew[col]) / det;
        col += 1;
    }
    let mut m = j;
    let mut coeffs = [0.0; 4];
    let mut order = 1;
    while order <= 4 {
        if order > 1 {
            let mut shifted = m;
            let mut d = 0;
            while d < 4 {
                shifted[d][d] += coeffs[order - 2];
                d += 1;
            }
            m = multiply(&j, &shifted);
        }
        coeffs[order - 1] = -(m[0][0] + m[1][1] + m[2][2] + m[3][3]) / order as f64;
        order += 1;
    }
    let a3 = coeffs[0].abs();
    let a2 = coeffs[1].abs().sqrt();
    let a1 = cube_root(coeffs[2].abs());
    let a0 = (coeffs[3].abs() / 2.0).sqrt().sqrt();
    let mut largest = a3;
    if a2 > largest {
        largest = a2;
    }
    if a1 > largest {
        largest = a1;
    }
    if a0 > largest {
        largest = a0;
    }
    2.0 * largest
}

/// (kinetic, potential, magnetic, the sum of each term's size), J.
fn stored(card: &TokamakVdeMast, y: &[f64; 4]) -> (f64, f64, f64, f64) {
    let kinetic = 0.5 * card.plasma_mass_kg * y[1] * y[1];
    let potential = clenshaw(&card.energy_series_j, unit(card, y[0]));
    let cross = card.wall_p6_mutual_h * y[2] * y[3];
    let wall = 0.5 * card.wall_inductance_h * y[2] * y[2];
    let coil = 0.5 * card.p6_inductance_h * y[3] * y[3];
    let magnetic = wall + cross + coil;
    (kinetic, potential, magnetic, kinetic + potential.abs() + wall + cross.abs() + coil)
}

/// (loss power, its rate, supply power, its rate) at `y` with its rates `r`.
fn powers(card: &TokamakVdeMast, y: &[f64; 4], r: &[f64; 4], volts: f64) -> (f64, f64, f64, f64) {
    let loss = card.wall_resistance_ohm * y[2] * y[2] + card.p6_resistance_ohm * y[3] * y[3];
    let loss_rate = 2.0 * card.wall_resistance_ohm * y[2] * r[2] + 2.0 * card.p6_resistance_ohm * y[3] * r[3];
    (loss, loss_rate, volts * y[3], volts * r[3])
}

fn publish(state: &VdeState, frame: &mut LastStateFrame64) {
    frame.t = state.t_s;
    frame.pos[0] = state.displacement_m as f32;
    frame.pos[1] = state.wall_current_a as f32;
    frame.pos[2] = state.p6_current_a as f32;
    frame.vel[0] = state.velocity_m_s as f32;
    frame.vel[1] = state.wall_power_w as f32;
    frame.vel[2] = state.gap_to_wall_m as f32;
    frame.force_torque = state.wall_force_n as f32;
    frame.residual = state.residual_conservation_error as f32;
    frame.flags = if state.past_ideal_wall_limit { FLAG_PAST_IDEAL_WALL_LIMIT } else { 0 };
}

fn state_is_finite(state: &VdeState) -> bool {
    state.t_s.is_finite()
        && state.displacement_m.is_finite()
        && state.velocity_m_s.is_finite()
        && state.wall_current_a.is_finite()
        && state.p6_current_a.is_finite()
        && state.field_force_n.is_finite()
        && state.wall_force_n.is_finite()
        && state.p6_force_n.is_finite()
        && state.wall_power_w.is_finite()
        && state.p6_power_w.is_finite()
        && state.supply_power_w.is_finite()
        && state.kinetic_energy_j.is_finite()
        && state.potential_energy_j.is_finite()
        && state.magnetic_energy_j.is_finite()
        && state.energy_j.is_finite()
        && state.energy_anchor_j.is_finite()
        && state.stored_scale_anchor_j.is_finite()
        && state.energy_supplied_j.is_finite()
        && state.energy_dissipated_j.is_finite()
        && state.energy_defect_j.is_finite()
        && state.residual_conservation_error.is_finite()
        && state.gap_to_wall_m.is_finite()
}

/// Every limit a flag of this card can mark: what [`exceedances`] reports
/// beside a flag's bit (`crate::exceedance`).
pub const EXCEEDANCE_KINDS: &[crate::exceedance::ExceedanceKind] = &[
    WALL_KIND,
];

const WALL_KIND: ExceedanceKind = ExceedanceKind::fixed(
    "FLAG_PAST_IDEAL_WALL_LIMIT",
    FLAG_PAST_IDEAL_WALL_LIMIT,
    "ideal_wall_stiffness_ratio",
    "1",
    Side::Below,
    1.0,
    "the push of the wall and P6 as perfect conductors over the held field's stiffness, Ip² Dᵀ L⁻¹ D / G'(z), at one: one at +0.238 m and -0.239 m, 2.267 at the equilibrium",
);

/// The limits the step from `before` to `after` under `load` over `dt_s`
/// is past: one record for each limit a flag set on its frame marks.
/// The value is the card's ideal-wall margin plus one, the ratio the margin
/// is that ratio less one of.
pub fn exceedances(
    card: &TokamakVdeMast,
    before: &VdeState,
    after: &VdeState,
    load: VdeLoad,
    dt_s: f64,
) -> crate::exceedance::Exceedances {
    let _ = (before, load, dt_s);
    let mut records = crate::exceedance::Exceedances::new();
    if after.past_ideal_wall_limit {
        let margin = card.ideal_wall_margin(after.displacement_m).unwrap_or(0.0);
        records.push(Exceedance::fixed(WALL_KIND, margin + 1.0));
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

    use super::{
        ALFVEN_SPEED_M_S, FLOOR_DT_S, FORCE_SERIES_N, GATE_DT_S, GROWTH_RATE_PER_S, LOWER_CONTACT_M,
        MAX_SUBSTEPS, P6_INDUCTANCE_H, PLASMA_CURRENT_A, RESIDUAL_CONSERVATION_LIMIT, StepReject,
        TokamakVdeMast, UPPER_CONTACT_M, VdeLoad, VdeState, WALL_INDUCTANCE_H, WALL_P6_MUTUAL_H, advance,
        clenshaw, clenshaw_slope, rates, start_in_card, step, step_with, unit,
    };
    use crate::frame::LastStateFrame64;

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

    fn within(got: f64, expected: f64, relative: f64, absolute: f64) -> bool {
        (got - expected).abs() <= relative * expected.abs() + absolute
    }

    fn volts(v: f64) -> VdeLoad {
        VdeLoad { p6_voltage_v: v }
    }

    /// The growing mode at 1 mm, as `analysis/card_model.py` writes it.
    fn growing_mode() -> [f64; 4] {
        [0.001, 0.006546503766672119, -21.946577663159488, -8.150423262860404]
    }

    fn start(y: [f64; 4]) -> (TokamakVdeMast, VdeState) {
        let card = TokamakVdeMast::mast_15045();
        let state = VdeState::try_start(&card, y[0], y[1], y[2], y[3]).unwrap();
        (card, state)
    }

    #[test]
    fn constants_are_locked() {
        let card = TokamakVdeMast::mast_15045();
        assert!(card.is_valid());
        assert_eq!(card.plasma_current_a, 755340.5452248498);
        assert_eq!(card.upper_contact_m, 0.38888768851757044);
        assert_eq!(card.lower_contact_m, -0.3910649269819259);
        let alter: [fn(&mut TokamakVdeMast); 20] = [
            |c| c.plasma_current_a *= 1.001,
            |c| c.plasma_mass_kg *= 2.0,
            |c| c.wall_inductance_h *= 1.001,
            |c| c.wall_resistance_ohm *= 0.5,
            |c| c.wall_p6_mutual_h *= 1.001,
            |c| c.p6_inductance_h *= 1.001,
            |c| c.p6_resistance_ohm *= 10.0,
            |c| c.lower_contact_m = -1.0,
            |c| c.upper_contact_m = 1.0,
            |c| c.alfven_speed_m_s = 1.0e9,
            |c| c.force_series_n[1] *= 2.0,
            |c| c.energy_series_j[2] *= 2.0,
            |c| c.wall_coupling_series_h_per_m[79] = 1.0e-9,
            |c| c.p6_coupling_series_h_per_m[0] *= 0.5,
            |c| c.substep_pole_fraction = 1.0,
            |c| c.max_substeps = u32::MAX,
            |c| c.residual_conservation_limit = 1.0,
            |c| c.energy_scale_floor_j = 1.0e9,
            |c| c.gate_dt_s = 1.0,
            |c| c.floor_dt_s = 1.0e-15,
        ];
        for change in alter {
            let mut altered = card;
            change(&mut altered);
            assert!(!altered.is_valid());
            let mut state = VdeState::try_start(&card, 0.001, 0.0, 0.0, 0.0).unwrap();
            let held = state;
            let mut frame = blank_frame();
            assert_eq!(step(&altered, &mut state, &mut frame, volts(0.0), 1.0e-5), Err(StepReject::InvalidCard));
            assert_eq!(state, held);
            assert_eq!(frame.to_bytes(), blank_frame().to_bytes());
        }
    }

    /// The energy series is minus the integral of the force series, the force
    /// is nil at the equilibrium, and the stiffness there is the derivation's.
    #[test]
    fn the_series_hold_their_relations() {
        let card = TokamakVdeMast::mast_15045();
        let slope = 2.0 / (UPPER_CONTACT_M - LOWER_CONTACT_M);
        let mut i = 0;
        while i <= 100 {
            let z = LOWER_CONTACT_M + (UPPER_CONTACT_M - LOWER_CONTACT_M) * (0.005 + 0.99 * i as f64 / 100.0);
            let x = unit(&card, z);
            let force = clenshaw(&card.force_series_n, x);
            let energy_slope = clenshaw_slope(&card.energy_series_j, x) * slope;
            assert!(within(-energy_slope, force, 1.0e-9, 1.0e-6), "{z}: {energy_slope} {force}");
            i += 1;
        }
        // nil at the equilibrium: under 1e-9 of the force's range of 43,918 N
        assert!(card.field_force_n(0.0).unwrap().abs() < 1.0e-9 * 43_918.0 * 1.0);
        assert!(card.field_energy_j(0.0).unwrap().abs() < 1.0e-12);
        // the derivation's direct sum over the 878 cells and 6,744 held filaments is 28,022.5037 N/m
        let stiffness = clenshaw_slope(&FORCE_SERIES_N, unit(&card, 0.0)) * slope;
        assert!(within(stiffness, 28_022.503685, 1.0e-7, 0.0), "{stiffness}");
        // the held field pushes away from the equilibrium on both sides
        assert!(card.field_force_n(0.3).unwrap() > 13_000.0);
        assert!(card.field_force_n(-0.3).unwrap() < -13_000.0);
        let (wall, p6) = card.couplings_h_per_m(0.0).unwrap();
        assert!(within(wall, 1.6600582803e-7, 1.0e-9, 0.0));
        assert!(within(p6, 4.1048057424e-6, 1.0e-9, 0.0));
        assert!(card.couplings_h_per_m(UPPER_CONTACT_M).is_none());
    }

    struct Reference {
        start: [f64; 4],
        volts: f64,
        dt_s: f64,
        steps: usize,
        end: [f64; 4],
        substeps_last: u32,
        dissipated_j: f64,
        supplied_j: f64,
        residual: f64,
    }

    /// Three runs against `analysis/out/reference_runs.json`, written by the
    /// plain-Python twin in the same order of operations.
    #[test]
    fn the_card_agrees_with_its_python_twin() {
        let references = [
            Reference {
                start: growing_mode(),
                volts: 0.0,
                dt_s: 1.0e-4,
                steps: 10,
                end: [0.0010065773496870255, 0.0038555605873482907, -22.09258923504652, -8.204066135114298],
                substeps_last: 2438,
                dissipated_j: 8.875666133603463e-05,
                supplied_j: 0.0,
                residual: 7.649243995808578e-12,
            },
            Reference {
                start: [0.05, 0.0, 0.0, 0.0],
                volts: 0.0,
                dt_s: 1.0e-6,
                steps: 20,
                end: [0.11191246185526248, -9471.868972646647, -13699.564623500464, -710.581591922409],
                substeps_last: 22,
                dissipated_j: 0.12095641557573304,
                supplied_j: 0.0,
                residual: 2.380954843623596e-12,
            },
            Reference {
                start: [0.0, 0.0, 0.0, 0.0],
                volts: 300.0,
                dt_s: 1.0e-4,
                steps: 5,
                end: [0.04982389845487482, 104.80441844897702, -20472.39759181294, 373.9643977202515],
                substeps_last: 2393,
                dissipated_j: 1.8420678008944305,
                supplied_j: 28.489713853730713,
                residual: 5.8458845113683205e-15,
            },
        ];
        for reference in references {
            let (card, mut state) = start(reference.start);
            let mut frame = blank_frame();
            for _ in 0..reference.steps {
                step(&card, &mut state, &mut frame, volts(reference.volts), reference.dt_s).unwrap();
            }
            let got = [state.displacement_m, state.velocity_m_s, state.wall_current_a, state.p6_current_a];
            for i in 0..4 {
                assert!(within(got[i], reference.end[i], 1.0e-10, 1.0e-15), "{i}: {} {}", got[i], reference.end[i]);
            }
            assert_eq!(state.substeps, reference.substeps_last);
            assert!(within(state.energy_dissipated_j, reference.dissipated_j, 1.0e-10, 1.0e-18));
            assert!(within(state.energy_supplied_j, reference.supplied_j, 1.0e-10, 1.0e-18));
            assert!(state.residual_conservation_error < 1.0e3 * reference.residual + 1.0e-14);
            proof_intact(&frame);
        }
    }

    /// From the growing mode the plasma rises at the card's own rate, the wall
    /// time and the stability margin's: nothing in the step is fitted to it.
    /// P6, whose time is 172 ms, carries nearly the current that holds its flux.
    #[test]
    fn the_growing_mode_grows_at_its_rate() {
        let (card, mut state) = start(growing_mode());
        let mut frame = blank_frame();
        for _ in 0..100 {
            step(&card, &mut state, &mut frame, volts(0.0), GATE_DT_S).unwrap();
            assert!(state.residual_conservation_error < 1.0e-9);
        }
        let rate = (state.displacement_m / 0.001).ln() / state.t_s;
        assert!(within(rate, GROWTH_RATE_PER_S, 2.0e-3, 0.0), "{rate}");
        // the velocity over the displacement is the rate too, to the few percent the
        // inertial oscillation the start rings (25.8 µs, damped at 20 per second) adds
        assert!(within(state.velocity_m_s / state.displacement_m, GROWTH_RATE_PER_S, 0.1, 0.0));
        // P6 driven by the growing motion alone: L_c dI/dt + R_c I = -Ip D_c v with
        // v = rate z gives I = -Ip D_c rate z / (L_c rate + R_c), 0.53 of the
        // current that would hold its flux, since its time, 172 ms, is near the
        // growth's, 153 ms; the wall mode's mutual changes it by a few percent
        let (_, p6) = card.couplings_h_per_m(0.0).unwrap();
        let driven = -PLASMA_CURRENT_A * p6 * GROWTH_RATE_PER_S * state.displacement_m
            / (P6_INDUCTANCE_H * GROWTH_RATE_PER_S + super::P6_RESISTANCE_OHM);
        let ratio = state.p6_current_a / driven;
        assert!(ratio > 0.9 && ratio < 1.1, "{ratio}");
        // the wall mode pushes the plasma down, P6 too, and the held field up
        assert!(state.wall_force_n < 0.0 && state.p6_force_n < 0.0 && state.field_force_n > 0.0);
    }

    /// Hard starts, far off the balance of the field and the currents, under
    /// three voltages: the ledger stays far inside its limit at the ceiling.
    #[test]
    fn the_ledger_holds_from_hard_starts() {
        let starts: [[f64; 4]; 6] = [
            [0.05, 0.0, 0.0, 0.0],
            [-0.05, 0.0, 0.0, 0.0],
            [0.0, 20_000.0, 0.0, 0.0],
            [0.05, 0.0, 2.0e4, -1.25e3],
            [0.0, 0.0, 1.0e6, 0.0],
            [0.0, 0.0, 0.0, 2.5e4],
        ];
        for y in starts {
            for v in [0.0, 300.0, -3_000.0] {
                let (card, mut state) = start(y);
                let mut frame = blank_frame();
                for _ in 0..3 {
                    match step(&card, &mut state, &mut frame, volts(v), GATE_DT_S) {
                        Ok(()) => assert!(state.residual_conservation_error < 0.1 * RESIDUAL_CONSERVATION_LIMIT),
                        Err(StepReject::PlasmaAtWall) => break,
                        Err(other) => ::core::panic!("{y:?} {v}: {other:?}"),
                    }
                }
            }
        }
    }

    fn euler(
        card: &TokamakVdeMast,
        y: &[f64; 4],
        k1: &[f64; 4],
        volts: f64,
        h: f64,
    ) -> Result<([f64; 4], [f64; 4]), StepReject> {
        let out = advance(y, k1, h);
        let there = rates(card, &out, volts)?;
        Ok((out, there))
    }

    /// Fourth-order stages with the weights 1, 3, 1, 1 over 6: second order only.
    fn wrong_weights(
        card: &TokamakVdeMast,
        y: &[f64; 4],
        k1: &[f64; 4],
        volts: f64,
        h: f64,
    ) -> Result<([f64; 4], [f64; 4]), StepReject> {
        let k2 = rates(card, &advance(y, k1, 0.5 * h), volts)?;
        let k3 = rates(card, &advance(y, &k2, 0.5 * h), volts)?;
        let k4 = rates(card, &advance(y, &k3, h), volts)?;
        let mut out = [0.0; 4];
        for i in 0..4 {
            out[i] = y[i] + h / 6.0 * (k1[i] + 3.0 * k2[i] + k3[i] + k4[i]);
        }
        let there = rates(card, &out, volts)?;
        Ok((out, there))
    }

    /// The classical method on rates that leave out the voltage the plasma's
    /// motion induces in the circuits: the force on the plasma is kept, so the
    /// work it does appears nowhere.
    fn no_induction(
        card: &TokamakVdeMast,
        y: &[f64; 4],
        _k1: &[f64; 4],
        volts: f64,
        h: f64,
    ) -> Result<([f64; 4], [f64; 4]), StepReject> {
        let falsified = |s: &[f64; 4]| -> Result<[f64; 4], StepReject> {
            let mut r = rates(card, s, volts)?;
            let still = rates(card, &[s[0], 0.0, s[2], s[3]], volts)?;
            r[2] = still[2];
            r[3] = still[3];
            Ok(r)
        };
        let k1 = falsified(y)?;
        let k2 = falsified(&advance(y, &k1, 0.5 * h))?;
        let k3 = falsified(&advance(y, &k2, 0.5 * h))?;
        let k4 = falsified(&advance(y, &k3, h))?;
        let mut out = [0.0; 4];
        for i in 0..4 {
            out[i] = y[i] + h / 6.0 * (k1[i] + 2.0 * k2[i] + 2.0 * k3[i] + k4[i]);
        }
        let there = rates(card, &out, volts)?;
        Ok((out, there))
    }

    fn breach(result: Result<(), StepReject>) -> bool {
        result == Err(StepReject::FirstLawBreach) || result == Err(StepReject::SpontaneousGeneration)
    }

    /// Where the plasma's inertial oscillation carries energy, the explicit
    /// Euler method, a second-order set of weights and rates without the induced
    /// voltage are each named by the ledger, and nothing moves. On the growing
    /// mode alone the first two, at 24,400 substeps a millisecond, are accurate,
    /// and the ledger rightly passes them; the third breaks the balance of work
    /// anywhere.
    #[test]
    fn a_wrong_stepping_method_is_named_by_the_ledger_and_moves_nothing() {
        // with no current in either circuit, rates without the induced voltage
        // are the true rates, so that attack is made where currents flow
        let off_balance: [[f64; 4]; 2] = [[0.05, 0.0, 0.0, 0.0], [0.0, 20_000.0, 0.0, 0.0]];
        let mut cases: std::vec::Vec<(super::Integrator, [f64; 4])> = std::vec::Vec::new();
        for y in off_balance {
            cases.push((euler, y));
            cases.push((wrong_weights, y));
        }
        cases.push((no_induction, growing_mode()));
        cases.push((no_induction, [0.05, 0.0, 2.0e4, -1.25e3]));
        for (integrate, y) in cases {
            {
                let (card, mut state) = start(y);
                let before = state;
                let mut frame = blank_frame();
                let result = step_with(&card, &mut state, &mut frame, volts(0.0), GATE_DT_S, integrate);
                assert!(breach(result), "{result:?} from {y:?}, residual {}", {
                    let (c2, mut s2) = start(y);
                    let mut f2 = blank_frame();
                    let _ = step_with(&c2, &mut s2, &mut f2, volts(0.0), GATE_DT_S, integrate);
                    s2.residual_conservation_error
                });
                assert_eq!(state, before);
                assert_eq!(frame.to_bytes(), blank_frame().to_bytes());
            }
        }
    }

    #[test]
    fn a_falsified_ledger_is_named() {
        let (card, mut state) = start(growing_mode());
        let mut frame = blank_frame();
        step(&card, &mut state, &mut frame, volts(0.0), GATE_DT_S).unwrap();
        let held = frame.to_bytes();
        for (field, change, named) in [
            ("dissipated", 1.0, StepReject::SpontaneousGeneration),
            ("dissipated", -1.0, StepReject::FirstLawBreach),
            ("anchor", 0.5, StepReject::FirstLawBreach),
            ("supplied", 0.5, StepReject::FirstLawBreach),
        ] {
            let mut falsified = state;
            match field {
                "dissipated" => falsified.energy_dissipated_j += change,
                "anchor" => falsified.energy_anchor_j += change,
                _ => falsified.energy_supplied_j += change,
            }
            let before = falsified;
            assert_eq!(step(&card, &mut falsified, &mut frame, volts(0.0), GATE_DT_S), Err(named), "{field}");
            assert_eq!(falsified, before);
            assert_eq!(frame.to_bytes(), held);
        }
    }

    /// Up and down: the step that would carry the boundary to the limiter is
    /// refused as the end, and the state and frame stay at the last step.
    #[test]
    fn the_plasma_reaching_the_wall_ends_the_event() {
        for (z, v, dt) in [(0.38, 30.0, 1.0e-5), (-0.38, -30.0, 1.0e-5), (0.0, -500.0, GATE_DT_S)] {
            let (card, mut state) = start([z, v, 0.0, 0.0]);
            let mut frame = blank_frame();
            let mut landed = 0;
            loop {
                let before = state;
                let held = frame.to_bytes();
                match step(&card, &mut state, &mut frame, volts(0.0), dt) {
                    Ok(()) => {
                        landed += 1;
                        assert!(card.inside_vessel(state.displacement_m));
                        assert!(state.gap_to_wall_m > 0.0);
                    }
                    Err(StepReject::PlasmaAtWall) => {
                        assert_eq!(state, before);
                        assert_eq!(frame.to_bytes(), held);
                        break;
                    }
                    Err(other) => ::core::panic!("({z}, {v}) after {landed} steps: {other:?}, residual {}", state.residual_conservation_error),
                }
                assert!(landed < 100_000);
            }
            // past the ideal-wall limit the plasma reaches the wall within a step of
            // the ceiling: the last landed state need not be near it
            assert!(state.gap_to_wall_m > 0.0);
            proof_intact(&frame);
        }
    }

    #[test]
    fn a_start_outside_the_vessel_is_refused_by_name() {
        let card = TokamakVdeMast::mast_15045();
        for z in [UPPER_CONTACT_M, LOWER_CONTACT_M, 0.5, -0.5, 10.0] {
            assert_eq!(start_in_card(&card, z, 0.0), Err(StepReject::StartOutsideVessel), "{z}");
            assert!(VdeState::try_start(&card, z, 0.0, 0.0, 0.0).is_none());
            let mut state = VdeState::try_start(&card, 0.0, 0.0, 0.0, 0.0).unwrap();
            state.displacement_m = z;
            let before = state;
            let mut frame = blank_frame();
            assert_eq!(step(&card, &mut state, &mut frame, volts(0.0), 1.0e-6), Err(StepReject::StartOutsideVessel));
            assert_eq!(state, before);
            assert_eq!(frame.to_bytes(), blank_frame().to_bytes());
        }
        let below = f64::from_bits(UPPER_CONTACT_M.to_bits() - 4);
        assert_eq!(start_in_card(&card, below, 0.0), Ok(()));
    }

    /// The numbers the description states of the held field: the stiffness at
    /// each contact, the energy it releases to each, and the size of the force
    /// at z = 0 that the derivation takes out (113.3176 N, `derive_card.py`,
    /// stage 4, `out/derive_rest.npz`).
    #[test]
    fn the_held_fields_stated_numbers_are_the_cards_own() {
        let card = TokamakVdeMast::mast_15045();
        let slope = 2.0 / (UPPER_CONTACT_M - LOWER_CONTACT_M);
        let up = clenshaw_slope(&FORCE_SERIES_N, 1.0) * slope;
        let down = clenshaw_slope(&FORCE_SERIES_N, -1.0) * slope;
        assert!((up - 109_777.4).abs() < 0.5, "{up}");
        assert!((down - 109_811.9).abs() < 0.5, "{down}");
        let released_up = -clenshaw(&card.energy_series_j, 1.0);
        let released_down = -clenshaw(&card.energy_series_j, -1.0);
        assert!((released_up - 3_208.75).abs() < 0.05, "{released_up}");
        assert!((released_down - 3_245.02).abs() < 0.05, "{released_down}");
        let removed_n = 113.31755776044929;
        let stiffness = clenshaw_slope(&FORCE_SERIES_N, unit(&card, 0.0)) * slope;
        assert!((removed_n / stiffness * 1.0e3 - 4.04).abs() < 0.005);
        assert!((removed_n * UPPER_CONTACT_M - 44.1).abs() < 0.05);
        assert!((removed_n * -LOWER_CONTACT_M - 44.3).abs() < 0.05);
    }

    /// The speed and the wall are tested at the same stages, so the step asked
    /// for does not turn one into the other. From rest at the equilibrium with
    /// 1 MA in P6 (3.1 MN on 0.6 µg) the plasma reaches its Alfvén speed near
    /// 0.30 m, before the wall: every step from 1e-8 s to 1 ms is refused as
    /// SpeedAtAlfvenSpeed at the same time, within its own length, the state
    /// kept. A start at 0.30 m moving at 10 km/s reaches the wall at far under
    /// the Alfvén speed: every step ends there, PlasmaAtWall.
    #[test]
    fn which_limit_a_run_meets_does_not_follow_the_step_asked_for() {
        for (begin, named) in [
            ([0.0, 0.0, 0.0, 1.0e6], StepReject::SpeedAtAlfvenSpeed),
            ([0.3, 1.0e4, 0.0, 0.0], StepReject::PlasmaAtWall),
        ] {
            let mut stops = std::vec::Vec::new();
            for dt in [1.0e-3, 1.0e-4, 1.0e-5, 1.0e-6, 1.0e-7, 1.0e-8] {
                let (card, mut state) = start(begin);
                let mut frame = blank_frame();
                let reject = loop {
                    let before = state;
                    let held = frame.to_bytes();
                    match step(&card, &mut state, &mut frame, volts(0.0), dt) {
                        Ok(()) => assert!(state.t_s < 1.0e-3, "{dt}"),
                        Err(reject) => {
                            assert_eq!(state, before, "{dt}");
                            assert_eq!(frame.to_bytes(), held, "{dt}");
                            break reject;
                        }
                    }
                };
                assert_eq!(reject, named, "{begin:?} at {dt}: {reject:?} after {} s", state.t_s);
                stops.push((dt, state.t_s));
            }
            // the limit falls inside the refused step of every run: each
            // interval [t, t + dt) holds the finest run's
            let (fine_dt, fine_t) = stops[stops.len() - 1];
            for (dt, t) in &stops {
                assert!(*t < fine_t + fine_dt && t + dt > fine_t, "{begin:?}: {dt} stops at {t}, 1e-8 at {fine_t}");
            }
        }
        // where the speed limit is met: near 0.30 m, at 3.4e-7 s
        let (card, mut state) = start([0.0, 0.0, 0.0, 1.0e6]);
        let mut frame = blank_frame();
        while step(&card, &mut state, &mut frame, volts(0.0), 1.0e-9).is_ok() {}
        assert!(state.displacement_m > 0.29 && state.displacement_m < 0.31, "{}", state.displacement_m);
        assert!(state.t_s > 3.3e-7 && state.t_s < 3.6e-7, "{}", state.t_s);
        assert!(state.velocity_m_s > 0.99 * ALFVEN_SPEED_M_S);
    }

    #[test]
    fn the_alfven_speed_is_a_limit_at_the_start_and_at_every_stage() {
        let card = TokamakVdeMast::mast_15045();
        for v in [ALFVEN_SPEED_M_S, -ALFVEN_SPEED_M_S, 2.0 * ALFVEN_SPEED_M_S] {
            assert_eq!(start_in_card(&card, 0.0, v), Err(StepReject::SpeedAtAlfvenSpeed));
            assert!(VdeState::try_start(&card, 0.0, v, 0.0, 0.0).is_none());
        }
        assert_eq!(start_in_card(&card, 0.0, 0.999 * ALFVEN_SPEED_M_S), Ok(()));
        // a wall current that drives the plasma down through the Alfven speed within a nanosecond
        let (card, mut state) = start([0.3, -0.999 * ALFVEN_SPEED_M_S, -1.0e9, 0.0]);
        let before = state;
        let mut frame = blank_frame();
        assert_eq!(step(&card, &mut state, &mut frame, volts(0.0), 1.0e-9), Err(StepReject::SpeedAtAlfvenSpeed));
        assert_eq!(state, before);
        assert_eq!(frame.to_bytes(), blank_frame().to_bytes());
    }

    /// A step of a second from the growing mode needs about 24 million
    /// substeps: it is refused by name after the budget, and nothing moves.
    #[test]
    fn a_step_beyond_the_substep_budget_is_named() {
        let (card, mut state) = start(growing_mode());
        let before = state;
        let mut frame = blank_frame();
        assert_eq!(step(&card, &mut state, &mut frame, volts(0.0), 1.0), Err(StepReject::SubstepsAboveBudget));
        assert_eq!(state, before);
        assert_eq!(frame.to_bytes(), blank_frame().to_bytes());
        assert_eq!(MAX_SUBSTEPS, 1 << 20);
    }

    #[test]
    fn non_finite_and_non_positive_inputs_are_named() {
        let (card, state) = start(growing_mode());
        for (load, dt, named) in [
            (volts(f64::NAN), 1.0e-4, StepReject::NonFiniteInput),
            (volts(f64::INFINITY), 1.0e-4, StepReject::NonFiniteInput),
            (volts(0.0), f64::NAN, StepReject::NonFiniteInput),
            (volts(0.0), 0.0, StepReject::NonPositiveDt),
            (volts(0.0), -1.0e-4, StepReject::NonPositiveDt),
        ] {
            let mut s = state;
            let mut frame = blank_frame();
            assert_eq!(step(&card, &mut s, &mut frame, load, dt), Err(named));
            assert_eq!(s, state);
            assert_eq!(frame.to_bytes(), blank_frame().to_bytes());
        }
        let mut s = state;
        s.wall_current_a = f64::NAN;
        let mut frame = blank_frame();
        assert_eq!(step(&card, &mut s, &mut frame, volts(0.0), 1.0e-4), Err(StepReject::NonFiniteState));
        assert!(VdeState::try_start(&card, 0.0, 0.0, f64::INFINITY, 0.0).is_none());
    }

    #[test]
    fn the_ceiling_and_the_floor_are_the_cards_own() {
        let card = TokamakVdeMast::mast_15045();
        assert_eq!(card.gate_dt_s(), Some(GATE_DT_S));
        assert_eq!(card.floor_dt_s(), Some(FLOOR_DT_S));
        assert!(FLOOR_DT_S < GATE_DT_S);
        // a step at the floor still moves a clock of 100 s
        assert!(100.0 + FLOOR_DT_S > 100.0);
        // and a step under the floor is not clamped by the card: it lands
        let (card, mut state) = start(growing_mode());
        let mut frame = blank_frame();
        step(&card, &mut state, &mut frame, volts(0.0), 0.5 * FLOOR_DT_S).unwrap();
        assert_eq!(state.t_s, 0.5 * FLOOR_DT_S);
        assert_eq!(state.substeps, 1);
    }

    /// From rest at the equilibrium the amplifier's voltage drives P6 through
    /// the two circuits' inductance: dI_c/dt = L_w V / (L_w L_c − M²), and the
    /// supply's work is in the ledger.
    #[test]
    fn the_amplifier_drives_p6_and_its_work_is_counted() {
        let (card, mut state) = start([0.0, 0.0, 0.0, 0.0]);
        let mut frame = blank_frame();
        let dt = 1.0e-7;
        assert_eq!(step(&card, &mut state, &mut frame, volts(100.0), dt), Ok(()));
        let det = WALL_INDUCTANCE_H * P6_INDUCTANCE_H - WALL_P6_MUTUAL_H * WALL_P6_MUTUAL_H;
        let expected = WALL_INDUCTANCE_H * 100.0 / det * dt;
        assert!(within(state.p6_current_a, expected, 1.0e-3, 0.0), "{} {expected}", state.p6_current_a);
        assert!(state.energy_supplied_j > 0.0);
        assert!(within(state.supply_power_w, 100.0 * state.p6_current_a, 1.0e-15, 0.0));
        for n in 0..100 {
            let r = step(&card, &mut state, &mut frame, volts(100.0), 1.0e-5);
            assert_eq!(r, Ok(()), "step {n}: z {} v {}", state.displacement_m, state.velocity_m_s);
        }
        assert!(state.residual_conservation_error < 1.0e-9);
        assert!(state.energy_supplied_j > state.energy_dissipated_j);
        // P6 above the plasma in the sense of Ip, the lower coil against it: the plasma is pulled up
        assert!(state.displacement_m > 0.0);
    }

    #[test]
    fn the_frame_holds_the_state() {
        let (card, mut state) = start(growing_mode());
        let mut frame = blank_frame();
        step(&card, &mut state, &mut frame, volts(0.0), GATE_DT_S).unwrap();
        assert_eq!(frame.t, state.t_s);
        assert_eq!(frame.pos[0], state.displacement_m as f32);
        assert_eq!(frame.pos[1], state.wall_current_a as f32);
        assert_eq!(frame.pos[2], state.p6_current_a as f32);
        assert_eq!(frame.vel[0], state.velocity_m_s as f32);
        assert_eq!(frame.vel[1], state.wall_power_w as f32);
        assert_eq!(frame.vel[2], state.gap_to_wall_m as f32);
        assert_eq!(frame.force_torque, state.wall_force_n as f32);
        assert_eq!(frame.residual, state.residual_conservation_error as f32);
        assert_eq!(frame.flags, 0);
        assert!(!state.past_ideal_wall_limit);
        proof_intact(&frame);
        assert!(within(state.energy_j, state.kinetic_energy_j + state.potential_energy_j + state.magnetic_energy_j, 1.0e-15, 0.0));
    }

    /// The card's ideal-wall margin is 1.267 at the equilibrium and falls to
    /// zero at +0.238 m and −0.239 m; a landing past it carries the flag, and
    /// the plasma there runs to the wall on its inertial time.
    #[test]
    fn past_the_ideal_wall_limit_the_frame_carries_its_flag() {
        let card = TokamakVdeMast::mast_15045();
        assert!(within(card.ideal_wall_margin(0.0).unwrap(), 1.26696, 1.0e-4, 0.0));
        assert!(card.ideal_wall_margin(0.23).unwrap() > 0.0 && card.ideal_wall_margin(0.24).unwrap() < 0.0);
        assert!(card.ideal_wall_margin(-0.23).unwrap() > 0.0 && card.ideal_wall_margin(-0.24).unwrap() < 0.0);
        assert!(card.ideal_wall_margin(UPPER_CONTACT_M).is_none());
        // balanced past the limit: at rest at 0.26 m with the wall's currents of the linear mode, scaled
        let mode = growing_mode();
        let k = 260.0;
        let (card, mut state) = start([0.26, mode[1] * k, mode[2] * k, mode[3] * k]);
        let mut frame = blank_frame();
        step(&card, &mut state, &mut frame, volts(0.0), 1.0e-7).unwrap();
        assert!(state.past_ideal_wall_limit);
        assert_eq!(frame.flags, super::FLAG_PAST_IDEAL_WALL_LIMIT);
        let mut steps = 0;
        let ended = loop {
            match step(&card, &mut state, &mut frame, volts(0.0), 1.0e-6) {
                Ok(()) => steps += 1,
                Err(reject) => break reject,
            }
            assert!(steps < 100_000);
        };
        assert_eq!(ended, StepReject::PlasmaAtWall);
        // inertial: well inside a millisecond
        assert!(state.t_s < 1.0e-3, "{}", state.t_s);
    }

    #[test]
    fn card_state_and_load_are_copy() {
        fn copy<T: Copy>() {}
        copy::<TokamakVdeMast>();
        copy::<VdeState>();
        copy::<VdeLoad>();
    }
}
