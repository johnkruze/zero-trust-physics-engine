//! What an agent reads of the momentum_jwst_srp card: its fields and bounds,
//! its step floor and ceiling, its refusals word for word from the card's
//! source, its frame, its worked examples and its record. A child module of
//! `descriptor.rs`, so it uses that file's own builders.

use genesis_kernel::momentum_jwst_srp;
use serde_json::Value;

use super::{Advisory, Card, Role, finish, obj, pair_examples_of, scalar, scalar_value, slots, token, vec3_stated, vec3_value};

const DOMAIN: &str = "aerospace";
const CARD: &str = "momentum_jwst_srp";

/// What the card is, in one sentence of the list.
const SPECIMEN: &str = "The total angular momentum of the James Webb Space Telescope at Sun-Earth L2, held by its reaction wheels and growing between momentum unloads under the pressure of sunlight on its sunshield, as a function of the observatory's attitude quaternion: one sunlit layer of 220 m² with JWST's published optical numbers, its centre of pressure fitted to the observatory's own momentum telemetry of 12 May to 14 June 2023. Compared with the held-back month of 15 June to 14 July 2023 it met four of five marks: the daily growth of |H| within its allowance on 23 of 28 days, |H| at the end of 10.7 and 16.3 days between thruster events within 8.4 and 0.9 percent, the hourly growth rate's error 0.437 of its spread, the torque on one attitude 15.4 against 58.6 µN m rms; it missed the daily change of the vector, within its allowance on 14 of 28 days, its torque about the boresight off by about 30 µN m where the Sun's roll is beyond 2 degrees. Scored blind by the same marks on the two months after it, it met two of five on 15 July to 14 August 2023, the daily growth of |H| on 15 of 28 days, the vector on 8 of 28 and |H| at the end of the run after an unload 16.2 percent low, and three of five on 15 August to 14 September 2023, the daily growth of |H| on 17 of 30 days and the vector on 8 of 30: 9 of 15 marks in its three blind months, the daily change of the vector met in none.";

/// The bound of a quaternion component and of a component of the Sun's
/// direction: a unit vector's component, with the card's tolerance on the norm.
const UNIT_COMPONENT: f64 = 1.0 + momentum_jwst_srp::UNIT_NORM_TOLERANCE;

