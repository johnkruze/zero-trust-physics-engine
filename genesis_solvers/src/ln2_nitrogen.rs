//! Liquid nitrogen's properties for `ln2_line_dlr_fttf`: density, speed of
//! sound and viscosity of the compressed liquid, and the vapour pressure,
//! from a table of the NIST Chemistry WebBook (SRD 69, retrieved
//! 2026-10-14), which evaluates the equation of state of Span, Lemmon,
//! Jacobsen, Wagner and Yokozeki (J. Phys. Chem. Ref. Data 29 (2000) 1361)
//! and the viscosity of Lemmon and Jacobsen (Int. J. Thermophys. 25 (2004)
//! 21). Not a solver.
//!
//! The table, `ln2_nitrogen_nist.tsv`, holds 43 isotherms, at 68.561 K and
//! at every kelvin from 69 K to 110 K. Each starts with the saturated liquid
//! at the vapour pressure, then the compressed liquid from 0.001 bar over the
//! vapour pressure in steps of 2.5 bar to 262.5 bar over it, or to the
//! melting line. A state between two isotherms is interpolated linearly in
//! temperature at the same pressure over the vapour pressure (the same
//! subcooling), and along an isotherm linearly in pressure; the vapour
//! pressure between two isotherms is interpolated as `ln p` linear in `1/T`.
//! Against the WebBook's own values at ten temperatures half a kelvin off the
//! grid, 69.5 K to 109.5 K and 15 to 250 bar, the interpolation is within
//! 1.6e-4 of the speed of sound, 7.3e-5 of the density and 4.2e-4 of the
//! viscosity, and the vapour pressure within 5.4e-5 every quarter kelvin
//! (`evidence/research/hydraulics_robotics/dlr_ln2_line/analysis/table_proto.py`).
//! The equation of state's own uncertainty in the liquid is larger: about
//! 0.02 percent in density and 0.5 to 1 percent in the speed of sound (Span
//! et al. 2000).

use genesis_kernel::math;
use std::sync::OnceLock;

/// The table as the WebBook printed it.
const TABLE: &str = include_str!("ln2_nitrogen_nist.tsv");

/// Pascals in a bar.
pub const PA_PER_BAR: f64 = 1.0e5;
/// Nitrogen's triple point, kelvin (Span et al. 2000).
pub const TRIPLE_POINT_K: f64 = 63.151;
/// Nitrogen's triple-point pressure, pascals (Span et al. 2000): no liquid
/// nitrogen exists below it at any temperature.
pub const TRIPLE_POINT_PA: f64 = 12_523.0;
/// Nitrogen's critical temperature, kelvin (Span et al. 2000): at and above
/// it there is no liquid at any pressure.
pub const CRITICAL_POINT_K: f64 = 126.192;
/// The two constants of Span et al.'s (2000) melting line,
/// `p_m = p_t (1 + a ((T/T_t)^b - 1))`.
pub const MELTING_A: f64 = 12_798.61;
pub const MELTING_B: f64 = 1.789_63;
/// The NIST Chemistry WebBook, the source of the table.
pub const SOURCE_NAME: &str = "NIST Chemistry WebBook, SRD 69, Thermophysical Properties of Fluid Systems: nitrogen (the equation of state of Span, Lemmon, Jacobsen, Wagner and Yokozeki 2000; the viscosity of Lemmon and Jacobsen 2004)";
pub const SOURCE_URL: &str = "https://webbook.nist.gov/chemistry/fluid/";
pub const SOURCE_RETRIEVED: &str = "2026-10-14";

/// One state of the liquid.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LiquidState {
    pub density_kg_m3: f64,
    pub sound_speed_m_s: f64,
    pub viscosity_pa_s: f64,
}

#[derive(Debug)]
struct Isotherm {
    temperature_k: f64,
    vapour_pressure_pa: f64,
    vapour_density_kg_m3: f64,
    /// Pressure over the vapour pressure, rising from zero, and the state.
    offsets_pa: Vec<f64>,
    states: Vec<LiquidState>,
}

fn isotherms() -> &'static [Isotherm] {
    static PARSED: OnceLock<Vec<Isotherm>> = OnceLock::new();
    PARSED.get_or_init(|| {
        let mut out: Vec<Isotherm> = Vec::new();
        for line in TABLE.lines() {
            if line.starts_with('#') || line.trim().is_empty() {
                continue;
            }
            let v: Vec<f64> = line.split('\t').map(|x| x.trim().parse::<f64>().expect("the table's numbers parse")).collect();
            let (t, p, rho, c, mu, rho_v) = (v[0], v[1] * PA_PER_BAR, v[2], v[3], v[4] * 1.0e-6, v[5]);
            let state = LiquidState { density_kg_m3: rho, sound_speed_m_s: c, viscosity_pa_s: mu };
            if rho_v > 0.0 {
                out.push(Isotherm { temperature_k: t, vapour_pressure_pa: p, vapour_density_kg_m3: rho_v, offsets_pa: vec![0.0], states: vec![state] });
            } else {
                let iso = out.last_mut().expect("an isotherm starts with its saturation row");
                assert_eq!(iso.temperature_k, t);
                iso.offsets_pa.push(p - iso.vapour_pressure_pa);
                iso.states.push(state);
            }
        }
        out
    })
}

