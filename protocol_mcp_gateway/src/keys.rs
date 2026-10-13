//! The clients of the hosted door: a label, the SHA-256 of a key, the cards
//! and solvers the key may see, a cap on requests in flight, and a share of
//! the physics pool.
//!
//! The keys file holds hashes and never keys. It is read once, at start, and
//! a fault in it is a sentence that names the client and the field.

use std::collections::BTreeSet;
use std::path::Path;
use std::sync::Arc;

use serde_json::Value;
use sha2::{Digest, Sha256};
use tokio::sync::Semaphore;

use crate::json_text::{self, Scan};
use crate::view::View;
use crate::{descriptor, solvers};

/// The longest key the door looks at. A longer token is not a key.
const KEY_BYTES_AT_MOST: usize = 256;
const LABEL_BYTES_AT_MOST: usize = 64;
/// The most `max_in_flight` or `max_permits` a keys file may state.
pub const COUNT_AT_MOST: u64 = 1_000_000;

pub struct Client {
    pub label: String,
    key_sha256: [u8; 32],
    pub view: Arc<View>,
    /// The folder under the dataset directory that holds this client's
    /// datasets: the SHA-256 of the client's `key_sha256` text, in lowercase
    /// hex. It is not the label.
    pub folder: String,
    pub max_in_flight: usize,
    pub in_flight: Arc<Semaphore>,
    /// Bodies being received at once, as many as `max_in_flight`. A body in
    /// the course of arriving holds one of these and no slot of `in_flight`.
    pub receiving: Arc<Semaphore>,
    /// How many physics permits this client may hold at once: a run or a
    /// solve holds one, a dataset job one for each worker.
    pub max_permits: usize,
    /// The client's share of the physics pool, with `max_permits` places.
    pub permit_share: Arc<Semaphore>,
}

pub struct Keys {
    clients: Vec<Client>,
}

pub fn sha256_hex(bytes: &[u8]) -> String {
    hex::encode(Sha256::digest(bytes))
}

impl Keys {
    pub fn load(path: &Path) -> Result<Keys, String> {
        let named = path.display();
        let bytes = std::fs::read(path)
            .map_err(|error| format!("the keys file {named} cannot be read: {error}."))?;
        Keys::parse(&bytes).map_err(|sentence| format!("the keys file {named} is refused: {sentence}"))
    }

    pub fn parse(bytes: &[u8]) -> Result<Keys, String> {
        match json_text::scan(bytes) {
            Scan::Repeated(key) => return Err(json_text::repeated_message(&key)),
            Scan::Unreadable(reason) => {
                return Err(format!("it is not one JSON value: {reason}."));
            }
            Scan::ReadsAsZero { field, token } => {
                return Err(json_text::reads_as_zero_message(&field, &token));
            }
            Scan::Clean => {}
        }
        let value: Value = serde_json::from_slice(bytes)
            .map_err(|error| format!("it is not one JSON value: {error}."))?;
        let list = value
            .as_array()
            .ok_or_else(|| "it must be a JSON list of clients.".to_string())?;
        if list.is_empty() {
            return Err("it names no client.".to_string());
        }
        let mut clients: Vec<Client> = Vec::with_capacity(list.len());
        let mut labels = BTreeSet::new();
        for (index, entry) in list.iter().enumerate() {
            let client = read_client(index, entry)?;
            if !labels.insert(client.label.clone()) {
                return Err(format!("the client label '{}' is used twice.", client.label));
            }
            if let Some(other) = clients.iter().find(|other| other.key_sha256 == client.key_sha256)
            {
                return Err(format!(
                    "the clients '{}' and '{}' have the same key_sha256.",
                    other.label, client.label
                ));
            }
            clients.push(client);
        }
        Ok(Keys { clients })
    }

    pub fn clients(&self) -> &[Client] {
        &self.clients
    }

    /// The client whose key this is. The key is hashed and the hash is
    /// compared with every entry, and every comparison reads all 32 bytes, so
    /// the time taken does not say which entry matched or how much of a hash
    /// agreed.
    pub fn authenticate(&self, key: &[u8]) -> Option<&Client> {
        let digest: [u8; 32] = Sha256::digest(key).into();
        let mut found = usize::MAX;
        for (index, client) in self.clients.iter().enumerate() {
            let mut difference = 0_u8;
            for (left, right) in client.key_sha256.iter().zip(digest.iter()) {
                difference |= left ^ right;
            }
            // 0xFF...F when equal, zero when not, with no branch on it.
            let equal = usize::from(difference == 0).wrapping_neg();
            found = (index & equal) | (found & !equal);
        }
        self.clients.get(found)
    }
}

