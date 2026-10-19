#![no_implicit_prelude]

//! The angular momentum of the James Webb Space Telescope at the Sun-Earth L2
//! point, growing under the pressure of sunlight on the two slopes of its
//! sunshield's first layer: the companion of `momentum_jwst_srp`, which takes
//! that layer as one flat surface.
//!
//! The observatory holds its attitude with six reaction wheels. Sunlight
//! pushes on the sunshield, and where the push does not pass through the
//! centre of mass it is a torque. The wheels take that torque up, so the
//! observatory's total angular momentum (wheels and body) grows between
//! momentum unloads, when thrusters take it out.
//!
//! The card is a rigid body whose momentum is held by its wheels. Its state is
//! the total angular momentum `H`, in the inertial frame of the flight
//! software (ECI, J2000 equatorial), in N m s. Its load is that of
//! `momentum_jwst_srp`, number for number: the attitude, the quaternion of
//! the ACS J-frame against ECI as the telemetry gives it (`SA_ZATTEST1` to
//! `SA_ZATTEST4` of the JWST engineering database, `q4` the scalar), and the
//! Sun's direction and distance from the observatory, which an ephemeris
//! gives. Over one step the load is held; the sunlight torque is then
//! constant, and the step is exact:
//!
//! ```text
//! H(t + dt) = H(t) + tau_ECI dt
//! ```
//!
//! The sunshield. Its five layers fan out from the spacecraft bus at dihedral
//! angles, so that no heat is trapped between them (Fellini and Kropp 2008,
//! Northrop Grumman Technology Review Journal, Spring/Summer 2008, p. 22),
//! and its shape is of planes and not one flat sheet (Bely et al. 2025,
//! arXiv:2501.09072, "a 3-plane configuration in lieu of a single flat one").
//! The Sun lights only the first layer over the field of regard (Arenberg et
//! al. 2016, SPIE 9904, 990405). The card takes that layer as two plane
//! slopes meeting along a ridge parallel to the boresight `J1`, one on each
//! side of it, each falling away from the ridge towards the Sun by the
//! dihedral angle `delta`:
//!
//! - the Sun-side normal of the slope on the `+J2` side is
//!   `(0, -sin delta, -cos delta)` in the J-frame, and of the slope on the
//!   `-J2` side `(0, +sin delta, -cos delta)`; with `delta` zero the two are
//!   the one flat layer of `momentum_jwst_srp`, normal `-J3`;
//! - each slope is half the lit area, 110 m², of the 220 m² that take the
//!   "~300 kW of power received from the Sun" of Arenberg et al. at
//!   1360.8 W/m², to two figures, the area of `momentum_jwst_srp`;
//! - each slope's push acts at its centroid, `w = W/6` from the ridge along
//!   `J2` and `w tan(delta)` from it towards the Sun, with `W` = 14.6 m, the
//!   deployed width of the sunshield (Menzel et al. 2023, PASP 135, 058002,
//!   figure 7): the centroid of half a kite of that width, measured from its
//!   long axis;
//! - each slope has the optical numbers of `momentum_jwst_srp`: its Sun side
//!   absorbs 0.47 of the sunlight and emits with an emittance of 0.72,
//!   end-of-life values of JWST's layer 1 (Origins Space Telescope Mission
//!   Concept Study Report, 2019, appendix C, arXiv:1912.06213); the rest, 0.53,
//!   is reflected, taken as specular; its back is VDA, emittance 0.025;
//! - the solar irradiance at 1 au is 1360.8 W/m² (Kopp and Lean 2011,
//!   Geophys. Res. Lett. 38, L01706), falling as the inverse square of the
//!   distance.
//!
//! Each slope is in radiative balance at every step, and its push is that of
//! `momentum_jwst_srp` on a layer of its normal and area. With `s` the unit
//! vector to the Sun in the J-frame, for each slope `k`:
//!
//! ```text
//! cos(theta_k) = n_k . s
//! S            = 1360.8 / d^2                    W/m², d in au
//! P_abs,k      = alpha S A_k cos(theta_k)
//! T_k^4        = alpha S cos(theta_k) / (sigma (eps_f + eps_b))
//! P_front,k    = eps_f sigma A_k T_k^4,  P_back,k = eps_b sigma A_k T_k^4
//! F_k          = -(S A_k cos(theta_k) / c) [ alpha s + 2 (1 - alpha) cos(theta_k) n_k ]
//!                - (2/3) (P_front,k / c) n_k
//! tau_J        = sum_k (r_ridge + g_k) x F_k     g_k = (0, +-w, -w tan(delta))
//! ```
//!
//! With the Sun rolled about the boresight towards `+J2`, the slope on the
//! `-J2` side faces it more squarely than the other and pushes harder, on the
//! far side of the ridge: the torque about `J1` changes with roll about four
//! times as fast as on one flat layer, which is what the observatory does.
//!
//! Four numbers are fitted, by least squares, to the observatory's own
//! momentum telemetry: the changes of `H` over the 1,404 hourly blocks of the
//! UTC days 2023-05-12 to 2023-07-14, settled minute to settled minute, away
//! from the unloads and the burns (`analysis/10_fit.py` of the research record
//! at `evidence/research/momentum_jwst_srp/companion_dihedral/`). They are the
//! point of the ridge at the sunshield's middle, from the centre of mass,
//! -0.0404585, 0.00776958 and -0.171418 m, and the dihedral angle,
//! 0.0704146 rad, 4.03 degrees: each slope's edge at the mid-booms, 7.3 m out,
//! lies 0.51 m nearer the Sun than the ridge. The area, the width and the
//! optical numbers are not fitted. Fitted on one of the two months and run on
//! the other, the dihedral came out at 0.0715 and 0.0692 rad.
//!
//! Compared once, forecast first, with the month of 2023-07-15 to
//! 2023-08-14, which no one had read: the protocol
//! (`companion_dihedral/validation_protocol.md`, SHA-256 da7dfe4b...dadb843)
//! was fingerprinted with the month's 310 files and committed in the gateway's
//! Git before any of them was opened, with the five marks of
//! `momentum_jwst_srp` (`companion_dihedral/RESULT.md` of the record): four
//! met, one missed, as forecast. The daily growth of `|H|` was within 0.1 of
//! the observed plus 0.2 N m s on 25 of 28 days (line 80 percent); `|H|` at the
//! end of four runs of 3.9 to 12.4 days between thruster events was within
//! 0.7 to 4.9 percent (line 10); the hourly growth rate's error was 0.272 of
//! its spread (line 0.5); the torque on one attitude was off by 7.59 against
//! 52.65 µN m rms (line 0.4 of it). The daily change of the vector, the mark
//! the two slopes were built to meet, was within its allowance on 18 of 28
//! days, and that mark was missed. Beyond 2 degrees of roll the torque about
//! `J1` is off by -1.6 and +1.0 µN m on average, where `momentum_jwst_srp` is
//! off by -28.9 and +27.3 on the same month; about `J3` both are off by 11 to
//! 13 µN m rms there. `momentum_jwst_srp` on that month met two of the five.
//!
//! The ledgers, at every step:
//!
//! - **Angular momentum.** The change of `H` equals the integrated external
//!   torque. The step turns the body torque into ECI twice, once by the
//!   quaternion's own product, `q tau_J q*`, which moves `H`, and once by the
//!   transpose of the telemetry's direction-cosine matrix (JWST-STScI-003222,
//!   SM-12, Eq. 24), which books the impulse `J`. The account is
//!   `H - H_anchor - J`; a step whose rotation is not the attitude's, or a
//!   momentum moved without its torque, is named `AngularMomentumBreach`.
//!   The account is held to 1e-9 of the impulse booked plus 1 N m s, what
//!   is in play in it, and not of `|H|`: a momentum so large that the 64-bit
//!   `H` does not carry the step's impulse into it to that share is named,
//!   and not hidden.
//! - **The quaternion's unit norm.** An attitude is a unit quaternion. The
//!   card refuses one whose norm `|q|` is outside 1 - 1e-6 to 1 + 1e-6, both
//!   inclusive, each edge the 64-bit number nearest its decimal, 0.999999 and
//!   1.000001 (`QuaternionNotUnit`), as `momentum_jwst_srp` does. It does not
//!   normalise one: the rotation it uses is that of `q / |q|`. The Sun's
//!   direction is held to the same edges (`SunDirectionNotUnit`).
//! - **Energy, where it applies: each slope of the first layer.** Absorbed
//!   equals emitted on each slope, `P_abs,k = P_front,k + P_back,k`, with
//!   `T_k` from the balance and the emitted power taken again from `T_k`.
//!   More emitted than absorbed on a slope is named `SpontaneousGeneration`,
//!   less `FirstLawBreach`. The body itself is held still, so the torque does
//!   no work on it; the wheels' energy comes from the observatory's electric
//!   power, and their speeds are not in the public telemetry, so no ledger of
//!   their energy is kept.
//!
//! `residual_conservation_error` is the larger of `|H - H_anchor - J|` over
//! `|J| + 1 N m s` and, on each slope, `|P_front,k + P_back,k - P_abs,k|`
//! over `P_abs,k`; both close to rounding, and the limit is 1e-9. The two
//! ledgers confirm the arithmetic of one rotation and of two balances:
//! that the torque the quaternion's product turns into ECI is the torque the
//! direction-cosine matrix books, and that each slope's emission, taken again
//! from its temperature, is what it absorbs. Neither is a measured quantity's
//! conservation over the run, since the step is exact over a held load.
//!
//! Where the card stops, each a limit of the mathematics or the physics:
//!
//! - [`StepReject::QuaternionNotUnit`] and
//!   [`StepReject::SunDirectionNotUnit`]: an attitude or a direction is a
//!   unit quaternion or a unit vector; one whose norm is outside 0.999999 to
//!   1.000001 is refused.
//! - [`StepReject::SunDistanceInsideSun`]: the Sun's distance at or under the
//!   Sun's radius, 695,700 km (the IAU's nominal solar radius, 2015
//!   Resolution B3) over the astronomical unit, 149,597,870.7 km (IAU 2012
//!   Resolution B2): 0.00465047 au. A distance of zero or below is no
//!   distance ([`StepReject::SunDistanceNotPositive`]).
//! - [`StepReject::SunBehindSunshield`]: the Sun at or behind the plane of
//!   either slope. That slope's front is not lit, and the light falls between
//!   the layers and on the telescope's side, which the card does not hold.
//!   With the slopes 4.03 degrees from the mean plane, this is met where the
//!   Sun is within 4.03 degrees of edge-on to the layer across the ridge, a
//!   roll of about 86 degrees.
//! - [`StepReject::SunshieldAboveKaptonLimit`]: a slope's balance temperature
//!   above 673.15 K, 400 °C, the top of the range NASA states for the
//!   stability of Kapton ("Sunshield Membrane Coatings", jwst.nasa.gov). At
//!   L2, with the Sun on the mean normal, each slope is at 347 K; the limit is
//!   met at about 0.27 au from the Sun.
//!
//! A flag, [`FLAG_OUTSIDE_FIELD_OF_REGARD`], marks a step whose Sun is outside
//! the science field of regard, pitch from -45 to +5 degrees and roll from -5
//! to +5 degrees (Arenberg et al. 2016, figure 7), where the sunshield is
//! built so that the Sun lights only its first layer; the step lands. Pitch
//! and roll are those of `momentum_jwst_srp`, taken from the mean plane: the
//! pitch `asin(s_J1 / |s|)`, the roll `asin(s_J2 / |(s_J2, s_J3)|)`, each the
//! angle of the direction as given, with nothing pulled back. The spent
//! months had the Sun at a pitch of -44.6 to +4.6 degrees and a roll of -5.5
//! to +6.1 degrees, the compared month -44.5 to +4.5 and -6.3 to +5.6.
//!
//! The telemetry's momentum departs by up to 3.5 N m s while the observatory
//! slews and returns when it settles; across each large slew the settled
//! momentum also moves by about 0.26 N m s rms that the sunlight does not
//! account for. The comparison used settled minutes only.
//!
//! What the card leaves out: the momentum trim flap at the aft end of the
//! sunshield (Menzel et al. 2023), whose area and angle are not published.
//! With them fitted beside the four numbers, the spent months gave two
//! solutions, its normal turned 23 to 30 degrees forward with 6 to 7 m², or
//! 74 to 80 degrees aft with 16 to 28 m²; fitted on one month and run on the
//! other, each moved the daily marks against the two slopes alone by at most
//! three days, up one way and down the other (`analysis/11_forecast.py`). The
//! spent months do not fix it, and the card leaves it out. A second dihedral about
//! `J2` (the forward and aft halves sloping apart), which the spent months did
//! not fix either. The other four layers, the edges and the curvature of the
//! membranes. The thrusters (an unload, a station-keeping burn), events of
//! the flight that a run does not cross. Light from the Earth and the Moon,
//! the spacecraft bus and its solar array, the gravity-gradient torque.
//! Diffuse reflection. The wheels' momentum capacity: the card has no
//! physical limit of its own on `|H|`. The arithmetic's limit stands: a
//! momentum so large that the 64-bit `H` does not carry a step's impulse
//! into it to 1e-9 of the impulse booked plus 1 N m s is named by the
//! momentum ledger.
//!
//! [`MomentumJwstDihedral::gate_dt_s`] is the time in which the Sun's
//! direction, seen from an observatory that goes round the Sun once a year,
//! turns by 0.01 rad: 50,226 s. The floor is the ceiling over 2^20, 0.0479 s,
//! under the telemetry's attitude sample of 0.256 s.
//!
//! `core` only: no heap and no standard prelude. The card, the state and the
//! load are `Copy`. An accepted step writes into [`LastStateFrame64`] the
//! time, `H` in ECI, the torque in ECI, `|H|` and the ledger residual, as
//! `momentum_jwst_srp` does. The 16-byte frame proof is not touched.

