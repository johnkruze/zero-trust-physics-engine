//! The proton-boron-11 reaction and the slowing-down of a fast ion: the one
//! foundation the two proton-boron solvers share.
//!
//! This file is not a solver. `fusion_pb11_lhd` and `fusion_pb11_thermal`
//! both take their cross-section, their reactivity and their test-particle
//! energy-loss rates from here, so the two cannot disagree on them. The
//! slowing-down rates, the Coulomb logarithms, [`Background`], [`Species`]
//! and the beam-target rate [`beam_target_rate_for_m3_per_s`] are of no one
//! fuel: `tokamak_dt_beam_target` takes them from here too, with the
//! deuterium-tritium cross-sections of `bosch_hale`.
//!
//! The reaction is p + ¹¹B → 3 α, 8.68 MeV. The cross-section is the S-factor
//! fit of Tentori and Belloni, Nucl. Fusion 63 (2023) 086001, Table 1
//! (CC BY 4.0), with the 148 keV term of Nevins and Swain (2000):
//!
//! ```text
//! σ(E) = S(E)/E · exp(−√(E_G/E))        E centre of mass, keV; S in MeV b; the 1/E in MeV
//! E ≤ 400:        S = 197 + 0.269 E + 2.54e-4 E² + 1.82e4 / [(E − 148)² + 2.35²]
//! 400 < E ≤ 668:  S = 346 + 150 x − 59.9 x² − 0.460 x⁵,  x = (E − 400)/100
//! 668 < E ≤ 9760: S = 0.381 + Σ A_k / [(E − E_k)² + δ_k²]
//! ```
//!
//! Above 9760 keV the fit is not defined and [`cross_section_barn`] names it.
//! The Maxwellian reactivity is the integral of that fit,
//! `⟨σv⟩ = √(8/(π μc²)) · c · T^(−3/2) ∫ σ E e^(−E/T) dE`, with the papers'
//! `μc² = 859 526 keV` and `E_G = 22 589 keV`. It is declared from 1 to
//! 1000 keV, each end by the integral's own arithmetic. The energies that
//! carry the rate, the Gamow window `E₀ ± Δ/2`, `E₀ = (E_G T²/4)^(1/3)`,
//! `Δ = 4 √(E₀ T / 3)`, run from 12.9 to 22.7 keV at 1 keV, from 50 to
//! 116 keV at 10 keV and from 241 to 3321 keV at 1000 keV. At the bottom the
//! panels, 1 keV wide below 400 keV, hold that window within 1e-13 of a rule
//! a hundred times finer. At the top the integral stops where the fit does,
//! 9760 keV, and the part of the rate above it, which the integral leaves
//! out, is 1.7e-5 at 1000 keV and grows past it (3.6e-4 at 1500 keV). The
//! fit rests on measured points from 22 keV (Becker 1987) to 3481 keV
//! (Sikora and Weller 2016): the window leaves the last of them at about
//! 1060 keV, and falls below the first under about 3 keV, where the rate
//! rests on the fit's S-factor near zero energy, 197 MeV b (NACRE II:
//! 210, +20 or −30). Until this version the bottom was 10 keV. A second
//! quadrature, Simpson's rule on its own grid, gives the same integral
//! within 1e-10 from 1 to 1000 keV (test
//! `a_second_quadrature_gives_the_same_reactivity_to_1000_kev`). The
//! evaluations differ by about ±10 % from 10 to 60 keV and by −15 % / +5 %
//! from 70 to 500 keV; above 500 keV and below 10 keV no other evaluation
//! was integrated. The research record is
//! `evidence/research/fusion_pb11/reaction_rates/FINDINGS.md`.
//!
//! The D-T card `tokamak_jet_dte1` of `genesis_kernel` uses the same units
//! and names for its reactivity: temperature in keV, `⟨σv⟩` in m³/s.
//!
//! The energy a fast ion of charge `Z`, mass `m` and speed `v` gives per
//! second to a Maxwellian species `s` is the test-particle rate of the NRL
//! Plasma Formulary:
//!
//! ```text
//! dE/dt|s = − Z² Z_s² e⁴ n_s lnΛ_s / (4π ε0² m_s v) · [ψ(x_s) − (m_s/m) ψ′(x_s)]
//! x_s = m_s v² / (2 T_s)        ψ(x) = erf(√x) − (2/√π) √x e^(−x)
//! ```
//!
//! with the Formulary's Coulomb logarithms (electrons: `24 − ln(√n_e/T_e)`;
//! ions: the counter-streaming form) and, on the electron channel, the
//! relativistic factor of Putvinski, Ryutov and Yushmanov (2019) as written
//! in Ochs et al. (2022), equation A7.

use genesis_kernel::math;
use std::f64::consts::PI;

