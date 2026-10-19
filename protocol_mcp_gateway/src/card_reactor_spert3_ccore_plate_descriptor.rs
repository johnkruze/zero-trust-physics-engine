//! What an agent reads of the reactor_spert3_ccore_plate card: its fields and bounds, its step
//! floor and ceiling, its refusals word for word from the card's source, its frame, its worked
//! examples and its record. Stamped from the kit; a child module of `descriptor.rs`, so it uses
//! that file's own builders.
//!
//! The card is the twin of reactor_spert3_ccore, fitted on the same record. Its description leads
//! with what it computes and its numbers against IDO-16918; the result opens with the words the
//! kit holds a calibrated model to, naming reactor_spert3_ccore.

use genesis_kernel::reactor_spert3_ccore_plate;
use serde_json::Value;

use super::{Advisory, Card, Role, finish, obj, pair_examples_recorded, scalar, scalar_value, slots};

const DOMAIN: &str = "energy";
const CARD: &str = "reactor_spert3_ccore";

/// What the card is, in one sentence of the list.
const SPECIMEN: &str = "The SPERT III C-core test reactor (stainless-steel-clad, highly enriched plates) in step transients from room temperature at 0 to 2500 psig and 0 to 20,000 gpm, with subcooled boiling and void, each plate as its meat and its cladding, the plates' feedback lagging their temperature and the film next to them thinning with the flow: peak power, energy to peak, time to peak and the hot-spot plate temperature through its first maximum. With each tenth of the 111 transients of IDO-16918 left out of the fit, peak power within 10.4 percent RMS, energy to peak 17.2, the hot-spot plate rise 12.3 at the power peak and 11.3 at its maximum.";

/// The object.
const WHAT: &str = "The SPERT III reactor at the National Reactor Testing Station, Idaho, with the C-19/52 core: 93 percent enriched UO2-stainless-steel plates, a 0.020 in meat clad in 0.005 in of type 304L stainless steel on each side, 0.030 in in all, with 0.128 in water channels, 36 in active length. Step insertions of reactivity from room temperature at system pressures of 0 to 2500 psig with no forced flow, and at 230 psig with 200 to 20,000 gpm. Point kinetics with six delayed groups, and three plate-and-water nodes: the core average, a feedback node at a fitted peaking of 2.44 (the core's temperatures as its reactivity weighs them) and the hot spot at 2.04 times 1.30, where the thermocouples sat. Each node carries the meat of the plate, where the fission heat goes, its cladding (a third of the plate's heat capacity), joined to the meat by conduction at 108,600 W/m2 K from the plate's thicknesses and conductivities, a film of the water next to the plates, the bulk water and the vapour in its channels. The cladding passes heat to the film by conduction and, above the saturation temperature at the system pressure, by subcooled boiling (Thom's nucleate-boiling law, capped smoothly by Zuber's critical heat flux with the Ivey-Morris subcooling term); a small share of the boiling heat stays as vapour, which condenses in 3 ms and is swept out with the flow. The film's share of the water, 0.2385 with no flow, thins as the flow's Dittus-Boelter coefficient passes the plate's, to 0.0043 at 20,000 gpm; where a segment changes the flow the film gives water to the bulk or takes it, the heat held unchanged. The reactivity takes back the water's temperature (the measured isothermal coefficient, 1.8 cents per degC rising to 16 cents per degF at 650 degF, on the film and the bulk), the plates' (the water they displace as they expand, and their own Doppler and bowing coefficient, on the plate's mean temperature, followed with a fitted lag of 5.69 ms) and the void (45 cents per percent). The water and steam properties at saturation are IAPWS-IF97 region 4 and the IAPWS auxiliary equations of Wagner and Pruss, computed in the engine's own math, from the triple point to the critical point.";

