//! What kind of evidence stands behind each served model, and the spec
//! sheet of each model compared with measurement.
//!
//! `against_measurement` names the kind of evidence, in one of three words
//! ([`COMPARED`], [`REFERENCE`], [`SPECIMEN`]); the numbers say how good the
//! model is. A model compared with measurement is described by its spec
//! sheet, six sentences in a fixed order: what it computes, its range, what
//! it was fitted on, what it was compared with outside the fit and within
//! what, the gap with its number, and where it refuses. Each view says each
//! part once:
//!
//! - `describe_card` and `describe_solver`, full form: the six as one
//!   paragraph (a card's `specimen`, a solver's `summary`);
//! - the short form: what it computes, its range, what it was fitted on and
//!   where it refuses as `specimen` / `summary`, and the comparison and the
//!   gap as `accuracy_against_measurement`;
//! - `list_cards` and `list_solvers`: the six as one paragraph, the row's
//!   one line, as the full form gives it;
//! - `find_model`: what it computes and its range as `model_of`, what it was
//!   fitted on as `compared_with`, and the comparison and the gap as
//!   `accuracy_against_measurement`.
//!
//! A new model compared with measurement adds one row to [`SHEETS`]; the
//! tests below hold every compared model in `find_model::INDEX` to having
//! one, in the shape described.

/// A real object compared with measurements: its record states what it was
/// fitted on and what it was compared with outside the fit, in numbers.
pub const COMPARED: &str = "compared with measurement";
/// Compared with a published calculation.
pub const REFERENCE: &str = "held to a reference calculation";
/// An object the company specified, every constant stated in its description.
pub const SPECIMEN: &str = "company specimen";

/// The three words, in the order `AGAINST_MEASUREMENT_SAYS` gives them.
pub const WORDS: [&str; 3] = [COMPARED, REFERENCE, SPECIMEN];

/// What `against_measurement` holds, said in the description of each tool
/// that returns it.
pub const AGAINST_MEASUREMENT_SAYS: &str = "against_measurement names the evidence behind the model, in one of three words: \"compared with measurement\" (a real object; its description says what it was fitted on, what it was compared with outside the fit and within what, the gap, and where it refuses), \"held to a reference calculation\" (compared with a published calculation), or \"company specimen\" (an object the company specified, every constant stated). The numbers say how good it is.";

/// The spec sheet of a model compared with measurement. Each part is one or
/// more whole sentences, and opens as the test below holds it to.
pub struct SpecSheet {
    /// The model's served name: a card's name, or a solver's.
    pub model: &'static str,
    /// What it computes, of which real object.
    pub computes: &'static str,
    /// "Range: ...": where it was compared, in numbers, and how far it
    /// computes past that.
    pub range: &'static str,
    /// "Fitted on N of <source>: ...", or "Nothing fitted: ...".
    pub fitted_on: &'static str,
    /// "Compared with M not in the fit ...: within X", or, for a model
    /// fitted on all its data, "within X across all N".
    pub compared: &'static str,
    /// "Gap: ...", one line with its number.
    pub gap: &'static str,
    /// "Refuses ...": the limits it refuses at, by name.
    pub refuses: &'static str,
}

impl SpecSheet {
    /// The six parts, in their order.
    pub fn parts(&self) -> [&'static str; 6] {
        [self.computes, self.range, self.fitted_on, self.compared, self.gap, self.refuses]
    }

    /// The six parts as one paragraph: a card's `specimen`, a solver's `summary`.
    pub fn paragraph(&self) -> String {
        self.parts().join(" ")
    }

    /// What the model was compared with and the gap: the short form's and
    /// `find_model`'s `accuracy_against_measurement`.
    pub fn accuracy(&self) -> String {
        format!("{} {}", self.compared, self.gap)
    }

    /// What it computes and its range: `find_model`'s `model_of`.
    pub fn model_of(&self) -> String {
        format!("{} {}", self.computes, self.range)
    }

    /// What it computes, its range, what it was fitted on and where it
    /// refuses: the short form's `specimen` or `summary`, beside its
    /// `accuracy_against_measurement`.
    pub fn short(&self) -> String {
        format!("{} {} {} {}", self.computes, self.range, self.fitted_on, self.refuses)
    }
}

/// The spec sheet of a model, by its served name.
pub fn of(model: &str) -> Option<&'static SpecSheet> {
    SHEETS.iter().find(|sheet| sheet.model == model)
}

/// A solver's `summary`: its spec sheet's paragraph where it has one, else
/// its own sentence.
pub fn summary_of(model: &str, own: &'static str) -> String {
    of(model).map(SpecSheet::paragraph).unwrap_or_else(|| own.to_string())
}

/// The short form's `specimen` or `summary`: its spec sheet's short part
/// where it has one, else its own sentence.
pub fn short_of(model: &str, own: &str) -> String {
    of(model).map(SpecSheet::short).unwrap_or_else(|| own.to_string())
}

