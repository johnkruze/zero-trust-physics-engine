//! The ends a card declares: the place where the card's own description, or
//! its kernel, says the physical event it models finishes (a first maximum
//! of power, the flown drogue or parachute deployment, a mission's last day).
//! A run that reaches one stops there with the outcome `ended`, the reject's
//! name and a sentence that names the end. Every other refusal stays a refusal: a limit added because
//! a region was never compared with measurement (a reject named for what was
//! not compared, a corridor, a twist past the compared 75 degrees) is never
//! an end.
//!
//! A reject is an end only on the side the run reaches it from inside the
//! card. A reject whose incoming state was already outside, on the other side,
//! stays a refusal: the test on the incoming state below says which side.

use serde_json::Value;

/// How the incoming state shows that a reject is the end.
#[derive(Clone, Copy)]
enum Side {
    /// The reject has one side only.
    Always,
    /// The incoming value of this state field is above this level: the run
    /// came from inside and passed the top.
    IncomingAbove(&'static str, f64),
}

/// One declared end.
pub(crate) struct End {
    pub card: &'static str,
    pub reject: &'static str,
    /// The end in words, as the sentence names it.
    pub end: &'static str,
    side: Side,
}

/// Every declared end. Each comes from the card's description and the reject's
/// meaning in its kernel.
pub(crate) const ENDS: &[End] = &[
    End {
        card: "reactor_spert3",
        reject: "PastFirstMaximum",
        end: "the first maximum of power, where the card's burst ends",
        side: Side::Always,
    },
    End {
        card: "reactor_spert3_fuel",
        reject: "PastFirstMaximum",
        end: "the first maximum of power, where the card's burst ends",
        side: Side::Always,
    },
    End {
        card: "reactor_spert3_ccore",
        reject: "PastPlateTemperatureMaximum",
        end: "the first maximum of the hot-spot plate surface temperature after the first maximum of power, where the card's transient ends",
        side: Side::Always,
    },
    End {
        card: "capsule_stardust",
        reject: "PastFlightDeployment",
        end: "the drogue-deployment condition the radar recorded, altitude under 31,030 m and Mach number under 1.23 at once, where the card's entry ends; the step that first crosses both lands",
        // The incoming state is past both conditions: the reject has one side.
        side: Side::Always,
    },
    End {
        card: "capsule_insight",
        reject: "PastComparedFlight",
        end: "5700 m or Mach 1.46, the lowest state the compared run reached at the flown parachute deployment, 6164.6 m above the datum, where the card's entry ends",
        // Below the floor only: the reject has one side.
        side: Side::Always,
    },
    End {
        card: "generator_mmrtg_f1",
        reject: "MissionTimeOutsideCard",
        end: "mission day 4,087, the last day of the published record and the last the card covers",
        // Halfway through the 4,087 days, in seconds.
        side: Side::IncomingAbove("t_s", 2_043.5 * 86_400.0),
    },
    End {
        card: "crater_ryugu_sci",
        reject: "RadiusAboveFinal",
        end: "the radius of 8.571304329477922 m, where the crater stops growing",
        side: Side::Always,
    },
    End {
        card: "grid_gb",
        reject: "TimeBeyondCard",
        end: "60 s after the start, the end of the 60 seconds after a trip that the card covers",
        side: Side::Always,
    },
    End {
        card: "tire_eve_r19_ice",
        reject: "TravelBeyondTrack",
        end: "6 m along the track, the length rolled in a run",
        // Halfway along the 6 m.
        side: Side::IncomingAbove("distance_m", 3.0),
    },
];

/// The declared end a reject of `card` is, given the specimen's state before
/// the refused step; `None` when the reject is a refusal.
pub(crate) fn declared_end(card: &str, reject: &str, incoming: Option<&Value>) -> Option<&'static End> {
    let end = ENDS.iter().find(|end| end.card == card && end.reject == reject)?;
    let read = |field: &str| -> Option<f64> {
        let text = incoming?.get(format!("{field}_hex"))?.as_str()?;
        u64::from_str_radix(text, 16).ok().map(f64::from_bits)
    };
    let holds = match end.side {
        Side::Always => true,
        Side::IncomingAbove(field, level) => read(field).is_some_and(|value| value > level),
    };
    holds.then_some(end)
}

/// The sentence of a run that stopped at a declared end.
pub(crate) fn sentence(card: &str, end: &End, steps_landed: u64) -> String {
    let sealed = if steps_landed == 0 {
        "no step landed before it, and the run is sealed with none".to_string()
    } else {
        format!("the run is sealed to step {steps_landed}")
    };
    format!(
        "Ended: the run reached the end the {card} card declares ({}): {}. The step past it is not taken, and {sealed}.",
        end.reject, end.end
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_declared_end_names_a_reject_its_card_lists_and_a_published_field() {
        for end in ENDS {
            let pair = crate::descriptor::all()
                .iter()
                .find(|item| item.card == end.card)
                .unwrap_or_else(|| panic!("{} is not a card", end.card));
            assert!(pair.rejects.contains(&end.reject), "{} {}", end.card, end.reject);
            let field = match end.side {
                Side::Always => continue,
                Side::IncomingAbove(field, _) => field,
            };
            assert_eq!(
                crate::state_field_kind(pair.domain, end.card, field),
                Some(Some(crate::QuantityKind::Number)),
                "{} {field}",
                end.card
            );
        }
    }
}
