//! A plan of solves the open connection does not start: one of more solves
//! than end inside the time for an answer.
//!
//! A plan of solves is computed one solve after another under one permit,
//! and the open connection waits a fixed time for its answer. So a plan is
//! counted before it starts, each solve at its solver's measured time at the
//! slowest input of its range, and one that holds more than
//! [`OPEN_PLAN_SECONDS`] of them is refused by name, from the request alone,
//! before any physics permit is taken: in `run_dataset`, in `verify_dataset`
//! by request, and in `validate_series` given `solves`.
//!
//! The limit is a named constant, said in the limits an agent reads when the
//! connection opens. No other connection holds it: the local connection and
//! a connection with a key take the engine's own limit of solves in a plan.
//! A plan that runs longer than it was counted at is ended at the
//! connection's time for an answer (`src/long_work.rs`).
//!
//! The open connection holds no floor of its own on any solver's input. An
//! input far below anything physical is refused by the solver itself, on
//! every connection, in the solver's words and before anything is computed:
//! `fusion_pb11_lhd` a boron density that is not zero and is under one
//! nucleus in the plasma (`BoronBelowOneNucleus`), and `ablator_slab` a
//! history row less than a millisecond after the row before it
//! (`HistoryRowsTooClose`).

use serde_json::{Map, Value};

use crate::view::View;

/// The seconds of solving one plan of solves may be counted at. A plan is
/// computed one solve after another under one permit, and the connection
/// waits 60 s for its answer. Each solve is counted at its solver's measured
/// time at the slowest input of its range on the machine the limits were
/// measured on ([`OPEN_SOLVE_SECONDS`]), and a plan at no more than this, a
/// quarter of the wait, so that it ends inside the wait on a machine four
/// times slower. The engine's own limit of solves in a plan holds as well.
pub const OPEN_PLAN_SECONDS: f64 = 15.0;

/// What one solve of each solver is counted at, in seconds: its measured
/// time at the slowest input of its range that is not a refusal, on one core
/// of an Apple M4 Pro on 2026-10-12. `step_flow_armaly` at Reynolds number
/// 389; `tokamak_dt_beam_target` with 128 shells and 12 beam components;
/// `ablator_slab` on an arc-jet history of a few rows (a history with a
/// corner at every row takes longer, and is ended at the time for an
/// answer); `atom_dft` for argon. A solver not listed is counted at
/// [`OPEN_SOLVE_SECONDS_UNLISTED`], the slowest listed, so that a solver
/// added later is held until it is measured; a unit test holds the list to
/// the served solvers.
pub const OPEN_SOLVE_SECONDS: &[(&str, f64)] = &[
    ("step_flow_armaly", 3.0),
    ("tokamak_dt_beam_target", 1.0),
    ("ablator_slab", 1.0),
    ("atom_dft", 0.5),
    ("journal_bearing_sheffield", 0.15),
    ("argon_pair_gas", 0.15),
    ("fusion_pb11_lhd", 0.1),
    ("seawater_column", 0.06),
    ("fusion_pb11_thermal", 0.05),
    ("culture_trichoderma_ambr250", 0.01),
    ("pipe_flow_superpipe", 0.01),
    ("propeller_apc_10x7e", 0.01),
    ("modulus_bam_s355", 0.01),
];
pub const OPEN_SOLVE_SECONDS_UNLISTED: f64 = 3.0;

fn solve_seconds(solver: &str) -> f64 {
    OPEN_SOLVE_SECONDS.iter().find(|(name, _)| *name == solver).map_or(OPEN_SOLVE_SECONDS_UNLISTED, |(_, seconds)| *seconds)
}

/// The most solves of `solver` one plan may hold on the open connection.
pub fn open_plan_solves(solver: &str) -> u64 {
    ((OPEN_PLAN_SECONDS / solve_seconds(solver)).floor() as u64).clamp(1, crate::solve_plan::MAX_SOLVES)
}

