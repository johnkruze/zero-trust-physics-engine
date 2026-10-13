//! The entry for `culture_trichoderma_ambr250`: one fed-batch culture of
//! Trichoderma sp. VTT D-221704 in an ambr 250 vessel. The numbers come from
//! `genesis_solvers` constants.

use genesis_solvers::culture_trichoderma_ambr250::{self as culture, Culture, CultureReject, Question};
use serde_json::{Value, json};

use crate::solvers::{
    Answer, Given, Input, Kind, Output, Published, Refusal, Refused, Sections, Shape, Solved,
    Solver,
};

/// What has been compared with measurement, and how it came out.
pub const COMPARED: &str = "Compared once, on 2026-10-09, through the gateway, with the four held-back cultures, and it did not pass: three of the six lines were met and three were missed. Missed: galacturonate, root-mean-square error 4.60 g/L against 2.74 allowed; mucic acid, 6.45 g/L against 4.51; carbon dioxide made, 0.01125 mol against 0.00265. Met: dry biomass to 170 h, 1.05 g/L against 2.15; glucose, 0.51 g/L against 0.85; oxygen taken up, 0.0108 mol against 0.0153. Of the three met, only the oxygen line is evidence for the model. The mean of the two fitted cultures at the same yeast extract scores 1.02 g/L on biomass and 0.35 g/L on glucose and meets both lines, and is closer than the solver on both: in these cultures the biomass and the glucose do not change enough with the glucose of the feed for the test to see. On oxygen that mean scores 0.0190 mol and the mean of all eight fitted cultures 0.0180 mol, and both miss. On the missed lines the same-yeast-extract mean scores 6.20 g/L, 6.46 g/L and 0.0139 mol and the mean of all eight 5.83 g/L, 7.23 g/L and 0.0102 mol: the solver is closer than both on galacturonate, level with the better one on mucic acid, and behind the mean of all eight on carbon dioxide. The starting value held scores 6.49, 19.57, 5.93 and 21.39 g/L, 0.0693 and 0.0819 mol; the same equations with constants not taken from this data score 5.14, 3.85, 8.84 and 12.17 g/L, 0.0303 and 0.0361 mol. On the eight cultures it was fitted to the solver gives 1.44, 1.38, 2.02 and 3.67 g/L, and on seven of them 0.0088 and 0.0111 mol of gas: T2 pH 4 (2) has a blank online volume, so it is not scored on the gases. It therefore misses the glucose and the carbon dioxide lines on its own fitted data. What it gets wrong: it makes mucic acid too fast in all four held-back cultures, by 4.4 to 6.8 g/L on average. Fed a third of the glucose, the real cultures left galacturonate unconverted, 18 g/L at 212 h against 8 to 13 g/L here, and burnt it after the feed stopped, making 0.046 and 0.049 mol of carbon dioxide in the last hundred hours where the solver makes 0.009: in the solver galacturonate uptake does not depend on the glucose supplied, and in the organism it does. The forecast written beforehand expected the carbon dioxide and the glucose lines missed; the carbon dioxide line was missed, the glucose line was met, and the galacturonate and mucic acid misses were not forecast. The measurements of the held-back cultures close carbon to between 0.76 and 1.05 of what was supplied between 40 and 215 h, and to between 0.98 and 1.12 at the last sample. All four answers agree with a recomputation in 50-digit arithmetic within 2e-10 of each number's scale. The binary that made the scored run (SHA-256 572ce3bedbb195b637045a60c5e2452f5b939315fc6fa699a9545949c5c63f71) is kept. The scoring was done by the agent that built the solver. Second scoring, 2026-10-09 (evidence/checks/second_party/V-41): every number confirmed, the same six verdicts. What it adds. The form fails, not the constants: the same equations refitted to all twelve cultures after the fact score 4.30 g/L, 5.21 g/L and 0.0108 mol and miss the same three lines. The eight fitted cultures were fed one glucose level, 29.375 to 34.676 g/L, and one level cannot identify any dependence of galacturonate oxidation on the glucose supplied: forms that carry such a term, fitted to the eight, return the term as zero. The carbon dioxide line was out of reach: the measurements themselves do not close carbon, so a model that returned the four liquid measurements exactly and closed carbon would be 0.013 to 0.016 mol out in carbon dioxide, five to six times the line of 0.00265; that miss says more about the data than about the solver, and what carries the verdict is galacturonate and mucic acid. The two repeats of each held-back condition differ from each other by 3.33 g/L in galacturonate and 0.0063 mol in carbon dioxide, against 1.82 g/L and 0.0018 mol between the fitted repeats from which the lines were drawn. The mucic acid column: the workbook's own description says it \"includes that removed in samples\" and has been \"adjusted\" for evaporation and sample removal, and the solver's number is the concentration in the broth; read the other way, as everything made over the volume present, the held-back error is 10.6 g/L in place of 6.45. Which reading is right cannot be settled from the file, and the line is missed on both. The starting biomass given to both held-back cultures of block (1) is one reading, 0.9333 g/L, of one fitted culture, T2 pH 4 (1): the glucose line is met with that start from 0.775 to 1.05 g/L and missed at 0.60. The first 45 h are the batch, the same medium in all twelve cultures, and each held-back culture shares its inoculum and its day with four fitted ones, so dry biomass and glucose are not tested by the held-back cultures. What the solver refuses since that scoring, set with the held-back cultures seen: a feed outside the eight fitted cultures' 29.375 to 34.676 g/L glucose (FeedGlucoseOutOfRange) and a record that ends after 215 h (RunTooLong). Since 0.7.0 it also refuses an operating record that none of the eight fitted cultures ran: a feed before 21.18 h, started faster than 0.2234 or slower than 0.0913 mL/h, faster than 0.331 mL/h, or cut back below 0.243 mL/h or stopped from 45.19 h (FeedScheduleOutOfRange, FeedRateOutOfRange), water outside -0.296 to 0.19 mL/h (WaterRateOutOfRange), a sample above 4.3 mL (SampleOutOfRange) and broth outside 98.0 to 127.7 mL (VolumeOutOfRange). These are the fitted cultures' own spans, and nothing outside them was compared with measurement. The four held-back questions, fed 10.077, 11.06, 48.32 and 52.101 g/L and ending at 307 and 312 h, are therefore refused by this build; the registered result above stays the record, made by the kept binary. The solver stays served as a solver that failed its comparison.";

const fn start(
    name: &'static str,
    minimum: f64,
    maximum: f64,
    refusal: &'static str,
    bound: &'static str,
) -> Input {
    Input {
        name,
        kind: Kind::Number { minimum, maximum, unit: "g/L" },
        refusal: Some(refusal),
        bound,
        only_when: None,
        asked_by_check: false,
    }
}

