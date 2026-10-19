//! What `describe_card` says about the bounds of a card, its validation, and
//! the refusals the description names: each statement is held to the step.

use std::time::Duration;

use protocol_mcp_gateway::app;
use reqwest::Client;
use serde_json::{Value, json};
use tokio::net::TcpListener;

async fn spawn_app() -> String {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    tokio::spawn(async move {
        axum::serve(listener, app()).await.unwrap();
    });
    tokio::task::yield_now().await;
    format!("http://127.0.0.1:{port}")
}

async fn post(base: &str, path: &str, body: &Value) -> (u16, String) {
    let client = Client::new();
    let mut last_error = None;
    for _ in 0..20 {
        match client
            .post(format!("{base}{path}"))
            .header(reqwest::header::CONTENT_TYPE, "application/json")
            .body(serde_json::to_string(body).unwrap())
            .send()
            .await
        {
            Ok(response) => {
                let status = response.status().as_u16();
                return (status, response.text().await.unwrap());
            }
            Err(error) => {
                last_error = Some(error);
                tokio::time::sleep(Duration::from_millis(25)).await;
            }
        }
    }
    panic!("never accepted a connection: {last_error:?}");
}

async fn describe(base: &str, domain: &str, card: &str) -> Value {
    let (status, text) = post(base, "/describe_card", &json!({"domain": domain, "card": card})).await;
    assert_eq!(status, 200, "{text}");
    serde_json::from_str(&text).unwrap()
}

/// One step: the status and the text.
async fn step(base: &str, domain: &str, card: &str, parameters: Value) -> (u16, String) {
    post(
        base,
        "/verify_physics",
        &json!({"domain": domain, "card": card, "parameters": parameters}),
    )
    .await
}

fn field<'a>(description: &'a Value, list: &str, name: &str) -> &'a Value {
    description[list]
        .as_array()
        .unwrap()
        .iter()
        .find(|field| field["name"] == name)
        .unwrap_or_else(|| panic!("no field {name}"))
}

fn above(x: f64) -> f64 {
    f64::from_bits(x.to_bits() + 1)
}

fn below(x: f64) -> f64 {
    f64::from_bits(x.to_bits() - 1)
}

fn number(value: &Value) -> f64 {
    value.as_f64().expect("a number")
}

fn floor_of(description: &Value) -> f64 {
    number(&description["dt_s"]["floor_s"])
}

/// A grid plan of one axis: a run whose `initial` holds `initial`, one step of
/// `load`, and the values of `axis` (a path such as `initial.pressure_pa`).
fn plan_of(initial: Value, dt_s: f64, load: Value, axis: &str, values: Vec<f64>) -> Value {
    json!({
        "kind": "grid",
        "base": {
            "initial": initial,
            "dt_s": dt_s,
            "segments": [{"steps": 1, "load": load}],
            "record_every": 1
        },
        "axes": [{"path": axis, "values": values}]
    })
}

/// A grid row with a value past a published bound is admitted row by row:
/// the job answers, and the row is unadmitted with the door's sentence, which
/// names the field.
fn unadmitted_row(status: u16, text: &str, field: &str) {
    assert_eq!(status, 200, "{text}");
    let answer: Value = serde_json::from_str(text).unwrap();
    let row = &answer["rows"][0];
    // A value the door turns away is unadmitted; one the card itself refuses
    // at its first step is a refused row that landed no step.
    match row["outcome"].as_str() {
        Some("unadmitted") => {
            assert_eq!(row["run_seal"], Value::Null, "{text}");
            assert!(row["reject_detail"].as_str().unwrap().contains(field), "{text}");
        }
        // A step from the last day of the card ends the run there.
        Some("refused") | Some("ended") => {
            assert_eq!(row["steps_landed"], 0, "{text}");
            assert!(row["reject"].is_string(), "{text}");
        }
        _ => panic!("{text}"),
    }
}

async fn dataset(base: &str, domain: &str, card: &str, plan: Value) -> (u16, String) {
    post(
        base,
        "/run_dataset",
        &json!({"domain": domain, "card": card, "plan": plan, "store": "none", "return": "inline", "answer": "full"}),
    )
    .await
}

