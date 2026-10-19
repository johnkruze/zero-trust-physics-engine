//! A refusal's name, on the local connection.
//!
//! A value of a model's own input outside what the model takes is refused by
//! name: the name stands in round brackets at the end of the sentence, and
//! the model's description lists it with its meaning. A required key left
//! out is named InputMissing, and a solver's key given with a choice it does
//! not go with InputNotWithChoice, which the tools' input schemas state. A request that is not otherwise well formed (a key unknown, a
//! value of the wrong type, a number the number rule refuses, a count that
//! is not a count) is turned away in a sentence that carries no name. Both
//! are held here on every card and every solver the connection lists.

mod hosted_support;

use std::collections::BTreeSet;

use hosted_support::*;
use serde_json::{Value, json};

/// The name a sentence gives in round brackets, the last one when it gives
/// more than one bracket: a word that begins with a capital and holds no space.
fn name_in(sentence: &str) -> Option<&str> {
    let mut found = None;
    let mut rest = sentence;
    while let Some(open) = rest.find('(') {
        let after = &rest[open + 1..];
        let Some(close) = after.find(')') else { break };
        let inner = &after[..close];
        if inner.len() > 3
            && inner.chars().next().is_some_and(|first| first.is_ascii_uppercase())
            && inner.chars().all(|letter| letter.is_ascii_alphanumeric())
            && inner.chars().any(|letter| letter.is_ascii_lowercase())
        {
            found = Some(inner);
        }
        rest = &after[close + 1..];
    }
    found
}

fn described(door: &mut Stdio2, tool: &str, named: &Value) -> Value {
    let (error, text) = door.tool(tool, named);
    assert!(!error, "{named}: {text}");
    serde_json::from_str(&text).unwrap()
}

/// The sentence of a request the connection turns away before computing.
fn stopped(door: &mut Stdio2, tool: &str, request: &Value) -> String {
    let (error, text) = door.tool(tool, request);
    assert!(error, "{tool} {request} was answered: {text}");
    text
}

/// The names a description lists, each with a meaning that is a sentence.
fn listed(description: &Value, list: &str) -> BTreeSet<String> {
    description[list]
        .as_array()
        .unwrap_or_else(|| panic!("no {list}"))
        .iter()
        .map(|row| {
            let name = row["name"].as_str().expect("a name");
            let meaning = row["meaning"].as_str().unwrap_or_else(|| panic!("{name} has no meaning"));
            assert!(meaning.ends_with('.') && meaning.len() > 10, "{name}: {meaning}");
            name.to_string()
        })
        .collect()
}

fn cards(door: &mut Stdio2) -> Vec<(String, String)> {
    let list = described(door, "list_cards", &json!({}));
    list["cards"]
        .as_array()
        .unwrap()
        .iter()
        .map(|row| (row["domain"].as_str().unwrap().to_string(), row["card"].as_str().unwrap().to_string()))
        .collect()
}

fn below(x: f64) -> f64 {
    f64::from_bits(x.to_bits() - 1)
}

fn above(x: f64) -> f64 {
    f64::from_bits(x.to_bits() + 1)
}

/// The worked run of a card cut to one step, and its worked step.
fn worked(description: &Value) -> (Value, Value) {
    let mut run = description["example_run_trajectory"].clone();
    let first = run["segments"][0].clone();
    run["segments"] = json!([first]);
    run["segments"][0]["steps"] = json!(1);
    run["record_every"] = json!(1);
    (run, description["example_compute_step"].clone())
}

