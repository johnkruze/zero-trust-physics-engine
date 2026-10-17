//! The hosted door: the Model Context Protocol over HTTP, one endpoint
//! `/mcp`, a key per client, and a view of the engine per key.
//!
//! **Transport.** Streamable HTTP as the MCP specification describes it, for
//! a server that never streams: every `POST /mcp` carries one JSON-RPC message
//! and is answered with `application/json` (a request) or `202 Accepted` with
//! no body (a notification or a response). `GET` and `DELETE` on the endpoint
//! answer 405 with `Allow: POST`, which the specification allows a server that
//! offers no event stream and no sessions. The door keeps no session: it sends
//! no `Mcp-Session-Id`, and every request stands alone. A batch (an array) is
//! refused, as the 2025-06-18 revision removed batching. The tools run the same
//! functions as the stdio door and return the same JSON.
//!
//! **Revisions.** The door speaks every revision in [`SUPPORTED_VERSIONS`],
//! and each request says by itself which it is of. An `initialize` is of the
//! revisions that have one, up to 2025-11-25, and is answered with the version
//! it asks for or the newest of those. A request that states revision
//! 2026-07-28 in `params._meta`, and every `server/discover`, is of the
//! revision with no `initialize`: it must carry the client's capabilities
//! beside its version, the `MCP-Protocol-Version` and `Mcp-Method` headers
//! must say what its body says, and so must `Mcp-Name` on a `tools/call`; its
//! result carries `resultType`. A request whose `MCP-Protocol-Version` header
//! names a revision the door does not speak is answered `400` with the list of
//! those it does, whatever its method.
//!
//! **Order of a request.** The key is read first. A missing, malformed or
//! unknown key is a 401 with one fixed body, and nothing else is read or
//! computed. Then the method, the origin and the content type are checked;
//! then the client's cap on requests in flight is looked
//! at, and a client at its cap is answered 429 before its body is read; then
//! the body is received, within a time limit and without holding one of the
//! client's slots; then the slot is taken and the body is passed through the
//! guards the stdio door has; then the revision the request states is checked
//! against its headers, and only then is a tool called.
//!
//! **Limits of the connection.** Four, each a named constant in [`Limits`]:
//! the time to receive a request's headers, the time to receive its whole
//! body, the time a kept-alive connection may sit idle, and the number of
//! connections open at once. They are described in `AGENTS.md` and
//! `HOSTING.md`.

use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use axum::body::{Body, Bytes, HttpBody};
use axum::extract::State;
use axum::http::{HeaderMap, HeaderValue, Method, Request, StatusCode, header};
use axum::response::Response;
use axum::routing::{any, get};
use axum::Router;
use rmcp::handler::server::ServerHandler;
use rmcp::model::{CacheScope, DiscoverResult, InitializeRequestParams, ListToolsResult};
use serde_json::{Map, Value, json};

use crate::guard::{self, Inspection, LINE_LIMIT};
use crate::json_text;
use crate::keys::{self, Client, Keys};
use crate::mcp_server::{self, McpGateway};
use crate::{AppState, new_gateway_state, prepare_dataset_dir};

pub use crate::open_door::*;

pub use crate::mcp_server::{FIRST_VERSION_WITHOUT_INITIALIZE, SUPPORTED_VERSIONS, begins_with_initialize, speaks};
/// The version the door reads a request as when it carries no
/// `MCP-Protocol-Version` header, as the specification tells a server to.
pub const ASSUMED_VERSION: &str = "2025-03-26";

/// The limits of a connection to the door.
#[derive(Clone, Copy, Debug)]
pub struct Limits {
    /// The longest a request's headers may take to arrive, counted from the
    /// moment the door starts waiting for them.
    pub header_read: Duration,
    /// The longest the whole body of a request may take to arrive, counted
    /// from the end of its headers.
    pub body_read: Duration,
    /// The longest a connection kept alive between two requests may sit with
    /// nothing sent. The library times the wait for the next request's headers
    /// with the same timer as the headers, so the door applies the larger of
    /// this and `header_read` to both.
    pub idle_keep_alive: Duration,
    /// Connections open at once. One more is accepted and closed at once.
    pub max_connections: usize,
    /// The most bytes of headers the library reads before it answers 431.
    pub max_header_bytes: usize,
}

/// Seconds the door waits for the headers of a request.
pub const HEADER_READ_SECONDS: u64 = 30;
/// Seconds the door waits for the whole body of a request.
pub const BODY_READ_SECONDS: u64 = 30;
/// Seconds a kept-alive connection may sit idle.
pub const IDLE_KEEP_ALIVE_SECONDS: u64 = 30;
/// Connections open at once.
pub const MAX_OPEN_CONNECTIONS: usize = 256;
/// Bytes of headers read before the library answers 431.
pub const MAX_HEADER_BYTES: usize = 65_536;

impl Limits {
    pub const DEFAULT: Limits = Limits {
        header_read: Duration::from_secs(HEADER_READ_SECONDS),
        body_read: Duration::from_secs(BODY_READ_SECONDS),
        idle_keep_alive: Duration::from_secs(IDLE_KEEP_ALIVE_SECONDS),
        max_connections: MAX_OPEN_CONNECTIONS,
        max_header_bytes: MAX_HEADER_BYTES,
    };
}

// ------------------------------------------------ the limits of the open door
//
// `hosted_door --open` asks no caller for a key, so the binary holds these in
// the key's place. They are sized for a machine of one core, from times
// measured on one core (`HOSTING.md`, "What the limits were measured against").
// The machinery is in `src/open_door.rs`.

/// Requests one source address may make in a minute, and the most it may
/// make at once after a quiet minute. One more is answered `429` with
/// `Retry-After`. `--requests-per-minute` changes it.
pub const OPEN_REQUESTS_PER_MINUTE: u32 = 300;
/// Requests in flight at once, from every address together.
/// `--max-in-flight` changes it.
pub const OPEN_MAX_IN_FLIGHT: usize = 16;
/// Requests in flight at once from one source address.
/// `--max-in-flight-per-address` changes it.
pub const OPEN_MAX_IN_FLIGHT_PER_ADDRESS: usize = 4;
/// The fewest permits the open door's physics pool has, whatever the machine.
/// With two, dataset jobs together hold one (half the pool, the engine's own
/// rule) and one is left that only a step, a run, a solve or a validation can
/// take. `--physics-permits` changes the pool.
pub const OPEN_PHYSICS_PERMITS_AT_LEAST: usize = 2;
/// The most steps a run may ask and still be short work. A run of more, a
/// solve and a dataset job are long work, and long work together holds at
/// most half the physics pool: on a pool of two, one permit, so the other is
/// always left for a step or a short run.
pub const OPEN_LIGHT_RUN_STEPS: u64 = 10_000;
/// Physics permits one source address may hold at once: how many of its
/// calls compute at the same time. Every agent that reaches the door through
/// one maker's servers arrives from that maker's addresses and shares this.
/// `--permits-per-address` changes it.
pub const OPEN_PERMITS_PER_ADDRESS: usize = 1;
/// The longest request body the open door reads: 4 MiB, a quarter of what a
/// key is given.
pub const OPEN_MAX_BODY_BYTES: usize = 4 * 1024 * 1024;
/// Seconds the open door waits for the whole body of a request.
pub const OPEN_BODY_READ_SECONDS: u64 = 10;
/// Seconds the open door waits for the answer to one tool call.
pub const OPEN_REQUEST_SECONDS: u64 = 60;
/// Runs in one dataset job on the open door.
pub const OPEN_MAX_DATASET_ROWS: u64 = 300;
/// Steps in one dataset job on the open door, every run added together: as
/// many as one run may ask, so that a job costs at most what a run can.
pub const OPEN_MAX_DATASET_STEPS: u64 = 1_000_000;
/// The most bytes a dataset's answer may come to, counted from the plan
/// before it runs: 4 MiB.
pub const OPEN_MAX_DATASET_ANSWER_BYTES: u64 = 4 * 1024 * 1024;
/// The bytes the open door counts for each run of a dataset beyond the run's
/// own request, when it counts an answer from its plan: the final state and
/// the seals of a row. It is the open door's own number and does not follow
/// the size the tools state for a row, so the plans this door refuses do not
/// move when a card with a larger state is served. A row of a card whose
/// final state is larger than this is counted short by the difference.
pub const OPEN_DATASET_ROW_BYTES: u64 = 4_000;
/// Source addresses the open door keeps a count for. When the count is full,
/// addresses quiet for a minute are forgotten, and when none is, the one
/// heard from longest ago gives up its place.
pub const OPEN_MAX_TRACKED_ADDRESSES: usize = 16_384;

