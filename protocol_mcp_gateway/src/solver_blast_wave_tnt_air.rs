//! The entry for `blast_wave_tnt_air`: the incident blast wave of a charge of
//! TNT equivalent in still sea-level air, the company's own derivation. The
//! numbers come from the `genesis_solvers` constants, the sentence of each
//! refusal from the solver's source, and the validation record from the
//! solver's research record. Stamped from the kit.

use genesis_solvers::blast_wave_tnt_air::{self as model, BlastQuestion, BlastReject, BlastWave, Burst};
use serde_json::{Value, json};

use crate::solvers::{
    Answer, FurtherExample, Given, Input, Kind, Output, Published, Refusal, Refused, Sections, Shape, Solved, Solver,
};

/// The fingerprint of the record of the comparison.
pub const PASS_MARKS_FIXED: &str = "Record SHA-256 e8f3ac5d85277fd482b81c8a8c2d9eb586e85a6406c3635117a34b82e03f9416.";

const fn number(name: &'static str, unit: &'static str, note: &'static str) -> Output {
    Output { name, unit, shape: Shape::Number, each: &[], note, asked_by: None }
}

const fn numbers(name: &'static str, unit: &'static str, note: &'static str) -> Output {
    Output { name, unit, shape: Shape::Numbers, each: &[], note, asked_by: None }
}

const BURSTS: &[&str] = &["free_air", "surface"];

