//! The long form of a verification: the request an answer carries computed
//! again, and the answer the engine writes for it compared with the answer
//! as it was sent, value by value and then byte by byte.
//!
//! A seal has no key: it is a plain SHA-384 built by a published rule, so
//! anyone can change a value of an answer, its hex, the seal parts, the seal
//! and the receipt, and every seal still holds. Recomputing the seals from
//! the answer's own values therefore shows only that the answer is
//! consistent with itself. The long form does that first, to name a seal
//! that does not hold; then it computes the request the answer carries again,
//! under a physics permit and the limits of the tool that computes it, and
//! compares the whole answer with the engine's answer to that request, the
//! receipt included. A number is compared by its 64-bit value and a string
//! exactly, to name the first field that differs; then the text sent is
//! compared with the engine's text, so a number spelled another way, a string
//! written with an escape, or a space the engine does not write is named too,
//! and the receipt the answer carries covers the very bytes that held. The
//! first place where the two differ is named.
//!
//! An answer whose request is not in it whole, a run continued from an
//! upstream sent alone, is refused by name ([`NO_REQUEST`]): its links, in
//! order, carry it.

use serde_json::{Map, Number, Value};

/// The cause of a long form whose seals hold and whose text is not the text
/// the engine writes for them.
pub(crate) const CAUSE_ANSWER_TEXT: &str = "answer_text";
/// The cause of a long form of an answer that names another engine version.
pub(crate) const CAUSE_ENGINE_VERSION: &str = "engine_version";

/// The name a long form is refused with when the answer does not carry the
/// whole request it was computed from, so there is nothing to compute again.
pub const NO_REQUEST: &str = "AnswerCarriesNoRequest";

/// The refusal of a long form of a run continued from a sealed upstream run,
/// sent alone: its answer carries its own request and the upstream's seal,
/// and not the upstream's request.
pub(crate) fn chained_alone() -> crate::DoorError {
    crate::DoorError::Unprocessable(format!(
        "Physical Violation: this answer continues a sealed upstream run (its chain names upstream_run_seal), and the upstream's request is not in it, so the request it was computed from cannot be computed again here ({NO_REQUEST}). Send its chain's answers in order, from the first run, as links, which computes each link again from the links before it; or its request with from, run_seal and chain_seal, the short form."
    ))
}

/// The first place where the answer sent is not the answer the engine
/// writes, and what is different there, in one sentence.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Difference {
    /// The path of the field: `reject_detail`, `final_state.t_s`,
    /// `frames[2].step`.
    pub at: String,
    pub says: String,
}

impl Difference {
    pub(crate) fn new(at: impl Into<String>, says: impl Into<String>) -> Self {
        Self { at: at.into(), says: says.into() }
    }
}

/// The first place, in the engine's order, where `sent` is not `engine`.
/// Objects are compared key by key in order, arrays item by item, a number
/// by its 64-bit value (minus zero is not zero), and strings, flags and null
/// exactly.
pub(crate) fn first_difference(engine: &Value, sent: &Value) -> Option<Difference> {
    walk("", engine, sent)
}


/// [`first_difference`] of a part of an answer, named under `path`.
pub(crate) fn first_difference_under(path: &str, engine: &Value, sent: &Value) -> Option<Difference> {
    walk(path, engine, sent)
}

/// What the comparison of an answer sent with the engine's answer found: the
/// first place they differ, if any, and whether every value of the answer
/// sent is the engine's (each number by its 64-bit value, strings, flags and
/// null exactly, keys and their order, the receipt string among them),
/// whatever its spelling. `holds` is every byte; `values_hold` every value.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Compared {
    pub difference: Option<Difference>,
    pub values_hold: bool,
}

/// The whole answer sent held to the engine's answer to the same request:
/// value by value first, which names the first field that differs and says
/// whether every value held, and then the text sent against the engine's
/// text, byte for byte, so the receipt the answer carries covers the bytes
/// that held. `engine_text` is the text the engine writes; `sent` and
/// `sent_text` the answer as it came, parsed and as text. Space or line
/// breaks before or after the whole answer are not part of it.
pub(crate) fn compare(engine_text: &str, sent: &Value, sent_text: &str) -> Result<Compared, crate::DoorError> {
    compare_under("", engine_text, sent, sent_text)
}

