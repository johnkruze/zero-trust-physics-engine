//! The table of couplings: one entry each. A coupling names one card whose
//! sealed run hands values on, one card that takes them, and for each value
//! where it is read in the upstream run, where it stands in the downstream
//! request, its unit, and the downstream card's declared range for it.
//!
//! A coupling is served only where both pieces were compared with
//! measurement on the same object and the value handed is one both pieces
//! take with the same meaning. No physics is added here: a value is carried
//! bit for bit, never converted. A value outside the downstream card's
//! declared range is refused as `CouplingOutsideRange` before anything of the
//! downstream piece runs.
//!
//! Served: the SPERT III E-core burst between the two SPERT III cards, both
//! compared with IDO-17281 (Phillips Petroleum Company, 1969). What is handed
//! is the test itself, the inlet water temperature at the start and the
//! reactivity inserted, so the two cards' answers are sealed as answers about
//! one burst. No coupling hands a computed field on yet: the capsule_stardust
//! card publishes one stagnation-point convective heat flux, while
//! ablator_slab's Stardust stream takes wall pressure, a heat-transfer
//! coefficient, flow enthalpy and radiative heating at the core, which the
//! card does not compute; turning one into the others would be physics
//! added in the coupling, and the slab's Stardust comparison was made with
//! NASA's heating, not the card's. (Besides: the card's compared entry
//! reaches 12,573.44 m/s, a kinetic enthalpy of 79.05 MJ/kg, above the
//! slab's compared 78.31 MJ/kg in the first part of the entry.)

use genesis_kernel::{reactor_spert3, reactor_spert3_fuel};

