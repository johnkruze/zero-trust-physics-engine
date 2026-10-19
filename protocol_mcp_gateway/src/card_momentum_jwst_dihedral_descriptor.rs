//! What an agent reads of the momentum_jwst_dihedral card: its fields and
//! bounds, its step floor and ceiling, its refusals word for word from the
//! card's source, its frame, its worked examples and its record. A child
//! module of `descriptor.rs`, so it uses that file's own builders.

use genesis_kernel::momentum_jwst_dihedral;
use serde_json::Value;

use super::{Advisory, Card, Role, finish, obj, pair_examples_of, scalar, scalar_value, slots, token, vec3_stated, vec3_value};

const DOMAIN: &str = "aerospace";
const CARD: &str = "momentum_jwst_dihedral";

/// What the card is, in one sentence of the list.
const SPECIMEN: &str = "The total angular momentum of the James Webb Space Telescope at Sun-Earth L2, held by its reaction wheels and growing between momentum unloads under the pressure of sunlight on its sunshield, as a function of the observatory's attitude quaternion: the companion of momentum_jwst_srp, with the sunshield's first layer as two plane slopes of 110 m² each meeting along a ridge on the boresight, each falling 4.03 degrees towards the Sun, so that the Sun's roll turns into torque about the boresight; JWST's published optical numbers, the ridge's place and the dihedral fitted to the observatory's own momentum telemetry of 12 May to 14 July 2023. Forecast first and compared with the month of 15 July to 14 August 2023, it met four of five marks: the daily growth of |H| within its allowance on 25 of 28 days, |H| at the end of four runs of 3.9 to 12.4 days between thruster events within 4.9 percent, the hourly growth rate's error 0.272 of its spread, the torque on one attitude 7.59 against 52.65 µN m rms; it missed the daily change of the vector, the mark the two slopes were built to meet, within its allowance on 18 of 28 days. On the next month, 15 August to 14 September 2023, scored blind by the same marks, it met three of five, missing the daily growth of |H| as well, on 22 of 30 days, and the vector on 13 of 30: 7 of 10 marks in its two blind months. Beyond 2 degrees of roll its torque about the boresight is off by -1.6 and +1.0 µN m on average, where momentum_jwst_srp's is off by -28.9 and +27.3 on the same month.";

/// The bound of a quaternion component and of a component of the Sun's
/// direction: a unit vector's component, with the card's tolerance on the norm.
const UNIT_COMPONENT: f64 = 1.0 + momentum_jwst_dihedral::UNIT_NORM_TOLERANCE;

