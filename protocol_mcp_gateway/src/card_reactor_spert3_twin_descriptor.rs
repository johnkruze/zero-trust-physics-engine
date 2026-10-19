//! What an agent reads of the reactor_spert3_twin card: its fields and bounds, its step
//! floor and ceiling, its refusals word for word from the card's source, its
//! frame, its worked examples and its record. Stamped from the kit; a child
//! module of `descriptor.rs`, so it uses that file's own builders.

use genesis_kernel::reactor_spert3_twin;
use serde_json::Value;

use super::{Advisory, Card, Role, finish, obj, pair_examples_recorded, scalar, scalar_value, slots};

const DOMAIN: &str = "energy";
const CARD: &str = "reactor_spert3_twin";

/// What the card is, in one sentence of the list.
const SPECIMEN: &str = "The SPERT III E-core test reactor as one twin for all 70 tabulated tests of IDO-17281, cold startup, hot startup at 260 and 500 degF, hot standby and operating power: six-group kinetics, the transient rod's worth curve over its travel, radial conduction in the fuel, Doppler and moderator feedback, and the hot-spot clad temperature, fitted on all 70 tests; with each test left out of the fit in turn, peak power within 6.2 percent and time to peak within 3.0 percent at the median test.";

/// The accuracy of the card against IDO-17281, fitted on all 70 tests and
/// compared with each test left out of the fit. Written from
/// `evidence/research/reactor_spert3/twin/` (`stats.py`, `loo.py`).
pub(crate) const RESULT: &str = "Fitted on all 70 tests of IDO-17281, every test read when the structure was chosen: nine constants of the card and the three startup powers. In the fit, at the median test, peak power within 6.0 percent, energy to the peak within 7.9, time to the peak within 3.4 and the hot-spot clad temperature rise within 9.5; root mean square in the report's standard deviations 1.46, 2.78, 3.19 and 1.31; 67 of 70, 62 of 70, 41 of 66 and 62 of 69 tests within two. With each test left out of the fit in turn, the twelve numbers refitted on the other 69: 6.2, 7.8, 3.0 and 10.5 percent at the median, 1.47, 2.82, 3.37 and 1.36 standard deviations, and 67, 62, 43 and 61 tests within two. By series, left out of the fit, peak power, energy, time to the peak and clad at the median: cold startup (30 tests) 4.2, 4.4, 2.7 and 11.7 percent; 260 degF (15) 13.4, 10.1, 2.7 and 9.0, peak power high; 500 degF (17) 3.5, 6.4, 3.0 and 2.8; hot standby (5) 15.5 low, 13.7 low, 2.5 percent and 14 degF RMS in the clad maximum; operating power (3) 11.3 low, 81.8 low, 41.7 percent early and 12 degF. The operating-power gap is the largest: from 19 MW the tests reached broad maxima, timed on a hand-started clock. Test 52, scrammed at 20 s at 0.14 MW, reads 2.0 times its power. The four scrammed startup tests (24, 25, 52, 53) are compared at their printed time. The two worked answers through this card: the cold 1-dollar burst at 76 degF, 10.61 MW at 0.993 s with 2.17 MW s (Table III test 49 at 1.00 dollar: 11 MW, 0.97 s, 2.1 MW s), and hot standby test 81 at 1.17 dollars from 0.9 MW, 281 MW at 0.135 s with 6.85 MW s net (measured 330 MW, 0.135 s, 7.8 MW s). Record SHA-256 1f20ff51ef2ff50879731ea2a05451e3397bc6572585fd605871b5766eccbb56, with its per-test table SHA-256 8a30e1c4749aee218d3c9d1d4ce39f16b027c48786816ea9faaeae3887c747b5.";