extern crate core;

use core::prelude::v1::*;

use crate::exceedance::{Exceedance, ExceedanceKind, Side};
use crate::frame::LastStateFrame64;
use crate::normal::{normal_or_zero, normal_or_zero_each};

/// The Sun's irradiance at 1 au, W/m² (Kopp and Lean 2011).
pub const SOLAR_IRRADIANCE_AT_1_AU_W_PER_M2: f64 = 1360.8;
/// The speed of light, m/s.
pub const SPEED_OF_LIGHT_M_PER_S: f64 = 299_792_458.0;
/// The Stefan-Boltzmann constant, W/(m² K^4) (CODATA 2018).
pub const STEFAN_BOLTZMANN_W_PER_M2_K4: f64 = 5.670_374_419e-8;
/// The sunshield's lit area, both slopes together, m²: 300 kW (Arenberg et
/// al. 2016) over 1360.8 W/m², to two figures.
pub const SUNSHIELD_AREA_M2: f64 = 220.0;
/// The share of sunlight the first layer's Sun side absorbs, end of life.
pub const LAYER1_SOLAR_ABSORPTANCE: f64 = 0.47;
/// The emittance of the first layer's Sun side, end of life.
pub const LAYER1_FRONT_EMITTANCE: f64 = 0.72;
/// The emittance of the first layer's back, vapour-deposited aluminium.
pub const LAYER1_BACK_EMITTANCE: f64 = 0.025;
/// The deployed width of the sunshield across the boresight, m (Menzel et
/// al. 2023, figure 7).
pub const SUNSHIELD_WIDTH_M: f64 = 14.6;
/// Where a slope's push acts, from the ridge along `J2`, m: the centroid of
/// half a kite of the sunshield's width, a sixth of it.
pub const SLOPE_CENTROID_FROM_RIDGE_M: f64 = SUNSHIELD_WIDTH_M / 6.0;
/// The angle by which each slope falls away from the ridge towards the Sun,
/// rad. Fitted.
pub const DIHEDRAL_RAD: f64 = 0.0704146;
/// The ridge's point at the sunshield's middle, from the centre of mass,
/// along J1, m. Fitted.
pub const RIDGE_J1_M: f64 = -0.0404585;
/// The ridge's point at the sunshield's middle, from the centre of mass,
/// along J2, m. Fitted.
pub const RIDGE_J2_M: f64 = 0.00776958;
/// The ridge's point at the sunshield's middle, from the centre of mass,
/// along J3, m. Fitted.
pub const RIDGE_J3_M: f64 = -0.171418;
/// The top of the range of temperature NASA states for Kapton, K (400 °C).
pub const KAPTON_LIMIT_K: f64 = 673.15;
/// The Sun's radius, km: the IAU's nominal solar radius (2015 Resolution B3).
pub const SUN_RADIUS_KM: f64 = 695_700.0;
/// The astronomical unit, km (IAU 2012 Resolution B2).
pub const ASTRONOMICAL_UNIT_KM: f64 = 149_597_870.7;
/// The Sun's radius, au: 0.00465047. A distance at or under it is inside the Sun.
pub const SUN_RADIUS_AU: f64 = SUN_RADIUS_KM / ASTRONOMICAL_UNIT_KM;
/// How far from one the norm of the quaternion and of the Sun's direction
/// may be, either way: the norm from `1 - 1e-6` to `1 + 1e-6`, both
/// inclusive, each the 64-bit number nearest 0.999999 and 1.000001.
pub const UNIT_NORM_TOLERANCE: f64 = 1.0e-6;
/// The science field of regard: the lowest pitch, degrees.
pub const FIELD_OF_REGARD_PITCH_MIN_DEG: f64 = -45.0;
/// The science field of regard: the highest pitch, degrees.
pub const FIELD_OF_REGARD_PITCH_MAX_DEG: f64 = 5.0;
/// The science field of regard: the largest roll either way, degrees.
pub const FIELD_OF_REGARD_ROLL_MAX_DEG: f64 = 5.0;
/// One Julian year, s: the time the observatory takes round the Sun.
pub const YEAR_S: f64 = 31_557_600.0;
/// The Sun's turn, seen from the observatory, over the step ceiling, rad.
pub const SUN_TURN_AT_CEILING_RAD: f64 = 0.01;
/// Step ceiling divided by step floor: 2^20.
pub const GATE_OVER_FLOOR: f64 = 1_048_576.0;
/// The momentum added to the impulse booked in the ledger's scale, N m s.
pub const MOMENTUM_SCALE_FLOOR_NMS: f64 = 1.0;
/// The ledger's residual must stay at or under this.
pub const RESIDUAL_CONSERVATION_LIMIT: f64 = 1.0e-9;

/// The Sun is outside the science field of regard, pitch -45 to +5 degrees and
/// roll -5 to +5 degrees, where the sunshield is built so that the Sun lights
/// only its first layer.
pub const FLAG_OUTSIDE_FIELD_OF_REGARD: u64 = 1 << 0;

/// Why a step did not advance the momentum or the frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StepReject {
    /// `dt` or a number of the load was NaN or infinite.
    NonFiniteInput,
    /// The incoming state, or a number of the trial step, was not finite.
    NonFiniteState,
    /// `dt` was zero or negative.
    NonPositiveDt,
    /// A recorded constant is not this observatory's.
    InvalidObservatory,
    /// The attitude quaternion's norm is outside 0.999999 to 1.000001: it is not an attitude.
    QuaternionNotUnit,
    /// The Sun's direction's norm is outside 0.999999 to 1.000001: it is not a unit vector.
    SunDirectionNotUnit,
    /// The Sun's distance is zero or negative.
    SunDistanceNotPositive,
    /// The Sun's distance is at or under the Sun's radius, 0.00465047 au (695,700 km): the observatory would be inside the Sun.
    SunDistanceInsideSun,
    /// The Sun is at or behind the plane of a slope of the sunshield's first layer: that slope's front is not lit, and the light falls between the layers and on the telescope's side, which the card does not hold.
    SunBehindSunshield,
    /// A slope of the first layer has a balance temperature above 673.15 K, 400 °C, the top of the range NASA states for the stability of Kapton.
    SunshieldAboveKaptonLimit,
    /// The trial step moved the angular momentum by other than the torque it booked.
    AngularMomentumBreach,
    /// The trial step had a slope of the first layer emit more than it absorbed.
    SpontaneousGeneration,
    /// The trial step had a slope of the first layer emit less than it absorbed.
    FirstLawBreach,
}

/// Held across one step: the attitude and the Sun, as for `momentum_jwst_srp`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MomentumJwstDihedralLoad {
    /// `SA_ZATTEST1`: the attitude quaternion's first vector component.
    pub q1: f64,
    /// `SA_ZATTEST2`: the second vector component.
    pub q2: f64,
    /// `SA_ZATTEST3`: the third vector component.
    pub q3: f64,
    /// `SA_ZATTEST4`: the scalar component.
    pub q4: f64,
    /// The unit vector from the observatory to the Sun, ECI x.
    pub sun_x: f64,
    /// The unit vector from the observatory to the Sun, ECI y.
    pub sun_y: f64,
    /// The unit vector from the observatory to the Sun, ECI z.
    pub sun_z: f64,
    /// The observatory's distance from the Sun, au.
    pub sun_distance_au: f64,
}

