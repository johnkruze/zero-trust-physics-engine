//! The card contract, attacked the same way on all thirty-seven cards.
//!
//! Each card is one `kit!` entry: its specimen, a start and a load that
//! land, its ledger limit, its flags, and the fields of its ledger and its
//! physical state. `conformance_support::findings` attacks the contract
//! (`genesis_kernel/AGENTS.md`) and returns what it found.
//!
//! `every_card_meets_the_contract_or_its_findings_are_reported` prints every
//! finding, and writes them as JSON lines to the file `CONFORMANCE_REPORT`
//! names when it is set. It fails only where the harness itself cannot run a
//! card. A new card is held to the whole contract by a test of its own that
//! calls `findings` and fails on any finding of the contract, as the kit's
//! template (`protocol_mcp_gateway/kit/templates/card/kernel_conformance.rs`) does.

#[macro_use]
mod conformance_support;

use conformance_support::{Finding, Kit, Weight, findings};
use genesis_kernel::*;

/// The fields of a list, each `name`, `name[index]` or `name.name`,
/// separated by commas.
macro_rules! fields {
    ($state:ty; $($rest:tt)*) => { fields!(@ $state; []; []; $($rest)*) };
    (@ $state:ty; [$($done:expr),*]; [$($cur:tt)+]; , $($rest:tt)*) => {
        fields!(@ $state; [$($done,)* field!($state, $($cur)+)]; []; $($rest)*)
    };
    (@ $state:ty; [$($done:expr),*]; [$($cur:tt)*]; $next:tt $($rest:tt)*) => {
        fields!(@ $state; [$($done),*]; [$($cur)* $next]; $($rest)*)
    };
    (@ $state:ty; [$($done:expr),*]; [$($cur:tt)+];) => { vec![$($done,)* field!($state, $($cur)+)] };
    (@ $state:ty; [$($done:expr),*]; [];) => { vec![$($done),*] };
}

/// `kit!(module, Specimen::constructor(), State, start, load, dt, steps,
/// flags, [ledger fields], [physical fields])`, with `card` naming the
/// specimen inside `start`.
macro_rules! kit {
    (
        $model:ident, $card:expr, $state:ty,
        start: |$c:ident| $start:expr,
        load: $load:expr,
        dt: $dt:expr,
        steps: $steps:expr,
        flags: $flags:expr,
        ledger: [$($ledger:tt)*],
        physical: [$($physical:tt)*] $(,)?
    ) => {{
        let card = $card;
        let $c = &card;
        let start: $state = $start;
        findings(&Kit {
            model: stringify!($model),
            card,
            is_valid: |card| card.is_valid(),
            step: $model::step,
            gate_dt_s: card.gate_dt_s(),
            floor_dt_s: card.floor_dt_s(),
            start,
            load: $load,
            dt_s: $dt,
            steps: $steps,
            residual: |state| state.residual_conservation_error,
            limit: $model::RESIDUAL_CONSERVATION_LIMIT,
            flags: $flags,
            ledger: fields!($state; $($ledger)*),
            physical: fields!($state; $($physical)*),
            departures: departures(stringify!($model)),
        })
    }};
}

/// The departures `genesis_kernel/AGENTS.md` declares: the fuse link and the
/// titanium cell take specimen values through `try_new` that `is_valid` does
/// not hold to the constants.
fn departures(card: &str) -> Vec<&'static str> {
    match card {
        "grid_node" | "ti64_lattice" => vec!["fixed constants"],
        _ => vec![],
    }
}

fn thermal() -> Vec<Finding> {
    use thermal::*;
    kit!(thermal, ThermalNode::avionics_cold_plate(), ThermalNodeState,
        start: |c| ThermalNodeState::try_phase(c, 300.0).unwrap(),
        load: ThermalLoad { heat_power_w: 50.0, ambient_k: 293.15 },
        dt: 23.625, steps: 16, flags: FLAG_THERMAL_DERATE,
        ledger: [energy_anchor_j, boundary_work_j], physical: [temperature_k])
}

fn resonance() -> Vec<Finding> {
    use resonance::*;
    kit!(resonance, DynamicOscillator::mount_10hz(), OscillatorState,
        start: |c| OscillatorState::try_phase(c, 1.0e-4, 0.0).unwrap(),
        load: OscillatorLoad { accel_m_s2: 0.5 },
        dt: 0.0003183098861837907, steps: 16, flags: FLAG_STROKE_LIMIT,
        ledger: [energy_anchor_j, boundary_work_j], physical: [displacement_m, velocity_m_s])
}

fn swing() -> Vec<Finding> {
    use swing::*;
    kit!(swing, SwingMachine::synchronous_60hz(), SwingState,
        start: |c| SwingState::try_dispatched(c, 1.2925495040600992, 0.0, 0.0, 1.0).unwrap(),
        load: SwingLoad { p_mech_pu: 1.05 },
        dt: 5.3051647697298455e-05, steps: 16, flags: FLAG_PLL_SLIP,
        ledger: [energy_anchor_pu, boundary_work_pu], physical: [delta_rad, delta_omega_rad_s])
}

fn tesseract() -> Vec<Finding> {
    use tesseract::*;
    kit!(tesseract, Tesseract::drive_mode(), TesseractState,
        start: |c| TesseractState::try_phase(c, 1.0e-6, 0.0).unwrap(),
        load: TesseractLoad { accel_m_s2: 0.1, omega_ext_rad_s: 0.0, sense_velocity_m_s: 1.0 },
        dt: 3.978873577297384e-05, steps: 16, flags: 0,
        ledger: [energy_anchor_j, boundary_work_j], physical: [displacement_m, velocity_m_s])
}

