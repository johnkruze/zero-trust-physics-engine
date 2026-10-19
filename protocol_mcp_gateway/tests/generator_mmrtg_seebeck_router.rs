//! The generator_mmrtg_seebeck card, the second form of the generator beside
//! generator_mmrtg_f1, through the local connection. Stamped from the kit:
//! the whole served contract, attacked from the card's own description by
//! `conformance_support`, and each refusal's sentence word for word from the
//! card's source. Then the card's own answers: the end of the record and its
//! tolerance as generator_mmrtg_f1 serves them, the forecast served as it was
//! published, and a description that gives the days it is fitted on, its
//! error across all of them, the forecast's hash, and names
//! generator_mmrtg_f1.
//!
//! No measurement is read here.

mod conformance_support;

use conformance_support::{Door, assert_card_conforms};
use genesis_kernel::generator_mmrtg_seebeck;
use serde_json::{Value, json};

const SOURCE: &str = include_str!("../../genesis_kernel/src/generator_mmrtg_seebeck.rs");
const DAY_S: f64 = 86_400.0;

#[test]
fn generator_mmrtg_seebeck_meets_the_served_contract() {
    assert_card_conforms("energy", "generator_mmrtg_seebeck");
}

/// Each `StepReject` variant's doc sentence, as the card's source writes it.
fn documented() -> Vec<(String, String)> {
    let start = SOURCE.find("pub enum StepReject {").expect("the enum is in the card");
    let mut found = Vec::new();
    let mut doc = String::new();
    for line in SOURCE[start..].lines().skip(1) {
        if line == "}" {
            break;
        }
        let text = line.trim_start();
        if let Some(sentence) = text.strip_prefix("/// ") {
            if !doc.is_empty() {
                doc.push(' ');
            }
            doc.push_str(sentence);
        } else if text.starts_with(|c: char| c.is_ascii_uppercase()) {
            let name: String = text.chars().take_while(|c| c.is_ascii_alphanumeric()).collect();
            found.push((name, doc.replace("[`", "`").replace("`]", "`")));
            doc.clear();
        }
    }
    found
}

fn describe(door: &mut Door, card: &str) -> Value {
    door.value("describe_card", &json!({ "domain": "energy", "card": card })).expect("described")
}

fn run(door: &mut Door, mission_time_s: f64, dt_s: f64, steps: u64) -> Value {
    let request = json!({
        "domain": "energy", "card": "generator_mmrtg_seebeck",
        "initial": { "mission_time_s": mission_time_s },
        "dt_s": dt_s,
        "segments": [{ "steps": steps, "load": {} }],
        "record_every": 1
    });
    door.value("run_trajectory", &request).unwrap_or_else(|| panic!("{request} is answered"))
}

fn bits(value: &Value) -> f64 {
    f64::from_bits(u64::from_str_radix(value.as_str().expect("a hex number"), 16).expect("hex"))
}

/// A frame's flags, bytes 40 to 48 of its 64.
fn frame_flags(frame: &Value) -> u64 {
    let hex = frame["frame_hex"].as_str().expect("frame_hex");
    let bytes: Vec<u8> = (0..hex.len()).step_by(2).map(|at| u8::from_str_radix(&hex[at..at + 2], 16).unwrap()).collect();
    u64::from_le_bytes(bytes[40..48].try_into().unwrap())
}

#[test]
fn every_refusal_carries_the_sentence_of_its_source() {
    let mut door = Door::start();
    let description = describe(&mut door, "generator_mmrtg_seebeck");
    let rows = description["rejects"].as_array().expect("rejects");
    let documented = documented();
    assert_eq!(documented.len(), 9);
    for (name, sentence) in documented {
        let row = rows.iter().find(|row| row["name"] == name.as_str()).unwrap_or_else(|| panic!("{name} is not listed"));
        assert_eq!(row["meaning"], sentence.as_str(), "{name}");
    }
}

#[test]
fn the_worked_run_lands_and_closes_its_ledger() {
    let mut door = Door::start();
    let description = describe(&mut door, "generator_mmrtg_seebeck");
    let run = door.value("run_trajectory", &description["example_run_trajectory"]).expect("the worked run is answered");
    assert_eq!(run["outcome"], "completed", "{run}");
    assert!(run["max_residual_conservation_error"].as_f64().unwrap() <= 1.0e-6);
}

