//! The solar_system card on the three connections.
//!
//! A step is bit-checked against the compiled kernel step with the same proof
//! bytes, and its receipt verifies. A run of one day, 48 steps of 1800 s,
//! ends on the kernel loop's state to the bit and its tape verifies. A final
//! state handed back as a new start, with its ledger, continues the same
//! motion and the same ledger to the bit, and without its ledger it is
//! refused. Each refusal a request can reach is named and carries no proof: a
//! body outside its distance band, an orbit outside its band, a body where
//! the compared run never had it, a frame that moves, two bodies closer than
//! the compared run came, a centre of mass far from the origin, a carried
//! ledger out of balance, a step outside the card's floor and ceiling, a
//! missing input, a load the card does not have, a card constant. A grid of
//! three step sizes gives rows whose proofs are the runs alone, and `validate_series` compares the
//! Earth-to-Mars distance. The local, the web and the hosted connection
//! return the same bytes.
//!
//! No measurement is read here. The start is the card's own worked example,
//! the state of 1999-01-01 00:00 TDB from JPL Horizons.

mod answer_text;

use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::Duration;

use genesis_kernel::frame::LastStateFrame64;
use genesis_kernel::solar_system::{
    self, BODIES, BODY_NAMES, SolarSystem, SolarSystemLedger, SolarSystemLoad, SolarSystemState,
    StepReject,
};
use protocol_mcp_gateway::{VerifyPhysicsResponse, app, commit_request_proof};
use reqwest::Client;
use serde_json::{Map, Value, json};
use sha2::{Digest, Sha384};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpListener;
use tokio::process::{Child, ChildStdin, ChildStdout, Command};

// ------------------------------------------------------------------ helpers

async fn spawn_app() -> String {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    tokio::spawn(async move {
        axum::serve(listener, app()).await.unwrap();
    });
    tokio::task::yield_now().await;
    format!("http://127.0.0.1:{port}")
}

async fn post_text(base: &str, path: &str, body: &str) -> (u16, String) {
    let client = Client::builder().timeout(Duration::from_secs(300)).build().unwrap();
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
                let text = response.text().await.unwrap();
                answer_text::remember(&text);
                return (status, text);
            }
            Err(error) => {
                last_error = Some(error);
                tokio::time::sleep(Duration::from_millis(25)).await;
            }
        }
    }
    panic!("{path} never accepted a connection: {last_error:?}");
}

async fn post(base: &str, path: &str, body: &Value) -> (u16, String) {
    post_text(base, path, &answer_text::as_sent(body)).await
}

fn sha384_hex(bytes: &[u8]) -> String {
    hex::encode(Sha384::digest(bytes))
}

fn system() -> SolarSystem {
    SolarSystem::de440()
}

const NO_LOAD: SolarSystemLoad = SolarSystemLoad {};
const AU_M: f64 = solar_system::ASTRONOMICAL_UNIT_M;
const MARS: usize = solar_system::MARS;

type Vectors = [[f64; 3]; BODIES];

fn start() -> (Vectors, Vectors) {
    (solar_system::START_1999_POSITION_M, solar_system::START_1999_VELOCITY_M_S)
}

/// The ledger of a new start at these vectors: the kernel's own `E` and `L`
/// when the card takes them, and otherwise the same sums worked out here.
fn new_ledger(position_m: &Vectors, velocity_m_s: &Vectors) -> SolarSystemLedger {
    if let Some(state) = SolarSystemState::try_state(&system(), *position_m, *velocity_m_s) {
        return state.ledger();
    }
    let mu = solar_system::GM_M3_S2;
    let mut energy = 0.0;
    let mut turning = [0.0; 3];
    for i in 0..BODIES {
        let (r, v) = (position_m[i], velocity_m_s[i]);
        energy += 0.5 * mu[i] * (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]);
        turning[0] += mu[i] * (r[1] * v[2] - r[2] * v[1]);
        turning[1] += mu[i] * (r[2] * v[0] - r[0] * v[2]);
        turning[2] += mu[i] * (r[0] * v[1] - r[1] * v[0]);
        for j in i + 1..BODIES {
            let d = [position_m[j][0] - r[0], position_m[j][1] - r[1], position_m[j][2] - r[2]];
            let apart = (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt();
            if apart > 0.0 {
                energy -= mu[i] * mu[j] / apart;
            }
        }
    }
    SolarSystemLedger { energy_anchor: energy, booked_work: 0.0, angular_momentum_anchor: turning, booked_torque: [0.0; 3] }
}

/// The twenty-six vectors and a ledger as a request carries them.
fn phase_with(position_m: &Vectors, velocity_m_s: &Vectors, ledger: &SolarSystemLedger) -> Map<String, Value> {
    let mut map = Map::new();
    for body in 0..BODIES {
        map.insert(format!("{}_position_m", BODY_NAMES[body]), json!(position_m[body]));
        map.insert(format!("{}_velocity_m_s", BODY_NAMES[body]), json!(velocity_m_s[body]));
    }
    map.insert("energy_anchor".to_string(), json!(ledger.energy_anchor));
    map.insert("booked_work".to_string(), json!(ledger.booked_work));
    map.insert("angular_momentum_anchor".to_string(), json!(ledger.angular_momentum_anchor));
    map.insert("booked_torque".to_string(), json!(ledger.booked_torque));
    map
}

/// The twenty-six vectors with the ledger of a new start.
fn phase(position_m: &Vectors, velocity_m_s: &Vectors) -> Map<String, Value> {
    phase_with(position_m, velocity_m_s, &new_ledger(position_m, velocity_m_s))
}

fn step_body(position_m: &Vectors, velocity_m_s: &Vectors, dt_s: f64) -> Value {
    let mut parameters = phase(position_m, velocity_m_s);
    parameters.insert("dt_s".to_string(), json!(dt_s));
    json!({ "domain": "aerospace", "card": "solar_system", "parameters": parameters })
}

fn run_from(initial: Map<String, Value>, dt_s: f64, steps: u64, record_every: u64) -> Value {
    json!({
        "domain": "aerospace",
        "card": "solar_system",
        "initial": initial,
        "dt_s": dt_s,
        "segments": [{ "steps": steps, "load": {} }],
        "record_every": record_every
    })
}

fn run_body(position_m: &Vectors, velocity_m_s: &Vectors, dt_s: f64, steps: u64, record_every: u64) -> Value {
    run_from(phase(position_m, velocity_m_s), dt_s, steps, record_every)
}

/// The ledger a published state carries.
fn ledger_of(published_state: &Value) -> SolarSystemLedger {
    SolarSystemLedger {
        energy_anchor: published_state["energy_anchor"].as_f64().unwrap(),
        booked_work: published_state["booked_work"].as_f64().unwrap(),
        angular_momentum_anchor: vector(published_state, "angular_momentum_anchor"),
        booked_torque: vector(published_state, "booked_torque"),
    }
}

