#![no_implicit_prelude]

//! The Solar System as thirteen point masses, stepped through time.
//!
//! The bodies, in the order of every table: the Sun, Mercury, Venus, the
//! Earth, the Moon, the barycentres of the Mars, Jupiter, Saturn, Uranus and
//! Neptune systems, and the asteroids Ceres, Pallas and Vesta. Seventy-eight
//! numbers move through time: a position and a velocity for each, in metres
//! and metres per second, from the barycentre of the Solar System, on the
//! axes of the ICRF. Time is TDB. Nothing is fitted.
//!
//! The caller supplies the positions and the velocities. The card holds no
//! ephemeris and no date: the forces depend on where the bodies are and how
//! they move, and on nothing else.
//!
//! # Where the locked numbers come from
//!
//! Paths are under `ephemeris_solar_system/` in the research folder.
//!
//! - The gravitational parameters, the Earth's J2 and radius, the speed of
//!   light and the astronomical unit: the header of JPL's planetary ephemeris
//!   DE440, `raw/de440/header.440`, SHA-256
//!   `0f4636b663e6f00af9efa2b605ef23635489f56b23dd1a5e3821d3acc6ac3359`
//!   (Park, Folkner, Williams and Boggs 2021, AJ 161:105). The header gives
//!   each parameter in au³/day²: GMS, GM1, GM2, GMB with EMRAT for the Earth
//!   and the Moon, GM4 to GM8, MA0001, MA0002, MA0004. They are converted
//!   with the header's AU and a day of 86 400 s.
//! - The state of 1999-01-01 00:00 TDB (JD 2451179.5) the card was scored
//!   from, kept here for the tests and as the worked example: JPL Horizons
//!   vector tables, `raw/horizons/start_*.txt`. Horizons names its source as
//!   DE441 for the planets and its own orbit solutions for the three
//!   asteroids. DE441 is a calculation whose start values JPL fitted to
//!   observations; it is not a measurement.
//! - Both tables were written into this file as text by
//!   `analysis/03_card_tables.py`. Its output is
//!   `analysis/out/card_solar_system_tables.rs.txt`, SHA-256
//!   `5ac0c00499e92905a8600b2762fc37e47eb2b84a4c6b4645f82055edc56a60ab`,
//!   the text between the two marker lines below.
//!
//! # The model
//!
//! ```text
//! r_i, v_i     position and velocity of body i;  μ_i its gravitational parameter
//! d_ij = r_j − r_i,  ρ_ij = |d_ij|,  c the speed of light
//!
//! d r_i/dt = v_i
//! d v_i/dt = n_i + e_i + o_i
//!
//! n_i = Σ_j μ_j d_ij / ρ_ij³                                    Newton, every pair
//!
//! e_i = Σ_j μ_j d_ij / ρ_ij³ × (1/c²) [ −4 Σ_l μ_l/ρ_il − Σ_k μ_k/ρ_jk
//!                                       + v_i·v_i + 2 v_j·v_j − 4 v_i·v_j
//!                                       − (3/2) (d_ij·v_j / ρ_ij)² + ½ d_ij·n_j ]
//!     + Σ_j μ_j / ρ_ij³ × (1/c²) [ −d_ij·(4 v_i − 3 v_j) ] (v_i − v_j)
//!     + (7/2) (1/c²) Σ_j μ_j n_j / ρ_ij                         relativity, every pair
//!
//! o_moon  = −(3/2) J2 μ_earth R² / ρ⁵ [ (1 − 5 z²/ρ²) x + 2 z p ]
//!           x from the Earth to the Moon, ρ = |x|, p the pole, z = x·p
//! o_earth = −(μ_moon/μ_earth) o_moon                            the Earth's oblateness
//! ```
//!
//! `e` is the point-mass term of Einstein, Infeld and Hoffmann as the JPL
//! ephemerides write it (Park et al. 2021, eq. 27) with β = γ = 1. The pole
//! `p` is the ICRF z axis and does not move.
//!
//! Left out: every other asteroid and the Kuiper belt, Pluto, the Sun's
//! oblateness, the figure of the Moon, every tide, the Earth's figure acting
//! on anything but the Moon, the slow turning of the Earth's pole, and every
//! force that is not gravity.
//!
//! # The step
//!
//! One step is classical RK4 on the seventy-eight numbers and on four more,
//! the booked work and the three components of the booked torque.
//! [`SolarSystem::gate_dt_s`] is 3600 s. Measured with this card from the
//! 1999 state over 2830 days against the same run at 1800 s, the largest
//! distance at the end of a day, the step error at the ceiling is 359 m for
//! the Moon, 90 m for Mercury, 4.3 m for the Earth, 0.30 m for Venus,
//! 0.21 m for Mars and under 0.2 m for each other body, the three asteroids
//! among them.
//! [`SolarSystem::floor_dt_s`] is that divided by 4096. The public step
//! applies neither.
//!
//! The arithmetic is `+ − × ÷` and `sqrt`.
//!
//! # The ledger
//!
//! ```text
//! E = ½ Σ μ_i v_i·v_i − Σ_{i<j} μ_i μ_j / ρ_ij      Newton's energy times G
//! L = Σ μ_i r_i × v_i                               angular momentum times G
//! dW/dt = Σ μ_i v_i·(e_i + o_i)                     work of the two small forces
//! dT/dt = Σ μ_i r_i × (e_i + o_i)                   their torque
//!
//! energy defect  = E − E_anchor − W
//! turning defect = |L − L_anchor − T|
//! residual = max(|energy defect| / LEDGER_SCALE_ENERGY, turning defect / LEDGER_SCALE_TURNING)
//! ```
//!
//! Newton's forces between pairs keep `E` and `L` exactly. The two small
//! forces change them, and the step integrates that change beside the state.
//!
//! What it detects: a step whose positions and velocities do not follow from
//! Newton's pairwise forces and the small forces whose work and torque were
//! booked. One first-order step of 60 s or more, a step of a day, a falsely
//! booked work and a falsely booked torque are each refused by name in this
//! file's tests. Honest steps at the ceiling stayed under 1.0 × 10⁻¹³ of a
//! limit of 10⁻¹² over a million steps, 114 years, from the 1999 state,
//! measured before the card held a state to where the compared run went;
//! that run is now refused 5,934.6 days from the 1999 state as
//! [`StepReject::DistanceOutsideComparedRun`], Saturn farther from the Sun
//! than 9.97 au.
//!
//! The ledger is carried from run to run. A run starts from positions,
//! velocities and a [`SolarSystemLedger`]: the anchors `E_anchor` and
//! `L_anchor` and the booked `W` and `T`. A state a run returns, handed back
//! with its ledger, continues the same run bit for bit, and its defect goes
//! on gathering against the same limit. A carried ledger out of balance by
//! more than the limit is refused at the start by the ledger's names. A new
//! start carries its own `E` and `L` as the anchors and no work or torque;
//! the card cannot tell that from a state returned by another run, and it
//! begins a new ledger.
//!
//! What it cannot detect:
//!
//! - A small force that is wrong and booked consistently: relativity left
//!   out, a wrong J2, a wrong speed of light. The comparison of positions
//!   catches those.
//! - A wrong gravitational parameter used by both the step and the ledger.
//! - An error in the Moon's motion smaller than the scale allows. The scale
//!   is the energy of the whole system, which is Jupiter's and the Sun's;
//!   the Moon's orbit about the Earth holds 2 parts in 10⁷ of it.
//! - One first-order step shorter than about 15 s: its error is below the
//!   limit.
//! - How accurate the step is. That rests on the ceiling and on the position
//!   tests.
//!
//! # The frame
//!
//! ```text
//! t             t_s, 64-bit
//! pos[0..3]     the vector from the Earth to the Mars barycentre, each
//!               coordinate rounded to the nearest 32-bit number, metres
//! vel[0..3]     the same coordinate less the number in pos, metres
//! force_torque  the distance from the Earth to the Mars barycentre, metres
//! residual      residual_conservation_error
//! flags         0
//! ```
//!
//! A reader adds `pos[i]` and `vel[i]` as 64-bit numbers and has each
//! coordinate to about a millimetre. Every position and velocity is in the
//! state.
//!
//! # The axes
//!
//! The Earth's pole is held on the z axis of the caller's state, so the axes
//! must be those of the ICRF, whose equator is the Earth's at J2000. A state
//! in the axes of the ecliptic puts the pole 23.4 degrees wrong. The card
//! holds one check that can tell the two apart: the orbit of the Earth-Moon
//! barycentre about the Sun lies in the ecliptic, so the angle between that
//! orbit's angular momentum and the card's pole is the obliquity. In DE441,
//! as JPL Horizons serves it, that angle is 23.426 to 23.452 degrees from
//! 1900 to 2100, and from 9999 BC to AD 9999 it falls steadily from 24.939
//! to 22.560 degrees, with a spread inside a year under 0.0001 degree
//! (`evidence/research/ephemeris_solar_system/analysis/out/frame_axes_after_defects/`).
//! The card takes 21 to 27 degrees, [`ORBIT_POLE_COSINE_MIN`] to
//! [`ORBIT_POLE_COSINE_MAX`] as cosines: the whole of that span with 1.5
//! degrees to spare below and 2 above, which at the rates at its two ends,
//! 0.09 and 0.11 degrees a thousand years, is the rest of DE441's own span,
//! 13,200 BC to AD 17,191. A state in ecliptic axes gives 0 to 1.5 degrees
//! over those years, a state in galactic axes 60 degrees, and a state with
//! the z axis turned over 157 degrees: each is
//! [`StepReject::AxesOutsideCard`]. [`StepReject::FrameOutsideCard`] is the
//! other check of the frame, the speed of the centre of mass, and tells
//! nothing of the axes.
//!
//! What this check cannot tell: a turn of the axes about the pole, by any
//! angle, which leaves the angle as it was and moves nothing the card
//! computes; the equator of another date in place of the ICRF's, which is a
//! turn about the pole of the ecliptic and leaves the angle within the band
//! for thousands of years while the true pole is up to 1.4 degrees a century
//! of precession away from the z axis; and two bodies given in each other's
//! place where their bands overlap, as Ceres and Pallas do.
//!
//! # Where the compared run went
//!
//! The comparison with measurement was one run from the 1999 state in steps
//! of 1800 s. It went on 7,670 days, to 2020-01-01, but the last range
//! compared with measurement is MESSENGER's of 2015-03-18, 5,920.4 days
//! (16.2 years) after the start; from 16.2 to 21 years the run was compared
//! only with DE441, a calculation. The compared run here is that run to the
//! end of day 5,922, as far as the scoring needed to compare the last range
//! (it compares a range only up to one day before the run's end). The card
//! holds a start state, and every landing, to three more things the compared
//! run held to, and refuses by name outside them:
//!
//! - Each body's distance from its primary inside the distances the run went
//!   through, [`COMPARED_DISTANCE_M`], each end rounded outward to 0.01 au.
//!   The run swept the whole orbit of each inner body, of Jupiter and of the
//!   asteroids, and only part of the orbits of Saturn, Uranus and Neptune.
//!   [`StepReject::DistanceOutsideComparedRun`]. Carried on past day 5,922 in
//!   the same steps, the run is refused 5,934.6 days from the start, Saturn
//!   farther from the Sun than 9.97 au: no date is held, and this is where
//!   the start state compared with measurement for 16.2 years stops.
//! - No two bodies but the Earth and the Moon closer than
//!   [`PAIR_SEPARATION_MIN_M`], 0.26 au; the run's closest such pair was
//!   Mercury and Venus at 0.2631 au. No two of the three asteroids closer
//!   than [`ASTEROID_SEPARATION_MIN_M`], 0.38 au; the run's closest were Ceres
//!   and Vesta at 0.3885 au. Point masses closer than that were never
//!   compared, and bodies inside one another are not point masses.
//!   [`StepReject::BodiesTooClose`]; two bodies at exactly one point are
//!   [`StepReject::BodiesCoincide`].
//! - The centre of mass of the thirteen bodies no farther than
//!   [`CENTRE_OF_MASS_DISTANCE_MAX_M`], 1.2 × 10⁵ m, from the origin; the
//!   run's was within 1.1493 × 10⁵ m. A state moved bodily is the same motion
//!   in other numbers, and its rounding is not the run's: moved 6 × 10¹³ m,
//!   the Moon is 98.5 m off in a year. [`StepReject::OriginOutsideCard`].
//!
//! `core` only: no heap and no standard prelude. The card and the state are
//! `Copy`. The 16-byte frame proof is not touched.

extern crate core;

use core::prelude::v1::*;

use crate::frame::LastStateFrame64;

/// How many bodies the card carries.
pub const BODIES: usize = 13;
/// The bodies, in the order of every table.
pub const BODY_NAMES: [&str; BODIES] = [
    "sun", "mercury", "venus", "earth", "moon", "mars", "jupiter", "saturn", "uranus", "neptune",
    "ceres", "pallas", "vesta",
];
/// Index of the Sun.
pub const SUN: usize = 0;
/// Index of the Earth.
pub const EARTH: usize = 3;
/// Index of the Moon.
pub const MOON: usize = 4;
/// Index of the Mars system barycentre.
pub const MARS: usize = 5;

