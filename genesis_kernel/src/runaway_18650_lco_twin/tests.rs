extern crate core;
extern crate std;

use core::prelude::v1::*;
use std::assert;
use std::assert_eq;
use std::assert_ne;

use super::*;

const PROOF: [u8; 16] = [0xab; 16];

fn blank() -> LastStateFrame64 {
    LastStateFrame64 {
        t: -1.0,
        pos: [-1.0; 3],
        vel: [-1.0; 3],
        force_torque: -1.0,
        residual: -1.0,
        flags: 0xdead_beef,
        proof: PROOF,
    }
}

fn oven(oven_temperature_k: f64) -> RunawayLoad {
    RunawayLoad { oven_temperature_k, heater_power_w: 0.0, adiabatic: 0.0 }
}

fn calorimeter(heater_power_w: f64) -> RunawayLoad {
    RunawayLoad { oven_temperature_k: 298.15, heater_power_w, adiabatic: 1.0 }
}

fn assert_copy<T: Copy>() {}

fn relative(got: f64, expected: f64) -> f64 {
    (got - expected).abs() / expected.abs()
}

fn bits(state: &RunawayState) -> std::vec::Vec<u64> {
    let mut out: std::vec::Vec<u64> = state.amounts().iter().chain(state.heats().iter()).map(|x| x.to_bits()).collect();
    for x in [
        state.t_s,
        state.temperature_k,
        state.sei_thickness,
        state.start_temperature_k,
        state.heat_released_j,
        state.heater_heat_j,
        state.heat_lost_j,
        state.heat_stored_j,
        state.chemical_heat_w,
        state.chemical_heating_k_s,
        state.energy_defect_j,
        state.residual_conservation_error,
        state.substeps,
    ] {
        out.push(x.to_bits());
    }
    for b in [state.self_heating, state.runaway, state.separator_melted, state.vented, state.outside_compared] {
        out.push(b as u64);
    }
    out
}

fn fresh(temperature_k: f64) -> (Runaway18650LcoTwin, RunawayState) {
    let cell = Runaway18650LcoTwin::fitted();
    let state = RunawayState::try_fresh(&cell, temperature_k).unwrap();
    (cell, state)
}

fn spent(temperature_k: f64, amounts: [f64; STAGES]) -> RunawayState {
    RunawayState::try_start(&Runaway18650LcoTwin::fitted(), temperature_k, amounts).unwrap()
}

/// The state of an oven run from 28 degC at `t_s`, stepping at 2 s.
fn oven_state_at(oven_k: f64, t_s: f64) -> (Runaway18650LcoTwin, RunawayState) {
    let (cell, mut state) = fresh(301.15);
    let mut frame = blank();
    while state.t_s < t_s {
        step(&cell, &mut state, &mut frame, oven(oven_k), 2.0).unwrap();
    }
    (cell, state)
}

/// The first time the run's net stage heat reaches `rate_k_s`, stepping at
/// 2 s, and the highest temperature to `until_s`.
fn oven_run(oven_k: f64, until_s: f64, rate_k_s: f64) -> (Option<f64>, f64, f64, RunawayState) {
    let (cell, mut state) = fresh(301.15);
    let mut frame = blank();
    let mut first = None;
    let mut peak: f64 = 0.0;
    let mut worst: f64 = 0.0;
    while state.t_s < until_s {
        step(&cell, &mut state, &mut frame, oven(oven_k), 2.0).unwrap();
        worst = worst.max(state.residual_conservation_error);
        peak = peak.max(state.temperature_k);
        if first.is_none() && state.chemical_heating_k_s >= rate_k_s {
            first = Some(state.t_s);
        }
    }
    (first, peak, worst, state)
}

