//! The engine work orders of 18 October 2026 (`command-center/FLEET.md`,
//! "Engine work orders"; `RELEASES.md`, 0.14.0, "Known and left open"), each
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
    text
}

fn exact(state: &Value, name: &str) -> f64 {
    f64::from_bits(u64::from_str_radix(state[format!("{name}_hex")].as_str().unwrap(), 16).unwrap())
}

/// The worked run of a card, its last segment asked for `steps` in all.
fn worked_run(door: &mut Stdio2, card: &str, steps: u64) -> Value {
    let described = ok(door, "describe_card", &json!({ "domain": "energy", "card": card }));
    let mut run = described["example_run_trajectory"].clone();
    let segments = run["segments"].as_array_mut().unwrap();
    let before: u64 = segments[..segments.len() - 1].iter().map(|s| s["steps"].as_u64().unwrap()).sum();
    segments.last_mut().unwrap()["steps"] = json!(steps - before);
    run["record_every"] = json!(1000);
    run
}

/// The three SPERT III cards of 0.14.0 and the C-core twin of 0.14.3 land the
/// step that passes their maximum and refuse the next as their declared end.
/// A run asked for exactly the steps the uncut run landed was sealed
/// `completed` on the uncut run's final state: from the worked runs, 1,187,
/// 2,311, 990 and 2,297 steps (FIX_014, "Found here, not changed", 1). Rule
/// 13: a run that reaches an end its card declares is `ended`, as the
/// capsules' have been since 0.14.0. From 0.15.0 one card of the four is
/// served with such an end: reactor_spert3_ccore, the C-core plate card, its
/// worked run ending at 2,303 steps; the E-core twin served as reactor_spert3
/// has no first-maximum end, and the burst-to-fuel coupling is removed.
#[test]
fn a_spert3_run_cut_on_its_maximum_has_ended() {
    let mut door = Stdio2::start(None);
    for (card, end, reject, flag) in [
        ("reactor_spert3_ccore", 2303_u64, "PastPlateTemperatureMaximum", "past_plate_maximum"),
    ] {
        let uncut_request = worked_run(&mut door, card, end + 100);
        let uncut = ok(&mut door, "run_trajectory", &uncut_request);
        assert_eq!((uncut["outcome"].as_str(), uncut["steps_landed"].as_u64(), uncut["reject"].as_str()), (Some("ended"), Some(end), Some(reject)), "{card}");
        let cut_request = worked_run(&mut door, card, end);
        let cut = ok(&mut door, "run_trajectory", &cut_request);
        assert_eq!(cut["outcome"], "ended", "{card}: {}", cut["reject_detail"]);
        assert_eq!(cut["steps_landed"], json!(end), "{card}");
        assert_eq!(cut["reject"], reject, "{card}");
        assert_eq!(cut["reject_detail"], uncut["reject_detail"], "{card}");
        assert_eq!(cut["final_state"], uncut["final_state"], "{card}");
        assert_eq!(cut["final_state"][flag], json!(true), "{card}");
        assert_eq!(cut["state_digest"], uncut["state_digest"], "{card}");
        // The cut run verifies by request as the engine's own answer.
        let held = ok(&mut door, "verify_tape", &json!({ "request": cut_request, "run_seal": cut["run_seal"] }));
        assert_eq!(held["holds"], json!(true), "{card}: {held}");
        // One step short: completed, the flag not yet set.
        let short_request = worked_run(&mut door, card, end - 1);
        let short = ok(&mut door, "run_trajectory", &short_request);
        assert_eq!(short["outcome"], "completed", "{card}");
        assert_eq!(short["final_state"][flag], json!(false), "{card}");
    }
    // The card is not continued through from, cut or not, as before.
    let up = worked_run(&mut door, "reactor_spert3_ccore", 2303);
    let first = ok(&mut door, "run_trajectory", &up);
    assert_eq!(first["outcome"], "ended");
    let mut next = worked_run(&mut door, "reactor_spert3_ccore", 10);
    next["from"] = json!({ "request": up, "run_seal": first["run_seal"] });
    let text = refused(&mut door, "run_trajectory", &next);
    assert!(text.contains("(ContinuationNotOnCard)"), "{text}");
}