/// A step one 64-bit number under a card's floor is refused as
/// `DtBelowFloor` and one over its ceiling as `DtAboveCeiling`, by a run and
/// by a single step, on every card (from 0.15.0 no served card names both
/// `StepNotOneSecond`, the retired cell_2170_m80's names). A zero step is
/// `NonPositiveDt`. Each name is listed by the card with its meaning, in the
/// full form and among the stops of the short form.
#[test]
fn a_step_outside_a_cards_floor_or_ceiling_is_refused_by_a_name_the_card_lists() {
    let mut door = Stdio2::start(None);
    let pairs = cards(&mut door);
    assert_eq!(pairs.len(), protocol_mcp_gateway::CARDS);
    for (domain, card) in &pairs {
        let named = json!({ "domain": domain, "card": card });
        let description = described(&mut door, "describe_card", &named);
        let names = listed(&description, "rejects");
        let short = described(&mut door, "describe_card", &json!({ "domain": domain, "card": card, "form": "short" }));
        let stops: BTreeSet<String> = short["stops"].as_array().unwrap().iter().map(|name| name.as_str().unwrap().to_string()).collect();
        assert_eq!(stops, names, "{card}: the short form's stops are the full form's names");
        let (floor, ceiling) = (description["dt_s"]["floor_s"].as_f64().unwrap(), description["dt_s"]["ceiling_s"].as_f64().unwrap());
        let (under, over) = ("DtBelowFloor", "DtAboveCeiling");
        // The card lists the names it gives.
        assert!(names.contains(under) && names.contains(over), "{card}: {names:?}");
        // No served card's ceiling lies above pi over its frequency, so none gives or lists that name.
        assert!(!names.contains("DtAboveNyquist"), "{card}");
        assert!(description["dt_s"]["nyquist_s"].as_f64().is_none_or(|nyquist| nyquist >= ceiling), "{card}");

        let (run, step) = worked(&description);
        for (dt_s, side, name) in [
            (below(floor), "below", under),
            (floor / 4.0, "below", under),
            (above(ceiling), "above", over),
            (ceiling * 4.0, "above", over),
        ] {
            let what = if side == "below" { "floor" } else { "ceiling" };
            let mut a_run = run.clone();
            a_run["dt_s"] = json!(dt_s);
            let mut a_step = step.clone();
            a_step["parameters"]["dt_s"] = json!(dt_s);
            for (tool, request) in [("run_trajectory", &a_run), ("verify_step", &a_step)] {
                let sentence = stopped(&mut door, tool, request);
                assert!(sentence.starts_with("Physical Violation: 'dt_s' ") && sentence.contains(&format!(" is {side} ")), "{card} {tool} {dt_s}: {sentence}");
                assert!(sentence.ends_with(&format!(" s, the step {what} of the {card} card ({name}).")), "{card} {tool} {dt_s}: {sentence}");
                assert_eq!(name_in(&sentence), Some(name), "{sentence}");
            }
        }
        for dt_s in [0.0, -1.0] {
            let mut a_run = run.clone();
            a_run["dt_s"] = json!(dt_s);
            let sentence = stopped(&mut door, "run_trajectory", &a_run);
            assert_eq!(name_in(&sentence), Some("NonPositiveDt"), "{card}: {sentence}");
            assert!(names.contains("NonPositiveDt"), "{card}");
        }
    }
}

/// The sentences of the two step names, as an agent reads them.
#[test]
fn the_step_sentences_read_as_written() {
    let mut door = Stdio2::start(None);
    let creep = described(&mut door, "describe_card", &json!({ "domain": "materials", "card": "creep_cmsx6" }));
    let mut run = worked(&creep).0;
    run["dt_s"] = json!(60);
    assert_eq!(
        stopped(&mut door, "run_trajectory", &run),
        "Physical Violation: 'dt_s' 60 is above 49.97408036862698 s, the step ceiling of the creep_cmsx6 card (DtAboveCeiling)."
    );
    run["dt_s"] = json!(0.5);
    assert_eq!(
        stopped(&mut door, "run_trajectory", &run),
        "Physical Violation: 'dt_s' 0.5 is below 1 s, the step floor of the creep_cmsx6 card (DtBelowFloor)."
    );
    let meaning = |description: &Value, name: &str| -> String {
        description["rejects"].as_array().unwrap().iter().find(|row| row["name"] == name).unwrap_or_else(|| panic!("{name}"))["meaning"]
            .as_str()
            .unwrap()
            .to_string()
    };
    assert_eq!(meaning(&creep, "DtBelowFloor"), "`dt_s` is below the card's step floor, `floor_s` under `dt_s`. Refused before anything is computed.");
    assert_eq!(meaning(&creep, "DtAboveCeiling"), "`dt_s` is above the card's step ceiling, `ceiling_s` under `dt_s`. Refused before anything is computed.");

}