pub const BLAST_WAVE_TNT_AIR: Solver = Solver {
    name: "blast_wave_tnt_air",
    summary: SUMMARY,
    inputs: &[
        Input {
            name: "charge_mass_kg",
            kind: Kind::NumberAbove { above: 0.0, maximum: model::CHARGE_MASS_MAX_KG, unit: "kg" },
            refusal: Some("ChargeBeyondUniformAtmosphere"),
            bound: "Mass of the charge in kilograms of TNT equivalent: the charge enters the model as its energy, 4.184 MJ per kilogram. Above zero (ChargeMassNotPositive) and at most 106,342,834 kg in free air, 53,171,417 kg on the ground (ChargeBeyondUniformAtmosphere): above it, at every point, the shock is beyond a tenth of the air's scale height, 843.45 m, when the positive phase there ends. Below it a point is answered where its own shock radius at the end of its phase is within 843.45 m, else refused sealed as WaveBeyondUniformAtmosphere; near the largest charge the points answered are those near Sachs distance 0.18. For another explosive, its TNT equivalence, which depends on the quantity and the distance, is the caller's: the model holds one energy.",
            only_when: None,
            asked_by_check: true,
        },
        Input {
            name: "distance_m",
            kind: Kind::NumberWithin {
                minimum: model::DISTANCE_MIN_M,
                minimum_exclusive: false,
                maximum: model::DISTANCE_MAX_M,
                maximum_exclusive: true,
                unit: "m",
            },
            refusal: Some("DistanceBeyondUniformAtmosphere"),
            bound: "Distance of the point from the charge's centre: from 6.37e-6 m, 100 mean free paths of the air, nearer than which it is no continuum (DistanceBelowContinuum; zero or less, DistanceNotPositive), to below 843.45 m, a tenth of the air's scale height, which the shock is past before the positive phase there ends (DistanceBeyondUniformAtmosphere). The point is answered where the shock's radius when its positive phase ends is within 843.45 m (WaveBeyondUniformAtmosphere), about 841 m for 1 kg. The scaled distance Z = R/W^(1/3) must be at least 0.5798 m/kg^(1/3) in free air and 0.7305 on the ground, the radius at which the air the shock has engulfed weighs as much as the charge (ChargeProductsNearField). Beyond Sachs distance 4, about Z = 13.8 in free air and 17.4 on the ground, weak-shock theory carries the wave: a regime of the same wave. Far out a small charge's shock is not sharp against its phase (ShockNotSharp): a gram at 840 m.",
            only_when: None,
            asked_by_check: true,
        },
        Input {
            name: "burst",
            kind: Kind::Word { allowed: BURSTS },
            refusal: Some("UnknownBurst"),
            bound: "`free_air`: a spherical charge, the point anywhere around it. `surface`: a hemispherical charge resting on rigid, flat ground, the point on or above the ground; the ground is a plane of symmetry, so it is half of a free-air burst of twice the charge (a ground that gives, cratering or soft, holds less).",
            only_when: None,
            asked_by_check: false,
        },
    ],
    kept_inside: "The air (ISA sea level), the energy of TNT, the grids, the start from Sedov-Taylor and the tolerances stay inside the solver.",
    result: &[
        number("scaled_distance_m_per_kg13", "m/kg^(1/3)", "Z = R/W^(1/3), Hopkinson-Cranz scaled distance."),
        number("sachs_scaled_distance", "1", "R/(E/p0)^(1/3), E the free-air point source's energy: the one variable of the dimensionless wave."),
        number("charge_energy_j", "J", "The free-air point source's energy: W times 4.184 MJ/kg, twice that on the ground. The energy ledger holds the air's energy above ambient to it."),
        number("arrival_time_s", "s", "Time of arrival of the shock at the point after the release of the energy (half-height of its rise). Early against the gas charges compared with measurement, by 5 to 25 percent."),
        number("peak_overpressure_pa", "Pa", "The incident (side-on) peak overpressure: the Rankine-Hugoniot jump of the shock's Mach number. Above the gas charges compared by 33 to 72 percent; below TNT, by Dewey's tables, by 6 to 33 percent from Z 1.3 to 15."),
        number("shock_mach_number", "1", "The shock's speed over the ambient sound speed, 340.29 m/s, at the point."),
        number("positive_phase_duration_s", "s", "From the arrival to the first fall of the overpressure to zero."),
        number("positive_impulse_pa_s", "Pa s", "The overpressure history integrated over the positive phase."),
        number("friedlander_decay", "1", "a in p = Ps (1 - tau/td) exp(-a tau/td), the Friedlander form whose integral is the impulse."),
        number("scaled_arrival_time_ms_per_kg13", "ms/kg^(1/3)", "The arrival time over W^(1/3), as Kingery-Bulmash tabulate it."),
        number("scaled_positive_phase_duration_ms_per_kg13", "ms/kg^(1/3)", "The duration over W^(1/3)."),
        number("scaled_positive_impulse_kpa_ms_per_kg13", "kPa ms/kg^(1/3)", "The impulse over W^(1/3)."),
        number("sedov_taylor_overpressure_pa", "Pa", "What the strong-shock Sedov-Taylor law alone gives at this distance, 8 xi0^5/(25 (gamma + 1)) E/R^3 = 0.1567 E/R^3: near the answer where the shock is strong, far below it where p0 matters (falling as 1/R^3)."),
        number("euler_reach_sachs_distance", "1", "The farthest Sachs distance the Euler equations carried the wave: the point's own, or 4, beyond which weak-shock theory carried it."),
        number("shock_radius_at_end_m", "m", "How far from the charge the shock is when the positive phase at the point ends (the half-height of its rise): the air the answer rests on. At most 843.45 m, a tenth of the scale height (WaveBeyondUniformAtmosphere)."),
        number("shock_rise_time_s", "s", "The shock's thermoviscous rise time at the point, Taylor's 2 rho0 delta/(beta Ps), delta = 2.771e-5 m^2/s the air's diffusivity of sound from its viscosity and conduction, beta = 1.2. Joined to the decay behind it the rise lowers the peak by (e/4)(1 + ln(4/e)), e = 2 (1 + a) rise/duration, at most 2.5 percent where answered (ShockNotSharp)."),
        numbers("history_time_after_arrival_s", "s", "33 times from the arrival to the end of the positive phase, evenly spaced."),
        numbers("history_overpressure_pa", "Pa", "The computed overpressure at the point at those times. In the Euler range the shock's jump is spread over a few cells of the grid, so the first values rise to the peak; in the weak-shock range the jump is sharp."),
        numbers("friedlander_overpressure_pa", "Pa", "The Friedlander form at those times, from the peak, the duration and the decay."),
    ],
    invariants: &[
        number("energy_ledger_residual", "1", ""),
        number("impulse_closure_residual", "1", ""),
        number("weak_shock_area_residual", "1", ""),
        number("grid_change_peak", "1", ""),
        number("grid_change_impulse", "1", ""),
        number("grid_change_duration", "1", ""),
        number("grid_change_arrival", "1", ""),
    ],
    iterations: "Time steps of the two Euler runs together, the coarse grid's and the fine grid's.",
    field: field_words,
    answer: "a blast wave",
    refusals: REFUSALS,
    door_refusals: &[Refusal { name: "UnknownBurst", meaning: "The burst is neither free_air nor surface.", sealed: false }],
    sections,
    validation: Some(validation),
    example,
    further_examples: &[
        FurtherExample { key: "example_solve_surface_burst", parameters: surface_example },
        FurtherExample { key: "example_solve_acoustic_far_field", parameters: far_example },
    ],
    parts: &[],
    check,
    solve,
};

const SUMMARY: &str = "The incident blast wave of a charge of TNT equivalent in still sea-level air, a sphere in free air or a hemisphere on rigid ground, at a point up to 843.45 m from it: arrival time, peak overpressure, positive-phase duration and impulse, the Friedlander form and the pressure history. The company's own derivation: a point source of the charge's energy, 4.184 MJ per kg, started from the Sedov-Taylor strong-shock solution, carried by the spherical Euler equations with the ambient pressure to Sachs distance 4 and by weak-shock theory beyond; the air's energy above ambient is held to the charge's at every step. Nearer than Z = 0.5798 m/kg^(1/3) in free air or 0.7305 on the ground, where the air engulfed weighs less than the charge, it is refused by name, and so is a wave whose shock is past 843.45 m when the positive phase at the point ends, or whose shock is not sharp against that phase. Compared once, with pass marks fixed before, with the gauge records of propane/oxygen surface bursts of 0.37 g (Dewey and Sochet 2020) from 0.255 to 0.904 m: missed every line on all 24 held-back traces, the peak 33 to 72 percent above the measured, the impulse 43 to 78 percent above, the arrival 5 to 25 percent early; computed after the scoring, on peak and impulse the gas charges act as 38 to 58 percent of their heat of reaction. Against TNT, by Dewey's measurement-derived tables, it is below: the peak 22 to 33 percent from Z 1.3 to 2 and 6 to 17 percent from 3 to 15.";

