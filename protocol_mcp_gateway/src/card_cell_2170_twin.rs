//! The gateway's call of the cell_2170_twin card: it reads a request, holds it to the card's
//! published bounds, steps the card and publishes its state, as the gateway does for
//! cell_2170_model3. Its description is `card_cell_2170_twin_descriptor.rs`, a child of
//! `descriptor.rs`.

use genesis_kernel::cell_2170_twin;
use genesis_kernel::frame::LastStateFrame64;

use crate::live::StepFailure;
use crate::{
    IntegratedFrame, Published, StateField, blank_frame, card_clock, descriptor, enforce_published, fields_are_finite,
    message_number, phase_not_valid, refused, reject_unknown_load, required_f64, state_object, unsealed,
};

pub(crate) const DOMAIN: &str = "energy";
pub(crate) const CARD: &str = "cell_2170_model3";

/// The flags the card declares, ORed.
pub(crate) const FLAGS: u64 = cell_2170_twin::FLAG_SUSTAINED_ABOVE_1C | cell_2170_twin::FLAG_OUTSIDE_COMPARED_RUNS;

/// The cell type holds its three curves; one copy serves every call.
pub(crate) fn cell_type() -> &'static cell_2170_twin::Cell2170Twin {
    static CELL: std::sync::OnceLock<cell_2170_twin::Cell2170Twin> = std::sync::OnceLock::new();
    CELL.get_or_init(cell_2170_twin::Cell2170Twin::cell)
}

pub(crate) struct Cell2170TwinCall {
    cell: &'static cell_2170_twin::Cell2170Twin,
    pub(crate) state: cell_2170_twin::CellState,
    load: cell_2170_twin::CellLoad,
    dt_s: f64,
}

/// The cells the card holds, in words: `80 (cell m80), 1000 (cell m1000) and 448 (cell m448-N)`.
pub(crate) fn cells_in_words() -> String {
    let names: Vec<String> =
        cell_2170_twin::CELLS.iter().map(|row| format!("{} (cell {})", message_number(row.cell_number), row.name)).collect();
    match names.split_last() {
        Some((last, rest)) if !rest.is_empty() => format!("{} and {last}", rest.join(", ")),
        _ => names.join(""),
    }
}

pub(crate) const BELOW_ABSOLUTE_ZERO: &str =
    "case temperature is at or below absolute zero, -273.15 degC, where the temperature laws have no value (CaseTemperatureBelowAbsoluteZero).";

fn cell_not_recognized(cell_number: f64) -> String {
    unsealed(&format!(
        "'cell_number' {} is not one of the cells of the {CARD} card, {} (CellNotRecognized).",
        message_number(cell_number),
        cells_in_words()
    ))
}

fn span_inverted(least_c: f64, most_c: f64) -> String {
    unsealed(&format!(
        "the start carries {} to {} degC of case temperature, a least above its most (CaseTemperatureSpanInverted).",
        message_number(least_c),
        message_number(most_c)
    ))
}

/// The sentence of each refusal the card gives part way, in plain words and with its name in
/// round brackets at the end.
fn reject_sentence(reject: cell_2170_twin::StepReject) -> String {
    use cell_2170_twin::StepReject;
    match reject {
        StepReject::UndervoltageLimit => unsealed("the cell's voltage would fall below the 2.8 V lower limit (UndervoltageLimit)."),
        StepReject::OvervoltageLimit => unsealed("the cell's voltage would rise above the 4.242 V upper limit (OvervoltageLimit)."),
        StepReject::ElectrolyteDepleted => unsealed(
            "the salt in the electrolyte at one electrode would be used up: its concentration difference reaches the transport limit, and the cell cannot carry the current at its core temperature (ElectrolyteDepleted).",
        ),
        StepReject::CoreAtElectrolyteGlassTemperature => unsealed(&format!(
            "the cell's core is at or below the electrolyte's glass temperature, {} K, where the electrolyte no longer conducts (CoreAtElectrolyteGlassTemperature).",
            message_number(cell_2170_twin::ELECTROLYTE_GLASS_TEMPERATURE_K)
        )),
        StepReject::StateOfChargeOutsideCard => unsealed(
            "the state of charge would leave [-0.05, 1.05], the ends of the card's rest-voltage curves, outside which it has no rest voltage, by more than the rounding allowance of its running sum (StateOfChargeOutsideCard).",
        ),
        StepReject::SurfaceOutsideCurve => unsealed(
            "the state of charge at the particle surface would leave [-0.05, 1.05], the ends of the card's rest-voltage curves, outside which it has no rest voltage (SurfaceOutsideCurve).",
        ),
        StepReject::CaseTemperatureSpanInverted => unsealed(
            "the least case temperature the state carries is above the most it carries (CaseTemperatureSpanInverted).",
        ),
        StepReject::CaseTemperatureBelowAbsoluteZero => unsealed(BELOW_ABSOLUTE_ZERO),
        StepReject::CapacityNotPositive => unsealed("the capacity is zero or less (CapacityNotPositive)."),
        StepReject::CellNotRecognized => {
            unsealed(&format!("the cell number is not one of {} (CellNotRecognized).", cells_in_words()))
        }
        StepReject::SpontaneousGeneration => {
            unsealed("the cell created energy, at its terminals or in its core's heat (SpontaneousGeneration).")
        }
        StepReject::FirstLawBreach => unsealed("the cell destroyed energy, at its terminals or in its core's heat (FirstLawBreach)."),
        other => refused(CARD, other),
    }
}

