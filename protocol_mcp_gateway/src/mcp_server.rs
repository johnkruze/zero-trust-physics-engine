//! MCP tools over stdio. Each tool calls the same function the HTTP route calls.

use rmcp::ErrorData as McpError;
use rmcp::handler::server::ServerHandler;
use rmcp::model::{
    CallToolRequestParams, CallToolResponse, CallToolResult, ContentBlock, Implementation,
    ListToolsResult, PaginatedRequestParams, ProtocolVersion, ServerCapabilities, ServerConfig, Tool,
};
use rmcp::service::{RequestContext, RoleServer};
use serde::Serialize;
use serde_json::{Map, Value};

use crate::descriptor::{self};
use crate::view::View;
use crate::{
    AppState, DoorError, dataset, new_gateway_state, receipt_check, solution, solvers, trajectory,
    validation,
};

/// Every revision of the Model Context Protocol the gateway speaks, oldest
/// first. The local connection and the hosted connection both read it: it is
/// what `server/discover` publishes, what an `initialize` may agree to, what a
/// request's own version is checked against, and what the error for a version
/// not spoken lists.
pub const SUPPORTED_VERSIONS: [&str; 5] = ["2024-11-05", "2025-03-26", "2025-06-18", "2025-11-25", "2026-07-28"];
/// The first revision with no `initialize`: from it on, every request states
/// its own version and the client's capabilities in `params._meta`. A
/// revision is a date, so the earlier of two is the lesser text.
pub const FIRST_VERSION_WITHOUT_INITIALIZE: &str = "2026-07-28";

/// Whether the gateway speaks a revision.
pub fn speaks(version: &str) -> bool {
    SUPPORTED_VERSIONS.contains(&version)
}

/// Whether a revision begins with `initialize`.
pub fn begins_with_initialize(version: &str) -> bool {
    version < FIRST_VERSION_WITHOUT_INITIALIZE
}

pub struct McpGateway {
    state: AppState,
}

impl McpGateway {
    pub fn new() -> Self {
        Self {
            state: new_gateway_state(),
        }
    }

    /// The same door with a directory for dataset files, or none. The
    /// directory has been through `prepare_dataset_dir`.
    pub fn with_dataset_dir(dir: Option<std::path::PathBuf>) -> Self {
        Self {
            state: new_gateway_state().with_dataset_dir(dir),
        }
    }

    fn tools(&self) -> Vec<Tool> {
        tools_in(self.state.view())
    }
}

/// What the two kinds of refused run are, and what the two openings of a
/// refusal sentence mean. Stated once, in the description of `run_trajectory`;
/// the description of `run_dataset` points here.
pub(crate) const REFUSAL_WORDS: &str = "A run can be refused in two ways, and they come back differently. A request the door turns away before computing is an error, not a result: its text is one sentence, nothing is sealed, and when the sentence gives the card's name for the refusal the name is in round brackets. A run that starts and is refused part way is a result: outcome is \"refused\", reject holds the name (null when the cause has none), reject_detail holds the sentence, which ends \"The run is sealed to step N. The state at step N+1 was refused and is not sealed.\", and the run is sealed up to that last landed step N: steps_landed is N, the count of steps that landed, so the step that was refused is steps_landed + 1. A run that reaches an end its card declares (the first maximum of power of a reactor burst, a capsule's flown parachute deployment, a mission's last day, the end of a record's clock or track) is not refused: outcome is \"ended\", reject holds the end's name, reject_detail a sentence beginning \"Ended\" that names the end, and the run is sealed up to its last landed step; describe_card lists a card's declared ends under declared_ends. A sentence that begins \"Physical Failure\" says the state asked for or reached is outside the card: past a published bound, a limit of the specimen, or the conservation ledger. A sentence that begins \"Physical Violation\" says the request, or a number in it or computed from it, is not one the engine takes: its shape, a missing or unknown key, the number rule, the step floor or ceiling, a load rule, a size limit, or a value that is not finite.";