/// The data.
const DATA: &str = "IDO-16918, C. R. Toole, The Effects of Pressure and Flow on Room Temperature Power Excursions in Spert III, Phillips Petroleum Co. for the US Atomic Energy Commission, June 1964, OSTI 4042264: its appendix tabulates 126 step transients with pressure, initial temperature, flow, period, peak power, energy to the time of peak power, and the plate surface temperature at that time and at its maximum, read by thermocouples spot-welded to a plate at the flux peak of a central assembly. Of the transcription, 111 rows are used: 15 rows are set aside for an abnormal thermocouple, a likely misprint or a missing or doubtful initial temperature. Of the 111, peak power and energy are on 105 rows, the plate temperature at peak power on 104 and its maximum on 95, the rest carrying the report's marks for a value still rising or higher at the scram. Core constants from IDO-17138 (Condit, Scott and Johnson, 1967), Table B-1, and IDO-16918 Table I. No measured value of either report is stored in the card.";

/// How the card is compared.
const COMPARED: &str = "Ten constants fitted by least squares on the logarithms of card over measured, all four quantities and all 111 rows together: the plate-to-film and film-to-bulk conductances with no flow (2295.18 and 95.6826 W/m2 K), the film's share of the water with no flow (0.238546) and its exponent with flow (-1.80032), the feedback node's peaking (2.44405), the plate's coefficient beyond the water it displaces (0.000500615 dollars per K), the lag of the plate's feedback (5.68852 ms), the Thom and Zuber multipliers (1.30438 and 2.16680) and the share of boiling heat that stays as vapour (0.000371141). Fixed: the share of fission energy the water takes directly, 0.026, the condensation time, 3 ms, the cladding's share of the plate, one third, and the meat-to-cladding conductance, 108,600 W/m2 K. For the plate maximum the card's first maximum after the power peak is compared, or its value 1.88020 s after the peak where that comes first (a fitted eleventh number of the comparison: the record gives no scram time). Then each tenth of the 111 rows, every tenth in printed order, left out in turn, the constants fitted on the rest and the left-out tenth computed with them. Conduction within the plate and a lag of the thermocouples at the hot spot were each tried and read the same (sums of squares 6.451 and 6.449 with the rest): the card carries conduction, whose conductance comes from the plate and is not fitted. The error is card over measured less one, in percent; temperatures are rises above the initial temperature.";

/// The result, its first sentence the word it is served under.
const RESULT: &str = "Fitted on all 111 transients of IDO-16918, every transient read when the structure was chosen. In the fit, on the 111 transients, run at 0.1 ms steps: peak power 10.2 percent RMS, worst 57 (105 transients, 74 percent of them within the report's power calibration of about 10 percent); energy to peak 16.6 percent, worst 109; the hot-spot plate rise at peak power 11.9 percent, worst 60 (104); at its first maximum 10.6 percent, worst 56 (95). Compared with transients left out of the fit (10-fold): 10.4, 17.2, 12.3 and 11.3 percent RMS. On 10 transients not in the fit, whose temperatures the report marks abnormal: peak power 14.5 and energy 12.7 percent RMS. By band, left out, peak power, energy, rise at peak and plate maximum: 0 psig (29 transients) 8.7, 13.9, 9.3, 7.8; 20 to 110 psig (22) 9.7, 25.8, 6.7, 4.5; 175 to 505 psig (21) 8.0, 10.4, 12.4, 13.1; 1000 to 2500 psig (14) 8.0, 11.9, 10.0, 10.4; 200 to 2040 gpm (9; 3 in plate maximum) 10.3, 22.9, 18.5, 13.6; 5000 to 20,000 gpm (10) 19.6, 13.4, 21.3, 24.6; periods under 15 ms (31) 7.7, 12.4, 15.5, 11.6; 15 to 50 ms (44) 12.1, 19.8, 7.6, 12.6; 50 to 200 ms (18) 8.9, 12.3, 10.3, 6.7; 200 ms and longer (12) 11.5, 22.6, 17.9, 9.5; at 1000 to 2500 psig and 11.5 ms or less (7) 8.6, 10.7, 9.1, 14.5. The record against itself, repeated transients at one pressure and flow with periods within 3 percent: 8.9, 13.7, 8.5 and 5.5 percent RMS. At about 11 ms the card's plates take back 0.134 dollar at the power peak at 0 psig and 0.227 above 1000 psig, the void 6.6 cents at 0 psig and none above 1000 psig. Record SHA-256 757918a318fd4292d9bad8b533990c51b65669ce520c699a25f772ec69adaec1, with its table of numbers SHA-256 73568cdaaa9bcafc60e5a7540dc5ae189344614305e98720aceb0ff8bbcc0f41.";

