//! MCP tools over stdio. Each tool calls the same function the HTTP route calls.

use rmcp::ErrorData as McpError;
use rmcp::handler::server::ServerHandler;
use rmcp::model::{
    CallToolRequestParams, CallToolResponse, CallToolResult, ContentBlock, Implementation,
    ListToolsResult, PaginatedRequestParams, ProtocolVersion, ServerCapabilities, ServerConfig, Tool,
    ToolAnnotations,
};
use rmcp::service::{RequestContext, RoleServer};
use serde::Serialize;
use serde_json::{Map, Value};

use crate::descriptor::{self};
use crate::view::View;
use crate::{
    AppState, DoorError, dataset, new_gateway_state, verify_receipt, solution, solvers, trajectory,
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
    /// A test sets it to call a tool as engine 0.11.0 did, with no guard,
    /// to show what a panic did then.
    #[cfg(test)]
    pub(crate) unguarded: bool,
}

impl McpGateway {
    pub fn new() -> Self {
        Self {
            state: new_gateway_state(),
            #[cfg(test)]
            unguarded: false,
        }
    }

    /// The same door with a directory for dataset files, or none. The
    /// directory has been through `prepare_dataset_dir`.
    pub fn with_dataset_dir(dir: Option<std::path::PathBuf>) -> Self {
        Self {
            state: new_gateway_state().with_dataset_dir(dir),
            #[cfg(test)]
            unguarded: false,
        }
    }

    fn tools(&self) -> Vec<Tool> {
        tools_in(self.state.view())
    }
}

/// How a run comes back, in full: the two ways it is refused, an end its card
/// declares, and what the two openings of a refusal sentence mean. The
/// description of `run_trajectory` says it in short; this is placed in its
/// input schema, where the length a description is held to does not apply.
pub(crate) const REFUSAL_WORDS: &str = "A run can be refused in two ways, and they come back differently. A request turned away before computing is an error, not a result: its text is one sentence, nothing is sealed, and when the sentence gives the card's name for the refusal the name is in round brackets. A run that starts and is refused part way is a result: outcome is \"refused\", reject holds the name (null when the cause has none), reject_detail holds the sentence, which ends \"The run is sealed to step N. The state at step N+1 was refused and is not sealed.\", and the run is sealed up to that last landed step N: steps_landed is N, the count of steps that landed, so the step that was refused is steps_landed + 1. A run that reaches an end its card declares (the first maximum of power of a reactor burst, a capsule's flown parachute deployment, a crater that stops growing, a cell's runaway once every reactant is consumed) is not refused: outcome is \"ended\", reject holds the end's name, reject_detail a sentence beginning \"Ended\" that names the end, and the run is sealed up to its last landed step; describe_card lists a card's declared ends under declared_ends. A sentence that begins \"Physical Failure\" says the state asked for or reached is outside the card: past a published bound, a limit of the specimen, or the conservation ledger. A sentence that begins \"Physical Violation\" says the request, or a number in it or computed from it, is not one the engine takes: its shape, a missing or unknown key, the number rule, the step floor or ceiling, a load rule, a size limit, or a value that is not finite. A required key left out is named InputMissing, and a card the engine does not serve UnknownCard. A sentence with no name in round brackets is of a request that is not well formed (a key unknown, a value of the wrong type, a number the number rule refuses, a count or a size past the tool's own limit) or of a value that is not finite: no model stopped, and the request is what is corrected.";

/// What the `compute_step` tool says of itself.
pub(crate) fn compute_step_description() -> String {
    "Computes one anchored step of one card and returns the state it lands on, in decimals and exact hex, with its cryptographic proof (sha256_seal) and a receipt. parameters holds the card's phase fields, its load fields and dt_s, as describe_card gives them. A step whose request or whose landing state is outside the card is refused in one sentence, with the refusal's name in round brackets when it has one, and no proof is issued for that state. With from, the step starts from a sealed run, which is computed again first.".to_string()
}

/// The earlier name of `compute_step`, served for one version beside it: the
/// same input schema, and the same answer, byte for byte, to the same
/// arguments. It says what it does first, then that it is that earlier name.
pub(crate) const EARLIER_COMPUTE_STEP: &str = "verify_step";

/// What `verify_step` says of itself while it is served.
pub(crate) fn verify_step_description() -> String {
    "Computes one anchored step of one card, as compute_step does: verify_step is the earlier name of compute_step, served for this one version beside it, and takes the same arguments and gives the same answer, byte for byte. The name compute_step is the one that stays.".to_string()
}