/// The address of the other end of a connection, as the socket gives it.
#[derive(Clone, Copy, Debug)]
pub struct PeerAddress(pub std::net::IpAddr);

/// The one body of every 401.
pub const UNAUTHORIZED_BODY: &str = "{\"error\":\"unauthorized\"}";
pub(crate) const BUSY: &str = "Too many concurrent physics tasks.";

/// How many body buffers the door keeps between requests. Each grows to the
/// longest body it has held, at most the line limit, so the door keeps at
/// most this many times 16 MiB for bodies when it is idle.
const BODY_BUFFERS_KEPT: usize = 4;

/// The buffers request bodies are read into. A body is read into a buffer
/// taken from here and the buffer comes back when the request has been read
/// and looked at, so a run of large bodies reuses the same memory where it
/// used to ask the system for a new block for each.
#[derive(Default)]
pub(crate) struct BodyBuffers {
    idle: std::sync::Mutex<Vec<Vec<u8>>>,
}

impl BodyBuffers {
    pub(crate) fn take(self: &Arc<Self>) -> BodyBuffer {
        let bytes = self
            .idle
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .pop()
            .unwrap_or_default();
        BodyBuffer { bytes, home: Arc::clone(self) }
    }

    fn give(&self, mut bytes: Vec<u8>) {
        bytes.clear();
        let mut idle = self.idle.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        if idle.len() < BODY_BUFFERS_KEPT && bytes.capacity() <= LINE_LIMIT {
            idle.push(bytes);
        }
    }
}

/// One body, in a buffer that goes back to the door when this is dropped.
pub(crate) struct BodyBuffer {
    pub(crate) bytes: Vec<u8>,
    home: Arc<BodyBuffers>,
}

impl std::ops::Deref for BodyBuffer {
    type Target = [u8];

    fn deref(&self) -> &[u8] {
        &self.bytes
    }
}

impl Drop for BodyBuffer {
    fn drop(&mut self) {
        self.home.give(std::mem::take(&mut self.bytes));
    }
}

struct Door {
    keys: Keys,
    /// One state for each client, in the order of the keys: the caller's view,
    /// the caller's dataset folder, and the permit pools every client shares.
    states: Vec<AppState>,
    gateway: McpGateway,
    allowed_origins: Vec<String>,
    body_read: Duration,
    bodies: Arc<BodyBuffers>,
}

/// Arguments of the `hosted_door` binary.
pub struct Options {
    pub port: u16,
    /// The keys file of a door that asks every caller for a key. `None` for
    /// the open door, which asks none.
    pub keys: Option<PathBuf>,
    pub dataset_dir: Option<PathBuf>,
    pub allowed_origins: Vec<String>,
    /// The open door's rules: `Some` when `--open` was given.
    pub open: Option<OpenRules>,
}

const USAGE: &str = "It reads --port <number>, --keys <file>, --dataset-dir <path> and --allowed-origin <origin>. --port and --keys are required. --allowed-origin may be given more than once. In place of --keys it reads --open, which asks no caller for a key, and with --open it reads --client-address-header <name>, --requests-per-minute <number>, --max-in-flight <number>, --max-in-flight-per-address <number>, --physics-permits <number>, --permits-per-address <number> and --request-seconds <number>.";

/// The arguments only the open door reads.
const OPEN_ONLY: [&str; 7] = [
    "--client-address-header",
    "--requests-per-minute",
    "--max-in-flight",
    "--max-in-flight-per-address",
    "--physics-permits",
    "--permits-per-address",
    "--request-seconds",
];

impl Options {
    pub fn from_args(arguments: impl IntoIterator<Item = String>) -> Result<Options, String> {
        let mut port: Option<u16> = None;
        let mut keys: Option<PathBuf> = None;
        let mut dataset_dir: Option<PathBuf> = None;
        let mut origins: Vec<String> = Vec::new();
        let mut open = false;
        let mut rules = OpenRules::for_this_machine();
        let mut open_only: Vec<&'static str> = Vec::new();
        let mut arguments = arguments.into_iter();
        while let Some(argument) = arguments.next() {
            if argument == "--open" {
                if open {
                    return Err("--open was given more than once.".to_string());
                }
                open = true;
                continue;
            }
            let (name, inline) = match argument.split_once('=') {
                Some((name, value)) if name.starts_with("--") => (name.to_string(), Some(value.to_string())),
                _ => (argument.clone(), None),
            };
            if !["--port", "--keys", "--dataset-dir", "--allowed-origin"].contains(&name.as_str())
                && !OPEN_ONLY.contains(&name.as_str())
            {
                return Err(format!("'{argument}' is not an argument of this program. {USAGE}"));
            }
            let value = match inline {
                Some(value) => value,
                None => arguments
                    .next()
                    .ok_or_else(|| format!("{name} needs a value."))?,
            };
            let twice = || format!("{name} was given more than once.");
            match name.as_str() {
                "--port" => {
                    let number: u16 = value
                        .parse()
                        .map_err(|_| format!("--port needs a number from 0 to 65535, not '{value}'."))?;
                    if port.replace(number).is_some() {
                        return Err(twice());
                    }
                }
                "--keys" => {
                    if keys.replace(PathBuf::from(value)).is_some() {
                        return Err(twice());
                    }
                }
                "--dataset-dir" => {
                    if dataset_dir.replace(PathBuf::from(value)).is_some() {
                        return Err(twice());
                    }
                }
                "--allowed-origin" => origins.push(value),
                other => {
                    let known = OPEN_ONLY
                        .iter()
                        .copied()
                        .find(|flag| *flag == other)
                        .expect("an argument of the open door");
                    if open_only.contains(&known) {
                        return Err(twice());
                    }
                    open_only.push(known);
                    rules.read_flag(known, &value)?;
                }
            }
        }
        let port = port.ok_or_else(|| format!("--port is required. {USAGE}"))?;
        if open {
            if keys.is_some() {
                return Err("--open and --keys cannot be given together: a door asks every caller for a key or asks none.".to_string());
            }
            if dataset_dir.is_some() {
                return Err("--dataset-dir cannot be given with --open: without a key no caller has a folder of its own, so the open door stores no dataset and returns every dataset inline.".to_string());
            }
            rules.check()?;
            return Ok(Options { port, keys: None, dataset_dir: None, allowed_origins: origins, open: Some(rules) });
        }
        if let Some(flag) = open_only.first() {
            return Err(format!("{flag} is an argument of the open door and needs --open."));
        }
        Ok(Options {
            port,
            keys: Some(keys.ok_or_else(|| format!("--keys is required. {USAGE}"))?),
            dataset_dir,
            allowed_origins: origins,
            open: None,
        })
    }
}

