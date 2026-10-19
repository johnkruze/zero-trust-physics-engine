// The tests of `cell_2170_twin`, a child module of the card.

extern crate core;
extern crate std;

use core::prelude::v1::*;
use std::assert;
use std::assert_eq;
use std::format;
use std::include_str;
use std::matches;
use std::vec::Vec;

use super::*;
use crate::frame::LastStateFrame64;

::core::include!("cell_2170_twin_reference.rs");

/// Largest difference from the Python model the tests allow in a voltage, a temperature in
/// kelvin, or a state of charge. The card's `exp` and `log` are the kernel's, the Python's the
/// platform's; both are double precision.
const AGREEMENT: f64 = 1.0e-12;
/// The same for work and heat, as a share of the value.
const ENERGY_AGREEMENT: f64 = 1.0e-12;

fn blank_frame() -> LastStateFrame64 {
    LastStateFrame64 { t: -1.0, pos: [1.0, 2.0, 3.0], vel: [4.0, 5.0, 6.0], force_torque: 7.0, residual: 8.0, flags: 9, proof: [0xA5; 16] }
}

fn load(current_a: f64, case_temperature_c: f64) -> CellLoad {
    CellLoad { current_a, case_temperature_c }
}

fn rested(cell: &Cell2170Twin, cell_number: f64, state_of_charge: f64, capacity_ah: f64) -> CellState {
    CellState::try_rested(cell, cell_number, state_of_charge, capacity_ah).unwrap()
}

fn series(segments: &[(usize, f64, f64, f64)]) -> Vec<(f64, f64)> {
    let mut out = Vec::new();
    for &(seconds, current_a, start_c, slope) in segments {
        for k in 0..seconds {
            out.push((current_a, start_c + slope * (k as f64 + 1.0)));
        }
    }
    out
}

/// Held current at a held case temperature until a step is refused; the landed steps and the refusal.
fn hold(cell: &Cell2170Twin, start: CellState, current_a: f64, case_c: f64, most: usize) -> (CellState, usize, Option<StepReject>) {
    let mut state = start;
    let mut frame = blank_frame();
    for k in 0..most {
        if let Err(reject) = step(cell, &mut state, &mut frame, load(current_a, case_c), 1.0) {
            return (state, k, Some(reject));
        }
    }
    (state, most, None)
}

fn close(a: f64, b: f64, tolerance: f64, what: &str) {
    assert!((a - b).abs() <= tolerance * (1.0 + b.abs()), "{what}: card {a:e}, reference {b:e}");
}

#[test]
fn the_reference_states_are_reproduced() {
    let cell = Cell2170Twin::cell();
    for case in CASES {
        let mut state = rested(&cell, case.cell_number, case.state_of_charge, case.capacity_ah);
        let mut frame = blank_frame();
        let mut next = 0;
        for (current_a, case_c) in series(case.segments) {
            step(&cell, &mut state, &mut frame, load(current_a, case_c), 1.0).unwrap();
            if next < case.states.len() && state.t_s == case.states[next].t_s {
                let want = &case.states[next];
                close(state.terminal_v, want.terminal_v, AGREEMENT, "terminal voltage");
                close(state.state_of_charge, want.state_of_charge, AGREEMENT, "state of charge");
                close(state.surface_state_of_charge, want.surface_state_of_charge, AGREEMENT, "surface");
                close(state.reaction_overpotential_v, want.reaction_overpotential_v, AGREEMENT, "reaction");
                close(state.near_full_overpotential_v, want.near_full_overpotential_v, AGREEMENT, "near full");
                close(state.hysteresis_v, want.hysteresis_v, AGREEMENT, "hysteresis");
                close(state.electrolyte_v, want.electrolyte_v, AGREEMENT, "electrolyte");
                close(state.electrolyte_drop_v, want.electrolyte_drop_v, AGREEMENT, "electrolyte drop");
                close(state.current_mean_a, want.current_mean_a, AGREEMENT, "running mean");
                close(state.core_temperature_c, want.core_temperature_c, AGREEMENT, "core");
                close(state.diffusion_lag[0], want.diffusion_first, AGREEMENT, "first mode");
                close(state.diffusion_lag[DIFFUSION_MODES - 1], want.diffusion_last, AGREEMENT, "last mode");
                close(state.electrical_work_j, want.work_j, ENERGY_AGREEMENT, "work");
                close(state.heat_j, want.heat_j, ENERGY_AGREEMENT, "heat");
                close(state.heat_stored_j, want.heat_stored_j, ENERGY_AGREEMENT, "heat stored");
                close(state.heat_to_case_j, want.heat_to_case_j, ENERGY_AGREEMENT, "heat to the case");
                assert!(state.residual_conservation_error <= 1.0e-13);
                next += 1;
            }
        }
        assert_eq!(next, case.states.len(), "every reference state was met");
    }
}

