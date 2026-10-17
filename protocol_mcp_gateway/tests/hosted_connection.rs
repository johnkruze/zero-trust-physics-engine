//! The limits of a connection to the hosted door (time to receive the
//! headers, time to receive the body, idle time, connections open) and what the
//! door and its HTTP library answer to a bad key header. Real sockets; the
//! door runs in this process with short limits so a test need not wait the
//! time the door waits. The values in force are in `Limits::DEFAULT`.

mod hosted_support;

use std::io::{Read, Write};
use std::net::TcpStream;
use std::time::{Duration, Instant};

use hosted_support::*;
use protocol_mcp_gateway::hosted::{self, Limits};
use protocol_mcp_gateway::keys::Keys;
use serde_json::json;

const KEY: &str = "key-of-the-only-client";

struct Running {
    port: u16,
    _runtime: tokio::runtime::Runtime,
}

fn run_door(limits: Limits, max_in_flight: u64) -> Running {
    run_door_sharing(limits, max_in_flight, 1)
}

fn run_door_sharing(limits: Limits, max_in_flight: u64, max_permits: u64) -> Running {
    let keys = Keys::parse(json!([client_entry("a", KEY, max_in_flight, max_permits)]).to_string().as_bytes()).unwrap();
    let app = hosted::router_limited(protocol_mcp_gateway::new_gateway_state(), keys, None, Vec::new(), limits).unwrap();
    let runtime = tokio::runtime::Builder::new_multi_thread().worker_threads(2).enable_all().build().unwrap();
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let port = listener.local_addr().unwrap().port();
    runtime.spawn(async move {
        hosted::serve_listener(tokio::net::TcpListener::from_std(listener).unwrap(), app, limits).await;
    });
    Running { port, _runtime: runtime }
}

fn short() -> Limits {
    Limits {
        header_read: Duration::from_millis(800),
        body_read: Duration::from_millis(800),
        idle_keep_alive: Duration::from_millis(800),
        max_connections: 4,
        max_header_bytes: 65_536,
    }
}

/// Wait for the other side to close, up to `seconds`. The bytes it sent first, and how long it took.
fn until_closed(stream: &mut TcpStream, seconds: u64) -> (Vec<u8>, Duration, bool) {
    stream.set_read_timeout(Some(Duration::from_secs(seconds))).unwrap();
    let start = Instant::now();
    let mut out = Vec::new();
    let mut buffer = [0_u8; 4096];
    loop {
        match stream.read(&mut buffer) {
            Ok(0) => return (out, start.elapsed(), true),
            Ok(count) => out.extend_from_slice(&buffer[..count]),
            Err(error) if error.kind() == std::io::ErrorKind::ConnectionReset => return (out, start.elapsed(), true),
            Err(_) => return (out, start.elapsed(), false),
        }
    }
}

const PING: &str = r#"{"jsonrpc":"2.0","id":1,"method":"ping"}"#;

fn partial_body_head(body_len: usize) -> String {
    format!("POST /mcp HTTP/1.1\r\nHost: x\r\nAuthorization: Bearer {KEY}\r\nContent-Type: application/json\r\nContent-Length: {body_len}\r\n\r\n")
}

#[test]
fn headers_that_do_not_arrive_in_time_close_the_connection() {
    let door = run_door(short(), 2);
    // A connection that sends nothing, and one that sends half a header.
    let mut silent = TcpStream::connect(("127.0.0.1", door.port)).unwrap();
    let mut half = TcpStream::connect(("127.0.0.1", door.port)).unwrap();
    half.write_all(b"POST /mcp HTTP/1.1\r\nHost: x\r\nAuthor").unwrap();
    for stream in [&mut silent, &mut half] {
        let (bytes, took, closed) = until_closed(stream, 10);
        assert!(closed, "the door left the connection open");
        assert!(bytes.is_empty(), "{}", String::from_utf8_lossy(&bytes));
        assert!(took < Duration::from_secs(5), "{took:?}");
    }
    // The door is not harmed.
    assert_eq!(parse_reply(&raw(door.port, &request_bytes(Some(KEY), PING), 10)).status, 200);
}