/// The hosted router over a keys file and, optionally, a dataset directory.
/// Each client's datasets live in a folder of the directory named by the
/// SHA-256 of the client's `key_sha256`.
pub fn router(
    keys: Keys,
    dataset_dir: Option<PathBuf>,
    allowed_origins: Vec<String>,
) -> Result<Router, String> {
    router_over(new_gateway_state(), keys, dataset_dir, allowed_origins)
}

/// The same over a given base state, so a test can hold the permit pools.
#[doc(hidden)]
pub fn router_over(
    base: AppState,
    keys: Keys,
    dataset_dir: Option<PathBuf>,
    allowed_origins: Vec<String>,
) -> Result<Router, String> {
    router_limited(base, keys, dataset_dir, allowed_origins, Limits::DEFAULT)
}

/// The same with the limits of the connection chosen, so a test need not
/// wait the time the door waits.
#[doc(hidden)]
pub fn router_limited(
    base: AppState,
    keys: Keys,
    dataset_dir: Option<PathBuf>,
    allowed_origins: Vec<String>,
    limits: Limits,
) -> Result<Router, String> {
    let pool = base.physics_pool_size();
    for client in keys.clients() {
        if client.max_permits > pool {
            return Err(format!(
                "client '{}': 'max_permits' is {}, more than the {pool} permits of the physics pool of this machine. A client's share cannot be larger than the pool.",
                client.label, client.max_permits
            ));
        }
    }
    let root = dataset_dir.map(|path| prepare_dataset_dir(&path)).transpose()?;
    let mut states = Vec::with_capacity(keys.clients().len());
    for client in keys.clients() {
        let folder = match &root {
            Some(root) => {
                let folder = root.join(&client.folder);
                make_private_folder(&folder).map_err(|error| {
                    format!(
                        "the dataset folder of client '{}' cannot be made: {error}.",
                        client.label
                    )
                })?;
                Some(folder)
            }
            None => None,
        };
        states.push(
            base.clone()
                .with_dataset_dir(folder)
                .with_view(Arc::clone(&client.view))
                .with_share(Arc::clone(&client.permit_share)),
        );
    }
    let door = Arc::new(Door {
        keys,
        states,
        gateway: McpGateway::new(),
        allowed_origins: allowed_origins
            .into_iter()
            .map(|origin| origin.to_ascii_lowercase())
            .collect(),
        body_read: limits.body_read,
        bodies: Arc::new(BodyBuffers::default()),
    });
    Ok(Router::new()
        .route("/health", get(|| async { StatusCode::OK }))
        .route("/mcp", any(mcp))
        .layer(axum::middleware::from_fn(refuse_both_lengths))
        .with_state(door))
}

/// What the door answers a request that carries both a `Content-Length` and
/// a `Transfer-Encoding` header.
pub const BOTH_LENGTHS: &str =
    "Bad Request: a request must not carry both Content-Length and Transfer-Encoding.";

/// A request that carries both a `Content-Length` and a `Transfer-Encoding`
/// header is refused with 400 before anything else is looked at, the key
/// included, and the connection is closed: two readers of such a request can
/// disagree on where it ends. The library refuses two lengths, two encodings
/// and a short length the same way. The connection's own record says whether
/// this request carried both (`framing::Framing`); a request served without
/// such a record, as in a test that calls the router by itself, is passed on.
pub(crate) async fn refuse_both_lengths(request: Request<Body>, next: axum::middleware::Next) -> Response {
    let both = request
        .extensions()
        .get::<Arc<crate::framing::Framing>>()
        .is_some_and(|framing| framing.next_carries_both());
    if both {
        let mut response = plain(StatusCode::BAD_REQUEST, BOTH_LENGTHS);
        response
            .headers_mut()
            .insert(header::CONNECTION, HeaderValue::from_static("close"));
        return response;
    }
    next.run(request).await
}

/// A client's dataset folder, readable by the account that runs the door and
/// no other. A folder made by an earlier start with wider modes is narrowed.
fn make_private_folder(folder: &std::path::Path) -> std::io::Result<()> {
    use std::os::unix::fs::{DirBuilderExt, PermissionsExt};
    std::fs::DirBuilder::new().recursive(true).mode(0o700).create(folder)?;
    std::fs::set_permissions(folder, std::fs::Permissions::from_mode(0o700))
}

pub async fn serve(options: Options) -> Result<(), String> {
    if let Some(rules) = options.open {
        return crate::open_door::serve(options.port, rules, options.allowed_origins).await;
    }
    let keys = Keys::load(options.keys.as_deref().expect("a door that is not open has a keys file"))?;
    let labels: Vec<String> = keys.clients().iter().map(|client| client.label.clone()).collect();
    let app = router(keys, options.dataset_dir, options.allowed_origins)?;
    let address = SocketAddr::from(([127, 0, 0, 1], options.port));
    let listener = tokio::net::TcpListener::bind(address)
        .await
        .map_err(|error| format!("it cannot listen on {address}: {error}."))?;
    let bound = listener.local_addr().map_err(|error| error.to_string())?;
    announce(&format!("Hosted door running on {bound} for {} clients: {}", labels.len(), labels.join(", ")));
    serve_listener(listener, app, Limits::DEFAULT).await;
    Ok(())
}

/// What a door prints when it starts, written to standard output in one
/// piece, every line of it in one write. Whoever started the door may have
/// closed that output by then: the door has its port and serves whether or
/// not anyone reads the lines, so a write that fails is let go.
pub(crate) fn announce(text: &str) {
    use std::io::Write;
    let mut out = std::io::stdout().lock();
    let _ = out.write_all(format!("{text}\n").as_bytes()).and_then(|()| out.flush());
}

