//! The entry for `junction_nist_b7_s2`: the current-voltage characteristic
//! of one NIST Nb/a-Si/Nb Josephson junction, D220831b-B7_S2_1.5, and its
//! Shapiro steps under a microwave drive. The numbers come from
//! `genesis_solvers` constants.

use genesis_solvers::junction_nist_b7_s2::{
    self as junction, Drive, JunctionCurve, JunctionQuestion, JunctionReject, Start,
};
use serde_json::{Value, json};

use crate::solvers::{
    Answer, Given, Input, Kind, Output, Published, Refusal, Refused, Sections, Shape, Solved,
    Solver,
};

const STARTS: &[&str] = &["rest", "running"];
const DRIVES: &[&str] = &["none", "microwave"];
const MICROWAVE: Option<(&str, &str)> = Some(("drive", "microwave"));

const fn output(name: &'static str, unit: &'static str, shape: Shape, note: &'static str) -> Output {
    Output {
        name,
        unit,
        shape,
        each: &[],
        note,
        asked_by: None,
    }
}

pub const JUNCTION_NIST_B7_S2: Solver = Solver {
    name: "junction_nist_b7_s2",
    summary: SUMMARY,
    inputs: &[
        Input {
            name: "bias_current_a",
            kind: Kind::Numbers {
                fewest: 1,
                most: junction::MAX_BIAS_POINTS,
                minimum: -junction::CURRENT_REACH_MAX_A,
                maximum: junction::CURRENT_REACH_MAX_A,
                unit: "A",
            },
            refusal: Some("StepsBeyondSolveBudget"),
            bound: "The held bias current of each point of the characteristic, 1 to 36 points, each computed on its own from its own start. The sign is the direction of the current; the junction is the same either way. The junction's critical current is 0.298 mA and its normal resistance 4.8169 ohm. Compared with measurement at 0 to 0.91847 mA, both sweeps; past that it computes the same equations. Each point takes about 1.1 million steps, and more where its bias and drive can carry the voltage above the table (the step is made finer there); a question whose steps, counted with every window of every point, exceed the budget of 120,000,000 one solve computes is refused before it starts (StepsBeyondSolveBudget: a budget of computing, not a limit of the physics), and so is one with no point (NoBiasPoints). 33.456 mA, the published bound, is exactly the largest bias one point alone with no drive takes within the budget; more points, or a drive, take less.",
            only_when: None,
            asked_by_check: true,
        },
        Input {
            name: "start",
            kind: Kind::Word { allowed: STARTS },
            refusal: Some("UnknownStart"),
            bound: "`rest`: each point starts at zero voltage, the phase where the supercurrent carries the bias (pi/2 above the critical current): the branch a sweep rising from zero reaches. `running`: each point starts at a voltage above the running motion: the branch a sweep falling from the voltage state reaches. Between the return current (about 0.060 mA) and the critical current the two differ: the junction is hysteretic.",
            only_when: None,
            asked_by_check: false,
        },
        Input {
            name: "drive",
            kind: Kind::Word { allowed: DRIVES },
            refusal: Some("UnknownDrive"),
            bound: "`none`: the bias is held. `microwave`: a current I_ac sin(2 pi f t) is added to the bias of each point, t from the point's start, and the mean voltage is read over whole periods of the drive.",
            only_when: None,
            asked_by_check: false,
        },
        Input {
            name: "drive_current_amplitude_a",
            kind: Kind::Number {
                minimum: -junction::CURRENT_REACH_MAX_A,
                maximum: junction::CURRENT_REACH_MAX_A,
                unit: "A",
            },
            refusal: Some("StepsBeyondSolveBudget"),
            bound: "I_ac, the amplitude of the microwave current. A negative amplitude is the drive half a period later. With the bias it sets the largest voltage the point can reach, and so its step; the count of steps is held to the budget as for the bias (StepsBeyondSolveBudget). A driven point counts at least the steps of an undriven one at |I_dc| + |I_ac|, so no amplitude above 33.456 mA is answered; each half window is whole drive periods, so at zero bias the largest amplitude answered hangs on the frequency: 33.437 mA at 650 GHz, 26.164 mA at 100 GHz, 9.209 mA at 10 GHz. Zero is no drive: no photon, and the frequency then sets the windows alone.",
            only_when: MICROWAVE,
            asked_by_check: true,
        },
        Input {
            name: "drive_frequency_hz",
            kind: Kind::NumberWithin {
                minimum: 0.0,
                minimum_exclusive: true,
                maximum: junction::GAP_FREQUENCY_HZ,
                maximum_exclusive: true,
                unit: "Hz",
            },
            refusal: Some("DriveAtOrAboveGapFrequency"),
            bound: "f, the frequency of the drive, above zero (DriveFrequencyNotPositive) and below the gap frequency, 691.6 GHz, e times the gap voltage 2.8602516 mV over h, both ends exclusive: at and above it a photon of the drive breaks a Cooper pair (DriveAtOrAboveGapFrequency), where the amplitude is not zero; at zero amplitude there is no photon and any frequency sets the windows alone. Each point's mean is read over whole periods of the drive, at least 15 plasma periods (21.5 ps) in each half window, so below about 1 GHz one point exceeds one solve's budget of steps (StepsBeyondSolveBudget).",
            only_when: MICROWAVE,
            asked_by_check: true,
        },
    ],
    kept_inside: "The junction's constants (critical current, capacitance, the 199 knots of its dissipative table, normal resistance), the time step, the settling and averaging times and the tolerances stay inside the solver.",
    result: &[
        output("bias_current_a", "A", Shape::Numbers, "The held bias current of each point, as asked."),
        output("mean_voltage_v", "V", Shape::Numbers, "The mean voltage of each point. With no drive: (hbar/2e) 2 pi whole turns of the phase over the time they took, from the first crossing of a multiple of 2 pi in the accepted window to the last (in the worked example at 0.1 mA running, 40 turns over 42.14 ps: 1.9630 mV). Under a drive, the phase advance over the whole window of whole drive periods. Compared with the measured zero-field curve at zero drive: within 11.84 percent of I_c R_n RMS."),
        output("josephson_frequency_hz", "Hz", Shape::Numbers, "The mean voltage over the flux quantum h/2e: flux quanta crossing the junction per second. On a Shapiro step it is a whole multiple, or a simple fraction, of the drive frequency."),
        output("phase_turns", "1", Shape::Numbers, "Turns of the phase the mean was read over: with no drive the whole turns from the first crossing of a multiple of 2 pi in the window to the last, when at least one was made; otherwise the phase advance over the window over 2 pi."),
        output("averaging_time_s", "s", Shape::Numbers, "The accepted window: 30 plasma periods (43.04 ps) or whole drive periods, or a window twice, four or eight times that when an earlier one did not settle. The powers are means over it. With no drive the mean voltage is read over the whole turns inside it, a shorter time, so phase_turns flux quanta over averaging_time_s is not the mean voltage: 1.9217 mV against 1.9630 at 0.1 mA running."),
        output("peak_voltage_v", "V", Shape::Numbers, "The largest |voltage| landed in the accepted window."),
        output("source_power_w", "W", Shape::Numbers, "The mean power the bias source delivered over the accepted window (averaging_time_s), not over whole turns: with no drive, the bias times the window's own mean voltage (1.9658 mV at 0.1 mA running, 0.14 percent from mean_voltage_v)."),
        output("dissipated_power_w", "W", Shape::Numbers, "The mean power the dissipative current took over the accepted window. It differs from the source's by the change of stored energy over the window, which is not whole turns: by 1.7e-3, 3.2e-3 and 1.1e-3 of the source's at 0.1, 0.2 and 0.4 mA in the worked example."),
        output("normal_line_reached", "1", Shape::Numbers, "1 where a landed voltage of the accepted window was above the top of the table, 4.4255 mV, so that the dissipation there followed the normal line, slope 1/R_n from the table's last knot; else 0."),
        output("time_step_s", "s", Shape::Numbers, "The step of each point: 2.826e-16 s, finer where the bias and drive can carry the voltage above the table, and dividing the drive period whole under a drive."),
        output("steps", "1", Shape::Numbers, "The steps each point took, settling and windows together."),
    ],
    invariants: &[
        output("largest_energy_residual", "1", Shape::Number, ""),
        output("largest_flux_residual_quanta", "1", Shape::Number, ""),
        output("largest_settle_difference", "1", Shape::Number, ""),
    ],
    iterations: "Runge-Kutta steps over every point. On a refusal as NotSettled it is the steps the point took; null otherwise.",
    field: field_words,
    answer: "a current-voltage characteristic",
    refusals: REFUSALS,
    door_refusals: DOOR_REFUSALS,
    sections,
    validation: Some(validation),
    example,
    further_examples: &[crate::solvers::FurtherExample {
        key: "example_solve_shapiro_step",
        parameters: shapiro_example,
    }],
    parts: &[],
    check,
    solve,
};