/// What the `run_trajectory` tool says of itself: how large an answer a
/// request makes, what keeps it small, and how a refusal comes back.
pub(crate) fn run_trajectory_description() -> String {
    format!(
        "One anchored run under a changing load. Same bytes as POST /run_trajectory. The size of the answer follows from the request: about 230 characters for each recorded state, plus the request as it was sent, plus up to {} characters for the final state and the seals. A state is recorded at each multiple of record_every and at the last landed step, so 10,000 steps with record_every 1 answer with about 2.3 million characters, and the same steps with record_every 10000 with one recorded state. What keeps it small: a larger record_every; the final state is returned in full whatever is recorded. To read quantities without the frames, add readout: {{\"quantities\": [names the card publishes], \"first_past\": {{\"quantity\": <one of them>, \"above\" or \"below\": <level>}}}}. The answer then holds each quantity in decimals at the recorded steps, its minimum and maximum over every landed step with the step, the time (the step times dt_s) and every other listed quantity at that step (at_minimum, at_maximum), and, when asked, the first landed step at which it is past the level, with every other listed quantity there (at_step); with record_every equal to the steps, one state is recorded and the readout gives the extremes of every step. The readout is not part of the sealed request: the run_seal is the seal of the same run without it, the receipt covers it, and at most {} quantities are read. With from (see its schema) the run starts from a sealed run, and is refused by name when the upstream's seal or a carried value does not hold. {REFUSAL_WORDS}",
        dataset::REST_OF_AN_ANSWER_CHARS,
        crate::readout::MAX_READOUT_QUANTITIES
    )
}

/// What the receipt of an answer covers, said where a caller reads a receipt.
pub(crate) const RECEIPT_FOLLOWS_THE_TEXT: &str = "The receipt covers the request as it was written, number spellings included, while a seal covers its values: a request sent with 310 and the same request sent with 310.0 give different receipts and the same seal, so a receipt that differs between two such answers is not a sign of tampering.";

/// The run a check by request is shown with: the cold plate's when the view
/// holds it, the worked run of the view's first card when it does not.
fn tape_example_request(view: &View) -> Option<Value> {
    let cards = view.cards();
    let card = cards
        .iter()
        .find(|item| (item.domain, item.card) == ("energy", "thermal"))
        .or_else(|| cards.first())?;
    if (card.domain, card.card) == ("energy", "thermal") {
        return Some(serde_json::json!({
            "domain": "energy",
            "card": "thermal",
            "initial": { "temperature_k": 293.15 },
            "dt_s": 1.0,
            "segments": [{ "steps": 60, "load": { "heat_power_w": 5.0, "ambient_k": 293.15 } }],
            "record_every": 60
        }));
    }
    let described = descriptor::describe_value_in(view, card.domain, card.card)?;
    Some(described["example_run_trajectory"].clone())
}

/// The solve a check by request is shown with: the atom's when the view
/// holds it, the worked solve of the view's first solver when it does not.
fn solution_example_request(view: &View) -> Option<Value> {
    let listed = view.solvers();
    let solver = listed
        .iter()
        .find(|solver| solver.name == "atom_dft")
        .or_else(|| listed.first())?;
    Some(solvers::example_solve(solver))
}

/// One worked check by request, as the text of its arguments.
fn by_request_example(request: Value, seal_key: &str, from: &str) -> String {
    format!(
        " Worked example: {{\"request\":{request},\"{seal_key}\":\"<the {seal_key} of its {from} answer, 64 lowercase hex digits>\"}}."
    )
}

/// What the `verify_tape` tool says of itself: the short check first.
pub(crate) fn verify_tape_description(view: &View) -> String {
    let mut text = String::from(
        "Check a run's seal. Short form, which needs only what is short: request is the run_trajectory request as it was sent, with its readout and its from when it had them (the answer's inputs holds neither), and run_seal is the seal its answer gave. The run is computed again, under a physics permit and the limits and refusals of run_trajectory, and the answer is holds (the seal given is the seal the request runs to), cause (\"run_seal\" when it is not), recomputed_run_seal and recomputed_receipt_sha256 when it holds (when it does not, no seal is returned: run_trajectory gives the true one), a sentence of what was checked, and text_not_compared: the short form compares no answer text you hold, so a copy with a decimal changed still holds; verify_receipt checks a copy against the receipt that copy carries, and the check is complete only when that receipt equals recomputed_receipt_sha256 here. Nothing is stored, so the short form answers the same on every connection.",
    );
    if let Some(request) = tape_example_request(view) {
        text.push_str(&by_request_example(request, "run_seal", "run_trajectory"));
    }
    text.push_str(" Long form: the whole run_trajectory answer sent back byte for byte, without request; it takes no physics permit, recomputes the chain, the state digest and the run seal, and checks the exact hex values and the seals, not the decimal copies of the values and not the receipt; verify_receipt checks a copy's bytes against the receipt it carries. A chain: its request with from, run_seal and chain_seal, computed again; or links, its answers in order, with no physics. Same bytes as POST /verify_tape.");
    text
}