/// [`compare`] of a part of an answer, named under `path`.
pub(crate) fn compare_under(path: &str, engine_text: &str, sent: &Value, sent_text: &str) -> Result<Compared, crate::DoorError> {
    let engine: Value = serde_json::from_str(engine_text).map_err(|error| {
        crate::DoorError::Internal(format!("Physical Violation: the engine's answer could not be read again. {error}"))
    })?;
    if let Some(difference) = first_difference_under(path, &engine, sent) {
        return Ok(Compared { difference: Some(difference), values_hold: false });
    }
    Ok(Compared { difference: spelling_difference_under(path, engine_text, sent_text), values_hold: true })
}

/// The text of the answer sent, when no connection gave it: its JSON text,
/// which spells every number as serde_json does.
pub(crate) fn text_of(sent: &Value) -> String {
    serde_json::to_string(sent).unwrap_or_default()
}

/// The text of the answer sent: the text the connection read, when it reads
/// as the very values of the answer, and otherwise the answer's JSON text.
pub(crate) fn sent_text(sent: &Value, text: Option<&str>) -> String {
    if let Some(text) = text {
        if let Ok(read) = serde_json::from_str::<Value>(text) {
            if first_difference(&read, sent).is_none() {
                return text.to_string();
            }
        }
    }
    text_of(sent)
}

fn is_space(byte: u8) -> bool {
    matches!(byte, b' ' | b'\t' | b'\n' | b'\r')
}

/// One text read token by token.
struct Cursor<'a> {
    text: &'a [u8],
    at: usize,
}

impl<'a> Cursor<'a> {
    fn new(text: &'a str) -> Self {
        let bytes = text.as_bytes();
        let start = bytes.iter().position(|byte| !is_space(*byte)).unwrap_or(bytes.len());
        let end = bytes.iter().rposition(|byte| !is_space(*byte)).map_or(start, |last| last + 1);
        Self { text: &bytes[..end.max(start)], at: start }
    }

    fn peek(&self) -> Option<u8> {
        self.text.get(self.at).copied()
    }

    fn spaces(&mut self) -> usize {
        let start = self.at;
        while self.peek().is_some_and(is_space) {
            self.at += 1;
        }
        self.at - start
    }

    /// A string token, quotes included, or a number or a word.
    fn token(&mut self) -> &'a [u8] {
        let start = self.at;
        if self.peek() == Some(b'"') {
            self.at += 1;
            while let Some(byte) = self.peek() {
                self.at += 1;
                match byte {
                    b'\\' => self.at += 1,
                    b'"' => break,
                    _ => {}
                }
            }
        } else {
            while self.peek().is_some_and(|byte| !matches!(byte, b',' | b':' | b'}' | b']') && !is_space(byte)) {
                self.at += 1;
            }
        }
        &self.text[start..self.at.min(self.text.len())]
    }

    fn take(&mut self, byte: u8) -> bool {
        if self.peek() == Some(byte) {
            self.at += 1;
            true
        } else {
            false
        }
    }
}

fn shown(token: &[u8]) -> String {
    String::from_utf8_lossy(token).into_owned()
}

/// What every difference of spelling adds: the values held, the bytes did
/// not.
const VALUES_HELD: &str = "Every value of the answer sent is the engine's answer to its request, computed again, and its text is not the engine's text, so the receipt it carries does not cover these bytes. A program that writes an answer's numbers again in its own spelling (a JSON library that writes 1.0e-4 as 0.0001) makes such a copy; the answer as it came, byte for byte, holds.";

fn spaced(at: &str) -> Difference {
    let place = if at.is_empty() { "inside the answer".to_string() } else { format!("at '{at}'") };
    Difference::new(
        at,
        format!("The answer sent holds a space or a line break {place}, and the engine writes none. {VALUES_HELD}"),
    )
}

