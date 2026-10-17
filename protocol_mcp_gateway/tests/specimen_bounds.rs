//! The inputs that published no bound in version 0.2.1, one by one.
//!
//! A request could reach a conservation refusal in one step on the titanium
//! lattice, the fuse link and the drive-mode resonator, through inputs that
//! published no bound. Each such input now does one of two things, and this
//! file holds each to it:
//!
//! - it publishes a bound taken from the specimen (its yield, its melting
//!   point, its amplitude limit), and the door turns a value outside it away by
//!   name before the card sees it; or
//! - its description states its reach, as the spring mount and the capsule do,
//!   and the facts in that statement are run here.
//!
//! Seven requests that reached such a refusal on version 0.2.1 are sent here.

use std::time::Duration;

use genesis_kernel::frame::LastStateFrame64;
use genesis_kernel::{grid_node, tesseract, ti64_lattice};
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

fn field<'a>(card: &'a Value, name: &str) -> &'a Value {
    for list in ["phase_fields", "load_fields"] {
        for field in card[list].as_array().unwrap() {
            if field["name"] == name {
                return field;
            }
        }
    }
    panic!("no field {name}");
}

fn largest(card: &Value, name: &str) -> f64 {
    let field = field(card, name);
    let maximum = field["maximum"].as_f64().unwrap_or_else(|| panic!("{name} publishes no maximum"));
    assert_eq!(field["minimum"].as_f64(), Some(-maximum), "{name}: the range is not symmetric");
    assert_eq!(field["minimum_exclusive"], false, "{name}");
    assert_eq!(field["maximum_exclusive"], false, "{name}");
    maximum
}

/// A run of `steps` steps on one card, from `initial`, under one load.
fn run_of(domain: &str, card: &str, initial: Value, load: Value, dt_s: f64, steps: u64) -> Value {
    json!({
        "domain": domain,
        "card": card,
        "initial": initial,
        "dt_s": dt_s,
        "segments": [{ "steps": steps, "load": load }],
        "record_every": steps
    })
}

async fn run(base: &str, request: &Value) -> Value {
    let (status, text) = post(base, "/run_trajectory", request).await;
    assert_eq!(status, 200, "{text}");
    serde_json::from_str(&text).unwrap()
}

/// The request is turned away at the door: 422, nothing sealed, and a sentence
/// that names the field, the value's side of the bound, the bound and the card.
async fn turned_away(base: &str, request: &Value, label: &str, bound: f64, unit: &str, card: &str) -> String {
    let (status, text) = post(base, "/run_trajectory", request).await;
    assert_eq!(status, 422, "{label}: {text}");
    assert!(!text.contains("run_seal"), "{text}");
    assert!(text.starts_with(&format!("Physical Failure: '{label}' ")), "{text}");
    let wanted = format!("{} {unit}, the inclusive ", message_number(bound));
    assert!(text.contains(&wanted), "{label}: {text} lacks {wanted}");
    assert!(text.contains(&format!(" of the {card} card")), "{text}");
    assert!(text.ends_with("That state is not sealed."), "{text}");
    text
}

const TI64_REST_LOAD: fn() -> Value =
    || json!({ "force_n": [0.0, 0.0, 0.0], "torque_nm": [0.0, 0.0, 0.0], "temperature_k": 293.15 });
const GRID_REST_LOAD: fn() -> Value =
    || json!({ "upstream_voltage_v": 0.0, "load_current_a": 0.0, "ambient_k": 293.15 });
const TESSERACT_REST_LOAD: fn() -> Value =
    || json!({ "accel_m_s2": 0.0, "omega_ext_rad_s": 0.0, "sense_velocity_m_s": 1.0 });

/// The titanium cell's published inclusive maximum of temperature_k is
/// answered, from a start position part of the way out and not from rest,
/// and the description says so with the two numbers.
#[tokio::test]
async fn the_titanium_cells_largest_temperature_is_answered_from_a_stretched_start_and_not_from_rest() {
    let base = spawn_app().await;
    let ti64 = describe(&base, "materials", "ti64_lattice").await;
    let field = ti64["load_fields"].as_array().unwrap().iter().find(|f| f["name"] == "temperature_k").unwrap();
    assert_eq!(field["maximum"], 1268.15);
    assert_eq!(field["maximum_exclusive"], false);
    let bound = field["bound"].as_str().unwrap();
    assert!(bound.contains("a step at exactly 1268.15 K lands from a start position of 3e-5 m in each component"), "{bound}");
    assert!(bound.contains("from the rest position the largest temperature that lands is 1192.3203110311847 K"), "{bound}");
    let step = |position: f64, temperature_k: f64| {
        let mut request = ti64["example_compute_step"].clone();
        request["parameters"]["position_m"] = json!([position, position, position]);
        request["parameters"]["temperature_k"] = json!(temperature_k);
        // The start's ledger is anchored at its own strain energy at 293.15 K.
        let cell = ti64_lattice::Ti64LatticeCell::annealed_column();
        request["parameters"]["start_energy_anchor_j"] =
            json!(3.0 * 0.5 * cell.stiffness_n_m()[0] * position * position);
        request
    };
    let (status, text) = post(&base, "/verify_physics", &step(3.0e-5, 1268.15)).await;
    assert_eq!(status, 200, "{text}");
    let (status, text) = post(&base, "/verify_physics", &step(0.0, 1268.15)).await;
    assert_eq!(status, 422, "{text}");
    assert!(text.contains("(YieldReached)"), "{text}");
    let (status, text) = post(&base, "/verify_physics", &step(0.0, 1192.3203110311847)).await;
    assert_eq!(status, 200, "{text}");
    let (status, text) = post(&base, "/verify_physics", &step(0.0, 1192.320311031185)).await;
    assert_eq!(status, 422, "{text}");
    assert!(text.contains("(YieldReached)"), "{text}");
    let (status, text) = post(&base, "/verify_physics", &step(3.0e-5, 1268.1500000000003)).await;
    assert_eq!(status, 422, "{text}");
    assert!(text.contains("(TemperatureOutsideCard)"), "{text}");
}