/// D2 of the random conformance test of 16 October 2026 (known since 14
/// October): a grid_node step that reaches the copper's melt was refused in
/// a sentence with no name in round brackets, where every other refusal of
/// a step carries its name, as compute_step's description says; the run's
/// reject_detail carried the same unnamed sentence. The sentence now names
/// CopperMelt. The run seal does not cover reject_detail, and a refused step
/// has no proof: no seal moves.
#[test]
fn grid_node_names_copper_melt() {
    let mut door = Stdio2::start(None);
    let step = json!({ "domain": "energy", "card": "grid_node", "parameters": {
        "current_a": 10.0, "voltage_v": 48.0, "temperature_k": 1357.7699999999998,
        "upstream_voltage_v": 48.0, "load_current_a": 10.0, "ambient_k": 293.15, "dt_s": 1e-06 } });
    let text = refused(&mut door, "compute_step", &step);
    assert_eq!(text, "Physical Failure: copper reached its melt point, 1357.77 K (CopperMelt). A molten state is not sealed.");
    let run = json!({ "domain": "energy", "card": "grid_node",
        "initial": { "current_a": 10.0, "voltage_v": 48.0, "temperature_k": 1357.7699999999998 }, "dt_s": 1e-06,
        "segments": [{ "steps": 1, "load": { "upstream_voltage_v": 48.0, "load_current_a": 10.0, "ambient_k": 293.15 } }], "record_every": 1000 });
    let tape = ok(&mut door, "run_trajectory", &run);
    assert_eq!(tape["reject"], "CopperMelt");
    let detail = tape["reject_detail"].as_str().unwrap();
    assert!(detail.starts_with("Physical Failure: copper reached its melt point, 1357.77 K (CopperMelt)."), "{detail}");
    // 0.14.3's run seal of the same request: the sentence is not under it.
    // Moved in 0.15.0 by seal rule v2 (SHA-384).
    assert_eq!(tape["run_seal"], "4584e06e906b35fbe815d69fe51a8850281f567e6d462c2280781807db36aa67dd87d07e4eb43f98e49c7b55f398a6ac");
}

/// The text of `object` with a receipt over it, as the engine writes one.
fn receipted(object: &Value) -> String {
    let mut object = object.clone();
    object.as_object_mut().unwrap().remove("receipt_sha384");
    let document = serde_json::to_string(&object).unwrap();
    let receipt = seal_sha(&document);
    format!("{},\"receipt_sha384\":\"{receipt}\"}}", &document[..document.len() - 1])
}

