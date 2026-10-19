//! The operator's answer key: makes one, or prints its public half.
//!
//! ```text
//! answer_key new <seed file>     a new key from the operating system's randomness,
//!                                written as 64 hex digits, mode 0600; an existing file is refused
//! answer_key public <seed file>  {"alg":"ML-DSA-87","key_id":…,"public_key":<base64>}
//! ```
//!
//! The seed is never printed. The file goes to the program that signs
//! (`--signing-key`, or `ZTP_SIGNING_KEY_FILE`), and is never committed.

use std::io::Write;
use std::path::Path;

use genesis_seal::answer_signature::SigningKey;
use protocol_mcp_gateway::answer_key::read_key_file;

fn main() {
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    let code = match arguments.iter().map(String::as_str).collect::<Vec<_>>().as_slice() {
        ["new", path] => make(Path::new(path)),
        ["public", path] => match read_key_file(Path::new(path)) {
            Ok(key) => {
                println!("{}", key.public_key().description_json());
                Ok(())
            }
            Err(why) => Err(why),
        },
        _ => Err("usage: answer_key new <seed file> | answer_key public <seed file>".to_string()),
    };
    if let Err(why) = code {
        eprintln!("answer_key: {why}");
        std::process::exit(2);
    }
}

fn make(path: &Path) -> Result<(), String> {
    let (key, seed): (SigningKey, String) =
        SigningKey::generate().map_err(|why| format!("no key was made: {why:?}"))?;
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options
        .open(path)
        .map_err(|error| format!("{} was not written: {error} (an existing file is never overwritten)", path.display()))?;
    file.write_all(format!("{seed}\n").as_bytes())
        .and_then(|()| file.sync_all())
        .map_err(|error| format!("{} was not written: {error}", path.display()))?;
    println!("{}", key.public_key().description_json());
    Ok(())
}