/// Every field of every card, sent past each end of its published range and
/// as a negative number: the refusal carries a name, and the card lists it.
/// A card that takes no start past a fresh one without `from`, and a card
/// that serves no continuation, list those two names as well.
#[test]
fn a_value_outside_a_published_bound_is_refused_by_a_name_the_card_lists() {
    let mut door = Stdio2::start(None);
    let mut sent = 0;
    for (domain, card) in cards(&mut door) {
        let description = described(&mut door, "describe_card", &json!({ "domain": domain, "card": card }));
        let names = listed(&description, "rejects");
        let (run, _) = worked(&description);
        for (section, list) in [("initial", "phase_fields"), ("load", "load_fields")] {
            for field in description[list].as_array().unwrap() {
                let name = field["name"].as_str().unwrap();
                let vector = field["components"] != json!(1);
                let mut values = vec![-1.0e30, 1.0e30, -1.0];
                if let Some(minimum) = field["minimum"].as_f64() {
                    values.push(minimum - minimum.abs() - 1.0);
                }
                if let Some(maximum) = field["maximum"].as_f64() {
                    values.push(maximum + maximum.abs() + 1.0);
                }
                for value in values {
                    let mut request = run.clone();
                    let target = if section == "initial" { &mut request["initial"] } else { &mut request["segments"][0]["load"] };
                    target[name] = if vector { json!([value, 0.0, 0.0]) } else { json!(value) };
                    let (error, text) = door.tool("run_trajectory", &request);
                    sent += 1;
                    let sentence = if error {
                        text
                    } else {
                        // A run that starts and is refused part way carries its name in a field.
                        let body: Value = serde_json::from_str(&text).unwrap();
                        match body["reject"].as_str() {
                            Some(reject) => {
                                assert!(names.contains(reject), "{card} {name} {value}: {reject} is not listed");
                                continue;
                            }
                            None => continue,
                        }
                    };
                    let given = name_in(&sentence).unwrap_or_else(|| panic!("{card} {name} {value}: no name in: {sentence}"));
                    assert!(names.contains(given), "{card} {name} {value}: {given} is not listed: {sentence}");
                }
            }
        }
        // The two names of a continuation, where the card's own description states them.
        let fresh = description["phase_fields"].as_array().unwrap().iter().any(|field| field["bound"].as_str().unwrap().contains("ContinuationWithoutUpstream"));
        assert_eq!(names.contains("ContinuationWithoutUpstream"), fresh, "{card}");
        // A card that serves no continuation lists ContinuationNotOnCard, and
        // so does one whose statement says it is refused from some states
        // (the crater before it stops, since the fix pass of 0.14.0).
        let continuation = &description["continuation"];
        let named = continuation["served"] == json!(false)
            || continuation["sentence"].as_str().unwrap().contains("(ContinuationNotOnCard)");
        assert_eq!(names.contains("ContinuationNotOnCard"), named, "{card}");
    }
    assert!(sent > 600, "{sent}");
}