/// Elementary charge, C (SI 2019, exact).
pub const ELEMENTARY_CHARGE_C: f64 = 1.602176634e-19;
/// Electron mass, kg (CODATA 2018).
pub const ELECTRON_MASS_KG: f64 = 9.1093837015e-31;
/// Proton mass, kg (CODATA 2018).
pub const PROTON_MASS_KG: f64 = 1.67262192369e-27;
/// Atomic mass constant, kg (CODATA 2018).
pub const ATOMIC_MASS_KG: f64 = 1.66053906660e-27;
/// Mass of the bare boron-11 nucleus, kg: 11.0093054 u less five electrons.
pub const BORON11_MASS_KG: f64 = 11.0093054 * ATOMIC_MASS_KG - 5.0 * ELECTRON_MASS_KG;
/// Mass of the alpha particle, kg (CODATA 2018).
pub const ALPHA_MASS_KG: f64 = 4.001506179 * ATOMIC_MASS_KG;
/// Speed of light, m/s (exact).
pub const SPEED_OF_LIGHT_M_PER_S: f64 = 299_792_458.0;
/// Vacuum permittivity, F/m (CODATA 2018).
pub const VACUUM_PERMITTIVITY_F_PER_M: f64 = 8.8541878128e-12;
/// Vacuum permeability, H/m (CODATA 2018).
pub const VACUUM_PERMEABILITY_H_PER_M: f64 = 1.25663706212e-6;
/// Fine-structure constant (CODATA 2018).
pub const FINE_STRUCTURE: f64 = 7.2973525693e-3;
/// Classical electron radius, m (CODATA 2018).
pub const ELECTRON_RADIUS_M: f64 = 2.8179403262e-15;
/// Electron rest energy, keV (CODATA 2018).
pub const ELECTRON_REST_KEV: f64 = 510.99895;
/// Joules in one keV.
pub const JOULE_PER_KEV: f64 = 1.0e3 * ELEMENTARY_CHARGE_C;

/// Energy of one p + ¹¹B reaction, keV (Putvinski, Ryutov and Yushmanov 2019, equation 1).
pub const REACTION_ENERGY_KEV: f64 = 8.68e3;
/// Gamow energy of the fit, keV (Tentori and Belloni 2023).
pub const GAMOW_ENERGY_KEV: f64 = 22_589.0;
/// Reduced mass energy of the fit, keV (Tentori and Belloni 2023).
pub const REDUCED_MASS_KEV: f64 = 859_526.0;
/// Highest centre-of-mass energy of the cross-section fit, keV.
pub const CROSS_SECTION_ENERGY_MAX_KEV: f64 = 9760.0;
/// Lowest temperature of the declared reactivity, keV: the quadrature's
/// panels, 1 keV wide below 400 keV, hold the Gamow window there (12.9 to
/// 22.7 keV) within 1e-13 of a rule a hundred times finer (test
/// `the_ends_of_the_reactivity_are_where_its_integral_holds`). It was 10 keV
/// until this version.
pub const REACTIVITY_TEMPERATURE_MIN_KEV: f64 = 1.0;
/// Highest temperature of the declared reactivity, keV: the integral stops at
/// the top of the cross-section fit, [`CROSS_SECTION_ENERGY_MAX_KEV`], and the
/// part of the rate above it, which the integral leaves out, is 1.7e-5 of the
/// rate here and grows past it (3.6e-4 at 1500 keV). At about 1060 keV the
/// Gamow window also reaches past the fit's last measured point, 3481 keV
/// (Sikora and Weller 2016), into the digitised curve. It was 500 keV, the
/// top of the published reactivity fit's comparison, until engine 0.11.0.
pub const REACTIVITY_TEMPERATURE_MAX_KEV: f64 = 1000.0;
/// Lowest and highest centre-of-mass energies of the measured points the
/// cross-section fit was made on, keV (Tentori and Belloni 2023: Becker 1987
/// from 22 keV, Sikora and Weller 2016 to 3481 keV).
pub const MEASURED_POINTS_MIN_KEV: f64 = 22.0;
pub const MEASURED_POINTS_MAX_KEV: f64 = 3481.0;

/// The Gamow window at `temperature_kev`, keV: its centre `E₀ = (E_G T²/4)^(1/3)`
/// and its width `Δ = 4 √(E₀ T / 3)`, the energies that carry most of the
/// Maxwellian rate of a reaction of Gamow energy `E_G`.
pub fn gamow_window_kev(temperature_kev: f64) -> (f64, f64) {
    let centre = math::cbrt(GAMOW_ENERGY_KEV * temperature_kev * temperature_kev / 4.0);
    (centre, 4.0 * (centre * temperature_kev / 3.0).sqrt())
}

/// The published fit the reactivity is checked against.
pub const REFERENCE_NAME: &str = "Tentori and Belloni (2023), 11B(p,α)2α reactivity revisited, Nucl. Fusion 63, 086001: the reactivity fit of Table 2, stated by its authors to be within 1 % of their integral from 70 to 500 keV";
pub const REFERENCE_URL: &str = "https://doi.org/10.1088/1741-4326/acda4b";
/// Date the reference was read.
pub const REFERENCE_RETRIEVED: &str = "2026-10-09";
/// The reactivity agrees with the published fit to this fraction from 70 to 500 keV.
pub const REFERENCE_TOLERANCE: f64 = 0.01;

/// Nodes of the eight-point Gauss-Legendre rule on (−1, 1).
pub const GAUSS_NODES: [f64; 8] = [
    -0.960_289_856_497_536_3,
    -0.796_666_477_413_626_7,
    -0.525_532_409_916_329,
    -0.183_434_642_495_649_8,
    0.183_434_642_495_649_8,
    0.525_532_409_916_329,
    0.796_666_477_413_626_7,
    0.960_289_856_497_536_3,
];
/// Weights of the eight-point Gauss-Legendre rule.
pub const GAUSS_WEIGHTS: [f64; 8] = [
    0.101_228_536_290_376_3,
    0.222_381_034_453_374_5,
    0.313_706_645_877_887_3,
    0.362_683_783_378_362,
    0.362_683_783_378_362,
    0.313_706_645_877_887_3,
    0.222_381_034_453_374_5,
    0.101_228_536_290_376_3,
];

