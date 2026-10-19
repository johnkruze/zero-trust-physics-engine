//! What a solver's flag reports beside it, in a solution: the limit, the
//! value, and the extent past it in the limit's own unit and as a fraction
//! of it (NASA-STD-7009B [M&S 26]; `src/exceedance.rs` for a card's frame).
//!
//! A solver has no frame and no flags word: its flags are numbers of its
//! `result`, 1 where the question lies past what was compared. A solution
//! whose result carries a flag at 1 carries `exceedances` beside `result`,
//! one record for each limit the question is past, named by the flag's key.
//! Each record is read from the request's own parameters, which the
//! solution seal covers, and the solver's own constants: so the records are
//! fixed by the sealed request, the receipt covers them, and the long form
//! computes them again. A solution with no flag at 1 has none, and its bytes
//! are those of 0.14.3.

use genesis_kernel::exceedance::{Exceedance, ExceedanceKind, Side};
use genesis_solvers::{culture_trichoderma_atp as culture, fuel_rod_triga_jsi as rod, stress_plate_hole_s235_elastic as elastic};
use serde_json::{Map, Value, json};

/// The flags of each solver that carries any: the keys of its `result`.
pub(crate) const FLAGS: &[(&str, &[&str])] = &[
    ("stress_plate_hole_s235_elastic", &["past_compared_traction"]),
    (
        "fuel_rod_triga_jsi",
        &[
            "start_temperature_outside_compared",
            "core_energy_outside_compared",
            "core_power_outside_compared",
            "history_outside_compared",
        ],
    ),
    (
        "culture_trichoderma_atp",
        &["ph_outside_compared", "yeast_extract_outside_compared", "feed_glucose_outside_compared", "record_past_compared"],
    ),
    ("ablator_slab", &ENVELOPES),
];

/// The three envelopes of `ablator_slab`, 1 inside: a record for each at 0.
const ENVELOPES: [&str; 3] = ["inside_arc_jet_comparison", "inside_stardust_comparison", "inside_thermocouple_block_comparison"];

const fn kind(flag: &'static str, quantity: &'static str, unit: &'static str, side: Side, limit: f64, words: &'static str) -> ExceedanceKind {
    ExceedanceKind::fixed(flag, 0, quantity, unit, side, limit, words)
}

const TRACTION: ExceedanceKind = kind(
    "past_compared_traction",
    "traction_mpa",
    "MPa",
    Side::Above,
    elastic::COMPARED_TRACTION_MPA,
    "the one traction the record compares, 106.26 MPa",
);

const ROD_START_WORDS: &str = "the start temperatures of the histories compared, 293.14 to 305.53 K";
const ROD_START_LOW: ExceedanceKind = kind(
    "start_temperature_outside_compared",
    "start_temperature_k",
    "K",
    Side::Below,
    rod::START_TEMPERATURE_COMPARED_MIN_K,
    ROD_START_WORDS,
);
const ROD_START_HIGH: ExceedanceKind = kind(
    "start_temperature_outside_compared",
    "start_temperature_k",
    "K",
    Side::Above,
    rod::START_TEMPERATURE_COMPARED_MAX_K,
    ROD_START_WORDS,
);
const ROD_ENERGY_WORDS: &str = "the core energy released by the end of the histories compared, 6.85 to 19.41 MW s";
const ROD_ENERGY_LOW: ExceedanceKind = kind(
    "core_energy_outside_compared",
    "core_energy_released_mws_at_end",
    "MW s",
    Side::Below,
    rod::CORE_ENERGY_COMPARED_MIN_MWS,
    ROD_ENERGY_WORDS,
);
const ROD_ENERGY_HIGH: ExceedanceKind = kind(
    "core_energy_outside_compared",
    "core_energy_released_mws_at_end",
    "MW s",
    Side::Above,
    rod::CORE_ENERGY_COMPARED_MAX_MWS,
    ROD_ENERGY_WORDS,
);
const ROD_POWER: ExceedanceKind = kind(
    "core_power_outside_compared",
    "largest_interval_core_power_mw",
    "MW",
    Side::Above,
    rod::CORE_POWER_COMPARED_MAX_MW,
    "the highest core power over an interval of the histories compared, 994 MW",
);
const ROD_INTERVAL_WORDS: &str = "the intervals of the histories compared, 1e-5 s to 10.1 ms";
const ROD_INTERVAL_LOW: ExceedanceKind = kind(
    "history_outside_compared",
    "smallest_interval_s",
    "s",
    Side::Below,
    rod::HISTORY_INTERVAL_COMPARED_MIN_S,
    ROD_INTERVAL_WORDS,
);
const ROD_INTERVAL_HIGH: ExceedanceKind = kind(
    "history_outside_compared",
    "largest_interval_s",
    "s",
    Side::Above,
    rod::HISTORY_INTERVAL_COMPARED_MAX_S,
    ROD_INTERVAL_WORDS,
);
const ROD_SPAN: ExceedanceKind = kind(
    "history_outside_compared",
    "history_end_s",
    "s",
    Side::Above,
    rod::HISTORY_SPAN_COMPARED_MAX_S,
    "the longest history compared, 14 s",
);