/// Seven requests, each of which reached a conservation refusal or a
/// non-finite state in one step on the engine of version 0.2.1. Each is now
/// turned away at the door by name.
#[tokio::test]
async fn seven_requests_outside_the_specimen_are_turned_away_at_the_door_by_name() {
    let base = spawn_app().await;
    let ti64 = describe(&base, "materials", "ti64_lattice").await;
    let grid = describe(&base, "energy", "grid_node").await;
    let drive = describe(&base, "signal", "tesseract").await;
    let zero = json!([0.0, 0.0, 0.0]);

    // Titanium, FirstLawBreach: at rest at the origin, force_n [1e7, 0, 0], 293.15 K, one step at the largest step.
    let mut load = TI64_REST_LOAD();
    load["force_n"] = json!([1.0e7, 0.0, 0.0]);
    let request = run_of(
        "materials",
        "ti64_lattice",
        json!({ "position_m": zero, "velocity_m_s": zero, "start_t_s": 0.0, "start_force_n": zero,
                "start_temperature_k": 293.15, "start_energy_anchor_j": 0.0, "start_boundary_work_j": 0.0 }),
        load,
        ti64["dt_s"]["ceiling_s"].as_f64().unwrap(),
        1,
    );
    let text = turned_away(&base, &request, "force_n[0]", largest(&ti64, "force_n"), "N", "ti64_lattice").await;
    assert!(text.contains("10000000 N is above"), "{text}");

    // Titanium, SpontaneousGeneration: velocity_m_s [3e6, 0, 0], no force, one step at the smallest step.
    let request = run_of(
        "materials",
        "ti64_lattice",
        json!({ "position_m": zero, "velocity_m_s": [3.0e6, 0.0, 0.0], "start_t_s": 0.0, "start_force_n": zero,
                "start_temperature_k": 293.15, "start_energy_anchor_j": 0.0, "start_boundary_work_j": 0.0 }),
        TI64_REST_LOAD(),
        ti64["dt_s"]["floor_s"].as_f64().unwrap(),
        1,
    );
    let text = turned_away(&base, &request, "velocity_m_s[0]", largest(&ti64, "velocity_m_s"), "m/s", "ti64_lattice").await;
    assert!(text.contains("3000000 m/s is above"), "{text}");

    // Fuse link, FirstLawBreach: current_a 4000, voltage_v 0, 293.15 K, loads zero, one step of 1e-6 s.
    let request = run_of(
        "energy",
        "grid_node",
        json!({ "current_a": 4000.0, "voltage_v": 0.0, "temperature_k": 293.15 }),
        GRID_REST_LOAD(),
        1.0e-6,
        1,
    );
    let text = turned_away(&base, &request, "current_a", largest(&grid, "current_a"), "A", "grid_node").await;
    assert!(text.contains("4000 A is above"), "{text}");

    // Fuse link, NonFiniteState: voltage_v 1e38.
    let request = run_of(
        "energy",
        "grid_node",
        json!({ "current_a": 0.0, "voltage_v": 1.0e38, "temperature_k": 293.15 }),
        GRID_REST_LOAD(),
        1.0e-6,
        1,
    );
    turned_away(&base, &request, "voltage_v", largest(&grid, "voltage_v"), "V", "grid_node").await;

    // Resonator: displacement_m 1 with sense_velocity_m_s 1 (FirstLawBreach), 1000 (SpontaneousGeneration)
    // and 1e12 (NonFiniteState), one step at the largest step.
    for displacement_m in [1.0, 1000.0, 1.0e12] {
        let request = run_of(
            "signal",
            "tesseract",
            json!({ "displacement_m": displacement_m, "velocity_m_s": 0.0, "energy_defect_j": 0.0 }),
            TESSERACT_REST_LOAD(),
            drive["dt_s"]["ceiling_s"].as_f64().unwrap(),
            1,
        );
        turned_away(&base, &request, "displacement_m", largest(&drive, "displacement_m"), "m", "tesseract").await;
    }
}

