#![no_implicit_prelude]

//! The orbit of the laser-ranging satellite LAGEOS-1 in free flight.
//!
//! LAGEOS-1 (COSPAR 7603901) is a passive sphere of 406.965 kg, 60 cm across,
//! carrying 426 retroreflectors, launched 4 May 1976. It flies a near-circular
//! orbit of radius 12 223 to 12 319 km inclined 109.8°. Source: the ILRS
//! mission page, saved in the research folder as
//! `lageos1_orbit/raw/reference/ilrs/lag1_general.html`.
//!
//! The card will be compared with the weekly orbit files of the International
//! Laser Ranging Service, product doi 10.5067/SLR/SLR_ILRSORB_001. The ILRS
//! asks for this citation: Pearlman, Noll, Pavlis et al. (2019), "The ILRS:
//! approaching 20 years and planning for the future", J Geod 93, 2161–2180,
//! doi 10.1007/s00190-019-01241-1.
//!
//! Six numbers move through time: position and velocity. Nothing is fitted.
//! The card works in a frame that turns about its own z axis at one locked
//! rate, the rate of the Earth Rotation Angle. The z axis is the Earth's
//! rotation pole as it stood at the start of the run and it does not move. In
//! that frame the Earth's field stands still. The card holds no table of Earth
//! orientation and no ephemeris.
//!
//! The forces, all with locked constants:
//!
//! - the Earth's field to degree and order 12: 165 numbers;
//! - the Sun and the Moon, placed from a short series by the epoch alone, to
//!   about a tenth of a degree;
//! - the tide of degree 2 those two raise in the solid Earth;
//! - sunlight pressure on a sphere, switched off in the Earth's shadow;
//! - the relativistic term of a single central mass.
//!
//! The card has no drag, no thrust, no light reflected from the Earth, no
//! heat-driven forces, no ocean tide and no frequency-dependent part of the
//! solid tide. Its Sun and Moon are approximate.
//!
//! This structure was chosen after scores on two opened weeks of the orbit
//! data had been seen: a ladder of structures fixed in advance failed the pass
//! line on those weeks, and the tide was added afterwards. The sealed weeks
//! have had no model scored on them.
//!
//! This is a LAGEOS-1 card. The mass and the diameter are that satellite's and
//! enter only through sunlight pressure. The card was also run on LAGEOS-2.
//! Using the card for the second satellite is a second-tier test, with a mass
//! that differs by 0.4 percent.
//!
//! # What was compared, and the card's range
//!
//! The card was compared with the published orbits of two satellites on 28
//! calendar days, 2026-08-23 to 2026-09-19: four weeks of LAGEOS-1 (two of
//! them opened while the card was designed) and four of LAGEOS-2, 56 runs of
//! 10 s steps, each started on one day of a week and run for at most seven
//! days, to 2026-09-20 00:00 UTC at the latest. The card serves only that: the
//! two orbits, as bands of semi-major axis, eccentricity and inclination
//! around each that hold every start and every landing of those runs; the
//! radii those runs reached; those dates; and a forecast of at most seven days
//! from a state taken from a published orbit, counted over every run it is
//! restarted through. Outside them the card refuses by name:
//! [`StepReject::RadiusOutsideCard`], [`StepReject::OrbitOutsideCard`],
//! [`StepReject::EpochOutsideCard`] and [`StepReject::HorizonOutsideCard`].
//! The step compared was 10 s.
//!
//! # The start state
//!
//! The published orbit files are in a frame fixed to the Earth's crust, which
//! does not turn uniformly. A state must be turned into the card's frame, the
//! true pole of the start epoch, before it is given to the card. That is done
//! outside the engine. A crust-fixed state fed straight in is 21 to 22 m off
//! within one revolution for that reason alone. `epoch_utc_s` in a request is
//! UTC at the start of the run in seconds after 2000-01-01 12:00 UTC. The
//! `epoch_utc_s` of a returned state is the UTC of that state: the start epoch
//! plus `t_s`, worked out from the two at each state with one rounding and not
//! kept as a sum that gains a step at a time. `t_s` counts from the start of
//! the run. A final state handed back as a new start, with its `epoch_utc_s`,
//! `position_m` and `velocity_m_s`, continues the same orbit, and the new run's
//! `t_s` begins again at zero. `forecast_elapsed_s` in a request is the time
//! since the state was taken from a published orbit: zero for a state turned
//! from a published orbit, and the `forecast_elapsed_s` of a returned state for
//! a state handed back. It is required. The card holds it, plus `t_s`, to seven
//! days, so a forecast carried on through restarts stops where a run straight
//! through stops. The card cannot check the number a caller states: a state
//! stated as fresh that is not gets answers the comparison does not cover.
//!
//! # Where the locked numbers come from
//!
//! Paths are under `lageos1_orbit/` in the research folder. `raw/MANIFEST.csv`
//! there holds the SHA-256 of every saved source.
//!
//! - The field: EGM2008, tide-free, fully normalised, from
//!   `raw/reference/gravity/EGM2008.gfc.first_40000_bytes`, SHA-256
//!   `db25796bebbf66e67e44571219b5bcbb55b164ef02cbd7d7efc47e304a0f52ff`. The
//!   file header gives the gravitational parameter and the radius. Three
//!   numbers are not from that file: degree 2 order 0 is the zero-tide value,
//!   and degrees 3 and 4 order 0 the values at 2000.0, of IERS Conventions
//!   (2010) Table 6.2. Zero-tide because the card's tide has its permanent
//!   part removed. The tabulated yearly rates are not applied. The table as
//!   88 rows of text is `analysis/out/card_gravity_coefficients_degree_12.csv`,
//!   SHA-256
//!   `3ffe678122ac51affc23a49ed6424ed758227bf6ac041760a63d11614e5012cb`.
//! - The Sun and Moon series: Montenbruck and Gill, "Satellite Orbits",
//!   section 3.3.2, as transcribed in two source files of the Orekit library
//!   (Apache 2.0). The book was not opened. The files are
//!   `raw/reference/other/orekit_AnalyticalSolarPositionProvider.java`,
//!   SHA-256
//!   `1e884067f68e0286a3c3b83c5efca2c39e1a0332da00359338c8ee99c31c9c38`, and
//!   `raw/reference/other/orekit_AnalyticalLunarPositionProvider.java`,
//!   SHA-256
//!   `ff2d8a39dbd9b1f75cfa439339b4a6ab7cb409483aaef977b0e11d0ce6a915a3`.
//! - Both tables were written into this file as text by
//!   `analysis/08_card_tables_and_reference.py`. Its output is
//!   `analysis/out/card_satellite_lageos1_tables.rs.txt`, SHA-256
//!   `b5c235ffb485c0577e190aae4e3620483c762bd198beefc410be2606eb9634c8`. A
//!   test hashes the text between the two marker lines below and compares,
//!   and checks each field literal against the 88 rows, text for text.
//! - IERS Conventions (2010), chapters 1, 5, 6 and 10, saved under
//!   `raw/reference/iers/`: the rotation rate, the Earth Rotation Angle, the
//!   sidereal-time term, the tide numbers, the speed of light, the
//!   astronomical unit, the relativistic term.
//! - JPL Horizons data sheets, saved under `raw/reference/jpl_horizons/`: the
//!   gravitational parameters of the Sun and the Moon, the solar constant.
//! - Analysis-centre notes `raw/edc_notes/README_AC.bkg` and `README_AC.esa`:
//!   the radiation coefficient.
//!
//! # The model
//!
//! ```text
//! r = (x, y, z), v      position and velocity in the turning frame
//! ω                     EARTH_ROTATION_RATE_RAD_S
//!
//! d r/dt = v
//! d v/dt = g(r)                                    the locked field, point mass included
//!        + (2 ω v_y + ω² x, −2 ω v_x + ω² y, 0)    Coriolis and centrifugal
//!        + a(t, r, v)                              everything else
//! d W/dt = a · v                                   work of everything else, per kilogram
//!
//! a = GM_b [ (s − r)/|s − r|³ − s/|s|³ ]           for the Sun and for the Moon at s
//!   + the field of five changes to the degree-2 coefficients raised by those two bodies
//!   + lit (S/c) (AU/|r − s_sun|)² C_R (π d²/4 ÷ m) (r − s_sun)/|r − s_sun|
//!   + GM/(c² |r|³) [ (4 GM/|r| − u·u) r + 4 (r·u) u ],   u = v + (−ω y, ω x, 0)
//!
//! J = ½ v·v − U(r) − ½ ω² (x² + y²)                U the potential of the locked field
//! defect = J − J_anchor − W
//! residual = |defect| / LEDGER_SCALE_J_PER_KG
//! ```
//!
//! The field is evaluated in Cartesian form by a recursion on solid harmonics
//! (Cunningham's), which needs no latitude, no longitude and no angle. It is
//! regular on the axis of rotation.
//!
//! The Sun and Moon positions at time `epoch_utc_s + t`:
//!
//! ```text
//! T = (epoch_utc_s + t + 69.184) / (36525 × 86400)      Julian centuries of TT from 2000.0
//! Sun:  M = 357.5256° + 35999.049° T
//!       distance = (149.619 − 2.499 cos M − 0.021 cos 2M) × 10⁹ m
//!       longitude = 282.9400° + M + 6892″ sin M + 72″ sin 2M + 1.3972° T,  latitude 0
//! Moon: five angles, and the 8 distance terms, 14 longitude terms and 8
//!       latitude terms of the locked series
//! each: turned about x by 23.43929111°, then about z by
//!       2π (frac(d) + 0.7790572732640 + 0.00273781191135448 d) + 4612.156534″ T,
//!       d = (epoch_utc_s + t) / 86400
//! ```
//!
//! Sunlight is off when the satellite is behind the Earth as seen from the
//! Sun and within `FIELD_RADIUS_M` of the line from the Sun through the Earth.
//! The opened weeks had no shadow passage, so that switch has not been
//! compared with data. A landing in the shadow sets [`FLAG_EARTH_SHADOW`] and
//! lands.
//!
//! # The step
//!
//! One step is classical RK4 on seven numbers, `r`, `v` and `W`, with the Sun
//! and the Moon placed again at each stage time. [`SatelliteLageos1::gate_dt_s`]
//! is 10 s. Measured against an 8th-order reference from the worked example
//! of engine 0.6.0, a circular polar orbit that the card now refuses, the
//! step error is 11.2 mm after one day and 13.9 cm after seven days, and
//! 0.4 mm to 21 mm after one day from four other start states inside the
//! 0.6.0 bands, none of them on a date and an orbit the card now serves (one
//! recomputation outside this crate, 2026-10-07, not confirmed by a second). [`SatelliteLageos1::floor_dt_s`] is that divided by
//! 2^20, 9.5367431640625e-6 s. The public step applies neither.
//!
//! The radius band, the two orbit bands, the dates and the seven-day forecast
//! are applied to the incoming state and to the landing. Nothing is pulled
//! back to a bound.
//!
//! The arithmetic is `+ − × ÷`, `sqrt`, `sin` and `cos`. The sines and cosines
//! are in the Sun and Moon series and in the two turns that follow them. There
//! is no arctangent, no arcsine, no exponential and no logarithm.
//!
//! # The ledger
//!
//! `J` is the energy per kilogram as seen from the turning frame: kinetic
//! energy, less the potential of the locked field, less the centrifugal term.
//! In a field that stands still in that frame `J` changes only by the work of
//! the other forces. The step integrates that work beside the state and the
//! ledger compares.
//!
//! What it detects: a step whose position and velocity do not follow from the
//! locked field, the centrifugal term and the forces whose work was booked. A
//! first-order or second-order method, a step several times the ceiling, a
//! centrifugal term with the wrong sign, and work that was booked falsely are
//! each refused by name.
//!
//! What it cannot detect:
//!
//! - A Coriolis term with the wrong sign. That force is at right angles to the
//!   velocity and does no work, so the ledger stays clean while the satellite
//!   goes 25 000 km astray in a day. A comparison of positions catches it at
//!   the first step, 65 m off after 10 s, and this file's tests carry one.
//!   The card's orbit bands also stop such a run, at the first step from the
//!   test state, as a range refusal. The ledger itself never objects.
//! - A force that is wrong and booked consistently: a Sun in the wrong place,
//!   a wrong tide number, a wrong mass. The work of the wrong force is booked
//!   and the ledger balances.
//! - A wrong field coefficient used by both the step and the ledger.
//! - How accurate the step is. RK4 is not exact here; the residual is small
//!   because the step is small. Accuracy rests on the ceiling and on the
//!   position tests.
//!
//! # The frame
//!
//! A 32-bit number near 12 270 km is spaced 1 m apart, which is the pass line
//! of the comparison at one revolution. So the six 32-bit slots carry the
//! position twice over and the velocity is not in the frame:
//!
//! ```text
//! t             t_s, 64-bit
//! pos[0..3]     x, y, z, each rounded to the nearest 32-bit number, metres
//! vel[0..3]     x, y, z less the number in pos, metres: at most 0.5 m in size
//! force_torque  radius_m − REFERENCE_RADIUS_M, metres
//! residual      residual_conservation_error
//! flags         FLAG_EARTH_SHADOW or 0
//! ```
//!
//! A reader adds `pos[i]` and `vel[i]` as 64-bit numbers and has each
//! coordinate of the position to better than 3 × 10⁻⁸ m. The velocity is in
//! the state.
//!
//! # The last date and rounding
//!
//! The clock of a run is a running sum, `t += dt` in 64-bit arithmetic, and
//! each addition rounds by at most half the spacing of 64-bit numbers at the
//! clock, which is at most one part in 2^53 of it. The time of a state is the
//! start epoch plus that clock, rounded to the spacing of 64-bit numbers at
//! the epoch, 2^-23 s. A run whose end in decimal is exactly 2026-09-20 00:00 can
//! therefore reach its last step with a clock a little late: 499,999 steps of
//! 0.6 s read 299999.40000032616 s. The comparison with the last date carries
//! [`SatelliteLageos1::epoch_end_allowance_s`]: [`CLOCK_ROUNDING_PER_STEP`],
//! 2^-53, of the clock at the end of the step for each step taken so far at
//! this step, the clock over `dt`, and two more for this step and for the
//! step read from a decimal; and [`EPOCH_END_ALLOWANCE_S`], 2^-23 s, for the
//! two roundings at the epoch. It is never more than half the step, and the
//! comparison is with the 64-bit number nearest the last date plus the
//! allowance. On the first step of a run it is 1.2e-7 s; after 1,000,000
//! steps of 0.1 s, 1.1e-5 s. The clock is stepped at its own value and
//! nothing is pulled back: a run that ends on the last date in decimal can
//! read up to the allowance past it, and a step that would end later is
//! [`StepReject::EpochOutsideCard`]. The seven days of a forecast carry the
//! same allowance, [`SatelliteLageos1::horizon_end_allowance_s`], with the
//! steps counted from the published state, and a step that would end later
//! is [`StepReject::HorizonOutsideCard`].
//!
//! `core` only: no heap and no standard prelude. The card and the state are
//! `Copy`. The 16-byte frame proof is not touched.

extern crate core;

use core::prelude::v1::*;

use crate::frame::LastStateFrame64;

/// Gravitational parameter of the Earth. EGM2008 file header; IERS Conventions
/// (2010) section 6.1 names it as the value to use with EGM2008.
pub const GM_EARTH_M3_S2: f64 = 0.3986004415E+15;
/// Radius the field coefficients are scaled to. EGM2008 file header.
pub const FIELD_RADIUS_M: f64 = 0.63781363E+07;
/// Rate at which the frame turns: `2π × 1.00273781191135448 ÷ 86400`, the rate
/// of the Earth Rotation Angle, IERS Conventions (2010) Table 1.1.
pub const EARTH_ROTATION_RATE_RAD_S: f64 = 7.29211514670698e-05;
/// Gravitational parameter of the Sun. JPL Horizons data sheet for the Sun.
pub const GM_SUN_M3_S2: f64 = 1.3271244004193938e20;
/// Gravitational parameter of the Moon. JPL Horizons data sheet for the Moon.
pub const GM_MOON_M3_S2: f64 = 4.902800066e12;
/// Tide number of degree 2, order 0. IERS Conventions (2010) Table 6.3,
/// anelastic Earth, real part.
pub const LOVE_K20: f64 = 0.30190;
/// Tide number of degree 2, order 1. The same table.
pub const LOVE_K21: f64 = 0.29830;
/// Tide number of degree 2, order 2. The same table.
pub const LOVE_K22: f64 = 0.30102;
/// The part of the tide that does not vary with time, as a change to the
/// degree 2 order 0 coefficient. IERS Conventions (2010) eq. 6.14. It is
/// already inside the zero-tide coefficient, so the tide term leaves it out.
pub const PERMANENT_TIDE_DC20: f64 = 4.4228e-8 * (-0.31460) * 0.30190;
/// Mass of LAGEOS-1. ILRS mission page.
pub const SATELLITE_MASS_KG: f64 = 406.965;
/// Diameter of LAGEOS-1. ILRS mission page.
pub const SATELLITE_DIAMETER_M: f64 = 0.60;
/// Sunlight pressure on the sphere over that on a black sphere. Analysis-centre
/// notes of BKG and ESA: held fixed at 1.13.
pub const RADIATION_COEFFICIENT: f64 = 1.13;
/// Sunlight power per square metre at one astronomical unit. JPL Horizons
/// data sheet for the Sun.
pub const SOLAR_CONSTANT_W_M2: f64 = 1367.6;
/// IERS Conventions (2010) Table 1.1.
pub const ASTRONOMICAL_UNIT_M: f64 = 1.49597870700e11;
/// IERS Conventions (2010) Table 1.1.
pub const SPEED_OF_LIGHT_M_S: f64 = 299792458.0;
/// Terrestrial Time less UTC: 37 s (IERS leap-second file, in force since
/// 2017-01-01) and 32.184 s.
pub const TT_MINUS_UTC_S: f64 = 69.184;
/// Earth Rotation Angle at 2000-01-01 12:00, in turns. IERS Conventions (2010)
/// Table 1.1.
pub const ERA_AT_J2000_REV: f64 = 0.7790572732640;
/// Turns of the Earth Rotation Angle per day, less one. IERS Conventions
/// (2010) eq. 5.15.
pub const ERA_RATE_MINUS_ONE: f64 = 0.00273781191135448;
/// Linear term that takes the Earth Rotation Angle to sidereal time. IERS
/// Conventions (2010) eq. 5.32.
pub const GMST_PRECESSION_ARCSEC_PER_CENTURY: f64 = 4612.156534;

// <locked tables: generated text begins>
// Written by `lageos1_orbit/analysis/08_card_tables_and_reference.py` in the research
// folder. Every number is copied as text from its source file. Do not edit by hand.

/// Highest degree and order of the locked field.
pub const FIELD_DEGREE: usize = 12;
/// Rows and columns of the two field tables: degree 0 to 12, order 0 to 12.
pub const FIELD_ROWS: usize = 13;

/// Cosine coefficients of the Earth's field, fully normalised, `[degree][order]`.
/// Degree 2 order 0 is the zero-tide value and degrees 3 and 4 order 0 the values
/// at 2000.0 of IERS Conventions (2010) Table 6.2. The other 85 are EGM2008,
/// tide-free. An entry with no coefficient is `0.0`.
#[cfg_attr(rustfmt, rustfmt::skip)]
pub const FIELD_C: [[f64; FIELD_ROWS]; FIELD_ROWS] = [
    // degree 0
    [
        0.0,
        0.0,
        0.0,
        0.0,
        0.0,
        0.0,
        0.0,
        0.0,
        0.0,
        0.0,
        0.0,
        0.0,
        0.0,
    ],
    // degree 1
    [
        0.0,
        0.0,
        0.0,
        0.0,
        0.0,
        0.0,
        0.0,
        0.0,
        0.0,
        0.0,
        0.0,
        0.0,
        0.0,
    ],
    // degree 2
    [
        -0.48416948e-03,
        -0.206615509074176e-09,
        0.243938357328313e-05,
        0.0,
        0.0,
        0.0,
        0.0,
        0.0,
        0.0,
        0.0,
        0.0,
        0.0,
        0.0,
    ],
    // degree 3
    [
        0.9571612e-06,
        0.203046201047864e-05,
        0.904787894809528e-06,
        0.721321757121568e-06,
        0.0,
        0.0,
        0.0,
        0.0,
        0.0,
        0.0,
        0.0,
        0.0,
        0.0,
    ],
    // degree 4
    [
        0.5399659e-06,
        -0.536157389388867e-06,
        0.350501623962649e-06,
        0.990856766672321e-06,
        -0.188519633023033e-06,
        0.0,
        0.0,
        0.0,
        0.0,
        0.0,
        0.0,
        0.0,
        0.0,
    ],
    // degree 5
    [
        0.686702913736681e-07,
        -0.629211923042529e-07,
        0.652078043176164e-06,
        -0.451847152328843e-06,
        -0.295328761175629e-06,
        0.174811795496002e-06,
        0.0,
        0.0,
        0.0,
        0.0,
        0.0,
        0.0,
        0.0,
    ],
    // degree 6
    [
        -0.149953927978527e-06,
        -0.759210081892527e-07,
        0.486488924604690e-07,
        0.572451611175653e-07,
        -0.860237937191611e-07,
        -0.267166423703038e-06,
        0.947068749756882e-08,
        0.0,
        0.0,
        0.0,
        0.0,
        0.0,
        0.0,
    ],
    // degree 7
    [
        0.905120844521618e-07,
        0.280887555776673e-06,
        0.330407993702235e-06,
        0.250458409225729e-06,
        -0.274993935591631e-06,
        0.164773255934658e-08,
        -0.358798423464889e-06,
        0.150746472872675e-08,
        0.0,
        0.0,
        0.0,
        0.0,
        0.0,
    ],
    // degree 8
    [
        0.494756003005199e-07,
        0.231607991248329e-07,
        0.800143604736599e-07,
        -0.193745381715290e-07,
        -0.244360480007096e-06,
        -0.257011477267991e-07,
        -0.659648680031408e-07,
        0.672569751771483e-07,
        -0.124022771917136e-06,
        0.0,
        0.0,
        0.0,
        0.0,
    ],
    // degree 9
    [
        0.280180753216300e-07,
        0.142151377236084e-06,
        0.214144381199757e-07,
        -0.160612356882835e-06,
        -0.936529556592536e-08,
        -0.163134050605937e-07,
        0.627879491161446e-07,
        -0.117983924385618e-06,
        0.188136188986452e-06,
        -0.475568433357652e-07,
        0.0,
        0.0,
        0.0,
    ],
    // degree 10
    [
        0.533304381729473e-07,
        0.837623112620412e-07,
        -0.939894766092874e-07,
        -0.700709997317429e-08,
        -0.844715388074630e-07,
        -0.492894049964295e-07,
        -0.375849022022301e-07,
        0.826209286523474e-08,
        0.405981624580941e-07,
        0.125376631604340e-06,
        0.100435991936118e-06,
        0.0,
        0.0,
    ],
    // degree 11
    [
        -0.507683787085927e-07,
        0.156127678638183e-07,
        0.201135250154855e-07,
        -0.305773531606647e-07,
        -0.379499015091407e-07,
        0.374192407050580e-07,
        -0.156429128694775e-08,
        0.465461661449953e-08,
        -0.630174049861897e-08,
        -0.310727993686101e-07,
        -0.522444922089646e-07,
        0.462340571475799e-07,
        0.0,
    ],
    // degree 12
    [
        0.364361922614572e-07,
        -0.535856270449833e-07,
        0.142665936828290e-07,
        0.396211271409354e-07,
        -0.677284618097416e-07,
        0.308775410911475e-07,
        0.313421100991039e-08,
        -0.190517957483100e-07,
        -0.258866871220994e-07,
        0.419147664170774e-07,
        -0.619955079880774e-08,
        0.113644952089825e-07,
        -0.242377235648074e-08,
    ],
];

/// Sine coefficients of the Earth's field, fully normalised, `[degree][order]`.
/// 77 numbers, all EGM2008. An entry with no coefficient is `0.0`.
#[cfg_attr(rustfmt, rustfmt::skip)]
pub const FIELD_S: [[f64; FIELD_ROWS]; FIELD_ROWS] = [
    // degree 0
    [
        0.0,
        0.0,
        0.0,
        0.0,
        0.0,
        0.0,
        0.0,
        0.0,
        0.0,
        0.0,
        0.0,
        0.0,
        0.0,
    ],
    // degree 1
    [
        0.0,
        0.0,
        0.0,
        0.0,
        0.0,
        0.0,
        0.0,
        0.0,
        0.0,
        0.0,
        0.0,
        0.0,
        0.0,
    ],
    // degree 2
    [
        0.0,
        0.138441389137979e-08,
        -0.140027370385934e-05,
        0.0,
        0.0,
        0.0,
        0.0,
        0.0,
        0.0,
        0.0,
        0.0,
        0.0,
        0.0,
    ],
    // degree 3
    [
        0.0,
        0.248200415856872e-06,
        -0.619005475177618e-06,
        0.141434926192941e-05,
        0.0,
        0.0,
        0.0,
        0.0,
        0.0,
        0.0,
        0.0,
        0.0,
        0.0,
    ],
    // degree 4
    [
        0.0,
        -0.473567346518086e-06,
        0.662480026275829e-06,
        -0.200956723567452e-06,
        0.308803882149194e-06,
        0.0,
        0.0,
        0.0,
        0.0,
        0.0,
        0.0,
        0.0,
        0.0,
    ],
    // degree 5
    [
        0.0,
        -0.943698073395769e-07,
        -0.323353192540522e-06,
        -0.214955408306046e-06,
        0.498070550102351e-07,
        -0.669379935180165e-06,
        0.0,
        0.0,
        0.0,
        0.0,
        0.0,
        0.0,
        0.0,
    ],
    // degree 6
    [
        0.0,
        0.265122593213647e-07,
        -0.373789324523752e-06,
        0.895201130010730e-08,
        -0.471425573429095e-06,
        -0.536493151500206e-06,
        -0.237382353351005e-06,
        0.0,
        0.0,
        0.0,
        0.0,
        0.0,
        0.0,
    ],
    // degree 7
    [
        0.0,
        0.951259362869275e-07,
        0.929969290624092e-07,
        -0.217118287729610e-06,
        -0.124058403514343e-06,
        0.179281782751438e-07,
        0.151798257443669e-06,
        0.241068767286303e-07,
        0.0,
        0.0,
        0.0,
        0.0,
        0.0,
    ],
    // degree 8
    [
        0.0,
        0.588974540927606e-07,
        0.652805043667369e-07,
        -0.859639339125694e-07,
        0.698072508472777e-07,
        0.892034891745881e-07,
        0.308946730783065e-06,
        0.748686063738231e-07,
        0.120551889384997e-06,
        0.0,
        0.0,
        0.0,
        0.0,
    ],
    // degree 9
    [
        0.0,
        0.214004665077510e-07,
        -0.316984195352417e-07,
        -0.742658786809216e-07,
        0.199026740710063e-07,
        -0.540394840426217e-07,
        0.222962377434615e-06,
        -0.969222126840068e-07,
        -0.300538974811744e-08,
        0.968804214389955e-07,
        0.0,
        0.0,
        0.0,
    ],
    // degree 10
    [
        0.0,
        -0.131092332261065e-06,
        -0.512746772537482e-07,
        -0.154139929404373e-06,
        -0.790255527979406e-07,
        -0.506137282060864e-07,
        -0.797688616388143e-07,
        -0.304903703914366e-08,
        -0.917138622482163e-07,
        -0.379436584841270e-07,
        -0.238596204211893e-07,
        0.0,
        0.0,
    ],
    // degree 11
    [
        0.0,
        -0.271235374123689e-07,
        -0.990003954905590e-07,
        -0.148835345047152e-06,
        -0.637669897493018e-07,
        0.495908160271967e-07,
        0.342735099884706e-07,
        -0.898252194924903e-07,
        0.245446551115189e-07,
        0.420682585407293e-07,
        -0.184216383163730e-07,
        -0.696711251523700e-07,
        0.0,
    ],
    // degree 12
    [
        0.0,
        -0.431656037232084e-07,
        0.310937162901519e-07,
        0.250622628960907e-07,
        0.383823469584472e-08,
        0.759066416791107e-08,
        0.389801868153392e-07,
        0.357268620672699e-07,
        0.169362538600173e-07,
        0.249625636010847e-07,
        0.309398171578482e-07,
        -0.638551119140755e-08,
        -0.110993698692881e-07,
    ],
];

/// Angle between the plane of the Earth's orbit and the mean equator of 2000.0, degrees.
pub const OBLIQUITY_DEG: f64 = 23.43929111;
/// Rate at which the equinox moves along that plane, degrees per Julian century:
/// the term the Moon series carries in its own mean longitude.
pub const PRECESSION_IN_LONGITUDE_DEG_PER_CENTURY: f64 = 1.3972;

