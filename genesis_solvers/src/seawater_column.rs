//! A column of seawater at rest: the depth, the density and the sound speed
//! at every measured level, and the depth an echo sounder would read over it.
//!
//! The caller gives one measured cast: the latitude, and at each level the
//! sea pressure, the in-situ temperature and the Practical Salinity, as a CTD
//! or a profiling float records them. The solver returns how deep each level
//! lies, or a named limit. It is the subject V1 opened as "pressure at
//! depth" with one density and one gravity; here both are computed.
//!
//! Water at rest carries the weight of the water above it:
//!
//! ```text
//! dp = g(z) dz / v(S, t, p)
//! g(z) = g_surface(latitude) (1 + GRAVITY_RISE_PER_M z)
//! ```
//!
//! `v` is the specific volume of seawater from the Gibbs function of TEOS-10,
//! as the International Association for the Properties of Water and Steam
//! publishes it: the water part of IAPWS SR7-09 (Feistel 2003) and the saline
//! part of IAPWS R13-08 (Feistel 2008). The sound speed is from the same
//! function. Both are reference equations fitted by their authors to
//! laboratory measurements; nothing in this file is fitted.
//!
//! The layer between two levels has the mean specific volume of the two, and
//! gravity at its middle depth. The middle depth depends on the thickness, so
//! the depths of the whole column are swept until no level moves by more than
//! [`SETTLE_TOLERANCE_M`]. An accepted column is then held to the balance
//! layer by layer: the weight of each layer is its pressure step, to
//! [`HYDROSTATIC_TOLERANCE`] of the step and the weight of
//! [`DEPTH_ROUNDING_SPACINGS`], half, of the spacing of 64-bit numbers at the
//! layer's lower depth. The second part is the rounding of the depth itself:
//! a level's depth is the depth above it plus the layer's thickness, a sum
//! that rounds by at most half that spacing, 9e-13 m at 6,000 m, and the
//! balance reads the thickness back as the difference of the two depths. In
//! a layer of 2 dbar that is 5e-13 of the step. In a layer of 0.02 dbar at
//! 6,000 dbar it is 5e-11 of the step, more than the first part, which is
//! why such a cast was once refused. With it, levels as close as two
//! neighbouring 64-bit pressures are answered. The layer
//! from the sea surface to the first level is counted with the first level's
//! water. An echo crosses a layer in its thickness times the mean of `1/c`
//! at its two levels, and the water between the deepest level and the sea
//! floor at the deepest level's sound speed.
//!
//! The Gibbs function of seawater holds for liquid seawater only: IAPWS
//! R13-08 states its ocean region from the freezing temperature of seawater
//! up, and SR7-09 states the water part from `270.5 K − p × 7.43e-8 K/Pa`
//! up. A level colder than the higher of the two is refused by name. The
//! freezing temperature is where the chemical potential of water in
//! seawater equals the Gibbs energy of ice Ih, R13-08 Eq. (32), with ice
//! from IAPWS R10-06 (2009): air-free seawater, as R13-08 defines it.
//!
//! Compared with measurement on the casts and echo soundings of R/V Knorr
//! cruise 145, the WOCE Indian Ocean expedition of 1994 to 1996: see
//! `evidence/research/marine_knorr145_indian_ocean/`. The casts scored
//! there span the envelope of the `COMPARED_` constants: latitudes −34.0023
//! to +26.4108, the last level no deeper than 6108 dbar, temperatures
//! 0.5088 to 31.169 °C, Practical Salinity 30.9805 to 40.0731, and the last
//! level no more than 86 m above the sea floor. The solver answers past
//! that envelope wherever the Gibbs function holds: any latitude, down to
//! 10,000 dbar, Practical Salinity 0 to 41.8, from the freezing temperature
//! of seawater up to 40 °C, and up to 100 m above the sea floor, so the
//! Arctic, the Southern Ocean and the trenches to 10,000 dbar are answered.
//! Through engine 0.8.0 a cast outside the envelope was refused by name.
//!
//! Left out: the difference between Absolute Salinity and Reference Salinity
//! (the composition anomaly, at most 0.03 g/kg in the open ocean), the
//! atmosphere's departure from one standard atmosphere, the sea surface's
//! height above the geoid, currents, and the local anomaly of gravity.

use genesis_kernel::math;
use crate::normal::NormalOrZero;
use std::fmt;

/// Fewest and most levels of one cast.
pub const LEVELS_MIN: usize = 2;
pub const LEVELS_MAX: usize = 1024;
/// Sea pressure of a level, decibars: absolute pressure less one standard atmosphere.
pub const SEA_PRESSURE_MIN_DBAR: f64 = 0.0;
/// The upper pressure limit of the Gibbs function, 100 MPa.
pub const SEA_PRESSURE_MAX_DBAR: f64 = 10_000.0;
/// The first level lies no deeper than this. The layer from the sea surface
/// down to it has no level above it, so it is weighed at the first level's
/// specific volume alone, a one-level layer in place of the mean of two; the
/// layers between levels may be up to [`LEVEL_GAP_MAX_DBAR`], five times as
/// thick, because each takes the mean of its two levels.
pub const FIRST_LEVEL_MAX_DBAR: f64 = 20.0;
/// No two neighbouring levels are further apart than this.
pub const LEVEL_GAP_MAX_DBAR: f64 = 100.0;
/// Practical Salinity of a level. The upper end keeps Absolute Salinity under 42 g/kg.
pub const PRACTICAL_SALINITY_MIN: f64 = 0.0;
pub const PRACTICAL_SALINITY_MAX: f64 = 41.8;
/// In-situ temperature of a level, degrees Celsius (ITS-90).
pub const TEMPERATURE_MAX_C: f64 = 40.0;
/// The lowest temperature at zero absolute pressure, and its fall with absolute
/// pressure: IAPWS SR7-09 is valid from `270.5 K − p × 7.43e-8 K/Pa`, with
/// `p` the absolute pressure. At sea pressure 0, one standard atmosphere, the
/// lowest temperature is therefore −2.6575284475 °C, not −2.65 °C: −2.6575 °C
/// is taken there and −2.6576 °C is named.
pub const TEMPERATURE_MIN_AT_ZERO_PRESSURE_C: f64 = -2.65;
pub const TEMPERATURE_MIN_FALL_C_PER_PA: f64 = 7.43e-8;
/// The lowest temperature any level may have: the limit at [`SEA_PRESSURE_MAX_DBAR`].
pub const TEMPERATURE_MIN_C: f64 = TEMPERATURE_MIN_AT_ZERO_PRESSURE_C
    - TEMPERATURE_MIN_FALL_C_PER_PA * (SEA_PRESSURE_MAX_DBAR * PA_PER_DBAR + STANDARD_ATMOSPHERE_PA);
pub const LATITUDE_MIN_DEG: f64 = -90.0;
pub const LATITUDE_MAX_DEG: f64 = 90.0;
/// Height of the deepest level above the sea floor, metres.
pub const HEIGHT_ABOVE_BOTTOM_MIN_M: f64 = 0.0;
pub const HEIGHT_ABOVE_BOTTOM_MAX_M: f64 = 100.0;

/// The envelope of the 761 casts compared with measurement: the 632 scored
/// on the held-back legs 7, 8, 9, 10 and 13 of Knorr cruise 145 and the 129
/// of the opened leg 6, every level as the scored request gave it. These
/// are stated, not enforced: a cast outside them is answered.
pub const COMPARED_LATITUDE_MIN_DEG: f64 = -34.0023;
pub const COMPARED_LATITUDE_MAX_DEG: f64 = 26.4108;
/// The deepest last level of a compared cast, decibars.
pub const COMPARED_DEEPEST_MAX_DBAR: f64 = 6108.0;
pub const COMPARED_TEMPERATURE_MIN_C: f64 = 0.5088;
pub const COMPARED_TEMPERATURE_MAX_C: f64 = 31.169;
pub const COMPARED_SALINITY_MIN: f64 = 30.9805;
pub const COMPARED_SALINITY_MAX: f64 = 40.0731;
/// The greatest height above bottom of a compared cast, metres.
pub const COMPARED_HEIGHT_ABOVE_BOTTOM_MAX_M: f64 = 86.0;

/// Ice Ih, IAPWS R10-06 (2009), Tables 1 and 2: the triple point, the
/// normal pressure, the real coefficients `g00..g04` and the residual
/// entropy of the IAPWS-95 convention, which the water part of SR7-09 shares.
pub const ICE_TRIPLE_POINT_K: f64 = 273.16;
pub const ICE_TRIPLE_POINT_PA: f64 = 611.657;
const ICE_G0: [f64; 5] = [
    -0.632_020_233_335_886e6,
    0.655_022_213_658_955,
    -0.189_369_929_326_131e-7,
    0.339_746_123_271_053e-14,
    -0.556_464_869_058_991e-21,
];
const ICE_S0: f64 = -0.332_733_756_492_168e4;
/// The complex constants `t1`, `r1`, `t2`, `r20`, `r21`, `r22`, as (real, imaginary).
const ICE_T1: (f64, f64) = (0.368_017_112_855_051e-1, 0.510_878_114_959_572e-1);
const ICE_R1: (f64, f64) = (0.447_050_716_285_388e2, 0.656_876_847_463_481e2);
const ICE_T2: (f64, f64) = (0.337_315_741_065_416, 0.335_449_415_919_309);
const ICE_R2: [(f64, f64); 3] = [
    (-0.725_974_574_329_220e2, -0.781_008_427_112_870e2),
    (-0.557_107_698_030_123e-4, 0.464_578_634_580_806e-4),
    (0.234_801_409_215_913e-10, -0.285_651_142_904_972e-10),
];
/// The freezing temperature is settled when a Newton step moves it by no more than this, kelvin.
pub const FREEZING_TOLERANCE_K: f64 = 1.0e-12;
/// The freezing search stops after this many Newton steps.
pub const FREEZING_MAX_STEPS: u32 = 30;

pub const PA_PER_DBAR: f64 = 1.0e4;
pub const STANDARD_ATMOSPHERE_PA: f64 = 101_325.0;
/// Reference Salinity in g/kg of water of Practical Salinity 1: 35.16504 / 35.
pub const REFERENCE_SALINITY_G_PER_KG_PER_PRACTICAL: f64 = 35.16504 / 35.0;
/// The reducing temperature, pressure and salinity of the Gibbs function.
pub const REDUCING_TEMPERATURE_K: f64 = 40.0;
pub const REDUCING_PRESSURE_PA: f64 = 1.0e8;
pub const REDUCING_SALINITY_KG_PER_KG: f64 = 0.035_165_04 * 40.0 / 35.0;
/// Gravity at the sea surface, m/s²: `GRAVITY_EQUATOR (1 + (GRAVITY_SIN2 +
/// GRAVITY_SIN4 sin²φ) sin²φ)`, the formula of the TEOS-10 toolbox.
pub const GRAVITY_EQUATOR_M_PER_S2: f64 = 9.780_327;
pub const GRAVITY_SIN2: f64 = 5.2792e-3;
pub const GRAVITY_SIN4: f64 = 2.32e-5;
/// Fractional rise of gravity with each metre of depth.
pub const GRAVITY_RISE_PER_M: f64 = 2.26e-7;
/// The sound speed an echo sounder assumes when it turns a time into a depth.
pub const ECHO_SOUNDER_SOUND_SPEED_M_PER_S: f64 = 1500.0;