#[test]
fn constants_are_locked_and_a_changed_copy_is_not_the_cell() {
    let cell = Cell2170Twin::cell();
    assert!(cell.is_valid());
    assert_eq!(FITTED.len(), FITTED_NUMBERS);
    assert!(FITTED_FREE <= FITTED_NUMBERS);
    for index in 0..FITTED_NUMBERS {
        let mut changed = cell;
        changed.fitted[index] = f64::from_bits(changed.fitted[index].to_bits() ^ 1);
        assert!(!changed.is_valid(), "fitted number {index}");
        let mut state = rested(&cell, 80.0, 0.9, 4.5);
        let mut frame = blank_frame();
        assert_eq!(step(&changed, &mut state, &mut frame, load(1.0, 25.0), 1.0), Err(StepReject::InvalidCell));
    }
    for index in 0..COMPARED_RANGE_COUNT {
        let mut changed = cell;
        changed.compared[index] += 1.0;
        assert!(!changed.is_valid(), "compared range {index}");
    }
    let mut changed = cell;
    changed.cells[1].resistance_scale *= 1.0 + 1.0e-15;
    assert!(!changed.is_valid());
    let mut changed = cell;
    changed.rest_v_per_k[CURVE_POINTS / 2] += 1.0e-9;
    assert!(!changed.is_valid());
}

#[test]
fn the_cells_are_three_rows_and_another_number_is_refused() {
    let cell = Cell2170Twin::cell();
    assert_eq!(CELL_COUNT, 3);
    let names: Vec<&str> = CELLS.iter().map(|row| row.name).collect();
    assert_eq!(names, ["m80", "m1000", "m448-N"]);
    for row in CELLS {
        assert!(row.reference_capacity_ah > 4.6 && row.reference_capacity_ah < 4.7);
        assert!(row.charge_return_ratio > 0.99 && row.charge_return_ratio < 1.0);
        assert!(row.resistance_scale > 0.5 && row.resistance_scale < 1.5);
    }
    assert!(CellState::try_rested(&cell, 81.0, 0.9, 4.5).is_none());
    let mut state = rested(&cell, 80.0, 0.9, 4.5);
    state.cell_number = 2.0;
    let mut frame = blank_frame();
    assert_eq!(step(&cell, &mut state, &mut frame, load(1.0, 25.0), 1.0), Err(StepReject::CellNotRecognized));
}

#[test]
fn every_refusal_is_named_and_leaves_state_and_frame_as_they_were() {
    let cell = Cell2170Twin::cell();
    let start = rested(&cell, 80.0, 0.5, 4.5);
    let cases: [(CellState, CellLoad, f64, StepReject); 8] = [
        (start, load(f64::NAN, 25.0), 1.0, StepReject::NonFiniteInput),
        (start, load(1.0, f64::INFINITY), 1.0, StepReject::NonFiniteInput),
        (start, load(1.0, 25.0), 0.0, StepReject::NonPositiveDt),
        (start, load(1.0, -273.15), 1.0, StepReject::CaseTemperatureBelowAbsoluteZero),
        (CellState { capacity_ah: 0.0, ..start }, load(1.0, 25.0), 1.0, StepReject::CapacityNotPositive),
        (CellState { state_of_charge: 1.06, ..start }, load(1.0, 25.0), 1.0, StepReject::StateOfChargeOutsideCard),
        (CellState { hysteresis_v: f64::NAN, ..start }, load(1.0, 25.0), 1.0, StepReject::NonFiniteState),
        (
            CellState { case_span_known: true, case_temperature_least_c: 30.0, case_temperature_most_c: 20.0, ..start },
            load(1.0, 25.0),
            1.0,
            StepReject::CaseTemperatureSpanInverted,
        ),
    ];
    for (state, step_load, dt, want) in cases {
        let mut moved = state;
        let mut frame = blank_frame();
        assert_eq!(step(&cell, &mut moved, &mut frame, step_load, dt), Err(want));
        assert!(moved == state || (moved.hysteresis_v.is_nan() && state.hysteresis_v.is_nan()), "{want:?} moved the state");
        assert!(frame.t == -1.0 && frame.pos == [1.0, 2.0, 3.0] && frame.vel == [4.0, 5.0, 6.0] && frame.flags == 9, "{want:?} moved the frame");
    }
}