const SUMMARY: &str = "One NIST niobium / amorphous-silicon / niobium Josephson junction, D220831b-B7_S2_1.5, at 4 K: its current-voltage characteristic, point by point from rest or from the voltage state, and its Shapiro steps under a microwave drive below the gap frequency (691.6 GHz), the shunted junction with its capacitance and its own measured dissipative curve stepped in time, the energy ledger held at every step and the flux quanta at every window; one constant, the capacitance, fitted to the return current; compared once with the held-back zero-field curve of NIST (Hopkins 2023), 0 to 0.91847 mA on both sweeps: the voltage line missed, 11.84 percent of I_c R_n rms against 5 (rising 7.10, falling 15.13), its plateau tilting where the measured one is flat, and the switching and return currents met on the measured bias step, so by the rule fixed beforehand the comparison failed.";

/// Each `JunctionReject` variant with the sentence from its doc comment. A
/// refusal the request itself shows is not sealed.
const REFUSALS: &[Refusal] = &[
    Refusal {
        name: "InputNotFinite",
        meaning: "An input was NaN or infinite.",
        sealed: false,
    },
    Refusal {
        name: "NoBiasPoints",
        meaning: "The question holds no bias current.",
        sealed: false,
    },
    Refusal {
        name: "DriveFrequencyNotPositive",
        meaning: "The drive frequency is zero or negative.",
        sealed: false,
    },
    Refusal {
        name: "DriveAtOrAboveGapFrequency",
        meaning: "The drive frequency is at or above the gap frequency `GAP_FREQUENCY_HZ`, and its amplitude is not zero: a photon of the drive breaks a Cooper pair.",
        sealed: false,
    },
    Refusal {
        name: "StepsBeyondSolveBudget",
        meaning: "The question needs more steps than `MAX_STEPS_PER_SOLVE`, counted before it starts with every window of every point: the budget of computing one solve is given, a budget and not a limit of the physics.",
        sealed: false,
    },
    Refusal {
        name: "NonFiniteValue",
        meaning: "A value of the motion was NaN or infinite.",
        sealed: true,
    },
    Refusal {
        name: "EnergyLedgerBroken",
        meaning: "A step landed with the stored energy departing from the source's work less the dissipation by more than `RESIDUAL_CONSERVATION_LIMIT` of `ENERGY_SCALE_J`.",
        sealed: true,
    },
    Refusal {
        name: "FluxAccountingBroken",
        meaning: "The phase advance over a window departed from 2e/hbar times its voltage integral by more than `FLUX_ACCOUNTING_LIMIT_QUANTA` flux quanta.",
        sealed: true,
    },
    Refusal {
        name: "NotSettled",
        meaning: "After `MAX_WINDOWS` windows the two half-window means of a point still differed by more than `SETTLE_TOLERANCE` of the characteristic voltage, or, with no drive and a bias above the critical current, where no state at rest exists, its phase had not made a whole turn.",
        sealed: true,
    },
];

