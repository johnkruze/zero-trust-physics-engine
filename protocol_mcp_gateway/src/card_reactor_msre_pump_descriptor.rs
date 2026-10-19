//! What an agent reads of the reactor_msre_pump card: its fields and bounds, its step
//! floor and ceiling, its refusals word for word from the card's source, its
//! frame, its worked examples and its record. Stamped from the kit; a child
//! module of `descriptor.rs`, so it uses that file's own builders.
//!
//! The description skeleton. Every sentence here is served, so each keeps
//! the rules a description is held to (`tests/conformance_support`): the
//! company's own specimen is named so, never by what it lacks; a range is
//! written in plain words, "0 to 933.47 K"; no word "check"; and where the
//! model is compared with measurements, the record states the range compared
//! on and the result there in numbers.

use genesis_kernel::reactor_msre_pump;
use serde_json::Value;

use super::{Advisory, Card, Role, exact_value, finish, obj, pair_examples_of, scalar, scalar_value, slots};

const DOMAIN: &str = "energy";
const CARD: &str = "reactor_msre_pump";

/// What the card is, in one sentence of the list.
const SPECIMEN: &str = "The Molten-Salt Reactor Experiment (Oak Ridge National Laboratory) at zero power with its 1965-66 U-235 fuel salt, held critical while the fuel pump starts or coasts down: the reactivity the control rods supply, rod 1 moved by its drive at 0.526 in/s, as the delayed-neutron precursors are carried out of the core and round the 25.2 s loop with the salt. Two constants fitted on ORNL-4233: steady circulation loss 0.2120 % dk/k against the measured 0.212; rod reactivity within 23.9 pcm root mean square on the start-up's 48 readings, fitted on, and 6.5 pcm on the coast-down's 73 readings, left out of the fit.";

/// The result, its first sentence the word it is served under.
const RESULT: &str = "Two constants fitted on the ORNL-4233 record, all of it read when the form was chosen: the same equations with the constants read from the sources alone had been compared with the whole record first. Steady circulation loss at full flow 0.2120 % dk/k against the measured 0.212 +- 0.004 (the flux shape's height fitted on it). The rod drive's speed, 0.526 in/s, is rod 1's own position on the start-up (Fig. 24, 3.5 to 13.2 s), on 0.05432 % dk/k per inch of its worth over that travel: 0.02857 % dk/k a second, not fitted to the compensation. Rod reactivity through the start-up, 48 readings from 0.1 to 46.1 s: root mean square 23.9 pcm, mean +7.7 pcm, on the first 12 s within 18.2 pcm, largest 53.9 pcm at 17.1 s, after the rod's peak (the fuel-flow lag, 2.05 s, fitted on these readings with the drive in place). Through the coast-down, 73 readings from 0.06 to 70 s, left out of the fit: root mean square 6.5 pcm, mean +2.3 pcm, largest 19.7 pcm. Largest ledger residual on both runs 2.4e-7, under the card's limit of 2e-5. Before the drive was carried, the lag, 2.55 s, stood in for it: 24.8 pcm on the start-up, largest 55.4 at 17.1 s, and 8.1 pcm on the coast-down. With the constants read from the sources alone (the 79 in effective core, no lag, no drive), the same equations gave 0.2546 % dk/k, 72.4 pcm and 36.7 pcm.";

/// What the card does not show, and where its numbers stop.
const LIMITS: &str = "Zero power: the servo holds 10 W, there is no temperature or power feedback, and the answer does not depend on the power. One axial shape for flux and importance, plug flow in every region, one fuel flow for the whole loop, no radial variation, no helium bubbles and no servo dead band; rod 1 carries one worth per inch over its travel. The fuel flow's 2.05 s lag behind the pump is fitted: the rod drive alone, with no lag, leaves the start-up at 33.3 pcm. After the rod's peak the card's precursor wave round the loop runs larger than the record's: 53.9 pcm high at 17.1 s and 40.6 pcm low at 27.9 s. Fuel flow is taken to follow the pump speed: ORNL-4233 has no fuel flowmeter. The rod readings are Fig. 24 digitised and turned into reactivity by the integral of rod 1's differential worth (Fig. 5) over 1.087, the report's normalisation to the final loading; without that normalisation the readings are 8.7 percent larger. Compared range: starts with the pump stopped or at full speed, the pump speed rising no faster than 1.3 and falling no faster than 0.81 of full speed a second, 70 s from the start. Outside it a step lands with FLAG_OUTSIDE_COMPARED_RUNS, the same equations carried on. The card stops at a pump speed or a fuel flow below 0 or above full speed (FlowOutsideCard). Record SHA-256 ebeed609852caa38211c507b2bee1cf330c167c5554bec85a3e850916eaf9090 (rod_drive_2026-10-18/ROD_DRIVE.md), with its comparison SHA-256 4b9fa338d5af52a36be6431af907b87d4547a9aa84f8deda59cd4a4192b825fa.";