/// One row for each model compared with measurement, in the engine's order:
/// the cards, then the solvers.
pub const SHEETS: &[SpecSheet] = &[
    SpecSheet {
        model: "reactor_spert3",
        computes: "Power burst of the SPERT III E-core test reactor, cold, by point kinetics with energy feedback: power, energy released and time, from a step in reactivity to the first maximum of power.",
        range: "Range: compared at 0.769 to 1.210 dollars, coolant 56 to 92 degF, no flow, 5e-5 MW at the start; computes any reactivity above zero and any coolant from 273.15 to 647.096 K with the same equations.",
        fitted_on: "Fitted on 20 of the 30 tests of IDO-17281 Table III: one constant, the strength of the feedback with energy released.",
        compared: "Compared with the 10 tests not in the fit (0.890 to 1.128 dollars, 4.3 to 110 MW at the peak): peak power within 0.58 of the report's standard deviation RMS (worst 1.13), energy to the peak within 1.01 RMS (worst 1.99).",
        gap: "Gap: the peak comes 23 percent early RMS (worst 29 percent), and the energy to the peak reads low, mean -0.705 standard deviations.",
        refuses: "Refuses a first step of zero or less reactivity (ReactivityNotPositive) and a coolant that is not liquid water (CoolantTemperatureNotLiquidWater); a run ends at the first maximum of power (PastFirstMaximum).",
    },
    SpecSheet {
        model: "creep_cmsx6",
        computes: "Creep of one cast of the single-crystal nickel superalloy CMSX-6, loaded along [001] at 980 degC under a held load: the percentage creep extension of a 6 mm round test piece against the time since full load.",
        range: "Range: 140 to 230 MPa, 0 to 5 percent extension; compared from 0.1 to 5 percent at 170 to 230 MPa and from 0.1 to 0.2 percent at 140 MPa.",
        fitted_on: "Fitted on 6 of the 12 BAM creep tests of this cast, two at each of 140, 170 and 230 MPa: six constants.",
        compared: "Compared with the 6 tests not in the fit: time to each level from 0.1 to 5 percent within 0.093 RMS in log time (about 9 percent), worst 0.240, over 33 times; at 200 MPa, a stress no fitted test has, within 0.085 RMS over 18 times.",
        gap: "Gap: below 0.1 percent it starts late, 1.5 to 4 times the hours to 0.05 percent; at 140 MPa past 0.2 percent it runs ahead, about 2 times the extension at the hour the tests stopped (1.70 to 1.78 percent against 0.79 to 0.83).",
        refuses: "Refuses an extension below 0 (CreepExtensionBelowZero) or above 5 percent (CreepExtensionOutsideCard), a stress outside 140 to 230 MPa (StressOutsideCard) and a stress changed during a run (StressNotHeld).",
    },
    SpecSheet {
        model: "pipe_flow_superpipe",
        computes: "Fully developed turbulent air flow in the smooth 129.36 mm bore of the Princeton Superpipe, by a mixing length with van Driest damping (kappa 0.41, A+ 26): Darcy friction factor, centreline-to-bulk velocity ratio and the mean velocity profile.",
        range: "Range: compared at bulk Reynolds number 31,577 to 35,724,000, air at 291.8 to 300.1 K and 100,090 to 18,928,000 Pa; computes fully turbulent flow at 4000 to 1e10 up to bulk Mach 0.3.",
        fitted_on: "Nothing fitted: the outer shape of the mixing length was chosen on the 12 Zagarola-Smits profiles below Reynolds number 1e6.",
        compared: "Compared with 19 friction factors and 21 profiles not used to choose it (McKeon 2004, NSTAP 2012 and 2013): friction factor within 1.38 percent RMS, worst 2.69 (the Prandtl-Karman law 2.17 RMS); velocity profile within 1.59 percent pooled RMS, worst profile 2.58; centreline ratio within 1.33 percent RMS.",
        gap: "Gap: the centreline ratio reads low on 20 of 21 profiles by 0.75 to 1.79 percent, and 2.99 percent low on the NSTAP profile at 81,330; the friction factor drifts low as the Reynolds number rises, 2.7 percent at 35.7 million.",
        refuses: "Refuses laminar flow below 2300 (LaminarFlow), transitional flow from 2300 to 4000 (TransitionalFlow), Reynolds number above 1e10 (ReynoldsAboveGrid), a wall rougher than 0.15 um (RoughWall), bulk Mach above 0.3 (MachAboveIncompressible) and air outside 170 to 1900 K (TemperatureOutsideViscosityLaw).",
    },
    SpecSheet {
        model: "reactor_spert3_fuel",
        computes: "Power burst of the SPERT III E-core test reactor at low initial power in three bands of inlet temperature, by point kinetics with the fuel and the water temperature as state and a square-root Doppler feedback: power, energy released, fuel and water temperature rise and time, from the start of the transient rod's motion to the first maximum of power.",
        range: "Range: compared at 0.769 to 1.210 dollars at 56 to 92 degF with no flow, 0.919 to 1.207 dollars at 251 to 259 degF with 2.4 to 22 ft/s, and 0.837 to 1.227 dollars at 498 to 501 degF with 4.8 to 24 ft/s; computes any reactivity above zero and any flow of zero or more in each band with the same equations.",
        fitted_on: "Fitted on 28 of the 58 tests of IDO-17281 Tables III and V, the 20 cold-startup tests of reactor_spert3's fit and eight at 500 degF: four constants, the power at the start, the Doppler strength, the heat capacity of the fuel and one weight on the water feedback; nothing is fitted at 251 to 259 degF.",
        compared: "Compared with the 30 tests not in the fit (10 cold, 13 at 251 to 259 degF, 7 at 498 to 501 degF): peak power within 0.644 of the report's standard deviation RMS (worst 1.650), energy to the peak within 0.908 RMS (worst 3.212), time of the peak within 6.69 percent RMS (worst 14.88).",
        gap: "Gap: at 251 to 259 degF the peak comes early, 8.46 percent RMS (worst 14.88), and at 498 to 501 degF the energy to the peak reads low on all seven tests, mean -0.890 standard deviations (worst 3.212).",
        refuses: "Refuses an inlet temperature outside or between the three bands (InletTemperatureOutsideCard), zero or less reactivity (ReactivityNotPositive), a reactivity changed during a burst (ReactivityChangedDuringBurst) and a flow below zero (NegativeFlow); a run ends at the first maximum of power (PastFirstMaximum).",
    },
    SpecSheet {
        model: "reactor_spert3_ccore",
        computes: "Power burst of the SPERT III C-core test reactor, 93 percent enriched UO2-stainless-steel plates clad in stainless steel, by point kinetics with six delayed groups and four lumped temperatures: power, energy released and the plate surface temperature at the hot spot, from a step in reactivity at room temperature through the first maximum of power to the first maximum of that temperature.",
        range: "Range: compared at 0.412 to 1.305 dollars with no flow at 0 to 2500 psig, and 0.870 to 1.305 dollars at 230 psig with 200 to 20,000 gpm (0.055 to 5.49 m/s), from starts at 21 to 36 degC; computes any reactivity above zero, any flow of zero or more and any start of liquid water at any pressure above absolute zero with the same equations.",
        fitted_on: "Fitted on 72 of the 111 transients of IDO-16918 kept after 15 were excluded: two constants, the plate-to-coolant conductance with no flow and the share of fission energy deposited directly in the water.",
        compared: "Compared with the 39 transients not in the fit, the whole 1000 psig series among them: peak power within 33.3 percent (exp of the RMS log error, less 1; target 20), energy to the peak within 56.7 percent, maximum plate surface temperature rise within 71.0 percent.",
        gap: "Gap: the nearest fitted row is closer on all three, 16.7, 17.2 and 18.1 percent, and with no pressure term the card reads low on the 1000 psig series by 13.5 percent in peak power, 11.7 in energy and 67.9 in temperature rise.",
        refuses: "Refuses zero or less reactivity (ReactivityNotPositive), a start that is not liquid water at the system pressure (InitialTemperatureNotLiquidWater), a pressure at or below absolute zero (PressureNotPositive), a flow below zero (NegativeFlow) and a reactivity changed during a run (ReactivityChangedDuringRun); a run ends at the first maximum of the hot-spot plate temperature after the power peak (PastPlateTemperatureMaximum).",
    },
    SpecSheet {
        model: "reactor_triga_jsi",
        computes: "Pulses of the TRIGA Mark II reactor of the Jozef Stefan Institute, Ljubljana (56-element core of standard U-ZrH fuel, rod tables of cores 215, 217 and 231), by point kinetics with six delayed groups and fuel-temperature feedback: power, energy released and core-average fuel temperature through the pulse, the scram and after it.",
        range: "Range: compared at 2.0 to 3.0 printed dollars, starts of 20.9 to 29.2 degC, scrams 1.51 to 7.45 s after the peak, up to 954 MW and 585 K; computes any reactivity above zero up to the rod's whole worth of 4.16 printed dollars, any start in liquid water and any scram time or none.",
        fitted_on: "Fitted on 36 of the 43 recorded pulses of cores 215, 217 and 231 in the institute's pulse database, 1.4 to 3.0 printed dollars: ten constants, a scale and an offset for each rod table, two feedback coefficients, the generation time and the heat flow to the water, with five more fitted to the instruments.",
        compared: "Compared with the 20 table rows of the same cores not in the fit, other days on the fitted tables, in units of the comparison's uncertainty: on 12 rows of core 231 from 22 and 28 March 2018, peak power within 1.21 RMS (target 1.5; 18.0 percent), energy within 50 ms of the peak within 1.47 (15.7 percent) and thermocouple maximum within 1.29 (8.0 K); on 8 repeats of core 217 at 2.5 dollars, peak power within 1.09 (5.6 percent high), thermocouple maximum within 0.42 (2.1 K low) and energy within 2.01 (5.4 percent high).",
        gap: "Gap: peak power reads 44 and 23 percent low on the two rows of 22 March 2018, a day whose printed 1.5 and 2.0 dollars stand at rod positions the fitted day prints as 1.75 and 2.25 (worst 3.02, target 3); on the 2018 rows the worst energy and thermocouple maximum reach 3.75 and 3.10 (target 3); below 2.0 dollars peak power reads 29, 21 and 9 percent low at 1.4, 1.6 and 1.8 dollars; the fuel temperature is compared through one thermocouple of one element.",
        refuses: "Refuses a start outside liquid water, 273.15 to 373.124 K (StartTemperatureNotLiquidWater), a reactivity of zero or less (ReactivityNotPositive) or above the rod's whole worth of 4.16 printed dollars (ReactivityAboveRodWorth), a rod table other than those of cores 215, 217 and 231 (RodTableOutsideCard) and a reactivity changed during a run (ReactivityChangedDuringRun).",
    },
    SpecSheet {
        model: "tokamak_jet_dte1",
        computes: "Thermal energy, fast-ion energy and D-T fusion power of the JET tokamak's plasma in an ELMy H-mode pulse of 1996 to 1997, deuterium or deuterium-tritium, stepped under the heating power with the energy ledger closed at every step.",
        range: "Range: compared on nine JET pulses at 1.65 to 3.84 MA, 1.80 to 4.04 T, up to 24.3 MW of heating, up to 0.834 of the Greenwald density, a tritium fraction of 0 to 0.034 or 0.451 to 0.504 and windows up to 5.475 s; computes past that with the same equations, to the limits below.",
        fitted_on: "Fitted on 8 of the 10 JET pulses of the International Multi-Tokamak Confinement Profile Database: four constants.",
        compared: "Compared with the 2 pulses not in the fit, every 25 ms: on 42982, the 1997 fusion-energy record pulse, thermal energy within 7.9 percent of its peak RMS (target 14.5), fast-ion energy 22.5 percent (target 25), fusion power 23.4 percent (target 25), fusion energy 18.4 MJ against 24.8 MJ, a ratio of 0.743 (target 0.75 to 1.25); on 42976, a hot-ion pulse outside the card's regime, 19.2, 25.1 and 34.7 percent and 5.4 MJ against 10.3.",
        gap: "Gap: the fusion power reads low, its peak 3.7 MW against 6.1 MW on 42982 and 5.1 MW against 14.4 MW on 42976; at tritium fractions of 0.75 to 0.93 it is off by 33 to 65 percent of its peak on three training pulses.",
        refuses: "Refuses a density above the Greenwald density (DensityAboveGreenwald), an edge safety factor under 2 (SafetyFactorBelowLimit), a normalised beta above 2.8 (BetaAboveTroyon), a temperature outside the 0.2 to 100 keV of the reactivity fit (TemperatureBelowReactivityFit, TemperatureAboveReactivityFit), a tritium fraction outside 0 to 1 (FuelMixOutsideCard) and a step over its state's ceiling (DtAboveStateCeiling).",
    },
    SpecSheet {
        model: "tokamak_vde_mast",
        computes: "Vertical displacement event of the MAST spherical tokamak's plasma in shot 15045, from 0.250 s: a rigid plasma carrying EFIT's 755 kA in MAST's held coil field, against the eddy currents of its 126 passive elements as one wall circuit and the P6 vertical-control pair driven by the amplifier's voltage, from any start inside the vessel until its boundary reaches the wall: height, velocity, the two currents and the vertical force on the vessel, with the energy ledger closed at every substep.",
        range: "Range: compared with the shot's upward event over its linear phase, 1.97 to 9.05 cm above the baseline from 0.2608 to 0.2746 s, and with its P6 current at the end of that window; computes any start from -0.391 to +0.389 m in height below the Alfven speed of 1.78e6 m/s, with any wall and P6 currents and any amplifier voltage, with the same equations up to the wall.",
        fitted_on: "Nothing fitted to the motion: the plasma, the coils and the passive structure are EFIT's equilibrium and MAST's machine description at 0.250 s of shot 15045 (FAIR-MAST, level 2); the resistivities come from the AMM currents over the flux-loop voltages and a handbook value for stainless steel at 20 degC, and the coils' ampere-turn convention reproduces EFIT's flux outside the plasma to 1.1 percent RMS.",
        compared: "Compared with the measured event of shot 15045: P6U current at the end of the linear phase within 6.2 percent of the shot's (-2,933 ampere-turns against -3,126; target 25); growth rate 6.55 per second against the published 96.3 (target within 25 percent), and the card's synthetic probe signal 7.10 per second against the measured 121.3 by the same rule.",
        gap: "Gap: the growth rate is 93.2 percent low (6.55 per second against 96.3; 94.1 percent low on the probe signal): in the shot P6 held its flux, and a rigid plasma held so needs 2.04 times the rigid shift's stiffness to grow at 96.3.",
        refuses: "Refuses a start at or beyond the contact heights, -0.391 and +0.389 m (StartOutsideVessel), a speed at or above the Alfven speed, 1.78e6 m/s (SpeedAtAlfvenSpeed), and a step of more than 1,048,576 substeps (SubstepsAboveBudget); a run ends when EFIT's boundary reaches the limiter at +0.389 m or -0.391 m (PlasmaAtWall).",
    },
    SpecSheet {
        model: "reactor_spert3_twin",
        computes: "Reactivity-insertion transient of the SPERT III E-core test reactor, 1392 UO2 rods, from cold startup, hot startup at 260 and 500 degF, hot standby or operating power, by six-group point kinetics with the transient rod's worth over its travel, radial conduction in six rings of the average and the hot rod, Doppler and moderator feedback and convection to the water: power, energy released, the fuel, clad and water temperatures and the hot-spot clad temperature, with the neutron and heat ledgers closed at every step.",
        range: "Range: compared at 0.46 to 1.29 dollars, inlet 53 to 513 degF (284.8 to 540.4 K), 0.0853 and 10.44 MPa, no flow to 24 ft/s (7.32 m/s) and initial power 4e-6 to 19 MW; computes any reactivity of zero or more, any inlet above 273.15 K and below saturation and any pressure from 0.000611657 to 22.064 MPa with the same equations, each step outside the compared span flagged FLAG_OUTSIDE_COMPARED.",
        fitted_on: "Fitted on all 70 tests of IDO-17281 Tables III, V and VII, every test read when the structure was chosen: twelve numbers, nine constants of the card and the start powers of the three startup series.",
        compared: "Compared with each of the 70 tests left out of the fit in turn, the twelve numbers refitted on the other 69: at the median test peak power within 6.2 percent, energy to the peak within 7.8, time to the peak within 3.0 and hot-spot clad rise within 10.5; 67 of 70, 62 of 70, 43 of 66 and 61 of 69 tests within two of the report's standard deviations.",
        gap: "Gap: from operating power, 19 MW, the 3 tests read 11.3 percent low in peak power, 81.8 percent low in energy and 41.7 percent early at the median; in hot standby peak power reads 15.5 percent low (test 81: 281 MW against 330), and at 260 degF 13.4 percent high.",
        refuses: "Refuses a reactivity below zero (ReactivityNegative) or changed during a run (ReactivityChangedDuringRun), a start outside the card (StateOutsideCard) and a step above its ceiling (DtAboveStateCeiling); a run ends where a ring of the hot rod reaches 3120 K (FuelMelting) or the core water reaches saturation (WaterAtSaturation).",
    },
    SpecSheet {
        model: "reactor_spert3_ccore_twin",
        computes: "Power burst of the SPERT III C-core test reactor, 93 percent enriched UO2-stainless-steel plates, with subcooled boiling and void: point kinetics with six delayed groups and three plate, film, bulk and vapour nodes, boiling by Thom's law capped by Zuber's critical heat flux: power, energy released and the hot-spot plate temperature through its first maximum.",
        range: "Range: compared at 0.412 to 1.305 dollars with no flow at 0 to 2500 psig, and 0.870 to 1.305 dollars at 230 psig with 200 to 20,000 gpm (0.055 to 5.49 m/s), from starts at 21 to 36 degC; computes any reactivity above zero, any flow of zero or more and any start of liquid water at pressures from 611.657 Pa to 22.064 MPa with the same equations.",
        fitted_on: "Fitted on all 111 transients of IDO-16918 kept after 15 were excluded, every transient and reactor_spert3_ccore's comparison with 39 of them read when the structure was chosen: eight constants, and the time 1.47437 s after the peak at which a plate maximum still rising is read.",
        compared: "Compared with each tenth of the 111 left out of the fit in turn, the eight constants refitted on the rest: peak power within 12.6 percent RMS (105 transients), energy to the peak within 19.6, the hot-spot plate rise at peak power within 13.4 (104) and at its first maximum within 11.4 (95); on 10 transients whose temperatures the report marks abnormal, never in the fit, peak power within 19.0 and energy within 14.4.",
        gap: "Gap: at 230 psig with 5000 to 20,000 gpm peak power reads 21.2 percent RMS and the plate maximum 26.2 (worst 62), and energy with 200 to 2040 gpm 33.8, against a scatter of 8.9, 13.7, 8.5 and 5.5 percent RMS among the record's own repeated transients.",
        refuses: "Refuses zero or less reactivity (ReactivityNotPositive), a start that is not liquid water (InitialTemperatureNotLiquidWater), a pressure outside the steam tables (PressureOutsideSteamTables), a flow below zero (NegativeFlow) and a reactivity changed during a run (ReactivityChangedDuringRun); a run ends at the first maximum of the hot-spot plate temperature after the power peak (PastPlateTemperatureMaximum).",
    },
    SpecSheet {
        model: "reactor_msre_pump",
        computes: "Reactivity the control rods of the Molten-Salt Reactor Experiment supply above their static critical position at zero power while the fuel pump starts or coasts down: six delayed-neutron groups carried round the 25.2 s loop in 252 parcels of salt, born and weighted by one axial sine of flux and importance, the fuel flow following the pump speed with a first-order lag, and a per-group ledger closed at every step.",
        range: "Range: compared on one start-up from the pump stopped and one coast-down from full speed, 0 to 70 s, the pump speed rising no faster than 1.3 and falling no faster than 0.81 of full speed a second; computes any pump speed from 0 to 1 of full speed at any time with the same equations, each step outside the compared runs flagged FLAG_OUTSIDE_COMPARED_RUNS.",
        fitted_on: "Fitted on the ORNL-4233 record, all of it read when the form was chosen: two constants, the flux shape's height, 91.6 in, on the measured steady circulation loss of 0.212 % dk/k, and the fuel-flow lag, 2.55 s, on the 48 start-up rod readings of Fig. 24.",
        compared: "Compared with the 73 coast-down rod readings of Fig. 24, not in the fit: rod compensation within 8.1 pcm RMS, mean +3.6 pcm, largest 23.7; on the 48 start-up readings it is fitted on, within 24.8 pcm RMS, and the steady loss 0.2120 % dk/k against 0.212 +- 0.004.",
        gap: "Gap: on the start-up's first 12 s the card leads the measured rod by up to 55.4 pcm, at 5 s, where the rod drive's own speed holds the rod back and the card has no rod drive; with the constants read from the sources alone the same equations give 0.2546 % dk/k, 72.4 and 36.7 pcm.",
        refuses: "Refuses a pump speed or a fuel flow below 0 or above full speed (FlowOutsideCard).",
    },
    // <<< sheets: nuclear >>>
    SpecSheet {
        model: "generator_mmrtg_f1",
        computes: "Daily-average electrical output of one radioisotope generator, flight unit F1 on the Curiosity rover, from landing on, with no load: electrical power, thermal power and converter fading, with the energy ledger closed at every step.",
        range: "Range: compared on mission days 2,809 to 4,087, the end of the published record; computes from landing to about 2,060 years after it, 6.502e10 s, with the same equations, each step past day 4,087 flagged FLAG_PAST_RECORD.",
        fitted_on: "Fitted on mission days 0 to 2,060 of the published daily record: three constants; days 2,061 to 2,808 chose the structure.",
        compared: "Compared with the 1,279 days 2,809 to 4,087, outside the fit and the choice of structure: within 0.008129 of 115.4 W RMS (about 0.94 W; target 0.010), worst day +0.02251 (2.60 W high, day 3,546).",
        gap: "Gap: it reads high and the gap grows, +0.0031, +0.0045 and +0.0096 of 115.4 W over three successive years and +0.0107 over the last 184 days; the mean over the final 365 days is +0.010094 (about 1.16 W; target 0.010), and later days are expected to read higher still.",
        refuses: "Refuses a mission time before landing (MissionTimeBeforeLanding) and past about 2,060 years after landing, 6.502e10 s, where the decay heat still held falls under 1e-6 of the heat released to day 4,087 (DecayHeatBelowLedgerResolution).",
    },
    SpecSheet {
        model: "satellite_lageos1",
        computes: "Free flight of the laser-ranging satellite LAGEOS-1 from a published position and velocity, in a frame turning with the Earth: the Earth's field to degree and order 12, the Sun and the Moon, the solid tide, sunlight pressure and relativity, with the energy and the angular momentum about the Earth's axis held at every step.",
        range: "Range: compared on the orbits of LAGEOS-1 and LAGEOS-2, radii 11 997.57 to 12 326.40 km, on 28 days of August and September 2026, for up to seven days; computes any orbit down to the Earth's equatorial radius, 6 378.1363 km, any forecast length and any start from 1 January 2017 to 28 June 2027 with the same equations.",
        fitted_on: "Nothing fitted: the Earth's field is EGM2008 to degree and order 12 and sunlight pressure is the analysis centres' 1.13, every constant locked; two LAGEOS-1 weeks chose the structure.",
        compared: "Compared with the 42 runs of the six weeks outside that choice, two of LAGEOS-1 and four of LAGEOS-2: LAGEOS-1 within 0.65 m of the published orbit after one revolution (target 1 m), 3.8 m after one day (10 m) and 32 m after seven days (100 m); LAGEOS-2 within 0.34 m, 2.9 m and 22 m.",
        gap: "Gap: the error grows with the length of a forecast, from 0.65 m after one revolution to 32 m after seven days on LAGEOS-1, as the card carries no drag, no light reflected from the Earth and no ocean tide.",
        refuses: "Refuses a radius under the Earth's equatorial radius, 6 378.1363 km (RadiusBelowEarthRadius), a time before 1 January 2017 or after 28 June 2027, outside its locked TT - UTC of 69.184 s (EpochOutsideLeapSecondSpan), and a forecast time below zero (ForecastElapsedBelowZero).",
    },
    SpecSheet {
        model: "cell_18650pf",
        computes: "Terminal voltage and state of charge of one Panasonic NCR18650PF lithium-ion cell, new, at 25 degC, in a held discharge at one current from a rested cell at any state of charge, with no temperature and no memory of the load before.",
        range: "Range: compared on held discharges from full charge at 1C, 2.899 to 2.89982 A, and at 0.86888 to 0.86970 A; computes any held discharge current above 0 A from any state of charge from 0.03 to 1 with the same equations, each step outside 2.89817 to 2.90065 A flagged.",
        fitted_on: "Fitted on the cell's low-rate and pulse tests: the rest-voltage table on the C/20 test at 0.145 A, the resistance on the 0.87 A holds of the pulse test.",
        compared: "Compared with the eleven held 1C discharges from full charge not in the fit: terminal voltage within 27.8 to 30.0 mV RMS (target 50), largest single error 57 mV; delivered capacity within +0.04 percent (target 3) on the one run to cutoff.",
        gap: "Gap: on one discharge at 0.87 A in 14 holds, not in the fit, 56.7 mV RMS (target 50), 204 and 206 mV in the last two holds, and it runs 71 s past the cell's 2.5 V, delivering 0.60 percent more than the cell.",
        refuses: "Refuses a current of zero or less (CurrentNotDischarging) and a current more than 0.83 mA from the held current (CurrentChangedDuringDischarge); a run ends at 2.5 V (UndervoltageCutoff) or at the bottom of the table, soc 0.03 (StateOfChargeOutsideCard).",
    },
    SpecSheet {
        model: "cell_2170_m80",
        computes: "Terminal voltage of one Tesla Model 3 2170 lithium-ion cell, cell m80, in each of six blocks of tests from -20 to 40 degC, from a terminal current and a measured case temperature, by fitted tables of rest voltage and resistance with one relaxing branch, a step of one second. The physical model cell_2170_model3 reads 17.4 mV lower on one 1.5 A discharge at 12 degC.",
        range: "Range: compared in each block at the case temperatures and currents of its own tests, -21.898 to 45.535 degC and up to 11.375 A discharging and 9.737 A charging; computes to 13.5 A either way at any case temperature from -21.898 to 45.535 degC with the same tables, each step outside its block's tests flagged.",
        fitted_on: "Fitted on 17 characterisation files of cell m80, the slow discharges and the pulse test of each block: 2,307 numbers.",
        compared: "Compared with the 60 tests not in the fit, 12 held discharges and 48 drive cycles: the discharge at a third of the rated current within 5.0, 6.4, 15.0 and 29.2 mV RMS at 40, 25, 10 and 0 degC (target 42); of the 32 drive cycles from 0 to 40 degC, 29 within the voltage target; 27 of 60 runs within every target.",
        gap: "Gap: the held discharge at the rated current reads 48.2, 44.1, 51.2 and 88.2 mV RMS at 40, 25, 10 and 0 degC (target 42), and the -10 and -20 degC blocks read 55.6 to 425 mV, each step there flagged.",
        refuses: "Refuses a block other than 40, 25, 10, 0, -10 and -20 degC (BlockNotRecognized), a current beyond 13.5 A either way (CurrentOutsideCard), a case temperature outside -21.898 to 45.535 degC (CaseTemperatureOutsideCard) and any step but 1 s (StepNotOneSecond); a run ends below 2.8 V (LowerVoltageLimit), above 4.242 V (UpperVoltageLimit) or past the block's tables (ChargeOutsideTables).",
    },
    SpecSheet {
        model: "cell_2170_model3",
        computes: "Terminal voltage of one Tesla Model 3 2170 lithium-ion cell, cell m80, m1000 or m448-N, as a physical model with memory, from a terminal current and a measured case temperature, a step of one second. The table card cell_2170_m80 reads 17.4 mV higher on one 1.5 A discharge at 12 degC.",
        range: "Range: compared at case temperatures from -12.9 to 45.9 degC, currents from -13.984 to 13.524 A and capacities from 4.16 to 4.7 Ah; computes any current, any case temperature above absolute zero and any capacity above zero, each step outside that range flagged.",
        fitted_on: "Fitted on every m80 test from -10 to 40 degC: the rest-voltage curve and 22 numbers; one resistance scale each for m1000 and m448-N, from four of their own tests per temperature.",
        compared: "Compared with the runs of two cells not in the fit, -10 to 40 degC: within every target on 51 of 60 runs of m448-N (48 to 54 expected) and 31 of 45 of m1000 (36 to 41 expected); every ordinary drive cycle it ran, 78 of 80, within 42 mV RMS.",
        gap: "Gap: m1000's discharge at half the rated current gives 3.3 to 4.0 percent too much charge, three of its 40 degC drive cycles end 128 to 145 mV high (target 126), and at -20 degC none of m80's 13 runs is within its targets.",
        refuses: "Refuses a capacity of zero or less (CapacityNotPositive), a case temperature at or below absolute zero (CaseTemperatureBelowAbsoluteZero) and a cell other than its three (CellNotRecognized); a run ends outside 2.8 to 4.242 V (UndervoltageLimit, OvervoltageLimit) or a state of charge of -0.05 to 1.05 (StateOfChargeOutsideCard).",
    },
    SpecSheet {
        model: "runaway_18650_lco",
        computes: "Thermal runaway of one LiCoO2 18650 cell in a held oven, under a heater or in an adiabatic calorimeter: the four decomposition stages of Hatchard et al. (2001) with Kim et al. (2007) constants, one lumped heat capacity, oven loss and an energy ledger.",
        range: "Range: compared with ovens at 145 to 155 degC, heating at 2 degC/min, heat-wait-seek and seven adiabatic records; computes any oven above 0 K, any heater power from 0 W and cell temperatures to 1811 K, where the can melts.",
        fitted_on: "Nothing fitted: every constant published, as Ostanek tabulates the Hatchard-Kim mechanism; the card was built before any measured number was read.",
        compared: "Compared with open records of LiCoO2 18650 cells on 23 lines: within target on 8, 2 of 10 primary and 6 of 13 reprinted; the six adiabatic peaks within 100 K (624.4 against 654.3 to 708.8 degC).",
        gap: "Gap: it self-heats at 83 degC where cells start at 100 to 150 degC (onsets 32.8 to 58.8 K early), runs away in a 155 degC oven at 24.3 minutes against 51.1, and at 150 degC where the cell did not.",
        refuses: "Refuses an oven at or below 0 K (OvenTemperatureNotPositive), a negative heater power (HeaterPowerNegative) and a cell at or above 1811 K, where the steel can melts (CanMelting); a run ends when the heat the reactants still hold falls to 0.0205 J (EveryReactantConsumed).",
    },
    SpecSheet {
        model: "accumulator_sbo450",
        computes: "Gas pressure, temperature and energy ledger of the nitrogen in one HYDAC SBO450-0.6 diaphragm accumulator on the TU Darmstadt rig, as a piston moves oil in and out: Peng-Robinson nitrogen, one time constant of heat exchange with the wall.",
        range: "Range: compared at the 20 and 40 bar charges, 37 to 43 and 55 to 67 bar (20 bar) and 57 to 64 and 75 to 86 bar (40 bar), wall 293.4 to 293.7 K, gas 285.9 to 301.8 K, piston to 0.7 m/s, 0.2 Hz and faster; computes any gas mass up to the rated 330 bar, each step outside that range flagged.",
        fitted_on: "Fitted on the 100 runs from 0.01 to 10 Hz of the two records at 40 bar precharge and 60 bar load: piston area over gas volume, 3.3348 per metre, and the thermal time constant, 17.52 s.",
        compared: "Compared with the 285 runs not in the fit, 40 to 120 bar load and half the charge, 0.01 to 20 Hz: pressure swing per piston travel within 0.93 percent median (target 2.0), lead within 0.27 degrees median (target 0.5); on the 196 runs of the served range, drawn afterwards, within 0.49 percent median, 4.05 at most (target 5.0).",
        gap: "Gap: 2.72 percent median at 109 to 132 bar (target 2.0); up to 13.96 percent too stiff at 0.1 Hz and slower (target 5.0), where it now refuses; median lead above two baselines' 0.10 and 0.11 degrees.",
        refuses: "Refuses a gas mass of zero or less (GasMassNotPositive), a pressure above the rated 330 bar (PressureAboveRated), gas at or below 126.192 K, nitrogen's critical temperature (GasBelowCriticalTemperature), and heat above 0.065 of the work, 0.1 Hz and slower (HeatShareAboveCard).",
    },
    SpecSheet {
        model: "grid_gb",
        computes: "Frequency of the Great Britain synchronous area, the 50 Hz transmission system of England, Scotland and Wales as one rotating mass, after an importing interconnector trips: inertia, demand relief and a fitted primary response, with an energy ledger every step.",
        range: "Range: compared on single trips of 693 to 1402 MW over their first 60 s, frequencies down to 49.553 Hz; computes any loss up to the demand, down to 48.8 Hz and past 60 s with the same equations, each step outside the compared trips flagged.",
        fitted_on: "Fitted on 28 of the 41 trips found in 114 months of frequency: three constants, a response of 2074 MW/Hz below 50 Hz, a further 3136 MW/Hz below the 49.8 Hz knee, and a lag of 1.296 s.",
        compared: "Compared with the 12 trips not in the fit, every third in time order: depth of the fall within 0.166 RMS relative error (target 0.25; 0.182 for depth proportional to loss), steepest second within 0.103 (0.25), frequency near 60 s within 0.046 Hz (0.06), whole trace within 0.036 Hz median (0.05).",
        gap: "Gap: its error follows the calendar, +0.066 a year in depth, and the six compared trips since November 2025 fall 39 to 80 percent deeper than measured; on the 28 fitted trips, each left out in turn, the depth error is 0.302.",
        refuses: "Refuses a loss larger than the demand (LossLargerThanDemand), an inertia of zero or less (InertiaNotPositive) and a frequency above the 50.5 Hz statutory limit (FrequencyAboveCard); a run ends at 48.8 Hz, where low-frequency demand disconnection begins (LowFrequencyDemandDisconnection).",
    },
    SpecSheet {
        model: "link_gothenburg_10010",
        computes: "Path loss of one 32.445 GHz microwave link across 1.26 km of Gothenburg in rain, OpenMRG link 10010 sublink 587, from the rain rate at the gauge beside it: Recommendation ITU-R P.838-3 with nothing fitted for the rain, and an account of the water on the antennas.",
        range: "Range: compared on the nine rain events of August 2015, rain to 54 mm/h, dry levels 52.60 to 52.90 dB, one-minute rates at steps of 60 s; computes any rain rate from 0 mm/h, any dry level from 44.32325388 dB and any step up to about 11.6 years with the same equations.",
        fitted_on: "Fitted on this link's own readings in the 18 rain events of June and July 2015: three constants of the water on the antennas, a ceiling of 2.13 dB, a wetting time of 81 s and a drying time of 404 s.",
        compared: "Compared with the nine August rain events, 773 five-minute blocks not in the fit: within 0.581 dB RMS (target 0.90), 0.585 of the error of P.838 alone (target 0.80), summed attenuation 1.069 of the measured (0.75 to 1.25).",
        gap: "Gap: the card's five-minute peak is below the measured peak in seven of the nine events, by up to 2.2 dB, and the other direction of the link, sublink 588, reads 0.794 dB, a summed attenuation 2.30 of the measured.",
        refuses: "Refuses a rain rate below 0 mm/h (NegativeRainRate), a dry level below 44.32 dB, the path's free-space loss less two ideal 0.3 m apertures (DryPathLossBelowFreeSpace), and antenna water outside 0 to 2.13 dB (AntennaWaterOutsideCard).",
    },
    // <<< sheets: energy >>>
    SpecSheet {
        model: "capsule_stardust",
        computes: "Entry of the Stardust sample return capsule on 15 January 2006, as a point mass with tabulated drag, no lift, no wind and a yearly standard atmosphere: altitude, speed through the air, Mach number, load and heat flux, from entry interface to the drogue-deployment condition the radar recorded.",
        range: "Range: compared on the flown entry from entry interface, 133.9 km, to 31.03 km and Mach 1.23, at 11911.98 to 12573.44 m/s through the air above 68.96 km; computes any path from 0 to 134.7 km with the same equations, at 11900 to 12620 m/s above 68.96 km and Mach 0.16 to 40 below it.",
        fitted_on: "Nothing fitted: the capsule's pre-flight design and navigation values, the drag tables of AIAA 97-2304, the U.S. Standard Atmosphere 1976 and World Geodetic System 1984.",
        compared: "Compared with NASA's reconstruction of the entry and the radar record of parachute opening: peak load 32.17 g against 32.89 g, within 0.72 g (target 3.64 g); the drogue altitude, 31.03 km, reached at 138.909 s and Mach 1.23 at 136.953 s against 137.9 s recorded, within 1.01 s (target 7.4 s).",
        gap: "Gap: heat flux 696 W/cm2 against NASA's computed 762 to 1000; the yearly standard atmosphere in place of that morning's air is worth about 1 g of peak load either way.",
        refuses: "Refuses an altitude above 134.7 km (AltitudeOutsideCard) or below 0 m (BelowStandardSeaLevel), a speed through the air outside 11900 to 12620 m/s above 68.96 km (AirSpeedOutsideCard) and a Mach number below 0.16, or above 40 below 68.96 km (MachOutsideCard); a run ends under 31030 m and Mach 1.23 at once (PastFlightDeployment).",
    },
    SpecSheet {
        model: "capsule_insight",
        computes: "Entry of NASA's InSight capsule at Mars on 26 November 2018, as a point mass with the 2008 Phoenix drag database, no wind, and the lift over drag as the load: altitude, speed through the air, Mach number, dynamic pressure and load, from the top of the atmosphere to the flown parachute deployment.",
        range: "Range: compared on the flown entry from the locked entry state, 126.5 km up, to parachute deployment 209.3 s later, down to 5793 m and Mach 1.470, with lift over drag from -0.08 to 0.08; computes any path from -2634.5 m to 128 km, from Mach 0.9 to 5850 m/s through the air, with the same equations.",
        fitted_on: "Fitted on the first 150 s of the flight's record: one constant, the lift over drag, -0.031, held through the run; the air is orbiter soundings set on the lander's barometer, the drag the Phoenix database, the entry mass 605.648 kg, and no number comes from the accelerometer.",
        compared: "Compared with the last 59 s before the parachute, not in the fit: speed within 7.2 m/s RMS, radius within 245 m and deceleration within 0.031 g over the 58 whole seconds; at the flight's deployment time the radius within 114 m (target 3.5 km) and Mach 1.501 against 1.522 (target 0.2); the deployment speed reached 1.31 s late and the radius 0.58 s early (target 15.1 s).",
        gap: "Gap: with no lift the run reaches the flight's radius at the parachute 22.8 s late and is 4.24 km high there (targets 15.1 s and 3.5 km); between 36 and 65 km the card's air is 8 to 13 percent denser than the flight's, and the peak load comes 4.0 s early.",
        refuses: "Refuses a lift over drag outside -0.08 to 0.08 (LiftOutsideCard), an altitude above 128 km (AltitudeOutsideCard) or below -2634.5 m (BelowLandingSite), and a speed through the air below Mach 0.9 (MachOutsideCard) or above 5850 m/s (AirSpeedOutsideCard); a run ends under 6164.62 m and 352.1169 m/s at once (PastFlightDeployment).",
    },
    SpecSheet {
        model: "cantilever_eth",
        computes: "Free ring-down of one aluminium strip clamped at one end at ETH Zürich, in room air, in its first bending mode, with a fitted softening spring and fitted damping: the deflection and speed at the laser-1 station, 268 mm from the clamp, with no load.",
        range: "Range: compared on envelopes from 15.79 mm down to 0.058 mm, over 12 s to 24 s of a second ring-down; computes the same equations from 18.75 mm, the largest amplitude fitted, down to rest, and flags each frame above 15.79 mm or below 0.058 mm.",
        fitted_on: "Fitted on one recording of the ETH set, test1a, from 0.863 s to 24 s: six constants, among them the frequency at vanishing amplitude, 8.29235 Hz, and power laws of softening and damping on amplitudes from 0.072 to 18.75 mm.",
        compared: "Compared with 12 s to 24 s of a second ring-down of the same strip in the same clamp, test1b, not in the fit, in eleven blocks of eight periods: frequency within 0.0031 Hz (target 0.010), timing within 0.021 of a period (target 0.20), amplitude within 7.03 percent (target 5.0).",
        gap: "Gap: the swing reads high in every block, 3.6 percent at 13 s growing to 11.7 percent at 23 s: the ring-down dies away too slowly once the swing is small.",
        refuses: "Refuses an envelope above 0.01875 m, the largest amplitude fitted (EnvelopeAboveCard), and a step outside 9.5367431640625e-10 to 2.5e-4 s (DtBelowFloor, DtAboveCeiling).",
    },
    SpecSheet {
        model: "crater_ryugu_sci",
        computes: "Growth of the crater made on asteroid Ryugu by the 2 kg copper impactor of Hayabusa2 on 5 April 2019, by the law of crater size for dry sand held by gravity, with a laboratory clock: the radius at the level of the undisturbed surface from 0.1622 m until it stops, with no load.",
        range: "Range: compared at its final radius on Ryugu, 2.5 decades in scaled gravity below the smallest laboratory crater, and in the laboratory at 1 to 464 times Earth's gravity; computes the growth from 0.1622 m to the stop at 8.571304329477922 m with the same equations.",
        fitted_on: "Fitted on the thirteen laboratory craters of Schmidt in dense dry Ottawa Flintshot sand at 1, 111 and 464 times Earth's gravity: two constants, the exponent 0.1661 and 0.871; nothing fitted to Ryugu.",
        compared: "Compared with the measured crater on Ryugu, not in the fit: final radius 8.571 m against the apparent radius of 7.25 m, within 1.32 m, 18.2 percent (allowance 0.97 m); in the laboratory, fitted on the nine craters at 1 g, the four centrifuge craters within 6.9 to 9.5 percent, mean 7.9 percent (target 10), each inside its 95 percent prediction interval.",
        gap: "Gap: on Ryugu the card stops 1.32 m, 18.2 percent, above the measured apparent radius, outside the allowance of 0.97 m; the growth and its clock, read by eye at 1 g, have no measurement of Ryugu to meet.",
        refuses: "Refuses a radius under 0.1622 m (RadiusBelowLedgerResolution) or above 8.571304329477922 m, where this crater stops (RadiusAboveFinal), and a continuation of a crater still growing (ContinuationNotOnCard); at the stop the radius holds and a step moves the time alone.",
    },
    SpecSheet {
        model: "solar_system",
        computes: "The Solar System as thirteen point masses in free motion: the Sun, Mercury, Venus, the Earth, the Moon, the barycentres of the Mars, Jupiter, Saturn, Uranus and Neptune systems, and Ceres, Pallas and Vesta, under Newton's and the relativistic forces between every pair and the Earth's oblateness on the Moon. The caller supplies every position and velocity.",
        range: "Range: compared from JPL's state of 1999-01-01 for up to 16.2 years with Deep Space Network ranges, and to 21 years with JPL's DE441; computes any state with each body inside its band, from Mercury at 0.29 to 0.48 au from the Sun to Neptune at 29.5 to 30.7 au, with the same equations, and long runs go on.",
        fitted_on: "Nothing fitted: the constants are JPL's DE440 and the start state is DE441's, which JPL fitted to observations, these ranges among them; the Mars Global Surveyor ranges of 1999 to 2006 chose the card's structure.",
        compared: "Compared with 10,134 ranges of three spacecraft in four files, not used to choose it, in one run of 7,670 days: Mars Odyssey to 2006 within 695 m (target 700 m), 270 m RMS; Odyssey after 2006 and Mars Reconnaissance Orbiter, out to 15.0 years, 612 and 619 m RMS, worst 1,312 and 1,314 m; MESSENGER at Mercury, 12.3 to 16.2 years out, within 2.08 km (target 2.5 to 4.4 km), 1.42 km RMS.",
        gap: "Gap: in mid-2007, 44 Odyssey ranges and 34 Mars Reconnaissance Orbiter ranges are outside their targets, by at most 8 and 10 percent, from the 340 asteroids the card leaves out; the Juno and Cassini files are outside the comparison, as they leave 20 km and 10 km RMS with JPL's own positions.",
        refuses: "Refuses a body outside its distance band or its orbit band (DistanceOutsideCard, OrbitOutsideCard), a centre of mass moving faster than 0.01 m/s (FrameOutsideCard), axes in which the Earth-Moon orbit is not 21 to 27 degrees from the z axis (AxesOutsideCard), and two bodies at one point (BodiesCoincide).",
    },
    SpecSheet {
        model: "tire_eve_r19_ice",
        computes: "Longitudinal force of one 235/55 R19 all-season tyre of the European project EVE on the prepared ice of the Terramechanics Rig at Virginia Tech, at a creep of about 0.06 m/s and a held slip, driven and retarded: the force as a friction ratio of the normal load, with the wheel work, the drawbar work and the slip heat.",
        range: "Range: compared at loads of 3.5 to 6.5 kN, pressures of 1.5 to 3.5 bar, slips of 5 to 20 percent in size and 5 s to 21 s after a slip is set; computes any load and pressure above zero and any hold with the same law, and flags each frame outside that span (FLAG_BEYOND_COMPARED).",
        fitted_on: "Fitted on 72 of the 252 runs of the EVE ice dataset, at 3.5 and 5 kN and at 1.5 and 3.5 bar: three constants, the friction ratio 0.228109 in traction and 0.24162 in braking and the slip exponent -0.132823.",
        compared: "Compared with the 60 cells, three repeats each, of the runs not in the fit, at 6.5 kN and at 2.5 bar: friction ratio within 0.034 RMS (target 0.045), 0.029 at 6.5 kN and 0.040 at 2.5 bar; a ratio of 0.1 gives 0.122 and the fitted runs' mean with no slip term 0.036.",
        gap: "Gap: the error sits between test sessions, not repeats: the 18 condition means run 0.185 to 0.287, 0.029 apart in standard deviation against 0.005 to 0.011 between repeats, so a session carries about ±0.03 on the ratio; at 6.5 kN the card reads high by 0.023 on average.",
        refuses: "Refuses a slip below 0.05 or above 0.20 in size (SlipOutsideCard), a carriage speed outside 0.05 to 0.07 m/s (CarriageSpeedOutsideCard), a normal load of zero or less (NoContact) and a gauge pressure of zero or less (TyreFlat).",
    },
    SpecSheet {
        model: "los_jwst_fine_guide",
        computes: "The line of sight of the James Webb Space Telescope in quiet fine guide, read by its Fine Guidance Sensor every 64 ms: a Kalman filter through one fine-guide file's flight centroids, giving on both axes the next centroid and its stated variance.",
        range: "Range: compared on the quiet frames of 24 visits of May and June 2022 at 62,000 to 816,000 counts per second; computes any lock point within 70 arcsec, any count rate above zero and the frames inside disturbances, which it flags.",
        fitted_on: "Fitted on 127 files of 19 visits of April 2022, 936,273 frames: eighteen constants, nine an axis, fitted to this observatory's own centroids.",
        compared: "Compared with the 795,021 quiet frames of 134 files not in the fit: error one frame ahead 0.510 mas on x and 0.472 on y RMS, within 0.790 and 0.807 of the last centroid's (target 0.85); squared error over the stated variance 1.004 and 0.986 (target 0.67 to 1.50).",
        gap: "Gap: inside disturbances it is worse than the last centroid, 2.03 against 1.00 mas on x and 1.57 against 0.78 on y, its stated variance 12 to 17 times too small; it flags those frames.",
        refuses: "Refuses a lock point beyond 70 arcsec (LockOutsideSensorField), a centroid more than 250 mas from the lock point (OffsetOutsideCard) and a count rate of zero or below on a measured frame (CountRateNotPositive).",
    },
    SpecSheet {
        model: "fingerpad_p02_glass",
        computes: "One human finger pad, the right index finger of participant P02 of the torsion experiment at UCLouvain, pressed on flat glass and twisted about the normal until it slips, as a bed of springs under a parabolic friction limit: the torque, the stuck fraction of the contact, the work and the heat.",
        range: "Range: compared at 0.5 to 10 N and 5 to 100 degrees a second, from 1 to 75 degrees of twist; computes every force and rate above zero and the twist past 75 degrees with the same equations, and flags each frame outside that span (FLAG_BEYOND_COMPARED).",
        fitted_on: "Fitted on 80 of P02's trials of the UCLouvain torsion dataset, at 0.5, 2 and 5 N and 5 to 50 degrees a second, 70 of them in the block at 2 N: eight constants fitted to this finger's own trials.",
        compared: "Compared with P02's 30 trials not in the fit, at 1 N, 10 N and 100 degrees a second: slip torque within 9.4, 29.3 and 13.0 percent of the mean of ten trials (target 5.62); torque curve within 0.073, 0.263 and 0.115 of the slip torque RMS (target 0.0832).",
        gap: "Gap: the slip torque reads +9.4, +29.3 and -13.0 percent at 1 N, 10 N and 100 degrees a second, and +13.6 and +21.7 percent on the fitted trials at 0.5 and 5 N, against a target of 5.62; another block of the same finger differs by 13 to 15 percent, another finger by about a fifth.",
        refuses: "Refuses a force of zero or less (NoContact), a rate of zero or less (TwistRateNotPositive), a start at any twist but zero (TwistNotStartedFromZero), a load changed during a twist (LoadChangedDuringTwist) and a step that turns the plate by more than 0.05 of the characteristic twist of the torque (TwistStepAboveQuadrature).",
    },
    // <<< sheets: aerospace and the rest >>>
    SpecSheet {
        model: "generator_mmrtg_seebeck",
        computes: "Daily-average electrical output of the radioisotope generator on the Curiosity rover, flight unit F1, in the Seebeck matched-load form, the second form of this generator beside generator_mmrtg_f1: electrical power, thermal power and energy from landing on, with no load, the fuel's decay entering twice.",
        range: "Range: compared on mission days 0 to 4,087, every published day; computes from landing to 6.502e10 s, about 2,060 years, with the same equations, each step past day 4,087 carrying FLAG_PAST_FITTING_SPAN and FLAG_PAST_RECORD: those days are its forecast, 0.677319 of 115.4 W on day 4,088 and 0.633481 on day 5,183.",
        fitted_on: "Fitted on all 4,088 published days, mission days 0 to 4,087 of the Dryad record of the generator: three constants, by least squares, each day weighted equally.",
        compared: "Compared with all 4,088 days, all in the fit: within 0.00319 of 115.4 W RMS across all 4,088, about 0.37 W; on days 2,809 to 4,087 within 0.00397 RMS, with a mean of +0.00132 over the last 365 days.",
        gap: "Gap: its largest error on one day is +0.0160 of 115.4 W, 1.85 W, on mission day 3,546; refitted with any one Mars year left out, its forecast for day 5,183 moves over 0.6308 to 0.6390.",
        refuses: "Refuses a mission time before landing (MissionTimeBeforeLanding) and one past 6.502e10 s, where the decay heat the fuel still holds falls under 1e-6 of the heat released to mission day 4,087 (DecayHeatBelowLedgerResolution).",
    },
    SpecSheet {
        model: "momentum_jwst_dihedral",
        computes: "Total angular momentum of the James Webb Space Telescope at Sun-Earth L2, held by its reaction wheels between momentum unloads under sunlight pressure on its sunshield, from its attitude quaternion and the Sun's direction and distance: the first layer as two plane slopes of 110 m² meeting along a ridge on the boresight, each falling 4.03 degrees towards the Sun, so that the Sun's roll turns into torque about the boresight.",
        range: "Range: compared on 15 July to 14 September 2023, the Sun at a pitch of -44.5 to +4.5 degrees and a roll of -6.3 to +6.4 degrees, |H| 1.9 to 54.9 N m s, runs of up to 29.3 days; computes any attitude with both slopes lit and at or under 673.15 K (from about 0.27 au) with the same equations, and flags a Sun outside the field of regard (FLAG_OUTSIDE_FIELD_OF_REGARD).",
        fitted_on: "Fitted on the 64 days 12 May to 14 July 2023 of the observatory's momentum telemetry, 1,404 hourly blocks: four numbers, the ridge's point and the dihedral angle.",
        compared: "Compared with the two months after the fit: the daily growth of |H| within 0.1 of the observed plus 0.2 N m s on 25 of 28 and 22 of 30 days (target 80 percent); the hourly growth rate within 0.272 and 0.375 of its spread RMS (target 0.5); the torque within 0.144 and 0.149 of the RMS torque (target 0.4); |H| at a run's end within 4.9 percent over 3.9 to 12.4 days and 5.5 percent after 29.3 days (target 10).",
        gap: "Gap: the daily change of the vector H is within that allowance on 18 of 28 and 13 of 30 days (target 80 percent), 0.42 N m s RMS against momentum_jwst_srp's 0.81; beyond 2 degrees of roll its torque about J3 is off by 11 and 10.4 µN m RMS.",
        refuses: "Refuses a quaternion or Sun direction whose norm is outside 0.999999 to 1.000001 (QuaternionNotUnit, SunDirectionNotUnit), the Sun at or behind either slope (SunBehindSunshield), a distance inside the Sun (SunDistanceInsideSun) and a slope above 673.15 K, Kapton's top (SunshieldAboveKaptonLimit).",
    },
    SpecSheet {
        model: "momentum_jwst_srp",
        computes: "Total angular momentum of the James Webb Space Telescope at Sun-Earth L2, held by its reaction wheels between momentum unloads under sunlight pressure on its sunshield, from its attitude quaternion and the Sun's direction and distance: one sunlit layer of 220 m² with JWST's published optical numbers.",
        range: "Range: compared on 15 June to 14 September 2023, the Sun at a pitch of -44.5 to +4.6 degrees and a roll of -6.3 to +6.4 degrees, |H| 1.6 to 54.9 N m s, runs of up to 29.3 days; computes any attitude with the layer lit and at or under 673.15 K (from about 0.27 au) with the same equations, and flags a Sun outside the field of regard (FLAG_OUTSIDE_FIELD_OF_REGARD).",
        fitted_on: "Fitted on the 34 days 12 May to 14 June 2023 of the observatory's momentum telemetry, 763 hourly blocks: three numbers, the centre of pressure in the J-frame.",
        compared: "Compared with the three months after the fit: the daily growth of |H| within 0.1 of the observed plus 0.2 N m s on 23 of 28, 15 of 28 and 17 of 30 days (target 80 percent); the hourly growth rate within 0.437, 0.384 and 0.421 of its spread RMS (target 0.5); the torque within 0.262, 0.314 and 0.347 of the RMS torque (target 0.4); |H| at a run's end within 8.4 and 0.9 percent after 10.7 and 16.3 days and 7.4 percent after 29.3 days (target 10).",
        gap: "Gap: the daily change of the vector H is within that allowance on 14 of 28, 8 of 28 and 8 of 30 days (target 80 percent): beyond 2 degrees of roll its torque about the boresight is off by about 30 µN m; |H| after an unload in the second month reads 16.2 percent low at 3.9 days.",
        refuses: "Refuses a quaternion or Sun direction whose norm is outside 0.999999 to 1.000001 (QuaternionNotUnit, SunDirectionNotUnit), the Sun at or behind the layer (SunBehindSunshield), a distance inside the Sun (SunDistanceInsideSun) and a layer above 673.15 K, the top of NASA's range for Kapton (SunshieldAboveKaptonLimit).",
    },
    SpecSheet {
        model: "grasp_lift_glass_4tu",
        computes: "Lift of a glass-faced object from a table between two finger pads: a company specimen object, 0.2 kg with two vertical glass faces 60 mm tall, gripped at their middle by two pads with the friction and stiffness of the 4TU set's human index finger pads on glass, each a Cattaneo-Mindlin contact that sticks, slips at its edge and slides whole: the object's height and velocity, each pad's force, travel and stuck fraction, and whether the grip slips.",
        range: "Range: the pads compared on natural glass at normal forces from 0.35 to 2.98 N on twelve fingers; computes any grip above 0 N and any hand motion with the same equations, and finds the least grip that lifts the object without slip, 1.8816 N under the worked lift against 1.142 N at rest.",
        fitted_on: "Fitted on 9 of the 12 fingers of the 4TU set, S1 whole and S4 to S11 at six of its seven lubrication levels: two constants, Coulomb friction 0.858686 and a tangential stiffness of 339.696 N/m.",
        compared: "Compared with the trials not in the fit, S2, S3 and S12 whole and level 4 of S4 to S11: on the 12 trials on natural glass of three fingers at 0.74 to 2.67 N, the card's friction within 22.2 percent median (target 10; a constant force 31.5, each finger's own coefficient 8.8) and its plate travel to full slip within 38.1 percent median (target 18; a constant travel 33.8); over 116 trials the lateral force at the onset of full slip within 6.7 percent median of the trial's own sliding friction times its normal force (target 10), one coefficient serving for static and kinetic friction.",
        gap: "Gap: the stuck-fraction law, the stick ratio as the safety margin to the power 2/3, is off the measured stick ratio at 1 mm by 0.389 RMS over 16,126 samples of incipient slip (target 0.10; the ratio equal to the margin, 0.334); on the fingers not in the fit S2 reads 21 to 35 percent low in friction and S12 up to 25 percent high.",
        refuses: "Refuses a grip of 0 N or below (NoContact) and pads that slide down the faces to the table's surface (PadsReachTable); a run ends where a pad slides past the end of its face, 0.03 m from the grip (ObjectSlippedOut), or where the lifted object comes back down onto the table (ObjectLanded).",
    },
    // <<< sheets: card files >>>
    SpecSheet {
        model: "argon_pair_gas",
        computes: "Argon gas at low density from one Lennard-Jones 12-6 pair potential: second virial coefficient, speed of sound at the asked pressure, and viscosity at zero density.",
        range: "Range: speed of sound compared at 250 to 349.868 K and 30.5 to 467.2 kPa, viscosity at 291.09 to 681.96 K; computes every property from 250 to 6000 K and 0 to 500 kPa with the same equations.",
        fitted_on: "Fitted on 42 of the 66 measured speeds of sound at or below 500 kPa, eleven isotherms from 250 to 300.117 K: two constants, a well depth of 119.33 K and a zero distance of 3.3918 angstrom.",
        compared: "Compared with the 24 speeds of sound not in the fit (310 to 349.868 K): within 0.0074 m/s RMS (worst 0.0146), 0.023 of the ideal gas's error; with 49 viscosities, none fitted (291.09 to 681.96 K): within 1.47 percent mean (worst 2.97).",
        gap: "Gap: the viscosity reads high at all 49 points, 2.95 percent at 291 K falling to 0.28 percent at 682 K, 1.4 to 20 times the measurement's uncertainty.",
        refuses: "Refuses a temperature outside 250 to 6000 K (TemperatureOutOfRange), argon beginning to ionize above 6000 K, and a pressure outside 0 to 500 kPa (PressureOutOfRange), the reach of the pressure series cut after its first term.",
    },
    SpecSheet {
        model: "blast_wave_tnt_air",
        computes: "The incident blast wave of a charge of TNT equivalent in still sea-level air, a sphere in free air or a hemisphere on rigid ground: arrival time, peak overpressure, positive-phase duration and impulse, the Friedlander form and the pressure history, from a point source of 4.184 MJ per kg carried by the spherical Euler equations to Sachs distance 4 and by weak-shock theory beyond, the air's energy above ambient held to the charge's at every step.",
        range: "Range: compared at 0.255 to 0.904 m from surface bursts of 0.37 g of propane and oxygen, given as 0.7317 to 0.8841 g of TNT; computes from Z = 0.5798 m/kg^(1/3) in free air (0.7305 on the ground) out to 843.45 m, for charges up to 106,342,834 kg in free air, with the same equations.",
        fitted_on: "Nothing fitted: every constant is the air's, TNT's 4.184 MJ/kg, Sedov's, or a numerical choice held to the solver's own grid refinement.",
        compared: "Compared with the 24 gauge traces of the rich and lean series (targets: peak within 15 percent, impulse within 20, arrival within 5): no trace within its targets; the peak 33 to 72 percent above the measured (median 44.5), the impulse 43 to 78 percent above (median 54, on 13), the arrival 5.2 to 25.4 percent early (median 10.9).",
        gap: "Gap: a point source of a gas charge's heat of reaction overstates its blast; the measured peaks match 38 to 58 percent of that heat (median 46), and against TNT, by Dewey's measurement-derived tables, the peak reads 22 to 33 percent low from Z 1.3 to 2 and 6 to 17 percent low from 3 to 15.",
        refuses: "Refuses a point nearer than Z = 0.5798 m/kg^(1/3) in free air or 0.7305 on the ground (ChargeProductsNearField), a distance of 843.45 m or more (DistanceBeyondUniformAtmosphere) or under 6.37e-6 m (DistanceBelowContinuum), a wave whose shock is past 843.45 m when the positive phase at the point ends (WaveBeyondUniformAtmosphere), and a shock whose rise lowers the peak by more than 2.5 percent (ShockNotSharp).",
    },
    SpecSheet {
        model: "culture_trichoderma_ambr250",
        computes: "One fed-batch culture of the fungus Trichoderma sp. VTT D-221704 in an ambr 250 vessel at pH 4.0 and 35 °C, turning D-galacturonate into mucic acid: biomass, glucose, galacturonate, mucic acid, carbon dioxide made and oxygen taken up at the end of every segment of the caller's feeding record, carbon and electrons closed at every segment end.",
        range: "Range: compared on the twelve cultures' own starts, feeds (glucose 10.077 to 52.101 g/L) and schedule, to 312 h; computes any start, feed and schedule from zero with the same equations, to 1000 h, while the broth stays inside the 250 mL vessel.",
        fitted_on: "Fitted on 8 of the 12 cultures, fed 29.375 to 34.676 g/L glucose with 0 to 2 g/L yeast extract: six constants.",
        compared: "Compared with the 4 cultures not in the fit, fed about 10.6 and about 50 g/L glucose, RMS error against targets of one and a half times the spread between repeats: oxygen taken up within 0.0108 mol (target 0.0153), dry biomass to 170 h within 1.05 g/L (target 2.15), glucose within 0.51 g/L (target 0.85); galacturonate 4.60 g/L RMS (target 2.74), mucic acid 6.45 g/L (target 4.51), carbon dioxide 0.01125 mol (target 0.00265).",
        gap: "Gap: outside the fitted feed glucose it makes mucic acid too fast, by 4.4 to 6.8 g/L on average, and galacturonate and mucic acid are 6 to 8 g/L off from about 60 h on.",
        refuses: "Refuses a concentration below zero (BiomassNegative, GlucoseNegative, GalacturonateNegative, FeedGlucoseNegative, FeedGalacturonateNegative), a broth not above 0 or over 250 mL (VolumeOutOfRange), a record past 1000 h (RunTooLong) and a record whose step halvings do not settle (NotSettled).",
    },
    SpecSheet {
        model: "culture_trichoderma_atp",
        computes: "One fed-batch culture of the fungus Trichoderma sp. VTT D-221704 in an ambr 250 vessel at 35 °C, turning D-galacturonate into mucic acid on the energy the glucose supplies, oxidised where glucose is plentiful and burnt to carbon dioxide as it runs short: biomass, glucose, galacturonate, mucic acid, carbon dioxide made, oxygen taken up and galacturonate burnt at the end of every segment of the caller's feeding record, carbon and electrons closed.",
        range: "Range: compared at pH 3.0 to 4.0, feeds of 10.077 to 52.101 g/L glucose and records to 312 h; computes any start, feed and schedule from zero and any pH from 0 to 14 with the same equations, to 1000 h, and marks an answer outside the compared pH, yeast extract, feed glucose or record length.",
        fitted_on: "Fitted on 12 of the 16 cultures, all at pH 4.0, fed 10.077 to 52.101 g/L glucose with 0 to 2 g/L yeast extract: eight constants.",
        compared: "Compared with the 4 cultures not in the fit, at pH 3.5 and 3.0, RMS error against the targets of culture_trichoderma_ambr250: galacturonate within 1.83 g/L (target 2.74), mucic acid within 4.19 g/L (target 4.51), oxygen taken up within 0.01396 mol (target 0.0153); dry biomass to 170 h 3.64 g/L (target 2.15), glucose 1.80 g/L (target 0.85), carbon dioxide 0.01065 mol (target 0.00265).",
        gap: "Gap: at pH 3.0 the measured batch runs slower than the solver's, 5.81 g/L of glucose left at 45 h against 0.01, and the measured dry weight at pH 3.5 and 3.0 climbs to 13.8 to 17.4 g/L by 165 h where the solver holds 8.1 to 8.6.",
        refuses: "Refuses a pH outside 0 to 14 (PhOutsideAqueousScale), a concentration below zero (BiomassNegative, GlucoseNegative, YeastExtractNegative and the others), a broth not above 0 or over 250 mL (VolumeOutOfRange), a record past 1000 h (RunTooLong) and a record whose step halvings do not settle (NotSettled).",
    },
    SpecSheet {
        model: "fusion_pb11_lhd",
        computes: "Proton-boron-11 fusion of one hydrogen neutral beam in the boron-seeded hydrogen plasma of the Large Helical Device, the beam protons slowed on electrons, protons and boron in twenty radial shells: alpha particles per second, fusion power, the share of beam protons that fuse, the alpha emissivity profile, and the fast protons over the electrons.",
        range: "Range: compared at beams of 45 and 127.45 to 160.25 keV, 0.9389 to 4.515 MW, line-averaged electron density 2.261e18 to 6.872e18 m^-3 and central temperature 2.1252 to 3.7872 keV; computes beams above 10 to 190 keV, any power above zero, 1e18 to 3e19 m^-3 and 0.5 to 6.67 keV with the same equations.",
        fitted_on: "Nothing fitted: every constant is from the published LHD papers or a declared assumption of the solver, and the boron profile is digitised from a published curve, fitted to no alpha rate.",
        compared: "Compared with measured alpha count rates under negative-ion beams of 127.45 to 160.25 keV and a 45 keV beam, each shape after one common scale factor (target: every point within a factor of 2): the six beam-energy points in the measured order (Spearman 0.943), worst factor 2.94; the time shape within a worst factor of 2.42; the collapse under the 45 keV beam to 1.26e-5 against a measured 0.062, within its target of at most the measured ratio.",
        gap: "Gap: from 128 to 160 kV it rises 11.1 times where the measurement rises 2.10, below the 4.4 that a cross-section with no resonance at all gives; through the published detection efficiency its absolute count rate is 4.7 times the measured one.",
        refuses: "Refuses a beam at or below 10 keV or above 190 keV (BeamEnergyOutOfRange), an electron density outside 1e18 to 3e19 m^-3 (ElectronDensityOutOfRange), a central temperature outside 0.5 to 6.67 keV (ElectronTemperatureOutOfRange), boron that outweighs the electrons in any shell (BoronChargeExceedsElectrons) and fast protons that with the boron would (FastProtonChargeExceedsElectrons).",
    },
    SpecSheet {
        model: "icf_hotspot_burn",
        computes: "Burn of a stagnated deuterium-tritium hot spot of inertial confinement fusion, from the mirror image of the ideal implosion through peak compression and ignition to disassembly: a 0-D hot spot (one temperature, Bosch-Hale burn, Krokhin-Rozanov alpha deposition, Betti's conduction ablation, bremsstrahlung with free-free absorption) inside a 64-zone Lagrangian shell of cold fuel, with an energy, particle and mass ledger at every step: yield, burn-up, burn width, the neutron-averaged hot spot, the energy accounts and the field's ignition measures.",
        range: "Range: compared with seven NIF shots, N201101 to N221204, at T_DD 4.61 to 12 keV, 297 to 608 Gbar and 0.200 to 0.220 mg of fuel; computes any stagnation state from 0.2 to 100 keV with a shell denser than its hot spot, with the same equations.",
        fitted_on: "Nothing fitted: every coefficient is the published one, and the numerics were set on generic hot spots with no NIF shot run.",
        compared: "Compared with the seven shots as published (targets: yield within a factor of 2, burn width within 30 percent, burn-up within a factor of 2, yield amplification within a factor of 1.5): 3 of 24 targets reached and no shot within all of its own; N221204 yield 2.32 MJ against 3.1 and burn-up 3.1 percent against 4.33, each within its factor of 2, burn width 113 ps against 75; the four burning-plasma shots ignite, 4.3 to 6.1 MJ against 0.10 to 0.17 MJ measured.",
        gap: "Gap: no reading reproduces both the yield and the burn width of the ignition shots; with the stagnated mass as fuel, N221204 yields 2.19 to 2.97 MJ against 3.1 and burns 2.9 to 4.0 percent of its fuel against 4.33, in 28 ps against 75.",
        refuses: "Refuses a temperature outside 0.2 to 100 keV at the door (TemperatureOutsideReactivityFit) or during the burn (TemperatureLeftReactivityFit), a degenerate hot spot (StagnationDegenerate, HotSpotDegenerate), a geometry the fuel cannot fill (FuelMassNotAboveHotSpotMass, ShellMassBelowArealDensity), and a shell no denser than its hot spot (ShellNotDenserThanHotSpot), which turns away each of the seven shots as published.",
    },
    SpecSheet {
        model: "ablator_slab",
        computes: "A flat slab of charring heat-shield material heated on one face: wear, face temperature, depth of charring and the heat and mass accounts, for a PICA model built from the open record or the test material TACOT 3.0.",
        range: "Range: PICA compared in arc-jet tests at 107 to 1,102 W/cm² and 2.3 to 84 kPa, on the Stardust entry and in thermocouple blocks; computes slabs of 0.2 mm to 1 m under any heating with the same equations.",
        fitted_on: "Fitted on 259 thermocouple samples of four arc-jet blocks: one number, a char conductivity scale of 0.71; the surface rules were set on 36 arc-jet tests.",
        compared: "Compared with 31 arc-jet tests outside the fit: wear within its target on 28 of 31 (target 25), mean −2.6 percent; hottest face within 7 percent on 24 of 28 (target 23); Stardust wear 7.58 and 3.32 mm against 5.7 and 3.2.",
        gap: "Gap: the face runs 4.1 percent hot on average (target 3.5), and Stardust nose wear is a third high.",
        refuses: "Refuses a start outside 260 to 800 K (StartTemperatureOutOfRange), a temperature above the material's tables (TemperatureOutOfBounds), a face that boils (FaceBeyondSurfaceTable) and a step that does not settle (StepNotConverged).",
    },
    // <<< sheets: solvers one >>>
    SpecSheet {
        model: "journal_bearing_sheffield",
        computes: "Oil film of the University of Sheffield journal bearing test platform at 65 °C, by the Reynolds equation for a rigid aligned shaft and bush under a steady load and speed: Sommerfeld number, eccentricity ratio, attitude angle, minimum film thickness and peak film pressure.",
        range: "Range: compared at 2 to 20 kN and 100 to 800 rpm (Sommerfeld number 0.016 to 1.30); computes any load and speed above zero with the same equations while the film is laminar and carries the load at an eccentricity ratio of 0.05 to 0.95, the oil held at 65 °C.",
        fitted_on: "Fitted on 34 of the 50 runs of Mendeley Data doi 10.17632/txdrvd382p.2: one constant, a multiplier on the film's viscosity.",
        compared: "Compared with the 16 runs not in the fit (every third run, inside the same loads and speeds): minimum film thickness within 0.058 mean absolute relative residual, worst 0.156, mean +0.020.",
        gap: "Gap: on the 34 fitted runs, each left out of the fit in turn, the worst run is 0.324 off (mean absolute 0.062); the attitude angle and the peak film pressure come from the same settled film and have no measurement.",
        refuses: "Refuses a load or a speed of zero or less (LoadNotPositive, SpeedNotPositive), a film Reynolds number above 41.2 sqrt(R/c), about 1450, near 143,000 rpm (FilmNotLaminar), and a load that would put the shaft outside an eccentricity ratio of 0.05 to 0.95 (EccentricityRatioOutOfRange).",
    },
    SpecSheet {
        model: "junction_nist_b7_s2",
        computes: "Current-voltage characteristic of the NIST niobium / amorphous-silicon / niobium Josephson junction D220831b-B7_S2_1.5 at 4 K, the shunted junction with its capacitance and its own measured dissipative curve stepped in time: the mean voltage point by point from rest or from the voltage state, and its Shapiro steps under a microwave drive below the gap frequency, 691.6 GHz.",
        range: "Range: compared at a bias of 0 to 0.91847 mA on both sweeps with no drive; computes 1 to 36 points of any bias, and drives above zero and below 691.6 GHz, with the same equations within one solve's budget of 120,000,000 steps (33.456 mA for one undriven point), the energy ledger held at every step.",
        fitted_on: "Fitted on the junction's field-suppressed curve, its stated critical current of 0.298 mA and two bias currents of its zero-field curve: one constant, the capacitance, fitted to the return bracket 0.05567 to 0.06494 mA; the field-suppressed curve is the dissipative table itself.",
        compared: "Compared with the zero-field curve outside the fit, 100 points rising and 100 falling, 0 to 0.91847 mA: voltage within 11.84 percent of I_c R_n RMS over the 175 points on the same branch (rising 7.10, falling 15.13, worst -36.3 at 0.1113 mA falling); switching current 0.30616 mA and return current 0.06494 mA each within one bias step.",
        gap: "Gap: the measured plateau is flat at 2.8498 to 2.8651 mV from 0.33 to 0.57 mA and the solver's rises from 2.826 to 3.096 mV, up to 19.5 percent high; no driven curve has a measurement.",
        refuses: "Refuses a drive at or above the gap frequency, 691.6 GHz, with an amplitude not zero (DriveAtOrAboveGapFrequency), a drive frequency of zero or less (DriveFrequencyNotPositive), a question with no bias (NoBiasPoints) and one past 120,000,000 steps (StepsBeyondSolveBudget, a budget of computing); a point with no steady mean is refused as NotSettled.",
    },
    SpecSheet {
        model: "ln2_line_dlr_fttf",
        computes: "Fluid hammer in the liquid-nitrogen line of DLR's Fluid Transient Test Facility, Lampoldshausen (9.29 m of 1.4541 tube, 19 x 1.5 mm, in a vacuum jacket), when its fast valve closes on a steady flow: pressure at the stations S1, S2 and S3, with the liquid's stiffening under pressure, Churchill's friction, the valve's measured stroke and vapour cavities, the energy and mass ledgers closed at every step.",
        range: "Range: compared at 82.7 to 86.4 K, 3.5 to 21.4 bar gauge at S1 and 1.06 to 2.82 kg/s; computes liquid from 68.5614 to 109.909 K, any flow up to a tenth of the wave speed and any pressure up to the bore's first yield, 25.41 MPa, with the same equations, for up to 10 s.",
        fitted_on: "Fitted on 3 of the 9 DLR tests of Zenodo 15526459: three constants, the roughness (12.53 um) and the valve's seat (0.04 of the stroke) and series ratio (2.0).",
        compared: "Compared with the 6 tests not in the fit on eleven error measures: in all six, the first rise at S2 within 1.7 percent, the front's speed from S1 to S2 within 0.7 percent, the second surge's time within 8.1 percent and the steady drop from S3 to S1 within 9.3 percent; the first rise at S1 4.8 to 6.8 percent high.",
        gap: "Gap: the plateau at S1 is 4.2 to 7.4 percent short, the first rise at S3 2.6 to 14.9 percent low, the second surge 6.9 to 22.6 percent high (39 and 111 percent at the two lowest pressures), and the late ringing after cavitation 10.0 to 13.1 percent short in period with a decay rate 78 to 91 percent under the measured.",
        refuses: "Refuses liquid below 68.5614 K, which can freeze at a pressure the tube holds (LiquidFreezesBelowWallYield), and above 109.909 K, where the vapour is more than a tenth as dense as the liquid (VapourNotLightAgainstLiquid), a supply above the bore's first yield at 25.41 MPa (SupplyAboveWallYield) and a stroke faster than ten grid steps (StrokeFasterThanGrid); a run stops where the pressure reaches the first yield (WallYield).",
    },
    SpecSheet {
        model: "modulus_bam_s355",
        computes: "Young's modulus and shear modulus of one of four test bars of S355J2+N structural steel, about 100 x 9 x 3 mm, cut from one hot-rolled sheet and rung at 24 °C at BAM Berlin, worked back from the bar's measured flatwise and torsion resonant frequencies by a Timoshenko beam and Saint-Venant torsion, with the edgewise frequency they predict.",
        range: "Range: compared at flatwise 1587.428 to 1608.28 Hz, torsion 8899.446 to 8966.189 Hz and edgewise 4695.035 to 4755.856 Hz, where the answers run 210.7 to 216.3 GPa in E and 79.0 to 80.8 GPa in G; computes any frequency above zero at which an isotropic bar of that size rings, E from 1 GPa to 2 TPa and G from 0.3 GPa to 1 TPa, with the same equations.",
        fitted_on: "Nothing fitted: the solver holds no property of steel, the two moduli are solved from each scan's two frequencies, and the model was chosen on bars ERX1 and ERX2, along the rolling direction, of doi 10.5281/zenodo.6778336.",
        compared: "Compared with bars ERY1 and ERY2, across the rolling direction, not used to choose the model: Young's modulus 215.68 GPa against 210.25 GPa from the tensile slope, 2.58 percent apart (target 2.49); the edgewise frequency, never used to find the moduli, within 0.114 percent on all four scans (target 0.25); E and G within 0.053 percent of the laboratory's own reduction of the same frequencies.",
        gap: "Gap: against the tensile slope Young's modulus reads 2.58 percent high across the rolling direction and 7.4 percent high along it (211.6 against 197.0 GPa), where the slope itself moves by up to 8 percent within one tensile test; the shear modulus has no second measurement.",
        refuses: "Refuses a frequency of zero or less (FrequencyNotPositive), a series without a flatwise or a torsion frequency (YoungsModulusNotIdentifiable, ShearModulusNotIdentifiable), a pair at which no isotropic bar rings, E / (2 G) - 1 outside 0 to 0.5 (PoissonRatioOutOfRange), and moduli outside 1 GPa to 2 TPa in E or 0.3 GPa to 1 TPa in G (OutsideSearchRange).",
    },
    SpecSheet {
        model: "plasma_sheath_ram_c2",
        computes: "The plasma sheath around NASA's RAM C-II re-entry probe (a 15.24 cm hemisphere on a 9 degree cone) at one flight point: the equilibrium normal shock and stagnation point of 11-species air, the gas carried along the body to the nose station (x/D 0.15) and the cone station (x/D 0.76) with Dunn and Kang's 26 reactions at finite rate, the electron density, the plasma frequency, and whether a radio signal of the asked frequency is blacked out at each.",
        range: "Range: compared at 7.65 km/s from 70 to 80 km; computes 0 to 80,637 m and any flight speed above Mach 1 up to 20,000 m/s with the same equations, and flags every answer from 73.6 km up, where the shock is as thick as the layer (shock_as_thick_as_layer).",
        fitted_on: "Nothing fitted: Dunn and Kang's reactions and rates as printed in NASA CR-2232, the 1962 standard atmosphere and the NASA Glenn fits.",
        compared: "Compared with the reflectometer onsets of NASA TN D-6062 Table X at 7.65 km/s: electron density 0.94 of the measured, within a factor of 1.07, at the cone station's X band at 70.0 km; 3.28 and 2.92 times the measured at 77.7 and 80.0 km.",
        gap: "Gap: above 73.6 km, where the shock is as thick as the layer and electrons are lost to the cold wall, the density reads about 3 times high (3.28 at 77.7 km, 2.92 at 80.0 km).",
        refuses: "Refuses an altitude above 80,637 m, where the Knudsen number reaches 0.03 (RarefiedFlow), or below sea level (AltitudeBelowSeaLevel), a speed of Mach 1 or less (NoBowShock) or above 20,000 m/s (VelocityPastFits), and a gas behind the shock above 20,000 K (TemperatureAboveFits).",
    },
    SpecSheet {
        model: "propeller_apc_10x7e",
        computes: "Thrust and shaft power of the APC Thin Electric 10x7 propeller (two blades, 10 inch) at one rotational speed, airspeed and air density, by blade-element momentum theory on the chord and blade angle measured at 18 stations of this propeller, with one blade section and Prandtl's tip loss.",
        range: "Range: compared at 4007 to 6531 rev/min and advance ratio 0.111 to 0.748 with the air moving, and 4144 to 6542 rev/min at rest; computes any speed and density above zero and any airspeed from zero with the same equations, while the chord Reynolds number stays inside 44399 to 75516 and the tip Mach number at or below 0.3.",
        fitted_on: "Fitted on 43 points of the 3 sweeps at 4007 to 5018 rev/min of the UIUC Propeller Data Site, volume 1: two blade-section constants, the zero-lift angle (-2.77 degrees) and the minimum drag coefficient (0.027).",
        compared: "Compared with the 64 points of the 4 sweeps at 6015 to 6531 rev/min, not in the fit: within 4.85 percent of full scale on average (target 4.0), worst 10.16 (target 10.0); at rest, 9 rows at 4144 to 6542 rev/min, within 6.02 on average (target 4.0), worst 11.76 (target 10.0).",
        gap: "Gap: the answer reads low at every point outside the fit, by 5.0 percent of full scale in thrust coefficient and 4.7 in power coefficient on average, because the measured coefficients rise with speed and the model's do not; at rest the shaft power reads about 9 percent of full scale low.",
        refuses: "Refuses a chord Reynolds number outside 44399 to 75516 (ReynoldsNumberOutOfRange), a tip Mach number above 0.3 (TipMachAboveIncompressible), a speed or a density of zero or less (RotationalSpeedNotPositive, AirDensityNotPositive) and a negative airspeed (AirspeedNegative).",
    },
    SpecSheet {
        model: "seawater_column",
        computes: "Depth, density and sound speed at every level of one measured seawater cast (pressure, temperature, salinity), from the TEOS-10 Gibbs function and the weight of the water above, and the depth an echo sounder reads over it.",
        range: "Range: compared on 632 casts of the WOCE Indian Ocean expedition of the Knorr, cruise 145, at latitudes -34.0023 to +26.4108, down to 6108 dbar, 0.5088 to 31.169 degC and Practical Salinity 30.9805 to 40.0731; computes any latitude, 0 to 10000 dbar, Practical Salinity 0 to 41.8 and the freezing temperature up to 40 degC with the same equations.",
        fitted_on: "Nothing fitted: the density and the sound speed are the TEOS-10 reference equations, and gravity a formula.",
        compared: "Compared with the ship's echo sounder at 632 stations of 5 legs, none used to set the comparison: depth within a median of +4.59 m (target 10), 0.804 of stations within 20 m (target 0.80), each leg's median +4.30 to +4.92 m (target 15).",
        gap: "Gap: the solver's depth reads deeper than the sounder's, about +3 m on flat sea floor and +4.6 m pooled, and the fraction within 20 m is two stations over its target, 0.789 to 0.798 with leg 9, 10 or 13 left out.",
        refuses: "Refuses seawater below its freezing temperature (TemperatureBelowFreezing), a pressure outside 0 to 10000 dbar (PressureOutOfRange), Practical Salinity outside 0 to 41.8 (SalinityOutOfRange), a temperature above 40 degC (TemperatureOutOfRange) and a last level more than 100 m above the sea floor (HeightAboveBottomOutOfRange).",
    },
    SpecSheet {
        model: "step_flow_armaly",
        computes: "Steady laminar air flow over the backward-facing step of the Armaly rig (step 4.9 mm, channel 5.2 mm opening to 10.1 mm, expansion ratio 1.94), by the two-dimensional Navier-Stokes equations: the reattachment length and the velocity profile at one station.",
        range: "Range: velocity profiles compared at Reynolds number 100 (stations 0 to 7.76 and 12.04 to 30.31 step heights) and 389 (stations 0 to 13.57), the reattachment length at 97 to 389; computes any Reynolds number above 0 to 400 and any station out to 30 step heights with the same equations.",
        fitted_on: "Nothing fitted: the rig's geometry and the paper's own Reynolds number, Re = VD/ν with V two-thirds of the maximum inlet speed and D twice the inlet height.",
        compared: "Compared with 174 digitised circles at Reynolds number 389, none read when the solver was built: velocity profile within 0.0437 of the largest inlet speed RMS (target 0.05), worst station 0.052 (target 0.08); at 100, on 80 circles of Armaly's Figure 15 at 12.04 to 30.31 step heights, within 0.019 RMS.",
        gap: "Gap: the reattachment length reads 6 to 7 percent short of Armaly's plot at 389 (8.07 against about 8.6 step heights) and 5.8 percent short at 97, agreeing within 1 percent near Reynolds number 300; no velocity profile between 100 and 389 has a measurement.",
        refuses: "Refuses a Reynolds number of 0 or less (ReynoldsNotPositive) or above 400, where the flow over this step turns three-dimensional (ReynoldsOutOfRange), and a station outside 0 to 30 step heights, where the grid ends (StationOutOfRange).",
    },
    SpecSheet {
        model: "stress_plate_hole_s235",
        computes: "The in-plane stress and displacement field of an S235 steel tensile bar with a 4 mm hole, the specimen of the Clausthal digital image correlation record (parallel length 20.05 by 3.05 mm, d/W 0.399), pulled on its gross section: plane stress with von Mises yield at 235 MPa, settled on finite-element meshes of rising refinement, stress concentration 2.249 on the net section.",
        range: "Range: compared at 106.26 MPa, the record's one load, over the ring 4.5 to 12 mm from the hole's centre; computes any traction above 0 up to 139.81 MPa with the same equations, the steel yielding from 63.49 MPa.",
        fitted_on: "Fitted on the 3,269 points 16 mm or more from the hole of Zenodo 10.5281/zenodo.11257192: two constants, Young's modulus 218,540 MPa and Poisson's ratio 0.2835.",
        compared: "Compared with the 1,186 points 4.5 to 12 mm from the hole, not in the fit: displacement within 0.675 um RMS in u_x (target 2.0) and 0.444 um in u_y (target 1.5), 0.571 um pooled against 1.015 for Kirsch's infinite plate; the strain 1.5 mm from the hole's edge 2.380 against 1.889 measured, the solver 26.0 percent above the measured, the measured 20.6 percent below the solver's (target 20).",
        gap: "Gap: the solver yields a zone 1.53 mm deep at the hole, absent from the measured strain, so the strain 1.5 mm from the edge reads 26.0 percent high; the same field with the steel not yielding comes within 1.4 percent (stress_plate_hole_s235_elastic).",
        refuses: "Refuses a traction of 0 or less (TractionNotTensile) or above 235 MPa (TractionAboveYield), a strain past 1 percent, from 139.99 MPa (StrainBeyondSmallStrain), meshes that do not settle, from 139.82 to 139.98 MPa (NotSettled), and a point outside -45 to 45 mm along the bar (PointOutsidePlate).",
    },
    SpecSheet {
        model: "stress_plate_hole_s235_elastic",
        computes: "The in-plane stress and displacement field of the same S235 tensile bar with a 4 mm hole as stress_plate_hole_s235 (parallel length 20.05 by 3.05 mm, d/W 0.399), with the steel not yielding: plane stress and Hooke's law, settled on the same finite-element meshes, stress concentration 2.249 on the net section.",
        range: "Range: compared at 106.26 MPa, the record's one load, over the ring 4.5 to 12 mm from the hole's centre; computes any traction above 0 up to 590.04 MPa with the same equations, and flags every answer above 106.26 MPa (past_compared_traction).",
        fitted_on: "Nothing fitted here: Young's modulus 218,540 MPa and Poisson's ratio 0.2835 are stress_plate_hole_s235's, fitted on its 3,269 points 16 mm or more from the hole, and the structure, no yield, was chosen with stress_plate_hole_s235's whole comparison read.",
        compared: "Compared with the same 1,186 points 4.5 to 12 mm from the hole: displacement within 0.638 um RMS in u_x (target 2.0) and 0.486 um in u_y (target 1.5), 0.567 um pooled against 1.015 for Kirsch's infinite plate; the strain 1.5 mm from the hole's edge 1.917 against 1.889 measured, the measured 1.4 percent below (target 20).",
        gap: "Gap: the record gives no yield strength of this sheet, and the elastic stress at the hole's edge, 397.6 MPa at 106.26 MPa, has no measurement, the ring read starting 0.5 mm from the edge; past the sheet's yield strength this field is not that steel's.",
        refuses: "Refuses a traction of 0 or less (TractionNotTensile), a strain past 1 percent, from 590.05 MPa (StrainBeyondSmallStrain), and a point outside -45 to 45 mm along the bar (PointOutsidePlate) or inside the hole (PointInsideHole).",
    },
    SpecSheet {
        model: "fuel_rod_triga_jsi",
        computes: "Temperature of the instrumented fuel element 6753 of the JSI TRIGA Mark II reactor through a pulse, from the core's energy release given as a history: radial conduction through the zirconium rod, the U-ZrH1.6 meat, a gas gap and the SS-304 clad by backward Euler in the enthalpy, 1.387/56 of the core's energy into the meat, convection and boiling to the pool, and the fuel thermocouple as a first-order lag, every step's energy account closed to 1e-9.",
        range: "Range: compared from starts of 293.14 to 305.53 K and core energy releases of 6.85 to 19.41 MW s, core power to 994 MW, histories in 1 ms and 10 ms intervals to 14 s; computes any start of liquid pool water from 273.15 to 385 K and any history up to 600 s with the same equations, each answer outside the compared span carrying one of four *_outside_compared keys.",
        fitted_on: "Fitted on 39 pulses of cores 215, 217 and 231, 1.4 to 3.0 printed dollars: two constants, the gap conductance, 1780 W/m2 K, and the thermocouple's time constant, 1.03 s.",
        compared: "Compared with 16 pulses of cores 217 and 231 at 2.0 to 2.6 printed dollars not in the fit, from each record's own power: the thermocouple's maximum rise within 9.2 percent on all 16 (9.2 low to 0.3 high, mean 5.5; target 10), its time after the power peak within 19.6 percent (mean 11.9; target 20), and the rise 10 s after the peak within 4.3 percent on the 8 records that run that long (mean 3.1; target 10).",
        gap: "Gap: on the maximum rise the lumped core-average baseline is closer, mean 4.0 percent against 5.5; with one gap conductance the 2015 core-217 pulses read 8.7 to 9.2 percent low and the 2018 core-231 pulses 0.3 high to 3.0 low; fed by the reactor_triga_jsi card through the coupling, the 10 s rise reads 11.2 to 5.0 percent low.",
        refuses: "Refuses a start outside a liquid pool (StartTemperatureOutsideLiquidPool), times not rising (HistoryTimesNotRising), energy released falling (EnergyFalls), the meat above 1150 degC (FuelAboveSafetyLimit), a clad heat flux above Zuber's critical heat flux, 1.47 MW/m2 (CriticalHeatFluxPassed), and a history past 600 s, the capacity of one solve (HistoryBeyondCapacity).",
    },
    // <<< sheets: solvers two >>>
];

