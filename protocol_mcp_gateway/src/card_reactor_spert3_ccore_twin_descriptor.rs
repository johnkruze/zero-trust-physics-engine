//! What an agent reads of the reactor_spert3_ccore_twin card: its fields and bounds, its step
//! floor and ceiling, its refusals word for word from the card's source, its frame, its worked
//! examples and its record. Stamped from the kit; a child module of `descriptor.rs`, so it uses
//! that file's own builders.
//!
//! The card is the twin of reactor_spert3_ccore, fitted on the same record. Its description leads
//! with what it computes and its numbers against IDO-16918; the result opens with the words the
//! kit holds a calibrated model to, naming reactor_spert3_ccore.

use genesis_kernel::reactor_spert3_ccore_twin;
use serde_json::Value;

use super::{Advisory, Card, Role, finish, obj, pair_examples_recorded, scalar, scalar_value, slots};

const DOMAIN: &str = "energy";
const CARD: &str = "reactor_spert3_ccore_twin";

/// What the card is, in one sentence of the list.
const SPECIMEN: &str = "The SPERT III C-core test reactor (stainless-steel-clad, highly enriched plates) in step transients from room temperature at 0 to 2500 psig and 0 to 20,000 gpm, with subcooled boiling and void keyed on the saturation temperature at the system pressure: peak power, energy to peak, time to peak and the hot-spot plate temperature through its first maximum. Fitted on 111 transients of IDO-16918, it reads peak power within 12.6 percent RMS, energy to peak 19.4, and the hot-spot plate rise 13.1 at the power peak and 11.2 at its maximum; on transients left out of the fit, 12.6, 19.6, 13.4 and 11.4.";

/// The object.
const WHAT: &str = "The SPERT III reactor at the National Reactor Testing Station, Idaho, with the C-19/52 core: 93 percent enriched UO2-stainless-steel plates clad in stainless steel, 0.030 in thick with 0.128 in water channels, 36 in active length. Step insertions of reactivity from room temperature at system pressures of 0 to 2500 psig with no forced flow, and at 230 psig with 200 to 20,000 gpm. Point kinetics with six delayed groups, and three plate-and-water nodes: the core average, a feedback node at a fitted peaking of 2.10 (the core's temperatures as its reactivity weighs them) and the hot spot at 2.04 times 1.30, where the thermocouples sat. Each node carries the plate, a film of the water next to the plates, the bulk water and the vapour in its channels. The plate passes heat to the film by conduction and, above the saturation temperature at the system pressure, by subcooled boiling (Thom's nucleate-boiling law, capped smoothly by Zuber's critical heat flux with the Ivey-Morris subcooling term); a small share of the boiling heat stays as vapour, which condenses in 3 ms and is swept out with the flow. The reactivity takes back the water's temperature (the measured isothermal coefficient, 1.8 cents per degC rising to 16 cents per degF at 650 degF, on the film and the bulk), the plates' (the water they displace as they expand, and their own Doppler and bowing coefficient) and the void (45 cents per percent). The water and steam properties at saturation are IAPWS-IF97 region 4 and the IAPWS auxiliary equations of Wagner and Pruss, computed in the engine's own math, from the triple point to the critical point.";

/// The data.
const DATA: &str = "IDO-16918, C. R. Toole, The Effects of Pressure and Flow on Room Temperature Power Excursions in Spert III, Phillips Petroleum Co. for the US Atomic Energy Commission, June 1964, OSTI 4042264: its appendix tabulates 126 step transients with pressure, initial temperature, flow, period, peak power, energy to the time of peak power, and the plate surface temperature at that time and at its maximum, read by thermocouples spot-welded to a plate at the flux peak of a central assembly. The transcription and the 111 usable rows are those of reactor_spert3_ccore: 15 rows are set aside for an abnormal thermocouple, a likely misprint or a missing or doubtful initial temperature. Of the 111, peak power and energy are on 105 rows, the plate temperature at peak power on 104 and its maximum on 95, the rest carrying the report's marks for a value still rising or higher at the scram. Core constants from IDO-17138 (Condit, Scott and Johnson, 1967), Table B-1, and IDO-16918 Table I. No measured value of either report is stored in the card.";