// BEGIN TABLES WRITTEN BY analysis/03_card_tables.py
/// Gravitational parameter of each body, m^3/s^2, in the order of [`BODY_NAMES`].
#[cfg_attr(rustfmt, rustfmt::skip)]
pub const GM_M3_S2: [f64; BODIES] = [
    1.3271244004127944e+20, // sun
    22031868551400.008, // mercury
    324858592000000.06, // venus
    398600435507022.6, // earth
    4902800118457.551, // moon
    42828375815756.11, // mars
    1.267127641e+17, // jupiter
    3.794058484180001e+16, // saturn
    5794556399999999.0, // uranus
    6836527100580400.0, // neptune
    62628888644.40994, // ceres
    13665878145.967424, // pallas
    17288232879.171516, // vesta
];
/// Position of each body at 1999-01-01 00:00 TDB, metres, barycentric, ICRF axes.
#[cfg_attr(rustfmt, rustfmt::skip)]
pub const START_1999_POSITION_M: [[f64; 3]; BODIES] = [
    [-1248725112.811171, 27610869.63072595, 48137495.83829496], // sun
    [-54959739620.612564, -34739034562.710144, -12952097024.96358], // mercury
    [77393032790.07492, -66736135199.51423, -34964599126.811935], // venus
    [-27114553679.253643, 132885999576.4611, 57650147986.189026], // earth
    [-27082679253.72811, 133229568896.714, 57768865212.789925], // moon
    [-238815313009.12262, 65320036889.64328, 36419272178.069244], // mars
    [739091294160.3148, 40261266007.14453, -745200303.3874352], // jupiter
    [1165552102589.3708, 709014149527.9258, 242695520007.6174], // saturn
    [2005894424916.856, -2000753337408.4849, -904650764199.8887], // uranus
    [2370869146902.504, -3526515351882.3438, -1502448884368.042], // neptune
    [113102746429.3008, 361147974066.7736, 146744266009.8253], // ceres
    [413969510387.4615, 93477204959.91364, -47166322022.203026], // pallas
    [-221618355660.36792, 265655876376.7846, 134604213038.70992], // vesta
];
/// Velocity of each body at 1999-01-01 00:00 TDB, metres per second.
#[cfg_attr(rustfmt, rustfmt::skip)]
pub const START_1999_VELOCITY_M_S: [[f64; 3]; BODIES] = [
    [1.991257260489346, -14.4420045816918, -6.232991648306259], // sun
    [17683.59083807344, -32937.101057214764, -19426.07856920143], // mercury
    [23974.952177867148, 23504.595057063358, 9057.313503792193], // venus
    [-29793.79442089389, -4924.265176982369, -2135.5630698527852], // earth
    [-30866.171477203872, -4834.924833645682, -2042.638065393104], // moon
    [-6327.562782274058, -19220.612103277228, -8644.36214475904], // mars
    [-815.9947235097208, 12547.37996910138, 5398.220168827967], // jupiter
    [-5718.231163285546, 7411.459233528688, 3306.9100967619233], // saturn
    [4977.37424953215, 3943.308946575435, 1656.6601536241922], // uranus
    [4587.20657983019, 2717.396366599794, 998.0438782606243], // neptune
    [-17524.75132381718, 2120.6058176880683, 4566.961725394932], // ceres
    [-7792.302441953558, 15256.958876855091, -2498.893537480062], // pallas
    [-13792.69196623213, -11924.88840575639, -2945.591421372861], // vesta
];
// END TABLES WRITTEN BY analysis/03_card_tables.py

/// DE440 header, CLIGHT, in metres per second.
pub const SPEED_OF_LIGHT_M_S: f64 = 299792458.0;
/// DE440 header, AU, in metres.
pub const ASTRONOMICAL_UNIT_M: f64 = 149597870700.0;
/// DE440 header, J2E: the Earth's oblateness.
pub const EARTH_J2: f64 = 0.00108262539;
/// DE440 header, RE, in metres: the radius J2 is scaled to.
pub const EARTH_RADIUS_M: f64 = 6378136.6;
/// The Earth's pole, held on the ICRF z axis.
pub const EARTH_POLE: [f64; 3] = [0.0, 0.0, 1.0];

/// The DE440 header's gravitational parameters in au³/day², in the order of
/// [`BODY_NAMES`]; the Earth and the Moon are GMB split by EMRAT.
#[cfg_attr(rustfmt, rustfmt::skip)]
pub const HEADER_GM_AU3_DAY2: [f64; BODIES] = [
    0.295912208284119561e-03,
    0.491250019488931818e-10,
    0.724345233264411869e-09,
    0.899701139294734660e-09 * HEADER_EMRAT / (1.0 + HEADER_EMRAT),
    0.899701139294734660e-09 / (1.0 + HEADER_EMRAT),
    0.954954882972581189e-10,
    0.282534582522579175e-06,
    0.845970599337629027e-07,
    0.129202656496823994e-07,
    0.152435734788519386e-07,
    0.139645181230810698e-12,
    0.304711463300432000e-13,
    0.385480002252579039e-13,
];
/// DE440 header, EMRAT: the Earth's mass over the Moon's.
pub const HEADER_EMRAT: f64 = 0.813005682214972154e+02;

/// Each body's distance from its primary, incoming or landing, must be in
/// this band, and so must the semi-major axis of its orbit about that
/// primary. Metres. The primary is the Sun, and for the Moon the Earth. The
/// Sun's own row is not used. Each band holds the body's whole orbit with
/// room to spare; the daily positions of 1999 to 2006 lie inside.
#[cfg_attr(rustfmt, rustfmt::skip)]
pub const DISTANCE_BAND_M: [[f64; 2]; BODIES] = [
    [0.0, 0.0],
    [0.29 * ASTRONOMICAL_UNIT_M, 0.48 * ASTRONOMICAL_UNIT_M],
    [0.70 * ASTRONOMICAL_UNIT_M, 0.74 * ASTRONOMICAL_UNIT_M],
    [0.97 * ASTRONOMICAL_UNIT_M, 1.03 * ASTRONOMICAL_UNIT_M],
    [3.4e8, 4.2e8],
    [1.35 * ASTRONOMICAL_UNIT_M, 1.70 * ASTRONOMICAL_UNIT_M],
    [4.8 * ASTRONOMICAL_UNIT_M, 5.6 * ASTRONOMICAL_UNIT_M],
    [8.8 * ASTRONOMICAL_UNIT_M, 10.3 * ASTRONOMICAL_UNIT_M],
    [18.0 * ASTRONOMICAL_UNIT_M, 20.4 * ASTRONOMICAL_UNIT_M],
    [29.5 * ASTRONOMICAL_UNIT_M, 30.7 * ASTRONOMICAL_UNIT_M],
    [2.4 * ASTRONOMICAL_UNIT_M, 3.1 * ASTRONOMICAL_UNIT_M],
    [2.0 * ASTRONOMICAL_UNIT_M, 3.6 * ASTRONOMICAL_UNIT_M],
    [2.0 * ASTRONOMICAL_UNIT_M, 2.7 * ASTRONOMICAL_UNIT_M],
];
/// Each body's distance from its primary, incoming or landing, must also lie
/// in this band: the distances the compared run went through, 5,922 days
/// from the 1999 state in steps of 1800 s, to the last range compared with
/// measurement, each end rounded outward to 0.01 au, and to 1000 km for the
/// Moon. Metres. The Sun's own row is not used. The run swept the whole
/// orbit of each inner body, of Jupiter and of the asteroids; Saturn, Uranus
/// and Neptune went through only part of theirs (Saturn 9.0309 to 9.9679 au,
/// Uranus 19.8862 to 20.0988 au, Neptune 29.9668 to 30.1333 au), and a state
/// with one of them elsewhere was not compared.
#[cfg_attr(rustfmt, rustfmt::skip)]
pub const COMPARED_DISTANCE_M: [[f64; 2]; BODIES] = [
    [0.0, 0.0],
    [0.30 * ASTRONOMICAL_UNIT_M, 0.47 * ASTRONOMICAL_UNIT_M],
    [0.71 * ASTRONOMICAL_UNIT_M, 0.73 * ASTRONOMICAL_UNIT_M],
    [0.98 * ASTRONOMICAL_UNIT_M, 1.02 * ASTRONOMICAL_UNIT_M],
    [3.56e8, 4.07e8],
    [1.38 * ASTRONOMICAL_UNIT_M, 1.67 * ASTRONOMICAL_UNIT_M],
    [4.94 * ASTRONOMICAL_UNIT_M, 5.46 * ASTRONOMICAL_UNIT_M],
    [9.03 * ASTRONOMICAL_UNIT_M, 9.97 * ASTRONOMICAL_UNIT_M],
    [19.88 * ASTRONOMICAL_UNIT_M, 20.10 * ASTRONOMICAL_UNIT_M],
    [29.96 * ASTRONOMICAL_UNIT_M, 30.14 * ASTRONOMICAL_UNIT_M],
    [2.54 * ASTRONOMICAL_UNIT_M, 2.99 * ASTRONOMICAL_UNIT_M],
    [2.13 * ASTRONOMICAL_UNIT_M, 3.42 * ASTRONOMICAL_UNIT_M],
    [2.14 * ASTRONOMICAL_UNIT_M, 2.58 * ASTRONOMICAL_UNIT_M],
];
/// The largest speed of the centre of mass of the thirteen bodies. The
/// relativistic term is written for the frame in which the Solar System's
/// barycentre is at rest. The 1999 state has 0.00004 m/s.
pub const CENTRE_OF_MASS_SPEED_MAX_M_S: f64 = 0.01;
/// The largest distance of the centre of mass of the thirteen bodies from
/// the origin, incoming or landing, metres. The positions are taken from the
/// barycentre of the Solar System, and a state moved bodily away from it is
/// the same motion carried in other numbers, with other rounding. In the
/// compared run, 5,922 days from the 1999 state in steps of 1800 s, to the
/// last range compared with measurement, the centre of mass went from
/// 0.9841 × 10⁵ m to 1.1493 × 10⁵ m from the origin. The line is
/// 1.2 × 10⁵ m.
pub const CENTRE_OF_MASS_DISTANCE_MAX_M: f64 = 1.2e5;
/// The smallest distance between any two bodies but the Earth and the Moon,
/// incoming or landing, metres: 0.26 au. In the compared run the closest such
/// pair was Mercury and Venus at 0.2631 au, then Venus and the Earth at
/// 0.2661 au.
pub const PAIR_SEPARATION_MIN_M: f64 = 0.26 * ASTRONOMICAL_UNIT_M;
/// The smallest distance between two of the three asteroids, incoming or
/// landing, metres: 0.38 au. Their bands overlap, so nothing else keeps them
/// apart. In the compared run the closest two were Ceres and Vesta at
/// 0.3885 au.
pub const ASTEROID_SEPARATION_MIN_M: f64 = 0.38 * ASTRONOMICAL_UNIT_M;
/// Index of the first of the three asteroids, Ceres; Pallas and Vesta follow.
pub const FIRST_ASTEROID: usize = 10;
/// The cosine of 27 degrees: the smallest cosine of the angle between the
/// angular momentum of the Earth-Moon barycentre about the Sun and the
/// Earth's pole that the card takes.
pub const ORBIT_POLE_COSINE_MIN: f64 = 0.8910065241883679;
/// The cosine of 21 degrees: the largest. In DE441 the angle is 22.56 to
/// 24.94 degrees from 9999 BC to AD 9999, and 23.44 at J2000.
pub const ORBIT_POLE_COSINE_MAX: f64 = 0.9335804264972017;
/// What the energy defect is measured against: the size of the system's
/// energy times G, m⁵/s⁴. The 1999 state has −1.3219 × 10²⁵.
pub const LEDGER_SCALE_ENERGY: f64 = 1.32e25;
/// What the turning defect is measured against: the size of the system's
/// angular momentum times G, m⁵/s³. The 1999 state has 2.0908 × 10³³.
pub const LEDGER_SCALE_TURNING: f64 = 2.09e33;
/// The largest residual a step may land with.
pub const RESIDUAL_CONSERVATION_LIMIT: f64 = 1.0e-12;
/// Largest step the card stands behind.
pub const GATE_DT_S: f64 = 3600.0;
/// [`GATE_DT_S`] over the smallest step the card stands behind.
pub const GATE_OVER_FLOOR: f64 = 4096.0;

/// Why a step did not advance the bodies or the frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StepReject {
    /// `dt` was NaN or infinite.
    NonFiniteInput,
    /// The incoming state, or the trial step, was non-finite.
    NonFiniteState,
    /// `dt` was zero or negative.
    NonPositiveDt,
    /// A recorded constant is not this card's.
    InvalidSystem,
    /// A body's distance from its primary, incoming or landing, is outside that body's band.
    DistanceOutsideCard,
    /// The semi-major axis of a body's orbit about its primary, incoming or landing, is outside that body's band, or the orbit is not closed.
    OrbitOutsideCard,
    /// A body's distance from its primary, incoming or landing, is outside the distances the compared run went through up to the last range compared with measurement, 16.2 years after the 1999 state: Mercury 0.30 to 0.47 au, Venus 0.71 to 0.73, the Earth 0.98 to 1.02, the Moon 356 000 to 407 000 km, Mars 1.38 to 1.67, Jupiter 4.94 to 5.46, Saturn 9.03 to 9.97, Uranus 19.88 to 20.10, Neptune 29.96 to 30.14, Ceres 2.54 to 2.99, Pallas 2.13 to 3.42, Vesta 2.14 to 2.58 au.
    DistanceOutsideComparedRun,
    /// Two bodies, incoming or landing, are at exactly one point, where the attraction between them has no value.
    BodiesCoincide,
    /// Two bodies other than the Earth and the Moon, incoming or landing, are closer than 0.26 au, or two of the three asteroids closer than 0.38 au: closer than any pair in the compared run.
    BodiesTooClose,
    /// The centre of mass of the thirteen bodies, incoming or landing, moves faster than 0.01 m/s.
    FrameOutsideCard,
    /// The centre of mass of the thirteen bodies, incoming or landing, is more than 120 000 m from the origin.
    OriginOutsideCard,
    /// The orbit of the Earth-Moon barycentre about the Sun, incoming or landing, is not inclined 21 to 27 degrees to the z axis, as the ecliptic is to the ICRF equator: the axes are not those of the ICRF.
    AxesOutsideCard,
    /// The incoming ledger, or the trial step, gained energy that the work of relativity and of the Earth's oblateness does not account for.
    SpontaneousGeneration,
    /// The incoming ledger, or the trial step, lost energy that the work of relativity and of the Earth's oblateness does not account for.
    FirstLawBreach,
    /// The incoming ledger, or the trial step, changed the angular momentum by more than the torque of relativity and of the Earth's oblateness accounts for.
    AngularMomentumBreach,
}

