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
//! The limit is a named constant. The rule is said once, in the same words
//! for any number of solvers, in the description of each tool that takes a
//! plan of solves on the open connection and in the limits the open door
//! prints when it starts; each solver's own count, its measured time and the
//! solves of it one plan holds, is said in that solver's description
//! (`describe_solver`, `open_connection`), so no tool description grows with
//! the solvers served. No other connection holds it: the local connection and a connection with a key take the engine's
//! own limit of solves in a plan. A plan that runs longer than it was counted
//! at is ended at the connection's time for an answer (`src/long_work.rs`).
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
/// time at the slowest input of its range that is not a refusal, with every
/// history and return it can give asked for, on one core of an Apple M4 Pro
/// (2026-10-12 to 16), with room above it. `junction_nist_b7_s2` at the most
/// steps one solve takes, 117.6 million in 17.2 s (2026-10-15; its budget
/// of steps holds about 18 s, 19 s with canonical math); `step_flow_armaly` at Reynolds number 400,
/// its top since 0.12.0 (2.13 to 2.16 s); `tokamak_dt_beam_target` with 128 shells and 12 beam
/// components at 1000 keV, its top since beams opened to ITER's energy, in
/// its slowest corner, electrons at 0.05 keV and ions at 0.2 keV: about 60 s
/// a shell measured on 2026-10-14 (59.4 s for one shell, more than 300 s for
/// eight), so about 7,700 s for 128, counted at 10,000 s (the same request at
/// 200 keV takes about 2.4 s a shell, 310 s; at 1 keV ions and electrons
/// 6.9 s at 200 keV and 36 s at 1000 keV); `fusion_pb11_thermal` at its slowest corner
/// of 1 to 1000 keV, the 1000 keV one, 0.19 s on 2026-10-14, counted at 0.4 s;
/// `fusion_pb11_lhd` at its slowest corner, 0.09 s, counted at 0.2 s;
/// `icf_hotspot_burn` on the edge of ignition, where its step is halved
/// (6.25 s at 15 keV, 1000 Gbar, 40 um, 0.8 g/cm^2, 0.2 mg, on 2026-10-14);
/// `ablator_slab` at its slowest found, a stream of an hour on 1 m of PICA,
/// 256 rows each a corner (379 s on the fix pass of 0.14.0, 2026-10-16,
/// counted at 450 s; it was counted at 1 s, an arc-jet history of a few
/// rows, until the breadth tests found its held face of an hour and an
/// hour after at 75 s); `ln2_line_dlr_fttf` at its 10 s budget of transient, 20 MPa at
/// 0.5 kg/s and cavitating at the coldest liquid (1.8 to 1.95 s,
/// 2026-10-15); `stress_plate_hole_s235` at 139.81 MPa, the highest traction
/// answered, with 1,000 points of history (1.66 s); `stress_plate_hole_s235_elastic`
/// at any traction it answers with 1,000 points, the same at every traction
/// (0.16 s on 2026-10-15, counted at 0.3 s); `culture_trichoderma_ambr250`
/// at its 1000 h budget with every halving taken (0.68 s on one thread, to
/// 1.8 s on a loaded machine); `culture_trichoderma_atp` the same, a 1000 h
/// record taking 0.19 s at six halvings (2026-10-15), about 0.75 s with all
/// eight; `blast_wave_tnt_air` at the near-field limit,
/// 0.52 s; `plasma_sheath_ram_c2` at sea level and 12 km/s (0.94 s, on
/// 2026-10-15); `water_hammer_oil_line` 1 s of transient with air at 1 or
/// 10 MPa and every history (0.26 s); `orifice_air_chamber` 20 s of charge or
/// vent with every history (0.24 to 0.26 s); `atom_dft` for argon. Measured
/// again on 2026-10-16 with canonical math (genesis_solvers 0.12.0 computes
/// with genesis_kernel::math), the frozen 0.13.0 candidate beside it on the
/// same machine, the median of five after a warm-up: `icf_hotspot_burn`
/// 8.32 s against 5.95 s at the input above, `plasma_sheath_ram_c2` 1.52 s
/// against 0.90 s, `water_hammer_oil_line` 0.39 s against 0.26 s,
/// `pneumatic_hose_resonance` 0.057 s against 0.034 s, `blast_wave_tnt_air`
/// 1.10, `ln2_line_dlr_fttf` 1.04 and `junction_nist_b7_s2` 1.05 times
/// (a step under a drive), each row set again with room above it;
/// `tokamak_dt_beam_target` one shell of its slowest corner 56.8 s against
/// 56.8 s, and every other row inside its count. A solver
/// not listed is counted at [`OPEN_SOLVE_SECONDS_UNLISTED`], the slowest
/// listed, so that a solver added later is held until it is measured; a unit
/// test holds the list to the served solvers.
pub const OPEN_SOLVE_SECONDS: &[(&str, f64)] = &[
    ("tokamak_dt_beam_target", 10_000.0),
    ("junction_nist_b7_s2", 19.0), // 117.6 million steps: 17.2 s, 18.1 s with canonical math
    ("icf_hotspot_burn", 8.7), // on the edge of ignition, its step halved: 6.25 s, 8.32 s with canonical math
    ("fuel_rod_triga_jsi", 3.5), // 600 s of history in 60,000 points, its capacity, 300,000 steps: 2.79 s under load
    ("step_flow_armaly", 3.0), // at Reynolds number 400: 2.16 s
    ("ln2_line_dlr_fttf", 2.1), // 10 s of transient at 20 MPa: 1.95 s, 2.03 s with canonical math
    ("stress_plate_hole_s235", 1.7), // at 139.81 MPa, the highest traction answered, 1,000 points: 1.66 s
    ("stress_plate_hole_s235_elastic", 0.3), // elastic at any traction answered, 1,000 points: 0.16 s
    ("plasma_sheath_ram_c2", 1.6), // at sea level and 12 km/s, the slowest of a 90-point sweep: 0.94 s, 1.52 s with canonical math
    ("culture_trichoderma_ambr250", 1.5), // a 1000 h record with every halving: 0.68 s, 1.8 s under load
    ("culture_trichoderma_atp", 1.5), // a 1000 h record: 0.19 s at six halvings, about 0.75 s with all eight
    ("ablator_slab", 450.0), // an hour of stream on 1 m, 256 rows each a corner: 379 s; the held face, an hour and an hour after: 74 to 76 s
    ("blast_wave_tnt_air", 0.6), // at the near-field limit: 0.52 s, 0.54 s with canonical math
    ("atom_dft", 0.5),
    ("water_hammer_oil_line", 0.42), // 1 s of transient with air and every history: 0.26 s, 0.39 s with canonical math
    ("fusion_pb11_thermal", 0.4),
    ("orifice_air_chamber", 0.3), // 20 s of charge or vent with every history: 0.26 s, 0.27 s with canonical math
    ("fusion_pb11_lhd", 0.2), // its slowest corner: 0.09 s, 0.12 s with canonical math
    ("journal_bearing_sheffield", 0.15),
    ("argon_pair_gas", 0.15),
    ("pneumatic_hose_resonance", 0.065), // 64 frequencies to the cut-on at the tube's rating: 0.039 s, 0.057 s with canonical math
    ("seawater_column", 0.06),
    ("pipe_flow_superpipe", 0.01),
    ("propeller_apc_10x7e", 0.01),
    ("modulus_bam_s355", 0.01),
];
pub const OPEN_SOLVE_SECONDS_UNLISTED: f64 = 10_000.0;