/// The integral of `f` from `low` to `high` over `panels` equal panels of the eight-point rule.
pub fn integrate(low: f64, high: f64, panels: usize, mut f: impl FnMut(f64) -> f64) -> f64 {
    let width = (high - low) / panels as f64;
    let half = 0.5 * width;
    let mut sum = 0.0;
    for panel in 0..panels {
        let middle = low + (panel as f64 + 0.5) * width;
        let mut inside = 0.0;
        for (node, weight) in GAUSS_NODES.iter().zip(GAUSS_WEIGHTS) {
            inside += weight * f(middle + half * node);
        }
        sum += half * inside;
    }
    sum
}

/// The astrophysical S-factor of the fit, MeV b, at a centre-of-mass energy in keV.
fn s_factor_mev_b(energy_kev: f64) -> f64 {
    let e = energy_kev;
    if e <= 400.0 {
        197.0 + 0.269 * e + 2.54e-4 * e * e + 1.82e4 / ((e - 148.0) * (e - 148.0) + 2.35 * 2.35)
    } else if e <= 668.0 {
        let x = (e - 400.0) / 100.0;
        346.0 + 150.0 * x - 59.9 * x * x - 0.460 * x * x * x * x * x
    } else {
        const A: [f64; 4] = [1.98e6, 3.89e6, 1.36e6, 3.71e6];
        const E: [f64; 4] = [640.9, 1211.0, 2340.0, 3294.0];
        const D: [f64; 4] = [85.5, 414.0, 221.0, 351.0];
        let mut s = 0.381;
        for k in 0..4 {
            s += A[k] / ((e - E[k]) * (e - E[k]) + D[k] * D[k]);
        }
        s
    }
}

/// The p + ¹¹B cross-section in barns at a centre-of-mass energy in keV, or
/// `None` outside `0 < E ≤` [`CROSS_SECTION_ENERGY_MAX_KEV`].
pub fn cross_section_barn(energy_kev: f64) -> Option<f64> {
    if !(energy_kev > 0.0 && energy_kev <= CROSS_SECTION_ENERGY_MAX_KEV) {
        return None;
    }
    Some(s_factor_mev_b(energy_kev) * 1.0e3 / energy_kev * math::exp(-(GAMOW_ENERGY_KEV / energy_kev).sqrt()))
}

/// The cross-section inside an integral: zero where the fit is not defined.
/// Every integral here stops at or below the fit's highest energy, or carries
/// a weight under 1e-15 there.
fn cross_section_or_zero(energy_kev: f64) -> f64 {
    cross_section_barn(energy_kev).unwrap_or(0.0)
}

/// The Maxwellian reactivity integral at any temperature, m³/s. The panels
/// are 1 keV wide to 400 keV, so the 4.7 keV wide term at 148 keV is resolved,
/// 4 keV to 1500 keV and 20 keV above, and split at the joins of the fit.
pub(crate) fn reactivity_integral_m3_per_s(temperature_kev: f64) -> f64 {
    let t = temperature_kev;
    let weight = |e: f64| cross_section_or_zero(e) * e * math::exp(-e / t);
    let integral = integrate(0.0, 400.0, 400, weight)
        + integrate(400.0, 668.0, 67, weight)
        + integrate(668.0, 1500.0, 208, weight)
        + integrate(1500.0, CROSS_SECTION_ENERGY_MAX_KEV, 413, weight);
    (8.0 / (PI * REDUCED_MASS_KEV)).sqrt() * SPEED_OF_LIGHT_M_PER_S * integral * 1.0e-28 / (t * t.sqrt())
}

/// The Maxwellian p + ¹¹B reactivity in m³/s at an ion temperature in keV, or
/// `None` outside [`REACTIVITY_TEMPERATURE_MIN_KEV`]..=[`REACTIVITY_TEMPERATURE_MAX_KEV`].
pub fn reactivity_pb11_m3_per_s(temperature_kev: f64) -> Option<f64> {
    if !(REACTIVITY_TEMPERATURE_MIN_KEV..=REACTIVITY_TEMPERATURE_MAX_KEV).contains(&temperature_kev) {
        return None;
    }
    Some(reactivity_integral_m3_per_s(temperature_kev))
}

/// The reduced mass of proton and boron-11 over the proton mass: the
/// centre-of-mass energy is this times the energy of the relative motion
/// reckoned on the proton.
pub fn reduced_mass_over_proton_mass() -> f64 {
    BORON11_MASS_KG / (PROTON_MASS_KG + BORON11_MASS_KG)
}

/// `⟨σ v⟩` in m³/s for one proton of `proton_energy_kev` moving through
/// boron-11 nuclei at `boron_temperature_kev`: [`beam_target_rate_for_m3_per_s`]
/// with this file's cross-section, the proton mass and the boron-11 mass.
pub fn beam_target_rate_m3_per_s(proton_energy_kev: f64, boron_temperature_kev: f64, panel_kev: f64) -> f64 {
    beam_target_rate_for_m3_per_s(
        cross_section_or_zero,
        PROTON_MASS_KG,
        BORON11_MASS_KG,
        proton_energy_kev,
        boron_temperature_kev,
        panel_kev,
    )
}

