//! The entry for `ablator_slab`: a flat slab of charring heat-shield material
//! heated on one face. The bounds come from `genesis_solvers` constants, the
//! sentence of each refusal from the solver's source, and the validation
//! record from the solver's header and the record of the Stardust model run.
//!
//! The solver is the judge of a request. Every limit a request itself shows
//! is asked of the solver's own `check_question` before a physics permit is
//! taken, so the name and the sentence are the solver's.
//!
//! The heating history is carried as five columns, each an array under a
//! name that states its quantity and unit, with one number in each for each
//! row. Every number of a request is then under a name, each column has the
//! cap the solver states for the rows, and a history of 256 rows is a few
//! thousand characters.
//!
//! The whole description is long, so the description a caller reads first
//! holds the record, the inputs and two short worked examples, and keeps the
//! rest for asking, part by part: how to read an answer, the refusals, the
//! locked constants, and the Stardust question with its 136 rows.

use genesis_solvers::ablator_slab::{
    self as slab, SlabQuestion, SlabReject, StreamGas, StreamRow, SurfaceCondition, Wanted,
};
use serde_json::{Value, json};

use crate::solver_ablator_slab_stardust as stardust;
use crate::solvers::{
    Answer, FurtherExample, Given, Input, Kind, Output, Part, Published, Refusal, Refused,
    Sections, Shape, Solved, Solver,
};

/// Every material name the solver takes, in the solver's order. The PICA
/// model a caller is pointed to is `slab::MATERIAL_PICA_SERVED`.
const MATERIALS: &[&str] = &slab::MATERIAL_NAMES;

/// The two things that can be done to the heated face.
const FACE_HELD: &str = "held_temperature";
const FACE_STREAM: &str = "gas_stream";
const FACES: &[&str] = &[FACE_HELD, FACE_STREAM];

/// What the record says, among its data, of the 31 arc-jet tests that were
/// held back: the final check of 2026-10-09.
pub const HELD_BACK: &str = "On 2026-10-09 the model was run once on 31 further arc-jet tests that no model had seen, with its predictions fixed before the measurements were read.";

/// The outcome of that check, as its record gives it.
pub const HELD_BACK_RESULT: &str = "The 31 held-back arc-jet tests, against the lines fixed beforehand. Wear: inside the line on 28 of 31 where 25 were needed, with a mean error of −2.6 percent against 8 allowed: passes. Hottest face temperature: within 7 percent on 24 of 28 where 23 were needed, but 4.1 percent hot on average against 3.5 allowed: misses on the average, as it did on the 36 development tests (3.8). So by the rule fixed beforehand the model as a whole does not pass. One test, in pure nitrogen (416 W/cm², 17.9 kPa, 120 s), was refused at the time because its stream had less oxygen than the solver then took, and counts as outside. It is answered now: 0.22 mm of wear against 1.75 mm measured, 87 percent low, outside its line of ±0.57 mm; with no oxygen the model burns no char. What the misses share: the face runs hot above about 45 kPa, most at 190 and 970 W/cm²; two long exposures in one facility miss on wear.";

/// What the limits say of the same tests.
pub const HELD_BACK_SPENT: &str = "The 31 held-back tests are now spent, and 26 of them repeat a condition and exposure from the development set.";

const fn number(
    name: &'static str,
    minimum: f64,
    maximum: f64,
    unit: &'static str,
    refusal: &'static str,
    only_when: Option<(&'static str, &'static str)>,
    bound: &'static str,
) -> Input {
    Input {
        name,
        kind: Kind::Number {
            minimum,
            maximum,
            unit,
        },
        refusal: Some(refusal),
        bound,
        only_when,
        asked_by_check: true,
    }
}

/// One column of the heating history.
const fn column(
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
            fewest: slab::HISTORY_ROWS_MIN,
            most: slab::HISTORY_ROWS_MAX,
            minimum,
            maximum,
            unit,
        },
        refusal: Some(refusal),
        bound,
        only_when: Some(("face", FACE_STREAM)),
        asked_by_check: true,
    }
}

const fn flag(name: &'static str, only_when: Option<(&'static str, &'static str)>, bound: &'static str) -> Input {
    Input {
        name,
        kind: Kind::Flag,
        refusal: None,
        bound,
        only_when,
        asked_by_check: false,
    }
}

const fn one(name: &'static str, unit: &'static str, note: &'static str) -> Output {
    Output {
        name,
        unit,
        shape: Shape::Number,
        each: &[],
        note,
        asked_by: None,
    }
}

const fn asked(
    name: &'static str,
    unit: &'static str,
    shape: Shape,
    asked_by: &'static str,
    note: &'static str,
) -> Output {
    Output {
        name,
        unit,
        shape,
        each: &[],
        note,
        asked_by: Some(asked_by),
    }
}

const HELD: Option<(&str, &str)> = Some(("face", FACE_HELD));
const STREAM: Option<(&str, &str)> = Some(("face", FACE_STREAM));

/// The two ways of saying what a gas stream is made of.
const GAS_AIR: &str = "air";
const GAS_MIXTURE: &str = "mixture";
const STREAM_GASES: &[&str] = &[GAS_AIR, GAS_MIXTURE];
const MIXTURE: Option<(&str, &str)> = Some(("stream_gas", GAS_MIXTURE));

const ASK_FACE: &str = "return_face_history";
const ASK_THERMOCOUPLES: &str = "return_thermocouple_histories";
const ASK_PROFILE: &str = "return_final_profile";

pub const ABLATOR_SLAB: Solver = Solver {
    name: "ablator_slab",
    summary: "A flat slab of charring heat-shield material heated on one face, held at a temperature or in a gas stream: wear, face temperature, depth of charring, and the heat and mass accounts, for a PICA model built from the open record, compared with measurement in 36 arc-jet tests, on which its surface rules were chosen, 31 more held back, the Stardust flight and four thermocouple blocks, or the test material TACOT, compared with other codes only.",
    inputs: &[
        Input {
            name: "material",
            kind: Kind::Word { allowed: MATERIALS },
            refusal: Some("UnknownMaterial"),
            bound: "The PICA model a caller is pointed to is `PICA, open record, version 5`. Older versions are kept so recorded runs can be remade. `TACOT 3.0` is not PICA.",
            only_when: None,
            asked_by_check: false,
        },
        number(
            "thickness_m",
            slab::THICKNESS_MIN_M,
            slab::THICKNESS_MAX_M,
            "m",
            "ThicknessOutOfRange",
            None,
            "Slab thickness from the heated face to the closed back, 0.2 mm to 1 m: below 0.2 mm the grid holds no more than 8 cells, and 1 m is a size cap on the request. PICA was compared with measurement at 0.06 m (arc-jet tests), 0.0582 m (Stardust flight) and 0.0224 to 0.0325 m (thermocouple blocks); any other thickness is answered in no compared envelope.",
        ),
        number(
            "start_temperature_k",
            slab::START_TEMPERATURE_MIN_K,
            slab::START_TEMPERATURE_MAX_K,
            "K",
            "StartTemperatureOutOfRange",
            None,
            "Temperature of the whole slab at the start, 260 to 800 K. Above 800 K untouched material decomposes so fast from the first step that the step fails. PICA was compared from 300 K (arc-jet tests), 260 K (Stardust flight) and 303.15 K (thermocouple blocks).",
        ),
        Input {
            name: "thermocouple_depths_m",
            kind: Kind::Numbers {
                fewest: 0,
                most: slab::THERMOCOUPLES_MAX,
                minimum: 0.0,
                maximum: slab::THICKNESS_MAX_M,
                unit: "m",
            },
            refusal: Some("ThermocoupleOutsideSlab"),
            bound: "Depths below the starting face where a temperature history is kept, each within the slab. May be empty.",
            only_when: None,
            asked_by_check: true,
        },
        Input {
            name: "face",
            kind: Kind::Word { allowed: FACES },
            refusal: Some("UnknownFace"),
            bound: "`held_temperature`: the face is held at a stated temperature and does not wear. `gas_stream`: the face sits in hot gas given as a heating history, finds its own temperature, radiates, and wears if `face_wears`.",
            only_when: None,
            asked_by_check: false,
        },
        number(
            "held_temperature_k",
            slab::HELD_TEMPERATURE_MIN_K,
            slab::HELD_TEMPERATURE_MAX_K,
            "K",
            "HeldTemperatureOutOfRange",
            HELD,
            "Temperature the face is held at, 260 to 3300 K, reached in a straight line over 0.1 s; below the start temperature it cools the slab. TACOT 3.0 was compared with other codes held at 1644 K for 60 s. No PICA case was held: a held PICA face is answered in no compared envelope.",
        ),
        number(
            "hold_duration_s",
            slab::HOLD_DURATION_MIN_S,
            slab::HOLD_DURATION_MAX_S,
            "s",
            "HoldDurationOutOfRange",
            HELD,
            "How long the face is held, from the start, 1 s to an hour: longer than the 0.1 s ramps up and down, and an hour is a size cap on the request.",
        ),
        number(
            "continue_after_s",
            0.0,
            slab::CONTINUE_AFTER_MAX_S,
            "s",
            "ContinueAfterOutOfRange",
            HELD,
            "Time computed after the hold, the face back at the start temperature, 0 s to an hour, a size cap on the request.",
        ),
        Input {
            name: "stream_gas",
            kind: Kind::Word { allowed: STREAM_GASES },
            refusal: Some("UnknownStreamGas"),
            bound: "What the stream is made of: `air`, with no fraction given, or `mixture`, oxygen and argon by mass as given and the rest nitrogen. `TACOT 3.0` takes `air` alone.",
            only_when: STREAM,
            asked_by_check: false,
        },
        number(
            "stream_oxygen_mass_fraction",
            0.0,
            1.0,
            "by mass",
            "StreamOxygenNotAMassFraction",
            MIXTURE,
            "Oxygen in a mixture by mass, 0 to 1. Oxygen and argon together are at most 1 (`StreamOxygenAndArgonAboveOne`); the rest is nitrogen. Pure nitrogen is 0 and 0. PICA was compared in air with 7.6 to 27.6 percent argon and in nitrogen with 10 or 30 percent oxygen (67 arc-jet tests).",
        ),
        number(
            "stream_argon_mass_fraction",
            0.0,
            1.0,
            "by mass",
            "StreamArgonNotAMassFraction",
            MIXTURE,
            "Argon in a mixture by mass, 0 to 1.",
        ),
        column(
            "history_time_s",
            0.0,
            slab::HISTORY_DURATION_MAX_S,
            "s",
            "HistoryDurationOutOfRange",
            "One of five columns of the heating history, a number in each for each row, straight lines between rows. Time of each row: zero first, each 0.001 s or more after the one before, or it is refused as HistoryRowsTooClose (rows written 1 and 1.001 s are taken), the last from 1 s to an hour, a size cap on the request.",
        ),
        column(
            "history_pressure_pa",
            slab::PICA_STREAM_PRESSURE_MIN_PA,
            slab::STREAM_PRESSURE_MAX_PA,
            "Pa",
            "StreamPressureOutOfRange",
            "Gas pressure at the wall, the ends of the material's surface table: 1.01325 to 101325 Pa for a PICA model, 101.325 to 101325 Pa for TACOT 3.0. PICA was compared with measurement at 2.3 to 84.4 kPa (arc-jet tests), 1.42 Pa to 34.0 kPa (Stardust flight) and one pressure from 20.265 to 45.596 kPa (thermocouple blocks).",
        ),
        column(
            "history_heat_transfer_coefficient_kg_per_m2_s",
            0.0,
            slab::HEAT_TRANSFER_COEFFICIENT_MAX_KG_PER_M2_S,
            "kg/m²/s",
            "HeatTransferCoefficientOutOfRange",
            "Heat-transfer coefficient of the stream with no gas blowing from the face, zero or more and finite. Times the flow enthalpy it is the heating of a cold wall. Zero: no stream. PICA was compared to 0.1971 kg/m²/s (Stardust flight). A stream too strong stops the run where the face boils (`FaceBeyondSurfaceTable`), the slab wears away (the run ends there, `slab_worn_away` 1) or a temperature passes the top of the material's tables (`TemperatureOutOfBounds`).",
        ),
        column(
            "history_flow_enthalpy_j_per_kg",
            0.0,
            slab::FLOW_ENTHALPY_MAX_J_PER_KG,
            "J/kg",
            "FlowEnthalpyOutOfRange",
            "Enthalpy of the flow (recovery enthalpy), zero or more and finite. With a coefficient above zero, at least the stream gas's enthalpy at a wall at the lower of the start and surroundings temperatures, 1,871 J/kg for air at 300 K (`FlowEnthalpyBelowWall`). PICA was compared at 7.3 to 29.2 MJ/kg (arc-jet tests) and 85 kJ/kg to 78.3 MJ/kg (Stardust flight).",
        ),
        column(
            "history_radiative_heating_w_per_m2",
            0.0,
            slab::RADIATIVE_HEATING_MAX_W_PER_M2,
            "W/m²",
            "RadiativeHeatingOutOfRange",
            "Radiative heating arriving at the face, zero or more and finite. PICA was compared to 0.74 MW/m² (Stardust flight).",
        ),
        number(
            "surroundings_temperature_k",
            slab::SURROUNDINGS_TEMPERATURE_MIN_K,
            genesis_solvers::pica::TABLE_TOP_K,
            "K",
            "SurroundingsTemperatureOutOfRange",
            STREAM,
            "Temperature the face radiates to, from 250 K to the top of the material's tables: 3333.33 K for TACOT 3.0, 3975 K for a PICA model. PICA was compared radiating to 300 K (arc-jet tests and thermocouple blocks) and 266.48 K (Stardust flight).",
        ),
        flag(
            "face_wears",
            STREAM,
            "`true`: the face loses char and moves inward. `false`: no char leaves. TACOT 3.0 was compared with other codes with the face kept (case 2.1). Every PICA case wore its face: a PICA face kept is answered in no compared envelope.",
        ),
        flag(
            ASK_FACE,
            None,
            "`true` adds seven arrays of the face against time, at most 121 points each. Required: `false` gives the summary alone.",
        ),
        flag(
            ASK_THERMOCOUPLES,
            None,
            "`true` adds the temperature history at each thermocouple depth, at most 121 points each.",
        ),
        flag(
            ASK_PROFILE,
            None,
            "`true` adds temperature and density through the thickness at the end, at most 201 points.",
        ),
    ],
    kept_inside: "The material's numbers, the cells, the time steps and the tolerances stay inside the solver.",
    result: &[
        one(
            "material_compared_with_measurement",
            "1 or 0",
            "1 for a PICA model, which has been compared with measurements. 0 for TACOT 3.0, a made-up material that has been compared with other codes only.",
        ),
        one("duration_s", "s", "How long the run was: the end of its time, or the moment the slab wore away."),
        one(
            "slab_worn_away",
            "1 or 0",
            "1 when the face wore to within 8 cells of the back wall: the slab is worn away, the event is over, and the run ended there, at `duration_s`, with every account closed over the run it had. 0 when the run reached the end of its time.",
        ),
        one("wear_m", "m", "How far the face wore inward. Zero for a held face."),
        one(
            "peak_face_temperature_k",
            "K",
            "Hottest the face was at a report time. Report times are every whole second, the end of the heating and the end of the run.",
        ),
        one("peak_face_temperature_at_s", "s", "When that was."),
        one("final_face_temperature_k", "K", "Temperature of the face at the end."),
        one("final_back_temperature_k", "K", "Temperature at the back of the slab at the end."),
        one(
            "depth_half_way_to_virgin_m",
            "m",
            "Depth below the face as it ended at which the density is half way from char to untouched material.",
        ),
        one(
            "depth_within_2_percent_of_virgin_m",
            "m",
            "Depth below the face as it ended at which the density is within 2 percent of the untouched material's. For the PICA model this depth comes out 3 to 4 mm too deep: see `validation.limits`.",
        ),
        one("heat_from_stream_j_per_m2", "J/m²", "The heat account. Heat from the gas stream, after the correction for gas blowing out of the face."),
        one("heat_absorbed_from_radiation_j_per_m2", "J/m²", "Radiative heating the face took up."),
        one("heat_radiated_away_j_per_m2", "J/m²", "Heat the face radiated to its surroundings."),
        one(
            "heat_taken_at_face_by_gases_j_per_m2",
            "J/m²",
            "Heat taken at the face by the decomposition gas and the char as they become gas at the wall.",
        ),
        one(
            "heat_in_through_face_j_per_m2",
            "J/m²",
            "Heat conducted into the slab through the face. Negative when more left than entered.",
        ),
        one("heat_stored_in_solid_j_per_m2", "J/m²", "Heat stored in the solid as temperature rise."),
        one("heat_taken_by_gas_j_per_m2", "J/m²", "Heat the decomposition gas took from the solid on its way to the face."),
        one(
            "heat_given_off_by_decomposition_j_per_m2",
            "J/m²",
            "Heat given off as the resin decomposed. Negative when taken up.",
        ),
        one("char_enthalpy_removed_j_per_m2", "J/m²", "Enthalpy of the char worn off the face, as it left."),
        one("solid_mass_lost_kg_per_m2", "kg/m²", "The mass account. Mass the solid lost: gas out plus char worn off."),
        one("gas_mass_out_kg_per_m2", "kg/m²", "Decomposition gas that left through the face."),
        one("char_mass_removed_kg_per_m2", "kg/m²", "Char worn off the face."),
        one(
            "inside_arc_jet_comparison",
            "1 or 0",
            "1 when the question lies wholly inside the envelope of the arc-jet tests the PICA model was compared with measurement on. The envelopes are in `locked.compared_with_measurement`. A PICA question inside no envelope is answered with the same equations and accounts, and has 0 here, in `inside_stardust_comparison` and in `inside_thermocouple_block_comparison`: a held face, a face kept, or any value no compared case had. 0 for TACOT 3.0.",
        ),
        one(
            "inside_stardust_comparison",
            "1 or 0",
            "1 when the question lies wholly inside the envelope of the two Stardust flight comparisons. 0 for TACOT 3.0.",
        ),
        one(
            "inside_thermocouple_block_comparison",
            "1 or 0",
            "1 when the question lies wholly inside the envelope of the four thermocouple blocks (Tran and others 1997), the data the one fitted number was fitted to. 0 for TACOT 3.0.",
        ),
        one(
            "stream_enthalpy_at_cold_wall_j_per_kg",
            "J/kg",
            "Enthalpy of the stream's gas at a cold wall, at 300 K. It depends on the gas alone, and it is what turns an arc-jet test's cold-wall heating into a heat-transfer coefficient: see `arc_jet_test_to_question` in the part `reference`. Zero for a held face, which has no stream.",
        ),
        one("cells", "cells", "Cells through the thickness at the start. A whole number."),
        one("cells_at_end", "cells", "Cells left at the end. A wearing face uses cells up. A whole number."),
        one("time_steps", "steps", "Time steps taken. A whole number, and the same count as `iterations`."),
        one(
            "hardest_step_newton_passes",
            "passes",
            "Passes the step that needed the most took to settle. A whole number.",
        ),
        asked(
            "face_history_time_s",
            "s",
            Shape::Numbers,
            ASK_FACE,
            "The times of the face history: every whole second, the end of the heating and the end of the run, thinned evenly to at most 121 points that keep the first and the last. The six arrays that follow have one value for each of these times.",
        ),
        asked("face_history_temperature_k", "K", Shape::Numbers, ASK_FACE, "Temperature of the face."),
        asked(
            "face_history_recession_m",
            "m",
            Shape::Numbers,
            ASK_FACE,
            "How far the face has worn inward.",
        ),
        asked(
            "face_history_char_front_depth_m",
            "m",
            Shape::Numbers,
            ASK_FACE,
            "Depth below the face as it started at which the density is 2 percent of the way from char to untouched material. Zero while the face itself is denser than that.",
        ),
        asked(
            "face_history_virgin_front_depth_m",
            "m",
            Shape::Numbers,
            ASK_FACE,
            "Depth below the face as it started at which the density is 98 percent of the way from char to untouched material.",
        ),
        asked(
            "face_history_gas_mass_flux_kg_per_m2_s",
            "kg/m²/s",
            Shape::Numbers,
            ASK_FACE,
            "Decomposition gas leaving through the face, averaged over the time step that ends at each time.",
        ),
        asked(
            "face_history_char_mass_flux_kg_per_m2_s",
            "kg/m²/s",
            Shape::Numbers,
            ASK_FACE,
            "Char leaving the face, averaged over the time step that ends at each time.",
        ),
        asked(
            "thermocouple_depth_m",
            "m",
            Shape::Numbers,
            ASK_THERMOCOUPLES,
            "The thermocouple depths as asked. The four arrays that follow have one entry for each, in this order.",
        ),
        asked(
            "thermocouple_passed_by_face",
            "1 or 0",
            Shape::Numbers,
            ASK_THERMOCOUPLES,
            "1 when the face wore past that depth, 0 when it did not.",
        ),
        asked(
            "thermocouple_covered_until_s",
            "s",
            Shape::Numbers,
            ASK_THERMOCOUPLES,
            "When the face passed that depth, or the end of the run when it did not.",
        ),
        asked(
            "thermocouple_time_s",
            "s",
            Shape::Lists,
            ASK_THERMOCOUPLES,
            "For each thermocouple, the times of its record, at most 121. A record ends where the face passed it, so the arrays can differ in length.",
        ),
        asked(
            "thermocouple_temperature_k",
            "K",
            Shape::Lists,
            ASK_THERMOCOUPLES,
            "For each thermocouple, its temperature at those times.",
        ),
        asked(
            "final_profile_depth_m",
            "m",
            Shape::Numbers,
            ASK_PROFILE,
            "Depths below the face as it started, from the face as it ended to the back: the centres of the cells left, thinned evenly to at most 201 points that keep the first and the last.",
        ),
        asked(
            "final_profile_temperature_k",
            "K",
            Shape::Numbers,
            ASK_PROFILE,
            "Temperature at each of those depths at the end. `field_sha256` covers these values.",
        ),
        asked(
            "final_profile_density_kg_per_m3",
            "kg/m³",
            Shape::Numbers,
            ASK_PROFILE,
            "Density of the solid at each of those depths at the end.",
        ),
    ],
    invariants: &[
        one(
            "energy_account_residual_j_per_m2",
            "J/m²",
            "How far the energy account of the whole run is open: heat stored, less heat in through the face, less heat given off by decomposition, plus heat the gas took.",
        ),
        one(
            "energy_account_tolerance_j_per_m2",
            "J/m²",
            "What that residual was allowed.",
        ),
        one(
            "face_balance_residual_j_per_m2",
            "J/m²",
            "How far the heat balance at the face is open: heat conducted in, less heat from the stream, less radiation taken up, plus heat radiated away, plus heat taken by the gases at the face. Zero for a held face.",
        ),
        one(
            "face_balance_tolerance_j_per_m2",
            "J/m²",
            "What that residual was allowed.",
        ),
        one(
            "mass_account_residual_kg_per_m2",
            "kg/m²",
            "How far the mass account is open: solid lost, less gas out, less char worn off.",
        ),
        one(
            "mass_account_tolerance_kg_per_m2",
            "kg/m²",
            "What that residual was allowed.",
        ),
    ],
    iterations: "Time steps taken: the same count as `result.time_steps`. On a refusal it is null.",
    field: field_words,
    answer: "a solved slab",
    refusals: REFUSALS,
    door_refusals: DOOR_REFUSALS,
    sections,
    validation: Some(validation),
    example,
    further_examples: &[
        FurtherExample {
            key: "example_solve_arc_jet_test",
            parameters: arc_jet_example,
        },
        FurtherExample {
            key: "example_solve_stardust_near_nose",
            parameters: stardust_example,
        },
    ],
    parts: PARTS,
    check,
    solve,
};

