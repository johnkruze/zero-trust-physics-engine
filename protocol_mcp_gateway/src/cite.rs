//! `cite`: how to cite the people whose data a card or a solver stands on,
//! and the paper, as text and as BibTeX.
//!
//! The entries are `data/citations.json`, compiled in: one entry for each
//! served name (`domain/card` for a card, the solver's name for a solver) and
//! one for the paper. A unit test holds its keys to the served cards and
//! solvers exactly. The tool computes no physics and takes no permit.

use std::sync::OnceLock;

use serde_json::{Map, Value, json};

use crate::live::DoorError;
use crate::view::View;

const CITATIONS: &str = include_str!("../data/citations.json");

/// The paper's address, said in the tool's description.
pub const PAPER_DOI_URL: &str = "https://doi.org/10.5281/zenodo.23305697";

const KEYS: &[&str] = &["domain", "card", "solver"];

/// What each source of an entry gives a reader who cites it: who, what,
/// when, where, and what the source itself asks of those who use its data.
/// No licence and no word on passing data on: licences are named in the
/// repository's NOTICE. The compiled file holds public facts only. The
/// company's working notes (what was read to confirm a reference, whether a
/// data author has been written to) are kept outside the crate, keyed the
/// same way, and a unit test holds the compiled file free of them.
const SOURCE_KEYS: &[&str] = &[
    "role",
    "authors",
    "title",
    "year",
    "publisher_or_venue",
    "identifier",
    "url",
    "requested_acknowledgment",
    "citation_text",
    "bibtex",
];

const ABOUT: &str = "Cite Zero-Trust Physics: the service, then the paper. After them, data lists the people whose measurements this model stands on, each with its own citation. text and bibtex hold every entry in that order, ready to paste. requested_acknowledgment is what a source itself asks of those who use its data, and is absent where it asks nothing.";

const ABOUT_THE_SERVICE: &str = "How to cite the service itself: service is its name, who made it and where it is served, with a line ready to paste, and paper is the paper. For an answer, give also the card or solver, the engine_version the answer carries, its cryptographic proof (run_seal or solution_seal) and the date you asked. cite with domain and card, or solver, adds the people whose measurements that model stands on.";

/// Where an agent writes the date it asked, in the service's citation.
const DATE_ASKED: &str = "[the date you asked]";

/// The service itself, for a citation: its name, who made it, where it is
/// served and the engine version its answers carry, as fields and as a line
/// and a BibTeX entry ready to paste. `engine_version` is the version the
/// answers of the model asked about carry: the kernel's for a card, the
/// solvers' for a solver, and both when no model was named.
fn service(engine_version: &[String]) -> Value {
    use crate::identity::{ADDRESS, COMPANY, EMAIL, MAKER, NAME, SITE, TITLE};
    let versions = engine_version.join(" and ");
    json!({
        "name": NAME,
        "what": "a physics verifier for Super Intelligence",
        "maker": MAKER,
        "publisher": COMPANY,
        "address": ADDRESS,
        "site": SITE,
        "paper": PAPER_DOI_URL,
        "contact": EMAIL,
        "engine_version": versions,
        "citation_text": format!("{MAKER} ({COMPANY}). {TITLE}. Served at {ADDRESS}, engine {versions}. Accessed {DATE_ASKED}."),
        "bibtex": format!("@misc{{zerotrustphysics_service,\n  author       = {{Kruze, John}},\n  title        = {{{TITLE}}},\n  organization = {{{COMPANY}}},\n  howpublished = {{\\url{{{ADDRESS}}}}},\n  note         = {{Engine {versions}. Accessed {DATE_ASKED}.}}\n}}"),
    })
}

fn table() -> &'static Value {
    static TABLE: OnceLock<Value> = OnceLock::new();
    TABLE.get_or_init(|| serde_json::from_str(CITATIONS).expect("data/citations.json is JSON"))
}

/// Every key of the table's `models`, in the file's order.
#[cfg(test)]
pub fn model_keys() -> Vec<&'static str> {
    table()["models"]
        .as_object()
        .map(|models| models.keys().map(String::as_str).collect())
        .unwrap_or_default()
}

pub fn description() -> String {
    format!(
        "Gives how to cite a card or a solver: the service itself first (Zero-Trust Physics, its maker and its address) and the paper ({PAPER_DOI_URL}), then the people whose measurements the model stands on (authors, title, year, address, and what each asks of those who use its data), each as text and BibTeX. With domain and card, or solver, the answer is for that model; with neither, it is how to cite the service itself: its name, who made it, its address and the paper."
    )
}