fn grid_node() -> Vec<Finding> {
    use grid_node::*;
    kit!(grid_node, GridNode::copper_fuse_link(), GridNodeState,
        start: |c| GridNodeState::try_phase(c, 1.0, 47.0, 293.15).unwrap(),
        load: GridLoad { upstream_voltage_v: 48.0, load_current_a: 10.0, ambient_k: 293.15 },
        dt: 1.0e-6, steps: 16, flags: FLAG_GRID_SURGE,
        ledger: [energy_anchor_j, boundary_work_j], physical: [current_a, voltage_v, temperature_k])
}

fn ti64_lattice() -> Vec<Finding> {
    use ti64_lattice::*;
    kit!(ti64_lattice, Ti64LatticeCell::annealed_column(), Ti64LatticeState,
        start: |c| Ti64LatticeState::try_rest(c).unwrap(),
        load: Ti64Load { force_n: [10.0, 0.0, 0.0], torque_nm: [0.0; 3], temperature_k: 293.15 },
        dt: 1.3669459757163928e-07, steps: 16, flags: 0,
        ledger: [energy_anchor_j, boundary_work_j], physical: [position_m[0], velocity_m_s[0]])
}

fn battery_sag() -> Vec<Finding> {
    use battery_sag::*;
    kit!(battery_sag, LiIonPack::pack_12s(), PackState,
        start: |c| PackState::try_soc(c, 0.9).unwrap(),
        load: PackLoad { current_a: 10.0 },
        dt: 6.6, steps: 16, flags: FLAG_SAG_WARNING,
        ledger: [energy_anchor_j, electrical_work_j, joule_j], physical: [soc])
}

fn cable_stretch() -> Vec<Finding> {
    use cable_stretch::*;
    kit!(cable_stretch, Tendon::dyneema(), TendonState,
        start: |c| TendonState::try_phase(c, 0.0, 0.0).unwrap(),
        load: TendonLoad { force_n: 100.0 },
        dt: 2.1577766334817883e-06, steps: 16, flags: FLAG_TRACKING_DEGRADE | FLAG_CONTROL_LIMIT,
        ledger: [energy_anchor_j, boundary_work_j], physical: [elongation_m, velocity_m_s])
}

fn hydraulic_fluid() -> Vec<Finding> {
    use hydraulic_fluid::*;
    kit!(hydraulic_fluid, FluidColumn::mineral_oil(), FluidState,
        start: |c| FluidState::try_pressure(c, 2.0e5).unwrap(),
        load: FluidLoad { flow_m3_s: 1.0e-6 },
        dt: 2.263838962255316e-06, steps: 16, flags: FLAG_COLUMN_SOFT | FLAG_COLUMN_LIMIT,
        ledger: [mass_anchor_kg, energy_anchor_j, boundary_mass_kg, boundary_work_j], physical: [pressure_pa])
}

fn cell_18650pf() -> Vec<Finding> {
    use cell_18650pf::*;
    kit!(cell_18650pf, Cell18650Pf::cell(), CellState,
        start: |c| CellState::try_continued(c, 0.9, 2.9).unwrap(),
        load: CellLoad { current_a: 2.9 },
        dt: 17.436, steps: 16, flags: FLAG_SAG_WARNING,
        ledger: [energy_anchor_j, electrical_work_j, joule_j], physical: [soc])
}

fn cell_2170_m80() -> Vec<Finding> {
    use cell_2170_m80::*;
    kit!(cell_2170_m80, Cell2170M80::cell(), CellState,
        start: |c| CellState::try_start(c, 25.0, 0.5).unwrap(),
        load: CellLoad { current_a: -2.25, case_temperature_c: 25.0 },
        dt: 1.0, steps: 16, flags: 0,
        ledger: [energy_anchor_j, electrical_work_j, heat_at_once_j], physical: [charge_removed_ah, branch_v])
}

fn cell_2170_model3() -> Vec<Finding> {
    use cell_2170_model3::*;
    kit!(cell_2170_model3, Cell2170Model3::cell(), CellState,
        start: |c| CellState::try_rested(c, 80.0, 0.9, 4.5006).unwrap(),
        load: CellLoad { current_a: 4.5, case_temperature_c: 25.0 },
        dt: 1.0, steps: 16, flags: FLAG_SUSTAINED_ABOVE_1C,
        ledger: [energy_anchor_j, electrical_work_j, heat_j], physical: [state_of_charge])
}

fn cell_2170_twin() -> Vec<Finding> {
    use cell_2170_twin::*;
    kit!(cell_2170_twin, Cell2170Twin::cell(), CellState,
        start: |c| CellState::try_rested(c, 80.0, 0.9, 4.2).unwrap(),
        load: CellLoad { current_a: 4.5, case_temperature_c: -20.0 },
        dt: 1.0, steps: 16, flags: FLAG_SUSTAINED_ABOVE_1C | FLAG_OUTSIDE_COMPARED_RUNS,
        ledger: [energy_anchor_j, electrical_work_j, heat_j, heat_stored_j, heat_to_case_j], physical: [state_of_charge, core_temperature_c])
}

fn generator_mmrtg_f1() -> Vec<Finding> {
    use generator_mmrtg_f1::*;
    kit!(generator_mmrtg_f1, GeneratorMmrtgF1::curiosity(), GeneratorState,
        start: |c| GeneratorState::try_mission_time(c, 8.64e6).unwrap(),
        load: GeneratorLoad {},
        dt: 86400.0, steps: 16, flags: FLAG_PAST_FITTING_SPAN,
        ledger: [energy_anchor_j, electrical_energy_j, heat_rejected_j, converter_account_anchor_j], physical: [])
}