/// Ten steps of 3,843,485.5 s from 314,681,945 s land on the last day, and
/// so does the one step from the ninth returned mission time; a single step
/// from the decimal start and step lands on the day itself, each with
/// neither flag. The same verdict generator_mmrtg_f1 gives its own.
#[test]
fn a_restart_from_a_returned_mission_time_lands_where_the_run_straight_through_lands() {
    let mut door = Door::start();
    let dt_s = 3_843_485.5;
    let straight = run(&mut door, 314_681_945.0, dt_s, 10);
    assert_eq!(straight["outcome"], "completed", "{straight}");
    assert_eq!(straight["steps_landed"], 10);
    let end_s = bits(&straight["final_state"]["t_s_hex"]);
    assert!((end_s - generator_mmrtg_seebeck::MISSION_TIME_MAX_S).abs() <= generator_mmrtg_seebeck::MISSION_END_TOLERANCE_S);
    for frame in straight["frames"].as_array().unwrap() {
        assert_eq!(frame_flags(frame), 0, "{frame}");
    }

    let nine = run(&mut door, 314_681_945.0, dt_s, 9);
    let ninth_s = bits(&nine["final_state"]["t_s_hex"]);
    let restart = run(&mut door, ninth_s, dt_s, 1);
    assert_eq!(restart["outcome"], "completed", "{restart}");
    let restart_end_s = bits(&restart["final_state"]["t_s_hex"]);
    assert!((restart_end_s - end_s).abs() <= generator_mmrtg_seebeck::MISSION_END_TOLERANCE_S);
    assert_eq!(frame_flags(&restart["frames"][0]), 0);

    let single = run(&mut door, 349_273_314.5, dt_s, 1);
    assert_eq!(single["outcome"], "completed", "{single}");
    assert_eq!(bits(&single["final_state"]["t_s_hex"]), generator_mmrtg_seebeck::MISSION_TIME_MAX_S);

    let step = door
        .value("compute_step", &json!({
            "domain": "energy", "card": "generator_mmrtg_seebeck",
            "parameters": { "mission_time_s": ninth_s, "dt_s": dt_s }
        }))
        .expect("the single step is answered");
    assert!(step["sha384_seal"].is_string(), "{step}");
}

/// Steps past the last day of the record complete with both flags: two
/// steps at the floor whose end in decimal is 0.89 s past it, and a thousand
/// days past it, each with its proof. A start past where the decay heat left
/// falls under the ledger's resolution is refused by name.
#[test]
fn steps_past_the_last_day_of_the_record_complete_with_both_flags() {
    let mut door = Door::start();
    let both = generator_mmrtg_seebeck::FLAG_PAST_FITTING_SPAN | generator_mmrtg_seebeck::FLAG_PAST_RECORD;
    let card = generator_mmrtg_seebeck::GeneratorMmrtgSeebeck::curiosity();
    let floor_s = card.floor_dt_s().unwrap();
    assert_eq!(floor_s, 3.6654334769526953);
    let two = run(&mut door, 353_116_793.559_133_05, floor_s, 2);
    assert_eq!(two["outcome"], "completed", "{two}");
    assert!(two["run_seal"].is_string(), "{two}");
    assert_eq!(frame_flags(&two["frames"][0]), 0);
    assert_eq!(frame_flags(&two["frames"][1]), both);
    let days = run(&mut door, generator_mmrtg_seebeck::MISSION_TIME_MAX_S, DAY_S, 1_000);
    assert_eq!(days["outcome"], "completed", "{days}");
    assert_eq!(days["steps_landed"], 1_000);
    assert_eq!(bits(&days["final_state"]["t_s_hex"]), generator_mmrtg_seebeck::MISSION_TIME_MAX_S + 1_000.0 * DAY_S);
    assert_eq!(days["final_state"]["range_margin_s"].as_f64().unwrap(), 0.0);
    assert_eq!(days["final_state"]["past_fitting_span"], true);
    assert!(days["frames"].as_array().unwrap().iter().all(|frame| frame_flags(frame) == both));
    let ledger_end_s = card.ledger_end_mission_time_s().unwrap();
    let (error, text) = door.call(
        "run_trajectory",
        &json!({
            "domain": "energy", "card": "generator_mmrtg_seebeck",
            "initial": { "mission_time_s": 1.01 * ledger_end_s },
            "dt_s": DAY_S,
            "segments": [{ "steps": 1, "load": {} }],
            "record_every": 1
        }),
    );
    assert!(error, "{text}");
    assert!(text.contains("DecayHeatBelowLedgerResolution"), "{text}");
}

/// The card serves the published forecast: from the last day of the record,
/// in daily steps, it gives on days 4,088, 4,452, 4,818 and 5,183 the
/// forecast table's numbers (companion_seebeck/forecast_table.csv, column
/// primary_D_0_4087) to its twelve decimals, in 64 bits.
#[test]
fn the_card_serves_the_registered_forecast() {
    let mut door = Door::start();
    let start_s = generator_mmrtg_seebeck::MISSION_TIME_MAX_S;
    for (day, forecast) in [(4_088u32, 0.677318739978), (4_452, 0.661797565865), (4_818, 0.647186590932), (5_183, 0.633480592952)] {
        let answer = run(&mut door, start_s, DAY_S, u64::from(day - 4_087));
        assert_eq!(answer["outcome"], "completed", "{answer}");
        let ratio = bits(&answer["final_state"]["electrical_power_w_hex"]) / generator_mmrtg_seebeck::ELECTRICAL_POWER_AT_BEGINNING_OF_MISSION_W;
        assert!((ratio - forecast).abs() <= 1.0e-12, "day {day}: {ratio} against {forecast}");
    }
}

