//! The entry for `seawater_column`: a column of seawater at rest from one
//! measured cast. The numbers come from `genesis_solvers` constants.

use genesis_solvers::seawater_column::{self as column, Cast, ColumnReject, SettledColumn};
use serde_json::{Value, json};

use crate::solvers::{
    Answer, Given, Input, Kind, Output, Published, Refusal, Refused, Sections, Shape, Solved,
    Solver,
};

/// What has been compared with measurement, and how it came out.
pub const COMPARED: &str = "Five of the ten legs not used to set the comparison have a sounding at the bottom of the cast (legs 7, 8, 9, 10 and 13; 632 stations): the median difference from the echo sounder is +4.59 m (target 10); 0.804 of the stations are within 20 m (target 0.80), two stations over the target; and the five legs' own medians are +4.30 to +4.92 m (target 15). The 0.80 target is held by two stations: the binomial standard error of the fraction is 0.016, and with leg 9, 10 or 13 left out it falls under 0.80 (0.790, 0.789, 0.798). The measure is one-sided: all three targets hold for a constant shift of the solver's depth between -14.55 m and +0.10 m, so the solver sits at the deep edge. The fraction within 20 m is governed by the roughness of the sea floor, not by the model: on flat sea floor the scatter is 2.7 m. Where the three soundings of a station agree within 20 m (488 stations) 0.891 are within 20 m. The other five legs (5, 11, 12, 14 and 15) give no sounding at the bottom of the cast. Legs 14 and 15, compared with the sounding at the start of the cast, a worse measurement (0.769 within 20 m on the compared stations), give 0.775; pooled in, the fraction would be 632 of 792, 0.798. A model of one density and one gravity (1025 kg/m³, 9.81 m/s²) gives a median of +48.17 m and 0.133 within 20 m on the same stations, outside all three targets. The earlier equation of state, EOS-80, also holds all three targets on the same stations (+3.72 m, 509 of 632), and so does a quadratic in the maximum pressure with three constants fitted on the read leg (511 of 632). The comparison separates the solver from one density and one gravity, 48 m off, and does not separate equations of state. What the physics shows is the legs' medians spreading over 0.6 m for the solver against 12 m for the quadratic, which the three targets do not measure. On the read leg (129 stations) the solver gives +2.87 m and 0.907. A second program, from its own cut of the raw files, recomputes the same numbers.";

/// One array of the cast, one number for each level.
const fn levels(
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
            fewest: column::LEVELS_MIN,
            most: column::LEVELS_MAX,
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
    Output {
        name,
        unit,
        shape: Shape::Number,
        each: &[],
        note,
        asked_by: None,
    }
}

const fn each_level(name: &'static str, unit: &'static str, note: &'static str) -> Output {
    Output {
        name,
        unit,
        shape: Shape::Numbers,
        each: &[],
        note,
        asked_by: None,
    }
}