fn blank_frame(proof: [u8; 16]) -> LastStateFrame64 {
    LastStateFrame64 { t: 0.0, pos: [0.0; 3], vel: [0.0; 3], force_torque: 0.0, residual: 0.0, flags: 0, proof }
}

/// The kernel loop: the state after `steps` steps, and the largest residual.
fn direct_loop(position_m: Vectors, velocity_m_s: Vectors, dt_s: f64, steps: u64) -> (SolarSystemState, f64) {
    let system = system();
    let mut state = SolarSystemState::try_state(&system, position_m, velocity_m_s).unwrap();
    let mut frame = blank_frame([0; 16]);
    let mut worst = 0.0_f64;
    for _ in 0..steps {
        solar_system::step(&system, &mut state, &mut frame, NO_LOAD, dt_s).unwrap();
        worst = worst.max(state.residual_conservation_error);
    }
    (state, worst)
}

fn vector(published_state: &Value, name: &str) -> [f64; 3] {
    let list = published_state[name].as_array().unwrap_or_else(|| panic!("no {name}"));
    [list[0].as_f64().unwrap(), list[1].as_f64().unwrap(), list[2].as_f64().unwrap()]
}

fn vectors_of(published_state: &Value) -> (Vectors, Vectors) {
    let mut position_m = [[0.0; 3]; BODIES];
    let mut velocity_m_s = [[0.0; 3]; BODIES];
    for body in 0..BODIES {
        position_m[body] = vector(published_state, &format!("{}_position_m", BODY_NAMES[body]));
        velocity_m_s[body] = vector(published_state, &format!("{}_velocity_m_s", BODY_NAMES[body]));
    }
    (position_m, velocity_m_s)
}

fn bits3(a: [f64; 3]) -> [u64; 3] {
    [a[0].to_bits(), a[1].to_bits(), a[2].to_bits()]
}

/// Every published number of a state is the kernel's, to the bit, and each
/// hex sibling is the same number.
fn assert_state_is(published_state: &Value, state: &SolarSystemState, label: &str) {
    let (position_m, velocity_m_s) = vectors_of(published_state);
    for body in 0..BODIES {
        assert_eq!(bits3(position_m[body]), bits3(state.position_m[body]), "{label} {}", BODY_NAMES[body]);
        assert_eq!(bits3(velocity_m_s[body]), bits3(state.velocity_m_s[body]), "{label} {}", BODY_NAMES[body]);
        let hex = published_state[format!("{}_position_m_hex", BODY_NAMES[body])].as_array().unwrap();
        for axis in 0..3 {
            assert_eq!(hex[axis], format!("{:016x}", state.position_m[body][axis].to_bits()), "{label}");
        }
    }
    for (name, value) in [
        ("t_s", state.t_s),
        ("earth_mars_distance_m", state.earth_mars_distance_m),
        ("energy", state.energy),
        ("energy_anchor", state.energy_anchor),
        ("booked_work", state.booked_work),
        ("energy_defect", state.energy_defect),
        ("turning_defect", state.turning_defect),
        ("residual_conservation_error", state.residual_conservation_error),
    ] {
        assert_eq!(published_state[name].as_f64().unwrap().to_bits(), value.to_bits(), "{label} {name}");
    }
    for (name, value) in [
        ("earth_to_mars_m", state.earth_to_mars_m),
        ("angular_momentum", state.angular_momentum),
        ("angular_momentum_anchor", state.angular_momentum_anchor),
        ("booked_torque", state.booked_torque),
    ] {
        assert_eq!(bits3(vector(published_state, name)), bits3(value), "{label} {name}");
    }
    // 26 vectors and 4 more, 8 numbers, each with its hex: 76 keys.
    assert_eq!(published_state.as_object().unwrap().len(), 76, "{label}");
}

fn split_receipt(text: &str) -> (String, String) {
    const MARKER: &str = ",\"receipt_sha384\":\"";
    let at = text.rfind(MARKER).expect("a receipt");
    (format!("{}}}", &text[..at]), text[at + MARKER.len()..text.len() - 2].to_string())
}

/// The refusal is named and the answer carries no proof.
fn names(text: &str, name: &str) -> bool {
    text.contains(&format!("({name}). That state is not sealed."))
        && !text.contains("sha384_seal")
        && !text.contains("run_seal")
}


// --------------------------------------------------------------- the step

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_step_of_the_system_matches_the_compiled_step_with_the_same_proof_bytes_and_its_receipt_verifies() {
    let base = spawn_app().await;
    let (position_m, velocity_m_s) = start();
    let system = system();
    let mut seals = Vec::new();
    for dt_s in [system.gate_dt_s().unwrap(), 1800.0, system.floor_dt_s().unwrap()] {
        let request = step_body(&position_m, &velocity_m_s, dt_s);
        let text = serde_json::to_string(&request).unwrap();
        let (status, answer) = post_text(&base, "/verify_physics", &text).await;
        assert_eq!(status, 200, "{answer}");
        let response: VerifyPhysicsResponse = serde_json::from_str(&answer).unwrap();
        assert_eq!(response.inputs, request);
        assert_eq!(response.engine_version, "genesis_kernel_v0.13.1");

        let mut state = SolarSystemState::try_state(&system, position_m, velocity_m_s).unwrap();
        let mut frame = blank_frame(commit_request_proof(&request));
        solar_system::step(&system, &mut state, &mut frame, NO_LOAD, dt_s).unwrap();
        assert_eq!(frame.flags, 0);
        assert_state_is(&response.state, &state, "step");
        assert_eq!(response.residual_conservation_error.to_bits(), state.residual_conservation_error.to_bits());
        assert_eq!(response.sha384_seal, sha384_hex(&frame.to_bytes()));
        let mut naked = frame;
        naked.proof = [0; 16];
        assert_ne!(response.sha384_seal, sha384_hex(&naked.to_bytes()));
        let (document, receipt) = split_receipt(&answer);
        assert_eq!(receipt, response.receipt_sha384);
        assert_eq!(sha384_hex(document.as_bytes()), receipt);
        let (status, report) = post(&base, "/verify_receipt", &json!({ "response_text": answer })).await;
        assert_eq!(status, 200, "{report}");
        let report: Value = serde_json::from_str(&report).unwrap();
        assert_eq!(report["matches"], true, "{report}");
        let changed = answer.replacen("\"t_s\":", "\"t_s\":1", 1);
        assert_ne!(changed, answer);
        let (_, report) = post(&base, "/verify_receipt", &json!({ "response_text": changed })).await;
        let report: Value = serde_json::from_str(&report).unwrap();
        assert_eq!(report["matches"], false, "{report}");
        let (_, again) = post_text(&base, "/verify_physics", &text).await;
        assert_eq!(again, answer);
        seals.push(response.sha384_seal);
    }
    seals.sort();
    seals.dedup();
    assert_eq!(seals.len(), 3, "three different steps, three different proofs");
}

