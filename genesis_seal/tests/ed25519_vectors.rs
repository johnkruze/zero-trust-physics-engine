//! The Ed25519 vectors both verifiers are held to: RFC 8032, section 7.1;
//! the twelve cases of "Taming the many EdDSAs" (Chalkias, Garillot and
//! Nikolaenko, 2020); and the engine's own cases over its worked signed
//! reading. `verify_signed.py --vectors` reads the same file and must give
//! the same answer to every one.

use ed25519_dalek::{Signer, SigningKey};
use genesis_seal::attest::verify_strict;

struct Vector {
    name: String,
    key: Vec<u8>,
    message: Vec<u8>,
    signature: Vec<u8>,
    expect: String,
    secret: Option<Vec<u8>>,
}

fn vectors() -> Vec<Vector> {
    let text = include_str!("../vectors/ed25519.txt");
    let mut out = Vec::new();
    for block in text.split("\n\n") {
        let mut fields = std::collections::BTreeMap::new();
        for line in block.lines() {
            if line.starts_with('#') || line.trim().is_empty() {
                continue;
            }
            let (name, value) = line.split_once(':').expect("a line is name: value");
            fields.insert(name.trim().to_string(), value.trim().to_string());
        }
        if fields.is_empty() {
            continue;
        }
        let hex_of = |key: &str| hex::decode(&fields[key]).unwrap_or_else(|_| panic!("{key} is hex"));
        out.push(Vector {
            name: fields["name"].clone(),
            key: hex_of("key"),
            message: hex_of("message"),
            signature: hex_of("signature"),
            expect: fields["expect"].clone(),
            secret: fields.get("secret").map(|text| hex::decode(text).expect("hex")),
        });
    }
    out
}

#[test]
fn every_vector_gives_the_answer_it_states() {
    let all = vectors();
    assert_eq!(all.len(), 35);
    assert_eq!(all.iter().filter(|vector| vector.name.starts_with("rfc8032-")).count(), 5);
    assert_eq!(all.iter().filter(|vector| vector.name.starts_with("taming-")).count(), 12);
    for vector in &all {
        let key: [u8; 32] = vector.key.clone().try_into().expect("32 bytes");
        let signature: [u8; 64] = vector.signature.clone().try_into().expect("64 bytes");
        let got = match verify_strict(&key, &vector.message, &signature) {
            Ok(()) => "holds",
            Err(why) => why.as_str(),
        };
        assert_eq!(got, vector.expect, "{}", vector.name);
    }
}

#[test]
fn every_vector_with_a_secret_rebuilds_its_key_and_its_signature() {
    let mut rebuilt = 0;
    for vector in vectors() {
        let Some(secret) = &vector.secret else { continue };
        let signing = SigningKey::from_bytes(&secret.clone().try_into().expect("32 bytes"));
        assert_eq!(signing.verifying_key().to_bytes().to_vec(), vector.key, "{}", vector.name);
        assert_eq!(signing.sign(&vector.message).to_bytes().to_vec(), vector.signature, "{}", vector.name);
        rebuilt += 1;
    }
    // The five of RFC 8032, the worked reading and the attacker's own key.
    assert_eq!(rebuilt, 7);
}

#[test]
fn of_the_taming_cases_the_strict_rule_holds_only_the_one_both_equations_hold() {
    let held: Vec<String> = vectors()
        .into_iter()
        .filter(|vector| vector.name.starts_with("taming-") && vector.expect == "holds")
        .map(|vector| vector.name)
        .collect();
    assert_eq!(held, vec!["taming-3".to_string()]);
}