/// The words of `start` and `drive` that are not the solver's own.
const DOOR_REFUSALS: &[Refusal] = &[
    Refusal {
        name: "UnknownStart",
        meaning: "`start` is not `rest` or `running`.",
        sealed: false,
    },
    Refusal {
        name: "UnknownDrive",
        meaning: "`drive` is not `none` or `microwave`.",
        sealed: false,
    },
];

/// The `JunctionReject` variant name, without its fields.
pub fn reject_name(reject: &JunctionReject) -> &'static str {
    match reject {
        JunctionReject::InputNotFinite { .. } => "InputNotFinite",
        JunctionReject::NoBiasPoints => "NoBiasPoints",
        JunctionReject::DriveFrequencyNotPositive { .. } => "DriveFrequencyNotPositive",
        JunctionReject::DriveAtOrAboveGapFrequency { .. } => "DriveAtOrAboveGapFrequency",
        JunctionReject::StepsBeyondSolveBudget { .. } => "StepsBeyondSolveBudget",
        JunctionReject::NonFiniteValue { .. } => "NonFiniteValue",
        JunctionReject::EnergyLedgerBroken { .. } => "EnergyLedgerBroken",
        JunctionReject::FluxAccountingBroken { .. } => "FluxAccountingBroken",
        JunctionReject::NotSettled { .. } => "NotSettled",
    }
}

