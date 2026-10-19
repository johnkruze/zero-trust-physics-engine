//! What an agent reads of the tokamak_jet_dte1_twin card: its fields and bounds, its step
//! floor and ceiling, its refusals word for word from the card's source, its
//! frame, its worked examples and its record. Stamped from the kit; a child
//! module of `descriptor.rs`, so it uses that file's own builders.

use genesis_kernel::tokamak_jet_dte1_twin;
use serde_json::Value;

use super::{Advisory, Card, Role, finish, obj, pair_examples_recorded, scalar, scalar_value, slots};

const DOMAIN: &str = "energy";
const CARD: &str = "tokamak_jet_dte1";

/// What the card is, in one sentence of the list (its spec sheet replaces it where served).
const SPECIMEN: &str = "JET in a heated pulse of its 1996 to 1997 deuterium-tritium campaign as one model of ten pulses: the thermal plasma, the deuterium and tritium beam ions slowed at the classical rate on the measured electron temperature, the ion-cyclotron tail and the alphas, with the thermal and the beam-target D-T fusion power, fitted on all ten pulses.";

/// The accuracy of the card against the ten pulses, fitted on all ten and
/// with each pulse left out of the fit. Written from
/// `evidence/research/tokamak_jet_dte1/twin/` (`fit_twin.py`, `loo_twin.py`,
/// `gateway_twin.py`).
pub(crate) const RESULT: &str = "Fitted on all ten pulses, every pulse read when the structure was chosen: four constants. Each error is the RMS over the heated window, sampled every 25 ms, as a fraction of the pulse's own peak; with the pulse left out of the fit and the four refitted on the other nine after the slash. Thermal energy: 38285 0.193/0.197, 38287 0.178/0.180, 38415 0.158/0.160, 42762 0.050/0.078, 42794 0.180/0.182, 42997 0.156/0.159, 43002 0.142/0.142, 43134 0.157/0.159, 42982 0.055/0.064, 42976 0.074/0.271; median 0.156/0.160. Fast-ion energy: 0.081/0.085, 0.049/0.051, 0.045/0.047, 0.081/0.078, 0.070/0.071, 0.035/0.032, 0.036/0.042, 0.041/0.045, 0.051/0.056, 0.050/0.065; median 0.050/0.053. Fusion power, the seven pulses with tritium: 42762 0.139/0.240, 42794 0.142/0.153, 42997 0.110/0.116, 43002 0.179/0.184, 43134 0.120/0.127, 42982 0.145/0.164, 42976 0.106/0.390; median 0.139/0.164. Peak fusion power, measured, fitted and left out, MW: 42762 4.49, 4.50, 5.56; 42794 0.34, 0.40, 0.40; 42997 0.45, 0.51, 0.51; 43002 1.31, 0.80, 0.79; 43134 0.08, 0.07, 0.06; 42982 6.07, 5.14, 4.89; 42976 14.39, 14.06, 4.30. Fusion energy over the window, measured, fitted and left out, MJ: 42762 12.67, 13.73, 15.68; 42794 0.86, 1.00, 1.02; 42997 1.73, 1.87, 1.89; 43002 3.66, 2.90, 2.86; 43134 0.33, 0.29, 0.29; 42982 24.78, 21.35, 20.57 (0.862 and 0.830 of the database's); 42976 10.30, 9.27, 4.03 (0.900 and 0.392). Left out, 42976 has no hot-ion pulse to fit its confinement multiplier on and takes the ELMy one. The thermal part at the peak, measured and fitted, MW: 42982 2.18 and 2.96, 42976 9.03 and 10.25, 42762 1.57 and 3.30. Where tritium is three quarters to 93 percent of the fuel, the fusion energy is 1.165, 1.087 and 0.792 of the database's (42794, 42997, 43002). The database's fusion power is TRANSP's from the measured profiles; its comment files put it 10 to 20 percent above the measured neutron rate after 15 s of 42982, whose measured yield is published as 22 MJ, and below it late in 42976, whose measured peak is 16 MW.";

