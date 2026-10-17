//! Release candidate 2 of 0.14.0: what the red team of rc1 found
//! (`evidence/verification/redteam_0.14.0_rc1/`, RT1, RT2 and RT3), each
//! held here by its own case, on the local connection.

mod hosted_support;

use hosted_support::*;
use serde_json::{Value, json};

fn parsed(text: &str) -> Value {
    serde_json::from_str(text).unwrap_or_else(|_| panic!("not JSON: {text}"))
}

fn ok(door: &mut Stdio2, tool: &str, arguments: &Value) -> Value {
    let (error, text) = door.tool(tool, arguments);
    assert!(!error, "{tool}: {text}");
    parsed(&text)
}

fn refused(door: &mut Stdio2, tool: &str, arguments: &Value) -> String {
    let (error, text) = door.tool(tool, arguments);
    assert!(error, "{tool} answered: {text}");
    assert!(!text.contains("run_seal\":") && !text.contains("chain_seal\":") && !text.contains("sha256_seal"), "{text}");
    text
}

/// A run of thermal and its continuation: the two answers of a chain.
fn a_chain(door: &mut Stdio2) -> (Value, Value) {
    let up = thermal_run(3, 293.15);
    let first = ok(door, "run_trajectory", &up);
    let mut down = thermal_run(3, 293.15);
    down["initial"]["temperature_k"] = Value::from(f64::from_bits(
        u64::from_str_radix(first["final_state"]["temperature_k_hex"].as_str().unwrap(), 16).unwrap(),
    ));
    down["from"] = json!({ "request": up, "run_seal": first["run_seal"] });
    let second = ok(door, "run_trajectory", &down);
    assert_eq!(second["chain"]["kind"], "continuation");
    (first, second)
}

/// A field of a link that is there and of the wrong type is named as a type
/// error, as the long form of one answer names it, and not as missing
/// (RT1 D-1, `rt1/repro/R3_links_outcome_number.py`): a link's `outcome`
/// that is not a string, and a `chain`'s `upstream_run_seal` or `chain_seal`
/// that is null. A field left out is still `InputMissing`.
#[test]
fn a_link_field_of_the_wrong_type_is_a_type_error() {
    let mut door = Stdio2::start(None);
    let (first, second) = a_chain(&mut door);
    let mut five = first.clone();
    five["outcome"] = json!(5);
    let text = refused(&mut door, "verify_tape", &json!({ "links": [five, second] }));
    assert_eq!(text, "Physical Violation: 'links[0].outcome' must be a string.");
    // The long form of the one answer says the same of the same field.
    let text = refused(&mut door, "verify_tape", &five);
    assert_eq!(text, "Physical Violation: 'outcome' must be a string.");
    for wrong in [json!(true), json!(["completed"]), json!({ "outcome": "completed" }), Value::Null] {
        let mut link = first.clone();
        link["outcome"] = wrong.clone();
        let text = refused(&mut door, "verify_tape", &json!({ "links": [link, second] }));
        assert_eq!(text, "Physical Violation: 'links[0].outcome' must be a string.", "{wrong}");
    }
    let mut gone = first.clone();
    gone.as_object_mut().unwrap().remove("outcome");
    let text = refused(&mut door, "verify_tape", &json!({ "links": [gone, second] }));
    assert_eq!(text, "Missing required parameter: 'links[0].outcome' (InputMissing).");
    // A later link's two seals of its chain, null and left out.
    for key in ["upstream_run_seal", "chain_seal"] {
        let mut link = second.clone();
        link["chain"][key] = Value::Null;
        let text = refused(&mut door, "verify_tape", &json!({ "links": [first, link] }));
        assert_eq!(text, format!("Physical Violation: 'links[1].chain.{key}' must be 64 lowercase hex digits."));
        link["chain"].as_object_mut().unwrap().remove(key);
        let text = refused(&mut door, "verify_tape", &json!({ "links": [first, link] }));
        assert_eq!(text, format!("Missing required parameter: 'links[1].chain.{key}' (InputMissing)."));
    }
    // The other way about: a field genesis_seal names by its type when it is
    // left out is missing, as at the top level; given of the wrong type it
    // keeps its sentence.
    for key in ["from", "to", "unit"] {
        let mut link = second.clone();
        link["chain"]["handed"][0].as_object_mut().unwrap().remove(key);
        let text = refused(&mut door, "verify_tape", &json!({ "links": [first, link] }));
        assert_eq!(text, format!("Missing required parameter: 'links[1].chain.handed[0].{key}' (InputMissing)."));
        link["chain"]["handed"][0][key] = json!(7);
        let text = refused(&mut door, "verify_tape", &json!({ "links": [first, link] }));
        assert!(text.starts_with(&format!("Physical Violation: 'links[1].chain.handed[0].{key}' must be a string")), "{text}");
    }
    let mut link = second.clone();
    link["chain"].as_object_mut().unwrap().remove("coupling");
    let text = refused(&mut door, "verify_tape", &json!({ "links": [first, link] }));
    assert_eq!(text, "Missing required parameter: 'links[1].chain.coupling' (InputMissing).");
    // upstream_chain_seal is null on a chain's second link, and holds.
    assert_eq!(second["chain"]["upstream_chain_seal"], Value::Null);
    let report = ok(&mut door, "verify_tape", &json!({ "links": [first, second] }));
    assert_eq!(report["holds"], json!(true), "{report}");
}