/// The observatory. Every field is a recorded copy of a crate constant and
/// none is a caller input.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MomentumJwstDihedral {
    /// Recorded copy of [`SOLAR_IRRADIANCE_AT_1_AU_W_PER_M2`].
    pub irradiance_1_au_w_per_m2: f64,
    /// Recorded copy of [`SPEED_OF_LIGHT_M_PER_S`].
    pub speed_of_light_m_per_s: f64,
    /// Recorded copy of [`STEFAN_BOLTZMANN_W_PER_M2_K4`].
    pub stefan_boltzmann_w_per_m2_k4: f64,
    /// Recorded copy of [`SUNSHIELD_AREA_M2`].
    pub area_m2: f64,
    /// Recorded copy of [`LAYER1_SOLAR_ABSORPTANCE`].
    pub absorptance: f64,
    /// Recorded copy of [`LAYER1_FRONT_EMITTANCE`].
    pub front_emittance: f64,
    /// Recorded copy of [`LAYER1_BACK_EMITTANCE`].
    pub back_emittance: f64,
    /// Recorded copy of [`SLOPE_CENTROID_FROM_RIDGE_M`].
    pub slope_centroid_m: f64,
    /// Recorded copy of [`DIHEDRAL_RAD`].
    pub dihedral_rad: f64,
    /// Recorded copies of [`RIDGE_J1_M`], [`RIDGE_J2_M`] and [`RIDGE_J3_M`].
    pub ridge_m: [f64; 3],
    /// Recorded copy of [`KAPTON_LIMIT_K`].
    pub kapton_limit_k: f64,
    /// Recorded copy of [`SUN_RADIUS_AU`].
    pub sun_radius_au: f64,
    /// Recorded copy of [`UNIT_NORM_TOLERANCE`].
    pub unit_norm_tolerance: f64,
    /// Recorded copy of [`FIELD_OF_REGARD_PITCH_MIN_DEG`].
    pub pitch_min_deg: f64,
    /// Recorded copy of [`FIELD_OF_REGARD_PITCH_MAX_DEG`].
    pub pitch_max_deg: f64,
    /// Recorded copy of [`FIELD_OF_REGARD_ROLL_MAX_DEG`].
    pub roll_max_deg: f64,
    /// Recorded copy of [`YEAR_S`].
    pub year_s: f64,
    /// Recorded copy of [`SUN_TURN_AT_CEILING_RAD`].
    pub sun_turn_at_ceiling_rad: f64,
    /// Recorded copy of [`GATE_OVER_FLOOR`].
    pub gate_over_floor: f64,
    /// Recorded copy of [`MOMENTUM_SCALE_FLOOR_NMS`].
    pub momentum_scale_floor_nms: f64,
    /// Recorded copy of [`RESIDUAL_CONSERVATION_LIMIT`].
    pub residual_conservation_limit: f64,
}

impl MomentumJwstDihedral {
    /// Card: JWST's angular momentum under sunlight on the two slopes of its
    /// sunshield. The constructor takes no arguments.
    #[must_use]
    pub fn observatory() -> Self {
        let card = Self {
            irradiance_1_au_w_per_m2: SOLAR_IRRADIANCE_AT_1_AU_W_PER_M2,
            speed_of_light_m_per_s: SPEED_OF_LIGHT_M_PER_S,
            stefan_boltzmann_w_per_m2_k4: STEFAN_BOLTZMANN_W_PER_M2_K4,
            area_m2: SUNSHIELD_AREA_M2,
            absorptance: LAYER1_SOLAR_ABSORPTANCE,
            front_emittance: LAYER1_FRONT_EMITTANCE,
            back_emittance: LAYER1_BACK_EMITTANCE,
            slope_centroid_m: SLOPE_CENTROID_FROM_RIDGE_M,
            dihedral_rad: DIHEDRAL_RAD,
            ridge_m: [RIDGE_J1_M, RIDGE_J2_M, RIDGE_J3_M],
            kapton_limit_k: KAPTON_LIMIT_K,
            sun_radius_au: SUN_RADIUS_AU,
            unit_norm_tolerance: UNIT_NORM_TOLERANCE,
            pitch_min_deg: FIELD_OF_REGARD_PITCH_MIN_DEG,
            pitch_max_deg: FIELD_OF_REGARD_PITCH_MAX_DEG,
            roll_max_deg: FIELD_OF_REGARD_ROLL_MAX_DEG,
            year_s: YEAR_S,
            sun_turn_at_ceiling_rad: SUN_TURN_AT_CEILING_RAD,
            gate_over_floor: GATE_OVER_FLOOR,
            momentum_scale_floor_nms: MOMENTUM_SCALE_FLOOR_NMS,
            residual_conservation_limit: RESIDUAL_CONSERVATION_LIMIT,
        };
        if card.is_valid() {
            card
        } else {
            ::core::panic!("momentum card failed is_valid");
        }
    }

    /// Every recorded copy equals its crate constant bit for bit.
    #[must_use]
    pub fn is_valid(&self) -> bool {
        self.irradiance_1_au_w_per_m2.to_bits() == SOLAR_IRRADIANCE_AT_1_AU_W_PER_M2.to_bits()
            && self.speed_of_light_m_per_s.to_bits() == SPEED_OF_LIGHT_M_PER_S.to_bits()
            && self.stefan_boltzmann_w_per_m2_k4.to_bits() == STEFAN_BOLTZMANN_W_PER_M2_K4.to_bits()
            && self.area_m2.to_bits() == SUNSHIELD_AREA_M2.to_bits()
            && self.absorptance.to_bits() == LAYER1_SOLAR_ABSORPTANCE.to_bits()
            && self.front_emittance.to_bits() == LAYER1_FRONT_EMITTANCE.to_bits()
            && self.back_emittance.to_bits() == LAYER1_BACK_EMITTANCE.to_bits()
            && self.slope_centroid_m.to_bits() == SLOPE_CENTROID_FROM_RIDGE_M.to_bits()
            && self.dihedral_rad.to_bits() == DIHEDRAL_RAD.to_bits()
            && self.ridge_m[0].to_bits() == RIDGE_J1_M.to_bits()
            && self.ridge_m[1].to_bits() == RIDGE_J2_M.to_bits()
            && self.ridge_m[2].to_bits() == RIDGE_J3_M.to_bits()
            && self.kapton_limit_k.to_bits() == KAPTON_LIMIT_K.to_bits()
            && self.sun_radius_au.to_bits() == SUN_RADIUS_AU.to_bits()
            && self.unit_norm_tolerance.to_bits() == UNIT_NORM_TOLERANCE.to_bits()
            && self.pitch_min_deg.to_bits() == FIELD_OF_REGARD_PITCH_MIN_DEG.to_bits()
            && self.pitch_max_deg.to_bits() == FIELD_OF_REGARD_PITCH_MAX_DEG.to_bits()
            && self.roll_max_deg.to_bits() == FIELD_OF_REGARD_ROLL_MAX_DEG.to_bits()
            && self.year_s.to_bits() == YEAR_S.to_bits()
            && self.sun_turn_at_ceiling_rad.to_bits() == SUN_TURN_AT_CEILING_RAD.to_bits()
            && self.gate_over_floor.to_bits() == GATE_OVER_FLOOR.to_bits()
            && self.momentum_scale_floor_nms.to_bits() == MOMENTUM_SCALE_FLOOR_NMS.to_bits()
            && self.residual_conservation_limit.to_bits() == RESIDUAL_CONSERVATION_LIMIT.to_bits()
    }

    /// Largest step the card stands behind: the Sun's direction turns by
    /// 0.01 rad, 50,226 s. Not applied by [`step`].
    #[must_use]
    pub fn gate_dt_s(&self) -> Option<f64> {
        let gate = self.sun_turn_at_ceiling_rad * self.year_s / (2.0 * core::f64::consts::PI);
        if gate.is_finite() && gate > 0.0 { Some(gate) } else { None }
    }

    /// Smallest step the card stands behind: the ceiling over 2^20. Not
    /// applied by [`step`].
    #[must_use]
    pub fn floor_dt_s(&self) -> Option<f64> {
        match self.gate_dt_s() {
            Some(gate) => Some(gate / self.gate_over_floor),
            None => None,
        }
    }

    /// The two slopes, the one on the `+J2` side first: each one's Sun-side
    /// normal and its centroid from the ridge, J-frame.
    fn slopes(&self) -> [([f64; 3], [f64; 3]); 2] {
        let s = crate::math::sin(self.dihedral_rad);
        let c = crate::math::cos(self.dihedral_rad);
        let drop = -self.slope_centroid_m * s / c;
        [([0.0, -s, -c], [0.0, self.slope_centroid_m, drop]), ([0.0, s, -c], [0.0, -self.slope_centroid_m, drop])]
    }
}

/// The observatory's momentum. The published fields are part of the state;
/// write them only through [`step`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MomentumJwstDihedralState {
    /// Time since the start of the run, s.
    pub t_s: f64,
    /// Total angular momentum, ECI, N m s.
    pub h_eci_nms: [f64; 3],
    /// `|H|`, N m s.
    pub h_magnitude_nms: f64,
    /// `H` at the start of the run, ECI, N m s.
    pub h_anchor_eci_nms: [f64; 3],
    /// The angular impulse booked since the start of the run, ECI, N m s.
    pub impulse_eci_nms: [f64; 3],
    /// `|H - H_anchor - J|`, N m s.
    pub momentum_defect_nms: f64,
    /// The sunlight torque of the last step, ECI, N m.
    pub torque_eci_nm: [f64; 3],
    /// The sunlight torque of the last step, J-frame, N m.
    pub torque_body_nm: [f64; 3],
    /// How fast `|H|` grows, `H . tau / |H|` (`|tau|` where `H` is zero), N m.
    pub h_growth_nm: f64,
    /// The sunlight force on the sunshield, both slopes, its size, N.
    pub force_n: f64,
    /// The Sun's pitch: its angle towards the boresight `J1`, degrees.
    pub sun_pitch_deg: f64,
    /// The Sun's roll: its angle out of the `J1`-`J3` plane, degrees.
    pub sun_roll_deg: f64,
    /// The Sun's incidence on the layer's mean plane, normal `-J3`, as `momentum_jwst_srp` gives it.
    pub incidence_cosine: f64,
    /// The Sun's incidence on the slope on the `+J2` side.
    pub slope_plus_j2_incidence_cosine: f64,
    /// The Sun's incidence on the slope on the `-J2` side.
    pub slope_minus_j2_incidence_cosine: f64,
    /// The sunlight at the observatory, W/m².
    pub irradiance_w_per_m2: f64,
    /// The balance temperature of the slope on the `+J2` side, K.
    pub slope_plus_j2_temperature_k: f64,
    /// The balance temperature of the slope on the `-J2` side, K.
    pub slope_minus_j2_temperature_k: f64,
    /// The power the two slopes absorb, W.
    pub absorbed_w: f64,
    /// The power their Sun sides emit, W.
    pub emitted_front_w: f64,
    /// The power their backs emit, W.
    pub emitted_back_w: f64,
    /// `P_front + P_back - P_abs` over both slopes, W. Positive is energy from nowhere.
    pub energy_defect_w: f64,
    /// `| |q| - 1 |` of the attitude.
    pub quaternion_norm_defect: f64,
    /// The larger of the two ledgers' residuals, the energy ledger's on its worse slope.
    pub residual_conservation_error: f64,
    /// The Sun was outside the science field of regard on the last step.
    pub outside_field_of_regard: bool,
}