pub(super) fn card() -> Card {
    let specimen = tokamak_jet_dte1_twin::TokamakJetDte1Twin::dte1();
    let gate = specimen.gate_dt_s().expect("card gate");
    let floor = specimen.floor_dt_s().expect("card floor");
    let store = |name: &'static str, of: &str| {
        scalar(
            name,
            Role::Phase,
            "J",
            Some(0.0),
            false,
            None,
            false,
            format!("at least 0 J: {of} at the start; below zero is refused as EnergyOutsideCard. The four stores together are held to the Troyon limit under the first load (BetaAboveTroyon)"),
        )
    };
    let power = |name: &'static str, of: &str| {
        scalar(
            name,
            Role::Load,
            "W",
            Some(0.0),
            false,
            None,
            false,
            format!("at least 0 W: {of}; below zero is refused as HeatingPowerNegative. With the other three powers it sets the confinement time; all four zero is refused as NoHeatingPower. The compared runs had 1.31 to 23.8 MW of the four together"),
        )
    };
    finish(
        DOMAIN,
        CARD,
        SPECIMEN,
        vec![
            scalar(
                "thermal_energy_j",
                Role::Phase,
                "J",
                Some(0.0),
                true,
                None,
                false,
                "above 0 J: the thermal energy of the plasma at the start; zero or less is refused as EnergyOutsideCard, and so is a thermal energy not above the electrons' energy at the first load's electron temperature, where the ions would hold none. Compared at 0.462 to 15.2 MJ; outside it the card computes the same equations. The central ion temperature it leaves is held to 100 keV, the ceiling of the reactivity fit (TemperatureAboveReactivityFit)".into(),
            ),
            store("deuterium_beam_ion_energy_j", "the energy of the deuterium beam ions"),
            store("tritium_beam_ion_energy_j", "the energy of the tritium beam ions"),
            store("tail_ion_energy_j", "the energy of the ion-cyclotron tail and the fusion alphas. The three fast stores together were compared at 5.59 kJ to 4.06 MJ"),
            scalar(
                "elapsed_time_s",
                Role::Phase,
                "s",
                Some(0.0),
                false,
                None,
                false,
                "at least 0 s: the time since the start of the heated window at the start of this request, and t_s of the state it starts from. A new pulse states 0; a run continued from a returned state states that state's t_s. Below zero is refused as NegativeTime. The longest compared window is 5.475 s, pulse 42982; the card has no end in time".into(),
            ),
            power("deuterium_beam_power_w", "the deuterium beam power absorbed, the injected power less shine-through and first-orbit loss"),
            power("tritium_beam_power_w", "the tritium beam power absorbed"),
            scalar(
                "deuterium_beam_energy_kev",
                Role::Load,
                "keV",
                Some(0.0),
                true,
                Some(tokamak_jet_dte1_twin::DEUTERIUM_BEAM_ENERGY_MAX_KEV),
                false,
                "above 0 keV and at most 917.25 keV: the full injection energy of the deuterium beams, whose power is shared 0.77, 0.17 and 0.06 at the full, half and third energy; outside it is refused as BeamEnergyOutsideCrossSection, since above it a deuteron meets a triton above 550 keV in the centre of mass, the top of the cross-section fit. Compared at 78 to 109 keV".into(),
            ),
            scalar(
                "tritium_beam_energy_kev",
                Role::Load,
                "keV",
                Some(0.0),
                true,
                Some(tokamak_jet_dte1_twin::TRITIUM_BEAM_ENERGY_MAX_KEV),
                false,
                "above 0 keV and at most 1373.68 keV: the full injection energy of the tritium beams, shared 0.65, 0.21 and 0.14 at the full, half and third energy; outside it is refused as BeamEnergyOutsideCrossSection. Compared at 150 and 152 keV".into(),
            ),
            power("icrh_power_w", "the coupled ion-cyclotron power, which feeds the tail"),
            power("ohmic_power_w", "the ohmic power, which heats the thermal plasma"),
            scalar(
                "line_density_per_m3",
                Role::Load,
                "1/m^3",
                Some(0.0),
                true,
                None,
                false,
                "above 0 per cubic metre: the line-averaged electron density; zero or less is refused as DensityNotPositive. It is held to the Greenwald density of the segment's current, 1e20 x Ip[MA] / (pi x 0.9382^2) per cubic metre, and refused above it as DensityAboveGreenwald. Compared at 1.19e19 to 8.87e19 per cubic metre; a frame carries FLAG_NEAR_DENSITY_LIMIT above 0.85 of the Greenwald density".into(),
            ),
            scalar(
                "plasma_current_a",
                Role::Load,
                "A",
                Some(0.0),
                true,
                None,
                false,
                "above 0 A: the plasma current, as a size; zero or less is refused as PlasmaCurrentNotPositive. With the toroidal field it sets the edge safety factor, 5 a^2 Bt / (R Ip[MA]) times a shape factor, and a pair under 2 is refused as SafetyFactorBelowLimit. Compared at 1.65 to 4.16 MA".into(),
            ),
            scalar(
                "toroidal_field_t",
                Role::Load,
                "T",
                Some(0.0),
                true,
                None,
                false,
                "above 0 T: the toroidal field at the major radius, as a size; zero or less is refused as ToroidalFieldNotPositive. Compared at 1.80 to 4.04 T".into(),
            ),
            scalar(
                "tritium_fraction",
                Role::Load,
                "1",
                Some(0.0),
                false,
                Some(1.0),
                false,
                "from 0 to 1, both inclusive: tritium atoms per fuel atom; outside it is refused as FuelMixOutsideCard. The thermal fusion follows the deuterium fraction times the tritium fraction; the deuterium beams fuse with the tritons and the tritium beams with the deuterons. Compared at 0 (38285, 38287, 38415), 0.034 (43134), 0.451 and 0.503 (42982, 42762), 0.501 (42976), 0.754 and 0.757 (42997, 42794) and 0.928 (43002)".into(),
            ),
            scalar(
                "electron_temperature_kev",
                Role::Load,
                "keV",
                Some(0.0),
                true,
                None,
                false,
                "above 0 keV: the central electron temperature, the pulse's measured trace; zero or less is refused as ElectronTemperatureOutsideCard, and so is a temperature so low against the density that a cell's Coulomb logarithm is zero or less. It sets the electrons' share of the thermal energy and the beams' slowing down. Compared at 1.76 to 12.0 keV".into(),
            ),
            scalar(
                "effective_charge",
                Role::Load,
                "1",
                Some(1.0),
                false,
                Some(6.0),
                true,
                "from 1, inclusive, to 6, exclusive: the line-averaged effective charge, the pulse's measured trace, with carbon the one impurity; outside it is refused as EffectiveChargeOutsideCard, since at 6 carbon leaves no fuel. Compared at 1.21 to 5.30".into(),
            ),
            scalar(
                "hot_ion_regime",
                Role::Load,
                "1",
                Some(0.0),
                false,
                Some(1.0),
                false,
                "0 or 1 and nothing between: 1 for the ELM-free hot-ion H-mode, whose confinement multiplier is 1.77573, fitted on 42976, the one such pulse; 0 for an ELMy H-mode, 0.799621. Any other value is refused as RegimeNotNamed".into(),
            ),
        ],
        floor,
        "a locked 1e-6 s: at 1 MW it moves a store of 20 MJ by one joule, which 64-bit numbers resolve to nine figures",
        gate,
        "a locked 1 ms, the largest step the card stands behind anywhere. The card's own step refuses as DtAboveStateCeiling, with nothing changed, a step longer than an eighth of the shortest of the deuterium and tritium beam ions' thermalisation times, the tail's and the confinement time of the segment's load; over the ten compared runs the shortest is 19.7 ms, so the ceiling there is at least 2.47 ms",
        None,
        "Every load field is held through a segment and a later segment may change any of them: a run of a pulse is one segment for each 25 ms of its measured traces. A value outside its published bound, all four powers zero, a density above the Greenwald density of the segment's current, a current and field whose safety factor is under 2, a regime other than 0 or 1, or an electron temperature whose Coulomb logarithm is zero or less, in any segment, refuses the whole run before its first step. The four stores are held at the start and after every step: the fast stores not negative, the thermal energy above the electrons' energy at the segment's electron temperature, the central ion temperature at most 100 keV and the stores under the Troyon limit; a run that reaches one part way is sealed with that refusal on its last landed step. A run starts at elapsed_time_s with its ledger anchored at the four stores given and no fusion energy released; a run cut in two and continued from a returned state anchors its ledger again and counts its fusion energy from zero. The state reports the fast-ion energy of the three fast stores, the electrons' energy, the central ion temperature, the loss power, the confinement time, the fusion power with its thermal part and the parts of the deuterium and of the tritium beams, the normalised beta, the Greenwald fraction and the safety factor under the load of the last step; the derived values are zero at the start. A frame carries FLAG_OUTSIDE_COMPARED where the load or the landed state is outside the compared envelope. The clock is a running sum from elapsed_time_s, t_s += dt_s in 64-bit arithmetic, and the card has no end in time.".into(),
        &[
            "NonFiniteInput",
            "NonFiniteState",
            "NonPositiveDt",
            "InvalidCard",
            "HeatingPowerNegative",
            "NoHeatingPower",
            "BeamEnergyOutsideCrossSection",
            "PlasmaCurrentNotPositive",
            "ToroidalFieldNotPositive",
            "FuelMixOutsideCard",
            "EffectiveChargeOutsideCard",
            "DensityNotPositive",
            "DensityAboveGreenwald",
            "SafetyFactorBelowLimit",
            "ElectronTemperatureOutsideCard",
            "RegimeNotNamed",
            "EnergyOutsideCard",
            "TemperatureAboveReactivityFit",
            "BetaAboveTroyon",
            "NegativeTime",
            "StepBelowClockResolution",
            "DtAboveStateCeiling",
            "SpontaneousGeneration",
            "FirstLawBreach",
        ],
        vec![
            Advisory { name: "FLAG_NEAR_DENSITY_LIMIT", mask: tokamak_jet_dte1_twin::FLAG_NEAR_DENSITY_LIMIT, seals: true },
            Advisory { name: "FLAG_OUTSIDE_COMPARED", mask: tokamak_jet_dte1_twin::FLAG_OUTSIDE_COMPARED, seals: true },
        ],
        slots(&[
            ("t", "Time since the start of the heated window, in seconds."),
            ("pos[0]", "thermal_energy_j"),
            ("pos[1]", "fast_ion_energy_j"),
            ("pos[2]", "fusion_energy_j"),
            ("vel[0]", "fusion_power_w"),
            ("vel[1]", "ion_temperature_kev"),
            ("vel[2]", "beta_normalised"),
            ("force_torque", "loss_power_w"),
            ("residual", "Conservation residual."),
            (
                "flags",
                "FLAG_NEAR_DENSITY_LIMIT: the line-averaged density is above 0.85 of the Greenwald density. FLAG_OUTSIDE_COMPARED: the load or the landed state is outside the compared envelope of the ten pulses: 1.31 to 23.8 MW of heating, 1.19e19 to 8.87e19 per cubic metre, 1.65 to 4.16 MA, 1.80 to 4.04 T, a central electron temperature of 1.76 to 12.0 keV, an effective charge of 1.21 to 5.30, a tritium fraction of 0 to 0.928, 0.462 to 15.2 MJ thermal and 5.59 kJ to 4.06 MJ fast, a central ion temperature of 0.0568 to 35.1 keV, and 0 to 5.48 s. Both seal.",
            ),
            ("proof", "Not written by the step."),
        ]),
    )
}