/// The name a required key left out is refused under, said at the inputs of
/// the step; the run says it in `REFUSAL_WORDS`.
pub(crate) const MISSING_WORDS: &str = "A required field left out is refused as InputMissing, in a sentence that names the field, and a card the engine does not serve as UnknownCard; nothing is filled in.";

/// The names of a solve's own malformed inputs, said at its parameters.
pub(crate) const SOLVE_MISSING_WORDS: &str = "A required input left out is refused as InputMissing, and an input given with a choice it does not go with as InputNotWithChoice, each in a sentence that names the input, and a solver the engine does not serve as UnknownSolver; nothing is filled in.";

/// What the `run_trajectory` tool says of itself: what it does and returns,
/// what keeps an answer small, and how a run comes back.
pub(crate) fn run_trajectory_description() -> String {
    format!("Runs one card from a start state through segments of steps, each under its own load, and seals the run. The answer holds the outcome, the final state in decimals and exact hex, a frame for each recorded step, the run_seal and a receipt. record_every sets which steps are recorded, about 230 characters each; the final state comes back in full whatever is recorded. readout gives named quantities in decimals without the frames, with each one's minimum and maximum over every landed step and the first step past a level; it is not part of the sealed request. from starts the run from a sealed run, which is computed again first. A run comes back in one of four ways. outcome \"completed\": every step landed. outcome \"ended\": the run reached an end its card declares, which describe_card lists under declared_ends; reject holds the end's name. outcome \"refused\": a step part way was refused; reject holds the name (null when the cause has none) and reject_detail the sentence. An ended or a refused run is sealed up to its last landed step, steps_landed. An error, not a result: the request was turned away before computing, in one sentence with the refusal's name in round brackets when it has one, and nothing is sealed. A sentence that begins \"Physical Failure\" says the state asked for or reached is outside the card; one that begins \"Physical Violation\" says the request, or a number in it or computed from it, is not one the engine takes. {} The input schema says each of these in full.", crate::signed_load::RUN_SAYS)
}

/// What the `record_every` input of `run_trajectory` says: which steps are
/// recorded and how large an answer a request makes.
pub(crate) fn record_every_says() -> String {
    format!(
        "A state is recorded at each multiple of record_every and at the last landed step. The size of the answer follows from the request: about 230 characters for each recorded state, plus the request as it was sent, plus up to {} characters for the final state and the seals. So 10,000 steps with record_every 1 answer with about 2.3 million characters, and the same steps with record_every 10000 with one recorded state. What keeps it small: a larger record_every; the final state is returned in full whatever is recorded.",
        dataset::REST_OF_AN_ANSWER_CHARS
    )
}

/// What the receipt of an answer covers, said where an agent reads a receipt.
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

/// One worked check by request, as the text of its arguments. It is said at
/// the `request` input of each of the three checks, since its length follows
/// the worked request of the view.
fn by_request_example(request: Value, seal_key: &str, from: &str) -> String {
    format!(
        "Worked example of the short form, as the whole arguments: {{\"request\":{request},\"{seal_key}\":\"<the {seal_key} of its {from} answer, 64 lowercase hex digits>\"}}."
    )
}

/// What the `verify_tape` tool says of itself: the short check first.
pub(crate) fn verify_tape_description() -> String {
    "Verifies the seal of a run. Short form, which needs only what is short: request is the run_trajectory request as it was sent, with its readout and its from, whose request has none, when it had them (the answer's inputs holds neither), and run_seal is the seal its answer gave. The run is computed again, under a physics permit and the limits and refusals of run_trajectory, and the answer is holds (the seal given is the seal the request runs to), cause (\"run_seal\" when it is not), recomputed_run_seal and recomputed_receipt_sha256 when it holds (else no seal: run_trajectory gives the true one), compared, and text_not_compared: no answer text the agent holds is compared. Long form: the whole run_trajectory answer sent back as it came, without request. Its state digest and run seal are recomputed from its exact hex values, then its request is computed again under a physics permit and the engine's answer compared with it, value by value and byte for byte, receipt included: holds is true only when every byte is the engine's, values_hold when every value is, else cause names the seal, load_signature (a signed load that does not verify), answer_text (with differs_at, the first field that differs) or engine_version (an answer of another version). A chain is verified by its request with from, run_seal and chain_seal, or by links, its answers in order, each computed again from the links before it; a chained answer alone is refused (AnswerCarriesNoRequest). Nothing is stored, so each form answers the same on every connection.".to_string()
}