pub(super) fn card() -> Card {
    let specimen = reactor_spert3_twin::ReactorSpert3Twin::e_core();
    let gate = specimen.gate_dt_s().expect("card gate");
    let floor = specimen.floor_dt_s().expect("card floor");
    finish(
        DOMAIN,
        CARD,
        SPECIMEN,
        vec![
            scalar(
                "inlet_temperature_k",
                Role::Phase,
                "K",
                Some(273.15),
                true,
                None,
                false,
                "the coolant at the core inlet at the start, above 273.15 K and below the saturation temperature at the system pressure (IAPWS-IF97), which a start at or above it is refused as StateOutsideCard. The 70 tests span 53 to 92 degF cold, 251 to 259 degF and 498 to 501 degF hot startup, and 502 to 513 degF hot standby and operating power: 284.8 to 540.4 K. The generation time, the Doppler and moderator coefficients, the rod worth curve, the gap width and the peak-to-average power density are the report's at 70, 250 and 500 degF, linear between them and continued along the nearer segment outside".to_string(),
            ),
            scalar(
                "system_pressure_mpa",
                Role::Phase,
                "MPa",
                Some(reactor_spert3_twin::PRESSURE_MIN_MPA),
                true,
                Some(reactor_spert3_twin::PRESSURE_MAX_MPA),
                true,
                "the system pressure, absolute, above 0.000611657 MPa and below 22.064 MPa, the triple and critical points of IAPWS-IF97 region 4. Tested at two: 0.0853 MPa, the atmosphere at the Idaho site, for cold startup, and 10.44 MPa, 1500 psig, for every hot test. It sets the saturation temperature, the nucleate-boiling superheat and the rod's acceleration".to_string(),
            ),
            scalar(
                "coolant_flow_m_per_s",
                Role::Phase,
                "m/s",
                Some(0.0),
                false,
                None,
                false,
                "the average coolant speed through the core, held for the run, zero or more. Tested with no flow cold, 0.73 to 6.71 m/s (2.4 to 22 ft/s) at 260 degF, 1.46 to 7.32 m/s (4.8 to 24 ft/s) at 500 degF and 4.27 m/s (14 ft/s) in hot standby and operating power. It sets the forced-convection coefficient and the sweep 2 v / L of the water; with no flow the water has no steady temperature and starts at the inlet's".to_string(),
            ),
            scalar(
                "initial_power_mw",
                Role::Phase,
                "MW",
                Some(0.0),
                true,
                None,
                false,
                "the steady power before the rod moves, above zero. Table VII gives it per test, 0.9 to 1.3 MW in hot standby and 19 MW at operating power. The startup tests of Tables III and V give none; the twin was fitted with 11.7719 W (1.17719e-5 MW) cold, 4.04032 W at 260 degF and 12.4711 W at 500 degF, against the report's about 50 W and 10 to 50 W. The start is the steady state at this power".to_string(),
            ),
            scalar(
                "reactivity_inserted_dollar",
                Role::Load,
                "dollar",
                Some(0.0),
                false,
                None,
                false,
                "the reactivity the transient rod is ejected through, zero or more; below zero is refused as ReactivityNegative. The rod starts at the height x_0 where the worth curve of Fig. B-4 holds this reactivity and falls at 2000 in/s^2 (1 + 0.0219966 p) from 2.46 ms after the clock's zero. Compared from 0.46 to 1.29 dollars: Tables III and V by the inhour value of each tested period, Table VII as printed. Past them the card computes the same rod and feedback. A run is one ejection with one reactivity: a segment with another is refused as ReactivityChangedDuringRun".to_string(),
            ),
        ],
        floor,
        "a locked 1e-9 s: the card's own constant",
        gate,
        "0.2 of the reduced generation time at 70 degF, 2.15 ms. The step itself refuses as DtAboveStateCeiling, the state kept, a step longer than 0.4 of the prompt time constant l* / |1 - rho| at the state it starts from or lands on, or 0.06 of it while rho is above one dollar: at this ceiling that binds below a net reactivity of -1.0 dollar, and above 1.30 dollars at 70 degF and 1.31 at 500 degF",
        None,
        "The four start fields are held for the run and the start is the steady reactor at the initial power: precursors at equilibrium, each fuel ring, the clad and the water at their steady temperatures. A run is one ejection of the transient rod: every segment carries the reactivity of the first, and another is refused before the first step as ReactivityChangedDuringRun. A run goes on for as long as it is asked: through the burst, its peak, and the clad's rise after it. The last state carries peak_power_mw, time_of_peak_s and energy_to_peak_mws, the highest landed power, when, and the energy released by then (energy_to_peak_mws minus initial_power_mw times time_of_peak_s is Table VII's net energy), and hot_clad_max_k with its time, the highest hot-spot clad temperature of the run. The report's clad temperatures were compared over the run to 2 s past the peak for startups and to 1 s for Table VII, where the rods were scrammed. A single step starts again from the start fields at each call.".to_string(),
        &[
            "NonFiniteInput",
            "NonFiniteState",
            "NonPositiveDt",
            "InvalidSpecimen",
            "ReactivityNegative",
            "ReactivityChangedDuringRun",
            "StateOutsideCard",
            "DtAboveStateCeiling",
            "FuelMelting",
            "WaterAtSaturation",
            "SpontaneousGeneration",
            "FirstLawBreach",
        ],
        vec![
            Advisory { name: "FLAG_ROD_MOVING", mask: reactor_spert3_twin::FLAG_ROD_MOVING, seals: true },
            Advisory { name: "FLAG_PAST_PEAK", mask: reactor_spert3_twin::FLAG_PAST_PEAK, seals: true },
            Advisory { name: "FLAG_HOT_SPOT_BOILING", mask: reactor_spert3_twin::FLAG_HOT_SPOT_BOILING, seals: true },
            Advisory { name: "FLAG_PROMPT_SUPERCRITICAL", mask: reactor_spert3_twin::FLAG_PROMPT_SUPERCRITICAL, seals: true },
            Advisory { name: "FLAG_OUTSIDE_COMPARED", mask: reactor_spert3_twin::FLAG_OUTSIDE_COMPARED, seals: true },
        ],
        slots(&[
            ("t", "Time since the clock's zero, the initiation of the transient, in seconds."),
            ("pos[0]", "power_mw"),
            ("pos[1]", "energy_released_mws"),
            ("pos[2]", "net_reactivity_dollar"),
            ("vel[0]", "fuel_average_k"),
            ("vel[1]", "hot_clad_k"),
            ("vel[2]", "water_k"),
            ("force_torque", "fuel_melt_margin_k"),
            ("residual", "Conservation residual."),
            (
                "flags",
                "FLAG_ROD_MOVING: the transient rod is moving. FLAG_PAST_PEAK: the power is below the run's highest. FLAG_HOT_SPOT_BOILING: the hot-spot clad is above saturation. FLAG_PROMPT_SUPERCRITICAL: net reactivity above one dollar. FLAG_OUTSIDE_COMPARED: a start field or the reactivity is outside the span of the 70 tests. All seal.",
            ),
            ("proof", "Not written by the step."),
        ]),
    )
}