/// The bodies are in free motion. The load is empty.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct SolarSystemLoad {}

/// The locked system. Every field is a recorded copy of a crate constant.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SolarSystem {
    pub gm_m3_s2: [f64; BODIES],
    pub speed_of_light_m_s: f64,
    pub earth_j2: f64,
    pub earth_radius_m: f64,
    pub earth_pole: [f64; 3],
    pub distance_band_m: [[f64; 2]; BODIES],
    pub compared_distance_m: [[f64; 2]; BODIES],
    pub centre_of_mass_speed_max_m_s: f64,
    pub centre_of_mass_distance_max_m: f64,
    pub pair_separation_min_m: f64,
    pub asteroid_separation_min_m: f64,
    pub orbit_pole_cosine_min: f64,
    pub orbit_pole_cosine_max: f64,
    pub ledger_scale_energy: f64,
    pub ledger_scale_turning: f64,
    pub residual_conservation_limit: f64,
}

impl SolarSystem {
    /// The thirteen bodies. The constructor takes no arguments.
    #[must_use]
    pub fn de440() -> Self {
        let system = Self {
            gm_m3_s2: GM_M3_S2,
            speed_of_light_m_s: SPEED_OF_LIGHT_M_S,
            earth_j2: EARTH_J2,
            earth_radius_m: EARTH_RADIUS_M,
            earth_pole: EARTH_POLE,
            distance_band_m: DISTANCE_BAND_M,
            compared_distance_m: COMPARED_DISTANCE_M,
            centre_of_mass_speed_max_m_s: CENTRE_OF_MASS_SPEED_MAX_M_S,
            centre_of_mass_distance_max_m: CENTRE_OF_MASS_DISTANCE_MAX_M,
            pair_separation_min_m: PAIR_SEPARATION_MIN_M,
            asteroid_separation_min_m: ASTEROID_SEPARATION_MIN_M,
            orbit_pole_cosine_min: ORBIT_POLE_COSINE_MIN,
            orbit_pole_cosine_max: ORBIT_POLE_COSINE_MAX,
            ledger_scale_energy: LEDGER_SCALE_ENERGY,
            ledger_scale_turning: LEDGER_SCALE_TURNING,
            residual_conservation_limit: RESIDUAL_CONSERVATION_LIMIT,
        };
        if system.is_valid() {
            system
        } else {
            ::core::panic!("solar system failed is_valid");
        }
    }

    /// Every recorded constant equals the crate's, bit for bit.
    #[must_use]
    pub fn is_valid(&self) -> bool {
        let mut body = 0;
        while body < BODIES {
            if !same_bits(self.gm_m3_s2[body], GM_M3_S2[body])
                || !same_bits(self.distance_band_m[body][0], DISTANCE_BAND_M[body][0])
                || !same_bits(self.distance_band_m[body][1], DISTANCE_BAND_M[body][1])
                || !same_bits(self.compared_distance_m[body][0], COMPARED_DISTANCE_M[body][0])
                || !same_bits(self.compared_distance_m[body][1], COMPARED_DISTANCE_M[body][1])
            {
                return false;
            }
            body += 1;
        }
        let mut axis = 0;
        while axis < 3 {
            if !same_bits(self.earth_pole[axis], EARTH_POLE[axis]) {
                return false;
            }
            axis += 1;
        }
        same_bits(self.speed_of_light_m_s, SPEED_OF_LIGHT_M_S)
            && same_bits(self.earth_j2, EARTH_J2)
            && same_bits(self.earth_radius_m, EARTH_RADIUS_M)
            && same_bits(self.centre_of_mass_speed_max_m_s, CENTRE_OF_MASS_SPEED_MAX_M_S)
            && same_bits(self.centre_of_mass_distance_max_m, CENTRE_OF_MASS_DISTANCE_MAX_M)
            && same_bits(self.pair_separation_min_m, PAIR_SEPARATION_MIN_M)
            && same_bits(self.asteroid_separation_min_m, ASTEROID_SEPARATION_MIN_M)
            && same_bits(self.orbit_pole_cosine_min, ORBIT_POLE_COSINE_MIN)
            && same_bits(self.orbit_pole_cosine_max, ORBIT_POLE_COSINE_MAX)
            && same_bits(self.ledger_scale_energy, LEDGER_SCALE_ENERGY)
            && same_bits(self.ledger_scale_turning, LEDGER_SCALE_TURNING)
            && same_bits(self.residual_conservation_limit, RESIDUAL_CONSERVATION_LIMIT)
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

    /// Smallest step the card stands behind: [`GATE_DT_S`] divided by 4096,
    /// 0.87890625 s.
    ///
    /// The state: in one floor step Neptune, the slowest and farthest body,
    /// moves 4.8 km against a spacing of 0.001 m in a coordinate near
    /// 4.5 × 10¹² m, and its velocity changes by 5.8 × 10⁻⁶ m/s against a
    /// spacing of 9 × 10⁻¹³ m/s. Each still holds more than six figures of
    /// its own movement. A step a thousand times smaller would hold three.
    ///
    /// The ledger: this file's tests take honest steps at the floor from the
    /// 1999 state and every one lands.
    ///
    /// Not applied by [`step`].
    #[must_use]
    pub fn floor_dt_s(&self) -> Option<f64> {
        match self.gate_dt_s() {
            Some(gate_dt_s) => Some(gate_dt_s / GATE_OVER_FLOOR),
            None => None,
        }
    }
}

/// The integrated system. The ledger fields are part of the state; write them
/// only through [`step`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SolarSystemState {
    /// Seconds since the start of the run.
    pub t_s: f64,
    /// Barycentric, ICRF axes, in the order of [`BODY_NAMES`].
    pub position_m: [[f64; 3]; BODIES],
    /// Barycentric, ICRF axes, in the order of [`BODY_NAMES`].
    pub velocity_m_s: [[f64; 3]; BODIES],
    /// `E`: Newton's energy of the thirteen bodies times G, m⁵/s⁴.
    pub energy: f64,
    /// `E` at the state the ledger started from.
    pub energy_anchor: f64,
    /// Work of relativity and of the Earth's oblateness, times G, integrated by the step.
    pub booked_work: f64,
    /// Signed `E − E_anchor − W`. Positive is created energy.
    pub energy_defect: f64,
    /// `L`: angular momentum of the thirteen bodies times G, m⁵/s³.
    pub angular_momentum: [f64; 3],
    /// `L` at the state the ledger started from.
    pub angular_momentum_anchor: [f64; 3],
    /// Torque of relativity and of the Earth's oblateness, times G, integrated by the step.
    pub booked_torque: [f64; 3],
    /// `|L − L_anchor − T|`.
    pub turning_defect: f64,
    /// The larger of `|energy_defect| / LEDGER_SCALE_ENERGY` and `turning_defect / LEDGER_SCALE_TURNING`.
    pub residual_conservation_error: f64,
    /// The vector from the Earth to the Mars system barycentre at this instant.
    pub earth_to_mars_m: [f64; 3],
    /// Its length.
    pub earth_mars_distance_m: f64,
}

/// What a run's ledger has gathered since it was anchored: the four numbers
/// a state handed back as a new start carries so that the new run is held to
/// the same ledger as the run it continues.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SolarSystemLedger {
    /// `E` at the state the ledger started from.
    pub energy_anchor: f64,
    /// Work of relativity and of the Earth's oblateness since then, times G.
    pub booked_work: f64,
    /// `L` at the state the ledger started from.
    pub angular_momentum_anchor: [f64; 3],
    /// Torque of relativity and of the Earth's oblateness since then, times G.
    pub booked_torque: [f64; 3],
}

impl SolarSystemState {
    /// Anchor a new ledger at these positions and velocities. `None` unless
    /// [`SolarSystemState::try_continue`] would take them with the ledger of
    /// a new start: `E` and `L` of these positions and velocities as the
    /// anchors, no work and no torque booked. A request to the gateway
    /// carries its ledger and is read through `try_continue`.
    #[must_use]
    pub fn try_state(
        system: &SolarSystem,
        position_m: [[f64; 3]; BODIES],
        velocity_m_s: [[f64; 3]; BODIES],
    ) -> Option<Self> {
        if !system.is_valid() {
            return None;
        }
        let here = match place(system, &position_m, &velocity_m_s) {
            Ok(here) => here,
            Err(_) => return None,
        };
        let ledger = SolarSystemLedger {
            energy_anchor: here.energy,
            booked_work: 0.0,
            angular_momentum_anchor: here.angular_momentum,
            booked_torque: [0.0; 3],
        };
        match Self::try_continue(system, position_m, velocity_m_s, ledger) {
            Ok(state) => Some(state),
            Err(_) => None,
        }
    }

    /// Start a run at these positions and velocities, held to the ledger it
    /// carries. The ledger is closed at the incoming state as at a landing:
    /// a carried ledger whose energy or angular momentum is out of balance by
    /// more than the limit is refused by the ledger's own names. `t_s` begins
    /// at zero. The state a run returns, with its ledger, continues that run
    /// bit for bit.
    pub fn try_continue(
        system: &SolarSystem,
        position_m: [[f64; 3]; BODIES],
        velocity_m_s: [[f64; 3]; BODIES],
        ledger: SolarSystemLedger,
    ) -> Result<Self, StepReject> {
        if !system.is_valid() {
            return Err(StepReject::InvalidSystem);
        }
        if !ledger.energy_anchor.is_finite()
            || !ledger.booked_work.is_finite()
            || !finite3(ledger.angular_momentum_anchor)
            || !finite3(ledger.booked_torque)
        {
            return Err(StepReject::NonFiniteState);
        }
        let here = place(system, &position_m, &velocity_m_s)?;
        let closed = close_ledger(
            system,
            &here,
            ledger.energy_anchor,
            ledger.angular_momentum_anchor,
            ledger.booked_work,
            ledger.booked_torque,
        )?;
        let state = Self {
            t_s: 0.0,
            position_m,
            velocity_m_s,
            energy: here.energy,
            energy_anchor: ledger.energy_anchor,
            booked_work: ledger.booked_work,
            energy_defect: closed.energy_defect,
            angular_momentum: here.angular_momentum,
            angular_momentum_anchor: ledger.angular_momentum_anchor,
            booked_torque: ledger.booked_torque,
            turning_defect: closed.turning_defect,
            residual_conservation_error: closed.residual,
            earth_to_mars_m: here.earth_to_mars_m,
            earth_mars_distance_m: here.earth_mars_distance_m,
        };
        if state_is_finite(&state) {
            Ok(state)
        } else {
            Err(StepReject::NonFiniteState)
        }
    }

    /// The ledger this state carries.
    #[must_use]
    pub fn ledger(&self) -> SolarSystemLedger {
        SolarSystemLedger {
            energy_anchor: self.energy_anchor,
            booked_work: self.booked_work,
            angular_momentum_anchor: self.angular_momentum_anchor,
            booked_torque: self.booked_torque,
        }
    }

    /// Why [`SolarSystemState::try_state`] would not take these positions and
    /// velocities, by the card's own name for it, in the order the card
    /// decides. `None` when it would take them.
    #[must_use]
    pub fn refusal(
        system: &SolarSystem,
        position_m: &[[f64; 3]; BODIES],
        velocity_m_s: &[[f64; 3]; BODIES],
    ) -> Option<StepReject> {
        if !system.is_valid() {
            return Some(StepReject::InvalidSystem);
        }
        match place(system, position_m, velocity_m_s) {
            Ok(_) => None,
            Err(reject) => Some(reject),
        }
    }
}

/// What RK4 carries: positions, velocities, booked work, booked torque.
#[derive(Clone, Copy)]
struct Trial {
    r: [[f64; 3]; BODIES],
    v: [[f64; 3]; BODIES],
    work: f64,
    torque: [f64; 3],
}

/// Advance one RK4 step and write the frame.
///
/// `load` is empty. A rejected step leaves `state` and `frame` as they were.
#[must_use]
pub fn step(
    system: &SolarSystem,
    state: &mut SolarSystemState,
    frame: &mut LastStateFrame64,
    load: SolarSystemLoad,
    dt_s: f64,
) -> Result<(), StepReject> {
    validate(system, state, load, dt_s)?;
    let predicted = integrate(system, &recorded(state), dt_s)?;
    commit(system, state, frame, dt_s, &predicted)
}

