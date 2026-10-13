//! What one caller may see: a set of domain/card pairs and a set of solvers.
//!
//! The stdio door and the plain HTTP door pass the full view. The hosted door
//! passes the view of the key that made the request. Every function that
//! lists, describes, builds a schema or looks up a pair takes the view, so a
//! pair outside it takes the path of a pair the engine does not have.

use std::collections::BTreeSet;

use crate::descriptor::{self, Card};
use crate::solvers;

#[derive(Clone, Debug)]
pub struct View {
    /// `None` is every pair the engine serves.
    pairs: Option<BTreeSet<(String, String)>>,
    /// `None` is every solver the engine serves.
    solvers: Option<BTreeSet<String>>,
}

impl View {
    pub fn full() -> Self {
        Self {
            pairs: None,
            solvers: None,
        }
    }

    /// A view of the listed pairs and solvers. A name the engine does not
    /// serve is kept out of the view: it sees what the engine has.
    pub fn limited(
        pairs: impl IntoIterator<Item = (String, String)>,
        solvers: impl IntoIterator<Item = String>,
    ) -> Self {
        Self {
            pairs: Some(
                pairs
                    .into_iter()
                    .filter(|(domain, card)| descriptor::find(domain, card).is_some())
                    .collect(),
            ),
            solvers: Some(
                solvers
                    .into_iter()
                    .filter(|solver| solvers::is_known(solver))
                    .collect(),
            ),
        }
    }

    pub fn is_full(&self) -> bool {
        self.pairs.is_none() && self.solvers.is_none()
    }

    /// False for a pair this view hides. True for any pair of the full view,
    /// served or not: the lookup that follows decides.
    pub fn sees_pair(&self, domain: &str, card: &str) -> bool {
        match &self.pairs {
            None => true,
            Some(set) => set.contains(&(domain.to_string(), card.to_string())),
        }
    }

    pub fn sees_solver(&self, solver: &str) -> bool {
        match &self.solvers {
            None => solvers::is_known(solver),
            Some(set) => set.contains(solver),
        }
    }

    /// The pair itself when the view shows it, and a pair that no card has
    /// when it does not. A `match` on the result takes the arm of an unknown
    /// pair for a hidden one.
    pub fn resolve<'a>(&self, domain: &'a str, card: &'a str) -> (&'a str, &'a str) {
        if self.sees_pair(domain, card) {
            (domain, card)
        } else {
            ("", "")
        }
    }

    /// The pair's card, or nothing when the engine has no such card or the
    /// view hides it.
    pub fn find(&self, domain: &str, card: &str) -> Option<&'static Card> {
        if !self.sees_pair(domain, card) {
            return None;
        }
        descriptor::find(domain, card)
    }

    /// The cards of this view, in the engine's order.
    pub fn cards(&self) -> Vec<&'static Card> {
        descriptor::all()
            .iter()
            .filter(|item| self.sees_pair(item.domain, item.card))
            .collect()
    }

    /// The solvers of this view, in the engine's order.
    pub(crate) fn solvers(&self) -> Vec<&'static solvers::Solver> {
        solvers::ALL
            .iter()
            .copied()
            .filter(|solver| self.sees_solver(solver.name))
            .collect()
    }

    /// The names of the solvers of this view, in the engine's order.
    pub fn solver_names(&self) -> Vec<&'static str> {
        self.solvers().into_iter().map(|solver| solver.name).collect()
    }
}

impl Default for View {
    fn default() -> Self {
        Self::full()
    }
}