/// Where a handed value is read in the upstream run: `final_state.<name>`
/// (its sealed final value) or `inputs.initial.<name>` /
/// `inputs.segments[*].load.<name>` (its sealed request, every segment
/// holding the same value).
pub(crate) struct Mapping {
    pub from: &'static str,
    /// Where it stands in a downstream run request: `initial.<name>` or
    /// `segments[*].load.<name>`. A step request takes it at
    /// `parameters.<name>`.
    pub to: &'static str,
    pub unit: &'static str,
    /// `Some(sentence)` when the value is outside the downstream card's
    /// declared range, given the values already handed by this coupling.
    pub outside: fn(f64, &[(&'static str, f64)]) -> Option<String>,
}

pub(crate) struct Coupling {
    pub name: &'static str,
    pub upstream: (&'static str, &'static str),
    pub downstream: (&'static str, &'static str),
    pub mappings: &'static [Mapping],
    /// What is handed and why the pairing is served, in one sentence.
    pub meaning: &'static str,
}

fn outside_span(value: f64, min: f64, max: f64, unit: &str, what: &str) -> Option<String> {
    if value >= min && value <= max {
        None
    } else {
        Some(format!("{value} {unit} is outside {what}, {min} to {max} {unit}"))
    }
}

fn spert3_coolant(value: f64, _: &[(&'static str, f64)]) -> Option<String> {
    outside_span(
        value,
        reactor_spert3::COOLANT_TEMPERATURE_MIN_K,
        reactor_spert3::COOLANT_TEMPERATURE_MAX_K,
        "K",
        "reactor_spert3's declared coolant temperature (56 to 92 degF)",
    )
}

fn spert3_reactivity(value: f64, _: &[(&'static str, f64)]) -> Option<String> {
    outside_span(
        value,
        reactor_spert3::REACTIVITY_MIN_DOLLAR,
        reactor_spert3::REACTIVITY_MAX_DOLLAR,
        "dollars",
        "reactor_spert3's declared reactivity",
    )
}

fn fuel_band(inlet_k: f64) -> Option<usize> {
    (0..3).find(|&band| {
        inlet_k >= reactor_spert3_fuel::BAND_INLET_MIN_K[band] && inlet_k <= reactor_spert3_fuel::BAND_INLET_MAX_K[band]
    })
}

fn fuel_inlet(value: f64, _: &[(&'static str, f64)]) -> Option<String> {
    match fuel_band(value) {
        Some(_) => None,
        None => Some(format!(
            "{value} K is in none of reactor_spert3_fuel's three inlet bands, {:?} to {:?} K",
            reactor_spert3_fuel::BAND_INLET_MIN_K,
            reactor_spert3_fuel::BAND_INLET_MAX_K
        )),
    }
}

fn fuel_reactivity(value: f64, handed: &[(&'static str, f64)]) -> Option<String> {
    let inlet = handed
        .iter()
        .find(|(to, _)| *to == "initial.inlet_temperature_k")
        .map(|(_, value)| *value)?;
    let band = fuel_band(inlet)?;
    outside_span(
        value,
        reactor_spert3_fuel::BAND_REACTIVITY_MIN_DOLLAR[band],
        reactor_spert3_fuel::BAND_REACTIVITY_MAX_DOLLAR[band],
        "dollars",
        "reactor_spert3_fuel's declared reactivity in the band of the handed inlet temperature",
    )
}

pub(crate) static COUPLINGS: &[Coupling] = &[
    Coupling {
        name: "spert3_burst_to_fuel_card",
        upstream: ("energy", "reactor_spert3"),
        downstream: ("energy", "reactor_spert3_fuel"),
        mappings: &[
            Mapping {
                from: "final_state.coolant_temperature_k",
                to: "initial.inlet_temperature_k",
                unit: "K",
                outside: fuel_inlet,
            },
            Mapping {
                from: "inputs.segments[*].load.reactivity_inserted_dollar",
                to: "segments[*].load.reactivity_inserted_dollar",
                unit: "dollars",
                outside: fuel_reactivity,
            },
        ],
        meaning: "One SPERT III E-core burst (IDO-17281) handed from the point-kinetics card to the card with fuel and water temperature: the water temperature the burst started at and the reactivity inserted, each carried bit for bit. Both cards were compared with the same report's tests; the coupling carries the test, not a computed field.",
    },
    Coupling {
        name: "spert3_fuel_burst_to_point_card",
        upstream: ("energy", "reactor_spert3_fuel"),
        downstream: ("energy", "reactor_spert3"),
        mappings: &[
            Mapping {
                from: "final_state.inlet_temperature_k",
                to: "initial.coolant_temperature_k",
                unit: "K",
                outside: spert3_coolant,
            },
            Mapping {
                from: "inputs.segments[*].load.reactivity_inserted_dollar",
                to: "segments[*].load.reactivity_inserted_dollar",
                unit: "dollars",
                outside: spert3_reactivity,
            },
        ],
        meaning: "One SPERT III E-core burst (IDO-17281) handed from the card with fuel and water temperature to the point-kinetics card: the inlet water temperature and the reactivity inserted, each carried bit for bit. It carries the test, not a computed field: the inlet temperature is the start value, held unchanged through the run. The point-kinetics card was compared on the cold-startup tests only, so a hot-startup burst is refused as CouplingOutsideRange.",
    },
];

pub(crate) fn find(name: &str) -> Option<&'static Coupling> {
    COUPLINGS.iter().find(|coupling| coupling.name == name)
}

/// The couplings a card hands from or takes, as `describe_card` states
/// them under `couplings`: only those whose two cards the view holds, so a
/// card outside the view is not named.
pub(crate) fn table_in(view: &crate::view::View, domain: &str, card: &str) -> serde_json::Value {
    serde_json::Value::Array(
        COUPLINGS
            .iter()
            .filter(|coupling| coupling.upstream == (domain, card) || coupling.downstream == (domain, card))
            .filter(|coupling| {
                view.find(coupling.upstream.0, coupling.upstream.1).is_some()
                    && view.find(coupling.downstream.0, coupling.downstream.1).is_some()
            })
            .map(|coupling| {
                serde_json::json!({
                    "name": coupling.name,
                    "upstream": format!("{}/{}", coupling.upstream.0, coupling.upstream.1),
                    "downstream": format!("{}/{}", coupling.downstream.0, coupling.downstream.1),
                    "handed": coupling.mappings.iter().map(|mapping| serde_json::json!({
                        "from": mapping.from,
                        "to": mapping.to,
                        "unit": mapping.unit,
                    })).collect::<Vec<_>>(),
                    "meaning": coupling.meaning,
                })
            })
            .collect(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_coupling_names_cards_the_engine_serves_and_fields_they_take() {
        for coupling in COUPLINGS {
            let up = crate::descriptor::find(coupling.upstream.0, coupling.upstream.1).expect("upstream card");
            let down = crate::descriptor::find(coupling.downstream.0, coupling.downstream.1).expect("downstream card");
            for mapping in coupling.mappings {
                if let Some(name) = mapping.from.strip_prefix("inputs.initial.") {
                    assert!(up.phase_names.contains(&name), "{}", mapping.from);
                } else if let Some(name) = mapping.from.strip_prefix("inputs.segments[*].load.") {
                    assert!(up.load_names.contains(&name), "{}", mapping.from);
                } else {
                    assert!(mapping.from.starts_with("final_state."), "{}", mapping.from);
                }
                if let Some(name) = mapping.to.strip_prefix("initial.") {
                    assert!(down.phase_names.contains(&name), "{}", mapping.to);
                } else {
                    let name = mapping.to.strip_prefix("segments[*].load.").expect("a load path");
                    assert!(down.load_names.contains(&name), "{}", mapping.to);
                }
            }
            assert_eq!(COUPLINGS.iter().filter(|other| other.name == coupling.name).count(), 1);
        }
    }

    #[test]
    fn a_value_outside_the_downstream_range_is_named_and_one_inside_is_not() {
        let cold = reactor_spert3::COOLANT_TEMPERATURE_MIN_K;
        assert!(fuel_inlet(cold, &[]).is_none());
        assert!(fuel_inlet(400.0, &[]).is_some(), "between the fuel card's bands");
        let hot = reactor_spert3_fuel::BAND_INLET_MIN_K[2];
        assert!(spert3_coolant(hot, &[]).is_some());
        assert!(spert3_coolant(cold, &[]).is_none());
        let handed = [("initial.inlet_temperature_k", cold)];
        assert!(fuel_reactivity(reactor_spert3::REACTIVITY_MAX_DOLLAR, &handed).is_none());
        assert!(fuel_reactivity(reactor_spert3::REACTIVITY_MAX_DOLLAR.next_up(), &handed).is_some());
        assert!(spert3_reactivity(reactor_spert3_fuel::BAND_REACTIVITY_MAX_DOLLAR[2], &[]).is_some());
        // Every value reactor_spert3 admits, the fuel card admits.
        for t in [reactor_spert3::COOLANT_TEMPERATURE_MIN_K, reactor_spert3::COOLANT_TEMPERATURE_MAX_K] {
            assert!(fuel_inlet(t, &[]).is_none(), "{t}");
        }
    }
}