/// The fuel is generator_mmrtg_f1's and the output is not: one day from
/// landing on each card gives the same thermal power and decay heat, bit for
/// bit, and an electrical power 0.199 W lower here.
#[test]
fn the_companion_shares_the_fuel_of_generator_mmrtg_f1_and_not_its_output() {
    let mut door = Door::start();
    let step = |door: &mut Door, card: &str| {
        door.value("compute_step", &json!({ "domain": "energy", "card": card, "parameters": { "mission_time_s": 0.0, "dt_s": DAY_S } }))
            .expect("answered")
    };
    let original = step(&mut door, "generator_mmrtg_f1");
    let companion = step(&mut door, "generator_mmrtg_seebeck");
    for key in ["t_s_hex", "thermal_power_w_hex", "fuel_heat_j_hex", "range_margin_s_hex"] {
        assert_eq!(original["state"][key], companion["state"][key], "{key}");
    }
    let gap_w = bits(&original["state"]["electrical_power_w_hex"]) - bits(&companion["state"]["electrical_power_w_hex"]);
    assert!((gap_w - 0.1994).abs() < 1.0e-3, "{gap_w}");
    assert_ne!(original["sha384_seal"], companion["sha384_seal"]);
}

/// The description says, first, what the card is: the second form of the
/// generator beside generator_mmrtg_f1, fitted on all 4,088 published days
/// and within 0.00319 of 115.4 W RMS across them, in the six parts of its
/// spec sheet; its pass line ends with the hash of the forecast as the kit
/// holds it; its result gives the numbers on the fitted days, the form's
/// numbers with its constants fitted on days 0 to 2,060 alone, and the
/// forecast; and generator_mmrtg_f1's own description does not name it.
#[test]
fn the_description_says_calibrated_to_the_record_and_names_its_companion() {
    let mut door = Door::start();
    let card = describe(&mut door, "generator_mmrtg_seebeck");
    let summary = card["specimen"].as_str().unwrap();
    assert!(summary.starts_with("Daily-average electrical output of the radioisotope generator on the Curiosity rover, flight unit F1, in the Seebeck matched-load form, the second form of this generator beside generator_mmrtg_f1"), "{summary}");
    for words in [
        " Range: compared on mission days 0 to 4,087, every published day; computes from landing to 6.502e10 s",
        " Fitted on all 4,088 published days, mission days 0 to 4,087",
        " Compared with all 4,088 days, all in the fit: within 0.00319 of 115.4 W RMS across all 4,088",
        " Gap: its largest error on one day is +0.0160 of 115.4 W, 1.85 W, on mission day 3,546",
        "(MissionTimeBeforeLanding)",
        "(DecayHeatBelowLedgerResolution)",
    ] {
        assert!(summary.contains(words), "{words}: {summary}");
    }
    // The forecast's hash is the kit's own record of it.
    let forecast: Value = serde_json::from_str(include_str!("../kit/forecasts/generator_mmrtg_seebeck.json")).expect("the forecast is JSON");
    let hash = forecast["protocol"]["sha256"].as_str().expect("its hash");
    assert_eq!(hash, "0afc18e0e7af9b4405b4ce5b16764226ee917774d72253a1590cc179e76d86d1");
    let pass_line = card["validation"]["pass_line"].as_str().unwrap();
    assert!(pass_line.ends_with(&format!("Forecast committed before the data was opened, as {hash}.")), "{pass_line}");
    assert!(pass_line.contains("each at most 0.010 of 115.4 W, 1.154 W, on 365 days or more"), "{pass_line}");
    let result = card["validation"]["result"].as_str().unwrap();
    assert!(result.starts_with("On mission days 0 to 4,087, all in the fit, run through the engine in daily steps from landing: RMS error 0.00319 of 115.4 W"), "{result}");
    for words in [
        "+0.010094, 0.000094 over its target of 0.010",
        "an RMS error of 0.00507 and a final-365 mean of +0.00468",
        "0.00572 and +0.00187",
        "on Perseverance's generator F2, its days 0 to 985",
        "commit a0aff5e",
        "0.677319 of 115.4 W on day 4,088, 0.661798 on day 4,452, 0.647187 on day 4,818 and 0.633481 on day 5,183",
    ] {
        assert!(result.contains(words), "{words}: {result}");
    }
    let limits = card["validation"]["limits"].as_str().unwrap();
    for words in [
        "Its form was chosen with all 4,088 days read",
        "generator_mmrtg_f1 is the form compared on days 2,809 to 4,087 with its constants fitted on days 0 to 2,060",
        "0.6602 to 0.6662 of 115.4 W on day 4,452 and 0.6308 to 0.6390 on day 5,183",
    ] {
        assert!(limits.contains(words), "{words}: {limits}");
    }
    let flags = card["advisory_flags"].to_string();
    assert!(flags.contains("FLAG_PAST_FITTING_SPAN") && flags.contains("FLAG_PAST_RECORD"), "{flags}");
    assert!(card["declared_ends"].as_array().is_none_or(|ends| ends.is_empty()), "{}", card["declared_ends"]);
    // The original gives its own days and does not name this form.
    let original = describe(&mut door, "generator_mmrtg_f1");
    assert!(original["validation"].to_string().contains("2,809 to 4,087"), "{}", original["validation"]);
    assert!(!original.to_string().contains("generator_mmrtg_seebeck"));
}