impl MomentumJwstDihedralState {
    /// Start at this momentum, ECI, N m s, at time 0, the ledger anchored
    /// there with nothing booked. `None` for a momentum that is not finite.
    #[must_use]
    pub fn try_start(card: &MomentumJwstDihedral, h_eci_nms: [f64; 3]) -> Option<Self> {
        if !card.is_valid() || !(h_eci_nms[0].is_finite() && h_eci_nms[1].is_finite() && h_eci_nms[2].is_finite()) {
            return None;
        }
        let magnitude = norm(h_eci_nms);
        if !magnitude.is_finite() {
            return None;
        }
        Some(Self {
            t_s: 0.0,
            h_eci_nms,
            h_magnitude_nms: magnitude,
            h_anchor_eci_nms: h_eci_nms,
            impulse_eci_nms: [0.0; 3],
            momentum_defect_nms: 0.0,
            torque_eci_nm: [0.0; 3],
            torque_body_nm: [0.0; 3],
            h_growth_nm: 0.0,
            force_n: 0.0,
            sun_pitch_deg: 0.0,
            sun_roll_deg: 0.0,
            incidence_cosine: 0.0,
            slope_plus_j2_incidence_cosine: 0.0,
            slope_minus_j2_incidence_cosine: 0.0,
            irradiance_w_per_m2: 0.0,
            slope_plus_j2_temperature_k: 0.0,
            slope_minus_j2_temperature_k: 0.0,
            absorbed_w: 0.0,
            emitted_front_w: 0.0,
            emitted_back_w: 0.0,
            energy_defect_w: 0.0,
            quaternion_norm_defect: 0.0,
            residual_conservation_error: 0.0,
            outside_field_of_regard: false,
        })
    }
}

fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]]
}

fn add(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}

fn norm(a: [f64; 3]) -> f64 {
    dot(a, a).sqrt()
}

fn finite3(a: [f64; 3]) -> bool {
    a[0].is_finite() && a[1].is_finite() && a[2].is_finite()
}

/// The attitude's direction-cosine matrix, ECI to J-frame: rows are the
/// J-frame's axes in ECI (SM-12 Eq. 24), in the homogeneous form divided by
/// `|q|^2`, which for a unit quaternion is the same matrix.
fn eci_to_j(load: &MomentumJwstDihedralLoad) -> [[f64; 3]; 3] {
    let (q1, q2, q3, q4) = (load.q1, load.q2, load.q3, load.q4);
    let n2 = q1 * q1 + q2 * q2 + q3 * q3 + q4 * q4;
    [
        [
            (q4 * q4 + q1 * q1 - q2 * q2 - q3 * q3) / n2,
            2.0 * (q1 * q2 + q3 * q4) / n2,
            2.0 * (q3 * q1 - q2 * q4) / n2,
        ],
        [
            2.0 * (q1 * q2 - q3 * q4) / n2,
            (q4 * q4 - q1 * q1 + q2 * q2 - q3 * q3) / n2,
            2.0 * (q2 * q3 + q1 * q4) / n2,
        ],
        [
            2.0 * (q3 * q1 + q2 * q4) / n2,
            2.0 * (q2 * q3 - q1 * q4) / n2,
            (q4 * q4 - q1 * q1 - q2 * q2 + q3 * q3) / n2,
        ],
    ]
}

fn times(m: &[[f64; 3]; 3], v: [f64; 3]) -> [f64; 3] {
    [dot(m[0], v), dot(m[1], v), dot(m[2], v)]
}

fn transpose_times(m: &[[f64; 3]; 3], v: [f64; 3]) -> [f64; 3] {
    [
        m[0][0] * v[0] + m[1][0] * v[1] + m[2][0] * v[2],
        m[0][1] * v[0] + m[1][1] * v[1] + m[2][1] * v[2],
        m[0][2] * v[0] + m[1][2] * v[1] + m[2][2] * v[2],
    ]
}

/// A J-frame vector in ECI by the quaternion's product `q v q*` (Hamilton,
/// scalar `q4`), divided by `|q|^2`: `v + (2/|q|^2) (w u x v + u x (u x v))`.
fn j_to_eci_by_product(load: &MomentumJwstDihedralLoad, v: [f64; 3]) -> [f64; 3] {
    let u = [load.q1, load.q2, load.q3];
    let w = load.q4;
    let n2 = load.q1 * load.q1 + load.q2 * load.q2 + load.q3 * load.q3 + load.q4 * load.q4;
    let uv = cross(u, v);
    let uuv = cross(u, uv);
    let k = 2.0 / n2;
    [
        v[0] + k * (w * uv[0] + uuv[0]),
        v[1] + k * (w * uv[1] + uuv[1]),
        v[2] + k * (w * uv[2] + uuv[2]),
    ]
}

fn degrees(rad: f64) -> f64 {
    rad * (180.0 / core::f64::consts::PI)
}

/// The sine of the Sun's roll about `J1`, `y / sqrt(y^2 + z^2)`, with both
/// divided first by the larger of the two in size, so that no square
/// underflows: the larger is then one exactly, the root at least one, and
/// the ratio at most one in size. `z` is not zero, since the Sun is lit.
fn roll_sine(y: f64, z: f64) -> f64 {
    let larger = if y.abs() > z.abs() { y.abs() } else { z.abs() };
    let (y, z) = (y / larger, z / larger);
    y / (y * y + z * z).sqrt()
}

/// What the Sun does to one slope on one step.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Slope {
    incidence: f64,
    temperature_k: f64,
    absorbed_w: f64,
    emitted_front_w: f64,
    emitted_back_w: f64,
    force_j_n: [f64; 3],
    torque_j_nm: [f64; 3],
}

/// What the Sun does to the sunshield on one step.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Sunlight {
    incidence: f64,
    irradiance: f64,
    slopes: [Slope; 2],
    force_j_n: [f64; 3],
    torque_j_nm: [f64; 3],
    pitch_deg: f64,
    roll_deg: f64,
    quaternion_norm_defect: f64,
}

/// How a step is carried out. The card's own is the attitude's rotation on
/// both routes and the full balance of each slope; the tests run the others
/// and require a ledger to name them.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Method {
    /// Move `H` by the torque turned with the inverse rotation.
    inverse_rotation: bool,
    /// Leave the back's emission out of the slopes' balance.
    no_back_emission: bool,
}

const OWN_METHOD: Method = Method {
    inverse_rotation: false,
    no_back_emission: false,
};

/// Advance one exact step and write the frame. A refused step leaves `state`
/// and `frame` as they were.
///
/// A value of the landed state below the normal range is written as zero
/// with its sign ([`crate::normal`]).
#[must_use]
pub fn step(
    card: &MomentumJwstDihedral,
    state: &mut MomentumJwstDihedralState,
    frame: &mut LastStateFrame64,
    load: MomentumJwstDihedralLoad,
    dt_s: f64,
) -> Result<(), StepReject> {
    #[cfg(any(test, debug_assertions, feature = "check_exceedances"))]
    let before = *state;
    let stepped = step_landed(card, state, frame, load, dt_s);
    #[cfg(any(test, debug_assertions, feature = "check_exceedances"))]
    if stepped.is_ok() {
        crate::exceedance::debug_check("momentum_jwst_dihedral", exceedances(card, &before, state, load, dt_s), frame.flags);
    }
    stepped
}

/// The step itself; [`step`] is it with, in a build that checks
/// (`crate::exceedance::CHECKED`), the check that [`exceedances`] stands for
/// the flags it wrote.
fn step_landed(
    card: &MomentumJwstDihedral,
    state: &mut MomentumJwstDihedralState,
    frame: &mut LastStateFrame64,
    load: MomentumJwstDihedralLoad,
    dt_s: f64,
) -> Result<(), StepReject> {
    step_as_computed(card, state, frame, load, dt_s)?;
    normal_or_zero_state(state);
    Ok(())
}

/// The step as its equations compute it, before a value of the landed
/// state below the normal range is written as zero.
fn step_as_computed(
    card: &MomentumJwstDihedral,
    state: &mut MomentumJwstDihedralState,
    frame: &mut LastStateFrame64,
    load: MomentumJwstDihedralLoad,
    dt_s: f64,
) -> Result<(), StepReject> {
    step_with(card, state, frame, load, dt_s, OWN_METHOD)
}

/// Each value of a landed state below the normal range written as zero
/// with its sign ([`crate::normal`]): the state carried to the next step
/// and published holds no subnormal number.
fn normal_or_zero_state(state: &mut MomentumJwstDihedralState) {
    state.t_s = normal_or_zero(state.t_s);
    normal_or_zero_each(&mut state.h_eci_nms);
    state.h_magnitude_nms = normal_or_zero(state.h_magnitude_nms);
    normal_or_zero_each(&mut state.h_anchor_eci_nms);
    normal_or_zero_each(&mut state.impulse_eci_nms);
    state.momentum_defect_nms = normal_or_zero(state.momentum_defect_nms);
    normal_or_zero_each(&mut state.torque_eci_nm);
    normal_or_zero_each(&mut state.torque_body_nm);
    state.h_growth_nm = normal_or_zero(state.h_growth_nm);
    state.force_n = normal_or_zero(state.force_n);
    state.sun_pitch_deg = normal_or_zero(state.sun_pitch_deg);
    state.sun_roll_deg = normal_or_zero(state.sun_roll_deg);
    state.incidence_cosine = normal_or_zero(state.incidence_cosine);
    state.slope_plus_j2_incidence_cosine = normal_or_zero(state.slope_plus_j2_incidence_cosine);
    state.slope_minus_j2_incidence_cosine = normal_or_zero(state.slope_minus_j2_incidence_cosine);
    state.irradiance_w_per_m2 = normal_or_zero(state.irradiance_w_per_m2);
    state.slope_plus_j2_temperature_k = normal_or_zero(state.slope_plus_j2_temperature_k);
    state.slope_minus_j2_temperature_k = normal_or_zero(state.slope_minus_j2_temperature_k);
    state.absorbed_w = normal_or_zero(state.absorbed_w);
    state.emitted_front_w = normal_or_zero(state.emitted_front_w);
    state.emitted_back_w = normal_or_zero(state.emitted_back_w);
    state.energy_defect_w = normal_or_zero(state.energy_defect_w);
    state.quaternion_norm_defect = normal_or_zero(state.quaternion_norm_defect);
    state.residual_conservation_error = normal_or_zero(state.residual_conservation_error);
}

fn step_with(
    card: &MomentumJwstDihedral,
    state: &mut MomentumJwstDihedralState,
    frame: &mut LastStateFrame64,
    load: MomentumJwstDihedralLoad,
    dt_s: f64,
    method: Method,
) -> Result<(), StepReject> {
    validate(card, state, &load, dt_s)?;
    let sun = sunlight(card, &load, method)?;
    commit(card, state, frame, &load, dt_s, &sun, method)
}

fn load_is_finite(load: &MomentumJwstDihedralLoad) -> bool {
    load.q1.is_finite()
        && load.q2.is_finite()
        && load.q3.is_finite()
        && load.q4.is_finite()
        && load.sun_x.is_finite()
        && load.sun_y.is_finite()
        && load.sun_z.is_finite()
        && load.sun_distance_au.is_finite()
}

fn validate(card: &MomentumJwstDihedral, state: &MomentumJwstDihedralState, load: &MomentumJwstDihedralLoad, dt_s: f64) -> Result<(), StepReject> {
    if !dt_s.is_finite() || !load_is_finite(load) {
        return Err(StepReject::NonFiniteInput);
    }
    if !finite(state) {
        return Err(StepReject::NonFiniteState);
    }
    if dt_s <= 0.0 {
        return Err(StepReject::NonPositiveDt);
    }
    if !card.is_valid() {
        return Err(StepReject::InvalidObservatory);
    }
    let q_norm = (load.q1 * load.q1 + load.q2 * load.q2 + load.q3 * load.q3 + load.q4 * load.q4).sqrt();
    if !is_unit(card, q_norm) {
        return Err(StepReject::QuaternionNotUnit);
    }
    if !is_unit(card, norm([load.sun_x, load.sun_y, load.sun_z])) {
        return Err(StepReject::SunDirectionNotUnit);
    }
    if !(load.sun_distance_au > 0.0) {
        return Err(StepReject::SunDistanceNotPositive);
    }
    if !(load.sun_distance_au > card.sun_radius_au) {
        return Err(StepReject::SunDistanceInsideSun);
    }
    Ok(())
}

