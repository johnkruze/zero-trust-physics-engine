//! The solver contract, attacked the same way on all twenty-two solvers.
//!
//! Each solver is one `Kit`: its worked question as named numbers and a
//! function that builds the question from them and solves it. The questions
//! are the worked examples the gateway serves. `every_solver_meets_the_
//! contract_or_its_findings_are_reported` prints every finding, and writes
//! them as JSON lines to the file `CONFORMANCE_REPORT` names when it is set.

mod conformance_support;

use conformance_support::{Finding, Kit, Weight, findings};
use genesis_solvers::*;

fn named(prefix: &str, values: &[f64]) -> Vec<(String, f64)> {
    if values.len() == 1 {
        return vec![(prefix.to_string(), values[0])];
    }
    values.iter().enumerate().map(|(index, value)| (format!("{prefix}[{index}]"), *value)).collect()
}

fn debug<A: std::fmt::Debug, R: std::fmt::Debug>(result: Result<A, R>) -> Result<String, String> {
    result.map(|answer| format!("{answer:?}")).map_err(|refusal| format!("{refusal:?}"))
}

fn atom_dft() -> Kit {
    Kit { model: "atom_dft", numbers: vec![], solve: |_| debug(atom_dft::solve(10)), every_number: false }
}

fn ablator_slab() -> Kit {
    use ablator_slab::*;
    let case = ablation_workshop_test_case_1();
    let SurfaceCondition::HeldTemperature { held_temperature_k, hold_duration_s } = case.surface else { unreachable!() };
    let mut numbers = named("thickness_m", &[case.thickness_m]);
    numbers.extend(named("start_temperature_k", &[case.start_temperature_k]));
    numbers.extend(named("held_temperature_k", &[held_temperature_k]));
    numbers.extend(named("hold_duration_s", &[hold_duration_s]));
    numbers.extend(named("continue_after_s", &[case.continue_after_s]));
    numbers.extend(named("thermocouple_depths_m", &case.thermocouple_depths_m));
    Kit {
        model: "ablator_slab",
        numbers,
        solve: |n| {
            let question = SlabQuestion {
                thickness_m: n[0],
                start_temperature_k: n[1],
                surface: SurfaceCondition::HeldTemperature { held_temperature_k: n[2], hold_duration_s: n[3] },
                continue_after_s: n[4],
                thermocouple_depths_m: n[5..].to_vec(),
            };
            debug(answer(MATERIAL_TACOT, &question, &Wanted::default()))
        },
        every_number: true,
    }
}

fn journal_bearing_sheffield() -> Kit {
    Kit {
        model: "journal_bearing_sheffield",
        numbers: vec![("load_n".into(), 10000.0), ("speed_rpm".into(), 400.0)],
        solve: |n| debug(journal_bearing_sheffield::solve(n[0], n[1])),
        every_number: true,
    }
}

fn propeller_apc_10x7e() -> Kit {
    use propeller_apc_10x7e::*;
    Kit {
        model: "propeller_apc_10x7e",
        numbers: vec![("rotational_speed_rev_per_min".into(), 5000.0), ("airspeed_m_s".into(), 8.0), ("air_density_kg_m3".into(), 1.225)],
        solve: |n| debug(solve(&PropellerQuestion { rotational_speed_rev_per_min: n[0], airspeed_m_s: n[1], air_density_kg_m3: n[2] })),
        every_number: true,
    }
}

fn modulus_bam_s355() -> Kit {
    use modulus_bam_s355::*;
    Kit {
        model: "modulus_bam_s355",
        numbers: named("frequencies_hz", &[1588.627, 8950.895, 4700.015]),
        solve: |n| {
            let series = [
                Reading { mode: Mode::Flatwise, frequency_hz: n[0] },
                Reading { mode: Mode::Torsion, frequency_hz: n[1] },
                Reading { mode: Mode::Edgewise, frequency_hz: n[2] },
            ];
            debug(solve(Bar::Erx1, &series))
        },
        every_number: true,
    }
}

const SEA_PRESSURE_DBAR: [f64; 13] = [0.0, 50.0, 100.0, 150.0, 200.0, 300.0, 400.0, 500.0, 600.0, 700.0, 800.0, 900.0, 1000.0];
const SEA_TEMPERATURE_C: [f64; 13] = [21.5, 20.9, 18.2, 16.1, 14.3, 11.8, 10.2, 9.0, 7.6, 6.1, 5.0, 4.2, 3.7];
const SEA_SALINITY: [f64; 13] = [35.6, 35.6, 35.7, 35.5, 35.3, 35.0, 34.8, 34.7, 34.55, 34.45, 34.42, 34.45, 34.5];

