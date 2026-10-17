//! One fed-batch culture of the engineered marine fungus *Trichoderma* sp.
//! VTT D-221704 in an ambr 250 vessel at pH 4.0 and 35 °C, growing on glucose
//! and oxidising D-galacturonic acid to mucic acid: biomass, glucose,
//! galacturonate, mucic acid, broth volume, carbon dioxide made and oxygen
//! taken up, at the end of every segment of the caller's operating record.
//!
//! The specimen and its data. Tamminen, Turunen, Barth, Vidgren and Wiebe,
//! "Use of ambr®250 to assess mucic acid production in fed-batch cultures of
//! a marine Trichoderma sp. D-221704", AMB Express 12 (2022) 90, and its
//! data, Zenodo record 5846028, CC BY 4.0: twenty cultures of 110 mL at
//! 1100 rpm and 100 mL/min of air, each with the off-gas analysed every five
//! minutes and the broth sampled about once a day for dry biomass, glucose,
//! galacturonate and mucic acid. The record in the research folder is
//! `evidence/research/bioreactor_trichoderma_ambr250/`.
//!
//! The six fitted constants. [`GLUCOSE_UPTAKE_MAX_G_PER_G_H`],
//! [`BIOMASS_YIELD_G_PER_G`], [`BIOMASS_DECAY_PER_H`],
//! [`GALACTURONATE_UPTAKE_MAX_G_PER_G_H`], [`MUCIC_ACID_FRACTION`] and
//! [`GLUCOSE_REPRESSION_G_PER_L`] are fitted by least squares to this
//! organism's own measurements in this vessel: the eight cultures at pH 4.0
//! with a feed of 29 to 35 g/L glucose and 0, 0.5, 1 or 2 g/L yeast extract.
//! The two saturation constants, [`GLUCOSE_SATURATION_G_PER_L`] and
//! [`GALACTURONATE_SATURATION_G_PER_L`], are set, not fitted: the
//! measurements do not resolve them. The elemental formulas are the
//! compounds' own, and biomass is taken as CH1.8O0.5N0.2, 24.6 g for each
//! mole of carbon, a textbook composition that was not measured here.
//!
//! The comparison with cultures the fit never saw, made once on 2026-10-09.
//! Held back whole: the four cultures fed about 10 and about 50 g/L glucose.
//! The six lines were fixed beforehand (protocol SHA-256
//! 68b833108c589ce7a6e063ec84b21483fa50c7cffa145108e69ed0de816c92db), each
//! one and a half times the difference between repeats, and the solver did
//! not pass: it met three (biomass 1.05 g/L against 2.15, glucose 0.51
//! against 0.85, oxygen 0.0108 mol against 0.0153) and missed three
//! (galacturonate 4.60 g/L against 2.74, mucic acid 6.45 against 4.51,
//! carbon dioxide 0.01125 mol against 0.00265). A mean curve of the fitted
//! cultures also meets the biomass and the glucose lines, so of the three
//! met only the oxygen line is evidence for the model. Fed a third of the
//! glucose, the organism leaves galacturonate unconverted and burns it when
//! the feed stops; here galacturonate uptake does not depend on the glucose
//! supplied. The solver is not validated against measurement.
//!
//! The second scoring of 2026-10-09 (`evidence/verification/second_party/V-41`)
//! confirmed every number. It found that the form fails and not the
//! constants: refitted to all twelve cultures the same equations still miss
//! the same three lines (4.30 g/L, 5.21 g/L and 0.0108 mol). The eight fitted
//! cultures were fed one glucose level, which cannot fix any dependence of
//! galacturonate oxidation on the glucose supplied.
//!
//! What the solver answers. Every start, feed, schedule and record the
//! equations can take, held only by their own limits: a concentration is not
//! below zero, a feed is not below zero, the broth stays in the vessel, above
//! zero and up to [`VOLUME_MAX_ML`], the 250 mL the ambr 250 vessel holds, a
//! concentration the settled record drives below zero is refused
//! (`ConcentrationBelowZero`), a step that does not settle is refused
//! (`NotSettled`), and both ledgers must close. A record ends by
//! [`RUN_END_MAX_H`], the capacity of one solve. Water only dilutes and a
//! sample takes broth as it is, with no fitted constant. Every compound is
//! taken as dissolved, as the mucic acid crystals are.
//!
//! What was compared, stated and not enforced. The twelve compared cultures,
//! the eight fitted and the four held back, were started at 0.600 to 1.067
//! g/L biomass, 20.097 to 21.151 g/L glucose and 4.146 to 4.774 g/L
//! galacturonate; fed 10.077 to 52.101 g/L glucose (fitted 29.375 to
//! 34.676; outside it galacturonate and mucic acid were wrong by 6 to 8 g/L
//! from about 60 h on) and 88.5 to 103.3 g/L galacturonate; unfed until
//! 21.18 or 22.87 h, fed 0.0914 to 0.2234 mL/h in the segment where the feed
//! began, 0.2421 to 0.3301 mL/h in every later segment to 211.28 or 213.18
//! h, and from 211.28 h, when the feeds were slowed and then stopped, 0.0093
//! to 0.2601 mL/h. Their records end at 307.12 and 312 h, and mucic acid,
//! carbon dioxide and oxygen were compared to 215 h, after which mucic acid
//! misses its line on the fitted cultures themselves. Their segments were at
//! most 98.8 h, their water -0.3122 to 0.2284 mL/h, their samples 4.3 mL and
//! their broth 97.67 to 127.70 mL. Until this version every start, feed and
//! the feed schedule were held to the twelve cultures' own, and a record to
//! 312 h; through engine 0.8.0 the four held-back questions were refused,
//! and the build that answered them for the scored run is the record of the
//! score.
//!
//! What is left out. Yeast extract, as a nutrient and as carbon; nitrogen,
//! with ammonia taken as its source in the electron count; oxygen limitation
//! (the dissolved oxygen is not a state); the lag of the inoculum; pH and
//! temperature, which are those of the specimen and not inputs; mucic acid
//! crystals, which form above a few grams per litre and are counted here as
//! dissolved; ash in the biomass; the acid and the alkali, counted as water.
//!
//! The equations. Per litre of broth, with biomass `X`, glucose `G`,
//! galacturonate `U`, mucic acid `P` in g/L, volume `V`, feed `F` and water
//! `W` in L/h, feed concentrations `G_f` and `U_f`:
//!
//! ```text
//! q_G = q_Gmax · G / (K_G + G)                      g glucose per g biomass per hour
//! q_U = q_Umax · U / (K_U + U) · K_I / (K_I + G)    g galacturonate per g biomass per hour
//! dX/dt = (Y · q_G − k_d) · X − (F + W)/V · X
//! dG/dt = −q_G · X + F/V · G_f − (F + W)/V · G
//! dU/dt = −q_U · X + F/V · U_f − (F + W)/V · U
//! dP/dt = y_P · (M_P/M_U) · q_U · X − (F + W)/V · P
//! dV/dt = F + W
//! ```
//!
//! Four reactions carry the carbon and the electrons. Growth: one C-mole of
//! glucose gives `y_c` C-mole of biomass and `1 − y_c` of carbon dioxide and
//! takes `(4 − 4.2 y_c)/4` mole of oxygen. Decay: one C-mole of biomass gives
//! one of carbon dioxide and takes 1.05 of oxygen. Oxidation: one mole of
//! galacturonate and half a mole of oxygen give one mole of mucic acid.
//! Burning: one mole of galacturonate and five of oxygen give six of carbon
//! dioxide. A sample is drawn at the end of its segment, at once, and takes
//! broth as it is.
//!
//! The two ledgers. Carbon supplied (the charge and the feed) against carbon
//! found (broth, samples drawn, carbon dioxide); electrons supplied against
//! electrons found (broth, samples drawn, four for each mole of oxygen). Each
//! must close within [`LEDGER_TOLERANCE`] of what was supplied. They close
//! for any values of the fitted constants: they check the integration and
//! the table of reactions, not the biology. What tests the biology is the
//! comparison of the carbon dioxide and the oxygen with the measured off-gas.
//!
//! Settling. Every segment is stepped by classical Runge-Kutta in equal
//! steps no longer than [`FIRST_STEP_H`], then with every step halved, and
//! again, until no published number moves by more than [`SETTLE_TOLERANCE`]
//! of its scale between one halving and the next. The finer of the last two
//! is returned.
//!
//! Range ends. The broth volume, a number the caller's arithmetic produces,
//! is held to the vessel with an allowance of [`RANGE_END_ALLOWANCE`] of the
//! limit itself, so that a broth that ends exactly at 250 mL in decimal is
//! inside it in binary. A limit at zero has no allowance, and the record's
//! end, a capacity, has none: a record that ends at [`RUN_END_MAX_H`] or
//! before is answered.

use crate::normal::NormalOrZero;
use std::fmt;

/// Largest glucose uptake, g glucose per g biomass per hour. Fitted.
pub const GLUCOSE_UPTAKE_MAX_G_PER_G_H: f64 = 0.1269;
/// Biomass formed per glucose taken up, g per g. Fitted.
pub const BIOMASS_YIELD_G_PER_G: f64 = 0.4124;
/// Biomass lost to its own respiration, per hour. Fitted.
pub const BIOMASS_DECAY_PER_H: f64 = 0.00335;
/// Largest galacturonate uptake, g galacturonate per g biomass per hour. Fitted.
pub const GALACTURONATE_UPTAKE_MAX_G_PER_G_H: f64 = 0.03001;
/// Moles of mucic acid formed for each mole of galacturonate taken up; the rest is burnt. Fitted.
pub const MUCIC_ACID_FRACTION: f64 = 0.9883;
/// Glucose at which galacturonate uptake is halved, g/L. Fitted.
pub const GLUCOSE_REPRESSION_G_PER_L: f64 = 4.640;
/// Glucose at which its uptake is half the largest, g/L. Set, not fitted.
pub const GLUCOSE_SATURATION_G_PER_L: f64 = 0.1;
/// Galacturonate at which its uptake is half the largest, g/L. Set, not fitted.
pub const GALACTURONATE_SATURATION_G_PER_L: f64 = 0.05;

/// Molar mass of glucose, C6H12O6, g/mol.
pub const GLUCOSE_G_PER_MOL: f64 = 180.156;
/// Molar mass of D-galacturonic acid, C6H10O7, g/mol.
pub const GALACTURONATE_G_PER_MOL: f64 = 194.139;
/// Molar mass of mucic acid, C6H10O8, g/mol.
pub const MUCIC_ACID_G_PER_MOL: f64 = 210.138;
/// Biomass for each mole of its carbon, CH1.8O0.5N0.2, g. Not measured here.
pub const BIOMASS_G_PER_CMOL: f64 = 24.6;
/// Electrons each mole of biomass carbon gives to oxygen, with ammonia as the nitrogen source.
pub const BIOMASS_ELECTRONS_PER_CMOL: f64 = 4.2;
/// Electrons a mole of glucose gives to oxygen when burnt.
pub const GLUCOSE_ELECTRONS_PER_MOL: f64 = 24.0;
/// Electrons a mole of galacturonic acid gives to oxygen when burnt.
pub const GALACTURONATE_ELECTRONS_PER_MOL: f64 = 20.0;
/// Electrons a mole of mucic acid gives to oxygen when burnt.
pub const MUCIC_ACID_ELECTRONS_PER_MOL: f64 = 18.0;

