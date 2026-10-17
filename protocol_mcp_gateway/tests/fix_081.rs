//! Engine 0.8.1: corrections to the 0.7.1 tool changes, applied to 0.8.0.
//!
//! A fresh start states its anchors equal to the card's own energy (and on
//! solar_system its own angular momentum) at the stated start, within the
//! published rounding allowance, or it is refused as
//! ContinuationWithoutUpstream; a continuation with from still runs. The
//! short checks say truly what they compared; a chained check given no chain
//! seal returns none; the dataset long form returns no rebuilt seal that does
//! not hold; a solve request with neither parameters nor example names both;
//! the cell_2170_model3 card no longer explains a continuation it refuses.

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
    text
}

/// A run of the cold plate as a plan lists it, without its pair.
fn plan_run() -> Value {
    let mut run = thermal_run(5, 293.15);
    let object = run.as_object_mut().unwrap();
    object.remove("domain");
    object.remove("card");
    run
}

fn exact(hex: &Value) -> Value {
    match hex {
        Value::String(text) => json!(f64::from_bits(u64::from_str_radix(text, 16).unwrap())),
        Value::Array(items) => Value::Array(items.iter().map(exact).collect()),
        _ => panic!("{hex}"),
    }
}

/// Where a continuation reads a start field its card's returned state gives
/// under another name (src/chain.rs).
fn source<'a>(card: &str, field: &'a str) -> &'a str {
    match (card, field) {
        ("ti64_lattice", "start_t_s") => "t_s",
        ("ti64_lattice", "start_force_n") => "applied_force_n",
        ("ti64_lattice", "start_temperature_k") => "temperature_k",
        ("ti64_lattice", "start_energy_anchor_j") => "energy_anchor_j",
        ("ti64_lattice", "start_boundary_work_j") => "boundary_work_j",
        _ => field,
    }
}

fn handed_back(described: &Value, answer: &Value) -> Value {
    let card = described["card"].as_str().unwrap();
    let state = &answer["final_state"];
    let mut initial = serde_json::Map::new();
    for field in described["phase_fields"].as_array().unwrap() {
        let name = field["name"].as_str().unwrap();
        initial.insert(name.to_string(), exact(&state[format!("{}_hex", source(card, name))]));
    }
    Value::Object(initial)
}

/// An anchor moved past the allowance and inside the card's own ledger
/// limit, so that the door's rule, not the card's, is what refuses it.
fn moved(card: &str, field: &str, value: &Value) -> Value {
    match (card, field) {
        // The limits are 1e-6 of a scale of the order of a millijoule or
        // more; the allowance is a few 1e-15 of the start's own energy.
        ("resonance", _) | ("ti64_lattice", _) => json!(value.as_f64().unwrap() + 1.0e-12),
        // The limit is 1e-12 of 1.32e25; the allowance 2.4e-14 of the sum
        // of the energy's terms, about 4e25.
        ("solar_system", "energy_anchor") => json!(value.as_f64().unwrap() * (1.0 + 2.0e-13)),
        // The limit is 1e-12 of 2.09e33; the allowance 9.3e-15 of the sum
        // of mu |r| |v|. Move the largest component by 2e-13 of |L|.
        ("solar_system", "angular_momentum_anchor") => {
            let mut items: Vec<f64> = value.as_array().unwrap().iter().map(|item| item.as_f64().unwrap()).collect();
            let size = items.iter().map(|item| item * item).sum::<f64>().sqrt();
            let largest = (0..3).max_by(|a, b| items[*a].abs().total_cmp(&items[*b].abs())).unwrap();
            items[largest] += 2.0e-13 * size;
            json!(items)
        }
        other => panic!("{other:?}"),
    }
}

const ANCHORS: [(&str, &str, &str); 4] = [
    ("signal", "resonance", "energy_anchor_j"),
    ("materials", "ti64_lattice", "start_energy_anchor_j"),
    ("aerospace", "solar_system", "energy_anchor"),
    ("aerospace", "solar_system", "angular_momentum_anchor"),
];

