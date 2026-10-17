//! The deuterium-tritium and deuterium-deuterium fusion cross-sections and
//! Maxwellian reactivities of Bosch and Hale, Nucl. Fusion 32 (1992) 611.
//!
//! This file is not a solver. `tokamak_dt_beam_target` takes its
//! cross-sections and its reactivities from here.
//!
//! Three reactions: T(d,n)⁴He, 17.589 MeV; D(d,n)³He, 3.269 MeV; D(d,p)T,
//! 4.033 MeV. The cross-section is the paper's S-factor fit (its equations 8
//! and 9, Table IV, and Table VI for D-T above 550 keV):
//!
//! ```text
//! σ(E) = S(E) / (E · exp(B_G / √E))            E centre of mass, keV; σ in millibarn
//! S(E) = (A1 + E (A2 + E (A3 + E (A4 + E A5)))) / (1 + E (B1 + E (B2 + E (B3 + E B4))))
//! ```
//!
//! The fits are stated from 0.5 keV to 4700 keV (D-T, in two pieces joined at
//! 550 keV), to 4900 keV (the neutron branch of D-D) and to 5000 keV (its
//! proton branch). Outside its range [`cross_section_barn`] returns nothing.
//! The reactivity is the paper's own fit (its equations 12 to 14, Table VII),
//! stated from 0.2 to 100 keV:
//!
//! ```text
//! θ    = T / (1 − T (C2 + T (C4 + T C6)) / (1 + T (C3 + T (C5 + T C7))))
//! ξ    = (B_G² / (4 θ))^(1/3)
//! ⟨σv⟩ = C1 θ √(ξ / (m_r c² T³)) e^(−3ξ)        cm³/s, T in keV
//! ```
//!
//! The paper is not open. The cross-section coefficients were read on
//! 2026-10-09 from two open codes that carry its tables, ASCOT5
//! (`src/boschhale.c`, GPL-3.0) and FIDASIM (`tables/atomic_tables.f90`),
//! which agree on every coefficient; the reactivity coefficients from UKAEA
//! PROCESS and cfspopcon, and they are the ones of the card
//! `tokamak_jet_dte1`, digit for digit. The copies and their checksums are in
//! `evidence/research/tokamak_dt_beam_target/raw/cross_section/`. The energy
//! and temperature ranges are as those codes state them, not confirmed
//! against the paper.
//!
//! Two checks, both in the tests here. The Maxwellian integral of each
//! cross-section fit reproduces the paper's reactivity fit from 0.2 to
//! 100 keV within 1.3 % (D-T), 2.0 % (D-D neutron branch) and 2.5 % (D-D
//! proton branch); from 1 keV up, within 0.8 %, 1.1 % and 2.0 %. Against the
//! ENDF/B-VIII.0 evaluation of the same reactions the cross-section fits lie
//! within 1.5 % (D-T), 1.7 % and 2.0 % (D-D) from 5 to 150 keV in the centre
//! of mass, the energies a neutral beam reaches.

use genesis_kernel::math;
use std::f64::consts::PI;

use crate::pb11_reaction::{ATOMIC_MASS_KG, SPEED_OF_LIGHT_M_PER_S, integrate};

/// Mass of the deuteron, kg (CODATA 2018).
pub const DEUTERON_MASS_KG: f64 = 3.3435837724e-27;
/// Mass of the triton, kg (CODATA 2018).
pub const TRITON_MASS_KG: f64 = 5.0073567446e-27;
/// Mass of a carbon-12 nucleus, kg: twelve atomic mass units less six electrons.
pub const CARBON12_MASS_KG: f64 = 12.0 * ATOMIC_MASS_KG - 6.0 * crate::pb11_reaction::ELECTRON_MASS_KG;

/// Lowest centre-of-mass energy of every cross-section fit, keV.
pub const CROSS_SECTION_ENERGY_MIN_KEV: f64 = 0.5;
/// Lowest temperature of every reactivity fit, keV.
pub const REACTIVITY_TEMPERATURE_MIN_KEV: f64 = 0.2;
/// Highest temperature of every reactivity fit, keV.
pub const REACTIVITY_TEMPERATURE_MAX_KEV: f64 = 100.0;