/// One step at the ceiling, and a second of the record pulse 42982 near its
/// peak, recorded every 25 steps.
pub(super) fn examples() -> (Value, Value) {
    let specimen = tokamak_jet_dte1_twin::TokamakJetDte1Twin::dte1();
    pair_examples_recorded(
        DOMAIN,
        CARD,
        obj(&[
            ("thermal_energy_j", scalar_value(9.0e6)),
            ("deuterium_beam_ion_energy_j", scalar_value(5.0e5)),
            ("tritium_beam_ion_energy_j", scalar_value(7.0e5)),
            ("tail_ion_energy_j", scalar_value(4.0e5)),
            ("elapsed_time_s", scalar_value(0.0)),
        ]),
        obj(&[
            ("deuterium_beam_power_w", scalar_value(1.05e7)),
            ("tritium_beam_power_w", scalar_value(9.5e6)),
            ("deuterium_beam_energy_kev", scalar_value(80.0)),
            ("tritium_beam_energy_kev", scalar_value(150.0)),
            ("icrh_power_w", scalar_value(2.4e6)),
            ("ohmic_power_w", scalar_value(5.0e5)),
            ("line_density_per_m3", scalar_value(6.4e19)),
            ("plasma_current_a", scalar_value(3.8e6)),
            ("toroidal_field_t", scalar_value(3.9)),
            ("tritium_fraction", scalar_value(0.451)),
            ("electron_temperature_kev", scalar_value(7.4)),
            ("effective_charge", scalar_value(2.4)),
            ("hot_ion_regime", scalar_value(0.0)),
        ]),
        super::card_dt(specimen.gate_dt_s()),
        1000,
        25,
    )
}