fn field_words() -> String {
    "SHA-256 of the count of the numbers that follow as 8 little-endian bytes, which is twice the number of points, then the phase each point ended at, in radians, then the voltage each point ended at, in volts, each in the order asked, all as little-endian f64 bytes.".to_string()
}

fn sections() -> Sections {
    Sections {
        locked: json!({
            "critical_current_a": junction::CRITICAL_CURRENT_A,
            "capacitance_f": junction::CAPACITANCE_F,
            "normal_resistance_ohm": junction::NORMAL_RESISTANCE_OHM,
            "table_knots": junction::TABLE_KNOTS,
            "table_current_a": junction::TABLE_CURRENT_A.to_vec(),
            "table_voltage_v": junction::TABLE_VOLTAGE_V.to_vec(),
            "gap_voltage_v": junction::GAP_VOLTAGE_V,
            "gap_frequency_hz": junction::GAP_FREQUENCY_HZ,
            "josephson_energy_j": junction::JOSEPHSON_ENERGY_J,
            "energy_scale_j": junction::ENERGY_SCALE_J,
            "characteristic_voltage_v": junction::CHARACTERISTIC_VOLTAGE_V,
            "plasma_rate_rad_s": junction::plasma_rate_rad_s(),
            "quality": junction::quality(),
            "time_step_s": junction::table_step_s(),
            "phase_step_limit_rad": junction::phase_step_limit_rad(),
            "settle_s": junction::settle_s(),
            "equations": "phase: d(phase)/dt = (2e/hbar) V. Voltage: C dV/dt = I_b(t) - I_d(V) - I_c sin(phase), I_b(t) = I_dc + I_ac sin(2 pi f t). Dissipative current I_d(V): straight lines between the table's knots, odd in V, and above the last knot the line of slope 1/R_n through it. e and h are the exact SI values.",
            "where_each_comes_from": "Critical current: stated in the headers of the junction's two files (the last bias the laboratory read at zero voltage). Table: the junction's own field-suppressed curve, Fig9b-IV-D220831b-B7_S2_1.5-Suppr.dat, both sweeps, made single-valued (readings above 0.05 mV, averaged at one current, voltage made non-decreasing, every interval at least 1 percent of R_n, the origin first). Normal resistance: voltage over current at the top of that curve. Capacitance: fitted so that the model's return current falls in the middle of the measured return bracket, 0.05567 to 0.06494 mA, two bias currents of the zero-field curve; the source publishes none, and a parallel-plate estimate gives 36.6 fF. Gap voltage: the table's voltage where its plateau begins, knot 33.",
            "fitted": "One constant, the capacitance, to two bias currents of the compared curve's falling sweep (its return bracket). The table is the junction's own curve with its supercurrent suppressed by a field; nothing else is fitted.",
            "bath": "The junction's bath is held at the measurement temperature, 4 K by the paper's abstract: no self-heating, no thermal noise, zero applied field, and a dissipative current that follows the voltage of the instant.",
        }),
        invariants: json!([
            {
                "name": "largest_energy_residual",
                "rule": "At every landed step of every point: |E - E_start - (W_s - W_d)| over hbar I_c / e (1.961e-19 J), E = 1/2 C V^2 + E_J (1 - cos(phase)) the charging and Josephson energy, W_s the work of the source, the integral of I_b V, and W_d the dissipation, the integral of I_d V. The Runge-Kutta step carries W_s - W_d as a variable of its own and never evaluates E; the ledger compares the two.",
                "tolerance": junction::RESIDUAL_CONSERVATION_LIMIT,
                "unit": "hbar I_c / e",
                "compared": "every step",
                "refusal": "EnergyLedgerBroken",
            },
            {
                "name": "largest_flux_residual_quanta",
                "rule": "Over every window: the phase's advance against 2e/hbar times the integral of the voltage, in flux quanta. Each whole turn of the phase is one flux quantum h/2e crossing the junction; the mean voltage is read from those turns.",
                "tolerance": junction::FLUX_ACCOUNTING_LIMIT_QUANTA,
                "unit": "flux quanta",
                "compared": "every window",
                "refusal": "FluxAccountingBroken",
            },
            {
                "name": "largest_settle_difference",
                "rule": "The mean voltages of the two halves of each accepted window differ by at most this fraction of I_c R_n (1.4354 mV); with no drive and a bias above the critical current, where no state at rest exists, the window's phase has also made a whole turn.",
                "tolerance": junction::SETTLE_TOLERANCE,
                "unit": "I_c R_n",
                "compared": "every window",
                "refusal": "NotSettled",
            },
        ]),
        convergence: json!({
            "settle": "(60 + 12 Q) plasma periods, Q = 11.11 the quality of the small-voltage motion (plasma rate x capacitance x the table's chord resistance at 0.02 of the critical current): 277 ps.",
            "average": "30 plasma periods (43.0 ps), or the fewest whole drive periods spanning 15 plasma periods in each half.",
            "max_windows": junction::MAX_WINDOWS,
            "settle_tolerance": junction::SETTLE_TOLERANCE,
            "max_steps_per_solve": junction::MAX_STEPS_PER_SOLVE,
            "rule": "Fixed-step classical fourth-order Runge-Kutta in SI units. Each point settles, then reads its mean over a window; the window is accepted when the means of its two halves agree within the tolerance. One that does not counts as more settling, and the next window is twice as long, up to four. With no drive and a bias above the critical current a window whose phase made no whole turn does not settle, however alike its halves: the point is creeping toward its switch, and a bias 1e-10 above the critical current, whose switch takes longer than every window, is refused. A point with no steady mean (a driven junction can be chaotic) is refused as NotSettled, and the whole question with it.",
            "step": "An eighth of the shortest RC time of the table (0.0479 ohm x 47.21 fF): 2.826e-16 s, the step at which a million steps on the plateau held the ledger at 1.7e-8 in a stand-in of the junction. Where (|I_dc| + |I_ac| + I_c) R_n exceeds the table's top the step is shortened in proportion, so that the phase moves no more than 0.0038 rad per step.",
            "run_time": "On one core of an Apple M4 Pro in a release build, measured under load, 117 ns a step with no drive and 147 ns under one: about 0.15 s a point that settles in its first window (36 points at 0.5 mA, 40.8 million steps, 6.3 s); a question whose every point takes all four windows runs nearly its whole count (34 points at 0.15 mA running under 0.1 mA at 300 GHz, 117.6 million steps, 17.2 s), so the budget of 120,000,000 steps is about 18 s.",
        }),
        reference: json!({
            "name": junction::REFERENCE_NAME,
            "url": junction::REFERENCE_URL,
            "retrieved": junction::REFERENCE_RETRIEVED,
            "test": "nist_return_bracket_and_critical_current",
            "covers": "The fitting data of the compared curve: at the stated critical current, 0.298 mA, a point from rest holds zero voltage and at 0.3 mA it switches; on the falling branch the junction returns to zero between 0.0596 and 0.0610 mA, inside the measured return bracket 0.05567 to 0.06494 mA.",
            "kind": "Measurements of the junction itself, in the fit. The comparison with the zero-field curve outside the fit is in validation.",
        }),
    }
}