#[test]
fn the_card_locks_its_constants() {
    let cell = Runaway18650LcoTwin::fitted();
    assert!(cell.is_valid());
    assert_eq!(cell.activation_j_per_mol, [1.3508e5, 1.3508e5, 1.396e5, 2.74e5]);
    assert_eq!(cell.start_amounts, [0.15, 0.75, 0.96, 1.0, 1.0, 1.0]);
    assert_eq!(cell.phase_k, [403.15, 433.15]);
    let mut frame = blank();
    let held = frame.to_bytes();
    for index in 0..20 {
        let mut other = cell;
        match index {
            0 => other.frequency_factor_per_s[0] *= 1.000001,
            1 => other.activation_j_per_mol[1] = 1.35e5,
            2 => other.stage_heat_j[4] = 0.0,
            3 => other.start_amounts[5] = 0.5,
            4 => other.phase_k[1] = 440.0,
            5 => other.phase_width_k = 2.0,
            6 => other.phase_time_s = 1.0,
            7 => other.sei_plus_anode_lithium = 0.8,
            8 => other.sei_thickness_reference = 0.05,
            9 => other.heat_capacity_j_per_k = 40.0,
            10 => other.convective_conductance_w_per_k = 0.0,
            11 => other.radiative_conductance_w_per_k4 = 0.0,
            12 => other.can_melting_k = 2000.0,
            13 => other.chemical_heat_j = 1.0,
            14 => other.compared_oven_k[0] = 400.0,
            15 => other.compared_heater_w = 2.0,
            16 => other.gate_over_floor = 2.0,
            17 => other.consumed_heat_fraction = 1.0e-3,
            18 => other.amount_limits[4] = 0.5,
            _ => other.cathode_converted_least = 0.0,
        }
        assert!(!other.is_valid(), "{index}");
        assert_eq!(RunawayState::try_fresh(&other, 300.0), Err(StepReject::InvalidCell));
        let (_, mut state) = fresh(300.0);
        let before = bits(&state);
        assert_eq!(step(&other, &mut state, &mut frame, oven(400.0), 1.0), Err(StepReject::InvalidCell));
        assert_eq!(bits(&state), before);
        assert_eq!(frame.to_bytes(), held);
    }
}

/// The published constants are runaway_18650_lco's; the cell's heat is the
/// same 20,499 J and the two phase stages take heat.
#[test]
fn the_published_constants_are_those_of_runaway_18650_lco() {
    use crate::runaway_18650_lco as card;
    assert_eq!(ANODE_FREQUENCY_FACTOR_PER_S, card::ANODE_FREQUENCY_FACTOR_PER_S);
    assert_eq!(ELECTROLYTE_FREQUENCY_FACTOR_PER_S, card::ELECTROLYTE_FREQUENCY_FACTOR_PER_S);
    assert_eq!(CELL_HEAT_CAPACITY_J_PER_K, card::CELL_HEAT_CAPACITY_J_PER_K);
    assert_eq!(CELL_VOLUME_M3, card::CELL_VOLUME_M3);
    assert!(relative(CELL_CHEMICAL_HEAT_J, card::CELL_CHEMICAL_HEAT_J) < 1.0e-14);
    assert!(STAGE_HEAT_J[4] < 0.0 && STAGE_HEAT_J[5] == 0.0);
    assert_eq!(SEI_PLUS_ANODE_LITHIUM, SEI_THICKNESS_START + 0.75);
    let cell = Runaway18650LcoTwin::fitted();
    // The phase stages run at 1/60 per second at their temperature, e-fold per kelvin.
    assert!(relative(cell.rate_constant_per_s(4, 403.15), 1.0 / 60.0) < 1.0e-15);
    assert!(relative(cell.rate_constant_per_s(5, 434.15) / cell.rate_constant_per_s(5, 433.15), 433.15 / 434.15 * 1.0_f64.exp()) < 1.0e-3);
    assert_eq!(cell.floor_dt_s().unwrap(), cell.gate_dt_s().unwrap() / 1_048_576.0);
}

#[test]
fn types_are_copy() {
    assert_copy::<Runaway18650LcoTwin>();
    assert_copy::<RunawayState>();
    assert_copy::<RunawayLoad>();
    assert_copy::<StepReject>();
}

/// The step does not follow the step asked for: 1 s, 5 s and the ceiling
/// land on the same heat released through the 155 degC oven's runaway.
#[test]
fn the_runaway_does_not_follow_the_step_asked_for() {
    let cell = Runaway18650LcoTwin::fitted();
    let gate = cell.gate_dt_s().unwrap();
    let mut ends = std::vec::Vec::new();
    for dt in [1.0, 5.0, gate] {
        let (cell, mut state) = fresh(301.15);
        let mut frame = blank();
        let steps = (7200.0 / dt) as usize;
        for _ in 0..steps {
            step(&cell, &mut state, &mut frame, oven(428.15), dt).unwrap();
        }
        ends.push((state.anode_lithium_fraction, state.heat_released_j, state.t_s));
    }
    for end in &ends[1..] {
        assert!((end.0 - ends[0].0).abs() < 1.0e-4, "{end:?} {:?}", ends[0]);
        assert!((end.1 - ends[0].1).abs() < 2.0, "{end:?} {:?}", ends[0]);
    }
}