/// Sun: mean anomaly at 2000.0 and its rate per Julian century, degrees.
#[cfg_attr(rustfmt, rustfmt::skip)]
pub const SUN_MEAN_ANOMALY_DEG: [f64; 2] = [357.5256, 35999.049];
/// Sun: distance in units of 10⁹ m: the constant, the cosine of the mean anomaly,
/// the cosine of twice the mean anomaly.
#[cfg_attr(rustfmt, rustfmt::skip)]
pub const SUN_DISTANCE_GM: [f64; 3] = [149.619, -2.499, -0.021];
/// Sun: longitude of perihelion from the equinox of 2000.0, degrees.
pub const SUN_LONGITUDE_OFFSET_DEG: f64 = 282.9400;
/// Sun: longitude terms in arcseconds: the sine of the mean anomaly, the sine of twice it.
#[cfg_attr(rustfmt, rustfmt::skip)]
pub const SUN_LONGITUDE_ARCSEC: [f64; 2] = [6892.0, 72.0];

/// Moon: five angles, each its value at 2000.0 and its rate per Julian century, degrees:
/// mean longitude from the equinox of the date, mean anomaly, the Sun's mean anomaly,
/// mean distance from the ascending node, mean elongation from the Sun. The source
/// writes the first rate as `481267.88088 − 1.3972`; the card leaves the `− 1.3972`
/// out so that the longitude is measured from the equinox of the date.
#[cfg_attr(rustfmt, rustfmt::skip)]
pub const MOON_ARGUMENTS_DEG: [[f64; 2]; 5] = [
    [218.31617, 481267.88088],
    [134.96292, 477198.86753],
    [357.52543, 35999.04944],
    [93.27283, 483202.01873],
    [297.85207, 445267.11135],
];
/// Moon: constant term of the distance, kilometres.
pub const MOON_DISTANCE_CONSTANT_KM: f64 = 385000.0;
/// Moon: cosine terms of the distance, kilometres.
/// Each row: the amplitude, then the whole-number multipliers of the mean anomaly, the
/// Sun's mean anomaly, the distance from the node and the elongation inside the
/// cosine.
#[cfg_attr(rustfmt, rustfmt::skip)]
pub const MOON_DISTANCE_TERMS_KM: [[f64; 5]; 8] = [
    [-20905.0, 1.0, 0.0, 0.0, 0.0],
    [-3699.0, -1.0, 0.0, 0.0, 2.0],
    [-2956.0, 0.0, 0.0, 0.0, 2.0],
    [-570.0, 2.0, 0.0, 0.0, 0.0],
    [246.0, 2.0, 0.0, 0.0, -2.0],
    [-205.0, 0.0, 1.0, 0.0, -2.0],
    [-171.0, 1.0, 0.0, 0.0, 2.0],
    [-152.0, 1.0, 1.0, 0.0, -2.0],
];
/// Moon: sine terms of the longitude, arcseconds.
/// Each row: the amplitude, then the whole-number multipliers of the mean anomaly, the
/// Sun's mean anomaly, the distance from the node and the elongation inside the
/// sine.
#[cfg_attr(rustfmt, rustfmt::skip)]
pub const MOON_LONGITUDE_TERMS_ARCSEC: [[f64; 5]; 14] = [
    [22640.0, 1.0, 0.0, 0.0, 0.0],
    [769.0, 2.0, 0.0, 0.0, 0.0],
    [-4586.0, 1.0, 0.0, 0.0, -2.0],
    [2370.0, 0.0, 0.0, 0.0, 2.0],
    [-668.0, 0.0, 1.0, 0.0, 0.0],
    [-412.0, 0.0, 0.0, 2.0, 0.0],
    [-212.0, 2.0, 0.0, 0.0, -2.0],
    [-206.0, 1.0, 1.0, 0.0, -2.0],
    [192.0, 1.0, 0.0, 0.0, 2.0],
    [-165.0, 0.0, 1.0, 0.0, -2.0],
    [148.0, 1.0, -1.0, 0.0, 0.0],
    [-125.0, 0.0, 0.0, 0.0, 1.0],
    [-110.0, 1.0, 1.0, 0.0, 0.0],
    [-55.0, 0.0, 0.0, 2.0, -2.0],
];
/// Moon: the leading term of the latitude, arcseconds: its amplitude, and the two
/// amplitudes inside its argument (sine of twice the distance from the node, sine of
/// the Sun's mean anomaly).
#[cfg_attr(rustfmt, rustfmt::skip)]
pub const MOON_LATITUDE_LEAD_ARCSEC: [f64; 3] = [18520.0, 412.0, 541.0];
/// Moon: the other sine terms of the latitude, arcseconds.
/// Each row: the amplitude, then the whole-number multipliers of the mean anomaly, the
/// Sun's mean anomaly, the distance from the node and the elongation inside the
/// sine.
#[cfg_attr(rustfmt, rustfmt::skip)]
pub const MOON_LATITUDE_TERMS_ARCSEC: [[f64; 5]; 7] = [
    [-526.0, 0.0, 0.0, 1.0, -2.0],
    [44.0, 1.0, 0.0, 1.0, -2.0],
    [-31.0, -1.0, 0.0, 1.0, -2.0],
    [-25.0, -2.0, 0.0, 1.0, 0.0],
    [-23.0, 0.0, 1.0, 1.0, -2.0],
    [21.0, -1.0, 0.0, 1.0, 0.0],
    [11.0, 0.0, -1.0, 1.0, -2.0],
];
// <locked tables: generated text ends>

/// Orbit radius IERS Conventions (2010) Table 6.1 lists for this satellite.
pub const REFERENCE_RADIUS_M: f64 = 12270000.0;
/// `GM_EARTH_M3_S2 / REFERENCE_RADIUS_M`, 32 485 773.553 J/kg. Scale of the residual.
pub const LEDGER_SCALE_J_PER_KG: f64 = GM_EARTH_M3_S2 / REFERENCE_RADIUS_M;
/// `|J − J_anchor − W| / LEDGER_SCALE_J_PER_KG` must stay at or under this.
pub const RESIDUAL_CONSERVATION_LIMIT: f64 = 1.0e-9;
/// Bottom of the card in radius: under the lowest point any compared run
/// reached, 11 997.57 km, LAGEOS-2 near its perigee. Below it nothing was
/// compared with a published orbit.
pub const RADIUS_MIN_M: f64 = 11997000.0;
/// Top of the card in radius: over the highest point any compared run
/// reached, 12 326.40 km, LAGEOS-2 near its apogee.
pub const RADIUS_MAX_M: f64 = 12327000.0;
/// The two orbits compared with published orbits, LAGEOS-1 then LAGEOS-2,
/// each as the smallest and the largest semi-major axis, metres. Each band
/// holds every start and every landing of the 28 compared runs of its
/// satellite: 12 265.14 to 12 274.83 km and 12 158.51 to 12 165.68 km.
pub const ORBIT_SEMI_MAJOR_AXIS_M: [[f64; 2]; 2] = [[12265000.0, 12275000.0], [12158000.0, 12166000.0]];
/// The eccentricity of the same two orbits, smallest and largest: the
/// compared runs spanned 0.003438 to 0.004375 and 0.013232 to 0.013761.
pub const ORBIT_ECCENTRICITY: [[f64; 2]; 2] = [[0.0034, 0.0044], [0.0132, 0.0138]];
/// The inclination of the same two orbits to the card's z axis, smallest and
/// largest, degrees: the compared runs spanned 109.8039 to 109.8235 and
/// 52.6315 to 52.6508.
pub const ORBIT_INCLINATION_DEG: [[f64; 2]; 2] = [[109.80, 109.83], [52.63, 52.66]];
/// 2026-08-23 00:00 UTC, in seconds of UTC after 2000-01-01 12:00: the first
/// start of the compared runs, day 0 of week 260829.
pub const EPOCH_MIN_S: f64 = 840715200.0;
/// 2026-09-20 00:00 UTC, in the same seconds: the end of the last compared
/// run, seven days from day 0 of week 260919. The last published epoch it
/// was scored at is 2026-09-19 23:58 UTC.
pub const EPOCH_MAX_S: f64 = 843134400.0;
/// Seven days, seconds: the longest forecast the card gives from a state
/// taken from a published orbit, counted over every run it is restarted
/// through. The compared runs were 604 800 s long and were scored up to
/// 604 680 s, the last epoch of the published file.
pub const FORECAST_HORIZON_S: f64 = 604800.0;
/// The spacing of 64-bit numbers at the dates of the card, 2^-23 s: half of
/// it for the start epoch plus the clock, half for that sum plus the step.
/// The fixed part of [`SatelliteLageos1::epoch_end_allowance_s`].
pub const EPOCH_END_ALLOWANCE_S: f64 = 1.192_092_895_507_812_5e-7;
/// One part in 2^53: half the spacing of 64-bit numbers at the clock of a
/// run, over the clock, at most. The most one addition to the clock can
/// round by, as a part of the clock.
pub const CLOCK_ROUNDING_PER_STEP: f64 = 1.110_223_024_625_156_5e-16;
/// Steps added to the count in [`SatelliteLageos1::epoch_end_allowance_s`]:
/// the step being taken, and one for the step read from a decimal.
pub const EPOCH_END_ALLOWANCE_EXTRA_STEPS: f64 = 2.0;
/// Largest step the card stands behind. Against an 8th-order reference from
/// the card's own test state, RK4 at this step is 11.2 mm off after one day and
/// 13.9 cm after seven days, as measured by one recomputation outside this
/// crate on 2026-10-07, not confirmed by a second. Two hundredths of a radian
/// of the orbit would be 43 s, where RK4 is tens of metres off in a week.
pub const GATE_DT_S: f64 = 10.0;
/// Step ceiling divided by step floor: 2^20.
pub const GATE_OVER_FLOOR: f64 = 1_048_576.0;

/// The landing is in the Earth's shadow. Sunlight pressure was off there, and
/// that part of the model has not been compared with data.
pub const FLAG_EARTH_SHADOW: u64 = 1 << 0;

const SECONDS_PER_DAY: f64 = 86_400.0;
const SECONDS_PER_JULIAN_CENTURY: f64 = 36_525.0 * 86_400.0;
const ARCSECONDS_PER_DEGREE: f64 = 3_600.0;
const RADIANS_PER_DEGREE: f64 = core::f64::consts::PI / 180.0;
const RADIANS_PER_ARCSECOND: f64 = core::f64::consts::PI / (180.0 * 3_600.0);
const RADIANS_PER_TURN: f64 = 2.0 * core::f64::consts::PI;
/// Rows of the solid-harmonic tables: one degree above the field.
const HARMONIC_ROWS: usize = FIELD_ROWS + 1;

/// One table of field coefficients, `[degree][order]`.
type Coefficients = [[f64; FIELD_ROWS]; FIELD_ROWS];

/// Why a step did not advance the satellite or the frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StepReject {
    /// `dt` was NaN or infinite.
    NonFiniteInput,
    /// The incoming state, or the trial step, was non-finite.
    NonFiniteState,
    /// `dt` was zero or negative.
    NonPositiveDt,
    /// A recorded constant or table entry is not this satellite and its Earth.
    InvalidSatellite,
    /// Distance from the Earth's centre, incoming or landing, is below 11 997 km or above 12 327 km.
    RadiusOutsideCard,
    /// The orbit, incoming or landing, is not closed, or is in neither of the two compared orbits: LAGEOS-1, semi-major axis 12 265 to 12 275 km, eccentricity 0.0034 to 0.0044 and inclination 109.80 to 109.83 degrees, and LAGEOS-2, 12 158 to 12 166 km, 0.0132 to 0.0138 and 52.63 to 52.66 degrees.
    OrbitOutsideCard,
    /// The time, incoming or landing, is before 2026-08-23 00:00 or after 2026-09-20 00:00 UTC; the landing by more than the rounding allowance of the running sum of the clock: 1.19e-7 s, and one part in 2^53 of the clock for each step taken at this step and two more.
    EpochOutsideCard,
    /// The forecast, incoming or landing, is more than seven days, 604 800 s, from the state taken from a published orbit, or the stated time since that state is below zero; the landing by more than the same rounding allowance, counted over the whole forecast.
    HorizonOutsideCard,
    /// The trial step gained energy that the work of the Sun, the Moon, the tide, sunlight and relativity does not account for.
    SpontaneousGeneration,
    /// The trial step lost energy that the work of the Sun, the Moon, the tide, sunlight and relativity does not account for.
    FirstLawBreach,
}

/// The satellite is in free flight. The load is empty.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct SatelliteLoad {}

/// The locked satellite, its Earth, and its Sun and Moon. Every field is a
/// recorded copy of a crate constant.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SatelliteLageos1 {
    pub gm_earth_m3_s2: f64,
    pub field_radius_m: f64,
    pub field_c: Coefficients,
    pub field_s: Coefficients,
    pub earth_rotation_rate_rad_s: f64,
    pub gm_sun_m3_s2: f64,
    pub gm_moon_m3_s2: f64,
    pub love_k20: f64,
    pub love_k21: f64,
    pub love_k22: f64,
    pub permanent_tide_dc20: f64,
    pub satellite_mass_kg: f64,
    pub satellite_diameter_m: f64,
    pub radiation_coefficient: f64,
    pub solar_constant_w_m2: f64,
    pub astronomical_unit_m: f64,
    pub speed_of_light_m_s: f64,
    pub tt_minus_utc_s: f64,
    pub era_at_j2000_rev: f64,
    pub era_rate_minus_one: f64,
    pub gmst_precession_arcsec_per_century: f64,
    pub obliquity_deg: f64,
    pub precession_in_longitude_deg_per_century: f64,
    pub sun_mean_anomaly_deg: [f64; 2],
    pub sun_distance_gm: [f64; 3],
    pub sun_longitude_offset_deg: f64,
    pub sun_longitude_arcsec: [f64; 2],
    pub moon_arguments_deg: [[f64; 2]; 5],
    pub moon_distance_constant_km: f64,
    pub moon_distance_terms_km: [[f64; 5]; 8],
    pub moon_longitude_terms_arcsec: [[f64; 5]; 14],
    pub moon_latitude_lead_arcsec: [f64; 3],
    pub moon_latitude_terms_arcsec: [[f64; 5]; 7],
    pub reference_radius_m: f64,
    pub ledger_scale_j_per_kg: f64,
    pub residual_conservation_limit: f64,
    pub radius_min_m: f64,
    pub radius_max_m: f64,
    pub orbit_semi_major_axis_m: [[f64; 2]; 2],
    pub orbit_eccentricity: [[f64; 2]; 2],
    pub orbit_inclination_deg: [[f64; 2]; 2],
    pub epoch_min_s: f64,
    pub epoch_max_s: f64,
    /// Recorded copy of [`FORECAST_HORIZON_S`].
    pub forecast_horizon_s: f64,
    /// Recorded copy of [`EPOCH_END_ALLOWANCE_S`].
    pub epoch_end_allowance_s: f64,
    /// Recorded copy of [`CLOCK_ROUNDING_PER_STEP`].
    pub clock_rounding_per_step: f64,
    /// Recorded copy of [`EPOCH_END_ALLOWANCE_EXTRA_STEPS`].
    pub epoch_end_allowance_extra_steps: f64,
}

impl SatelliteLageos1 {
    /// LAGEOS-1. The constructor takes no arguments.
    #[must_use]
    pub fn lageos1() -> Self {
        let satellite = Self {
            gm_earth_m3_s2: GM_EARTH_M3_S2,
            field_radius_m: FIELD_RADIUS_M,
            field_c: FIELD_C,
            field_s: FIELD_S,
            earth_rotation_rate_rad_s: EARTH_ROTATION_RATE_RAD_S,
            gm_sun_m3_s2: GM_SUN_M3_S2,
            gm_moon_m3_s2: GM_MOON_M3_S2,
            love_k20: LOVE_K20,
            love_k21: LOVE_K21,
            love_k22: LOVE_K22,
            permanent_tide_dc20: PERMANENT_TIDE_DC20,
            satellite_mass_kg: SATELLITE_MASS_KG,
            satellite_diameter_m: SATELLITE_DIAMETER_M,
            radiation_coefficient: RADIATION_COEFFICIENT,
            solar_constant_w_m2: SOLAR_CONSTANT_W_M2,
            astronomical_unit_m: ASTRONOMICAL_UNIT_M,
            speed_of_light_m_s: SPEED_OF_LIGHT_M_S,
            tt_minus_utc_s: TT_MINUS_UTC_S,
            era_at_j2000_rev: ERA_AT_J2000_REV,
            era_rate_minus_one: ERA_RATE_MINUS_ONE,
            gmst_precession_arcsec_per_century: GMST_PRECESSION_ARCSEC_PER_CENTURY,
            obliquity_deg: OBLIQUITY_DEG,
            precession_in_longitude_deg_per_century: PRECESSION_IN_LONGITUDE_DEG_PER_CENTURY,
            sun_mean_anomaly_deg: SUN_MEAN_ANOMALY_DEG,
            sun_distance_gm: SUN_DISTANCE_GM,
            sun_longitude_offset_deg: SUN_LONGITUDE_OFFSET_DEG,
            sun_longitude_arcsec: SUN_LONGITUDE_ARCSEC,
            moon_arguments_deg: MOON_ARGUMENTS_DEG,
            moon_distance_constant_km: MOON_DISTANCE_CONSTANT_KM,
            moon_distance_terms_km: MOON_DISTANCE_TERMS_KM,
            moon_longitude_terms_arcsec: MOON_LONGITUDE_TERMS_ARCSEC,
            moon_latitude_lead_arcsec: MOON_LATITUDE_LEAD_ARCSEC,
            moon_latitude_terms_arcsec: MOON_LATITUDE_TERMS_ARCSEC,
            reference_radius_m: REFERENCE_RADIUS_M,
            ledger_scale_j_per_kg: LEDGER_SCALE_J_PER_KG,
            residual_conservation_limit: RESIDUAL_CONSERVATION_LIMIT,
            radius_min_m: RADIUS_MIN_M,
            radius_max_m: RADIUS_MAX_M,
            orbit_semi_major_axis_m: ORBIT_SEMI_MAJOR_AXIS_M,
            orbit_eccentricity: ORBIT_ECCENTRICITY,
            orbit_inclination_deg: ORBIT_INCLINATION_DEG,
            epoch_min_s: EPOCH_MIN_S,
            epoch_max_s: EPOCH_MAX_S,
            forecast_horizon_s: FORECAST_HORIZON_S,
            epoch_end_allowance_s: EPOCH_END_ALLOWANCE_S,
            clock_rounding_per_step: CLOCK_ROUNDING_PER_STEP,
            epoch_end_allowance_extra_steps: EPOCH_END_ALLOWANCE_EXTRA_STEPS,
        };
        if satellite.is_valid() {
            satellite
        } else {
            ::core::panic!("LAGEOS-1 satellite failed is_valid");
        }
    }

    /// Every recorded constant and table entry equals the crate's, bit for bit.
    #[must_use]
    pub fn is_valid(&self) -> bool {
        if !same_bits(self.gm_earth_m3_s2, GM_EARTH_M3_S2)
            || !same_bits(self.field_radius_m, FIELD_RADIUS_M)
            || !same_bits(self.earth_rotation_rate_rad_s, EARTH_ROTATION_RATE_RAD_S)
            || !same_bits(self.gm_sun_m3_s2, GM_SUN_M3_S2)
            || !same_bits(self.gm_moon_m3_s2, GM_MOON_M3_S2)
            || !same_bits(self.love_k20, LOVE_K20)
            || !same_bits(self.love_k21, LOVE_K21)
            || !same_bits(self.love_k22, LOVE_K22)
            || !same_bits(self.permanent_tide_dc20, PERMANENT_TIDE_DC20)
            || !same_bits(self.satellite_mass_kg, SATELLITE_MASS_KG)
            || !same_bits(self.satellite_diameter_m, SATELLITE_DIAMETER_M)
            || !same_bits(self.radiation_coefficient, RADIATION_COEFFICIENT)
            || !same_bits(self.solar_constant_w_m2, SOLAR_CONSTANT_W_M2)
            || !same_bits(self.astronomical_unit_m, ASTRONOMICAL_UNIT_M)
            || !same_bits(self.speed_of_light_m_s, SPEED_OF_LIGHT_M_S)
            || !same_bits(self.tt_minus_utc_s, TT_MINUS_UTC_S)
            || !same_bits(self.era_at_j2000_rev, ERA_AT_J2000_REV)
            || !same_bits(self.era_rate_minus_one, ERA_RATE_MINUS_ONE)
            || !same_bits(
                self.gmst_precession_arcsec_per_century,
                GMST_PRECESSION_ARCSEC_PER_CENTURY,
            )
            || !same_bits(self.obliquity_deg, OBLIQUITY_DEG)
            || !same_bits(
                self.precession_in_longitude_deg_per_century,
                PRECESSION_IN_LONGITUDE_DEG_PER_CENTURY,
            )
            || !same_bits(self.sun_longitude_offset_deg, SUN_LONGITUDE_OFFSET_DEG)
            || !same_bits(self.moon_distance_constant_km, MOON_DISTANCE_CONSTANT_KM)
            || !same_bits(self.reference_radius_m, REFERENCE_RADIUS_M)
            || !same_bits(self.ledger_scale_j_per_kg, LEDGER_SCALE_J_PER_KG)
            || !same_bits(
                self.residual_conservation_limit,
                RESIDUAL_CONSERVATION_LIMIT,
            )
            || !same_bits(self.radius_min_m, RADIUS_MIN_M)
            || !same_bits(self.radius_max_m, RADIUS_MAX_M)
            || !same_bits(self.epoch_min_s, EPOCH_MIN_S)
            || !same_bits(self.epoch_max_s, EPOCH_MAX_S)
            || !same_bits(self.forecast_horizon_s, FORECAST_HORIZON_S)
            || !same_bits(self.epoch_end_allowance_s, EPOCH_END_ALLOWANCE_S)
            || !same_bits(self.clock_rounding_per_step, CLOCK_ROUNDING_PER_STEP)
            || !same_bits(self.epoch_end_allowance_extra_steps, EPOCH_END_ALLOWANCE_EXTRA_STEPS)
        {
            return false;
        }
        let mut degree = 0;
        while degree < FIELD_ROWS {
            if !same_row(&self.field_c[degree], &FIELD_C[degree])
                || !same_row(&self.field_s[degree], &FIELD_S[degree])
            {
                return false;
            }
            degree += 1;
        }
        let mut orbit = 0;
        while orbit < ORBIT_SEMI_MAJOR_AXIS_M.len() {
            if !same_row(&self.orbit_semi_major_axis_m[orbit], &ORBIT_SEMI_MAJOR_AXIS_M[orbit])
                || !same_row(&self.orbit_eccentricity[orbit], &ORBIT_ECCENTRICITY[orbit])
                || !same_row(&self.orbit_inclination_deg[orbit], &ORBIT_INCLINATION_DEG[orbit])
            {
                return false;
            }
            orbit += 1;
        }
        if !same_row(&self.sun_mean_anomaly_deg, &SUN_MEAN_ANOMALY_DEG)
            || !same_row(&self.sun_distance_gm, &SUN_DISTANCE_GM)
            || !same_row(&self.sun_longitude_arcsec, &SUN_LONGITUDE_ARCSEC)
            || !same_row(&self.moon_latitude_lead_arcsec, &MOON_LATITUDE_LEAD_ARCSEC)
        {
            return false;
        }
        let mut row = 0;
        while row < MOON_ARGUMENTS_DEG.len() {
            if !same_row(&self.moon_arguments_deg[row], &MOON_ARGUMENTS_DEG[row]) {
                return false;
            }
            row += 1;
        }
        let mut row = 0;
        while row < MOON_DISTANCE_TERMS_KM.len() {
            if !same_row(
                &self.moon_distance_terms_km[row],
                &MOON_DISTANCE_TERMS_KM[row],
            ) {
                return false;
            }
            row += 1;
        }
        let mut row = 0;
        while row < MOON_LONGITUDE_TERMS_ARCSEC.len() {
            if !same_row(
                &self.moon_longitude_terms_arcsec[row],
                &MOON_LONGITUDE_TERMS_ARCSEC[row],
            ) {
                return false;
            }
            row += 1;
        }
        let mut row = 0;
        while row < MOON_LATITUDE_TERMS_ARCSEC.len() {
            if !same_row(
                &self.moon_latitude_terms_arcsec[row],
                &MOON_LATITUDE_TERMS_ARCSEC[row],
            ) {
                return false;
            }
            row += 1;
        }
        true
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
    /// 9.5367431640625e-6 s. Nothing in this card's arithmetic raised it.
    ///
    /// The clock: the longest forecast the card allows ends 604 800 s after
    /// the published state, and the time the Sun and the Moon are placed at
    /// reaches 843 134 400 s. 64-bit numbers there are spaced at most 2^-33 s
    /// and 2^-23 s. The floor is 81 920 and 80 of those, and half of it, the
    /// stage time, is 40 960 and 40, so the clock and every stage time advance
    /// by the step exactly. A step sixteen times smaller would not.
    ///
    /// The state: in one floor step the satellite moves 0.054 m, against a
    /// spacing of 1.9e-9 m in a coordinate near 12 270 km; its velocity
    /// changes by 2.5e-5 m/s, against a spacing of 9.1e-13 m/s; the work of
    /// the other forces changes by up to 5e-8 J/kg, against a spacing of
    /// 6e-14 J/kg at 300 J/kg. Each still holds about seven figures of its
    /// own movement.
    ///
    /// The ledger: 16 777 216 honest steps at the floor, 160 s of flight from
    /// the start state of the tests, all landed. The residual stayed under
    /// 6.5e-14 of a limit of 1e-9, and the position ended 3.2e-6 m from the
    /// same span taken in steps of 10 s.
    ///
    /// Not applied by [`step`].
    #[must_use]
    pub fn floor_dt_s(&self) -> Option<f64> {
        match self.gate_dt_s() {
            Some(gate_dt_s) => Some(gate_dt_s / GATE_OVER_FLOOR),
            None => None,
        }
    }

    /// How far past [`EPOCH_MAX_S`] the time of a state may read at the end
    /// of a step of `dt_s` taken from a run's clock of `t_s`, seconds:
    /// [`EPOCH_END_ALLOWANCE_S`], and [`CLOCK_ROUNDING_PER_STEP`] of the
    /// clock at the end of the step for each step taken so far at this step
    /// and [`EPOCH_END_ALLOWANCE_EXTRA_STEPS`] more; never more than half the
    /// step. Zero for a step that is not positive or a clock below zero.
    #[must_use]
    pub fn epoch_end_allowance_s(&self, t_s: f64, dt_s: f64) -> f64 {
        if !(dt_s > 0.0 && t_s >= 0.0) {
            return 0.0;
        }
        let steps = t_s / dt_s + self.epoch_end_allowance_extra_steps;
        let allowance_s =
            self.epoch_end_allowance_s + steps * self.clock_rounding_per_step * (t_s + dt_s);
        if allowance_s < 0.5 * dt_s { allowance_s } else { 0.5 * dt_s }
    }

    /// How far past [`FORECAST_HORIZON_S`] the time since the published
    /// state may read at the end of a step of `dt_s` taken at a time since
    /// that state of `elapsed_s`, seconds: the allowance of the last date,
    /// [`Self::epoch_end_allowance_s`], with the steps counted over the whole
    /// forecast, so the steps of the runs a forecast was restarted through
    /// are counted too.
    #[must_use]
    pub fn horizon_end_allowance_s(&self, elapsed_s: f64, dt_s: f64) -> f64 {
        self.epoch_end_allowance_s(elapsed_s, dt_s)
    }

    /// The latest epoch, in seconds, a step can start from: the largest
    /// 64-bit number `epoch_utc_s` for which `epoch_utc_s + floor_dt_s` is
    /// not after the last date of the card. This is the published bound and
    /// holds no rounding allowance: [`Self::epoch_end_allowance_s`] belongs
    /// to the comparison a step makes, not to the bound a caller reads.
    #[must_use]
    pub fn last_start_epoch_utc_s(&self) -> Option<f64> {
        let floor_dt_s = self.floor_dt_s()?;
        let end_s = self.epoch_max_s;
        let mut epoch_utc_s = end_s - floor_dt_s;
        if !(epoch_utc_s >= self.epoch_min_s) {
            return None;
        }
        while !(epoch_utc_s + floor_dt_s <= end_s) {
            epoch_utc_s = f64::from_bits(epoch_utc_s.to_bits() - 1);
        }
        loop {
            let above_s = f64::from_bits(epoch_utc_s.to_bits() + 1);
            if above_s + floor_dt_s <= end_s {
                epoch_utc_s = above_s;
            } else {
                break;
            }
        }
        Some(epoch_utc_s)
    }
}

/// Integrated satellite. The ledger fields are part of the state; write them
/// only through [`step`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SatelliteState {
    /// Seconds since the start of the run.
    pub t_s: f64,
    /// UTC at the start of the run, seconds after 2000-01-01 12:00 UTC. Set once.
    pub start_epoch_utc_s: f64,
    /// Time since the state was taken from a published orbit, at the start
    /// of the run, seconds. Stated by the caller. Set once.
    pub forecast_elapsed_at_start_s: f64,
    /// In the turning frame.
    pub position_m: [f64; 3],
    /// In the turning frame.
    pub velocity_m_s: [f64; 3],
    /// `J = ½ v·v − U(r) − ½ ω² (x² + y²)`: energy per kilogram seen from the turning frame.
    pub jacobi_j_per_kg: f64,
    /// `J` at the state the ledger started from.
    pub jacobi_anchor_j_per_kg: f64,
    /// Work per kilogram of the Sun, the Moon, the tide, sunlight and
    /// relativity, integrated by the step.
    pub external_work_j_per_kg: f64,
    /// Signed `J − J_anchor − W`. Positive is created energy.
    pub energy_defect_j_per_kg: f64,
    /// `|energy_defect_j_per_kg| / LEDGER_SCALE_J_PER_KG`.
    pub residual_conservation_error: f64,
    /// Distance from the Earth's centre.
    pub radius_m: f64,
    /// `−GM ÷ (u·u − 2 GM/|r|)`, with `u` the velocity seen from a frame that does not turn.
    pub semi_major_axis_m: f64,
    /// Sunlight pressure is off at this state.
    pub in_earth_shadow: bool,
}