/// A norm from `1 - 1e-6` to `1 + 1e-6`, both inclusive. In 64-bit
/// arithmetic each edge is the number nearest its decimal, 0.999999 and
/// 1.000001, and the norm of a vector of one component is that component's
/// size exactly, so a component written at either decimal edge lands.
fn is_unit(card: &MomentumJwstDihedral, norm: f64) -> bool {
    norm >= 1.0 - card.unit_norm_tolerance && norm <= 1.0 + card.unit_norm_tolerance
}

/// One slope's balance, force and torque about the centre of mass.
fn slope(
    card: &MomentumJwstDihedral,
    sun_j: [f64; 3],
    irradiance: f64,
    normal: [f64; 3],
    lever: [f64; 3],
    method: Method,
) -> Result<Slope, StepReject> {
    let incidence = dot(normal, sun_j);
    if !(incidence > 0.0) {
        return Err(StepReject::SunBehindSunshield);
    }
    let area = card.area_m2 / 2.0;
    let absorbed_flux = card.absorptance * irradiance * incidence;
    let emittance = if method.no_back_emission { card.front_emittance } else { card.front_emittance + card.back_emittance };
    let t4 = absorbed_flux / (card.stefan_boltzmann_w_per_m2_k4 * emittance);
    let temperature_k = t4.sqrt().sqrt();
    if !temperature_k.is_finite() {
        return Err(StepReject::NonFiniteState);
    }
    if temperature_k > card.kapton_limit_k {
        return Err(StepReject::SunshieldAboveKaptonLimit);
    }
    let t4_again = temperature_k * temperature_k * temperature_k * temperature_k;
    let radiated_per_emittance = card.stefan_boltzmann_w_per_m2_k4 * area * t4_again;
    let absorbed_w = absorbed_flux * area;
    let emitted_front_w = card.front_emittance * radiated_per_emittance;
    let emitted_back_w = card.back_emittance * radiated_per_emittance;
    let photon = irradiance * area * incidence / card.speed_of_light_m_per_s;
    let along_normal = photon * 2.0 * (1.0 - card.absorptance) * incidence
        + (2.0 / 3.0) * emitted_front_w / card.speed_of_light_m_per_s;
    let along_sun = photon * card.absorptance;
    let force_j_n = [
        -along_sun * sun_j[0] - along_normal * normal[0],
        -along_sun * sun_j[1] - along_normal * normal[1],
        -along_sun * sun_j[2] - along_normal * normal[2],
    ];
    Ok(Slope {
        incidence,
        temperature_k,
        absorbed_w,
        emitted_front_w,
        emitted_back_w,
        force_j_n,
        torque_j_nm: cross(lever, force_j_n),
    })
}

/// The Sun in the J-frame, each slope's balance, and the force and torque of
/// the sunlight on the two.
fn sunlight(card: &MomentumJwstDihedral, load: &MomentumJwstDihedralLoad, method: Method) -> Result<Sunlight, StepReject> {
    let m = eci_to_j(load);
    let sun_j = times(&m, [load.sun_x, load.sun_y, load.sun_z]);
    if !finite3(sun_j) {
        return Err(StepReject::NonFiniteState);
    }
    // The layer's mean plane, normal -J3: the incidence and the angles of momentum_jwst_srp.
    let incidence = -sun_j[2];
    if !(incidence > 0.0) {
        return Err(StepReject::SunBehindSunshield);
    }
    let irradiance = card.irradiance_1_au_w_per_m2 / (load.sun_distance_au * load.sun_distance_au);
    let [(normal_plus, centroid_plus), (normal_minus, centroid_minus)] = card.slopes();
    let plus = slope(card, sun_j, irradiance, normal_plus, add(card.ridge_m, centroid_plus), method)?;
    let minus = slope(card, sun_j, irradiance, normal_minus, add(card.ridge_m, centroid_minus), method)?;
    let force_j_n = add(plus.force_j_n, minus.force_j_n);
    let torque_j_nm = add(plus.torque_j_nm, minus.torque_j_nm);
    // Each ratio is at most one in size by its own arithmetic: a square
    // root of a sum of squares is at least the size of each term's root,
    // which is the term's size exactly, so nothing is pulled back.
    let pitch_deg = degrees(crate::math::asin(sun_j[0] / norm(sun_j)));
    let roll_deg = degrees(crate::math::asin(roll_sine(sun_j[1], sun_j[2])));
    let q_norm = (load.q1 * load.q1 + load.q2 * load.q2 + load.q3 * load.q3 + load.q4 * load.q4).sqrt();
    let sun = Sunlight {
        incidence,
        irradiance,
        slopes: [plus, minus],
        force_j_n,
        torque_j_nm,
        pitch_deg,
        roll_deg,
        quaternion_norm_defect: (q_norm - 1.0).abs(),
    };
    if !(finite3(sun.force_j_n) && finite3(sun.torque_j_nm) && sun.pitch_deg.is_finite() && sun.roll_deg.is_finite()) {
        return Err(StepReject::NonFiniteState);
    }
    Ok(sun)
}

fn commit(
    card: &MomentumJwstDihedral,
    state: &mut MomentumJwstDihedralState,
    frame: &mut LastStateFrame64,
    load: &MomentumJwstDihedralLoad,
    dt_s: f64,
    sun: &Sunlight,
    method: Method,
) -> Result<(), StepReject> {
    // Two routes from the body to ECI: the product moves H, the matrix books J.
    let by_product = if method.inverse_rotation {
        let conjugate = MomentumJwstDihedralLoad { q1: -load.q1, q2: -load.q2, q3: -load.q3, ..*load };
        j_to_eci_by_product(&conjugate, sun.torque_j_nm)
    } else {
        j_to_eci_by_product(load, sun.torque_j_nm)
    };
    let booked = transpose_times(&eci_to_j(load), sun.torque_j_nm);
    let [plus, minus] = sun.slopes;
    let mut next = *state;
    next.t_s += dt_s;
    next.h_eci_nms = [
        state.h_eci_nms[0] + by_product[0] * dt_s,
        state.h_eci_nms[1] + by_product[1] * dt_s,
        state.h_eci_nms[2] + by_product[2] * dt_s,
    ];
    next.impulse_eci_nms = [
        state.impulse_eci_nms[0] + booked[0] * dt_s,
        state.impulse_eci_nms[1] + booked[1] * dt_s,
        state.impulse_eci_nms[2] + booked[2] * dt_s,
    ];
    next.h_magnitude_nms = norm(next.h_eci_nms);
    next.torque_eci_nm = booked;
    next.torque_body_nm = sun.torque_j_nm;
    next.h_growth_nm = if next.h_magnitude_nms > 0.0 {
        dot(next.h_eci_nms, booked) / next.h_magnitude_nms
    } else {
        norm(booked)
    };
    next.force_n = norm(sun.force_j_n);
    next.sun_pitch_deg = sun.pitch_deg;
    next.sun_roll_deg = sun.roll_deg;
    next.incidence_cosine = sun.incidence;
    next.slope_plus_j2_incidence_cosine = plus.incidence;
    next.slope_minus_j2_incidence_cosine = minus.incidence;
    next.irradiance_w_per_m2 = sun.irradiance;
    next.slope_plus_j2_temperature_k = plus.temperature_k;
    next.slope_minus_j2_temperature_k = minus.temperature_k;
    next.absorbed_w = plus.absorbed_w + minus.absorbed_w;
    next.emitted_front_w = plus.emitted_front_w + minus.emitted_front_w;
    next.emitted_back_w = plus.emitted_back_w + minus.emitted_back_w;
    next.energy_defect_w = next.emitted_front_w + next.emitted_back_w - next.absorbed_w;
    next.quaternion_norm_defect = sun.quaternion_norm_defect;
    next.outside_field_of_regard = !(sun.pitch_deg >= card.pitch_min_deg
        && sun.pitch_deg <= card.pitch_max_deg
        && sun.roll_deg.abs() <= card.roll_max_deg);
    let defect = [
        next.h_eci_nms[0] - next.h_anchor_eci_nms[0] - next.impulse_eci_nms[0],
        next.h_eci_nms[1] - next.h_anchor_eci_nms[1] - next.impulse_eci_nms[1],
        next.h_eci_nms[2] - next.h_anchor_eci_nms[2] - next.impulse_eci_nms[2],
    ];
    next.momentum_defect_nms = norm(defect);
    // What is in play in the account: the impulse booked. Not |H|, in whose
    // rounding an impulse that never reached H would hide.
    let scale = norm(next.impulse_eci_nms) + card.momentum_scale_floor_nms;
    let momentum_residual = next.momentum_defect_nms / scale;
    // Each slope's balance on its own: a defect on one cannot hide in the
    // other's.
    let defect_plus = plus.emitted_front_w + plus.emitted_back_w - plus.absorbed_w;
    let defect_minus = minus.emitted_front_w + minus.emitted_back_w - minus.absorbed_w;
    let share_plus = defect_plus / plus.absorbed_w;
    let share_minus = defect_minus / minus.absorbed_w;
    let worse = if share_plus.abs() >= share_minus.abs() { share_plus } else { share_minus };
    let energy_residual = worse.abs();
    next.residual_conservation_error = if momentum_residual > energy_residual { momentum_residual } else { energy_residual };
    if !finite(&next) || !worse.is_finite() {
        return Err(StepReject::NonFiniteState);
    }
    if !(momentum_residual <= card.residual_conservation_limit) {
        return Err(StepReject::AngularMomentumBreach);
    }
    if worse > card.residual_conservation_limit {
        return Err(StepReject::SpontaneousGeneration);
    }
    if worse < -card.residual_conservation_limit {
        return Err(StepReject::FirstLawBreach);
    }
    *state = next;
    publish(state, frame);
    Ok(())
}

fn finite(state: &MomentumJwstDihedralState) -> bool {
    state.t_s.is_finite()
        && finite3(state.h_eci_nms)
        && state.h_magnitude_nms.is_finite()
        && finite3(state.h_anchor_eci_nms)
        && finite3(state.impulse_eci_nms)
        && state.momentum_defect_nms.is_finite()
        && finite3(state.torque_eci_nm)
        && finite3(state.torque_body_nm)
        && state.h_growth_nm.is_finite()
        && state.force_n.is_finite()
        && state.sun_pitch_deg.is_finite()
        && state.sun_roll_deg.is_finite()
        && state.incidence_cosine.is_finite()
        && state.slope_plus_j2_incidence_cosine.is_finite()
        && state.slope_minus_j2_incidence_cosine.is_finite()
        && state.irradiance_w_per_m2.is_finite()
        && state.slope_plus_j2_temperature_k.is_finite()
        && state.slope_minus_j2_temperature_k.is_finite()
        && state.absorbed_w.is_finite()
        && state.emitted_front_w.is_finite()
        && state.emitted_back_w.is_finite()
        && state.energy_defect_w.is_finite()
        && state.quaternion_norm_defect.is_finite()
        && state.residual_conservation_error.is_finite()
}

