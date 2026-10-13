//! The scored run of `pipe_flow_superpipe`: one question per input line,
//! `bulk_reynolds_number eta_1 eta_2 ...` (wall distances over the radius),
//! one JSON object per output line with the friction factor, the centreline
//! ratio and the velocity over the bulk velocity at each `eta`. It asks
//! `velocity_over_bulk_at`, which settles the served flow without the range
//! check, because the range is set from the compared runs.
use std::io::{self, BufRead, Write};

fn main() {
    let stdin = io::stdin();
    let mut out = io::stdout().lock();
    for line in stdin.lock().lines() {
        let line = line.expect("line");
        let mut words = line.split_whitespace();
        let Some(first) = words.next() else { continue };
        let re: f64 = first.parse().expect("Reynolds number");
        let eta: Vec<f64> = words.map(|w| w.parse().expect("eta")).collect();
        match genesis_solvers::pipe_flow_superpipe::velocity_over_bulk_at(re, &eta) {
            Ok((flow, u)) => {
                let list = |v: &[f64]| v.iter().map(|x| format!("{x:e}")).collect::<Vec<_>>().join(",");
                writeln!(
                    out,
                    "{{\"re\":{re:e},\"f\":{:e},\"ucl\":{:e},\"karman_number\":{:e},\"iterations\":{},\"imbalance\":{:e},\"ledger\":{:e},\"flux\":{:e},\"eta\":[{}],\"u_over_bulk\":[{}]}}",
                    flow.darcy_friction_factor,
                    flow.centreline_to_bulk_velocity_ratio,
                    flow.karman_number,
                    flow.newton_iterations,
                    flow.largest_imbalance,
                    flow.momentum_ledger_residual,
                    flow.flux_residual,
                    list(&eta),
                    list(&u)
                )
                .expect("write");
            }
            Err(e) => writeln!(out, "{{\"re\":{re:e},\"refused\":\"{e}\"}}").expect("write"),
        }
    }
}