/// What the `verify_solution` tool says of itself: the short check first.
pub(crate) fn verify_solution_description() -> String {
    "Verifies the seal of a solution. Short form, which needs only what is short: request is the solve request as it was sent (the answer's inputs), and solution_seal is the seal its answer gave. The request is solved again, under a physics permit and the limits and refusals of solve, and the answer is holds, cause (\"solution_seal\" when the seal given is not the seal the request solves to), recomputed_solution_seal and recomputed_receipt_sha256 when it holds (else no seal: solve gives the true one), compared, and text_not_compared: no answer text the agent holds is compared. Long form: the whole solve answer sent back as it came, without request. Its solution seal is recomputed from its exact hex values and the part that fails is named, then its request is solved again under a physics permit and the engine's answer compared with it, value by value and byte for byte, receipt included: holds is true only when every byte is the engine's, values_hold when every value is, else cause names the part, solution_seal (the request solves to another seal), answer_text (with differs_at, the first field that differs) or engine_version (an answer of another version). Nothing is stored, so each form answers the same on every connection.".to_string()
}

/// What the `verify_dataset` tool says of itself on every connection: the
/// short check first, then the check of a full answer's three parts.
pub(crate) fn verify_dataset_description() -> String {
    "Verifies the seal of a dataset. Short form: request is the run_dataset request as it was sent, and dataset_seal is the seal its answer gave. The plan is run again under run_dataset's permits and limits; the answer is holds, cause (\"dataset_seal\" when the seal given is not the seal the plan runs to), recomputed_dataset_seal when it holds (else no seal: run_dataset gives the true one), compared, and text_not_compared: no rows the agent holds are compared. A plan of solves is verified by this short form only. Long form: head, rows and frames_hex, the three parts a full answer carries (answer \"full\"), together, as they came. The rows, the root and the seal are recomputed, and row and fault name the first row changed, swapped, dropped or repeated; then the plan is run again under run_dataset's permits and limits and the engine's answer compared with it, value by value and byte for byte (cause answer_text, with differs_at and row): holds is true only when every byte is the engine's, values_hold when every value is.".to_string()
}

/// What `run_dataset` says after its own words on a connection that may
/// store a dataset as files. The open connection stores none and does not
/// say it.
pub(crate) const RUN_DATASET_FILES_SAYS: &str = " With return \"file\", on a connection started with a directory for dataset files, the rows are stored and the answer is the head and the names of the files, whatever the plan's size and whatever answer says; verify_dataset verifies a stored dataset by its dataset_seal.";

/// What `verify_dataset` says after its own words on a connection that may
/// store a dataset as files.
pub(crate) const VERIFY_DATASET_FILES_SAYS: &str = " A dataset stored as files (return \"file\") is also verified by its dataset_seal alone: its seals are recomputed from the files, its plan is run again into a folder of its own, and the stored files are compared with that run's byte for byte; the short form runs a plan again whatever its return mode.";

/// What the `validate_series` tool says of itself: a run, or a plan of solves.
pub(crate) fn validate_series_description() -> String {
    format!(
        "Runs a run_trajectory request (without a readout) and compares one published quantity with the agent's measurements, each at a landed step. The answer is the error in numbers (rms, mean signed, greatest absolute, model minus measurement, each with its exact hex), the model value at every compared step in decimals (model_values) and exact (model_values_hex), and a hash of the measurements, under one seal. There is no pass or fail and no threshold: the numbers are not a verdict. The seal has no key, so it does not show who measured or who computed. commitment, optional, opens a salted SHA-256 the measurements were locked with before the comparison: data that are not the committed ones are refused before computing (CommitmentDoesNotMatch), and the seal covers it. genesis_seal verify-validation rebuilds the seal, the statistics and the commitment from the answer alone. In place of run, solves takes a plan of solves as run_dataset takes it (solver and plan, at most {} solves), each measurement at a row of the plan: the solves are sealed under one dataset_seal, and the comparison by the same rules with the dataset_seal in the place of the run seal and the row in the place of the step; a measured row whose solve did not converge is counted in not_reached. The same request sent again gives the same seal.",
        crate::solve_plan::MAX_SOLVES
    )
}