/// The limit on a plan of solves in a sentence, for the limits an agent
/// reads when the open connection opens. It begins with a space.
pub fn statement() -> String {
    // The plans of solves that hold fewer than the engine's own limit.
    let fewer: Vec<String> = OPEN_SOLVE_SECONDS
        .iter()
        .filter(|(solver, _)| open_plan_solves(solver) < crate::solve_plan::MAX_SOLVES)
        .map(|(solver, _)| format!("{} of {solver}", open_plan_solves(solver)))
        .collect();
    format!(
        " A plan of solves is counted before it starts, each solve at its solver's measured time at the slowest input of its range, and holds no more than {OPEN_PLAN_SECONDS} s of them (OPEN_PLAN_SECONDS): at most {}, and the engine's own {} of every other solver.",
        fewer.join(", "),
        crate::solve_plan::MAX_SOLVES
    )
}

/// A plan of solves, as `run_dataset` takes it, counted before it starts. A
/// plan that cannot be read is refused by the engine in its own words, and a
/// row its solver refuses before computing is `unadmitted` in the plan's
/// answer, in the solver's words.
fn of_plan(request: &Map<String, Value>) -> Option<String> {
    let solver = request.get("solver")?.as_str()?.to_string();
    let normalized = crate::dataset::normalize(Value::Object(request.clone())).ok()?;
    let plan = crate::solve_plan::read(&View::full(), normalized.request).ok()?;
    let (asked, most) = (plan.rows().len() as u64, open_plan_solves(&solver));
    (asked > most).then(|| {
        format!(
            "Physical Violation: the plan asks for {asked} solves of {solver}, more than the {most} one plan of it may have on the open connection (OPEN_PLAN_SECONDS): a solve of {solver} is counted at {} s, its measured time at the slowest input of its range, and a plan at no more than {OPEN_PLAN_SECONDS} s, so that it ends inside the time for an answer. Send it as plans of {most} or fewer: each solve's proof is the same either way, and a dataset proof is the proof of the plan sent. Nothing was computed and no proof is issued.",
            solve_seconds(&solver)
        )
    })
}

