//! The fix pass of 0.14.0: the defects the breadth tests of 16 October 2026
//! found (`evidence/verification/breadth_2026-10-16/`), each held here by its
//! own case, on the local connection.

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
    assert!(!text.contains("run_seal\":") && !text.contains("chain_seal\":") && !text.contains("sha384_seal"), "{text}");
    text
}

/// `verify_tape`'s links form names its refusals as the rest of the engine
/// names the same thing (`chains/.../repro_cases/links_form_unnamed_refusals`):
/// more answers than a chain holds is ChainTooLong, as a request with from
/// nested 17 deep; a link without a required field is InputMissing, as the
/// top level is; and a first link that continues an upstream carries the
/// upstream's seal and not its request, AnswerCarriesNoRequest, as such an
/// answer sent alone to the long form is.
#[test]
fn the_links_form_names_its_refusals() {
    let mut door = Stdio2::start(None);
    let text = refused(&mut door, "verify_tape", &json!({ "links": vec![json!({}); 17] }));
    assert_eq!(
        text,
        "Physical Violation: 'links' must be an array of at most 16 answers: the chain holds more than 16 runs, a step at its end counted as one, the most one request may carry (ChainTooLong)."
    );
    let text = refused(&mut door, "verify_tape", &json!({ "links": [{}, {}] }));
    assert_eq!(text, "Missing required parameter: 'links[0].inputs' (InputMissing).");
    // The same name at the top level, as before.
    let text = refused(&mut door, "verify_tape", &json!({}));
    assert_eq!(text, "Missing required parameter: 'inputs' (InputMissing).");
    // A first link that carries a chain: a real chained answer first.
    let up = thermal_run(3, 293.15);
    let first = ok(&mut door, "run_trajectory", &up);
    let mut down = thermal_run(3, 293.15);
    down["initial"]["temperature_k"] = Value::from(f64::from_bits(
        u64::from_str_radix(first["final_state"]["temperature_k_hex"].as_str().unwrap(), 16).unwrap(),
    ));
    down["from"] = json!({ "request": up, "run_seal": first["run_seal"] });
    let second = ok(&mut door, "run_trajectory", &down);
    assert_eq!(second["chain"]["kind"], "continuation");
    let text = refused(&mut door, "verify_tape", &json!({ "links": [second, first] }));
    assert!(text.starts_with("Physical Violation: 'links[0].chain' must be absent: the first link is where the chain starts, so send its upstream links too."), "{text}");
    assert!(text.contains("(AnswerCarriesNoRequest)"), "{text}");
    // The chain in order holds, as before.
    let report = ok(&mut door, "verify_tape", &json!({ "links": [first, second] }));
    assert_eq!(report["holds"], json!(true), "{report}");
    // Fewer than two answers keeps its sentence: REFERENCE.md states the
    // links form's two to sixteen answers.
    let text = refused(&mut door, "verify_tape", &json!({ "links": [first] }));
    assert_eq!(text, "Physical Violation: 'links' must be an array of at least two answers.");
}

/// A run of `steps` from the start the chains test cut on the declared end.
fn capsule_run(card: &str, steps: u64) -> Value {
    match card {
        "capsule_stardust" => json!({
            "domain": "aerospace", "card": "capsule_stardust", "dt_s": 0.05,
            "initial": { "radius_m": 6503140.0, "radial_speed_m_s": -1842.1416729435698, "transverse_speed_m_s": 12767.792058801891 },
            "segments": [{ "steps": steps, "load": {} }], "record_every": 100
        }),
        _ => json!({
            "domain": "aerospace", "card": "capsule_insight", "dt_s": 0.1,
            "initial": { "radius_m": 3522199.9999655997, "radial_speed_m_s": -1202.1585438056352, "transverse_speed_m_s": 5633.705477158609 },
            "segments": [{ "steps": steps, "load": { "lift_over_drag": -0.031 } }], "record_every": 100
        }),
    }
}

fn exact(state: &Value, name: &str) -> f64 {
    f64::from_bits(u64::from_str_radix(state[format!("{name}_hex")].as_str().unwrap(), 16).unwrap())
}

