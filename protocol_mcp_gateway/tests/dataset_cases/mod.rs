//! Dataset requests the tests share. Pure JSON, so the unit tests inside the
//! crate and the router tests build the same requests.
#![allow(dead_code)]

use serde_json::{Value, json};

/// The wrapper every dataset request has. An inline request asks for the
/// full rows, which the tests read.
pub fn request(plan: Value, store: &str, returns: &str) -> Value {
    let mut request = json!({
        "domain": "energy",
        "card": "thermal",
        "plan": plan,
        "store": store,
        "return": returns
    });
    if returns == "inline" {
        request["answer"] = json!("full");
    }
    request
}

/// A whole run on `energy`/`thermal`, as a `list` plan carries it.
pub fn run(temperature_k: f64, heat_power_w: f64, steps: u64) -> Value {
    json!({
        "initial": { "temperature_k": temperature_k },
        "dt_s": 0.1,
        "segments": [{
            "steps": steps,
            "load": { "heat_power_w": heat_power_w, "ambient_k": 293.15 }
        }],
        "record_every": 1
    })
}

/// Three runs.
pub fn list_request(store: &str) -> Value {
    request(
        json!({
            "kind": "list",
            "runs": [run(280.0, 0.0, 4), run(290.0, 2.5, 3), run(300.0, 5.0, 5)]
        }),
        store,
        "inline",
    )
}

/// Six rows. The last axis, heat power, varies fastest.
pub fn grid_request(store: &str) -> Value {
    request(
        json!({
            "kind": "grid",
            "base": {
                "initial": {},
                "dt_s": 0.1,
                "segments": [{ "steps": 3, "load": { "ambient_k": 293.15 } }],
                "record_every": 1
            },
            "axes": [
                { "path": "initial.temperature_k", "values": [280.0, 290.0, 300.0] },
                { "path": "segments[0].load.heat_power_w", "values": [0.0, 5.0] }
            ]
        }),
        store,
        "inline",
    )
}

pub const SAMPLE_SEED: &str = "0123456789abcdef";

/// Five rows drawn by the counter sampler.
pub fn sample_request(store: &str) -> Value {
    request(
        json!({
            "kind": "sample",
            "base": {
                "initial": {},
                "segments": [{ "steps": 3, "load": { "ambient_k": 293.15 } }],
                "record_every": 1
            },
            "rows": 5,
            "generator": "splitmix64_counter_v1",
            "seed_hex": SAMPLE_SEED,
            "draws": [
                { "path": "initial.temperature_k", "uniform": { "low": 250.0, "high": 390.0 } },
                { "path": "segments[0].load.heat_power_w", "uniform": { "low": -5.0, "high": 5.0 } },
                { "path": "dt_s", "choice": [0.05, 0.1, 0.2] }
            ]
        }),
        store,
        "inline",
    )
}

/// A whole-job refusal: a name, the request text, and a piece of the sentence.
pub struct Refusal {
    pub name: &'static str,
    pub body: String,
    pub says: &'static str,
}

fn refusal(name: &'static str, body: Value, says: &'static str) -> Refusal {
    Refusal {
        name,
        body: serde_json::to_string(&body).unwrap(),
        says,
    }
}

/// A grid of `a` heat powers and `b` temperatures on a base of `steps` steps.
fn grid_of(a: u64, b: u64, steps: u64, record_every: u64, store: &str, returns: &str) -> Value {
    let powers: Vec<f64> = (0..a).map(|i| i as f64 * 0.01).collect();
    let mut axes = vec![json!({ "path": "segments[0].load.heat_power_w", "values": powers })];
    if b > 1 {
        let temperatures: Vec<f64> = (0..b).map(|i| 250.0 + i as f64 * 0.001).collect();
        axes.insert(0, json!({ "path": "initial.temperature_k", "values": temperatures }));
    }
    let mut initial = json!({});
    if b <= 1 {
        initial = json!({ "temperature_k": 293.15 });
    }
    request(
        json!({
            "kind": "grid",
            "base": {
                "initial": initial,
                "dt_s": 0.1,
                "segments": [{ "steps": steps, "load": { "ambient_k": 293.15 } }],
                "record_every": record_every
            },
            "axes": axes
        }),
        store,
        returns,
    )
}