fn recorded(state: &SolarSystemState) -> Trial {
    Trial {
        r: state.position_m,
        v: state.velocity_m_s,
        work: state.booked_work,
        torque: state.booked_torque,
    }
}

fn validate(
    system: &SolarSystem,
    state: &SolarSystemState,
    _load: SolarSystemLoad,
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
    if !system.is_valid() {
        return Err(StepReject::InvalidSystem);
    }
    place(system, &state.position_m, &state.velocity_m_s)?;
    Ok(())
}

/// One classical RK4 step. The stages are trial points of the method, not
/// places the bodies are said to be: the card's bands are applied to the
/// incoming state and to the landing.
fn integrate(system: &SolarSystem, y: &Trial, dt_s: f64) -> Result<Trial, StepReject> {
    let k1 = rates(system, y);
    let k2 = rates(system, &axpy(y, dt_s * 0.5, &k1));
    let k3 = rates(system, &axpy(y, dt_s * 0.5, &k2));
    let k4 = rates(system, &axpy(y, dt_s, &k3));
    let sixth = dt_s / 6.0;
    let mut next = *y;
    let mut body = 0;
    while body < BODIES {
        let mut axis = 0;
        while axis < 3 {
            next.r[body][axis] += sixth
                * (k1.r[body][axis] + 2.0 * k2.r[body][axis] + 2.0 * k3.r[body][axis] + k4.r[body][axis]);
            next.v[body][axis] += sixth
                * (k1.v[body][axis] + 2.0 * k2.v[body][axis] + 2.0 * k3.v[body][axis] + k4.v[body][axis]);
            axis += 1;
        }
        body += 1;
    }
    next.work += sixth * (k1.work + 2.0 * k2.work + 2.0 * k3.work + k4.work);
    let mut axis = 0;
    while axis < 3 {
        next.torque[axis] +=
            sixth * (k1.torque[axis] + 2.0 * k2.torque[axis] + 2.0 * k3.torque[axis] + k4.torque[axis]);
        axis += 1;
    }
    if trial_is_finite(&next) {
        Ok(next)
    } else {
        Err(StepReject::NonFiniteState)
    }
}

fn axpy(y: &Trial, h: f64, k: &Trial) -> Trial {
    let mut out = *y;
    let mut body = 0;
    while body < BODIES {
        let mut axis = 0;
        while axis < 3 {
            out.r[body][axis] += h * k.r[body][axis];
            out.v[body][axis] += h * k.v[body][axis];
            axis += 1;
        }
        body += 1;
    }
    out.work += h * k.work;
    let mut axis = 0;
    while axis < 3 {
        out.torque[axis] += h * k.torque[axis];
        axis += 1;
    }
    out
}

/// The rate of change of everything RK4 carries.
fn rates(system: &SolarSystem, y: &Trial) -> Trial {
    let (newton, small) = accelerations(system, &y.r, &y.v);
    let mut out = Trial {
        r: y.v,
        v: [[0.0; 3]; BODIES],
        work: 0.0,
        torque: [0.0; 3],
    };
    let mut body = 0;
    while body < BODIES {
        let mu = system.gm_m3_s2[body];
        let mut axis = 0;
        while axis < 3 {
            out.v[body][axis] = newton[body][axis] + small[body][axis];
            axis += 1;
        }
        out.work += mu * dot(y.v[body], small[body]);
        let twist = cross(y.r[body], small[body]);
        out.torque[0] += mu * twist[0];
        out.torque[1] += mu * twist[1];
        out.torque[2] += mu * twist[2];
        body += 1;
    }
    out
}

/// Newton's acceleration of each body, and beside it the sum of the two
/// small ones: relativity and the Earth's oblateness.
fn accelerations(
    system: &SolarSystem,
    r: &[[f64; 3]; BODIES],
    v: &[[f64; 3]; BODIES],
) -> ([[f64; 3]; BODIES], [[f64; 3]; BODIES]) {
    let mu = &system.gm_m3_s2;
    let inv_c2 = 1.0 / (system.speed_of_light_m_s * system.speed_of_light_m_s);
    let mut distance = [[0.0_f64; BODIES]; BODIES];
    let mut newton = [[0.0_f64; 3]; BODIES];
    let mut potential = [0.0_f64; BODIES];
    let mut i = 0;
    while i < BODIES {
        let mut j = 0;
        while j < BODIES {
            if j != i {
                let d = sub(r[j], r[i]);
                let rho = dot(d, d).sqrt();
                distance[i][j] = rho;
                let pull = mu[j] / (rho * rho * rho);
                newton[i][0] += pull * d[0];
                newton[i][1] += pull * d[1];
                newton[i][2] += pull * d[2];
                potential[i] += mu[j] / rho;
            }
            j += 1;
        }
        i += 1;
    }
    let mut small = [[0.0_f64; 3]; BODIES];
    let mut i = 0;
    while i < BODIES {
        let vi2 = dot(v[i], v[i]);
        let mut j = 0;
        while j < BODIES {
            if j != i {
                let d = sub(r[j], r[i]);
                let rho = distance[i][j];
                let pull = mu[j] / (rho * rho * rho);
                let d_vj = dot(d, v[j]);
                let d_vi = dot(d, v[i]);
                let along = d_vj / rho;
                let bracket = (-4.0 * potential[i] - potential[j] + vi2 + 2.0 * dot(v[j], v[j])
                    - 4.0 * dot(v[i], v[j])
                    - 1.5 * along * along
                    + 0.5 * dot(d, newton[j]))
                    * inv_c2;
                let drag = pull * (-4.0 * d_vi + 3.0 * d_vj) * inv_c2;
                let lift = 3.5 * inv_c2 * mu[j] / rho;
                let mut axis = 0;
                while axis < 3 {
                    small[i][axis] += pull * bracket * d[axis]
                        + drag * (v[i][axis] - v[j][axis])
                        + lift * newton[j][axis];
                    axis += 1;
                }
            }
            j += 1;
        }
        i += 1;
    }
    let x = sub(r[MOON], r[EARTH]);
    let rho = dot(x, x).sqrt();
    let z = dot(x, system.earth_pole);
    let rho2 = rho * rho;
    let factor = -1.5 * system.earth_j2 * mu[EARTH] * system.earth_radius_m * system.earth_radius_m
        / (rho2 * rho2 * rho);
    let share = mu[MOON] / mu[EARTH];
    let mut axis = 0;
    while axis < 3 {
        let on_moon = factor * ((1.0 - 5.0 * z * z / rho2) * x[axis] + 2.0 * z * system.earth_pole[axis]);
        small[MOON][axis] += on_moon;
        small[EARTH][axis] -= share * on_moon;
        axis += 1;
    }
    (newton, small)
}

fn commit(
    system: &SolarSystem,
    state: &mut SolarSystemState,
    frame: &mut LastStateFrame64,
    dt_s: f64,
    predicted: &Trial,
) -> Result<(), StepReject> {
    let saved = *state;
    if !trial_is_finite(predicted) {
        return Err(StepReject::NonFiniteState);
    }
    let here = place(system, &predicted.r, &predicted.v)?;
    let closed = close_ledger(
        system,
        &here,
        state.energy_anchor,
        state.angular_momentum_anchor,
        predicted.work,
        predicted.torque,
    )?;
    let energy_defect = closed.energy_defect;
    let turning_defect = closed.turning_defect;
    state.t_s += dt_s;
    state.position_m = predicted.r;
    state.velocity_m_s = predicted.v;
    state.energy = here.energy;
    state.booked_work = predicted.work;
    state.energy_defect = energy_defect;
    state.angular_momentum = here.angular_momentum;
    state.booked_torque = predicted.torque;
    state.turning_defect = turning_defect;
    state.residual_conservation_error = closed.residual;
    state.earth_to_mars_m = here.earth_to_mars_m;
    state.earth_mars_distance_m = here.earth_mars_distance_m;
    if !state_is_finite(state) {
        *state = saved;
        return Err(StepReject::NonFiniteState);
    }
    publish(state, frame);
    Ok(())
}

/// The ledger closed at one set of positions and velocities.
struct Closed {
    energy_defect: f64,
    turning_defect: f64,
    residual: f64,
}

/// `E − E_anchor − W` and `|L − L_anchor − T|` at `here`, each held to the
/// limit times its scale and refused by name past it.
fn close_ledger(
    system: &SolarSystem,
    here: &Place,
    energy_anchor: f64,
    angular_momentum_anchor: [f64; 3],
    work: f64,
    torque: [f64; 3],
) -> Result<Closed, StepReject> {
    let energy_scale = system.ledger_scale_energy;
    let turning_scale = system.ledger_scale_turning;
    if !(energy_scale > 0.0 && energy_scale.is_finite() && turning_scale > 0.0 && turning_scale.is_finite()) {
        return Err(StepReject::InvalidSystem);
    }
    let energy_defect = here.energy - energy_anchor - work;
    let mut turning = [0.0_f64; 3];
    let mut axis = 0;
    while axis < 3 {
        turning[axis] = here.angular_momentum[axis] - angular_momentum_anchor[axis] - torque[axis];
        axis += 1;
    }
    let turning_defect = dot(turning, turning).sqrt();
    if !energy_defect.is_finite() || !turning_defect.is_finite() {
        return Err(StepReject::NonFiniteState);
    }
    let limit = system.residual_conservation_limit;
    if energy_defect > limit * energy_scale {
        return Err(StepReject::SpontaneousGeneration);
    }
    if energy_defect < -limit * energy_scale {
        return Err(StepReject::FirstLawBreach);
    }
    if turning_defect > limit * turning_scale {
        return Err(StepReject::AngularMomentumBreach);
    }
    let energy_residual = abs_f64(energy_defect) / energy_scale;
    let turning_residual = turning_defect / turning_scale;
    Ok(Closed {
        energy_defect,
        turning_defect,
        residual: if energy_residual > turning_residual {
            energy_residual
        } else {
            turning_residual
        },
    })
}

/// Each coordinate of the Earth-to-Mars vector goes into the frame as its
/// nearest 32-bit number and, in the slot beside it, what that rounding left
/// over.
fn publish(state: &SolarSystemState, frame: &mut LastStateFrame64) {
    frame.t = state.t_s;
    let mut axis = 0;
    while axis < 3 {
        let coordinate = state.earth_to_mars_m[axis];
        let rounded = coordinate as f32;
        frame.pos[axis] = rounded;
        frame.vel[axis] = (coordinate - f64::from(rounded)) as f32;
        axis += 1;
    }
    frame.force_torque = state.earth_mars_distance_m as f32;
    frame.residual = state.residual_conservation_error as f32;
    frame.flags = 0;
}

/// What follows from one set of positions and velocities.
struct Place {
    energy: f64,
    angular_momentum: [f64; 3],
    earth_to_mars_m: [f64; 3],
    earth_mars_distance_m: f64,
}

