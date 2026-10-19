//! Every tool's input schema is a plain object: `type: "object"`, a
//! `properties` object that names each top-level key the parser accepts, a
//! `required` list that names each key it requires, and no choice between
//! shapes at the top level. Clients that need that shape show a tool with a
//! choice at the top as taking no arguments. One service refuses a list of
//! allowed values on anything but a string, and an empty allowed value: no
//! tool declares either, anywhere in its schema, in any view.

use std::collections::BTreeSet;

use serde_json::Value;

use crate::mcp_server::tools_in;
use crate::view::View;
use crate::{dataset, descriptor, solution, trajectory, validation, STEP_KEYS};

const FORBIDDEN_AT_TOP: [&str; 5] = ["oneOf", "anyOf", "allOf", "not", "if"];

fn names(list: &[&str]) -> BTreeSet<String> {
    list.iter().map(|name| name.to_string()).collect()
}

/// For each tool: the top-level keys its parser accepts, and the keys it
/// requires. Taken from the parser's own lists where the parser has one.
fn parser_keys(tool: &str) -> (BTreeSet<String>, BTreeSet<String>) {
    let (accepted, required): (Vec<&str>, Vec<&str>) = match tool {
        "list_cards" | "list_solvers" => (vec![], vec![]),
        // The form of the description is optional.
        "describe_card" => {
            let mut accepted = descriptor::DESCRIBE_KEYS.to_vec();
            accepted.push(descriptor::FORM);
            (accepted, descriptor::DESCRIBE_KEYS.to_vec())
        }
        // `from` is optional and is not part of the step.
        "compute_step" | "verify_step" => {
            let mut accepted = STEP_KEYS.to_vec();
            accepted.push(crate::chain::FROM);
            (accepted, STEP_KEYS.to_vec())
        }
        // The readout is optional and is not part of the run.
        "run_trajectory" => {
            let mut accepted = trajectory::TOP_LEVEL.to_vec();
            accepted.push(crate::readout::READOUT);
            accepted.push(crate::chain::FROM);
            (accepted, trajectory::TOP_LEVEL.to_vec())
        }
        "verify_tape" => {
            let keys = vec![
                "inputs",
                "frames",
                "outcome",
                "steps_landed",
                "reject",
                "run_seal",
                "final_state",
                "final_residual_conservation_error_hex",
                "max_residual_conservation_error_hex",
                "state_digest",
                crate::answer_key::PUBLIC_KEY_FIELD,
                crate::answer_key::SEEN_NONCES_FIELD,
                "request",
                "chain_seal",
                "links",
            ];
            // Either the request and its seal or the whole answer: no key is
            // always required.
            (keys, vec![])
        }
        "describe_solver" => (vec!["solver", "part", descriptor::FORM], vec!["solver"]),
        // The three tools added after 0.8.1.
        "cite" => (vec!["domain", "card", "solver"], vec![]),
        "find_model" => (vec!["description", "limit"], vec!["description"]),
        "find_flip" => (vec!["request", "vary", "low", "high", "level"], vec!["request", "vary", "low", "high"]),
        "design_run" => (vec!["request", "vary", "target"], vec!["request", "vary", "target"]),
        "solve" => {
            let mut accepted = solution::TOP_LEVEL.to_vec();
            accepted.push(solution::EXAMPLE);
            accepted.push(crate::chain::FROM);
            (accepted, vec!["solver"])
        }
        "verify_solution" => {
            let keys = vec![
                "inputs",
                "engine_version",
                "outcome",
                "iterations",
                "reject",
                "result",
                "invariants",
                "field_sha384",
                "seal_parts",
                "solution_seal",
                "request",
                crate::chain::FROM,
                crate::answer_key::PUBLIC_KEY_FIELD,
                crate::answer_key::SEEN_NONCES_FIELD,
            ];
            (keys, vec![])
        }
        // A plan of runs (domain, card, store, return) or of solves (solver),
        // each with plan and an optional answer form: only plan is always
        // required.
        "run_dataset" => {
            let keys = vec!["domain", "card", "plan", "store", "return", "solver", dataset::ANSWER];
            (keys, vec!["plan"])
        }
        // The request and its seal, the three parts, or the seal alone: no key
        // is always required.
        "verify_dataset" => {
            let mut accepted = dataset::PARTS_KEYS.to_vec();
            accepted.push("dataset_seal");
            accepted.push("request");
            accepted.push(crate::answer_key::PUBLIC_KEY_FIELD);
            accepted.push(crate::answer_key::SEEN_NONCES_FIELD);
            (accepted, vec![])
        }
        "verify_receipt" => (vec!["response_text", crate::answer_key::PUBLIC_KEY_FIELD, crate::answer_key::SEEN_NONCES_FIELD], vec!["response_text"]),
        // A run or a plan of solves: run and solves are each optional.
        "validate_series" => {
            let mut accepted = validation::TOP_LEVEL.to_vec();
            accepted.push("solves");
            (accepted, vec!["quantity", "measurements"])
        }
        other => panic!("a tool with no parser list: {other}"),
    };
    (names(&accepted), names(&required))
}