/// One array of the operating record, one number for each segment.
const fn segments(
    name: &'static str,
    minimum: f64,
    maximum: f64,
    unit: &'static str,
    refusal: &'static str,
    bound: &'static str,
) -> Input {
    Input {
        name,
        kind: Kind::Numbers {
            fewest: culture::SEGMENTS_MIN,
            most: culture::SEGMENTS_MAX,
            minimum,
            maximum,
            unit,
        },
        refusal: Some(refusal),
        bound,
        only_when: None,
        asked_by_check: true,
    }
}

const fn one(name: &'static str, unit: &'static str, note: &'static str) -> Output {
    Output { name, unit, shape: Shape::Number, each: &[], note, asked_by: None }
}

const fn each_segment(name: &'static str, unit: &'static str, note: &'static str) -> Output {
    Output { name, unit, shape: Shape::Numbers, each: &[], note, asked_by: None }
}

pub const CULTURE_TRICHODERMA_AMBR250: Solver = Solver {
    name: "culture_trichoderma_ambr250",
    summary: SUMMARY,
    inputs: &[
        start(
            "biomass_g_per_l",
            culture::BIOMASS_MIN_G_PER_L,
            culture::BIOMASS_MAX_G_PER_L,
            "BiomassOutOfRange",
            "Dry biomass at inoculation: 0.600 to 1.034 g/L, the starts the eight fitted cultures were asked with (0.600 to 1.0333), rounded outward to the thousandth. Outside it the solver refuses as BiomassOutOfRange. Until 0.7.0 it took 0.3 to 1.5 g/L.",
        ),
        start(
            "glucose_g_per_l",
            culture::GLUCOSE_MIN_G_PER_L,
            culture::GLUCOSE_MAX_G_PER_L,
            "GlucoseOutOfRange",
            "Glucose at inoculation, the batch medium of the specimen: 20.097 to 21.025 g/L, the starts the eight fitted cultures were asked with. Outside it the solver refuses as GlucoseOutOfRange. Until 0.7.0 it took 19 to 22 g/L.",
        ),
        start(
            "galacturonate_g_per_l",
            culture::GALACTURONATE_MIN_G_PER_L,
            culture::GALACTURONATE_MAX_G_PER_L,
            "GalacturonateOutOfRange",
            "D-galacturonate at inoculation, the batch medium of the specimen: 4.146 to 4.745 g/L, the starts the eight fitted cultures were asked with (4.146 to 4.7445), rounded outward to the thousandth. Outside it the solver refuses as GalacturonateOutOfRange. Until 0.7.0 it took 3.5 to 5.5 g/L.",
        ),
        start(
            "feed_glucose_g_per_l",
            culture::FEED_GLUCOSE_MIN_G_PER_L,
            culture::FEED_GLUCOSE_MAX_G_PER_L,
            "FeedGlucoseOutOfRange",
            "Glucose in the feed, one feed for the whole record: no measured culture changed its feed. The range is the feeds of the eight cultures the constants were fitted to, 29.375 to 34.676 g/L, both ends accepted; the record rounds them to 29.4 to 34.7. Outside it the solver refuses: on the held-back cultures, fed 10.1 to 11.1 and 48.3 to 52.1 g/L, galacturonate and mucic acid were wrong by 6 to 8 g/L from about 60 h on. The range was 9 to 54 g/L for that scored run.",
        ),
        start(
            "feed_galacturonate_g_per_l",
            culture::FEED_GALACTURONATE_MIN_G_PER_L,
            culture::FEED_GALACTURONATE_MAX_G_PER_L,
            "FeedGalacturonateOutOfRange",
            "D-galacturonate in the feed, one feed for the whole record. The constants were fitted at 88.5 to 103.3 g/L.",
        ),
        segments(
            "segment_end_h",
            0.0,
            culture::RUN_END_MAX_H,
            "h",
            "RunTooLong",
            "End of each segment of the operating record, hours after inoculation, rising. A segment is 1 h to 100 h long and the record ends by 215 h; a record that ends exactly at 215 h keeps its last segment. The culture is returned at each end. 215 h is the end of what was compared with measurement for mucic acid, carbon dioxide and oxygen: after it mucic acid misses its line on the fitted cultures themselves. The fitted cultures were fed until near 250 h, so a culture left unfed, or fed below the fitted cultures' rates, before 215 h was never compared, and feed_ml refuses it. The limit was 320 h for the scored run.",
        ),
        segments(
            "feed_ml",
            0.0,
            culture::FEED_RATE_MAX_ML_PER_H * culture::SEGMENT_MAX_H,
            "mL",
            "FeedRateOutOfRange",
            "Feed pumped in during each segment, at an even rate over the segment. The eight fitted cultures were unfed until 21.18 or 22.87 h, fed 0.0914 to 0.2234 mL/h in the segment where the feed began, and 0.2432 to 0.3301 mL/h in every later segment to 211.28 or 213.18 h; none went unfed, below 0.2432 mL/h or above 0.3301 mL/h in those segments once its feed had begun. The rate must lie inside that schedule at every hour of its segment: no feed in a segment that starts before 21.18 h; at most 0.2234 mL/h in a segment that starts before 43.45 h and at most 0.331 mL/h in any segment; at least 0.0913 mL/h in a segment that ends after 22.87 h and at least 0.243 mL/h in one that ends after 45.19 h. A segment that starts before 43.45 h and ends after 45.19 h therefore cannot be fed and is refused: end a segment between those hours, as every fitted culture did. A rate above 0.331 mL/h is FeedRateOutOfRange and any other feed outside the schedule is FeedScheduleOutOfRange. The fastest feed was 0.4 mL/h until 0.7.0.",
        ),
        segments(
            "water_ml",
            culture::WATER_RATE_MIN_ML_PER_H * culture::SEGMENT_MAX_H,
            culture::WATER_RATE_MAX_ML_PER_H * culture::SEGMENT_MAX_H,
            "mL",
            "WaterRateOutOfRange",
            "Water gained during each segment, at an even rate from -0.296 to 0.19 mL/h: alkali and acid in, evaporation out as a negative number. Give 0 for none. The fitted cultures' segments ran from -0.2956 to 0.1894 mL/h, and nothing outside that was compared; the range was -0.4 to 0.3 mL/h until 0.7.0.",
        ),
        segments(
            "sample_ml",
            0.0,
            culture::SAMPLE_MAX_ML,
            "mL",
            "SampleOutOfRange",
            "Broth drawn at the end of each segment, at once, at most 4.3 mL, the sample of every fitted culture. Give 0 for none, as a caller does to split a segment: a sample takes broth as it is and leaves the concentrations as they are. The broth must stay between 98.0 and 122.15 mL at each segment end and after each sample (VolumeOutOfRange), the fitted cultures' own 98.03 to 122.15 mL by 215 h. Until 0.7.0 a sample could be 5 mL and the broth 90 to 135 mL.",
        ),
    ],
    kept_inside: "The organism's constants, the reactions, the 110 mL charge, pH 4.0 and 35 °C, the step and the tolerances stay inside the solver.",
    result: &[
        each_segment("biomass_g_per_l", "g/L", "Dry biomass at the end of each segment, before its sample. Compared with measurement to 170 h only: later the measured dry weight includes mucic acid crystals, 9 to 21 g/L measured where the solver has 4 to 10, so from 170 h to the record's end at 215 h this number is cells alone and is not a quantity that was compared. To 170 h: 1.44 g/L root mean square on the fitted cultures and 1.05 on the held-back, against a line of 2.15, met; a mean curve of the fitted cultures meets it as well."),
        each_segment("glucose_g_per_l", "g/L", "Glucose at the end of each segment. Compared over the whole record: 1.38 g/L root mean square on the fitted cultures, over the line of 0.85, and 0.51 on the held-back, met. The miss is the end of the batch, before 60 h, which the solver mistimes, worst with little or no yeast extract; the batch timing is for 1 to 2 g/L yeast extract, which is not an input."),
        each_segment("galacturonate_g_per_l", "g/L", "D-galacturonate at the end of each segment. Compared over the fitted cultures' whole records: 2.02 g/L root mean square against a line of 2.74, met, though 3.61, over the line, between 170 and 215 h in the cultures with 1 and 2 g/L yeast extract. On the held-back cultures, outside the feed range the solver now takes, 4.60, missed."),
        each_segment("mucic_acid_g_per_l", "g/L", "Mucic acid at the end of each segment, dissolved and crystalline together, as a concentration in the broth. Compared to 215 h: on the fitted cultures it holds its line of 4.51 g/L to 215 h (2.6 to 4.4 by window) and misses it after, which is why the record ends there. On the held-back cultures, outside the feed range the solver now takes, 6.45, missed. The measured column may count what the samples removed; on that reading the held-back error is 10.6."),
        each_segment("volume_ml", "mL", "Broth volume at the end of each segment, before its sample. It follows from the request alone."),
        each_segment("carbon_dioxide_mol", "mol", "Carbon dioxide made since inoculation. Not held anywhere: 0.0088 mol root mean square on seven of the fitted cultures (T2 pH 4 (2) has a blank online volume and is not scored on the gases) and 0.0113 mol on the held-back, against a line of 0.00265. It is what is left of the carbon supplied after the broth and the cells are counted, and the measurements' own carbon balance is off by 0.013 to 0.016 mol."),
        each_segment("oxygen_mol", "mol", "Oxygen taken up since inoculation. Compared over whole records: 0.0111 mol root mean square on seven of the fitted cultures (T2 pH 4 (2) has a blank online volume and is not scored on the gases) and 0.0108 on the held-back, against a line of 0.0153, met, and 0.0062 on the held-back to 215 h; the one met line on the held-back cultures that a mean curve does not also meet. It follows from the electrons of what was fed and what is left, so it is evidence for the bookkeeping of glucose and not for the kinetics."),
        one("carbon_supplied_mol", "mol", "Carbon in the charge and in all the feed of the record."),
    ],
    invariants: &[
        one("carbon_residual", "", "Largest size, at any segment end, of carbon found less carbon supplied over carbon supplied."),
        one("electron_residual", "", "The same for the electrons the compounds can give to oxygen."),
        one("settle_gap", "", "Largest move of a published number between the last two halvings of the step, as a fraction of its scale."),
    ],
    iterations: "Times the step was halved. On a refusal it is the count for `NotSettled` and null otherwise.",
    field: field_words,
    answer: "a settled culture",
    refusals: REFUSALS,
    sections,
    validation: Some(validation),
    example,
    further_examples: &[],
    parts: &[],
    check,
    solve,
};

