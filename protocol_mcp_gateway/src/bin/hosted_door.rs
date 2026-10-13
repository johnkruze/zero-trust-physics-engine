//! The hosted door: MCP over HTTP at `POST /mcp`, a key per client.
//! It listens on 127.0.0.1 only. TLS is ended in front of it.

use protocol_mcp_gateway::hosted::{self, Options};

#[tokio::main]
async fn main() {
    // Started by the open connection itself to make one long tool call in a
    // process that can be ended at the time limit; not an operator's argument.
    if std::env::args().nth(1).as_deref() == Some(protocol_mcp_gateway::long_work::ONE_CALL_FLAG) {
        std::process::exit(protocol_mcp_gateway::long_work::one_call().await);
    }
    let options = match Options::from_args(std::env::args().skip(1)) {
        Ok(options) => options,
        Err(sentence) => {
            eprintln!("The hosted door did not start: {sentence}");
            std::process::exit(2);
        }
    };
    if let Err(sentence) = hosted::serve(options).await {
        eprintln!("The hosted door did not start: {sentence}");
        std::process::exit(2);
    }
}