/// Each `BlastReject` variant with the sentence above it in the solver's
/// source. A refusal the request itself shows is answered at the door.
const REFUSALS: &[Refusal] = &[
    Refusal {
        name: "InputNotFinite",
        meaning: "An input is not a finite number.",
        sealed: false,
    },
    Refusal {
        name: "ChargeMassNotPositive",
        meaning: "The charge mass is zero or less: there is no charge.",
        sealed: false,
    },
    Refusal {
        name: "DistanceNotPositive",
        meaning: "The distance is zero or less.",
        sealed: false,
    },
    Refusal {
        name: "ChargeBeyondUniformAtmosphere",
        meaning: "The charge is above 106,342,834 kg of TNT equivalent in free air, or half that on the ground: at every point of it the shock is beyond a tenth of the air's scale height, 843.45 m, when the positive phase there ends, that radius being least, 0.51507 Sachs lengths, near the charge, where the phases end at nearly one time.",
        sealed: false,
    },
    Refusal {
        name: "DistanceBeyondUniformAtmosphere",
        meaning: "The distance is a tenth of the air's scale height, 843.45 m, or beyond, where the ambient pressure differs from the charge's by 1 - e^-0.1 = 9.52 percent or more: the shock is past it before the positive phase at the point ends.",
        sealed: false,
    },
    Refusal {
        name: "DistanceBelowContinuum",
        meaning: "The distance is under 100 mean free paths of the air, 6.37e-6 m: the mean free path over the distance, the Knudsen number, passes 0.01, the edge of the continuum, where the Euler equations of a gas stop holding.",
        sealed: false,
    },
    Refusal {
        name: "ChargeProductsNearField",
        meaning: "The point is inside the radius at which the air the shock has engulfed weighs as much as the charge, Z = 0.5798 m/kg^(1/3) in free air and 0.7305 on the ground: the charge's detonation products carry the blast there, and a point source of its energy cannot.",
        sealed: false,
    },
    Refusal {
        name: "WaveBeyondUniformAtmosphere",
        meaning: "When the positive phase at the point ends, the shock is beyond a tenth of the air's scale height, 843.45 m, from the charge: the air the answer rests on reaches where the ambient pressure differs from the charge's by more than 1 - e^-0.1 = 9.52 percent, and the model's air is uniform.",
        sealed: true,
    },
    Refusal {
        name: "ShockNotSharp",
        meaning: "The shock's thermoviscous rise time, Taylor's 2 rho0 delta/(beta Ps), lowers the peak of the positive phase it heads by more than 2.5 percent, the peak's own settling line: the shock is not sharp against its phase, and the inviscid wave the model carries is not the air's.",
        sealed: true,
    },
    Refusal {
        name: "NonFiniteValue",
        meaning: "A value of the flow was not finite.",
        sealed: true,
    },
    Refusal {
        name: "InternalEnergyNotPositive",
        meaning: "A cell's internal energy fell to zero or below.",
        sealed: true,
    },
    Refusal {
        name: "CellInverted",
        meaning: "A cell's outer face crossed its inner face.",
        sealed: true,
    },
    Refusal {
        name: "EnergyLedgerBroken",
        meaning: "The energy of the air above ambient departed from the charge's energy by more than 1e-10 of it at a step.",
        sealed: true,
    },
    Refusal {
        name: "StepCapReached",
        meaning: "The Euler run reached 400,000 steps before the positive phase at the point ended.",
        sealed: true,
    },
    Refusal {
        name: "ImpulseClosureBroken",
        meaning: "The Friedlander form's integral departs from the impulse by more than 1e-12 of it.",
        sealed: true,
    },
    Refusal {
        name: "AreaNotCarried",
        meaning: "Weak-shock theory changed r times the impulse by more than 1e-12 of it.",
        sealed: true,
    },
    Refusal {
        name: "NotSettled",
        meaning: "The coarse and fine grids disagree by more than 2.5 percent in the peak, 1 percent in the impulse, 1.5 percent in the duration or 0.5 percent in the arrival time.",
        sealed: true,
    },
];

fn field_words() -> String {
    format!(
        "SHA-256 of the count of the numbers that follow ({}) as 8 little-endian bytes, then the {} times after arrival of the history in seconds, then its {} overpressures in pascals, all as little-endian f64 bytes.",
        2 * model::HISTORY_POINTS,
        model::HISTORY_POINTS,
        model::HISTORY_POINTS
    )
}