/// How the card is compared.
const COMPARED: &str = "Eight constants fitted by least squares on the logarithms of card over measured, all four quantities and all 111 rows together: the plate-to-film and film-to-bulk conductances with no flow (5315.50 and 240.502 W/m2 K), the film's share of the water (0.191875), the feedback node's peaking (2.10289), the plate's coefficient beyond the water it displaces (0.000362933 dollars per K), the Thom and Zuber multipliers (2.63504 and 2.06650) and the share of boiling heat that stays as vapour (0.000159156). Fixed: the share of fission energy the water takes directly, 0.026, and the condensation time, 3 ms. For the plate maximum the card's first maximum after the power peak is compared, or its value 1.47437 s after the peak where that comes first (a fitted ninth number of the comparison: the record gives no scram time). Then each tenth of the 111 rows, every tenth in printed order, left out in turn, the constants fitted on the rest and the left-out tenth computed with them. The error is card over measured less one, in percent; temperatures are rises above the initial temperature.";

/// The result, its first sentence the word it is served under.
const RESULT: &str = "Fitted on all 111 transients of IDO-16918, every transient and reactor_spert3_ccore's comparison with 39 of them read when the structure was chosen; reactor_spert3_ccore, with two constants fitted on 72 of the same transients, stays served under its own ID. In the fit, on the 111 transients, run at 0.1 ms steps: peak power 12.6 percent RMS, worst 55 (105 transients, 56 percent of them within the report's power calibration of about 10 percent); energy to peak 19.4 percent, worst 116 (105); the hot-spot plate rise at peak power 13.1 percent, worst 62 (104); at its first maximum 11.2 percent, worst 61 (95). Compared with transients left out of the fit (10-fold): 12.6, 19.6, 13.4 and 11.4 percent RMS. On 10 transients not in the fit, whose temperatures the report marks abnormal and whose power and energy stand: peak power 19.0 and energy 14.4 percent RMS. Strongest at 0 psig with no flow (29 transients: 8.7, 13.6, 8.3 and 8.4 percent) and at 1000 to 2500 psig (14: 10.7, 12.4, 8.7 and 10.1); weakest with 5000 to 20,000 gpm at 230 psig (10: peak power 20.6, plate maximum 26.1, worst 61) and in energy with 200 to 2040 gpm (9: 33.4). The record against itself, repeated transients at one pressure and flow with periods within 3 percent: 8.9, 13.7, 8.5 and 5.5 percent RMS. Across 0 to 2500 psig at about 11 ms the card's void takes back 4.2 cents at the power peak at 0 psig, 0.5 at 110 psig and none above 1000 psig, as the report found boiling suppressed by pressure. Record SHA-256 51e7fbb15d4fbfd8428f8af0bc043555404fb9c4b0f37edc4abc1979b69284f8, with its table of numbers SHA-256 c11cc41699acbd81c3e76d6070fd002db2a791fce895cfeb08a083b0726a30ec.";

/// Where the numbers stop.
const LIMITS: &str = "One core, from room temperature: compared on initial temperatures of 21 to 36 degC, pressures of 0 to 2500 psig with no flow, flows of 200 to 20,000 gpm (0.054864 to 5.4864 m/s) at 230 psig only, and reactivities from the inhour value of the longest compared period to that of the shortest, 0.412287 to 1.30540 dollars with no flow and 0.870348 to 1.30540 with flow. A start or a load outside that span is computed by the same equations and every frame carries FLAG_OUTSIDE_COMPARED_RANGE. IDO-16918 tabulates no time to peak: the card's, 0.175 to 104 s over the compared transients from its 10 W start, is not compared. The plate is one temperature across its 0.030 in, the thermocouple's reading; the water's own heat capacity and its Dittus-Boelter properties are those of 25 degC. Thom's law was fitted by its authors at 5.2 to 14 MPa and is used here from 0.1 to 17.3 MPa with its multiplier fitted. The card refuses by name a pressure outside the steam tables (PressureOutsideSteamTables: below the triple point or at or above the critical pressure, where water has no saturation temperature), a start that is not liquid water (InitialTemperatureNotLiquidWater), a void fraction of one or more (VoidFractionAboveOne), and a step longer than the state allows (DtAboveStateCeiling). At 1000 to 2500 psig and periods of 11.5 ms and under, six of seven transients read peak power 12 to 17 percent low. The isothermal coefficient continues past 650 degF along its last measured segment.";

