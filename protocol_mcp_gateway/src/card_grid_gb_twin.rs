//! The gateway's call of the grid_gb_twin card: it reads a request, holds it to
//! the card's published bounds, steps the card and publishes its state.
//! Its description is `card_grid_gb_twin_descriptor.rs`, a child of
//! `descriptor.rs`.

use genesis_kernel::frame::LastStateFrame64;
use genesis_kernel::grid_gb_twin;

use crate::live::StepFailure;
use crate::{
    IntegratedFrame, Published, StateField, blank_frame, card_clock, descriptor, enforce_published, fields_are_finite,
    phase_not_valid, refused, reject_unknown_load, required_f64, state_object, unsealed,
};

pub(crate) const DOMAIN: &str = "energy";
pub(crate) const CARD: &str = "grid_gb";

/// The flags the card declares, ORed.
pub(crate) const FLAGS: u64 = grid_gb_twin::FLAG_OUTSIDE_COMPARED_TRIPS
    | grid_gb_twin::FLAG_PAST_COMPARED_MINUTE
    | grid_gb_twin::FLAG_BELOW_COMPARED_FREQUENCY;

pub(crate) struct GridGbTwinCall {
    card: grid_gb_twin::GridGbTwin,
    pub(crate) state: grid_gb_twin::GridGbTwinState,
    load: grid_gb_twin::GridGbTwinLoad,
    dt_s: f64,
}

/// The sentence of each refusal the card gives part way, in plain words and
/// with its name in round brackets at the end.
fn reject_sentence(reject: grid_gb_twin::StepReject) -> String {
    match reject {
        grid_gb_twin::StepReject::InertiaNotPositive => {
            unsealed("the stored kinetic energy is zero or less; the rate of change of frequency divides by it (InertiaNotPositive).")
        }
        grid_gb_twin::StepReject::DemandNegative => unsealed("the demand is below zero (DemandNegative)."),
        grid_gb_twin::StepReject::ServiceVolumeNegative => {
            unsealed("a contracted volume of a dynamic service is below zero (ServiceVolumeNegative).")
        }
        grid_gb_twin::StepReject::DeliveryTimeNotPositive => {
            unsealed("a time to full delivery is zero or less; the service's lag divides by it (DeliveryTimeNotPositive).")
        }
        grid_gb_twin::StepReject::LossLargerThanDemand => {
            unsealed("the power lost is larger in size than the demand the area carries (LossLargerThanDemand).")
        }
        grid_gb_twin::StepReject::NegativeTime => unsealed("the clock is below zero (NegativeTime)."),
        grid_gb_twin::StepReject::LowFrequencyDemandDisconnection => unsealed(&format!(
            "the frequency is at or below {} Hz, where low-frequency demand disconnection begins (LowFrequencyDemandDisconnection).",
            grid_gb_twin::FREQUENCY_MIN_HZ
        )),
        grid_gb_twin::StepReject::FrequencyAboveCard => unsealed(&format!(
            "the frequency is above {} Hz, the upper statutory limit (FrequencyAboveCard).",
            grid_gb_twin::FREQUENCY_MAX_HZ
        )),
        grid_gb_twin::StepReject::SpontaneousGeneration => {
            unsealed("the step created kinetic energy from nothing (SpontaneousGeneration).")
        }
        grid_gb_twin::StepReject::FirstLawBreach => unsealed("the step destroyed kinetic energy (FirstLawBreach)."),
        other => refused(CARD, other),
    }
}

fn read_load(params: &serde_json::Value) -> Result<grid_gb_twin::GridGbTwinLoad, String> {
    Ok(grid_gb_twin::GridGbTwinLoad { loss_mw: required_f64(params, "loss_mw")? })
}

