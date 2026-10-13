//! The columns of a final state, found from the rows themselves.
//!
//! The crate knows no card. The state of a row is a JSON object. A key with
//! a `_hex` sibling is a number: a string of 16 lowercase hex digits is one
//! 64-bit float, an array of three such strings is a 3-vector. A boolean is a
//! flag. The decimal beside a hex value is never read.

use serde_json::{Map, Value};

/// The two residuals every row with a run carries, and their column names.
pub const FINAL_RESIDUAL: &str = "final_residual_conservation_error";
pub const MAX_RESIDUAL: &str = "max_residual_conservation_error";

/// The names the converter writes itself.
pub const FIXED_COLUMNS: [&str; 10] = [
    "row",
    "outcome",
    "steps_landed",
    "reject",
    "reject_detail",
    "frames",
    "state_digest",
    "last_link",
    "run_seal",
    "request",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// One float64 column.
    Scalar,
    /// Three float64 columns, `<key>_0`, `<key>_1`, `<key>_2`.
    Triple,
    /// One boolean column.
    Flag,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Field {
    pub key: String,
    pub kind: Kind,
}

/// The state fields of one dataset, in the order of the first state found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StateSchema {
    pub fields: Vec<Field>,
}

/// The value of one field of one row.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Cell {
    Scalar(f64),
    Triple([f64; 3]),
    Flag(bool),
}

/// The decoded state of one row: one cell for each field, then the residuals.
#[derive(Debug, Clone, PartialEq)]
pub struct Decoded {
    pub cells: Vec<Cell>,
    pub final_residual: f64,
    pub max_residual: f64,
}

/// A float from 16 lowercase hex digits: the digits of `f64::to_bits`.
pub fn f64_from_hex(text: &str) -> Result<f64, String> {
    let exact = text.len() == 16 && text.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'));
    if !exact {
        return Err(format!("'{text}' is not 16 lowercase hex digits"));
    }
    u64::from_str_radix(text, 16)
        .map(f64::from_bits)
        .map_err(|error| error.to_string())
}

impl StateSchema {
    /// Read the fields of one state.
    pub fn from_state(state: &Map<String, Value>) -> Result<StateSchema, String> {
        let mut fields: Vec<Field> = Vec::new();
        for (key, value) in state {
            if let Some(base) = key.strip_suffix("_hex") {
                if !state.contains_key(base) {
                    return Err(format!("final_state holds '{key}' and no '{base}' beside it"));
                }
                continue;
            }
            let kind = if value.is_boolean() {
                Kind::Flag
            } else {
                match state.get(&format!("{key}_hex")) {
                    Some(Value::String(_)) => Kind::Scalar,
                    Some(Value::Array(items)) if items.len() == 3 => Kind::Triple,
                    Some(_) => {
                        return Err(format!(
                            "final_state '{key}_hex' is neither a hex string nor an array of three"
                        ));
                    }
                    None => {
                        return Err(format!(
                            "final_state '{key}' is not a boolean and has no '{key}_hex' beside it"
                        ));
                    }
                }
            };
            fields.push(Field { key: key.clone(), kind });
        }
        let schema = StateSchema { fields };
        let mut names: Vec<String> = FIXED_COLUMNS.iter().map(|s| s.to_string()).collect();
        names.push(FINAL_RESIDUAL.into());
        names.push(MAX_RESIDUAL.into());
        for field in &schema.fields {
            for name in schema.column_names(field) {
                if names.contains(&name) {
                    return Err(format!("final_state '{}' gives a column name used twice: '{name}'", field.key));
                }
                names.push(name);
            }
        }
        Ok(schema)
    }

    pub fn column_names(&self, field: &Field) -> Vec<String> {
        match field.kind {
            Kind::Triple => (0..3).map(|i| format!("{}_{i}", field.key)).collect(),
            _ => vec![field.key.clone()],
        }
    }

    /// Hold a later state to this schema, by the name of the first key that differs.
    pub fn agrees_with(&self, state: &Map<String, Value>) -> Result<(), String> {
        let other = StateSchema::from_state(state)?;
        for (index, field) in self.fields.iter().enumerate() {
            match other.fields.get(index) {
                None => return Err(format!("it has no '{}'", field.key)),
                Some(found) if found.key != field.key => {
                    return Err(format!("it has '{}' where the schema has '{}'", found.key, field.key));
                }
                Some(found) if found.kind != field.kind => {
                    return Err(format!("'{}' is {:?} and the schema has {:?}", field.key, found.kind, field.kind));
                }
                Some(_) => {}
            }
        }
        match other.fields.get(self.fields.len()) {
            Some(extra) => Err(format!("it has '{}', which the schema does not", extra.key)),
            None => Ok(()),
        }
    }