/// RT3 W1 of the red team of 0.14.0 rc1: the head of a plan of solves and a
/// comparison of solves named the kernel's version, genesis_kernel_v0.12.0,
/// at 0.13.1 and 0.14.x alike, though their rows are solves and their
/// solution seals cover the solvers' version; so verify_receipt called a
/// 0.13.1 plan this engine's. They now name the solvers' version, the one
/// describe_solver and every solve name, and verify_receipt calls a version
/// this engine's only where this engine writes it on that kind of answer.
/// Moves the dataset seal of every plan of solves (its counts carry
/// engine_version) and so the validation seal of every comparison of
/// solves; 0.14.3's seal of the same plan is named as that engine's pair.
#[test]
fn a_plan_of_solves_names_the_solvers_version() {
    let mut door = Stdio2::start(None);
    let described = ok(&mut door, "describe_solver", &json!({ "solver": "propeller_apc_10x7e" }));
    let solvers = described["engine_version"].as_str().unwrap().to_string();
    assert_eq!(solvers, "genesis_solvers_v0.13.0");
    let parameters = described["example_solve"]["parameters"].clone();
    let plan = json!({ "solver": "propeller_apc_10x7e", "plan": { "kind": "list", "solves": [{ "parameters": parameters }] } });
    let answer = ok(&mut door, "run_dataset", &plan);
    let head = &answer["head"];
    assert_eq!(head["engine_version"], json!(solvers), "{head}");
    let comparison = ok(&mut door, "validate_series", &json!({ "solves": plan, "quantity": "advance_ratio", "measurements": [{ "row": 0, "value": 1.0 }] }));
    assert_eq!(comparison["engine_version"], json!(solvers), "{comparison}");
    assert_eq!(comparison["dataset_seal"], head["dataset_seal"]);
    // The engine's own seal holds by request.
    let held = ok(&mut door, "verify_dataset", &json!({ "request": plan, "dataset_seal": head["dataset_seal"] }));
    assert_eq!(held["holds"], json!(true), "{held}");
    // 0.14.3's dataset seal of this plan, genesis_kernel_v0.12.0 in its
    // counts, is a SHA-256 seal: from 0.15.0 (seal rule v2) it is refused by
    // its length before any solve.
    const SEAL_0143: &str = "c8e71ba015d46d5d67c7790d36c514c185c8979323c87a3bf14957ca0f363ef6";
    let text = refused(&mut door, "verify_dataset", &json!({ "request": plan, "dataset_seal": SEAL_0143 }));
    assert_eq!(text, "Physical Violation: 'dataset_seal' must be 96 lowercase hex digits.");
    // verify_receipt: this head is this engine's; the same head naming the
    // kernel, as 0.14.3 wrote it, is not.
    let report = ok(&mut door, "verify_receipt", &json!({ "response_text": receipted(head) }));
    assert_eq!((report["matches"].as_bool(), report["engine_version_is_this_engines"].as_bool()), (Some(true), Some(true)), "{report}");
    let mut old = head.clone();
    old["engine_version"] = json!("genesis_kernel_v0.13.0");
    let report = ok(&mut door, "verify_receipt", &json!({ "response_text": receipted(&old) }));
    assert_eq!((report["matches"].as_bool(), report["engine_version_is_this_engines"].as_bool()), (Some(true), Some(false)), "{report}");
    let mut old = comparison.clone();
    old["engine_version"] = json!("genesis_kernel_v0.13.0");
    let report = ok(&mut door, "verify_receipt", &json!({ "response_text": receipted(&old) }));
    assert_eq!(report["engine_version_is_this_engines"], json!(false), "{report}");
    // A solve naming the kernel is not this engine's; a card's run naming
    // the solvers is not either; each kind under its own version is.
    let solved = ok(&mut door, "solve", &described["example_solve"]);
    let mut old = solved.clone();
    old["engine_version"] = json!("genesis_kernel_v0.13.0");
    let report = ok(&mut door, "verify_receipt", &json!({ "response_text": receipted(&old) }));
    assert_eq!(report["engine_version_is_this_engines"], json!(false), "{report}");
    let report = ok(&mut door, "verify_receipt", &json!({ "response_text": receipted(&solved) }));
    assert_eq!(report["engine_version_is_this_engines"], json!(true), "{report}");
    let run = ok(&mut door, "run_trajectory", &thermal_run(3, 293.15));
    let report = ok(&mut door, "verify_receipt", &json!({ "response_text": receipted(&run) }));
    assert_eq!(report["engine_version_is_this_engines"], json!(true), "{report}");
    let mut old = run.clone();
    old["engine_version"] = json!(solvers);
    let report = ok(&mut door, "verify_receipt", &json!({ "response_text": receipted(&old) }));
    assert_eq!(report["engine_version_is_this_engines"], json!(false), "{report}");
    // A plan of runs keeps the kernel's version, and its head is this engine's.
    let one = thermal_run(3, 293.15);
    let runs = json!({ "domain": "energy", "card": "thermal", "plan": { "kind": "list", "runs": [{ "initial": one["initial"], "dt_s": one["dt_s"], "segments": one["segments"], "record_every": 3 }] }, "store": "none", "return": "inline" });
    let made = ok(&mut door, "run_dataset", &runs);
    assert_eq!(made["head"]["engine_version"], "genesis_kernel_v0.13.0", "{made}");
    let report = ok(&mut door, "verify_receipt", &json!({ "response_text": receipted(&made["head"]) }));
    assert_eq!(report["engine_version_is_this_engines"], json!(true), "{report}");
}

