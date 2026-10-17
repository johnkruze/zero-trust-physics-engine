#![no_implicit_prelude]

//! The angular momentum of the James Webb Space Telescope at the Sun-Earth L2
//! point, growing under the pressure of sunlight on its sunshield.
//!
//! The observatory holds its attitude with six reaction wheels. Sunlight
//! pushes on the sunshield, and where the push does not pass through the
//! centre of mass it is a torque. The wheels take that torque up by spinning
//! up, so the observatory's total angular momentum (wheels and body) grows
//! between momentum unloads, when thrusters take it out. How fast it grows,
//! and in what direction, depends on the attitude: the Sun's direction on the
//! sunshield sets the size and the direction of the push.
//!
//! The card is a rigid body whose momentum is held by its wheels. Its state is
//! the total angular momentum `H`, in the inertial frame of the flight
//! software (ECI, J2000 equatorial), in N m s. Its load is the attitude, the
//! quaternion of the ACS J-frame against ECI as the observatory's telemetry
//! gives it (`SA_ZATTEST1` to `SA_ZATTEST4` of the JWST engineering database,
//! `q4` the scalar), and the Sun's direction and distance from the
//! observatory, which an ephemeris gives. Over one step the load is held; the
//! sunlight torque is then constant, and the step is exact:
//!
//! ```text
//! H(t + dt) = H(t) + tau_ECI dt
//! ```
//!
//! The sunshield. The Sun lights only its first layer over the field of
//! regard (Arenberg et al. 2016, SPIE 9904, 990405): a 50 um Kapton E
//! membrane with a doped-silicon coating on its Sun side and vapour-deposited
//! aluminium (VDA) on its back. The card takes it as one flat surface:
//!
//! - its normal is the J-frame's `-J3`: over the settled minutes of the
//!   fitted month the Sun lay 0.2 to 44.6 degrees from `-J3`, at a pitch of
//!   -44.6 to +4.6 degrees towards `J1`, inside the field of regard's -45 to
//!   +5, and a roll of -5.5 to +6.1 degrees (`analysis/02_frames.py` of the
//!   record);
//! - its area, 220 m², is the area that takes the "~300 kW of power received
//!   from the Sun" of Arenberg et al. at 1360.8 W/m², to two figures;
//! - its Sun side absorbs 0.47 of the sunlight and emits with an emittance of
//!   0.72, end-of-life values of JWST's layer 1 (Origins Space Telescope
//!   Mission Concept Study Report, 2019, appendix C, arXiv:1912.06213); the
//!   rest, 0.53, is reflected, taken as specular, as from a smooth film;
//! - its back is VDA, emittance 0.025 (the same report's VDA, warm);
//! - the solar irradiance at 1 au is 1360.8 W/m² (Kopp and Lean 2011,
//!   Geophys. Res. Lett. 38, L01706), falling as the inverse square of the
//!   distance.
//!
//! The layer is in radiative balance at every step: what it absorbs it emits
//! from its two faces, `alpha S A cos(theta) = (eps_f + eps_b) sigma A T^4`.
//! The emission from its face carries momentum away; the emission from its
//! back falls on the second layer, inside the observatory, and gives no net
//! push. With `s` the unit vector to the Sun and `n = -J3` in the J-frame:
//!
//! ```text
//! cos(theta) = n . s                          the Sun's incidence
//! S          = 1360.8 / d^2                   W/m², d in au
//! P_abs      = alpha S A cos(theta)           absorbed, W
//! T^4        = alpha S cos(theta) / (sigma (eps_f + eps_b))
//! P_front    = eps_f sigma A T^4,  P_back = eps_b sigma A T^4
//! F_J        = -(S A cos(theta) / c) [ alpha s + 2 (1 - alpha) cos(theta) n ]
//!              - (2/3) (P_front / c) n        N, J-frame
//! tau_J      = r_cp x F_J                     N m
//! ```
//!
//! `r_cp` is where the push acts, from the centre of mass, in the J-frame. It
//! is not published. Its three numbers are fitted, by least squares, to the
//! observatory's own momentum telemetry: the changes of `H` over the 763
//! hourly blocks of the UTC days 2023-05-12 to 2023-06-14, settled minute to
//! settled minute, away from the unloads and the one burn (`analysis/04_fit.py`
//! of the research record at `evidence/research/momentum_jwst_srp/`). They
//! are -0.0410191, 0.00858426 and -0.355723 m: the push acts 0.36 m on the
//! Sun's side of the centre of mass and 4 cm aft of it along the boresight.
//! The area and the optical numbers are not fitted, so the fit is of where
//! the sunshield's push acts and of nothing else.
//!
//! Compared on 2026-10-14 with the held-back month of 2023-06-15 to
//! 2023-07-14, downloaded after these constants were committed, by five
//! marks fixed beforehand (`validation_protocol.md` and `RESULT.md` of the
//! record): four met, one missed. The daily growth of `|H|` was within 0.1
//! of the observed plus 0.2 N m s on 23 of 28 days (line 80 percent); `|H|`
//! at the end of runs of 10.7 and 16.3 days between thruster events was
//! within 8.4 and 0.9 percent (line 10); the hourly growth rate's error was
//! 0.437 of its spread (line 0.5); the torque on one attitude was off by
//! 15.4 against 58.6 µN m rms (line 0.4 of it). The daily change of the
//! vector was within its allowance on 14 of 28 days, and that mark was
//! missed. Where the Sun's roll is beyond 2 degrees the torque about `J1`
//! is off by about 30 µN m: one flat layer turns roll into torque about a
//! quarter as fast as the observatory does. Within 2 degrees of roll the
//! torque's error is 5.8 against 56.7 µN m.
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
//!   `H` does not carry the step's impulse into it to that share (at
//!   10^12 N m s and more on the worked load) is named, and not hidden.
//! - **The quaternion's unit norm.** An attitude is a unit quaternion. The
//!   card refuses one whose norm `|q|` is outside 1 - 1e-6 to 1 + 1e-6, both
//!   inclusive, each edge the 64-bit number nearest its decimal, 0.999999 and
//!   1.000001, so a norm written 1e-6 from one lands on either side
//!   (`QuaternionNotUnit`): the telemetry's components are 32-bit numbers,
//!   whose rounding moves the norm by about 1e-7, and the largest departure
//!   in the fitted month's telemetry was 5.2e-8. It does not normalise one:
//!   the rotation it uses is that of `q / |q|`, in which a norm of 1 - 1e-6
//!   moves no torque by more than a few parts in 1e-6. The Sun's direction
//!   is held to the same edges (`SunDirectionNotUnit`).
//! - **Energy, where it applies: the first layer.** Absorbed equals emitted,
//!   `P_abs = P_front + P_back`, with `T` from the balance and the emitted
//!   power taken again from `T`. More emitted than absorbed is named
//!   `SpontaneousGeneration`, less `FirstLawBreach`. The body itself is held
//!   still, so the torque does no work on it; the wheels' energy comes from
//!   the observatory's electric power, and their speeds are not in the public
//!   telemetry, so no ledger of their energy is kept.
//!
//! `residual_conservation_error` is the larger of `|H - H_anchor - J|` over
//! `|J| + 1 N m s` and `|P_front + P_back - P_abs|` over `P_abs`; both close
//! to rounding, and the limit is 1e-9. The two ledgers confirm the
//! arithmetic of one rotation and of one balance: that the torque the
//! quaternion's product turns into ECI is the torque the direction-cosine
//! matrix books, and that the layer's emission, taken again from its
//! temperature, is what it absorbs. Neither is a measured quantity's
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
//!   Resolution B2): 0.00465047 au. There the observatory would be inside
//!   the Sun. The card's light is the Sun's as seen from outside it, falling
//!   as the inverse square of the distance; on a face turned to the Sun that
//!   law holds for a round Sun of even brightness at any distance above its
//!   radius. A distance of zero or below is no distance
//!   ([`StepReject::SunDistanceNotPositive`]).
//! - [`StepReject::SunBehindSunshield`]: the Sun at or behind the plane of
//!   the first layer. Its front is not lit, and the light falls on the cold
//!   side, the telescope, which the card does not hold.
//! - [`StepReject::SunshieldAboveKaptonLimit`]: the first layer's balance
//!   temperature above 673.15 K, 400 °C, the top of the range NASA states for
//!   the stability of Kapton ("Sunshield Membrane Coatings",
//!   jwst.nasa.gov). At L2, with the Sun on the normal, the layer's balance
//!   temperature is 347 K; the limit is met at about 0.27 au from the Sun.
//!
//! A flag, [`FLAG_OUTSIDE_FIELD_OF_REGARD`], marks a step whose Sun is outside
//! the science field of regard, pitch from -45 to +5 degrees and roll from -5
//! to +5 degrees (Arenberg et al. 2016, figure 7), where the sunshield is
//! built so that the Sun lights only its first layer. Outside it the Sun may
//! also light the edges of other layers, which the card does not hold; the
//! step lands. Pitch is the Sun's angle from the `J2`-`J3` plane towards the
//! boresight `J1`, `asin(s_J1 / |s|)`; roll is its angle about `J1` from
//! `-J3`, `asin(s_J2 / |(s_J2, s_J3)|)`. Each ratio is at most one by its own
//! arithmetic, so neither is pulled back to a range before its arcsine. In the
//! fitted month 180 of the 38,151 settled minutes had a roll beyond 5
//! degrees, at most 6.1.
//!
//! The telemetry's momentum departs by up to 3.5 N m s while the observatory
//! slews and returns when it settles: the body rate times one fixed matrix,
//! to 99.7 percent, an inertia off in its products. Across each large slew
//! the settled momentum also moves by about 0.26 N m s rms that the sunlight
//! does not account for; its cause was not identified. The comparison used
//! settled minutes only.
//!
//! What the card leaves out: the thrusters (an unload, a station-keeping
//! burn) are events of the flight, not predicted here: a run covers an
//! interval between them. The sunshield's real shape (five layers spread at
//! angles, the momentum trim flap at its aft end), which moves the point
//! where the push acts as the Sun moves on it; the card's point is fixed.
//! Light from the Earth and the Moon, the spacecraft bus and its solar array,
//! the gravity-gradient torque (about 1e-9 of the sunlight torque at L2).
//! Diffuse reflection. The wheels' momentum capacity, which no open source
//! gives: the card has no physical limit of its own on `|H|`, and the
//! momentum the observatory carried before its unloads in the record was
//! 73.7 N m s at most. The arithmetic's limit stands: a momentum so large
//! that the 64-bit `H` does not carry a step's impulse into it to 1e-9 of
//! the impulse booked plus 1 N m s is named by the momentum ledger.
//!
//! [`MomentumJwstSrp::gate_dt_s`] is the time in which the Sun's direction,
//! seen from an observatory that goes round the Sun once a year, turns by
//! 0.01 rad: 50,226 s. A step holds the Sun's direction, so over a longer
//! step the torque's error from holding it passes about one part in a
//! hundred. The floor is the ceiling over 2^20, 0.0479 s, under the
//! telemetry's attitude sample of 0.256 s.
//!
//! `core` only: no heap and no standard prelude. The card, the state and the
//! load are `Copy`. An accepted step writes into [`LastStateFrame64`] the
//! time, `H` in ECI, the torque in ECI, `|H|` and the ledger residual. The
//! 16-byte frame proof is not touched.