pub const SEAWATER_COLUMN: Solver = Solver {
    name: "seawater_column",
    summary: "Depth, density and sound speed at every level of one measured seawater cast (pressure, temperature, salinity), from the TEOS-10 Gibbs function and the weight of the water above, and the depth an echo sounder reads over it; no constant is fitted.",
    inputs: &[
        Input {
            name: "latitude_deg",
            kind: Kind::Number {
                minimum: column::LATITUDE_MIN_DEG,
                maximum: column::LATITUDE_MAX_DEG,
                unit: "deg",
            },
            refusal: Some("LatitudeOutOfRange"),
            bound: "Latitude of the cast, north positive, -90 to 90 degrees; outside that it is not a latitude and is refused as LatitudeOutOfRange. It enters gravity at the sea surface and nothing else, and that formula holds at every latitude. The casts compared with measurement lie from -34.0023 to +26.4108 (Indian Ocean); there the echo-sounder depth is within a median of +4.59 m, with 0.804 of stations within 20 m. The Arctic and the Southern Ocean are answered with the same equations, outside the compared latitudes.",
            only_when: None,
            asked_by_check: true,
        },
        levels(
            "sea_pressure_dbar",
            column::SEA_PRESSURE_MIN_DBAR,
            column::SEA_PRESSURE_MAX_DBAR,
            "dbar",
            "PressureOutOfRange",
            "Sea pressure of each level, as a CTD records it: absolute pressure less one standard atmosphere. Rising from the first level to the last, the first no deeper than 20 dbar (FirstLevelTooDeep: the layer above it is weighed at the first level's water alone), no two neighbours more than 100 dbar apart (LevelGapTooWide: each layer between levels takes the mean of its two). A pressure below 0 or above 10000 dbar, the limit of the Gibbs function, is refused as PressureOutOfRange. The deepest last level compared with measurement is 6108 dbar; the solver's equations give 9712.78 m for 10000 dbar at Practical Salinity 35, 0 degC and latitude 30, against 9712.653 m from Unesco 1983.",
        ),
        levels(
            "temperature_c",
            column::TEMPERATURE_MIN_C,
            column::TEMPERATURE_MAX_C,
            "degC",
            "TemperatureOutOfRange",
            "In-situ temperature of each level on ITS-90, one for each pressure, answered from the freezing temperature of the level's own water up to 40 degC. The temperatures compared with measurement ran from 0.5088 to 31.169 degC. Two limits of the physics stop it. Above 40 degC, or below the floor of the water part, IAPWS SR7-09, is TemperatureOutOfRange: that floor is -2.65 degC at absolute pressure zero, as SR7-09 states it for pure water, so -2.6575 degC at sea pressure 0, falling by 0.000743 degC for each dbar. Below the freezing temperature of air-free seawater of the level's salinity at its pressure, where IAPWS R13-08 states the seawater function no longer holds, is TemperatureBelowFreezing: about -1.92 degC at Practical Salinity 35 and sea pressure 0.",
        ),
        levels(
            "practical_salinity",
            column::PRACTICAL_SALINITY_MIN,
            column::PRACTICAL_SALINITY_MAX,
            "PSS-78",
            "SalinityOutOfRange",
            "Practical Salinity of each level, one for each pressure. The solver takes Absolute Salinity as 35.16504/35 g/kg times this. Outside 0 to 41.8, the range of the Gibbs function, it is refused as SalinityOutOfRange. The salinities compared with measurement ran from 30.9805 to 40.0731; fresh, brackish and hypersaline water are answered with the same equations.",
        ),
        Input {
            name: "height_above_bottom_m",
            kind: Kind::Number {
                minimum: column::HEIGHT_ABOVE_BOTTOM_MIN_M,
                maximum: column::HEIGHT_ABOVE_BOTTOM_MAX_M,
                unit: "m",
            },
            refusal: Some("HeightAboveBottomOutOfRange"),
            bound: "Height of the last level above the sea floor, as an altimeter reads it. Give 0 when the question stops at the last level: the sea-floor depth and the echo are then those of the last level. The water between the last level and the sea floor is taken at the last level's sound speed. A height below 0, or above 100 m, where too much of the column would be water no level measured, is refused as HeightAboveBottomOutOfRange. The greatest height above bottom of a compared cast was 86 m.",
            only_when: None,
            asked_by_check: true,
        },
    ],
    kept_inside: "The equation of state, gravity, the tolerances and the sweep cap stay inside the solver.",
    result: &[
        one("seabed_depth_m", "m", "Depth of the last level plus the height above bottom."),
        one("echo_one_way_time_s", "s", "Time sound takes from the sea surface to the sea floor."),
        one("echo_sounder_depth_m", "m", "What an echo sounder set to 1500 m/s reads over this column: 1500 m/s times the one-way time. No transducer depth is taken off."),
        one("mean_sound_speed_m_per_s", "m/s", "Sea-floor depth over one-way time."),
        each_level("depth_m", "m", "Depth of each level below the sea surface."),
        each_level("density_kg_per_m3", "kg/m³", "Density of the water at each level, in place."),
        each_level("sound_speed_m_per_s", "m/s", "Sound speed at each level."),
    ],
    invariants: &[
        one("hydrostatic_residual", "", "Largest imbalance of any layer, as a fraction of its pressure step. In a thin layer at depth it holds the rounding of the depth and can read above the tolerance."),
        one("column_weight_residual_dbar", "dbar", "The weight of all the layers less the last level's pressure."),
    ],
    iterations: "Sweeps of the column taken. On a refusal it is the count for `ColumnNotSettled` and null otherwise.",
    field: field_words,
    answer: "a settled column",
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

/// Each `ColumnReject` variant with the sentence from its doc comment. A
/// limit the cast itself shows is answered with no seal.
const REFUSALS: &[Refusal] = &[
    Refusal {
        name: "NonFiniteValue",
        meaning: "An input or an intermediate value was NaN or infinite.",
        sealed: true,
    },
    Refusal {
        name: "LatitudeOutOfRange",
        meaning: "`latitude_deg` is outside `LATITUDE_MIN_DEG` to `LATITUDE_MAX_DEG`.",
        sealed: false,
    },
    Refusal {
        name: "LevelCountOutOfRange",
        meaning: "The cast has fewer than `LEVELS_MIN` or more than `LEVELS_MAX` levels.",
        sealed: false,
    },
    Refusal {
        name: "LevelsDoNotMatch",
        meaning: "The pressures, the temperatures and the salinities are not one for each level.",
        sealed: false,
    },
    Refusal {
        name: "PressureOutOfRange",
        meaning: "A level's sea pressure is outside `SEA_PRESSURE_MIN_DBAR` to `SEA_PRESSURE_MAX_DBAR`, the range of the Gibbs function.",
        sealed: false,
    },
    Refusal {
        name: "FirstLevelTooDeep",
        meaning: "The first level is deeper than `FIRST_LEVEL_MAX_DBAR`: the layer from the sea surface down to it has no level above it to take a mean with, so it is weighed at the first level's specific volume alone, and the solver takes that one-level layer no thicker than this.",
        sealed: false,
    },
    Refusal {
        name: "PressureNotRising",
        meaning: "A level's pressure is not above the pressure of the level before it.",
        sealed: false,
    },
    Refusal {
        name: "LevelGapTooWide",
        meaning: "Two neighbouring levels are more than `LEVEL_GAP_MAX_DBAR` apart.",
        sealed: false,
    },
    Refusal {
        name: "SalinityOutOfRange",
        meaning: "A level's Practical Salinity is outside `PRACTICAL_SALINITY_MIN` to `PRACTICAL_SALINITY_MAX`.",
        sealed: false,
    },
    Refusal {
        name: "TemperatureOutOfRange",
        meaning: "A level's temperature is above `TEMPERATURE_MAX_C` or below the lowest temperature of the water part of the Gibbs function, IAPWS SR7-09, at its pressure.",
        sealed: false,
    },
    Refusal {
        name: "TemperatureBelowFreezing",
        meaning: "A level's temperature is below the freezing temperature of air-free seawater of its salinity at its pressure, where the Gibbs function of seawater does not hold.",
        sealed: false,
    },
    Refusal {
        name: "HeightAboveBottomOutOfRange",
        meaning: "`height_above_bottom_m` is outside `HEIGHT_ABOVE_BOTTOM_MIN_M` to `HEIGHT_ABOVE_BOTTOM_MAX_M`.",
        sealed: false,
    },
    Refusal {
        name: "ColumnNotSettled",
        meaning: "The sweep cap was reached with a level still moving.",
        sealed: true,
    },
    Refusal {
        name: "HydrostaticBalanceBroken",
        meaning: "A layer of an otherwise settled column left `HYDROSTATIC_TOLERANCE` of its pressure step by more than the weight of `DEPTH_ROUNDING_SPACINGS` of the spacing of 64-bit numbers at its lower depth.",
        sealed: true,
    },
];

fn field_words() -> String {
    "SHA-384 of the level count as 8 little-endian bytes, then the settled depth of each level in metres as little-endian f64 bytes.".to_string()
}

fn sections() -> Sections {
    Sections {
        locked: json!({
            "equation_of_state": "The Gibbs function of TEOS-10: water part IAPWS SR7-09 (41 coefficients), saline part IAPWS R13-08 (64 coefficients). Specific volume is its pressure derivative, and sound speed follows from its second derivatives. Reference equations fitted by their authors to laboratory measurements.",
            "reference_salinity_g_per_kg_per_practical": column::REFERENCE_SALINITY_G_PER_KG_PER_PRACTICAL,
            "standard_atmosphere_pa": column::STANDARD_ATMOSPHERE_PA,
            "gravity_equator_m_per_s2": column::GRAVITY_EQUATOR_M_PER_S2,
            "gravity_sin2": column::GRAVITY_SIN2,
            "gravity_sin4": column::GRAVITY_SIN4,
            "gravity_rise_per_m": column::GRAVITY_RISE_PER_M,
            "gravity": "gravity_equator (1 + (gravity_sin2 + gravity_sin4 sin²φ) sin²φ) (1 + gravity_rise_per_m z), z the depth in metres.",
            "echo_sounder_sound_speed_m_per_s": column::ECHO_SOUNDER_SOUND_SPEED_M_PER_S,
            "levels_min": column::LEVELS_MIN,
            "levels_max": column::LEVELS_MAX,
            "first_level_max_dbar": column::FIRST_LEVEL_MAX_DBAR,
            "level_gap_max_dbar": column::LEVEL_GAP_MAX_DBAR,
            "temperature_min_at_zero_pressure_c": column::TEMPERATURE_MIN_AT_ZERO_PRESSURE_C,
            "temperature_min_fall_c_per_pa": column::TEMPERATURE_MIN_FALL_C_PER_PA,
            "freezing": "The freezing temperature of air-free seawater, where the chemical potential of water in seawater equals the Gibbs energy of ice Ih: IAPWS R13-08 Eq. (32), with ice from IAPWS R10-06 (2009), found by Newton steps to 1e-12 K. A level colder than it is refused; R13-08 states its ocean region from that temperature up.",
            "compared_latitude_min_deg": column::COMPARED_LATITUDE_MIN_DEG,
            "compared_latitude_max_deg": column::COMPARED_LATITUDE_MAX_DEG,
            "compared_deepest_max_dbar": column::COMPARED_DEEPEST_MAX_DBAR,
            "compared_temperature_min_c": column::COMPARED_TEMPERATURE_MIN_C,
            "compared_temperature_max_c": column::COMPARED_TEMPERATURE_MAX_C,
            "compared_salinity_min": column::COMPARED_SALINITY_MIN,
            "compared_salinity_max": column::COMPARED_SALINITY_MAX,
            "compared_height_above_bottom_max_m": column::COMPARED_HEIGHT_ABOVE_BOTTOM_MAX_M,
            "compared": "The envelope of the 761 casts compared with measurement, every level as the compared request gave it: 632 on legs 7, 8, 9, 10 and 13 of Knorr cruise 145, not used to set the comparison, and 129 on leg 6, which set it. A cast outside it is answered as a cast inside it is: by the same equations, outcome converged, with its solution_seal and the same result fields, and no flag or field marks it as outside. Inside the envelope or outside it, a cast is refused only by name, at the limits the inputs and refusals state: the range of the Gibbs function, 0 to 10000 dbar, Practical Salinity 0 to 41.8 and up to 40 degC (PressureOutOfRange, SalinityOutOfRange, TemperatureOutOfRange); the freezing temperature of the level's water (TemperatureBelowFreezing); a latitude outside -90 to 90 degrees (LatitudeOutOfRange); a last level more than 100 m above the sea floor (HeightAboveBottomOutOfRange); the count and spacing of the levels; and a column that does not settle (ColumnNotSettled, HydrostaticBalanceBroken).",
            "layer": "The layer between two levels has the mean of their specific volumes, gravity at its middle depth, and the mean of their 1/c for the echo. The layer from the sea surface to the first level has the first level's water, and the water from the last level to the sea floor the last level's sound speed. The height above bottom is given in metres, so no density enters below the last level.",
            "fitted": "Nothing.",
        }),
        invariants: json!([
            {
                "name": "hydrostatic_residual",
                "rule": "The weight of each layer on a square metre is its pressure step, to the tolerance as a fraction of the step and the weight of half the spacing of 64-bit numbers at the layer's lower depth, the most the sum of the depth above and the thickness rounds by: 4.5e-13 m at 6,000 m. In a layer of 2 dbar that rounding is 5e-13 of the step; in a layer of 0.02 dbar at 6,000 dbar it is 5e-11 of the step, so the residual of a cast with levels that close can read above the tolerance and the cast is still answered.",
                "tolerance": column::HYDROSTATIC_TOLERANCE,
                "depth_rounding_spacings": column::DEPTH_ROUNDING_SPACINGS,
                "unit": "fraction of the layer's pressure step",
                "compared": "at acceptance, on every layer",
                "refusal": "HydrostaticBalanceBroken",
            },
            {
                "name": "column_weight_residual_dbar",
                "rule": "The weight of all the layers is the last level's pressure. Published; it follows from the layers.",
                "unit": "dbar",
                "compared": "at acceptance",
            },
        ]),
        convergence: json!({
            "settle_tolerance_m": column::SETTLE_TOLERANCE_M,
            "max_sweeps": column::MAX_SWEEPS,
            "rule": "Gravity in a layer depends on how deep the layer lies, so the depths of all levels are swept until no level moves by more than the tolerance in one sweep.",
            "refusal": "ColumnNotSettled",
        }),
        reference: json!({
            "name": column::REFERENCE_NAME,
            "url": column::REFERENCE_URL,
            "url_saline_part": column::REFERENCE_URL_SALINE,
            "retrieved": column::REFERENCE_RETRIEVED,
            "tolerance": column::REFERENCE_TOLERANCE,
            "test": "iapws_published_values_of_the_water_part_and_the_saline_part",
            "covers": "The four derivatives of the Gibbs function the solver uses, for the water part at three points and for the saline part at three points, with the density and the sound speed of water, to the nine figures the tables print. The freezing temperature is held to the test values of ice Ih in IAPWS R10-06 (2009) Table 6, the chemical potential of water in R13-08 Table 8, the melting point of pure water at normal pressure, and within 0.5 mK of the TEOS-10 toolbox polynomial at seven points, in the test the_freezing_temperature_meets_the_published_values_of_ice_water_and_salt.",
            "kind": "A reference calculation: the printed test values of the IAPWS releases and TEOS-10.",
            "also": "The solver's equations give 9712.78 m for the depth of 10000 dbar in an ocean of salinity 35 and 0 degC at latitude 30, and the earlier equation of state (Unesco 1983) gives 9712.653 m, held to a quarter of a metre in the test the_standard_ocean_is_within_a_quarter_metre_of_the_unesco_depth. The served solver answers that cast; latitude 30, 10000 dbar and 0 degC are each outside the casts compared with measurement.",
        }),
    }
}

fn validation() -> [String; 5] {
    let object = "The water columns of the WOCE Indian Ocean expedition of the research vessel Knorr, cruise 145, legs 5 to 15, December 1994 to January 1996. At each station a CTD was lowered to within about ten metres of the sea floor and recorded pressure, temperature and salinity every 2 dbar; an altimeter or a pinger gave its height above the bottom; and the ship's 12 kHz echo sounder, read by eye from a paper recorder set to 1500 m/s, gave the depth under the ship.".to_string();
    let data = "Public files of the CLIVAR and Carbon Hydrographic Data Office (cchdo.ucsd.edu): the station summary and the CTD casts of each leg. Measured: the pressure, temperature and salinity of every level, the height above bottom, and the echo sounder's reading. Not measured: the density and the sound speed, which are the TEOS-10 reference equations, and gravity, which is a formula. One leg (leg 6, I09N, 130 stations) was read and the error measure set on it. The ten other legs, in other parts of the ocean, were unread when it was set.".to_string();
    let pass_line = "The error at a station is the solver's echo-sounder depth less the reading. Over all compared stations of the ten legs not used to set the comparison: the median within 10 m, and at least 0.80 of the stations within 20 m. For each of those legs with 20 or more compared stations: its median within 15 m. Medians, because the sounder's errors are one-sided: it hears the shallowest point inside its beam.".to_string();
    let limits = format!(
        "Nothing is fitted. The comparison tests the whole chain at about a fifth of a percent: pressure to depth through the density, and depth to echo time through the sound speed. It does not separate TEOS-10 from another modern equation of state: the echo depth by EOS-80 differs from this solver's by 1.12 m in the median, 27 parts in a hundred thousand. No transducer depth, tide or sea state is applied: the solver computes the echo depth from the sea surface. The cruise reports give about 5 m for the depth of the ADCP transducer; no report gives the depth of the 12 kHz sounder's transducer, and one report lists that sounder in the bow chamber. The casts compared span latitudes -34.0023 to +26.4108, the last level no deeper than {compared_deepest} dbar, temperatures 0.5088 to 31.169 degC, Practical Salinity 30.9805 to 40.0731, and the last level no more than 86 m above the sea floor. The solver answers past that envelope with the same equations, to the limits of the Gibbs function and of its method: any latitude, down to {deepest} dbar, Practical Salinity 0 to 41.8, from the freezing temperature of the level's water up to 40 degC, and up to 100 m above the sea floor. Polar water, trenches, fresh and brackish water, and hot or hypersaline water are answered, outside the compared envelope. The compared casts had 8 to 612 levels, 1 to 66 dbar apart; the solver takes 2 to 1024 levels up to 100 dbar apart, and thinning a compared cast from every 10 to every 100 dbar moved its echo depth by 0.022 m. Seawater below its freezing temperature is refused as TemperatureBelowFreezing. The water between the last level and the sea floor is taken at the last level's sound speed. Absolute Salinity is taken as Reference Salinity; the composition anomaly, at most 0.03 g/kg in the open ocean, is left out. The legs outside the fit are other places and months, not other instruments: the same ship, the same sounder, the same recorder and logging practice, and for the five compared the same CTD group as the read leg. The solver's depths are steadily deeper than the sounder's: about +3 m on flat sea floor and +4.6 m pooled, rough floor adding about 1.5 m. A transducer a few metres below the surface accounts for the sign and the size of that offset, and it is not proven. With a transducer depth of 4.27 or 5.0 m applied, 515 or 516 of the 632 stations are within 20 m. A cast whose levels lie close together at depth is answered: any pressures that rise from level to level are taken, down to two neighbouring 64-bit numbers, and no least spacing of levels is declared; the balance allows each layer the rounding of its depth, half the spacing of 64-bit numbers there. A proof confirms an answer is unchanged; how closely the model matches the sea is the comparison above.",
        deepest = column::SEA_PRESSURE_MAX_DBAR,
        compared_deepest = column::COMPARED_DEEPEST_MAX_DBAR,
    );
    [object, data, pass_line, COMPARED.to_string(), limits]
}

/// A short cast: thirteen levels from the surface to 1000 dbar, 12 m above
/// the sea floor.
fn example() -> Value {
    json!({
        "latitude_deg": -31.0,
        "sea_pressure_dbar": [0.0, 50.0, 100.0, 150.0, 200.0, 300.0, 400.0, 500.0, 600.0, 700.0, 800.0, 900.0, 1000.0],
        "temperature_c": [21.5, 20.9, 18.2, 16.1, 14.3, 11.8, 10.2, 9.0, 7.6, 6.1, 5.0, 4.2, 3.7],
        "practical_salinity": [35.6, 35.6, 35.7, 35.5, 35.3, 35.0, 34.8, 34.7, 34.55, 34.45, 34.42, 34.45, 34.5],
        "height_above_bottom_m": 12.0,
    })
}

fn cast(given: &Given) -> Result<Cast<'_>, String> {
    Ok(Cast {
        latitude_deg: given.number("latitude_deg")?,
        sea_pressure_dbar: given.numbers("sea_pressure_dbar")?,
        temperature_c: given.numbers("temperature_c")?,
        practical_salinity: given.numbers("practical_salinity")?,
        height_above_bottom_m: given.number("height_above_bottom_m")?,
    })
}

