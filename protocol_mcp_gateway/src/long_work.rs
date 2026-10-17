//! Long work on the open connection, in a process of its own.
//!
//! A solve, a dataset job and a long run compute on a thread that nothing can
//! stop: until this module, a call that passed the open connection's time for
//! an answer was answered with an error while its work went on, holding its
//! physics permit and the one place for long work until it ended by itself.
//! One slow request shut every other agent's long work out for as long as it
//! computed.
//!
//! So the open connection runs long work in a child process: the same binary,
//! started with [`ONE_CALL_FLAG`], which reads one tool call on its standard
//! input, makes it with the same function every connection calls, and writes
//! the result on its standard output. The parent waits the connection's time
//! for an answer and no longer. At that time the child is ended, the work
//! ends with it, and the permit and the place come back.
//!
//! **The answer is the same bytes.** The child calls
//! [`crate::mcp_server::dispatch`] on a state that sees every card and
//! solver, as the open connection's does. The arguments go to it as JSON and
//! are read back as the same values: a number keeps its value bit for bit and
//! its kind, whole or not, which is what a receipt follows. A verification's
//! long form also gets the exact text of its arguments as the connection read
//! them, since it holds the answer sent byte for byte.
//!
//! **The permits are counted where they were.** The parent takes the address's
//! share and one permit of the physics pool before the child starts, and holds
//! them while it computes; the child computes on one worker. When the parent
//! finds no permit, the child is started with an empty pool: a request that
//! stops before computing is then answered with its own sentence, and one that
//! reaches the physics is answered `Too many concurrent physics tasks.`, as
//! the same request is answered by a full pool on every other connection.

use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::Arc;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::Semaphore;

use crate::AppState;
use crate::view::View;

/// The argument the hosted connection's binary is started with to make one
/// tool call and exit. It is not an argument an operator gives.
pub const ONE_CALL_FLAG: &str = "--one-call";

/// The permits of the child's own pool: one for its work and one that a
/// dataset job leaves free, as the open connection's smallest pool has.
const CHILD_POOL: usize = 2;

/// One tool call, as the parent hands it to the child.
#[derive(Serialize, Deserialize)]
struct OneCall {
    name: String,
    arguments: Map<String, Value>,
    /// The exact text of the arguments as the connection read them, for a
    /// verification's long form, which holds the answer sent byte for byte.
    #[serde(default)]
    text: Option<String>,
    /// Whether the parent holds a physics permit for this call. Without one
    /// the child's pool is empty.
    permit: bool,
}

/// How a call made in a child ended.
pub(crate) enum Ended {
    /// The tool's result, as the child wrote it.
    Answered(Value),
    /// The answer did not come in the time given. The child was ended.
    Stopped,
    /// The child could not be started, or ended without a result.
    Failed,
}

/// The binary to start for long work: this one. On Linux the path is the
/// running image itself, which is still there after a deploy has put a new
/// file in its place.
pub fn worker() -> Option<PathBuf> {
    let image = Path::new("/proc/self/exe");
    if image.exists() {
        return Some(image.to_path_buf());
    }
    std::env::current_exe().ok()
}

/// Make one tool call in a child process and wait `limit` for its result.
pub(crate) async fn run(worker: &Path, name: &str, arguments: Map<String, Value>, text: Option<String>, permit: bool, limit: Duration) -> Ended {
    let call = OneCall { name: name.to_string(), arguments, text, permit };
    let Ok(text) = serde_json::to_vec(&call) else { return Ended::Failed };
    let spawned = tokio::process::Command::new(worker)
        .arg(ONE_CALL_FLAG)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        // If the agent hangs up, the wait is dropped and the work ends too.
        .kill_on_drop(true)
        .spawn();
    let Ok(mut child) = spawned else { return Ended::Failed };
    let (Some(mut input), Some(mut output)) = (child.stdin.take(), child.stdout.take()) else {
        let _ = child.kill().await;
        return Ended::Failed;
    };
    let exchange = async {
        input.write_all(&text).await?;
        input.shutdown().await?;
        drop(input);
        let mut answer = Vec::new();
        output.read_to_end(&mut answer).await?;
        Ok::<Vec<u8>, std::io::Error>(answer)
    };
    match tokio::time::timeout(limit, exchange).await {
        Ok(Ok(answer)) => {
            let ended_well = child.wait().await.map(|status| status.success()).unwrap_or(false);
            match serde_json::from_slice::<Value>(&answer) {
                Ok(result) if ended_well => Ended::Answered(result),
                _ => Ended::Failed,
            }
        }
        Ok(Err(_)) => {
            let _ = child.kill().await;
            Ended::Failed
        }
        Err(_) => {
            // The work is ended here: the child is killed and waited for.
            let _ = child.kill().await;
            Ended::Stopped
        }
    }
}

/// The child: one tool call read from standard input, its result written to
/// standard output. The exit code is 0 when a result was written.
pub async fn one_call() -> i32 {
    let mut text = Vec::new();
    if tokio::io::stdin().read_to_end(&mut text).await.is_err() {
        return 2;
    }
    let Ok(call) = serde_json::from_slice::<OneCall>(&text) else { return 2 };
    let state = AppState::new(CHILD_POOL)
        .with_view(Arc::new(View::full()))
        .with_share(Arc::new(Semaphore::new(1)));
    // Without a permit of the parent's, nothing here may compute.
    let _held: Vec<_> = if call.permit {
        Vec::new()
    } else {
        (0..CHILD_POOL).filter_map(|_| Arc::clone(&state.physics_permits).try_acquire_owned().ok()).collect()
    };
    // A panic in the call is answered by name, InternalFault, as on every
    // connection: the parent reads that answer like any other.
    let result = crate::mcp_server::dispatch_guarded_text(&state, &call.name, call.arguments, call.text).await;
    let Ok(answer) = serde_json::to_vec(&result) else { return 2 };
    let mut out = tokio::io::stdout();
    if out.write_all(&answer).await.is_err() || out.flush().await.is_err() {
        return 2;
    }
    0
}