/// A column has settled when no level moved by more than this in one sweep.
pub const SETTLE_TOLERANCE_M: f64 = 1.0e-9;
/// The public solver stops after this many sweeps.
pub const MAX_SWEEPS: u32 = 50;
/// Largest imbalance of any layer of an accepted column, as a fraction of its
/// pressure step, before the rounding of its depth is allowed for.
pub const HYDROSTATIC_TOLERANCE: f64 = 1.0e-11;
/// The part of the spacing of 64-bit numbers at a layer's lower depth whose
/// weight is added to what the layer may be off by: half, the most the sum
/// of the depth above and the thickness can round by.
pub const DEPTH_ROUNDING_SPACINGS: f64 = 0.5;

/// The published formulation this solver is checked against.
pub const REFERENCE_NAME: &str = "IAPWS SR7-09, Supplementary Release on a Computationally Efficient Thermodynamic Formulation for Liquid Water for Oceanographic Use (2009), Table 6, and IAPWS R13-08, Release on the IAPWS Formulation 2008 for the Thermodynamic Properties of Seawater (2008), Table 8";
pub const REFERENCE_URL: &str = "https://www.teos-10.org/pubs/IAPWS-09.pdf";
pub const REFERENCE_URL_SALINE: &str = "https://www.teos-10.org/pubs/IAPWS-08.pdf";
/// Date the two releases were read.
pub const REFERENCE_RETRIEVED: &str = "2026-10-09";
/// Every check value agrees to this fraction. The tables print nine figures.
pub const REFERENCE_TOLERANCE: f64 = 5.0e-9;

/// `(j, k, g_jk)` of the water part: IAPWS SR7-09, Table 2.
const WATER: [(usize, usize, f64); 41] = [
    (0, 0, 0.101342743139674e3),
    (1, 0, 0.590578347909402e1),
    (2, 0, -0.123577859330390e5),
    (3, 0, 0.736741204151612e3),
    (4, 0, -0.148185936433658e3),
    (5, 0, 0.580259125842571e2),
    (6, 0, -0.189843846514172e2),
    (7, 0, 0.305081646487967e1),
    (0, 1, 0.100015695367145e6),
    (1, 1, -0.270983805184062e3),
    (2, 1, 0.145503645404680e4),
    (3, 1, -0.672507783145070e3),
    (4, 1, 0.397968445406972e3),
    (5, 1, -0.194618310617595e3),
    (6, 1, 0.635113936641785e2),
    (7, 1, -0.963108119393062e1),
    (0, 2, -0.254457654203630e4),
    (1, 2, 0.776153611613101e3),
    (2, 2, -0.756558385769359e3),
    (3, 2, 0.499360390819152e3),
    (4, 2, -0.301815380621876e3),
    (5, 2, 0.120520654902025e3),
    (6, 2, -0.222897317140459e2),
    (0, 3, 0.284517778446287e3),
    (1, 3, -0.196512550881220e3),
    (2, 3, 0.273479662323528e3),
    (3, 3, -0.239545330654412e3),
    (4, 3, 0.152196371733841e3),
    (5, 3, -0.552723052340152e2),
    (6, 3, 0.817060541818112e1),
    (0, 4, -0.333146754253611e2),
    (1, 4, 0.289796526294175e2),
    (2, 4, -0.555604063817218e2),
    (3, 4, 0.488012518593872e2),
    (4, 4, -0.263748377232802e2),
    (5, 4, 0.648190668077221e1),
    (0, 5, 0.420263108803084e1),
    (1, 5, -0.213290083518327e1),
    (2, 5, 0.434420671917197e1),
    (3, 5, -0.166307106208905e1),
    (0, 6, -0.546428511471039e0),
];

/// `(i, j, k, g_ijk)` of the saline part: IAPWS R13-08, Table 2.
const SALINE: [(usize, usize, usize, f64); 64] = [
    (1, 0, 0, 0.581281456626732e4),
    (2, 0, 0, 0.141627648484197e4),
    (3, 0, 0, -0.243214662381794e4),
    (4, 0, 0, 0.202580115603697e4),
    (5, 0, 0, -0.109166841042967e4),
    (6, 0, 0, 0.374601237877840e3),
    (7, 0, 0, -0.485891069025409e2),
    (1, 1, 0, 0.851226734946706e3),
    (2, 1, 0, 0.168072408311545e3),
    (3, 1, 0, -0.493407510141682e3),
    (4, 1, 0, 0.543835333000098e3),
    (5, 1, 0, -0.196028306689776e3),
    (6, 1, 0, 0.367571622995805e2),
    (2, 2, 0, 0.880031352997204e3),
    (3, 2, 0, -0.430664675978042e2),
    (4, 2, 0, -0.685572509204491e2),
    (2, 3, 0, -0.225267649263401e3),
    (3, 3, 0, -0.100227370861875e2),
    (4, 3, 0, 0.493667694856254e2),
    (2, 4, 0, 0.914260447751259e2),
    (3, 4, 0, 0.875600661808945e0),
    (4, 4, 0, -0.171397577419788e2),
    (2, 5, 0, -0.216603240875311e2),
    (4, 5, 0, 0.249697009569508e1),
    (2, 6, 0, 0.213016970847183e1),
    (2, 0, 1, -0.331049154044839e4),
    (3, 0, 1, 0.199459603073901e3),
    (4, 0, 1, -0.547919133532887e2),
    (5, 0, 1, 0.360284195611086e2),
    (2, 1, 1, 0.729116529735046e3),
    (3, 1, 1, -0.175292041186547e3),
    (4, 1, 1, -0.226683558512829e2),
    (2, 2, 1, -0.860764303783977e3),
    (3, 2, 1, 0.383058066002476e3),
    (2, 3, 1, 0.694244814133268e3),
    (3, 3, 1, -0.460319931801257e3),
    (2, 4, 1, -0.297728741987187e3),
    (3, 4, 1, 0.234565187611355e3),
    (2, 0, 2, 0.384794152978599e3),
    (3, 0, 2, -0.522940909281335e2),
    (4, 0, 2, -0.408193978912261e1),
    (2, 1, 2, -0.343956902961561e3),
    (3, 1, 2, 0.831923927801819e2),
    (2, 2, 2, 0.337409530269367e3),
    (3, 2, 2, -0.541917262517112e2),
    (2, 3, 2, -0.204889641964903e3),
    (2, 4, 2, 0.747261411387560e2),
    (2, 0, 3, -0.965324320107458e2),
    (3, 0, 3, 0.680444942726459e2),
    (4, 0, 3, -0.301755111971161e2),
    (2, 1, 3, 0.124687671116248e3),
    (3, 1, 3, -0.294830643494290e2),
    (2, 2, 3, -0.178314556207638e3),
    (3, 2, 3, 0.256398487389914e2),
    (2, 3, 3, 0.113561697840594e3),
    (2, 4, 3, -0.364872919001588e2),
    (2, 0, 4, 0.158408172766824e2),
    (3, 0, 4, -0.341251932441282e1),
    (2, 1, 4, -0.316569643860730e2),
    (2, 2, 4, 0.442040358308000e2),
    (2, 3, 4, -0.111282734326413e2),
    (2, 0, 5, -0.262480156590992e1),
    (2, 1, 5, 0.704658803315449e1),
    (2, 2, 5, -0.792001547211682e1),
];

/// Why a column was not returned.
#[derive(Clone, Debug, PartialEq)]
pub enum ColumnReject {
    /// An input or an intermediate value was NaN or infinite.
    NonFiniteValue,
    /// `latitude_deg` is outside [`LATITUDE_MIN_DEG`] to [`LATITUDE_MAX_DEG`].
    LatitudeOutOfRange { latitude_deg: f64 },
    /// The cast has fewer than [`LEVELS_MIN`] or more than [`LEVELS_MAX`] levels.
    LevelCountOutOfRange { levels: usize },
    /// The pressures, the temperatures and the salinities are not one for each level.
    LevelsDoNotMatch { pressures: usize, temperatures: usize, salinities: usize },
    /// A level's sea pressure is outside [`SEA_PRESSURE_MIN_DBAR`] to [`SEA_PRESSURE_MAX_DBAR`], the range of the Gibbs function.
    PressureOutOfRange { level: usize, sea_pressure_dbar: f64 },
    /// The first level is deeper than [`FIRST_LEVEL_MAX_DBAR`]: the layer from the sea surface down to it has no level above it to take a mean with, so it is weighed at the first level's specific volume alone, and the solver takes that one-level layer no thicker than this.
    FirstLevelTooDeep { sea_pressure_dbar: f64 },
    /// A level's pressure is not above the pressure of the level before it.
    PressureNotRising { level: usize },
    /// Two neighbouring levels are more than [`LEVEL_GAP_MAX_DBAR`] apart.
    LevelGapTooWide { level: usize, gap_dbar: f64 },
    /// A level's Practical Salinity is outside [`PRACTICAL_SALINITY_MIN`] to [`PRACTICAL_SALINITY_MAX`].
    SalinityOutOfRange { level: usize, practical_salinity: f64 },
    /// A level's temperature is above [`TEMPERATURE_MAX_C`] or below the lowest temperature of the water part of the Gibbs function, IAPWS SR7-09, at its pressure.
    TemperatureOutOfRange { level: usize, temperature_c: f64, lowest_c: f64 },
    /// A level's temperature is below the freezing temperature of air-free seawater of its salinity at its pressure, where the Gibbs function of seawater does not hold.
    TemperatureBelowFreezing { level: usize, temperature_c: f64, freezing_c: f64 },
    /// `height_above_bottom_m` is outside [`HEIGHT_ABOVE_BOTTOM_MIN_M`] to [`HEIGHT_ABOVE_BOTTOM_MAX_M`].
    HeightAboveBottomOutOfRange { height_above_bottom_m: f64 },
    /// The sweep cap was reached with a level still moving.
    ColumnNotSettled { sweeps: u32 },
    /// A layer of an otherwise settled column left [`HYDROSTATIC_TOLERANCE`] of its pressure step by more than the weight of [`DEPTH_ROUNDING_SPACINGS`] of the spacing of 64-bit numbers at its lower depth.
    HydrostaticBalanceBroken { residual: f64 },
}

impl ColumnReject {
    /// The variant name, without its fields.
    pub fn name(&self) -> &'static str {
        match self {
            Self::NonFiniteValue => "NonFiniteValue",
            Self::LatitudeOutOfRange { .. } => "LatitudeOutOfRange",
            Self::LevelCountOutOfRange { .. } => "LevelCountOutOfRange",
            Self::LevelsDoNotMatch { .. } => "LevelsDoNotMatch",
            Self::PressureOutOfRange { .. } => "PressureOutOfRange",
            Self::FirstLevelTooDeep { .. } => "FirstLevelTooDeep",
            Self::PressureNotRising { .. } => "PressureNotRising",
            Self::LevelGapTooWide { .. } => "LevelGapTooWide",
            Self::SalinityOutOfRange { .. } => "SalinityOutOfRange",
            Self::TemperatureOutOfRange { .. } => "TemperatureOutOfRange",
            Self::TemperatureBelowFreezing { .. } => "TemperatureBelowFreezing",
            Self::HeightAboveBottomOutOfRange { .. } => "HeightAboveBottomOutOfRange",
            Self::ColumnNotSettled { .. } => "ColumnNotSettled",
            Self::HydrostaticBalanceBroken { .. } => "HydrostaticBalanceBroken",
        }
    }
}

