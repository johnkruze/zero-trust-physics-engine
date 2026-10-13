//! Reads a heat-shield slab question as JSON (a file named on the command
//! line, or standard input) and prints the answer as JSON: the summary, and
//! whatever else was asked for by name. A refusal is printed by its name and
//! its sentence.
//!
//! ```text
//! {
//!   "material": "TACOT 3.0" | "PICA, open record, version 5" | an earlier PICA version's name,
//!   "thickness_m": 0.0582,
//!   "start_temperature_k": 260.0,
//!   "thermocouple_depths_m": [0.005, 0.01],
//!   "surface": { "held_temperature_k": 1644.0, "hold_duration_s": 60.0, "continue_after_s": 0.0 }
//!          or  { "history": [[time_s, pressure_pa, heat_transfer_coefficient_kg_per_m2_s,
//!                             flow_enthalpy_j_per_kg, radiative_heating_w_per_m2], ...],
//!                "surroundings_temperature_k": 266.48, "face_wears": true,
//!                "stream_gas": "air"
//!                           or { "oxygen_mass_fraction": 0.1675, "argon_mass_fraction": 0.276 } },
//!   "want": ["face_history", "thermocouple_histories", "final_profile"]
//! }
//! ```
//!
//! One other question is taken, for turning an arc-jet test into a heating
//! history: `{ "cold_wall_enthalpy_of": "air" or { ...stream gas... } }`
//! prints the enthalpy of that stream at a cold wall, J/kg.
//!
//! No outside library is used, so the JSON reader here is a small one: it
//! takes objects, arrays, numbers, strings without escapes, true and false.

use genesis_solvers::ablator_slab::{
    Answer, SlabQuestion, SlabReject, StreamGas, StreamRow, SurfaceCondition, Wanted, answer,
    stream_enthalpy_at_cold_wall_j_per_kg,
};
use std::collections::BTreeMap;
use std::io::Read;

#[derive(Debug, Clone)]
enum Json {
    Number(f64),
    Text(String),
    Flag(bool),
    List(Vec<Json>),
    Object(BTreeMap<String, Json>),
}

struct Reader<'a> {
    bytes: &'a [u8],
    at: usize,
}

impl Reader<'_> {
    fn space(&mut self) {
        while self.at < self.bytes.len() && self.bytes[self.at].is_ascii_whitespace() {
            self.at += 1;
        }
    }
    fn value(&mut self) -> Result<Json, String> {
        self.space();
        match self.bytes.get(self.at) {
            Some(b'{') => {
                self.at += 1;
                let mut object = BTreeMap::new();
                loop {
                    self.space();
                    if self.bytes.get(self.at) == Some(&b'}') {
                        self.at += 1;
                        return Ok(Json::Object(object));
                    }
                    let Json::Text(key) = self.value()? else {
                        return Err(format!("a name was expected at byte {}", self.at));
                    };
                    self.space();
                    if self.bytes.get(self.at) != Some(&b':') {
                        return Err(format!("':' was expected at byte {}", self.at));
                    }
                    self.at += 1;
                    object.insert(key, self.value()?);
                    self.space();
                    if self.bytes.get(self.at) == Some(&b',') {
                        self.at += 1;
                    }
                }
            }
            Some(b'[') => {
                self.at += 1;
                let mut list = Vec::new();
                loop {
                    self.space();
                    if self.bytes.get(self.at) == Some(&b']') {
                        self.at += 1;
                        return Ok(Json::List(list));
                    }
                    list.push(self.value()?);
                    self.space();
                    if self.bytes.get(self.at) == Some(&b',') {
                        self.at += 1;
                    }
                }
            }
            Some(b'"') => {
                let start = self.at + 1;
                let mut end = start;
                while self.bytes.get(end).is_some_and(|b| *b != b'"') {
                    end += 1;
                }
                self.at = end + 1;
                Ok(Json::Text(
                    String::from_utf8_lossy(&self.bytes[start..end.min(self.bytes.len())])
                        .into_owned(),
                ))
            }
            Some(_) => {
                let start = self.at;
                while self
                    .bytes
                    .get(self.at)
                    .is_some_and(|b| !b",]} \n\r\t".contains(b))
                {
                    self.at += 1;
                }
                let word =
                    std::str::from_utf8(&self.bytes[start..self.at]).map_err(|e| e.to_string())?;
                match word {
                    "true" => Ok(Json::Flag(true)),
                    "false" => Ok(Json::Flag(false)),
                    _ => word
                        .parse()
                        .map(Json::Number)
                        .map_err(|_| format!("{word:?} is not a number")),
                }
            }
            None => Err("the JSON ended early".to_string()),
        }
    }
}