#[tokio::test]
async fn the_oil_column_publishes_its_pressure_ceiling_and_the_plan_is_held_to_it() {
    let base = spawn_app().await;
    let d = describe(&base, "materials", "hydraulic_fluid").await;
    let pressure = field(&d, "phase_fields", "pressure_pa");
    let ceiling = number(&pressure["maximum"]);
    assert_eq!(pressure["maximum_exclusive"], false);
    assert!((ceiling - 20.64e6).abs() < 1.0e6, "{ceiling}");
    let floor = floor_of(&d);
    let params = |p: f64| json!({"pressure_pa": p, "flow_m3_s": 0.0, "dt_s": floor});
    let (status, text) = step(&base, "materials", "hydraulic_fluid", params(ceiling)).await;
    assert_eq!(status, 200, "{text}");
    let (status, text) = step(&base, "materials", "hydraulic_fluid", params(above(ceiling))).await;
    assert_eq!(status, 422, "{text}");
    assert!(text.contains("PressureAboveCard") || text.contains("ceiling"), "{text}");
    // A grid row with a value above the published ceiling is not run.
    let load = json!({"flow_m3_s": 0.0});
    let (status, text) = dataset(
        &base,
        "materials",
        "hydraulic_fluid",
        plan_of(json!({}), floor, load.clone(), "initial.pressure_pa", vec![above(ceiling)]),
    )
    .await;
    unadmitted_row(status, &text, "pressure_pa");
    let (status, text) = dataset(
        &base,
        "materials",
        "hydraulic_fluid",
        plan_of(json!({}), floor, load, "initial.pressure_pa", vec![ceiling]),
    )
    .await;
    assert_eq!(status, 200, "{text}");
}

/// The oil column publishes a bound on the flow itself, the bore speed of
/// 0.012 of the wave speed, and refuses past it by name at the door, in a run
/// segment, and in a plan. The two strokes sealed on 0.6.0 are refused.
#[tokio::test]
async fn the_oil_column_refuses_a_flow_above_its_acoustic_limit_by_name() {
    let base = spawn_app().await;
    let d = describe(&base, "materials", "hydraulic_fluid").await;
    let flow = field(&d, "load_fields", "flow_m3_s");
    let max = number(&flow["maximum"]);
    let min = number(&flow["minimum"]);
    assert_eq!(flow["maximum_exclusive"], false);
    assert_eq!(flow["minimum_exclusive"], false);
    assert_eq!(min.to_bits(), (-max).to_bits());
    assert!((max - 0.0331).abs() < 1.0e-4, "{max}");
    let text = d.to_string();
    assert!(text.contains("FlowAboveAcoustic"), "the reject is listed");
    let floor = floor_of(&d);
    let ceiling_s = number(&d["dt_s"]["ceiling_s"]);
    let params = |q: f64, dt: f64| json!({"pressure_pa": 1.0e7, "flow_m3_s": q, "dt_s": dt});
    for dt in [floor, ceiling_s] {
        for q in [max, min] {
            let (status, text) = step(&base, "materials", "hydraulic_fluid", params(q, dt)).await;
            assert_eq!(status, 200, "{q} {dt} {text}");
        }
        for q in [above(max), -above(max)] {
            let (status, text) = step(&base, "materials", "hydraulic_fluid", params(q, dt)).await;
            assert_eq!(status, 422, "{q} {dt} {text}");
            assert!(text.contains("FlowAboveAcoustic"), "{text}");
        }
    }
    // A 3.0 mm stroke at the step floor, and the largest stroke flow at the
    // step ceiling, 0.94 of the wave speed.
    for (q, dt) in [(2728384.2125486047, floor), (2.6019899487958953, ceiling_s)] {
        let (status, text) = step(
            &base,
            "materials",
            "hydraulic_fluid",
            json!({"pressure_pa": 101325.0, "flow_m3_s": q, "dt_s": dt}),
        )
        .await;
        assert_eq!(status, 422, "{text}");
        assert!(text.contains("FlowAboveAcoustic"), "{text}");
    }
    // A second segment past the bound refuses the run whole at the door.
    let run = json!({
        "domain": "materials",
        "card": "hydraulic_fluid",
        "initial": {"pressure_pa": 101325.0},
        "dt_s": ceiling_s,
        "segments": [
            {"steps": 2, "load": {"flow_m3_s": max}},
            {"steps": 2, "load": {"flow_m3_s": above(max)}}
        ],
        "record_every": 1
    });
    let (status, text) = post(&base, "/run_trajectory", &run).await;
    assert_eq!(status, 422, "{text}");
    assert!(text.contains("FlowAboveAcoustic"), "{text}");
    // A grid is admitted row by row: the row whose flow leaves the bound is
    // unadmitted and carries no run seal, and the row on the bound runs.
    let (status, text) = dataset(
        &base,
        "materials",
        "hydraulic_fluid",
        plan_of(
            json!({"pressure_pa": 101325.0}),
            ceiling_s,
            json!({"flow_m3_s": 0.0}),
            "segments[0].load.flow_m3_s",
            vec![max, above(max)],
        ),
    )
    .await;
    assert_eq!(status, 200, "{text}");
    let answer: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(answer["rows"][0]["outcome"], "completed", "{text}");
    assert_eq!(answer["rows"][1]["outcome"], "unadmitted", "{text}");
    assert!(answer["rows"][1]["run_seal"].is_null(), "{text}");
    assert!(answer["rows"][1]["reject_detail"].as_str().unwrap().contains("flow_m3_s"), "{text}");
}