fn seawater_column() -> Kit {
    use seawater_column::*;
    let mut numbers = named("latitude_deg", &[-31.0]);
    numbers.extend(named("sea_pressure_dbar", &SEA_PRESSURE_DBAR));
    numbers.extend(named("temperature_c", &SEA_TEMPERATURE_C));
    numbers.extend(named("practical_salinity", &SEA_SALINITY));
    numbers.extend(named("height_above_bottom_m", &[12.0]));
    Kit {
        model: "seawater_column",
        numbers,
        solve: |n| {
            let cast = Cast {
                latitude_deg: n[0],
                sea_pressure_dbar: &n[1..14],
                temperature_c: &n[14..27],
                practical_salinity: &n[27..40],
                height_above_bottom_m: n[40],
            };
            debug(solve(&cast))
        },
        every_number: true,
    }
}

fn argon_pair_gas() -> Kit {
    use argon_pair_gas::*;
    Kit {
        model: "argon_pair_gas",
        numbers: vec![("temperature_k".into(), 300.0), ("pressure_kpa".into(), 101.325)],
        solve: |n| debug(solve(Question { temperature_k: n[0], pressure_kpa: n[1] })),
        every_number: true,
    }
}

fn fusion_pb11_lhd() -> Kit {
    use fusion_pb11_lhd::*;
    Kit {
        model: "fusion_pb11_lhd",
        numbers: vec![
            // The worked question of engine 0.11.0, inside the bounds of 0.10.1 as well.
            ("beam_energy_kev".into(), 159.56),
            ("beam_power_w".into(), 1311100.0),
            ("electron_density_per_m3".into(), 4.268e18),
            ("electron_temperature_kev".into(), 3.7872),
            ("boron_density_per_m3".into(), 1.4298e17),
        ],
        solve: |n| {
            debug(solve(&LhdQuestion {
                beam_energy_kev: n[0],
                beam_power_w: n[1],
                electron_density_per_m3: n[2],
                electron_temperature_kev: n[3],
                boron_density_per_m3: n[4],
            }))
        },
        every_number: true,
    }
}

fn fusion_pb11_thermal() -> Kit {
    use fusion_pb11_thermal::*;
    Kit {
        model: "fusion_pb11_thermal",
        numbers: vec![
            ("ion_temperature_kev".into(), 300.0),
            ("electron_density_per_m3".into(), 1.6e20),
            ("boron_fraction".into(), 0.15),
            ("ash_fraction".into(), 0.0),
            ("magnetic_field_t".into(), 6.661098700751347),
            ("minor_radius_m".into(), 2.0),
            ("wall_reflectivity".into(), 0.95),
            ("energy_confinement_time_s".into(), 100.0),
        ],
        solve: |n| {
            debug(solve(&ThermalQuestion {
                ion_temperature_kev: n[0],
                electron_density_per_m3: n[1],
                boron_fraction: n[2],
                ash_fraction: n[3],
                magnetic_field_t: n[4],
                minor_radius_m: n[5],
                wall_reflectivity: n[6],
                energy_confinement_time_s: n[7],
                kinetic_enhancement: false,
            }))
        },
        every_number: true,
    }
}

fn step_flow_armaly() -> Kit {
    use step_flow_armaly::*;
    Kit {
        model: "step_flow_armaly",
        numbers: vec![("reynolds_number".into(), 100.0), ("station_x_over_s".into(), 3.06)],
        solve: |n| debug(solve(&StepQuestion { reynolds_number: n[0], station_x_over_s: n[1] })),
        every_number: true,
    }
}