/// Serve `app` on `listener` for ever, holding every connection to `limits`.
/// A connection over the cap is accepted and closed at once. A failure to
/// accept (no descriptor left) is waited out, not fatal.
#[doc(hidden)]
pub async fn serve_listener(listener: tokio::net::TcpListener, app: Router, limits: Limits) {
    use hyper_util::rt::{TokioIo, TokioTimer};
    use hyper_util::service::TowerToHyperService;

    let open = Arc::new(tokio::sync::Semaphore::new(limits.max_connections));
    let header_wait = limits.header_read.max(limits.idle_keep_alive);
    loop {
        let (stream, peer) = match listener.accept().await {
            Ok(accepted) => accepted,
            Err(_) => {
                tokio::time::sleep(Duration::from_millis(100)).await;
                continue;
            }
        };
        let Ok(place) = Arc::clone(&open).try_acquire_owned() else {
            // Over the cap: the connection is closed at once.
            drop(stream);
            continue;
        };
        // The connection's bytes pass a watch on their way to the library, and
        // each request carries the connection's record to the door, so that a
        // request with both a length and an encoding is known for what it is.
        let framing = Arc::new(crate::framing::Framing::default());
        let inner = TowerToHyperService::new(app.clone());
        let record = Arc::clone(&framing);
        let service = hyper::service::service_fn(move |mut request: hyper::Request<hyper::body::Incoming>| {
            request.extensions_mut().insert(Arc::clone(&record));
            request.extensions_mut().insert(PeerAddress(peer.ip()));
            hyper::service::Service::call(&inner, request)
        });
        let stream = crate::framing::Watched::new(stream, framing);
        tokio::spawn(async move {
            let _place = place;
            let mut connection = hyper::server::conn::http1::Builder::new();
            connection
                .timer(TokioTimer::new())
                .header_read_timeout(header_wait)
                .max_buf_size(limits.max_header_bytes)
                .keep_alive(true);
            let _ = connection
                .serve_connection(TokioIo::new(stream), service)
                .await;
        });
    }
}

fn unauthorized() -> Response {
    let mut response = Response::new(Body::from(UNAUTHORIZED_BODY));
    *response.status_mut() = StatusCode::UNAUTHORIZED;
    let headers = response.headers_mut();
    headers.insert(header::CONTENT_TYPE, HeaderValue::from_static("application/json"));
    headers.insert(header::WWW_AUTHENTICATE, HeaderValue::from_static("Bearer"));
    headers.insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    response
}

pub(crate) fn plain(status: StatusCode, text: &str) -> Response {
    let mut response = Response::new(Body::from(text.to_string()));
    *response.status_mut() = status;
    response
        .headers_mut()
        .insert(header::CONTENT_TYPE, HeaderValue::from_static("text/plain; charset=utf-8"));
    response
}

pub(crate) fn json_response(status: StatusCode, text: String) -> Response {
    let mut response = Response::new(Body::from(text));
    *response.status_mut() = status;
    let headers = response.headers_mut();
    headers.insert(header::CONTENT_TYPE, HeaderValue::from_static("application/json"));
    headers.insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    response
}

/// The client behind the request's key, or nothing for no header, more than
/// one header, a header that is not `Bearer <key>`, and a key nobody holds.
fn who<'a>(door: &'a Door, headers: &HeaderMap) -> Option<usize> {
    let mut values = headers.get_all(header::AUTHORIZATION).iter();
    let value = values.next();
    // The key is looked up whatever the header says, so a missing header and
    // a wrong key take the same steps.
    let key = match (value, values.next()) {
        (Some(value), None) => keys::bearer_key(value.as_bytes()),
        _ => None,
    };
    let client = door.keys.authenticate(key.unwrap_or(b""));
    let client: Option<&Client> = if key.is_some() { client } else { None };
    client.and_then(|found| {
        door.keys
            .clients()
            .iter()
            .position(|candidate| std::ptr::eq(candidate, found))
    })
}

async fn mcp(State(door): State<Arc<Door>>, request: Request<Body>) -> Response {
    let Some(index) = who(&door, request.headers()) else {
        return unauthorized();
    };
    let client = &door.keys.clients()[index];
    let state = door.states[index].clone();
    let (parts, body) = request.into_parts();

    if parts.method != Method::POST {
        // GET would open an event stream and DELETE would end a session. This
        // door offers neither.
        let mut response = plain(StatusCode::METHOD_NOT_ALLOWED, "Method Not Allowed");
        response
            .headers_mut()
            .insert(header::ALLOW, HeaderValue::from_static("POST"));
        return response;
    }
    if let Some(origin) = parts.headers.get(header::ORIGIN) {
        let allowed = origin
            .to_str()
            .map(|text| door.allowed_origins.contains(&text.to_ascii_lowercase()))
            .unwrap_or(false);
        if !allowed {
            return plain(StatusCode::FORBIDDEN, "Forbidden: this origin is not allowed.");
        }
    }
    if !is_json(&parts.headers) {
        return plain(
            StatusCode::UNSUPPORTED_MEDIA_TYPE,
            "Expected request with `Content-Type: application/json`",
        );
    }
    if !accepts_json(&parts.headers) {
        return plain(
            StatusCode::NOT_ACCEPTABLE,
            "Not Acceptable: the client must accept application/json.",
        );
    }
    // What the headers say of the request's revision, method and tool is kept
    // for `answer`, which checks it against the body.
    let named = Named::of(&parts.headers);
    // A client at its cap is told so before its body is read.
    if client.in_flight.available_permits() == 0 {
        return plain(StatusCode::TOO_MANY_REQUESTS, BUSY);
    }
    // A body being received holds a place in `receiving`, which has as many
    // places as the client has slots, and holds none of the slots: a body that
    // does not arrive cannot stop the client's running requests from ending
    // or its slots from being used. It is cut off at the limit.
    let Ok(receiving) = Arc::clone(&client.receiving).try_acquire_owned() else {
        return plain(StatusCode::TOO_MANY_REQUESTS, BUSY);
    };
    let mut bytes = door.bodies.take();
    let received = tokio::time::timeout(door.body_read, read_capped(body, &mut bytes.bytes)).await;
    drop(receiving);
    let Ok(whole) = received else {
        return body_too_slow();
    };
    if !whole {
        return rpc_http_response(guard::too_long_response(&bytes));
    }
    let Ok(permit) = Arc::clone(&client.in_flight).try_acquire_owned() else {
        return plain(StatusCode::TOO_MANY_REQUESTS, BUSY);
    };
    let call = Call { gateway: &door.gateway, state, slot: Box::new(permit), open: None, long_work: None, named };
    answer(call, bytes).await
}

/// The body did not arrive within the limit. The connection is closed after
/// the answer, as a half-received body leaves it unusable.
pub(crate) fn body_too_slow() -> Response {
    let mut response = plain(
        StatusCode::REQUEST_TIMEOUT,
        "Request Timeout: the body did not arrive in time.",
    );
    response
        .headers_mut()
        .insert(header::CONNECTION, HeaderValue::from_static("close"));
    response
}