/// Rule 13 on the two capsules (`chains/.../repro_cases/capsule_*_cut_on_end`):
/// the step that first crosses the flown deployment lands, and a run whose
/// last requested step is that step has reached the end its card declares,
/// so it is `ended`, as the same run asked for more steps is, on the same
/// final state; a continuation from it is refused by name, as from any run
/// that ended, and a run one step short completes and its continuation ends
/// at its first step. tokamak_vde_mast's end is a step that does not land, so
/// a run cut where its uncut run ended completes, and its continuation ends
/// with no step landed, as before.
#[test]
fn a_capsule_run_whose_last_step_lands_the_deployment_has_ended() {
    let mut door = Stdio2::start(None);
    for (card, end) in [("capsule_stardust", 2779_u64), ("capsule_insight", 2106)] {
        let uncut = ok(&mut door, "run_trajectory", &capsule_run(card, end + 10));
        assert_eq!((uncut["outcome"].as_str(), uncut["steps_landed"].as_u64()), (Some("ended"), Some(end)), "{card}");
        let cut = ok(&mut door, "run_trajectory", &capsule_run(card, end));
        assert_eq!(cut["outcome"], "ended", "{card}: {}", cut["reject_detail"]);
        assert_eq!(cut["steps_landed"], json!(end), "{card}");
        assert_eq!(cut["reject"], "PastFlightDeployment", "{card}");
        assert_eq!(cut["reject_detail"], uncut["reject_detail"], "{card}");
        assert_eq!(cut["final_state"], uncut["final_state"], "{card}");
        assert_eq!(cut["state_digest"], uncut["state_digest"], "{card}");
        // A continuation goes on only from a run that completed.
        let mut next = capsule_run(card, 10);
        for name in ["radius_m", "radial_speed_m_s", "transverse_speed_m_s"] {
            next["initial"][name] = Value::from(exact(&cut["final_state"], name));
        }
        next["from"] = json!({ "request": capsule_run(card, end), "run_seal": cut["run_seal"] });
        let text = refused(&mut door, "run_trajectory", &next);
        assert!(text.contains("(UpstreamOutcomeNotCarried)"), "{card}: {text}");
        // One step short: completed, and its continuation ends at its first step.
        let short = ok(&mut door, "run_trajectory", &capsule_run(card, end - 1));
        assert_eq!(short["outcome"], "completed", "{card}");
        let mut next = capsule_run(card, 11);
        for name in ["radius_m", "radial_speed_m_s", "transverse_speed_m_s"] {
            next["initial"][name] = Value::from(exact(&short["final_state"], name));
        }
        next["from"] = json!({ "request": capsule_run(card, end - 1), "run_seal": short["run_seal"] });
        let continued = ok(&mut door, "run_trajectory", &next);
        assert_eq!((continued["outcome"].as_str(), continued["steps_landed"].as_u64()), (Some("ended"), Some(1)), "{card}");
        // The same start one step short, run one step: ended on that step.
        let mut one = next.clone();
        one["segments"][0]["steps"] = json!(1);
        let one = ok(&mut door, "run_trajectory", &one);
        assert_eq!((one["outcome"].as_str(), one["steps_landed"].as_u64()), (Some("ended"), Some(1)), "{card}");
    }
    // tokamak_vde_mast: the run cut where its uncut run ended completes, its
    // last landed state inside the vessel, and the continuation ends with
    // no step landed.
    let vde = |steps: u64| {
        json!({
            "domain": "energy", "card": "tokamak_vde_mast", "dt_s": 0.0001,
            "initial": { "displacement_m": 0.001, "velocity_m_s": 0.006546503766672119, "wall_current_a": -21.946577663159488, "p6_current_a": -8.150423262860404 },
            "segments": [{ "steps": steps, "load": { "p6_voltage_v": 0.0 } }], "record_every": 100
        })
    };
    let uncut = ok(&mut door, "run_trajectory", &vde(7309));
    assert_eq!((uncut["outcome"].as_str(), uncut["steps_landed"].as_u64()), (Some("ended"), Some(7299)));
    let cut = ok(&mut door, "run_trajectory", &vde(7299));
    assert_eq!(cut["outcome"], "completed");
    assert_eq!(cut["final_state"], uncut["final_state"]);
    let displacement = exact(&cut["final_state"], "displacement_m");
    assert!(displacement > -0.391 && displacement < 0.389, "{displacement}");
    let mut next = vde(10);
    for name in ["displacement_m", "velocity_m_s", "wall_current_a", "p6_current_a"] {
        next["initial"][name] = Value::from(exact(&cut["final_state"], name));
    }
    next["from"] = json!({ "request": vde(7299), "run_seal": cut["run_seal"] });
    let continued = ok(&mut door, "run_trajectory", &next);
    assert_eq!((continued["outcome"].as_str(), continued["steps_landed"].as_u64()), (Some("ended"), Some(0)));
}

