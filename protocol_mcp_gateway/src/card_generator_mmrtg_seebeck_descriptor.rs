//! What an agent reads of the generator_mmrtg_seebeck card: its fields and
//! bounds, its step floor and ceiling, its refusals word for word from the
//! card's source, its frame, its worked examples and its record. Stamped
//! from the kit; a child module of `descriptor.rs`, so it uses that file's
//! own builders.
//!
//! The card is the second form of the generator beside generator_mmrtg_f1:
//! its constants are fitted on every published day, 0 to 4,087, and past
//! that day it is a forecast committed before the data was opened.

use genesis_kernel::generator_mmrtg_seebeck;
use serde_json::Value;

use super::{Advisory, Card, Role, clock, finish, obj, pair_examples_of, scalar, scalar_value, slots, token};

const DOMAIN: &str = "energy";
const CARD: &str = "generator_mmrtg_seebeck";

/// What the card is, in one sentence of the list.
const SPECIMEN: &str = "One radioisotope generator, flight unit F1, on the Curiosity rover, in the Seebeck matched-load form, the companion of generator_mmrtg_f1: its daily-average electrical output from landing on, with no load, the fuel's decay entering twice. Calibrated to the record and not a blind test: the form was preferred after generator_mmrtg_f1 missed its final-year line on the held-back days 2,809 to 4,087, and its three constants were fitted on every published day, 0 to 4,087, where it reads within 0.00319 of 115.4 W by root mean square, in sample. Past day 4,087 the card is the blind forecast registered before any later day's value was published as a number, and the frame sets FLAG_PAST_FITTING_SPAN and FLAG_PAST_RECORD.";

/// The result, its first sentence the word it is served under.
const RESULT: &str = "On mission days 0 to 4,087, all in the fit, run through the engine in daily steps from landing: RMS error 0.00319 of 115.4 W, about 0.37 W; on days 2,809 to 4,087, 0.00397 RMS and a mean of +0.00132 over the final 365 days; largest error on one day +0.0160, 1.85 W, on mission day 3,546, the day of generator_mmrtg_f1's largest, +0.0225. The same form with its constants fitted on days 0 to 2,060 alone (a = 0.995915, c = 0.217256, m = 21.1586): on days 2,809 to 4,087 an RMS error of 0.00507 and a final-365 mean of +0.00468, where generator_mmrtg_f1 gives 0.00813 and +0.010094, 0.000094 over its target of 0.010; on Perseverance's generator F2, its days 0 to 985, 0.00572 and +0.00187, where generator_mmrtg_f1 gives 0.00573 and +0.00190. The forecast, the card as served at gateway commit a0aff5e, for the dataset's next release: 0.677319 of 115.4 W on day 4,088, 0.661798 on day 4,452, 0.647187 on day 4,818 and 0.633481 on day 5,183, 2026-10-15.";

/// What the card does not show, and where its numbers stop.
const LIMITS: &str = "One generator; the daily-average electrical output; the record holds no heat measurement. The thermal power at landing, 1953.2 W, is a pre-flight estimate, and the fuel, the heat, the clock and the ledger are generator_mmrtg_f1's number for number. The record cannot tell the fuel's share of the loss from the converter's: this form gives the fuel's decay twice the weight generator_mmrtg_f1's form gives it, on the reasoning that the thermocouples' temperature drop follows the heat and their output into a matched load its square, and the fit decides the rest; the record does not separate the two. Its form was chosen with all 4,088 days read, generator_mmrtg_f1's comparison on days 2,809 to 4,087 among them; its three constants are fitted on all 4,088. generator_mmrtg_f1 is the form compared on days 2,809 to 4,087 with its constants fitted on days 0 to 2,060. Refitted with any one Mars year left out of the fit, the forecast is 0.6602 to 0.6662 of 115.4 W on day 4,452 and 0.6308 to 0.6390 on day 5,183. NASA/TM-20250004743 plots this generator to about mission day 4,341 and prints no number of it; the forecast uses no value of that plot. Past day 4,087 the card carries the same exact map and sets FLAG_PAST_FITTING_SPAN and FLAG_PAST_RECORD on each step: those are the forecast's days, compared when the dataset's next release gives 365 of them or more.";