/// Broth in the vessel at inoculation, mL.
pub const CHARGE_ML: f64 = 110.0;
/// The starting biomass of the twelve compared cultures, g/L: 0.600000000000156
/// in T2 YE 0 (2) and the second block's others, to 1.0667 in T2 glucose 50
/// (2), rounded outward to the thousandth. Stated, not enforced: any start
/// from zero is answered. Until this version it was the start's range.
pub const COMPARED_BIOMASS_G_PER_L: (f64, f64) = (0.6, 1.067);
/// The starting glucose of the twelve compared cultures, g/L: 20.097 in T2 YE
/// 0.5 (1) to 21.151 in T2 glucose 10 (2). Stated, not enforced.
pub const COMPARED_GLUCOSE_G_PER_L: (f64, f64) = (20.097, 21.151);
/// The starting galacturonate of the twelve compared cultures, g/L: 4.146 in
/// T2 YE 0 (2) to 4.7735 in T2 glucose 50 (1), rounded outward to the
/// thousandth. Stated, not enforced.
pub const COMPARED_GALACTURONATE_G_PER_L: (f64, f64) = (4.146, 4.774);
/// The glucose in the feed of the twelve compared cultures, g/L: the held-back
/// T2 glucose 10 (1) to T2 glucose 50 (2), where galacturonate and mucic acid
/// were wrong by 6 to 8 g/L from about 60 h on. Stated, not enforced.
pub const COMPARED_FEED_GLUCOSE_G_PER_L: (f64, f64) = (10.077, 52.101);
/// The feed glucose of the eight fitted cultures, g/L: T2 YE 0.5 (1) to
/// T2 YE 0 (2). Stated, not enforced.
pub const FITTED_FEED_GLUCOSE_G_PER_L: (f64, f64) = (29.375, 34.676);
/// The galacturonate in the feed of the twelve compared cultures, g/L: the
/// fitted 88.5 to 103.3, the held-back 89.257 to 97.5675. Stated, not
/// enforced; until this version a feed outside 80 to 110 g/L was refused.
pub const COMPARED_FEED_GALACTURONATE_G_PER_L: (f64, f64) = (88.5, 103.3);
/// Fewest segments of an operating record.
pub const SEGMENTS_MIN: usize = 1;
/// Most segments of an operating record.
pub const SEGMENTS_MAX: usize = 32;
/// The longest segment of the twelve compared cultures, h: 98.8 h between two
/// samples. Stated, not enforced: any segment above zero length is answered.
pub const COMPARED_SEGMENT_MAX_H: f64 = 98.82;
/// Latest end of a record, h after inoculation: the capacity of one solve, and
/// no physics. A solve steps every hour of the record in steps of at most
/// [`FIRST_STEP_H`], then with every step halved, up to [`MAX_HALVINGS`]
/// times: up to 511 passes' worth, about 10,240 Runge-Kutta steps for each
/// hour. At 1000 h the slowest solve, every halving taken, took 0.68 s on one
/// core of an Apple M4 Pro (2026-10-14; 0.22 s at 312 h). The twelve compared
/// cultures' records end at 307.12 and 312 h ([`COMPARED_RECORD_END_H`]);
/// until this version that was the limit.
pub const RUN_END_MAX_H: f64 = 1000.0;
/// The end of the twelve compared cultures' records, h. Stated, not enforced.
pub const COMPARED_RECORD_END_H: f64 = 312.0;
/// The end of what was compared with measurement for mucic acid, carbon
/// dioxide and oxygen, h. Stated, not enforced.
pub const COMPARED_RUN_END_H: f64 = 215.0;
/// Most broth the vessel holds, mL: the top of the ambr 250 vessel's working
/// volume. Below zero there is no broth; at zero the dilution rate is not
/// finite. The twelve compared cultures held 97.67 to 127.70 mL.
pub const VOLUME_MAX_ML: f64 = 250.0;
/// Fraction of a limit by which a computed number may pass it: decimal ends are inside.
pub const RANGE_END_ALLOWANCE: f64 = 1.0e-12;

/// Longest step of the first pass, h.
pub const FIRST_STEP_H: f64 = 0.05;
/// Most times the step is halved.
pub const MAX_HALVINGS: u32 = 8;
/// Largest move of a published number between two halvings, as a fraction of its scale.
pub const SETTLE_TOLERANCE: f64 = 1.0e-9;
/// Scale of a concentration for settling, g/L.
pub const SETTLE_SCALE_G_PER_L: f64 = 1.0;
/// Scale of an amount of gas for settling, mol.
pub const SETTLE_SCALE_MOL: f64 = 1.0e-3;
/// Largest carbon or electron imbalance, as a fraction of what was supplied.
pub const LEDGER_TOLERANCE: f64 = 1.0e-9;

/// The public source of the measurements the reference test reads.
pub const REFERENCE_NAME: &str = "Tamminen et al., Data for growth of marine Trichoderma sp. T2 in ambr250 to produce mucic acid, Zenodo record 5846028, sheet T2 YE 1 (2)";
/// Where it is.
pub const REFERENCE_URL: &str = "https://zenodo.org/records/5846028";
/// When it was read.
pub const REFERENCE_RETRIEVED: &str = "2026-10-09";

/// Why a culture was not returned.
#[derive(Clone, Debug, PartialEq)]
pub enum CultureReject {
    /// An intermediate value was NaN or infinite, or an input was when the solver is called directly: through the gateway no input can be, since a number that is not finite is not JSON.
    NonFiniteValue,
    /// `biomass_g_per_l` is below zero: a concentration is not negative.
    BiomassNegative { biomass_g_per_l: f64 },
    /// `glucose_g_per_l` is below zero: a concentration is not negative.
    GlucoseNegative { glucose_g_per_l: f64 },
    /// `galacturonate_g_per_l` is below zero: a concentration is not negative.
    GalacturonateNegative { galacturonate_g_per_l: f64 },
    /// `feed_glucose_g_per_l` is below zero: a concentration is not negative.
    FeedGlucoseNegative { feed_glucose_g_per_l: f64 },
    /// `feed_galacturonate_g_per_l` is below zero: a concentration is not negative.
    FeedGalacturonateNegative { feed_galacturonate_g_per_l: f64 },
    /// The record has fewer than [`SEGMENTS_MIN`] or more than [`SEGMENTS_MAX`] segments.
    SegmentCountOutOfRange { segments: usize },
    /// The end times, the feed volumes, the water volumes and the sample volumes are not one for each segment.
    SegmentsDoNotMatch { ends: usize, feeds: usize, waters: usize, samples: usize },
    /// A segment ends at or before the segment before it, or at or before inoculation.
    SegmentNotRising { segment: usize, length_h: f64 },
    /// The record ends later than [`RUN_END_MAX_H`] after inoculation, the capacity of one solve: its Runge-Kutta steps grow with the record's length, up to 10,240 for each hour when every halving is taken.
    RunTooLong { end_h: f64 },
    /// A segment's feed is below zero: no pump takes broth back into the feed.
    FeedRateNegative { segment: usize, feed_ml: f64 },
    /// A segment's sample is below zero.
    SampleNegative { segment: usize, sample_ml: f64 },
    /// The broth is not above zero, or is more than the vessel's [`VOLUME_MAX_ML`], at the end of a segment or after its sample.
    VolumeOutOfRange { segment: usize, volume_ml: f64 },
    /// A concentration fell below zero in the settled record.
    ConcentrationBelowZero { segment: usize },
    /// A published number was still moving by more than [`SETTLE_TOLERANCE`] after [`MAX_HALVINGS`] halvings of the step.
    NotSettled { halvings: u32 },
    /// Carbon found and carbon supplied differ by more than [`LEDGER_TOLERANCE`] of the carbon supplied.
    CarbonLedgerBroken { residual: f64 },
    /// Electrons found and electrons supplied differ by more than [`LEDGER_TOLERANCE`] of the electrons supplied.
    ElectronLedgerBroken { residual: f64 },
}

impl CultureReject {
    /// The variant name, without its fields.
    pub fn name(&self) -> &'static str {
        match self {
            Self::NonFiniteValue => "NonFiniteValue",
            Self::BiomassNegative { .. } => "BiomassNegative",
            Self::GlucoseNegative { .. } => "GlucoseNegative",
            Self::GalacturonateNegative { .. } => "GalacturonateNegative",
            Self::FeedGlucoseNegative { .. } => "FeedGlucoseNegative",
            Self::FeedGalacturonateNegative { .. } => "FeedGalacturonateNegative",
            Self::SegmentCountOutOfRange { .. } => "SegmentCountOutOfRange",
            Self::SegmentsDoNotMatch { .. } => "SegmentsDoNotMatch",
            Self::SegmentNotRising { .. } => "SegmentNotRising",
            Self::RunTooLong { .. } => "RunTooLong",
            Self::FeedRateNegative { .. } => "FeedRateNegative",
            Self::SampleNegative { .. } => "SampleNegative",
            Self::VolumeOutOfRange { .. } => "VolumeOutOfRange",
            Self::ConcentrationBelowZero { .. } => "ConcentrationBelowZero",
            Self::NotSettled { .. } => "NotSettled",
            Self::CarbonLedgerBroken { .. } => "CarbonLedgerBroken",
            Self::ElectronLedgerBroken { .. } => "ElectronLedgerBroken",
        }
    }
}