/// Each published bound is the specimen's own number, recomputed here from
/// the card's public functions, bit for bit.
#[tokio::test]
async fn each_new_bound_is_the_specimens_own_number() {
    let base = spawn_app().await;
    let ti64 = describe(&base, "materials", "ti64_lattice").await;
    let cell = ti64_lattice::Ti64LatticeCell::annealed_column();
    let position = field(&ti64, "position_m");
    let highest_m = position["maximum"].as_f64().unwrap();
    let lowest_m = position["minimum"].as_f64().unwrap();
    // The position: the card's own yield test passes at each end of the span
    // and fails at the next number, at the beta transus above the rest
    // position and at 250 K below it, on each ligament.
    for axis in 0..3 {
        for (end, temperature_k) in [(highest_m, ti64_lattice::BETA_TRANSUS_K), (lowest_m, ti64_lattice::CARD_TEMPERATURE_MIN_K)] {
            let beyond = if end > 0.0 { end.next_up() } else { end.next_down() };
            // The other two ligaments stand unstressed at their thermal length.
            let standing = |kelvin: f64, x: f64| {
                let mut at = [ti64_lattice::thermal_extension_m(0.02, kelvin); 3];
                at[axis] = x;
                at
            };
            assert!(!ti64_lattice::past_yield(&cell, standing(temperature_k, end), temperature_k), "axis {axis} {end}");
            assert!(ti64_lattice::past_yield(&cell, standing(temperature_k, beyond), temperature_k), "axis {axis} {beyond}");
            // Past the far end it is past yield at every temperature of the card.
            for kelvin in [250.0, 293.15, 700.0, 1268.15] {
                assert!(ti64_lattice::past_yield(&cell, standing(kelvin, beyond), kelvin), "axis {axis} {beyond} {kelvin}");
            }
        }
    }
    // At the reference temperature the yield position is the yield strain
    // times the ligament, to within a few last digits of the card's own arithmetic.
    let by_hand = ti64_lattice::uniaxial_yield_strain() * cell.length_m[0];
    let mut position_m = by_hand;
    while ti64_lattice::past_yield(&cell, [position_m, 0.0, 0.0], 293.15) {
        position_m = position_m.next_down();
    }
    while !ti64_lattice::past_yield(&cell, [position_m.next_up(), 0.0, 0.0], 293.15) {
        position_m = position_m.next_up();
    }
    assert!((position_m - by_hand).abs() <= 4.0 * f64::EPSILON * by_hand, "{position_m} {by_hand}");
    let growth_m = ti64_lattice::thermal_extension_m(0.02, ti64_lattice::BETA_TRANSUS_K)
        - ti64_lattice::thermal_extension_m(0.02, ti64_lattice::CARD_TEMPERATURE_MIN_K);
    assert!((highest_m - lowest_m - 2.0 * by_hand - growth_m).abs() < 1.0e-15, "{highest_m} {lowest_m}");
    // The velocity: 2 omega sqrt(x_y (x_y + g)), the largest speed a run in range lands.
    let velocity_m_s = largest(&ti64, "velocity_m_s");
    let omega = cell.max_natural_frequency_rad_s();
    assert_eq!(
        velocity_m_s.to_bits(),
        (2.0 * omega * (position_m * (highest_m - lowest_m - position_m)).sqrt()).to_bits()
    );
    assert!(velocity_m_s > 2.0 * omega * position_m);
    // The force: the yield strength times the section.
    let force_n = largest(&ti64, "force_n");
    assert_eq!(force_n.to_bits(), (ti64_lattice::YIELD_STRENGTH_MPA * 1.0e6 * cell.area_m2[0]).to_bits());
    assert_eq!(force_n, 3520.0);
    // At every step the card accepts, the load rule allows more than the force bound.
    let impulse: f64 = {
        let rule = ti64["dt_s"]["rule"].as_str().unwrap();
        let before = &rule[..rule.find(". The product").unwrap()];
        before.rsplit(" = ").next().unwrap().trim().parse().unwrap()
    };
    assert!(impulse / ti64["dt_s"]["ceiling_s"].as_f64().unwrap() > 1000.0 * force_n);

    let grid = describe(&base, "energy", "grid_node").await;
    let node = grid_node::GridNode::copper_fuse_link();
    let current_a = largest(&grid, "current_a");
    let voltage_v = largest(&grid, "voltage_v");
    assert_eq!(current_a.to_bits(), (2.0 * node.melt_enthalpy_j() / node.inductance_h).sqrt().to_bits());
    assert_eq!(voltage_v.to_bits(), (2.0 * node.melt_enthalpy_j() / node.capacitance_f).sqrt().to_bits());
    // The energy the feeder or the shunt holds at the bound is the heat from the reference temperature to the melting point.
    assert!((node.electrical_energy_j(current_a, 0.0) / node.melt_enthalpy_j() - 1.0).abs() < 1.0e-12);
    assert!((node.electrical_energy_j(0.0, voltage_v) / node.melt_enthalpy_j() - 1.0).abs() < 1.0e-12);
    assert_eq!(largest(&grid, "upstream_voltage_v").to_bits(), voltage_v.to_bits());
    assert_eq!(largest(&grid, "load_current_a").to_bits(), current_a.to_bits());
    // The plant voltage and the surge ceiling of the card are well inside.
    assert!(voltage_v > 4.0 * grid_node::SURGE_WITHSTAND_V && voltage_v > 12.0 * grid_node::CATALOG_VOLTAGE_V);

    let drive = describe(&base, "signal", "tesseract").await;
    let resonator = tesseract::Tesseract::drive_mode();
    assert_eq!(largest(&drive, "displacement_m").to_bits(), tesseract::AMPLITUDE_LIMIT_M.to_bits());
    let speed_m_s = largest(&drive, "velocity_m_s");
    assert_eq!(speed_m_s.to_bits(), (2.0 * resonator.limit_energy_j() / resonator.mass_kg).sqrt().to_bits());
    assert_eq!(speed_m_s.to_bits(), resonator.speed_limit_m_s().to_bits());
    assert!((resonator.mechanical_energy_j(0.0, speed_m_s) / resonator.limit_energy_j() - 1.0).abs() < 1.0e-12);
    assert_eq!(
        largest(&drive, "energy_defect_j").to_bits(),
        (tesseract::RESIDUAL_CONSERVATION_LIMIT * resonator.limit_energy_j()).to_bits()
    );

    // Each bound is stated with its reason.
    for (card, name, reason) in [
        (&ti64, "position_m", "specimen's yield"),
        (&ti64, "velocity_m_s", "the highest speed along a ligament that a run"),
        (&ti64, "force_n", "specimen's yield"),
        (&grid, "current_a", "specimen's energy bound, not a melt"),
        (&grid, "voltage_v", "specimen's energy bound, not a melt"),
        (&grid, "upstream_voltage_v", "the bound of voltage_v"),
        (&grid, "load_current_a", "the bound of current_a"),
        (&drive, "displacement_m", "the card's amplitude limit, 1/128 m. It is a limit of the card and not the amplitude where the driven response folds"),
        (&drive, "velocity_m_s", "the energy of the mode at the amplitude limit"),
        (&drive, "energy_defect_j", "the same limit every step is held to"),
    ] {
        let bound = field(card, name)["bound"].as_str().unwrap();
        assert!(bound.contains(reason), "{name}: {bound}");
    }
}