const fn culture_kinds(flag: &'static str, quantity: &'static str, unit: &'static str, range: (f64, f64), words: &'static str) -> [ExceedanceKind; 2] {
    [kind(flag, quantity, unit, Side::Below, range.0, words), kind(flag, quantity, unit, Side::Above, range.1, words)]
}
const CULTURE_PH: [ExceedanceKind; 2] =
    culture_kinds("ph_outside_compared", "ph", "1", culture::COMPARED_PH, "the pH range compared with measurement, 3.0 to 4.0");
const CULTURE_YEAST: [ExceedanceKind; 2] = culture_kinds(
    "yeast_extract_outside_compared",
    "yeast_extract_g_per_l",
    "g/L",
    culture::COMPARED_YEAST_EXTRACT_G_PER_L,
    "the yeast extract of the twelve fitted cultures, 0 to 2 g/L",
);
const CULTURE_FEED: [ExceedanceKind; 2] = culture_kinds(
    "feed_glucose_outside_compared",
    "feed_glucose_g_per_l",
    "g/L",
    culture::COMPARED_FEED_GLUCOSE_G_PER_L,
    "the feed glucose of the twelve fitted cultures, 10.077 to 52.101 g/L",
);
const CULTURE_RECORD: ExceedanceKind = kind(
    "record_past_compared",
    "record_end_h",
    "h",
    Side::Above,
    culture::COMPARED_RECORD_END_H,
    "the end of the twelve fitted cultures' records, 312 h",
);

/// Every limit a solver's flags can mark, for `describe_solver`.
pub(crate) fn kinds(solver: &str) -> Vec<ExceedanceKind> {
    match solver {
        "stress_plate_hole_s235_elastic" => vec![TRACTION],
        "fuel_rod_triga_jsi" => vec![
            ROD_START_LOW,
            ROD_START_HIGH,
            ROD_ENERGY_LOW,
            ROD_ENERGY_HIGH,
            ROD_POWER,
            ROD_INTERVAL_LOW,
            ROD_INTERVAL_HIGH,
            ROD_SPAN,
        ],
        "culture_trichoderma_atp" => {
            let mut all = Vec::new();
            all.extend(CULTURE_PH);
            all.extend(CULTURE_YEAST);
            all.extend(CULTURE_FEED);
            all.push(CULTURE_RECORD);
            all
        }
        "ablator_slab" => ENVELOPES
            .iter()
            .map(|flag| {
                ExceedanceKind::varying(
                    flag,
                    0,
                    "the first reason the question is outside the envelope",
                    "",
                    Side::OtherThan,
                    "the envelope of the compared cases: the first number of the question found outside it, with its quantity, unit, side and limit, or a reason with no number (a stream not of air, a held face, TACOT 3.0); the reason's words are in reason",
                )
            })
            .collect(),
        _ => Vec::new(),
    }
}

