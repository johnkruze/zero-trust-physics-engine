//! The fuel_rod_triga_jsi solver through the local connection. Stamped from the kit:
//! the whole served contract, attacked from the solver's own description by
//! `conformance_support`, and each refusal's sentence word for word from the
//! solver's source. Add the solver's own worked answers below.

mod conformance_support;

use conformance_support::{Door, assert_solver_conforms};
use serde_json::{Value, json};

const SOURCE: &str = include_str!("../../genesis_solvers/src/fuel_rod_triga_jsi.rs");

#[test]
fn fuel_rod_triga_jsi_meets_the_served_contract() {
    assert_solver_conforms("fuel_rod_triga_jsi");
}

#[test]
fn every_refusal_carries_the_sentence_of_its_source() {
    let mut door = Door::start();
    let description: Value = door.value("describe_solver", &json!({ "solver": "fuel_rod_triga_jsi" })).expect("described");
    let start = SOURCE.find("pub enum FuelRodReject {").expect("the enum is in the solver");
    let mut doc = String::new();
    let mut seen = 0;
    for line in SOURCE[start..].lines().skip(1) {
        if line == "}" {
            break;
        }
        let text = line.trim_start();
        if let Some(sentence) = text.strip_prefix("/// ") {
            doc.push_str(sentence);
        } else if text.starts_with(|c: char| c.is_ascii_uppercase()) {
            let name: String = text.chars().take_while(|c| c.is_ascii_alphanumeric()).collect();
            let row = description["refusals"].as_array().unwrap().iter().find(|row| row["name"] == name.as_str()).unwrap_or_else(|| panic!("{name}"));
            assert_eq!(row["meaning"], doc.as_str(), "{name}");
            doc.clear();
            seen += 1;
        }
    }
    // The entry's own door refusals are listed after the solver's.
    assert!(seen > 0 && seen <= description["refusals"].as_array().unwrap().len());
}

#[test]
fn the_worked_question_converges() {
    let mut door = Door::start();
    let description: Value = door.value("describe_solver", &json!({ "solver": "fuel_rod_triga_jsi" })).expect("described");
    let answer = door.value("solve", &description["example_solve"]).expect("answered");
    assert_eq!(answer["outcome"], "converged", "{answer}");
}

/// The worked pulse of the description: 14 MW s at 297 K. The element takes
/// 1.387/56 of it, and every step's account closes.
#[test]
fn the_worked_pulse_gives_its_numbers() {
    let mut door = Door::start();
    let description: Value = door.value("describe_solver", &json!({ "solver": "fuel_rod_triga_jsi" })).expect("described");
    let answer = door.value("solve", &description["example_solve"]).expect("answered");
    let result = &answer["result"];
    let deposited = result["element_energy_deposited_j"].as_f64().unwrap();
    assert!((deposited - 14.0e6 * 1.387 / 56.0).abs() < 1e-6 * deposited, "{deposited}");
    let peak = result["peak_fuel_temperature_k"].as_f64().unwrap();
    assert!(peak > 600.0 && peak < 650.0, "{peak}");
    assert!(answer["invariants"]["largest_step_energy_residual"].as_f64().unwrap() <= 1e-9);
    for flag in ["start_temperature_outside_compared", "core_energy_outside_compared", "core_power_outside_compared", "history_outside_compared"] {
        assert_eq!(result[flag].as_f64(), Some(0.0), "{flag}");
    }
}

/// The card's pulse handed to the element under one chain proof: the card
/// run, then the solve with `from`, then verify_solution and verify_tape
/// hold; a request whose history is not the card's is refused by name.
fn upstream() -> Value {
    json!({
        "domain": "energy", "card": "reactor_triga_jsi",
        "initial": { "start_temperature_k": 296.15, "rod_table_core": 231.0 },
        "dt_s": 5.0e-5,
        "segments": [
            { "steps": 60000, "load": { "reactivity_inserted_dollar": 2.5, "rods_dropped": 0.0 } },
            { "steps": 219000, "load": { "reactivity_inserted_dollar": 2.5, "rods_dropped": 1.0 } }
        ],
        "record_every": 200
    })
}