fn read_held(params: &serde_json::Value) -> Result<grid_gb_twin::ResponseHeld, String> {
    Ok(grid_gb_twin::ResponseHeld {
        dc_low_mw: required_f64(params, "dc_low_mw")?,
        dm_low_mw: required_f64(params, "dm_low_mw")?,
        dr_low_mw: required_f64(params, "dr_low_mw")?,
        dc_high_mw: required_f64(params, "dc_high_mw")?,
        dm_high_mw: required_f64(params, "dm_high_mw")?,
        dr_high_mw: required_f64(params, "dr_high_mw")?,
        fast_delivery_s: required_f64(params, "fast_delivery_s")?,
        regulation_delivery_s: required_f64(params, "regulation_delivery_s")?,
    })
}

impl GridGbTwinCall {
    pub(crate) fn from_parameters(params: &serde_json::Value) -> Result<Self, String> {
        reject_unknown_load(params, descriptor::parameter_names(DOMAIN, CARD))?;
        enforce_published(DOMAIN, CARD, Published::Step, params)?;
        let frequency_hz = required_f64(params, "frequency_hz")?;
        let inertia_mw_s = required_f64(params, "inertia_mw_s")?;
        let demand_mw = required_f64(params, "demand_mw")?;
        let held = read_held(params)?;
        let load = read_load(params)?;
        let dt_s = required_f64(params, "dt_s")?;
        let card = grid_gb_twin::GridGbTwin::great_britain();
        // The ceiling is a fiftieth of the fastest time scale the card can
        // reach with this run's inertia, demand and response held: the
        // published ceiling, or the run's own where that is smaller.
        let own = card.gate_dt_s_at(inertia_mw_s, demand_mw, &held);
        // At a vanishing inertia the fastest rate overflows (no ceiling is
        // returned) or the ceiling falls under the normal range: every step
        // is above it, and no number under 2^-1022 is written (the random
        // conformance test of 16 October 2026, D1b, fixed in 0.14.0 on the
        // earlier grid_gb card and kept on this one since the fold of 0.15.0).
        if inertia_mw_s > 0.0 && demand_mw >= 0.0 && own.is_none_or(|own| own < f64::MIN_POSITIVE) && dt_s > 0.0 {
            return Err(format!(
                "Physical Violation: 'dt_s' {} is above the step ceiling of the grid_gb card at this inertia and demand, a fiftieth of the time of the fastest rate the swing reaches, which is under {:e} s, the smallest normal 64-bit number ({}).",
                crate::message_number(dt_s),
                f64::MIN_POSITIVE,
                crate::step_outside_name("grid_gb", true)
            ));
        }
        let ceiling = match (card.gate_dt_s(), own) {
            (Some(published), Some(own)) => Some(published.min(own)),
            (published, _) => published,
        };
        card_clock(dt_s, None, card.floor_dt_s(), ceiling, CARD)?;
        let state = grid_gb_twin::GridGbTwinState::try_start(&card, frequency_hz, inertia_mw_s, demand_mw, held)
            .ok_or_else(|| phase_not_valid(CARD))?;
        Ok(Self { card, state, load, dt_s })
    }

    /// The load of a segment. A segment may carry another loss than the
    /// first: the card books each step's own loss, and flags a loss that is
    /// not the run's first as outside the compared trips.
    pub(crate) fn replace_load(&mut self, params: &serde_json::Value) -> Result<(), String> {
        reject_unknown_load(params, descriptor::load_names(DOMAIN, CARD))?;
        enforce_published(DOMAIN, CARD, Published::Load, params)?;
        self.load = read_load(params)?;
        Ok(())
    }

    pub(crate) fn step_into(&mut self, frame: &mut LastStateFrame64) -> Result<(), StepFailure> {
        grid_gb_twin::step(&self.card, &mut self.state, frame, self.load, self.dt_s)
            .map_err(|reject| StepFailure::from_reject(reject, reject_sentence(reject)))
    }