impl SatelliteState {
    /// UTC of this state, seconds after 2000-01-01 12:00 UTC: the start epoch
    /// plus the time elapsed. It is worked out from the two each time it is
    /// asked, with one rounding, and never kept as a sum that gains a step at
    /// a time, so it does not drift with the number of steps. It is the time
    /// the next step starts from. A state handed back as a new start with this
    /// epoch continues the same orbit.
    #[must_use]
    pub fn epoch_utc_s(&self) -> f64 {
        self.start_epoch_utc_s + self.t_s
    }

    /// Time since the state was taken from a published orbit, seconds: the
    /// stated time at the start of the run plus `t_s`, with one rounding. A
    /// state handed back as a new start carries this number.
    #[must_use]
    pub fn forecast_elapsed_s(&self) -> f64 {
        self.forecast_elapsed_at_start_s + self.t_s
    }

    /// Anchor the ledger at this epoch, time since the published state,
    /// position and velocity. `None` unless every number is finite, the
    /// radius is from 11 997 to 12 327 km, the orbit is one of the two
    /// compared orbits, the epoch is from 2026-08-23 00:00 to 2026-09-20 00:00
    /// UTC, and the time since the published state is from zero to seven days.
    #[must_use]
    pub fn try_state(
        satellite: &SatelliteLageos1,
        epoch_utc_s: f64,
        forecast_elapsed_s: f64,
        position_m: [f64; 3],
        velocity_m_s: [f64; 3],
    ) -> Option<Self> {
        if !satellite.is_valid()
            || !epoch_utc_s.is_finite()
            || !forecast_elapsed_s.is_finite()
            || !finite3(position_m)
            || !finite3(velocity_m_s)
        {
            return None;
        }
        let here = match place(satellite, epoch_utc_s, 0.0, position_m, velocity_m_s) {
            Ok(here) => here,
            Err(_) => return None,
        };
        if !(forecast_elapsed_s >= 0.0 && forecast_elapsed_s <= satellite.forecast_horizon_s) {
            return None;
        }
        let state = Self {
            t_s: 0.0,
            start_epoch_utc_s: epoch_utc_s,
            forecast_elapsed_at_start_s: forecast_elapsed_s,
            position_m,
            velocity_m_s,
            jacobi_j_per_kg: here.jacobi_j_per_kg,
            jacobi_anchor_j_per_kg: here.jacobi_j_per_kg,
            external_work_j_per_kg: 0.0,
            energy_defect_j_per_kg: 0.0,
            residual_conservation_error: 0.0,
            radius_m: here.radius_m,
            semi_major_axis_m: here.semi_major_axis_m,
            in_earth_shadow: here.in_earth_shadow,
        };
        if state_is_finite(&state) {
            Some(state)
        } else {
            None
        }
    }
}

/// Advance one RK4 step and write the frame.
///
/// The satellite is driven by the clock alone; `load` is empty. A rejected
/// step leaves `state` and `frame` as they were.
#[must_use]
pub fn step(
    satellite: &SatelliteLageos1,
    state: &mut SatelliteState,
    frame: &mut LastStateFrame64,
    load: SatelliteLoad,
    dt_s: f64,
) -> Result<(), StepReject> {
    validate(satellite, state, load, dt_s)?;
    let predicted = integrate(
        satellite,
        state.epoch_utc_s(),
        recorded(state),
        dt_s,
    )?;
    commit(satellite, state, frame, dt_s, predicted)
}

fn validate(
    satellite: &SatelliteLageos1,
    state: &SatelliteState,
    _load: SatelliteLoad,
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
    if !satellite.is_valid() {
        return Err(StepReject::InvalidSatellite);
    }
    let utc_s = state.epoch_utc_s();
    place(satellite, utc_s, 0.0, state.position_m, state.velocity_m_s)?;
    // The Sun and the Moon are placed at every stage time, so the whole step
    // must be inside the card's dates before the first stage is taken.
    if !(utc_s + dt_s
        <= satellite.epoch_max_s + satellite.epoch_end_allowance_s(state.t_s, dt_s))
    {
        return Err(StepReject::EpochOutsideCard);
    }
    // The forecast: the time since the published state, at the landing, is
    // held to seven days, with the steps counted over the whole forecast.
    let elapsed_s = state.forecast_elapsed_s();
    if !(state.forecast_elapsed_at_start_s >= 0.0
        && state.forecast_elapsed_at_start_s + (state.t_s + dt_s)
            <= satellite.forecast_horizon_s
                + satellite.horizon_end_allowance_s(elapsed_s, dt_s))
    {
        return Err(StepReject::HorizonOutsideCard);
    }
    Ok(())
}

/// One classical RK4 step on `(r, v, W)` from time `utc_s`. The stages are
/// trial points of the method, not places the satellite is said to be: the
/// card's bands are applied to the incoming state and to the landing.
fn integrate(
    satellite: &SatelliteLageos1,
    utc_s: f64,
    y: [f64; 7],
    dt_s: f64,
) -> Result<[f64; 7], StepReject> {
    let k1 = rates(satellite, utc_s, y)?;
    let k2 = rates(satellite, utc_s + 0.5 * dt_s, axpy(y, dt_s * 0.5, k1))?;
    let k3 = rates(satellite, utc_s + 0.5 * dt_s, axpy(y, dt_s * 0.5, k2))?;
    let k4 = rates(satellite, utc_s + dt_s, axpy(y, dt_s, k3))?;
    let mut next = y;
    let mut index = 0;
    while index < 7 {
        next[index] += (dt_s / 6.0) * (k1[index] + 2.0 * k2[index] + 2.0 * k3[index] + k4[index]);
        index += 1;
    }
    if finite7(next) {
        Ok(next)
    } else {
        Err(StepReject::NonFiniteState)
    }
}

fn commit(
    satellite: &SatelliteLageos1,
    state: &mut SatelliteState,
    frame: &mut LastStateFrame64,
    dt_s: f64,
    predicted: [f64; 7],
) -> Result<(), StepReject> {
    let saved = *state;
    if !finite7(predicted) {
        return Err(StepReject::NonFiniteState);
    }
    let position_m = [predicted[0], predicted[1], predicted[2]];
    let velocity_m_s = [predicted[3], predicted[4], predicted[5]];
    let t_s = state.t_s + dt_s;
    let here = place(
        satellite,
        state.start_epoch_utc_s + t_s,
        satellite.epoch_end_allowance_s(state.t_s, dt_s),
        position_m,
        velocity_m_s,
    )?;
    let scale = satellite.ledger_scale_j_per_kg;
    if !(scale > 0.0 && scale.is_finite()) {
        return Err(StepReject::InvalidSatellite);
    }
    let defect = here.jacobi_j_per_kg - state.jacobi_anchor_j_per_kg - predicted[6];
    if !defect.is_finite() {
        return Err(StepReject::NonFiniteState);
    }
    let gate = satellite.residual_conservation_limit * scale;
    if defect > gate {
        return Err(StepReject::SpontaneousGeneration);
    }
    if defect < -gate {
        return Err(StepReject::FirstLawBreach);
    }
    state.t_s = t_s;
    state.position_m = position_m;
    state.velocity_m_s = velocity_m_s;
    state.jacobi_j_per_kg = here.jacobi_j_per_kg;
    state.external_work_j_per_kg = predicted[6];
    state.energy_defect_j_per_kg = defect;
    state.residual_conservation_error = abs_f64(defect) / scale;
    state.radius_m = here.radius_m;
    state.semi_major_axis_m = here.semi_major_axis_m;
    state.in_earth_shadow = here.in_earth_shadow;
    if !state_is_finite(state) {
        *state = saved;
        return Err(StepReject::NonFiniteState);
    }
    publish(satellite, state, frame);
    Ok(())
}

/// Each coordinate goes into the frame as its nearest 32-bit number and, in
/// the slot beside it, what that rounding left over.
fn publish(satellite: &SatelliteLageos1, state: &SatelliteState, frame: &mut LastStateFrame64) {
    frame.t = state.t_s;
    let mut axis = 0;
    while axis < 3 {
        let rounded = state.position_m[axis] as f32;
        frame.pos[axis] = rounded;
        frame.vel[axis] = (state.position_m[axis] - f64::from(rounded)) as f32;
        axis += 1;
    }
    frame.force_torque = (state.radius_m - satellite.reference_radius_m) as f32;
    frame.residual = state.residual_conservation_error as f32;
    frame.flags = if state.in_earth_shadow {
        FLAG_EARTH_SHADOW
    } else {
        0
    };
}

/// What follows from one time, position and velocity.
struct Place {
    radius_m: f64,
    semi_major_axis_m: f64,
    jacobi_j_per_kg: f64,
    in_earth_shadow: bool,
}

/// The derived fields at one time, position and velocity. A radius, an orbit
/// or a time outside the card is refused by name. `end_allowance_s` is
/// how far past the last date the time may read: zero for an incoming state,
/// [`SatelliteLageos1::epoch_end_allowance_s`] for the landing of a step.
fn place(
    satellite: &SatelliteLageos1,
    utc_s: f64,
    end_allowance_s: f64,
    position_m: [f64; 3],
    velocity_m_s: [f64; 3],
) -> Result<Place, StepReject> {
    let radius_m = dot(position_m, position_m).sqrt();
    if !radius_m.is_finite() || !finite3(velocity_m_s) || !utc_s.is_finite() {
        return Err(StepReject::NonFiniteState);
    }
    if !(radius_m >= satellite.radius_min_m && radius_m <= satellite.radius_max_m) {
        return Err(StepReject::RadiusOutsideCard);
    }
    let fixed_m_s = fixed_frame_velocity_m_s(satellite, position_m, velocity_m_s);
    let semi_major_axis_m = -satellite.gm_earth_m3_s2
        / (dot(fixed_m_s, fixed_m_s) - 2.0 * satellite.gm_earth_m3_s2 / radius_m);
    if !in_a_compared_orbit(satellite, position_m, fixed_m_s, semi_major_axis_m) {
        return Err(StepReject::OrbitOutsideCard);
    }
    if !(utc_s >= satellite.epoch_min_s && utc_s <= satellite.epoch_max_s + end_allowance_s) {
        return Err(StepReject::EpochOutsideCard);
    }
    let jacobi_j_per_kg = jacobi_j_per_kg(satellite, position_m, velocity_m_s);
    if !jacobi_j_per_kg.is_finite() {
        return Err(StepReject::NonFiniteState);
    }
    let sun_m = sun_position_m(satellite, utc_s);
    Ok(Place {
        radius_m,
        semi_major_axis_m,
        jacobi_j_per_kg,
        in_earth_shadow: !sunlit(satellite, position_m, sun_m),
    })
}

/// The orbit is closed and inside one of the two compared orbits in its
/// semi-major axis, its eccentricity and its inclination together. The
/// eccentricity is `√(1 − h·h ÷ (GM a))` and the cosine of the inclination is
/// `h_z ÷ |h|`, with `h = r × u` and `u` the velocity seen from a frame that
/// does not turn: two-body elements at this instant, which the compared runs
/// also had at every landing. No angle is taken of `h`: the cosine is held
/// between the cosines of the two bounds.
fn in_a_compared_orbit(
    satellite: &SatelliteLageos1,
    position_m: [f64; 3],
    fixed_m_s: [f64; 3],
    semi_major_axis_m: f64,
) -> bool {
    if !(semi_major_axis_m > 0.0 && semi_major_axis_m.is_finite()) {
        return false;
    }
    let h = [
        position_m[1] * fixed_m_s[2] - position_m[2] * fixed_m_s[1],
        position_m[2] * fixed_m_s[0] - position_m[0] * fixed_m_s[2],
        position_m[0] * fixed_m_s[1] - position_m[1] * fixed_m_s[0],
    ];
    let h_squared = dot(h, h);
    if !(h_squared > 0.0 && h_squared.is_finite()) {
        return false;
    }
    let one_less_e_squared = h_squared / (satellite.gm_earth_m3_s2 * semi_major_axis_m);
    let e_squared = 1.0 - one_less_e_squared;
    let eccentricity = if e_squared > 0.0 { e_squared.sqrt() } else { 0.0 };
    let cos_inclination = h[2] / h_squared.sqrt();
    let mut orbit = 0;
    while orbit < satellite.orbit_semi_major_axis_m.len() {
        let a = satellite.orbit_semi_major_axis_m[orbit];
        let e = satellite.orbit_eccentricity[orbit];
        let i = satellite.orbit_inclination_deg[orbit];
        // The cosine falls as the inclination rises from 0 to 180 degrees.
        let cos_lowest = crate::math::cos(i[0] * RADIANS_PER_DEGREE);
        let cos_highest = crate::math::cos(i[1] * RADIANS_PER_DEGREE);
        if semi_major_axis_m >= a[0]
            && semi_major_axis_m <= a[1]
            && eccentricity >= e[0]
            && eccentricity <= e[1]
            && cos_inclination <= cos_lowest
            && cos_inclination >= cos_highest
        {
            return true;
        }
        orbit += 1;
    }
    false
}

/// Rates of `(r, v, W)` at time `utc_s`.
fn rates(satellite: &SatelliteLageos1, utc_s: f64, y: [f64; 7]) -> Result<[f64; 7], StepReject> {
    let position_m = [y[0], y[1], y[2]];
    let velocity_m_s = [y[3], y[4], y[5]];
    let field = field_acceleration_m_s2(satellite, position_m);
    let other = other_acceleration_m_s2(satellite, utc_s, position_m, velocity_m_s);
    let turning = turning_frame_acceleration_m_s2(satellite, position_m, velocity_m_s);
    let out = [
        velocity_m_s[0],
        velocity_m_s[1],
        velocity_m_s[2],
        (field[0] + other[0]) + turning[0],
        (field[1] + other[1]) + turning[1],
        (field[2] + other[2]) + turning[2],
        dot(other, velocity_m_s),
    ];
    if finite7(out) {
        Ok(out)
    } else {
        Err(StepReject::NonFiniteState)
    }
}

/// Coriolis and centrifugal terms: `(2 ω v_y + ω² x, −2 ω v_x + ω² y, 0)`.
fn turning_frame_acceleration_m_s2(
    satellite: &SatelliteLageos1,
    position_m: [f64; 3],
    velocity_m_s: [f64; 3],
) -> [f64; 3] {
    let w = satellite.earth_rotation_rate_rad_s;
    [
        2.0 * w * velocity_m_s[1] + w * w * position_m[0],
        -2.0 * w * velocity_m_s[0] + w * w * position_m[1],
        0.0,
    ]
}

/// Velocity seen from a frame that does not turn, in the turning frame's axes.
fn fixed_frame_velocity_m_s(
    satellite: &SatelliteLageos1,
    position_m: [f64; 3],
    velocity_m_s: [f64; 3],
) -> [f64; 3] {
    let w = satellite.earth_rotation_rate_rad_s;
    [
        velocity_m_s[0] - w * position_m[1],
        velocity_m_s[1] + w * position_m[0],
        velocity_m_s[2],
    ]
}

/// `J = ½ v·v − U(r) − ½ ω² (x² + y²)`.
fn jacobi_j_per_kg(
    satellite: &SatelliteLageos1,
    position_m: [f64; 3],
    velocity_m_s: [f64; 3],
) -> f64 {
    let w = satellite.earth_rotation_rate_rad_s;
    0.5 * dot(velocity_m_s, velocity_m_s)
        - field_potential_j_per_kg(satellite, position_m)
        - 0.5 * w * w * (position_m[0] * position_m[0] + position_m[1] * position_m[1])
}

/// The locked field: point mass and degrees 2 to 12.
fn field_acceleration_m_s2(satellite: &SatelliteLageos1, position_m: [f64; 3]) -> [f64; 3] {
    let harmonics = harmonic_acceleration_m_s2(
        satellite,
        &satellite.field_c,
        &satellite.field_s,
        FIELD_DEGREE,
        position_m,
    );
    let radius_squared = dot(position_m, position_m);
    let central = -satellite.gm_earth_m3_s2 / (radius_squared * radius_squared.sqrt());
    [
        central * position_m[0] + harmonics[0],
        central * position_m[1] + harmonics[1],
        central * position_m[2] + harmonics[2],
    ]
}

/// Potential of the locked field, positive: `GM/|r|` and degrees 2 to 12.
fn field_potential_j_per_kg(satellite: &SatelliteLageos1, position_m: [f64; 3]) -> f64 {
    let solid = solid_harmonics(satellite.field_radius_m, position_m, FIELD_DEGREE);
    let mut sum = 0.0;
    let mut degree = FIELD_DEGREE;
    while degree >= 2 {
        let mut ratio = 1.0;
        let mut order = 0;
        while order <= degree {
            if order > 0 {
                ratio /= ((degree - order + 1) * (degree + order)) as f64;
            }
            let scale = unnormalised_over_normalised(degree, order, ratio);
            sum += scale
                * (satellite.field_c[degree][order] * solid.v[degree][order]
                    + satellite.field_s[degree][order] * solid.w[degree][order]);
            order += 1;
        }
        degree -= 1;
    }
    satellite.gm_earth_m3_s2 / dot(position_m, position_m).sqrt()
        + satellite.gm_earth_m3_s2 / satellite.field_radius_m * sum
}

/// Solid harmonics `V` and `W`, `[degree][order]`, unnormalised.
struct SolidHarmonics {
    v: [[f64; HARMONIC_ROWS]; HARMONIC_ROWS],
    w: [[f64; HARMONIC_ROWS]; HARMONIC_ROWS],
}

/// `V + iW = (R/|r|)^(n+1) P_nm(z/|r|) ((x + iy)/ρ)^m` for degrees 0 to `top`,
/// by recursion on `x`, `y`, `z` alone. `P_nm` is the unnormalised associated
/// Legendre function.
fn solid_harmonics(field_radius_m: f64, position_m: [f64; 3], top: usize) -> SolidHarmonics {
    let radius_squared = dot(position_m, position_m);
    let scale = field_radius_m / radius_squared;
    let x = scale * position_m[0];
    let y = scale * position_m[1];
    let z = scale * position_m[2];
    let ratio_squared = field_radius_m * scale;
    let mut v = [[0.0; HARMONIC_ROWS]; HARMONIC_ROWS];
    let mut w = [[0.0; HARMONIC_ROWS]; HARMONIC_ROWS];
    v[0][0] = field_radius_m / radius_squared.sqrt();
    let mut order = 0;
    while order <= top {
        if order > 0 {
            let odd = (2 * order - 1) as f64;
            v[order][order] = odd * (x * v[order - 1][order - 1] - y * w[order - 1][order - 1]);
            w[order][order] = odd * (x * w[order - 1][order - 1] + y * v[order - 1][order - 1]);
        }
        if order < top {
            let odd = (2 * order + 1) as f64;
            v[order + 1][order] = odd * z * v[order][order];
            w[order + 1][order] = odd * z * w[order][order];
        }
        let mut degree = order + 2;
        while degree <= top {
            let first = (2 * degree - 1) as f64;
            let second = (degree + order - 1) as f64;
            let span = (degree - order) as f64;
            v[degree][order] = (first * z * v[degree - 1][order]
                - second * ratio_squared * v[degree - 2][order])
                / span;
            w[degree][order] = (first * z * w[degree - 1][order]
                - second * ratio_squared * w[degree - 2][order])
                / span;
            degree += 1;
        }
        order += 1;
    }
    SolidHarmonics { v, w }
}

/// What a fully normalised coefficient is multiplied by to make the
/// unnormalised one: `sqrt((2 − δ) (2n + 1) (n − m)! / (n + m)!)`, with `δ`
/// one at order 0 and zero otherwise. `factorial_ratio` is `(n − m)! / (n + m)!`.
fn unnormalised_over_normalised(degree: usize, order: usize, factorial_ratio: f64) -> f64 {
    let twice = if order == 0 { 1.0 } else { 2.0 };
    (twice * ((2 * degree + 1) as f64) * factorial_ratio).sqrt()
}

/// Acceleration of the degrees 2 to `top` of a field given by fully
/// normalised coefficients. The point mass is not included.
fn harmonic_acceleration_m_s2(
    satellite: &SatelliteLageos1,
    cosine: &Coefficients,
    sine: &Coefficients,
    top: usize,
    position_m: [f64; 3],
) -> [f64; 3] {
    let solid = solid_harmonics(satellite.field_radius_m, position_m, top + 1);
    let mut sum = [0.0; 3];
    let mut degree = top;
    while degree >= 2 {
        let up = degree + 1;
        let mut ratio = 1.0;
        let mut order = 0;
        while order <= degree {
            if order > 0 {
                ratio /= ((degree - order + 1) * (degree + order)) as f64;
            }
            let scale = unnormalised_over_normalised(degree, order, ratio);
            let c = scale * cosine[degree][order];
            let s = scale * sine[degree][order];
            if order == 0 {
                sum[0] -= c * solid.v[up][1];
                sum[1] -= c * solid.w[up][1];
            } else {
                let pair = ((degree - order + 2) * (degree - order + 1)) as f64;
                sum[0] += 0.5
                    * ((-c * solid.v[up][order + 1] - s * solid.w[up][order + 1])
                        + pair * (c * solid.v[up][order - 1] + s * solid.w[up][order - 1]));
                sum[1] += 0.5
                    * ((-c * solid.w[up][order + 1] + s * solid.v[up][order + 1])
                        + pair * (-c * solid.w[up][order - 1] + s * solid.v[up][order - 1]));
            }
            sum[2] +=
                ((degree - order + 1) as f64) * (-c * solid.v[up][order] - s * solid.w[up][order]);
            order += 1;
        }
        degree -= 1;
    }
    let size = satellite.gm_earth_m3_s2 / (satellite.field_radius_m * satellite.field_radius_m);
    [size * sum[0], size * sum[1], size * sum[2]]
}

/// Everything that is not the locked field or the turning of the frame: the
/// Sun, the Moon, the tide they raise, sunlight, relativity.
fn other_acceleration_m_s2(
    satellite: &SatelliteLageos1,
    utc_s: f64,
    position_m: [f64; 3],
    velocity_m_s: [f64; 3],
) -> [f64; 3] {
    let sun_m = sun_position_m(satellite, utc_s);
    let moon_m = moon_position_m(satellite, utc_s);
    let sun = third_body_acceleration_m_s2(position_m, sun_m, satellite.gm_sun_m3_s2);
    let moon = third_body_acceleration_m_s2(position_m, moon_m, satellite.gm_moon_m3_s2);
    let tide = tide_acceleration_m_s2(satellite, position_m, sun_m, moon_m);
    let light = sunlight_acceleration_m_s2(satellite, position_m, sun_m);
    let relativity = relativity_acceleration_m_s2(satellite, position_m, velocity_m_s);
    [
        (((sun[0] + moon[0]) + tide[0]) + light[0]) + relativity[0],
        (((sun[1] + moon[1]) + tide[1]) + light[1]) + relativity[1],
        (((sun[2] + moon[2]) + tide[2]) + light[2]) + relativity[2],
    ]
}

/// Pull of a distant body on the satellite less its pull on the Earth:
/// `GM_b [ (s − r)/|s − r|³ − s/|s|³ ]`.
fn third_body_acceleration_m_s2(position_m: [f64; 3], body_m: [f64; 3], gm_m3_s2: f64) -> [f64; 3] {
    let apart_m = [
        body_m[0] - position_m[0],
        body_m[1] - position_m[1],
        body_m[2] - position_m[2],
    ];
    let apart = dot(apart_m, apart_m).sqrt();
    let apart_cubed = apart * apart * apart;
    let distance = dot(body_m, body_m).sqrt();
    let distance_cubed = distance * distance * distance;
    [
        gm_m3_s2 * (apart_m[0] / apart_cubed - body_m[0] / distance_cubed),
        gm_m3_s2 * (apart_m[1] / apart_cubed - body_m[1] / distance_cubed),
        gm_m3_s2 * (apart_m[2] / apart_cubed - body_m[2] / distance_cubed),
    ]
}

/// The five changes the Sun and the Moon make to the degree-2 coefficients of
/// the solid Earth, IERS Conventions (2010) eq. 6.6, with the permanent part
/// removed, eq. 6.13: `[C20, C21, C22, S21, S22]`, fully normalised.
fn tide_coefficients(satellite: &SatelliteLageos1, sun_m: [f64; 3], moon_m: [f64; 3]) -> [f64; 5] {
    let root_5 = 5.0_f64.sqrt();
    let root_15 = 15.0_f64.sqrt();
    let bodies = [
        (sun_m, satellite.gm_sun_m3_s2),
        (moon_m, satellite.gm_moon_m3_s2),
    ];
    let mut change = [0.0; 5];
    let mut index = 0;
    while index < 2 {
        let (body_m, gm_m3_s2) = bodies[index];
        let distance = dot(body_m, body_m).sqrt();
        let reach = satellite.field_radius_m / distance;
        let size = gm_m3_s2 / satellite.gm_earth_m3_s2 * (reach * reach * reach) / 5.0;
        let x = body_m[0] / distance;
        let y = body_m[1] / distance;
        let z = body_m[2] / distance;
        change[0] += satellite.love_k20 * size * (root_5 * 0.5 * (3.0 * z * z - 1.0));
        change[1] += satellite.love_k21 * size * (root_15 * z * x);
        change[2] += satellite.love_k22 * size * (root_15 * 0.5 * (x * x - y * y));
        change[3] += satellite.love_k21 * size * (root_15 * z * y);
        change[4] += satellite.love_k22 * size * (root_15 * 0.5 * (2.0 * x * y));
        index += 1;
    }
    change[0] -= satellite.permanent_tide_dc20;
    change
}

/// Acceleration of the degree-2 field of those five numbers.
fn tide_acceleration_m_s2(
    satellite: &SatelliteLageos1,
    position_m: [f64; 3],
    sun_m: [f64; 3],
    moon_m: [f64; 3],
) -> [f64; 3] {
    let change = tide_coefficients(satellite, sun_m, moon_m);
    let mut cosine = [[0.0; FIELD_ROWS]; FIELD_ROWS];
    let mut sine = [[0.0; FIELD_ROWS]; FIELD_ROWS];
    cosine[2][0] = change[0];
    cosine[2][1] = change[1];
    cosine[2][2] = change[2];
    sine[2][1] = change[3];
    sine[2][2] = change[4];
    harmonic_acceleration_m_s2(satellite, &cosine, &sine, 2, position_m)
}

/// False when the satellite is behind the Earth as seen from the Sun and
/// within `FIELD_RADIUS_M` of the line from the Sun through the Earth.
fn sunlit(satellite: &SatelliteLageos1, position_m: [f64; 3], sun_m: [f64; 3]) -> bool {
    let sun_distance = dot(sun_m, sun_m).sqrt();
    let toward_sun = [
        sun_m[0] / sun_distance,
        sun_m[1] / sun_distance,
        sun_m[2] / sun_distance,
    ];
    let along_m = dot(position_m, toward_sun);
    if along_m >= 0.0 {
        return true;
    }
    let across_m = [
        position_m[0] - along_m * toward_sun[0],
        position_m[1] - along_m * toward_sun[1],
        position_m[2] - along_m * toward_sun[2],
    ];
    !(dot(across_m, across_m) < satellite.field_radius_m * satellite.field_radius_m)
}