/// The two column flags carry their thresholds, and the description says
/// that neither is a limit and the card stands to 6.0 mm.
#[tokio::test]
async fn the_oil_column_flags_state_their_thresholds_and_are_not_limits() {
    let base = spawn_app().await;
    let d = describe(&base, "materials", "hydraulic_fluid").await;
    let bound = field(&d, "phase_fields", "pressure_pa")["bound"].as_str().unwrap().to_string();
    let text = d.to_string();
    for sentence in [
        "FLAG_COLUMN_SOFT is set at or above 1.8 mm of compression and FLAG_COLUMN_LIMIT at or above 3.0 mm",
        "Neither",
        "6.0 mm",
        "PressureAboveCard",
    ] {
        assert!(bound.contains(sentence), "{bound}");
        assert!(text.contains(sentence));
    }
    // A state on 3.0 mm and one on 6.0 mm both seal with both flags.
    let floor = floor_of(&d);
    for p in [10_421_325.0, 20_741_325.0] {
        let (status, text) = step(
            &base,
            "materials",
            "hydraulic_fluid",
            json!({"pressure_pa": p, "flow_m3_s": 0.0, "dt_s": floor}),
        )
        .await;
        assert_eq!(status, 200, "{text}");
        assert!(text.contains("column_limited"), "{text}");
    }
}

#[tokio::test]
async fn the_spring_mount_publishes_the_largest_displacement_and_velocity_it_can_hold() {
    let base = spawn_app().await;
    let d = describe(&base, "signal", "resonance").await;
    let x = field(&d, "phase_fields", "displacement_m");
    let v = field(&d, "phase_fields", "velocity_m_s");
    let x_max = number(&x["maximum"]);
    let v_max = number(&v["maximum"]);
    assert_eq!(number(&x["minimum"]), -x_max);
    assert_eq!(number(&v["minimum"]), -v_max);
    assert!((x_max - 2.0 / 64.0).abs() < 1.0e-6, "{x_max}");
    let floor = floor_of(&d);
    // A new start: no time, no booked work, the anchor at its own energy, 0.5 v^2 + 0.5 k x^2.
    let k = (20.0 * std::f64::consts::PI) * (20.0 * std::f64::consts::PI);
    let params = |x: f64, v: f64| json!({"displacement_m": x, "velocity_m_s": v, "t_s": 0.0, "energy_anchor_j": 0.5 * v * v + 0.5 * k * x * x, "boundary_work_j": 0.0, "accel_m_s2": 0.0, "dt_s": floor});
    for (x_at, v_at) in [(x_max, 0.0), (-x_max, 0.0), (0.0, v_max), (0.0, -v_max)] {
        let (status, text) = step(&base, "signal", "resonance", params(x_at, v_at)).await;
        assert_eq!(status, 200, "{x_at} {v_at}: {text}");
    }
    for (x_at, v_at) in [(above(x_max), 0.0), (-above(x_max), 0.0), (0.0, above(v_max)), (0.0, -above(v_max))] {
        let (status, text) = step(&base, "signal", "resonance", params(x_at, v_at)).await;
        assert_eq!(status, 422, "{x_at} {v_at}: {text}");
        assert!(text.contains("EnergyAboveCard"), "{text}");
    }
    let load = json!({"accel_m_s2": 0.0});
    let (status, text) = dataset(
        &base,
        "signal",
        "resonance",
        plan_of(json!({"velocity_m_s": 0.0, "t_s": 0.0, "energy_anchor_j": 0.0, "boundary_work_j": 0.0}), floor, load.clone(), "initial.displacement_m", vec![above(x_max)]),
    )
    .await;
    unadmitted_row(status, &text, "displacement_m");
    let (status, text) = dataset(
        &base,
        "signal",
        "resonance",
        plan_of(json!({"displacement_m": 0.0, "t_s": 0.0, "energy_anchor_j": 0.0, "boundary_work_j": 0.0}), floor, load, "initial.velocity_m_s", vec![above(v_max)]),
    )
    .await;
    unadmitted_row(status, &text, "velocity_m_s");
}

#[tokio::test]
async fn the_battery_pack_publishes_its_charging_limit_and_no_charging_current_beyond_it_lands() {
    let base = spawn_app().await;
    let d = describe(&base, "energy", "battery_sag").await;
    let current = field(&d, "load_fields", "current_a");
    let limit_a = number(&current["minimum"]);
    assert_eq!(current["minimum_exclusive"], true);
    assert!((limit_a + 120.0).abs() < 1.0e-9, "{limit_a}");
    assert!(current["bound"].as_str().unwrap().contains("5.04e1"), "{}", current["bound"]);
    let floor = floor_of(&d);
    for soc in [1.0, 0.9999, 0.9, 0.5, 0.1, 0.001, 1.0e-6] {
        for amps in [limit_a, below(limit_a), 2.0 * limit_a] {
            let (status, text) = step(
                &base,
                "energy",
                "battery_sag",
                json!({"soc": soc, "current_a": amps, "dt_s": floor}),
            )
            .await;
            assert_eq!(status, 422, "soc {soc} current {amps}: {text}");
        }
    }
    // A grid row on the exclusive bound itself is not run.
    let (status, text) = dataset(
        &base,
        "energy",
        "battery_sag",
        plan_of(json!({"soc": 0.5}), floor, json!({}), "segments[0].load.current_a", vec![limit_a]),
    )
    .await;
    unadmitted_row(status, &text, "current_a");
}