/// On each of the eight fields: the bound itself is accepted, the next number
/// beyond it is 422 before a permit, on either side, in a step and in a run,
/// at the start, in the first segment and in a later one.
#[tokio::test]
async fn the_value_at_each_new_bound_is_accepted_and_the_next_number_is_turned_away() {
    let base = spawn_app().await;
    // The titanium position and velocity, whose starts carry a ledger to
    // match, are held at their bounds in tests/ti64_lattice_router.rs.
    let cases: [(&str, &str, &str, bool, bool, &str); 8] = [
        ("materials", "ti64_lattice", "force_n", true, true, "N"),
        ("energy", "grid_node", "current_a", false, false, "A"),
        ("energy", "grid_node", "voltage_v", false, false, "V"),
        ("energy", "grid_node", "upstream_voltage_v", false, true, "V"),
        ("energy", "grid_node", "load_current_a", false, true, "A"),
        ("signal", "tesseract", "displacement_m", false, false, "m"),
        ("signal", "tesseract", "velocity_m_s", false, false, "m/s"),
        ("signal", "tesseract", "energy_defect_j", false, false, "J"),
    ];
    let mut checked = 0;
    for (domain, name, key, vector, is_load, unit) in cases {
        let card = describe(&base, domain, name).await;
        let bound = largest(&card, key);
        for sign in [1.0, -1.0] {
            let components: &[usize] = if vector { &[0, 1, 2] } else { &[0] };
            for &component in components {
                let written = |value: f64| -> Value {
                    if vector {
                        let mut items = [0.0; 3];
                        items[component] = value;
                        json!(items)
                    } else {
                        json!(value)
                    }
                };
                let label = if vector { format!("{key}[{component}]") } else { key.to_string() };
                let side = if sign > 0.0 { "above" } else { "below" };
                let kind = if sign > 0.0 { "maximum" } else { "minimum" };
                let sentence = format!(
                    "Physical Failure: '{label}' {} {unit} is {side} {} {unit}, the inclusive {kind} of the {name} card",
                    message_number(sign * bound.next_up()),
                    message_number(sign * bound)
                );

                // The step.
                let mut step = card["example_compute_step"].clone();
                step["parameters"][key] = written(sign * bound);
                let (status, text) = post(&base, "/verify_physics", &step).await;
                assert!(status == 200 || !text.contains(&format!("'{label}'")), "{name} {label} at the bound: {text}");
                step["parameters"][key] = written(sign * bound.next_up());
                let (status, text) = post(&base, "/verify_physics", &step).await;
                assert_eq!(status, 422, "{name} {label}: {text}");
                assert!(text.starts_with(&sentence), "{text}\n{sentence}");
                assert!(!text.contains("sha256_seal"), "{text}");

                // The run: at the start, or in the first segment and in a later one.
                let positions: &[usize] = if is_load { &[0, 1] } else { &[usize::MAX] };
                for &position in positions {
                    for (value, accepted) in [(sign * bound, true), (sign * bound.next_up(), false)] {
                        let mut request = card["example_run_trajectory"].clone();
                        let segment = request["segments"][0].clone();
                        request["segments"] = json!([segment.clone(), segment]);
                        if is_load {
                            request["segments"][position]["load"][key] = written(value);
                        } else {
                            request["initial"][key] = written(value);
                        }
                        let (status, text) = post(&base, "/run_trajectory", &request).await;
                        if accepted && text.ends_with("(ContinuationWithoutUpstream).") {
                            // The card takes the value at its bound; it is a
                            // start that carries a run on, which arrives with from.
                            assert_eq!(status, 422, "{text}");
                            assert!(!text.contains("run_seal"), "{text}");
                        } else if accepted {
                            assert_eq!(status, 200, "{name} {label} at the bound, position {position}: {text}");
                            assert!(text.contains("run_seal"), "{text}");
                        } else {
                            assert_eq!(status, 422, "{name} {label}: {text}");
                            assert!(text.starts_with(&sentence), "{text}\n{sentence}");
                            assert!(!text.contains("run_seal"), "{text}");
                        }
                        checked += 1;
                    }
                }
            }
        }
    }
    assert_eq!(checked, 2 * 2 * (6 + 1 + 1 + 2 + 2 + 1 + 1 + 1));
    // The titanium position names the card's own refusal for it.
    let card = describe(&base, "materials", "ti64_lattice").await;
    let mut step = card["example_compute_step"].clone();
    step["parameters"]["position_m"] = json!([0.0, 1.0, 0.0]);
    let (_, text) = post(&base, "/verify_physics", &step).await;
    assert!(text.ends_with("of the ti64_lattice card (YieldReached). That state is not sealed."), "{text}");
}

