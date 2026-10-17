//! A start that carries a run on arrives with `from`.
//!
//! A chain makes a continuation checkable, so a start whose continuation
//! fields are not those of a fresh start must name the sealed run it
//! continues in `from`, or it is refused as `ContinuationWithoutUpstream`. A
//! fresh start needs no `from`. The fresh values are each card's own, as its
//! description of the field states them: a clock at its start, a ledger with
//! nothing booked, sums of zero, a case-temperature pair equal to the first
//! case temperature, a cell at full charge, and (0.8.1) a ledger anchored at
//! the card's own energy, and angular momentum, of the stated start.
//!
//! The anchors (0.8.1). A limit that depends on an accumulated quantity is
//! not to be evaded by stating that quantity, and a stated ledger is what
//! `from` exists to check. So an anchor is a fresh start's only when it equals
//! the card's own value at the stated state within a rounding allowance
//! worked out from the card's arithmetic, `2 (n + 16) u S`, where `n` is the
//! number of terms the card sums, `S` the sum of their magnitudes at the
//! stated state and `u = 2^-53` the unit rounding of a double: the summation
//! of two honest computations in different orders differs by at most about
//! `2 n u S`, and the 16 covers the few roundings inside each term. Every
//! card with an anchor among its start fields is listed: `resonance`,
//! `ti64_lattice`, `solar_system` (energy and angular momentum), and
//! `tesseract`, whose start states the defect itself and is held to 0. The
//! other anchors of the kernel are not start fields.
//!
//! A start sent with `from` is exempt only for the fields `from` carries: a
//! continuation carries every start field, a coupling only the fields it
//! names, so the fields a coupling does not hand are held to the rule.
//!
//! Cards whose fresh values exist but where a value other than fresh is also
//! an honest start the card cannot tell from a continuation (wet antennas
//! from earlier rain, a cell rested at partial charge, a known quiet period,
//! a tire part way down the track) keep taking the values stated, unchecked,
//! and are not listed: `tire_eve_r19_ice` (`distance_m`, `slip_held_s`),
//! `los_jwst_fine_guide` (`quiet_before_start_s`), `cell_18650pf` (`soc`, a
//! rested cell below full charge), `generator_mmrtg_f1` and its companion
//! `generator_mmrtg_seebeck` (`mission_time_s`, the mission clock itself
//! and the card's whole state),
//! `link_gothenburg_10010`'s `antenna_water_db`, and `cell_2170_m80`'s
//! `charge_removed_ah`. `fingerpad_p02_glass` refuses every start but zero
//! twist on its own.

use serde_json::{Map, Value};

use crate::live::Live;

/// The refusal's name.
pub(crate) const NAME: &str = "ContinuationWithoutUpstream";

/// The unit rounding of a double, 2^-53.
const UNIT_ROUNDING: f64 = f64::EPSILON / 2.0;

/// The roundings allowed inside each term, beside one per term summed.
const ROUNDINGS_PER_TERM: f64 = 16.0;