#[tokio::test]
async fn the_published_top_of_the_generator_and_the_satellite_is_usable_and_the_next_number_is_not() {
    let base = spawn_app().await;
    let generator = describe(&base, "energy", "generator_mmrtg_f1").await;
    let mission = field(&generator, "phase_fields", "mission_time_s");
    let top = number(&mission["maximum"]);
    // The top is where the decay heat left is the ledger's resolution, some 2,070 years on.
    assert!(top > 6.0e10 && top < 7.0e10, "{top}");
    assert_eq!(mission["maximum_exclusive"], false);
    let floor = floor_of(&generator);
    let (status, text) = step(
        &base,
        "energy",
        "generator_mmrtg_f1",
        json!({"mission_time_s": top, "dt_s": floor}),
    )
    .await;
    assert_eq!(status, 200, "{text}");
    let (status, text) = step(
        &base,
        "energy",
        "generator_mmrtg_f1",
        json!({"mission_time_s": above(top), "dt_s": floor}),
    )
    .await;
    assert_eq!(status, 422, "{text}");
    assert!(text.contains("DecayHeatBelowLedgerResolution"), "{text}");
    let (status, text) = dataset(
        &base,
        "energy",
        "generator_mmrtg_f1",
        plan_of(json!({}), floor, json!({}), "initial.mission_time_s", vec![above(top)]),
    )
    .await;
    unadmitted_row(status, &text, "mission_time_s");

    let satellite = describe(&base, "aerospace", "satellite_lageos1").await;
    let epoch = field(&satellite, "phase_fields", "epoch_utc_s");
    let top = number(&epoch["maximum"]);
    assert!(top < 867_412_800.0 && 867_412_800.0 - top < 2.0 * floor_of(&satellite));
    let floor = floor_of(&satellite);
    let rest = |epoch_utc_s: f64| {
        json!({
            "epoch_utc_s": epoch_utc_s,
            "forecast_elapsed_s": 0.0,
            "position_m": [3767126.6844992605, 8956617.400906362, 7564717.856833777],
            "velocity_m_s": [4119.825668508274, 1676.7840658085788, -4053.2216929389388],
            "dt_s": floor
        })
    };
    let (status, text) = step(&base, "aerospace", "satellite_lageos1", rest(top)).await;
    assert_eq!(status, 200, "{text}");
    let (status, text) = step(&base, "aerospace", "satellite_lageos1", rest(above(top))).await;
    assert_eq!(status, 422, "{text}");
    assert!(text.contains("EpochOutsideLeapSecondSpan"), "{text}");
    let (status, text) = dataset(
        &base,
        "aerospace",
        "satellite_lageos1",
        plan_of(
            json!({"forecast_elapsed_s": 0.0,
                   "position_m": [3767126.6844992605, 8956617.400906362, 7564717.856833777],
                   "velocity_m_s": [4119.825668508274, 1676.7840658085788, -4053.2216929389388]}),
            floor,
            json!({}),
            "initial.epoch_utc_s",
            vec![above(top)],
        ),
    )
    .await;
    unadmitted_row(status, &text, "epoch_utc_s");
}

#[tokio::test]
async fn the_satellite_states_the_step_error_that_was_measured() {
    let base = spawn_app().await;
    let d = describe(&base, "aerospace", "satellite_lageos1").await;
    let text = d["dt_s"]["ceiling_from"].as_str().unwrap();
    assert!(text.contains("11.2 mm after one day"), "{text}");
    assert!(text.contains("13.9 cm after seven days"), "{text}");
    assert!(text.contains("not yet reviewed"), "{text}");
    assert!(!text.contains("6 mm"), "{text}");
}