/// The coldest temperature of the table, kelvin.
pub fn table_coldest_k() -> f64 {
    isotherms()[0].temperature_k
}

/// The warmest temperature of the table, kelvin.
pub fn table_warmest_k() -> f64 {
    isotherms()[isotherms().len() - 1].temperature_k
}

/// The two isotherms about a temperature and the weight of the upper.
fn bracket(temperature_k: f64) -> Option<(&'static Isotherm, &'static Isotherm, f64)> {
    let table = isotherms();
    if !(temperature_k >= table[0].temperature_k && temperature_k <= table[table.len() - 1].temperature_k) {
        return None;
    }
    let upper = table.iter().position(|i| i.temperature_k >= temperature_k).expect("inside the table");
    let upper = upper.max(1);
    let (a, b) = (&table[upper - 1], &table[upper]);
    Some((a, b, (temperature_k - a.temperature_k) / (b.temperature_k - a.temperature_k)))
}

/// The vapour pressure, pascals: `ln p` linear in `1/T` between isotherms.
pub fn vapour_pressure_pa(temperature_k: f64) -> Option<f64> {
    let (a, b, _) = bracket(temperature_k)?;
    let w = (1.0 / temperature_k - 1.0 / a.temperature_k) / (1.0 / b.temperature_k - 1.0 / a.temperature_k);
    let (la, lb) = (math::log(a.vapour_pressure_pa), math::log(b.vapour_pressure_pa));
    Some(math::exp(la + w * (lb - la)))
}

/// The saturated vapour's density over the saturated liquid's.
pub fn saturated_vapour_over_liquid(temperature_k: f64) -> Option<f64> {
    let (a, b, w) = bracket(temperature_k)?;
    let rho_v = a.vapour_density_kg_m3 + w * (b.vapour_density_kg_m3 - a.vapour_density_kg_m3);
    let rho_l = a.states[0].density_kg_m3 + w * (b.states[0].density_kg_m3 - a.states[0].density_kg_m3);
    Some(rho_v / rho_l)
}

/// The melting pressure, pascals (Span et al. 2000): Simon's form, from the
/// triple point up; below the triple point nitrogen is solid at every
/// pressure and the form does not hold.
pub fn melting_pressure_pa(temperature_k: f64) -> f64 {
    TRIPLE_POINT_PA * (1.0 + MELTING_A * (math::powf(temperature_k / TRIPLE_POINT_K, MELTING_B) - 1.0))
}

fn along(iso: &Isotherm, offset_pa: f64) -> Option<LiquidState> {
    let offsets = &iso.offsets_pa;
    if !(offset_pa >= 0.0 && offset_pa <= offsets[offsets.len() - 1]) {
        return None;
    }
    let j = offsets.partition_point(|o| *o <= offset_pa).clamp(1, offsets.len() - 1) - 1;
    let w = (offset_pa - offsets[j]) / (offsets[j + 1] - offsets[j]);
    let (s, t) = (iso.states[j], iso.states[j + 1]);
    Some(LiquidState {
        density_kg_m3: s.density_kg_m3 + w * (t.density_kg_m3 - s.density_kg_m3),
        sound_speed_m_s: s.sound_speed_m_s + w * (t.sound_speed_m_s - s.sound_speed_m_s),
        viscosity_pa_s: s.viscosity_pa_s + w * (t.viscosity_pa_s - s.viscosity_pa_s),
    })
}

/// The compressed liquid at a temperature and an absolute pressure, or none
/// where the table does not reach: below the vapour pressure, or past its
/// last row.
pub fn liquid(temperature_k: f64, pressure_pa: f64) -> Option<LiquidState> {
    let (a, b, w) = bracket(temperature_k)?;
    let offset = pressure_pa - vapour_pressure_pa(temperature_k)?;
    let (s, t) = (along(a, offset)?, along(b, offset)?);
    Some(LiquidState {
        density_kg_m3: s.density_kg_m3 + w * (t.density_kg_m3 - s.density_kg_m3),
        sound_speed_m_s: s.sound_speed_m_s + w * (t.sound_speed_m_s - s.sound_speed_m_s),
        viscosity_pa_s: s.viscosity_pa_s + w * (t.viscosity_pa_s - s.viscosity_pa_s),
    })
}

