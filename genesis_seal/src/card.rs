//! What a plan is checked against: a card's field names and published bounds.
//!
//! This crate knows no card. The gateway supplies this description, and
//! `describe_card` publishes the same facts, so [`CardFields::from_description`]
//! reads them from that tool's answer.

use std::fmt;

use serde_json::{Map, Value};

/// One phase field or load field of a card.
#[derive(Debug, Clone, PartialEq)]
pub struct Field {
    pub name: String,
    /// 1 for one number, 3 for a 3-vector.
    pub components: u8,
    pub bounds: Bounds,
}

/// The published range of one number. A side that is not published is open.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Bounds {
    pub minimum: Option<f64>,
    pub minimum_exclusive: bool,
    pub maximum: Option<f64>,
    pub maximum_exclusive: bool,
}

impl Bounds {
    /// No published range.
    pub const OPEN: Bounds = Bounds {
        minimum: None,
        minimum_exclusive: false,
        maximum: None,
        maximum_exclusive: false,
    };

    /// `true` when `value` lies inside the range. Nothing is pulled back to a
    /// bound: a value on an exclusive bound is outside.
    pub fn holds(&self, value: f64) -> bool {
        let above = match self.minimum {
            None => true,
            Some(minimum) if self.minimum_exclusive => value > minimum,
            Some(minimum) => value >= minimum,
        };
        let below = match self.maximum {
            None => true,
            Some(maximum) if self.maximum_exclusive => value < maximum,
            Some(maximum) => value <= maximum,
        };
        above && below
    }
}

/// The fields of one card, in the order the card lists them.
#[derive(Debug, Clone, PartialEq)]
pub struct CardFields {
    /// The fields of `initial`.
    pub phase: Vec<Field>,
    /// The fields of a segment's `load`.
    pub load: Vec<Field>,
    /// The card's own step floor and step ceiling, both inclusive.
    pub dt_s: Bounds,
}

/// Why a card description was not read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CardRefusal {
    /// The description is not a JSON object.
    NotAnObject,
    /// A part of the description is missing.
    Missing(String),
    /// A part of the description has the wrong JSON type.
    WrongType(String),
    /// A field states a count of components other than 1 or 3.
    Components(String),
    /// Two fields carry one name.
    NameRepeated(String),
}

impl fmt::Display for CardRefusal {
    fn fmt(&self, out: &mut fmt::Formatter) -> fmt::Result {
        match self {
            CardRefusal::NotAnObject => {
                write!(out, "The card description must be a JSON object.")
            }
            CardRefusal::Missing(name) => write!(out, "The card description has no '{name}'."),
            CardRefusal::WrongType(name) => {
                write!(out, "The card description's '{name}' has the wrong type.")
            }
            CardRefusal::Components(name) => write!(
                out,
                "The card description's '{name}' must have 1 or 3 components."
            ),
            CardRefusal::NameRepeated(name) => {
                write!(out, "The card description lists '{name}' twice.")
            }
        }
    }
}

impl CardFields {
    /// Read the answer of `describe_card`: `phase_fields` and `load_fields`,
    /// each entry with `name`, `components`, `minimum`, `minimum_exclusive`,
    /// `maximum` and `maximum_exclusive`, and `dt_s` with `floor_s` and
    /// `ceiling_s`. Every other part of the description is left unread.
    pub fn from_description(description: &Value) -> Result<CardFields, CardRefusal> {
        let object = description.as_object().ok_or(CardRefusal::NotAnObject)?;
        let phase = read_fields(object, "phase_fields")?;
        let load = read_fields(object, "load_fields")?;
        let dt = part(object, "dt_s")?
            .as_object()
            .ok_or_else(|| CardRefusal::WrongType("dt_s".to_string()))?;
        let floor = number(dt, "floor_s", "dt_s.floor_s")?;
        let ceiling = number(dt, "ceiling_s", "dt_s.ceiling_s")?;
        Ok(CardFields {
            phase,
            load,
            dt_s: Bounds {
                minimum: Some(floor),
                minimum_exclusive: false,
                maximum: Some(ceiling),
                maximum_exclusive: false,
            },
        })
    }

    pub fn phase_field(&self, name: &str) -> Option<&Field> {
        self.phase.iter().find(|field| field.name == name)
    }

    pub fn load_field(&self, name: &str) -> Option<&Field> {
        self.load.iter().find(|field| field.name == name)
    }
}

fn part<'a>(object: &'a Map<String, Value>, key: &str) -> Result<&'a Value, CardRefusal> {
    object
        .get(key)
        .ok_or_else(|| CardRefusal::Missing(key.to_string()))
}

fn number(object: &Map<String, Value>, key: &str, name: &str) -> Result<f64, CardRefusal> {
    object
        .get(key)
        .ok_or_else(|| CardRefusal::Missing(name.to_string()))?
        .as_f64()
        .ok_or_else(|| CardRefusal::WrongType(name.to_string()))
}