fn culture_trichoderma_ambr250() -> Kit {
    use culture_trichoderma_ambr250::*;
    let mut numbers = vec![
        ("biomass_g_per_l".into(), 0.9),
        ("glucose_g_per_l".into(), 20.8),
        ("galacturonate_g_per_l".into(), 4.6),
        ("feed_glucose_g_per_l".into(), 32.0),
        ("feed_galacturonate_g_per_l".into(), 94.0),
    ];
    numbers.extend(named("segment_end_h", &[22.0, 44.0, 68.0]));
    numbers.extend(named("feed_ml", &[0.0, 4.4, 7.2]));
    numbers.extend(named("water_ml", &[-2.0, -1.0, 0.5]));
    numbers.extend(named("sample_ml", &[4.3, 4.3, 4.3]));
    Kit {
        model: "culture_trichoderma_ambr250",
        numbers,
        solve: |n| {
            debug(solve(&Question {
                biomass_g_per_l: n[0],
                glucose_g_per_l: n[1],
                galacturonate_g_per_l: n[2],
                feed_glucose_g_per_l: n[3],
                feed_galacturonate_g_per_l: n[4],
                segment_end_h: &n[5..8],
                feed_ml: &n[8..11],
                water_ml: &n[11..14],
                sample_ml: &n[14..17],
            }))
        },
        every_number: true,
    }
}

fn culture_trichoderma_atp() -> Kit {
    use culture_trichoderma_atp::*;
    let mut numbers = vec![
        ("biomass_g_per_l".into(), 0.9),
        ("glucose_g_per_l".into(), 20.8),
        ("galacturonate_g_per_l".into(), 4.6),
        ("yeast_extract_g_per_l".into(), 2.0),
        ("ph".into(), 3.5),
        ("feed_glucose_g_per_l".into(), 32.0),
        ("feed_galacturonate_g_per_l".into(), 94.0),
    ];
    numbers.extend(named("segment_end_h", &[22.0, 44.0, 68.0]));
    numbers.extend(named("feed_ml", &[0.0, 4.4, 7.2]));
    numbers.extend(named("water_ml", &[-2.0, -1.0, 0.5]));
    numbers.extend(named("sample_ml", &[4.3, 4.3, 4.3]));
    Kit {
        model: "culture_trichoderma_atp",
        numbers,
        solve: |n| {
            debug(solve(&Question {
                biomass_g_per_l: n[0],
                glucose_g_per_l: n[1],
                galacturonate_g_per_l: n[2],
                yeast_extract_g_per_l: n[3],
                ph: n[4],
                feed_glucose_g_per_l: n[5],
                feed_galacturonate_g_per_l: n[6],
                segment_end_h: &n[7..10],
                feed_ml: &n[10..13],
                water_ml: &n[13..16],
                sample_ml: &n[16..19],
            }))
        },
        every_number: true,
    }
}

const SLICE: [(&str, &[f64]); 14] = [
    ("shell_volume_m3", &[0.185232, 1.13996, 2.13983, 2.78539]),
    ("electron_temperature_kev", &[10.9616, 10.2851, 7.36622, 4.37061]),
    ("ion_temperature_kev", &[19.2543, 18.2776, 8.04433, 4.37061]),
    ("electron_density_per_m3", &[4.06983e19, 4.47374e19, 5.06836e19, 4.15438e19]),
    ("deuterium_density_per_m3", &[1.69228e19, 2.04297e19, 1.82594e19, 1.18337e19]),
    ("tritium_density_per_m3", &[1.37276e19, 1.62253e19, 1.76105e19, 1.52491e19]),
    ("impurity_density_per_m3", &[8.09588e17, 7.77101e17, 1.92707e18, 2.19865e18]),
    ("impurity_charge", &[6.0]),
    ("impurity_mass_u", &[12.0]),
    ("beam_power_w", &[77277.5, 319378.0, 593792.0, 150108.0]),
    ("beam_mass_number", &[2.0, 3.0]),
    ("beam_energy_kev", &[80.0, 152.0]),
    ("beam_power_share", &[0.5, 0.5]),
    ("time_since_beam_change_s", &[0.5503]),
];

fn tokamak_dt_beam_target() -> Kit {
    use tokamak_dt_beam_target::*;
    let numbers = SLICE.iter().flat_map(|(name, values)| named(name, values)).collect();
    Kit {
        model: "tokamak_dt_beam_target",
        numbers,
        solve: |n| {
            let mut at = 0;
            let mut take = |count: usize| {
                let part = n[at..at + count].to_vec();
                at += count;
                part
            };
            let shell_volume_m3 = take(4);
            let electron_temperature_kev = take(4);
            let ion_temperature_kev = take(4);
            let electron_density_per_m3 = take(4);
            let deuterium_density_per_m3 = take(4);
            let tritium_density_per_m3 = take(4);
            let impurity_density_per_m3 = take(4);
            let impurity_charge = take(1)[0];
            let impurity_mass_u = take(1)[0];
            let beam_power_w = take(4);
            let beam_mass_number = take(2);
            let beam_energy_kev = take(2);
            let beam_power_share = take(2);
            let time_since_beam_change_s = take(1)[0];
            debug(solve(&SliceQuestion {
                shell_volume_m3,
                electron_temperature_kev,
                ion_temperature_kev,
                electron_density_per_m3,
                deuterium_density_per_m3,
                tritium_density_per_m3,
                impurity_density_per_m3,
                impurity_charge,
                impurity_mass_u,
                beam_power_w,
                beam_mass_number,
                beam_energy_kev,
                beam_power_share,
                time_since_beam_change_s,
            }))
        },
        every_number: true,
    }
}

