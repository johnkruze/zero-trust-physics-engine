//! The coupling stardust_entry_to_pica_slab through the local connection:
//! the capsule_stardust card's sealed entry handed into the ablator_slab
//! solver on the PICA model under one chain proof. A true chain holds by
//! request and in the full check; a forged upstream and a tampered handoff
//! are named; each condition names the request that does not meet it.

mod conformance_support;

use conformance_support::Door;
use serde_json::{Value, json};

const COUPLING: &str = "stardust_entry_to_pica_slab";

/// The worked chain's numbers: the slab's wear at the near-nose core and the
/// chain seal. The chain seal commits the solve's solution seal, which covers
/// engine_version: a55272ce...4781 on genesis_solvers 0.12.0 (hyper_chain),
/// e7d40c9d...f150 on 0.13.0, the same run seal and the same wear.
const WORKED_WEAR_M: f64 = 0.004647861283103977;
// Seal rule v2 (0.15.0): SHA-384; e7d40c9d...f150 under rule v1.
const WORKED_CHAIN_SEAL: &str = "21a8ace2dc1223ed33f32f97173269980e469ca11d23bed4ccbcf41544fee9e55a3228621dd12a52452ffec09ddf80b1";

/// The card from entry interface at its ceiling step to the drogue end,
/// a frame every 1.5 s.
fn upstream(record_every: u64) -> Value {
    json!({
        "domain": "aerospace", "card": "capsule_stardust",
        "initial": { "radius_m": 6503140.0, "radial_speed_m_s": -1842.1416729435698, "transverse_speed_m_s": 12767.792058801891 },
        "dt_s": 0.05,
        "segments": [{ "steps": 2779, "load": {} }],
        "record_every": record_every
    })
}

fn slot(bytes: &[u8], at: usize) -> f64 {
    f64::from(f32::from_le_bytes(bytes[at..at + 4].try_into().unwrap()))
}

/// Each frame's time, stagnation pressure, recovery enthalpy and
/// heat-transfer coefficient.
fn frames(card: &Value) -> [Vec<f64>; 4] {
    let mut out: [Vec<f64>; 4] = Default::default();
    for frame in card["frames"].as_array().unwrap() {
        let hex = frame["frame_hex"].as_str().unwrap();
        let bytes: Vec<u8> = (0..hex.len()).step_by(2).map(|i| u8::from_str_radix(&hex[i..i + 2], 16).unwrap()).collect();
        out[0].push(f64::from_le_bytes(bytes[0..8].try_into().unwrap()));
        out[1].push(slot(&bytes, 12));
        out[2].push(slot(&bytes, 20));
        out[3].push(slot(&bytes, 24));
    }
    out
}

/// The slab's question: row 0 at 0 s unheated, the frames, then the stream
/// off one second after the last frame and the shield cooling to 600 s.
fn parameters(card: &Value) -> Value {
    let [time, pressure, enthalpy, coefficient] = frames(card);
    let last = time.len() - 1;
    let column = |first: f64, values: &[f64], after: f64| {
        let mut out = vec![first];
        out.extend_from_slice(values);
        out.extend([after, after]);
        out
    };
    let rows = time.len() + 3;
    let mut times = vec![0.0];
    times.extend_from_slice(&time);
    times.extend([time[last] + 1.0, 600.0]);
    json!({
        "material": "PICA, open record, version 5",
        "thickness_m": 0.0582,
        "start_temperature_k": 260.0,
        "thermocouple_depths_m": [0.005, 0.01, 0.02, 0.0582],
        "face": "gas_stream",
        "stream_gas": "air",
        "history_time_s": times,
        "history_pressure_pa": column(pressure[0], &pressure, pressure[last]),
        "history_heat_transfer_coefficient_kg_per_m2_s": column(0.0, &coefficient, 0.0),
        "history_flow_enthalpy_j_per_kg": column(enthalpy[0], &enthalpy, enthalpy[last]),
        "history_radiative_heating_w_per_m2": vec![0.0; rows],
        "surroundings_temperature_k": 266.48,
        "face_wears": true,
        "return_face_history": false,
        "return_thermocouple_histories": true,
        "return_final_profile": false
    })
}

fn chained(parameters: Value, up: &Value, seal: &str) -> Value {
    json!({
        "solver": "ablator_slab",
        "parameters": parameters,
        "from": { "request": up, "run_seal": seal, "coupling": COUPLING }
    })
}