/// The names given to the bounds and load rules that carried none.
#[test]
fn the_bounds_and_load_rules_that_carried_no_name_carry_their_cards_own() {
    let mut door = Stdio2::start(None);
    let mut run_of = |door: &mut Stdio2, domain: &str, card: &str| {
        let description = described(door, "describe_card", &json!({ "domain": domain, "card": card }));
        (worked(&description).0, listed(&description, "rejects"))
    };
    // The fuse link: a start and a load past the bounds a landed step is held to.
    let (run, names) = run_of(&mut door, "energy", "grid_node");
    for (section, field, unit, name) in [
        ("initial", "current_a", "A", "CurrentOutsideCard"),
        ("initial", "voltage_v", "V", "VoltageOutsideCard"),
        ("load", "load_current_a", "A", "CurrentOutsideCard"),
        ("load", "upstream_voltage_v", "V", "VoltageOutsideCard"),
    ] {
        for (value, side, kind, sign) in [(600.0, "above", "maximum", ""), (-600.0, "below", "minimum", "-")] {
            let mut request = run.clone();
            if section == "initial" { request["initial"][field] = json!(value) } else { request["segments"][0]["load"][field] = json!(value) };
            assert_eq!(
                stopped(&mut door, "run_trajectory", &request),
                format!("Physical Failure: '{field}' {value} {unit} is {side} {sign}595.7150486594088 {unit}, the inclusive {kind} of the grid_node card ({name}). That state is not sealed.")
            );
        }
        assert!(names.contains(name));
    }
    // The drive mode: its amplitude limit and its ledger limit at the start.
    let (run, names) = run_of(&mut door, "signal", "tesseract");
    for (field, value, name) in [
        ("displacement_m", 1.0, "AmplitudeOutsideCard"),
        ("displacement_m", -1.0, "AmplitudeOutsideCard"),
        ("velocity_m_s", 5.0, "AmplitudeOutsideCard"),
        ("velocity_m_s", -5.0, "AmplitudeOutsideCard"),
        ("energy_defect_j", 1.0e-9, "SpontaneousGeneration"),
        ("energy_defect_j", -1.0e-9, "FirstLawBreach"),
    ] {
        let mut request = run.clone();
        request["initial"][field] = json!(value);
        let sentence = stopped(&mut door, "run_trajectory", &request);
        assert!(sentence.ends_with(&format!("of the tesseract card ({name}). That state is not sealed.")), "{sentence}");
        assert!(names.contains(name));
    }
    // The titanium cell: the force that holds a ligament at yield, a start that stood under more, and the load rule.
    let (run, names) = run_of(&mut door, "materials", "ti64_lattice");
    for (section, field, value, ending) in [
        ("load", "force_n", json!([3600.0, 0.0, 0.0]), "the inclusive maximum of the ti64_lattice card (YieldReached). That state is not sealed."),
        ("initial", "start_force_n", json!([0.0, -3600.0, 0.0]), "the inclusive minimum of the ti64_lattice card (StartNotWritten). That state is not sealed."),
        ("load", "force_n", json!([1.0e30, 0.0, 0.0]), "N*s) (ForceAboveNyquist)."),
    ] {
        let mut request = run.clone();
        if section == "initial" { request["initial"][field] = value } else { request["segments"][0]["load"][field] = value };
        let sentence = stopped(&mut door, "run_trajectory", &request);
        assert!(sentence.ends_with(ending), "{sentence}");
    }
    for name in ["YieldReached", "StartNotWritten", "ForceAboveNyquist"] {
        assert!(names.contains(name), "{name}");
    }
    // The load rule of the mount, the drive mode, the oil column and the tendon, under the name each card lists.
    for (domain, card, field, name) in [
        ("signal", "resonance", "accel_m_s2", "ForceAboveNyquist"),
        ("signal", "tesseract", "accel_m_s2", "ForceAboveNyquist"),
        ("materials", "hydraulic_fluid", "flow_m3_s", "FlowAboveNyquist"),
        ("materials", "cable_stretch", "force_n", "ForceAboveNyquist"),
        ("energy", "battery_sag", "current_a", "CurrentAboveNyquist"),
    ] {
        let (mut request, names) = run_of(&mut door, domain, card);
        request["segments"][0]["load"][field] = json!(1.0e30);
        let sentence = stopped(&mut door, "run_trajectory", &request);
        assert!(sentence.contains("the Nyquist limit of the RK4 step") && sentence.ends_with(&format!(") ({name}).")), "{card}: {sentence}");
        assert!(names.contains(name), "{card}");
    }
    // A negative temperature, mass or density is a value of the card's own field below what it takes.
    for (domain, card, section, field, sentence) in [
        ("energy", "thermal", "initial", "temperature_k", "Physical Violation: Temperature 'temperature_k' cannot be below absolute zero, a negative number of kelvin (TemperatureOutsideCard)."),
        ("energy", "accumulator_sbo450", "initial", "gas_mass_kg", "Physical Violation: Mass 'gas_mass_kg' cannot be negative (GasMassNotPositive)."),
        ("energy", "tokamak_jet_dte1", "load", "line_density_per_m3", "Physical Violation: Density 'line_density_per_m3' cannot be negative (DensityNotPositive)."),
    ] {
        let (mut request, names) = run_of(&mut door, domain, card);
        if section == "initial" { request["initial"][field] = json!(-1.0) } else { request["segments"][0]["load"][field] = json!(-1.0) };
        assert_eq!(stopped(&mut door, "run_trajectory", &request), sentence);
        assert!(names.contains(name_in(sentence).unwrap()), "{card}");
    }
    // A range that runs the same distance either side of zero under a name that speaks of size.
    let (mut request, _) = run_of(&mut door, "signal", "cantilever_eth");
    request["initial"]["deflection_m"] = json!(-0.02);
    assert_eq!(
        stopped(&mut door, "run_trajectory", &request),
        "Physical Failure: 'deflection_m' -0.02 m is below -0.01875 m, the inclusive minimum of the cantilever_eth card: larger in size than the card takes (EnvelopeAboveCard). That state is not sealed."
    );
    request["initial"]["deflection_m"] = json!(0.02);
    assert_eq!(
        stopped(&mut door, "run_trajectory", &request),
        "Physical Failure: 'deflection_m' 0.02 m is above 0.01875 m, the inclusive maximum of the cantilever_eth card (EnvelopeAboveCard). That state is not sealed."
    );
}

fn solver_description(door: &mut Stdio2, solver: &str) -> (Value, BTreeSet<String>) {
    let description = described(door, "describe_solver", &json!({ "solver": solver }));
    let refusals = match description.get("refusals") {
        Some(_) => description.clone(),
        // A solver described in parts gives its refusals when asked.
        None => described(door, "describe_solver", &json!({ "solver": solver, "part": "refusals" })),
    };
    let names = listed(&refusals, "refusals");
    (description, names)
}