fn generator_mmrtg_seebeck() -> Vec<Finding> {
    use generator_mmrtg_seebeck::*;
    kit!(generator_mmrtg_seebeck, GeneratorMmrtgSeebeck::curiosity(), GeneratorMmrtgSeebeckState,
        start: |c| GeneratorMmrtgSeebeckState::try_mission_time(c, 8.64e6).unwrap(),
        load: GeneratorMmrtgSeebeckLoad {},
        dt: 86400.0, steps: 16, flags: FLAG_PAST_FITTING_SPAN | FLAG_PAST_RECORD,
        ledger: [energy_anchor_j, electrical_energy_j, heat_rejected_j, converter_account_anchor_j], physical: [])
}

fn capsule_stardust() -> Vec<Finding> {
    use capsule_stardust::*;
    kit!(capsule_stardust, CapsuleStardust::sample_return_capsule(), CapsuleState,
        start: |c| CapsuleState::try_state(c, 6503140.0, -1842.1416729435698, 12767.792058801891).unwrap(),
        load: CapsuleLoad {},
        dt: 0.05, steps: 16, flags: FLAG_DROGUE_MACH_WINDOW,
        ledger: [energy_anchor_j_kg, drag_loss_j_kg], physical: [radial_speed_m_s, transverse_speed_m_s, radius_m])
}

fn satellite_lageos1() -> Vec<Finding> {
    use satellite_lageos1::*;
    kit!(satellite_lageos1, SatelliteLageos1::lageos1(), SatelliteState,
        start: |c| SatelliteState::try_state(
            c,
            841924800.0,
            0.0,
            [295558.7678903363, -5108650.370522532, -11100383.23606335],
            [-4821.08544993592, -3331.9602315669836, 1405.5539438152634],
        ).unwrap(),
        load: SatelliteLoad {},
        dt: 10.0, steps: 4, flags: FLAG_EARTH_SHADOW,
        ledger: [jacobi_anchor_j_per_kg, external_work_j_per_kg], physical: [velocity_m_s[0], position_m[0]])
}

fn solar_system() -> Vec<Finding> {
    use solar_system::*;
    let gate = SolarSystem::de440().gate_dt_s().unwrap();
    kit!(solar_system, SolarSystem::de440(), SolarSystemState,
        start: |c| SolarSystemState::try_state(c, START_1999_POSITION_M, START_1999_VELOCITY_M_S).unwrap(),
        load: SolarSystemLoad {},
        dt: gate, steps: 4, flags: 0,
        ledger: [energy_anchor, booked_work, angular_momentum_anchor[2]], physical: [velocity_m_s[3][0]])
}

fn reactor_spert3() -> Vec<Finding> {
    use reactor_spert3::*;
    kit!(reactor_spert3, ReactorSpert3::e_core_cold_startup(), ReactorState,
        start: |c| ReactorState::try_cold_start(c, 300.92777777777775).unwrap(),
        load: ReactorLoad { reactivity_inserted_dollar: 0.9982818460448638 },
        dt: 0.0007151852604202717, steps: 16, flags: FLAG_PROMPT_SUPERCRITICAL,
        ledger: [population_anchor_mw, reactivity_energy_integral_dollar_mws], physical: [power_mw, precursor_mw[0]])
}

fn cantilever_eth() -> Vec<Finding> {
    use cantilever_eth::*;
    kit!(cantilever_eth, CantileverEth::first_bending_mode(), CantileverState,
        start: |c| CantileverState::try_phase(c, -0.00104914043915228, 0.7408030982848752).unwrap(),
        load: CantileverLoad {},
        dt: 0.00025, steps: 16, flags: FLAG_ENVELOPE_BELOW_COMPARED | FLAG_ENVELOPE_ABOVE_COMPARED,
        ledger: [energy_anchor_j_per_kg, heat_j_per_kg], physical: [deflection_m, velocity_m_s])
}

fn creep_cmsx6() -> Vec<Finding> {
    use creep_cmsx6::*;
    kit!(creep_cmsx6, CreepCmsx6::at_980_c(), CreepState,
        start: |c| CreepState::try_new(c, 200.0, 0.1).unwrap(),
        load: CreepLoad { stress_mpa: 200.0 },
        dt: 49.97408036862698, steps: 16, flags: FLAG_CREEP_RATE_RISING,
        ledger: [work_by_load_mj_per_m3, heat_dissipated_mj_per_m3], physical: [creep_extension_percent])
}

fn capsule_insight() -> Vec<Finding> {
    use capsule_insight::*;
    kit!(capsule_insight, CapsuleInsight::entry_capsule(), CapsuleState,
        start: |c| CapsuleState::try_state(c, 3522199.9999655997, -1202.1585438056352, 5633.705477158609).unwrap(),
        load: CapsuleLoad { lift_over_drag: -0.031 },
        dt: 0.1, steps: 16, flags: FLAG_LIFT_ACTING,
        ledger: [energy_anchor_j_kg, drag_loss_j_kg], physical: [radial_speed_m_s, transverse_speed_m_s])
}

fn reactor_spert3_fuel() -> Vec<Finding> {
    use reactor_spert3_fuel::*;
    kit!(reactor_spert3_fuel, ReactorSpert3Fuel::e_core_low_initial_power(), ReactorState,
        start: |c| ReactorState::try_start(c, 300.92777777777775).unwrap(),
        load: ReactorLoad { reactivity_inserted_dollar: 1.0, coolant_flow_m_per_s: 0.0 },
        dt: 0.00043, steps: 16, flags: FLAG_PROMPT_SUPERCRITICAL | FLAG_ROD_MOVING,
        ledger: [population_anchor_mw, insertion_integral_dollar_mws, sweep_integral_k2], physical: [power_mw])
}

