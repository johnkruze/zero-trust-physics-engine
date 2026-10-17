//! The solver contract, attacked the same way on every solver.
//!
//! A solver plugs in with one `Kit`: its question as a list of named numbers
//! (a list input is one number per entry, as `segment_end_h[1]`), and a
//! function that builds the question from those numbers, solves it and
//! returns the answer or the refusal as its `Debug` text. `findings` then
//! attacks the contract of `genesis_solvers/AGENTS.md`:
//!
//! - **A converged result or a named limit** (rule 2): the question as given
//!   is answered, and the answer holds no number that is not finite.
//! - **Nothing is filled in** (rule 5): each number set to NaN and to each
//!   infinity in turn is refused by name, never answered.
//! - **Same input, same bits** (rule 6): the question solved twice gives the
//!   same answer, bit for bit.
//! - **Normal or zero** (engine invariant 4, `genesis_solvers::normal`): no
//!   answer holds a subnormal number, neither the kit's question's nor one
//!   with a number of it set to the smallest normal number or to 2^-1000,
//!   each with its sign, which drives the values that scale with it under
//!   the normal range.
//!
//! What a solver takes and refuses at its declared bounds is attacked through
//! the gateway, from its description: `protocol_mcp_gateway/tests/conformance_solvers.rs`.

#![allow(dead_code)]

use std::panic::{AssertUnwindSafe, catch_unwind};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Weight {
    Contract,
    Note,
}

#[derive(Clone, Debug)]
pub struct Finding {
    pub model: &'static str,
    pub rule: &'static str,
    pub weight: Weight,
    pub detail: String,
}

impl Finding {
    pub fn line(&self) -> String {
        let weight = if self.weight == Weight::Contract { "CONTRACT" } else { "note" };
        format!("{} | {} | {} | {}", self.model, weight, self.rule, self.detail)
    }

    pub fn json(&self) -> String {
        let escape = |text: &str| text.replace('\\', "\\\\").replace('"', "\\\"").replace('\n', " ");
        format!(
            "{{\"model\":\"{}\",\"layer\":\"solver\",\"weight\":\"{}\",\"rule\":\"{}\",\"detail\":\"{}\"}}",
            self.model,
            if self.weight == Weight::Contract { "contract" } else { "note" },
            self.rule,
            escape(&self.detail)
        )
    }
}

/// Everything the attacks need of one solver.
pub struct Kit {
    pub model: &'static str,
    /// The question's numbers, each with its name.
    pub numbers: Vec<(String, f64)>,
    /// Build the question from `numbers`, in the same order, and solve it:
    /// `Ok` with the answer's `Debug`, or `Err` with the refusal's.
    pub solve: fn(&[f64]) -> Result<String, String>,
    /// Attack each number with NaN and the infinities. Off for a solver whose
    /// one solve takes long enough that the sweep would not finish.
    pub every_number: bool,
}

fn attempt(kit: &Kit, numbers: &[f64]) -> Result<Result<String, String>, String> {
    let solve = kit.solve;
    catch_unwind(AssertUnwindSafe(|| solve(numbers))).map_err(|panic| {
        panic.downcast_ref::<String>().cloned().or_else(|| panic.downcast_ref::<&str>().map(|s| s.to_string())).unwrap_or_default()
    })
}

/// The refusal's name: the first word of its `Debug`.
fn name(debug: &str) -> &str {
    debug.split(|c: char| !c.is_ascii_alphanumeric()).next().unwrap_or(debug)
}

/// The subnormal numbers an answer's `Debug` holds, as written.
pub fn subnormal_numbers(debug: &str) -> Vec<String> {
    debug
        .split(|c: char| !(c.is_ascii_digit() || c == '.' || c == '-' || c == 'e'))
        .filter(|word| word.contains('e') && word.parse::<f64>().is_ok_and(f64::is_subnormal))
        .map(str::to_string)
        .collect()
}

/// The smallest normal number and 2^-1000, the sizes a number is set to.
const TINY: [f64; 2] = [f64::MIN_POSITIVE, 9.332636185032189e-302];

fn not_finite(debug: &str) -> bool {
    debug.split(|c: char| !(c.is_ascii_alphanumeric() || c == '.' || c == '-')).any(|word| word == "NaN" || word == "inf" || word == "-inf")
}

pub fn findings(kit: &Kit) -> Vec<Finding> {
    let mut found = Vec::new();
    let mut put = |rule: &'static str, weight: Weight, detail: String| found.push(Finding { model: kit.model, rule, weight, detail });
    let values: Vec<f64> = kit.numbers.iter().map(|(_, value)| *value).collect();
    let first = match attempt(kit, &values) {
        Ok(Ok(answer)) => {
            if not_finite(&answer) {
                put("converged result", Weight::Contract, "the answer to the kit's question holds a number that is not finite".to_string());
            }
            let subnormal = subnormal_numbers(&answer);
            if !subnormal.is_empty() {
                put("normal or zero", Weight::Contract, format!("the answer to the kit's question holds a subnormal number: {}", subnormal.join(", ")));
            }
            Some(answer)
        }
        Ok(Err(refusal)) => {
            put("harness", Weight::Note, format!("the kit's question is refused as {}", name(&refusal)));
            None
        }
        Err(panic) => {
            put("converged result", Weight::Contract, format!("the kit's question panicked: {panic}"));
            None
        }
    };
    if let Some(first) = &first {
        match attempt(kit, &values) {
            Ok(Ok(second)) if &second == first => {}
            _ => put("same input, same bits", Weight::Contract, "the same question twice gave different answers".to_string()),
        }
    }
    if !kit.every_number {
        return found;
    }
    let mut subnormal = Vec::new();
    for (index, (label, value)) in kit.numbers.iter().enumerate() {
        for tiny in TINY {
            let mut moved = values.clone();
            moved[index] = if value.is_sign_negative() { -tiny } else { tiny };
            if let Ok(Ok(answer)) = attempt(kit, &moved) {
                let numbers = subnormal_numbers(&answer);
                if !numbers.is_empty() {
                    subnormal.push(format!("{label} = {:e}: {}", moved[index], numbers.join(", ")));
                }
            }
        }
    }
    if !subnormal.is_empty() {
        put("normal or zero", Weight::Contract, format!("an answer holds a subnormal number: {}", subnormal.join("; ")));
    }
    for (index, (label, _)) in kit.numbers.iter().enumerate() {
        for bad in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            let mut moved = values.clone();
            moved[index] = bad;
            match attempt(kit, &moved) {
                Ok(Err(_)) => {}
                Ok(Ok(answer)) => {
                    let what = if not_finite(&answer) { "answered with a number that is not finite" } else { "answered" };
                    put("nothing filled in", Weight::Contract, format!("{label} = {bad}: {what}"));
                    break;
                }
                Err(panic) => {
                    put("nothing filled in", Weight::Contract, format!("{label} = {bad}: the solver panicked: {panic}"));
                    break;
                }
            }
        }
    }
    found
}