pub(super) fn card() -> Card {
    let unit = generator_mmrtg_seebeck::GeneratorMmrtgSeebeck::curiosity();
    let (gate, nyquist) = clock(unit.combined_rate_per_s, unit.gate_dt_s(), None);
    let last_s = generator_mmrtg_seebeck::MISSION_TIME_MAX_S;
    let ledger_end_s = unit.ledger_end_mission_time_s().expect("ledger end");
    let floor = unit.floor_dt_s().expect("specimen floor");
    finish(
        DOMAIN,
        CARD,
        SPECIMEN,
        vec![scalar(
            "mission_time_s",
            Role::Phase,
            "s",
            Some(0.0),
            false,
            Some(ledger_end_s),
            false,
            format!(
                "inclusive [0, {}] s. The maximum is where the decay heat the fuel still holds falls to the ledger's resolution, 1e-6 of the heat released from landing to mission day 4,087, about 2,060 years after landing: past it a breach of the ledger the size of its limit could not be seen (DecayHeatBelowLedgerResolution). The published record, and the days the constants are fitted on, end on mission day 4,087 ({} s); a later start is answered with FLAG_PAST_FITTING_SPAN and FLAG_PAST_RECORD, on the days of the forecast. The returned state gives the mission time as t_s",
                token(ledger_end_s),
                token(last_s)
            ),
        )],
        floor,
        "the ceiling divided by 2^20: about one part in a thousand million of thermal power. Under it a step is mostly rounding",
        gate,
        "0.02 rad on its fastest rate, the fading part's output: twice the fuel's decay plus the converter's wear",
        nyquist,
        "No load. A run segment carries an empty load object. The electrical power is (Q^2 / Q0)(eta_s + eta_f g): the thermal power Q decays with the fuel at ln 2 / 87.7 per year, the thermocouples' temperature drop follows it, so the efficiency falls with it, and the wearing share g of the converter's output falls at 18.777 times the fuel's rate; each part's electrical energy over a step is its exact integral, and the heat rejected the rest of the decay heat released. Mission days 0 to 4,087, every published day, are the days the constants are fitted on. Mission day 4,087 is the last day of the published record and not an end of the generator: a step that lands past it by more than the clock's tolerance lands with FLAG_PAST_FITTING_SPAN and FLAG_PAST_RECORD set, the same exact map carried forward, and those days are the forecast committed before the data was opened. The card stops where the decay heat still held falls under the ledger's resolution, about 2,060 years after landing, as DecayHeatBelowLedgerResolution, and before landing as MissionTimeBeforeLanding. The mission clock is a compensated sum: each step adds dt_s to t_s in 64-bit arithmetic, keeps what the addition rounded off and folds it back in, so t_s stays within half a spacing of 64-bit numbers of the exact sum of the start and the steps at any length of run. Every step, the first included, compares its landing with mission day 4,087 plus one tolerance, MISSION_END_TOLERANCE_S, 2^-22 s or about 2.38e-7 s: four spacings of 64-bit numbers between 2^28 s and 2^29 s, where the last day lies, enough for a start and a step read from decimals and for the clock's own rounding. A run or a single step whose end in decimal is exactly mission day 4,087 lands its last step with neither flag, and range_margin_s, the time left to the last day of the record, is zero on and past it. A restart from a returned t_s begins with no carry. electrical_energy_j, heat_rejected_j and fading_electrical_energy_j count from the start of this run, the anchor of the ledger, and not from landing: energy_anchor_j is the decay heat held at that start. A restart from a returned state starts each again at zero. No refusal depends on these totals: every limit of the card is on the mission time, the heat still held or on a single step's ledgers".into(),
        &[
            "NonFiniteInput",
            "NonFiniteState",
            "NonPositiveDt",
            "InvalidGenerator",
            "MissionTimeBeforeLanding",
            "DecayHeatBelowLedgerResolution",
            "SpontaneousGeneration",
            "FirstLawBreach",
            "ConverterAccountBreach",
        ],
        vec![
            Advisory {
                name: "FLAG_PAST_FITTING_SPAN",
                mask: generator_mmrtg_seebeck::FLAG_PAST_FITTING_SPAN,
                seals: true,
            },
            Advisory {
                name: "FLAG_PAST_RECORD",
                mask: generator_mmrtg_seebeck::FLAG_PAST_RECORD,
                seals: true,
            },
        ],
        slots(&[
            ("t", "Mission time since landing, in seconds."),
            ("pos[0]", "electrical_power_w"),
            ("pos[1]", "thermal_power_w"),
            ("pos[2]", "converter_fading_fraction"),
            ("vel[0]", "electrical_energy_j"),
            ("vel[1]", "heat_rejected_j"),
            ("vel[2]", "fuel_heat_j"),
            ("force_torque", "Range margin, in seconds: the time left to mission day 4,087, the last day of the record; zero on and past it."),
            ("residual", "Conservation residual."),
            (
                "flags",
                "FLAG_PAST_FITTING_SPAN and FLAG_PAST_RECORD together after mission day 4,087, the last day of the published record and of the days the constants were fitted on. Both seal.",
            ),
            ("proof", "Not written by the step."),
        ]),
    )
}