    /// The limits the step from `before` to the state now held is past
    /// (`crate::exceedance`).
    pub(crate) fn exceedances(&self, before: &grid_gb_twin::GridGbTwinState) -> genesis_kernel::exceedance::Exceedances {
        genesis_kernel::grid_gb_twin::exceedances(&self.card, before, &self.state, self.load, self.dt_s)
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

pub(crate) fn state(state: &grid_gb_twin::GridGbTwinState) -> Result<serde_json::Value, String> {
    with(state, state_object)
}

pub(crate) fn published_is_finite(state: &grid_gb_twin::GridGbTwinState) -> Result<(), String> {
    with(state, fields_are_finite)
}

/// The published fields of the state, in order.
pub(crate) fn with<T>(state: &grid_gb_twin::GridGbTwinState, finish: impl FnOnce(&[(&str, StateField)]) -> T) -> T {
    finish(&[
        ("t_s", StateField::F64(state.t_s)),
        ("frequency_hz", StateField::F64(state.frequency_hz)),
        ("plant_response_mw", StateField::F64(state.plant_response_mw)),
        ("fast_response_mw", StateField::F64(state.fast_response_mw)),
        ("regulation_response_mw", StateField::F64(state.regulation_response_mw)),
        ("service_response_mw", StateField::F64(state.service_response_mw())),
        ("relief_mw", StateField::F64(state.relief_mw)),
        ("net_power_mw", StateField::F64(state.net_power_mw)),
        ("frequency_rate_hz_per_s", StateField::F64(state.frequency_rate_hz_per_s)),
        ("lowest_frequency_hz", StateField::F64(state.lowest_frequency_hz)),
        ("lowest_time_s", StateField::F64(state.lowest_time_s)),
        ("start_frequency_hz", StateField::F64(state.start_frequency_hz)),
        ("inertia_mw_s", StateField::F64(state.inertia_mw_s)),
        ("demand_mw", StateField::F64(state.demand_mw)),
        ("dc_low_mw", StateField::F64(state.held.dc_low_mw)),
        ("dm_low_mw", StateField::F64(state.held.dm_low_mw)),
        ("dr_low_mw", StateField::F64(state.held.dr_low_mw)),
        ("dc_high_mw", StateField::F64(state.held.dc_high_mw)),
        ("dm_high_mw", StateField::F64(state.held.dm_high_mw)),
        ("dr_high_mw", StateField::F64(state.held.dr_high_mw)),
        ("fast_delivery_s", StateField::F64(state.held.fast_delivery_s)),
        ("regulation_delivery_s", StateField::F64(state.held.regulation_delivery_s)),
        ("kinetic_energy_mw_s", StateField::F64(state.kinetic_energy_mw_s)),
        ("kinetic_energy_anchor_mw_s", StateField::F64(state.kinetic_energy_anchor_mw_s)),
        ("energy_lost_mw_s", StateField::F64(state.energy_lost_mw_s)),
        ("energy_response_mw_s", StateField::F64(state.energy_response_mw_s)),
        ("frequency_deficit_integral_hz_s", StateField::F64(state.frequency_deficit_integral_hz_s)),
        ("energy_defect_mw_s", StateField::F64(state.energy_defect_mw_s)),
        ("residual_conservation_error", StateField::F64(state.residual_conservation_error)),
    ])
}

/// The card at rest: the published fields of a fresh start at 50 Hz, the
/// smallest compared inertia and demand, and no response held.
pub(crate) fn rest<T>(finish: impl FnOnce(&[(&str, StateField)]) -> T) -> T {
    let card = grid_gb_twin::GridGbTwin::great_britain();
    let state = grid_gb_twin::GridGbTwinState::try_start(
        &card,
        grid_gb_twin::NOMINAL_FREQUENCY_HZ,
        grid_gb_twin::INERTIA_MIN_MW_S,
        grid_gb_twin::DEMAND_MIN_MW,
        grid_gb_twin::ResponseHeld::none(),
    )
    .expect("the area rests");
    with(&state, finish)
}