/// The record, in the five parts every card gives: the object, the data, the
/// rule it is held to, the result and the limits.
pub(super) fn validation(_specimen: &str) -> [String; 5] {
    [
        "The JET tokamak at Culham in heated pulses of 1996 and 1997 with the Mark IIa divertor, in deuterium and in deuterium-tritium (the DTE1 experiment): ELMy H-modes and one ELM-free hot-ion H-mode. The card follows the thermal energy of the plasma, the energy of its deuterium and tritium beam ions and of its ion-cyclotron tail and alphas, the central ion temperature, and the D-T fusion power in its thermal and beam-target parts through the heated part of a pulse.".to_string(),
        "International Multi-Tokamak Confinement Profile Database, 2008 public release: the 0D, 1D and 2D files of ten JET pulses, 38285, 38287, 38415, 43134 (deuterium; 43134 with 3.4 percent tritium), 42762 and 42982 (an even mix, beams of both species), 42794 and 42997 (three quarters tritium, tritium beams), 43002 (93 percent tritium) and 42976 (hot-ion). The energies, the fusion power and its thermal part and the profiles are TRANSP's reconstruction of each pulse from its measured profiles (R. V. Budny, PPPL). The beams' species and energies are each pulse's comment and 0D file; the absorbed beam power is PNBI less PFLOSS. Four constants are fitted on all ten: the confinement multiplier of the ELMy H-mode, 0.799621, and of the hot-ion H-mode, 1.77573 (42976 alone); the slowing-down time 1.13763 times the classical; the tail's thermalisation time 1.42815 times half the classical time of a proton. From the ten pulses' radial profiles, not fitted to the traces: volume over line density 0.956, the exponents of (1 - x)^a in the volume coordinate of the density 0.14, the electron temperature 0.793, the ion temperature 1.106 and the beam source 0.47; the geometry is the mean of the ten 0D files, R 2.903 m, a 0.9382 m, elongation 1.755, volume 81.88 m3. Bosch and Hale (1992) for the D-T cross-section and reactivity, the NRL Plasma Formulary for the slowing-down time and the critical energy, the IPB98(y,2) scaling for the confinement time.".to_string(),
        "Every step holds the energy ledger of the four stores to 2e-8: the stores against the heating supplied, the alpha share of the fusion energy and the loss energy by the corrected trapezoid rule. Its accuracy is stated as the RMS error over each pulse's heated window, sampled every 25 ms, as a fraction of that pulse's peak, fitted on all ten and with each pulse left out of the fit.".to_string(),
        RESULT.to_string(),
        "Compared at 1.31 to 23.8 MW of heating, 1.19e19 to 8.87e19 per cubic metre, 1.65 to 4.16 MA, 1.80 to 4.04 T, central electron temperatures of 1.76 to 12.0 keV, effective charges of 1.21 to 5.30, tritium fractions of 0 to 0.928, and 5.48 s of the heated window; outside it the card computes the same equations and FLAG_OUTSIDE_COMPARED is set. The electron temperature and the effective charge are the pulse's measured traces, given as loads: the card does not compute them. The ion temperature is what the thermal energy leaves after the electrons, so an error in the thermal energy falls on the ions and is doubled in the thermal fusion power. One shape of each profile serves every pulse; the measured shapes vary from pulse to pulse (ion temperature exponent 0.84 to 1.47). The hot-ion multiplier rests on one pulse: with 42976 left out its fusion energy is 0.392 of the database's. Left out: beam-beam reactions, the target's thermal motion, D-D fusion, ELMs, sawteeth, the L-mode before the transition and orbit losses past the shine-through the caller subtracts. Every fusion power compared with is TRANSP's, not the measured neutron rate. One tokamak, one campaign. The record is evidence/research/tokamak_jet_dte1/twin/, with every request and answer of the ten runs in runs/.".to_string(),
    ]
}

