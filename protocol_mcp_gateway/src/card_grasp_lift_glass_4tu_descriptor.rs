//! What an agent reads of the grasp_lift_glass_4tu card: its fields and
//! bounds, its step floor and ceiling, its refusals word for word from the
//! card's source, its frame, its worked examples and its record. Stamped from
//! the kit; a child module of `descriptor.rs`, so it uses that file's own
//! builders.
//!
//! Every sentence here is served, so each keeps the rules a description is
//! held to (`tests/conformance_support`): the company's specimen is named
//! so, never by what it lacks; a range is written in plain words; no word
//! "check"; and the record states the range the card was compared on and the
//! result there in numbers.

use genesis_kernel::grasp_lift_glass_4tu;
use serde_json::Value;

use super::{Advisory, Card, Role, finish, obj, scalar, scalar_value, slots, token};

const DOMAIN: &str = "materials";
const CARD: &str = "grasp_lift_glass_4tu";

/// What the card is, in one sentence of the list.
const SPECIMEN: &str = "A glass-faced object lifted from a table between two finger pads: a company specimen object, 0.2 kg with two vertical glass faces 60 mm tall, gripped at their middle by two pads that are the 4TU set's human index finger pads on glass, each a Cattaneo-Mindlin contact that sticks, slips at its edge and slides whole, with Coulomb friction 0.858686 and a tangential stiffness of 339.696 N/m fitted to the set's development trials.";

/// The worked lift: the hand accelerates at 2 m/s² for 350 steps of the
/// ceiling, 0.300 s, and holds its speed for 233 more, 0.200 s.
pub(crate) const LIFT_ACCELERATION_M_S2: f64 = 2.0;
pub(crate) const LIFT_ACCELERATION_STEPS: u64 = 350;
pub(crate) const LIFT_HOLD_STEPS: u64 = 233;
/// The worked grip, N: just above the least at which the worked lift does
/// not slip, 1.8816 N (find_flip finds it between 1.881619961722 and
/// 1.881619961723 N).
pub(crate) const LIFT_GRIP_N: f64 = 1.89;