/// What the `verify_solution` tool says of itself: the short check first.
pub(crate) fn verify_solution_description(view: &View) -> String {
    let mut text = String::from(
        "Check a solution's seal. Short form, which needs only what is short: request is the solve request as it was sent (the answer's inputs), and solution_seal is the seal its answer gave. The request is solved again, under a physics permit and the limits and refusals of solve, and the answer is holds, cause (\"solution_seal\" when the seal given is not the seal the request solves to), recomputed_solution_seal and recomputed_receipt_sha256 when it holds (when it does not, no seal is returned: solve gives the true one), a sentence of what was checked, and text_not_compared: the short form compares no answer text you hold, so a copy with a decimal changed still holds; verify_receipt checks a copy against the receipt that copy carries, and the check is complete only when that receipt equals recomputed_receipt_sha256 here. Nothing is stored, so the short form answers the same on every connection.",
    );
    if let Some(request) = solution_example_request(view) {
        text.push_str(&by_request_example(request, "solution_seal", "solve"));
    }
    text.push_str(" Long form: the whole solve answer sent back, without request; it takes no physics permit, recomputes the solution seal and names the part that fails, and checks only the exact hex values and the seals, not the decimal copies of the values and not the receipt, as its answer says; verify_receipt checks a copy's bytes against the receipt it carries. Same bytes as POST /verify_solution.");
    text
}

/// What the `verify_dataset` tool says of itself: the short check first.
pub(crate) fn verify_dataset_description(view: &View) -> String {
    let mut text = String::from(
        "Check a dataset's seal. Short form, which needs only what is short: request is the run_dataset request as it was sent (the head's request), and dataset_seal is the seal its answer gave. The plan is run again, under the permits and the limits of run_dataset, whatever its return mode and its answer form, and the answer is holds, cause (\"dataset_seal\" when the seal given is not the seal the plan runs to), recomputed_dataset_seal when it holds (when it does not, no seal is returned: run_dataset gives the true one), a sentence of what was checked, and text_not_compared: the short form compares no rows you hold. Nothing is stored or read from storage, so the short form answers the same on every connection.",
    );
    if let Some([(_, list), ..]) = dataset::worked_plans_in(view) {
        text.push_str(&by_request_example(list, "dataset_seal", "run_dataset"));
    }
    text.push_str(" A plan of solves is checked by this short form only. Two long forms take no physics permit. The three parts a full inline answer carries (answer \"full\": head, rows and frames_hex, all three together): the rows, the root and the seal are recomputed and the first break is named; a dataset made without the engine can pass, and replaying a row through run_trajectory shows the engine made it; the exact hex values and the seals are checked, not the decimal copies of the values in the rows, and holds does not cover the receipt, which the report states apart. Or dataset_seal alone, for a dataset the gateway stored (return \"file\"). Same bytes as POST /verify_dataset.");
    text
}

/// What the `validate_series` tool says of itself: a run, or a plan of solves.
pub(crate) fn validate_series_description() -> String {
    format!(
        "Run a run_trajectory request (without a readout) and compare one published quantity with the caller's measurements, each at a landed step. The answer is the error in numbers (rms, mean signed, greatest absolute, model minus measurement, each with its exact hex), the model value at every compared step in decimals (model_values) and exact (model_values_hex), and a hash of the measurements, under one seal. Same bytes as POST /validate_series. There is no pass or fail and no threshold: the numbers are not a verdict. The seal has no key, so it does not show who measured or who computed. genesis_seal verify-validation rebuilds the seal and the statistics from the answer alone, of a run or of a plan of solves. In place of run, solves takes a plan of solves as run_dataset takes it (solver and plan, at most {} solves): quantity is one number the solver publishes in its result, and each measurement is {{\"row\": <the plan's row, from 0>, \"value\": <number>}}. The solves are sealed under one dataset_seal as run_dataset seals them, and the comparison is sealed by the same statistics, measurement hash and validation seal as a run's, with the dataset_seal in the place of the run seal and the row in the place of the step; a measured row whose solve did not converge is counted in not_reached. The same request sent again gives the same seal.",
        crate::solve_plan::MAX_SOLVES
    )
}

