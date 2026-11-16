//! What an agent reads of the reactor_spert3_twin card: its fields and bounds, its step
//! floor and ceiling, its refusals word for word from the card's source, its
//! frame, its worked examples and its record. Stamped from the kit; a child
//! module of `descriptor.rs`, so it uses that file's own builders.

use genesis_kernel::reactor_spert3_twin;
use serde_json::Value;

use super::{Advisory, Card, Role, finish, obj, pair_examples_recorded, scalar, scalar_value, slots};

const DOMAIN: &str = "energy";
const CARD: &str = "reactor_spert3";

/// What the card is, in one sentence of the list.
const SPECIMEN: &str = "The SPERT III E-core test reactor as one model for all 70 tabulated tests of IDO-17281, cold startup, hot startup at 260 and 500 degF, hot standby and operating power: six-group kinetics, the transient rod's worth curve over its travel, radial conduction in the fuel, Doppler and moderator feedback, and the hot-spot clad temperature, fitted on all 70 tests; with each test left out of the fit in turn, from a start that never saw it, peak power within 6.8 percent and time to peak within 4.1 percent at the median test.";

/// The accuracy of the card against IDO-17281, fitted on all 70 tests and
/// compared with each test left out of the fit. Written from
/// `evidence/research/reactor_spert3/twin/` (`stats.py`, `loo.py`).
pub(crate) const RESULT: &str = "Fitted on all 70 tests of IDO-17281, every test read when the structure was chosen: eight constants of the card, the moderator weight held at one, and the three startup powers, run on to the minimum of the fit's objective (226.98, from 237.67 at 0.14.3; no refit on 69 tests scores lower on all 70). In the fit, at the median test, peak power within 6.3 percent, energy to the peak within 8.7, time to the peak within 4.0 and the hot-spot clad temperature rise within 9.5; root mean square in the report's standard deviations 0.99, 2.72, 3.22 and 1.34; 67 of 70, 64 of 70, 40 of 66 and 60 of 69 tests within two. With each test left out of the fit in turn, refitted on the other 69 from the fit of a tenth of the tests that left it out, itself started from the report's own values: 6.8, 8.6, 4.1 and 9.2 percent at the median, 2.15, 2.87, 3.33 and 1.39 standard deviations, and 67, 61, 39 and 60 tests within two. By series, left out of the fit, peak power, energy, time to the peak and clad at the median: cold startup (30 tests) 5.0, 6.7, 3.1 and 11.8 percent; 260 degF (15) 14.9, 13.4, 4.2 and 8.3, peak power high; 500 degF (17) 6.3, 6.0, 5.0 and 2.6; hot standby (5) 12.7 low, 10.9 low, 3.6 percent and 14 degF RMS in the clad maximum; operating power (3) 11.5 low, 81.2 low, 39.0 percent early and 12 degF. The operating-power gap is the largest, and it is the first 0.15 s: the report's Figs. D-68 to D-70 put the reactivity in over 0.10 to 0.16 s from the transient's start, longer for the smaller insertion, where the card's rod takes 0.06 to 0.08 s; the hand-started clock of those tests is not the cause, Table VII's times agreeing with the figures' times from the start within 0.012 s. Test 52, scrammed at 20 s at 0.14 MW, reads 3.3 times its power left out of the fit, 1.4 in it. The four scrammed startup tests (24, 25, 52, 53) are compared at their printed time. The two worked answers through this card: the cold 1-dollar burst at 76 degF, 10.78 MW at 1.015 s with 2.16 MW s (Table III test 49 at 1.00 dollar: 11 MW, 0.97 s, 2.1 MW s), and hot standby test 81 at 1.17 dollars from 0.9 MW, 287 MW at 0.133 s with 6.96 MW s net (measured 330 MW, 0.135 s, 7.8 MW s). Record SHA-256 ba3ed7ee1a30824a0a230e54858d0bd9aad5aa93d0ad6838b99481f9bdda54de (twin/COMPARISON.md), with its per-test table SHA-256 ceadb117ee543f36cacd32a4549544b048e96b9b418a8615f7eecce9331c7427 (twin/refit_2026-10-18/out/scored_new/per_test.csv).";