/// What a fresh start states in one field.
#[derive(Clone, Copy)]
enum Fresh {
    /// The number 0.
    Zero,
    /// A 3-vector of zeros.
    ZeroVector,
    /// The first segment's value of the named load field.
    FirstLoad(&'static str),
    /// The card's own value of a ledger quantity at the stated start, within
    /// the rounding allowance of `terms` terms summed; `what` names the
    /// quantity and its terms in words.
    Own { terms: u32, what: &'static str },
}

struct Field {
    name: &'static str,
    fresh: Fresh,
}

struct Rule {
    card: &'static str,
    fields: &'static [Field],
    /// What a fresh start of this card states, in words.
    fresh_words: &'static str,
}

const RESONANCE_ENERGY: Fresh = Fresh::Own {
    terms: 2,
    what: "the mechanical energy of the stated displacement and velocity, 0.5 m v^2 + 0.5 k x^2 (2 terms)",
};
const TI64_ENERGY: Fresh = Fresh::Own {
    terms: 12,
    what: "the mechanical energy of the stated position and velocity under start_force_n and start_temperature_k, 1/2 m |v|^2 plus, over the three ligaments, 1/2 k (x_i - alpha (T - 293.15 K) L)^2 + beta/4 x_i^4 - F_i x_i (12 terms)",
};
const SOLAR_ENERGY: Fresh = Fresh::Own {
    terms: 91,
    what: "Newton's energy of the stated positions and velocities times G, 1/2 sum mu_i v_i.v_i - sum_{i<j} mu_i mu_j / rho_ij (91 terms)",
};
const SOLAR_TURNING: Fresh = Fresh::Own {
    terms: 26,
    what: "the angular momentum of the stated positions and velocities times G, sum mu_i r_i x v_i, each component (26 terms; S is sum mu_i |r_i| |v_i|)",
};

const RULES: &[Rule] = &[
    Rule {
        card: "ti64_lattice",
        fields: &[
            Field { name: "start_t_s", fresh: Fresh::Zero },
            Field { name: "start_boundary_work_j", fresh: Fresh::Zero },
            Field { name: "start_energy_anchor_j", fresh: TI64_ENERGY },
        ],
        fresh_words: "start_t_s 0, start_boundary_work_j 0 and start_energy_anchor_j its own mechanical energy",
    },
    Rule {
        card: "resonance",
        fields: &[
            Field { name: "t_s", fresh: Fresh::Zero },
            Field { name: "boundary_work_j", fresh: Fresh::Zero },
            Field { name: "energy_anchor_j", fresh: RESONANCE_ENERGY },
        ],
        fresh_words: "t_s 0, boundary_work_j 0 and energy_anchor_j the mechanical energy of its own displacement and velocity",
    },
    Rule {
        card: "tesseract",
        fields: &[Field { name: "energy_defect_j", fresh: Fresh::Zero }],
        fresh_words: "energy_defect_j 0",
    },
    Rule {
        card: "satellite_lageos1",
        fields: &[Field { name: "forecast_elapsed_s", fresh: Fresh::Zero }],
        fresh_words: "forecast_elapsed_s 0, a state turned from a published orbit",
    },
    Rule {
        card: "cell_2170_model3",
        fields: &[
            Field { name: "case_temperature_least_c", fresh: Fresh::FirstLoad("case_temperature_c") },
            Field { name: "case_temperature_most_c", fresh: Fresh::FirstLoad("case_temperature_c") },
        ],
        fresh_words: "case_temperature_least_c and case_temperature_most_c both equal to its first case_temperature_c",
    },
    Rule {
        card: "tokamak_jet_dte1",
        fields: &[Field { name: "elapsed_time_s", fresh: Fresh::Zero }],
        fresh_words: "elapsed_time_s 0, a new pulse",
    },
    Rule {
        card: "accumulator_sbo450",
        fields: &[
            Field { name: "work_moved_j", fresh: Fresh::Zero },
            Field { name: "heat_moved_j", fresh: Fresh::Zero },
        ],
        fresh_words: "work_moved_j 0 and heat_moved_j 0, gas that has not moved",
    },
    Rule {
        card: "solar_system",
        fields: &[
            Field { name: "booked_work", fresh: Fresh::Zero },
            Field { name: "booked_torque", fresh: Fresh::ZeroVector },
            Field { name: "energy_anchor", fresh: SOLAR_ENERGY },
            Field { name: "angular_momentum_anchor", fresh: SOLAR_TURNING },
        ],
        fresh_words: "booked_work 0, booked_torque [0, 0, 0], and energy_anchor and angular_momentum_anchor its own E and L",
    },
];

fn rule(card: &str) -> Option<&'static Rule> {
    RULES.iter().find(|rule| rule.card == card)
}

/// The cards this rule holds, for the map and the tests.
#[allow(dead_code)]
pub(crate) fn cards() -> impl Iterator<Item = &'static str> {
    RULES.iter().map(|rule| rule.card)
}