#[tokio::test]
async fn the_cell_names_a_step_of_zero_or_less_and_takes_any_discharge_current() {
    let base = spawn_app().await;
    let d = describe(&base, "energy", "cell_18650pf").await;
    let names: Vec<&str> = d["rejects"].as_array().unwrap().iter().map(|r| r["name"].as_str().unwrap()).collect();
    assert!(names.contains(&"NonPositiveDt"));
    for dt_s in [0.0, -0.0, -1.0, -1.0e-300] {
        let (status, text) = step(&base, "energy", "cell_18650pf", json!({"soc": 0.5, "held_current_a": 2.9, "current_a": 2.9, "dt_s": dt_s})).await;
        assert_eq!(status, 422, "{text}");
        assert!(text.contains("NonPositiveDt"), "{dt_s}: {text}");
    }
    let floor = floor_of(&d);
    let (status, text) = step(&base, "energy", "cell_18650pf", json!({"soc": 0.5, "held_current_a": 2.9, "current_a": 2.9, "dt_s": floor / 2.0})).await;
    assert_eq!(status, 422, "{text}");
    assert!(!text.contains("NonPositiveDt") && text.contains("step floor"), "{text}");
    // A current too small to move the charge is a discharge: answered, at its own held current.
    for current_a in [f64::MIN_POSITIVE, 1.0e-306, 1.0e-304] {
        for soc in [0.5, 1.0, 0.0300001] {
            let (status, text) = step(&base, "energy", "cell_18650pf", json!({"soc": soc, "held_current_a": current_a, "current_a": current_a, "dt_s": floor})).await;
            assert_eq!(status, 200, "{current_a} A at {soc}: {text}");
        }
    }
    // At the floor a 1C step moves the charge, from full and from a rested cell below it.
    for soc in [1.0, 0.5] {
        let (status, text) = step(&base, "energy", "cell_18650pf", json!({"soc": soc, "held_current_a": 2.9, "current_a": 2.9, "dt_s": floor})).await;
        assert_eq!(status, 200, "{soc}: {text}");
        let body: Value = serde_json::from_str(&text).unwrap();
        assert!(body["state"]["soc"].as_f64().unwrap() < soc, "{text}");
    }
    // The two behaviours the description now states.
    let rule = d["dt_s"]["rule"].as_str().unwrap();
    assert!(rule.contains("refuses the whole run before its first step"), "{rule}");
    assert!(rule.contains("takes the state of charge and held_current_a alone"), "{rule}");
    let (status, text) = post(
        &base,
        "/run_trajectory",
        &json!({"domain": "energy", "card": "cell_18650pf", "initial": {"soc": 0.9, "held_current_a": 2.9}, "dt_s": 1.0,
                "segments": [{"steps": 5, "load": {"current_a": 2.9}}, {"steps": 5, "load": {"current_a": 3.5}}],
                "record_every": 1}),
    )
    .await;
    assert_eq!(status, 422, "{text}");
    assert!(text.contains("CurrentChangedDuringDischarge"), "{text}");
    assert!(!text.contains("run_seal"), "{text}");
}