fn restricted() -> View {
    View::limited([("energy".to_string(), "thermal".to_string())], [])
}

fn views() -> Vec<(&'static str, View)> {
    vec![
        ("full", View::full()),
        ("restricted", restricted()),
        ("empty", View::limited([], [])),
    ]
}

#[test]
fn every_tool_declares_its_inputs_plainly_in_every_view() {
    for (label, view) in views() {
        let tools = tools_in(&view);
        assert_eq!(tools.len(), 18, "{label}");
        for tool in tools {
            let schema = Value::Object(tool.input_schema.as_ref().clone());
            let name = tool.name.to_string();
            for word in FORBIDDEN_AT_TOP {
                assert!(schema.get(word).is_none(), "{label} {name}: top-level {word}");
            }
            assert_eq!(schema["type"], "object", "{label} {name}");
            let properties = schema["properties"]
                .as_object()
                .unwrap_or_else(|| panic!("{label} {name}: no properties object"));
            let (accepted, required) = parser_keys(&name);
            let declared: BTreeSet<String> = properties.keys().cloned().collect();
            assert_eq!(declared, accepted, "{label} {name}: properties");
            // A tool with no always-required key omits the list: an empty
            // `required` is not valid in every draft of the schema language.
            let listed: BTreeSet<String> = match schema.get("required") {
                None => BTreeSet::new(),
                Some(list) => list
                    .as_array()
                    .unwrap_or_else(|| panic!("{label} {name}: required is not a list"))
                    .iter()
                    .map(|key| key.as_str().expect("a key").to_string())
                    .collect(),
            };
            if required.is_empty() {
                assert!(schema.get("required").is_none(), "{label} {name}: empty required list");
            }
            assert_eq!(listed, required, "{label} {name}: required");
        }
    }
}

/// Every schema node under `node`, the node itself first, with the path to it.
/// A node is any JSON object reached through objects and arrays, so the walk
/// reaches a property, an item, a branch of a choice and a nested property.
fn nodes<'a>(node: &'a Value, path: String, found: &mut Vec<(String, &'a Value)>) {
    match node {
        Value::Object(map) => {
            found.push((path.clone(), node));
            for (key, child) in map {
                nodes(child, format!("{path}.{key}"), found);
            }
        }
        Value::Array(items) => {
            for (index, child) in items.iter().enumerate() {
                nodes(child, format!("{path}[{index}]"), found);
            }
        }
        _ => {}
    }
}

/// The forms of a declaration that one laboratory's service or client refuses,
/// found in one tool's schema: a choice between shapes at the top level, a
/// list of allowed values on anything but a string, and an allowed value that
/// is empty. Each fault is one sentence that names the place.
fn refused_forms(schema: &Value) -> Vec<String> {
    let mut faults = Vec::new();
    for word in ["oneOf", "anyOf", "allOf"] {
        if schema.get(word).is_some() {
            faults.push(format!("top-level {word}"));
        }
    }
    let mut found = Vec::new();
    nodes(schema, String::new(), &mut found);
    for (path, node) in found {
        // A property that is itself named `enum` is a schema, not a list of values.
        let Some(Value::Array(values)) = node.get("enum") else {
            continue;
        };
        if node.get("type") != Some(&Value::String("string".to_string())) {
            faults.push(format!("{path}: an enum on a type that is not \"string\""));
        }
        if values.is_empty() {
            faults.push(format!("{path}: an enum with no value"));
        }
        for (index, value) in values.iter().enumerate() {
            match value {
                Value::String(text) if !text.is_empty() => {}
                Value::String(_) => faults.push(format!("{path}.enum[{index}]: an empty enum value")),
                _ => faults.push(format!("{path}.enum[{index}]: an enum value that is not a string")),
            }
        }
    }
    faults
}