// ---------------------------------------------------------------- the run

/// One day, 48 steps of 1800 s with a frame every twelfth: the final state is
/// the kernel loop's to the bit, each frame carries the Earth-to-Mars vector,
/// and the tape verifies. The final state handed back as a new start, with
/// the decimals as they were returned and its ledger, continues to the kernel
/// loop's state after two days, ledger and all, to the bit, named in from;
/// without from it is refused by name. Handed back without its ledger it is refused, and with a ledger moved past the limit
/// it is refused by the ledger's name.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn one_day_ends_on_the_kernel_loop_and_handed_back_it_continues_the_same_motion() {
    let base = spawn_app().await;
    let (position_m, velocity_m_s) = start();
    let request = run_body(&position_m, &velocity_m_s, 1800.0, 48, 12);
    let (status, answer) = post(&base, "/run_trajectory", &request).await;
    assert_eq!(status, 200, "{}", &answer[..answer.len().min(600)]);
    let tape: Value = serde_json::from_str(&answer).unwrap();
    assert_eq!(tape["outcome"], "completed", "{}", tape["reject_detail"]);
    assert_eq!(tape["steps_landed"], 48);
    let (one_day, worst) = direct_loop(position_m, velocity_m_s, 1800.0, 48);
    assert_state_is(&tape["final_state"], &one_day, "one day");
    assert_eq!(tape["final_state"]["t_s"], 86_400.0);
    assert_eq!(tape["max_residual_conservation_error"].as_f64().unwrap().to_bits(), worst.to_bits());
    assert!(worst < 1.0e-13, "{worst:e}");

    let frames = tape["frames"].as_array().unwrap();
    assert_eq!(frames.len(), 4);
    let last = hex::decode(frames[3]["frame_hex"].as_str().unwrap()).unwrap();
    assert_eq!(f64::from_le_bytes(last[0..8].try_into().unwrap()), 86_400.0);
    for axis in 0..3 {
        let rounded = f32::from_le_bytes(last[8 + 4 * axis..12 + 4 * axis].try_into().unwrap());
        let rest = f32::from_le_bytes(last[20 + 4 * axis..24 + 4 * axis].try_into().unwrap());
        let read = f64::from(rounded) + f64::from(rest);
        assert!((read - one_day.earth_to_mars_m[axis]).abs() < 1.0e-3, "axis {axis}");
    }
    assert_eq!(u64::from_le_bytes(last[40..48].try_into().unwrap()), 0);

    let (status, report) = post(&base, "/verify_tape", &tape).await;
    assert_eq!(status, 200, "{report}");
    let report: Value = serde_json::from_str(&report).unwrap();
    assert_eq!(report["holds"], true, "{report}");

    let (handed_r, handed_v) = vectors_of(&tape["final_state"]);
    let handed_ledger = ledger_of(&tape["final_state"]);
    assert_eq!(handed_ledger, one_day.ledger());
    // Handed back with its ledger, the state carries the run on: without from
    // it is refused by name, and it names the first day's sealed run.
    let mut second = run_from(phase_with(&handed_r, &handed_v, &handed_ledger), 1800.0, 48, 48);
    let (status, text) = post(&base, "/run_trajectory", &second).await;
    assert_eq!(status, 422, "{text}");
    assert!(text.ends_with("(ContinuationWithoutUpstream)."), "{text}");
    second["from"] = json!({ "request": request, "run_seal": tape["run_seal"] });
    let (status, answer) = post(&base, "/run_trajectory", &second).await;
    assert_eq!(status, 200, "{}", &answer[..answer.len().min(600)]);
    let second: Value = serde_json::from_str(&answer).unwrap();
    let (two_days, _) = direct_loop(position_m, velocity_m_s, 1800.0, 96);
    let (r2, v2) = vectors_of(&second["final_state"]);
    for body in 0..BODIES {
        assert_eq!(bits3(r2[body]), bits3(two_days.position_m[body]), "{}", BODY_NAMES[body]);
        assert_eq!(bits3(v2[body]), bits3(two_days.velocity_m_s[body]), "{}", BODY_NAMES[body]);
    }
    // The time begins again at the new start; the ledger goes on as one run's.
    assert_eq!(second["final_state"]["t_s"], 86_400.0);
    let mut as_one = two_days;
    as_one.t_s = 86_400.0;
    assert_state_is(&second["final_state"], &as_one, "two days");

    // Handed back without its ledger, or without any one of its four parts: refused, no proof.
    for key in ["energy_anchor", "booked_work", "angular_momentum_anchor", "booked_torque"] {
        let mut bare = phase_with(&handed_r, &handed_v, &handed_ledger);
        bare.remove(key);
        let (status, text) = post(&base, "/run_trajectory", &run_from(bare, 1800.0, 48, 48)).await;
        assert_eq!(status, 422, "{key}: {text}");
        assert!(text.contains(&format!("Missing required parameter: '{key}' (InputMissing).")) && !text.contains("run_seal"), "{key}: {text}");
    }
    // A ledger begun again at the returned state is a new start: it is taken, and
    // its ledger is not the run's.
    let fresh = new_ledger(&handed_r, &handed_v);
    assert_ne!(fresh, handed_ledger);
    let (status, text) =
        post(&base, "/run_trajectory", &run_from(phase_with(&handed_r, &handed_v, &fresh), 1800.0, 48, 48)).await;
    assert_eq!(status, 200, "{}", &text[..text.len().min(600)]);
    let begun_again: Value = serde_json::from_str(&text).unwrap();
    assert_ne!(begun_again["run_seal"], second["run_seal"]);
    assert_eq!(begun_again["final_state"]["energy_anchor"].as_f64().unwrap().to_bits(), fresh.energy_anchor.to_bits());
    // The work booked over the first day moved by twice the limit: refused by the ledger's names.
    for (shift, name) in [(-2.0e-12, "SpontaneousGeneration"), (2.0e-12, "FirstLawBreach")] {
        let mut wrong = handed_ledger;
        wrong.booked_work += shift * solar_system::LEDGER_SCALE_ENERGY;
        let body = run_from(phase_with(&handed_r, &handed_v, &wrong), 1800.0, 48, 48);
        let (status, text) = post(&base, "/run_trajectory", &body).await;
        assert_eq!(status, 422, "{name}: {text}");
        assert!(names(&text, name), "{name}: {text}");
        let mut step = body.clone();
        let mut parameters = step["initial"].as_object().unwrap().clone();
        parameters.insert("dt_s".to_string(), json!(1800.0));
        step = json!({ "domain": "aerospace", "card": "solar_system", "parameters": parameters });
        let (status, text) = post(&base, "/verify_physics", &step).await;
        assert_eq!(status, 422, "{name}: {text}");
        assert!(names(&text, name), "{name}: {text}");
    }
    let mut twisted = handed_ledger;
    twisted.booked_torque[2] += 2.0e-12 * solar_system::LEDGER_SCALE_TURNING;
    let (status, text) =
        post(&base, "/run_trajectory", &run_from(phase_with(&handed_r, &handed_v, &twisted), 1800.0, 48, 48)).await;
    assert_eq!(status, 422, "{text}");
    assert!(names(&text, "AngularMomentumBreach"), "{text}");
}

