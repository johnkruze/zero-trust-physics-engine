//! The plasma sheath of RAM C-II at flight points read from a CSV file:
//! `altitude_m,velocity_m_s` per line (a header line and lines starting with
//! `#` are skipped). One CSV line out per point, the solver's answer or the
//! name of the limit that stopped it. Used by the record's forecast and
//! scoring; never served.
//!
//! cargo run --release -p genesis_solvers --example plasma_sheath_ram_c2_points -- points.csv

use genesis_solvers::plasma_sheath_ram_c2::{self as model, PlasmaSheathRamC2Question};
use std::time::Instant;

fn main() {
    let path = std::env::args().nth(1).expect("a CSV file of altitude_m,velocity_m_s");
    let text = std::fs::read_to_string(&path).expect("readable");
    println!(
        "altitude_m,velocity_m_s,outcome,knudsen,mach,shock_as_thick_as_layer,shock_t_k,shock_density_ratio,shock_ne_m3,stag_t_k,stag_p_pa,stag_ne_m3,\
s1_p_pa,s1_t_k,s1_ne_m3,s1_ne_eq_m3,s1_ne_frozen_m3,s1_transit_s,s2_p_pa,s2_t_k,s2_ne_m3,s2_ne_eq_m3,s2_ne_frozen_m3,s2_transit_s,\
step_tolerance,steps,shock_ledger_max,stag_ledger_max,nuclei,charge,energy,entropy_fall,seconds"
    );
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with("altitude") {
            continue;
        }
        let mut parts = line.split(',');
        let z: f64 = parts.next().unwrap().trim().parse().unwrap();
        let v: f64 = parts.next().unwrap().trim().parse().unwrap();
        let q = PlasmaSheathRamC2Question { altitude_m: z, velocity_m_s: v, signal_frequency_hz: 1.0e9 };
        let clock = Instant::now();
        let answer = model::solve(&q);
        let seconds = clock.elapsed().as_secs_f64();
        match answer {
            Err(reject) => println!("{z},{v},{},,,,,,,,,,,,,,,,,,,,,,,,,,,,,,{seconds:.4}", reject.name()),
            Ok(a) => {
                let s = &a.stations;
                println!(
                    "{z},{v},answer,{:.6e},{:.6},{},{:.6},{:.8e},{:.6e},{:.6},{:.6},{:.6e},{:.6},{:.6},{:.6e},{:.6e},{:.6e},{:.6e},{:.6},{:.6},{:.6e},{:.6e},{:.6e},{:.6e},{:e},{},{:.3e},{:.3e},{:.3e},{:.3e},{:.3e},{:.3e},{seconds:.4}",
                    a.knudsen_number,
                    a.mach_number,
                    a.shock_as_thick_as_layer,
                    a.shock_temperature_k,
                    a.shock_density_ratio,
                    a.shock_electron_density_m3,
                    a.stagnation_temperature_k,
                    a.stagnation_pressure_pa,
                    a.stagnation_electron_density_m3,
                    s[0].pressure_pa,
                    s[0].temperature_k,
                    s[0].electron_density_m3,
                    s[0].equilibrium_electron_density_m3,
                    s[0].frozen_electron_density_m3,
                    s[0].transit_time_s,
                    s[1].pressure_pa,
                    s[1].temperature_k,
                    s[1].electron_density_m3,
                    s[1].equilibrium_electron_density_m3,
                    s[1].frozen_electron_density_m3,
                    s[1].transit_time_s,
                    a.step_tolerance,
                    a.steps,
                    a.shock_mass_residual.max(a.shock_momentum_residual).max(a.shock_energy_residual),
                    a.stagnation_enthalpy_residual.max(a.stagnation_entropy_residual),
                    a.streamline_nuclei_residual,
                    a.streamline_charge_residual,
                    a.streamline_energy_residual,
                    a.streamline_entropy_fall,
                )
            }
        }
    }
}
