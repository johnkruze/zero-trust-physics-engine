//! The hosted door: the same tools as the stdio door, a key per client, and a
//! view of the engine per key. Every test starts the real binary and talks to
//! it over a real TCP socket on 127.0.0.1.

mod answer_text;

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::atomic::{AtomicUsize, Ordering};

use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStdin, ChildStdout, Command};

static COUNTER: AtomicUsize = AtomicUsize::new(0);

fn scratch(name: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!(
        "hosted_{}_{}_{name}",
        std::process::id(),
        COUNTER.fetch_add(1, Ordering::SeqCst)
    ));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn sha(text: &str) -> String {
    hex::encode(Sha256::digest(text.as_bytes()))
}

const KEY_ALL: &str = "key-for-the-client-that-sees-everything";
const KEY_THERMAL: &str = "key-for-the-thermal-client";
const KEY_MATERIALS: &str = "key-for-the-materials-client";
const KEY_NONE: &str = "key-for-the-client-with-no-card";
const KEY_ONE: &str = "key-for-the-client-with-one-slot";

fn client(label: &str, key: &str, cards: Value, solvers: Value, max: u64) -> Value {
    json!({ "client": label, "key_sha256": sha(key), "cards": cards, "solvers": solvers, "max_in_flight": max, "max_permits": 1 })
}

/// A keys file whose first client sees every card the engine lists. The
/// cards come from the engine, not from a number in this file.
fn keys_file(all_cards: &Value) -> Value {
    json!([
        client("all", KEY_ALL, all_cards.clone(), json!(["atom_dft", "ablator_slab", "journal_bearing_sheffield", "propeller_apc_10x7e", "modulus_bam_s355", "seawater_column", "argon_pair_gas", "fusion_pb11_lhd", "fusion_pb11_thermal", "step_flow_armaly", "culture_trichoderma_ambr250", "tokamak_dt_beam_target", "pipe_flow_superpipe", "blast_wave_tnt_air", "fuel_rod_triga_jsi", "culture_trichoderma_atp", "junction_nist_b7_s2", "water_hammer_oil_line", "orifice_air_chamber", "pneumatic_hose_resonance", "stress_plate_hole_s235", "stress_plate_hole_s235_elastic", "icf_hotspot_burn", "ln2_line_dlr_fttf", "plasma_sheath_ram_c2"]), 8),
        client("thermal", KEY_THERMAL, json!([["energy", "thermal"]]), json!([]), 8),
        client(
            "materials",
            KEY_MATERIALS,
            json!([["materials", "ti64_lattice"], ["materials", "cable_stretch"]]),
            json!(["atom_dft"]),
            8
        ),
        client("none", KEY_NONE, json!([]), json!([]), 8),
        client("one", KEY_ONE, all_cards.clone(), json!(["atom_dft"]), 1),
    ])
}

/// Every pair the engine lists, asked of the stdio door.
async fn engine_pairs(stdio: &mut StdioDoor) -> Vec<(String, String)> {
    let answer = stdio.raw(&tool_line(1, "list_cards", &json!({}))).await;
    let text = result_text(&answer);
    let listed: Value = serde_json::from_str(&text).unwrap();
    listed["cards"]
        .as_array()
        .unwrap()
        .iter()
        .map(|item| {
            (
                item["domain"].as_str().unwrap().to_string(),
                item["card"].as_str().unwrap().to_string(),
            )
        })
        .collect()
}

fn pairs_json(pairs: &[(String, String)]) -> Value {
    Value::Array(pairs.iter().map(|(d, c)| json!([d, c])).collect())
}

// ---------------------------------------------------------------- the doors

struct StdioDoor {
    _child: Child,
    stdin: ChildStdin,
    out: BufReader<ChildStdout>,
}

impl StdioDoor {
    async fn start() -> StdioDoor {
        let mut child = Command::new(env!("CARGO_BIN_EXE_mcp_stdio"))
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .kill_on_drop(true)
            .spawn()
            .unwrap();
        let stdin = child.stdin.take().unwrap();
        let out = BufReader::new(child.stdout.take().unwrap());
        let mut door = StdioDoor { _child: child, stdin, out };
        let init = json!({"jsonrpc":"2.0","id":0,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"test","version":"1"}}});
        door.raw(&init.to_string()).await;
        door.stdin
            .write_all(b"{\"jsonrpc\":\"2.0\",\"method\":\"notifications/initialized\"}\n")
            .await
            .unwrap();
        door
    }

    /// One line in, one line out.
    async fn raw(&mut self, line: &str) -> String {
        self.stdin.write_all(line.as_bytes()).await.unwrap();
        self.stdin.write_all(b"\n").await.unwrap();
        self.stdin.flush().await.unwrap();
        let mut answer = String::new();
        self.out.read_line(&mut answer).await.unwrap();
        answer.trim_end_matches('\n').to_string()
    }
}

struct Hosted {
    _child: Child,
    base: String,
}

impl Hosted {
    async fn start(keys: &Value, dataset_dir: Option<&Path>) -> Hosted {
        let dir = scratch("keys");
        let file = dir.join("keys.json");
        std::fs::write(&file, serde_json::to_string_pretty(keys).unwrap()).unwrap();
        Hosted::start_file(&file, dataset_dir, &[]).await
    }

    async fn start_file(file: &Path, dataset_dir: Option<&Path>, origins: &[&str]) -> Hosted {
        let mut command = Command::new(env!("CARGO_BIN_EXE_hosted_door"));
        // A door with no answer key serves unsigned answers only when told so.
        command.arg("--unsigned");
        command.arg("--port").arg("0").arg("--keys").arg(file);
        if let Some(dir) = dataset_dir {
            command.arg("--dataset-dir").arg(dir);
        }
        for origin in origins {
            command.arg("--allowed-origin").arg(origin);
        }
        let mut child = command
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
        assert!(address.starts_with("127.0.0.1:"), "{address}");
        // Keep reading so the child never blocks on a full pipe.
        tokio::spawn(async move {
            let mut rest = String::new();
            while out.read_line(&mut rest).await.unwrap_or(0) > 0 {
                rest.clear();
            }
        });
        Hosted { _child: child, base: format!("http://{address}") }
    }

    async fn send(&self, key: Option<&str>, body: &str) -> (u16, String) {
        let mut request = reqwest::Client::new()
            .post(format!("{}/mcp", self.base))
            .header("Content-Type", "application/json")
            .header("Accept", "application/json, text/event-stream")
            .header("MCP-Protocol-Version", "2025-06-18")
            .body(body.to_string());
        if let Some(key) = key {
            request = request.header("Authorization", format!("Bearer {key}"));
        }
        let response = request.send().await.unwrap();
        let status = response.status().as_u16();
        (status, response.text().await.unwrap())
    }

