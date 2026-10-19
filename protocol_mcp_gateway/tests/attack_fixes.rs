//! Faults of the candidate of version 0.2.1, each held here: the battery's
//! charging limit and lowest state of charge, one rule for a value outside a
//! published bound, and refusals that name what they refuse.

use std::time::Duration;

use protocol_mcp_gateway::{app, message_number};
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
    for _ in 0..20 {
        match client
            .post(format!("{base}{path}"))
            .header(reqwest::header::CONTENT_TYPE, "application/json")
            .body(body.to_string())
            .send()
            .await
        {
            Ok(response) => {
                let status = response.status().as_u16();
                return (status, response.text().await.unwrap());
            }
            Err(_) => tokio::time::sleep(Duration::from_millis(25)).await,
        }
    }
    panic!("{path} never accepted a connection");
}

async fn describe(base: &str, domain: &str, card: &str) -> Value {
    let (status, text) = post(base, "/describe_card", &json!({ "domain": domain, "card": card })).await;
    assert_eq!(status, 200, "{text}");
    serde_json::from_str(&text).unwrap()
}

fn step(card: &str, domain: &str, parameters: Value) -> Value {
    json!({ "domain": domain, "card": card, "parameters": parameters })
}

fn next_larger(x: f64) -> f64 {
    x.next_up()
}

fn next_smaller(x: f64) -> f64 {
    x.next_down()
}

#[tokio::test]
async fn the_battery_charging_minimum_says_it_depends_on_the_state_of_charge() {
    let base = spawn_app().await;
    let card = describe(&base, "energy", "battery_sag").await;
    let field = &card["load_fields"][0];
    assert_eq!(field["name"], "current_a");
    let bound = field["bound"].as_str().unwrap();
    // The dependency is the first thing the field says, not the end of a sentence.
    assert!(
        bound.starts_with("The largest charging current (negative) that is accepted depends on the state of charge"),
        "{bound}"
    );
    assert!(bound.contains("approached as the state of charge goes to zero and nowhere else"), "{bound}");
    assert!(bound.contains("charging_current_limit_by_state_of_charge"), "{bound}");
}

#[tokio::test]
async fn each_published_charging_current_is_accepted_and_the_next_one_beyond_it_is_refused() {
    let base = spawn_app().await;
    let card = describe(&base, "energy", "battery_sag").await;
    let table = &card["charging_current_limit_by_state_of_charge"];
    let dt_s = table["dt_s"].as_f64().unwrap();
    let rows = table["rows"].as_array().unwrap();
    assert!(rows.len() >= 8, "{table}");
    let mut last_size = f64::INFINITY;
    for row in rows {
        let soc = row["soc"].as_f64().unwrap();
        let current_a = row["current_a"].as_f64().unwrap();
        let size = row["largest_charging_current_size_a"].as_f64().unwrap();
        assert_eq!(current_a, -size);
        // A fuller pack takes a smaller charge, and none takes the published minimum.
        assert!(size < last_size, "{row}");
        assert!(size < 120.0, "{row}");
        last_size = size;
        let (status, text) = post(
            &base,
            "/verify_physics",
            &step("battery_sag", "energy", json!({ "soc": soc, "current_a": current_a, "dt_s": dt_s })),
        )
        .await;
        assert_eq!(status, 200, "soc {soc} current {current_a}: {text}");
        let beyond = -next_larger(size);
        let (status, text) = post(
            &base,
            "/verify_physics",
            &step("battery_sag", "energy", json!({ "soc": soc, "current_a": beyond, "dt_s": dt_s })),
        )
        .await;
        assert_eq!(status, 422, "soc {soc} current {beyond}: {text}");
        assert!(text.contains("OvervoltageCutoff"), "soc {soc} current {beyond}: {text}");
        assert!(!text.contains("sha384_seal"), "{text}");
    }
    // The published minimum itself is the limit only as the state of charge goes to zero.
    let minimum = card["load_fields"][0]["minimum"].as_f64().unwrap();
    assert!(minimum < -119.9 && -last_size > minimum);
}