/// Faults of the plan itself. Each refuses the whole job.
pub fn plan_refusals() -> Vec<Refusal> {
    let grid = |axes: Value, base: Value| {
        request(
            json!({ "kind": "grid", "base": base, "axes": axes }),
            "frames",
            "inline",
        )
    };
    let base = json!({
        "initial": {},
        "dt_s": 0.1,
        "segments": [{ "steps": 3, "load": { "ambient_k": 293.15 } }],
        "record_every": 1
    });
    let mut cases = vec![
        refusal(
            "a field set by two axes",
            grid(
                json!([
                    { "path": "initial.temperature_k", "values": [290.0] },
                    { "path": "initial.temperature_k", "values": [291.0] },
                    { "path": "segments[0].load.heat_power_w", "values": [1.0] }
                ]),
                base.clone(),
            ),
            "is set twice",
        ),
        refusal(
            "a field not set",
            grid(
                json!([{ "path": "initial.temperature_k", "values": [290.0] }]),
                base.clone(),
            ),
            "is not set",
        ),
        refusal(
            "a path to steps",
            grid(
                json!([
                    { "path": "initial.temperature_k", "values": [290.0] },
                    { "path": "segments[0].load.heat_power_w", "values": [1.0] },
                    { "path": "segments[0].steps", "values": [3] }
                ]),
                base.clone(),
            ),
            "cannot vary",
        ),
        refusal(
            "a path to record_every",
            grid(
                json!([
                    { "path": "initial.temperature_k", "values": [290.0] },
                    { "path": "segments[0].load.heat_power_w", "values": [1.0] },
                    { "path": "record_every", "values": [1] }
                ]),
                base.clone(),
            ),
            "cannot vary",
        ),
        refusal(
            "a card constant as a path",
            grid(
                json!([
                    { "path": "initial.temperature_k", "values": [290.0] },
                    { "path": "segments[0].load.heat_power_w", "values": [1.0] },
                    { "path": "segments[0].load.plate_capacitance_j_k", "values": [600.0] }
                ]),
                base.clone(),
            ),
            "plate_capacitance_j_k",
        ),
    ];
    let mut unknown = list_request("frames");
    unknown["workers"] = json!(4);
    cases.push(refusal("an unknown key", unknown, "'workers' is not a field"));
    cases.push(refusal(
        "an empty plan",
        request(json!({ "kind": "list", "runs": [] }), "frames", "inline"),
        "the plan has no rows",
    ));
    cases.push(refusal(
        "return file",
        list_request("frames").as_object().map(|object| {
            let mut object = object.clone();
            object.insert("return".into(), json!("file"));
            object.remove("answer");
            Value::Object(object)
        }).unwrap(),
        "this connection was started without a directory for dataset files.",
    ));
    cases
}

/// The same faults for text that is not a value: a repeated key at the top
/// and inside a run.
pub fn repeated_key_bodies() -> Vec<(&'static str, String)> {
    vec![
        (
            "a repeated top-level key",
            r#"{"domain":"energy","card":"thermal","plan":{"kind":"list","runs":[]},"store":"frames","store":"none","return":"inline"}"#.to_string(),
        ),
        (
            "a repeated key inside a run",
            r#"{"domain":"energy","card":"thermal","plan":{"kind":"list","runs":[{"initial":{"temperature_k":290.0,"temperature_k":291.0},"dt_s":0.1,"segments":[{"steps":3,"load":{"heat_power_w":1.0,"ambient_k":293.15}}],"record_every":1}]},"store":"frames","return":"inline"}"#.to_string(),
        ),
    ]
}

/// The four limits of a job, each just passed. The numbers are the gateway's.
pub fn limit_refusals(
    rows: u64,
    steps: u64,
    frames: u64,
    inline_rows: u64,
    inline_frames: u64,
) -> Vec<Refusal> {
    let rows_axis = rows / 1000 + 1;
    let steps_axis = steps / 1_000_000 + 1;
    let frames_axis = frames / 10_000 + 1;
    let inline_frames_axis = inline_frames / 1000 + 1;
    vec![
        refusal(
            "more rows than a job may hold",
            grid_of(1000, rows_axis, 1, 1, "none", "inline"),
            "the dataset asks for more than 100000 runs.",
        ),
        refusal(
            "more steps in total than a job may take",
            grid_of(steps_axis, 1, 1_000_000, 100, "none", "inline"),
            "the dataset asks for more than 100000000 steps in total.",
        ),
        refusal(
            "more frames than a job may store",
            grid_of(frames_axis, 1, 10_000, 1, "frames", "inline"),
            "the dataset would record more than 16777216 frames.",
        ),
        refusal(
            "more rows than may come back inline",
            grid_of(inline_rows + 1, 1, 1, 1, "none", "inline"),
            "the dataset is too large to return inline.",
        ),
        refusal(
            "more frames than may come back inline",
            grid_of(inline_frames_axis, 1, 1000, 1, "frames", "inline"),
            "the dataset is too large to return inline.",
        ),
        refusal(
            "a run longer than a run may be",
            grid_of(1, 1, 1_000_001, 1000, "none", "inline"),
            "the run asks for more than 1000000 steps.",
        ),
    ]
}