// ----------------------------------------------------------- the refusals

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_state_off_the_card_is_refused_by_the_card_s_own_name_and_nothing_is_pulled_back() {
    let base = spawn_app().await;
    let (position_m, velocity_m_s) = start();
    let system = system();

    // Mars at three astronomical units from the Sun, along its own direction.
    let mut far = position_m;
    let sun = position_m[0];
    let x = [position_m[MARS][0] - sun[0], position_m[MARS][1] - sun[1], position_m[MARS][2] - sun[2]];
    let distance_m = (x[0] * x[0] + x[1] * x[1] + x[2] * x[2]).sqrt();
    for axis in 0..3 {
        far[MARS][axis] = sun[axis] + x[axis] * (3.0 * AU_M / distance_m);
    }
    assert_eq!(SolarSystemState::refusal(&system, &far, &velocity_m_s), Some(StepReject::DistanceOutsideCard));
    // Venus stopped dead against the Sun.
    let mut stopped = velocity_m_s;
    stopped[2] = velocity_m_s[0];
    assert_eq!(SolarSystemState::refusal(&system, &position_m, &stopped), Some(StepReject::OrbitOutsideCard));
    // Every body given 0.02 m/s more along x.
    let mut moving = velocity_m_s;
    for body in moving.iter_mut() {
        body[0] += 0.02;
    }
    assert_eq!(SolarSystemState::refusal(&system, &position_m, &moving), Some(StepReject::FrameOutsideCard));

    // Pallas at the place of Ceres, with the velocity of Ceres: two bodies at one point.
    let mut met_r = position_m;
    let mut met_v = velocity_m_s;
    met_r[11] = position_m[10];
    met_v[11] = velocity_m_s[10];
    assert_eq!(SolarSystemState::refusal(&system, &met_r, &met_v), Some(StepReject::BodiesCoincide));
    // The same state in the axes of the ecliptic of J2000: every vector turned about x by the obliquity.
    let (sin, cos) = (-23.439_291_1_f64).to_radians().sin_cos();
    let turn = |a: [f64; 3]| [a[0], cos * a[1] - sin * a[2], sin * a[1] + cos * a[2]];
    let ecliptic_r = position_m.map(turn);
    let ecliptic_v = velocity_m_s.map(turn);
    assert_eq!(SolarSystemState::refusal(&system, &ecliptic_r, &ecliptic_v), Some(StepReject::AxesOutsideCard));

    // Pallas 100 km from the centre of Ceres, inside it, with the velocity of
    // Ceres: a request that 0.6.0 accepted and sealed.
    let mut inside_r = position_m;
    inside_r[11] = position_m[10];
    inside_r[11][0] += 1.0e5;
    assert_eq!(inside_r[11], [113102846429.3008, 361147974066.7736, 146744266009.8253]);
    assert_eq!(SolarSystemState::refusal(&system, &inside_r, &met_v), None);
    // Every position moved 6e13 m along each axis: computed, the same motion with other rounding.
    let shifted_r = position_m.map(|r| [r[0] + 6.0e13, r[1] + 6.0e13, r[2] + 6.0e13]);
    assert_eq!(SolarSystemState::refusal(&system, &shifted_r, &velocity_m_s), None);
    // Uranus at 19.87 au from the Sun along its own direction: inside its band
    // of 18.0 to 20.4 au, outside the 19.88 to 20.10 au the compared run went
    // through up to the last range compared with measurement.
    let mut uranus_r = position_m;
    let x = [position_m[8][0] - sun[0], position_m[8][1] - sun[1], position_m[8][2] - sun[2]];
    let distance_m = (x[0] * x[0] + x[1] * x[1] + x[2] * x[2]).sqrt();
    for axis in 0..3 {
        uranus_r[8][axis] = sun[axis] + x[axis] * (19.87 * AU_M / distance_m);
    }
    assert_eq!(SolarSystemState::refusal(&system, &uranus_r, &velocity_m_s), None);

    for (r, v, name) in [
        (&far, &velocity_m_s, "DistanceOutsideCard"),
        (&position_m, &stopped, "OrbitOutsideCard"),
        (&position_m, &moving, "FrameOutsideCard"),
        (&met_r, &met_v, "BodiesCoincide"),
        (&ecliptic_r, &ecliptic_v, "AxesOutsideCard"),
    ] {
        let (status, text) = post(&base, "/verify_physics", &step_body(r, v, 1800.0)).await;
        assert_eq!(status, 422, "{name}: {text}");
        assert!(names(&text, name), "{name}: {text}");
        assert!(text.contains("Physical Failure"), "{text}");
        let (status, text) = post(&base, "/run_trajectory", &run_body(r, v, 1800.0, 4, 1)).await;
        assert_eq!(status, 422, "{name}: {text}");
        assert!(names(&text, name), "{name}: {text}");
        // by the card's name, and not by the sentence for a state that is not finite
        assert!(!text.contains("builds no starting state"), "{name}: {text}");
    }
    // Where 0.10 refused, the card computes: Uranus outside the compared run's
    // distances, the state moved 6e13 m, and Pallas 3000 km from Ceres, in
    // steps of 60 s. Each run completes, its tape verifies, and the moved
    // run's Earth-to-Mars distance is the unmoved run's to a metre.
    let mut beside_r = position_m;
    beside_r[11] = position_m[10];
    beside_r[11][0] += 3.0e6;
    for (r, v, dt_s) in [(&uranus_r, &velocity_m_s, 1800.0), (&shifted_r, &velocity_m_s, 1800.0), (&beside_r, &met_v, 60.0)] {
        let (status, text) = post(&base, "/run_trajectory", &run_body(r, v, dt_s, 48, 48)).await;
        assert_eq!(status, 200, "{text}");
        let tape: Value = serde_json::from_str(&text).unwrap();
        assert_eq!(tape["outcome"], "completed", "{text}");
        let (status, report) = post(&base, "/verify_tape", &tape).await;
        assert_eq!(status, 200, "{report}");
        assert_eq!(serde_json::from_str::<Value>(&report).unwrap()["holds"], true);
    }
    let (plain, _) = direct_loop(position_m, velocity_m_s, 1800.0, 48);
    let (moved, _) = direct_loop(shifted_r, velocity_m_s, 1800.0, 48);
    assert!((plain.earth_mars_distance_m - moved.earth_mars_distance_m).abs() < 1.0);

    // What the check of the axes cannot tell is taken: a turn about the z axis,
    // and Ceres and Pallas in each other's place.
    let (sin, cos) = 30.0_f64.to_radians().sin_cos();
    let about_z = |a: [f64; 3]| [cos * a[0] - sin * a[1], sin * a[0] + cos * a[1], a[2]];
    let (status, text) =
        post(&base, "/run_trajectory", &run_body(&position_m.map(about_z), &velocity_m_s.map(about_z), 1800.0, 4, 4)).await;
    assert_eq!(status, 200, "{text}");
    let mut exchanged_r = position_m;
    let mut exchanged_v = velocity_m_s;
    exchanged_r.swap(10, 11);
    exchanged_v.swap(10, 11);
    let (status, text) = post(&base, "/run_trajectory", &run_body(&exchanged_r, &exchanged_v, 1800.0, 4, 4)).await;
    assert_eq!(status, 200, "{text}");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_step_outside_the_floor_and_the_ceiling_a_missing_input_and_a_load_are_refused() {
    let base = spawn_app().await;
    let (position_m, velocity_m_s) = start();
    let system = system();
    let ceiling = system.gate_dt_s().unwrap();
    let floor = system.floor_dt_s().unwrap();
    assert_eq!((floor, ceiling), (0.87890625, 3600.0));

    for dt_s in [f64::from_bits(ceiling.to_bits() + 1), 86_400.0, f64::from_bits(floor.to_bits() - 1), 0.0, -1800.0] {
        let (status, text) = post(&base, "/verify_physics", &step_body(&position_m, &velocity_m_s, dt_s)).await;
        assert_eq!(status, 422, "{dt_s}: {text}");
        assert!(text.contains("dt_s") && !text.contains("sha384_seal"), "{dt_s}: {text}");
        let (status, text) = post(&base, "/run_trajectory", &run_body(&position_m, &velocity_m_s, dt_s, 2, 1)).await;
        assert_eq!(status, 422, "{dt_s}: {text}");
        assert!(!text.contains("run_seal"), "{dt_s}: {text}");
    }

    // A missing vector, by its name.
    let mut missing = step_body(&position_m, &velocity_m_s, 1800.0);
    missing["parameters"].as_object_mut().unwrap().remove("vesta_velocity_m_s");
    let (status, text) = post(&base, "/verify_physics", &missing).await;
    assert_eq!(status, 422, "{text}");
    assert!(text.contains("Missing required parameter: 'vesta_velocity_m_s' (InputMissing)."), "{text}");
    // A vector of two numbers, and one that is not a number.
    let mut short = step_body(&position_m, &velocity_m_s, 1800.0);
    short["parameters"]["moon_position_m"] = json!([1.0, 2.0]);
    let (status, text) = post(&base, "/verify_physics", &short).await;
    assert_eq!(status, 422, "{text}");
    assert!(text.contains("'moon_position_m' must contain three components"), "{text}");
    // A key the card does not have, and a constant of the card.
    for key in ["pluto_position_m", "gm_m3_s2", "speed_of_light_m_s"] {
        let mut extra = step_body(&position_m, &velocity_m_s, 1800.0);
        extra["parameters"][key] = json!([0.0, 0.0, 0.0]);
        let (status, text) = post(&base, "/verify_physics", &extra).await;
        assert_eq!(status, 422, "{key}: {text}");
        assert!(text.contains(key) && !text.contains("sha384_seal"), "{key}: {text}");
    }
    // The card has no load: a load in a segment is refused.
    let mut loaded = run_body(&position_m, &velocity_m_s, 1800.0, 2, 1);
    loaded["segments"][0]["load"] = json!({ "thrust_n": 1.0 });
    let (status, text) = post(&base, "/run_trajectory", &loaded).await;
    assert_eq!(status, 422, "{text}");
    assert!(text.contains("thrust_n") && !text.contains("run_seal"), "{text}");
}

// ---------------------------------------------------------------- dataset

/// A grid of three step sizes over the same eight steps from the 1999 state.
/// Each row carries the run seal of its run alone, each run alone ends on the
/// kernel loop's state to the bit, and the dataset verifies.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_grid_of_three_step_sizes_gives_rows_whose_proofs_are_the_runs_alone() {
    let base = spawn_app().await;
    let (position_m, velocity_m_s) = start();
    let steps_s = [900.0, 1800.0, 3600.0];
    let mut plan_base = run_body(&position_m, &velocity_m_s, 1800.0, 8, 4);
    plan_base.as_object_mut().unwrap().remove("domain");
    plan_base.as_object_mut().unwrap().remove("card");
    plan_base.as_object_mut().unwrap().remove("dt_s");
    let request = json!({
        "domain": "aerospace",
        "card": "solar_system",
        "plan": { "kind": "grid", "base": plan_base, "axes": [
            { "path": "dt_s", "values": steps_s }
        ] },
        "store": "none",
        "return": "inline", "answer": "full"
    });
    let (status, text) = post(&base, "/run_dataset", &request).await;
    assert_eq!(status, 200, "{}", &text[..text.len().min(600)]);
    let dataset: Value = serde_json::from_str(&text).unwrap();
    let rows = dataset["rows"].as_array().unwrap();
    assert_eq!(rows.len(), 3);
    let mut seals = Vec::new();
    for (row, dt_s) in rows.iter().zip(steps_s) {
        assert_eq!(row["outcome"], "completed", "dt_s {dt_s}: {}", row["reject_detail"]);
        let (status, alone) =
            post(&base, "/run_trajectory", &run_body(&position_m, &velocity_m_s, dt_s, 8, 4)).await;
        assert_eq!(status, 200, "{}", &alone[..alone.len().min(600)]);
        let alone: Value = serde_json::from_str(&alone).unwrap();
        assert_eq!(alone["steps_landed"], 8, "dt_s {dt_s}");
        assert_eq!(row["run_seal"], alone["run_seal"], "dt_s {dt_s}");
        assert_eq!(row["state_digest"], alone["state_digest"], "dt_s {dt_s}");
        let (kernel, _) = direct_loop(position_m, velocity_m_s, dt_s, 8);
        assert_state_is(&alone["final_state"], &kernel, "a row alone");
        assert!(!seals.contains(&row["run_seal"]), "dt_s {dt_s} reused a run seal");
        seals.push(row["run_seal"].clone());
    }
    let (status, report) = post(&base, "/verify_dataset", &dataset).await;
    assert_eq!(status, 200, "{report}");
    let report: Value = serde_json::from_str(&report).unwrap();
    assert_eq!(report["holds"], true, "{report}");
}