/// The flags in a 155 degC oven: the separator melts, the cell vents, it
/// self-heats and runs away; every frame carries only declared bits, and an
/// oven inside the compared ones never sets FLAG_OUTSIDE_COMPARED.
#[test]
fn the_flags_mark_the_melt_the_vent_and_the_runaway_and_land() {
    let (cell, mut state) = fresh(301.15);
    let mut frame = blank();
    let mut seen = 0;
    while state.t_s < 7200.0 {
        step(&cell, &mut state, &mut frame, oven(428.15), 2.0).unwrap();
        let declared = FLAG_SELF_HEATING | FLAG_RUNAWAY | FLAG_SEPARATOR_MELTED | FLAG_VENTED | FLAG_OUTSIDE_COMPARED;
        assert_eq!(frame.flags & !declared, 0);
        assert_eq!(frame.flags & FLAG_RUNAWAY != 0, state.runaway);
        assert_eq!(frame.flags & FLAG_SEPARATOR_MELTED != 0, state.separator_melted);
        assert_eq!(frame.flags & FLAG_VENTED != 0, state.vented);
        assert_eq!(frame.flags & FLAG_OUTSIDE_COMPARED, 0);
        if state.vented {
            assert!(state.separator_melted);
        }
        seen |= frame.flags;
        assert_eq!(frame.proof, PROOF);
    }
    assert_eq!(seen, FLAG_SELF_HEATING | FLAG_RUNAWAY | FLAG_SEPARATOR_MELTED | FLAG_VENTED);
    assert!(state.heat_separator_j < 0.0 && state.heat_vent_j == 0.0);
    assert!(relative(-state.heat_separator_j, SEPARATOR_MELT_HEAT_J) < 1.0e-6);
    assert!(state.vent_gas_held_fraction < 1.0e-6);
    for (load, outside) in [
        (oven(418.15), false),
        (oven(417.0), true),
        (oven(430.0), true),
        (calorimeter(1.18), false),
        (calorimeter(5.0), true),
        (RunawayLoad { heater_power_w: 1.0, ..oven(420.0) }, true),
    ] {
        let (cell, mut state) = fresh(300.0);
        step(&cell, &mut state, &mut frame, load, 1.0).unwrap();
        assert_eq!(state.outside_compared, outside, "{load:?}");
        assert_eq!(frame.flags & FLAG_OUTSIDE_COMPARED != 0, outside);
    }
}

/// In a calorimeter that tracks it, a fresh cell from 160 degC releases its
/// heat and the ledger closes over the whole release: the rise is the net
/// heat over the heat capacity.
#[test]
fn an_adiabatic_cell_rises_by_its_heat_over_its_heat_capacity() {
    let (cell, mut state) = fresh(423.15);
    let mut frame = blank();
    let mut worst: f64 = 0.0;
    for _ in 0..600 {
        step(&cell, &mut state, &mut frame, calorimeter(0.0), 10.0).unwrap();
        worst = worst.max(state.residual_conservation_error);
    }
    assert_eq!(state.heat_lost_j, 0.0);
    let rise = state.temperature_k - 423.15;
    assert!(relative(rise * CELL_HEAT_CAPACITY_J_PER_K, state.heat_released_j) < 1.0e-6);
    // A start 20 K above the melt, where the separator melts in
    // milliseconds: the ledger holds to 6 mJ, under half its limit.
    assert!(worst < 0.5 * RESIDUAL_CONSERVATION_LIMIT, "{worst:e}");
    assert!(state.temperature_k > 800.0, "{}", state.temperature_k);
    assert_eq!(sum(&state.heats()), state.heat_released_j);
}