fn sections() -> Sections {
    let sedov = model::sedov_taylor(model::GAMMA, model::SEDOV_TABLE_POINTS);
    Sections {
        locked: json!({
            "air": "The International Standard Atmosphere at sea level, still and uniform: an ideal gas of ratio of specific heats 1.4.",
            "ambient_pressure_pa": model::AMBIENT_PRESSURE_PA,
            "ambient_temperature_k": model::AMBIENT_TEMPERATURE_K,
            "air_gas_constant_j_per_kg_k": model::AIR_GAS_CONSTANT_J_PER_KG_K,
            "gamma": model::GAMMA,
            "ambient_density_kg_per_m3": model::ambient_density_kg_per_m3(),
            "sound_speed_m_per_s": model::sound_speed_m_per_s(),
            "tnt_energy_j_per_kg": model::TNT_ENERGY_J_PER_KG,
            "tnt_equivalence": "A kilogram of TNT equivalent is 4.184 MJ, the convention by which energy is stated in TNT (the ton of TNT is 4.184e9 J). The charge enters as that energy alone, released at a point at one instant.",
            "surface_burst": "On rigid ground the ground plane is a plane of symmetry: a hemispherical burst of W is half of a free-air burst of 2W, so its point source holds twice the energy.",
            "similarity": "For a point release of energy E in an ideal gas at rest, every overpressure over p0 is a function of lambda = R/(E/p0)^(1/3) and t c0/(E/p0)^(1/3) alone (Sachs scaling; at one ambient state, Hopkinson-Cranz scaling in Z = R/W^(1/3)). One dimensionless wave is computed and scaled to the question.",
            "sedov_taylor": "Where the shock is strong, R = xi0 (E t^2/rho0)^(1/5) and the overpressure behind it is 8 xi0^5/(25 (gamma + 1)) E/R^3. Sedov's closed-form profile (Landau and Lifshitz, Fluid Mechanics, section 106) is evaluated at 20,000 points; its energy integral gives alpha and xi0. The wave starts from it at lambda = 0.05, the jump 1254 p0.",
            "sedov_taylor_alpha": sedov.alpha,
            "sedov_taylor_xi0": sedov.xi0,
            "euler": "From lambda = 0.05 the spherical Euler equations of the air, with p0 ahead of the shock: Lagrangian finite volumes in the radius, limited linear reconstruction of pressure and velocity, two-stage strong-stability-preserving Runge-Kutta, a two-shock acoustic Riemann solver at every node, face areas averaged over the step so a node's swept volume is exact. Cells are added at ambient ahead of the wave; pairs of hot cells far behind the shock are joined, their mass, momentum and energy summed.",
            "peak": "The Rankine-Hugoniot jump 2 gamma (M^2 - 1)/(gamma + 1) p0 of the shock's Mach number M, from a least-squares quadratic of arrival time in radius over seven gauges within 5 percent of the point.",
            "weak_shock": "Beyond lambda = 4 the Friedlander form of the positive phase there, as r p' against retarded time, is carried out: each level gains (gamma + 1)/(2 rho0 c0^3) r p' ln(r/r1) in retarded time, and the shock sits where the overrun areas balance (Whitham's equal-area rule), sampled at 4096 points.",
            "near_field_scaled_distance_free_air_m_per_kg13": model::near_field_scaled_distance(Burst::FreeAir),
            "near_field_scaled_distance_surface_m_per_kg13": model::near_field_scaled_distance(Burst::Surface),
            "near_field_sachs_distance": model::near_field_sachs_distance(),
            "near_field": "Inside the radius at which the air engulfed weighs as much as the charge, (4/3) pi R^3 rho0 = W in free air and (2/3) pi R^3 rho0 = W on the ground, the detonation products carry the blast. There the shock is at Mach 5.5067, 34.21 p0, about 1,970 K behind it in gamma = 1.4 air.",
            "uniform_air": "One criterion holds the model's uniform air: the shock's radius when the positive phase at the point ends, at most a tenth of the scale height, 843.45 m, across which the ambient pressure falls by 1 - e^-0.1 = 9.52 percent. A point at 843.45 m or beyond, and a charge above the largest, are its cases the question shows, refused at the door. From the near-field limit out to Sachs distance 0.2 the phases end at nearly one time and that radius is 0.51507 to 0.51522 Sachs lengths, least at 0.1835; beyond, it rises.",
            "least_reach_sachs_distance": model::LEAST_REACH_SACHS_DISTANCE,
            "least_shock_reach_sachs": model::LEAST_SHOCK_REACH_SACHS,
            "continuum": "The air is a continuum, and the Euler equations hold, where its mean free path, (mu/p0)(pi R T0/2)^(1/2) = 6.37e-8 m, is under 0.01 of the distance: 6.37e-6 m and out.",
            "air_viscosity_pa_s": model::AIR_VISCOSITY_PA_S,
            "air_thermal_conductivity_w_per_m_k": model::AIR_THERMAL_CONDUCTIVITY_W_PER_M_K,
            "sound_diffusivity_m2_per_s": model::sound_diffusivity_m2_per_s(),
            "mean_free_path_m": model::mean_free_path_m(),
            "distance_min_m": model::DISTANCE_MIN_M,
            "sharp_shock": "The model's shock is a jump. The air's viscosity and heat conduction give a weak shock Taylor's rise, p = (Ps/2)(1 + tanh(tau/tau_s)), tau_s = 2 rho0 delta/(beta Ps) (the steady solution of Burgers' equation); joined to the Friedlander decay behind it, of slope Ps (1 + a)/td, it lowers the peak by (e/4)(1 + ln(4/e)), e = 2 (1 + a) tau_s/td, to first order. Where that exceeds 2.5 percent, the peak's own settling tolerance, the answer is refused as ShockNotSharp. Viscosity (U.S. Standard Atmosphere 1976, 1.7894e-5 Pa s) and conduction (2.5326e-2 W/(m K)) at sea level. Molecular relaxation of oxygen and nitrogen, which hangs on the humidity, lengthens a real weak shock's rise beyond Taylor's.",
            "scale_height_m": model::SCALE_HEIGHT_M,
            "distance_max_m": model::DISTANCE_MAX_M,
            "charge_mass_max_kg": model::CHARGE_MASS_MAX_KG,
            "weak_shock_from_sachs_distance": model::WEAK_SHOCK_FROM_SACHS_DISTANCE,
            "fine_cell_sachs": model::FINE_CELL_SACHS,
            "fine_cells_across_distance": model::FINE_CELLS_ACROSS_DISTANCE,
            "coarse_factor": model::COARSE_FACTOR,
            "courant_number": model::COURANT_NUMBER,
            "history_points": model::HISTORY_POINTS,
            "fitted": "Nothing. Every constant is the air's, TNT's convention, Sedov's, or a numerical choice fixed against the solver's own refinement.",
        }),
        invariants: json!([
            {
                "name": "energy_ledger_residual",
                "rule": "The energy of the air above ambient, kinetic plus internal less the ambient's, over the whole domain, equals the charge's energy at every step: the largest departure over the fine grid's run, as a fraction of the charge's energy. The fluxes telescope and the cells added ahead are at ambient, so it closes to rounding, about 1e-14.",
                "tolerance": model::ENERGY_LEDGER_TOLERANCE,
                "unit": "fraction of the charge's energy",
                "held": "every step of both runs",
                "refusal": "EnergyLedgerBroken",
            },
            {
                "name": "impulse_closure_residual",
                "rule": "The Friedlander form's integral, Ps td (1/a - (1 - exp(-a))/a^2), equals the impulse: a is solved by bisection to adjacent 64-bit numbers.",
                "tolerance": model::IMPULSE_CLOSURE_TOLERANCE,
                "unit": "fraction of the impulse",
                "held": "at acceptance",
                "refusal": "ImpulseClosureBroken",
            },
            {
                "name": "weak_shock_area_residual",
                "rule": "Through the weak-shock regime the area of r p' over the positive phase, r times the impulse, is carried unchanged. Zero where the Euler run reached the point.",
                "tolerance": model::AREA_TOLERANCE,
                "unit": "fraction of r times the impulse",
                "held": "at acceptance",
                "refusal": "AreaNotCarried",
            },
            {
                "name": "grid_change_peak",
                "rule": "The change of the peak overpressure from the coarse grid to the fine.",
                "tolerance": model::SETTLE_PEAK,
                "unit": "fraction",
                "held": "at acceptance",
                "refusal": "NotSettled",
            },
            {
                "name": "grid_change_impulse",
                "rule": "The change of the impulse from the coarse grid to the fine.",
                "tolerance": model::SETTLE_IMPULSE,
                "unit": "fraction",
                "held": "at acceptance",
                "refusal": "NotSettled",
            },
            {
                "name": "grid_change_duration",
                "rule": "The change of the positive-phase duration from the coarse grid to the fine.",
                "tolerance": model::SETTLE_DURATION,
                "unit": "fraction",
                "held": "at acceptance",
                "refusal": "NotSettled",
            },
            {
                "name": "grid_change_arrival",
                "rule": "The change of the arrival time from the coarse grid to the fine.",
                "tolerance": model::SETTLE_ARRIVAL,
                "unit": "fraction",
                "held": "at acceptance",
                "refusal": "NotSettled",
            },
        ]),
        convergence: json!({
            "rule": "The whole wave is computed on two grids, the fine one's cell the smaller of 0.00125 Sachs lengths and the point's distance over 800, the coarse one's twice it. The fine grid's values are returned when the two agree within the grid_change tolerances, every step held the energy ledger, and the Euler run ended the positive phase at the point within 400,000 steps. Across the range the changes are at most about 0.7 percent in the peak, 0.5 in the impulse, 0.2 in the duration and 0.1 in the arrival.",
            "refusal": "NotSettled, StepCapReached, EnergyLedgerBroken, InternalEnergyNotPositive, CellInverted or NonFiniteValue; once settled, WaveBeyondUniformAtmosphere or ShockNotSharp",
            "run_time": "0.06 to 0.53 s for one solve on one core of an Apple M4 Pro in a release build, measured under load: the longest at the near-field limit (0.52 s) and from Sachs distance 4 out (0.36 s, the uniform air's reach found by bisection included); a refusal the solve reaches takes the same.",
        }),
        reference: json!({
            "name": model::REFERENCE_NAME,
            "url": model::REFERENCE_URL,
            "retrieved": model::REFERENCE_RETRIEVED,
            "test": "dewey_tnt_free_field_comparison",
            "covers": "1 kg of TNT in free air at Z = 2, 5 and 10 m/kg^(1/3): the model's peak 21.7, 11.1 and 7.8 percent below Dewey's, its impulse 5.3, 8.4 and 7.6 percent below, its arrival 7.8, 4.0 and 2.0 percent late, its duration 25.8, 4.5 and 2.5 percent long, each held to 1 percentage point. A second test, kingery_bulmash_surface_burst_comparison, holds the surface burst against Kingery-Bulmash in Swisdak's simplified form as coded by fcento100 (MIT), the code and not either original report: the peak 5.5 to 16.3 percent below from Z = 2 to 40, the impulse 16 to 20.5 percent below, the arrival within 3.6 percent.",
            "kind": "Measurement-derived tables of TNT explosions (Dewey and McMillin's particle-trajectory analyses of 4 to 500 kg shots), and a published correlation of TNT tests. The comparison with gauge records, with nothing fitted, is in validation.",
        }),
    }
}

