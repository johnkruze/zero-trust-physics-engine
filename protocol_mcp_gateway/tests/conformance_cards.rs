//! The served contract of every card, attacked through the local connection
//! from each card's own description (`conformance_support`).
//!
//! `every_card_is_attacked_and_its_findings_are_reported` prints every
//! finding and, with `CONFORMANCE_REPORT` set, adds them as JSON lines to
//! that file, after the lines the solver sweep wrote there (`report`). It
//! fails only where a card cannot be described or attacked at all: the
//! findings are for the card's owner to read. A new card's router test holds
//! it to the whole contract with `assert_card_conforms`.

mod conformance_support;

use conformance_support::*;
use serde_json::{Value, json};

#[test]
fn every_card_is_attacked_and_its_findings_are_reported() {
    let only = std::env::var("CONFORMANCE_ONLY").ok();
    let mut door = Door::start();
    let cards: Vec<_> = verdicts(&mut door).into_iter().filter(|(domain, _, _)| !domain.is_empty()).collect();
    assert!(cards.len() >= 31, "{}", cards.len());
    let mut lines = Vec::new();
    for (domain, card, verdict) in &cards {
        if only.as_deref().is_some_and(|only| only != card) {
            continue;
        }
        let found = card_findings(&mut door, domain, card, verdict);
        assert!(!found.iter().any(|f| f.rule == "described"), "{card}: {found:?}");
        for finding in &found {
            println!("{}", finding.line());
            lines.push(finding.json());
        }
        if found.is_empty() {
            println!("{card} | met every attack");
        }
    }
    let contract = lines.iter().filter(|line| line.contains("\"weight\":\"contract\"")).count();
    println!("{contract} contract findings, {} findings in all, on {} cards", lines.len(), cards.len());
    report(&lines);
}

/// The rules on words catch each thing they look for, and pass a
/// description that says what is so.
#[test]
fn the_rules_on_words_catch_what_they_look_for() {
    let clean = json!({
        "specimen": "The company's own specimen: a copper fuse link on a 48 V node.",
        "validation": { "result": "Compared from 1 to 3 A, the error was 0.4 percent." },
        "rejects": [{ "name": "MeltReached", "meaning": "The link reached its melting point, 1357.77 K." }],
    });
    let rejects = vec![("MeltReached".to_string(), "The link reached its melting point, 1357.77 K.".to_string())];
    assert!(text_findings("clean", &clean, "met", &rejects).is_empty(), "{:?}", text_findings("clean", &clean, "met", &rejects));

    let dirty = json!({
        "specimen": "A generic specimen, not a named real object.",
        "validation": { "result": "Checked over 1..=3 A." },
        "invariants": [{ "checked": "at every step" }],
    });
    let rejects = vec![("CurrentNotCompared".to_string(), "The current is outside the span compared with measurement.".to_string())];
    let found = text_findings("dirty", &dirty, "met", &rejects);
    let rules: Vec<&str> = found.iter().map(|f| f.rule).collect();
    for rule in ["no check", "ranges in plain words", "not described by what it lacks", "a refusal is a physical limit", "compared range in numbers"] {
        assert!(rules.contains(&rule), "{rule}: {found:#?}");
    }
    // "checked" as a served key is found as a key.
    assert!(found.iter().any(|f| f.rule == "no check" && f.detail.contains("(key)")), "{found:#?}");

    // An input's own name is a field, not the edge of data; the same word
    // outside the name still is.
    let flagged = json!({ "load_fields": { "measured": { "unit": "1" } } });
    let field = vec![("MeasuredNotZeroOrOne".to_string(), "`measured` was neither 0 nor 1.".to_string())];
    assert!(text_findings("flagged", &flagged, "", &field).is_empty(), "{:?}", text_findings("flagged", &flagged, "", &field));
    let edge = vec![("MeasuredNotZeroOrOne".to_string(), "`measured` was neither 0 nor 1, the values measured.".to_string())];
    assert!(text_findings("flagged", &flagged, "", &edge).iter().any(|f| f.rule == "a refusal is a physical limit"));
    let wall = vec![("RateOutsideMeasured".to_string(), "`measured` is outside the span.".to_string())];
    assert!(text_findings("flagged", &flagged, "", &wall).iter().any(|f| f.rule == "a refusal is a physical limit"));
    // describe_card serves its fields as an array, each with its name: the
    // same three, read from the form an agent is served.
    let served = json!({ "load_fields": [{ "name": "measured", "unit": "1" }] });
    assert!(text_findings("served", &served, "", &field).is_empty(), "{:?}", text_findings("served", &served, "", &field));
    assert!(text_findings("served", &served, "", &edge).iter().any(|f| f.rule == "a refusal is a physical limit"));
    assert!(text_findings("served", &served, "", &wall).iter().any(|f| f.rule == "a refusal is a physical limit"));
}