/// The records of `ablator_slab`: for each envelope the question lies
/// outside, the first reason, read by the solver's own comparisons.
fn envelope_records(request: &Value, result: &Value) -> Vec<Value> {
    let Some(misses) = crate::solution::given_of(request).and_then(|given| crate::solver_ablator_slab::envelope_misses(&given)) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for (flag, miss) in ENVELOPES.iter().zip(misses) {
        let inside = result.get(*flag).and_then(Value::as_f64) == Some(1.0);
        assert!(
            !genesis_kernel::exceedance::CHECKED || inside == miss.is_none(),
            "ablator_slab: {flag} and its miss {miss:?}"
        );
        let Some(miss) = miss.filter(|_| !inside) else {
            continue;
        };
        let mut object = Map::new();
        object.insert("flag".into(), json!(flag));
        match miss.past {
            Some(past) => {
                object.insert("quantity".into(), json!(past.quantity));
                object.insert("unit".into(), json!(past.unit));
                object.insert("side".into(), json!(past.side.word()));
                let kind = ExceedanceKind::fixed(flag, 0, past.quantity, past.unit, past.side, past.limit, "");
                let record = Exceedance::at(kind, past.limit, past.value);
                put(&mut object, "limit", record.limit());
                put(&mut object, "value", record.value());
                put(&mut object, "extent", record.extent());
                put(&mut object, "fraction", record.fraction());
            }
            None => {
                object.insert("quantity".into(), Value::Null);
                object.insert("unit".into(), Value::Null);
                object.insert("side".into(), json!(Side::State.word()));
                for key in ["limit", "value", "extent", "fraction"] {
                    put(&mut object, key, None);
                }
            }
        }
        object.insert("reason".into(), json!(miss.words));
        out.push(Value::Object(object));
    }
    out
}

fn number(parameters: &Value, key: &str) -> Option<f64> {
    parameters.get(key).and_then(Value::as_f64)
}

fn numbers(parameters: &Value, key: &str) -> Vec<f64> {
    parameters.get(key).and_then(Value::as_array).map(|items| items.iter().filter_map(Value::as_f64).collect()).unwrap_or_default()
}

/// The records of a range: below its low end or above its high end.
fn range(records: &mut Vec<Exceedance>, kinds: [ExceedanceKind; 2], value: f64) {
    let [low, high] = kinds;
    if value < low.limit.unwrap_or(f64::NEG_INFINITY) {
        records.push(Exceedance::fixed(low, value));
    } else if value > high.limit.unwrap_or(f64::INFINITY) {
        records.push(Exceedance::fixed(high, value));
    }
}

/// The records of a question, from its parameters, by the comparisons the
/// solver writes its flags by.
fn records(solver: &str, parameters: &Value) -> Vec<Exceedance> {
    let mut records = Vec::new();
    match solver {
        "stress_plate_hole_s235_elastic" => {
            if let Some(traction) = number(parameters, "traction_mpa") {
                if traction > elastic::COMPARED_TRACTION_MPA {
                    records.push(Exceedance::fixed(TRACTION, traction));
                }
            }
        }
        "fuel_rod_triga_jsi" => {
            if let Some(start) = number(parameters, "start_temperature_k") {
                range(&mut records, [ROD_START_LOW, ROD_START_HIGH], start);
            }
            let times = numbers(parameters, "time_s");
            let energies = numbers(parameters, "core_energy_released_mws");
            // As `fuel_rod_triga_jsi::outside_compared` reads the history.
            let (mut before_t, mut before_e) = (0.0_f64, 0.0_f64);
            let mut power: Option<f64> = None;
            let mut smallest: Option<f64> = None;
            let mut largest: Option<f64> = None;
            let mut end: Option<f64> = None;
            for (&t, &e) in times.iter().zip(&energies) {
                let interval = t - before_t;
                if !(rod::HISTORY_INTERVAL_COMPARED_MIN_S..=rod::HISTORY_INTERVAL_COMPARED_MAX_S).contains(&interval) {
                    if interval < rod::HISTORY_INTERVAL_COMPARED_MIN_S {
                        smallest = Some(smallest.map_or(interval, |held: f64| if interval < held { interval } else { held }));
                    } else {
                        largest = Some(largest.map_or(interval, |held: f64| if interval > held { interval } else { held }));
                    }
                }
                if t > rod::HISTORY_SPAN_COMPARED_MAX_S {
                    end = Some(t);
                }
                let step_power = (e - before_e) / interval;
                if step_power > rod::CORE_POWER_COMPARED_MAX_MW {
                    power = Some(power.map_or(step_power, |held: f64| if step_power > held { step_power } else { held }));
                }
                before_t = t;
                before_e = e;
            }
            range(&mut records, [ROD_ENERGY_LOW, ROD_ENERGY_HIGH], before_e);
            if let Some(power) = power {
                records.push(Exceedance::fixed(ROD_POWER, power));
            }
            if let Some(smallest) = smallest {
                records.push(Exceedance::fixed(ROD_INTERVAL_LOW, smallest));
            }
            if let Some(largest) = largest {
                records.push(Exceedance::fixed(ROD_INTERVAL_HIGH, largest));
            }
            if let Some(end) = end {
                records.push(Exceedance::fixed(ROD_SPAN, end));
            }
        }
        "culture_trichoderma_atp" => {
            if let Some(ph) = number(parameters, "ph") {
                range(&mut records, CULTURE_PH, ph);
            }
            if let Some(yeast) = number(parameters, "yeast_extract_g_per_l") {
                range(&mut records, CULTURE_YEAST, yeast);
            }
            if let Some(feed) = number(parameters, "feed_glucose_g_per_l") {
                range(&mut records, CULTURE_FEED, feed);
            }
            let end_h = numbers(parameters, "segment_end_h").last().copied().unwrap_or(0.0);
            if end_h > culture::COMPARED_RECORD_END_H {
                records.push(Exceedance::fixed(CULTURE_RECORD, end_h));
            }
        }
        _ => {}
    }
    records
}