/// One service answers 400 to a whole tool list when one declaration holds a
/// list of allowed values that are not strings ("enum[0]: cannot be empty"),
/// and one client shows a tool with a choice at the top as taking no input.
/// No tool, in any view, declares either form.
#[test]
fn no_tool_declares_a_form_that_a_service_refuses_in_any_view() {
    for (label, view) in views() {
        for tool in tools_in(&view) {
            let schema = Value::Object(tool.input_schema.as_ref().clone());
            let faults = refused_forms(&schema);
            assert!(faults.is_empty(), "{label} {}: {faults:?}", tool.name);
        }
    }
    // The pair of a card is a string with no list of names.
    let full = tools_in(&View::full());
    let run = full.iter().find(|tool| tool.name == "run_trajectory").expect("tool");
    assert_eq!(run.input_schema["properties"]["card"]["type"], "string");
    assert!(run.input_schema["properties"]["card"].get("enum").is_none());
}

/// The probe itself: each refused form is found where it is put, and the
/// declaration that one service refused on 2026-10-08 is among them.
#[test]
fn the_probe_finds_each_refused_form() {
    use serde_json::json;
    let plain = json!({ "type": "object", "properties": { "a": { "type": "string", "enum": ["x", "y"] } } });
    assert!(refused_forms(&plain).is_empty());
    for word in ["oneOf", "anyOf", "allOf"] {
        let mut schema = plain.clone();
        schema[word] = json!([{ "type": "object" }]);
        assert_eq!(refused_forms(&schema), vec![format!("top-level {word}")]);
    }
    // The declaration of `component` in version 0.2.1.
    let whole_number = json!({ "type": "object", "properties": { "component": { "type": "integer", "enum": [0, 1, 2] } } });
    let faults = refused_forms(&whole_number);
    assert_eq!(faults.len(), 4, "{faults:?}");
    assert!(faults[0].contains(".properties.component: an enum on a type that is not"), "{faults:?}");
    assert!(faults[1].contains("enum[0]: an enum value that is not a string"), "{faults:?}");
    // No type at all beside the list, an empty value, and a list deep inside a choice.
    let untyped = json!({ "type": "object", "properties": { "store": { "enum": ["frames", "none"] } } });
    assert_eq!(refused_forms(&untyped).len(), 1);
    let empty = json!({ "type": "object", "properties": { "a": { "type": "string", "enum": ["x", ""] } } });
    assert_eq!(refused_forms(&empty), vec![".properties.a.enum[1]: an empty enum value".to_string()]);
    let deep = json!({ "type": "object", "properties": { "plan": { "oneOf": [{ "type": "object", "properties": { "n": { "type": "integer", "enum": [1] } } }] } } });
    assert_eq!(refused_forms(&deep).len(), 2);
    // A key that is itself called `enum` is a property, not a list of values.
    let named = json!({ "type": "object", "properties": { "enum": { "type": "string" } } });
    assert!(refused_forms(&named).is_empty());
}

/// `component` of validate_series is a whole number from 0 to 2, declared by
/// its range, and its description states the three values.
#[test]
fn component_is_declared_by_its_range_and_its_description_states_the_three_values() {
    for (label, view) in views() {
        let tools = tools_in(&view);
        let tool = tools.iter().find(|tool| tool.name == "validate_series").expect("tool");
        let component = &tool.input_schema["properties"]["component"];
        assert_eq!(component["type"], "integer", "{label}");
        assert!(component.get("enum").is_none(), "{label}");
        assert_eq!(component["minimum"], 0, "{label}");
        assert_eq!(component["maximum"], 2, "{label}");
        assert!(component["description"].as_str().unwrap().starts_with("0, 1 or 2"), "{label}");
    }
}

/// A declaration without the worked examples it carries. What a declaration
/// says at an input ends, where the view has a card or a solver to show one
/// with, with a worked request: the worked plans of `run_dataset`, and the
/// worked check of each of the three checks by request. A worked request is
/// whole, so it names a card or a solver of the view.
fn without_worked_examples(schema: &Value) -> Value {
    const OPENINGS: [&str; 3] = [
        " One worked plan of each kind, each a whole request:",
        " Worked plan of solves:",
        " Worked example of the short form, as the whole arguments:",
    ];
    match schema {
        Value::Object(map) => Value::Object(
            map.iter()
                .map(|(key, value)| match value {
                    Value::String(words) if key == "description" => {
                        let ends = OPENINGS.iter().filter_map(|opening| words.find(opening)).min().unwrap_or(words.len());
                        (key.clone(), Value::String(words[..ends].to_string()))
                    }
                    other => (key.clone(), without_worked_examples(other)),
                })
                .collect(),
        ),
        Value::Array(items) => Value::Array(items.iter().map(without_worked_examples).collect()),
        other => other.clone(),
    }
}

