//! The two inputs a solver holds to a floor of its own, on the local
//! connection: a boron density of `fusion_pb11_lhd` that is not zero and is
//! under one nucleus in the plasma, and a row of an `ablator_slab` heating
//! history less than a millisecond after the row before it. Each is refused
//! by the solver's name and in the solver's words before anything is
//! computed, and `describe_solver` says so. What lies inside the floor is
//! answered, with the solver's own bits and a proof that holds.

mod hosted_support;

use std::time::{Duration, Instant};

use genesis_solvers::ablator_slab as slab;
use genesis_solvers::fusion_pb11_lhd::{self as lhd, LhdQuestion};
use hosted_support::*;
use serde_json::{Value, json};

/// A refusal before computing is answered well inside this: the solves these
/// inputs once started took seconds to minutes.
const AT_ONCE: Duration = Duration::from_secs(2);

fn described(door: &mut Stdio2, solver: &str) -> Value {
    let (error, text) = door.tool("describe_solver", &json!({ "solver": solver }));
    assert!(!error, "{text}");
    serde_json::from_str(&text).unwrap()
}

fn answered(door: &mut Stdio2, request: &Value) -> Value {
    let (error, text) = door.tool("solve", request);
    assert!(!error, "{request}: {text}");
    serde_json::from_str(&text).unwrap()
}

/// The solver's sentence for a request it stops before computing, and how
/// long the answer took.
fn stopped(door: &mut Stdio2, request: &Value) -> (String, Duration) {
    let started = Instant::now();
    let (error, text) = door.tool("solve", request);
    let took = started.elapsed();
    assert!(error, "{request}: {text}");
    (text, took)
}

fn refusal<'a>(described: &'a Value, name: &str) -> &'a Value {
    described["refusals"]
        .as_array()
        .unwrap()
        .iter()
        .find(|refusal| refusal["name"] == name)
        .unwrap_or_else(|| panic!("{name} is not listed"))
}

fn input<'a>(described: &'a Value, name: &str) -> &'a Value {
    described["inputs"]
        .as_array()
        .unwrap()
        .iter()
        .find(|input| input["name"] == name)
        .unwrap_or_else(|| panic!("{name} is not an input"))
}

fn hex_of(value: f64) -> String {
    format!("{:016x}", value.to_bits())
}

