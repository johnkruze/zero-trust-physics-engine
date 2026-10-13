//! PICA, as far as the open record goes: the second material of the slab
//! solver. Data and the origin of each number; no equations.
//!
//! PICA's own property model is export controlled. What is open is its
//! density, its conductivity at room temperature and its emissivity. The
//! rest is TACOT's, the made-up material of the same family, and is named as
//! such below. So this block is a stand-in for PICA with PICA's density and
//! conductivity level, not PICA.
//!
//! Four versions are held. Version 5 is the one a caller is pointed to.
//! Version 1 is the table below. Version 3 is version 1 with the
//! conductivity depending on pressure. Version 4 takes PICA's own measured
//! heat capacity and the shape of its measured char conductivity, and has no
//! decomposition below 556 K (`VERSION_4`). Version 5 is version 4 with
//! one change to the surface options (`SURFACE_OPTIONS_VERSION_5`). The
//! earlier versions are kept so that the recorded runs can be made again.
//!
//! | Quantity | Value | Origin |
//! |---|---|---|
//! | Virgin density | 274 kg/m³ | PICA nominal (Agrawal and others 2016; Milos and Chen 2010 use it for their arc-jet models). The recovered Stardust shield measured 272. |
//! | Char density | 215.3 kg/m³ | TACOT's char yield, 220/280, applied to 274. Not a PICA measurement: none is open. The recovered Stardust core measured 210. |
//! | Resin parts, rate constants | TACOT 3.0 | the fibre kept at 160 kg/m³ of material and the resin rescaled to the two densities, half the loss to each part |
//! | Specific heat, virgin and char | TACOT 3.0 | no open PICA values |
//! | Decomposition gas enthalpy | TACOT 3.0, 1 atm | no open PICA values |
//! | Virgin enthalpy at 298 K | −2 MJ per kg of resin, as TACOT, times this material's resin share | |
//! | Conductivity, virgin | TACOT 3.0's curve times 0.432 | PICA's measured room-temperature value over TACOT's; see [`VIRGIN_CONDUCTIVITY_SCALE`] |
//! | Conductivity, char | TACOT 3.0's curve times 0.745 | **fitted** to four arc-jet thermocouple records; see [`CHAR_CONDUCTIVITY_SCALE`] |
//! | Emissivity | 0.8 virgin (TACOT), 0.9 char (Milos and Chen 2010) | |
//! | Conductivity against pressure (from version 3 on) | virgin falls by 0.042 and char by 0.024 W/m/K between 1 atm and 0.001 atm at room temperature | Monk and others 2022, Table 3; see [`VIRGIN_DROP_1_ATM_TO_0P001_ATM_W_PER_M_K`] |
//! | Above 3333 K, where TACOT's tables stop | specific heat held; conductivity continued along its last straight line to 3975 K. Not measured; the served entry refuses a PICA run above `ablator_slab::COMPARED_TEMPERATURE_MAX_K`, the hottest compared run | see [`TABLE_TOP_K`] |
//! | Surface chemistry | the stream's gas with TACOT's decomposition gas, 23 pressures from 0.00001 to 1 atm | computed from NASA's open thermodynamic file: for air by the table code in `evidence/research/heatshield_surface_model`, locked in `surface_air_tacotgas.bin`; for any other stream of oxygen, nitrogen and argon by the same calculation in `wall_gas.rs` when a question asks for it |

use crate::ablator_slab::{GasBesideUnusedOxygen, Material, PoreGas, ResinPart, SurfaceOptions};
use crate::tacot;