#[tokio::test]
async fn all_twenty_one_cards_carry_a_validation_of_five_parts_and_nine_are_the_companys_own_specimens() {
    let base = spawn_app().await;
    let (status, text) = {
        let client = Client::new();
        let r = client.get(format!("{base}/cards")).send().await.unwrap();
        (r.status().as_u16(), r.text().await.unwrap())
    };
    assert_eq!(status, 200);
    let list: Value = serde_json::from_str(&text).unwrap();
    let cards = list["cards"].as_array().unwrap_or_else(|| list.as_array().unwrap());
    assert_eq!(cards.len(), 40);
    let fitted = ["cell_18650pf", "generator_mmrtg_f1", "generator_mmrtg_seebeck", "capsule_stardust", "satellite_lageos1", "cantilever_eth", "creep_cmsx6", "capsule_insight", "reactor_spert3_fuel", "reactor_spert3_twin", "crater_ryugu_sci", "reactor_spert3_ccore_twin", "momentum_jwst_dihedral", "momentum_jwst_srp", "link_gothenburg_10010", "solar_system", "los_jwst_fine_guide", "tokamak_jet_dte1", "fingerpad_p02_glass", "accumulator_sbo450", "reactor_triga_jsi", "reactor_spert3_ccore", "runaway_18650_lco", "grasp_lift_glass_4tu", "tokamak_vde_mast", "reactor_msre_pump"];
    // The reactor is fitted and has not been compared yet: its result says so.
    let awaiting = "reactor_spert3";
    // The 2170 cell was compared with its held-back tests and missed: its result says so.
    let compared_and_missed = "cell_2170_m80";
    // The physical 2170 cell was run once on runs of two further cells that no
    // model had seen: its result gives both counts and what was not tested.
    let run_once_on_two_further_cells = "cell_2170_model3";
    let mut generic = 0;
    for entry in cards {
        let (domain, card) = (entry["domain"].as_str().unwrap(), entry["card"].as_str().unwrap());
        let d = describe(&base, domain, card).await;
        let record = d["validation"].as_object().unwrap_or_else(|| panic!("{card} has no validation"));
        let mut keys: Vec<&str> = record.keys().map(String::as_str).collect();
        // The second form of a generator gives, after its five parts, the
        // card it stands beside.
        if record.contains_key("companion_to") {
            assert_eq!(keys.split_off(5), ["companion_to"], "{card}");
            assert_eq!(card, "generator_mmrtg_seebeck");
            assert_eq!(record["companion_to"], "generator_mmrtg_f1");
        }
        assert_eq!(keys, ["object", "data", "pass_line", "result", "limits"], "{card}");
        for key in &keys {
            assert!(!record[*key].as_str().unwrap().is_empty(), "{card} {key}");
        }
        if card == run_once_on_two_further_cells {
            let result = record["result"].as_str().unwrap();
            assert!(
                result.starts_with("Run once on runs of two further cells of the same type that no model had seen"),
                "{card}"
            );
            for words in [
                "Third cell, m448-N: 51 of 60 runs within every target",
                "Second cell, m1000: 31 of 45 runs within every target, under the 36 to 41 expected",
                "two of the five conditions hold",
                "All ten hill-climb cycles",
            ] {
                assert!(result.contains(words), "{card}: {words}");
            }
            assert!(record["limits"].as_str().unwrap().contains("have now been read"), "{card}");
            assert!(record["data"].as_str().unwrap().contains("Borealis 10.5683/SP3/ZVTR4B"), "{card}");
            assert!(record["limits"].as_str().unwrap().contains("The model was developed on all the tests of the first cell, m80."), "{card}");
            assert!(!record.values().any(|part| part.as_str().unwrap().contains("seal")), "{card}");
        } else if card == compared_and_missed {
            assert!(
                record["result"].as_str().is_some_and(|text| text.starts_with(
                    "On the 60 tests: 27 runs within every target, and no temperature with every run within."
                )),
                "{card}"
            );
            assert!(record["data"].as_str().unwrap().contains("Borealis 10.5683/SP3/ZVTR4B"), "{card}");
            assert!(!record.values().any(|part| part.as_str().unwrap().contains("seal")), "{card}");
        } else if card == awaiting {
            assert!(
                record["result"].as_str().is_some_and(|text| text.starts_with(
                    "On the 10 tests not in the fit. Peak power: RMS 0.58 of the report's standard deviation (target 1.5)"
                )),
                "{card}"
            );
        } else if card == "grid_gb" {
            // The Great Britain card was run once on its held-back trips: its
            // result gives the count, the dropped trip and each line, and its
            // limits say that it missed on the opened trips.
            let result = record["result"].as_str().unwrap();
            assert!(
                result.starts_with("On the 12 trips outside the fit, run once through the engine: every target held."),
                "{card}"
            );
            for words in ["dropped by the quality rule", "0.166", "0.182", "0.046 Hz", "No trip was refused"] {
                assert!(result.contains(words), "{card}: {words}");
            }
            let limits = record["limits"].as_str().unwrap();
            assert!(limits.contains("are an easier set than the 28 in it") && limits.contains("the depth error is 0.302 (target 0.25)"), "{card}");
            assert!(record["data"].as_str().unwrap().contains("NESO Open Data"), "{card}");
            assert!(!record.values().any(|part| part.as_str().unwrap().contains("seal")), "{card}");
        } else if card == "tire_eve_r19_ice" {
            // The tyre is fitted and was compared once on its runs outside the fit: its result gives the errors.
            let result = record["result"].as_str().unwrap();
            assert!(
                result.starts_with("On the 60 of the 90 runs outside the fit that hold a slip of 5 to 20 percent, as 60 cells of three repeats each"),
                "{card}"
            );
            assert!(result.contains("0.034 over the 60 cells (target 0.045)"), "{card}");
            assert!(result.contains("number 153"), "{card}");
            assert!(record["data"].as_str().unwrap().contains("doi 10.5281/zenodo.1009017"), "{card}");
        } else if fitted.contains(&card) {
            assert_ne!(record["result"], "not compared with a measurement", "{card}");
        } else if ["cable_stretch", "hydraulic_fluid", "resonance", "swing", "tesseract", "thermal", "ti64_lattice", "grid_node", "battery_sag"].contains(&card) {
            // The company specimens: each record names it so, and says what
            // its runs are held to.
            generic += 1;
            assert!(record["object"].as_str().unwrap().starts_with("Company specimen: "), "{card}");
            assert!(record["object"].as_str().unwrap().contains("every constant is stated in this description"), "{card}");
            let result = record["result"].as_str().unwrap();
            assert!(result.starts_with("An outside recomputation in arithmetic of 50 digits or more gives the card's runs"), "{card}");
            assert!(result.contains("conservation ledger to its stated limit"), "{card}");
        } else {
            generic += 1;
            assert_eq!(record["result"], "not compared with a measurement", "{card}");
        }
    }
    assert_eq!(generic, 9);
    let generator = describe(&base, "energy", "generator_mmrtg_f1").await;
    let result = generator["validation"]["result"].as_str().unwrap();
    assert!(result.contains("RMS error 0.008129, about 0.94 W (target 0.010)") && result.contains("+0.010094, about 1.16 W high (target 0.010 in size), 0.0000936, about 0.011 W, past the target"), "{result}");
    // No seal of a current run is quoted in a record.
    for card in fitted {
        let domain = cards.iter().find(|e| e["card"] == card).unwrap()["domain"].as_str().unwrap();
        let text = describe(&base, domain, card).await["validation"].to_string();
        assert!(!text.contains("seal"), "{card}");
    }
}