#[test]
fn a_boron_density_under_one_nucleus_is_refused_at_once_and_zero_the_floor_and_the_example_are_answered() {
    let mut local = Stdio2::start(None);
    let described = described(&mut local, "fusion_pb11_lhd");
    let example = described["example_solve"].clone();
    let with = |boron: Value| {
        let mut request = example.clone();
        request["parameters"]["boron_density_per_m3"] = boron;
        request
    };
    let floor = lhd::BORON_DENSITY_ONE_NUCLEUS_PER_M3;
    assert_eq!(floor, 0.04973892441153873);

    // Refused by the solver's name, in its words, the value by its exponent.
    for (written, boron) in [("1e-297", 1.0e-297), ("1.8008101798042446e-289", 1.8008101798042446e-289)] {
        let request = with(json!(boron));
        let (sentence, took) = stopped(&mut local, &request);
        assert_eq!(
            sentence,
            format!(
                "Physical Violation: boron density {written} m^-3 is not zero and is below 0.04973892441153873 m^-3, the peak density at which the plasma holds one boron nucleus: less than one nucleus is no boron, and zero is answered (BoronBelowOneNucleus)."
            )
        );
        assert!(took < AT_ONCE, "{written}: {took:?}");
        // A check of it by request stops on the same sentence, with no proof.
        let started = Instant::now();
        let check = local.tool("verify_solution", &json!({ "request": request, "solution_seal": "ab".repeat(48) }));
        assert_eq!(check, (true, sentence));
        assert!(started.elapsed() < AT_ONCE, "{written}: {:?}", started.elapsed());
    }
    // The number next below the floor, and a density a person might write.
    for boron in [f64::from_bits(floor.to_bits() - 1), 0.04, 1.0e-3] {
        let (sentence, took) = stopped(&mut local, &with(json!(boron)));
        assert!(sentence.ends_with("and zero is answered (BoronBelowOneNucleus)."), "{boron}: {sentence}");
        assert!(took < AT_ONCE, "{boron}: {took:?}");
    }

    // Zero, the floor itself and the worked example are answered, each with
    // the solver's own bits and a proof that holds by request.
    let parameter = |name: &str| example["parameters"][name].as_f64().unwrap();
    assert_eq!(parameter("boron_density_per_m3"), 1.4298e17);
    for boron in [0.0, floor, 1.4298e17] {
        let request = with(json!(boron));
        let body = answered(&mut local, &request);
        assert_eq!(body["outcome"], "converged", "{boron}");
        assert_eq!(body["engine_version"], "genesis_solvers_v0.13.0");
        let direct = lhd::solve(&LhdQuestion {
            beam_energy_kev: parameter("beam_energy_kev"),
            beam_power_w: parameter("beam_power_w"),
            electron_density_per_m3: parameter("electron_density_per_m3"),
            electron_temperature_kev: parameter("electron_temperature_kev"),
            boron_density_per_m3: boron,
        })
        .unwrap_or_else(|reject| panic!("{boron}: {reject}"));
        for (name, value) in [
            ("alpha_emission_rate_per_s", direct.alpha_emission_rate_per_s),
            ("fusion_power_w", direct.fusion_power_w),
            ("fusion_gain", direct.fusion_gain),
            ("fused_fraction", direct.fused_fraction),
            ("peak_alpha_emissivity_per_m3_s", direct.peak_alpha_emissivity_per_m3_s),
            ("peak_emissivity_radius", direct.peak_emissivity_radius),
            ("slowing_time_s", direct.slowing_time_s),
            ("fast_proton_fraction_max", direct.fast_proton_fraction_max),
            ("energy_fraction_to_electrons", direct.energy_fraction_to_electrons),
            ("energy_fraction_to_ions", direct.energy_fraction_to_ions),
        ] {
            assert_eq!(body["result"][format!("{name}_hex")], hex_of(value), "{boron}: {name}");
        }
        assert_eq!(body["invariants"]["proton_ledger_residual_hex"], hex_of(direct.proton_ledger_residual), "{boron}");
        assert_eq!(body["invariants"]["energy_ledger_residual_hex"], hex_of(direct.energy_ledger_residual), "{boron}");
        assert_eq!(body["iterations"], u64::from(direct.refinements), "{boron}");
        let (error, text) = local.tool("verify_solution", &json!({ "request": request, "solution_seal": body["solution_seal"] }));
        assert!(!error, "{text}");
        let check: Value = serde_json::from_str(&text).unwrap();
        assert_eq!(check["holds"], true, "{boron}: {text}");
    }
    // No boron fuses nothing, and one nucleus fuses something.
    assert_eq!(answered(&mut local, &with(json!(0.0)))["result"]["alpha_emission_rate_per_s"], 0.0);
    assert!(answered(&mut local, &with(json!(floor)))["result"]["alpha_emission_rate_per_s"].as_f64().unwrap() > 0.0);

    // describe_solver names the refusal, how it is reported, and the floor.
    let listed = refusal(&described, "BoronBelowOneNucleus");
    assert_eq!(listed["reported_as"], "422, no seal");
    assert_eq!(
        listed["meaning"],
        "`boron_density_per_m3` is not zero and is below `BORON_DENSITY_ONE_NUCLEUS_PER_M3`, the peak density at which the plasma holds one boron nucleus: less than one nucleus is no boron, and zero is answered."
    );
    let names: Vec<&str> = described["refusals"].as_array().unwrap().iter().map(|refusal| refusal["name"].as_str().unwrap()).collect();
    assert_eq!(names.len(), 13);
    assert_eq!(&names[4..7], ["BoronDensityOutOfRange", "BoronBelowOneNucleus", "BoronChargeExceedsElectrons"]);
    let boron = input(&described, "boron_density_per_m3");
    assert_eq!((boron["minimum"].as_f64(), boron["maximum"].as_f64()), (Some(0.0), Some(lhd::ELECTRON_DENSITY_MAX_PER_M3)));
    let bound = boron["bound"].as_str().unwrap();
    assert!(bound.contains("zero, or from 0.04973892441153873 m^-3 to what quasi-neutrality allows"), "{bound}");
    assert!(bound.contains("A density that is not zero and is below it is refused as BoronBelowOneNucleus"), "{bound}");
}

fn slab_request(times: Value) -> Value {
    let rows = times.as_array().unwrap().len();
    json!({ "solver": "ablator_slab", "parameters": {
        "material": "TACOT 3.0", "thickness_m": 0.05, "start_temperature_k": 300.0, "thermocouple_depths_m": [],
        "face": "gas_stream", "stream_gas": "air", "history_time_s": times,
        "history_pressure_pa": vec![1000.0; rows], "history_heat_transfer_coefficient_kg_per_m2_s": vec![0.3; rows],
        "history_flow_enthalpy_j_per_kg": vec![2.0e6; rows], "history_radiative_heating_w_per_m2": vec![0.0; rows],
        "surroundings_temperature_k": 300.0, "face_wears": true, "return_face_history": false,
        "return_thermocouple_histories": false, "return_final_profile": false
    } })
}