pub(super) fn card() -> Card {
    let specimen = reactor_msre_pump::ReactorMsrePump::specimen();
    let gate = specimen.gate_dt_s().expect("specimen gate");
    let floor = specimen.floor_dt_s().expect("specimen floor");
    finish(
        DOMAIN,
        CARD,
        SPECIMEN,
        vec![
            scalar(
                "pump_speed_at_start",
                Role::Phase,
                "1",
                Some(0.0),
                false,
                Some(1.0),
                false,
                "0 to 1, the fuel-pump speed as a fraction of full speed at the start: the run starts from the equilibrium at that steady speed, static at 0 and circulating above it. The runs compared start at 0 (a start-up) and 1 (a coast-down); any other start lands with FLAG_OUTSIDE_COMPARED_RUNS on every step. Below 0 or above full speed the pump does not run (FlowOutsideCard)".to_string(),
            ),
            scalar(
                "pump_speed_fraction",
                Role::Load,
                "1",
                Some(0.0),
                false,
                Some(1.0),
                false,
                "0 to 1, the fuel-pump speed as a fraction of full speed at the end of the step, linear within the step from the speed before it. A rise faster than 1.3 or a fall faster than 0.81 of full speed a second (the fastest of the start-up and coast-down records, rounded up) lands with FLAG_OUTSIDE_COMPARED_RUNS. Below 0 or above full speed the pump does not run (FlowOutsideCard)".to_string(),
            ),
        ],
        floor,
        "a locked 1e-6 s, a step under which the salt moves less than one part in a hundred thousand of a parcel",
        gate,
        "0.05 s, half of one 0.1 s parcel's passage at full flow",
        None,
        "The load is the pump speed at the end of each step, linear within the step. The fuel flow follows it with a first-order lag of 2.05 s, solved exactly over the step; the salt's travel takes the fuel flow linear within the step between its two ends. The salt is 252 parcels of 0.1 s of residence at full flow round the 25.2 s loop (lower plenum 3.8 s, graphite region 9.4 s, upper plenum 3.9 s, external system 8.1 s). Six delayed-neutron groups (ORNL-4233 Table 2) are born in each parcel by the axial flux sin(pi (z + 6.3 in) / 91.6 in) and emit with the same weight; the salt asks of the rods rho = sum of beta_i less the weighted emission, in % dk/k, and rod_compensation_percent_dk_k is rho less its value at the start; rod_reactivity_percent_dk_k, what the rods supply, follows it each step by no more than the drive's 0.526 in/s on 0.05432 % dk/k per inch, 0.02857 % dk/k a second, and a step at that speed carries FLAG_ROD_AT_DRIVE_SPEED. Classical fourth-order Runge-Kutta; the ledger, per group, books born, decayed in the vessel and decayed outside it, and holds inventory - anchor - born + decayed to 2e-5 of the group's inventory. Worked: from the pump stopped, the pump brought to full speed in 1 s and held (the worked run is its first 20 s), the rods run at the drive's speed from 4.65 to 11.8 s, and the rod reactivity peaks at 0.275 % dk/k at 15.3 s, as the salt that held no precursors fills the core, and is 0.220 at 70 s, on its way to the steady loss 0.212; on the start-up schedule of Fig. 22 the same, against a measured peak of 0.274 at 13.0 s; from full speed, the coast-down of Fig. 23 takes it to -0.196 % dk/k at 70 s, the drive keeping up throughout".to_string(),
        &[
            "NonFiniteInput",
            "NonFiniteState",
            "NonPositiveDt",
            "InvalidSpecimen",
            "FlowOutsideCard",
            "NegativeState",
            "InconsistentState",
            "SpontaneousGeneration",
            "FirstLawBreach",
        ],
        vec![
            Advisory { name: "FLAG_FLOW_CHANGING", mask: reactor_msre_pump::FLAG_FLOW_CHANGING, seals: true },
            Advisory { name: "FLAG_FUEL_CIRCULATING", mask: reactor_msre_pump::FLAG_FUEL_CIRCULATING, seals: true },
            Advisory { name: "FLAG_OUTSIDE_COMPARED_RUNS", mask: reactor_msre_pump::FLAG_OUTSIDE_COMPARED_RUNS, seals: true },
            Advisory { name: "FLAG_ROD_AT_DRIVE_SPEED", mask: reactor_msre_pump::FLAG_ROD_AT_DRIVE_SPEED, seals: true },
        ],
        slots(&[
            ("t", "Time since the start of the run, in seconds."),
            ("pos[0]", "rod_reactivity_percent_dk_k"),
            ("pos[1]", "circulation_loss_percent_dk_k"),
            ("pos[2]", "fuel_flow_fraction"),
            ("vel[0]", "pump_speed_fraction"),
            ("vel[1]", "loss_at_start_percent_dk_k"),
            ("vel[2]", "phase_s"),
            ("force_torque", "fuel_flow_rate_per_s"),
            ("residual", "Conservation residual: the largest group's precursor balance over its inventory."),
            (
                "flags",
                "FLAG_FLOW_CHANGING when the fuel flow changed during the step; FLAG_FUEL_CIRCULATING when the salt moves at its end; FLAG_OUTSIDE_COMPARED_RUNS when the step lies outside the runs compared with the record; FLAG_ROD_AT_DRIVE_SPEED when the rods ran at their drive's speed, short of what the salt asks. All seal.",
            ),
            ("proof", "Not written by the step."),
        ]),
    )
}