pub(super) fn card() -> Card {
    let observatory = momentum_jwst_srp::MomentumJwstSrp::observatory();
    let gate = observatory.gate_dt_s().expect("observatory gate");
    let floor = observatory.floor_dt_s().expect("observatory floor");
    let quaternion = |name: &'static str, which: &str| {
        scalar(
            name,
            Role::Load,
            "1",
            Some(-UNIT_COMPONENT),
            false,
            Some(UNIT_COMPONENT),
            false,
            format!(
                "from {} to {}, both inclusive: {which} of the attitude quaternion of the ACS J-frame against ECI, as the telemetry gives it (q4 is the scalar). The four together must have a norm from 0.999999 to 1.000001, both inclusive, {} either side of one, each edge the 64-bit number nearest its decimal; a norm outside is refused as QuaternionNotUnit, and a component beyond these bounds is refused so before the step",
                token(-UNIT_COMPONENT),
                token(UNIT_COMPONENT),
                token(momentum_jwst_srp::UNIT_NORM_TOLERANCE)
            ),
        )
    };
    let sun = |name: &'static str, axis: &str| {
        scalar(
            name,
            Role::Load,
            "1",
            Some(-UNIT_COMPONENT),
            false,
            Some(UNIT_COMPONENT),
            false,
            format!(
                "from {} to {}, both inclusive: the ECI {axis} component of the unit vector from the observatory to the Sun, from an ephemeris of JWST (the record used JPL Horizons, target -170). The three together must have a norm from 0.999999 to 1.000001, both inclusive, {} either side of one, each edge the 64-bit number nearest its decimal; a norm outside is refused as SunDirectionNotUnit",
                token(-UNIT_COMPONENT),
                token(UNIT_COMPONENT),
                token(momentum_jwst_srp::UNIT_NORM_TOLERANCE)
            ),
        )
    };
    finish(
        DOMAIN,
        CARD,
        SPECIMEN,
        vec![
            vec3_stated(
                "h_eci_nms",
                "N m s",
                "no bound of its own: the observatory's total angular momentum at the start, three components in ECI (J2000 equatorial), N m s. From the telemetry it is SA_ZMOMMAG times the unit vector SA_ZMOMECIX, SA_ZMOMECIY, SA_ZMOMECIZ; the most before an unload in the record was 73.8 N m s. The reaction wheels' capacity is not published. A momentum so large that 64-bit arithmetic does not carry a step's impulse into it to 1e-9 of the impulse booked plus 1 N m s is refused as AngularMomentumBreach: on the worked load from 10^9 N m s on the first step of any size, from 10^6 N m s within 70 steps from the floor to one minute, and from 10^3 N m s after 27,000 steps of 0.256 s. A run continued through from starts from the upstream's final h_eci_nms".to_string(),
            ),
            quaternion("q1", "SA_ZATTEST1, the first vector component"),
            quaternion("q2", "SA_ZATTEST2, the second vector component"),
            quaternion("q3", "SA_ZATTEST3, the third vector component"),
            quaternion("q4", "SA_ZATTEST4, the scalar component"),
            sun("sun_x", "x"),
            sun("sun_y", "y"),
            sun("sun_z", "z"),
            scalar(
                "sun_distance_au",
                Role::Load,
                "au",
                Some(0.0),
                true,
                None,
                false,
                format!(
                    "above 0 au: the observatory's distance from the Sun. Sunlight falls as its inverse square. At or under the Sun's radius, {} au (695,700 km, the IAU's nominal solar radius, over the astronomical unit of 149,597,870.7 km), the observatory would be inside the Sun, and the step is refused as SunDistanceInsideSun. Near the Sun the first layer's balance temperature passes {} K, 400 degC, the top of NASA's range for Kapton, and the step is refused as SunshieldAboveKaptonLimit: with the Sun on the layer's normal that is at about 0.27 au, and the more the Sun grazes the layer the closer to the Sun that limit lies: at an incidence cosine under 2.9e-4 it lies inside the Sun's radius. At L2 the distance is about 1.01 to 1.02 au",
                    token(momentum_jwst_srp::SUN_RADIUS_AU),
                    token(momentum_jwst_srp::KAPTON_LIMIT_K)
                ),
            ),
        ],
        floor,
        "the ceiling divided by 2^20, under the telemetry's attitude sample of 0.256 s",
        gate,
        "the time in which the Sun's direction, seen from an observatory going round the Sun once a year, turns by 0.01 rad. A step holds the Sun's direction, so over a longer step the torque's error from holding it passes about one part in a hundred",
        None,
        "The load, the attitude and the Sun, is held across each step; over a held load the sunlight torque is constant and the step H(t + dt) = H(t) + tau dt is exact. The record stepped one minute at a time, with each minute's attitude from the telemetry and the Sun from JPL Horizons. A run covers an interval between thruster events: a momentum unload or a station-keeping burn moves the momentum by thrust, outside the sunlight's torque, so a run ends before one and a new run starts after it from the telemetered momentum. The momentum the telemetry reports while the observatory slews departs from the settled value by up to 3.5 N m s in the held-back month and comes back when it settles; the record compares only settled minutes. The two ledgers confirm the arithmetic of one rotation and of one balance: the momentum ledger holds H - H_anchor - J to 1e-9 of the impulse booked plus 1 N m s, where the torque that moves H is turned into ECI by the quaternion's product and the impulse J by the transpose of the telemetry's direction-cosine matrix; the energy ledger holds the first layer's emission, taken again from its balance temperature, to 1e-9 of what it absorbs. From 1 to 74 N m s, the most in the record, two million steps of the worked load close it at the floor, at 0.256 s, at one minute and at the ceiling, to 6.7e-10 at the floor. The Sun behind the plane of the sunshield's first layer is refused as SunBehindSunshield. The Sun's pitch is asin(s_J1 / |s|), its angle towards the boresight, and its roll asin(s_J2 / |(s_J2, s_J3)|), its angle about the boresight: each the angle of the direction as given, with nothing pulled back. A Sun outside the science field of regard, pitch -45 to +5 degrees and roll -5 to +5 degrees, lands with FLAG_OUTSIDE_FIELD_OF_REGARD.".into(),
        &[
            "NonFiniteInput",
            "NonFiniteState",
            "NonPositiveDt",
            "InvalidObservatory",
            "QuaternionNotUnit",
            "SunDirectionNotUnit",
            "SunDistanceNotPositive",
            "SunDistanceInsideSun",
            "SunBehindSunshield",
            "SunshieldAboveKaptonLimit",
            "AngularMomentumBreach",
            "SpontaneousGeneration",
            "FirstLawBreach",
        ],
        vec![Advisory {
            name: "FLAG_OUTSIDE_FIELD_OF_REGARD",
            mask: momentum_jwst_srp::FLAG_OUTSIDE_FIELD_OF_REGARD,
            seals: true,
        }],
        slots(&[
            ("t", "Time since the start of the run, in seconds."),
            ("pos[0]", "h_eci_nms[0]"),
            ("pos[1]", "h_eci_nms[1]"),
            ("pos[2]", "h_eci_nms[2]"),
            ("vel[0]", "torque_eci_nm[0]"),
            ("vel[1]", "torque_eci_nm[1]"),
            ("vel[2]", "torque_eci_nm[2]"),
            ("force_torque", "h_magnitude_nms"),
            ("residual", "Conservation residual: the larger of the angular-momentum ledger and the first layer's energy balance."),
            ("flags", "FLAG_OUTSIDE_FIELD_OF_REGARD when the Sun was outside the science field of regard on the step."),
            ("proof", "Not written by the step."),
        ]),
    )
}