/// Sunlight pressure on a sphere, along the line from the Sun to the
/// satellite. Zero in the Earth's shadow.
fn sunlight_acceleration_m_s2(
    satellite: &SatelliteLageos1,
    position_m: [f64; 3],
    sun_m: [f64; 3],
) -> [f64; 3] {
    if !sunlit(satellite, position_m, sun_m) {
        return [0.0; 3];
    }
    let from_sun_m = [
        position_m[0] - sun_m[0],
        position_m[1] - sun_m[1],
        position_m[2] - sun_m[2],
    ];
    let distance = dot(from_sun_m, from_sun_m).sqrt();
    let pressure_n_m2 = satellite.solar_constant_w_m2 / satellite.speed_of_light_m_s;
    let half_m = satellite.satellite_diameter_m / 2.0;
    let area_over_mass_m2_kg =
        core::f64::consts::PI * (half_m * half_m) / satellite.satellite_mass_kg;
    let nearness = satellite.astronomical_unit_m / distance;
    let size = pressure_n_m2
        * (nearness * nearness)
        * satellite.radiation_coefficient
        * area_over_mass_m2_kg;
    [
        size * from_sun_m[0] / distance,
        size * from_sun_m[1] / distance,
        size * from_sun_m[2] / distance,
    ]
}

/// The relativistic term of one central mass, IERS Conventions (2010)
/// eq. 10.12, first line: `GM/(c² |r|³) [ (4 GM/|r| − u·u) r + 4 (r·u) u ]`
/// with `u` the velocity seen from a frame that does not turn.
fn relativity_acceleration_m_s2(
    satellite: &SatelliteLageos1,
    position_m: [f64; 3],
    velocity_m_s: [f64; 3],
) -> [f64; 3] {
    let u = fixed_frame_velocity_m_s(satellite, position_m, velocity_m_s);
    let gm = satellite.gm_earth_m3_s2;
    let radius = dot(position_m, position_m).sqrt();
    let light_squared = satellite.speed_of_light_m_s * satellite.speed_of_light_m_s;
    let size = gm / (light_squared * (radius * radius * radius));
    let along_position = 4.0 * gm / radius - dot(u, u);
    let along_velocity = 4.0 * dot(position_m, u);
    [
        size * (along_position * position_m[0] + along_velocity * u[0]),
        size * (along_position * position_m[1] + along_velocity * u[1]),
        size * (along_position * position_m[2] + along_velocity * u[2]),
    ]
}

/// Julian centuries of Terrestrial Time from 2000-01-01 12:00.
fn centuries_tt(satellite: &SatelliteLageos1, utc_s: f64) -> f64 {
    (utc_s + satellite.tt_minus_utc_s) / SECONDS_PER_JULIAN_CENTURY
}

/// The Sun in the card's frame at `utc_s`, metres from the Earth's centre.
fn sun_position_m(satellite: &SatelliteLageos1, utc_s: f64) -> [f64; 3] {
    let centuries = centuries_tt(satellite, utc_s);
    let equinox_shift_rad =
        satellite.precession_in_longitude_deg_per_century * centuries * RADIANS_PER_DEGREE;
    let anomaly_rad = (satellite.sun_mean_anomaly_deg[0]
        + satellite.sun_mean_anomaly_deg[1] * centuries)
        * RADIANS_PER_DEGREE;
    let distance_m = (satellite.sun_distance_gm[0]
        + satellite.sun_distance_gm[1] * crate::math::cos(anomaly_rad)
        + satellite.sun_distance_gm[2] * crate::math::cos(2.0 * anomaly_rad))
        * 1.0e9;
    let longitude_rad = satellite.sun_longitude_offset_deg * RADIANS_PER_DEGREE
        + anomaly_rad
        + (satellite.sun_longitude_arcsec[0] * crate::math::sin(anomaly_rad)
            + satellite.sun_longitude_arcsec[1] * crate::math::sin(2.0 * anomaly_rad))
            / ARCSECONDS_PER_DEGREE
            * RADIANS_PER_DEGREE
        + equinox_shift_rad;
    let in_orbit_plane_m = [
        distance_m * crate::math::cos(longitude_rad),
        distance_m * crate::math::sin(longitude_rad),
        0.0,
    ];
    orbit_plane_to_card_frame(satellite, utc_s, in_orbit_plane_m)
}

/// The Moon in the card's frame at `utc_s`, metres from the Earth's centre.
fn moon_position_m(satellite: &SatelliteLageos1, utc_s: f64) -> [f64; 3] {
    let centuries = centuries_tt(satellite, utc_s);
    let mut angle_rad = [0.0; 5];
    let mut index = 0;
    while index < 5 {
        angle_rad[index] = (satellite.moon_arguments_deg[index][0]
            + satellite.moon_arguments_deg[index][1] * centuries)
            * RADIANS_PER_DEGREE;
        index += 1;
    }
    let mean_longitude_rad = angle_rad[0];
    let four = [angle_rad[1], angle_rad[2], angle_rad[3], angle_rad[4]];
    let distance_m = cosine_terms(
        &satellite.moon_distance_terms_km,
        four,
        satellite.moon_distance_constant_km,
    ) * 1.0e3;
    let longitude_rad = mean_longitude_rad
        + sine_terms(&satellite.moon_longitude_terms_arcsec, four, 0.0) * RADIANS_PER_ARCSECOND;
    let lead = satellite.moon_latitude_lead_arcsec;
    let lead_argument_rad = four[2] + longitude_rad - mean_longitude_rad
        + lead[1] * crate::math::sin(2.0 * four[2]) * RADIANS_PER_ARCSECOND
        + lead[2] * crate::math::sin(four[1]) * RADIANS_PER_ARCSECOND;
    let latitude_rad = sine_terms(
        &satellite.moon_latitude_terms_arcsec,
        four,
        lead[0] * crate::math::sin(lead_argument_rad),
    ) * RADIANS_PER_ARCSECOND;
    let in_plane_m = distance_m * crate::math::cos(latitude_rad);
    let in_orbit_plane_m = [
        in_plane_m * crate::math::cos(longitude_rad),
        in_plane_m * crate::math::sin(longitude_rad),
        distance_m * crate::math::sin(latitude_rad),
    ];
    orbit_plane_to_card_frame(satellite, utc_s, in_orbit_plane_m)
}

/// `start + Σ amplitude × cos(argument)`. Each row is an amplitude and four
/// whole-number multipliers of the four angles.
fn cosine_terms(terms: &[[f64; 5]], angle_rad: [f64; 4], start: f64) -> f64 {
    let mut sum = start;
    let mut row = 0;
    while row < terms.len() {
        sum += terms[row][0] * crate::math::cos(term_argument_rad(&terms[row], angle_rad));
        row += 1;
    }
    sum
}

/// `start + Σ amplitude × sin(argument)`.
fn sine_terms(terms: &[[f64; 5]], angle_rad: [f64; 4], start: f64) -> f64 {
    let mut sum = start;
    let mut row = 0;
    while row < terms.len() {
        sum += terms[row][0] * crate::math::sin(term_argument_rad(&terms[row], angle_rad));
        row += 1;
    }
    sum
}

fn term_argument_rad(term: &[f64; 5], angle_rad: [f64; 4]) -> f64 {
    term[1] * angle_rad[0]
        + term[2] * angle_rad[1]
        + term[3] * angle_rad[2]
        + term[4] * angle_rad[3]
}

/// From axes in the plane of the Earth's orbit, x toward the equinox of the
/// date, to the card's frame: a turn about x by the obliquity, then a turn
/// about z by sidereal time.
fn orbit_plane_to_card_frame(
    satellite: &SatelliteLageos1,
    utc_s: f64,
    in_orbit_plane_m: [f64; 3],
) -> [f64; 3] {
    let obliquity_rad = satellite.obliquity_deg * RADIANS_PER_DEGREE;
    let tilt_cos = crate::math::cos(obliquity_rad);
    let tilt_sin = crate::math::sin(obliquity_rad);
    let equatorial_m = [
        in_orbit_plane_m[0],
        tilt_cos * in_orbit_plane_m[1] - tilt_sin * in_orbit_plane_m[2],
        tilt_sin * in_orbit_plane_m[1] + tilt_cos * in_orbit_plane_m[2],
    ];
    let days = utc_s / SECONDS_PER_DAY;
    let day_fraction = days - ((days as i64) as f64);
    let rotation_angle_rad = RADIANS_PER_TURN
        * (day_fraction + satellite.era_at_j2000_rev + satellite.era_rate_minus_one * days);
    let sidereal_rad = rotation_angle_rad
        + satellite.gmst_precession_arcsec_per_century
            * centuries_tt(satellite, utc_s)
            * RADIANS_PER_ARCSECOND;
    let turn_cos = crate::math::cos(sidereal_rad);
    let turn_sin = crate::math::sin(sidereal_rad);
    [
        turn_cos * equatorial_m[0] + turn_sin * equatorial_m[1],
        -turn_sin * equatorial_m[0] + turn_cos * equatorial_m[1],
        equatorial_m[2],
    ]
}

fn axpy(y: [f64; 7], scale: f64, k: [f64; 7]) -> [f64; 7] {
    let mut out = y;
    let mut index = 0;
    while index < 7 {
        out[index] += scale * k[index];
        index += 1;
    }
    out
}

fn recorded(state: &SatelliteState) -> [f64; 7] {
    [
        state.position_m[0],
        state.position_m[1],
        state.position_m[2],
        state.velocity_m_s[0],
        state.velocity_m_s[1],
        state.velocity_m_s[2],
        state.external_work_j_per_kg,
    ]
}

fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn finite3(x: [f64; 3]) -> bool {
    x[0].is_finite() && x[1].is_finite() && x[2].is_finite()
}

fn finite7(x: [f64; 7]) -> bool {
    let mut index = 0;
    while index < 7 {
        if !x[index].is_finite() {
            return false;
        }
        index += 1;
    }
    true
}

fn same_bits(recorded: f64, locked: f64) -> bool {
    recorded.to_bits() == locked.to_bits()
}

fn same_row(recorded: &[f64], locked: &[f64]) -> bool {
    if recorded.len() != locked.len() {
        return false;
    }
    let mut index = 0;
    while index < locked.len() {
        if !same_bits(recorded[index], locked[index]) {
            return false;
        }
        index += 1;
    }
    true
}

fn abs_f64(value: f64) -> f64 {
    if value < 0.0 { -value } else { value }
}

fn state_is_finite(state: &SatelliteState) -> bool {
    state.t_s.is_finite()
        && state.start_epoch_utc_s.is_finite()
        && state.forecast_elapsed_at_start_s.is_finite()
        && finite3(state.position_m)
        && finite3(state.velocity_m_s)
        && state.jacobi_j_per_kg.is_finite()
        && state.jacobi_anchor_j_per_kg.is_finite()
        && state.external_work_j_per_kg.is_finite()
        && state.energy_defect_j_per_kg.is_finite()
        && state.residual_conservation_error.is_finite()
        && state.radius_m.is_finite()
        && state.semi_major_axis_m.is_finite()
}

#[cfg(test)]
mod tests {
    extern crate core;
    extern crate std;

    use core::prelude::v1::*;
    use std::assert;
    use std::assert_eq;
    use std::concat;
    use std::format;
    use std::include_str;
    use std::string::String;
    use std::vec::Vec;

    use super::*;
    use crate::frame::LastStateFrame64;

    const DT: f64 = GATE_DT_S;
    const NO_LOAD: SatelliteLoad = SatelliteLoad {};

    // <test reference: generated text begins>
    // Written by `08_card_tables_and_reference.py`. Stand-in values are 64-bit results of
    // the Python stand-in. High-precision values were computed with 60 digits and rounded.
    #[cfg_attr(rustfmt, rustfmt::skip)]
    const START_EPOCH_UTC_S: f64 = 840715200.0;
    #[cfg_attr(rustfmt, rustfmt::skip)]
    const START_POSITION_M: [f64; 3] = [3767126.6844992605, 8956617.400906362, 7564717.856833777];
    #[cfg_attr(rustfmt, rustfmt::skip)]
    const START_VELOCITY_M_S: [f64; 3] = [4119.825668508274, 1676.7840658085788, -4053.2216929389388];
    #[cfg_attr(rustfmt, rustfmt::skip)]
    const STAND_IN_JACOBI_ANCHOR_J_PER_KG: f64 = -14513184.64982171;
    /// Steps of 10 s, position, velocity, work of the other forces: the stand-in.
    #[cfg_attr(rustfmt, rustfmt::skip)]
    const STAND_IN_STATES: [(u32, [f64; 3], [f64; 3], f64); 5] = [
        (1, [3808297.775005862, 8973261.975195887, 7524105.0043151975], [4114.372117134941, 1652.1262395982358, -4069.334443883426], -0.03394236851044896),
        (12, [4257272.5725825755, 9140013.996748488, 7066949.787150934], [4046.3572987569196, 1379.2608573756665, -4240.799016235441], -0.4111773216358437),
        (1356, [9658146.730467925, 1729619.4895086086, 7440246.154718623], [3601.0968215846624, -2535.7491920957195, -4102.130675262415], -0.13148031990108527),
        (8640, [2018743.029663631, -3983911.5264663887, -11378818.754372645], [-4125.733517267737, -4317.820781808597, 784.5502926109502], 9.828595407014186),
        (60480, [-8756140.995484363, -5928363.936934775, 6250435.626571182], [501.3986472392647, 3977.6619411508095, 4516.422203344968], 321.68918066719476),
    ];
    /// The stand-in's field acceleration at the start.
    #[cfg_attr(rustfmt, rustfmt::skip)]
    const STAND_IN_FIELD_AT_START_M_S2: [f64; 3] = [-0.8038366101201173, -1.9111935235914486, -1.6155822991793172];
    /// The sum of the stand-in's other forces at the start.
    #[cfg_attr(rustfmt, rustfmt::skip)]
    const STAND_IN_OTHER_FORCES_AT_START_M_S2: [f64; 3] = [-9.96033789115604e-07, 4.627175508141881e-07, 1.5576556173362897e-08];
    #[cfg_attr(rustfmt, rustfmt::skip)]
    const STAND_IN_SUN_AT_START_M: [f64; 3] = [-148231960299.67767, -1594107316.978218, 30253750967.53513];
    #[cfg_attr(rustfmt, rustfmt::skip)]
    const STAND_IN_MOON_AT_START_M: [f64; 3] = [180766077.0386719, -308218288.50191903, -189443465.48916864];
    /// The stand-in's position one day on with the sign of the Coriolis term flipped.
    #[cfg_attr(rustfmt, rustfmt::skip)]
    const STAND_IN_FLIPPED_CORIOLIS_ONE_DAY_M: [f64; 3] = [-2313602.6232886016, 6026230.2597044995, 11066832.477814311];
    /// Position, field acceleration with the point mass, potential with the point mass: high precision.
    #[cfg_attr(rustfmt, rustfmt::skip)]
    const FIELD_REFERENCE: [([f64; 3], [f64; 3], f64); 5] = [
        // the start position
        ([3767126.6844992605, 8956617.400906362, 7564717.856833777], [-0.8038366101201173, -1.9111935235914486, -1.6155822991793172], 32368753.594894286),
        // the stand-in position after 1,356 steps
        ([9658146.730467925, 1729619.4895086086, 7440246.154718623], [-2.0611291150027156, -0.3691206748431343, -1.5891942687296903], 32369922.69798325),
        // the stand-in position after one day
        ([2018743.029663631, -3983911.5264663887, -11378818.754372645], [-0.439886884820477, 0.8681113636567723, 2.4816861240607615], 32600527.385795385),
        // on the axis of rotation, 12,270 km
        ([0.0, 0.0, 12270000.0], [3.967412049013883e-06, -3.083414838124057e-07, -2.645259153526912], 32476285.76353663),
        // in the equatorial plane, 12,267 km
        ([-8000000.0, 9300000.0, 0.0], [1.7280553340649714, -2.0088783937383257, 4.952116364399614e-07], 32497364.193797998),
    ];
    /// Epoch, Sun, Moon in the card frame: the series in high precision.
    #[cfg_attr(rustfmt, rustfmt::skip)]
    const BODY_REFERENCE: [(f64, [f64; 3], [f64; 3]); 4] = [
        // the start epoch
        (840715200.0, [-148231960299.67743, -1594107316.9787445, 30253750967.536366], [180766077.0387622, -308218288.5018687, -189443465.48916817]),
        // 12 h 10 s later
        (840758410.0, [148307395327.29236, 1402517744.2199006, 29812023830.41521], [-213542745.8154516, 288089415.82222825, -185988329.74510437]),
        // the first epoch of the card, 2017-01-01
        (536500800.0, [-135386452384.8862, -1907027295.4161954, -57494764684.45982], [-316671146.11281025, -204963663.7206244, -103457520.24797061]),
        // the last epoch of the card, 2027-06-28
        (867412800.0, [-139661307692.82788, -1707783975.825368, 60157689737.65077], [-43472604.945302434, 380122507.1625227, 70011487.41893962]),
    ];
    /// Epoch, position, velocity, then the Sun's pull, the Moon's pull, the tide, sunlight,
    /// the relativistic term and their sum: closed forms in high precision.
    #[cfg_attr(rustfmt, rustfmt::skip)]
    const FORCE_REFERENCE: [(f64, [f64; 3], [f64; 3], [[f64; 3]; 6]); 3] = [
        // the start state
        (840715200.0, [3767126.6844992605, 8956617.400906362, 7564717.856833777], [4119.825668508274, 1676.7840658085788, -4053.2216929389388], [
            [1.1165647745895052e-07, -3.4045051175581995e-07, -3.421185542138123e-07],
            [-1.0976026366670964e-06, 8.027029761345801e-07, 3.5024572956693797e-07],
            [-1.4383921690364057e-08, -1.6379607626943995e-09, 6.397767637485134e-09],
            [3.4304579743945294e-09, 3.70979624995517e-11, -6.999545427857709e-10],
            [8.658336175952918e-10, 2.065949128366078e-09, 1.7515677137138576e-09],
            [-9.9603378930652e-07, 4.627175507069315e-07, 1.557655616153893e-08],
        ]),
        // the stand-in state after 1,356 steps
        (840728760.0, [9658146.730467925, 1729619.4895086086, 7440246.154718623], [3601.0968215846624, -2535.7491920957195, -4102.130675262415], [
            [-2.170348104837412e-07, -2.9233795251389205e-07, -3.4057998660885136e-07],
            [-1.9113616971153028e-08, 1.3377786293251986e-06, 3.2418665575371113e-07],
            [-7.790062097693704e-09, 1.1190174697787832e-08, 7.929521632871463e-09],
            [1.924311018962342e-09, -2.8411903274127492e-09, -6.968204496676637e-10],
            [2.2262627153185455e-09, 4.0164475954660295e-10, 1.7231915352436428e-09],
            [-2.3978791581830705e-07, 1.0541913059412282e-06, -7.437438136692811e-09],
        ]),
        // the stand-in state after one day
        (840801600.0, [2018743.029663631, -3983911.5264663887, -11378818.754372645], [-4125.733517267737, -4317.820781808597, 784.5502926109502], [
            [3.911175764289863e-07, 1.5724757972859219e-07, 3.4354171412243373e-07],
            [1.0974104315285745e-06, -1.0444176722245282e-06, -1.8687769854979153e-08],
            [1.0094039506913711e-08, 3.3881560846534103e-09, 2.25922582832105e-08],
            [3.4358467056512583e-09, 3.2913669462941444e-11, -6.803020336163245e-10],
            [4.809852237281149e-10, -9.430220969339653e-10, -2.700140746527082e-09],
            [1.502538879393854e-06, -8.846920448387536e-07, 3.440657597705217e-07],
        ]),
    ];
    /// The tide's five coefficient changes at the start: C20, C21, C22, S21, S22.
    #[cfg_attr(rustfmt, rustfmt::skip)]
    const TIDE_COEFFICIENTS_AT_START: [f64; 5] = [1.6057253317515813e-09, -3.46315742766083e-09, 6.480342498090282e-10, 3.967437105641144e-09, -3.77188168527529e-09];
    /// `out/card_gravity_coefficients_degree_12.csv`, byte for byte.
    #[cfg_attr(rustfmt, rustfmt::skip)]
    const GRAVITY_TABLE_TEXT: &str = "\
degree,order,normalised_C,normalised_S,source\n\
2,0,-0.48416948e-03,0.0,IERS Conventions (2010) Table 6.2; zero-tide; at 2000.0\n\
2,1,-0.206615509074176e-09,0.138441389137979e-08,EGM2008\n\
2,2,0.243938357328313e-05,-0.140027370385934e-05,EGM2008\n\
3,0,0.9571612e-06,0.0,IERS Conventions (2010) Table 6.2; at 2000.0\n\
3,1,0.203046201047864e-05,0.248200415856872e-06,EGM2008\n\
3,2,0.904787894809528e-06,-0.619005475177618e-06,EGM2008\n\
3,3,0.721321757121568e-06,0.141434926192941e-05,EGM2008\n\
4,0,0.5399659e-06,0.0,IERS Conventions (2010) Table 6.2; at 2000.0\n\
4,1,-0.536157389388867e-06,-0.473567346518086e-06,EGM2008\n\
4,2,0.350501623962649e-06,0.662480026275829e-06,EGM2008\n\
4,3,0.990856766672321e-06,-0.200956723567452e-06,EGM2008\n\
4,4,-0.188519633023033e-06,0.308803882149194e-06,EGM2008\n\
5,0,0.686702913736681e-07,0.000000000000000e+00,EGM2008\n\
5,1,-0.629211923042529e-07,-0.943698073395769e-07,EGM2008\n\
5,2,0.652078043176164e-06,-0.323353192540522e-06,EGM2008\n\
5,3,-0.451847152328843e-06,-0.214955408306046e-06,EGM2008\n\
5,4,-0.295328761175629e-06,0.498070550102351e-07,EGM2008\n\
5,5,0.174811795496002e-06,-0.669379935180165e-06,EGM2008\n\
6,0,-0.149953927978527e-06,0.000000000000000e+00,EGM2008\n\
6,1,-0.759210081892527e-07,0.265122593213647e-07,EGM2008\n\
6,2,0.486488924604690e-07,-0.373789324523752e-06,EGM2008\n\
6,3,0.572451611175653e-07,0.895201130010730e-08,EGM2008\n\
6,4,-0.860237937191611e-07,-0.471425573429095e-06,EGM2008\n\
6,5,-0.267166423703038e-06,-0.536493151500206e-06,EGM2008\n\
6,6,0.947068749756882e-08,-0.237382353351005e-06,EGM2008\n\
7,0,0.905120844521618e-07,0.000000000000000e+00,EGM2008\n\
7,1,0.280887555776673e-06,0.951259362869275e-07,EGM2008\n\
7,2,0.330407993702235e-06,0.929969290624092e-07,EGM2008\n\
7,3,0.250458409225729e-06,-0.217118287729610e-06,EGM2008\n\
7,4,-0.274993935591631e-06,-0.124058403514343e-06,EGM2008\n\
7,5,0.164773255934658e-08,0.179281782751438e-07,EGM2008\n\
7,6,-0.358798423464889e-06,0.151798257443669e-06,EGM2008\n\
7,7,0.150746472872675e-08,0.241068767286303e-07,EGM2008\n\
8,0,0.494756003005199e-07,0.000000000000000e+00,EGM2008\n\
8,1,0.231607991248329e-07,0.588974540927606e-07,EGM2008\n\
8,2,0.800143604736599e-07,0.652805043667369e-07,EGM2008\n\
8,3,-0.193745381715290e-07,-0.859639339125694e-07,EGM2008\n\
8,4,-0.244360480007096e-06,0.698072508472777e-07,EGM2008\n\
8,5,-0.257011477267991e-07,0.892034891745881e-07,EGM2008\n\
8,6,-0.659648680031408e-07,0.308946730783065e-06,EGM2008\n\
8,7,0.672569751771483e-07,0.748686063738231e-07,EGM2008\n\
8,8,-0.124022771917136e-06,0.120551889384997e-06,EGM2008\n\
9,0,0.280180753216300e-07,0.000000000000000e+00,EGM2008\n\
9,1,0.142151377236084e-06,0.214004665077510e-07,EGM2008\n\
9,2,0.214144381199757e-07,-0.316984195352417e-07,EGM2008\n\
9,3,-0.160612356882835e-06,-0.742658786809216e-07,EGM2008\n\
9,4,-0.936529556592536e-08,0.199026740710063e-07,EGM2008\n\
9,5,-0.163134050605937e-07,-0.540394840426217e-07,EGM2008\n\
9,6,0.627879491161446e-07,0.222962377434615e-06,EGM2008\n\
9,7,-0.117983924385618e-06,-0.969222126840068e-07,EGM2008\n\
9,8,0.188136188986452e-06,-0.300538974811744e-08,EGM2008\n\
9,9,-0.475568433357652e-07,0.968804214389955e-07,EGM2008\n\
10,0,0.533304381729473e-07,0.000000000000000e+00,EGM2008\n\
10,1,0.837623112620412e-07,-0.131092332261065e-06,EGM2008\n\
10,2,-0.939894766092874e-07,-0.512746772537482e-07,EGM2008\n\
10,3,-0.700709997317429e-08,-0.154139929404373e-06,EGM2008\n\
10,4,-0.844715388074630e-07,-0.790255527979406e-07,EGM2008\n\
10,5,-0.492894049964295e-07,-0.506137282060864e-07,EGM2008\n\
10,6,-0.375849022022301e-07,-0.797688616388143e-07,EGM2008\n\
10,7,0.826209286523474e-08,-0.304903703914366e-08,EGM2008\n\
10,8,0.405981624580941e-07,-0.917138622482163e-07,EGM2008\n\
10,9,0.125376631604340e-06,-0.379436584841270e-07,EGM2008\n\
10,10,0.100435991936118e-06,-0.238596204211893e-07,EGM2008\n\
11,0,-0.507683787085927e-07,0.000000000000000e+00,EGM2008\n\
11,1,0.156127678638183e-07,-0.271235374123689e-07,EGM2008\n\
11,2,0.201135250154855e-07,-0.990003954905590e-07,EGM2008\n\
11,3,-0.305773531606647e-07,-0.148835345047152e-06,EGM2008\n\
11,4,-0.379499015091407e-07,-0.637669897493018e-07,EGM2008\n\
11,5,0.374192407050580e-07,0.495908160271967e-07,EGM2008\n\
11,6,-0.156429128694775e-08,0.342735099884706e-07,EGM2008\n\
11,7,0.465461661449953e-08,-0.898252194924903e-07,EGM2008\n\
11,8,-0.630174049861897e-08,0.245446551115189e-07,EGM2008\n\
11,9,-0.310727993686101e-07,0.420682585407293e-07,EGM2008\n\
11,10,-0.522444922089646e-07,-0.184216383163730e-07,EGM2008\n\
11,11,0.462340571475799e-07,-0.696711251523700e-07,EGM2008\n\
12,0,0.364361922614572e-07,0.000000000000000e+00,EGM2008\n\
12,1,-0.535856270449833e-07,-0.431656037232084e-07,EGM2008\n\
12,2,0.142665936828290e-07,0.310937162901519e-07,EGM2008\n\
12,3,0.396211271409354e-07,0.250622628960907e-07,EGM2008\n\
12,4,-0.677284618097416e-07,0.383823469584472e-08,EGM2008\n\
12,5,0.308775410911475e-07,0.759066416791107e-08,EGM2008\n\
12,6,0.313421100991039e-08,0.389801868153392e-07,EGM2008\n\
12,7,-0.190517957483100e-07,0.357268620672699e-07,EGM2008\n\
12,8,-0.258866871220994e-07,0.169362538600173e-07,EGM2008\n\
12,9,0.419147664170774e-07,0.249625636010847e-07,EGM2008\n\
12,10,-0.619955079880774e-08,0.309398171578482e-07,EGM2008\n\
12,11,0.113644952089825e-07,-0.638551119140755e-08,EGM2008\n\
12,12,-0.242377235648074e-08,-0.110993698692881e-07,EGM2008\n\
";
    #[cfg_attr(rustfmt, rustfmt::skip)]
    const GRAVITY_TABLE_SHA256: &str = "3ffe678122ac51affc23a49ed6424ed758227bf6ac041760a63d11614e5012cb";
    #[cfg_attr(rustfmt, rustfmt::skip)]
    const GENERATED_TABLES_SHA256: &str = "b5c235ffb485c0577e190aae4e3620483c762bd198beefc410be2606eb9634c8";
    // <test reference: generated text ends>
    // The last two epochs of BODY_REFERENCE, 2017-01-01 and 2027-06-28, were
    // the first and the last dates of the card up to engine 0.6.0. The Sun and
    // Moon series is still tested there; the card's dates are now the 28
    // compared days, EPOCH_MIN_S to EPOCH_MAX_S.

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

    fn state_bits(state: &SatelliteState) -> [u64; 16] {
        [
            state.t_s.to_bits(),
            state.start_epoch_utc_s.to_bits(),
            state.position_m[0].to_bits(),
            state.position_m[1].to_bits(),
            state.position_m[2].to_bits(),
            state.velocity_m_s[0].to_bits(),
            state.velocity_m_s[1].to_bits(),
            state.velocity_m_s[2].to_bits(),
            state.jacobi_j_per_kg.to_bits(),
            state.jacobi_anchor_j_per_kg.to_bits(),
            state.external_work_j_per_kg.to_bits(),
            state.energy_defect_j_per_kg.to_bits(),
            state.residual_conservation_error.to_bits(),
            state.radius_m.to_bits(),
            state.semi_major_axis_m.to_bits(),
            u64::from(state.in_earth_shadow),
        ]
    }