/// The key in an `Authorization: Bearer <key>` header value, or nothing for
/// any other shape.
pub fn bearer_key(value: &[u8]) -> Option<&[u8]> {
    let rest = value.get(7..)?;
    if !value[..7].eq_ignore_ascii_case(b"bearer ") {
        return None;
    }
    if rest.is_empty()
        || rest.len() > KEY_BYTES_AT_MOST
        || !rest.iter().all(|byte| byte.is_ascii_graphic())
    {
        return None;
    }
    Some(rest)
}

const FIELDS: [&str; 6] = ["client", "key_sha256", "cards", "solvers", "max_in_flight", "max_permits"];

fn read_client(index: usize, entry: &Value) -> Result<Client, String> {
    let object = entry
        .as_object()
        .ok_or_else(|| format!("entry {index} must be a JSON object."))?;
    let label = match object.get("client") {
        Some(Value::String(text))
            if !text.is_empty()
                && text.len() <= LABEL_BYTES_AT_MOST
                && text.bytes().all(|byte| byte.is_ascii_graphic() || byte == b' ') =>
        {
            text.clone()
        }
        _ => {
            return Err(format!(
                "entry {index}: 'client' must be a label of 1 to {LABEL_BYTES_AT_MOST} printable ASCII characters."
            ));
        }
    };
    for key in object.keys() {
        if !FIELDS.contains(&key.as_str()) {
            return Err(format!("client '{label}': '{key}' is not a field of a client."));
        }
    }
    for field in FIELDS {
        if !object.contains_key(field) {
            return Err(format!("client '{label}': '{field}' is missing."));
        }
    }
    let hash_text = object["key_sha256"].as_str().unwrap_or("");
    if !json_text::is_lower_hex(hash_text, 64) {
        return Err(format!(
            "client '{label}': 'key_sha256' must be 64 lowercase hex digits."
        ));
    }
    let mut key_sha256 = [0_u8; 32];
    hex::decode_to_slice(hash_text, &mut key_sha256).expect("64 hex digits");

    let cards = object["cards"]
        .as_array()
        .ok_or_else(|| format!("client '{label}': 'cards' must be a list of [domain, card] pairs."))?;
    let mut pairs: Vec<(String, String)> = Vec::new();
    for item in cards {
        let pair = item
            .as_array()
            .filter(|pair| pair.len() == 2)
            .and_then(|pair| Some((pair[0].as_str()?, pair[1].as_str()?)))
            .ok_or_else(|| {
                format!("client '{label}': each entry of 'cards' must be [domain, card], two strings.")
            })?;
        if descriptor::find(pair.0, pair.1).is_none() {
            return Err(format!(
                "client '{label}': the engine does not serve the card [\"{}\", \"{}\"].",
                pair.0, pair.1
            ));
        }
        let pair = (pair.0.to_string(), pair.1.to_string());
        if pairs.contains(&pair) {
            return Err(format!(
                "client '{label}': the card [\"{}\", \"{}\"] is listed twice.",
                pair.0, pair.1
            ));
        }
        pairs.push(pair);
    }
    let named = object["solvers"]
        .as_array()
        .ok_or_else(|| format!("client '{label}': 'solvers' must be a list of solver names."))?;
    let mut solver_names: Vec<String> = Vec::new();
    for item in named {
        let name = item
            .as_str()
            .ok_or_else(|| format!("client '{label}': each entry of 'solvers' must be a string."))?;
        if !solvers::is_known(name) {
            return Err(format!(
                "client '{label}': the engine does not serve the solver \"{name}\"."
            ));
        }
        if solver_names.iter().any(|other| other == name) {
            return Err(format!("client '{label}': the solver \"{name}\" is listed twice."));
        }
        solver_names.push(name.to_string());
    }
    let count = |field: &str| -> Result<usize, String> {
        object[field]
            .as_u64()
            .filter(|count| (1..=COUNT_AT_MOST).contains(count))
            .map(|count| count as usize)
            .ok_or_else(|| {
                format!("client '{label}': '{field}' must be a whole number from 1 to {COUNT_AT_MOST}.")
            })
    };
    let max_in_flight = count("max_in_flight")?;
    let max_permits = count("max_permits")?;
    Ok(Client {
        folder: sha256_hex(hash_text.as_bytes()),
        view: Arc::new(View::limited(pairs, solver_names)),
        in_flight: Arc::new(Semaphore::new(max_in_flight)),
        receiving: Arc::new(Semaphore::new(max_in_flight)),
        max_in_flight,
        permit_share: Arc::new(Semaphore::new(max_permits)),
        max_permits,
        key_sha256,
        label,
    })
}