fn validation() -> [String; 5] {
    let object = "Junction D220831b-B7_S2_1.5, made and measured at NIST Boulder: sputtered niobium electrodes and an amorphous-silicon barrier, a nominal diameter of 1.5 um, at 4 K and zero applied field. A superconductor-insulator-superconductor tunnel junction: on a rising bias its voltage stays zero to the critical current, 0.298 mA, then jumps to a plateau near the gap voltage, 2.86 mV; on a falling bias it stays in the voltage state down to about 0.06 mA. The solver steps the shunted junction with its capacitance and the junction's own dissipative curve.".to_string();
    let data = "NIST Public Data Repository, doi 10.18434/mds2-2921: the current-voltage curves of junction D220831b-B7_S2_1.5 behind Applied Physics Letters 122, 182601, doi 10.1063/5.0148250. In the fit: the junction's field-suppressed curve (Fig9b), its stated critical current, and two bias currents of its zero-field curve, the return bracket. Outside the fit: the voltages of the zero-field curve, Fig9a-IV-D220831b-B7_S2_1.5.dat, 100 bias points rising from rest and 100 falling from the voltage state, 0 to 0.91847 mA.".to_string();
    let pass_line = "Voltage: at every bias point of both sweeps within the table's current range (to 0.91872 mA) where model and measurement are on the same branch (both above or both at or below 0.05 mV), the model's mean voltage less the measured, over I_c R_n (1.4354 mV): RMS at most 5 percent; points where the model's landed voltage rose above the table's top are counted beside it. Switching current (lowest bias of the rising sweep in the voltage state) and return current (lowest bias of the falling sweep in the voltage state): each within one bias step of the measured. Record SHA-256 27ad14413a30eed6e939afd1e1603ddaf4fb3d829d6823b8f5887a0f03e6238b.".to_string();
    let limits = "The model leaves out self-heating (the measured plateau falls about 1 percent as the current rises), thermal noise, the magnetic field (the table was measured in one, the compared curve without; on a sibling junction the plateau moved 4 percent of I_c R_n with the field) and any memory of the past voltage in the dissipative current. The voltage error is read as the last of these: on the plateau the model's voltage ripples at the Josephson frequency, 1.4 THz, twice the gap frequency, and each swing above the plateau draws a burst of quasiparticle current, so its plateau tilts where the measured one is flat. The dissipation table is the junction's dc curve taken quasi-statically, which holds where h f/e is small against the 22 uV over which its current steps up at the gap (f well below 5.4 GHz); the junction's own ac Josephson frequency exceeds the gap frequency above V_g/2 = 1.430 mV, and the drives served (above 1 GHz, the Shapiro example at 650 GHz, whose step locks the junction at 1.3 THz) lie far above it too, where photon-assisted tunnelling (Tien and Gordon) spreads the step over n h f/e. The capacitance is fitted to the return bracket of the compared curve, so the return current reproduces that fit, and the switching current confirms that the stated critical current is the last bias at zero voltage. Of the zero-field curve, these were read before the comparison: its first two readings near zero bias and its largest current and voltage. Under a microwave drive the solver computes the same equations with no measurement to compare: the steps fall at n h f / 2e, constants of nature, and a driven point with no steady mean is refused by name (NotSettled). The ledger and the flux accounting confirm the equations were solved, and a proof confirms an answer is unchanged; how closely the model matches the junction is the comparison above.".to_string();
    [object, data, pass_line, RESULT.to_string(), limits]
}