/// Which start fields a request's `from` carries.
#[derive(Clone, Debug)]
pub(crate) enum Carried {
    /// No `from`: a fresh start.
    Nothing,
    /// A continuation of the same card: every start field is carried from
    /// the upstream's sealed state, bit for bit.
    Every,
    /// A coupling: only the named fields are handed on.
    Only(Vec<&'static str>),
}

impl Carried {
    fn carries(&self, field: &str) -> bool {
        match self {
            Carried::Nothing => false,
            Carried::Every => true,
            Carried::Only(fields) => fields.contains(&field),
        }
    }
}

/// The relative allowance of an anchor summed from `terms` terms, as a
/// fraction of the sum of the terms' magnitudes.
fn relative_allowance(terms: u32) -> f64 {
    2.0 * (f64::from(terms) + ROUNDINGS_PER_TERM) * UNIT_ROUNDING
}

fn allowance_words(terms: u32, what: &str) -> String {
    format!(
        "the card's own value, {what}, within 2 (n + 16) u S, with n = {terms} the terms the card sums, S the sum of their magnitudes at the stated state and u = 2^-53 the unit rounding of a double: {:.3e} of S, the most two honest double-precision computations of it can differ by",
        relative_allowance(terms)
    )
}

/// The sentence added to the description of a continuation field, or `None`
/// for a field that is not one.
pub(crate) fn field_sentence(card: &str, field: &str) -> Option<String> {
    let rule = rule(card)?;
    let one = rule.fields.iter().find(|item| item.name == field)?;
    let fresh = match one.fresh {
        Fresh::Zero => "0".to_string(),
        Fresh::ZeroVector => "[0, 0, 0]".to_string(),
        Fresh::FirstLoad(load) => format!("the first segment's {load} (in a single step, its {load})"),
        Fresh::Own { terms, what } => allowance_words(terms, what),
    };
    let carries = match one.fresh {
        Fresh::Own { .. } => "carries a stated ledger defect on, which only the sealed run it came from can show",
        _ => "carries a run on",
    };
    if crate::chain::not_continued(card).is_some() {
        return Some(format!(
            " At the door: a fresh start of this card states {}; a start whose {field} is not {fresh} {carries}, and a continuation through from is not served on this card (ContinuationNotOnCard), so such a start is refused as {NAME}.",
            rule.fresh_words
        ));
    }
    Some(format!(
        " At the door: a fresh start of this card states {}; a start whose {field} is not {fresh} {carries}, and is taken only with from, naming the sealed run it continues, whose sealed values it must carry bit for bit; without from it is refused as {NAME}.",
        rule.fresh_words
    ))
}

fn number(value: Option<&Value>) -> Option<f64> {
    value.and_then(Value::as_f64)
}

fn how(card: &str) -> &'static str {
    if crate::chain::not_continued(card).is_some() {
        "a start that carries a run on must arrive with from, and a continuation through from is not served on this card (ContinuationNotOnCard), so only a fresh start is taken"
    } else {
        "a start that carries a run on must arrive with from, naming the sealed run it continues, so that what it carries is compared with that run's seal"
    }
}

/// A start held to the card's fresh values, the anchors aside (those are
/// held by [`check_anchors`] once the start is opened). `start` holds the
/// start fields (a run's `initial`, a step's `parameters`), `first_load` the
/// load of the first step (a run's first segment's `load`, a step's
/// `parameters`), and `section` the name the caller wrote them under. A
/// field `carried` names is the upstream's and is not held here. The error
/// is the whole sentence.
pub(crate) fn check(
    card: &str,
    start: &Map<String, Value>,
    first_load: &Map<String, Value>,
    section: &str,
    load_section: &str,
    carried: &Carried,
) -> Result<(), String> {
    let Some(rule) = rule(card) else {
        return Ok(());
    };
    for field in rule.fields {
        if carried.carries(field.name) {
            continue;
        }
        let Some(given) = start.get(field.name) else {
            // A missing start field is the card's own refusal.
            continue;
        };
        let (holds, fresh) = match field.fresh {
            Fresh::Zero => (number(Some(given)) == Some(0.0), "0".to_string()),
            Fresh::ZeroVector => (
                given
                    .as_array()
                    .is_some_and(|items| items.len() == 3 && items.iter().all(|item| item.as_f64() == Some(0.0))),
                "[0, 0, 0]".to_string(),
            ),
            Fresh::FirstLoad(load) => {
                let first = number(first_load.get(load));
                match first {
                    // A missing load is the card's own refusal.
                    None => continue,
                    Some(first) => (
                        number(Some(given)) == Some(first),
                        format!("{} ('{load_section}.{load}')", Value::from(first)),
                    ),
                }
            }
            // Held once the start is opened.
            Fresh::Own { .. } => continue,
        };
        if !holds {
            return Err(format!(
                "Physical Violation: '{section}.{}' is {given}, where a fresh start of the {card} card states {fresh}; {}. A fresh start states {} ({NAME}).",
                field.name,
                how(card),
                rule.fresh_words
            ));
        }
    }
    Ok(())
}

/// Whether the card has an anchor held to its own value at the start, so
/// that a step must be opened to hold it.
pub(crate) fn has_anchors(card: &str) -> bool {
    rule(card).is_some_and(|rule| rule.fields.iter().any(|field| matches!(field.fresh, Fresh::Own { .. })))
}