/// Where the numbers stop.
const LIMITS: &str = "One core, from room temperature: compared on initial temperatures of 21 to 36 degC, pressures of 0 to 2500 psig with no flow, flows of 200 to 20,000 gpm (0.054864 to 5.4864 m/s) at 230 psig only, and reactivities from the inhour value of the longest compared period to that of the shortest, 0.412287 to 1.30540 dollars with no flow and 0.870348 to 1.30540 with flow. A start or a load outside that span is computed by the same equations and every frame carries FLAG_OUTSIDE_COMPARED_RANGE. IDO-16918 tabulates no time to peak: the card's, 0.175 to 103 s over the compared transients from its 10 W start, is not compared. The meat and the cladding are one temperature each across their thicknesses; the water's own heat capacity and its Dittus-Boelter properties are those of 25 degC. Thom's law was fitted by its authors at 5.2 to 14 MPa and is used here from 0.1 to 17.3 MPa with its multiplier fitted. The card refuses by name a pressure outside the steam tables (PressureOutsideSteamTables: below the triple point or at or above the critical pressure, where water has no saturation temperature), a start that is not liquid water (InitialTemperatureNotLiquidWater), a void fraction of one or more (VoidFractionAboveOne), and a step longer than the state allows (DtAboveStateCeiling). At 5000 to 20,000 gpm the plate maximum reads 24.6 percent RMS left out, worst 57 at transient 03618 (108 MW printed where 02911 at the same flow printed 58); without it 12.9. The isothermal coefficient continues past 650 degF along its last measured segment.";