/// What the `solve` tool says of itself.
pub(crate) fn solve_description() -> String {
    format!(
        "One sealed solution. Same bytes as POST /solve. Send parameters, or example in their place: the name of a worked example describe_solver lists (example_solve, or a key beginning example_solve_), so a long worked input need not be typed again; the request is filled with that example's parameters, and the filled request is the one solved, sealed and returned as inputs. {RECEIPT_FOLLOWS_THE_TEXT} To check a solution later, send this request, or its inputs, and its solution_seal to verify_solution."
    )
}

/// What the `verify_receipt` tool says of itself.
pub(crate) fn verify_receipt_description() -> String {
    format!(
        "Check the receipt at the end of one response. Same bytes as POST /verify_receipt. Takes no physics permit. The receipt covers every byte of the response before it, the decimal values included, and shows who produced the response or when in no way: it has no key. {RECEIPT_FOLLOWS_THE_TEXT} Replaying the request through run_trajectory, solve or verify_step shows the engine produced it; verify_tape, verify_solution and verify_dataset with the request and its seal do that replay and compare the seals."
    )
}

/// The tools as one caller sees them. The descriptions and input
/// schemas name the cards and solvers of the view and no others.
pub fn tools_in(view: &View) -> Vec<Tool> {
    {
        vec![
            Tool::new(
                "list_cards",
                format!("Every domain and card, each with one sentence, and count, how many there are. {}", crate::find_model::AGAINST_MEASUREMENT_SAYS),
                descriptor::list_schema(),
            ),
            Tool::new(
                "describe_card",
                format!("Inputs, units, bounds, step limits, reject names and meanings, advisory flags, frame slots, and worked examples for one pair. {}", descriptor::SHORT_FORM_SAYS),
                descriptor::describe_schema_in(view),
            ),
            Tool::new(
                "verify_step",
                "One anchored step. Same bytes as POST /verify_physics.",
                descriptor::verify_step_schema_in(view),
            ),
            Tool::new(
                "run_trajectory",
                run_trajectory_description(),
                descriptor::run_trajectory_schema_in(view),
            ),
            Tool::new(
                "verify_tape",
                verify_tape_description(view),
                descriptor::tape_schema_in(view),
            ),
            Tool::new(
                "list_solvers",
                format!("Every field solver, each with one sentence, and count, how many there are. {}", crate::find_model::AGAINST_MEASUREMENT_SAYS),
                solvers::list_schema(),
            ),
            Tool::new(
                "describe_solver",
                format!("Inputs and bounds, outputs, invariants and tolerances, refusal names and meanings, the published reference, and a worked example for one solver. {}", descriptor::SHORT_FORM_SAYS),
                solvers::describe_schema_in(view),
            ),
            Tool::new(
                "solve",
                solve_description(),
                solvers::solve_schema_in(view),
            ),
            Tool::new(
                "verify_solution",
                verify_solution_description(view),
                solvers::solution_schema_in(view),
            ),
            Tool::new(
                "run_dataset",
                dataset::run_description_in(view),
                dataset::run_schema(),
            ),
            Tool::new(
                "verify_dataset",
                verify_dataset_description(view),
                dataset::verify_schema(),
            ),
            Tool::new(
                "verify_receipt",
                verify_receipt_description(),
                receipt_check::schema(),
            ),
            Tool::new(
                "validate_series",
                validate_series_description(),
                validation::schema_in(view),
            ),
            Tool::new("cite", crate::cite::description(), crate::cite::schema_in(view)),
            Tool::new("find_model", crate::find_model::description(), crate::find_model::schema()),
            Tool::new("find_flip", crate::flip::description(), crate::flip::schema()),
        ]
    }
}