#[tokio::test]
async fn the_battery_publishes_its_lowest_state_of_charge_and_names_it_when_a_value_is_below() {
    let base = spawn_app().await;
    let card = describe(&base, "energy", "battery_sag").await;
    let soc = &card["phase_fields"][0];
    let floor = soc["minimum"].as_f64().unwrap();
    assert_eq!(soc["minimum_exclusive"], false);
    assert!(floor > 0.0 && floor < 1.0e-15, "{floor}");
    let bound = soc["bound"].as_str().unwrap();
    assert!(bound.contains("lowest state of charge"), "{bound}");
    assert!(bound.contains("UndervoltageCutoff"), "{bound}");

    let at = |value: f64| step("battery_sag", "energy", json!({ "soc": value, "current_a": 0.0, "dt_s": 0.1 }));
    let (status, text) = post(&base, "/verify_physics", &at(floor)).await;
    assert_eq!(status, 200, "{text}");
    for below in [next_smaller(floor), 1.0e-20, 0.0] {
        let (status, text) = post(&base, "/verify_physics", &at(below)).await;
        assert_eq!(status, 422, "{below}: {text}");
        assert!(text.contains("'soc'"), "{text}");
        assert!(text.contains(&message_number(below)), "{text}");
        assert!(text.contains(&message_number(floor)), "{text}");
        assert!(text.contains("UndervoltageCutoff"), "{text}");
        assert!(!text.contains("phase is not valid"), "{text}");
    }
    // Just above the lowest state of charge a discharging step cannot land, and the refusal is named.
    let (status, text) = post(
        &base,
        "/verify_physics",
        &step("battery_sag", "energy", json!({ "soc": 1.0e-4, "current_a": 1.0, "dt_s": 0.1 })),
    )
    .await;
    assert_eq!(status, 422, "{text}");
    assert!(text.contains("UndervoltageCutoff"), "{text}");
}

/// Every single-number field with a published bound, with its value one step
/// outside the bound. The end of the generator's and the satellite's time axes
/// is the end of the card, not the published last start, and is left out here.
fn bounded_fields(card: &Value) -> Vec<(String, bool, &'static str, f64)> {
    let mut out = Vec::new();
    for (list, is_load) in [("phase_fields", false), ("load_fields", true)] {
        for field in card[list].as_array().unwrap() {
            if field["components"] != 1 {
                continue;
            }
            let name = field["name"].as_str().unwrap().to_string();
            if let Some(minimum) = field["minimum"].as_f64() {
                let value = if field["minimum_exclusive"] == true { minimum } else { next_smaller(minimum) };
                if value.is_normal() {
                    out.push((name.clone(), is_load, "minimum", value));
                }
            }
            if let Some(maximum) = field["maximum"].as_f64() {
                let value = if field["maximum_exclusive"] == true { maximum } else { next_larger(maximum) };
                out.push((name, is_load, "maximum", value));
            }
        }
    }
    out
}

