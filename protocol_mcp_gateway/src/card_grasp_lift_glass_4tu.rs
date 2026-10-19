//! The gateway's call of the grasp_lift_glass_4tu card: it reads a request,
//! holds it to the card's published bounds, steps the card and publishes its
//! state. Stamped from the kit. Its description is
//! `card_grasp_lift_glass_4tu_descriptor.rs`, a child of `descriptor.rs`.

use genesis_kernel::frame::LastStateFrame64;
use genesis_kernel::grasp_lift_glass_4tu;

use crate::live::StepFailure;
use crate::{
    IntegratedFrame, Published, StateField, blank_frame, card_clock, descriptor, enforce_published, fields_are_finite,
    phase_not_valid, refused, reject_unknown_load, required_f64, state_object, unsealed,
};

pub(crate) const DOMAIN: &str = "materials";
pub(crate) const CARD: &str = "grasp_lift_glass_4tu";

/// The flags the card declares, ORed.
pub(crate) const FLAGS: u64 = grasp_lift_glass_4tu::FLAG_INCIPIENT_SLIP
    | grasp_lift_glass_4tu::FLAG_GROSS_SLIP
    | grasp_lift_glass_4tu::FLAG_ON_TABLE;

pub(crate) struct GraspLiftGlass4tuCall {
    card: grasp_lift_glass_4tu::GraspLiftGlass4tu,
    pub(crate) state: grasp_lift_glass_4tu::GraspLiftGlass4tuState,
    load: grasp_lift_glass_4tu::GraspLiftGlass4tuLoad,
    dt_s: f64,
}

/// The sentence of each refusal the card gives part way, in plain words and
/// with its name in round brackets at the end.
fn reject_sentence(reject: grasp_lift_glass_4tu::StepReject) -> String {
    use grasp_lift_glass_4tu::StepReject;
    match reject {
        StepReject::NoContact => unsealed("the grip force is zero or below: the pads do not press on the faces (NoContact)."),
        StepReject::ObjectBelowTable => unsealed("the object is below the surface of the table (ObjectBelowTable)."),
        StepReject::PadsReachTable => unsealed(&format!(
            "the pads slide down to the lower end of the faces, {} m below the grip, with the object resting on the table: the fingers reach the table's surface (PadsReachTable).",
            grasp_lift_glass_4tu::FACE_HALF_HEIGHT_M
        )),
        StepReject::ObjectSlippedOut => unsealed(&format!(
            "a pad slides past the end of its face, {} m from the grip: the grip lets the object go (ObjectSlippedOut).",
            grasp_lift_glass_4tu::FACE_HALF_HEIGHT_M
        )),
        StepReject::ObjectLanded => unsealed("the lifted object comes back down onto the table, the end of its lift (ObjectLanded)."),
        StepReject::FrictionHeatNegative => unsealed("the step's friction heat fell (FrictionHeatNegative)."),
        StepReject::SpontaneousGeneration => unsealed("the step created energy (SpontaneousGeneration)."),
        StepReject::FirstLawBreach => unsealed("the step destroyed energy (FirstLawBreach)."),
        StepReject::MomentumLedgerBroken => {
            unsealed("the change of the object's momentum is not the impulse on it (MomentumLedgerBroken).")
        }
        other => refused(CARD, other),
    }
}

fn read_load(params: &serde_json::Value) -> Result<grasp_lift_glass_4tu::GraspLiftGlass4tuLoad, String> {
    Ok(grasp_lift_glass_4tu::GraspLiftGlass4tuLoad {
        grip_force_n: required_f64(params, "grip_force_n")?,
        hand_acceleration_m_s2: required_f64(params, "hand_acceleration_m_s2")?,
    })
}

impl GraspLiftGlass4tuCall {
    pub(crate) fn from_parameters(params: &serde_json::Value) -> Result<Self, String> {
        reject_unknown_load(params, descriptor::parameter_names(DOMAIN, CARD))?;
        enforce_published(DOMAIN, CARD, Published::Step, params)?;
        let hand_velocity_m_s = required_f64(params, "hand_velocity_m_s")?;
        let load = read_load(params)?;
        let dt_s = required_f64(params, "dt_s")?;
        let card = grasp_lift_glass_4tu::GraspLiftGlass4tu::specimen();
        card_clock(dt_s, Some(card.contact_pole_rad_s()), card.floor_dt_s(), card.gate_dt_s(), CARD)?;
        let state = grasp_lift_glass_4tu::GraspLiftGlass4tuState::try_start(&card, hand_velocity_m_s)
            .ok_or_else(|| phase_not_valid(CARD))?;
        Ok(Self { card, state, load, dt_s })
    }

    pub(crate) fn replace_load(&mut self, params: &serde_json::Value) -> Result<(), String> {
        reject_unknown_load(params, descriptor::load_names(DOMAIN, CARD))?;
        enforce_published(DOMAIN, CARD, Published::Load, params)?;
        self.load = read_load(params)?;
        Ok(())
    }