fn optional_number(
    object: &Map<String, Value>,
    key: &str,
    name: &str,
) -> Result<Option<f64>, CardRefusal> {
    match object.get(key) {
        None => Err(CardRefusal::Missing(format!("{name}.{key}"))),
        Some(Value::Null) => Ok(None),
        Some(value) => value
            .as_f64()
            .map(Some)
            .ok_or_else(|| CardRefusal::WrongType(format!("{name}.{key}"))),
    }
}

fn flag(object: &Map<String, Value>, key: &str, name: &str) -> Result<bool, CardRefusal> {
    object
        .get(key)
        .ok_or_else(|| CardRefusal::Missing(format!("{name}.{key}")))?
        .as_bool()
        .ok_or_else(|| CardRefusal::WrongType(format!("{name}.{key}")))
}

fn read_fields(object: &Map<String, Value>, key: &str) -> Result<Vec<Field>, CardRefusal> {
    let items = part(object, key)?
        .as_array()
        .ok_or_else(|| CardRefusal::WrongType(key.to_string()))?;
    let mut fields: Vec<Field> = Vec::with_capacity(items.len());
    for item in items {
        let item = item
            .as_object()
            .ok_or_else(|| CardRefusal::WrongType(key.to_string()))?;
        let name = part(item, "name")?
            .as_str()
            .ok_or_else(|| CardRefusal::WrongType(format!("{key}.name")))?
            .to_string();
        let components = match part(item, "components")?.as_u64() {
            Some(1) => 1,
            Some(3) => 3,
            _ => return Err(CardRefusal::Components(name)),
        };
        if fields.iter().any(|field| field.name == name) {
            return Err(CardRefusal::NameRepeated(name));
        }
        let bounds = Bounds {
            minimum: optional_number(item, "minimum", &name)?,
            minimum_exclusive: flag(item, "minimum_exclusive", &name)?,
            maximum: optional_number(item, "maximum", &name)?,
            maximum_exclusive: flag(item, "maximum_exclusive", &name)?,
        };
        fields.push(Field {
            name,
            components,
            bounds,
        });
    }
    Ok(fields)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn description(name: &str) -> Value {
        let text = match name {
            "thermal" => include_str!("../tests/fixtures/describe_thermal.json"),
            "ti64_lattice" => include_str!("../tests/fixtures/describe_ti64_lattice.json"),
            _ => include_str!("../tests/fixtures/describe_satellite_lageos1.json"),
        };
        serde_json::from_str(text).expect("a saved description reads")
    }

    #[test]
    fn the_thermal_description_gives_one_phase_field_two_load_fields_and_the_step_range() {
        let card = CardFields::from_description(&description("thermal")).unwrap();
        assert_eq!(card.phase.len(), 1);
        assert_eq!(card.phase[0].name, "temperature_k");
        assert_eq!(card.phase[0].components, 1);
        let names: Vec<&str> = card.load.iter().map(|field| field.name.as_str()).collect();
        assert_eq!(names, ["heat_power_w", "ambient_k"]);
        assert_eq!(card.load_field("heat_power_w").unwrap().bounds, Bounds::OPEN);
        let window = card.phase_field("temperature_k").unwrap().bounds;
        assert!(window.holds(233.15));
        assert!(!window.holds(233.14999999999998));
        assert!(window.holds(398.1499999999999));
        assert!(!window.holds(398.15));
        assert!(card.dt_s.holds(23.625));
        assert!(!card.dt_s.holds(23.625000000000004));
        assert!(card.dt_s.holds(2.2530555725097656e-5));
        assert!(!card.dt_s.holds(2.2530555725097653e-5));
    }

    #[test]
    fn a_3_vector_field_is_read_as_three_components() {
        let lattice = CardFields::from_description(&description("ti64_lattice")).unwrap();
        assert_eq!(lattice.phase_field("position_m").unwrap().components, 3);
        assert_eq!(lattice.load_field("force_n").unwrap().components, 3);
        assert_eq!(lattice.load_field("temperature_k").unwrap().components, 1);
        assert!(lattice.load_field("temperature_k").unwrap().bounds.holds(1268.15));
        let satellite = CardFields::from_description(&description("satellite_lageos1")).unwrap();
        assert!(satellite.load.is_empty());
        assert_eq!(satellite.phase.len(), 3);
    }

    #[test]
    fn a_description_with_a_part_missing_is_refused_by_name() {
        let mut body = description("thermal");
        body.as_object_mut().unwrap().remove("load_fields");
        assert_eq!(
            CardFields::from_description(&body).unwrap_err(),
            CardRefusal::Missing("load_fields".to_string())
        );
        let mut body = description("thermal");
        body["dt_s"].as_object_mut().unwrap().remove("ceiling_s");
        assert_eq!(
            CardFields::from_description(&body).unwrap_err(),
            CardRefusal::Missing("dt_s.ceiling_s".to_string())
        );
        let mut body = description("thermal");
        body["phase_fields"][0]["components"] = serde_json::json!(2);
        assert_eq!(
            CardFields::from_description(&body).unwrap_err(),
            CardRefusal::Components("temperature_k".to_string())
        );
        let mut body = description("thermal");
        body["phase_fields"][0].as_object_mut().unwrap().remove("maximum_exclusive");
        assert!(CardFields::from_description(&body).is_err());
    }
}