#[test]
fn a_full_discharge_ends_at_the_lower_limit_with_both_ledgers_closed() {
    let cell = Cell2170Twin::cell();
    for case_c in [40.0, 25.0, 0.0, -10.0, -20.0] {
        let (state, steps, reject) = hold(&cell, rested(&cell, 80.0, 1.0, 4.25), 4.5, case_c, 10_000);
        assert_eq!(reject, Some(StepReject::UndervoltageLimit), "{case_c} degC");
        assert!(steps > 300 && steps < 3_900, "{case_c} degC: {steps} s");
        assert!(state.residual_conservation_error <= 1.0e-12, "{case_c} degC: {:e}", state.residual_conservation_error);
        assert!(state.heat_j > 0.0 && state.heat_stored_j > 0.0 && state.heat_to_case_j > 0.0);
    }
}

#[test]
fn the_core_warms_above_the_case_by_its_own_heat_and_more_in_the_cold() {
    let cell = Cell2170Twin::cell();
    let rise = |case_c: f64| {
        let (state, _, _) = hold(&cell, rested(&cell, 80.0, 1.0, 4.25), 4.5, case_c, 900);
        assert!(state.core_temperature_c > case_c);
        state.core_temperature_c - case_c
    };
    let warm = rise(25.0);
    let cold = rise(-20.0);
    assert!(cold > warm, "the cold cell makes more heat at the same current: {cold} K against {warm} K");
    // at rest the core comes back to the case
    let (state, _, _) = hold(&cell, rested(&cell, 80.0, 1.0, 4.25), 4.5, -20.0, 600);
    let (rested_state, _, _) = hold(&cell, state, 0.0, -20.0, 20_000);
    assert!((rested_state.core_temperature_c + 20.0).abs() < 1.0e-3);
    assert!(rested_state.residual_conservation_error <= 1.0e-12);
}

#[test]
fn a_rested_start_takes_its_core_from_the_case_of_its_first_step() {
    let cell = Cell2170Twin::cell();
    let mut state = rested(&cell, 448.0, 0.8, 4.4);
    assert!(!state.core_known);
    let mut frame = blank_frame();
    step(&cell, &mut state, &mut frame, load(0.0, -15.0), 1.0).unwrap();
    assert!(state.core_known);
    assert_eq!(state.core_temperature_c, -15.0, "no current, no heat: the core is the case");
    assert_eq!(frame.vel[1], -15.0_f32);
}

#[test]
fn the_salt_runs_out_under_a_current_the_cold_cannot_carry() {
    let cell = Cell2170Twin::cell();
    // well past anything compared: a held 30 A at -40 degC. Either the salt at one electrode runs
    // out, or the voltage reaches the lower limit first; both are named.
    let (_, _, reject) = hold(&cell, rested(&cell, 80.0, 1.0, 4.25), 30.0, -40.0, 5_000);
    assert!(matches!(reject, Some(StepReject::ElectrolyteDepleted) | Some(StepReject::UndervoltageLimit)), "{reject:?}");
    // the drop through the limit is never smaller than the linear drop it comes from
    let (state, _, _) = hold(&cell, rested(&cell, 80.0, 1.0, 4.25), 4.5, -20.0, 300);
    assert!(state.electrolyte_drop_v >= state.electrolyte_v);
}

#[test]
fn past_each_edge_of_the_compared_range_a_step_lands_with_the_flag() {
    let cell = Cell2170Twin::cell();
    let start = rested(&cell, 80.0, 0.6, 4.2);
    let inside = |current_a: f64, case_c: f64| {
        let mut state = start;
        let mut frame = blank_frame();
        step(&cell, &mut state, &mut frame, load(current_a, case_c), 1.0).unwrap();
        frame.flags & FLAG_OUTSIDE_COMPARED_RUNS == 0
    };
    assert!(inside(1.0, -15.0));
    assert!(!inside(1.0, CASE_MIN_COMPARED - 0.5));
    assert!(!inside(DISCHARGE_MAX_COMPARED + 0.5, 40.0));
    assert!(!inside(-(CHARGE_MAX_COMPARED + 0.5), 40.0));
    let outside_capacity = CellState::try_rested(&cell, 80.0, 0.6, CAPACITY_MAX_COMPARED + 0.05).unwrap();
    let mut frame = blank_frame();
    let mut state = outside_capacity;
    step(&cell, &mut state, &mut frame, load(1.0, 25.0), 1.0).unwrap();
    assert!(frame.flags & FLAG_OUTSIDE_COMPARED_RUNS != 0);
}