    pub(crate) fn step_into(&mut self, frame: &mut LastStateFrame64) -> Result<(), StepFailure> {
        grasp_lift_glass_4tu::step(&self.card, &mut self.state, frame, self.load, self.dt_s)
            .map_err(|reject| StepFailure::from_reject(reject, reject_sentence(reject)))
    }

    /// The limits the step from `before` to the state now held is past
    /// (`crate::exceedance`).
    pub(crate) fn exceedances(&self, before: &grasp_lift_glass_4tu::GraspLiftGlass4tuState) -> genesis_kernel::exceedance::Exceedances {
        genesis_kernel::grasp_lift_glass_4tu::exceedances(&self.card, before, &self.state, self.load, self.dt_s)
    }

    pub(crate) fn run(mut self, proof: [u8; 16]) -> Result<IntegratedFrame, String> {
        let before = self.state;
        let mut frame = blank_frame(proof);
        self.step_into(&mut frame).map_err(|failure| failure.detail)?;
        Ok(IntegratedFrame {
            frame,
            exceedances: self.exceedances(&before),
            residual_conservation_error: self.state.residual_conservation_error,
            state: state(&self.state)?,
        })
    }
}

pub(crate) fn state(state: &grasp_lift_glass_4tu::GraspLiftGlass4tuState) -> Result<serde_json::Value, String> {
    with(state, state_object)
}

pub(crate) fn published_is_finite(state: &grasp_lift_glass_4tu::GraspLiftGlass4tuState) -> Result<(), String> {
    with(state, fields_are_finite)
}

/// The published fields of the state, in order. The thirty-two element
/// forces of a pad are the card's own and are not published; what they sum
/// to is.
pub(crate) fn with<T>(
    state: &grasp_lift_glass_4tu::GraspLiftGlass4tuState,
    finish: impl FnOnce(&[(&str, StateField)]) -> T,
) -> T {
    finish(&[
        ("t_s", StateField::F64(state.t_s)),
        ("hand_height_m", StateField::F64(state.hand_height_m)),
        ("hand_velocity_m_s", StateField::F64(state.hand_velocity_m_s)),
        ("object_height_m", StateField::F64(state.object_height_m)),
        ("object_velocity_m_s", StateField::F64(state.object_velocity_m_s)),
        ("object_acceleration_m_s2", StateField::F64(state.object_acceleration_m_s2)),
        ("grip_force_n", StateField::F64(state.grip_force_n)),
        ("hand_acceleration_m_s2", StateField::F64(state.hand_acceleration_m_s2)),
        ("pad_travel_m", StateField::F64(state.pad_travel_m)),
        ("tangential_force_n", StateField::F64(state.tangential_force_n)),
        ("friction_limit_n", StateField::F64(state.friction_limit_n)),
        ("load_ratio", StateField::F64(state.load_ratio)),
        ("stuck_fraction", StateField::F64(state.stuck_fraction)),
        ("table_force_n", StateField::F64(state.table_force_n)),
        ("kinetic_energy_j", StateField::F64(state.kinetic_energy_j)),
        ("potential_energy_j", StateField::F64(state.potential_energy_j)),
        ("stored_energy_j", StateField::F64(state.stored_energy_j)),
        ("energy_anchor_j", StateField::F64(state.energy_anchor_j)),
        ("hand_work_j", StateField::F64(state.hand_work_j)),
        ("hand_work_in_j", StateField::F64(state.hand_work_in_j)),
        ("hand_work_out_j", StateField::F64(state.hand_work_out_j)),
        ("friction_heat_j", StateField::F64(state.friction_heat_j)),
        ("energy_defect_j", StateField::F64(state.energy_defect_j)),
        ("energy_scale_j", StateField::F64(state.energy_scale_j)),
        ("momentum_n_s", StateField::F64(state.momentum_n_s)),
        ("momentum_anchor_n_s", StateField::F64(state.momentum_anchor_n_s)),
        ("contact_impulse_n_s", StateField::F64(state.contact_impulse_n_s)),
        ("gravity_impulse_n_s", StateField::F64(state.gravity_impulse_n_s)),
        ("table_impulse_n_s", StateField::F64(state.table_impulse_n_s)),
        ("momentum_defect_n_s", StateField::F64(state.momentum_defect_n_s)),
        ("momentum_scale_n_s", StateField::F64(state.momentum_scale_n_s)),
        ("residual_conservation_error", StateField::F64(state.residual_conservation_error)),
        ("gross_slip_m", StateField::F64(state.gross_slip_m)),
        ("peak_load_ratio", StateField::F64(state.peak_load_ratio)),
        ("slipped", StateField::Bool(state.slipped)),
        ("incipient_slip", StateField::Bool(state.incipient_slip)),
        ("gross_slip", StateField::Bool(state.gross_slip)),
        ("on_table", StateField::Bool(state.on_table)),
    ])
}

/// The card at rest: the published fields of a fresh grip, the hand still.
pub(crate) fn rest<T>(finish: impl FnOnce(&[(&str, StateField)]) -> T) -> T {
    let card = grasp_lift_glass_4tu::GraspLiftGlass4tu::specimen();
    let state = grasp_lift_glass_4tu::GraspLiftGlass4tuState::try_start(&card, 0.0).expect("the grip closes");
    with(&state, finish)
}
