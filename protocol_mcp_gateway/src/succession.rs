//! A served model that a newer one supersedes.
//!
//! An old model ID stays served unchanged, so every proof printed with it
//! still verifies: its request computes the same bytes. Where a newer model
//! of the same object is served, the old one's row in `list_cards` or
//! `list_solvers`, its `describe_card` or `describe_solver` (both forms) and
//! its `find_model` match carry `superseded_by`, the newer model's ID, and
//! `find_model` puts the newer model first, then the original under its own
//! ID. A row whose newer model is not served in the caller's view is not
//! said there.

/// One model superseded by another, by served names (a card's name or a
/// solver's).
pub struct Succession {
    /// The model kept served under its own ID.
    pub model: &'static str,
    /// The newer model of the same object.
    pub by: &'static str,
}

// ============================================================================
// FOR THE COMMANDER, AT MERGE: fill this table.
//
// Each row is { model: <old ID, kept served>, by: <new ID> }. A row is
// served only when both models are served; the test
// `every_row_names_two_served_models` fails on a row whose model or
// successor is not served, so uncomment a row only once its successor is
// merged. The rows planned for 0.14.3:
//
//     Succession { model: "reactor_spert3", by: "reactor_spert3_twin" },
//     Succession { model: "reactor_spert3_fuel", by: "reactor_spert3_twin" },
//     Succession { model: "reactor_spert3_ccore", by: "reactor_spert3_ccore_twin" },
// ============================================================================
pub const TABLE: &[Succession] = &[
    Succession { model: "reactor_spert3", by: "reactor_spert3_twin" },
    Succession { model: "reactor_spert3_fuel", by: "reactor_spert3_twin" },
    Succession { model: "reactor_spert3_ccore", by: "reactor_spert3_ccore_twin" },
];

/// The newer model that supersedes `model` in `table`, where both are served
/// in the view `served` describes.
pub fn superseded_by_in(table: &'static [Succession], model: &str, served: impl Fn(&str) -> bool) -> Option<&'static str> {
    table.iter().find(|row| row.model == model && row.by != model && served(row.by) && served(row.model)).map(|row| row.by)
}

/// The models `model` supersedes in `table`, where both are served.
pub fn supersedes_in(table: &'static [Succession], model: &str, served: impl Fn(&str) -> bool) -> Vec<&'static str> {
    table.iter().filter(|row| row.by == model && row.model != model && served(row.model) && served(row.by)).map(|row| row.model).collect()
}

/// Whether `view` serves a model by this name, card or solver.
pub fn served_in(view: &crate::view::View, model: &str) -> bool {
    view.cards().iter().any(|card| card.card == model) || view.sees_solver(model)
}

#[cfg(test)]
thread_local! {
    /// A table of stand-ins in place of [`TABLE`], for this thread's tests.
    static STAND_IN: std::cell::Cell<Option<&'static [Succession]>> = const { std::cell::Cell::new(None) };
}

/// Runs `body` with `table` in place of [`TABLE`] on this thread: the tests
/// of every answer that says `superseded_by`, before any successor exists.
#[cfg(test)]
pub fn with_table<T>(table: &'static [Succession], body: impl FnOnce() -> T) -> T {
    STAND_IN.with(|cell| cell.set(Some(table)));
    let out = body();
    STAND_IN.with(|cell| cell.set(None));
    out
}

/// The table in force: [`TABLE`], or a test's stand-ins.
pub fn table() -> &'static [Succession] {
    #[cfg(test)]
    if let Some(table) = STAND_IN.with(std::cell::Cell::get) {
        return table;
    }
    TABLE
}

/// The newer model that supersedes `model`, in `view`.
pub fn superseded_by(view: &crate::view::View, model: &str) -> Option<&'static str> {
    superseded_by_in(table(), model, |name| served_in(view, name))
}

/// The models `model` supersedes, in `view`.
pub fn supersedes(view: &crate::view::View, model: &str) -> Vec<&'static str> {
    supersedes_in(table(), model, |name| served_in(view, name))
}

/// The key every answer that says it uses.
pub const KEY: &str = "superseded_by";

/// Puts `superseded_by` into a row or a description of `model`, where the
/// served table names one in `view`.
pub fn into(map: &mut serde_json::Map<String, serde_json::Value>, view: &crate::view::View, model: &str) {
    if let Some(by) = superseded_by(view, model) {
        map.insert(KEY.to_string(), serde_json::json!(by));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The served table: each row names two different served models, and no
    /// model is superseded twice.
    #[test]
    fn every_row_names_two_served_models() {
        let view = crate::view::View::full();
        for (index, row) in TABLE.iter().enumerate() {
            assert_ne!(row.model, row.by, "a model does not supersede itself");
            assert!(served_in(&view, row.model), "{} is not served: an old ID stays served", row.model);
            assert!(served_in(&view, row.by), "{} is not served: uncomment its row once it is merged", row.by);
            assert!(TABLE[..index].iter().all(|other| other.model != row.model), "{} superseded twice", row.model);
        }
    }

    /// The mechanism, on a table of stand-ins: two served cards of one object.
    #[test]
    fn a_row_is_said_only_where_both_are_served() {
        static STAND_IN: &[Succession] = &[
            Succession { model: "reactor_spert3", by: "reactor_spert3_fuel" },
            Succession { model: "cell_2170_m80", by: "not_served_anywhere" },
        ];
        let view = crate::view::View::full();
        let served = |name: &str| served_in(&view, name);
        assert_eq!(superseded_by_in(STAND_IN, "reactor_spert3", served), Some("reactor_spert3_fuel"));
        assert_eq!(supersedes_in(STAND_IN, "reactor_spert3_fuel", served), vec!["reactor_spert3"]);
        assert_eq!(superseded_by_in(STAND_IN, "reactor_spert3_fuel", served), None);
        assert_eq!(superseded_by_in(STAND_IN, "cell_2170_m80", served), None, "a successor not served is not said");
        assert!(supersedes_in(STAND_IN, "not_served_anywhere", served).is_empty());
    }
}