pub fn schema_in(view: &View) -> Map<String, Value> {
    let _ = view;
    match json!({
        "type": "object",
        "properties": {
            "domain": { "type": "string", "description": "With card: a card, by the name find_model or list_cards gives; the two give the same names." },
            "card": { "type": "string" },
            "solver": { "type": "string", "description": "In place of domain and card: a solver, by the name find_model or list_solvers gives." }
        },
        "additionalProperties": false
    }) {
        Value::Object(map) => map,
        _ => unreachable!(),
    }
}

fn violation(text: impl Into<String>) -> DoorError {
    DoorError::Unprocessable(text.into())
}

fn text_of<'a>(object: &'a Map<String, Value>, key: &str) -> Result<Option<&'a str>, DoorError> {
    match object.get(key) {
        None => Ok(None),
        Some(Value::String(text)) => Ok(Some(text.as_str())),
        Some(_) => Err(violation(format!("Physical Violation: '{key}' must be a string."))),
    }
}

fn paper() -> Value {
    let paper = &table()["paper"];
    json!({
        "authors": paper["authors"],
        "title": paper["title"],
        "year": paper["year"],
        "version": paper["version"],
        "publisher_or_venue": paper["publisher_or_venue"],
        "identifier": paper["identifier"],
        "url": paper["url"],
        "citation_text": paper["citation_text"],
        "bibtex": paper["bibtex"],
    })
}

