//! blast_wave_tnt_air's answers as CSV, for the record in
//! evidence/research/blast_overpressure/analysis/ (inputs.py, compare_tnt.py,
//! after_scoring_energy_fraction.py call it):
//!
//!     cargo run --release --example blast_wave_tnt_air_csv -- free|surface <kg of TNT> <R,R,...>
//!
//! One row per distance: R, Z, lambda, arrival ms, peak kPa, Mach, duration ms,
//! impulse Pa s, Friedlander decay, Sedov-Taylor kPa, the ledgers, the grid
//! changes, the steps, the seconds taken, and the refusal's name if refused.
//! The scoring of 2026-10-14 ran this file under the name blast_probe.
use genesis_solvers::blast_wave_tnt_air::*;
use std::time::Instant;
fn main() {
    let args: Vec<String> = std::env::args().collect();
    let burst = if args.get(1).map(|s| s == "surface").unwrap_or(false) { Burst::Surface } else { Burst::FreeAir };
    let w: f64 = args.get(2).map(|s| s.parse().unwrap()).unwrap_or(1.0);
    let rs: Vec<f64> = args.get(3).map(|s| s.split(',').map(|x| x.parse().unwrap()).collect()).unwrap_or(vec![1.0]);
    println!("R,Z,lambda,ta_ms,ps_kpa,mach,td_ms,i_pa_s,alpha,sedov_kpa,energy_resid,closure,area,dp,di,dd,da,steps,seconds,reject");
    for r in rs {
        let t0 = Instant::now();
        match solve(&BlastQuestion { charge_mass_kg: w, distance_m: r, burst }) {
            Ok(b) => println!("{r},{},{},{},{},{},{},{},{},{},{:e},{:e},{:e},{},{},{},{},{},{:.3},",
                b.scaled_distance_m_per_kg13, b.sachs_scaled_distance, b.arrival_time_s*1e3, b.peak_overpressure_pa/1e3, b.shock_mach_number, b.positive_phase_duration_s*1e3, b.positive_impulse_pa_s, b.friedlander_decay, b.sedov_taylor_overpressure_pa/1e3, b.energy_ledger_residual, b.impulse_closure_residual, b.weak_shock_area_residual, b.grid_change_peak, b.grid_change_impulse, b.grid_change_duration, b.grid_change_arrival, b.steps, t0.elapsed().as_secs_f64()),
            Err(e) => println!("{r},,,,,,,,,,,,,,,,,,{:.3},{}", t0.elapsed().as_secs_f64(), e.name()),
        }
    }
}