#[cfg(test)]
mod tests {
    use super::*;

    /// A spec sheet's shape: each part opens as its place says, the range,
    /// the comparison and the gap carry numbers, the range is a span in
    /// numbers ("a to b"), and the refusals name at least one name.
    pub(crate) fn shape(sheet: &SpecSheet) -> Vec<String> {
        let mut wrong = Vec::new();
        let digit = |text: &str| text.chars().any(|c| c.is_ascii_digit());
        let opens = |text: &str, words: &[&str]| words.iter().any(|word| text.starts_with(word));
        let name = sheet.model;
        if sheet.computes.is_empty() || !sheet.computes.ends_with('.') {
            wrong.push(format!("{name}: computes is not a sentence"));
        }
        if !opens(sheet.range, &["Range: "]) || !crate::spec_sheet::span_in_numbers(sheet.range) {
            wrong.push(format!("{name}: range does not open \"Range: \" with a span in numbers"));
        }
        if !opens(sheet.fitted_on, &["Fitted on ", "Nothing fitted"]) {
            wrong.push(format!("{name}: fitted_on does not open \"Fitted on \" or \"Nothing fitted\""));
        }
        if !opens(sheet.compared, &["Compared with "]) || !digit(sheet.compared) || !sheet.compared.contains("within ") {
            wrong.push(format!("{name}: compared does not open \"Compared with \", with a number and \"within \""));
        }
        if !opens(sheet.gap, &["Gap: "]) || !digit(sheet.gap) {
            wrong.push(format!("{name}: gap does not open \"Gap: \" with a number"));
        }
        if !opens(sheet.refuses, &["Refuses "]) || !crate::spec_sheet::names_in(sheet.refuses).next().is_some() {
            wrong.push(format!("{name}: refuses does not open \"Refuses \" with a refusal by name"));
        }
        for part in sheet.parts() {
            for word in crate::spec_sheet::UNSAID {
                if crate::spec_sheet::says(part, word) {
                    wrong.push(format!("{name}: says \"{word}\": {part}"));
                }
            }
        }
        wrong
    }