impl fmt::Display for CultureReject {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NonFiniteValue => write!(f, "an input or a computed value was not finite"),
            Self::BiomassNegative { biomass_g_per_l } => {
                write!(f, "starting biomass {biomass_g_per_l} g/L is below zero: a concentration is not negative")
            }
            Self::GlucoseNegative { glucose_g_per_l } => {
                write!(f, "starting glucose {glucose_g_per_l} g/L is below zero: a concentration is not negative")
            }
            Self::GalacturonateNegative { galacturonate_g_per_l } => write!(
                f,
                "starting galacturonate {galacturonate_g_per_l} g/L is below zero: a concentration is not negative"
            ),
            Self::FeedGlucoseNegative { feed_glucose_g_per_l } => {
                write!(f, "feed glucose {feed_glucose_g_per_l} g/L is below zero: a concentration is not negative")
            }
            Self::FeedGalacturonateNegative { feed_galacturonate_g_per_l } => write!(
                f,
                "feed galacturonate {feed_galacturonate_g_per_l} g/L is below zero: a concentration is not negative"
            ),
            Self::SegmentCountOutOfRange { segments } => {
                write!(f, "{segments} segments is outside {SEGMENTS_MIN} to {SEGMENTS_MAX} segments")
            }
            Self::SegmentsDoNotMatch { ends, feeds, waters, samples } => write!(
                f,
                "{ends} end times, {feeds} feed volumes, {waters} water volumes and {samples} sample volumes are not one for each segment"
            ),
            Self::SegmentNotRising { segment, length_h } => write!(
                f,
                "segment {} is {length_h} h long: it does not end after the segment before it",
                segment + 1
            ),
            Self::RunTooLong { end_h } => write!(
                f,
                "the record ends at {end_h} h, later than {RUN_END_MAX_H} h after inoculation, the capacity of one solve: its Runge-Kutta steps grow with the record's length, up to 10,240 for each hour when every halving is taken"
            ),
            Self::FeedRateNegative { segment, feed_ml } => write!(
                f,
                "segment {} is fed {feed_ml} mL, below zero: no pump takes broth back into the feed",
                segment + 1
            ),
            Self::SampleNegative { segment, sample_ml } => write!(
                f,
                "segment {} ends with a sample of {sample_ml} mL, below zero",
                segment + 1
            ),
            Self::VolumeOutOfRange { segment, volume_ml } => write!(
                f,
                "the broth is {volume_ml} mL at the end of segment {}: it must be above 0 and at most {VOLUME_MAX_ML} mL, the vessel's working volume",
                segment + 1
            ),
            Self::ConcentrationBelowZero { segment } => {
                write!(f, "a concentration is below zero at the end of segment {}", segment + 1)
            }
            Self::NotSettled { halvings } => {
                write!(f, "a published number was still moving after {halvings} halvings of the step")
            }
            Self::CarbonLedgerBroken { residual } => write!(
                f,
                "carbon found and carbon supplied differ by {residual} of the carbon supplied, more than {LEDGER_TOLERANCE}"
            ),
            Self::ElectronLedgerBroken { residual } => write!(
                f,
                "electrons found and electrons supplied differ by {residual} of the electrons supplied, more than {LEDGER_TOLERANCE}"
            ),
        }
    }
}

/// One culture and its operating record. Segment `i` runs from the end of
/// segment `i − 1` (inoculation for the first) to `segment_end_h[i]`.
#[derive(Clone, Debug, PartialEq)]
pub struct Question<'a> {
    /// Dry biomass at inoculation, g/L.
    pub biomass_g_per_l: f64,
    /// Glucose at inoculation, g/L.
    pub glucose_g_per_l: f64,
    /// D-galacturonate at inoculation, g/L.
    pub galacturonate_g_per_l: f64,
    /// Glucose in the feed, g/L. One feed for the whole record.
    pub feed_glucose_g_per_l: f64,
    /// D-galacturonate in the feed, g/L. One feed for the whole record.
    pub feed_galacturonate_g_per_l: f64,
    /// End of each segment, hours after inoculation, rising.
    pub segment_end_h: &'a [f64],
    /// Feed pumped in during each segment, mL, at an even rate.
    pub feed_ml: &'a [f64],
    /// Water gained during each segment, mL, at an even rate: alkali and acid in, evaporation out (negative).
    pub water_ml: &'a [f64],
    /// Broth drawn at the end of each segment, mL.
    pub sample_ml: &'a [f64],
}

/// The settled culture: one number for each segment, at its end and before
/// its sample is drawn.
#[derive(Clone, Debug, PartialEq)]
pub struct Culture {
    /// Dry biomass, g/L.
    pub biomass_g_per_l: Vec<f64>,
    /// Glucose, g/L.
    pub glucose_g_per_l: Vec<f64>,
    /// D-galacturonate, g/L.
    pub galacturonate_g_per_l: Vec<f64>,
    /// Mucic acid, dissolved and crystalline together, g/L.
    pub mucic_acid_g_per_l: Vec<f64>,
    /// Broth volume, mL.
    pub volume_ml: Vec<f64>,
    /// Carbon dioxide made since inoculation, mol.
    pub carbon_dioxide_mol: Vec<f64>,
    /// Oxygen taken up since inoculation, mol.
    pub oxygen_mol: Vec<f64>,
    /// Carbon supplied by the charge and the feed over the whole record, mol.
    pub carbon_supplied_mol: f64,
    /// Carbon found less carbon supplied, over carbon supplied: the largest size at any segment end.
    pub carbon_residual: f64,
    /// Electrons found less electrons supplied, over electrons supplied: the largest size at any segment end.
    pub electron_residual: f64,
    /// Largest move of a published number between the last two halvings, as a fraction of its scale.
    pub settle_gap: f64,
    /// Times the step was halved.
    pub halvings: u32,
}

/// The table of reactions: what each gives and takes. The solver uses
/// [`Reactions::of_the_specimen`]; a test changes one entry to see the
/// ledger name it.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Reactions {
    /// Carbon dioxide for each C-mole of glucose taken up in growth.
    growth_carbon_dioxide: f64,
    /// Oxygen for each C-mole of glucose taken up in growth.
    growth_oxygen: f64,
    /// Carbon dioxide for each C-mole of biomass decayed.
    decay_carbon_dioxide: f64,
    /// Oxygen for each C-mole of biomass decayed.
    decay_oxygen: f64,
    /// Oxygen for each mole of galacturonate oxidised to mucic acid.
    oxidation_oxygen: f64,
    /// Carbon dioxide for each mole of galacturonate burnt.
    burning_carbon_dioxide: f64,
    /// Oxygen for each mole of galacturonate burnt.
    burning_oxygen: f64,
}

/// C-moles of biomass formed for each C-mole of glucose taken up.
fn carbon_yield() -> f64 {
    BIOMASS_YIELD_G_PER_G * GLUCOSE_G_PER_MOL / 6.0 / BIOMASS_G_PER_CMOL
}

impl Reactions {
    fn of_the_specimen() -> Self {
        let y_c = carbon_yield();
        Self {
            growth_carbon_dioxide: 1.0 - y_c,
            growth_oxygen: (4.0 - BIOMASS_ELECTRONS_PER_CMOL * y_c) / 4.0,
            decay_carbon_dioxide: 1.0,
            decay_oxygen: BIOMASS_ELECTRONS_PER_CMOL / 4.0,
            oxidation_oxygen: 0.5,
            burning_carbon_dioxide: 6.0,
            burning_oxygen: 5.0,
        }
    }
}

/// Biomass, glucose, galacturonate, mucic acid (g/L), volume (L), carbon
/// dioxide and oxygen (mol).
type State = [f64; 7];

/// What is held over one segment.
#[derive(Clone, Copy)]
struct Held {
    feed_l_per_h: f64,
    water_l_per_h: f64,
    feed_glucose_g_per_l: f64,
    feed_galacturonate_g_per_l: f64,
}

fn rates(reactions: &Reactions, y: &State, held: &Held) -> State {
    let [biomass, glucose, galacturonate, mucic, volume, _, _] = *y;
    let glucose_uptake = if glucose > 0.0 {
        GLUCOSE_UPTAKE_MAX_G_PER_G_H * glucose / (GLUCOSE_SATURATION_G_PER_L + glucose)
    } else {
        0.0
    };
    let mut galacturonate_uptake = if galacturonate > 0.0 {
        GALACTURONATE_UPTAKE_MAX_G_PER_G_H * galacturonate
            / (GALACTURONATE_SATURATION_G_PER_L + galacturonate)
    } else {
        0.0
    };
    if glucose > 0.0 {
        galacturonate_uptake *= GLUCOSE_REPRESSION_G_PER_L / (GLUCOSE_REPRESSION_G_PER_L + glucose);
    }
    // Reaction rates for each litre of broth and hour.
    let growth_cmol = glucose_uptake * biomass / GLUCOSE_G_PER_MOL * 6.0;
    let decay_cmol = BIOMASS_DECAY_PER_H * biomass / BIOMASS_G_PER_CMOL;
    let uptake_mol = galacturonate_uptake * biomass / GALACTURONATE_G_PER_MOL;
    let oxidation_mol = MUCIC_ACID_FRACTION * uptake_mol;
    let burning_mol = (1.0 - MUCIC_ACID_FRACTION) * uptake_mol;
    let dilution = (held.feed_l_per_h + held.water_l_per_h) / volume;
    let feed = held.feed_l_per_h / volume;
    [
        (carbon_yield() * growth_cmol - decay_cmol) * BIOMASS_G_PER_CMOL - dilution * biomass,
        -growth_cmol / 6.0 * GLUCOSE_G_PER_MOL + feed * held.feed_glucose_g_per_l - dilution * glucose,
        -uptake_mol * GALACTURONATE_G_PER_MOL + feed * held.feed_galacturonate_g_per_l
            - dilution * galacturonate,
        oxidation_mol * MUCIC_ACID_G_PER_MOL - dilution * mucic,
        held.feed_l_per_h + held.water_l_per_h,
        (reactions.growth_carbon_dioxide * growth_cmol
            + reactions.decay_carbon_dioxide * decay_cmol
            + reactions.burning_carbon_dioxide * burning_mol)
            * volume,
        (reactions.growth_oxygen * growth_cmol
            + reactions.decay_oxygen * decay_cmol
            + reactions.oxidation_oxygen * oxidation_mol
            + reactions.burning_oxygen * burning_mol)
            * volume,
    ]
}

fn runge_kutta(reactions: &Reactions, y: &State, held: &Held, step_h: f64) -> State {
    let shifted = |by: &State, part: f64| {
        let mut at = *y;
        for (value, rate) in at.iter_mut().zip(by) {
            *value += part * step_h * rate;
        }
        at
    };
    let k1 = rates(reactions, y, held);
    let k2 = rates(reactions, &shifted(&k1, 0.5), held);
    let k3 = rates(reactions, &shifted(&k2, 0.5), held);
    let k4 = rates(reactions, &shifted(&k3, 1.0), held);
    let mut next = *y;
    for index in 0..7 {
        next[index] += step_h / 6.0 * (k1[index] + 2.0 * k2[index] + 2.0 * k3[index] + k4[index]);
    }
    next
}

/// Carbon in a litre of broth, mol.
fn carbon_per_l(y: &State) -> f64 {
    y[0] / BIOMASS_G_PER_CMOL
        + 6.0 * y[1] / GLUCOSE_G_PER_MOL
        + 6.0 * y[2] / GALACTURONATE_G_PER_MOL
        + 6.0 * y[3] / MUCIC_ACID_G_PER_MOL
}

/// Electrons a litre of broth would give to oxygen if burnt, mol.
fn electrons_per_l(y: &State) -> f64 {
    BIOMASS_ELECTRONS_PER_CMOL * y[0] / BIOMASS_G_PER_CMOL
        + GLUCOSE_ELECTRONS_PER_MOL * y[1] / GLUCOSE_G_PER_MOL
        + GALACTURONATE_ELECTRONS_PER_MOL * y[2] / GALACTURONATE_G_PER_MOL
        + MUCIC_ACID_ELECTRONS_PER_MOL * y[3] / MUCIC_ACID_G_PER_MOL
}

/// One pass over the whole record with every first step cut in `pieces`.
struct Pass {
    ends: Vec<State>,
    carbon_supplied_mol: f64,
    carbon_residual: f64,
    electron_residual: f64,
}

