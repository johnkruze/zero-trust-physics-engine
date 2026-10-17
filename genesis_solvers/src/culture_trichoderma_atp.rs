//! One fed-batch culture of the engineered marine fungus *Trichoderma* sp.
//! VTT D-221704 in an ambr 250 vessel at 35 °C, growing on glucose and
//! oxidising D-galacturonic acid to mucic acid on the energy the glucose
//! supplies: biomass, glucose, galacturonate, mucic acid, broth volume, carbon
//! dioxide made, oxygen taken up and galacturonate burnt, at the end of every
//! segment of the caller's operating record. The companion of
//! `culture_trichoderma_ambr250`, whose record asked for this form: there the
//! galacturonate taken up is oxidised in one fixed fraction whatever the
//! glucose, and the cultures fed a third of the glucose showed that it is not.
//!
//! The specimen and its data. Tamminen, Turunen, Barth, Vidgren and Wiebe,
//! "Use of ambr®250 to assess mucic acid production in fed-batch cultures of
//! a marine Trichoderma sp. D-221704", AMB Express 12 (2022) 90, and its
//! data, Zenodo record 5846028, CC BY 4.0: cultures of 110 mL at 1100 rpm and
//! 100 mL/min of air, each with the off-gas analysed every five minutes and
//! the broth sampled about once a day for dry biomass, glucose, galacturonate
//! and mucic acid. The record in the research folder is
//! `evidence/research/bioreactor_trichoderma_ambr250/companion_atp/`.
//!
//! The form. The glucose taken up is the culture's energy supply: its yield
//! is fixed, so the glucose it respires is a fixed share of what it takes up.
//! That supply sets the share `phi = q_G / (q_G + K_E)` of the galacturonate
//! machinery that runs to mucic acid: oxidation goes at up to
//! [`GALACTURONATE_OXIDATION_MAX_G_PER_G_H`] times `phi`, and burning to
//! carbon dioxide at up to [`GALACTURONATE_BURNING_MAX_G_PER_G_H`] times
//! `1 − phi`. With glucose plentiful against [`GLUCOSE_ENERGY_HALF_G_PER_G_H`]
//! almost none is burnt; as the glucose supply falls the burnt fraction rises,
//! and with no glucose all the galacturonate taken up is burnt. Yeast extract
//! in the batch medium raises the largest glucose uptake by
//! [`YEAST_EXTRACT_GAIN_L_PER_G`] for each g/L; its carbon is not counted.
//!
//! The eight fitted constants. [`GLUCOSE_UPTAKE_MAX_G_PER_G_H`],
//! [`YEAST_EXTRACT_GAIN_L_PER_G`], [`BIOMASS_YIELD_G_PER_G`],
//! [`BIOMASS_DECAY_PER_H`], [`GALACTURONATE_OXIDATION_MAX_G_PER_G_H`],
//! [`GLUCOSE_REPRESSION_G_PER_L`], [`GLUCOSE_ENERGY_HALF_G_PER_G_H`] and
//! [`GALACTURONATE_BURNING_MAX_G_PER_G_H`] are fitted by least squares to
//! twelve of this organism's cultures in this vessel, all at pH 4.0: fed 10.1
//! to 52.1 g/L glucose with 88.5 to 103.3 g/L galacturonate, with 0, 0.5, 1 or
//! 2 g/L yeast extract (the eight the first solver was fitted to and the four
//! it was scored on). The two saturation constants,
//! [`GLUCOSE_SATURATION_G_PER_L`] and [`GALACTURONATE_SATURATION_G_PER_L`],
//! are the first solver's, set and not fitted. The elemental formulas are the
//! compounds' own, and biomass is taken as CH1.8O0.5N0.2, 24.6 g for each
//! mole of carbon, a textbook composition that was not measured here. The
//! coupling's two constants are not sharply fixed: a second minimum of the fit
//! 0.7 above this one has the burning's largest rate 0.0126 and the energy
//! half 0.00152, and fitted without the cultures fed a third of the glucose
//! the energy half falls to 0.00022.
//!
//! pH. The twelve fitted cultures were at pH 4.0, so no constant is fitted to
//! pH. The forecast fixed before the scoring is that the kinetics do not
//! depend on pH from 3.0 to 4.0: the pH is an input and moves no number. It
//! is held to the scale of a dilute aqueous broth, 0 to 14
//! (`PhOutsideAqueousScale`); outside [`COMPARED_PH`] the answer is the same
//! arithmetic and carries `ph_outside_compared`.
//!
//! The comparison with cultures no one had read, made once on 2026-10-15.
//! Held back whole: the four cultures at pH 3.5 and 3.0, run beside the pH
//! 4.0 pair with the same medium and feed. The protocol (SHA-256
//! af5c49e15cf9236922ebe9fc2bde272b10b06bd294d1d5372ce56629e695fa33) was
//! committed before any of their values was read, with the first solver's six
//! lines unchanged and the forecast written out: missed, the mucic acid and
//! carbon dioxide lines missed and the other four met if pH changes nothing.
//! The solver did not pass: it met three (galacturonate 1.83 g/L against
//! 2.74, mucic acid 4.19 against 4.51, oxygen 0.01396 mol against 0.0153) and
//! missed three (dry biomass to 170 h 3.64 g/L against 2.15, glucose 1.80
//! against 0.85, carbon dioxide 0.01065 mol against 0.00265). Of the three
//! met only mucic acid is evidence for the model: the mean of the pH 4.0 pair
//! meets the galacturonate and oxygen lines too. At pH 3.0 the measured batch
//! ran slower than at 4.0, so the pH statement does not hold there; at pH 3.5
//! and 3.0 the measured dry weight climbed from about 90 h, where at pH 4.0
//! it rose only once mucic acid passed about 35 g/L. The solver is not
//! validated against measurement.
//!
//! What the solver answers. Every start, feed, schedule and record the
//! equations can take, held only by their own limits: a concentration is not
//! below zero, a feed is not below zero, the pH is on the aqueous scale, the
//! broth stays in the vessel, above zero and up to [`VOLUME_MAX_ML`], a
//! concentration the settled record drives below zero is refused
//! (`ConcentrationBelowZero`), a step that does not settle is refused
//! (`NotSettled`), and both ledgers must close. A record ends by
//! [`RUN_END_MAX_H`], the capacity of one solve. Water only dilutes and a
//! sample takes broth as it is. Every compound is taken as dissolved, as the
//! mucic acid crystals are.
//!
//! What was compared, stated and not enforced. The twelve fitted cultures were
//! started at 0.600 to 1.067 g/L biomass, 20.097 to 21.151 g/L glucose and
//! 4.146 to 4.774 g/L galacturonate, fed 10.077 to 52.101 g/L glucose and
//! 88.5 to 103.3 g/L galacturonate, with 0 to 2 g/L yeast extract, at pH 4.0,
//! their records to 312 h. Outside the compared feed glucose, yeast extract,
//! pH and record the answer carries `feed_glucose_outside_compared`,
//! `yeast_extract_outside_compared`, `ph_outside_compared` or
//! `record_past_compared`.
//!
//! The equations. Per litre of broth, with biomass `X`, glucose `G`,
//! galacturonate `U`, mucic acid `P` in g/L, volume `V`, feed `F` and water
//! `W` in L/h, feed concentrations `G_f` and `U_f`, yeast extract `E`:
//!
//! ```text
//! q_G  = q_Gmax (1 + a_E E) G / (K_G + G)            g glucose per g biomass per hour
//! phi  = q_G / (q_G + K_E)
//! q_ox = q_Umax U / (K_U + U) · K_I / (K_I + G) · phi g galacturonate oxidised per g biomass per hour
//! q_B  = q_Bmax U / (K_U + U) · (1 − phi)            g galacturonate burnt per g biomass per hour
//! dX/dt = (Y q_G − k_d) X − (F + W)/V X
//! dG/dt = −q_G X + F/V G_f − (F + W)/V G
//! dU/dt = −(q_ox + q_B) X + F/V U_f − (F + W)/V U
//! dP/dt = (M_P/M_U) q_ox X − (F + W)/V P
//! dV/dt = F + W
//! ```
//!
//! Four reactions carry the carbon and the electrons, as in the first solver.
//! Growth: one C-mole of glucose gives `y_c` C-mole of biomass and `1 − y_c` of
//! carbon dioxide and takes `(4 − 4.2 y_c)/4` mole of oxygen. Decay: one
//! C-mole of biomass gives one of carbon dioxide and takes 1.05 of oxygen.
//! Oxidation: one mole of galacturonate and half a mole of oxygen give one
//! mole of mucic acid. Burning: one mole of galacturonate and five of oxygen
//! give six of carbon dioxide. A sample is drawn at the end of its segment, at
//! once, and takes broth as it is.
//!
//! The two ledgers. Carbon supplied (the charge and the feed) against carbon
//! found (broth, samples drawn, carbon dioxide); electrons supplied against
//! electrons found (broth, samples drawn, four for each mole of oxygen). Each
//! must close within [`LEDGER_TOLERANCE`] of what was supplied. They close for
//! any values of the fitted constants: they hold the integration and the table
//! of reactions, not the biology.
//!
//! Settling. Every segment is stepped by classical Runge-Kutta in equal steps
//! no longer than [`FIRST_STEP_H`], then with every step halved, and again,
//! until no published number moves by more than [`SETTLE_TOLERANCE`] of its
//! scale between one halving and the next. The finer of the last two is
//! returned.
//!
//! Range ends. The broth volume, a number the caller's arithmetic produces,
//! is held to the vessel with an allowance of [`RANGE_END_ALLOWANCE`] of the
//! limit itself, so that a broth that ends exactly at 250 mL in decimal is
//! inside it in binary. A limit at zero has none; the pH's ends, 0 and 14,
//! are exact in binary and have none; and the record's end, a capacity, has
//! none.