pub(super) fn card() -> Card {
    let specimen = grasp_lift_glass_4tu::GraspLiftGlass4tu::specimen();
    let gate = specimen.gate_dt_s().expect("specimen gate");
    let floor = specimen.floor_dt_s().expect("specimen floor");
    let half_period = core::f64::consts::PI / specimen.contact_pole_rad_s();
    finish(
        DOMAIN,
        CARD,
        SPECIMEN,
        vec![
            scalar(
                "hand_velocity_m_s",
                Role::Phase,
                "m/s",
                None,
                false,
                None,
                false,
                "no bound of its own: the hand's vertical velocity when the grip closes, upward positive. Every run starts with the object at rest on the table, the pads unstressed and the hand at height zero".to_string(),
            ),
            scalar(
                "grip_force_n",
                Role::Load,
                "N",
                Some(0.0),
                true,
                None,
                false,
                format!(
                    "above 0 N: the force with which each pad presses on its face. At 0 N or below the pads do not touch the faces and there is no contact, NoContact. Each pad's friction force is {} times it. The pads' friction and stiffness were compared on natural glass at normal forces from 0.35 to 2.98 N; the least grip that holds the object at rest is {} N",
                    token(grasp_lift_glass_4tu::FRICTION_COEFFICIENT),
                    token(specimen.least_grip_at_rest_n())
                ),
            ),
            scalar(
                "hand_acceleration_m_s2",
                Role::Load,
                "m/s2",
                None,
                false,
                None,
                false,
                "no bound of its own: the hand's vertical acceleration, upward positive, held across the step. The pads move with the hand".to_string(),
            ),
        ],
        floor,
        "the ceiling divided by 2^10",
        gate,
        "one twentieth of a radian of the object's oscillation on both pads' springs, sqrt(2K/m), where the trapezoid rule's error of phase is about two parts in ten thousand",
        Some(half_period),
        format!(
            "The grip force and the hand's acceleration are held across each step. The hand moves exactly; the object moves by the trapezoid rule, m (v' - v) = dt (2 Q_mean - m g), with Q_mean the mean force of one pad over the step's travel along the faces, each of the 32 elements of a pad followed exactly, solved in closed form between the travels at which elements reach their strength. Element j of a pad, outermost first, has the stiffness share u_(j-1) - u_j of K = {} N/m and the strength share 1.5 [g(u_(j-1)) - g(u_j)] of F_s = {} N per newton of grip, u_j = sqrt(1 - j/32), g(u) = u - u^3/3: on a first loading the pad gives Cattaneo-Mindlin's Q = F_s [1 - (1 - d/d*)^1.5], d* = 1.5 F_s / K, and stuck fraction (1 - Q/F_s)^(2/3) at the slice ends, and a load turned back follows Mindlin and Deresiewicz. A grip that falls slides each element above its new strength back to it at once, its spring's energy going to heat. The object rests on the table until the pads' mean force over a step exceeds its weight. The ledgers close at every step: energy, the object's kinetic and potential energy, the springs' and the heat against the hand's work; momentum, the object's against the impulse of the pads, gravity and the table; each to {} of what is in play in it, the sum of the sizes of its terms, and never of less than 2 m g H = {} J and m sqrt(4 g H) = {} N s (energy_scale_j and momentum_scale_n_s). The hand's work is booked in two sums, what it gives the object (hand_work_in_j) and what it takes back (hand_work_out_j), so a height lifted and lowered again stays in play: a hold of ten minutes in the air, a lift at 10 m/s to 5.2 km, and one to 2.5 km and back down onto the table each close both ledgers at every step, to 3.4e-12 at most. A run ends, with the outcome ended, where a pad slides past the end of its face above the table, 0.03 m from the grip, and lets the object go (ObjectSlippedOut), or where the lifted object comes back down onto the table (ObjectLanded). The pads slid down to the lower end of the faces of an object resting on the table are at the table's surface: the fingers reach the table, which is solid, and the step is refused as PadsReachTable. peak_load_ratio and slipped carry the largest load ratio and whether the whole contact slid since the start: the least grip that lifts the object without slip is where slipped turns, which find_flip finds along grip_force_n; under the worked lift it is 1.8816 N, against {} N at rest.",
            token(grasp_lift_glass_4tu::PAD_TANGENTIAL_STIFFNESS_N_M),
            token(grasp_lift_glass_4tu::FRICTION_COEFFICIENT),
            token(grasp_lift_glass_4tu::RESIDUAL_CONSERVATION_LIMIT),
            token(specimen.energy_scale_floor_j()),
            token(specimen.momentum_scale_floor_n_s()),
            token(specimen.least_grip_at_rest_n()),
        ),
        &[
            "NonFiniteInput",
            "NonFiniteState",
            "NonPositiveDt",
            "InvalidSpecimen",
            "NoContact",
            "ObjectBelowTable",
            "PadsReachTable",
            "ObjectSlippedOut",
            "ObjectLanded",
            "FrictionHeatNegative",
            "SpontaneousGeneration",
            "FirstLawBreach",
            "MomentumLedgerBroken",
        ],
        vec![
            Advisory { name: "FLAG_INCIPIENT_SLIP", mask: grasp_lift_glass_4tu::FLAG_INCIPIENT_SLIP, seals: true },
            Advisory { name: "FLAG_GROSS_SLIP", mask: grasp_lift_glass_4tu::FLAG_GROSS_SLIP, seals: true },
            Advisory { name: "FLAG_ON_TABLE", mask: grasp_lift_glass_4tu::FLAG_ON_TABLE, seals: true },
        ],
        slots(&[
            ("t", "Time since the grip closed, in seconds."),
            ("pos[0]", "object_height_m"),
            ("pos[1]", "hand_height_m"),
            ("pos[2]", "pad_travel_m"),
            ("vel[0]", "object_velocity_m_s"),
            ("vel[1]", "hand_velocity_m_s"),
            ("vel[2]", "stuck_fraction"),
            ("force_torque", "Tangential force of one pad on the object, in newtons, upward positive."),
            ("residual", "The larger of the energy and the momentum ledger's residual."),
            (
                "flags",
                "FLAG_INCIPIENT_SLIP, an event: some of each pad's contact slid in the step and some held, the ring at the edge sliding. FLAG_GROSS_SLIP, an event: the whole contact slid along the faces. FLAG_ON_TABLE, a condition: the object rests on the table at the end of the step, not yet lifted. Each seals.",
            ),
            ("proof", "Not written by the step."),
        ]),
    )
}

/// One step at the ceiling from the hand at rest at the worked grip, and the
/// worked lift: the moment a robot cares about, the least grip that lifts
/// the object without slip.
pub(super) fn examples() -> (Value, Value) {
    let specimen = grasp_lift_glass_4tu::GraspLiftGlass4tu::specimen();
    let dt = super::card_dt(specimen.gate_dt_s());
    let phase = obj(&[("hand_velocity_m_s", scalar_value(0.0))]);
    let load = |acceleration: f64| {
        obj(&[("grip_force_n", scalar_value(LIFT_GRIP_N)), ("hand_acceleration_m_s2", scalar_value(acceleration))])
    };
    let mut parameters = phase.as_object().expect("phase").clone();
    for (key, value) in load(LIFT_ACCELERATION_M_S2).as_object().expect("load") {
        parameters.insert(key.clone(), value.clone());
    }
    parameters.insert("dt_s".to_string(), dt.clone());
    let step = obj(&[
        ("domain", Value::String(DOMAIN.to_string())),
        ("card", Value::String(CARD.to_string())),
        ("parameters", Value::Object(parameters)),
    ]);
    let segment = |steps: u64, acceleration: f64| obj(&[("steps", Value::Number(steps.into())), ("load", load(acceleration))]);
    let run = obj(&[
        ("domain", Value::String(DOMAIN.to_string())),
        ("card", Value::String(CARD.to_string())),
        ("initial", phase),
        ("dt_s", dt),
        (
            "segments",
            Value::Array(vec![segment(LIFT_ACCELERATION_STEPS, LIFT_ACCELERATION_M_S2), segment(LIFT_HOLD_STEPS, 0.0)]),
        ),
        ("record_every", Value::Number(53.into())),
    ]);
    (step, run)
}