fn history(card: &Value) -> (Vec<f64>, Vec<f64>) {
    let mut times = Vec::new();
    let mut energy = Vec::new();
    for frame in card["frames"].as_array().unwrap() {
        let hex = frame["frame_hex"].as_str().unwrap();
        let bytes: Vec<u8> = (0..hex.len()).step_by(2).map(|i| u8::from_str_radix(&hex[i..i + 2], 16).unwrap()).collect();
        times.push(f64::from_le_bytes(bytes[0..8].try_into().unwrap()));
        energy.push(f64::from(f32::from_le_bytes(bytes[12..16].try_into().unwrap())));
    }
    (times, energy)
}

#[test]
fn the_card_s_pulse_is_handed_to_the_element_under_one_chain_proof() {
    let mut door = Door::start();
    let up = upstream();
    let card = door.value("run_trajectory", &up).expect("the card runs");
    assert_eq!(card["outcome"], "completed");
    let seal = card["run_seal"].as_str().unwrap().to_string();
    let (times, energy) = history(&card);
    assert_eq!(times.len(), 1395);
    let request = json!({
        "solver": "fuel_rod_triga_jsi",
        "parameters": { "start_temperature_k": 296.15, "time_s": times, "core_energy_released_mws": energy },
        "from": { "request": up, "run_seal": seal, "coupling": "triga_pulse_to_element_6753" }
    });
    let chained = door.value("solve", &request).expect("the chained solve");
    assert_eq!(chained["outcome"], "converged");
    let chain = &chained["chain"];
    assert_eq!(chain["kind"], "coupling");
    assert_eq!(chain["coupling"], "triga_pulse_to_element_6753");
    assert_eq!(chain["upstream_run_seal"], seal.as_str());
    let handed: Vec<&str> = chain["handed"].as_array().unwrap().iter().map(|entry| entry["from"].as_str().unwrap()).collect();
    assert_eq!(handed, ["inputs.initial.start_temperature_k", "frames[*].t", "frames[*].pos[1]"]);
    // The same request typed by hand has the same solution seal and no chain.
    let mut by_hand = request.clone();
    by_hand.as_object_mut().unwrap().remove("from");
    let plain = door.value("solve", &by_hand).expect("by hand");
    assert_eq!(plain["solution_seal"], chained["solution_seal"]);
    assert!(plain.get("chain").is_none());
    // Verified by request: the card run computed again and the solve.
    let verified = door.value("verify_solution", &json!({ "request": request, "solution_seal": chained["solution_seal"] })).expect("verified");
    assert_eq!(verified["holds"], true, "{verified}");
    assert_eq!(verified["recomputed_chain_seal"], chain["chain_seal"]);
    let tape = door.value("verify_tape", &json!({ "request": up, "run_seal": seal })).expect("tape");
    assert_eq!(tape["holds"], true);
    // A history that is not the card's, still rising: refused by name.
    let mut other = request.clone();
    let last = other["parameters"]["core_energy_released_mws"].as_array().unwrap().len() - 1;
    let end = other["parameters"]["core_energy_released_mws"][last].as_f64().unwrap();
    other["parameters"]["core_energy_released_mws"][last] = json!(end + 1.0);
    let (error, text) = door.call("solve", &other);
    assert!(error && text.contains("(CouplingLedgerBreach)"), "{text}");
    // A seal that is not the card run's: refused by name.
    let mut forged = request.clone();
    forged["from"]["run_seal"] = json!("0".repeat(96));
    let (error, text) = door.call("solve", &forged);
    assert!(error && text.contains("(UpstreamProofBroken)"), "{text}");
}

