//! Makes a key for a client of the hosted door: 32 random bytes from the
//! operating system, as 64 hex digits, and the SHA-256 of that text. The key goes
//! to the client. The hash goes into the keys file as `key_sha256`. The key is
//! printed once and stored nowhere.

use std::io::Read;

use protocol_mcp_gateway::keys::sha256_hex;

fn main() {
    if std::env::args().len() > 1 {
        eprintln!("hosted_door_key takes no argument. It prints a new key and its SHA-256.");
        std::process::exit(2);
    }
    let mut bytes = [0_u8; 32];
    let read = std::fs::File::open("/dev/urandom").and_then(|mut source| source.read_exact(&mut bytes));
    if let Err(error) = read {
        eprintln!("hosted_door_key could not read random bytes from the operating system: {error}.");
        std::process::exit(1);
    }
    let key = hex::encode(bytes);
    println!("key (give it to the client, shown once): {key}");
    println!("key_sha256 (put it in the keys file):    {}", sha256_hex(key.as_bytes()));
}
