//! The entry for `culture_trichoderma_atp`: one fed-batch culture of
//! Trichoderma sp. VTT D-221704 in an ambr 250 vessel, its galacturonate
//! oxidised on the energy the glucose supplies; the companion of
//! `culture_trichoderma_ambr250`. The numbers come from the `genesis_solvers`
//! constants and the sentence of each refusal from the solver's source.
//! Stamped from the kit.

use genesis_solvers::culture_trichoderma_atp::{self as culture, Culture, CultureReject, Question};
use serde_json::{Value, json};

use crate::solvers::{
    Answer, Given, Input, Kind, Output, Published, Refusal, Refused, Sections, Shape, Solved, Solver,
};

/// What has been compared with measurement, in numbers beside their targets.
pub const COMPARED: &str = "On the four cultures not in the fit, at pH 3.5 and 3.0, RMS error against its target: galacturonate 1.83 g/L (target 2.74); mucic acid 4.19 g/L (target 4.51); oxygen taken up 0.01396 mol (target 0.0153); dry biomass to 170 h 3.64 g/L (target 2.15); glucose 1.80 g/L (target 0.85); carbon dioxide made 0.01065 mol (target 0.00265). Mucic acid is within its target where the baselines are not: the mean of the two pH 4.0 cultures gives 8.36 g/L on it and the mean of all twelve fitted cultures 5.10. On galacturonate the pH 4.0 mean gives 1.23 g/L and on oxygen 0.00869 mol, both within their targets and closer than the solver. The record's forecast put mucic acid and carbon dioxide above their targets and the other four within, if pH 3.5 and 3.0 change nothing: carbon dioxide, galacturonate and oxygen came out as forecast, biomass and glucose above their targets, and mucic acid within. The largest gaps. At pH 3.0 the batch runs slower than at 4.0: 5.81 g/L of glucose left at 45 h against 0.01 here, and 7.27 at 43 h against 0.00, so the kinetics do depend on pH between 3.0 and 3.5; each pH 3.5 culture is within the glucose target alone (0.58 and 0.68 g/L), each pH 3.0 culture above it (2.08 and 2.76). The dry weight measured at pH 3.5 and 3.0 climbs from about 90 h, to 13.8 to 17.4 g/L by 165 h where the solver holds 8.1 to 8.6, while mucic acid is 11 to 25 g/L; at pH 4.0 it rose once mucic acid exceeded about 35 g/L, from crystals in the pellet, and the record cannot tell whether this is crystals at the lower pH or cells. At the last sample mucic acid is 40.2 to 43.9 g/L here against 29.0 to 35.9 measured. Asked the same cultures with the same start, culture_trichoderma_ambr250 gives galacturonate 1.83 g/L and oxygen 0.01286 mol, within their targets, and dry biomass 4.21 g/L, glucose 1.60 g/L, mucic acid 5.35 g/L and carbon dioxide 0.01121 mol, above theirs. Every answer agrees with the record's reference program within 1e-6 g/L and 1e-7 mol and closes both ledgers within 2e-12. The binary that made the comparison's run is kept, SHA-256 04d2eafc22b04d69c65354428ba4e31aec8a7fc160e2c5f6b6174e8c13000d28, and the solver answers the four questions of the comparison with the numbers it sealed. The comparison was run by the agent that built the solver; no second, independent run has recomputed it yet.";

/// A concentration of the start, the medium or the feed: any from zero,
/// asked of the solver's own limits so that its sentence says why below zero
/// is refused.
const fn start(name: &'static str, refusal: &'static str, bound: &'static str) -> Input {
    Input {
        name,
        kind: Kind::Number { minimum: 0.0, maximum: f64::MAX, unit: "g/L" },
        refusal: Some(refusal),
        bound,
        only_when: None,
        asked_by_check: true,
    }
}