fn pipe_flow_superpipe() -> Kit {
    use pipe_flow_superpipe::*;
    Kit {
        model: "pipe_flow_superpipe",
        numbers: vec![
            ("bulk_reynolds_number".into(), 1.0e6),
            ("static_pressure_pa".into(), 1.47e6),
            ("temperature_k".into(), 296.0),
            ("wall_roughness_rms_um".into(), 0.15),
        ],
        solve: |n| {
            debug(solve(&PipeQuestion {
                bulk_reynolds_number: n[0],
                static_pressure_pa: n[1],
                temperature_k: n[2],
                wall_roughness_rms_um: n[3],
            }))
        },
        every_number: true,
    }
}

fn junction_nist_b7_s2() -> Kit {
    use junction_nist_b7_s2::*;
    Kit {
        model: "junction_nist_b7_s2",
        numbers: vec![("bias_current_a".into(), 0.0004)],
        solve: |n| debug(solve(&JunctionQuestion { bias_current_a: vec![n[0]], start: Start::Running, drive: Drive::None })),
        every_number: true,
    }
}

fn water_hammer_oil_line() -> Kit {
    use water_hammer_oil_line::*;
    Kit {
        model: "water_hammer_oil_line",
        numbers: vec![
            ("supply_pressure_pa".into(), 10.0e6),
            ("return_pressure_pa".into(), 0.2e6),
            ("initial_velocity_m_s".into(), 2.0),
            ("closure_time_s".into(), 0.001),
            ("free_air_fraction".into(), 0.0),
            ("duration_s".into(), 0.05),
        ],
        solve: |n| {
            debug(solve(&HammerQuestion {
                supply_pressure_pa: n[0],
                return_pressure_pa: n[1],
                initial_velocity_m_s: n[2],
                closure_time_s: n[3],
                free_air_fraction: n[4],
                duration_s: n[5],
            }))
        },
        every_number: true,
    }
}

fn orifice_air_chamber() -> Kit {
    use orifice_air_chamber::*;
    Kit {
        model: "orifice_air_chamber",
        numbers: vec![("reservoir_pressure_pa".into(), 7.0e5), ("chamber_start_pressure_pa".into(), 101325.0), ("duration_s".into(), 1.0)],
        solve: |n| debug(solve(&ChamberQuestion { reservoir_pressure_pa: n[0], chamber_start_pressure_pa: n[1], duration_s: n[2] })),
        every_number: true,
    }
}

fn pneumatic_hose_resonance() -> Kit {
    use pneumatic_hose_resonance::*;
    let frequencies = [1.0, 24.0, 50.0, 92.0, 200.0, 1000.0];
    let mut numbers = named("mean_pressure_pa", &[6.0e5]);
    numbers.extend(named("frequencies_hz", &frequencies));
    Kit {
        model: "pneumatic_hose_resonance",
        numbers,
        solve: |n| debug(solve(&HoseQuestion { mean_pressure_pa: n[0], frequencies_hz: n[1..].to_vec() })),
        every_number: true,
    }
}

fn blast_wave_tnt_air() -> Kit {
    use blast_wave_tnt_air::*;
    Kit {
        model: "blast_wave_tnt_air",
        numbers: vec![("charge_mass_kg".into(), 1.0), ("distance_m".into(), 5.0)],
        solve: |n| debug(solve(&BlastQuestion { charge_mass_kg: n[0], distance_m: n[1], burst: Burst::FreeAir })),
        every_number: true,
    }
}