/// Virgin density, kg/m³.
pub const VIRGIN_DENSITY_KG_PER_M3: f64 = 274.0;
/// Char density, kg/m³: TACOT's char yield applied to the virgin density.
pub const CHAR_DENSITY_KG_PER_M3: f64 = 274.0 * 220.0 / 280.0;
/// PICA's measured conductivity at room temperature and 1 atm, W/m/K
/// (`evidence/research/heatshield_pica_arcjet/README.md`).
pub const VIRGIN_CONDUCTIVITY_AT_ROOM_W_PER_M_K: f64 = 0.174;
pub const CHAR_CONDUCTIVITY_AT_ROOM_W_PER_M_K: f64 = 0.224;
/// What TACOT's virgin conductivity curve is multiplied by: PICA's measured
/// room-temperature value over TACOT's 0.4025 W/m/K. Not fitted.
pub const VIRGIN_CONDUCTIVITY_SCALE: f64 = VIRGIN_CONDUCTIVITY_AT_ROOM_W_PER_M_K / 0.40249965408;
/// What TACOT's char conductivity curve is multiplied by. **Fitted**: the one
/// fitted number of this block. It is the value that brings the solver
/// closest to the in-depth thermocouple records of four arc-jet blocks of
/// PICA (9B, 7B, 3B and 6A of Tran and others 1997, 400 and 580 W/cm²; 259
/// samples, each record used up to its first sample above 1500 K): root mean
/// square 27.9 K when it was fitted and 29.3 K with the surface options as
/// frozen below, against 83.8 K with the room-temperature ratio (0.5565).
/// It puts the char at 0.300 W/m/K at room temperature where 0.224 was
/// measured, so it stands for the char's conductivity when hot, not when
/// cold. A second fitted number for the virgin curve lowered the root mean
/// square by 1.5 K and was not kept.
pub const CHAR_CONDUCTIVITY_SCALE: f64 = 0.745;
/// Emissivity of virgin material (TACOT's) and of char (Milos and Chen 2010).
pub const VIRGIN_EMISSIVITY: f64 = 0.8;
pub const CHAR_EMISSIVITY: f64 = 0.9;
/// Enthalpy of formation of the resin, J per kg of resin (TACOT 3.0).
pub const RESIN_ENTHALPY_AT_298_K_J_PER_KG: f64 = -2.0e6;
/// The property tables are continued from TACOT's last temperature
/// (3333 K) to this one, the top of the decomposition-gas table, kelvin.
/// Char there is nearly pure carbon: graphite's specific heat is flat by
/// then (TACOT's char value, 2152 J/kg/K, is within 4 percent of the
/// classical limit for carbon, 2078), so it is held. Conductivity in a
/// porous char at these temperatures is carried mostly by radiation between
/// fibres and keeps rising, so its last straight line is continued. No
/// measurement stands above 3333 K; the served entry refuses a PICA run
/// above `ablator_slab::COMPARED_TEMPERATURE_MAX_K`, about 3412 K.
pub const TABLE_TOP_K: f64 = 3975.0;

/// Version 3: conductivity that depends on pressure.
///
/// Monk and others 2022 (NTRS 20210024735), Table 3, give PICA's
/// conductivity at room temperature at 1 atm and at 0.001 atm. For the lot
/// that was measured (the MEDLI2 flight lot, measured at 1, 0.1 and 0.01
/// atm) virgin falls from 0.169 to 0.127 W/m/K and char from 0.169 to
/// 0.143. For the heritage model char falls from 0.224 to 0.202. (The
/// heritage virgin row prints 0.174 and 0.520, a rise with falling pressure
/// that no porous material shows; it is taken as a misprint and not used.)
/// So the gas in the pores carries 0.042 W/m/K in virgin material and 0.022
/// to 0.026 in char: for char, the conductivity of still air, 0.026.
///
/// Size: those drops, virgin 0.042 and char 0.024 (the mean of the two
/// lots), between 1 atm and 0.001 atm at room temperature. Form: see
/// `PoreGas`. The one number the two tabulated pressures do not give is the
/// pressure at which the gas carries half: it is set from the pore size,
/// 50 micrometres (the spacing of the fibres), at which the free path of
/// air leaves the gas half its conductivity at 0.0044 atm and 300 K.
pub const VIRGIN_DROP_1_ATM_TO_0P001_ATM_W_PER_M_K: f64 = 0.042;
pub const CHAR_DROP_1_ATM_TO_0P001_ATM_W_PER_M_K: f64 = 0.024;
pub const PORE_HALF_PRESSURE_AT_300_K_PA: f64 = 0.0044 * 101325.0;
/// The virgin curve holds at 1 atm, where its room-temperature value was
/// measured. The char curve holds at 0.3 atm, between the 0.2 and 0.45 atm
/// of the arc-jet blocks its scale was fitted on.
pub const VIRGIN_REFERENCE_PRESSURE_PA: f64 = 101325.0;
pub const CHAR_REFERENCE_PRESSURE_PA: f64 = 0.3 * 101325.0;

