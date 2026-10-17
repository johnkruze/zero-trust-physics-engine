//! The served contract of every solver, attacked through the local
//! connection from each solver's own description (`conformance_support`).
//!
//! `every_solver_is_attacked_and_its_findings_are_reported` prints every
//! finding and, with `CONFORMANCE_REPORT` set, adds them as JSON lines to
//! that file, after the lines the card sweep wrote there (`report`). It
//! solves each input at both of its bounds only with `CONFORMANCE_BOUNDS`
//! set, since each is a full solve. A new solver's router test holds it to
//! the whole contract with `assert_solver_conforms`.

mod conformance_support;

use conformance_support::*;

#[test]
fn every_solver_is_attacked_and_its_findings_are_reported() {
    let only = std::env::var("CONFORMANCE_ONLY").ok();
    let at_the_bounds = std::env::var("CONFORMANCE_BOUNDS").is_ok();
    let mut door = Door::start();
    let solvers: Vec<_> = verdicts(&mut door).into_iter().filter(|(domain, _, _)| domain.is_empty()).collect();
    assert!(solvers.len() >= 13, "{}", solvers.len());
    let mut lines = Vec::new();
    for (_, solver, verdict) in &solvers {
        if only.as_deref().is_some_and(|only| only != solver) {
            continue;
        }
        let found = solver_findings(&mut door, solver, verdict, at_the_bounds);
        assert!(!found.iter().any(|f| f.rule == "described"), "{solver}: {found:?}");
        for finding in &found {
            println!("{}", finding.line());
            lines.push(finding.json());
        }
        if found.is_empty() {
            println!("{solver} | met every attack");
        }
    }
    let contract = lines.iter().filter(|line| line.contains("\"weight\":\"contract\"")).count();
    println!("{contract} contract findings, {} findings in all, on {} solvers", lines.len(), solvers.len());
    report(&lines);
}