use crate::normal::NormalOrZero;
use std::fmt;

/// Largest glucose uptake with no yeast extract, g glucose per g biomass per hour. Fitted.
pub const GLUCOSE_UPTAKE_MAX_G_PER_G_H: f64 = 0.1172;
/// Rise of the largest glucose uptake for each g/L of yeast extract in the batch medium, L/g. Fitted.
pub const YEAST_EXTRACT_GAIN_L_PER_G: f64 = 0.1195;
/// Biomass formed per glucose taken up, g per g. Fitted.
pub const BIOMASS_YIELD_G_PER_G: f64 = 0.3718;
/// Biomass lost to its own respiration, per hour. Fitted.
pub const BIOMASS_DECAY_PER_H: f64 = 0.001101;
/// Largest galacturonate oxidation to mucic acid, g galacturonate per g biomass per hour. Fitted.
pub const GALACTURONATE_OXIDATION_MAX_G_PER_G_H: f64 = 0.02996;
/// Glucose at which galacturonate uptake for oxidation is halved, g/L. Fitted.
pub const GLUCOSE_REPRESSION_G_PER_L: f64 = 6.837;
/// Glucose uptake at which oxidation and burning share the galacturonate machinery equally, g per g biomass per hour. Fitted.
pub const GLUCOSE_ENERGY_HALF_G_PER_G_H: f64 = 0.001418;
/// Largest galacturonate burning to carbon dioxide, g galacturonate per g biomass per hour. Fitted.
pub const GALACTURONATE_BURNING_MAX_G_PER_G_H: f64 = 0.01415;
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
/// The lowest pH of a dilute aqueous broth: below it hydrogen ions exceed 1 mol/L.
pub const PH_MIN: f64 = 0.0;
/// The highest pH of a dilute aqueous broth: above it hydroxide ions exceed about 1 mol/L.
pub const PH_MAX: f64 = 14.0;
/// The pH compared with measurement: fitted at 4.0, the forecast scored at
/// 3.5 and 3.0. Stated, not enforced: outside it the answer carries
/// `ph_outside_compared`.
pub const COMPARED_PH: (f64, f64) = (3.0, 4.0);
/// The yeast extract of the fitted cultures' batch medium, g/L. Stated, not enforced.
pub const COMPARED_YEAST_EXTRACT_G_PER_L: (f64, f64) = (0.0, 2.0);
/// The glucose in the feed of the fitted cultures, g/L: T2 glucose 10 (1) to
/// T2 glucose 50 (2). Stated, not enforced.
pub const COMPARED_FEED_GLUCOSE_G_PER_L: (f64, f64) = (10.077, 52.101);
/// The end of the fitted cultures' records, h. Stated, not enforced.
pub const COMPARED_RECORD_END_H: f64 = 312.0;
/// Fewest segments of an operating record.
pub const SEGMENTS_MIN: usize = 1;
/// Most segments of an operating record.
pub const SEGMENTS_MAX: usize = 32;
/// Latest end of a record, h after inoculation: the capacity of one solve, and
/// no physics. A solve steps every hour of the record in steps of at most
/// [`FIRST_STEP_H`], then with every step halved, up to [`MAX_HALVINGS`]
/// times: up to 511 passes' worth, about 10,240 Runge-Kutta steps for each
/// hour.
pub const RUN_END_MAX_H: f64 = 1000.0;
/// Most broth the vessel holds, mL: the top of the ambr 250 vessel's working
/// volume. Below zero there is no broth; at zero the dilution rate is not
/// finite.
pub const VOLUME_MAX_ML: f64 = 250.0;
/// Fraction of a limit by which a number may pass it: decimal ends are inside.
pub const RANGE_END_ALLOWANCE: f64 = 1.0e-12;

