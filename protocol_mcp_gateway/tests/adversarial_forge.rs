//! Float-armor blast against every `/verify_physics` route.
//!
//! Subnormals, a clock under every card's own floor, a clock above every
//! card's own ceiling, bare NaN, `1e309`, and `1e39`
//! are posted at each of the thirty cards. None of them may seal. A short
//! valid control on each route must return the post-step state, a frame seal,
//! and a receipt over the exact JSON.

use std::time::Duration;

use genesis_kernel::{
    battery_sag, cable_stretch, cell_18650pf, grid_node, hydraulic_fluid, reactor_spert3,
    reactor_spert3_fuel,
    reactor_triga_jsi,
    resonance, satellite_lageos1, solar_system, swing, tesseract, thermal, ti64_lattice,
};
use protocol_mcp_gateway::app;
use reqwest::Client;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use tokio::net::TcpListener;
use tokio::task::JoinSet;

const REPEATS: usize = 32;
const ROUTES: usize = 30;
const ATTACKS_PER_ROUTE: usize = 7;

#[derive(Clone, Copy, Debug)]
enum Kind {
    Subnormal,
    Clock,
    Nan,
    FrameSlot,
}

struct Attack {
    route: &'static str,
    kind: Kind,
    body: String,
}

struct Arm {
    route: &'static str,
    domain: &'static str,
    card: &'static str,
    parameters: Value,
    poison_key: &'static str,
    poison_vec3: bool,
    state_field: &'static str,
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn adversarial_forge_float_armor() {
    raise_nofile();
    let arms = arms();
    assert_eq!(arms.len(), ROUTES, "the forge must attack every route");
    let base_url = spawn_app().await;
    let client = Client::builder()
        .pool_max_idle_per_host(1024)
        .tcp_nodelay(true)
        .timeout(Duration::from_secs(60))
        .build()
        .unwrap();

    for arm in &arms {
        let body = envelope(
            arm.domain,
            arm.card,
            &serde_json::to_string(&arm.parameters).unwrap(),
        );
        assert_sealed_state(&client, &base_url, arm, &body).await;
    }

    let attacks = attacks(&arms);
    assert_eq!(attacks.len(), ROUTES * ATTACKS_PER_ROUTE);
    let mut join_set = JoinSet::new();
    for repeat in 0..REPEATS {
        for attack in &attacks {
            let client = client.clone();
            let url = format!("{base_url}/verify_physics");
            let route = attack.route;
            let kind = attack.kind;
            let body = attack.body.clone();
            join_set.spawn(async move {
                let response = match client
                    .post(&url)
                    .header("content-type", "application/json")
                    .body(body)
                    .send()
                    .await
                {
                    Ok(response) => response,
                    Err(err) => {
                        return Miss {
                            route,
                            kind,
                            status: 0,
                            detail: format!("repeat {repeat} transport: {err}"),
                        };
                    }
                };
                let status = response.status().as_u16();
                let text = response.text().await.unwrap_or_else(|err| err.to_string());
                let detail = match expect(kind, status, &text) {
                    Ok(()) => String::new(),
                    Err(detail) => format!("repeat {repeat}: {detail}"),
                };
                Miss {
                    route,
                    kind,
                    status,
                    detail,
                }
            });
        }
    }

    let expected = REPEATS * attacks.len();
    let mut completed = 0usize;
    let mut failures = Vec::new();
    while let Some(joined) = join_set.join_next().await {
        let hit = joined.expect("armor task panicked");
        completed += 1;
        if !hit.detail.is_empty() {
            failures.push(hit);
        }
    }

    let health = client
        .get(format!("{base_url}/health"))
        .send()
        .await
        .expect("listener died after the armor blast");
    assert_eq!(
        health.status().as_u16(),
        200,
        "listener died after the armor blast"
    );
    assert_eq!(completed, expected, "dropped in-flight armor requests");
    assert!(
        failures.is_empty(),
        "{} of {expected} armor attacks were sealed or mis-statused. first: {} {:?} status {} {}",
        failures.len(),
        failures[0].route,
        failures[0].kind,
        failures[0].status,
        failures[0].detail,
    );
}

struct Miss {
    route: &'static str,
    kind: Kind,
    status: u16,
    detail: String,
}

fn expect(kind: Kind, status: u16, body: &str) -> Result<(), String> {
    if body.contains("sha256_seal") || body.contains("receipt_sha256") {
        return Err(format!("sealed a rejected step: {status} {body}"));
    }
    match kind {
        Kind::Subnormal => {
            if status != 422 || !body.contains("not a finite normal number") {
                return Err(format!("subnormal was not armored: {status} {body}"));
            }
        }
        Kind::Clock => {
            let named = body.contains("the step floor of the")
                || body.contains("the step ceiling of the");
            if status != 422 || !body.contains("'dt_s'") || !named {
                return Err(format!("extreme clock was not armored: {status} {body}"));
            }
        }
        Kind::Nan => {
            if status != 400 && status != 422 {
                return Err(format!("NaN boundary status {status} {body}"));
            }
        }
        Kind::FrameSlot => {
            if status != 422 || !body.contains("frame slot") {
                return Err(format!(
                    "frame-slot boundary was not armored: {status} {body}"
                ));
            }
        }
    }
    Ok(())
}

async fn assert_sealed_state(client: &Client, base_url: &str, arm: &Arm, body: &str) {
    let response = client
        .post(format!("{base_url}/verify_physics"))
        .header("content-type", "application/json")
        .body(body.to_string())
        .send()
        .await
        .unwrap_or_else(|err| panic!("{} valid control failed to send: {err}", arm.route));
    let status = response.status();
    let text = response
        .text()
        .await
        .unwrap_or_else(|err| panic!("{} valid control body: {err}", arm.route));
    assert_eq!(status.as_u16(), 200, "{} {text}", arm.route);
    let parsed: Value = serde_json::from_str(&text).unwrap_or_else(|err| panic!("{err}: {text}"));
    let seal = parsed["sha256_seal"].as_str().unwrap_or("");
    assert_eq!(seal.len(), 64, "{} {text}", arm.route);
    assert!(seal.chars().all(|c| c.is_ascii_hexdigit()), "{seal}");
    assert_receipt(&text);
    assert!(parsed["residual_conservation_error"].is_number(), "{text}");
    assert!(parsed["state"]["t_s"].is_number(), "{text}");
    assert_eq!(
        parsed["state"]["t_s_hex"].as_str().unwrap().len(),
        16,
        "{text}"
    );
    let field = &parsed["state"][arm.state_field];
    assert!(
        field.is_number() || field.is_array(),
        "{} state.{} missing from {text}",
        arm.route,
        arm.state_field
    );
    assert_state_hex(&parsed["state"], arm.route);
}

fn assert_receipt(text: &str) {
    const MARKER: &str = ",\"receipt_sha256\":\"";
    let pos = text
        .rfind(MARKER)
        .unwrap_or_else(|| panic!("receipt missing from {text}"));
    assert!(text.ends_with("\"}"), "{text}");
    let receipt = &text[pos + MARKER.len()..text.len() - 2];
    assert_eq!(receipt.len(), 64, "{receipt}");
    let document = format!("{}}}", &text[..pos]);
    let mut hasher = Sha256::new();
    hasher.update(document.as_bytes());
    assert_eq!(hex::encode(hasher.finalize()), receipt, "{text}");
    assert!(!document.contains("receipt_sha256"));
}

fn assert_state_hex(state: &Value, route: &str) {
    let object = state
        .as_object()
        .unwrap_or_else(|| panic!("{route} state is not an object"));
    for (key, value) in object {
        if key.ends_with("_hex") {
            continue;
        }
        if value.is_number() {
            let hex = object[&format!("{key}_hex")]
                .as_str()
                .unwrap_or_else(|| panic!("{route} missing {key}_hex"));
            let bits = value.as_f64().unwrap().to_bits();
            assert_eq!(hex, format!("{:016x}", bits), "{route} {key}");
            assert_eq!(
                f64::from_bits(u64::from_str_radix(hex, 16).unwrap()).to_bits(),
                bits
            );
        } else if value.is_array() {
            let hexes = object[&format!("{key}_hex")]
                .as_array()
                .unwrap_or_else(|| panic!("{route} missing {key}_hex"));
            let numbers = value.as_array().unwrap();
            assert_eq!(hexes.len(), numbers.len(), "{route} {key}");
            for (hex, number) in hexes.iter().zip(numbers) {
                let bits = number.as_f64().unwrap().to_bits();
                assert_eq!(
                    hex.as_str().unwrap(),
                    format!("{:016x}", bits),
                    "{route} {key}"
                );
            }
        }
    }
}

fn attacks(arms: &[Arm]) -> Vec<Attack> {
    let mut attacks = Vec::with_capacity(ROUTES * ATTACKS_PER_ROUTE);
    for arm in arms {
        let parameters = serde_json::to_string(&arm.parameters).unwrap();
        let subnormal = if arm.poison_vec3 {
            "[0.0,5e-324,0.0]"
        } else {
            "5e-324"
        };
        let nan = if arm.poison_vec3 {
            "[0.0,NaN,0.0]"
        } else {
            "NaN"
        };
        let overflow = if arm.poison_vec3 {
            "[0.0,1e309,0.0]"
        } else {
            "1e309"
        };
        let frame_slot = if arm.poison_vec3 {
            "[1e39,0.0,0.0]"
        } else {
            "1e39"
        };
        let cases = [
            (Kind::Subnormal, arm.poison_key, subnormal),
            (Kind::Subnormal, "dt_s", "1e-320"),
            // Above every card's own ceiling. The generator's is 4.2e6 s.
            (Kind::Clock, "dt_s", "1e9"),
            // Below every card's own floor. The lattice's is 1.3e-13 s.
            (Kind::Clock, "dt_s", "1e-20"),
            (Kind::Nan, arm.poison_key, nan),
            (Kind::Nan, arm.poison_key, overflow),
            (Kind::FrameSlot, arm.poison_key, frame_slot),
        ];
        for (kind, key, raw) in cases {
            let poisoned = replace_field(&parameters, key, raw);
            attacks.push(Attack {
                route: arm.route,
                kind,
                body: envelope(arm.domain, arm.card, &poisoned),
            });
        }
    }
    attacks
}

fn replace_field(object_json: &str, key: &str, raw_value: &str) -> String {
    let pattern = format!("\"{key}\":");
    let start = object_json
        .find(&pattern)
        .unwrap_or_else(|| panic!("missing {key} in {object_json}"))
        + pattern.len();
    let rest = &object_json[start..];
    let end = if rest.starts_with('[') {
        rest.find(']').expect("vector") + 1
    } else {
        rest.find([',', '}']).expect("scalar")
    };
    format!("{}{raw_value}{}", &object_json[..start], &rest[end..])
}

fn envelope(domain: &str, card: &str, parameters_json: &str) -> String {
    format!(
        r#"{{"domain":"{domain}","card":"{card}","parameters":{parameters_json}}}"#
    )
}

fn dt_in_window(gate: Option<f64>) -> f64 {
    gate.expect("card gate").min(0.1)
}

fn arms() -> Vec<Arm> {
    let cell = ti64_lattice::Ti64LatticeCell::annealed_column();
    let force = 0.4 * ti64_lattice::YIELD_STRENGTH_MPA * 1.0e6 * cell.area_m2[0];
    let machine = swing::SwingMachine::synchronous_60hz();
    let rest = swing::SwingState::try_rest(&machine).unwrap();
    vec![
        Arm {
            route: "materials/ti64_lattice",
            domain: "materials",
            card: "ti64_lattice",
            poison_key: "position_m",
            poison_vec3: true,
            state_field: "temperature_k",
            parameters: json!({
                "position_m": [0.0, 0.0, 0.0],
                "velocity_m_s": [0.0, 0.0, 0.0],
                "start_t_s": 0.0, "start_force_n": [0.0, 0.0, 0.0], "start_temperature_k": 293.15, "start_energy_anchor_j": 0.0, "start_boundary_work_j": 0.0,
                "force_n": [force, 0.0, 0.0],
                "torque_nm": [0.0, 0.0, 0.0],
                "temperature_k": ti64_lattice::REFERENCE_TEMPERATURE_K,
                "dt_s": cell.gate_dt_s().unwrap(),
            }),
        },
        Arm {
            route: "energy/grid_node",
            domain: "energy",
            card: "grid_node",
            poison_key: "current_a",
            poison_vec3: false,
            state_field: "temperature_k",
            parameters: json!({
                "current_a": 0.0,
                "voltage_v": 0.0,
                "temperature_k": grid_node::REFERENCE_TEMPERATURE_K,
                "upstream_voltage_v": 25.0,
                "load_current_a": 0.0,
                "ambient_k": grid_node::REFERENCE_TEMPERATURE_K,
                "dt_s": grid_node::GridNode::copper_fuse_link().gate_dt_s().unwrap(),
            }),
        },
        Arm {
            route: "energy/swing",
            domain: "energy",
            card: "swing",
            poison_key: "delta_rad",
            poison_vec3: false,
            state_field: "delta_rad",
            parameters: json!({
                "delta_rad": rest.delta_rad,
                "delta_omega_rad_s": rest.delta_omega_rad_s,
                "pll_err_rad": rest.pll_err_rad,
                "p_mech_pu": swing::NOMINAL_P_MECH_PU,
                "dt_s": dt_in_window(machine.gate_dt_s()),
            }),
        },
        Arm {
            route: "energy/battery_sag",
            domain: "energy",
            card: "battery_sag",
            poison_key: "current_a",
            poison_vec3: false,
            state_field: "soc",
            parameters: json!({
                "soc": 1.0,
                "current_a": 1.0,
                "dt_s": dt_in_window(battery_sag::LiIonPack::pack_12s().gate_dt_s()),
            }),
        },
        Arm {
            route: "energy/cell_18650pf",
            domain: "energy",
            card: "cell_18650pf",
            poison_key: "current_a",
            poison_vec3: false,
            state_field: "soc",
            parameters: json!({
                "soc": 1.0,
                "held_current_a": 2.9,
                "current_a": 2.9,
                "dt_s": dt_in_window(cell_18650pf::Cell18650Pf::cell().gate_dt_s()),
            }),
        },
        Arm {
            route: "energy/thermal",
            domain: "energy",
            card: "thermal",
            poison_key: "heat_power_w",
            poison_vec3: false,
            state_field: "temperature_k",
            parameters: json!({
                "temperature_k": thermal::REFERENCE_TEMPERATURE_K,
                "heat_power_w": 1.0,
                "ambient_k": thermal::REFERENCE_TEMPERATURE_K,
                "dt_s": dt_in_window(thermal::ThermalNode::avionics_cold_plate().gate_dt_s()),
            }),
        },
        Arm {
            route: "signal/resonance",
            domain: "signal",
            card: "resonance",
            poison_key: "displacement_m",
            poison_vec3: false,
            state_field: "displacement_m",
            parameters: json!({
                "displacement_m": 0.0,
                "velocity_m_s": 0.0,
                "t_s": 0.0,
                "energy_anchor_j": 0.0,
                "boundary_work_j": 0.0,
                "accel_m_s2": 0.0,
                "dt_s": dt_in_window(resonance::DynamicOscillator::mount_10hz().gate_dt_s()),
            }),
        },
        Arm {
            route: "signal/tesseract",
            domain: "signal",
            card: "tesseract",
            poison_key: "displacement_m",
            poison_vec3: false,
            state_field: "displacement_m",
            parameters: json!({
                "displacement_m": 0.0,
                "velocity_m_s": 0.0,
                "energy_defect_j": 0.0,
                "accel_m_s2": 0.0,
                "omega_ext_rad_s": 0.0,
                "sense_velocity_m_s": tesseract::SENSE_PLATEAU_M_S,
                "dt_s": dt_in_window(tesseract::Tesseract::drive_mode().gate_dt_s()),
            }),
        },
        Arm {
            route: "materials/hydraulic_fluid",
            domain: "materials",
            card: "hydraulic_fluid",
            poison_key: "flow_m3_s",
            poison_vec3: false,
            state_field: "pressure_pa",
            parameters: json!({
                "pressure_pa": hydraulic_fluid::REFERENCE_PRESSURE_PA,
                "flow_m3_s": 0.0,
                "dt_s": dt_in_window(hydraulic_fluid::FluidColumn::mineral_oil().gate_dt_s()),
            }),
        },
        Arm {
            route: "materials/cable_stretch",
            domain: "materials",
            card: "cable_stretch",
            poison_key: "force_n",
            poison_vec3: false,
            state_field: "elongation_m",
            parameters: json!({
                "elongation_m": 0.0,
                "velocity_m_s": 0.0,
                "force_n": 0.0,
                "dt_s": dt_in_window(cable_stretch::Tendon::dyneema().gate_dt_s()),
            }),
        },
        Arm {
            route: "energy/generator_mmrtg_f1",
            domain: "energy",
            card: "generator_mmrtg_f1",
            poison_key: "mission_time_s",
            poison_vec3: false,
            state_field: "electrical_power_w",
            parameters: json!({
                "mission_time_s": 0.0,
                "dt_s": 86_400.0,
            }),
        },
        Arm {
            route: "aerospace/capsule_stardust",
            domain: "aerospace",
            card: "capsule_stardust",
            poison_key: "radius_m",
            poison_vec3: false,
            state_field: "load_g",
            parameters: json!({
                "radius_m": 6_503_140.0,
                "radial_speed_m_s": -1842.1416729435698,
                "transverse_speed_m_s": 12767.792058801891,
                "dt_s": 0.05,
            }),
        },
        Arm {
            route: "aerospace/capsule_insight",
            domain: "aerospace",
            card: "capsule_insight",
            poison_key: "radius_m",
            poison_vec3: false,
            state_field: "load_g",
            // The locked entry state, with the flight's lift, at the step ceiling.
            parameters: json!({
                "radius_m": 3522199.9999655997,
                "radial_speed_m_s": -1202.1585438056352,
                "transverse_speed_m_s": 5633.705477158609,
                "lift_over_drag": -0.031,
                "dt_s": 0.1,
            }),
        },
        Arm {
            route: "aerospace/satellite_lageos1",
            domain: "aerospace",
            card: "satellite_lageos1",
            poison_key: "position_m",
            poison_vec3: true,
            state_field: "radius_m",
            parameters: json!({
                // The compared start of LAGEOS-1 week 260829, 2026-08-23 0 h UTC.
                "epoch_utc_s": 840_715_200.0,
                "forecast_elapsed_s": 0.0,
                "position_m": [3767126.6844992605, 8956617.400906362, 7564717.856833777],
                "velocity_m_s": [4119.825668508274, 1676.7840658085788, -4053.2216929389388],
                "dt_s": 10.0,
            }),
        },
        Arm {
            route: "energy/reactor_spert3",
            domain: "energy",
            card: "reactor_spert3",
            poison_key: "reactivity_inserted_dollar",
            poison_vec3: false,
            state_field: "power_mw",
            parameters: json!({
                "coolant_temperature_k": reactor_spert3::COOLANT_TEMPERATURE_MIN_K,
                "reactivity_inserted_dollar": 1.0,
                "dt_s": dt_in_window(
                    reactor_spert3::ReactorSpert3::e_core_cold_startup().gate_dt_s()
                ),
            }),
        },
        Arm {
            route: "energy/cell_2170_m80",
            domain: "energy",
            card: "cell_2170_m80",
            poison_key: "current_a",
            poison_vec3: false,
            state_field: "terminal_v",
            // The cell steps by one second and by no other step.
            parameters: json!({
                "block_chamber_c": 0.0,
                "charge_removed_ah": 1.0,
                "current_a": -2.25,
                "case_temperature_c": 5.0,
                "dt_s": 1.0,
            }),
        },
        Arm {
            route: "signal/cantilever_eth",
            domain: "signal",
            card: "cantilever_eth",
            poison_key: "deflection_m",
            poison_vec3: false,
            state_field: "envelope_m",
            // 0.25 ms is the card's step ceiling.
            parameters: json!({
                "deflection_m": 1.0e-3,
                "velocity_m_s": 0.0,
                "dt_s": 2.5e-4,
            }),
        },
        Arm {
            route: "energy/cell_2170_model3",
            domain: "energy",
            card: "cell_2170_model3",
            poison_key: "current_a",
            poison_vec3: false,
            state_field: "terminal_v",
            // One second is the card's step ceiling.
            parameters: json!({
                "cell_number": 1000.0,
                "state_of_charge": 0.7,
                "capacity_ah": 4.24,
                "case_temperature_least_c": -7.5,
                "case_temperature_most_c": -7.5,
                "current_a": 2.25,
                "case_temperature_c": -7.5,
                "dt_s": 1.0,
            }),
        },
        Arm {
            route: "materials/creep_cmsx6",
            domain: "materials",
            card: "creep_cmsx6",
            poison_key: "stress_mpa",
            poison_vec3: false,
            state_field: "creep_extension_percent",
            // 40 s is under the card's step ceiling of about 50 s.
            parameters: json!({
                "creep_extension_percent": 0.3,
                "stress_mpa": 185.0,
                "dt_s": 40.0,
            }),
        },
        Arm {
            route: "energy/reactor_spert3_fuel",
            domain: "energy",
            card: "reactor_spert3_fuel",
            poison_key: "reactivity_inserted_dollar",
            poison_vec3: false,
            state_field: "power_mw",
            // The cold band, where the one flow tested is none.
            parameters: json!({
                "inlet_temperature_k": reactor_spert3_fuel::BAND_INLET_MIN_K[0],
                "reactivity_inserted_dollar": 1.0,
                "coolant_flow_m_per_s": reactor_spert3_fuel::BAND_FLOW_MIN_M_PER_S[0],
                "dt_s": dt_in_window(
                    reactor_spert3_fuel::ReactorSpert3Fuel::e_core_low_initial_power().gate_dt_s()
                ),
            }),
        },
        Arm {
            route: "energy/reactor_triga_jsi",
            domain: "energy",
            card: "reactor_triga_jsi",
            poison_key: "reactivity_inserted_dollar",
            poison_vec3: false,
            state_field: "power_mw",
            parameters: json!({
                "start_temperature_k": 298.15,
                "rod_table_core": reactor_triga_jsi::ROD_TABLE_CORE[1],
                "reactivity_inserted_dollar": 2.0,
                "rods_dropped": 0.0,
                "dt_s": dt_in_window(
                    reactor_triga_jsi::ReactorTrigaJsi::core_56_pulse_mode().gate_dt_s()
                ),
            }),
        },
        Arm {
            route: "aerospace/crater_ryugu_sci",
            domain: "aerospace",
            card: "crater_ryugu_sci",
            poison_key: "radius_m",
            poison_vec3: false,
            state_field: "radius_m",
            // A quarter of a second is the card's step ceiling.
            parameters: json!({
                "radius_m": 3.0,
                "dt_s": 0.25,
            }),
        },
        Arm {
            route: "signal/link_gothenburg_10010",
            domain: "signal",
            card: "link_gothenburg_10010",
            poison_key: "rain_rate_mm_h",
            poison_vec3: false,
            state_field: "path_loss_db",
            // One minute is the card's step ceiling.
            parameters: json!({
                "dry_path_loss_db": 52.3,
                "antenna_water_db": 0.5,
                "gauge_minute_elapsed_s": 0.0,
                "gauge_minute_rain_mm_h": 0.0,
                "rain_rate_mm_h": 18.0,
                "dt_s": 60.0,
            }),
        },
        Arm {
            route: "energy/grid_gb",
            domain: "energy",
            card: "grid_gb",
            poison_key: "loss_mw",
            poison_vec3: false,
            state_field: "frequency_hz",
            // 0.01 s is under the card's step ceiling of 0.0208 s.
            parameters: json!({
                "frequency_hz": 50.0,
                "inertia_mw_s": 200000.0,
                "demand_mw": 28000.0,
                "loss_mw": 1000.0,
                "dt_s": 0.01,
            }),
        },
        Arm {
            route: "aerospace/solar_system",
            domain: "aerospace",
            card: "solar_system",
            poison_key: "earth_position_m",
            poison_vec3: true,
            state_field: "earth_mars_distance_m",
            // The state of the card's worked example. One hour is the card's
            // step ceiling.
            parameters: solar_system_parameters(),
        },
        Arm {
            route: "materials/tire_eve_r19_ice",
            domain: "materials",
            card: "tire_eve_r19_ice",
            poison_key: "normal_load_n",
            poison_vec3: false,
            state_field: "longitudinal_force_n",
            // One hold from 5 s to 21 s, the card's step ceiling.
            parameters: json!({
                "distance_m": 1.5,
                "slip_held_s": 5.0,
                "normal_load_n": 6500.0,
                "inflation_pressure_pa": 250000.0,
                "slip_ratio": -0.15,
                "carriage_speed_m_s": 0.06,
                "dt_s": 16.0,
            }),
        },
        Arm {
            route: "energy/accumulator_sbo450",
            domain: "energy",
            card: "accumulator_sbo450",
            poison_key: "piston_speed_m_s",
            poison_vec3: false,
            state_field: "gas_force_n",
            // 2.5 ms at 0.03 m/s is 0.075 mm of piston travel.
            parameters: json!({
                "gas_mass_kg": 0.02747,
                "gas_pressure_pa": 6.0e6,
                "gas_temperature_k": 293.15,
                "wall_temperature_k": 293.55,
                "work_moved_j": 0.0,
                "heat_moved_j": 0.0,
                "piston_speed_m_s": 0.03,
                "dt_s": 0.0025,
            }),
        },
        Arm {
            route: "aerospace/los_jwst_fine_guide",
            domain: "aerospace",
            card: "los_jwst_fine_guide",
            poison_key: "centroid_x_arcsec",
            poison_vec3: false,
            state_field: "predicted_x_mas",
            // 0.064 s is one frame of the sensor.
            parameters: json!({
                "lock_x_arcsec": 17.3039684,
                "lock_y_arcsec": -15.2342768,
                "quiet_before_start_s": 30.016,
                "measured": 1.0,
                "centroid_x_arcsec": 17.3038845,
                "centroid_y_arcsec": -15.2345047,
                "counts_per_s": 553666.4,
                "dt_s": 0.064,
            }),
        },
        Arm {
            route: "energy/tokamak_jet_dte1",
            domain: "energy",
            card: "tokamak_jet_dte1",
            poison_key: "thermal_energy_j",
            poison_vec3: false,
            state_field: "fusion_power_w",
            // 1 ms is the card's step ceiling.
            parameters: json!({
                "thermal_energy_j": 4.0e6,
                "fast_ion_energy_j": 0.5e6,
                "elapsed_time_s": 0.0,
                "heating_power_w": 20.0e6,
                "ohmic_power_w": 0.5e6,
                "line_density_per_m3": 6.0e19,
                "plasma_current_a": 3.8e6,
                "toroidal_field_t": 4.0,
                "tritium_fraction": 0.5,
                "dt_s": 1.0e-3,
            }),
        },
        Arm {
            route: "materials/fingerpad_p02_glass",
            domain: "materials",
            card: "fingerpad_p02_glass",
            poison_key: "normal_force_n",
            poison_vec3: false,
            state_field: "torque_n_m",
            // 0.01 s is the card's step ceiling. Every step starts untwisted.
            parameters: json!({
                "twist_deg": 0.0,
                "normal_force_n": 2.0,
                "twist_rate_deg_s": 20.0,
                "dt_s": 0.01,
            }),
        },
    ]
}

/// The twenty-six vectors of 1999-01-01 00:00 TDB, a position and a velocity
/// for each body in the card's order, and a step of one hour.
fn solar_system_parameters() -> Value {
    let mut parameters = serde_json::Map::new();
    for body in 0..solar_system::BODIES {
        let name = solar_system::BODY_NAMES[body];
        parameters.insert(
            format!("{name}_position_m"),
            json!(solar_system::START_1999_POSITION_M[body]),
        );
        parameters.insert(
            format!("{name}_velocity_m_s"),
            json!(solar_system::START_1999_VELOCITY_M_S[body]),
        );
    }
    // The ledger of a new start: its own energy and angular momentum, nothing booked.
    let start = solar_system::SolarSystemState::try_state(
        &solar_system::SolarSystem::de440(),
        solar_system::START_1999_POSITION_M,
        solar_system::START_1999_VELOCITY_M_S,
    )
    .unwrap();
    parameters.insert("energy_anchor".to_string(), json!(start.energy_anchor));
    parameters.insert("booked_work".to_string(), json!(start.booked_work));
    parameters.insert("angular_momentum_anchor".to_string(), json!(start.angular_momentum_anchor));
    parameters.insert("booked_torque".to_string(), json!(start.booked_torque));
    parameters.insert("dt_s".to_string(), json!(3600.0));
    Value::Object(parameters)
}

async fn spawn_app() -> String {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    tokio::spawn(async move {
        axum::serve(listener, app()).await.unwrap();
    });
    tokio::task::yield_now().await;
    format!("http://127.0.0.1:{port}")
}

fn raise_nofile() {
    unsafe {
        #[repr(C)]
        struct Rlimit {
            cur: u64,
            max: u64,
        }
        unsafe extern "C" {
            fn getrlimit(resource: i32, rlim: *mut Rlimit) -> i32;
            fn setrlimit(resource: i32, rlim: *const Rlimit) -> i32;
        }
        let resource: i32 = if cfg!(target_os = "macos") { 8 } else { 7 };
        let mut lim = Rlimit { cur: 0, max: 0 };
        if getrlimit(resource, &mut lim) != 0 {
            return;
        }
        lim.cur = 65_536u64.min(lim.max);
        let _ = setrlimit(resource, &lim);
    }
}