pub(super) fn card() -> Card {
    let specimen = reactor_spert3_twin::ReactorSpert3Twin::e_core();
    let gate = specimen.gate_dt_s().expect("card gate");
    let floor = specimen.floor_dt_s().expect("card floor");
    finish(
        DOMAIN,
        CARD,
        SPECIMEN,
        vec![
            scalar(
                "inlet_temperature_k",
                Role::Phase,
                "K",
                Some(273.15),
                true,
                None,
                false,
                "the coolant at the core inlet at the start, above 273.15 K and below the saturation temperature at the system pressure (IAPWS-IF97), which a start at or above it is refused as StateOutsideCard. The 70 tests span 53 to 92 degF cold, 251 to 259 degF and 498 to 501 degF hot startup, and 502 to 513 degF hot standby and operating power: 284.8 to 540.4 K. The generation time, the Doppler coefficient, the rod worth curve, the gap width and the peak-to-average power density are the report's at 70, 250 and 500 degF, linear in this inlet temperature between them and continued along the nearer segment outside. The moderator coefficient is the report's at the same three temperatures, taken in the core water's temperature and integrated from the start's water temperature T_w0 to the water's T_w: within one segment it acts at the mean core water temperature, (T_w0 + T_w) / 2, not at the inlet".to_string(),
            ),
            scalar(
                "system_pressure_mpa",
                Role::Phase,
                "MPa",
                Some(reactor_spert3_twin::PRESSURE_MIN_MPA),
                true,
                Some(reactor_spert3_twin::PRESSURE_MAX_MPA),
                true,
                "the system pressure, absolute, above 0.000611657 MPa and below 22.064 MPa, the triple and critical points of IAPWS-IF97 region 4. Tested at two: 0.0853 MPa, the atmosphere at the Idaho site, for cold startup, and 10.44 MPa, 1500 psig, for every hot test. It sets the saturation temperature, the nucleate-boiling superheat and the rod's acceleration".to_string(),
            ),
            scalar(
                "coolant_flow_m_per_s",
                Role::Phase,
                "m/s",
                Some(0.0),
                false,
                None,
                false,
                "the average coolant speed through the core, held for the run, zero or more. Tested with no flow cold, 0.73 to 6.71 m/s (2.4 to 22 ft/s) at 260 degF, 1.46 to 7.32 m/s (4.8 to 24 ft/s) at 500 degF and 4.27 m/s (14 ft/s) in hot standby and operating power. It sets the forced-convection coefficient and the sweep 2 v / L of the water; with no flow the water has no steady temperature and starts at the inlet's".to_string(),
            ),
            scalar(
                "initial_power_mw",
                Role::Phase,
                "MW",
                Some(0.0),
                true,
                None,
                false,
                "the steady power before the rod moves, above zero. Table VII gives it per test, 0.9 to 1.3 MW in hot standby and 19 MW at operating power. The startup tests of Tables III and V give none; the card was fitted with 7.9949547 W (7.9949547e-6 MW) cold, 4.1397337 W at 260 degF and 8.3415176 W at 500 degF, against the report's about 50 W and 10 to 50 W. The start is the steady state at this power".to_string(),
            ),
            scalar(
                "reactivity_inserted_dollar",
                Role::Load,
                "dollar",
                Some(0.0),
                false,
                None,
                false,
                format!(
                    "the reactivity the transient rod is ejected through, zero or more; below zero is refused as ReactivityNegative. The rod starts at the height x_0 where the worth curve of Fig. B-4 holds this reactivity and falls at {} in/s^2 (1 + {} p), p the system pressure in MPa, from {:.1} ms after the clock's zero. Compared from 0.46 to 1.29 dollars: Tables III and V by the inhour value of each tested period, Table VII as printed. Past them the card computes the same rod and feedback. A run is one ejection with one reactivity: a segment with another is refused as ReactivityChangedDuringRun",
                    reactor_spert3_twin::ROD_ACCELERATION_IN_PER_S2,
                    reactor_spert3_twin::ROD_ACCELERATION_GAIN_PER_MPA,
                    reactor_spert3_twin::ROD_DELAY_S * 1000.0,
                ),
            ),
        ],
        floor,
        "a locked 1e-9 s: the card's own constant",
        gate,
        "0.2 of the reduced generation time at 70 degF, 2.15 ms. The step also holds a ceiling that follows the state, 0.4 of the prompt time constant l* / |1 - rho| at the state it starts from or lands on, or 0.06 of it while rho is above one dollar: at this ceiling that binds below a net reactivity of -1.0 dollar, and above 1.30 dollars at 70 degF and 1.31 at 500 degF. A step at or under this ceiling that the state's own ceiling does not take whole is taken in 2, 4, 8, ... equal pieces, each held to it at its start and its landing and closed on the same ledgers, none shorter than the floor (since engine 0.15.2; through 0.15.1 it was refused as DtAboveStateCeiling, at 4e-4 s from 1.3228 dollars); a step that lands whole lands as it always did. DtAboveStateCeiling now names only a step whose pieces would be shorter than the floor",
        None,
        "The four start fields are held for the run and the start is the steady reactor at the initial power: precursors at equilibrium, each fuel ring, the clad and the water at their steady temperatures. A run is one ejection of the transient rod: every segment carries the reactivity of the first, and another is refused before the first step as ReactivityChangedDuringRun. A run goes on for as long as it is asked: through the burst, its peak, and the clad's rise after it. The last state carries peak_power_mw, time_of_peak_s and energy_to_peak_mws, the highest landed power, when, and the energy released by then (energy_to_peak_mws minus initial_power_mw times time_of_peak_s is Table VII's net energy), and hot_clad_max_k with its time, the highest hot-spot clad temperature of the run. The report's clad temperatures were compared over the run to 2 s past the peak for startups and to 1 s for Table VII, where the rods were scrammed. A single step starts again from the start fields at each call.".to_string(),
        &[
            "NonFiniteInput",
            "NonFiniteState",
            "NonPositiveDt",
            "InvalidSpecimen",
            "ReactivityNegative",
            "ReactivityChangedDuringRun",
            "StateOutsideCard",
            "DtAboveStateCeiling",
            "FuelMelting",
            "WaterAtSaturation",
            "SpontaneousGeneration",
            "FirstLawBreach",
        ],
        vec![
            Advisory { name: "FLAG_ROD_MOVING", mask: reactor_spert3_twin::FLAG_ROD_MOVING, seals: true },
            Advisory { name: "FLAG_PAST_PEAK", mask: reactor_spert3_twin::FLAG_PAST_PEAK, seals: true },
            Advisory { name: "FLAG_HOT_SPOT_BOILING", mask: reactor_spert3_twin::FLAG_HOT_SPOT_BOILING, seals: true },
            Advisory { name: "FLAG_PROMPT_SUPERCRITICAL", mask: reactor_spert3_twin::FLAG_PROMPT_SUPERCRITICAL, seals: true },
            Advisory { name: "FLAG_OUTSIDE_COMPARED", mask: reactor_spert3_twin::FLAG_OUTSIDE_COMPARED, seals: true },
        ],
        with_flags_words(slots(&[
            ("t", "Time since the clock's zero, the initiation of the transient, in seconds."),
            ("pos[0]", "power_mw"),
            ("pos[1]", "energy_released_mws"),
            ("pos[2]", "net_reactivity_dollar"),
            ("vel[0]", "fuel_average_k"),
            ("vel[1]", "hot_clad_k"),
            ("vel[2]", "water_k"),
            ("force_torque", "fuel_melt_margin_k"),
            ("residual", "Conservation residual."),
            ("flags", ""),
            ("proof", "Not written by the step."),
        ])),
    )
}