// ------------------------------------------------------- compared with a series

/// The kernel loop's Earth-to-Mars distance and the x of the vector after
/// each of twelve steps of 1800 s, given as measurements, are met with errors
/// of exactly zero and the answer carries the run seal of the run alone. A
/// known offset comes back as that offset.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn validate_series_compares_the_earth_to_mars_distance_of_a_run_and_carries_its_run_seal() {
    let base = spawn_app().await;
    let (position_m, velocity_m_s) = start();
    let run = run_body(&position_m, &velocity_m_s, 1800.0, 12, 4);
    let (status, alone) = post(&base, "/run_trajectory", &run).await;
    assert_eq!(status, 200, "{}", &alone[..alone.len().min(600)]);
    let alone: Value = serde_json::from_str(&alone).unwrap();

    let system = system();
    let mut state = SolarSystemState::try_state(&system, position_m, velocity_m_s).unwrap();
    let mut frame = blank_frame([0; 16]);
    let mut kept = Vec::new();
    for _ in 0..12 {
        solar_system::step(&system, &mut state, &mut frame, NO_LOAD, 1800.0).unwrap();
        kept.push(state);
    }

    let exact: Vec<Value> = kept
        .iter()
        .enumerate()
        .map(|(index, state)| json!({ "step": index + 1, "value": state.earth_mars_distance_m }))
        .collect();
    let body = json!({ "run": run, "quantity": "earth_mars_distance_m", "measurements": exact });
    let (status, text) = post(&base, "/validate_series", &body).await;
    assert_eq!(status, 200, "{}", &text[..text.len().min(600)]);
    let answer: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(answer["run_seal"], alone["run_seal"]);
    assert_eq!(answer["compared"], 12);
    assert_eq!(answer["rms_error"], 0.0);
    assert_eq!(answer["max_abs_error"], 0.0);

    // One component of a vector of the state: the x of the Earth-to-Mars vector.
    let exact_x: Vec<Value> = kept
        .iter()
        .enumerate()
        .map(|(index, state)| json!({ "step": index + 1, "value": state.earth_to_mars_m[0] }))
        .collect();
    let body = json!({ "run": run, "quantity": "earth_to_mars_m", "component": 0, "measurements": exact_x });
    let (status, text) = post(&base, "/validate_series", &body).await;
    assert_eq!(status, 200, "{}", &text[..text.len().min(600)]);
    let answer: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(answer["run_seal"], alone["run_seal"]);
    assert_eq!(answer["compared"], 12);
    assert_eq!(answer["rms_error"], 0.0);
    assert_eq!(answer["max_abs_error"], 0.0);

    // A range that read 1024 m long at every step. The distance is near
    // 2.4e11 m, where 1024 m added to it is exact.
    let offset: Vec<Value> = kept
        .iter()
        .enumerate()
        .map(|(index, state)| json!({ "step": index + 1, "value": state.earth_mars_distance_m + 1024.0 }))
        .collect();
    let body = json!({ "run": run, "quantity": "earth_mars_distance_m", "measurements": offset });
    let (status, text) = post(&base, "/validate_series", &body).await;
    assert_eq!(status, 200, "{}", &text[..text.len().min(600)]);
    let answer: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(answer["run_seal"], alone["run_seal"]);
    assert!((answer["rms_error"].as_f64().unwrap() - 1024.0).abs() < 1.0e-3, "{}", answer["rms_error"]);
    assert!((answer["mean_signed_error"].as_f64().unwrap().abs() - 1024.0).abs() < 1.0e-3, "{}", answer["mean_signed_error"]);
    assert!(answer.get("pass").is_none() && answer.get("verdict").is_none());

    // A vector without its component, a number with one, and a name the card
    // does not publish are refused and carry no run seal.
    for body in [
        json!({ "run": run, "quantity": "earth_to_mars_m", "measurements": [{ "step": 1, "value": 0.0 }] }),
        json!({ "run": run, "quantity": "earth_mars_distance_m", "component": 0, "measurements": [{ "step": 1, "value": 0.0 }] }),
        json!({ "run": run, "quantity": "light_time_s", "measurements": [{ "step": 1, "value": 0.0 }] }),
    ] {
        let (status, text) = post(&base, "/validate_series", &body).await;
        assert_eq!(status, 422, "{}", &text[..text.len().min(600)]);
        assert!(!text.contains("run_seal"), "{}", &text[..text.len().min(600)]);
    }
}