pub(super) fn card() -> Card {
    let reactor = reactor_spert3_ccore_twin::ReactorSpert3CcoreTwin::c_core_twin();
    let gate = reactor.gate_dt_s().expect("specimen gate");
    let floor = reactor.floor_dt_s().expect("specimen floor");
    let triple_psig = reactor_spert3_ccore_twin::TRIPLE_POINT_PRESSURE_MPA / reactor_spert3_ccore_twin::MPA_PER_PSI
        - reactor_spert3_ccore_twin::ATMOSPHERE_PSI;
    let critical_psig = reactor_spert3_ccore_twin::CRITICAL_PRESSURE_MPA / reactor_spert3_ccore_twin::MPA_PER_PSI
        - reactor_spert3_ccore_twin::ATMOSPHERE_PSI;
    finish(
        DOMAIN,
        CARD,
        SPECIMEN,
        vec![
            scalar(
                "initial_temperature_k",
                Role::Phase,
                "K",
                Some(reactor_spert3_ccore_twin::FREEZING_K),
                false,
                Some(reactor_spert3_ccore_twin::CRITICAL_TEMPERATURE_K),
                true,
                format!(
                    "the temperature of plates and water at the step: liquid water, from {} K up to the saturation temperature of water at system_pressure_psig (IAPWS-IF97 region 4: 373.12 K at 0 psig, 627.06 K at 2500 psig), below the critical temperature, {} K. A start that is not liquid water is refused as InitialTemperatureNotLiquidWater. Compared on starts of 21 to 36 degC; outside them every frame carries FLAG_OUTSIDE_COMPARED_RANGE",
                    super::token(reactor_spert3_ccore_twin::FREEZING_K),
                    super::token(reactor_spert3_ccore_twin::CRITICAL_TEMPERATURE_K),
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
                    "the system pressure in pounds per square inch above the atmosphere, from {} psig, the triple point of water, up to {} psig, the critical pressure, 22.064 MPa: the steam tables of the boiling term hold there, and outside it the run is refused as PressureOutsideSteamTables. It sets the saturation temperature, the vapour's density and latent heat, the surface tension and the boiling laws. Compared at 0 to 2500 psig with no flow and at 230 psig with flow: peak power 8.7 percent RMS at 0 psig (29 transients), 12.2 at 20 to 110 psig (22), 11.4 at 175 to 505 psig (21), 10.7 at 1000 to 2500 psig (14)",
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
                    super::token(reactor_spert3_ccore_twin::REACTIVITY_MIN_NO_FLOW_DOLLAR),
                    super::token(reactor_spert3_ccore_twin::REACTIVITY_MAX_DOLLAR),
                    super::token(reactor_spert3_ccore_twin::REACTIVITY_MIN_FLOW_DOLLAR),
                    super::token(reactor_spert3_ccore_twin::REACTIVITY_MAX_DOLLAR),
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
                    "the coolant velocity through the core, 18 ft/s at 20,000 gpm, zero or more; below zero it is refused as NegativeFlow. Compared from {} to {} m/s (200 to 20,000 gpm) at 230 psig: peak power 17.1 percent RMS at 200 to 2040 gpm (9 transients) and 20.6 at 5000 to 20,000 gpm (10). Each segment may carry its own flow, and each step is taken at its own",
                    super::token(reactor_spert3_ccore_twin::FLOW_MIN_M_PER_S),
                    super::token(reactor_spert3_ccore_twin::FLOW_MAX_M_PER_S),
                ),
            ),
        ],
        floor,
        "a locked 1e-9 s: the card's own constant, a step that still moves the clock at 600 s, the longest a run of 1,000,000 steps at the ceiling reaches",
        gate,
        "0.2 of the reduced generation time, 3 ms, the time constant of the prompt pole at zero net reactivity, where every run starts. The step ceiling also follows the state: at the state a step starts from and at its trial end, the card refuses as DtAboveStateCeiling a step that would advance the growing prompt pole (net reactivity - 1) / l* by more than 0.0611, the decaying prompt pole (1 - net reactivity) / l* by more than 0.5, or the fastest node's heat transfer, boiling slope, condensation and sweep by more than 0.6, each at least the most a 0.6 ms step reaches over the 124 transients of the record that start as liquid water (0.0598, 0.099 and 0.581). The compared numbers were run at 0.1 ms, where the peak samples sit within 0.1 ms of the peak",
        None,
        format!(
            "A run is one transient with one reactivity: every segment carries the reactivity of the first, and each segment may carry its own flow. The reactor starts at {} MW in equilibrium with every rise zero, and the reactivity is a step at time zero. hot_spot_plate_temperature_k is the plate temperature at the hot spot, the quantity the thermocouples measured, and hot_spot_plate_rise_k its rise above the start. moderator_feedback_dollar, plate_feedback_dollar and void_feedback_dollar are the reactivity the water, the plates and the vapour take back; void_fraction is the feedback node's. The run passes the first maximum of power (FLAG_PAST_POWER_PEAK from that step on; peak_power_mw, peak_energy_mws, peak_time_s and peak_plate_rise_k hold the highest landed sample before it, within one step of the true maximum) and ends at the first maximum of the hot-spot plate temperature after it: the step after is refused as PastPlateTemperatureMaximum and the run is sealed with that refusal, the outcome ended. FLAG_HOT_SPOT_BOILING marks a frame whose hot-spot plate is above the saturation temperature at the system pressure. A single step starts again from the card's start at each call. NegativeState and InconsistentState are refusals of the card's step for a state handed to it that the card did not write; this door takes the initial temperature and the pressure and nothing else of a state, so no request reaches them. A step of zero or less is refused as NonPositiveDt before the step floor is tested.",
            super::token(reactor_spert3_ccore_twin::INITIAL_POWER_MW),
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
            Advisory { name: "FLAG_PROMPT_SUPERCRITICAL", mask: reactor_spert3_ccore_twin::FLAG_PROMPT_SUPERCRITICAL, seals: true },
            Advisory { name: "FLAG_PAST_POWER_PEAK", mask: reactor_spert3_ccore_twin::FLAG_PAST_POWER_PEAK, seals: true },
            Advisory { name: "FLAG_HOT_SPOT_BOILING", mask: reactor_spert3_ccore_twin::FLAG_HOT_SPOT_BOILING, seals: true },
            Advisory { name: "FLAG_OUTSIDE_COMPARED_RANGE", mask: reactor_spert3_ccore_twin::FLAG_OUTSIDE_COMPARED_RANGE, seals: true },
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
        "DtAboveStateCeiling" => "The step is longer than the state lets it be, at the state it starts from or at its trial end: one Runge–Kutta step would advance the growing prompt pole (net reactivity − 1) / l* by more than 0.0611 of its time constant, the decaying prompt pole (1 − net reactivity) / l* by more than 0.5, or the fastest node’s heat transfer, boiling, condensation and sweep by more than 0.6.",
        "ReactivityChangedDuringRun" => "The reactivity of the load is not the one of the first step of this run; a run is one step of reactivity.",
        "InconsistentState" => "The incoming state is not one this card writes: at time zero it is not the card's start; later, a derived field, a flag or a peak field is not what the card computes from the rest of the state.",
        "SpontaneousGeneration" => "The trial step created neutrons or energy beyond what the ledger supplies.",
        "FirstLawBreach" => "The trial step destroyed neutrons or energy the ledger supplies.",
        _ => return None,
    })
}