#[tokio::test]
async fn the_titanium_card_state_has_no_always_false_flag() {
    let base = spawn_app().await;
    let (status, text) = step(
        &base,
        "materials",
        "ti64_lattice",
        json!({
            "position_m": [0.0, 0.0, 0.0], "velocity_m_s": [0.0, 0.0, 0.0],
            "start_t_s": 0.0, "start_force_n": [0.0, 0.0, 0.0], "start_temperature_k": 293.15, "start_energy_anchor_j": 0.0, "start_boundary_work_j": 0.0,
            "force_n": [0.0, 0.0, 0.0], "torque_nm": [0.0, 0.0, 0.0],
            "temperature_k": 293.15, "dt_s": 1.0e-9
        }),
    )
    .await;
    assert_eq!(status, 200, "{text}");
    let body: Value = serde_json::from_str(&text).unwrap();
    let state = body["state"].as_object().unwrap();
    assert!(state.contains_key("von_mises_mpa"));
    assert!(!state.contains_key("yielded") && !state.contains_key("residual_breach"), "{text}");
    assert!(!text.contains("yielded\":") && !text.contains("residual_breach"), "{text}");
}

#[tokio::test]
async fn a_final_state_handed_back_to_the_satellite_continues_the_same_orbit_to_the_bit() {
    let base = spawn_app().await;
    let initial = json!({
        "epoch_utc_s": 840715200.0,
        "forecast_elapsed_s": 0.0,
        "position_m": [3767126.6844992605, 8956617.400906362, 7564717.856833777],
        "velocity_m_s": [4119.825668508274, 1676.7840658085788, -4053.2216929389388]
    });
    let run = |initial: Value, steps: u64| {
        json!({"domain": "aerospace", "card": "satellite_lageos1", "initial": initial, "dt_s": 10.0,
               "segments": [{"steps": steps, "load": {}}], "record_every": steps})
    };
    let (status, text) = post(&base, "/run_trajectory", &run(initial.clone(), 60)).await;
    assert_eq!(status, 200, "{text}");
    let whole: Value = serde_json::from_str(&text).unwrap();
    let (status, text) = post(&base, "/run_trajectory", &run(initial.clone(), 30)).await;
    assert_eq!(status, 200, "{text}");
    let half: Value = serde_json::from_str(&text).unwrap();
    // The state's epoch is the epoch of that state, not of the start.
    assert_eq!(half["final_state"]["t_s"].as_f64().unwrap(), 300.0);
    assert_eq!(half["final_state"]["epoch_utc_s"].as_f64().unwrap(), 840715500.0);
    assert_eq!(whole["final_state"]["epoch_utc_s"].as_f64().unwrap(), 840715800.0);
    // The time since the published state is handed back too.
    assert_eq!(half["final_state"]["forecast_elapsed_s"].as_f64().unwrap(), 300.0);
    let again = json!({
        "epoch_utc_s": half["final_state"]["epoch_utc_s"],
        "forecast_elapsed_s": half["final_state"]["forecast_elapsed_s"],
        "position_m": half["final_state"]["position_m"],
        "velocity_m_s": half["final_state"]["velocity_m_s"]
    });
    // Handed back, the forecast's time carries the run on: it names the
    // sealed half in from, and without from it is refused by name.
    let (status, text) = post(&base, "/run_trajectory", &run(again.clone(), 30)).await;
    assert_eq!(status, 422, "{text}");
    assert!(text.ends_with("(ContinuationWithoutUpstream)."), "{text}");
    let mut carried_on = run(again, 30);
    carried_on["from"] = json!({ "request": run(initial.clone(), 30), "run_seal": half["run_seal"] });
    let (status, text) = post(&base, "/run_trajectory", &carried_on).await;
    assert_eq!(status, 200, "{text}");
    let second: Value = serde_json::from_str(&text).unwrap();
    for name in ["position_m_hex", "velocity_m_s_hex", "epoch_utc_s_hex"] {
        assert_eq!(whole["final_state"][name], second["final_state"][name], "{name}");
    }
    // Sixty single steps, each state handed back as the next start, give the
    // same bits; each after the first names the sealed run of the steps before it.
    let mut current = initial.clone();
    let mut last = Value::Null;
    for before in 0..60_u64 {
        let mut parameters = current.as_object().unwrap().clone();
        parameters.insert("dt_s".to_string(), json!(10.0));
        let mut body = json!({ "domain": "aerospace", "card": "satellite_lageos1", "parameters": parameters });
        if before > 0 {
            let (_, up) = post(&base, "/run_trajectory", &run(initial.clone(), before)).await;
            let up: Value = serde_json::from_str(&up).unwrap();
            body["from"] = json!({ "request": run(initial.clone(), before), "run_seal": up["run_seal"] });
        }
        let (status, text) = post(&base, "/verify_physics", &body).await;
        assert_eq!(status, 200, "{text}");
        last = serde_json::from_str(&text).unwrap();
        current = json!({
            "epoch_utc_s": last["state"]["epoch_utc_s"],
            "forecast_elapsed_s": last["state"]["forecast_elapsed_s"],
            "position_m": last["state"]["position_m"],
            "velocity_m_s": last["state"]["velocity_m_s"]
        });
    }
    for name in ["position_m_hex", "velocity_m_s_hex", "epoch_utc_s_hex", "forecast_elapsed_s_hex"] {
        assert_eq!(whole["final_state"][name], last["state"][name], "{name}");
    }
    let start_state = describe(&base, "aerospace", "satellite_lageos1").await["start_state"]
        .as_str()
        .unwrap()
        .to_string();
    assert!(start_state.contains("the start epoch plus `t_s`"), "{start_state}");
}