pub(crate) fn is_json(headers: &HeaderMap) -> bool {
    headers
        .get(header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .map(|text| {
            let mime = text.split(';').next().unwrap_or("").trim().to_ascii_lowercase();
            mime == "application/json" || mime.ends_with("+json")
        })
        .unwrap_or(false)
}

/// A client with no `Accept` header is taken to accept anything. One that
/// names types must name one that covers `application/json`.
pub(crate) fn accepts_json(headers: &HeaderMap) -> bool {
    let mut any = false;
    for value in headers.get_all(header::ACCEPT) {
        let Ok(text) = value.to_str() else { continue };
        any = true;
        for item in text.split(',') {
            let mime = item.split(';').next().unwrap_or("").trim().to_ascii_lowercase();
            if mime == "application/json" || mime == "application/*" || mime == "*/*" {
                return true;
            }
        }
    }
    !any
}

/// The body, read into `kept`, never holding more than the line limit. The
/// answer is false when the body was longer, and `kept` is then its first
/// `LINE_LIMIT` bytes. `kept` is a buffer the door reuses: it grows by
/// doubling and never past the line limit.
pub(crate) async fn read_capped(body: Body, kept: &mut Vec<u8>) -> bool {
    read_capped_at(body, kept, LINE_LIMIT).await
}

/// The same with the limit chosen: the open door reads less.
pub(crate) async fn read_capped_at(mut body: Body, kept: &mut Vec<u8>, limit: usize) -> bool {
    kept.clear();
    loop {
        let frame = std::future::poll_fn(|cx| std::pin::Pin::new(&mut body).poll_frame(cx)).await;
        let Some(Ok(frame)) = frame else {
            return true;
        };
        let Ok(data) = frame.into_data() else { continue };
        let data: Bytes = data;
        let room = limit - kept.len();
        let take = data.len().min(room);
        let needed = kept.len() + take;
        if needed > kept.capacity() {
            let grown = needed.max(kept.capacity().saturating_mul(2)).max(65_536).min(limit);
            kept.reserve_exact(grown - kept.len());
        }
        kept.extend_from_slice(&data[..take]);
        if data.len() > room {
            return false;
        }
    }
}

/// A JSON-RPC answer the door wrote, with the HTTP status that goes with it:
/// 413 for a message over its limit, 400 for any other error answer of the
/// guards, and 200 for a tool refusal, which is a result.
pub(crate) fn rpc_http_response(text: String) -> Response {
    let status = match serde_json::from_str::<Value>(&text) {
        Ok(value) => match value.get("error") {
            Some(error)
                if error
                    .get("message")
                    .and_then(Value::as_str)
                    .is_some_and(|message| message.starts_with("Request too large")) =>
            {
                StatusCode::PAYLOAD_TOO_LARGE
            }
            Some(_) => StatusCode::BAD_REQUEST,
            None => StatusCode::OK,
        },
        Err(_) => StatusCode::OK,
    };
    json_response(status, text)
}

pub(crate) fn accepted() -> Response {
    let mut response = Response::new(Body::empty());
    *response.status_mut() = StatusCode::ACCEPTED;
    response
}

pub(crate) fn rpc_error(id: &Value, code: i64, message: &str) -> String {
    serde_json::to_string(&json!({
        "jsonrpc": "2.0",
        "id": id,
        "error": { "code": code, "message": message }
    }))
    .expect("the error encodes")
}

/// A result as the stdio door writes it. For a request of a revision that
/// begins with `initialize` the library drops a `resultType` of "complete";
/// for a request of a later revision (`modern`) every result carries one, and
/// it comes first.
pub(crate) fn rpc_result(id: &Value, mut result: Value, modern: bool) -> String {
    if let Some(object) = result.as_object_mut() {
        if modern {
            if !object.contains_key("resultType") {
                object.shift_insert(0, "resultType".to_string(), json!("complete"));
            }
        } else if object.get("resultType").and_then(Value::as_str) == Some("complete") {
            object.shift_remove("resultType");
        }
    }
    serde_json::to_string(&json!({ "jsonrpc": "2.0", "id": id, "result": result }))
        .expect("the result encodes")
}

/// The key of `params._meta` under which a request states its revision.
pub const META_VERSION: &str = "io.modelcontextprotocol/protocolVersion";
/// The key of `params._meta` under which a request states what its client can do.
pub const META_CAPABILITIES: &str = "io.modelcontextprotocol/clientCapabilities";
/// The code of the error for headers that do not say what the body says.
pub const HEADER_MISMATCH: i64 = -32020;
/// The code of the error for a revision the door does not speak.
pub const UNSUPPORTED_PROTOCOL_VERSION: i64 = -32022;

/// One header of a request, as the door reads it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Stated {
    Absent,
    One(String),
    /// Not text, or given more than once with different values.
    Unreadable,
}

impl Stated {
    fn of(headers: &HeaderMap, name: &str) -> Stated {
        let mut found = Stated::Absent;
        for value in headers.get_all(name) {
            let Ok(text) = value.to_str() else { return Stated::Unreadable };
            match &found {
                Stated::Absent => found = Stated::One(text.to_string()),
                Stated::One(first) if first == text => {}
                _ => return Stated::Unreadable,
            }
        }
        found
    }

    fn one(&self) -> Option<&str> {
        match self {
            Stated::One(text) => Some(text),
            _ => None,
        }
    }
}

/// What the headers of a request say of its revision, its method and the
/// tool it calls. `answer` holds each against the body.
#[derive(Clone, Debug)]
pub(crate) struct Named {
    pub(crate) version: Stated,
    pub(crate) method: Stated,
    pub(crate) name: Stated,
}

impl Named {
    pub(crate) fn of(headers: &HeaderMap) -> Named {
        Named {
            version: Stated::of(headers, "mcp-protocol-version"),
            method: Stated::of(headers, "mcp-method"),
            name: Stated::of(headers, "mcp-name"),
        }
    }

    /// The revision the `MCP-Protocol-Version` header names, when the door
    /// does not speak it.
    fn version_not_spoken(&self) -> Option<&str> {
        self.version.one().filter(|version| !speaks(version))
    }
}

/// The value of an `Mcp-Name` header as the client meant it: a value written
/// `=?base64?...?=` is the Base64 of the name's UTF-8 bytes. Nothing for a
/// value in that form that does not decode.
fn header_value(text: &str) -> Option<String> {
    match text.strip_prefix("=?base64?").and_then(|rest| rest.strip_suffix("?=")) {
        Some(encoded) => String::from_utf8(base64(encoded)?).ok(),
        None => Some(text.to_string()),
    }
}

/// Base64 of the standard alphabet, with its padding or without.
fn base64(text: &str) -> Option<Vec<u8>> {
    let digits = text.trim_end_matches('=');
    if text.len() - digits.len() > 2 || digits.len() % 4 == 1 {
        return None;
    }
    let mut out = Vec::with_capacity(digits.len() * 3 / 4);
    let (mut held, mut bits) = (0_u32, 0_u32);
    for byte in digits.bytes() {
        let six = match byte {
            b'A'..=b'Z' => byte - b'A',
            b'a'..=b'z' => byte - b'a' + 26,
            b'0'..=b'9' => byte - b'0' + 52,
            b'+' => 62,
            b'/' => 63,
            _ => return None,
        };
        held = (held << 6) | u32::from(six);
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((held >> bits) as u8);
            held &= (1 << bits) - 1;
        }
    }
    Some(out)
}

/// `400` with the error of revision 2026-07-28 for a revision the door does
/// not speak: the revision asked for and the list of those it speaks. A
/// client that speaks one of them sends the request again as that one.
pub(crate) fn unsupported_version(id: &Value, requested: &str) -> Response {
    let text = serde_json::to_string(&json!({
        "jsonrpc": "2.0",
        "id": id,
        "error": {
            "code": UNSUPPORTED_PROTOCOL_VERSION,
            "message": "Unsupported protocol version",
            "data": { "supported": SUPPORTED_VERSIONS, "requested": requested }
        }
    }))
    .expect("the error encodes");
    json_response(StatusCode::BAD_REQUEST, text)
}

/// `400` with the error for headers that do not say what the body says.
fn header_mismatch(id: &Value, sentence: &str) -> Response {
    json_response(
        StatusCode::BAD_REQUEST,
        rpc_error(id, HEADER_MISMATCH, &format!("Header mismatch: {sentence}")),
    )
}