/// The names the solvers' entries give at the door: an atomic number outside
/// 1 to 18, a bar that is not one of the four, a fourth frequency, and a face
/// or a stream gas that is not one of its two words. Each is listed with its
/// meaning and as answered with no seal, and the input's row names it.
#[test]
fn the_solver_refusals_that_carried_no_name_carry_one_their_description_lists() {
    let mut door = Stdio2::start(None);
    let row = |description: &Value, name: &str| -> Value {
        description["refusals"].as_array().unwrap().iter().find(|row| row["name"] == name).unwrap_or_else(|| panic!("{name}")).clone()
    };

    let (atom, names) = solver_description(&mut door, "atom_dft");
    for (number, sentence) in [
        (json!(19), "Physical Violation: atomic number 19 is outside 1 to 18 (AtomicNumberOutOfRange)."),
        (json!(0), "Physical Violation: atomic number 0 is outside 1 to 18 (AtomicNumberOutOfRange)."),
        (json!(-3), "Physical Violation: atomic number -3 is outside 1 to 18 (AtomicNumberOutOfRange)."),
    ] {
        assert_eq!(stopped(&mut door, "solve", &json!({ "solver": "atom_dft", "parameters": { "atomic_number": number } })), sentence);
    }
    assert!(names.contains("AtomicNumberOutOfRange"));
    assert_eq!(atom["inputs"][0]["refusal"], "AtomicNumberOutOfRange");
    assert_eq!(row(&atom, "AtomicNumberOutOfRange")["reported_as"], "422, no seal");

    let (bar, names) = solver_description(&mut door, "modulus_bam_s355");
    let mut request = bar["example_solve"].clone();
    request["parameters"]["bar"] = json!("ERZ9");
    assert_eq!(
        stopped(&mut door, "solve", &request),
        "Physical Violation: 'bar' must be one of \"ERX1\", \"ERX2\", \"ERY1\", \"ERY2\"; this request has \"ERZ9\" (UnknownBar)."
    );
    let mut request = bar["example_solve"].clone();
    request["parameters"]["frequencies_hz"] = json!([1588.627, 8950.895, 4700.015, 4700.015]);
    assert_eq!(
        stopped(&mut door, "solve", &request),
        "Physical Violation: 'frequencies_hz' holds 4 frequencies, and a series is at most 3: flatwise, torsion, edgewise (TooManyFrequencies)."
    );
    for name in ["UnknownBar", "TooManyFrequencies"] {
        assert!(names.contains(name), "{name}");
        assert_eq!(row(&bar, name)["reported_as"], "422, no seal", "{name}");
    }
    assert_eq!(bar["inputs"][0]["refusal"], "UnknownBar");

    let (slab, names) = solver_description(&mut door, "ablator_slab");
    let listed_whole = described(&mut door, "describe_solver", &json!({ "solver": "ablator_slab", "part": "refusals" }));
    let held = slab["example_solve"].clone();
    let mut request = held.clone();
    request["parameters"]["face"] = json!("in_the_sun");
    assert_eq!(
        stopped(&mut door, "solve", &request),
        "Physical Violation: 'face' must be one of \"held_temperature\", \"gas_stream\"; this request has \"in_the_sun\" (UnknownFace)."
    );
    let stream = described(&mut door, "describe_solver", &json!({ "solver": "ablator_slab", "part": "example_solve_arc_jet_test" }));
    let mut request = stream["example_solve_arc_jet_test"].clone();
    assert!(request["parameters"]["stream_gas"].is_string(), "{request}");
    request["parameters"]["stream_gas"] = json!("helium");
    assert_eq!(
        stopped(&mut door, "solve", &request),
        "Physical Violation: 'stream_gas' must be one of \"air\", \"mixture\"; this request has \"helium\" (UnknownStreamGas)."
    );
    for name in ["UnknownFace", "UnknownStreamGas", "UnknownMaterial"] {
        assert!(names.contains(name), "{name}");
        assert_eq!(row(&listed_whole, name)["reported_as"], "422, no seal", "{name}");
    }
}