/// `⟨σ v⟩` in m³/s for one ion of mass `beam_mass_kg` and energy
/// `beam_energy_kev` moving through target nuclei of mass `target_mass_kg`
/// at `target_temperature_kev`: the cross-section averaged over the thermal
/// motion of the target,
///
/// ```text
/// ⟨σv⟩ = 1/(√π v_b v_t) ∫ σ(u) u² [e^(−(u−v_b)²/v_t²) − e^(−(u+v_b)²/v_t²)] du,   v_t² = 2T/m_target
/// ```
///
/// over six thermal speeds each side of the ion's speed, in panels
/// `panel_kev` wide in the energy of the relative motion reckoned on the
/// beam ion. `cross_section_barn` takes the centre-of-mass energy in keV,
/// `½ μ u²` with `μ` the reduced mass, and returns barns; it must return
/// zero where its fit is not defined. Any reaction's cross-section and any
/// pair of masses may be given: the proton-boron-11 solvers and the
/// deuterium-tritium solver all take their beam-target rate from here.
pub fn beam_target_rate_for_m3_per_s(
    cross_section_barn: impl Fn(f64) -> f64,
    beam_mass_kg: f64,
    target_mass_kg: f64,
    beam_energy_kev: f64,
    target_temperature_kev: f64,
    panel_kev: f64,
) -> f64 {
    let beam_speed = (2.0 * beam_energy_kev * JOULE_PER_KEV / beam_mass_kg).sqrt();
    let thermal_speed = (2.0 * target_temperature_kev * JOULE_PER_KEV / target_mass_kg).sqrt();
    let low = (beam_speed - 6.0 * thermal_speed).max(0.0);
    let high = beam_speed + 6.0 * thermal_speed;
    let speed_step = panel_kev * JOULE_PER_KEV / (beam_mass_kg * beam_speed);
    let panels = (((high - low) / speed_step).ceil() as usize).max(8);
    let to_centre_of_mass_kev = 0.5 * (target_mass_kg / (beam_mass_kg + target_mass_kg)) * beam_mass_kg / JOULE_PER_KEV;
    let integral = integrate(low, high, panels, |u| {
        let near = (u - beam_speed) / thermal_speed;
        let far = (u + beam_speed) / thermal_speed;
        cross_section_barn(to_centre_of_mass_kev * u * u) * 1.0e-28 * u * u * (math::exp(-near * near) - math::exp(-far * far))
    });
    integral / (PI.sqrt() * beam_speed * thermal_speed)
}

/// `ψ(x) = erf(√x) − (2/√π) √x e^(−x)`, the regularized lower incomplete gamma
/// function of order 3/2, and its derivative `ψ′(x) = (2/√π) √x e^(−x)`.
/// A series below `x = 2.5`, a continued fraction above.
pub fn psi_and_derivative(x: f64) -> (f64, f64) {
    let derivative = 2.0 / PI.sqrt() * x.sqrt() * math::exp(-x);
    if x <= 0.0 {
        return (0.0, 0.0);
    }
    // Γ(3/2) = √π / 2, so x^(3/2) e^(−x) / Γ(3/2) = x ψ′(x).
    let scale = x * derivative;
    if x < 2.5 {
        // γ(a, x) = x^a e^(−x) Σ x^n / (a (a+1) … (a+n)),  a = 3/2
        let mut term = 1.0 / 1.5;
        let mut sum = term;
        let mut a = 1.5;
        for _ in 0..200 {
            a += 1.0;
            term *= x / a;
            sum += term;
            if term <= 1.0e-17 * sum {
                break;
            }
        }
        (scale * sum, derivative)
    } else {
        // Γ(a, x) = x^a e^(−x) / (x + 1 − a − 1·(1 − a)/(x + 3 − a − …)), by Lentz's method.
        let a = 1.5;
        let tiny = 1.0e-300;
        let mut b = x + 1.0 - a;
        let mut c = 1.0 / tiny;
        let mut d = 1.0 / b;
        let mut h = d;
        for i in 1..400 {
            let an = -(i as f64) * (i as f64 - a);
            b += 2.0;
            d = an * d + b;
            if d.abs() < tiny {
                d = tiny;
            }
            c = b + an / c;
            if c.abs() < tiny {
                c = tiny;
            }
            d = 1.0 / d;
            let delta = d * c;
            h *= delta;
            if (delta - 1.0).abs() < 1.0e-16 {
                break;
            }
        }
        (1.0 - scale * h, derivative)
    }
}

/// Coulomb logarithm of electrons with ions, NRL Plasma Formulary, the branch
/// for `T_e` above `10 Z²` eV: `24 − ln(√n_e[cm⁻³] / T_e[eV])`.
pub fn coulomb_logarithm_electron(electron_density_per_m3: f64, electron_temperature_kev: f64) -> f64 {
    24.0 - math::log((electron_density_per_m3 * 1.0e-6).sqrt() / (electron_temperature_kev * 1.0e3))
}

/// Coulomb logarithm of a fast ion with a thermal ion species, NRL Plasma
/// Formulary, counter-streaming ions:
/// `43 − ln[Z Z′ (μ+μ′)/(μ μ′ β²) · √(n_e[cm⁻³]/T_e[eV])]`.
pub fn coulomb_logarithm_fast_ion(
    electron_density_per_m3: f64,
    electron_temperature_kev: f64,
    fast: &Species,
    speed_m_per_s: f64,
    field: &Species,
) -> f64 {
    let mu = fast.mass_kg / PROTON_MASS_KG;
    let mu_field = field.mass_kg / PROTON_MASS_KG;
    let beta = speed_m_per_s / SPEED_OF_LIGHT_M_PER_S;
    43.0 - math::log(fast.charge * field.charge * (mu + mu_field) / (mu * mu_field * beta * beta)
        * (electron_density_per_m3 * 1.0e-6 / (electron_temperature_kev * 1.0e3)).sqrt())
}