fn refused(door: &mut Door, request: &Value, name: &str) -> String {
    let (error, text) = door.call("solve", request);
    assert!(error && text.contains(&format!("({name})")), "{name}: {text}");
    text
}

#[test]
fn the_card_s_entry_is_handed_to_the_pica_slab_under_one_chain_proof() {
    let mut door = Door::start();
    let up = upstream(30);
    let card = door.value("run_trajectory", &up).expect("the card runs");
    assert_eq!((card["outcome"].clone(), card["reject"].clone()), (json!("ended"), json!("PastFlightDeployment")));
    let seal = card["run_seal"].as_str().unwrap().to_string();
    let request = chained(parameters(&card), &up, &seal);
    let answer = door.value("solve", &request).expect("the chained solve");
    assert_eq!(answer["outcome"], "converged", "{answer}");
    let chain = &answer["chain"];
    assert_eq!((chain["kind"].clone(), chain["coupling"].clone()), (json!("coupling"), json!(COUPLING)));
    assert_eq!(chain["upstream_run_seal"], seal.as_str());
    let handed: Vec<(&str, &str, &str)> = chain["handed"]
        .as_array()
        .unwrap()
        .iter()
        .map(|entry| (entry["from"].as_str().unwrap(), entry["to"].as_str().unwrap(), entry["unit"].as_str().unwrap()))
        .collect();
    assert_eq!(
        handed,
        [
            ("frames[*].t", "parameters.history_time_s[1..]", "s"),
            ("frames[*].pos[1]", "parameters.history_pressure_pa[1..]", "Pa"),
            ("frames[*].vel[1]", "parameters.history_heat_transfer_coefficient_kg_per_m2_s[1..]", "kg/m²/s"),
            ("frames[*].vel[0]", "parameters.history_flow_enthalpy_j_per_kg[1..]", "J/kg"),
        ]
    );
    // 93 frames, a frame every 1.5 s and the last landed step at 138.95 s.
    assert_eq!(chain["handed"][0]["value_hex"].as_array().unwrap().len(), 93);
    // The card's numbers at the nose: the stagnation pressure peaks at 34.55
    // kPa, the recovery enthalpy at 79.05 MJ/kg and the coefficient at 0.1407 kg/m²/s.
    let [_, pressure, enthalpy, coefficient] = frames(&card);
    let most = |values: &[f64]| values.iter().copied().fold(0.0, f64::max);
    assert!((most(&pressure) - 34_554.86).abs() < 0.1, "{}", most(&pressure));
    assert!((most(&enthalpy) - 79.0457e6).abs() < 1e3, "{}", most(&enthalpy));
    assert!((most(&coefficient) - 0.14073).abs() < 1e-5, "{}", most(&coefficient));
    // The slab's answer: the wear at the near-nose core, outside the
    // envelope of the Stardust comparisons, which used NASA's heating.
    let result = &answer["result"];
    assert_eq!(result["wear_m"].as_f64().unwrap().to_bits(), WORKED_WEAR_M.to_bits(), "{}", result["wear_m"]);
    assert_eq!(result["inside_stardust_comparison"], 0.0);
    let peak = result["peak_face_temperature_k"].as_f64().unwrap();
    assert!(peak > 3_100.0 && peak < 3_250.0, "{peak}");
    assert_eq!(chain["chain_seal"], WORKED_CHAIN_SEAL, "{}", chain["chain_seal"]);
    // The same request typed by hand has the same solution seal and no chain.
    let mut by_hand = request.clone();
    by_hand.as_object_mut().unwrap().remove("from");
    let plain = door.value("solve", &by_hand).expect("by hand");
    assert_eq!(plain["solution_seal"], answer["solution_seal"]);
    assert!(plain.get("chain").is_none());
    // Verified by request: the card run computed again, then the solve.
    let verified = door.value("verify_solution", &json!({ "request": request, "solution_seal": answer["solution_seal"] })).expect("verified");
    assert_eq!(verified["holds"], true, "{verified}");
    assert_eq!(verified["recomputed_chain_seal"], chain["chain_seal"]);
    let tape = door.value("verify_tape", &json!({ "request": up, "run_seal": seal })).expect("tape");
    assert_eq!(tape["holds"], true);
}