/// Which kind of revision a request is of, from its method, its `_meta` and
/// its headers: true for the revisions with no `initialize`. A request that
/// states a revision the wrong way is the answer to send in its place.
fn stated_revision(id: &Value, method: &str, params: Option<&Value>, named: &Named) -> Result<bool, Response> {
    if named.version == Stated::Unreadable {
        return Err(header_mismatch(
            id,
            "the MCP-Protocol-Version header is not text, or is given more than once with different values.",
        ));
    }
    // An `initialize` is of the revisions that have one, whatever version it
    // asks for: the answer names one of those.
    if method == "initialize" {
        return Ok(false);
    }
    let meta = params.and_then(|params| params.get("_meta")).and_then(Value::as_object);
    let in_body = meta.and_then(|meta| meta.get(META_VERSION)).and_then(Value::as_str);
    let in_header = named.version.one();
    let version = in_body.or(in_header).unwrap_or(ASSUMED_VERSION);
    let modern = method == "server/discover" || !begins_with_initialize(version);
    if modern {
        let mut missing = Vec::new();
        if in_body.is_none() {
            missing.push(META_VERSION);
        }
        if !meta.and_then(|meta| meta.get(META_CAPABILITIES)).is_some_and(Value::is_object) {
            missing.push(META_CAPABILITIES);
        }
        if !missing.is_empty() {
            return Err(json_response(
                StatusCode::BAD_REQUEST,
                rpc_error(
                    id,
                    -32602,
                    &format!(
                        "Invalid params: params._meta is missing or has the wrong type for {}. A request of revision {FIRST_VERSION_WITHOUT_INITIALIZE}, and every server/discover, states its revision and its client's capabilities there.",
                        missing.join(" and ")
                    ),
                ),
            ));
        }
    }
    let Some(in_body) = in_body else {
        return Ok(false);
    };
    // The body states a revision: the header must state the same one.
    match in_header {
        None => {
            return Err(header_mismatch(
                id,
                &format!("the MCP-Protocol-Version header is missing; params._meta states revision '{in_body}'."),
            ));
        }
        Some(header) if header != in_body => {
            return Err(header_mismatch(
                id,
                &format!("the MCP-Protocol-Version header value '{header}' does not match the revision '{in_body}' in params._meta."),
            ));
        }
        Some(_) => {}
    }
    if !speaks(in_body) {
        return Err(unsupported_version(id, in_body));
    }
    if !modern {
        return Ok(false);
    }
    match &named.method {
        Stated::One(header) if header == method => {}
        Stated::One(header) => {
            return Err(header_mismatch(
                id,
                &format!("the Mcp-Method header value '{header}' does not match the method '{method}' in the body."),
            ));
        }
        Stated::Absent => {
            return Err(header_mismatch(id, &format!("the Mcp-Method header is missing; the body's method is '{method}'.")));
        }
        Stated::Unreadable => {
            return Err(header_mismatch(id, "the Mcp-Method header is not text, or is given more than once with different values."));
        }
    }
    // A call with no name to compare is answered as a call with wrong
    // parameters, further on.
    let tool = params.and_then(|params| params.get("name")).and_then(Value::as_str);
    if let (true, Some(tool)) = (method == "tools/call", tool) {
        match &named.name {
            Stated::One(header) => match header_value(header) {
                Some(decoded) if decoded == tool => {}
                Some(decoded) => {
                    return Err(header_mismatch(
                        id,
                        &format!("the Mcp-Name header value '{decoded}' does not match the tool name '{tool}' in the body."),
                    ));
                }
                None => {
                    return Err(header_mismatch(id, "the Mcp-Name header is written as Base64 and does not decode to text."));
                }
            },
            Stated::Absent => {
                return Err(header_mismatch(id, &format!("the Mcp-Name header is missing; the body calls the tool '{tool}'.")));
            }
            Stated::Unreadable => {
                return Err(header_mismatch(id, "the Mcp-Name header is not text, or is given more than once with different values."));
            }
        }
    }
    Ok(true)
}

/// One request as `answer` takes it: whose view and share it runs under, the
/// places it holds until its work ends, and, on the open door, that door's
/// rules.
pub(crate) struct Call<'a> {
    pub(crate) gateway: &'a McpGateway,
    pub(crate) state: AppState,
    pub(crate) slot: Box<dyn Send + 'static>,
    pub(crate) open: Option<&'a OpenRules>,
    /// The open door's places for long work.
    pub(crate) long_work: Option<Arc<tokio::sync::Semaphore>>,
    /// What the request's headers say of its revision, method and tool.
    pub(crate) named: Named,
}

