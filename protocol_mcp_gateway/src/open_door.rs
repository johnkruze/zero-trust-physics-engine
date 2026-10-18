//! The open door: `hosted_door --open`. The same endpoint as the keyed door,
//! with no key. Any caller that has the address is served every card and
//! every solver the engine serves, and in place of the key the door holds
//! limits of its own. Their values are the `OPEN_*` constants of
//! `src/hosted.rs`; six of them can be changed when the door starts.
//!
//! **Order of a request.** The source address is found; its allowance of
//! requests a minute is charged (`429` with `Retry-After`); the method, the
//! origin, the content type and the accepted type are checked; a body longer
//! than the limit by its own `Content-Length` is refused unread (`413`); the
//! requests in flight, in total and from this address, are looked at (`429`);
//! the body is received within its time; the places are taken; the message
//! passes the guards the other doors have; a dataset larger than the open
//! door runs is refused by name; and only then is the tool called, which is
//! where a physics permit is taken, under the address's share of the pool.
//!
//! **What it does not do.** It reads no `Authorization` header and writes no
//! log of any header. It never answers `401` or `WWW-Authenticate`, and it has
//! no OAuth metadata: a client that looks for some gets `404`. It stores no
//! dataset.

use std::collections::HashMap;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use axum::Router;
use axum::body::Body;
use axum::extract::State;
use axum::http::{HeaderName, HeaderValue, Method, Request, StatusCode, header};
use axum::response::Response;
use axum::routing::{any, get};
use genesis_seal::plan::DatasetRequest;
use serde_json::{Map, Value};
use tokio::sync::Semaphore;

use crate::hosted::{
    self, BodyBuffers, Call, Limits, OPEN_BODY_READ_SECONDS, OPEN_MAX_BODY_BYTES,
    OPEN_DATASET_ROW_BYTES, OPEN_MAX_DATASET_ANSWER_BYTES, OPEN_MAX_DATASET_ROWS, OPEN_MAX_DATASET_STEPS,
    OPEN_LIGHT_RUN_STEPS, OPEN_MAX_IN_FLIGHT, OPEN_MAX_IN_FLIGHT_PER_ADDRESS, OPEN_MAX_TRACKED_ADDRESSES,
    OPEN_PERMITS_PER_ADDRESS, OPEN_PHYSICS_PERMITS_AT_LEAST, OPEN_REQUESTS_PER_MINUTE,
    OPEN_REQUEST_SECONDS, PeerAddress,
};
use crate::mcp_server::McpGateway;
use crate::view::View;
use crate::{AppState, dataset, trajectory};

/// The rules one open door runs under.
#[derive(Clone, Debug)]
pub struct OpenRules {
    /// The header the TLS front writes the client's address into. Without
    /// one the address is the socket's.
    pub client_address_header: Option<HeaderName>,
    pub requests_per_minute: u32,
    pub max_in_flight: usize,
    pub max_in_flight_per_address: usize,
    pub physics_permits: usize,
    pub permits_per_address: usize,
    pub max_body_bytes: usize,
    /// The longest the door waits for the answer to one tool call.
    pub request: Duration,
    /// The most steps of a run that is short work.
    pub light_run_steps: u64,
    pub max_dataset_rows: u64,
    pub max_dataset_steps: u64,
    pub max_dataset_answer_bytes: u64,
    pub max_tracked_addresses: usize,
    /// The binary long work is run in, a process for each call, so that it
    /// can be ended when the time for an answer is up (`src/long_work.rs`).
    /// The hosted connection's binary names itself. A router made with none
    /// runs long work in its own process, where it cannot be stopped.
    pub worker: Option<std::path::PathBuf>,
}

impl OpenRules {
    /// The stated defaults, with a physics pool of one permit for each core
    /// and never fewer than `OPEN_PHYSICS_PERMITS_AT_LEAST`.
    pub fn for_this_machine() -> OpenRules {
        OpenRules {
            client_address_header: None,
            requests_per_minute: OPEN_REQUESTS_PER_MINUTE,
            max_in_flight: OPEN_MAX_IN_FLIGHT,
            max_in_flight_per_address: OPEN_MAX_IN_FLIGHT_PER_ADDRESS,
            physics_permits: crate::physics_permit_count().max(OPEN_PHYSICS_PERMITS_AT_LEAST),
            permits_per_address: OPEN_PERMITS_PER_ADDRESS,
            max_body_bytes: OPEN_MAX_BODY_BYTES,
            request: Duration::from_secs(OPEN_REQUEST_SECONDS),
            light_run_steps: OPEN_LIGHT_RUN_STEPS,
            max_dataset_rows: OPEN_MAX_DATASET_ROWS,
            max_dataset_steps: OPEN_MAX_DATASET_STEPS,
            max_dataset_answer_bytes: OPEN_MAX_DATASET_ANSWER_BYTES,
            max_tracked_addresses: OPEN_MAX_TRACKED_ADDRESSES,
            worker: None,
        }
    }

    /// One start-up argument of the open door.
    pub(crate) fn read_flag(&mut self, name: &str, value: &str) -> Result<(), String> {
        let count = |most: u64| -> Result<u64, String> {
            value
                .parse::<u64>()
                .ok()
                .filter(|count| (1..=most).contains(count))
                .ok_or_else(|| format!("{name} needs a whole number from 1 to {most}, not '{value}'."))
        };
        match name {
            "--client-address-header" => {
                let header = HeaderName::from_bytes(value.as_bytes())
                    .map_err(|_| format!("--client-address-header needs the name of an HTTP header, not '{value}'."))?;
                self.client_address_header = Some(header);
            }
            "--requests-per-minute" => self.requests_per_minute = count(1_000_000)? as u32,
            "--max-in-flight" => self.max_in_flight = count(10_000)? as usize,
            "--max-in-flight-per-address" => self.max_in_flight_per_address = count(10_000)? as usize,
            "--physics-permits" => self.physics_permits = count(32)? as usize,
            "--permits-per-address" => self.permits_per_address = count(32)? as usize,
            "--request-seconds" => self.request = Duration::from_secs(count(3600)?),
            _ => return Err(format!("'{name}' is not an argument of this program.")),
        }
        Ok(())
    }

    /// The rules must agree with each other.
    pub(crate) fn check(&self) -> Result<(), String> {
        if self.max_in_flight_per_address > self.max_in_flight {
            return Err(format!(
                "--max-in-flight-per-address is {}, more than the {} of --max-in-flight. One address cannot have more in flight than the door.",
                self.max_in_flight_per_address, self.max_in_flight
            ));
        }
        if self.physics_permits < OPEN_PHYSICS_PERMITS_AT_LEAST {
            return Err(format!(
                "--physics-permits is {}, fewer than {OPEN_PHYSICS_PERMITS_AT_LEAST}. With one permit a dataset job would hold the whole pool and no step could run beside it.",
                self.physics_permits
            ));
        }
        if self.permits_per_address > self.physics_permits {
            return Err(format!(
                "--permits-per-address is {}, more than the {} of --physics-permits. One address cannot hold more permits than the pool has.",
                self.permits_per_address, self.physics_permits
            ));
        }
        Ok(())
    }