/// A force the load rule refuses is still refused in the load rule's words,
/// and a force between the bound and the load rule in the bound's words.
#[tokio::test]
async fn the_titanium_load_rule_keeps_its_sentence() {
    let base = spawn_app().await;
    let card = describe(&base, "materials", "ti64_lattice").await;
    let mut step = card["example_compute_step"].clone();
    step["parameters"]["force_n"] = json!([0.0, 1.0e30, 0.0]);
    let (status, text) = post(&base, "/verify_physics", &step).await;
    assert_eq!(status, 422);
    assert!(text.contains("'force_n[1]'") && text.contains("force Nyquist limit"), "{text}");
    step["parameters"]["force_n"] = json!([0.0, 1.0e6, 0.0]);
    let (status, text) = post(&base, "/verify_physics", &step).await;
    assert_eq!(status, 422);
    assert!(text.contains("'force_n[1]' 1000000 N is above 3520 N"), "{text}");
}

/// A dataset is held to the same bounds: a grid row or a listed run with a
/// value outside one is a row that was not admitted and carries the door's
/// sentence, and the other rows run.
#[tokio::test]
async fn a_dataset_plan_is_held_to_the_new_bounds() {
    let base = spawn_app().await;
    let card = describe(&base, "energy", "grid_node").await;
    let bound = largest(&card, "current_a");
    let mut run = card["example_run_trajectory"].clone();
    run.as_object_mut().unwrap().remove("domain");
    run.as_object_mut().unwrap().remove("card");
    // The node at rest for four steps, with no load and no voltage held: a
    // start at the bound of the current lands there. The card's worked run
    // carries a load since 0.11.0, under which that start rings past it.
    run["initial"]["voltage_v"] = json!(0.0);
    run["segments"] = json!([{ "steps": 4, "load": { "upstream_voltage_v": 0.0, "load_current_a": 0.0, "ambient_k": 293.15 } }]);
    run["record_every"] = json!(1);
    let mut grid_base = run.clone();
    grid_base["initial"].as_object_mut().unwrap().remove("current_a");
    let plan = |values: Value| {
        json!({
            "domain": "energy", "card": "grid_node",
            "plan": { "kind": "grid", "base": grid_base, "axes": [{ "path": "initial.current_a", "values": values }] },
            "store": "none", "return": "inline", "answer": "full"
        })
    };
    let (status, text) = post(&base, "/run_dataset", &plan(json!([0.0, bound]))).await;
    assert_eq!(status, 200, "{text}");
    let answer: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(answer["head"]["completed"], 2, "{text}");
    // A grid is admitted row by row: the row past the bound is unadmitted
    // with the door's sentence, and the other row runs.
    let (status, text) = post(&base, "/run_dataset", &plan(json!([0.0, bound.next_up()]))).await;
    assert_eq!(status, 200, "{text}");
    let answer: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(answer["rows"][0]["outcome"], "completed", "{text}");
    assert_eq!(answer["rows"][1]["outcome"], "unadmitted", "{text}");
    assert!(answer["rows"][1]["reject_detail"].as_str().unwrap().contains("current_a"), "{text}");

    let mut outside = run.clone();
    outside["initial"]["current_a"] = json!(4000.0);
    let list = json!({
        "domain": "energy", "card": "grid_node",
        "plan": { "kind": "list", "runs": [run, outside] },
        "store": "none", "return": "inline", "answer": "full"
    });
    let (status, text) = post(&base, "/run_dataset", &list).await;
    assert_eq!(status, 200, "{text}");
    let answer: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(answer["rows"][0]["outcome"], "completed");
    assert_eq!(answer["rows"][1]["outcome"], "unadmitted");
    let detail = answer["rows"][1]["reject_detail"].as_str().unwrap();
    assert!(detail.starts_with("Physical Failure: 'current_a' 4000 A is above"), "{detail}");

    // One component of a 3-vector, set by a path.
    let lattice = describe(&base, "materials", "ti64_lattice").await;
    let force_n = largest(&lattice, "force_n");
    let mut lattice_base = lattice["example_run_trajectory"].clone();
    lattice_base.as_object_mut().unwrap().remove("domain");
    lattice_base.as_object_mut().unwrap().remove("card");
    lattice_base["segments"][0]["load"].as_object_mut().unwrap().remove("force_n");
    let plan = |value: f64| {
        json!({
            "domain": "materials", "card": "ti64_lattice",
            "plan": { "kind": "grid", "base": lattice_base, "axes": [
                { "path": "segments[0].load.force_n[0]", "values": [0.0] },
                { "path": "segments[0].load.force_n[1]", "values": [value] },
                { "path": "segments[0].load.force_n[2]", "values": [0.0] }
            ] },
            "store": "none", "return": "inline", "answer": "full"
        })
    };
    let (status, text) = post(&base, "/run_dataset", &plan(force_n)).await;
    assert_eq!(status, 200, "{text}");
    let (status, text) = post(&base, "/run_dataset", &plan(force_n.next_up())).await;
    assert_eq!(status, 200, "{text}");
    let answer: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(answer["rows"][0]["outcome"], "unadmitted", "{text}");
    assert!(answer["rows"][0]["reject_detail"].as_str().unwrap().contains("force_n"), "{text}");
}