fn validation() -> [String; 5] {
    let object = "Small hemispherical surface bursts of propane and oxygen, each 0.37 g in a soap bubble of 5 cm radius on the ground, measured by piezoelectric gauges flush in the ground (University of Victoria and INSA Centre Val de Loire, PRISME). They are gas micro-charges, not high explosive. The model answers a charge of TNT equivalent; each charge is given as its heat of reaction, the limiting reactant burnt to CO2 and water vapour at 298.15 K (2043.13 kJ per mol of propane), at 4.184 MJ/kg of TNT: stoichiometric 3,699.0 J, 0.8841 g; rich 3,588.4 J, 0.8576 g (the propane its oxygen cannot burn stays unburnt); lean 3,061.3 J, 0.7317 g; on rigid ground.".to_string();
    let data = "Gauge records of the three mixtures, Borealis doi 10.5683/SP2/VHN1UQ: for each mixture 14 gauges at 0.145 to 0.904 m, the time of arrival after ignition, the overpressure record at 1 us, and a record truncated to the second shock. Nothing fitted. Compared with the rich and lean series from 0.255 to 0.904 m, 24 traces; the stoichiometric series and the gauges at 0.145 and 0.152 m are reported beside them. The shots' ambient pressure and temperature are unknown, by the source's own account; the model's air is ISA sea level. Each record is read one way: the peak is its largest value; the impulse the record integrated from the arrival to its first zero crossing; the arrival the measured time. In the rich sheet from 0.255 m out the raw column holds times, so the truncated record is read there; 11 rich records and one stoichiometric end before their zero crossing and are compared on the peak and arrival alone.".to_string();
    let pass_line = format!("Per trace, on the 24 traces of the rich and lean series from 0.255 to 0.904 m: the peak within 15 percent, the impulse within 20 percent and the arrival within 5 percent; at least 20 of the 24 within every target compared on them. {PASS_MARKS_FIXED}");
    [object, data, pass_line, RESULT.to_string(), LIMITS.to_string()]
}