#[tokio::test]
async fn a_value_outside_a_published_bound_is_422_before_a_permit_on_every_card_in_every_segment() {
    let base = spawn_app().await;
    let (_, list) = post(&base, "/describe_card", &json!({ "domain": "energy", "card": "thermal" })).await;
    let _ = list;
    let cards = [
        ("materials", "ti64_lattice"),
        ("energy", "grid_node"),
        ("energy", "swing"),
        ("energy", "battery_sag"),
        ("energy", "cell_18650pf"),
        ("energy", "thermal"),
        ("signal", "resonance"),
        ("materials", "hydraulic_fluid"),
        ("energy", "reactor_spert3"),
        ("signal", "cantilever_eth"),
        ("energy", "cell_2170_model3"),
        ("materials", "creep_cmsx6"),
        ("aerospace", "capsule_insight"),
        ("energy", "reactor_triga_jsi"),
        ("aerospace", "crater_ryugu_sci"),
        ("signal", "link_gothenburg_10010"),
        ("energy", "grid_gb"),
        // Every field of this card is a 3-vector with no published bound, so
        // it has no single number to put outside one and nothing is posted.
        ("aerospace", "solar_system"),
        ("materials", "tire_eve_r19_ice"),
        ("energy", "accumulator_sbo450"),
        ("aerospace", "los_jwst_fine_guide"),
        ("energy", "tokamak_jet_dte1"),
        ("materials", "fingerpad_p02_glass"),
        ("energy", "generator_mmrtg_f1"),
        ("aerospace", "satellite_lageos1"),
    ];
    let mut checked = 0;
    for (domain, name) in cards {
        let card = describe(&base, domain, name).await;
        for (field, is_load, side, value) in bounded_fields(&card) {
            let exempt = matches!((name, field.as_str(), side), ("generator_mmrtg_f1", "mission_time_s", "maximum") | ("satellite_lageos1", "epoch_utc_s", "maximum"));
            if exempt {
                continue;
            }
            // The step.
            let mut example = card["example_compute_step"].clone();
            example["parameters"][field.as_str()] = json!(value);
            let (status, text) = post(&base, "/verify_physics", &example).await;
            assert_eq!(status, 422, "{name} {field} {side} step: {text}");
            assert!(text.contains(&format!("'{field}'")), "{name} {field}: {text}");
            assert!(!text.contains("sha384_seal"), "{text}");
            // The run: the value at the start, in the first segment, and in a later one.
            let positions: &[usize] = if is_load { &[0, 1] } else { &[usize::MAX] };
            for &position in positions {
                let mut run = card["example_run_trajectory"].clone();
                let segment = run["segments"][0].clone();
                run["segments"] = json!([segment.clone(), segment]);
                if is_load {
                    run["segments"][position]["load"][field.as_str()] = json!(value);
                } else {
                    run["initial"][field.as_str()] = json!(value);
                }
                let (status, text) = post(&base, "/run_trajectory", &run).await;
                assert_eq!(status, 422, "{name} {field} {side} run position {position}: {text}");
                assert!(text.contains(&format!("'{field}'")), "{name} {field}: {text}");
                assert!(!text.contains("run_seal"), "{text}");
                checked += 1;
            }
        }
    }
    assert!(checked >= 30, "only {checked} cases were checked");
}

#[tokio::test]
async fn a_state_that_leaves_the_card_during_a_run_is_still_a_sealed_refusal() {
    let base = spawn_app().await;
    let card = describe(&base, "energy", "generator_mmrtg_f1").await;
    let mut run = card["example_run_trajectory"].clone();
    // The last day of the record is not the end of the generator: the step
    // from it lands, sealed.
    run["initial"]["mission_time_s"] = json!(353_116_800.0);
    run["segments"] = json!([{ "steps": 1, "load": {} }]);
    run["dt_s"] = json!(86_400.0);
    let (status, text) = post(&base, "/run_trajectory", &run).await;
    assert_eq!(status, 200, "{text}");
    let body: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(body["outcome"], "completed", "{body}");
    assert!(body["run_seal"].is_string());
    // A state that leaves the card during a run is a sealed refusal: a cell
    // held at 2.9 A runs down to its 2.5 V cutoff.
    let cell = describe(&base, "energy", "cell_18650pf").await;
    let mut run = cell["example_run_trajectory"].clone();
    run["segments"] = json!([{ "steps": 2_000, "load": { "current_a": 2.9 } }]);
    run["dt_s"] = json!(10.0);
    run["record_every"] = json!(2_000);
    let (status, text) = post(&base, "/run_trajectory", &run).await;
    assert_eq!(status, 200, "{text}");
    let body: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(body["outcome"], "refused", "{body}");
    assert_eq!(body["reject"], "UndervoltageCutoff", "{body}");
    assert!(body["run_seal"].is_string());
}