/// Every phase field and every load field of every card says something of
/// its range: a bound with its reason, a rule, or a statement of its reach.
#[tokio::test]
async fn every_field_of_every_card_states_its_range_or_its_reach() {
    let base = spawn_app().await;
    let (status, text) = post(&base, "/describe_card", &json!({ "domain": "energy", "card": "thermal" })).await;
    assert_eq!(status, 200, "{text}");
    let client = Client::new();
    let list: Value = client.get(format!("{base}/cards")).send().await.unwrap().json().await.unwrap();
    let cards = list["cards"].as_array().unwrap();
    assert_eq!(cards.len(), 37);
    let mut fields = 0;
    for item in cards {
        let card = describe(&base, item["domain"].as_str().unwrap(), item["card"].as_str().unwrap()).await;
        for list in ["phase_fields", "load_fields"] {
            for field in card[list].as_array().unwrap() {
                let bound = field["bound"].as_str().unwrap();
                assert!(!bound.trim().is_empty(), "{} {} says nothing of its range", item["card"], field["name"]);
                fields += 1;
            }
        }
    }
    // The titanium card's five continuation fields are counted here.
    // 131 with the tyre's slip_held_s, the time its slip has been held.
    // 131 with the tokamak's elapsed_time_s, added in 0.7.0.
    // 157 with the four fields of the C-core reactor.
    // 165 with the eight fields of the 18650 thermal-runaway card, 168 with
    // the three of the grasped object, 177 with the nine of the JWST
    // momentum card, 182 with the five of the MAST VDE card, 191 with the nine
    // of the JWST momentum card on the two slopes of its sunshield, 192 with
    // the mission time of the generator in the Seebeck matched-load form.
    assert_eq!(fields, 192);
}