/// Every input of every solver, sent past each end of its range, as a word
/// that is not one of its own, and as an array one number too long: where
/// the refusal is of the value, it carries a name the description lists.
#[test]
fn a_value_outside_what_a_solver_takes_is_refused_by_a_name_the_solver_lists() {
    let mut door = Stdio2::start(None);
    let list = described(&mut door, "list_solvers", &json!({}));
    let mut named_refusals = 0;
    for solver in list["solvers"].as_array().unwrap().iter().map(|row| row["solver"].as_str().unwrap().to_string()) {
        let (description, names) = solver_description(&mut door, &solver);
        let inputs = match description.get("inputs") {
            Some(inputs) => inputs.clone(),
            None => described(&mut door, "describe_solver", &json!({ "solver": solver, "part": "inputs" }))["inputs"].clone(),
        };
        let example = description["example_solve"].clone();
        for input in inputs.as_array().unwrap() {
            let key = input["name"].as_str().unwrap();
            if example["parameters"].get(key).is_none() {
                // A key of the choice the worked example did not make.
                continue;
            }
            let mut values: Vec<Value> = Vec::new();
            match input["type"].as_str().unwrap() {
                "number" => {
                    if let Some(minimum) = input["minimum"].as_f64() {
                        values.push(json!(minimum - minimum.abs() - 1.0));
                    }
                    // An input with no ceiling but the largest 64-bit number
                    // has no number past it to send.
                    if let Some(maximum) = input["maximum"].as_f64().filter(|maximum| (maximum + maximum.abs() + 1.0).is_finite()) {
                        values.push(json!(maximum + maximum.abs() + 1.0));
                    }
                }
                "integer" => {
                    values.push(json!(input["minimum"].as_i64().unwrap() - 1));
                    values.push(json!(input["maximum"].as_i64().unwrap() + 1));
                }
                "string" => values.push(json!("not a word of this input")),
                "array of numbers" => {
                    let mut longer = example["parameters"][key].as_array().unwrap().clone();
                    if let Some(most) = input["most"].as_u64() {
                        if most < 200 {
                            let fill = longer.first().cloned().unwrap_or(json!(0.0));
                            longer.resize(most as usize + 1, fill);
                            values.push(Value::Array(longer));
                        }
                    }
                }
                _ => {}
            }
            for value in values {
                let mut request = example.clone();
                request["parameters"][key] = value.clone();
                let (error, text) = door.tool("solve", &request);
                if !error {
                    // Solved, or refused by the solver with a seal: its name is in a field.
                    let body: Value = serde_json::from_str(&text).unwrap();
                    if let Some(reject) = body["reject"].as_str() {
                        assert!(names.contains(reject), "{solver} {key}: {reject}");
                    }
                    continue;
                }
                let given = name_in(&text).unwrap_or_else(|| panic!("{solver} {key} {value}: no name in: {text}"));
                assert!(names.contains(given), "{solver} {key} {value}: {given} is not listed: {text}");
                // A range the entry writes is written in words.
                assert!(!text.contains("..=") || !text.contains(&format!("'{key}'")), "{solver} {key}: {text}");
                named_refusals += 1;
            }
        }
    }
    assert!(named_refusals > 60, "{named_refusals}");
}

