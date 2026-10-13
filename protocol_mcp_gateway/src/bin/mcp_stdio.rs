use std::sync::Arc;

use rmcp::ServiceExt;
use tokio::io::{AsyncBufRead, AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::sync::Mutex;

use protocol_mcp_gateway::guard::{self, Inspection, LINE_LIMIT};
use protocol_mcp_gateway::mcp_server::McpGateway;

/// The largest trajectory body the HTTP door accepts, with room for the
/// JSON-RPC envelope around it. A larger line waits until the server reads.
const PIPE_BYTES: usize = 32 * 1024 * 1024;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let dataset_dir = match protocol_mcp_gateway::dataset_dir_from_args(std::env::args().skip(1)) {
        Ok(dir) => dir,
        Err(sentence) => {
            eprintln!("The gateway did not start: {sentence}");
            std::process::exit(2);
        }
    };
    let (client_write, server_read) = tokio::io::duplex(PIPE_BYTES);
    let (client_read, server_write) = tokio::io::duplex(PIPE_BYTES);
    let stdout = Arc::new(Mutex::new(tokio::io::stdout()));
    let stdout_for_copy = Arc::clone(&stdout);

    let server = tokio::spawn(async move {
        let running = McpGateway::with_dataset_dir(dataset_dir)
            .serve((server_read, server_write))
            .await?;
        running.waiting().await?;
        Ok::<(), Box<dyn std::error::Error + Send + Sync>>(())
    });

    let copy = tokio::spawn(async move {
        let mut reader = BufReader::new(client_read);
        let mut line = Vec::new();
        loop {
            line.clear();
            let read = reader.read_until(b'\n', &mut line).await?;
            if read == 0 {
                break;
            }
            let mut out = stdout_for_copy.lock().await;
            // An error the library wrote for a message it could not read has
            // no id member. Every error answer carries one.
            match guard::complete_id(&line) {
                Some(completed) => out.write_all(&completed).await?,
                None => out.write_all(&line).await?,
            }
            out.flush().await?;
        }
        Ok::<(), std::io::Error>(())
    });

    let mut stdin = BufReader::new(tokio::io::stdin());
    let mut client_write = client_write;
    let mut line = Vec::new();
    loop {
        line.clear();
        match read_capped(&mut stdin, &mut line, LINE_LIMIT).await? {
            LineRead::End => break,
            LineRead::TooLong => {
                let response = guard::too_long_response(&line);
                write_line(&stdout, &response).await?;
                continue;
            }
            LineRead::Line => {}
        }
        match guard::inspect(&line) {
            // A blank line is not a message. The library skips it and waits
            // for the next line, so the blank line is forwarded as it arrived.
            Inspection::Forward => {
                // A tools/call with wrong parameters is -32602, not an unknown method.
                if let Some(response) = guard::tool_call_fault(&line) {
                    write_line(&stdout, &response).await?;
                    continue;
                }
                client_write.write_all(&line).await?;
                if !line.ends_with(b"\n") {
                    client_write.write_all(b"\n").await?;
                }
                client_write.flush().await?;
            }
            Inspection::Blank => {
                client_write.write_all(&line).await?;
                if !line.ends_with(b"\n") {
                    client_write.write_all(b"\n").await?;
                }
                client_write.flush().await?;
            }
            Inspection::Answer(response) => write_line(&stdout, &response).await?,
            Inspection::NoAnswer => {}
        }
    }
    drop(client_write);
    server.await??;
    // The server has dropped its write end. Drain whatever it still wrote.
    let _ = copy.await?;
    Ok(())
}

async fn write_line(
    stdout: &Mutex<tokio::io::Stdout>,
    text: &str,
) -> Result<(), std::io::Error> {
    let mut out = stdout.lock().await;
    out.write_all(text.as_bytes()).await?;
    out.write_all(b"\n").await?;
    out.flush().await
}

enum LineRead {
    /// The input ended with no more bytes.
    End,
    /// One line, with its line break when it had one.
    Line,
    /// The line held more than the limit. The buffer holds its first `limit`
    /// bytes; the rest was read through to the line break and dropped.
    TooLong,
}

/// One line, never holding more than `limit` bytes of it (the line break
/// is not counted).
async fn read_capped<R: AsyncBufRead + Unpin>(
    reader: &mut R,
    line: &mut Vec<u8>,
    limit: usize,
) -> std::io::Result<LineRead> {
    let mut content = 0_usize;
    let mut over = false;
    let mut any = false;
    loop {
        let chunk = reader.fill_buf().await?;
        if chunk.is_empty() {
            return Ok(if over {
                LineRead::TooLong
            } else if any {
                LineRead::Line
            } else {
                LineRead::End
            });
        }
        any = true;
        let found = chunk.iter().position(|&byte| byte == b'\n');
        let segment = found.unwrap_or(chunk.len());
        if !over {
            let room = limit - content;
            if segment <= room {
                line.extend_from_slice(&chunk[..segment]);
                content += segment;
                if found.is_some() {
                    line.push(b'\n');
                }
            } else {
                line.extend_from_slice(&chunk[..room]);
                over = true;
            }
        }
        let used = if found.is_some() { segment + 1 } else { segment };
        reader.consume(used);
        if found.is_some() {
            return Ok(if over { LineRead::TooLong } else { LineRead::Line });
        }
    }
}