/// The fields that publish no bound state their reach, and what each states
/// is so.
#[tokio::test]
async fn each_statement_of_reach_is_in_the_description_and_holds() {
    let base = spawn_app().await;

    // Titanium torque: reported, and no part of the motion.
    let ti64 = describe(&base, "materials", "ti64_lattice").await;
    let torque = field(&ti64, "torque_nm");
    assert!(torque["minimum"].is_null() && torque["maximum"].is_null());
    let sentence = torque["bound"].as_str().unwrap();
    assert!(sentence.starts_with("no bound. The specimen is concentric: the torque is reported in the state and does not enter the motion, the stress or the ledger."), "{sentence}");
    let rest = ti64["example_run_trajectory"].clone();
    let mut turned = rest.clone();
    turned["segments"][0]["load"]["torque_nm"] = json!([3.0e38, -3.0e38, 1.0]);
    let (plain, twisted) = (run(&base, &rest).await, run(&base, &turned).await);
    assert_eq!(twisted["outcome"], "completed");
    assert_eq!(twisted["final_state"]["torque_nm"], json!([3.0e38, -3.0e38, 1.0]));
    for key in ["position_m_hex", "velocity_m_s_hex", "energy_j_hex", "von_mises_mpa_hex"] {
        assert_eq!(plain["final_state"][key], twisted["final_state"][key], "{key}");
    }
    assert_eq!(plain["max_residual_conservation_error_hex"], twisted["max_residual_conservation_error_hex"]);

    // The machine's PLL error: carried beside the rotor.
    let swing = describe(&base, "energy", "swing").await;
    let sentence = field(&swing, "pll_err_rad")["bound"].as_str().unwrap();
    assert!(sentence.starts_with("no bound. The PLL error is carried beside the rotor and does not enter the rotor's motion or the ledger."), "{sentence}");
    let rest = swing["example_run_trajectory"].clone();
    let mut slipped = rest.clone();
    slipped["initial"]["pll_err_rad"] = json!(3.0e38);
    let (plain, far) = (run(&base, &rest).await, run(&base, &slipped).await);
    assert_eq!(far["outcome"], "completed");
    assert_eq!(far["final_state"]["pll_slipped"], true);
    for key in ["delta_rad_hex", "delta_omega_rad_s_hex"] {
        assert_eq!(plain["final_state"][key], far["final_state"][key], "{key}");
    }
    assert_eq!(plain["max_residual_conservation_error_hex"], far["max_residual_conservation_error_hex"]);

    // The cold plate's heater power: the temperature is what the card holds.
    let thermal = describe(&base, "energy", "thermal").await;
    let sentence = field(&thermal, "heat_power_w")["bound"].as_str().unwrap();
    assert!(sentence.contains("refused as TemperatureOutsideCard, or as NegativeTemperature"), "{sentence}");
    let ceiling_s = thermal["dt_s"]["ceiling_s"].as_f64().unwrap();
    for (power_w, name) in [(3.0e38, "TemperatureOutsideCard"), (1.0e4, "TemperatureOutsideCard"), (-3.0e38, "NegativeTemperature"), (-1.0e6, "NegativeTemperature")] {
        let request = run_of("energy", "thermal", json!({ "temperature_k": 293.15 }), json!({ "heat_power_w": power_w, "ambient_k": 293.15 }), ceiling_s, 1);
        let body = run(&base, &request).await;
        assert_eq!(body["outcome"], "refused", "{power_w}");
        assert_eq!(body["reject"], name, "{power_w}");
    }

    // The resonator's rate and sense speed: a reported scale, and a 32-bit slot.
    let drive = describe(&base, "signal", "tesseract").await;
    for name in ["omega_ext_rad_s", "sense_velocity_m_s"] {
        let sentence = field(&drive, name)["bound"].as_str().unwrap();
        assert!(sentence.starts_with("no bound. The external rate and the sense speed set the reported Coriolis scale"), "{sentence}");
        assert!(sentence.contains("refused as a non-finite frame slot and that state is not sealed"), "{sentence}");
    }
    let ceiling_s = drive["dt_s"]["ceiling_s"].as_f64().unwrap();
    let rest = json!({ "displacement_m": 0.001, "velocity_m_s": 0.0, "energy_defect_j": 0.0 });
    let quiet = run(&base, &run_of("signal", "tesseract", rest.clone(), TESSERACT_REST_LOAD(), ceiling_s, 50)).await;
    let loud = run(
        &base,
        &run_of("signal", "tesseract", rest.clone(), json!({ "accel_m_s2": 0.0, "omega_ext_rad_s": 1.0e22, "sense_velocity_m_s": -1.0e22 }), ceiling_s, 50),
    )
    .await;
    assert_eq!(loud["outcome"], "completed");
    for key in ["displacement_m_hex", "velocity_m_s_hex", "mechanical_energy_j_hex"] {
        assert_eq!(quiet["final_state"][key], loud["final_state"][key], "{key}");
    }
    let over = run(
        &base,
        &run_of("signal", "tesseract", rest, json!({ "accel_m_s2": 0.0, "omega_ext_rad_s": 3.0e38, "sense_velocity_m_s": 3.0e38 }), ceiling_s, 1),
    )
    .await;
    assert_eq!(over["outcome"], "refused");
    assert_eq!(over["reject"], Value::Null);
    assert_eq!(over["reject_detail"], "Physical Violation: the integrated frame has a non-finite slot. The run is sealed with no step landed. The state at step 1 was refused and is not sealed.");

    // The tendon's velocity: part of the tension.
    let cable = describe(&base, "materials", "cable_stretch").await;
    let sentence = field(&cable, "velocity_m_s")["bound"].as_str().unwrap();
    assert!(sentence.starts_with("no fixed range. The velocity enters the tension with the elongation x"), "{sentence}");
    for (velocity_m_s, name) in [(1.0e6, "TensileFailure"), (-1.0e6, "Slack")] {
        let mut request = cable["example_run_trajectory"].clone();
        request["initial"]["elongation_m"] = json!(0.001);
        request["initial"]["velocity_m_s"] = json!(velocity_m_s);
        let (status, text) = post(&base, "/run_trajectory", &request).await;
        assert_eq!(status, 422, "{text}");
        assert!(text.contains(&format!("({name})")), "{text}");
    }

    // The capsule's speeds: what the speed through the air is, a band above
    // 68.96 km, Mach 1.0 to 40 and the compared corridor below it.
    let capsule = describe(&base, "aerospace", "capsule_stardust").await;
    for name in ["radial_speed_m_s", "transverse_speed_m_s"] {
        let sentence = field(&capsule, name)["bound"].as_str().unwrap();
        for words in [
            "The speed through the air is the square root of the sum of the squares of radial_speed_m_s and of transverse_speed_m_s less the air's own turning, 5.4197601852574874e-05 rad/s times radius_m.",
            "the speed through the air must be from 11900 to 12620 m/s, the top the highest speed the table was computed at, and a state outside is refused as AirSpeedOutsideCard",
            "The locked entry, which met all three lines, passed through 11911.98 to 12573.44 m/s there at every step from 0.05 s to 0.0005 s",
            "12487 to 12620 m/s in Table 1, 12173 m/s at the 68.96 km row from Table 4, of Mitcheltree, Wilmoth, Cheatwood, Brauckmann and Greene, Aerodynamics of Stardust Sample Return Capsule, AIAA 97-2304, 1997; that paper does not say whether these speeds are inertial or through the air",
            "the Mach number must be from 0.16 to 40, the ends of that table, and a state outside is refused as MachOutsideCard",
            "Any path is computed on the same equations and ledger.",
            "a state off it was not compared, and engines 0.7 to 0.9 refused it as OffComparedCorridor",
            "(at 40 km 1147.51 to 1350.11 m/s)",
            "Above 86 km the speed of sound is held at its 86 km value, 274.096 m/s",
            "is refused as PastFlightDeployment",
            "can be refused by the ledger at the largest step and lands at a smaller one",
        ] {
            assert!(sentence.contains(words), "{words}: {sentence}");
        }
        for words in [
            "no upper bound",
            "this speed less the air's own turning",
            "11400",
            "13100",
            "widened by about 500 m/s",
            "12625",
            "joined with Table 1",
        ] {
            assert!(!sentence.contains(words), "{words}: {sentence}");
        }
    }
    // The served definition is the kernel's: at entry the radial speed adds
    // about 135 m/s to the transverse speed through the air.
    let entry = &capsule["entry_state"];
    let radius_m = entry["radius_m"].as_f64().unwrap();
    let radial = entry["radial_speed_m_s"].as_f64().unwrap();
    let transverse = entry["transverse_speed_m_s"].as_f64().unwrap() - 5.4197601852574874e-05 * radius_m;
    let state = genesis_kernel::capsule_stardust::CapsuleState::try_entry_interface(
        &genesis_kernel::capsule_stardust::CapsuleStardust::sample_return_capsule(),
    )
    .unwrap();
    assert_eq!((radial * radial + transverse * transverse).sqrt().to_bits(), state.air_speed_m_s.to_bits());
    assert!(state.air_speed_m_s - transverse > 130.0);
    // Thirty times the entry speed, sealed through engine 0.6.0, is outside the band.
    let fast = json!({
        "radius_m": entry["radius_m"],
        "radial_speed_m_s": 30.0 * entry["radial_speed_m_s"].as_f64().unwrap(),
        "transverse_speed_m_s": 30.0 * entry["transverse_speed_m_s"].as_f64().unwrap()
    });
    let (status, text) = post(&base, "/run_trajectory", &run_of("aerospace", "capsule_stardust", fast, json!({}), 0.005, 20)).await;
    assert_eq!(status, 422, "{text}");
    assert!(text.contains("(AirSpeedOutsideCard)"), "{text}");
    // From 40 km at the entry speed: off the corridor the locked entry flew,
    // refused by engines 0.7 to 0.9, and computed now at either step.
    let deep = json!({
        "radius_m": genesis_kernel::capsule_stardust::LOCAL_EARTH_RADIUS_M + 40_000.0,
        "radial_speed_m_s": entry["radial_speed_m_s"],
        "transverse_speed_m_s": entry["transverse_speed_m_s"]
    });
    for dt_s in [0.05, 0.005] {
        let (status, text) = post(&base, "/run_trajectory", &run_of("aerospace", "capsule_stardust", deep.clone(), json!({}), dt_s, 20)).await;
        assert_eq!(status, 200, "{text}");
    }
    // The gate and the load say the same.
    let gate = capsule.to_string();
    assert!(!gate.contains("From 40 km at the entry speed a step of 0.05 s is refused and a step of 0.005 s completes"), "{gate}");
    assert!(gate.contains("The step that first meets both drogue conditions lands, so a run at a smaller step can end earlier, by less than one of the larger steps"), "{gate}");
}