pub(super) fn card() -> Card {
    let observatory = momentum_jwst_dihedral::MomentumJwstDihedral::observatory();
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
                token(momentum_jwst_dihedral::UNIT_NORM_TOLERANCE)
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
                token(momentum_jwst_dihedral::UNIT_NORM_TOLERANCE)
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
                "no bound of its own: the observatory's total angular momentum at the start, three components in ECI (J2000 equatorial), N m s. From the telemetry it is SA_ZMOMMAG times the unit vector SA_ZMOMECIX, SA_ZMOMECIY, SA_ZMOMECIZ; the most before an unload in the record's spent months was 73.8 N m s. The sunlight torque does not depend on it. A momentum so large that 64-bit arithmetic does not carry a step's impulse into it to 1e-9 of the impulse booked plus 1 N m s is refused as AngularMomentumBreach: on the worked load from 10^8 N m s on the first step of any size, from 10^6 N m s within 21 steps from the floor to one minute, from 10^3 N m s after 16,234 steps of the floor and 33,165 of 0.256 s, and at 74 N m s on the 271,866th step of the floor, where each step leaves the same rounding in H; at 74 N m s two million steps of 0.256 s or of one minute close. A run continued through from starts from the upstream's final h_eci_nms".to_string(),
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
                    "above 0 au: the observatory's distance from the Sun. Sunlight falls as its inverse square. At or under the Sun's radius, {} au (695,700 km, the IAU's nominal solar radius, over the astronomical unit of 149,597,870.7 km), the observatory would be inside the Sun, and the step is refused as SunDistanceInsideSun. Near the Sun a slope's balance temperature rises above {} K, 400 degC, the top of NASA's range for Kapton, and the step is refused as SunshieldAboveKaptonLimit: with the Sun on the layer's mean normal that is at about 0.27 au, and the more the Sun grazes a slope the closer to the Sun that limit lies. At L2 the distance is about 1.01 to 1.02 au",
                    token(momentum_jwst_dihedral::SUN_RADIUS_AU),
                    token(momentum_jwst_dihedral::KAPTON_LIMIT_K)
                ),
            ),
        ],
        floor,
        "the ceiling divided by 2^20, under the telemetry's attitude sample of 0.256 s",
        gate,
        "the time in which the Sun's direction, seen from an observatory going round the Sun once a year, turns by 0.01 rad. A step holds the Sun's direction, so over a longer step the torque's error from holding it exceeds about one part in a hundred",
        None,
        "The load, the attitude and the Sun, is that of momentum_jwst_srp number for number, and is held across each step; over a held load the sunlight torque is constant and the step H(t + dt) = H(t) + tau dt is exact. The first layer is two plane slopes meeting along a ridge parallel to the boresight J1, each falling away from the ridge towards the Sun by 0.0704146 rad, 4.03 degrees: the Sun-side normal of the slope on the +J2 side is (0, -sin 4.03 deg, -cos 4.03 deg), and of the other (0, +sin 4.03 deg, -cos 4.03 deg). Each slope is 110 m², balances what it absorbs with what it emits, and pushes at its centroid, 2.433 m from the ridge (a sixth of the sunshield's 14.6 m width) and 0.17 m towards the Sun; the ridge's point at the sunshield's middle lies at -0.0404585, 0.00776958 and -0.171418 m from the centre of mass. With the Sun rolled about the boresight, the slope it faces more squarely pushes harder, so the torque about J1 changes with roll about four times as fast as on the one flat layer of momentum_jwst_srp. The record stepped one minute at a time, with each minute's attitude from the telemetry and the Sun from JPL Horizons. A run covers an interval between thruster events: a momentum unload or a station-keeping burn moves the momentum by thrust, outside the sunlight's torque, so a run ends before one and a new run starts after it from the telemetered momentum. The momentum the telemetry reports while the observatory slews departs from the settled value by up to 3.5 N m s and comes back when it settles; the record compares settled minutes. The two ledgers confirm the arithmetic of one rotation and of two balances: the momentum ledger holds H - H_anchor - J to 1e-9 of the impulse booked plus 1 N m s, where the torque that moves H is turned into ECI by the quaternion's product and the impulse J by the transpose of the telemetry's direction-cosine matrix; the energy ledger holds each slope's emission, taken again from its balance temperature, to 1e-9 of what it absorbs. The Sun behind the plane of either slope is refused as SunBehindSunshield; within the science field of regard both slopes are lit. The Sun's pitch is asin(s_J1 / |s|), its angle towards the boresight, and its roll asin(s_J2 / |(s_J2, s_J3)|), its angle about the boresight, each taken from the layer's mean plane as momentum_jwst_srp takes them, with nothing pulled back. A Sun outside the science field of regard, pitch -45 to +5 degrees and roll -5 to +5 degrees, lands with FLAG_OUTSIDE_FIELD_OF_REGARD.".into(),
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
            mask: momentum_jwst_dihedral::FLAG_OUTSIDE_FIELD_OF_REGARD,
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
            ("residual", "Conservation residual: the larger of the angular-momentum ledger and the energy balance of the worse of the two slopes."),
            ("flags", "FLAG_OUTSIDE_FIELD_OF_REGARD when the Sun was outside the science field of regard on the step."),
            ("proof", "Not written by the step."),
        ]),
    )
}