fn publish(state: &MomentumJwstDihedralState, frame: &mut LastStateFrame64) {
    frame.t = state.t_s;
    frame.pos[0] = state.h_eci_nms[0] as f32;
    frame.pos[1] = state.h_eci_nms[1] as f32;
    frame.pos[2] = state.h_eci_nms[2] as f32;
    frame.vel[0] = state.torque_eci_nm[0] as f32;
    frame.vel[1] = state.torque_eci_nm[1] as f32;
    frame.vel[2] = state.torque_eci_nm[2] as f32;
    frame.force_torque = state.h_magnitude_nms as f32;
    frame.residual = state.residual_conservation_error as f32;
    frame.flags = if state.outside_field_of_regard { FLAG_OUTSIDE_FIELD_OF_REGARD } else { 0 };
}

/// Every limit a flag of this card can mark: what [`exceedances`] reports
/// beside a flag's bit (`crate::exceedance`).
pub const EXCEEDANCE_KINDS: &[crate::exceedance::ExceedanceKind] = &[
    PITCH_LOW,
    PITCH_HIGH,
    ROLL_HIGH,
];

const PITCH_WORDS: &str = "the science field of regard in pitch of the Sun, -45 to +5 degrees";
const PITCH_LOW: ExceedanceKind = ExceedanceKind::fixed(
    "FLAG_OUTSIDE_FIELD_OF_REGARD",
    FLAG_OUTSIDE_FIELD_OF_REGARD,
    "sun_pitch_deg",
    "deg",
    Side::Below,
    FIELD_OF_REGARD_PITCH_MIN_DEG,
    PITCH_WORDS,
);
const PITCH_HIGH: ExceedanceKind = ExceedanceKind::fixed(
    "FLAG_OUTSIDE_FIELD_OF_REGARD",
    FLAG_OUTSIDE_FIELD_OF_REGARD,
    "sun_pitch_deg",
    "deg",
    Side::Above,
    FIELD_OF_REGARD_PITCH_MAX_DEG,
    PITCH_WORDS,
);
const ROLL_HIGH: ExceedanceKind = ExceedanceKind::fixed(
    "FLAG_OUTSIDE_FIELD_OF_REGARD",
    FLAG_OUTSIDE_FIELD_OF_REGARD,
    "sun_roll_deg_magnitude",
    "deg",
    Side::Above,
    FIELD_OF_REGARD_ROLL_MAX_DEG,
    "the science field of regard in roll of the Sun, -5 to +5 degrees",
);