/// The solvers whose worked solve (`describe_solver`'s `example_solve`) one
/// inline answer holds fewer of than the engine's own limit of solves in a
/// plan, and how many it holds: `run_dataset` answers a plan of that many
/// worked solves in summary and refuses one more, its answer above the
/// [`crate::dataset::MAX_INLINE_ANSWER_CHARS`] one inline answer may have,
/// on every connection where the plan is admitted. The first five hold fewer
/// than the open connection's count admits; the other ten hold at least that
/// count, and fewer than the 64 the local connection and a connection with a
/// key admit, where a plan of solves is answered only inline (the red team of
/// 0.14.0 rc1, RT1 W-4: their sentence said those connections "admit the
/// engine's own 64", and 64 of their worked solve were refused inline there).
/// Measured on 0.14.0 rc2's build, every solver from a plan of one and of two
/// and then held at N and N + 1 (rc2_014/measure_rc2/inline/inline_counts_item01.json:
/// ablator_slab 24, 39241 characters, 25 at 40834; fusion_pb11_thermal 37,
/// 39422, 38 at 40460; tokamak_dt_beam_target 22, 38991, 23 at 40717;
/// orifice_air_chamber 57, 39434, 58 at 40108; blast_wave_tnt_air 42, 39697,
/// 43 at 40618; stress_plate_hole_s235 38, 39171, 39 at 40175;
/// stress_plate_hole_s235_elastic 51, 39328, 52 at 40079; icf_hotspot_burn 16,
/// 38789, 17 at 41150; ln2_line_dlr_fttf 25, 38764, 26 at 40274;
/// plasma_sheath_ram_c2 17, 39692, 18 at 41967; the first five as before; the
/// other nine solvers' 64 are answered), as RT1 measured them on rc1
/// (rt1/out/t10_inline_measure.json). Measured on 2026-10-16 with the build of
/// feat/words-014 (evidence/verification/words_014_2026-10-16/logs/
/// inline_solves_five.log; every other solver's worked plan at its count is
/// answered, inline_solves_others.log), and again on the build of next-0.14
/// with those words merged, canonical math's (integration_014's
/// measure_014w/inline_solves_merge.txt): the same four, and
/// water_hammer_oil_line 31, where its 32 come to 40058 characters (41146
/// for 33 before); the test
/// `an_inline_answer_holds_the_worked_solves_its_solver_says` holds each
/// number to the engine.
pub const INLINE_WORKED_SOLVES: &[(&str, u64)] = &[
    ("modulus_bam_s355", 55),
    ("seawater_column", 60),
    ("argon_pair_gas", 52),
    ("fusion_pb11_lhd", 53),
    ("water_hammer_oil_line", 31),
    ("ablator_slab", 24),
    ("fusion_pb11_thermal", 37),
    ("tokamak_dt_beam_target", 22),
    ("orifice_air_chamber", 57),
    ("blast_wave_tnt_air", 42),
    ("stress_plate_hole_s235", 38),
    ("stress_plate_hole_s235_elastic", 51),
    ("icf_hotspot_burn", 16),
    ("ln2_line_dlr_fttf", 25),
    ("plasma_sheath_ram_c2", 17),
    // Its worked solve answers the thermocouple and the fuel at each of the
    // history's 300 times: one inline answer holds 10 (37,145 characters), and 11
    // are refused (2026-10-18).
    ("fuel_rod_triga_jsi", 10),
];