/// A request that is not well formed is turned away in a sentence that
/// carries no name, by every tool that computes: a key that is not one of the
/// request's, a value of the wrong type, a number the number rule refuses, a
/// count that is not a count. A key missing is named InputMissing, and a key
/// that goes with a choice the request did not make InputNotWithChoice.
#[test]
fn a_request_that_is_not_well_formed_carries_no_name_and_a_missing_or_unchosen_key_is_named() {
    let named = |sentence: &str| if sentence.starts_with("Missing required parameter") { Some("InputMissing") } else { None };
    let mut door = Stdio2::start(None);
    let plate = json!({
        "domain": "energy", "card": "thermal",
        "initial": { "temperature_k": 293.15 }, "dt_s": 1.0, "record_every": 1,
        "segments": [{ "steps": 1, "load": { "heat_power_w": 1.0, "ambient_k": 293.15 } }]
    });
    let changed = |change: &dyn Fn(&mut Value)| {
        let mut request = plate.clone();
        change(&mut request);
        request
    };
    let runs: Vec<(Value, &str)> = vec![
        (changed(&|r| { r.as_object_mut().unwrap().remove("dt_s"); }), "Missing required parameter: 'dt_s' (InputMissing)."),
        (changed(&|r| { r["initial"].as_object_mut().unwrap().remove("temperature_k"); }), "Missing required parameter: 'temperature_k' (InputMissing)."),
        (changed(&|r| r["initial"]["pressure_pa"] = json!(1.0)), "Physical Violation: 'pressure_pa' is not a field of this request."),
        (changed(&|r| r["dt_s"] = json!("1")), "Physical Violation: 'dt_s' must be a number; this request has a string."),
        (changed(&|r| r["initial"]["temperature_k"] = json!(1.0e-310)), "Physical Violation: 'temperature_k' is not a finite normal number."),
        (changed(&|r| r["segments"][0]["load"]["heat_power_w"] = json!(1.0e39)), "Physical Violation: 'heat_power_w' does not fit in a finite frame slot."),
        (changed(&|r| r["segments"][0]["steps"] = json!(0)), "Physical Violation: 'steps' must be an integer >= 1."),
        (changed(&|r| r["segments"][0]["steps"] = json!(1_000_001)), "Physical Violation: the run asks for more than 1000000 steps."),
        (changed(&|r| r["segments"] = json!([])), "Physical Violation: 'segments' must contain at least one segment."),
    ];
    for (request, sentence) in &runs {
        let text = stopped(&mut door, "run_trajectory", request);
        assert_eq!(&text, sentence);
        assert_eq!(name_in(&text), named(sentence), "{text}");
    }
    let step = json!({ "domain": "energy", "card": "thermal", "parameters": { "temperature_k": 293.15, "heat_power_w": 1.0, "ambient_k": 293.15, "dt_s": 1.0 } });
    for (change, sentence) in [
        (json!({ "remove": "ambient_k" }), "Missing required parameter: 'ambient_k' (InputMissing)."),
        (json!({ "set": "pressure_pa", "to": 1.0 }), "Physical Violation: 'pressure_pa' is not a field of this request."),
        (json!({ "set": "dt_s", "to": null }), "Physical Violation: 'dt_s' must be a number; this request has null."),
    ] {
        let mut request = step.clone();
        match change.get("remove") {
            Some(key) => { request["parameters"].as_object_mut().unwrap().remove(key.as_str().unwrap()); }
            None => request["parameters"][change["set"].as_str().unwrap()] = change["to"].clone(),
        }
        let text = stopped(&mut door, "compute_step", &request);
        assert_eq!(text, sentence);
        assert_eq!(name_in(&text), named(sentence), "{text}");
    }

    // A solve: the flag of the thermal proton-boron solver left out, as its description says.
    let (thermal, _) = solver_description(&mut door, "fusion_pb11_thermal");
    let flag = thermal["inputs"].as_array().unwrap().iter().find(|input| input["name"] == "kinetic_enhancement").unwrap();
    assert!(flag["bound"].as_str().unwrap().contains("a request that leaves it out is refused as a missing key, in a sentence that names the key, as InputMissing"));
    let mut request = thermal["example_solve"].clone();
    request["parameters"].as_object_mut().unwrap().remove("kinetic_enhancement");
    assert_eq!(stopped(&mut door, "solve", &request), "Missing required parameter: 'kinetic_enhancement' (InputMissing).");
    let mut request = thermal["example_solve"].clone();
    request["parameters"]["kinetic_enhancement"] = json!("yes");
    let text = stopped(&mut door, "solve", &request);
    assert_eq!(text, "Physical Violation: 'kinetic_enhancement' must be true or false; this request has \"yes\".");
    assert_eq!(name_in(&text), None);
    let mut request = thermal["example_solve"].clone();
    request["parameters"]["plasma_current_a"] = json!(1.0);
    let text = stopped(&mut door, "solve", &request);
    assert!(text.starts_with("Physical Violation: 'plasma_current_a' is not an input of this solver."), "{text}");
    assert_eq!(name_in(&text), None);
    // The culture: a column of its record left out.
    let (culture, _) = solver_description(&mut door, "culture_trichoderma_ambr250");
    let mut request = culture["example_solve"].clone();
    request["parameters"].as_object_mut().unwrap().remove("water_ml");
    assert_eq!(stopped(&mut door, "solve", &request), "Missing required parameter: 'water_ml' (InputMissing).");
    // The heat shield: a key that goes with the choice the request did not make.
    let stream = described(&mut door, "describe_solver", &json!({ "solver": "ablator_slab", "part": "example_solve_arc_jet_test" }));
    let mut request = stream["example_solve_arc_jet_test"].clone();
    if request["parameters"]["stream_gas"] == "air" {
        request["parameters"]["stream_oxygen_mass_fraction"] = json!(0.2329);
        let text = stopped(&mut door, "solve", &request);
        assert_eq!(text, "Physical Violation: 'stream_oxygen_mass_fraction' is an input only when 'stream_gas' is \"mixture\"; this request has 'stream_gas' \"air\" (InputNotWithChoice).");
        assert_eq!(name_in(&text), Some("InputNotWithChoice"));
    } else {
        request["parameters"]["held_temperature_k"] = json!(1644.0);
        let text = stopped(&mut door, "solve", &request);
        assert!(text.contains("is an input only when 'face' is \"held_temperature\""), "{text}");
        assert_eq!(name_in(&text), Some("InputNotWithChoice"));
    }
    // The tools that take a model's inputs say the two names where the
    // inputs are: the step and the solve at their parameters, the run in
    // the words of its whole request.
    let tools = protocol_mcp_gateway::mcp_server::tools_in(&protocol_mcp_gateway::view::View::full());
    let schema = |name: &str| serde_json::to_string(tools.iter().find(|tool| tool.name == name).unwrap().input_schema.as_ref()).unwrap();
    assert!(schema("compute_step").contains("A required field left out is refused as InputMissing"));
    assert!(schema("run_trajectory").contains("A required key left out is named InputMissing"));
    assert!(schema("solve").contains("refused as InputMissing, and an input given with a choice it does not go with as InputNotWithChoice"));
}