/// One step at the ceiling, and the cold 1-dollar burst of Table III from
/// 76 degF to 3.0 s, 2 s past its peak, recorded every 100 steps.
pub(super) fn examples() -> (Value, Value) {
    let specimen = reactor_spert3_twin::ReactorSpert3Twin::e_core();
    pair_examples_recorded(
        DOMAIN,
        CARD,
        obj(&[
            ("inlet_temperature_k", scalar_value(crate::card_reactor_spert3_twin::REST_INLET_K)),
            ("system_pressure_mpa", scalar_value(crate::card_reactor_spert3_twin::REST_PRESSURE_MPA)),
            ("coolant_flow_m_per_s", scalar_value(0.0)),
            ("initial_power_mw", scalar_value(crate::card_reactor_spert3_twin::REST_POWER_MW)),
        ]),
        obj(&[("reactivity_inserted_dollar", scalar_value(1.0))]),
        super::card_dt(specimen.gate_dt_s()),
        6977,
        100,
    )
}

/// The record, in the five parts every card gives: the object, the data, the
/// rule it is held to, the result and the limits.
pub(super) fn validation(_specimen: &str) -> [String; 5] {
    [
        "The SPERT III E-core, a pressurised-water test reactor of 1392 UO2 rods (4.8 percent enriched, 10.5 g/cm3, 38.3 in active) at the National Reactor Testing Station, Idaho, measured by the Phillips Petroleum Company: power excursions from the ejection of its transient rod, cold at atmospheric pressure with no flow, and hot at 1500 psig with flow, from about 10 W, 1 MW and 19 MW.".to_string(),
        "IDO-17281 (March 1969), OSTI 4792676: Table III (30 cold-startup tests), Table V (32 hot-startup tests at 260 and 500 degF) and Table VII (5 hot-standby and 3 operating-power tests), with their standard deviations; the design data of Appendix A, the kinetics, gap, heat-source and rod data of Appendix B and Table II, and the rod worth curves of Fig. B-4, read at 37 points. Fink (2000) for UO2, IAPWS-IF97 for saturation, the NIST WebBook 10.44 MPa isobar for water. The twelve fitted numbers, as the card carries them: the rod's delay from the clock's zero, 0.00246118 s (2.5 ms); the rod's acceleration 2000 in/s2 times (1 + 0.0219966 p), p the system pressure in MPa; the Doppler weight F_D 0.954200 on the average fuel temperature rise, the Doppler coefficient at 70 degF as Table II prints it (0.72 cents per degF) and at 250 and 500 degF Table II's times 1.13261 and 1.48122 (0.56 and 0.37 cents per degF printed); the Doppler saturation kappa = 0.476328 times the peak-to-average power density (5.7, 5.2 and 3.6 at 70, 250 and 500 degF); the moderator weight 1.02792 on Table II's 0.4, 2.0 and 4.0 cents per degF; the hot rod at 1.40615 times the peak-to-average power density; free convection 3787.37 W/m2 K added to Dittus-Boelter's; and the start powers of the startup series, 11.7719 W cold, 4.04032 W at 260 degF and 12.4711 W at 500 degF. The rod's worth below x inches of travel, fitted to Fig. B-4: R(x) = s (x + x_c (exp(-x / x_c) - 1)), with s = 0.28995, 0.28964 and 0.18104 dollars per inch and x_c = 1.2363, 1.5458 and 1.9070 in at 70, 250 and 500 degF; at 1, 2, 3, 4, 6 and 8 in it gives 0.0911, 0.2925, 0.5431, 0.8154, 1.3840 and 1.9617 dollars at 70 degF, 0.0764, 0.2543, 0.4855, 0.7445, 1.2993 and 1.8719 at 250 degF, and 0.0402, 0.1378, 0.2695, 0.4213, 0.7558 and 1.1083 at 500 degF. Every quantity given at the three table temperatures is linear in the inlet temperature between them and continued along the nearer segment outside.".to_string(),
        "Every step holds two ledgers to 1e-6: the neutron population against the Hermite integral of rho P, and the energy released against the heat held in the rings, clad and water plus the heat the flow carried out, for the core and for the hot rod. Its accuracy is stated against the report's own standard deviations: peak power 15 percent (10 at 500 degF), energy to peak 17 (13), time to peak as tabulated, clad temperature 10 percent.".to_string(),
        RESULT.to_string(),
        "Compared from 284.8 to 540.4 K inlet, 0.0853 and 10.44 MPa, no flow to 7.32 m/s, 4e-6 to 19 MW initial power and 0.46 to 1.29 dollars; outside, the same equations run and FLAG_OUTSIDE_COMPARED is set. The startup tests give no initial power: the fitted 11.8, 4.0 and 12.5 W are effective values for the three series. Fuel temperature was never measured; the clad thermocouple is modelled as a hot rod at 1.406 times the peak-to-average power density, and Table VII's tests 79, 80, 84, 85 and 86 ran on a hand-started clock (IDO-17281 p. 148). The card stops by name where a hot-rod ring reaches 3120 K or the core water reaches saturation, and has no scram, void feedback or second burst.".to_string(),
    ]
}

