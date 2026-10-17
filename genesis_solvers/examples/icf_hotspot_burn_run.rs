//! Runs `icf_hotspot_burn` on one question given on the command line and
//! prints the answer as JSON, without its history unless `--history` is given:
//!
//! ```text
//! cargo run --release --example icf_hotspot_burn_run -- T_keV p_Gbar R_um rhoR_shell_g_per_cm2 fuel_mass_mg [ablator_mass_mg] [--history]
//! ```
//!
//! With five numbers the ablator mass is zero.
//!
//! ```text
//! ```

use genesis_solvers::icf_hotspot_burn::{HotSpotQuestion, solve};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let numbers: Vec<f64> = args.iter().filter(|a| !a.starts_with("--")).map(|a| a.parse().expect("a number")).collect();
    if numbers.len() != 5 && numbers.len() != 6 {
        eprintln!("five or six numbers: temperature keV, pressure Gbar, radius um, shell areal density g/cm2, fuel mass mg, ablator mass mg");
        std::process::exit(2);
    }
    let question = HotSpotQuestion {
        hot_spot_temperature_kev: numbers[0],
        hot_spot_pressure_gbar: numbers[1],
        hot_spot_radius_um: numbers[2],
        shell_areal_density_g_per_cm2: numbers[3],
        fuel_mass_mg: numbers[4],
        ablator_mass_mg: numbers.get(5).copied().unwrap_or(0.0),
    };
    match solve(&question) {
        Err(reject) => println!("{{\"refused\": \"{reject:?}\", \"sentence\": \"{reject}\"}}"),
        Ok(burn) => {
            let text = format!("{burn:?}");
            let body = if args.iter().any(|a| a == "--history") {
                text
            } else {
                text.split(", history:").next().unwrap_or("").to_string() + " }"
            };
            println!("{body}");
        }
    }
}