impl fmt::Display for ColumnReject {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NonFiniteValue => write!(f, "an input or a computed value was not finite"),
            Self::LatitudeOutOfRange { latitude_deg } => write!(
                f,
                "latitude {latitude_deg} deg is outside {LATITUDE_MIN_DEG} to {LATITUDE_MAX_DEG} deg"
            ),
            Self::LevelCountOutOfRange { levels } => {
                write!(f, "{levels} levels is outside {LEVELS_MIN} to {LEVELS_MAX} levels")
            }
            Self::LevelsDoNotMatch { pressures, temperatures, salinities } => write!(
                f,
                "{pressures} pressures, {temperatures} temperatures and {salinities} salinities are not one of each for every level"
            ),
            Self::PressureOutOfRange { level, sea_pressure_dbar } => write!(
                f,
                "sea pressure {sea_pressure_dbar} dbar at level {level} is outside {SEA_PRESSURE_MIN_DBAR} to {SEA_PRESSURE_MAX_DBAR} dbar, the range of the Gibbs function"
            ),
            Self::FirstLevelTooDeep { sea_pressure_dbar } => write!(
                f,
                "the first level is at {sea_pressure_dbar} dbar, deeper than {FIRST_LEVEL_MAX_DBAR} dbar: the layer from the sea surface down to it is weighed at the first level's water alone, and no thicker one is taken"
            ),
            Self::PressureNotRising { level } => write!(
                f,
                "the sea pressure at level {level} is not above the pressure of the level before it"
            ),
            Self::LevelGapTooWide { level, gap_dbar } => write!(
                f,
                "level {level} is {gap_dbar} dbar below the level before it, more than {LEVEL_GAP_MAX_DBAR} dbar"
            ),
            Self::SalinityOutOfRange { level, practical_salinity } => write!(
                f,
                "Practical Salinity {practical_salinity} at level {level} is outside {PRACTICAL_SALINITY_MIN} to {PRACTICAL_SALINITY_MAX}, the range of the Gibbs function"
            ),
            Self::TemperatureOutOfRange { level, temperature_c, lowest_c } => write!(
                f,
                "temperature {temperature_c} degC at level {level} is outside {lowest_c} to {TEMPERATURE_MAX_C} degC, the range of the Gibbs function at its pressure"
            ),
            Self::TemperatureBelowFreezing { level, temperature_c, freezing_c } => write!(
                f,
                "temperature {temperature_c} degC at level {level} is below {freezing_c} degC, the freezing temperature of seawater of its salinity at its pressure"
            ),
            Self::HeightAboveBottomOutOfRange { height_above_bottom_m } => write!(
                f,
                "height above bottom {height_above_bottom_m} m is outside {HEIGHT_ABOVE_BOTTOM_MIN_M} to {HEIGHT_ABOVE_BOTTOM_MAX_M} m"
            ),
            Self::ColumnNotSettled { sweeps } => {
                write!(f, "a level was still moving after {sweeps} sweeps")
            }
            Self::HydrostaticBalanceBroken { residual } => write!(
                f,
                "a layer's weight was off its pressure step by {residual} of the step"
            ),
        }
    }
}

impl std::error::Error for ColumnReject {}

/// One measured cast, and how far its deepest level was from the sea floor.
#[derive(Clone, Debug, PartialEq)]
pub struct Cast<'a> {
    pub latitude_deg: f64,
    /// Sea pressure of each level, decibars, rising.
    pub sea_pressure_dbar: &'a [f64],
    /// In-situ temperature of each level, degrees Celsius (ITS-90).
    pub temperature_c: &'a [f64],
    /// Practical Salinity of each level (PSS-78).
    pub practical_salinity: &'a [f64],
    /// Height of the deepest level above the sea floor, metres. Zero when the question stops at the deepest level.
    pub height_above_bottom_m: f64,
}

/// The settled column.
#[derive(Clone, Debug, PartialEq)]
pub struct SettledColumn {
    /// Depth of each level below the sea surface, metres.
    pub depth_m: Vec<f64>,
    /// Density of the water at each level, kg/m³.
    pub density_kg_per_m3: Vec<f64>,
    /// Sound speed at each level, m/s.
    pub sound_speed_m_per_s: Vec<f64>,
    /// Depth of the sea floor: the deepest level's depth and the height above bottom, metres.
    pub seabed_depth_m: f64,
    /// Time sound takes from the sea surface to the sea floor, seconds.
    pub echo_one_way_time_s: f64,
    /// The depth an echo sounder set to [`ECHO_SOUNDER_SOUND_SPEED_M_PER_S`] reads over this column, metres.
    pub echo_sounder_depth_m: f64,
    /// Sea-floor depth over one-way time, m/s.
    pub mean_sound_speed_m_per_s: f64,
    /// Largest imbalance of any layer, as a fraction of its pressure step.
    pub hydrostatic_residual: f64,
    /// The weight of all the layers less the deepest level's pressure, decibars.
    pub column_weight_residual_dbar: f64,
    /// Sweeps taken.
    pub sweeps: u32,
}

/// The four pressure and temperature derivatives of the Gibbs function the
/// column needs: `g_p` (m³/kg), `g_tt` (J/(kg K²)), `g_tp` (m³/(kg K)) and
/// `g_pp` (m³/(kg Pa)). Salinity in kg/kg, temperature in degrees Celsius,
/// absolute pressure in pascals.
fn gibbs_derivatives(
    salinity_kg_per_kg: f64,
    temperature_c: f64,
    absolute_pressure_pa: f64,
) -> [f64; 4] {
    let y = temperature_c / REDUCING_TEMPERATURE_K;
    let z = (absolute_pressure_pa - STANDARD_ATMOSPHERE_PA) / REDUCING_PRESSURE_PA;
    let x = (salinity_kg_per_kg / REDUCING_SALINITY_KG_PER_KG).sqrt();
    let mut y_to = [1.0; 8];
    let mut z_to = [1.0; 7];
    let mut x_to = [1.0; 8];
    for power in 1..8 {
        y_to[power] = y_to[power - 1] * y;
        x_to[power] = x_to[power - 1] * x;
    }
    for power in 1..7 {
        z_to[power] = z_to[power - 1] * z;
    }
    let (mut g_p, mut g_tt, mut g_tp, mut g_pp) = (0.0, 0.0, 0.0, 0.0);
    let mut add = |coefficient: f64, j: usize, k: usize| {
        let (jf, kf) = (j as f64, k as f64);
        if k >= 1 {
            g_p += coefficient * kf * y_to[j] * z_to[k - 1];
        }
        if j >= 2 {
            g_tt += coefficient * jf * (jf - 1.0) * y_to[j - 2] * z_to[k];
        }
        if j >= 1 && k >= 1 {
            g_tp += coefficient * jf * kf * y_to[j - 1] * z_to[k - 1];
        }
        if k >= 2 {
            g_pp += coefficient * kf * (kf - 1.0) * y_to[j] * z_to[k - 2];
        }
    };
    for (j, k, coefficient) in WATER {
        add(coefficient, j, k);
    }
    for (i, j, k, coefficient) in SALINE {
        // The two logarithmic terms, g_100 and g_110, are of the first power
        // in temperature and hold no pressure: none of the four derivatives
        // reaches them.
        if i == 1 {
            continue;
        }
        add(coefficient * x_to[i], j, k);
    }
    let (t, p) = (REDUCING_TEMPERATURE_K, REDUCING_PRESSURE_PA);
    [g_p / p, g_tt / (t * t), g_tp / (t * p), g_pp / (p * p)]
}

/// Specific volume (m³/kg) and sound speed (m/s) of seawater at one level.
fn specific_volume_and_sound_speed(
    practical_salinity: f64,
    temperature_c: f64,
    sea_pressure_dbar: f64,
) -> Result<(f64, f64), ColumnReject> {
    let salinity = practical_salinity * REFERENCE_SALINITY_G_PER_KG_PER_PRACTICAL * 1.0e-3;
    let pressure = sea_pressure_dbar * PA_PER_DBAR + STANDARD_ATMOSPHERE_PA;
    let [g_p, g_tt, g_tp, g_pp] = gibbs_derivatives(salinity, temperature_c, pressure);
    let sound_speed = g_p * (g_tt / (g_tp * g_tp - g_tt * g_pp)).sqrt();
    if !(g_p.is_finite() && g_p > 0.0 && sound_speed.is_finite() && sound_speed > 0.0) {
        return Err(ColumnReject::NonFiniteValue);
    }
    Ok((g_p, sound_speed))
}

/// Gravity at the sea surface at one latitude, m/s².
pub fn surface_gravity_m_per_s2(latitude_deg: f64) -> f64 {
    let sine = math::sin(latitude_deg.to_radians());
    let sin2 = sine * sine;
    GRAVITY_EQUATOR_M_PER_S2 * (1.0 + (GRAVITY_SIN2 + GRAVITY_SIN4 * sin2) * sin2)
}

/// The lowest temperature the Gibbs function takes at one sea pressure, degrees Celsius.
pub fn lowest_temperature_c(sea_pressure_dbar: f64) -> f64 {
    TEMPERATURE_MIN_AT_ZERO_PRESSURE_C
        - TEMPERATURE_MIN_FALL_C_PER_PA * (sea_pressure_dbar * PA_PER_DBAR + STANDARD_ATMOSPHERE_PA)
}

/// The chemical potential of water in seawater, `g − S ∂g/∂S` (J/kg), and
/// its temperature derivative (J/(kg K)). Salinity in kg/kg, temperature in
/// degrees Celsius, absolute pressure in pascals. The logarithmic terms of
/// the saline part leave `−g_1jk ξ²/2` here, and a term in `ξ^i` keeps
/// `1 − i/2` of itself.
fn water_chemical_potential(
    salinity_kg_per_kg: f64,
    temperature_c: f64,
    absolute_pressure_pa: f64,
) -> (f64, f64) {
    let y = temperature_c / REDUCING_TEMPERATURE_K;
    let z = (absolute_pressure_pa - STANDARD_ATMOSPHERE_PA) / REDUCING_PRESSURE_PA;
    let x = (salinity_kg_per_kg / REDUCING_SALINITY_KG_PER_KG).sqrt();
    let (mut potential, mut slope) = (0.0, 0.0);
    let mut add = |coefficient: f64, j: usize, k: usize| {
        let z_to_k = math::powi(z, k as i32);
        potential += coefficient * math::powi(y, j as i32) * z_to_k;
        if j >= 1 {
            slope += coefficient * j as f64 * math::powi(y, j as i32 - 1) * z_to_k / REDUCING_TEMPERATURE_K;
        }
    };
    for (j, k, coefficient) in WATER {
        add(coefficient, j, k);
    }
    for (i, j, k, coefficient) in SALINE {
        let share = if i == 1 {
            -0.5 * x * x
        } else {
            (1.0 - 0.5 * i as f64) * math::powi(x, i as i32)
        };
        add(coefficient * share, j, k);
    }
    (potential, slope)
}

/// A complex number as (real, imaginary).
type Complex = (f64, f64);

fn complex_mul(a: Complex, b: Complex) -> Complex {
    (a.0 * b.0 - a.1 * b.1, a.0 * b.1 + a.1 * b.0)
}

