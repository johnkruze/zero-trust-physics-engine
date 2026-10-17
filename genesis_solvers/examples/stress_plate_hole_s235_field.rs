//! The field of `stress_plate_hole_s235` at many points, for the record's
//! programs in `evidence/research/stress_plate_s235/analysis/`: the
//! calibration on the opened band and the scoring.
//!
//! ```text
//! cargo run --release --example stress_plate_hole_s235_field -- \
//!     <traction MPa> <Young's modulus MPa | served> <Poisson's ratio | served> <yields: yes | no> <points.csv> <out.csv>
//! ```
//!
//! `points.csv` has a header and `x_mm,y_mm` (mm from the hole's centre)
//! first on each line. `out.csv` gets, for each point, `x_mm, y_mm`, the
//! displacement, the stress and the strain there, and a last line of the
//! field's own numbers.

use genesis_solvers::stress_plate_hole_s235 as plate;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 7 {
        eprintln!("usage: <traction MPa> <E MPa | served> <nu | served> <yes | no> <points.csv> <out.csv>");
        std::process::exit(2);
    }
    let traction: f64 = args[1].parse().expect("a traction");
    let youngs = if args[2] == "served" { plate::YOUNGS_MODULUS_MPA } else { args[2].parse().expect("a modulus") };
    let poisson = if args[3] == "served" { plate::POISSON_RATIO } else { args[3].parse().expect("a ratio") };
    let yields = match args[4].as_str() {
        "yes" => true,
        "no" => false,
        other => panic!("yields is yes or no, not {other}"),
    };
    let text = std::fs::read_to_string(&args[5]).expect("the points");
    let points: Vec<(f64, f64)> = text
        .lines()
        .skip(1)
        .filter(|line| !line.trim().is_empty())
        .map(|line| {
            let mut it = line.split(',');
            let x: f64 = it.next().expect("x").trim().parse().expect("x");
            let y: f64 = it.next().expect("y").trim().parse().expect("y");
            (x, y)
        })
        .collect();
    let (field, at) = match plate::probe_with_moduli(traction, youngs, poisson, yields, &points) {
        Ok(found) => found,
        Err(reject) => {
            eprintln!("refused: {} ({})", reject, reject.name());
            std::process::exit(1);
        }
    };
    let mut out = String::from("x_mm,y_mm,u_x_mm,u_y_mm,stress_xx_mpa,stress_yy_mpa,stress_xy_mpa,von_mises_mpa,strain_xx,strain_yy,strain_xy\n");
    for ((x, y), p) in points.iter().zip(&at) {
        out.push_str(&format!(
            "{x:?},{y:?},{:?},{:?},{:?},{:?},{:?},{:?},{:?},{:?},{:?}\n",
            p.displacement_x_mm, p.displacement_y_mm, p.stress_xx_mpa, p.stress_yy_mpa, p.stress_xy_mpa, p.von_mises_mpa, p.strain_xx, p.strain_yy, p.strain_xy
        ));
    }
    std::fs::write(&args[6], out).expect("written");
    println!(
        "{{\"traction_mpa\": {traction:?}, \"youngs_modulus_mpa\": {youngs:?}, \"poisson_ratio\": {poisson:?}, \"yields\": {yields}, \"level\": {}, \"elements\": {}, \"hole_edge_stress_mpa\": {:?}, \"elastic_hole_edge_stress_mpa\": {:?}, \"stress_concentration_net\": {:?}, \"first_yield_traction_mpa\": {:?}, \"plastic_zone_area_mm2\": {:?}, \"plastic_zone_reach_mm\": {:?}, \"largest_strain\": {:?}, \"equilibrium_residual\": {:?}, \"energy_residual\": {:?}, \"section_force_residual\": {:?}, \"hole_traction_residual\": {:?}, \"displacement_change\": {:?}, \"stress_change\": {:?}, \"dissipation_change\": {:?}, \"points\": {}}}",
        field.level, field.elements, field.hole_edge_stress_mpa, field.elastic_hole_edge_stress_mpa, field.stress_concentration_net, field.first_yield_traction_mpa, field.plastic_zone_area_mm2, field.plastic_zone_reach_mm, field.largest_strain, field.equilibrium_residual, field.energy_residual, field.section_force_residual, field.hole_traction_residual, field.displacement_change, field.stress_change, field.dissipation_change, points.len()
    );
}