    /// Every limit in one paragraph, written from the values in force: what
    /// the open door prints when it starts, for the one who runs it.
    pub fn statement(&self) -> String {
        format!(
            "This is the open connection: no key is asked, and every card and every solver the engine serves is listed. It holds these limits, each before a physics permit is taken, and answers each by name. Requests: at most {rpm} a minute from one address, then HTTP 429 with Retry-After (requests-per-minute); at most {flight} in flight at once and {per} from one address, then HTTP 429 (max-in-flight, max-in-flight-per-address); a body of at most {body} bytes, then HTTP 413 (OPEN_MAX_BODY_BYTES), and a call of a tool other than {wide} at most {narrow} bytes, then HTTP 413 as well (NARROW_LIMIT, as on every connection). A run (run_trajectory, validate_series): at most {run_steps} steps and {run_frames} recorded states, the engine's own limits. A dataset (run_dataset): at most {rows} runs (OPEN_MAX_DATASET_ROWS), {steps} steps in total (OPEN_MAX_DATASET_STEPS) and {frames} recorded states, an answer counted from the plan at no more than {answer} bytes (OPEN_MAX_DATASET_ANSWER_BYTES), and returned inline, where the answer counted from the plan is at most {inline} characters, as on every connection: a plan reaches that first, a grid on one axis at about 140 summary rows and a list sooner, its head repeating each run's request, so a plan of {rows} runs is refused by it; no dataset is stored; a plan of solves holds at most the engine's own limit of solves, and fewer of a slow solver (OPEN_PLAN_SECONDS, below). A verification (verify_tape, verify_solution or verify_dataset, given the request and its seal or the whole answer) computes the request again and holds the limits of the tool that computes it. The physics pool has {pool} permits and one address holds at most {share} at a time (permits-per-address). A solve, a dataset job, a comparison of solves and a run of more than {light} steps (OPEN_LIGHT_RUN_STEPS), each also when a verification computes it again, are long work, and long work together holds at most {long} of the {pool}, so long work never takes the permit a single step or a short run needs. A request that finds no permit, or long work that finds no place, is answered \"Too many concurrent physics tasks.\" and nothing waits: the same request sent later is answered once one is free. An answer is waited for {seconds} seconds (OPEN_REQUEST_SECONDS); after that the call is answered with an error and its result is not returned.{stopped}{plans}",
            rpm = self.requests_per_minute,
            flight = self.max_in_flight,
            per = self.max_in_flight_per_address,
            body = self.max_body_bytes,
            wide = crate::guard::WIDE_TOOLS.join(", "),
            narrow = crate::guard::NARROW_LIMIT,
            inline = dataset::MAX_INLINE_ANSWER_CHARS,
            run_steps = trajectory::MAX_TRAJECTORY_STEPS,
            run_frames = trajectory::MAX_RECORDED_FRAMES,
            rows = self.max_dataset_rows,
            steps = self.max_dataset_steps,
            frames = dataset::MAX_INLINE_FRAMES,
            answer = self.max_dataset_answer_bytes,
            pool = self.physics_permits,
            share = self.permits_per_address,
            light = self.light_run_steps,
            long = self.long_work_places(),
            seconds = self.request.as_secs(),
            stopped = if self.worker.is_some() { " Long work is stopped at that time, and its permit and its place come back: a request that was stopped takes as long when sent again, and a smaller one ends in time." } else { "" },
            plans = crate::open_plan::statement(),
        )
    }

    /// The limits every call meets, in a few sentences, written from the
    /// values in force: what `initialize` and `server/discover` carry in
    /// `instructions` after the verifier's own words. The limits of one tool
    /// are in that tool's description ([`OpenRules::says_of`]), so the whole
    /// of the instructions stays inside what a client shows. Both limits on
    /// a body are said: until 0.14.0 rc2 only the 4 MiB was, and a call of
    /// any of the twelve tools outside `WIDE_TOOLS` is refused past
    /// `NARROW_LIMIT`, 2 MiB (WORDS_014, proposal 1; the red team of rc1, RT3
    /// W5). The room for it was made by saying the other limits in fewer
    /// words, none of them left out: every 429 carries Retry-After, so it is
    /// said once for the three. Since 0.14.1 the first sentence says the
    /// connection asks no key and stores nothing, where it also said it
    /// serves every model and names each limit: list_cards and list_solvers
    /// name every model served, and the sentences after it name each limit
    /// and its answer. With "long work then stops", that made the room for
    /// John Kruze's sentence in the verifier's own words (`identity.rs`), so
    /// the instructions keep their length on every connection.
    pub fn said_at_opening(&self) -> String {
        format!(
            "This open connection asks no key and stores nothing. From one address at most {rpm} requests a minute and {per} in flight, {flight} in flight in all, then HTTP 429 with Retry-After; a body of at most {body} bytes to {wide} and {narrow} to any other tool, then HTTP 413. One address holds at most {share} of the {pool} physics permits: a call that finds none is answered \"Too many concurrent physics tasks.\" at once. An answer is waited for {seconds} seconds{stopped}.",
            rpm = self.requests_per_minute,
            flight = self.max_in_flight,
            per = self.max_in_flight_per_address,
            body = self.max_body_bytes,
            wide = crate::guard::WIDE_TOOLS[..5].join(", ") + " or " + crate::guard::WIDE_TOOLS[5],
            narrow = crate::guard::NARROW_LIMIT,
            pool = self.physics_permits,
            share = self.permits_per_address,
            seconds = self.request.as_secs(),
            stopped = if self.worker.is_some() { "; long work then stops and frees its permit and place" } else { "" },
        )
    }

    /// The open connection's limits on a dataset job, for the descriptions
    /// of `run_dataset` and of `verify_dataset`, which runs the plan again.
    fn dataset_words(&self) -> String {
        format!(
            "a dataset is returned inline and holds at most {} runs (OPEN_MAX_DATASET_ROWS), {} steps in all (OPEN_MAX_DATASET_STEPS), {} frames and an answer counted from the plan at {} bytes (OPEN_MAX_DATASET_ANSWER_BYTES) and {} characters, up to about 140 summary rows; {}.",
            self.max_dataset_rows,
            self.max_dataset_steps,
            dataset::MAX_INLINE_FRAMES,
            self.max_dataset_answer_bytes,
            dataset::MAX_INLINE_ANSWER_CHARS,
            crate::open_plan::said_in_a_description()
        )
    }

    /// What every tool that computes says on the open connection: how many
    /// permits an address holds, what long work is, what a call that finds
    /// no permit is answered, and how long an answer is waited for. It opens
    /// a sentence of its own, or goes on after " On this open connection".
    fn busy_words(&self, opens_a_sentence: bool) -> String {
        format!(
            "{} address holds at most {} at a time, and long work (a solve, a plan of runs or of solves, a run of more than {} steps, or a verification that computes one again) runs {} at a time. A call that finds no permit or no place is answered \"Too many concurrent physics tasks.\" and nothing waits: the same call sent later is answered once one is free. An answer is waited for {} seconds{}.",
            if opens_a_sentence { "One" } else { "one" },
            counted(self.permits_per_address, "physics permit"),
            self.light_run_steps,
            self.long_work_places(),
            self.request.as_secs(),
            if self.worker.is_some() { ", and long work is stopped there" } else { "" }
        )
    }

    /// What one tool's description says on the open connection, after the
    /// engine's own words: this connection's limits for that tool, and what
    /// a call that finds no permit is answered. It begins " On this open
    /// connection". A tool that takes no permit says nothing more.
    pub(crate) fn says_of(&self, tool: &str) -> String {
        let own = match tool {
            "run_dataset" | "verify_dataset" => format!("{} {}", self.dataset_words(), self.busy_words(true)),
            "validate_series" => format!("{}. {}", crate::open_plan::said_in_a_description(), self.busy_words(true)),
            "find_flip" => format!(
                "a search is long work whenever it varies a solve or its runs may take more than {} steps together. {}",
                self.light_run_steps,
                self.busy_words(true)
            ),
            "design_run" => format!(
                "a design is long work whenever its candidates may take more than {} steps together. {}",
                self.light_run_steps,
                self.busy_words(true)
            ),
            "compute_step" | "verify_step" | "run_trajectory" | "solve" | "verify_tape" | "verify_solution" => self.busy_words(false),
            _ => return String::new(),
        };
        format!(" On this open connection {own}")
    }

    /// `initialize` and `server/discover` carry the limits in `instructions`.
    pub(crate) fn state_in_instructions(&self, info: &mut Value) {
        let Some(object) = info.as_object_mut() else { return };
        let before = object.get("instructions").and_then(Value::as_str).unwrap_or("").to_string();
        let joined = if before.is_empty() { self.said_at_opening() } else { format!("{before} {}", self.said_at_opening()) };
        object.insert("instructions".to_string(), Value::String(joined));
    }