fn spelled(at: &str, engine: &[u8], sent: &[u8]) -> Difference {
    let what = if engine.first() == Some(&b'"') {
        "the same text written with other characters (an escape)"
    } else {
        "the same value spelled another way"
    };
    Difference::new(
        at,
        format!("'{at}' in the answer sent is written {}, and the engine writes {}: {what}. {VALUES_HELD}", shown(sent), shown(engine)),
    )
}

fn key_spelled(at: &str, engine: &[u8], sent: &[u8]) -> Difference {
    Difference::new(
        at,
        format!("The key of '{at}' in the answer sent is written {}, and the engine writes {}: the same key written with other characters (an escape). {VALUES_HELD}", shown(sent), shown(engine)),
    )
}

/// The first place where the text sent is not the engine's text, read in
/// step with it, named under `path`. The two hold the same values in the
/// same order, so they differ only in how a number, a string or a key is
/// written, or in a space or a line break.
fn spelling_difference_under(path: &str, engine_text: &str, sent_text: &str) -> Option<Difference> {
    let mut engine = Cursor::new(engine_text);
    let mut sent = Cursor::new(sent_text);
    spell(path, &mut engine, &mut sent).or_else(|| {
        (engine.at < engine.text.len() || sent.at < sent.text.len())
            .then(|| Difference::new(path, "The answer sent is not byte for byte the engine's answer: the two texts do not end at the same place."))
    })
}

fn spell(path: &str, engine: &mut Cursor, sent: &mut Cursor) -> Option<Difference> {
    if engine.spaces() != sent.spaces() {
        return Some(spaced(path));
    }
    match engine.peek() {
        Some(open @ (b'{' | b'[')) => {
            let close = if open == b'{' { b'}' } else { b']' };
            engine.at += 1;
            if !sent.take(open) {
                return Some(changed(path));
            }
            let mut index = 0_usize;
            loop {
                if engine.spaces() != sent.spaces() {
                    // Named by the member or the item the space stands before.
                    let next = if open == b'{' {
                        let at = engine.at;
                        let key = serde_json::from_slice::<String>(engine.token()).ok();
                        engine.at = at;
                        key.map(|key| join(path, &key))
                    } else {
                        Some(format!("{path}[{index}]"))
                    };
                    return Some(spaced(&next.filter(|_| engine.peek() != Some(close)).unwrap_or_else(|| path.to_string())));
                }
                if engine.take(close) {
                    return (!sent.take(close)).then(|| changed(path));
                }
                let at = if open == b'{' {
                    let engine_key = engine.token();
                    let sent_key = sent.token();
                    let key = serde_json::from_slice::<String>(engine_key).unwrap_or_default();
                    let at = join(path, &key);
                    if engine_key != sent_key {
                        return Some(key_spelled(&at, engine_key, sent_key));
                    }
                    if engine.spaces() != sent.spaces() {
                        return Some(spaced(&at));
                    }
                    if !(engine.take(b':') && sent.take(b':')) {
                        return Some(changed(&at));
                    }
                    at
                } else {
                    format!("{path}[{index}]")
                };
                if let Some(difference) = spell(&at, engine, sent) {
                    return Some(difference);
                }
                if engine.spaces() != sent.spaces() {
                    return Some(spaced(&at));
                }
                if engine.take(b',') != sent.take(b',') {
                    return Some(changed(path));
                }
                index += 1;
            }
        }
        Some(_) => {
            let engine_token = engine.token();
            let sent_token = sent.token();
            (engine_token != sent_token).then(|| spelled(path, engine_token, sent_token))
        }
        None => (sent.peek().is_some()).then(|| changed(path)),
    }
}

fn join(path: &str, key: &str) -> String {
    if path.is_empty() { key.to_string() } else { format!("{path}.{key}") }
}

