//! The rules for sealing and checking an engine's records.
//!
//! This crate computes no physics. It writes the sealed form of a request,
//! follows the chain of a run, expands a plan of runs, builds the row tree
//! and the dataset seal, and reads and writes the dataset files. A check made
//! here never needs the engine. A signed load is verified here too
//! (`attest`), and a commitment to the measurements of a comparison is
//! opened here (`commitment`): one implementation of each, which the gateway
//! calls.
//!
//! Every hash of a seal is SHA-384 by the seal rule [`SEAL_RULE`], written as
//! 96 lowercase hex digits; an answer names the rule in `seal_rule`, just
//! before its receipt. The engine's signature on an answer, ML-DSA-87 over the
//! receipt, is checked here too (`answer_signature`); the gateway signs with
//! the feature `sign`.

pub mod answer_signature;
pub mod attest;
pub mod card;
pub mod chain;
pub mod commitment;
pub mod dataset;
pub mod fault;
pub mod frames;
pub mod json_text;
pub mod plan;
pub mod receipt;
pub mod refusal;
pub mod rows;
pub mod run;
pub mod sampler;
pub mod sealed_form;
pub mod solution;
#[cfg(unix)]
pub mod stored_file;
pub mod tree;
pub mod validation;

/// The seal rule of every answer this crate verifies: SHA-384 throughout.
/// Seal rule v1, SHA-256, was the rule of engine 0.14.3 and earlier.
pub const SEAL_RULE: &str = "zero-trust-physics/seal/v2";
/// The key that names the seal rule in an answer.
pub const SEAL_RULE_KEY: &str = "seal_rule";
/// Bytes of a hash of seal rule v2 (SHA-384).
pub const HASH_LEN: usize = 48;
/// Hex digits of a hash of seal rule v2.
pub const HASH_HEX: usize = 96;