fn temperature_of(answer: &Value) -> f64 {
    f64::from_bits(u64::from_str_radix(answer["final_state"]["temperature_k_hex"].as_str().unwrap(), 16).unwrap())
}

/// ChainTooLong counts a step at the end of a chain as one of its sixteen,
/// and its sentence says so (RT1 W-1, `rt1/repro/R3_links_16_runs_plus_step.py`):
/// sixteen runs and a step were refused "the chain holds more than 16 runs".
#[test]
fn chain_too_long_counts_a_step_at_its_end() {
    let mut door = Stdio2::start(None);
    let said = "the chain holds more than 16 runs, a step at its end counted as one, the most one request may carry (ChainTooLong).";
    // Sixteen runs, each continuing the one before.
    let mut requests = vec![thermal_run(2, 293.15)];
    let mut answers = vec![ok(&mut door, "run_trajectory", &requests[0])];
    while answers.len() < 16 {
        let mut next = thermal_run(2, temperature_of(answers.last().unwrap()));
        next["from"] = json!({ "request": requests.last().unwrap(), "run_seal": answers.last().unwrap()["run_seal"] });
        answers.push(ok(&mut door, "run_trajectory", &next));
        requests.push(next);
    }
    let step_after = |requests: &Vec<Value>, answers: &Vec<Value>, at: usize| {
        json!({
            "domain": "energy", "card": "thermal",
            "parameters": { "temperature_k": temperature_of(&answers[at]), "heat_power_w": 1.0, "ambient_k": 293.15, "dt_s": 0.1 },
            "from": { "request": requests[at], "run_seal": answers[at]["run_seal"] }
        })
    };
    // A step after sixteen runs: seventeen, refused before any physics.
    let text = refused(&mut door, "compute_step", &step_after(&requests, &answers, 15));
    assert!(text.ends_with(said), "{text}");
    // A run after sixteen runs: the same.
    let mut seventeenth = thermal_run(2, temperature_of(&answers[15]));
    seventeenth["from"] = json!({ "request": requests[15], "run_seal": answers[15]["run_seal"] });
    let text = refused(&mut door, "run_trajectory", &seventeenth);
    assert!(text.ends_with(said), "{text}");
    // A step after fifteen runs is sixteen, and is answered.
    let step = ok(&mut door, "compute_step", &step_after(&requests, &answers, 14));
    // As links: fifteen runs and the step hold; sixteen runs and a step are seventeen answers.
    let mut links: Vec<Value> = answers[..15].to_vec();
    links.push(step.clone());
    let report = ok(&mut door, "verify_tape", &json!({ "links": links }));
    assert_eq!((report["holds"].as_bool(), report["links"].as_u64()), (Some(true), Some(16)), "{report}");
    let mut links: Vec<Value> = answers.clone();
    links.push(step);
    let text = refused(&mut door, "verify_tape", &json!({ "links": links }));
    assert_eq!(text, format!("Physical Violation: 'links' must be an array of at most 16 answers: {said}"));
}