/// What the `solve` tool says of itself.
pub(crate) fn solve_description() -> String {
    format!(
        "Solves one field problem with one solver and seals the solution: the outcome, the result in decimals and exact hex, its invariants, the solution_seal and a receipt. parameters holds the solver's inputs as describe_solver gives them. example, in their place, is the name of a worked example describe_solver lists (example_solve, or a key beginning example_solve_), so a long worked input need not be typed again; the request is filled with that example's parameters, and the filled request is the one solved, sealed and returned as inputs. {RECEIPT_FOLLOWS_THE_TEXT} verify_solution verifies a solution later from this request, or its inputs, and its solution_seal."
    )
}

/// What the `verify_receipt` tool says of itself.
pub(crate) fn verify_receipt_description() -> String {
    format!(
        "Verifies the receipt at the end of one response against the text before it. It takes no physics permit. The receipt covers every byte of the response before it, the decimal values included, and shows who produced the response or when in no way: it has no key. A response that names an engine_version has it said beside matches, with whether it is this engine's. {RECEIPT_FOLLOWS_THE_TEXT} Replaying the request through run_trajectory, solve or compute_step shows the engine produced it; verify_tape, verify_solution and verify_dataset with the request and its seal do that replay and compare the seals."
    )
}

/// A tool's title: its name with spaces, the first letter a capital.
pub(crate) fn title_of(tool: &str) -> String {
    let mut words = tool.replace('_', " ");
    if let Some(first) = words.get_mut(0..1) {
        first.make_ascii_uppercase();
    }
    words
}

/// Adds words to what an input schema says of one input: after the words the
/// input has, or as its words when it has none. With no input named, the
/// words are added to what the schema says of the request as a whole. What a
/// description has no room for is said here, at the input it is about: the
/// length a description is held to does not apply to a schema.
fn say(schema: &mut Map<String, Value>, input: Option<&str>, words: &str) {
    if words.is_empty() {
        return;
    }
    let at = match input {
        Some(name) => schema
            .get_mut("properties")
            .and_then(|properties| properties.get_mut(name))
            .and_then(Value::as_object_mut),
        None => Some(schema),
    };
    let Some(at) = at else { return };
    let said = match at.get("description").and_then(Value::as_str) {
        Some(before) if !before.is_empty() => format!("{before} {words}"),
        _ => words.to_string(),
    };
    at.insert("description".to_string(), Value::String(said));
}

/// The tools as an agent of this view reads them on the local connection
/// and on a hosted connection with a key. The descriptions and input schemas
/// name the cards and solvers of the view and no others.
pub fn tools_in(view: &View) -> Vec<Tool> {
    tools_for(view, None)
}