// ------------------------------------------------------ the three connections

const KEY_ALL: &str = "key-for-the-client-that-sees-everything";
const KEY_SYSTEM: &str = "solar-system-key-0123456789abcdef0123456789abcdef";

fn client_entry(label: &str, key: &str, cards: Value) -> Value {
    json!({ "client": label, "key_sha256": key_sha256_hex(key.as_bytes()), "cards": cards, "solvers": [], "max_in_flight": 8, "max_permits": 1 })
}

/// The local connection: `mcp_stdio` as a child process.
struct Local {
    _child: Child,
    stdin: ChildStdin,
    out: BufReader<ChildStdout>,
}

impl Local {
    async fn start() -> Local {
        let mut child = Command::new(env!("CARGO_BIN_EXE_mcp_stdio"))
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .kill_on_drop(true)
            .spawn()
            .unwrap();
        let stdin = child.stdin.take().unwrap();
        let out = BufReader::new(child.stdout.take().unwrap());
        let mut local = Local { _child: child, stdin, out };
        let init = json!({"jsonrpc":"2.0","id":0,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"test","version":"1"}}});
        local.raw(&init.to_string()).await;
        local
            .stdin
            .write_all(b"{\"jsonrpc\":\"2.0\",\"method\":\"notifications/initialized\"}\n")
            .await
            .unwrap();
        local
    }

    async fn raw(&mut self, line: &str) -> String {
        self.stdin.write_all(line.as_bytes()).await.unwrap();
        self.stdin.write_all(b"\n").await.unwrap();
        self.stdin.flush().await.unwrap();
        let mut answer = String::new();
        self.out.read_line(&mut answer).await.unwrap();
        answer.trim_end_matches('\n').to_string()
    }

    /// The whole answer line of a tool call, as the hosted connection frames it.
    async fn call(&mut self, name: &str, arguments: &Value) -> String {
        let line = answer_text::tool_call(7, name, arguments);
        self.raw(&line.to_string()).await
    }
}

/// The hosted connection: `hosted_door` as a child process, with a keys file.
struct Hosted {
    _child: Child,
    base: String,
    _dir: PathBuf,
}