fn put(object: &mut Map<String, Value>, key: &str, value: Option<f64>) {
    match value.filter(|value| value.is_finite()) {
        Some(value) => {
            object.insert(key.to_string(), serde_json::Number::from_f64(value).map_or(Value::Null, Value::Number));
            object.insert(format!("{key}_hex"), Value::String(format!("{:016x}", value.to_bits())));
        }
        None => {
            object.insert(key.to_string(), Value::Null);
            object.insert(format!("{key}_hex"), Value::Null);
        }
    }
}

fn record_json(record: &Exceedance) -> Value {
    let mut object = Map::new();
    object.insert("flag".into(), json!(record.kind.flag));
    object.insert("quantity".into(), json!(record.kind.quantity));
    object.insert("unit".into(), json!(record.kind.unit));
    object.insert("side".into(), json!(record.kind.side.word()));
    put(&mut object, "limit", record.limit());
    put(&mut object, "value", record.value());
    put(&mut object, "extent", record.extent());
    put(&mut object, "fraction", record.fraction());
    Value::Object(object)
}

fn flag_is_set(result: &Value, key: &str) -> bool {
    result.get(key).and_then(Value::as_f64) == Some(1.0)
}

/// The `exceedances` of a solution: for each flag of its result at 1, the
/// records of the limits its question is past. Empty when it carries none.
pub(crate) fn of(request: &Value, result: Option<&Value>) -> Vec<Value> {
    let Some(result) = result else {
        return Vec::new();
    };
    let solver = request.get("solver").and_then(Value::as_str).unwrap_or_default();
    let Some((_, flags)) = FLAGS.iter().find(|(name, _)| *name == solver) else {
        return Vec::new();
    };
    if solver == "ablator_slab" {
        return envelope_records(request, result);
    }
    let parameters = request.get("parameters").cloned().unwrap_or(Value::Null);
    let found = records(solver, &parameters);
    for flag in flags.iter() {
        // A build that checks holds the records to the solver's own flags.
        assert!(
            !genesis_kernel::exceedance::CHECKED
                || flag_is_set(result, flag) == found.iter().any(|record| record.kind.flag == *flag),
            "{solver}: {flag} is {} and its records are {:?}",
            result.get(*flag).unwrap_or(&Value::Null),
            found.iter().filter(|record| record.kind.flag == *flag).collect::<Vec<_>>()
        );
    }
    found
        .iter()
        .filter(|record| flag_is_set(result, record.kind.flag))
        .map(record_json)
        .collect()
}

/// The limits a solver's flags mark, as `describe_solver` lists them.
pub(crate) fn describe(solver: &str) -> Option<Value> {
    let kinds = kinds(solver);
    if kinds.is_empty() {
        return None;
    }
    Some(Value::Array(
        kinds
            .iter()
            .map(|kind| {
                let mut object = Map::new();
                object.insert("flag".into(), json!(kind.flag));
                object.insert("quantity".into(), json!(kind.quantity));
                object.insert("unit".into(), json!(kind.unit));
                object.insert("side".into(), json!(kind.side.word()));
                put(&mut object, "limit", kind.limit);
                object.insert("limit_words".into(), json!(kind.limit_words));
                Value::Object(object)
            })
            .collect(),
    ))
}