extern crate core;

use core::prelude::v1::*;

use crate::frame::LastStateFrame64;
use crate::normal::{normal_or_zero, normal_or_zero_each};

/// The Sun's irradiance at 1 au, W/m² (Kopp and Lean 2011).
pub const SOLAR_IRRADIANCE_AT_1_AU_W_PER_M2: f64 = 1360.8;
/// The speed of light, m/s.
pub const SPEED_OF_LIGHT_M_PER_S: f64 = 299_792_458.0;
/// The Stefan-Boltzmann constant, W/(m² K^4) (CODATA 2018).
pub const STEFAN_BOLTZMANN_W_PER_M2_K4: f64 = 5.670_374_419e-8;
/// The sunshield's lit area, m²: 300 kW (Arenberg et al. 2016) over
/// 1360.8 W/m², to two figures.
pub const SUNSHIELD_AREA_M2: f64 = 220.0;
/// The share of sunlight the first layer's Sun side absorbs, end of life.
pub const LAYER1_SOLAR_ABSORPTANCE: f64 = 0.47;
/// The emittance of the first layer's Sun side, end of life.
pub const LAYER1_FRONT_EMITTANCE: f64 = 0.72;
/// The emittance of the first layer's back, vapour-deposited aluminium.
pub const LAYER1_BACK_EMITTANCE: f64 = 0.025;
/// Where the push acts, from the centre of mass, along J1, m. Fitted.
pub const CENTRE_OF_PRESSURE_J1_M: f64 = -0.0410191;
/// Where the push acts, from the centre of mass, along J2, m. Fitted.
pub const CENTRE_OF_PRESSURE_J2_M: f64 = 0.00858426;
/// Where the push acts, from the centre of mass, along J3, m. Fitted.
pub const CENTRE_OF_PRESSURE_J3_M: f64 = -0.355723;
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
    /// The Sun is at or behind the plane of the sunshield's first layer: its front is not lit, and the light falls on the telescope's side, which the card does not hold.
    SunBehindSunshield,
    /// The first layer's balance temperature is above 673.15 K, 400 °C, the top of the range NASA states for the stability of Kapton.
    SunshieldAboveKaptonLimit,
    /// The trial step moved the angular momentum by other than the torque it booked.
    AngularMomentumBreach,
    /// The trial step had the first layer emit more than it absorbed.
    SpontaneousGeneration,
    /// The trial step had the first layer emit less than it absorbed.
    FirstLawBreach,
}