/// One step of a start-up, and a start-up: the pump brought from stopped to
/// full speed in 1 s, 0.05 of full speed a step, then held, 20 s in all.
pub(super) fn examples() -> (Value, Value) {
    let specimen = reactor_msre_pump::ReactorMsrePump::specimen();
    let dt = exact_value(specimen.gate_dt_s().expect("specimen gate"));
    let (step, mut run) = pair_examples_of(
        DOMAIN,
        CARD,
        obj(&[("pump_speed_at_start", scalar_value(0.0))]),
        obj(&[("pump_speed_fraction", scalar_value(0.05))]),
        dt,
        1,
    );
    let mut segments = Vec::new();
    for n in 1..=20u64 {
        segments.push(obj(&[("steps", Value::Number(1u64.into())), ("load", obj(&[("pump_speed_fraction", scalar_value(n as f64 * 0.05))]))]));
    }
    segments.push(obj(&[("steps", Value::Number(380u64.into())), ("load", obj(&[("pump_speed_fraction", scalar_value(1.0))]))]));
    run["segments"] = Value::Array(segments);
    run["record_every"] = Value::Number(20u64.into());
    (step, run)
}

/// The record, in the five parts every card gives: the object, the data, the
/// fit, the result and the limits.
pub(super) fn validation(_specimen: &str) -> [String; 5] {
    [
        "The Molten-Salt Reactor Experiment at Oak Ridge National Laboratory, zero-power experiments of 1965-66 with U-235 fuel salt: the fuel pump started from stopped and coasted down from full speed while the flux servo held the reactor critical at 10 W.".into(),
        "B. E. Prince, J. R. Engel, S. J. Ball, P. N. Haubenreich and T. W. Kerlin, ORNL-4233 (February 1968): the measured reactivity effect of circulation, 0.212 +- 0.004 % dk/k (section 3.6); the fuel-pump speed through the start-up and the coast-down (Figs. 22 and 23) and the position of rod 1 through both, 48 and 73 readings (Fig. 24), digitised, the rod's position turned into reactivity by its differential worth (Fig. 5) over 1.087 (Fig. 6); the delayed-neutron constants (Table 2) and the vessel's elevations (Fig. 2). B. E. Prince, ORNL-TM-1626 (October 1966), Table 1: the residence times round the loop. Read from the record, not fitted: the rod drive's speed, 0.526 in/s, from rod 1's position on the start-up (Fig. 24, 3.5 to 13.2 s), and its worth over that travel, 0.05432 % dk/k per inch. Fitted: the flux shape's height, 91.6 in, on the steady loss; the fuel-flow lag, 2.05 s, on the 48 start-up readings with the drive in place. Left out of the fit: the 73 coast-down readings.".into(),
        "Each comparison is the card through the engine at 0.05 s steps on the digitised pump-speed schedules: the steady loss against 0.212 % dk/k, and the root mean square of the card's rod reactivity less the measured over each series' readings, in pcm (1e-5 dk/k).".into(),
        RESULT.into(),
        LIMITS.into(),
    ]
}

/// Each refusal's sentence, word for word from the doc comment above the
/// variant in the card's source.
pub(super) fn meaning(name: &str) -> Option<&'static str> {
    Some(match name {
        "NonFiniteInput" => "`dt` or the pump speed was NaN or infinite.",
        "NonFiniteState" => "The incoming state, or the trial step, was not finite.",
        "NonPositiveDt" => "`dt` was zero or negative.",
        "InvalidSpecimen" => "A recorded constant is not this reactor's.",
        "FlowOutsideCard" => "The pump speed or the fuel flow is below zero or above the pump's full speed, incoming, in the load or landing.",
        "NegativeState" => "The incoming state holds a precursor content or a ledger term below zero.",
        "InconsistentState" => "The incoming state is not one this card writes: at time zero it is not the card's start; later, an anchor or a derived field is not what the card computes from the rest of the state.",
        "SpontaneousGeneration" => "The trial step created precursors beyond what the ledger supplies.",
        "FirstLawBreach" => "The trial step destroyed precursors the ledger supplies.",
        _ => return None,
    })
}