/// The full check of the chained solve: a true answer holds with its chain
/// seal recomputed; a forged upstream seal and a tampered handoff fail by
/// name.
#[test]
fn the_full_check_holds_and_names_a_forged_upstream_and_a_tampered_handoff() {
    let mut door = Door::start();
    let up = upstream(30);
    let card = door.value("run_trajectory", &up).expect("the card runs");
    let seal = card["run_seal"].as_str().unwrap().to_string();
    let request = chained(parameters(&card), &up, &seal);
    let from = request["from"].clone();
    let (error, chained_text) = door.call("solve", &request);
    assert!(!error, "{chained_text}");
    let answer: Value = serde_json::from_str(&chained_text).unwrap();
    let with_from = |answer: &Value, from: &Value| {
        let mut whole = answer.clone();
        whole.as_object_mut().unwrap().insert("from".into(), from.clone());
        whole
    };
    // A true chain holds, sent back as it came with its from.
    let whole = format!("{},\"from\":{}}}", &chained_text[..chained_text.len() - 1], from);
    let (error, verified) = door.call_text("verify_solution", &whole);
    assert!(!error, "{verified}");
    let verified: Value = serde_json::from_str(&verified).unwrap();
    assert_eq!((verified["holds"].clone(), verified["values_hold"].clone(), verified["cause"].clone()), (json!(true), json!(true), Value::Null), "{verified}");
    assert_eq!(verified["recomputed_chain_seal"], answer["chain"]["chain_seal"]);
    // A forged upstream: the card run computed again is not the seal given.
    let mut forged = from.clone();
    forged["run_seal"] = json!("0".repeat(96));
    let report = door.value("verify_solution", &with_from(&answer, &forged)).expect("a report");
    assert_eq!((report["holds"].clone(), report["cause"].clone(), report["differs_at"].clone()), (json!(false), json!("chain"), json!("from")), "{report}");
    assert!(report["difference"].as_str().unwrap().contains("(UpstreamProofBroken)"), "{report}");
    let mut forged_request = request.clone();
    forged_request["from"]["run_seal"] = json!("0".repeat(96));
    refused(&mut door, &forged_request, "UpstreamProofBroken");
    // An upstream request changed under its seal: a later start is another entry.
    let mut moved = request.clone();
    moved["from"]["request"]["initial"]["radius_m"] = json!(6503000.0);
    refused(&mut door, &moved, "UpstreamProofBroken");
    // A tampered handoff: the peak coefficient raised by a tenth, solved by
    // hand and sealed as the slab's own answer, claiming the true chain.
    let coefficients = request["parameters"]["history_heat_transfer_coefficient_kg_per_m2_s"].as_array().unwrap();
    let (at, peak) = coefficients.iter().enumerate().map(|(at, value)| (at, value.as_f64().unwrap())).fold((0, 0.0), |best, row| if row.1 > best.1 { row } else { best });
    let mut tampered = request.clone();
    tampered["parameters"]["history_heat_transfer_coefficient_kg_per_m2_s"][at] = json!(peak * 1.1);
    refused(&mut door, &tampered, "CouplingLedgerBreach");
    let mut by_hand = tampered.clone();
    by_hand.as_object_mut().unwrap().remove("from");
    let mut resealed = door.value("solve", &by_hand).expect("solved by hand");
    assert_ne!(resealed["result"]["wear_m"], answer["result"]["wear_m"]);
    resealed["chain"] = answer["chain"].clone();
    let report = door.value("verify_solution", &with_from(&resealed, &from)).expect("a report");
    assert_eq!((report["holds"].clone(), report["cause"].clone(), report["differs_at"].clone()), (json!(false), json!("chain"), json!("from")), "{report}");
    assert!(report["difference"].as_str().unwrap().contains("(CouplingLedgerBreach)"), "{report}");
    // The same value changed under the seal it came with: its seal part fails.
    let mut stale = answer.clone();
    stale["inputs"]["parameters"]["history_heat_transfer_coefficient_kg_per_m2_s"][at] = json!(peak * 1.1);
    let report = door.value("verify_solution", &with_from(&stale, &from)).expect("a report");
    assert_eq!((report["holds"].clone(), report["cause"].clone()), (json!(false), json!("inputs")), "{report}");
    // A time shifted by one frame: the history is not the card's.
    let mut shifted = request.clone();
    let times = shifted["parameters"]["history_time_s"].as_array_mut().unwrap();
    times.remove(1);
    times.insert(times.len() - 2, json!(139.0));
    refused(&mut door, &shifted, "CouplingLedgerBreach");
}