const RESULT: &str = "On the 24 traces: 0 of 24 within every target (target 20); the peak 0 of 24 within 15 percent, the impulse 0 of 13 within 20, the arrival 0 of 24 within 5. The model's peak is 33 to 72 percent above the measured (median 44.5), its impulse 43 to 78 percent above (median 54, on the 13 with an impulse), its arrival 5.2 to 25.4 percent early (median 10.9, least at 0.904 m). Rich: peak 40 to 72 percent above; lean: 33 to 65. The stoichiometric series lies the same way: peak 36 to 77 percent above, impulse 61 to 82, arrival 7 to 28 percent early. The gauges at 0.145 and 0.152 m, within 3 bubble radii: peak 102 to 150 percent above, arrival 38 to 45 percent early. Against the authors' fitted modified-Friedlander values: peak 27 to 54 percent above, impulse 41 to 96 percent above. With no constant of the model changed, the point source reaches the measured peaks at 38 to 58 percent of each charge's heat of reaction on rigid ground (median 46; stoichiometric about 42, rich 44, lean 51), the impulses at 39 to 57 percent (median 47), and the arrivals at 27 to 53 percent, least near the charge, where the bubble's own size delays the shock. The gap is the distance between an energy-equivalent point source and a gaseous charge's blast: TNT equivalence by energy is not equivalence in blast, and it differs by quantity and distance.";