    /// The exact values of one state, from its hex, and its two residuals.
    /// The state must already agree with the schema.
    pub fn decode(
        &self,
        state: &Map<String, Value>,
        final_residual_hex: Option<&str>,
        max_residual_hex: Option<&str>,
    ) -> Result<Decoded, String> {
        let mut cells = Vec::with_capacity(self.fields.len());
        for field in &self.fields {
            let hex_key = format!("{}_hex", field.key);
            let cell = match field.kind {
                Kind::Flag => Cell::Flag(
                    state.get(&field.key).and_then(Value::as_bool).ok_or_else(|| {
                        format!("final_state '{}' is not a boolean", field.key)
                    })?,
                ),
                Kind::Scalar => Cell::Scalar(decode_one(state.get(&hex_key), &hex_key)?),
                Kind::Triple => {
                    let items = state
                        .get(&hex_key)
                        .and_then(Value::as_array)
                        .filter(|items| items.len() == 3)
                        .ok_or_else(|| format!("final_state '{hex_key}' is not an array of three"))?;
                    Cell::Triple([
                        decode_one(items.first(), &hex_key)?,
                        decode_one(items.get(1), &hex_key)?,
                        decode_one(items.get(2), &hex_key)?,
                    ])
                }
            };
            cells.push(cell);
        }
        let residual = |text: Option<&str>, name: &str| match text {
            Some(text) => f64_from_hex(text).map_err(|e| format!("{name}_hex: {e}")),
            None => Err(format!("{name}_hex is null in a row that has a state")),
        };
        Ok(Decoded {
            cells,
            final_residual: residual(final_residual_hex, FINAL_RESIDUAL)?,
            max_residual: residual(max_residual_hex, MAX_RESIDUAL)?,
        })
    }
}

fn decode_one(value: Option<&Value>, name: &str) -> Result<f64, String> {
    let text = value
        .and_then(Value::as_str)
        .ok_or_else(|| format!("final_state '{name}' is not a hex string"))?;
    f64_from_hex(text).map_err(|e| format!("final_state '{name}': {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn state(value: Value) -> Map<String, Value> {
        value.as_object().unwrap().clone()
    }

    #[test]
    fn a_hex_value_is_decoded_to_its_bits_and_the_decimal_is_not_read() {
        let s = state(json!({
            "t_s": 1.0, "t_s_hex": "3ff0000000000000",
            "p_m": [1.0, 2.0, 3.0], "p_m_hex": ["3ff0000000000000", "4000000000000000", "4008000000000000"],
            "flag": true
        }));
        let schema = StateSchema::from_state(&s).unwrap();
        assert_eq!(schema.fields.iter().map(|f| f.kind).collect::<Vec<_>>(),
            vec![Kind::Scalar, Kind::Triple, Kind::Flag]);
        let d = schema.decode(&s, Some("0000000000000000"), Some("3ff0000000000000")).unwrap();
        assert_eq!(d.cells[0], Cell::Scalar(1.0));
        assert_eq!(d.cells[1], Cell::Triple([1.0, 2.0, 3.0]));
        assert_eq!(d.cells[2], Cell::Flag(true));
        assert_eq!(d.max_residual, 1.0);
    }

    #[test]
    fn a_state_that_is_not_in_the_form_is_refused_by_name() {
        let no_hex = state(json!({ "a": 1.0 }));
        assert!(StateSchema::from_state(&no_hex).unwrap_err().contains("'a'"));
        let orphan = state(json!({ "a_hex": "3ff0000000000000" }));
        assert!(StateSchema::from_state(&orphan).unwrap_err().contains("'a'"));
        let upper = state(json!({ "a": 1.0, "a_hex": "3FF0000000000000" }));
        let schema = StateSchema::from_state(&upper).unwrap();
        assert!(schema.decode(&upper, Some("0000000000000000"), Some("0000000000000000")).is_err());
        let clash = state(json!({ "row": 1.0, "row_hex": "3ff0000000000000" }));
        assert!(StateSchema::from_state(&clash).unwrap_err().contains("used twice"));
    }
}