/// The published fit and where its coefficients were read.
pub const REFERENCE_NAME: &str = "Bosch and Hale (1992), Improved formulas for fusion cross-sections and thermal reactivities, Nucl. Fusion 32, 611: the S-factor fits of Tables IV and VI and the reactivity fits of Table VII, as carried by the open codes ASCOT5 and FIDASIM (cross-sections) and UKAEA PROCESS and cfspopcon (reactivities)";
pub const REFERENCE_URL: &str = "https://github.com/ascot4fusion/ascot5/blob/main/src/boschhale.c";
/// Date the reference was read.
pub const REFERENCE_RETRIEVED: &str = "2026-10-09";
/// The Maxwellian integral of each cross-section fit agrees with the paper's
/// reactivity fit to this fraction from 0.2 to 100 keV, in the order of [`Reaction`].
pub const REFERENCE_TOLERANCE: [f64; 3] = [0.013, 0.020, 0.025];

/// One of the three reactions.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Reaction {
    /// T(d,n)⁴He.
    DeuteriumTritium,
    /// D(d,n)³He.
    DeuteriumDeuteriumNeutron,
    /// D(d,p)T.
    DeuteriumDeuteriumProton,
}

/// The S-factor fit of one reaction over one range of energy.
struct SFactor {
    a: [f64; 5],
    b: [f64; 4],
}

impl SFactor {
    fn at(&self, e: f64) -> f64 {
        let [a1, a2, a3, a4, a5] = self.a;
        let [b1, b2, b3, b4] = self.b;
        (a1 + e * (a2 + e * (a3 + e * (a4 + e * a5)))) / (1.0 + e * (b1 + e * (b2 + e * (b3 + e * b4))))
    }
}

const DT_LOW: SFactor = SFactor { a: [6.927e4, 7.454e8, 2.050e6, 5.2002e4, 0.0], b: [6.38e1, -9.95e-1, 6.981e-5, 1.728e-4] };
const DT_HIGH: SFactor = SFactor { a: [-1.4714e6, 0.0, 0.0, 0.0, 0.0], b: [-8.4127e-3, 4.7983e-6, -1.0748e-9, 8.5184e-14] };
const DD_NEUTRON: SFactor = SFactor { a: [5.3701e4, 3.3027e2, -1.2706e-1, 2.9327e-5, -2.5151e-9], b: [0.0; 4] };
const DD_PROTON: SFactor = SFactor { a: [5.5576e4, 2.1054e2, -3.2638e-2, 1.4987e-6, 1.8181e-10], b: [0.0; 4] };
/// The two D-T fits join at this centre-of-mass energy, keV.
pub const DT_JOIN_ENERGY_KEV: f64 = 550.0;

impl Reaction {
    /// The Gamow constant `B_G`, √keV.
    pub fn gamow_constant(self) -> f64 {
        match self {
            Self::DeuteriumTritium => 34.3827,
            Self::DeuteriumDeuteriumNeutron | Self::DeuteriumDeuteriumProton => 31.3970,
        }
    }

    /// The reduced mass energy `m_r c²` of the paper's reactivity fit, keV.
    pub fn reduced_mass_kev(self) -> f64 {
        match self {
            Self::DeuteriumTritium => 1_124_656.0,
            Self::DeuteriumDeuteriumNeutron | Self::DeuteriumDeuteriumProton => 937_814.0,
        }
    }

    /// Highest centre-of-mass energy of the cross-section fit, keV.
    pub fn cross_section_energy_max_kev(self) -> f64 {
        match self {
            Self::DeuteriumTritium => 4700.0,
            Self::DeuteriumDeuteriumNeutron => 4900.0,
            Self::DeuteriumDeuteriumProton => 5000.0,
        }
    }

    /// Energy of one reaction, keV.
    pub fn energy_kev(self) -> f64 {
        match self {
            Self::DeuteriumTritium => 17_589.0,
            Self::DeuteriumDeuteriumNeutron => 3_269.0,
            Self::DeuteriumDeuteriumProton => 4_033.0,
        }
    }

    /// The seven coefficients of the reactivity fit.
    fn reactivity_coefficients(self) -> [f64; 7] {
        match self {
            Self::DeuteriumTritium => [1.17302e-9, 1.51361e-2, 7.51886e-2, 4.60643e-3, 1.35000e-2, -1.06750e-4, 1.36600e-5],
            Self::DeuteriumDeuteriumNeutron => [5.43360e-12, 5.85778e-3, 7.68222e-3, 0.0, -2.96400e-6, 0.0, 0.0],
            Self::DeuteriumDeuteriumProton => [5.65718e-12, 3.41267e-3, 1.99167e-3, 0.0, 1.05060e-5, 0.0, 0.0],
        }
    }
}