    #[test]
    fn every_spec_sheet_has_the_shape() {
        let wrong: Vec<String> = SHEETS.iter().flat_map(shape).collect();
        assert!(wrong.is_empty(), "{wrong:#?}");
    }

    #[test]
    fn one_sheet_a_model_each_served() {
        for (index, sheet) in SHEETS.iter().enumerate() {
            assert!(SHEETS[..index].iter().all(|other| other.model != sheet.model), "{} twice", sheet.model);
            let served = crate::descriptor::all().iter().any(|card| card.card == sheet.model) || crate::solvers::find(sheet.model).is_some();
            assert!(served, "{} is not served", sheet.model);
        }
    }

    #[test]
    fn the_words_are_three() {
        assert_eq!(WORDS, ["compared with measurement", "held to a reference calculation", "company specimen"]);
        for word in WORDS {
            assert!(AGAINST_MEASUREMENT_SAYS.contains(&format!("\"{word}\"")), "{word}");
        }
    }
}

/// Words no served sentence of a model's evidence says.
pub const UNSAID: &[&str] = &[
    "blind", "pre-regist", "registered", "registration", "pass mark", "pass line", "pass_line", "fixed before", "beforehand",
    "amendment", "held-back", "held back", "scored", "scoring", "verdict", "not a test", "calibrated to the record", "met in part",
    "met", "missed", "passed", "failed", "research record", "gated",
];