/// The record, in the five parts every card gives: the object, the data, the
/// pass line, the result and the limits.
pub(super) fn validation(_specimen: &str) -> [String; 5] {
    [
        format!("{CARD}: {SPECIMEN} The object, its mass and its faces are a company specimen; the pads' friction and stiffness are the 4TU set's."),
        "Efficient tactile encoding of object slippage, Sci. Rep. 12, doi 10.1038/s41598-022-16938-1; its data, 4TU.ResearchData doi 10.4121/19329506.v1: twelve index finger pads at about 20 degrees to a glass plate slid at 10 mm/s under about 1.2 N, seven levels of ultrasonic lubrication, four repeats, 336 trials at 300 samples a second. Fitted on S1 whole and S4 to S11 at levels 1, 2, 3, 5, 6 and 7. Compared with the trials not in the fit, chosen by a random seed: S2, S3 and S12 whole and level 4 of S4 to S11.".to_string(),
        "Error measures and targets. L1, the stuck-fraction law: the RMS of the measured stick ratio at 1 mm less the safety margin to the power 2/3 over the incipient slip, at most 0.10. L2, the lateral force at the onset of full slip from the trial's own sliding friction times its normal force: median error at most 10 percent. L3, the card's friction on the fingers not in the fit, on natural glass, through the engine: median error at most 10 percent. L4, the card's plate travel to full slip on them, through the engine: median error at most 18 percent. Record SHA-256 51df1473e92b2182863ac8a30f9520368bce9ae3fd7e30b3dffe4ae3a45410b6.".to_string(),
        "L2: median error 6.7 percent over 116 trials not in the fit (target 10): within a trial the lateral force at the onset of full slip is the trial's sliding ratio times its normal force, so one coefficient serves for static and kinetic friction. L3: the card's friction, 0.858686 times the grip, on the 12 trials on natural glass of three fingers not in the fit, at normal forces from 0.74 to 2.67 N: median error 22.2 percent (target 10), S2 reading 21 to 35 percent low and S12 up to 25 percent high, against 31.5 for a constant force and 8.8 with each finger's own coefficient. L4: the card's plate travel to full slip on the same trials, median error 38.1 percent (target 18), against 33.8 for a constant travel; at 2.37 and 2.47 N the card's travel runs beyond the 9 mm of the trial. L1: the measured stick ratio at 1 mm against the safety margin to the power 2/3, RMS 0.389 over 16,126 samples of the incipient slip (target 0.10), against 0.334 for the stick ratio equal to the margin; the ratio counts the points that moved less than 1 mm and falls below every such law. L3 and L4 are computed through the engine.".to_string(),
        "The pads' friction and stiffness are natural glass's, compared on twelve fingers at normal forces from 0.35 to 2.98 N. Within one finger the friction ratio falls as the grip rises, the force as the grip to the power 0.61 on the development fingers; the card's one coefficient is the fingers' mean. The card's physics is the pads' elastic contact under Coulomb friction: the object's ringing on the pads, at 9.3 Hz, is damped by the slip at the edge of the contacts alone, the finger is rigid behind its pad, the friction has no rate law, and the set shows none, and the object moves straight up and down without tilting or turning.".to_string(),
    ]
}

/// Each refusal's sentence, word for word from the doc comment above the
/// variant in the card's source.
pub(super) fn meaning(name: &str) -> Option<&'static str> {
    Some(match name {
        "NonFiniteInput" => "The grip force, the hand's acceleration or `dt` was NaN or infinite.",
        "NonFiniteState" => "The incoming state, or the trial step, was not finite.",
        "NonPositiveDt" => "`dt` was zero or negative.",
        "InvalidSpecimen" => "A recorded constant is not this specimen's.",
        "NoContact" => "The grip force is zero or below: the pads do not press on the faces, and there is no contact to hold the object.",
        "ObjectBelowTable" => "The object is below the surface of the table, which is solid.",
        "PadsReachTable" => "The pads slide down to the lower end of the faces of the object resting on the table: the fingers reach the table's surface, which is solid.",
        "ObjectSlippedOut" => "A pad slides past the end of its face: the grip lets the object go, the end of the event.",
        "ObjectLanded" => "The lifted object comes back down onto the table, set down or dropped: the end of its lift.",
        "FrictionHeatNegative" => "The trial step's friction heat fell: friction gave energy back.",
        "SpontaneousGeneration" => "The trial step created energy: the object's energy, the springs' and the heat rose above the hand's work.",
        "FirstLawBreach" => "The trial step destroyed energy: the object's energy, the springs' and the heat fell below the hand's work.",
        "MomentumLedgerBroken" => "The trial step's change of the object's momentum is not the impulse of the pads, gravity and the table.",
        _ => return None,
    })
}