/// The cross-section in barns at a centre-of-mass energy in keV, or `None`
/// outside [`CROSS_SECTION_ENERGY_MIN_KEV`]`..=`the reaction's highest energy.
pub fn cross_section_barn(reaction: Reaction, energy_kev: f64) -> Option<f64> {
    if !(energy_kev >= CROSS_SECTION_ENERGY_MIN_KEV && energy_kev <= reaction.cross_section_energy_max_kev()) {
        return None;
    }
    let s = match reaction {
        Reaction::DeuteriumTritium if energy_kev <= DT_JOIN_ENERGY_KEV => DT_LOW.at(energy_kev),
        Reaction::DeuteriumTritium => DT_HIGH.at(energy_kev),
        Reaction::DeuteriumDeuteriumNeutron => DD_NEUTRON.at(energy_kev),
        Reaction::DeuteriumDeuteriumProton => DD_PROTON.at(energy_kev),
    };
    Some(s / (energy_kev * math::exp(reaction.gamow_constant() / energy_kev.sqrt())) * 1.0e-3)
}

/// The cross-section inside an integral: zero where the fit is not defined.
/// Below 0.5 keV the D-T cross-section is under 1e-21 barn; no integral of
/// the solver that uses this reaches the top of a fit.
pub fn cross_section_or_zero(reaction: Reaction, energy_kev: f64) -> f64 {
    cross_section_barn(reaction, energy_kev).unwrap_or(0.0)
}

/// The Maxwellian reactivity in m³/s at an ion temperature in keV, or `None`
/// outside [`REACTIVITY_TEMPERATURE_MIN_KEV`]`..=`[`REACTIVITY_TEMPERATURE_MAX_KEV`].
pub fn reactivity_m3_per_s(reaction: Reaction, temperature_kev: f64) -> Option<f64> {
    if !(REACTIVITY_TEMPERATURE_MIN_KEV..=REACTIVITY_TEMPERATURE_MAX_KEV).contains(&temperature_kev) {
        return None;
    }
    let t = temperature_kev;
    let c = reaction.reactivity_coefficients();
    let theta = t / (1.0 - t * (c[1] + t * (c[3] + t * c[5])) / (1.0 + t * (c[2] + t * (c[4] + t * c[6]))));
    let gamow = reaction.gamow_constant();
    let xi = math::cbrt(gamow * gamow / (4.0 * theta));
    Some(c[0] * theta * (xi / (reaction.reduced_mass_kev() * t * t * t)).sqrt() * math::exp(-3.0 * xi) * 1.0e-6)
}

/// The Maxwellian integral of the cross-section fit, m³/s:
/// `√(8/(π m_r c²)) · c · T^(−3/2) ∫ σ E e^(−E/T) dE`, taken from 0.5 keV to
/// sixty temperatures or 30 keV, whichever is more, in panels a twentieth of
/// a temperature wide. For the tests and the outside recomputation.
pub fn reactivity_integral_m3_per_s(reaction: Reaction, temperature_kev: f64) -> f64 {
    let t = temperature_kev;
    let top = (60.0 * t).max(30.0).min(reaction.cross_section_energy_max_kev());
    let weight = |e: f64| cross_section_or_zero(reaction, e) * e * math::exp(-e / t);
    let join = DT_JOIN_ENERGY_KEV.min(top);
    let panels = |low: f64, high: f64| (((high - low) / (0.05 * t)).ceil() as usize).max(1);
    let integral = integrate(CROSS_SECTION_ENERGY_MIN_KEV, join, panels(CROSS_SECTION_ENERGY_MIN_KEV, join), weight)
        + if top > join { integrate(join, top, panels(join, top), weight) } else { 0.0 };
    (8.0 / (PI * reaction.reduced_mass_kev())).sqrt() * SPEED_OF_LIGHT_M_PER_S * integral * 1.0e-28 / (t * t.sqrt())
}

#[cfg(test)]
mod tests {
    use super::*;
    use Reaction::{DeuteriumDeuteriumNeutron as Ddn, DeuteriumDeuteriumProton as Ddp, DeuteriumTritium as Dt};