/// The observatory on 2023-05-13 at 00:00:30 UTC, the day after the unload of
/// 12 May: the telemetered momentum, attitude and the Sun from JPL Horizons.
/// One step of a minute, and an hour of such steps.
pub(super) fn examples() -> (Value, Value) {
    pair_examples_of(
        DOMAIN,
        CARD,
        obj(&[("h_eci_nms", vec3_value([-1.4853731309945153, -0.6656560404607257, 1.1653501123098389]))]),
        obj(&[
            ("q1", scalar_value(0.7216001749)),
            ("q2", scalar_value(0.0366541296)),
            ("q3", scalar_value(-0.5597470999)),
            ("q4", scalar_value(0.4057497233)),
            ("sun_x", scalar_value(0.6160922205743601)),
            ("sun_y", scalar_value(0.7226624561625701)),
            ("sun_z", scalar_value(0.31335179942172875)),
            ("sun_distance_au", scalar_value(1.0193963859269388)),
        ]),
        scalar_value(60.0),
        60,
    )
}

/// The record, in the five parts every card gives: the object, the data, the
/// pass line, the result and the limits.
pub(super) fn validation(_specimen: &str) -> [String; 5] {
    [
        "The James Webb Space Telescope at Sun-Earth L2, holding its attitude on six reaction wheels: its total angular momentum H, wheels and body together, as the flight software telemeters it, growing between momentum unloads under the pressure of sunlight on the sunshield. The compared quantities are the change of H and of |H| between settled minutes, the observatory not slewing.".into(),
        "The JWST engineering database at MAST, open with no token: SA_ZMOMMAG and the unit vector SA_ZMOMECIX, SA_ZMOMECIY, SA_ZMOMECIZ every 4.096 s, the attitude quaternion SA_ZATTEST1 to SA_ZATTEST4 every 0.256 s, the slew status SA_ZSLEWST and the unload status SA_ZMOMUNLST, read as one row a minute; the Sun's direction and distance from JPL Horizons, target -170. Fitted, and only the centre of pressure: the UTC days 2023-05-12 to 2023-06-14, 763 hourly blocks. Held back: 2023-06-15 to 2023-07-14. Scored blind after it, each month downloaded unread and fingerprinted before any of its files was opened: 2023-07-15 to 2023-08-14, with momentum_jwst_dihedral's comparison, and 2023-08-15 to 2023-09-14, the ledger of 2026-10-16. The sunshield's area from Arenberg et al. (2016), Proc. SPIE 9904, 990405; its optical numbers from the Origins Space Telescope study report (2019), appendix C; the irradiance from Kopp and Lean (2011). Record: evidence/research/momentum_jwst_srp.".into(),
        "Five marks fixed before the held-back month was downloaded, the protocol's SHA-256 in the commit that fixed the constants: A, the growth of |H| over each day within 0.1 of the observed plus 0.2 N m s on 80 percent of days; B, the change of the vector H over each day within the same allowance on 80 percent of days; C, |H| at the end of each run of 3 days or more between thruster events within 10 percent; D, the hourly growth rate of |H| with an rms error at most 0.5 of its spread; E, the torque on one attitude with an rms error at most 0.4 of the rms torque. The same five marks, unchanged, scored the two later months, each with a forecast registered before the month was opened.".into(),
        "Met four of five marks and missed B. A: 23 of 28 days, 82 percent. B: 14 of 28 days, 50 percent, missed. C: -8.4 percent after 10.7 days, 28.05 against 30.63 N m s, and -0.9 percent after 16.3 days, 44.64 against 45.05. D: 0.058 against 0.133 N m s per hour, 0.437, correlation 0.900. E: 15.4 against 58.6 µN m over 163 stretches of one attitude, 0.262. Two models with nothing of the sunshield in them, a constant torque in the body frame and in ECI, met none of the five. The engine's momentum agreed with the record's own arithmetic to 1.9e-6 N m s at every minute. Scored again by the same marks, blind, on the card as served: on 2023-07-15 to 2023-08-14, two of five, A 15 of 28 days, B 8 of 28, C missed at -16.2 percent after the 3.9-day run that began after an unload, D 0.384, E 0.314; on 2023-08-15 to 2023-09-14, three of five, missed as forecast, A 17 of 30 days, B 8 of 30, C -7.4 percent after 29.3 days, 26.63 against 28.75 N m s, D 0.421, E 0.347, the torque about J1 off by -29.5 and +31.5 µN m on average beyond 2 degrees of roll. In all, 9 of 15 marks in the three blind months: D and E met in each, B in none, A in the first only; the first month's four of five did not come back.".into(),
        "Compared on one observatory over three months: the held-back month, 2023-06-15 to 2023-07-14, the Sun at a pitch of -44.1 to +4.6 degrees and a roll of -5.3 to +4.6 degrees, |H| from 1.6 to 45.1 N m s, in runs of up to 16.3 days between thruster events; and, blind, 2023-07-15 to 2023-09-14, the Sun at a pitch of -44.5 to +4.5 degrees and a roll of -6.3 to +6.4 degrees, |H| from 1.9 to 54.9 N m s, in runs of up to 29.3 days. Where the Sun's roll is beyond 2 degrees the torque about the boresight is off by about 30 µN m: one flat layer turns roll into torque about a quarter as fast as the observatory does; within 2 degrees of roll the torque's error is 5.8 against 56.7 µN m. Across each slew of more than 0.5 rad the settled momentum moved by 0.26 N m s rms that the sunlight does not account for; its cause is not identified. While the observatory slews the telemetry's momentum departs by up to 3.5 N m s and returns when it settles, the body rate times a fixed matrix to 99.7 percent; only settled minutes were compared. The thrusters, the wheels' speeds and attitudes outside the field of regard are outside the comparison.".into(),
    ]
}