    /// The body of the answer, for a call that must be answered with 200.
    async fn ask(&self, key: &str, body: &str) -> String {
        let (status, text) = self.send(Some(key), body).await;
        assert_eq!(status, 200, "{text}");
        text
    }

    async fn tool(&self, key: &str, tool: &str, arguments: &Value) -> String {
        self.ask(key, &tool_line(7, tool, arguments)).await
    }
}

fn tool_line(id: u64, tool: &str, arguments: &Value) -> String {
    answer_text::tool_call(id, tool, arguments)
        .to_string()
}

fn result_text(answer: &str) -> String {
    let value: Value = serde_json::from_str(answer).unwrap();
    let text = value["result"]["content"][0]["text"]
        .as_str()
        .unwrap_or_else(|| panic!("no text in {answer}"))
        .to_string();
    answer_text::remember(&text);
    text
}

fn is_error(answer: &str) -> bool {
    let value: Value = serde_json::from_str(answer).unwrap();
    value["result"]["isError"] == json!(true)
}

const LIST_TOOLS: &str = r#"{"jsonrpc":"2.0","id":1,"method":"tools/list"}"#;

// ---------------------------------------------------------------- requests

struct Specimen {
    domain: String,
    card: String,
    description: Value,
}

impl Specimen {
    fn step(&self) -> Value {
        self.description["example_compute_step"].clone()
    }
    fn run(&self) -> Value {
        self.description["example_run_trajectory"].clone()
    }
    fn dataset(&self) -> Value {
        let mut run = self.run();
        let object = run.as_object_mut().unwrap();
        object.remove("domain");
        object.remove("card");
        json!({
            "domain": self.domain, "card": self.card,
            "plan": { "kind": "list", "runs": [run] },
            "store": "none", "return": "inline", "answer": "full"
        })
    }
    fn validation(&self) -> Value {
        let quantity = self.description["phase_fields"][0]["name"].clone();
        json!({
            "run": self.run(),
            "quantity": quantity,
            "measurements": [{ "step": 1, "value": 1.0 }]
        })
    }
    /// The same request with the pair renamed.
    fn renamed(&self, domain: &str, card: &str) -> Specimen {
        let mut description = self.description.clone();
        for key in ["example_compute_step", "example_run_trajectory"] {
            description[key]["domain"] = json!(domain);
            description[key]["card"] = json!(card);
        }
        Specimen { domain: domain.into(), card: card.into(), description }
    }
}

async fn specimens(hosted: &Hosted, pairs: &[(String, String)]) -> Vec<Specimen> {
    let mut all = Vec::new();
    for (domain, card) in pairs {
        let answer = hosted
            .tool(KEY_ALL, "describe_card", &json!({ "domain": domain, "card": card }))
            .await;
        all.push(Specimen {
            domain: domain.clone(),
            card: card.clone(),
            description: serde_json::from_str(&result_text(&answer)).unwrap(),
        });
    }
    all
}

// ---------------------------------------------------------------- tests

#[tokio::test]
async fn a_key_that_sees_everything_gets_the_bytes_of_the_stdio_door() {
    let mut stdio = StdioDoor::start().await;
    let pairs = engine_pairs(&mut stdio).await;
    assert!(!pairs.is_empty());
    let hosted = Hosted::start(&keys_file(&pairs_json(&pairs)), None).await;
    let all = specimens(&hosted, &pairs).await;

    let mut lines: Vec<String> = vec![
        LIST_TOOLS.to_string(),
        json!({"jsonrpc":"2.0","id":2,"method":"ping"}).to_string(),
        tool_line(3, "list_cards", &json!({})),
        tool_line(4, "list_solvers", &json!({})),
        tool_line(5, "describe_solver", &json!({"solver": "atom_dft"})),
        tool_line(6, "describe_solver", &json!({"solver": "nope"})),
        tool_line(7, "describe_card", &json!({"domain": "energy", "card": "nope"})),
        tool_line(8, "no_such_tool", &json!({})),
        tool_line(9, "solve", &json!({"solver": "atom_dft", "parameters": {"atomic_number": 3}})),
        tool_line(10, "solve", &json!({"solver": "atom_dft", "parameters": {"atomic_number": 99}})),
        tool_line(11, "verify_receipt", &json!({"response_text": "no receipt"})),
        tool_line(12, "verify_dataset", &json!({"dataset_seal": "ab".repeat(48)})),
        tool_line(13, "verify_step", &json!({"domain": "energy", "card": "nope", "parameters": {}})),
    ];
    for specimen in &all {
        let (d, c) = (&specimen.domain, &specimen.card);
        lines.push(tool_line(20, "describe_card", &json!({"domain": d, "card": c})));
        lines.push(tool_line(21, "verify_step", &specimen.step()));
        lines.push(tool_line(22, "run_trajectory", &specimen.run()));
        lines.push(tool_line(23, "run_dataset", &specimen.dataset()));
        lines.push(tool_line(24, "validate_series", &specimen.validation()));
        // A tape of the card's own run, and the receipt of its step.
        let run_text = result_text(&stdio.raw(&tool_line(25, "run_trajectory", &specimen.run())).await);
        lines.push(tool_line(26, "verify_tape", &serde_json::from_str::<Value>(&run_text).unwrap()));
        let step_text = result_text(&stdio.raw(&tool_line(27, "verify_step", &specimen.step())).await);
        lines.push(tool_line(28, "verify_receipt", &json!({"response_text": step_text})));
    }
    let solved = result_text(
        &stdio
            .raw(&tool_line(30, "solve", &json!({"solver": "atom_dft", "parameters": {"atomic_number": 2}})))
            .await,
    );
    lines.push(tool_line(31, "verify_solution", &serde_json::from_str::<Value>(&solved).unwrap()));
    let dataset = result_text(&stdio.raw(&tool_line(32, "run_dataset", &all[0].dataset())).await);
    let dataset: Value = serde_json::from_str(&dataset).unwrap();
    lines.push(tool_line(
        33,
        "verify_dataset",
        &json!({"head": dataset["head"], "rows": dataset["rows"], "frames_hex": dataset["frames_hex"]}),
    ));

    // What the stdio door answers by itself, before the library reads a line.
    lines.extend(
        [
            r#"{"jsonrpc":"2.0","id":40,"method":"tools/call","params":{"name":"verify_step","arguments":{"domain":"energy","domain":"energy"}}}"#,
            r#"{"jsonrpc":"2.0","id":41,"method":"tools/call","params":{"name":"verify_step","arguments":{"domain":"energy","card":"thermal","parameters":{"temperature_k":1e-400}}}}"#,
            r#"{"jsonrpc":"2.0","id":42,"method":"tools/call","params":{"name":"verify_step","arguments":{"domain":"#,
            r#"{"jsonrpc":"2.0","id":1.5,"method":"tools/list"}"#,
            r#"{"jsonrpc":"2.0","id":1e3,"method":"tools/list"}"#,
            r#"{"jsonrpc":"2.0","id":-0,"method":"tools/list"}"#,
            r#"{"jsonrpc":"2.0","id":99999999999999999999,"method":"tools/list"}"#,
            r#"{"jsonrpc":"2.0","id":null,"method":"tools/list"}"#,
            r#"{"jsonrpc":"2.0","id":true,"method":"tools/list"}"#,
            r#"{"jsonrpc":"2.0","id":[1],"method":"tools/list"}"#,
            r#"{"jsonrpc":"2.0","id":{"a":1},"method":"tools/list"}"#,
            r#"{"jsonrpc":"2.0","id":"s","id":"t","method":"tools/list"}"#,
            r#"{"jsonrpc":"2.0","id":43,"method":"tools/call","params":{"name":"solve","arguments":{"solver":"atom_dft","parameters":{"atomic_number":1,"atomic_number":2}}}}"#,
            // A tools/call with wrong parameters: the same specific sentence on both doors.
            r#"{"jsonrpc":"2.0","id":50,"method":"tools/call","params":{"name":"verify_step","arguments":"x"}}"#,
            r#"{"jsonrpc":"2.0","id":51,"method":"tools/call","params":{"arguments":{}}}"#,
            r#"{"jsonrpc":"2.0","id":52,"method":"tools/call","params":{"name":7,"arguments":{}}}"#,
            r#"{"jsonrpc":"2.0","id":53,"method":"tools/call","params":null}"#,
            r#"{"jsonrpc":"2.0","id":54,"method":"tools/call"}"#,
            r#"not json at all"#,
            r#"{"id":44,"x":}"#,
        ]
        .map(String::from),
    );
    // Over the size limit of a tool whose route takes 2 MiB.
    lines.push(format!(
        r#"{{"jsonrpc":"2.0","id":45,"method":"tools/call","params":{{"name":"verify_step","arguments":{{"pad":"{}"}}}}}}"#,
        "x".repeat(2 * 1024 * 1024 + 10)
    ));

    let mut compared = 0;
    for line in &lines {
        let expected = stdio.raw(line).await;
        let (status, body) = hosted.send(Some(KEY_ALL), line).await;
        assert!(status == 200 || status == 400 || status == 413, "{status} {body}");
        assert_eq!(body, expected, "for the line {}", &line[..line.len().min(200)]);
        compared += 1;
    }
    assert!(compared > 100, "{compared}");

    // The sentence is the specific one, not the generic one.
    let (_, body) = hosted
        .send(
            Some(KEY_ALL),
            r#"{"jsonrpc":"2.0","id":50,"method":"tools/call","params":{"name":"verify_step","arguments":"x"}}"#,
        )
        .await;
    assert!(body.contains("-32602") && body.contains("'arguments' must be an object of named values"), "{body}");
    assert!(!body.contains("tools/call needs a 'name' string"), "{body}");

    // The 16 MiB limit of the stdio door, and the same on the hosted door.
    let wide = format!(
        r#"{{"jsonrpc":"2.0","id":46,"method":"tools/call","params":{{"name":"run_trajectory","arguments":{{"pad":"{}"}}}}}}"#,
        "x".repeat(16 * 1024 * 1024)
    );
    let expected = stdio.raw(&wide).await;
    let (status, body) = hosted.send(Some(KEY_ALL), &wide).await;
    assert_eq!(status, 413);
    assert_eq!(body, expected);
    assert!(body.contains("Request too large"), "{body}");
}