/// What the description a caller reads first keeps for asking, and the
/// sentence it gives in place of each.
const PARTS: &[Part] = &[
    Part {
        key: "locked",
        holds: "What a caller cannot set: pressure ranges by material, the envelopes the PICA model was compared with measurement on, cells, time steps, constants named in refusals.",
    },
    Part {
        key: "returns",
        holds: "Every name of `result` and `invariants` with its unit and a note, and the rule for the `_hex` values the proof covers.",
    },
    Part {
        key: "invariants",
        holds: "The three accounts every step and every answer must close, and the refusal that names a break.",
    },
    Part {
        key: "convergence",
        holds: "When the solver accepts a time step.",
    },
    Part {
        key: "refusals",
        holds: "The 34 named refusals with the solver's sentence for each: 23 a request itself shows (422, no proof) and 11 met while computing (200, outcome `refused`, no result).",
    },
    Part {
        key: "reference",
        holds: "The published calculation the solver's first test compares with, and the rule for turning a published arc-jet test into a question.",
    },
    Part {
        key: "example_solve_stardust_near_nose",
        holds: "The Stardust near-nose question on `PICA, open record, version 5` in full, in air, 136 rows of NASA's bare-wall heating, as a request that runs unchanged. It returns the 7.58 mm of `validation.result`.",
    },
];

/// What this entry names itself: a `face` and a `stream_gas` that are not
/// one of their two words. Neither reaches the solver, which is asked a
/// question already made from them.
const DOOR_REFUSALS: &[Refusal] = &[
    Refusal {
        name: "UnknownFace",
        meaning: "`face` is neither `held_temperature` nor `gas_stream`.",
        sealed: false,
    },
    Refusal {
        name: "UnknownStreamGas",
        meaning: "`stream_gas` is neither `air` nor `mixture`.",
        sealed: false,
    },
];

/// Each `SlabReject` variant with the sentence from its doc comment, in the
/// solver's order. A refusal the request itself shows is not sealed. The
/// refusals the solver can name only once it is computing are sealed.
const REFUSALS: &[Refusal] = &[
    Refusal {
        name: "ThicknessOutOfRange",
        meaning: "`thickness_m` is outside 0.0002 to 1 m. Below 0.2 mm the grid holds no more than 8 cells; 1 m is a size cap on the request.",
        sealed: false,
    },
    Refusal {
        name: "StartTemperatureOutOfRange",
        meaning: "`start_temperature_k` is outside 260 to 800 K. The bottom is just above the bottom of the property tables, 255.6 K. Above 800 K untouched material decomposes so fast from the first step that the step fails.",
        sealed: false,
    },
    Refusal {
        name: "HeldTemperatureOutOfRange",
        meaning: "`held_temperature_k` is outside 260 to 3300 K. The top is 33 K under the top of TACOT's property tables.",
        sealed: false,
    },
    Refusal {
        name: "HoldDurationOutOfRange",
        meaning: "`hold_duration_s` is outside 1 to 3600 s. The hold is longer than the 0.1 s ramps up and down; an hour is a size cap on the request.",
        sealed: false,
    },
    Refusal {
        name: "ContinueAfterOutOfRange",
        meaning: "`continue_after_s` is outside 0 to 3600 s. An hour is a size cap on the request.",
        sealed: false,
    },
    Refusal {
        name: "TooManyThermocouples",
        meaning: "More than `THERMOCOUPLES_MAX` thermocouple depths were given.",
        sealed: false,
    },
    Refusal {
        name: "ThermocoupleOutsideSlab",
        meaning: "A thermocouple depth is not between the heated face and the back.",
        sealed: false,
    },
    Refusal {
        name: "StepNotConverged",
        meaning: "A time step did not settle within the Newton pass limit.",
        sealed: true,
    },
    Refusal {
        name: "NonFiniteValue",
        meaning: "A temperature, a density or a flux was NaN or infinite.",
        sealed: true,
    },
    Refusal {
        name: "TemperatureOutOfBounds",
        meaning: "A temperature left its band by more than 0.000001 K. For a held face the bottom of the band is the lower of the start and the held temperature, and the top is the higher, or the temperature below which decomposition gives off heat (about 920 to 950 K, by material) when that is higher: a slab below it can be warmer inside than at its face. In a gas stream the bottom is the lower of the start and the surroundings temperatures, and the top is the top of the material's property tables: 3333.33 K for TACOT 3.0, 3975 K for a PICA model, past which the material has no properties.",
        sealed: true,
    },
    Refusal {
        name: "DecompositionRanBackwards",
        meaning: "The solid's density rose somewhere, or fell below char.",
        sealed: true,
    },
    Refusal {
        name: "MassAccountBroken",
        meaning: "The solid lost a different mass from the gas that left.",
        sealed: true,
    },
    Refusal {
        name: "EnergyAccountBroken",
        meaning: "The heat stored differs from the heat in, the heat of decomposition and the heat the gas took; or the heat balance at the face is open.",
        sealed: true,
    },
    Refusal {
        name: "HistoryRowsOutOfRange",
        meaning: "The heating history has fewer than 2 or more than 256 rows.",
        sealed: false,
    },
    Refusal {
        name: "HistoryTimesOutOfOrder",
        meaning: "The first row of the heating history is not at time zero, or a row is not later than the one before it.",
        sealed: false,
    },
    Refusal {
        name: "HistoryDurationOutOfRange",
        meaning: "The heating history's last row is outside 1 to 3600 s. An hour is a size cap on the request.",
        sealed: false,
    },
    Refusal {
        name: "HistoryRowsTooClose",
        meaning: "A row of the heating history is less than 0.001 s after the row before it. A row at the same time as the one before it, or earlier, is `HistoryTimesOutOfOrder`.",
        sealed: false,
    },
    Refusal {
        name: "StreamPressureOutOfRange",
        meaning: "A wall pressure is outside the surface table of the material that was asked for: 101.325 to 101325 Pa for TACOT 3.0, 1.01325 to 101325 Pa for a PICA model. The refusal carries that table's two ends.",
        sealed: false,
    },
    Refusal {
        name: "HeatTransferCoefficientOutOfRange",
        meaning: "A heat-transfer coefficient is below zero or not a finite number.",
        sealed: false,
    },
    Refusal {
        name: "FlowEnthalpyOutOfRange",
        meaning: "A flow enthalpy is below zero or not a finite number.",
        sealed: false,
    },
    Refusal {
        name: "RadiativeHeatingOutOfRange",
        meaning: "A radiative heating is below zero or not a finite number.",
        sealed: false,
    },
    Refusal {
        name: "SurroundingsTemperatureOutOfRange",
        meaning: "`surroundings_temperature_k` is outside 250 K to the top of the material's property tables: 3333.33 K for TACOT 3.0, 3975 K for a PICA model.",
        sealed: false,
    },
    Refusal {
        name: "ContinueAfterWithStream",
        meaning: "`continue_after_s` is not zero with a gas stream. The heating history is the whole run.",
        sealed: false,
    },
    Refusal {
        name: "FaceBeyondSurfaceTable",
        meaning: "The face is hotter than the surface table goes at this pressure: at or above the temperature at which the table's carbon boils.",
        sealed: true,
    },
    Refusal {
        name: "FaceMovedBackward",
        meaning: "The char flux at the face came out negative.",
        sealed: true,
    },
    Refusal {
        name: "UnknownMaterial",
        meaning: "The material name is not one the solver holds.",
        sealed: false,
    },
    Refusal {
        name: "StreamOxygenNotAMassFraction",
        meaning: "The stream's oxygen is not a mass fraction: below 0 or above 1, or not a number.",
        sealed: false,
    },
    Refusal {
        name: "StreamArgonNotAMassFraction",
        meaning: "The stream's argon is not a mass fraction: below 0 or above 1, or not a number.",
        sealed: false,
    },
    Refusal {
        name: "StreamOxygenAndArgonAboveOne",
        meaning: "The stream's oxygen and argon together are more than the whole stream: their mass fractions add to more than 1, which leaves less than no nitrogen.",
        sealed: false,
    },
    Refusal {
        name: "StreamNotAirForThisMaterial",
        meaning: "The stream is not air and the material's surface table is for air alone (TACOT's published table).",
        sealed: false,
    },
    Refusal {
        name: "SurfaceChemistryNotConverged",
        meaning: "The chemical equilibrium of the wall gas did not converge at a state of the surface table being computed for the stream, or for the stream's own gas at a wall.",
        sealed: true,
    },
    Refusal {
        name: "SurfaceBundleUnreadable",
        meaning: "A surface bundle handed to a study could not be read.",
        sealed: true,
    },
    Refusal {
        name: "FaceOutranItsCell",
        meaning: "The face would have crossed more than the allowed share of its cell in one step.",
        sealed: true,
    },
    Refusal {
        name: "FlowEnthalpyBelowWall",
        meaning: "A row of the heating history has a stream (a coefficient above zero) whose flow enthalpy is below the enthalpy of the stream's own gas at a wall at the lower of the start and surroundings temperatures. Such a stream draws heat out of a face that cold, below its band.",
        sealed: false,
    },
];

fn field_words() -> String {
    format!(
        "SHA-256 of the count of points in the final profile through the thickness (at most {}) as 8 little-endian bytes, then the temperature at each point in kelvin as little-endian f64 bytes. These are the values of `final_profile_temperature_k`. The hash is made whether or not the request asks for the profile.",
        slab::PROFILE_POINTS_MAX
    )
}