fn stress_plate_hole_s235() -> Kit {
    use stress_plate_hole_s235::*;
    Kit {
        model: "stress_plate_hole_s235",
        numbers: vec![
            ("traction_mpa".into(), MEASURED_TRACTION_MPA),
            ("points_x_mm[0]".into(), 0.0),
            ("points_x_mm[1]".into(), -20.0),
            ("points_y_mm[0]".into(), 4.0),
            ("points_y_mm[1]".into(), 5.0),
        ],
        solve: |n| debug(solve(&PlateQuestion { traction_mpa: n[0], points_x_mm: vec![n[1], n[2]], points_y_mm: vec![n[3], n[4]] })),
        every_number: true,
    }
}

fn stress_plate_hole_s235_elastic() -> Kit {
    use stress_plate_hole_s235_elastic::*;
    Kit {
        model: "stress_plate_hole_s235_elastic",
        numbers: vec![
            ("traction_mpa".into(), COMPARED_TRACTION_MPA),
            ("points_x_mm[0]".into(), 0.0),
            ("points_x_mm[1]".into(), -20.0),
            ("points_y_mm[0]".into(), 4.0),
            ("points_y_mm[1]".into(), 5.0),
        ],
        solve: |n| debug(solve(&ElasticPlateQuestion { traction_mpa: n[0], points_x_mm: vec![n[1], n[2]], points_y_mm: vec![n[3], n[4]] })),
        every_number: true,
    }
}

fn icf_hotspot_burn() -> Kit {
    use icf_hotspot_burn::*;
    Kit {
        model: "icf_hotspot_burn",
        numbers: vec![
            ("hot_spot_temperature_kev".into(), 4.0),
            ("hot_spot_pressure_gbar".into(), 150.0),
            ("hot_spot_radius_um".into(), 35.0),
            ("shell_areal_density_g_per_cm2".into(), 0.8),
            ("fuel_mass_mg".into(), 0.2),
            ("ablator_mass_mg".into(), 0.0),
        ],
        solve: |n| {
            debug(solve(&HotSpotQuestion {
                hot_spot_temperature_kev: n[0],
                hot_spot_pressure_gbar: n[1],
                hot_spot_radius_um: n[2],
                shell_areal_density_g_per_cm2: n[3],
                fuel_mass_mg: n[4],
                ablator_mass_mg: n[5],
            }))
        },
        every_number: true,
    }
}

fn ln2_line_dlr_fttf() -> Kit {
    use ln2_line_dlr_fttf::*;
    Kit {
        model: "ln2_line_dlr_fttf",
        numbers: vec![
            ("temperature_k".into(), 83.7),
            ("mass_flow_kg_s".into(), 1.9739),
            ("s1_pressure_pa".into(), 1144325.0),
            ("return_pressure_pa".into(), 170000.0),
            ("stroke_time_s".into(), 0.0072555),
            ("duration_s".into(), 0.05),
        ],
        solve: |n| {
            debug(solve(&LineQuestion {
                temperature_k: n[0],
                mass_flow_kg_s: n[1],
                s1_pressure_pa: n[2],
                return_pressure_pa: n[3],
                stroke_time_s: n[4],
                duration_s: n[5],
            }))
        },
        every_number: true,
    }
}

fn plasma_sheath_ram_c2() -> Kit {
    use plasma_sheath_ram_c2::*;
    Kit {
        model: "plasma_sheath_ram_c2",
        numbers: vec![("altitude_m".into(), 71_000.0), ("velocity_m_s".into(), 7650.0), ("signal_frequency_hz".into(), 9.21e9)],
        solve: |n| debug(solve(&PlasmaSheathRamC2Question { altitude_m: n[0], velocity_m_s: n[1], signal_frequency_hz: n[2] })),
        every_number: true,
    }
}

const SOLVERS: [(&str, fn() -> Kit); 24] = [
    ("atom_dft", atom_dft),
    ("ablator_slab", ablator_slab),
    ("journal_bearing_sheffield", journal_bearing_sheffield),
    ("propeller_apc_10x7e", propeller_apc_10x7e),
    ("modulus_bam_s355", modulus_bam_s355),
    ("seawater_column", seawater_column),
    ("argon_pair_gas", argon_pair_gas),
    ("fusion_pb11_lhd", fusion_pb11_lhd),
    ("fusion_pb11_thermal", fusion_pb11_thermal),
    ("step_flow_armaly", step_flow_armaly),
    ("culture_trichoderma_ambr250", culture_trichoderma_ambr250),
    ("tokamak_dt_beam_target", tokamak_dt_beam_target),
    ("pipe_flow_superpipe", pipe_flow_superpipe),
    ("culture_trichoderma_atp", culture_trichoderma_atp),
    ("junction_nist_b7_s2", junction_nist_b7_s2),
    ("water_hammer_oil_line", water_hammer_oil_line),
    ("orifice_air_chamber", orifice_air_chamber),
    ("pneumatic_hose_resonance", pneumatic_hose_resonance),
    ("blast_wave_tnt_air", blast_wave_tnt_air),
    ("stress_plate_hole_s235", stress_plate_hole_s235),
    ("stress_plate_hole_s235_elastic", stress_plate_hole_s235_elastic),
    ("icf_hotspot_burn", icf_hotspot_burn),
    ("ln2_line_dlr_fttf", ln2_line_dlr_fttf),
    ("plasma_sheath_ram_c2", plasma_sheath_ram_c2),
];