/// The resonator's step ceiling states the reach of a run at that step,
/// measured inside the amplitude and speed limits, and the held runs it names
/// give what it says through the door.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn the_resonator_states_the_reach_of_a_driven_run_and_the_held_runs_agree() {
    let base = spawn_app().await;
    let drive = describe(&base, "signal", "tesseract").await;
    let from = drive["dt_s"]["ceiling_from"].as_str().unwrap();
    assert!(from.contains("The step method's own error is summed in the ledger over a run"), "{from}");
    assert!(from.contains("that run continued from its last state with its energy_defect_j is refused as SpontaneousGeneration on step 914,931 of the second run, where the same run carried straight through is refused"), "{from}");
    assert!(from.contains("A held 1,000 m/s^2 from rest completes a million steps at 4.3e-4 of the limit; a held 1,500 m/s^2 is refused as AmplitudeOutsideCard on step 97. The largest acceleration the load rule allows at this step, held from rest, lands one step and is refused as AmplitudeOutsideCard on the second"), "{from}");
    assert!(!from.contains("fold"), "{from}");

    let ceiling_s = drive["dt_s"]["ceiling_s"].as_f64().unwrap();
    let limit: f64 = {
        let rule = drive["dt_s"]["rule"].as_str().unwrap();
        let after = rule.split("C = omega * AMPLITUDE_LIMIT_M = ").nth(1).unwrap();
        after[..after.find(". The product").unwrap()].parse().unwrap()
    };
    // The largest acceleration the load rule allows at the largest step.
    let mut largest_m_s2 = limit / ceiling_s;
    while !(largest_m_s2 * ceiling_s <= limit) {
        largest_m_s2 = largest_m_s2.next_down();
    }
    let rest = json!({ "displacement_m": 0.0, "velocity_m_s": 0.0, "energy_defect_j": 0.0 });
    let held = |accel_m_s2: f64, steps: u64| {
        run_of(
            "signal",
            "tesseract",
            rest.clone(),
            json!({ "accel_m_s2": accel_m_s2, "omega_ext_rad_s": 0.0, "sense_velocity_m_s": 1.0 }),
            ceiling_s,
            steps,
        )
    };

    let body = run(&base, &held(largest_m_s2, 6)).await;
    assert_eq!(body["outcome"], "refused");
    assert_eq!(body["reject"], "AmplitudeOutsideCard");
    assert_eq!(body["steps_landed"], 1);
    assert!(body["run_seal"].is_string());

    let body = run(&base, &held(1500.0, 1_000_000)).await;
    assert_eq!(body["outcome"], "refused");
    assert_eq!(body["reject"], "AmplitudeOutsideCard");
    assert_eq!(body["steps_landed"], 96);

    let body = run(&base, &held(1000.0, 1_000_000)).await;
    assert_eq!(body["outcome"], "completed");
    assert_eq!(body["steps_landed"], 1_000_000);
    let worst = body["max_residual_conservation_error"].as_f64().unwrap();
    assert!((4.2e-10..4.3e-10).contains(&worst), "{worst}");
}