pub(crate) fn solve_seconds(solver: &str) -> f64 {
    OPEN_SOLVE_SECONDS.iter().find(|(name, _)| *name == solver).map_or(OPEN_SOLVE_SECONDS_UNLISTED, |(_, seconds)| *seconds)
}

/// The most solves of `solver` one plan may hold on the open connection.
pub fn open_plan_solves(solver: &str) -> u64 {
    ((OPEN_PLAN_SECONDS / solve_seconds(solver)).floor() as u64).clamp(1, crate::solve_plan::MAX_SOLVES)
}

/// The limit on a plan of solves in a sentence, for the limits the open
/// connection prints when it starts. It begins with a space. It names no
/// solver: each solver's count is in its own description
/// ([`said_of_solver`]), so the sentence is the same for any number of
/// solvers.
pub fn statement() -> String {
    format!(
        " A plan of solves is counted before it starts, each solve at its solver's measured time at the slowest input of its range, and holds no more than {OPEN_PLAN_SECONDS} s of them (OPEN_PLAN_SECONDS), or one solve of a solver counted at more, and the engine's own {} solves; each solver's description states its time and how many of its solves one plan holds (describe_solver, open_connection).",
        crate::solve_plan::MAX_SOLVES
    )
}

/// The same limit as one clause with no full stop, for the description of
/// each tool that takes a plan of solves on the open connection. Like
/// [`statement`] it names no solver, so a description that holds it stays
/// the same length for any number of solvers served.
pub(crate) fn said_in_a_description() -> String {
    format!(
        "a plan of solves holds the engine's own {} and at most {OPEN_PLAN_SECONDS} s of them, or one, at each solver's measured time, which describe_solver states with the count (OPEN_PLAN_SECONDS)",
        crate::solve_plan::MAX_SOLVES
    )
}