/// The comparison with the zero-field curve outside the fit.
const RESULT: &str = "On the zero-field curve outside the fit, 100 bias points rising from rest and 100 falling from the voltage state, 0 to 0.91847 mA, computed through the engine. Voltage error, RMS over the 175 points on the same branch, 11.84 percent of I_c R_n (target 5): rising sweep 7.10, falling 15.13; mean -1.22, worst -36.3 at 0.1113 mA falling; with the 25 points whose voltage rippled past the table's top included, 11.11 percent. Switching current 0.30616 mA and return current 0.06494 mA, each on the measured bias step (target one bias step); both are inputs of the solver, the stated critical current and the return bracket the capacitance is fitted to. The same voltage error on three sibling junctions, each with its own constants, computed outside the engine: 10.5 to 16.2 percent. Where the voltage is off: the measured plateau is flat, 2.8498 to 2.8651 mV from 0.33 to 0.57 mA, and the solver's rises from 2.826 to 3.096 mV (+16.8 percent at 0.566 mA rising, +19.5 at 0.594 mA falling); below the critical current on the falling sweep it reads low, by up to 36 percent; above 0.64 mA it is within -8.5 to +3.4 percent. Every proof held, the largest energy residual was 7.3e-8 of hbar I_c / e, and every point settled in its first window.";

