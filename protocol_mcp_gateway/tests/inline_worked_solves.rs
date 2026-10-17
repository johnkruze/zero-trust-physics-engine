//! What a solver says one inline answer holds of its worked solve is what
//! the engine answers: a plan of that many worked solves (describe_solver's
//! example_solve) is answered in summary, and one more is refused, its
//! answer above the 40000 characters one inline answer may have (breadth
//! test of 2026-10-16, W3: the five solvers whose open_connection said a
//! plan holds the engine's own 64, or 50, of it, where the inline answer
//! held fewer; the red team of 0.14.0 rc1, RT1 W-4: ten more, whose
//! sentence said the local connection admits the engine's own 64). Every
//! other solver's 64 worked solves are answered inline, so a change of
//! digits that makes a sixteenth is named here (INTEGRATION_0.14.md, "Found
//! while merging the words", 2).

mod conformance_support;

use conformance_support::Door;
use serde_json::{Value, json};

/// The number a solver's open_connection says one inline answer holds of
/// its worked solve, when it says one.
fn said_held(open_connection: &str) -> Option<usize> {
    let (before, _) = open_connection.split_once(" of its worked solve (example_solve)")?;
    before.rsplit(' ').next()?.parse().ok()
}

fn said_text<'a>(said: &'a [(String, String)], solver: &str) -> &'a str {
    said.iter().find(|(name, _)| name == solver).map_or("", |(_, text)| text.as_str())
}

#[test]
fn an_inline_answer_holds_the_worked_solves_its_solver_says() {
    let mut door = Door::start();
    let listed = door.value("list_solvers", &json!({})).expect("listed");
    let mut said = Vec::new();
    let mut door_said = Vec::new();
    for row in listed["solvers"].as_array().unwrap() {
        let solver = row["solver"].as_str().unwrap().to_string();
        let described = door.value("describe_solver", &json!({ "solver": solver })).expect("described");
        let open_connection = described["open_connection"].as_str().unwrap().to_string();
        door_said.push((solver.clone(), open_connection.clone()));
        if let Some(held) = said_held(&open_connection) {
            said.push((solver, held, described["example_solve"]["parameters"].clone()));
        }
    }
    let names: Vec<&str> = said.iter().map(|(solver, _, _)| solver.as_str()).collect();
    assert_eq!(names.len(), 15, "{names:?}");
    for (solver, held, parameters) in said {
        let plan = |count: usize| json!({ "solver": solver, "plan": { "kind": "list", "solves": vec![json!({ "parameters": parameters.clone() }); count] } });
        let (error, text) = door.call("run_dataset", &plan(held));
        assert!(!error, "{solver}, {held}: {text}");
        let answer: Value = serde_json::from_str(&text).expect("an answer");
        assert_eq!(answer["summary"].as_array().map(Vec::len), Some(held), "{solver}");
        assert!(text.len() <= 40_000, "{solver}: {}", text.len());
        let (error, text) = door.call("run_dataset", &plan(held + 1));
        assert!(error && text.contains("more than the 40000 one inline answer may have"), "{solver}, {}: {text}", held + 1);
        // One more is solved and then refused, as the sentence says (RT1 W-3 of 0.14.0 rc1).
        assert!(text.contains("Its summary was counted once the plan was solved"), "{solver}, {}: {text}", held + 1);
        assert!(said_text(&door_said, &solver).contains("is otherwise counted once solved, so a plan of a few more of it is solved and then refused"), "{solver}");
    }
}

/// Every solver whose sentence says no inline count: the engine's own 64 of
/// its worked solve are answered inline, as its sentence says the local
/// connection admits them.
#[test]
fn every_other_solver_holds_the_engines_own_64_of_its_worked_solve_inline() {
    let mut door = Door::start();
    let listed = door.value("list_solvers", &json!({})).expect("listed");
    let mut others = Vec::new();
    for row in listed["solvers"].as_array().unwrap() {
        let solver = row["solver"].as_str().unwrap().to_string();
        let described = door.value("describe_solver", &json!({ "solver": solver })).expect("described");
        if said_held(described["open_connection"].as_str().unwrap()).is_none() {
            others.push((solver, described["example_solve"]["parameters"].clone()));
        }
    }
    assert_eq!(others.len(), 9, "{:?}", others.iter().map(|(solver, _)| solver).collect::<Vec<_>>());
    for (solver, parameters) in others {
        let plan = json!({ "solver": solver, "plan": { "kind": "list", "solves": vec![json!({ "parameters": parameters }); 64] } });
        let (error, text) = door.call("run_dataset", &plan);
        assert!(!error, "{solver}, 64: {text}");
        assert!(text.len() <= 40_000, "{solver}: {}", text.len());
        let answer: Value = serde_json::from_str(&text).expect("an answer");
        assert_eq!(answer["summary"].as_array().map(Vec::len), Some(64), "{solver}");
    }
}