/// The sentence an agent reads in the list: the specimen, that the constants
/// are fitted to it, and how the comparison came out.
pub const SUMMARY: &str = "One fed-batch culture of the fungus Trichoderma sp. VTT D-221704 in an ambr 250 vessel at pH 4.0 and 35 °C, turning D-galacturonate into mucic acid: biomass, glucose, galacturonate, mucic acid, carbon dioxide made and oxygen taken up at the end of every segment of the caller's feeding record, with carbon and electrons closed; six constants are fitted to this organism's own measurements, at a feed of 29.4 to 34.7 g/L glucose; on four held-back cultures fed 10 and 50 g/L it met three of its six lines and missed three (galacturonate, mucic acid and carbon dioxide), a second scoring confirmed it, and it is not validated against measurement. Since then it refuses by name a feed outside the fitted 29.375 to 34.676 g/L glucose and a record longer than 215 h, so the four held-back questions are refused today, and since 0.7.0 a starting biomass, glucose or galacturonate, a feed schedule, water, sample or broth volume outside the fitted cultures' own.";

/// Each `CultureReject` variant with the sentence from its doc comment. A
/// limit the question itself shows is answered with no seal.
const REFUSALS: &[Refusal] = &[
    Refusal {
        name: "NonFiniteValue",
        meaning: "An intermediate value was NaN or infinite, or an input was when the solver is called directly: through the gateway no input can be, since a number that is not finite is not JSON.",
        sealed: true,
    },
    Refusal {
        name: "BiomassOutOfRange",
        meaning: "`biomass_g_per_l` is outside `BIOMASS_MIN_G_PER_L`..=`BIOMASS_MAX_G_PER_L`, the starts of the eight fitted cultures.",
        sealed: false,
    },
    Refusal {
        name: "GlucoseOutOfRange",
        meaning: "`glucose_g_per_l` is outside `GLUCOSE_MIN_G_PER_L`..=`GLUCOSE_MAX_G_PER_L`, the starts of the eight fitted cultures.",
        sealed: false,
    },
    Refusal {
        name: "GalacturonateOutOfRange",
        meaning: "`galacturonate_g_per_l` is outside `GALACTURONATE_MIN_G_PER_L`..=`GALACTURONATE_MAX_G_PER_L`, the starts of the eight fitted cultures.",
        sealed: false,
    },
    Refusal {
        name: "FeedGlucoseOutOfRange",
        meaning: "`feed_glucose_g_per_l` is outside `FEED_GLUCOSE_MIN_G_PER_L`..=`FEED_GLUCOSE_MAX_G_PER_L`, the feeds of the eight fitted cultures.",
        sealed: false,
    },
    Refusal {
        name: "FeedGalacturonateOutOfRange",
        meaning: "`feed_galacturonate_g_per_l` is outside `FEED_GALACTURONATE_MIN_G_PER_L`..=`FEED_GALACTURONATE_MAX_G_PER_L`, the feeds measured.",
        sealed: false,
    },
    Refusal {
        name: "SegmentCountOutOfRange",
        meaning: "The record has fewer than `SEGMENTS_MIN` or more than `SEGMENTS_MAX` segments.",
        sealed: false,
    },
    Refusal {
        name: "SegmentsDoNotMatch",
        meaning: "The end times, the feed volumes, the water volumes and the sample volumes are not one for each segment.",
        sealed: false,
    },
    Refusal {
        name: "SegmentLengthOutOfRange",
        meaning: "A segment is shorter than `SEGMENT_MIN_H` or longer than `SEGMENT_MAX_H`, or ends at or before the segment before it.",
        sealed: false,
    },
    Refusal {
        name: "RunTooLong",
        meaning: "The record ends later than `RUN_END_MAX_H` after inoculation, the end of what was compared with measurement.",
        sealed: false,
    },
    Refusal {
        name: "FeedRateOutOfRange",
        meaning: "A segment's feed is below zero or faster than `FEED_RATE_MAX_ML_PER_H`.",
        sealed: false,
    },
    Refusal {
        name: "FeedScheduleOutOfRange",
        meaning: "A segment's feed is outside the fitted cultures' schedule at some hour of the segment: fed before `FEED_START_EARLIEST_H`, faster than `FEED_START_RATE_MAX_ML_PER_H` before `FEED_START_WINDOW_END_H`, slower than `FEED_START_RATE_MIN_ML_PER_H` from `FEED_START_LATEST_H`, or slower than `FEED_RATE_MIN_ML_PER_H` from `FEED_FULL_FROM_H`.",
        sealed: false,
    },
    Refusal {
        name: "WaterRateOutOfRange",
        meaning: "A segment's water is outside `WATER_RATE_MIN_ML_PER_H`..=`WATER_RATE_MAX_ML_PER_H`.",
        sealed: false,
    },
    Refusal {
        name: "SampleOutOfRange",
        meaning: "A segment's sample is below zero or above `SAMPLE_MAX_ML`.",
        sealed: false,
    },
    Refusal {
        name: "VolumeOutOfRange",
        meaning: "The broth volume leaves `VOLUME_MIN_ML`..=`VOLUME_MAX_ML` at the end of a segment or after its sample.",
        sealed: false,
    },
    Refusal {
        name: "ConcentrationBelowZero",
        meaning: "A concentration fell below zero in the settled record.",
        sealed: true,
    },
    Refusal {
        name: "NotSettled",
        meaning: "A published number was still moving by more than `SETTLE_TOLERANCE` after `MAX_HALVINGS` halvings of the step.",
        sealed: true,
    },
    Refusal {
        name: "CarbonLedgerBroken",
        meaning: "Carbon found and carbon supplied differ by more than `LEDGER_TOLERANCE` of the carbon supplied.",
        sealed: true,
    },
    Refusal {
        name: "ElectronLedgerBroken",
        meaning: "Electrons found and electrons supplied differ by more than `LEDGER_TOLERANCE` of the electrons supplied.",
        sealed: true,
    },
];

