//! The stdio server calls the same functions as the HTTP routes.

mod answer_text;

use std::time::Duration;

use rmcp::ServiceExt;
use rmcp::model::CallToolRequestParams;
use rmcp::transport::TokioChildProcess;
use serde_json::{Map, Value, json};
use tokio::net::TcpListener;

async fn spawn_app() -> String {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    tokio::spawn(async move {
        axum::serve(listener, protocol_mcp_gateway::app())
            .await
            .unwrap();
    });
    tokio::task::yield_now().await;
    format!("http://127.0.0.1:{port}")
}

async fn post(base: &str, path: &str, body: &Value) -> (u16, String) {
    let client = reqwest::Client::new();
    let url = format!("{base}{path}");
    let text = answer_text::as_sent(body);
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
                let response_text = response.text().await.unwrap();
                answer_text::remember(&response_text);
                return (status, response_text);
            }
            Err(error) => {
                last_error = Some(error);
                tokio::time::sleep(Duration::from_millis(25)).await;
            }
        }
    }
    panic!("{path} never accepted a connection: {last_error:?}");
}

fn arguments(value: &Value) -> Map<String, Value> {
    value.as_object().expect("object").clone()
}

#[tokio::test]
async fn mcp_matches_http_for_a_step_a_run_and_a_violation() {
    let bin = env!("CARGO_BIN_EXE_mcp_stdio");
    let transport =
        TokioChildProcess::new(tokio::process::Command::new(bin)).expect("spawn mcp_stdio");
    let mut client = ().serve(transport).await.expect("initialize");
    let info = client.peer_info().expect("initialize result");
    assert!(
        info.instructions
            .as_deref()
            .unwrap_or("")
            .contains("find_model")
    );
    let server = info.server_info.as_ref().expect("initialize server info");
    assert_eq!(server.name, "Zero-Trust Physics");
    assert_eq!(server.version, env!("CARGO_PKG_VERSION"));
    assert_eq!(
        server.description.as_deref(),
        Some(protocol_mcp_gateway::ENGINE_VERSION)
    );

    let listed = client.list_tools(None).await.expect("tools/list");
    let names: Vec<_> = listed
        .tools
        .iter()
        .map(|tool| tool.name.to_string())
        .collect();
    assert_eq!(
        names,
        vec![
            "list_cards",
            "describe_card",
            "compute_step",
            "run_trajectory",
            "verify_tape",
            "list_solvers",
            "describe_solver",
            "solve",
            "verify_solution",
            "run_dataset",
            "verify_dataset",
            "verify_receipt",
            "validate_series",
            "cite",
            "find_model",
            "find_flip",
            "design_run",
            "verify_step",
        ]
    );
    assert_eq!(listed.tools.len(), 18);

    let base = spawn_app().await;
    let step = json!({
        "domain": "energy",
        "card": "thermal",
        "parameters": {
            "temperature_k": 293.15,
            "heat_power_w": 1.0,
            "ambient_k": 293.15,
            "dt_s": 0.1
        }
    });
    let (status, http_step) = post(&base, "/verify_physics", &step).await;
    assert_eq!(status, 200, "{http_step}");
    let mcp_step = client
        .call_tool(CallToolRequestParams::new("verify_step").with_arguments(arguments(&step)))
        .await
        .expect("verify_step");
    assert_eq!(mcp_step.is_error, Some(false));
    let mcp_step_text = mcp_step.content[0].as_text().expect("text").text.clone();
    assert_eq!(mcp_step_text, http_step);
    let http_json: Value = serde_json::from_str(&http_step).unwrap();
    let mcp_json: Value = serde_json::from_str(&mcp_step_text).unwrap();
    assert_eq!(http_json["sha384_seal"], mcp_json["sha384_seal"]);
    assert_eq!(http_json["inputs"], step, "a step echoes the whole request");

    let run = json!({
        "domain": "energy",
        "card": "thermal",
        "initial": { "temperature_k": 293.15 },
        "dt_s": 0.1,
        "segments": [{
            "steps": 4,
            "load": { "heat_power_w": 1.0, "ambient_k": 293.15 }
        }],
        "record_every": 1
    });
    let (status, http_run) = post(&base, "/run_trajectory", &run).await;
    assert_eq!(status, 200, "{http_run}");
    let mcp_run = client
        .call_tool(CallToolRequestParams::new("run_trajectory").with_arguments(arguments(&run)))
        .await
        .expect("run_trajectory");
    assert_eq!(mcp_run.is_error, Some(false));
    let mcp_run_text = mcp_run.content[0].as_text().expect("text").text.clone();
    assert_eq!(mcp_run_text, http_run);
    let http_json: Value = serde_json::from_str(&http_run).unwrap();
    let mcp_json: Value = serde_json::from_str(&mcp_run_text).unwrap();
    assert_eq!(http_json["run_seal"], mcp_json["run_seal"]);

    let bad = json!({
        "domain": "energy",
        "card": "thermal",
        "parameters": {
            "temperature_k": -1.0,
            "heat_power_w": 1.0,
            "ambient_k": 293.15,
            "dt_s": 0.1
        }
    });
    let (status, http_error) = post(&base, "/verify_physics", &bad).await;
    assert_eq!(status, 422, "{http_error}");
    let mcp_error = client
        .call_tool(CallToolRequestParams::new("verify_step").with_arguments(arguments(&bad)))
        .await
        .expect("tool result");
    assert_eq!(mcp_error.is_error, Some(true));
    assert_eq!(
        mcp_error.content[0].as_text().expect("text").text,
        http_error
    );

    let listed_solvers = client
        .call_tool(CallToolRequestParams::new("list_solvers"))
        .await
        .expect("list_solvers");
    assert_eq!(listed_solvers.is_error, Some(false));
    let http_listed = reqwest::get(format!("{base}/solvers"))
        .await
        .unwrap()
        .text()
        .await
        .unwrap();
    assert_eq!(
        listed_solvers.content[0].as_text().expect("text").text,
        http_listed
    );

    let which = json!({ "solver": "atom_dft" });
    let (status, http_described) = post(&base, "/describe_solver", &which).await;
    assert_eq!(status, 200, "{http_described}");
    let mcp_described = client
        .call_tool(CallToolRequestParams::new("describe_solver").with_arguments(arguments(&which)))
        .await
        .expect("describe_solver");
    assert_eq!(mcp_described.is_error, Some(false));
    assert_eq!(
        mcp_described.content[0].as_text().expect("text").text,
        http_described
    );

    let hydrogen = json!({ "solver": "atom_dft", "parameters": { "atomic_number": 1 } });
    let (status, http_solution) = post(&base, "/solve", &hydrogen).await;
    assert_eq!(status, 200, "{http_solution}");
    let mcp_solution = client
        .call_tool(CallToolRequestParams::new("solve").with_arguments(arguments(&hydrogen)))
        .await
        .expect("solve");
    assert_eq!(mcp_solution.is_error, Some(false));
    let mcp_solution_text = mcp_solution.content[0]
        .as_text()
        .expect("text")
        .text
        .clone();
    assert_eq!(mcp_solution_text, http_solution);
    let http_json: Value = serde_json::from_str(&http_solution).unwrap();
    let mcp_json: Value = serde_json::from_str(&mcp_solution_text).unwrap();
    assert_eq!(http_json["outcome"], "converged");
    assert_eq!(http_json["solution_seal"], mcp_json["solution_seal"]);
    assert_eq!(http_json["field_sha384"], mcp_json["field_sha384"]);

    let (status, http_verdict) = post(&base, "/verify_solution", &http_json).await;
    assert_eq!(status, 200, "{http_verdict}");
    assert_eq!(http_verdict, format!(r#"{{"holds":true,"values_hold":true,"cause":null,"compared":{}}}"#, serde_json::json!(protocol_mcp_gateway::WHOLE_SOLUTION_COMPUTED)));
    let mcp_verdict = client
        .call_tool(
            CallToolRequestParams::new("verify_solution").with_arguments(arguments(&mcp_json)),
        )
        .await
        .expect("verify_solution");
    assert_eq!(mcp_verdict.is_error, Some(false));
    // The MCP library writes the arguments again from their values, so the
    // copy it sends spells each number as serde_json does: every value is the
    // engine's and its text is not, and the long form names where the
    // spelling first differs. The web connection given that same text says
    // the same, byte for byte.
    let mcp_verdict_text = mcp_verdict.content[0].as_text().expect("text").text.clone();
    let report: Value = serde_json::from_str(&mcp_verdict_text).unwrap();
    assert_eq!((report["holds"].clone(), report["cause"].clone()), (json!(false), json!("answer_text")), "{report}");
    assert!(report["difference"].as_str().unwrap().contains("the same value spelled another way. Every value of the answer sent is the engine's answer to its request, computed again"), "{report}");
    let written = serde_json::to_string(&mcp_json).unwrap();
    let http_written = reqwest::Client::new()
        .post(format!("{base}/verify_solution"))
        .header(reqwest::header::CONTENT_TYPE, "application/json")
        .body(written)
        .send()
        .await
        .unwrap()
        .text()
        .await
        .unwrap();
    assert_eq!(mcp_verdict_text, http_written);

    let potassium = json!({ "solver": "atom_dft", "parameters": { "atomic_number": 19 } });
    let (status, http_range) = post(&base, "/solve", &potassium).await;
    assert_eq!(status, 422, "{http_range}");
    assert_eq!(
        http_range,
        // Re-pinned for 0.10.1: an atomic number outside the range carries the name AtomicNumberOutOfRange, and the entry writes the range in words.
        "Physical Violation: atomic number 19 is outside 1 to 18 (AtomicNumberOutOfRange)."
    );
    let mcp_range = client
        .call_tool(CallToolRequestParams::new("solve").with_arguments(arguments(&potassium)))
        .await
        .expect("tool result");
    assert_eq!(mcp_range.is_error, Some(true));
    assert_eq!(
        mcp_range.content[0].as_text().expect("text").text,
        http_range
    );

    client.close().await.expect("close");
}

#[tokio::test]
async fn a_repeated_key_on_the_agent_door_is_refused_and_not_sealed() {
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
    use tokio::process::Command;

    let bin = env!("CARGO_BIN_EXE_mcp_stdio");
    let mut child = Command::new(bin)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .kill_on_drop(true)
        .spawn()
        .expect("spawn mcp_stdio");
    let mut stdin = child.stdin.take().unwrap();
    let mut reader = BufReader::new(child.stdout.take().unwrap());

    async fn exchange(
        stdin: &mut tokio::process::ChildStdin,
        reader: &mut BufReader<tokio::process::ChildStdout>,
        line: &str,
    ) -> String {
        stdin.write_all(line.as_bytes()).await.unwrap();
        stdin.write_all(b"\n").await.unwrap();
        stdin.flush().await.unwrap();
        let mut response = String::new();
        tokio::time::timeout(Duration::from_secs(10), reader.read_line(&mut response))
            .await
            .expect("the agent door stayed silent")
            .unwrap();
        response
    }

    let hello = exchange(
        &mut stdin,
        &mut reader,
        r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"checks","version":"1"}}}"#,
    )
    .await;
    assert!(hello.contains("Zero-Trust Physics"), "{hello}");
    stdin
        .write_all(b"{\"jsonrpc\":\"2.0\",\"method\":\"notifications/initialized\"}\n")
        .await
        .unwrap();
    stdin.flush().await.unwrap();

    let refused = exchange(
        &mut stdin,
        &mut reader,
        r#"{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"verify_step","arguments":{"domain":"energy","card":"thermal","parameters":{"temperature_k":-500.0,"temperature_k":293.15,"heat_power_w":1.0,"ambient_k":293.15,"dt_s":0.1}}}}"#,
    )
    .await;
    let message: Value = serde_json::from_str(refused.trim()).expect(&refused);
    assert_eq!(message["id"], 2, "{refused}");
    assert_eq!(message["result"]["isError"], true, "{refused}");
    let text = message["result"]["content"][0]["text"].as_str().unwrap();
    assert_eq!(text, "Physical Violation: the key 'temperature_k' is repeated.");
    assert!(!refused.contains("sha384_seal"), "{refused}");

    let listed = exchange(
        &mut stdin,
        &mut reader,
        r#"{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"list_cards","arguments":{}}}"#,
    )
    .await;
    let listed: Value = serde_json::from_str(listed.trim()).unwrap();
    assert_eq!(listed["id"], 3, "{listed}");
    let body: Value =
        serde_json::from_str(listed["result"]["content"][0]["text"].as_str().unwrap()).unwrap();
    let pairs = body["cards"].as_array().expect("pairs");
    assert_eq!(pairs.len(), protocol_mcp_gateway::CARDS, "{body}");
}