    /// The sentence for a tool call the open door does not run, or nothing.
    /// Only a dataset has limits of the open door's own, for `run_dataset`
    /// and for `verify_dataset` given its request: a run keeps the engine's,
    /// which `run_trajectory`, `validate_series` and `verify_tape` enforce.
    pub(crate) fn fault(&self, tool: &str, arguments: &Map<String, Value>, body_bytes: usize) -> Option<String> {
        // A plan of more solves than end inside the time for an answer is
        // refused by name, before any permit.
        if let Some(sentence) = crate::open_plan::fault(tool, arguments) {
            return Some(sentence);
        }
        // A check by request of a dataset runs the plan again, and is held to
        // the limits a dataset job is held to; so does its long form, which
        // runs its head's plan again. The answer the long form runs again is
        // the one it was sent, so it is not counted again.
        let (arguments, counts_the_answer) = match (tool, arguments.get(crate::verify_by_request::REQUEST)) {
            ("run_dataset", _) => (arguments, true),
            ("verify_dataset", Some(Value::Object(request))) => (request, true),
            ("verify_dataset", None) => match arguments.get("head").and_then(|head| head.get("request")) {
                Some(Value::Object(request)) => (request, false),
                _ => return None,
            },
            _ => return None,
        };
        // A plan that cannot be read is refused by the engine in its own words.
        // The answer form is read out and an axis's base value replaced first,
        // as the engine does. A plan of solves is held to the engine's own
        // limit of solves.
        let normalized = dataset::normalize(Value::Object(arguments.clone())).ok()?;
        if normalized.is_solves() {
            return None;
        }
        let request = DatasetRequest::from_value(normalized.request).ok()?;
        let totals = request.totals();
        if totals.rows > self.max_dataset_rows {
            return Some(format!(
                "Physical Violation: the dataset asks for {} runs, more than the {} one job may have on the open connection (OPEN_MAX_DATASET_ROWS). The engine's own limit is {}; this connection is open to anyone and runs smaller jobs.",
                totals.rows, self.max_dataset_rows, dataset::MAX_DATASET_ROWS
            ));
        }
        if totals.steps > self.max_dataset_steps {
            return Some(format!(
                "Physical Violation: the dataset asks for {} steps in total, more than the {} one job may have on the open connection (OPEN_MAX_DATASET_STEPS). The engine's own limit is {}; this connection is open to anyone and runs smaller jobs.",
                totals.steps, self.max_dataset_steps, dataset::MAX_DATASET_STEPS
            ));
        }
        if !counts_the_answer {
            return None;
        }
        // The answer, counted from the plan: the head repeats the request,
        // each row carries its own request and OPEN_DATASET_ROW_BYTES more,
        // and each recorded state is 128 characters. A list's rows are
        // in the request already; a grid or a sample repeats its base run in
        // every row.
        let body = body_bytes as u64;
        let list = arguments.get("plan").and_then(|plan| plan.get("kind")).and_then(Value::as_str) == Some("list");
        let rows_text = if list { body } else { totals.rows.saturating_mul(body) };
        let counted = body
            .saturating_add(rows_text)
            .saturating_add(totals.rows.saturating_mul(OPEN_DATASET_ROW_BYTES))
            .saturating_add(totals.frames.min(dataset::MAX_INLINE_FRAMES).saturating_mul(128));
        if counted > self.max_dataset_answer_bytes {
            return Some(format!(
                "Physical Violation: the dataset's answer would come to about {counted} bytes, more than the {} one answer may have on the open connection (OPEN_MAX_DATASET_ANSWER_BYTES). It is counted from the plan: the request once, each run's own request, {} bytes for each run, and 128 for each recorded state. Fewer runs, a shorter base run or a larger record_every makes it smaller.",
                self.max_dataset_answer_bytes, OPEN_DATASET_ROW_BYTES
            ));
        }
        None
    }

    /// How much long work runs at once: half the physics pool, the share the
    /// engine gives dataset jobs.
    pub fn long_work_places(&self) -> usize {
        (self.physics_permits / 2).max(1)
    }

    /// Whether a call is long work: a solve, a dataset job, a comparison of
    /// solves, and a run or a
    /// validation that asks for more steps than a short run may, each also
    /// when a check by request computes it again. A request
    /// whose steps cannot be read is not long work: the engine refuses it
    /// before it computes.
    pub(crate) fn is_long_work(&self, tool: &str, arguments: &Map<String, Value>) -> bool {
        // A run's steps, and with `from` the steps of every upstream run the
        // engine computes again.
        let steps_of = |run: Option<&Value>| -> u64 { run.map(crate::chain::chained_steps).unwrap_or(0) };
        let by_request = arguments.get(crate::verify_by_request::REQUEST);
        match tool {
            "solve" | "run_dataset" => true,
            // A search of solves, and a search whose runs may take more steps
            // together than a short run may.
            "find_flip" => {
                arguments.get("request").is_some_and(|request| request.get("solver").is_some())
                    || crate::flip::steps_at_most(arguments) > self.light_run_steps
            }
            // A design whose candidates may take more steps together than a short run may.
            "design_run" => crate::design::steps_at_most(arguments) > self.light_run_steps,
            "run_trajectory" => steps_of(Some(&Value::Object(arguments.clone()))) > self.light_run_steps,
            // A step is one step; with `from` its upstream runs are computed again.
            "compute_step" | "verify_step" => steps_of(Some(&Value::Object(arguments.clone()))) > self.light_run_steps,
            "validate_series" => arguments.contains_key("solves") || steps_of(arguments.get("run")) > self.light_run_steps,
            // A check by request computes its request again: a solve and a
            // dataset job are long work, and a run by its steps. So does every
            // long form, which computes the request its answer carries again:
            // a solve, a dataset's plan, a run and a chain's links by their
            // steps, a design by its candidates'.
            "verify_dataset" => true,
            "verify_solution" => by_request.is_some() || crate::solution::long_form_solves(arguments),
            "verify_tape" => match by_request {
                Some(_) => steps_of(by_request) > self.light_run_steps,
                None => crate::trajectory::long_form_steps(arguments).is_some_and(|steps| steps > self.light_run_steps),
            },
            _ => false,
        }
    }

    /// What a call is answered when its answer did not come in time and its
    /// work was stopped: long work, run in a process of its own.
    pub(crate) fn stopped_at_the_limit(&self) -> String {
        format!(
            "Physical Violation: the answer did not come within {} seconds, the longest the open connection waits for one call (OPEN_REQUEST_SECONDS). The work was stopped there: nothing is returned for this call and no proof was issued. The same request takes as long again and is stopped again, so do not send it as it is. Send a smaller one: fewer steps, fewer rows, a shorter history, or a value nearer the model's worked example.",
            self.request.as_secs()
        )
    }

    /// What a call is answered when its answer did not come in time and its
    /// work runs on: short work, which no limit of size lets run long.
    pub(crate) fn too_slow(&self) -> String {
        format!(
            "Physical Violation: the answer did not come within {} seconds, the longest the open connection waits for one call (OPEN_REQUEST_SECONDS). Nothing is returned for this call and no proof was issued. The request is short work that the machine did not reach in time; it holds its permit until it ends, so wait before sending another.",
            self.request.as_secs()
        )
    }
}

fn counted(count: usize, what: &str) -> String {
    if count == 1 { format!("1 {what}") } else { format!("{count} {what}s") }
}

/// The limits of a connection to the open door: the keyed door's, with less
/// time for a body.
pub const OPEN_CONNECTION_LIMITS: Limits = Limits {
    body_read: Duration::from_secs(OPEN_BODY_READ_SECONDS),
    ..Limits::DEFAULT
};

/// What the door keeps for one source address.
struct Source {
    /// Requests the address may still make now.
    allowance: f64,
    /// When the allowance was last brought up to date.
    counted: Instant,
    in_flight: Arc<Semaphore>,
    receiving: Arc<Semaphore>,
    /// The address's share of the physics pool.
    share: Arc<Semaphore>,
}

/// What one admitted request holds of its address.
pub(crate) struct Admitted {
    in_flight: Arc<Semaphore>,
    receiving: Arc<Semaphore>,
    share: Arc<Semaphore>,
}

struct Sources {
    rules: OpenRules,
    known: Mutex<HashMap<IpAddr, Source>>,
}

/// The address a newcomer is counted under when the door can keep no more
/// addresses and every one it keeps has a request in flight. No caller has it.
const CROWD: IpAddr = IpAddr::V4(Ipv4Addr::UNSPECIFIED);

/// The address a count is kept under. An IPv6 caller is counted by its first
/// 64 bits, the network one subscriber is given, so that the addresses inside
/// one network are one caller.
pub fn counted_address(address: IpAddr) -> IpAddr {
    match address {
        IpAddr::V4(_) => address,
        IpAddr::V6(v6) => match v6.to_ipv4_mapped() {
            Some(v4) => IpAddr::V4(v4),
            None => {
                let s = v6.segments();
                IpAddr::V6(Ipv6Addr::new(s[0], s[1], s[2], s[3], 0, 0, 0, 0))
            }
        },
    }
}

impl Sources {
    fn new(rules: OpenRules) -> Sources {
        Sources { rules, known: Mutex::new(HashMap::new()) }
    }

    fn fresh(&self, now: Instant) -> Source {
        Source {
            allowance: f64::from(self.rules.requests_per_minute),
            counted: now,
            in_flight: Arc::new(Semaphore::new(self.rules.max_in_flight_per_address)),
            receiving: Arc::new(Semaphore::new(self.rules.max_in_flight_per_address)),
            share: Arc::new(Semaphore::new(self.rules.permits_per_address)),
        }
    }

