//! The rules for sealing and checking an engine's records.
//!
//! This crate computes no physics. It writes the sealed form of a request,
//! follows the chain of a run, expands a plan of runs, builds the row tree
//! and the dataset seal, and reads and writes the dataset files. A check made
//! here never needs the engine. A signed load is verified here too
//! (`attest`), and a commitment to the measurements of a comparison is
//! opened here (`commitment`): one implementation of each, which the gateway
//! calls.

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
#[cfg(unix)]
pub mod stored_file;
pub mod tree;
pub mod validation;