/// A heater alone, with nothing left, warms the cell by its power over the
/// heat capacity.
#[test]
fn a_heater_warms_a_spent_cell_by_its_power_over_its_heat_capacity() {
    let cell = Runaway18650LcoTwin::fitted();
    let mut state = spent(300.0, [0.0; STAGES]);
    let mut frame = blank();
    for _ in 0..10 {
        step(&cell, &mut state, &mut frame, calorimeter(CELL_HEAT_CAPACITY_J_PER_K), 1.0).unwrap();
    }
    assert!((state.temperature_k - 310.0).abs() < 1.0e-9, "{}", state.temperature_k);
    assert!(state.residual_conservation_error < 1.0e-12);
    // spent, in an oven: no end
    let mut cooling = spent(500.0, [0.0; STAGES]);
    for _ in 0..100 {
        step(&cell, &mut cooling, &mut frame, oven(300.0), 10.0).unwrap();
    }
    assert!(cooling.temperature_k < 500.0 && cooling.temperature_k > 300.0);
}

/// The end: at 900 K a thousandth of the electrolyte (1.04 J) burns out
/// inside the first step, with the state and the frame kept.
#[test]
fn every_reactant_consumed_is_the_end() {
    let cell = Runaway18650LcoTwin::fitted();
    let start = spent(900.0, [0.0, 0.0, 0.0, 1.0e-3, 0.0, 0.0]);
    let mut state = start;
    let mut frame = blank();
    let held = frame.to_bytes();
    assert_eq!(step(&cell, &mut state, &mut frame, calorimeter(0.0), 1.0), Err(StepReject::EveryReactantConsumed));
    assert_eq!(bits(&state), bits(&start));
    assert_eq!(frame.to_bytes(), held);
    // a separator not yet melted at 900 K takes its heat and ends the same way
    let start = spent(900.0, [0.0, 0.0, 0.0, 0.0, 1.0e-3, 0.0]);
    let mut state = start;
    assert_eq!(step(&cell, &mut state, &mut frame, calorimeter(0.0), 1.0), Err(StepReject::EveryReactantConsumed));
    assert_eq!(bits(&state), bits(&start));
}

/// The real `validate`, a chosen way of stepping, the real `commit`.
fn step_with(
    cell: &Runaway18650LcoTwin,
    state: &mut RunawayState,
    frame: &mut LastStateFrame64,
    load: RunawayLoad,
    dt_s: f64,
    substep: Substep,
    sized: bool,
) -> Result<(), StepReject> {
    validate(cell, state, &load, dt_s)?;
    let trial = integrate(cell, state, &load, dt_s, substep, sized, 1.0)?;
    commit(cell, state, frame, &load, dt_s, trial)
}

/// Explicit Euler over the same variables.
fn euler(cell: &Runaway18650LcoTwin, point: Point, live: [bool; STAGES], load: &RunawayLoad, h: f64, first: &Rates) -> (Point, f64) {
    let _ = (cell, live, load);
    let mut logs = point.logs;
    for stage in 0..STAGES {
        logs[stage] += h * first.logs_per_s[stage];
    }
    (Point { temperature_k: point.temperature_k + h * first.temperature_k_s, logs }, h * first.loss_w)
}

/// The state of the 155 degC oven run at the first step whose net stage
/// heat warms the cell at 100 K per minute or more, inside the runaway.
fn just_before_runaway() -> (Runaway18650LcoTwin, RunawayState) {
    let (cell, mut state) = fresh(301.15);
    let mut frame = blank();
    loop {
        step(&cell, &mut state, &mut frame, oven(428.15), 2.0).unwrap();
        if state.chemical_heating_k_s >= 100.0 / 60.0 {
            return (cell, state);
        }
        assert!(state.t_s < 14_400.0);
    }
}

/// Rule 10: a wrong stepping method is named. Explicit Euler with the
/// card's own substeps, and the card's Runge-Kutta without them, are each
/// refused by the ledger in the runaway; the card's own step lands.
#[test]
fn a_wrong_stepping_method_is_named_by_the_ledger() {
    let (cell, start) = just_before_runaway();
    let mut frame = blank();
    let held = frame.to_bytes();
    let mut state = start;
    let got = step_with(&cell, &mut state, &mut frame, oven(428.15), 10.0, euler, true);
    assert!(got == Err(StepReject::SpontaneousGeneration) || got == Err(StepReject::FirstLawBreach), "{got:?}");
    assert_eq!(bits(&state), bits(&start));
    assert_eq!(frame.to_bytes(), held);
    let got = step_with(&cell, &mut state, &mut frame, oven(428.15), 10.0, rk4, false);
    assert!(got == Err(StepReject::SpontaneousGeneration) || got == Err(StepReject::FirstLawBreach), "{got:?}");
    assert_eq!(bits(&state), bits(&start));
    assert_eq!(frame.to_bytes(), held);
    step(&cell, &mut state, &mut frame, oven(428.15), 10.0).unwrap();
    assert!(state.residual_conservation_error < 0.1 * RESIDUAL_CONSERVATION_LIMIT);
    assert_eq!(frame.proof, PROOF);
}

