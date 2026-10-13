//! A stand-in solver, compiled for tests only, and what the gateway does with
//! a solver that is not `atom_dft`.
//!
//! The stand-in takes a number, a string and an array, and returns numbers,
//! an array and an array of arrays, or a named refusal. It is in the list of
//! solvers only when the crate is built for its own tests: this module is
//! `#[cfg(test)]`, so a served build that named it would not compile, and the
//! tests in `tests/` link the served build and count thirteen solvers. Its entry is
//! written the way an entry for a solver of `genesis_solvers` is written.

use serde_json::{Value, json};

use crate::solvers::{
    Answer, Given, Input, Kind, Output, Published, Refusal, Refused, Sections, Shape, Solved,
    Solver,
};

/// The stand-in itself: what a module of `genesis_solvers` holds. Constants,
/// a request, a result, named refusals and one function.
mod stand_in {
    use std::fmt;

    pub const GAIN_MIN: f64 = 0.5;
    pub const GAIN_MAX: f64 = 4.0;
    pub const SAMPLES_MOST: usize = 8;
    pub const SAMPLE_MIN: f64 = -100.0;
    pub const SAMPLE_MAX: f64 = 100.0;
    /// The largest scaled sum the stand-in returns.
    pub const SUM_LIMIT: f64 = 500.0;

    /// The one mode the stand-in has.
    pub const MODE: &str = "plain";

    #[derive(Clone, Debug, PartialEq)]
    pub struct Request {
        pub gain: f64,
        pub mode: String,
        pub samples: Vec<f64>,
    }

    #[derive(Clone, Debug, PartialEq)]
    pub struct Scaled {
        pub sample_count: u32,
        pub scaled_sum: f64,
        /// Each sample times the gain.
        pub scaled: Vec<f64>,
        /// For each sample, the sums of the scaled samples up to it.
        pub running: Vec<Vec<f64>>,
        /// The scaled sum less the last running sum.
        pub sum_residual: f64,
    }

    #[derive(Clone, Debug, PartialEq)]
    pub enum Reject {
        /// `gain` is outside `GAIN_MIN`..=`GAIN_MAX`.
        GainOutOfRange { gain: f64 },
        /// `mode` is not `plain`.
        ModeNotSupported,
        /// A sample is outside its range, or there are more than `SAMPLES_MOST`.
        SamplesOutOfRange,
        /// A sample is not above the one before it.
        SamplesNotIncreasing { index: usize },
        /// The scaled sum is above `SUM_LIMIT`.
        SumTooLarge { scaled_sum: f64 },
    }

    impl fmt::Display for Reject {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            match self {
                Self::GainOutOfRange { gain } => {
                    write!(f, "gain {gain} is outside {GAIN_MIN}..={GAIN_MAX}")
                }
                Self::ModeNotSupported => write!(f, "the mode is not plain"),
                Self::SamplesOutOfRange => write!(f, "a sample is outside its range"),
                Self::SamplesNotIncreasing { index } => {
                    write!(f, "sample {index} is not above the one before it")
                }
                Self::SumTooLarge { scaled_sum } => {
                    write!(f, "scaled sum {scaled_sum} is above {SUM_LIMIT}")
                }
            }
        }
    }

    /// The first sample that is not above the one before it.
    pub fn first_not_increasing(samples: &[f64]) -> Option<usize> {
        (1..samples.len()).find(|index| samples[*index] <= samples[*index - 1])
    }

    pub fn solve(request: &Request) -> Result<Scaled, Reject> {
        if !(GAIN_MIN..=GAIN_MAX).contains(&request.gain) {
            return Err(Reject::GainOutOfRange { gain: request.gain });
        }
        if request.mode != MODE {
            return Err(Reject::ModeNotSupported);
        }
        if request.samples.len() > SAMPLES_MOST
            || request
                .samples
                .iter()
                .any(|sample| !(SAMPLE_MIN..=SAMPLE_MAX).contains(sample))
        {
            return Err(Reject::SamplesOutOfRange);
        }
        if let Some(index) = first_not_increasing(&request.samples) {
            return Err(Reject::SamplesNotIncreasing { index });
        }
        let scaled: Vec<f64> = request.samples.iter().map(|sample| sample * request.gain).collect();
        let mut running = Vec::with_capacity(scaled.len());
        let mut sums = Vec::with_capacity(scaled.len());
        let mut scaled_sum = 0.0;
        for value in &scaled {
            scaled_sum += value;
            sums.push(scaled_sum);
            running.push(sums.clone());
        }
        if scaled_sum > SUM_LIMIT {
            return Err(Reject::SumTooLarge { scaled_sum });
        }
        Ok(Scaled {
            sample_count: scaled.len() as u32,
            scaled_sum,
            sum_residual: scaled_sum - sums.last().copied().unwrap_or(0.0),
            scaled,
            running,
        })
    }
}

pub const STAND_IN: Solver = Solver {
    name: "stand_in",
    summary: "A stand-in compiled for tests only: each sample times the gain, and the running sums.",
    inputs: &[
        Input {
            name: "gain",
            kind: Kind::Number {
                minimum: stand_in::GAIN_MIN,
                maximum: stand_in::GAIN_MAX,
                unit: "",
            },
            refusal: Some("GainOutOfRange"),
            bound: "What each sample is multiplied by.",
            only_when: None,
            asked_by_check: false,
        },
        Input {
            name: "mode",
            kind: Kind::Word { allowed: &[stand_in::MODE] },
            refusal: Some("ModeNotSupported"),
            bound: "Only `plain`.",
            only_when: None,
            asked_by_check: false,
        },
        Input {
            name: "samples",
            kind: Kind::Numbers {
                fewest: 0,
                most: stand_in::SAMPLES_MOST,
                minimum: stand_in::SAMPLE_MIN,
                maximum: stand_in::SAMPLE_MAX,
                unit: "cm",
            },
            refusal: Some("SamplesOutOfRange"),
            bound: "Each sample is above the one before it.",
            only_when: None,
            asked_by_check: false,
        },
    ],
    kept_inside: "The sum limit stays inside the solver.",
    result: &[
        Output {
            name: "scaled_sum",
            unit: "cm",
            shape: Shape::Number,
            each: &[],
            note: "",
            asked_by: None,
        },
        Output {
            name: "sample_count",
            unit: "samples",
            shape: Shape::Number,
            each: &[],
            note: "",
            asked_by: None,
        },
        Output {
            name: "scaled",
            unit: "cm",
            shape: Shape::Numbers,
            each: &[],
            note: "One for each sample.",
            asked_by: None,
        },
        Output {
            name: "running",
            unit: "cm",
            shape: Shape::Lists,
            each: &[],
            note: "For each sample, the sums up to it.",
            asked_by: None,
        },
    ],
    invariants: &[Output {
        name: "sum_residual",
        unit: "cm",
        shape: Shape::Number,
        each: &[],
        note: "",
        asked_by: None,
    }],
    iterations: "Samples scaled. On a refusal it is null.",
    field: field_words,
    answer: "scaled samples",
    refusals: &[
        Refusal {
            name: "GainOutOfRange",
            meaning: "`gain` is outside `GAIN_MIN`..=`GAIN_MAX`.",
            sealed: false,
        },
        Refusal {
            name: "ModeNotSupported",
            meaning: "`mode` is not `plain`.",
            sealed: false,
        },
        Refusal {
            name: "SamplesOutOfRange",
            meaning: "A sample is outside its range, or there are more than `SAMPLES_MOST`.",
            sealed: false,
        },
        Refusal {
            name: "SamplesNotIncreasing",
            meaning: "A sample is not above the one before it.",
            sealed: false,
        },
        Refusal {
            name: "SumTooLarge",
            meaning: "The scaled sum is above `SUM_LIMIT`.",
            sealed: true,
        },
    ],
    sections,
    validation: None,
    example,
    further_examples: &[],
    parts: &[],
    check,
    solve,
};

fn field_words() -> String {
    "SHA-256 of the count of samples as 8 little-endian bytes, then each scaled sample as little-endian f64 bytes.".to_string()
}

fn sections() -> Sections {
    Sections {
        locked: json!({ "sum_limit": stand_in::SUM_LIMIT }),
        invariants: json!([{
            "name": "sum_residual",
            "rule": "The scaled sum is the last running sum.",
            "tolerance": 0.0,
            "unit": "cm",
            "checked": "at the end",
            "refusal": null,
        }]),
        convergence: json!({ "rule": "One pass. Nothing is iterated." }),
        reference: json!({ "kind": "None. The stand-in is not physics." }),
    }
}

fn example() -> Value {
    json!({ "gain": 2.0, "mode": "plain", "samples": [1.5, 2.5, 4.0] })
}

/// The limit no single field's bounds state: each sample is above the one
/// before it. The request shows it, so it is refused before a permit is taken.
fn check(given: &Given) -> Result<(), String> {
    let samples = given.numbers("samples")?;
    match stand_in::first_not_increasing(samples) {
        Some(index) => Err(format!(
            "Physical Violation: 'samples[{index}]' {} is not above 'samples[{}]' {} (SamplesNotIncreasing).",
            samples[index],
            index - 1,
            samples[index - 1]
        )),
        None => Ok(()),
    }
}

/// The solver's own request, made from the parameters.
fn request(given: &Given) -> Result<stand_in::Request, String> {
    Ok(stand_in::Request {
        gain: given.number("gain")?,
        mode: given.word("mode")?.to_string(),
        samples: given.numbers("samples")?.to_vec(),
    })
}