/// The facts the descriptions state, held to the models as they compute.
#[test]
fn the_corrected_facts_are_as_the_models_compute() {
    let mut door = Stdio2::start(None);

    // culture_trichoderma_ambr250: the broth the vessel holds, in both places, and segments counted from 1.
    let culture = described(&mut door, "describe_solver", &json!({ "solver": "culture_trichoderma_ambr250" }));
    let result = culture["validation"]["result"].as_str().unwrap();
    assert!(result.contains("above 0 and up to 250 mL"), "{result}");
    let inputs = culture["inputs"].as_array().unwrap();
    let bound = |name: &str| inputs.iter().find(|input| input["name"] == name).unwrap()["bound"].as_str().unwrap().to_string();
    assert!(bound("sample_ml").contains("above 0 and at most 250 mL"));
    assert!(bound("segment_end_h").contains("A refusal names a segment by its place counted from 1: the first is segment 1."));
    let mut request = culture["example_solve"].clone();
    let segments = request["parameters"]["segment_end_h"].as_array().unwrap().len();
    assert!(segments >= 3, "{segments}");
    // The broth past the vessel at the end of the third segment: the solver's sentence calls it segment 3.
    request["parameters"]["water_ml"][2] = json!(200.0);
    let (error, text) = door.tool("solve", &request);
    assert!(error && text.contains("at the end of segment 3: it must be above 0 and at most 250 mL") && text.ends_with("(VolumeOutOfRange)."), "{text}");

    // cell_2170_model3: an inverted start is named for what it is, and the card serves no continuation.
    let cell = described(&mut door, "describe_card", &json!({ "domain": "energy", "card": "cell_2170_model3" }));
    let span = cell["rejects"].as_array().unwrap().iter().find(|row| row["name"] == "CaseTemperatureSpanInverted").unwrap();
    assert_eq!(span["meaning"], "The least case temperature the state carries is above the most it carries.");
    assert!(!cell["rejects"].as_array().unwrap().iter().any(|row| row["name"] == "CaseTemperatureSpanAboveCard"));
    assert_eq!(cell["continuation"]["served"], false);

    // fingerpad_p02_glass: no backslash is served before a quotation mark.
    let pad = described(&mut door, "describe_card", &json!({ "domain": "materials", "card": "fingerpad_p02_glass" }));
    let limits = pad["validation"]["limits"].as_str().unwrap();
    assert!(!limits.contains('\\'), "a backslash in: {limits}");
    assert!(limits.contains("and that \"the experiments were conducted on separate days\", so that \"the moisture of the participant's finger could have impacted the friction\"."));

    // capsule_insight: the end is the flown deployment, and the lowest state the compared run reached is named as such.
    let capsule = described(&mut door, "describe_card", &json!({ "domain": "aerospace", "card": "capsule_insight" }));
    assert_eq!(
        capsule["declared_ends"][0]["end"],
        "the flown parachute deployment, the last row of the flight's record, altitude under 6164.62 m and speed through the air under 352.1169 m/s at once, where the card's entry ends; the step that first crosses both lands, and with the flight's lift that step is at 5793 m and Mach 1.470, the lowest state the compared run reached"
    );
    assert!(capsule["phase_fields"][0]["bound"].as_str().unwrap().contains("The compared run with the flight's lift reaches down to 5793 m"));

    // accumulator_sbo450: the worked examples carry the 40 bar charge typed to four figures, and the text says so.
    let accumulator = described(&mut door, "describe_card", &json!({ "domain": "energy", "card": "accumulator_sbo450" }));
    assert_eq!(accumulator["example_run_trajectory"]["initial"]["gas_mass_kg"], json!(0.02747));
    assert!(accumulator["phase_fields"][0]["bound"].as_str().unwrap().contains("0.027467 kg for 40 bar. Each band is a thousandth either side, so a charge typed to four figures is inside, as the worked examples' 0.02747 kg is"));

    // pipe_flow_superpipe: the lowest compared temperature prints as 291.79, and the
    // temperature is held to Sutherland's law.
    let pipe = described(&mut door, "describe_solver", &json!({ "solver": "pipe_flow_superpipe" }));
    let temperature = pipe["inputs"].as_array().unwrap().iter().find(|input| input["name"] == "temperature_k").unwrap();
    assert_eq!(temperature["minimum"], json!(170.0));
    assert!(temperature["bound"].as_str().unwrap().contains("The compared runs were at 291.79 to 300.13 K"));
    assert!(!pipe.to_string().contains("291.78999999999996"));
    let mut request = pipe["example_solve"].clone();
    request["parameters"]["temperature_k"] = json!(291.79);
    let (error, text) = door.tool("solve", &request);
    assert!(!error, "{text}");
    request["parameters"]["temperature_k"] = json!(291.7);
    let (error, text) = door.tool("solve", &request);
    assert!(!error, "{text}");
    request["parameters"]["temperature_k"] = json!(169.9);
    let sentence = stopped(&mut door, "solve", &request);
    assert!(sentence.contains("outside 170 to 1900 K") && sentence.ends_with("(TemperatureOutsideViscosityLaw)."), "{sentence}");
}