fn field_words() -> String {
    "SHA-256 of the count of the numbers that follow as 8 little-endian bytes, which is seven times the number of segments, then for each segment in turn its biomass, glucose, galacturonate and mucic acid in g/L, its volume in mL, and its carbon dioxide and oxygen in mol, all as little-endian f64 bytes.".to_string()
}

fn sections() -> Sections {
    Sections {
        locked: json!({
            "specimen": "Trichoderma sp. VTT D-221704 (T2), engineered to oxidise D-galacturonic acid to mucic acid, in a Sartorius ambr 250 vessel: 110 mL at inoculation, 35 °C, pH 4.0, 1100 rpm, 100 mL/min of air.",
            "fitted": "Six constants, by least squares on eight cultures of this organism in this vessel (feed glucose 29.375 to 34.676 g/L, yeast extract 0 to 2 g/L): the largest glucose uptake, the biomass yield, the biomass decay, the largest galacturonate uptake, the fraction of galacturonate that becomes mucic acid, and the glucose that halves galacturonate uptake. The last is not fixed by the data: fits with one condition left out put it anywhere from 0.01 to 8.5 g/L.",
            "glucose_uptake_max_g_per_g_h": culture::GLUCOSE_UPTAKE_MAX_G_PER_G_H,
            "biomass_yield_g_per_g": culture::BIOMASS_YIELD_G_PER_G,
            "biomass_decay_per_h": culture::BIOMASS_DECAY_PER_H,
            "galacturonate_uptake_max_g_per_g_h": culture::GALACTURONATE_UPTAKE_MAX_G_PER_G_H,
            "mucic_acid_fraction": culture::MUCIC_ACID_FRACTION,
            "glucose_repression_g_per_l": culture::GLUCOSE_REPRESSION_G_PER_L,
            "set_not_fitted": "The two saturation constants, which the measurements do not resolve, and the biomass composition CH1.8O0.5N0.2, a textbook value not measured here.",
            "glucose_saturation_g_per_l": culture::GLUCOSE_SATURATION_G_PER_L,
            "galacturonate_saturation_g_per_l": culture::GALACTURONATE_SATURATION_G_PER_L,
            "biomass_g_per_cmol": culture::BIOMASS_G_PER_CMOL,
            "biomass_electrons_per_cmol": culture::BIOMASS_ELECTRONS_PER_CMOL,
            "glucose_g_per_mol": culture::GLUCOSE_G_PER_MOL,
            "galacturonate_g_per_mol": culture::GALACTURONATE_G_PER_MOL,
            "mucic_acid_g_per_mol": culture::MUCIC_ACID_G_PER_MOL,
            "charge_ml": culture::CHARGE_ML,
            "uptake": "q_G = q_Gmax G/(K_G + G); q_U = q_Umax U/(K_U + U) · K_I/(K_I + G), each per gram of biomass and hour.",
            "balances": "dX/dt = (Y q_G − k_d) X − D X; dG/dt = −q_G X + (F/V) G_f − D G; dU/dt = −q_U X + (F/V) U_f − D U; dP/dt = y_P (M_P/M_U) q_U X − D P; dV/dt = F + W; D = (F + W)/V. A sample is drawn at the end of its segment, at once.",
            "reactions": "Growth: a C-mole of glucose gives y_c of biomass and 1 − y_c of carbon dioxide and takes (4 − 4.2 y_c)/4 of oxygen. Decay: a C-mole of biomass gives one of carbon dioxide and takes 1.05 of oxygen. Oxidation: a mole of galacturonate and half a mole of oxygen give a mole of mucic acid. Burning: a mole of galacturonate and five of oxygen give six of carbon dioxide.",
            "left_out": "Yeast extract, as a nutrient and as carbon; nitrogen; oxygen limitation; the lag of the inoculum; pH and temperature other than the specimen's; mucic acid crystals; ash; the acid and the alkali, counted as water.",
            "range_end_allowance": culture::RANGE_END_ALLOWANCE,
            "range_ends": "A segment's length, its start and end against the hours of the feed schedule, its rates and the broth volume are numbers the caller's arithmetic produces. Each is held to each of its limits with an allowance of range_end_allowance of that limit itself, and a limit at zero has none, so a record that ends exactly on a limit in decimal is inside it; the solver computes at the caller's own value.",
        }),
        invariants: json!([
            {
                "name": "carbon_residual",
                "rule": "Carbon in the charge and the feed is the carbon in the broth, in the samples drawn and in the carbon dioxide made. It closes for any values of the fitted constants: it checks the integration and the table of reactions, not the biology.",
                "tolerance": culture::LEDGER_TOLERANCE,
                "unit": "fraction of the carbon supplied",
                "checked": "at the end of every segment",
                "refusal": "CarbonLedgerBroken",
            },
            {
                "name": "electron_residual",
                "rule": "The electrons the charge and the feed can give to oxygen are those of the broth and the samples drawn plus four for each mole of oxygen taken up. It closes for any values of the fitted constants.",
                "tolerance": culture::LEDGER_TOLERANCE,
                "unit": "fraction of the electrons supplied",
                "checked": "at the end of every segment",
                "refusal": "ElectronLedgerBroken",
            },
            {
                "name": "settle_gap",
                "rule": "No published number moves by more than the tolerance of its scale when every step is halved once more.",
                "tolerance": culture::SETTLE_TOLERANCE,
                "unit": "fraction of 1 g/L, 1 mL or 0.001 mol",
                "checked": "at acceptance",
                "refusal": "NotSettled",
            },
        ]),
        convergence: json!({
            "first_step_h": culture::FIRST_STEP_H,
            "max_halvings": culture::MAX_HALVINGS,
            "settle_tolerance": culture::SETTLE_TOLERANCE,
            "rule": "Every segment is stepped by classical Runge-Kutta in equal steps no longer than first_step_h, then with every step halved, and again, until no published number moves by more than the tolerance between one halving and the next. The finer of the last two is returned.",
            "refusal": "NotSettled",
        }),
        reference: json!({
            "name": culture::REFERENCE_NAME,
            "url": culture::REFERENCE_URL,
            "retrieved": culture::REFERENCE_RETRIEVED,
            "test": "the_opened_culture_of_the_zenodo_record_is_followed",
            "covers": "One of the eight cultures the constants were fitted to: glucose within 3.5 g/L, galacturonate within 4.5 g/L, mucic acid within 6 g/L, carbon dioxide within 0.02 mol and, to 170 h, biomass within 3 g/L at each of its first nine samples, to 211 h; its last two, at 259 and 307 h, are past the 215 h the solver serves.",
            "kind": "Measurements the solver was fitted to. Agreement here is not validation; the validation record is the comparison with the held-back cultures.",
            "also": "The last segment of the same culture's whole record, at 307 h, agrees with a recomputation in 50-digit arithmetic within 5e-9 of each number's scale. That record is past 215 h and is refused when asked: the test calls the integration past the check of the question, as a test of the arithmetic, and it is the_settled_culture_matches_fifty_digit_arithmetic.",
        }),
    }
}