#[test]
fn a_body_that_does_not_arrive_in_time_is_cut_off_and_holds_none_of_the_clients_slots() {
    let pool = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(16).clamp(1, 32);
    if pool < 2 {
        eprintln!("skipped: two runs at once need a pool of two permits");
        return;
    }
    let door = run_door_sharing(short(), 2, 2);
    // One request is running (it holds one of the two slots) ...
    let long = tool_body(1, "run_trajectory", &thermal_run(1_000_000, 293.15));
    let runner = {
        let port = door.port;
        std::thread::spawn(move || parse_reply(&raw(port, &request_bytes(Some(KEY), &long), 60)))
    };
    std::thread::sleep(Duration::from_millis(100));
    // ... and a body stalls half way ...
    let mut stalled = TcpStream::connect(("127.0.0.1", door.port)).unwrap();
    stalled.write_all(partial_body_head(100).as_bytes()).unwrap();
    stalled.write_all(b"{\"jsonrpc\"").unwrap();
    std::thread::sleep(Duration::from_millis(200));
    // ... yet the other slot still takes a request: the stalled body holds no slot.
    let second = tool_body(2, "run_trajectory", &thermal_run(1_000_000, 294.15));
    let second = std::thread::spawn({
        let port = door.port;
        move || parse_reply(&raw(port, &request_bytes(Some(KEY), &second), 60))
    });
    let (bytes, took, closed) = until_closed(&mut stalled, 10);
    let reply = parse_reply(&bytes);
    assert!(closed && reply.status == 408, "{reply:?}");
    assert!(reply.body.contains("the body did not arrive in time"));
    assert!(took < Duration::from_secs(5), "{took:?}");
    for reply in [runner.join().unwrap(), second.join().unwrap()] {
        assert_eq!(reply.status, 200, "{reply:?}");
        assert!(!tool_result(&reply).0, "a run was refused: {}", reply.body);
    }
}

#[test]
fn a_client_with_one_slot_is_not_held_for_ever_by_a_stalled_body() {
    let door = run_door(short(), 1);
    let mut stalled = TcpStream::connect(("127.0.0.1", door.port)).unwrap();
    stalled.write_all(partial_body_head(100).as_bytes()).unwrap();
    stalled.write_all(b"{").unwrap();
    std::thread::sleep(Duration::from_millis(200));
    // While it stalls, the client's single place for a body is taken: busy, as at the cap.
    let during = parse_reply(&raw(door.port, &request_bytes(Some(KEY), PING), 10));
    assert_eq!((during.status, during.body.as_str()), (429, "Too many concurrent physics tasks."));
    let (_, _, closed) = until_closed(&mut stalled, 10);
    assert!(closed);
    let after = parse_reply(&raw(door.port, &request_bytes(Some(KEY), PING), 10));
    assert_eq!(after.status, 200, "{after:?}");
}

#[test]
fn a_kept_alive_connection_that_goes_quiet_is_closed() {
    let door = run_door(short(), 2);
    let mut stream = TcpStream::connect(("127.0.0.1", door.port)).unwrap();
    let keep = format!("POST /mcp HTTP/1.1\r\nHost: x\r\nAuthorization: Bearer {KEY}\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{PING}", PING.len());
    stream.write_all(keep.as_bytes()).unwrap();
    stream.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
    let mut buffer = [0_u8; 4096];
    let count = stream.read(&mut buffer).unwrap();
    assert!(String::from_utf8_lossy(&buffer[..count]).starts_with("HTTP/1.1 200"));
    // The same connection serves a second request while it is young ...
    stream.write_all(keep.as_bytes()).unwrap();
    let count = stream.read(&mut buffer).unwrap();
    assert!(String::from_utf8_lossy(&buffer[..count]).starts_with("HTTP/1.1 200"));
    // ... and is closed when it sits idle.
    let (bytes, took, closed) = until_closed(&mut stream, 10);
    assert!(closed && bytes.is_empty(), "{}", String::from_utf8_lossy(&bytes));
    assert!(took < Duration::from_secs(5), "{took:?}");
}

#[test]
fn connections_over_the_cap_are_closed_at_once_and_the_door_recovers() {
    let door = run_door(short(), 2);
    // Four idle connections fill the cap of four.
    let mut idle: Vec<TcpStream> = (0..4).map(|_| TcpStream::connect(("127.0.0.1", door.port)).unwrap()).collect();
    std::thread::sleep(Duration::from_millis(200));
    // The fifth is accepted and closed at once, with nothing said.
    let mut fifth = TcpStream::connect(("127.0.0.1", door.port)).unwrap();
    let (bytes, took, closed) = until_closed(&mut fifth, 5);
    assert!(closed && bytes.is_empty() && took < Duration::from_millis(600), "{took:?} closed={closed}");
    // The idle ones are timed out, and then a request is answered.
    for stream in &mut idle {
        let (_, _, closed) = until_closed(stream, 10);
        assert!(closed);
    }
    assert_eq!(parse_reply(&raw(door.port, &request_bytes(Some(KEY), PING), 10)).status, 200);
}

/// A map of this crate, where it is on the disk. The maps are beside the
/// code and are not in its Git history, so a checkout of the code alone has
/// none: the pages are held to the code where they are present.
fn pages(names: &[&str]) -> Vec<String> {
    names
        .iter()
        .filter_map(|name| std::fs::read_to_string(format!("{}/{name}", env!("CARGO_MANIFEST_DIR"))).ok())
        .collect()
}