fn crater_ryugu_sci() -> Vec<Finding> {
    use crater_ryugu_sci::*;
    kit!(crater_ryugu_sci, CraterRyuguSci::sci_impact(), CraterState,
        start: |c| CraterState::try_radius(c, 1.75).unwrap(),
        load: CraterLoad {},
        dt: 0.25, steps: 16, flags: FLAG_STOPPED,
        ledger: [], physical: [wall_speed_m_s, radius_m])
}

fn link_gothenburg_10010() -> Vec<Finding> {
    use link_gothenburg_10010::*;
    kit!(link_gothenburg_10010, LinkGothenburg10010::sublink_587(), LinkState,
        start: |c| LinkState::try_new(c, 52.3, 0.0).unwrap(),
        load: LinkLoad { rain_rate_mm_h: 12.0 },
        dt: 60.0, steps: 16, flags: FLAG_RAINING,
        ledger: [dry_path_energy_j, received_energy_j, rain_energy_j, antenna_energy_j, antenna_account_anchor_db],
        physical: [antenna_water_db])
}

fn grid_gb() -> Vec<Finding> {
    use grid_gb::*;
    kit!(grid_gb, GridGb::interconnector_loss(), GridState,
        start: |c| GridState::try_start(c, 49.942, 241000.0, 27132.0).unwrap(),
        load: GridLoad { loss_mw: 996.0 },
        dt: 0.02076490883128537, steps: 16, flags: 0,
        ledger: [kinetic_energy_anchor_mw_s, energy_lost_mw_s, energy_response_mw_s], physical: [frequency_hz])
}

fn grid_gb_twin() -> Vec<Finding> {
    use grid_gb_twin::*;
    kit!(grid_gb_twin, GridGbTwin::great_britain(), GridGbTwinState,
        start: |c| GridGbTwinState::try_start(c, 49.948, 177000.0, 26774.0, ResponseHeld { dc_low_mw: 1145.0, dm_low_mw: 500.0, dr_low_mw: 480.0, dc_high_mw: 1080.0, dm_high_mw: 510.0, dr_high_mw: 480.0, fast_delivery_s: 1.0, regulation_delivery_s: 10.0 }).unwrap(),
        load: GridGbTwinLoad { loss_mw: 1007.0 },
        dt: 0.005, steps: 16, flags: 0,
        ledger: [kinetic_energy_anchor_mw_s, energy_lost_mw_s, energy_response_mw_s], physical: [frequency_hz])
}

fn tire_eve_r19_ice() -> Vec<Finding> {
    use tire_eve_r19_ice::*;
    kit!(tire_eve_r19_ice, TireEveR19Ice::on_rig_ice(), TireState,
        start: |c| TireState::try_start(c, 0.0, 5.0).unwrap(),
        load: TireLoad { normal_load_n: 5000.0, inflation_pressure_pa: 250000.0, slip_ratio: 0.12, carriage_speed_m_s: 0.06 },
        dt: 4.0, steps: 16, flags: FLAG_LOAD_ABOVE_FITTED | FLAG_BEYOND_COMPARED,
        ledger: [wheel_work_j, drawbar_work_j, slip_heat_j], physical: [])
}

fn los_jwst_fine_guide() -> Vec<Finding> {
    use los_jwst_fine_guide::*;
    kit!(los_jwst_fine_guide, LosJwstFineGuide::observatory(), LosState,
        start: |c| LosState::try_lock(c, 17.3039684, -15.2342768, 30.016).unwrap(),
        load: LosLoad { measured: 1.0, centroid_x_arcsec: 17.3038845, centroid_y_arcsec: -15.2345047, counts_per_s: 553666.4 },
        dt: 0.064, steps: 16, flags: FLAG_NO_MEASUREMENT,
        // The card's ledger is the covariance of its two axes; the estimate
        // is not in it, so no physical field is moved unbooked.
        ledger: [x.cov_mas2[0]], physical: [])
}

fn tokamak_jet_dte1() -> Vec<Finding> {
    use tokamak_jet_dte1::*;
    kit!(tokamak_jet_dte1, TokamakJetDte1::elmy_h_mode(), TokamakState,
        start: |c| TokamakState::try_start(c, 4.0e6, 5.0e5).unwrap(),
        load: TokamakLoad {
            heating_power_w: 2.0e7,
            ohmic_power_w: 5.0e5,
            line_density_per_m3: 6.0e19,
            plasma_current_a: 3.8e6,
            toroidal_field_t: 4.0,
            tritium_fraction: 0.5,
        },
        dt: 0.001, steps: 16, flags: FLAG_NEAR_DENSITY_LIMIT,
        ledger: [energy_anchor_j, energy_supplied_j, energy_lost_j], physical: [thermal_energy_j, fast_ion_energy_j])
}

fn tokamak_jet_dte1_twin() -> Vec<Finding> {
    use tokamak_jet_dte1_twin::*;
    kit!(tokamak_jet_dte1_twin, TokamakJetDte1Twin::dte1(), TwinState,
        start: |c| TwinState::try_start(c, 9.0e6, 5.0e5, 7.0e5, 4.0e5).unwrap(),
        load: TwinLoad {
            deuterium_beam_power_w: 1.05e7,
            tritium_beam_power_w: 9.5e6,
            deuterium_beam_energy_kev: 80.0,
            tritium_beam_energy_kev: 150.0,
            icrh_power_w: 2.4e6,
            ohmic_power_w: 5.0e5,
            line_density_per_m3: 6.4e19,
            plasma_current_a: 3.8e6,
            toroidal_field_t: 3.9,
            tritium_fraction: 0.451,
            electron_temperature_kev: 7.4,
            effective_charge: 2.4,
            hot_ion_regime: 0.0,
        },
        dt: 0.001, steps: 16, flags: FLAG_NEAR_DENSITY_LIMIT | FLAG_OUTSIDE_COMPARED,
        ledger: [energy_anchor_j, energy_supplied_j, energy_lost_j],
        physical: [thermal_energy_j, deuterium_beam_ion_energy_j, tritium_beam_ion_energy_j, tail_ion_energy_j])
}