/// The current and the case temperature of a step or of a segment. A load outside the compared
/// range is the step's flag, not a refusal.
fn read_load(params: &serde_json::Value) -> Result<cell_2170_twin::CellLoad, String> {
    let current_a = required_f64(params, "current_a")?;
    let case_temperature_c = required_f64(params, "case_temperature_c")?;
    if !(case_temperature_c + cell_type().kelvin_offset > 0.0) {
        return Err(unsealed(BELOW_ABSOLUTE_ZERO));
    }
    Ok(cell_2170_twin::CellLoad { current_a, case_temperature_c })
}

impl Cell2170TwinCall {
    pub(crate) fn from_parameters(params: &serde_json::Value) -> Result<Self, String> {
        reject_unknown_load(params, descriptor::parameter_names(DOMAIN, CARD))?;
        enforce_published(DOMAIN, CARD, Published::Step, params)?;
        let cell_number = required_f64(params, "cell_number")?;
        let state_of_charge = required_f64(params, "state_of_charge")?;
        let capacity_ah = required_f64(params, "capacity_ah")?;
        // The span of case temperature is accumulated over the run, so a start carries the least
        // and the most of the run so far; nothing defaults.
        let least_c = required_f64(params, "case_temperature_least_c")?;
        let most_c = required_f64(params, "case_temperature_most_c")?;
        let load = read_load(params)?;
        let dt_s = required_f64(params, "dt_s")?;
        let cell = cell_type();
        card_clock(dt_s, None, cell.floor_dt_s(), cell.gate_dt_s(), CARD)?;
        if cell.cell_of(cell_number).is_none() {
            return Err(cell_not_recognized(cell_number));
        }
        let state = cell_2170_twin::CellState::try_rested(cell, cell_number, state_of_charge, capacity_ah)
            .ok_or_else(|| phase_not_valid(CARD))?;
        if !(least_c <= most_c) {
            return Err(span_inverted(least_c, most_c));
        }
        if !(least_c + cell.kelvin_offset > 0.0) {
            return Err(unsealed(BELOW_ABSOLUTE_ZERO));
        }
        let state = state.with_case_span(cell, least_c, most_c).ok_or_else(|| phase_not_valid(CARD))?;
        Ok(Self { cell, state, load, dt_s })
    }

    /// A later segment's load, held to the card's own limits.
    pub(crate) fn replace_load(&mut self, params: &serde_json::Value) -> Result<(), String> {
        reject_unknown_load(params, descriptor::load_names(DOMAIN, CARD))?;
        enforce_published(DOMAIN, CARD, Published::Load, params)?;
        self.load = read_load(params)?;
        Ok(())
    }

    pub(crate) fn step_into(&mut self, frame: &mut LastStateFrame64) -> Result<(), StepFailure> {
        cell_2170_twin::step(self.cell, &mut self.state, frame, self.load, self.dt_s).map_err(|reject| {
            let detail = reject_sentence(reject);
            StepFailure::from_reject(reject, detail)
        })
    }

    /// The limits the step from `before` to the state now held is past
    /// (`crate::exceedance`).
    pub(crate) fn exceedances(&self, before: &cell_2170_twin::CellState) -> genesis_kernel::exceedance::Exceedances {
        genesis_kernel::cell_2170_twin::exceedances(self.cell, before, &self.state, self.load, self.dt_s)
    }