/// Every constant a refusal's sentence names, under the name the sentence
/// uses, so the sentence can be read without the solver's source.
fn constants_named_in_refusals() -> Value {
    json!({
        "THICKNESS_MIN_M": slab::THICKNESS_MIN_M,
        "THICKNESS_MAX_M": slab::THICKNESS_MAX_M,
        "START_TEMPERATURE_MIN_K": slab::START_TEMPERATURE_MIN_K,
        "START_TEMPERATURE_MAX_K": slab::START_TEMPERATURE_MAX_K,
        "HELD_TEMPERATURE_MIN_K": slab::HELD_TEMPERATURE_MIN_K,
        "HELD_TEMPERATURE_MAX_K": slab::HELD_TEMPERATURE_MAX_K,
        "HOLD_DURATION_MIN_S": slab::HOLD_DURATION_MIN_S,
        "HOLD_DURATION_MAX_S": slab::HOLD_DURATION_MAX_S,
        "CONTINUE_AFTER_MAX_S": slab::CONTINUE_AFTER_MAX_S,
        "THERMOCOUPLES_MAX": slab::THERMOCOUPLES_MAX,
        "TEMPERATURE_BOUND_TOLERANCE_K": slab::TEMPERATURE_BOUND_TOLERANCE_K,
        "HISTORY_ROWS_MIN": slab::HISTORY_ROWS_MIN,
        "HISTORY_ROWS_MAX": slab::HISTORY_ROWS_MAX,
        "HISTORY_DURATION_MIN_S": slab::HISTORY_DURATION_MIN_S,
        "HISTORY_DURATION_MAX_S": slab::HISTORY_DURATION_MAX_S,
        "HISTORY_ROW_SPAN_MIN_S": slab::HISTORY_ROW_SPAN_MIN_S,
        "HEAT_TRANSFER_COEFFICIENT_MAX_KG_PER_M2_S": slab::HEAT_TRANSFER_COEFFICIENT_MAX_KG_PER_M2_S,
        "FLOW_ENTHALPY_MAX_J_PER_KG": slab::FLOW_ENTHALPY_MAX_J_PER_KG,
        "RADIATIVE_HEATING_MAX_W_PER_M2": slab::RADIATIVE_HEATING_MAX_W_PER_M2,
        "SURROUNDINGS_TEMPERATURE_MIN_K": slab::SURROUNDINGS_TEMPERATURE_MIN_K,
        "SURROUNDINGS_TEMPERATURE_MAX_K": slab::SURROUNDINGS_TEMPERATURE_MAX_K,
        "CELLS_MIN": slab::CELLS_MIN,
        "ARC_JET_THICKNESS_M": slab::ARC_JET_THICKNESS_M,
        "ARC_JET_START_TEMPERATURE_K": slab::ARC_JET_START_TEMPERATURE_K,
        "ARC_JET_SURROUNDINGS_TEMPERATURE_K": slab::ARC_JET_SURROUNDINGS_TEMPERATURE_K,
        "STARDUST_THICKNESS_M": slab::STARDUST_THICKNESS_M,
        "STARDUST_START_TEMPERATURE_K": slab::STARDUST_START_TEMPERATURE_K,
        "STARDUST_SURROUNDINGS_TEMPERATURE_K": slab::STARDUST_SURROUNDINGS_TEMPERATURE_K,
        "THERMOCOUPLE_BLOCK_THICKNESS_MIN_M": slab::THERMOCOUPLE_BLOCK_THICKNESS_MIN_M,
        "THERMOCOUPLE_BLOCK_THICKNESS_MAX_M": slab::THERMOCOUPLE_BLOCK_THICKNESS_MAX_M,
        "THERMOCOUPLE_BLOCK_START_TEMPERATURE_K": slab::THERMOCOUPLE_BLOCK_START_TEMPERATURE_K,
        "THERMOCOUPLE_BLOCK_SURROUNDINGS_TEMPERATURE_K": slab::THERMOCOUPLE_BLOCK_SURROUNDINGS_TEMPERATURE_K,
        "COMPARED_TEMPERATURE_MAX_K": slab::COMPARED_TEMPERATURE_MAX_K,
    })
}

/// Each material's lowest and highest wall pressure, from the solver.
fn pressure_ranges() -> Value {
    let mut ranges = serde_json::Map::new();
    for material in MATERIALS {
        if let Ok((lowest, highest)) = slab::stream_pressure_range_pa(material) {
            ranges.insert((*material).to_string(), json!([lowest, highest]));
        }
    }
    Value::Object(ranges)
}

/// The rule the PICA model was developed with for turning an arc-jet
/// test's published condition into a question, as the record of the
/// Stardust model run writes it out.
fn arc_jet_recipe() -> Value {
    json!({
        "what": "How a published arc-jet test (cold-wall heating, pressure, flow enthalpy, exposure time and the stream's gas) becomes a question. It is the rule the PICA model was developed with, and the 36 development tests asked this way return the recorded wear and face temperature.",
        "written_out_in": "evidence/research/stardust_heatshield_model_run/arcjet_test_to_question/RECIPE.md",
        "steps": [
            format!(
                "The stream's gas. An arc jet's stream is not air: 'stream_gas' is \"mixture\". Air with argon mixed in: 'stream_argon_mass_fraction' is the argon, and 'stream_oxygen_mass_fraction' is {} times one less the argon. Nitrogen with oxygen: the oxygen as given, and argon 0.",
                slab::ARCJET_AIR_OXYGEN_MASS_FRACTION
            ),
            format!(
                "The enthalpy of that gas at a cold wall, at {} K: `stream_enthalpy_at_cold_wall_j_per_kg` in the answer to any question asked with that gas. It depends on the gas alone, so a question of one second at the test's own pressure, heating and enthalpy gives it. It is 1,621 to 1,901 J/kg for the development tests.",
                slab::COLD_WALL_TEMPERATURE_K
            ),
            "The heat-transfer coefficient, kg/m²/s: the cold-wall heating in W/m², divided by the flow enthalpy in J/kg less the enthalpy of step two.".to_string(),
            "The question: `PICA, open record, version 5`, thickness 0.06 m, start and surroundings at 300 K, the face wears, no thermocouples, and three rows of history: at 0 s the coefficient is zero; at 0.5 s and at the end of the exposure it is the coefficient of step three. The pressure and the flow enthalpy are the test's in every row, and the radiative heating is zero.".to_string(),
            "What is read: `wear_m` against the measured wear, and `peak_face_temperature_k` against the highest measured face temperature.".to_string(),
        ],
        "worked": "`example_solve_arc_jet_test` is the first development test done this way: 107 W/cm², 2.3 kPa, 15.2 MJ/kg, 55 s, argon 0.276. The coefficient is 1,070,000 / (15,200,000 − 1,620.577) = 0.070402243 kg/m²/s. It returns wear of 2.0047 mm (2.33 measured) and a peak face of 2,012.5 K (2,033 measured).",
    })
}

fn sections() -> Sections {
    Sections {
        locked: json!({
            "material": "Each material is a file of locked numbers inside the solver, and none is an input. `validation.object` says where they come from and names the one that is fitted.",
            "surface_table_pressure_pa": pressure_ranges(),
            "air": {
                "what": "`stream_gas` `air` is 21 percent oxygen and 79 percent nitrogen by mole, and reads the surface table locked in the solver. A `mixture` has its table computed when it is asked for, from NASA's open thermodynamic records.",
                "oxygen_mass_fraction": slab::AIR_OXYGEN_MASS_FRACTION,
                "a_mixture_with_airs_numbers": "A `mixture` whose oxygen is exactly this number, with no argon, is air to the solver: it reads the locked table, returns air's numbers and is taken by `TACOT 3.0`. Its request still says `mixture`, so its proof is not that of the same question asked as `air`. Any other number, 0.2329 for one, is another gas.",
            },
            "cold_wall_temperature_k": slab::COLD_WALL_TEMPERATURE_K,
            "compared_with_measurement": {
                "what": "What the PICA model has been compared with measurement on. A PICA question is answered inside these envelopes and outside them alike, with the same equations and the same accounts at every step, and stops only at a limit of the physics or the arithmetic. The answer says whether the question lies wholly inside an envelope, its thickness, start and surroundings included, in `inside_arc_jet_comparison`, `inside_stardust_comparison` and `inside_thermocouple_block_comparison`; a question in none has 0 in all three. Values within one part in 10^9 of an end count as at it. TACOT 3.0, compared with other codes only, lies in none.",
                "every_compared_pica_case": {
                    "face": "a gas stream, the face wearing; no PICA case held its face or kept it from wearing",
                    "temperature_max_k": slab::COMPARED_TEMPERATURE_MAX_K,
                    "temperature_max_is": "The hottest temperature any accepted step reached in the compared runs, the Stardust near-nose run. A PICA run may go hotter, to the top of its tables at 3975 K, past which it is refused as `TemperatureOutOfBounds`. From 3333 K, where TACOT's tables stop, the PICA tables hold the specific heat at its 3333 K value and continue the conductivity along its last straight line; nothing measured stands there.",
                    "table_top_k": genesis_solvers::pica::TABLE_TOP_K,
                },
                "arc_jet": {
                    "compared": "The 36 development tests (Milos and Chen 2010) and the 31 held-back tests, asked by the recipe in `reference`: 0.06 m, start and surroundings at 300 K. Wear and the hottest face temperature were measured.",
                    "thickness_m": slab::ARC_JET_THICKNESS_M,
                    "start_temperature_k": slab::ARC_JET_START_TEMPERATURE_K,
                    "surroundings_temperature_k": slab::ARC_JET_SURROUNDINGS_TEMPERATURE_K,
                    "pressure_pa": [slab::ARC_JET_PRESSURE_MIN_PA, slab::ARC_JET_PRESSURE_MAX_PA],
                    "cold_wall_heating_w_per_m2": [slab::ARC_JET_HEATING_MIN_W_PER_M2, slab::ARC_JET_HEATING_MAX_W_PER_M2],
                    "cold_wall_heating_is": "the coefficient times the flow enthalpy less `stream_enthalpy_at_cold_wall_j_per_kg`, everywhere between rows; below the lowest only before `swing_in_s`",
                    "swing_in_s": slab::ARC_JET_SWING_IN_S,
                    "flow_enthalpy_j_per_kg": [slab::ARC_JET_FLOW_ENTHALPY_MIN_J_PER_KG, slab::ARC_JET_FLOW_ENTHALPY_MAX_J_PER_KG],
                    "heating_history_ends_by_s": slab::ARC_JET_DURATION_MAX_S,
                    "stream_oxygen_mass_fraction": [slab::ARC_JET_STREAM_OXYGEN_MASS_FRACTION_MIN, slab::ARC_JET_STREAM_OXYGEN_MASS_FRACTION_MAX],
                    "stream_argon_mass_fraction_max": slab::ARC_JET_STREAM_ARGON_MASS_FRACTION_MAX,
                    "radiative_heating": "none",
                    "cooling_after_the_stream": "none: the heating stays within the range to the end",
                },
                "stardust_flight": {
                    "compared": "The two cores cut from the recovered Stardust shield, near the nose and on the flank, with NASA's bare-wall heating: 0.0582 m, start 260 K, surroundings 266.48 K, in air. Wear and the density through each core were measured; the heating is NASA's computation, not a measurement.",
                    "thickness_m": slab::STARDUST_THICKNESS_M,
                    "start_temperature_k": slab::STARDUST_START_TEMPERATURE_K,
                    "surroundings_temperature_k": slab::STARDUST_SURROUNDINGS_TEMPERATURE_K,
                    "stream_gas": "air",
                    "pressure_pa": [slab::STARDUST_PRESSURE_MIN_PA, slab::STARDUST_PRESSURE_MAX_PA],
                    "flow_enthalpy_j_per_kg": [slab::STARDUST_FLOW_ENTHALPY_MIN_J_PER_KG, slab::STARDUST_FLOW_ENTHALPY_MAX_J_PER_KG],
                    "heat_transfer_coefficient_max_kg_per_m2_s": slab::STARDUST_COEFFICIENT_MAX_KG_PER_M2_S,
                    "cold_wall_heating_max_w_per_m2": slab::STARDUST_HEATING_MAX_W_PER_M2,
                    "cold_wall_heating_min": "none: the flight's tails run below 107 W/cm²",
                    "radiative_heating_max_w_per_m2": slab::STARDUST_RADIATIVE_HEATING_MAX_W_PER_M2,
                    "stream": "on in one stretch that ends by the time below, then cooling by radiation to the end of the history",
                    "stream_ends_by_s": slab::STARDUST_STREAM_ENDS_BY_S,
                },
                "thermocouple_blocks": {
                    "compared": "The four PICA blocks of Tran and others 1997 (9B, 7B, 3B and 6A), with thermocouples in depth, asked as the development runs asked them: in air, the coefficient set from the block's cold-wall heating with the flow enthalpy 29.5 MJ/kg, swung in over 0.5 s, held to the end of the exposure, then falling to zero over 0.5 s, and the block cooling. The thermocouple temperatures were measured; the one fitted number was fitted to them.",
                    "thickness_m": [slab::THERMOCOUPLE_BLOCK_THICKNESS_MIN_M, slab::THERMOCOUPLE_BLOCK_THICKNESS_MAX_M],
                    "start_temperature_k": slab::THERMOCOUPLE_BLOCK_START_TEMPERATURE_K,
                    "surroundings_temperature_k": slab::THERMOCOUPLE_BLOCK_SURROUNDINGS_TEMPERATURE_K,
                    "stream_gas": "air",
                    "pressure_pa": [slab::THERMOCOUPLE_BLOCK_PRESSURE_MIN_PA, slab::THERMOCOUPLE_BLOCK_PRESSURE_MAX_PA],
                    "pressure_is": "one pressure, the same in every row",
                    "flow_enthalpy_j_per_kg": slab::THERMOCOUPLE_BLOCK_FLOW_ENTHALPY_J_PER_KG,
                    "cold_wall_heating_w_per_m2": [slab::THERMOCOUPLE_BLOCK_HEATING_MIN_W_PER_M2, slab::THERMOCOUPLE_BLOCK_HEATING_MAX_W_PER_M2],
                    "cold_wall_heating_is": "everywhere between rows at most the top; at least the bottom from `swing_in_s` to the end of the exposure",
                    "swing_in_s": slab::THERMOCOUPLE_BLOCK_SWING_IN_S,
                    "exposure_s": [slab::THERMOCOUPLE_BLOCK_EXPOSURE_MIN_S, slab::THERMOCOUPLE_BLOCK_EXPOSURE_MAX_S],
                    "ramp_down_s": slab::THERMOCOUPLE_BLOCK_RAMP_DOWN_S,
                    "stream": "on in one stretch: held to the end of the exposure, then falling to zero in a straight line over `ramp_down_s`, then cooling by radiation to the end of the history",
                    "heating_history_ends_by_s": slab::THERMOCOUPLE_BLOCK_HISTORY_ENDS_BY_S,
                    "radiative_heating": "none",
                },
            },
            "surface_ramp_s": slab::SURFACE_RAMP_S,
            "report_interval_s": slab::REPORT_INTERVAL_S,
            "history_points_max": slab::HISTORY_POINTS_MAX,
            "profile_points_max": slab::PROFILE_POINTS_MAX,
            "first_cell_m": slab::FIRST_CELL_M,
            "cell_growth": slab::CELL_GROWTH,
            "largest_cell_m": slab::LARGEST_CELL_M,
            "largest_cell_in_a_stream": slab::LARGEST_CELL_IN_A_STREAM,
            "first_step_s": slab::FIRST_STEP_S,
            "step_fraction": slab::STEP_FRACTION,
            "largest_step_s": slab::LARGEST_STEP_S,
            "wear_per_step": slab::WEAR_PER_STEP,
            "blowing_correction_lambda": slab::BLOWING_CORRECTION_LAMBDA,
            "stefan_boltzmann_w_per_m2_k4": slab::STEFAN_BOLTZMANN_W_PER_M2_K4,
            "constants_named_in_refusals": constants_named_in_refusals(),
        }),
        invariants: json!([
            {
                "name": "energy_account_residual_j_per_m2",
                "rule": "The heat stored in the solid equals the heat conducted in through the face, plus the heat given off by decomposition, less the heat the gas took on its way out.",
                "tolerance": "`energy_account_tolerance_j_per_m2` of the same answer",
                "tolerance_of_a_step": slab::ENERGY_ACCOUNT_TOLERANCE,
                "unit": "fraction of the energy that moved in the step",
                "compared": "every step, and over the whole run",
                "refusal": "EnergyAccountBroken",
            },
            {
                "name": "face_balance_residual_j_per_m2",
                "rule": "Under a gas stream, the heat conducted into the slab equals the heat from the stream plus the radiation taken up, less the heat radiated away and the heat the gases take at the face.",
                "tolerance": "`face_balance_tolerance_j_per_m2` of the same answer",
                "unit": "J/m²",
                "compared": "every step, and over the whole run",
                "refusal": "EnergyAccountBroken",
            },
            {
                "name": "mass_account_residual_kg_per_m2",
                "rule": "The mass the solid lost equals the gas that left plus the char worn off.",
                "tolerance": "`mass_account_tolerance_kg_per_m2` of the same answer",
                "tolerance_of_a_step": slab::MASS_ACCOUNT_TOLERANCE,
                "unit": "fraction of the mass that moved in the step",
                "compared": "every step, and over the whole run",
                "refusal": "MassAccountBroken",
            },
        ]),
        convergence: json!({
            "newton_temperature_tolerance_k": slab::NEWTON_TEMPERATURE_TOLERANCE_K,
            "newton_gas_flux_tolerance_kg_per_m2_s": slab::NEWTON_GAS_FLUX_TOLERANCE_KG_PER_M2_S,
            "newton_pass_limit": slab::NEWTON_PASS_LIMIT,
            "rule": "Inside each time step the face temperature, the char leaving the face and every cell's temperature and gas flow are solved together, pass after pass, until no temperature and no gas flow moves by its tolerance. A step that has not settled within the passes allowed stops the run, and no number is returned. Every accepted step is then verified: every value finite, every temperature in its band, no resin grown back, the face not moved outward, and the three accounts closed.",
            "refusal": "StepNotConverged",
        }),
        reference: json!({
            "name": slab::REFERENCE_NAME,
            "url": slab::REFERENCE_URL,
            "retrieved": slab::REFERENCE_RETRIEVED,
            "material": slab::MATERIAL_NAME,
            "material_url": slab::MATERIAL_URL,
            "test": "test_case_1_against_fiat",
            "covers": "TACOT 3.0 with the face held at 1644 K for 60 s: temperature at five depths over the minute. The same archive holds the reference results of workshop cases 2.1, 2.2 and 2.3, which the solver's tests also compare with.",
            "kind": "A calculation by an established code. It is not a measurement. The comparisons with measurement are in `validation`.",
            "arc_jet_test_to_question": arc_jet_recipe(),
        }),
    }
}