fn fingerpad_p02_glass() -> Vec<Finding> {
    use fingerpad_p02_glass::*;
    kit!(fingerpad_p02_glass, FingerPadP02Glass::on_glass(), FingerPadState,
        start: |c| FingerPadState::try_start(c, 0.0).unwrap(),
        load: FingerPadLoad { normal_force_n: 2.0, twist_rate_deg_s: 20.0 },
        dt: 0.01, steps: 16, flags: FLAG_FULL_SLIP,
        ledger: [plate_work_j, friction_heat_j, stored_energy_at_start_j], physical: [twist_deg])
}

fn accumulator_sbo450() -> Vec<Finding> {
    use accumulator_sbo450::*;
    kit!(accumulator_sbo450, AccumulatorSbo450::on_darmstadt_rig(), GasState,
        start: |c| GasState::try_start(c, 0.02747, 6.0e6, 293.15, 293.55, 0.0, 0.0).unwrap(),
        load: AccumulatorLoad { piston_speed_m_s: 0.03 },
        dt: 0.0025, steps: 16, flags: 0,
        ledger: [work_on_gas_j, heat_to_wall_j, start_energy_j], physical: [gas_temperature_k, gas_volume_m3])
}

fn reactor_triga_jsi() -> Vec<Finding> {
    use reactor_triga_jsi::*;
    kit!(reactor_triga_jsi, ReactorTrigaJsi::core_56_pulse_mode(), ReactorState,
        start: |c| ReactorState::try_start(c, 298.15, 217.0).unwrap(),
        load: ReactorLoad { reactivity_inserted_dollar: 2.0, rods_dropped: 0.0 },
        dt: 9.393650793650795e-05, steps: 16, flags: FLAG_PROMPT_SUPERCRITICAL | FLAG_ROD_MOVING | FLAG_RODS_DROPPED,
        ledger: [population_anchor_mw, population_supplied_mw], physical: [power_mw])
}

fn reactor_spert3_ccore() -> Vec<Finding> {
    use reactor_spert3_ccore::*;
    kit!(reactor_spert3_ccore, ReactorSpert3Ccore::c_core_step_transients(), ReactorState,
        start: |c| ReactorState::try_start(c, 298.15, 0.0).unwrap(),
        load: ReactorLoad { reactivity_inserted_dollar: 1.1, coolant_flow_m_per_s: 0.0 },
        dt: 0.0006, steps: 16, flags: FLAG_PROMPT_SUPERCRITICAL | FLAG_PAST_POWER_PEAK,
        ledger: [population_anchor_mw, feedback_integral_dollar_mws, swept_energy_mws], physical: [power_mw, plate_rise_k])
}

fn reactor_spert3_twin() -> Vec<Finding> {
    use reactor_spert3_twin::*;
    kit!(reactor_spert3_twin, ReactorSpert3Twin::e_core(), ReactorState,
        start: |c| ReactorState::try_start(c, (76.0 + 459.67) / 1.8, 0.0853, 0.0, 1.17719e-5).unwrap(),
        load: ReactorLoad { reactivity_inserted_dollar: 1.0 },
        dt: 0.00043, steps: 16,
        flags: FLAG_ROD_MOVING | FLAG_PAST_PEAK | FLAG_HOT_SPOT_BOILING | FLAG_PROMPT_SUPERCRITICAL | FLAG_OUTSIDE_COMPARED,
        ledger: [population_anchor_mw, heat_anchor_mws, hot_heat_anchor_mws, reactivity_power_integral, heat_carried_out_mws, hot_heat_to_water_mws],
        physical: [power_mw, clad_k, hot_clad_k, water_k, energy_released_mws])
}

fn reactor_spert3_ccore_twin() -> Vec<Finding> {
    use reactor_spert3_ccore_twin::*;
    kit!(reactor_spert3_ccore_twin, ReactorSpert3CcoreTwin::c_core_twin(), ReactorState,
        start: |c| ReactorState::try_start(c, 298.15, 0.0).unwrap(),
        load: ReactorLoad { reactivity_inserted_dollar: 1.1, coolant_flow_m_per_s: 0.0 },
        dt: 0.0006, steps: 16, flags: FLAG_PROMPT_SUPERCRITICAL | FLAG_PAST_POWER_PEAK | FLAG_HOT_SPOT_BOILING | FLAG_OUTSIDE_COMPARED_RANGE,
        ledger: [population_anchor_mw, feedback_integral_dollar_mws, swept_energy_mws], physical: [power_mw, plate_rise_k, hot_spot_plate_rise_k])
}

fn reactor_spert3_ccore_plate() -> Vec<Finding> {
    use reactor_spert3_ccore_plate::*;
    kit!(reactor_spert3_ccore_plate, ReactorSpert3CcorePlate::c_core_plate(), ReactorState,
        start: |c| ReactorState::try_start(c, 298.15, 0.0).unwrap(),
        load: ReactorLoad { reactivity_inserted_dollar: 1.1, coolant_flow_m_per_s: 0.0 },
        dt: 0.0003, steps: 16, flags: FLAG_PROMPT_SUPERCRITICAL | FLAG_PAST_POWER_PEAK | FLAG_HOT_SPOT_BOILING | FLAG_OUTSIDE_COMPARED_RANGE,
        ledger: [population_anchor_mw, feedback_integral_dollar_mws, swept_energy_mws], physical: [power_mw, plate_rise_k, meat_rise_k, hot_spot_plate_rise_k])
}