#[tokio::test]
async fn an_unreadable_line_on_the_agent_door_is_answered() {
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
    use tokio::process::Command;

    let bin = env!("CARGO_BIN_EXE_mcp_stdio");
    let mut child = Command::new(bin)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .kill_on_drop(true)
        .spawn()
        .expect("spawn mcp_stdio");
    let mut stdin = child.stdin.take().unwrap();
    let mut reader = BufReader::new(child.stdout.take().unwrap());

    async fn exchange(
        stdin: &mut tokio::process::ChildStdin,
        reader: &mut BufReader<tokio::process::ChildStdout>,
        line: &[u8],
    ) -> String {
        stdin.write_all(line).await.unwrap();
        if !line.ends_with(b"\n") {
            stdin.write_all(b"\n").await.unwrap();
        }
        stdin.flush().await.unwrap();
        let mut response = String::new();
        tokio::time::timeout(Duration::from_secs(10), reader.read_line(&mut response))
            .await
            .expect("the agent door stayed silent")
            .unwrap();
        response
    }

    fn parse_error(response: &str) -> Value {
        let message: Value = serde_json::from_str(response.trim()).expect(response);
        assert_eq!(message["jsonrpc"], "2.0", "{response}");
        assert_eq!(message["error"]["code"], -32700, "{response}");
        let text = message["error"]["message"].as_str().expect("message");
        assert!(text.contains("could not be read as JSON"), "{response}");
        message
    }

    let too_big = r#"{"jsonrpc":"2.0","id":7,"method":"tools/call","params":{"name":"verify_step","arguments":{"a":1e400}}}"#;
    let too_big_reason = serde_json::from_str::<Value>(too_big)
        .expect_err("1e400 is not a JSON number")
        .to_string();
    let answered = exchange(&mut stdin, &mut reader, too_big.as_bytes()).await;
    let message = parse_error(&answered);
    assert_eq!(message["id"], 7, "{answered}");
    assert!(
        message["error"]["message"]
            .as_str()
            .unwrap()
            .contains(&too_big_reason),
        "{answered} reason {too_big_reason}"
    );

    let plus = exchange(&mut stdin, &mut reader, b"+1").await;
    let message = parse_error(&plus);
    assert!(message["id"].is_null(), "{plus}");
    let plus_reason = serde_json::from_slice::<Value>(b"+1")
        .expect_err("a leading plus is not JSON")
        .to_string();
    assert!(
        message["error"]["message"]
            .as_str()
            .unwrap()
            .contains(&plus_reason),
        "{plus} reason {plus_reason}"
    );

    let mut invalid = br#"{"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"list_solvers","arguments":{"note":""#.to_vec();
    invalid.push(0xff);
    invalid.extend_from_slice(br#""}}}"#);
    let invalid_reason = serde_json::from_slice::<Value>(&invalid)
        .expect_err("0xFF is not UTF-8")
        .to_string();
    let answered = exchange(&mut stdin, &mut reader, &invalid).await;
    let message = parse_error(&answered);
    assert_eq!(message["id"], 4, "{answered}");
    assert!(
        message["error"]["message"]
            .as_str()
            .unwrap()
            .contains(&invalid_reason),
        "{answered} reason {invalid_reason}"
    );

    // A blank line is forwarded and draws no answer of its own. The next
    // call is the initialize that follows it.
    stdin.write_all(b"\n").await.unwrap();
    let hello = exchange(
        &mut stdin,
        &mut reader,
        br#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"checks","version":"1"}}}"#,
    )
    .await;
    let hello: Value = serde_json::from_str(hello.trim()).expect(&hello);
    assert_eq!(hello["id"], 1, "{hello}");
    assert!(
        hello["result"]["serverInfo"]["name"]
            .as_str()
            .unwrap_or("")
            .contains("Zero-Trust Physics")
            || hello.to_string().contains("Zero-Trust Physics"),
        "{hello}"
    );

    stdin
        .write_all(b"{\"jsonrpc\":\"2.0\",\"method\":\"notifications/initialized\"}\n")
        .await
        .unwrap();
    stdin.flush().await.unwrap();
    let listed = exchange(
        &mut stdin,
        &mut reader,
        br#"{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"list_cards","arguments":{}}}"#,
    )
    .await;
    let listed: Value = serde_json::from_str(listed.trim()).unwrap();
    assert_eq!(listed["id"], 3, "{listed}");
    let body: Value =
        serde_json::from_str(listed["result"]["content"][0]["text"].as_str().unwrap()).unwrap();
    assert_eq!(body["cards"].as_array().expect("pairs").len(), protocol_mcp_gateway::CARDS);
}

fn tool_text(result: &rmcp::model::CallToolResult) -> String {
    result.content[0].as_text().expect("text").text.clone()
}

#[tokio::test]
async fn the_dataset_tools_give_the_bytes_of_the_dataset_routes() {
    let bin = env!("CARGO_BIN_EXE_mcp_stdio");
    let transport =
        TokioChildProcess::new(tokio::process::Command::new(bin)).expect("spawn mcp_stdio");
    let mut client = ().serve(transport).await.expect("initialize");
    let base = spawn_app().await;

    let plan = |store: &str| {
        json!({
            "domain": "energy",
            "card": "thermal",
            "plan": {
                "kind": "grid",
                "base": {
                    "initial": {},
                    "dt_s": 0.1,
                    "segments": [{ "steps": 3, "load": { "ambient_k": 293.15 } }],
                    "record_every": 1
                },
                "axes": [
                    { "path": "initial.temperature_k", "values": [280.0, 290.0] },
                    { "path": "segments[0].load.heat_power_w", "values": [0.0, 5.0] }
                ]
            },
            "store": store,
            "return": "inline", "answer": "full"
        })
    };
    for store in ["frames", "none"] {
        let request = plan(store);
        let (status, http_answer) = post(&base, "/run_dataset", &request).await;
        assert_eq!(status, 200, "{http_answer}");
        let mcp_answer = client
            .call_tool(CallToolRequestParams::new("run_dataset").with_arguments(arguments(&request)))
            .await
            .expect("run_dataset");
        assert_eq!(mcp_answer.is_error, Some(false));
        assert_eq!(tool_text(&mcp_answer), http_answer, "{store}");

        let answer: Value = serde_json::from_str(&http_answer).unwrap();
        let check = json!({
            "head": answer["head"],
            "rows": answer["rows"],
            "frames_hex": answer["frames_hex"]
        });
        let (status, http_report) = post(&base, "/verify_dataset", &check).await;
        assert_eq!(status, 200, "{http_report}");
        let mcp_report = client
            .call_tool(CallToolRequestParams::new("verify_dataset").with_arguments(arguments(&check)))
            .await
            .expect("verify_dataset");
        assert_eq!(mcp_report.is_error, Some(false));
        assert_eq!(tool_text(&mcp_report), http_report, "{store}");
        let report: Value = serde_json::from_str(&http_report).unwrap();
        assert_eq!(report["holds"], true, "{report}");
    }

    // A refusal is the same sentence on both doors. A grid value outside a
    // bound is not a refusal of the job: its row is unadmitted.
    let mut file = plan("frames");
    file["return"] = json!("file");
    let mut unknown = plan("frames");
    unknown["workers"] = json!(4);
    for request in [file, unknown] {
        let (status, http_text) = post(&base, "/run_dataset", &request).await;
        assert_eq!(status, 422, "{http_text}");
        let called = client
            .call_tool(CallToolRequestParams::new("run_dataset").with_arguments(arguments(&request)))
            .await
            .expect("tool result");
        assert_eq!(called.is_error, Some(true));
        assert_eq!(tool_text(&called), http_text);
    }
    let broken = json!({ "head": {}, "rows": [], "frames_hex": null, "extra": 1 });
    let (status, http_text) = post(&base, "/verify_dataset", &broken).await;
    assert_eq!(status, 422, "{http_text}");
    let called = client
        .call_tool(CallToolRequestParams::new("verify_dataset").with_arguments(arguments(&broken)))
        .await
        .expect("tool result");
    assert_eq!(called.is_error, Some(true));
    assert_eq!(tool_text(&called), http_text);

    client.close().await.expect("close");
}

#[tokio::test]
async fn list_cards_and_describe_card_serve_the_thirty_one_pairs() {
    let bin = env!("CARGO_BIN_EXE_mcp_stdio");
    let transport =
        TokioChildProcess::new(tokio::process::Command::new(bin)).expect("spawn mcp_stdio");
    let mut client = ().serve(transport).await.expect("initialize");
    let base = spawn_app().await;

    let listed = client
        .call_tool(CallToolRequestParams::new("list_cards"))
        .await
        .expect("list_cards");
    assert_eq!(listed.is_error, Some(false));
    let listed: Value = serde_json::from_str(&tool_text(&listed)).unwrap();
    assert_eq!(listed.as_object().unwrap().len(), 2, "{listed}");
    assert_eq!(listed["count"], json!(protocol_mcp_gateway::CARDS), "{listed}");
    let cards = listed["cards"].as_array().expect("cards");
    let pairs: Vec<(&str, &str)> = cards
        .iter()
        .map(|entry| {
            // domain, card, against_measurement and specimen.
            assert_eq!(entry.as_object().unwrap().len(), 4, "{entry}");
            assert!(entry["against_measurement"].is_string(), "{entry}");
            assert!(entry["specimen"].as_str().unwrap().ends_with('.'));
            (
                entry["domain"].as_str().expect("domain"),
                entry["card"].as_str().expect("card"),
            )
        })
        .collect();
    // The served list of 0.15.0: one model per real object, each ID in its
    // 0.14.3 place.
    assert_eq!(
        pairs,
        vec![
            ("materials", "ti64_lattice"),
            ("energy", "grid_node"),
            ("energy", "swing"),
            ("energy", "battery_sag"),
            ("energy", "cell_18650pf"),
            ("energy", "thermal"),
            ("signal", "resonance"),
            ("signal", "tesseract"),
            ("materials", "hydraulic_fluid"),
            ("materials", "cable_stretch"),
            ("energy", "generator_mmrtg_f1"),
            ("aerospace", "capsule_stardust"),
            ("aerospace", "satellite_lageos1"),
            ("energy", "reactor_spert3"),
            ("signal", "cantilever_eth"),
            ("energy", "cell_2170_model3"),
            ("materials", "creep_cmsx6"),
            ("aerospace", "capsule_insight"),
            ("aerospace", "crater_ryugu_sci"),
            ("energy", "generator_mmrtg_seebeck"),
            ("aerospace", "momentum_jwst_dihedral"),
            ("aerospace", "momentum_jwst_srp"),
            ("signal", "link_gothenburg_10010"),
            ("energy", "grid_gb"),
            ("aerospace", "solar_system"),
            ("materials", "tire_eve_r19_ice"),
            ("aerospace", "los_jwst_fine_guide"),
            ("energy", "tokamak_jet_dte1"),
            ("materials", "fingerpad_p02_glass"),
            ("energy", "accumulator_sbo450"),
            ("energy", "reactor_triga_jsi"),
            ("energy", "reactor_spert3_ccore"),
            ("energy", "runaway_18650_lco"),
            ("materials", "grasp_lift_glass_4tu"),
            ("energy", "tokamak_vde_mast"),
            ("energy", "reactor_msre_pump"),
        ]
    );
    // The one line of each model compared with measurement is its spec
    // sheet's row: what it computes, the comparison and the gap, in order,
    // with the record's own numbers; since 0.14.4 the range, the fit and the
    // refusals are the description's.
    // By name since the fold of 0.15.0 moved the places.
    let place = |name: &str| cards.iter().position(|card| card["card"] == name).unwrap_or_else(|| panic!("{name} is not listed"));
    let compared = ["generator_mmrtg_f1", "capsule_stardust", "satellite_lageos1", "cantilever_eth", "cell_2170_model3", "creep_cmsx6", "capsule_insight", "crater_ryugu_sci", "grid_gb", "tokamak_jet_dte1", "runaway_18650_lco", "reactor_spert3", "reactor_spert3_ccore", "link_gothenburg_10010", "tire_eve_r19_ice", "reactor_triga_jsi", "reactor_msre_pump"];
    for index in compared.map(place) {
        let line = cards[index]["specimen"].as_str().unwrap();
        let at = |mark: &str| line.find(mark).unwrap_or_else(|| panic!("cards[{index}] has no {mark:?}: {line}"));
        assert!(at(" Compared with ") < at(" Gap: "), "cards[{index}]: {line}");
        assert!(!line.contains(" Range: ") && !line.contains(" Fitted on ") && !line.contains(" Refuses "), "cards[{index}]: {line}");
        for word in ["blind", "held-back", "pass mark", "pass line", "fixed beforehand", " met ", " missed", "failed"] {
            assert!(!line.contains(word), "cards[{index}] says {word:?}: {line}");
        }
    }
    for (name, number) in [("reactor_spert3_ccore", "the 111 left out of the fit"), ("reactor_triga_jsi", "20 table rows"), ("generator_mmrtg_f1", "2,809 to 4,087"), ("creep_cmsx6", "0.093"), ("link_gothenburg_10010", "32.445 GHz"), ("tokamak_jet_dte1", "0.830")] {
        let index = place(name);
        assert!(cards[index]["specimen"].as_str().unwrap().contains(number), "cards[{index}]: {number}");
    }
    assert!(cards[place("reactor_triga_jsi")]["specimen"].as_str().unwrap().contains("one thermocouple of one element"));
    let twin = cards.iter().find(|row| row["card"] == "runaway_18650_lco").expect("the twin is listed");
    assert!(twin["specimen"].as_str().unwrap().contains("onsets within 16.2 K RMS"), "{twin}");

    for (domain, card) in pairs {
        let which = json!({ "domain": domain, "card": card });
        let described = client
            .call_tool(CallToolRequestParams::new("describe_card").with_arguments(arguments(&which)))
            .await
            .expect("describe_card");
        assert_eq!(described.is_error, Some(false), "{domain}/{card}");
        let described: Value = serde_json::from_str(&tool_text(&described)).unwrap();
        assert_eq!(described["domain"], domain);
        assert_eq!(described["card"], card);

        // The worked examples run as they are returned, on both doors.
        let step = &described["example_compute_step"];
        assert_eq!(step["domain"], domain);
        assert_eq!(step["card"], card);
        let (status, http_step) = post(&base, "/verify_physics", step).await;
        assert_eq!(status, 200, "{domain}/{card}: {http_step}");
        let mcp_step = client
            .call_tool(CallToolRequestParams::new("verify_step").with_arguments(arguments(step)))
            .await
            .expect("verify_step");
        assert_eq!(mcp_step.is_error, Some(false), "{domain}/{card}");
        assert_eq!(tool_text(&mcp_step), http_step, "{domain}/{card}");

        let run = &described["example_run_trajectory"];
        assert_eq!(run["domain"], domain);
        assert_eq!(run["card"], card);
        let (status, http_run) = post(&base, "/run_trajectory", run).await;
        assert_eq!(status, 200, "{domain}/{card}: {http_run}");
        let mcp_run = client
            .call_tool(CallToolRequestParams::new("run_trajectory").with_arguments(arguments(run)))
            .await
            .expect("run_trajectory");
        assert_eq!(mcp_run.is_error, Some(false), "{domain}/{card}");
        assert_eq!(tool_text(&mcp_run), http_run, "{domain}/{card}");
        let tape: Value = serde_json::from_str(&http_run).unwrap();
        // Every worked run completes since the fold of 0.15.0: reactor_spert3 is
        // the E-core model, which declares no end, and the C-core model's worked
        // transient, 1750 steps, stops short of its plate maximum (step 2303).
        assert_eq!(tape["outcome"], "completed", "{domain}/{card}");
    }

    client.close().await.expect("close");
}

#[tokio::test]
async fn an_old_domain_pair_is_not_recognized() {
    let bin = env!("CARGO_BIN_EXE_mcp_stdio");
    let transport =
        TokioChildProcess::new(tokio::process::Command::new(bin)).expect("spawn mcp_stdio");
    let mut client = ().serve(transport).await.expect("initialize");
    for (domain, card) in [
        ("thermal", "thermal"),
        ("mechanical", "resonance"),
        ("inertial", "tesseract"),
        ("hydraulics", "hydraulic_fluid"),
        ("structures", "cable_stretch"),
    ] {
        let which = json!({ "domain": domain, "card": card });
        let described = client
            .call_tool(CallToolRequestParams::new("describe_card").with_arguments(arguments(&which)))
            .await
            .expect("tool result");
        assert_eq!(described.is_error, Some(true), "{domain}/{card}");
        assert_eq!(tool_text(&described), "Card not recognized (UnknownCard).", "{domain}/{card}");
    }
    for retired in ["list_orchestrators", "describe_orchestrator"] {
        let called = client
            .call_tool(CallToolRequestParams::new(retired))
            .await
            .expect("tool result");
        assert_eq!(called.is_error, Some(true), "{retired}");
        assert_eq!(tool_text(&called), format!("Tool not recognized: {retired}"));
    }
    client.close().await.expect("close");
}

#[tokio::test]
async fn the_old_key_orchestrator_is_refused_by_name() {
    const REFUSAL: &str = "Physical Violation: 'orchestrator' is not a field of this request.";
    let bin = env!("CARGO_BIN_EXE_mcp_stdio");
    let transport =
        TokioChildProcess::new(tokio::process::Command::new(bin)).expect("spawn mcp_stdio");
    let mut client = ().serve(transport).await.expect("initialize");
    let base = spawn_app().await;

    let step = json!({
        "domain": "energy",
        "orchestrator": "thermal",
        "parameters": {
            "temperature_k": 293.15,
            "heat_power_w": 1.0,
            "ambient_k": 293.15,
            "dt_s": 0.1
        }
    });
    let run = json!({
        "domain": "energy",
        "orchestrator": "thermal",
        "initial": { "temperature_k": 293.15 },
        "dt_s": 0.1,
        "segments": [{
            "steps": 4,
            "load": { "heat_power_w": 1.0, "ambient_k": 293.15 }
        }],
        "record_every": 1
    });
    for (tool, path, body) in [
        ("verify_step", "/verify_physics", &step),
        ("run_trajectory", "/run_trajectory", &run),
    ] {
        let mut beside = body.clone();
        beside["card"] = json!("thermal");
        for request in [body, &beside] {
            let (status, http_text) = post(&base, path, request).await;
            assert_eq!(status, 422, "{path}: {http_text}");
            assert_eq!(http_text, REFUSAL, "{path}");
            let called = client
                .call_tool(CallToolRequestParams::new(tool).with_arguments(arguments(request)))
                .await
                .expect("tool result");
            assert_eq!(called.is_error, Some(true), "{tool}");
            assert_eq!(tool_text(&called), REFUSAL, "{tool}");
        }
    }

    // The key is refused in place of `card` and beside it.
    for which in [
        json!({ "domain": "energy", "orchestrator": "thermal" }),
        json!({ "domain": "energy", "card": "thermal", "orchestrator": "thermal" }),
    ] {
        let called = client
            .call_tool(CallToolRequestParams::new("describe_card").with_arguments(arguments(&which)))
            .await
            .expect("tool result");
        assert_eq!(called.is_error, Some(true), "{which}");
        assert_eq!(tool_text(&called), REFUSAL, "{which}");
    }

    client.close().await.expect("close");
}

#[tokio::test]
async fn a_number_written_as_non_zero_that_reads_as_zero_is_refused_by_name() {
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
    use tokio::process::Command;

    let bin = env!("CARGO_BIN_EXE_mcp_stdio");
    let mut child = Command::new(bin)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .kill_on_drop(true)
        .spawn()
        .expect("spawn mcp_stdio");
    let mut stdin = child.stdin.take().unwrap();
    let mut reader = BufReader::new(child.stdout.take().unwrap());

    async fn exchange(
        stdin: &mut tokio::process::ChildStdin,
        reader: &mut BufReader<tokio::process::ChildStdout>,
        line: &str,
    ) -> Value {
        stdin.write_all(line.as_bytes()).await.unwrap();
        stdin.write_all(b"\n").await.unwrap();
        stdin.flush().await.unwrap();
        let mut response = String::new();
        tokio::time::timeout(Duration::from_secs(30), reader.read_line(&mut response))
            .await
            .expect("the agent door stayed silent")
            .unwrap();
        serde_json::from_str(response.trim()).expect(&response)
    }

    let hello = exchange(
        &mut stdin,
        &mut reader,
        r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"checks","version":"1"}}}"#,
    )
    .await;
    assert_eq!(hello["id"], 1, "{hello}");
    stdin
        .write_all(b"{\"jsonrpc\":\"2.0\",\"method\":\"notifications/initialized\"}\n")
        .await
        .unwrap();
    stdin.flush().await.unwrap();

    let base = spawn_app().await;
    let client = reqwest::Client::new();
    let step = |heat: &str, dt: &str| {
        format!(
            r#"{{"domain":"energy","card":"thermal","parameters":{{"temperature_k":293.15,"heat_power_w":{heat},"ambient_k":293.15,"dt_s":{dt}}}}}"#
        )
    };
    let run = |temperature: &str, heat: &str, dt: &str| {
        format!(
            r#"{{"domain":"energy","card":"thermal","initial":{{"temperature_k":{temperature}}},"dt_s":{dt},"segments":[{{"steps":2,"load":{{"heat_power_w":{heat},"ambient_k":293.15}}}}],"record_every":1}}"#
        )
    };

    // A genuine tape and a genuine solution, with one input respelled.
    let (status, tape) = post(
        &base,
        "/run_trajectory",
        &serde_json::from_str(&run("293.15", "0.0", "0.1")).unwrap(),
    )
    .await;
    assert_eq!(status, 200, "{tape}");
    let forged_tape = tape.replacen("\"heat_power_w\":0.0", "\"heat_power_w\":1e-400", 1);
    assert_ne!(forged_tape, tape);
    let (status, solution) = post(
        &base,
        "/solve",
        &json!({ "solver": "atom_dft", "parameters": { "atomic_number": 1 } }),
    )
    .await;
    assert_eq!(status, 200, "{solution}");
    let forged_solution =
        solution.replacen("\"atomic_number\":1}", "\"atomic_number\":1e-400}", 1);
    assert_ne!(forged_solution, solution);

    let cases = [
        ("verify_step", "/verify_physics", step("1e-400", "0.1"), "heat_power_w", "1e-400"),
        ("verify_step", "/verify_physics", step("-1e-400", "0.1"), "heat_power_w", "-1e-400"),
        ("verify_step", "/verify_physics", step("1.0", "1e-400"), "dt_s", "1e-400"),
        ("run_trajectory", "/run_trajectory", run("293.15", "1e-400", "0.1"), "heat_power_w", "1e-400"),
        ("run_trajectory", "/run_trajectory", run("1e-400", "1.0", "0.1"), "temperature_k", "1e-400"),
        ("run_trajectory", "/run_trajectory", run("293.15", "1.0", "0.1e-400"), "dt_s", "0.1e-400"),
        (
            "solve",
            "/solve",
            r#"{"solver":"atom_dft","parameters":{"atomic_number":1e-400}}"#.to_string(),
            "atomic_number",
            "1e-400",
        ),
        ("verify_tape", "/verify_tape", forged_tape, "heat_power_w", "1e-400"),
        ("verify_solution", "/verify_solution", forged_solution, "atomic_number", "1e-400"),
    ];
    let mut id = 10;
    for (tool, path, body, field, token) in cases {
        let refusal = format!(
            "Physical Violation: '{field}' is written {token}, a number that is not zero and reads as zero."
        );
        let response = client
            .post(format!("{base}{path}"))
            .header(reqwest::header::CONTENT_TYPE, "application/json")
            .body(body.clone())
            .send()
            .await
            .unwrap();
        assert_eq!(response.status().as_u16(), 422, "{path} {field}");
        assert_eq!(response.text().await.unwrap(), refusal, "{path} {field}");

        id += 1;
        let line = format!(
            r#"{{"jsonrpc":"2.0","id":{id},"method":"tools/call","params":{{"name":"{tool}","arguments":{body}}}}}"#
        );
        let message = exchange(&mut stdin, &mut reader, &line).await;
        assert_eq!(message["id"], id, "{tool} {field}: {message}");
        assert_eq!(message["result"]["isError"], true, "{tool} {field}: {message}");
        assert_eq!(message["result"]["content"][0]["text"], refusal, "{tool} {field}");
    }

    // Every spelling of zero is still a zero, on both doors.
    for zero in ["0", "0.0", "-0.0", "0e0"] {
        let body = step(zero, "0.1");
        let response = client
            .post(format!("{base}/verify_physics"))
            .header(reqwest::header::CONTENT_TYPE, "application/json")
            .body(body.clone())
            .send()
            .await
            .unwrap();
        assert_eq!(response.status().as_u16(), 200, "{zero}");
        let http_text = response.text().await.unwrap();
        id += 1;
        let line = format!(
            r#"{{"jsonrpc":"2.0","id":{id},"method":"tools/call","params":{{"name":"verify_step","arguments":{body}}}}}"#
        );
        let message = exchange(&mut stdin, &mut reader, &line).await;
        assert_eq!(message["result"]["isError"], false, "{zero}: {message}");
        assert_eq!(message["result"]["content"][0]["text"], http_text, "{zero}");
    }
}