#[tokio::test]
async fn refusals_name_the_field_the_value_and_the_limit() {
    let base = spawn_app().await;
    let thermal = |extra: Value, temperature: Value| {
        let mut parameters = json!({ "temperature_k": temperature, "heat_power_w": 1.0, "ambient_k": 293.15, "dt_s": 0.1 });
        for (key, value) in extra.as_object().unwrap() {
            parameters[key] = value.clone();
        }
        step("thermal", "energy", parameters)
    };
    // A value of the wrong kind is not called missing.
    for (written, kind) in [(json!("293.15"), "a string"), (json!(null), "null"), (json!([293.15]), "a list"), (json!(true), "a boolean")] {
        let (status, text) = post(&base, "/verify_physics", &thermal(json!({}), written)).await;
        assert_eq!(status, 422, "{text}");
        assert_eq!(text, format!("Physical Violation: 'temperature_k' must be a number; this request has {kind}."));
    }
    let (_, text) = post(
        &base,
        "/verify_physics",
        &step("thermal", "energy", json!({ "heat_power_w": 1.0, "ambient_k": 293.15, "dt_s": 0.1 })),
    )
    .await;
    assert_eq!(text, "Missing required parameter: 'temperature_k' (InputMissing).");
    // An unknown key is called unknown, whatever its name or value.
    for key in ["attempt", "mass_flow", "temp_extra"] {
        let (status, text) = post(&base, "/verify_physics", &thermal(json!({ key: -1.0 }), json!(293.15))).await;
        assert_eq!(status, 422, "{text}");
        assert_eq!(text, format!("Physical Violation: '{key}' is not a field of this request."));
    }
    // A run names an unknown key in a load, a segment or the initial state before it reads any sign.
    let run = json!({"domain":"energy","card":"thermal","initial":{"temperature_k":293.15,"mass_extra":-1.0},"dt_s":0.1,
        "segments":[{"steps":1,"load":{"heat_power_w":0.0,"ambient_k":293.15}}],"record_every":1});
    let (_, text) = post(&base, "/run_trajectory", &run).await;
    assert_eq!(text, "Physical Violation: 'mass_extra' is not a field of this request.");
    // A pair that is not a string names which one.
    let (status, text) = post(&base, "/verify_physics", &json!({"domain":["thermal"],"card":"thermal","parameters":{}})).await;
    assert_eq!(status, 422);
    assert_eq!(text, "Physical Violation: 'domain' must be a string; this request has a list.");
    let (_, text) = post(&base, "/verify_physics", &json!({"card":"thermal","parameters":{}})).await;
    assert_eq!(text, "Missing required parameter: 'domain' (InputMissing).");
    // A step at the smallest normal number is a short sentence with the limit in it.
    let (status, text) = post(
        &base,
        "/verify_physics",
        &step("thermal", "energy", json!({ "temperature_k": 293.15, "heat_power_w": 1.0, "ambient_k": 293.15, "dt_s": 2.2250738585072014e-308 })),
    )
    .await;
    assert_eq!(status, 422);
    assert!(text.len() < 200, "{text}");
    assert!(text.contains("2.2250738585072014e-308") && text.contains("step floor of the thermal card"), "{text}");
    // A solver number that is negative is outside the range, not "not an integer".
    let (_, text) = post(&base, "/solve", &json!({"solver":"atom_dft","parameters":{"atomic_number":-1}})).await;
    // Re-pinned for 0.10.1: an atomic number outside the range carries the name AtomicNumberOutOfRange, and the entry writes the range in words.
    assert!(text.contains("atomic number -1 is outside 1 to 18 (AtomicNumberOutOfRange)."), "{text}");
    // A grid start below the floor names the floor, not "phase is not valid".
    let (_, text) = post(
        &base,
        "/verify_physics",
        &step("grid_node", "energy", json!({"current_a":0.0,"voltage_v":0.0,"temperature_k":200.0,"upstream_voltage_v":0.0,"load_current_a":0.0,"ambient_k":293.15,"dt_s":1e-6})),
    )
    .await;
    assert!(text.contains("'temperature_k' 200 K is below 233.15 K") && text.contains("grid_node"), "{text}");
    assert!(!text.contains("phase is not valid"), "{text}");
    // A grid start current far above anything the link can carry names the field, the value and the limit.
    // In version 0.2.1 this current published no bound, the request reached the card, and the answer was
    // the card's NonFiniteState with its meaning. The current now publishes a bound from the melting point.
    let (status, text) = post(
        &base,
        "/verify_physics",
        &step("grid_node", "energy", json!({"current_a":1e30,"voltage_v":0.0,"temperature_k":293.15,"upstream_voltage_v":0.0,"load_current_a":0.0,"ambient_k":293.15,"dt_s":1e-6})),
    )
    .await;
    assert_eq!(status, 422, "{text}");
    assert!(text.contains("'current_a' 1e30 A is above") && text.contains("the inclusive maximum of the grid_node card"), "{text}");
    assert!(!text.contains("NonFiniteState"), "{text}");
}