/// Each condition names the request that does not meet it, before the slab
/// runs; a value outside the slab's table is named as outside its range.
#[test]
fn each_condition_and_the_slab_s_range_are_named() {
    let mut door = Door::start();
    let up = upstream(30);
    let card = door.value("run_trajectory", &up).expect("the card runs");
    let seal = card["run_seal"].as_str().unwrap().to_string();
    let base = parameters(&card);
    let with = |change: &dyn Fn(&mut Value)| {
        let mut parameters = base.clone();
        change(&mut parameters);
        chained(parameters, &up, &seal)
    };
    // TACOT 3.0 is refused by the slab's own pressure range first, below
    // 101.325 Pa; its condition is held in the couplings' own tests.
    let cases: [(&str, Box<dyn Fn(&mut Value)>, &str); 5] = [
        ("mixture", Box::new(|p: &mut Value| {
            p["stream_gas"] = json!("mixture");
            p["stream_oxygen_mass_fraction"] = json!(0.2);
            p["stream_argon_mass_fraction"] = json!(0.0);
        }), "the solve states stream_gas 'mixture'"),
        ("heated at 0 s", Box::new(|p: &mut Value| p["history_heat_transfer_coefficient_kg_per_m2_s"][0] = json!(0.001)), "the history's first row is at 0 s with a coefficient of 0.001"),
        ("radiation", Box::new(|p: &mut Value| p["history_radiative_heating_w_per_m2"][40] = json!(1.0e5)), "row 40 of the history holds a radiative heating of 100000"),
        ("heated after", Box::new(|p: &mut Value| p["history_heat_transfer_coefficient_kg_per_m2_s"][94] = json!(0.01)), "row 94, after the 93 rows handed, holds a coefficient of 0.01"),
        ("cooled last row", Box::new(|p: &mut Value| p["history_heat_transfer_coefficient_kg_per_m2_s"][95] = json!(0.002)), "row 95, after the 93 rows handed"),
    ];
    for (what, change, instead) in cases {
        let text = refused(&mut door, &with(&*change), "CouplingConditionNotMet");
        assert!(text.contains(instead), "{what}: {text}");
        assert!(text.contains(&format!("the coupling '{COUPLING}'")), "{what}: {text}");
    }
    // A frame a second after entry interface: 0.99 Pa at the nose, below the
    // PICA surface table, named by the slab's own check of the request
    // before anything runs.
    let up = upstream(20);
    let card = door.value("run_trajectory", &up).expect("the card runs");
    let mut parameters = parameters(&card);
    parameters["history_pressure_pa"][0] = json!(1.5);
    let text = refused(&mut door, &chained(parameters, &up, card["run_seal"].as_str().unwrap()), "StreamPressureOutOfRange");
    assert!(text.contains("row 1 of the heating history has pressure 0.9945328235626221 Pa"), "{text}");
}

/// describe_solver lists the coupling under the slab's couplings, with what
/// it hands and its conditions; describe_card does not list a coupling into a
/// solver.
#[test]
fn the_slab_s_description_lists_the_coupling() {
    let mut door = Door::start();
    let described = door.value("describe_solver", &json!({ "solver": "ablator_slab" })).expect("described");
    let couplings = described["couplings"].as_array().expect("couplings");
    let entry = couplings.iter().find(|entry| entry["name"] == COUPLING).expect("the coupling is listed");
    assert_eq!(entry["upstream"], "aerospace/capsule_stardust");
    assert_eq!(entry["downstream"], "solver/ablator_slab");
    assert_eq!(entry["handed"].as_array().unwrap().len(), 4);
    assert_eq!(entry["conditions"].as_array().unwrap().len(), 5);
    assert!(entry["meaning"].as_str().unwrap().contains("wear 4.65 mm against 5.7 (6.1 by X-ray)"), "{}", entry["meaning"]);
    let card = door.value("describe_card", &json!({ "domain": "aerospace", "card": "capsule_stardust" })).expect("described");
    assert!(!card.to_string().contains(COUPLING));
}
