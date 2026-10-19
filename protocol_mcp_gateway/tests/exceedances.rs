//! What a flag reports beside its bit (0.15.0): the limit, the value, and
//! the extent past it in the limit's unit and as a fraction
//! (`src/exceedance.rs`, `src/solver_exceedance.rs`, REFERENCE.md "The extent
//! of a flag").
//!
//! One test for each kind of record: a limit reached at or above, a limit
//! below, other than, a window with two edges, a state with no number, a
//! limit that moves with the run, a flag of several limits; the extremes of
//! a run; `verify_tape` holding the records to their frame's bits; every
//! card's worked run read whole; every flag of every card described with
//! its limits; a solver's flag. The gateway's tests' build also holds every
//! landed step's records to its flags (`exceedance::debug_check`), and every
//! card's `step` does in the kernel's.

use std::time::Duration;

use reqwest::Client;
use serde_json::{Value, json};
use tokio::net::TcpListener;

async fn spawn_app() -> String {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    tokio::spawn(async move {
        axum::serve(listener, protocol_mcp_gateway::app()).await.unwrap();
    });
    tokio::task::yield_now().await;
    format!("http://127.0.0.1:{port}")
}

async fn post_text(base: &str, path: &str, body: &str) -> (u16, String) {
    let client = Client::new();
    let url = format!("{base}{path}");
    let mut last_error = None;
    for _ in 0..20 {
        match client
            .post(&url)
            .header(reqwest::header::CONTENT_TYPE, "application/json")
            .body(body.to_string())
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
    panic!("{path} never accepted a connection: {last_error:?}");
}

async fn post(base: &str, path: &str, body: &Value) -> Value {
    let (status, text) = post_text(base, path, &body.to_string()).await;
    assert_eq!(status, 200, "{path}: {text}");
    serde_json::from_str(&text).unwrap()
}

async fn get(base: &str, path: &str) -> Value {
    let text = Client::new().get(format!("{base}{path}")).send().await.unwrap().text().await.unwrap();
    serde_json::from_str(&text).unwrap()
}

fn hex_of(value: f64) -> String {
    format!("{:016x}", value.to_bits())
}

fn from_hex(value: &Value) -> Option<f64> {
    value.as_str().map(|text| f64::from_bits(u64::from_str_radix(text, 16).unwrap()))
}

/// A record says what it is, and its extent and fraction follow from its
/// limit, value and side as the rule writes them, bit for bit.
fn assert_record(record: &Value) {
    for key in ["flag", "quantity", "unit", "side"] {
        assert!(record[key].is_string(), "{key}: {record}");
    }
    let side = record["side"].as_str().unwrap();
    assert_ne!(side, "not_stated", "a bit with no record: {record}");
    let limit = from_hex(&record["limit_hex"]);
    let value = from_hex(&record["value_hex"]);
    let extent = from_hex(&record["extent_hex"]);
    let fraction = from_hex(&record["fraction_hex"]);
    if side == "state" {
        assert!(limit.is_none() && value.is_none() && extent.is_none() && fraction.is_none(), "{record}");
        return;
    }
    let (limit, value, extent) = (limit.unwrap(), value.unwrap(), extent.unwrap());
    for (key, number) in [("limit", limit), ("value", value), ("extent", extent)] {
        assert_eq!(record[key].as_f64().unwrap().to_bits(), number.to_bits(), "{key}: {record}");
    }
    let expected = match side {
        "above" | "at_or_above" => value - limit,
        "below" | "at_or_below" => limit - value,
        "other_than" => (value - limit).abs(),
        other => panic!("side {other}"),
    };
    assert_eq!(extent.to_bits(), expected.to_bits(), "{record}");
    assert!(extent >= 0.0, "a flagged value is past its limit: {record}");
    match fraction {
        Some(fraction) => assert_eq!(fraction.to_bits(), (extent / limit.abs()).to_bits(), "{record}"),
        None => assert_eq!(limit, 0.0, "no fraction only of a zero limit: {record}"),
    }
}

/// The flags word of a recorded frame, from its 64 bytes.
fn frame_flags(frame: &Value) -> u64 {
    let bytes = hex::decode(frame["frame_hex"].as_str().unwrap()).unwrap();
    u64::from_le_bytes(bytes[40..48].try_into().unwrap())
}

/// Every recorded frame's records stand for its flags word, and each holds.
fn assert_frames(run: &Value) -> u64 {
    let mut seen = 0;
    for frame in run["frames"].as_array().unwrap() {
        let flags = frame_flags(frame);
        match frame.get("exceedances") {
            None => assert_eq!(flags, 0, "a flagged frame with no records: {frame}"),
            Some(records) => {
                let records = records.as_array().unwrap();
                assert!(!records.is_empty());
                let mut bits = 0u64;
                for record in records {
                    assert_record(record);
                    bits |= 1 << record["bit"].as_u64().unwrap();
                }
                assert_eq!(bits, flags, "{frame}");
                seen |= bits;
            }
        }
    }
    for extreme in run.get("exceedance_extremes").and_then(Value::as_array).into_iter().flatten() {
        assert!(extreme["steps"].as_u64().unwrap() >= 1);
        assert!(extreme["first_step"].as_u64().unwrap() <= extreme["largest_at_step"].as_u64().unwrap_or(u64::MAX));
        if extreme["side"] != "state" {
            assert_record(extreme);
        }
    }
    seen
}

#[tokio::test]
async fn a_limit_at_or_above_reports_the_value_the_extent_and_the_fraction() {
    let base = spawn_app().await;
    let answer = post(
        &base,
        "/compute_step",
        &json!({"domain": "energy", "card": "thermal", "parameters": {"temperature_k": 360.0, "heat_power_w": 0.0, "ambient_k": 360.0, "dt_s": 23.625}}),
    )
    .await;
    let records = answer["exceedances"].as_array().expect("a flagged step carries exceedances");
    assert_eq!(records.len(), 1);
    let record = &records[0];
    assert_record(record);
    assert_eq!(record["flag"], "FLAG_THERMAL_DERATE");
    assert_eq!(record["bit"], 0);
    assert_eq!(record["quantity"], "temperature_k");
    assert_eq!(record["unit"], "K");
    assert_eq!(record["side"], "at_or_above");
    assert_eq!(record["limit_hex"], hex_of(358.15));
    assert_eq!(record["value_hex"], answer["state"]["temperature_k_hex"]);
    assert_eq!(answer["state"]["derated"], true);
    // The records stand beside the state; the frame seal is the frame's.
    let keys: Vec<&str> = answer.as_object().unwrap().keys().map(String::as_str).collect();
    assert_eq!(keys, ["inputs", "engine_version", "state", "exceedances", "residual_conservation_error", "sha384_seal", "seal_rule", "receipt_sha384"]);
}

#[tokio::test]
async fn a_step_with_no_flag_carries_no_record() {
    let base = spawn_app().await;
    let answer = post(
        &base,
        "/compute_step",
        &json!({"domain": "energy", "card": "thermal", "parameters": {"temperature_k": 293.15, "heat_power_w": 0.0, "ambient_k": 293.15, "dt_s": 23.625}}),
    )
    .await;
    assert!(answer.get("exceedances").is_none(), "{answer}");
    let run = post(
        &base,
        "/run_trajectory",
        &json!({"domain": "energy", "card": "thermal", "initial": {"temperature_k": 293.15}, "dt_s": 23.625, "segments": [{"steps": 4, "load": {"heat_power_w": 0.0, "ambient_k": 293.15}}], "record_every": 1}),
    )
    .await;
    assert!(run.get("exceedance_extremes").is_none());
    assert!(run["frames"].as_array().unwrap().iter().all(|frame| frame.get("exceedances").is_none()));
}

/// The GB twin served as grid_gb from 0.15.0: the worked example's volumes
/// and delivery times, and a step of 6.2 ms, under the twin's ceiling of
/// 1/150 s: 9,600 steps to 59.52 s at the first loss, then 300 at a loss
/// above the compared span, past the minute.
fn grid_gb_run(record_every: u64) -> Value {
    json!({
        "domain": "energy",
        "card": "grid_gb",
        "initial": {
            "frequency_hz": 49.942, "inertia_mw_s": 241000.0, "demand_mw": 27132.0,
            "dc_low_mw": 1145.0, "dm_low_mw": 500.0, "dr_low_mw": 480.0,
            "dc_high_mw": 1080.0, "dm_high_mw": 510.0, "dr_high_mw": 480.0,
            "fast_delivery_s": 1.0, "regulation_delivery_s": 10.0
        },
        "dt_s": 0.0062,
        "segments": [{"steps": 9600, "load": {"loss_mw": 996.0}}, {"steps": 300, "load": {"loss_mw": 1500.0}}],
        "record_every": record_every,
    })
}

#[tokio::test]
async fn a_flag_of_several_limits_has_one_record_for_each_limit_passed_and_a_run_its_extremes() {
    let base = spawn_app().await;
    let run = post(&base, "/run_trajectory", &grid_gb_run(4000)).await;
    assert_eq!(run["outcome"], "completed");
    assert_eq!(assert_frames(&run), 0b11);
    // The last frame, step 9,900: a loss above the compared span, a loss
    // other than the run's first, both under bit 0, and past the minute, bit 1.
    let last = run["frames"].as_array().unwrap().last().unwrap();
    assert_eq!(last["step"], 9900);
    let records = last["exceedances"].as_array().unwrap();
    let described: Vec<(String, String, u64)> = records
        .iter()
        .map(|record| (record["quantity"].as_str().unwrap().to_string(), record["side"].as_str().unwrap().to_string(), record["bit"].as_u64().unwrap()))
        .collect();
    assert_eq!(
        described,
        [("loss_mw".to_string(), "above".to_string(), 0), ("loss_mw".to_string(), "other_than".to_string(), 0), ("t_s".to_string(), "above".to_string(), 1)]
    );
    assert_eq!(records[0]["limit_hex"], hex_of(1402.0));
    assert_eq!(records[0]["extent_hex"], hex_of(98.0));
    assert_eq!(records[1]["limit_hex"], hex_of(996.0), "the run's first loss, a limit that moves with the run");
    assert_eq!(records[1]["extent_hex"], hex_of(504.0));
    assert_eq!(records[2]["limit_hex"], hex_of(60.0));
    // The extremes count every landed step, the ones not recorded too.
    let extremes = run["exceedance_extremes"].as_array().unwrap();
    assert_eq!(extremes.len(), 3);
    assert_eq!((extremes[0]["steps"].as_u64(), extremes[0]["first_step"].as_u64()), (Some(300), Some(9601)));
    let minute = &extremes[2];
    assert_eq!(minute["quantity"], "t_s");
    assert_eq!(minute["largest_at_step"], 9900);
    assert_eq!(minute["value_hex"], last["exceedances"][2]["value_hex"]);
    let first_late = minute["first_step"].as_u64().unwrap();
    assert_eq!(minute["steps"].as_u64().unwrap(), 9900 - first_late + 1);
    // Recorded every step, the same extremes, and each flagged frame its records.
    let every = post(&base, "/run_trajectory", &grid_gb_run(1)).await;
    assert_eq!(every["exceedance_extremes"], run["exceedance_extremes"]);
    assert_frames(&every);
    let frames = every["frames"].as_array().unwrap();
    assert!(frames[9599].get("exceedances").is_none());
    assert_eq!(frames[9600]["exceedances"].as_array().unwrap().len(), 2);
    assert_eq!(frames[(first_late - 1) as usize]["exceedances"].as_array().unwrap().len(), 3);
    // The run seal does not cover the records: a run sealed as before.
    assert_eq!(every["run_seal"], post(&base, "/run_trajectory", &grid_gb_run(1)).await["run_seal"]);
}

#[tokio::test]
async fn verify_tape_holds_each_frames_records_to_its_bits() {
    let base = spawn_app().await;
    let (status, text) = post_text(&base, "/run_trajectory", &grid_gb_run(4000).to_string()).await;
    assert_eq!(status, 200);
    let verified: Value = serde_json::from_str(&post_text(&base, "/verify_tape", &text).await.1).unwrap();
    assert_eq!(verified["holds"], true, "{verified}");
    let run: Value = serde_json::from_str(&text).unwrap();
    // A whole answer whose frame's records do not name its bits does not
    // hold, by the check of its seals, before anything is computed again.
    let with_frames = |frames: Value| {
        let mut answer = run.clone();
        answer["frames"] = frames;
        answer
    };
    let mut dropped = run["frames"].clone();
    dropped[2]["exceedances"].as_array_mut().unwrap().pop();
    let report = post(&base, "/verify_tape", &with_frames(dropped)).await;
    assert_eq!((report["holds"].as_bool(), report["cause"].as_str(), report["broken_link"].as_u64()), (Some(false), Some("exceedances"), Some(2)), "{report}");
    let mut moved = run["frames"].clone();
    moved[2]["exceedances"][0]["bit"] = json!(5);
    let report = post(&base, "/verify_tape", &with_frames(moved)).await;
    assert_eq!(report["cause"], "exceedances");
    let mut added = run["frames"].clone();
    added[0]["exceedances"] = run["frames"][2]["exceedances"].clone();
    let report = post(&base, "/verify_tape", &with_frames(added)).await;
    assert_eq!((report["cause"].as_str(), report["broken_link"].as_u64()), (Some("exceedances"), Some(0)));
    // An answer with its records taken out passes that check, and the long
    // form names the records the engine writes.
    let mut none = run["frames"].clone();
    for frame in none.as_array_mut().unwrap() {
        frame.as_object_mut().unwrap().remove("exceedances");
    }
    let report = post(&base, "/verify_tape", &with_frames(none)).await;
    assert_eq!(report["cause"], "answer_text");
    assert_eq!(report["differs_at"], "frames[2].exceedances", "{report}");
    // A value changed is not seen by the short form, as a decimal of the
    // state is not; the long form computes it again.
    let mut changed: Value = serde_json::from_str(&text).unwrap();
    changed["frames"][2]["exceedances"][0]["value_hex"] = json!(hex_of(1600.0));
    let report = post(&base, "/verify_tape", &changed).await;
    assert_eq!(report["holds"], false);
    assert_eq!(report["cause"], "answer_text");
}

#[tokio::test]
async fn a_limit_below_a_window_and_a_limit_that_moves_with_the_run() {
    let base = spawn_app().await;
    // A window: Stardust's drogue Mach window, a record for each edge.
    let stardust = post(
        &base,
        "/run_trajectory",
        &json!({"domain": "aerospace", "card": "capsule_stardust", "initial": {"radius_m": 6503140.0, "radial_speed_m_s": -1842.1416729435698, "transverse_speed_m_s": 12767.792058801891}, "dt_s": 0.05, "segments": [{"steps": 6000, "load": {}}], "record_every": 1}),
    )
    .await;
    assert_frames(&stardust);
    let window: Vec<&Value> = stardust["frames"].as_array().unwrap().iter().filter_map(|frame| frame.get("exceedances")).collect();
    assert!(!window.is_empty(), "the run reaches the drogue window: {}", stardust["outcome"]);
    for records in &window {
        let sides: Vec<&str> = records.as_array().unwrap().iter().map(|record| record["side"].as_str().unwrap()).collect();
        assert_eq!(sides, ["at_or_above", "at_or_below"]);
        assert_eq!(records[0]["limit_hex"], hex_of(1.2));
        assert_eq!(records[1]["limit_hex"], hex_of(1.6));
    }
    // A limit that moves with the run: the rod of the SPERT III E-core
    // moves from its delay to the time it is out, which the inserted
    // reactivity sets; while it moves, the time is above the one and below
    // the other. The E-core twin served as reactor_spert3, on its worked start.
    let ecore = post(
        &base,
        "/run_trajectory",
        &json!({"domain": "energy", "card": "reactor_spert3", "initial": {"inlet_temperature_k": 297.5944444444445, "system_pressure_mpa": 0.0853, "coolant_flow_m_per_s": 0.0, "initial_power_mw": 7.9949547e-6}, "dt_s": 0.00043000000000000004, "segments": [{"steps": 400, "load": {"reactivity_inserted_dollar": 1.0}}], "record_every": 1}),
    )
    .await;
    assert_frames(&ecore);
    let first = ecore["frames"].as_array().unwrap().iter()
        .filter_map(|frame| frame.get("exceedances"))
        .find(|records| records.as_array().unwrap().iter().any(|record| record["flag"] == "FLAG_ROD_MOVING"))
        .expect("the rod moves");
    let rod: Vec<&Value> = first.as_array().unwrap().iter().filter(|record| record["flag"] == "FLAG_ROD_MOVING").collect();
    let sides: Vec<&str> = rod.iter().map(|record| record["side"].as_str().unwrap()).collect();
    assert_eq!(sides, ["above", "below"]);
    assert!(rod.iter().all(|record| record["quantity"] == "t_s"));
    assert_eq!(rod[0]["limit_hex"], hex_of(0.0010369315));
    assert_eq!(rod[1]["limit_hex"], "3fb1bbe303ed3cf4", "the time the rod is out at 1 dollar");
    // A state with no number: a frame of the fine guide with no centroid.
    let mut guide = post(&base, "/describe_card", &json!({"domain": "aerospace", "card": "los_jwst_fine_guide"})).await["example_run_trajectory"].clone();
    guide["segments"] = json!([{"steps": 2, "load": {"measured": 0.0, "centroid_x_arcsec": 0.0, "centroid_y_arcsec": 0.0, "counts_per_s": 0.0}}]);
    let unmeasured = post(&base, "/run_trajectory", &guide).await;
    assert_eq!(unmeasured["outcome"], "completed", "{unmeasured}");
    assert_frames(&unmeasured);
    let records = unmeasured["frames"][0]["exceedances"].as_array().unwrap();
    let state = records.iter().find(|record| record["flag"] == "FLAG_NO_MEASUREMENT").unwrap();
    assert_eq!(state["side"], "state");
    assert!(state["limit"].is_null() && state["extent"].is_null() && state["fraction"].is_null());
    let extremes = unmeasured["exceedance_extremes"].as_array().unwrap();
    let no_measurement = extremes.iter().find(|extreme| extreme["flag"] == "FLAG_NO_MEASUREMENT").unwrap();
    assert_eq!((no_measurement["steps"].as_u64(), no_measurement["largest_at_step"].is_null()), (Some(2), true));
}

/// Every card's worked run, each flagged frame read whole: the records stand
/// for the frame's bits and each holds by the rule.
#[tokio::test]
async fn every_cards_worked_run_carries_its_records() {
    let base = spawn_app().await;
    let cards = get(&base, "/cards").await;
    let mut flagged = Vec::new();
    for card in cards["cards"].as_array().unwrap() {
        let body = json!({"domain": card["domain"], "card": card["card"]});
        let described = post(&base, "/describe_card", &body).await;
        let run = post(&base, "/run_trajectory", &described["example_run_trajectory"]).await;
        if assert_frames(&run) != 0 {
            flagged.push(card["card"].as_str().unwrap().to_string());
        }
    }
    assert!(flagged.len() >= 5, "{flagged:?}");
}

/// Every flag of every card is described with the limits it marks, and a
/// card that sets flags states the rule.
#[tokio::test]
async fn every_flag_of_every_card_is_described_with_its_limits() {
    let base = spawn_app().await;
    let cards = get(&base, "/cards").await;
    let mut flags = 0;
    let mut limits = 0;
    for card in cards["cards"].as_array().unwrap() {
        let body = json!({"domain": card["domain"], "card": card["card"]});
        let described = post(&base, "/describe_card", &body).await;
        let advisory = described["advisory_flags"].as_array().unwrap();
        assert_eq!(described.get("exceedances").is_some(), !advisory.is_empty(), "{}", card["card"]);
        for flag in advisory {
            flags += 1;
            let listed = flag["limits"].as_array().unwrap();
            assert!(!listed.is_empty(), "{} {}", card["card"], flag["name"]);
            for limit in listed {
                limits += 1;
                assert!(limit["quantity"].as_str().is_some_and(|text| !text.is_empty()));
                assert!(limit["limit_words"].as_str().is_some_and(|text| !text.is_empty()));
                let side = limit["side"].as_str().unwrap();
                assert!(["above", "at_or_above", "below", "at_or_below", "other_than", "state"].contains(&side));
                assert_eq!(limit["limit"].is_null(), limit["limit_hex"].is_null());
                if side == "state" {
                    assert!(limit["limit"].is_null());
                }
            }
        }
    }
    // 70 from 0.15.0: the retired models' flags are not served.
    assert_eq!(flags, 70);
    assert!(limits > flags, "{limits}");
}

#[tokio::test]
async fn a_solver_flag_carries_its_extent_beside_the_result() {
    let base = spawn_app().await;
    let described = post(&base, "/describe_solver", &json!({"solver": "stress_plate_hole_s235_elastic"})).await;
    assert!(described["returns"]["exceedances"].is_string());
    assert_eq!(described["returns"]["exceedance_limits"][0]["flag"], "past_compared_traction");
    let mut request = described["example_solve"].clone();
    request["parameters"]["traction_mpa"] = json!(120.0);
    let past = post(&base, "/solve", &request).await;
    assert_eq!(past["outcome"], "converged", "{past}");
    assert_eq!(past["result"]["past_compared_traction"], 1.0);
    let records = past["exceedances"].as_array().expect("a flag at 1 carries exceedances");
    assert_eq!(records.len(), 1);
    assert!(records[0].get("bit").is_none(), "a solver has no flags word");
    assert_record(&records[0]);
    assert_eq!(records[0]["value_hex"], hex_of(120.0));
    let keys: Vec<&str> = past.as_object().unwrap().keys().map(String::as_str).collect();
    let at = keys.iter().position(|key| *key == "exceedances").unwrap();
    assert_eq!(keys[at - 1], "result");
    // The worked example, at the compared traction, carries none.
    let inside = post(&base, "/solve", &described["example_solve"]).await;
    assert!(inside.get("exceedances").is_none());
    let solvers = get(&base, "/solvers").await;
    let mut with = Vec::new();
    for solver in solvers["solvers"].as_array().unwrap() {
        let name = solver["solver"].as_str().or_else(|| solver["name"].as_str()).unwrap();
        let described = post(&base, "/describe_solver", &json!({"solver": name})).await;
        // A description in parts gives its returns as a part of its own.
        let returns = match described.get("returns") {
            Some(returns) => returns.clone(),
            None => post(&base, "/describe_solver", &json!({"solver": name, "part": "returns"})).await["returns"].clone(),
        };
        if returns.get("exceedance_limits").is_some() {
            with.push(name.to_string());
        }
    }
    with.sort();
    assert_eq!(with, ["ablator_slab", "culture_trichoderma_atp", "fuel_rod_triga_jsi", "stress_plate_hole_s235_elastic"]);
}
