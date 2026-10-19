//! The solver `ablator_slab` on all three connections: the web routes, the
//! local binary over its pipes and the hosted binary over a socket. A solve
//! is held to a direct call to `genesis_solvers` bit for bit, its proof is
//! written out here by the rule in `REFERENCE.md`, and every fault of a
//! request is held to its sentence.

mod hosted_support;
use hosted_support::answer_text;

use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::time::{Duration, Instant};

use genesis_solvers::ablator_slab::{
    self as slab, SlabQuestion, SlabReject, StreamGas, StreamRow, SurfaceCondition, Wanted,
};
use hosted_support::{Hosted, parse_reply, sha, thermal_step, tool_result};
use protocol_mcp_gateway::mcp_server::tools_in;
use protocol_mcp_gateway::view::View;
use protocol_mcp_gateway::{app, app_with_permits};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

const KEY: &str = "key-that-sees-both-solvers";
const KEY_ATOM: &str = "key-that-sees-the-atom-alone";
const KEY_SLAB: &str = "key-that-sees-the-slab-alone";

const PICA: &str = "PICA, open record, version 5";
const TACOT: &str = "TACOT 3.0";

/// The local binary, one line in and one line out.
struct Local {
    child: Child,
    stdin: ChildStdin,
    out: BufReader<ChildStdout>,
}

impl Local {
    fn start() -> Local {
        let mut child = Command::new(env!("CARGO_BIN_EXE_mcp_stdio"))
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .expect("spawn mcp_stdio");
        let stdin = child.stdin.take().unwrap();
        let out = BufReader::new(child.stdout.take().unwrap());
        let mut local = Local { child, stdin, out };
        let hello = local.line(r#"{"jsonrpc":"2.0","id":0,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"test","version":"1"}}}"#);
        assert_eq!(hello["id"], 0);
        local
            .stdin
            .write_all(b"{\"jsonrpc\":\"2.0\",\"method\":\"notifications/initialized\"}\n")
            .unwrap();
        local
    }

    fn line(&mut self, text: &str) -> Value {
        self.stdin.write_all(text.as_bytes()).unwrap();
        self.stdin.write_all(b"\n").unwrap();
        self.stdin.flush().unwrap();
        let mut answer = String::new();
        self.out.read_line(&mut answer).unwrap();
        serde_json::from_str(&answer).unwrap_or_else(|_| panic!("not JSON: {answer}"))
    }

    /// `(is_error, text)` of a tool answer.
    fn tool(&mut self, tool: &str, arguments: &str) -> (bool, String) {
        let answer = self.line(&tool_line(tool, arguments));
        let text = answer["result"]["content"][0]["text"]
            .as_str()
            .unwrap_or_else(|| panic!("no tool text in {answer}"));
        (answer["result"]["isError"] == json!(true), text.to_string())
    }
}

impl Drop for Local {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn tool_line(tool: &str, arguments: &str) -> String {
    format!(
        r#"{{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{{"name":"{tool}","arguments":{arguments}}}}}"#
    )
}

/// One request to a web route: the status and the body. A long run has as
/// long as it needs.
fn web(port: u16, method: &str, path: &str, body: &str, seconds: u64) -> (u16, String) {
    let request = format!(
        "{method} {path} HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{body}",
        body.len()
    );
    let mut stream = TcpStream::connect(("127.0.0.1", port)).unwrap();
    stream.set_read_timeout(Some(Duration::from_secs(seconds))).unwrap();
    stream.write_all(request.as_bytes()).unwrap();
    let mut out = Vec::new();
    let _ = stream.read_to_end(&mut out);
    let reply = parse_reply(&out);
    (reply.status, reply.body)
}

/// The web connection alone, with the pool of physics permits given.
fn serve(runtime: &tokio::runtime::Runtime, permits: Option<usize>) -> u16 {
    let listener = runtime
        .block_on(tokio::net::TcpListener::bind("127.0.0.1:0"))
        .unwrap();
    let port = listener.local_addr().unwrap().port();
    let router = match permits {
        Some(permits) => app_with_permits(permits, None),
        None => app(),
    };
    runtime.spawn(async move { axum::serve(listener, router).await.unwrap() });
    port
}

/// The three connections. The first hosted key sees every solver, as the
/// web and the local connections do.
struct Connections {
    _runtime: tokio::runtime::Runtime,
    web: u16,
    local: Local,
    hosted: Hosted,
}

impl Connections {
    fn open() -> Connections {
        let runtime = tokio::runtime::Runtime::new().unwrap();
        let web = serve(&runtime, None);
        let entry = |label: &str, key: &str, solvers: Value| {
            json!({
                "client": label, "key_sha256": sha(key), "cards": [],
                "solvers": solvers, "max_in_flight": 8, "max_permits": 1
            })
        };
        let hosted = Hosted::start(
            vec![
                entry("all", KEY, json!(["atom_dft", "ablator_slab", "journal_bearing_sheffield", "propeller_apc_10x7e", "modulus_bam_s355", "seawater_column", "argon_pair_gas", "fusion_pb11_lhd", "fusion_pb11_thermal", "step_flow_armaly", "culture_trichoderma_ambr250", "tokamak_dt_beam_target", "pipe_flow_superpipe", "fuel_rod_triga_jsi", "culture_trichoderma_atp", "junction_nist_b7_s2", "water_hammer_oil_line", "orifice_air_chamber", "pneumatic_hose_resonance", "blast_wave_tnt_air", "stress_plate_hole_s235", "stress_plate_hole_s235_elastic", "icf_hotspot_burn", "ln2_line_dlr_fttf", "plasma_sheath_ram_c2"])),
                entry("atom", KEY_ATOM, json!(["atom_dft"])),
                entry("slab", KEY_SLAB, json!(["ablator_slab"])),
            ],
            None,
        );
        Connections {
            _runtime: runtime,
            web,
            local: Local::start(),
            hosted,
        }
    }

    fn web(&self, method: &str, path: &str, body: &str) -> (u16, String) {
        web(self.web, method, path, body, 300)
    }

    fn hosted(&self, key: &str, tool: &str, arguments: &str) -> (bool, String) {
        let reply = self.hosted.post(key, &tool_line(tool, arguments));
        assert_eq!(reply.status, 200, "{reply:?}");
        tool_result(&reply)
    }

    /// One tool on all three connections. The answer must be the same bytes
    /// on each, and an error on all or on none.
    fn all(&mut self, path: &str, tool: &str, body: &str) -> (u16, String) {
        let (status, web) = if tool == "list_solvers" {
            self.web("GET", path, "")
        } else {
            self.web("POST", path, body)
        };
        let (local_error, local) = self.local.tool(tool, body);
        let (hosted_error, hosted) = self.hosted(KEY, tool, body);
        assert_eq!(web, local, "web and local: {tool}");
        assert_eq!(web, hosted, "web and hosted: {tool}");
        assert_eq!(local_error, status != 200, "{tool}: {web}");
        assert_eq!(hosted_error, status != 200, "{tool}: {web}");
        answer_text::remember(&web);
        (status, web)
    }

    fn solved(&mut self, request: &str) -> (String, Value) {
        let (status, text) = self.all("/solve", "solve", request);
        assert_eq!(status, 200, "{text}");
        let body = serde_json::from_str(&text).expect("a solution");
        (text, body)
    }

    fn solved_on_the_web(&self, request: &str) -> (String, Value) {
        let (status, text) = self.web("POST", "/solve", request);
        assert_eq!(status, 200, "{text}");
        answer_text::remember(&text);
        let body = serde_json::from_str(&text).expect("a solution");
        (text, body)
    }

    /// The long form of an answer sent as an agent sends it back: an answer
    /// the engine gave, as it came.
    fn verdict(&mut self, body: &Value) -> Value {
        let (status, text) = self.all("/verify_solution", "verify_solution", &answer_text::as_sent(body));
        assert_eq!(status, 200, "{text}");
        serde_json::from_str(&text).expect("a report")
    }

    fn assert_holds(&mut self, body: &Value) {
        assert_eq!(self.verdict(body), json!({ "holds": true, "values_hold": true, "cause": null, "compared": protocol_mcp_gateway::WHOLE_SOLUTION_COMPUTED }));
    }

    fn assert_fails(&mut self, body: &Value, part: &str, label: &str) {
        assert_eq!(
            self.verdict(body),
            json!({ "holds": false, "values_hold": false, "cause": part, "compared": protocol_mcp_gateway::WHOLE_SOLUTION_COMPARED }),
            "{label}"
        );
    }

    /// One part of the description, alone, on all three connections.
    fn part(&mut self, part: &str) -> Value {
        let request = json!({ "solver": "ablator_slab", "part": part }).to_string();
        let (status, text) = self.all("/describe_solver", "describe_solver", &request);
        assert_eq!(status, 200, "{text}");
        let body: Value = serde_json::from_str(&text).expect("a part");
        assert_eq!(body.as_object().unwrap().len(), 3, "{part}");
        assert_eq!(body["solver"], "ablator_slab");
        assert_eq!(body["part"], part);
        body[part].clone()
    }