fn example() -> Value {
    json!({
        "bias_current_a": [0.0001, 0.0002, 0.0004],
        "start": "running",
        "drive": "none"
    })
}

fn shapiro_example() -> Value {
    json!({
        "bias_current_a": [0.00025, 0.0003, 0.00035],
        "start": "running",
        "drive": "microwave",
        "drive_current_amplitude_a": 0.0002,
        "drive_frequency_hz": 650000000000.0
    })
}

/// The solver's own question, from the values of `parameters`.
fn question(given: &Given) -> Result<JunctionQuestion, String> {
    let start = match given.word("start")? {
        "rest" => Start::Rest,
        _ => Start::Running,
    };
    let drive = match given.word("drive")? {
        "microwave" => Drive::Microwave {
            current_amplitude_a: given.number("drive_current_amplitude_a")?,
            frequency_hz: given.number("drive_frequency_hz")?,
        },
        _ => Drive::None,
    };
    Ok(JunctionQuestion {
        bias_current_a: given.numbers("bias_current_a")?.to_vec(),
        start,
        drive,
    })
}

/// Every limit the request shows, asked of the solver's own check before a
/// physics permit is taken. The sentence is the solver's, with its name for
/// the refusal.
fn check(given: &Given) -> Result<(), String> {
    junction::admit(&question(given)?)
        .map(|_| ())
        .map_err(|reject| format!("Physical Violation: {reject} ({}).", reject_name(&reject)))
}

fn solve(given: &Given) -> Result<Solved, String> {
    Ok(solved(junction::solve(&question(given)?)))
}