/// The words of the frame's flags, the distance of FLAG_OUTSIDE_COMPARED in
/// the card's own scales (`reactor_spert3_twin::COMPARED_SCALES`): one unit
/// is so many kelvin of inlet temperature, a factor in pressure, so many m/s
/// of flow, a factor in initial power and so many dollars of reactivity.
fn with_flags_words(mut frame: Vec<super::FrameSlot>) -> Vec<super::FrameSlot> {
    static WORDS: std::sync::OnceLock<String> = std::sync::OnceLock::new();
    let words: &'static str = WORDS.get_or_init(flags_words).as_str();
    for slot in frame.iter_mut().filter(|slot| slot.slot == "flags") {
        slot.holds = words;
    }
    frame
}

fn flags_words() -> String {
    let scales = reactor_spert3_twin::COMPARED_SCALES;
    format!(
        "FLAG_ROD_MOVING: the transient rod is moving. FLAG_PAST_PEAK: the power is below the run's highest. FLAG_HOT_SPOT_BOILING: the hot-spot clad is above saturation. FLAG_PROMPT_SUPERCRITICAL: net reactivity above one dollar. FLAG_OUTSIDE_COMPARED: the run is farther than one from the nearest of the 70 tested conditions, inlet temperature counted in {} K, pressure in a factor of {}, flow in {} m/s, initial power in a factor of {} and reactivity in {} dollar. All seal.",
        scales[0],
        genesis_kernel::math::powf(2.0, scales[1]),
        scales[2],
        genesis_kernel::math::powf(10.0, scales[3]),
        scales[4],
    )
}

/// One step at the ceiling, and the cold 1-dollar burst of Table III from
/// 76 degF to 3.0 s, 2 s past its peak, recorded every 100 steps.
pub(super) fn examples() -> (Value, Value) {
    let specimen = reactor_spert3_twin::ReactorSpert3Twin::e_core();
    pair_examples_recorded(
        DOMAIN,
        CARD,
        obj(&[
            ("inlet_temperature_k", scalar_value(crate::card_reactor_spert3_twin::REST_INLET_K)),
            ("system_pressure_mpa", scalar_value(crate::card_reactor_spert3_twin::REST_PRESSURE_MPA)),
            ("coolant_flow_m_per_s", scalar_value(0.0)),
            ("initial_power_mw", scalar_value(crate::card_reactor_spert3_twin::REST_POWER_MW)),
        ]),
        obj(&[("reactivity_inserted_dollar", scalar_value(1.0))]),
        super::card_dt(specimen.gate_dt_s()),
        6977,
        100,
    )
}