fn number(object: &BTreeMap<String, Json>, name: &str) -> Result<f64, String> {
    match object.get(name) {
        Some(Json::Number(v)) => Ok(*v),
        _ => Err(format!("{name:?} is missing or not a number")),
    }
}

fn flag_or(object: &BTreeMap<String, Json>, name: &str, otherwise: bool) -> Result<bool, String> {
    match object.get(name) {
        None => Ok(otherwise),
        Some(Json::Flag(v)) => Ok(*v),
        _ => Err(format!("{name:?} is not true or false")),
    }
}

fn numbers(value: Option<&Json>, name: &str) -> Result<Vec<f64>, String> {
    let Some(Json::List(list)) = value else {
        return Err(format!("{name:?} is missing or not a list"));
    };
    list.iter()
        .map(|v| {
            if let Json::Number(x) = v {
                Ok(*x)
            } else {
                Err(format!("{name:?} holds something that is not a number"))
            }
        })
        .collect()
}

/// The stream's gas: the word "air", or its oxygen and argon by mass.
fn stream_gas(value: Option<&Json>, name: &str) -> Result<StreamGas, String> {
    match value {
        Some(Json::Text(word)) if word == "air" => Ok(StreamGas::AIR),
        Some(Json::Object(gas)) => Ok(StreamGas {
            oxygen_mass_fraction: number(gas, "oxygen_mass_fraction")?,
            argon_mass_fraction: number(gas, "argon_mass_fraction")?,
        }),
        _ => Err(format!(
            "{name:?} is missing: \"air\", or an object with \"oxygen_mass_fraction\" and \"argon_mass_fraction\" (the rest is nitrogen)"
        )),
    }
}

fn question(object: &BTreeMap<String, Json>) -> Result<(String, SlabQuestion, Wanted), String> {
    let Some(Json::Text(material)) = object.get("material") else {
        return Err("\"material\" is missing".to_string());
    };
    let Some(Json::Object(surface)) = object.get("surface") else {
        return Err("\"surface\" is missing".to_string());
    };
    let mut continue_after_s = 0.0;
    let surface = if surface.contains_key("history") {
        let Some(Json::List(rows)) = surface.get("history") else {
            return Err("\"history\" is not a list".to_string());
        };
        let mut history = Vec::new();
        for row in rows {
            let r = numbers(Some(row), "history")?;
            if r.len() != 5 {
                return Err("a row of \"history\" does not have five numbers".to_string());
            }
            history.push(StreamRow {
                time_s: r[0],
                pressure_pa: r[1],
                heat_transfer_coefficient_kg_per_m2_s: r[2],
                flow_enthalpy_j_per_kg: r[3],
                radiative_heating_w_per_m2: r[4],
            });
        }
        let stream_gas = stream_gas(surface.get("stream_gas"), "stream_gas")?;
        SurfaceCondition::GasStream {
            history,
            surroundings_temperature_k: number(surface, "surroundings_temperature_k")?,
            face_wears: flag_or(surface, "face_wears", true)?,
            stream_gas,
        }
    } else {
        continue_after_s = number(surface, "continue_after_s")?;
        SurfaceCondition::HeldTemperature {
            held_temperature_k: number(surface, "held_temperature_k")?,
            hold_duration_s: number(surface, "hold_duration_s")?,
        }
    };
    let mut wanted = Wanted::default();
    if let Some(Json::List(names)) = object.get("want") {
        for name in names {
            match name {
                Json::Text(word) if word == "face_history" => wanted.face_history = true,
                Json::Text(word) if word == "thermocouple_histories" => {
                    wanted.thermocouple_histories = true
                }
                Json::Text(word) if word == "final_profile" => wanted.final_profile = true,
                other => {
                    return Err(format!(
                        "\"want\" holds {other:?}; the names are face_history, thermocouple_histories, final_profile"
                    ));
                }
            }
        }
    }
    Ok((
        material.clone(),
        SlabQuestion {
            thickness_m: number(object, "thickness_m")?,
            start_temperature_k: number(object, "start_temperature_k")?,
            surface,
            continue_after_s,
            thermocouple_depths_m: numbers(
                object.get("thermocouple_depths_m"),
                "thermocouple_depths_m",
            )?,
        },
        wanted,
    ))
}