#[test]
fn a_history_row_under_a_millisecond_after_the_one_before_is_refused_at_once_and_rows_a_millisecond_apart_are_answered() {
    let mut local = Stdio2::start(None);
    assert_eq!(slab::HISTORY_ROW_SPAN_MIN_S, 0.001);

    // One row 1e-300 s after the first: 139,626 time steps before the solver
    // held the floor, and now its own sentence before the first step.
    let vanishing = slab_request(json!([0.0, 1.0e-300, 10.0]));
    let (sentence, took) = stopped(&mut local, &vanishing);
    assert_eq!(
        sentence,
        "Physical Violation: row 1 of the heating history is 1e-300 s after the row before it; the least time between two rows is 0.001 s (HistoryRowsTooClose)."
    );
    assert!(took < AT_ONCE, "{took:?}");
    let started = Instant::now();
    let check = local.tool("verify_solution", &json!({ "request": vanishing, "solution_seal": "ab".repeat(48) }));
    assert_eq!(check, (true, sentence));
    assert!(started.elapsed() < AT_ONCE, "{:?}", started.elapsed());
    // A later row is named by its own number, and a span a millionth of a
    // millisecond short is still refused.
    let (sentence, _) = stopped(&mut local, &slab_request(json!([0.0, 1.0, 1.0005, 2.0])));
    assert!(sentence.starts_with("Physical Violation: row 2 of the heating history is "), "{sentence}");
    assert!(sentence.ends_with("the least time between two rows is 0.001 s (HistoryRowsTooClose)."), "{sentence}");
    let (sentence, _) = stopped(&mut local, &slab_request(json!([0.0, 1.0, 1.000999999, 2.0])));
    assert!(sentence.ends_with("(HistoryRowsTooClose)."), "{sentence}");
    // Rows at the same time keep the refusal they had.
    let (sentence, _) = stopped(&mut local, &slab_request(json!([0.0, 1.0, 1.0, 2.0])));
    assert!(sentence.ends_with("(HistoryTimesOutOfOrder)."), "{sentence}");

    // Rows written at 1 and 1.001 s are a hair under a millisecond apart as
    // binary numbers, and are answered with a proof that holds.
    assert!(1.001_f64 - 1.0 < slab::HISTORY_ROW_SPAN_MIN_S);
    let apart = slab_request(json!([0.0, 1.0, 1.001, 2.0]));
    let body = answered(&mut local, &apart);
    assert_eq!(body["outcome"], "converged", "{body}");
    assert_eq!(body["engine_version"], "genesis_solvers_v0.13.0");
    assert!(body["iterations"].as_u64().unwrap() > 0);
    assert_eq!(body["result"]["time_steps"].as_f64(), body["iterations"].as_f64());
    let (error, text) = local.tool("verify_solution", &json!({ "request": apart, "solution_seal": body["solution_seal"] }));
    assert!(!error, "{text}");
    let check: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(check["holds"], true, "{text}");

    // describe_solver names the refusal, how it is reported, and the floor.
    let short = described(&mut local, "ablator_slab");
    assert_eq!(
        short["more"]["refusals"],
        "The 34 named refusals with the solver's sentence for each: 23 a request itself shows (422, no proof) and 11 found while computing (200, outcome `refused`, no result)."
    );
    let part = |door: &mut Stdio2, part: &str| -> Value {
        let (error, text) = door.tool("describe_solver", &json!({ "solver": "ablator_slab", "part": part }));
        assert!(!error, "{text}");
        serde_json::from_str(&text).unwrap()
    };
    let described = part(&mut local, "refusals");
    let locked = part(&mut local, "locked");
    let listed = refusal(&described, "HistoryRowsTooClose");
    assert_eq!(listed["reported_as"], "422, no seal");
    assert_eq!(
        listed["meaning"],
        "A row of the heating history is less than 0.001 s after the row before it. A row at the same time as the one before it, or earlier, is `HistoryTimesOutOfOrder`."
    );
    assert_eq!(locked["locked"]["constants_named_in_refusals"]["HISTORY_ROW_SPAN_MIN_S"], 0.001);
    let names: Vec<&str> = described["refusals"].as_array().unwrap().iter().map(|refusal| refusal["name"].as_str().unwrap()).collect();
    // After the solver's 34 names the list holds the two the entry names itself, UnknownFace and UnknownStreamGas.
    assert_eq!(names.len(), 36);
    assert_eq!(&names[14..17], ["HistoryTimesOutOfOrder", "HistoryDurationOutOfRange", "HistoryRowsTooClose"]);
    let bound = input(&short, "history_time_s")["bound"].as_str().unwrap();
    assert!(
        bound.ends_with("Time of each row: zero first, each 0.001 s or more after the one before, or it is refused as HistoryRowsTooClose (rows written 1 and 1.001 s are taken), the last from 1 s to an hour, a size cap on the request."),
        "{bound}"
    );
}