/// The temperature at which the liquid at an absolute pressure has a
/// density, kelvin, by bisection over the table: the density falls as the
/// temperature rises. None outside the table.
pub fn temperature_at_density_k(density_kg_m3: f64, pressure_pa: f64) -> Option<f64> {
    let (mut lo, mut hi) = (table_coldest_k(), table_warmest_k());
    let at = |t: f64| liquid(t, pressure_pa).map(|s| s.density_kg_m3);
    // The warm end may be below the vapour pressure at this pressure: walk
    // the upper end down to where the liquid exists.
    while at(hi).is_none() {
        hi -= 0.5;
        if hi <= lo {
            return None;
        }
    }
    let (d_lo, d_hi) = (at(lo)?, at(hi)?);
    if !(density_kg_m3 <= d_lo && density_kg_m3 >= d_hi) {
        return None;
    }
    for _ in 0..200 {
        let mid = 0.5 * (lo + hi);
        match at(mid) {
            Some(d) if d > density_kg_m3 => lo = mid,
            Some(_) => hi = mid,
            None => hi = mid,
        }
    }
    Some(0.5 * (lo + hi))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_table_reads_whole() {
        let table = isotherms();
        assert_eq!(table.len(), 43);
        assert_eq!(table[0].temperature_k, 68.561);
        assert_eq!(table[42].temperature_k, 110.0);
        for iso in table {
            // Every isotherm rises in pressure, and the liquid stiffens as it
            // is compressed: strictly over each 2.5 bar step, and never the
            // other way between the saturated liquid and the row 0.001 bar
            // above it, which the WebBook's six digits may print alike.
            assert!(iso.offsets_pa.windows(2).all(|w| w[1] > w[0]));
            let rising = |w: &[LiquidState], strict: bool| {
                let (a, b) = (w[0], w[1]);
                if strict { b.sound_speed_m_s > a.sound_speed_m_s && b.density_kg_m3 > a.density_kg_m3 } else { b.sound_speed_m_s >= a.sound_speed_m_s && b.density_kg_m3 >= a.density_kg_m3 }
            };
            assert!(rising(&iso.states[0..2], false));
            assert!(iso.states[1..].windows(2).all(|w| rising(w, true)), "{}", iso.temperature_k);
        }
    }

    #[test]
    fn the_grid_points_come_back_as_printed() {
        // 86 K at 2.51736 bar: the WebBook's saturated liquid, 765.226 kg/m^3 and 761.891 m/s.
        let s = liquid(86.0, 2.51736e5).unwrap();
        assert!((s.density_kg_m3 - 765.226).abs() < 1e-9 && (s.sound_speed_m_s - 761.891).abs() < 1e-9, "{s:?}");
        assert!((vapour_pressure_pa(86.0).unwrap() - 2.51736e5).abs() < 1e-6);
        // Half a kelvin off the grid the vapour pressure is within 1e-4 of the WebBook's.
        assert!((vapour_pressure_pa(83.651).unwrap() / 2.00504e5 - 1.0).abs() < 1e-4);
    }

    #[test]
    fn off_the_grid_the_liquid_is_the_webbooks_to_its_digits() {
        // The WebBook's own values half a kelvin off the grid (density,
        // speed of sound, viscosity in uPa s), in the validation files.
        for (t, p_bar, rho, c, mu) in [(86.5, 15.0, 767.030, 770.274, 118.099), (86.5, 100.0, 791.984, 849.071, 136.298), (109.5, 100.0, 688.521, 651.974, 74.4590)] {
            let s = liquid(t, p_bar * PA_PER_BAR).unwrap();
            assert!((s.density_kg_m3 / rho - 1.0).abs() < 1e-4, "{t} {p_bar} {s:?}");
            assert!((s.sound_speed_m_s / c - 1.0).abs() < 2e-4, "{t} {p_bar} {s:?}");
            assert!((s.viscosity_pa_s / (mu * 1e-6) - 1.0).abs() < 5e-4, "{t} {p_bar} {s:?}");
        }
    }

    #[test]
    fn outside_the_liquid_there_is_no_state() {
        assert!(liquid(68.0, 10.0e5).is_none());
        assert!(liquid(110.5, 20.0e5).is_none());
        assert!(liquid(86.0, 2.0e5).is_none());
        assert!(liquid(86.0, 300.0e5).is_none());
    }

    #[test]
    fn the_melting_line_meets_the_triple_point() {
        assert!((melting_pressure_pa(TRIPLE_POINT_K) - TRIPLE_POINT_PA).abs() < 1e-9);
        // About 39 bar at 64 K and 275 bar at 69 K.
        assert!((melting_pressure_pa(64.0) / 38.9e5 - 1.0).abs() < 0.01);
        assert!((melting_pressure_pa(69.0) / 275.4e5 - 1.0).abs() < 0.01);
    }
}