fn list(values: &[f64]) -> String {
    let parts: Vec<String> = values.iter().map(|v| format!("{v:?}")).collect();
    format!("[{}]", parts.join(","))
}

fn object(fields: &[(&str, String)]) -> String {
    let parts: Vec<String> = fields
        .iter()
        .map(|(name, value)| format!("\"{name}\":{value}"))
        .collect();
    format!("{{{}}}", parts.join(","))
}

fn print(a: &Answer) {
    let s = &a.summary;
    let n = |v: f64| format!("{v:?}");
    let summary = object(&[
        ("material", format!("{:?}", s.material)),
        (
            "material_compared_with_measurement",
            s.material_compared_with_measurement.to_string(),
        ),
        ("duration_s", n(s.duration_s)),
        ("wear_m", n(s.wear_m)),
        ("peak_face_temperature_k", n(s.peak_face_temperature_k)),
        (
            "peak_face_temperature_at_s",
            n(s.peak_face_temperature_at_s),
        ),
        ("final_face_temperature_k", n(s.final_face_temperature_k)),
        ("final_back_temperature_k", n(s.final_back_temperature_k)),
        (
            "depth_half_way_to_virgin_m",
            n(s.depth_half_way_to_virgin_m),
        ),
        (
            "depth_within_2_percent_of_virgin_m",
            n(s.depth_within_2_percent_of_virgin_m),
        ),
        ("heat_from_stream_j_per_m2", n(s.heat_from_stream_j_per_m2)),
        (
            "heat_absorbed_from_radiation_j_per_m2",
            n(s.heat_absorbed_from_radiation_j_per_m2),
        ),
        (
            "heat_radiated_away_j_per_m2",
            n(s.heat_radiated_away_j_per_m2),
        ),
        (
            "heat_taken_at_face_by_gases_j_per_m2",
            n(s.heat_taken_at_face_by_gases_j_per_m2),
        ),
        (
            "heat_in_through_face_j_per_m2",
            n(s.heat_in_through_face_j_per_m2),
        ),
        (
            "heat_stored_in_solid_j_per_m2",
            n(s.heat_stored_in_solid_j_per_m2),
        ),
        (
            "heat_taken_by_gas_j_per_m2",
            n(s.heat_taken_by_gas_j_per_m2),
        ),
        (
            "heat_given_off_by_decomposition_j_per_m2",
            n(s.heat_given_off_by_decomposition_j_per_m2),
        ),
        (
            "char_enthalpy_removed_j_per_m2",
            n(s.char_enthalpy_removed_j_per_m2),
        ),
        (
            "energy_account_residual_j_per_m2",
            n(s.energy_account_residual_j_per_m2),
        ),
        (
            "energy_account_tolerance_j_per_m2",
            n(s.energy_account_tolerance_j_per_m2),
        ),
        (
            "face_balance_residual_j_per_m2",
            n(s.face_balance_residual_j_per_m2),
        ),
        (
            "face_balance_tolerance_j_per_m2",
            n(s.face_balance_tolerance_j_per_m2),
        ),
        ("solid_mass_lost_kg_per_m2", n(s.solid_mass_lost_kg_per_m2)),
        ("gas_mass_out_kg_per_m2", n(s.gas_mass_out_kg_per_m2)),
        (
            "char_mass_removed_kg_per_m2",
            n(s.char_mass_removed_kg_per_m2),
        ),
        (
            "mass_account_residual_kg_per_m2",
            n(s.mass_account_residual_kg_per_m2),
        ),
        (
            "mass_account_tolerance_kg_per_m2",
            n(s.mass_account_tolerance_kg_per_m2),
        ),
        (
            "inside_arc_jet_comparison",
            s.inside_arc_jet_comparison.to_string(),
        ),
        (
            "inside_stardust_comparison",
            s.inside_stardust_comparison.to_string(),
        ),
        (
            "inside_thermocouple_block_comparison",
            s.inside_thermocouple_block_comparison.to_string(),
        ),
        ("cells", s.cells.to_string()),
        ("cells_at_end", s.cells_at_end.to_string()),
        ("time_steps", s.time_steps.to_string()),
        (
            "hardest_step_newton_passes",
            s.hardest_step_newton_passes.to_string(),
        ),
    ]);
    let mut fields = vec![("summary", summary)];
    if let Some(h) = &a.face_history {
        fields.push((
            "face_history",
            object(&[
                ("time_s", list(&h.time_s)),
                ("face_temperature_k", list(&h.face_temperature_k)),
                ("recession_m", list(&h.recession_m)),
                ("char_front_depth_m", list(&h.char_front_depth_m)),
                ("virgin_front_depth_m", list(&h.virgin_front_depth_m)),
                (
                    "gas_mass_flux_kg_per_m2_s",
                    list(&h.gas_mass_flux_kg_per_m2_s),
                ),
                (
                    "char_mass_flux_kg_per_m2_s",
                    list(&h.char_mass_flux_kg_per_m2_s),
                ),
            ]),
        ));
    }
    if let Some(histories) = &a.thermocouple_histories {
        let each: Vec<String> = histories
            .iter()
            .map(|h| {
                object(&[
                    ("depth_m", n(h.depth_m)),
                    ("passed_by_face", h.passed_by_face.to_string()),
                    ("covered_until_s", n(h.covered_until_s)),
                    ("time_s", list(&h.time_s)),
                    ("temperature_k", list(&h.temperature_k)),
                ])
            })
            .collect();
        fields.push(("thermocouple_histories", format!("[{}]", each.join(","))));
    }
    if let Some(p) = &a.final_profile {
        fields.push((
            "final_profile",
            object(&[
                ("depth_m", list(&p.depth_m)),
                ("temperature_k", list(&p.temperature_k)),
                ("density_kg_per_m3", list(&p.density_kg_per_m3)),
            ]),
        ));
    }
    println!("{}", object(&fields));
}