pub(super) fn card() -> Card {
    let reactor = reactor_spert3_ccore_plate::ReactorSpert3CcorePlate::c_core_plate();
    let gate = reactor.gate_dt_s().expect("specimen gate");
    let floor = reactor.floor_dt_s().expect("specimen floor");
    let triple_psig = reactor_spert3_ccore_plate::TRIPLE_POINT_PRESSURE_MPA / reactor_spert3_ccore_plate::MPA_PER_PSI
        - reactor_spert3_ccore_plate::ATMOSPHERE_PSI;
    let critical_psig = reactor_spert3_ccore_plate::CRITICAL_PRESSURE_MPA / reactor_spert3_ccore_plate::MPA_PER_PSI
        - reactor_spert3_ccore_plate::ATMOSPHERE_PSI;
    finish(
        DOMAIN,
        CARD,
        SPECIMEN,
        vec![
            scalar(
                "initial_temperature_k",
                Role::Phase,
                "K",
                Some(reactor_spert3_ccore_plate::FREEZING_K),
                false,
                Some(reactor_spert3_ccore_plate::CRITICAL_TEMPERATURE_K),
                true,
                format!(
                    "the temperature of plates and water at the step: liquid water, from {} K up to the saturation temperature of water at system_pressure_psig (IAPWS-IF97 region 4: 373.12 K at 0 psig, 627.06 K at 2500 psig), below the critical temperature, {} K. A start that is not liquid water is refused as InitialTemperatureNotLiquidWater. Compared on starts of 21 to 36 degC; outside them every frame carries FLAG_OUTSIDE_COMPARED_RANGE",
                    super::token(reactor_spert3_ccore_plate::FREEZING_K),
                    super::token(reactor_spert3_ccore_plate::CRITICAL_TEMPERATURE_K),
                ),
            ),
            scalar(
                "system_pressure_psig",
                Role::Phase,
                "psig",
                Some(triple_psig),
                false,
                Some(critical_psig),
                true,
                format!(
                    "the system pressure in pounds per square inch above the atmosphere, from {} psig, the triple point of water, up to {} psig, the critical pressure, 22.064 MPa: the steam tables of the boiling term hold there, and outside it the run is refused as PressureOutsideSteamTables. It sets the saturation temperature, the vapour's density and latent heat, the surface tension and the boiling laws. Compared at 0 to 2500 psig with no flow and at 230 psig with flow, left out of the fit: peak power 8.7 percent RMS at 0 psig (29 transients), 9.7 at 20 to 110 psig (22), 8.0 at 175 to 505 psig (21), 8.0 at 1000 to 2500 psig (14)",
                    super::token(triple_psig),
                    super::token(critical_psig),
                ),
            ),
            scalar(
                "reactivity_inserted_dollar",
                Role::Load,
                "dollar",
                Some(0.0),
                true,
                None,
                false,
                format!(
                    "the reactivity stepped in at time zero, above zero; zero or less is refused as ReactivityNotPositive. Compared from {} to {} dollars with no flow and from {} to {} with flow, the inhour values (the card's inhour_dollar) of the longest and the shortest compared period, 9.3 s or 516 ms to 9.7 ms. Above about 1.31 dollars the step ceiling follows the state (DtAboveStateCeiling). A run carries one reactivity: a segment with another is refused as ReactivityChangedDuringRun",
                    super::token(reactor_spert3_ccore_plate::REACTIVITY_MIN_NO_FLOW_DOLLAR),
                    super::token(reactor_spert3_ccore_plate::REACTIVITY_MAX_DOLLAR),
                    super::token(reactor_spert3_ccore_plate::REACTIVITY_MIN_FLOW_DOLLAR),
                    super::token(reactor_spert3_ccore_plate::REACTIVITY_MAX_DOLLAR),
                ),
            ),
            scalar(
                "coolant_flow_m_per_s",
                Role::Load,
                "m/s",
                Some(0.0),
                false,
                None,
                false,
                format!(
                    "the coolant velocity through the core, 18 ft/s at 20,000 gpm, zero or more; below zero it is refused as NegativeFlow. Compared from {} to {} m/s (200 to 20,000 gpm) at 230 psig: peak power, left out of the fit, 10.3 percent RMS at 200 to 2040 gpm (9 transients) and 19.6 at 5000 to 20,000 gpm (10). It sets the film's share of the water, 0.2385 with no flow and 0.0043 at 20,000 gpm. Each segment may carry its own flow, and each step is taken at its own, the film giving water to the bulk or taking it where the flow changes",
                    super::token(reactor_spert3_ccore_plate::FLOW_MIN_M_PER_S),
                    super::token(reactor_spert3_ccore_plate::FLOW_MAX_M_PER_S),
                ),
            ),
        ],
        floor,
        "a locked 1e-9 s: the card's own constant, a step that still moves the clock at 600 s, the longest a run of 1,000,000 steps at the ceiling reaches",
        gate,
        "0.1 of the reduced generation time, 3 ms: with 5000 to 20,000 gpm the film is thin and its exchange fast, and at 0.6 ms four of the 124 transients of the record that start as liquid water close their ledger only to 3.5e-6 or worse, at 0.3 ms every one within 6.7e-7. The step ceiling also follows the state: at the state a step starts from and at its trial end, the card refuses as DtAboveStateCeiling a step that would advance the growing prompt pole (net reactivity - 1) / l* by more than 0.0611, the decaying prompt pole (1 - net reactivity) / l* by more than 0.5, or the fastest node's conduction, heat transfer, boiling slope, condensation, plate-feedback lag and sweep by more than 1.11, the last the most a 0.3 ms step reaches over those 124 transients (1.108), rounded up. The compared numbers were run at 0.1 ms, where the peak samples sit within 0.1 ms of the peak",
        None,
        format!(
            "A run is one transient with one reactivity: every segment carries the reactivity of the first, and each segment may carry its own flow; where it changes, the film gives water to the bulk or takes it before the step, the heat held unchanged. The reactor starts at {} MW in equilibrium with every rise zero, and the reactivity is a step at time zero. hot_spot_plate_temperature_k is the cladding's temperature at the hot spot, the quantity the thermocouples measured, and hot_spot_plate_rise_k its rise above the start; the meat_rise_k fields are the meat's rises, and plate_feedback_lagged_dollar the plates' feedback as it lags their temperature. moderator_feedback_dollar, plate_feedback_dollar and void_feedback_dollar are the reactivity the water, the plates and the vapour take back; void_fraction is the feedback node's. The run passes the first maximum of power (FLAG_PAST_POWER_PEAK from that step on; peak_power_mw, peak_energy_mws, peak_time_s and peak_plate_rise_k hold the highest landed sample before it, within one step of the true maximum) and ends at the first maximum of the hot-spot plate temperature after it: the step after is refused as PastPlateTemperatureMaximum and the run is sealed with that refusal, the outcome ended. FLAG_HOT_SPOT_BOILING marks a frame whose hot-spot plate is above the saturation temperature at the system pressure. A single step starts again from the card's start at each call. NegativeState and InconsistentState are refusals of the card's step for a state handed to it that the card did not write; this door takes the initial temperature and the pressure and nothing else of a state, so no request reaches them. A step of zero or less is refused as NonPositiveDt before the step floor is tested.",
            super::token(reactor_spert3_ccore_plate::INITIAL_POWER_MW),
        ),
        &[
            "NonFiniteInput",
            "NonFiniteState",
            "NonPositiveDt",
            "InvalidCard",
            "PressureOutsideSteamTables",
            "InitialTemperatureNotLiquidWater",
            "NegativeFlow",
            "ReactivityNotPositive",
            "VoidFractionAboveOne",
            "PastPlateTemperatureMaximum",
            "NegativeTime",
            "NegativeState",
            "DtAboveStateCeiling",
            "ReactivityChangedDuringRun",
            "InconsistentState",
            "SpontaneousGeneration",
            "FirstLawBreach",
        ],
        vec![
            Advisory { name: "FLAG_PROMPT_SUPERCRITICAL", mask: reactor_spert3_ccore_plate::FLAG_PROMPT_SUPERCRITICAL, seals: true },
            Advisory { name: "FLAG_PAST_POWER_PEAK", mask: reactor_spert3_ccore_plate::FLAG_PAST_POWER_PEAK, seals: true },
            Advisory { name: "FLAG_HOT_SPOT_BOILING", mask: reactor_spert3_ccore_plate::FLAG_HOT_SPOT_BOILING, seals: true },
            Advisory { name: "FLAG_OUTSIDE_COMPARED_RANGE", mask: reactor_spert3_ccore_plate::FLAG_OUTSIDE_COMPARED_RANGE, seals: true },
        ],
        slots(&[
            ("t", "Time since the step of reactivity, in seconds."),
            ("pos[0]", "power_mw"),
            ("pos[1]", "energy_mws"),
            ("pos[2]", "reactivity_net_dollar"),
            ("vel[0]", "hot_spot_plate_temperature_k"),
            ("vel[1]", "void_feedback_dollar"),
            ("vel[2]", "peak_power_mw"),
            ("force_torque", "Rate of change of power, in MW per second."),
            ("residual", "Conservation residual: the largest of the neutron balance and the energy balances of the three nodes, each relative to its own scale."),
            (
                "flags",
                "FLAG_PROMPT_SUPERCRITICAL: net reactivity above one dollar. FLAG_PAST_POWER_PEAK: the first maximum of power has been passed. FLAG_HOT_SPOT_BOILING: the hot-spot plate is above the saturation temperature. FLAG_OUTSIDE_COMPARED_RANGE: the start or the load is outside the compared span. All seal.",
            ),
            ("proof", "Not written by the step."),
        ]),
    )
}