/// The record, in the five parts every card gives: the object, the data, the
/// rule it is held to, the result and the limits.
pub(super) fn validation(_specimen: &str) -> [String; 5] {
    [
        "The SPERT III E-core, a pressurised-water test reactor of 1392 UO2 rods (4.8 percent enriched, 10.5 g/cm3, 38.3 in active) at the National Reactor Testing Station, Idaho, measured by the Phillips Petroleum Company: power excursions from the ejection of its transient rod, cold at atmospheric pressure with no flow, and hot at 1500 psig with flow, from about 10 W, 1 MW and 19 MW.".to_string(),
        "IDO-17281 (March 1969), OSTI 4792676: Table III (30 cold-startup tests), Table V (32 hot-startup tests at 260 and 500 degF) and Table VII (5 hot-standby and 3 operating-power tests), with their standard deviations; the design data of Appendix A, the kinetics, gap, heat-source and rod data of Appendix B and Table II, and the rod worth curves of Fig. B-4, read at 37 points. Fink (2000) for UO2, IAPWS-IF97 for saturation, the NIST WebBook 10.44 MPa isobar for water. The eleven fitted numbers, as the card carries them to eight figures: the rod's delay from the clock's zero, 0.0010369315 s (1.0 ms); the rod's acceleration 2000 in/s2 times (1 + 0.024930321 p), p the system pressure in MPa; the Doppler weight F_D 0.92835769 on the average fuel temperature rise, the Doppler coefficient at 70 degF as Table II prints it (0.72 cents per degF) and at 250 and 500 degF Table II's times 1.1674972 and 1.4828376 (0.56 and 0.37 cents per degF printed); the Doppler saturation kappa = 0.26584318 times the peak-to-average power density (5.7, 5.2 and 3.6 at 70, 250 and 500 degF); the moderator weight held at 1, Table II's 0.4, 2.0 and 4.0 cents per degF as printed; the clad thermocouple's hot rod at 1.4328386 times the peak-to-average power density; free convection 3707.8868 W/m2 K added to Dittus-Boelter's; and the start powers of the startup series, 7.9949547 W cold, 4.1397337 W at 260 degF and 8.3415176 W at 500 degF. The peak rod, for the fuel temperatures and the melting stop, runs at the peak-to-average power density itself, nothing fitted. The rod's worth below x inches of travel, fitted to Fig. B-4: R(x) = s (x + x_c (exp(-x / x_c) - 1)), with s = 0.28995, 0.28964 and 0.18104 dollars per inch and x_c = 1.2363, 1.5458 and 1.9070 in at 70, 250 and 500 degF; at 1, 2, 3, 4, 6 and 8 in it gives 0.0911, 0.2925, 0.5431, 0.8154, 1.3840 and 1.9617 dollars at 70 degF, 0.0764, 0.2543, 0.4855, 0.7445, 1.2993 and 1.8719 at 250 degF, and 0.0402, 0.1378, 0.2695, 0.4213, 0.7558 and 1.1083 at 500 degF. Every quantity given at the three table temperatures is linear between them and continued along the nearer segment outside, in the inlet temperature but for the moderator coefficient, which is taken in the core water's temperature and integrated from the start's water temperature T_w0 to the water's T_w, so that within one segment it acts at the mean core water temperature, (T_w0 + T_w) / 2, and not at the inlet. Every equation and constant of the card is in equations.".to_string(),
        "Every step holds two ledgers to 1e-6: the neutron population against the Hermite integral of rho P, and the energy released against the heat held in the rings, clad and water plus the heat the flow carried out, for the core, the hot rod and the peak rod. Its accuracy is stated against the report's own standard deviations: peak power 15 percent (10 at 500 degF), energy to peak 17 (13), time to peak as tabulated, clad temperature 10 percent.".to_string(),
        RESULT.to_string(),
        "Compared from 284.8 to 540.4 K inlet, 0.0853 and 10.44 MPa, no flow to 7.32 m/s, 4e-6 to 19 MW initial power and 0.46 to 1.29 dollars; the flag is not this box but the distance from the nearest tested condition (25 K, a factor of 2 in pressure, 1.5 m/s, a factor of 10 in initial power and 0.1 dollar each count one), so a run inside the box far from every test, 350 degF at 5 MPa with no flow from 5 MW, is flagged; outside, the same equations run and FLAG_OUTSIDE_COMPARED is set. The startup tests give no initial power: the fitted 8.0, 4.1 and 8.3 W are effective values for the three series. Fuel temperature was never measured; the clad thermocouple is modelled as a hot rod at 1.433 times the peak-to-average power density, hotter than any rod of the core (2821 K at its centre at the 19 MW start), and the fuel temperatures by a peak rod at the peak-to-average itself (1942 K at test 86's start, 2316 K at most in its first second, against PARET's 2866 to 2977 K for 3.5 dollars from 20 MW in Table IX); and Table VII's tests 79, 80, 84, 85 and 86 ran on a hand-started clock (IDO-17281 p. 148); their tabulated times agree with the figures' times from the transient's start (Figs. D-63, D-64 and D-68 to D-70), within 0.012 s except test 80 (0.150 s against 0.193), and the net energies to the peak of tests 84 and 85 integrated from Figs. D-68 and D-69, 1.8 and 5.9 MW s, are 0.39 and 0.42 of the table's 4.7 and 14. The card stops by name where a peak-rod ring reaches 3120 K or the core water reaches saturation, and has no scram, void feedback or second burst.".to_string(),
    ]
}