fn refused(refusal: &SlabReject) {
    println!(
        "{{\"refused\":{{\"name\":{:?},\"sentence\":{:?}}}}}",
        refusal.name(),
        refusal.to_string()
    );
}

fn main() {
    let mut text = String::new();
    match std::env::args().nth(1) {
        Some(path) => {
            text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{path}: {e}"))
        }
        None => {
            std::io::stdin()
                .read_to_string(&mut text)
                .expect("standard input");
        }
    }
    let parsed = Reader {
        bytes: text.as_bytes(),
        at: 0,
    }
    .value();
    if let Ok(Json::Object(object)) = &parsed {
        if object.contains_key("cold_wall_enthalpy_of") {
            let name = "cold_wall_enthalpy_of";
            match stream_gas(object.get(name), name) {
                Err(why) => {
                    println!("{{\"error\":{why:?}}}");
                    std::process::exit(2);
                }
                Ok(gas) => match stream_enthalpy_at_cold_wall_j_per_kg(&gas) {
                    Ok(h) => println!("{{\"stream_enthalpy_at_cold_wall_j_per_kg\":{h:?}}}"),
                    Err(refusal) => refused(&refusal),
                },
            }
            return;
        }
    }
    let asked = match parsed {
        Ok(Json::Object(object)) => question(&object),
        Ok(_) => Err("the question is not a JSON object".to_string()),
        Err(why) => Err(why),
    };
    match asked {
        Err(why) => {
            println!("{{\"error\":{why:?}}}");
            std::process::exit(2);
        }
        Ok((material, question, wanted)) => match answer(&material, &question, &wanted) {
            Ok(a) => print(&a),
            Err(refusal) => refused(&refusal),
        },
    }
}