fn pass(reactions: &Reactions, question: &Question<'_>, pieces: u32) -> Pass {
    let mut y: State = [
        question.biomass_g_per_l,
        question.glucose_g_per_l,
        question.galacturonate_g_per_l,
        0.0,
        CHARGE_ML / 1000.0,
        0.0,
        0.0,
    ];
    let feed_carbon_per_l = 6.0 * question.feed_glucose_g_per_l / GLUCOSE_G_PER_MOL
        + 6.0 * question.feed_galacturonate_g_per_l / GALACTURONATE_G_PER_MOL;
    let feed_electrons_per_l = GLUCOSE_ELECTRONS_PER_MOL * question.feed_glucose_g_per_l
        / GLUCOSE_G_PER_MOL
        + GALACTURONATE_ELECTRONS_PER_MOL * question.feed_galacturonate_g_per_l
            / GALACTURONATE_G_PER_MOL;
    let mut carbon_supplied = carbon_per_l(&y) * y[4];
    let mut electrons_supplied = electrons_per_l(&y) * y[4];
    let mut carbon_drawn = 0.0;
    let mut electrons_drawn = 0.0;
    let mut carbon_residual = 0.0_f64;
    let mut electron_residual = 0.0_f64;
    let mut ends = Vec::with_capacity(question.segment_end_h.len());
    let mut start_h = 0.0;
    for segment in 0..question.segment_end_h.len() {
        let length_h = question.segment_end_h[segment] - start_h;
        let held = Held {
            feed_l_per_h: question.feed_ml[segment] / 1000.0 / length_h,
            water_l_per_h: question.water_ml[segment] / 1000.0 / length_h,
            feed_glucose_g_per_l: question.feed_glucose_g_per_l,
            feed_galacturonate_g_per_l: question.feed_galacturonate_g_per_l,
        };
        let first = (length_h / FIRST_STEP_H).ceil().max(1.0);
        let steps = first as u64 * u64::from(pieces);
        let step_h = length_h / steps as f64;
        for _ in 0..steps {
            y = runge_kutta(reactions, &y, &held, step_h);
        }
        carbon_supplied += feed_carbon_per_l * question.feed_ml[segment] / 1000.0;
        electrons_supplied += feed_electrons_per_l * question.feed_ml[segment] / 1000.0;
        let carbon_found = carbon_per_l(&y) * y[4] + carbon_drawn + y[5];
        let electrons_found = electrons_per_l(&y) * y[4] + electrons_drawn + 4.0 * y[6];
        carbon_residual = carbon_residual.max(ledger_gap(carbon_found, carbon_supplied));
        electron_residual = electron_residual.max(ledger_gap(electrons_found, electrons_supplied));
        if !(carbon_found.is_finite() && electrons_found.is_finite()) {
            carbon_residual = f64::NAN;
        }
        ends.push(y);
        let sample_l = question.sample_ml[segment] / 1000.0;
        carbon_drawn += carbon_per_l(&y) * sample_l;
        electrons_drawn += electrons_per_l(&y) * sample_l;
        y[4] -= sample_l;
        start_h = question.segment_end_h[segment];
    }
    Pass {
        ends,
        carbon_supplied_mol: carbon_supplied,
        carbon_residual,
        electron_residual,
    }
}

/// What a ledger found against what was supplied, over what was supplied. A
/// vessel and a feed with no carbon at all supply none: nothing can be found,
/// and the ledger closes at zero if nothing is.
fn ledger_gap(found: f64, supplied: f64) -> f64 {
    if supplied == 0.0 {
        if found == 0.0 { 0.0 } else { f64::INFINITY }
    } else {
        ((found - supplied) / supplied).abs()
    }
}

/// Each end is held with the allowance of its own size: an end at zero has none.
fn inside(value: f64, minimum: f64, maximum: f64) -> bool {
    value >= minimum - RANGE_END_ALLOWANCE * minimum.abs()
        && value <= maximum + RANGE_END_ALLOWANCE * maximum.abs()
}

/// Every limit the question itself shows, in the order the solver asks them.
/// Nothing is integrated.
pub fn check_question(question: &Question<'_>) -> Result<(), CultureReject> {
    let segments = question.segment_end_h.len();
    let all = [
        question.biomass_g_per_l,
        question.glucose_g_per_l,
        question.galacturonate_g_per_l,
        question.feed_glucose_g_per_l,
        question.feed_galacturonate_g_per_l,
    ];
    let arrays = [question.segment_end_h, question.feed_ml, question.water_ml, question.sample_ml];
    if all.iter().any(|value| !value.is_finite())
        || arrays.iter().any(|array| array.iter().any(|value| !value.is_finite()))
    {
        return Err(CultureReject::NonFiniteValue);
    }
    if !(question.biomass_g_per_l >= 0.0) {
        return Err(CultureReject::BiomassNegative { biomass_g_per_l: question.biomass_g_per_l });
    }
    if !(question.glucose_g_per_l >= 0.0) {
        return Err(CultureReject::GlucoseNegative { glucose_g_per_l: question.glucose_g_per_l });
    }
    if !(question.galacturonate_g_per_l >= 0.0) {
        return Err(CultureReject::GalacturonateNegative { galacturonate_g_per_l: question.galacturonate_g_per_l });
    }
    if !(question.feed_glucose_g_per_l >= 0.0) {
        return Err(CultureReject::FeedGlucoseNegative { feed_glucose_g_per_l: question.feed_glucose_g_per_l });
    }
    if !(question.feed_galacturonate_g_per_l >= 0.0) {
        return Err(CultureReject::FeedGalacturonateNegative {
            feed_galacturonate_g_per_l: question.feed_galacturonate_g_per_l,
        });
    }
    if !(SEGMENTS_MIN..=SEGMENTS_MAX).contains(&segments) {
        return Err(CultureReject::SegmentCountOutOfRange { segments });
    }
    if question.feed_ml.len() != segments
        || question.water_ml.len() != segments
        || question.sample_ml.len() != segments
    {
        return Err(CultureReject::SegmentsDoNotMatch {
            ends: segments,
            feeds: question.feed_ml.len(),
            waters: question.water_ml.len(),
            samples: question.sample_ml.len(),
        });
    }
    let mut start_h = 0.0;
    let mut volume_ml = CHARGE_ML;
    for segment in 0..segments {
        let end_h = question.segment_end_h[segment];
        let length_h = end_h - start_h;
        if !(length_h > 0.0) {
            return Err(CultureReject::SegmentNotRising { segment, length_h });
        }
        if !(end_h <= RUN_END_MAX_H) {
            return Err(CultureReject::RunTooLong { end_h });
        }
        if !(question.feed_ml[segment] >= 0.0) {
            return Err(CultureReject::FeedRateNegative { segment, feed_ml: question.feed_ml[segment] });
        }
        let sample_ml = question.sample_ml[segment];
        if !(sample_ml >= 0.0) {
            return Err(CultureReject::SampleNegative { segment, sample_ml });
        }
        // The broth through the segment moves linearly from its start to its
        // end, so it stays inside the vessel if both ends do.
        volume_ml += question.feed_ml[segment] + question.water_ml[segment];
        if !(volume_ml > 0.0 && inside(volume_ml, 0.0, VOLUME_MAX_ML)) {
            return Err(CultureReject::VolumeOutOfRange { segment, volume_ml });
        }
        volume_ml -= sample_ml;
        if !(volume_ml > 0.0 && inside(volume_ml, 0.0, VOLUME_MAX_ML)) {
            return Err(CultureReject::VolumeOutOfRange { segment, volume_ml });
        }
        start_h = end_h;
    }
    Ok(())
}

/// The culture at the end of every segment of the record, settled, or the
/// named limit that stopped it.
///
/// Each value of the answer below the normal range is written as zero
/// with its sign ([`crate::normal`]).
pub fn solve(question: &Question<'_>) -> Result<Culture, CultureReject> {
    crate::normal::held(solve_as_computed(question))
}

/// The answer as the equations compute it, before a value below the
/// normal range is written as zero.
fn solve_as_computed(question: &Question<'_>) -> Result<Culture, CultureReject> {
    settle(&Reactions::of_the_specimen(), question)
}

impl NormalOrZero for Culture {
    fn normal_or_zero(&mut self) {
        self.biomass_g_per_l.normal_or_zero();
        self.glucose_g_per_l.normal_or_zero();
        self.galacturonate_g_per_l.normal_or_zero();
        self.mucic_acid_g_per_l.normal_or_zero();
        self.volume_ml.normal_or_zero();
        self.carbon_dioxide_mol.normal_or_zero();
        self.oxygen_mol.normal_or_zero();
        self.carbon_supplied_mol.normal_or_zero();
        self.carbon_residual.normal_or_zero();
        self.electron_residual.normal_or_zero();
        self.settle_gap.normal_or_zero();
    }
}

fn settle(reactions: &Reactions, question: &Question<'_>) -> Result<Culture, CultureReject> {
    check_question(question)?;
    integrate(reactions, question)
}

/// The settling itself, for a question whose limits have been asked.
fn integrate(reactions: &Reactions, question: &Question<'_>) -> Result<Culture, CultureReject> {
    let mut coarse = pass(reactions, question, 1);
    let mut halvings = 0_u32;
    loop {
        halvings += 1;
        let fine = pass(reactions, question, 1 << halvings);
        let mut gap = 0.0_f64;
        for (a, b) in coarse.ends.iter().zip(&fine.ends) {
            for index in 0..7 {
                if !(a[index].is_finite() && b[index].is_finite()) {
                    return Err(CultureReject::NonFiniteValue);
                }
                let scale = match index {
                    0..=3 => SETTLE_SCALE_G_PER_L,
                    4 => 1.0e-3,
                    _ => SETTLE_SCALE_MOL,
                };
                gap = gap.max((a[index] - b[index]).abs() / scale);
            }
        }
        if gap <= SETTLE_TOLERANCE {
            return accept(fine, gap, halvings);
        }
        if halvings >= MAX_HALVINGS {
            return Err(CultureReject::NotSettled { halvings });
        }
        coarse = fine;
    }
}