/// The tools of one connection. Given the open connection's rules, each
/// description leaves out what that connection does not serve, a stored
/// dataset, and ends with that connection's own limits for the tool; and
/// each tool carries the hints a client reads to know that no tool there
/// changes anything outside its own answer. Every tool carries a title.
pub(crate) fn tools_for(view: &View, open: Option<&crate::open_door::OpenRules>) -> Vec<Tool> {
    let by_request = |example: Option<Value>, seal_key: &str, from: &str| {
        example.map(|request| by_request_example(request, seal_key, from)).unwrap_or_default()
    };

    let mut step_schema = descriptor::compute_step_schema_in(view);
    say(&mut step_schema, Some("parameters"), MISSING_WORDS);
    let mut solve_schema = solvers::solve_schema_in(view);
    say(&mut solve_schema, Some("parameters"), SOLVE_MISSING_WORDS);
    say(&mut solve_schema, Some("parameters"), crate::signed_load::SOLVE_SAYS);

    let mut run_schema = descriptor::run_trajectory_schema_in(view);
    say(&mut run_schema, Some("record_every"), &record_every_says());
    say(&mut run_schema, Some("segments"), crate::signed_load::SIGNED_SAYS);
    say(&mut run_schema, Some("segments"), crate::signed_load::LOAD_SOURCE_SAYS);
    say(&mut run_schema, None, REFUSAL_WORDS);

    let mut tape_schema = descriptor::tape_schema_in(view);
    say(&mut tape_schema, Some("request"), &by_request(tape_example_request(view), "run_seal", "run_trajectory"));

    let mut solution_schema = solvers::solution_schema_in(view);
    say(&mut solution_schema, Some("request"), &by_request(solution_example_request(view), "solution_seal", "solve"));

    let mut dataset_schema = dataset::run_schema();
    say(&mut dataset_schema, Some("plan"), &dataset::plan_says_in(view));
    say(&mut dataset_schema, Some("answer"), &dataset::answer_says());
    say(&mut dataset_schema, Some("solver"), &dataset::solves_say_in(view));

    let mut dataset_verify_schema = dataset::verify_schema();
    let listed = dataset::worked_plans_in(view).map(|[(_, list), ..]| list);
    say(&mut dataset_verify_schema, Some("request"), &by_request(listed, "dataset_seal", "run_dataset"));

    // A stored dataset is said only where one may be stored.
    let (run_files, check_files) = if open.is_some() { ("", "") } else { (RUN_DATASET_FILES_SAYS, VERIFY_DATASET_FILES_SAYS) };

    let declared: Vec<(&str, String, Map<String, Value>)> = vec![
        (
            "list_cards",
            format!("Lists every domain and card this connection serves, each with one sentence, and count, how many there are. {}", crate::find_model::AGAINST_MEASUREMENT_SAYS),
            descriptor::list_schema(),
        ),
        (
            "describe_card",
            format!("Describes one domain and card pair: inputs, units, bounds, step limits, reject names and meanings, advisory flags, frame slots, and worked examples. {}", descriptor::SHORT_FORM_SAYS),
            descriptor::describe_schema_in(view),
        ),
        ("compute_step", compute_step_description(), step_schema.clone()),
        ("run_trajectory", run_trajectory_description(), run_schema),
        ("verify_tape", verify_tape_description(), tape_schema),
        (
            "list_solvers",
            format!("Lists every field solver this connection serves, each with one sentence, and count, how many there are. {}", crate::find_model::AGAINST_MEASUREMENT_SAYS),
            solvers::list_schema(),
        ),
        (
            "describe_solver",
            format!("Describes one solver: inputs and bounds, outputs, invariants and tolerances, refusal names and meanings, the published reference, and a worked example. {}", descriptor::SHORT_FORM_SAYS),
            solvers::describe_schema_in(view),
        ),
        ("solve", solve_description(), solve_schema),
        ("verify_solution", verify_solution_description(), solution_schema),
        ("run_dataset", format!("{}{run_files}", dataset::run_description()), dataset_schema),
        ("verify_dataset", format!("{}{check_files}", verify_dataset_description()), dataset_verify_schema),
        ("verify_receipt", verify_receipt_description(), verify_receipt::schema()),
        ("validate_series", validate_series_description(), validation::schema_in(view)),
        ("cite", crate::cite::description(), crate::cite::schema_in(view)),
        ("find_model", crate::find_model::description(), crate::find_model::schema()),
        ("find_flip", crate::flip::description(), crate::flip::schema()),
        ("design_run", crate::design::description(), crate::design::schema()),
        (EARLIER_COMPUTE_STEP, verify_step_description(), step_schema),
    ];
    declared
        .into_iter()
        .map(|(name, description, schema)| {
            let title = title_of(name);
            match open {
                Some(rules) => Tool::new(name, format!("{description}{}", rules.says_of(name)), schema)
                    .with_title(title.clone())
                    .with_annotations(
                        ToolAnnotations::with_title(title).read_only(true).destructive(false).idempotent(true).open_world(false),
                    ),
                None => Tool::new(name, description, schema).with_title(title),
            }
        })
        .collect()
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
            // maker, then what the tools are for. The version is the
            // gateway's and the description the engine's, as every answer
            // carries it.
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
        context: RequestContext<RoleServer>,
    ) -> impl std::future::Future<Output = Result<CallToolResponse, McpError>> + Send + '_ {
        let name = request.name.to_string();
        let arguments = request.arguments.unwrap_or_default();
        // The exact text of the arguments, which the stdio door hands over
        // beside the call for a verification's long form.
        let text = context
            .meta
            .get(crate::guard::ARGUMENTS_TEXT)
            .and_then(Value::as_str)
            .map(str::to_string);
        let state = self.state.clone();
        #[cfg(test)]
        let unguarded = self.unguarded;
        async move {
            #[cfg(test)]
            if unguarded {
                return Ok(dispatch_text(&state, &name, arguments, text).await.into());
            }
            Ok(dispatch_guarded_text(&state, &name, arguments, text).await.into())
        }
    }
}