/// A link whose own run answer leaves out frames, final_state or
/// state_digest is named at its place, `links[i].`, as its inputs, run_seal,
/// outcome and chain are; and a place is written without a dot at its end
/// (RT1 W-2, `rt1/repro/R3_links_missing_place.py`).
#[test]
fn a_link_is_named_at_its_place() {
    let mut door = Stdio2::start(None);
    let (first, second) = a_chain(&mut door);
    for key in ["frames", "final_state", "state_digest", "steps_landed", "reject", "final_residual_conservation_error_hex", "max_residual_conservation_error_hex"] {
        for at in [0usize, 1] {
            let mut links = [first.clone(), second.clone()];
            links[at].as_object_mut().unwrap().remove(key);
            let text = refused(&mut door, "verify_tape", &json!({ "links": links }));
            assert_eq!(text, format!("Missing required parameter: 'links[{at}].{key}' (InputMissing)."), "{key} at {at}");
        }
    }
    // Of the wrong type, at the link's place.
    let mut link = second.clone();
    link["frames"] = json!({});
    let text = refused(&mut door, "verify_tape", &json!({ "links": [first, link] }));
    assert_eq!(text, "Physical Violation: 'links[1].frames' must be a JSON array.");
    let mut link = second.clone();
    link["state_digest"] = json!("AB");
    let text = refused(&mut door, "verify_tape", &json!({ "links": [first, link] }));
    assert_eq!(text, "Physical Violation: 'links[1].state_digest' must be 64 lowercase hex digits.");
    // An outcome that is an empty string is missing there, as in the long form of one answer.
    let mut link = first.clone();
    link["outcome"] = json!("");
    let text = refused(&mut door, "verify_tape", &json!({ "links": [link, second] }));
    assert_eq!(text, "Missing required parameter: 'links[0].outcome' (InputMissing).");
    // A broken link before one with a field left out is named broken, as before: the check stops there.
    let mut broken = first.clone();
    broken["final_state"]["temperature_k_hex"] = json!("0000000000000000");
    let mut later = second.clone();
    later.as_object_mut().unwrap().remove("frames");
    let report = ok(&mut door, "verify_tape", &json!({ "links": [broken, later] }));
    assert_eq!((report["holds"].as_bool(), report["broken_link"].as_u64()), (Some(false), Some(0)), "{report}");
    // A chain that is null, or not an object, is named without a dot at its end.
    for chain in [Value::Null, json!(3), json!("continuation")] {
        let mut link = second.clone();
        link["chain"] = chain.clone();
        let text = refused(&mut door, "verify_tape", &json!({ "links": [first, link] }));
        assert_eq!(text, "Physical Violation: 'links[1].chain' must be a JSON object.", "{chain}");
    }
    let mut link = second.clone();
    link["chain"]["handed"][0] = json!(1);
    let text = refused(&mut door, "verify_tape", &json!({ "links": [first, link] }));
    assert_eq!(text, "Physical Violation: 'links[1].chain.handed[0]' must be a JSON object.");
    let mut link = second.clone();
    link["chain"]["handed"][0]["note"] = json!("x");
    let text = refused(&mut door, "verify_tape", &json!({ "links": [first, link] }));
    assert_eq!(text, "Physical Violation: 'links[1].chain.handed[0].note' is not a field of this request.");
    // A key the whole request does not have is named as before.
    let text = refused(&mut door, "verify_tape", &json!({ "links": [first, second], "extra": 1 }));
    assert_eq!(text, "Physical Violation: 'extra' is not a field of this request.");
}