/// Each refusal's sentence, word for word from the doc comment above the
/// variant in the card's source.
pub(super) fn meaning(name: &str) -> Option<&'static str> {
    Some(match name {
        "NonFiniteInput" => "The reactivity or `dt` was NaN or infinite.",
        "NonFiniteState" => "The incoming state, or the trial step, was not finite.",
        "NonPositiveDt" => "`dt` was zero or negative.",
        "InvalidSpecimen" => "A recorded constant is not this card's.",
        "ReactivityNegative" => "The reactivity is below zero: the transient rod is ejected, never driven in.",
        "ReactivityChangedDuringRun" => "The reactivity of the load is not the one the first step of this run sent the rod through; a run is one ejection.",
        "StateOutsideCard" => "The incoming state is not one this card writes: a power at or below zero, a temperature at or below zero, a clock below zero, or a start outside the card.",
        "DtAboveStateCeiling" => "The step is longer than 0.4 of the prompt time constant, or than the growth share of it while the prompt pole grows, at the state it starts from or lands on.",
        "FuelMelting" => "A ring of the hot rod reached the melting point of UO2, 3120 K.",
        "WaterAtSaturation" => "The water in the core reached its saturation temperature: bulk boiling, where the single-phase water of the card no longer holds.",
        "SpontaneousGeneration" => "The trial step created neutrons or heat from nothing.",
        "FirstLawBreach" => "The trial step destroyed neutrons or heat.",
        _ => return None,
    })
}