fn solve(given: &Given) -> Result<Solved, String> {
    Ok(solved(stand_in::solve(&request(given)?)))
}

fn reject_name(reject: &stand_in::Reject) -> &'static str {
    match reject {
        stand_in::Reject::GainOutOfRange { .. } => "GainOutOfRange",
        stand_in::Reject::ModeNotSupported => "ModeNotSupported",
        stand_in::Reject::SamplesOutOfRange => "SamplesOutOfRange",
        stand_in::Reject::SamplesNotIncreasing { .. } => "SamplesNotIncreasing",
        stand_in::Reject::SumTooLarge { .. } => "SumTooLarge",
    }
}

/// What the stand-in returned, in the words every solver shares.
fn solved(returned: Result<stand_in::Scaled, stand_in::Reject>) -> Solved {
    match returned {
        Ok(scaled) => Solved::Answer(Answer {
            iterations: u64::from(scaled.sample_count),
            result: vec![
                ("scaled_sum".to_string(), Published::Number(scaled.scaled_sum)),
                (
                    "sample_count".to_string(),
                    Published::Number(f64::from(scaled.sample_count)),
                ),
                ("scaled".to_string(), Published::Numbers(scaled.scaled.clone())),
                ("running".to_string(), Published::Lists(scaled.running)),
            ],
            invariants: vec![("sum_residual".to_string(), Published::Number(scaled.sum_residual))],
            field: scaled.scaled,
        }),
        Err(reject) => Solved::Refused(Refused {
            name: reject_name(&reject),
            sentence: reject.to_string(),
            iterations: None,
        }),
    }
}

mod tests {
    use std::sync::Arc;

    use serde_json::{Map, Value, json};
    use tokio::net::TcpListener;

    use super::{STAND_IN, stand_in};
    use crate::guard::{self, Inspection};
    use crate::keys::{self, Keys};
    use crate::mcp_server::{dispatch, tools_in};
    use crate::view::View;
    use crate::{AppState, digest_hex, hosted, solution, solvers};

    const KEY: &str = "k-stand-in";

    /// The three connections over one state: the web routes, the function
    /// the local connection calls for a tool, and the hosted endpoint with a
    /// key that sees the stand-in and nothing else.
    struct Connections {
        state: AppState,
        web: String,
        hosted: String,
    }

    impl Connections {
        async fn open() -> Self {
            let state = crate::new_gateway_state();
            let web = serve(crate::router(state.clone())).await;
            let file = json!([{
                "client": "probe",
                "key_sha256": keys::sha256_hex(KEY.as_bytes()),
                "cards": [],
                "solvers": ["stand_in"],
                "max_in_flight": 4,
                "max_permits": 2
            }]);
            let keys = Keys::parse(file.to_string().as_bytes()).expect("keys");
            let router = hosted::router_over(state.clone(), keys, None, Vec::new()).expect("router");
            let hosted = format!("{}/mcp", serve(router).await);
            Self { state, web, hosted }
        }

        async fn web(&self, path: &str, body: &str) -> (u16, String) {
            let response = reqwest::Client::new()
                .post(format!("{}{path}", self.web))
                .header(reqwest::header::CONTENT_TYPE, "application/json")
                .body(body.to_string())
                .send()
                .await
                .unwrap();
            (response.status().as_u16(), response.text().await.unwrap())
        }

        /// The text of a tool's answer on the local connection, and whether
        /// it is an error. The local binary passes a line through
        /// `guard::inspect` and then calls `dispatch`. So does this.
        async fn local(&self, tool: &str, arguments: &str) -> (bool, String) {
            let line = tool_line(tool, arguments);
            match guard::inspect(line.as_bytes()) {
                Inspection::Forward => {}
                Inspection::Answer(answer) => return answer_text(&answer),
                Inspection::Blank | Inspection::NoAnswer => panic!("no answer to {line}"),
            }
            let arguments: Map<String, Value> = serde_json::from_str(arguments).expect("arguments");
            let state = self.state.clone();
            let result = dispatch(&state, tool, arguments).await;
            let value = serde_json::to_value(&result).expect("the result encodes");
            (
                value["isError"] == json!(true),
                value["content"][0]["text"].as_str().expect("text").to_string(),
            )
        }

        async fn hosted(&self, tool: &str, arguments: &str) -> (bool, String) {
            let response = reqwest::Client::new()
                .post(&self.hosted)
                .header(reqwest::header::CONTENT_TYPE, "application/json")
                .header("Authorization", format!("Bearer {KEY}"))
                .body(tool_line(tool, arguments))
                .send()
                .await
                .unwrap();
            assert_eq!(response.status().as_u16(), 200);
            answer_text(&response.text().await.unwrap())
        }

        /// One tool on all three connections. The answer must be the same
        /// bytes on each, and an error on all or on none.
        async fn all(&self, path: &str, tool: &str, body: &str) -> (u16, String) {
            let (status, web) = self.web(path, body).await;
            let (local_error, local) = self.local(tool, body).await;
            let (hosted_error, hosted) = self.hosted(tool, body).await;
            assert_eq!(web, local, "web and local: {body}");
            assert_eq!(web, hosted, "web and hosted: {body}");
            assert_eq!(local_error, status != 200, "{body}");
            assert_eq!(hosted_error, status != 200, "{body}");
            (status, web)
        }

        async fn solved(&self, request: &Value) -> (String, Value) {
            let (status, text) = self.all("/solve", "solve", &request.to_string()).await;
            assert_eq!(status, 200, "{text}");
            let body = serde_json::from_str(&text).expect("a solution");
            (text, body)
        }

        async fn verdict(&self, body: &Value) -> Value {
            let (status, text) = self
                .all("/verify_solution", "verify_solution", &body.to_string())
                .await;
            assert_eq!(status, 200, "{text}");
            serde_json::from_str(&text).expect("a report")
        }

        async fn assert_fails(&self, body: &Value, part: &str, label: &str) {
            let report = self.verdict(body).await;
            assert_eq!(report, json!({ "holds": false, "cause": part, "checked": crate::solution::WHOLE_SOLUTION_CHECKED }), "{label}");
        }

        /// A posted solution that is refused before its proof is recomputed.
        async fn assert_unread(&self, body: &Value, message: &str, label: &str) {
            let (status, text) = self
                .all("/verify_solution", "verify_solution", &body.to_string())
                .await;
            assert_eq!(status, 422, "{label}: {text}");
            assert_eq!(text, message, "{label}");
        }
    }

    async fn serve(router: axum::Router) -> String {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
        tokio::task::yield_now().await;
        format!("http://127.0.0.1:{port}")
    }

    fn tool_line(tool: &str, arguments: &str) -> String {
        format!(
            r#"{{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{{"name":"{tool}","arguments":{arguments}}}}}"#
        )
    }

    /// Whether a JSON-RPC answer to a tool call is an error, and its text.
    fn answer_text(answer: &str) -> (bool, String) {
        let value: Value = serde_json::from_str(answer).expect("an answer");
        let text = value["result"]["content"][0]["text"]
            .as_str()
            .unwrap_or_else(|| panic!("no tool text in {answer}"));
        (value["result"]["isError"] == json!(true), text.to_string())
    }

    fn sample_request() -> Value {
        json!({ "solver": "stand_in", "parameters": { "gain": 2.0, "mode": "plain", "samples": [1.5, 2.5, 4.0] } })
    }

    fn with(key: &str, value: Value) -> Value {
        let mut request = sample_request();
        request["parameters"][key] = value;
        request
    }

    const THREE: &str = "4008000000000000";
    const FIVE: &str = "4014000000000000";
    const EIGHT: &str = "4020000000000000";
    const SIXTEEN: &str = "4030000000000000";

    #[test]
    fn the_stand_in_is_listed_beside_the_atom_in_this_build() {
        let listed = solvers::list_value();
        let names: Vec<&str> = listed["solvers"]
            .as_array()
            .expect("solvers")
            .iter()
            .map(|row| row["solver"].as_str().expect("a name"))
            .collect();
        assert_eq!(names, ["atom_dft", "ablator_slab", "journal_bearing_sheffield", "propeller_apc_10x7e", "modulus_bam_s355", "seawater_column", "argon_pair_gas", "fusion_pb11_lhd", "fusion_pb11_thermal", "step_flow_armaly", "culture_trichoderma_ambr250", "tokamak_dt_beam_target", "pipe_flow_superpipe", "stand_in"]);
        assert_eq!(listed["solvers"][13]["summary"], STAND_IN.summary);
        assert!(solvers::is_known("stand_in"));
        // A view lists what it holds, in the engine's order.
        let one = View::limited([], ["stand_in".to_string()]);
        assert_eq!(
            solvers::list_value_in(&one),
            json!({ "count": 1, "solvers": [{ "solver": "stand_in", "summary": STAND_IN.summary }] })
        );
        assert!(!one.sees_solver("atom_dft"));
        let none = View::limited([], []);
        assert_eq!(solvers::list_value_in(&none), json!({ "count": 0, "solvers": [] }));
        // The entry is under `#[cfg(test)]` in the list of solvers, and this
        // module is under it in the crate.
        let list = include_str!("solvers.rs");
        assert!(list.contains("    #[cfg(test)]\n    &crate::solver_probe::STAND_IN,\n"));
        let crate_root = include_str!("lib.rs");
        assert!(crate_root.contains("#[cfg(test)]\nmod solver_probe;\n"));
    }

