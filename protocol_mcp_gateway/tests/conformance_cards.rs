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
        "specimen": "A copper fuse link on a 48 V node: its temperature under a load current. Range: compared from 1 to 3 A. Fitted on 4 of 8 runs: one constant. Compared with the 4 runs not in the fit: temperature within 0.4 percent RMS. Gap: at 3 A it reads 0.9 percent high. Refuses a link past its melting point, 1357.77 K (MeltReached).",
        "validation": { "result": "Compared from 1 to 3 A, the error was 0.4 percent." },
        "rejects": [{ "name": "MeltReached", "meaning": "The link reached its melting point, 1357.77 K." }],
    });
    let rejects = vec![("MeltReached".to_string(), "The link reached its melting point, 1357.77 K.".to_string())];
    assert!(text_findings("clean", &clean, COMPARED, &rejects).is_empty(), "{:?}", text_findings("clean", &clean, COMPARED, &rejects));

    let dirty = json!({
        "specimen": "A generic specimen, not a named real object.",
        "validation": { "result": "Checked over 1..=3 A." },
        "invariants": [{ "checked": "at every step" }],
    });
    let rejects = vec![("CurrentNotCompared".to_string(), "The current is outside the span compared with measurement.".to_string())];
    let found = text_findings("dirty", &dirty, COMPARED, &rejects);
    let rules: Vec<&str> = found.iter().map(|f| f.rule).collect();
    for rule in ["no check", "ranges in plain words", "not described by what it lacks", "a refusal is a physical limit", "compared range in numbers", SPEC_SHEET] {
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
fn a_forecast_reads_times_as_git_and_a_registration_write_them() {
    assert_eq!(utc_seconds("1970-01-01T00:00:00Z"), Some((0, false)));
    assert_eq!(utc_seconds("2026-10-15T03:21:59Z"), Some((1_792_034_519, false)));
    assert_eq!(utc_seconds("2026-10-14T20:21:59-07:00"), Some((1_792_034_519, false)));
    assert_eq!(utc_seconds("2026-10-15T03:21:59.250Z"), Some((1_792_034_519, false)));
    assert_eq!(utc_seconds("2026-10-15"), Some((1_792_022_400, true)));
    assert_eq!(utc_seconds("2024-02-29T12:00:00+00:00"), Some((1_709_208_000, false)));
    assert_eq!(utc_seconds("15 October"), None);
}

/// A model that states no forecast is held to nothing of the registry,
/// whatever its evidence: a company specimen, a compared model with no
/// registration, a second form fitted on all its data. A forecast stated is
/// held to the registration that carries its SHA-256.
#[test]
fn a_forecast_stated_is_held_to_its_registration_and_nothing_else_is() {
    for word in EVIDENCE_WORDS {
        assert!(forecast_findings("a_model_of_nothing", word, &json!({ "specimen": "Fitted on all 12 runs." }), &[]).is_empty(), "{word}");
    }
    let hash = "a".repeat(64);
    let stated = json!({ "validation": { "pass_line": format!("Forecast committed before the data was opened, as {hash}.") } });
    assert_eq!(forecasts_said(&stated), vec![hash.clone()]);
    let found = forecast_findings("a_model_of_nothing", COMPARED, &stated, &[]);
    assert!(found.iter().any(|f| f.weight == Weight::Contract && f.detail.contains("kit/forecasts/a_model_of_nothing.json is missing")), "{found:#?}");
    // A served forecast: the registry carries the hash its description states.
    let lageos: Value = serde_json::from_str(&std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/kit/forecasts/satellite_lageos1.json")).unwrap()).unwrap();
    let registered = lageos["protocol"]["sha256"].as_str().unwrap().to_string();
    let other = json!({ "validation": { "pass_line": format!("Forecast committed before the data was opened, as {}.", "b".repeat(64)) } });
    let found = forecast_findings("satellite_lageos1", COMPARED, &other, &[]);
    assert!(found.iter().any(|f| f.detail.contains(&format!("registers {registered}"))), "{found:#?}");
}

/// The spec sheet in a compared model's list sentence: each part named
/// when it is missing, out of order or without its number.
#[test]
fn a_spec_sheet_is_held_to_its_shape() {
    let good = "Creep of one cast: extension against time. Range: 140 to 230 MPa. Fitted on 6 of 12 tests: six constants. Compared with the 6 tests not in the fit: within 0.093 RMS in log time. Gap: below 0.1 percent it starts late, 1.5 to 4 times. Refuses a stress outside 140 to 230 MPa (StressOutsideCard).";
    assert!(spec_sheet_findings(good, &["StressOutsideCard"]).is_empty(), "{:?}", spec_sheet_findings(good, &["StressOutsideCard"]));
    let bad = |sentence: &str, words: &str| {
        let found = spec_sheet_findings(sentence, &["StressOutsideCard"]);
        assert!(found.iter().any(|wrong| wrong.contains(words)), "{words}: {found:#?}");
    };
    bad(&good.replace(" Gap: below 0.1 percent it starts late, 1.5 to 4 times.", ""), "no \"Gap: \" part");
    bad(&good.replace("(StressOutsideCard)", "(SomethingElse)"), "none of the model's own refusal names");
    bad(&good.replace("within 0.093 RMS", "about as good as the tests"), "no number within which");
    bad(&good.replace("Range: 140 to 230 MPa.", "Range: high stress."), "not a span in numbers");
    bad(&good.replace("Fitted on 6 of 12 tests: six constants.", "Fitted on 6 of 12 held-back tests."), "says \"held-back\"");
    bad(&good.replace("Compared with the 6 tests not in the fit: within 0.093 RMS in log time.", "it met its marks. Compared with the 6 tests not in the fit: within 0.093 RMS in log time."), "says \"met\"");
    let swapped = "Creep. Fitted on 6 of 12 tests. Range: 140 to 230 MPa. Compared with 6: within 0.093. Gap: 1.5 to 4 times. Refuses StressOutsideCard.";
    bad(swapped, "not in the order");
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