/// RT3 D1 of the red team of 0.14.0 rc1: find_flip's FLIP_MAX_SECONDS was a
/// wall clock, so the same request gave 28, 28 and 29 solves and two pairs
/// on one machine, and 2 solves on the same machine at a load average of
/// about 100 (18 October 2026, `frozen_0.14.0_rc1/mcp_stdio`). Since 0.14.0
/// rc2 it is a count: each solve at its solver's time in OPEN_SOLVE_SECONDS,
/// so the plate's search stops at 12 solves on any machine. Held here: the
/// red team's request, sent to two processes at once, gives 12 solves, the
/// same pair and the same bytes.
#[test]
fn find_flip_stops_the_same_request_at_the_same_solve() {
    let request: Value = json!({
        "request": { "solver": "stress_plate_hole_s235", "parameters": { "traction_mpa": 106.26, "points_x_mm": [0.0, 0.0, 4.5, -20.0], "points_y_mm": [4.0, 10.025, 0.0, 5.0] } },
        "vary": "parameters.traction_mpa", "low": 103.0722, "high": 109.44780000000002,
        "level": { "quantity": "axial_force_n", "value": 6498.064650000001 }
    });
    let answers: Vec<String> = std::thread::scope(|scope| {
        let searches: Vec<_> = (0..2)
            .map(|_| {
                let request = request.clone();
                scope.spawn(move || {
                    let (error, text) = Stdio2::start(None).tool("find_flip", &request);
                    assert!(!error, "{text}");
                    text
                })
            })
            .collect();
        searches.into_iter().map(|search| search.join().unwrap()).collect()
    });
    assert_eq!(answers[0], answers[1]);
    let answer = parsed(&answers[0]);
    assert_eq!((answer["solves"].as_u64(), answer["stopped_by"].as_str()), (Some(12), Some("FLIP_MAX_SECONDS")), "{answer}");
    assert_eq!((answer["low"]["value_hex"].as_str(), answer["high"]["value_hex"].as_str()), (Some("405a903dd495182a"), Some("405a90a3d70a3d71")), "{answer}");
}

/// D1b of the random conformance test of 16 October 2026: grid_gb with an
/// inertia under about 7.42e-304 MW s was refused NonFiniteState, where a
/// little above it the same step is DtAboveCeiling, with a ceiling printed
/// under the normal range (8.5453595774902e-310 s; invariant 4). The run's
/// own ceiling, a fiftieth of its fastest rate, is not a normal number
/// there, and every step is above it: DtAboveCeiling, with no subnormal
/// number in the sentence. Unsealed refusals: no seal moves.
/// From 0.15.0 the GB grid twin is served as grid_gb: the request carries its
/// six service volumes and two delivery times, the worked example's, and a
/// step of 5 ms, under the card's fixed ceiling of 1/150 s, so the ceiling
/// at the inertia is the one tested. 5e-303 MW s is where that ceiling is
/// under the normal range.
#[test]
fn grid_gb_at_a_vanishing_inertia_names_its_ceiling() {
    let mut door = Stdio2::start(None);
    let step = |inertia: f64| json!({ "domain": "energy", "card": "grid_gb", "parameters": {
        "frequency_hz": 49.942, "inertia_mw_s": inertia, "demand_mw": 27132.0,
        "dc_low_mw": 1145.0, "dm_low_mw": 500.0, "dr_low_mw": 480.0, "dc_high_mw": 1080.0, "dm_high_mw": 510.0, "dr_high_mw": 480.0,
        "fast_delivery_s": 1.0, "regulation_delivery_s": 10.0, "loss_mw": 996.0, "dt_s": 0.005 } });
    for inertia in [1e-305, 7.423562245660344e-304, 1e-303, 5e-303] {
        let text = refused(&mut door, "compute_step", &step(inertia));
        assert!(text.starts_with("Physical Violation: 'dt_s' 0.005 is above the step ceiling of the grid_gb card"), "{inertia:e}: {text}");
        assert!(text.ends_with("which is under 2.2250738585072014e-308 s, the smallest normal 64-bit number (DtAboveCeiling)."), "{inertia:e}: {text}");
    }
    // Where the ceiling is a normal number it is printed, as before.
    let text = refused(&mut door, "compute_step", &step(1e-290));
    assert!(text.ends_with("s, the step ceiling of the grid_gb card (DtAboveCeiling)."), "{text}");
    assert!(!text.contains("e-31") && !text.contains("e-309"), "{text}");
}