/// One array of the operating record, one number for each segment.
const fn segments(name: &'static str, minimum: f64, maximum: f64, unit: &'static str, refusal: &'static str, bound: &'static str) -> Input {
    Input {
        name,
        kind: Kind::Numbers { fewest: culture::SEGMENTS_MIN, most: culture::SEGMENTS_MAX, minimum, maximum, unit },
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

pub const CULTURE_TRICHODERMA_ATP: Solver = Solver {
    name: "culture_trichoderma_atp",
    summary: SUMMARY,
    inputs: &[
        start(
            "biomass_g_per_l",
            "BiomassNegative",
            "Dry biomass at inoculation, any from 0 (BiomassNegative below it): with none, nothing grows. The twelve fitted cultures were started at 0.600 to 1.067 g/L.",
        ),
        start(
            "glucose_g_per_l",
            "GlucoseNegative",
            "Glucose at inoculation, the batch medium, any from 0 (GlucoseNegative below it); every compound is taken as dissolved. The twelve fitted cultures were started at 20.097 to 21.151 g/L.",
        ),
        start(
            "galacturonate_g_per_l",
            "GalacturonateNegative",
            "D-galacturonate at inoculation, the batch medium, any from 0 (GalacturonateNegative below it). The twelve fitted cultures were started at 4.146 to 4.774 g/L.",
        ),
        start(
            "yeast_extract_g_per_l",
            "YeastExtractNegative",
            "Yeast extract in the batch medium, any from 0 (YeastExtractNegative below it): it raises the largest glucose uptake by 0.1195 for each g/L, so the batch runs faster; its carbon is not counted. The twelve fitted cultures had 0, 0.5, 1 or 2 g/L, the medium of the paper 2 g/L; outside 0 to 2 g/L the answer carries yeast_extract_outside_compared.",
        ),
        Input {
            name: "ph",
            kind: Kind::Number { minimum: culture::PH_MIN, maximum: culture::PH_MAX, unit: "1" },
            refusal: Some("PhOutsideAqueousScale"),
            bound: "The broth's pH, held through the record, from 0 to 14, the scale of a dilute aqueous broth (PhOutsideAqueousScale outside it: hydrogen or hydroxide ions above about 1 mol/L make a concentrated acid or alkali). It moves no number: the twelve fitted cultures were at pH 4.0, and the model holds the kinetics the same from pH 3.0 to 4.0. Compared at 3.5 and 3.0: at 3.5 the batch ran as at 4.0 and at 3.0 it ran slower than the solver's (glucose 2.08 and 2.76 g/L RMS on the two pH 3.0 cultures against a target of 0.85, 0.58 and 0.68 on the two at 3.5). Outside 3.0 to 4.0 the answer carries ph_outside_compared.",
            only_when: None,
            asked_by_check: true,
        },
        start(
            "feed_glucose_g_per_l",
            "FeedGlucoseNegative",
            "Glucose in the feed, one feed for the whole record, any from 0 (FeedGlucoseNegative below it). The twelve fitted cultures were fed 10.077 to 52.101 g/L; outside that the answer carries feed_glucose_outside_compared. The glucose supplied sets how the galacturonate is used: oxidised to mucic acid where glucose is plentiful, burnt where it is short.",
        ),
        start(
            "feed_galacturonate_g_per_l",
            "FeedGalacturonateNegative",
            "D-galacturonate in the feed, one feed for the whole record, any from 0 (FeedGalacturonateNegative below it). The twelve fitted cultures were fed 88.5 to 103.3 g/L.",
        ),
        segments(
            "segment_end_h",
            0.0,
            culture::RUN_END_MAX_H,
            "h",
            "RunTooLong",
            "End of each segment of the operating record, hours after inoculation, rising. A refusal names a segment by its place counted from 1: the first is segment 1. A segment may be any length above zero (one that does not end after the one before is SegmentNotRising), and the record ends by 1000 h (RunTooLong past it), the capacity of one solve and no physics. The culture is returned at each end. The twelve fitted cultures' records end at 307.12 and 312 h; past 312 h the answer carries record_past_compared. A segment with no sample, cut where a feed starts or stops, is how a record pumped unevenly is put.",
        ),
        segments(
            "feed_ml",
            0.0,
            f64::MAX,
            "mL",
            "FeedRateNegative",
            "Feed pumped in during each segment, at an even rate over the segment: any amount from 0 (FeedRateNegative below it), finite, from any hour, while the broth stays above 0 and at most 250 mL, the vessel's working volume (VolumeOutOfRange): no segment has a maximum of its own, and a feed of more than the vessel holds is answered where water taken out in the same segment keeps the broth in the vessel. The fitted cultures' feed started at about 24 or 38 h and stopped at about 261 or 265 h.",
        ),
        segments(
            "water_ml",
            -f64::MAX,
            f64::MAX,
            "mL",
            "VolumeOutOfRange",
            "Water gained during each segment, at an even rate: alkali and acid in, evaporation out as a negative number. Give 0 for none. Water dilutes and does nothing else, so any finite amount is answered while the broth stays above 0 and at most 250 mL, the vessel's working volume (VolumeOutOfRange): no segment has a minimum or a maximum of its own.",
        ),
        segments(
            "sample_ml",
            0.0,
            culture::VOLUME_MAX_ML,
            "mL",
            "SampleNegative",
            "Broth drawn at the end of each segment, at once, of any size from 0 (SampleNegative below it). Give 0 for none, as a caller does to cut a segment: a sample takes broth as it is. The broth must stay above 0 and at most 250 mL, the vessel's working volume, at each segment end and after each sample (VolumeOutOfRange). Every sample of the fitted cultures was 4.3 mL.",
        ),
    ],
    kept_inside: "The organism's constants, the reactions, the 110 mL charge, 35 °C, the step and the tolerances stay inside the solver.",
    result: &[
        each_segment("biomass_g_per_l", "g/L", "Dry biomass at the end of each segment, before its sample. Compared with measurement to 170 h: at pH 4.0 the measured dry weight includes mucic acid crystals from about 170 h, so from then on this number is cells alone. 1.75 g/L RMS on the twelve fitted cultures; 3.64 on the cultures not in the fit, at pH 3.5 and 3.0 (target 2.15), their dry weight climbing from about 90 h."),
        each_segment("glucose_g_per_l", "g/L", "Glucose at the end of each segment. 1.01 g/L RMS on the fitted cultures; 1.80 on the cultures not in the fit (target 0.85), the batch at pH 3.0 running slower than the solver's."),
        each_segment("galacturonate_g_per_l", "g/L", "D-galacturonate at the end of each segment. 2.87 g/L RMS on the fitted cultures; 1.83 on the cultures not in the fit (target 2.74), where the mean of the pH 4.0 cultures gives 1.23."),
        each_segment("mucic_acid_g_per_l", "g/L", "Mucic acid at the end of each segment, dissolved and crystalline together, as a concentration in the broth. 3.48 g/L RMS on the fitted cultures; 4.19 on the cultures not in the fit (target 4.51), where the mean of the pH 4.0 cultures gives 8.36 and of all twelve 5.10."),
        each_segment("volume_ml", "mL", "Broth volume at the end of each segment, before its sample. It follows from the request alone."),
        each_segment("carbon_dioxide_mol", "mol", "Carbon dioxide made since inoculation: from growth, decay and the galacturonate burnt. 0.0083 mol RMS on the fitted cultures; 0.01065 on the cultures not in the fit (target 0.00265), and culture_trichoderma_ambr250 0.01121 on the same cultures."),
        each_segment("oxygen_mol", "mol", "Oxygen taken up since inoculation. 0.0112 mol RMS on the fitted cultures; 0.01396 on the cultures not in the fit (target 0.0153), where the means of the fitted cultures are within the target as well."),
        each_segment("galacturonate_burnt_mol", "mol", "Galacturonate burnt to carbon dioxide since inoculation, six of carbon dioxide for each: what the glucose's short supply turned from mucic acid to burning."),
        one("carbon_supplied_mol", "mol", "Carbon in the charge and in all the feed of the record."),
        one("ph_outside_compared", "1", "1 where the pH asked is outside 3.0 to 4.0, the range compared with measurement, 0 inside it. The answer is the same arithmetic either way."),
        one("yeast_extract_outside_compared", "1", "1 where the yeast extract is outside 0 to 2 g/L, the twelve fitted cultures' own, 0 inside it."),
        one("feed_glucose_outside_compared", "1", "1 where the feed glucose is outside 10.077 to 52.101 g/L, the twelve fitted cultures' own, 0 inside it."),
        one("record_past_compared", "1", "1 where the record ends after 312 h, the end of the twelve fitted cultures' records, 0 where it ends by then."),
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
    door_refusals: &[],
    sections,
    validation: Some(validation),
    example,
    further_examples: &[],
    parts: &[],
    check,
    solve,
};

/// The sentence an agent reads in the list: the specimen, the form, what is
/// fitted, and how the comparison stands.
pub const SUMMARY: &str = "One fed-batch culture of the fungus Trichoderma sp. VTT D-221704 in an ambr 250 vessel at 35 °C, turning D-galacturonate into mucic acid on the energy the glucose supplies, the companion of culture_trichoderma_ambr250: where glucose is plentiful the galacturonate is oxidised to mucic acid, and as it runs short more of it is burnt to carbon dioxide; biomass, glucose, galacturonate, mucic acid, carbon dioxide made, oxygen taken up and galacturonate burnt at the end of every segment of the caller's feeding record, with carbon and electrons closed; eight constants are fitted to twelve of this organism's cultures at pH 4.0, fed 10.1 to 52.1 g/L glucose with 0 to 2 g/L yeast extract; on four held-back cultures at pH 3.5 and 3.0 it met three of its six lines and missed three (dry biomass 3.64 g/L against 2.15, glucose 1.80 g/L against 0.85, carbon dioxide 0.01065 mol against 0.00265), mucic acid met at 4.19 g/L against 4.51, and it is not validated against measurement. It takes the pH from 0 to 14 and moves no number with it; at pH 3.0 the measured batch ran slower than its own. It answers any start, feed and feed schedule from zero, held by its own limits, to 1000 h; it was compared on pH 3.0 to 4.0, feeds of 10.1 to 52.1 g/L glucose and records to 312 h.";

/// Each `CultureReject` variant with the sentence from its doc comment. A
/// limit the question itself shows is answered with no seal.
const REFUSALS: &[Refusal] = &[
    Refusal { name: "NonFiniteValue", meaning: "An intermediate value was NaN or infinite, or an input was when the solver is called directly: through the gateway no input can be, since a number that is not finite is not JSON.", sealed: true },
    Refusal { name: "BiomassNegative", meaning: "`biomass_g_per_l` is below zero: a concentration is not negative.", sealed: false },
    Refusal { name: "GlucoseNegative", meaning: "`glucose_g_per_l` is below zero: a concentration is not negative.", sealed: false },
    Refusal { name: "GalacturonateNegative", meaning: "`galacturonate_g_per_l` is below zero: a concentration is not negative.", sealed: false },
    Refusal { name: "YeastExtractNegative", meaning: "`yeast_extract_g_per_l` is below zero: a concentration is not negative.", sealed: false },
    Refusal { name: "PhOutsideAqueousScale", meaning: "`ph` is below 0 or above 14, off the scale of a dilute aqueous broth: hydrogen or hydroxide ions above about 1 mol/L make a concentrated acid or alkali.", sealed: false },
    Refusal { name: "FeedGlucoseNegative", meaning: "`feed_glucose_g_per_l` is below zero: a concentration is not negative.", sealed: false },
    Refusal { name: "FeedGalacturonateNegative", meaning: "`feed_galacturonate_g_per_l` is below zero: a concentration is not negative.", sealed: false },
    Refusal { name: "SegmentCountOutOfRange", meaning: "The record has fewer than `SEGMENTS_MIN` or more than `SEGMENTS_MAX` segments.", sealed: false },
    Refusal { name: "SegmentsDoNotMatch", meaning: "The end times, the feed volumes, the water volumes and the sample volumes are not one for each segment.", sealed: false },
    Refusal { name: "SegmentNotRising", meaning: "A segment ends at or before the segment before it, or at or before inoculation.", sealed: false },
    Refusal { name: "RunTooLong", meaning: "The record ends later than `RUN_END_MAX_H` after inoculation, the capacity of one solve: its Runge-Kutta steps grow with the record's length, up to 10,240 for each hour when every halving is taken.", sealed: false },
    Refusal { name: "FeedRateNegative", meaning: "A segment's feed is below zero: no pump takes broth back into the feed.", sealed: false },
    Refusal { name: "SampleNegative", meaning: "A segment's sample is below zero.", sealed: false },
    Refusal { name: "VolumeOutOfRange", meaning: "The broth is not above zero, or is more than the vessel's `VOLUME_MAX_ML`, at the end of a segment or after its sample.", sealed: false },
    Refusal { name: "ConcentrationBelowZero", meaning: "A concentration fell below zero in the settled record.", sealed: true },
    Refusal { name: "NotSettled", meaning: "A published number was still moving by more than `SETTLE_TOLERANCE` after `MAX_HALVINGS` halvings of the step.", sealed: true },
    Refusal { name: "CarbonLedgerBroken", meaning: "Carbon found and carbon supplied differ by more than `LEDGER_TOLERANCE` of the carbon supplied.", sealed: true },
    Refusal { name: "ElectronLedgerBroken", meaning: "Electrons found and electrons supplied differ by more than `LEDGER_TOLERANCE` of the electrons supplied.", sealed: true },
];

fn field_words() -> String {
    "SHA-256 of the count of the numbers that follow as 8 little-endian bytes, which is eight times the number of segments, then for each segment in turn its biomass, glucose, galacturonate and mucic acid in g/L, its volume in mL, and its carbon dioxide, oxygen and galacturonate burnt in mol, all as little-endian f64 bytes.".to_string()
}

fn sections() -> Sections {
    Sections {
        locked: json!({
            "specimen": "Trichoderma sp. VTT D-221704 (T2), engineered to oxidise D-galacturonic acid to mucic acid, in a Sartorius ambr 250 vessel: 110 mL at inoculation, 35 °C, 1100 rpm, 100 mL/min of air.",
            "fitted": "Eight constants, by least squares on twelve cultures of this organism in this vessel, all at pH 4.0 (feed glucose 10.077 to 52.101 g/L, yeast extract 0 to 2 g/L): the largest glucose uptake and its rise with yeast extract, the biomass yield, the biomass decay, the largest galacturonate oxidation, the glucose that halves galacturonate uptake for oxidation, the glucose uptake at which oxidation and burning share the galacturonate equally, and the largest galacturonate burning. The last two are not sharply fixed: a second minimum of the fit, 0.7 above this one, has 0.00152 and 0.0126.",
            "glucose_uptake_max_g_per_g_h": culture::GLUCOSE_UPTAKE_MAX_G_PER_G_H,
            "yeast_extract_gain_l_per_g": culture::YEAST_EXTRACT_GAIN_L_PER_G,
            "biomass_yield_g_per_g": culture::BIOMASS_YIELD_G_PER_G,
            "biomass_decay_per_h": culture::BIOMASS_DECAY_PER_H,
            "galacturonate_oxidation_max_g_per_g_h": culture::GALACTURONATE_OXIDATION_MAX_G_PER_G_H,
            "glucose_repression_g_per_l": culture::GLUCOSE_REPRESSION_G_PER_L,
            "glucose_energy_half_g_per_g_h": culture::GLUCOSE_ENERGY_HALF_G_PER_G_H,
            "galacturonate_burning_max_g_per_g_h": culture::GALACTURONATE_BURNING_MAX_G_PER_G_H,
            "set_not_fitted": "The two saturation constants, the first solver's, and the biomass composition CH1.8O0.5N0.2, a textbook value not measured here.",
            "glucose_saturation_g_per_l": culture::GLUCOSE_SATURATION_G_PER_L,
            "galacturonate_saturation_g_per_l": culture::GALACTURONATE_SATURATION_G_PER_L,
            "biomass_g_per_cmol": culture::BIOMASS_G_PER_CMOL,
            "biomass_electrons_per_cmol": culture::BIOMASS_ELECTRONS_PER_CMOL,
            "glucose_g_per_mol": culture::GLUCOSE_G_PER_MOL,
            "galacturonate_g_per_mol": culture::GALACTURONATE_G_PER_MOL,
            "mucic_acid_g_per_mol": culture::MUCIC_ACID_G_PER_MOL,
            "charge_ml": culture::CHARGE_ML,
            "uptake": "q_G = q_Gmax (1 + a_E E) G/(K_G + G), E the yeast extract; phi = q_G/(q_G + K_E), the share of the glucose's energy; q_ox = q_Umax U/(K_U + U) · K_I/(K_I + G) · phi, oxidised to mucic acid; q_B = q_Bmax U/(K_U + U) · (1 − phi), burnt; each per gram of biomass and hour.",
            "balances": "dX/dt = (Y q_G − k_d) X − D X; dG/dt = −q_G X + (F/V) G_f − D G; dU/dt = −(q_ox + q_B) X + (F/V) U_f − D U; dP/dt = (M_P/M_U) q_ox X − D P; dV/dt = F + W; D = (F + W)/V. A sample is drawn at the end of its segment, at once.",
            "reactions": "Growth: a C-mole of glucose gives y_c of biomass and 1 − y_c of carbon dioxide and takes (4 − 4.2 y_c)/4 of oxygen. Decay: a C-mole of biomass gives one of carbon dioxide and takes 1.05 of oxygen. Oxidation: a mole of galacturonate and half a mole of oxygen give a mole of mucic acid. Burning: a mole of galacturonate and five of oxygen give six of carbon dioxide.",
            "ph": "An input from 0 to 14 that moves no number: the model holds the kinetics the same from pH 3.0 to 4.0.",
            "compared_ph": [culture::COMPARED_PH.0, culture::COMPARED_PH.1],
            "compared_yeast_extract_g_per_l": [culture::COMPARED_YEAST_EXTRACT_G_PER_L.0, culture::COMPARED_YEAST_EXTRACT_G_PER_L.1],
            "compared_feed_glucose_g_per_l": [culture::COMPARED_FEED_GLUCOSE_G_PER_L.0, culture::COMPARED_FEED_GLUCOSE_G_PER_L.1],
            "compared_record_end_h": culture::COMPARED_RECORD_END_H,
            "left_out": "The carbon of the yeast extract; nitrogen; oxygen limitation; the lag of the inoculum; any effect of pH and temperature; mucic acid crystals; ash; the acid and the alkali, counted as water.",
            "range_end_allowance": culture::RANGE_END_ALLOWANCE,
            "range_ends": "The broth volume is a number the caller's arithmetic produces. It is held to the vessel's 250 mL with an allowance of range_end_allowance of that limit itself, and a limit at zero has none, so a broth that ends exactly at 250 mL in decimal is inside it; the solver computes at the caller's own value. The pH's ends and the record's end carry none.",
        }),
        invariants: json!([
            {
                "name": "carbon_residual",
                "rule": "Carbon in the charge and the feed is the carbon in the broth, in the samples drawn and in the carbon dioxide made. It closes for any values of the fitted constants: it holds the integration and the table of reactions, not the biology.",
                "tolerance": culture::LEDGER_TOLERANCE,
                "unit": "fraction of the carbon supplied",
                "compared": "at the end of every segment",
                "refusal": "CarbonLedgerBroken",
            },
            {
                "name": "electron_residual",
                "rule": "The electrons the charge and the feed can give to oxygen are those of the broth and the samples drawn plus four for each mole of oxygen taken up. It closes for any values of the fitted constants.",
                "tolerance": culture::LEDGER_TOLERANCE,
                "unit": "fraction of the electrons supplied",
                "compared": "at the end of every segment",
                "refusal": "ElectronLedgerBroken",
            },
            {
                "name": "settle_gap",
                "rule": "No published number moves by more than the tolerance of its scale when every step is halved once more.",
                "tolerance": culture::SETTLE_TOLERANCE,
                "unit": "fraction of 1 g/L, 1 mL or 0.001 mol",
                "compared": "at acceptance",
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
            "test": "the_fitted_culture_of_the_zenodo_record_is_followed",
            "covers": "One of the twelve cultures the constants were fitted to, T2 pH 4 (1): at each of its ten samples, glucose within 1 g/L, galacturonate within 5 g/L, mucic acid within 14 g/L, carbon dioxide within 0.01 mol, oxygen within 0.03 mol and, to 170 h, biomass within 2.5 g/L.",
            "kind": "Measurements the solver was fitted to: agreement here holds the fit, and the comparison with the cultures not in the fit is the validation record.",
            "also": "The answers to the record's two stand-in records, the records of T2 pH 4 (1) and (2) asked at pH 3.5, agree with the record's reference program within 1e-6 g/L and 1e-7 mol at every segment end, and pH 3.0, 3.5 and 4.0 give the same bits, in the test the_protocols_forecast_is_the_answer_served.",
        }),
    }
}

fn validation() -> [String; 5] {
    let object = "Trichoderma sp. VTT D-221704 (called T2 in the data), a marine fungus engineered to oxidise D-galacturonic acid to mucic acid, grown in fed batch in a Sartorius ambr 250 vessel at VTT: 110 mL at inoculation, 35 °C, 1100 rpm, 100 mL/min of air, a batch on about 20.8 g/L glucose and 4.6 g/L galacturonate with 0 to 2 g/L yeast extract, then a feed of glucose and galacturonate from about 24 or 38 h to about 261 or 265 h.".to_string();
    let data = "AMB Express 12, 90, data at Zenodo record 5846028. Measured in each culture: dry biomass, glucose, galacturonate and mucic acid in a 4.3 mL sample about once a day; carbon dioxide and oxygen in the off-gas every five minutes; the feed, the alkali and the acid pumped. Fitted on the twelve cultures at pH 4.0: fed 10.077 to 52.101 g/L glucose and 88.5 to 103.3 g/L galacturonate, with 0, 0.5, 1 and 2 g/L yeast extract (the eight culture_trichoderma_ambr250 is fitted on and the four it is compared with). Compared with the four cultures of the same organism at pH 3.5 and 3.0, not in the fit: the same medium and feed as the pH 4.0 pair, run at the same time in the same two blocks. The workbook says the acid volumes recorded for the pH 3.5 and 3.0 cultures of the second block are larger than those added, from air bubbles in the line. The measurements do not close carbon on their own, as the record of culture_trichoderma_ambr250 states.".to_string();
    let pass_line = "The six targets of culture_trichoderma_ambr250, unchanged, so the two compare: RMS error over every sample of the four cultures not in the fit, dry biomass to 170 h at most 2.15 g/L; glucose 0.85 g/L; galacturonate 2.74 g/L; mucic acid 4.51 g/L; carbon dioxide made 0.00265 mol; oxygen taken up 0.0153 mol; each target one and a half times the difference between repeats of a fitted condition. Four baselines are computed on the same points: the mean of the two pH 4.0 cultures (pH makes no difference, with no model), the mean of the twelve, the starting value held, and culture_trichoderma_ambr250. Record SHA-256 af5c49e15cf9236922ebe9fc2bde272b10b06bd294d1d5372ce56629e695fa33.".to_string();
    let limits = "The cultures outside the fit test the pH, at 3.5 and 3.0, with no constant fitted to it: the model holds the kinetics the same from pH 3.0 to 4.0, and at 3.0 the batch runs slower than that. Not modelled: another temperature, vessel or scale; another organism. Outside what was fitted (a start outside 0.600 to 1.067, 20.097 to 21.151 and 4.146 to 4.774 g/L; yeast extract outside 0 to 2 g/L; feed glucose outside 10.077 to 52.101 g/L; a record past 312 h) the same equations answer, every compound taken as dissolved, and the answer marks a pH, a yeast extract, a feed glucose or a record outside the compared ones; the solver's own limits hold: a concentration below zero is refused as BiomassNegative, GlucoseNegative, GalacturonateNegative, YeastExtractNegative, FeedGlucoseNegative or FeedGalacturonateNegative, a pH outside 0 to 14 as PhOutsideAqueousScale, a feed below zero as FeedRateNegative, a broth outside the 250 mL vessel as VolumeOutOfRange, a record the step halvings cannot settle as NotSettled. The carbon and electron ledgers close for any constants: they hold the arithmetic. A proof shows that an answer is the one the engine computed; how close it is to the culture is the comparison above.".to_string();
    [object, data, pass_line, COMPARED.to_string(), limits]
}

/// A three-day record: the batch to 22 h, the feed started to 44 h, then at its full rate to 68 h, a sample at each end, at pH 3.5.
fn example() -> Value {
    json!({
        "biomass_g_per_l": 0.9,
        "glucose_g_per_l": 20.8,
        "galacturonate_g_per_l": 4.6,
        "yeast_extract_g_per_l": 2.0,
        "ph": 3.5,
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
        yeast_extract_g_per_l: given.number("yeast_extract_g_per_l")?,
        ph: given.number("ph")?,
        feed_glucose_g_per_l: given.number("feed_glucose_g_per_l")?,
        feed_galacturonate_g_per_l: given.number("feed_galacturonate_g_per_l")?,
        segment_end_h: given.numbers("segment_end_h")?,
        feed_ml: given.numbers("feed_ml")?,
        water_ml: given.numbers("water_ml")?,
        sample_ml: given.numbers("sample_ml")?,
    })
}

/// Every limit the question itself shows, asked of the solver before a
/// physics permit is taken: the solver's sentence and its name for it.
fn check(given: &Given) -> Result<(), String> {
    culture::check_question(&question(given)?).map_err(|reject| format!("Physical Violation: {reject} ({}).", reject.name()))
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
    let mut field = Vec::with_capacity(8 * segments);
    for segment in 0..segments {
        field.extend_from_slice(&[
            settled.biomass_g_per_l[segment],
            settled.glucose_g_per_l[segment],
            settled.galacturonate_g_per_l[segment],
            settled.mucic_acid_g_per_l[segment],
            settled.volume_ml[segment],
            settled.carbon_dioxide_mol[segment],
            settled.oxygen_mol[segment],
            settled.galacturonate_burnt_mol[segment],
        ]);
    }
    let many = |name: &str, values: Vec<f64>| (name.to_string(), Published::Numbers(values));
    let single = |name: &str, value: f64| (name.to_string(), Published::Number(value));
    let flag = |name: &str, value: bool| (name.to_string(), Published::Number(if value { 1.0 } else { 0.0 }));
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
            many("galacturonate_burnt_mol", settled.galacturonate_burnt_mol),
            single("carbon_supplied_mol", settled.carbon_supplied_mol),
            flag("ph_outside_compared", settled.ph_outside_compared),
            flag("yeast_extract_outside_compared", settled.yeast_extract_outside_compared),
            flag("feed_glucose_outside_compared", settled.feed_glucose_outside_compared),
            flag("record_past_compared", settled.record_past_compared),
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

    const SOLVER_SOURCE: &str = include_str!("../../genesis_solvers/src/culture_trichoderma_atp.rs");

    /// `(variant, doc sentence)` for every variant of `CultureReject`, read
    /// from the solver's source. An intra-doc link is written as its bare
    /// code span.
    fn documented_rejects() -> Vec<(String, String)> {
        let start = SOLVER_SOURCE.find("pub enum CultureReject {").expect("CultureReject is in the solver");
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
                let name: String = text.chars().take_while(|letter| letter.is_ascii_alphanumeric()).collect();
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
        let sealed: Vec<&str> = REFUSALS.iter().filter(|refusal| refusal.sealed).map(|refusal| refusal.name).collect();
        assert_eq!(sealed, ["NonFiniteValue", "ConcentrationBelowZero", "NotSettled", "CarbonLedgerBroken", "ElectronLedgerBroken"]);
    }

    #[test]
    fn the_reference_is_the_one_stored_in_the_solver_test() {
        let body = describe_value(CULTURE_TRICHODERMA_ATP.name).expect("described");
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

    /// The constants served are the ones the protocol froze, by the names the
    /// scoring program reads.
    #[test]
    fn the_locked_constants_are_the_protocols() {
        let body = describe_value(CULTURE_TRICHODERMA_ATP.name).expect("described");
        let locked = &body["locked"];
        for (name, value) in [
            ("glucose_uptake_max_g_per_g_h", 0.1172),
            ("yeast_extract_gain_l_per_g", 0.1195),
            ("biomass_yield_g_per_g", 0.3718),
            ("biomass_decay_per_h", 0.001101),
            ("galacturonate_oxidation_max_g_per_g_h", 0.02996),
            ("glucose_repression_g_per_l", 6.837),
            ("glucose_energy_half_g_per_g_h", 0.001418),
            ("galacturonate_burning_max_g_per_g_h", 0.01415),
            ("glucose_saturation_g_per_l", 0.1),
            ("galacturonate_saturation_g_per_l", 0.05),
        ] {
            assert_eq!(locked[name], value, "{name}");
        }
        assert_eq!(body["summary"], crate::spec_sheet::summary_of(CULTURE_TRICHODERMA_ATP.name, SUMMARY));
        assert_eq!(body["validation"]["result"], COMPARED);
    }

    /// The list an agent reads first says what is fitted and how the one
    /// comparison came out, in the numbers of the scored run, and the record
    /// says it failed and which lines are evidence.
    #[test]
    fn the_summary_carries_the_standing() {
        let summary = crate::spec_sheet::summary_of(CULTURE_TRICHODERMA_ATP.name, SUMMARY);
        assert!(summary.contains("Fitted on 12 of the 16 cultures, all at pH 4.0"), "{summary}");
        assert!(summary.contains("dry biomass to 170 h 3.64 g/L (target 2.15), glucose 1.80 g/L (target 0.85), carbon dioxide 0.01065 mol (target 0.00265)"), "{summary}");
        assert!(summary.contains("mucic acid within 4.19 g/L (target 4.51)"), "{summary}");
        assert!(summary.contains("Range: compared at pH 3.0 to 4.0, feeds of 10.077 to 52.101 g/L glucose and records to 312 h"), "{summary}");
        assert!(COMPARED.contains("Mucic acid is within its target where the baselines are not"));
        assert!(!summary.contains("..="));
        assert!(COMPARED.contains("SHA-256 04d2eafc22b04d69c65354428ba4e31aec8a7fc160e2c5f6b6174e8c13000d28"));
        assert!(!SUMMARY.contains("..=") && !COMPARED.contains("..="));
        let body = describe_value(CULTURE_TRICHODERMA_ATP.name).expect("described");
        assert_eq!(body["validation"]["result"], COMPARED);
        let pass_line = body["validation"]["pass_line"].as_str().expect("a pass line");
        assert!(pass_line.contains("Record SHA-256 af5c49e15cf9236922ebe9fc2bde272b10b06bd294d1d5372ce56629e695fa33."));
    }

    /// The example runs, every limit the question shows is refused at the
    /// door with the solver's own sentence and name, and no value is moved.
    #[test]
    fn the_example_settles_and_each_limit_of_the_question_is_named_at_the_door() {
        let request = json!({ "solver": "culture_trichoderma_atp", "parameters": example() });
        let given = crate::solution::read_parameters(&CULTURE_TRICHODERMA_ATP, request["parameters"].as_object().expect("an object"))
            .unwrap_or_else(|error| panic!("{}", error.message()));
        let Solved::Answer(answer) = solve(&given).expect("runs") else {
            panic!("the example was refused");
        };
        assert!(answer.iterations >= 1);
        assert_eq!(answer.field.len(), 24);
        assert_eq!(answer.result.len(), CULTURE_TRICHODERMA_ATP.result.len());
        for ((name, value), output) in answer.result.iter().zip(CULTURE_TRICHODERMA_ATP.result) {
            assert_eq!(name, output.name);
            assert_eq!(value.shape(), output.shape);
        }
        let refused = |field: &str, value: Value| {
            let mut parameters = example();
            parameters[field] = value;
            match crate::solution::read_parameters(&CULTURE_TRICHODERMA_ATP, parameters.as_object().expect("an object")) {
                Ok(_) => panic!("{field} was not refused"),
                Err(error) => error.message().to_string(),
            }
        };
        for (field, value, name) in [
            ("biomass_g_per_l", json!(-0.1), "BiomassNegative"),
            ("glucose_g_per_l", json!(-1.0), "GlucoseNegative"),
            ("galacturonate_g_per_l", json!(-1.0), "GalacturonateNegative"),
            ("yeast_extract_g_per_l", json!(-0.5), "YeastExtractNegative"),
            ("ph", json!(-0.5), "PhOutsideAqueousScale"),
            ("ph", json!(14.5), "PhOutsideAqueousScale"),
            ("feed_glucose_g_per_l", json!(-1.0), "FeedGlucoseNegative"),
            ("feed_galacturonate_g_per_l", json!(-1.0), "FeedGalacturonateNegative"),
            ("feed_ml", json!([0.0, 6.0]), "SegmentsDoNotMatch"),
            ("segment_end_h", json!([22.0, 22.0, 68.0]), "SegmentNotRising"),
            ("feed_ml", json!([0.0, -0.1, 7.2]), "FeedRateNegative"),
            ("sample_ml", json!([4.3, -0.1, 4.3]), "SampleNegative"),
            ("sample_ml", json!([4.3, 200.0, 4.3]), "VolumeOutOfRange"),
            ("water_ml", json!([150.0, -1.0, 0.5]), "VolumeOutOfRange"),
        ] {
            let message = refused(field, value);
            assert!(message.starts_with("Physical Violation: "), "{message}");
            assert!(message.ends_with(&format!("({name}).")), "{field}: {message}");
            assert!(!message.contains("..="), "{message}");
        }
        // pH at both ends of the scale, and far outside what was compared, is answered and flagged.
        for (ph, flagged) in [(0.0, 1.0), (14.0, 1.0), (3.0, 0.0), (4.0, 0.0), (7.0, 1.0)] {
            let mut parameters = example();
            parameters["ph"] = json!(ph);
            let given = crate::solution::read_parameters(&CULTURE_TRICHODERMA_ATP, parameters.as_object().expect("an object"))
                .unwrap_or_else(|error| panic!("{ph}: {}", error.message()));
            let Solved::Answer(answer) = solve(&given).expect("runs") else {
                panic!("pH {ph} was refused");
            };
            let found = answer.result.iter().find(|(name, _)| name == "ph_outside_compared").expect("the flag");
            assert!(matches!(found.1, Published::Number(value) if value == flagged), "{ph}");
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
                assert_eq!(refused.sentence, "a published number was still moving after 8 halvings of the step");
            }
            Solved::Answer(_) => panic!("an answer"),
        }
        match solved(Err(CultureReject::PhOutsideAqueousScale { ph: 15.0 })) {
            Solved::Refused(refused) => {
                assert_eq!(refused.iterations, None);
                assert!(refused.sentence.ends_with("(PhOutsideAqueousScale)"), "{}", refused.sentence);
            }
            Solved::Answer(_) => panic!("an answer"),
        }
    }
}