const LIMITS: &str = "Fitted constants: none. The model holds a charge as its energy alone, in uniform still air at sea level, so a charge of another explosive, another shape or another composition is the caller's TNT equivalence, which differs by quantity and distance. Against TNT itself, by Dewey's measurement-derived tables, the model is below in the peak from Z 1.3 to 15 m/kg^(1/3) (33 to 22 percent below from 1.3 to 2, 17 to 6 percent from 3 to 15), its impulse 3 to 9 percent below from Z 2 to 15 and 22 percent above at 1.3, its duration 79 percent long at 1.3 and 2 to 7 percent long from 4 to 15, its arrival 1 to 8 percent late from 1.3 to 15: TNT's products sustain its shock where a point source's energy, spent heating the air it first shocks, cannot. Against Kingery-Bulmash for a surface burst (Swisdak's simplified form as coded by fcento100, the code and not the original report) the peak is 5 to 16 percent below from Z 2 to 40 and 16 to 39 percent above from 100 to 198.5, and the impulse 15 to 20 percent below from 2 to 40; against Kinney-Graham's free-air peak (as given by Karlos and Solomos, Eq. 8) it is within 6 percent from Z 5 to 15, 26 to 33 percent below from 1 to 2, and 9 to 37 percent below from 20 to 198.5. Far out the correlations part from each other and from the model: Kingery-Bulmash's peak falls as Z^-1.41, Kinney-Graham's as 1/Z, weak-shock theory's as 1/(Z (ln Z)^(1/2)); the two TNT references themselves differ, Kingery-Bulmash's durations 14 to 255 percent longer and impulses 14 to 63 percent larger than Dewey's. In the far field the weak-shock regime's peak falls about 2 percent faster than the Euler equations carry it (2.3 percent below at Sachs distance 6 and 3 at 8 when started at 4, measured against the Euler solution), its impulse within 0.1 percent. The answer is the incident (side-on) wave of the positive phase in still, uniform air over rigid ground; a surface the wave strikes, a ground that gives, and wind and temperature gradients change it, and the negative phase follows it. Nearer than the near-field limit the charge's products carry the blast and the point is refused; a measured near-field shot at Z = 0.39 is refused there. The model's shock is a jump: where the air's viscosity and conduction would round its peak by more than 2.5 percent it is refused (ShockNotSharp, a gram at 840 m), and where it answers, the vibrational relaxation of oxygen and nitrogen in moist air, which lengthens a real weak shock's rise beyond that, can lower the peak a gauge reads far out. The record of this comparison was written with the sheets' header rows read, which hold the authors' fitted peaks, durations and impulses for every gauge, with every measured arrival time read, and with the model's answers at the 42 gauges computed; its targets were proposed before this model existed, its two rules for reading the records are in it, and the model is unchanged since. Its forecast, written with all of that read, put nearly every trace outside its targets.";

fn example() -> Value {
    json!({ "charge_mass_kg": 1.0, "distance_m": 5.0, "burst": "free_air" })
}

fn surface_example() -> Value {
    json!({ "charge_mass_kg": 100.0, "distance_m": 30.0, "burst": "surface" })
}

fn far_example() -> Value {
    json!({ "charge_mass_kg": 1.0, "distance_m": 200.0, "burst": "free_air" })
}

/// The solver's own question, from the values of `parameters`.
fn question(given: &Given) -> Result<BlastQuestion, String> {
    let burst = match given.word("burst")? {
        "surface" => Burst::Surface,
        _ => Burst::FreeAir,
    };
    Ok(BlastQuestion { charge_mass_kg: given.number("charge_mass_kg")?, distance_m: given.number("distance_m")?, burst })
}

/// Every limit the request shows, asked of the solver's own `admit` before
/// a physics permit is taken. The sentence is the solver's, with its name.
fn check(given: &Given) -> Result<(), String> {
    model::admit(&question(given)?).map_err(|reject| format!("Physical Violation: {reject} ({}).", reject.name()))
}

fn solve(given: &Given) -> Result<Solved, String> {
    Ok(solved(model::solve(&question(given)?)))
}