/// Per card with an anchor among its start fields: the worked run and step,
/// fresh, answer with no from; an anchor off the card's own value by more
/// than the allowance is refused by name, run and step, with no proof; the
/// worked run continued from its own sealed answer with from answers; and the
/// field's text publishes the allowance.
#[test]
fn a_fresh_start_states_its_own_anchors_or_is_refused_by_name() {
    let mut door = Stdio2::start(None);
    for (domain, card, field) in ANCHORS {
        let described = ok(&mut door, "describe_card", &json!({ "domain": domain, "card": card }));
        let words = described["phase_fields"].as_array().unwrap().iter().find(|row| row["name"] == field).unwrap()["bound"]
            .as_str()
            .unwrap()
            .to_string();
        assert!(
            words.contains("within 2 (n + 16) u S") && words.contains("refused as ContinuationWithoutUpstream"),
            "{card} {field}: {words}"
        );
        let run = described["example_run_trajectory"].clone();
        let step = described["example_compute_step"].clone();
        // Fresh: no from needed.
        let upstream = ok(&mut door, "run_trajectory", &run);
        ok(&mut door, "verify_step", &step);
        // The anchor off its own value: refused by name, run and step.
        let mut off_run = run.clone();
        off_run["initial"][field] = moved(card, field, &run["initial"][field]);
        let mut off_step = step.clone();
        off_step["parameters"][field] = moved(card, field, &step["parameters"][field]);
        for (tool, request, section) in [("run_trajectory", &off_run, "initial"), ("verify_step", &off_step, "parameters")] {
            let text = refused(&mut door, tool, request);
            assert!(text.ends_with("(ContinuationWithoutUpstream)."), "{card} {tool}: {text}");
            assert!(text.contains(&format!("'{section}.{field}' is ")), "{card} {tool}: {text}");
            assert!(text.contains("where the allowance is"), "{card} {tool}: {text}");
            assert!(!text.contains("run_seal") && !text.contains("sha256_seal"), "{card} {tool}: {text}");
        }
        // Continued from the worked run's sealed answer, with from: taken,
        // its anchor being the upstream's, not its own energy.
        assert_eq!(upstream["outcome"], "completed", "{card}");
        let mut downstream = run.clone();
        downstream["initial"] = handed_back(&described, &upstream);
        downstream["from"] = json!({ "request": run, "run_seal": upstream["run_seal"] });
        let continued = ok(&mut door, "run_trajectory", &downstream);
        assert_eq!(continued["chain"]["kind"], "continuation", "{card}");
    }
}

/// The allowance is not zero: on the mount, an anchor one unit in the last
/// place off the start's own energy is an honest hand computation and runs,
/// and one 1e-12 J off is refused.
#[test]
fn an_anchor_within_the_rounding_allowance_is_a_fresh_start() {
    let mut door = Stdio2::start(None);
    let oscillator = genesis_kernel::resonance::DynamicOscillator::mount_10hz();
    let own = oscillator.mechanical_energy_j(0.005, 0.1);
    let mount = |anchor: f64| json!({
        "domain": "signal", "card": "resonance",
        "initial": { "displacement_m": 0.005, "velocity_m_s": 0.1, "t_s": 0.0, "energy_anchor_j": anchor, "boundary_work_j": 0.0 },
        "dt_s": 0.0003, "segments": [{ "steps": 10, "load": { "accel_m_s2": 1.0 } }], "record_every": 10
    });
    ok(&mut door, "run_trajectory", &mount(own));
    ok(&mut door, "run_trajectory", &mount(f64::from_bits(own.to_bits() + 1)));
    ok(&mut door, "run_trajectory", &mount(f64::from_bits(own.to_bits() - 1)));
    let text = refused(&mut door, "run_trajectory", &mount(own + 1.0e-12));
    assert!(text.ends_with("(ContinuationWithoutUpstream).") && text.contains("'initial.energy_anchor_j' is"), "{text}");
    // At rest the own energy is 0, and so is the allowance.
    let mut rest = mount(1.0e-12);
    rest["initial"]["displacement_m"] = json!(0.0);
    rest["initial"]["velocity_m_s"] = json!(0.0);
    let text = refused(&mut door, "run_trajectory", &rest);
    assert!(text.ends_with("(ContinuationWithoutUpstream)."), "{text}");
}