fn validation() -> [String; 5] {
    let object = "Trichoderma sp. VTT D-221704 (called T2 in the data), a marine fungus engineered to oxidise D-galacturonic acid to mucic acid, grown in fed batch in a Sartorius ambr 250 vessel at VTT: 110 mL at inoculation, 35 °C, pH 4.0, 1100 rpm, 100 mL/min of air, a batch on about 20.8 g/L glucose and 4.6 g/L galacturonate, then a feed of glucose and galacturonate at up to 0.35 mL/h for 240 to 310 h.".to_string();
    let data = "Tamminen, Turunen, Barth, Vidgren and Wiebe, AMB Express 12 (2022) 90, data at Zenodo record 5846028, CC BY 4.0. Measured in each culture: dry biomass, glucose, galacturonate and mucic acid in a 4.3 mL sample about once a day; carbon dioxide and oxygen in the off-gas every five minutes; the feed, the alkali and the acid pumped. Not measured: the biomass composition; the carbon of the yeast extract. Fitted on the eight cultures at pH 4.0 fed 29.375 to 34.676 g/L glucose (29.4 to 34.7 rounded) with 0, 0.5, 1 and 2 g/L yeast extract. Held back whole and unread until the one scored run: the four cultures fed about 10.6 and about 50 g/L glucose (two each, yeast extract 1 g/L). The measurements do not close on their own: carbon found over carbon supplied in the fitted cultures is 0.78 to 1.15 between 40 and 215 h, yeast extract not counted. In two sheets of the workbook the dry biomass column is the same, all twelve values to the last bit (T2 YE 0 (2) and T2 YE 0.5 (2)): one of them is a copy in the published file, it cannot be told which, and both are fitted cultures, so one culture's biomass is counted twice in the fit and the other culture is fitted to biomass that is not its own. No held-back sheet shares a column with another. The workbook's description of its mucic acid column says it includes what the samples removed and is adjusted for evaporation and sample removal.".to_string();
    let pass_line = "Fixed on 2026-10-09 before any held-back number was read (protocol SHA-256 68b833108c589ce7a6e063ec84b21483fa50c7cffa145108e69ed0de816c92db). Root-mean-square error over the samples of the four held-back cultures: dry biomass to 170 h at most 2.15 g/L; glucose 0.85 g/L; galacturonate 2.74 g/L; mucic acid 4.51 g/L; carbon dioxide made 0.00265 mol; oxygen taken up 0.0153 mol. Each line is one and a half times the root-mean-square difference between the two repeats of a fitted condition, so it is set by the measurement and not by the model. The solver passes only if all six are met. The forecast, written beforehand, was that it would miss the carbon dioxide line by a factor of three to five and miss the glucose line. Scored beside it on the same points: the mean of the two fitted cultures at the same yeast extract (feed glucose makes no difference), the mean of all eight fitted cultures, the starting value held, and the same equations with constants not taken from this data. A line that one of the first two also meets, or on which the solver is not closer than both, is not evidence for the model.".to_string();
    let limits = "The held-back cultures test extrapolation in the glucose of the feed, to a third and to one and a half times the fitted value, in the same vessel type, campaign and two inoculation blocks as the fitted cultures, at one yeast extract level. The first 45 h of every culture are the same batch, and each held-back culture shares its inoculum and its day with four fitted ones, so dry biomass and glucose are not tested by them. That extrapolation failed, and the solver now refuses it: inside the feed range it takes, every comparison is with the eight cultures the constants were fitted to, and nothing unseen tests it there. One glucose level in the fitted set cannot identify a dependence on glucose, and the form of the model, not its constants, is what failed. Per output, comparison stops at 170 h for dry biomass and at 215 h, the record's end, for the others; each output's own note says how it came out. Not tested: another pH or temperature (the organism was also grown at pH 3.5 and 3.0; those cultures were not read and the solver does not take a pH); another vessel or scale; any other organism. A starting biomass, glucose or galacturonate outside the eight fitted cultures' own starts, 0.600 to 1.034, 20.097 to 21.025 and 4.146 to 4.745 g/L, is refused as BiomassOutOfRange, GlucoseOutOfRange or GalacturonateOutOfRange. The operating record is held to the fitted cultures' own: the feed schedule they ran, hour by hour, their water rates, their 4.3 mL sample and their broth volumes, and the solver refuses any other by name. The dissolved oxygen is not modelled. The carbon and electron ledgers close for any constants: they check the arithmetic. A proof shows that an answer was not changed, not that the model matches the culture.".to_string();
    [object, data, pass_line, COMPARED.to_string(), limits]
}