/// Whether `text` says `word` as a word of its own, ignoring case.
pub fn says(text: &str, word: &str) -> bool {
    let lower = text.to_lowercase();
    let mut from = 0;
    while let Some(at) = lower[from..].find(word) {
        let at = from + at;
        let before = lower[..at].chars().next_back();
        let after = lower[at + word.len()..].chars().next();
        if !before.is_some_and(char::is_alphanumeric) && !after.is_some_and(char::is_alphanumeric) {
            return true;
        }
        from = at + word.len();
    }
    false
}

/// A span in plain numbers: a digit, " to " and a digit within a few characters.
pub fn span_in_numbers(text: &str) -> bool {
    let bytes = text.as_bytes();
    let mut from = 0;
    while let Some(at) = text[from..].find(" to ") {
        let at = from + at;
        let mut start = at.saturating_sub(14);
        while !text.is_char_boundary(start) {
            start -= 1;
        }
        let before = &text[start..at];
        let after = &bytes[at + 4..(at + 7).min(bytes.len())];
        if before.chars().any(|c| c.is_ascii_digit()) && after.iter().any(u8::is_ascii_digit) {
            return true;
        }
        from = at + 4;
    }
    false
}

/// The refusal names a sentence gives: words in CamelCase of two capitals or more.
pub fn names_in(text: &str) -> impl Iterator<Item = &str> {
    text.split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|word| word.len() > 3 && word.starts_with(|c: char| c.is_ascii_uppercase()) && word.chars().filter(char::is_ascii_uppercase).count() >= 2 && word.chars().any(|c| c.is_ascii_lowercase()))
}