    /// Charge one request to the address. The answer is what the request
    /// holds of the address, or the seconds until the address may ask again.
    fn admit(&self, address: IpAddr, now: Instant) -> Result<Admitted, u64> {
        let address = counted_address(address);
        let per_minute = f64::from(self.rules.requests_per_minute);
        let mut known = self.known.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        let mut key = address;
        let per_address = self.rules.max_in_flight_per_address;
        if !known.contains_key(&key) {
            if known.len() >= self.rules.max_tracked_addresses {
                // Forget the addresses that are as good as new: nothing in
                // flight and a minute or more since their last request.
                known.retain(|kept, source| {
                    *kept == CROWD
                        || source.in_flight.available_permits() < per_address
                        || source.receiving.available_permits() < per_address
                        || now.duration_since(source.counted) < Duration::from_secs(60)
                });
            }
            if known.len() >= self.rules.max_tracked_addresses {
                // None is quiet: the one heard from longest ago, with nothing
                // in flight, gives up its place. It is counted afresh if it
                // comes back. So an address the door has not seen is never
                // turned away because others filled the count, and a flood
                // from more addresses than the door counts is not held by
                // this allowance: the limits on requests in flight hold it.
                let oldest = known
                    .iter()
                    .filter(|(kept, source)| {
                        **kept != CROWD
                            && source.in_flight.available_permits() == per_address
                            && source.receiving.available_permits() == per_address
                    })
                    .min_by_key(|(_, source)| source.counted)
                    .map(|(kept, _)| *kept);
                match oldest {
                    Some(oldest) => {
                        known.remove(&oldest);
                    }
                    None => key = CROWD,
                }
            }
        }
        let fresh = self.fresh(now);
        let source = known.entry(key).or_insert(fresh);
        let gained = now.duration_since(source.counted).as_secs_f64() * per_minute / 60.0;
        source.allowance = (source.allowance + gained).min(per_minute);
        source.counted = now;
        if source.allowance < 1.0 {
            let wait = (1.0 - source.allowance) * 60.0 / per_minute;
            return Err((wait.ceil() as u64).max(1));
        }
        source.allowance -= 1.0;
        Ok(Admitted {
            in_flight: Arc::clone(&source.in_flight),
            receiving: Arc::clone(&source.receiving),
            share: Arc::clone(&source.share),
        })
    }

    #[cfg(test)]
    fn tracked(&self) -> usize {
        self.known.lock().unwrap().len()
    }
}

/// Bodies on their way to the door at once, for each request it may have in
/// flight. A body that is arriving holds what has arrived of it and nothing
/// else, so more of them are let in than requests are run: a few addresses
/// that send their bodies slowly do not fill the door for the others.
const BODIES_ARRIVING_FOR_EACH_IN_FLIGHT: usize = 4;

struct OpenDoor {
    rules: OpenRules,
    sources: Sources,
    state: AppState,
    gateway: McpGateway,
    allowed_origins: Vec<String>,
    body_read: Duration,
    bodies: Arc<BodyBuffers>,
    in_flight: Arc<Semaphore>,
    receiving: Arc<Semaphore>,
    long_work: Arc<Semaphore>,
}

/// The open router: every card and solver, no key, the rules given.
pub fn router(rules: OpenRules, allowed_origins: Vec<String>) -> Result<Router, String> {
    router_limited(rules, allowed_origins, OPEN_CONNECTION_LIMITS)
}

/// The same with the limits of the connection chosen, so a test need not
/// wait the time the door waits.
#[doc(hidden)]
pub fn router_limited(rules: OpenRules, allowed_origins: Vec<String>, limits: Limits) -> Result<Router, String> {
    rules.check()?;
    let state = AppState::new(rules.physics_permits).with_view(Arc::new(View::full()));
    router_over(state, rules, allowed_origins, limits)
}

/// The same over a given state, so a test can hold the permit pools.
pub(crate) fn router_over(state: AppState, rules: OpenRules, allowed_origins: Vec<String>, limits: Limits) -> Result<Router, String> {
    let door = Arc::new(OpenDoor {
        sources: Sources::new(rules.clone()),
        state,
        gateway: McpGateway::new(),
        allowed_origins: allowed_origins.into_iter().map(|origin| origin.to_ascii_lowercase()).collect(),
        body_read: limits.body_read,
        bodies: Arc::new(BodyBuffers::default()),
        in_flight: Arc::new(Semaphore::new(rules.max_in_flight)),
        long_work: Arc::new(Semaphore::new(rules.long_work_places())),
        receiving: Arc::new(Semaphore::new(rules.max_in_flight * BODIES_ARRIVING_FOR_EACH_IN_FLIGHT)),
        rules,
    });
    // Any other path is the router's plain 404 with no body, the OAuth
    // metadata paths among them: this door has no authorization to discover.
    Ok(Router::new()
        .route("/health", get(|| async { StatusCode::OK }))
        .route("/mcp", any(mcp))
        .layer(axum::middleware::from_fn(hosted::refuse_both_lengths))
        .with_state(door))
}

/// What the open door prints when it starts: three lines, the address and
/// that it is open, the limits in force, and where an address is read from.
pub fn banner(bound: SocketAddr, rules: &OpenRules) -> String {
    let view = View::full();
    let address = match &rules.client_address_header {
        Some(name) => format!(
            "A caller's address is read from the '{name}' header, which the front in front of this door must overwrite on every request; a request without it is counted under the address of the socket."
        ),
        None => "A caller's address is the address of the socket. Behind a front that is the front's own address, and every caller is then counted as one: name the front's header with --client-address-header.".to_string(),
    };
    format!(
        "Hosted door running on {bound} OPEN: no key is asked. Anyone who has the address reaches every card and every solver the engine serves ({}, {}).\n{}\n{address}",
        counted(view.cards().len(), "card"),
        counted(view.solver_names().len(), "solver"),
        rules.statement()
    )
}

pub(crate) async fn serve(port: u16, mut rules: OpenRules, allowed_origins: Vec<String>) -> Result<(), String> {
    // Long work runs in a process of its own, started from this binary.
    rules.worker = crate::long_work::worker();
    if rules.worker.is_none() {
        return Err("it cannot find its own binary to run long work in, so long work could not be stopped at the time limit.".to_string());
    }
    let app = router(rules.clone(), allowed_origins)?;
    let address = SocketAddr::from(([127, 0, 0, 1], port));
    let listener = tokio::net::TcpListener::bind(address)
        .await
        .map_err(|error| format!("it cannot listen on {address}: {error}."))?;
    let bound = listener.local_addr().map_err(|error| error.to_string())?;
    hosted::announce(&banner(bound, &rules));
    hosted::serve_listener(listener, app, OPEN_CONNECTION_LIMITS).await;
    Ok(())
}

fn too_many(text: String, retry_after: u64) -> Response {
    let mut response = hosted::plain(StatusCode::TOO_MANY_REQUESTS, &text);
    response.headers_mut().insert(
        header::RETRY_AFTER,
        HeaderValue::from_str(&retry_after.to_string()).expect("digits are a header value"),
    );
    response
}

/// The caller's address: the one value of the front's header when the door
/// was started with its name, and the socket's address without the header.
/// A header that is there and is not one address is refused: it means the
/// front appends to what the caller sent, and the caller could then write
/// any address it liked.
fn source_address(door: &OpenDoor, request: &Request<Body>) -> Result<IpAddr, Response> {
    let peer = request
        .extensions()
        .get::<PeerAddress>()
        .map(|peer| peer.0)
        .unwrap_or(IpAddr::V4(Ipv4Addr::LOCALHOST));
    let Some(name) = &door.rules.client_address_header else {
        return Ok(peer);
    };
    let mut values = request.headers().get_all(name).iter();
    let Some(first) = values.next() else {
        return Ok(peer);
    };
    let one = values.next().is_none();
    match first.to_str().ok().map(str::trim).and_then(|text| text.parse::<IpAddr>().ok()) {
        Some(address) if one => Ok(address),
        _ => Err(hosted::plain(
            StatusCode::BAD_REQUEST,
            &format!("Bad Request: the '{name}' header must hold one address, written by the front that serves this connection (client-address-header)."),
        )),
    }
}

async fn mcp(State(door): State<Arc<OpenDoor>>, request: Request<Body>) -> Response {
    let origin = request.headers().get(header::ORIGIN).cloned();
    let mut response = serve_one(&door, request).await;
    // A browser page may read the answer: the door has no key and no cookie,
    // so a page learns nothing a direct caller would not.
    if let Some(origin) = origin {
        let allowed = if door.allowed_origins.is_empty() { HeaderValue::from_static("*") } else { origin };
        let headers = response.headers_mut();
        headers.insert(header::ACCESS_CONTROL_ALLOW_ORIGIN, allowed);
        headers.insert(header::VARY, HeaderValue::from_static("Origin"));
    }
    response
}