fn runaway_18650_lco() -> Vec<Finding> {
    use runaway_18650_lco::*;
    kit!(runaway_18650_lco, Runaway18650Lco::hatchard_kim(), RunawayState,
        start: |c| RunawayState::try_fresh(c, 298.15).unwrap(),
        load: RunawayLoad { oven_temperature_k: 428.15, heater_power_w: 0.0, adiabatic: 0.0 },
        dt: 1.0, steps: 16, flags: FLAG_SELF_HEATING | FLAG_RUNAWAY,
        ledger: [heater_heat_j, heat_lost_j, start_temperature_k], physical: [temperature_k])
}

fn grasp_lift_glass_4tu() -> Vec<Finding> {
    use grasp_lift_glass_4tu::*;
    kit!(grasp_lift_glass_4tu, GraspLiftGlass4tu::specimen(), GraspLiftGlass4tuState,
        start: |c| GraspLiftGlass4tuState::try_start(c, 0.0).unwrap(),
        load: GraspLiftGlass4tuLoad { grip_force_n: 2.0, hand_acceleration_m_s2: 2.0 },
        dt: 0.0008, steps: 16, flags: FLAG_INCIPIENT_SLIP | FLAG_GROSS_SLIP | FLAG_ON_TABLE,
        ledger: [energy_anchor_j, hand_work_in_j, hand_work_out_j, friction_heat_j, momentum_anchor_n_s, contact_impulse_n_s, gravity_impulse_n_s, table_impulse_n_s],
        physical: [object_height_m, object_velocity_m_s, element_force_n[0], element_force_n[31]])
}

fn momentum_jwst_srp() -> Vec<Finding> {
    use momentum_jwst_srp::*;
    // The observatory on 2023-05-13 at 00:00:30 UTC, as its own conformance test starts it.
    kit!(momentum_jwst_srp, MomentumJwstSrp::observatory(), MomentumState,
        start: |c| MomentumState::try_start(c, [-1.4853731309945153, -0.6656560404607257, 1.1653501123098389]).unwrap(),
        load: MomentumLoad {
            q1: 0.7216001749, q2: 0.0366541296, q3: -0.5597470999, q4: 0.4057497233,
            sun_x: 0.6160922205743601, sun_y: 0.7226624561625701, sun_z: 0.31335179942172875, sun_distance_au: 1.0193963859269388,
        },
        dt: 60.0, steps: 16, flags: FLAG_OUTSIDE_FIELD_OF_REGARD,
        ledger: [h_anchor_eci_nms[0], h_anchor_eci_nms[1], h_anchor_eci_nms[2], impulse_eci_nms[0], impulse_eci_nms[1], impulse_eci_nms[2]],
        physical: [h_eci_nms[0], h_eci_nms[1], h_eci_nms[2]])
}

fn momentum_jwst_dihedral() -> Vec<Finding> {
    use momentum_jwst_dihedral::*;
    // The worked start and load of momentum_jwst_srp, on the two slopes.
    kit!(momentum_jwst_dihedral, MomentumJwstDihedral::observatory(), MomentumJwstDihedralState,
        start: |c| MomentumJwstDihedralState::try_start(c, [-1.4853731309945153, -0.6656560404607257, 1.1653501123098389]).unwrap(),
        load: MomentumJwstDihedralLoad {
            q1: 0.7216001749, q2: 0.0366541296, q3: -0.5597470999, q4: 0.4057497233,
            sun_x: 0.6160922205743601, sun_y: 0.7226624561625701, sun_z: 0.31335179942172875, sun_distance_au: 1.0193963859269388,
        },
        dt: 60.0, steps: 16, flags: FLAG_OUTSIDE_FIELD_OF_REGARD,
        ledger: [h_anchor_eci_nms[0], h_anchor_eci_nms[1], h_anchor_eci_nms[2], impulse_eci_nms[0], impulse_eci_nms[1], impulse_eci_nms[2]],
        physical: [h_eci_nms[0], h_eci_nms[1], h_eci_nms[2]])
}

fn tokamak_vde_mast() -> Vec<Finding> {
    use tokamak_vde_mast::*;
    // The growing mode at 1 mm, the amplifier at 0 V, as in shot 15045 after 0.250 s.
    kit!(tokamak_vde_mast, TokamakVdeMast::mast_15045(), VdeState,
        start: |c| VdeState::try_start(c, 0.001, 0.0, 0.0, 0.0).unwrap(),
        load: VdeLoad { p6_voltage_v: 0.0 },
        dt: 1.0e-4, steps: 16, flags: FLAG_PAST_IDEAL_WALL_LIMIT,
        ledger: [energy_anchor_j, energy_supplied_j, energy_dissipated_j],
        physical: [displacement_m, velocity_m_s, wall_current_a, p6_current_a])
}

fn reactor_msre_pump() -> Vec<Finding> {
    use reactor_msre_pump::*;
    kit!(reactor_msre_pump, ReactorMsrePump::specimen(), ReactorMsrePumpState,
        start: |c| ReactorMsrePumpState::try_start(c, 0.0).unwrap(),
        load: ReactorMsrePumpLoad { pump_speed_fraction: 0.05 },
        dt: 0.05, steps: 16, flags: FLAG_FLOW_CHANGING | FLAG_FUEL_CIRCULATING | FLAG_OUTSIDE_COMPARED_RUNS,
        ledger: [inventory_anchor_ws[1], born_ws[1], decayed_vessel_ws[3], decayed_outside_ws[3]],
        physical: [precursor_ws[1][40], precursor_ws[3][200]])
}