/// One day from landing, and thirty daily steps from landing, as
/// generator_mmrtg_f1's examples are.
pub(super) fn examples() -> (Value, Value) {
    let unit = generator_mmrtg_seebeck::GeneratorMmrtgSeebeck::curiosity();
    let state = generator_mmrtg_seebeck::GeneratorMmrtgSeebeckState::try_landing(&unit).expect("landing");
    pair_examples_of(
        DOMAIN,
        CARD,
        obj(&[("mission_time_s", scalar_value(state.t_s))]),
        obj(&[]),
        scalar_value(86_400.0),
        30,
    )
}

/// The record, in the five parts every card gives: the object, the data, the
/// pass line, the result and the limits.
pub(super) fn validation(_specimen: &str) -> [String; 5] {
    [
        "One radioisotope generator, flight unit F1, on NASA's Curiosity rover: its daily-average electrical output, in the Seebeck matched-load form, the second form of the generator beside generator_mmrtg_f1.".into(),
        "Historical RTG Performance Data, 2023 update, Dryad, doi 10.5061/dryad.1zcrjdfw2, version 6: the daily-average electrical output on mission days 0 to 4,087, each as a fraction of 115.4 W, the highest daily average of the first 30 days. All 4,088 days are in the fit: three constants, a = 0.99401, c = 0.230509 and m = 18.777, by ordinary least squares, each day weighted equally, rounded to six significant figures; with the fuel's decay, ln 2 / 87.7 per year, they are the forecast's own. Outside the fit: the days from 4,088 on, in the dataset's next release.".into(),
        "Error measures and targets, for the days from 4,088 on: the RMS error of the daily values and the mean error over the final 365 days, each at most 0.010 of 115.4 W, 1.154 W, on 365 days or more of the dataset's next release (tier A); the same two on 30 to 364 days (tier B); from a source with fewer days, three or more means over stated periods of 7 days or more (tier C), or single points (tier D). No value read off a plot is used. Days 0 to 4,087 are all in the fit and carry no target. Forecast committed before the data was opened, as 0afc18e0e7af9b4405b4ce5b16764226ee917774d72253a1590cc179e76d86d1.".into(),
        RESULT.into(),
        LIMITS.into(),
    ]
}

/// Each refusal's sentence, word for word from the doc comment above the
/// variant in the card's source.
pub(super) fn meaning(name: &str) -> Option<&'static str> {
    Some(match name {
        "NonFiniteInput" => "`dt` was NaN or infinite.",
        "NonFiniteState" => "The incoming state, or the trial step, was non-finite.",
        "NonPositiveDt" => "`dt` was zero or negative.",
        "InvalidGenerator" => "A recorded constant is not this generator's.",
        "MissionTimeBeforeLanding" => "Mission time, incoming or landing, is before landing on Mars, where the card's thermal power and converter are anchored.",
        "DecayHeatBelowLedgerResolution" => "The decay heat the fuel still holds, incoming or landing, is under the ledger's resolution, 1e-6 of the decay heat released from landing to mission day 4,087 (about 6.6e5 J, some 2,070 years after landing): a breach of that size could no longer be seen.",
        "SpontaneousGeneration" => "The trial step created energy from nothing.",
        "FirstLawBreach" => "The trial step destroyed energy.",
        "ConverterAccountBreach" => "The trial step broke the fitted model's account of the fading part of the converter, which is bookkeeping that follows from the model and not a conservation law.",
        _ => return None,
    })
}