const CRATER_FINAL_RADIUS_M: f64 = 8.571304329477922;

fn crater_run(radius_m: f64, steps: u64) -> Value {
    json!({
        "domain": "aerospace", "card": "crater_ryugu_sci", "dt_s": 0.25,
        "initial": { "radius_m": radius_m }, "segments": [{ "steps": steps, "load": {} }], "record_every": 1
    })
}

/// crater_ryugu_sci's start fields do not carry the crater while it grows:
/// a continuation starts from radius_m alone, the wall at the speed the card
/// sets from the radius, so the wall's speed and the energy accounts the
/// upstream integrated are not carried, and a run of 1,000 steps from 1.75 m
/// cut at 500 landed radius_m 8.15908435779992 (4020517381441c94) against
/// 8.159084357848565 (402051738144878d) uncut
/// (`chains/.../repro_cases/crater_cut_before_stop`). REFERENCE.md: a card
/// whose start fields do not carry its state refuses a continuation by name,
/// ContinuationNotOnCard. Before the stop it is refused, by run and by step,
/// with no proof and before any physics; from a crater that has stopped, at
/// 8.571304329477922 m with its wall at rest, a run cut in two is the run
/// uncut but for t_s, as its description says.
#[test]
fn a_crater_is_continued_only_once_it_has_stopped() {
    let mut door = Stdio2::start(None);
    let described = ok(&mut door, "describe_card", &json!({ "domain": "aerospace", "card": "crater_ryugu_sci" }));
    let sentence = described["continuation"]["sentence"].as_str().unwrap();
    assert_eq!(described["continuation"]["served"], json!(true));
    assert!(sentence.contains("(ContinuationNotOnCard)") && sentence.ends_with("bit for bit except in t_s."), "{sentence}");
    assert!(described["rejects"].as_array().unwrap().iter().any(|reject| reject["name"] == "ContinuationNotOnCard"), "{}", described["rejects"]);
    // Cut before the stop: refused.
    let up = crater_run(1.75, 500);
    let first = ok(&mut door, "run_trajectory", &up);
    assert_eq!(first["final_state"]["stopped"], json!(false));
    let radius = exact(&first["final_state"], "radius_m");
    let mut next = crater_run(radius, 500);
    next["from"] = json!({ "request": up, "run_seal": first["run_seal"] });
    let text = refused(&mut door, "run_trajectory", &next);
    assert!(text.ends_with("(ContinuationNotOnCard)."), "{text}");
    // Before any physics: a forged upstream seal is not what is named.
    next["from"]["run_seal"] = json!("0".repeat(96));
    let text = refused(&mut door, "run_trajectory", &next);
    assert!(text.ends_with("(ContinuationNotOnCard)."), "{text}");
    let step = json!({
        "domain": "aerospace", "card": "crater_ryugu_sci", "parameters": { "radius_m": radius, "dt_s": 0.25 },
        "from": { "request": up, "run_seal": first["run_seal"] }
    });
    let text = refused(&mut door, "compute_step", &step);
    assert!(text.ends_with("(ContinuationNotOnCard)."), "{text}");
    // The same radius typed by hand, with no from, is a fresh start, as before.
    ok(&mut door, "run_trajectory", &crater_run(radius, 500));
    // Cut after the stop, at step 1,597 and at 1,600 of 2,000: the run uncut but for t_s.
    let uncut = ok(&mut door, "run_trajectory", &crater_run(1.75, 2000));
    for cut in [1597_u64, 1600] {
        let up = crater_run(1.75, cut);
        let first = ok(&mut door, "run_trajectory", &up);
        assert_eq!(first["final_state"]["stopped"], json!(true), "{cut}");
        assert_eq!(exact(&first["final_state"], "radius_m").to_bits(), CRATER_FINAL_RADIUS_M.to_bits(), "{cut}");
        let mut next = crater_run(CRATER_FINAL_RADIUS_M, 2000 - cut);
        next["from"] = json!({ "request": up, "run_seal": first["run_seal"] });
        let continued = ok(&mut door, "run_trajectory", &next);
        assert_eq!(continued["chain"]["kind"], "continuation", "{cut}");
        let (a, b) = (continued["final_state"].as_object().unwrap(), uncut["final_state"].as_object().unwrap());
        let differ: Vec<&String> = a.keys().filter(|key| key.ends_with("_hex") && a[*key] != b[*key]).collect();
        assert_eq!(differ, vec!["t_s_hex"], "{cut}");
        let mut step = json!({
            "domain": "aerospace", "card": "crater_ryugu_sci", "parameters": { "radius_m": CRATER_FINAL_RADIUS_M, "dt_s": 0.25 }
        });
        step["from"] = json!({ "request": up, "run_seal": first["run_seal"] });
        ok(&mut door, "compute_step", &step);
    }
}