/// The validation record, in the five parts a card's description gives: the
/// object, the data, the pass line, the result and the limits. Every
/// statement is from the solver's header comment or from the record of the
/// Stardust model run.
fn validation() -> [String; 5] {
    let object = format!(
        "A flat slab of charring heat-shield material heated on one face. Heat conducts in through the thickness, the resin breaks down once as it gets hot, its gas flows out through the heated face and takes heat with it, and what is left is char. The back passes no heat. `{served}` is the PICA model a caller is pointed to. PICA's own property model is export controlled, so this is a stand-in built from the open record: PICA's measured density and room-temperature conductivity; PICA's own heat capacity and char conductivity as measured in 1997, the conductivity used at 0.71 of the measured curve, the one fitted number, fitted to thermocouples in four arc-jet blocks; conductivity that falls with pressure as measured in 2022; the decomposition law of the stand-in material TACOT, held off below 556 K; a table of the surface chemistry in air computed at 23 pressures; and three published rules at the surface: gas and stream do not react at low pressure, oxygen reacts at the wall only so fast, and 5 percent of the char is lost as solid. `{tacot}` is a made-up open test material: not PICA, nothing fitted.",
        served = slab::MATERIAL_PICA_SERVED,
        tacot = slab::MATERIAL_TACOT,
    );
    let data = format!(
        "Calculations, on TACOT: four standard test cases of the Ablation Workshop against the codes FIAT, PATO and Amaryllis. Measurements, on the PICA model: thermocouples in four blocks tested in an arc jet, where very hot gas is blown at a sample (Tran and others 1997), 259 samples, the data the fitted number was fitted to; wear and face temperature in 36 arc-jet tests (Milos and Chen 2010; 107 to 1,102 W/cm², 2.3 to 84 kPa; streams with oxygen {oxygen_min} to {oxygen_max} and argon up to {argon_max} by mass), on which the surface rules were chosen; and the Stardust capsule's entry at the two places where cores were cut from the recovered shield, with wear of 5.7 mm near the nose (6.1 by X-ray) and 3.2 mm on the flank, and the density through each core. Each version of the model was frozen before it was run on the flight, and nothing in version 5 was chosen on the flight. The flight's heating is NASA's computation, not a measurement. What was compared: the arc-jet tests, 0.06 m from 300 K, 7.3 to 29.2 MJ/kg, up to 400 s, no radiative heating, no cooling; the Stardust cores, 0.0582 m from 260 K in air, to 78.3 MJ/kg and 0.74 MW/m² radiative, the stream off by 134 s, then cooling; the thermocouple blocks, 0.0224 to 0.0325 m from 303.15 K in air at 0.2 to 0.45 atm, 29.5 MJ/kg, 400 to 580 W/cm² for 15 to 40 s, then cooling to 152 s. No PICA case held its face or kept it from wearing, and no compared run went above 3,412 K. The numerics, on TACOT, were measured over 888 held-temperature questions and 3,456 heating histories with slabs of 0.01 to 0.10 m, starts of 260 to 400 K, holds of 1 to 300 s, histories to 600 s, coefficients to 1 kg/m²/s, flow enthalpies to 100 MJ/kg and radiative heating to 10 MW/m², hardest step 4 and 6 Newton passes; on the PICA model over 294 histories across that range, 269 answered with every account closed and 25 worn through, hardest step 11 passes, and 350 in seven streams from 5 to 40 percent oxygen and 0 to 50 percent argon, all answered, hardest step 9. {HELD_BACK}",
        oxygen_min = slab::ARC_JET_STREAM_OXYGEN_MASS_FRACTION_MIN,
        oxygen_max = slab::ARC_JET_STREAM_OXYGEN_MASS_FRACTION_MAX,
        argon_max = slab::ARC_JET_STREAM_ARGON_MASS_FRACTION_MAX,
    );
    let pass_line = "For the 31 held-back arc-jet tests the lines were fixed on 2026-10-07, before any model was run. Wear: within sqrt(0.5² + (0.16 × measured)²) mm on at least 25 of the 31, with a mean error within 8 percent. Hottest face temperature: within 7 percent on at least 23 of the 28 that have one, with a mean error within 3.5 percent. The model passes only if both do. No line was fixed beforehand for the workshop cases or for Stardust, where each version of the model was frozen before its run. The comparison is not blind: before the lines were fixed, the author of the protocol had transcribed every number of the paper's Table 4, the held-back tests included, and the wear line's 16 percent takes in the 6.5 percent replicate scatter of blocks in both sets; the model's author did not open the held-back file, and the surface rules were chosen on the 36 development tests.".to_string();
    let result = format!("TACOT against codes: test case 1 within 0.63 percent root mean square of FIAT; cases 2.1 and 2.2 with the face within 1.4 K of Amaryllis and wear of 11.906 against 11.918 mm; case 2.3 as close only with the surface table of its day. PICA version 5 against measurement. Thermocouple blocks: 31.3 K root mean square. Arc-jet tests: wear within 20 percent on 35 of 36, 4.9 percent low on average; face temperature within 7 percent on 30 of 34, 3.8 percent high on average. {HELD_BACK_RESULT} Stardust, nose and flank, with NASA's bare-wall heating and this solver's own corrections, which is what a request computes: wear of 7.58 and 3.32 mm against 5.7 and 3.2 measured, a third high at the nose and 3 percent high on the flank; density half way from char to untouched material at 7.4 and 9.1 mm deep against 5.6 and 9.0; back within 2 percent of untouched at 12.0 and 14.4 mm against 8.65 and 10.74. With NASA's coupled heating, run in a study and not open to a request: wear of 6.45 and 3.19 mm.");
    let limits = format!("Known misses. 1. The faint end of the charring is too deep: the density is back within 2 percent of untouched material 3 to 4 mm deeper than the cores show and than NASA's code computes. Not explained. 2. Wear at the Stardust nose is a third high with the solver's own heating and 13 percent high with NASA's coupled heating: its stream heating at the peak is 22 to 35 percent above NASA's coupled solutions; one number cannot stand in for a solution of the flow. 3. The face runs 6.8 percent hot above 700 W/cm² in the arc-jet tests. 4. Wear is 12 percent low at 8.5 kPa and below in them. 5. One number is fitted, the char conductivity scale (0.71). 6. The decomposition law is the stand-in's: measured PICA starts later when heated slowly and earlier when heated fast. 7. Above 13 kPa a cool wall still burns the decomposition gas in oxygen it leaves unused; no rule was found. 8. A face at 800 K still burns a little char. 9. The decomposition gas inside the slab is read at 1 atm at every wall pressure. 10. From 3333 K, where TACOT's tables stop, to {table_top} K the PICA tables are extended, not measured: the specific heat is held at its 3333 K value and the conductivity is continued along its last straight line. No compared run went above {top} K. 11. In a stream with no oxygen the model burns no char and wears only the char it loses as solid: in the held-back test in pure nitrogen it wears 0.22 mm where 1.75 mm was measured. Not compared: PICA cooling with a measured temperature; measured radiative heating; a held PICA face; a PICA face kept from wearing. {HELD_BACK_SPENT} The part `reference` has the rule for turning a published arc-jet test into a question. A PICA question outside what was compared with measurement is answered with the same equations and the same accounts, and says so: `inside_arc_jet_comparison`, `inside_stardust_comparison` and `inside_thermocouple_block_comparison` are all 0. The envelopes are in the part `locked`. The declared range of both materials is the same: a thickness of 0.2 mm to 1 m, a start of 260 to 800 K, a held face of 260 to 3300 K, a hold, a time after it and a heating history each up to an hour, any finite coefficient, flow enthalpy and radiative heating, and a stream of any make-up of oxygen, argon and nitrogen; the metre and the hour are size caps on the request. A run stops only at a limit of the physics or the arithmetic: the slab worn away, a face that boils, a temperature above the top of the material's tables ({tacot_top} K for TACOT 3.0, {table_top} K for PICA), a step that does not settle, or a broken account. A proof shows that an answer was not changed, not that the model matches the material.", top = slab::COMPARED_TEMPERATURE_MAX_K, table_top = genesis_solvers::pica::TABLE_TOP_K, tacot_top = slab::SURROUNDINGS_TEMPERATURE_MAX_K);
    [object, data, pass_line, result, limits]
}

/// Test case 1 of the Ablation Workshop on TACOT, as the solver holds it:
/// 5 cm from 298 K, the face held at 1644 K for 60 s. Summary only.
fn example() -> Value {
    let question = slab::ablation_workshop_test_case_1();
    let (held_temperature_k, hold_duration_s) = match question.surface {
        SurfaceCondition::HeldTemperature {
            held_temperature_k,
            hold_duration_s,
        } => (held_temperature_k, hold_duration_s),
        SurfaceCondition::GasStream { .. } => unreachable!("test case 1 is a held face"),
    };
    json!({
        "material": slab::MATERIAL_TACOT,
        "thickness_m": question.thickness_m,
        "start_temperature_k": question.start_temperature_k,
        "thermocouple_depths_m": question.thermocouple_depths_m,
        "face": FACE_HELD,
        "held_temperature_k": held_temperature_k,
        "hold_duration_s": hold_duration_s,
        "continue_after_s": question.continue_after_s,
        ASK_FACE: false,
        ASK_THERMOCOUPLES: false,
        ASK_PROFILE: false,
    })
}

/// The first of the 36 development arc-jet tests (Milos and Chen 2010, model
/// AA-43-209-N), made into a question by the rule in `arc_jet_recipe`: 107
/// W/cm², 2.3 kPa, 15.2 MJ/kg, 55 s, in air with 27.6 percent argon. The
/// numbers are those of the first of the record's 36 questions. Summary
/// only.
fn arc_jet_example() -> Value {
    json!({
        "material": slab::MATERIAL_PICA_SERVED,
        "thickness_m": 0.06,
        "start_temperature_k": 300.0,
        "thermocouple_depths_m": [],
        "face": FACE_STREAM,
        "stream_gas": GAS_MIXTURE,
        "stream_oxygen_mass_fraction": 0.16753359999999998,
        "stream_argon_mass_fraction": 0.276,
        "history_time_s": [0.0, 0.5, 55.0],
        "history_pressure_pa": [2300.0, 2300.0, 2300.0],
        "history_heat_transfer_coefficient_kg_per_m2_s": [0.0, 0.07040224291374243, 0.07040224291374243],
        "history_flow_enthalpy_j_per_kg": [1.52e7, 1.52e7, 1.52e7],
        "history_radiative_heating_w_per_m2": [0.0, 0.0, 0.0],
        "surroundings_temperature_k": 300.0,
        "face_wears": true,
        ASK_FACE: false,
        ASK_THERMOCOUPLES: false,
        ASK_PROFILE: false,
    })
}

/// The Stardust entry at the near-nose core on the PICA model a caller is
/// pointed to, with NASA's bare-wall heating. Summary only.
fn stardust_example() -> Value {
    let column = |index: usize| -> Vec<f64> {
        stardust::HISTORY.iter().map(|row| row[index]).collect()
    };
    json!({
        "material": slab::MATERIAL_PICA_SERVED,
        "thickness_m": stardust::THICKNESS_M,
        "start_temperature_k": stardust::START_TEMPERATURE_K,
        "thermocouple_depths_m": stardust::THERMOCOUPLE_DEPTHS_M,
        "face": FACE_STREAM,
        "stream_gas": GAS_AIR,
        "history_time_s": column(0),
        "history_pressure_pa": column(1),
        "history_heat_transfer_coefficient_kg_per_m2_s": column(2),
        "history_flow_enthalpy_j_per_kg": column(3),
        "history_radiative_heating_w_per_m2": column(4),
        "surroundings_temperature_k": stardust::SURROUNDINGS_TEMPERATURE_K,
        "face_wears": true,
        ASK_FACE: false,
        ASK_THERMOCOUPLES: false,
        ASK_PROFILE: false,
    })
}

/// The solver's own question, made from the parameters, with the material
/// and what to return. Nothing is in it that the request did not give: with
/// a gas stream the time after the hold is not an input, because the heating
/// history is the whole run, and it is the zero the solver requires there.
fn question(given: &Given) -> Result<(String, SlabQuestion, Wanted), String> {
    let material = given.word("material")?.to_string();
    let (surface, continue_after_s) = if given.word("face")? == FACE_HELD {
        (
            SurfaceCondition::HeldTemperature {
                held_temperature_k: given.number("held_temperature_k")?,
                hold_duration_s: given.number("hold_duration_s")?,
            },
            given.number("continue_after_s")?,
        )
    } else {
        let time_s = given.numbers("history_time_s")?;
        let columns = [
            ("history_pressure_pa", given.numbers("history_pressure_pa")?),
            (
                "history_heat_transfer_coefficient_kg_per_m2_s",
                given.numbers("history_heat_transfer_coefficient_kg_per_m2_s")?,
            ),
            (
                "history_flow_enthalpy_j_per_kg",
                given.numbers("history_flow_enthalpy_j_per_kg")?,
            ),
            (
                "history_radiative_heating_w_per_m2",
                given.numbers("history_radiative_heating_w_per_m2")?,
            ),
        ];
        // A row with a number missing is not a row. This is asked before any
        // limit of the solver, which takes whole rows.
        for (name, column) in columns {
            if column.len() != time_s.len() {
                return Err(format!(
                    "Physical Violation: the five columns of the heating history hold one number for each row, and 'history_time_s' holds {} where '{name}' holds {}.",
                    time_s.len(),
                    column.len()
                ));
            }
        }
        let history = (0..time_s.len())
            .map(|row| StreamRow {
                time_s: time_s[row],
                pressure_pa: columns[0].1[row],
                heat_transfer_coefficient_kg_per_m2_s: columns[1].1[row],
                flow_enthalpy_j_per_kg: columns[2].1[row],
                radiative_heating_w_per_m2: columns[3].1[row],
            })
            .collect();
        (
            SurfaceCondition::GasStream {
                history,
                surroundings_temperature_k: given.number("surroundings_temperature_k")?,
                face_wears: given.flag("face_wears")?,
                // `air` is the solver's own air, exactly. A mixture is the
                // two fractions as given.
                stream_gas: if given.word("stream_gas")? == GAS_AIR {
                    StreamGas::AIR
                } else {
                    StreamGas {
                        oxygen_mass_fraction: given.number("stream_oxygen_mass_fraction")?,
                        argon_mass_fraction: given.number("stream_argon_mass_fraction")?,
                    }
                },
            },
            0.0,
        )
    };
    let question = SlabQuestion {
        thickness_m: given.number("thickness_m")?,
        start_temperature_k: given.number("start_temperature_k")?,
        surface,
        continue_after_s,
        thermocouple_depths_m: given.numbers("thermocouple_depths_m")?.to_vec(),
    };
    let wanted = Wanted {
        face_history: given.flag(ASK_FACE)?,
        thermocouple_histories: given.flag(ASK_THERMOCOUPLES)?,
        final_profile: given.flag(ASK_PROFILE)?,
    };
    Ok((material, question, wanted))
}

/// The sentence a caller reads for a limit the request itself shows: the
/// solver's own sentence and the solver's name for it.
fn door_sentence(reject: &SlabReject) -> String {
    format!("Physical Violation: {reject} ({}).", reject.name())
}

/// Every limit the request itself shows, asked of the solver's own check of
/// a question before a physics permit is taken.
fn check(given: &Given) -> Result<(), String> {
    let (material, question, _) = question(given)?;
    slab::check_question(&material, &question).map_err(|reject| door_sentence(&reject))
}

fn solve(given: &Given) -> Result<Solved, String> {
    let (material, question, wanted) = question(given)?;
    // The final profile is always taken from the solver: `field_sha256`
    // covers its temperatures. It is published only when asked for.
    let taken = Wanted {
        final_profile: true,
        ..wanted
    };
    let stream_gas = match &question.surface {
        SurfaceCondition::GasStream { stream_gas, .. } => Some(*stream_gas),
        SurfaceCondition::HeldTemperature { .. } => None,
    };
    Ok(solved(
        slab::answer(&material, &question, &taken),
        stream_gas,
        &wanted,
    ))
}

/// A refusal in the words every solver shares. A refusal the request itself
/// shows is answered with no seal and with the solver's sentence alone, so
/// its sentence carries the solver's name for it.
fn refused(reject: &SlabReject) -> Refused {
    let name = reject.name();
    let sealed = REFUSALS
        .iter()
        .any(|refusal| refusal.name == name && refusal.sealed);
    Refused {
        name,
        sentence: if sealed {
            reject.to_string()
        } else {
            format!("{reject} ({name})")
        },
        iterations: None,
    }
}

/// 1 for yes and 0 for no, as a solution publishes a yes or a no.
fn yes_or_no(yes: bool) -> f64 {
    if yes { 1.0 } else { 0.0 }
}