/// Version 4: PICA's own open measurements against temperature, in place of
/// TACOT's curves, wherever they exist.
///
/// Heat capacity of PICA measured to 2,928 K (Tran and others, NASA
/// TM-110440, 1997, Fig. 7, on PICA of 240 kg/m³; digitised in
/// `evidence/research/pica_charring_data/tables/digitised/`). The report
/// does not say whether the specimen was virgin throughout, so the one curve
/// is used for virgin and for char. Heat capacity is per kilogram and is not
/// rescaled for density. Below the first point (475 K), where nothing is
/// measured, TACOT's virgin curve is used, raised by 3 percent to meet the
/// measurement there; above the last it is held. Kelvin, J/kg/K.
pub const MEASURED_SPECIFIC_HEAT: [(f64, f64); 14] = [
    (475.0, 1381.0),
    (545.0, 1434.0),
    (985.0, 1749.0),
    (1061.0, 1800.0),
    (1127.0, 1836.0),
    (1278.0, 1915.0),
    (1359.0, 1960.0),
    (1479.0, 2017.0),
    (1569.0, 2059.0),
    (1721.0, 2116.0),
    (2141.0, 2223.0),
    (2406.0, 2266.0),
    (2493.0, 2274.0),
    (2928.0, 2303.0),
];
/// Conductivity of the same PICA through its thickness, measured from
/// diffusivity at 0.001 to 0.05 atm (same report, Fig. 9 and printed p. 3:
/// 0.369 W/m/K "at temperatures below 2000 °F", 1.718 at 5000 °F). A specimen
/// in that test is char above about 800 K, so the curve is the char's. The
/// printed 0.369 is used up to 1,239 K, where the digitised points scatter
/// about it, then the digitised points, then the printed value at 3,033 K.
/// Kelvin, W/m/K, for 240 kg/m³.
pub const MEASURED_CHAR_CONDUCTIVITY: [(f64, f64); 12] = [
    (255.55555555555554, 0.369),
    (1239.0, 0.369),
    (1486.0, 0.455),
    (1556.0, 0.486),
    (1734.0, 0.58),
    (1804.0, 0.624),
    (1973.0, 0.727),
    (2088.0, 0.802),
    (2216.0, 0.89),
    (2286.0, 0.941),
    (2788.0, 1.426),
    (3033.0, 1.718),
];
/// Density of the PICA those curves were measured on, kg/m³ (15 lb/ft³).
pub const MEASURED_CURVES_DENSITY_KG_PER_M3: f64 = 240.3;
/// The pressure the measured conductivity holds at, Pa: 0.01 atm, the
/// middle of the three test pressures.
pub const MEASURED_CONDUCTIVITY_PRESSURE_PA: f64 = 1013.25;
/// The char density NASA's own runs of the two Stardust cores used, kg/m³
/// (a char yield of 0.835, about the yield measured at slow heating). It is
/// not version 4's: the yield measured at fast heating, which is how a
/// wearing face is heated, is 0.785 (thermogravimetry, NASA TM-110440), and
/// that is the 215.3 kg/m³ the block already has. With 229 the 36 arc-jet
/// tests run 9.8 percent low on wear in place of 5. Kept for studies.
pub const NASA_CORE_RUN_CHAR_DENSITY_KG_PER_M3: f64 = 229.0;
/// What version 4 multiplies the measured char conductivity by. **Fitted**,
/// to the same four arc-jet thermocouple records as version 1's scale.
/// Unscaled, the measured curve leaves the thermocouples 121 K out (root
/// mean square); with this number 31.5 K, against version 3's 30.3 K.
pub const VERSION_4_CHAR_CONDUCTIVITY_SCALE: f64 = 0.71;
/// In version 4 neither resin part decomposes below this, kelvin: the
/// starting temperature NASA's 1997 deck prints for the first reaction. The
/// slow measurements lose under 1 percent of their mass below it.
pub const VERSION_4_NO_DECOMPOSITION_BELOW_K: f64 = 555.5555555555555;