async fn serve_one(door: &Arc<OpenDoor>, request: Request<Body>) -> Response {
    let rules = &door.rules;
    let address = match source_address(door, &request) {
        Ok(address) => address,
        Err(response) => return response,
    };
    let admitted = match door.sources.admit(address, Instant::now()) {
        Ok(admitted) => admitted,
        Err(seconds) => {
            return too_many(
                format!(
                    "Too many requests from this address: the open connection takes at most {} a minute from one address (requests-per-minute). Send the next one in {seconds} seconds.",
                    rules.requests_per_minute
                ),
                seconds,
            );
        }
    };
    let (parts, body) = request.into_parts();

    // The Authorization header is not read: this door has no key.
    if let Some(origin) = parts.headers.get(header::ORIGIN) {
        // With no origin named at start every origin is served. With some
        // named, only those are.
        let allowed = door.allowed_origins.is_empty()
            || origin
                .to_str()
                .map(|text| door.allowed_origins.contains(&text.to_ascii_lowercase()))
                .unwrap_or(false);
        if !allowed {
            return hosted::plain(StatusCode::FORBIDDEN, "Forbidden: this origin is not allowed.");
        }
    }
    if parts.method == Method::OPTIONS {
        // A browser asks before it posts.
        let mut response = Response::new(Body::empty());
        *response.status_mut() = StatusCode::NO_CONTENT;
        let headers = response.headers_mut();
        headers.insert(header::ALLOW, HeaderValue::from_static("POST, OPTIONS"));
        headers.insert(header::ACCESS_CONTROL_ALLOW_METHODS, HeaderValue::from_static("POST, OPTIONS"));
        headers.insert(
            header::ACCESS_CONTROL_ALLOW_HEADERS,
            HeaderValue::from_static("content-type, accept, authorization, mcp-protocol-version, mcp-session-id, mcp-method, mcp-name, last-event-id"),
        );
        headers.insert(header::ACCESS_CONTROL_MAX_AGE, HeaderValue::from_static("86400"));
        return response;
    }
    if parts.method != Method::POST {
        let mut response = hosted::plain(StatusCode::METHOD_NOT_ALLOWED, "Method Not Allowed");
        response.headers_mut().insert(header::ALLOW, HeaderValue::from_static("POST, OPTIONS"));
        return response;
    }
    if !hosted::is_json(&parts.headers) {
        return hosted::plain(
            StatusCode::UNSUPPORTED_MEDIA_TYPE,
            "Expected request with `Content-Type: application/json`",
        );
    }
    if !hosted::accepts_json(&parts.headers) {
        return hosted::plain(
            StatusCode::NOT_ACCEPTABLE,
            "Not Acceptable: the client must accept application/json.",
        );
    }
    let named = hosted::Named::of(&parts.headers);
    // A body that says it is longer than the limit is not read at all.
    let stated = parts
        .headers
        .get(header::CONTENT_LENGTH)
        .and_then(|value| value.to_str().ok())
        .and_then(|text| text.parse::<u64>().ok());
    if stated.is_some_and(|length| length > rules.max_body_bytes as u64) {
        let mut response = too_long(rules, &[]);
        response.headers_mut().insert(header::CONNECTION, HeaderValue::from_static("close"));
        return response;
    }
    let in_flight_total = || {
        too_many(
            format!(
                "Too many requests in flight: the open connection runs at most {} at once (max-in-flight). Send this one again.",
                rules.max_in_flight
            ),
            1,
        )
    };
    let in_flight_address = || {
        too_many(
            format!(
                "Too many requests in flight from this address: the open connection runs at most {} at once from one address (max-in-flight-per-address). Send this one again when one has been answered.",
                rules.max_in_flight_per_address
            ),
            1,
        )
    };
    if admitted.in_flight.available_permits() == 0 {
        return in_flight_address();
    }
    if door.in_flight.available_permits() == 0 {
        return in_flight_total();
    }
    // A body being received holds a place for a body, of the address and of
    // the door, and none of the places of a running request.
    let Ok(receiving_address) = Arc::clone(&admitted.receiving).try_acquire_owned() else {
        return in_flight_address();
    };
    let Ok(receiving_total) = Arc::clone(&door.receiving).try_acquire_owned() else {
        return too_many(
            format!(
                "Too many request bodies on their way: the open connection receives at most {} at once ({BODIES_ARRIVING_FOR_EACH_IN_FLIGHT} for each of max-in-flight). Send this one again.",
                rules.max_in_flight * BODIES_ARRIVING_FOR_EACH_IN_FLIGHT
            ),
            1,
        );
    };
    let mut bytes = door.bodies.take();
    let received = tokio::time::timeout(
        door.body_read,
        hosted::read_capped_at(body, &mut bytes.bytes, rules.max_body_bytes),
    )
    .await;
    drop(receiving_total);
    drop(receiving_address);
    let Ok(whole) = received else {
        return hosted::body_too_slow();
    };
    if !whole {
        return too_long(rules, &bytes);
    }
    let Ok(place_address) = Arc::clone(&admitted.in_flight).try_acquire_owned() else {
        return in_flight_address();
    };
    let Ok(place_total) = Arc::clone(&door.in_flight).try_acquire_owned() else {
        return in_flight_total();
    };
    let call = Call {
        gateway: &door.gateway,
        state: door.state.clone().with_share(Arc::clone(&admitted.share)),
        slot: Box::new((place_address, place_total)),
        open: Some(rules),
        long_work: Some(Arc::clone(&door.long_work)),
        named,
    };
    hosted::answer(call, bytes).await
}