/// What the solver returned, in the words every solver shares.
fn solved(
    returned: Result<slab::Answer, SlabReject>,
    stream_gas: Option<StreamGas>,
    wanted: &Wanted,
) -> Solved {
    let answer = match returned {
        Ok(answer) => answer,
        Err(reject) => return Solved::Refused(refused(&reject)),
    };
    // The enthalpy of the stream's gas at a cold wall, from the solver. It
    // depends on the gas alone. A held face has no stream, and zero.
    let cold_wall_j_per_kg = match stream_gas {
        None => 0.0,
        Some(gas) => match slab::stream_enthalpy_at_cold_wall_j_per_kg(&gas) {
            Ok(enthalpy) => enthalpy,
            Err(reject) => return Solved::Refused(refused(&reject)),
        },
    };
    let s = &answer.summary;
    let number = |name: &str, value: f64| (name.to_string(), Published::Number(value));
    // A count of cells, steps or passes is a `u32`, and every `u32` is exact
    // as a 64-bit number.
    let count = |name: &str, value: u32| (name.to_string(), Published::Number(f64::from(value)));
    let mut result = vec![
        number(
            "material_compared_with_measurement",
            yes_or_no(s.material_compared_with_measurement),
        ),
        number("duration_s", s.duration_s),
        number("slab_worn_away", yes_or_no(s.worn_away)),
        number("wear_m", s.wear_m),
        number("peak_face_temperature_k", s.peak_face_temperature_k),
        number("peak_face_temperature_at_s", s.peak_face_temperature_at_s),
        number("final_face_temperature_k", s.final_face_temperature_k),
        number("final_back_temperature_k", s.final_back_temperature_k),
        number("depth_half_way_to_virgin_m", s.depth_half_way_to_virgin_m),
        number(
            "depth_within_2_percent_of_virgin_m",
            s.depth_within_2_percent_of_virgin_m,
        ),
        number("heat_from_stream_j_per_m2", s.heat_from_stream_j_per_m2),
        number(
            "heat_absorbed_from_radiation_j_per_m2",
            s.heat_absorbed_from_radiation_j_per_m2,
        ),
        number("heat_radiated_away_j_per_m2", s.heat_radiated_away_j_per_m2),
        number(
            "heat_taken_at_face_by_gases_j_per_m2",
            s.heat_taken_at_face_by_gases_j_per_m2,
        ),
        number("heat_in_through_face_j_per_m2", s.heat_in_through_face_j_per_m2),
        number("heat_stored_in_solid_j_per_m2", s.heat_stored_in_solid_j_per_m2),
        number("heat_taken_by_gas_j_per_m2", s.heat_taken_by_gas_j_per_m2),
        number(
            "heat_given_off_by_decomposition_j_per_m2",
            s.heat_given_off_by_decomposition_j_per_m2,
        ),
        number("char_enthalpy_removed_j_per_m2", s.char_enthalpy_removed_j_per_m2),
        number("solid_mass_lost_kg_per_m2", s.solid_mass_lost_kg_per_m2),
        number("gas_mass_out_kg_per_m2", s.gas_mass_out_kg_per_m2),
        number("char_mass_removed_kg_per_m2", s.char_mass_removed_kg_per_m2),
        number(
            "inside_arc_jet_comparison",
            yes_or_no(s.inside_arc_jet_comparison),
        ),
        number(
            "inside_stardust_comparison",
            yes_or_no(s.inside_stardust_comparison),
        ),
        number(
            "inside_thermocouple_block_comparison",
            yes_or_no(s.inside_thermocouple_block_comparison),
        ),
        number("stream_enthalpy_at_cold_wall_j_per_kg", cold_wall_j_per_kg),
        count("cells", s.cells),
        count("cells_at_end", s.cells_at_end),
        count("time_steps", s.time_steps),
        count("hardest_step_newton_passes", s.hardest_step_newton_passes),
    ];
    let numbers = |name: &str, values: Vec<f64>| (name.to_string(), Published::Numbers(values));
    if let (true, Some(history)) = (wanted.face_history, answer.face_history) {
        result.push(numbers("face_history_time_s", history.time_s));
        result.push(numbers("face_history_temperature_k", history.face_temperature_k));
        result.push(numbers("face_history_recession_m", history.recession_m));
        result.push(numbers(
            "face_history_char_front_depth_m",
            history.char_front_depth_m,
        ));
        result.push(numbers(
            "face_history_virgin_front_depth_m",
            history.virgin_front_depth_m,
        ));
        result.push(numbers(
            "face_history_gas_mass_flux_kg_per_m2_s",
            history.gas_mass_flux_kg_per_m2_s,
        ));
        result.push(numbers(
            "face_history_char_mass_flux_kg_per_m2_s",
            history.char_mass_flux_kg_per_m2_s,
        ));
    }
    if let (true, Some(histories)) = (wanted.thermocouple_histories, answer.thermocouple_histories)
    {
        result.push(numbers(
            "thermocouple_depth_m",
            histories.iter().map(|history| history.depth_m).collect(),
        ));
        result.push(numbers(
            "thermocouple_passed_by_face",
            histories
                .iter()
                .map(|history| yes_or_no(history.passed_by_face))
                .collect(),
        ));
        result.push(numbers(
            "thermocouple_covered_until_s",
            histories
                .iter()
                .map(|history| history.covered_until_s)
                .collect(),
        ));
        let (times, temperatures): (Vec<Vec<f64>>, Vec<Vec<f64>>) = histories
            .into_iter()
            .map(|history| (history.time_s, history.temperature_k))
            .unzip();
        result.push(("thermocouple_time_s".to_string(), Published::Lists(times)));
        result.push((
            "thermocouple_temperature_k".to_string(),
            Published::Lists(temperatures),
        ));
    }
    // The solver was asked for the profile whatever the request said.
    let (profile_depth_m, profile_temperature_k, profile_density) = match answer.final_profile {
        Some(profile) => (
            profile.depth_m,
            profile.temperature_k,
            profile.density_kg_per_m3,
        ),
        None => (Vec::new(), Vec::new(), Vec::new()),
    };
    if wanted.final_profile {
        result.push(numbers("final_profile_depth_m", profile_depth_m));
        result.push(numbers(
            "final_profile_temperature_k",
            profile_temperature_k.clone(),
        ));
        result.push(numbers("final_profile_density_kg_per_m3", profile_density));
    }
    Solved::Answer(Answer {
        iterations: u64::from(s.time_steps),
        result,
        invariants: vec![
            number(
                "energy_account_residual_j_per_m2",
                s.energy_account_residual_j_per_m2,
            ),
            number(
                "energy_account_tolerance_j_per_m2",
                s.energy_account_tolerance_j_per_m2,
            ),
            number(
                "face_balance_residual_j_per_m2",
                s.face_balance_residual_j_per_m2,
            ),
            number(
                "face_balance_tolerance_j_per_m2",
                s.face_balance_tolerance_j_per_m2,
            ),
            number(
                "mass_account_residual_kg_per_m2",
                s.mass_account_residual_kg_per_m2,
            ),
            number(
                "mass_account_tolerance_kg_per_m2",
                s.mass_account_tolerance_kg_per_m2,
            ),
        ],
        field: profile_temperature_k,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::solution;
    use crate::solvers::{describe_part, describe_value, describe_whole};

    const SOLVER_SOURCE: &str = include_str!("../../genesis_solvers/src/ablator_slab.rs");

    /// `(variant, doc sentence)` for every variant of `SlabReject`, read from
    /// the solver's source. An intra-doc link is written as its bare code
    /// span, and a sentence over several lines is joined by single spaces.
    fn documented_rejects() -> Vec<(String, String)> {
        let start = SOLVER_SOURCE
            .find("pub enum SlabReject {")
            .expect("SlabReject is in the solver");
        let body = &SOLVER_SOURCE[start..];
        let mut found = Vec::new();
        let mut doc = String::new();
        for line in body.lines().skip(1) {
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
                let name: String = text
                    .chars()
                    .take_while(|letter| letter.is_ascii_alphanumeric())
                    .collect();
                found.push((name, doc.replace("[`", "`").replace("`]", "`")));
                doc.clear();
            }
        }
        found
    }

    /// The refusals a request itself shows.
    const AT_THE_DOOR: [&str; 23] = [
        "ThicknessOutOfRange",
        "StartTemperatureOutOfRange",
        "HeldTemperatureOutOfRange",
        "HoldDurationOutOfRange",
        "ContinueAfterOutOfRange",
        "TooManyThermocouples",
        "ThermocoupleOutsideSlab",
        "HistoryRowsOutOfRange",
        "HistoryTimesOutOfOrder",
        "HistoryDurationOutOfRange",
        "HistoryRowsTooClose",
        "StreamPressureOutOfRange",
        "HeatTransferCoefficientOutOfRange",
        "FlowEnthalpyOutOfRange",
        "RadiativeHeatingOutOfRange",
        "SurroundingsTemperatureOutOfRange",
        "ContinueAfterWithStream",
        "UnknownMaterial",
        "StreamOxygenNotAMassFraction",
        "StreamArgonNotAMassFraction",
        "StreamOxygenAndArgonAboveOne",
        "StreamNotAirForThisMaterial",
        "FlowEnthalpyBelowWall",
    ];

    #[test]
    fn all_34_refusals_carry_the_sentence_from_the_solvers_doc_comment() {
        let documented = documented_rejects();
        assert_eq!(documented.len(), 34);
        assert_eq!(REFUSALS.len(), 34);
        for ((variant, sentence), refusal) in documented.iter().zip(REFUSALS) {
            assert_eq!(variant, refusal.name);
            assert_eq!(sentence, refusal.meaning, "{}", refusal.name);
            assert_eq!(
                refusal.sealed,
                !AT_THE_DOOR.contains(&refusal.name),
                "{}",
                refusal.name
            );
        }
        let body = describe_whole(ABLATOR_SLAB.name).expect("described");
        let refusals = body["refusals"].as_array().expect("refusals");
        // After the solver's 34 the description lists the two this entry
        // names itself, each answered with no seal.
        assert_eq!(refusals.len(), 36);
        for (refusal, own) in refusals[34..].iter().zip(DOOR_REFUSALS) {
            assert_eq!(refusal["name"], own.name);
            assert_eq!(refusal["meaning"], own.meaning);
            assert_eq!(refusal["reported_as"], "422, no seal", "{}", own.name);
        }
        assert_eq!(
            DOOR_REFUSALS.iter().map(|own| own.name).collect::<Vec<_>>(),
            ["UnknownFace", "UnknownStreamGas"]
        );
        for (refusal, listed) in refusals.iter().zip(REFUSALS) {
            assert_eq!(refusal["name"], listed.name);
            assert_eq!(refusal["meaning"], listed.meaning);
            let reported = if listed.sealed {
                "200, outcome refused, sealed"
            } else {
                "422, no seal"
            };
            assert_eq!(refusal["reported_as"], reported, "{}", listed.name);
        }
        // Every constant a sentence names can be read in the description.
        let named = &body["locked"]["constants_named_in_refusals"];
        for refusal in REFUSALS {
            for span in refusal.meaning.split('`').skip(1).step_by(2) {
                let is_a_constant = span
                    .chars()
                    .all(|letter| letter.is_ascii_uppercase() || letter.is_ascii_digit() || letter == '_');
                if is_a_constant {
                    assert!(named[span].is_number(), "{}: {span}", refusal.name);
                }
            }
        }
    }

    /// A value of each refusal, as the solver would return it.
    fn one_of_each() -> Vec<SlabReject> {
        vec![
            SlabReject::ThicknessOutOfRange { thickness_m: 1.5 },
            SlabReject::StartTemperatureOutOfRange { start_temperature_k: 10.0 },
            SlabReject::HeldTemperatureOutOfRange { held_temperature_k: 5000.0 },
            SlabReject::HoldDurationOutOfRange { hold_duration_s: 0.5 },
            SlabReject::ContinueAfterOutOfRange { continue_after_s: 3601.0 },
            SlabReject::TooManyThermocouples { count: 9 },
            SlabReject::ThermocoupleOutsideSlab { index: 1, depth_m: 0.2 },
            SlabReject::StepNotConverged { time_s: 1.5, passes: 24 },
            SlabReject::NonFiniteValue { time_s: 1.5 },
            SlabReject::TemperatureOutOfBounds { time_s: 1.5, depth_m: 0.001, temperature_k: 4000.0 },
            SlabReject::DecompositionRanBackwards { time_s: 1.5, depth_m: 0.001 },
            SlabReject::MassAccountBroken {
                time_s: 1.5,
                solid_mass_lost_kg_per_m2: 0.25,
                gas_mass_out_kg_per_m2: 0.5,
            },
            SlabReject::EnergyAccountBroken {
                time_s: 1.5,
                residual_j_per_m2: 2.0,
                tolerance_j_per_m2: 1.0,
            },
            SlabReject::HistoryRowsOutOfRange { rows: 257 },
            SlabReject::HistoryTimesOutOfOrder { row: 2, time_s: 1.0 },
            SlabReject::HistoryDurationOutOfRange { duration_s: 3601.0 },
            SlabReject::HistoryRowsTooClose { row: 1, span_s: 1.0e-300 },
            SlabReject::StreamPressureOutOfRange {
                row: 0,
                pressure_pa: 0.5,
                lowest_pressure_pa: 1.01325,
                highest_pressure_pa: 101_325.0,
            },
            SlabReject::HeatTransferCoefficientOutOfRange {
                row: 0,
                heat_transfer_coefficient_kg_per_m2_s: -2.0,
            },
            SlabReject::FlowEnthalpyOutOfRange { row: 0, flow_enthalpy_j_per_kg: -2.0e8 },
            SlabReject::RadiativeHeatingOutOfRange { row: 0, radiative_heating_w_per_m2: -2.0e7 },
            SlabReject::SurroundingsTemperatureOutOfRange {
                surroundings_temperature_k: 5000.0,
                highest_k: 3975.0,
            },
            SlabReject::ContinueAfterWithStream { continue_after_s: 1.0 },
            SlabReject::FaceBeyondSurfaceTable { time_s: 1.5, face_temperature_k: 3900.0 },
            SlabReject::FaceMovedBackward { time_s: 1.5 },
            SlabReject::UnknownMaterial { name: "PICA".to_string() },
            SlabReject::StreamOxygenNotAMassFraction { oxygen_mass_fraction: 1.5 },
            SlabReject::StreamArgonNotAMassFraction { argon_mass_fraction: -0.6 },
            SlabReject::StreamOxygenAndArgonAboveOne {
                oxygen_mass_fraction: 0.6,
                argon_mass_fraction: 0.5,
            },
            SlabReject::StreamNotAirForThisMaterial {
                stream_gas: StreamGas::air_with_argon(0.1),
            },
            SlabReject::SurfaceChemistryNotConverged {
                pressure_pa: 2300.0,
                blowing: 0.1,
                temperature_k: 400.0,
            },
            SlabReject::SurfaceBundleUnreadable { why: "short".to_string() },
            SlabReject::FaceOutranItsCell { time_s: 1.5 },
            SlabReject::FlowEnthalpyBelowWall {
                row: 1,
                flow_enthalpy_j_per_kg: 1600.0,
                wall_enthalpy_j_per_kg: 1871.0,
                wall_temperature_k: 300.0,
            },
        ]
    }

    /// Every refusal the solver has is one the entry lists, under the name
    /// the solver gives it. A refusal the solver names once it is computing
    /// is sealed under that name with no result, and the seal binds it. A
    /// refusal the request itself shows is never sealed, whoever names it.
    #[test]
    fn a_refusal_is_sealed_only_when_the_request_does_not_show_it() {
        let rejects = one_of_each();
        assert_eq!(rejects.len(), REFUSALS.len());
        let request = json!({ "solver": "ablator_slab", "parameters": example() });
        let mut seals = Vec::new();
        for (reject, listed) in rejects.iter().zip(REFUSALS) {
            let name = reject.name();
            assert_eq!(name, listed.name);
            let solved = solved(Err(reject.clone()), None, &Wanted::default());
            match solution::seal(&ABLATOR_SLAB, request.clone(), solved) {
                Ok(response) => {
                    assert!(listed.sealed, "{name} was sealed");
                    let body: Value =
                        serde_json::from_str(&serde_json::to_string(&response).unwrap()).unwrap();
                    assert_eq!(body["outcome"], "refused");
                    assert_eq!(body["reject"], name);
                    assert_eq!(
                        body["reject_detail"],
                        format!(
                            "Solver Refusal: ablator_slab did not return a solved slab ({name}). {reject}."
                        )
                    );
                    assert!(body["iterations"].is_null());
                    assert!(body["result"].is_null());
                    assert!(body["invariants"].is_null());
                    assert!(body["field_sha256"].is_null());
                    let report = solution::check(&body).unwrap_or_else(|_| panic!("{name} parses"));
                    assert!(report.holds, "{name} {:?}", report.cause);
                    let mut renamed = body.clone();
                    renamed["reject"] = json!("ThicknessOutOfRange");
                    let report = solution::check(&renamed).unwrap_or_else(|_| panic!("{name} parses"));
                    assert_eq!(report.cause.as_deref(), Some("outcome"), "{name}");
                    seals.push(body["solution_seal"].as_str().unwrap().to_string());
                }
                Err(message) => {
                    assert!(!listed.sealed, "{name} was not sealed: {message}");
                    assert_eq!(message, format!("Physical Violation: {reject} ({name})."));
                }
            }
        }
        seals.sort();
        seals.dedup();
        assert_eq!(seals.len(), 11, "each sealed refusal has its own seal");
    }

    fn held(thickness_m: f64, start_k: f64, held_k: f64, hold_s: f64, after_s: f64, depths: &[f64]) -> SlabQuestion {
        SlabQuestion {
            thickness_m,
            start_temperature_k: start_k,
            surface: SurfaceCondition::HeldTemperature {
                held_temperature_k: held_k,
                hold_duration_s: hold_s,
            },
            continue_after_s: after_s,
            thermocouple_depths_m: depths.to_vec(),
        }
    }

    fn row(time_s: f64, pressure_pa: f64, coefficient: f64, enthalpy: f64, radiative: f64) -> StreamRow {
        StreamRow {
            time_s,
            pressure_pa,
            heat_transfer_coefficient_kg_per_m2_s: coefficient,
            flow_enthalpy_j_per_kg: enthalpy,
            radiative_heating_w_per_m2: radiative,
        }
    }

    /// A stream question inside the arc-jet envelope compared with
    /// measurement when its rows are [`plain_rows`]: 0.06 m from 300 K.
    fn stream(history: Vec<StreamRow>, surroundings_k: f64, after_s: f64, depths: &[f64]) -> SlabQuestion {
        SlabQuestion {
            thickness_m: 0.06,
            start_temperature_k: 300.0,
            surface: SurfaceCondition::GasStream {
                history,
                surroundings_temperature_k: surroundings_k,
                face_wears: true,
                stream_gas: StreamGas::AIR,
            },
            continue_after_s: after_s,
            thermocouple_depths_m: depths.to_vec(),
        }
    }

    fn plain_rows() -> Vec<StreamRow> {
        vec![
            row(0.0, 20_000.0, 0.0, 2.5e7, 0.0),
            row(0.5, 20_000.0, 0.17, 2.5e7, 0.0),
            row(2.0, 20_000.0, 0.17, 2.5e7, 0.0),
        ]
    }

    /// One way to break each limit a question shows. Each takes a question
    /// inside every limit and puts one thing outside.
    fn ways_to_break_a_held_question() -> Vec<(&'static str, fn(&mut SlabQuestion))> {
        fn held_of(q: &mut SlabQuestion) -> (&mut f64, &mut f64) {
            match &mut q.surface {
                SurfaceCondition::HeldTemperature {
                    held_temperature_k,
                    hold_duration_s,
                } => (held_temperature_k, hold_duration_s),
                SurfaceCondition::GasStream { .. } => unreachable!("a held question"),
            }
        }
        vec![
            ("thin", |q| q.thickness_m = 1.9e-4),
            ("thick", |q| q.thickness_m = 1.01),
            ("cold start", |q| q.start_temperature_k = 259.0),
            ("warm start", |q| q.start_temperature_k = 801.0),
            ("held low", |q| *held_of(q).0 = 259.0),
            ("held high", |q| *held_of(q).0 = 3301.0),
            ("short hold", |q| *held_of(q).1 = 0.5),
            ("long hold", |q| *held_of(q).1 = 3601.0),
            ("negative after", |q| q.continue_after_s = -1.0),
            ("long after", |q| q.continue_after_s = 3601.0),
            ("nine thermocouples", |q| q.thermocouple_depths_m = vec![0.001; 9]),
            ("a thermocouple behind the back", |q| q.thermocouple_depths_m = vec![0.001, 0.2]),
            ("a thermocouple in front of the face", |q| q.thermocouple_depths_m = vec![-0.001]),
        ]
    }

    fn ways_to_break_a_stream_question() -> Vec<(&'static str, fn(&mut SlabQuestion))> {
        fn rows_of(q: &mut SlabQuestion) -> &mut Vec<StreamRow> {
            match &mut q.surface {
                SurfaceCondition::GasStream { history, .. } => history,
                SurfaceCondition::HeldTemperature { .. } => unreachable!("a stream question"),
            }
        }
        fn gas_of(q: &mut SlabQuestion) -> &mut StreamGas {
            match &mut q.surface {
                SurfaceCondition::GasStream { stream_gas, .. } => stream_gas,
                SurfaceCondition::HeldTemperature { .. } => unreachable!("a stream question"),
            }
        }
        fn surroundings_of(q: &mut SlabQuestion) -> &mut f64 {
            match &mut q.surface {
                SurfaceCondition::GasStream {
                    surroundings_temperature_k,
                    ..
                } => surroundings_temperature_k,
                SurfaceCondition::HeldTemperature { .. } => unreachable!("a stream question"),
            }
        }
        vec![
            ("thin", |q| q.thickness_m = 1.9e-4),
            ("warm start", |q| q.start_temperature_k = 801.0),
            ("one row", |q| rows_of(q).truncate(1)),
            ("257 rows", |q| {
                let rows = rows_of(q);
                while rows.len() < 257 {
                    let last = rows[rows.len() - 1];
                    rows.push(StreamRow { time_s: last.time_s + 1.0, ..last });
                }
            }),
            ("first row not at zero", |q| {
                if let Some(row) = rows_of(q).get_mut(0) {
                    row.time_s = 0.25;
                }
            }),
            ("a row not later", |q| {
                if let Some(row) = rows_of(q).get_mut(2) {
                    row.time_s = 0.5;
                }
            }),
            ("a row earlier", |q| {
                if let Some(row) = rows_of(q).get_mut(1) {
                    row.time_s = -0.5;
                }
            }),
            ("a row a tenth of a millisecond after the one before", |q| {
                if let Some(row) = rows_of(q).get_mut(1) {
                    row.time_s = 1.0e-4;
                }
            }),
            ("pressure low in row 0", |q| {
                if let Some(row) = rows_of(q).get_mut(0) {
                    row.pressure_pa = 0.5;
                }
            }),
            ("pressure high in row 2", |q| {
                if let Some(row) = rows_of(q).get_mut(2) {
                    row.pressure_pa = 101_326.0;
                }
            }),
            ("coefficient negative in row 1", |q| {
                if let Some(row) = rows_of(q).get_mut(1) {
                    row.heat_transfer_coefficient_kg_per_m2_s = -0.1;
                }
            }),
            ("enthalpy negative in row 2", |q| {
                if let Some(row) = rows_of(q).get_mut(2) {
                    row.flow_enthalpy_j_per_kg = -1.0;
                }
            }),
            ("radiative negative in row 2", |q| {
                if let Some(row) = rows_of(q).get_mut(2) {
                    row.radiative_heating_w_per_m2 = -1.0;
                }
            }),
            ("too short", |q| {
                for (index, row) in rows_of(q).iter_mut().enumerate() {
                    row.time_s = 0.25 * index as f64;
                }
            }),
            ("too long", |q| {
                if let Some(row) = rows_of(q).get_mut(2) {
                    row.time_s = 3601.0;
                }
            }),
            ("oxygen below zero", |q| gas_of(q).oxygen_mass_fraction = -0.001),
            ("oxygen above one", |q| gas_of(q).oxygen_mass_fraction = 1.01),
            ("argon below zero", |q| gas_of(q).argon_mass_fraction = -0.001),
            ("argon above one", |q| gas_of(q).argon_mass_fraction = 1.01),
            ("oxygen and argon above one", |q| *gas_of(q) = StreamGas { oxygen_mass_fraction: 0.6, argon_mass_fraction: 0.5 }),
            // A break for TACOT alone, whose table is for air.
            ("air with argon", |q| *gas_of(q) = StreamGas::air_with_argon(0.1)),
            ("cold surroundings", |q| *surroundings_of(q) = 249.0),
            ("surroundings above the tables", |q| *surroundings_of(q) = 3976.0),
            ("nine thermocouples", |q| q.thermocouple_depths_m = vec![0.001; 9]),
            ("a thermocouple behind the back", |q| q.thermocouple_depths_m = vec![0.2]),
            // A stream colder than the wall, for every material.
            ("enthalpy below the wall in row 1", |q| {
                if let Some(row) = rows_of(q).get_mut(1) {
                    row.flow_enthalpy_j_per_kg = 1000.0;
                }
            }),
        ]
    }

    /// The request that asks a question of a material, as a caller writes it.
    fn request_of(material: &str, question: &SlabQuestion) -> Value {
        let mut parameters = json!({
            "material": material,
            "thickness_m": question.thickness_m,
            "start_temperature_k": question.start_temperature_k,
            "thermocouple_depths_m": question.thermocouple_depths_m,
        });
        match &question.surface {
            SurfaceCondition::HeldTemperature {
                held_temperature_k,
                hold_duration_s,
            } => {
                parameters["face"] = json!(FACE_HELD);
                parameters["held_temperature_k"] = json!(held_temperature_k);
                parameters["hold_duration_s"] = json!(hold_duration_s);
                parameters["continue_after_s"] = json!(question.continue_after_s);
            }
            SurfaceCondition::GasStream {
                history,
                surroundings_temperature_k,
                face_wears,
                stream_gas,
            } => {
                assert_eq!(question.continue_after_s, 0.0, "a request cannot say otherwise");
                if stream_gas.is_air() {
                    parameters["stream_gas"] = json!(GAS_AIR);
                } else {
                    parameters["stream_gas"] = json!(GAS_MIXTURE);
                    parameters["stream_oxygen_mass_fraction"] = json!(stream_gas.oxygen_mass_fraction);
                    parameters["stream_argon_mass_fraction"] = json!(stream_gas.argon_mass_fraction);
                }
                let column = |pick: fn(&StreamRow) -> f64| -> Vec<f64> {
                    history.iter().map(pick).collect()
                };
                parameters["face"] = json!(FACE_STREAM);
                parameters["history_time_s"] = json!(column(|row| row.time_s));
                parameters["history_pressure_pa"] = json!(column(|row| row.pressure_pa));
                parameters["history_heat_transfer_coefficient_kg_per_m2_s"] =
                    json!(column(|row| row.heat_transfer_coefficient_kg_per_m2_s));
                parameters["history_flow_enthalpy_j_per_kg"] =
                    json!(column(|row| row.flow_enthalpy_j_per_kg));
                parameters["history_radiative_heating_w_per_m2"] =
                    json!(column(|row| row.radiative_heating_w_per_m2));
                parameters["surroundings_temperature_k"] = json!(surroundings_temperature_k);
                parameters["face_wears"] = json!(face_wears);
            }
        }
        for flag in [ASK_FACE, ASK_THERMOCOUPLES, ASK_PROFILE] {
            parameters[flag] = json!(false);
        }
        json!({ "solver": "ablator_slab", "parameters": parameters })
    }

    /// What the gateway says of a request before a permit is taken: nothing,
    /// or the sentence it refuses with.
    fn the_gateway_says(request: &Value) -> Option<String> {
        solution::prepare(request)
            .err()
            .map(|error| error.message().to_string())
    }

    /// The gateway refuses exactly what the solver's `check_question`
    /// refuses, and with its sentence and its name: for a question inside
    /// every limit, for every way of breaking one limit, and for every two
    /// ways together in either order, on each material. So a request that
    /// breaks two limits is named for the one the solver names, and the
    /// gateway holds no copy of any limit that could drift from the solver's.
    #[test]
    fn the_gateway_refuses_exactly_what_check_question_refuses() {
        let mut compared = 0;
        let mut names = std::collections::BTreeSet::new();
        for material in MATERIALS {
            let bases = [
                (held(0.02, 300.0, 1644.0, 2.0, 0.0, &[0.001]), ways_to_break_a_held_question()),
                (stream(plain_rows(), 300.0, 0.0, &[0.001]), ways_to_break_a_stream_question()),
            ];
            for (base, ways) in &bases {
                // A held face on a PICA model was never compared with
                // measurement, and is taken as on TACOT.
                assert_eq!(slab::check_question(material, base), Ok(()));
                assert_eq!(the_gateway_says(&request_of(material, base)), None);
                let mut questions = Vec::new();
                for (first_label, first) in ways {
                    let mut once = base.clone();
                    first(&mut once);
                    questions.push((format!("{first_label}"), once.clone()));
                    for (second_label, second) in ways {
                        let mut twice = once.clone();
                        second(&mut twice);
                        let mut reversed = base.clone();
                        second(&mut reversed);
                        first(&mut reversed);
                        questions.push((format!("{first_label}, then {second_label}"), twice));
                        questions.push((format!("{second_label}, then {first_label}"), reversed));
                    }
                }
                for (label, question) in questions {
                    // A stream that is not air breaks nothing for a PICA
                    // model; then neither the solver nor the gateway refuses.
                    let reject = match slab::check_question(material, &question) {
                        Ok(()) => {
                            assert_eq!(
                                the_gateway_says(&request_of(material, &question)),
                                None,
                                "{material}: {label}"
                            );
                            assert!(*material != slab::MATERIAL_TACOT, "{label}");
                            assert!(label.contains("air with argon"), "{material}: {label}");
                            continue;
                        }
                        Err(reject) => reject,
                    };
                    assert_eq!(
                        the_gateway_says(&request_of(material, &question)),
                        Some(format!("Physical Violation: {reject} ({}).", reject.name())),
                        "{material}: {label}"
                    );
                    // And it is what the solver's answer would refuse with.
                    assert_eq!(
                        slab::answer(material, &question, &Wanted::default()).err(),
                        Some(reject.clone()),
                        "{material}: {label}"
                    );
                    names.insert(reject.name());
                    compared += 1;
                }
            }
        }
        assert!(compared > 6000, "{compared}");
        // Air is a word, not a number to paste. TACOT takes air alone, and a
        // mixture that is nearly air is not air: it is refused by name.
        let air = stream(plain_rows(), 300.0, 0.0, &[]);
        let request = request_of(slab::MATERIAL_TACOT, &air);
        assert_eq!(request["parameters"]["stream_gas"], "air");
        assert!(request["parameters"].get("stream_oxygen_mass_fraction").is_none());
        assert_eq!(the_gateway_says(&request), None);
        let mut nearly_air = request.clone();
        nearly_air["parameters"]["stream_gas"] = json!("mixture");
        nearly_air["parameters"]["stream_oxygen_mass_fraction"] = json!(0.2329);
        nearly_air["parameters"]["stream_argon_mass_fraction"] = json!(0);
        assert_eq!(
            the_gateway_says(&nearly_air),
            Some("Physical Violation: the stream has oxygen mass fraction 0.2329 and argon mass fraction 0; this material's surface table is for air alone (0.23291751145757963 and 0) (StreamNotAirForThisMaterial).".to_string())
        );
        nearly_air["parameters"]["material"] = json!(slab::MATERIAL_PICA_SERVED);
        assert_eq!(the_gateway_says(&nearly_air), None);
        // A mixture with exactly air's numbers is air to the solver, so
        // TACOT takes it; the request still says `mixture`.
        let mut airs_numbers = request.clone();
        airs_numbers["parameters"]["stream_gas"] = json!("mixture");
        airs_numbers["parameters"]["stream_oxygen_mass_fraction"] = json!(slab::AIR_OXYGEN_MASS_FRACTION);
        airs_numbers["parameters"]["stream_argon_mass_fraction"] = json!(0.0);
        assert_eq!(the_gateway_says(&airs_numbers), None);
        // Air with a fraction given is refused by name, and so is a mixture
        // with a fraction missing, and a gas for a held face.
        let mut air_with_a_fraction = request.clone();
        air_with_a_fraction["parameters"]["stream_oxygen_mass_fraction"] = json!(slab::AIR_OXYGEN_MASS_FRACTION);
        assert_eq!(
            the_gateway_says(&air_with_a_fraction),
            Some("Physical Violation: 'stream_oxygen_mass_fraction' is an input only when 'stream_gas' is \"mixture\"; this request has 'stream_gas' \"air\" (InputNotWithChoice).".to_string())
        );
        let mut air_with_argon = request.clone();
        air_with_argon["parameters"]["stream_argon_mass_fraction"] = json!(0);
        assert_eq!(
            the_gateway_says(&air_with_argon),
            Some("Physical Violation: 'stream_argon_mass_fraction' is an input only when 'stream_gas' is \"mixture\"; this request has 'stream_gas' \"air\" (InputNotWithChoice).".to_string())
        );
        let mut half_a_mixture = nearly_air.clone();
        half_a_mixture["parameters"].as_object_mut().unwrap().remove("stream_argon_mass_fraction");
        assert_eq!(
            the_gateway_says(&half_a_mixture),
            Some("Missing required parameter: 'stream_argon_mass_fraction' (InputMissing).".to_string())
        );
        let mut no_gas = request.clone();
        no_gas["parameters"].as_object_mut().unwrap().remove("stream_gas");
        assert_eq!(the_gateway_says(&no_gas), Some("Missing required parameter: 'stream_gas' (InputMissing).".to_string()));
        let mut held_with_a_gas = request_of(slab::MATERIAL_TACOT, &held(0.02, 300.0, 1644.0, 2.0, 0.0, &[]));
        held_with_a_gas["parameters"]["stream_gas"] = json!("air");
        assert_eq!(
            the_gateway_says(&held_with_a_gas),
            Some("Physical Violation: 'stream_gas' is an input only when 'face' is \"gas_stream\"; this request has 'face' \"held_temperature\" (InputNotWithChoice).".to_string())
        );
        let mut held_with_a_fraction = request_of(slab::MATERIAL_TACOT, &held(0.02, 300.0, 1644.0, 2.0, 0.0, &[]));
        held_with_a_fraction["parameters"]["stream_oxygen_mass_fraction"] = json!(0.2);
        assert_eq!(
            the_gateway_says(&held_with_a_fraction),
            Some("Physical Violation: 'stream_oxygen_mass_fraction' is an input only when 'stream_gas' is \"mixture\"; this request has no 'stream_gas' (InputNotWithChoice).".to_string())
        );
        // Every refusal a request can show was reached, but for two: the
        // material, which the field list asks first, and a time after a gas
        // stream, which is not an input.
        let mut expected: std::collections::BTreeSet<&str> = AT_THE_DOOR.into_iter().collect();
        expected.remove("UnknownMaterial");
        expected.remove("ContinueAfterWithStream");
        assert_eq!(names, expected);

        // A material the solver does not hold is refused before anything
        // else, under the solver's name for it, with the names the solver
        // lists.
        let mut request = request_of("PICA", &held(1.5, 10.0, 1644.0, 2.0, 0.0, &[]));
        assert_eq!(
            slab::check_question("PICA", &held(1.5, 10.0, 1644.0, 2.0, 0.0, &[])),
            Err(SlabReject::UnknownMaterial { name: "PICA".to_string() })
        );
        assert_eq!(
            the_gateway_says(&request),
            Some("Physical Violation: 'material' must be one of \"TACOT 3.0\", \"PICA, open record, version 5\", \"PICA, open record, version 4\", \"PICA, open record, version 3\", \"PICA, open record\"; this request has \"PICA\" (UnknownMaterial).".to_string())
        );
        for material in slab::MATERIAL_NAMES {
            request["parameters"]["material"] = json!(material);
            assert_eq!(
                the_gateway_says(&request),
                Some("Physical Violation: thickness 1.5 m is outside 0.0002 to 1 m: below 0.0002 m the grid holds no more than 8 cells, and 1 m is a size cap on the request (ThicknessOutOfRange).".to_string())
            );
        }
    }

    /// The wall pressure a material takes is the solver's: the bounds the
    /// field list publishes are the widest of the solver's ranges, the
    /// description gives each material's own, and a pressure just outside a
    /// material's range is refused in the solver's sentence, which states
    /// that material's range.
    #[test]
    fn the_pressure_range_of_each_material_is_the_solvers_own() {
        assert_eq!(MATERIALS, slab::MATERIAL_NAMES);
        assert_eq!(slab::MATERIAL_PICA_SERVED, "PICA, open record, version 5");
        let ranges: Vec<(f64, f64)> = MATERIALS
            .iter()
            .map(|material| slab::stream_pressure_range_pa(material).expect("a material"))
            .collect();
        let widest = (
            ranges.iter().map(|range| range.0).fold(f64::INFINITY, f64::min),
            ranges.iter().map(|range| range.1).fold(f64::NEG_INFINITY, f64::max),
        );
        let column = ABLATOR_SLAB
            .inputs
            .iter()
            .find(|input| input.name == "history_pressure_pa")
            .expect("listed");
        match column.kind {
            Kind::Numbers { minimum, maximum, .. } => assert_eq!((minimum, maximum), widest),
            _ => panic!("the pressure is a column"),
        }
        let locked = describe_whole("ablator_slab").unwrap()["locked"]["surface_table_pressure_pa"].clone();
        assert_eq!(locked.as_object().unwrap().len(), MATERIALS.len());
        for (material, (lowest, highest)) in MATERIALS.iter().zip(&ranges) {
            assert_eq!(locked[material], json!([lowest, highest]), "{material}");
            for (inside, outside) in [(*lowest, lowest.next_down()), (*highest, highest.next_up())] {
                let mut rows = plain_rows();
                rows[1].pressure_pa = inside;
                let question = stream(rows.clone(), 300.0, 0.0, &[]);
                let request = request_of(material, &question);
                // The ends of a PICA surface table lie outside every
                // envelope compared with measurement, and are taken.
                assert_eq!(the_gateway_says(&request), None, "{material} at {inside} Pa");
                rows[1].pressure_pa = outside;
                let request = request_of(material, &stream(rows, 300.0, 0.0, &[]));
                assert_eq!(
                    the_gateway_says(&request),
                    Some(format!(
                        "Physical Violation: row 1 of the heating history has pressure {outside} Pa, outside this material's surface table, {lowest} to {highest} Pa (StreamPressureOutOfRange)."
                    )),
                    "{material} at {outside} Pa"
                );
            }
        }
        assert_eq!(ranges[0], (101.325, 101_325.0), "TACOT 3.0");
        for range in &ranges[1..] {
            assert_eq!(*range, (1.01325, 101_325.0), "a PICA model");
        }
    }

    /// The numbers written into the sentences of the field list are the
    /// solver's constants.
    #[test]
    fn the_numbers_in_the_sentences_are_the_solvers_constants() {
        let bound = |name: &str| {
            ABLATOR_SLAB
                .inputs
                .iter()
                .find(|input| input.name == name)
                .unwrap_or_else(|| panic!("{name}"))
                .bound
        };
        assert_eq!(slab::SURFACE_RAMP_S, 0.1);
        assert!(bound("held_temperature_k").contains("over 0.1 s"));
        assert_eq!((slab::HISTORY_DURATION_MIN_S, slab::HISTORY_DURATION_MAX_S), (1.0, 3600.0));
        assert!(bound("history_time_s").contains("the last from 1 s to an hour, a size cap on the request"));
        assert_eq!(slab::HISTORY_ROW_SPAN_MIN_S, 0.001);
        assert!(bound("history_time_s").contains("each 0.001 s or more after the one before"));
        assert!(bound("history_time_s").contains("or it is refused as HistoryRowsTooClose (rows written 1 and 1.001 s are taken)"));
        // The sentence of a row too close is the solver's, with its name.
        let close = stream(
            vec![row(0.0, 20_000.0, 0.0, 2.5e7, 0.0), row(1.0e-300, 20_000.0, 0.17, 2.5e7, 0.0), row(10.0, 20_000.0, 0.17, 2.5e7, 0.0)],
            300.0,
            0.0,
            &[],
        );
        assert_eq!(
            the_gateway_says(&request_of(slab::MATERIAL_TACOT, &close)),
            Some("Physical Violation: row 1 of the heating history is 1e-300 s after the row before it; the least time between two rows is 0.001 s (HistoryRowsTooClose).".to_string())
        );
        // Rows written a millisecond apart are taken, though 1 and 1.001 s
        // are a hair under one apart as binary numbers.
        let apart = stream(
            vec![row(0.0, 20_000.0, 0.0, 2.5e7, 0.0), row(1.0, 20_000.0, 0.17, 2.5e7, 0.0), row(1.001, 20_000.0, 0.17, 2.5e7, 0.0), row(10.0, 20_000.0, 0.17, 2.5e7, 0.0)],
            300.0,
            0.0,
            &[],
        );
        assert!(1.001_f64 - 1.0 < slab::HISTORY_ROW_SPAN_MIN_S);
        assert_eq!(the_gateway_says(&request_of(slab::MATERIAL_TACOT, &apart)), None);
        assert_eq!(slab::PICA_STREAM_PRESSURE_MIN_PA, 1.01325);
        assert_eq!(slab::STREAM_PRESSURE_MIN_PA, 101.325);
        assert!(bound("history_pressure_pa").contains("1.01325 to 101325 Pa for a PICA model"));
        assert!(bound("history_pressure_pa").contains("101.325 to 101325 Pa for TACOT 3.0"));
        // Air is a word in a request. The number the solver holds for it is
        // in the locked constants, for a reader.
        assert_eq!(stardust_example()["stream_gas"], "air");
        assert!(stardust_example().get("stream_oxygen_mass_fraction").is_none());
        assert!(stardust_example().get("stream_argon_mass_fraction").is_none());
        let air = describe_whole("ablator_slab").unwrap()["locked"]["air"].clone();
        assert_eq!(air["oxygen_mass_fraction"], slab::AIR_OXYGEN_MASS_FRACTION);
        assert!(air["a_mixture_with_airs_numbers"].as_str().unwrap().contains("is air to the solver"));
        // The arc-jet example is the first development test by the recipe:
        // its gas is air with 27.6 percent argon, and its coefficient is the
        // cold-wall heating over the flow enthalpy less the solver's
        // cold-wall enthalpy of that gas, to the last bit.
        let test_gas = StreamGas::air_with_argon(0.276);
        let arc_jet = arc_jet_example();
        assert_eq!(arc_jet["stream_gas"], "mixture");
        assert_eq!(arc_jet["stream_oxygen_mass_fraction"], test_gas.oxygen_mass_fraction);
        assert_eq!(arc_jet["stream_argon_mass_fraction"], 0.276);
        let cold_wall = slab::stream_enthalpy_at_cold_wall_j_per_kg(&test_gas).expect("in range");
        assert_eq!((cold_wall * 1000.0).round(), 1_620_577.0, "{cold_wall}");
        let coefficient = 107.0e4 / (15.2e6 - cold_wall);
        assert_eq!(arc_jet["history_heat_transfer_coefficient_kg_per_m2_s"][1], coefficient);
        assert_eq!(coefficient, 0.07040224291374243);
        let recipe = arc_jet_recipe();
        assert_eq!(slab::ARCJET_AIR_OXYGEN_MASS_FRACTION, 0.2314);
        assert!(recipe["steps"][0].as_str().unwrap().contains("is 0.2314 times one less the argon"));
        assert!(recipe["steps"][0].as_str().unwrap().contains("'stream_gas' is \"mixture\""));
        assert_eq!(slab::COLD_WALL_TEMPERATURE_K, 300.0);
        assert!(recipe["steps"][1].as_str().unwrap().contains("at 300 K"));
        assert!(recipe["worked"].as_str().unwrap().contains("1,070,000 / (15,200,000 − 1,620.577) = 0.070402243"));
        assert_eq!((slab::HISTORY_POINTS_MAX, slab::PROFILE_POINTS_MAX), (121, 201));
        assert!(bound(ASK_FACE).contains("at most 121 points"));
        assert!(bound(ASK_THERMOCOUPLES).contains("at most 121 points"));
        assert!(bound(ASK_PROFILE).contains("at most 201 points"));
        assert!(bound("material").contains(&format!("is `{}`", slab::MATERIAL_PICA_SERVED)));
        assert!(bound("material").contains(&format!("`{}` is not PICA", slab::MATERIAL_TACOT)));
        for output in ABLATOR_SLAB.result {
            match output.name {
                "face_history_time_s" | "thermocouple_time_s" => assert!(output.note.contains("121")),
                "final_profile_depth_m" => assert!(output.note.contains("201")),
                _ => {}
            }
        }
        // The record says a PICA question outside what was compared is
        // answered and flagged, and the envelopes are the constants.
        let limits = &validation()[4];
        for name in [
            "HeldFaceNotCompared",
            "FaceNotWearingNotCompared",
            "ThicknessNotCompared",
            "StartTemperatureNotCompared",
            "SurroundingsNotCompared",
            "QuestionNotCompared",
            "TemperatureAboveCompared",
        ] {
            assert!(!limits.contains(name), "{name}");
            assert!(!REFUSALS.iter().any(|refusal| refusal.name == name), "{name}");
        }
        assert!(limits.contains("is answered with the same equations and the same accounts, and says so"));
        assert!(limits.contains("specific heat is held at its 3333 K value"));
        assert!(limits.contains(&format!("{} K", slab::COMPARED_TEMPERATURE_MAX_K)));
        assert!(limits.contains("the metre and the hour are size caps on the request"));
        for text in validation() {
            assert!(!text.contains("check"), "{text}");
            assert!(!text.contains("..="), "{text}");
        }
        let compared = describe_whole("ablator_slab").unwrap()["locked"]["compared_with_measurement"].clone();
        let every = &compared["every_compared_pica_case"];
        assert_eq!(every["table_top_k"], genesis_solvers::pica::TABLE_TOP_K);
        assert_eq!(every["temperature_max_k"], slab::COMPARED_TEMPERATURE_MAX_K);
        assert!(every.get("thickness_m").is_none(), "each envelope has its own slab");
        let arc = &compared["arc_jet"];
        assert_eq!(arc["pressure_pa"], json!([slab::ARC_JET_PRESSURE_MIN_PA, slab::ARC_JET_PRESSURE_MAX_PA]));
        assert_eq!(arc["cold_wall_heating_w_per_m2"], json!([slab::ARC_JET_HEATING_MIN_W_PER_M2, slab::ARC_JET_HEATING_MAX_W_PER_M2]));
        assert_eq!(arc["flow_enthalpy_j_per_kg"], json!([slab::ARC_JET_FLOW_ENTHALPY_MIN_J_PER_KG, slab::ARC_JET_FLOW_ENTHALPY_MAX_J_PER_KG]));
        assert_eq!(arc["heating_history_ends_by_s"], slab::ARC_JET_DURATION_MAX_S);
        let flight = &compared["stardust_flight"];
        assert_eq!(flight["pressure_pa"], json!([slab::STARDUST_PRESSURE_MIN_PA, slab::STARDUST_PRESSURE_MAX_PA]));
        assert_eq!(flight["stream_ends_by_s"], slab::STARDUST_STREAM_ENDS_BY_S);
        assert_eq!(
            (&arc["thickness_m"], &flight["thickness_m"], &flight["start_temperature_k"], &flight["surroundings_temperature_k"]),
            (&json!(0.06), &json!(0.0582), &json!(260.0), &json!(266.48))
        );
        let blocks = &compared["thermocouple_blocks"];
        assert_eq!(blocks["thickness_m"], json!([0.0224, 0.0325]));
        assert_eq!(blocks["start_temperature_k"], 303.15);
        assert_eq!(blocks["surroundings_temperature_k"], 300.0);
        assert_eq!(blocks["pressure_pa"], json!([20265.0, 45596.25]));
        assert_eq!(blocks["exposure_s"], json!([15.0, 40.0]));
        assert_eq!(blocks["heating_history_ends_by_s"], 152.0);
        assert!(validation()[1].contains("the thermocouple blocks, 0.0224 to 0.0325 m from 303.15 K"));
        // The Stardust example's own rows sit at the flight envelope's ends.
        let column = |k: usize| stardust::HISTORY.iter().map(move |row| row[k]);
        assert!(column(1).fold(f64::INFINITY, f64::min) >= slab::STARDUST_PRESSURE_MIN_PA);
        assert_eq!(column(1).fold(0.0, f64::max), slab::STARDUST_PRESSURE_MAX_PA);
        assert_eq!(column(2).fold(0.0, f64::max), slab::STARDUST_COEFFICIENT_MAX_KG_PER_M2_S);
        assert_eq!(column(3).fold(0.0, f64::max), slab::STARDUST_FLOW_ENTHALPY_MAX_J_PER_KG);
        assert_eq!(column(3).fold(f64::INFINITY, f64::min), slab::STARDUST_FLOW_ENTHALPY_MIN_J_PER_KG);
        assert_eq!(column(4).fold(0.0, f64::max), slab::STARDUST_RADIATIVE_HEATING_MAX_W_PER_M2);
        assert!(validation()[1].contains("streams with oxygen 0.1 to 0.3 and argon up to 0.276 by mass"));
        // The Stardust part says what its request holds.
        let stardust_part = PARTS
            .iter()
            .find(|part| part.key == "example_solve_stardust_near_nose")
            .expect("kept");
        assert!(stardust_part.holds.contains(&format!("{} rows", stardust::HISTORY.len())));
        assert!(stardust_part.holds.contains(slab::MATERIAL_PICA_SERVED));
        let refusals_part = PARTS.iter().find(|part| part.key == "refusals").expect("kept");
        let sealed = REFUSALS.iter().filter(|refusal| refusal.sealed).count();
        assert!(refusals_part.holds.starts_with(&format!("The {} named refusals", REFUSALS.len())));
        assert!(refusals_part.holds.contains(&format!("{} a request itself shows", REFUSALS.len() - sealed)));
        assert!(refusals_part.holds.contains(&format!("{sealed} met while computing")));
    }

    /// The description a caller reads first is short enough to read: the
    /// record, the inputs with their bounds, and two short requests that run
    /// unchanged. Everything else is one question away, by name, and nothing
    /// the whole description holds is lost.
    #[test]
    fn the_description_is_short_and_keeps_the_rest_for_asking() {
        let first = describe_value("ablator_slab").expect("described");
        let keys: Vec<&String> = first.as_object().unwrap().keys().collect();
        assert_eq!(
            keys,
            [
                "solver",
                "summary",
                "engine_version",
                "validation",
                "inputs",
                "number_rule",
                "open_connection",
                "example_solve",
                "example_solve_arc_jet_test",
                "more",
            ]
        );
        // The gateway's number rule, the same for every solver, adds about 470 characters.
        let text = serde_json::to_string(&first).unwrap();
        // The envelopes compared with measurement are stated in the record
        // since engine 0.7.0; since the walls at their edges came down, the
        // record and each input's bound also state the ranges the numerics
        // were measured on and the compared range of each input, in numbers;
        // since 0.13.1 the record also says what was seen before the lines
        // were fixed, so the comparison is not blind; since 0.14.0 rc2 its
        // open_connection also says how many of its worked solve one inline
        // answer holds where the local connection admits the engine's own 64
        // (rc2 item 6): 20,805 characters, where rc1's came to 20,497.
        assert!(text.chars().count() < 21_000, "{} characters", text.chars().count());
        let more = first["more"].as_object().expect("more");
        assert_eq!(
            more.keys().collect::<Vec<_>>(),
            [
                "how",
                "locked",
                "returns",
                "invariants",
                "convergence",
                "refusals",
                "reference",
                "example_solve_stardust_near_nose",
            ]
        );
        assert!(more["how"].as_str().unwrap().contains("describe_solver with 'solver' and 'part'"));

        let whole = describe_whole("ablator_slab").expect("described");
        let keys: Vec<&String> = whole.as_object().unwrap().keys().collect();
        assert_eq!(
            keys,
            [
                "solver",
                "summary",
                "engine_version",
                "validation",
                "inputs",
                "number_rule",
                "locked",
                "returns",
                "invariants",
                "convergence",
                "open_connection",
                "refusals",
                "reference",
                "example_solve",
                "example_solve_arc_jet_test",
                "example_solve_stardust_near_nose",
            ]
        );
        for key in more.keys().filter(|key| *key != "how") {
            let part = describe_part("ablator_slab", key).expect("a part");
            assert_eq!(part, json!({ "solver": "ablator_slab", "part": key, key: whole[key] }));
        }
        // The Stardust question with its whole history is one of them.
        let stardust = describe_part("ablator_slab", "example_solve_stardust_near_nose").unwrap();
        let request = &stardust["example_solve_stardust_near_nose"];
        assert_eq!(request["parameters"], stardust_example());
        assert_eq!(request["parameters"]["material"], slab::MATERIAL_PICA_SERVED);
        assert_eq!(request["parameters"]["history_time_s"].as_array().unwrap().len(), 136);
        // The two requests in the description run unchanged, and neither is
        // the Stardust question cut short.
        assert_eq!(first["example_solve"]["parameters"], example());
        assert_eq!(first["example_solve"]["parameters"]["material"], "TACOT 3.0");
        let stream = &first["example_solve_arc_jet_test"]["parameters"];
        assert_eq!(stream, &arc_jet_example());
        assert_eq!(stream["material"], slab::MATERIAL_PICA_SERVED);
        assert_eq!(stream["history_time_s"], json!([0.0, 0.5, 55.0]));
        assert_eq!(stream["stream_argon_mass_fraction"], 0.276);
        assert_eq!(stream["stream_gas"], "mixture");
        assert_eq!(request["parameters"]["stream_gas"], "air");
        for flag in [ASK_FACE, ASK_THERMOCOUPLES, ASK_PROFILE] {
            assert_eq!(first["example_solve"]["parameters"][flag], false);
            assert_eq!(stream[flag], false);
            assert_eq!(request["parameters"][flag], false);
        }
        // The atom is described whole, as before, with no record and no part.
        let atom = describe_value("atom_dft").unwrap();
        assert_eq!(atom, describe_whole("atom_dft").unwrap());
        assert!(atom.get("validation").is_none());
        assert!(atom.get("more").is_none());
    }

    /// The description is the field list, and its record says what the
    /// solver's header says of version five: what the model is made of, what
    /// it was compared with, how each came out, the misses, and the tests no
    /// model has seen.
    #[test]
    fn the_description_publishes_the_fields_the_refusals_and_the_record() {
        let described = describe_whole("ablator_slab").expect("described");
        let inputs = described["inputs"].as_array().expect("inputs");
        let names: Vec<&str> = inputs.iter().map(|row| row["name"].as_str().unwrap()).collect();
        assert_eq!(
            names,
            [
                "material",
                "thickness_m",
                "start_temperature_k",
                "thermocouple_depths_m",
                "face",
                "held_temperature_k",
                "hold_duration_s",
                "continue_after_s",
                "stream_gas",
                "stream_oxygen_mass_fraction",
                "stream_argon_mass_fraction",
                "history_time_s",
                "history_pressure_pa",
                "history_heat_transfer_coefficient_kg_per_m2_s",
                "history_flow_enthalpy_j_per_kg",
                "history_radiative_heating_w_per_m2",
                "surroundings_temperature_k",
                "face_wears",
                "return_face_history",
                "return_thermocouple_histories",
                "return_final_profile",
            ]
        );
        assert_eq!(
            inputs[0],
            json!({
                "name": "material", "type": "string",
                "allowed": [
                    "TACOT 3.0",
                    "PICA, open record, version 5",
                    "PICA, open record, version 4",
                    "PICA, open record, version 3",
                    "PICA, open record"
                ],
                "refusal": "UnknownMaterial", "bound": ABLATOR_SLAB.inputs[0].bound
            })
        );
        assert_eq!(
            inputs[1],
            json!({
                "name": "thickness_m", "type": "number", "minimum": 0.0002, "maximum": 1.0, "unit": "m",
                "refusal": "ThicknessOutOfRange", "bound": ABLATOR_SLAB.inputs[1].bound
            })
        );
        assert_eq!(
            inputs[3],
            json!({
                "name": "thermocouple_depths_m", "type": "array of numbers", "most": 8,
                "minimum": 0.0, "maximum": 1.0, "unit": "m",
                "refusal": "ThermocoupleOutsideSlab", "bound": ABLATOR_SLAB.inputs[3].bound
            })
        );
        assert_eq!(inputs[4]["allowed"], json!(["held_temperature", "gas_stream"]));
        assert_eq!(inputs[5]["given_only_when"], "'face' is \"held_temperature\"");
        assert_eq!(
            inputs[8],
            json!({
                // Re-pinned for 0.10.1: a gas that is not one of the two words is refused as UnknownStreamGas.
                "name": "stream_gas", "type": "string", "allowed": ["air", "mixture"], "refusal": "UnknownStreamGas",
                "given_only_when": "'face' is \"gas_stream\"",
                "bound": "What the stream is made of: `air`, with no fraction given, or `mixture`, oxygen and argon by mass as given and the rest nitrogen. `TACOT 3.0` takes `air` alone."
            })
        );
        assert_eq!(
            inputs[9],
            json!({
                "name": "stream_oxygen_mass_fraction", "type": "number", "minimum": 0.0, "maximum": 1.0,
                "unit": "by mass", "refusal": "StreamOxygenNotAMassFraction",
                "given_only_when": "'stream_gas' is \"mixture\"",
                "bound": ABLATOR_SLAB.inputs[9].bound
            })
        );
        assert_eq!(
            inputs[10],
            json!({
                "name": "stream_argon_mass_fraction", "type": "number", "minimum": 0.0, "maximum": 1.0,
                "unit": "by mass", "refusal": "StreamArgonNotAMassFraction",
                "given_only_when": "'stream_gas' is \"mixture\"",
                "bound": "Argon in a mixture by mass, 0 to 1."
            })
        );
        assert_eq!(
            inputs[12],
            json!({
                "name": "history_pressure_pa", "type": "array of numbers", "fewest": 2, "most": 256,
                "minimum": 1.01325, "maximum": 101325.0, "unit": "Pa",
                "refusal": "StreamPressureOutOfRange",
                "given_only_when": "'face' is \"gas_stream\"",
                "bound": ABLATOR_SLAB.inputs[12].bound
            })
        );
        assert_eq!(inputs[17]["name"], "face_wears");
        assert_eq!(inputs[17]["type"], "true or false");
        assert_eq!(inputs[17]["given_only_when"], "'face' is \"gas_stream\"");
        for row in &inputs[18..] {
            assert_eq!(row["type"], "true or false");
            assert!(row.get("given_only_when").is_none());
        }
        assert!(inputs[18]["bound"].as_str().unwrap().contains("Required: `false` gives the summary alone"));
        // Every bound published is the solver's constant.
        let bounds = |name: &str| {
            let row = inputs.iter().find(|row| row["name"] == name).expect("listed");
            (row["minimum"].as_f64().unwrap(), row["maximum"].as_f64().unwrap())
        };
        assert_eq!(bounds("thickness_m"), (slab::THICKNESS_MIN_M, slab::THICKNESS_MAX_M));
        // A mass fraction is 0 to 1; oxygen and argon together are at most
        // 1, which the solver's own sentence states.
        assert_eq!(bounds("stream_oxygen_mass_fraction"), (0.0, 1.0));
        assert_eq!(bounds("stream_argon_mass_fraction"), (0.0, 1.0));
        assert_eq!(
            bounds("start_temperature_k"),
            (slab::START_TEMPERATURE_MIN_K, slab::START_TEMPERATURE_MAX_K)
        );
        assert_eq!(
            bounds("held_temperature_k"),
            (slab::HELD_TEMPERATURE_MIN_K, slab::HELD_TEMPERATURE_MAX_K)
        );
        assert_eq!(bounds("hold_duration_s"), (slab::HOLD_DURATION_MIN_S, slab::HOLD_DURATION_MAX_S));
        assert_eq!(bounds("continue_after_s"), (0.0, slab::CONTINUE_AFTER_MAX_S));
        assert_eq!(bounds("history_time_s"), (0.0, slab::HISTORY_DURATION_MAX_S));
        assert_eq!(
            bounds("history_heat_transfer_coefficient_kg_per_m2_s"),
            (0.0, slab::HEAT_TRANSFER_COEFFICIENT_MAX_KG_PER_M2_S)
        );
        assert_eq!(bounds("history_flow_enthalpy_j_per_kg"), (0.0, slab::FLOW_ENTHALPY_MAX_J_PER_KG));
        assert_eq!(
            bounds("history_radiative_heating_w_per_m2"),
            (0.0, slab::RADIATIVE_HEATING_MAX_W_PER_M2)
        );
        assert_eq!(
            bounds("surroundings_temperature_k"),
            (slab::SURROUNDINGS_TEMPERATURE_MIN_K, genesis_solvers::pica::TABLE_TOP_K)
        );
        // The widest of the materials' tops; each material's own is the
        // solver's.
        assert_eq!(slab::solid_table_top_k(slab::MATERIAL_TACOT), Ok(slab::SURROUNDINGS_TEMPERATURE_MAX_K));
        assert_eq!(slab::solid_table_top_k(slab::MATERIAL_PICA_SERVED), Ok(genesis_solvers::pica::TABLE_TOP_K));

        // What comes back: 30 numbers of the summary and the account, 15
        // arrays that are asked for by name, and 6 numbers of the accounts.
        let result = described["returns"]["result"].as_array().expect("result");
        let always = result.iter().filter(|row| row.get("returned_only_when").is_none()).count();
        assert_eq!((always, result.len() - always), (30, 15));
        assert_eq!(described["returns"]["invariants"].as_array().unwrap().len(), 6);
        for row in result.iter().chain(described["returns"]["invariants"].as_array().unwrap()) {
            assert!(row["note"].as_str().is_some_and(|note| note.ends_with('.')), "{row}");
        }
        let asked_by = |name: &str| {
            let row = result.iter().find(|row| row["name"] == name).expect("listed");
            row["returned_only_when"].as_str().unwrap().to_string()
        };
        assert_eq!(asked_by("face_history_recession_m"), "'return_face_history' is true");
        assert_eq!(asked_by("thermocouple_temperature_k"), "'return_thermocouple_histories' is true");
        assert_eq!(asked_by("final_profile_density_kg_per_m3"), "'return_final_profile' is true");

        // The record, in the five parts a card gives.
        let record = described["validation"].as_object().expect("a record");
        assert_eq!(
            record.keys().collect::<Vec<_>>(),
            ["object", "data", "pass_line", "result", "limits"]
        );
        let part = |name: &str| record[name].as_str().unwrap();
        for words in [
            "`PICA, open record, version 5` is the PICA model a caller is pointed to",
            "export controlled",
            "PICA's own heat capacity and char conductivity as measured in 1997",
            "used at 0.71 of the measured curve, the one fitted number",
            "held off below 556 K",
            "computed at 23 pressures",
            "three published rules at the surface",
            "not PICA, nothing fitted",
        ] {
            assert!(part("object").contains(words), "{words}");
        }
        for words in [
            "four standard test cases",
            "thermocouples in four blocks",
            "36 arc-jet tests",
            "streams with oxygen 0.1 to 0.3 and argon up to 0.276 by mass",
            "5.7 mm near the nose (6.1 by X-ray) and 3.2 mm on the flank, and the density through each core",
            "nothing in version 5 was chosen on the flight",
            "NASA's computation, not a measurement",
        ] {
            assert!(part("data").contains(words), "{words}");
        }
        // The 31 held-back tests: run once, and how they came out by the
        // lines fixed beforehand, as the final check's result gives it.
        assert!(part("data").ends_with(HELD_BACK));
        assert_eq!(
            HELD_BACK,
            "On 2026-10-09 the model was run once on 31 further arc-jet tests that no model had seen, with its predictions fixed before the measurements were read."
        );
        assert!(!part("data").contains("unlooked-at"));
        assert!(part("pass_line").starts_with(
            "For the 31 held-back arc-jet tests the lines were fixed on 2026-10-07, before any model was run."
        ));
        for words in [
            "on at least 25 of the 31, with a mean error within 8 percent",
            "on at least 23 of the 28 that have one, with a mean error within 3.5 percent",
            "The model passes only if both do.",
        ] {
            assert!(part("pass_line").contains(words), "{words}");
        }
        assert!(part("result").contains(HELD_BACK_RESULT));
        for words in [
            "inside the line on 28 of 31 where 25 were needed, with a mean error of −2.6 percent against 8 allowed: passes",
            "within 7 percent on 24 of 28 where 23 were needed, but 4.1 percent hot on average against 3.5 allowed: misses on the average",
            "as it did on the 36 development tests (3.8)",
            "the model as a whole does not pass",
            "One test, in pure nitrogen (416 W/cm², 17.9 kPa, 120 s), was refused at the time because its stream had less oxygen than the solver then took, and counts as outside.",
            "It is answered now: 0.22 mm of wear against 1.75 mm measured, 87 percent low, outside its line of ±0.57 mm",
            "the face runs hot above about 45 kPa, most at 190 and 970 W/cm²; two long exposures in one facility miss on wear",
        ] {
            assert!(HELD_BACK_RESULT.contains(words), "{words}");
        }
        assert!(part("limits").contains(HELD_BACK_SPENT));
        assert_eq!(
            HELD_BACK_SPENT,
            "The 31 held-back tests are now spent, and 26 of them repeat a condition and exposure from the development set."
        );
        for words in [
            "0.63 percent root mean square of FIAT",
            "11.906 against 11.918 mm",
            "case 2.3 as close only with the surface table of its day",
            "31.3 K root mean square",
            "wear within 20 percent on 35 of 36, 4.9 percent low on average",
            "within 7 percent on 30 of 34, 3.8 percent high on average",
            "7.58 and 3.32 mm against 5.7 and 3.2 measured, a third high at the nose and 3 percent high on the flank",
            "at 7.4 and 9.1 mm deep against 5.6 and 9.0",
            "12.0 and 14.4 mm against 8.65 and 10.74",
            "6.45 and 3.19 mm",
            "not open to a request",
        ] {
            assert!(part("result").contains(words), "{words}");
        }
        // The nine misses of the version-five report, in its order, and
        // the tenth, the extended tables.
        let limits = part("limits");
        let mut from = 0;
        for words in [
            "Known misses. 1. The faint end of the charring is too deep",
            "3 to 4 mm deeper than the cores show and than NASA's code computes",
            "2. Wear at the Stardust nose is a third high with the solver's own heating and 13 percent high with NASA's coupled heating",
            "one number cannot stand in for a solution of the flow",
            "3. The face runs 6.8 percent hot above 700 W/cm²",
            "4. Wear is 12 percent low at 8.5 kPa and below in them",
            "5. One number is fitted, the char conductivity scale (0.71)",
            "6. The decomposition law is the stand-in's",
            "7. Above 13 kPa a cool wall still burns the decomposition gas",
            "8. A face at 800 K still burns a little char",
            "9. The decomposition gas inside the slab is read at 1 atm at every wall pressure",
            "10. From 3333 K, where TACOT's tables stop,",
            "the PICA tables are extended, not measured",
            "The 31 held-back tests are now spent",
            "The part `reference` has the rule for turning a published arc-jet test into a question.",
            "A proof shows that an answer was not changed, not that the model matches the material.",
        ] {
            let at = limits[from..].find(words).unwrap_or_else(|| panic!("{words}"));
            from += at + words.len();
        }
        // The numbers of the record are the numbers of the solver's header.
        for words in [
            "root mean square 31.3 K over 259",
            "20 percent on 35, mean −4.9 percent",
            "within 7 percent on 30 of 34, mean +3.8 percent",
            "wear 7.58 and 3.32 mm, a third high at the nose",
            "3 percent high on the flank",
            "7.4 and 9.1 mm",
            "Within 2 percent at 12.0 and 14.4 mm",
            "wear 6.45 and 3.19 mm, 13 percent high at the nose",
            "5.6 and 9.0 mm; back within 2 percent of virgin:",
            "8.65 and 10.74 mm",
            "the measured char conductivity times 0.71",
            "neither reaction below 556 K",
            "6.8 percent in the eight arc-jet",
            "12 percent on the ten arc-jet tests at",
            "31 further arc-jet tests that no model had seen, run once on",
            "Wear: inside the line on 28 of 31 where 25 were needed, mean −2.6",
            "percent on 24 of 28 where 23 were needed, mean +4.1 percent against",
            "beforehand the model as a whole does not pass.",
            "them repeat a condition and exposure of the 36.",
            "air with 7.6 to 27.6 percent argon; nitrogen with 10 or 30",
            "From engine 0.7.0 until this version the entry refused a PICA question",
            "no PICA case held its face at a",
            "Now the PICA model answers on its whole declared range,",
        ] {
            assert!(SOLVER_SOURCE.contains(words), "{words}");
        }
    }

    /// The Stardust history as the example carries it: 136 rows in five
    /// columns, rising from zero to 600 s, inside the declared range.
    #[test]
    fn the_stardust_example_is_the_recorded_history_in_five_columns() {
        let parameters = stardust_example();
        let column = |name: &str| -> Vec<f64> {
            parameters[name]
                .as_array()
                .unwrap_or_else(|| panic!("{name}"))
                .iter()
                .map(|value| value.as_f64().unwrap())
                .collect()
        };
        let time_s = column("history_time_s");
        assert_eq!(time_s.len(), 136);
        assert_eq!((time_s[0], time_s[134], time_s[135]), (0.0, 134.0, 600.0));
        assert!(time_s.windows(2).all(|pair| pair[1] > pair[0]));
        let pressure = column("history_pressure_pa");
        assert_eq!(pressure[0], 1.65787);
        assert!(pressure[0] < slab::STREAM_PRESSURE_MIN_PA, "the entry starts below TACOT's table");
        let coefficient = column("history_heat_transfer_coefficient_kg_per_m2_s");
        assert_eq!((coefficient[133], coefficient[134], coefficient[135]), (0.01411, 0.0, 0.0));
        assert_eq!(column("history_flow_enthalpy_j_per_kg")[11], 78_313_424.0);
        assert_eq!(column("history_radiative_heating_w_per_m2")[49], 740_000.0);
        for (index, row) in stardust::HISTORY.iter().enumerate() {
            assert_eq!(row[0], time_s[index]);
            assert_eq!(row[1], pressure[index]);
        }
        assert_eq!(parameters["thickness_m"], 0.0582);
        assert_eq!(parameters["start_temperature_k"], 260.0);
        assert_eq!(parameters["surroundings_temperature_k"], 266.48);
        assert_eq!(parameters["thermocouple_depths_m"], json!([0.005, 0.01, 0.02, 0.0582]));
        assert_eq!(parameters["face_wears"], true);
        // As text it is a few thousand characters.
        let text = json!({ "solver": "ablator_slab", "parameters": parameters }).to_string();
        assert!(text.len() < 8_000, "{}", text.len());
    }
}