/// Held across one step: the attitude and the Sun.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MomentumLoad {
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
pub struct MomentumJwstSrp {
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
    /// Recorded copies of [`CENTRE_OF_PRESSURE_J1_M`], [`CENTRE_OF_PRESSURE_J2_M`] and [`CENTRE_OF_PRESSURE_J3_M`].
    pub centre_of_pressure_m: [f64; 3],
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

impl MomentumJwstSrp {
    /// Card: JWST's angular momentum under sunlight. The constructor takes no
    /// arguments.
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
            centre_of_pressure_m: [CENTRE_OF_PRESSURE_J1_M, CENTRE_OF_PRESSURE_J2_M, CENTRE_OF_PRESSURE_J3_M],
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
            && self.centre_of_pressure_m[0].to_bits() == CENTRE_OF_PRESSURE_J1_M.to_bits()
            && self.centre_of_pressure_m[1].to_bits() == CENTRE_OF_PRESSURE_J2_M.to_bits()
            && self.centre_of_pressure_m[2].to_bits() == CENTRE_OF_PRESSURE_J3_M.to_bits()
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
}

/// The observatory's momentum. The published fields are part of the state;
/// write them only through [`step`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MomentumState {
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
    /// The sunlight force on the sunshield, its size, N.
    pub force_n: f64,
    /// The Sun's pitch: its angle towards the boresight `J1`, degrees.
    pub sun_pitch_deg: f64,
    /// The Sun's roll: its angle out of the `J1`-`J3` plane, degrees.
    pub sun_roll_deg: f64,
    /// `cos(theta)`, the Sun's incidence on the first layer.
    pub incidence_cosine: f64,
    /// The sunlight at the observatory, W/m².
    pub irradiance_w_per_m2: f64,
    /// The first layer's balance temperature, K.
    pub layer1_temperature_k: f64,
    /// The power the first layer absorbs, W.
    pub absorbed_w: f64,
    /// The power its Sun side emits, W.
    pub emitted_front_w: f64,
    /// The power its back emits, W.
    pub emitted_back_w: f64,
    /// `P_front + P_back - P_abs`, W. Positive is energy from nowhere.
    pub energy_defect_w: f64,
    /// `| |q| - 1 |` of the attitude.
    pub quaternion_norm_defect: f64,
    /// The larger of the two ledgers' residuals.
    pub residual_conservation_error: f64,
    /// The Sun was outside the science field of regard on the last step.
    pub outside_field_of_regard: bool,
}