    fn described(&mut self) -> Value {
        let (status, text) = self.all(
            "/describe_solver",
            "describe_solver",
            r#"{"solver":"ablator_slab"}"#,
        );
        assert_eq!(status, 200, "{text}");
        serde_json::from_str(&text).expect("a description")
    }
}

fn sha256_hex(bytes: &[u8]) -> String {
    hex::encode(Sha256::digest(bytes))
}

fn hex_of(value: f64) -> String {
    format!("{:016x}", value.to_bits())
}

/// The text of a request as `REFERENCE.md` gives it, written here apart from
/// the gateway's code: members sorted by key, a number as `x` and its bits.
fn sealed_text(value: &Value) -> String {
    match value {
        Value::Null => "null".to_string(),
        Value::Bool(flag) => flag.to_string(),
        Value::Number(number) => format!("x{}", hex_of(number.as_f64().expect("a number"))),
        Value::String(text) => serde_json::to_string(text).unwrap(),
        Value::Array(items) => {
            let items: Vec<String> = items.iter().map(sealed_text).collect();
            format!("[{}]", items.join(","))
        }
        Value::Object(map) => {
            let mut members: Vec<(&String, &Value)> = map.iter().collect();
            members.sort_by(|left, right| left.0.as_bytes().cmp(right.0.as_bytes()));
            let members: Vec<String> = members
                .iter()
                .map(|(key, value)| {
                    format!("{}:{}", serde_json::to_string(key).unwrap(), sealed_text(value))
                })
                .collect();
            format!("{{{}}}", members.join(","))
        }
    }
}

/// A hex as it is, and an array in brackets with commas between its items.
fn hex_text(value: &Value) -> String {
    match value {
        Value::String(hex) => hex.clone(),
        Value::Array(items) => {
            let items: Vec<String> = items.iter().map(hex_text).collect();
            format!("[{}]", items.join(","))
        }
        other => panic!("not a hex or an array of them: {other}"),
    }
}

/// One line for every `_hex` key of a section, sorted by key.
fn hex_lines(section: &str, object: &Value) -> String {
    let object = object.as_object().expect("a section");
    let mut keys: Vec<&String> = object.keys().filter(|key| key.ends_with("_hex")).collect();
    keys.sort();
    keys.iter()
        .map(|key| format!("{section}.{key}:{}\n", hex_text(&object[key.as_str()])))
        .collect()
}

/// The hex is the solver's bits, and the decimal beside it reads back to them.
fn assert_number(section: &Value, key: &str, expected: f64) {
    assert_eq!(section[format!("{key}_hex")], hex_of(expected), "{key}");
    let decimal = section[key].as_f64().unwrap_or_else(|| panic!("{key}"));
    assert_eq!(decimal.to_bits(), expected.to_bits(), "{key}");
}

fn assert_numbers(section: &Value, key: &str, expected: &[f64]) {
    let hexes: Vec<String> = expected.iter().map(|value| hex_of(*value)).collect();
    assert_eq!(section[format!("{key}_hex")], json!(hexes), "{key}");
    let decimals = section[key].as_array().unwrap_or_else(|| panic!("{key}"));
    assert_eq!(decimals.len(), expected.len(), "{key}");
    for (decimal, value) in decimals.iter().zip(expected) {
        assert_eq!(decimal.as_f64().unwrap().to_bits(), value.to_bits(), "{key}");
    }
}

fn assert_lists(section: &Value, key: &str, expected: &[Vec<f64>]) {
    let hexes: Vec<Vec<String>> = expected
        .iter()
        .map(|list| list.iter().map(|value| hex_of(*value)).collect())
        .collect();
    assert_eq!(section[format!("{key}_hex")], json!(hexes), "{key}");
    let lists = section[key].as_array().unwrap_or_else(|| panic!("{key}"));
    assert_eq!(lists.len(), expected.len(), "{key}");
    for (list, values) in lists.iter().zip(expected) {
        let list = list.as_array().expect("a list");
        assert_eq!(list.len(), values.len(), "{key}");
        for (decimal, value) in list.iter().zip(values) {
            assert_eq!(decimal.as_f64().unwrap().to_bits(), value.to_bits(), "{key}");
        }
    }
}

fn yes_or_no(yes: bool) -> f64 {
    if yes { 1.0 } else { 0.0 }
}

/// Every number of a solution against the solver's own, bit for bit, and no
/// name beside them. `direct` was asked for everything; `wanted` is what the
/// request asked for; `stream_gas` is the stream's gas, or nothing for a
/// held face.
fn assert_matches_the_solver(
    body: &Value,
    direct: &slab::Answer,
    stream_gas: Option<StreamGas>,
    wanted: &Wanted,
) {
    let cold_wall_j_per_kg = match stream_gas {
        None => 0.0,
        Some(gas) => slab::stream_enthalpy_at_cold_wall_j_per_kg(&gas).expect("a gas in range"),
    };
    assert_eq!(body["outcome"], "converged");
    assert!(body["reject"].is_null());
    assert!(body["reject_detail"].is_null());
    let s = &direct.summary;
    assert_eq!(body["iterations"], s.time_steps);
    let result = &body["result"];
    let numbers: [(&str, f64); 30] = [
        (
            "material_compared_with_measurement",
            yes_or_no(s.material_compared_with_measurement),
        ),
        ("duration_s", s.duration_s),
        ("slab_worn_away", yes_or_no(s.worn_away)),
        ("wear_m", s.wear_m),
        ("peak_face_temperature_k", s.peak_face_temperature_k),
        ("peak_face_temperature_at_s", s.peak_face_temperature_at_s),
        ("final_face_temperature_k", s.final_face_temperature_k),
        ("final_back_temperature_k", s.final_back_temperature_k),
        ("depth_half_way_to_virgin_m", s.depth_half_way_to_virgin_m),
        (
            "depth_within_2_percent_of_virgin_m",
            s.depth_within_2_percent_of_virgin_m,
        ),
        ("heat_from_stream_j_per_m2", s.heat_from_stream_j_per_m2),
        (
            "heat_absorbed_from_radiation_j_per_m2",
            s.heat_absorbed_from_radiation_j_per_m2,
        ),
        ("heat_radiated_away_j_per_m2", s.heat_radiated_away_j_per_m2),
        (
            "heat_taken_at_face_by_gases_j_per_m2",
            s.heat_taken_at_face_by_gases_j_per_m2,
        ),
        ("heat_in_through_face_j_per_m2", s.heat_in_through_face_j_per_m2),
        ("heat_stored_in_solid_j_per_m2", s.heat_stored_in_solid_j_per_m2),
        ("heat_taken_by_gas_j_per_m2", s.heat_taken_by_gas_j_per_m2),
        (
            "heat_given_off_by_decomposition_j_per_m2",
            s.heat_given_off_by_decomposition_j_per_m2,
        ),
        ("char_enthalpy_removed_j_per_m2", s.char_enthalpy_removed_j_per_m2),
        ("solid_mass_lost_kg_per_m2", s.solid_mass_lost_kg_per_m2),
        ("gas_mass_out_kg_per_m2", s.gas_mass_out_kg_per_m2),
        ("char_mass_removed_kg_per_m2", s.char_mass_removed_kg_per_m2),
        (
            "inside_arc_jet_comparison",
            yes_or_no(s.inside_arc_jet_comparison),
        ),
        (
            "inside_stardust_comparison",
            yes_or_no(s.inside_stardust_comparison),
        ),
        (
            "inside_thermocouple_block_comparison",
            yes_or_no(s.inside_thermocouple_block_comparison),
        ),
        ("stream_enthalpy_at_cold_wall_j_per_kg", cold_wall_j_per_kg),
        ("cells", f64::from(s.cells)),
        ("cells_at_end", f64::from(s.cells_at_end)),
        ("time_steps", f64::from(s.time_steps)),
        ("hardest_step_newton_passes", f64::from(s.hardest_step_newton_passes)),
    ];
    for (key, expected) in numbers {
        assert_number(result, key, expected);
    }
    let mut names = numbers.len();
    let face = direct.face_history.as_ref().expect("asked directly");
    let face_keys = [
        ("face_history_time_s", &face.time_s),
        ("face_history_temperature_k", &face.face_temperature_k),
        ("face_history_recession_m", &face.recession_m),
        ("face_history_char_front_depth_m", &face.char_front_depth_m),
        ("face_history_virgin_front_depth_m", &face.virgin_front_depth_m),
        (
            "face_history_gas_mass_flux_kg_per_m2_s",
            &face.gas_mass_flux_kg_per_m2_s,
        ),
        (
            "face_history_char_mass_flux_kg_per_m2_s",
            &face.char_mass_flux_kg_per_m2_s,
        ),
    ];
    for (key, expected) in face_keys {
        if wanted.face_history {
            assert_numbers(result, key, expected);
            names += 1;
        } else {
            assert!(result.get(key).is_none(), "{key}");
            assert!(result.get(format!("{key}_hex")).is_none(), "{key}");
        }
    }
    let thermocouples = direct.thermocouple_histories.as_ref().expect("asked directly");
    if wanted.thermocouple_histories {
        let column = |pick: fn(&slab::ThermocoupleHistory) -> f64| -> Vec<f64> {
            thermocouples.iter().map(pick).collect()
        };
        assert_numbers(result, "thermocouple_depth_m", &column(|t| t.depth_m));
        assert_numbers(
            result,
            "thermocouple_passed_by_face",
            &column(|t| if t.passed_by_face { 1.0 } else { 0.0 }),
        );
        assert_numbers(
            result,
            "thermocouple_covered_until_s",
            &column(|t| t.covered_until_s),
        );
        let times: Vec<Vec<f64>> = thermocouples.iter().map(|t| t.time_s.clone()).collect();
        let kelvin: Vec<Vec<f64>> = thermocouples.iter().map(|t| t.temperature_k.clone()).collect();
        assert_lists(result, "thermocouple_time_s", &times);
        assert_lists(result, "thermocouple_temperature_k", &kelvin);
        names += 5;
    } else {
        for key in [
            "thermocouple_depth_m",
            "thermocouple_passed_by_face",
            "thermocouple_covered_until_s",
            "thermocouple_time_s",
            "thermocouple_temperature_k",
        ] {
            assert!(result.get(key).is_none(), "{key}");
            assert!(result.get(format!("{key}_hex")).is_none(), "{key}");
        }
    }
    let profile = direct.final_profile.as_ref().expect("asked directly");
    let profile_keys = [
        ("final_profile_depth_m", &profile.depth_m),
        ("final_profile_temperature_k", &profile.temperature_k),
        ("final_profile_density_kg_per_m3", &profile.density_kg_per_m3),
    ];
    for (key, expected) in profile_keys {
        if wanted.final_profile {
            assert_numbers(result, key, expected);
            names += 1;
        } else {
            assert!(result.get(key).is_none(), "{key}");
            assert!(result.get(format!("{key}_hex")).is_none(), "{key}");
        }
    }
    assert_eq!(
        result.as_object().unwrap().len(),
        2 * names,
        "a value and its hex for each name, and nothing else"
    );
    let invariants = &body["invariants"];
    for (key, expected) in [
        (
            "energy_account_residual_j_per_m2",
            s.energy_account_residual_j_per_m2,
        ),
        (
            "energy_account_tolerance_j_per_m2",
            s.energy_account_tolerance_j_per_m2,
        ),
        ("face_balance_residual_j_per_m2", s.face_balance_residual_j_per_m2),
        (
            "face_balance_tolerance_j_per_m2",
            s.face_balance_tolerance_j_per_m2,
        ),
        (
            "mass_account_residual_kg_per_m2",
            s.mass_account_residual_kg_per_m2,
        ),
        (
            "mass_account_tolerance_kg_per_m2",
            s.mass_account_tolerance_kg_per_m2,
        ),
    ] {
        assert_number(invariants, key, expected);
    }
    assert_eq!(invariants.as_object().unwrap().len(), 2 * 6);

    // The field hash covers the temperatures of the final profile, whether
    // or not the profile was asked for.
    let mut field = (profile.temperature_k.len() as u64).to_le_bytes().to_vec();
    for kelvin in &profile.temperature_k {
        field.extend_from_slice(&kelvin.to_le_bytes());
    }
    assert_eq!(body["field_sha256"], sha256_hex(&field));
}

/// The proof of a solution, rebuilt from the response alone by the rule in
/// `REFERENCE.md`.
fn assert_proof(text: &str, body: &Value) {
    let request = sealed_text(&body["inputs"]);
    let invariants = hex_lines("invariants", &body["invariants"]);
    let result = hex_lines("result", &body["result"]);
    let field = body["field_sha256"].as_str().expect("a field hash");
    // When the profile was returned, the field hash is its temperatures.
    if let Some(points) = body["result"]["final_profile_temperature_k_hex"].as_array() {
        let mut bytes = (points.len() as u64).to_le_bytes().to_vec();
        for point in points {
            let bits = u64::from_str_radix(point.as_str().expect("a hex"), 16).expect("16 hex digits");
            bytes.extend_from_slice(&bits.to_le_bytes());
        }
        assert_eq!(field, sha256_hex(&bytes));
    }
    let tail = format!(
        r#"{{"outcome":"converged","iterations":{},"reject":null}}"#,
        body["iterations"]
    );
    let version = format!("engine_version:{}\n", body["engine_version"].as_str().expect("a version"));
    assert_eq!(
        body["seal_parts"],
        json!({
            "inputs": sha256_hex(request.as_bytes()),
            "engine_version": sha256_hex(version.as_bytes()),
            "result": sha256_hex(result.as_bytes()),
            "invariants": sha256_hex(invariants.as_bytes()),
            "field_sha256": sha256_hex(field.as_bytes()),
            "outcome": sha256_hex(tail.as_bytes()),
        })
    );
    let mut seal = Vec::new();
    seal.extend_from_slice(request.as_bytes());
    seal.extend_from_slice(version.as_bytes());
    seal.extend_from_slice(invariants.as_bytes());
    seal.extend_from_slice(result.as_bytes());
    seal.extend_from_slice(field.as_bytes());
    seal.extend_from_slice(tail.as_bytes());
    assert_eq!(body["solution_seal"], sha256_hex(&seal));
    // The receipt covers every byte before it.
    let (before, receipt) = text
        .rsplit_once(",\"receipt_sha256\":\"")
        .expect("a receipt at the end");
    assert_eq!(receipt.trim_end_matches("\"}"), sha256_hex(format!("{before}}}").as_bytes()));
}

/// A short question under a gas stream: 6 cm of PICA version 5, the
/// thickness of the arc-jet samples, 4 s of an arc-jet stream inside the
/// envelope compared with measurement, three thermocouples of which the
/// face passes the first. Each field is the text the request carries.
const SHORT_STREAM: [(&str, &str); 16] = [
    ("material", "\"PICA, open record, version 5\""),
    ("thickness_m", "0.06"),
    ("start_temperature_k", "300.0"),
    ("thermocouple_depths_m", "[0.0002,0.002,0.02]"),
    ("face", "\"gas_stream\""),
    ("stream_gas", "\"air\""),
    ("history_time_s", "[0,0.5,4.0]"),
    ("history_pressure_pa", "[20000,20000,20000]"),
    ("history_heat_transfer_coefficient_kg_per_m2_s", "[0,0.17,0.17]"),
    ("history_flow_enthalpy_j_per_kg", "[2.5e7,2.5e7,2.5e7]"),
    ("history_radiative_heating_w_per_m2", "[0,0,0]"),
    ("surroundings_temperature_k", "300.0"),
    ("face_wears", "true"),
    ("return_face_history", "false"),
    ("return_thermocouple_histories", "false"),
    ("return_final_profile", "false"),
];

fn short_stream_question() -> SlabQuestion {
    let row = |time_s: f64, coefficient: f64| StreamRow {
        time_s,
        pressure_pa: 20_000.0,
        heat_transfer_coefficient_kg_per_m2_s: coefficient,
        flow_enthalpy_j_per_kg: 2.5e7,
        radiative_heating_w_per_m2: 0.0,
    };
    SlabQuestion {
        thickness_m: 0.06,
        start_temperature_k: 300.0,
        surface: SurfaceCondition::GasStream {
            history: vec![row(0.0, 0.0), row(0.5, 0.17), row(4.0, 0.17)],
            surroundings_temperature_k: 300.0,
            face_wears: true,
            stream_gas: StreamGas::AIR,
        },
        continue_after_s: 0.0,
        thermocouple_depths_m: vec![0.0002, 0.002, 0.02],
    }
}

/// A short question with a held face: 1 cm of TACOT held at 1644 K for 2 s.
const SHORT_HELD: [(&str, &str); 11] = [
    ("material", "\"TACOT 3.0\""),
    ("thickness_m", "0.01"),
    ("start_temperature_k", "298.0"),
    ("thermocouple_depths_m", "[0.001]"),
    ("face", "\"held_temperature\""),
    ("held_temperature_k", "1644.0"),
    ("hold_duration_s", "2.0"),
    ("continue_after_s", "0.0"),
    ("return_face_history", "false"),
    ("return_thermocouple_histories", "false"),
    ("return_final_profile", "false"),
];

/// The text of a request: the base with each named field replaced by the
/// text given, added when it is not a field of the base, or left out.
fn request_text(base: &[(&str, &str)], changes: &[(&str, Option<&str>)]) -> String {
    let mut fields: Vec<(String, String)> = base
        .iter()
        .map(|(key, text)| (key.to_string(), text.to_string()))
        .collect();
    for (key, change) in changes {
        let place = fields.iter().position(|(name, _)| name == key);
        match (place, change) {
            (Some(index), Some(text)) => fields[index].1 = text.to_string(),
            (Some(index), None) => {
                fields.remove(index);
            }
            (None, Some(text)) => fields.push((key.to_string(), text.to_string())),
            (None, None) => panic!("{key} is not a field of the base"),
        }
    }
    let parameters: Vec<String> = fields
        .iter()
        .map(|(key, text)| format!("\"{key}\":{text}"))
        .collect();
    format!(
        "{{\"solver\":\"ablator_slab\",\"parameters\":{{{}}}}}",
        parameters.join(",")
    )
}

fn stream_with(changes: &[(&str, &str)]) -> String {
    let changes: Vec<(&str, Option<&str>)> =
        changes.iter().map(|(key, text)| (*key, Some(*text))).collect();
    request_text(&SHORT_STREAM, &changes)
}

/// The short stream as a mixture of the oxygen and argon given, with any
/// further changes.
fn mixture_with(oxygen: &str, argon: &str, changes: &[(&str, &str)]) -> String {
    let mut all = vec![
        ("stream_gas", "\"mixture\""),
        ("stream_oxygen_mass_fraction", oxygen),
        ("stream_argon_mass_fraction", argon),
    ];
    all.extend_from_slice(changes);
    stream_with(&all)
}

fn held_with(changes: &[(&str, &str)]) -> String {
    let changes: Vec<(&str, Option<&str>)> =
        changes.iter().map(|(key, text)| (*key, Some(*text))).collect();
    request_text(&SHORT_HELD, &changes)
}

#[test]
fn the_list_shows_thirteen_solvers_and_the_description_publishes_fields_bounds_refusals_and_the_record() {
    let mut connections = Connections::open();
    let (status, text) = connections.all("/solvers", "list_solvers", "{}");
    assert_eq!(status, 200, "{text}");
    let listed: Value = serde_json::from_str(&text).unwrap();
    let solvers = listed["solvers"].as_array().expect("solvers");
    assert_eq!(solvers.len(), 25);
    assert_eq!(solvers[0]["solver"], "atom_dft");
    assert_eq!(solvers[1]["solver"], "ablator_slab");
    assert_eq!(solvers[3]["solver"], "propeller_apc_10x7e");
    assert_eq!(solvers[6]["solver"], "argon_pair_gas");
    let summary = solvers[1]["summary"].as_str().expect("a sentence");
    assert!(summary.starts_with("A flat slab of charring heat-shield material"), "{summary}");
    assert!(summary.ends_with('.'));

    let described = connections.described();
    assert_eq!(described["solver"], "ablator_slab");
    assert_eq!(described["summary"], summary);
    assert_eq!(described["engine_version"], "genesis_solvers_v0.12.0");
    let keys: Vec<&String> = described.as_object().unwrap().keys().collect();
    assert_eq!(
        keys,
        [
            "solver",
            "summary",
            "engine_version",
            "validation",
            "inputs",
            "number_rule",
            "open_connection",
            "example_solve",
            "example_solve_arc_jet_test",
            "more",
        ]
    );
    // The description a caller reads first is short enough to read.
    let (_, text) = connections.all(
        "/describe_solver",
        "describe_solver",
        r#"{"solver":"ablator_slab"}"#,
    );
    eprintln!("the description: {} characters, {} bytes", text.chars().count(), text.len());
    // The envelopes stated since engine 0.7.0, with the ranges the numerics
    // were measured on and each input's compared range since the walls at
    // their edges came down, and the gateway's number rule, the same for
    // every solver, which adds about 470 characters; since 0.13.1 the record
    // also says what was seen before the lines were fixed; since 0.14.0 rc2
    // its open_connection says how many of its worked solve one inline answer
    // holds (rc2 item 6).
    assert!(text.chars().count() < 21_000, "{}", text.chars().count());
    // What it keeps for asking, each said in a sentence.
    let more = described["more"].as_object().expect("more");
    assert_eq!(
        more.keys().collect::<Vec<_>>(),
        [
            "how",
            "locked",
            "returns",
            "invariants",
            "convergence",
            "refusals",
            "reference",
            "example_solve_stardust_near_nose",
        ]
    );
    assert_eq!(
        more["how"],
        "describe_solver with 'solver' and 'part' set to a name below returns that part alone."
    );
    let locked = connections.part("locked");
    let returns = connections.part("returns");
    let refusals = connections.part("refusals");
    for part in ["invariants", "convergence", "reference"] {
        assert!(!connections.part(part).is_null(), "{part}");
    }
    // A part of the description itself can be asked for alone too.
    assert_eq!(connections.part("validation"), described["validation"]);
    assert_eq!(connections.part("inputs"), described["inputs"]);
    // A part no description has is refused with the names it has.
    for wrong in ["\"stardust\"", "\"more\"", "\"solver\"", "\"\""] {
        let request = format!(r#"{{"solver":"ablator_slab","part":{wrong}}}"#);
        let (status, text) = connections.all("/describe_solver", "describe_solver", &request);
        assert_eq!(status, 422, "{text}");
        assert_eq!(
            text,
            format!(
                "Physical Violation: 'part' must be one of \"validation\", \"inputs\", \"number_rule\", \"locked\", \"returns\", \"invariants\", \"convergence\", \"open_connection\", \"refusals\", \"reference\", \"example_solve\", \"example_solve_arc_jet_test\", \"example_solve_stardust_near_nose\"; this request has {wrong}."
            )
        );
    }
    let (status, text) = connections.all(
        "/describe_solver",
        "describe_solver",
        r#"{"solver":"ablator_slab","part":3}"#,
    );
    assert_eq!(status, 422);
    assert_eq!(text, "Physical Violation: 'part' must be a string; this request has 3.");
    // The atom has every part of its own description and no other.
    let (status, text) = connections.all(
        "/describe_solver",
        "describe_solver",
        r#"{"solver":"atom_dft","part":"reference"}"#,
    );
    assert_eq!(status, 200, "{text}");
    assert!(text.starts_with(r#"{"solver":"atom_dft","part":"reference","reference":{"name":"#), "{text}");
    let (status, text) = connections.all(
        "/describe_solver",
        "describe_solver",
        r#"{"solver":"atom_dft","part":"validation"}"#,
    );
    assert_eq!(status, 422, "{text}");
    assert!(text.starts_with("Physical Violation: 'part' must be one of \"inputs\", \"number_rule\", \"locked\""), "{text}");

    // Each field with the bounds of the solver's own constants.
    let inputs = described["inputs"].as_array().expect("inputs");
    assert_eq!(inputs.len(), 21);
    let input = |name: &str| {
        inputs
            .iter()
            .find(|row| row["name"] == name)
            .unwrap_or_else(|| panic!("{name}"))
    };
    assert_eq!(
        input("material")["allowed"],
        json!(slab::MATERIAL_NAMES)
    );
    let bounds = |name: &str| {
        (
            input(name)["minimum"].as_f64().unwrap(),
            input(name)["maximum"].as_f64().unwrap(),
            input(name)["refusal"].as_str().unwrap().to_string(),
        )
    };
    let named = |low: f64, high: f64, refusal: &str| (low, high, refusal.to_string());
    assert_eq!(
        bounds("thickness_m"),
        named(slab::THICKNESS_MIN_M, slab::THICKNESS_MAX_M, "ThicknessOutOfRange")
    );
    assert_eq!(
        bounds("start_temperature_k"),
        named(
            slab::START_TEMPERATURE_MIN_K,
            slab::START_TEMPERATURE_MAX_K,
            "StartTemperatureOutOfRange"
        )
    );
    assert_eq!(
        bounds("held_temperature_k"),
        named(
            slab::HELD_TEMPERATURE_MIN_K,
            slab::HELD_TEMPERATURE_MAX_K,
            "HeldTemperatureOutOfRange"
        )
    );
    assert_eq!(
        bounds("hold_duration_s"),
        named(slab::HOLD_DURATION_MIN_S, slab::HOLD_DURATION_MAX_S, "HoldDurationOutOfRange")
    );
    assert_eq!(
        bounds("continue_after_s"),
        named(0.0, slab::CONTINUE_AFTER_MAX_S, "ContinueAfterOutOfRange")
    );
    assert_eq!(
        bounds("thermocouple_depths_m"),
        named(0.0, slab::THICKNESS_MAX_M, "ThermocoupleOutsideSlab")
    );
    assert_eq!(input("thermocouple_depths_m")["most"], slab::THERMOCOUPLES_MAX);
    assert_eq!(
        bounds("history_time_s"),
        named(0.0, slab::HISTORY_DURATION_MAX_S, "HistoryDurationOutOfRange")
    );
    assert_eq!(
        bounds("history_pressure_pa"),
        named(
            slab::PICA_STREAM_PRESSURE_MIN_PA,
            slab::STREAM_PRESSURE_MAX_PA,
            "StreamPressureOutOfRange"
        )
    );
    assert_eq!(
        bounds("history_heat_transfer_coefficient_kg_per_m2_s"),
        named(
            0.0,
            slab::HEAT_TRANSFER_COEFFICIENT_MAX_KG_PER_M2_S,
            "HeatTransferCoefficientOutOfRange"
        )
    );
    assert_eq!(
        bounds("history_flow_enthalpy_j_per_kg"),
        named(0.0, slab::FLOW_ENTHALPY_MAX_J_PER_KG, "FlowEnthalpyOutOfRange")
    );
    assert_eq!(
        bounds("history_radiative_heating_w_per_m2"),
        named(0.0, slab::RADIATIVE_HEATING_MAX_W_PER_M2, "RadiativeHeatingOutOfRange")
    );
    assert_eq!(
        bounds("surroundings_temperature_k"),
        named(
            slab::SURROUNDINGS_TEMPERATURE_MIN_K,
            genesis_solvers::pica::TABLE_TOP_K,
            "SurroundingsTemperatureOutOfRange"
        )
    );
    for column in [
        "history_time_s",
        "history_pressure_pa",
        "history_heat_transfer_coefficient_kg_per_m2_s",
        "history_flow_enthalpy_j_per_kg",
        "history_radiative_heating_w_per_m2",
    ] {
        assert_eq!(input(column)["fewest"], slab::HISTORY_ROWS_MIN, "{column}");
        assert_eq!(input(column)["most"], slab::HISTORY_ROWS_MAX, "{column}");
        assert_eq!(input(column)["given_only_when"], "'face' is \"gas_stream\"");
    }
    for held in ["held_temperature_k", "hold_duration_s", "continue_after_s"] {
        assert_eq!(input(held)["given_only_when"], "'face' is \"held_temperature\"");
    }
    for flag in [
        "face_wears",
        "return_face_history",
        "return_thermocouple_histories",
        "return_final_profile",
    ] {
        assert_eq!(input(flag)["type"], "true or false", "{flag}");
    }
    assert_eq!(
        locked["surface_table_pressure_pa"],
        json!({
            "TACOT 3.0": [101.325, 101325.0],
            "PICA, open record, version 5": [1.01325, 101325.0],
            "PICA, open record, version 4": [1.01325, 101325.0],
            "PICA, open record, version 3": [1.01325, 101325.0],
            "PICA, open record": [1.01325, 101325.0]
        })
    );
    assert_eq!(
        locked["compared_with_measurement"]["arc_jet"]["pressure_pa"],
        json!([slab::ARC_JET_PRESSURE_MIN_PA, slab::ARC_JET_PRESSURE_MAX_PA])
    );

    // All 35 refusals by the solver's names, 23 that a request shows and
    // 12 the solver names once it is computing. After the solver's 35 the
    // list holds the two the entry names itself at the door, UnknownFace and
    // UnknownStreamGas.
    let refusals = refusals.as_array().expect("refusals");
    assert_eq!(refusals.len(), 36);
    let at_the_door = refusals
        .iter()
        .filter(|row| row["reported_as"] == "422, no seal")
        .count();
    let sealed = refusals
        .iter()
        .filter(|row| row["reported_as"] == "200, outcome refused, sealed")
        .count();
    assert_eq!((at_the_door, sealed), (25, 11));
    for reject in [
        SlabReject::ThicknessOutOfRange { thickness_m: 1.0 },
        SlabReject::HistoryRowsOutOfRange { rows: 257 },
        SlabReject::HistoryRowsTooClose { row: 1, span_s: 1.0e-300 },
        SlabReject::UnknownMaterial { name: String::new() },
        SlabReject::StreamOxygenNotAMassFraction { oxygen_mass_fraction: 1.5 },
        SlabReject::StreamArgonNotAMassFraction { argon_mass_fraction: -0.6 },
        SlabReject::StreamOxygenAndArgonAboveOne { oxygen_mass_fraction: 0.6, argon_mass_fraction: 0.5 },
        SlabReject::StreamNotAirForThisMaterial { stream_gas: StreamGas::air_with_argon(0.1) },
        SlabReject::SurfaceChemistryNotConverged { pressure_pa: 1.0, blowing: 0.0, temperature_k: 300.0 },
        SlabReject::FaceOutranItsCell { time_s: 1.0 },
        SlabReject::FlowEnthalpyBelowWall { row: 1, flow_enthalpy_j_per_kg: 0.0, wall_enthalpy_j_per_kg: 1.0, wall_temperature_k: 300.0 },
    ] {
        assert!(
            refusals.iter().any(|row| row["name"] == reject.name()),
            "{}",
            reject.name()
        );
    }
    assert_eq!(refusals[0]["name"], "ThicknessOutOfRange");
    assert_eq!(
        refusals[0]["meaning"],
        "`thickness_m` is outside 0.0002 to 1 m. Below 0.2 mm the grid holds no more than 8 cells; 1 m is a size cap on the request."
    );
    assert_eq!(refusals[16]["name"], "HistoryRowsTooClose");
    assert_eq!(refusals[32]["name"], "FaceOutranItsCell");
    assert_eq!(refusals[33]["name"], "FlowEnthalpyBelowWall");
    // No meaning writes a range in program notation.
    for row in refusals {
        assert!(!row["meaning"].as_str().unwrap().contains("..="), "{}", row["name"]);
    }
    let reported = |name: &str| {
        refusals.iter().find(|row| row["name"] == name).expect("listed")["reported_as"].clone()
    };
    assert_eq!(reported("StreamNotAirForThisMaterial"), "422, no seal");
    assert_eq!(reported("HistoryRowsTooClose"), "422, no seal");
    assert_eq!(reported("SurfaceChemistryNotConverged"), "200, outcome refused, sealed");
    assert_eq!(reported("StreamOxygenAndArgonAboveOne"), "422, no seal");
    for gone in [
        "HeldFaceNotCompared",
        "FaceNotWearingNotCompared",
        "ThicknessNotCompared",
        "StartTemperatureNotCompared",
        "SurroundingsNotCompared",
        "QuestionNotCompared",
        "TemperatureAboveCompared",
        "StreamOxygenOutOfRange",
        "StreamArgonOutOfRange",
    ] {
        assert!(!refusals.iter().any(|row| row["name"] == gone), "{gone}");
    }
    // The stream's gas: its two inputs, air to paste, what was compared
    // with measurement, and the rule for an arc-jet test.
    assert_eq!(
        input("stream_gas"),
        // Re-pinned for 0.10.1: a gas that is not one of the two words is refused as UnknownStreamGas, and the row says so.
        &json!({
            "name": "stream_gas", "type": "string", "allowed": ["air", "mixture"], "refusal": "UnknownStreamGas",
            "given_only_when": "'face' is \"gas_stream\"",
            "bound": "What the stream is made of: `air`, with no fraction given, or `mixture`, oxygen and argon by mass as given and the rest nitrogen. `TACOT 3.0` takes `air` alone."
        })
    );
    for fraction in ["stream_oxygen_mass_fraction", "stream_argon_mass_fraction"] {
        assert_eq!(input(fraction)["given_only_when"], "'stream_gas' is \"mixture\"", "{fraction}");
    }
    assert_eq!(
        bounds("stream_oxygen_mass_fraction"),
        named(0.0, 1.0, "StreamOxygenNotAMassFraction")
    );
    assert_eq!(
        bounds("stream_argon_mass_fraction"),
        named(0.0, 1.0, "StreamArgonNotAMassFraction")
    );
    assert_eq!(locked["air"]["oxygen_mass_fraction"], slab::AIR_OXYGEN_MASS_FRACTION);
    assert_eq!(
        locked["air"]["a_mixture_with_airs_numbers"],
        "A `mixture` whose oxygen is exactly this number, with no argon, is air to the solver: it reads the locked table, returns air's numbers and is taken by `TACOT 3.0`. Its request still says `mixture`, so its proof is not that of the same question asked as `air`. Any other number, 0.2329 for one, is another gas."
    );
    assert_eq!(locked["compared_with_measurement"]["arc_jet"]["stream_oxygen_mass_fraction"], json!([0.1, 0.3]));
    assert_eq!(locked["compared_with_measurement"]["arc_jet"]["stream_argon_mass_fraction_max"], 0.276);
    assert_eq!(locked["compared_with_measurement"]["stardust_flight"]["stream_gas"], "air");
    let recipe = connections.part("reference")["arc_jet_test_to_question"].clone();
    assert_eq!(recipe["steps"].as_array().unwrap().len(), 5);
    assert_eq!(
        recipe["written_out_in"],
        "the steps below, which are the whole rule"
    );

    // What comes back, and the record in five parts.
    let result = returns["result"].as_array().expect("result");
    assert_eq!(result.len(), 30 + 15);
    assert_eq!(result[3], json!({
        "name": "wear_m", "unit": "m", "note": "How far the face wore inward. Zero for a held face."
    }));
    assert!(
        returns["hex"]
            .as_str()
            .unwrap()
            .contains("An array has a `{field}_hex` sibling of the same shape")
    );
    let record = described["validation"].as_object().expect("a record");
    assert_eq!(
        record.keys().collect::<Vec<_>>(),
        ["object", "data", "pass_line", "result", "limits"]
    );
    let data = record["data"].as_str().unwrap();
    assert!(
        data.ends_with("Compared with 31 further arc-jet tests, not in the fit and not used to choose the surface rules."),
        "{data}"
    );
    let limits = record["limits"].as_str().unwrap();
    assert!(limits.starts_with("Known gaps. 1. The faint end of the charring is too deep"), "{limits}");
    assert!(limits.contains("9. The decomposition gas inside the slab is read at 1 atm"), "{limits}");
    let object = record["object"].as_str().unwrap();
    assert!(object.contains("`PICA, open record, version 5` is the PICA model a caller is pointed to"));
    assert!(object.contains("0.71 of the measured curve, the one fitted number"));
    assert!(record["result"].as_str().unwrap().contains(
        "wear of 7.58 and 3.32 mm against 5.7 and 3.2 measured, a third high at the nose and 3 percent high on the flank"
    ));

    // The atom is described as before, with no such record, and a key that
    // sees the atom alone is told the slab does not exist.
    let (status, atom) = connections.all("/describe_solver", "describe_solver", r#"{"solver":"atom_dft"}"#);
    assert_eq!(status, 200);
    let atom: Value = serde_json::from_str(&atom).unwrap();
    assert!(atom.get("validation").is_none());
    assert!(atom.get("more").is_none());
    assert_eq!(atom["example_solve"], json!({ "solver": "atom_dft", "parameters": { "atomic_number": 10 } }));
    let (hidden_error, hidden) =
        connections.hosted(KEY_ATOM, "describe_solver", r#"{"solver":"ablator_slab"}"#);
    assert!(hidden_error);
    assert_eq!(hidden, "Solver not recognized (UnknownSolver).");
    let (hidden_error, hidden) = connections.hosted(KEY_ATOM, "solve", &held_with(&[]));
    assert!(hidden_error);
    assert_eq!(hidden, "Solver not recognized (UnknownSolver).");
    let (error, listed) = connections.hosted(KEY_SLAB, "list_solvers", "{}");
    assert!(!error);
    let listed: Value = serde_json::from_str(&listed).unwrap();
    assert_eq!(listed["solvers"].as_array().unwrap().len(), 1);
    assert_eq!(listed["solvers"][0]["solver"], "ablator_slab");
}

/// No tool's schema holds a choice between shapes, with the slab in view
/// alone or beside the atom. The inputs that go with one kind of face are
/// declared, are not in the required list, and the rule is in words.
#[test]
fn the_schema_of_solve_declares_every_input_plainly() {
    let slab_alone = View::limited([], ["ablator_slab".to_string()]);
    let tools = tools_in(&slab_alone);
    let solve = tools.iter().find(|tool| tool.name == "solve").expect("solve");
    let schema = Value::Object(solve.input_schema.as_ref().clone());
    assert_eq!(schema["properties"]["solver"], json!({ "const": "ablator_slab" }));
    let parameters = &schema["properties"]["parameters"];
    assert_eq!(parameters["type"], "object");
    assert_eq!(parameters["additionalProperties"], false);
    // Twenty-one inputs, and signed, an instrument's signature beside them.
    assert_eq!(parameters["properties"].as_object().unwrap().len(), 22);
    assert_eq!(
        parameters["properties"]["stream_gas"],
        json!({ "type": "string", "enum": ["air", "mixture"] })
    );
    assert_eq!(
        parameters["properties"]["stream_oxygen_mass_fraction"],
        json!({ "type": "number", "minimum": 0.0, "maximum": 1.0 })
    );
    assert_eq!(
        parameters["properties"]["stream_argon_mass_fraction"],
        json!({ "type": "number", "minimum": 0.0, "maximum": 1.0 })
    );
    assert_eq!(
        parameters["required"],
        json!([
            "material",
            "thickness_m",
            "start_temperature_k",
            "thermocouple_depths_m",
            "face",
            "return_face_history",
            "return_thermocouple_histories",
            "return_final_profile"
        ])
    );
    assert_eq!(
        parameters["description"],
        // Such a key is refused in a sentence that names the key, as InputNotWithChoice, and the schema says so.
        "Every key in the required list, and when face is held_temperature also held_temperature_k, hold_duration_s, continue_after_s, and when face is gas_stream also stream_gas, history_time_s, history_pressure_pa, history_heat_transfer_coefficient_kg_per_m2_s, history_flow_enthalpy_j_per_kg, history_radiative_heating_w_per_m2, surroundings_temperature_k, face_wears, and when stream_gas is mixture also stream_oxygen_mass_fraction, stream_argon_mass_fraction. A key that goes with the other choice is refused in a sentence that names the key, as InputNotWithChoice. A required input left out is refused as InputMissing, and an input given with a choice it does not go with as InputNotWithChoice, each in a sentence that names the input, and a solver the engine does not serve as UnknownSolver; nothing is filled in. parameters may also carry signed, an instrument's signature over the other parameters under the key \"parameters\", by the rule run_trajectory's input schema gives at segments; it is verified before solving, the solver takes the other parameters, and the answer then carries load_source, naming the key. A solve answer without load_source holds no signed parameter."
    );
    assert_eq!(
        parameters["properties"]["material"],
        json!({
            "type": "string",
            "enum": [
                "TACOT 3.0",
                "PICA, open record, version 5",
                "PICA, open record, version 4",
                "PICA, open record, version 3",
                "PICA, open record"
            ]
        })
    );
    assert_eq!(
        parameters["properties"]["face"],
        json!({ "type": "string", "enum": ["held_temperature", "gas_stream"] })
    );
    assert_eq!(
        parameters["properties"]["thickness_m"],
        json!({ "type": "number", "minimum": 0.0002, "maximum": 1.0 })
    );
    assert_eq!(
        parameters["properties"]["history_pressure_pa"],
        json!({
            "type": "array",
            "items": { "type": "number", "minimum": 1.01325, "maximum": 101325.0 },
            "minItems": 2,
            "maxItems": 256
        })
    );
    assert_eq!(
        parameters["properties"]["thermocouple_depths_m"],
        json!({
            "type": "array",
            "items": { "type": "number", "minimum": 0.0, "maximum": 1.0 },
            "maxItems": 8
        })
    );
    assert_eq!(parameters["properties"]["face_wears"], json!({ "type": "boolean" }));
    assert_eq!(parameters["properties"]["return_final_profile"], json!({ "type": "boolean" }));

    // All three solvers in view: a list of names, and the inputs of each in words.
    // No solver tool holds a choice between shapes at any depth.
    let full = View::full();
    for view in [&slab_alone, &full] {
        for tool in tools_in(view) {
            if !["list_solvers", "describe_solver", "solve", "verify_solution"]
                .contains(&tool.name.as_ref())
            {
                continue;
            }
            let text = Value::Object(tool.input_schema.as_ref().clone()).to_string();
            for word in ["\"oneOf\"", "\"anyOf\"", "\"allOf\"", "\"if\":", "\"not\":"] {
                assert!(!text.contains(word), "{}: {word}", tool.name);
            }
        }
    }
    let tools = tools_in(&full);
    let solve = tools.iter().find(|tool| tool.name == "solve").expect("solve");
    let schema = Value::Object(solve.input_schema.as_ref().clone());
    // The solvers are left to list_solvers and their inputs to describe_solver.
    assert_eq!(
        schema["properties"]["solver"],
        json!({ "type": "string", "description": "The solver, as list_solvers names it." })
    );
    assert!(!schema["properties"]["parameters"].to_string().contains("thickness_m"));
    // The description takes the solver and, when a part is wanted, its name,
    // and the form of the description.
    let describe = tools.iter().find(|tool| tool.name == "describe_solver").expect("describe_solver");
    let schema = Value::Object(describe.input_schema.as_ref().clone());
    assert_eq!(schema["required"], json!(["solver"]));
    assert_eq!(
        schema["properties"].as_object().unwrap().keys().collect::<Vec<_>>(),
        ["solver", "part", "form"]
    );
    assert_eq!(schema["properties"]["part"]["type"], "string");
    // The atom alone: the schema it had when it was the only solver.
    let atom_alone = View::limited([], ["atom_dft".to_string()]);
    let tools = tools_in(&atom_alone);
    let solve = tools.iter().find(|tool| tool.name == "solve").expect("solve");
    // A signed load: an instrument's signature may stand beside the atom's
    // one field, and the parameters say so. Without them it is the schema
    // the atom had, byte for byte.
    let mut declared = Value::Object(solve.input_schema.as_ref().clone());
    let parameters = declared["properties"]["parameters"].as_object_mut().unwrap();
    assert!(parameters["properties"].as_object_mut().unwrap().shift_remove("signed").is_some());
    let said = parameters["description"].as_str().unwrap().to_string();
    let (before, signed_words) = said.split_once(" parameters may also carry signed").expect("the signed words");
    assert!(signed_words.ends_with("A solve answer without load_source holds no signed parameter."), "{said}");
    parameters["description"] = json!(before);
    // A sealed card run handed in through a coupling into a solver
    // (triga_pulse_to_element_6753) may stand beside them too: without it, the
    // schema the atom had.
    assert!(declared["properties"].as_object_mut().unwrap().shift_remove("from").is_some());
    assert_eq!(
        serde_json::to_string(&declared).unwrap(),
        r#"{"type":"object","properties":{"solver":{"const":"atom_dft"},"parameters":{"type":"object","properties":{"atomic_number":{"type":"integer","minimum":1,"maximum":18}},"required":["atomic_number"],"additionalProperties":false,"description":"A required input left out is refused as InputMissing, and an input given with a choice it does not go with as InputNotWithChoice, each in a sentence that names the input, and a solver the engine does not serve as UnknownSolver; nothing is filled in."},"example":{"type":"string","description":"In place of parameters: the name of one of the solver's worked examples as describe_solver keys it (example_solve, or a key that begins example_solve_). The request is filled with that example's parameters, and the filled request is the one solved, sealed and returned as inputs."}},"required":["solver"],"additionalProperties":false,"description":"One sealed solution."}"#
    );
}

/// The first worked example, run as the description gives it.
#[test]
fn workshop_test_case_1_lands_seals_and_is_the_same_bytes_on_three_connections_and_twice() {
    let mut connections = Connections::open();
    let described = connections.described();
    let example = &described["example_solve"];
    assert_eq!(
        example,
        &json!({
            "solver": "ablator_slab",
            "parameters": {
                "material": "TACOT 3.0",
                "thickness_m": 0.05,
                "start_temperature_k": 298.0,
                "thermocouple_depths_m": [0.001, 0.002, 0.004, 0.008, 0.016, 0.05],
                "face": "held_temperature",
                "held_temperature_k": 1644.0,
                "hold_duration_s": 60.0,
                "continue_after_s": 0.0,
                "return_face_history": false,
                "return_thermocouple_histories": false,
                "return_final_profile": false
            }
        })
    );
    let request = example.to_string();
    let (text, body) = connections.solved(&request);
    assert_eq!(body["inputs"], *example);
    assert_eq!(body["engine_version"], "genesis_solvers_v0.12.0");

    let everything = Wanted {
        face_history: true,
        thermocouple_histories: true,
        final_profile: true,
    };
    let direct = slab::answer(TACOT, &slab::ablation_workshop_test_case_1(), &everything)
        .expect("test case 1 is answered");
    assert_matches_the_solver(&body, &direct, None, &Wanted::default());
    assert_proof(&text, &body);
    connections.assert_holds(&body);

    // What the answer says: a held face does not wear, and TACOT has not
    // been compared with measurement, so it lies in neither envelope.
    let result = &body["result"];
    assert_eq!(result["wear_m"], 0.0);
    assert_eq!(result["material_compared_with_measurement"], 0.0);
    assert_eq!(result["duration_s"], 60.0);
    assert_eq!(result["peak_face_temperature_k"], 1644.0);
    assert_eq!(result["inside_arc_jet_comparison"], 0.0);
    assert_eq!(result["inside_stardust_comparison"], 0.0);
    assert_eq!(result["cells"], 195.0);
    assert_eq!(body["iterations"], result["time_steps"].as_f64().unwrap() as u64);
    // The summary alone is a few thousand characters.
    assert!(text.len() < 6_500, "{}", text.len());

    // The same request again is the same bytes, and so is the same request
    // with its keys in another order and its numbers spelled another way,
    // from the proof on: the answer echoes the request as it was typed.
    let (again, _) = connections.solved_on_the_web(&request);
    assert_eq!(again, text);
    let respelled = r#"{"parameters":{"return_final_profile":false,"return_thermocouple_histories":false,"return_face_history":false,"continue_after_s":0,"hold_duration_s":6e1,"held_temperature_k":1644,"face":"held_temperature","thermocouple_depths_m":[1e-3,0.002,0.004,0.008,0.016,0.05],"start_temperature_k":298,"thickness_m":0.05,"material":"TACOT 3.0"},"solver":"ablator_slab"}"#;
    let (_, other) = connections.solved_on_the_web(respelled);
    for key in ["result", "invariants", "field_sha256", "seal_parts", "solution_seal", "iterations"] {
        assert_eq!(other[key], body[key], "{key}");
    }
    connections.assert_holds(&other);
}

/// The first of the 36 development arc-jet tests, asked as the description
/// gives it: air with 27.6 percent argon, a stream that is not air. It
/// returns the wear and the face temperature the development record holds
/// for it (the first of the record's 36 answers:
/// 0.002004727694553583 m and 2012.4767125432215 K), and the enthalpy of its
/// gas at a cold wall, from which its heat-transfer coefficient is made.
#[test]
fn a_development_arc_jet_test_in_its_own_gas_returns_the_recorded_wear_and_face_temperature() {
    let mut connections = Connections::open();
    let described = connections.described();
    let example = &described["example_solve_arc_jet_test"];
    assert_eq!(
        example,
        &json!({
            "solver": "ablator_slab",
            "parameters": {
                "material": "PICA, open record, version 5",
                "thickness_m": 0.06,
                "start_temperature_k": 300.0,
                "thermocouple_depths_m": [],
                "face": "gas_stream",
                "stream_gas": "mixture",
                "stream_oxygen_mass_fraction": 0.16753359999999998,
                "stream_argon_mass_fraction": 0.276,
                "history_time_s": [0.0, 0.5, 55.0],
                "history_pressure_pa": [2300.0, 2300.0, 2300.0],
                "history_heat_transfer_coefficient_kg_per_m2_s": [0.0, 0.07040224291374243, 0.07040224291374243],
                "history_flow_enthalpy_j_per_kg": [15200000.0, 15200000.0, 15200000.0],
                "history_radiative_heating_w_per_m2": [0.0, 0.0, 0.0],
                "surroundings_temperature_k": 300.0,
                "face_wears": true,
                "return_face_history": false,
                "return_thermocouple_histories": false,
                "return_final_profile": false
            }
        })
    );
    let (text, body) = connections.solved(&example.to_string());
    assert_eq!(body["inputs"], *example);
    let result = &body["result"];
    let wear_m = result["wear_m"].as_f64().unwrap();
    let peak_k = result["peak_face_temperature_k"].as_f64().unwrap();
    eprintln!("the arc-jet test: wear {wear_m} m, peak face {peak_k} K, {} characters", text.len());
    assert!((wear_m / 0.002004727694553583 - 1.0).abs() < 1.0e-12, "{wear_m}");
    assert!((peak_k - 2012.4767125432215).abs() < 1.0e-9, "{peak_k}");
    // The measurements were 2.33 mm and 2,033 K.
    assert_eq!((wear_m * 1.0e6).round(), 2005.0);
    assert_eq!(peak_k.round(), 2012.0);

    // Bit for bit what the solver gives when asked directly.
    let gas = StreamGas::air_with_argon(0.276);
    assert_eq!(gas.oxygen_mass_fraction, 0.16753359999999998);
    let row = |time_s: f64, coefficient: f64| StreamRow {
        time_s,
        pressure_pa: 2300.0,
        heat_transfer_coefficient_kg_per_m2_s: coefficient,
        flow_enthalpy_j_per_kg: 15.2e6,
        radiative_heating_w_per_m2: 0.0,
    };
    let question = SlabQuestion {
        thickness_m: 0.06,
        start_temperature_k: 300.0,
        surface: SurfaceCondition::GasStream {
            history: vec![
                row(0.0, 0.0),
                row(0.5, 0.07040224291374243),
                row(55.0, 0.07040224291374243),
            ],
            surroundings_temperature_k: 300.0,
            face_wears: true,
            stream_gas: gas,
        },
        continue_after_s: 0.0,
        thermocouple_depths_m: Vec::new(),
    };
    let everything = Wanted {
        face_history: true,
        thermocouple_histories: true,
        final_profile: true,
    };
    let direct = slab::answer(PICA, &question, &everything).expect("the test is answered");
    assert_matches_the_solver(&body, &direct, Some(gas), &Wanted::default());
    assert_proof(&text, &body);
    connections.assert_holds(&body);

    // The enthalpy of the gas at a cold wall is in the answer, and by the
    // recipe it gives the coefficient the question was asked with: the
    // cold-wall heating of 107 W/cm² over the flow enthalpy less it.
    let cold_wall = result["stream_enthalpy_at_cold_wall_j_per_kg"].as_f64().unwrap();
    assert_eq!((cold_wall * 1000.0).round(), 1_620_577.0, "{cold_wall}");
    assert_eq!(107.0e4 / (15.2e6 - cold_wall), 0.07040224291374243);
    // The stream is inside the arc-jet envelope, at its lowest heating, and
    // not in the Stardust one: its gas is not air.
    assert_eq!(result["inside_arc_jet_comparison"], 1.0);
    assert_eq!(result["inside_stardust_comparison"], 0.0);
    // The gas is in the proof: another gas is another request.
    let mut less_argon = body.clone();
    less_argon["inputs"]["parameters"]["stream_argon_mass_fraction"] = json!(0.0);
    connections.assert_fails(&less_argon, "inputs", "the stream's gas altered");
    let mut as_air = body.clone();
    as_air["inputs"]["parameters"]["stream_gas"] = json!("air");
    connections.assert_fails(&as_air, "inputs", "the stream's gas renamed");

    // A caller gets the enthalpy for a gas from a question of one second at
    // a test's own conditions.
    let mut short = json!({
        "solver": "ablator_slab",
        "parameters": {
            "material": "PICA, open record, version 5",
            "thickness_m": 0.06,
            "start_temperature_k": 300.0,
            "thermocouple_depths_m": [],
            "face": "gas_stream",
            "stream_gas": "mixture",
            "stream_oxygen_mass_fraction": 0.25,
            "stream_argon_mass_fraction": 0.1,
            "history_time_s": [0.0, 1.0],
            "history_pressure_pa": [20000.0, 20000.0],
            "history_heat_transfer_coefficient_kg_per_m2_s": [0.2, 0.2],
            "history_flow_enthalpy_j_per_kg": [1.0e7, 1.0e7],
            "history_radiative_heating_w_per_m2": [0.0, 0.0],
            "surroundings_temperature_k": 300.0,
            "face_wears": true,
            "return_face_history": false,
            "return_thermocouple_histories": false,
            "return_final_profile": false
        }
    });
    let (_, body) = connections.solved_on_the_web(&short.to_string());
    let other = StreamGas { oxygen_mass_fraction: 0.25, argon_mass_fraction: 0.1 };
    assert_eq!(
        body["result"]["stream_enthalpy_at_cold_wall_j_per_kg"],
        slab::stream_enthalpy_at_cold_wall_j_per_kg(&other).expect("in range")
    );
    assert_eq!(body["result"]["inside_arc_jet_comparison"], 1.0);
    // A stream whose gas no test ran in is answered, sealed, in no
    // envelope; so are pure nitrogen and oxygen with argon and no nitrogen.
    for (oxygen, argon) in [(0.35, 0.0), (0.0, 0.0), (0.6, 0.4)] {
        short["parameters"]["stream_oxygen_mass_fraction"] = json!(oxygen);
        short["parameters"]["stream_argon_mass_fraction"] = json!(argon);
        let (_, body) = connections.solved_on_the_web(&short.to_string());
        assert_eq!(body["outcome"], "converged", "{oxygen} {argon}: {body}");
        for flag in ["inside_arc_jet_comparison", "inside_stardust_comparison", "inside_thermocouple_block_comparison"] {
            assert_eq!(body["result"][flag], 0.0, "{oxygen} {argon}: {flag}");
        }
        connections.assert_holds(&body);
    }
    // More than the whole stream is refused at the door by name.
    short["parameters"]["stream_oxygen_mass_fraction"] = json!(0.7);
    short["parameters"]["stream_argon_mass_fraction"] = json!(0.4);
    let (status, text) = connections.all("/solve", "solve", &short.to_string());
    assert_eq!(status, 422, "{text}");
    assert_eq!(
        text,
        "Physical Violation: the stream's oxygen mass fraction 0.7 and argon mass fraction 0.4 add to more than 1, the whole stream (StreamOxygenAndArgonAboveOne)."
    );
}

/// A slow request for the web connection, sent from its own thread.
fn in_flight(port: u16, request: String) -> std::thread::JoinHandle<(u16, String)> {
    std::thread::spawn(move || web(port, "POST", "/solve", &request, 900))
}

fn step(port: u16) -> (u16, String) {
    web(port, "POST", "/verify_physics", &thermal_step().to_string(), 60)
}

/// The second worked example, run as the description gives it: the Stardust
/// entry at the near-nose core on the served PICA model. It is the longest
/// question the description holds, about a minute in a debug build, so it is
/// also the run that is watched for what it holds while it computes: one
/// physics permit and nothing else.
#[test]
fn the_stardust_example_lands_with_the_recorded_wear_and_holds_one_permit_while_it_runs() {
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let port = serve(&runtime, Some(1));
    // The description says where the question is, and the part holds it.
    let (status, described) = web(port, "POST", "/describe_solver", r#"{"solver":"ablator_slab"}"#, 60);
    assert_eq!(status, 200);
    let described: Value = serde_json::from_str(&described).unwrap();
    assert_eq!(
        described["more"]["example_solve_stardust_near_nose"],
        "The Stardust near-nose question on `PICA, open record, version 5` in full, in air, 136 rows of NASA's bare-wall heating, as a request that runs unchanged. It returns the 7.58 mm of `validation.result`."
    );
    let (status, part) = web(
        port,
        "POST",
        "/describe_solver",
        r#"{"solver":"ablator_slab","part":"example_solve_stardust_near_nose"}"#,
        60,
    );
    assert_eq!(status, 200);
    let part: Value = serde_json::from_str(&part).unwrap();
    let example = part["example_solve_stardust_near_nose"].clone();
    let parameters = &example["parameters"];
    assert_eq!(parameters["material"], PICA);
    assert_eq!(parameters["history_time_s"].as_array().unwrap().len(), 136);
    let request = example.to_string();

    // Before the run the one permit is free.
    assert_eq!(step(port).0, 200);
    let started = Instant::now();
    let run = in_flight(port, request.clone());
    // The run takes the permit as soon as its request is read. From then on
    // another physics request is turned away at once, and nothing waits.
    let mut turned_away = None;
    while started.elapsed() < Duration::from_secs(30) && !run.is_finished() {
        let asked = Instant::now();
        let (status, text) = step(port);
        if status == 429 {
            turned_away = Some((text, asked.elapsed()));
            break;
        }
        assert_eq!(status, 200, "{text}");
    }
    let (text, waited) = turned_away.expect("the run never held the permit");
    assert!(text.contains("Too many concurrent physics tasks."), "{text}");
    assert!(!text.contains("sha256_seal"), "{text}");
    assert!(waited < Duration::from_secs(5), "a request waited {waited:?}");
    // Everything that takes no permit is answered while the run computes.
    let (status, _) = web(port, "GET", "/health", "", 60);
    assert_eq!(status, 200);
    let (status, listed) = web(port, "GET", "/solvers", "", 60);
    assert_eq!(status, 200);
    assert!(listed.contains("ablator_slab"));
    let (status, _) = web(port, "POST", "/describe_solver", r#"{"solver":"atom_dft"}"#, 60);
    assert_eq!(status, 200);
    // A request the slab's own limits refuse is refused while the run
    // computes: a limit the request shows is asked before a permit.
    let (status, refused) = web(port, "POST", "/solve", &held_with(&[("thickness_m", "1.5")]), 60);
    assert_eq!(status, 422);
    assert_eq!(
        refused,
        "Physical Violation: thickness 1.5 m is outside 0.0002 to 1 m: below 0.0002 m the grid holds no more than 8 cells, and 1 m is a size cap on the request (ThicknessOutOfRange)."
    );
    // A second slab is turned away like any other physics request.
    let (status, second) = web(port, "POST", "/solve", &held_with(&[]), 60);
    assert_eq!(status, 429, "{second}");
    assert!(second.contains("Too many concurrent physics tasks."), "{second}");
    let still_running = !run.is_finished();

    let (status, text) = run.join().expect("the run returned");
    let seconds = started.elapsed().as_secs_f64();
    assert_eq!(status, 200, "{text}");
    let body: Value = serde_json::from_str(&text).expect("a solution");
    assert_eq!(body["outcome"], "converged");
    assert_eq!(body["inputs"], example);
    // The permit is back.
    assert_eq!(step(port).0, 200);
    eprintln!(
        "the Stardust example: {seconds:.1} s, {} characters, of which the request is {}; still running after the checks: {still_running}",
        text.len(),
        request.len()
    );

    // The wear the record holds for version five with the solver's own
    // corrections: that answer of the Stardust model run for the near-nose
    // core ends at 0.007575342631352343 m, which the solver's header gives
    // as 7.58 mm, a third above the 5.7 mm measured.
    let result = &body["result"];
    let wear_m = result["wear_m"].as_f64().unwrap();
    assert!((wear_m - 0.007575342631352343).abs() < 1.0e-9, "{wear_m}");
    assert_eq!((wear_m * 1.0e5).round(), 758.0);
    let peak_k = result["peak_face_temperature_k"].as_f64().unwrap();
    assert_eq!(peak_k.round(), 3412.0, "{peak_k}");
    assert_eq!(result["peak_face_temperature_at_s"], 51.0);
    assert_eq!(result["duration_s"], 600.0);
    assert_eq!(result["material_compared_with_measurement"], 1.0);
    // The two depths of the charring as the header gives them for version
    // five, read at 600 s: 7.4 mm half way and 12.0 mm at the faint end,
    // which is the first known miss.
    let half_way_m = result["depth_half_way_to_virgin_m"].as_f64().unwrap();
    assert_eq!((half_way_m * 1.0e4).round(), 74.0, "{half_way_m}");
    let outer_m = result["depth_within_2_percent_of_virgin_m"].as_f64().unwrap();
    assert_eq!((outer_m * 1.0e4).round(), 120.0, "{outer_m}");
    // The answer says which envelope compared with measurement it lies in:
    // the Stardust flight's, not the arc jet's (thin air at first, high
    // enthalpy, radiative heating and a cooling phase). Its hottest step is
    // the top of what was compared, so it is answered.
    assert_eq!(result["inside_stardust_comparison"], 1.0);
    assert_eq!(result["inside_arc_jet_comparison"], 0.0);
    assert!(peak_k <= slab::COMPARED_TEMPERATURE_MAX_K);
    // The accounts closed inside what they were allowed.
    let invariants = &body["invariants"];
    for (residual, tolerance) in [
        ("energy_account_residual_j_per_m2", "energy_account_tolerance_j_per_m2"),
        ("face_balance_residual_j_per_m2", "face_balance_tolerance_j_per_m2"),
        ("mass_account_residual_kg_per_m2", "mass_account_tolerance_kg_per_m2"),
    ] {
        let open = invariants[residual].as_f64().unwrap().abs();
        let allowed = invariants[tolerance].as_f64().unwrap();
        assert!(open <= allowed, "{residual}: {open} against {allowed}");
    }
    // The stream is air, stated in the request: inside what was compared
    // with measurement, and the enthalpy of air at a cold wall beside it.
    assert_eq!(parameters["stream_gas"], "air");
    assert!(parameters.get("stream_oxygen_mass_fraction").is_none());
    assert_eq!(
        result["stream_enthalpy_at_cold_wall_j_per_kg"],
        slab::stream_enthalpy_at_cold_wall_j_per_kg(&StreamGas::AIR).expect("air")
    );
    // Summary only: 29 numbers and 6, no array, and the answer beyond the
    // echoed request is a few thousand characters.
    assert_eq!(result.as_object().unwrap().len(), 2 * 30);
    assert!(result.as_object().unwrap().values().all(|value| !value.is_array()));
    assert_eq!(invariants.as_object().unwrap().len(), 2 * 6);
    assert!(text.len() - request.len() < 6_500, "{}", text.len() - request.len());
    assert!(text.len() < 13_500, "{}", text.len());

    assert_proof(&text, &body);
    let (status, verdict) = web(port, "POST", "/verify_solution", &text, 60);
    assert_eq!(status, 200);
    assert_eq!(verdict, format!(r#"{{"holds":true,"values_hold":true,"cause":null,"compared":{}}}"#, serde_json::json!(protocol_mcp_gateway::WHOLE_SOLUTION_COMPUTED)));
    let mut worn_less = body.clone();
    worn_less["result"]["wear_m_hex"] = json!(hex_of(0.0057));
    let (_, verdict) = web(port, "POST", "/verify_solution", &worn_less.to_string(), 60);
    assert_eq!(verdict, format!(r#"{{"holds":false,"values_hold":false,"cause":"result","compared":{}}}"#, serde_json::json!(protocol_mcp_gateway::WHOLE_SOLUTION_COMPARED)));
}

/// With two permits, a slab that is computing leaves the other free the
/// whole time: it holds one permit, not the pool.
#[test]
fn a_slab_that_is_computing_leaves_the_other_permit_free() {
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let port = serve(&runtime, Some(2));
    let example = json!({
        "solver": "ablator_slab",
        "parameters": {
            "material": "TACOT 3.0",
            "thickness_m": 0.05,
            "start_temperature_k": 298.0,
            "thermocouple_depths_m": [],
            "face": "held_temperature",
            "held_temperature_k": 1644.0,
            "hold_duration_s": 60.0,
            "continue_after_s": 0.0,
            "return_face_history": false,
            "return_thermocouple_histories": false,
            "return_final_profile": false
        }
    });
    let run = in_flight(port, example.to_string());
    let mut answered = 0;
    while !run.is_finished() {
        let (status, text) = step(port);
        assert_eq!(status, 200, "a step was turned away while one slab ran: {text}");
        answered += 1;
    }
    let (status, text) = run.join().expect("the run returned");
    assert_eq!(status, 200, "{text}");
    assert!(answered >= 1, "no step was asked while the slab ran");
}

/// Histories and the final profile come back when asked for by name, each
/// under its own flag, and every element of them is in the proof.
#[test]
fn histories_come_back_when_asked_for_and_every_element_is_sealed() {
    let mut connections = Connections::open();
    let everything = Wanted {
        face_history: true,
        thermocouple_histories: true,
        final_profile: true,
    };
    let direct = slab::answer(PICA, &short_stream_question(), &everything)
        .expect("the short stream is answered");

    // Everything asked for, on all three connections.
    let all = stream_with(&[
        ("return_face_history", "true"),
        ("return_thermocouple_histories", "true"),
        ("return_final_profile", "true"),
    ]);
    let (text, body) = connections.solved(&all);
    assert_matches_the_solver(&body, &direct, Some(StreamGas::AIR), &everything);
    assert_proof(&text, &body);
    connections.assert_holds(&body);
    let result = &body["result"];
    // The face passed the first thermocouple, and its record ends there.
    assert_eq!(result["thermocouple_passed_by_face"], json!([1.0, 0.0, 0.0]));
    let lengths: Vec<usize> = result["thermocouple_time_s"]
        .as_array()
        .unwrap()
        .iter()
        .map(|record| record.as_array().unwrap().len())
        .collect();
    assert_eq!(lengths[1], 5);
    assert_eq!(lengths[2], 5);
    assert!(lengths[0] < 5, "{lengths:?}");
    assert_eq!(result["face_history_time_s"], json!([0.0, 1.0, 2.0, 3.0, 4.0]));
    let profile = result["final_profile_depth_m"].as_array().unwrap();
    assert!(profile.len() <= slab::PROFILE_POINTS_MAX && profile.len() > 100);
    assert!(result["wear_m"].as_f64().unwrap() > 0.0002);

    // Nothing asked for: the summary alone. Each flag alone: its own arrays
    // and no others. The summary's numbers are the same every time.
    let (summary_text, summary) = connections.solved_on_the_web(&stream_with(&[]));
    assert_matches_the_solver(&summary, &direct, Some(StreamGas::AIR), &Wanted::default());
    assert_proof(&summary_text, &summary);
    assert!(summary["result"].as_object().unwrap().values().all(|value| !value.is_array()));
    assert_eq!(summary["field_sha256"], body["field_sha256"]);
    assert_eq!(summary["invariants"], body["invariants"]);
    assert_ne!(summary["solution_seal"], body["solution_seal"]);
    for (flag, wanted) in [
        (
            "return_face_history",
            Wanted { face_history: true, ..Wanted::default() },
        ),
        (
            "return_thermocouple_histories",
            Wanted { thermocouple_histories: true, ..Wanted::default() },
        ),
        (
            "return_final_profile",
            Wanted { final_profile: true, ..Wanted::default() },
        ),
    ] {
        let (text, one) = connections.solved_on_the_web(&stream_with(&[(flag, "true")]));
        assert_matches_the_solver(&one, &direct, Some(StreamGas::AIR), &wanted);
        assert_proof(&text, &one);
    }
    eprintln!(
        "the short stream: {} characters with the summary alone, {} with everything",
        summary_text.len(),
        text.len()
    );

    // A mixture with exactly air's numbers is air to the solver: the same
    // values, number for number, under another request and so another proof.
    let (_, as_mixture) = connections.solved_on_the_web(&mixture_with("0.23291751145757963", "0", &[]));
    assert_eq!(as_mixture["inputs"]["parameters"]["stream_gas"], "mixture");
    assert_eq!(as_mixture["result"], summary["result"]);
    assert_eq!(as_mixture["invariants"], summary["invariants"]);
    assert_eq!(as_mixture["seal_parts"]["result"], summary["seal_parts"]["result"]);
    assert_ne!(as_mixture["seal_parts"]["inputs"], summary["seal_parts"]["inputs"]);
    assert_ne!(as_mixture["solution_seal"], summary["solution_seal"]);
    // And a mixture that is nearly air is another gas, with other numbers.
    let (_, nearly_air) = connections.solved_on_the_web(&mixture_with("0.2329", "0", &[]));
    assert_ne!(nearly_air["result"]["wear_m"], summary["result"]["wear_m"]);
    let ratio = nearly_air["result"]["wear_m"].as_f64().unwrap() / summary["result"]["wear_m"].as_f64().unwrap();
    assert!((ratio - 1.0).abs() < 0.01, "{ratio}");

    // One element changed, removed, added or swapped, in an array and in an
    // array of arrays, in the hex the proof covers.
    let hexes = |body: &Value, key: &str| body["result"][key].as_array().unwrap().clone();
    let flat = "face_history_temperature_k_hex";
    let nested = "thermocouple_temperature_k_hex";
    let mut changed = body.clone();
    changed["result"][flat][2] = json!(hex_of(1234.5));
    connections.assert_fails(&changed, "result", "an element changed");
    let mut removed = body.clone();
    removed["result"][flat].as_array_mut().unwrap().pop();
    connections.assert_fails(&removed, "result", "the last element removed");
    let mut removed = body.clone();
    removed["result"][flat].as_array_mut().unwrap().remove(0);
    connections.assert_fails(&removed, "result", "the first element removed");
    let mut added = body.clone();
    added["result"][flat].as_array_mut().unwrap().push(json!(hex_of(300.0)));
    connections.assert_fails(&added, "result", "an element added");
    let mut swapped = body.clone();
    let items = hexes(&body, flat);
    assert_ne!(items[1], items[3]);
    swapped["result"][flat][1] = items[3].clone();
    swapped["result"][flat][3] = items[1].clone();
    connections.assert_fails(&swapped, "result", "two elements swapped");

    let mut changed = body.clone();
    changed["result"][nested][1][2] = json!(hex_of(1234.5));
    connections.assert_fails(&changed, "result", "an element of a record changed");
    let mut removed = body.clone();
    removed["result"][nested][2].as_array_mut().unwrap().pop();
    connections.assert_fails(&removed, "result", "an element of a record removed");
    let mut added = body.clone();
    added["result"][nested][0].as_array_mut().unwrap().push(json!(hex_of(300.0)));
    connections.assert_fails(&added, "result", "an element added to a record");
    let mut swapped = body.clone();
    let records = hexes(&body, nested);
    assert_ne!(records[1], records[2]);
    swapped["result"][nested][1] = records[2].clone();
    swapped["result"][nested][2] = records[1].clone();
    connections.assert_fails(&swapped, "result", "two records swapped");
    let mut moved = body.clone();
    let carried = moved["result"][nested][1].as_array_mut().unwrap().pop().unwrap();
    moved["result"][nested][2].as_array_mut().unwrap().push(carried);
    connections.assert_fails(&moved, "result", "an element moved to the next record");
    let mut emptied = body.clone();
    emptied["result"][nested] = json!([[], [], []]);
    connections.assert_fails(&emptied, "result", "the records emptied");

    // A whole history taken out of a solution, or put into one that did not
    // ask for it, is another proof.
    let mut taken_out = body.clone();
    let object = taken_out["result"].as_object_mut().unwrap();
    object.remove("face_history_recession_m");
    object.remove("face_history_recession_m_hex");
    connections.assert_fails(&taken_out, "result", "a history taken out");
    let mut put_in = summary.clone();
    put_in["result"]["face_history_recession_m"] = body["result"]["face_history_recession_m"].clone();
    put_in["result"]["face_history_recession_m_hex"] =
        body["result"]["face_history_recession_m_hex"].clone();
    connections.assert_fails(&put_in, "result", "a history put in");
    // The request is in the proof too: what was asked for cannot be altered.
    let mut asked_less = body.clone();
    asked_less["inputs"]["parameters"]["return_face_history"] = json!(false);
    connections.assert_fails(&asked_less, "inputs", "the request altered");
    let mut another_row = body.clone();
    another_row["inputs"]["parameters"]["history_flow_enthalpy_j_per_kg"][2] = json!(2.4e7);
    connections.assert_fails(&another_row, "inputs", "a row of the heating history altered");
    // A value that is not a hex, and a name the solver does not publish, are
    // refused before the proof is recomputed.
    let mut not_hex = body.clone();
    not_hex["result"][flat][0] = json!(300.0);
    let (status, message) = connections.all("/verify_solution", "verify_solution", &not_hex.to_string());
    assert_eq!(status, 422);
    assert_eq!(
        message,
        "Physical Violation: 'result.face_history_temperature_k_hex' must be an array, and each item 16 lowercase hex digits."
    );
    let mut unpublished = body.clone();
    unpublished["result"]["face_history_pressure_pa_hex"] = json!([hex_of(1.0)]);
    let (status, message) =
        connections.all("/verify_solution", "verify_solution", &unpublished.to_string());
    assert_eq!(status, 422);
    assert_eq!(
        message,
        "Physical Violation: 'result.face_history_pressure_pa_hex' is not published by this solver."
    );
    // Untouched, it still holds.
    connections.assert_holds(&body);
}

fn numbers_text(count: usize, value: impl Fn(usize) -> f64) -> String {
    let items: Vec<String> = (0..count).map(|index| format!("{:?}", value(index))).collect();
    format!("[{}]", items.join(","))
}

/// A heating history of `rows` rows, one second apart, as the five columns.
fn long_history(rows: usize) -> Vec<(&'static str, String)> {
    vec![
        ("history_time_s", numbers_text(rows, |index| index as f64)),
        ("history_pressure_pa", numbers_text(rows, |_| 20_000.0)),
        (
            "history_heat_transfer_coefficient_kg_per_m2_s",
            numbers_text(rows, |_| 0.0),
        ),
        ("history_flow_enthalpy_j_per_kg", numbers_text(rows, |_| 0.0)),
        ("history_radiative_heating_w_per_m2", numbers_text(rows, |_| 0.0)),
    ]
}

#[test]
fn each_fault_of_a_request_is_named_and_carries_no_proof() {
    let mut connections = Connections::open();
    let violation = |sentence: &str| format!("Physical Violation: {sentence}");
    let rows_257 = long_history(257);
    let rows_257: Vec<(&str, &str)> = rows_257.iter().map(|(key, text)| (*key, text.as_str())).collect();
    let nine_depths = "[0.001,0.002,0.003,0.004,0.005,0.006,0.007,0.008,0.009]";

    // Every limit a request can show, in the solver's own sentence and under
    // the solver's own name. `SlabReject`'s sentence is the source of each.
    let shown: Vec<(String, SlabReject)> = vec![
        (
            held_with(&[("thickness_m", "1.5")]),
            SlabReject::ThicknessOutOfRange { thickness_m: 1.5 },
        ),
        (
            held_with(&[("thickness_m", "0.00019")]),
            SlabReject::ThicknessOutOfRange { thickness_m: 0.00019 },
        ),
        (
            stream_with(&[("start_temperature_k", "253")]),
            SlabReject::StartTemperatureOutOfRange { start_temperature_k: 253.0 },
        ),
        (
            held_with(&[("held_temperature_k", "3301")]),
            SlabReject::HeldTemperatureOutOfRange { held_temperature_k: 3301.0 },
        ),
        (
            held_with(&[("hold_duration_s", "0.5")]),
            SlabReject::HoldDurationOutOfRange { hold_duration_s: 0.5 },
        ),
        (
            held_with(&[("continue_after_s", "3600.5")]),
            SlabReject::ContinueAfterOutOfRange { continue_after_s: 3600.5 },
        ),
        (
            held_with(&[("continue_after_s", "-1")]),
            SlabReject::ContinueAfterOutOfRange { continue_after_s: -1.0 },
        ),
        (
            held_with(&[("thermocouple_depths_m", nine_depths)]),
            SlabReject::TooManyThermocouples { count: 9 },
        ),
        (
            held_with(&[("thermocouple_depths_m", "[0.001,0.011]")]),
            SlabReject::ThermocoupleOutsideSlab { index: 1, depth_m: 0.011 },
        ),
        (
            stream_with(&[("thermocouple_depths_m", "[-0.001]")]),
            SlabReject::ThermocoupleOutsideSlab { index: 0, depth_m: -0.001 },
        ),
        (
            stream_with(&rows_257),
            SlabReject::HistoryRowsOutOfRange { rows: 257 },
        ),
        (
            stream_with(&[
                ("history_time_s", "[0]"),
                ("history_pressure_pa", "[20000]"),
                ("history_heat_transfer_coefficient_kg_per_m2_s", "[0]"),
                ("history_flow_enthalpy_j_per_kg", "[0]"),
                ("history_radiative_heating_w_per_m2", "[0]"),
            ]),
            SlabReject::HistoryRowsOutOfRange { rows: 1 },
        ),
        (
            stream_with(&[("history_time_s", "[0,4.0,0.5]")]),
            SlabReject::HistoryTimesOutOfOrder { row: 2, time_s: 0.5 },
        ),
        (
            stream_with(&[("history_time_s", "[0.25,0.5,4.0]")]),
            SlabReject::HistoryTimesOutOfOrder { row: 0, time_s: 0.25 },
        ),
        (
            stream_with(&[("history_time_s", "[0,0.5,0.5]")]),
            SlabReject::HistoryTimesOutOfOrder { row: 2, time_s: 0.5 },
        ),
        (
            stream_with(&[("history_time_s", "[0,0.5,3600.5]")]),
            SlabReject::HistoryDurationOutOfRange { duration_s: 3600.5 },
        ),
        (
            stream_with(&[("history_time_s", "[0,0.5,0.75]")]),
            SlabReject::HistoryDurationOutOfRange { duration_s: 0.75 },
        ),
        (
            stream_with(&[
                ("material", "\"TACOT 3.0\""),
                ("history_pressure_pa", "[20000,20000,101326]"),
            ]),
            SlabReject::StreamPressureOutOfRange {
                row: 2,
                pressure_pa: 101_326.0,
                lowest_pressure_pa: 101.325,
                highest_pressure_pa: 101_325.0,
            },
        ),
        (
            stream_with(&[("history_heat_transfer_coefficient_kg_per_m2_s", "[0,-1.5,0.17]")]),
            SlabReject::HeatTransferCoefficientOutOfRange {
                row: 1,
                heat_transfer_coefficient_kg_per_m2_s: -1.5,
            },
        ),
        (
            stream_with(&[("history_flow_enthalpy_j_per_kg", "[2.5e7,2.5e7,-1.5e8]")]),
            SlabReject::FlowEnthalpyOutOfRange { row: 2, flow_enthalpy_j_per_kg: -1.5e8 },
        ),
        (
            stream_with(&[("history_radiative_heating_w_per_m2", "[-1,0,0]")]),
            SlabReject::RadiativeHeatingOutOfRange { row: 0, radiative_heating_w_per_m2: -1.0 },
        ),
        (
            stream_with(&[("surroundings_temperature_k", "4000")]),
            SlabReject::SurroundingsTemperatureOutOfRange { surroundings_temperature_k: 4000.0, highest_k: 3975.0 },
        ),
        (
            stream_with(&[("material", "\"TACOT 3.0\""), ("surroundings_temperature_k", "3400")]),
            SlabReject::SurroundingsTemperatureOutOfRange {
                surroundings_temperature_k: 3400.0,
                highest_k: slab::SURROUNDINGS_TEMPERATURE_MAX_K,
            },
        ),
        (
            mixture_with("-0.04", "0", &[]),
            SlabReject::StreamOxygenNotAMassFraction { oxygen_mass_fraction: -0.04 },
        ),
        (
            mixture_with("1.01", "0", &[]),
            SlabReject::StreamOxygenNotAMassFraction { oxygen_mass_fraction: 1.01 },
        ),
        (
            mixture_with("0.2", "1.6", &[]),
            SlabReject::StreamArgonNotAMassFraction { argon_mass_fraction: 1.6 },
        ),
        (
            mixture_with("0.2", "-0.01", &[]),
            SlabReject::StreamArgonNotAMassFraction { argon_mass_fraction: -0.01 },
        ),
        (
            mixture_with("0.6", "0.41", &[]),
            SlabReject::StreamOxygenAndArgonAboveOne { oxygen_mass_fraction: 0.6, argon_mass_fraction: 0.41 },
        ),
        // TACOT's table is for air alone, and a mixture that is nearly air
        // is not air.
        (
            mixture_with("0.2", "0.1", &[("material", "\"TACOT 3.0\"")]),
            SlabReject::StreamNotAirForThisMaterial {
                stream_gas: StreamGas { oxygen_mass_fraction: 0.2, argon_mass_fraction: 0.1 },
            },
        ),
        (
            mixture_with("0.2329", "0", &[("material", "\"TACOT 3.0\"")]),
            SlabReject::StreamNotAirForThisMaterial {
                stream_gas: StreamGas { oxygen_mass_fraction: 0.2329, argon_mass_fraction: 0.0 },
            },
        ),
        // The stream's gas is asked before the heating history.
        (
            mixture_with("1.5", "0", &[("history_time_s", "[0,4.0,0.5]")]),
            SlabReject::StreamOxygenNotAMassFraction { oxygen_mass_fraction: 1.5 },
        ),
        // Two limits at once are named for the one the solver asks first: a
        // row of the history before the surroundings, the thickness before
        // everything, an earlier row before a later one.
        (
            stream_with(&[
                ("surroundings_temperature_k", "4000"),
                ("history_flow_enthalpy_j_per_kg", "[2.5e7,2.5e7,-1.5e8]"),
            ]),
            SlabReject::FlowEnthalpyOutOfRange { row: 2, flow_enthalpy_j_per_kg: -1.5e8 },
        ),
        (
            stream_with(&[
                ("thickness_m", "1.2"),
                ("history_time_s", "[0,4.0,0.5]"),
                ("thermocouple_depths_m", nine_depths),
            ]),
            SlabReject::ThicknessOutOfRange { thickness_m: 1.2 },
        ),
        (
            stream_with(&[
                ("history_pressure_pa", "[20000,20000,101326]"),
                ("history_radiative_heating_w_per_m2", "[0,-2e7,0]"),
            ]),
            SlabReject::RadiativeHeatingOutOfRange { row: 1, radiative_heating_w_per_m2: -2.0e7 },
        ),
        (
            stream_with(&[
                ("history_time_s", "[0,0.5,3700]"),
                ("thermocouple_depths_m", "[0.5]"),
            ]),
            SlabReject::HistoryDurationOutOfRange { duration_s: 3700.0 },
        ),
    ];
    let mut names = std::collections::BTreeSet::new();
    let mut faults: Vec<(String, u16, String)> = shown
        .into_iter()
        .map(|(request, reject)| {
            names.insert(reject.name());
            (request, 422, violation(&format!("{reject} ({}).", reject.name())))
        })
        .collect();
    assert_eq!(
        names.into_iter().collect::<Vec<_>>(),
        [
            "ContinueAfterOutOfRange",
            "FlowEnthalpyOutOfRange",
            "HeatTransferCoefficientOutOfRange",
            "HeldTemperatureOutOfRange",
            "HistoryDurationOutOfRange",
            "HistoryRowsOutOfRange",
            "HistoryTimesOutOfOrder",
            "HoldDurationOutOfRange",
            "RadiativeHeatingOutOfRange",
            "StartTemperatureOutOfRange",
            "StreamArgonNotAMassFraction",
            "StreamNotAirForThisMaterial",
            "StreamOxygenAndArgonAboveOne",
            "StreamOxygenNotAMassFraction",
            "StreamPressureOutOfRange",
            "SurroundingsTemperatureOutOfRange",
            "ThermocoupleOutsideSlab",
            "ThicknessOutOfRange",
            "TooManyThermocouples",
        ]
    );

    let mut fault = |request: String, status: u16, message: &str| {
        faults.push((request, status, message.to_string()));
    };
    // The sentences written out, so that a reader sees what an agent reads.
    fault(
        stream_with(&rows_257),
        422,
        "Physical Violation: the heating history has 257 rows; it may have 2 to 256 (HistoryRowsOutOfRange).",
    );
    fault(
        stream_with(&[("history_time_s", "[0,4.0,0.5]")]),
        422,
        "Physical Violation: row 2 of the heating history is at 0.5 s; the first row is at zero and each row is later than the one before (HistoryTimesOutOfOrder).",
    );
    fault(
        stream_with(&[("history_time_s", "[0,1e-300,10]")]),
        422,
        "Physical Violation: row 1 of the heating history is 1e-300 s after the row before it; the least time between two rows is 0.001 s (HistoryRowsTooClose).",
    );
    fault(
        mixture_with("0.2329", "0", &[("material", "\"TACOT 3.0\"")]),
        422,
        "Physical Violation: the stream has oxygen mass fraction 0.2329 and argon mass fraction 0; this material's surface table is for air alone (0.23291751145757963 and 0) (StreamNotAirForThisMaterial).",
    );
    fault(
        mixture_with("-0.04", "0", &[]),
        422,
        "Physical Violation: the stream's oxygen mass fraction is -0.04; a mass fraction is 0 to 1 (StreamOxygenNotAMassFraction).",
    );
    fault(
        mixture_with("0.2", "\"none\"", &[]),
        422,
        "Physical Violation: 'stream_argon_mass_fraction' must be a number; this request has \"none\".",
    );
    // Air takes no fraction: one given with it is refused by name, whatever
    // its value. A mixture takes both.
    for (fraction, value) in [
        ("stream_oxygen_mass_fraction", "0.23291751145757963"),
        ("stream_oxygen_mass_fraction", "0.2329"),
        ("stream_argon_mass_fraction", "0"),
    ] {
        fault(
            stream_with(&[(fraction, value)]),
            422,
            &format!(
                "Physical Violation: '{fraction}' is an input only when 'stream_gas' is \"mixture\"; this request has 'stream_gas' \"air\" (InputNotWithChoice)."
            ),
        );
    }
    fault(
        stream_with(&[("stream_gas", "\"mixture\""), ("stream_oxygen_mass_fraction", "0.2")]),
        422,
        "Missing required parameter: 'stream_argon_mass_fraction' (InputMissing).",
    );
    fault(
        stream_with(&[("stream_gas", "\"mixture\""), ("stream_argon_mass_fraction", "0")]),
        422,
        "Missing required parameter: 'stream_oxygen_mass_fraction' (InputMissing).",
    );
    fault(
        stream_with(&[("stream_gas", "\"argon\"")]),
        422,
        // Re-pinned for 0.10.1: a gas that is not one of the two words carries the name UnknownStreamGas.
        "Physical Violation: 'stream_gas' must be one of \"air\", \"mixture\"; this request has \"argon\" (UnknownStreamGas).",
    );
    fault(
        stream_with(&[("stream_gas", "0.2329")]),
        422,
        "Physical Violation: 'stream_gas' must be a string; this request has 0.2329.",
    );
    // A held face has no stream and no gas.
    fault(
        held_with(&[("stream_gas", "\"air\"")]),
        422,
        "Physical Violation: 'stream_gas' is an input only when 'face' is \"gas_stream\"; this request has 'face' \"held_temperature\" (InputNotWithChoice).",
    );
    fault(
        held_with(&[("stream_oxygen_mass_fraction", "0.2")]),
        422,
        "Physical Violation: 'stream_oxygen_mass_fraction' is an input only when 'stream_gas' is \"mixture\"; this request has no 'stream_gas' (InputNotWithChoice).",
    );
    // A wall pressure outside the table of the material asked for: the
    // solver's sentence states that material's own range.
    fault(
        stream_with(&[("history_pressure_pa", "[0.5,20000,20000]")]),
        422,
        "Physical Violation: row 0 of the heating history has pressure 0.5 Pa, outside this material's surface table, 1.01325 to 101325 Pa (StreamPressureOutOfRange).",
    );
    fault(
        stream_with(&[("history_pressure_pa", "[20000,20000,101326]")]),
        422,
        "Physical Violation: row 2 of the heating history has pressure 101326 Pa, outside this material's surface table, 1.01325 to 101325 Pa (StreamPressureOutOfRange).",
    );
    fault(
        stream_with(&[("material", "\"TACOT 3.0\""), ("history_pressure_pa", "[50,20000,20000]")]),
        422,
        "Physical Violation: row 0 of the heating history has pressure 50 Pa, outside this material's surface table, 101.325 to 101325 Pa (StreamPressureOutOfRange).",
    );
    // Every older version of the PICA model is still taken by name.
    for older in ["PICA, open record, version 4", "PICA, open record, version 3", "PICA, open record"] {
        fault(
            stream_with(&[("material", &format!("\"{older}\"")), ("history_pressure_pa", "[1,20000,20000]")]),
            422,
            "Physical Violation: row 0 of the heating history has pressure 1 Pa, outside this material's surface table, 1.01325 to 101325 Pa (StreamPressureOutOfRange).",
        );
    }
    // A material the solver does not hold, by the name the solver gives it.
    for unknown in ["\"PICA\"", "\"TACOT\"", "\"pica, open record, version 5\"", "\"PICA, open record, version 6\"", "\"\""] {
        fault(
            held_with(&[("material", unknown)]),
            422,
            &format!(
                "Physical Violation: 'material' must be one of \"TACOT 3.0\", \"PICA, open record, version 5\", \"PICA, open record, version 4\", \"PICA, open record, version 3\", \"PICA, open record\"; this request has {unknown} (UnknownMaterial)."
            ),
        );
    }
    fault(
        held_with(&[("material", "3")]),
        422,
        "Physical Violation: 'material' must be a string; this request has 3.",
    );
    // A key no list holds, at either depth.
    let unknown_key = "Physical Violation: 'char_conductivity_scale' is not an input of this solver. The material's numbers, the cells, the time steps and the tolerances stay inside the solver.";
    fault(held_with(&[("char_conductivity_scale", "0.5")]), 422, unknown_key);
    fault(stream_with(&[("char_conductivity_scale", "0.5")]), 422, unknown_key);
    fault(
        stream_with(&[("history", "[[0,20000,0,2.5e7,0]]")]),
        422,
        "Physical Violation: 'history' is not an input of this solver. The material's numbers, the cells, the time steps and the tolerances stay inside the solver.",
    );
    fault(
        r#"{"solver":"ablator_slab","parameters":{},"want":["face_history"]}"#.to_string(),
        422,
        "Physical Violation: 'want' is not an input of this solver. The material's numbers, the cells, the time steps and the tolerances stay inside the solver.",
    );
    // A missing field, of every kind: nothing is filled in.
    for (base, key) in [
        (&SHORT_HELD[..], "material"),
        (&SHORT_HELD[..], "thickness_m"),
        (&SHORT_HELD[..], "start_temperature_k"),
        (&SHORT_HELD[..], "thermocouple_depths_m"),
        (&SHORT_HELD[..], "face"),
        (&SHORT_HELD[..], "held_temperature_k"),
        (&SHORT_HELD[..], "hold_duration_s"),
        (&SHORT_HELD[..], "continue_after_s"),
        (&SHORT_HELD[..], "return_face_history"),
        (&SHORT_HELD[..], "return_thermocouple_histories"),
        (&SHORT_HELD[..], "return_final_profile"),
        (&SHORT_STREAM[..], "stream_gas"),
        (&SHORT_STREAM[..], "history_time_s"),
        (&SHORT_STREAM[..], "history_pressure_pa"),
        (&SHORT_STREAM[..], "history_heat_transfer_coefficient_kg_per_m2_s"),
        (&SHORT_STREAM[..], "history_flow_enthalpy_j_per_kg"),
        (&SHORT_STREAM[..], "history_radiative_heating_w_per_m2"),
        (&SHORT_STREAM[..], "surroundings_temperature_k"),
        (&SHORT_STREAM[..], "face_wears"),
        (&SHORT_STREAM[..], "return_final_profile"),
    ] {
        fault(
            request_text(base, &[(key, None)]),
            422,
            &format!("Missing required parameter: '{key}' (InputMissing)."),
        );
    }
    fault(
        r#"{"solver":"ablator_slab"}"#.to_string(),
        422,
        "Missing required parameter: 'parameters' (or 'example', the name of a worked example describe_solver lists) (InputMissing).",
    );
    // A key that goes with the other kind of face.
    fault(
        held_with(&[("surroundings_temperature_k", "300")]),
        422,
        "Physical Violation: 'surroundings_temperature_k' is an input only when 'face' is \"gas_stream\"; this request has 'face' \"held_temperature\" (InputNotWithChoice).",
    );
    fault(
        held_with(&[("history_time_s", "[0,1]")]),
        422,
        "Physical Violation: 'history_time_s' is an input only when 'face' is \"gas_stream\"; this request has 'face' \"held_temperature\" (InputNotWithChoice).",
    );
    fault(
        stream_with(&[("continue_after_s", "0")]),
        422,
        "Physical Violation: 'continue_after_s' is an input only when 'face' is \"held_temperature\"; this request has 'face' \"gas_stream\" (InputNotWithChoice).",
    );
    fault(
        stream_with(&[("held_temperature_k", "1644")]),
        422,
        "Physical Violation: 'held_temperature_k' is an input only when 'face' is \"held_temperature\"; this request has 'face' \"gas_stream\" (InputNotWithChoice).",
    );
    fault(
        held_with(&[("face", "\"arc_jet\"")]),
        422,
        // Re-pinned for 0.10.1: a face that is not one of the two words carries the name UnknownFace.
        "Physical Violation: 'face' must be one of \"held_temperature\", \"gas_stream\"; this request has \"arc_jet\" (UnknownFace).",
    );
    // A row with a number missing: the columns do not hold the same count.
    fault(
        stream_with(&[("history_flow_enthalpy_j_per_kg", "[2.5e7,2.5e7]")]),
        422,
        "Physical Violation: the five columns of the heating history hold one number for each row, and 'history_time_s' holds 3 where 'history_flow_enthalpy_j_per_kg' holds 2.",
    );
    fault(
        stream_with(&[("history_time_s", "[0,0.5,4,5]")]),
        422,
        "Physical Violation: the five columns of the heating history hold one number for each row, and 'history_time_s' holds 4 where 'history_pressure_pa' holds 3.",
    );
    // A value of the wrong kind.
    fault(
        held_with(&[("thickness_m", "\"0.01\"")]),
        422,
        "Physical Violation: 'thickness_m' must be a number; this request has \"0.01\".",
    );
    fault(
        held_with(&[("thermocouple_depths_m", "0.001")]),
        422,
        "Physical Violation: 'thermocouple_depths_m' must be an array of numbers; this request has 0.001.",
    );
    fault(
        stream_with(&[("history_pressure_pa", "[20000,null,20000]")]),
        422,
        "Physical Violation: 'history_pressure_pa[1]' must be a number; this request has null.",
    );
    fault(
        stream_with(&[("history_time_s", "[[0,20000,0,0,0],[1,20000,0,0,0]]")]),
        422,
        "Physical Violation: 'history_time_s[0]' must be a number; this request has [0,20000,0,0,0].",
    );
    fault(
        stream_with(&[("face_wears", "1")]),
        422,
        "Physical Violation: 'face_wears' must be true or false; this request has 1.",
    );
    fault(
        held_with(&[("return_face_history", "\"yes\"")]),
        422,
        "Physical Violation: 'return_face_history' must be true or false; this request has \"yes\".",
    );
    fault(
        held_with(&[("return_final_profile", "null")]),
        422,
        "Physical Violation: 'return_final_profile' must be true or false; this request has null.",
    );
    fault(
        r#"{"solver":"ablator_slab","parameters":[0.01,298.0]}"#.to_string(),
        422,
        "Physical Violation: 'parameters' must be a JSON object.",
    );
    // A repeated key, and a number that is not zero and reads as zero.
    let repeated = held_with(&[]).replace(
        "\"thickness_m\":0.01,",
        "\"thickness_m\":0.01,\"thickness_m\":0.02,",
    );
    assert!(repeated.matches("thickness_m").count() == 2);
    fault(repeated, 422, "Physical Violation: the key 'thickness_m' is repeated.");
    let repeated = stream_with(&[]).replace(
        "\"history_time_s\":[0,0.5,4.0],",
        "\"history_time_s\":[0,0.5,4.0],\"history_time_s\":[0,0.5,5.0],",
    );
    assert!(repeated.matches("history_time_s").count() == 2);
    fault(repeated, 422, "Physical Violation: the key 'history_time_s' is repeated.");
    let repeated = held_with(&[]).replace(
        "\"return_face_history\":false,",
        "\"return_face_history\":false,\"return_face_history\":true,",
    );
    fault(repeated, 422, "Physical Violation: the key 'return_face_history' is repeated.");
    fault(
        stream_with(&[("history_heat_transfer_coefficient_kg_per_m2_s", "[0,1e-400,0.17]")]),
        422,
        "Physical Violation: 'history_heat_transfer_coefficient_kg_per_m2_s[1]' is written 1e-400, a number that is not zero and reads as zero.",
    );
    // A solver no connection has.
    fault(
        r#"{"solver":"pica_arcjet_slab","parameters":{}}"#.to_string(),
        404,
        "Solver not recognized (UnknownSolver).",
    );

    for (request, status, message) in &faults {
        let (answered, text) = connections.all("/solve", "solve", request);
        let shown = &request[..request.len().min(400)];
        assert_eq!(&text, message, "{shown}");
        assert_eq!(answered, *status, "{shown}");
        for proof in ["solution_seal", "receipt_sha256", "seal_parts", "field_sha256"] {
            assert!(!text.contains(proof), "{shown}: {text}");
        }
    }
    assert_eq!(faults.len(), 107);

    // The edges of the declared range are inside it: on TACOT 3.0, the
    // lowest wall pressure of its table and a history of 256 rows are
    // answered, and so is a thermocouple at the back of the slab. On the
    // PICA model the lowest pressure of its table, 1.01325 Pa, is below
    // anything compared with measurement and is answered in no envelope.
    let rows_256 = long_history(256);
    let mut edge: Vec<(&str, &str)> =
        rows_256.iter().map(|(key, text)| (*key, text.as_str())).collect();
    edge.push(("history_pressure_pa", ""));
    let lowest = numbers_text(256, |_| 101.325);
    let last = edge.len() - 1;
    edge[last] = ("history_pressure_pa", lowest.as_str());
    edge.push(("thermocouple_depths_m", "[0,0.06]"));
    edge.push(("material", "\"TACOT 3.0\""));
    let (_, body) = connections.solved_on_the_web(&stream_with(&edge));
    assert_eq!(body["outcome"], "converged");
    assert_eq!(body["result"]["duration_s"], 255.0);
    assert_eq!(body["result"]["wear_m"], 0.0);
    let pica_lowest = numbers_text(256, |_| 1.01325);
    let edge_count = edge.len();
    edge[last] = ("history_pressure_pa", pica_lowest.as_str());
    edge[edge_count - 1] = ("material", "\"PICA, open record, version 5\"");
    let (_, body) = connections.solved_on_the_web(&stream_with(&edge));
    assert_eq!(body["outcome"], "converged");
    assert_eq!(body["result"]["inside_arc_jet_comparison"], 0.0);
    assert_eq!(body["result"]["inside_stardust_comparison"], 0.0);
    assert_eq!(body["result"]["inside_thermocouple_block_comparison"], 0.0);
}

/// What the solver itself says of a question it refuses while computing.
fn refused_directly(question: &SlabQuestion) -> SlabReject {
    match slab::answer(TACOT, question, &Wanted::default()) {
        Ok(_) => panic!("the solver answered"),
        Err(reject) => reject,
    }
}

/// A steady stream on TACOT from 300 K, as the solver's own tests ask it.
fn steady(
    thickness_m: f64,
    coefficient: f64,
    enthalpy: f64,
    pressure_pa: f64,
    radiative: f64,
    end_s: f64,
    face_wears: bool,
) -> (String, SlabQuestion) {
    let request = json!({
        "solver": "ablator_slab",
        "parameters": {
            "material": TACOT,
            "thickness_m": thickness_m,
            "start_temperature_k": 300.0,
            "thermocouple_depths_m": [],
            "face": "gas_stream",
            "stream_gas": "air",
            "history_time_s": [0.0, 0.1, end_s],
            "history_pressure_pa": [pressure_pa, pressure_pa, pressure_pa],
            "history_heat_transfer_coefficient_kg_per_m2_s": [0.0, coefficient, coefficient],
            "history_flow_enthalpy_j_per_kg": [0.0, enthalpy, enthalpy],
            "history_radiative_heating_w_per_m2": [0.0, radiative, radiative],
            "surroundings_temperature_k": 300.0,
            "face_wears": face_wears,
            "return_face_history": true,
            "return_thermocouple_histories": false,
            "return_final_profile": false
        }
    })
    .to_string();
    let row = |time_s: f64, on: f64| StreamRow {
        time_s,
        pressure_pa,
        heat_transfer_coefficient_kg_per_m2_s: coefficient * on,
        flow_enthalpy_j_per_kg: enthalpy * on,
        radiative_heating_w_per_m2: radiative * on,
    };
    let question = SlabQuestion {
        thickness_m,
        start_temperature_k: 300.0,
        surface: SurfaceCondition::GasStream {
            history: vec![row(0.0, 0.0), row(0.1, 1.0), row(end_s, 1.0)],
            surroundings_temperature_k: 300.0,
            face_wears,
            stream_gas: StreamGas::AIR,
        },
        continue_after_s: 0.0,
        thermocouple_depths_m: Vec::new(),
    };
    (request, question)
}

/// A question inside every limit a request shows, which the solver refuses
/// once it is computing: the refusal is returned under the solver's name
/// with the solver's sentence, with no result, no invariants and no field
/// hash. Its seal binds the refusal and nothing else.
#[test]
fn a_refusal_met_while_computing_is_named_and_carries_no_result() {
    let mut connections = Connections::open();
    let cases = [
        // Hotter than the material's tables go.
        ("TemperatureOutOfBounds", steady(0.05, 1.0, 1.0e8, 101_325.0, 0.0, 5.0, true)),
        // A face driven to where the table's carbon boils at 0.001 atm.
        ("FaceBeyondSurfaceTable", steady(0.05, 0.003, 1.0e4, 101.325, 6.0e6, 20.0, false)),
    ];
    let mut seals = Vec::new();
    for (index, (name, (request, question))) in cases.iter().enumerate() {
        let direct = refused_directly(question);
        assert_eq!(direct.name(), *name);
        // The first on all three connections, the others on the web.
        let (text, body) = if index == 0 {
            connections.solved(request)
        } else {
            connections.solved_on_the_web(request)
        };
        assert_eq!(body["outcome"], "refused", "{name}");
        assert_eq!(body["reject"], *name);
        assert_eq!(
            body["reject_detail"],
            format!("Solver Refusal: ablator_slab did not return a solved slab ({name}). {direct}.")
        );
        assert!(body["iterations"].is_null());
        assert!(body["result"].is_null());
        assert!(body["invariants"].is_null());
        assert!(body["field_sha256"].is_null());
        assert!(!text.contains("wear_m"), "{text}");
        // The seal is the request, no lines, the word null and the refusal.
        let request_text = sealed_text(&body["inputs"]);
        let tail = format!(r#"{{"outcome":"refused","iterations":null,"reject":"{name}"}}"#);
        let version = format!("engine_version:{}\n", body["engine_version"].as_str().expect("a version"));
        assert_eq!(
            body["solution_seal"],
            sha256_hex(format!("{request_text}{version}null{tail}").as_bytes())
        );
        connections.assert_holds(&body);
        // A refusal cannot be turned into another, or into an answer.
        let mut renamed = body.clone();
        renamed["reject"] = json!("StepNotConverged");
        connections.assert_fails(&renamed, "outcome", "the refusal renamed");
        let mut promoted = body.clone();
        promoted["outcome"] = json!("converged");
        connections.assert_fails(&promoted, "outcome", "the refusal made an answer");
        seals.push(body["solution_seal"].as_str().unwrap().to_string());
    }
    seals.sort();
    seals.dedup();
    assert_eq!(seals.len(), 2);
}

/// A slab worn away is the end of the event, not a refusal: a thin slab
/// under the stream of workshop case 2.3 wears through, and the run ends
/// there, answered with its proof, `slab_worn_away` 1, its duration the
/// moment it wore away and every account closed.
#[test]
fn a_slab_worn_away_ends_its_run_and_is_answered() {
    let mut connections = Connections::open();
    let (request, question) = steady(0.01, 0.3, 2.5e7, 101_325.0, 0.0, 60.0, true);
    let direct = slab::answer(TACOT, &question, &Wanted::default()).expect("an end, answered");
    assert!(direct.summary.worn_away);
    let (_, body) = connections.solved(&request);
    assert_eq!(body["outcome"], "converged", "{body}");
    assert!(body["reject"].is_null());
    let result = &body["result"];
    assert_eq!(result["slab_worn_away"], 1.0);
    let duration = result["duration_s"].as_f64().expect("duration");
    assert!(duration > 0.0 && duration < 60.0, "{duration}");
    assert_eq!(duration, direct.summary.duration_s);
    assert!(result["wear_m"].as_f64().unwrap() > 0.008, "{result}");
    connections.assert_holds(&body);
}

/// A PICA request outside what was compared with measurement is answered
/// with a proof on all three connections and says it lies in no compared
/// envelope; past the top of the PICA tables, 3975 K, a run is refused by
/// name with a sealed refusal; a stream colder than the wall on any material
/// is refused at the door by name, with no proof.
#[test]
fn a_request_outside_what_was_compared_is_answered_with_a_proof_and_says_so() {
    let mut connections = Connections::open();
    let answered = [
        // A held face on PICA.
        held_with(&[("material", "\"PICA, open record, version 5\"")]),
        // A thickness, start or surroundings no measured case had.
        stream_with(&[("thickness_m", "0.05")]),
        stream_with(&[("start_temperature_k", "350")]),
        stream_with(&[("surroundings_temperature_k", "350")]),
        // Values each compared in some envelope but never together: the
        // arc-jet stream on a block's thickness, from the Stardust start, or
        // radiating to the Stardust surroundings.
        stream_with(&[("thickness_m", "0.03")]),
        stream_with(&[("start_temperature_k", "260")]),
        stream_with(&[("surroundings_temperature_k", "266.48")]),
        // A face kept from wearing.
        stream_with(&[("face_wears", "false")]),
        // 10 W/cm² in the arc-jet gas: below the lowest arc-jet heating, and
        // not air.
        mixture_with(
            "0.1675336",
            "0.276",
            &[("history_heat_transfer_coefficient_kg_per_m2_s", "[0,0.0042,0.0042]")],
        ),
    ];
    for (index, request) in answered.iter().enumerate() {
        let (_, body) = if index == 0 {
            connections.solved(request)
        } else {
            connections.solved_on_the_web(request)
        };
        assert_eq!(body["outcome"], "converged", "{request}");
        for flag in ["inside_arc_jet_comparison", "inside_stardust_comparison", "inside_thermocouple_block_comparison"] {
            assert_eq!(body["result"][flag], 0.0, "{flag}: {request}");
        }
        connections.assert_holds(&body);
    }
    // Hotter than any compared run, 3412 K, and answered; past the top of
    // the tables, 3975 K, refused by name, the refusal sealed.
    let hot = stream_with(&[
        ("history_pressure_pa", "[101325,101325,101325]"),
        ("history_heat_transfer_coefficient_kg_per_m2_s", "[0,1,1]"),
        ("history_flow_enthalpy_j_per_kg", "[1e8,1e8,1e8]"),
        ("thermocouple_depths_m", "[]"),
        ("thickness_m", "0.1"),
    ]);
    let (_, body) = connections.solved_on_the_web(&hot);
    assert_eq!(body["outcome"], "converged");
    let peak = body["result"]["peak_face_temperature_k"].as_f64().unwrap();
    assert!(peak > slab::COMPARED_TEMPERATURE_MAX_K && peak < genesis_solvers::pica::TABLE_TOP_K, "{peak}");
    let past = stream_with(&[
        ("history_pressure_pa", "[101325,101325,101325]"),
        ("history_heat_transfer_coefficient_kg_per_m2_s", "[0,0.2,0.2]"),
        ("history_flow_enthalpy_j_per_kg", "[5e7,5e7,5e7]"),
        ("history_radiative_heating_w_per_m2", "[0,2e8,2e8]"),
        ("thermocouple_depths_m", "[]"),
    ]);
    let (_, body) = connections.solved_on_the_web(&past);
    assert_eq!(body["outcome"], "refused");
    assert_eq!(body["reject"], "TemperatureOutOfBounds");
    connections.assert_holds(&body);
    // 1,600 J/kg in air below air's 1,871 J/kg at a wall at 300 K, on
    // TACOT as on PICA: refused at the door by name.
    for request in [
        stream_with(&[
            ("material", "\"TACOT 3.0\""),
            ("history_flow_enthalpy_j_per_kg", "[1600,1600,1600]"),
        ]),
        stream_with(&[("history_flow_enthalpy_j_per_kg", "[1600,1600,1600]")]),
    ] {
        let (status, text) = connections.all("/solve", "solve", &request);
        assert_eq!(status, 422, "{text}");
        assert!(text.starts_with("Physical Violation: "), "{text}");
        assert!(text.ends_with("(FlowEnthalpyBelowWall)."), "{text}");
    }
    // The short stream itself, in air at 20 kPa and 425 W/cm² for 4 s, lies
    // inside the arc-jet envelope and is answered; on the Stardust slab,
    // start and surroundings it lies inside the Stardust one.
    let (_, body) = connections.solved(&stream_with(&[]));
    assert_eq!(body["outcome"], "converged");
    assert_eq!(body["result"]["inside_arc_jet_comparison"], 1.0);
    assert_eq!(body["result"]["inside_stardust_comparison"], 0.0);
    assert_eq!(body["result"]["inside_thermocouple_block_comparison"], 0.0);
    let (_, body) = connections.solved(&stream_with(&[
        ("thickness_m", "0.0582"),
        ("start_temperature_k", "260"),
        ("surroundings_temperature_k", "266.48"),
    ]));
    assert_eq!(body["outcome"], "converged");
    assert_eq!(body["result"]["inside_arc_jet_comparison"], 0.0);
    assert_eq!(body["result"]["inside_stardust_comparison"], 1.0);
}

/// The four thermocouple blocks of Tran and others 1997, the data the one
/// fitted number was fitted to, asked as the development runs asked them,
/// are answered with a proof inside their own envelope on all three
/// connections. Their thermocouples are scored in the research record of
/// the PICA arc-jet model (31.28 K root mean square over 259 samples).
#[test]
fn the_four_thermocouple_blocks_are_answered_inside_their_own_envelope() {
    let mut connections = Connections::open();
    let c400 = 400.0e4 / (29.5e6 - 1871.4044377954367);
    let c580 = 580.0e4 / (29.5e6 - 1871.4044377954367);
    for (block, thickness, c, pressure, exposure, end, depths) in [
        ("9B", "0.0274", c400, 20265.0, 29.0, 125.0, "[0.0052,0.0102,0.0176]"),
        ("7B", "0.0274", c580, 45596.25, 15.0, 135.0, "[0.0052,0.0102,0.0176]"),
        ("3B", "0.0325", c580, 45596.25, 40.0, 152.0, "[0.0084,0.013300000000000001,0.0207]"),
        ("6A", "0.022400000000000003", c580, 45596.25, 20.0, 115.0, "[0.0052,0.0143]"),
    ] {
        let times = format!("[0,0.5,{exposure:?},{:?},{end:?}]", exposure + 0.5);
        let pressures = format!("[{pressure:?},{pressure:?},{pressure:?},{pressure:?},{pressure:?}]");
        let coefficients = format!("[0,{c:?},{c:?},0,0]");
        let request = stream_with(&[
            ("thickness_m", thickness),
            ("start_temperature_k", "303.15"),
            ("thermocouple_depths_m", depths),
            ("history_time_s", &times),
            ("history_pressure_pa", &pressures),
            ("history_heat_transfer_coefficient_kg_per_m2_s", &coefficients),
            ("history_flow_enthalpy_j_per_kg", "[2.95e7,2.95e7,2.95e7,2.95e7,2.95e7]"),
            ("history_radiative_heating_w_per_m2", "[0,0,0,0,0]"),
        ]);
        let (_, body) = connections.solved(&request);
        assert_eq!(body["outcome"], "converged", "{block}");
        assert_eq!(body["result"]["inside_thermocouple_block_comparison"], 1.0, "{block}");
        assert_eq!(body["result"]["inside_arc_jet_comparison"], 0.0, "{block}");
        assert_eq!(body["result"]["inside_stardust_comparison"], 0.0, "{block}");
        let peak = body["result"]["peak_face_temperature_k"].as_f64().unwrap();
        assert!(peak > 2700.0 && peak < slab::COMPARED_TEMPERATURE_MAX_K, "{block}: {peak}");
    }
}