/// The full check of a chained solve: the whole answer with the `from` solve
/// took. A true answer holds with its chain seal recomputed; a forged
/// upstream seal and a tampered handed array fail by name; the answer alone
/// is refused, as a chained run's is.
#[test]
fn the_full_check_of_a_chained_solve_holds_and_names_every_forgery() {
    let mut door = Door::start();
    let up = upstream();
    let card = door.value("run_trajectory", &up).expect("the card runs");
    let seal = card["run_seal"].as_str().unwrap().to_string();
    let (times, energy) = history(&card);
    let from = json!({ "request": up, "run_seal": seal, "coupling": "triga_pulse_to_element_6753" });
    let request = json!({
        "solver": "fuel_rod_triga_jsi",
        "parameters": { "start_temperature_k": 296.15, "time_s": times, "core_energy_released_mws": energy },
        "from": from
    });
    let (error, chained_text) = door.call("solve", &request);
    assert!(!error, "{chained_text}");
    let chained: Value = serde_json::from_str(&chained_text).unwrap();
    let with_from = |answer: &Value, from: &Value| {
        let mut whole = answer.clone();
        whole.as_object_mut().unwrap().insert("from".into(), from.clone());
        whole
    };
    // A true chained answer, sent back as it came with its from, holds in the full check.
    let whole = format!("{},\"from\":{}}}", &chained_text[..chained_text.len() - 1], from);
    let (error, verified) = door.call_text("verify_solution", &whole);
    assert!(!error, "{verified}");
    let verified: Value = serde_json::from_str(&verified).unwrap();
    assert_eq!((verified["holds"].clone(), verified["values_hold"].clone(), verified["cause"].clone()), (json!(true), json!(true), Value::Null), "{verified}");
    assert_eq!(verified["recomputed_chain_seal"], chained["chain"]["chain_seal"]);
    assert!(verified["chain_verified"].as_str().unwrap().contains("byte for byte"), "{verified}");
    // The answer alone carries no upstream request: refused by name.
    let (error, text) = door.call("verify_solution", &chained);
    assert!(error && text.contains("(AnswerCarriesNoRequest)"), "{text}");
    // A forged upstream seal: the card run computed again is not it.
    let mut forged = from.clone();
    forged["run_seal"] = json!("0".repeat(96));
    let report = door.value("verify_solution", &with_from(&chained, &forged)).expect("a report");
    assert_eq!((report["holds"].clone(), report["cause"].clone(), report["differs_at"].clone()), (json!(false), json!("chain"), json!("from")), "{report}");
    assert!(report["difference"].as_str().unwrap().contains("(UpstreamProofBroken)"), "{report}");
    assert!(report.get("recomputed_chain_seal").is_none(), "{report}");
    // The upstream seal the answer's chain names, rewritten: not the engine's.
    let mut renamed = chained.clone();
    renamed["chain"]["upstream_run_seal"] = json!("1".repeat(96));
    let report = door.value("verify_solution", &with_from(&renamed, &from)).expect("a report");
    assert_eq!((report["holds"].clone(), report["cause"].clone(), report["differs_at"].clone()), (json!(false), json!("answer_text"), json!("chain.upstream_run_seal")), "{report}");
    // A tampered handed array under a seal that holds: the request solved by
    // hand with one energy changed, claiming the true chain.
    let mut tampered = request.clone();
    tampered.as_object_mut().unwrap().remove("from");
    let last = tampered["parameters"]["core_energy_released_mws"].as_array().unwrap().len() - 1;
    let end = tampered["parameters"]["core_energy_released_mws"][last].as_f64().unwrap();
    tampered["parameters"]["core_energy_released_mws"][last] = json!(end * 1.01);
    let mut resealed = door.value("solve", &tampered).expect("solved by hand");
    resealed["chain"] = chained["chain"].clone();
    let report = door.value("verify_solution", &with_from(&resealed, &from)).expect("a report");
    assert_eq!((report["holds"].clone(), report["cause"].clone(), report["differs_at"].clone()), (json!(false), json!("chain"), json!("from")), "{report}");
    assert!(report["difference"].as_str().unwrap().contains("(CouplingLedgerBreach)"), "{report}");
    // The same array changed under the seal it came with: its seal part fails.
    let mut stale = chained.clone();
    stale["inputs"]["parameters"]["core_energy_released_mws"][last] = json!(end * 1.01);
    let report = door.value("verify_solution", &with_from(&stale, &from)).expect("a report");
    assert_eq!((report["holds"].clone(), report["cause"].clone()), (json!(false), json!("inputs")), "{report}");
}