/// The tools declare domain, card and solver once each as strings and list
/// no catalogue: list_cards and list_solvers name what an agent's view
/// holds. So, outside the worked examples said at an input, the declarations
/// are the same in every view and name no card or solver; a worked example
/// names a card or a solver of the view; and a restricted view names no card
/// outside it, in a declaration or in a description.
#[test]
fn the_pair_and_the_solver_are_strings_and_no_declaration_lists_the_catalogue() {
    let find = |tools: &[rmcp::model::Tool], name: &str| -> Value {
        let tool = tools.iter().find(|tool| tool.name == name).expect("tool");
        Value::Object(tool.input_schema.as_ref().clone())
    };
    let full = tools_in(&View::full());
    for tool in ["describe_card", "compute_step", "verify_step", "run_trajectory"] {
        let schema = find(&full, tool);
        for key in ["domain", "card"] {
            assert_eq!(schema["properties"][key]["type"], "string", "{tool} {key}");
            assert!(schema["properties"][key].get("enum").is_none(), "{tool} {key}");
        }
    }
    for tool in ["describe_solver", "solve"] {
        let schema = find(&full, tool);
        assert_eq!(schema["properties"]["solver"]["type"], "string", "{tool}");
        assert!(schema["properties"]["solver"].get("enum").is_none(), "{tool}");
    }
    // Outside its worked examples no declaration names a card or a solver, in any view.
    let all_cards: BTreeSet<String> = View::full().cards().iter().map(|c| c.card.to_string()).collect();
    // 0.15.0: one model per real object, 36 cards.
    assert_eq!(all_cards.len(), 36);
    let all_solvers: Vec<String> = View::full().solver_names().iter().map(|name| name.to_string()).collect();
    for (label, view) in views() {
        for tool in tools_in(&view) {
            let text = serde_json::to_string(&without_worked_examples(&Value::Object(tool.input_schema.as_ref().clone()))).unwrap();
            for name in all_cards.iter().chain(all_solvers.iter()) {
                // "thermal" is also a word of the proton-boron solver's name.
                if name == "thermal" {
                    assert!(!text.contains("\"thermal\""), "{label} {} names thermal", tool.name);
                    continue;
                }
                assert!(!text.contains(name.as_str()), "{label} {} names {name}", tool.name);
            }
        }
    }
    // A restricted view names no card outside it and no solver, in a
    // description or in a declaration, its worked examples included; and a
    // view of nothing has no worked example to show.
    let hidden: Vec<String> = all_cards.iter().filter(|card| *card != "thermal").cloned().collect();
    for tool in tools_in(&restricted()) {
        let description = tool.description.as_deref().unwrap_or("");
        let declaration = serde_json::to_string(&Value::Object(tool.input_schema.as_ref().clone())).unwrap();
        for name in hidden.iter().chain(all_solvers.iter()) {
            assert!(!description.contains(name.as_str()), "{} describes {name}", tool.name);
            assert!(!declaration.contains(name.as_str()), "{} declares {name}", tool.name);
        }
    }
    // Outside the worked examples the declarations are the same in every
    // view, and no description follows the view.
    let declared = |view: &View| -> Vec<Value> {
        tools_in(view).iter().map(|tool| Value::Object(tool.input_schema.as_ref().clone())).collect()
    };
    let plain = |view: &View| -> Vec<Value> { declared(view).iter().map(without_worked_examples).collect() };
    let nothing = View::limited([], []);
    assert_eq!(declared(&nothing), plain(&nothing));
    assert_ne!(declared(&View::full()), plain(&View::full()));
    for (label, view) in views() {
        assert_eq!(plain(&view), plain(&nothing), "{label}");
        let described = |view: &View| -> Vec<String> { tools_in(view).iter().map(|tool| tool.description.as_deref().unwrap_or("").to_string()).collect() };
        assert_eq!(described(&view), described(&nothing), "{label}");
    }
}