fn mount_run(initial: Value, steps: u64) -> Value {
    let gate = genesis_kernel::resonance::DynamicOscillator::mount_10hz().gate_dt_s().unwrap();
    json!({
        "domain": "signal", "card": "resonance", "initial": initial, "dt_s": gate,
        "segments": [{ "steps": steps, "load": { "accel_m_s2": 3.0 } }],
        "record_every": steps
    })
}

/// A chained check given only the run seal returns no chain seal: it was not
/// compared, and the answer says so and where the chain seal is.
#[test]
fn a_chained_check_given_only_a_run_seal_returns_no_chain_seal() {
    let mut door = Stdio2::start(None);
    let own = genesis_kernel::resonance::DynamicOscillator::mount_10hz().mechanical_energy_j(0.005, 0.1);
    let first_request = mount_run(
        json!({ "displacement_m": 0.005, "velocity_m_s": 0.1, "t_s": 0.0, "energy_anchor_j": own, "boundary_work_j": 0.0 }),
        400,
    );
    let first = ok(&mut door, "run_trajectory", &first_request);
    let described = ok(&mut door, "describe_card", &json!({ "domain": "signal", "card": "resonance" }));
    let mut request = mount_run(handed_back(&described, &first), 600);
    request["from"] = json!({ "request": first_request, "run_seal": first["run_seal"] });
    let chained = ok(&mut door, "run_trajectory", &request);
    let alone = ok(&mut door, "verify_tape", &json!({ "request": request, "run_seal": chained["run_seal"] }));
    assert_eq!(alone["holds"], json!(true), "{alone}");
    assert!(alone.get("recomputed_chain_seal").is_none(), "{alone}");
    let words = alone["chain_compared"].as_str().unwrap();
    assert!(words.contains("No chain_seal was given, so the chain seal was not compared and is not returned"), "{words}");
    // Given and matched, it is returned beside the comparison.
    let both = ok(
        &mut door,
        "verify_tape",
        &json!({ "request": request, "run_seal": chained["run_seal"], "chain_seal": chained["chain"]["chain_seal"] }),
    );
    assert_eq!(both["holds"], json!(true), "{both}");
    assert_eq!(both["recomputed_chain_seal"], chained["chain"]["chain_seal"]);
}

/// Each short check says what it did not compare in true words: a run's and
/// a solution's send a copy to verify_receipt and to recomputed_receipt_sha256
/// together; a dataset's says it compared no rows, that only the head has a
/// receipt, and what the long form does with the decimals of a row.
#[test]
fn each_short_check_says_truly_what_it_did_not_compare() {
    let mut door = Stdio2::start(None);
    let run = thermal_run(10, 293.15);
    let made = ok(&mut door, "run_trajectory", &run);
    let report = ok(&mut door, "verify_tape", &json!({ "request": run, "run_seal": made["run_seal"] }));
    let words = report["text_not_compared"].as_str().unwrap();
    assert!(words.contains("compare that receipt_sha256 with recomputed_receipt_sha256"), "{words}");
    assert!(words.contains("A copy is verified in full only when the two are equal"), "{words}");
    assert!(!words.contains("compares every byte of an answer"), "{words}");
    let solve = json!({ "solver": "atom_dft", "parameters": { "atomic_number": 2 } });
    let solved = ok(&mut door, "solve", &solve);
    let report = ok(&mut door, "verify_solution", &json!({ "request": solve, "solution_seal": solved["solution_seal"] }));
    assert_eq!(report["text_not_compared"].as_str(), Some(words));
    let plan = json!({
        "domain": "energy", "card": "thermal",
        "plan": { "kind": "list", "runs": [plan_run()] },
        "store": "none", "return": "inline"
    });
    let report = ok(&mut door, "verify_dataset", &json!({ "request": plan, "dataset_seal": "0".repeat(64) }));
    let dataset_words = report["text_not_compared"].as_str().unwrap();
    // Until the long form held each decimal of a final state to the hex
    // beside it, the sentence began "No tool compares the decimals in a
    // dataset's rows: only the head carries a receipt".
    assert!(dataset_words.starts_with("The plan was run again and the seals compared; no rows you hold were compared. Only the head carries a receipt"), "{dataset_words}");
    // Since 0.12.0 the long form runs the plan again and compares the full
    // answer byte for byte.
    assert!(dataset_words.contains("then runs its plan again and compares the engine's full answer with the one sent, value by value and byte for byte, naming the first row or field that differs"), "{dataset_words}");
    assert!(!dataset_words.contains("verify_receipt compares every byte"), "{dataset_words}");
    // The tools say the same.
    let listed = serde_json::to_string(&protocol_mcp_gateway::mcp_server::tools_in(&protocol_mcp_gateway::view::View::full())).unwrap();
    // Since 0.11.0 the long form of a run and of a solution compares the copy
    // it is given, and their descriptions say so; since 0.12.0 it computes the
    // copy's request again and compares it byte for byte.
    assert_eq!(listed.matches("the engine's answer compared with it, value by value and byte for byte, receipt included").count(), 2);
    assert!(!listed.contains("verify_receipt covers both"));
}