    /// A state taken from a published orbit: no time has passed since it.
    fn fresh(
        satellite: &SatelliteLageos1,
        epoch_utc_s: f64,
        position_m: [f64; 3],
        velocity_m_s: [f64; 3],
    ) -> Option<SatelliteState> {
        SatelliteState::try_state(satellite, epoch_utc_s, 0.0, position_m, velocity_m_s)
    }

    /// The first start of the opened week: 2026-08-23 0 h UTC.
    fn start(satellite: &SatelliteLageos1) -> SatelliteState {
        fresh(
            satellite,
            START_EPOCH_UTC_S,
            START_POSITION_M,
            START_VELOCITY_M_S,
        )
        .unwrap()
    }

    fn size(a: [f64; 3]) -> f64 {
        dot(a, a).sqrt()
    }

    fn apart(a: [f64; 3], b: [f64; 3]) -> f64 {
        size([a[0] - b[0], a[1] - b[1], a[2] - b[2]])
    }

    /// Signed `(J − J_anchor − W) / LEDGER_SCALE_J_PER_KG` of a trial `(r, v, W)`.
    fn signed_residual(satellite: &SatelliteLageos1, anchor_j_per_kg: f64, y: [f64; 7]) -> f64 {
        (jacobi_j_per_kg(satellite, [y[0], y[1], y[2]], [y[3], y[4], y[5]])
            - anchor_j_per_kg
            - y[6])
            / LEDGER_SCALE_J_PER_KG
    }

    /// The card's rates with a chosen sign on the Coriolis term and on the
    /// centrifugal term. `(1, 1)` is the card.
    fn rates_with_signs(
        satellite: &SatelliteLageos1,
        utc_s: f64,
        y: [f64; 7],
        coriolis_sign: f64,
        centrifugal_sign: f64,
    ) -> Result<[f64; 7], StepReject> {
        let position_m = [y[0], y[1], y[2]];
        let velocity_m_s = [y[3], y[4], y[5]];
        let field = field_acceleration_m_s2(satellite, position_m);
        let other = other_acceleration_m_s2(satellite, utc_s, position_m, velocity_m_s);
        let w = satellite.earth_rotation_rate_rad_s;
        Ok([
            velocity_m_s[0],
            velocity_m_s[1],
            velocity_m_s[2],
            (field[0] + other[0])
                + (coriolis_sign * 2.0 * w * velocity_m_s[1]
                    + centrifugal_sign * w * w * position_m[0]),
            (field[1] + other[1])
                + (-coriolis_sign * 2.0 * w * velocity_m_s[0]
                    + centrifugal_sign * w * w * position_m[1]),
            (field[2] + other[2]) + 0.0,
            dot(other, velocity_m_s),
        ])
    }

    /// Classical RK4 with those signs.
    fn rk4_with_signs(
        satellite: &SatelliteLageos1,
        utc_s: f64,
        y: [f64; 7],
        dt_s: f64,
        coriolis_sign: f64,
        centrifugal_sign: f64,
    ) -> Result<[f64; 7], StepReject> {
        let f = |at_s: f64, at: [f64; 7]| {
            rates_with_signs(satellite, at_s, at, coriolis_sign, centrifugal_sign)
        };
        let k1 = f(utc_s, y)?;
        let k2 = f(utc_s + 0.5 * dt_s, axpy(y, dt_s * 0.5, k1))?;
        let k3 = f(utc_s + 0.5 * dt_s, axpy(y, dt_s * 0.5, k2))?;
        let k4 = f(utc_s + dt_s, axpy(y, dt_s, k3))?;
        let mut next = y;
        let mut index = 0;
        while index < 7 {
            next[index] +=
                (dt_s / 6.0) * (k1[index] + 2.0 * k2[index] + 2.0 * k3[index] + k4[index]);
            index += 1;
        }
        Ok(next)
    }

    /// Explicit Euler on `(r, v, W)`, through the real `validate` and `commit`.
    fn step_euler(
        satellite: &SatelliteLageos1,
        state: &mut SatelliteState,
        frame: &mut LastStateFrame64,
        dt_s: f64,
    ) -> Result<(), StepReject> {
        validate(satellite, state, NO_LOAD, dt_s)?;
        let y = recorded(state);
        let begin = rates(satellite, state.epoch_utc_s(), y)?;
        commit(satellite, state, frame, dt_s, axpy(y, dt_s, begin))
    }

    /// The midpoint rule, second order, through the real `validate` and `commit`.
    fn step_midpoint(
        satellite: &SatelliteLageos1,
        state: &mut SatelliteState,
        frame: &mut LastStateFrame64,
        dt_s: f64,
    ) -> Result<(), StepReject> {
        validate(satellite, state, NO_LOAD, dt_s)?;
        let y = recorded(state);
        let utc_s = state.epoch_utc_s();
        let begin = rates(satellite, utc_s, y)?;
        let middle = rates(satellite, utc_s + 0.5 * dt_s, axpy(y, dt_s * 0.5, begin))?;
        commit(satellite, state, frame, dt_s, axpy(y, dt_s, middle))
    }

    /// RK4 with a chosen sign on the two frame terms, through the real
    /// `validate` and `commit`.
    fn step_with_signs(
        satellite: &SatelliteLageos1,
        state: &mut SatelliteState,
        frame: &mut LastStateFrame64,
        dt_s: f64,
        coriolis_sign: f64,
        centrifugal_sign: f64,
    ) -> Result<(), StepReject> {
        validate(satellite, state, NO_LOAD, dt_s)?;
        let predicted = rk4_with_signs(
            satellite,
            state.epoch_utc_s(),
            recorded(state),
            dt_s,
            coriolis_sign,
            centrifugal_sign,
        )?;
        commit(satellite, state, frame, dt_s, predicted)
    }

    /// Honest RK4, then `extra_j_per_kg` added to the booked work.
    fn step_bias_work(
        satellite: &SatelliteLageos1,
        state: &mut SatelliteState,
        frame: &mut LastStateFrame64,
        dt_s: f64,
        extra_j_per_kg: f64,
    ) -> Result<(), StepReject> {
        validate(satellite, state, NO_LOAD, dt_s)?;
        let mut predicted = integrate(
            satellite,
            state.epoch_utc_s(),
            recorded(state),
            dt_s,
        )?;
        predicted[6] += extra_j_per_kg;
        commit(satellite, state, frame, dt_s, predicted)
    }

    /// A copy of the card with a point mass for the Earth and nothing else.
    fn point_mass_only(satellite: &SatelliteLageos1) -> SatelliteLageos1 {
        let mut alone = *satellite;
        alone.field_c = [[0.0; FIELD_ROWS]; FIELD_ROWS];
        alone.field_s = [[0.0; FIELD_ROWS]; FIELD_ROWS];
        alone.gm_sun_m3_s2 = 0.0;
        alone.gm_moon_m3_s2 = 0.0;
        alone.permanent_tide_dc20 = 0.0;
        alone.radiation_coefficient = 0.0;
        alone.speed_of_light_m_s = f64::INFINITY;
        alone
    }

    /// The velocity in the turning frame of a velocity seen from a frame that does not turn.
    fn turning_frame_velocity_m_s(
        satellite: &SatelliteLageos1,
        position_m: [f64; 3],
        fixed_m_s: [f64; 3],
    ) -> [f64; 3] {
        let w = satellite.earth_rotation_rate_rad_s;
        [
            fixed_m_s[0] + w * position_m[1],
            fixed_m_s[1] - w * position_m[0],
            fixed_m_s[2],
        ]
    }

    fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
        [
            a[1] * b[2] - a[2] * b[1],
            a[2] * b[0] - a[0] * b[2],
            a[0] * b[1] - a[1] * b[0],
        ]
    }

    /// A position `radius_m` out along the unit vector `toward`, on an orbit
    /// of semi-major axis `a_m` and eccentricity `e` inclined
    /// `inclination_deg` to the z axis, moving outward: the position and the
    /// velocity in the turning frame. With `e` equal to |1 − radius ÷ a| the
    /// position is an apsis and the velocity is at right angles to it.
    fn on_orbit(
        satellite: &SatelliteLageos1,
        toward: [f64; 3],
        radius_m: f64,
        a_m: f64,
        e: f64,
        inclination_deg: f64,
    ) -> ([f64; 3], [f64; 3]) {
        let d = toward;
        // The unit vector at right angles to `toward` in the plane it makes
        // with the z axis, and the one at right angles to both.
        let up = [-d[2] * d[0], -d[2] * d[1], 1.0 - d[2] * d[2]];
        let up_size = size(up);
        let e1 = [up[0] / up_size, up[1] / up_size, up[2] / up_size];
        let e2 = cross(d, e1);
        // The orbit's pole: at right angles to `toward`, its z part the
        // cosine of the inclination.
        let cos_turn = crate::math::cos(inclination_deg * RADIANS_PER_DEGREE) / up_size;
        let sin_turn = (1.0 - cos_turn * cos_turn).sqrt();
        let pole = [
            cos_turn * e1[0] + sin_turn * e2[0],
            cos_turn * e1[1] + sin_turn * e2[1],
            cos_turn * e1[2] + sin_turn * e2[2],
        ];
        let along = cross(pole, d);
        let gm = satellite.gm_earth_m3_s2;
        let speed_squared = gm * (2.0 / radius_m - 1.0 / a_m);
        let across = (gm * a_m * (1.0 - e * e)).sqrt() / radius_m;
        let out_squared = speed_squared - across * across;
        let out = if out_squared > 0.0 { out_squared.sqrt() } else { 0.0 };
        let position_m = [d[0] * radius_m, d[1] * radius_m, d[2] * radius_m];
        let fixed_m_s = [
            along[0] * across + d[0] * out,
            along[1] * across + d[1] * out,
            along[2] * across + d[2] * out,
        ];
        (position_m, turning_frame_velocity_m_s(satellite, position_m, fixed_m_s))
    }

    /// Point mass and the zonal terms of degree 2, 3 and 4, written out in
    /// Cartesian closed form from the three locked zonal coefficients.
    fn zonal_closed_form_m_s2(position_m: [f64; 3]) -> [f64; 3] {
        let [x, y, z] = position_m;
        let r2 = x * x + y * y + z * z;
        let r = r2.sqrt();
        let s2 = (z / r) * (z / r);
        let s4 = s2 * s2;
        let g = GM_EARTH_M3_S2 / r2;
        let q = FIELD_RADIUS_M / r;
        let f2 = 1.5 * (-(5.0_f64.sqrt()) * FIELD_C[2][0]) * q * q * g;
        let f3 = 0.5 * (-(7.0_f64.sqrt()) * FIELD_C[3][0]) * q * q * q * g;
        let f4 = 0.625 * (-(9.0_f64.sqrt()) * FIELD_C[4][0]) * q * q * q * q * g;
        let across = -g
            + f2 * (5.0 * s2 - 1.0)
            + f3 * 5.0 * (z / r) * (7.0 * s2 - 3.0)
            + f4 * (63.0 * s4 - 42.0 * s2 + 3.0);
        [
            across * (x / r),
            across * (y / r),
            (-g + f2 * (5.0 * s2 - 3.0) + f4 * (63.0 * s4 - 70.0 * s2 + 15.0)) * (z / r)
                + f3 * (35.0 * s4 - 30.0 * s2 + 3.0),
        ]
    }

    /// SHA-256 of a byte string, as lower-case hex. Test use only.
    fn sha256_hex(data: &[u8]) -> String {
        const ROUND: [u32; 64] = [
            0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4,
            0xab1c5ed5, 0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe,
            0x9bdc06a7, 0xc19bf174, 0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f,
            0x4a7484aa, 0x5cb0a9dc, 0x76f988da, 0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7,
            0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967, 0x27b70a85, 0x2e1b2138, 0x4d2c6dfc,
            0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85, 0xa2bfe8a1, 0xa81a664b,
            0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070, 0x19a4c116,
            0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
            0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7,
            0xc67178f2,
        ];
        let mut hash: [u32; 8] = [
            0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab,
            0x5be0cd19,
        ];
        let mut message: Vec<u8> = Vec::from(data);
        message.push(0x80);
        while message.len() % 64 != 56 {
            message.push(0);
        }
        message.extend_from_slice(&((data.len() as u64) * 8).to_be_bytes());
        for block in message.chunks(64) {
            let mut word = [0u32; 64];
            let mut index = 0;
            while index < 16 {
                word[index] = u32::from_be_bytes([
                    block[4 * index],
                    block[4 * index + 1],
                    block[4 * index + 2],
                    block[4 * index + 3],
                ]);
                index += 1;
            }
            while index < 64 {
                let low = word[index - 15].rotate_right(7)
                    ^ word[index - 15].rotate_right(18)
                    ^ (word[index - 15] >> 3);
                let high = word[index - 2].rotate_right(17)
                    ^ word[index - 2].rotate_right(19)
                    ^ (word[index - 2] >> 10);
                word[index] = word[index - 16]
                    .wrapping_add(low)
                    .wrapping_add(word[index - 7])
                    .wrapping_add(high);
                index += 1;
            }
            let mut v = hash;
            let mut index = 0;
            while index < 64 {
                let mix = v[4].rotate_right(6) ^ v[4].rotate_right(11) ^ v[4].rotate_right(25);
                let choose = (v[4] & v[5]) ^ (!v[4] & v[6]);
                let first = v[7]
                    .wrapping_add(mix)
                    .wrapping_add(choose)
                    .wrapping_add(ROUND[index])
                    .wrapping_add(word[index]);
                let mix = v[0].rotate_right(2) ^ v[0].rotate_right(13) ^ v[0].rotate_right(22);
                let majority = (v[0] & v[1]) ^ (v[0] & v[2]) ^ (v[1] & v[2]);
                let second = mix.wrapping_add(majority);
                v = [
                    first.wrapping_add(second),
                    v[0],
                    v[1],
                    v[2],
                    v[3].wrapping_add(first),
                    v[4],
                    v[5],
                    v[6],
                ];
                index += 1;
            }
            let mut index = 0;
            while index < 8 {
                hash[index] = hash[index].wrapping_add(v[index]);
                index += 1;
            }
        }
        let mut hex = String::new();
        for part in hash {
            hex.push_str(&format!("{part:08x}"));
        }
        hex
    }

    /// The literals of one field table in the generated text, row by row.
    fn table_literals<'a>(generated: &'a str, head: &str) -> Vec<&'a str> {
        let from = generated.find(head).unwrap();
        let mut literals = Vec::new();
        for line in generated[from..].lines().skip(1) {
            let line = line.trim();
            if line == "];" {
                break;
            }
            if line.starts_with("//") || line == "[" || line == "]," {
                continue;
            }
            literals.push(line.trim_end_matches(','));
        }
        literals
    }

    #[test]
    fn constants_and_tables_are_locked() {
        assert_copy::<SatelliteLageos1>();
        assert_copy::<SatelliteState>();
        assert_copy::<SatelliteLoad>();
        assert_copy::<StepReject>();
        let satellite = SatelliteLageos1::lageos1();
        assert!(satellite.is_valid());
        assert_eq!(satellite.gate_dt_s(), Some(10.0));
        assert_eq!(
            LEDGER_SCALE_J_PER_KG.to_bits(),
            32485773.553382233_f64.to_bits()
        );
        let mut state = start(&satellite);
        let saved = state_bits(&state);
        let mut frame = blank_frame();
        let held = frame.to_bytes();
        let mut refuse = |tampered: SatelliteLageos1, what: &str| {
            assert!(
                !tampered.is_valid(),
                "{what} changed and the card is still valid"
            );
            assert!(tampered.gate_dt_s().is_none(), "{what}");
            assert!(tampered.floor_dt_s().is_none(), "{what}");
            assert!(
                fresh(
                    &tampered,
                    START_EPOCH_UTC_S,
                    START_POSITION_M,
                    START_VELOCITY_M_S
                )
                .is_none(),
                "{what}"
            );
            let got = step(&tampered, &mut state, &mut frame, NO_LOAD, DT);
            assert_eq!(got, Err(StepReject::InvalidSatellite), "{what}");
            assert_eq!(state_bits(&state), saved, "{what}");
            assert_eq!(frame.to_bytes(), held, "{what}");
            proof_intact(&frame);
        };

        let mut tampered = satellite;
        tampered.field_c[7][3] *= 1.000_001;
        refuse(tampered, "one cosine coefficient of the field");
        let mut tampered = satellite;
        tampered.field_s[12][12] = -0.110993698692882e-07;
        refuse(tampered, "the last digit of one sine coefficient");
        let mut tampered = satellite;
        tampered.field_c[2][0] = -0.484165143790815e-03;
        refuse(
            tampered,
            "the tide-free value in place of the zero-tide one",
        );
        let mut tampered = satellite;
        tampered.field_s[5][0] = 1.0e-12;
        refuse(tampered, "a coefficient where the table has none");
        let mut tampered = satellite;
        tampered.satellite_mass_kg = 405.38;
        refuse(tampered, "the mass");
        let mut tampered = satellite;
        tampered.love_k21 = 0.3;
        refuse(tampered, "one tide number");
        let mut tampered = satellite;
        tampered.moon_longitude_terms_arcsec[0][0] = 22641.0;
        refuse(tampered, "one amplitude of the Moon series");
        let mut tampered = satellite;
        tampered.moon_distance_terms_km[1][4] = 1.0;
        refuse(tampered, "one multiplier of the Moon series");
        let mut tampered = satellite;
        tampered.sun_distance_gm[1] = 2.499;
        refuse(tampered, "the sign of one Sun term");
        let mut tampered = satellite;
        tampered.moon_arguments_deg[0][1] = 481267.88088 - 1.3972;
        refuse(tampered, "the Moon's mean longitude rate");
        let mut tampered = satellite;
        tampered.obliquity_deg = 23.44;
        refuse(tampered, "the obliquity");
        let mut tampered = satellite;
        tampered.earth_rotation_rate_rad_s = 7.292115e-5;
        refuse(tampered, "the rotation rate");
        let mut tampered = satellite;
        tampered.gm_earth_m3_s2 = 3.986004418e14;
        refuse(tampered, "the Earth's gravitational parameter");
        let mut tampered = satellite;
        tampered.gm_moon_m3_s2 = 0.0;
        refuse(tampered, "the Moon's gravitational parameter");
        let mut tampered = satellite;
        tampered.radiation_coefficient = 1.0;
        refuse(tampered, "the radiation coefficient");
        let mut tampered = satellite;
        tampered.tt_minus_utc_s = 68.184;
        refuse(tampered, "the offset of Terrestrial Time");
        let mut tampered = satellite;
        tampered.residual_conservation_limit = 1.0e-3;
        refuse(tampered, "the ledger limit");
        let mut tampered = satellite;
        tampered.ledger_scale_j_per_kg = 1.0e12;
        refuse(tampered, "the ledger scale");
        let mut tampered = satellite;
        tampered.radius_max_m = 1.0e9;
        refuse(tampered, "the top of the radius band");
        let mut tampered = satellite;
        tampered.orbit_semi_major_axis_m[0][0] = 0.0;
        refuse(tampered, "the bottom of the LAGEOS-1 orbit band");
        let mut tampered = satellite;
        tampered.orbit_eccentricity[1][1] = 0.5;
        refuse(tampered, "the top of the LAGEOS-2 eccentricity band");
        let mut tampered = satellite;
        tampered.orbit_inclination_deg[0][0] = 0.0;
        refuse(tampered, "the bottom of the LAGEOS-1 inclination band");
        let mut tampered = satellite;
        tampered.forecast_horizon_s = 1.0e9;
        refuse(tampered, "the seven days of a forecast");
        let mut tampered = satellite;
        tampered.epoch_max_s = 1.0e10;
        refuse(tampered, "the last date");
        let mut tampered = satellite;
        tampered.epoch_end_allowance_s = 1.0;
        refuse(tampered, "the allowance at the last date");
        let mut tampered = satellite;
        tampered.clock_rounding_per_step = 1.0e-9;
        refuse(tampered, "the rounding of the clock");
        let mut tampered = satellite;
        tampered.epoch_end_allowance_extra_steps = 3.0;
        refuse(tampered, "the extra steps of the allowance");
        let mut tampered = satellite;
        tampered.reference_radius_m = 12_271_000.0;
        refuse(tampered, "the reference radius");

        step(&satellite, &mut state, &mut frame, NO_LOAD, DT).unwrap();
        proof_intact(&frame);
    }

    #[test]
    fn the_locked_tables_match_their_sources_text_for_text() {
        assert_eq!(
            sha256_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        // The table of 88 rows the stand-in wrote from the EGM2008 file and
        // IERS Table 6.2, carried in this module byte for byte.
        assert_eq!(GRAVITY_TABLE_TEXT.len(), 5142);
        assert_eq!(
            sha256_hex(GRAVITY_TABLE_TEXT.as_bytes()),
            GRAVITY_TABLE_SHA256
        );
        assert_eq!(
            GRAVITY_TABLE_SHA256,
            "3ffe678122ac51affc23a49ed6424ed758227bf6ac041760a63d11614e5012cb"
        );

        // The text between the two marker lines of this file is the output of
        // the script that wrote the tables, byte for byte.
        let source = include_str!("satellite_lageos1.rs");
        let begin_mark = concat!("// <locked tables: ", "generated text begins>");
        let end_mark = concat!("// <locked tables: ", "generated text ends>\n");
        let begin = source.find(begin_mark).unwrap();
        let end = source.find(end_mark).unwrap() + end_mark.len();
        let generated = &source[begin..end];
        assert_eq!(sha256_hex(generated.as_bytes()), GENERATED_TABLES_SHA256);

        // Every literal of the two field tables is the text of the table row
        // it came from, and its 64-bit value is the value of that text.
        let cosine = table_literals(generated, "pub const FIELD_C:");
        let sine = table_literals(generated, "pub const FIELD_S:");
        assert_eq!(cosine.len(), FIELD_ROWS * FIELD_ROWS);
        assert_eq!(sine.len(), FIELD_ROWS * FIELD_ROWS);
        let mut rows = GRAVITY_TABLE_TEXT.lines();
        assert_eq!(
            rows.next(),
            Some("degree,order,normalised_C,normalised_S,source")
        );
        let mut seen = [[false; FIELD_ROWS]; FIELD_ROWS];
        let mut row_count = 0;
        let mut numbers = 0;
        for row in rows {
            let mut cells = row.split(',');
            let degree: usize = cells.next().unwrap().parse().unwrap();
            let order: usize = cells.next().unwrap().parse().unwrap();
            let c_text = cells.next().unwrap();
            let s_text = cells.next().unwrap();
            let c: f64 = c_text.parse().unwrap();
            let s: f64 = s_text.parse().unwrap();
            assert!(degree >= 2 && degree <= FIELD_DEGREE && order <= degree);
            assert!(!seen[degree][order]);
            seen[degree][order] = true;
            assert_eq!(FIELD_C[degree][order].to_bits(), c.to_bits(), "{row}");
            assert_eq!(FIELD_S[degree][order].to_bits(), s.to_bits(), "{row}");
            assert_eq!(cosine[degree * FIELD_ROWS + order], c_text, "{row}");
            assert!(c != 0.0);
            numbers += 1;
            if order == 0 {
                assert!(s == 0.0, "{row}");
                assert_eq!(sine[degree * FIELD_ROWS + order], "0.0");
            } else {
                assert_eq!(sine[degree * FIELD_ROWS + order], s_text, "{row}");
                assert!(s != 0.0);
                numbers += 1;
            }
            row_count += 1;
        }
        assert_eq!(row_count, 88);
        assert_eq!(numbers, 165);
        let mut degree = 0;
        while degree < FIELD_ROWS {
            let mut order = 0;
            while order < FIELD_ROWS {
                if !seen[degree][order] {
                    assert_eq!(FIELD_C[degree][order].to_bits(), 0);
                    assert_eq!(FIELD_S[degree][order].to_bits(), 0);
                    assert_eq!(cosine[degree * FIELD_ROWS + order], "0.0");
                    assert_eq!(sine[degree * FIELD_ROWS + order], "0.0");
                }
                order += 1;
            }
            degree += 1;
        }
        // The three numbers that are not EGM2008's.
        assert_eq!(FIELD_C[2][0].to_bits(), (-0.48416948e-3_f64).to_bits());
        assert_eq!(FIELD_C[3][0].to_bits(), 0.9571612e-6_f64.to_bits());
        assert_eq!(FIELD_C[4][0].to_bits(), 0.5399659e-6_f64.to_bits());
        // The series: 29 rows of terms, and the leading latitude term.
        assert_eq!(
            MOON_DISTANCE_TERMS_KM.len()
                + MOON_LONGITUDE_TERMS_ARCSEC.len()
                + MOON_LATITUDE_TERMS_ARCSEC.len(),
            29
        );
        assert_eq!(
            MOON_LONGITUDE_TERMS_ARCSEC[0],
            [22640.0, 1.0, 0.0, 0.0, 0.0]
        );
        assert_eq!(MOON_DISTANCE_TERMS_KM[1], [-3699.0, -1.0, 0.0, 0.0, 2.0]);
        assert_eq!(MOON_LATITUDE_TERMS_ARCSEC[6], [11.0, 0.0, -1.0, 1.0, -2.0]);
        assert_eq!(MOON_LATITUDE_LEAD_ARCSEC, [18520.0, 412.0, 541.0]);
        assert_eq!(SUN_DISTANCE_GM, [149.619, -2.499, -0.021]);
    }

    #[test]
    fn the_field_matches_the_stand_in_and_the_high_precision_values() {
        let satellite = SatelliteLageos1::lageos1();
        // The stand-in's value at the start, by a different recursion: to 1 part in 10^13.
        let got = field_acceleration_m_s2(&satellite, START_POSITION_M);
        assert!(
            apart(got, STAND_IN_FIELD_AT_START_M_S2)
                <= 1.0e-13 * size(STAND_IN_FIELD_AT_START_M_S2)
        );
        // Five positions in 60-digit arithmetic, from explicit Legendre
        // polynomials and a numerical gradient. One is on the axis of
        // rotation, where a formula in latitude and longitude fails.
        // Measured: at most 1.7 parts in 10^16.
        for (position_m, acceleration_m_s2, potential_j_per_kg) in FIELD_REFERENCE {
            let got = field_acceleration_m_s2(&satellite, position_m);
            let miss = apart(got, acceleration_m_s2) / size(acceleration_m_s2);
            assert!(miss <= 1.0e-14, "field at {position_m:?}: {miss:e}");
            let got = field_potential_j_per_kg(&satellite, position_m);
            let miss = abs_f64(got - potential_j_per_kg) / potential_j_per_kg;
            assert!(miss <= 1.0e-15, "potential at {position_m:?}: {miss:e}");
        }
        // The acceleration is the gradient of the potential the ledger uses:
        // a central difference over 40 m, whose own error is near 1e-9.
        let mut axis = 0;
        while axis < 3 {
            let mut ahead = START_POSITION_M;
            let mut behind = START_POSITION_M;
            ahead[axis] += 20.0;
            behind[axis] -= 20.0;
            let slope = (field_potential_j_per_kg(&satellite, ahead)
                - field_potential_j_per_kg(&satellite, behind))
                / 40.0;
            assert!(abs_f64(slope - got_axis(&satellite, axis)) < 5.0e-9);
            axis += 1;
        }
    }

    fn got_axis(satellite: &SatelliteLageos1, axis: usize) -> f64 {
        field_acceleration_m_s2(satellite, START_POSITION_M)[axis]
    }

    #[test]
    fn the_zonal_terms_match_their_closed_forms() {
        // A copy of the card that holds only the zonal coefficients of degree
        // 2, 3 and 4, against the three terms written out by hand. The z
        // parts are 35 s⁴ − 30 s² + 3 for degree 3 and 63 s⁴ − 70 s² + 15 for
        // degree 4. Measured: at most 1.8 parts in 10^16.
        let satellite = SatelliteLageos1::lageos1();
        let mut zonal = satellite;
        zonal.field_c = [[0.0; FIELD_ROWS]; FIELD_ROWS];
        zonal.field_s = [[0.0; FIELD_ROWS]; FIELD_ROWS];
        zonal.field_c[2][0] = FIELD_C[2][0];
        zonal.field_c[3][0] = FIELD_C[3][0];
        zonal.field_c[4][0] = FIELD_C[4][0];
        for (position_m, _, _) in FIELD_REFERENCE {
            let got = field_acceleration_m_s2(&zonal, position_m);
            let want = zonal_closed_form_m_s2(position_m);
            let miss = apart(got, want) / size(want);
            assert!(miss <= 1.0e-14, "zonal terms at {position_m:?}: {miss:e}");
        }
        // The degree-3 term alone pulls along the axis by the same amount at
        // equal distances north and south of the equator, and toward the
        // axis by opposite amounts.
        let mut third = zonal;
        third.field_c[2][0] = 0.0;
        third.field_c[4][0] = 0.0;
        let pull = |position_m: [f64; 3]| {
            let all = field_acceleration_m_s2(&third, position_m);
            let central = field_acceleration_m_s2(&point_mass_only(&satellite), position_m);
            [
                all[0] - central[0],
                all[1] - central[1],
                all[2] - central[2],
            ]
        };
        let [x, y, z] = START_POSITION_M;
        let north = pull([x, y, z]);
        let south = pull([x, y, -z]);
        assert!(abs_f64(north[2]) > 1.0e-7);
        assert!(abs_f64(north[2] - south[2]) < 1.0e-9 * abs_f64(north[2]));
        assert!(abs_f64(north[0] + south[0]) < 1.0e-9 * abs_f64(north[0]));
    }

    #[test]
    fn the_sun_and_moon_are_where_the_spec_puts_them() {
        let satellite = SatelliteLageos1::lageos1();
        // The stand-in at the start epoch: to 1 part in 10^10.
        let sun_m = sun_position_m(&satellite, START_EPOCH_UTC_S);
        let moon_m = moon_position_m(&satellite, START_EPOCH_UTC_S);
        assert!(apart(sun_m, STAND_IN_SUN_AT_START_M) <= 1.0e-10 * size(sun_m));
        assert!(apart(moon_m, STAND_IN_MOON_AT_START_M) <= 1.0e-10 * size(moon_m));
        // The series in 60-digit arithmetic at four epochs, the first and the
        // last of the card among them. Measured: at most 1.5 parts in 10^12,
        // which is what a day count near 9 730 leaves of a turn in 64 bits.
        for (utc_s, sun_m, moon_m) in BODY_REFERENCE {
            let miss = apart(sun_position_m(&satellite, utc_s), sun_m) / size(sun_m);
            assert!(miss <= 1.0e-10, "Sun at {utc_s}: {miss:e}");
            let miss = apart(moon_position_m(&satellite, utc_s), moon_m) / size(moon_m);
            assert!(miss <= 1.0e-10, "Moon at {utc_s}: {miss:e}");
        }
        // Plain sizes: the Sun near one astronomical unit, the Moon near 60 Earth radii.
        assert!(size(sun_m) > 1.47e11 && size(sun_m) < 1.53e11);
        assert!(size(moon_m) > 3.5e8 && size(moon_m) < 4.1e8);
    }

    #[test]
    fn each_of_the_other_forces_matches_its_closed_form() {
        let satellite = SatelliteLageos1::lageos1();
        // The stand-in's sum at the start: to 1 part in 10^8. The stand-in
        // forms its tide as a difference of two numbers near 2.66 m/s², so
        // its own rounding is 2 parts in 10^10 of this sum.
        let got = other_acceleration_m_s2(
            &satellite,
            START_EPOCH_UTC_S,
            START_POSITION_M,
            START_VELOCITY_M_S,
        );
        let miss = apart(got, STAND_IN_OTHER_FORCES_AT_START_M_S2) / size(got);
        assert!(miss <= 1.0e-8, "sum against the stand-in: {miss:e}");

        // Each term alone at three states, against its closed form in
        // 60-digit arithmetic. Measured: the Sun's pull to 1.1 parts in
        // 10^11, the Moon's to 5 in 10^12, the tide to 6 in 10^12, sunlight
        // to 3 in 10^12, relativity to 2 in 10^16, the sum to 6 in 10^12. The
        // Sun's pull is a difference of two numbers 10 000 times its size.
        let limit = [1.0e-10, 1.0e-10, 1.0e-10, 1.0e-10, 1.0e-14, 1.0e-10];
        let name = [
            "the Sun's pull",
            "the Moon's pull",
            "the tide",
            "sunlight",
            "relativity",
            "the sum",
        ];
        for (utc_s, position_m, velocity_m_s, want) in FORCE_REFERENCE {
            let sun_m = sun_position_m(&satellite, utc_s);
            let moon_m = moon_position_m(&satellite, utc_s);
            let got = [
                third_body_acceleration_m_s2(position_m, sun_m, satellite.gm_sun_m3_s2),
                third_body_acceleration_m_s2(position_m, moon_m, satellite.gm_moon_m3_s2),
                tide_acceleration_m_s2(&satellite, position_m, sun_m, moon_m),
                sunlight_acceleration_m_s2(&satellite, position_m, sun_m),
                relativity_acceleration_m_s2(&satellite, position_m, velocity_m_s),
                other_acceleration_m_s2(&satellite, utc_s, position_m, velocity_m_s),
            ];
            let mut index = 0;
            while index < 6 {
                let miss = apart(got[index], want[index]) / size(want[index]);
                assert!(miss <= limit[index], "{} at {utc_s}: {miss:e}", name[index]);
                index += 1;
            }
            // Sizes, so that a term cannot be lost: m/s².
            assert!(size(want[0]) > 3.0e-7 && size(want[1]) > 1.0e-6);
            assert!(size(want[2]) > 1.0e-8 && size(want[3]) > 3.0e-9 && size(want[4]) > 2.0e-9);
        }
        // The five tide numbers at the start, from latitude and longitude in
        // 60-digit arithmetic. Measured: 4 parts in 10^12.
        let tide = tide_coefficients(
            &satellite,
            sun_position_m(&satellite, START_EPOCH_UTC_S),
            moon_position_m(&satellite, START_EPOCH_UTC_S),
        );
        let mut index = 0;
        while index < 5 {
            let miss = abs_f64(tide[index] - TIDE_COEFFICIENTS_AT_START[index])
                / abs_f64(TIDE_COEFFICIENTS_AT_START[index]);
            assert!(miss <= 1.0e-10, "tide number {index}: {miss:e}");
            index += 1;
        }
    }

    #[test]
    fn a_point_mass_alone_conserves_energy_and_angular_momentum_and_closes_its_orbit() {
        // A copy of the card with the field tables, the Sun, the Moon, the
        // tide, sunlight and relativity all off, stepped by the card's own
        // `integrate` for one revolution in 13 527 steps of 1.00003 s.
        let satellite = SatelliteLageos1::lageos1();
        let alone = point_mass_only(&satellite);
        assert!(!alone.is_valid());
        let other = other_acceleration_m_s2(
            &alone,
            START_EPOCH_UTC_S,
            START_POSITION_M,
            START_VELOCITY_M_S,
        );
        assert_eq!(other, [0.0; 3]);
        let gm = alone.gm_earth_m3_s2;
        let pull = field_acceleration_m_s2(&alone, START_POSITION_M);
        let radius_m = size(START_POSITION_M);
        let mut axis = 0;
        while axis < 3 {
            let want = -gm * START_POSITION_M[axis] / (radius_m * radius_m * radius_m);
            assert!(abs_f64(pull[axis] - want) <= 1.0e-15 * abs_f64(want));
            axis += 1;
        }

        let fixed_0 = fixed_frame_velocity_m_s(&alone, START_POSITION_M, START_VELOCITY_M_S);
        let energy_0 = 0.5 * dot(fixed_0, fixed_0) - gm / radius_m;
        let momentum_0 = cross(START_POSITION_M, fixed_0);
        let semi_major_axis_m = -gm / (2.0 * energy_0);
        let period_s = 2.0
            * core::f64::consts::PI
            * (semi_major_axis_m * semi_major_axis_m * semi_major_axis_m / gm).sqrt();
        assert!(period_s > 13_527.0 && period_s < 13_528.0);
        let steps = 13_527u32;
        let dt_s = period_s / f64::from(steps);
        let mut y = recorded(&start(&satellite));
        let mut worst_energy = 0.0;
        let mut worst_momentum = 0.0;
        let mut n = 0;
        while n < steps {
            y = integrate(&alone, START_EPOCH_UTC_S + f64::from(n) * dt_s, y, dt_s).unwrap();
            n += 1;
            let position_m = [y[0], y[1], y[2]];
            let fixed = fixed_frame_velocity_m_s(&alone, position_m, [y[3], y[4], y[5]]);
            let energy = 0.5 * dot(fixed, fixed) - gm / size(position_m);
            let momentum = cross(position_m, fixed);
            let miss = abs_f64(energy - energy_0) / abs_f64(energy_0);
            if miss > worst_energy {
                worst_energy = miss;
            }
            // In the turning frame's axes the angular momentum keeps its
            // size and its part along the axis of rotation.
            let miss = abs_f64(size(momentum) - size(momentum_0)) / size(momentum_0);
            if miss > worst_momentum {
                worst_momentum = miss;
            }
            let miss = abs_f64(momentum[2] - momentum_0[2]) / size(momentum_0);
            if miss > worst_momentum {
                worst_momentum = miss;
            }
        }
        // Measured: energy 1.6 parts in 10^14, angular momentum 8 in 10^15.
        assert!(worst_energy < 1.0e-13, "energy moved {worst_energy:e}");
        assert!(
            worst_momentum < 1.0e-13,
            "angular momentum moved {worst_momentum:e}"
        );
        // No work was booked.
        assert_eq!(y[6].to_bits(), 0.0_f64.to_bits());
        // Turned back by what the frame turned in one period, the satellite
        // is where it started. Measured: 0.7 micrometres.
        let angle_rad = alone.earth_rotation_rate_rad_s * period_s;
        let back_m = [
            y[0] * crate::math::cos(angle_rad) - y[1] * crate::math::sin(angle_rad),
            y[0] * crate::math::sin(angle_rad) + y[1] * crate::math::cos(angle_rad),
            y[2],
        ];
        let open_m = apart(back_m, START_POSITION_M);
        assert!(open_m < 1.0e-5, "the orbit is {open_m} m from closing");
        // Without that turn it is thousands of kilometres away: the frame terms matter.
        assert!(apart([y[0], y[1], y[2]], START_POSITION_M) > 1.0e6);
    }

    #[test]
    fn the_trajectory_reproduces_the_stand_in_at_stated_epochs() {
        // RK4 at 10 s from the first start of the opened week. The stand-in
        // is a different program in a different language with a different
        // recursion for the field. Measured: 0 m after 1 and 12 steps,
        // 7.6e-8 m after one revolution, 5.2e-6 m after one day. Moving the
        // stand-in's own field acceleration by one unit in the last place
        // moves it by 5.7e-8 m and 4.8e-6 m at the same two epochs.
        let satellite = SatelliteLageos1::lageos1();
        let mut state = start(&satellite);
        assert_eq!(
            state.jacobi_anchor_j_per_kg.to_bits(),
            STAND_IN_JACOBI_ANCHOR_J_PER_KG.to_bits()
        );
        let mut frame = blank_frame();
        let limit_m = [1.0e-6, 1.0e-6, 1.0e-6, 1.0e-4];
        let mut n = 0u32;
        let mut next = 0;
        while next < 4 {
            step(&satellite, &mut state, &mut frame, NO_LOAD, DT).unwrap();
            n += 1;
            let (at, position_m, velocity_m_s, work_j_per_kg) = STAND_IN_STATES[next];
            if n == at {
                let miss = apart(state.position_m, position_m);
                assert!(
                    miss <= limit_m[next],
                    "step {n}: {miss} m from the stand-in"
                );
                let miss = apart(state.velocity_m_s, velocity_m_s);
                assert!(miss <= 1.0e-7, "step {n}: {miss} m/s from the stand-in");
                let miss = abs_f64(state.external_work_j_per_kg - work_j_per_kg);
                assert!(
                    miss <= 1.0e-8,
                    "step {n}: work {miss} J/kg from the stand-in"
                );
                assert_eq!(state.t_s.to_bits(), (f64::from(n) * DT).to_bits());
                next += 1;
            }
        }
        assert_eq!(n, 8_640);
        // After 12 steps, as the spec prints it.
        assert_eq!(STAND_IN_STATES[1].0, 12);
        assert!(
            apart(
                STAND_IN_STATES[1].1,
                [4257272.572583, 9140013.996748, 7066949.787151]
            ) < 1.0e-6
        );
        proof_intact(&frame);
    }

    #[test]
    fn a_week_at_the_ceiling_step_holds_the_ledger_and_the_stand_in() {
        let satellite = SatelliteLageos1::lageos1();
        let mut state = start(&satellite);
        let mut frame = blank_frame();
        let mut worst = 0.0;
        let mut worst_in_a_day = 0.0;
        let mut n = 0u32;
        while n < 60_480 {
            step(&satellite, &mut state, &mut frame, NO_LOAD, DT).unwrap();
            n += 1;
            if state.residual_conservation_error > worst {
                worst = state.residual_conservation_error;
            }
            if n == 8_640 {
                worst_in_a_day = worst;
            }
        }
        // Measured: 2.24e-12 in one day, 1.47e-11 in seven; the limit is 1e-9.
        assert!(worst_in_a_day < 1.0e-11, "{worst_in_a_day:e}");
        assert!(worst < 1.0e-10, "{worst:e}");
        assert!(worst > worst_in_a_day);
        // Measured against the stand-in after seven days: 3.6e-5 m. Moving the
        // stand-in's own field by one unit in the last place moves it 8e-5 m.
        let (at, position_m, velocity_m_s, work_j_per_kg) = STAND_IN_STATES[4];
        assert_eq!(at, 60_480);
        let miss = apart(state.position_m, position_m);
        assert!(
            miss <= 2.0e-3,
            "{miss} m from the stand-in after seven days"
        );
        let miss = apart(state.velocity_m_s, velocity_m_s);
        assert!(
            miss <= 1.0e-6,
            "{miss} m/s from the stand-in after seven days"
        );
        assert!(abs_f64(state.external_work_j_per_kg - work_j_per_kg) <= 1.0e-7);
        assert_eq!(state.t_s.to_bits(), 604_800.0_f64.to_bits());
        proof_intact(&frame);
    }

    #[test]
    fn honest_steps_conserve() {
        let satellite = SatelliteLageos1::lageos1();
        let mut frame = blank_frame();
        // One day at 10 s. Measured: 3.1e-15 after one step, 2.24e-12 at most.
        let mut state = start(&satellite);
        let mut n = 0u32;
        while n < 8_640 {
            step(&satellite, &mut state, &mut frame, NO_LOAD, DT).unwrap();
            assert!(state.residual_conservation_error < 1.0e-11);
            assert_eq!(
                state.residual_conservation_error.to_bits(),
                (abs_f64(state.energy_defect_j_per_kg) / LEDGER_SCALE_J_PER_KG).to_bits()
            );
            assert_eq!(
                state.energy_defect_j_per_kg.to_bits(),
                (state.jacobi_j_per_kg
                    - state.jacobi_anchor_j_per_kg
                    - state.external_work_j_per_kg)
                    .to_bits()
            );
            if n == 0 {
                assert!(state.residual_conservation_error < 1.0e-14);
            }
            n += 1;
        }
        // The work of the other forces is not small beside the limit: the
        // ledger would refuse this day at once if it were not booked.
        assert!(state.external_work_j_per_kg > 9.0);
        assert!(
            state.external_work_j_per_kg
                > 100.0 * RESIDUAL_CONSERVATION_LIMIT * LEDGER_SCALE_J_PER_KG
        );
        // One hour at 0.1 s. Measured: 7.8e-15 at most.
        let mut state = start(&satellite);
        let mut n = 0u32;
        while n < 36_000 {
            step(&satellite, &mut state, &mut frame, NO_LOAD, 0.1).unwrap();
            assert!(state.residual_conservation_error < 1.0e-13);
            n += 1;
        }
        proof_intact(&frame);
    }

    #[test]
    fn explicit_euler_is_refused_on_the_first_step() {
        let satellite = SatelliteLageos1::lageos1();
        let mut state = start(&satellite);
        let saved = state_bits(&state);
        let mut frame = blank_frame();
        let held = frame.to_bytes();
        // At the ceiling, and at a hundredth of it.
        for (dt_s, stand_in) in [
            (10.0, 2.5548615005664525e-05),
            (0.1, 2.5548441163364096e-09),
        ] {
            let got = step_euler(&satellite, &mut state, &mut frame, dt_s);
            assert_eq!(got, Err(StepReject::SpontaneousGeneration));
            assert_eq!(state_bits(&state), saved);
            assert_eq!(frame.to_bytes(), held);
            proof_intact(&frame);
            // How far over the limit of 1e-9, against the stand-in's number.
            let y = recorded(&state);
            let trial = axpy(y, dt_s, rates(&satellite, START_EPOCH_UTC_S, y).unwrap());
            let residual = signed_residual(&satellite, state.jacobi_anchor_j_per_kg, trial);
            assert!(residual > 2.5 * RESIDUAL_CONSERVATION_LIMIT);
            assert!(
                abs_f64(residual - stand_in) < 1.0e-6 * stand_in,
                "{residual:e}"
            );
        }
        // The honest step from the same state lands.
        step(&satellite, &mut state, &mut frame, NO_LOAD, DT).unwrap();
        assert!(state.residual_conservation_error < 1.0e-14);
        proof_intact(&frame);
    }

    #[test]
    fn the_midpoint_rule_lands_at_first_and_adds_up_to_a_refusal() {
        let satellite = SatelliteLageos1::lageos1();
        let mut state = start(&satellite);
        let mut frame = blank_frame();
        step_midpoint(&satellite, &mut state, &mut frame, DT).unwrap();
        // Measured after one step: 1.52e-10, fifty thousand times an honest step.
        let first = state.residual_conservation_error;
        assert!(first > 1.0e-10 && first < 2.0e-10, "{first:e}");
        let mut landed = 1u32;
        let mut before = first;
        let why = loop {
            let saved = state_bits(&state);
            let held = frame.to_bytes();
            match step_midpoint(&satellite, &mut state, &mut frame, DT) {
                Ok(()) => {
                    assert!(state.residual_conservation_error > before);
                    before = state.residual_conservation_error;
                    landed += 1;
                    assert!(landed < 8_640, "the midpoint rule ran a whole day");
                }
                Err(why) => {
                    assert_eq!(state_bits(&state), saved);
                    assert_eq!(frame.to_bytes(), held);
                    break why;
                }
            }
        };
        assert_eq!(why, StepReject::SpontaneousGeneration);
        assert!(before > 5.0 * first);
        assert!(before <= RESIDUAL_CONSERVATION_LIMIT);
        proof_intact(&frame);
    }

    #[test]
    fn a_flipped_centrifugal_sign_is_refused_on_the_first_step() {
        let satellite = SatelliteLageos1::lageos1();
        let mut state = start(&satellite);
        let saved = state_bits(&state);
        let mut frame = blank_frame();
        let held = frame.to_bytes();
        // The test's own RK4 with both signs right is the card's, bit for bit.
        let y = recorded(&state);
        let own = rk4_with_signs(&satellite, START_EPOCH_UTC_S, y, DT, 1.0, 1.0).unwrap();
        let card = integrate(&satellite, START_EPOCH_UTC_S, y, DT).unwrap();
        let mut index = 0;
        while index < 7 {
            assert_eq!(own[index].to_bits(), card[index].to_bits());
            index += 1;
        }

        let got = step_with_signs(&satellite, &mut state, &mut frame, DT, 1.0, -1.0);
        assert_eq!(got, Err(StepReject::FirstLawBreach));
        assert_eq!(state_bits(&state), saved);
        assert_eq!(frame.to_bytes(), held);
        proof_intact(&frame);
        // Measured: −9.99e-5, a hundred thousand times the limit.
        let trial = rk4_with_signs(&satellite, START_EPOCH_UTC_S, y, DT, 1.0, -1.0).unwrap();
        let residual = signed_residual(&satellite, state.jacobi_anchor_j_per_kg, trial);
        assert!(
            abs_f64(residual - -9.988546432534618e-05) < 1.0e-10,
            "{residual:e}"
        );

        // Both signs right through the same helper: lands.
        step_with_signs(&satellite, &mut state, &mut frame, DT, 1.0, 1.0).unwrap();
        assert!(state.residual_conservation_error < 1.0e-14);
        proof_intact(&frame);
    }

    #[test]
    fn a_flipped_coriolis_sign_passes_the_ledger_and_the_position_test_catches_it() {
        // The Coriolis force is at right angles to the velocity and does no
        // work. The ledger cannot see its sign. This is a stated limit.
        let satellite = SatelliteLageos1::lageos1();
        let mut state = start(&satellite);
        let saved = state_bits(&state);
        let mut frame = blank_frame();
        let held = frame.to_bytes();
        let y = recorded(&state);
        let trial = rk4_with_signs(&satellite, START_EPOCH_UTC_S, y, DT, -1.0, 1.0).unwrap();
        // The ledger is as clean as after an honest step. Measured: 2.8e-15.
        let residual = abs_f64(signed_residual(&satellite, state.jacobi_anchor_j_per_kg, trial));
        assert!(residual < 1.0e-14, "{residual:e}");
        // The position is 64.8 m from the stand-in's after one step of 10 s.
        // The position test above allows one micrometre.
        let miss_m = apart([trial[0], trial[1], trial[2]], STAND_IN_STATES[0].1);
        assert!(miss_m > 60.0 && miss_m < 70.0, "{miss_m} m");

        // What stops the step is the card's orbit band, not the ledger: the
        // orbit worked out from the wrong velocity is in neither of the two
        // compared orbits, at the first landing.
        let got = step_with_signs(&satellite, &mut state, &mut frame, DT, -1.0, 1.0);
        assert_eq!(got, Err(StepReject::OrbitOutsideCard));
        assert_eq!(state_bits(&state), saved);
        assert_eq!(frame.to_bytes(), held);

        // Without the card's bands, as in the stand-in: one day of RK4 with
        // the wrong sign. The ledger stays clean all day, and the satellite
        // ends 24 955 km from where it should be.
        let begin = start(&satellite);
        let mut y = recorded(&begin);
        let mut worst = 0.0;
        let mut n = 0u32;
        while n < 8_640 {
            let utc_s = START_EPOCH_UTC_S + f64::from(n) * DT;
            y = rk4_with_signs(&satellite, utc_s, y, DT, -1.0, 1.0).unwrap();
            n += 1;
            let residual = abs_f64(signed_residual(&satellite, begin.jacobi_anchor_j_per_kg, y));
            if residual > worst {
                worst = residual;
            }
        }
        // Measured: 1.89e-12, the size of an honest day.
        assert!(worst < 1.0e-11, "{worst:e}");
        let end_m = [y[0], y[1], y[2]];
        assert!(apart(end_m, STAND_IN_FLIPPED_CORIOLIS_ONE_DAY_M) < 1.0e-3);
        let astray_m = apart(end_m, STAND_IN_STATES[3].1);
        assert!(astray_m > 2.49e7 && astray_m < 2.50e7, "{astray_m} m");
        proof_intact(&frame);
    }

    #[test]
    fn a_step_six_times_the_ceiling_is_stopped_by_the_ledger() {
        // The public step does not apply the ceiling; the gateway does. If a
        // 60 s step were let through, the ledger refuses it within the day.
        let satellite = SatelliteLageos1::lageos1();
        let mut state = start(&satellite);
        let mut frame = blank_frame();
        step(&satellite, &mut state, &mut frame, NO_LOAD, 60.0).unwrap();
        // Measured after one step: 3.36e-11, ten thousand times a 10 s step.
        let first = state.residual_conservation_error;
        assert!(first > 3.0e-11 && first < 4.0e-11, "{first:e}");
        let mut landed = 1u32;
        let why = loop {
            let saved = state_bits(&state);
            let held = frame.to_bytes();
            match step(&satellite, &mut state, &mut frame, NO_LOAD, 60.0) {
                Ok(()) => {
                    landed += 1;
                    assert!(landed < 1_440, "a 60 s step ran a whole day");
                }
                Err(why) => {
                    assert_eq!(state_bits(&state), saved);
                    assert_eq!(frame.to_bytes(), held);
                    break why;
                }
            }
        };
        assert_eq!(why, StepReject::FirstLawBreach);
        assert!(state.residual_conservation_error > 10.0 * first);
        // From the refused state a step at the ceiling still lands: the
        // ledger was not reset and nothing was pulled back.
        step(&satellite, &mut state, &mut frame, NO_LOAD, DT).unwrap();
        proof_intact(&frame);
    }

    #[test]
    fn a_biased_ledger_is_refused_both_ways() {
        let satellite = SatelliteLageos1::lageos1();
        let mut state = start(&satellite);
        let mut frame = blank_frame();
        step(&satellite, &mut state, &mut frame, NO_LOAD, DT).unwrap();
        let saved = state_bits(&state);
        let held = frame.to_bytes();
        let extra = 1.0e-6 * LEDGER_SCALE_J_PER_KG;
        // Work booked that no force did.
        let got = step_bias_work(&satellite, &mut state, &mut frame, DT, extra);
        assert_eq!(got, Err(StepReject::FirstLawBreach));
        assert_eq!(state_bits(&state), saved);
        assert_eq!(frame.to_bytes(), held);
        // Work a force did that was not booked.
        let got = step_bias_work(&satellite, &mut state, &mut frame, DT, -extra);
        assert_eq!(got, Err(StepReject::SpontaneousGeneration));
        assert_eq!(state_bits(&state), saved);
        assert_eq!(frame.to_bytes(), held);
        // Just inside and just outside the limit.
        let edge = RESIDUAL_CONSERVATION_LIMIT * LEDGER_SCALE_J_PER_KG;
        let got = step_bias_work(&satellite, &mut state, &mut frame, DT, 1.01 * edge);
        assert_eq!(got, Err(StepReject::FirstLawBreach));
        assert_eq!(state_bits(&state), saved);
        proof_intact(&frame);
        let mut inside = state;
        step_bias_work(&satellite, &mut inside, &mut frame, DT, 0.99 * edge).unwrap();
        assert!(inside.residual_conservation_error > 0.98e-9);
        assert!(inside.residual_conservation_error <= RESIDUAL_CONSERVATION_LIMIT);
        assert!(inside.energy_defect_j_per_kg < 0.0);
        // No bias: the same helper is the card's step.
        let mut honest = state;
        step_bias_work(&satellite, &mut state, &mut frame, DT, 0.0).unwrap();
        step(&satellite, &mut honest, &mut frame, NO_LOAD, DT).unwrap();
        assert_eq!(state_bits(&state), state_bits(&honest));
        proof_intact(&frame);
    }

    #[test]
    fn every_edge_of_the_card_is_refused_by_name() {
        let satellite = SatelliteLageos1::lageos1();
        let good = start(&satellite);
        let gm = satellite.gm_earth_m3_s2;
        let mut frame = blank_frame();
        let held = frame.to_bytes();
        let mut refuse = |mut state: SatelliteState, dt_s: f64, want: StepReject, what: &str| {
            let saved = state_bits(&state);
            let got = step(&satellite, &mut state, &mut frame, NO_LOAD, dt_s);
            assert_eq!(got, Err(want), "{what}");
            assert_eq!(state_bits(&state), saved, "{what}");
            assert_eq!(frame.to_bytes(), held, "{what}");
            proof_intact(&frame);
        };
        let none = |epoch_utc_s: f64, position_m: [f64; 3], velocity_m_s: [f64; 3]| {
            fresh(&satellite, epoch_utc_s, position_m, velocity_m_s).is_none()
        };

        // The step.
        refuse(good, 0.0, StepReject::NonPositiveDt, "a step of zero");
        refuse(good, -10.0, StepReject::NonPositiveDt, "a step backwards");
        refuse(good, f64::NAN, StepReject::NonFiniteInput, "a step of NaN");
        refuse(
            good,
            f64::INFINITY,
            StepReject::NonFiniteInput,
            "an endless step",
        );
        refuse(
            good,
            f64::NEG_INFINITY,
            StepReject::NonFiniteInput,
            "an endless step backwards",
        );

        // The state.
        let mut state = good;
        state.position_m[1] = f64::NAN;
        refuse(state, DT, StepReject::NonFiniteState, "NaN in the position");
        refuse(state, f64::NAN, StepReject::NonFiniteInput, "NaN in both");
        let mut state = good;
        state.velocity_m_s[2] = f64::INFINITY;
        refuse(
            state,
            DT,
            StepReject::NonFiniteState,
            "an infinite velocity",
        );
        let mut state = good;
        state.external_work_j_per_kg = f64::NAN;
        refuse(
            state,
            DT,
            StepReject::NonFiniteState,
            "NaN in the booked work",
        );
        let mut state = good;
        state.t_s = f64::NAN;
        refuse(state, DT, StepReject::NonFiniteState, "NaN in the time");
        let mut bad = START_POSITION_M;
        bad[0] = f64::NAN;
        assert!(none(START_EPOCH_UTC_S, bad, START_VELOCITY_M_S));
        assert!(none(f64::NAN, START_POSITION_M, START_VELOCITY_M_S));
        assert!(none(
            START_EPOCH_UTC_S,
            START_POSITION_M,
            [f64::INFINITY, 0.0, 0.0]
        ));

        // The radius band: 11 996 km and 12 328 km in the direction of the start.
        let scale = |radius_m: f64| {
            let by = radius_m / size(START_POSITION_M);
            [
                START_POSITION_M[0] * by,
                START_POSITION_M[1] * by,
                START_POSITION_M[2] * by,
            ]
        };
        for radius_m in [11_996_000.0, 12_328_000.0] {
            assert!(none(START_EPOCH_UTC_S, scale(radius_m), START_VELOCITY_M_S));
            let mut state = good;
            state.position_m = scale(radius_m);
            refuse(
                state,
                DT,
                StepReject::RadiusOutsideCard,
                "a radius outside the band",
            );
        }
        // The edges themselves are inside, and one metre past each is not. A
        // LAGEOS-2 orbit of 12 162 km inclined 52.645 degrees has its lowest
        // point at 11 997 km or its highest at 12 327 km, eccentricity 0.01357,
        // when it crosses the x axis there at right angles.
        for (edge_m, past_m) in [(11_997_000.0, 11_996_999.0), (12_327_000.0, 12_327_001.0)] {
            let at = |radius_m: f64| {
                let e = abs_f64(1.0 - radius_m / 12_162_000.0);
                on_orbit(&satellite, [1.0, 0.0, 0.0], radius_m, 12_162_000.0, e, 52.645)
            };
            let (position_m, velocity_m_s) = at(edge_m);
            let mut state =
                fresh(&satellite, START_EPOCH_UTC_S, position_m, velocity_m_s)
                    .unwrap();
            assert_eq!(state.radius_m.to_bits(), edge_m.to_bits());
            assert!(abs_f64(state.semi_major_axis_m - 12_162_000.0) < 1.0e-3);
            let mut scratch = blank_frame();
            step(&satellite, &mut state, &mut scratch, NO_LOAD, DT).unwrap();
            let (position_m, velocity_m_s) = at(past_m);
            assert!(none(START_EPOCH_UTC_S, position_m, velocity_m_s));
            let mut state = good;
            state.position_m = position_m;
            state.velocity_m_s = velocity_m_s;
            refuse(
                state,
                DT,
                StepReject::RadiusOutsideCard,
                "one metre past an edge",
            );
        }
        // A landing outside: 10 m under the top and rising at 10 m/s, on a
        // LAGEOS-2 orbit. A 10 s step would end near 90 m above the top. It
        // is refused, the state stays where it was, and a 0.1 s step from it
        // lands. Nothing was pulled back.
        let position_m = [12_326_990.0, 0.0, 0.0];
        let speed_squared = gm * (2.0 / 12_326_990.0 - 1.0 / 12_162_000.0);
        let across = (speed_squared - 100.0).sqrt();
        let tilt = 52.645 * RADIANS_PER_DEGREE;
        let fixed_m_s = [
            10.0,
            across * crate::math::cos(tilt),
            across * crate::math::sin(tilt),
        ];
        let velocity_m_s = turning_frame_velocity_m_s(&satellite, position_m, fixed_m_s);
        let rising =
            fresh(&satellite, START_EPOCH_UTC_S, position_m, velocity_m_s)
                .unwrap();
        refuse(
            rising,
            DT,
            StepReject::RadiusOutsideCard,
            "a landing above the band",
        );
        let mut state = rising;
        let mut scratch = blank_frame();
        step(&satellite, &mut state, &mut scratch, NO_LOAD, 0.1).unwrap();
        assert!(state.radius_m > 12_326_990.5 && state.radius_m < 12_326_991.5);
        assert_eq!(state.position_m[0].to_bits(), {
            let mut again = rising;
            let mut other = blank_frame();
            step(&satellite, &mut again, &mut other, NO_LOAD, 0.1).unwrap();
            again.position_m[0].to_bits()
        });

        // The orbit band: the start state with its speed changed.
        for (by, what) in [
            (1.02, "the speed raised 2 percent"),
            (1.5, "the speed raised 50 percent: escaping"),
            (0.98, "the speed lowered 2 percent"),
            (0.0, "no speed at all: falling"),
        ] {
            let velocity_m_s = [
                START_VELOCITY_M_S[0] * by,
                START_VELOCITY_M_S[1] * by,
                START_VELOCITY_M_S[2] * by,
            ];
            assert!(
                none(START_EPOCH_UTC_S, START_POSITION_M, velocity_m_s),
                "{what}"
            );
            let mut state = good;
            state.velocity_m_s = velocity_m_s;
            refuse(state, DT, StepReject::OrbitOutsideCard, what);
        }
        assert!(good.semi_major_axis_m > 12_265_000.0 && good.semi_major_axis_m < 12_275_000.0);

        // The two compared orbits, and nothing else. Orbits that cross the x
        // axis at a distance of their semi-major axis, inside every band, and
        // past each edge of one band with the other two numbers held inside.
        let x_axis = [1.0, 0.0, 0.0];
        let orbit = |a_m: f64, e: f64, inclination_deg: f64| {
            on_orbit(&satellite, x_axis, a_m, a_m, e, inclination_deg)
        };
        for (a_m, e, inclination_deg, what) in [
            (12_270_000.0, 0.0039, 109.815, "LAGEOS-1"),
            (12_265_100.0, 0.0039, 109.815, "LAGEOS-1, the smallest orbit"),
            (12_274_900.0, 0.0039, 109.815, "LAGEOS-1, the largest orbit"),
            (12_270_000.0, 0.00341, 109.815, "LAGEOS-1, the roundest orbit"),
            (12_270_000.0, 0.00439, 109.815, "LAGEOS-1, the least round orbit"),
            (12_270_000.0, 0.0039, 109.801, "LAGEOS-1, the lowest inclination"),
            (12_270_000.0, 0.0039, 109.829, "LAGEOS-1, the highest inclination"),
            (12_162_000.0, 0.0135, 52.645, "LAGEOS-2"),
            (12_158_100.0, 0.0135, 52.645, "LAGEOS-2, the smallest orbit"),
            (12_165_900.0, 0.0135, 52.645, "LAGEOS-2, the largest orbit"),
            (12_162_000.0, 0.01321, 52.645, "LAGEOS-2, the roundest orbit"),
            (12_162_000.0, 0.01379, 52.645, "LAGEOS-2, the least round orbit"),
            (12_162_000.0, 0.0135, 52.631, "LAGEOS-2, the lowest inclination"),
            (12_162_000.0, 0.0135, 52.659, "LAGEOS-2, the highest inclination"),
        ] {
            let (position_m, velocity_m_s) = orbit(a_m, e, inclination_deg);
            let mut state = fresh(&satellite, START_EPOCH_UTC_S, position_m, velocity_m_s)
                .unwrap_or_else(|| ::core::panic!("{what} is refused"));
            let mut scratch = blank_frame();
            step(&satellite, &mut state, &mut scratch, NO_LOAD, DT).unwrap();
        }
        for (a_m, e, inclination_deg, what) in [
            (12_264_900.0, 0.0039, 109.815, "LAGEOS-1 under its smallest orbit"),
            (12_275_100.0, 0.0039, 109.815, "LAGEOS-1 over its largest orbit"),
            (12_270_000.0, 0.00339, 109.815, "LAGEOS-1 rounder than compared"),
            (12_270_000.0, 0.00441, 109.815, "LAGEOS-1 less round than compared"),
            (12_270_000.0, 0.0039, 109.799, "LAGEOS-1 under its inclination"),
            (12_270_000.0, 0.0039, 109.831, "LAGEOS-1 over its inclination"),
            (12_157_900.0, 0.0135, 52.645, "LAGEOS-2 under its smallest orbit"),
            (12_166_100.0, 0.0135, 52.645, "LAGEOS-2 over its largest orbit"),
            (12_162_000.0, 0.01319, 52.645, "LAGEOS-2 rounder than compared"),
            (12_162_000.0, 0.01381, 52.645, "LAGEOS-2 less round than compared"),
            (12_162_000.0, 0.0135, 52.629, "LAGEOS-2 under its inclination"),
            (12_162_000.0, 0.0135, 52.661, "LAGEOS-2 over its inclination"),
            // the bands of one orbit with a number of the other
            (12_270_000.0, 0.0039, 52.645, "LAGEOS-1 at the inclination of LAGEOS-2"),
            (12_270_000.0, 0.0135, 109.815, "LAGEOS-1 at the eccentricity of LAGEOS-2"),
            (12_162_000.0, 0.0135, 109.815, "LAGEOS-2 at the inclination of LAGEOS-1"),
            (12_162_000.0, 0.0039, 52.645, "LAGEOS-2 at the eccentricity of LAGEOS-1"),
            // orbits never compared: equatorial, polar, retrograde, circular
            (12_270_000.0, 0.0039, 0.0, "an equatorial orbit"),
            (12_270_000.0, 0.0039, 90.0, "a polar orbit"),
            (12_270_000.0, 0.0039, 180.0, "a retrograde equatorial orbit"),
            (12_270_000.0, 0.0039, 81.0, "between the two inclinations"),
            (12_270_000.0, 0.0, 109.815, "a circle"),
            (12_216_000.0, 0.0039, 109.815, "between the two orbit sizes"),
        ] {
            let (position_m, velocity_m_s) = orbit(a_m, e, inclination_deg);
            assert!(
                none(START_EPOCH_UTC_S, position_m, velocity_m_s),
                "{what}"
            );
            let mut state = good;
            state.position_m = position_m;
            state.velocity_m_s = velocity_m_s;
            refuse(state, DT, StepReject::OrbitOutsideCard, what);
        }
        // The polar orbit of 0.6.0's worked example, circular at 12 270 km
        // over the pole, is refused.
        assert!(none(
            START_EPOCH_UTC_S,
            [12_270_000.0, 0.0, 0.0],
            [0.0, -894.7425285009465, 5699.629247010917]
        ));

        // The dates: 2026-08-22 and 2026-09-21, a day outside each end of the
        // compared runs, and 2017-01-01, which 0.6.0 took.
        for epoch_utc_s in [EPOCH_MIN_S - 86_400.0, EPOCH_MAX_S + 86_400.0, 536_500_800.0] {
            assert!(none(epoch_utc_s, START_POSITION_M, START_VELOCITY_M_S));
            let mut state = good;
            state.start_epoch_utc_s = epoch_utc_s;
            refuse(
                state,
                DT,
                StepReject::EpochOutsideCard,
                "a date outside the card",
            );
        }
        // The first second of the card is inside. So is the last, but no step
        // can be taken from it; a step that lands on it exactly lands.
        let mut first = fresh(
            &satellite,
            EPOCH_MIN_S,
            START_POSITION_M,
            START_VELOCITY_M_S,
        )
        .unwrap();
        let mut scratch = blank_frame();
        step(&satellite, &mut first, &mut scratch, NO_LOAD, DT).unwrap();
        assert!(none(
            EPOCH_MIN_S - 1.0,
            START_POSITION_M,
            START_VELOCITY_M_S
        ));
        let last = fresh(
            &satellite,
            EPOCH_MAX_S,
            START_POSITION_M,
            START_VELOCITY_M_S,
        )
        .unwrap();
        refuse(
            last,
            DT,
            StepReject::EpochOutsideCard,
            "a step past the last date",
        );
        let mut near = fresh(
            &satellite,
            EPOCH_MAX_S - 10.0,
            START_POSITION_M,
            START_VELOCITY_M_S,
        )
        .unwrap();
        step(&satellite, &mut near, &mut scratch, NO_LOAD, DT).unwrap();
        assert_eq!(
            near.epoch_utc_s().to_bits(),
            EPOCH_MAX_S.to_bits()
        );
        refuse(
            near,
            DT,
            StepReject::EpochOutsideCard,
            "a second step past the last date",
        );
        // The clock of a run counts too.
        let mut late = good;
        late.t_s = EPOCH_MAX_S - START_EPOCH_UTC_S - 5.0;
        refuse(
            late,
            DT,
            StepReject::EpochOutsideCard,
            "a run that reaches the last date",
        );

        // The seven days of a forecast. A state stated as 604 790 s from its
        // published state takes one 10 s step, to the seven days exactly, and
        // no more.
        let mut old = SatelliteState::try_state(
            &satellite,
            START_EPOCH_UTC_S,
            FORECAST_HORIZON_S - 10.0,
            START_POSITION_M,
            START_VELOCITY_M_S,
        )
        .unwrap();
        let mut scratch = blank_frame();
        step(&satellite, &mut old, &mut scratch, NO_LOAD, DT).unwrap();
        assert_eq!(old.forecast_elapsed_s().to_bits(), FORECAST_HORIZON_S.to_bits());
        refuse(old, DT, StepReject::HorizonOutsideCard, "a step past seven days");
        refuse(old, 0.1, StepReject::HorizonOutsideCard, "a short step past seven days");
        // The run's own clock counts the same way.
        let mut long = good;
        long.t_s = FORECAST_HORIZON_S - 5.0;
        refuse(long, DT, StepReject::HorizonOutsideCard, "a run past seven days");
        // A start at seven days is a state; no step can be taken from it.
        let at_end = SatelliteState::try_state(
            &satellite,
            START_EPOCH_UTC_S,
            FORECAST_HORIZON_S,
            START_POSITION_M,
            START_VELOCITY_M_S,
        )
        .unwrap();
        refuse(at_end, DT, StepReject::HorizonOutsideCard, "a step from seven days");
        let stated = |elapsed_s: f64| {
            SatelliteState::try_state(
                &satellite,
                START_EPOCH_UTC_S,
                elapsed_s,
                START_POSITION_M,
                START_VELOCITY_M_S,
            )
        };
        assert!(stated(-1.0e-9).is_none());
        assert!(stated(f64::from_bits(FORECAST_HORIZON_S.to_bits() + 1)).is_none());
        assert!(stated(f64::NAN).is_none());
        assert!(stated(f64::INFINITY).is_none());
        let mut early = good;
        early.forecast_elapsed_at_start_s = -1.0;
        refuse(early, DT, StepReject::HorizonOutsideCard, "a stated time below zero");
        let mut broken = good;
        broken.forecast_elapsed_at_start_s = f64::NAN;
        refuse(broken, DT, StepReject::NonFiniteState, "NaN in the stated time");

        // A step far too long for the orbit: refused by name, not stepped.
        refuse(
            good,
            3_000.0,
            StepReject::RadiusOutsideCard,
            "a 50 minute step",
        );
    }

    #[test]
    fn the_shadow_is_only_a_flag() {
        let satellite = SatelliteLageos1::lageos1();
        let sun_m = sun_position_m(&satellite, START_EPOCH_UTC_S);
        let away = [
            -sun_m[0] / size(sun_m),
            -sun_m[1] / size(sun_m),
            -sun_m[2] / size(sun_m),
        ];
        let side = {
            let c = cross(away, [0.0, 0.0, 1.0]);
            [c[0] / size(c), c[1] / size(c), c[2] / size(c)]
        };
        // Directly behind the Earth from the Sun at 12 270 km, on the orbit
        // of LAGEOS-1.
        let radius_m = 12_270_000.0;
        let (position_m, velocity_m_s) =
            on_orbit(&satellite, away, radius_m, 12_270_000.0, 0.0039, 109.815);
        let mut state =
            fresh(&satellite, START_EPOCH_UTC_S, position_m, velocity_m_s)
                .unwrap();
        assert!(state.in_earth_shadow);
        assert!(!sunlit(&satellite, position_m, sun_m));
        // The other forces there have no sunlight term.
        let light = sunlight_acceleration_m_s2(&satellite, position_m, sun_m);
        assert_eq!(light, [0.0; 3]);
        let moon_m = moon_position_m(&satellite, START_EPOCH_UTC_S);
        let sun = third_body_acceleration_m_s2(position_m, sun_m, satellite.gm_sun_m3_s2);
        let moon = third_body_acceleration_m_s2(position_m, moon_m, satellite.gm_moon_m3_s2);
        let tide = tide_acceleration_m_s2(&satellite, position_m, sun_m, moon_m);
        let relativity = relativity_acceleration_m_s2(&satellite, position_m, velocity_m_s);
        let other =
            other_acceleration_m_s2(&satellite, START_EPOCH_UTC_S, position_m, velocity_m_s);
        let mut axis = 0;
        while axis < 3 {
            assert_eq!(
                other[axis].to_bits(),
                ((((sun[axis] + moon[axis]) + tide[axis]) + 0.0) + relativity[axis]).to_bits()
            );
            axis += 1;
        }
        // On the Sun's side of the Earth the same sphere feels 3.5e-9 m/s²,
        // pointing away from the Sun.
        let sunward_m = [-position_m[0], -position_m[1], -position_m[2]];
        assert!(sunlit(&satellite, sunward_m, sun_m));
        let light = sunlight_acceleration_m_s2(&satellite, sunward_m, sun_m);
        assert!(
            size(light) > 3.4e-9 && size(light) < 3.7e-9,
            "{}",
            size(light)
        );
        assert!(dot(light, away) > 0.999 * size(light));

        // The step lands, with the flag and nothing else.
        let mut frame = blank_frame();
        step(&satellite, &mut state, &mut frame, NO_LOAD, DT).unwrap();
        assert!(state.in_earth_shadow);
        assert_eq!(frame.flags, FLAG_EARTH_SHADOW);
        assert!(state.residual_conservation_error < 1.0e-13);
        proof_intact(&frame);

        // The edge of the shadow is one Earth radius from the line through
        // the Sun and the Earth, behind the Earth only.
        let at = |along_m: f64, across_m: f64| {
            [
                away[0] * along_m + side[0] * across_m,
                away[1] * along_m + side[1] * across_m,
                away[2] * along_m + side[2] * across_m,
            ]
        };
        assert!(!sunlit(&satellite, at(1.0e7, FIELD_RADIUS_M - 1.0), sun_m));
        assert!(sunlit(&satellite, at(1.0e7, FIELD_RADIUS_M + 1.0), sun_m));
        assert!(sunlit(&satellite, at(-1.0e7, 0.0), sun_m));
        assert!(sunlit(&satellite, at(-1.0, FIELD_RADIUS_M - 1.0), sun_m));

        // The start of the opened week is in sunlight: no flag.
        let mut state = start(&satellite);
        assert!(!state.in_earth_shadow);
        step(&satellite, &mut state, &mut frame, NO_LOAD, DT).unwrap();
        assert_eq!(frame.flags, 0);
        proof_intact(&frame);
    }

    #[test]
    fn the_frame_holds_the_position_to_thirty_billionths_of_a_metre() {
        let satellite = SatelliteLageos1::lageos1();
        let mut state = start(&satellite);
        let mut frame = blank_frame();
        let mut worst_m = 0.0;
        let mut worst_rounding_m = 0.0;
        let mut n = 0u32;
        // One revolution, every landed step.
        while n < 1_356 {
            step(&satellite, &mut state, &mut frame, NO_LOAD, DT).unwrap();
            n += 1;
            assert_eq!(frame.t.to_bits(), state.t_s.to_bits());
            let mut axis = 0;
            while axis < 3 {
                assert_eq!(frame.pos[axis], state.position_m[axis] as f32);
                // The 32-bit number alone is up to half a metre off.
                let rounding_m = abs_f64(f64::from(frame.pos[axis]) - state.position_m[axis]);
                assert!(rounding_m <= 0.5);
                assert!(abs_f64(f64::from(frame.vel[axis])) <= 0.5);
                if rounding_m > worst_rounding_m {
                    worst_rounding_m = rounding_m;
                }
                // The two slots added as 64-bit numbers are the position.
                let rebuilt_m = f64::from(frame.pos[axis]) + f64::from(frame.vel[axis]);
                let miss_m = abs_f64(rebuilt_m - state.position_m[axis]);
                assert!(miss_m <= 3.0e-8, "step {n} axis {axis}: {miss_m} m");
                if miss_m > worst_m {
                    worst_m = miss_m;
                }
                axis += 1;
            }
            assert_eq!(
                frame.force_torque,
                (state.radius_m - REFERENCE_RADIUS_M) as f32
            );
            assert_eq!(frame.residual, state.residual_conservation_error as f32);
            assert_eq!(frame.flags, 0);
        }
        // Measured over the revolution: 1.5e-8 m, against 0.49996 m for the
        // 32-bit number alone.
        assert!(worst_m > 0.0 && worst_m <= 1.5e-8, "{worst_m}");
        assert!(worst_rounding_m > 0.49 && worst_rounding_m <= 0.5);
        // The radius slot holds LAGEOS-1 between 12 223 and 12 319 km.
        assert!(
            f64::from(frame.force_torque) > -47_100.0 && f64::from(frame.force_torque) < 49_400.0
        );
        // The velocity is not in the frame. It is in the state.
        assert!(apart(state.velocity_m_s, STAND_IN_STATES[2].2) < 1.0e-7);
        proof_intact(&frame);
    }

    #[test]
    fn the_proof_bytes_are_never_touched() {
        let satellite = SatelliteLageos1::lageos1();
        let mut state = start(&satellite);
        let mut frame = blank_frame();
        let mut n = 0;
        while n < 4 {
            step(&satellite, &mut state, &mut frame, NO_LOAD, DT).unwrap();
            proof_intact(&frame);
            let bytes = frame.to_bytes();
            let mut index = 48;
            while index < 64 {
                assert_eq!(bytes[index], 0xA5);
                index += 1;
            }
            n += 1;
        }
        let got = step(&satellite, &mut state, &mut frame, NO_LOAD, 0.0);
        assert_eq!(got, Err(StepReject::NonPositiveDt));
        proof_intact(&frame);
        let got = step(&satellite, &mut state, &mut frame, NO_LOAD, f64::NAN);
        assert_eq!(got, Err(StepReject::NonFiniteInput));
        proof_intact(&frame);
        let got = step(&satellite, &mut state, &mut frame, NO_LOAD, 3_000.0);
        assert_eq!(got, Err(StepReject::RadiusOutsideCard));
        proof_intact(&frame);
        let got = step_euler(&satellite, &mut state, &mut frame, DT);
        assert_eq!(got, Err(StepReject::SpontaneousGeneration));
        proof_intact(&frame);
        let got = step_with_signs(&satellite, &mut state, &mut frame, DT, 1.0, -1.0);
        assert_eq!(got, Err(StepReject::FirstLawBreach));
        proof_intact(&frame);
        let mut faster = state;
        faster.velocity_m_s[0] *= 1.5;
        let got = step(&satellite, &mut faster, &mut frame, NO_LOAD, DT);
        assert_eq!(got, Err(StepReject::OrbitOutsideCard));
        proof_intact(&frame);
        let mut later = state;
        later.start_epoch_utc_s = EPOCH_MAX_S;
        let got = step(&satellite, &mut later, &mut frame, NO_LOAD, DT);
        assert_eq!(got, Err(StepReject::EpochOutsideCard));
        proof_intact(&frame);
        let mut broken = state;
        broken.position_m[0] = f64::NAN;
        let got = step(&satellite, &mut broken, &mut frame, NO_LOAD, DT);
        assert_eq!(got, Err(StepReject::NonFiniteState));
        proof_intact(&frame);
        let mut tampered = satellite;
        tampered.satellite_mass_kg = 405.38;
        let got = step(&tampered, &mut state, &mut frame, NO_LOAD, DT);
        assert_eq!(got, Err(StepReject::InvalidSatellite));
        proof_intact(&frame);
    }

    #[test]
    fn a_step_at_the_floor_moves_the_clock_and_every_integrated_quantity() {
        let card = SatelliteLageos1::lageos1();
        let ceiling_s = card.gate_dt_s().unwrap();
        let floor_s = card.floor_dt_s().unwrap();
        assert_eq!(GATE_OVER_FLOOR.to_bits(), 1_048_576.0_f64.to_bits());
        assert_eq!(floor_s.to_bits(), (ceiling_s / GATE_OVER_FLOOR).to_bits());
        assert_eq!(floor_s.to_bits(), 9.5367431640625e-6_f64.to_bits());

        // The clock. The card allows a forecast of seven days, and its dates
        // span 28 days. The Sun and the Moon are placed at the time counted
        // from 2000, at the start, the middle and the end of a step. Each of
        // those advances by the floor step, or by half of it, exactly.
        let dates_s = EPOCH_MAX_S - EPOCH_MIN_S;
        assert_eq!(dates_s.to_bits(), 2_419_200.0_f64.to_bits());
        for late_s in [FORECAST_HORIZON_S, FORECAST_HORIZON_S - 1.0, EPOCH_MAX_S - 1.0] {
            assert_eq!(((late_s + floor_s) - late_s).to_bits(), floor_s.to_bits());
            let half_s = 0.5 * floor_s;
            assert_eq!(((late_s + half_s) - late_s).to_bits(), half_s.to_bits());
        }
        // Where 64 bits stop: a step sixteen times smaller no longer puts the
        // middle of a step on the clock exactly.
        let half_s = 0.5 * (floor_s / 16.0);
        let late_s = EPOCH_MAX_S - 1.0;
        assert!(((late_s + half_s) - late_s).to_bits() != half_s.to_bits());

        // One step at the floor from the start of the opened week.
        let mut state = start(&card);
        let before = state;
        let mut frame = blank_frame();
        step(&card, &mut state, &mut frame, NO_LOAD, floor_s).unwrap();
        assert_eq!(state.t_s.to_bits(), floor_s.to_bits());
        let mut axis = 0;
        while axis < 3 {
            assert!(state.position_m[axis] != before.position_m[axis]);
            assert!(state.velocity_m_s[axis] != before.velocity_m_s[axis]);
            // What moved is the velocity times the step, to seven figures.
            let moved_m = state.position_m[axis] - before.position_m[axis];
            let want_m = before.velocity_m_s[axis] * floor_s;
            assert!(
                abs_f64(moved_m - want_m) < 2.0e-7 * abs_f64(want_m),
                "{moved_m} {want_m}"
            );
            axis += 1;
        }
        assert!(state.external_work_j_per_kg != 0.0);
        assert!(abs_f64(state.external_work_j_per_kg) > 1.0e-9);
        assert!(state.residual_conservation_error <= RESIDUAL_CONSERVATION_LIMIT);
        assert!(state.residual_conservation_error < 1.0e-14);

        // 4 095 more steps at the floor land: the ledger holds an honest
        // step this small. Measured in a release run of 16 777 216 floor
        // steps, 160 s of flight: at most 6.4e-14.
        let mut taken = 1;
        while taken < 4_096 {
            step(&card, &mut state, &mut frame, NO_LOAD, floor_s).unwrap();
            assert!(state.residual_conservation_error < 1.0e-13);
            taken += 1;
        }
        assert_eq!(state.t_s.to_bits(), (4_096.0 * floor_s).to_bits());
        assert_eq!(state.t_s.to_bits(), 0.0390625_f64.to_bits());
        // The same span in one step: the two agree to a tenth of a micrometre.
        let mut one = before;
        step(&card, &mut one, &mut frame, NO_LOAD, 4_096.0 * floor_s).unwrap();
        let miss_m = apart(state.position_m, one.position_m);
        assert!(miss_m < 1.0e-7, "{miss_m} m");
        assert!(apart(state.velocity_m_s, one.velocity_m_s) < 1.0e-9);

        // The last minute of the card, where the clock is coarsest.
        let mut late = fresh(
            &card,
            EPOCH_MAX_S - 60.0,
            START_POSITION_M,
            START_VELOCITY_M_S,
        )
        .unwrap();
        let before = late;
        let mut taken = 0;
        while taken < 64 {
            step(&card, &mut late, &mut frame, NO_LOAD, floor_s).unwrap();
            taken += 1;
        }
        assert_eq!(late.t_s.to_bits(), (64.0 * floor_s).to_bits());
        assert!(late.position_m[0] != before.position_m[0]);
        assert!(late.external_work_j_per_kg != 0.0);
        assert!(late.residual_conservation_error < 1.0e-14);
        proof_intact(&frame);
    }

    #[test]
    fn the_last_start_is_the_latest_the_smallest_step_can_use() {
        let card = SatelliteLageos1::lageos1();
        let floor_s = card.floor_dt_s().unwrap();
        let last_s = card.last_start_epoch_utc_s().unwrap();
        assert!(last_s < EPOCH_MAX_S);
        assert!(EPOCH_MAX_S - last_s < 2.0 * floor_s);
        // The published bound holds no allowance: one floor step before the
        // last date, to the spacing of 64-bit numbers there.
        assert_eq!(last_s, 843_134_399.999_990_5);
        let mut frame = blank_frame();
        let mut state =
            fresh(&card, last_s, START_POSITION_M, START_VELOCITY_M_S)
                .unwrap();
        step(&card, &mut state, &mut frame, NO_LOAD, floor_s).unwrap();
        assert!(state.epoch_utc_s() <= EPOCH_MAX_S);
        // A start past the bound by more than the allowance is refused at every step.
        let beyond_s = last_s + 1.0e-6;
        for dt_s in [floor_s, 2.0 * floor_s, 10.0] {
            let mut state =
                fresh(&card, beyond_s, START_POSITION_M, START_VELOCITY_M_S)
                    .unwrap();
            let before = state;
            assert_eq!(
                step(&card, &mut state, &mut frame, NO_LOAD, dt_s),
                Err(StepReject::EpochOutsideCard)
            );
            assert_eq!(state, before);
        }
    }

    /// The allowance is what its derivation says.
    #[test]
    fn the_epoch_end_allowance_is_its_derivation() {
        let card = SatelliteLageos1::lageos1();
        let spacing_s = f64::from_bits(EPOCH_MAX_S.to_bits() + 1) - EPOCH_MAX_S;
        assert_eq!(EPOCH_END_ALLOWANCE_S, spacing_s);
        assert_eq!(CLOCK_ROUNDING_PER_STEP, 0.5 * f64::EPSILON);
        assert_eq!(EPOCH_END_ALLOWANCE_EXTRA_STEPS, 2.0);
        // the first step of a run: the spacing at the epoch and a hair
        let first = card.epoch_end_allowance_s(0.0, 10.0);
        assert!(first > spacing_s && first < 1.000_001 * spacing_s);
        // after 999,999 steps of 0.1 s
        let late = card.epoch_end_allowance_s(99_999.9, 0.1);
        assert_eq!(
            late,
            EPOCH_END_ALLOWANCE_S + (99_999.9 / 0.1 + 2.0) * CLOCK_ROUNDING_PER_STEP * (99_999.9 + 0.1)
        );
        assert!(late > 1.1e-5 && late < 1.2e-5);
        // held to half the step
        assert_eq!(card.epoch_end_allowance_s(0.0, 1.0e-7), 0.5e-7);
        assert_eq!(card.epoch_end_allowance_s(0.0, 0.0), 0.0);
        assert_eq!(card.epoch_end_allowance_s(-1.0, 1.0), 0.0);
    }

    /// Runs that end on the last date in decimal land every step, at twelve
    /// step sizes over a thousand steps and at one over half a million, the
    /// length at which the running sum of the clock is late by more than the
    /// spacing at the epoch; and the step after is refused by name.
    #[test]
    fn a_decimal_run_to_the_last_date_lands_every_step() {
        let card = SatelliteLageos1::lageos1();
        // (the step in units of 0.1 s, steps)
        let runs: [(u64, u64); 13] = [
            (100, 1_000),
            (75, 1_000),
            (64, 1_000),
            (50, 1_000),
            (25, 1_000),
            (16, 1_000),
            (11, 1_000),
            (10, 1_000),
            (7, 1_000),
            (5, 1_000),
            (3, 1_000),
            (1, 1_000),
            (6, 500_000),
        ];
        for (dt_e1, count) in runs {
            let decimal = |tenths: u64| -> f64 {
                std::format!("{}.{}", tenths / 10, tenths % 10).parse().unwrap()
            };
            let dt_s = decimal(dt_e1);
            let start_s = decimal(8_431_344_000 - count * dt_e1);
            let mut state =
                fresh(&card, start_s, START_POSITION_M, START_VELOCITY_M_S)
                    .unwrap();
            let mut frame = blank_frame();
            for landed in 0..count {
                assert_eq!(
                    step(&card, &mut state, &mut frame, NO_LOAD, dt_s),
                    Ok(()),
                    "dt {dt_s}: step {} of {count}",
                    landed + 1
                );
            }
            let allowance_s = card.epoch_end_allowance_s(state.t_s - dt_s, dt_s);
            assert!((state.epoch_utc_s() - EPOCH_MAX_S).abs() <= allowance_s);
            if count == 500_000 {
                // the clock is late by more than the spacing at the epoch
                assert_eq!(state.t_s, 300_000.000_000_326_14);
                assert!(state.epoch_utc_s() > EPOCH_MAX_S);
            }
            let before = state;
            let held = frame.to_bytes();
            assert_eq!(
                step(&card, &mut state, &mut frame, NO_LOAD, dt_s),
                Err(StepReject::EpochOutsideCard)
            );
            assert_eq!(state, before);
            assert_eq!(frame.to_bytes(), held);
        }
    }

    /// A step that would end later than the last date by a spacing more than
    /// the allowance is refused by name and nothing moves; every end inside
    /// the allowance lands at its own value.
    #[test]
    fn a_step_beyond_the_epoch_end_allowance_is_refused_by_name() {
        let card = SatelliteLageos1::lageos1();
        let spacing_s = EPOCH_END_ALLOWANCE_S;
        // (the clock of the run at the start of the step, the step)
        for (t_s, dt_s) in [(0.0, 10.0), (0.0, 0.1), (99_999.9, 0.1), (299_999.4, 0.6)] {
            let allowance_s = card.epoch_end_allowance_s(t_s, dt_s);
            let whole_spacings = (((EPOCH_MAX_S + allowance_s) - EPOCH_MAX_S) / spacing_s) as u64;
            assert!(whole_spacings >= 1);
            let mut landed_past_the_end = 0;
            let mut refused = 0;
            for spacings in (0..whole_spacings + 4).rev().take(8).chain(0..2) {
                let end_s = f64::from_bits(EPOCH_MAX_S.to_bits() + spacings);
                // the start epoch whose state lands on this end
                let start_s = end_s - (t_s + dt_s);
                let mut state =
                    fresh(&card, start_s, START_POSITION_M, START_VELOCITY_M_S)
                        .unwrap();
                state.t_s = t_s;
                if state.start_epoch_utc_s + (t_s + dt_s) != end_s
                    || state.epoch_utc_s() + dt_s != end_s
                {
                    continue;
                }
                let before = state;
                let mut frame = blank_frame();
                let result = step(&card, &mut state, &mut frame, NO_LOAD, dt_s);
                if end_s <= EPOCH_MAX_S + allowance_s {
                    assert_eq!(result, Ok(()));
                    assert_eq!(state.epoch_utc_s(), end_s);
                    if spacings > 0 {
                        landed_past_the_end += 1;
                    }
                } else {
                    assert_eq!(result, Err(StepReject::EpochOutsideCard));
                    assert_eq!(state, before);
                    assert_eq!(frame.to_bytes(), blank_frame().to_bytes());
                    refused += 1;
                }
            }
            assert!(landed_past_the_end >= 1, "t {t_s} dt {dt_s}");
            assert!(refused >= 1, "t {t_s} dt {dt_s}");
            // one allowance and a half beyond, and a spacing, is refused
            let start_s = EPOCH_MAX_S + 1.5 * allowance_s + spacing_s - (t_s + dt_s);
            let mut state =
                fresh(&card, start_s, START_POSITION_M, START_VELOCITY_M_S)
                    .unwrap();
            state.t_s = t_s;
            let mut frame = blank_frame();
            assert_eq!(
                step(&card, &mut state, &mut frame, NO_LOAD, dt_s),
                Err(StepReject::EpochOutsideCard)
            );
        }
    }

    /// The start of each of the 56 compared runs, as the saved 0.2.1 requests
    /// in `evidence/runs_v021/satellite_lageos1_validation/` give it: the run,
    /// the epoch, the position and velocity in the card frame, and the steps
    /// of 10 s.
    #[cfg_attr(rustfmt, rustfmt::skip)]
    const COMPARED_RUNS: [(&str, f64, [f64; 3], [f64; 3], u32); 56] = [
    ("rehearsal_lageos1_260829_d0_60480_steps", 840715200.0, [3767126.6844992605, 8956617.400906362, 7564717.856833777], [4119.825668508274, 1676.7840658085788, -4053.2216929389388], 60480),
    ("rehearsal_lageos1_260829_d1_25920_steps", 840801600.0, [2018748.0571799346, -3983906.2060222127, -11378819.806905696], [-4125.733209268742, -4317.821686469918, 784.5467362741291], 25920),
    ("rehearsal_lageos1_260829_d2_25920_steps", 840888000.0, [-6937053.184080561, -2711998.7589558526, 9789318.55625625], [2125.9812500351977, 4830.578732180791, 2866.189134778212], 25920),
    ("rehearsal_lageos1_260829_d3_25920_steps", 840974400.0, [8610424.163039407, 8153979.652224977, -3172134.455924324], [951.6585399024611, -3042.1531056611047, -5148.87307150983], 25920),
    ("rehearsal_lageos1_260829_d4_8640_steps", 841060800.0, [-6140083.77899313, -9466802.423014391, -4740044.0267089605], [-3580.9668648232396, -149.18704535719792, 4893.620700464813], 8640),
    ("rehearsal_lageos1_260829_d5_8640_steps", 841147200.0, [599822.376476163, 6311768.806860098, 10562420.778412344], [4528.063429579714, 3247.674291121012, -2199.4590111285024], 8640),
    ("rehearsal_lageos1_260829_d6_1356_steps", 841233600.0, [5331275.489340223, -95409.25552207688, -11008827.678596484], [-3326.487938796062, -4775.169658620122, -1554.4632996910868], 1356),
    ("rehearsal_lageos1_260905_d0_60480_steps", 841320000.0, [-8756148.098841948, -5928386.823964605, 6250403.888405811], [501.38719542788965, 3977.6503358926925, 4516.433230224034], 60480),
    ("rehearsal_lageos1_260905_d1_25920_steps", 841406400.0, [7997232.78107024, 9156854.247944884, 1832470.113424584], [2647.520514545997, -1280.2784825829049, -5287.833860577359], 25920),
    ("rehearsal_lageos1_260905_d2_25920_steps", 841492800.0, [-3360301.7898122743, -7844375.814966433, -8756433.11355354], [-4537.6705173384225, -1958.2849447967467, 3482.943575379358], 25920),
    ("rehearsal_lageos1_260905_d3_25920_steps", 841579200.0, [-3019395.7016592203, 2886907.9023175007, 11586416.63115208], [4245.437706675399, 4213.096124371908, 64.21555423176152], 25920),
    ("rehearsal_lageos1_260905_d4_8640_steps", 841665600.0, [8063155.280604142, 3472994.6888182345, -8539774.08421911], [-1913.7038802084583, -4461.242768561849, -3592.414742063183], 8640),
    ("rehearsal_lageos1_260905_d5_8640_steps", 841752000.0, [-9241786.990637233, -7914815.263520515, 1533165.9435233695], [-1351.810438698523, 2575.445832010167, 5321.082813463062], 8640),
    ("rehearsal_lageos1_260905_d6_1356_steps", 841838400.0, [5927881.029875598, 8614795.858718254, 6494001.099115934], [4047.2931612676584, 537.7571191244601, -4432.855841934437], 1356),
    ("sealed_lageos1_260912_d0_60480_steps", 841924800.0, [295558.7678903363, -5108650.370522532, -11100383.23606335], [-4821.08544993592, -3331.9602315669836, 1405.5539438152634], 60480),
    ("sealed_lageos1_260912_d1_25920_steps", 842011200.0, [-6474340.391182555, -671875.433956757, 10444317.633032318], [3249.158226543766, 4475.6995413582645, 2319.120259123172], 25920),
    ("sealed_lageos1_260912_d2_25920_steps", 842097600.0, [9636917.439408831, 6151023.391114161, -4447224.265085683], [-76.80203516875571, -3497.868018565211, -4941.761809465462], 25920),
    ("sealed_lageos1_260912_d3_25920_steps", 842184000.0, [-8145328.666175985, -8458777.105037643, -3471955.2744702026], [-3134.367707968767, 889.8673328150794, 5118.659324507514], 25920),
    ("sealed_lageos1_260912_d4_8640_steps", 842270400.0, [2631509.2887506043, 6785318.79839571, 9937939.256380908], [4887.068368084381, 2124.842310904187, -2751.0897935060902], 8640),
    ("sealed_lageos1_260912_d5_8640_steps", 842356800.0, [4232422.145194259, -1839684.589292362, -11323610.630609944], [-4318.373933697098, -4091.343715146105, -938.5173364703617], 8640),
    ("sealed_lageos1_260912_d6_1356_steps", 842443200.0, [-9107816.662507689, -3763438.435888201, 7345405.309598077], [1640.238690742603, 4058.4272675295074, 4146.047560937712], 1356),
    ("sealed_lageos1_260919_d0_60480_steps", 842529600.0, [9646566.688196914, 7594617.836173506, 488431.1112540568], [1899.0661520637286, -2102.169966716554, -5351.792733780952], 60480),
    ("sealed_lageos1_260919_d1_25920_steps", 842616000.0, [-5507559.256118774, -7628157.538663178, -7815280.888658554], [-4524.796481751782, -789.9055414424793, 3940.0686182702593], 25920),
    ("sealed_lageos1_260919_d2_25920_steps", 842702400.0, [-1402335.4274658016, 4118894.5633427445, 11524334.61687353], [4979.688890508648, 3266.3448034016537, -557.6203427298377], 25920),
    ("sealed_lageos1_260919_d3_25920_steps", 842788800.0, [7738773.91335156, 1357912.4489553135, -9386238.86920616], [-3054.259423299698, -4181.574124416706, -3099.9371375818014], 25920),
    ("sealed_lageos1_260919_d4_8640_steps", 842875200.0, [-10349731.538846988, -5940148.091672636, 2864409.0452160127], [-359.3304253263271, 3088.0870643531634, 5199.5270319391875], 8640),
    ("sealed_lageos1_260919_d5_8640_steps", 842961600.0, [7954146.779041704, 7724397.158145512, 5334135.708676257], [3680.8172908031456, -533.5179917521267, -4753.1408824060545], 8640),
    ("sealed_lageos1_260919_d6_1356_steps", 843048000.0, [-1682299.9952795303, -5727603.92023848, -10666975.18020775], [-5241.526934880734, -2206.265319371338, 2007.1798931348142], 1356),
    ("sealed_lageos2_260829_d0_60480_steps", 840715200.0, [-7835780.523719332, -2211627.9980579205, -8978505.224845292], [-555.7598519992441, -4931.68307870167, 1597.8193435740316], 60480),
    ("sealed_lageos2_260829_d1_25920_steps", 840801600.0, [7563095.696389918, -502362.81494893454, 9600948.687238777], [1188.2513331298971, 4953.330809633568, -765.0055959362664], 25920),
    ("sealed_lageos2_260829_d2_25920_steps", 840888000.0, [-7047747.29156439, 2037908.9361762928, -9583492.965291334], [-1731.4880458275352, -4937.1023330851085, 142.08503830968297], 25920),
    ("sealed_lageos2_260829_d3_25920_steps", 840974400.0, [6100643.892477343, -4557986.183797557, 9630215.789206237], [2226.31768352778, 4568.8073073046635, 682.7746674028842], 25920),
    ("sealed_lageos2_260829_d4_8640_steps", 841060800.0, [-5067436.716848544, 5919507.892282011, -9163823.713472033], [-2704.214329943692, -4304.593987706823, -1345.5593030050552], 8640),
    ("sealed_lageos2_260829_d5_8640_steps", 841147200.0, [3616748.520370232, -7941208.900678507, 8682032.064763285], [2962.8170195192415, 3640.6776856680717, 2048.1429811466837], 8640),
    ("sealed_lageos2_260829_d6_1356_steps", 841233600.0, [-2166245.8786218227, 8904094.411594884, -7754108.1910079075], [-3296.9842524285486, -3123.8897616316303, -2699.832485855336], 1356),
    ("sealed_lageos2_260905_d0_60480_steps", 841320000.0, [458500.35125685186, -10229664.782615703, 6859122.037320271], [3279.1030973118804, 2303.986067454224, 3200.5287125085797], 60480),
    ("sealed_lageos2_260905_d1_25920_steps", 841406400.0, [1228719.415459287, 10591882.853281558, -5500665.738845802], [-3387.673847882116, -1568.2126773635355, -3765.715128021696], 25920),
    ("sealed_lageos2_260905_d2_25920_steps", 841492800.0, [-2926314.547992093, -11152455.38765623, 4346968.087280939], [3111.9828198243126, 736.3861122283488, 4030.703330717272], 25920),
    ("sealed_lageos2_260905_d3_25920_steps", 841579200.0, [4599492.256401409, 10775262.247286899, -2647270.722661206], [-2935.119268445362, 132.47767043490614, -4420.583313060292], 25920),
    ("sealed_lageos2_260905_d4_8640_steps", 841665600.0, [-6048167.645488046, -10620394.622376757, 1397286.2793176703], [2461.314074176348, -861.8826911731932, 4457.819524295062], 8640),
    ("sealed_lageos2_260905_d5_8640_steps", 841752000.0, [7420851.028726414, 9470630.857317409, 493637.88566264417], [-1989.5979865105146, 1731.1854539837823, -4592.502948469964], 8640),
    ("sealed_lageos2_260905_d6_1356_steps", 841838400.0, [-8440691.832667544, -8735668.73651865, -1693696.7646540073], [1392.22393601968, -2289.4007324336594, 4435.887948809895], 1356),
    ("sealed_lageos2_260912_d0_60480_steps", 841924800.0, [9245944.942874985, 6911440.410061576, 3580443.1716034017], [-683.8711993392474, 3007.0575151723224, -4269.634930527845], 60480),
    ("sealed_lageos2_260912_d1_25920_steps", 842011200.0, [-9724773.271500079, -5778988.118885716, -4611924.819919628], [31.112324882312578, -3366.185051117559, 3959.1781233901183], 25920),
    ("sealed_lageos2_260912_d2_25920_steps", 842097600.0, [9776280.744097315, 3504294.830354928, 6284712.285849432], [792.1905761538076, 3800.8293011349097, -3498.409988040518], 25920),
    ("sealed_lageos2_260912_d3_25920_steps", 842184000.0, [-9664351.601446351, -2174347.4593290347, -7054778.034489779], [-1445.4609725197847, -3956.1386142059173, 3065.5742144222063], 25920),
    ("sealed_lageos2_260912_d4_8640_steps", 842270400.0, [8904064.458420364, -240492.79794711788, 8327872.798092549], [2225.296773448182, 4034.661604150747, -2372.4355325224783], 8640),
    ("sealed_lageos2_260912_d5_8640_steps", 842356800.0, [-8207236.933020918, 1565739.5392674557, -8760706.989248207], [-2829.892919802975, -3987.093621153911, 1837.2138303837196], 8640),
    ("sealed_lageos2_260912_d6_1356_steps", 842443200.0, [6723334.981246833, -3788001.31416994, 9507540.776401624], [3411.94317548407, 3714.943189499106, -1017.0413682320585], 1356),
    ("sealed_lageos2_260919_d0_60480_steps", 842529600.0, [-5503963.950775675, 4910046.537485569, -9538357.134958329], [-3911.101883632213, -3465.1963359817264, 396.94249464223645], 60480),
    ("sealed_lageos2_260919_d1_25920_steps", 842616000.0, [3512292.0083620017, -6654139.515296457, 9713691.72884568], [4185.529165527377, 2922.115133680786, 426.398835794336], 25920),
    ("sealed_lageos2_260919_d2_25920_steps", 842702400.0, [-1897300.7436680433, 7386148.013277541, -9292346.57599241], [-4508.197003781527, -2479.143171576777, -1101.646041055129], 25920),
    ("sealed_lageos2_260919_d3_25920_steps", 842788800.0, [-308141.6223866581, -8468532.937951943, 8935322.563284291], [4435.076398006108, 1792.7223436039192, 1814.6944995601518], 25920),
    ("sealed_lageos2_260919_d4_8640_steps", 842875200.0, [2120881.8545856588, 8653874.711820194, -8039731.369313786], [-4504.162953251712, -1190.2869081199817, -2491.3014978004126], 8640),
    ("sealed_lageos2_260919_d5_8640_steps", 842961600.0, [-4231794.081053995, -9018418.763864005, 7257598.668901946], [4115.346683297348, 497.9782387227945, 3014.606530398559], 8640),
    ("sealed_lageos2_260919_d6_1356_steps", 843048000.0, [5981005.26521009, 8560416.98039933, -5911933.496294359], [-3871.3373597623813, 192.46225487972362, -3613.213410011136], 1356),
    ];

    /// Every compared run starts inside the card, in the band of its own
    /// satellite, and ends on or before the last date and the seven days.
    #[test]
    fn every_compared_run_starts_inside_the_card() {
        let card = SatelliteLageos1::lageos1();
        for (name, epoch_utc_s, position_m, velocity_m_s, steps) in COMPARED_RUNS {
            let state = fresh(&card, epoch_utc_s, position_m, velocity_m_s)
                .unwrap_or_else(|| ::core::panic!("{name} is refused"));
            let own = if name.contains("lageos1") { 0 } else { 1 };
            let a = card.orbit_semi_major_axis_m[own];
            assert!(state.semi_major_axis_m >= a[0] && state.semi_major_axis_m <= a[1], "{name}");
            let end_s = f64::from(steps) * DT;
            assert!(end_s <= FORECAST_HORIZON_S, "{name}");
            assert!(epoch_utc_s + end_s <= EPOCH_MAX_S, "{name}");
            assert!(epoch_utc_s >= EPOCH_MIN_S, "{name}");
        }
        // The first and the last of them are the ends of the dates.
        assert!(COMPARED_RUNS.iter().any(|run| run.1 == EPOCH_MIN_S));
        assert!(COMPARED_RUNS
            .iter()
            .any(|run| run.1 + f64::from(run.4) * DT == EPOCH_MAX_S && run.4 == 60_480));
    }

    /// The measurement behind the bands: every landing of the 56 compared
    /// runs, 1 255 008 steps, is inside the card. Slow in a debug build, so
    /// it is asked for by name: `cargo test -p genesis_kernel -- --ignored
    /// every_landing_of_the_compared_runs`.
    #[test]
    #[ignore]
    fn every_landing_of_the_compared_runs_is_inside_the_card() {
        let card = SatelliteLageos1::lageos1();
        let mut frame = blank_frame();
        for (name, epoch_utc_s, position_m, velocity_m_s, steps) in COMPARED_RUNS {
            let mut state = fresh(&card, epoch_utc_s, position_m, velocity_m_s).unwrap();
            for landed in 0..steps {
                assert_eq!(
                    step(&card, &mut state, &mut frame, NO_LOAD, DT),
                    Ok(()),
                    "{name}: step {}",
                    landed + 1
                );
            }
        }
    }

    /// A forecast carried on through restarts stops where a run straight
    /// through stops: at seven days from the published state.
    #[test]
    fn a_forecast_restarted_from_its_own_states_stops_at_seven_days() {
        let card = SatelliteLageos1::lageos1();
        let mut frame = blank_frame();
        // Seven restarts of a day each, every one from the state the last
        // returned, with the time since the published state it returned.
        let mut state = start(&card);
        for day in 0..7 {
            for _ in 0..8_640 {
                step(&card, &mut state, &mut frame, NO_LOAD, DT).unwrap();
            }
            assert_eq!(state.forecast_elapsed_s(), f64::from(day + 1) * 86_400.0);
            let next = SatelliteState::try_state(
                &card,
                state.epoch_utc_s(),
                state.forecast_elapsed_s(),
                state.position_m,
                state.velocity_m_s,
            )
            .unwrap();
            state = next;
        }
        let before = state;
        assert_eq!(
            step(&card, &mut state, &mut frame, NO_LOAD, DT),
            Err(StepReject::HorizonOutsideCard)
        );
        assert_eq!(state, before);
        // Straight through, the same end.
        let mut whole = start(&card);
        for _ in 0..60_480 {
            step(&card, &mut whole, &mut frame, NO_LOAD, DT).unwrap();
        }
        assert_eq!(whole.forecast_elapsed_s(), FORECAST_HORIZON_S);
        assert_eq!(whole.position_m, state.position_m);
        assert_eq!(
            step(&card, &mut whole, &mut frame, NO_LOAD, DT),
            Err(StepReject::HorizonOutsideCard)
        );
        // A run of decimal steps that ends on seven days in decimal lands its
        // last step, after a restart half way.
        let dt_s = 0.6;
        let mut first = start(&card);
        for _ in 0..1_000 {
            step(&card, &mut first, &mut frame, NO_LOAD, dt_s).unwrap();
        }
        let mut second = SatelliteState::try_state(
            &card,
            first.epoch_utc_s(),
            FORECAST_HORIZON_S - 1_000.0 * dt_s,
            first.position_m,
            first.velocity_m_s,
        )
        .unwrap();
        for landed in 0..1_000 {
            assert_eq!(
                step(&card, &mut second, &mut frame, NO_LOAD, dt_s),
                Ok(()),
                "step {}",
                landed + 1
            );
        }
        let allowance_s = card.horizon_end_allowance_s(second.forecast_elapsed_s() - dt_s, dt_s);
        assert!(abs_f64(second.forecast_elapsed_s() - FORECAST_HORIZON_S) <= allowance_s);
        assert_eq!(
            step(&card, &mut second, &mut frame, NO_LOAD, dt_s),
            Err(StepReject::HorizonOutsideCard)
        );
    }

    /// A single first step that ends exactly on the last date lands, for
    /// decimal steps, and so does one that ends exactly on seven days.
    #[test]
    fn a_single_first_step_to_the_last_date_or_seven_days_lands() {
        let card = SatelliteLageos1::lageos1();
        let mut frame = blank_frame();
        for dt_s in [10.0, 3.3, 0.1, 0.7, 1.9, 9.9, 2.5e-5] {
            let mut state =
                fresh(&card, EPOCH_MAX_S - dt_s, START_POSITION_M, START_VELOCITY_M_S).unwrap();
            assert_eq!(step(&card, &mut state, &mut frame, NO_LOAD, dt_s), Ok(()), "dt {dt_s}");
            assert_eq!(state.epoch_utc_s().to_bits(), EPOCH_MAX_S.to_bits(), "dt {dt_s}");
            let mut state = SatelliteState::try_state(
                &card,
                START_EPOCH_UTC_S,
                FORECAST_HORIZON_S - dt_s,
                START_POSITION_M,
                START_VELOCITY_M_S,
            )
            .unwrap();
            assert_eq!(step(&card, &mut state, &mut frame, NO_LOAD, dt_s), Ok(()), "dt {dt_s}");
            assert!(state.forecast_elapsed_s() <= FORECAST_HORIZON_S + card.horizon_end_allowance_s(0.0, dt_s));
        }
    }

    /// A run of `steps` steps of `dt_s`, and the same run cut in two with the
    /// last state of the first half handed back as a new start. The two final
    /// positions and velocities.
    fn whole_and_halved(steps: u32, dt_s: f64) -> ([[f64; 3]; 2], [[f64; 3]; 2]) {
        let card = SatelliteLageos1::lageos1();
        let mut frame = blank_frame();
        let mut whole =
            fresh(&card, START_EPOCH_UTC_S, START_POSITION_M, START_VELOCITY_M_S)
                .unwrap();
        for _ in 0..steps {
            step(&card, &mut whole, &mut frame, NO_LOAD, dt_s).unwrap();
        }
        let mut first =
            fresh(&card, START_EPOCH_UTC_S, START_POSITION_M, START_VELOCITY_M_S)
                .unwrap();
        for _ in 0..steps / 2 {
            step(&card, &mut first, &mut frame, NO_LOAD, dt_s).unwrap();
        }
        // The state's own epoch is the epoch of that state.
        assert_eq!(first.epoch_utc_s().to_bits(), (first.start_epoch_utc_s + first.t_s).to_bits());
        let mut second =
            fresh(&card, first.epoch_utc_s(), first.position_m, first.velocity_m_s)
                .unwrap();
        for _ in 0..steps - steps / 2 {
            step(&card, &mut second, &mut frame, NO_LOAD, dt_s).unwrap();
        }
        (
            [whole.position_m, second.position_m],
            [whole.velocity_m_s, second.velocity_m_s],
        )
    }

    #[test]
    fn a_final_state_handed_back_continues_the_same_orbit_to_the_bit_at_a_step_of_ten_seconds() {
        // The epoch 840715200 s and every multiple of 10 s are whole numbers
        // a 64-bit number holds exactly, so no time label is rounded and the
        // two runs take every stage at the same time to the bit.
        for steps in [2_u32, 60, 600, 6_000] {
            let (position_m, velocity_m_s) = whole_and_halved(steps, 10.0);
            for i in 0..3 {
                assert_eq!(position_m[0][i].to_bits(), position_m[1][i].to_bits(), "{steps} steps, position {i}");
                assert_eq!(velocity_m_s[0][i].to_bits(), velocity_m_s[1][i].to_bits(), "{steps} steps, velocity {i}");
            }
        }
    }

    #[test]
    fn a_final_state_handed_back_continues_the_same_orbit_within_a_nanometre_at_a_step_that_is_not_exact() {
        // These steps are not 64-bit numbers, so the elapsed time is a rounded
        // sum and the epoch is rounded to 1.2e-7 s: the two runs may take
        // their stages at times that differ in the last place. The bound is a
        // nanometre of position and a picometre per second of velocity; on
        // these five runs the two halves came out equal to the bit.
        for (steps, dt_s) in [(600_u32, 0.1), (777, 0.37), (2_000, 3.3), (4_001, 7.1), (30_000, 0.0123)] {
            let (position_m, velocity_m_s) = whole_and_halved(steps, dt_s);
            for i in 0..3 {
                let miss_m = (position_m[0][i] - position_m[1][i]).abs();
                assert!(miss_m < 1.0e-9, "{steps} steps of {dt_s} s, position {i}: {miss_m} m");
                let miss_m_s = (velocity_m_s[0][i] - velocity_m_s[1][i]).abs();
                assert!(miss_m_s < 1.0e-12, "{steps} steps of {dt_s} s, velocity {i}: {miss_m_s} m/s");
            }
        }
    }
}