/// Rule 10: a falsified ledger is named, both ways, with the state and the
/// frame kept.
#[test]
fn a_falsified_ledger_is_named_both_ways() {
    let (cell, start) = oven_state_at(428.15, 1200.0);
    let mut frame = blank();
    let held = frame.to_bytes();
    let gate = RESIDUAL_CONSERVATION_LIMIT * CELL_CHEMICAL_HEAT_J;
    let attempt = |alter: &dyn Fn(&mut RunawayState), expected: StepReject, frame: &mut LastStateFrame64| {
        let mut altered = start;
        alter(&mut altered);
        let before = bits(&altered);
        assert_eq!(step(&cell, &mut altered, frame, oven(428.15), 1.0), Err(expected));
        assert_eq!(bits(&altered), before);
    };
    attempt(&|s| s.temperature_k += 0.01, StepReject::SpontaneousGeneration, &mut frame);
    attempt(&|s| s.temperature_k -= 0.01, StepReject::FirstLawBreach, &mut frame);
    attempt(&|s| s.heat_anode_j += 10.0 * gate, StepReject::FirstLawBreach, &mut frame);
    attempt(&|s| s.heat_vent_j -= 10.0 * gate, StepReject::SpontaneousGeneration, &mut frame);
    attempt(&|s| s.heat_cathode_j -= 10.0 * gate, StepReject::SpontaneousGeneration, &mut frame);
    attempt(&|s| s.heat_separator_j += 10.0 * gate, StepReject::FirstLawBreach, &mut frame);
    attempt(&|s| s.heat_lost_j -= 10.0 * gate, StepReject::FirstLawBreach, &mut frame);
    attempt(&|s| s.start_temperature_k += 0.01, StepReject::FirstLawBreach, &mut frame);
    assert_eq!(frame.to_bytes(), held);
    let mut state = start;
    let load = oven(428.15);
    let honest = integrate(&cell, &state, &load, 1.0, rk4, true, 1.0).unwrap();
    let mut forged = honest;
    forged.temperature_k += 0.01;
    assert_eq!(commit(&cell, &mut state, &mut frame, &load, 1.0, forged), Err(StepReject::SpontaneousGeneration));
    let mut forged = honest;
    forged.lost_j += 1.0;
    assert_eq!(commit(&cell, &mut state, &mut frame, &load, 1.0, forged), Err(StepReject::SpontaneousGeneration));
    let mut forged = honest;
    forged.amounts[5] = 1.5;
    assert_eq!(commit(&cell, &mut state, &mut frame, &load, 1.0, forged), Err(StepReject::VentGasOutsideCell));
    let mut forged = honest;
    forged.amounts[4] = -1.0e-12;
    assert_eq!(commit(&cell, &mut state, &mut frame, &load, 1.0, forged), Err(StepReject::SeparatorOutsideCell));
    assert_eq!(bits(&state), bits(&start));
    assert_eq!(frame.to_bytes(), held);
    commit(&cell, &mut state, &mut frame, &load, 1.0, honest).unwrap();
    assert_eq!(frame.proof, PROOF);
}