/// A three-day record: the batch to 22 h, the feed started to 44 h, then at its full rate to 68 h, a sample at each end.
fn example() -> Value {
    json!({
        "biomass_g_per_l": 0.9,
        "glucose_g_per_l": 20.8,
        "galacturonate_g_per_l": 4.6,
        "feed_glucose_g_per_l": 32.0,
        "feed_galacturonate_g_per_l": 94.0,
        "segment_end_h": [22.0, 44.0, 68.0],
        "feed_ml": [0.0, 4.4, 7.2],
        "water_ml": [-2.0, -1.0, 0.5],
        "sample_ml": [4.3, 4.3, 4.3],
    })
}

fn question(given: &Given) -> Result<Question<'_>, String> {
    Ok(Question {
        biomass_g_per_l: given.number("biomass_g_per_l")?,
        glucose_g_per_l: given.number("glucose_g_per_l")?,
        galacturonate_g_per_l: given.number("galacturonate_g_per_l")?,
        feed_glucose_g_per_l: given.number("feed_glucose_g_per_l")?,
        feed_galacturonate_g_per_l: given.number("feed_galacturonate_g_per_l")?,
        segment_end_h: given.numbers("segment_end_h")?,
        feed_ml: given.numbers("feed_ml")?,
        water_ml: given.numbers("water_ml")?,
        sample_ml: given.numbers("sample_ml")?,
    })
}

/// Every limit the question itself shows, asked of the solver's own check
/// before a physics permit is taken: the solver's sentence and its name for it.
fn check(given: &Given) -> Result<(), String> {
    culture::check_question(&question(given)?)
        .map_err(|reject| format!("Physical Violation: {reject} ({}).", reject.name()))
}

fn solve(given: &Given) -> Result<Solved, String> {
    Ok(solved(culture::solve(&question(given)?)))
}