/// The name a tool call is answered with when the engine's own code panics
/// while it answers. It is a fault of the engine, not a limit of the physics
/// or of the request, so it is no refusal of a card or a solver and carries
/// no seal.
pub const INTERNAL_FAULT: &str = "InternalFault";

/// What a tool call is answered when the engine's own code panicked while
/// answering it: the call's own id is answered, no proof is issued, and the
/// connection goes on serving.
pub fn internal_fault(tool: &str) -> String {
    format!(
        "Internal fault: the engine's own code stopped while answering {tool} ({INTERNAL_FAULT}). No answer and no proof are issued for this call. It is a fault of the engine, not a limit of the physics or of the request, and the connection goes on serving: the next request is answered."
    )
}

/// A test's way to make the engine's own code panic inside a tool call.
#[cfg(test)]
pub(crate) const PANIC_FOR_TEST: &str = "panic_for_test";

/// One tool call, answered whatever happens inside it. The call runs as a
/// task of its own; a panic in it, or in the physics it waits for, ends that
/// task alone, and the call is answered [`internal_fault`]. Every connection
/// calls a tool through here: the stdio door, the hosted door with a key and
/// the open one, and the child that computes the open connection's long work.
pub async fn dispatch_guarded(state: &AppState, name: &str, arguments: Map<String, Value>) -> CallToolResult {
    dispatch_guarded_text(state, name, arguments, None).await
}

/// [`dispatch_guarded`] with the exact text of the arguments as they came,
/// which a verification's long form holds byte for byte to the engine's
/// answer. With none, their JSON text stands for it.
pub async fn dispatch_guarded_text(state: &AppState, name: &str, arguments: Map<String, Value>, text: Option<String>) -> CallToolResult {
    let state = state.clone();
    let tool = name.to_string();
    match tokio::spawn(async move { dispatch_text(&state, &tool, arguments, text).await }).await {
        Ok(result) => result,
        Err(_) => CallToolResult::error(vec![ContentBlock::text(internal_fault(name))]),
    }
}

/// One tool call, as the stdio door and the hosted door both make it. The
/// caller's view is the state's.
pub async fn dispatch(state: &AppState, name: &str, arguments: Map<String, Value>) -> CallToolResult {
    dispatch_text(state, name, arguments, None).await
}

/// [`dispatch`] with the exact text of the arguments as they came. Only the
/// three verifications read it, for their long form.
pub async fn dispatch_text(state: &AppState, name: &str, arguments: Map<String, Value>, text: Option<String>) -> CallToolResult {
    let value = Value::Object(arguments);
    let text = text.as_deref();
    let result = match name {
        "list_cards" => encoded(&descriptor::list_value_in(state.view())),
        "describe_card" => descriptor::describe_request_in(state.view(), &value).map(|body| encoded_value(&body)),
        "compute_step" | EARLIER_COMPUTE_STEP => match crate::compute_step(state, value).await {
            Ok(response) => encoded(&response),
            Err(error) => Err(error),
        },
        "run_trajectory" => match trajectory::run(state, value).await {
            Ok(response) => encoded(&response),
            Err(error) => Err(error),
        },
        "verify_tape" => trajectory::verify(state, &value, text).await,
        "list_solvers" => encoded(&solvers::list_value_in(state.view())),
        "describe_solver" => solvers::describe_request_in(state.view(), &value).map(|body| encoded_value(&body)),
        "solve" => match solution::run(state, value).await {
            Ok(response) => encoded(&response),
            Err(error) => Err(error),
        },
        "verify_solution" => solution::verify(state, &value, text).await,
        "run_dataset" => dataset::run(state, value).await,
        "verify_dataset" => dataset::verify(state, &value, text).await,
        "validate_series" => validation::run(state, value).await,
        "verify_receipt" => match verify_receipt::verify(&value) {
            Ok(report) => encoded(&report),
            Err(error) => Err(error),
        },
        "cite" => crate::cite::cite_in(state.view(), &value).map(|body| encoded_value(&body)),
        "find_model" => crate::find_model::find_in(state.view(), &value).map(|body| encoded_value(&body)),
        "find_flip" => crate::flip::find(state, value).await,
        "design_run" => crate::design::design(state, value).await,
        #[cfg(test)]
        PANIC_FOR_TEST => panic!("a fault injected by a test inside a tool call"),
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