/// The limits the step from `before` to `after` under `load` over `dt_s`
/// is past: one record for each limit a flag set on its frame marks.
pub fn exceedances(
    card: &MomentumJwstDihedral,
    before: &MomentumJwstDihedralState,
    after: &MomentumJwstDihedralState,
    load: MomentumJwstDihedralLoad,
    dt_s: f64,
) -> crate::exceedance::Exceedances {
    let _ = (before, load, dt_s);
    let mut records = crate::exceedance::Exceedances::new();
    if after.outside_field_of_regard {
        let pitch = after.sun_pitch_deg;
        let roll = after.sun_roll_deg.abs();
        records.push_if(!(pitch >= card.pitch_min_deg), Exceedance::at(PITCH_LOW, card.pitch_min_deg, pitch));
        records.push_if(!(pitch <= card.pitch_max_deg), Exceedance::at(PITCH_HIGH, card.pitch_max_deg, pitch));
        records.push_if(!(roll <= card.roll_max_deg), Exceedance::at(ROLL_HIGH, card.roll_max_deg, roll));
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

    use super::*;

    fn blank() -> LastStateFrame64 {
        LastStateFrame64 { t: 0.0, pos: [0.0; 3], vel: [0.0; 3], force_torque: 0.0, residual: 0.0, flags: 0, proof: [7; 16] }
    }

    /// The attitude of 2023-05-13 00:00:30 UTC, as telemetered, and the Sun
    /// from JWST then, from the JPL Horizons vectors of the record: the
    /// worked load of `momentum_jwst_srp`.
    fn may13() -> MomentumJwstDihedralLoad {
        MomentumJwstDihedralLoad {
            q1: 0.7216001749,
            q2: 0.0366541296,
            q3: -0.5597470999,
            q4: 0.4057497233,
            sun_x: 0.6160922205743601,
            sun_y: 0.7226624561625701,
            sun_z: 0.31335179942172875,
            sun_distance_au: 1.0193963859269388,
        }
    }

    fn unit_sun(load: MomentumJwstDihedralLoad) -> MomentumJwstDihedralLoad {
        let n = norm([load.sun_x, load.sun_y, load.sun_z]);
        MomentumJwstDihedralLoad { sun_x: load.sun_x / n, sun_y: load.sun_y / n, sun_z: load.sun_z / n, ..load }
    }

    fn start() -> MomentumJwstDihedralState {
        MomentumJwstDihedralState::try_start(&MomentumJwstDihedral::observatory(), [-1.4853731, -0.6656560, 1.1653501]).unwrap()
    }

    /// A Sun at a pitch and a roll, in degrees, with the identity attitude
    /// (J-frame on ECI).
    fn sun_at(pitch_deg: f64, roll_deg: f64, distance_au: f64) -> MomentumJwstDihedralLoad {
        let (p, r) = (pitch_deg.to_radians(), roll_deg.to_radians());
        let (sp, cp, sr, cr) = (crate::math::sin(p), crate::math::cos(p), crate::math::sin(r), crate::math::cos(r));
        let v = [sp, cp * sr, -cp * cr];
        let n = norm(v);
        MomentumJwstDihedralLoad { q1: 0.0, q2: 0.0, q3: 0.0, q4: 1.0, sun_x: v[0] / n, sun_y: v[1] / n, sun_z: v[2] / n, sun_distance_au: distance_au }
    }

    #[test]
    fn the_observatory_holds_its_constants_and_a_moved_one_is_refused() {
        let card = MomentumJwstDihedral::observatory();
        assert!(card.is_valid());
        for moved in [
            MomentumJwstDihedral { absorptance: f64::from_bits(LAYER1_SOLAR_ABSORPTANCE.to_bits() + 1), ..card },
            MomentumJwstDihedral { dihedral_rad: f64::from_bits(DIHEDRAL_RAD.to_bits() + 1), ..card },
            MomentumJwstDihedral { slope_centroid_m: f64::from_bits(SLOPE_CENTROID_FROM_RIDGE_M.to_bits() - 1), ..card },
            MomentumJwstDihedral { ridge_m: [RIDGE_J1_M, RIDGE_J2_M, f64::from_bits(RIDGE_J3_M.to_bits() + 1)], ..card },
        ] {
            assert!(!moved.is_valid());
            let mut state = start();
            let before = state;
            let mut frame = blank();
            assert_eq!(step(&moved, &mut state, &mut frame, unit_sun(may13()), 60.0), Err(StepReject::InvalidObservatory));
            assert_eq!(state, before);
        }
        assert!((SLOPE_CENTROID_FROM_RIDGE_M - 14.6 / 6.0).abs() < 1.0e-15);
    }

    #[test]
    fn the_ledgers_close_at_every_step_from_the_floor_to_the_ceiling() {
        let card = MomentumJwstDihedral::observatory();
        for dt_s in [card.floor_dt_s().unwrap(), 60.0, card.gate_dt_s().unwrap()] {
            let mut state = start();
            let mut frame = blank();
            for _ in 0..2000 {
                step(&card, &mut state, &mut frame, unit_sun(may13()), dt_s).unwrap();
                assert!(state.residual_conservation_error <= RESIDUAL_CONSERVATION_LIMIT, "{}", state.residual_conservation_error);
                assert_eq!(frame.proof, [7; 16]);
            }
        }
    }

    /// The worked load's torque, against the record's numpy arithmetic of
    /// the same slopes (`analysis/dihedral.py`): J-frame 1.2109742264315715e-05,
    /// 1.6616639408766323e-05, -1.060380072919094e-06 N m.
    #[test]
    fn the_worked_torque_is_the_records_arithmetic() {
        let card = MomentumJwstDihedral::observatory();
        let mut state = start();
        let mut frame = blank();
        step(&card, &mut state, &mut frame, unit_sun(may13()), 60.0).unwrap();
        let record = [1.2109742264315715e-05, 1.6616639408766323e-05, -1.060380072919094e-06];
        for k in 0..3 {
            assert!((state.torque_body_nm[k] - record[k]).abs() <= 1.0e-12 * norm(record), "{k}: {} {}", state.torque_body_nm[k], record[k]);
        }
        assert!((state.slope_plus_j2_temperature_k - 342.42273).abs() < 1.0e-4, "{}", state.slope_plus_j2_temperature_k);
        assert!((state.slope_minus_j2_temperature_k - 342.42659).abs() < 1.0e-4, "{}", state.slope_minus_j2_temperature_k);
    }

    /// With the Sun rolled towards +J2 the slope on the -J2 side is lit more
    /// squarely, and the torque about J1 falls with roll about four times as
    /// fast as on one flat layer: from +53.2 to -28.0 µN m between -4 and +4
    /// degrees at a pitch of -20 degrees and 1 au, where `momentum_jwst_srp`
    /// goes from +24.2 to +3.7.
    #[test]
    fn roll_turns_into_torque_about_the_boresight() {
        let card = MomentumJwstDihedral::observatory();
        let mut frame = blank();
        let mut torque = |roll: f64| {
            let mut state = start();
            step(&card, &mut state, &mut frame, sun_at(-20.0, roll, 1.0), 60.0).unwrap();
            state
        };
        let (minus, level, plus) = (torque(-4.0), torque(0.0), torque(4.0));
        assert!(plus.slope_minus_j2_incidence_cosine > plus.slope_plus_j2_incidence_cosine);
        assert!(minus.slope_plus_j2_incidence_cosine > minus.slope_minus_j2_incidence_cosine);
        assert!((level.slope_plus_j2_incidence_cosine - level.slope_minus_j2_incidence_cosine).abs() < 1.0e-15);
        assert!((minus.torque_body_nm[0] * 1.0e6 - 53.17).abs() < 0.01, "{}", minus.torque_body_nm[0]);
        assert!((level.torque_body_nm[0] * 1.0e6 - 12.62).abs() < 0.01, "{}", level.torque_body_nm[0]);
        assert!((plus.torque_body_nm[0] * 1.0e6 + 28.04).abs() < 0.01, "{}", plus.torque_body_nm[0]);
    }

    #[test]
    fn a_quaternion_that_is_not_unit_is_refused_with_the_state_whole() {
        let card = MomentumJwstDihedral::observatory();
        let mut state = start();
        let before = state;
        let mut frame = blank();
        let load = unit_sun(may13());
        let stretched = MomentumJwstDihedralLoad { q1: load.q1 * 1.00001, q2: load.q2 * 1.00001, q3: load.q3 * 1.00001, q4: load.q4 * 1.00001, ..load };
        assert_eq!(step(&card, &mut state, &mut frame, stretched, 60.0), Err(StepReject::QuaternionNotUnit));
        let zero = MomentumJwstDihedralLoad { q1: 0.0, q2: 0.0, q3: 0.0, q4: 0.0, ..load };
        assert_eq!(step(&card, &mut state, &mut frame, zero, 60.0), Err(StepReject::QuaternionNotUnit));
        assert_eq!(state, before);
        assert_eq!(frame.proof, [7; 16]);
        let long_sun = MomentumJwstDihedralLoad { sun_x: load.sun_x * 1.01, ..load };
        assert_eq!(step(&card, &mut state, &mut frame, long_sun, 60.0), Err(StepReject::SunDirectionNotUnit));
        let nowhere = MomentumJwstDihedralLoad { sun_distance_au: 0.0, ..load };
        assert_eq!(step(&card, &mut state, &mut frame, nowhere, 60.0), Err(StepReject::SunDistanceNotPositive));
        assert_eq!(state, before);
    }

    /// The Sun behind the mean plane, edge-on to it, or behind the plane of
    /// one slope while the other is lit, is refused. A roll of 85 degrees
    /// still lights both slopes; at 87 the slope on the +J2 side is turned
    /// away from the Sun.
    #[test]
    fn the_sun_behind_a_slope_is_refused() {
        let card = MomentumJwstDihedral::observatory();
        let mut state = start();
        let before = state;
        let mut frame = blank();
        // The identity attitude puts J3 on ECI z; a Sun on +z is behind.
        let behind = MomentumJwstDihedralLoad { q1: 0.0, q2: 0.0, q3: 0.0, q4: 1.0, sun_x: 0.0, sun_y: 0.0, sun_z: 1.0, sun_distance_au: 1.0 };
        assert_eq!(step(&card, &mut state, &mut frame, behind, 60.0), Err(StepReject::SunBehindSunshield));
        let edge_on = MomentumJwstDihedralLoad { sun_x: 1.0, sun_z: 0.0, ..behind };
        assert_eq!(step(&card, &mut state, &mut frame, edge_on, 60.0), Err(StepReject::SunBehindSunshield));
        assert_eq!(step(&card, &mut state, &mut frame, sun_at(0.0, 87.0, 1.0), 60.0), Err(StepReject::SunBehindSunshield));
        assert_eq!(step(&card, &mut state, &mut frame, sun_at(0.0, -87.0, 1.0), 60.0), Err(StepReject::SunBehindSunshield));
        assert_eq!(state, before);
        step(&card, &mut state, &mut frame, sun_at(0.0, 85.0, 1.0), 60.0).unwrap();
        assert!(state.outside_field_of_regard);
        assert!(state.slope_plus_j2_incidence_cosine > 0.0 && state.slope_plus_j2_incidence_cosine < 0.02, "{}", state.slope_plus_j2_incidence_cosine);
    }

    #[test]
    fn the_kapton_limit_is_met_near_a_quarter_of_an_au() {
        let card = MomentumJwstDihedral::observatory();
        let face_on = MomentumJwstDihedralLoad { q1: 0.0, q2: 0.0, q3: 0.0, q4: 1.0, sun_x: 0.0, sun_y: 0.0, sun_z: -1.0, sun_distance_au: 1.0 };
        let mut state = start();
        let mut frame = blank();
        step(&card, &mut state, &mut frame, MomentumJwstDihedralLoad { sun_distance_au: 0.28, ..face_on }, 60.0).unwrap();
        assert!(state.slope_plus_j2_temperature_k < KAPTON_LIMIT_K);
        let before = state;
        assert_eq!(
            step(&card, &mut state, &mut frame, MomentumJwstDihedralLoad { sun_distance_au: 0.26, ..face_on }, 60.0),
            Err(StepReject::SunshieldAboveKaptonLimit)
        );
        assert_eq!(state, before);
        // At L2 with the Sun on the mean normal: 347 K on each slope.
        step(&card, &mut state, &mut frame, MomentumJwstDihedralLoad { sun_distance_au: 1.0194, ..face_on }, 60.0).unwrap();
        assert!((state.slope_plus_j2_temperature_k - 347.3).abs() < 0.5, "{}", state.slope_plus_j2_temperature_k);
        assert_eq!(state.slope_plus_j2_temperature_k.to_bits(), state.slope_minus_j2_temperature_k.to_bits());
    }

    #[test]
    fn the_field_of_regard_is_flagged_and_the_step_lands() {
        let card = MomentumJwstDihedral::observatory();
        let mut state = start();
        let mut frame = blank();
        step(&card, &mut state, &mut frame, sun_at(-50.0, 0.0, 1.0), 60.0).unwrap();
        assert!(state.outside_field_of_regard);
        assert_eq!(frame.flags, FLAG_OUTSIDE_FIELD_OF_REGARD);
        assert!((state.sun_pitch_deg + 50.0).abs() < 1e-9);
        step(&card, &mut state, &mut frame, sun_at(-20.0, 0.0, 1.0), 60.0).unwrap();
        assert!(!state.outside_field_of_regard);
        assert_eq!(frame.flags, 0);
        step(&card, &mut state, &mut frame, sun_at(-20.0, 6.0, 1.0), 60.0).unwrap();
        assert!(state.outside_field_of_regard);
    }

    /// The force at a face-on Sun, worked by hand: each slope is met at the
    /// incidence cos(delta), and the two pushes along J2 cancel.
    #[test]
    fn the_force_at_a_face_on_sun_is_the_photon_pressure_on_the_slopes() {
        let card = MomentumJwstDihedral::observatory();
        let face_on = MomentumJwstDihedralLoad { q1: 0.0, q2: 0.0, q3: 0.0, q4: 1.0, sun_x: 0.0, sun_y: 0.0, sun_z: -1.0, sun_distance_au: 1.0 };
        let mut state = start();
        let mut frame = blank();
        step(&card, &mut state, &mut frame, face_on, 60.0).unwrap();
        let (s, c) = (crate::math::sin(DIHEDRAL_RAD), crate::math::cos(DIHEDRAL_RAD));
        let half = SUNSHIELD_AREA_M2 / 2.0;
        let photon = SOLAR_IRRADIANCE_AT_1_AU_W_PER_M2 * half * c / SPEED_OF_LIGHT_M_PER_S;
        let thermal = (2.0 / 3.0) * LAYER1_SOLAR_ABSORPTANCE * SOLAR_IRRADIANCE_AT_1_AU_W_PER_M2 * half * c * LAYER1_FRONT_EMITTANCE
            / (LAYER1_FRONT_EMITTANCE + LAYER1_BACK_EMITTANCE)
            / SPEED_OF_LIGHT_M_PER_S;
        let along_normal = photon * 2.0 * (1.0 - LAYER1_SOLAR_ABSORPTANCE) * c + thermal;
        let along_j3 = 2.0 * (photon * LAYER1_SOLAR_ABSORPTANCE + along_normal * c);
        assert!((state.force_n - along_j3).abs() < 1e-15 * along_j3 * 10.0, "{} {}", state.force_n, along_j3);
        let w = SLOPE_CENTROID_FROM_RIDGE_M;
        let drop = -w * s / c;
        let f_plus = [0.0, along_normal * s, photon * LAYER1_SOLAR_ABSORPTANCE + along_normal * c];
        let f_minus = [0.0, -along_normal * s, photon * LAYER1_SOLAR_ABSORPTANCE + along_normal * c];
        let ridge = [RIDGE_J1_M, RIDGE_J2_M, RIDGE_J3_M];
        let torque = add(cross(add(ridge, [0.0, w, drop]), f_plus), cross(add(ridge, [0.0, -w, drop]), f_minus));
        for k in 0..3 {
            assert!((state.torque_body_nm[k] - torque[k]).abs() < 1e-18, "{k}");
            assert!((state.torque_eci_nm[k] - torque[k]).abs() < 1e-18, "{k}");
        }
    }

    #[test]
    fn nothing_is_filled_in() {
        let card = MomentumJwstDihedral::observatory();
        let mut state = start();
        let before = state;
        let mut frame = blank();
        let load = unit_sun(may13());
        for bad in [
            MomentumJwstDihedralLoad { q1: f64::NAN, ..load },
            MomentumJwstDihedralLoad { sun_z: f64::INFINITY, ..load },
            MomentumJwstDihedralLoad { sun_distance_au: f64::NEG_INFINITY, ..load },
        ] {
            assert_eq!(step(&card, &mut state, &mut frame, bad, 60.0), Err(StepReject::NonFiniteInput));
            assert_eq!(state, before);
        }
        assert_eq!(step(&card, &mut state, &mut frame, load, 0.0), Err(StepReject::NonPositiveDt));
        assert_eq!(state, before);
        assert!(MomentumJwstDihedralState::try_start(&card, [f64::NAN, 0.0, 0.0]).is_none());
    }

    /// Rule 10: H moved by the torque turned the wrong way is named by the
    /// momentum ledger.
    #[test]
    fn a_torque_turned_the_wrong_way_is_named_by_the_momentum_ledger() {
        let card = MomentumJwstDihedral::observatory();
        let mut state = start();
        let before = state;
        let mut frame = blank();
        let wrong = Method { inverse_rotation: true, no_back_emission: false };
        let named = step_with(&card, &mut state, &mut frame, unit_sun(may13()), 60.0, wrong);
        assert_eq!(named, Err(StepReject::AngularMomentumBreach));
        assert_eq!(state, before);
    }

    /// Rule 10: a balance that leaves the back's emission out is named by the
    /// energy ledger.
    #[test]
    fn a_slope_balance_without_its_back_is_named_by_the_energy_ledger() {
        let card = MomentumJwstDihedral::observatory();
        let mut state = start();
        let before = state;
        let mut frame = blank();
        let wrong = Method { inverse_rotation: false, no_back_emission: true };
        assert_eq!(step_with(&card, &mut state, &mut frame, unit_sun(may13()), 60.0, wrong), Err(StepReject::SpontaneousGeneration));
        assert_eq!(state, before);
    }

    /// Rule 10: a falsified ledger is named.
    #[test]
    fn a_falsified_ledger_is_named() {
        let card = MomentumJwstDihedral::observatory();
        let mut state = start();
        state.h_anchor_eci_nms[1] += 1.0e-6;
        let mut frame = blank();
        assert_eq!(step(&card, &mut state, &mut frame, unit_sun(may13()), 60.0), Err(StepReject::AngularMomentumBreach));
        let mut state = start();
        state.h_eci_nms[0] -= 1.0e-6;
        assert_eq!(step(&card, &mut state, &mut frame, unit_sun(may13()), 60.0), Err(StepReject::AngularMomentumBreach));
    }

    /// The Sun's distance at or under its radius is inside the Sun: refused
    /// by name with the state whole, at a grazing Sun where the slopes stay
    /// under the Kapton limit, down to 1e-9 au. Just above the radius the step
    /// lands.
    #[test]
    fn a_sun_distance_inside_the_sun_is_refused() {
        let card = MomentumJwstDihedral::observatory();
        assert!((SUN_RADIUS_AU - 0.004650467260962157).abs() < 1.0e-18, "{SUN_RADIUS_AU}");
        let grazing = |distance: f64| {
            let z: f64 = -1.0e-6;
            MomentumJwstDihedralLoad { q1: 0.0, q2: 0.0, q3: 0.0, q4: 1.0, sun_x: (1.0 - z * z).sqrt(), sun_y: 0.0, sun_z: z, sun_distance_au: distance }
        };
        let mut state = start();
        let before = state;
        let mut frame = blank();
        for distance in [SUN_RADIUS_AU, 0.004, 0.001, 1.0e-6, 1.0e-9, f64::MIN_POSITIVE] {
            assert_eq!(step(&card, &mut state, &mut frame, grazing(distance), 60.0), Err(StepReject::SunDistanceInsideSun), "{distance}");
            assert_eq!(state, before);
            assert_eq!(frame.to_bytes(), blank().to_bytes());
        }
        assert_eq!(step(&card, &mut state, &mut frame, grazing(0.0), 60.0), Err(StepReject::SunDistanceNotPositive));
        let above = f64::from_bits(SUN_RADIUS_AU.to_bits() + 1);
        step(&card, &mut state, &mut frame, grazing(above), 60.0).unwrap();
        assert!(state.slope_plus_j2_temperature_k < KAPTON_LIMIT_K);
        assert!(state.residual_conservation_error <= RESIDUAL_CONSERVATION_LIMIT);
    }

    /// The momentum account is held to what is in play in it, the impulse
    /// booked, and not to |H|: an impulse that never reached H, as at a start
    /// of 1e17 N m s, is named on the first step. At 74 N m s, the most in the
    /// record, two million steps of 0.256 s, the telemetry's attitude sample,
    /// close; at the floor, 0.0479 s, the same impulse each step leaves the
    /// same rounding in H each step, and the account is named on the
    /// 271,866th, with the state as it was.
    #[test]
    fn an_impulse_that_never_reaches_h_is_named_and_the_records_momentum_closes() {
        let card = MomentumJwstDihedral::observatory();
        let load = unit_sun(may13());
        for h0 in [[1.0e17, 0.0, 0.0], [1.0e12, 0.0, 0.0], [0.0, -1.0e9, 0.0]] {
            let mut state = MomentumJwstDihedralState::try_start(&card, h0).unwrap();
            let before = state;
            let mut frame = blank();
            assert_eq!(step(&card, &mut state, &mut frame, load, card.gate_dt_s().unwrap()), Err(StepReject::AngularMomentumBreach), "{h0:?}");
            assert_eq!(state, before);
        }
        let direction = [-1.4853731309945153, -0.6656560404607257, 1.1653501123098389];
        let size = norm(direction);
        let h0 = [74.0 * direction[0] / size, 74.0 * direction[1] / size, 74.0 * direction[2] / size];
        let mut state = MomentumJwstDihedralState::try_start(&card, h0).unwrap();
        let mut frame = blank();
        let mut worst = 0.0_f64;
        for _ in 0..2_000_000 {
            step(&card, &mut state, &mut frame, load, 0.256).unwrap();
            worst = worst.max(state.residual_conservation_error);
        }
        assert!(worst <= RESIDUAL_CONSERVATION_LIMIT, "{worst:e}");
        let in_play = norm(state.impulse_eci_nms) + MOMENTUM_SCALE_FLOOR_NMS;
        let mut moved = state;
        moved.h_eci_nms[2] += 1.0e-8 * in_play;
        assert_eq!(step(&card, &mut moved, &mut frame, load, 0.256), Err(StepReject::AngularMomentumBreach));
        let floor = card.floor_dt_s().unwrap();
        let mut state = MomentumJwstDihedralState::try_start(&card, h0).unwrap();
        for _ in 0..271_865 {
            step(&card, &mut state, &mut frame, load, floor).unwrap();
        }
        let before = state;
        assert_eq!(step(&card, &mut state, &mut frame, load, floor), Err(StepReject::AngularMomentumBreach));
        assert_eq!(state, before);
    }

    /// The norm's edges are 0.999999 and 1.000001, both inclusive, on either
    /// side alike: one component written at either decimal lands, the next
    /// 64-bit number past it is refused. The same for the Sun's direction.
    #[test]
    fn the_unit_norm_holds_its_decimal_edges_on_both_sides() {
        let card = MomentumJwstDihedral::observatory();
        assert_eq!((1.0 - UNIT_NORM_TOLERANCE).to_bits(), 0.999999_f64.to_bits());
        assert_eq!((1.0 + UNIT_NORM_TOLERANCE).to_bits(), 1.000001_f64.to_bits());
        let lit = |q1: f64, q4: f64| {
            // Identity-like attitudes: q4 alone keeps J3 on z, q1 alone turns it to -z.
            let sun_z = if q1 != 0.0 { 1.0 } else { -1.0 };
            MomentumJwstDihedralLoad { q1, q2: 0.0, q3: 0.0, q4, sun_x: 0.0, sun_y: 0.0, sun_z, sun_distance_au: 1.0 }
        };
        let mut frame = blank();
        let mut state = start();
        for (q1, q4) in [(0.0, 0.999999), (0.0, 1.000001), (1.000001, 0.0), (0.999999, 0.0), (-1.000001, 0.0), (0.0, -0.999999)] {
            step(&card, &mut state, &mut frame, lit(q1, q4), 60.0).unwrap_or_else(|reject| std::panic!("{q1} {q4} {reject:?}"));
        }
        let below = f64::from_bits(0.999999_f64.to_bits() - 1);
        let above = f64::from_bits(1.000001_f64.to_bits() + 1);
        for (q1, q4) in [(0.0, below), (0.0, above), (above, 0.0), (below, 0.0)] {
            let before = state;
            assert_eq!(step(&card, &mut state, &mut frame, lit(q1, q4), 60.0), Err(StepReject::QuaternionNotUnit), "{q1} {q4}");
            assert_eq!(state, before);
        }
        let face_on = |sun_z: f64| MomentumJwstDihedralLoad { q1: 0.0, q2: 0.0, q3: 0.0, q4: 1.0, sun_x: 0.0, sun_y: 0.0, sun_z, sun_distance_au: 1.0 };
        for sun_z in [-0.999999, -1.000001] {
            step(&card, &mut state, &mut frame, face_on(sun_z), 60.0).unwrap();
        }
        for sun_z in [-below, -above] {
            assert_eq!(step(&card, &mut state, &mut frame, face_on(sun_z), 60.0), Err(StepReject::SunDirectionNotUnit), "{sun_z}");
        }
    }

    /// The pitch and the roll are each the arcsine of a ratio that is at most
    /// one by its own arithmetic, with nothing pulled back.
    #[test]
    fn the_pitch_and_the_roll_are_the_directions_own_with_nothing_pulled_back() {
        let card = MomentumJwstDihedral::observatory();
        let mut state = start();
        let mut frame = blank();
        let length = 1.0 + 0.9e-6;
        let off = 0.3_f64;
        let sun = [length * (1.0 - off * off).sqrt(), 0.0, -length * off];
        let along = MomentumJwstDihedralLoad { q1: 0.0, q2: 0.0, q3: 0.0, q4: 1.0, sun_x: sun[0], sun_y: sun[1], sun_z: sun[2], sun_distance_au: 1.0 };
        step(&card, &mut state, &mut frame, along, 60.0).unwrap();
        let expected = 90.0 - crate::math::asin(off).to_degrees();
        assert!(state.sun_pitch_deg < 90.0 && (state.sun_pitch_deg - expected).abs() < 1.0e-6, "{}", state.sun_pitch_deg);
        assert!(state.outside_field_of_regard);
    }

    #[test]
    fn the_same_run_gives_the_same_bits() {
        let card = MomentumJwstDihedral::observatory();
        let run = || {
            let mut state = start();
            let mut frame = blank();
            for _ in 0..100 {
                step(&card, &mut state, &mut frame, unit_sun(may13()), 60.0).unwrap();
            }
            (frame.to_bytes(), state)
        };
        assert_eq!(run(), run());
    }

    /// Whether every value of a state is normal or zero: the state is the
    /// one the rule of `crate::normal` leaves as it is.
    fn normal_or_zero_throughout(state: &MomentumJwstDihedralState) -> bool {
        let mut held = *state;
        normal_or_zero_state(&mut held);
        held == *state
    }

    /// The red team's grazing Sun on engine 0.13.0 (D1 and D2): the Sun at
    /// (1, 0, -1e-300), all but edge-on to the first layer, at 1, 100 and
    /// 10,000 au, from a momentum of zero, in steps of 0.05 s. The momentum,
    /// its growth, the torque and the energy's defect the step computes are
    /// under the smallest normal number. Each lands as zero with its sign,
    /// every value of the landed state is normal or zero, and the state that
    /// landed steps on, so a continuation from the sealed state is refused at
    /// no number of the engine's own.
    #[test]
    fn a_grazing_sun_lands_every_value_normal_or_zero_and_the_state_steps_on() {
        let card = MomentumJwstDihedral::observatory();
        let mut computed_subnormal = 0;
        for distance_au in [1.0, 100.0, 1.0e4] {
            let load = MomentumJwstDihedralLoad { q1: 0.0, q2: 0.0, q3: 0.0, q4: 1.0, sun_x: 1.0, sun_y: 0.0, sun_z: -1.0e-300, sun_distance_au: distance_au };
            let start = MomentumJwstDihedralState::try_start(&card, [0.0; 3]).unwrap();
            let mut as_computed = start;
            let mut frame_computed = blank();
            step_as_computed(&card, &mut as_computed, &mut frame_computed, load, 0.05).unwrap();
            if !normal_or_zero_throughout(&as_computed) {
                computed_subnormal += 1;
            }
            let mut state = start;
            let mut frame = blank();
            step(&card, &mut state, &mut frame, load, 0.05).unwrap();
            assert!(normal_or_zero_throughout(&state), "{distance_au} au: {state:?}");
            // The frame's 32-bit slots held zero for those values already.
            assert_eq!(frame.to_bytes(), frame_computed.to_bytes());
            // Each value the rule wrote is the computed one within the
            // smallest normal number, and keeps its sign.
            let mut held = as_computed;
            normal_or_zero_state(&mut held);
            assert_eq!(held, state);
            for (landed, computed) in state.h_eci_nms.iter().zip(as_computed.h_eci_nms) {
                assert!((landed - computed).abs() < f64::MIN_POSITIVE);
                assert_eq!(landed.is_sign_negative(), computed.is_sign_negative());
            }
            // The landed state, sent back as the start of the next step,
            // steps on, and lands normal or zero again.
            for _ in 0..3 {
                step(&card, &mut state, &mut frame, load, 0.05).unwrap();
                assert!(normal_or_zero_throughout(&state), "{distance_au} au: {state:?}");
            }
            let mut again = MomentumJwstDihedralState::try_start(&card, state.h_eci_nms).unwrap();
            step(&card, &mut again, &mut blank(), load, 0.05).unwrap();
            assert!(normal_or_zero_throughout(&again));
        }
        // At 100 and 10,000 au the step computes a momentum under the smallest normal number.
        assert!(computed_subnormal >= 2, "{computed_subnormal}");
    }
}