/// Every solver attacked; every finding printed and, with
/// `CONFORMANCE_REPORT` set, written to that file. A kit whose worked
/// question is refused is reported as a harness note, and its numbers are
/// still attacked.
#[test]
fn every_solver_meets_the_contract_or_its_findings_are_reported() {
    let only = std::env::var("CONFORMANCE_ONLY").ok();
    let mut lines = Vec::new();
    for (name, kit) in SOLVERS {
        if only.as_deref().is_some_and(|only| only != name) {
            continue;
        }
        let found: Vec<Finding> = findings(&kit());
        for finding in &found {
            // A kit whose question is refused is printed as a harness note:
            // the number attacks still run from it.
            println!("{}", finding.line());
            lines.push(finding.json());
        }
        if found.is_empty() {
            println!("{name} | met every attack");
        }
    }
    let contract = lines.iter().filter(|line| line.contains("\"weight\":\"contract\"")).count();
    println!("{contract} contract findings, {} findings in all", lines.len());
    if let Ok(path) = std::env::var("CONFORMANCE_REPORT") {
        std::fs::write(&path, lines.join("\n") + "\n").expect("the report is written");
    }
}

/// The harness itself catches a solver that fills in a value, answers a
/// non-finite number, or answers differently twice.
#[test]
fn the_harness_catches_a_solver_that_breaks_the_contract() {
    let good = Kit {
        model: "square",
        numbers: vec![("side_m".into(), 2.0)],
        solve: |n| if n[0].is_finite() && n[0] > 0.0 { Ok(format!("{:?}", n[0] * n[0])) } else { Err("SideOutOfRange".into()) },
        every_number: true,
    };
    assert!(findings(&good).is_empty(), "{:?}", findings(&good));
    let filled = Kit {
        model: "square",
        numbers: vec![("side_m".into(), 2.0)],
        solve: |n| Ok(format!("{:?}", if n[0].is_nan() { 4.0 } else { n[0] * n[0] })),
        every_number: true,
    };
    let found = findings(&filled);
    assert!(found.iter().any(|f| f.rule == "nothing filled in" && f.weight == Weight::Contract && f.detail.contains("side_m = NaN: answered")), "{found:?}");
    let unchecked = Kit {
        model: "square",
        numbers: vec![("side_m".into(), 2.0)],
        solve: |n| if n[0].is_nan() { Err("NonFinite".into()) } else { Ok(format!("{:?}", n[0] * n[0])) },
        every_number: true,
    };
    let found = findings(&unchecked);
    assert!(found.iter().any(|f| f.detail.contains("side_m = inf: answered with a number that is not finite")), "{found:?}");
    // A quarter of the side, answered as computed, holds a subnormal number
    // at the smallest normal side; written by the rule, it holds zero.
    let quarter = Kit {
        model: "quarter",
        numbers: vec![("side_m".into(), 2.0)],
        solve: |n| if n[0].is_finite() && n[0] > 0.0 { Ok(format!("{:?}", n[0] * 0.25)) } else { Err("SideOutOfRange".into()) },
        every_number: true,
    };
    let found = findings(&quarter);
    assert!(found.iter().any(|f| f.rule == "normal or zero" && f.weight == Weight::Contract && f.detail.contains("side_m = 2.2250738585072014e-308: 5.562684646268003e-309")), "{found:?}");
    let held = Kit {
        solve: |n| if n[0].is_finite() && n[0] > 0.0 { Ok(format!("{:?}", genesis_solvers::normal::normal_or_zero(n[0] * 0.25))) } else { Err("SideOutOfRange".into()) },
        ..quarter
    };
    assert!(findings(&held).is_empty(), "{:?}", findings(&held));
}