/// The words of `describe_solver` on what a flag reports beside it.
pub(crate) const DESCRIBE_RULE: &str = "A solution whose result carries a flag at 1 (for ablator_slab, an envelope flag at 0, outside the envelope) carries exceedances beside result: for each limit the question is past, the flag's key, the quantity, its unit, the side (above or below), the limit, the value, the extent past the limit in its unit and the extent as a fraction of the limit's size, each number with its _hex (NASA-STD-7009B, M&S 26). The records are read from the request's parameters, which the solution seal covers; the receipt covers them, and the long form of verify_solution computes them again. The limits are listed under exceedance_limits. A solution with no flag at 1 carries none.";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_traction_past_the_compared_one_is_read_with_its_extent() {
        let request = json!({"solver": "stress_plate_hole_s235_elastic", "parameters": {"traction_mpa": 120.0}});
        let out = of(&request, Some(&json!({"past_compared_traction": 1.0})));
        assert_eq!(out.len(), 1);
        assert_eq!(out[0]["flag"], "past_compared_traction");
        assert_eq!(out[0]["side"], "above");
        assert_eq!(out[0]["limit"], elastic::COMPARED_TRACTION_MPA);
        assert_eq!(out[0]["extent_hex"], format!("{:016x}", (120.0 - elastic::COMPARED_TRACTION_MPA).to_bits()));
        let inside = json!({"solver": "stress_plate_hole_s235_elastic", "parameters": {"traction_mpa": 100.0}});
        assert!(of(&inside, Some(&json!({"past_compared_traction": 0.0}))).is_empty());
    }

    #[test]
    fn a_rod_history_is_read_as_the_solver_reads_it() {
        let request = json!({"solver": "fuel_rod_triga_jsi", "parameters": {
            "start_temperature_k": 310.0,
            "time_s": [0.005, 0.0050001, 0.02, 15.0],
            "core_energy_released_mws": [1.0, 2.0, 3.0, 25.0],
        }});
        let result = json!({
            "start_temperature_outside_compared": 1.0,
            "core_energy_outside_compared": 1.0,
            "core_power_outside_compared": 1.0,
            "history_outside_compared": 1.0,
        });
        let out = of(&request, Some(&result));
        let named = |quantity: &str| out.iter().find(|entry| entry["quantity"] == quantity).unwrap_or_else(|| panic!("{quantity}: {out:?}")).clone();
        assert_eq!(named("start_temperature_k")["value"], 310.0);
        assert_eq!(named("core_energy_released_mws_at_end")["value"], 25.0);
        assert_eq!(named("largest_interval_s")["side"], "above");
        assert_eq!(named("history_end_s")["value"], 15.0);
        assert!(named("smallest_interval_s")["value"].as_f64().unwrap() < rod::HISTORY_INTERVAL_COMPARED_MIN_S);
        assert!(named("largest_interval_core_power_mw")["value"].as_f64().unwrap() > rod::CORE_POWER_COMPARED_MAX_MW);
    }

    #[test]
    fn a_culture_outside_its_ranges_is_read_on_each() {
        let request = json!({"solver": "culture_trichoderma_atp", "parameters": {
            "ph": 2.5, "yeast_extract_g_per_l": 3.0, "feed_glucose_g_per_l": 5.0, "segment_end_h": [100.0, 400.0],
        }});
        let result = json!({
            "ph_outside_compared": 1.0, "yeast_extract_outside_compared": 1.0,
            "feed_glucose_outside_compared": 1.0, "record_past_compared": 1.0,
        });
        let out = of(&request, Some(&result));
        assert_eq!(out.len(), 4);
        assert_eq!(out[0]["quantity"], "ph");
        assert_eq!(out[0]["side"], "below");
        assert_eq!(out[0]["extent"], 0.5);
        assert_eq!(out[3]["value"], 400.0);
        assert_eq!(out[3]["extent"], 88.0);
    }
}