/// Each refusal's sentence, word for word from the doc comment above the
/// variant in the card's source.
pub(super) fn meaning(name: &str) -> Option<&'static str> {
    Some(match name {
        "NonFiniteInput" => "`dt` or a load value was NaN or infinite.",
        "NonFiniteState" => "The incoming state, or the trial step, was non-finite.",
        "NonPositiveDt" => "`dt` was zero or negative.",
        "InvalidCard" => "A recorded constant is not this tokamak's.",
        "HeatingPowerNegative" => "A beam, ion-cyclotron or ohmic power is below zero.",
        "NoHeatingPower" => "The heating supplied is zero: the confinement time of the scaling, `P^-0.69`, has no value.",
        "BeamEnergyOutsideCrossSection" => "A beam's injection energy is zero or less, or its centre-of-mass energy on the target is above 550 keV, the top of the cross-section fit: above 917.25 keV for deuterium and 1373.68 keV for tritium.",
        "PlasmaCurrentNotPositive" => "The plasma current is zero or less.",
        "ToroidalFieldNotPositive" => "The toroidal field is zero or less.",
        "FuelMixOutsideCard" => "The tritium fraction of the fuel is below 0 or above 1.",
        "EffectiveChargeOutsideCard" => "The effective charge is below 1, or at or above 6, where carbon leaves no fuel.",
        "DensityNotPositive" => "The line-averaged density is zero or less.",
        "DensityAboveGreenwald" => "The line-averaged density is above the Greenwald density of the plasma current.",
        "SafetyFactorBelowLimit" => "The edge safety factor of the current and the field is below 2.",
        "ElectronTemperatureOutsideCard" => "The central electron temperature is zero or less, or the Coulomb logarithm of a cell is zero or less.",
        "RegimeNotNamed" => "`hot_ion_regime` is neither 0 nor 1.",
        "EnergyOutsideCard" => "A fast store is negative, or the thermal energy is not above the electrons' energy at the load's electron temperature, so the ions would hold none.",
        "TemperatureAboveReactivityFit" => "The central ion temperature is above 100 keV, the ceiling of the reactivity fit.",
        "BetaAboveTroyon" => "The normalised beta of the four stores is above the Troyon limit of 2.8.",
        "NegativeTime" => "The clock is negative.",
        "StepBelowClockResolution" => "The step is too small to move the clock at its time.",
        "DtAboveStateCeiling" => "The step is longer than an eighth of the shortest of the beam ions' and the tail's thermalisation times and the confinement time of the load.",
        "SpontaneousGeneration" => "The trial step created energy beyond what the heating and the fusion supply.",
        "FirstLawBreach" => "The trial step destroyed energy the heating and the fusion supply.",
        _ => return None,
    })
}