/// The long form of verify_dataset returns no rebuilt root or seal when the
/// check does not hold.
#[test]
fn the_dataset_long_form_returns_no_rebuilt_seal_that_does_not_hold() {
    let mut door = Stdio2::start(None);
    let plan = json!({
        "domain": "energy", "card": "thermal",
        "plan": { "kind": "list", "runs": [plan_run()] },
        "store": "none", "return": "inline", "answer": "full"
    });
    let made = ok(&mut door, "run_dataset", &plan);
    let whole = json!({ "head": made["head"], "rows": made["rows"], "frames_hex": made["frames_hex"] });
    let held = ok(&mut door, "verify_dataset", &whole);
    assert_eq!(held["holds"], json!(true), "{held}");
    assert_eq!(held["dataset_seal"], made["head"]["dataset_seal"]);
    let mut forged = whole.clone();
    forged["head"]["dataset_seal"] = json!("0".repeat(64));
    let (_, text) = door.tool("verify_dataset", &forged);
    let report = parsed(&text);
    assert_eq!(report["holds"], json!(false), "{report}");
    assert_eq!(report["dataset_seal"], Value::Null, "{report}");
    assert_eq!(report["merkle_root"], Value::Null, "{report}");
    assert!(!text.contains(made["head"]["dataset_seal"].as_str().unwrap()), "{text}");
}

/// The cell_2170_model3 card does not explain a continuation it refuses.
#[test]
fn the_model3_card_does_not_explain_a_continuation_it_refuses() {
    let mut door = Stdio2::start(None);
    let described = ok(&mut door, "describe_card", &json!({ "domain": "energy", "card": "cell_2170_model3" }));
    let text = described.to_string();
    for stale in ["continued from a returned state copies", "carried across a restart", "a continued start copies", "across every continued start"] {
        assert!(!text.contains(stale), "{stale}");
    }
    let link = ok(&mut door, "describe_card", &json!({ "domain": "signal", "card": "link_gothenburg_10010" }));
    let water = link["phase_fields"].as_array().unwrap().iter().find(|row| row["name"] == "antenna_water_db").unwrap()["bound"]
        .as_str()
        .unwrap()
        .to_string();
    assert!(water.contains("it cannot confirm it"), "{water}");
}

/// A solve request that names its solver and neither parameters nor example
/// is answered with a sentence naming both.
#[test]
fn a_solver_alone_is_answered_with_both_routes() {
    let mut door = Stdio2::start(None);
    let text = refused(&mut door, "solve", &json!({ "solver": "atom_dft" }));
    assert_eq!(
        text,
        "Missing required parameter: 'parameters' (or 'example', the name of a worked example describe_solver lists) (InputMissing)."
    );
}