#[test]
fn every_refusal_is_named_and_leaves_state_and_frame_bit_identical() {
    let (cell, start) = fresh(350.0);
    let mut frame = blank();
    let held = frame.to_bytes();
    let mut attempt = |alter: &dyn Fn(&mut RunawayState), load: RunawayLoad, dt_s: f64, expected: StepReject| {
        let mut altered = start;
        alter(&mut altered);
        let before = bits(&altered);
        assert_eq!(step(&cell, &mut altered, &mut frame, load, dt_s), Err(expected));
        assert_eq!(bits(&altered), before);
        assert_eq!(frame.to_bytes(), held);
    };
    let none = |_: &mut RunawayState| {};
    attempt(&none, oven(400.0), f64::NAN, StepReject::NonFiniteInput);
    attempt(&none, oven(f64::INFINITY), 1.0, StepReject::NonFiniteInput);
    attempt(&|s| s.vent_gas_held_fraction = f64::NAN, oven(400.0), 1.0, StepReject::NonFiniteState);
    attempt(&none, oven(400.0), 0.0, StepReject::NonPositiveDt);
    attempt(&none, oven(0.0), 1.0, StepReject::OvenTemperatureNotPositive);
    attempt(&none, calorimeter(-1.0e-9), 1.0, StepReject::HeaterPowerNegative);
    attempt(&none, RunawayLoad { adiabatic: 0.5, ..oven(400.0) }, 1.0, StepReject::AdiabaticNotZeroOrOne);
    attempt(&|s| s.sei_fraction = 1.0000000000000002, oven(400.0), 1.0, StepReject::SeiFractionOutsideCell);
    attempt(&|s| s.anode_lithium_fraction = SEI_PLUS_ANODE_LITHIUM * 1.0000001, oven(400.0), 1.0, StepReject::AnodeLithiumOutsideCell);
    attempt(&|s| s.cathode_unreacted_fraction = 1.0000000000000002, oven(400.0), 1.0, StepReject::CathodeUnreactedOutsideCell);
    attempt(&|s| s.electrolyte_fraction = -1.0e-300, oven(400.0), 1.0, StepReject::ElectrolyteOutsideCell);
    attempt(&|s| s.separator_unmelted_fraction = 1.5, oven(400.0), 1.0, StepReject::SeparatorOutsideCell);
    attempt(&|s| s.vent_gas_held_fraction = -0.1, oven(400.0), 1.0, StepReject::VentGasOutsideCell);
    attempt(&none, calorimeter(1.0e6), 1.0, StepReject::CanMelting);
    let mut cold = RunawayState::try_fresh(&cell, 1.0).unwrap();
    let before = bits(&cold);
    let through_zero: Substep = |_, point, _, _, _, _| (Point { temperature_k: -point.temperature_k, logs: point.logs }, 0.0);
    assert_eq!(step_with(&cell, &mut cold, &mut frame, oven(300.0), 1.0, through_zero, true), Err(StepReject::TemperatureNotPositive));
    assert_eq!(bits(&cold), before);
    let fresh_amounts = cell.start_amounts;
    for (temperature, index, value, expected) in [
        (0.0, 0, 0.15, StepReject::TemperatureNotPositive),
        (1811.0, 0, 0.15, StepReject::CanMelting),
        (300.0, 0, 1.5, StepReject::SeiFractionOutsideCell),
        (300.0, 1, 0.99, StepReject::AnodeLithiumOutsideCell),
        (300.0, 2, 1.01, StepReject::CathodeUnreactedOutsideCell),
        (300.0, 3, -0.1, StepReject::ElectrolyteOutsideCell),
        (300.0, 4, 2.0, StepReject::SeparatorOutsideCell),
        (300.0, 5, -1.0, StepReject::VentGasOutsideCell),
    ] {
        let mut amounts = fresh_amounts;
        amounts[index] = value;
        assert_eq!(RunawayState::try_start(&cell, temperature, amounts), Err(expected), "{temperature} {index} {value}");
    }
    let mut amounts = fresh_amounts;
    amounts[4] = f64::NAN;
    assert_eq!(RunawayState::try_start(&cell, 300.0, amounts), Err(StepReject::NonFiniteInput));
    assert!(RunawayState::try_start(&cell, 300.0, AMOUNT_LIMITS).is_ok());
    assert!(RunawayState::try_start(&cell, 300.0, [0.0; STAGES]).is_ok());
    assert_eq!(frame.proof, PROOF);
}