/// A body over the open door's limit: `413` and a JSON-RPC error, under the
/// caller's id when the part that was read shows a usable one.
fn too_long(rules: &OpenRules, kept: &[u8]) -> Response {
    let text = format!(
        "Request too large: a request is at most {} bytes on the open connection (OPEN_MAX_BODY_BYTES). This one is longer and was not read.",
        rules.max_body_bytes
    );
    let id = if kept.is_empty() { "null".to_string() } else { crate::guard::answer_id(kept, false) };
    hosted::rpc_http_response(crate::guard::invalid_request(&id, &text))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn rules() -> OpenRules {
        OpenRules {
            client_address_header: Some(HeaderName::from_static("x-client")),
            physics_permits: 2,
            ..OpenRules::for_this_machine()
        }
    }

    async fn serve_in_process(state: AppState, rules: OpenRules) -> String {
        let app = router_over(state, rules, Vec::new(), OPEN_CONNECTION_LIMITS).expect("router");
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
        format!("http://127.0.0.1:{port}/mcp")
    }

    async fn post(url: &str, address: &str, body: String) -> (u16, String) {
        let response = reqwest::Client::new()
            .post(url)
            .header("Content-Type", "application/json")
            .header("x-client", address)
            .body(body)
            .send()
            .await
            .unwrap();
        (response.status().as_u16(), response.text().await.unwrap())
    }

    fn call(tool: &str, arguments: Value) -> String {
        json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":tool,"arguments":arguments}}).to_string()
    }

    fn step() -> String {
        call("verify_step", json!({"domain":"energy","card":"thermal","parameters":{"temperature_k":293.15,"heat_power_w":1.0,"ambient_k":293.15,"dt_s":0.1}}))
    }

    fn run(temperature_k: f64) -> String {
        call("run_trajectory", json!({"domain":"energy","card":"thermal","initial":{"temperature_k":temperature_k},"dt_s":0.1,
            "segments":[{"steps":3,"load":{"heat_power_w":1.0,"ambient_k":293.15}}],"record_every":3}))
    }

    fn dataset(temperature_k: f64) -> String {
        call("run_dataset", json!({"domain":"energy","card":"thermal","plan":{"kind":"list","runs":[
            {"initial":{"temperature_k":temperature_k},"dt_s":0.1,"segments":[{"steps":3,"load":{"heat_power_w":1.0,"ambient_k":293.15}}],"record_every":3}
        ]},"store":"none","return":"inline"}))
    }

    const BUSY: &str = "Too many concurrent physics tasks.";

    /// On a pool of two, as on one core: one address holds one permit and no
    /// more, so it cannot fill the pool by itself; a second address still runs
    /// while the first computes; with both permits held a third is busy; and
    /// every permit comes back.
    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn one_address_holds_one_permit_and_another_address_still_runs() {
        let state = AppState::with_hold(2, Duration::from_millis(700));
        let pool = Arc::clone(&state.physics_permits);
        let url = serve_in_process(state, rules()).await;

        let first = tokio::spawn({ let url = url.clone(); async move { post(&url, "10.0.0.1", run(293.15)).await } });
        tokio::time::sleep(Duration::from_millis(200)).await;
        assert_eq!(pool.available_permits(), 1, "the first run holds one permit");
        // The same address is at its share: busy in the words of a full pool, and nothing is taken.
        let (status, body) = post(&url, "10.0.0.1", step()).await;
        assert_eq!(status, 200);
        assert!(body.contains(BUSY) && body.contains("\"isError\":true"), "{body}");
        assert_eq!(pool.available_permits(), 1);
        // Another address takes the other permit ...
        let second = tokio::spawn({ let url = url.clone(); async move { post(&url, "10.0.0.2", run(294.15)).await } });
        tokio::time::sleep(Duration::from_millis(200)).await;
        assert_eq!(pool.available_permits(), 0);
        // ... and with the pool full a third address is busy too.
        let (_, body) = post(&url, "10.0.0.3", step()).await;
        assert!(body.contains(BUSY), "{body}");
        for joined in [first.await.unwrap(), second.await.unwrap()] {
            assert!(joined.1.contains("run_seal"), "{}", joined.1);
        }
        assert_eq!(pool.available_permits(), 2);
        let (_, body) = post(&url, "10.0.0.1", step()).await;
        assert!(body.contains("sha256_seal"), "{body}");
    }

    /// An address given two permits at start computes two calls at once and
    /// no more: its third is busy in the words of a full pool, and another
    /// address still finds the permit that is left.
    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn an_address_given_two_permits_computes_two_at_once_and_no_more() {
        let state = AppState::with_hold(3, Duration::from_millis(900));
        let pool = Arc::clone(&state.physics_permits);
        let mut two = OpenRules { physics_permits: 3, ..rules() };
        two.read_flag("--permits-per-address", "2").unwrap();
        assert_eq!((two.permits_per_address, two.check()), (2, Ok(())));
        let url = serve_in_process(state, two).await;

        let first = tokio::spawn({ let url = url.clone(); async move { post(&url, "10.0.0.1", run(293.15)).await } });
        let second = tokio::spawn({ let url = url.clone(); async move { post(&url, "10.0.0.1", run(294.15)).await } });
        tokio::time::sleep(Duration::from_millis(300)).await;
        assert_eq!(pool.available_permits(), 1, "two runs of one address hold two permits");
        let (status, body) = post(&url, "10.0.0.1", step()).await;
        assert_eq!(status, 200);
        assert!(body.contains(BUSY) && body.contains("\"isError\":true"), "{body}");
        assert_eq!(pool.available_permits(), 1, "a call past the share takes nothing from the pool");
        let (_, body) = post(&url, "10.0.0.2", step()).await;
        assert!(body.contains("sha256_seal"), "{body}");
        for joined in [first.await.unwrap(), second.await.unwrap()] {
            assert!(joined.1.contains("run_seal"), "{}", joined.1);
        }
        assert_eq!(pool.available_permits(), 3);
        // More permits for an address than the pool has is refused at start, by name.
        let over = OpenRules { physics_permits: 2, permits_per_address: 3, ..rules() };
        assert_eq!(over.check(), Err("--permits-per-address is 3, more than the 2 of --physics-permits. One address cannot hold more permits than the pool has.".to_string()));
    }

    /// The reason the pool has two permits on one core: a dataset job holds
    /// one, a second dataset job from anyone is busy because dataset jobs
    /// together hold half the pool, and a single step from another address
    /// runs beside the job on the permit no dataset job may take.
    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn a_dataset_job_never_takes_the_permit_a_single_step_needs() {
        let state = AppState::with_hold(2, Duration::from_millis(900));
        let pool = Arc::clone(&state.physics_permits);
        let datasets = Arc::clone(&state.dataset_permits);
        assert_eq!(datasets.available_permits(), 1);
        let url = serve_in_process(state, rules()).await;

        let job = tokio::spawn({ let url = url.clone(); async move { post(&url, "10.0.0.1", dataset(293.15)).await } });
        tokio::time::sleep(Duration::from_millis(250)).await;
        assert_eq!((pool.available_permits(), datasets.available_permits()), (1, 0));
        let (_, second_job) = post(&url, "10.0.0.2", dataset(294.15)).await;
        assert!(second_job.contains(BUSY), "{second_job}");
        assert_eq!(pool.available_permits(), 1, "a refused job keeps no physics permit");
        let started = Instant::now();
        let stepped = tokio::spawn({ let url = url.clone(); async move { post(&url, "10.0.0.2", step()).await } });
        tokio::time::sleep(Duration::from_millis(250)).await;
        assert_eq!(pool.available_permits(), 0, "the step runs on the other permit while the job runs");
        let (_, body) = stepped.await.unwrap();
        assert!(body.contains("sha256_seal"), "{body}");
        assert!(started.elapsed() < Duration::from_secs(5));
        let (_, body) = job.await.unwrap();
        assert!(body.contains("dataset_seal"), "{body}");
        assert_eq!((pool.available_permits(), datasets.available_permits()), (2, 1));
    }

    /// Long work (a solve, a dataset job, a run of more steps than a short
    /// run may ask) holds one permit of the two at most, whoever asks, so a
    /// flood of it from many addresses leaves the other permit to steps and
    /// short runs.
    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn long_work_leaves_a_permit_for_a_step_and_a_short_run() {
        let state = AppState::with_hold(2, Duration::from_millis(900));
        let pool = Arc::clone(&state.physics_permits);
        let rules = rules();
        assert_eq!((rules.long_work_places(), rules.light_run_steps), (1, 10_000));
        let url = serve_in_process(state, rules).await;
        let long_run = |steps: u64| {
            call("run_trajectory", json!({"domain":"energy","card":"thermal","initial":{"temperature_k":293.15},"dt_s":0.1,
                "segments":[{"steps":steps - 1,"load":{"heat_power_w":1.0,"ambient_k":293.15}},{"steps":1,"load":{"heat_power_w":1.0,"ambient_k":293.15}}],"record_every":steps}))
        };
        let solve = call("solve", json!({"solver":"atom_dft","parameters":{"atomic_number":1}}));
        let first = tokio::spawn({ let url = url.clone(); let body = long_run(10_001); async move { post(&url, "10.0.0.1", body).await } });
        tokio::time::sleep(Duration::from_millis(250)).await;
        assert_eq!(pool.available_permits(), 1);
        // More long work, from other addresses, finds no place and takes no permit.
        for (address, body) in [("10.0.0.2", long_run(10_001)), ("10.0.0.3", solve.clone()), ("10.0.0.4", dataset(293.15)),
            ("10.0.0.5", json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"validate_series","arguments":{
                "run": serde_json::from_str::<Value>(&long_run(10_001)).unwrap()["params"]["arguments"], "quantity":"temperature_k","measurements":[{"step":1,"value":293.2}]}}}).to_string())] {
            let (status, answer) = post(&url, address, body).await;
            assert_eq!(status, 200);
            assert!(answer.contains(BUSY) && answer.contains("\"isError\":true"), "{address}: {answer}");
            assert_eq!(pool.available_permits(), 1);
        }
        // A step and a run of exactly the short limit still run, on the other permit.
        let (_, stepped) = post(&url, "10.0.0.6", step()).await;
        assert!(stepped.contains("sha256_seal"), "{stepped}");
        let (_, short) = post(&url, "10.0.0.7", long_run(10_000)).await;
        assert!(short.contains("run_seal"), "{short}");
        let (_, done) = first.await.unwrap();
        assert!(done.contains("run_seal"), "{done}");
        assert_eq!(pool.available_permits(), 2);
        // With the place free again the long work is taken.
        let (_, solved) = post(&url, "10.0.0.3", solve).await;
        assert!(solved.contains("solution_seal"), "{solved}");
    }

    /// A chained run is long work by the steps of every run the engine
    /// computes again, its upstream runs included; so is a chained step and
    /// a check by request of a chained run.
    #[test]
    fn a_chained_request_counts_the_steps_of_its_upstream_runs() {
        let rules = rules();
        let run = |steps: u64| json!({"domain":"energy","card":"thermal","initial":{"temperature_k":293.15},"dt_s":0.1,
            "segments":[{"steps":steps,"load":{"heat_power_w":1.0,"ambient_k":293.15}}],"record_every":steps});
        let from = |upstream: Value| json!({"request": upstream, "run_seal": "ab".repeat(32)});
        let object = |value: Value| value.as_object().unwrap().clone();
        assert!(!rules.is_long_work("run_trajectory", &object(run(6_000))));
        let mut chained = run(6_000);
        chained["from"] = from(run(6_000));
        assert!(rules.is_long_work("run_trajectory", &object(chained.clone())));
        let mut step = json!({"domain":"energy","card":"thermal","parameters":{"temperature_k":293.15,"heat_power_w":1.0,"ambient_k":293.15,"dt_s":0.1}});
        assert!(!rules.is_long_work("verify_step", &object(step.clone())));
        step["from"] = from(run(10_001));
        assert!(rules.is_long_work("verify_step", &object(step)));
        assert!(rules.is_long_work("verify_tape", &object(json!({"request": chained, "run_seal": "ab".repeat(32)}))));
    }

    /// Every long form computes the request its answer carries again, and is
    /// counted as that computation is: a solve and a dataset's plan are long
    /// work, a run by its steps, a chain's links by the steps each link
    /// computes again with the links before it; a chained answer sent alone
    /// is refused before any physics and is not.
    #[test]
    fn every_long_form_is_counted_as_the_computation_it_makes() {
        let rules = rules();
        let object = |value: Value| value.as_object().unwrap().clone();
        let run = |steps: u64| json!({"domain":"energy","card":"thermal","initial":{"temperature_k":293.15},"dt_s":0.1,
            "segments":[{"steps":steps,"load":{"heat_power_w":1.0,"ambient_k":293.15}}],"record_every":steps});
        let answer = |steps: u64| json!({"inputs": run(steps), "outcome": "completed", "frames": [], "run_seal": "ab".repeat(32)});
        assert!(rules.is_long_work("verify_solution", &object(json!({"inputs": {"solver": "atom_dft", "parameters": {"atomic_number": 1}}, "outcome": "converged"}))));
        assert!(rules.is_long_work("verify_dataset", &object(json!({"head": {"request": {}}, "rows": [], "frames_hex": null}))));
        assert!(!rules.is_long_work("verify_tape", &object(answer(10_000))));
        assert!(rules.is_long_work("verify_tape", &object(answer(10_001))));
        // Two links of 4,000 steps: the second computes the first again, 12,000 in all.
        assert!(!rules.is_long_work("verify_tape", &object(json!({"links": [answer(4_000)]}))));
        assert!(rules.is_long_work("verify_tape", &object(json!({"links": [answer(4_000), answer(4_000)]}))));
        let mut chained = answer(20_000);
        chained["chain"] = json!({"kind": "continuation"});
        assert!(!rules.is_long_work("verify_tape", &object(chained)));
    }

    /// Requests that are running hold the door's places: with as many
    /// running as the door runs at once, the next is told so by name before
    /// its body is read, and an address at its own limit is told that.
    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn running_requests_fill_the_door_and_the_next_is_told_so_by_name() {
        let state = AppState::with_hold(4, Duration::from_millis(800));
        let narrow = OpenRules { max_in_flight: 2, max_in_flight_per_address: 1, physics_permits: 4, ..rules() };
        let url = serve_in_process(state, narrow).await;
        let first = tokio::spawn({ let url = url.clone(); async move { post(&url, "10.0.0.1", step()).await } });
        tokio::time::sleep(Duration::from_millis(200)).await;
        let (status, body) = post(&url, "10.0.0.1", call("list_solvers", json!({}))).await;
        assert_eq!((status, body.as_str()), (429, "Too many requests in flight from this address: the open connection runs at most 1 at once from one address (max-in-flight-per-address). Send this one again when one has been answered."));
        let second = tokio::spawn({ let url = url.clone(); async move { post(&url, "10.0.0.2", step()).await } });
        tokio::time::sleep(Duration::from_millis(200)).await;
        let (status, body) = post(&url, "10.0.0.3", call("list_solvers", json!({}))).await;
        assert_eq!((status, body.as_str()), (429, "Too many requests in flight: the open connection runs at most 2 at once (max-in-flight). Send this one again."));
        assert!(first.await.unwrap().1.contains("sha256_seal"));
        assert!(second.await.unwrap().1.contains("sha256_seal"));
        let (status, _) = post(&url, "10.0.0.3", call("list_solvers", json!({}))).await;
        assert_eq!(status, 200);
    }

    /// The wall-clock ceiling: the call is answered by name when its answer
    /// is late, the work is not stopped, and its places come back when it ends.
    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn a_late_answer_is_named_and_its_places_come_back() {
        let state = AppState::with_hold(2, Duration::from_millis(900));
        let pool = Arc::clone(&state.physics_permits);
        let short = OpenRules { request: Duration::from_millis(200), ..rules() };
        let url = serve_in_process(state, short.clone()).await;
        let started = Instant::now();
        let (status, body) = post(&url, "10.0.0.1", run(293.15)).await;
        assert!(started.elapsed() < Duration::from_millis(800), "{:?}", started.elapsed());
        assert_eq!(status, 200);
        let answer: Value = serde_json::from_str(&body).unwrap();
        assert_eq!(answer["result"]["isError"], json!(true));
        assert_eq!(answer["result"]["content"][0]["text"], json!(short.too_slow()));
        assert!(short.too_slow().contains("OPEN_REQUEST_SECONDS"));
        assert_eq!(pool.available_permits(), 1, "the work goes on and holds its permit");
        tokio::time::sleep(Duration::from_millis(1100)).await;
        assert_eq!(pool.available_permits(), 2);
        // A call that needs no physics is not slowed by any of this.
        let (_, listed) = post(&url, "10.0.0.1", call("list_solvers", json!({}))).await;
        assert!(listed.contains("atom_dft"), "{listed}");
    }

    /// Each limit of a dataset is refused by name with the pool held full,
    /// which shows that it is refused before a permit is asked for.
    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn a_dataset_over_a_limit_is_refused_by_name_before_a_permit() {
        let state = AppState::new(2);
        let pool = Arc::clone(&state.physics_permits);
        let url = serve_in_process(state, rules()).await;
        let held: Vec<_> = (0..2).map(|_| Arc::clone(&pool).try_acquire_owned().unwrap()).collect();
        let grid = |values: usize, steps: u64, record_every: u64, padding: usize| {
            let load = json!({"heat_power_w": 1.0, "ambient_k": 293.15});
            let mut segments = vec![json!({"steps": steps, "load": load})];
            for _ in 0..padding {
                segments.push(json!({"steps": 1, "load": load}));
            }
            call("run_dataset", json!({"domain":"energy","card":"thermal","plan":{"kind":"grid",
                "base":{"initial":{},"dt_s":0.1,"segments":segments,"record_every":record_every},
                "axes":[{"path":"initial.temperature_k","values":(0..values).map(|i| 293.15 + i as f64 * 0.01).collect::<Vec<_>>()}]},
                "store":"none","return":"inline"}))
        };
        let refused = |body: &str| -> String {
            let answer: Value = serde_json::from_str(body).unwrap();
            assert_eq!(answer["result"]["isError"], json!(true), "{body}");
            answer["result"]["content"][0]["text"].as_str().unwrap().to_string()
        };
        let (_, body) = post(&url, "10.0.0.1", grid(301, 10, 10, 0)).await;
        assert_eq!(refused(&body), "Physical Violation: the dataset asks for 301 runs, more than the 300 one job may have on the open connection (OPEN_MAX_DATASET_ROWS). The engine's own limit is 100000; this connection is open to anyone and runs smaller jobs.");
        let (_, body) = post(&url, "10.0.0.1", grid(100, 10_001, 10_001, 0)).await;
        assert_eq!(refused(&body), "Physical Violation: the dataset asks for 1000100 steps in total, more than the 1000000 one job may have on the open connection (OPEN_MAX_DATASET_STEPS). The engine's own limit is 100000000; this connection is open to anyone and runs smaller jobs.");
        // A base run long in text, repeated in every one of 300 rows.
        let (_, body) = post(&url, "10.0.0.1", grid(300, 10, 1_000_000, 400)).await;
        let text = refused(&body);
        assert!(text.contains("more than the 4194304 one answer may have on the open connection (OPEN_MAX_DATASET_ANSWER_BYTES)"), "{text}");
        // Each run is counted at 4000 bytes beyond its own request, the open
        // door's own number: 300 runs of a request of about 7,600 bytes come
        // to about 3.5 million bytes and are admitted. Counted at the 8000
        // characters the tools state for a row, they would not be.
        assert_eq!(OPEN_DATASET_ROW_BYTES, 4_000);
        // Admitted by the open door, it is then held to the engine's own
        // limit of an inline answer, before a permit is asked for.
        let (_, body) = post(&url, "10.0.0.1", grid(300, 10, 1_000_000, 50)).await;
        assert!(refused(&body).contains("one inline answer may have"), "{body}");
        let asked = serde_json::to_string(&grid(300, 10, 1_000_000, 50)).unwrap().len() as u64;
        assert!(asked * 301 + 300 * 4_000 <= OPEN_MAX_DATASET_ANSWER_BYTES, "{asked}");
        assert!(asked * 301 + 300 * 8_000 > OPEN_MAX_DATASET_ANSWER_BYTES, "{asked}");
        // At the limit of runs the open door admits the job, and the engine's
        // limit of an inline answer refuses it. A job inside both is admitted,
        // and only then is the pool looked at.
        let (_, body) = post(&url, "10.0.0.1", grid(300, 3_333, 3_333, 0)).await;
        assert!(refused(&body).contains("one inline answer may have"), "{body}");
        let (_, body) = post(&url, "10.0.0.1", grid(20, 10_000, 10_000, 0)).await;
        assert_eq!(refused(&body), BUSY);
        assert_eq!(pool.available_permits(), 0);
        drop(held);
        let (_, body) = post(&url, "10.0.0.1", grid(20, 10_000, 10_000, 0)).await;
        assert!(body.contains("dataset_seal") && !body.contains("\"isError\":true"), "{:.300}", body);
        // Nothing is stored on this door.
        let mut to_file: Value = serde_json::from_str(&dataset(293.15)).unwrap();
        to_file["params"]["arguments"]["return"] = json!("file");
        let (_, body) = post(&url, "10.0.0.1", to_file.to_string()).await;
        assert_eq!(refused(&body), "this connection was started without a directory for dataset files.");
        let (_, body) = post(&url, "10.0.0.1", call("verify_dataset", json!({"dataset_seal": "ab".repeat(32)}))).await;
        assert!(refused(&body).starts_with("this connection was started without a directory for dataset files."));
        assert_eq!(pool.available_permits(), 2);
    }

    /// A request of revision 2026-07-28 meets the same limits in the same
    /// order: with the pool held full a dataset over a limit is refused by
    /// name, a step is busy, and a request that states its revision the wrong
    /// way is told so, none of them taking a permit.
    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn a_request_of_the_newest_revision_meets_every_limit_before_a_permit() {
        let state = AppState::new(2);
        let pool = Arc::clone(&state.physics_permits);
        let url = serve_in_process(state, rules()).await;
        let modern = |body: String| -> String {
            let mut value: Value = serde_json::from_str(&body).unwrap();
            value["params"]["_meta"] = json!({"io.modelcontextprotocol/protocolVersion": "2026-07-28", "io.modelcontextprotocol/clientCapabilities": {}});
            value.to_string()
        };
        let send = |body: String, name: Option<&'static str>| {
            let url = url.clone();
            async move {
                let mut request = reqwest::Client::new()
                    .post(&url)
                    .header("Content-Type", "application/json")
                    .header("x-client", "10.0.0.1")
                    .header("MCP-Protocol-Version", "2026-07-28")
                    .header("Mcp-Method", "tools/call");
                if let Some(name) = name {
                    request = request.header("Mcp-Name", name);
                }
                let response = request.body(body).send().await.unwrap();
                (response.status().as_u16(), serde_json::from_str::<Value>(&response.text().await.unwrap()).unwrap())
            }
        };
        let held: Vec<_> = (0..2).map(|_| Arc::clone(&pool).try_acquire_owned().unwrap()).collect();
        let too_many = call("run_dataset", json!({"domain":"energy","card":"thermal","plan":{"kind":"grid",
            "base":{"initial":{},"dt_s":0.1,"segments":[{"steps":10,"load":{"heat_power_w":1.0,"ambient_k":293.15}}],"record_every":10},
            "axes":[{"path":"initial.temperature_k","values":(0..301).map(|i| 293.15 + f64::from(i) * 0.01).collect::<Vec<_>>()}]},
            "store":"none","return":"inline"}));
        let (status, answer) = send(modern(too_many), Some("run_dataset")).await;
        assert_eq!(status, 200);
        assert_eq!(answer["result"]["resultType"], json!("complete"));
        assert_eq!(answer["result"]["isError"], json!(true));
        assert!(answer["result"]["content"][0]["text"].as_str().unwrap().contains("(OPEN_MAX_DATASET_ROWS)"), "{answer}");
        let (status, answer) = send(modern(step()), Some("verify_step")).await;
        assert_eq!((status, &answer["result"]["content"][0]["text"], &answer["result"]["resultType"]), (200, &json!(BUSY), &json!("complete")));
        // Headers that do not say what the body says, and a body that does not state its revision.
        let (status, answer) = send(modern(step()), Some("run_dataset")).await;
        assert_eq!((status, &answer["error"]["code"]), (400, &json!(-32020)));
        let (status, answer) = send(modern(step()), None).await;
        assert_eq!((status, &answer["error"]["code"]), (400, &json!(-32020)));
        let (status, answer) = send(step(), Some("verify_step")).await;
        assert_eq!((status, &answer["error"]["code"]), (400, &json!(-32602)));
        assert_eq!(pool.available_permits(), 0);
        drop(held);
        let (status, answer) = send(modern(step()), Some("verify_step")).await;
        assert_eq!(status, 200);
        assert!(answer["result"]["content"][0]["text"].as_str().unwrap().contains("sha256_seal"), "{answer}");
        assert_eq!(pool.available_permits(), 2);
    }

    /// The door keeps a count for so many addresses and no more. When the
    /// count is full, the addresses that have been quiet a minute are
    /// forgotten, and when none has, the one heard from longest ago gives up
    /// its place. A newcomer is never turned away because others filled the
    /// count, and the door's memory does not grow with a flood of addresses.
    #[test]
    fn the_addresses_the_door_counts_are_bounded() {
        let rules = OpenRules { requests_per_minute: 5, max_tracked_addresses: 8, ..rules() };
        let sources = Sources::new(rules);
        let start = Instant::now();
        let address = |n: u32| IpAddr::V4(Ipv4Addr::from(0x0a00_0000 + n));
        for n in 0..8 {
            assert!(sources.admit(address(n), start + Duration::from_millis(u64::from(n))).is_ok());
        }
        assert_eq!(sources.tracked(), 8);
        // A hundred newcomers are each served, and the count stays at eight.
        let later = start + Duration::from_secs(1);
        let admitted = (100..200).filter(|n| sources.admit(address(*n), later).is_ok()).count();
        assert_eq!(admitted, 100);
        assert_eq!(sources.tracked(), 8);
        // An address with a request in flight keeps its place, and its allowance, through such a flood.
        let busy = sources.admit(address(900), later).unwrap();
        let _place = Arc::clone(&busy.in_flight).try_acquire_owned().unwrap();
        for _ in 0..4 {
            assert!(sources.admit(address(900), later).is_ok());
        }
        for n in 1000..1100 {
            assert!(sources.admit(address(n), later).is_ok());
        }
        assert_eq!(sources.admit(address(900), later).err(), Some(12));
        assert_eq!(sources.tracked(), 8);
        // A minute on, the quiet ones are forgotten at once.
        assert!(sources.admit(address(5000), later + Duration::from_secs(61)).is_ok());
        assert_eq!(sources.tracked(), 2);
    }

    #[test]
    fn an_allowance_refills_and_says_how_long_to_wait() {
        let rules = OpenRules { requests_per_minute: 6, ..rules() };
        let sources = Sources::new(rules);
        let start = Instant::now();
        let address: IpAddr = "192.0.2.7".parse().unwrap();
        for _ in 0..6 {
            assert!(sources.admit(address, start).is_ok());
        }
        // Six a minute is one every ten seconds.
        assert_eq!(sources.admit(address, start).err(), Some(10));
        assert_eq!(sources.admit(address, start + Duration::from_secs(4)).err(), Some(6));
        assert!(sources.admit(address, start + Duration::from_secs(10)).is_ok());
        assert_eq!(sources.admit(address, start + Duration::from_secs(10)).err(), Some(10));
        // Every address of one IPv6 network of 64 bits is one caller; the next network is another.
        let v6 = |text: &str| -> IpAddr { text.parse().unwrap() };
        for n in 0..6 {
            assert!(sources.admit(v6(&format!("2001:db8:1:2::{n:x}")), start).is_ok());
        }
        assert!(sources.admit(v6("2001:db8:1:2:ffff:ffff:ffff:ffff"), start).is_err());
        assert!(sources.admit(v6("2001:db8:1:3::1"), start).is_ok());
        // An IPv4 address written as IPv6 is that IPv4 address.
        assert_eq!(counted_address(v6("::ffff:192.0.2.7")), address);
    }
}