/// What the solver returned, in the words every solver shares. A refusal the
/// question itself shows is answered with no seal and with the solver's
/// sentence alone, so its sentence carries the solver's name for it.
pub fn solved(returned: Result<Culture, CultureReject>) -> Solved {
    let settled = match returned {
        Ok(settled) => settled,
        Err(reject) => {
            let name = reject.name();
            let sealed = REFUSALS.iter().any(|refusal| refusal.name == name && refusal.sealed);
            return Solved::Refused(Refused {
                name,
                sentence: if sealed { reject.to_string() } else { format!("{reject} ({name})") },
                iterations: match reject {
                    CultureReject::NotSettled { halvings } => Some(u64::from(halvings)),
                    _ => None,
                },
            });
        }
    };
    let segments = settled.biomass_g_per_l.len();
    let mut field = Vec::with_capacity(7 * segments);
    for segment in 0..segments {
        field.extend_from_slice(&[
            settled.biomass_g_per_l[segment],
            settled.glucose_g_per_l[segment],
            settled.galacturonate_g_per_l[segment],
            settled.mucic_acid_g_per_l[segment],
            settled.volume_ml[segment],
            settled.carbon_dioxide_mol[segment],
            settled.oxygen_mol[segment],
        ]);
    }
    let many = |name: &str, values: Vec<f64>| (name.to_string(), Published::Numbers(values));
    let single = |name: &str, value: f64| (name.to_string(), Published::Number(value));
    Solved::Answer(Answer {
        iterations: u64::from(settled.halvings),
        result: vec![
            many("biomass_g_per_l", settled.biomass_g_per_l),
            many("glucose_g_per_l", settled.glucose_g_per_l),
            many("galacturonate_g_per_l", settled.galacturonate_g_per_l),
            many("mucic_acid_g_per_l", settled.mucic_acid_g_per_l),
            many("volume_ml", settled.volume_ml),
            many("carbon_dioxide_mol", settled.carbon_dioxide_mol),
            many("oxygen_mol", settled.oxygen_mol),
            single("carbon_supplied_mol", settled.carbon_supplied_mol),
        ],
        invariants: vec![
            single("carbon_residual", settled.carbon_residual),
            single("electron_residual", settled.electron_residual),
            single("settle_gap", settled.settle_gap),
        ],
        field,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::solvers::describe_value;

    const SOLVER_SOURCE: &str =
        include_str!("../../genesis_solvers/src/culture_trichoderma_ambr250.rs");

    /// `(variant, doc sentence)` for every variant of `CultureReject`, read
    /// from the solver's source. An intra-doc link is written as its bare
    /// code span.
    fn documented_rejects() -> Vec<(String, String)> {
        let start = SOLVER_SOURCE
            .find("pub enum CultureReject {")
            .expect("CultureReject is in the solver");
        let mut found = Vec::new();
        let mut doc = String::new();
        for line in SOLVER_SOURCE[start..].lines().skip(1) {
            if line == "}" {
                break;
            }
            let Some(text) = line.strip_prefix("    ") else {
                continue;
            };
            if let Some(sentence) = text.strip_prefix("/// ") {
                if !doc.is_empty() {
                    doc.push(' ');
                }
                doc.push_str(sentence);
            } else if text.starts_with(|first: char| first.is_ascii_uppercase()) {
                let name: String =
                    text.chars().take_while(|letter| letter.is_ascii_alphanumeric()).collect();
                found.push((name, doc.replace("[`", "`").replace("`]", "`")));
                doc.clear();
            }
        }
        found
    }

    #[test]
    fn every_refusal_carries_the_sentence_from_its_doc_comment() {
        let documented = documented_rejects();
        assert_eq!(documented.len(), REFUSALS.len());
        for ((variant, sentence), refusal) in documented.iter().zip(REFUSALS) {
            assert_eq!(variant, refusal.name);
            assert_eq!(sentence, refusal.meaning, "{}", refusal.name);
        }
        // A limit the question itself shows is answered at the door; what
        // only the computation can find is sealed.
        let sealed: Vec<&str> =
            REFUSALS.iter().filter(|refusal| refusal.sealed).map(|refusal| refusal.name).collect();
        assert_eq!(
            sealed,
            ["NonFiniteValue", "ConcentrationBelowZero", "NotSettled", "CarbonLedgerBroken", "ElectronLedgerBroken"]
        );
    }

    #[test]
    fn the_reference_is_the_one_stored_in_the_solver_test() {
        let body = describe_value(CULTURE_TRICHODERMA_AMBR250.name).expect("described");
        let url = body["reference"]["url"].as_str().expect("url");
        assert_eq!(url, "https://zenodo.org/records/5846028");
        assert!(SOLVER_SOURCE.contains(&format!("<{url}>")), "{url}");
        let retrieved = body["reference"]["retrieved"].as_str().expect("retrieved");
        assert!(SOLVER_SOURCE.contains(&format!("Retrieved {retrieved}")), "{retrieved}");
        for key in ["test", "also"] {
            let words = body["reference"][key].as_str().expect("words");
            let test = words.rsplit(' ').next().expect("a word").trim_end_matches('.');
            assert!(SOLVER_SOURCE.contains(&format!("fn {test}()")), "{test}");
        }
    }

    /// The list an agent reads first says that the constants are fitted to
    /// the organism's own measurements and how the comparison stands, and the
    /// description says that the ledgers close for any constants and what the
    /// held-back cultures do and do not test.
    #[test]
    fn the_summary_carries_the_standing() {
        assert!(SUMMARY.contains("six constants are fitted to this organism's own measurements"));
        assert!(SUMMARY.contains("met three of its six lines and missed three"));
        assert!(SUMMARY.contains("a second scoring confirmed it, and it is not validated against measurement."));
        assert!(SUMMARY.contains("so the four held-back questions are refused today"));
        assert!(SUMMARY.ends_with("and since 0.7.0 a starting biomass, glucose or galacturonate, a feed schedule, water, sample or broth volume outside the fitted cultures' own."));
        assert!(SUMMARY.contains("29.375 to 34.676 g/L glucose and a record longer than 215 h"));
        assert!(COMPARED.contains("it did not pass"));
        assert!(COMPARED.contains("only the oxygen line is evidence for the model"));
        let body = describe_value(CULTURE_TRICHODERMA_AMBR250.name).expect("described");
        assert_eq!(body["summary"], SUMMARY);
        let limits = body["validation"]["limits"].as_str().expect("limits");
        assert!(limits.contains("extrapolation in the glucose of the feed"));
        assert!(limits.contains("Not tested"));
        assert!(limits.contains("close for any constants"));
        assert_eq!(body["validation"]["result"], COMPARED);
        let carbon = body["invariants"][0]["rule"].as_str().expect("rule");
        assert!(carbon.contains("closes for any values of the fitted constants"));
        assert_eq!(body["locked"]["biomass_yield_g_per_g"], 0.4124);
        assert_eq!(body["locked"]["glucose_uptake_max_g_per_g_h"], 0.1269);
    }

    /// The example runs, every limit the question shows is refused at the
    /// door with the solver's own sentence and name, and no value is moved.
    #[test]
    fn the_example_settles_and_each_limit_of_the_question_is_named_at_the_door() {
        let request = json!({ "solver": "culture_trichoderma_ambr250", "parameters": example() });
        let given = crate::solution::read_parameters(
            &CULTURE_TRICHODERMA_AMBR250,
            request["parameters"].as_object().expect("an object"),
        )
        .unwrap_or_else(|error| panic!("{}", error.message()));
        let Solved::Answer(answer) = solve(&given).expect("runs") else {
            panic!("the example was refused");
        };
        assert!(answer.iterations >= 1);
        assert_eq!(answer.field.len(), 21);
        assert_eq!(answer.result.len(), CULTURE_TRICHODERMA_AMBR250.result.len());
        for ((name, value), output) in answer.result.iter().zip(CULTURE_TRICHODERMA_AMBR250.result) {
            assert_eq!(name, output.name);
            assert_eq!(value.shape(), output.shape);
        }
        for ((name, _), output) in answer.invariants.iter().zip(CULTURE_TRICHODERMA_AMBR250.invariants) {
            assert_eq!(name, output.name);
        }

        let refused = |field: &str, value: Value| {
            let mut parameters = example();
            parameters[field] = value;
            match crate::solution::read_parameters(
                &CULTURE_TRICHODERMA_AMBR250,
                parameters.as_object().expect("an object"),
            ) {
                Ok(_) => panic!("{field} was not refused"),
                Err(error) => error.message().to_string(),
            }
        };
        for (field, value, name) in [
            ("biomass_g_per_l", json!(1.6), "BiomassOutOfRange"),
            ("glucose_g_per_l", json!(40.0), "GlucoseOutOfRange"),
            ("galacturonate_g_per_l", json!(1.0), "GalacturonateOutOfRange"),
            ("feed_glucose_g_per_l", json!(60.0), "FeedGlucoseOutOfRange"),
            ("feed_glucose_g_per_l", json!(10.6), "FeedGlucoseOutOfRange"),
            ("feed_galacturonate_g_per_l", json!(70.0), "FeedGalacturonateOutOfRange"),
            ("feed_ml", json!([0.0, 6.0]), "SegmentsDoNotMatch"),
            ("segment_end_h", json!([22.0, 22.5, 68.0]), "SegmentLengthOutOfRange"),
            ("segment_end_h", json!([22.0, 20.0, 68.0]), "SegmentLengthOutOfRange"),
            ("feed_ml", json!([0.0, 9.7, 7.2]), "FeedRateOutOfRange"),
            ("feed_ml", json!([0.0, 4.4, 9.6]), "FeedRateOutOfRange"),
            // Fed from inoculation, started too fast, and stopped once the feed is full.
            ("feed_ml", json!([1.0, 4.4, 7.2]), "FeedScheduleOutOfRange"),
            ("feed_ml", json!([0.0, 5.0, 7.2]), "FeedScheduleOutOfRange"),
            ("feed_ml", json!([0.0, 4.4, 0.0]), "FeedScheduleOutOfRange"),
            ("feed_ml", json!([0.0, 0.0, 0.0]), "FeedScheduleOutOfRange"),
            // A batch to 24 h is a feed started later than any fitted culture's.
            ("segment_end_h", json!([24.0, 44.0, 68.0]), "FeedScheduleOutOfRange"),
            ("water_ml", json!([-9.7, -1.0, 0.5]), "WaterRateOutOfRange"),
            ("water_ml", json!([-2.0, -1.0, 4.8]), "WaterRateOutOfRange"),
            ("sample_ml", json!([4.3, 4.3, 4.4]), "SampleOutOfRange"),
            ("water_ml", json!([-2.0, -6.0, 0.5]), "VolumeOutOfRange"),
        ] {
            let message = refused(field, value);
            assert!(message.starts_with("Physical Violation: "), "{message}");
            assert!(message.ends_with(&format!("({name}).")), "{field}: {message}");
        }
        // A record within the schedule that ends after 215 h.
        let mut parameters = example();
        parameters["segment_end_h"] = json!([21.5, 45.0, 120.0, 180.0, 215.01]);
        parameters["feed_ml"] = json!([0.0, 2.25, 18.75, 15.0, 8.75]);
        parameters["water_ml"] = json!([-4.5, -0.75, -12.0, -12.0, -7.0]);
        parameters["sample_ml"] = json!([4.3, 4.3, 4.3, 4.3, 0.0]);
        match crate::solution::read_parameters(&CULTURE_TRICHODERMA_AMBR250, parameters.as_object().expect("an object")) {
            Ok(_) => panic!("a record to 215.01 h was not refused"),
            Err(error) => assert!(error.message().ends_with("(RunTooLong)."), "{}", error.message()),
        }
    }

    /// The served words say what was compared: the gas errors on the fitted
    /// cultures are of seven, the carbon recovery is the record's corrected
    /// range, and no input is said to be untested where the solver answers.
    #[test]
    fn the_served_words_match_the_record() {
        assert!(COMPARED.contains("and on seven of them 0.0088 and 0.0111 mol of gas: T2 pH 4 (2) has a blank online volume"));
        assert!(!COMPARED.contains("On the eight cultures it was fitted to the solver gives 1.44, 1.38, 2.02 and 3.67 g/L, 0.0088"));
        let body = describe_value(CULTURE_TRICHODERMA_AMBR250.name).expect("described");
        let data = body["validation"]["data"].as_str().expect("data");
        assert!(data.contains("0.78 to 1.15 between 40 and 215 h"), "{data}");
        assert!(!data.contains("1.10"), "{data}");
        let whole = body.to_string();
        assert!(!whole.contains("untested"), "untested");
        assert!(!whole.contains("up to 0.33 mL/h"));
        let returns = serde_json::to_string(&body["returns"]).expect("returns");
        assert_eq!(returns.matches("on seven of the fitted cultures (T2 pH 4 (2) has a blank online volume").count(), 2);
        let range_ends = body["locked"]["range_ends"].as_str().expect("range ends");
        assert!(range_ends.contains("an allowance of range_end_allowance of that limit itself"));
        let inputs = body["inputs"].as_array().expect("inputs");
        let bound = |name: &str| {
            inputs.iter().find(|input| input["name"] == name).unwrap_or_else(|| panic!("{name}")).clone()
        };
        assert!(bound("feed_ml")["bound"].as_str().unwrap().contains("FeedScheduleOutOfRange"));
        assert_eq!(bound("sample_ml")["maximum"], 4.3);
        let water = bound("water_ml");
        assert!((water["minimum"].as_f64().unwrap() + 29.6).abs() < 1.0e-12, "{water}");
        assert!((water["maximum"].as_f64().unwrap() - 19.0).abs() < 1.0e-12, "{water}");
        assert!((bound("feed_ml")["maximum"].as_f64().unwrap() - 33.1).abs() < 1.0e-12);
        assert!(bound("sample_ml")["bound"].as_str().unwrap().contains("between 98.0 and 122.15 mL"));
        let limits = body["validation"]["limits"].as_str().expect("limits");
        assert!(!limits.contains("other than the batch medium's"));
        for (name, low, high, reject) in [
            ("biomass_g_per_l", 0.6, 1.034, "BiomassOutOfRange"),
            ("glucose_g_per_l", 20.097, 21.025, "GlucoseOutOfRange"),
            ("galacturonate_g_per_l", 4.146, 4.745, "GalacturonateOutOfRange"),
        ] {
            let input = bound(name);
            assert_eq!(input["minimum"], low, "{name}");
            assert_eq!(input["maximum"], high, "{name}");
            assert!(input["bound"].as_str().unwrap().contains(reject), "{name}");
            assert!(limits.contains(reject), "{name}");
        }
    }

    /// A refusal the computation finds is sealed with its count; one the
    /// question shows is not sealed if the solver is somehow asked.
    #[test]
    fn a_refusal_is_put_in_the_shared_words() {
        match solved(Err(CultureReject::NotSettled { halvings: 8 })) {
            Solved::Refused(refused) => {
                assert_eq!(refused.name, "NotSettled");
                assert_eq!(refused.iterations, Some(8));
                assert_eq!(
                    refused.sentence,
                    "a published number was still moving after 8 halvings of the step"
                );
            }
            Solved::Answer(_) => panic!("an answer"),
        }
        match solved(Err(CultureReject::RunTooLong { end_h: 400.0 })) {
            Solved::Refused(refused) => {
                assert_eq!(refused.iterations, None);
                assert!(refused.sentence.ends_with("(RunTooLong)"), "{}", refused.sentence);
            }
            Solved::Answer(_) => panic!("an answer"),
        }
    }
}
