//! The model IDs no longer served, each with the ID its object is served
//! under.
//!
//! From engine 0.15.0 the engine serves one model for each real object,
//! under the object's original ID: a better model of the same object
//! replaces the earlier one under that ID, and its seals move. The IDs the
//! newer models were first served under, and the IDs of earlier models of an
//! object served beside them, are not served. A request that names one, as
//! a card or as a solver, anywhere in its arguments, is refused before any
//! work by name (`RetiredModel`), with the ID that now serves its object.

use serde_json::Value;

/// One retired ID and the ID its object is served under.
pub struct Retired {
    pub id: &'static str,
    pub served_as: &'static str,
}

/// Every retired ID. The cards first, then the solver.
pub const TABLE: &[Retired] = &[
    Retired { id: "reactor_spert3_twin", served_as: "reactor_spert3" },
    Retired { id: "reactor_spert3_fuel", served_as: "reactor_spert3" },
    Retired { id: "reactor_spert3_ccore_twin", served_as: "reactor_spert3_ccore" },
    Retired { id: "reactor_spert3_ccore_plate", served_as: "reactor_spert3_ccore" },
    Retired { id: "grid_gb_twin", served_as: "grid_gb" },
    Retired { id: "cell_2170_twin", served_as: "cell_2170_model3" },
    Retired { id: "cell_2170_m80", served_as: "cell_2170_model3" },
    Retired { id: "tokamak_jet_dte1_twin", served_as: "tokamak_jet_dte1" },
    Retired { id: "runaway_18650_lco_twin", served_as: "runaway_18650_lco" },
    Retired { id: "blast_wave_tnt_air_twin", served_as: "blast_wave_tnt_air" },
];

/// The name of the refusal.
pub const NAME: &str = "RetiredModel";

/// The ID that serves the object `id` was a model of, when `id` is retired.
pub fn served_as(id: &str) -> Option<&'static str> {
    TABLE.iter().find(|row| row.id == id).map(|row| row.served_as)
}

/// The refusal of a request that names `id` as a card (`kind` "Card") or a
/// solver (`kind` "Solver").
pub fn sentence(kind: &str, id: &str, served_as: &str) -> String {
    format!(
        "{kind} not served under this ID: {id} is served as {served_as} since engine 0.15.0, one model for each real object; send {served_as} ({NAME})."
    )
}

/// The refusal of a request whose arguments name a retired ID under `card`
/// or `solver`, at any depth (a chained request's upstream, a plan's head, a
/// search's request), or `None`.
pub fn refusal(arguments: &Value) -> Option<String> {
    match arguments {
        Value::Object(map) => {
            for (key, kind) in [("card", "Card"), ("solver", "Solver")] {
                if let Some(id) = map.get(key).and_then(Value::as_str) {
                    if let Some(by) = served_as(id) {
                        return Some(sentence(kind, id, by));
                    }
                }
            }
            map.values().find_map(refusal)
        }
        Value::Array(items) => items.iter().find_map(refusal),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    /// Each retired ID names a served model, is not served itself, and is
    /// named once.
    #[test]
    fn every_retired_id_names_a_served_model_and_is_not_served() {
        let view = crate::view::View::full();
        let served = |name: &str| view.cards().iter().any(|card| card.card == name) || view.sees_solver(name);
        for (index, row) in TABLE.iter().enumerate() {
            assert!(!served(row.id), "{} is still served", row.id);
            assert!(served(row.served_as), "{} is not served", row.served_as);
            assert!(TABLE[..index].iter().all(|other| other.id != row.id), "{} named twice", row.id);
        }
    }

    /// A retired ID is refused wherever the arguments name it.
    #[test]
    fn a_retired_id_is_refused_by_name_with_the_id_that_serves_it() {
        let said = refusal(&json!({ "domain": "energy", "card": "grid_gb_twin", "initial": {} })).unwrap();
        assert_eq!(said, "Card not served under this ID: grid_gb_twin is served as grid_gb since engine 0.15.0, one model for each real object; send grid_gb (RetiredModel).");
        let chained = json!({ "solver": "ablator_slab", "parameters": {}, "from": { "request": { "domain": "energy", "card": "reactor_spert3_fuel" } } });
        assert!(refusal(&chained).unwrap().contains("reactor_spert3_fuel is served as reactor_spert3"));
        assert!(refusal(&json!({ "solver": "blast_wave_tnt_air_twin", "parameters": {} })).unwrap().starts_with("Solver not served under this ID"));
        assert_eq!(refusal(&json!({ "domain": "energy", "card": "grid_gb", "initial": {} })), None);
    }
}