pub(crate) async fn answer(call: Call<'_>, bytes: BodyBuffer) -> Response {
    let Call { gateway, state, slot: permit, open, long_work, named } = call;
    // A header that names a revision the door does not speak is answered
    // `400` whatever the body holds, an `initialize` and a notification
    // included, under the request's id when the body shows one.
    let not_spoken = named.version_not_spoken().map(str::to_string);
    let refuse_version = |id: &Value| unsupported_version(id, not_spoken.as_deref().unwrap_or(""));
    let message = match guard::inspect(&bytes) {
        Inspection::Answer(text) => {
            if not_spoken.is_some() {
                let id = serde_json::from_str::<Value>(&text).ok().and_then(|answer| answer.get("id").cloned());
                return refuse_version(&id.unwrap_or(Value::Null));
            }
            return rpc_http_response(text);
        }
        Inspection::NoAnswer if not_spoken.is_some() => return refuse_version(&Value::Null),
        Inspection::NoAnswer => return accepted(),
        Inspection::Blank => {
            let reason = match json_text::scan(&bytes) {
                json_text::Scan::Unreadable(reason) => reason,
                _ => "the body is empty".to_string(),
            };
            return rpc_http_response(guard::unreadable_response(&bytes, &reason));
        }
        Inspection::Forward => match serde_json::from_slice::<Value>(guard::json_line(&bytes)) {
            Ok(value) => value,
            Err(error) => {
                return rpc_http_response(guard::unreadable_response(&bytes, &error.to_string()));
            }
        },
    };
    let Some(object) = message.as_object() else {
        let text = if message.is_array() {
            "Invalid Request: a batch is not accepted. Send one JSON-RPC message in each request."
        } else {
            "Invalid Request: the body must be one JSON-RPC message, an object."
        };
        return rpc_http_response(rpc_error(&Value::Null, -32600, text));
    };
    if object.get("jsonrpc").and_then(Value::as_str) != Some("2.0") {
        let id = object.get("id").cloned().unwrap_or(Value::Null);
        return rpc_http_response(rpc_error(
            &id,
            -32600,
            "Invalid Request: 'jsonrpc' must be \"2.0\".",
        ));
    }
    let Some(method) = object.get("method") else {
        // A response to a request the server made. The door makes none.
        if object.contains_key("result") || object.contains_key("error") {
            if not_spoken.is_some() {
                return refuse_version(&Value::Null);
            }
            return accepted();
        }
        let id = object.get("id").cloned().unwrap_or(Value::Null);
        return rpc_http_response(rpc_error(&id, -32600, "Invalid Request: 'method' is missing."));
    };
    let Some(method) = method.as_str() else {
        return rpc_http_response(rpc_error(
            &Value::Null,
            -32600,
            "Invalid Request: 'method' must be a string.",
        ));
    };
    let Some(id) = object.get("id") else {
        // A notification: `notifications/initialized`, `notifications/cancelled`
        // and the rest. None has an answer.
        if not_spoken.is_some() {
            return refuse_version(&Value::Null);
        }
        return accepted();
    };
    if not_spoken.is_some() {
        return refuse_version(id);
    }
    let params = object.get("params");
    // Which kind of revision this request is of. Nothing is remembered from
    // an earlier request: each says it by itself.
    let modern = match stated_revision(id, method, params, &named) {
        Ok(modern) => modern,
        Err(response) => return response,
    };
    let body_bytes = bytes.len();
    // A tools/call with wrong parameters is answered with the sentence the
    // stdio door gives for the same fault.
    if method == "tools/call" {
        if let Some(response) = guard::tool_call_fault(&bytes) {
            return json_response(StatusCode::OK, response);
        }
    }
    // A verification's long form holds the answer sent byte for byte, so the
    // exact text of its arguments is kept beside them.
    let arguments_text = match guard::arguments_text(&bytes) {
        Some((name, text)) if method == "tools/call" && guard::TEXT_TOOLS.contains(&name.as_str()) => Some(text),
        _ => None,
    };
    // The body has been read into a message. Its buffer goes back to the door
    // before the tool runs.
    drop(bytes);
    let text = match method {
        // Revision 2026-07-28 removed `ping`: there it is a method not found.
        "ping" if !modern => rpc_result(id, json!({}), false),
        "server/discover" => {
            let found = DiscoverResult::from_server_info(
                gateway.supported_protocol_versions().into_owned(),
                gateway.get_info(),
            );
            let mut found = serde_json::to_value(found).expect("the discovery encodes");
            if let Some(rules) = open {
                rules.state_in_instructions(&mut found);
            }
            rpc_result(id, found, true)
        }
        "initialize" => {
            match params
                .cloned()
                .map(serde_json::from_value::<InitializeRequestParams>)
            {
                Some(Ok(params)) => match gateway.negotiate_initialize(&params) {
                    Ok(info) => {
                        let mut info = serde_json::to_value(info).expect("the info encodes");
                        if let Some(rules) = open {
                            rules.state_in_instructions(&mut info);
                        }
                        rpc_result(id, info, false)
                    }
                    Err(error) => rpc_error(id, i64::from(error.code.0), &error.message),
                },
                _ => rpc_error(id, -32602, "Invalid params: initialize needs protocolVersion, capabilities and clientInfo."),
            }
        }
        "tools/list" => {
            // On the open door each description ends with that door's own
            // limits for the tool, and each tool is marked read-only.
            let mut listed = ListToolsResult::with_all_items(mcp_server::tools_for(state.view(), open));
            if modern {
                // The hints revision 2026-07-28 requires on a list, with the
                // values the stdio door gives: not to be kept, and not to be
                // shared between callers.
                listed.ttl_ms = Some(0);
                listed.cache_scope = Some(CacheScope::Private);
            }
            rpc_result(id, serde_json::to_value(listed).expect("the tools encode"), modern)
        }
        "tools/call" => match tool_call(params) {
            Ok((name, arguments)) => {
                // What the open door will not run is refused here, by name,
                // before the tool is called and so before any permit.
                if let Some(fault) = open.and_then(|rules| rules.fault(&name, &arguments, body_bytes)) {
                    let refused = rmcp::model::CallToolResult::error(vec![rmcp::model::ContentBlock::text(fault)]);
                    return json_response(
                        StatusCode::OK,
                        rpc_result(id, serde_json::to_value(refused).expect("the result encodes"), modern),
                    );
                }
                // Long work on the open door takes one of the places for long
                // work first, and finds the door busy when none is free.
                let long_place = match (open, long_work) {
                    (Some(rules), Some(places)) if rules.is_long_work(&name, &arguments) => {
                        match places.try_acquire_owned() {
                            Ok(place) => Some(place),
                            Err(_) => {
                                let busy = rmcp::model::CallToolResult::error(vec![rmcp::model::ContentBlock::text(BUSY)]);
                                return json_response(
                                    StatusCode::OK,
                                    rpc_result(id, serde_json::to_value(busy).expect("the result encodes"), modern),
                                );
                            }
                        }
                    }
                    _ => None,
                };
                // Long work on the open connection runs in a process of its
                // own, which is ended when the time for an answer is up: the
                // work stops there and its permit and its place come back.
                if let (Some(rules), Some(worker), true) = (open, open.and_then(|rules| rules.worker.as_deref()), long_place.is_some()) {
                    // The address's share and one permit of the pool are held
                    // here while the child computes. With none to be had the
                    // child is given an empty pool, and answers as a full
                    // pool answers on every connection.
                    let held = crate::acquire_physics_permit(&state).ok();
                    let ended = crate::long_work::run(worker, &name, arguments, arguments_text, held.is_some(), rules.request).await;
                    drop(held);
                    drop(long_place);
                    drop(permit);
                    let text = match ended {
                        crate::long_work::Ended::Answered(result) => rpc_result(id, result, modern),
                        crate::long_work::Ended::Stopped => {
                            let late = rmcp::model::CallToolResult::error(vec![rmcp::model::ContentBlock::text(rules.stopped_at_the_limit())]);
                            rpc_result(id, serde_json::to_value(late).expect("the result encodes"), modern)
                        }
                        // The child stopped with no result: answered by name, as a
                        // panic in this process is.
                        crate::long_work::Ended::Failed => {
                            let fault = rmcp::model::CallToolResult::error(vec![rmcp::model::ContentBlock::text(mcp_server::internal_fault(&name))]);
                            rpc_result(id, serde_json::to_value(fault).expect("the result encodes"), modern)
                        }
                    };
                    return json_response(StatusCode::OK, text);
                }
                let tool = name.clone();
                let handle = tokio::spawn(async move {
                    // The client's slot is held until the work ends, even if
                    // the caller hangs up.
                    let _slot = permit;
                    let _long_place = long_place;
                    mcp_server::dispatch_text(&state, &name, arguments, arguments_text).await
                });
                let joined = match open {
                    // The open door waits so long for an answer and no
                    // longer. Work that runs in this process is short work,
                    // or the long work of a door given no binary to start: it
                    // is not stopped, and holds its places until it ends.
                    Some(rules) => match tokio::time::timeout(rules.request, handle).await {
                        Ok(joined) => joined,
                        Err(_) => {
                            let late = rmcp::model::CallToolResult::error(vec![rmcp::model::ContentBlock::text(rules.too_slow())]);
                            return json_response(
                                StatusCode::OK,
                                rpc_result(id, serde_json::to_value(late).expect("the result encodes"), modern),
                            );
                        }
                    },
                    None => handle.await,
                };
                match joined {
                    Ok(result) => {
                        rpc_result(id, serde_json::to_value(result).expect("the result encodes"), modern)
                    }
                    // A panic inside the call ended its task alone: the call
                    // is answered by name, and the door goes on serving.
                    Err(_) => {
                        let fault = rmcp::model::CallToolResult::error(vec![rmcp::model::ContentBlock::text(mcp_server::internal_fault(&tool))]);
                        rpc_result(id, serde_json::to_value(fault).expect("the result encodes"), modern)
                    }
                }
            }
            Err(text) => rpc_error(id, -32602, &text),
        },
        // Revision 2026-07-28 answers a method the server does not have with
        // `404` and this error; the revisions before it, with the error alone.
        _ if modern => return json_response(StatusCode::NOT_FOUND, rpc_error(id, -32601, "Method not found")),
        _ => rpc_error(id, -32601, "Method not found"),
    };
    json_response(StatusCode::OK, text)
}