/// The sentence for a tool call the open connection does not start, or
/// nothing.
pub(crate) fn fault(tool: &str, arguments: &Map<String, Value>) -> Option<String> {
    let by_request = || arguments.get(crate::check_by_request::REQUEST).and_then(Value::as_object);
    match tool {
        "run_dataset" => of_plan(arguments),
        "verify_dataset" => by_request().and_then(of_plan),
        "validate_series" => arguments.get("solves").and_then(Value::as_object).and_then(of_plan),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn object(value: Value) -> Map<String, Value> {
        value.as_object().unwrap().clone()
    }

    fn lhd(boron: Value) -> Value {
        let mut request = crate::solvers::example_solve(crate::solvers::find("fusion_pb11_lhd").unwrap());
        request["parameters"]["boron_density_per_m3"] = boron;
        request
    }

    #[test]
    fn a_plan_of_more_solves_than_end_in_time_is_refused_by_name_before_it_starts() {
        // Every served solver is measured, and the slow ones hold fewer solves than the engine's own limit.
        for solver in crate::solvers::ALL.iter().filter(|solver| solver.name != crate::solver_probe::STAND_IN.name) {
            assert!(OPEN_SOLVE_SECONDS.iter().any(|(name, _)| *name == solver.name), "{} has no measured time", solver.name);
        }
        for (name, _) in OPEN_SOLVE_SECONDS {
            assert!(crate::solvers::find(name).is_some(), "{name} is not a served solver");
        }
        assert_eq!(
            ["step_flow_armaly", "tokamak_dt_beam_target", "ablator_slab", "atom_dft", "journal_bearing_sheffield", "propeller_apc_10x7e"].map(open_plan_solves),
            [5, 15, 15, 30, 64, 64]
        );
        assert_eq!(open_plan_solves("a_solver_not_yet_measured"), 5);
        let step = crate::solvers::example_solve(crate::solvers::find("step_flow_armaly").unwrap());
        let plan = |solves: usize| json!({ "solver": "step_flow_armaly", "plan": { "kind": "list", "solves": vec![json!({ "parameters": step["parameters"] }); solves] } });
        assert_eq!(fault("run_dataset", &object(plan(5))), None);
        let sentence = fault("run_dataset", &object(plan(6))).expect("refused");
        assert_eq!(
            sentence,
            "Physical Violation: the plan asks for 6 solves of step_flow_armaly, more than the 5 one plan of it may have on the open connection (OPEN_PLAN_SECONDS): a solve of step_flow_armaly is counted at 3 s, its measured time at the slowest input of its range, and a plan at no more than 15 s, so that it ends inside the time for an answer. Send it as plans of 5 or fewer: each solve's proof is the same either way, and a dataset proof is the proof of the plan sent. Nothing was computed and no proof is issued."
        );
        // The same plan checked by request, and compared with measurements.
        assert_eq!(fault("verify_dataset", &object(json!({ "request": plan(6), "dataset_seal": "ab".repeat(32) }))), Some(sentence.clone()));
        assert_eq!(fault("validate_series", &object(json!({ "solves": plan(6), "quantity": "reattachment_length_step_heights", "measurements": [] }))), Some(sentence));
        // A fast solver keeps the engine's own limit, and a plan of runs on a card is not counted here.
        let bar = crate::solvers::example_solve(crate::solvers::find("propeller_apc_10x7e").unwrap());
        let many = json!({ "solver": "propeller_apc_10x7e", "plan": { "kind": "list", "solves": vec![json!({ "parameters": bar["parameters"] }); 64] } });
        assert_eq!(fault("run_dataset", &object(many)), None);
        assert_eq!(fault("run_dataset", &object(json!({ "domain": "energy", "card": "thermal", "plan": { "kind": "list", "runs": [] } }))), None);
        let said = statement();
        assert!(said.contains("holds no more than 15 s of them (OPEN_PLAN_SECONDS): at most 5 of step_flow_armaly, 15 of tokamak_dt_beam_target, 15 of ablator_slab, 30 of atom_dft, and the engine's own 64 of every other solver."), "{said}");
    }

    /// The open connection turns away no solve for the value of an input:
    /// the two inputs it once held to floors of its own are held by the
    /// solvers, which refuse them on every connection in their own words.
    #[test]
    fn the_open_connection_holds_no_floor_of_its_own_on_a_solvers_input() {
        let vanishing = lhd(json!(1.0e-297));
        let history = json!({ "solver": "ablator_slab", "parameters": { "history_time_s": [0.0, 1.0e-300, 10.0] } });
        for (request, vary, low, high) in [
            (&vanishing, "parameters.boron_density_per_m3", 0.0, 1.0e17),
            (&history, "parameters.history_time_s[1]", 1.0e-9, 5.0),
        ] {
            assert_eq!(fault("solve", &object(request.clone())), None, "{request}");
            assert_eq!(fault("verify_solution", &object(json!({ "request": request, "solution_seal": "ab".repeat(32) }))), None, "{request}");
            let flip = json!({ "request": request, "vary": vary, "low": low, "high": high });
            assert_eq!(fault("find_flip", &object(flip)), None, "{request}");
        }
        // A plan with such a row is counted like any other plan; the row is the solver's to refuse, and is unadmitted.
        let plan = json!({ "solver": "fusion_pb11_lhd", "plan": { "kind": "list", "solves": [
            { "parameters": lhd(json!(8.6706e16))["parameters"] }, { "parameters": vanishing["parameters"] }
        ] } });
        assert_eq!(fault("run_dataset", &object(plan)), None);
        // The solver's own check names it, before anything is computed.
        let lhd_solver = crate::solvers::find("fusion_pb11_lhd").unwrap();
        let given = crate::solution::read_parameters(lhd_solver, vanishing["parameters"].as_object().unwrap());
        let sentence = given.err().expect("the solver refuses it").message().to_string();
        assert!(sentence.ends_with("less than one nucleus is no boron, and zero is answered (BoronBelowOneNucleus)."), "{sentence}");
        // The limits name the plan limit and no floor.
        let said = statement();
        assert!(said.starts_with(" A plan of solves is counted before it starts"), "{said}");
        assert!(said.contains("OPEN_PLAN_SECONDS"), "{said}");
        for gone in ["FLOOR", "boron", "history_time_s", "below anything physical", "caller"] {
            assert!(!said.contains(gone), "{gone}: {said}");
        }
    }
}