/// The surface options of the frozen model, chosen on the 36 opened arc-jet
/// tests of PICA (Milos and Chen 2010) run through this solver, and on the
/// sources named. Wear within 20 percent on 35 of 36, mean −5.0 percent;
/// face temperature within 7 percent on 30 of 34, mean +3.8 percent.
///
/// - Low-pressure rule on (Milos, Chen and Gokcen 2012): 33 of 36 against 28
///   without it.
/// - Wall-reaction limit on, as a flight's cool tail needs, with the atom
///   probability 0.9 measured on PICA's fibre preform (Driver and others
///   2010) and Park's form for molecules. The tests lose 1.5 points of wear
///   to it; with Park's atom form they lose 3.4.
/// - Evaporation limit off: the published rates differ a hundredfold and
///   the tests are slightly worse with it.
/// - Char lost as solid, 5 percent of the char consumed: NASA's practice
///   for PICA, a borrowed number; the tests' mean error goes from −9.4 to
///   −5.0 percent with it.
pub(crate) const SURFACE_OPTIONS: SurfaceOptions = SurfaceOptions {
    low_pressure_rule: true,
    oxidation_limited_by_wall_reaction: true,
    oxygen_atom_reaction_probability: Some(0.9),
    carbon_vapour_limited_by_evaporation: false,
    solid_char_loss_fraction: 0.05,
    stream_heating_supplied_separately: false,
    gas_beside_unused_oxygen: GasBesideUnusedOxygen::Burns,
    air_table_computed_afresh: false,
};

/// Version five's surface options: the same, with the decomposition gas
/// left unburnt, at low pressure, in the share of the wall gas whose oxygen
/// the wall reaction leaves unused. The low-pressure rule above already
/// says that at or below 7 kPa the gas and the stream do not react with
/// each other; versions one to four applied it only to the share of the
/// wall gas whose oxygen the wall takes. No new number.
pub(crate) const SURFACE_OPTIONS_VERSION_5: SurfaceOptions = SurfaceOptions {
    gas_beside_unused_oxygen: GasBesideUnusedOxygen::FollowsTheLowPressureRule,
    ..SURFACE_OPTIONS
};

/// The surface bundle of the frozen model: air with TACOT's decomposition gas.
pub(crate) static SURFACE_BUNDLE: &[u8] = include_bytes!("surface_air_tacotgas.bin");

/// The departures from the frozen block that a study may make.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Departures {
    pub virgin_density_kg_per_m3: f64,
    pub char_density_kg_per_m3: f64,
    pub virgin_conductivity_scale: f64,
    pub char_conductivity_scale: f64,
    /// Hold the conductivity above 3333 K instead of continuing its line.
    pub conductivity_held_above_3333_k: bool,
    pub char_emissivity: f64,
    /// Temperature below which neither resin part decomposes, kelvin, when
    /// it is higher than TACOT's own thresholds (333 K and 556 K). A
    /// bracket for studies, not a property of any material.
    pub no_decomposition_below_k: f64,
    /// Conductivity depends on pressure through the gas in the pores
    /// (version 3). The number is what the half pressure is multiplied by:
    /// 1 for 50 micrometre pores, 10 for 5 micrometre pores.
    pub pore_gas_half_pressure_scale: Option<f64>,
    /// Heat capacity and char conductivity from the 1997 measurements on
    /// PICA in place of TACOT's curves (version 4). The char conductivity
    /// scale then multiplies the measured curve.
    pub measured_curves: bool,
}

/// No departure: the frozen block.
pub(crate) const FROZEN: Departures = Departures {
    virgin_density_kg_per_m3: VIRGIN_DENSITY_KG_PER_M3,
    char_density_kg_per_m3: CHAR_DENSITY_KG_PER_M3,
    virgin_conductivity_scale: VIRGIN_CONDUCTIVITY_SCALE,
    char_conductivity_scale: CHAR_CONDUCTIVITY_SCALE,
    conductivity_held_above_3333_k: false,
    char_emissivity: CHAR_EMISSIVITY,
    no_decomposition_below_k: 0.0,
    pore_gas_half_pressure_scale: None,
    measured_curves: false,
};

/// Version 4: PICA's own open data wherever it exists. One fitted number.
pub(crate) const VERSION_4: Departures = Departures {
    char_conductivity_scale: VERSION_4_CHAR_CONDUCTIVITY_SCALE,
    no_decomposition_below_k: VERSION_4_NO_DECOMPOSITION_BELOW_K,
    pore_gas_half_pressure_scale: Some(1.0),
    measured_curves: true,
    ..FROZEN
};