fn walk(path: &str, engine: &Value, sent: &Value) -> Option<Difference> {
    match (engine, sent) {
        (Value::Object(engine), Value::Object(sent)) => walk_object(path, engine, sent),
        (Value::Array(engine), Value::Array(sent)) => {
            for (index, item) in engine.iter().enumerate() {
                let at = format!("{path}[{index}]");
                match sent.get(index) {
                    Some(other) => {
                        if let Some(difference) = walk(&at, item, other) {
                            return Some(difference);
                        }
                    }
                    None => return Some(missing(&at)),
                }
            }
            if sent.len() > engine.len() {
                return Some(added(&format!("{path}[{}]", engine.len())));
            }
            None
        }
        (Value::Number(engine), Value::Number(sent)) if same_number(engine, sent) => None,
        (Value::String(engine), Value::String(sent)) if engine == sent => None,
        (Value::Bool(engine), Value::Bool(sent)) if engine == sent => None,
        (Value::Null, Value::Null) => None,
        _ => Some(changed(path)),
    }
}

fn walk_object(path: &str, engine: &Map<String, Value>, sent: &Map<String, Value>) -> Option<Difference> {
    let mut theirs = sent.iter();
    for (key, value) in engine {
        let Some((sent_key, sent_value)) = theirs.next() else {
            return Some(missing(&join(path, key)));
        };
        if sent_key != key {
            return Some(if !sent.contains_key(key) {
                missing(&join(path, key))
            } else if !engine.contains_key(sent_key) {
                added(&join(path, sent_key))
            } else {
                Difference::new(
                    join(path, sent_key),
                    format!(
                        "The answer sent holds '{}' where the engine's answer holds '{}': its keys are not in the order the engine writes them.",
                        join(path, sent_key),
                        join(path, key)
                    ),
                )
            });
        }
        if let Some(difference) = walk(&join(path, key), value, sent_value) {
            return Some(difference);
        }
    }
    theirs.next().map(|(key, _)| added(&join(path, key)))
}

fn same_number(engine: &Number, sent: &Number) -> bool {
    if let (Some(left), Some(right)) = (engine.as_u64(), sent.as_u64()) {
        return left == right;
    }
    if let (Some(left), Some(right)) = (engine.as_i64(), sent.as_i64()) {
        return left == right;
    }
    match (engine.as_f64(), sent.as_f64()) {
        (Some(left), Some(right)) => left.to_bits() == right.to_bits(),
        _ => false,
    }
}

fn missing(at: &str) -> Difference {
    Difference::new(at, format!("The answer sent has no '{at}', which the engine's answer holds."))
}

fn added(at: &str) -> Difference {
    Difference::new(at, format!("The answer sent holds '{at}', which the engine's answer does not."))
}

fn changed(at: &str) -> Difference {
    if at == "receipt_sha384" || at.ends_with(".receipt_sha384") {
        return Difference::new(
            at,
            "The receipt the answer sent carries is not the receipt of the answer the engine writes for this request, while every other value is the engine's: the receipt was changed, or worked out again over a text that is not the engine's (a number spelled another way, a key in another order), which gives another receipt and the same seal. verify_receipt compares a copy's own bytes with the receipt that same copy carries.",
        );
    }
    Difference::new(
        at,
        format!("'{at}' in the answer sent is not what the engine writes for this request."),
    )
}