#[test]
fn forecast_first_reads_times_as_git_and_a_registration_write_them() {
    assert_eq!(utc_seconds("1970-01-01T00:00:00Z"), Some((0, false)));
    assert_eq!(utc_seconds("2026-10-15T03:21:59Z"), Some((1_792_034_519, false)));
    assert_eq!(utc_seconds("2026-10-14T20:21:59-07:00"), Some((1_792_034_519, false)));
    assert_eq!(utc_seconds("2026-10-15T03:21:59.250Z"), Some((1_792_034_519, false)));
    assert_eq!(utc_seconds("2026-10-15"), Some((1_792_022_400, true)));
    assert_eq!(utc_seconds("2024-02-29T12:00:00+00:00"), Some((1_709_208_000, false)));
    assert_eq!(utc_seconds("15 October"), None);
    // A model held to no measurement is not held to forecast first; a compared
    // model with no registration is.
    assert!(forecast_findings("a_model_of_nothing", "not held to a measurement", &json!({}), &[]).is_empty());
    let found = forecast_findings("a_model_of_nothing", "missed", &json!({}), &[]);
    assert!(found.iter().any(|f| f.rule == "forecast first" && f.detail.contains("kit/forecasts/a_model_of_nothing.json is missing")), "{found:#?}");
}