/// Version 3: version 1 with the conductivity depending on pressure.
pub(crate) const VERSION_3: Departures = Departures {
    pore_gas_half_pressure_scale: Some(1.0),
    ..FROZEN
};

fn kept(values: Vec<f64>) -> &'static [f64] {
    Box::leak(values.into_boxed_slice())
}

/// The material block, built from TACOT's tables and the numbers above.
/// Each call keeps its tables for the life of the program.
pub(crate) fn material(d: &Departures) -> Material {
    let t = &tacot::TACOT;
    let last = t.solid_table_k.len() - 1;
    let span = TABLE_TOP_K - t.solid_table_k[last];
    let extended = |table: &[f64], scale: f64, continue_line: bool| {
        let mut out: Vec<f64> = table.iter().map(|v| v * scale).collect();
        let slope =
            (table[last] - table[last - 1]) / (t.solid_table_k[last] - t.solid_table_k[last - 1]);
        out.push(scale * (table[last] + if continue_line { slope * span } else { 0.0 }));
        kept(out)
    };
    if d.measured_curves {
        return measured_material(d);
    }
    let mut temperatures = t.solid_table_k.to_vec();
    temperatures.push(TABLE_TOP_K);
    let line = !d.conductivity_held_above_3333_k;
    // The fibre is kept and the resin rescaled to the two densities; each
    // part gives up half the loss, as in TACOT.
    let fibre =
        t.solid_volume_fraction * (1.0 - t.resin_volume_fraction) * t.fibre_density_kg_per_m3;
    let per_resin = t.solid_volume_fraction * t.resin_volume_fraction;
    let resin_virgin = (d.virgin_density_kg_per_m3 - fibre) / per_resin;
    let resin_char = (d.char_density_kg_per_m3 - fibre) / per_resin;
    let loss = resin_virgin - resin_char;
    let part = |from: &ResinPart, virgin: f64, char: f64| ResinPart {
        virgin_density_kg_per_m3: virgin,
        char_density_kg_per_m3: char,
        threshold_temperature_k: from.threshold_temperature_k.max(d.no_decomposition_below_k),
        ..*from
    };
    Material {
        solid_table_k: kept(temperatures),
        virgin_specific_heat_j_per_kg_k: extended(t.virgin_specific_heat_j_per_kg_k, 1.0, false),
        char_specific_heat_j_per_kg_k: extended(t.char_specific_heat_j_per_kg_k, 1.0, false),
        virgin_conductivity_w_per_m_k: extended(
            t.virgin_conductivity_w_per_m_k,
            d.virgin_conductivity_scale,
            line,
        ),
        char_conductivity_w_per_m_k: extended(
            t.char_conductivity_w_per_m_k,
            d.char_conductivity_scale,
            line,
        ),
        virgin_enthalpy_at_anchor_j_per_kg: RESIN_ENTHALPY_AT_298_K_J_PER_KG
            * (d.virgin_density_kg_per_m3 - fibre)
            / d.virgin_density_kg_per_m3,
        virgin_density_kg_per_m3: d.virgin_density_kg_per_m3,
        char_density_kg_per_m3: d.char_density_kg_per_m3,
        resin_parts: [
            part(&t.resin_parts[0], 0.5 * loss, 0.0),
            part(&t.resin_parts[1], resin_virgin - 0.5 * loss, resin_char),
        ],
        virgin_emissivity: VIRGIN_EMISSIVITY,
        char_emissivity: d.char_emissivity,
        pore_gas: d.pore_gas_half_pressure_scale.map(|scale| {
            // The tabulated drops are between 1 atm and 0.001 atm at 300 K.
            let half = PORE_HALF_PRESSURE_AT_300_K_PA * scale;
            let between = 101325.0 / (101325.0 + half) - 101.325 / (101.325 + half);
            PoreGas {
                virgin_w_per_m_k_at_300_k: VIRGIN_DROP_1_ATM_TO_0P001_ATM_W_PER_M_K / between,
                char_w_per_m_k_at_300_k: CHAR_DROP_1_ATM_TO_0P001_ATM_W_PER_M_K / between,
                half_pressure_pa_at_300_k: half,
                virgin_reference_pa: VIRGIN_REFERENCE_PRESSURE_PA,
                char_reference_pa: CHAR_REFERENCE_PRESSURE_PA,
            }
        }),
        ..*t
    }
}