/// Every limit the cast itself shows, asked of the solver's own check before
/// a physics permit is taken: the solver's sentence and its name for it.
fn check(given: &Given) -> Result<(), String> {
    column::check_cast(&cast(given)?)
        .map_err(|reject| format!("Physical Violation: {reject} ({}).", reject.name()))
}

fn solve(given: &Given) -> Result<Solved, String> {
    Ok(solved(column::solve(&cast(given)?)))
}

/// What the solver returned, in the words every solver shares. A refusal the
/// request itself shows is answered with no seal and with the solver's
/// sentence alone, so its sentence carries the solver's name for it.
pub fn solved(returned: Result<SettledColumn, ColumnReject>) -> Solved {
    let settled = match returned {
        Ok(settled) => settled,
        Err(reject) => {
            let name = reject.name();
            let sealed = REFUSALS.iter().any(|refusal| refusal.name == name && refusal.sealed);
            return Solved::Refused(Refused {
                name,
                sentence: if sealed { reject.to_string() } else { format!("{reject} ({name})") },
                iterations: match reject {
                    ColumnReject::ColumnNotSettled { sweeps } => Some(u64::from(sweeps)),
                    _ => None,
                },
            });
        }
    };
    Solved::Answer(Answer {
        iterations: u64::from(settled.sweeps),
        result: vec![
            ("seabed_depth_m".to_string(), Published::Number(settled.seabed_depth_m)),
            ("echo_one_way_time_s".to_string(), Published::Number(settled.echo_one_way_time_s)),
            ("echo_sounder_depth_m".to_string(), Published::Number(settled.echo_sounder_depth_m)),
            (
                "mean_sound_speed_m_per_s".to_string(),
                Published::Number(settled.mean_sound_speed_m_per_s),
            ),
            ("depth_m".to_string(), Published::Numbers(settled.depth_m.clone())),
            ("density_kg_per_m3".to_string(), Published::Numbers(settled.density_kg_per_m3)),
            ("sound_speed_m_per_s".to_string(), Published::Numbers(settled.sound_speed_m_per_s)),
        ],
        invariants: vec![
            ("hydrostatic_residual".to_string(), Published::Number(settled.hydrostatic_residual)),
            (
                "column_weight_residual_dbar".to_string(),
                Published::Number(settled.column_weight_residual_dbar),
            ),
        ],
        field: settled.depth_m,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::solvers::describe_value;

    const SOLVER_SOURCE: &str = include_str!("../../genesis_solvers/src/seawater_column.rs");

    /// `(variant, doc sentence)` for every variant of `ColumnReject`, read
    /// from the solver's source. An intra-doc link is written as its bare
    /// code span.
    fn documented_rejects() -> Vec<(String, String)> {
        let start = SOLVER_SOURCE
            .find("pub enum ColumnReject {")
            .expect("ColumnReject is in the solver");
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

    #[test]
    fn every_refusal_carries_the_sentence_from_its_doc_comment() {
        let documented = documented_rejects();
        assert_eq!(documented.len(), REFUSALS.len());
        for ((variant, sentence), refusal) in documented.iter().zip(REFUSALS) {
            assert_eq!(variant, refusal.name);
            assert_eq!(sentence, refusal.meaning, "{}", refusal.name);
        }
        // A limit the cast itself shows is answered at the door; what only
        // the computation can find is sealed.
        let sealed: Vec<&str> = REFUSALS.iter().filter(|refusal| refusal.sealed).map(|refusal| refusal.name).collect();
        assert_eq!(sealed, ["NonFiniteValue", "ColumnNotSettled", "HydrostaticBalanceBroken"]);
    }

    #[test]
    fn the_reference_is_the_one_stored_in_the_solver_test() {
        let body = describe_value(SEAWATER_COLUMN.name).expect("described");
        for key in ["url", "url_saline_part"] {
            let url = body["reference"][key].as_str().expect("url");
            assert!(url.starts_with("https://www.teos-10.org/pubs/IAPWS-0"), "{url}");
            assert!(SOLVER_SOURCE.contains(&format!("<{url}>")), "{url}");
        }
        let retrieved = body["reference"]["retrieved"].as_str().expect("retrieved");
        assert!(SOLVER_SOURCE.contains(&format!("Retrieved {retrieved}")), "{retrieved}");
        let test = body["reference"]["test"].as_str().expect("test");
        assert!(SOLVER_SOURCE.contains(&format!("fn {test}()")), "{test}");
    }

    /// The description says in the list sentence that nothing is fitted, and
    /// in the record what is measured and what is a reference equation.
    #[test]
    fn the_description_says_what_is_fitted_and_what_is_measured() {
        assert!(SEAWATER_COLUMN.summary.ends_with("no constant is fitted."));
        let body = describe_value(SEAWATER_COLUMN.name).expect("described");
        assert_eq!(body["locked"]["fitted"], "Nothing.");
        let data = body["validation"]["data"].as_str().expect("data");
        assert!(data.contains("Not measured: the density and the sound speed"), "{data}");
        assert_eq!(body["validation"]["result"], COMPARED);
    }

    /// The example runs, every limit the cast shows is refused at the door
    /// with the solver's own sentence and name, and no value is moved.
    #[test]
    fn the_example_settles_and_each_limit_of_the_cast_is_named_at_the_door() {
        let request = json!({ "solver": "seawater_column", "parameters": example() });
        let given = crate::solution::read_parameters(
            &SEAWATER_COLUMN,
            request["parameters"].as_object().expect("an object"),
        )
        .unwrap_or_else(|error| panic!("{}", error.message()));
        let Solved::Answer(answer) = solve(&given).expect("runs") else {
            panic!("the example was refused");
        };
        assert!(answer.iterations >= 3);
        assert_eq!(answer.field.len(), 13);
        let depth = match &answer.result[0].1 {
            Published::Number(depth) => *depth,
            _ => panic!("a number"),
        };
        assert!(depth > 1002.0 && depth < 1006.0, "{depth}");

        let refused = |field: &str, value: Value| {
            let mut parameters = example();
            parameters[field] = value;
            match crate::solution::read_parameters(&SEAWATER_COLUMN, parameters.as_object().expect("an object")) {
                Ok(_) => panic!("{field} was not refused"),
                Err(error) => error.message().to_string(),
            }
        };
        let mut short = example()["temperature_c"].as_array().expect("an array").clone();
        short.pop();
        for (field, value, name) in [
            ("latitude_deg", json!(91.0), "LatitudeOutOfRange"),
            ("height_above_bottom_m", json!(100.5), "HeightAboveBottomOutOfRange"),
            ("temperature_c", json!([-2.0, 20.9, 18.2, 16.1, 14.3, 11.8, 10.2, 9.0, 7.6, 6.1, 5.0, 4.2, 3.7]), "TemperatureBelowFreezing"),
            ("temperature_c", Value::Array(short), "LevelsDoNotMatch"),
            ("sea_pressure_dbar", json!([0.0, 50.0, 100.0, 150.0, 200.0, 300.0, 400.0, 500.0, 600.0, 700.0, 800.0, 900.0, 1000.5]), "LevelGapTooWide"),
            ("sea_pressure_dbar", json!([0.0, 50.0, 100.0, 150.0, 200.0, 300.0, 400.0, 500.0, 600.0, 700.0, 800.0, 800.0, 900.0]), "PressureNotRising"),
            ("sea_pressure_dbar", json!([25.0, 50.0, 100.0, 150.0, 200.0, 300.0, 400.0, 500.0, 600.0, 700.0, 800.0, 900.0, 1000.0]), "FirstLevelTooDeep"),
            ("practical_salinity", json!([35.6, 35.6, 35.7, 35.5, 35.3, 35.0, 34.8, 34.7, 34.55, 34.45, 34.42, 34.45, 42.0]), "SalinityOutOfRange"),
            ("temperature_c", json!([-2.7, 20.9, 18.2, 16.1, 14.3, 11.8, 10.2, 9.0, 7.6, 6.1, 5.0, 4.2, 3.7]), "TemperatureOutOfRange"),
        ] {
            let message = refused(field, value);
            assert!(message.starts_with("Physical Violation: "), "{message}");
            assert!(message.ends_with(&format!("({name}).")), "{message}");
        }
        // Past the casts compared with measurement, where engine 0.8.0
        // refused at the door, the cast is answered.
        for (field, value) in [
            ("latitude_deg", json!(-34.1)),
            ("latitude_deg", json!(78.9)),
            ("latitude_deg", json!(-90.0)),
            ("height_above_bottom_m", json!(86.5)),
            ("height_above_bottom_m", json!(100.0)),
            ("temperature_c", json!([21.5, 20.9, 18.2, 16.1, 14.3, 11.8, 10.2, 9.0, 7.6, 6.1, 5.0, 4.2, 0.4])),
            ("temperature_c", json!([-1.8, 20.9, 18.2, 16.1, 14.3, 11.8, 10.2, 9.0, 7.6, 6.1, 5.0, 4.2, 3.7])),
            ("practical_salinity", json!([35.6, 35.6, 35.7, 35.5, 35.3, 35.0, 34.8, 34.7, 34.55, 34.45, 34.42, 34.45, 41.0])),
            ("practical_salinity", json!([0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0])),
        ] {
            let mut parameters = example();
            parameters[field] = value;
            let given = crate::solution::read_parameters(&SEAWATER_COLUMN, parameters.as_object().expect("an object"))
                .unwrap_or_else(|error| panic!("{field}: {}", error.message()));
            let Solved::Answer(answer) = solve(&given).expect("runs") else {
                panic!("{field} was refused");
            };
            let residual = match &answer.invariants[0].1 {
                Published::Number(residual) => *residual,
                _ => panic!("a number"),
            };
            assert!(residual <= column::HYDROSTATIC_TOLERANCE, "{field} {residual}");
        }
    }

    /// A refusal the computation finds is sealed with its count; one the
    /// cast shows is not sealed if the solver is somehow asked.
    #[test]
    fn a_refusal_is_put_in_the_shared_words() {
        match solved(Err(ColumnReject::ColumnNotSettled { sweeps: 50 })) {
            Solved::Refused(refused) => {
                assert_eq!(refused.name, "ColumnNotSettled");
                assert_eq!(refused.iterations, Some(50));
                assert_eq!(refused.sentence, "a level was still moving after 50 sweeps");
            }
            Solved::Answer(_) => panic!("an answer"),
        }
        match solved(Err(ColumnReject::PressureNotRising { level: 4 })) {
            Solved::Refused(refused) => {
                assert_eq!(refused.iterations, None);
                assert!(refused.sentence.ends_with("(PressureNotRising)"), "{}", refused.sentence);
            }
            Solved::Answer(_) => panic!("an answer"),
        }
    }
}