    #[test]
    fn the_description_publishes_the_fields_and_bounds_of_the_field_list() {
        let described = solvers::describe_value("stand_in").expect("described");
        assert_eq!(described["solver"], "stand_in");
        assert_eq!(described["engine_version"], solvers::engine_version());
        assert_eq!(
            described["inputs"],
            json!([
                {
                    "name": "gain", "type": "number", "minimum": 0.5, "maximum": 4.0, "unit": "",
                    "refusal": "GainOutOfRange", "bound": "What each sample is multiplied by."
                },
                {
                    "name": "mode", "type": "string", "allowed": ["plain"],
                    "refusal": "ModeNotSupported", "bound": "Only `plain`."
                },
                {
                    "name": "samples", "type": "array of numbers", "most": 8,
                    "minimum": -100.0, "maximum": 100.0, "unit": "cm",
                    "refusal": "SamplesOutOfRange", "bound": "Each sample is above the one before it."
                }
            ])
        );
        assert_eq!(
            described["returns"]["result"],
            json!([
                { "name": "scaled_sum", "unit": "cm" },
                { "name": "sample_count", "unit": "samples" },
                { "name": "scaled", "unit": "cm", "shape": "array of numbers", "note": "One for each sample." },
                { "name": "running", "unit": "cm", "shape": "array of arrays of numbers", "note": "For each sample, the sums up to it." }
            ])
        );
        assert_eq!(
            described["returns"]["invariants"],
            json!([{ "name": "sum_residual", "unit": "cm" }])
        );
        let hex = described["returns"]["hex"].as_str().expect("hex");
        assert!(hex.contains("An array has a `{field}_hex` sibling of the same shape"), "{hex}");
        let atom = solvers::describe_value("atom_dft").expect("described");
        assert!(!atom["returns"]["hex"].as_str().unwrap().contains("array"));
        assert_eq!(described["locked"], json!({ "sum_limit": 500.0 }));
        let reported: Vec<(&str, &str)> = described["refusals"]
            .as_array()
            .expect("refusals")
            .iter()
            .map(|row| (row["name"].as_str().unwrap(), row["reported_as"].as_str().unwrap()))
            .collect();
        assert_eq!(
            reported,
            [
                ("GainOutOfRange", "422, no seal"),
                ("ModeNotSupported", "422, no seal"),
                ("SamplesOutOfRange", "422, no seal"),
                ("SamplesNotIncreasing", "422, no seal"),
                ("SumTooLarge", "200, outcome refused, sealed"),
            ]
        );
        assert_eq!(described["example_solve"], sample_request());

        // Every key of the description is a key the atom's description has,
        // in the same order.
        let keys = |value: &Value| -> Vec<String> { value.as_object().unwrap().keys().cloned().collect() };
        assert_eq!(keys(&described), keys(&atom));
        assert_eq!(keys(&described["returns"]), keys(&atom["returns"]));
    }

    fn schema_of(view: &View, tool: &str) -> Value {
        let tools = tools_in(view);
        let tool = tools.iter().find(|listed| listed.name == tool).expect("tool");
        Value::Object(tool.input_schema.as_ref().clone())
    }

    #[test]
    fn a_schema_names_the_solvers_of_the_view_and_the_inputs_of_the_field_list() {
        // One solver in the view: its name, and each input with its bounds.
        let one = View::limited([], ["stand_in".to_string()]);
        let request = json!({
            "type": "object",
            "properties": {
                "solver": { "const": "stand_in" },
                "parameters": {
                    "type": "object",
                    "properties": {
                        "gain": { "type": "number", "minimum": 0.5, "maximum": 4.0 },
                        "mode": { "type": "string", "enum": ["plain"] },
                        "samples": {
                            "type": "array",
                            "items": { "type": "number", "minimum": -100.0, "maximum": 100.0 },
                            "maxItems": 8
                        }
                    },
                    "required": ["gain", "mode", "samples"],
                    "additionalProperties": false
                },
                "example": {
                    "type": "string",
                    "description": "In place of parameters: the name of one of the solver's worked examples as describe_solver keys it (example_solve, or a key that begins example_solve_). The request is filled with that example's parameters, and the filled request is the one solved, sealed and returned as inputs."
                }
            },
            "required": ["solver"],
            "additionalProperties": false
        });
        let mut solve = request.clone();
        solve["description"] = json!("One sealed solution. The same request as POST /solve.");
        assert_eq!(schema_of(&one, "solve"), solve);
        // A whole solution's inputs are hashed, not solved: a plain object.
        assert_eq!(schema_of(&one, "verify_solution")["properties"]["inputs"]["type"], "object");
        assert!(schema_of(&one, "verify_solution")["properties"]["inputs"].get("properties").is_none());
        assert_eq!(
            schema_of(&one, "describe_solver")["properties"]["solver"],
            json!({ "const": "stand_in" })
        );

        // The atom alone: the schema a served build gives, byte for byte.
        let atom = View::limited([], ["atom_dft".to_string()]);
        assert_eq!(
            serde_json::to_string(&schema_of(&atom, "solve")).unwrap(),
            r#"{"type":"object","properties":{"solver":{"const":"atom_dft"},"parameters":{"type":"object","properties":{"atomic_number":{"type":"integer","minimum":1,"maximum":18}},"required":["atomic_number"],"additionalProperties":false},"example":{"type":"string","description":"In place of parameters: the name of one of the solver's worked examples as describe_solver keys it (example_solve, or a key that begins example_solve_). The request is filled with that example's parameters, and the filled request is the one solved, sealed and returned as inputs."}},"required":["solver"],"additionalProperties":false,"description":"One sealed solution. The same request as POST /solve."}"#
        );
        assert_eq!(
            serde_json::to_string(&schema_of(&atom, "describe_solver")).unwrap(),
            r#"{"type":"object","properties":{"solver":{"const":"atom_dft"},"part":{"type":"string","description":"Leave this key out to get the solver's whole description. Some long descriptions name, under 'more', parts kept back for asking: set part to one of the names listed there for that part alone. Only a name listed under 'more' is a part."},"form":{"type":"string","enum":["short","full"],"description":"Leave this key out, or \"full\", for the whole description; \"short\" for a page that is enough to write a first request."}},"required":["solver"],"additionalProperties":false,"description":"Inputs and bounds, outputs, invariants and tolerances, refusals and meanings, the published reference, and a worked example for one solver."}"#
        );

        // More than one solver: a string, declared once, and `parameters` as
        // a plain object; the solvers and their inputs are left to
        // list_solvers and describe_solver. No choice between shapes is
        // declared.
        let full = View::full();
        let solve = schema_of(&full, "solve");
        assert_eq!(
            solve["properties"]["solver"],
            json!({ "type": "string", "description": "The solver, as list_solvers names it." })
        );
        assert_eq!(
            solve["properties"]["parameters"],
            json!({
                "type": "object",
                "description": format!(
                    "The inputs of the solver named in 'solver', each a number, a whole number, a word, a flag or an array as describe_solver gives it, and no other key. {}",
                    crate::solvers::SOLVER_NUMBER_RULE
                )
            })
        );
        assert_eq!(solve["required"], json!(["solver"]));
        let text = solve.to_string();
        for word in ["oneOf", "anyOf", "allOf", "atom_dft", "stand_in", "beam_energy_kev"] {
            assert!(!text.contains(word), "{word}");
        }

        // No solver: the same string and the same plain object.
        let none = View::limited([], []);
        assert_eq!(schema_of(&none, "solve")["properties"], solve["properties"]);
    }