/// Each refusal's sentence, word for word from the doc comment above the
/// variant in the card's source.
pub(super) fn meaning(name: &str) -> Option<&'static str> {
    Some(match name {
        "NonFiniteInput" => "The reactivity or `dt` was NaN or infinite.",
        "NonFiniteState" => "The incoming state, or the trial step, was not finite.",
        "NonPositiveDt" => "`dt` was zero or negative.",
        "InvalidSpecimen" => "A recorded constant is not this card's.",
        "ReactivityNegative" => "The reactivity is below zero: the transient rod is ejected, never driven in.",
        "ReactivityChangedDuringRun" => "The reactivity of the load is not the one the first step of this run sent the rod through; a run is one ejection.",
        "StateOutsideCard" => "The incoming state is not one this card writes: a power at or below zero, a temperature at or below zero, a clock below zero, or a start outside the card.",
        "DtAboveStateCeiling" => "The step is longer than 0.4 of the prompt time constant, or than the growth share of it while the prompt pole grows, at the state it starts from or lands on, and is above the card's ceiling or would be taken in pieces shorter than its floor, 1e-9 s.",
        "FuelMelting" => "A ring of the peak rod, at the core's own peak-to-average power density, reached the melting point of UO2, 3120 K.",
        "WaterAtSaturation" => "The water in the core reached its saturation temperature: bulk boiling, where the single-phase water of the card no longer holds.",
        "SpontaneousGeneration" => "The trial step created neutrons or heat from nothing.",
        "FirstLawBreach" => "The trial step destroyed neutrons or heat.",
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
/// sentence names another.
pub(super) fn equations() -> Vec<String> {
    use reactor_spert3_twin as c;
    let state_keys: Vec<String> = crate::card_reactor_spert3_twin::rest(|fields| fields.iter().map(|(name, _)| (*name).to_string()).collect());
    vec![
        format!(
            "Kinetics, six groups: dP/dt = ((rho - 1) / l*) P + sum lambda_i C_i and dC_i/dt = (b_i / l*) P - lambda_i C_i, rho the net reactivity in dollars, P and C_i in MW. b_i, fractions of the delayed fraction (sum 1), {}; lambda_i {} per s (IDO-17281 Appendix B 3.3). At the start C_i = b_i P_0 / (lambda_i l*).",
            numbers(&c::GROUP_FRACTION),
            numbers(&c::GROUP_DECAY_PER_S),
        ),
        format!(
            "Tables: T_1 = (70 + 459.67) / 1.8, T_2 = (250 + 459.67) / 1.8 and T_3 = (500 + 459.67) / 1.8 K, 70, 250 and 500 degF. A tabled value at T is v_1 + (v_2 - v_1) (T - T_1) / (T_2 - T_1) for T at or below T_2, and v_2 + (v_3 - v_2) (T - T_2) / (T_3 - T_2) above it. Tabled in the inlet temperature T_in and held for the run: l* {} s; the Doppler coefficient alpha_D 0.0072 x 1.8, (0.0056 x 1.8) x {} and (0.0037 x 1.8) x {} dollars per K (Table II's 0.72, 0.56 and 0.37 cents per degF, the hot two times the fitted scales); the peak-to-average power density PF {}; the rod worth slope s {} dollars per in and bottom length x_c {} in; the zero-power gap width {} in. Tabled in the water temperature: the moderator coefficient alpha_M {} dollars per K (0.4, 2.0 and 4.0 cents per degF).",
            numbers(&c::GENERATION_TIME_REDUCED_S),
            c::DOPPLER_SCALE_HOT[0],
            c::DOPPLER_SCALE_HOT[1],
            numbers(&c::PEAK_TO_AVERAGE),
            numbers(&c::ROD_WORTH_SLOPE_DOLLAR_PER_IN),
            numbers(&c::ROD_WORTH_BOTTOM_IN),
            numbers(&c::GAP_WIDTH_IN),
            numbers(&c::MODERATOR_COEFFICIENT_DOLLAR_PER_K),
        ),
        format!(
            "The transient rod: R(x) = s (x + x_c (e^(-x / x_c) - 1)) dollars below x inches. It starts at x_0 with R(x_0) = rho_0, the load, found by Newton's method from x = rho_0 / s + x_c while each iterate is smaller than the last, at most 200 (x_0 = 0 for rho_0 = 0). It holds until t_d = {} s, then falls as x = x_0 - a (t - t_d)^2 / 2 with a = {} (1 + {} p) in/s^2, p the system pressure in MPa, until x = 0 at t_e = t_d + sqrt(2 x_0 / a). Inserted reactivity rho_ins = 0 up to t_d, rho_0 - R(x) while the rod falls, rho_0 from t_e; its rate while it falls is s (1 - e^(-x / x_c)) a (t - t_d). rod_position_in is x_0 up to t_d and 0 from t_e.",
            c::ROD_DELAY_S,
            c::ROD_ACCELERATION_IN_PER_S2,
            c::ROD_ACCELERATION_GAIN_PER_MPA,
        ),
        format!(
            "Feedback: rho = rho_ins - rho_D - rho_M. Doppler, logarithmic in the average fuel temperature: rho_D = (F_D A / kappa) ln((Tf_0 + kappa (Tf - Tf_0)) / Tf_0), that is (F_D A / kappa) ln(1 + kappa (Tf - Tf_0) / Tf_0), with A = alpha_D(T_in) T_in, F_D = {}, kappa = {} PF(T_in), Tf = sum w_j T_j the average rod's ring-weighted fuel temperature (fuel_average_k) and Tf_0 its value at the start (start_fuel_average_k). Moderator: rho_M = W_M (I(T_w) - I(T_w0)), W_M = {}, T_w the core water (water_k), T_w0 its start (start_water_k), I(T) = alpha_M(T_2) d + m d^2 / 2 with d = T - T_2 and m the slope of alpha_M on the side of T_2 that T is on: alpha_M integrated in the water temperature, which within one segment of the table is W_M alpha_M((T_w0 + T_w) / 2) (T_w - T_w0), the coefficient at the mean core water temperature, not at the inlet. Their rates: F_D A (dTf/dt) / (Tf_0 + kappa (Tf - Tf_0)) and W_M alpha_M(T_w) dT_w/dt.",
            c::DOPPLER_WEIGHT,
            c::DOPPLER_PEAKING_SHARE,
            c::MODERATOR_WEIGHT,
        ),
        format!(
            "Core and rod: N = {} rods, active length L = 38.3 x 0.0254 m, pellet radius r_f = 0.210 x 0.0254 m, clad inner radius r_ci = 0.213 x 0.0254 m and outer radius r_co = 0.233 x 0.0254 m (0.21, 0.213 and 0.233 in); UO2 {} kg/m^3, molar mass {} kg/mol, melting at {} K; clad {} kg/m^3, {} W/(m K) and {} J/(kg K), 3.98 MJ/(m^3 K); clad-to-water surface S = 2 pi r_co L N; coolant flow area 240.9 x 0.0254^2 m^2; hydraulic diameter D_h = 0.46905 x 0.0254 m. Heat flows are in MW and capacities in MJ/K: a conductance in W/K times 1e-6, a resistance in K/W times 1e6, a capacity in J/K times 1e-6.",
            c::FUEL_RODS,
            c::PELLET_DENSITY_KG_PER_M3,
            c::UO2_MOLAR_MASS_KG_PER_MOL,
            c::UO2_MELTING_POINT_K,
            c::CLAD_DENSITY_KG_PER_M3,
            c::CLAD_CONDUCTIVITY_W_PER_M_K,
            c::CLAD_SPECIFIC_HEAT_J_PER_KG_K,
        ),
        format!(
            "Six fuel rings of equal radial thickness dr = r_f / {}, ring j = 0 to 5 from the centre, its node at (j + 1/2) dr. Ring j holds the share w_j = ((j + 1)^2 - j^2) / 36 = (2 j + 1) / 36 of the UO2 mass M_U = pi r_f^2 L 10500 N, m_j = w_j M_U, and of the fuel's heat. Ring to ring: G_j = 2 pi L N k_f((T_j + T_(j+1)) / 2) / ln((j + 1.5) / (j + 0.5)) W/K for j = 0 to 4, flow Q_j = G_j (T_j - T_(j+1)). Ring 5 to the clad's mid-wall r_m = (r_ci + r_co) / 2: Q_5 = (T_5 - T_c) / R_o, R_o = ln(6 / 5.5) / (2 pi L N k_f(T_5)) + 1 / (2 pi r_f L N h_gap) + ln(r_m / r_ci) / (2 pi L N k_c), in K/W. Rings: m_j c_f(T_j) dT_j/dt = {} w_j P + Q_(j-1) - Q_j (Q_(-1) = 0). Clad: C_c dT_c/dt = {} P + Q_5 - S q(T_c, T_w), C_c = pi (r_co^2 - r_ci^2) L N {} x {}. Water: C_w dT_w/dt = {} P + S q(T_c, T_w) - C_w (2 v / L) (T_w - T_in), C_w = rho_w c_w (flow area) L x 1e-3 MJ/K with c_w in kJ/(kg K), v the coolant flow. The fission heat split is {} in the fuel, {} in the clad and {} directly in the water. fuel_average_k = sum w_j T_j; hot_fuel_centre_k is the hot rod's ring 0, its node at dr / 2, with no extrapolation to the centre; fuel_melt_margin_k = {} - max(hot ring 0, hot ring 1).",
            c::RINGS,
            c::FUEL_HEATING_FRACTION,
            c::CLAD_HEATING_FRACTION,
            c::CLAD_DENSITY_KG_PER_M3,
            c::CLAD_SPECIFIC_HEAT_J_PER_KG_K,
            c::WATER_HEATING_FRACTION,
            c::FUEL_HEATING_FRACTION,
            c::CLAD_HEATING_FRACTION,
            c::WATER_HEATING_FRACTION,
            c::UO2_MELTING_POINT_K,
        ),
        format!(
            "The hot rod: the same six rings and clad at F_hot = {} PF(T_in) times the power, cooled by the same water at T_w; its heat to the water goes to its own account, hot_heat_to_water_mws, and not into the water's equation.",
            c::HOT_SPOT_SHARE,
        ),
        format!(
            "UO2, Fink (2000), 95 percent dense: heat capacity c_f = (C1 x^2 e^x / (e^x - 1)^2 + 2 C2 T + C3 Ea e^(-Ea / T) / T^2) / M J/(kg K), x = theta / T; enthalpy h_f = (C1 theta / (e^(theta / T) - 1) + C2 T^2 + C3 e^(-Ea / T)) / M J/kg; C1 = {}, theta = {} K, C2 = {}, C3 = {}, Ea = {} K, M = {} kg/mol. Conductivity k_f = 100 / (7.5408 + 17.692 t + 3.6142 t^2) + 6400 / (t^2 sqrt(t)) e^(-16.35 / t) W/(m K), t = T / 1000.",
            c::FINK_C1,
            c::FINK_THETA_K,
            c::FINK_C2,
            c::FINK_C3,
            c::FINK_EA_K,
            c::UO2_MOLAR_MASS_KG_PER_MOL,
        ),
        format!(
            "The helium gap: width w = w_0(T_in) e^(-P_0 / {} MW) in, held for the run, w_0 the tabled zero-power width ({} in) and P_0 the initial power: the closure law, 1 / ln(4/3) MW, so that 1 MW at 500 degF gives the report's 0.0015 in. Conductance (IDO-17281 Eq. B-23) h_gap = k_g x 1.73073466637139 / ((w + {}) x 0.0254) W/(m^2 K), k_g = (6.43 + 7.43e-3 T_F - 1.07e-6 T_F^2) / (22.56 + 2.54e4 w) Btu/(h ft degF), T_F = 1.8 T_g - 459.67 at the gap temperature T_g = (T_5 + T_c) / 2.",
            c::GAP_CLOSING_POWER_MW,
            numbers(&c::GAP_WIDTH_IN),
            c::GAP_JUMP_IN,
        ),
        format!(
            "Water, the NIST WebBook isobar at 10.44 MPa fitted from 290 to 540 K, polynomials in x = (T_in - 400) / 100, lowest power first, taken at the inlet temperature and held for the run: density {} kg/m^3; specific heat {} kJ/(kg K); natural log of the viscosity in micro-Pa s {}; conductivity {} W/(m K). The clad-to-water flux q(T_c, T_w) = h (T_c - T_w) + 1e6 ((T_c - T_sat) e^(p / {} MPa) / {} K)^4 W/m^2, the second term (Jens-Lottes nucleate boiling) only when T_c is above T_sat. h = 0.023 Re^0.8 Pr^0.4 k_w / D_h + {} W/(m^2 K), Re = rho_w v D_h / mu, Pr = 1000 c_w mu / k_w, mu = e^(ln mu) x 1e-6 Pa s; with no flow h is {} alone.",
            numbers(&c::WATER_DENSITY_POLY),
            numbers(&c::WATER_SPECIFIC_HEAT_POLY),
            numbers(&c::WATER_LOG_VISCOSITY_POLY),
            numbers(&c::WATER_CONDUCTIVITY_POLY),
            c::JENS_LOTTES_PRESSURE_MPA,
            c::JENS_LOTTES_K,
            c::NATURAL_CONVECTION_W_PER_M2_K,
            c::NATURAL_CONVECTION_W_PER_M2_K,
        ),
        format!(
            "Saturation, IAPWS-IF97 region 4 backward: beta = sqrt(sqrt(p)), p in MPa; E = beta^2 + n3 beta + n6, F = n1 beta^2 + n4 beta + n7, G = n2 beta^2 + n5 beta + n8, D = 2 G / (-F - sqrt(F^2 - 4 E G)), T_sat = (n10 + D - sqrt((n10 + D)^2 - 4 (n9 + n10 D))) / 2 K, with n1 to n10 = {}.",
            numbers(&c::IF97_N),
        ),
        format!(
            "The start: the steady reactor at P_0. Water T_w0 = T_in + P_0 / (C_w 2 v / L) with flow and T_in with none, which must be below T_sat. Each rod's clad: q(T_c, T_w0) = ({} + {}) P_rod x 1e6 / S W/m^2 solved by bisection between T_w0 and T_w0 + ({} + {}) P_rod x 1e6 / (S h), lower end raised where q at the midpoint is below it, at most 300 halvings, stopping when the midpoint no longer lies strictly between the ends, T_c the last midpoint; then T_5 = T_c + {} P_rod R_o(T_5, T_c) iterated from T_5 = T_c until it repeats (at most 200), and for j = 4 down to 0 T_j = T_(j+1) + {} P_rod ((j + 1)^2 / 36) / G_j iterated from T_j = T_(j+1) likewise. P_rod is P_0 for the average rod and F_hot P_0 for the hot rod. hot_clad_max_k starts at the hot rod's steady clad.",
            c::FUEL_HEATING_FRACTION,
            c::CLAD_HEATING_FRACTION,
            c::FUEL_HEATING_FRACTION,
            c::CLAD_HEATING_FRACTION,
            c::FUEL_HEATING_FRACTION,
            c::FUEL_HEATING_FRACTION,
        ),
        "The step: the classical fourth-order Runge-Kutta rule on the 25 values P, C_1 to C_6, the six average rings, the clad, the six hot rings, the hot clad, T_w, E, Q_out and Q_hot, with dE/dt = P, dQ_out/dt = C_w (2 v / L) (T_w - T_in) (heat_carried_out_mws) and dQ_hot/dt = S q(T_hot clad, T_w) (hot_heat_to_water_mws), every temperature-dependent property taken at each stage's own temperatures and the rod at each stage's own time. A step that contains t_d or t_e is taken as Runge-Kutta parts split there. Over each part the integral W of rho P is h (f_0 + f_1) / 2 + h^2 (f'_0 - f'_1) / 12, f = rho P and f' = (d rho / dt) P + rho dP/dt at the part's two ends.".to_string(),
        format!(
            "residual_conservation_error = max(|d_N| / max(N, N_0), |d_E| / max(E, K), |d_H| / max(F_hot E, K)), K = C_w + C_c + 0.3 MJ/K, about one kelvin of the core. Neutrons: N = P + sum C_i (C_i is precursor_i_mw), N_0 its start value (population_anchor_mw), W the booked integral of rho P (reactivity_power_integral), d_N = N - N_0 - W / l* (population_defect_mw). Core heat: H = sum m_j h_f(T_j) 1e-6 + C_c T_c + C_w T_w MJ (T_j is fuel_ring_j_k, T_c clad_k), H_0 at the start (heat_anchor_mws), d_E = E - (H - H_0 + Q_out) (energy_defect_mws). Hot rod: H_hot = sum m_j h_f(T_hot j) 1e-6 + C_c T_hot c (hot_fuel_ring_j_k and hot_clad_k), H_hot 0 at the start (hot_heat_anchor_mws), d_H = F_hot ({} + {}) E - (H_hot - H_hot 0) - Q_hot (hot_energy_defect_mws). A step past {:e} is refused as SpontaneousGeneration when d_N > 0, d_E < 0 or d_H < 0, else as FirstLawBreach.",
            c::FUEL_HEATING_FRACTION,
            c::CLAD_HEATING_FRACTION,
            c::RESIDUAL_CONSERVATION_LIMIT,
        ),
        "The run's records: peak_power_mw starts at P_0 with time_of_peak_s = 0 and energy_to_peak_mws = 0, and a landed step with P strictly above peak_power_mw sets all three to its P, t and E; a landed step with the hot clad strictly above hot_clad_max_k sets it and time_of_hot_clad_max_s. past_peak is P below peak_power_mw, rod_moving t between t_d and t_e, hot_spot_boiling the hot clad above T_sat (saturation_k), prompt_supercritical rho above 1; hot_clad_heat_flux_mw_per_m2 is q(T_hot clad, T_w) x 1e-6; inserted_reactivity_dollar, doppler_dollar, moderator_dollar and net_reactivity_dollar are rho_ins, rho_D, rho_M and rho.".to_string(),
        format!("The state's keys, in the order the engine writes them: {}.", state_keys.join(", ")),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The served words quote the card's own constants: the flags frame the
    /// scales of FLAG_OUTSIDE_COMPARED, and the reactivity's bound the rod's
    /// acceleration gain and delay (0.15.1; 0.15.0 quoted 0.14.3's).
    #[test]
    fn the_flags_and_the_rod_words_are_the_card_s_constants() {
        let flags = flags_words();
        assert!(flags.contains("inlet temperature counted in 25 K, pressure in a factor of 2, flow in 1.5 m/s, initial power in a factor of 10 and reactivity in 0.1 dollar."), "{flags}");
        assert_eq!(reactor_spert3_twin::COMPARED_SCALES, [25.0, 1.0, 1.5, 1.0, 0.1]);
        let card = card();
        let frame: Vec<&str> = card.slots.iter().filter(|slot| slot.slot == "flags").map(|slot| slot.holds).collect();
        assert_eq!(frame, vec![flags.as_str()]);
        let words = card.fields.iter().map(|field| field.bound.clone()).collect::<Vec<_>>().join(" | ");
        assert!(words.contains("falls at 2000 in/s^2 (1 + 0.024930321 p), p the system pressure in MPa, from 1.0 ms after the clock's zero."), "{words}");
        assert!(!words.contains("0.0219966") && !words.contains("2.46 ms"), "{words}");
    }
}