/// A model calibrated to the record is held to its calibration, and to
/// nothing of forecast first without a blind registration: each rule broken
/// is named, and the word follows a blind scoring beside it.
#[test]
fn a_model_calibrated_to_the_record_is_held_to_its_calibration() {
    use sha2::{Digest, Sha256};
    // Served under the sixth word with no registration: named, and nothing of forecast first.
    let found = forecast_findings("a_model_of_nothing", CALIBRATED, &json!({}), &[]);
    assert!(found.iter().any(|f| f.rule == CALIBRATED && f.weight == Weight::Contract && f.detail.contains("kit/forecasts/a_model_of_nothing.json is missing")), "{found:#?}");
    assert!(!found.iter().any(|f| f.rule == "forecast first"), "{found:#?}");

    let dir = std::env::temp_dir().join(format!("calibrated_to_the_record_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let protocol = dir.join("comparison.json");
    std::fs::write(&protocol, b"the comparison's own record\n").unwrap();
    let sha: String = Sha256::digest(b"the comparison's own record\n").iter().map(|byte| format!("{byte:02x}")).collect();
    // The companion is a served solver whose own record is scored.
    let record = json!({
        "model": "a_calibrated_model",
        "verdict": CALIBRATED,
        "calibration": {
            "companion_to": "stress_plate_hole_s235",
            "protocol": { "path": protocol.to_str().unwrap(), "sha256": sha },
            "scored": { "utc": "2026-10-15T12:00:00Z", "output": { "path": "out.json", "sha256": "a".repeat(64) }, "program": { "path": "score.py", "sha256": "b".repeat(64) } },
            "not_blind": "The record was compared once before the constants were chosen."
        }
    });
    let served: Vec<String> = ["stress_plate_hole_s235", "a_calibrated_model"].map(String::from).to_vec();
    let described = json!({ "validation": { "result": "Calibrated to the record, not a blind test: the companion of stress_plate_hole_s235." } });
    let held = |record: &Value, verdict: &str, described: &Value, served: &[String]| -> Vec<String> {
        calibration_findings("a_calibrated_model", verdict, Some(record), described, served).iter().filter(|f| f.weight == Weight::Contract).map(|f| f.detail.clone()).collect()
    };
    assert_eq!(held(&record, CALIBRATED, &described, &served), Vec::<String>::new());
    let broken = |change: &dyn Fn(&mut Value), words: &str| {
        let mut changed = record.clone();
        change(&mut changed);
        let found = held(&changed, CALIBRATED, &described, &served);
        assert!(found.iter().any(|detail| detail.contains(words)), "{words}: {found:#?}");
    };
    // 1. The companion.
    broken(&|r| r["calibration"]["companion_to"] = json!("a_calibrated_model"), "names no served model beside it");
    broken(&|r| r["calibration"]["companion_to"] = json!("a_model_of_nothing"), "a_model_of_nothing, is not a served model");
    broken(&|r| r["calibration"]["companion_to"] = json!("a_model_of_nothing"), "is not scored");
    // 2. The fingerprint of the record compared with.
    broken(&|r| r["calibration"]["protocol"]["sha256"] = json!("abc"), "calibration.protocol.sha256 is not 64 hex characters");
    broken(&|r| r["calibration"]["protocol"]["sha256"] = json!("c".repeat(64)), "is not the one fingerprinted");
    // 3. The scoring.
    broken(&|r| r["calibration"]["scored"]["utc"] = Value::Null, "calibration.scored gives no time of the comparison");
    broken(&|r| r["calibration"]["scored"]["output"] = Value::Null, "calibration.scored.output gives no SHA-256");
    broken(&|r| r["calibration"]["scored"]["program"]["sha256"] = json!(""), "calibration.scored.program gives no SHA-256");
    // 4. The word and the description.
    assert!(held(&record, "missed", &described, &served).iter().any(|detail| detail.contains("the word served is calibrated to the record")));
    assert!(held(&record, CALIBRATED, &json!({ "validation": { "result": "Calibrated to the record." } }), &served).iter().any(|detail| detail.contains("does not say it is not a blind test")));
    // The word follows a blind scoring in the same record: then it is the scoring's.
    let mut scored = record.clone();
    scored["protocol"] = json!({ "path": protocol.to_str().unwrap(), "sha256": sha, "committed": null });
    scored["scored"] = json!({ "utc": "2026-10-16T12:00:00Z", "output": "out.json", "output_sha256": "d".repeat(64) });
    scored["verdict"] = json!("met");
    let words = |verdict: &str| -> Vec<String> {
        calibration_findings("a_calibrated_model", verdict, Some(&scored), &described, &served).iter().filter(|f| f.rule == CALIBRATED).map(|f| f.detail.clone()).collect()
    };
    assert_eq!(words("met"), Vec::<String>::new());
    assert!(words(CALIBRATED).iter().any(|detail| detail.contains("its blind registration is scored, met, and the list says calibrated to the record")), "{:#?}", words(CALIBRATED));
    // The blind registration is held to forecast first: this one is in no commit.
    assert!(calibration_findings("a_calibrated_model", "met", Some(&scored), &described, &served).iter().any(|f| f.rule == "forecast first" && f.detail.contains("is not committed")));
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn a_range_in_numbers_is_found_in_plain_words() {
    assert!(has_range_in_numbers("load 2 to 20 kN"));
    assert!(has_range_in_numbers("from 233.15 K to 398.15 K"));
    assert!(has_range_in_numbers("Mach 1.0 to 40"));
    assert!(!has_range_in_numbers("compared with the held-back runs"));
    assert!(!has_range_in_numbers("went to the lab"));
    assert_eq!(name_in("is above 1 s, the step ceiling of the x card (DtAboveCeiling)."), Some("DtAboveCeiling".to_string()));
    assert_eq!(name_in("no name (here) at all"), None);
}