/// What the solver returned, in the words every solver shares.
pub fn solved(returned: Result<BlastWave, BlastReject>) -> Solved {
    let wave = match returned {
        Ok(wave) => wave,
        Err(reject) => {
            let name = reject.name();
            let sealed = REFUSALS.iter().any(|refusal| refusal.name == name && refusal.sealed);
            return Solved::Refused(Refused {
                name,
                sentence: if sealed { reject.to_string() } else { format!("{reject} ({name})") },
                iterations: None,
            });
        }
    };
    let one = |name: &str, value: f64| (name.to_string(), Published::Number(value));
    let many = |name: &str, values: Vec<f64>| (name.to_string(), Published::Numbers(values));
    let mut field = Vec::with_capacity(2 * wave.history_time_after_arrival_s.len());
    field.extend_from_slice(&wave.history_time_after_arrival_s);
    field.extend_from_slice(&wave.history_overpressure_pa);
    Solved::Answer(Answer {
        iterations: wave.steps,
        result: vec![
            one("scaled_distance_m_per_kg13", wave.scaled_distance_m_per_kg13),
            one("sachs_scaled_distance", wave.sachs_scaled_distance),
            one("charge_energy_j", wave.charge_energy_j),
            one("arrival_time_s", wave.arrival_time_s),
            one("peak_overpressure_pa", wave.peak_overpressure_pa),
            one("shock_mach_number", wave.shock_mach_number),
            one("positive_phase_duration_s", wave.positive_phase_duration_s),
            one("positive_impulse_pa_s", wave.positive_impulse_pa_s),
            one("friedlander_decay", wave.friedlander_decay),
            one("scaled_arrival_time_ms_per_kg13", wave.scaled_arrival_time_ms_per_kg13),
            one("scaled_positive_phase_duration_ms_per_kg13", wave.scaled_positive_phase_duration_ms_per_kg13),
            one("scaled_positive_impulse_kpa_ms_per_kg13", wave.scaled_positive_impulse_kpa_ms_per_kg13),
            one("sedov_taylor_overpressure_pa", wave.sedov_taylor_overpressure_pa),
            one("euler_reach_sachs_distance", wave.euler_reach_sachs_distance),
            one("shock_radius_at_end_m", wave.shock_radius_at_end_m),
            one("shock_rise_time_s", wave.shock_rise_time_s),
            many("history_time_after_arrival_s", wave.history_time_after_arrival_s),
            many("history_overpressure_pa", wave.history_overpressure_pa),
            many("friedlander_overpressure_pa", wave.friedlander_overpressure_pa),
        ],
        invariants: vec![
            one("energy_ledger_residual", wave.energy_ledger_residual),
            one("impulse_closure_residual", wave.impulse_closure_residual),
            one("weak_shock_area_residual", wave.weak_shock_area_residual),
            one("grid_change_peak", wave.grid_change_peak),
            one("grid_change_impulse", wave.grid_change_impulse),
            one("grid_change_duration", wave.grid_change_duration),
            one("grid_change_arrival", wave.grid_change_arrival),
        ],
        field,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::solvers::describe_value;

    #[test]
    fn every_refusal_is_the_solvers_own_sentence() {
        assert_eq!(REFUSALS.len(), model::ALL_REJECTS.len());
        for (listed, reject) in REFUSALS.iter().zip(model::ALL_REJECTS) {
            assert_eq!(listed.name, reject.name());
            assert_eq!(listed.meaning, reject.meaning(), "{}", listed.name);
        }
    }

    #[test]
    fn every_published_name_is_in_the_entry() {
        let answer = match solved(model::solve(&BlastQuestion { charge_mass_kg: 1.0, distance_m: 5.0, burst: Burst::FreeAir })) {
            Solved::Answer(answer) => answer,
            Solved::Refused(refused) => panic!("{}", refused.sentence),
        };
        assert_eq!(answer.result.len(), BLAST_WAVE_TNT_AIR.result.len());
        for ((name, value), output) in answer.result.iter().zip(BLAST_WAVE_TNT_AIR.result) {
            assert_eq!(name, output.name);
            assert_eq!(value.shape(), output.shape);
        }
        assert_eq!(answer.invariants.len(), BLAST_WAVE_TNT_AIR.invariants.len());
        for ((name, _), output) in answer.invariants.iter().zip(BLAST_WAVE_TNT_AIR.invariants) {
            assert_eq!(name, output.name);
        }
        assert_eq!(answer.field.len(), 2 * model::HISTORY_POINTS);
    }

    #[test]
    fn the_reference_is_the_one_stored_in_the_solver_test() {
        let source = include_str!("../../genesis_solvers/src/blast_wave_tnt_air.rs");
        let body = describe_value(BLAST_WAVE_TNT_AIR.name).expect("described");
        let test = body["reference"]["test"].as_str().expect("test");
        assert!(source.contains(&format!("fn {test}()")), "{test}");
        assert!(source.contains("fn kingery_bulmash_surface_burst_comparison()"));
        assert!(source.contains("fn dewey_sochet_propane_oxygen_held_back_comparison()"));
        assert_eq!(body["reference"]["url"], model::REFERENCE_URL);
    }

    /// The list an agent reads first carries the standing.
    #[test]
    fn the_summary_carries_the_standing() {
        let summary = crate::spec_sheet::summary_of(BLAST_WAVE_TNT_AIR.name, BLAST_WAVE_TNT_AIR.summary);
        assert!(summary.contains("no trace within its targets"), "{summary}");
        assert!(summary.contains("0.255 to 0.904 m"), "{summary}");
        assert!(summary.contains("(ChargeProductsNearField)"), "{summary}");
        let body = describe_value(BLAST_WAVE_TNT_AIR.name).expect("described");
        assert_eq!(body["locked"]["fitted"].as_str().expect("fitted").split('.').next(), Some("Nothing"));
    }
}
