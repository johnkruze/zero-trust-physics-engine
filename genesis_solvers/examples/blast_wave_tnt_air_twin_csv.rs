//! blast_wave_tnt_air_twin's answers as CSV, for the record in
//! evidence/research/blast_overpressure/twin/ (fit.py and compare.py call it):
//!
//!     cargo run --release --example blast_wave_tnt_air_twin_csv -- free|surface <kg of TNT> <R,R,...> [<energy factor> <ground factor>]
//!
//! With the two factors it answers with those constants in place of the
//! served ones (the fit's own door); without them, the served solver.
//! One row per distance: R, Z, arrival ms, peak kPa, Mach, duration ms,
//! impulse Pa s, Friedlander decay, products' largest radius m, the ledgers,
//! the grid changes, the steps, the seconds taken, and the refusal if refused.
use genesis_solvers::blast_wave_tnt_air::Burst;
use genesis_solvers::blast_wave_tnt_air_twin::*;
use std::time::Instant;
fn main() {
    let args: Vec<String> = std::env::args().collect();
    let burst = if args.get(1).map(|s| s == "surface").unwrap_or(false) { Burst::Surface } else { Burst::FreeAir };
    let w: f64 = args.get(2).map(|s| s.parse().unwrap()).unwrap_or(1.0);
    let rs: Vec<f64> = args.get(3).map(|s| s.split(',').map(|x| x.parse().unwrap()).collect()).unwrap_or(vec![1.0]);
    let calibration = match (args.get(4), args.get(5)) {
        (Some(e), Some(g)) => Some(Calibration { products_energy_factor: e.parse().unwrap(), ground_factor: g.parse().unwrap() }),
        _ => None,
    };
    println!("R,Z,ta_ms,ps_kpa,mach,td_ms,i_pa_s,alpha,products_max_m,shock_end_m,energy_resid,closure,area,dp,di,dd,da,steps,seconds,reject");
    for r in rs {
        let t0 = Instant::now();
        let q = TwinQuestion { charge_mass_kg: w, distance_m: r, burst };
        let answer = match &calibration { Some(c) => solve_calibrated(&q, c), None => solve(&q) };
        match answer {
            Ok(b) => println!("{r},{},{},{},{},{},{},{},{},{},{:e},{:e},{:e},{},{},{},{},{},{:.3},",
                b.scaled_distance_m_per_kg13, b.arrival_time_s*1e3, b.peak_overpressure_pa/1e3, b.shock_mach_number, b.positive_phase_duration_s*1e3, b.positive_impulse_pa_s, b.friedlander_decay, b.products_radius_max_m, b.shock_radius_at_end_m, b.energy_ledger_residual, b.impulse_closure_residual, b.weak_shock_area_residual, b.grid_change_peak, b.grid_change_impulse, b.grid_change_duration, b.grid_change_arrival, b.steps, t0.elapsed().as_secs_f64()),
            Err(e) => println!("{r},,,,,,,,,,,,,,,,,,{:.3},{}", t0.elapsed().as_secs_f64(), e.name()),
        }
    }
}