/// What a solver's own description says of the open connection's limit on
/// a plan of solves: the time one solve of it is counted at and the most
/// solves of it one plan holds there. `describe_solver` gives it as
/// `open_connection` on every connection, the same bytes everywhere.
pub fn said_of_solver(solver: &str) -> String {
    let (seconds, most) = (solve_seconds(solver), open_plan_solves(solver));
    // Where one inline answer holds fewer of its worked solve than a plan
    // admits, the sentence says how many it holds, and when the answer is
    // counted: before solving only where the least its summary can come to
    // is already above the limit, and otherwise once the plan is solved
    // (solve_plan.rs, too_large_before and too_large_after), so a plan of a
    // few more is solved and then refused. Until 0.14.0 rc2 it said "counted
    // from the plan" (the red team of rc1, RT1 W-3).
    let inline_held = INLINE_WORKED_SOLVES.iter().find(|(name, _)| *name == solver).map(|(_, held)| *held);
    let inline_words = |held: u64| {
        format!(
            "an answer returned inline is at most {} characters and holds {held} of its worked solve (example_solve) in summary",
            crate::dataset::MAX_INLINE_ANSWER_CHARS
        )
    };
    let counted = ": a plan is refused before solving only where the least its summary can come to is more, and is otherwise counted once solved, so a plan of a few more of it is solved and then refused";
    let held = if most < crate::solve_plan::MAX_SOLVES {
        match inline_held {
            // The count binds first on the open connection; on the others
            // the engine's own 64 are admitted, and an inline answer, the
            // only answer of a plan of solves, holds fewer (RT1 W-4).
            Some(held) if held >= most => format!(
                "so the count admits at most {most} of it in one plan there; the local connection and a connection with a key admit the engine's own {}, and there {}{counted}",
                crate::solve_plan::MAX_SOLVES,
                inline_words(held)
            ),
            _ => format!(
                "so the count admits at most {most} of it in one plan there; the local connection and a connection with a key admit the engine's own {}",
                crate::solve_plan::MAX_SOLVES
            ),
        }
    } else {
        format!("so the count admits the engine's own {most} of it in one plan there, as on every connection")
    };
    let inline = match inline_held {
        Some(held) if held < most => format!("; {}, on every connection{counted}", inline_words(held)),
        _ => String::new(),
    };
    format!(
        "On the open connection a plan of solves is counted before it starts and holds no more than {OPEN_PLAN_SECONDS} s of solving (OPEN_PLAN_SECONDS), or one solve of a solver counted at more: a solve of {solver} is counted at {seconds} s, its measured time at the slowest input of its range, {held}{inline}."
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
    let by_request = || arguments.get(crate::verify_by_request::REQUEST).and_then(Value::as_object);
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

    /// ablator_slab's count is at least its slowest solve inside its declared
    /// ranges (the breadth tests of 16 October 2026, random_conformance D4:
    /// it was counted at 1 s, an arc-jet history of a few rows, and its worked
    /// held face, 1 m held an hour and an hour after, took 103 s under load).
    /// Measured on the fix pass's release build, one solve alone, on an Apple
    /// M4 Pro at a load average of 3 to 8 from other work: the held face at
    /// its slowest corners 74 to 76 s (148,102 steps at every one), and a
    /// stream of an hour on 1 m of PICA, 256 rows each a corner, 379 s at a
    /// heat-transfer coefficient alternating 0 and 0.2 kg/m2/s and 320 s at
    /// 0.4 (161,795 and 162,654 steps; fix_014/measure_fix/ablator_timing.jsonl
    /// and ablator_hour_*). So one plan holds one of it on the open connection,
    /// admitted alone as a solve counted above OPEN_PLAN_SECONDS, in the
    /// words of 0.14.0 ("the count admits at most 1 of it in one plan there").
    #[test]
    fn ablator_slab_is_counted_at_its_slowest_solve() {
        assert!(solve_seconds("ablator_slab") >= 379.3, "{}", solve_seconds("ablator_slab"));
        assert!(solve_seconds("ablator_slab") > OPEN_PLAN_SECONDS);
        assert_eq!(open_plan_solves("ablator_slab"), 1);
        assert!(said_of_solver("ablator_slab").contains("or one solve of a solver counted at more: a solve of ablator_slab is counted at 450 s"), "{}", said_of_solver("ablator_slab"));
        assert!(said_of_solver("ablator_slab").contains("so the count admits at most 1 of it in one plan there"), "{}", said_of_solver("ablator_slab"));
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
            ["step_flow_armaly", "tokamak_dt_beam_target", "ablator_slab", "atom_dft", "fusion_pb11_thermal", "fusion_pb11_lhd", "journal_bearing_sheffield", "propeller_apc_10x7e"].map(open_plan_solves),
            [5, 1, 1, 30, 37, 64, 64, 64]
        );
        assert_eq!(open_plan_solves("a_solver_not_yet_measured"), 1);
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
        // The rule is said once and names no solver; each solver's count is in its own description.
        assert_eq!(statement(), " A plan of solves is counted before it starts, each solve at its solver's measured time at the slowest input of its range, and holds no more than 15 s of them (OPEN_PLAN_SECONDS), or one solve of a solver counted at more, and the engine's own 64 solves; each solver's description states its time and how many of its solves one plan holds (describe_solver, open_connection).");
        assert_eq!(said_in_a_description(), "a plan of solves holds the engine's own 64 and at most 15 s of them, or one, at each solver's measured time, which describe_solver states with the count (OPEN_PLAN_SECONDS)");
        assert_eq!(said_of_solver("step_flow_armaly"), "On the open connection a plan of solves is counted before it starts and holds no more than 15 s of solving (OPEN_PLAN_SECONDS), or one solve of a solver counted at more: a solve of step_flow_armaly is counted at 3 s, its measured time at the slowest input of its range, so the count admits at most 5 of it in one plan there; the local connection and a connection with a key admit the engine's own 64.");
        assert_eq!(said_of_solver("propeller_apc_10x7e"), "On the open connection a plan of solves is counted before it starts and holds no more than 15 s of solving (OPEN_PLAN_SECONDS), or one solve of a solver counted at more: a solve of propeller_apc_10x7e is counted at 0.01 s, its measured time at the slowest input of its range, so the count admits the engine's own 64 of it in one plan there, as on every connection.");
        // A solver whose worked solve one inline answer holds fewer of than the count admits says how many (W3 of the breadth test of 2026-10-16).
        // Its time and count are read from the table, which canonical math measures again.
        assert_eq!(
            said_of_solver("water_hammer_oil_line"),
            format!(
                "On the open connection a plan of solves is counted before it starts and holds no more than 15 s of solving (OPEN_PLAN_SECONDS), or one solve of a solver counted at more: a solve of water_hammer_oil_line is counted at {} s, its measured time at the slowest input of its range, so the count admits at most {} of it in one plan there; the local connection and a connection with a key admit the engine's own 64; an answer returned inline is at most 40000 characters and holds 31 of its worked solve (example_solve) in summary, on every connection: a plan is refused before solving only where the least its summary can come to is more, and is otherwise counted once solved, so a plan of a few more of it is solved and then refused.",
                solve_seconds("water_hammer_oil_line"),
                open_plan_solves("water_hammer_oil_line")
            )
        );
        // A solve counted above the 15 s is still admitted alone, as the sentence says.
        assert!(solve_seconds("junction_nist_b7_s2") > OPEN_PLAN_SECONDS);
        assert!(said_of_solver("junction_nist_b7_s2").contains(&format!("or one solve of a solver counted at more: a solve of junction_nist_b7_s2 is counted at {} s", solve_seconds("junction_nist_b7_s2"))));
        assert_eq!(open_plan_solves("junction_nist_b7_s2"), 1);
        for (solver, held) in INLINE_WORKED_SOLVES {
            assert!(*held < crate::solve_plan::MAX_SOLVES, "{solver}");
            assert!(said_of_solver(solver).contains(&format!("holds {held} of its worked solve (example_solve)")), "{solver}");
            // Where the open connection's count binds first, the inline count is said of the other connections.
            assert_eq!(said_of_solver(solver).contains(&format!("admit the engine's own 64, and there an answer returned inline")), *held >= open_plan_solves(solver), "{solver}");
        }
        assert_eq!(
            said_of_solver("stress_plate_hole_s235"),
            "On the open connection a plan of solves is counted before it starts and holds no more than 15 s of solving (OPEN_PLAN_SECONDS), or one solve of a solver counted at more: a solve of stress_plate_hole_s235 is counted at 1.7 s, its measured time at the slowest input of its range, so the count admits at most 8 of it in one plan there; the local connection and a connection with a key admit the engine's own 64, and there an answer returned inline is at most 40000 characters and holds 38 of its worked solve (example_solve) in summary: a plan is refused before solving only where the least its summary can come to is more, and is otherwise counted once solved, so a plan of a few more of it is solved and then refused."
        );
    }

    /// The limit holds for any number of solvers: the sentence and the
    /// clause name none and do not grow with the list, and every solver
    /// served, measured or not, states its own count in its description.
    #[test]
    fn the_plan_limit_is_said_once_for_any_number_of_solvers() {
        for solver in crate::solvers::ALL {
            for said in [statement(), said_in_a_description()] {
                assert!(!said.contains(solver.name), "{}: {said}", solver.name);
            }
            let described = crate::solvers::describe_whole(solver.name).unwrap();
            let own = described["open_connection"].as_str().unwrap_or_else(|| panic!("{} states no count", solver.name));
            assert_eq!(own, said_of_solver(solver.name));
            assert!(own.contains(&format!("a solve of {} is counted at {} s", solver.name, solve_seconds(solver.name))), "{own}");
            assert!(own.contains(&format!("{} of it in one plan there", open_plan_solves(solver.name))), "{own}");
        }
        // A solver not yet measured is counted at the slowest listed, and says so in the same words.
        assert!(said_of_solver("a_solver_not_yet_measured").contains("counted at 10000 s, its measured time at the slowest input of its range, so the count admits at most 1 of it in one plan there"));
        // The new solvers of this version hold the counts their times give.
        assert_eq!(
            ["junction_nist_b7_s2", "icf_hotspot_burn", "stress_plate_hole_s235", "blast_wave_tnt_air", "water_hammer_oil_line", "orifice_air_chamber", "pneumatic_hose_resonance", "ln2_line_dlr_fttf", "plasma_sheath_ram_c2", "culture_trichoderma_ambr250"].map(open_plan_solves),
            [1, 1, 8, 25, 35, 50, 64, 7, 9, 10]
        );
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