/// The 18650PF cell is of one held current. A run whose segments carry the
/// tester's logged current of a 1C discharge, 2.899 to 2.89982 A, is taken;
/// a current 0.84 mA from the held current is refused whole, by name, before
/// the first step, with nothing sealed, and a rest is not a discharge.
#[tokio::test]
async fn a_cell_run_whose_current_changes_is_refused_whole_by_name() {
    let base = spawn_app().await;
    let run = |currents: Vec<f64>| {
        let segments: Vec<Value> =
            currents.iter().map(|current_a| json!({ "steps": 3, "load": { "current_a": current_a } })).collect();
        json!({
            "domain": "energy", "card": "cell_18650pf", "initial": { "soc": 1.0, "held_current_a": currents[0] },
            "dt_s": 10.0, "segments": segments, "record_every": 1
        })
    };
    let client = reqwest::Client::new();
    let ask = |body: Value| {
        let client = client.clone();
        let url = format!("{base}/run_trajectory");
        async move {
            let response = client.post(&url).json(&body).send().await.unwrap();
            (response.status().as_u16(), response.text().await.unwrap())
        }
    };
    for held in [vec![2.899, 2.89982, 2.899, 2.89941], vec![2.9], vec![2.9, 2.9006], vec![2.899, 2.89983]] {
        let (status, text) = ask(run(held.clone())).await;
        assert_eq!(status, 200, "{held:?}: {text}");
        let body: Value = serde_json::from_str(&text).unwrap();
        assert_eq!(body["outcome"], "completed", "{held:?}");
    }
    for changed in [vec![2.899, 2.89984], vec![2.9, 2.9, 2.89817], vec![2.89817, 2.90065]] {
        let (status, text) = ask(run(changed.clone())).await;
        assert_eq!(status, 422, "{changed:?}: {text}");
        assert!(text.contains("CurrentChangedDuringDischarge"), "{text}");
        assert!(text.contains("from the held current"), "{text}");
        assert!(!text.contains("run_seal"), "{text}");
    }
    for never in [vec![3.0, 0.0, 3.0], vec![0.0, 2.9], vec![2.9, 0.0], vec![0.0, 0.0]] {
        let (status, text) = ask(run(never.clone())).await;
        assert_eq!(status, 422, "{never:?}: {text}");
        assert!(text.contains("CurrentNotDischarging"), "{text}");
        assert!(!text.contains("run_seal"), "{text}");
    }
    let (status, text) = ask(run(vec![2.9, 2.9, 1.0])).await;
    assert_eq!(status, 422, "{text}");
    assert!(text.contains("CurrentChangedDuringDischarge"), "{text}");
    let (status, text) = ask(run(vec![3.0])).await;
    assert_eq!(status, 200, "{text}");
    let described = describe(&base, "energy", "cell_18650pf").await;
    let names: Vec<&str> =
        described["rejects"].as_array().unwrap().iter().map(|r| r["name"].as_str().unwrap()).collect();
    assert!(names.contains(&"CurrentChangedDuringDischarge"));
    let rule = described["dt_s"]["rule"].as_str().unwrap();
    assert!(rule.contains("every segment's current_a must lie within 0.00083 A (HELD_CURRENT_BAND_A) of the held current"), "{rule}");
    let limits = described["validation"]["limits"].as_str().unwrap();
    assert!(limits.contains("Compared at 1C on discharges from full charge, 2.899 to 2.89982 A;"), "{limits}");
}