const CARDS: [(&str, fn() -> Vec<Finding>); 44] = [
    ("thermal", thermal),
    ("resonance", resonance),
    ("swing", swing),
    ("tesseract", tesseract),
    ("grid_node", grid_node),
    ("ti64_lattice", ti64_lattice),
    ("battery_sag", battery_sag),
    ("cable_stretch", cable_stretch),
    ("hydraulic_fluid", hydraulic_fluid),
    ("cell_18650pf", cell_18650pf),
    ("cell_2170_m80", cell_2170_m80),
    ("cell_2170_model3", cell_2170_model3),
    ("generator_mmrtg_f1", generator_mmrtg_f1),
    ("generator_mmrtg_seebeck", generator_mmrtg_seebeck),
    ("capsule_stardust", capsule_stardust),
    ("satellite_lageos1", satellite_lageos1),
    ("solar_system", solar_system),
    ("reactor_spert3", reactor_spert3),
    ("cantilever_eth", cantilever_eth),
    ("creep_cmsx6", creep_cmsx6),
    ("capsule_insight", capsule_insight),
    ("reactor_spert3_fuel", reactor_spert3_fuel),
    ("crater_ryugu_sci", crater_ryugu_sci),
    ("link_gothenburg_10010", link_gothenburg_10010),
    ("grid_gb", grid_gb),
    ("grid_gb_twin", grid_gb_twin),
    ("tire_eve_r19_ice", tire_eve_r19_ice),
    ("los_jwst_fine_guide", los_jwst_fine_guide),
    ("tokamak_jet_dte1", tokamak_jet_dte1),
    ("tokamak_jet_dte1_twin", tokamak_jet_dte1_twin),
    ("fingerpad_p02_glass", fingerpad_p02_glass),
    ("accumulator_sbo450", accumulator_sbo450),
    ("reactor_triga_jsi", reactor_triga_jsi),
    ("reactor_spert3_ccore", reactor_spert3_ccore),
    ("reactor_spert3_twin", reactor_spert3_twin),
    ("reactor_spert3_ccore_twin", reactor_spert3_ccore_twin),
    ("reactor_spert3_ccore_plate", reactor_spert3_ccore_plate),
    ("runaway_18650_lco", runaway_18650_lco),
    ("grasp_lift_glass_4tu", grasp_lift_glass_4tu),
    ("momentum_jwst_srp", momentum_jwst_srp),
    ("momentum_jwst_dihedral", momentum_jwst_dihedral),
    ("tokamak_vde_mast", tokamak_vde_mast),
    ("reactor_msre_pump", reactor_msre_pump),
    ("cell_2170_twin", cell_2170_twin),
];

/// Every card attacked; every finding printed and, with `CONFORMANCE_REPORT`
/// set, written to that file. The harness must reach every card's step: a
/// card whose kit does not land is a fault of the kit, not a finding.
#[test]
fn every_card_meets_the_contract_or_its_findings_are_reported() {
    let only = std::env::var("CONFORMANCE_ONLY").ok();
    let mut lines = Vec::new();
    let mut contract = 0;
    for (name, attack) in CARDS {
        if only.as_deref().is_some_and(|only| only != name) {
            continue;
        }
        let found = attack();
        for finding in &found {
            assert!(finding.rule != "harness" || !finding.detail.starts_with("no step"), "{}", finding.line());
            if finding.weight == Weight::Contract {
                contract += 1;
            }
            println!("{}", finding.line());
            lines.push(finding.json());
        }
        if found.is_empty() {
            println!("{name} | met every attack");
        }
    }
    println!("{contract} contract findings, {} findings in all", lines.len());
    if let Ok(path) = std::env::var("CONFORMANCE_REPORT") {
        std::fs::write(&path, lines.join("\n") + "\n").expect("the report is written");
    }
}

/// The harness itself: a card that holds nothing is caught on every rule
/// it breaks, and a card that holds its contract is not.
mod the_harness_catches_a_card_that_breaks_the_contract {
    use super::conformance_support::*;
    use genesis_kernel::frame::LastStateFrame64;

    #[derive(Clone, Copy, Debug)]
    pub struct Plate {
        pub capacity_j_per_k: f64,
        pub conductance_w_per_k: f64,
    }

    #[derive(Clone, Copy, Debug)]
    pub struct PlateState {
        pub temperature_k: f64,
        pub anchor_j: f64,
        pub heat_in_j: f64,
        pub residual_conservation_error: f64,
    }

    #[derive(Clone, Copy, Debug)]
    pub struct PlateLoad {
        pub power_w: f64,
    }

    #[derive(Clone, Copy, Debug)]
    pub enum Reject {
        NonFiniteInput,
        NonPositiveDt,
        SpontaneousGeneration,
    }

    const CAPACITY: f64 = 100.0;

    fn valid(plate: &Plate) -> bool {
        plate.capacity_j_per_k.to_bits() == CAPACITY.to_bits()
    }

    /// A plate that holds its ledger, its limits and its state, and writes
    /// a value of its landed state below the normal range as zero.
    fn good(plate: &Plate, state: &mut PlateState, frame: &mut LastStateFrame64, load: PlateLoad, dt_s: f64) -> Result<(), Reject> {
        good_with_subnormals(plate, state, frame, load, dt_s)?;
        for value in [&mut state.temperature_k, &mut state.anchor_j, &mut state.heat_in_j, &mut state.residual_conservation_error] {
            *value = genesis_kernel::normal::normal_or_zero(*value);
        }
        Ok(())
    }