    /// The proof of the stand-in's example, written out here by the rule in
    /// `REFERENCE.md` and hashed apart from the gateway's code.
    #[tokio::test]
    async fn a_solve_is_sealed_over_every_item_and_is_the_same_bytes_on_three_connections() {
        let doors = Connections::open().await;
        let (text, body) = doors.solved(&sample_request()).await;
        assert!(text.starts_with(r#"{"inputs":{"solver":"stand_in","parameters":{"gain":2.0,"mode":"plain","samples":[1.5,2.5,4.0]}},"engine_version":"genesis_solvers_v"#), "{text}");
        assert_eq!(body["outcome"], "converged");
        assert!(body["reject"].is_null());
        assert_eq!(body["iterations"], 3);
        assert_eq!(
            body["result"],
            json!({
                "scaled_sum": 16.0, "scaled_sum_hex": SIXTEEN,
                "sample_count": 3.0, "sample_count_hex": THREE,
                "scaled": [3.0, 5.0, 8.0], "scaled_hex": [THREE, FIVE, EIGHT],
                "running": [[3.0], [3.0, 8.0], [3.0, 8.0, 16.0]],
                "running_hex": [[THREE], [THREE, EIGHT], [THREE, EIGHT, SIXTEEN]]
            })
        );
        assert_eq!(
            body["invariants"],
            json!({ "sum_residual": 0.0, "sum_residual_hex": "0000000000000000" })
        );

        let request = r#"{"parameters":{"gain":x4000000000000000,"mode":"plain","samples":[x3ff8000000000000,x4004000000000000,x4010000000000000]},"solver":"stand_in"}"#;
        let invariants = "invariants.sum_residual_hex:0000000000000000\n";
        let result = format!(
            "result.running_hex:[[{THREE}],[{THREE},{EIGHT}],[{THREE},{EIGHT},{SIXTEEN}]]\nresult.sample_count_hex:{THREE}\nresult.scaled_hex:[{THREE},{FIVE},{EIGHT}]\nresult.scaled_sum_hex:{SIXTEEN}\n"
        );
        let mut field = 3_u64.to_le_bytes().to_vec();
        for value in [3.0_f64, 5.0, 8.0] {
            field.extend_from_slice(&value.to_le_bytes());
        }
        let field = digest_hex(&field);
        let tail = r#"{"outcome":"converged","iterations":3,"reject":null}"#;
        let version = format!("engine_version:{}\n", body["engine_version"].as_str().expect("version"));
        assert_eq!(body["field_sha256"], field);
        assert_eq!(field, "34638362c2d1a0d7254b72506cd1fb8a3758a6d980b25df5c60e7b596df7471f");
        assert_eq!(
            body["seal_parts"],
            json!({
                "inputs": "98d4d8168a9a03c3c50bee9c6655d2cca63e268bf433caf5b960403dbc9f3ae1",
                "engine_version": digest_hex(version.as_bytes()),
                "result": "593057774641f16f38ac36b1cda018ec2796ec18ac528d045b01b4721394f230",
                "invariants": "4edf3f7c8ec30d9d1e2caec821fa16530a80439c5a249b3b2d05a63da6c37d1f",
                "field_sha256": "9e5eb14bd81a38bac3cbbfdbc11dc0d5cb577e4265755da5bf7536430f722d5c",
                "outcome": "0d4652e12010aabc6a5ba72c7f8de38695979966b17362b20061e9b3003471ae"
            })
        );
        assert_eq!(body["seal_parts"]["result"], digest_hex(result.as_bytes()));
        let whole = format!("{request}{version}{invariants}{result}{field}{tail}");
        assert_eq!(body["solution_seal"], digest_hex(whole.as_bytes()));
        // Worked out with Python's hashlib, with the solvers at version 0.8.0;
        // with 0.7.0 put back the same rule gives the seal pinned before,
        // 07d853e7...c096. Of the six parts only the version's has moved.
        assert_eq!(body["engine_version"], "genesis_solvers_v0.8.0");
        assert_eq!(
            body["solution_seal"],
            "dddd976acc1fca1c5e3aa10da46164736deed3ec4fab00360c96cfd0fed07170"
        );

        // The receipt covers the bytes before it.
        let receipt = body["receipt_sha256"].as_str().expect("receipt");
        let document = text
            .strip_suffix(&format!(",\"receipt_sha256\":\"{receipt}\"}}"))
            .expect("the receipt is last");
        assert_eq!(digest_hex(format!("{document}}}").as_bytes()), receipt);

        assert_eq!(doors.verdict(&body).await, json!({ "holds": true, "cause": null, "checked": crate::solution::WHOLE_SOLUTION_CHECKED }));

        // The description is the same bytes on each connection too.
        let (status, described) = doors
            .all("/describe_solver", "describe_solver", r#"{"solver":"stand_in"}"#)
            .await;
        assert_eq!(status, 200);
        assert_eq!(
            described,
            serde_json::to_string(&solvers::describe_value("stand_in").expect("described")).unwrap()
        );

        // The same values in another key order and another spelling seal the
        // same. One sample moved does not.
        let respelled: Value = serde_json::from_str(
            r#"{"parameters":{"samples":[15e-1,2.50,4],"mode":"plain","gain":2},"solver":"stand_in"}"#,
        )
        .unwrap();
        let (_, again) = doors.solved(&respelled).await;
        assert_eq!(again["solution_seal"], body["solution_seal"]);
        let (_, other) = doors.solved(&with("samples", json!([1.5, 2.5, 4.5]))).await;
        assert_ne!(other["solution_seal"], body["solution_seal"]);

        // No sample at all is a solution too: two empty arrays, sealed.
        let (_, empty) = doors.solved(&with("samples", json!([]))).await;
        assert_eq!(empty["result"]["scaled_hex"], json!([]));
        assert_eq!(empty["result"]["running_hex"], json!([]));
        assert_eq!(
            empty["seal_parts"]["result"],
            digest_hex(b"result.running_hex:[]\nresult.sample_count_hex:0000000000000000\nresult.scaled_hex:[]\nresult.scaled_sum_hex:0000000000000000\n")
        );
        assert_eq!(doors.verdict(&empty).await, json!({ "holds": true, "cause": null, "checked": crate::solution::WHOLE_SOLUTION_CHECKED }));
    }

    #[tokio::test]
    async fn verify_solution_refuses_one_item_changed_removed_added_or_moved() {
        let doors = Connections::open().await;
        let (_, body) = doors.solved(&sample_request()).await;
        assert_eq!(doors.verdict(&body).await, json!({ "holds": true, "cause": null, "checked": crate::solution::WHOLE_SOLUTION_CHECKED }));

        let one = "3ff0000000000000";
        let edits: Vec<(&str, &str, Value)> = vec![
            ("one item changed", "scaled_hex", json!([THREE, FIVE, "4020000000000001"])),
            ("the first item changed", "scaled_hex", json!([one, FIVE, EIGHT])),
            ("the last item removed", "scaled_hex", json!([THREE, FIVE])),
            ("the first item removed", "scaled_hex", json!([FIVE, EIGHT])),
            ("every item removed", "scaled_hex", json!([])),
            ("an item added at the end", "scaled_hex", json!([THREE, FIVE, EIGHT, EIGHT])),
            ("an item added at the start", "scaled_hex", json!([THREE, THREE, FIVE, EIGHT])),
            ("two items swapped", "scaled_hex", json!([FIVE, THREE, EIGHT])),
            ("the ends swapped", "scaled_hex", json!([EIGHT, FIVE, THREE])),
            ("one inner item changed", "running_hex", json!([[THREE], [THREE, EIGHT], [THREE, EIGHT, one]])),
            ("one inner item removed", "running_hex", json!([[THREE], [THREE, EIGHT], [THREE, EIGHT]])),
            ("one inner item added", "running_hex", json!([[THREE], [THREE, EIGHT, EIGHT], [THREE, EIGHT, SIXTEEN]])),
            ("two inner items swapped", "running_hex", json!([[THREE], [EIGHT, THREE], [THREE, EIGHT, SIXTEEN]])),
            ("two inner arrays swapped", "running_hex", json!([[THREE, EIGHT], [THREE], [THREE, EIGHT, SIXTEEN]])),
            ("one inner array removed", "running_hex", json!([[THREE], [THREE, EIGHT]])),
            ("one inner array emptied", "running_hex", json!([[], [THREE, EIGHT], [THREE, EIGHT, SIXTEEN]])),
            ("an empty inner array added", "running_hex", json!([[THREE], [THREE, EIGHT], [THREE, EIGHT, SIXTEEN], []])),
            // The same six items in the same order, with the breaks between
            // the inner arrays moved.
            ("an item moved to the array before", "running_hex", json!([[THREE, THREE], [EIGHT], [THREE, EIGHT, SIXTEEN]])),
            ("an item moved to the array after", "running_hex", json!([[THREE], [THREE], [EIGHT, THREE, EIGHT, SIXTEEN]])),
            ("the inner arrays joined", "running_hex", json!([[THREE, THREE, EIGHT, THREE, EIGHT, SIXTEEN]])),
            ("a number changed", "scaled_sum_hex", json!(one)),
        ];
        for (label, key, value) in &edits {
            let mut edited = body.clone();
            assert_ne!(&edited["result"][*key], value, "{label}");
            edited["result"][*key] = value.clone();
            doors.assert_fails(&edited, "result", label).await;

            // An edit that also replaces the digest of its part still fails
            // the seal.
            let mut lines: Vec<(String, String)> = edited["result"]
                .as_object()
                .unwrap()
                .iter()
                .filter(|(name, _)| name.ends_with("_hex"))
                .map(|(name, hex)| (name.clone(), hex.to_string().replace('"', "")))
                .collect();
            lines.sort();
            let text: String = lines
                .iter()
                .map(|(name, hex)| format!("result.{name}:{hex}\n"))
                .collect();
            edited["seal_parts"]["result"] = json!(digest_hex(text.as_bytes()));
            doors.assert_fails(&edited, "solution_seal", label).await;
        }

        let mut removed = body.clone();
        removed["result"].as_object_mut().unwrap().remove("scaled_hex");
        doors.assert_fails(&removed, "result", "the array removed").await;
        let mut invariant = body.clone();
        invariant["invariants"]["sum_residual_hex"] = json!(one);
        doors.assert_fails(&invariant, "invariants", "an invariant changed").await;
        let mut field = body.clone();
        field["field_sha256"] = json!("ab".repeat(32));
        doors.assert_fails(&field, "field_sha256", "the field hash changed").await;
        let mut inputs = body.clone();
        inputs["inputs"]["parameters"]["samples"] = json!([1.5, 4.0, 2.5]);
        doors.assert_fails(&inputs, "inputs", "two samples of the request swapped").await;
        let mut inputs = body.clone();
        inputs["inputs"]["parameters"]["samples"] = json!([1.5, 2.5]);
        doors.assert_fails(&inputs, "inputs", "one sample of the request removed").await;

        // A value that is not exact, or not in the shape the solver
        // publishes it in, is named before any proof is recomputed.
        let array = "Physical Violation: 'result.scaled_hex' must be an array, and each item 16 lowercase hex digits.";
        let lists = "Physical Violation: 'result.running_hex' must be an array of arrays, and each item 16 lowercase hex digits.";
        let number = "Physical Violation: 'result.scaled_sum_hex' must be 16 lowercase hex digits.";
        let shapes: Vec<(&str, Value, &str)> = vec![
            ("scaled_hex", json!(THREE), array),
            ("scaled_hex", json!([THREE, 5.0, EIGHT]), array),
            ("scaled_hex", json!([THREE, "4014000000000000 ", EIGHT]), array),
            ("scaled_hex", json!([THREE, "401400000000000A", EIGHT]), array),
            ("scaled_hex", json!([THREE, null, EIGHT]), array),
            ("scaled_hex", json!([[THREE, FIVE, EIGHT]]), array),
            ("scaled_hex", json!(format!("[{THREE},{FIVE},{EIGHT}]")), array),
            ("scaled_hex", json!(null), array),
            ("running_hex", json!([THREE, THREE, EIGHT]), lists),
            ("running_hex", json!([[THREE], [THREE, EIGHT], [[THREE, EIGHT, SIXTEEN]]]), lists),
            ("running_hex", json!([[THREE], [THREE, EIGHT], THREE]), lists),
            ("running_hex", json!([[THREE], [THREE, "x"]]), lists),
            ("running_hex", json!({ "0": [THREE] }), lists),
            ("scaled_sum_hex", json!([SIXTEEN]), number),
            ("scaled_sum_hex", json!([]), number),
            ("scaled_sum_hex", json!(16.0), number),
        ];
        for (key, value, message) in &shapes {
            let mut edited = body.clone();
            edited["result"][*key] = value.clone();
            doors.assert_unread(&edited, message, &format!("{key} {value}")).await;
        }
        let mut unpublished = body.clone();
        unpublished["result"]["peak_hex"] = json!([THREE]);
        doors
            .assert_unread(
                &unpublished,
                "Physical Violation: 'result.peak_hex' is not published by this solver.",
                "a name the solver does not publish",
            )
            .await;
        // A name of another solver, and a name of the other section.
        let mut atoms = body.clone();
        atoms["result"]["total_energy_ha_hex"] = json!(THREE);
        doors
            .assert_unread(
                &atoms,
                "Physical Violation: 'result.total_energy_ha_hex' is not published by this solver.",
                "a name of the atom",
            )
            .await;
        let mut crossed = body.clone();
        crossed["invariants"]["scaled_hex"] = json!([THREE]);
        doors
            .assert_unread(
                &crossed,
                "Physical Violation: 'invariants.scaled_hex' is not published by this solver.",
                "a result name under invariants",
            )
            .await;
    }

    #[tokio::test]
    async fn each_fault_of_a_request_is_named_and_nothing_is_sealed() {
        let doors = Connections::open().await;
        let kept = "The sum limit stays inside the solver.";
        let unknown = |key: &str| format!("Physical Violation: '{key}' is not an input of this solver. {kept}");
        let without = |key: &str| {
            let mut request = sample_request();
            request["parameters"].as_object_mut().unwrap().remove(key);
            request.to_string()
        };
        let mut extra = sample_request();
        extra["parameters"]["sum_limit"] = json!(900.0);
        let mut extra_at_the_top = sample_request();
        extra_at_the_top["atomic_number"] = json!(10);
        let mut the_atom_s_input = sample_request();
        the_atom_s_input["parameters"]["atomic_number"] = json!(10);
        let nine: Vec<f64> = (1..=9).map(f64::from).collect();

        let cases: Vec<(String, String)> = vec![
            // An unknown key, at either depth.
            (extra.to_string(), unknown("sum_limit")),
            (extra_at_the_top.to_string(), unknown("atomic_number")),
            (the_atom_s_input.to_string(), unknown("atomic_number")),
            // A missing field. Nothing is filled in.
            (without("gain"), "Missing required parameter: 'gain'".to_string()),
            (without("mode"), "Missing required parameter: 'mode'".to_string()),
            (without("samples"), "Missing required parameter: 'samples'".to_string()),
            (json!({ "solver": "stand_in" }).to_string(), "Missing required parameter: 'parameters' (or 'example', the name of a worked example describe_solver lists)".to_string()),
            (
                json!({ "solver": "stand_in", "parameters": [2.0, "plain", [1.5]] }).to_string(),
                "Physical Violation: 'parameters' must be a JSON object.".to_string(),
            ),
            // A repeated key, at any depth, and a number that reads as zero.
            (
                r#"{"solver":"stand_in","parameters":{"gain":2.0,"mode":"plain","samples":[1.5],"gain":3.0}}"#.to_string(),
                "Physical Violation: the key 'gain' is repeated.".to_string(),
            ),
            (
                r#"{"solver":"stand_in","parameters":{"gain":2.0,"mode":"plain","samples":[1.5],"samples":[1.5,2.5]}}"#.to_string(),
                "Physical Violation: the key 'samples' is repeated.".to_string(),
            ),
            (
                r#"{"solver":"stand_in","parameters":{"gain":2.0,"mode":"plain","samples":[1.5,1e-400]}}"#.to_string(),
                "Physical Violation: 'samples[1]' is written 1e-400, a number that is not zero and reads as zero.".to_string(),
            ),
            // A wrong string.
            (
                with("mode", json!("isothermal")).to_string(),
                "Physical Violation: 'mode' must be \"plain\"; this request has \"isothermal\" (ModeNotSupported).".to_string(),
            ),
            (
                with("mode", json!("Plain")).to_string(),
                "Physical Violation: 'mode' must be \"plain\"; this request has \"Plain\" (ModeNotSupported).".to_string(),
            ),
            (
                with("mode", json!("")).to_string(),
                "Physical Violation: 'mode' must be \"plain\"; this request has \"\" (ModeNotSupported).".to_string(),
            ),
            (
                with("mode", json!(1)).to_string(),
                "Physical Violation: 'mode' must be a string; this request has 1.".to_string(),
            ),
            (
                with("mode", json!(null)).to_string(),
                "Physical Violation: 'mode' must be a string; this request has null.".to_string(),
            ),
            // An array that is too long, and an item outside its range.
            (
                with("samples", json!(nine)).to_string(),
                "Physical Violation: 'samples' holds 9 numbers, more than 8 (SamplesOutOfRange).".to_string(),
            ),
            (
                with("samples", json!([1.5, 2.5, 100.5])).to_string(),
                "Physical Violation: 'samples[2]' 100.5 is outside -100..=100 cm (SamplesOutOfRange).".to_string(),
            ),
            (
                with("samples", json!([-101, 2.5])).to_string(),
                "Physical Violation: 'samples[0]' -101 is outside -100..=100 cm (SamplesOutOfRange).".to_string(),
            ),
            (
                with("samples", json!([1.5, "2.5"])).to_string(),
                "Physical Violation: 'samples[1]' must be a number; this request has \"2.5\".".to_string(),
            ),
            (
                with("samples", json!([1.5, [2.5]])).to_string(),
                "Physical Violation: 'samples[1]' must be a number; this request has [2.5].".to_string(),
            ),
            (
                with("samples", json!(1.5)).to_string(),
                "Physical Violation: 'samples' must be an array of numbers; this request has 1.5.".to_string(),
            ),
            (
                with("samples", json!({ "0": 1.5 })).to_string(),
                "Physical Violation: 'samples' must be an array of numbers; this request has {\"0\":1.5}.".to_string(),
            ),
            // A number outside its range. No value is moved to a bound.
            (
                with("gain", json!(4.5)).to_string(),
                "Physical Violation: 'gain' 4.5 is outside 0.5..=4 (GainOutOfRange).".to_string(),
            ),
            (
                with("gain", json!(0.4999999999999999)).to_string(),
                "Physical Violation: 'gain' 0.4999999999999999 is outside 0.5..=4 (GainOutOfRange).".to_string(),
            ),
            (
                with("gain", json!(-2)).to_string(),
                "Physical Violation: 'gain' -2 is outside 0.5..=4 (GainOutOfRange).".to_string(),
            ),
            (
                with("gain", json!("2.0")).to_string(),
                "Physical Violation: 'gain' must be a number; this request has \"2.0\".".to_string(),
            ),
            (
                with("gain", json!([2.0])).to_string(),
                "Physical Violation: 'gain' must be a number; this request has [2.0].".to_string(),
            ),
            // The limit that spans two items: the entry's own check.
            (
                with("samples", json!([1.5, 4.0, 2.5])).to_string(),
                "Physical Violation: 'samples[2]' 2.5 is not above 'samples[1]' 4 (SamplesNotIncreasing).".to_string(),
            ),
            (
                with("samples", json!([1.5, 1.5])).to_string(),
                "Physical Violation: 'samples[1]' 1.5 is not above 'samples[0]' 1.5 (SamplesNotIncreasing).".to_string(),
            ),
        ];
        let pool = Arc::clone(&doors.state.physics_permits);
        let held: Vec<_> = (0..pool.available_permits())
            .map(|_| Arc::clone(&pool).try_acquire_owned().unwrap())
            .collect();
        for (request, message) in &cases {
            // Every permit is held: a request that is refused here was
            // refused before a permit was asked for.
            let (status, text) = doors.all("/solve", "solve", request).await;
            assert_eq!(status, 422, "{request}: {text}");
            assert_eq!(&text, message, "{request}");
            assert!(!text.contains("solution_seal"), "{text}");
        }
        // A request with no fault does ask for one.
        let (status, text) = doors.all("/solve", "solve", &sample_request().to_string()).await;
        assert_eq!((status, text.as_str()), (429, "Too many concurrent physics tasks."));
        drop(held);

        // The bounds themselves are inside the range.
        for (gain, samples) in [(0.5, json!([-100.0, 100.0])), (4.0, json!([1, 2, 3, 4, 5, 6, 7, 8]))] {
            let mut request = with("gain", json!(gain));
            request["parameters"]["samples"] = samples;
            let (_, body) = doors.solved(&request).await;
            assert_eq!(body["outcome"], "converged", "{request}");
        }

        // A solver the engine does not have, and one the key does not see.
        let (status, text) = doors
            .all("/solve", "solve", r#"{"solver":"stand_out","parameters":{}}"#)
            .await;
        assert_eq!((status, text.as_str()), (404, "Solver not recognized"));
        let atom = r#"{"solver":"atom_dft","parameters":{"atomic_number":1}}"#;
        let (is_error, hidden) = doors.hosted("solve", atom).await;
        assert!(is_error);
        assert_eq!(hidden, "Solver not recognized");
    }

    /// The stand-in refuses a sum above its limit only once it has computed
    /// it. That refusal is a sealed solution with no result.
    #[tokio::test]
    async fn a_refusal_by_the_solver_is_sealed_under_its_name() {
        let doors = Connections::open().await;
        let request = json!({ "solver": "stand_in", "parameters": { "gain": 4.0, "mode": "plain", "samples": [50, 60, 70] } });
        let (_, body) = doors.solved(&request).await;
        assert_eq!(body["outcome"], "refused");
        assert_eq!(body["reject"], "SumTooLarge");
        assert_eq!(
            body["reject_detail"],
            "Solver Refusal: stand_in did not return scaled samples (SumTooLarge). scaled sum 720 is above 500."
        );
        assert!(body["iterations"].is_null());
        assert!(body["result"].is_null());
        assert!(body["invariants"].is_null());
        assert!(body["field_sha256"].is_null());
        let tail = r#"{"outcome":"refused","iterations":null,"reject":"SumTooLarge"}"#;
        assert_eq!(body["seal_parts"]["outcome"], digest_hex(tail.as_bytes()));
        let sealed = r#"{"parameters":{"gain":x4010000000000000,"mode":"plain","samples":[x4049000000000000,x404e000000000000,x4051800000000000]},"solver":"stand_in"}"#;
        let version = format!("engine_version:{}\n", body["engine_version"].as_str().expect("version"));
        assert_eq!(
            body["solution_seal"],
            digest_hex(format!("{sealed}{version}null{tail}").as_bytes())
        );
        // A solution claimed for another version of the solvers does not hold.
        let mut relabelled = body.clone();
        relabelled["engine_version"] = json!("genesis_solvers_v0.4.0");
        doors.assert_fails(&relabelled, "engine_version", "the engine version changed").await;
        assert_eq!(doors.verdict(&body).await, json!({ "holds": true, "cause": null, "checked": crate::solution::WHOLE_SOLUTION_CHECKED }));

        let mut renamed = body.clone();
        renamed["reject"] = json!("SamplesOutOfRange");
        doors.assert_fails(&renamed, "outcome", "the refusal renamed").await;
        let mut promoted = body.clone();
        promoted["outcome"] = json!("converged");
        doors.assert_fails(&promoted, "outcome", "the refusal promoted").await;
        let mut filled = body.clone();
        filled["result"] = json!({ "scaled_hex": [THREE] });
        doors.assert_fails(&filled, "result", "a result added to a refusal").await;

        // The stand-in checks its own range again. A refusal the request
        // shows is one the gateway answered before the solver ran, so each
        // of the two names it the same way.
        let outside = stand_in::solve(&stand_in::Request {
            gain: 4.5,
            mode: stand_in::MODE.to_string(),
            samples: vec![1.0],
        });
        assert_eq!(outside, Err(stand_in::Reject::GainOutOfRange { gain: 4.5 }));
        let (status, text) = doors
            .web("/solve", &with("gain", json!(4.5)).to_string())
            .await;
        assert_eq!(status, 422);
        assert!(text.ends_with("(GainOutOfRange)."), "{text}");
    }

    /// The worked example of an array in `REFERENCE.md`.
    #[test]
    fn the_worked_example_of_an_array_in_the_reference_is_this_text() {
        let result = json!({
            "cell_k": [300.0, 301.0],
            "cell_k_hex": ["4072c00000000000", "4072d00000000000"],
            "probe_k": [[300.0], [300.0, 301.0], []],
            "probe_k_hex": [["4072c00000000000"], ["4072c00000000000", "4072d00000000000"], []]
        });
        for (decimal, hex) in [(300.0_f64, "4072c00000000000"), (301.0, "4072d00000000000")] {
            assert_eq!(format!("{:016x}", decimal.to_bits()), hex);
        }
        let text = solution::values_text("result", result.as_object()).expect("text");
        assert_eq!(
            text,
            "result.cell_k_hex:[4072c00000000000,4072d00000000000]\nresult.probe_k_hex:[[4072c00000000000],[4072c00000000000,4072d00000000000],[]]\n"
        );
        assert_eq!(text.len(), 133);
        assert_eq!(
            digest_hex(text.as_bytes()),
            "292f816c5ca14b63291df87e289d74929fee18979689449807dbb838065b20d2"
        );
        // `REFERENCE.md` is beside the crate and is not in its history. Where
        // it is on the disk, it holds this example.
        let page = concat!(env!("CARGO_MANIFEST_DIR"), "/REFERENCE.md");
        if let Ok(reference) = std::fs::read_to_string(page) {
            for held in [
                "`result.cell_k_hex:[4072c00000000000,4072d00000000000]`",
                "`result.probe_k_hex:[[4072c00000000000],[4072c00000000000,4072d00000000000],[]]`",
                "292f816c5ca14b63291df87e289d74929fee18979689449807dbb838065b20d2",
            ] {
                assert!(
                    reference.contains(held),
                    "REFERENCE.md does not hold the worked example of an array: {held}"
                );
            }
        }
    }
}

/// A second stand-in, in no list of solvers: a switch with two choices and
/// the inputs that go with each, a choice inside one of those choices with an
/// input of its own, a flag, an array with a fewest and a most, a
/// field whose bounds its check asks, and a value returned only when asked
/// for. It holds the gateway to the kinds an entry may use, apart from any
/// solver that uses them.
mod switched {
    use serde_json::{Value, json};

    use crate::solvers::{
        Answer, Given, Input, Kind, Output, Published, Refusal, Sections, Shape, Solved, Solver,
    };

    pub const SIDE_MIN_CM: f64 = 1.0;
    pub const SIDE_MAX_CM: f64 = 8.0;

    const ROUND: Option<(&str, &str)> = Some(("shape", "round"));
    const SQUARE: Option<(&str, &str)> = Some(("shape", "square"));

    pub const SWITCHED: Solver = Solver {
        name: "switched",
        summary: "A stand-in compiled for tests only: the area of a round or a square plate.",
        inputs: &[
            Input {
                name: "shape",
                kind: Kind::Word { allowed: &["round", "square"] },
                refusal: None,
                bound: "The plate.",
                only_when: None,
                asked_by_check: false,
            },
            Input {
                name: "radius_cm",
                kind: Kind::Number { minimum: 0.5, maximum: 4.0, unit: "cm" },
                refusal: Some("SizeOutOfRange"),
                bound: "Radius of a round plate.",
                only_when: ROUND,
                asked_by_check: false,
            },
            Input {
                name: "side_cm",
                kind: Kind::Number { minimum: SIDE_MIN_CM, maximum: SIDE_MAX_CM, unit: "cm" },
                refusal: Some("SizeOutOfRange"),
                bound: "Side of a square plate.",
                only_when: SQUARE,
                asked_by_check: true,
            },
            Input {
                name: "hollow",
                kind: Kind::Flag,
                refusal: None,
                bound: "Whether a square plate is hollow.",
                only_when: SQUARE,
                asked_by_check: false,
            },
            Input {
                name: "corners",
                kind: Kind::Word { allowed: &["sharp", "rounded"] },
                refusal: None,
                bound: "The corners of a square plate.",
                only_when: SQUARE,
                asked_by_check: false,
            },
            Input {
                name: "corner_radius_cm",
                kind: Kind::Number { minimum: 0.1, maximum: 0.5, unit: "cm" },
                refusal: Some("SizeOutOfRange"),
                bound: "Radius of a rounded corner.",
                only_when: Some(("corners", "rounded")),
                asked_by_check: false,
            },
            Input {
                name: "marks_cm",
                kind: Kind::Numbers { fewest: 2, most: 4, minimum: 0.0, maximum: 10.0, unit: "cm" },
                refusal: Some("MarksOutOfRange"),
                bound: "Marks along the plate.",
                only_when: None,
                asked_by_check: false,
            },
            Input {
                name: "return_marks",
                kind: Kind::Flag,
                refusal: None,
                bound: "`true` to get the marks back doubled.",
                only_when: None,
                asked_by_check: false,
            },
        ],
        kept_inside: "Nothing else is an input.",
        result: &[
            Output {
                name: "area_cm2",
                unit: "cm²",
                shape: Shape::Number,
                each: &[],
                note: "",
                asked_by: None,
            },
            Output {
                name: "marks_doubled_cm",
                unit: "cm",
                shape: Shape::Numbers,
                each: &[],
                note: "Each mark twice over.",
                asked_by: Some("return_marks"),
            },
        ],
        invariants: &[],
        iterations: "Always one.",
        field: field_words,
        answer: "an area",
        refusals: &[
            Refusal {
                name: "SizeOutOfRange",
                meaning: "The radius or the side is outside its range.",
                sealed: false,
            },
            Refusal {
                name: "MarksOutOfRange",
                meaning: "A mark is outside its range, or there are too few or too many.",
                sealed: false,
            },
        ],
        sections,
        validation: None,
        example,
        further_examples: &[],
        parts: &[],
        check,
        solve,
    };

    fn field_words() -> String {
        "SHA-256 of the count of marks as 8 little-endian bytes, then each mark as little-endian f64 bytes.".to_string()
    }

    fn sections() -> Sections {
        Sections {
            locked: json!({}),
            invariants: json!([]),
            convergence: json!({ "rule": "One pass." }),
            reference: json!({ "kind": "None. The stand-in is not physics." }),
        }
    }

    fn example() -> Value {
        json!({ "shape": "round", "radius_cm": 2.0, "marks_cm": [1.0, 2.0], "return_marks": false })
    }

    /// The bounds of the side are asked here, in this stand-in's own words.
    fn check(given: &Given) -> Result<(), String> {
        if given.word("shape")? == "square" {
            let side_cm = given.number("side_cm")?;
            if !(SIDE_MIN_CM..=SIDE_MAX_CM).contains(&side_cm) {
                return Err(format!(
                    "Physical Violation: side {side_cm} cm is outside {SIDE_MIN_CM}..={SIDE_MAX_CM} (SizeOutOfRange)."
                ));
            }
        }
        Ok(())
    }

    fn solve(given: &Given) -> Result<Solved, String> {
        let area_cm2 = if given.word("shape")? == "round" {
            let radius_cm = given.number("radius_cm")?;
            std::f64::consts::PI * radius_cm * radius_cm
        } else {
            let side_cm = given.number("side_cm")?;
            // A rounded corner takes a little off each of the four.
            let off_cm2 = if given.word("corners")? == "rounded" {
                let radius_cm = given.number("corner_radius_cm")?;
                (4.0 - std::f64::consts::PI) * radius_cm * radius_cm
            } else {
                0.0
            };
            if given.flag("hollow")? { 0.0 } else { side_cm * side_cm - off_cm2 }
        };
        let marks = given.numbers("marks_cm")?;
        let mut result = vec![("area_cm2".to_string(), Published::Number(area_cm2))];
        if given.flag("return_marks")? {
            result.push((
                "marks_doubled_cm".to_string(),
                Published::Numbers(marks.iter().map(|mark| 2.0 * mark).collect()),
            ));
        }
        Ok(Solved::Answer(Answer {
            iterations: 1,
            result,
            invariants: Vec::new(),
            field: marks.to_vec(),
        }))
    }
}

mod switched_tests {
    use serde_json::{Map, Value, json};

    use super::switched::SWITCHED;
    use crate::solvers::{self, Answer, Published, Solved};
    use crate::{digest_hex, solution};

    fn parameters(value: Value) -> Map<String, Value> {
        value.as_object().expect("an object").clone()
    }

    /// The request as its solver takes it.
    fn read(value: Value) -> crate::solvers::Given {
        match solution::read_parameters(&SWITCHED, &parameters(value)) {
            Ok(given) => given,
            Err(error) => panic!("{}", error.message()),
        }
    }

    /// The sentence the request is refused with before a permit is taken.
    fn refused(value: Value) -> String {
        match solution::read_parameters(&SWITCHED, &parameters(value)) {
            Ok(_) => panic!("the request was read"),
            Err(error) => error.message().to_string(),
        }
    }

    fn round() -> Value {
        json!({ "shape": "round", "radius_cm": 2.0, "marks_cm": [1.0, 2.0], "return_marks": false })
    }

    fn square() -> Value {
        json!({ "shape": "square", "side_cm": 3.0, "hollow": false, "corners": "sharp", "marks_cm": [1.0, 2.0, 4.0], "return_marks": true })
    }

    fn with(mut request: Value, key: &str, value: Value) -> Value {
        request[key] = value;
        request
    }

    fn without(mut request: Value, key: &str) -> Value {
        request.as_object_mut().expect("an object").remove(key);
        request
    }

    #[test]
    fn each_choice_takes_its_own_inputs_and_refuses_the_others_by_name() {
        for request in [round(), square()] {
            let given = read(request);
            assert!(given.word("shape").is_ok());
        }
        // A round plate has a radius and no side; a square one the reverse.
        let given = read(round());
        assert_eq!(given.number("radius_cm"), Ok(2.0));
        assert!(given.number("side_cm").is_err());
        assert!(given.flag("hollow").is_err());

        assert_eq!(
            refused(with(round(), "side_cm", json!(3.0))),
            "Physical Violation: 'side_cm' is an input only when 'shape' is \"square\"; this request has 'shape' \"round\"."
        );
        assert_eq!(
            refused(with(round(), "hollow", json!(false))),
            "Physical Violation: 'hollow' is an input only when 'shape' is \"square\"; this request has 'shape' \"round\"."
        );
        assert_eq!(
            refused(with(square(), "radius_cm", json!(2.0))),
            "Physical Violation: 'radius_cm' is an input only when 'shape' is \"round\"; this request has 'shape' \"square\"."
        );
        // A key of the choice made is required, and nothing is filled in.
        assert_eq!(refused(without(round(), "radius_cm")), "Missing required parameter: 'radius_cm'");
        assert_eq!(refused(without(square(), "side_cm")), "Missing required parameter: 'side_cm'");
        assert_eq!(refused(without(square(), "hollow")), "Missing required parameter: 'hollow'");
        assert_eq!(refused(without(square(), "return_marks")), "Missing required parameter: 'return_marks'");
        assert_eq!(refused(without(round(), "shape")), "Missing required parameter: 'shape'");
        // A key in no list, and a choice the switch does not have.
        assert_eq!(
            refused(with(round(), "corner_cm", json!(1.0))),
            "Physical Violation: 'corner_cm' is not an input of this solver. Nothing else is an input."
        );
        assert_eq!(
            refused(with(round(), "shape", json!("oval"))),
            "Physical Violation: 'shape' must be one of \"round\", \"square\"; this request has \"oval\"."
        );
    }

    /// A choice inside a choice: the corners go with a square plate, and a
    /// corner radius with rounded corners. An input is refused by name when
    /// its own choice was not made, and when the choice it hangs on was
    /// never open to the request.
    #[test]
    fn a_choice_inside_a_choice_takes_its_inputs_only_when_both_are_made() {
        let rounded = with(with(square(), "corners", json!("rounded")), "corner_radius_cm", json!(0.5));
        let given = read(rounded.clone());
        assert_eq!(given.word("corners"), Ok("rounded"));
        assert_eq!(given.number("corner_radius_cm"), Ok(0.5));
        assert_eq!(given.word_if_given("corners"), Some("rounded"));
        assert_eq!(read(round()).word_if_given("corners"), None);
        let area = |request: Value| match (SWITCHED.solve)(&read(request)).expect("solved") {
            Solved::Answer(answer) => match answer.result[0].1 {
                Published::Number(area) => area,
                _ => panic!("an area is a number"),
            },
            Solved::Refused(_) => panic!("refused"),
        };
        assert_eq!(area(square()), 9.0);
        assert_eq!(area(rounded.clone()), 9.0 - (4.0 - std::f64::consts::PI) * 0.25);

        assert_eq!(
            refused(without(rounded.clone(), "corner_radius_cm")),
            "Missing required parameter: 'corner_radius_cm'"
        );
        assert_eq!(refused(without(square(), "corners")), "Missing required parameter: 'corners'");
        assert_eq!(
            refused(with(square(), "corner_radius_cm", json!(0.5))),
            "Physical Violation: 'corner_radius_cm' is an input only when 'corners' is \"rounded\"; this request has 'corners' \"sharp\"."
        );
        assert_eq!(
            refused(with(round(), "corner_radius_cm", json!(0.5))),
            "Physical Violation: 'corner_radius_cm' is an input only when 'corners' is \"rounded\"; this request has no 'corners'."
        );
        assert_eq!(
            refused(with(round(), "corners", json!("rounded"))),
            "Physical Violation: 'corners' is an input only when 'shape' is \"square\"; this request has 'shape' \"round\"."
        );
        assert_eq!(
            refused(with(rounded.clone(), "corner_radius_cm", json!(0.6))),
            "Physical Violation: 'corner_radius_cm' 0.6 is outside 0.1..=0.5 cm (SizeOutOfRange)."
        );
        assert_eq!(
            refused(with(square(), "corners", json!("bevelled"))),
            "Physical Violation: 'corners' must be one of \"sharp\", \"rounded\"; this request has \"bevelled\"."
        );
    }

    #[test]
    fn a_flag_is_true_or_false_and_nothing_else() {
        for (value, shown) in [
            (json!(1), "1"),
            (json!(0), "0"),
            (json!("true"), "\"true\""),
            (Value::Null, "null"),
            (json!([true]), "[true]"),
        ] {
            assert_eq!(
                refused(with(round(), "return_marks", value)),
                format!("Physical Violation: 'return_marks' must be true or false; this request has {shown}.")
            );
        }
        for flag in [true, false] {
            let given = read(with(round(), "return_marks", json!(flag)));
            assert_eq!(given.flag("return_marks"), Ok(flag));
            assert!(given.number("return_marks").is_err());
        }
    }

    #[test]
    fn an_array_has_a_fewest_and_a_most() {
        assert_eq!(
            refused(with(round(), "marks_cm", json!([1.0]))),
            "Physical Violation: 'marks_cm' holds 1 numbers, fewer than 2 (MarksOutOfRange)."
        );
        assert_eq!(
            refused(with(round(), "marks_cm", json!([]))),
            "Physical Violation: 'marks_cm' holds 0 numbers, fewer than 2 (MarksOutOfRange)."
        );
        assert_eq!(
            refused(with(round(), "marks_cm", json!([1, 2, 3, 4, 5]))),
            "Physical Violation: 'marks_cm' holds 5 numbers, more than 4 (MarksOutOfRange)."
        );
        assert_eq!(
            refused(with(round(), "marks_cm", json!([1, 11]))),
            "Physical Violation: 'marks_cm[1]' 11 is outside 0..=10 cm (MarksOutOfRange)."
        );
        for marks in [json!([0, 10]), json!([1, 2, 3, 4])] {
            read(with(round(), "marks_cm", marks));
        }
    }

    /// The radius is held to its bounds as it is read, in the gateway's
    /// words. The side is held to its kind as it is read and to its bounds by
    /// the entry's check, in the entry's words, and no value passes both.
    #[test]
    fn bounds_the_check_asks_are_asked_by_the_check_and_by_nothing_else() {
        assert_eq!(
            refused(with(round(), "radius_cm", json!(5.0))),
            "Physical Violation: 'radius_cm' 5.0 is outside 0.5..=4 cm (SizeOutOfRange)."
        );
        assert_eq!(
            refused(with(square(), "side_cm", json!(9.0))),
            "Physical Violation: side 9 cm is outside 1..=8 (SizeOutOfRange)."
        );
        assert_eq!(
            refused(with(square(), "side_cm", json!(0.5))),
            "Physical Violation: side 0.5 cm is outside 1..=8 (SizeOutOfRange)."
        );
        assert_eq!(
            refused(with(square(), "side_cm", json!("3"))),
            "Physical Violation: 'side_cm' must be a number; this request has \"3\"."
        );
        for side in [1.0, 8.0] {
            read(with(square(), "side_cm", json!(side)));
        }
    }

    #[test]
    fn the_description_and_the_schema_say_which_inputs_go_with_which_choice() {
        let rows: Vec<Value> = SWITCHED.inputs.iter().map(solvers::input_row).collect();
        assert_eq!(
            json!(rows),
            json!([
                { "name": "shape", "type": "string", "allowed": ["round", "square"], "bound": "The plate." },
                {
                    "name": "radius_cm", "type": "number", "minimum": 0.5, "maximum": 4.0, "unit": "cm",
                    "refusal": "SizeOutOfRange", "given_only_when": "'shape' is \"round\"",
                    "bound": "Radius of a round plate."
                },
                {
                    "name": "side_cm", "type": "number", "minimum": 1.0, "maximum": 8.0, "unit": "cm",
                    "refusal": "SizeOutOfRange", "given_only_when": "'shape' is \"square\"",
                    "bound": "Side of a square plate."
                },
                {
                    "name": "hollow", "type": "true or false", "given_only_when": "'shape' is \"square\"",
                    "bound": "Whether a square plate is hollow."
                },
                {
                    "name": "corners", "type": "string", "allowed": ["sharp", "rounded"],
                    "given_only_when": "'shape' is \"square\"", "bound": "The corners of a square plate."
                },
                {
                    "name": "corner_radius_cm", "type": "number", "minimum": 0.1, "maximum": 0.5, "unit": "cm",
                    "refusal": "SizeOutOfRange", "given_only_when": "'corners' is \"rounded\"",
                    "bound": "Radius of a rounded corner."
                },
                {
                    "name": "marks_cm", "type": "array of numbers", "fewest": 2, "most": 4,
                    "minimum": 0.0, "maximum": 10.0, "unit": "cm",
                    "refusal": "MarksOutOfRange", "bound": "Marks along the plate."
                },
                { "name": "return_marks", "type": "true or false", "bound": "`true` to get the marks back doubled." }
            ])
        );
        let outputs: Vec<Value> = SWITCHED.result.iter().map(solvers::output_row).collect();
        assert_eq!(
            json!(outputs),
            json!([
                { "name": "area_cm2", "unit": "cm²" },
                {
                    "name": "marks_doubled_cm", "unit": "cm", "shape": "array of numbers",
                    "returned_only_when": "'return_marks' is true", "note": "Each mark twice over."
                }
            ])
        );
        let schema = solvers::parameters_schema(&SWITCHED);
        assert_eq!(
            schema,
            json!({
                "type": "object",
                "properties": {
                    "shape": { "type": "string", "enum": ["round", "square"] },
                    "radius_cm": { "type": "number", "minimum": 0.5, "maximum": 4.0 },
                    "side_cm": { "type": "number", "minimum": 1.0, "maximum": 8.0 },
                    "hollow": { "type": "boolean" },
                    "corners": { "type": "string", "enum": ["sharp", "rounded"] },
                    "corner_radius_cm": { "type": "number", "minimum": 0.1, "maximum": 0.5 },
                    "marks_cm": {
                        "type": "array",
                        "items": { "type": "number", "minimum": 0.0, "maximum": 10.0 },
                        "minItems": 2,
                        "maxItems": 4
                    },
                    "return_marks": { "type": "boolean" }
                },
                "required": ["shape", "marks_cm", "return_marks"],
                "additionalProperties": false,
                "description": "Every key in the required list, and when shape is round also radius_cm, and when shape is square also side_cm, hollow, corners, and when corners is rounded also corner_radius_cm. A key that goes with the other choice is refused by name."
            })
        );
        let text = schema.to_string();
        for word in ["oneOf", "anyOf", "allOf", "\"if\"", "\"not\""] {
            assert!(!text.contains(word), "{word}");
        }
    }

    fn request(parameters: Value) -> Value {
        json!({ "solver": "switched", "parameters": parameters })
    }

    fn sealed(parameters: Value) -> Value {
        let given = read(parameters.clone());
        let solved = (SWITCHED.solve)(&given).expect("solved");
        let response = solution::seal(&SWITCHED, request(parameters), solved).expect("sealed");
        serde_json::from_str(&serde_json::to_string(&response).unwrap()).unwrap()
    }

    /// A value asked for by name is in the solution and in its proof. A
    /// solution that was not asked for it holds no such key and no such line.
    #[test]
    fn a_value_is_returned_and_sealed_when_asked_for_and_only_then() {
        let asked = sealed(square());
        assert_eq!(asked["result"]["marks_doubled_cm"], json!([2.0, 4.0, 8.0]));
        assert_eq!(
            asked["result"]["marks_doubled_cm_hex"],
            json!(["4000000000000000", "4010000000000000", "4020000000000000"])
        );
        let lines = "result.area_cm2_hex:4022000000000000\nresult.marks_doubled_cm_hex:[4000000000000000,4010000000000000,4020000000000000]\n";
        assert_eq!(asked["seal_parts"]["result"], digest_hex(lines.as_bytes()));

        let not_asked = sealed(with(square(), "return_marks", json!(false)));
        let result = not_asked["result"].as_object().expect("result");
        assert_eq!(result.keys().collect::<Vec<_>>(), ["area_cm2", "area_cm2_hex"]);
        assert_eq!(
            not_asked["seal_parts"]["result"],
            digest_hex(b"result.area_cm2_hex:4022000000000000\n")
        );
        assert_ne!(asked["solution_seal"], not_asked["solution_seal"]);
    }

    /// An entry that returns what was not asked for, or leaves out what was,
    /// is at fault, and nothing is sealed.
    #[test]
    fn a_value_returned_unasked_or_left_out_when_asked_is_not_sealed() {
        let answer = |with_marks: bool| {
            let mut result = vec![("area_cm2".to_string(), Published::Number(9.0))];
            if with_marks {
                result.push(("marks_doubled_cm".to_string(), Published::Numbers(vec![2.0])));
            }
            Solved::Answer(Answer {
                iterations: 1,
                result,
                invariants: Vec::new(),
                field: vec![1.0],
            })
        };
        let fault = |parameters: Value, with_marks: bool| {
            match solution::seal(&SWITCHED, request(parameters), answer(with_marks)) {
                Ok(_) => panic!("sealed"),
                Err(message) => message,
            }
        };
        assert_eq!(
            fault(with(square(), "return_marks", json!(false)), true),
            "Physical Violation: switched returned 'result.marks_doubled_cm', which this request did not ask for. Nothing is sealed."
        );
        assert_eq!(
            fault(square(), false),
            "Physical Violation: switched did not return 'result.marks_doubled_cm'. Nothing is sealed."
        );
        // A request with no flag at all asked for nothing.
        assert_eq!(
            fault(without(square(), "return_marks"), true),
            "Physical Violation: switched returned 'result.marks_doubled_cm', which this request did not ask for. Nothing is sealed."
        );
        assert!(solution::seal(&SWITCHED, request(square()), answer(true)).is_ok());
        assert!(
            solution::seal(
                &SWITCHED,
                request(with(square(), "return_marks", json!(false))),
                answer(false)
            )
            .is_ok()
        );
    }
}