/// A list of the card's numbers as its source holds them, joined for a sentence.
fn numbers(values: &[f64]) -> String {
    values.iter().map(|value| format!("{value}")).collect::<Vec<_>>().join(", ")
}

/// Every equation, constant, property law and discretisation of the card, in
/// the order a rebuild needs them, each number the card's own constant: what
/// an outsider needs to compute the card's answers again from its
/// description alone. Units: MW, MJ (MW s), K, s, m and dollars unless a
/// sentence names another; temperatures in degC are T - 273.15.
pub(super) fn equations() -> Vec<String> {
    use reactor_spert3_ccore_twin as c;
    let state_keys: Vec<String> = crate::card_reactor_spert3_ccore_twin::rest(|fields| fields.iter().map(|(name, _)| (*name).to_string()).collect());
    vec![
        format!(
            "Kinetics, six groups: dP/dt = ((rho - 1) / l*) P + sum lambda_i C_i and dC_i/dt = (b_i / l*) P - lambda_i C_i, rho the net reactivity in dollars. l* = {} s (IDO-16918 Table I); b_i {} and lambda_i {} per s (Keepin 1965, thermal fission of U-235). The start: P_0 = {} MW, C_i = b_i P_0 / (l* lambda_i), every rise, store and account zero, t = 0. A step of reactivity rho_0 (the load) is in from t = 0.",
            c::GENERATION_TIME_REDUCED_S,
            numbers(&c::GROUP_FRACTION),
            numbers(&c::GROUP_DECAY_PER_S),
            c::INITIAL_POWER_MW,
        ),
        format!(
            "Three nodes, each per the whole plate area A = {} m^2: the core average (F = 1), the feedback node (F = F_r = {}) and the hot spot (F = 2.04 x 1.30). Each holds the plate rise T_p, the film rise T_f, the bulk-water rise T_w (rises above the initial temperature theta_i) and the latent heat of its vapour U in MJ. C_p dT_p/dt = F (1 - f_w) P - H_1 (T_p - T_f) - A q_b; C_f dT_f/dt = F f_w phi P + H_1 (T_p - T_f) - H_2 (T_f - T_w) + (1 - eps) A q_b - s C_f T_f; C_b dT_w/dt = F f_w (1 - phi) P + H_2 (T_f - T_w) + U / tau_c - s C_b T_w; dU/dt = eps A q_b - U / tau_c - s U; dE/dt = P. Plates C_p = {} MJ/K, water C_w = {} MJ/K, film C_f = phi C_w and bulk C_b = (1 - phi) C_w, phi = {}; f_w = {} (fission heat directly in the water); tau_c = {} s; eps = {} (the share of boiling heat that stays as vapour); sweep s = 2 v / L, L = {} m, v the load's coolant flow.",
            c::HEAT_TRANSFER_AREA_M2,
            c::FEEDBACK_PEAKING,
            c::PLATE_HEAT_CAPACITY_MWS_PER_K,
            c::WATER_HEAT_CAPACITY_MWS_PER_K,
            c::FILM_SHARE,
            c::WATER_DIRECT_HEATING_FRACTION,
            c::CONDENSATION_TIME_S,
            c::NET_VAPOUR_SHARE,
            c::ACTIVE_LENGTH_M,
        ),
        format!(
            "Conductances: H_1 = A sqrt(h_s^2 + h_DB^2) / 1e6 and H_2 = A sqrt(h_fb^2 + h_DB^2) / 1e6 MW/K, h_s = {} and h_fb = {} W/(m^2 K). Dittus-Boelter for water at 25 degC between the plates: h_DB = 0.023 Re^0.8 Pr^0.4 k / D_h, Re = rho v D_h / mu, with rho = {} kg/m^3, mu = {} Pa s, k = {} W/(m K), Pr = {}, D_h = 2 x 0.128 x 0.0254 m; zero with no flow. A power x^y is exp(y ln x) throughout.",
            c::PLATE_FILM_CONDUCTANCE_W_PER_M2_K,
            c::FILM_BULK_CONDUCTANCE_W_PER_M2_K,
            c::WATER_DENSITY_KG_PER_M3,
            c::WATER_VISCOSITY_PA_S,
            c::WATER_CONDUCTIVITY_W_PER_M_K,
            c::WATER_PRANDTL,
        ),
        format!(
            "Boiling, per node, when the wall superheat dT_sat = theta_i + T_p - T_sat is above zero (else q_b = 0): q_n = m_n (dT_sat / ({} e^(-p / {} MPa)))^2 MW/m^2 (Thom), q_c = m_c q_Z (1 + {} (rho_f / rho_g)^{} c_l dT_sub / h_fg) (Zuber with the Ivey-Morris subcooling term), dT_sub = max(0, T_sat - (theta_i + T_w)), q_Z = {} sqrt(rho_g) h_fg (sigma g (rho_f - rho_g))^(1/4) / 1e6 MW/m^2 with the fourth root as sqrt(sqrt()), g = {} m/s^2, c_l = {} J/(kg K); q_b = q_n q_c / (q_n + q_c). m_n = {}, m_c = {}. p is the absolute pressure, (psig + {}) x {} MPa.",
            c::THOM_SUPERHEAT_K,
            c::THOM_PRESSURE_MPA,
            c::IVEY_MORRIS_CONSTANT,
            c::IVEY_MORRIS_EXPONENT,
            c::ZUBER_CONSTANT,
            c::GRAVITY_M_PER_S2,
            c::WATER_SPECIFIC_HEAT_J_PER_KG_K,
            c::NUCLEATE_MULTIPLIER,
            c::CRITICAL_FLUX_MULTIPLIER,
            c::ATMOSPHERE_PSI,
            c::MPA_PER_PSI,
        ),
        format!(
            "Saturation, at the system pressure and held for the run. T_sat by IAPWS-IF97 region 4 backward: beta = sqrt(sqrt(p)); E = beta^2 + n3 beta + n6, F = n1 beta^2 + n4 beta + n7, G = n2 beta^2 + n5 beta + n8, D = 2 G / (-F - sqrt(F^2 - 4 E G)), T_sat = (n10 + D - sqrt((n10 + D)^2 - 4 (n9 + n10 D))) / 2 K, with this card's n1 to n10 = {}. With tau = 1 - T_sat / {} K, Wagner and Pruss (1993): S = a1 tau + a2 tau sqrt(tau) + a3 tau^3 + a4 tau^3 sqrt(tau) + a5 tau^4 + a6 tau^7.5, a = {}; S' = a1 + 1.5 a2 sqrt(tau) + 3 a3 tau^2 + 3.5 a4 tau^2 sqrt(tau) + 4 a5 tau^3 + 7.5 a6 tau^6.5; p_s = {} e^(T_c S / T_sat) MPa, dp/dT = p_s (-T_c S / T_sat^2 - S' / T_sat); rho_f = {} (1 + sum b_i tau^(1/3, 2/3, 5/3, 16/3, 43/3, 110/3)), b = {}; rho_g = {} exp(sum c_i tau^(2/6, 4/6, 8/6, 18/6, 37/6, 71/6)), c = {}; the latent heat h_fg = T_sat (dp/dT) 1e6 (1 / rho_g - 1 / rho_f) J/kg (Clausius-Clapeyron); surface tension sigma = {} tau^{} (1 + b_s tau) N/m, b_s = {} (IAPWS 1994). tau^2 and tau^3 are products and every other power of tau is exp(y ln tau).",
            numbers(&c::SATURATION_N),
            c::CRITICAL_TEMPERATURE_K,
            numbers(&c::VAPOUR_PRESSURE_A),
            c::CRITICAL_PRESSURE_MPA,
            c::CRITICAL_DENSITY_KG_PER_M3,
            numbers(&c::LIQUID_DENSITY_B),
            c::CRITICAL_DENSITY_KG_PER_M3,
            numbers(&c::VAPOUR_DENSITY_C),
            c::SURFACE_TENSION_N_PER_M,
            c::SURFACE_TENSION_EXPONENT,
            c::SURFACE_TENSION_B,
        ),
        format!(
            "Reactivity, on the feedback node: rho = rho_0 - rho_mod - rho_plate - rho_void. rho_mod = phi (I(theta_f) - I(theta_f0)) + (1 - phi) (I(theta_i + T_w) - I(theta_i)) (moderator_feedback_dollar), temperatures in degC. I(theta) is the integral from 0 degC of the isothermal coefficient alpha(theta), dollars per K: {} flat up to {} degC, then straight through the points ({}) degC with ({}) dollars per K, its last segment continued; I(theta) = {} theta up to the first point, else {} x {} plus, segment by segment, alpha_i x + m_i x^2 / 2 over the width x of each segment to theta. The film's temperature as the reactivity takes it bends smoothly onto saturation: theta_f = T_sat - w ln(1 + e^((T_sat - theta_i - T_f) / w)) degC, w = {} K, computed with ln(1 + y) as ln_1p and, where the exponent u is above zero, as u + ln_1p(e^(-u)); theta_f0 is its value at T_f = 0. rho_plate = D chi (T_p - T_w) + a_D T_p (plate_feedback_dollar), D chi = {} x (3 x 17.3e-6 x 0.030 / 0.128) dollars per K, the product taken left to right: the water the plates displace, 44 cents per percent of moderator density times 3 x 17.3e-6 /K x 0.030 in / 0.128 in, a_D = {} dollars per K (Doppler and bowing, fitted). rho_void = K_v alpha_v (void_feedback_dollar), K_v = {} dollars, alpha_v = U x 1e6 / (rho_g h_fg V_w) the void fraction (void_fraction), V_w = C_w x 1e6 / ({} x {}) m^3, 0.159.",
            c::COEFFICIENT_DOLLAR_PER_K[0],
            c::COEFFICIENT_TEMPERATURE_C[0],
            numbers(&c::COEFFICIENT_TEMPERATURE_C),
            numbers(&c::COEFFICIENT_DOLLAR_PER_K),
            c::COEFFICIENT_DOLLAR_PER_K[0],
            c::COEFFICIENT_DOLLAR_PER_K[0],
            c::COEFFICIENT_TEMPERATURE_C[0],
            c::FILM_SATURATION_WIDTH_K,
            c::DENSITY_COEFFICIENT_DOLLAR,
            c::PLATE_COEFFICIENT_DOLLAR_PER_K,
            c::VOID_COEFFICIENT_DOLLAR,
            c::WATER_DENSITY_KG_PER_M3,
            c::WATER_SPECIFIC_HEAT_J_PER_KG_K,
        ),
        "The step: one classical fourth-order Runge-Kutta step of the 20 values P, C_1 to C_6, E and, for the core average, the feedback node and the hot spot in turn, T_p, T_f, T_w and U, every property at each stage's own values; the landed values below the normal range are written as zero. Over the step, by the two-point Hermite rule h (f_0 + f_1) / 2 + h^2 (f'_0 - f'_1) / 12 from the values and the rates at its two ends: Q = integral of (rho_0 - rho) P (feedback_integral_dollar_mws), f' = (d(rho_0 - rho)/dt) P + (rho_0 - rho) dP/dt, and for each node S_j = integral of s (C_f T_f + C_b T_w + U) (swept_energy_mws, feedback_swept_energy_mws, hot_spot_swept_energy_mws). The rate of rho_0 - rho is phi alpha(theta_f) (d theta_f / d T_f) dT_f/dt + (1 - phi) alpha(theta_i + T_w) dT_w/dt + D chi (dT_p/dt - dT_w/dt) + a_D dT_p/dt + K_v (dalpha_v/dt), on the feedback node, with d theta_f / d T_f = 1 / (1 + e^(-u)) for u above zero and e^u / (1 + e^u) otherwise.".to_string(),
        format!(
            "residual_conservation_error = max(|d_N| / max(N, N_0), |d_j| / (F_j E) over the three nodes while E is above zero). Neutrons: N = P + sum C_i, N_0 its start value (population_anchor_mw), d_N = N - N_0 - (rho_0 E - Q) / l* (population_defect_mw). Node j: d_j = F_j E - (C_p T_p + C_f T_f + C_b T_w + U + S_j) (energy_defect_mws, feedback_energy_defect_mws, hot_spot_energy_defect_mws). A step past {:e} is refused, as SpontaneousGeneration when the balance that sets the residual is above zero and as FirstLawBreach when below.",
            c::RESIDUAL_CONSERVATION_LIMIT,
        ),
        format!(
            "The step ceiling, at the state a step starts from and at its landing: with r_g = (rho - 1) / l* when above zero and r_d = (1 - rho) / l* otherwise, and r_t the largest over the nodes of H_1 / C_p + (H_1 + H_2) / C_f + H_2 / C_b + 1 / tau_c + 2 s + A q_b' / C_p, q_b' = q_c^2 (2 q_n / dT_sat) / (q_n + q_c)^2 the boiling slope (zero without boiling), a step is refused as DtAboveStateCeiling when dt r_g is above {}, dt r_d above {} or dt r_t above {}. A landing whose void fraction U x 1e6 / (rho_g h_fg V_w) is one or more in any node is refused as VoidFractionAboveOne.",
            c::GROWTH_PHASE_STEP,
            c::DECAY_PHASE_STEP,
            c::THERMAL_PHASE_STEP,
        ),
        "The run's records, at each landed step: while past_power_peak is unset, a P strictly above peak_power_mw (P_0 at the start) sets peak_power_mw, peak_energy_mws, peak_time_s and peak_plate_rise_k to its P, E, t and hot-spot T_p; a hot-spot T_p strictly above plate_rise_max_k (0 at the start) sets it and plate_rise_max_time_s. past_power_peak is set by the first landed step whose dP/dt (power_rate_mw_per_s, at the landed state) is not above zero; past_plate_maximum by the first landed step after it whose hot-spot dT_p/dt (hot_spot_plate_rate_k_per_s) is not above zero, and the next step is refused as PastPlateTemperatureMaximum, the run's end. hot_spot_plate_temperature_k = theta_i + hot-spot T_p in K; reactivity_net_dollar = rho; prompt_supercritical is rho above 1, hot_spot_boiling theta_i + hot-spot T_p above T_sat (saturation_temperature_k).".to_string(),
        format!("The state's keys, in the order the engine writes them: {}.", state_keys.join(", ")),
    ]
}