/// One anchor as stated, beside the card's own value of it at the stated
/// start and the sum of the magnitudes of the terms that value is summed from.
struct Anchor {
    field: &'static str,
    component: Option<usize>,
    stated: f64,
    own: f64,
    magnitudes: f64,
}

fn anchors_of(live: &Live) -> Vec<Anchor> {
    match live {
        Live::Resonance(call) => vec![Anchor {
            field: "energy_anchor_j",
            component: None,
            stated: call.state.energy_anchor_j,
            own: call.state.mechanical_energy_j,
            // Both terms are at least 0.
            magnitudes: call.state.kinetic_j.abs() + call.state.potential_j.abs(),
        }],
        Live::Ti64(call) => {
            let cell = &call.cell;
            let state = &call.state;
            let k = cell.stiffness_n_m();
            let v = state.velocity_m_s;
            let mut magnitudes = (0.5 * cell.mass_kg() * (v[0] * v[0] + v[1] * v[1] + v[2] * v[2])).abs();
            for i in 0..3 {
                let x = state.position_m[i];
                let stretch = x
                    - genesis_kernel::ti64_lattice::thermal_extension_m(cell.length_m[i], state.temperature_k);
                magnitudes += (0.5 * k[i] * stretch * stretch).abs()
                    + (0.25 * cell.hardening_n_m3[i] * x * x * x * x).abs()
                    + (state.applied_force_n[i] * x).abs();
            }
            vec![Anchor {
                field: "start_energy_anchor_j",
                component: None,
                stated: state.energy_anchor_j,
                own: state.energy_j,
                magnitudes,
            }]
        }
        Live::SolarSystem(call) => {
            let mu = &call.system.gm_m3_s2;
            let state = &call.state;
            let r = &state.position_m;
            let v = &state.velocity_m_s;
            let norm = |a: [f64; 3]| (a[0] * a[0] + a[1] * a[1] + a[2] * a[2]).sqrt();
            let mut energy = 0.0;
            let mut turning = 0.0;
            for i in 0..mu.len() {
                let speed = norm(v[i]);
                energy += (0.5 * mu[i] * speed * speed).abs();
                turning += (mu[i] * norm(r[i]) * speed).abs();
                for j in i + 1..mu.len() {
                    let apart = norm([r[j][0] - r[i][0], r[j][1] - r[i][1], r[j][2] - r[i][2]]);
                    energy += (mu[i] * mu[j] / apart).abs();
                }
            }
            let mut anchors = vec![Anchor {
                field: "energy_anchor",
                component: None,
                stated: state.energy_anchor,
                own: state.energy,
                magnitudes: energy,
            }];
            for axis in 0..3 {
                anchors.push(Anchor {
                    field: "angular_momentum_anchor",
                    component: Some(axis),
                    stated: state.angular_momentum_anchor[axis],
                    own: state.angular_momentum[axis],
                    magnitudes: turning,
                });
            }
            anchors
        }
        _ => Vec::new(),
    }
}