/// G4's Sector 9 red team of 0.14.3 (`evidence/research/gg_lab/s9_redteam/`,
/// `responses/0257.json`): HOSTING.md states MAX_HEADER_BYTES = 65536 and a
/// 431 for headers that do not fit, but a header line of 100,000 bytes was
/// answered 200, and so were 60, 70 and 100 KB: the library stops reading at
/// its buffer and its last read may bring up to one buffer more. The door
/// now counts the header section itself, the request line, each header line
/// and the blank line, and answers 431 with no body past 65,536 bytes.
#[test]
fn a_header_section_past_max_header_bytes_is_431() {
    const KEY: &str = "key-of-the-header-test";
    const PING: &str = r#"{"jsonrpc":"2.0","id":1,"method":"ping"}"#;
    let door = Hosted::start(vec![client_entry("one", KEY, 4, 2)], None);
    let send = |filler: usize| -> Reply {
        let head = format!(
            "POST /mcp HTTP/1.1\r\nHost: x\r\nConnection: close\r\nContent-Type: application/json\r\nAuthorization: Bearer {KEY}\r\nX-Big-Header: {}\r\nContent-Length: {}\r\n\r\n",
            "A".repeat(filler),
            PING.len()
        );
        parse_reply(&raw(door.port, format!("{head}{PING}").as_bytes(), 20))
    };
    let max = protocol_mcp_gateway::hosted::MAX_HEADER_BYTES;
    // The header section's size is the filler and `fixed` bytes more.
    let fixed = format!("POST /mcp HTTP/1.1\r\nHost: x\r\nConnection: close\r\nContent-Type: application/json\r\nAuthorization: Bearer {KEY}\r\nX-Big-Header: \r\nContent-Length: {}\r\n\r\n", PING.len()).len();
    for filler in [100_000, 70_000, max - fixed + 1] {
        let reply = send(filler);
        assert_eq!((reply.status, reply.body.as_str()), (431, ""), "{filler}: {reply:?}");
    }
    for filler in [60_000, max - fixed] {
        let reply = send(filler);
        assert_eq!(reply.status, 200, "{filler}: {reply:?}");
    }
}

/// Every number of `value` with a `_hex` copy, by its bits: finite, and
/// normal or zero.
fn subnormals(value: &Value, at: &str, out: &mut Vec<String>) {
    match value {
        Value::Object(map) => {
            for (key, inner) in map {
                if key.ends_with("_hex") {
                    if let Some(bits) = inner.as_str().and_then(|text| u64::from_str_radix(text, 16).ok()) {
                        let number = f64::from_bits(bits);
                        if !number.is_finite() || (number != 0.0 && !number.is_normal()) {
                            out.push(format!("{at}.{key} = {number:e}"));
                        }
                    }
                }
                subnormals(inner, &format!("{at}.{key}"), out);
            }
        }
        Value::Array(items) => items.iter().enumerate().for_each(|(i, inner)| subnormals(inner, &format!("{at}[{i}]"), out)),
        _ => {}
    }
}

/// G4's Sector 9 red team of 0.14.3 (`responses/0244.json`, `0246.json`):
/// cantilever_eth from a deflection of 4.1e-156 m was said to publish a
/// subnormal stored_j_per_kg, 2.2816693218437677e-308. Its bits,
/// 0010682edd72a70b, have exponent field 1: it is a normal number, just above
/// 2^-1022 = 2.2250738585072014e-308. The kinetic and potential energies,
/// computed under 2^-1022, are written as zero where they are computed, and
/// their sum, computed from the values before that, is normal; in 0246 the
/// sum is under 2^-1022 too and is written as zero. Invariant 4 holds on both
/// answers; held here on every number they publish.
#[test]
fn a_deep_small_cantilever_publishes_only_normal_numbers_or_zero() {
    let mut door = Stdio2::start(None);
    for (deflection, steps) in [(4.1e-156, 100_u64), (4.048831665655859e-156, 40_000)] {
        let run = json!({ "domain": "signal", "card": "cantilever_eth", "initial": { "deflection_m": deflection, "velocity_m_s": 0.0 },
            "dt_s": 0.00025, "segments": [{ "steps": steps, "load": {} }], "record_every": steps });
        let answer = ok(&mut door, "run_trajectory", &run);
        let mut found = Vec::new();
        subnormals(&answer, "", &mut found);
        assert_eq!(found, Vec::<String>::new(), "{deflection:e}");
        let stored = exact(&answer["final_state"], "stored_j_per_kg");
        assert!(stored == 0.0 || stored.is_normal(), "{stored:e}");
    }
    assert!(f64::from_bits(0x0010_682e_dd72_a70b).is_normal());
}