/// The relativistic factor on the energy flow between ions and electrons,
/// `R = (1 + 2t + 2t²) √(π t³/2) / ∫₀^∞ u² exp[−(√(1+u²) − 1)/t] du`,
/// `t = T_e / m_e c²`. The integral is taken to where its weight is e⁻⁴⁵.
pub fn relativistic_exchange_factor(electron_temperature_kev: f64) -> f64 {
    let t = electron_temperature_kev / ELECTRON_REST_KEV;
    let top = ((1.0 + 45.0 * t) * (1.0 + 45.0 * t) - 1.0).sqrt();
    let integral = integrate(0.0, top, 64, |u| u * u * math::exp(-((1.0 + u * u).sqrt() - 1.0) / t));
    (1.0 + 2.0 * t + 2.0 * t * t) * (PI * t * t * t / 2.0).sqrt() / integral
}

/// One ion species: its charge number, its mass and its density.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Species {
    pub charge: f64,
    pub mass_kg: f64,
    pub density_per_m3: f64,
}

/// The thermal plasma a fast ion slows in. The electron channel's Coulomb
/// logarithm and relativistic factor depend on the plasma alone and are
/// worked out once.
#[derive(Clone, Debug)]
pub struct Background {
    pub electron_density_per_m3: f64,
    pub electron_temperature_kev: f64,
    pub ion_temperature_kev: f64,
    electron_logarithm: f64,
    relativistic_factor: f64,
}

impl Background {
    pub fn new(electron_density_per_m3: f64, electron_temperature_kev: f64, ion_temperature_kev: f64) -> Self {
        Self {
            electron_density_per_m3,
            electron_temperature_kev,
            ion_temperature_kev,
            electron_logarithm: coulomb_logarithm_electron(electron_density_per_m3, electron_temperature_kev),
            relativistic_factor: relativistic_exchange_factor(electron_temperature_kev),
        }
    }

    /// `Z² e⁴ / (4π ε0² v)` in keV m³ kg / s: the part of the loss rate common to every species.
    fn common(fast: &Species, speed_m_per_s: f64) -> f64 {
        let e2 = ELEMENTARY_CHARGE_C * ELEMENTARY_CHARGE_C;
        fast.charge * fast.charge * e2 * e2
            / (4.0 * PI * VACUUM_PERMITTIVITY_F_PER_M * VACUUM_PERMITTIVITY_F_PER_M * speed_m_per_s)
            / JOULE_PER_KEV
    }

    /// Energy a fast ion of `energy_kev` gives to the electrons, keV per second.
    pub fn loss_to_electrons_kev_per_s(&self, fast: &Species, energy_kev: f64) -> f64 {
        let speed = (2.0 * energy_kev * JOULE_PER_KEV / fast.mass_kg).sqrt();
        let x = ELECTRON_MASS_KG * speed * speed / (2.0 * self.electron_temperature_kev * JOULE_PER_KEV);
        let (psi, derivative) = psi_and_derivative(x);
        Self::common(fast, speed) * self.electron_density_per_m3 * self.electron_logarithm / ELECTRON_MASS_KG
            * (psi - ELECTRON_MASS_KG / fast.mass_kg * derivative)
            * self.relativistic_factor
    }