    pub(crate) fn run(mut self, proof: [u8; 16]) -> Result<IntegratedFrame, String> {
        let mut frame = blank_frame(proof);
        let before = self.state;
        self.step_into(&mut frame).map_err(|failure| failure.detail)?;
        Ok(IntegratedFrame {
            frame,
            exceedances: self.exceedances(&before),
            residual_conservation_error: self.state.residual_conservation_error,
            state: state(&self.state)?,
        })
    }
}

pub(crate) fn state(state: &cell_2170_twin::CellState) -> Result<serde_json::Value, String> {
    with(state, state_object)
}

pub(crate) fn published_is_finite(state: &cell_2170_twin::CellState) -> Result<(), String> {
    with(state, fields_are_finite)
}

/// Every number of the state: what the caller gave, the eleven diffusion modes and the other
/// memories, the core, what the last step read and returned, and both ledgers.
pub(crate) fn with<T>(state: &cell_2170_twin::CellState, finish: impl FnOnce(&[(&str, StateField)]) -> T) -> T {
    let lag = &state.diffusion_lag;
    finish(&[
        ("t_s", StateField::F64(state.t_s)),
        ("cell_number", StateField::F64(state.cell_number)),
        ("capacity_ah", StateField::F64(state.capacity_ah)),
        ("state_of_charge", StateField::F64(state.state_of_charge)),
        ("surface_state_of_charge", StateField::F64(state.surface_state_of_charge)),
        ("charge_removed_ah", StateField::F64(state.charge_removed_ah)),
        ("current_a", StateField::F64(state.current_a)),
        ("case_temperature_c", StateField::F64(state.case_temperature_c)),
        ("case_temperature_least_c", StateField::F64(state.case_temperature_least_c)),
        ("case_temperature_most_c", StateField::F64(state.case_temperature_most_c)),
        ("core_temperature_c", StateField::F64(state.core_temperature_c)),
        ("terminal_v", StateField::F64(state.terminal_v)),
        ("rest_v", StateField::F64(state.rest_v)),
        ("resistance_at_once_ohm", StateField::F64(state.resistance_at_once_ohm)),
        ("reaction_overpotential_v", StateField::F64(state.reaction_overpotential_v)),
        ("near_full_overpotential_v", StateField::F64(state.near_full_overpotential_v)),
        ("hysteresis_v", StateField::F64(state.hysteresis_v)),
        ("electrolyte_v", StateField::F64(state.electrolyte_v)),
        ("electrolyte_drop_v", StateField::F64(state.electrolyte_drop_v)),
        ("current_mean_a", StateField::F64(state.current_mean_a)),
        ("heat_rate_w", StateField::F64(state.heat_rate_w)),
        ("diffusion_lag_1", StateField::F64(lag[0])),
        ("diffusion_lag_2", StateField::F64(lag[1])),
        ("diffusion_lag_3", StateField::F64(lag[2])),
        ("diffusion_lag_4", StateField::F64(lag[3])),
        ("diffusion_lag_5", StateField::F64(lag[4])),
        ("diffusion_lag_6", StateField::F64(lag[5])),
        ("diffusion_lag_7", StateField::F64(lag[6])),
        ("diffusion_lag_8", StateField::F64(lag[7])),
        ("diffusion_lag_9", StateField::F64(lag[8])),
        ("diffusion_lag_10", StateField::F64(lag[9])),
        ("diffusion_lag_11", StateField::F64(lag[10])),
        ("chemical_energy_j", StateField::F64(state.chemical_energy_j)),
        ("energy_anchor_j", StateField::F64(state.energy_anchor_j)),
        ("electrical_work_j", StateField::F64(state.electrical_work_j)),
        ("heat_j", StateField::F64(state.heat_j)),
        ("heat_stored_j", StateField::F64(state.heat_stored_j)),
        ("heat_to_case_j", StateField::F64(state.heat_to_case_j)),
        ("energy_defect_j", StateField::F64(state.energy_defect_j)),
        ("thermal_defect_j", StateField::F64(state.thermal_defect_j)),
        ("residual_conservation_error", StateField::F64(state.residual_conservation_error)),
        ("core_known", StateField::Bool(state.core_known)),
        ("sustained_above_1c", StateField::Bool(state.sustained_above_1c)),
    ])
}

/// The card at rest: the published fields of a full m80 at the most compared capacity.
pub(crate) fn rest<T>(finish: impl FnOnce(&[(&str, StateField)]) -> T) -> T {
    let state = cell_2170_twin::CellState::try_rested(
        cell_type(),
        cell_2170_twin::CELLS[0].cell_number,
        1.0,
        cell_2170_twin::CAPACITY_MAX_COMPARED,
    )
    .expect("the twin rests");
    with(&state, finish)
}