/// A start's anchors held to the card's own values at the stated start,
/// within the rounding allowance. `live` is the start opened, with no step
/// taken. An anchor `carried` names is the upstream's and is not held here.
pub(crate) fn check_anchors(card: &str, live: &Live, section: &str, carried: &Carried) -> Result<(), String> {
    let Some(rule) = rule(card) else {
        return Ok(());
    };
    for anchor in anchors_of(live) {
        if carried.carries(anchor.field) {
            continue;
        }
        let Some(Fresh::Own { terms, what }) =
            rule.fields.iter().find(|field| field.name == anchor.field).map(|field| field.fresh)
        else {
            continue;
        };
        let allowance = relative_allowance(terms) * anchor.magnitudes;
        let off = (anchor.stated - anchor.own).abs();
        if off <= allowance {
            continue;
        }
        let place = match anchor.component {
            None => format!("'{section}.{}' is {}", anchor.field, Value::from(anchor.stated)),
            Some(axis) => format!(
                "component {axis} of '{section}.{}' is {}",
                anchor.field,
                Value::from(anchor.stated)
            ),
        };
        let own = match anchor.component {
            None => "the card's own value".to_string(),
            Some(axis) => format!("component {axis} of the card's own value"),
        };
        return Err(format!(
            "Physical Violation: {place}, and {own} at the stated start is {}, off by {:e} where the allowance is {:e}: a fresh start of the {card} card states {}. An anchor that is not the start's own value carries a stated ledger defect; {}. A fresh start states {} ({NAME}).",
            Value::from(anchor.own),
            off,
            allowance,
            allowance_words(terms, what),
            how(card),
            rule.fresh_words
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn map(value: Value) -> Map<String, Value> {
        value.as_object().cloned().unwrap()
    }

    #[test]
    fn every_listed_field_is_a_start_field_of_its_card() {
        for rule in RULES {
            let pair = crate::descriptor::all()
                .iter()
                .find(|card| card.card == rule.card)
                .unwrap_or_else(|| panic!("{} is a card", rule.card));
            for field in rule.fields {
                assert!(pair.phase_names.contains(&field.name), "{} {}", rule.card, field.name);
                if let Fresh::FirstLoad(load) = field.fresh {
                    assert!(pair.load_names.contains(&load), "{} {load}", rule.card);
                }
                assert!(field_sentence(rule.card, field.name).unwrap().contains(NAME));
            }
        }
    }

    #[test]
    fn a_fresh_start_passes_and_each_continuation_field_is_named() {
        assert!(check("resonance", &map(json!({"t_s": 0.0, "boundary_work_j": 0.0})), &Map::new(), "initial", "load", &Carried::Nothing).is_ok());
        let refused = check("resonance", &map(json!({"t_s": 0.5, "boundary_work_j": 0.0})), &Map::new(), "initial", "load", &Carried::Nothing).unwrap_err();
        assert!(refused.contains("'initial.t_s' is 0.5") && refused.ends_with("(ContinuationWithoutUpstream)."), "{refused}");
        let model3 = map(json!({"case_temperature_least_c": 24.0, "case_temperature_most_c": 25.0}));
        let refused = check("cell_2170_model3", &model3, &map(json!({"case_temperature_c": 25.0})), "parameters", "parameters", &Carried::Nothing).unwrap_err();
        assert!(refused.contains("'parameters.case_temperature_least_c' is 24.0"), "{refused}");
        assert!(check("solar_system", &map(json!({"booked_work": 0.0, "booked_torque": [0.0, 0.0, 0.0]})), &Map::new(), "initial", "load", &Carried::Nothing).is_ok());
        assert!(check("solar_system", &map(json!({"booked_work": 0.0, "booked_torque": [0.0, 1.0, 0.0]})), &Map::new(), "initial", "load", &Carried::Nothing).is_err());
        // A card with no continuation field takes every start.
        assert!(check("thermal", &map(json!({"temperature_k": 300.0})), &Map::new(), "initial", "load", &Carried::Nothing).is_ok());
    }

    #[test]
    fn every_anchor_among_the_start_fields_is_held() {
        // Every start field that anchors a ledger, or states its defect, is
        // held to a fresh start's value here.
        for pair in crate::descriptor::all() {
            for name in &pair.phase_names {
                if name.contains("anchor") || name.contains("defect") {
                    let held = rule(pair.card).is_some_and(|rule| rule.fields.iter().any(|field| field.name == *name));
                    assert!(held, "{} {name} is a start field with an anchor and is not held", pair.card);
                }
            }
        }
    }

    #[test]
    fn a_coupling_is_exempt_only_for_the_fields_it_hands() {
        let start = map(json!({"t_s": 0.5, "boundary_work_j": 1.0}));
        let only_clock = Carried::Only(vec!["t_s"]);
        let refused = check("resonance", &start, &Map::new(), "initial", "load", &only_clock).unwrap_err();
        assert!(refused.contains("'initial.boundary_work_j' is 1.0"), "{refused}");
        let both = Carried::Only(vec!["t_s", "boundary_work_j"]);
        assert!(check("resonance", &start, &Map::new(), "initial", "load", &both).is_ok());
        assert!(check("resonance", &start, &Map::new(), "initial", "load", &Carried::Every).is_ok());
    }

    #[test]
    fn the_allowance_is_the_rounding_of_the_cards_sum() {
        assert_eq!(relative_allowance(2), 36.0 * f64::EPSILON / 2.0);
        // Far below each card's conservation limit: 1e-6 of a scale for
        // resonance and ti64_lattice, 1e-12 of a scale for solar_system.
        assert!(relative_allowance(91) < 3.0e-14);
        assert!(field_sentence("resonance", "energy_anchor_j").unwrap().contains("3.997e-15 of S"));
    }
}