/// Longest step of the first pass, h.
pub const FIRST_STEP_H: f64 = 0.05;
/// Most times the step is halved.
pub const MAX_HALVINGS: u32 = 8;
/// Largest move of a published number between two halvings, as a fraction of its scale.
pub const SETTLE_TOLERANCE: f64 = 1.0e-9;
/// Scale of a concentration for settling, g/L.
pub const SETTLE_SCALE_G_PER_L: f64 = 1.0;
/// Scale of an amount of gas or of galacturonate burnt for settling, mol.
pub const SETTLE_SCALE_MOL: f64 = 1.0e-3;
/// Largest carbon or electron imbalance, as a fraction of what was supplied.
pub const LEDGER_TOLERANCE: f64 = 1.0e-9;

/// The public source of the measurements the reference test reads.
pub const REFERENCE_NAME: &str = "Tamminen et al., Data for growth of marine Trichoderma sp. T2 in ambr250 to produce mucic acid, Zenodo record 5846028, sheet T2 pH 4 (1)";
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
    /// `yeast_extract_g_per_l` is below zero: a concentration is not negative.
    YeastExtractNegative { yeast_extract_g_per_l: f64 },
    /// `ph` is below 0 or above 14, off the scale of a dilute aqueous broth: hydrogen or hydroxide ions above about 1 mol/L make a concentrated acid or alkali.
    PhOutsideAqueousScale { ph: f64 },
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
            Self::YeastExtractNegative { .. } => "YeastExtractNegative",
            Self::PhOutsideAqueousScale { .. } => "PhOutsideAqueousScale",
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
            Self::YeastExtractNegative { yeast_extract_g_per_l } => write!(
                f,
                "yeast extract {yeast_extract_g_per_l} g/L is below zero: a concentration is not negative"
            ),
            Self::PhOutsideAqueousScale { ph } => write!(
                f,
                "pH {ph} is below {PH_MIN} or above {PH_MAX}, off the scale of a dilute aqueous broth: hydrogen or hydroxide ions above about 1 mol/L make a concentrated acid or alkali"
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
    /// Yeast extract in the batch medium, g/L.
    pub yeast_extract_g_per_l: f64,
    /// The broth's pH, held through the record.
    pub ph: f64,
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
    /// Galacturonate burnt to carbon dioxide since inoculation, mol.
    pub galacturonate_burnt_mol: Vec<f64>,
    /// Carbon supplied by the charge and the feed over the whole record, mol.
    pub carbon_supplied_mol: f64,
    /// The pH is outside [`COMPARED_PH`].
    pub ph_outside_compared: bool,
    /// The yeast extract is outside [`COMPARED_YEAST_EXTRACT_G_PER_L`].
    pub yeast_extract_outside_compared: bool,
    /// The feed glucose is outside [`COMPARED_FEED_GLUCOSE_G_PER_L`].
    pub feed_glucose_outside_compared: bool,
    /// The record ends after [`COMPARED_RECORD_END_H`].
    pub record_past_compared: bool,
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
/// dioxide, oxygen and galacturonate burnt (mol).
type State = [f64; 8];

/// What is held over one segment.
#[derive(Clone, Copy)]
struct Held {
    feed_l_per_h: f64,
    water_l_per_h: f64,
    feed_glucose_g_per_l: f64,
    feed_galacturonate_g_per_l: f64,
    /// The largest glucose uptake at this medium's yeast extract.
    glucose_uptake_max: f64,
}

fn rates(reactions: &Reactions, y: &State, held: &Held) -> State {
    let [biomass, glucose, galacturonate, mucic, volume, _, _, _] = *y;
    let glucose_uptake = if glucose > 0.0 {
        held.glucose_uptake_max * glucose / (GLUCOSE_SATURATION_G_PER_L + glucose)
    } else {
        0.0
    };
    // The share of the galacturonate machinery the glucose's energy runs to mucic acid.
    let share = glucose_uptake / (glucose_uptake + GLUCOSE_ENERGY_HALF_G_PER_G_H);
    let saturation = if galacturonate > 0.0 {
        galacturonate / (GALACTURONATE_SATURATION_G_PER_L + galacturonate)
    } else {
        0.0
    };
    let repression = if glucose > 0.0 {
        GLUCOSE_REPRESSION_G_PER_L / (GLUCOSE_REPRESSION_G_PER_L + glucose)
    } else {
        1.0
    };
    let oxidised = GALACTURONATE_OXIDATION_MAX_G_PER_G_H * saturation * repression * share;
    let burnt = GALACTURONATE_BURNING_MAX_G_PER_G_H * saturation * (1.0 - share);
    // Reaction rates for each litre of broth and hour.
    let growth_cmol = glucose_uptake * biomass / GLUCOSE_G_PER_MOL * 6.0;
    let decay_cmol = BIOMASS_DECAY_PER_H * biomass / BIOMASS_G_PER_CMOL;
    let oxidation_mol = oxidised * biomass / GALACTURONATE_G_PER_MOL;
    let burning_mol = burnt * biomass / GALACTURONATE_G_PER_MOL;
    let dilution = (held.feed_l_per_h + held.water_l_per_h) / volume;
    let feed = held.feed_l_per_h / volume;
    [
        (carbon_yield() * growth_cmol - decay_cmol) * BIOMASS_G_PER_CMOL - dilution * biomass,
        -growth_cmol / 6.0 * GLUCOSE_G_PER_MOL + feed * held.feed_glucose_g_per_l - dilution * glucose,
        -(oxidation_mol + burning_mol) * GALACTURONATE_G_PER_MOL + feed * held.feed_galacturonate_g_per_l
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
        burning_mol * volume,
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
    for index in 0..8 {
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
        0.0,
    ];
    let glucose_uptake_max = GLUCOSE_UPTAKE_MAX_G_PER_G_H * (1.0 + YEAST_EXTRACT_GAIN_L_PER_G * question.yeast_extract_g_per_l);
    let feed_carbon_per_l = 6.0 * question.feed_glucose_g_per_l / GLUCOSE_G_PER_MOL
        + 6.0 * question.feed_galacturonate_g_per_l / GALACTURONATE_G_PER_MOL;
    let feed_electrons_per_l = GLUCOSE_ELECTRONS_PER_MOL * question.feed_glucose_g_per_l / GLUCOSE_G_PER_MOL
        + GALACTURONATE_ELECTRONS_PER_MOL * question.feed_galacturonate_g_per_l / GALACTURONATE_G_PER_MOL;
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
            glucose_uptake_max,
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
    Pass { ends, carbon_supplied_mol: carbon_supplied, carbon_residual, electron_residual }
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
    value >= minimum - RANGE_END_ALLOWANCE * minimum.abs() && value <= maximum + RANGE_END_ALLOWANCE * maximum.abs()
}

/// Every limit the question itself shows, in the order the solver asks them.
/// Nothing is integrated.
pub fn check_question(question: &Question<'_>) -> Result<(), CultureReject> {
    let segments = question.segment_end_h.len();
    let all = [
        question.biomass_g_per_l,
        question.glucose_g_per_l,
        question.galacturonate_g_per_l,
        question.yeast_extract_g_per_l,
        question.ph,
        question.feed_glucose_g_per_l,
        question.feed_galacturonate_g_per_l,
    ];
    let arrays = [question.segment_end_h, question.feed_ml, question.water_ml, question.sample_ml];
    if all.iter().any(|value| !value.is_finite()) || arrays.iter().any(|array| array.iter().any(|value| !value.is_finite())) {
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
    if !(question.yeast_extract_g_per_l >= 0.0) {
        return Err(CultureReject::YeastExtractNegative { yeast_extract_g_per_l: question.yeast_extract_g_per_l });
    }
    if !(question.ph >= PH_MIN && question.ph <= PH_MAX) {
        return Err(CultureReject::PhOutsideAqueousScale { ph: question.ph });
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
    if question.feed_ml.len() != segments || question.water_ml.len() != segments || question.sample_ml.len() != segments {
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
        self.galacturonate_burnt_mol.normal_or_zero();
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
            for index in 0..8 {
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
            return accept(question, fine, gap, halvings);
        }
        if halvings >= MAX_HALVINGS {
            return Err(CultureReject::NotSettled { halvings });
        }
        coarse = fine;
    }
}

fn outside(value: f64, range: (f64, f64)) -> bool {
    !(value >= range.0 && value <= range.1)
}

fn accept(question: &Question<'_>, settled: Pass, settle_gap: f64, halvings: u32) -> Result<Culture, CultureReject> {
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
    let end_h = question.segment_end_h.last().copied().unwrap_or(0.0);
    Ok(Culture {
        biomass_g_per_l: column(0, 1.0),
        glucose_g_per_l: column(1, 1.0),
        galacturonate_g_per_l: column(2, 1.0),
        mucic_acid_g_per_l: column(3, 1.0),
        volume_ml: column(4, 1000.0),
        carbon_dioxide_mol: column(5, 1.0),
        oxygen_mol: column(6, 1.0),
        galacturonate_burnt_mol: column(7, 1.0),
        carbon_supplied_mol: settled.carbon_supplied_mol,
        ph_outside_compared: outside(question.ph, COMPARED_PH),
        yeast_extract_outside_compared: outside(question.yeast_extract_g_per_l, COMPARED_YEAST_EXTRACT_G_PER_L),
        feed_glucose_outside_compared: outside(question.feed_glucose_g_per_l, COMPARED_FEED_GLUCOSE_G_PER_L),
        record_past_compared: end_h > COMPARED_RECORD_END_H,
        carbon_residual: settled.carbon_residual,
        electron_residual: settled.electron_residual,
        settle_gap,
        halvings,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    // The protocol's two stand-in records, T2 pH 4 (1) and (2) of Zenodo
    // record 5846028, each cut where its feed starts and stops, with the
    // answer of the protocol's reference program (atp_model.py, steps of
    // 0.0125 h) at every segment end; and the measured values of T2 pH 4 (1).
    include!("culture_trichoderma_atp_reference.rs");

    fn stand_in_1(ph: f64) -> Question<'static> {
        Question {
            biomass_g_per_l: STAND_IN_1_BIOMASS_G_PER_L,
            glucose_g_per_l: STAND_IN_1_GLUCOSE_G_PER_L,
            galacturonate_g_per_l: STAND_IN_1_GALACTURONATE_G_PER_L,
            yeast_extract_g_per_l: STAND_IN_1_YEAST_EXTRACT_G_PER_L,
            ph,
            feed_glucose_g_per_l: STAND_IN_1_FEED_GLUCOSE_G_PER_L,
            feed_galacturonate_g_per_l: STAND_IN_1_FEED_GALACTURONATE_G_PER_L,
            segment_end_h: &STAND_IN_1_ENDS_H,
            feed_ml: &STAND_IN_1_FEED_ML,
            water_ml: &STAND_IN_1_WATER_ML,
            sample_ml: &STAND_IN_1_SAMPLE_ML,
        }
    }

    fn stand_in_2(ph: f64) -> Question<'static> {
        Question {
            biomass_g_per_l: STAND_IN_2_BIOMASS_G_PER_L,
            glucose_g_per_l: STAND_IN_2_GLUCOSE_G_PER_L,
            galacturonate_g_per_l: STAND_IN_2_GALACTURONATE_G_PER_L,
            yeast_extract_g_per_l: STAND_IN_2_YEAST_EXTRACT_G_PER_L,
            ph,
            feed_glucose_g_per_l: STAND_IN_2_FEED_GLUCOSE_G_PER_L,
            feed_galacturonate_g_per_l: STAND_IN_2_FEED_GALACTURONATE_G_PER_L,
            segment_end_h: &STAND_IN_2_ENDS_H,
            feed_ml: &STAND_IN_2_FEED_ML,
            water_ml: &STAND_IN_2_WATER_ML,
            sample_ml: &STAND_IN_2_SAMPLE_ML,
        }
    }

    /// A short record with round numbers, for the limits: a batch to 22 h, the
    /// feed started at 0.2 mL/h to 44 h, then 0.3 mL/h to 68 h.
    fn short() -> (Vec<f64>, Vec<f64>, Vec<f64>, Vec<f64>) {
        (vec![22.0, 44.0, 68.0], vec![0.0, 4.4, 7.2], vec![-2.0, -1.0, 0.5], vec![4.3, 4.3, 4.3])
    }

    /// A record to `last` h: a batch to 21.5 h, the feed started to 45 h, then
    /// 0.25 mL/h to 120 h and 180 h; the last segment is fed 0.25 mL/h.
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
            yeast_extract_g_per_l: 2.0,
            ph: 4.0,
            feed_glucose_g_per_l: 32.0,
            feed_galacturonate_g_per_l: 94.0,
            segment_end_h: &record.0,
            feed_ml: &record.1,
            water_ml: &record.2,
            sample_ml: &record.3,
        }
    }

    /// The forecast written in the protocol is the answer served: on both
    /// stand-in records the settled culture is the reference program's within
    /// 1e-6 g/L and 1e-7 mol at every segment end, the scoring program's own
    /// limit, and the pH asked, 3.0, 3.5 or 4.0, moves no bit.
    #[test]
    fn the_protocols_forecast_is_the_answer_served() {
        for (question, python) in [
            (
                stand_in_1(3.5),
                [&STAND_IN_1_PYTHON_BIOMASS[..], &STAND_IN_1_PYTHON_GLUCOSE, &STAND_IN_1_PYTHON_GALACTURONATE, &STAND_IN_1_PYTHON_MUCIC_ACID, &STAND_IN_1_PYTHON_VOLUME_ML, &STAND_IN_1_PYTHON_CARBON_DIOXIDE, &STAND_IN_1_PYTHON_OXYGEN],
            ),
            (
                stand_in_2(3.5),
                [&STAND_IN_2_PYTHON_BIOMASS[..], &STAND_IN_2_PYTHON_GLUCOSE, &STAND_IN_2_PYTHON_GALACTURONATE, &STAND_IN_2_PYTHON_MUCIC_ACID, &STAND_IN_2_PYTHON_VOLUME_ML, &STAND_IN_2_PYTHON_CARBON_DIOXIDE, &STAND_IN_2_PYTHON_OXYGEN],
            ),
        ] {
            let culture = solve(&question).expect("settles");
            let served = [
                &culture.biomass_g_per_l, &culture.glucose_g_per_l, &culture.galacturonate_g_per_l, &culture.mucic_acid_g_per_l,
                &culture.volume_ml, &culture.carbon_dioxide_mol, &culture.oxygen_mol,
            ];
            for (column, (got, want)) in served.iter().zip(python).enumerate() {
                assert_eq!(got.len(), want.len());
                let allowed = if column < 5 { 1.0e-6 } else { 1.0e-7 };
                for (a, b) in got.iter().zip(want.iter()) {
                    assert!((a - b).abs() <= allowed, "column {column}: {a} against {b}");
                }
            }
            assert!(culture.carbon_residual <= LEDGER_TOLERANCE && culture.electron_residual <= LEDGER_TOLERANCE);
            assert!(!culture.ph_outside_compared && !culture.yeast_extract_outside_compared && !culture.feed_glucose_outside_compared);
            assert!(!culture.record_past_compared);
            // Every sample is a segment end; the cuts at the feed's start and stop are the others.
            let samples: &[usize] = if question.segment_end_h.len() == STAND_IN_1_ENDS_H.len() { &STAND_IN_1_SAMPLES } else { &STAND_IN_2_SAMPLES };
            assert_eq!(question.segment_end_h.len() - samples.len(), 2);
            assert!(samples.iter().all(|segment| question.sample_ml[*segment] > 0.0));
            for ph in [3.0, 4.0] {
                let other = solve(&Question { ph, ..question.clone() }).expect("settles");
                assert_eq!(other, culture, "pH {ph}");
            }
        }
    }

    /// The published reference of rule 7: the culture T2 pH 4 (1) of
    /// <https://zenodo.org/records/5846028>, one of the twelve the constants
    /// are fitted to. Retrieved 2026-10-09. This is agreement with data it was
    /// fitted on: the measured values within the stated allowances at every
    /// sample, biomass to 170 h. The allowances are this culture's own misfit,
    /// rounded up: mucic acid is 13.7 g/L low at 312 h, where this culture made
    /// the most of the twelve.
    #[test]
    fn the_fitted_culture_of_the_zenodo_record_is_followed() {
        let culture = solve(&stand_in_1(4.0)).expect("settles");
        for (sample, segment) in STAND_IN_1_SAMPLES.iter().enumerate() {
            let within = |got: f64, measured: f64, allowed: f64| measured.is_nan() || (got - measured).abs() <= allowed;
            assert!(within(culture.glucose_g_per_l[*segment], MEASURED_1_GLUCOSE[sample], 1.0), "glucose {sample}");
            assert!(within(culture.galacturonate_g_per_l[*segment], MEASURED_1_GALACTURONATE[sample], 5.0), "galacturonate {sample}");
            assert!(within(culture.mucic_acid_g_per_l[*segment], MEASURED_1_MUCIC_ACID[sample], 14.0), "mucic acid {sample}");
            assert!(within(culture.carbon_dioxide_mol[*segment], MEASURED_1_CARBON_DIOXIDE_MOL[sample], 0.01), "carbon dioxide {sample}");
            assert!(within(culture.oxygen_mol[*segment], MEASURED_1_OXYGEN_MOL[sample], 0.03), "oxygen {sample}");
            if STAND_IN_1_ENDS_H[*segment] <= 170.0 {
                assert!(within(culture.biomass_g_per_l[*segment], MEASURED_1_BIOMASS[sample], 2.5), "biomass {sample}");
            }
        }
        assert!(culture.settle_gap <= SETTLE_TOLERANCE);
    }

    // The four held-back cultures as the scored run asked them, and the last
    // mucic acid, galacturonate and carbon dioxide of each scored answer.
    include!("culture_trichoderma_atp_scored.rs");

    /// The four held-back questions are answered with the numbers the scored
    /// run sealed (`evidence/research/bioreactor_trichoderma_ambr250/companion_atp/out/scored`),
    /// so the misses stated in the description are the misses of the answer
    /// served; each closes both ledgers and carries no advisory flag.
    #[test]
    fn the_four_held_back_questions_are_answered_as_they_were_scored() {
        for ((culture, biomass, glucose, galacturonate, yeast_extract, ph, feed_glucose, feed_galacturonate, ends, feed, water, samples), scored) in
            SCORED.iter().zip(SCORED_LAST)
        {
            let question = Question {
                biomass_g_per_l: *biomass,
                glucose_g_per_l: *glucose,
                galacturonate_g_per_l: *galacturonate,
                yeast_extract_g_per_l: *yeast_extract,
                ph: *ph,
                feed_glucose_g_per_l: *feed_glucose,
                feed_galacturonate_g_per_l: *feed_galacturonate,
                segment_end_h: ends,
                feed_ml: feed,
                water_ml: water,
                sample_ml: samples,
            };
            let answer = solve(&question).unwrap_or_else(|reject| panic!("{culture}: {reject}"));
            assert!(answer.carbon_residual <= LEDGER_TOLERANCE && answer.electron_residual <= LEDGER_TOLERANCE, "{culture}");
            assert!(!answer.ph_outside_compared && !answer.yeast_extract_outside_compared && !answer.feed_glucose_outside_compared, "{culture}");
            let last = ends.len() - 1;
            for (got, want) in [
                (answer.mucic_acid_g_per_l[last], scored.0),
                (answer.galacturonate_g_per_l[last], scored.1),
                (answer.carbon_dioxide_mol[last], scored.2),
            ] {
                assert_eq!(got.to_bits(), want.to_bits(), "{culture}: {got} against {want}");
            }
        }
    }

    #[test]
    fn the_same_question_gives_the_same_bits() {
        let a = solve(&stand_in_2(3.0)).expect("settles");
        let b = solve(&stand_in_2(3.0)).expect("settles");
        for (x, y) in a.galacturonate_burnt_mol.iter().zip(&b.galacturonate_burnt_mol) {
            assert_eq!(x.to_bits(), y.to_bits());
        }
        assert_eq!(a, b);
    }

    /// The energy coupling. With no glucose fed once the batch's is gone,
    /// nearly all the galacturonate the culture takes is burnt and its mucic
    /// acid hardly rises; fed glucose, most is oxidised; and the burnt
    /// fraction rises as the feed's glucose falls.
    #[test]
    fn the_burnt_fraction_rises_as_the_glucose_falls() {
        let ends = [60.0, 150.0];
        let burnt_share = |feed_glucose: f64| {
            let question = Question {
                biomass_g_per_l: 0.9,
                glucose_g_per_l: 20.5,
                galacturonate_g_per_l: 4.6,
                yeast_extract_g_per_l: 2.0,
                ph: 4.0,
                feed_glucose_g_per_l: feed_glucose,
                feed_galacturonate_g_per_l: 94.0,
                segment_end_h: &ends,
                feed_ml: &[0.0, 0.25 * 90.0],
                water_ml: &[0.0, 0.0],
                sample_ml: &[0.0, 0.0],
            };
            let culture = solve(&question).expect("settles");
            let mucic_mol = |i: usize| culture.mucic_acid_g_per_l[i] * culture.volume_ml[i] / 1000.0 / MUCIC_ACID_G_PER_MOL;
            let oxidised = mucic_mol(1) - mucic_mol(0);
            let burnt = culture.galacturonate_burnt_mol[1] - culture.galacturonate_burnt_mol[0];
            burnt / (burnt + oxidised)
        };
        let shares: Vec<f64> = [50.0, 32.0, 10.0, 0.0].iter().map(|g| burnt_share(*g)).collect();
        assert!(shares.windows(2).all(|pair| pair[1] > pair[0]), "{shares:?}");
        assert!(shares[0] < 0.2, "{shares:?}");
        assert!(shares[3] > 0.9, "{shares:?}");
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
        assert!((culture.volume_ml[2] - (110.0 + 11.6 - 2.5 - 8.6)).abs() < 1.0e-9);
        assert!(culture.carbon_dioxide_mol[2] > culture.carbon_dioxide_mol[1]);
        assert!(culture.galacturonate_burnt_mol[2] >= culture.galacturonate_burnt_mol[1]);
    }

    /// The attack of rule 8. A table of reactions with one entry changed (a
    /// burning that gives five carbon dioxide for six carbons, or a growth
    /// that takes no oxygen) is named by the ledger it breaks.
    #[test]
    fn a_falsified_reaction_is_named_by_its_ledger() {
        let record = to(300.0);
        let mut question = asked(&record);
        question.feed_glucose_g_per_l = 5.0;
        let honest = Reactions::of_the_specimen();
        assert!(settle(&honest, &question).is_ok());
        let lost_carbon = Reactions { burning_carbon_dioxide: 5.0, ..honest };
        match settle(&lost_carbon, &question) {
            Err(CultureReject::CarbonLedgerBroken { residual }) => assert!(residual > 1.0e-6),
            other => panic!("{other:?}"),
        }
        let free_growth = Reactions { growth_oxygen: 0.0, ..honest };
        match settle(&free_growth, &question) {
            Err(CultureReject::ElectronLedgerBroken { residual }) => assert!(residual > 1.0e-3),
            other => panic!("{other:?}"),
        }
        let free_burning = Reactions { burning_oxygen: 4.0, ..honest };
        assert!(matches!(settle(&free_burning, &question), Err(CultureReject::ElectronLedgerBroken { .. })));
    }

    /// Every limit the question shows is named, in the solver's order.
    #[test]
    fn each_limit_of_the_question_is_named() {
        let record = short();
        let base = asked(&record);
        let name = |question: &Question<'_>| solve(question).expect_err("refused").name();
        assert_eq!(name(&Question { ph: f64::NAN, ..base.clone() }), "NonFiniteValue");
        let below_zero = -f64::from_bits(1);
        for (question, refused) in [
            (Question { biomass_g_per_l: below_zero, ..base.clone() }, "BiomassNegative"),
            (Question { glucose_g_per_l: below_zero, ..base.clone() }, "GlucoseNegative"),
            (Question { galacturonate_g_per_l: below_zero, ..base.clone() }, "GalacturonateNegative"),
            (Question { yeast_extract_g_per_l: below_zero, ..base.clone() }, "YeastExtractNegative"),
            (Question { ph: below_zero, ..base.clone() }, "PhOutsideAqueousScale"),
            (Question { ph: 14.0 + 1.0e-9, ..base.clone() }, "PhOutsideAqueousScale"),
            (Question { feed_glucose_g_per_l: below_zero, ..base.clone() }, "FeedGlucoseNegative"),
            (Question { feed_galacturonate_g_per_l: below_zero, ..base.clone() }, "FeedGalacturonateNegative"),
        ] {
            assert_eq!(name(&question), refused);
        }
        for question in [
            Question { yeast_extract_g_per_l: 0.0, ..base.clone() },
            Question { yeast_extract_g_per_l: 20.0, ..base.clone() },
            Question { ph: 0.0, ..base.clone() },
            Question { ph: 14.0, ..base.clone() },
            Question { ph: 7.0, ..base.clone() },
        ] {
            assert_eq!(check_question(&question), Ok(()));
        }
        assert_eq!(name(&Question { segment_end_h: &[], ..base.clone() }), "SegmentCountOutOfRange");
        assert_eq!(name(&Question { feed_ml: &[0.0, 6.0], ..base.clone() }), "SegmentsDoNotMatch");
        assert_eq!(name(&Question { segment_end_h: &[22.0, 22.0, 68.0], ..base.clone() }), "SegmentNotRising");
        assert_eq!(name(&Question { segment_end_h: &[0.0, 44.0, 68.0], ..base.clone() }), "SegmentNotRising");
        assert_eq!(name(&Question { feed_ml: &[0.0, -0.1, 7.2], ..base.clone() }), "FeedRateNegative");
        assert_eq!(name(&Question { sample_ml: &[4.3, -1.0, 4.3], ..base.clone() }), "SampleNegative");
        assert_eq!(name(&Question { sample_ml: &[4.3, 120.0, 4.3], ..base.clone() }), "VolumeOutOfRange");
        assert_eq!(name(&Question { water_ml: &[150.0, -1.0, 0.5], ..base.clone() }), "VolumeOutOfRange");
        let long = to(RUN_END_MAX_H + 0.01);
        assert_eq!(name(&asked(&long)), "RunTooLong");
        let many = vec![0.0; 33];
        let ends: Vec<f64> = (1..=33).map(|segment| 2.0 * f64::from(segment)).collect();
        assert_eq!(
            name(&Question { segment_end_h: &ends, feed_ml: &many, water_ml: &many, sample_ml: &many, ..base }),
            "SegmentCountOutOfRange"
        );
    }

    /// The exact decimal ends of every range are inside it: the pH at 0 and
    /// at 14, a record that ends exactly at its capacity, and a volume that
    /// ends exactly at 250 mL.
    #[test]
    fn the_exact_ends_of_every_range_are_inside() {
        let record = short();
        let base = asked(&record);
        assert_eq!(check_question(&Question { ph: PH_MIN, ..base.clone() }), Ok(()));
        assert_eq!(check_question(&Question { ph: PH_MAX, ..base.clone() }), Ok(()));
        assert!(check_question(&Question { ph: PH_MAX.next_up(), ..base.clone() }).is_err());
        assert!(check_question(&Question { ph: (-0.0_f64).next_down(), ..base.clone() }).is_err());
        for last in [215.01, COMPARED_RECORD_END_H, RUN_END_MAX_H] {
            let record = to(last);
            assert_eq!(check_question(&asked(&record)), Ok(()), "{last}");
        }
        let past = to(RUN_END_MAX_H.next_up());
        assert!(matches!(check_question(&asked(&past)), Err(CultureReject::RunTooLong { .. })));
        let high = Question { segment_end_h: &[22.0, 44.0], feed_ml: &[0.0, 4.4], water_ml: &[100.0, 35.6], sample_ml: &[0.0, 0.0], ..base.clone() };
        assert_eq!(check_question(&high), Ok(()));
        let over = Question { water_ml: &[100.0, 35.600_000_1], ..high.clone() };
        assert!(matches!(check_question(&over), Err(CultureReject::VolumeOutOfRange { .. })));
    }

    /// Outside what was compared the same arithmetic answers, and the answer
    /// says so: a pH of 2 or 6, no yeast extract or 5 g/L, a feed of 5 or 60
    /// g/L glucose, a record to 400 h. Each closes both ledgers.
    #[test]
    fn outside_what_was_compared_the_answer_carries_its_flags() {
        let record = short();
        let base = asked(&record);
        let flags = |question: &Question<'_>| {
            let c = solve(question).expect("settles");
            assert!(c.carbon_residual <= LEDGER_TOLERANCE && c.electron_residual <= LEDGER_TOLERANCE);
            (c.ph_outside_compared, c.yeast_extract_outside_compared, c.feed_glucose_outside_compared, c.record_past_compared)
        };
        assert_eq!(flags(&base), (false, false, false, false));
        assert_eq!(flags(&Question { ph: 2.0, ..base.clone() }), (true, false, false, false));
        assert_eq!(flags(&Question { ph: 6.0, ..base.clone() }), (true, false, false, false));
        assert_eq!(flags(&Question { ph: 3.0, ..base.clone() }), (false, false, false, false));
        assert_eq!(flags(&Question { yeast_extract_g_per_l: 5.0, ..base.clone() }), (false, true, false, false));
        assert_eq!(flags(&Question { yeast_extract_g_per_l: 0.0, ..base.clone() }), (false, false, false, false));
        assert_eq!(flags(&Question { feed_glucose_g_per_l: 5.0, ..base.clone() }), (false, false, true, false));
        assert_eq!(flags(&Question { feed_glucose_g_per_l: 60.0, ..base.clone() }), (false, false, true, false));
        let long = to(400.0);
        assert_eq!(flags(&asked(&long)), (false, false, false, true));
        // The pH moves no number, inside the compared range or out of it.
        let at = |ph: f64| solve(&Question { ph, ..base.clone() }).expect("settles").mucic_acid_g_per_l;
        assert_eq!(at(2.0), at(4.0));
        assert_eq!(at(9.0), at(4.0));
    }

    /// Past every edge of the compared cultures the same equations settle and
    /// both ledgers close: no biomass, ten times the biomass, no glucose, no
    /// galacturonate, no sugar in the feed, 200 g/L glucose and 300 g/L
    /// galacturonate in it, a feed of 1 mL/h from inoculation, a record of
    /// 1000 h. A vessel with no carbon at all supplies none and finds none.
    #[test]
    fn past_the_compared_cultures_the_ledgers_close() {
        let record = short();
        let base = asked(&record);
        let fast = (vec![22.0, 44.0, 68.0], vec![22.0, 22.0, 24.0], vec![-2.0, -1.0, 0.5], vec![4.3, 4.3, 4.3]);
        let thousand = to(1000.0);
        let questions = [
            Question { biomass_g_per_l: 0.0, ..base.clone() },
            Question { biomass_g_per_l: 9.0, ..base.clone() },
            Question { glucose_g_per_l: 0.0, ..base.clone() },
            Question { galacturonate_g_per_l: 0.0, ..base.clone() },
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
        }
        let empty = solve(&questions[0]).expect("settles");
        assert!(empty.biomass_g_per_l.iter().all(|x| *x == 0.0));
        assert!(empty.galacturonate_burnt_mol.iter().all(|x| *x == 0.0));
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

    /// Every sentence a refusal writes counts segments from 1 and writes no
    /// range in program notation.
    #[test]
    fn a_refusal_counts_segments_from_one() {
        let record = short();
        let base = asked(&record);
        let refused = check_question(&Question { feed_ml: &[0.0, -0.5, 7.2], ..base.clone() }).expect_err("a negative feed").to_string();
        assert!(refused.starts_with("segment 2 is fed -0.5 mL, below zero"), "{refused}");
        let ph = check_question(&Question { ph: 15.0, ..base }).expect_err("off the scale").to_string();
        assert!(ph.starts_with("pH 15 is below 0 or above 14"), "{ph}");
        assert!(!refused.contains("..=") && !ph.contains("..="));
    }

    /// Whether every number of a culture is normal or zero: the answer is
    /// the one the rule of `crate::normal` leaves as it is.
    fn normal_or_zero_throughout(culture: &Culture) -> bool {
        let mut held = culture.clone();
        held.normal_or_zero();
        held == *culture
    }

    /// The red team's cultures of engine 0.13.0 (D1). Fed like the fitted
    /// cultures, 60 mL of 52.1 g/L glucose from 24 h to 261 h with a daily
    /// sample of 4.3 mL, the glucose at 312 h comes to 5.96e-309 g/L as
    /// computed; unfed to 312 h it comes to 1.2e-322. Each is answered as
    /// +0, every number of the answer is normal or zero, every other number
    /// is the computed one bit for bit, and both ledgers close.
    #[test]
    fn a_glucose_under_the_smallest_normal_number_is_answered_as_zero() {
        let fed_ends = [24.0, 48.0, 72.0, 96.0, 120.0, 144.0, 168.0, 192.0, 216.0, 240.0, 261.0, 288.0, 312.0];
        let fed_ml = [0.0, 6.075949, 6.075949, 6.075949, 6.075949, 6.075949, 6.075949, 6.075949, 6.075949, 6.075949, 5.316456, 0.0, 0.0];
        let fed = Question {
            biomass_g_per_l: 0.8,
            glucose_g_per_l: 20.5,
            galacturonate_g_per_l: 4.5,
            yeast_extract_g_per_l: 1.0,
            ph: 4.0,
            feed_glucose_g_per_l: 52.1,
            feed_galacturonate_g_per_l: 95.0,
            segment_end_h: &fed_ends,
            feed_ml: &fed_ml,
            water_ml: &[0.0; 13],
            sample_ml: &[4.3; 13],
        };
        let unfed = Question {
            biomass_g_per_l: 0.9,
            glucose_g_per_l: 20.8,
            galacturonate_g_per_l: 4.6,
            yeast_extract_g_per_l: 2.0,
            ph: 3.5,
            feed_glucose_g_per_l: 32.0,
            feed_galacturonate_g_per_l: 94.0,
            segment_end_h: &[312.0],
            feed_ml: &[0.0],
            water_ml: &[0.0],
            sample_ml: &[0.0],
        };
        for (question, segment) in [(fed, 12), (unfed, 0)] {
            let computed = solve_as_computed(&question).expect("settles");
            let answered = solve(&question).expect("settles");
            assert!(computed.glucose_g_per_l[segment].is_subnormal(), "{:e}", computed.glucose_g_per_l[segment]);
            assert_eq!(answered.glucose_g_per_l[segment].to_bits(), 0);
            assert!(normal_or_zero_throughout(&answered));
            let mut held = computed.clone();
            held.normal_or_zero();
            assert_eq!(held, answered);
            assert!(answered.carbon_residual <= LEDGER_TOLERANCE && answered.electron_residual <= LEDGER_TOLERANCE);
        }
    }
}