/// The derived fields at one set of positions and velocities. A body outside
/// its band, an orbit outside it, a body where the compared run never had it,
/// two bodies at one point or closer than the
/// compared run came, a moving frame, a centre of mass far from the origin or
/// axes that are not the ICRF's are refused by name.
fn place(
    system: &SolarSystem,
    r: &[[f64; 3]; BODIES],
    v: &[[f64; 3]; BODIES],
) -> Result<Place, StepReject> {
    let mu = &system.gm_m3_s2;
    let mut body = 0;
    while body < BODIES {
        if !finite3(r[body]) || !finite3(v[body]) {
            return Err(StepReject::NonFiniteState);
        }
        body += 1;
    }
    let mut body = 1;
    while body < BODIES {
        let primary = if body == MOON { EARTH } else { SUN };
        let x = sub(r[body], r[primary]);
        let u = sub(v[body], v[primary]);
        let distance_m = dot(x, x).sqrt();
        if !distance_m.is_finite() {
            return Err(StepReject::NonFiniteState);
        }
        let band = system.distance_band_m[body];
        if !(distance_m >= band[0] && distance_m <= band[1]) {
            return Err(StepReject::DistanceOutsideCard);
        }
        let pair = mu[body] + mu[primary];
        let semi_major_axis_m = -pair / (dot(u, u) - 2.0 * pair / distance_m);
        if !(semi_major_axis_m >= band[0] && semi_major_axis_m <= band[1]) {
            return Err(StepReject::OrbitOutsideCard);
        }
        body += 1;
    }
    // Then each body's distance is held to the distances the compared run
    // went through; inside the bands, so the bands name a body far off first.
    let mut body = 1;
    while body < BODIES {
        let primary = if body == MOON { EARTH } else { SUN };
        let x = sub(r[body], r[primary]);
        let distance_m = dot(x, x).sqrt();
        let span = system.compared_distance_m[body];
        if !(distance_m >= span[0] && distance_m <= span[1]) {
            return Err(StepReject::DistanceOutsideComparedRun);
        }
        body += 1;
    }
    // Two bodies at one point: named here, before the energy divides by the
    // distance between them. The bands above keep every body off its own
    // primary, so the pairs that can meet are the asteroids, whose bands
    // overlap. Then every pair but the Earth and the Moon is held apart by
    // the closest that pair's kind came in the compared run.
    let mut i = 0;
    while i < BODIES {
        let mut j = i + 1;
        while j < BODIES {
            if r[i][0] == r[j][0] && r[i][1] == r[j][1] && r[i][2] == r[j][2] {
                return Err(StepReject::BodiesCoincide);
            }
            j += 1;
        }
        i += 1;
    }
    let mut i = 0;
    while i < BODIES {
        let mut j = i + 1;
        while j < BODIES {
            if !(i == EARTH && j == MOON) {
                let d = sub(r[j], r[i]);
                let apart_m = dot(d, d).sqrt();
                let least_m = if i >= FIRST_ASTEROID {
                    system.asteroid_separation_min_m
                } else {
                    system.pair_separation_min_m
                };
                if !(apart_m >= least_m) {
                    return Err(StepReject::BodiesTooClose);
                }
            }
            j += 1;
        }
        i += 1;
    }
    let mut total = 0.0;
    let mut momentum = [0.0_f64; 3];
    let mut energy = 0.0;
    let mut angular_momentum = [0.0_f64; 3];
    let mut i = 0;
    while i < BODIES {
        total += mu[i];
        energy += 0.5 * mu[i] * dot(v[i], v[i]);
        let turn = cross(r[i], v[i]);
        let mut axis = 0;
        while axis < 3 {
            momentum[axis] += mu[i] * v[i][axis];
            angular_momentum[axis] += mu[i] * turn[axis];
            axis += 1;
        }
        let mut j = i + 1;
        while j < BODIES {
            let d = sub(r[j], r[i]);
            energy -= mu[i] * mu[j] / dot(d, d).sqrt();
            j += 1;
        }
        i += 1;
    }
    let speed_m_s = dot(momentum, momentum).sqrt() / total;
    if !speed_m_s.is_finite() || !energy.is_finite() || !finite3(angular_momentum) {
        return Err(StepReject::NonFiniteState);
    }
    if !(speed_m_s <= system.centre_of_mass_speed_max_m_s) {
        return Err(StepReject::FrameOutsideCard);
    }
    let mut centre = [0.0_f64; 3];
    let mut i = 0;
    while i < BODIES {
        let mut axis = 0;
        while axis < 3 {
            centre[axis] += mu[i] * r[i][axis];
            axis += 1;
        }
        i += 1;
    }
    let from_origin_m = dot(centre, centre).sqrt() / total;
    if !from_origin_m.is_finite() {
        return Err(StepReject::NonFiniteState);
    }
    if !(from_origin_m <= system.centre_of_mass_distance_max_m) {
        return Err(StepReject::OriginOutsideCard);
    }
    let cosine = orbit_pole_cosine(system, r, v);
    if !(cosine >= system.orbit_pole_cosine_min && cosine <= system.orbit_pole_cosine_max) {
        return Err(StepReject::AxesOutsideCard);
    }
    let to_mars = sub(r[MARS], r[EARTH]);
    Ok(Place {
        energy,
        angular_momentum,
        earth_to_mars_m: to_mars,
        earth_mars_distance_m: dot(to_mars, to_mars).sqrt(),
    })
}

/// The cosine of the angle between the angular momentum of the Earth-Moon
/// barycentre about the Sun and the card's pole. Not a number when that
/// angular momentum is zero, which the comparison with the band refuses.
fn orbit_pole_cosine(system: &SolarSystem, r: &[[f64; 3]; BODIES], v: &[[f64; 3]; BODIES]) -> f64 {
    let mu = &system.gm_m3_s2;
    let pair = mu[EARTH] + mu[MOON];
    let mut x = [0.0_f64; 3];
    let mut u = [0.0_f64; 3];
    let mut axis = 0;
    while axis < 3 {
        x[axis] = (mu[EARTH] * r[EARTH][axis] + mu[MOON] * r[MOON][axis]) / pair - r[SUN][axis];
        u[axis] = (mu[EARTH] * v[EARTH][axis] + mu[MOON] * v[MOON][axis]) / pair - v[SUN][axis];
        axis += 1;
    }
    let turn = cross(x, u);
    dot(turn, system.earth_pole) / dot(turn, turn).sqrt()
}

fn state_is_finite(state: &SolarSystemState) -> bool {
    let mut body = 0;
    while body < BODIES {
        if !finite3(state.position_m[body]) || !finite3(state.velocity_m_s[body]) {
            return false;
        }
        body += 1;
    }
    state.t_s.is_finite()
        && state.energy.is_finite()
        && state.energy_anchor.is_finite()
        && state.booked_work.is_finite()
        && state.energy_defect.is_finite()
        && finite3(state.angular_momentum)
        && finite3(state.angular_momentum_anchor)
        && finite3(state.booked_torque)
        && state.turning_defect.is_finite()
        && state.residual_conservation_error.is_finite()
        && finite3(state.earth_to_mars_m)
        && state.earth_mars_distance_m.is_finite()
}

fn trial_is_finite(trial: &Trial) -> bool {
    let mut body = 0;
    while body < BODIES {
        if !finite3(trial.r[body]) || !finite3(trial.v[body]) {
            return false;
        }
        body += 1;
    }
    trial.work.is_finite() && finite3(trial.torque)
}

fn same_bits(a: f64, b: f64) -> bool {
    a.to_bits() == b.to_bits()
}

fn abs_f64(x: f64) -> f64 {
    if x < 0.0 { -x } else { x }
}

fn finite3(a: [f64; 3]) -> bool {
    a[0].is_finite() && a[1].is_finite() && a[2].is_finite()
}

fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn sub(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

#[cfg(test)]
mod tests {
    extern crate std;

    use super::*;
    use std::assert;
    use std::assert_eq;
    use std::matches;

    // After 48 steps of 1800 s from the 1999 state, recomputed outside the
    // engine in 50-digit arithmetic:
    // evidence/checks/crosscheck/solar_system/recomputed_48_steps_of_1800_s.json.
    #[cfg_attr(rustfmt, rustfmt::skip)]
    const RECOMPUTED_DAY_POSITION_M: [[f64; 3]; BODIES] = [
        [-1248552101.1327434, 26363153.611735843, 47598967.38828745], // sun
        [-53338044380.920166, -37521819397.543, -14606600420.35701], // mercury
        [79434076118.68771, -64680019053.085144, -34168733148.361202], // venus
        [-29684577477.92678, 132440018428.1571, 57456725982.339005], // earth
        [-29745418079.685326, 132780790549.58842, 57579796280.30118], // moon
        [-239354389073.35947, 63657299784.86577, 35671240808.574], // mars
        [739019892576.6663, 41345310460.82637, -278793247.01187205], // jupiter
        [1165057830342.7842, 709654367303.0363, 242981191743.57425], // saturn
        [2006324432249.628, -2000412597813.0208, -904507611714.9272], // uranus
        [2371265468709.378, -3526280549731.5415, -1502362645237.7258], // neptune
        [111587767193.10934, 361328523954.0455, 147137765881.4546], // ceres
        [413293639658.9415, 94794812830.8189, -47381927244.85087], // pallas
        [-222807891770.78717, 264622980039.14755, 134348403106.8193], // vesta
    ];
    #[cfg_attr(rustfmt, rustfmt::skip)]
    const RECOMPUTED_DAY_VELOCITY_M_S: [[f64; 3]; BODIES] = [
        [2.0136485355949447, -14.44030115850411, -6.23294149590272], // sun
        [19835.238999447673, -31466.80292711027, -18863.864292970524], // mercury
        [23268.39757948365, 24087.650496676582, 9364.335767643117], // venus
        [-29696.086533515532, -5399.122886599607, -2341.6810325332813], // earth
        [-30758.893689471068, -5552.025714759847, -2334.0556262732825], // moon
        [-6150.974895821792, -19268.469357108006, -8671.087561067709], // mars
        [-836.8195470806432, 12546.236546717155, 5398.237280515331], // jupiter
        [-5723.255741826368, 7408.396167144145, 3305.8613483890485], // saturn
        [4976.499165556173, 3944.181661763201, 1657.0547644422368], // uranus
        [4586.9093116134745, 2717.8386009459714, 998.2322847371942], // neptune
        [-17544.172123271437, 2058.7754452489676, 4541.816661486772], // ceres
        [-7852.861110003414, 15243.185699189979, -2491.9600782573402], // pallas
        [-13742.81323063129, -11984.720031418497, -2975.9301457014344], // vesta
    ];
    const RECOMPUTED_DAY_BOOKED_WORK: f64 = 50868883215906.37;

    const PROOF: [u8; 16] = [7; 16];

    fn blank_frame() -> LastStateFrame64 {
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

    fn start() -> (SolarSystem, SolarSystemState, LastStateFrame64) {
        let system = SolarSystem::de440();
        let state = SolarSystemState::try_state(&system, START_1999_POSITION_M, START_1999_VELOCITY_M_S)
            .expect("the 1999 state is inside the card");
        (system, state, blank_frame())
    }

    fn norm(a: [f64; 3]) -> f64 {
        dot(a, a).sqrt()
    }

    fn same_frame(a: &LastStateFrame64, b: &LastStateFrame64) -> bool {
        a.to_bytes() == b.to_bytes()
    }

    /// A step that must be refused with this name, leaving the state and the frame as they were.
    fn refused(system: &SolarSystem, state: &SolarSystemState, dt_s: f64, name: StepReject) {
        let mut trial = *state;
        let mut frame = blank_frame();
        let before = frame;
        assert_eq!(step(system, &mut trial, &mut frame, SolarSystemLoad {}, dt_s), Err(name));
        assert!(same_frame(&frame, &before));
        assert_eq!(frame.proof, PROOF);
        // NaN is not equal to itself, so compare the bits
        let mut body = 0;
        while body < BODIES {
            let mut axis = 0;
            while axis < 3 {
                assert_eq!(trial.position_m[body][axis].to_bits(), state.position_m[body][axis].to_bits());
                assert_eq!(trial.velocity_m_s[body][axis].to_bits(), state.velocity_m_s[body][axis].to_bits());
                axis += 1;
            }
            body += 1;
        }
        assert_eq!(trial.t_s.to_bits(), state.t_s.to_bits());
        assert_eq!(trial.booked_work.to_bits(), state.booked_work.to_bits());
        assert_eq!(trial.energy_anchor.to_bits(), state.energy_anchor.to_bits());
    }

    #[test]
    fn the_locked_numbers_are_the_header_s() {
        let system = SolarSystem::de440();
        assert!(system.is_valid());
        let au3_day2 = ASTRONOMICAL_UNIT_M * ASTRONOMICAL_UNIT_M * ASTRONOMICAL_UNIT_M / (86_400.0 * 86_400.0);
        let mut body = 0;
        while body < BODIES {
            let converted = HEADER_GM_AU3_DAY2[body] * au3_day2;
            assert!(abs_f64(GM_M3_S2[body] / converted - 1.0) < 1.0e-15, "{}", BODY_NAMES[body]);
            body += 1;
        }
        assert_eq!(system.gate_dt_s(), Some(3600.0));
        assert_eq!(system.floor_dt_s(), Some(0.87890625));
    }

    #[test]
    fn a_changed_constant_is_not_this_card() {
        let (system, state, _) = start();
        let mut other = system;
        other.gm_m3_s2[MARS] *= 1.0 + 1.0e-15;
        assert!(!other.is_valid());
        assert_eq!(other.gate_dt_s(), None);
        assert_eq!(other.floor_dt_s(), None);
        refused(&other, &state, 1800.0, StepReject::InvalidSystem);
        assert!(SolarSystemState::try_state(&other, START_1999_POSITION_M, START_1999_VELOCITY_M_S).is_none());
        let mut other = system;
        other.speed_of_light_m_s = 3.0e8;
        refused(&other, &state, 1800.0, StepReject::InvalidSystem);
        let mut other = system;
        other.residual_conservation_limit = 1.0;
        refused(&other, &state, 1800.0, StepReject::InvalidSystem);
        let mut other = system;
        other.distance_band_m[MARS][1] *= 2.0;
        refused(&other, &state, 1800.0, StepReject::InvalidSystem);
    }

    #[test]
    fn one_day_lands_on_the_recomputation_in_fifty_digits() {
        let (system, mut state, mut frame) = start();
        let mut k = 0;
        while k < 48 {
            assert_eq!(step(&system, &mut state, &mut frame, SolarSystemLoad {}, 1800.0), Ok(()));
            k += 1;
        }
        assert_eq!(state.t_s, 86_400.0);
        let mut body = 0;
        while body < BODIES {
            let off_m = norm(sub(state.position_m[body], RECOMPUTED_DAY_POSITION_M[body]));
            let off_m_s = norm(sub(state.velocity_m_s[body], RECOMPUTED_DAY_VELOCITY_M_S[body]));
            assert!(off_m < 1.0e-13 * norm(RECOMPUTED_DAY_POSITION_M[body]) + 1.0e-4, "{} {off_m}", BODY_NAMES[body]);
            assert!(off_m_s < 1.0e-12 * norm(RECOMPUTED_DAY_VELOCITY_M_S[body]), "{} {off_m_s}", BODY_NAMES[body]);
            body += 1;
        }
        assert!(abs_f64(state.booked_work / RECOMPUTED_DAY_BOOKED_WORK - 1.0) < 1.0e-9);
        assert!(state.residual_conservation_error < 1.0e-14);
        assert_eq!(frame.proof, PROOF);
        assert_eq!(frame.flags, 0);
        assert_eq!(frame.t, 86_400.0);
    }

    #[test]
    fn the_frame_carries_the_earth_to_mars_vector_to_a_millimetre() {
        let (system, mut state, mut frame) = start();
        assert_eq!(step(&system, &mut state, &mut frame, SolarSystemLoad {}, 3600.0), Ok(()));
        let mut axis = 0;
        while axis < 3 {
            let coordinate = state.position_m[MARS][axis] - state.position_m[EARTH][axis];
            let read = f64::from(frame.pos[axis]) + f64::from(frame.vel[axis]);
            assert!(abs_f64(read - coordinate) < 1.0e-3);
            axis += 1;
        }
        assert_eq!(frame.force_torque, state.earth_mars_distance_m as f32);
        assert_eq!(frame.residual, state.residual_conservation_error as f32);
        let to_mars = sub(state.position_m[MARS], state.position_m[EARTH]);
        assert_eq!(state.earth_to_mars_m, to_mars);
        assert_eq!(state.earth_mars_distance_m, norm(to_mars));
        assert_eq!(
            SolarSystemState::refusal(&system, &state.position_m, &state.velocity_m_s),
            None
        );
    }

    #[test]
    fn a_step_that_is_not_a_number_or_not_forward_is_named() {
        let (system, state, _) = start();
        refused(&system, &state, f64::NAN, StepReject::NonFiniteInput);
        refused(&system, &state, f64::INFINITY, StepReject::NonFiniteInput);
        refused(&system, &state, 0.0, StepReject::NonPositiveDt);
        refused(&system, &state, -1800.0, StepReject::NonPositiveDt);
    }

    #[test]
    fn a_state_that_is_not_a_number_is_named() {
        let (system, state, _) = start();
        let mut broken = state;
        broken.position_m[9][2] = f64::NAN;
        refused(&system, &broken, 1800.0, StepReject::NonFiniteState);
        let mut broken = state;
        broken.velocity_m_s[0][0] = f64::INFINITY;
        refused(&system, &broken, 1800.0, StepReject::NonFiniteState);
        let mut broken = state;
        broken.booked_work = f64::NAN;
        refused(&system, &broken, 1800.0, StepReject::NonFiniteState);
        let mut position_m = START_1999_POSITION_M;
        position_m[2][1] = f64::NAN;
        assert!(SolarSystemState::try_state(&system, position_m, START_1999_VELOCITY_M_S).is_none());
    }

    #[test]
    fn a_body_outside_its_band_is_named_and_nothing_is_pulled_back() {
        let (system, state, _) = start();
        // every body in turn, pushed out along its own direction from its primary, both ways
        let mut body = 1;
        while body < BODIES {
            let primary = if body == MOON { EARTH } else { SUN };
            let x = sub(state.position_m[body], state.position_m[primary]);
            let distance_m = norm(x);
            for edge in [DISTANCE_BAND_M[body][0] * 0.999, DISTANCE_BAND_M[body][1] * 1.001] {
                let mut moved = state;
                let mut axis = 0;
                while axis < 3 {
                    moved.position_m[body][axis] = state.position_m[primary][axis] + x[axis] * (edge / distance_m);
                    axis += 1;
                }
                refused(&system, &moved, 1800.0, StepReject::DistanceOutsideCard);
                assert!(
                    SolarSystemState::try_state(&system, moved.position_m, moved.velocity_m_s).is_none(),
                    "{}",
                    BODY_NAMES[body]
                );
            }
            body += 1;
        }
    }

    #[test]
    fn an_orbit_outside_its_band_is_named() {
        let (system, state, _) = start();
        // Venus stopped dead relative to the Sun: it would fall in
        let mut stopped = state;
        stopped.velocity_m_s[2] = state.velocity_m_s[SUN];
        refused(&system, &stopped, 1800.0, StepReject::OrbitOutsideCard);
        // Mars at twice its speed: not a closed orbit
        let mut fast = state;
        let mut axis = 0;
        while axis < 3 {
            fast.velocity_m_s[MARS][axis] *= 2.0;
            axis += 1;
        }
        refused(&system, &fast, 1800.0, StepReject::OrbitOutsideCard);
        assert!(SolarSystemState::try_state(&system, fast.position_m, fast.velocity_m_s).is_none());
        assert_eq!(
            SolarSystemState::refusal(&system, &fast.position_m, &fast.velocity_m_s),
            Some(StepReject::OrbitOutsideCard)
        );
    }

    /// Two bodies at exactly one point are named before anything divides by
    /// the distance between them: Pallas given the place of Ceres, with its
    /// own velocity and with that of Ceres, and Vesta given the place of
    /// Pallas. A body on its own primary is named by its band, as before.
    #[test]
    fn two_bodies_at_one_point_are_named() {
        let (system, state, _) = start();
        const CERES: usize = 10;
        const PALLAS: usize = 11;
        const VESTA: usize = 12;
        let mut met = state;
        met.position_m[PALLAS] = state.position_m[CERES];
        refused(&system, &met, 1800.0, StepReject::BodiesCoincide);
        assert_eq!(
            SolarSystemState::refusal(&system, &met.position_m, &met.velocity_m_s),
            Some(StepReject::BodiesCoincide)
        );
        assert!(SolarSystemState::try_state(&system, met.position_m, met.velocity_m_s).is_none());
        met.velocity_m_s[PALLAS] = state.velocity_m_s[CERES];
        refused(&system, &met, 1800.0, StepReject::BodiesCoincide);
        assert_eq!(
            SolarSystemState::refusal(&system, &met.position_m, &met.velocity_m_s),
            Some(StepReject::BodiesCoincide)
        );
        // Vesta at the place of Pallas, if that place is inside Vesta's band
        let from_sun_m = norm(sub(state.position_m[PALLAS], state.position_m[SUN]));
        if from_sun_m >= DISTANCE_BAND_M[VESTA][0] && from_sun_m <= DISTANCE_BAND_M[VESTA][1] {
            let mut met = state;
            met.position_m[VESTA] = state.position_m[PALLAS];
            met.velocity_m_s[VESTA] = state.velocity_m_s[PALLAS];
            let named = SolarSystemState::refusal(&system, &met.position_m, &met.velocity_m_s);
            assert!(
                named == Some(StepReject::BodiesCoincide)
                    || named == Some(StepReject::OrbitOutsideCard)
            );
        }
        // one metre apart is not one point, and is closer than the compared run came
        let mut near = state;
        near.position_m[PALLAS] = state.position_m[CERES];
        near.position_m[PALLAS][0] += 1.0;
        near.velocity_m_s[PALLAS] = state.velocity_m_s[CERES];
        assert_eq!(
            SolarSystemState::refusal(&system, &near.position_m, &near.velocity_m_s),
            Some(StepReject::BodiesTooClose)
        );
        // the Moon at the Earth's place and Venus at Mercury's are named by the band, as before
        let mut on_primary = state;
        on_primary.position_m[MOON] = state.position_m[EARTH];
        refused(&system, &on_primary, 1800.0, StepReject::DistanceOutsideCard);
        let mut on_mercury = state;
        on_mercury.position_m[2] = state.position_m[1];
        refused(&system, &on_mercury, 1800.0, StepReject::DistanceOutsideCard);
    }

    #[test]
    fn a_frame_that_moves_is_named() {
        let (system, state, _) = start();
        let mut moving = state;
        let mut body = 0;
        while body < BODIES {
            moving.velocity_m_s[body][0] += 0.02;
            body += 1;
        }
        refused(&system, &moving, 1800.0, StepReject::FrameOutsideCard);
        assert!(SolarSystemState::try_state(&system, moving.position_m, moving.velocity_m_s).is_none());
        // the 1999 state is far inside the line
        let mut momentum = [0.0_f64; 3];
        let mut total = 0.0;
        let mut body = 0;
        while body < BODIES {
            total += GM_M3_S2[body];
            let mut axis = 0;
            while axis < 3 {
                momentum[axis] += GM_M3_S2[body] * state.velocity_m_s[body][axis];
                axis += 1;
            }
            body += 1;
        }
        assert!(norm(momentum) / total < 5.0e-5);
    }

    /// Every body turned about the x axis by `degrees`, positions and velocities.
    fn turned_about_x(state: &SolarSystemState, degrees: f64) -> SolarSystemState {
        let angle = degrees * core::f64::consts::PI / 180.0;
        let (sin, cos) = (crate::math::sin(angle), crate::math::cos(angle));
        let turn = |a: [f64; 3]| [a[0], cos * a[1] - sin * a[2], sin * a[1] + cos * a[2]];
        let mut turned = *state;
        let mut body = 0;
        while body < BODIES {
            turned.position_m[body] = turn(state.position_m[body]);
            turned.velocity_m_s[body] = turn(state.velocity_m_s[body]);
            body += 1;
        }
        turned
    }

    /// A state in the axes of the ecliptic, in galactic axes or with the z
    /// axis turned over is named: the orbit of the Earth-Moon barycentre is
    /// then not 21 to 27 degrees from the pole the card holds.
    #[test]
    fn axes_that_are_not_the_icrf_s_are_named() {
        let (system, state, _) = start();
        let degrees = |s: &SolarSystemState| {
            let cosine = orbit_pole_cosine(&system, &s.position_m, &s.velocity_m_s);
            crate::math::asin(cosine) * 180.0 / core::f64::consts::PI
        };
        // the 1999 state: 23.44 degrees from the pole, 66.56 from the equator
        let own = 90.0 - degrees(&state);
        assert!(own > 23.43 && own < 23.45, "{own}");
        assert_eq!(ORBIT_POLE_COSINE_MIN, 0.8910065241883679);
        assert_eq!(ORBIT_POLE_COSINE_MAX, 0.9335804264972017);
        // the same state in the axes of the ecliptic of J2000: the equator turned by the obliquity
        let ecliptic = turned_about_x(&state, -23.439_291_1);
        let angle = 90.0 - degrees(&ecliptic);
        assert!(angle < 0.05, "{angle}");
        refused(&system, &ecliptic, 1800.0, StepReject::AxesOutsideCard);
        assert_eq!(
            SolarSystemState::refusal(&system, &ecliptic.position_m, &ecliptic.velocity_m_s),
            Some(StepReject::AxesOutsideCard)
        );
        assert!(
            SolarSystemState::try_state(&system, ecliptic.position_m, ecliptic.velocity_m_s)
                .is_none()
        );
        // the ends of the band: turned to 21.1 and 26.9 degrees it is taken, to 20.9 and 27.1 named
        for (to, taken) in [(21.1, true), (26.9, true), (20.9, false), (27.1, false)] {
            let turned = turned_about_x(&state, to - own);
            let angle = 90.0 - degrees(&turned);
            assert!((angle - to).abs() < 0.01, "{angle} {to}");
            let named = SolarSystemState::refusal(&system, &turned.position_m, &turned.velocity_m_s);
            assert_eq!(named, if taken { None } else { Some(StepReject::AxesOutsideCard) }, "{to}");
        }
        // the z axis turned over, and the axes turned a right angle
        for by in [180.0, 90.0, -90.0] {
            let turned = turned_about_x(&state, by);
            refused(&system, &turned, 1800.0, StepReject::AxesOutsideCard);
        }
        let mut altered = system;
        altered.orbit_pole_cosine_min = 0.0;
        assert!(!altered.is_valid());
        let mut altered = system;
        altered.orbit_pole_cosine_max = 1.0;
        assert!(!altered.is_valid());
        // a frame that moves and whose axes are turned is named for its motion first, as before
        let mut moving = ecliptic;
        let mut body = 0;
        while body < BODIES {
            moving.velocity_m_s[body][0] += 0.02;
            body += 1;
        }
        refused(&system, &moving, 1800.0, StepReject::FrameOutsideCard);
    }

    /// What the check of the axes cannot tell, held here so that it stays
    /// said: a turn about the pole is taken, and so are Ceres and Pallas in
    /// each other's place.
    #[test]
    fn a_turn_about_the_pole_and_two_asteroids_exchanged_are_taken() {
        let (system, state, _) = start();
        for by in [30.0_f64, 90.0, 180.0] {
            let angle = by * core::f64::consts::PI / 180.0;
            let (sin, cos) = (crate::math::sin(angle), crate::math::cos(angle));
            let turn = |a: [f64; 3]| [cos * a[0] - sin * a[1], sin * a[0] + cos * a[1], a[2]];
            let mut turned = state;
            let mut body = 0;
            while body < BODIES {
                turned.position_m[body] = turn(state.position_m[body]);
                turned.velocity_m_s[body] = turn(state.velocity_m_s[body]);
                body += 1;
            }
            assert_eq!(
                SolarSystemState::refusal(&system, &turned.position_m, &turned.velocity_m_s),
                None
            );
        }
        let mut exchanged = state;
        exchanged.position_m.swap(10, 11);
        exchanged.velocity_m_s.swap(10, 11);
        assert_eq!(
            SolarSystemState::refusal(&system, &exchanged.position_m, &exchanged.velocity_m_s),
            None
        );
    }

    /// Rule 10: a first-order step is refused by the ledger, by name.
    #[test]
    fn a_first_order_step_is_refused_by_the_ledger() {
        for dt_s in [60.0, 1800.0, 3600.0] {
            let (system, mut state, mut frame) = start();
            let saved = state;
            let y = recorded(&state);
            let euler = axpy(&y, dt_s, &rates(&system, &y));
            assert_eq!(
                commit(&system, &mut state, &mut frame, dt_s, &euler),
                Err(StepReject::SpontaneousGeneration)
            );
            assert_eq!(state, saved);
            assert!(same_frame(&frame, &blank_frame()));
        }
    }

    /// Rule 10: an honest step with a falsified ledger is refused by name.
    #[test]
    fn a_falsified_ledger_is_refused() {
        let (system, mut state, mut frame) = start();
        let saved = state;
        let honest = integrate(&system, &recorded(&state), 1800.0).expect("an honest step");
        let mut less_work = honest;
        less_work.work -= 2.0e-12 * LEDGER_SCALE_ENERGY;
        assert_eq!(
            commit(&system, &mut state, &mut frame, 1800.0, &less_work),
            Err(StepReject::SpontaneousGeneration)
        );
        let mut more_work = honest;
        more_work.work += 2.0e-12 * LEDGER_SCALE_ENERGY;
        assert_eq!(
            commit(&system, &mut state, &mut frame, 1800.0, &more_work),
            Err(StepReject::FirstLawBreach)
        );
        let mut twisted = honest;
        twisted.torque[1] += 2.0e-12 * LEDGER_SCALE_TURNING;
        assert_eq!(
            commit(&system, &mut state, &mut frame, 1800.0, &twisted),
            Err(StepReject::AngularMomentumBreach)
        );
        // Jupiter given one part in a million more speed than the step produced
        let mut pushed = honest;
        let mut axis = 0;
        while axis < 3 {
            pushed.v[6][axis] *= 1.0 + 1.0e-6;
            axis += 1;
        }
        assert_eq!(
            commit(&system, &mut state, &mut frame, 1800.0, &pushed),
            Err(StepReject::SpontaneousGeneration)
        );
        assert_eq!(state, saved);
        assert!(same_frame(&frame, &blank_frame()));
        assert_eq!(commit(&system, &mut state, &mut frame, 1800.0, &honest), Ok(()));
    }

    #[test]
    fn a_step_of_a_day_is_refused_by_the_ledger() {
        let (system, state, _) = start();
        refused(&system, &state, 86_400.0, StepReject::FirstLawBreach);
    }

    /// What the ledger cannot see and a comparison of positions can: relativity left
    /// out and booked as left out. The ledger balances; Mercury is metres away after a day.
    #[test]
    fn relativity_left_out_balances_the_ledger_and_moves_mercury() {
        let (system, mut honest, mut frame) = start();
        let mut newton_only = honest;
        let mut k = 0;
        while k < 48 {
            assert_eq!(step(&system, &mut honest, &mut frame, SolarSystemLoad {}, 1800.0), Ok(()));
            // RK4 with the small forces dropped from both the motion and the booking
            let y = recorded(&newton_only);
            let bare = |y: &Trial| {
                let (newton, _) = accelerations(&system, &y.r, &y.v);
                Trial { r: y.v, v: newton, work: 0.0, torque: [0.0; 3] }
            };
            let k1 = bare(&y);
            let k2 = bare(&axpy(&y, 900.0, &k1));
            let k3 = bare(&axpy(&y, 900.0, &k2));
            let k4 = bare(&axpy(&y, 1800.0, &k3));
            let mut next = axpy(&y, 300.0, &k1);
            next = axpy(&next, 600.0, &k2);
            next = axpy(&next, 600.0, &k3);
            next = axpy(&next, 300.0, &k4);
            let mut scratch = blank_frame();
            assert_eq!(commit(&system, &mut newton_only, &mut scratch, 1800.0, &next), Ok(()));
            k += 1;
        }
        let off_m = norm(sub(honest.position_m[1], newton_only.position_m[1]));
        assert!(off_m > 1.0 && off_m < 100.0, "{off_m}");
    }

    #[test]
    fn steps_at_the_floor_and_at_the_ceiling_land() {
        for dt_s in [GATE_DT_S / GATE_OVER_FLOOR, GATE_DT_S] {
            let (system, mut state, mut frame) = start();
            let mut k = 0;
            while k < 2000 {
                assert_eq!(step(&system, &mut state, &mut frame, SolarSystemLoad {}, dt_s), Ok(()));
                assert!(state.residual_conservation_error < 1.0e-13);
                k += 1;
            }
            assert_eq!(state.t_s, 2000.0 * dt_s);
        }
    }

    /// A final state handed back as a new start continues the same motion, bit for bit.
    #[test]
    fn a_state_handed_back_continues_the_same_run() {
        let (system, mut whole, mut frame) = start();
        let mut k = 0;
        while k < 10 {
            assert_eq!(step(&system, &mut whole, &mut frame, SolarSystemLoad {}, 1800.0), Ok(()));
            k += 1;
        }
        let (_, mut first, _) = start();
        let mut k = 0;
        while k < 5 {
            assert_eq!(step(&system, &mut first, &mut frame, SolarSystemLoad {}, 1800.0), Ok(()));
            k += 1;
        }
        let mut second = SolarSystemState::try_state(&system, first.position_m, first.velocity_m_s)
            .expect("a landed state is a start");
        assert_eq!(second.t_s, 0.0);
        assert_eq!(second.booked_work, 0.0);
        let mut k = 0;
        while k < 5 {
            assert_eq!(step(&system, &mut second, &mut frame, SolarSystemLoad {}, 1800.0), Ok(()));
            k += 1;
        }
        assert_eq!(second.position_m, whole.position_m);
        assert_eq!(second.velocity_m_s, whole.velocity_m_s);
    }

    /// Three years at the ceiling from the 1999 state: every step lands inside the bands.
    #[test]
    fn three_years_at_the_ceiling_stay_inside_the_card() {
        let (system, mut state, mut frame) = start();
        let mut worst = 0.0_f64;
        let mut k = 0;
        while k < 26_298 {
            assert_eq!(step(&system, &mut state, &mut frame, SolarSystemLoad {}, GATE_DT_S), Ok(()));
            if state.residual_conservation_error > worst {
                worst = state.residual_conservation_error;
            }
            k += 1;
        }
        assert!(worst < 1.0e-13, "{worst}");
    }

    /// The run that was compared with measurement, from the 1999 state in
    /// steps of 1800 s to the end of day 5,922, as far as the scoring needed
    /// to compare the last range compared with measurement (MESSENGER,
    /// 5,920.4 days), lands every step
    /// inside every limit, and its distances, closest pairs and farthest
    /// centre are the ones the limits were taken from.
    #[test]
    fn the_compared_run_stays_inside_the_card() {
        let (system, mut state, mut frame) = start();
        let mut closest = f64::INFINITY;
        let mut closest_asteroids = f64::INFINITY;
        let mut farthest_centre = 0.0_f64;
        let mut least = [f64::INFINITY; BODIES];
        let mut most = [0.0_f64; BODIES];
        let mut k = 0;
        loop {
            let r = state.position_m;
            let mut body = 1;
            while body < BODIES {
                let primary = if body == MOON { EARTH } else { SUN };
                let distance_m = norm(sub(r[body], r[primary]));
                least[body] = least[body].min(distance_m);
                most[body] = most[body].max(distance_m);
                body += 1;
            }
            let mut i = 0;
            while i < BODIES {
                let mut j = i + 1;
                while j < BODIES {
                    let apart = norm(sub(r[j], r[i]));
                    if i >= FIRST_ASTEROID {
                        closest_asteroids = closest_asteroids.min(apart);
                    } else if !(i == EARTH && j == MOON) {
                        closest = closest.min(apart);
                    }
                    j += 1;
                }
                i += 1;
            }
            farthest_centre = farthest_centre.max(centre_from_origin_m(&r));
            if k == 5922 * 48 {
                break;
            }
            assert_eq!(step(&system, &mut state, &mut frame, SolarSystemLoad {}, 1800.0), Ok(()), "{k}");
            k += 1;
        }
        let au = ASTRONOMICAL_UNIT_M;
        // each span is the run's own, rounded outward to 0.01 au (1000 km for the Moon)
        let mut body = 1;
        while body < BODIES {
            let unit = if body == MOON { 1.0e6 } else { 0.01 * au };
            let span = COMPARED_DISTANCE_M[body];
            let lo = (least[body] / unit).floor() * unit;
            let hi = (most[body] / unit).ceil() * unit;
            assert!((span[0] - lo).abs() < 1.0e-9 * lo, "{} {} {}", BODY_NAMES[body], span[0], least[body] / au);
            assert!((span[1] - hi).abs() < 1.0e-9 * hi, "{} {} {}", BODY_NAMES[body], span[1], most[body] / au);
            body += 1;
        }
        assert!(most[7] > 9.9678 * au && most[7] < 9.9679 * au, "{}", most[7] / au);
        assert!(least[8] > 19.8862 * au && least[8] < 19.8863 * au, "{}", least[8] / au);
        assert!(least[9] > 29.9668 * au && least[9] < 29.9669 * au, "{}", least[9] / au);
        assert!(closest >= PAIR_SEPARATION_MIN_M && closest > 0.2630 * au && closest < 0.2632 * au, "{}", closest / au);
        assert!(
            closest_asteroids >= ASTEROID_SEPARATION_MIN_M && closest_asteroids > 0.3884 * au && closest_asteroids < 0.3885 * au,
            "{}",
            closest_asteroids / au
        );
        assert!(farthest_centre > 1.1492e5 && farthest_centre < 1.1494e5, "{farthest_centre}");
        assert!(farthest_centre <= CENTRE_OF_MASS_DISTANCE_MAX_M);
        // carried on past the last measured range in the same steps, the run
        // is refused when Saturn goes farther from the Sun than the run had it
        let named = loop {
            let before = state;
            match step(&system, &mut state, &mut frame, SolarSystemLoad {}, 1800.0) {
                Ok(()) => k += 1,
                Err(reject) => {
                    assert_eq!(state, before);
                    break reject;
                }
            }
            assert!(k < 2 * 5922 * 48);
        };
        assert_eq!(named, StepReject::DistanceOutsideComparedRun);
        assert_eq!(k, 284_862, "{k}");
        let saturn = norm(sub(state.position_m[7], state.position_m[SUN])) / au;
        assert!(saturn < 9.97 && saturn > 9.9699, "{saturn}");
    }

    /// Saturn (nearer the Sun), Uranus and Neptune moved along the line from
    /// the Sun to just outside the distances the compared run went through,
    /// inside their bands, are refused as DistanceOutsideComparedRun, incoming
    /// and at a landing. Uranus moved to just inside, with the Sun moved to
    /// keep the centre of mass where it was, is taken. Saturn farther from
    /// the Sun than the run had it is named where the run itself goes there,
    /// in the_compared_run_stays_inside_the_card.
    #[test]
    fn a_body_where_the_compared_run_never_had_it_is_named() {
        let (system, state, _) = start();
        let au = ASTRONOMICAL_UNIT_M;
        let moved_to = |body: usize, to_au: f64, keep_centre: bool| {
            let x = sub(state.position_m[body], state.position_m[SUN]);
            let from = norm(x);
            let by = to_au * au / from - 1.0;
            let mut moved = state;
            let mut axis = 0;
            while axis < 3 {
                moved.position_m[body][axis] += by * x[axis];
                if keep_centre {
                    moved.position_m[SUN][axis] -= GM_M3_S2[body] / GM_M3_S2[SUN] * by * x[axis];
                }
                axis += 1;
            }
            moved
        };
        for (body, to_au) in [(7, 9.02), (8, 19.87), (8, 20.11), (9, 29.95), (9, 30.15)] {
            let band = DISTANCE_BAND_M[body];
            assert!(to_au * au > band[0] && to_au * au < band[1]);
            let moved = moved_to(body, to_au, false);
            assert_eq!(
                SolarSystemState::refusal(&system, &moved.position_m, &moved.velocity_m_s),
                Some(StepReject::DistanceOutsideComparedRun),
                "{} {to_au}",
                BODY_NAMES[body]
            );
            refused(&system, &moved, 1800.0, StepReject::DistanceOutsideComparedRun);
            assert!(SolarSystemState::try_state(&system, moved.position_m, moved.velocity_m_s).is_none());
        }
        let inside = moved_to(8, 19.89, true);
        assert_eq!(SolarSystemState::refusal(&system, &inside.position_m, &inside.velocity_m_s), None);
        // outside the band itself the band is named first, as before
        let far = moved_to(8, 17.9, false);
        assert_eq!(
            SolarSystemState::refusal(&system, &far.position_m, &far.velocity_m_s),
            Some(StepReject::DistanceOutsideCard)
        );
        let mut altered = system;
        altered.compared_distance_m[8][0] = 18.0 * au;
        assert!(!altered.is_valid());
    }

    fn centre_from_origin_m(r: &[[f64; 3]; BODIES]) -> f64 {
        let mut centre = [0.0_f64; 3];
        let mut total = 0.0;
        let mut body = 0;
        while body < BODIES {
            total += GM_M3_S2[body];
            let mut axis = 0;
            while axis < 3 {
                centre[axis] += GM_M3_S2[body] * r[body][axis];
                axis += 1;
            }
            body += 1;
        }
        norm(centre) / total
    }

    /// Pallas inside Ceres, 100 km, 600 km and 1000 km from its centre, and
    /// Pallas 0.379 au from Vesta, are refused as BodiesTooClose, incoming
    /// and at a landing. Venus brought within 0.26 au of the Earth is
    /// refused; the Earth and the Moon are held by the Moon's band alone.
    #[test]
    fn two_bodies_closer_than_the_compared_run_are_named() {
        let (system, state, _) = start();
        const CERES: usize = 10;
        const PALLAS: usize = 11;
        const VESTA: usize = 12;
        for apart_m in [1.0, 1.0e5, 6.0e5, 1.0e6] {
            let mut near = state;
            near.position_m[PALLAS] = state.position_m[CERES];
            near.position_m[PALLAS][0] += apart_m;
            near.velocity_m_s[PALLAS] = state.velocity_m_s[CERES];
            refused(&system, &near, 10.0, StepReject::BodiesTooClose);
            assert_eq!(
                SolarSystemState::refusal(&system, &near.position_m, &near.velocity_m_s),
                Some(StepReject::BodiesTooClose)
            );
            assert!(SolarSystemState::try_state(&system, near.position_m, near.velocity_m_s).is_none());
        }
        // Pallas put on Vesta's own orbit, turned about the Sun's z axis until
        // it is 0.379 au and 0.381 au from Vesta: the same orbit, inside both bands
        let beside_vesta = |apart_au: f64, closing_m_s: f64| {
            let x = sub(state.position_m[VESTA], state.position_m[SUN]);
            let u = sub(state.velocity_m_s[VESTA], state.velocity_m_s[SUN]);
            let across = (x[0] * x[0] + x[1] * x[1]).sqrt();
            let angle = 2.0 * crate::math::asin(apart_au * ASTRONOMICAL_UNIT_M / (2.0 * across));
            let (sin, cos) = (crate::math::sin(angle), crate::math::cos(angle));
            let turn = |a: [f64; 3]| [cos * a[0] - sin * a[1], sin * a[0] + cos * a[1], a[2]];
            let mut near = state;
            let (tx, tu) = (turn(x), turn(u));
            let mut axis = 0;
            while axis < 3 {
                near.position_m[PALLAS][axis] = state.position_m[SUN][axis] + tx[axis];
                near.velocity_m_s[PALLAS][axis] = state.velocity_m_s[SUN][axis] + tu[axis];
                axis += 1;
            }
            let to = sub(near.position_m[VESTA], near.position_m[PALLAS]);
            let gap = norm(to);
            assert!((gap / ASTRONOMICAL_UNIT_M - apart_au).abs() < 1.0e-9);
            let mut axis = 0;
            while axis < 3 {
                near.velocity_m_s[PALLAS][axis] += closing_m_s * to[axis] / gap;
                axis += 1;
            }
            near
        };
        for (apart_au, taken) in [(0.379, false), (0.381, true)] {
            let near = beside_vesta(apart_au, 0.0);
            let named = SolarSystemState::refusal(&system, &near.position_m, &near.velocity_m_s);
            assert_eq!(named, if taken { None } else { Some(StepReject::BodiesTooClose) }, "{apart_au}");
        }
        // a landing: Pallas 0.3801 au from Vesta, closing on it at 1 km/s, in steps of 3600 s
        let closing = beside_vesta(0.3801, 1000.0);
        let mut running = SolarSystemState::try_state(&system, closing.position_m, closing.velocity_m_s)
            .expect("0.3801 au apart is taken");
        let mut frame = blank_frame();
        let mut landed = 0;
        let named = loop {
            let before = running;
            match step(&system, &mut running, &mut frame, SolarSystemLoad {}, 3600.0) {
                Ok(()) => landed += 1,
                Err(reject) => {
                    assert_eq!(running, before);
                    break reject;
                }
            }
            assert!(landed < 20);
        };
        assert_eq!(named, StepReject::BodiesTooClose);
        assert!(landed >= 1, "{landed}");
        // Venus pulled toward the Earth to 0.259 au; its band is 0.70 au from the Sun at least
        let to_earth = sub(state.position_m[EARTH], state.position_m[2]);
        let venus_gap = norm(to_earth);
        let mut near = state;
        let along = 1.0 - 0.259 * ASTRONOMICAL_UNIT_M / venus_gap;
        let mut axis = 0;
        while axis < 3 {
            near.position_m[2][axis] += along * to_earth[axis];
            axis += 1;
        }
        let named = SolarSystemState::refusal(&system, &near.position_m, &near.velocity_m_s);
        let venus_from_sun = norm(sub(near.position_m[2], near.position_m[SUN]));
        if venus_from_sun >= DISTANCE_BAND_M[2][0] && venus_from_sun <= DISTANCE_BAND_M[2][1] {
            assert!(
                named == Some(StepReject::BodiesTooClose) || named == Some(StepReject::OrbitOutsideCard),
                "{named:?}"
            );
        }
        let mut altered = system;
        altered.pair_separation_min_m = 0.0;
        assert!(!altered.is_valid());
        let mut altered = system;
        altered.asteroid_separation_min_m = 0.0;
        assert!(!altered.is_valid());
    }

    /// Every position moved bodily, with the velocities as they were: the
    /// centre of mass just inside 120 000 m from the origin is taken, just
    /// outside it, and 6 × 10¹³ m out, is refused as OriginOutsideCard,
    /// incoming and at a landing.
    #[test]
    fn a_state_far_from_the_origin_is_named() {
        let (system, state, _) = start();
        let moved_by = |s: &SolarSystemState, by: [f64; 3]| {
            let mut moved = *s;
            let mut body = 0;
            while body < BODIES {
                let mut axis = 0;
                while axis < 3 {
                    moved.position_m[body][axis] += by[axis];
                    axis += 1;
                }
                body += 1;
            }
            moved
        };
        let start_centre = centre_from_origin_m(&state.position_m);
        assert!(start_centre > 9.8e4 && start_centre < 9.9e4, "{start_centre}");
        // along the direction the centre already lies in
        let mut centre = [0.0_f64; 3];
        let mut total = 0.0;
        let mut body = 0;
        while body < BODIES {
            total += GM_M3_S2[body];
            let mut axis = 0;
            while axis < 3 {
                centre[axis] += GM_M3_S2[body] * state.position_m[body][axis];
                axis += 1;
            }
            body += 1;
        }
        let unit = [centre[0] / (total * start_centre), centre[1] / (total * start_centre), centre[2] / (total * start_centre)];
        for (to_m, taken) in [(1.199e5, true), (1.201e5, false), (1.0e9, false)] {
            let by = (to_m - start_centre) * 1.0;
            let moved = moved_by(&state, [unit[0] * by, unit[1] * by, unit[2] * by]);
            let named = SolarSystemState::refusal(&system, &moved.position_m, &moved.velocity_m_s);
            assert_eq!(named, if taken { None } else { Some(StepReject::OriginOutsideCard) }, "{to_m}");
        }
        let far = moved_by(&state, [6.0e13, 6.0e13, 6.0e13]);
        refused(&system, &far, 1800.0, StepReject::OriginOutsideCard);
        assert!(SolarSystemState::try_state(&system, far.position_m, far.velocity_m_s).is_none());
        // a landing: the centre drifting out at 0.009 m/s from 119 990 m
        let by = 1.1999e5 - start_centre;
        let mut drifting = moved_by(&state, [unit[0] * by, unit[1] * by, unit[2] * by]);
        let mut body = 0;
        while body < BODIES {
            let mut axis = 0;
            while axis < 3 {
                drifting.velocity_m_s[body][axis] += 0.009 * unit[axis];
                axis += 1;
            }
            body += 1;
        }
        let mut running = SolarSystemState::try_state(&system, drifting.position_m, drifting.velocity_m_s)
            .expect("inside the line and slow enough");
        let mut frame = blank_frame();
        let mut named = None;
        let mut k = 0;
        while k < 10 && named.is_none() {
            let before = running;
            if let Err(reject) = step(&system, &mut running, &mut frame, SolarSystemLoad {}, 3600.0) {
                assert_eq!(running, before);
                named = Some(reject);
            }
            k += 1;
        }
        assert_eq!(named, Some(StepReject::OriginOutsideCard));
        let mut altered = system;
        altered.centre_of_mass_distance_max_m = 1.0e20;
        assert!(!altered.is_valid());
    }

    /// The ledger a run returns, carried into the next run, holds that run to
    /// the same ledger bit for bit; a carried ledger out of balance is refused
    /// at the start by the ledger's own names.
    #[test]
    fn a_carried_ledger_continues_the_run_and_a_false_one_is_named() {
        let (system, mut whole, mut frame) = start();
        let mut k = 0;
        while k < 10 {
            assert_eq!(step(&system, &mut whole, &mut frame, SolarSystemLoad {}, 1800.0), Ok(()));
            k += 1;
        }
        let (_, mut first, _) = start();
        let mut k = 0;
        while k < 5 {
            assert_eq!(step(&system, &mut first, &mut frame, SolarSystemLoad {}, 1800.0), Ok(()));
            k += 1;
        }
        let mut second =
            SolarSystemState::try_continue(&system, first.position_m, first.velocity_m_s, first.ledger())
                .expect("a landed state with its ledger is a start");
        assert_eq!(second.t_s, 0.0);
        assert_eq!(second.booked_work.to_bits(), first.booked_work.to_bits());
        assert_eq!(second.energy_defect.to_bits(), first.energy_defect.to_bits());
        assert_eq!(second.residual_conservation_error.to_bits(), first.residual_conservation_error.to_bits());
        let mut k = 0;
        while k < 5 {
            assert_eq!(step(&system, &mut second, &mut frame, SolarSystemLoad {}, 1800.0), Ok(()));
            k += 1;
        }
        assert_eq!(second.position_m, whole.position_m);
        assert_eq!(second.velocity_m_s, whole.velocity_m_s);
        assert_eq!(second.booked_work.to_bits(), whole.booked_work.to_bits());
        assert_eq!(second.booked_torque, whole.booked_torque);
        assert_eq!(second.energy_defect.to_bits(), whole.energy_defect.to_bits());
        assert_eq!(second.residual_conservation_error.to_bits(), whole.residual_conservation_error.to_bits());
        // the fresh ledger of a new start is the ledger try_state anchors
        let (_, fresh, _) = start();
        assert_eq!(
            SolarSystemState::try_continue(&system, START_1999_POSITION_M, START_1999_VELOCITY_M_S, fresh.ledger()),
            Ok(fresh)
        );
        // the defect gathered by a run is held at the start of the next
        let (r, v) = (first.position_m, first.velocity_m_s);
        let mut ledger = first.ledger();
        ledger.booked_work -= 2.0e-12 * LEDGER_SCALE_ENERGY;
        assert_eq!(SolarSystemState::try_continue(&system, r, v, ledger), Err(StepReject::SpontaneousGeneration));
        let mut ledger = first.ledger();
        ledger.energy_anchor -= 2.0e-12 * LEDGER_SCALE_ENERGY;
        assert_eq!(SolarSystemState::try_continue(&system, r, v, ledger), Err(StepReject::SpontaneousGeneration));
        let mut ledger = first.ledger();
        ledger.booked_work += 2.0e-12 * LEDGER_SCALE_ENERGY;
        assert_eq!(SolarSystemState::try_continue(&system, r, v, ledger), Err(StepReject::FirstLawBreach));
        let mut ledger = first.ledger();
        ledger.booked_torque[2] += 2.0e-12 * LEDGER_SCALE_TURNING;
        assert_eq!(SolarSystemState::try_continue(&system, r, v, ledger), Err(StepReject::AngularMomentumBreach));
        let mut ledger = first.ledger();
        ledger.angular_momentum_anchor[0] = f64::NAN;
        assert_eq!(SolarSystemState::try_continue(&system, r, v, ledger), Err(StepReject::NonFiniteState));
        // a ledger just inside the limit is carried, and the next step is held to what is left
        let mut ledger = first.ledger();
        ledger.booked_work -= 0.999e-12 * LEDGER_SCALE_ENERGY - first.energy_defect;
        let carried = SolarSystemState::try_continue(&system, r, v, ledger).expect("inside the limit");
        assert!(carried.residual_conservation_error > 0.99e-12);
    }

    /// The step error at the ceiling, measured with the card: 2830 days from
    /// the 1999 state at 3600 s against 1800 s, the largest distance of each
    /// body at the end of each day.
    #[test]
    fn the_step_error_at_the_ceiling_is_as_stated_body_by_body() {
        let (system, mut coarse, mut frame) = start();
        let (_, mut fine, _) = start();
        let mut worst = [0.0_f64; BODIES];
        let mut day = 0;
        while day < 2830 {
            let mut k = 0;
            while k < 24 {
                assert_eq!(step(&system, &mut coarse, &mut frame, SolarSystemLoad {}, 3600.0), Ok(()));
                k += 1;
            }
            let mut k = 0;
            while k < 48 {
                assert_eq!(step(&system, &mut fine, &mut frame, SolarSystemLoad {}, 1800.0), Ok(()));
                k += 1;
            }
            let mut body = 0;
            while body < BODIES {
                worst[body] = worst[body].max(norm(sub(coarse.position_m[body], fine.position_m[body])));
                body += 1;
            }
            day += 1;
        }
        std::println!("{worst:?}");
        // Moon 359 m, Mercury 90 m, Earth 4.3 m, Venus 0.30 m, Mars 0.21 m, every other under 0.2 m
        let stated = [0.2, 90.5, 0.305, 4.35, 359.5, 0.215, 0.2, 0.2, 0.2, 0.2, 0.2, 0.2, 0.2];
        let mut body = 0;
        while body < BODIES {
            assert!(worst[body] <= stated[body], "{} {}", BODY_NAMES[body], worst[body]);
            body += 1;
        }
        assert!(worst[MOON] > 358.5 && worst[1] > 89.5 && worst[3] > 4.25);
        assert!(worst[2] > 0.29 && worst[MARS] > 0.2);
    }


}