impl MomentumState {
    /// Start at this momentum, ECI, N m s, at time 0, the ledger anchored
    /// there with nothing booked. `None` for a momentum that is not finite.
    #[must_use]
    pub fn try_start(card: &MomentumJwstSrp, h_eci_nms: [f64; 3]) -> Option<Self> {
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
            irradiance_w_per_m2: 0.0,
            layer1_temperature_k: 0.0,
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

fn norm(a: [f64; 3]) -> f64 {
    dot(a, a).sqrt()
}

fn finite3(a: [f64; 3]) -> bool {
    a[0].is_finite() && a[1].is_finite() && a[2].is_finite()
}

/// The attitude's direction-cosine matrix, ECI to J-frame: rows are the
/// J-frame's axes in ECI (SM-12 Eq. 24), in the homogeneous form divided by
/// `|q|^2`, which for a unit quaternion is the same matrix.
fn eci_to_j(load: &MomentumLoad) -> [[f64; 3]; 3] {
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
fn j_to_eci_by_product(load: &MomentumLoad, v: [f64; 3]) -> [f64; 3] {
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

/// What the Sun does to the sunshield on one step.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Sunlight {
    sun_j: [f64; 3],
    incidence: f64,
    irradiance: f64,
    temperature_k: f64,
    absorbed_w: f64,
    emitted_front_w: f64,
    emitted_back_w: f64,
    force_j_n: [f64; 3],
    torque_j_nm: [f64; 3],
    pitch_deg: f64,
    roll_deg: f64,
    quaternion_norm_defect: f64,
}

/// How a step is carried out. The card's own is the attitude's rotation on
/// both routes and the full balance of the first layer; the tests run the
/// others and require a ledger to name them.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Method {
    /// Move `H` by the torque turned with the inverse rotation.
    inverse_rotation: bool,
    /// Leave the back's emission out of the layer's balance.
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
    card: &MomentumJwstSrp,
    state: &mut MomentumState,
    frame: &mut LastStateFrame64,
    load: MomentumLoad,
    dt_s: f64,
) -> Result<(), StepReject> {
    step_as_computed(card, state, frame, load, dt_s)?;
    normal_or_zero_state(state);
    Ok(())
}

/// The step as its equations compute it, before a value of the landed
/// state below the normal range is written as zero.
fn step_as_computed(
    card: &MomentumJwstSrp,
    state: &mut MomentumState,
    frame: &mut LastStateFrame64,
    load: MomentumLoad,
    dt_s: f64,
) -> Result<(), StepReject> {
    step_with(card, state, frame, load, dt_s, OWN_METHOD)
}

/// Each value of a landed state below the normal range written as zero
/// with its sign ([`crate::normal`]): the state carried to the next step
/// and published holds no subnormal number.
fn normal_or_zero_state(state: &mut MomentumState) {
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
    state.irradiance_w_per_m2 = normal_or_zero(state.irradiance_w_per_m2);
    state.layer1_temperature_k = normal_or_zero(state.layer1_temperature_k);
    state.absorbed_w = normal_or_zero(state.absorbed_w);
    state.emitted_front_w = normal_or_zero(state.emitted_front_w);
    state.emitted_back_w = normal_or_zero(state.emitted_back_w);
    state.energy_defect_w = normal_or_zero(state.energy_defect_w);
    state.quaternion_norm_defect = normal_or_zero(state.quaternion_norm_defect);
    state.residual_conservation_error = normal_or_zero(state.residual_conservation_error);
}

fn step_with(
    card: &MomentumJwstSrp,
    state: &mut MomentumState,
    frame: &mut LastStateFrame64,
    load: MomentumLoad,
    dt_s: f64,
    method: Method,
) -> Result<(), StepReject> {
    validate(card, state, &load, dt_s)?;
    let sun = sunlight(card, &load, method)?;
    commit(card, state, frame, &load, dt_s, &sun, method)
}

fn load_is_finite(load: &MomentumLoad) -> bool {
    load.q1.is_finite()
        && load.q2.is_finite()
        && load.q3.is_finite()
        && load.q4.is_finite()
        && load.sun_x.is_finite()
        && load.sun_y.is_finite()
        && load.sun_z.is_finite()
        && load.sun_distance_au.is_finite()
}

fn validate(card: &MomentumJwstSrp, state: &MomentumState, load: &MomentumLoad, dt_s: f64) -> Result<(), StepReject> {
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
fn is_unit(card: &MomentumJwstSrp, norm: f64) -> bool {
    norm >= 1.0 - card.unit_norm_tolerance && norm <= 1.0 + card.unit_norm_tolerance
}

/// The Sun in the J-frame, the first layer's balance, and the force and
/// torque of the sunlight on it.
fn sunlight(card: &MomentumJwstSrp, load: &MomentumLoad, method: Method) -> Result<Sunlight, StepReject> {
    let m = eci_to_j(load);
    let sun_j = times(&m, [load.sun_x, load.sun_y, load.sun_z]);
    if !finite3(sun_j) {
        return Err(StepReject::NonFiniteState);
    }
    // The first layer's Sun-side normal is -J3.
    let normal = [0.0, 0.0, -1.0];
    let incidence = dot(normal, sun_j);
    if !(incidence > 0.0) {
        return Err(StepReject::SunBehindSunshield);
    }
    let irradiance = card.irradiance_1_au_w_per_m2 / (load.sun_distance_au * load.sun_distance_au);
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
    let radiated_per_emittance = card.stefan_boltzmann_w_per_m2_k4 * card.area_m2 * t4_again;
    let absorbed_w = absorbed_flux * card.area_m2;
    let emitted_front_w = card.front_emittance * radiated_per_emittance;
    let emitted_back_w = card.back_emittance * radiated_per_emittance;
    let photon = irradiance * card.area_m2 * incidence / card.speed_of_light_m_per_s;
    let along_normal = photon * 2.0 * (1.0 - card.absorptance) * incidence
        + (2.0 / 3.0) * emitted_front_w / card.speed_of_light_m_per_s;
    let along_sun = photon * card.absorptance;
    let force_j_n = [
        -along_sun * sun_j[0] - along_normal * normal[0],
        -along_sun * sun_j[1] - along_normal * normal[1],
        -along_sun * sun_j[2] - along_normal * normal[2],
    ];
    let torque_j_nm = cross(card.centre_of_pressure_m, force_j_n);
    // Each ratio is at most one in size by its own arithmetic: a square
    // root of a sum of squares is at least the size of each term's root,
    // which is the term's size exactly, so nothing is pulled back.
    let pitch_deg = degrees(crate::math::asin(sun_j[0] / norm(sun_j)));
    let roll_deg = degrees(crate::math::asin(roll_sine(sun_j[1], sun_j[2])));
    let q_norm = (load.q1 * load.q1 + load.q2 * load.q2 + load.q3 * load.q3 + load.q4 * load.q4).sqrt();
    let sun = Sunlight {
        sun_j,
        incidence,
        irradiance,
        temperature_k,
        absorbed_w,
        emitted_front_w,
        emitted_back_w,
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
    card: &MomentumJwstSrp,
    state: &mut MomentumState,
    frame: &mut LastStateFrame64,
    load: &MomentumLoad,
    dt_s: f64,
    sun: &Sunlight,
    method: Method,
) -> Result<(), StepReject> {
    // Two routes from the body to ECI: the product moves H, the matrix books J.
    let by_product = if method.inverse_rotation {
        let conjugate = MomentumLoad { q1: -load.q1, q2: -load.q2, q3: -load.q3, ..*load };
        j_to_eci_by_product(&conjugate, sun.torque_j_nm)
    } else {
        j_to_eci_by_product(load, sun.torque_j_nm)
    };
    let booked = transpose_times(&eci_to_j(load), sun.torque_j_nm);
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
    next.irradiance_w_per_m2 = sun.irradiance;
    next.layer1_temperature_k = sun.temperature_k;
    next.absorbed_w = sun.absorbed_w;
    next.emitted_front_w = sun.emitted_front_w;
    next.emitted_back_w = sun.emitted_back_w;
    next.energy_defect_w = sun.emitted_front_w + sun.emitted_back_w - sun.absorbed_w;
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
    let energy_residual = next.energy_defect_w.abs() / next.absorbed_w;
    next.residual_conservation_error = if momentum_residual > energy_residual { momentum_residual } else { energy_residual };
    if !finite(&next) {
        return Err(StepReject::NonFiniteState);
    }
    if !(momentum_residual <= card.residual_conservation_limit) {
        return Err(StepReject::AngularMomentumBreach);
    }
    if next.energy_defect_w > card.residual_conservation_limit * next.absorbed_w {
        return Err(StepReject::SpontaneousGeneration);
    }
    if next.energy_defect_w < -card.residual_conservation_limit * next.absorbed_w {
        return Err(StepReject::FirstLawBreach);
    }
    *state = next;
    publish(state, frame);
    Ok(())
}

fn finite(state: &MomentumState) -> bool {
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
        && state.irradiance_w_per_m2.is_finite()
        && state.layer1_temperature_k.is_finite()
        && state.absorbed_w.is_finite()
        && state.emitted_front_w.is_finite()
        && state.emitted_back_w.is_finite()
        && state.energy_defect_w.is_finite()
        && state.quaternion_norm_defect.is_finite()
        && state.residual_conservation_error.is_finite()
}

fn publish(state: &MomentumState, frame: &mut LastStateFrame64) {
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
    /// from JWST then, from the JPL Horizons vectors of the record.
    fn may13() -> MomentumLoad {
        MomentumLoad {
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

    fn unit_sun(load: MomentumLoad) -> MomentumLoad {
        let n = norm([load.sun_x, load.sun_y, load.sun_z]);
        MomentumLoad { sun_x: load.sun_x / n, sun_y: load.sun_y / n, sun_z: load.sun_z / n, ..load }
    }

    fn start() -> MomentumState {
        MomentumState::try_start(&MomentumJwstSrp::observatory(), [-1.4853731, -0.6656560, 1.1653501]).unwrap()
    }

    #[test]
    fn the_observatory_holds_its_constants_and_a_moved_one_is_refused() {
        let card = MomentumJwstSrp::observatory();
        assert!(card.is_valid());
        let mut moved = card;
        moved.absorptance = f64::from_bits(LAYER1_SOLAR_ABSORPTANCE.to_bits() + 1);
        assert!(!moved.is_valid());
        let mut state = start();
        let before = state;
        let mut frame = blank();
        assert_eq!(step(&moved, &mut state, &mut frame, unit_sun(may13()), 60.0), Err(StepReject::InvalidObservatory));
        assert_eq!(state, before);
    }

    #[test]
    fn the_ledgers_close_at_every_step_from_the_floor_to_the_ceiling() {
        let card = MomentumJwstSrp::observatory();
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

    #[test]
    fn a_quaternion_that_is_not_unit_is_refused_with_the_state_whole() {
        let card = MomentumJwstSrp::observatory();
        let mut state = start();
        let before = state;
        let mut frame = blank();
        let load = unit_sun(may13());
        let stretched = MomentumLoad { q1: load.q1 * 1.00001, q2: load.q2 * 1.00001, q3: load.q3 * 1.00001, q4: load.q4 * 1.00001, ..load };
        assert_eq!(step(&card, &mut state, &mut frame, stretched, 60.0), Err(StepReject::QuaternionNotUnit));
        let zero = MomentumLoad { q1: 0.0, q2: 0.0, q3: 0.0, q4: 0.0, ..load };
        assert_eq!(step(&card, &mut state, &mut frame, zero, 60.0), Err(StepReject::QuaternionNotUnit));
        assert_eq!(state, before);
        assert_eq!(frame.proof, [7; 16]);
        let long_sun = MomentumLoad { sun_x: load.sun_x * 1.01, ..load };
        assert_eq!(step(&card, &mut state, &mut frame, long_sun, 60.0), Err(StepReject::SunDirectionNotUnit));
        let nowhere = MomentumLoad { sun_distance_au: 0.0, ..load };
        assert_eq!(step(&card, &mut state, &mut frame, nowhere, 60.0), Err(StepReject::SunDistanceNotPositive));
        assert_eq!(state, before);
    }

    #[test]
    fn the_sun_behind_the_sunshield_is_refused() {
        let card = MomentumJwstSrp::observatory();
        let mut state = start();
        let before = state;
        let mut frame = blank();
        // The identity attitude puts J3 on ECI z; a Sun on +z is behind.
        let behind = MomentumLoad { q1: 0.0, q2: 0.0, q3: 0.0, q4: 1.0, sun_x: 0.0, sun_y: 0.0, sun_z: 1.0, sun_distance_au: 1.0 };
        assert_eq!(step(&card, &mut state, &mut frame, behind, 60.0), Err(StepReject::SunBehindSunshield));
        let edge_on = MomentumLoad { sun_x: 1.0, sun_z: 0.0, ..behind };
        assert_eq!(step(&card, &mut state, &mut frame, edge_on, 60.0), Err(StepReject::SunBehindSunshield));
        assert_eq!(state, before);
    }

    #[test]
    fn the_kapton_limit_is_met_near_a_quarter_of_an_au() {
        let card = MomentumJwstSrp::observatory();
        let face_on = MomentumLoad { q1: 0.0, q2: 0.0, q3: 0.0, q4: 1.0, sun_x: 0.0, sun_y: 0.0, sun_z: -1.0, sun_distance_au: 1.0 };
        let mut state = start();
        let mut frame = blank();
        step(&card, &mut state, &mut frame, MomentumLoad { sun_distance_au: 0.28, ..face_on }, 60.0).unwrap();
        assert!(state.layer1_temperature_k < KAPTON_LIMIT_K);
        let before = state;
        assert_eq!(
            step(&card, &mut state, &mut frame, MomentumLoad { sun_distance_au: 0.26, ..face_on }, 60.0),
            Err(StepReject::SunshieldAboveKaptonLimit)
        );
        assert_eq!(state, before);
        // At L2 with the Sun on the normal: 347 K.
        step(&card, &mut state, &mut frame, MomentumLoad { sun_distance_au: 1.0194, ..face_on }, 60.0).unwrap();
        assert!((state.layer1_temperature_k - 347.5).abs() < 0.5, "{}", state.layer1_temperature_k);
    }

    #[test]
    fn the_field_of_regard_is_flagged_and_the_step_lands() {
        let card = MomentumJwstSrp::observatory();
        let mut state = start();
        let mut frame = blank();
        // Identity attitude: J1 = x, J3 = z. The Sun 50 degrees past the normal, away from the boresight.
        let (s, c) = (crate::math::sin(50.0_f64.to_radians()), crate::math::cos(50.0_f64.to_radians()));
        let far = MomentumLoad { q1: 0.0, q2: 0.0, q3: 0.0, q4: 1.0, sun_x: -s, sun_y: 0.0, sun_z: -c, sun_distance_au: 1.0 };
        step(&card, &mut state, &mut frame, far, 60.0).unwrap();
        assert!(state.outside_field_of_regard);
        assert_eq!(frame.flags, FLAG_OUTSIDE_FIELD_OF_REGARD);
        assert!((state.sun_pitch_deg + 50.0).abs() < 1e-9);
        let (s, c) = (crate::math::sin(20.0_f64.to_radians()), crate::math::cos(20.0_f64.to_radians()));
        let inside = MomentumLoad { sun_x: -s, sun_z: -c, ..far };
        step(&card, &mut state, &mut frame, inside, 60.0).unwrap();
        assert!(!state.outside_field_of_regard);
        assert_eq!(frame.flags, 0);
    }

    /// The force and the torque at a face-on Sun, worked by hand.
    #[test]
    fn the_force_at_a_face_on_sun_is_the_photon_pressure_on_the_layer() {
        let card = MomentumJwstSrp::observatory();
        let face_on = MomentumLoad { q1: 0.0, q2: 0.0, q3: 0.0, q4: 1.0, sun_x: 0.0, sun_y: 0.0, sun_z: -1.0, sun_distance_au: 1.0 };
        let mut state = start();
        let mut frame = blank();
        step(&card, &mut state, &mut frame, face_on, 60.0).unwrap();
        let p = SOLAR_IRRADIANCE_AT_1_AU_W_PER_M2 / SPEED_OF_LIGHT_M_PER_S * SUNSHIELD_AREA_M2;
        let thermal = (2.0 / 3.0) * LAYER1_SOLAR_ABSORPTANCE * LAYER1_FRONT_EMITTANCE / (LAYER1_FRONT_EMITTANCE + LAYER1_BACK_EMITTANCE);
        let expected = p * (LAYER1_SOLAR_ABSORPTANCE + 2.0 * (1.0 - LAYER1_SOLAR_ABSORPTANCE) + thermal);
        assert!((state.force_n - expected).abs() < 1e-15, "{} {}", state.force_n, expected);
        let r = [CENTRE_OF_PRESSURE_J1_M, CENTRE_OF_PRESSURE_J2_M, CENTRE_OF_PRESSURE_J3_M];
        let torque = cross(r, [0.0, 0.0, expected]);
        for k in 0..3 {
            assert!((state.torque_body_nm[k] - torque[k]).abs() < 1e-18);
            assert!((state.torque_eci_nm[k] - torque[k]).abs() < 1e-18);
        }
    }

    #[test]
    fn nothing_is_filled_in() {
        let card = MomentumJwstSrp::observatory();
        let mut state = start();
        let before = state;
        let mut frame = blank();
        let load = unit_sun(may13());
        for bad in [
            MomentumLoad { q1: f64::NAN, ..load },
            MomentumLoad { sun_z: f64::INFINITY, ..load },
            MomentumLoad { sun_distance_au: f64::NEG_INFINITY, ..load },
        ] {
            assert_eq!(step(&card, &mut state, &mut frame, bad, 60.0), Err(StepReject::NonFiniteInput));
            assert_eq!(state, before);
        }
        assert_eq!(step(&card, &mut state, &mut frame, load, 0.0), Err(StepReject::NonPositiveDt));
        assert_eq!(state, before);
        assert!(MomentumState::try_start(&card, [f64::NAN, 0.0, 0.0]).is_none());
    }

    /// Rule 10: H moved by the torque turned the wrong way is named by the
    /// momentum ledger.
    #[test]
    fn a_torque_turned_the_wrong_way_is_named_by_the_momentum_ledger() {
        let card = MomentumJwstSrp::observatory();
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
    fn a_layer_balance_without_its_back_is_named_by_the_energy_ledger() {
        let card = MomentumJwstSrp::observatory();
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
        let card = MomentumJwstSrp::observatory();
        let mut state = start();
        state.h_anchor_eci_nms[1] += 1.0e-6;
        let mut frame = blank();
        assert_eq!(step(&card, &mut state, &mut frame, unit_sun(may13()), 60.0), Err(StepReject::AngularMomentumBreach));
        let mut state = start();
        state.h_eci_nms[0] -= 1.0e-6;
        assert_eq!(step(&card, &mut state, &mut frame, unit_sun(may13()), 60.0), Err(StepReject::AngularMomentumBreach));
    }

    /// The Sun's distance at or under its radius is inside the Sun: refused
    /// by name with the state whole, at a grazing Sun where the layer stays
    /// under the Kapton limit, down to 1e-9 au. Just above the radius the
    /// step lands.
    #[test]
    fn a_sun_distance_inside_the_sun_is_refused() {
        let card = MomentumJwstSrp::observatory();
        assert!((SUN_RADIUS_AU - 0.004650467260962157).abs() < 1.0e-18, "{SUN_RADIUS_AU}");
        let grazing = |distance: f64| {
            let z: f64 = -1.0e-6;
            MomentumLoad { q1: 0.0, q2: 0.0, q3: 0.0, q4: 1.0, sun_x: (1.0 - z * z).sqrt(), sun_y: 0.0, sun_z: z, sun_distance_au: distance }
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
        assert!(state.layer1_temperature_k < KAPTON_LIMIT_K);
        assert!(state.residual_conservation_error <= RESIDUAL_CONSERVATION_LIMIT);
    }

    /// The momentum account is held to what is in play in it, the impulse
    /// booked, and not to |H|: an impulse that never reached H, as at a start
    /// of 1e17 N m s, where the red team saw 70.73 N m s booked, H moved by
    /// none of it, and a residual of 7.1e-16 against the old scale, is named
    /// on the first step. At 74 N m s, the most in the record, a million
    /// steps of the floor, the hardest step, close.
    #[test]
    fn an_impulse_that_never_reaches_h_is_named_and_the_records_momentum_closes() {
        let card = MomentumJwstSrp::observatory();
        let load = unit_sun(may13());
        for h0 in [[1.0e17, 0.0, 0.0], [1.0e12, 0.0, 0.0], [0.0, -1.0e9, 0.0]] {
            let mut state = MomentumState::try_start(&card, h0).unwrap();
            let before = state;
            let mut frame = blank();
            assert_eq!(step(&card, &mut state, &mut frame, load, card.gate_dt_s().unwrap()), Err(StepReject::AngularMomentumBreach), "{h0:?}");
            assert_eq!(state, before);
        }
        let direction = [-1.4853731309945153, -0.6656560404607257, 1.1653501123098389];
        let size = norm(direction);
        let h0 = [74.0 * direction[0] / size, 74.0 * direction[1] / size, 74.0 * direction[2] / size];
        let mut state = MomentumState::try_start(&card, h0).unwrap();
        let mut frame = blank();
        let floor = card.floor_dt_s().unwrap();
        let mut worst = 0.0_f64;
        for _ in 0..1_000_000 {
            step(&card, &mut state, &mut frame, load, floor).unwrap();
            worst = worst.max(state.residual_conservation_error);
        }
        assert!(worst <= RESIDUAL_CONSERVATION_LIMIT, "{worst:e}");
        // The account still names an unbooked move of 1e-8 of what is in play.
        let in_play = norm(state.impulse_eci_nms) + MOMENTUM_SCALE_FLOOR_NMS;
        let mut moved = state;
        moved.h_eci_nms[2] += 1.0e-8 * in_play;
        assert_eq!(step(&card, &mut moved, &mut frame, load, floor), Err(StepReject::AngularMomentumBreach));
    }

    /// The norm's edges are 0.999999 and 1.000001, both inclusive, on either
    /// side alike: one component written at either decimal lands, the next
    /// 64-bit number past it is refused. The same for the Sun's direction.
    #[test]
    fn the_unit_norm_holds_its_decimal_edges_on_both_sides() {
        let card = MomentumJwstSrp::observatory();
        // In 64-bit arithmetic the edges are the numbers nearest the decimals.
        assert_eq!((1.0 - UNIT_NORM_TOLERANCE).to_bits(), 0.999999_f64.to_bits());
        assert_eq!((1.0 + UNIT_NORM_TOLERANCE).to_bits(), 1.000001_f64.to_bits());
        let lit = |q1: f64, q4: f64| {
            // Identity-like attitudes: q4 alone keeps J3 on z, q1 alone turns it to -z.
            let sun_z = if q1 != 0.0 { 1.0 } else { -1.0 };
            MomentumLoad { q1, q2: 0.0, q3: 0.0, q4, sun_x: 0.0, sun_y: 0.0, sun_z, sun_distance_au: 1.0 }
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
        let face_on = |sun_z: f64| MomentumLoad { q1: 0.0, q2: 0.0, q3: 0.0, q4: 1.0, sun_x: 0.0, sun_y: 0.0, sun_z, sun_distance_au: 1.0 };
        for sun_z in [-0.999999, -1.000001] {
            step(&card, &mut state, &mut frame, face_on(sun_z), 60.0).unwrap();
        }
        for sun_z in [-below, -above] {
            assert_eq!(step(&card, &mut state, &mut frame, face_on(sun_z), 60.0), Err(StepReject::SunDirectionNotUnit), "{sun_z}");
        }
    }

    /// The pitch and the roll are each the arcsine of a ratio that is at most
    /// one by its own arithmetic, with nothing pulled back: a Sun direction
    /// 1e-6 longer than one, almost on the boresight, has the pitch of its
    /// direction, under 90 degrees, where a clamp of its J1 component to
    /// one gave 90 exactly.
    #[test]
    fn the_pitch_and_the_roll_are_the_directions_own_with_nothing_pulled_back() {
        let card = MomentumJwstSrp::observatory();
        let mut state = start();
        let mut frame = blank();
        let length = 1.0 + 0.9e-6;
        let off = 1.0e-3_f64;
        let sun = [length * (1.0 - off * off).sqrt(), 0.0, -length * off];
        assert!(sun[0] > 1.0);
        let along = MomentumLoad { q1: 0.0, q2: 0.0, q3: 0.0, q4: 1.0, sun_x: sun[0], sun_y: sun[1], sun_z: sun[2], sun_distance_au: 1.0 };
        step(&card, &mut state, &mut frame, along, 60.0).unwrap();
        let expected = 90.0 - off.to_degrees();
        assert!(state.sun_pitch_deg < 90.0 && (state.sun_pitch_deg - expected).abs() < 1.0e-6, "{}", state.sun_pitch_deg);
        assert!(state.outside_field_of_regard);
        // The Sun all but on J2: a roll of nearly 90 degrees, from a ratio at most one.
        let sideways = MomentumLoad { sun_x: 0.0, sun_y: length * (1.0 - off * off).sqrt(), sun_z: -length * off, ..along };
        step(&card, &mut state, &mut frame, sideways, 60.0).unwrap();
        assert!(state.sun_roll_deg < 90.0 && (state.sun_roll_deg - expected).abs() < 1.0e-6, "{}", state.sun_roll_deg);
        // A grazing Sun so near J1 that its squares underflow keeps a roll.
        let tiny = MomentumLoad { sun_x: 1.0, sun_y: 1.0e-200, sun_z: -1.0e-200, ..along };
        step(&card, &mut state, &mut frame, tiny, 60.0).unwrap();
        assert!((state.sun_roll_deg - 45.0).abs() < 1.0e-9, "{}", state.sun_roll_deg);
    }

    #[test]
    fn the_same_run_gives_the_same_bits() {
        let card = MomentumJwstSrp::observatory();
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
    fn normal_or_zero_throughout(state: &MomentumState) -> bool {
        let mut held = *state;
        normal_or_zero_state(&mut held);
        held == *state
    }

    /// The red team's grazing Sun on engine 0.13.0 (D1 and D2), as on
    /// `momentum_jwst_dihedral`: the Sun at (1, 0, -1e-300) at 1, 100 and
    /// 10,000 au from a momentum of zero, in steps of 0.05 s. The momentum,
    /// its growth and the torque the step computes are under the smallest
    /// normal number at 100 and 10,000 au; each lands as zero with its sign,
    /// every value of the landed state is normal or zero, the frame is the
    /// computed step's, and the landed state steps on.
    #[test]
    fn a_grazing_sun_lands_every_value_normal_or_zero_and_the_state_steps_on() {
        let card = MomentumJwstSrp::observatory();
        let mut computed_subnormal = 0;
        for distance_au in [1.0, 100.0, 1.0e4] {
            let load = MomentumLoad { q1: 0.0, q2: 0.0, q3: 0.0, q4: 1.0, sun_x: 1.0, sun_y: 0.0, sun_z: -1.0e-300, sun_distance_au: distance_au };
            let start = MomentumState::try_start(&card, [0.0; 3]).unwrap();
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
            assert_eq!(frame.to_bytes(), frame_computed.to_bytes());
            let mut held = as_computed;
            normal_or_zero_state(&mut held);
            assert_eq!(held, state);
            for _ in 0..3 {
                step(&card, &mut state, &mut frame, load, 0.05).unwrap();
                assert!(normal_or_zero_throughout(&state), "{distance_au} au: {state:?}");
            }
            let mut again = MomentumState::try_start(&card, state.h_eci_nms).unwrap();
            step(&card, &mut again, &mut blank(), load, 0.05).unwrap();
            assert!(normal_or_zero_throughout(&again));
        }
        assert!(computed_subnormal >= 2, "{computed_subnormal}");
    }
}