/// What a long form says when the answer names another engine version: the
/// version is named, and the text is not compared, since this engine writes
/// only its own answers.
pub(crate) fn another_version(sent: &str, served: &str, seal: &str) -> String {
    format!(
        "The answer names engine_version {sent}, and this engine is {served}. An engine writes only its own answers, so the text of an answer of {sent} is not compared here and holds is false. Its seals were recomputed from its exact values and hold. Its request and its {seal}, given to the short form, are computed again by {served}."
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn the_first_difference_is_named_by_its_path() {
        let engine = json!({"a": 1, "b": {"c": [1.5, 2.0], "d": null}, "e": "x"});
        assert_eq!(first_difference(&engine, &engine.clone()), None);
        let at = |sent: Value| first_difference(&engine, &sent).map(|difference| difference.at);
        assert_eq!(at(json!({"a": 1, "b": {"c": [1.5, 2.5], "d": null}, "e": "x"})).as_deref(), Some("b.c[1]"));
        assert_eq!(at(json!({"a": 1, "b": {"c": [1.5, 2.0], "d": "y"}, "e": "x"})).as_deref(), Some("b.d"));
        assert_eq!(at(json!({"a": 1, "b": {"c": [1.5, 2.0], "d": null}, "e": "x", "f": 1})).as_deref(), Some("f"));
        assert_eq!(at(json!({"a": 1, "b": {"c": [1.5, 2.0], "d": null}})).as_deref(), Some("e"));
        assert_eq!(at(json!({"a": 1, "b": {"c": [1.5], "d": null}, "e": "x"})).as_deref(), Some("b.c[1]"));
        assert_eq!(at(json!({"a": 1, "b": {"c": [1.5, 2.0, 3.0], "d": null}, "e": "x"})).as_deref(), Some("b.c[2]"));
        assert_eq!(at(json!({"b": {"c": [1.5, 2.0], "d": null}, "a": 1, "e": "x"})).as_deref(), Some("b"));
        // A number is its value: 2 is 2.0, and minus zero is not zero.
        assert_eq!(at(json!({"a": 1.0, "b": {"c": [1.5, 2], "d": null}, "e": "x"})), None);
        let zero = json!({"z": 0.0});
        assert_eq!(first_difference(&zero, &json!({"z": -0.0})).map(|difference| difference.at).as_deref(), Some("z"));
    }

    /// A text that holds the engine's very values is held to the engine's
    /// bytes: a decimal spelled another way (the red team's
    /// 1.9630162443860627551062148e-03 for 1.9630162443860628e-3), a string or
    /// a key written with an escape, a number written whole, and a space are
    /// each named at their place; space around the whole answer is not part
    /// of it.
    #[test]
    fn a_text_of_the_same_values_is_held_to_the_engines_bytes() {
        let engine = r#"{"inputs":{"solver":"x","parameters":{"a":0.0001}},"outcome":"converged","result":{"v":[1.9630162443860628e-3,2.0e-4],"v_hex":["3f6014be5246e69b","3f2a36e2eb1c432d"]},"receipt_sha384":"ab"}"#;
        let at = |sent: &str| {
            let value: Value = serde_json::from_str(sent).unwrap();
            compare(engine, &value, sent).ok().and_then(|compared| compared.difference).map(|difference| difference.at)
        };
        assert_eq!(at(engine), None);
        assert_eq!(at(&format!("\n {engine}\n")), None);
        assert_eq!(at(&engine.replace("1.9630162443860628e-3", "1.9630162443860627551062148e-03")).as_deref(), Some("result.v[0]"));
        assert_eq!(at(&engine.replace("2.0e-4", "2e-4")).as_deref(), Some("result.v[1]"));
        assert_eq!(at(&engine.replace("0.0001", "1e-4")).as_deref(), Some("inputs.parameters.a"));
        assert_eq!(at(&engine.replace("\"converged\"", "\"\\u0063onverged\"")).as_deref(), Some("outcome"));
        assert_eq!(at(&engine.replace("\"outcome\"", "\"\\u006futcome\"")).as_deref(), Some("outcome"));
        assert_eq!(at(&engine.replace("\"result\":{", "\"result\": {")).as_deref(), Some("result"));
        assert_eq!(at(&engine.replace(",\"receipt_sha384\"", ", \"receipt_sha384\"")).as_deref(), Some("receipt_sha384"));
        // Each says what it is.
        let value: Value = serde_json::from_str(engine).unwrap();
        let respelled = engine.replace("2.0e-4", "2e-4");
        let compared = compare(engine, &value, &respelled).unwrap();
        assert!(compared.values_hold, "a spelling is not a value");
        let says = compared.difference.unwrap().says;
        let changed = engine.replace("2.0e-4", "3.0e-4");
        let compared = compare(engine, &serde_json::from_str(&changed).unwrap(), &changed).unwrap();
        assert!(!compared.values_hold && compared.difference.is_some());
        assert_eq!(compare(engine, &value, engine).unwrap(), Compared { difference: None, values_hold: true });
        assert!(says.contains("is written 2e-4, and the engine writes 2.0e-4"), "{says}");
    }
}