/// A step that would need more than 2^20 substeps is refused by name.
#[test]
fn a_step_beyond_the_substep_budget_is_named() {
    let (cell, start) = fresh(1000.0);
    let mut state = start;
    let mut frame = blank();
    let held = frame.to_bytes();
    let stuck: Substep = |_, point, _, _, _, _| (point, 0.0);
    assert_eq!(step_with(&cell, &mut state, &mut frame, calorimeter(0.0), 1.0, stuck, true), Err(StepReject::SubstepsAboveBudget));
    assert_eq!(bits(&state), bits(&start));
    assert_eq!(frame.to_bytes(), held);
    step(&cell, &mut state, &mut frame, calorimeter(0.0), 1.0).unwrap();
    assert!(state.substeps > 100.0 && state.substeps < 5000.0, "{}", state.substeps);
}

/// The frame carries the declared slots.
#[test]
fn the_frame_carries_the_declared_slots() {
    let (cell, mut state) = oven_state_at(428.15, 1200.0);
    let mut frame = blank();
    step(&cell, &mut state, &mut frame, oven(428.15), 1.0).unwrap();
    assert_eq!(frame.t, state.t_s);
    assert_eq!(frame.pos[0], state.temperature_k as f32);
    assert_eq!(frame.pos[1], state.chemical_heat_w as f32);
    assert_eq!(frame.pos[2], state.heat_released_j as f32);
    assert_eq!(frame.vel[0], state.anode_lithium_fraction as f32);
    assert_eq!(frame.vel[1], state.cathode_unreacted_fraction as f32);
    assert_eq!(frame.vel[2], state.electrolyte_fraction as f32);
    assert_eq!(frame.force_torque, state.heat_lost_j as f32);
    assert_eq!(frame.residual, state.residual_conservation_error as f32);
    assert_eq!(frame.proof, PROOF);
    assert!(relative(state.sei_thickness + state.anode_lithium_fraction, SEI_PLUS_ANODE_LITHIUM) < 1.0e-15);
}

/// A run cut in two by handing back the returned temperature and amounts
/// as a new start continues the run, the heat books begun again.
#[test]
fn a_returned_state_handed_back_continues_the_run() {
    let (cell, mut whole) = fresh(301.15);
    let mut frame = blank();
    for _ in 0..400 {
        step(&cell, &mut whole, &mut frame, oven(428.15), 10.0).unwrap();
    }
    let (_, mut first) = fresh(301.15);
    for _ in 0..250 {
        step(&cell, &mut first, &mut frame, oven(428.15), 10.0).unwrap();
    }
    let mut second = RunawayState::try_start(&cell, first.temperature_k, first.amounts()).unwrap();
    for _ in 0..150 {
        step(&cell, &mut second, &mut frame, oven(428.15), 10.0).unwrap();
    }
    assert_eq!(second.temperature_k.to_bits(), whole.temperature_k.to_bits());
    assert_eq!(second.amounts(), whole.amounts());
    assert!(relative(first.heat_released_j + second.heat_released_j, whole.heat_released_j) < 1.0e-12);
}

#[test]
fn the_step_ceiling_and_floor_are_present_and_a_tiny_step_is_not_clamped() {
    let (cell, mut state) = fresh(400.0);
    let mut broken = cell;
    broken.gate_over_convective_time = f64::NAN;
    assert_eq!(broken.gate_dt_s(), None);
    assert_eq!(broken.floor_dt_s(), None);
    let floor = cell.floor_dt_s().unwrap();
    let mut frame = blank();
    let before = state.temperature_k;
    step(&cell, &mut state, &mut frame, oven(500.0), floor).unwrap();
    assert_eq!(state.t_s, floor);
    assert!(state.temperature_k > before);
    assert_eq!(state.substeps, 1.0);
}

/// The twin on Hatchard's ovens against the independent integration of its
/// own equations (scipy LSODA, evidence/research/thermal_runaway_18650/twin/
/// model.py with its CARD constants): 20 K per minute in the 155 degC oven at
/// 3,714 s (61.9 min; Hatchard's cell 51.1), 10 K per minute at 150 degC at
/// 6,520 s (109 min; the cell did not run away in 120), none at 145 degC,
/// whose peak is 421.10 K (148.0 degC, as the cell's).
#[test]
fn hatchards_ovens_as_the_independent_integration_gives_them() {
    let (at_155, peak_155, worst, _) = oven_run(428.15, 14_400.0, 20.0 / 60.0);
    let t = at_155.unwrap();
    assert!((t - REFERENCE_155_TO_20_K_PER_MIN_S).abs() <= 6.0, "{t}");
    assert!(peak_155 > 600.0, "{peak_155}");
    assert!(worst < 0.1 * RESIDUAL_CONSERVATION_LIMIT, "{worst:e}");
    let (at_150, _, _, _) = oven_run(423.15, 7_200.0, 10.0 / 60.0);
    assert!((at_150.unwrap() - REFERENCE_150_TO_10_K_PER_MIN_S).abs() <= 6.0, "{at_150:?}");
    let (at_145, peak_145, _, _) = oven_run(418.15, 7_200.0, 10.0 / 60.0);
    assert_eq!(at_145, None);
    assert!((peak_145 - REFERENCE_145_PEAK_K).abs() < 0.05, "{peak_145}");
    assert_ne!(t, 0.0);
}