fn accept(settled: Pass, settle_gap: f64, halvings: u32) -> Result<Culture, CultureReject> {
    if !(settled.carbon_residual.is_finite() && settled.electron_residual.is_finite()) {
        return Err(CultureReject::NonFiniteValue);
    }
    if settled.carbon_residual > LEDGER_TOLERANCE {
        return Err(CultureReject::CarbonLedgerBroken { residual: settled.carbon_residual });
    }
    if settled.electron_residual > LEDGER_TOLERANCE {
        return Err(CultureReject::ElectronLedgerBroken { residual: settled.electron_residual });
    }
    for (segment, end) in settled.ends.iter().enumerate() {
        if end[..4].iter().any(|value| *value < 0.0) {
            return Err(CultureReject::ConcentrationBelowZero { segment });
        }
    }
    let column = |index: usize, factor: f64| settled.ends.iter().map(|end| end[index] * factor).collect();
    Ok(Culture {
        biomass_g_per_l: column(0, 1.0),
        glucose_g_per_l: column(1, 1.0),
        galacturonate_g_per_l: column(2, 1.0),
        mucic_acid_g_per_l: column(3, 1.0),
        volume_ml: column(4, 1000.0),
        carbon_dioxide_mol: column(5, 1.0),
        oxygen_mol: column(6, 1.0),
        carbon_supplied_mol: settled.carbon_supplied_mol,
        carbon_residual: settled.carbon_residual,
        electron_residual: settled.electron_residual,
        settle_gap,
        halvings,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    // The operating record and the measurements of the opened culture
    // T2 YE 1 (2) of Zenodo record 5846028, eleven segments between its
    // twelve samples, and the 50-digit values of its last segment. The
    // solver serves the first nine, which end by 215 h.
    include!("culture_trichoderma_ambr250_reference.rs");

    /// The operating records of the eight fitted cultures to their last sample by
    /// 215 h, each written as a question of this solver: name, starting
    /// biomass, glucose, galacturonate, feed glucose, feed galacturonate,
    /// segment ends, feed, water.
    #[allow(clippy::type_complexity)]
    const FITTED: [(&str, f64, f64, f64, f64, f64, [f64; 9], [f64; 9], [f64; 9]); 8] = [
        ("T2_YE_0_1", 0.9333333333333046, 20.151, 4.656499999999999, 31.753, 103.31649999999999,
         [21.1823338888889, 45.1823338888889, 69.1823338888889, 93.1823338888889, 117.1823338888889, 141.1823338888889, 165.1823338888889, 189.1823338888889, 213.1823338888889],
         [0.0, 2.26358488755066, 7.4214474287324705, 7.13999388923807, 6.796691728285598, 6.6485338990434, 6.315785361288597, 6.064024869581303, 5.838382601526199],
         [-2.405145717064255, -0.415282576462666, -0.3130251063424696, -2.032425409538061, -2.002389005534611, -1.7773874055563939, -5.8302605820545965, -1.678682514381319, -1.5003496557791793]),
        ("T2_YE_0_2", 0.600000000000156, 20.676, 4.146, 34.676, 92.244,
         [22.8673958333333, 43.4507291666667, 67.4507291666667, 91.4507291666667, 115.450729166667, 139.450729166667, 163.450729166667, 187.2840625, 211.2840625],
         [0.0, 4.39731817176122, 7.47241460128108, 7.172019335357801, 6.9211944332943, 6.6704208229662, 6.419811751952501, 6.150497359371698, 5.9677932808818],
         [2.2037653680175593, 0.7536332820520038, -0.6726240934530843, -2.004737210008794, -2.1056771404863293, -1.930587063915202, -0.5382581549235015, -2.255161010092789, 0.11743916368619534]),
        ("T2_YE_0p5_1", 0.9333333333333046, 20.097, 4.6754999999999995, 29.375, 95.768,
         [21.1823338888889, 45.1823338888889, 69.1823338888889, 93.1823338888889, 117.1823338888889, 141.1823338888889, 165.1823338888889, 189.1823338888889, 213.1823338888889],
         [0.0, 2.24255585692338, 7.420974203015371, 7.12646212305285, 6.795770111038699, 6.638207191839999, 6.312820262092103, 6.062817584730496, 5.837240976185299],
         [-6.240604993472698, 0.6253877014765092, 3.0053620497641536, -5.930877514309857, 2.708541293327297, -5.437406636161996, -1.1608331424471006, -1.2955610519515064, -1.9589880794012915]),
        ("T2_YE_0p5_2", 0.600000000000156, 20.782, 4.5525, 33.432, 88.54675,
         [22.8673958333333, 43.4507291666667, 67.4507291666667, 91.4507291666667, 115.450729166667, 139.450729166667, 163.450729166667, 187.2840625, 211.2840625],
         [0.0, 4.59754011790881, 7.47217328355099, 7.196714933065699, 6.946054655781602, 6.695292846008996, 6.469576652638004, 6.200066432989999, 6.042615469527696],
         [3.8490502318236706, 1.0824407742208555, -1.702913939105998, -2.4941710447896996, -2.3074645443316273, -2.163026546251996, -0.3216766715590085, -2.025488873747304, -2.779503986452686]),
        ("T2_YE_1_1", 0.9333333333333046, 20.439, 4.7145, 30.92, 100.9985,
         [21.1823338888889, 45.1823338888889, 69.1823338888889, 93.1823338888889, 117.1823338888889, 141.1823338888889, 165.1823338888889, 189.1823338888889, 213.1823338888889],
         [0.0, 2.2352141295853, 7.42364631987591, 7.14203080497809, 6.823418407667699, 6.658358213766903, 6.318600141896798, 6.065012766589298, 5.839425748761002],
         [-2.081857202664037, 0.8108326196897098, -1.374081019212917, -2.039435984299085, -1.7950193717187028, -1.5798543884119, -5.866788814608811, -1.3481184382052946, -0.5120984263420043]),
        ("T2_YE_1_2", 1.0333333333333676, 21.025, 4.619, 33.685, 89.00399999999999,
         [22.8673958333333, 43.4507291666667, 67.4507291666667, 91.4507291666667, 115.450729166667, 139.450729166667, 163.450729166667, 187.2840625, 211.2840625],
         [0.0, 4.29414879665725, 7.4718105680930496, 7.2212669425732, 6.945783377663101, 6.694925763978802, 6.469254145894098, 6.199695395635402, 6.0173591858793],
         [4.331913428890334, 1.4764407355920701, -1.5997603112360386, -2.282972002934204, -2.199260570715105, -1.9935545473998078, -0.45603270242911, -2.1513897857500526, 0.11022046404771668]),
        ("T2_pH_4_1", 0.9333333333333046, 20.37, 4.7445, 30.109, 97.71549999999999,
         [21.1823338888889, 45.1823338888889, 69.1823338888889, 93.1823338888889, 117.1823338888889, 141.1823338888889, 165.1823338888889, 189.1823338888889, 213.1823338888889],
         [0.0, 2.19272609464317, 7.421021815082449, 7.138761974982179, 6.820739615943001, 6.672788531200702, 6.341408333909495, 6.090327184376001, 5.865053061096205],
         [-4.657772817667035, 1.6368275607948375, 2.6916092734485435, -3.760652053021178, 2.643037077513, -0.17679914159770238, -4.780543140370491, -1.1354445020350186, -0.014373167681196897]),
        ("T2_pH_4_2", 0.8666666666666453, 20.886, 4.6025, 34.082, 90.13300000000001,
         [22.8673958333333, 43.4507291666667, 67.4507291666667, 91.4507291666667, 115.450729166667, 139.450729166667, 163.450729166667, 187.2840625, 211.2840625],
         [0.0, 2.81373231868742, 7.92219007177418, 7.771683949605702, 7.4709386488937, 7.195305457298499, 6.9446364360956, 6.6475694820613995, 6.468442295562703],
         [-6.7600517788918895, 3.1826175596980133, -0.666463143109179, -0.0003543048116911862, -1.3760955235317072, -1.0258680543894991, 0.22152510483939647, -1.7251321150736176, -0.0695307549636981]),
    ];

    /// Segments of the opened culture that end by 215 h.
    const SERVED_SEGMENTS: usize = 9;

    /// The opened culture as far as the solver serves it: to its sample at 211.28 h.
    fn opened() -> Question<'static> {
        let whole = whole_opened();
        Question {
            segment_end_h: &whole.segment_end_h[..SERVED_SEGMENTS],
            feed_ml: &whole.feed_ml[..SERVED_SEGMENTS],
            water_ml: &whole.water_ml[..SERVED_SEGMENTS],
            sample_ml: &whole.sample_ml[..SERVED_SEGMENTS],
            ..whole
        }
    }

    /// The whole record of the opened culture, to 307 h, which the solver now refuses.
    fn whole_opened() -> Question<'static> {
        Question {
            biomass_g_per_l: REFERENCE_BIOMASS_G_PER_L,
            glucose_g_per_l: REFERENCE_GLUCOSE_G_PER_L,
            galacturonate_g_per_l: REFERENCE_GALACTURONATE_G_PER_L,
            feed_glucose_g_per_l: REFERENCE_FEED_GLUCOSE_G_PER_L,
            feed_galacturonate_g_per_l: REFERENCE_FEED_GALACTURONATE_G_PER_L,
            segment_end_h: &ENDS_H,
            feed_ml: &REFERENCE_FEED_ML,
            water_ml: &REFERENCE_WATER_ML,
            sample_ml: &REFERENCE_SAMPLE_ML,
        }
    }

    /// A short record with round numbers, for the limits: a batch to 22 h, the
    /// feed started at 0.2 mL/h to 44 h, then 0.3 mL/h to 68 h.
    fn short() -> (Vec<f64>, Vec<f64>, Vec<f64>, Vec<f64>) {
        (vec![22.0, 44.0, 68.0], vec![0.0, 4.4, 7.2], vec![-2.0, -1.0, 0.5], vec![4.3, 4.3, 4.3])
    }

    /// A record to `last` h within the schedule: a batch to 21.5 h, the feed
    /// started to 45 h, then 0.25 mL/h to 120 h and 180 h; the last segment
    /// is fed 0.25 mL/h if it is reached.
    fn to(last: f64) -> (Vec<f64>, Vec<f64>, Vec<f64>, Vec<f64>) {
        (
            vec![21.5, 45.0, 120.0, 180.0, last],
            vec![0.0, 2.25, 18.75, 15.0, 0.25 * (last - 180.0)],
            vec![-4.5, -0.75, -12.0, -12.0, -0.2 * (last - 180.0)],
            vec![4.3, 4.3, 4.3, 4.3, 0.0],
        )
    }

    fn asked<'a>(record: &'a (Vec<f64>, Vec<f64>, Vec<f64>, Vec<f64>)) -> Question<'a> {
        Question {
            biomass_g_per_l: 0.9,
            glucose_g_per_l: 20.5,
            galacturonate_g_per_l: 4.6,
            feed_glucose_g_per_l: 32.0,
            feed_galacturonate_g_per_l: 94.0,
            segment_end_h: &record.0,
            feed_ml: &record.1,
            water_ml: &record.2,
            sample_ml: &record.3,
        }
    }

    /// The published reference of rule 7: the opened culture T2 YE 1 (2),
    /// <https://zenodo.org/records/5846028>. Retrieved 2026-10-09. The solver
    /// is fitted to this culture among eight, so this is agreement with data
    /// it was fitted on: the measured values within the stated allowances.
    #[test]
    fn the_opened_culture_of_the_zenodo_record_is_followed() {
        assert!(ENDS_H[SERVED_SEGMENTS - 1] <= COMPARED_RUN_END_H && ENDS_H[SERVED_SEGMENTS] > COMPARED_RUN_END_H);
        let culture = solve(&opened()).expect("settles");
        assert_eq!(culture.biomass_g_per_l.len(), SERVED_SEGMENTS);
        for segment in 0..SERVED_SEGMENTS {
            let within = |got: f64, measured: f64, allowed: f64| {
                measured.is_nan() || (got - measured).abs() <= allowed
            };
            assert!(
                within(culture.glucose_g_per_l[segment], REFERENCE_MEASURED_GLUCOSE[segment], 3.5),
                "glucose {segment}"
            );
            assert!(
                within(culture.galacturonate_g_per_l[segment], REFERENCE_MEASURED_GALACTURONATE[segment], 4.5),
                "galacturonate {segment}"
            );
            assert!(
                within(culture.mucic_acid_g_per_l[segment], REFERENCE_MEASURED_MUCIC_ACID[segment], 6.0),
                "mucic acid {segment}"
            );
            assert!(
                within(culture.carbon_dioxide_mol[segment], REFERENCE_MEASURED_CARBON_DIOXIDE_MOL[segment], 0.02),
                "carbon dioxide {segment}"
            );
            if ENDS_H[segment] <= 170.0 {
                assert!(
                    within(culture.biomass_g_per_l[segment], REFERENCE_MEASURED_BIOMASS[segment], 3.0),
                    "biomass {segment}"
                );
            }
        }
        assert!(culture.carbon_residual <= LEDGER_TOLERANCE);
        assert!(culture.electron_residual <= LEDGER_TOLERANCE);
        assert!(culture.settle_gap <= SETTLE_TOLERANCE);
    }

    /// The same culture against values worked outside Rust with Python
    /// mpmath at 50 digits, the step halved there until fifteen figures
    /// stood (`evidence/verification/crosscheck/culture_trichoderma_ambr250/`).
    /// Those values are of the whole record, to 307 h, which `solve` answers
    /// since engine 0.9.0 (it refused it from 0.7.0, when the record ended at
    /// 215 h), and the nine segments compared to 215 h are the first nine of
    /// the whole, bit for bit where both settle at the same halving.
    #[test]
    fn the_settled_culture_matches_fifty_digit_arithmetic() {
        let culture = integrate(&Reactions::of_the_specimen(), &whole_opened()).expect("settles");
        assert_eq!(solve(&whole_opened()), Ok(culture.clone()));
        let served = solve(&opened()).expect("settles");
        if served.halvings == culture.halvings {
            for segment in 0..SERVED_SEGMENTS {
                assert_eq!(served.mucic_acid_g_per_l[segment].to_bits(), culture.mucic_acid_g_per_l[segment].to_bits());
                assert_eq!(served.oxygen_mol[segment].to_bits(), culture.oxygen_mol[segment].to_bits());
            }
        }
        let last = 10;
        for (got, reference, scale) in [
            (culture.biomass_g_per_l[last], MPMATH_LAST_BIOMASS, 1.0),
            (culture.glucose_g_per_l[last], MPMATH_LAST_GLUCOSE, 1.0),
            (culture.galacturonate_g_per_l[last], MPMATH_LAST_GALACTURONATE, 1.0),
            (culture.mucic_acid_g_per_l[last], MPMATH_LAST_MUCIC_ACID, 1.0),
            (culture.volume_ml[last], MPMATH_LAST_VOLUME_ML, 1.0),
            (culture.carbon_dioxide_mol[last], MPMATH_LAST_CARBON_DIOXIDE_MOL, 1.0e-3),
            (culture.oxygen_mol[last], MPMATH_LAST_OXYGEN_MOL, 1.0e-3),
        ] {
            assert!(((got - reference) / scale).abs() <= 5.0e-9, "{got} against {reference}");
        }
    }

    #[test]
    fn the_same_question_gives_the_same_bits() {
        let a = solve(&opened()).expect("settles");
        let b = solve(&opened()).expect("settles");
        for (x, y) in a.carbon_dioxide_mol.iter().zip(&b.carbon_dioxide_mol) {
            assert_eq!(x.to_bits(), y.to_bits());
        }
        assert_eq!(a, b);
    }

    /// Carbon and electrons are counted by hand at the end of a short
    /// record: the charge and the feed against broth, samples and gas.
    #[test]
    fn carbon_and_electrons_close_on_a_short_record() {
        let record = short();
        let culture = solve(&asked(&record)).expect("settles");
        assert!(culture.carbon_residual < 1.0e-11, "{}", culture.carbon_residual);
        assert!(culture.electron_residual < 1.0e-11, "{}", culture.electron_residual);
        let charge = 0.110 * (0.9 / 24.6 + 6.0 * 20.5 / 180.156 + 6.0 * 4.6 / 194.139);
        let fed = 0.0116 * (6.0 * 32.0 / 180.156 + 6.0 * 94.0 / 194.139);
        assert!((culture.carbon_supplied_mol - (charge + fed)).abs() < 1.0e-15);
        // Volume: the charge, the feed and the water in, two samples out before the third end.
        assert!((culture.volume_ml[2] - (110.0 + 11.6 - 2.5 - 8.6)).abs() < 1.0e-9);
        assert!(culture.carbon_dioxide_mol[2] > culture.carbon_dioxide_mol[1]);
        assert!(culture.oxygen_mol[2] > 0.0);
    }

    /// The attack of rule 8. A table of reactions with one entry changed (a
    /// burning that gives five carbon dioxide for six carbons, or a growth
    /// that takes no oxygen) is named by the ledger it breaks.
    #[test]
    fn a_falsified_reaction_is_named_by_its_ledger() {
        let record = short();
        let honest = Reactions::of_the_specimen();
        assert!(settle(&honest, &asked(&record)).is_ok());
        let lost_carbon = Reactions { burning_carbon_dioxide: 5.0, ..honest };
        match settle(&lost_carbon, &asked(&record)) {
            Err(CultureReject::CarbonLedgerBroken { residual }) => assert!(residual > 1.0e-6),
            other => panic!("{other:?}"),
        }
        let free_growth = Reactions { growth_oxygen: 0.0, ..honest };
        match settle(&free_growth, &asked(&record)) {
            Err(CultureReject::ElectronLedgerBroken { residual }) => assert!(residual > 1.0e-3),
            other => panic!("{other:?}"),
        }
        let made_carbon = Reactions { growth_carbon_dioxide: 1.0, ..honest };
        assert!(matches!(
            settle(&made_carbon, &asked(&record)),
            Err(CultureReject::CarbonLedgerBroken { .. })
        ));
    }

    /// Every limit the question shows is named, in the solver's order.
    #[test]
    fn each_limit_of_the_question_is_named() {
        let record = short();
        let base = asked(&record);
        let name = |question: &Question<'_>| solve(question).expect_err("refused").name();
        assert_eq!(name(&Question { biomass_g_per_l: f64::NAN, ..base.clone() }), "NonFiniteValue");
        // A concentration below zero, by one unit in the last place, is named; zero is inside.
        let below_zero = -f64::from_bits(1);
        for (question, refused) in [
            (Question { biomass_g_per_l: below_zero, ..base.clone() }, "BiomassNegative"),
            (Question { glucose_g_per_l: below_zero, ..base.clone() }, "GlucoseNegative"),
            (Question { galacturonate_g_per_l: below_zero, ..base.clone() }, "GalacturonateNegative"),
            (Question { feed_glucose_g_per_l: below_zero, ..base.clone() }, "FeedGlucoseNegative"),
            (Question { feed_galacturonate_g_per_l: below_zero, ..base.clone() }, "FeedGalacturonateNegative"),
            (Question { biomass_g_per_l: -1.0, ..base.clone() }, "BiomassNegative"),
        ] {
            assert_eq!(name(&question), refused);
        }
        for question in [
            Question { biomass_g_per_l: 0.0, ..base.clone() },
            Question { glucose_g_per_l: 0.0, ..base.clone() },
            Question { galacturonate_g_per_l: 0.0, ..base.clone() },
            Question { feed_glucose_g_per_l: 0.0, ..base.clone() },
            Question { feed_galacturonate_g_per_l: 0.0, ..base.clone() },
        ] {
            assert_eq!(check_question(&question), Ok(()));
        }
        // What engine 0.12.0 held to the twelve compared cultures' own starts
        // and feeds, and the 0.6.0 ends with them, is asked of the physics now.
        for question in [
            Question { biomass_g_per_l: 0.3, ..base.clone() },
            Question { biomass_g_per_l: 1.5, ..base.clone() },
            Question { glucose_g_per_l: 19.0, ..base.clone() },
            Question { glucose_g_per_l: 50.0, ..base.clone() },
            Question { galacturonate_g_per_l: 3.5, ..base.clone() },
            Question { galacturonate_g_per_l: 5.5, ..base.clone() },
            Question { feed_glucose_g_per_l: 9.0, ..base.clone() },
            Question { feed_glucose_g_per_l: 54.0, ..base.clone() },
            Question { feed_galacturonate_g_per_l: 70.0, ..base.clone() },
            Question { feed_galacturonate_g_per_l: 111.0, ..base.clone() },
        ] {
            assert_eq!(check_question(&question), Ok(()));
        }
        assert_eq!(name(&Question { segment_end_h: &[], ..base.clone() }), "SegmentCountOutOfRange");
        assert_eq!(name(&Question { feed_ml: &[0.0, 6.0], ..base.clone() }), "SegmentsDoNotMatch");
        assert_eq!(name(&Question { segment_end_h: &[22.0, 22.0, 68.0], ..base.clone() }), "SegmentNotRising");
        assert_eq!(name(&Question { segment_end_h: &[22.0, 20.0, 68.0], ..base.clone() }), "SegmentNotRising");
        assert_eq!(name(&Question { segment_end_h: &[0.0, 44.0, 68.0], ..base.clone() }), "SegmentNotRising");
        assert_eq!(name(&Question { feed_ml: &[0.0, -0.1, 7.2], ..base.clone() }), "FeedRateNegative");
        assert_eq!(name(&Question { feed_ml: &[0.0, 4.4, below_zero], ..base.clone() }), "FeedRateNegative");
        // 0.4 mL/h, the old fastest feed, and a feed from inoculation are asked of the vessel now.
        for feed in [[0.0, 4.4, 0.4 * 24.0], [1.0, 4.4, 7.2], [0.0, 4.4, 0.0], [0.0, 0.0, 0.0]] {
            assert_eq!(check_question(&Question { feed_ml: &feed, ..base.clone() }), Ok(()), "{feed:?}");
        }
        // Water and samples are mass balance alone: only the broth leaving the vessel stops them.
        for water in [[-9.7, -1.0, 0.5], [-2.0, 7.3, 0.5], [-2.0, -1.0, -0.4 * 24.0], [-2.0, -1.0, 0.3 * 24.0]] {
            assert_eq!(check_question(&Question { water_ml: &water, ..base.clone() }), Ok(()), "{water:?}");
        }
        for samples in [[4.3, 4.31, 4.3], [4.3, 5.0, 4.3], [0.0, 50.0, 0.0]] {
            assert_eq!(check_question(&Question { sample_ml: &samples, ..base.clone() }), Ok(()), "{samples:?}");
        }
        assert_eq!(name(&Question { sample_ml: &[4.3, -1.0, 4.3], ..base.clone() }), "SampleNegative");
        // Broth drawn to nothing, or past what the vessel holds; a feed that overfills it.
        assert_eq!(name(&Question { sample_ml: &[4.3, 120.0, 4.3], ..base.clone() }), "VolumeOutOfRange");
        assert_eq!(name(&Question { water_ml: &[-110.0, -1.0, 0.5], ..base.clone() }), "VolumeOutOfRange");
        assert_eq!(name(&Question { water_ml: &[150.0, -1.0, 0.5], ..base.clone() }), "VolumeOutOfRange");
        assert_eq!(name(&Question { feed_ml: &[0.0, 4.4, 150.0], ..base.clone() }), "VolumeOutOfRange");
        let long = to(RUN_END_MAX_H + 0.01);
        assert_eq!(name(&asked(&long)), "RunTooLong");
        let many = vec![0.0; 33];
        let ends: Vec<f64> = (1..=33).map(|segment| 2.0 * f64::from(segment)).collect();
        assert_eq!(
            name(&Question { segment_end_h: &ends, feed_ml: &many, water_ml: &many, sample_ml: &many, ..base }),
            "SegmentCountOutOfRange"
        );
    }

    /// The exact decimal ends of every range are inside it, also where the
    /// limit is asked of a number the caller's arithmetic produced: a record
    /// that ends exactly at its capacity, and a volume that ends exactly at
    /// 250 mL.
    #[test]
    fn the_exact_ends_of_every_range_are_inside() {
        let record = short();
        let base = asked(&record);
        // The record ends exactly at its capacity and its last step is taken:
        // as one decimal and as the sum of 400 steps of 2.5 h, which is
        // exact. The capacity carries no allowance: the next number above it
        // is refused.
        assert_eq!(RUN_END_MAX_H, 1000.0);
        let summed = (0..400).fold(0.0_f64, |sum, _| sum + 2.5);
        assert_eq!(summed, RUN_END_MAX_H);
        for last in [215.01, COMPARED_RECORD_END_H, RUN_END_MAX_H, summed] {
            let record = to(last);
            assert_eq!(check_question(&asked(&record)), Ok(()), "{last}");
        }
        for past in [RUN_END_MAX_H.next_up(), RUN_END_MAX_H + 0.01, 1100.0] {
            let record = to(past);
            assert!(matches!(check_question(&asked(&record)), Err(CultureReject::RunTooLong { .. })), "{past}");
        }
        // The volume ends exactly at 250 mL, the vessel's top: 110 + 100 + 4.4 + 35.6.
        // A broth a ten-millionth of a mL above it is refused.
        assert_eq!(VOLUME_MAX_ML, 250.0);
        let high = Question {
            segment_end_h: &[22.0, 44.0],
            feed_ml: &[0.0, 4.4],
            water_ml: &[100.0, 35.6],
            sample_ml: &[0.0, 0.0],
            ..base.clone()
        };
        assert_eq!(check_question(&high), Ok(()));
        let over = Question { water_ml: &[100.0, 35.600_000_1], ..high.clone() };
        assert!(matches!(check_question(&over), Err(CultureReject::VolumeOutOfRange { .. })));
        // The same top reached by the feed alone, 110 + 140 mL in one segment.
        let fed = Question { feed_ml: &[0.0, 140.0], water_ml: &[0.0, 0.0], ..high.clone() };
        assert_eq!(check_question(&fed), Ok(()));
        let overfed = Question { feed_ml: &[0.0, 140.000_000_1], ..fed.clone() };
        assert!(matches!(check_question(&overfed), Err(CultureReject::VolumeOutOfRange { .. })));
        // 127.7 mL, the most the compared cultures held, at 259.28 h, is inside:
        // 110 + 4.18 + 4.4 + 4.18 + 12 - 7.06.
        let past = Question {
            segment_end_h: &[22.0, 44.0, 84.0],
            feed_ml: &[0.0, 4.4, 12.0],
            water_ml: &[4.18, 4.18, -7.06],
            sample_ml: &[0.0, 0.0, 0.0],
            ..base
        };
        assert_eq!(check_question(&past), Ok(()));
    }

    /// Past every wall engine 0.12.0 held at the twelve compared cultures'
    /// own starts, feeds, schedule and record end, the same equations are
    /// settled and both ledgers close: no biomass, ten times the biomass, no
    /// glucose and five times the glucose at the start, no galacturonate and
    /// thirty grams of it, a feed of no sugar at all and of 200 g/L glucose
    /// and 300 g/L galacturonate, a feed of 1 mL/h from inoculation until the
    /// vessel is nearly full, and a record of 1000 h. A vessel with no carbon
    /// at all supplies none and finds none.
    #[test]
    fn past_the_old_walls_the_ledgers_close() {
        let record = short();
        let base = asked(&record);
        let fast = (vec![22.0, 44.0, 68.0], vec![22.0, 22.0, 24.0], vec![-2.0, -1.0, 0.5], vec![4.3, 4.3, 4.3]);
        let thousand = to(1000.0);
        let questions = [
            Question { biomass_g_per_l: 0.0, ..base.clone() },
            Question { biomass_g_per_l: 9.0, ..base.clone() },
            Question { glucose_g_per_l: 0.0, ..base.clone() },
            Question { glucose_g_per_l: 100.0, ..base.clone() },
            Question { galacturonate_g_per_l: 0.0, ..base.clone() },
            Question { galacturonate_g_per_l: 30.0, ..base.clone() },
            Question { feed_glucose_g_per_l: 0.0, feed_galacturonate_g_per_l: 0.0, ..base.clone() },
            Question { feed_glucose_g_per_l: 200.0, feed_galacturonate_g_per_l: 300.0, ..base.clone() },
            asked(&fast),
            asked(&thousand),
        ];
        for (index, question) in questions.iter().enumerate() {
            let culture = solve(question).unwrap_or_else(|reject| panic!("{index}: {reject}"));
            assert!(culture.carbon_residual <= LEDGER_TOLERANCE, "{index}: {}", culture.carbon_residual);
            assert!(culture.electron_residual <= LEDGER_TOLERANCE, "{index}: {}", culture.electron_residual);
            assert!(culture.settle_gap <= SETTLE_TOLERANCE, "{index}");
            for end in 0..culture.biomass_g_per_l.len() {
                for value in [culture.biomass_g_per_l[end], culture.glucose_g_per_l[end], culture.galacturonate_g_per_l[end], culture.mucic_acid_g_per_l[end]] {
                    assert!(value >= 0.0, "{index}: {value}");
                }
            }
        }
        // No biomass: nothing grows, and the sugars are only fed and diluted.
        let empty = solve(&questions[0]).expect("settles");
        assert!(empty.biomass_g_per_l.iter().all(|x| *x == 0.0));
        assert!(empty.carbon_dioxide_mol.iter().all(|x| *x == 0.0) && empty.oxygen_mol.iter().all(|x| *x == 0.0));
        // Water and nothing else: no carbon supplied, none found, and the ledgers close at zero.
        let water = Question {
            biomass_g_per_l: 0.0,
            glucose_g_per_l: 0.0,
            galacturonate_g_per_l: 0.0,
            feed_glucose_g_per_l: 0.0,
            feed_galacturonate_g_per_l: 0.0,
            ..base
        };
        let nothing = solve(&water).expect("settles");
        assert_eq!((nothing.carbon_supplied_mol, nothing.carbon_residual, nothing.electron_residual), (0.0, 0.0, 0.0));
    }

    /// A segment of any length above zero is mass balance alone and is
    /// answered, where engine 0.8.0 held it to 1 to 100 h: a nanosecond, an
    /// hour less 5e-11 h, and one segment from the start of the feed to 140 h.
    /// The ledgers close on each.
    #[test]
    fn a_segment_of_any_length_above_zero_is_answered_with_its_ledgers_closed() {
        let record = short();
        let base = asked(&record);
        for length in [1.0e-9, 1.0 - 5.0e-11, 0.5] {
            let ends = [20.0, 20.0 + length, 22.0];
            let question = Question {
                segment_end_h: &ends,
                feed_ml: &[0.0, 0.0, 0.0],
                water_ml: &[0.0, 0.0, 0.0],
                sample_ml: &[0.0, 0.0, 4.3],
                ..base.clone()
            };
            let culture = solve(&question).unwrap_or_else(|reject| panic!("{length}: {reject}"));
            assert!(culture.carbon_residual <= LEDGER_TOLERANCE && culture.electron_residual <= LEDGER_TOLERANCE);
        }
        let long = Question {
            segment_end_h: &[22.0, 44.0, 140.0],
            feed_ml: &[0.0, 4.4, 0.25 * 96.0],
            water_ml: &[-2.0, -1.0, -0.2 * 96.0],
            sample_ml: &[4.3, 4.3, 4.3],
            ..base.clone()
        };
        let culture = solve(&long).expect("a 96 h segment");
        assert!(culture.carbon_residual <= LEDGER_TOLERANCE && culture.electron_residual <= LEDGER_TOLERANCE);
        assert!(matches!(
            check_question(&Question { segment_end_h: &[20.0, 20.0, 22.0], feed_ml: &[0.0; 3], water_ml: &[0.0; 3], sample_ml: &[0.0; 3], ..base }),
            Err(CultureReject::SegmentNotRising { segment: 1, .. })
        ));
    }

    /// The solver's own four held-back questions, as the scored run asked
    /// them (`evidence/research/bioreactor_trichoderma_ambr250/analysis/out/06_sealed`),
    /// refused through engine 0.8.0: each is answered with its ledgers
    /// closed, and with the numbers the scored run sealed, so the miss stated
    /// in the description is the miss of the answer served.
    const HELD_BACK: [(&str, f64, f64, f64, f64, f64, &[f64], &[f64], &[f64], &[f64]); 4] = [
        ("T2 glucose 10 1", 0.9333, 20.541, 4.745, 10.077, 97.25800000000001, &[21.1823338888889, 45.1823338888889, 69.1823338888889, 93.1823338888889, 117.1823338888889, 141.1823338888889, 165.1823338888889, 189.1823338888889, 213.1823338888889, 312.0], &[0.0, 2.2769408513932, 7.4209772819344595, 7.126034584128041, 6.795772697699199, 6.637834842954099, 6.311979784123904, 6.061521218783299, 5.811407332876499, 11.956929094107302], &[-6.384243795763449, 0.6847229753975006, 2.751279936774848, -6.213012287497054, 2.1313274573018024, -6.1064440913340885, -1.9293141497579083, -1.9248653192173013, -2.0446094239434958, -4.354164816886855], &[4.3, 4.3, 4.3, 4.3, 4.3, 4.3, 4.3, 4.3, 4.3, 4.3]),
        ("T2 glucose 10 2", 1.0333333333332195, 21.151, 4.6305, 11.06, 89.257, &[22.8673958333333, 43.4507291666667, 67.4507291666667, 91.4507291666667, 115.450729166667, 139.450729166667, 163.450729166667, 187.2840625, 211.2840625, 259.2840625, 307.117395833333], &[0.0, 4.51836207858949, 7.472844384940009, 7.197304398563, 6.946628554463299, 6.671063026052604, 6.445085424432797, 6.175618814988198, 5.993123871752005, 11.583537372221599, 0.4517331819969996], &[3.214795438537325, 1.3679951320698387, -1.9775159781539906, -2.5991765081020115, -2.4763898984103108, -2.477659577320602, -1.2401505950967968, -2.2103315286438456, -2.5540059515530062, -2.752333323911613, -4.280225521913631], &[4.3, 4.3, 4.3, 4.3, 4.3, 4.3, 4.3, 4.3, 4.3, 4.3, 4.3]),
        ("T2 glucose 50 1", 0.9333, 20.632, 4.7735, 48.32, 97.5675, &[21.1823338888889, 45.1823338888889, 69.1823338888889, 93.1823338888889, 117.1823338888889, 141.1823338888889, 165.1823338888889, 189.1823338888889, 213.1823338888889, 312.0], &[0.0, 2.29150650906139, 7.420968778364999, 7.127297833556611, 6.820693240775299, 6.6458794738747, 6.311820310830203, 6.061799921393998, 5.8362764654713, 12.000256890671501], &[-6.612332810480509, 0.5947533412406093, 2.7469853750561093, -6.044975007959632, 2.6719153088087086, -5.586437587037705, -1.3672874540871918, -1.615154212668, -2.0530505676923068, -4.183223831576091], &[4.3, 4.3, 4.3, 4.3, 4.3, 4.3, 4.3, 4.3, 4.3, 4.3]),
        ("T2 glucose 50 2", 1.06666666666662, 21.075, 4.609, 52.101, 90.34700000000001, &[22.8673958333333, 43.4507291666667, 67.4507291666667, 91.4507291666667, 115.450729166667, 139.450729166667, 163.450729166667, 187.2840625, 211.2840625, 259.2840625, 307.117395833333], &[0.0, 3.98615105523926, 7.498069897446941, 7.222724768365399, 6.947128822391399, 6.721148836119002, 6.470608684806996, 6.200873558819204, 6.042896426217794, 11.684024275857503, 0.44569881273650225], &[5.222295298264004, 1.6436722391097376, -1.4507605869519429, -1.9660635361234005, -2.080519954461412, -2.058702748594001, -0.8264547721280067, -1.9127412685511762, -2.4522179820667955, -0.6034437503395136, -5.363169304083474], &[4.3, 4.3, 4.3, 4.3, 4.3, 4.3, 4.3, 4.3, 4.3, 4.3, 4.3]),
    ];
    /// The scored run's last mucic acid (g/L), galacturonate (g/L) and carbon
    /// dioxide (mol) of each held-back culture.
    const HELD_BACK_SCORED: [(f64, f64, f64); 4] = [
        (43.449298498570336, 9.90683556005122, 0.0723017981915361),
        (43.923209932375805, 4.627835465931725, 0.07684392519039737),
        (53.364552804521665, 0.03746354455317878, 0.12516486902317597),
        (47.16596778679385, 0.0014649925858301598, 0.13611192796213184),
    ];

    #[test]
    fn the_four_held_back_questions_are_answered_as_they_were_scored() {
        for ((culture, biomass, glucose, galacturonate, feed_glucose, feed_galacturonate, ends, feed, water, samples), scored) in
            HELD_BACK.iter().zip(HELD_BACK_SCORED)
        {
            let question = Question {
                biomass_g_per_l: *biomass,
                glucose_g_per_l: *glucose,
                galacturonate_g_per_l: *galacturonate,
                feed_glucose_g_per_l: *feed_glucose,
                feed_galacturonate_g_per_l: *feed_galacturonate,
                segment_end_h: ends,
                feed_ml: feed,
                water_ml: water,
                sample_ml: samples,
            };
            let answer = solve(&question).unwrap_or_else(|reject| panic!("{culture}: {reject}"));
            assert!(answer.carbon_residual <= LEDGER_TOLERANCE, "{culture}");
            assert!(answer.electron_residual <= LEDGER_TOLERANCE, "{culture}");
            let last = ends.len() - 1;
            for (got, want) in [
                (answer.mucic_acid_g_per_l[last], scored.0),
                (answer.galacturonate_g_per_l[last], scored.1),
                (answer.carbon_dioxide_mol[last], scored.2),
            ] {
                assert!((got - want).abs() <= 1.0e-9 * want.abs().max(1.0), "{culture}: {got} against {want}");
            }
        }
    }

    /// The eight fitted cultures' own records to 215 h are answered, and so
    /// is every schedule none of them ran, which engine 0.12.0 refused by
    /// name: fed before 21.18 h, started faster than 0.2234 mL/h or slower
    /// than 0.0913 mL/h, cut back or stopped once the feed is full, one even
    /// rate from the start window into the full feed, unfed to 215 h, and
    /// 0.4 mL/h from inoculation. Each closes both ledgers.
    #[test]
    fn every_feed_schedule_is_answered_with_its_ledgers_closed() {
        for (culture, biomass, glucose, galacturonate, feed_glucose, feed_galacturonate, ends, feed, water) in FITTED {
            let question = Question {
                biomass_g_per_l: biomass,
                glucose_g_per_l: glucose,
                galacturonate_g_per_l: galacturonate,
                feed_glucose_g_per_l: feed_glucose,
                feed_galacturonate_g_per_l: feed_galacturonate,
                segment_end_h: &ends,
                feed_ml: &feed,
                water_ml: &water,
                sample_ml: &[4.3; 9],
            };
            assert_eq!(check_question(&question), Ok(()), "{culture}");
        }
        let record = short();
        let base = asked(&record);
        let settled = |ends: &[f64], feed: &[f64]| {
            let water = vec![0.0; ends.len()];
            let samples = vec![0.0; ends.len()];
            let culture = solve(&Question { segment_end_h: ends, feed_ml: feed, water_ml: &water, sample_ml: &samples, ..base.clone() })
                .unwrap_or_else(|reject| panic!("{ends:?} {feed:?}: {reject}"));
            assert!(culture.carbon_residual <= LEDGER_TOLERANCE && culture.electron_residual <= LEDGER_TOLERANCE, "{ends:?} {feed:?}");
        };
        settled(&[22.0, 44.0], &[0.5, 4.4]);
        settled(&[20.0, 44.0], &[0.0, 4.4]);
        settled(&[24.0, 44.0], &[0.0, 4.4]);
        settled(&[22.0, 44.0], &[0.0, 0.2235 * 22.0]);
        settled(&[22.0, 44.0], &[0.0, 0.0912 * 22.0]);
        settled(&[22.0, 44.0, 68.0], &[0.0, 4.4, 0.2419 * 24.0]);
        settled(&[22.0, 44.0, 68.0, 92.0], &[0.0, 4.4, 7.2, 0.0]);
        settled(&[22.0, 50.0], &[0.0, 0.23 * 28.0]);
        settled(&[21.5, 45.0, 120.0, 211.28, 259.28, 307.12], &[0.0, 2.25, 18.75, 0.25 * 91.28, 0.25 * 48.0, 0.0]);
        settled(&[24.0, 48.0, 72.0, 96.0, 120.0, 144.0, 168.0, 192.0, 215.0], &[0.0; 9]);
        settled(&[100.0], &[40.0]);
    }

    /// Every sentence a refusal writes counts segments from 1 and writes no
    /// range in program notation.
    #[test]
    fn a_refusal_counts_segments_from_one() {
        let record = short();
        let base = asked(&record);
        let refused = check_question(&Question { feed_ml: &[0.0, -0.5, 7.2], ..base.clone() }).expect_err("a negative feed").to_string();
        assert!(refused.starts_with("segment 2 is fed -0.5 mL, below zero"), "{refused}");
        let refused = check_question(&Question { water_ml: &[-2.0, -1.0, 200.0], ..base }).expect_err("past the vessel").to_string();
        assert!(refused.contains("at the end of segment 3"), "{refused}");
        for sentence in [refused] {
            assert!(!sentence.contains("..="), "{sentence}");
        }
    }

    /// With no feed and no biomass decay to speak of, what the culture does
    /// is what the reactions say: at the end of a batch the glucose is gone
    /// and biomass carbon plus carbon dioxide is the glucose carbon, less
    /// what decay returned. No compared culture went unfed past 22.87 h;
    /// engine 0.12.0 refused this record, and it is answered now.
    #[test]
    fn a_batch_ends_with_its_glucose_gone() {
        let ends = [60.0, 100.0];
        let question = Question {
            biomass_g_per_l: 0.9,
            glucose_g_per_l: 20.5,
            galacturonate_g_per_l: 4.6,
            feed_glucose_g_per_l: 32.0,
            feed_galacturonate_g_per_l: 94.0,
            segment_end_h: &ends,
            feed_ml: &[0.0, 0.0],
            water_ml: &[0.0, 0.0],
            sample_ml: &[0.0, 0.0],
        };
        let culture = solve(&question).expect("settles");
        assert_eq!(culture, integrate(&Reactions::of_the_specimen(), &question).expect("settles"));
        assert!(culture.glucose_g_per_l[1] < 1.0e-3);
        assert!(culture.biomass_g_per_l[0] > 7.0 && culture.biomass_g_per_l[0] < 9.5);
        assert!(culture.biomass_g_per_l[1] < culture.biomass_g_per_l[0]);
        assert!(culture.mucic_acid_g_per_l[1] > 3.0);
        assert!(culture.mucic_acid_g_per_l[1] < 4.6 * MUCIC_ACID_G_PER_MOL / GALACTURONATE_G_PER_MOL);
        let respiratory_quotient = culture.carbon_dioxide_mol[1] / culture.oxygen_mol[1];
        assert!(respiratory_quotient > 0.85 && respiratory_quotient < 1.1, "{respiratory_quotient}");
    }

    /// The red team's culture of engine 0.13.0 (D1): the worked culture's
    /// start, unfed to 312 h. Its glucose and galacturonate come to 5.4e-323
    /// and 2.4e-322 g/L as computed, under the smallest normal number; each
    /// is answered as +0, every number of the answer is normal or zero, every
    /// other number is the computed one bit for bit, and both ledgers close.
    #[test]
    fn a_concentration_under_the_smallest_normal_number_is_answered_as_zero() {
        use crate::normal::NormalOrZero;
        let question = Question {
            biomass_g_per_l: 0.9,
            glucose_g_per_l: 20.8,
            galacturonate_g_per_l: 4.6,
            feed_glucose_g_per_l: 32.0,
            feed_galacturonate_g_per_l: 94.0,
            segment_end_h: &[312.0],
            feed_ml: &[0.0],
            water_ml: &[0.0],
            sample_ml: &[0.0],
        };
        let computed = solve_as_computed(&question).expect("settles");
        let answered = solve(&question).expect("settles");
        assert!(computed.glucose_g_per_l[0].is_subnormal() && computed.galacturonate_g_per_l[0].is_subnormal());
        assert_eq!((answered.glucose_g_per_l[0].to_bits(), answered.galacturonate_g_per_l[0].to_bits()), (0, 0));
        let mut held = computed.clone();
        held.normal_or_zero();
        assert_eq!(held, answered);
        let mut again = answered.clone();
        again.normal_or_zero();
        assert_eq!(again, answered);
        assert!(answered.carbon_residual <= LEDGER_TOLERANCE && answered.electron_residual <= LEDGER_TOLERANCE);
    }
}