#[test]
fn a_held_rated_current_carries_the_sustained_flag() {
    let cell = Cell2170Twin::cell();
    let mut state = rested(&cell, 1000.0, 1.0, 4.5);
    let mut frame = blank_frame();
    for _ in 0..300 {
        step(&cell, &mut state, &mut frame, load(5.0, 25.0), 1.0).unwrap();
    }
    assert!(frame.flags & FLAG_SUSTAINED_ABOVE_1C != 0);
    assert!(state.sustained_above_1c);
}

#[test]
fn a_falsified_ledger_is_refused_both_ways() {
    let cell = Cell2170Twin::cell();
    let (state, _, _) = hold(&cell, rested(&cell, 80.0, 1.0, 4.4), 3.0, 0.0, 200);
    for (shift, want) in [(1.0, StepReject::SpontaneousGeneration), (-1.0, StepReject::FirstLawBreach)] {
        let mut forged = state;
        forged.energy_anchor_j -= shift;
        let mut frame = blank_frame();
        assert_eq!(step(&cell, &mut forged, &mut frame, load(3.0, 0.0), 1.0), Err(want));
        let mut forged = state;
        forged.heat_stored_j -= shift;
        let mut frame = blank_frame();
        assert_eq!(step(&cell, &mut forged, &mut frame, load(3.0, 0.0), 1.0), Err(want), "the core's heat ledger");
    }
}

#[test]
fn repeated_runs_are_the_same_to_the_bit() {
    let cell = Cell2170Twin::cell();
    let run = || {
        let mut state = rested(&cell, 448.0, 0.8, 4.3);
        let mut frame = blank_frame();
        let mut bits = Vec::new();
        for (k, (current_a, case_c)) in series(&[(300, 6.0, -18.0, 0.01), (120, -2.0, -15.0, 0.0), (600, 1.5, -15.0, 0.0)]).into_iter().enumerate() {
            step(&cell, &mut state, &mut frame, load(current_a, case_c), 1.0).unwrap();
            if k % 37 == 0 {
                bits.push(state.terminal_v.to_bits());
                bits.push(state.core_temperature_c.to_bits());
            }
        }
        bits
    };
    assert_eq!(run(), run());
}

#[test]
fn the_proof_slot_is_not_the_cards() {
    let cell = Cell2170Twin::cell();
    let mut state = rested(&cell, 80.0, 0.9, 4.5);
    let mut frame = blank_frame();
    step(&cell, &mut state, &mut frame, load(2.0, 10.0), 1.0).unwrap();
    assert_eq!(frame.proof, [0xA5; 16]);
    assert_eq!(frame.pos[0], state.terminal_v as f32);
    assert_eq!(frame.vel[2], state.electrolyte_drop_v as f32);
}

#[test]
fn types_are_copy_and_the_step_is_core_only() {
    fn assert_copy<T: Copy>() {}
    assert_copy::<Cell2170Twin>();
    assert_copy::<CellState>();
    assert_copy::<CellLoad>();
    let source = include_str!("cell_2170_twin.rs");
    let card = &source[..source.find("#[cfg(test)]").unwrap()];
    for word in ["std::", "Vec<", "Box<", "String", "f64::exp", ".exp()", ".ln()", ".powf("] {
        assert!(!card.contains(word), "the card uses {word}");
    }
}

#[test]
fn the_case_span_of_a_continued_run_is_carried() {
    let cell = Cell2170Twin::cell();
    let start = rested(&cell, 80.0, 0.7, 4.3);
    assert!(start.with_case_span(&cell, 10.0, 5.0).is_none());
    assert!(start.with_case_span(&cell, -300.0, 5.0).is_none());
    let carried = start.with_case_span(&cell, -20.0, 10.0).unwrap();
    let mut state = carried;
    let mut frame = blank_frame();
    step(&cell, &mut state, &mut frame, load(1.0, 5.0), 1.0).unwrap();
    assert_eq!((state.case_temperature_least_c, state.case_temperature_most_c), (-20.0, 10.0));
}

#[test]
fn the_rest_voltage_rises_with_charge_on_the_compared_range() {
    let cell = Cell2170Twin::cell();
    for row in 0..CELL_COUNT {
        let mut last = 0.0;
        for k in 0..=100 {
            let z = k as f64 / 100.0;
            let v = cell.rest_v(row, 4.4, z, 25.0).unwrap();
            assert!(v > last, "row {row} at {z}");
            last = v;
        }
        assert!(cell.rest_v(row, 4.4, 1.06, 25.0).is_none());
    }
    let text = format!("{:?}", StepReject::ElectrolyteDepleted);
    assert_eq!(text, "ElectrolyteDepleted");
}