impl Hosted {
    async fn start(keys: &Value) -> Hosted {
        let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("solar_system_hosted_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("keys.json");
        std::fs::write(&file, serde_json::to_string_pretty(keys).unwrap()).unwrap();
        let mut child = Command::new(env!("CARGO_BIN_EXE_hosted_door")).arg("--unsigned")
            .arg("--port")
            .arg("0")
            .arg("--keys")
            .arg(&file)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true)
            .spawn()
            .unwrap();
        let mut out = BufReader::new(child.stdout.take().unwrap());
        let mut line = String::new();
        out.read_line(&mut line).await.unwrap();
        let address = line
            .split("running on ")
            .nth(1)
            .unwrap_or_else(|| panic!("no address in {line:?}"))
            .split(' ')
            .next()
            .unwrap()
            .to_string();
        tokio::spawn(async move {
            let mut rest = String::new();
            while out.read_line(&mut rest).await.unwrap_or(0) > 0 {
                rest.clear();
            }
        });
        Hosted { _child: child, base: format!("http://{address}"), _dir: dir }
    }

    async fn call(&self, key: &str, tool: &str, arguments: &Value) -> String {
        let body = answer_text::tool_call(7, tool, arguments);
        let response = reqwest::Client::new()
            .post(format!("{}/mcp", self.base))
            .header("Content-Type", "application/json")
            .header("Accept", "application/json, text/event-stream")
            .header("MCP-Protocol-Version", "2025-06-18")
            .header("Authorization", format!("Bearer {key}"))
            .body(body.to_string())
            .send()
            .await
            .unwrap();
        assert_eq!(response.status().as_u16(), 200);
        response.text().await.unwrap()
    }
}