/// The answer for one served name.
pub fn cite_in(view: &View, request: &Value) -> Result<Value, DoorError> {
    let object = request
        .as_object()
        .ok_or_else(|| violation("Physical Violation: the request must be a JSON object."))?;
    for key in object.keys() {
        if !KEYS.contains(&key.as_str()) {
            return Err(violation(format!(
                "Physical Violation: '{key}' is not a key of cite; send domain and card, or solver."
            )));
        }
    }
    let domain = text_of(object, "domain")?;
    let card = text_of(object, "card")?;
    let solver = text_of(object, "solver")?;
    let (key, named) = match (domain, card, solver) {
        (Some(domain), Some(card), None) => {
            let spec = view
                .find(domain, card)
                .ok_or_else(|| DoorError::NotFound(crate::CARD_NOT_RECOGNIZED.to_string()))?;
            (format!("{}/{}", spec.domain, spec.card), json!({ "domain": spec.domain, "card": spec.card }))
        }
        (None, None, Some(solver)) => {
            if !view.sees_solver(solver) || !crate::solvers::is_known(solver) {
                return Err(DoorError::NotFound(crate::SOLVER_NOT_RECOGNIZED.to_string()));
            }
            (solver.to_string(), json!({ "solver": solver }))
        }
        (None, None, None) => {
            let paper = paper();
            let service = service(&[crate::ENGINE_VERSION.to_string(), crate::solvers::engine_version()]);
            let mut answer = Map::new();
            answer.insert(
                "text".to_string(),
                json!(format!("{}\n\n{}", service["citation_text"].as_str().unwrap_or(""), paper["citation_text"].as_str().unwrap_or(""))),
            );
            answer.insert(
                "bibtex".to_string(),
                json!(format!("{}\n\n{}", service["bibtex"].as_str().unwrap_or(""), paper["bibtex"].as_str().unwrap_or(""))),
            );
            answer.insert("service".to_string(), service);
            answer.insert("paper".to_string(), paper);
            answer.insert("about".to_string(), json!(ABOUT_THE_SERVICE));
            return Ok(Value::Object(answer));
        }
        (Some(_), None, None) => return Err(violation("Missing required parameter: 'card' (InputMissing).")),
        (None, Some(_), None) => return Err(violation("Missing required parameter: 'domain' (InputMissing).")),
        _ => {
            return Err(violation(
                "Physical Violation: send domain and card for a card, or solver for a solver, not both.",
            ));
        }
    };
    let entry = table()["models"].get(&key).ok_or_else(|| {
        DoorError::Internal(format!("Physical Violation: no citation entry is compiled in for {key}."))
    })?;
    let paper = paper();
    let service = service(&[if solver.is_some() { crate::solvers::engine_version() } else { crate::ENGINE_VERSION.to_string() }]);
    let mut data = Vec::new();
    let mut texts = Vec::new();
    let mut bibtex = Vec::new();
    // The service and the paper come first, then the sources in the entry's order.
    for entry in [&service, &paper] {
        if let Some(text) = entry["citation_text"].as_str() {
            texts.push(text.to_string());
        }
        if let Some(text) = entry["bibtex"].as_str() {
            bibtex.push(text.to_string());
        }
    }
    for source in entry["sources"].as_array().into_iter().flatten() {
        let mut row = Map::new();
        for name in SOURCE_KEYS {
            if let Some(value) = source.get(*name).filter(|value| !value.is_null()) {
                row.insert((*name).to_string(), value.clone());
            }
        }
        if let Some(text) = source["citation_text"].as_str() {
            texts.push(text.to_string());
        }
        if let Some(text) = source["bibtex"].as_str() {
            bibtex.push(text.to_string());
        }
        data.push(Value::Object(row));
    }
    let mut answer = Map::new();
    for (name, value) in named.as_object().expect("an object") {
        answer.insert(name.clone(), value.clone());
    }
    answer.insert("service".to_string(), service);
    answer.insert("paper".to_string(), paper);
    answer.insert("real_thing".to_string(), entry["real_thing"].clone());
    if let Some(how) = entry.get("how_used").filter(|value| !value.is_null()) {
        answer.insert("how_used".to_string(), how.clone());
    }
    answer.insert("data".to_string(), Value::Array(data));
    answer.insert("text".to_string(), json!(texts.join("\n\n")));
    answer.insert("bibtex".to_string(), json!(bibtex.join("\n\n")));
    answer.insert("about".to_string(), json!(ABOUT));
    Ok(Value::Object(answer))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The compiled file ships with the source: it holds what the tool
    /// serves and no working note. Working notes are kept outside the crate.
    #[test]
    fn the_compiled_file_holds_public_facts_only() {
        for word in ["/Users/", "Gmail", "BOARD", "command-center", "evidence/", "outreach", "author_contact", "evidence_folder", "engine_data_statement", "\"verified\"", "\"notes\"", "\"basis\"", "\"status\"", ".md", " disk", "the disk"] {
            assert!(!CITATIONS.contains(word), "data/citations.json holds {word}");
        }
        for (key, entry) in table()["models"].as_object().unwrap() {
            let mut fields: Vec<&str> = entry.as_object().unwrap().keys().map(String::as_str).collect();
            fields.sort_unstable();
            assert_eq!(fields, ["cite_also", "how_used", "kind", "not_held_to_a_measurement", "real_thing", "sources"], "{key}");
            for source in entry["sources"].as_array().unwrap() {
                for field in source.as_object().unwrap().keys() {
                    assert!(SOURCE_KEYS.contains(&field.as_str()), "{key}: {field}");
                }
            }
        }
    }

    /// A citation says who to cite and where their work is. It holds no
    /// licence and no word on whether data may be passed on, for a source,
    /// for the paper or in what the tool says of itself; what a source asks
    /// of those who use its data stays, in the source's own words.
    #[test]
    fn a_citation_holds_no_licence_and_no_word_on_passing_data_on() {
        for word in ["\"licence\"", "\"license\"", "may_redistribute", "by_redistribution", "CC BY", "CC-BY", "CC0", "Creative Commons", "Apache License", "MIT License"] {
            assert!(!CITATIONS.contains(word), "data/citations.json holds {word}");
        }
        let view = View::full();
        let grid = cite_in(&view, &json!({ "domain": "energy", "card": "grid_gb" })).unwrap();
        for source in grid["data"].as_array().unwrap() {
            assert!(source.get("licence").is_none() && source.get("may_redistribute_measurements").is_none(), "{source}");
        }
        assert_eq!(grid["data"][0]["requested_acknowledgment"], "Supported by National Energy SO Open Data");
        assert!(grid["paper"].get("licence").is_none());
        for text in [ABOUT, ABOUT_THE_SERVICE, description().as_str()] {
            assert!(!text.contains("licence") && !text.contains("redistribut"), "{text}");
        }
        // A source that asks nothing carries no acknowledgment.
        let lhd = cite_in(&view, &json!({ "solver": "fusion_pb11_lhd" })).unwrap();
        assert!(lhd["data"][0].get("requested_acknowledgment").is_none() && lhd["data"][1].get("requested_acknowledgment").is_none());
    }

    #[test]
    fn the_entries_are_the_served_cards_and_solvers_exactly() {
        let mut served: Vec<String> = crate::descriptor::all()
            .iter()
            .map(|card| format!("{}/{}", card.domain, card.card))
            .chain(crate::solvers::ALL.iter().map(|solver| solver.name.to_string()))
            .filter(|name| name != crate::solver_probe::STAND_IN.name)
            .collect();
        served.sort();
        let mut keyed: Vec<String> = model_keys().into_iter().map(str::to_string).collect();
        keyed.sort();
        assert_eq!(keyed, served, "data/citations.json must hold one entry for each served card and solver and no other");
        assert!(table()["paper"]["url"].as_str() == Some(PAPER_DOI_URL));
    }

    #[test]
    fn every_entry_answers_with_the_service_then_the_paper_then_its_sources() {
        let view = View::full();
        let paper_text = table()["paper"]["citation_text"].as_str().unwrap();
        // The order of text, of bibtex and of the answer's own keys.
        let opens = |answer: &Value, name: &str| {
            let service_text = answer["service"]["citation_text"].as_str().unwrap().to_string();
            let text = answer["text"].as_str().unwrap();
            assert!(text.starts_with(&format!("{service_text}\n\n{paper_text}")), "{name}: {text}");
            let bibtex = answer["bibtex"].as_str().unwrap();
            assert!(bibtex.starts_with("@misc{zerotrustphysics_service"), "{name}");
            let (service_at, paper_at) = (bibtex.find("@misc{zerotrustphysics_service").unwrap(), bibtex.find("@misc{kruze2026zerotrustphysics").unwrap());
            assert!(service_at < paper_at, "{name}");
            let keys: Vec<&str> = answer.as_object().unwrap().keys().map(String::as_str).collect();
            let at = |key: &str| keys.iter().position(|k| *k == key).unwrap();
            assert!(at("service") < at("paper") && at("paper") < at("data") && at("data") < at("text"), "{name}: {keys:?}");
        };
        for card in crate::descriptor::all() {
            let answer = cite_in(&view, &json!({ "domain": card.domain, "card": card.card })).unwrap();
            opens(&answer, card.card);
            let sources = answer["data"].as_array().unwrap();
            assert_eq!(answer["text"].as_str().unwrap().split("\n\n").count(), sources.len() + 2, "{}", card.card);
            if let Some(last) = sources.last() {
                assert!(answer["text"].as_str().unwrap().ends_with(last["citation_text"].as_str().unwrap()), "{}", card.card);
            }
            for source in sources {
                assert!(source["citation_text"].is_string() && source["bibtex"].is_string(), "{}: {source}", card.card);
                assert!(source.get("verified").is_none() && source.get("notes").is_none());
            }
            assert!(answer.get("author_contact").is_none());
        }
        for solver in crate::solvers::ALL.iter().filter(|solver| solver.name != crate::solver_probe::STAND_IN.name) {
            let answer = cite_in(&view, &json!({ "solver": solver.name })).unwrap();
            opens(&answer, solver.name);
        }
    }

    /// stress_plate_hole_s235's own record says its comparison is not blind
    /// in the held-back ring (the solver's validation data, and
    /// stress_plate_s235/validation_protocol.md, "What was seen before this
    /// protocol was fixed"): its entry says so, and its companion's names it
    /// as the scored original, never as compared blind.
    #[test]
    fn the_stress_plate_entries_say_what_its_record_says_of_the_ring() {
        let view = View::full();
        let original = cite_in(&view, &json!({ "solver": "stress_plate_hole_s235" })).unwrap();
        let companion = cite_in(&view, &json!({ "solver": "stress_plate_hole_s235_elastic" })).unwrap();
        let said = original["how_used"].as_str().unwrap();
        assert!(said.contains("The comparison is not blind in that ring"), "{said}");
        let said = companion["how_used"].as_str().unwrap();
        assert!(said.contains("stress_plate_hole_s235, the scored original, compared before this companion was built and not blind in that ring by its own record"), "{said}");
        assert!(!CITATIONS.contains("stress_plate_hole_s235, compared blind"));
    }

    /// Three entries said what their records contradict and the served
    /// validations have mended since 0.13.1 (AUDIT.md of 2026-10-16, defects
    /// 4, 7 and 10): cantilever_eth's next 12 s were not kept unread, their
    /// coarse envelope having been printed; reactor_triga_jsi's held-back
    /// records were fetched, after the scoring; ln2_line_dlr_fttf's marks were
    /// committed to the engine's own record, not posted.
    #[test]
    fn the_entries_say_what_the_mended_validations_say() {
        let view = View::full();
        let how = |request: Value| cite_in(&view, &request).unwrap()["how_used"].as_str().unwrap().to_string();
        let cantilever = how(json!({ "domain": "signal", "card": "cantilever_eth" }));
        assert!(cantilever.contains("their coarse envelope on Laser_1 had been printed by mistake on 2026-10-06"), "{cantilever}");
        let triga = how(json!({ "domain": "energy", "card": "reactor_triga_jsi" }));
        assert!(triga.contains("15 of those records were fetched on 2026-10-11, after the scoring"), "{triga}");
        let line = how(json!({ "solver": "ln2_line_dlr_fttf" }));
        assert!(line.contains("in the engine's own record and not in the public pass-marks repository"), "{line}");
        for words in ["kept unread until the card was compared", "whose records were never fetched", "marks posted before they were opened"] {
            assert!(!CITATIONS.contains(words), "{words}");
        }
    }
}