/// Straight lines through `(temperature, value)` points; beyond the ends
/// the end segment's line is continued when `continue_line`, else the end
/// value is held.
fn along(points: &[(f64, f64)], at: f64, continue_below: bool, continue_above: bool) -> f64 {
    let n = points.len();
    if at <= points[0].0 && !continue_below {
        return points[0].1;
    }
    if at >= points[n - 1].0 && !continue_above {
        return points[n - 1].1;
    }
    let mut j = 0;
    while j + 2 < n && at > points[j + 1].0 {
        j += 1;
    }
    let (a, b) = (points[j], points[j + 1]);
    a.1 + (b.1 - a.1) * (at - a.0) / (b.0 - a.0)
}

/// The version 4 block: the frozen block's densities, decomposition and
/// virgin conductivity as departed, with the measured heat capacity for
/// both states and the measured conductivity for the char.
///
/// The measured conductivity is carried from 240 to 274 kg/m³ in two parts.
/// What the solid carries (the flat 0.369 W/m/K below 1,239 K) goes with the
/// amount of solid, so it is multiplied by 274/240. What radiation between
/// the fibres carries (everything above the flat value) goes inversely with
/// the amount of solid in its way, so it is divided by the same ratio.
fn measured_material(d: &Departures) -> Material {
    let t = &tacot::TACOT;
    let frozen = material(&Departures {
        measured_curves: false,
        ..*d
    });
    let mut nodes: Vec<f64> = vec![255.55555555555554, 298.0];
    for point in MEASURED_SPECIFIC_HEAT
        .iter()
        .chain(MEASURED_CHAR_CONDUCTIVITY.iter().skip(1))
    {
        nodes.push(point.0);
    }
    nodes.extend_from_slice(&t.solid_table_k[2..]);
    nodes.push(TABLE_TOP_K);
    nodes.sort_by(|a, b| a.total_cmp(b));
    nodes.dedup_by(|a, b| (*a - *b).abs() < 1.0);
    let ratio = VIRGIN_DENSITY_KG_PER_M3 / MEASURED_CURVES_DENSITY_KG_PER_M3;
    let flat = MEASURED_CHAR_CONDUCTIVITY[0].1;
    let frozen_virgin: Vec<(f64, f64)> = frozen
        .solid_table_k
        .iter()
        .cloned()
        .zip(frozen.virgin_conductivity_w_per_m_k.iter().cloned())
        .collect();
    // Below the first measured point (475 K) nothing is measured: TACOT's
    // virgin curve is used there, raised to meet the measurement at 475 K.
    let tacot_virgin: Vec<(f64, f64)> = t
        .solid_table_k
        .iter()
        .cloned()
        .zip(t.virgin_specific_heat_j_per_kg_k.iter().cloned())
        .collect();
    let first = MEASURED_SPECIFIC_HEAT[0];
    let join = first.1 / along(&tacot_virgin, first.0, false, false);
    let specific_heat: Vec<f64> = nodes
        .iter()
        .map(|&k| {
            if k < first.0 {
                join * along(&tacot_virgin, k, false, false)
            } else {
                along(&MEASURED_SPECIFIC_HEAT, k, false, false)
            }
        })
        .collect();
    let char_conductivity: Vec<f64> = nodes
        .iter()
        .map(|&k| {
            let measured = along(
                &MEASURED_CHAR_CONDUCTIVITY,
                k,
                false,
                !d.conductivity_held_above_3333_k,
            );
            d.char_conductivity_scale * (flat * ratio + (measured - flat) / ratio)
        })
        .collect();
    let virgin_conductivity: Vec<f64> = nodes
        .iter()
        .map(|&k| along(&frozen_virgin, k, false, false))
        .collect();
    Material {
        solid_table_k: kept(nodes),
        solid_anchor_node: 1,
        virgin_specific_heat_j_per_kg_k: kept(specific_heat.clone()),
        char_specific_heat_j_per_kg_k: kept(specific_heat),
        virgin_conductivity_w_per_m_k: kept(virgin_conductivity),
        char_conductivity_w_per_m_k: kept(char_conductivity),
        pore_gas: frozen.pore_gas.map(|gas| PoreGas {
            char_reference_pa: MEASURED_CONDUCTIVITY_PRESSURE_PA,
            ..gas
        }),
        ..frozen
    }
}