/// What the solver returned, in the words every solver shares.
pub fn solved(returned: Result<JunctionCurve, JunctionReject>) -> Solved {
    let curve = match returned {
        Ok(curve) => curve,
        Err(reject) => {
            let name = reject_name(&reject);
            let sealed = REFUSALS
                .iter()
                .any(|refusal| refusal.name == name && refusal.sealed);
            return Solved::Refused(Refused {
                name,
                sentence: if sealed {
                    reject.to_string()
                } else {
                    format!("{reject} ({name})")
                },
                iterations: match reject {
                    JunctionReject::NotSettled { steps, .. } => Some(steps),
                    _ => None,
                },
            });
        }
    };
    let one = |name: &str, value: f64| (name.to_string(), Published::Number(value));
    let many = |name: &str, values: Vec<f64>| (name.to_string(), Published::Numbers(values));
    let mut field = Vec::with_capacity(2 * curve.final_phase_rad.len());
    field.extend_from_slice(&curve.final_phase_rad);
    field.extend_from_slice(&curve.final_voltage_v);
    Solved::Answer(Answer {
        iterations: curve.total_steps,
        result: vec![
            many("bias_current_a", curve.bias_current_a),
            many("mean_voltage_v", curve.mean_voltage_v),
            many("josephson_frequency_hz", curve.josephson_frequency_hz),
            many("phase_turns", curve.phase_turns),
            many("averaging_time_s", curve.averaging_time_s),
            many("peak_voltage_v", curve.peak_voltage_v),
            many("source_power_w", curve.source_power_w),
            many("dissipated_power_w", curve.dissipated_power_w),
            many("normal_line_reached", curve.normal_line_reached),
            many("time_step_s", curve.time_step_s),
            many("steps", curve.steps),
        ],
        invariants: vec![
            one("largest_energy_residual", curve.largest_energy_residual),
            one("largest_flux_residual_quanta", curve.largest_flux_residual_quanta),
            one("largest_settle_difference", curve.largest_settle_difference),
        ],
        field,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::solvers::describe_value;

    const SOLVER_SOURCE: &str = include_str!("../../genesis_solvers/src/junction_nist_b7_s2.rs");

    /// `(variant, doc sentence)` for every variant of `JunctionReject`, read
    /// from the solver's source. An intra-doc link is written as its bare code
    /// span.
    fn documented_rejects() -> Vec<(String, String)> {
        let start = SOLVER_SOURCE
            .find("pub enum JunctionReject {")
            .expect("JunctionReject is in the solver");
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

    #[test]
    fn every_refusal_carries_the_sentence_from_its_doc_comment() {
        let documented = documented_rejects();
        assert_eq!(documented.len(), REFUSALS.len());
        for ((variant, sentence), refusal) in documented.iter().zip(REFUSALS) {
            assert_eq!(variant, refusal.name);
            assert_eq!(sentence, refusal.meaning, "{}", refusal.name);
        }
        let body = describe_value(JUNCTION_NIST_B7_S2.name).expect("described");
        let refusals = body["refusals"].as_array().expect("refusals");
        assert_eq!(refusals.len(), documented.len() + DOOR_REFUSALS.len());
        for (refusal, listed) in refusals.iter().zip(REFUSALS.iter().chain(DOOR_REFUSALS)) {
            let reported = if listed.sealed { "200, outcome refused, sealed" } else { "422, no seal" };
            assert_eq!(refusal["reported_as"], reported, "{}", refusal["name"]);
        }
    }

    #[test]
    fn the_reference_is_the_one_stored_in_the_solver_test() {
        let body = describe_value(JUNCTION_NIST_B7_S2.name).expect("described");
        let url = body["reference"]["url"].as_str().expect("url");
        assert_eq!(url, junction::REFERENCE_URL);
        assert!(SOLVER_SOURCE.contains(url), "{url}");
        let test = body["reference"]["test"].as_str().expect("test");
        assert!(SOLVER_SOURCE.contains(&format!("fn {test}()")), "{test}");
    }

    /// The published bounds are the solver's own constants.
    #[test]
    fn the_bounds_are_the_solvers() {
        let body = describe_value(JUNCTION_NIST_B7_S2.name).expect("described");
        let text = body.to_string();
        assert!(text.contains("691.6 GHz"));
        assert!((junction::GAP_FREQUENCY_HZ / 691.6e9 - 1.0).abs() < 1e-4);
        assert!(text.contains("1 to 36 points"));
        assert_eq!(junction::MAX_BIAS_POINTS, 36);
        assert!((junction::CURRENT_REACH_MAX_A - 0.033456).abs() < 1e-6);
        let frequency = &body["inputs"].as_array().unwrap()[4];
        assert_eq!((frequency["minimum_exclusive"].clone(), frequency["maximum_exclusive"].clone()), (json!(true), json!(true)));
        assert_eq!(frequency["maximum"], junction::GAP_FREQUENCY_HZ);
        // The entry's own words never say "check".
        let mut own = vec![SUMMARY.to_string(), JUNCTION_NIST_B7_S2.kept_inside.to_string(), field_words()];
        own.extend(validation());
        let parts = sections();
        for value in [parts.locked, parts.invariants, parts.convergence, parts.reference] {
            own.push(value.to_string());
        }
        for input in JUNCTION_NIST_B7_S2.inputs {
            own.push(input.bound.to_string());
        }
        for output in JUNCTION_NIST_B7_S2.result {
            own.push(output.note.to_string());
        }
        for refusal in REFUSALS.iter().chain(DOOR_REFUSALS) {
            own.push(refusal.meaning.to_string());
        }
        for words in own {
            assert!(!words.to_lowercase().contains("check"), "{words}");
        }
    }

    #[test]
    fn every_published_name_is_in_the_entry() {
        let curve = match solved(junction::solve(&JunctionQuestion {
            bias_current_a: vec![4.0e-4],
            start: Start::Rest,
            drive: Drive::None,
        })) {
            Solved::Answer(answer) => answer,
            Solved::Refused(refused) => panic!("{}", refused.sentence),
        };
        assert_eq!(curve.result.len(), JUNCTION_NIST_B7_S2.result.len());
        for ((name, value), output) in curve.result.iter().zip(JUNCTION_NIST_B7_S2.result) {
            assert_eq!(name, output.name);
            assert_eq!(value.shape(), output.shape);
        }
        assert_eq!(curve.invariants.len(), JUNCTION_NIST_B7_S2.invariants.len());
        for ((name, _), output) in curve.invariants.iter().zip(JUNCTION_NIST_B7_S2.invariants) {
            assert_eq!(name, output.name);
        }
        assert_eq!(curve.field.len(), 2);
    }
}
