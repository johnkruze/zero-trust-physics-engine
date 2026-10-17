//! One question of `ln2_line_dlr_fttf`, its pressure at S1, S2 and S3 at
//! every step written to a CSV file, and a summary to standard output.
//!
//! cargo run --release --example ln2_line_dlr_fttf_traces -- \
//!     <temperature K> <mass flow kg/s> <S1 pressure Pa> <return pressure Pa> <stroke time s> <duration s> <out.csv>
//!
//! The temperature may be given as `density:<kg/m^3>`: the temperature at
//! which the table's liquid at the S1 pressure has that density.

use genesis_solvers::ln2_line_dlr_fttf::{solve, LineQuestion};
use std::io::Write;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 8 {
        eprintln!("usage: <temperature K> <mass flow kg/s> <S1 pressure Pa> <return pressure Pa> <stroke time s> <duration s> <out.csv>");
        std::process::exit(2);
    }
    let num = |i: usize| args[i].parse::<f64>().expect("a number");
    let temperature_k = match args[1].strip_prefix("density:") {
        Some(rho) => genesis_solvers::ln2_nitrogen::temperature_at_density_k(rho.parse().expect("a density"), num(3)).expect("a liquid of that density"),
        None => num(1),
    };
    println!("temperature_k={temperature_k:.6}");
    let question = LineQuestion {
        temperature_k,
        mass_flow_kg_s: num(2),
        s1_pressure_pa: num(3),
        return_pressure_pa: num(4),
        stroke_time_s: num(5),
        duration_s: num(6),
    };
    let started = std::time::Instant::now();
    let answer = solve(&question);
    match answer {
        Ok(h) => {
            let seconds = started.elapsed().as_secs_f64();
            let mut out = std::io::BufWriter::new(std::fs::File::create(&args[7]).expect("the output opens"));
            writeln!(out, "time_s,s1_pa,s2_pa,s3_pa").unwrap();
            let (a, b, c) = (h.station_every_step_pa(0), h.station_every_step_pa(1), h.station_every_step_pa(2));
            for i in 0..a.len() {
                writeln!(out, "{:.9},{},{},{}", i as f64 * h.time_step_s, a[i], b[i], c[i]).unwrap();
            }
            let s = &h.state;
            println!(
                "ok seconds={seconds:.3} steps={} dt={:.6e} rho0={:.3} c={:.2} a={:.2} a_sec={:.2} v={:.4} Re={:.4e} f={:.5} supply={:.1} pv={:.1} E={:.4e} joukowsky={:.1} steady_s1={:.1} s2={:.1} s3={:.1} valve={:.1} s1_max={:.1}@{:.5} s2_max={:.1}@{:.5} s3_max={:.1}@{:.5} vapour_max={:.4e} first_vapour={:?} mach={:.4} e_res={:.2e} v_res={:.2e} stored0={:.4} friction={:.4} valve_work={:.4} cavity_loss={:.4} stored_end={:.4e}",
                h.steps, h.time_step_s, s.density_kg_m3, s.sound_speed_m_s, s.wave_speed_m_s, s.section_wave_speed_m_s, s.velocity_m_s, s.reynolds_number, s.friction_factor,
                s.supply_pressure_pa, s.vapour_pressure_pa, s.steel_modulus_pa, s.joukowsky_rise_pa, h.steady_s1_pa, h.steady_s2_pa, h.steady_s3_pa, h.steady_valve_pa,
                h.s1_max_pa, h.s1_max_time_s, h.s2_max_pa, h.s2_max_time_s, h.s3_max_pa, h.s3_max_time_s, h.vapour_volume_max_m3,
                h.first_vapour_time_s.first(), h.largest_line_mach, h.largest_energy_ledger_residual, h.largest_volume_ledger_residual,
                h.stored_energy_start_j, h.friction_loss_j, h.valve_work_j, h.cavity_loss_j, h.stored_energy_end_j
            );
        }
        Err(e) => {
            println!("refused {e:?}: {e}");
            std::process::exit(1);
        }
    }
}