/// The principal value of the logarithm, as R10-06 asks.
fn complex_ln(a: Complex) -> Complex {
    (math::log(math::hypot(a.0, a.1)), math::atan2(a.1, a.0))
}

/// The Gibbs energy of ice Ih (J/kg) and its temperature derivative
/// (J/(kg K)), IAPWS R10-06 (2009) Eq. (1). Temperature in degrees Celsius,
/// absolute pressure in pascals.
fn ice_gibbs(temperature_c: f64, absolute_pressure_pa: f64) -> (f64, f64) {
    let tau = (temperature_c + 273.15) / ICE_TRIPLE_POINT_K;
    let pressure = (absolute_pressure_pa - STANDARD_ATMOSPHERE_PA) / ICE_TRIPLE_POINT_PA;
    let g0 = ICE_G0.iter().rev().fold(0.0, |sum, coefficient| sum * pressure + coefficient);
    let r2 = ICE_R2
        .iter()
        .rev()
        .fold((0.0, 0.0), |sum: Complex, coefficient| {
            (sum.0 * pressure + coefficient.0, sum.1 * pressure + coefficient.1)
        });
    let (mut energy, mut slope) = (0.0, 0.0);
    for (r, t) in [(ICE_R1, ICE_T1), (r2, ICE_T2)] {
        let below = (t.0 - tau, t.1);
        let above = (t.0 + tau, t.1);
        let (ln_below, ln_above, ln_t) = (complex_ln(below), complex_ln(above), complex_ln(t));
        let size = t.0 * t.0 + t.1 * t.1;
        let tau_squared_over_t = (tau * tau * t.0 / size, -tau * tau * t.1 / size);
        let two_tau_over_t = (2.0 * tau * t.0 / size, -2.0 * tau * t.1 / size);
        let lnt_twice = complex_mul((2.0, 0.0), complex_mul(t, ln_t));
        let f = {
            let a = complex_mul(below, ln_below);
            let b = complex_mul(above, ln_above);
            (
                a.0 + b.0 - lnt_twice.0 - tau_squared_over_t.0,
                a.1 + b.1 - lnt_twice.1 - tau_squared_over_t.1,
            )
        };
        let g = (
            ln_above.0 - ln_below.0 - two_tau_over_t.0,
            ln_above.1 - ln_below.1 - two_tau_over_t.1,
        );
        energy += complex_mul(r, f).0;
        slope += complex_mul(r, g).0;
    }
    (
        g0 - ICE_S0 * ICE_TRIPLE_POINT_K * tau + ICE_TRIPLE_POINT_K * energy,
        -ICE_S0 + slope,
    )
}

/// The freezing temperature of air-free seawater of one Practical Salinity
/// at one sea pressure, degrees Celsius: where the chemical potential of
/// water in seawater equals the Gibbs energy of ice, IAPWS R13-08 Eq. (32),
/// found by Newton steps until one moves it by no more than
/// [`FREEZING_TOLERANCE_K`].
pub fn freezing_temperature_c(practical_salinity: f64, sea_pressure_dbar: f64) -> Result<f64, ColumnReject> {
    let salinity = practical_salinity * REFERENCE_SALINITY_G_PER_KG_PER_PRACTICAL * 1.0e-3;
    let pressure = sea_pressure_dbar * PA_PER_DBAR + STANDARD_ATMOSPHERE_PA;
    let mut temperature = -0.0575 * practical_salinity - 7.53e-4 * sea_pressure_dbar;
    for _ in 0..FREEZING_MAX_STEPS {
        let (potential, potential_slope) = water_chemical_potential(salinity, temperature, pressure);
        let (ice, ice_slope) = ice_gibbs(temperature, pressure);
        let step = (potential - ice) / (potential_slope - ice_slope);
        if !step.is_finite() {
            return Err(ColumnReject::NonFiniteValue);
        }
        temperature -= step;
        if step.abs() <= FREEZING_TOLERANCE_K {
            return Ok(temperature);
        }
    }
    Err(ColumnReject::NonFiniteValue)
}

/// Every limit the cast itself shows, in the order the solver asks them:
/// the limits of the physics, level by level. The envelope of the casts
/// compared with measurement is not a limit.
pub fn check_cast(cast: &Cast<'_>) -> Result<(), ColumnReject> {
    check_physics(cast)
}

/// The limits of the physics: the Gibbs function, the freezing temperature,
/// and the shape of the cast.
fn check_physics(cast: &Cast<'_>) -> Result<(), ColumnReject> {
    let (pressures, temperatures, salinities) =
        (cast.sea_pressure_dbar, cast.temperature_c, cast.practical_salinity);
    let all_finite = cast.latitude_deg.is_finite()
        && cast.height_above_bottom_m.is_finite()
        && pressures.iter().chain(temperatures).chain(salinities).all(|value| value.is_finite());
    if !all_finite {
        return Err(ColumnReject::NonFiniteValue);
    }
    if !(LATITUDE_MIN_DEG..=LATITUDE_MAX_DEG).contains(&cast.latitude_deg) {
        return Err(ColumnReject::LatitudeOutOfRange { latitude_deg: cast.latitude_deg });
    }
    if !(HEIGHT_ABOVE_BOTTOM_MIN_M..=HEIGHT_ABOVE_BOTTOM_MAX_M).contains(&cast.height_above_bottom_m) {
        return Err(ColumnReject::HeightAboveBottomOutOfRange {
            height_above_bottom_m: cast.height_above_bottom_m,
        });
    }
    let levels = pressures.len();
    if temperatures.len() != levels || salinities.len() != levels {
        return Err(ColumnReject::LevelsDoNotMatch {
            pressures: levels,
            temperatures: temperatures.len(),
            salinities: salinities.len(),
        });
    }
    if !(LEVELS_MIN..=LEVELS_MAX).contains(&levels) {
        return Err(ColumnReject::LevelCountOutOfRange { levels });
    }
    for level in 0..levels {
        let sea_pressure_dbar = pressures[level];
        if !(SEA_PRESSURE_MIN_DBAR..=SEA_PRESSURE_MAX_DBAR).contains(&sea_pressure_dbar) {
            return Err(ColumnReject::PressureOutOfRange { level, sea_pressure_dbar });
        }
        if level == 0 {
            if sea_pressure_dbar > FIRST_LEVEL_MAX_DBAR {
                return Err(ColumnReject::FirstLevelTooDeep { sea_pressure_dbar });
            }
        } else {
            let gap_dbar = sea_pressure_dbar - pressures[level - 1];
            if !(gap_dbar > 0.0) {
                return Err(ColumnReject::PressureNotRising { level });
            }
            if gap_dbar > LEVEL_GAP_MAX_DBAR {
                return Err(ColumnReject::LevelGapTooWide { level, gap_dbar });
            }
        }
        let practical_salinity = salinities[level];
        if !(PRACTICAL_SALINITY_MIN..=PRACTICAL_SALINITY_MAX).contains(&practical_salinity) {
            return Err(ColumnReject::SalinityOutOfRange { level, practical_salinity });
        }
        let temperature_c = temperatures[level];
        let lowest_c = lowest_temperature_c(sea_pressure_dbar);
        if !(lowest_c..=TEMPERATURE_MAX_C).contains(&temperature_c) {
            return Err(ColumnReject::TemperatureOutOfRange { level, temperature_c, lowest_c });
        }
        let freezing_c = freezing_temperature_c(practical_salinity, sea_pressure_dbar)?;
        if temperature_c < freezing_c {
            return Err(ColumnReject::TemperatureBelowFreezing { level, temperature_c, freezing_c });
        }
    }
    Ok(())
}

/// The layers of one cast: layer `i` lies between level `i − 1` and level
/// `i`, and layer 0 between the sea surface and level 0.
struct Layers {
    surface_gravity: f64,
    /// Pressure step of each layer, pascals.
    pressure_step_pa: Vec<f64>,
    /// Mean specific volume of each layer, m³/kg.
    specific_volume: Vec<f64>,
}

impl Layers {
    /// Gravity at the middle of the layer between two depths, m/s².
    fn gravity_between(&self, top_m: f64, bottom_m: f64) -> f64 {
        self.surface_gravity * (1.0 + GRAVITY_RISE_PER_M * 0.5 * (top_m + bottom_m))
    }

    /// One sweep: every layer's thickness from the depths of the sweep
    /// before. Returns the largest move of any level.
    fn sweep(&self, depth_m: &mut [f64]) -> Result<f64, ColumnReject> {
        let mut moved: f64 = 0.0;
        let mut top_before = 0.0;
        let mut top_now = 0.0;
        for layer in 0..depth_m.len() {
            let bottom_before = depth_m[layer];
            let thickness = self.specific_volume[layer] * self.pressure_step_pa[layer]
                / self.gravity_between(top_before, bottom_before);
            let bottom_now = top_now + thickness;
            if !bottom_now.is_finite() {
                return Err(ColumnReject::NonFiniteValue);
            }
            moved = moved.max((bottom_now - bottom_before).abs());
            depth_m[layer] = bottom_now;
            top_before = bottom_before;
            top_now = bottom_now;
        }
        Ok(moved)
    }

    /// Holds a depth field to the balance. Returns the largest imbalance of
    /// any layer as a fraction of its pressure step, and the weight of the
    /// whole column less the deepest level's pressure, in decibars. A layer
    /// may be off by [`HYDROSTATIC_TOLERANCE`] of its step and the weight of
    /// [`DEPTH_ROUNDING_SPACINGS`] of the spacing of 64-bit numbers at its
    /// lower depth; the refusal carries the largest imbalance among the
    /// layers that are off by more.
    fn balance(&self, depth_m: &[f64]) -> Result<(f64, f64), ColumnReject> {
        let mut worst: f64 = 0.0;
        let mut worst_broken: Option<f64> = None;
        let mut weight_pa = 0.0;
        let mut pressure_pa = 0.0;
        let mut top = 0.0;
        for layer in 0..depth_m.len() {
            let bottom = depth_m[layer];
            let step = self.pressure_step_pa[layer];
            let weight_per_m = self.gravity_between(top, bottom) / self.specific_volume[layer];
            let weight = self.gravity_between(top, bottom) * (bottom - top) / self.specific_volume[layer];
            if !weight.is_finite() || !weight_per_m.is_finite() {
                return Err(ColumnReject::NonFiniteValue);
            }
            let allowed = HYDROSTATIC_TOLERANCE * step
                + weight_per_m.abs() * DEPTH_ROUNDING_SPACINGS * spacing_at(bottom);
            let off = if step > 0.0 {
                (weight - step).abs() / step
            } else if weight == 0.0 {
                0.0
            } else {
                f64::INFINITY
            };
            worst = worst.max(off);
            if !((weight - step).abs() <= allowed) {
                worst_broken = Some(worst_broken.map_or(off, |before| before.max(off)));
            }
            weight_pa += weight;
            pressure_pa += step;
            top = bottom;
        }
        if let Some(residual) = worst_broken {
            return Err(ColumnReject::HydrostaticBalanceBroken { residual });
        }
        Ok((worst, (weight_pa - pressure_pa) / PA_PER_DBAR))
    }
}

/// The spacing of 64-bit numbers at a depth: the next number above its size,
/// less that size. Zero for a depth that is not finite.
fn spacing_at(depth_m: f64) -> f64 {
    let size = depth_m.abs();
    if !size.is_finite() {
        return 0.0;
    }
    f64::from_bits(size.to_bits() + 1) - size
}