/// Each refusal's sentence, word for word from the doc comment above the
/// variant in the card's source.
pub(super) fn meaning(name: &str) -> Option<&'static str> {
    Some(match name {
        "NonFiniteInput" => "`dt` or a number of the load was NaN or infinite.",
        "NonFiniteState" => "The incoming state, or a number of the trial step, was not finite.",
        "NonPositiveDt" => "`dt` was zero or negative.",
        "InvalidObservatory" => "A recorded constant is not this observatory's.",
        "QuaternionNotUnit" => "The attitude quaternion's norm is outside 0.999999 to 1.000001: it is not an attitude.",
        "SunDirectionNotUnit" => "The Sun's direction's norm is outside 0.999999 to 1.000001: it is not a unit vector.",
        "SunDistanceNotPositive" => "The Sun's distance is zero or negative.",
        "SunDistanceInsideSun" => "The Sun's distance is at or under the Sun's radius, 0.00465047 au (695,700 km): the observatory would be inside the Sun.",
        "SunBehindSunshield" => "The Sun is at or behind the plane of the sunshield's first layer: its front is not lit, and the light falls on the telescope's side, which the card does not hold.",
        "SunshieldAboveKaptonLimit" => "The first layer's balance temperature is above 673.15 K, 400 °C, the top of the range NASA states for the stability of Kapton.",
        "AngularMomentumBreach" => "The trial step moved the angular momentum by other than the torque it booked.",
        "SpontaneousGeneration" => "The trial step had the first layer emit more than it absorbed.",
        "FirstLawBreach" => "The trial step had the first layer emit less than it absorbed.",
        _ => return None,
    })
}