    /// Energy a fast ion of `energy_kev` gives to one thermal ion species, keV per second.
    pub fn loss_to_ions_kev_per_s(&self, fast: &Species, energy_kev: f64, field: &Species) -> f64 {
        let speed = (2.0 * energy_kev * JOULE_PER_KEV / fast.mass_kg).sqrt();
        let x = field.mass_kg * speed * speed / (2.0 * self.ion_temperature_kev * JOULE_PER_KEV);
        let (psi, derivative) = psi_and_derivative(x);
        let logarithm = coulomb_logarithm_fast_ion(
            self.electron_density_per_m3,
            self.electron_temperature_kev,
            fast,
            speed,
            field,
        );
        Self::common(fast, speed) * field.charge * field.charge * field.density_per_m3 * logarithm / field.mass_kg
            * (psi - field.mass_kg / fast.mass_kg * derivative)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Tentori and Belloni's published reactivity fit, Table 2, read 2026-10-09
    /// ([`REFERENCE_RETRIEVED`]) from <https://doi.org/10.1088/1741-4326/acda4b>
    /// (Nucl. Fusion 63, 086001, CC BY 4.0): `P1 θ √(ξ/(μc² T³)) e^(−3ξ)` in
    /// m³/s, stated range 70 to 500 keV. It is a fit to a calculation, not a
    /// measurement.
    fn published_fit_m3_per_s(t: f64) -> f64 {
        const P: [f64; 7] = [9.7827e-14, -5.1610e-2, 1.3240e-1, 3.8446e-4, 1.2499e-3, -5.6715e-6, 1.1615e-6];
        let theta = t / (1.0 - t * (P[1] + t * (P[3] + t * P[5])) / (1.0 + t * (P[2] + t * (P[4] + t * P[6]))));
        let xi = (GAMOW_ENERGY_KEV / (4.0 * theta)).cbrt();
        P[0] * theta * (xi / (REDUCED_MASS_KEV * t * t * t)).sqrt() * (-3.0 * xi).exp()
    }

    /// The integral of the fit by mpmath at 30 digits, cm³/s:
    /// `evidence/research/fusion_pb11/reaction_rates/analysis/reference_table.csv`,
    /// column `sv_ref_cm3_s`, as its findings print it (five figures).
    const REFERENCE_TABLE: [(f64, f64); 13] = [
        (10.0, 4.4443e-22),
        (20.0, 7.8133e-20),
        (30.0, 6.5067e-19),
        (50.0, 5.8542e-18),
        (70.0, 2.1681e-17),
        (100.0, 7.0352e-17),
        (150.0, 1.8609e-16),
        (200.0, 2.9477e-16),
        (300.0, 4.3741e-16),
        (400.0, 5.1320e-16),
        (500.0, 5.6045e-16),
        (700.0, 6.2428e-16),
        (1000.0, 6.7905e-16),
    ];

    #[test]
    fn tentori_belloni_reactivity_fit_from_70_to_500_kev() {
        println!("Tentori and Belloni read {REFERENCE_RETRIEVED} from {REFERENCE_URL}");
        let mut worst: f64 = 0.0;
        for t in [70.0, 80.0, 90.0, 100.0, 125.0, 150.0, 200.0, 250.0, 300.0, 350.0, 400.0, 450.0, 500.0] {
            let solver = reactivity_pb11_m3_per_s(t).expect("in range");
            let off = (published_fit_m3_per_s(t) - solver) / solver;
            println!("T {t} keV: {solver:e} m3/s, published fit {off:+.4}");
            worst = worst.max(off.abs());
            assert!(off.abs() <= REFERENCE_TOLERANCE, "{t}: {off}");
        }
        println!("worst {worst}");
    }

    #[test]
    fn the_reactivity_is_the_integral_the_research_tabulated() {
        for (t, table_cm3_per_s) in REFERENCE_TABLE {
            let off = reactivity_integral_m3_per_s(t) * 1.0e6 / table_cm3_per_s - 1.0;
            assert!(off.abs() < 1.0e-4, "{t}: {off}");
        }
    }

    #[test]
    fn the_cross_section_has_its_printed_values() {
        // Peak 1.413 b at 603 keV; 0.012779977 b at the top of the fit (WarpX stores 0.01277998).
        let peak = cross_section_barn(603.0).expect("in range");
        assert!((peak - 1.413).abs() < 5.0e-4, "{peak}");
        let top = cross_section_barn(9760.0).expect("in range");
        assert!((top / 0.012779977 - 1.0).abs() < 1.0e-8, "{top}");
        let low = cross_section_barn(10.0).expect("in range");
        assert!((low / 4.585485519e-17 - 1.0).abs() < 1.0e-9, "{low}");
        // The two steps at the joins of the fit, in the S-factor.
        assert!((s_factor_mev_b(400.0) - 345.53).abs() < 0.01);
        assert!((s_factor_mev_b(400.000001) - 346.0).abs() < 0.01);
        assert!((s_factor_mev_b(668.0) - 254.18).abs() < 0.01);
        assert!((s_factor_mev_b(668.000001) - 255.86).abs() < 0.01);
    }

    #[test]
    fn outside_the_fit_nothing_is_returned() {
        assert_eq!(cross_section_barn(0.0), None);
        assert_eq!(cross_section_barn(-1.0), None);
        assert_eq!(cross_section_barn(9760.1), None);
        assert_eq!(cross_section_barn(f64::NAN), None);
        assert_eq!(reactivity_pb11_m3_per_s(0.99), None);
        assert_eq!(reactivity_pb11_m3_per_s(f64::from_bits(1.0_f64.to_bits() - 1)), None);
        assert!(reactivity_pb11_m3_per_s(1.0).is_some());
        assert!(reactivity_pb11_m3_per_s(9.99).is_some());
        assert_eq!(reactivity_pb11_m3_per_s(f64::from_bits(1000.0_f64.to_bits() + 1)), None);
        assert_eq!(reactivity_pb11_m3_per_s(f64::NAN), None);
        assert!(reactivity_pb11_m3_per_s(10.0).is_some());
        assert!(reactivity_pb11_m3_per_s(500.0).is_some());
        assert!(reactivity_pb11_m3_per_s(500.1).is_some());
        assert!(reactivity_pb11_m3_per_s(1000.0).is_some());
    }

    /// Simpson's rule, `steps` intervals (made even) from `low` to `high`.
    fn simpson(low: f64, high: f64, steps: usize, f: &impl Fn(f64) -> f64) -> f64 {
        let steps = steps + steps % 2;
        let h = (high - low) / steps as f64;
        // The fit steps at its joins: each piece starts on its own branch.
        let mut sum = f(f64::from_bits(low.to_bits() + 1)) + f(high);
        for i in 1..steps {
            sum += if i % 2 == 1 { 4.0 } else { 2.0 } * f(low + i as f64 * h);
        }
        sum * h / 3.0
    }

    /// The Maxwellian integral by a second rule written apart from
    /// [`reactivity_integral_m3_per_s`]: Simpson's rule, not Gauss-Legendre,
    /// on its own breaks (the 148 keV term taken on 0.01 keV steps from 128
    /// to 168 keV), with the cross-section of the published formula and the
    /// constants of the paper. `refine` divides every step.
    fn simpson_reactivity_m3_per_s(t: f64, refine: usize) -> f64 {
        let weight = |e: f64| if e > 1.0e-3 { cross_section_barn(e).unwrap_or(0.0) * e * (-e / t).exp() } else { 0.0 };
        let pieces: [(f64, f64, f64); 7] = [
            (0.0, 128.0, 0.25),
            (128.0, 168.0, 0.01),
            (168.0, 400.0, 0.25),
            (400.0, 668.0, 0.5),
            (668.0, 1500.0, 1.0),
            (1500.0, 4000.0, 2.0),
            (4000.0, CROSS_SECTION_ENERGY_MAX_KEV, 5.0),
        ];
        let mut integral = 0.0;
        for (low, high, step) in pieces {
            integral += simpson(low, high, ((high - low) / step).round() as usize * refine, &weight);
        }
        (8.0 / (PI * REDUCED_MASS_KEV)).sqrt() * SPEED_OF_LIGHT_M_PER_S * integral * 1.0e-28 / (t * t.sqrt())
    }

    /// The reactivity is confirmed above 500 keV and below 10 keV by a
    /// second, independent quadrature: every 1 keV from 1 to 9 keV, then
    /// from 10 to 1000 keV every 10 keV, the two rules agree
    /// within 1e-10 (6.7e-12 at worst when written), and Simpson's rule with
    /// every step halved moves by less than 1e-9, so the agreement is not
    /// that of two rules equally wrong.
    /// At 700 and 1000 keV both meet the research's 30-digit integral
    /// (`REFERENCE_TABLE`) to its five printed figures.
    #[test]
    fn a_second_quadrature_gives_the_same_reactivity_to_1000_kev() {
        let mut worst: f64 = 0.0;
        let mut worst_halving: f64 = 0.0;
        for t in (1..=9).map(f64::from).chain((1..=100).map(|step| 10.0 * f64::from(step))) {
            let gauss = reactivity_pb11_m3_per_s(t).expect("declared");
            let coarse = simpson_reactivity_m3_per_s(t, 1);
            let fine = simpson_reactivity_m3_per_s(t, 2);
            assert!(gauss.is_finite() && coarse.is_finite() && fine.is_finite(), "{t}");
            worst = worst.max((gauss / fine - 1.0).abs());
            worst_halving = worst_halving.max((coarse / fine - 1.0).abs());
        }
        println!("Gauss against Simpson, 1 to 1000 keV: worst {worst:e}; Simpson halved: {worst_halving:e}");
        assert!(worst < 1.0e-10, "{worst}");
        assert!(worst_halving < 1.0e-9, "{worst_halving}");
        for (t, table_cm3_per_s) in [(700.0, 6.2428e-16), (1000.0, 6.7905e-16)] {
            let off = simpson_reactivity_m3_per_s(t, 2) * 1.0e6 / table_cm3_per_s - 1.0;
            assert!(off.abs() < 1.0e-4, "{t}: {off}");
        }
    }

    /// The reason for each end of the declared reactivity, the integral's
    /// own. At the bottom, 1 keV, the Gamow window is 12.9 to 22.7 keV, and
    /// the served panels, 1 keV wide, hold the integral within 1e-13 of
    /// panels a hundredth as wide. At the top, 1000 keV, the part of the rate
    /// above the fit's top, 9760 keV, which the integral leaves out, is
    /// estimated with the cross-section continued as `σ(9760 keV) · 9760 / E`:
    /// under 2e-5 at 1000 keV, about 3.6e-4 at 1500 keV. The window lies
    /// inside the fit's measured points from about 3 keV to between 1000 and
    /// 1100 keV.
    #[test]
    fn the_ends_of_the_reactivity_are_where_its_integral_holds() {
        let inside = |t: f64| {
            let (centre, width) = gamow_window_kev(t);
            centre - 0.5 * width >= MEASURED_POINTS_MIN_KEV && centre + 0.5 * width <= MEASURED_POINTS_MAX_KEV
        };
        assert!(inside(3.0) && inside(10.0));
        assert!(!inside(REACTIVITY_TEMPERATURE_MIN_KEV) && !inside(2.0));
        assert!(inside(REACTIVITY_TEMPERATURE_MAX_KEV));
        assert!(!inside(1100.0));
        let (centre, width) = gamow_window_kev(REACTIVITY_TEMPERATURE_MIN_KEV);
        assert!((centre - 0.5 * width - 12.9).abs() < 0.05 && (centre + 0.5 * width - 22.7).abs() < 0.05, "{centre} {width}");
        for t in [REACTIVITY_TEMPERATURE_MIN_KEV, 2.0, 5.0] {
            let weight = |e: f64| cross_section_or_zero(e) * e * (-e / t).exp();
            let fine = integrate(0.0, 400.0, 40_000, weight)
                + integrate(400.0, 668.0, 6_700, weight)
                + integrate(668.0, 1500.0, 20_800, weight)
                + integrate(1500.0, CROSS_SECTION_ENERGY_MAX_KEV, 41_300, weight);
            let fine = (8.0 / (PI * REDUCED_MASS_KEV)).sqrt() * SPEED_OF_LIGHT_M_PER_S * fine * 1.0e-28 / (t * t.sqrt());
            let served = reactivity_pb11_m3_per_s(t).expect("declared");
            assert!((served / fine - 1.0).abs() < 1.0e-13, "{t}: {}", served / fine - 1.0);
        }
        let (centre, width) = gamow_window_kev(10.0);
        assert!((centre - 82.7).abs() < 0.1 && (width - 66.4).abs() < 0.2, "{centre} {width}");
        let (centre, width) = gamow_window_kev(1000.0);
        assert!((centre + 0.5 * width - 3321.0).abs() < 2.0, "{centre} {width}");
        let top = cross_section_barn(CROSS_SECTION_ENERGY_MAX_KEV).expect("in range");
        let t = REACTIVITY_TEMPERATURE_MAX_KEV;
        let tail = integrate(CROSS_SECTION_ENERGY_MAX_KEV, 60_000.0, 2000, |e| top * CROSS_SECTION_ENERGY_MAX_KEV / e * e * (-e / t).exp());
        let whole = integrate(0.0, 400.0, 400, |e| cross_section_or_zero(e) * e * (-e / t).exp())
            + integrate(400.0, CROSS_SECTION_ENERGY_MAX_KEV, 2000, |e| cross_section_or_zero(e) * e * (-e / t).exp());
        println!("left out above the fit at {t} keV: {:e} of the rate", tail / whole);
        assert!(tail / whole < 2.0e-5 && tail / whole > 1.6e-5, "{}", tail / whole);
        let t = 1500.0;
        let tail = integrate(CROSS_SECTION_ENERGY_MAX_KEV, 60_000.0, 2000, |e| top * CROSS_SECTION_ENERGY_MAX_KEV / e * e * (-e / t).exp());
        let whole = integrate(0.0, 400.0, 400, |e| cross_section_or_zero(e) * e * (-e / t).exp())
            + integrate(400.0, CROSS_SECTION_ENERGY_MAX_KEV, 2000, |e| cross_section_or_zero(e) * e * (-e / t).exp());
        assert!(tail / whole > 3.0e-4 && tail / whole < 4.5e-4, "{}", tail / whole);
    }

    #[test]
    fn psi_is_the_error_function_form_on_both_branches() {
        // ψ(1) = erf(1) − (2/√π) e⁻¹ and ψ(4) = erf(2) − (4/√π) e⁻⁴, with erf from tables.
        let (one, d_one) = psi_and_derivative(1.0);
        assert!((one - (0.842_700_792_949_714_9 - d_one)).abs() < 1.0e-15, "{one}");
        let (four, d_four) = psi_and_derivative(4.0);
        assert!((four - (0.995_322_265_018_952_7 - d_four)).abs() < 1.0e-15, "{four}");
        // The two branches meet.
        let below = psi_and_derivative(2.5 - 1.0e-9).0;
        let above = psi_and_derivative(2.5 + 1.0e-9).0;
        assert!((below - above).abs() < 1.0e-9);
        // Small argument: ψ → (4/(3√π)) x^1.5, with no cancellation.
        let (small, _) = psi_and_derivative(1.0e-8);
        assert!((small / (4.0 / (3.0 * PI.sqrt()) * 1.0e-12) - 1.0).abs() < 1.0e-7);
    }

    #[test]
    fn a_cold_target_gives_the_bare_cross_section() {
        // At 1 eV the boron barely moves: ⟨σv⟩ is σ(E) v off the narrow term.
        let energy = 120.0;
        let speed = (2.0 * energy * JOULE_PER_KEV / PROTON_MASS_KG).sqrt();
        let bare = cross_section_barn(reduced_mass_over_proton_mass() * energy).expect("in range") * 1.0e-28 * speed;
        let averaged = beam_target_rate_m3_per_s(energy, 1.0e-3, 0.05);
        assert!((averaged / bare - 1.0).abs() < 2.0e-3, "{}", averaged / bare);
        // Warm boron spreads the narrow term: below it the rate rises, on it the rate falls.
        let on = 148.0 / reduced_mass_over_proton_mass();
        assert!(beam_target_rate_m3_per_s(on, 5.0, 1.0) < beam_target_rate_m3_per_s(on, 0.1, 0.25));
        assert!(beam_target_rate_m3_per_s(150.0, 5.0, 1.0) > beam_target_rate_m3_per_s(150.0, 0.1, 0.25));
    }

    #[test]
    fn the_relativistic_factor_has_its_published_values() {
        // 1.008 at t = 0.05, 1.021 at 0.1, 1.092 at 0.3, 1.182 at 0.5 (power_balance findings, section 2.5).
        for (t, value) in [(0.05, 1.008), (0.1, 1.021), (0.3, 1.092), (0.5, 1.182)] {
            let factor = relativistic_exchange_factor(t * ELECTRON_REST_KEV);
            assert!((factor - value).abs() < 6.0e-4, "{t}: {factor}");
        }
    }

    #[test]
    fn a_fast_proton_slows_as_the_classical_formula_says() {
        // 160 keV proton in hydrogen at 5.4 keV, 2.2e19 m^-3: the Formulary's
        // slowing-down time on electrons is 0.656 s, so 2E/τ is 488 keV/s.
        let plasma = Background::new(2.2e19, 5.4, 5.4);
        let proton = Species { charge: 1.0, mass_kg: PROTON_MASS_KG, density_per_m3: 0.0 };
        let field = Species { charge: 1.0, mass_kg: PROTON_MASS_KG, density_per_m3: 2.2e19 };
        let electrons = plasma.loss_to_electrons_kev_per_s(&proton, 160.0);
        assert!((electrons / 488.0 - 1.0).abs() < 0.08, "{electrons}");
        let ions = plasma.loss_to_ions_kev_per_s(&proton, 160.0, &field);
        assert!(ions > 0.3 * electrons && ions < 0.7 * electrons, "{ions}");
        // Slower than the thermal ions, a test ion gains energy from them.
        assert!(plasma.loss_to_ions_kev_per_s(&proton, 1.0, &field) < 0.0);
    }
}