/// The settled column of one cast.
///
/// Each value of the answer below the normal range is written as zero
/// with its sign ([`crate::normal`]).
pub fn solve(cast: &Cast<'_>) -> Result<SettledColumn, ColumnReject> {
    crate::normal::held(solve_as_computed(cast))
}

/// The answer as the equations compute it, before a value below the
/// normal range is written as zero.
fn solve_as_computed(cast: &Cast<'_>) -> Result<SettledColumn, ColumnReject> {
    check_cast(cast)?;
    settle(cast, MAX_SWEEPS)
}

impl NormalOrZero for SettledColumn {
    fn normal_or_zero(&mut self) {
        self.depth_m.normal_or_zero();
        self.density_kg_per_m3.normal_or_zero();
        self.sound_speed_m_per_s.normal_or_zero();
        self.seabed_depth_m.normal_or_zero();
        self.echo_one_way_time_s.normal_or_zero();
        self.echo_sounder_depth_m.normal_or_zero();
        self.mean_sound_speed_m_per_s.normal_or_zero();
        self.hydrostatic_residual.normal_or_zero();
        self.column_weight_residual_dbar.normal_or_zero();
    }
}

/// The column of a cast with a sweep cap of the test's choosing.
#[cfg(test)]
fn solve_with(cast: &Cast<'_>, sweep_cap: u32) -> Result<SettledColumn, ColumnReject> {
    check_physics(cast)?;
    settle(cast, sweep_cap)
}