impl Default for McpGateway {
    fn default() -> Self {
        Self::new()
    }
}

impl ServerHandler for McpGateway {
    fn get_info(&self) -> ServerConfig {
        ServerConfig::new(ServerCapabilities::builder().enable_tools().build())
            // The first thing an agent reads: the verifier's name and its
            // maker, then the working instructions as they have been. The
            // version is the gateway's and the description the engine's, as
            // every answer carries it.
            .with_server_info(
                Implementation::new(crate::identity::NAME, env!("CARGO_PKG_VERSION"))
                    .with_title(crate::identity::TITLE)
                    .with_description(crate::ENGINE_VERSION)
                    .with_website_url(crate::identity::SITE),
            )
            .with_instructions(crate::identity::instructions())
    }

    fn supported_protocol_versions(&self) -> std::borrow::Cow<'static, [ProtocolVersion]> {
        std::borrow::Cow::Owned(
            SUPPORTED_VERSIONS
                .iter()
                .map(|version| {
                    serde_json::from_value(Value::String(version.to_string())).expect("a version is a string")
                })
                .collect(),
        )
    }

    fn list_tools(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> impl std::future::Future<Output = Result<ListToolsResult, McpError>> + Send + '_ {
        let tools = self.tools();
        std::future::ready(Ok(ListToolsResult::with_all_items(tools)))
    }

    fn call_tool(
        &self,
        request: CallToolRequestParams,
        _context: RequestContext<RoleServer>,
    ) -> impl std::future::Future<Output = Result<CallToolResponse, McpError>> + Send + '_ {
        let name = request.name.to_string();
        let arguments = request.arguments.unwrap_or_default();
        let state = self.state.clone();
        async move { Ok(dispatch(&state, &name, arguments).await.into()) }
    }
}

/// One tool call, as the stdio door and the hosted door both make it. The
/// caller's view is the state's.
pub async fn dispatch(state: &AppState, name: &str, arguments: Map<String, Value>) -> CallToolResult {
    let value = Value::Object(arguments);
    let result = match name {
        "list_cards" => encoded(&descriptor::list_value_in(state.view())),
        "describe_card" => descriptor::describe_request_in(state.view(), &value).map(|body| encoded_value(&body)),
        "verify_step" => match crate::verify_step(state, value).await {
            Ok(response) => encoded(&response),
            Err(error) => Err(error),
        },
        "run_trajectory" => match trajectory::run(state, value).await {
            Ok(response) => encoded(&response),
            Err(error) => Err(error),
        },
        "verify_tape" => trajectory::verify(state, &value).await,
        "list_solvers" => encoded(&solvers::list_value_in(state.view())),
        "describe_solver" => solvers::describe_request_in(state.view(), &value).map(|body| encoded_value(&body)),
        "solve" => match solution::run(state, value).await {
            Ok(response) => encoded(&response),
            Err(error) => Err(error),
        },
        "verify_solution" => solution::verify(state, &value).await,
        "run_dataset" => dataset::run(state, value).await,
        "verify_dataset" => dataset::check(state, &value).await,
        "validate_series" => validation::run(state, value).await,
        "verify_receipt" => match receipt_check::check(&value) {
            Ok(report) => encoded(&report),
            Err(error) => Err(error),
        },
        "cite" => crate::cite::cite_in(state.view(), &value).map(|body| encoded_value(&body)),
        "find_model" => crate::find_model::find_in(state.view(), &value).map(|body| encoded_value(&body)),
        "find_flip" => crate::flip::find(state, value).await,
        _ => Err(DoorError::NotFound(format!("Tool not recognized: {name}"))),
    };
    match result {
        Ok(text) => CallToolResult::success(vec![ContentBlock::text(text)]),
        Err(error) => CallToolResult::error(vec![ContentBlock::text(error.message())]),
    }
}

fn encoded<T: Serialize>(value: &T) -> Result<String, DoorError> {
    serde_json::to_string(value).map_err(|error| {
        DoorError::Internal(format!(
            "Physical Violation: the response could not be encoded. {error}"
        ))
    })
}

fn encoded_value(value: &Value) -> String {
    serde_json::to_string(value).expect("descriptor encodes")
}