    /// The same plate, landing a subnormal number where its arithmetic
    /// gives one.
    fn good_with_subnormals(plate: &Plate, state: &mut PlateState, frame: &mut LastStateFrame64, load: PlateLoad, dt_s: f64) -> Result<(), Reject> {
        if !dt_s.is_finite() || !load.power_w.is_finite() || !state.temperature_k.is_finite() || !state.anchor_j.is_finite() || !state.heat_in_j.is_finite() || !state.residual_conservation_error.is_finite() {
            return Err(Reject::NonFiniteInput);
        }
        if !(dt_s > 0.0) {
            return Err(Reject::NonPositiveDt);
        }
        let energy = plate.capacity_j_per_k * state.temperature_k;
        let defect = energy - state.anchor_j - state.heat_in_j;
        if defect.abs() > 1e-9 * state.anchor_j.abs().max(1.0) {
            return Err(Reject::SpontaneousGeneration);
        }
        let heat = load.power_w * dt_s;
        let temperature = (energy + heat) / plate.capacity_j_per_k;
        let residual = ((plate.capacity_j_per_k * temperature - state.anchor_j - state.heat_in_j - heat) / state.anchor_j).abs();
        if residual > 1e-9 {
            return Err(Reject::SpontaneousGeneration);
        }
        state.temperature_k = temperature;
        state.heat_in_j += heat;
        state.residual_conservation_error = residual;
        frame.t += dt_s;
        Ok(())
    }

    /// A plate that reads its conductance unheld, writes the proof, steps a
    /// negative step and keeps no ledger.
    fn bad(plate: &Plate, state: &mut PlateState, frame: &mut LastStateFrame64, load: PlateLoad, dt_s: f64) -> Result<(), Reject> {
        if !dt_s.is_finite() {
            state.temperature_k = 0.0;
            return Err(Reject::NonFiniteInput);
        }
        state.temperature_k += load.power_w * dt_s / plate.capacity_j_per_k;
        state.residual_conservation_error = plate.conductance_w_per_k;
        frame.proof[3] = 1;
        Ok(())
    }

    fn plate_kit(step: fn(&Plate, &mut PlateState, &mut LastStateFrame64, PlateLoad, f64) -> Result<(), Reject>) -> Kit<Plate, PlateState, PlateLoad, Reject> {
        Kit {
            model: "plate",
            card: Plate { capacity_j_per_k: CAPACITY, conductance_w_per_k: 0.5 },
            is_valid: valid,
            step,
            gate_dt_s: Some(1.0),
            floor_dt_s: Some(1.0e-6),
            start: PlateState { temperature_k: 300.0, anchor_j: 30000.0, heat_in_j: 0.0, residual_conservation_error: 0.0 },
            load: PlateLoad { power_w: 10.0 },
            dt_s: 0.5,
            steps: 8,
            residual: |state| state.residual_conservation_error,
            limit: 1e-9,
            flags: 0,
            ledger: vec![crate::field!(PlateState, anchor_j), crate::field!(PlateState, heat_in_j)],
            physical: vec![crate::field!(PlateState, temperature_k)],
            departures: vec![],
        }
    }

    #[test]
    fn a_plate_that_holds_the_contract_meets_every_attack() {
        let found = findings(&plate_kit(good));
        let contract: Vec<_> = found.iter().filter(|f| f.weight == Weight::Contract).map(Finding::line).collect();
        assert!(contract.is_empty(), "{contract:#?}");
        // Its conductance is a word is_valid does not hold and the step does not read.
        assert!(found.iter().any(|f| f.rule == "fixed constants" && f.detail.contains("conductance_w_per_k")), "{found:#?}");
    }

    #[test]
    fn a_plate_that_breaks_it_is_caught_on_each_rule() {
        let found = findings(&plate_kit(bad));
        let rules: Vec<&str> = found.iter().filter(|f| f.weight == Weight::Contract).map(|f| f.rule).collect();
        for rule in ["fixed constants", "proof", "refused with the state whole", "nothing filled in", "ledger attacked"] {
            assert!(rules.contains(&rule), "{rule} not found in {found:#?}");
        }
        assert!(found.iter().any(|f| f.detail.contains("conductance_w_per_k") && f.weight == Weight::Contract));
    }

    /// A heat under the smallest normal number, booked as it is, is caught;
    /// the plate that writes it as zero is not.
    #[test]
    fn a_plate_that_lands_a_subnormal_number_is_caught() {
        let found = findings(&plate_kit(good_with_subnormals));
        let caught: Vec<_> = found.iter().filter(|f| f.weight == Weight::Contract).collect();
        assert_eq!(caught.len(), 1, "{found:#?}");
        assert_eq!(caught[0].rule, "normal or zero");
        assert!(caught[0].detail.contains("load field power_w at") && caught[0].detail.contains("heat_in_j"), "{}", caught[0].detail);
        let state = PlateState { temperature_k: 300.0, anchor_j: 5.0e-324, heat_in_j: 2.0, residual_conservation_error: -1.0e-310 };
        assert_eq!(subnormal_words(&state), vec![8, 24]);
    }

    #[test]
    fn a_word_is_named_by_its_field() {
        let state = PlateState { temperature_k: 300.0, anchor_j: 1.0, heat_in_j: 2.0, residual_conservation_error: 0.0 };
        assert_eq!(name_of_word(&state, 8), "anchor_j");
        assert_eq!(number_words(&state), vec![0, 8, 16]);
    }
}