/// The column of a cast whose limits have been asked.
fn settle(cast: &Cast<'_>, sweep_cap: u32) -> Result<SettledColumn, ColumnReject> {
    let levels = cast.sea_pressure_dbar.len();
    let mut specific_volume = Vec::with_capacity(levels);
    let mut sound_speed = Vec::with_capacity(levels);
    for level in 0..levels {
        let (volume, speed) = specific_volume_and_sound_speed(
            cast.practical_salinity[level],
            cast.temperature_c[level],
            cast.sea_pressure_dbar[level],
        )?;
        specific_volume.push(volume);
        sound_speed.push(speed);
    }
    let mut layers = Layers {
        surface_gravity: surface_gravity_m_per_s2(cast.latitude_deg),
        pressure_step_pa: Vec::with_capacity(levels),
        specific_volume: Vec::with_capacity(levels),
    };
    let mut slowness = Vec::with_capacity(levels);
    for level in 0..levels {
        if level == 0 {
            layers.pressure_step_pa.push(cast.sea_pressure_dbar[0] * PA_PER_DBAR);
            layers.specific_volume.push(specific_volume[0]);
            slowness.push(1.0 / sound_speed[0]);
        } else {
            layers.pressure_step_pa.push(
                (cast.sea_pressure_dbar[level] - cast.sea_pressure_dbar[level - 1]) * PA_PER_DBAR,
            );
            layers.specific_volume.push(0.5 * (specific_volume[level] + specific_volume[level - 1]));
            slowness.push(0.5 * (1.0 / sound_speed[level] + 1.0 / sound_speed[level - 1]));
        }
    }
    let mut depth_m = vec![0.0; levels];
    let mut sweeps = 0;
    loop {
        if sweeps >= sweep_cap {
            return Err(ColumnReject::ColumnNotSettled { sweeps });
        }
        sweeps += 1;
        if layers.sweep(&mut depth_m)? <= SETTLE_TOLERANCE_M {
            break;
        }
    }
    let (hydrostatic_residual, column_weight_residual_dbar) = layers.balance(&depth_m)?;
    let mut time_s = 0.0;
    let mut top = 0.0;
    for level in 0..levels {
        time_s += (depth_m[level] - top) * slowness[level];
        top = depth_m[level];
    }
    let echo_one_way_time_s = time_s + cast.height_above_bottom_m / sound_speed[levels - 1];
    let seabed_depth_m = depth_m[levels - 1] + cast.height_above_bottom_m;
    let mean_sound_speed_m_per_s = seabed_depth_m / echo_one_way_time_s;
    if !(echo_one_way_time_s.is_finite() && mean_sound_speed_m_per_s.is_finite()) {
        return Err(ColumnReject::NonFiniteValue);
    }
    Ok(SettledColumn {
        density_kg_per_m3: specific_volume.iter().map(|volume| 1.0 / volume).collect(),
        sound_speed_m_per_s: sound_speed,
        seabed_depth_m,
        echo_one_way_time_s,
        echo_sounder_depth_m: ECHO_SOUNDER_SOUND_SPEED_M_PER_S * echo_one_way_time_s,
        mean_sound_speed_m_per_s,
        hydrostatic_residual,
        column_weight_residual_dbar,
        sweeps,
        depth_m,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A made-up cast shaped like an open-ocean one: warm and salty at the
    /// surface, cold below, a level every `step_dbar` down to `deepest_dbar`.
    fn made_up_cast(deepest_dbar: f64, step_dbar: f64) -> (Vec<f64>, Vec<f64>, Vec<f64>) {
        let mut pressure = Vec::new();
        let mut next = 0.0;
        while next < deepest_dbar {
            pressure.push(next);
            next += step_dbar;
        }
        pressure.push(deepest_dbar);
        let temperature = pressure.iter().map(|p| 1.2 + 23.0 * (-p / 400.0).exp() + 1.0e-4 * p).collect();
        let salinity = pressure.iter().map(|p| 34.7 + 0.6 * (-p / 300.0).exp()).collect();
        (pressure, temperature, salinity)
    }

    fn cast<'a>(latitude_deg: f64, columns: &'a (Vec<f64>, Vec<f64>, Vec<f64>), height: f64) -> Cast<'a> {
        Cast {
            latitude_deg,
            sea_pressure_dbar: &columns.0,
            temperature_c: &columns.1,
            practical_salinity: &columns.2,
            height_above_bottom_m: height,
        }
    }

    /// The column held to the limits of the physics alone, outside the
    /// compared envelope too: the equations are held to their references there.
    fn physics(cast: &Cast<'_>) -> Result<SettledColumn, ColumnReject> {
        solve_with(cast, MAX_SWEEPS)
    }

    fn close(got: f64, published: f64) -> f64 {
        ((got - published) / published).abs()
    }

    /// Check values of the water part: IAPWS SR7-09, Table 6
    /// (<https://www.teos-10.org/pubs/IAPWS-09.pdf>). Retrieved 2026-10-09.
    /// Each row: temperature in kelvin, absolute pressure in pascals, then
    /// `g_p`, `g_tt`, `g_tp`, `g_pp`, the density and the sound speed. The
    /// values are a published calculation, not a measurement.
    const WATER_CHECK: [(f64, f64, [f64; 6]); 3] = [
        (273.15, 101_325.0, [0.100015695e-2, -0.154472324e2, -0.677459513e-7, -0.508915308e-12, 0.999843071e3, 0.140240099e4]),
        (273.15, 1.0e8, [0.956683354e-3, -0.142970174e2, 0.199088060e-6, -0.371527164e-12, 0.104527793e4, 0.157543089e4]),
        (313.15, 101_325.0, [0.100784471e-2, -0.133463968e2, 0.388499694e-6, -0.445841077e-12, 0.992216354e3, 0.152891242e4]),
    ];

    /// Check values of the saline part alone: IAPWS R13-08, Table 8
    /// (<https://www.teos-10.org/pubs/IAPWS-08.pdf>). Retrieved 2026-10-09.
    /// Each row: salinity in kg/kg, temperature in kelvin, absolute pressure
    /// in pascals, then the saline part of `g_p`, `g_tt`, `g_tp`, `g_pp`.
    const SALINE_CHECK: [(f64, f64, f64, [f64; 4]); 3] = [
        (0.03516504, 273.15, 101_325.0, [-0.274957224e-4, 0.852861151, 0.119286787e-6, 0.581535172e-13]),
        (0.1, 353.0, 101_325.0, [-0.579227286e-4, 0.127922649e1, 0.803061596e-6, 0.213086154e-12]),
        (0.03516504, 273.15, 1.0e8, [-0.229123842e-4, 0.488076974, 0.466284412e-7, 0.357345736e-13]),
    ];

    #[test]
    fn iapws_published_values_of_the_water_part_and_the_saline_part() {
        println!("IAPWS releases retrieved {REFERENCE_RETRIEVED} from {REFERENCE_URL} and {REFERENCE_URL_SALINE}");
        let mut worst: f64 = 0.0;
        for (kelvin, pascal, published) in WATER_CHECK {
            let [g_p, g_tt, g_tp, g_pp] = gibbs_derivatives(0.0, kelvin - 273.15, pascal);
            let sound = g_p * (g_tt / (g_tp * g_tp - g_tt * g_pp)).sqrt();
            for (got, published) in [g_p, g_tt, g_tp, g_pp, 1.0 / g_p, sound].into_iter().zip(published) {
                let off = close(got, published);
                println!("water {kelvin} K {pascal} Pa: {got:e} against {published:e}, {off:.1e}");
                worst = worst.max(off);
                assert!(off <= REFERENCE_TOLERANCE, "{got} {published}");
            }
        }
        for (salinity, kelvin, pascal, published) in SALINE_CHECK {
            let with = gibbs_derivatives(salinity, kelvin - 273.15, pascal);
            let without = gibbs_derivatives(0.0, kelvin - 273.15, pascal);
            for index in 0..4 {
                let got = with[index] - without[index];
                let off = close(got, published[index]);
                println!("saline {salinity} {kelvin} K {pascal} Pa: {got:e} against {:e}, {off:.1e}", published[index]);
                worst = worst.max(off);
                assert!(off <= REFERENCE_TOLERANCE, "{got} {}", published[index]);
            }
        }
        println!("worst {worst:.2e}");
    }

    /// Seawater as a whole, IAPWS R13-08 Table 8: specific volume
    /// 0.972661217e-3 m³/kg and sound speed 1449.00246 m/s at Reference
    /// Salinity 35.16504 g/kg, 273.15 K and one atmosphere. That table takes
    /// its water part from IAPWS-95; this solver takes the water part of
    /// SR7-09, which SR7-09 states differs from IAPWS-95 by at most 0.23
    /// parts per million in density and 39 in sound speed.
    #[test]
    fn seawater_at_the_standard_point_is_within_the_stated_difference_of_the_two_water_parts() {
        let (volume, sound) = specific_volume_and_sound_speed(35.0, 0.0, 0.0).expect("in range");
        assert!(close(volume, 0.972661217e-3) < 0.23e-6, "{volume}");
        assert!(close(sound, 0.144900246e4) < 39.0e-6, "{sound}");
        assert!(sound > 0.144900246e4);
    }

    /// The depth of 10,000 dbar in an ocean of salinity 35 and 0 °C at
    /// latitude 30 is 9712.653 m by the earlier equation of state, EOS-80
    /// (Fofonoff and Millard, Unesco Technical Papers in Marine Science 44,
    /// 1983; the value as the open-source Python package `seawater` 3.3.5
    /// prints it, read 2026-10-09). The Gibbs function gives 0.13 m more.
    #[test]
    fn the_standard_ocean_is_within_a_quarter_metre_of_the_unesco_depth() {
        let pressure: Vec<f64> = (0..=200).map(|level| 50.0 * level as f64).collect();
        let columns = (pressure.clone(), vec![0.0; pressure.len()], vec![35.0; pressure.len()]);
        let column = physics(&cast(30.0, &columns, 0.0)).expect("settles");
        let deepest = *column.depth_m.last().expect("levels");
        println!("depth at 10000 dbar {deepest} m against 9712.653, sweeps {}", column.sweeps);
        assert!((deepest - 9712.653).abs() < 0.25, "{deepest}");
        assert!(deepest > 9712.653);
    }

    #[test]
    fn every_layer_carries_its_weight_and_the_column_carries_the_pressure() {
        let columns = made_up_cast(5000.0, 10.0);
        let column = physics(&cast(-32.5, &columns, 9.0)).expect("settles");
        assert!(column.hydrostatic_residual <= HYDROSTATIC_TOLERANCE);
        assert!(column.column_weight_residual_dbar.abs() < 1.0e-7);
        assert!(column.sweeps >= 3 && column.sweeps <= 8, "{}", column.sweeps);
        // The balance read back from the published numbers: density, depth and gravity.
        let gravity = surface_gravity_m_per_s2(-32.5);
        let mut weight_pa = 0.0;
        for level in 1..columns.0.len() {
            let thickness = column.depth_m[level] - column.depth_m[level - 1];
            let middle = 0.5 * (column.depth_m[level] + column.depth_m[level - 1]);
            let volume = 0.5 * (1.0 / column.density_kg_per_m3[level] + 1.0 / column.density_kg_per_m3[level - 1]);
            weight_pa += gravity * (1.0 + GRAVITY_RISE_PER_M * middle) * thickness / volume;
        }
        assert!((weight_pa / PA_PER_DBAR - 5000.0).abs() < 1.0e-6, "{weight_pa}");
        // Depth is less than the pressure in decibars would suggest, and V1's
        // one density and one gravity put the floor about 1.3 percent deeper.
        let deepest = column.depth_m[columns.0.len() - 1];
        assert!(deepest > 4890.0 && deepest < 4915.0, "{deepest}");
        let v1 = 5000.0 * PA_PER_DBAR / (1025.0 * 9.81);
        assert!(v1 - deepest > 55.0 && v1 - deepest < 80.0);
        assert_eq!(column.seabed_depth_m, deepest + 9.0);
        // Sound is faster than 1500 m/s on average here, so the sounder reads shallow.
        assert!(column.mean_sound_speed_m_per_s > 1500.0 && column.mean_sound_speed_m_per_s < 1530.0);
        assert!(column.echo_sounder_depth_m < column.seabed_depth_m);
        assert!(column.density_kg_per_m3.windows(2).all(|pair| pair[1] > pair[0]));
    }

    #[test]
    fn the_same_cast_gives_the_same_bits() {
        let columns = made_up_cast(3000.0, 10.0);
        let first = physics(&cast(10.0, &columns, 5.0)).expect("settles");
        let second = physics(&cast(10.0, &columns, 5.0)).expect("settles");
        assert_eq!(first.sweeps, second.sweeps);
        assert_eq!(first.echo_sounder_depth_m.to_bits(), second.echo_sounder_depth_m.to_bits());
        for (left, right) in first.depth_m.iter().zip(&second.depth_m) {
            assert_eq!(left.to_bits(), right.to_bits());
        }
    }

    /// Gravity is stronger at the poles, so the same water lies shallower there.
    #[test]
    fn the_same_water_is_shallower_at_the_pole_than_at_the_equator() {
        let columns = made_up_cast(4000.0, 20.0);
        let equator = physics(&cast(0.0, &columns, 0.0)).expect("settles");
        let pole = physics(&cast(90.0, &columns, 0.0)).expect("settles");
        let south = physics(&cast(-90.0, &columns, 0.0)).expect("settles");
        // The ratio of the two depths is the ratio of the two surface
        // gravities, less a few parts in a million: gravity also rises with
        // depth, and the deeper column gains more of that.
        let ratio = equator.seabed_depth_m / pole.seabed_depth_m;
        let gravities = surface_gravity_m_per_s2(90.0) / surface_gravity_m_per_s2(0.0);
        assert!(ratio < gravities && gravities - ratio < 5.0e-6, "{ratio} {gravities}");
        assert_eq!(pole.seabed_depth_m.to_bits(), south.seabed_depth_m.to_bits());
    }

    /// The levels are close enough: a cast thinned from every 2 dbar to every
    /// 100 dbar, the widest gap the solver takes, moves the sea floor by less
    /// than 0.2 m and the sounder depth by less than a metre.
    #[test]
    fn a_cast_thinned_to_the_widest_gap_moves_the_floor_by_less_than_a_fifth_of_a_metre() {
        let fine_columns = made_up_cast(2040.0, 2.0);
        let coarse_columns = made_up_cast(2040.0, LEVEL_GAP_MAX_DBAR);
        let fine = physics(&cast(20.0, &fine_columns, 0.0)).expect("settles");
        let coarse = physics(&cast(20.0, &coarse_columns, 0.0)).expect("settles");
        let floor = (fine.seabed_depth_m - coarse.seabed_depth_m).abs();
        let sounder = (fine.echo_sounder_depth_m - coarse.echo_sounder_depth_m).abs();
        println!("floor moves {floor} m, sounder {sounder} m");
        assert!(floor < 0.2, "{floor}");
        assert!(sounder < 1.0, "{sounder}");
    }

    #[test]
    fn a_cast_outside_the_declared_range_is_named() {
        let good = made_up_cast(1000.0, 50.0);
        let refused = |change: &dyn Fn(&mut (Vec<f64>, Vec<f64>, Vec<f64>)), latitude: f64, height: f64| {
            let mut columns = good.clone();
            change(&mut columns);
            solve(&cast(latitude, &columns, height)).expect_err("refused")
        };
        assert!(solve(&cast(0.0, &good, 0.0)).is_ok());
        assert_eq!(refused(&|_| {}, 90.5, 0.0), ColumnReject::LatitudeOutOfRange { latitude_deg: 90.5 });
        assert_eq!(refused(&|_| {}, f64::NAN, 0.0), ColumnReject::NonFiniteValue);
        assert_eq!(
            refused(&|_| {}, 0.0, 100.5),
            ColumnReject::HeightAboveBottomOutOfRange { height_above_bottom_m: 100.5 }
        );
        assert_eq!(
            refused(&|_| {}, 0.0, -1.0),
            ColumnReject::HeightAboveBottomOutOfRange { height_above_bottom_m: -1.0 }
        );
        assert_eq!(refused(&|c| c.1[3] = f64::INFINITY, 0.0, 0.0), ColumnReject::NonFiniteValue);
        assert_eq!(
            refused(&|c| { c.1.pop(); }, 0.0, 0.0),
            ColumnReject::LevelsDoNotMatch { pressures: 21, temperatures: 20, salinities: 21 }
        );
        assert_eq!(
            refused(&|c| { c.0.truncate(1); c.1.truncate(1); c.2.truncate(1); }, 0.0, 0.0),
            ColumnReject::LevelCountOutOfRange { levels: 1 }
        );
        assert_eq!(
            refused(&|c| *c = made_up_cast(2050.0, 2.0), 0.0, 0.0),
            ColumnReject::LevelCountOutOfRange { levels: 1026 }
        );
        assert_eq!(
            refused(&|c| c.0[0] = -0.5, 0.0, 0.0),
            ColumnReject::PressureOutOfRange { level: 0, sea_pressure_dbar: -0.5 }
        );
        assert_eq!(
            refused(&|c| c.0[0] = 20.5, 0.0, 0.0),
            ColumnReject::FirstLevelTooDeep { sea_pressure_dbar: 20.5 }
        );
        assert_eq!(refused(&|c| c.0[5] = c.0[4], 0.0, 0.0), ColumnReject::PressureNotRising { level: 5 });
        assert_eq!(
            refused(&|c| c.0[20] = 1050.5, 0.0, 0.0),
            ColumnReject::LevelGapTooWide { level: 20, gap_dbar: 100.5 }
        );
        assert_eq!(
            refused(&|c| c.2[2] = 41.9, 0.0, 0.0),
            ColumnReject::SalinityOutOfRange { level: 2, practical_salinity: 41.9 }
        );
        assert_eq!(
            refused(&|c| c.2[2] = -0.1, 0.0, 0.0),
            ColumnReject::SalinityOutOfRange { level: 2, practical_salinity: -0.1 }
        );
        assert!(matches!(
            refused(&|c| c.1[0] = 40.5, 0.0, 0.0),
            ColumnReject::TemperatureOutOfRange { level: 0, .. }
        ));
        // Below the water part's own floor, -2.6575 degC at the surface, the
        // name is the water part's.
        assert!(matches!(
            refused(&|c| c.1[0] = -3.0, 0.0, 0.0),
            ColumnReject::TemperatureOutOfRange { level: 0, .. }
        ));
        // Seawater freezes before that: -2 degC is ice at the surface and
        // liquid at 1000 dbar, where it is answered.
        match refused(&|c| c.1[0] = -2.0, 0.0, 0.0) {
            ColumnReject::TemperatureBelowFreezing { level: 0, temperature_c, freezing_c } => {
                assert_eq!(temperature_c, -2.0);
                assert!(freezing_c > -1.95 && freezing_c < -1.93, "{freezing_c}");
            }
            other => panic!("{other:?}"),
        }
        let mut cold = good.clone();
        cold.1[20] = -2.0;
        assert!(physics(&cast(0.0, &cold, 0.0)).is_ok());
        assert_eq!(solve(&cast(0.0, &cold, 0.0)), physics(&cast(0.0, &cold, 0.0)));
    }

    /// A request engine 0.6.0 answered with a proof: salinity 35 at -2.5 degC
    /// at 0 and 10 dbar, below the freezing temperature of seawater, about
    /// -1.92 degC. It is refused by name: ice, not seawater.
    #[test]
    fn seawater_below_its_freezing_temperature_is_named() {
        let columns = (vec![0.0, 10.0], vec![-2.5, -2.5], vec![35.0, 35.0]);
        for answer in [solve(&cast(0.0, &columns, 0.0)), physics(&cast(0.0, &columns, 0.0))] {
            match answer {
                Err(ColumnReject::TemperatureBelowFreezing { level: 0, temperature_c, freezing_c }) => {
                    assert_eq!(temperature_c, -2.5);
                    assert!(freezing_c > -1.93 && freezing_c < -1.91, "{freezing_c}");
                }
                other => panic!("{other:?}"),
            }
        }
        // three more below freezing: 35 at -2.65 and 41.8 at -2.6 at the
        // surface, 34.7 at -6.3 at 5000 dbar
        for (salinity, temperature, pressure) in [(35.0, -2.65, 0.0), (41.8, -2.6, 0.0), (34.7, -6.3, 5000.0)] {
            let pressures: Vec<f64> = if pressure > 0.0 {
                (0..=50).map(|level| pressure * level as f64 / 50.0).collect()
            } else {
                vec![0.0, 10.0]
            };
            let levels = pressures.len();
            let mut temperatures = vec![5.0; levels];
            temperatures[levels - 1] = temperature;
            let columns = (pressures, temperatures, vec![salinity; levels]);
            assert!(
                matches!(
                    physics(&cast(0.0, &columns, 0.0)),
                    Err(ColumnReject::TemperatureBelowFreezing { .. })
                ),
                "{salinity} {temperature} {pressure}"
            );
        }
        // at 10000 dbar seawater of 34.7 freezes below the water part's own
        // floor, -10.0875 degC, so that floor names -10.1 degC there
        let pressures: Vec<f64> = (0..=100).map(|level| 100.0 * level as f64).collect();
        let mut temperatures = vec![2.0; 101];
        temperatures[100] = -10.1;
        let columns = (pressures, temperatures, vec![34.7; 101]);
        assert!(matches!(
            physics(&cast(0.0, &columns, 0.0)),
            Err(ColumnReject::TemperatureOutOfRange { level: 100, .. })
        ));
        assert!(freezing_temperature_c(34.7, 10_000.0).unwrap() < lowest_temperature_c(10_000.0));
    }

    /// Check values of ice Ih: IAPWS R10-06 (2009), Table 6
    /// (<https://iapws.org/technical-guidance/release/Ice-2009>). Retrieved
    /// 2026-10-10. Each row: temperature in kelvin, absolute pressure in
    /// pascals, the Gibbs energy and its temperature derivative.
    const ICE_CHECK: [(f64, f64, f64, f64); 3] = [
        (273.16, 611.657, 0.611784135, 0.122069433940e4),
        (273.152519, 101_325.0, 0.10134274069e3, 0.122076932550e4),
        (100.0, 1.0e8, -0.222296513088e6, 0.261195122589e4),
    ];

    /// The chemical potential of water in seawater, the saline part alone:
    /// IAPWS R13-08, Table 8, the row µW. Each row: salinity in kg/kg,
    /// temperature in kelvin, absolute pressure in pascals, the value.
    const POTENTIAL_CHECK: [(f64, f64, f64, f64); 3] = [
        (0.03516504, 273.15, 101_325.0, -0.235181411e4),
        (0.1, 353.0, 101_325.0, -0.101085536e5),
        (0.03516504, 273.15, 1.0e8, -0.240897806e4),
    ];

    /// The Gibbs energy of the water part: IAPWS SR7-09, Table 6, the row g.
    const WATER_ENERGY_CHECK: [(f64, f64, f64); 3] = [
        (273.15, 101_325.0, 0.101342743e3),
        (273.15, 1.0e8, 0.977303868e5),
        (313.15, 101_325.0, -0.116198898e5),
    ];

    #[test]
    fn the_freezing_temperature_meets_the_published_values_of_ice_water_and_salt() {
        for (kelvin, pascal, energy, slope) in ICE_CHECK {
            let (got_energy, got_slope) = ice_gibbs(kelvin - 273.15, pascal);
            println!("ice {kelvin} K {pascal} Pa: g {got_energy:e} against {energy:e}, g_T {got_slope:e} against {slope:e}");
            assert!(close(got_energy, energy) <= REFERENCE_TOLERANCE, "{got_energy}");
            assert!(close(got_slope, slope) <= REFERENCE_TOLERANCE, "{got_slope}");
        }
        for (kelvin, pascal, energy) in WATER_ENERGY_CHECK {
            let (got, _) = water_chemical_potential(0.0, kelvin - 273.15, pascal);
            assert!(close(got, energy) <= REFERENCE_TOLERANCE, "{got} {energy}");
        }
        for (salinity, kelvin, pascal, potential) in POTENTIAL_CHECK {
            let (with, _) = water_chemical_potential(salinity, kelvin - 273.15, pascal);
            let (without, _) = water_chemical_potential(0.0, kelvin - 273.15, pascal);
            let got = with - without;
            println!("saline potential {salinity} {kelvin} K {pascal} Pa: {got:e} against {potential:e}");
            assert!(close(got, potential) <= REFERENCE_TOLERANCE, "{got} {potential}");
        }
        // The temperature derivative against a centred difference.
        let (_, slope) = water_chemical_potential(0.035, -1.9, 2.0e7);
        let step = 1.0e-4;
        let difference = (water_chemical_potential(0.035, -1.9 + step, 2.0e7).0
            - water_chemical_potential(0.035, -1.9 - step, 2.0e7).0)
            / (2.0 * step);
        assert!(close(slope, difference) < 1.0e-8, "{slope} {difference}");
        // Pure water melts at 273.152519 K at normal pressure, R10-06 Table 6.
        let pure = freezing_temperature_c(0.0, 0.0).unwrap();
        println!("pure water melts at {pure} degC");
        assert!((pure - 0.002519).abs() < 1.0e-6, "{pure}");
        // The polynomial the TEOS-10 toolbox starts its own search from
        // (GSW-C 3.05, gsw_t_freezing, saturation fraction 0), at Practical
        // Salinity 35 at 0 to 10000 dbar, Absolute Salinity 42 g/kg at 0 dbar,
        // 20 g/kg at 2000 dbar, and fresh water at 10000 dbar. Its fit is not exact; this solver's
        // temperatures are within 0.4 mK of it.
        for (practical, dbar, polynomial) in [
            (35.0, 0.0, -1.9191371999339475),
            (35.0, 1000.0, -2.6835946102037114),
            (35.0, 5000.0, -6.052187703016441),
            (35.0, 10_000.0, -10.950579499425931),
            (42.0 / REFERENCE_SALINITY_G_PER_KG_PER_PRACTICAL, 0.0, -2.3124380667248445),
            (20.0 / REFERENCE_SALINITY_G_PER_KG_PER_PRACTICAL, 2000.0, -2.6287944147248754),
            (0.0, 10_000.0, -8.951642762807687),
        ] {
            let got = freezing_temperature_c(practical, dbar).unwrap();
            println!("freezing {practical} {dbar} dbar: {got} against the polynomial {polynomial}, {:.1e}", got - polynomial);
            assert!((got - polynomial).abs() < 5.0e-4, "{practical} {dbar}: {got} {polynomial}");
        }
        // UNESCO 1983 (Millero): -1.9223 degC at salinity 35 and 0 dbar,
        // IPTS-68 and an older salinity scale, within a few millikelvin.
        let unesco = freezing_temperature_c(35.0, 0.0).unwrap();
        assert!((unesco + 1.9223).abs() < 5.0e-3, "{unesco}");
        // the same question gives the same bits
        assert_eq!(
            freezing_temperature_c(34.7, 4321.0).unwrap().to_bits(),
            freezing_temperature_c(34.7, 4321.0).unwrap().to_bits()
        );
    }

    /// Past the envelope of the casts compared with measurement, where
    /// engine 0.8.0 refused by name: every cast here is answered, the same
    /// bits as the column held to the physics alone, with every layer
    /// carrying its weight and the column carrying the last pressure, and
    /// the limits of the physics still stop it by name.
    #[test]
    fn a_cast_outside_the_compared_envelope_is_answered_to_the_limits_of_the_physics() {
        let next_up = |value: f64| f64::from_bits(if value >= 0.0 { value.to_bits() + 1 } else { value.to_bits() - 1 });
        let next_down = |value: f64| -next_up(-value);
        let held = |columns: &(Vec<f64>, Vec<f64>, Vec<f64>), latitude: f64, height: f64| -> SettledColumn {
            let column = solve(&cast(latitude, columns, height)).unwrap_or_else(|reject| panic!("{latitude} {height}: {reject}"));
            assert_eq!(Ok(column.clone()), physics(&cast(latitude, columns, height)));
            assert!(column.hydrostatic_residual <= 1.0e-9, "{}", column.hydrostatic_residual);
            assert!(column.column_weight_residual_dbar.abs() <= 1.0e-6, "{}", column.column_weight_residual_dbar);
            assert_eq!(check_cast(&cast(latitude, columns, height)), Ok(()));
            column
        };
        let base = made_up_cast(1000.0, 50.0);
        // latitude: every latitude, the poles included
        for latitude in [next_down(COMPARED_LATITUDE_MIN_DEG), next_up(COMPARED_LATITUDE_MAX_DEG), 61.0, -89.0, 90.0, -90.0] {
            held(&base, latitude, 0.0);
        }
        // height above bottom, to the 100 m of the method
        for height in [next_up(COMPARED_HEIGHT_ABOVE_BOTTOM_MAX_M), 100.0] {
            let column = held(&base, 0.0, height);
            assert_eq!(column.seabed_depth_m, column.depth_m[column.depth_m.len() - 1] + height);
        }
        // the last level, to the 10,000 dbar of the Gibbs function
        let trench = made_up_cast(10_000.0, 100.0);
        let deep = held(&trench, 11.3, 0.0);
        assert!(deep.seabed_depth_m > 9700.0 && deep.seabed_depth_m < 9760.0, "{}", deep.seabed_depth_m);
        // salinity, 0 to 41.8, and temperature, freezing to 40 degC
        for salinity in [next_down(COMPARED_SALINITY_MIN), next_up(COMPARED_SALINITY_MAX), 0.0, 41.8] {
            let mut columns = base.clone();
            columns.2[7] = salinity;
            held(&columns, 0.0, 0.0);
        }
        for temperature in [next_down(COMPARED_TEMPERATURE_MIN_C), next_up(COMPARED_TEMPERATURE_MAX_C), 39.9, 40.0] {
            let mut columns = base.clone();
            columns.1[7] = temperature;
            held(&columns, 0.0, 0.0);
        }
        // fresh water at 61 degN, a polar trench cast 100 m above the
        // floor, hot hypersaline water
        let fresh = (vec![0.0, 50.0, 100.0, 150.0, 200.0], vec![4.0; 5], vec![0.0; 5]);
        held(&fresh, 61.0, 10.0);
        let polar = ((0..=100).map(|level| 100.0 * level as f64).collect::<Vec<f64>>(), vec![1.5; 101], vec![34.7; 101]);
        held(&polar, -89.0, 100.0);
        let hot = (vec![0.0, 10.0, 20.0], vec![39.9; 3], vec![41.8; 3]);
        held(&hot, 27.0, 50.0);
        // past them the physics names the limit
        let mut hotter = hot.clone();
        hotter.1[1] = 40.5;
        assert!(matches!(solve(&cast(27.0, &hotter, 50.0)), Err(ColumnReject::TemperatureOutOfRange { level: 1, .. })));
        let mut saltier = hot.clone();
        saltier.2[2] = 41.9;
        assert_eq!(solve(&cast(27.0, &saltier, 50.0)), Err(ColumnReject::SalinityOutOfRange { level: 2, practical_salinity: 41.9 }));
        assert_eq!(solve(&cast(-90.5, &hot, 50.0)), Err(ColumnReject::LatitudeOutOfRange { latitude_deg: -90.5 }));
        assert_eq!(
            solve(&cast(0.0, &polar, 100.5)),
            Err(ColumnReject::HeightAboveBottomOutOfRange { height_above_bottom_m: 100.5 })
        );
    }

    /// The Arctic and the Southern Ocean, where engine 0.8.0 refused every
    /// cast by latitude and by temperature: cold casts down to the freezing
    /// temperature of their own water are answered with the column held,
    /// and the next 64-bit number colder than freezing at any level is
    /// refused as ice.
    #[test]
    fn polar_water_is_answered_down_to_its_freezing_temperature() {
        // Arctic: the Canada Basin, fresh cold surface water over Atlantic water.
        let arctic_pressure: Vec<f64> = (0..=38).map(|level| 100.0 * level as f64).collect();
        let arctic_salinity: Vec<f64> = arctic_pressure.iter().map(|p| 34.95 - 6.0 * (-p / 60.0).exp()).collect();
        let arctic_temperature: Vec<f64> = arctic_pressure
            .iter()
            .zip(&arctic_salinity)
            .enumerate()
            .map(|(level, (p, salinity))| if level == 0 { freezing_temperature_c(*salinity, *p).unwrap() } else { -0.3 + 0.8 * (-(p - 400.0).powi(2) / 1.0e5).exp() })
            .collect();
        // Southern Ocean: the Weddell Sea, water at freezing at the surface and
        // Antarctic Bottom Water near -0.8 degC at 4800 dbar.
        let weddell_pressure: Vec<f64> = (0..=48).map(|level| 100.0 * level as f64).collect();
        let weddell_salinity = vec![34.66; weddell_pressure.len()];
        let weddell_temperature: Vec<f64> = weddell_pressure
            .iter()
            .enumerate()
            .map(|(level, p)| if level == 0 { freezing_temperature_c(34.66, 0.0).unwrap() } else { -0.8 + 1.2 * (-p / 800.0).exp() })
            .collect();
        for (latitude, columns) in [
            (84.5, (arctic_pressure, arctic_temperature, arctic_salinity)),
            (-68.0, (weddell_pressure, weddell_temperature, weddell_salinity)),
        ] {
            let column = solve(&cast(latitude, &columns, 20.0)).unwrap_or_else(|reject| panic!("{latitude}: {reject}"));
            assert!(column.hydrostatic_residual <= 1.0e-9);
            assert!(column.column_weight_residual_dbar.abs() <= 1.0e-6);
            // the surface level sits exactly at freezing and is liquid
            assert!(columns.1[0] < -1.5 && columns.1[0] > -2.0, "{}", columns.1[0]);
            // a hair colder is ice
            let mut colder = columns.clone();
            colder.1[0] = f64::from_bits(colder.1[0].to_bits() + 1);
            assert!(matches!(
                solve(&cast(latitude, &colder, 20.0)),
                Err(ColumnReject::TemperatureBelowFreezing { level: 0, .. })
            ));
            // the gravity of the high latitude makes the same water shallower
            // than at the equator
            let equator = solve(&cast(0.0, &columns, 20.0)).expect("settles");
            assert!(column.seabed_depth_m < equator.seabed_depth_m);
        }
    }

    /// The Gibbs function ends at 10,000 dbar. The deepest trenches are
    /// deeper: a cast into the Challenger Deep is refused by name.
    #[test]
    fn a_cast_below_the_limit_of_the_gibbs_function_is_named() {
        let to_the_limit = made_up_cast(10_000.0, 100.0);
        let column = physics(&cast(11.3, &to_the_limit, 0.0)).expect("the limit is inside");
        assert!(column.seabed_depth_m > 9700.0 && column.seabed_depth_m < 9760.0);
        let mut beyond = to_the_limit.clone();
        beyond.0.push(10_050.0);
        beyond.1.push(2.4);
        beyond.2.push(34.7);
        assert_eq!(
            physics(&cast(11.3, &beyond, 0.0)),
            Err(ColumnReject::PressureOutOfRange { level: 101, sea_pressure_dbar: 10_050.0 })
        );
        assert_eq!(
            solve(&cast(11.3, &beyond, 0.0)),
            Err(ColumnReject::PressureOutOfRange { level: 101, sea_pressure_dbar: 10_050.0 })
        );
        assert_eq!(solve(&cast(11.3, &to_the_limit, 0.0)), Ok(column));
    }

    /// Attack: a column stopped before it has settled is named, not returned.
    #[test]
    fn a_column_stopped_early_is_named() {
        let columns = made_up_cast(5000.0, 10.0);
        for cap in [0, 1, 2] {
            assert_eq!(
                solve_with(&cast(0.0, &columns, 0.0), cap),
                Err(ColumnReject::ColumnNotSettled { sweeps: cap })
            );
        }
    }

    fn layers_of(columns: &(Vec<f64>, Vec<f64>, Vec<f64>), latitude_deg: f64) -> Layers {
        let mut layers = Layers {
            surface_gravity: surface_gravity_m_per_s2(latitude_deg),
            pressure_step_pa: Vec::new(),
            specific_volume: Vec::new(),
        };
        let mut before = None;
        for level in 0..columns.0.len() {
            let (volume, _) =
                specific_volume_and_sound_speed(columns.2[level], columns.1[level], columns.0[level]).expect("in range");
            match before {
                None => {
                    layers.pressure_step_pa.push(columns.0[0] * PA_PER_DBAR);
                    layers.specific_volume.push(volume);
                }
                Some(last) => {
                    layers.pressure_step_pa.push((columns.0[level] - columns.0[level - 1]) * PA_PER_DBAR);
                    layers.specific_volume.push(0.5 * (volume + last));
                }
            }
            before = Some(volume);
        }
        layers
    }

    /// The casts the second scorer found refused (V-29), and closer ones:
    /// forty levels 0.02 dbar apart at 3,000 and 6,000 dbar, 0.05 dbar apart
    /// at 9,900 dbar, 0.001 dbar apart at 1,000 dbar, and levels one 64-bit
    /// number apart at each depth. Every one is answered, with every layer
    /// inside its step's tolerance and the rounding of its depth.
    #[test]
    fn levels_close_together_at_depth_are_answered() {
        let mut answered = 0;
        let mut beyond_the_first_part = 0;
        for (deep_dbar, gap_dbar) in [
            (3000.0, 0.02),
            (6000.0, 0.02),
            (9900.0, 0.05),
            (1000.0, 0.001),
            (9990.0, 0.001),
            (6000.0, 1.0e-6),
            (9990.0, 1.0e-9),
            (3000.0, 0.0),
            (6000.0, 0.0),
            (9990.0, 0.0),
        ] {
            // a coarse cast down to the depth, then forty close levels
            let (mut pressures, mut temperatures, mut salinities) = made_up_cast(deep_dbar, 50.0);
            let (t, s) = (*temperatures.last().unwrap(), *salinities.last().unwrap());
            let mut pressure = *pressures.last().unwrap();
            for _ in 0..40 {
                pressure = if gap_dbar > 0.0 {
                    pressure + gap_dbar
                } else {
                    f64::from_bits(pressure.to_bits() + 1)
                };
                pressures.push(pressure);
                temperatures.push(t);
                salinities.push(s);
            }
            let columns = (pressures, temperatures, salinities);
            let column = physics(&cast(-30.0, &columns, 10.0))
                .unwrap_or_else(|reject| panic!("{deep_dbar} dbar, {gap_dbar} apart: {reject}"));
            answered += 1;
            if column.hydrostatic_residual > HYDROSTATIC_TOLERANCE {
                beyond_the_first_part += 1;
            }
            // every level at or below the one above it, and the weight of the column its pressure
            assert!(column.depth_m.windows(2).all(|pair| pair[1] >= pair[0]));
            assert!(column.column_weight_residual_dbar.abs() < 1.0e-9);
        }
        assert_eq!(answered, 10);
        // the casts that were refused are among them
        assert!(beyond_the_first_part >= 4, "{beyond_the_first_part}");
    }

    /// The rounding allowed for is half a spacing of the depth and no more:
    /// in a thin layer at depth, a level moved by one spacing is taken as
    /// rounding only where the layer's own tolerance covers it, and a level
    /// moved by four spacings is named.
    #[test]
    fn a_thin_layer_moved_by_more_than_its_rounding_is_named() {
        assert_eq!(DEPTH_ROUNDING_SPACINGS, 0.5);
        assert_eq!(spacing_at(6000.0), 9.094947017729282e-13);
        assert_eq!(spacing_at(-6000.0), spacing_at(6000.0));
        assert_eq!(spacing_at(f64::INFINITY), 0.0);
        let (mut pressures, mut temperatures, mut salinities) = made_up_cast(6000.0, 50.0);
        let (t, s) = (*temperatures.last().unwrap(), *salinities.last().unwrap());
        for index in 1..=40 {
            pressures.push(6000.0 + 0.02 * f64::from(index));
            temperatures.push(t);
            salinities.push(s);
        }
        let columns = (pressures, temperatures, salinities);
        let layers = layers_of(&columns, -30.0);
        let settled = physics(&cast(-30.0, &columns, 0.0)).expect("settles").depth_m;
        assert!(layers.balance(&settled).is_ok());
        let last = settled.len() - 10;
        let mut moved = settled.clone();
        moved[last] += 4.0 * spacing_at(settled[last]);
        match layers.balance(&moved) {
            Err(ColumnReject::HydrostaticBalanceBroken { residual }) => {
                assert!(residual > 1.0e-10 && residual < 1.0e-9, "{residual}");
            }
            other => panic!("four spacings were taken: {other:?}"),
        }
    }

    /// Attack: a falsified column. The settled depths stretched by one part
    /// in a thousand million, one level moved by a tenth of a millimetre, the
    /// depths of V1's one density and one gravity, the depths after a single
    /// sweep, and a depth that is not a number are each named.
    #[test]
    fn a_falsified_column_is_named() {
        let columns = made_up_cast(5000.0, 10.0);
        let layers = layers_of(&columns, 30.0);
        let settled = physics(&cast(30.0, &columns, 0.0)).expect("settles").depth_m;
        assert!(layers.balance(&settled).is_ok());

        let stretched: Vec<f64> = settled.iter().map(|depth| depth * (1.0 + 1.0e-9)).collect();
        assert!(matches!(
            layers.balance(&stretched),
            Err(ColumnReject::HydrostaticBalanceBroken { .. })
        ));

        let mut moved = settled.clone();
        moved[250] += 1.0e-4;
        assert!(matches!(layers.balance(&moved), Err(ColumnReject::HydrostaticBalanceBroken { .. })));

        let v1: Vec<f64> = columns.0.iter().map(|p| p * PA_PER_DBAR / (1025.0 * 9.81)).collect();
        match layers.balance(&v1) {
            Err(ColumnReject::HydrostaticBalanceBroken { residual }) => assert!(residual > 1.0e-3, "{residual}"),
            other => panic!("V1's depths were accepted: {other:?}"),
        }

        let mut once = vec![0.0; settled.len()];
        layers.sweep(&mut once).expect("finite");
        assert!(matches!(layers.balance(&once), Err(ColumnReject::HydrostaticBalanceBroken { .. })));

        let mut broken = settled.clone();
        broken[10] = f64::NAN;
        assert_eq!(layers.balance(&broken), Err(ColumnReject::NonFiniteValue));
    }

    /// The first level may sit at the surface or a little below it. The
    /// water above it is counted with its own density.
    #[test]
    fn the_water_above_the_first_level_is_counted() {
        let mut columns = made_up_cast(1000.0, 10.0);
        let from_surface = physics(&cast(0.0, &columns, 0.0)).expect("settles");
        columns.0.remove(0);
        columns.1.remove(0);
        columns.2.remove(0);
        let from_ten = physics(&cast(0.0, &columns, 0.0)).expect("settles");
        assert!((from_surface.seabed_depth_m - from_ten.seabed_depth_m).abs() < 0.01);
        assert!(from_ten.depth_m[0] > 9.9 && from_ten.depth_m[0] < 10.0);
        assert_eq!(from_surface.depth_m[0], 0.0);
    }
}