/// A name of the same length as `name`, which the engine does not have.
fn invented(name: &str) -> String {
    "q".repeat(name.len())
}

/// The words that only the hidden cards carry: their names, the field names
/// no visible card has, and the words of their specimen that no visible text
/// has.
fn forbidden_words(visible: &[&Specimen], hidden: &[&Specimen], neutral_text: &str) -> BTreeSet<String> {
    let words = |text: &str| -> BTreeSet<String> {
        text.to_lowercase()
            .split(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
            .filter(|word| word.len() >= 4)
            .map(String::from)
            .collect()
    };
    let mut seen = words(neutral_text);
    // A card's state carries its ledger under names shared by many cards,
    // and a visible card's own answers send them. A hidden card that takes
    // one as an input field reveals nothing by it.
    for name in ["energy_anchor_j", "boundary_work_j", "energy_defect_j"] {
        seen.insert(name.to_string());
    }
    for card in visible {
        seen.extend(words(&card.description.to_string()));
        seen.insert(card.card.to_lowercase());
    }
    let mut forbidden = BTreeSet::new();
    for card in hidden {
        forbidden.insert(card.card.to_lowercase());
        for part in ["phase_fields", "load_fields"] {
            for field in card.description[part].as_array().unwrap() {
                let name = field["name"].as_str().unwrap().to_lowercase();
                // A field name is looked for whole, however short.
                let named_in_neutral = neutral_text
                    .to_lowercase()
                    .split(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
                    .any(|word| word == name);
                if !seen.contains(&name) && !named_in_neutral {
                    forbidden.insert(name);
                }
            }
        }
        for word in words(card.description["specimen"].as_str().unwrap()) {
            if !seen.contains(&word) {
                forbidden.insert(word);
            }
        }
    }
    forbidden
}

#[tokio::test]
async fn a_hidden_card_answers_as_a_card_that_does_not_exist_on_every_tool() {
    let mut stdio = StdioDoor::start().await;
    let pairs = engine_pairs(&mut stdio).await;
    let hosted = Hosted::start(&keys_file(&pairs_json(&pairs)), None).await;
    let all = specimens(&hosted, &pairs).await;

    let mut combinations = 0;
    let mut cards_checked = 0;
    let mut session: Vec<String> = Vec::new();
    let mut visible_answers: Vec<String> = Vec::new();
    for (key, shown) in [
        (KEY_THERMAL, vec![("energy", "thermal")]),
        (KEY_MATERIALS, vec![("materials", "ti64_lattice"), ("materials", "cable_stretch")]),
        (KEY_NONE, vec![]),
    ] {
        let sees = |item: &&Specimen| shown.contains(&(item.domain.as_str(), item.card.as_str()));
        let visible: Vec<&Specimen> = all.iter().filter(sees).collect();
        let hidden: Vec<&Specimen> = all.iter().filter(|item| !sees(item)).collect();
        assert!(!hidden.is_empty());

        // The tools as this key is shown them.
        let listed = hosted.ask(key, LIST_TOOLS).await;
        let tools: Value = serde_json::from_str(&listed).unwrap();
        assert_eq!(tools["result"]["tools"].as_array().unwrap().len(), 18);
        session.push(listed.clone());
        let cards = hosted.tool(key, "list_cards", &json!({})).await;
        let named: Value = serde_json::from_str(&result_text(&cards)).unwrap();
        assert_eq!(named["cards"].as_array().unwrap().len(), visible.len());
        session.push(cards);
        for item in &visible {
            let described = hosted
                .tool(key, "describe_card", &json!({"domain": item.domain, "card": item.card}))
                .await;
            assert_eq!(result_text(&described), item.description.to_string());
            session.push(described);
            session.push(hosted.tool(key, "verify_step", &item.step()).await);
            session.push(hosted.tool(key, "run_trajectory", &item.run()).await);
            session.push(hosted.tool(key, "run_dataset", &item.dataset()).await);
            session.push(hosted.tool(key, "validate_series", &item.validation()).await);
            // The visible card's own state fields are its words: thermal
            // returns energy_anchor_j and boundary_work_j as the 10 Hz mount
            // now takes them.
            let own_step = hosted.tool(key, "verify_step", &item.step()).await;
            visible_answers.push(own_step);
        }

        for item in &hidden {
            cards_checked += 1;
            let fake_card = invented(&item.card);
            let fakes = [
                item.renamed(&item.domain, &fake_card),
                item.renamed(&invented(&item.domain), &fake_card),
            ];
            for fake in &fakes {
                let mut calls: Vec<(&str, Value, Value)> = vec![
                    (
                        "describe_card",
                        json!({"domain": item.domain, "card": item.card}),
                        json!({"domain": fake.domain, "card": fake.card}),
                    ),
                    ("verify_step", item.step(), fake.step()),
                    ("run_trajectory", item.run(), fake.run()),
                    ("run_dataset", item.dataset(), fake.dataset()),
                    ("validate_series", item.validation(), fake.validation()),
                ];
                // The refusals that come before the pair is looked up stay first.
                let mut bad_key = item.step();
                bad_key["surprise"] = json!(1);
                let mut bad_key_fake = fake.step();
                bad_key_fake["surprise"] = json!(1);
                calls.push(("verify_step", bad_key, bad_key_fake));
                let mut bad_number = item.step();
                bad_number["parameters"]["dt_s"] = json!(-1.0);
                let mut bad_number_fake = fake.step();
                bad_number_fake["parameters"]["dt_s"] = json!(-1.0);
                calls.push(("verify_step", bad_number, bad_number_fake));
                // A tape of the hidden card's own run, and the same tape under an invented name.
                let tape_text = result_text(&stdio.raw(&tool_line(1, "run_trajectory", &item.run())).await);
                let tape: Value = serde_json::from_str(&tape_text).unwrap();
                let mut renamed = tape.clone();
                renamed["inputs"]["domain"] = json!(fake.domain);
                renamed["inputs"]["card"] = json!(fake.card);
                calls.push(("verify_tape", tape, renamed));
                for (tool, hidden_call, fake_call) in calls {
                    let on_hidden = hosted.tool(key, tool, &hidden_call).await;
                    let on_fake = hosted.tool(key, tool, &fake_call).await;
                    assert_eq!(on_hidden, on_fake, "{tool} {} {}", item.domain, item.card);
                    combinations += 1;
                    session.push(on_hidden);
                }
            }
            // The plain answer, once, so the test shows what it compares.
            let plain = hosted
                .tool(key, "describe_card", &json!({"domain": item.domain, "card": item.card}))
                .await;
            assert!(is_error(&plain));
            assert_eq!(result_text(&plain), "Card not recognized (UnknownCard).");
        }

        // No hidden name, field or specimen word in anything the key was sent.
        // Every run answer and every comparison says where its loads and its
        // measurements came from in the engine's own words, whichever card it is of.
        let neutral = format!(
            "{} {} {}",
            hosted.ask(KEY_NONE, LIST_TOOLS).await,
            genesis_seal::attest::CALLERS_NUMBER,
            genesis_seal::attest::CALLERS_MEASUREMENTS
        );
        let forbidden = forbidden_words(&visible, &hidden, &format!("{neutral}\n{}", visible_answers.join("\n")));
        assert!(forbidden.len() >= hidden.len(), "{forbidden:?}");
        let text = session.join("\n").to_lowercase();
        let sent: BTreeSet<&str> = text
            .split(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
            .collect();
        for word in &forbidden {
            assert!(!sent.contains(word.as_str()), "{key}: the word '{word}' of a hidden card was sent");
        }
        // A card's name is looked for as text too, not only as a whole word.
        for item in &hidden {
            assert!(!text.contains(&item.card.to_lowercase()), "{key}: the name {} was sent", item.card);
        }
        session.clear();
        visible_answers.clear();
    }
    // Eight comparisons for each of two invented names, for every hidden card of three keys.
    assert_eq!(combinations, cards_checked * 2 * 8);
    assert!(cards_checked >= 3 * (pairs.len() - 2), "{cards_checked}");
    println!("hidden equals invented: {combinations} comparisons over {cards_checked} key and hidden-card pairs");

    // The empty list gets the tools, with no variants and no pair named.
    let listed = hosted.ask(KEY_NONE, LIST_TOOLS).await;
    assert!(!listed.contains("\"enum\":[\"energy"), "{listed}");
    assert!(!listed.contains("energy/"), "{listed}");
    let cards = hosted.tool(KEY_NONE, "list_cards", &json!({})).await;
    assert_eq!(result_text(&cards), r#"{"count":0,"cards":[]}"#);

    // A solver outside the list is a solver the engine does not have.
    let hidden = hosted.tool(KEY_THERMAL, "describe_solver", &json!({"solver": "atom_dft"})).await;
    let unknown = hosted.tool(KEY_THERMAL, "describe_solver", &json!({"solver": "atom_dfx"})).await;
    assert_eq!(hidden, unknown);
    assert_eq!(result_text(&hidden), "Solver not recognized (UnknownSolver).");
    let atom = json!({"solver": "atom_dft", "parameters": {"atomic_number": 3}});
    let other = json!({"solver": "atom_dfx", "parameters": {"atomic_number": 3}});
    assert_eq!(
        hosted.tool(KEY_THERMAL, "solve", &atom).await,
        hosted.tool(KEY_THERMAL, "solve", &other).await
    );
    let listed = hosted.tool(KEY_THERMAL, "list_solvers", &json!({})).await;
    assert_eq!(result_text(&listed), r#"{"count":0,"solvers":[]}"#);
    let tools = hosted.ask(KEY_THERMAL, LIST_TOOLS).await;
    assert!(!tools.contains("atom_dft"));
    // A solution of the hidden solver is checked as a solution of a solver the engine lacks.
    let solved = result_text(
        &stdio
            .raw(&tool_line(1, "solve", &json!({"solver": "atom_dft", "parameters": {"atomic_number": 2}})))
            .await,
    );
    let solution: Value = serde_json::from_str(&solved).unwrap();
    let mut renamed = solution.clone();
    renamed["inputs"]["solver"] = json!("atom_dfx");
    let on_hidden = hosted.tool(KEY_THERMAL, "verify_solution", &solution).await;
    let on_unknown = hosted.tool(KEY_THERMAL, "verify_solution", &renamed).await;
    assert_eq!(on_hidden, on_unknown);
    // A key that sees the solver gets the verdict.
    let verdict = hosted.tool(KEY_ALL, "verify_solution", &solution).await;
    assert_eq!(result_text(&verdict), format!(r#"{{"holds":true,"values_hold":true,"cause":null,"compared":{}}}"#, serde_json::json!(protocol_mcp_gateway::WHOLE_SOLUTION_COMPUTED)));
}

#[tokio::test]
async fn two_keys_see_their_own_cards_and_their_own_datasets() {
    let mut stdio = StdioDoor::start().await;
    let pairs = engine_pairs(&mut stdio).await;
    let store = scratch("datasets");
    let hosted = Hosted::start(&keys_file(&pairs_json(&pairs)), Some(&store)).await;
    let all = specimens(&hosted, &pairs).await;
    let thermal = all.iter().find(|item| item.card == "thermal").unwrap();
    let titanium = all.iter().find(|item| item.card == "ti64_lattice").unwrap();

    // Disjoint lists: each sees its own card and not the other's.
    let own = hosted.tool(KEY_THERMAL, "verify_step", &thermal.step()).await;
    assert!(!is_error(&own), "{own}");
    let other = hosted.tool(KEY_THERMAL, "verify_step", &titanium.step()).await;
    assert_eq!(result_text(&other), "Card not recognized (UnknownCard).");
    let own = hosted.tool(KEY_MATERIALS, "verify_step", &titanium.step()).await;
    assert!(!is_error(&own), "{own}");
    let other = hosted.tool(KEY_MATERIALS, "verify_step", &thermal.step()).await;
    assert_eq!(result_text(&other), "Card not recognized (UnknownCard).");

    // The same request gives the same seals under two keys, and on the stdio door.
    let a = hosted.tool(KEY_ALL, "verify_step", &thermal.step()).await;
    let b = hosted.tool(KEY_THERMAL, "verify_step", &thermal.step()).await;
    assert_eq!(a, b);
    let a = hosted.tool(KEY_ALL, "run_trajectory", &thermal.run()).await;
    let b = hosted.tool(KEY_THERMAL, "run_trajectory", &thermal.run()).await;
    assert_eq!(a, b);
    let direct = stdio.raw(&tool_line(7, "run_trajectory", &thermal.run())).await;
    assert_eq!(a, direct);

    // A dataset stored under one key is not found under another.
    let mut plan = thermal.dataset();
    plan["return"] = json!("file");
    plan["store"] = json!("frames");
    let stored = hosted.tool(KEY_ALL, "run_dataset", &plan).await;
    assert!(!is_error(&stored), "{stored}");
    let stored: Value = serde_json::from_str(&result_text(&stored)).unwrap();
    let seal = stored["head"]["dataset_seal"].as_str().unwrap().to_string();
    let found = hosted.tool(KEY_ALL, "verify_dataset", &json!({"dataset_seal": seal})).await;
    assert!(!is_error(&found), "{found}");
    assert_eq!(serde_json::from_str::<Value>(&result_text(&found)).unwrap()["holds"], true);
    let other = hosted.tool(KEY_THERMAL, "verify_dataset", &json!({"dataset_seal": seal})).await;
    let missing = hosted
        .tool(KEY_THERMAL, "verify_dataset", &json!({"dataset_seal": "ab".repeat(48)}))
        .await;
    assert!(is_error(&other));
    assert_eq!(
        result_text(&other),
        "no dataset is stored under that seal. A dataset returned in the answer (return \"inline\") is not stored: verify it by sending its run_dataset request as 'request' with its 'dataset_seal', which runs the plan again, or by sending back the head, rows and frames_hex of its full answer (answer \"full\")."
    );
    assert_eq!(other, missing);

    // The same plan under the second key stores the same dataset in the second key's folder.
    let again = hosted.tool(KEY_THERMAL, "run_dataset", &plan).await;
    let again: Value = serde_json::from_str(&result_text(&again)).unwrap();
    assert_eq!(again["head"]["dataset_seal"], stored["head"]["dataset_seal"]);
    let found = hosted.tool(KEY_THERMAL, "verify_dataset", &json!({"dataset_seal": seal})).await;
    assert!(!is_error(&found), "{found}");

    // The folders are named by a hash, never by the label.
    let folders: Vec<String> = std::fs::read_dir(&store)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    assert!(folders.contains(&sha(&sha(KEY_ALL))), "{folders:?}");
    assert!(folders.contains(&sha(&sha(KEY_THERMAL))), "{folders:?}");
    for folder in &folders {
        assert!(!["all", "thermal", "materials", "none", "one"].contains(&folder.as_str()));
    }
    assert!(store.join(sha(&sha(KEY_ALL))).join(&seal).is_dir());
    assert!(!store.join(sha(&sha(KEY_MATERIALS))).join(&seal).exists());
}

#[tokio::test]
async fn a_request_without_a_known_key_is_a_401_with_one_body() {
    let mut stdio = StdioDoor::start().await;
    let pairs = engine_pairs(&mut stdio).await;
    let hosted = Hosted::start(&keys_file(&pairs_json(&pairs)), None).await;
    let body = tool_line(1, "list_cards", &json!({}));
    let mut seen = Vec::new();
    let client = reqwest::Client::new();
    let headers: Vec<Vec<(&str, String)>> = vec![
        vec![],
        vec![("Authorization", format!("Basic {KEY_ALL}"))],
        vec![("Authorization", KEY_ALL.to_string())],
        vec![("Authorization", "Bearer ".to_string())],
        vec![("Authorization", "Bearer".to_string())],
        vec![("Authorization", "Bearer not-a-key".to_string())],
        vec![("Authorization", format!("Bearer {}", &KEY_ALL[..KEY_ALL.len() - 1]))],
        vec![("Authorization", format!("Bearer {KEY_ALL}x"))],
        vec![("Authorization", format!("Bearer {}", KEY_ALL.to_uppercase()))],
        vec![("Authorization", format!("Bearer {KEY_ALL} {KEY_ALL}"))],
        vec![("Authorization", sha(KEY_ALL))],
        vec![("Authorization", format!("Bearer {}", sha(KEY_ALL)))],
        vec![("Authorization", format!("Bearer {KEY_ALL}")), ("Authorization", format!("Bearer {KEY_THERMAL}"))],
    ];
    for set in headers {
        let mut request = client
            .post(format!("{}/mcp", hosted.base))
            .header("Content-Type", "application/json")
            .body(body.clone());
        for (name, value) in &set {
            request = request.header(*name, value);
        }
        let response = request.send().await.unwrap();
        let status = response.status().as_u16();
        let www = response.headers().get("www-authenticate").map(|v| v.to_str().unwrap().to_string());
        let kind = response.headers().get("content-type").map(|v| v.to_str().unwrap().to_string());
        let text = response.text().await.unwrap();
        seen.push((status, www, kind, text));
    }
    assert!(seen.iter().all(|one| one.0 == 401), "{seen:?}");
    assert!(seen.iter().all(|one| *one == seen[0]), "{seen:?}");
    assert_eq!(seen[0].3, protocol_mcp_gateway::hosted::UNAUTHORIZED_BODY);
    // A wrong method, a wrong content type and a bad body are 401 too, without a key.
    for request in [
        client.get(format!("{}/mcp", hosted.base)),
        client.delete(format!("{}/mcp", hosted.base)),
        client.post(format!("{}/mcp", hosted.base)).body("not json"),
        client.post(format!("{}/mcp", hosted.base)).header("Content-Type", "text/plain").body("x"),
    ] {
        let response = request.send().await.unwrap();
        assert_eq!(response.status().as_u16(), 401);
        assert_eq!(response.text().await.unwrap(), seen[0].3);
    }
    // The key itself works.
    let (status, _) = hosted.send(Some(KEY_ALL), &body).await;
    assert_eq!(status, 200);
}

#[tokio::test]
async fn the_endpoint_follows_the_streamable_http_rules_of_a_server_that_never_streams() {
    let mut stdio = StdioDoor::start().await;
    let pairs = engine_pairs(&mut stdio).await;
    let hosted = Hosted::start(&keys_file(&pairs_json(&pairs)), None).await;
    let url = format!("{}/mcp", hosted.base);
    let http = reqwest::Client::new();
    let bearer = format!("Bearer {KEY_ALL}");

    // initialize: the version asked for comes back, the header is not needed, and no session is made.
    for asked in protocol_mcp_gateway::hosted::SUPPORTED_VERSIONS.into_iter().filter(|version| protocol_mcp_gateway::hosted::begins_with_initialize(version)) {
        let init = json!({"jsonrpc":"2.0","id":"a","method":"initialize","params":{"protocolVersion":asked,"capabilities":{},"clientInfo":{"name":"t","version":"1"}}});
        let response = http
            .post(&url)
            .header("Authorization", &bearer)
            .header("Content-Type", "application/json")
            .header("Accept", "application/json, text/event-stream")
            .body(init.to_string())
            .send()
            .await
            .unwrap();
        assert_eq!(response.status().as_u16(), 200);
        assert_eq!(response.headers()["content-type"], "application/json");
        assert!(response.headers().get("mcp-session-id").is_none());
        let body: Value = serde_json::from_str(&response.text().await.unwrap()).unwrap();
        assert_eq!(body["id"], "a");
        assert_eq!(body["result"]["protocolVersion"], asked);
        assert_eq!(body["result"]["serverInfo"]["name"], "Zero-Trust Physics");
        assert!(body["result"]["capabilities"]["tools"].is_object());
        assert!(body["result"]["capabilities"].get("resources").is_none());
    }
    // An unknown version is answered with a version the door speaks.
    let init = json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2031-01-01","capabilities":{},"clientInfo":{"name":"t","version":"1"}}});
    let body = hosted.ask(KEY_ALL, &init.to_string()).await;
    let body: Value = serde_json::from_str(&body).unwrap();
    assert_eq!(body["result"]["protocolVersion"], "2025-11-25", "the newest revision that has an initialize: {body}");

    // Notifications and responses are accepted with no body.
    for message in [
        json!({"jsonrpc":"2.0","method":"notifications/initialized"}),
        json!({"jsonrpc":"2.0","method":"notifications/cancelled","params":{"requestId":1}}),
        json!({"jsonrpc":"2.0","id":5,"result":{}}),
    ] {
        let (status, text) = hosted.send(Some(KEY_ALL), &message.to_string()).await;
        assert_eq!((status, text.as_str()), (202, ""), "{message}");
    }
    // A request for something the server does not offer.
    let body = hosted.ask(KEY_ALL, r#"{"jsonrpc":"2.0","id":3,"method":"resources/list"}"#).await;
    assert_eq!(body, r#"{"jsonrpc":"2.0","id":3,"error":{"code":-32601,"message":"Method not found"}}"#);
    // A call with no tool name.
    let body = hosted.ask(KEY_ALL, r#"{"jsonrpc":"2.0","id":4,"method":"tools/call","params":{}}"#).await;
    assert!(body.contains("-32602"), "{body}");
    // A batch is refused (removed in 2025-06-18), and so is a message that is not JSON-RPC 2.0.
    let (status, text) = hosted.send(Some(KEY_ALL), r#"[{"jsonrpc":"2.0","id":1,"method":"ping"}]"#).await;
    assert_eq!(status, 400);
    assert!(text.contains("-32600") && text.contains("batch"), "{text}");
    let (status, text) = hosted.send(Some(KEY_ALL), r#"{"id":1,"method":"ping"}"#).await;
    assert_eq!(status, 400, "{text}");

    // GET and DELETE: no event stream and no session to end.
    for method in [reqwest::Method::GET, reqwest::Method::DELETE, reqwest::Method::PUT] {
        let response = http
            .request(method.clone(), &url)
            .header("Authorization", &bearer)
            .header("Accept", "text/event-stream")
            .send()
            .await
            .unwrap();
        assert_eq!(response.status().as_u16(), 405, "{method}");
        assert_eq!(response.headers()["allow"], "POST");
    }
    // The protocol-version header: a version the door does not speak is a 400.
    let response = http
        .post(&url)
        .header("Authorization", &bearer)
        .header("Content-Type", "application/json")
        .header("MCP-Protocol-Version", "1999-01-01")
        .body(r#"{"jsonrpc":"2.0","id":1,"method":"ping"}"#)
        .send()
        .await
        .unwrap();
    assert_eq!(response.status().as_u16(), 400);
    let refused: Value = serde_json::from_str(&response.text().await.unwrap()).unwrap();
    assert_eq!(refused["id"], 1);
    assert_eq!(refused["error"]["code"], -32022);
    assert_eq!(refused["error"]["data"]["supported"], json!(protocol_mcp_gateway::hosted::SUPPORTED_VERSIONS));
    assert_eq!(refused["error"]["data"]["requested"], "1999-01-01");
    // No version header is read as the earlier revision, and works.
    let response = http
        .post(&url)
        .header("Authorization", &bearer)
        .header("Content-Type", "application/json")
        .body(r#"{"jsonrpc":"2.0","id":1,"method":"ping"}"#)
        .send()
        .await
        .unwrap();
    assert_eq!(response.status().as_u16(), 200);
    assert_eq!(response.text().await.unwrap(), r#"{"jsonrpc":"2.0","id":1,"result":{}}"#);
    // The content type and the accept header.
    let response = http
        .post(&url)
        .header("Authorization", &bearer)
        .header("Content-Type", "text/plain")
        .body("x")
        .send()
        .await
        .unwrap();
    assert_eq!(response.status().as_u16(), 415);
    let response = http
        .post(&url)
        .header("Authorization", &bearer)
        .header("Content-Type", "application/json")
        .header("Accept", "text/event-stream")
        .body(r#"{"jsonrpc":"2.0","id":1,"method":"ping"}"#)
        .send()
        .await
        .unwrap();
    assert_eq!(response.status().as_u16(), 406);
    // A browser origin is refused unless it was named at start.
    let response = http
        .post(&url)
        .header("Authorization", &bearer)
        .header("Content-Type", "application/json")
        .header("Origin", "https://elsewhere.example")
        .body(r#"{"jsonrpc":"2.0","id":1,"method":"ping"}"#)
        .send()
        .await
        .unwrap();
    assert_eq!(response.status().as_u16(), 403);
    // Health, and nothing else, answers without a key.
    let response = http.get(format!("{}/health", hosted.base)).send().await.unwrap();
    assert_eq!(response.status().as_u16(), 200);
    let response = http.get(format!("{}/cards", hosted.base)).send().await.unwrap();
    assert_eq!(response.status().as_u16(), 404);
}

#[tokio::test]
async fn an_origin_named_at_start_is_allowed() {
    let mut stdio = StdioDoor::start().await;
    let pairs = engine_pairs(&mut stdio).await;
    let dir = scratch("origin");
    let file = dir.join("keys.json");
    std::fs::write(&file, keys_file(&pairs_json(&pairs)).to_string()).unwrap();
    let hosted = Hosted::start_file(&file, None, &["https://app.example"]).await;
    let send = |origin: &'static str| {
        let request = reqwest::Client::new()
            .post(format!("{}/mcp", hosted.base))
            .header("Authorization", format!("Bearer {KEY_ALL}"))
            .header("Content-Type", "application/json")
            .header("Origin", origin)
            .body(r#"{"jsonrpc":"2.0","id":1,"method":"ping"}"#);
        async move { request.send().await.unwrap().status().as_u16() }
    };
    assert_eq!(send("https://app.example").await, 200);
    assert_eq!(send("https://APP.example").await, 200);
    assert_eq!(send("https://other.example").await, 403);
}

fn refused_at_start(contents: &str, needle: &str) {
    let dir = scratch("refused");
    let file = dir.join("keys.json");
    std::fs::write(&file, contents).unwrap();
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_hosted_door")).arg("--unsigned")
        .args(["--port", "0", "--keys"])
        .arg(&file)
        .output()
        .unwrap();
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(output.status.code(), Some(2), "{contents} -> {stderr}");
    assert!(stderr.starts_with("The hosted door did not start:"), "{stderr}");
    assert!(stderr.contains(needle), "wanted '{needle}' in {stderr}");
    assert!(output.stdout.is_empty());
    // A refusal never prints a hash.
    assert!(!stderr.contains(&sha("k")));
}

#[test]
fn a_keys_file_with_a_fault_is_refused_at_start_by_name() {
    let good = |label: &str, key: &str| client(label, key, json!([["energy", "thermal"]]), json!([]), 2);
    let with = |value: Value| value.to_string();
    refused_at_start("not json", "not one JSON value");
    refused_at_start("", "not one JSON value");
    refused_at_start("{}", "must be a JSON list");
    refused_at_start("[]", "names no client");
    refused_at_start(&with(json!([1])), "entry 0 must be a JSON object");
    // Two clients with the same hash.
    refused_at_start(&with(json!([good("a", "same"), good("b", "same")])), "'a' and 'b' have the same key_sha256");
    refused_at_start(&with(json!([good("a", "one"), good("a", "two")])), "label 'a' is used twice");
    // A card or solver the engine does not serve.
    refused_at_start(
        &with(json!([client("a", "k", json!([["energy", "nothing"]]), json!([]), 2)])),
        "the engine does not serve the card [\"energy\", \"nothing\"]",
    );
    refused_at_start(
        &with(json!([client("a", "k", json!([]), json!(["atom_xyz"]), 2)])),
        "does not serve the solver \"atom_xyz\"",
    );
    refused_at_start(
        &with(json!([client("a", "k", json!([["energy", "thermal"], ["energy", "thermal"]]), json!([]), 2)])),
        "listed twice",
    );
    refused_at_start(&with(json!([client("a", "k", json!(["energy"]), json!([]), 2)])), "[domain, card]");
    // max_in_flight is required and at least 1.
    refused_at_start(&with(json!([client("a", "k", json!([]), json!([]), 0)])), "max_in_flight");
    let mut missing = good("a", "k");
    missing.as_object_mut().unwrap().remove("max_in_flight");
    refused_at_start(&with(json!([missing])), "'max_in_flight' is missing");
    // The bound of max_in_flight and of max_permits is stated in the sentence that refuses them.
    let mut huge = good("a", "k");
    huge["max_in_flight"] = json!(1_000_001);
    refused_at_start(&with(json!([huge])), "'max_in_flight' must be a whole number from 1 to 1000000.");
    let mut none = good("a", "k");
    none["max_permits"] = json!(0);
    refused_at_start(&with(json!([none])), "'max_permits' must be a whole number from 1 to 1000000.");
    let mut beyond = good("a", "k");
    beyond["max_permits"] = json!(1_000_001);
    refused_at_start(&with(json!([beyond])), "'max_permits' must be a whole number from 1 to 1000000.");
    let mut text = good("a", "k");
    text["max_permits"] = json!("2");
    refused_at_start(&with(json!([text])), "'max_permits' must be a whole number from 1 to 1000000.");
    let mut absent = good("a", "k");
    absent.as_object_mut().unwrap().remove("max_permits");
    refused_at_start(&with(json!([absent])), "client 'a': 'max_permits' is missing.");
    // A share larger than the pool is refused by name, with the pool's size.
    let mut larger = good("a", "k");
    larger["max_permits"] = json!(1_000_000);
    refused_at_start(&with(json!([larger])), "client 'a': 'max_permits' is 1000000, more than the ");
    let mut extra = good("a", "k");
    extra["key"] = json!("the key itself");
    refused_at_start(&with(json!([extra])), "'key' is not a field of a client");
    // The hash is 64 lowercase hex digits.
    let mut short = good("a", "k");
    short["key_sha256"] = json!("abcd");
    refused_at_start(&with(json!([short])), "64 lowercase hex");
    let mut upper = good("a", "k");
    upper["key_sha256"] = json!(sha("k").to_uppercase());
    refused_at_start(&with(json!([upper])), "64 lowercase hex");
    // A repeated key in the file is refused as a request is.
    refused_at_start(
        &format!(
            r#"[{{"client":"a","client":"b","key_sha256":"{}","cards":[],"solvers":[],"max_in_flight":1}}]"#,
            "0".repeat(64)
        ),
        "client",
    );
    // The file is not there.
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_hosted_door")).arg("--unsigned")
        .args(["--port", "0", "--keys", "/nonexistent/keys.json"])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&output.stderr).contains("cannot be read"));
    // A missing or unknown argument.
    for arguments in [
        vec!["--port", "0"],
        vec!["--keys", "x"],
        vec!["--port", "0", "--keys", "x", "--bogus", "1"],
        vec!["--port", "no", "--keys", "x"],
    ] {
        let output = std::process::Command::new(env!("CARGO_BIN_EXE_hosted_door")).arg("--unsigned")
            .args(arguments)
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(2));
    }
}

#[test]
fn the_key_tool_prints_a_fresh_key_and_its_hash() {
    let run = || {
        let output = std::process::Command::new(env!("CARGO_BIN_EXE_hosted_door_key")).output().unwrap();
        assert!(output.status.success());
        assert!(output.stderr.is_empty());
        String::from_utf8(output.stdout).unwrap()
    };
    let (first, second) = (run(), run());
    assert_ne!(first, second);
    let mut lines = first.lines();
    let key = lines.next().unwrap().rsplit(' ').next().unwrap().to_string();
    let hash = lines.next().unwrap().rsplit(' ').next().unwrap().to_string();
    assert_eq!(key.len(), 64);
    assert!(key.bytes().all(|b| b.is_ascii_hexdigit()));
    assert_eq!(hash, sha(&key));
    // A line made from it is accepted by the door.
    let entry = client("made", &key, json!([["energy", "thermal"]]), json!([]), 2);
    let parsed = protocol_mcp_gateway::keys::Keys::parse(json!([entry]).to_string().as_bytes());
    assert!(parsed.is_ok());
}

#[tokio::test]
async fn a_key_with_one_slot_is_answered_busy_while_its_request_runs() {
    let mut stdio = StdioDoor::start().await;
    let pairs = engine_pairs(&mut stdio).await;
    let hosted = Hosted::start(&keys_file(&pairs_json(&pairs)), None).await;
    let all = specimens(&hosted, &pairs).await;
    let thermal = all.iter().find(|item| item.card == "thermal").unwrap();
    // The longest run the door takes.
    let mut long = thermal.run();
    long["segments"] = json!([{ "steps": 1_000_000, "load": long["segments"][0]["load"].clone() }]);
    long["record_every"] = json!(1_000_000);
    let slow = tool_line(1, "run_trajectory", &long);
    let first = {
        let base = hosted.base.clone();
        tokio::spawn(async move {
            reqwest::Client::new()
                .post(format!("{base}/mcp"))
                .header("Authorization", format!("Bearer {KEY_ONE}"))
                .header("Content-Type", "application/json")
                .body(slow)
                .send()
                .await
                .unwrap()
                .text()
                .await
                .unwrap()
        })
    };
    let mut busy = None;
    for _ in 0..200 {
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        let (status, text) = hosted.send(Some(KEY_ONE), &tool_line(2, "list_cards", &json!({}))).await;
        if status == 429 {
            busy = Some(text);
            break;
        }
        if first.is_finished() {
            break;
        }
    }
    assert_eq!(busy.as_deref(), Some("Too many concurrent physics tasks."));
    // Another key is not held back by it.
    let (status, _) = hosted.send(Some(KEY_ALL), &tool_line(3, "list_cards", &json!({}))).await;
    assert_eq!(status, 200);
    let done = first.await.unwrap();
    assert!(!is_error(&done), "{}", &done[..done.len().min(300)]);
    // The slot is free again.
    let (status, _) = hosted.send(Some(KEY_ONE), &tool_line(4, "list_cards", &json!({}))).await;
    assert_eq!(status, 200);
}

/// A standard MCP client (the `rmcp` client over its Streamable HTTP transport)
/// initializes, lists the tools as its key sees them, and calls one.
#[tokio::test]
async fn a_standard_mcp_client_initializes_lists_tools_and_calls_one() {
    use rmcp::ServiceExt;
    use rmcp::model::CallToolRequestParams;
    use rmcp::transport::StreamableHttpClientTransport;
    use rmcp::transport::streamable_http_client::StreamableHttpClientTransportConfig;

    let mut stdio = StdioDoor::start().await;
    let pairs = engine_pairs(&mut stdio).await;
    let hosted = Hosted::start(&keys_file(&pairs_json(&pairs)), None).await;

    let config = StreamableHttpClientTransportConfig::with_uri(format!("{}/mcp", hosted.base))
        .auth_header(KEY_THERMAL);
    let transport = StreamableHttpClientTransport::from_config(config);
    let client = ().serve(transport).await.expect("initialize");
    let info = client.peer_info().expect("initialize result");
    assert_eq!(info.server_info.as_ref().unwrap().name, "Zero-Trust Physics");

    let listed = client.list_tools(None).await.expect("tools/list");
    assert_eq!(listed.tools.len(), 18);
    let schemas = serde_json::to_string(&listed.tools).unwrap();
    assert!(schemas.contains("thermal"));
    assert!(!schemas.contains("ti64_lattice"));
    assert!(!schemas.contains("satellite"));

    let step = json!({
        "domain": "energy", "card": "thermal",
        "parameters": {"temperature_k": 293.15, "heat_power_w": 1.0, "ambient_k": 293.15, "dt_s": 0.1}
    });
    let called = client
        .call_tool(CallToolRequestParams::new("verify_step").with_arguments(step.as_object().unwrap().clone()))
        .await
        .expect("verify_step");
    assert_eq!(called.is_error, Some(false));
    let text = called.content[0].as_text().unwrap().text.clone();
    let direct = result_text(&stdio.raw(&tool_line(1, "verify_step", &step)).await);
    assert_eq!(text, direct);
    assert!(text.contains("sha384_seal"));

    let hidden = json!({"domain": "materials", "card": "ti64_lattice"});
    let refused = client
        .call_tool(CallToolRequestParams::new("describe_card").with_arguments(hidden.as_object().unwrap().clone()))
        .await
        .expect("describe_card");
    assert_eq!(refused.is_error, Some(true));
    assert_eq!(refused.content[0].as_text().unwrap().text, "Card not recognized (UnknownCard).");
    client.cancel().await.expect("close");
}