/// The two culture solvers' schemas say what the engine applies (CC's check
/// of 16 October 2026): a segment's feed and water have no bound of their
/// own, and any finite amount is answered while the broth stays above 0 and
/// at most 250 mL (VolumeOutOfRange). A feed of 300 mL with water -200 mL in
/// one segment is answered and sealed, so the schema gives neither a maximum
/// of 250 mL nor a minimum of -250 mL; a feed of 260 mL alone is refused by
/// name, as before.
#[test]
fn the_culture_schemas_bound_feed_and_water_as_the_engine_does() {
    let mut door = Stdio2::start(None);
    for solver in ["culture_trichoderma_atp", "culture_trichoderma_ambr250"] {
        let described = ok(&mut door, "describe_solver", &json!({ "solver": solver }));
        let input = |name: &str| described["inputs"].as_array().unwrap().iter().find(|input| input["name"] == name).unwrap().clone();
        let (feed, water) = (input("feed_ml"), input("water_ml"));
        assert_eq!(feed["minimum"], json!(0.0), "{solver}");
        assert_eq!(feed["maximum"], json!(f64::MAX), "{solver}");
        assert_eq!((water["minimum"].clone(), water["maximum"].clone()), (json!(-f64::MAX), json!(f64::MAX)), "{solver}");
        let mut request = described["example_solve"].clone();
        request["parameters"]["feed_ml"][0] = json!(300.0);
        request["parameters"]["water_ml"][0] = json!(-200.0);
        let answered = ok(&mut door, "solve", &request);
        assert!(answered["solution_seal"].is_string(), "{solver}: {answered}");
        let mut alone = described["example_solve"].clone();
        alone["parameters"]["feed_ml"][0] = json!(260.0);
        let text = refused(&mut door, "solve", &alone);
        assert!(text.contains("(VolumeOutOfRange)"), "{solver}: {text}");
    }
}

/// The time label says how each card keeps its clock (the review of G^G's
/// sector 1, REVIEW_S1.md, M1): the two MMRTG cards keep a compensated mission
/// clock, ten steps of 4.1 s reading 41.0 s where a running sum reads
/// 41.00000000000001, and their label says so; every other card's clock is
/// the running sum the generic sentence states (measure_fix/item10_clock_probe.txt
/// probed all 37).
#[test]
fn the_time_label_says_which_cards_compensate_their_clock() {
    let mut door = Stdio2::start(None);
    let cards = ok(&mut door, "list_cards", &json!({}));
    for entry in cards["cards"].as_array().unwrap() {
        let (domain, card) = (entry["domain"].as_str().unwrap(), entry["card"].as_str().unwrap());
        let described = ok(&mut door, "describe_card", &json!({ "domain": domain, "card": card }));
        let label = described["time_label"].as_str().unwrap();
        let compensated = ["generator_mmrtg_f1", "generator_mmrtg_seebeck"].contains(&card);
        assert_eq!(label.contains("compensated sum"), compensated, "{card}: {label}");
        assert_eq!(label.contains("t_s += dt_s"), !compensated, "{card}: {label}");
        if compensated {
            assert!(label.contains("ten steps of 4.1 s read 41.0 s"), "{card}: {label}");
            let mut run = described["example_run_trajectory"].clone();
            run["initial"]["mission_time_s"] = json!(0.0);
            run["dt_s"] = json!(4.1);
            run["segments"] = json!([{ "steps": 10, "load": run["segments"][0]["load"] }]);
            run["record_every"] = json!(10);
            let answer = ok(&mut door, "run_trajectory", &run);
            assert_eq!(answer["final_state"]["t_s_hex"], json!(format!("{:016x}", 41.0_f64.to_bits())), "{card}");
        }
    }
}