fn tool_call(params: Option<&Value>) -> Result<(String, Map<String, Value>), String> {
    let invalid = || "Invalid params: tools/call needs a 'name' string and an 'arguments' object.".to_string();
    let object = params.and_then(Value::as_object).ok_or_else(invalid)?;
    let name = object.get("name").and_then(Value::as_str).ok_or_else(invalid)?;
    let arguments = match object.get("arguments") {
        None | Some(Value::Null) => Map::new(),
        Some(Value::Object(map)) => map.clone(),
        Some(_) => return Err(invalid()),
    };
    Ok((name.to_string(), arguments))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn keys() -> Keys {
        let entry = |label: &str, key: &str, cards: Value| {
            json!({
                "client": label,
                "key_sha256": keys::sha256_hex(key.as_bytes()),
                "cards": cards,
                "solvers": [],
                "max_in_flight": 4,
                "max_permits": 2
            })
        };
        let file = json!([entry("thermal", "k-thermal", json!([["energy", "thermal"]]))]);
        Keys::parse(file.to_string().as_bytes()).expect("keys")
    }

    async fn serve_in_process(base: AppState) -> String {
        let app = router_over(base, keys(), None, Vec::new()).expect("router");
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
        format!("http://127.0.0.1:{port}/mcp")
    }

    async fn post(url: &str, key: Option<&str>, body: &str) -> (u16, String) {
        let mut request = reqwest::Client::new()
            .post(url)
            .header("Content-Type", "application/json")
            .body(body.to_string());
        if let Some(key) = key {
            request = request.header("Authorization", format!("Bearer {key}"));
        }
        let response = request.send().await.unwrap();
        (response.status().as_u16(), response.text().await.unwrap())
    }

    fn step(domain: &str, card: &str) -> String {
        json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"verify_step","arguments":{
            "domain": domain, "card": card,
            "parameters": {"temperature_k": 293.15, "heat_power_w": 1.0, "ambient_k": 293.15, "dt_s": 0.1}
        }}})
        .to_string()
    }

    /// An `Mcp-Name` header is the name itself, or its Base64 between
    /// `=?base64?` and `?=`, as the specification's own examples write it.
    #[test]
    fn a_name_header_is_read_plain_or_from_base64() {
        assert_eq!(header_value("verify_step").as_deref(), Some("verify_step"));
        assert_eq!(header_value("=?base64?dmVyaWZ5X3N0ZXA=?=").as_deref(), Some("verify_step"));
        assert_eq!(header_value("=?base64?SGVsbG8sIOS4lueVjA==?=").as_deref(), Some("Hello, 世界"));
        assert_eq!(header_value("=?base64?IHBhZGRlZCA=?=").as_deref(), Some(" padded "));
        assert_eq!(header_value("=?base64?bGluZTEKbGluZTI=?=").as_deref(), Some("line1\nline2"));
        assert_eq!(header_value("=?base64?PT9iYXNlNjQ/bGl0ZXJhbD89?=").as_deref(), Some("=?base64?literal?="));
        // Without its padding it is read the same; what is not Base64, or not text, is not read.
        assert_eq!(header_value("=?base64?dmVyaWZ5X3N0ZXA?=").as_deref(), Some("verify_step"));
        for bad in ["=?base64?dmVyaWZ5*3N0ZXA=?=", "=?base64?d?=", "=?base64?dmVy====?=", "=?base64?/w==?="] {
            assert_eq!(header_value(bad), None, "{bad}");
        }
        // The markers are lowercase; anything else is the name as written.
        assert_eq!(header_value("=?BASE64?dmVyaWZ5X3N0ZXA=?=").as_deref(), Some("=?BASE64?dmVyaWZ5X3N0ZXA=?="));
    }

    /// The revisions are one list, and a revision begins with `initialize`
    /// or does not by its date.
    #[test]
    fn the_revisions_are_one_list() {
        assert_eq!(SUPPORTED_VERSIONS, crate::mcp_server::SUPPORTED_VERSIONS);
        assert!(SUPPORTED_VERSIONS.contains(&ASSUMED_VERSION) && SUPPORTED_VERSIONS.contains(&FIRST_VERSION_WITHOUT_INITIALIZE));
        assert!(SUPPORTED_VERSIONS.windows(2).all(|pair| pair[0] < pair[1]), "oldest first");
        let with: Vec<&str> = SUPPORTED_VERSIONS.iter().copied().filter(|version| begins_with_initialize(version)).collect();
        assert_eq!(with, ["2024-11-05", "2025-03-26", "2025-06-18", "2025-11-25"]);
        // The library the local connection is built on is given the same list.
        let library: Vec<String> = McpGateway::new().supported_protocol_versions().iter().map(|version| version.to_string()).collect();
        assert_eq!(library, SUPPORTED_VERSIONS);
        assert!(speaks("2026-07-28") && !speaks("2031-01-01") && !speaks(""));
    }

    /// A request with no known key takes no physics permit, and no permit is
    /// needed to be told so: with every permit held it is still a 401. A hidden
    /// card takes none either, and answers as an unknown card, not as a busy door.
    #[tokio::test]
    async fn a_refused_key_and_a_hidden_card_take_no_physics_permit() {
        let base = AppState::new(2);
        let url = serve_in_process(base.clone()).await;
        let pool = Arc::clone(&base.physics_permits);
        assert_eq!(pool.available_permits(), 2);

        for key in [None, Some("wrong"), Some("k-therma")] {
            let (status, body) = post(&url, key, &step("energy", "thermal")).await;
            assert_eq!((status, body.as_str()), (401, UNAUTHORIZED_BODY));
        }
        assert_eq!(pool.available_permits(), 2);

        let (status, body) = post(&url, Some("k-thermal"), &step("materials", "ti64_lattice")).await;
        assert_eq!(status, 200);
        assert!(body.contains("Card not recognized (UnknownCard)."), "{body}");
        let (_, unknown) = post(&url, Some("k-thermal"), &step("materials", "ti64_latticX")).await;
        assert_eq!(body, unknown);
        assert_eq!(pool.available_permits(), 2);

        // Hold the whole pool. A bad key is still 401. A hidden card is still
        // unknown. A visible card is the busy door the other doors give.
        let held: Vec<_> = (0..2)
            .map(|_| Arc::clone(&pool).try_acquire_owned().unwrap())
            .collect();
        assert_eq!(post(&url, None, &step("energy", "thermal")).await.0, 401);
        let (_, hidden) = post(&url, Some("k-thermal"), &step("materials", "ti64_lattice")).await;
        assert!(hidden.contains("Card not recognized (UnknownCard)."), "{hidden}");
        let (status, busy) = post(&url, Some("k-thermal"), &step("energy", "thermal")).await;
        assert_eq!(status, 200);
        assert!(busy.contains("Too many concurrent physics tasks."), "{busy}");
        drop(held);
        let (_, ran) = post(&url, Some("k-thermal"), &step("energy", "thermal")).await;
        assert!(ran.contains("sha256_seal"), "{ran}");
        assert_eq!(pool.available_permits(), 2);
    }
}