/// "About 140 summary rows" is said of a grid on one axis, and a list holds
/// fewer, its head repeating each run's request (RT1 W-5, RT3 W4,
/// `rt1/repro/R10_about_140_rows.py`): one-step thermal runs, 141 on a grid
/// and 107 as a list, and solar_system's worked run 15 as a list.
#[test]
fn about_140_summary_rows_is_said_of_a_grid_and_a_list_holds_fewer() {
    let mut door = Stdio2::start(None);
    let base = json!({ "initial": { "temperature_k": 293.15 }, "dt_s": 0.1, "segments": [{ "steps": 1, "load": { "heat_power_w": 1.0, "ambient_k": 293.15 } }], "record_every": 1 });
    let grid = |n: usize| json!({ "domain": "energy", "card": "thermal", "plan": { "kind": "grid", "base": base, "axes": [{ "path": "segments[0].load.heat_power_w", "values": (0..n).map(|i| 1.0 + i as f64).collect::<Vec<_>>() }] }, "store": "none", "return": "inline" });
    let list = |n: usize| json!({ "domain": "energy", "card": "thermal", "plan": { "kind": "list", "runs": vec![base.clone(); n] }, "store": "none", "return": "inline" });
    for (plan, held) in [(&grid as &dyn Fn(usize) -> Value, 141), (&list, 107)] {
        let answer = ok(&mut door, "run_dataset", &plan(held));
        assert_eq!(answer["summary"].as_array().map(Vec::len), Some(held));
        let text = refused(&mut door, "run_dataset", &plan(held + 1));
        assert!(text.contains("more than the 40000 one inline answer may have"), "{text}");
    }
    let (_, described) = door.tool("describe_card", &json!({ "domain": "aerospace", "card": "solar_system" }));
    let mut worked = parsed(&described)["example_run_trajectory"].clone();
    let object = worked.as_object_mut().unwrap();
    object.remove("domain");
    object.remove("card");
    worked["segments"] = json!([{ "steps": 1, "load": worked["segments"][0]["load"].clone() }]);
    worked["record_every"] = json!(1);
    let solar = |n: usize| json!({ "domain": "aerospace", "card": "solar_system", "plan": { "kind": "list", "runs": vec![worked.clone(); n] }, "store": "none", "return": "inline" });
    assert_eq!(ok(&mut door, "run_dataset", &solar(15))["summary"].as_array().map(Vec::len), Some(15));
    refused(&mut door, "run_dataset", &solar(16));
}

/// verify_dataset's short form says what the seal of the plan it was given
/// covers: a plan of solves, its rows' solution seals; a plan of runs, its
/// rows' run seals (RT3 W6, `rt3/reproducers/W6_reference_sizes_and_short_form_words.py`).
#[test]
fn a_short_form_of_a_plan_of_solves_names_its_rows_solution_seals() {
    let mut door = Stdio2::start(None);
    let (_, described) = door.tool("describe_solver", &json!({ "solver": "propeller_apc_10x7e" }));
    let parameters = parsed(&described)["example_solve"]["parameters"].clone();
    let solves = json!({ "solver": "propeller_apc_10x7e", "plan": { "kind": "list", "solves": vec![json!({ "parameters": parameters }); 3] } });
    let mut run = thermal_run(2, 293.15);
    run.as_object_mut().unwrap().retain(|key, _| key != "domain" && key != "card");
    let runs = json!({ "domain": "energy", "card": "thermal", "plan": { "kind": "list", "runs": vec![run; 3] }, "store": "none", "return": "inline" });
    for (plan, said, not_said) in [(solves, "every row's solution seal", "every row's run seal"), (runs, "every row's run seal", "every row's solution seal")] {
        let answer = ok(&mut door, "run_dataset", &plan);
        let report = ok(&mut door, "verify_dataset", &json!({ "request": plan, "dataset_seal": answer["head"]["dataset_seal"] }));
        assert_eq!(report["holds"], json!(true), "{report}");
        let compared = report["compared"].as_str().unwrap();
        assert!(compared.contains(said) && !compared.contains(not_said), "{compared}");
    }
}