/// The reference integration's numbers (model.py with the card's constants).
const REFERENCE_155_TO_20_K_PER_MIN_S: f64 = 3714.0;
const REFERENCE_150_TO_10_K_PER_MIN_S: f64 = 6520.0;
const REFERENCE_145_PEAK_K: f64 = 421.101;


/// Engine 0.15.2, work orders of 21 October: the card's own example state in
/// an oven above about 440 K at steps of 10 s was refused FirstLawBreach
/// part way (455 K at step 46, 452 K at 48, 520 K at 23, 700 K at 7 on
/// 0.15.1), with landed runs on either side: an outcome that turned with the
/// oven's temperature. Inside a substep sized at its start the separator's
/// melt rate, steep in temperature, ran away from the Runge-Kutta rule, and
/// the error passed the ledger's limit. Now the step is taken again with
/// substeps half as long, and again, until its ledger closes. Every oven
/// from 400 to 800 K runs the 600 steps, or ends at the runaway's own end,
/// and never on the ledger.
#[test]
fn an_oven_run_at_ten_seconds_never_ends_on_the_ledger() {
    let cell = Runaway18650LcoTwin::fitted();
    let amounts = [0.15, 0.75, 0.96, 1.0, 1.0, 1.0];
    let mut oven_k = 400.0;
    while oven_k <= 800.0 {
        let mut state = RunawayState::try_start(&cell, 301.15, amounts).unwrap();
        let mut frame = blank();
        let mut landed = 0;
        let end = loop {
            if landed == 600 {
                break None;
            }
            match step(&cell, &mut state, &mut frame, oven(oven_k), 10.0) {
                Ok(()) => landed += 1,
                Err(reject) => break Some(reject),
            }
        };
        assert!(
            end.is_none() || end == Some(StepReject::EveryReactantConsumed),
            "oven {oven_k} K: {end:?} after {landed} steps"
        );
        assert!(state.residual_conservation_error <= RESIDUAL_CONSERVATION_LIMIT);
        oven_k += 1.75;
    }
    for oven_k in [452.0, 455.0, 460.0, 520.0, 700.0] {
        let mut state = RunawayState::try_start(&cell, 301.15, amounts).unwrap();
        let mut frame = blank();
        for count in 0..600 {
            step(&cell, &mut state, &mut frame, oven(oven_k), 10.0)
                .unwrap_or_else(|e| std::panic!("{oven_k} K, step {count}: {e:?}"));
        }
    }
}

/// A step whose ledger closes with the substeps of engine 0.15.1 lands bit
/// for bit as it did: the shorter substeps are taken only after a refusal.
#[test]
fn a_step_that_landed_lands_as_before() {
    let cell = Runaway18650LcoTwin::fitted();
    let mut state = RunawayState::try_start(&cell, 301.15, [0.15, 0.75, 0.96, 1.0, 1.0, 1.0]).unwrap();
    let mut frame = blank();
    for _ in 0..60 {
        let mut by_hand = state;
        let mut hand_frame = frame;
        let trial = integrate(&cell, &by_hand, &oven(428.15), 10.0, rk4, true, 1.0).unwrap();
        commit(&cell, &mut by_hand, &mut hand_frame, &oven(428.15), 10.0, trial).unwrap();
        normal_or_zero_state(&mut by_hand);
        step(&cell, &mut state, &mut frame, oven(428.15), 10.0).unwrap();
        assert_eq!(bits(&state), bits(&by_hand));
        assert_eq!(frame.to_bytes(), hand_frame.to_bytes());
    }
}
