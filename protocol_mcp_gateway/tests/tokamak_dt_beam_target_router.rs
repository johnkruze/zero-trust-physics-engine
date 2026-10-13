//! The tokamak time-slice solver on the HTTP door: `POST /solve` against a
//! direct call to `genesis_solvers`, the limits the request itself shows, the
//! exact ends of the ranges, a sealed refusal, the description, and
//! `POST /verify_solution` against edits of a sealed answer.

use std::time::Duration;

use genesis_solvers::tokamak_dt_beam_target::{self as slice, SliceQuestion};
use protocol_mcp_gateway::app;
use reqwest::Client;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
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
    let url = format!("{base}{path}");
    let text = serde_json::to_string(body).unwrap();
    let mut last_error = None;
    for _ in 0..20 {
        match client
            .post(&url)
            .header(reqwest::header::CONTENT_TYPE, "application/json")
            .body(text.clone())
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

async fn solved(base: &str, request: &Value) -> (String, Value) {
    let (status, text) = post(base, "/solve", request).await;
    assert_eq!(status, 200, "{text}");
    let body = serde_json::from_str(&text).unwrap();
    (text, body)
}

async fn verdict(base: &str, body: &Value) -> Value {
    let (status, text) = post(base, "/verify_solution", body).await;
    assert_eq!(status, 200, "{text}");
    serde_json::from_str(&text).unwrap()
}

async fn assert_fails(base: &str, body: &Value, part: &str, label: &str) {
    let report = verdict(base, body).await;
    assert_eq!(report["holds"], false, "{label}");
    assert_eq!(report["cause"], part, "{label}");
}

fn hex_of(value: f64) -> String {
    format!("{:016x}", value.to_bits())
}

/// The hex is the solver's bits, and the decimal beside it reads back to them.
fn assert_number(section: &Value, key: &str, expected: f64) {
    assert_eq!(section[format!("{key}_hex")], hex_of(expected), "{key}");
    let decimal = section[key].as_f64().unwrap_or_else(|| panic!("{key}"));
    assert_eq!(decimal.to_bits(), expected.to_bits(), "{key}");
}

fn sha256_hex(bytes: &[u8]) -> String {
    hex::encode(Sha256::digest(bytes))
}

/// The receipt covers the wire bytes before it.
fn assert_receipt(text: &str, body: &Value) {
    let receipt = body["receipt_sha256"].as_str().expect("receipt");
    let tail = format!(",\"receipt_sha256\":\"{receipt}\"}}");
    let document = text.strip_suffix(&tail).expect("the receipt is last");
    assert_eq!(sha256_hex(format!("{document}}}").as_bytes()), receipt);
}

fn field_hash(samples: &[f64]) -> String {
    let mut field = Vec::with_capacity(8 + 8 * samples.len());
    field.extend_from_slice(&(samples.len() as u64).to_le_bytes());
    for sample in samples {
        field.extend_from_slice(&sample.to_le_bytes());
    }
    sha256_hex(&field)
}


/// Four shells of JET pulse 42976 at 13.35 s: the description's example.
fn four_shells() -> SliceQuestion {
    SliceQuestion {
        shell_volume_m3: vec![0.185232, 1.13996, 2.13983, 2.78539],
        electron_temperature_kev: vec![10.9616, 10.2851, 7.36622, 4.37061],
        ion_temperature_kev: vec![19.2543, 18.2776, 8.04433, 4.37061],
        electron_density_per_m3: vec![4.06983e19, 4.47374e19, 5.06836e19, 4.15438e19],
        deuterium_density_per_m3: vec![1.69228e19, 2.04297e19, 1.82594e19, 1.18337e19],
        tritium_density_per_m3: vec![1.37276e19, 1.62253e19, 1.76105e19, 1.52491e19],
        impurity_density_per_m3: vec![8.09588e17, 7.77101e17, 1.92707e18, 2.19865e18],
        impurity_charge: 6.0,
        impurity_mass_u: 12.0,
        beam_power_w: vec![77277.5, 319378.0, 593792.0, 150108.0],
        beam_mass_number: vec![2.0, 3.0],
        beam_energy_kev: vec![80.0, 152.0],
        beam_power_share: vec![0.5, 0.5],
        time_since_beam_change_s: 0.5503,
    }
}

fn request(question: &SliceQuestion) -> Value {
    json!({ "solver": "tokamak_dt_beam_target", "parameters": {
        "shell_volume_m3": question.shell_volume_m3,
        "electron_temperature_kev": question.electron_temperature_kev,
        "ion_temperature_kev": question.ion_temperature_kev,
        "electron_density_per_m3": question.electron_density_per_m3,
        "deuterium_density_per_m3": question.deuterium_density_per_m3,
        "tritium_density_per_m3": question.tritium_density_per_m3,
        "impurity_density_per_m3": question.impurity_density_per_m3,
        "impurity_charge": question.impurity_charge,
        "impurity_mass_u": question.impurity_mass_u,
        "beam_power_w": question.beam_power_w,
        "beam_mass_number": question.beam_mass_number,
        "beam_energy_kev": question.beam_energy_kev,
        "beam_power_share": question.beam_power_share,
        "time_since_beam_change_s": question.time_since_beam_change_s,
    } })
}

#[tokio::test]
async fn a_slice_matches_the_solver_bit_for_bit() {
    let base = spawn_app().await;
    let question = four_shells();
    let asked = request(&question);
    let (text, body) = solved(&base, &asked).await;
    let direct = slice::solve(&question).expect("the solver settles");

    assert_eq!(body["inputs"], asked);
    assert_eq!(body["engine_version"], "genesis_solvers_v0.8.0");
    assert_eq!(body["outcome"], "converged");
    assert!(body["reject"].is_null());
    assert_eq!(body["iterations"], direct.refinements);

    let result = &body["result"];
    assert_number(result, "thermal_dt_rate_per_s", direct.thermal_dt_rate_per_s);
    assert_number(result, "beam_target_dt_rate_per_s", direct.beam_target_dt_rate_per_s);
    assert_number(result, "beam_beam_dt_rate_per_s", direct.beam_beam_dt_rate_per_s);
    assert_number(result, "dt_rate_per_s", direct.dt_rate_per_s);
    assert_number(result, "dt_fusion_power_w", direct.dt_fusion_power_w);
    assert_number(result, "dd_neutron_rate_per_s", direct.dd_neutron_rate_per_s);
    assert_number(result, "dd_fusion_power_w", direct.dd_fusion_power_w);
    assert_number(result, "neutron_rate_per_s", direct.neutron_rate_per_s);
    assert_number(result, "fast_ion_count", direct.fast_ion_count);
    assert_number(result, "fast_ion_energy_j", direct.fast_ion_energy_j);
    assert_number(result, "fast_ion_energy_time_s", direct.fast_ion_energy_time_s);
    assert_number(result, "beam_power_to_electrons_w", direct.beam_power_to_electrons_w);
    assert_number(result, "beam_power_to_ions_w", direct.beam_power_to_ions_w);
    assert_number(result, "beam_power_thermalised_w", direct.beam_power_thermalised_w);
    assert_number(result, "beam_target_fused_fraction", direct.beam_target_fused_fraction);
    assert_number(result, "beam_beam_fused_fraction", direct.beam_beam_fused_fraction);
    assert_number(result, "fast_ion_fraction_max", direct.fast_ion_fraction_max);
    assert_number(result, "charge_excess_max", direct.charge_excess_max);
    assert_number(result, "charge_excess_min", direct.charge_excess_min);
    assert_eq!(result.as_object().unwrap().len(), 40, "a value and its hex for each");
    let densities = result["dt_rate_density_per_m3_s_hex"].as_array().expect("one per shell");
    assert_eq!(densities.len(), 4);
    for (hex, value) in densities.iter().zip(&direct.dt_rate_density_per_m3_s) {
        assert_eq!(hex, &json!(hex_of(*value)));
    }

    let invariants = &body["invariants"];
    assert_number(invariants, "particle_ledger_residual", direct.particle_ledger_residual);
    assert_number(invariants, "energy_ledger_residual", direct.energy_ledger_residual);
    assert_number(invariants, "power_ledger_residual", direct.power_ledger_residual);
    assert_eq!(invariants.as_object().unwrap().len(), 6);

    assert_eq!(body["field_sha256"], field_hash(&direct.dt_rate_density_per_m3_s));
    assert_eq!(body["solution_seal"].as_str().unwrap().len(), 64);
    assert_receipt(&text, &body);
    assert_eq!(verdict(&base, &body).await, json!({ "holds": true, "cause": null, "checked": protocol_mcp_gateway::WHOLE_SOLUTION_CHECKED }));

    // The same question asked again gives the same bytes.
    let (again, _) = solved(&base, &asked).await;
    assert_eq!(again, text);
}

#[tokio::test]
async fn what_the_request_itself_shows_is_refused_with_no_seal() {
    let base = spawn_app().await;
    let cases: [(fn(&mut SliceQuestion), &str); 23] = [
        (|q| q.ion_temperature_kev[3] = 0.22, "IonTemperatureOutOfRange"),
        (|q| q.ion_temperature_kev[0] = 41.9, "IonTemperatureOutOfRange"),
        (|q| q.electron_temperature_kev[0] = 13.2, "ElectronTemperatureOutOfRange"),
        (|q| q.electron_temperature_kev[3] = 0.19, "ElectronTemperatureOutOfRange"),
        // Each temperature inside its range, their ratio, 0.062, outside the span compared.
        (|q| q.electron_temperature_kev[2] = 0.5, "TemperatureRatioOutOfRange"),
        (|q| q.electron_density_per_m3[1] = 4.1e18, "ElectronDensityOutOfRange"),
        (|q| q.electron_density_per_m3[1] = 1.22e20, "ElectronDensityOutOfRange"),
        // A shell with no tritium, and one with no deuterium: fuel mixes never compared.
        (|q| q.tritium_density_per_m3[2] = 0.0, "TritiumShareOutOfRange"),
        (|q| q.deuterium_density_per_m3[3] = 0.0, "TritiumShareOutOfRange"),
        (|q| q.beam_energy_kev[1] = 152.5, "BeamEnergyOutOfRange"),
        (|q| q.beam_energy_kev[0] = 25.0, "BeamEnergyOutOfRange"),
        // The smallest volume above zero, once refused inside the solve as NonFiniteValue.
        (|q| q.shell_volume_m3[0] = 9.0e-7, "ShellVolumeOutOfRange"),
        // A subnormal is refused first by the number rule every request follows.
        (|q| q.shell_volume_m3[0] = 5e-324, "is not a finite normal number"),
        (|q| q.beam_mass_number[0] = 2.5, "BeamSpeciesUnknown"),
        (|q| q.beam_power_share = vec![0.5, 0.4], "BeamPowerShareOutOfRange"),
        (|q| q.deuterium_density_per_m3[0] = 4.0e19, "ThermalChargeExceedsElectrons"),
        // 2.6 percent more thermal charge than electrons, with the allowance at 2.5.
        (|q| { q.deuterium_density_per_m3[1] = 0.526 * 4.47374e19; q.tritium_density_per_m3[1] = 0.5 * 4.47374e19; q.impurity_density_per_m3[1] = 0.0; }, "ThermalChargeExceedsElectrons"),
        (|q| q.time_since_beam_change_s = -0.001, "TimeSinceBeamChangeOutOfRange"),
        (|q| q.time_since_beam_change_s = 100.5, "TimeSinceBeamChangeOutOfRange"),
        (|q| q.beam_power_w = vec![0.0; 4], "BeamPowerOutOfRange"),
        (|q| q.impurity_charge = 7.0, "ImpurityChargeOutOfRange"),
        (|q| q.impurity_mass_u = 22.0, "ImpurityMassOutOfRange"),
        (|q| { q.ion_temperature_kev.pop(); }, "ProfileLengthsDiffer"),
    ];
    for (change, name) in cases {
        let mut question = four_shells();
        change(&mut question);
        let (status, text) = post(&base, "/solve", &request(&question)).await;
        assert_eq!(status, 422, "{name}: {text}");
        assert!(text.contains(name), "{name}: {text}");
        assert!(!text.contains("solution_seal"), "{name}");
        assert!(slice::admit(&question).is_err(), "{name}");
    }
    // A key that is no input, and an input left out, are named.
    let mut extra = request(&four_shells());
    extra["parameters"]["plasma_current_a"] = json!(4.0e6);
    let (status, text) = post(&base, "/solve", &extra).await;
    assert_eq!(status, 422, "{text}");
    assert!(text.contains("plasma_current_a"), "{text}");
    let mut missing = request(&four_shells());
    missing["parameters"].as_object_mut().unwrap().remove("impurity_mass_u");
    let (status, text) = post(&base, "/solve", &missing).await;
    assert_eq!(status, 422, "{text}");
    assert!(text.contains("impurity_mass_u"), "{text}");
    // The time since the beam power changed is required: nothing stands in for it.
    let mut unstated = request(&four_shells());
    unstated["parameters"].as_object_mut().unwrap().remove("time_since_beam_change_s");
    let (status, text) = post(&base, "/solve", &unstated).await;
    assert_eq!(status, 422, "{text}");
    assert!(text.contains("time_since_beam_change_s"), "{text}");
    assert!(!text.contains("solution_seal"), "{text}");
}

#[tokio::test]
async fn the_exact_ends_of_the_ranges_are_answered() {
    let base = spawn_app().await;
    let mut question = four_shells();
    question.shell_volume_m3 = vec![200.0, 1.0e-6, 10.0, 10.0];
    question.electron_temperature_kev = vec![13.1, 0.199, 10.0, 0.73];
    question.ion_temperature_kev = vec![41.8, 0.224, 5.0, 10.0];
    question.electron_density_per_m3 = vec![1.21e20, 4.19e18, 5.0e19, 5.0e19];
    question.deuterium_density_per_m3 = question.electron_density_per_m3.iter().map(|n| 0.485 * n).collect();
    question.tritium_density_per_m3 = question.deuterium_density_per_m3.clone();
    question.impurity_density_per_m3 = vec![0.0; 4];
    question.impurity_charge = 6.5;
    question.impurity_mass_u = 10.0;
    question.beam_mass_number = vec![3.0, 2.0, 2.0];
    question.beam_energy_kev = vec![152.0, 152.0, 26.0];
    question.beam_power_share = vec![0.6, 0.3, 0.1];
    // The whole beam power is exactly the ceiling, none of it in the smallest shell, and the time exactly the end
    // of its range.
    question.beam_power_w = vec![4.4e7, 0.0, 9.0e5, 1.0e5];
    question.time_since_beam_change_s = 100.0;
    let (_, body) = solved(&base, &request(&question)).await;
    assert_eq!(body["outcome"], "converged", "{}", body["reject_detail"]);
    let direct = slice::solve(&question).expect("solves");
    assert_number(&body["result"], "dt_fusion_power_w", direct.dt_fusion_power_w);
}

/// Thermal ions that carry exactly 2.5 percent more charge than the
/// electrons, written as decimals, are answered from those densities in the
/// shells that hold no fast ions; one part in a thousand more is refused
/// before any physics. The beam goes into the first shell alone, which is
/// held neutral with few fast ions.
#[tokio::test]
async fn thermal_charge_at_exactly_the_allowance_is_answered_untrimmed() {
    let base = spawn_app().await;
    let mut question = four_shells();
    question.electron_density_per_m3 = vec![4.0e19; 4];
    question.deuterium_density_per_m3 = vec![2.0e19, 2.05e19, 2.05e19, 2.05e19];
    question.tritium_density_per_m3 = vec![2.0e19, 2.05e19, 2.05e19, 2.05e19];
    question.impurity_density_per_m3 = vec![0.0; 4];
    question.beam_power_w = vec![4000.0, 0.0, 0.0, 0.0];
    question.time_since_beam_change_s = 2.0;
    let (_, body) = solved(&base, &request(&question)).await;
    assert_eq!(body["outcome"], "converged", "{}", body["reject_detail"]);
    assert_eq!(body["inputs"]["parameters"]["deuterium_density_per_m3"], json!([2.0e19, 2.05e19, 2.05e19, 2.05e19]));
    let direct = slice::solve(&question).expect("solves");
    assert_number(&body["result"], "thermal_dt_rate_per_s", direct.thermal_dt_rate_per_s);
    assert_number(&body["result"], "dt_fusion_power_w", direct.dt_fusion_power_w);
    question.deuterium_density_per_m3 = vec![2.0e19, 2.054e19, 2.054e19, 2.054e19];
    let (status, text) = post(&base, "/solve", &request(&question)).await;
    assert_eq!(status, 422, "{text}");
    assert!(text.contains("ThermalChargeExceedsElectrons") && text.contains("QUASI_NEUTRALITY_ALLOWANCE"), "{text}");
}

/// The charge balance with the fast ions counted, both ways, on the door:
/// each side is a sealed refusal by name whose seal holds and which cannot
/// be made an answer.
#[tokio::test]
async fn the_charge_balance_with_the_fast_ions_is_a_sealed_refusal_both_ways() {
    let base = spawn_app().await;
    // The description's example, its thermal ions made neutral and its beam power tripled: its own fast ions put the
    // charge over the largest excess compared.
    let mut over = four_shells();
    over.deuterium_density_per_m3 = over.electron_density_per_m3.iter().map(|n| 0.5 * n).collect();
    over.tritium_density_per_m3 = over.deuterium_density_per_m3.clone();
    over.impurity_density_per_m3 = vec![0.0; 4];
    over.time_since_beam_change_s = 2.0;
    let mut under = over.clone();
    for power in &mut over.beam_power_w {
        *power *= 3.0;
    }
    // Its thermal ions at 0.75 of the electrons with a hundredth of the beam power: the charge is short of the
    // largest shortfall compared.
    for (deuterium, tritium) in under.deuterium_density_per_m3.iter_mut().zip(&mut under.tritium_density_per_m3) {
        *deuterium *= 0.75;
        *tritium *= 0.75;
    }
    for power in &mut under.beam_power_w {
        *power *= 0.01;
    }
    for (question, name, allowance) in [
        (over, "IonChargeExceedsElectrons", "QUASI_NEUTRALITY_FAST_ION_ALLOWANCE"),
        (under, "IonChargeBelowElectrons", "QUASI_NEUTRALITY_DEFICIT_ALLOWANCE"),
    ] {
        assert!(slice::admit(&question).is_ok(), "{name}");
        let (text, body) = solved(&base, &request(&question)).await;
        assert_eq!(body["outcome"], "refused", "{name}");
        assert_eq!(body["reject"], name);
        assert!(body["result"].is_null());
        assert!(body["reject_detail"].as_str().unwrap().contains(allowance), "{body}");
        assert_receipt(&text, &body);
        assert_eq!(verdict(&base, &body).await, json!({ "holds": true, "cause": null, "checked": protocol_mcp_gateway::WHOLE_SOLUTION_CHECKED }));
        let mut promoted = body.clone();
        promoted["outcome"] = json!("converged");
        assert_fails(&base, &promoted, "outcome", "a refusal made an answer").await;
    }
}

/// The steady-state limit on the door: exactly five energy times after a
/// beam change is answered with the bits of the settled slice, one 64-bit
/// step sooner is a sealed refusal that carries both times.
#[tokio::test]
async fn a_slice_too_soon_after_a_beam_change_is_a_sealed_refusal() {
    let base = spawn_app().await;
    let (_, settled) = solved(&base, &request(&four_shells())).await;
    assert_eq!(settled["outcome"], "converged");
    let energy_time = settled["result"]["fast_ion_energy_time_s"].as_f64().unwrap();
    assert_eq!(settled["result"]["fast_ion_energy_time_s_hex"], hex_of(energy_time));
    let end = slice::SETTLED_AFTER_ENERGY_TIMES * energy_time;
    assert!(end < 0.5503, "{end}");

    let mut question = four_shells();
    question.time_since_beam_change_s = end;
    let (_, at_the_end) = solved(&base, &request(&question)).await;
    assert_eq!(at_the_end["outcome"], "converged");
    assert_eq!(at_the_end["result"], settled["result"]);
    assert_eq!(at_the_end["field_sha256"], settled["field_sha256"]);
    assert_ne!(at_the_end["solution_seal"], settled["solution_seal"], "the input is sealed");

    question.time_since_beam_change_s = f64::from_bits(end.to_bits() - 1);
    let (text, body) = solved(&base, &request(&question)).await;
    assert_eq!(body["outcome"], "refused");
    assert_eq!(body["reject"], "BeamChangeTooRecent");
    assert!(body["result"].is_null());
    assert!(body["field_sha256"].is_null());
    let detail = body["reject_detail"].as_str().expect("the sentence");
    assert!(detail.contains(&format!("{energy_time}")), "{detail}");
    assert!(detail.contains("SETTLED_AFTER_ENERGY_TIMES"), "{detail}");
    assert_receipt(&text, &body);
    assert_eq!(verdict(&base, &body).await, json!({ "holds": true, "cause": null, "checked": protocol_mcp_gateway::WHOLE_SOLUTION_CHECKED }));
    let mut promoted = body.clone();
    promoted["outcome"] = json!("converged");
    assert_fails(&base, &promoted, "outcome", "a refusal made an answer").await;
    let mut later = body.clone();
    later["inputs"]["parameters"]["time_since_beam_change_s"] = json!(2.0);
    assert_fails(&base, &later, "inputs", "a longer time written in afterwards").await;
}

/// One shell of the description's example, deuterium beams only, thermal
/// ions at 0.565 of the electrons and the beam power scaled by `power`.
fn one_shell(power: f64) -> SliceQuestion {
    let full = four_shells();
    let electrons = full.electron_density_per_m3[1];
    SliceQuestion {
        shell_volume_m3: vec![full.shell_volume_m3[1]],
        electron_temperature_kev: vec![full.electron_temperature_kev[1]],
        ion_temperature_kev: vec![full.ion_temperature_kev[1]],
        electron_density_per_m3: vec![electrons],
        deuterium_density_per_m3: vec![0.2825 * electrons],
        tritium_density_per_m3: vec![0.2825 * electrons],
        impurity_density_per_m3: vec![0.0],
        beam_power_w: vec![full.beam_power_w[1] * power],
        beam_mass_number: vec![2.0, 2.0],
        time_since_beam_change_s: 2.0,
        ..full
    }
}

/// Fast ions above the largest share compared: a sealed refusal by name.
#[tokio::test]
async fn fast_ions_above_the_largest_share_compared_are_a_sealed_refusal() {
    let base = spawn_app().await;
    let found = match slice::solve(&one_shell(1.0)) {
        Ok(answer) => answer.fast_ion_fraction_max,
        Err(slice::SliceReject::FastIonShareTooLarge { share, .. }) => share,
        Err(slice::SliceReject::IonChargeBelowElectrons { deficit, .. }) => 1.0 - deficit - 0.565,
        Err(other) => panic!("{other:?}"),
    };
    let question = one_shell(0.445 / found);
    let (text, body) = solved(&base, &request(&question)).await;
    assert_eq!(body["outcome"], "refused");
    assert_eq!(body["reject"], "FastIonShareTooLarge");
    assert!(body["result"].is_null());
    assert!(body["reject_detail"].as_str().unwrap().contains("FAST_ION_SHARE_MAX"), "{body}");
    assert_receipt(&text, &body);
    assert_eq!(verdict(&base, &body).await, json!({ "holds": true, "cause": null, "checked": protocol_mcp_gateway::WHOLE_SOLUTION_CHECKED }));
    let (_, body) = solved(&base, &request(&one_shell(0.436 / found))).await;
    assert_eq!(body["outcome"], "converged", "{}", body["reject_detail"]);
    let share = body["result"]["fast_ion_fraction_max"].as_f64().unwrap();
    assert!(share > 0.435 && share <= slice::FAST_ION_SHARE_MAX, "{share}");
}

#[tokio::test]
async fn beam_beam_reactions_beyond_their_bound_are_a_sealed_refusal() {
    let base = spawn_app().await;
    let mut question = four_shells();
    question.shell_volume_m3 = vec![0.05; 4];
    question.beam_power_w = vec![1.1e7; 4];
    let (text, body) = solved(&base, &request(&question)).await;
    assert_eq!(body["outcome"], "refused");
    assert_eq!(body["reject"], "BeamBeamShareTooLarge");
    assert!(body["result"].is_null());
    assert!(body["field_sha256"].is_null());
    assert_receipt(&text, &body);
    assert_eq!(verdict(&base, &body).await, json!({ "holds": true, "cause": null, "checked": protocol_mcp_gateway::WHOLE_SOLUTION_CHECKED }));
    let mut promoted = body.clone();
    promoted["outcome"] = json!("converged");
    assert_fails(&base, &promoted, "outcome", "a refusal made an answer").await;
}

#[tokio::test]
async fn it_is_listed_and_described_and_its_example_runs_unchanged() {
    let base = spawn_app().await;
    let listed = reqwest::get(format!("{base}/solvers")).await.unwrap();
    let listed: Value = serde_json::from_str(&listed.text().await.unwrap()).unwrap();
    let solvers = listed["solvers"].as_array().expect("solvers");
    assert_eq!(solvers.len(), 13);
    assert_eq!(solvers[11]["solver"], "tokamak_dt_beam_target");
    let summary = solvers[11]["summary"].as_str().unwrap();
    assert!(summary.contains("No constant is fitted"), "{summary}");
    assert!(summary.contains("nothing about the neutron counters"), "{summary}");
    assert!(summary.contains("held-back comparison failed"), "{summary}");

    let (status, text) = post(&base, "/describe_solver", &json!({ "solver": "tokamak_dt_beam_target" })).await;
    assert_eq!(status, 200, "{text}");
    let described: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(described["inputs"].as_array().unwrap().len(), 14);
    assert_eq!(described["refusals"].as_array().unwrap().len(), 29);
    assert_eq!(described["locked"]["quasi_neutrality_allowance"], json!(0.025));
    assert_eq!(described["locked"]["quasi_neutrality_fast_ion_allowance"], json!(0.12));
    assert_eq!(described["locked"]["quasi_neutrality_deficit_allowance"], json!(0.215));
    assert_eq!(described["locked"]["tritium_share_range"], json!([0.00467, 0.998]));
    assert_eq!(described["locked"]["temperature_ratio_range"], json!([0.073, 2.0]));
    assert_eq!(described["locked"]["fast_ion_share_max"], json!(0.439));
    assert_eq!(described["locked"]["settled_after_energy_times"], json!(5.0));
    assert_eq!(described["locked"]["beam_change_fraction"], json!(0.2));
    assert_eq!(described["validation"].as_object().unwrap().len(), 5);
    assert!(described["validation"]["limits"].as_str().unwrap().starts_with("Fitted constants: None"));
    assert_eq!(described["locked"]["cross_sections"]["deuterium_tritium"]["cross_section_range_kev"], json!([0.5, 4700.0]));
    assert_eq!(described["example_solve"], request(&four_shells()));
    let (_, body) = solved(&base, &described["example_solve"]).await;
    assert_eq!(body["outcome"], "converged");
    for (section, count) in [("result", 20), ("invariants", 3)] {
        let named = described["returns"][section].as_array().expect("names");
        assert_eq!(named.len(), count);
        for entry in named {
            let name = entry["name"].as_str().unwrap();
            assert!(!body[section][name].is_null(), "{section}.{name}");
            assert!(!body[section][format!("{name}_hex")].is_null(), "{section}.{name}_hex");
        }
    }
}

#[tokio::test]
async fn an_edited_answer_is_named_by_its_part() {
    let base = spawn_app().await;
    let (_, body) = solved(&base, &request(&four_shells())).await;
    let mut more = body.clone();
    let power = body["result"]["dt_fusion_power_w"].as_f64().unwrap();
    more["result"]["dt_fusion_power_w_hex"] = json!(hex_of(2.0 * power));
    assert_fails(&base, &more, "result", "twice the fusion power").await;
    let mut closed = body.clone();
    closed["invariants"]["energy_ledger_residual_hex"] = json!(hex_of(0.0));
    assert_fails(&base, &closed, "invariants", "a better ledger").await;
    let mut hotter = body.clone();
    hotter["inputs"]["parameters"]["ion_temperature_kev"][0] = json!(25.0);
    assert_fails(&base, &hotter, "inputs", "hotter ions").await;
    let mut other_field = body.clone();
    other_field["field_sha256"] = json!("0".repeat(64));
    assert_fails(&base, &other_field, "field_sha256", "another profile").await;
}