    /// Rule 7. Bosch and Hale's reactivity fit is the published reference;
    /// the Maxwellian integral of their cross-section fit must reproduce it.
    /// Coefficients read 2026-10-09 ([`REFERENCE_RETRIEVED`]) from
    /// <https://github.com/ascot4fusion/ascot5/blob/main/src/boschhale.c>.
    /// The largest differences found by quadrature of 1e-11 relative accuracy
    /// (`evidence/research/tokamak_dt_beam_target/analysis/out/01_cross_section_checks.json`):
    /// D-T −1.26 % at 0.2 keV, D-D neutron −1.93 % at 0.2 keV, D-D proton −2.50 % at 0.2 keV.
    #[test]
    fn the_maxwellian_integral_of_each_cross_section_is_the_published_reactivity() {
        println!("Bosch and Hale read {REFERENCE_RETRIEVED} from {REFERENCE_URL}");
        for (index, reaction) in [Dt, Ddn, Ddp].into_iter().enumerate() {
            let mut worst: f64 = 0.0;
            for t in [0.2, 0.3, 0.5, 1.0, 2.0, 3.0, 5.0, 7.0, 10.0, 15.0, 20.0, 30.0, 40.0, 50.0, 70.0, 100.0] {
                let fit = reactivity_m3_per_s(reaction, t).expect("in range");
                let off = reactivity_integral_m3_per_s(reaction, t) / fit - 1.0;
                println!("{reaction:?} T {t} keV: fit {fit:e} m3/s, integral {off:+.5}");
                worst = worst.max(off.abs());
                assert!(off.abs() <= REFERENCE_TOLERANCE[index], "{reaction:?} {t}: {off}");
            }
            println!("{reaction:?} worst {worst}");
        }
    }

    /// The integral as computed outside the engine by adaptive quadrature
    /// (`analysis/out/01_cross_section_checks.json`, `integral_m3_per_s`).
    #[test]
    fn the_integral_is_the_one_computed_outside() {
        for (reaction, t, outside) in [
            (Dt, 0.2, 1.238_222_529_291_138_2e-32),
            (Dt, 10.0, 1.141_806_002_652_330_1e-22),
            (Dt, 100.0, 8.453_835_548_755_314e-22),
            (Ddn, 20.0, 2.631_097_544_416_112e-24),
            (Ddp, 30.0, 4.821_280_966_957_526_4e-24),
        ] {
            let off = reactivity_integral_m3_per_s(reaction, t) / outside - 1.0;
            assert!(off.abs() < 1.0e-9, "{reaction:?} {t}: {off}");
        }
    }

    #[test]
    fn the_fits_have_their_known_values() {
        // The card tokamak_jet_dte1 holds the same D-T fit: its values at 1, 10, 20, 50 and 100 keV.
        for (t, value) in [(1.0, 6.8569e-27), (10.0, 1.13617e-22), (20.0, 4.33020e-22), (50.0, 8.64908e-22), (100.0, 8.44766e-22)] {
            let fit = reactivity_m3_per_s(Dt, t).expect("in range");
            assert!((fit / value - 1.0).abs() < 2.0e-5, "{t}: {fit}");
        }
        // The D-T cross-section peaks at 5.06 barn near 64 keV in the centre of mass.
        let peak = cross_section_barn(Dt, 64.0).expect("in range");
        assert!((peak - 5.0635).abs() < 1.0e-3, "{peak}");
        assert!(cross_section_barn(Dt, 60.0).expect("in range") < peak && cross_section_barn(Dt, 70.0).expect("in range") < peak);
        // The two D-T pieces meet at 550 keV within one percent.
        let below = cross_section_barn(Dt, 550.0).expect("in range");
        let above = cross_section_barn(Dt, 550.0 + 1.0e-9).expect("in range");
        assert!((above / below - 1.0).abs() < 0.01, "{below} {above}");
        // The two D-D branches are near equal at low energy and the neutron branch leads above.
        let n = cross_section_barn(Ddn, 100.0).expect("in range");
        let p = cross_section_barn(Ddp, 100.0).expect("in range");
        assert!((n - 0.037012).abs() < 1.0e-5 && (p - 0.033037).abs() < 1.0e-5, "{n} {p}");
    }

    #[test]
    fn outside_a_fit_nothing_is_returned() {
        for reaction in [Dt, Ddn, Ddp] {
            assert_eq!(cross_section_barn(reaction, 0.499), None);
            assert_eq!(cross_section_barn(reaction, 0.0), None);
            assert_eq!(cross_section_barn(reaction, -1.0), None);
            assert_eq!(cross_section_barn(reaction, f64::NAN), None);
            assert!(cross_section_barn(reaction, 0.5).is_some());
            let top = reaction.cross_section_energy_max_kev();
            assert!(cross_section_barn(reaction, top).is_some());
            assert_eq!(cross_section_barn(reaction, top + 0.1), None);
            assert_eq!(cross_section_or_zero(reaction, top + 0.1), 0.0);
            assert_eq!(reactivity_m3_per_s(reaction, 0.199), None);
            assert_eq!(reactivity_m3_per_s(reaction, 100.1), None);
            assert_eq!(reactivity_m3_per_s(reaction, f64::NAN), None);
            assert!(reactivity_m3_per_s(reaction, 0.2).is_some());
            assert!(reactivity_m3_per_s(reaction, 100.0).is_some());
        }
    }
}