/// The worked run: a 1.25 dollar step at 2500 psig from 25 degC with no flow, at 0.1 ms, through
/// the power peak, and its first step.
pub(super) fn examples() -> (Value, Value) {
    pair_examples_recorded(
        DOMAIN,
        CARD,
        obj(&[("initial_temperature_k", scalar_value(298.15)), ("system_pressure_psig", scalar_value(2500.0))]),
        obj(&[("reactivity_inserted_dollar", scalar_value(1.25)), ("coolant_flow_m_per_s", scalar_value(0.0))]),
        scalar_value(1.0e-4),
        EXAMPLE_STEPS,
        100,
    )
}

/// The steps the worked run asks for: 0.2 s at 0.1 ms, through the power peak of a 1.25 dollar step.
pub(crate) const EXAMPLE_STEPS: u64 = 2_000;

/// The record, in the five parts every card gives.
pub(super) fn validation(_specimen: &str) -> [String; 5] {
    [WHAT.into(), DATA.into(), COMPARED.into(), RESULT.into(), LIMITS.into()]
}

/// Each refusal's sentence, word for word from the doc comment above the variant in the card's source.
pub(super) fn meaning(name: &str) -> Option<&'static str> {
    Some(match name {
        "NonFiniteInput" => "`dt`, the reactivity or the flow was NaN or infinite.",
        "NonFiniteState" => "The incoming state, or the trial step, was non-finite.",
        "NonPositiveDt" => "`dt` was zero or negative.",
        "InvalidCard" => "A recorded constant is not this reactor's.",
        "PressureOutsideSteamTables" => "The system pressure is outside the steam tables the boiling term stands on: below the triple point of water, 611.657 Pa absolute (about −14.6072 psig), or at or above the critical pressure, 22.064 MPa (about 3185.4 psig), where water has no saturation temperature.",
        "InitialTemperatureNotLiquidWater" => "The initial temperature is not that of liquid water at the system pressure: below 273.15 K, or at or above the saturation temperature of water at that pressure (IAPWS-IF97 region 4).",
        "NegativeFlow" => "The coolant flow is below zero.",
        "ReactivityNotPositive" => "The held reactivity is zero or less: no step in reactivity is inserted, and there is no burst.",
        "VoidFractionAboveOne" => "The vapour in a node's channels would fill them: a void fraction of one or more, where there is no water left between the plates.",
        "PastPlateTemperatureMaximum" => "The state is past the first maximum of the hot-spot plate temperature after the power peak; nothing after it is on the card.",
        "NegativeTime" => "The incoming state's clock is below zero.",
        "NegativeState" => "The incoming state holds an energy, a vapour store, a swept energy, a precursor or a flow below zero, or a power not above zero.",
        "DtAboveStateCeiling" => "The step is longer than the state lets it be, at the state it starts from or at its trial end: one Runge–Kutta step would advance the growing prompt pole (net reactivity − 1) / l* by more than 0.0611 of its time constant, the decaying prompt pole (1 − net reactivity) / l* by more than 0.5, or the fastest node’s conduction, heat transfer, boiling, condensation, plate-feedback lag and sweep by more than 1.11.",
        "ReactivityChangedDuringRun" => "The reactivity of the load is not the one of the first step of this run; a run is one step of reactivity.",
        "InconsistentState" => "The incoming state is not one this card writes: at time zero it is not the card's start; later, a derived field, a flag or a peak field is not what the card computes from the rest of the state.",
        "SpontaneousGeneration" => "The trial step created neutrons or energy beyond what the ledger supplies.",
        "FirstLawBreach" => "The trial step destroyed neutrons or energy the ledger supplies.",
        _ => return None,
    })
}