/// The text of a tool's result, and whether the connection marked it an error.
fn result_text(answer: &str) -> (String, bool) {
    let value: Value = serde_json::from_str(answer).unwrap();
    let text = value["result"]["content"][0]["text"]
        .as_str()
        .unwrap_or_else(|| panic!("no text in {answer}"))
        .to_string();
    answer_text::remember(&text);
    (text, value["result"]["isError"] == true)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_three_connections_list_the_system_and_give_the_same_bytes_for_its_description_and_its_tools() {
    let web = spawn_app().await;
    let mut local = Local::start().await;

    let (listed, error) = result_text(&local.call("list_cards", &json!({})).await);
    assert!(!error);
    let listed: Value = serde_json::from_str(&listed).unwrap();
    let cards = listed["cards"].as_array().unwrap();
    let entry = cards.iter().find(|card| card["card"] == "solar_system").expect("the system is listed");
    assert_eq!(entry["domain"], "aerospace");
    let summary = entry["specimen"].as_str().unwrap();
    // Since 0.14.4 a row gives what the model computes, the comparison and the gap; the whole spec sheet is the description's.
    assert!(!summary.contains("Nothing fitted") && summary.contains("The caller supplies every position and velocity"), "{summary}");

    let all_pairs: Vec<Value> = cards.iter().map(|card| json!([card["domain"], card["card"]])).collect();
    let keys = json!([
        client_entry("all", KEY_ALL, Value::Array(all_pairs)),
        client_entry("system", KEY_SYSTEM, json!([["aerospace", "solar_system"]])),
    ]);
    let hosted = Hosted::start(&keys).await;

    let pair = json!({"domain": "aerospace", "card": "solar_system"});
    let on_local = local.call("describe_card", &pair).await;
    let (described, error) = result_text(&on_local);
    assert!(!error);
    let (status, on_web) = post(&web, "/describe_card", &pair).await;
    assert_eq!(status, 200);
    assert_eq!(described, on_web);
    for key in [KEY_ALL, KEY_SYSTEM] {
        assert_eq!(hosted.call(key, "describe_card", &pair).await, on_local, "{key}");
    }

    // What the description holds: twenty-six vectors and the ledger, no load, fifteen refusals
    // each with its meaning, no flag, the start-state note, and the five
    // parts of the validation record.
    let description: Value = serde_json::from_str(&described).unwrap();
    let phase_names: Vec<&str> = description["phase_fields"].as_array().unwrap().iter().map(|field| field["name"].as_str().unwrap()).collect();
    assert_eq!(phase_names.len(), 30);
    assert_eq!(phase_names[0], "sun_position_m");
    assert_eq!(phase_names[25], "vesta_velocity_m_s");
    assert_eq!(phase_names[26..], ["energy_anchor", "booked_work", "angular_momentum_anchor", "booked_torque"]);
    assert!(description["load_fields"].as_array().unwrap().is_empty());
    let refusals: Vec<&str> = description["rejects"].as_array().unwrap().iter().map(|reject| reject["name"].as_str().unwrap()).collect();
    assert_eq!(
        refusals,
        [
            "NonFiniteInput", "NonFiniteState", "NonPositiveDt", "InvalidSystem", "DistanceOutsideCard",
            "OrbitOutsideCard", "BodiesCoincide", "FrameOutsideCard",
            "AxesOutsideCard", "SpontaneousGeneration", "FirstLawBreach", "AngularMomentumBreach",
            // The gateway's own three for this card follow the kernel's twelve.
            "DtBelowFloor", "DtAboveCeiling", "ContinuationWithoutUpstream"
        ]
    );
    for reject in description["rejects"].as_array().unwrap() {
        assert!(reject["meaning"].as_str().is_some_and(|meaning| !meaning.is_empty()), "{reject}");
    }
    assert!(description["advisory_flags"].as_array().unwrap().is_empty());
    assert!(description["start_state"].as_str().unwrap().contains("DE441 is a calculation"));
    assert!(description["start_state"].as_str().unwrap().contains("Every start carries a ledger"));
    // The ceiling's step error, body by body, as measured with the card.
    let rule = description.to_string();
    assert!(rule.contains("359 m for the Moon, 90 m for Mercury, 4.3 m for the Earth, 0.30 m for Venus, 0.21 m for Mars and under 0.2 m for each other body, the three asteroids among them"), "{rule}");
    assert!(!rule.contains("under 0.2 m for every other body"));
    assert!(!rule.contains("are refused as BodiesTooClose") && !rule.contains("is refused as OriginOutsideCard"));
    assert!(rule.contains("A state outside those distances, inside the bands, is computed") && rule.contains("the compared run goes on, past day 5,934.6 from the 1999 state"));
    assert!(rule.contains("Mars Odyssey to 2006 within 695 m at most (target 700 m)"));
    assert!(rule.contains("Saturn 9.03 to 9.97, Uranus 19.88 to 20.10, Neptune 29.96 to 30.14"));
    assert!(rule.contains("up to the last range compared with measurement, 16.2 years after the 1999 state"));
    assert!(rule.contains("From 16.2 to 21 years the run is compared with DE441, a calculation"));
    assert!(rule.contains("Its distance from the Sun is from 18.0 to 20.4 au; the compared run, to the last range compared with measurement, went through 19.88 to 20.10 au, and a distance outside that, inside the band, is computed"));
    assert!(rule.contains("Mercury and Venus at 0.2631 au") && rule.contains("the compared run's stayed 98 409 to 114 929 m from it"));
    assert!(!rule.contains("7,739") && !rule.contains("10.07") && !rule.contains("19.81") && !rule.contains("200000 m"));
    assert!(!rule.contains("for up to 21 years to where the stations measured"));
    let record = description["validation"].as_object().unwrap();
    let parts: Vec<&str> = record.keys().map(String::as_str).collect();
    assert_eq!(parts, ["object", "data", "pass_line", "result", "limits"]);
    assert!(record["data"].as_str().unwrap().contains("Nothing is fitted"));
    assert!(record["pass_line"].as_str().unwrap().contains("2d79e65df9fb8ad036444896e776e98d18cb98fc7b582f861747f27d79fd9679"));
    assert!(record["limits"].as_str().unwrap().contains("interpolation") && record["limits"].as_str().unwrap().contains("extrapolation"));
    // Sentences held in the one line of the list, the data, the result and
    // the limits.
    assert!(summary.contains(
        "Compared with 10,134 ranges of three spacecraft in four files, not used to choose it, in one run of 7,670 days: Mars Odyssey to 2006 within 695 m (target 700 m), 270 m RMS;"
    ), "{summary}");
    assert!(summary.contains("by at most 8 and 10 percent, from the 340 asteroids the card leaves out"), "{summary}");
    assert!(record["data"].as_str().unwrap().contains("The constants are those of DE440 and the start state is DE441's, through JPL Horizons"));
    let result = record["result"].as_str().unwrap();
    for words in [
        "In one run of 7,670 days from the 1999 state, none refused.",
        "equals the card's distance from JPL's DE441 ephemeris to about 5 m: this simpler model",
        "stays within 0.69 km of JPL's Earth-to-Mars distance over 7.75 years and 1.9 km over 21, within 2.7 km for Mercury, 3.2 km for Jupiter and 2.1 km for Saturn",
        "(2,389 ranges): 270 m rms and 695 m at most (target 700 m)",
        "44 ranges are outside the target, all in mid-2007, by at most 8 percent",
        "34 ranges outside, all in mid-2007, by at most 10 percent",
        "leaves out the station term of TDB - TT, up to 48 m one way and 22 m rms",
        "Odyssey up to 2006-10, 689 m at most (target 700 m); Odyssey after, 45 of 4,939 ranges outside, the worst 1.095 of the target",
        "33 of 1,993 outside, the worst 1.093; MESSENGER, 2,054 m at most, 0.640 of its target",
        "inside by 5 to 11 m of a target that is 678.3 m rounded up to 700; with 678.3 m, 11 to 13 ranges are outside",
        "the 340 asteroids the card leaves out, 730 of the 933 m in mid-2007, and no single asteroid does it",
        "the Sun's oblateness, 2,591 of 2,661 m",
        "the Earth's remaining 69 to 183 m is the Moon's error of 6 to 16 km",
        "Newton alone would be 351 km off by 2006-10",
        "end 5 to 24 m apart on the Moon after 21 years, so no 64-bit run holds the Moon to 10 m",
        "Against a 32-digit run the engine is 9.5 m off on the Moon and 0.92 m on the Earth",
        "78 m on the Moon, 17.9 m on Mercury and under 1 m on the Earth",
        "A second comparison program, on the same machine, with an independent reduction and an independent integrator, gives the same results",
    ] {
        assert!(result.contains(words), "{words}: {result}");
    }
    let limits = record["limits"].as_str().unwrap();
    for words in [
        "a reduction that has the term has a floor of about 5 m, and 1.3 m at Mercury",
        "grows about in proportion to time (T^0.96), so the target is too tight just after that span and twice too loose at 15 years",
        "leave about 20 km and 10 km with JPL's own ephemeris through this reduction",
        "this is an inference from DE441 and is not stated in the files' headers",
        "After that correction the Cassini file still leaves 590 m rms with JPL's own positions, so Jupiter and Saturn are outside the comparison",
        "FrameOutsideCard tests the speed of the centre of mass and nothing else",
        "AxesOutsideCard: the orbit of the Earth-Moon barycentre about the Sun must be inclined 21 to 27 degrees to the z axis",
        "falls from 24.94 to 22.56 degrees from 9999 BC to AD 9999",
        "a state given in ecliptic axes instead of ICRF axes has about 0 degrees and is refused",
        "a turn of the axes about the z axis, by any angle",
        "Ceres and Pallas, given in each other's place, which are taken without a refusal",
        "Two bodies at exactly one point are refused by a name of the card, BodiesCoincide",
        "Where the centre of mass is, is not held",
        "any pair but the Earth and the Moon under 0.26 au or any two asteroids under 0.38 au, are computed as point masses",
        "goes past day 5,934.6, where Saturn goes farther from the Sun than 9.97 au",
        "carry that one state forward for up to 16.2 years to where the stations measured (the run went on to 21 years, compared with DE441 alone)",
        "Neptune at another longitude for instance, has no comparison of its own",
    ] {
        assert!(limits.contains(words), "{words}: {limits}");
    }
    // The figures worked out after the run on rows chosen after it are not served.
    assert!(!limits.contains("163 of the 166") && !limits.contains("29 percent over"), "{limits}");

    // The tools that compute: the same bytes on the three connections.
    let step = description["example_compute_step"].clone();
    let run = description["example_run_trajectory"].clone();
    for (tool, route, arguments) in [("verify_step", "/verify_physics", &step), ("run_trajectory", "/run_trajectory", &run)] {
        let on_local = local.call(tool, arguments).await;
        let (text, error) = result_text(&on_local);
        assert!(!error, "{tool}: {text}");
        let (status, on_web) = post(&web, route, arguments).await;
        assert_eq!(status, 200, "{tool}: {on_web}");
        assert_eq!(text, on_web, "{tool}");
        for key in [KEY_ALL, KEY_SYSTEM] {
            assert_eq!(hosted.call(key, tool, arguments).await, on_local, "{tool} {key}");
        }
    }
    // A refusal is the same sentence on the local and the web connection.
    let mut off = step.clone();
    off["parameters"]["mars_position_m"] = json!([3.0 * AU_M, 0.0, 0.0]);
    let (text, error) = result_text(&local.call("verify_step", &off).await);
    assert!(error, "{text}");
    let (status, on_web) = post(&web, "/verify_physics", &off).await;
    assert_eq!(status, 422);
    assert!(names(&text, "DistanceOutsideCard") && on_web.contains("(DistanceOutsideCard)"), "{text} | {on_web}");
}

/// SHA-256 of a client's key, as the hosted door's keys file holds it: an
/// authentication hash, not a seal, so it stays SHA-256 under seal rule v2.
fn key_sha256_hex(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    hex::encode(Sha256::digest(bytes))
}