#[test]
fn the_limits_in_force_are_the_ones_the_pages_state() {
    let defaults = Limits::DEFAULT;
    assert_eq!(
        (defaults.header_read.as_secs(), defaults.body_read.as_secs(), defaults.idle_keep_alive.as_secs(), defaults.max_connections, defaults.max_header_bytes),
        (30, 30, 30, 256, 65_536)
    );
    for page in pages(&["AGENTS.md", "HOSTING.md"]) {
        for stated in [
            "HEADER_READ_SECONDS` = 30",
            "BODY_READ_SECONDS` = 30",
            "IDLE_KEEP_ALIVE_SECONDS` = 30",
            "MAX_OPEN_CONNECTIONS` = 256",
            "MAX_HEADER_BYTES` = 65536",
        ] {
            assert!(page.contains(stated), "a page does not state {stated}");
        }
    }
}

// ------------------------------------------------------------ item 5

#[test]
fn what_the_door_and_its_library_answer_to_a_key_header_is_what_the_pages_state() {
    let door = run_door(Limits::DEFAULT, 2);
    let send = |auth: &[u8]| -> Reply {
        let mut bytes = b"POST /mcp HTTP/1.1\r\nHost: x\r\nConnection: close\r\nContent-Type: application/json\r\nAuthorization: ".to_vec();
        bytes.extend_from_slice(auth);
        bytes.extend_from_slice(format!("\r\nContent-Length: {}\r\n\r\n{PING}", PING.len()).as_bytes());
        parse_reply(&raw(door.port, &bytes, 10))
    };
    let unauthorized = |reply: &Reply| reply.status == 401 && reply.body == r#"{"error":"unauthorized"}"# && reply.head.contains("www-authenticate: Bearer");
    // The door's own: one fixed 401.
    for bad in [
        "Bearer wrong".to_string(),
        "Basic dXNlcjpwYXNz".to_string(),
        "Bearer".to_string(),
        "Bearer ".to_string(),
        format!("Bearer {KEY}x"),
        format!("Bearer  {KEY}"),
        format!("Bearer {}", "a".repeat(257)),
        format!("Bearer {}", "a".repeat(40_000)),
        format!("Token {KEY}"),
    ] {
        let reply = send(bad.as_bytes());
        assert!(unauthorized(&reply), "{bad:.30}: {reply:?}");
    }
    // The scheme is case-insensitive (RFC 9110), so a key after `bearer` or `BEARER` is accepted.
    for scheme in ["Bearer", "bearer", "BEARER"] {
        assert_eq!(send(format!("{scheme} {KEY}").as_bytes()).status, 200, "{scheme}");
    }
    // The library answers before the door is reached: a control byte in the value.
    for bytes in [&b"Bearer ab\0cd"[..], &b"Bearer ab\rcd"[..], &b"Bearer ab\x01cd"[..]] {
        let reply = send(bytes);
        assert_eq!((reply.status, reply.body.as_str()), (400, ""), "{reply:?}");
    }
    // A header section that does not fit the library's buffer: 431 with no body. The buffer is set by
    // MAX_HEADER_BYTES (65,536). The library stops reading once what it holds reaches the buffer, and
    // its last read may bring up to one buffer more, as many bytes as the socket has delivered by then,
    // so the first length refused depends on how the bytes arrive (measured once at 114,563). What
    // holds whatever the delivery: a header section that fits the buffer is read, and one of twice the
    // buffer or more is refused.
    let buffer = protocol_mcp_gateway::hosted::MAX_HEADER_BYTES;
    let reply = send(format!("Bearer {}", "a".repeat(1 << 20)).as_bytes());
    assert_eq!((reply.status, reply.body.as_str()), (431, ""), "{reply:?}");
    let reply = send(format!("Bearer {}", "a".repeat(2 * buffer)).as_bytes());
    assert_eq!((reply.status, reply.body.as_str()), (431, ""), "{reply:?}");
    // Inside the buffer the door reads the value, and a key that long is just a wrong key.
    let reply = send(format!("Bearer {}", "a".repeat(buffer - 1024)).as_bytes());
    assert!(unauthorized(&reply), "{reply:?}");
    // Between the two, the answer is one of those two, never a third.
    for length in [buffer + 1, 114_562, 114_563, 2 * buffer - 1024] {
        let reply = send(format!("Bearer {}", "a".repeat(length)).as_bytes());
        assert!(unauthorized(&reply) || (reply.status, reply.body.as_str()) == (431, ""), "{length}: {reply:?}");
    }
    // Spaces or a tab around the value are stripped by the library; the key still has to be right.
    assert_eq!(send(format!("Bearer {KEY} ").as_bytes()).status, 200);
    assert_eq!(send(format!("Bearer {KEY}\t").as_bytes()).status, 200);
}