/// The observatory on 2023-05-13 at 00:00:30 UTC, the day after the unload of
/// 12 May: the telemetered momentum, attitude and the Sun from JPL Horizons,
/// the worked example of momentum_jwst_srp. One step of a minute, and an hour
/// of such steps.
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
        "The JWST engineering database at MAST, open with no token: SA_ZMOMMAG and the unit vector SA_ZMOMECIX, SA_ZMOMECIY, SA_ZMOMECIZ every 4.096 s, the attitude quaternion SA_ZATTEST1 to SA_ZATTEST4 every 0.256 s, the slew status SA_ZSLEWST and the unload status SA_ZMOMUNLST, read as one row a minute; the Sun's direction and distance from JPL Horizons, target -170. Fitted on the UTC days 12 May to 14 July 2023, 1,404 hourly blocks, the two months momentum_jwst_srp is fitted and first compared on: the ridge's point and the dihedral angle, four numbers. Compared with two months not in the fit, 310 files each: 15 July to 14 August and 15 August to 14 September 2023. The layers' dihedral angles (Northrop Grumman Technology Review Journal), the sunshield's width (PASP 135, 058002), its area (Proc. SPIE 9904, 990405), its optical numbers (Origins Space Telescope study report, appendix C) and the solar irradiance (Geophys. Res. Lett. 38, L01706) are published figures; cite gives the full references.".into(),
        "Error measures and targets, those of momentum_jwst_srp: A, the growth of |H| over each day within 0.1 of the observed plus 0.2 N m s on 80 percent of days; B, the change of the vector H over each day within the same allowance on 80 percent of days, the measure the two slopes are built for; C, |H| at the end of each run of 3 days or more between thruster events within 10 percent; D, the hourly growth rate of |H| with an rms error at most 0.5 of its spread; E, the torque on one attitude with an rms error at most 0.4 of the rms torque. The same five on each of the two months, with no constant changed.".into(),
        "On 15 July to 14 August 2023: A 25 of 28 days, 89 percent (target 80); B 18 of 28 days, 64 percent (80), the daily vector error 0.42 N m s rms, momentum_jwst_srp's 0.81; C +0.7 percent after 12.4 days, 50.84 against 50.49 N m s, -1.7 percent after 8.5 days, +4.9 percent after 3.9 days and +1.0 percent after 4.4 days (10); D 0.0377 against 0.1387 N m s per hour, 0.272 (0.5), correlation 0.962; E 7.59 against 52.65 µN m over 177 stretches of one attitude, 0.144 (0.4). Beyond 2 degrees of roll the torque about J1 is off by -1.6 and +1.0 µN m on average, within 2 degrees by 1.1 µN m rms. On the same month and measures momentum_jwst_srp gives A 15 of 28, B 8 of 28, C -16.2 percent after the unload, D 0.384, E 0.314, its torque about J1 off by -28.9 and +27.3 µN m beyond 2 degrees of roll. On 15 August to 14 September 2023: A 22 of 30 days, 73 percent (80), the first of the card's four months, the two fitted ones among them, under 80 percent on A; B 13 of 30 days, 43 percent (80); C -5.5 percent after 29.3 days, 27.17 against 28.75 N m s (10); D 0.0393 against 0.1050 N m s per hour, 0.375 (0.5), correlation 0.928; E 7.30 against 49.03 µN m over 161 stretches of one attitude, 0.149 (0.4). Beyond 2 degrees of roll the torque about J1 is off by -1.5 and +2.8 µN m on average. On the same month the card comes closer than momentum_jwst_srp on every measure. The engine's momentum agrees with the record's own arithmetic to 1.9e-6 N m s at every minute.".into(),
        "Compared on one observatory over two months, 15 July to 14 September 2023, the Sun at a pitch of -44.5 to +4.5 degrees and a roll of -6.3 to +6.4 degrees, |H| from 1.9 to 54.9 N m s, in runs of up to 29.3 days between thruster events; fitted on 12 May to 14 July 2023, the Sun at a pitch of -44.6 to +4.6 degrees and a roll of -5.5 to +6.1 degrees, |H| from 1.6 to 73.8 N m s. Beyond 2 degrees of roll the torque about J3 is off by 11 µN m rms on the first month and 10.4 on the second, as momentum_jwst_srp's is by 12.7 and 12.4. The card's sunlit surfaces are the two slopes of the first layer; the momentum trim flap at the sunshield's aft end has no published area or angle, and fitted beside the four numbers on the fitting months it came out at two solutions, its normal turned 23 to 30 degrees forward with 6 to 7 m² or 74 to 80 degrees aft with 16 to 28 m², neither of which ran the other month closer on every measure. Across each slew of more than 0.5 rad the settled momentum moved by about 0.26 N m s rms that the sunlight does not account for; its cause is not identified. While the observatory slews the telemetry's momentum departs by up to 3.5 N m s and returns when it settles; the comparison uses settled minutes. The thrusters, the wheels' speeds, and attitudes past those pitches and rolls are outside the comparison.".into(),
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
        "SunBehindSunshield" => "The Sun is at or behind the plane of a slope of the sunshield's first layer: that slope's front is not lit, and the light falls between the layers and on the telescope's side, which the card does not hold.",
        "SunshieldAboveKaptonLimit" => "A slope of the first layer has a balance temperature above 673.15 K, 400 °C, the top of the range NASA states for the stability of Kapton.",
        "AngularMomentumBreach" => "The trial step moved the angular momentum by other than the torque it booked.",
        "SpontaneousGeneration" => "The trial step had a slope of the first layer emit more than it absorbed.",
        "FirstLawBreach" => "The trial step had a slope of the first layer emit less than it absorbed.",
        _ => return None,
    })
}
